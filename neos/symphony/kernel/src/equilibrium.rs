//! Harmonic Force Equilibrium as load balancing.
//!
//! `div E = rho/eps0` with `E = -grad(phi)` becomes `L phi = -rho/eps0` on the
//! core-topology graph. Load flows down the gradient of `phi`.
//!
//! Law: `_mkb/resonance.md` Part 2. Contract §5.

use crate::constants::DIFFUSION_STABILITY_FACTOR;
use crate::KernelError;
use lattice::tessellation::CellId;
use lattice::Tiling;
use std::collections::HashMap;

/// Cores mapped onto a compact patch of {5,4} cells.
///
/// Adjacency is **consumed from `lattice`**, never recomputed here — see
/// contract §6. That is what makes neighbour naming free of runtime discovery:
/// it is a closed-form group operation, not a search.
#[derive(Debug, Clone)]
pub struct CoreTopology {
    cells: Vec<CellId>,
    adjacency: Vec<Vec<usize>>,
}

impl CoreTopology {
    /// Build a topology for `core_count` cores.
    ///
    /// Cores occupy cells in BFS ring order, so an n-core machine is a compact
    /// patch around the origin cell rather than a scattered set.
    pub fn from_tiling(core_count: usize) -> Self {
        assert!(core_count > 0, "a topology needs at least one core");

        // Grow until the tiling holds enough cells, then take the first n in
        // ring order. Ring sizes are 5*Fib(2n), so this terminates quickly.
        let mut depth = 1;
        let mut tiling = Tiling::grow(depth);
        while tiling.len() < core_count && depth < 12 {
            depth += 1;
            tiling = Tiling::grow(depth);
        }

        let mut cells = Vec::with_capacity(core_count);
        'outer: for ring in 0.. {
            match tiling.layer(ring) {
                Some(ids) => {
                    for id in ids {
                        if cells.len() == core_count {
                            break 'outer;
                        }
                        cells.push(*id);
                    }
                }
                None => break,
            }
        }

        let index: HashMap<CellId, usize> =
            cells.iter().enumerate().map(|(i, id)| (*id, i)).collect();

        // Adjacency restricted to the patch. Boundary cores legitimately have
        // fewer than five neighbours inside it.
        let adjacency = cells
            .iter()
            .map(|id| {
                let cell = tiling.get(id).expect("cell came from this tiling");
                cell.neighbors()
                    .iter()
                    .filter_map(|n| index.get(&n.id()).copied())
                    .collect()
            })
            .collect();

        Self { cells, adjacency }
    }

    pub fn len(&self) -> usize {
        self.cells.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }

    pub fn cells(&self) -> &[CellId] {
        &self.cells
    }

    pub fn neighbors(&self, core: usize) -> &[usize] {
        &self.adjacency[core]
    }

    pub fn degree(&self, core: usize) -> usize {
        self.adjacency[core].len()
    }

    pub fn max_degree(&self) -> usize {
        self.adjacency.iter().map(Vec::len).max().unwrap_or(0)
    }

    pub fn min_degree(&self) -> usize {
        self.adjacency.iter().map(Vec::len).min().unwrap_or(0)
    }

    /// The graph Laplacian `L = D - A`.
    pub fn laplacian(&self) -> Vec<Vec<f64>> {
        let n = self.len();
        let mut l = vec![vec![0.0; n]; n];
        for i in 0..n {
            l[i][i] = self.degree(i) as f64;
            for &j in &self.adjacency[i] {
                l[i][j] -= 1.0;
            }
        }
        l
    }

    /// Stability bound on the diffusion coupling: `alpha < 2 / lambda_max(L)`.
    ///
    /// `lambda_max <= 2 * d_max` by Gershgorin, which is cheap and safe — no
    /// eigensolver needed. Exceeding this makes the balancer oscillate, which
    /// is the thrashing the field model exists to prevent.
    pub fn stability_bound(&self) -> f64 {
        let d_max = self.max_degree().max(1) as f64;
        DIFFUSION_STABILITY_FACTOR / (2.0 * d_max)
    }

    /// Whether every core is reachable from core 0.
    ///
    /// A disconnected patch enlarges `L`'s nullspace beyond the constants and
    /// silently breaks the solvability condition.
    pub fn is_connected(&self) -> bool {
        if self.is_empty() {
            return false;
        }
        let mut seen = vec![false; self.len()];
        let mut stack = vec![0usize];
        seen[0] = true;
        let mut count = 1;
        while let Some(i) = stack.pop() {
            for &j in &self.adjacency[i] {
                if !seen[j] {
                    seen[j] = true;
                    count += 1;
                    stack.push(j);
                }
            }
        }
        count == self.len()
    }
}

/// Per-core load, relaxed toward equilibrium by the field equation.
#[derive(Debug, Clone)]
pub struct LoadField {
    load: Vec<f64>,
}

impl LoadField {
    pub fn new(load: Vec<f64>) -> Self {
        Self { load }
    }

    pub fn len(&self) -> usize {
        self.load.len()
    }

    pub fn is_empty(&self) -> bool {
        self.load.is_empty()
    }

    pub fn load(&self) -> &[f64] {
        &self.load
    }

    pub fn total(&self) -> f64 {
        self.load.iter().sum()
    }

    pub fn mean(&self) -> f64 {
        self.total() / self.len() as f64
    }

    pub fn spread(&self) -> f64 {
        let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
        for &v in &self.load {
            lo = lo.min(v);
            hi = hi.max(v);
        }
        hi - lo
    }

    /// Spread as a fraction of mean load.
    ///
    /// **Convergence must be judged on this, not on [`spread`](Self::spread).**
    /// Absolute spread is meaningless as a stopping criterion here: task
    /// energies are of order `1e-25` J, so any fixed absolute tolerance above
    /// that reports convergence before the field has relaxed at all. Relative
    /// spread is scale-free and means the same thing at every load magnitude.
    ///
    /// Zero when the mean is zero (an empty or fully reclaimed field).
    pub fn relative_spread(&self) -> f64 {
        let m = self.mean().abs();
        if m == 0.0 {
            0.0
        } else {
            self.spread() / m
        }
    }

    /// Task density `rho`, **always mean-centred**.
    ///
    /// `sum(rho) = 0` is the condition for `L phi = -rho/eps0` to have a
    /// solution at all — `L`'s nullspace is the constants. There is
    /// deliberately no accessor returning absolute load as a density, because
    /// absolute load makes the system unsolvable.
    pub fn task_density(&self) -> Vec<f64> {
        let m = self.mean();
        self.load.iter().map(|v| v - m).collect()
    }

    /// One diffusion step: `x <- x - alpha * L x`.
    ///
    /// # Errors
    /// [`KernelError::Unstable`] if `alpha` meets or exceeds the topology's
    /// stability bound. Rejecting is the point — an out-of-bound coupling
    /// oscillates instead of converging.
    pub fn relax(&mut self, topo: &CoreTopology, alpha: f64) -> Result<(), KernelError> {
        let bound = topo.stability_bound();
        if !(alpha > 0.0) || alpha >= bound {
            return Err(KernelError::Unstable { alpha, bound });
        }
        let flux: Vec<f64> = (0..self.len())
            .map(|i| {
                let d = topo.degree(i) as f64;
                d * self.load[i] - topo.neighbors(i).iter().map(|&j| self.load[j]).sum::<f64>()
            })
            .collect();
        for (v, f) in self.load.iter_mut().zip(flux) {
            *v -= alpha * f;
        }
        Ok(())
    }

    /// Relax until [`relative_spread`](Self::relative_spread) falls below
    /// `tolerance`, or `max_steps` elapse.
    ///
    /// The tolerance is **relative**, so it means the same thing whether loads
    /// are of order 1 or of order `1e-25`.
    ///
    /// # Errors
    /// [`KernelError::Unstable`] if `alpha` is outside the topology's bound.
    /// **Validated before the loop**, so an already-converged field still
    /// rejects a bad coupling rather than silently accepting it.
    pub fn relax_to_equilibrium(
        &mut self,
        topo: &CoreTopology,
        alpha: f64,
        tolerance: f64,
        max_steps: usize,
    ) -> Result<usize, KernelError> {
        let bound = topo.stability_bound();
        if !(alpha > 0.0) || alpha >= bound {
            return Err(KernelError::Unstable { alpha, bound });
        }
        for step in 0..max_steps {
            if self.relative_spread() <= tolerance {
                return Ok(step);
            }
            self.relax(topo, alpha)?;
        }
        Ok(max_steps)
    }
}

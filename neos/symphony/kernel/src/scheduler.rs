//! The tripartite scheduler: quantization, resonance, equilibrium.
//!
//! Processes are energy states on a self-stabilising harmonic field, not
//! time-sliced threads on a priority queue. This module is the policy that
//! drives the three mechanisms from real task arrivals.
//!
//! ## The cycle
//!
//! 1. **Ingest** arrivals, assigning each task to a core.
//! 2. **Quantize** — per-core load is the sum of task energies `E = C_H * nu`.
//! 3. **Relax** the field toward equilibrium (`equilibrium`).
//! 4. **Migrate** tasks down the load gradient, *along topology edges only*.
//! 5. **Reclaim** tasks whose frequency has decayed to nothing.
//!
//! Migration is edge-local by construction: a task moves to an adjacent core or
//! not at all. A scheduler that could relocate a task anywhere would not be
//! following a field — it would be a queue with extra steps.

use crate::equilibrium::{CoreTopology, LoadField};
use crate::quantization::{energy, is_reclaimable, Frequency};
use crate::resonance::xi;
use crate::KernelError;

/// Convergence threshold on *relative* load spread.
///
/// Deliberately relative. Absolute tolerances are meaningless here: energies
/// are of order `1e-25` J, so an absolute threshold anywhere above that reports
/// a pathologically imbalanced field as already converged.
pub const RELATIVE_TOLERANCE: f64 = 1e-9;

/// Identifies a task. Also the node identity in the wait-for graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TaskId(pub u64);

/// A unit of work, priced by its frequency.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Task {
    pub id: TaskId,
    /// Ordinary frequency. Priority is an energy state, not a queue position.
    pub frequency: Frequency,
    /// Observation scale for the resonance correction.
    pub scale: f64,
}

impl Task {
    pub fn new(id: u64, hertz: f64) -> Self {
        Self {
            id: TaskId(id),
            frequency: Frequency::hertz(hertz),
            scale: 1.0,
        }
    }

    pub fn with_scale(mut self, scale: f64) -> Self {
        self.scale = scale;
        self
    }

    /// Energy cost, with the resonance correction applied at this task's scale.
    ///
    /// `xi` never gates correctness — if the scale is invalid the uncorrected
    /// energy is returned rather than failing the task. Degraded precision, not
    /// a wrong answer.
    pub fn energy_joules(&self) -> f64 {
        let base = energy(self.frequency).0;
        match xi(self.scale) {
            Ok(correction) => base * correction,
            Err(_) => base,
        }
    }

    pub fn is_reclaimable(&self) -> bool {
        is_reclaimable(self.frequency)
    }
}

/// Tasks placed on cores, with the field that balances them.
#[derive(Debug, Clone)]
pub struct Scheduler {
    topology: CoreTopology,
    /// `placement[core]` holds the tasks currently resident there.
    placement: Vec<Vec<Task>>,
    reclaimed: Vec<TaskId>,
}

/// What one scheduling pass did.
#[derive(Debug, Clone, PartialEq)]
pub struct SchedulePass {
    pub relaxation_steps: usize,
    pub migrations: usize,
    pub reclaimed: usize,
    pub spread_before: f64,
    pub spread_after: f64,
}

impl Scheduler {
    pub fn new(core_count: usize) -> Self {
        let topology = CoreTopology::from_tiling(core_count);
        Self {
            placement: vec![Vec::new(); topology.len()],
            topology,
            reclaimed: Vec::new(),
        }
    }

    pub fn topology(&self) -> &CoreTopology {
        &self.topology
    }

    pub fn core_count(&self) -> usize {
        self.topology.len()
    }

    pub fn task_count(&self) -> usize {
        self.placement.iter().map(Vec::len).sum()
    }

    pub fn tasks_on(&self, core: usize) -> &[Task] {
        &self.placement[core]
    }

    pub fn reclaimed(&self) -> &[TaskId] {
        &self.reclaimed
    }

    /// Ingest an arrival stream.
    ///
    /// Arrivals land on the least-loaded core at admission. That is a *seed*,
    /// not the policy — the field decides where work ends up. Admission just
    /// avoids constructing a pathological starting state for no reason.
    pub fn ingest(&mut self, arrivals: impl IntoIterator<Item = Task>) {
        for task in arrivals {
            let core = self
                .load_per_core()
                .iter()
                .enumerate()
                .min_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
                .map(|(i, _)| i)
                .unwrap_or(0);
            self.placement[core].push(task);
        }
    }

    /// Per-core load: the summed energy of resident tasks.
    pub fn load_per_core(&self) -> Vec<f64> {
        self.placement
            .iter()
            .map(|ts| ts.iter().map(Task::energy_joules).sum())
            .collect()
    }

    /// The load field, from which mean-centred task density is derived.
    pub fn load_field(&self) -> LoadField {
        LoadField::new(self.load_per_core())
    }

    /// Task density `rho`, always mean-centred (solvability, contract §5.1).
    pub fn task_density(&self) -> Vec<f64> {
        self.load_field().task_density()
    }

    /// Drop tasks whose frequency has decayed to nothing.
    ///
    /// Reclamation is a consequence of `E = C_H * nu`: as `nu -> 0`, `E -> 0`,
    /// and the vector is unmapped. It is not a separate eviction policy.
    pub fn reclaim(&mut self) -> usize {
        let mut n = 0;
        for core in &mut self.placement {
            core.retain(|t| {
                if t.is_reclaimable() {
                    self.reclaimed.push(t.id);
                    n += 1;
                    false
                } else {
                    true
                }
            });
        }
        n
    }

    /// Run one scheduling pass.
    ///
    /// # Errors
    /// [`KernelError::Unstable`] if `alpha` is outside the topology's stability
    /// bound. Callers should take it from [`CoreTopology::stability_bound`].
    pub fn schedule(&mut self, alpha: f64, max_steps: usize) -> Result<SchedulePass, KernelError> {
        let reclaimed = self.reclaim();

        let mut field = self.load_field();
        let spread_before = field.spread();

        // Relative, not absolute: task energies are of order 1e-25 J, so any
        // fixed absolute tolerance would report convergence before the field
        // relaxed at all.
        let steps = field.relax_to_equilibrium(&self.topology, alpha, RELATIVE_TOLERANCE, max_steps)?;
        let target = field.load().to_vec();

        let migrations = self.migrate_toward(&target);

        Ok(SchedulePass {
            relaxation_steps: steps,
            migrations,
            reclaimed,
            spread_before,
            spread_after: self.load_field().spread(),
        })
    }

    /// Move tasks down the load gradient toward `target`.
    ///
    /// Two invariants, both load-bearing:
    ///
    /// 1. **Edge-local, one hop per pass.** A task moves to an *adjacent* core
    ///    or not at all, and moves at most once per pass. Without the
    ///    once-per-pass rule a task could chain 2 -> 5 -> 9 within a single
    ///    call — each hop legal, the net displacement not. Work diffuses
    ///    further across *repeated* passes, which is what diffusion means.
    ///
    /// 2. **Strict improvement.** A move is taken only if it reduces the total
    ///    absolute imbalance across the two cores involved. Load is quantized
    ///    into whole tasks while the target is continuous, so an unguarded
    ///    greedy move can overshoot and leave the field *worse* balanced than
    ///    it found it.
    ///
    /// Returns the number of migrations.
    fn migrate_toward(&mut self, target: &[f64]) -> usize {
        let n = self.topology.len();
        let mut current = self.load_per_core();
        let mut moved_this_pass = vec![false; n];
        let mut migrations = 0;

        for core in 0..n {
            if self.placement[core].len() < 2 {
                continue;
            }
            if current[core] <= target[core] {
                continue;
            }

            let mut best: Option<(usize, usize, f64)> = None; // (task idx, dest, gain)

            for &dest in self.topology.neighbors(core) {
                if current[dest] >= target[dest] {
                    continue;
                }
                for (i, task) in self.placement[core].iter().enumerate() {
                    if moved_this_pass[core] {
                        break;
                    }
                    let e = task.energy_joules();
                    let before =
                        (current[core] - target[core]).abs() + (current[dest] - target[dest]).abs();
                    let after = (current[core] - e - target[core]).abs()
                        + (current[dest] + e - target[dest]).abs();
                    let gain = before - after;
                    if gain > 0.0 && best.map_or(true, |(_, _, g)| gain > g) {
                        best = Some((i, dest, gain));
                    }
                }
            }

            if let Some((i, dest, _)) = best {
                let task = self.placement[core].remove(i);
                let e = task.energy_joules();
                self.placement[dest].push(task);
                current[core] -= e;
                current[dest] += e;
                moved_this_pass[core] = true;
                moved_this_pass[dest] = true;
                migrations += 1;
            }
        }

        migrations
    }

    /// Every task currently resident, in core order.
    pub fn all_tasks(&self) -> Vec<Task> {
        self.placement.iter().flatten().copied().collect()
    }
}

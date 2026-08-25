---
type: design
subsystem: symphony-kernel
stage: 02_design
derived_from: ["../01_derive/output/math-contract.md"]
---

# Symphony-kernel — Design

Types and interfaces. Every item traces to the math contract. Where the contract forbids a value, make it unrepresentable.

## Modules

| File | Job |
|---|---|
| `quantization.rs` | `E = C_H·ν`, frequency types, GC threshold |
| `resonance.rs` | `ξ(r)`, the `H(κ)` health invariant |
| `equilibrium.rs` | Laplacian load balancing over the core topology |
| `scheduler.rs` | ties them together: energy states + equilibrium |

## The `ν` / `ω` separation

Contract §2.1 — the highest-risk defect in the subsystem, so it gets the strongest mechanism.

```rust
pub struct Frequency(f64);        // ordinary, Hz
pub struct AngularFrequency(f64); // rad/s
```

Two distinct newtypes with **no arithmetic between them** and no `From`/`Into`. Conversion is explicit and named:

```rust
impl Frequency        { pub fn to_angular(self) -> AngularFrequency; }  // × 2π
impl AngularFrequency { pub fn to_ordinary(self) -> Frequency; }        // ÷ 2π
```

`energy()` accepts `Frequency` only. Passing `ω` becomes a compile error rather than a `2π` scaling bug that no test of units would catch.

## Quantization

```rust
pub fn energy(nu: Frequency) -> Joules;          // E = C_H·ν
pub fn is_reclaimable(nu: Frequency) -> bool;    // ν → 0 ⇒ E → 0 ⇒ unmap
pub struct Joules(f64);
```

GC is a consequence of the equation, not a separate policy: a process whose frequency has decayed to the reclamation threshold has no energy and its memory is unmapped.

## Resonance

```rust
pub fn xi(r_over_R: f64) -> Result<f64, KernelError>;
pub const XI_SUPREMUM: f64;   // e/sinh(1) = 2.3130352854993315
pub struct DriftIntegrator { /* accumulates H(κ) */ }
impl DriftIntegrator {
    pub fn observe(&mut self, phase_error: f64, dt: f64);
    pub fn residual(&self) -> f64;               // H(κ)
    pub fn is_converging(&self, threshold: f64) -> bool;
}
```

`xi` returns `Result` because `r = 0` is `0/0` in the expression. It is **not an error** — the limit is the supremum — but it must be handled explicitly rather than emitting `NaN`. Negative `r` is genuinely invalid and rejected.

`DriftIntegrator` implements contract §4: `H(κ)` is a monitored health invariant, never an input to scheduling. `is_converging` is what an alarm reads.

## Equilibrium

```rust
pub struct CoreTopology { /* adjacency from lattice cells */ }
impl CoreTopology {
    pub fn from_tiling(core_count: usize) -> Self;   // consumes lattice::Tiling
    pub fn laplacian(&self) -> Vec<Vec<f64>>;
    pub fn max_degree(&self) -> usize;
    pub fn stability_bound(&self) -> f64;            // 2/λ_max, contract §5.2
}

pub struct LoadField { /* per-core load */ }
impl LoadField {
    pub fn task_density(&self) -> Vec<f64>;          // mean-centred, §5.1
    pub fn total(&self) -> f64;
    pub fn spread(&self) -> f64;
    pub fn relax(&mut self, topo: &CoreTopology, alpha: f64)
        -> Result<(), KernelError>;                  // rejects α ≥ bound
}
```

`task_density` **only** returns mean-centred values — there is no accessor for absolute load as a density. The solvability condition of contract §5.1 is enforced by construction rather than checked.

`relax` rejects an out-of-bound `α` instead of silently oscillating. Callers get `α` from `stability_bound()`.

`λ_max` is bounded above by `2·d_max` (Gershgorin), which is cheap and safe — no eigensolver needed.

## Topology from `lattice`

`CoreTopology::from_tiling` grows a `lattice::Tiling` and maps cores onto `CellId`s, taking adjacency from `Cell::neighbors()`. Nothing about `{5,4}` geometry is recomputed here.

Cores are assigned in BFS ring order, so an *n*-core machine occupies a compact patch around the origin cell rather than a scattered set. Adjacency is then a group operation, not a search — which is what "no runtime discovery overhead" means concretely.

Note `lattice` guarantees 5 face-neighbours per cell, but a bounded patch has boundary cells with fewer neighbours **inside the patch**. The Laplacian is built from in-patch adjacency, so boundary cores have lower degree. That is correct — and it is why `max_degree()` is measured rather than assumed to be 5.

## Errors

```rust
pub enum KernelError {
    Unstable { alpha: f64, bound: f64 },      // α ≥ 2/λ_max
    UndefinedScale { r: f64 },                // r < 0 for ξ
    Diverged { residual: f64 },               // |H(κ)| growing
}
```

Named for the physical failure, per doctrine.

## Deliberately not built

- **Deadlock detection.** Required by contract §8, but it is a resource-graph problem, not a field problem — a separate slice with its own contract. Recording it here so its absence is a decision.
- **Phase-based branch evaluation (A2).** Belongs with the interpreter in `symphony-lang`; the kernel schedules tasks, it does not evaluate their conditionals. `logic_phases` is therefore not consumed by this slice.
- **Bifurcation forking (A1).** The scheduler must not scalar-duplicate, but actual `fork` semantics need the runtime task model that `symphony-lang` will define.

## Human check

For each type, could it hold a value the axioms forbid?

- `Frequency` / `AngularFrequency` — cannot be confused; no arithmetic crosses them.
- `LoadField` — cannot expose un-centred density; there is no such accessor.
- `xi` — cannot silently return `NaN`; `r = 0` returns the limit, `r < 0` is an error.
- `relax` — cannot run with an unstable `α`.

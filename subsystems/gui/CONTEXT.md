---
type: subsystem
subsystem: gui
tier: presentation
language: Rust
stage: 04_implement
status: complete
result: "63 tests passing. Tetryen construction, geodesic edges, isometric navigation, live visualisation of the kernel load field, gateway traffic, and pool memory usage, crystallised-media rendering (holographic faces and volumetric time-crystal nodes), and discrete time evolution of Tetryen node states (TetryenState, a synthesis closing the undistilled corpus's f(psi_n, psi_{n-1}) placeholder) — including a full-chain integration suite from real decoded PPM/WAV bytes through to a rendered amplitude. node_amplitude now delegates to lattice::tetryen_node_envelope (relocated so crystallisation can share the same law). Rasterisation and face tessellation still open."
consumes: [lattice, symphony-kernel, ftg, substrate, crystallisation]
slices: ["Tetryen geometry + navigation", "live visualisation", "memory usage + crystallised-media rendering", "Tetryen recurrence (discrete time evolution)"]
prd_sections: ["9"]
binds_axioms: ["A3"]
---

# GUI — Tetryen rendering and fractal UI

One job: render the system's wave mechanics visibly — curvilinear Tetryen boundaries, infinite fractal zoom, and standing-wave visualization of load, memory, and traffic.

## Status: unblocked

Previously blocked because "Tetryen" had no definition in the distilled layer. **Closed** — the definition now lives at [`_mkb/tetryen.md`](../../_mkb/tetryen.md): a curved tetrahedral structure of four nodes at standing-wave positions, characterised as the minimiser of `E[Γ] = ∫(K(s) + H(s)²)ds` with geodesic edges.

One rendering constraint follows directly and is worth knowing before design starts: **edges are geodesics, not line segments.** A straight-edge tetrahedron is a defect, not an approximation.

## The build loop

| Stage | Job | Output |
|---|---|---|
| `01_derive` | pull the exact law that binds this subsystem | `math-contract.md` |
| `02_design` | types and interfaces against that contract | `design.md` |
| `03_tests` | physics assertions, written before code | `test-plan.md` |
| `04_implement` | write the Rust into `neos/gui/` | `implementation-log.md` |

## Scope

**Owns:** `neos/gui/**` — `renderer.rs`, `fractal.rs`, `visualization.rs`, `telemetry.rs`, `evolution.rs`
**PRD sections:** §9 (Graphical User Interface)
**Axioms that bind it:** A3 (hyperbolic space → screen projection)
**Equations that bind it:** Standing Wave Superposition (interference visualization); Tetryen geometry per [`_mkb/tetryen.md`](../../_mkb/tetryen.md)

## Build last

This subsystem visualizes the others' state. It has the most upstream dependencies and the least ability to be validated in isolation, which puts it naturally at the end of the order.

## The Tetryen recurrence — a synthesis, verified before written

`_mkb/tetryen_recurrence.md` closes a placeholder the undistilled corpus (`_mkb/papers/The neccessity of a finite universe.pdf`) named but never defined: `ψ_{n+1} = f(ψ_n, ψ_{n-1})`. That paper was read and evaluated in full before anything was built — every other formula it carries restates law already distilled elsewhere with zero new content, and this placeholder itself has no operational definition, so nothing in the paper could be trusted as-is.

`evolution::TetryenState` is what actually got built: a real second-order discrete recurrence, verified as an exact trig identity (`ψ_{n+1}+ψ_{n-1} = 2cos(ωΔt)ψ_n`, worst error `5.7e-14`) coupled across the Tetryen's six edges by a weight that reuses `tetryen.md`'s own node standing-wave envelope — `node_amplitude(d_H(i,j))`, real geodesic distance, not a Euclidean chord — rather than inventing a new attenuation function. The coupling *structure* (nearest-neighbour relaxation) is an engineering choice, stated as such; the *weight* is real law, reused exactly where it already fits.

Stability was measured, not proven in closed form: bounded for `γ` up to `1e4` at `Δt=0.01`, genuinely diverges past that (`γ≥1e5` at that `Δt`, or `Δt≥1.0` at `γ=1`) — confirmed both directions, not just "it happened to work" in the range tried. `TetryenState::step` refuses rather than propagates a non-finite result (`GuiError::Diverged`), the same discipline `otimes` applies at its own domain limit.

**What this deliberately does not build**: an "emergence" gate. A coherence/dissonance threshold for declaring Tetryen emergence was proposed in the conversation that produced this and rejected — searched `tetryen.md` and `timecrystal.md` directly first, and neither defines any such criterion. Inventing one would have repeated the exact mistake this whole synthesis exists to avoid. Recorded as an open gap in `tetryen_recurrence.md` itself, not built around.

**The coupling weight moved out from under this record.** `node_amplitude`, the node envelope `TetryenState` couples through, used to be defined directly on `renderer::Tetryen`. When [[crystallisation]] needed the identical law for its own `TetryenRecurrence` and cannot depend on `gui` (dependency direction runs `crystallisation → lattice`, not through `gui`), the formula relocated to `lattice::tetryen_node_envelope`; `node_amplitude` is now a one-line delegation. Re-verified behaviour-preserving by the existing suite (37/37 unit tests unchanged) after the move.

## Do not

Load other subsystems' records. They don't share state; they share the factory (`_mkb/`, `_spec/`).

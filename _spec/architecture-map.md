---
type: architecture-map
layer: spec
status: canonical
---

# Core Architecture Mapping

The translation table: traditional discrete OS concepts → NEOS wave mechanics → the subsystem that owns it. This is the fastest route from "I know what a scheduler is" to "which folder do I open."

| Traditional OS Component | NEOS Wave-Based Equivalent | Governing Principle | Owned by |
|---|---|---|---|
| Kernel / CPU Scheduling | Harmonic Force Equilibrium | $\nabla \cdot \mathbf{E} = \rho / \epsilon_0$ | [[symphony-kernel]] |
| Power / Resource Management | Howard Comma Quantization | $E = C_{H}\nu$ | [[symphony-kernel]] |
| File System & Addressing | Hyperbolic Fractal 4D Lattice | $a \otimes b = a \times b + d(a,b)$ | [[lattice]] |
| Data Link & Networking | Standing Wave Confinement | $f(t) = A \sin(\omega t + \phi)$ | [[ftg]] |
| Application data (Layer 7) | crystallisation into 3D/4D shapes | CFT, Tetryen projection | [[crystallisation]] |
| Graphical User Interface | Tetryen Curvilinear Geometry | self-referential wave boundaries | [[gui]] |
| Hardware abstraction / VM | Binary↔wave translation | — | [[substrate]] |

## Subsystem ownership

| Subsystem | Tier | Language | PRD sections | State |
|---|---|---|---|---|
| [[lattice]] | storage | Rust | §5 | built |
| [[substrate]] | 1 — hypervisor | Rust | §3 | built |
| [[symphony-kernel]] | 2 — kernel logic | Rust | §4 | built |
| [[symphony-lang]] | 2 — kernel logic | custom DSL | §3 | deferred |
| [[ftg]] | bridge | Rust | §6, §7 | not started |
| [[crystallisation]] | presentation | Rust | §8 | deferred |
| [[gui]] | presentation | Rust | §9 | not started |

Two records were split after their scope proved too broad for a single four-stage loop: `symphony` → `-kernel` / `-lang`, and `ftg` → `ftg` / `crystallisation`.

Every PRD section has an owner. **§3 is the one section with two**, and legitimately so — "Language Stack" describes both tiers in a single section, the Rust substrate and the DSL above it. Split by subsection, not contested.

## Note on the Gauss law row

$\nabla \cdot \mathbf{E} = \rho / \epsilon_0$ now **has** an execution rule — it governs dynamic load balancing, with $\rho$ as mean-centred task density and $\mathbf{E}$ as the processing capacity field over the core topology. Discretised on the core graph it becomes diffusion-based load balancing, with verified convergence.

See [`_mkb/equations.md`](../_mkb/equations.md#harmonic-force-equilibrium) and [`_mkb/resonance.md` § Part 2](../_mkb/resonance.md#part-2--harmonic-force-equilibrium-as-load-balancing).

The earlier prohibition — do not implement a scheduler against an equation with no stated execution rule — is now satisfied rather than waived.

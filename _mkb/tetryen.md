---
type: geometry-primitive
layer: law
status: canonical
unblocks: gui
---

# The Tetryen

The core geometric primitive of NEOS. Previously undefined in the distilled layer — this file closes that gap and unblocks [[gui]].

Unusually for this corpus, the three sources **agree**. No reconciliation was needed; this is a straight distillation.

## Definition

A **Tetryen** is a curved tetrahedral structure formed by four nodes at the vertices of a tetrahedron, with edges and faces curved by the spherical standing waves the nodes produce.

Sources, all consistent:

| Paper | Contribution |
|---|---|
| `the-geometry-of-proton-and-the-tetryen-shape` | the primary: four particle cores at standing-wave nodes — "equidistant points in a three-dimensional structure at wavelengths where stability may occur" — producing "a curved tetrahedral structure as a result of spherical waves" |
| `Mathematical_Fra.pdf` | the hyperbolic reinterpretation: a **geodesic tetrahedron** minimising an energy functional |
| `vACUUM_FLUX.pdf` | the node dynamics: four soliton nodes, each oscillating as a standing wave |

## Variational characterisation

The Tetryen is the minimiser of

$$E[\Gamma] = \int_\Gamma \left( K(s) + H(s)^2 \right)\,ds$$

where `K(s)` is Gaussian curvature, `H(s)` mean curvature, and `Γ` the Tetryen boundary.

**Execution rule:** a renderer must not draw straight-edged tetrahedra. Edges are geodesics of the hyperbolic metric, satisfying

$$\frac{d^2x^i}{ds^2} + \Gamma^i_{jk}\frac{dx^j}{ds}\frac{dx^k}{ds} = 0$$

with `Γ^i_jk` the Christoffel symbols. A straight-edge approximation is a rendering defect, not a simplification.

## Node dynamics

Each of the four nodes oscillates as a standing wave:

$$\psi(r) = A\,\sinh\!\left(\frac{r}{R}\right) e^{-r/R}$$

In lattice-native units `R = 1` per [reconciliation.md § R2](reconciliation.md#r2--curvature-k--resolved), so this reduces to `ψ(r) = A·sinh(r)·e^(−r)`.

Node count is exactly **four**, and that is structural, not conventional — it follows from cores occupying standing-wave nodes, which come in fixed positions.

## A note on the reinterpretation

The primary paper describes the Tetryen in Euclidean 3-space, with curvature arising from spherical wave interference. `Mathematical_Fra.pdf` recasts it as a geodesic tetrahedron in hyperbolic space. These are not in conflict — the second is a change of ambient space that makes the same shape natural rather than emergent — but they are **different constructions**, and the distinction matters when implementing.

NEOS uses the **hyperbolic reading**, consistent with A3 and with the lattice being hyperbolic throughout. Recorded so the choice is visible rather than assumed.

## Binds

- [[gui]] — `renderer.rs` (curvilinear boundaries), `fractal.rs` (scale-invariant subdivision)
- [[lattice]] — Tetryen nodes embed in the `{5,4}` tessellation; not consumed by the current lattice build
- [tetryen_recurrence.md](tetryen_recurrence.md) — a synthesis built on this file's node dynamics, giving the nodes discrete time evolution
- Schema: [schemas/tetryen-node.schema.json](schemas/tetryen-node.schema.json) is the serialised single node, not the whole four-node structure

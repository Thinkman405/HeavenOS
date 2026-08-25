---
type: design
subsystem: gui
stage: 02_design
derived_from: ["../01_derive/output/math-contract.md"]
---

# GUI — Design

Types and interfaces. Where the contract forbids a value, make it unrepresentable.

## Modules

| File | PRD | Job |
|---|---|---|
| `renderer.rs` | §9 | Tetryen construction, geodesic edge sampling |
| `fractal.rs` | §9 | navigation as hyperbolic translation |
| `visualization.rs` | §9 | load and traffic as standing waves |

## Geodesic edges

```rust
pub struct GeodesicEdge { from: PoincarePoint, to: PoincarePoint }

impl GeodesicEdge {
    pub fn new(from: PoincarePoint, to: PoincarePoint) -> Self;
    pub fn length(&self) -> f64;
    pub fn point_at(&self, fraction: f64) -> PoincarePoint;   // ON the geodesic
    pub fn sample(&self, segments: usize) -> Vec<PoincarePoint>;
    pub fn deviation_at(&self, p: &PoincarePoint) -> f64;     // membership residual
}
```

`point_at` interpolates in the **hyperboloid model** — `γ(t) = u cosh t + w sinh t` — not by lerping ball coordinates. Lerping is the straight-edge defect the contract forbids, and it is the natural thing to write by accident.

There is deliberately **no `chord_at`** and no `straight: bool` flag. A straight edge is not a lower-quality option; it is a different, wrong geometry.

`deviation_at` returns `d(u,p) + d(p,v) − d(u,v)`, which is `≈0` on the geodesic and strictly positive off it. Exposed so tests assert membership directly rather than eyeballing coordinates.

## The Tetryen

```rust
pub struct Tetryen { nodes: [PoincarePoint; 4], circumradius: f64 }

impl Tetryen {
    pub fn new(circumradius: f64) -> Result<Self, GuiError>;
    pub fn at(centre: &PoincarePoint, circumradius: f64) -> Result<Self, GuiError>;
    pub fn nodes(&self) -> &[PoincarePoint; 4];
    pub fn edges(&self) -> [GeodesicEdge; 6];
    pub fn edge_length(&self) -> f64;
    pub fn node_amplitude(&self, r: f64) -> f64;    // psi(r) = A sinh(r) e^-r
    pub fn is_regular(&self, tol: f64) -> bool;
}
```

`[PoincarePoint; 4]` and `[GeodesicEdge; 6]` are **fixed-size arrays**, not `Vec`. A Tetryen with three or five nodes is not a degenerate Tetryen — it is not a Tetryen. The count is structural (four cores at standing-wave nodes), so the type carries it.

**`E[Γ]` is not minimised.** `new` constructs a regular tetrahedron satisfying the characterisation. The doc comment says so plainly, so the code is not mistaken for a solver.

## Navigation

```rust
pub struct Viewport { transform: Isometry }

impl Viewport {
    pub fn identity() -> Self;
    pub fn translate(&mut self, direction: [f64; 4], distance: f64) -> Result<(), GuiError>;
    pub fn rotate(&mut self, theta: f64);
    pub fn project(&self, p: &PoincarePoint) -> PoincarePoint;
    pub fn isometry_floor(distance: f64) -> f64;
}
```

`Viewport` holds an `Isometry` from `lattice`, so navigation composes through the existing group rather than a parallel implementation.

**There is no `zoom(factor: f64)`.** Offering one would invite Euclidean scaling, which destroys the isometry that makes "infinite resolution" true. Moving closer is `translate`.

`isometry_floor(distance)` gives the tolerance for asserting distance preservation. It **scales with translation distance** for the same reason `ftg::cancellation_floor` scales with the phase argument: ball coordinates crowd toward the boundary as the view moves out, and a fixed bound taken from a short translation fails on a long one.

## Interference visualisation

```rust
pub struct StandingWave { amplitude: f64, k: f64, omega: f64 }

impl StandingWave {
    pub fn for_load(load: f64, k: f64, omega: f64) -> Self;
    pub fn at(&self, x: f64, t: f64) -> f64;          // 2A sin(kx) cos(wt)
    pub fn peak(&self) -> f64;
}

pub enum Interference { Constructive, Destructive }
pub fn combine(a: &StandingWave, b: &StandingWave, phase_delta: f64, x: f64, t: f64) -> f64;
pub fn classify(phase_delta: f64, tol: f64) -> Interference;
```

Zero load renders zero amplitude — the visualisation cannot show activity where there is none.

## Errors

```rust
pub enum GuiError {
    Unrenderable { norm: f64 },       // point outside the ball
    DegenerateEdge,                   // endpoints coincide; no unique geodesic
    InvalidRadius { r: f64 },         // non-positive or non-finite circumradius
}
```

`DegenerateEdge` matters: `w = (v − u cosh d)/sinh d` divides by `sinh(d)`, which is zero when the endpoints coincide. Without the check that is a silent `NaN` propagating into the scene.

## Float tolerances required

| Site | Value | Why |
|---|---|---|
| geodesic membership | `1e-7` | `acosh'(x) → ∞` as `x → 1`; measured floor `2.1e-08` |
| Euclidean chord rejection | none | fails by `3e-3`, five orders above the floor |
| edge regularity | `1e-12` | equal by construction; measured spread exactly `0` |
| isometry preservation | `isometry_floor(d)` | grows with translation distance |
| destructive cancellation | exact `0.0` | opposed phases cancel exactly |

**No tolerance here is machine epsilon.** The geodesic one in particular *cannot* be — §3.2 of the contract explains why, and a `1e-15` choice would fail for reasons unrelated to the renderer.

## Deliberately not built

- **Pixels.** No framebuffer, window, or GPU binding. This is the geometry layer a rasteriser would consume; a rasteriser is a different subsystem and the PRD does not specify one.
- **`E[Γ]` minimisation.** Recorded in the contract as out of scope.
- **Face tessellation.** Edges are built; filling the four curved faces needs a surface-subdivision scheme the corpus does not give.
- **Live binding to kernel state.** `StandingWave::for_load` takes a number. Nothing yet reads `symphony-kernel`'s actual `LoadField` — that wiring is the natural next slice.

## Human check

For each type, could it hold a value the axioms forbid?

- `GeodesicEdge` — no straight-line path exists; `point_at` is the only interpolator.
- `Tetryen` — cannot have other than four nodes; the array type forbids it.
- `Viewport` — no `zoom`; navigation is isometric by construction.
- `StandingWave` — zero load gives zero amplitude.

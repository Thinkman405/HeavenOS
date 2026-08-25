---
type: design
subsystem: lattice
stage: 02_design
derived_from: ["../01_derive/output/math-contract.md"]
---

# Lattice — Design

Types and interfaces. No bodies. Every item traces to a section of the math contract.

Design principle throughout: **where the contract forbids a value, make it unrepresentable** rather than checking for it later.

## Types

### `LatticeScalar` — a value in ⊗ arithmetic

```rust
pub struct LatticeScalar(f64);
```

Newtype over `f64`. Exists so ⊗ cannot be reached by ordinary arithmetic.

- **No `impl Mul`, `Div`, or `Product`.** Contract §3.3 — those traits carry an associativity expectation ⊗ violates. Multiplication is the named method `otimes`, visible at every call site.
- `impl Add` **is** provided — ⊕ is ordinary addition (contract §3.4), and hiding that would be false caution.

### `PoincarePoint` — a point in the hyperbolic 4-ball

```rust
pub struct PoincarePoint([f64; 4]);
```

**Invariant: `‖u‖ < 1`, strictly.** Contract §4 — the boundary sits at infinite distance, so a point with `‖u‖ ≥ 1` is not a point of the space at all. The only constructor is checked; the array is private and never exposed mutably.

Four components: the PRD specifies a hyperbolic 4D lattice. The distance formula is dimension-agnostic (it consumes only norms), so it is written once over the array.

### `Cell` — a pentagon of the `{5,4}` tiling

```rust
pub struct Cell { center: PoincarePoint }
```

Identified by its center. Five edges → five face-neighbours; four cells meet at each vertex.

## Error model, in wave terms

```rust
pub enum LatticeError {
    Unmappable { norm: f64 },      // ‖u‖ ≥ 1 — no coordinate exists
    Dissonant  { product: f64 },   // a·b ≥ 805.56 — ⊗ diverges
}
```

Named for what physically fails, per the doctrine. `Unmappable` is a point outside the space; `Dissonant` is a product whose energy would diverge rather than resonate. Neither is a "validation error" — both are statements about the geometry.

## Interface

### `metric.rs`

```rust
impl LatticeScalar {
    pub fn new(v: f64) -> Self;
    pub fn get(self) -> f64;
    pub fn otimes(self, rhs: Self) -> Result<Self, LatticeError>;   // §3, checked per §3.2
    pub fn otimes_unchecked(self, rhs: Self) -> Self;               // callers proving the domain
    pub fn oslash(self, rhs: Self) -> Result<Self, LatticeError>;   // §3.4, NOT an inverse
}

impl PoincarePoint {
    pub fn new(coords: [f64; 4]) -> Result<Self, LatticeError>;     // enforces ‖u‖ < 1
    pub fn origin() -> Self;
    pub fn norm(&self) -> f64;
    pub fn distance_to(&self, other: &Self) -> f64;                 // d_H, §4
}
```

`otimes_unchecked` exists because the checked form returns `Result` and a hot traversal loop that has already proven its domain should not re-pay for it. It is `unsafe`-adjacent in spirit: documented with the precondition, and never used where input is unvalidated.

### `tessellation.rs`

```rust
pub const SCHLAFLI: (u32, u32) = (5, 4);
pub const VERTEX_DEGREE: u32 = 4;
pub const EDGES_PER_CELL: u32 = 5;

pub fn is_hyperbolic() -> bool;          // (p−2)(q−2) > 4
pub fn interior_angle() -> f64;          // 2π/q
pub fn cell_area() -> f64;               // Gauss–Bonnet
pub fn circumradius() -> f64;            // acosh(cos(π/p) / sin(π/q))
pub fn inradius() -> f64;                // acosh(cos(π/q) / sin(π/p))

impl Cell {
    pub fn at_origin() -> Self;
    pub fn neighbor_count(&self) -> u32;  // == EDGES_PER_CELL
}
```

The closed forms are standard hyperbolic trigonometry for a regular `{p,q}` tiling, derived from the fundamental triangle with angles `π/p, π/q, π/2`. They give **exact** expected values, which is what makes the tessellation testable without generating the full tiling.

## Exact values available for testing

At `{5,4}`, `K = −1`, from the fundamental right triangle with angles `π/p`, `π/q`, `π/2`:

| Quantity | Closed form | Value |
|---|---|---|
| interior angle | `2π/4` | `π/2` exactly |
| cell area | `(p−2)π − p·(2π/q)` | `π/2` exactly |
| **circumradius** (centre→vertex) | `acosh(cot(π/5)·cot(π/4))` | ≈ 0.8424821 |
| **inradius** (centre→edge mid) | `acosh(cos(π/4)/sin(π/5))` | ≈ 0.6268697 |
| **half edge** (vertex→edge mid) | `acosh(cos(π/5)/sin(π/4))` | ≈ 0.5306375 |

Tied together by hyperbolic Pythagoras: `cosh(c) = cosh(a)·cosh(b)`.

> **Correction.** The first version of this table gave the circumradius as `acosh(cos(π/p)/sin(π/q)) ≈ 0.5306` — that is the **half-edge length**, not the circumradius. It produced `inradius > circumradius`, which is impossible in any geometry, and the error was carried into a test that asserted the inversion as "counter-intuitive but correct". Corrected here and pinned by the Pythagoras identity, which would have caught it immediately.

Cell area `= π/2` follows from Gauss–Bonnet at curvature −1. It is an *exact* target, not an empirical one — a strong test.

## Float tolerances required

`03_tests` must pick and justify each:

| Site | Nature |
|---|---|
| `1⊗1 == 2` | **none** — exact per §3.1. Assert `==`. |
| `d(u,u) == 0` | exact — the arcosh argument is exactly 1 |
| symmetry `d(u,v) == d(v,u)` | expect exact; the expression is symmetric in the operands |
| triangle inequality | needs slack — accumulated arcosh error |
| circumradius / inradius | needs slack — transcendental |
| cell area vs `π/2` | needs slack — π representation |
| near-boundary divergence | scale-dependent; grows as `‖u‖ → 1` |

## Slice 2 — tiling generation and neighbour naming

Previously deferred; now built. Adds `isometry.rs` and extends `tessellation.rs`.

### `Isometry` — the hyperboloid model

```rust
pub struct Isometry([[f64; 3]; 3]);        // O(2,1)
pub struct HyperboloidPoint(pub [f64; 3]); // x² + y² − t² = −1, t > 0
```

Computation happens on the hyperboloid, not the disk. Cells crowd exponentially toward `‖u‖ = 1`, so disk coordinates lose absolute precision exactly where the tiling gets interesting; hyperboloid coordinates grow instead, preserving relative precision. Projection to the disk happens only for identity comparison.

Inversion is `J Mᵀ J` with `J = diag(1,1,−1)` — exact for `O(2,1)`, no general matrix inversion needed.

### Edge generators — reflections, necessarily

```rust
gen_k = R(2πk/p) · reflect(inradius) · R(−2πk/p)
```

**The generator must be a reflection, and this is the subtle part.** A rotate-then-translate step moves the correct distance and looks right, but it is not an involution — so stepping across an edge and back does not return, and the enumeration unfolds into a free tree of `5ⁿ` cells instead of closing into a tiling.

`reflect(d) = T(d) ∘ flip_x ∘ T(−d)` is an involution by construction.

### `CellId` — the word problem

A cell is the image of the fundamental pentagon under an isometry, so it is named by a **word in the five edge reflections**. Two words name the same cell exactly when their isometries agree on the origin.

Decided by geometric realisation: project the centre to the disk and quantise at 1e-9. This is sound because distinct cell centres are **provably separated by `2 × inradius ≈ 1.2537`** in the hyperbolic metric — nine orders of magnitude above the quantisation grid, and far above accumulated rounding at any depth this is used for.

### Dimension note

`{5,4}` tessellates **H²**, while the lattice is **H⁴**. The tiling occupies a 2-plane of the 4-ball, embedded with the remaining two coordinates zero. A genuine 4D honeycomb needs a rank-4 Schläfli symbol, which the corpus does not supply — recorded as a gap, not papered over.

### Still deferred

Geodesic path-finding between arbitrary cells, and `⊗`-based address arithmetic over cell coordinates. The tiling now supports both; neither is built.

## Human check

For each type, could it hold a value the axioms forbid?

- `LatticeScalar` — yes, it wraps any `f64`. **Deliberate**: the domain constraint is on the *product* `a·b`, not on either operand, so it cannot be enforced at construction. It is enforced at `otimes`, which is the only place it is well-defined.
- `PoincarePoint` — no. `‖u‖ < 1` is enforced at the single constructor and the field is private.
- `Cell` — no. It holds a `PoincarePoint`, inheriting that invariant.

---
type: design
subsystem: crystallisation
stage: 02_design
derived_from: ["../01_derive/output/math-contract.md"]
---

# Crystallisation — Design

Types and interfaces. Where the contract forbids a value, make it unrepresentable.

## Modules

| File | PRD §8 | Status |
|---|---|---|
| `linguistic.rs` | text / code → harmonic nodes | built |
| `holographic.rs` | images → DFT → Tetryen faces | built |
| `resonant.rs` | media → oscillators | **half** built, deliberately |

## Linguistic

```rust
pub struct HarmonicNode { pub codepoint: char, pub index: usize, pub phase: f64 }

pub struct Crystal {
    nodes: Vec<HarmonicNode>,
    bifurcations: usize,
    extent: f64,
}

impl Crystal {
    pub fn crystallise(text: &str) -> Result<Self, CrystalError>;
    pub fn nodes(&self) -> &[HarmonicNode];
    pub fn bifurcations(&self) -> usize;
    pub fn extent(&self) -> f64;
    pub fn max_bifurcations() -> usize;      // measured, not assumed
}
```

`crystallise` returns `Err(TooDeep)` when the document exceeds the bifurcation ceiling. **There is deliberately no truncating variant** — the contract says refuse, not lose content, and a `crystallise_lossy` would invite exactly that.

`max_bifurcations()` computes the ceiling by iterating ⊗ until the domain refuses, rather than hardcoding 4. If the operator's domain ever changes, the limit follows.

Extent comes from `lattice::LatticeScalar::otimes`. Not restated here.

## Holographic

```rust
pub struct FrequencyMap { height: usize, width: usize, coeffs: Vec<Complex> }

impl FrequencyMap {
    pub fn transform(grid: &PixelGrid) -> Self;      // DFT
    pub fn inverse(&self) -> PixelGrid;              // must round-trip
    pub fn dc(&self) -> f64;                         // == sum of pixels
    pub fn energy(&self) -> f64;                     // Parseval partner
    pub fn project_onto_faces(&self) -> Result<[FaceProjection; 4], CrystalError>;
}

pub struct PixelGrid { height: usize, width: usize, pixels: Vec<f64> }
pub struct FaceProjection { face: usize, coeffs: Vec<Complex> }
```

`[FaceProjection; 4]` is a **fixed-size array**, not a `Vec`. A Tetryen has four faces; a projection onto three or five is not a degenerate projection, it is not a Tetryen projection. The type carries it, as `Tetryen`'s node array does in `gui`.

`project_onto_faces` returns `Err(UnevenProjection)` when the coefficient count is not divisible by 4, rather than silently giving one face extra.

`Complex` is a local two-field struct — pulling a numerics crate for `a + bi` would be more dependency than arithmetic.

## Resonant — half a pipeline, on purpose

```rust
pub struct ResonantChamber { frequency: Frequency, samples: usize }

impl ResonantChamber {
    pub fn from_samples(samples: &[f64], sample_rate: f64) -> Result<Self, CrystalError>;
    pub fn frequency(&self) -> Frequency;
    pub fn energy(&self) -> Joules;          // via symphony-kernel? NO - see below
}
```

Returns `substrate::Frequency`, so the media rate cannot reach the angular carrier path.

**`energy()` is not provided.** Pricing a chamber would need `E = C_H·ν`, which lives in `symphony-kernel` — and this record consumes `lattice` and `substrate` only. Adding a kernel dependency to render media would invert the layering for one multiplication. A caller that wants energy already has `symphony_kernel::energy`.

**No time-crystal type exists.** Contract §1.1: the term has no definition in `_mkb/` and no paper defines it. A stub type would imply a semantics nobody has specified.

## Errors

```rust
pub enum CrystalError {
    TooDeep { bifurcations: usize, limit: usize },
    UnevenProjection { coefficients: usize },
    EmptyMedia,
    MalformedGrid { height: usize, width: usize, pixels: usize },
}
```

Named for what the data did, per doctrine.

## Float tolerances

| Site | Value | Why |
|---|---|---|
| unit bifurcation extent | **exact `2.0`** | A1, bit-exact via ⊗ |
| Parseval | `1e-9` relative | measured `425.0` vs `425.00000000000006` |
| DC = pixel sum | `1e-9` | summation order |
| DFT round trip | `1e-9` | forward + inverse accumulation |
| bifurcation ceiling | none | an integer count |

## Deliberately not built

- **Volumetric time-crystals**, 4D media rotation, "driving physical vibrations" — the first is undefined and the others rest on it.
- **Real image or audio decoding.** `PixelGrid` takes floats; nothing parses PNG or WAV. Codecs are not §8's subject.
- **Rendering the projection.** `FaceProjection` is data; `gui` owns drawing.
- **Wiring into `ftg` transport.** This record is `ftg`'s sibling, not its child — the transform does not need a network.

## Human check

For each type, could it hold a value the axioms forbid?

- `Crystal` — cannot exceed the bifurcation ceiling; no truncating constructor exists.
- `FrequencyMap` — cannot project unevenly; the array type fixes four faces.
- `ResonantChamber` — returns `Frequency`, never `AngularFrequency`.
- There is no time-crystal type to hold anything.

---
type: subsystem-law
layer: law
status: canonical
closes: "The undistilled paper's f(psi_n, psi_{n-1}) placeholder for Tetryen time evolution"
---

# Tetryen Recurrence — discrete time evolution of node states

`_mkb/papers/The neccessity of a finite universe.pdf` names a "governing
equation for Tetryen emergence," `ψ_{n+1} = f(ψ_n, ψ_{n-1})`, with `f`
never defined — a placeholder, not a formula. Evaluated directly against
the rest of that paper before writing anything here: every other equation
in it either restates law that already exists elsewhere with zero new
content (`1×1=2`, the standing-wave form, `E=C_Hν`, the Fourier transform
definition), or — like this placeholder — has no operational content at
all (see [papers/_index.md](papers/_index.md)).

Like [timecrystal.md](timecrystal.md) and [gates.md](gates.md), this is a
**synthesis, not a distillation**: no paper defines this recurrence. It is
built by composing law that already exists, and every step below is
verified rather than asserted.

## What composes it

- **The four Tetryen nodes and their standing-wave envelope** —
  [tetryen.md](tetryen.md)'s node dynamics, `ψ(r) = A·sinh(r/R)·e^(−r/R)`,
  already implemented as `Tetryen::node_amplitude`.
- **The real hyperbolic metric** — `PoincarePoint::distance_to`, geodesic
  distance between two nodes, never a Euclidean chord.
- **A standard second-order discrete oscillator recurrence** — an
  algorithmic choice, not a law citation. No paper or other `_mkb/` file
  specifies a time-stepping scheme, so none is claimed as settled here.

## 1. The uncoupled recurrence — an exact discrete identity

For an oscillator `ψ(t) = A·cos(ωt + φ)` sampled at `t_n = nΔt`:

$$\psi_{n+1} + \psi_{n-1} = 2\cos(\omega \Delta t)\,\psi_n$$

This is the sum-to-product identity `cos(A+B) + cos(A−B) = 2cos(A)cos(B)`
at `A = ωt_n+φ`, `B = ωΔt` — exact algebra, not a physical claim. Verified
to floating-point precision before use: worst observed error `5.7e-14`
over 10,000 samples at `ω=3.0, Δt=0.01`.

## 2. The coupling term — real weights, an explicitly-labelled engineering choice

$$\psi_{n+1,i} = \Bigl[2\cos(\omega\Delta t)\,\psi_{n,i} - \psi_{n-1,i}\Bigr] \;+\; \gamma\,\Delta t^{2}\!\!\sum_{j \ne i}\! \text{node\_amplitude}\bigl(d_H(i,j)\bigr)\,\bigl(\psi_{n,j} - \psi_{n,i}\bigr)$$

The coupling **structure** — nearest-neighbour relaxation toward adjacent
nodes — is a standard technique for coupled-oscillator networks over a
graph, chosen because it is the simplest form respecting the Tetryen's own
topology (all 6 edges of `K₄`). Nothing in `_mkb/` prescribes this
structure; it is not claimed as law, only as a defensible numerical
choice.

**What *is* real law is the weight.** `node_amplitude(d_H(i,j))` is
`tetryen.md`'s own node standing-wave envelope, evaluated at the real
geodesic distance between two nodes — literally "how much of node `j`'s
own wave reaches node `i`'s location." This reuses an existing primitive
exactly where it already fits rather than deriving a parallel one, the
same discipline [gates.md §2](gates.md) and
[timecrystal.md §5.1](timecrystal.md) both already establish.

**A structural fact, verified rather than assumed**: every Tetryen this
workspace can construct (`Tetryen::new`/`Tetryen::at`) is regular —
translations are isometries, so regularity survives them — which means
every pairwise geodesic distance between its four nodes is identical, and
therefore every coupling weight is identical too. Checked directly at
circumradius `0.5`: `d_H(i,j) = 0.827162` and `node_amplitude = 0.404389`
for all 12 ordered pairs.

## 3. Stability — measured, not proven in closed form

No general stability theorem is derived here, unlike
[resonance.md](resonance.md)'s `α < 2/λ_max(L)` bound for the
core-topology load balancer, which *is* a derived bound. This coupling
matrix is different (a fixed 4×4 system on `K₄`, not the core topology),
and deriving its exact spectral bound was not attempted. The safe
operating region was found empirically instead — the same way
`isometry_floor`/`cancellation_floor` are measured elsewhere in this
workspace rather than derived in closed form.

Measured (`ω = 3.0`; seed state arbitrary, not itself physically
meaningful — only its boundedness over time is being checked):

| `Δt` | `γ` | Behaviour |
|---|---|---|
| `0.01` | `0` to `1e4` | bounded, `\|ψ\| < 4` throughout, 5,000–200,000 steps |
| `0.01` | `1e5` and above | **diverges to `inf`** |
| fixed `γ=1` | `Δt` up to `0.5` | bounded |
| fixed `γ=1` | `Δt = 1.0` and above | **diverges to `inf`** |

The instability is real and was found, not merely not-yet-encountered —
confirming `γ` and `Δt` are load-bearing parameters, not decorative ones.

**Execution rule.** Callers must stay well inside the measured-safe
region (`Δt` on the order of `0.01`, `γ` on the order of `1` or smaller).
This is a measured safe region, not a proof of safety at other scales —
stated as such rather than oversold.

## What is *not* claimed

**"Emergence" has no operational definition anywhere in this corpus.**
Searched `tetryen.md` and `timecrystal.md` directly; neither defines a
threshold, criterion, or even an informal description of when a Tetryen
should be considered to have "emerged." A coherence/dissonance gate for
this was proposed in conversation and rejected because it had no basis in
law — inventing one here would repeat exactly that mistake. This file
provides the time-evolution recurrence only. Declaring "emergence" is
recorded as an open gap, not invented.

## Two implementations, one law

This recurrence is implemented twice, deliberately — not duplicated, composed from a shared root:

- **`gui::evolution::TetryenState`** — driven by a caller-supplied `omega`, coupled across a real `Tetryen`'s geodesic node distances.
- **`crystallisation::timecrystal::TetryenRecurrence`** — driven instead by `VolumetricTimeCrystal::fundamental()`, a real Howard-Comma-quantised frequency the crystal already carries, so no arbitrary `omega` is needed. `crystallisation` has no Tetryen geometry of its own, so its `coupling_weight` is caller-supplied — justified by the same regularity fact below (every pairwise weight on a regular Tetryen is identical, so one number loses nothing).

The node envelope both couplings weight by — `node_amplitude(d_H(i,j))` — lives in **`lattice::tetryen_node_envelope`**, not in `gui`. It started in `gui::renderer::Tetryen`; when `crystallisation` needed the same weight and cannot depend on `gui` (dependency direction runs `crystallisation → lattice`, not through `gui`), the fact moved down to the one crate both already depend on. `gui::renderer::Tetryen::node_amplitude` is now a one-line delegation to it — one home per fact, not two copies drifting apart.

## Binds

- [[tetryen]] — `lattice::tetryen_node_envelope`, node geometry, regularity
- [[gui]] — `neos/gui/src/evolution.rs`
- [[crystallisation]] — `neos/crystallisation/src/timecrystal.rs::TetryenRecurrence`

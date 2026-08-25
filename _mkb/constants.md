---
type: constant-set
layer: law
status: canonical
machine_readable: constants.json
---

# Physical Constants Library

All system energy computations and scaling factors derive from these fixed parameters.

**One home per fact:** the values live in [constants.json](constants.json) and **only** there. This file explains what each constant means and where it is used — deliberately without restating the numbers, because a value written twice is a value that will eventually disagree with itself.

To read a value, open the JSON at the key named below. To use one in code, read it through the build; never retype it.

| Symbol | Name | JSON key | Governs |
|---|---|---|---|
| $C_H$ | Howard Comma Constant | `howard_comma` | computational energy per unit angular frequency |
| $\omega_c$ | Baseline Carrier Frequency | `baseline_carrier_frequency` | hypervisor resting clock synchronization |
| $K$ | Hyperbolic Curvature Parameter | `hyperbolic_curvature` | negative spatial curvature of the 4D lattice |

Also in the JSON, with no prose section here because they are documented elsewhere:

| JSON group | Holds | Documented in |
|---|---|---|
| `logic_phases` | the $\pm\pi/2$ pair | [axioms.md § A2](axioms.md#a2--logic-gate-override) |
| `thresholds` | teardown and link-stability angles | [equations.md](equations.md) |
| `tessellation` | the $\{5,4\}$ pair and vertex degree 4 | [reconciliation.md § R3](reconciliation.md#r3--vertex-degree-of-the-tessellation--resolved) |
| `operators` | $\lambda$, the ⊗ domain limit, non-associativity flags | [operators.md](operators.md) |
| `scales` | $l_P$, lattice scale $R$, fractal dimension | [reconciliation.md](reconciliation.md) R2 / R4 |

## Unit convention

All lattice code works in **lattice-native units where $R = 1$**, which is what makes $K = -1/R^2 = -1$. The physical embedding value for $R$ is recorded in the JSON as `physical_embedding` and is used only at conversion boundaries. See [reconciliation.md § R2](reconciliation.md#r2--curvature-k--resolved).

## The Howard Comma Constant — $C_H$

Defines the ratio of computational energy to **ordinary frequency**: $E = C_H\nu$, with $C_H = h/\sqrt{2\pi}$.

Neither $h$ nor $\hbar$ — the $1/\sqrt{2\pi}$ normalisation is a deliberate departure from Planck, giving $0.3989\times$ the Planck energy for the same process. See [reconciliation.md § R5a](reconciliation.md#r5a--c_h-redefined-supersedes-the-earlier-value).

Used by [equations.md § Howard Equation](equations.md#quantum-energy-quantization-the-howard-equation) for all resource accounting. Every live thread, process, and connection has an energy cost derived from this constant — scheduling and garbage collection both read it.

> **⚠ $\nu$, never $\omega$.** They differ by $2\pi$ and **the units cannot distinguish them** — both are J·s × s⁻¹ = J. The two are separate types in code (`Frequency` / `AngularFrequency` in `substrate`), so a substitution is a compile error rather than a silent scaling bug.

**Binds:** [[symphony-kernel]] (quantization, GC)

## Baseline Carrier Frequency — $\omega_c$

The resting clock synchronization parameter for the Rust hypervisor substrate. 1 GHz harmonic baseline.

This is the carrier onto which FTG Layer 1/2 synthesizes phase-shifted binary input.

**Angular**, unlike $C_H$'s partner above. It belongs to wave synthesis and must never be substituted into $E = C_H\nu$. `substrate`'s `build.rs` asserts its declared units are `rad/s` and fails the build otherwise.

**Binds:** [[substrate]] (clock), [[ftg]] (carrier synthesis)

## Hyperbolic Curvature Parameter — $K$

Dictates the negative spatial curvature of the 4D lattice. The chosen value is standard Lobachevskian hyperbolic plane mapping, which is what makes the Poincaré disk distance formula in [equations.md](equations.md#hyperbolic-distance-function) valid as written.

Changing this parameter invalidates the distance function as stated. Do not change it without re-deriving. The JSON records this as a `constraint` field on the key.

**Binds:** [[lattice]] (tessellation, metric), [[ftg]] (Layer 3/4 routing)

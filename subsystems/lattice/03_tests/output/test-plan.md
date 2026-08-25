---
type: test-plan
subsystem: lattice
stage: 03_tests
derived_from: ["../02_design/output/design.md", "../01_derive/output/math-contract.md"]
doctrine: _mkb/test-doctrine.md
---

# Lattice — Test Plan

Assertions proving `neos/lattice` obeys its math contract. Written before implementation.

**Every value below was computed and verified before this plan was written** — the expected numbers are not predictions.

Target file: `neos/tests/lattice_metric.rs`.

## The doctrine question

Applied to every assertion: *would this still pass against a conventional Euclidean implementation?* If yes, it is not testing NEOS. Assertions marked **[D]** are the ones that would fail against a classical implementation — they are what make this suite meaningful rather than decorative.

## Group 1 — the ⊗ operator

| # | Assertion | Expected | ε | Justification |
|---|---|---|---|---|
| 1.1 **[D]** | `1 ⊗ 1 == 2` | `2.0` | **none — exact `==`** | `sinh(arcsinh(1)) = 1` identically. Verified bit-exact. An ε here would hide a regression. This is axiom A1 and the single most important assertion in the suite. |
| 1.2 **[D]** | `1 ⊗ 1 != 1` | — | none | A classical implementation gives 1. This is the doctrine check made explicit. |
| 1.3 | `2 ⊗ 3` | `104.99494936611664` | `1e-12` relative | Hand-computed: `6 + sinh(6λ)`. Relative because the magnitude is ~10². |
| 1.4 **[D]** | ⊗ is **not** associative | `(2⊗3)⊗4` and `2⊗(3⊗4)` differ beyond any plausible tolerance | n/a | Contract §3.3. `(2⊗3)⊗4 = 2.86e160`; `2⊗(3⊗4)` overflows. Asserting *non*-associativity is deliberate: a passing associativity test would mean ⊗ was implemented as ordinary multiplication. |
| 1.5 | domain guard rejects `a·b ≥ 805.5607865456228` | `Err(Dissonant)` | none | Contract §3.2. Test at 806 (reject) and 805 (accept). |
| 1.6 | `otimes` never returns a non-finite value on `Ok` | `is_finite()` | none | The guard's whole purpose. If this fails the bound is wrong. |

**Note on 1.4:** because `2⊗(3⊗4)` overflows to `+inf`, the assertion is that one side is finite and the other is not — a cleaner statement than comparing two huge numbers.

## Group 2 — hyperbolic distance

| # | Assertion | Expected | ε | Justification |
|---|---|---|---|---|
| 2.1 | `d(u,u) == 0` | `0.0` | **none — exact** | The arcosh argument is exactly 1 when `u = v`, and `acosh(1) = 0` exactly. Verified. |
| 2.2 | symmetry `d(u,v) == d(v,u)` | — | **none — exact** | The expression is symmetric in its operands; `‖u−v‖²` is order-independent. Verified exact. |
| 2.3 | `d(origin, [0.5,0,0,0])` | `1.0986122886681096` | `1e-15` absolute | Closed form `2·atanh(0.5) = ln 3 = ...098`. The arcosh route gives `...096` — **2 ulp apart**. Measured, not assumed; ε set just above the observed gap. |
| 2.4 | triangle inequality | `d(u,w) ≤ d(u,v) + d(v,w)` | `1e-12` slack | Accumulated arcosh error across three evaluations. Slack is on the permissive side so a true violation still fails. |
| 2.5 **[D]** | boundary divergence | `d` grows without bound as `‖u‖ → 1` | n/a | Verified: `0.9 → 2.944`, `0.99 → 5.293`, `0.999 → 7.600`, `0.9999 → 9.903`. Assert strict monotonic increase and that `0.9999` exceeds 9.9. **A Euclidean metric stays bounded by 1** — this is the sharpest doctrine discriminator in the suite. |
| 2.6 **[D]** | `d` exceeds the Euclidean distance for the same coordinates | `d_H(a,b) > ‖a−b‖` | none | `d_H = 1.6807` vs Euclidean `0.7071` for the sample pair. Fails immediately if someone substitutes a Euclidean metric. |

## Group 3 — constructor invariants

| # | Assertion | Expected | ε | Justification |
|---|---|---|---|---|
| 3.1 | `PoincarePoint::new` rejects `‖u‖ = 1.0` | `Err(Unmappable)` | none | Contract §4 — the boundary is at infinite distance, not in the space. |
| 3.2 | rejects `‖u‖ > 1` | `Err(Unmappable)` | none | Same. |
| 3.3 | accepts `‖u‖` just below 1 | `Ok` | none | The invariant must not be over-tight. Test at 0.9999. |
| 3.4 | rejects NaN and infinite coordinates | `Err(Unmappable)` | none | `NaN` fails every comparison, so a naive `< 1.0` check would **accept** it. Explicitly targeted. |

## Group 4 — tessellation

| # | Assertion | Expected | ε | Justification |
|---|---|---|---|---|
| 4.1 | `VERTEX_DEGREE == 4` | `4` | none | Reconciliation R3 — the decision that resolved the paper's self-contradiction. Asserted so it can never silently revert to 5. |
| 4.2 | `SCHLAFLI == (5,4)` | — | none | Topology identity. |
| 4.3 **[D]** | `is_hyperbolic()` | `true`, `(p−2)(q−2) = 6 > 4` | none | Euclidean tilings give exactly 4; spherical give less. Verified 6. |
| 4.4 | interior angle | `π/2` | **exact** | `2π/4`. Verified exactly representable. |
| 4.5 **[D]** | cell area | `π/2 ≈ 1.5707963267948966` | `1e-15` | Gauss–Bonnet at `K = −1`: `(p−2)π − p·(2π/q) = 3π − 2.5π`. **A Euclidean pentagon has no curvature-determined area at all** — this assertion is meaningless outside hyperbolic space, which is exactly why it belongs here. |
| 4.6 | circumradius (centre→vertex) | `0.842482081462008` | `1e-12` | `acosh(cot(π/5)·cot(π/4))`, verified. |
| 4.7 | inradius (centre→edge mid) | `0.626869662906178` | `1e-12` | `acosh(cos(π/4)/sin(π/5))`, verified. |
| 4.7b | half edge (vertex→edge mid) | `0.5306375309525176` | `1e-12` | `acosh(cos(π/5)/sin(π/4))`, verified. |
| 4.8 | `inradius < circumradius` | `0.6269 < 0.8425` | none | Centre-to-edge cannot exceed centre-to-vertex in any geometry. **This row previously asserted the opposite** and rationalised it as "counter-intuitive but correct" — see the correction note below. |
| 4.9 | hyperbolic Pythagoras | `cosh(c) = cosh(a)·cosh(b)` | `1e-12` | Ties all three radii together. This is the relation that would have caught the original error immediately, so it is now pinned. |

> **Correction to 4.6/4.8.** The original plan gave the circumradius as `acosh(cos(π/p)/sin(π/q)) ≈ 0.5306`, which is the half-edge length. The consequent `inradius > circumradius` was geometrically impossible and should have prompted a check rather than a justifying comment. Fixed; the Pythagoras identity is now a permanent guard.

## Group 5 — one home per fact

| # | Assertion | Justification |
|---|---|---|
| 5.1 | no numeric constant from `constants.json` appears literally in any `.rs` file | Verified by `grep` in the verification step, not by a Rust test. Contract §7. |

## Tolerance summary

Six assertions require **no tolerance at all** — 1.1, 1.2, 2.1, 2.2, 4.4, plus the structural ones. That is unusually strong, and it is a consequence of the R1 pinning decision: choosing `λ = arcsinh(1)` made the headline axiom exact rather than approximate.

No tolerance in this plan was chosen by reaching for `f64::EPSILON`. Each is either measured (2.3), justified by accumulated operation count (2.4), or set to the transcendental representation floor (4.5–4.7).

## Group 6 — tiling generation and neighbour naming (slice 2)

Target file: `neos/tests/lattice_tiling.rs`. All values verified before writing.

| # | Assertion | Expected | ε | Justification |
|---|---|---|---|---|
| 6.1 | edge generators are involutions | `g² = I` | `1e-12` | The property a rotate-then-translate step lacks. Its absence is what turns a tiling into a `5ⁿ` free tree. |
| 6.2 | generators step one separation | `2·inradius` | `1e-12` | — |
| 6.3 | five distinct neighbours per cell | 5 unique ids, none self | none | Naming must be injective across edges. |
| 6.4 **[D]** | edge round trip returns home | `nbr(k).nbr(k) == self` | none | Direct consequence of 6.1. Fails loudly under the free-tree bug. |
| 6.5 | adjacency is symmetric | no asymmetric pair | none | ≥1000 interior adjacencies sampled at depth 5. |
| 6.6 | neighbours one separation away | `2·inradius` | `1e-9` | — |
| 6.7 | distinct cells well separated | min = `2·inradius` ≈ 1.2537 | `1e-6` | **The soundness argument for the word problem.** Identity-by-centre is safe because genuine cells are ~1.25 apart while the quantisation grid is 1e-9. |
| 6.8 | equal words → equal names | round trip = `CellId::ORIGIN` | none | The decision procedure, stated directly. |
| 6.9 **[D]** | ring sizes = `5·Fib(2n)` | `[1,5,15,40,105,275,720,1885]` | **none — exact integers** | An exact integer identity in a geometry suite. No Euclidean `{5,4}` exists at all. |
| 6.10 | recurrence `a(n) = 3a(n−1) − a(n−2)` | — | none — integers | The structure behind 6.9. |
| 6.11 **[D]** | growth → `φ² = 2.618033988749895` | ratio → φ² | `1e-3` | Exponential ring growth is the signature of negative curvature; a Euclidean tiling tends to 1. |
| 6.12 **[D]** | four cells meet at a vertex | 4 | `1e-6` on radius | **Independently confirms reconciliation R3** from the group action alone, without reading `VERTEX_DEGREE`. |
| 6.13 | centres embed in the 4-ball | `‖u‖ < 1` | none | — |
| 6.14 | totals match ring sums | — | none | Enumeration bookkeeping. |

## Deliberate omissions

- Geodesic path-finding between arbitrary cells, and ⊗-based address arithmetic over cell coordinates. The tiling supports both; neither is built.
- `oslash` and ⊗-powers are defined but unused; they get construction tests only, not behavioural ones. Testing an operator no caller exercises would be theatre.

## Human check

Read Group 2.5 and 4.5 first. Those two are the assertions a conventional Euclidean implementation cannot pass under any tolerance — if the suite is meaningful, it is because of them.

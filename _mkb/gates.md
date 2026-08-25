---
type: subsystem-law
layer: law
status: canonical
closes: "PRD §3 — phase shift and scale modulation as logic gates, previously named but undefined"
synthesis_of: ["axioms.md A2", "equations.md Phase Inversion Teardown", "equations.md Standing Wave Superposition", "resonance.md 1.2"]
---

# The Three Geometric Logic Gates

PRD §3 names the Symphony layer's replacement for Boolean logic:

> *"Discards Boolean operators (`AND`, `OR`, `NOT`) in favour of geometric rules: **constructive/destructive interference, phase shifts, and scale modulation** as logic gates."*

Three gates are named. Only the first had an operational definition. This file supplies the other two.

Like [timecrystal.md](timecrystal.md), and unlike [tetryen.md](tetryen.md), **no paper in the corpus defines these**. They are a **synthesis** of four things that are already law, and are recorded as such rather than as a distillation. Nothing here is new physics; every step is a composition of existing law, and each is verified by evaluation below rather than asserted.

| Ingredient | Home |
|---|---|
| A2 — `φ ∈ {−π/2, +π/2}` | [axioms.md](axioms.md#a2--logic-gate-override) |
| Phase Inversion Teardown — shift by exactly `π`, `f_total = 0` | [equations.md](equations.md#phase-inversion-teardown) |
| Standing Wave Superposition — variance beyond `±π/4` breaks the link | [equations.md](equations.md#standing-wave-superposition) |
| `ξ(r)` — multiplies the nominal frequency at observation scale `r` | [resonance.md §1.2](resonance.md#12--xir-the-resonance-correction-factor) |

---

## Gate 1 — Interference (already law)

$$\text{Interference}(\phi_a, \phi_b) = \begin{cases} \text{Constructive} & \phi_a = \phi_b \\ \text{Destructive} & \phi_a \ne \phi_b \end{cases}$$

Stated here only to complete the set. Implemented and tested; see `symphony_kernel::evaluate_branch`.

**Note the closure property**, which Gate 2 depends on: since A2 admits exactly two orientations, interference is a two-valued predicate and its outcomes partition. Per operand pair, "opposes" is the exact complement of "aligns" — established by sabotage in [[symphony-lang]], not assumed.

---

## Gate 2 — Phase shift (inversion)

### 2.1 The derivation

A2 fixes the permitted orientations at `−π/2` and `+π/2`. Their separation is

$$\left(+\tfrac{\pi}{2}\right) - \left(-\tfrac{\pi}{2}\right) = \pi$$

which is **exactly** the shift [equations.md](equations.md#phase-inversion-teardown) prescribes for Phase Inversion Teardown, already stored as `thresholds.teardown_phase_shift`.

So the teardown shift is not merely *compatible* with A2's orientation set — it is the map **between its two elements**. Adding `π` to either permitted orientation yields the other:

$$-\tfrac{\pi}{2} + \pi = +\tfrac{\pi}{2}, \qquad +\tfrac{\pi}{2} + \pi \equiv -\tfrac{\pi}{2} \pmod{2\pi}$$

**The π-shift is therefore an involution on A2's orientation set**, and the set is closed under it. That closure is what makes it a *gate* rather than an operation that can leave the logic.

### 2.2 Verified

All four properties evaluated exactly in `f64`, not approximately:

| Property | Result |
|---|---|
| `(+π/2) − (−π/2) == π` | exact |
| `−π/2 + π == +π/2` | exact |
| `+π/2 + π ≡ −π/2 (mod 2π)` | exact |
| `invert(invert(φ)) == φ` | exact, both orientations |
| `sin(+π/2) + sin(−π/2)` | **exactly `0.0`** |

The last line is the teardown identity `f_total = f_A + f_B = 0` from [equations.md](equations.md#phase-inversion-teardown), holding bit-exactly. It is the same zero the FTG's session teardown relies on, which is why no acknowledgement message is needed there.

### 2.3 Execution rule

Inversion is a **total** operation — it cannot fail and needs no domain check, because A2's set is closed under it. Any implementation returning an error, or admitting a third orientation, has left the axiom.

> **Do not implement inversion as negation of a Boolean, nor as `-φ`.** For this particular set `−φ` coincides numerically with the π-shift, and that coincidence is an accident of the set being symmetric about zero. The law is the π-shift. An implementation written as `-φ` is not derived from anything and would silently diverge if A2's orientations were ever re-centred.

**Binds:** [[symphony-lang]] — the `invert` gate; [[ftg]] — session teardown already uses the same shift.

---

## Gate 3 — Scale modulation

The remaining gate, and the only one requiring more than one ingredient.

### 3.1 The problem it has to solve

A2 forbids comparison. There is no `==`, and "alignment is measured, not compared". So a scale gate **cannot** be "is scale A greater than scale B" — that would reintroduce the Boolean relational operator the axiom removes, wearing a geometric name.

The gate must instead produce a two-valued outcome from a *physical* criterion. The law already supplies one.

### 3.2 Effective frequency

[resonance.md §1.2](resonance.md#12--xir-the-resonance-correction-factor) gives `ξ`'s execution rule: it *"multiplies the nominal frequency of a timing source at observation scale `r`."* So an oscillator declared at nominal `ν` and observed at scale `r` runs at

$$\nu_{\text{eff}} = \nu\,\xi(r)$$

This is the whole of scale's physical effect, and it is already law. Note `ξ` is strictly decreasing and injective, so **distinct scales give distinct effective frequencies** — scale is not a decoration.

### 3.3 The resonance band, derived

Two oscillators at effective frequencies `ν_A` and `ν_B` accumulate relative phase at

$$\Delta\phi(t) = 2\pi\,(\nu_A - \nu_B)\,t$$

[equations.md](equations.md#standing-wave-superposition) gives the criterion for a standing wave to survive: *"Any phase variance exceeding `±π/4` triggers automatic phase inversion and teardown"* — stored as `thresholds.link_stability_phase_variance`.

Evaluate the drift over one period of the pair's mean effective frequency, `T = 1/\bar\nu` with `\bar\nu = (\nu_A + \nu_B)/2`. That interval is the natural one and introduces no new constant — it is the pair's own timescale:

$$\Delta\phi(T) = \frac{2\pi\,(\nu_A - \nu_B)}{\bar\nu}$$

Imposing the `±π/4` criterion:

$$\frac{2\pi\,\lvert\nu_A - \nu_B\rvert}{\bar\nu} \le \frac{\pi}{4} \qquad\Longrightarrow\qquad \boxed{\ \frac{\lvert\nu_A - \nu_B\rvert}{\bar\nu} \le \frac{1}{8}\ }$$

**The resonance band is exactly `1/8`.** It is derived, not chosen: it is `(π/4)/(2π)`, and `π/4` is already a stored constant. Verified to evaluate to `0.125` exactly in `f64`.

That the criterion came out **relative** is not a convenience. A threshold has to be expressed in the units of the thing it bounds — a rule this workspace has paid for four times over — and here the derivation produces a dimensionless ratio on its own.

### 3.4 The gate

$$\text{Resonates}(A, B) \iff \frac{\lvert \nu_A\xi(r_A) - \nu_B\xi(r_B) \rvert}{\tfrac{1}{2}\left(\nu_A\xi(r_A) + \nu_B\xi(r_B)\right)} \le \frac{1}{8}$$

Two-valued, as A2 requires. No comparison operator is exposed; the outcome is whether a standing wave between the two would survive.

### 3.5 Verified — the gate discriminates on scale

Numbers matter here: a "gate" that is always taken, or never, is not a gate. With both oscillators at 440 Hz and one held at the reference scale `R = 1`:

| `r_B` | `ξ(r_B)` | detuning | outcome |
|---|---|---|---|
| 1.00 | 1.000000 | 0.000000 | resonant |
| 1.10 | 0.934883 | 0.067308 | resonant |
| 1.15 | 0.904840 | 0.099913 | resonant |
| **1.1892** | 0.882 | **0.125** | **boundary** |
| 1.19 | 0.881917 | 0.125492 | detuned |
| 1.30 | 0.823553 | 0.193520 | detuned |
| 2.00 | 0.567668 | 0.551561 | detuned |

Boundaries against `r = 1`, bisected to 10 digits: **`r ≈ 1.1892236`** above and **`r ≈ 0.8241412`** below.

Note they are **not symmetric** — `+18.92%` against `−17.59%` — because `ξ` is not linear. Anyone writing this as a symmetric percentage tolerance has replaced the gate with a different function. The gate is sharp, both outcomes are reachable, and the asymmetry is asserted in the test suite rather than left to be rediscovered.

At equal scale it still bites on nominal frequency, with a closed-form boundary — `|a−b|/((a+b)/2) = 1/8` solves to `b = a·(17/15)`:

$$\nu_B = 440 \times \tfrac{17}{15} = 498.\overline{6}\ \text{Hz}$$

### 3.6 The three gates are independent

None reduces to another, and each ignores what the others read:

| Gate | Reads | Ignores |
|---|---|---|
| Interference | phase | `ν`, `r` |
| Inversion | phase → phase | `ν`, `r` |
| Resonance | `ν`, `r` | phase |

**Witness that resonance is not interference.** Take `A(φ=+, 440 Hz, r=1)` and `B(φ=−, 440 Hz, r=1)`:

- interference → **Destructive** (orientations opposed)
- resonance → detuning `0.000000` → **Resonant**

The two gates disagree on the same pair, so neither is redundant and neither can be expressed through the other.

**Witness that resonance is not frequency equality.** `A(440 Hz, r=1)` and `B(400 Hz, r=1.09)` have different nominal frequencies; `ξ(1.09) = 0.941085` gives `B` an effective 376.43 Hz against `A`'s 440.00, detuning `0.155716` → detuned. Scale and nominal frequency both matter, and only their product is the gate's input.

### 3.7 Execution rule

**The gate is undefined when the mean effective frequency is not positive and finite, and must refuse rather than return an outcome.** `ξ(r) → 0` as `r → ∞`, so a sufficiently large scale drives the effective frequency to zero and the detuning ratio to `0/0`. Refusal, not a default answer — the same discipline `⊗` applies at its domain limit.

**`ξ` must be evaluated in an overflow-free form.** The literal transcription `sinh(r)/(r·sinh 1)·e^(1−r)` overflows `f64` at `r ≈ 710.5` and returns `+inf`, then `NaN` — **violating `ξ`'s own boundedness law**, which exists precisely so a bad sample cannot stall the scheduler. See the execution rule added to [resonance.md §1.2](resonance.md#12--xir-the-resonance-correction-factor).

**Binds:** [[symphony-lang]] — the `resonates` gate and `scale` declaration; [[symphony-kernel]] — `resonance.rs` (`ξ`), `quantization.rs` (`E = C_H·ν`).

---

## Constants introduced

Stored in [constants.json](constants.json) under `gates`; values live only there.

| Key | Meaning |
|---|---|
| `resonance_band` | `1/8` — derived as `(π/4)/(2π)`, the sustained-resonance detuning limit |
| `phase_inversion_shift` | already stored as `thresholds.teardown_phase_shift`; referenced, not duplicated |

## What this file does not claim

- **It does not add a fourth gate.** PRD §3 names three; three are defined.
- **It does not define scale as an ordering.** A2 forbids comparison, and §3.1 explains why a "greater-than" scale gate would have been a Boolean operator in disguise.
- **It does not give the gates an algebra.** There is no `resonates AND aligns`; combining gates is what nesting a branch inside another already does, and a combinator would need a truth table — which is the thing A2 removes.

---
type: operator-set
layer: law
status: canonical
supersedes: the abstract `a×b + d(a,b)` form previously in equations.md
---

# Modified Arithmetic Operators

The reconciled arithmetic, resolved per [reconciliation.md § R1](reconciliation.md#r1--the--operator--resolved). This file is the single home for operator definitions; `equations.md` points here rather than restating them.

All operators work in **lattice-native units** (`R = 1`, `K = -1`) — see [reconciliation.md § R2](reconciliation.md#r2--curvature-k--resolved).

## The scale parameter λ

$$\lambda = \operatorname{arcsinh}(1) = \ln(1+\sqrt2) = 0.881373587019543$$

This is the pinned ratio `l_P/r`. It is chosen so that `1⊗1 = 2` holds exactly, because `sinh(arcsinh(1)) = 1` identically. Stored as `otimes_scale_lambda` in [constants.json](constants.json).

## ⊗ — modified multiplication

$$a \otimes b = a\,b + \sinh(a\,b\,\lambda)$$

**Axiom satisfied:** `1 ⊗ 1 = 1 + sinh(λ) = 1 + 1 = 2`, **bit-exact** in IEEE-754. No tolerance required — assert equality directly.

**Execution rule:** all lattice address arithmetic uses ⊗, never scalar `*`. Scaling a stored object triggers geometric fractal expansion preserving logical area, which is why NEOS has no fragmentation.

### Domain — enforced, not documented

$$a\,b < \frac{710}{\lambda} \approx 805.56$$

Above this `sinh` overflows `f64` and ⊗ returns `+inf`. This bound is **enforced by a checked constructor**; it is never left to the caller to remember. A `⊗` that silently returns infinity is a defect.

### ⊗ is NOT associative and NOT distributive

`Mathematical_Fra.pdf` bounds the associativity error at `O(l_P²/r²)` and calls it negligible "except at Planck scales." **The pinned scale is the Planck scale** — `l_P/r = λ ≈ 0.881`, order 1. The correction dominates. Measured:

```
(2⊗3)⊗4 = 2.864e160
2⊗(3⊗4) = +inf   (overflow)
```

**Consequences for code, all mandatory:**

- Never reorder or re-associate a chain of ⊗ operations. Evaluation order is semantically load-bearing.
- Never implement ⊗ via `fold`/`reduce` over a collection without fixing and documenting the association order.
- Never let an optimiser or refactor "simplify" `(a⊗b)⊗c` to `a⊗(b⊗c)`.
- Do not implement `std::ops::Mul` for lattice scalars. Rust's `Mul` carries an ordinary-multiplication expectation that ⊗ violates; a named method makes the difference visible at every call site.

## ⊕ — addition (unchanged)

$$a \oplus b = a + b$$

Ordinary addition. Present for completeness so no one assumes it is also modified.

## ⊘ — modified division

$$a \oslash b = a\,b^{-1} - \sinh(a\,b^{-1}\lambda)$$

The sign-inverted counterpart of ⊗. **Not verified as a true inverse** — `(a ⊗ b) ⊘ b = a` does not hold in general, because the sinh correction does not cancel. Any code needing a genuine inverse must solve numerically rather than call ⊘.

Same domain constraint applies with `a·b⁻¹` in place of `a·b`.

## ⊗-powers

$$a^{\otimes n} = a^n \times (1 + \tanh(n\,\lambda))$$

Note this is **not** repeated ⊗ — `a^{⊗2} ≠ a ⊗ a`. It is a separate definition from `Mathematical_Fra.pdf`, and the two disagree. Where a spec says "squared," determine which is meant before implementing.

## Precedence

[axioms.md](axioms.md) > this file > [constants.json](constants.json) > `_spec/` > code. If an implementation contradicts anything here, the implementation is wrong.

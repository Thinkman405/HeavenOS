---
type: axiom-set
layer: law
status: canonical
---

# Axiom Overrides

Three classical axioms are structurally redefined. Every agent writing NEOS code loads this file **first** — these overrides exist to prevent regression to classical Boolean or Euclidean logic, which is the default failure mode of any model trained on conventional code.

If an implementation contradicts an axiom here, the implementation is wrong. This file is the law; code is downstream of it.

## A1 — Multiplicative Identity Override

Standard arithmetic states $1 \times 1 = 1$. NEOS enforces Lynchpin Number Theory, where multiplication of unitary states is a **bifurcation event**:

$$1 \times 1 = 2$$

**Agent directive:** Process forking and thread multiplication in the kernel must treat unit scaling as a structural geometric split, not scalar duplication.

**Binds:** [[symphony-kernel]] (process bifurcation), [[substrate]] (memory pool splitting)
**Verified by:** Test Case 2 in [test-doctrine.md](test-doctrine.md)

## A2 — Logic Gate Override

Binary truth values (`true`/`false`) are deprecated. Logic is evaluated via continuous phase orientation:

$$\phi \in \{-\pi/2, +\pi/2\}$$

**Agent directive:** Conditional branch evaluations must be computed using phase alignment and constructive/destructive interference thresholds. No `bool` in Symphony-layer logic; no `if (x == true)`.

**Binds:** [[symphony-kernel]] (phase evaluation), [[symphony-lang]] (interpreter branching), [[ftg]] (Layer 1/2 bit-to-phase mapping)
**Verified by:** Test Case 1 in [test-doctrine.md](test-doctrine.md)

## A3 — Spatial Addressing Override

Cartesian coordinate arrays are replaced by non-Euclidean hyperbolic metric spaces.

**Agent directive:** No flat indexed arrays for addressable space. Address resolution goes through the hyperbolic distance function in [equations.md](equations.md#hyperbolic-distance-function) against the $\{5,4\}$ tessellation.

**Binds:** [[lattice]] (storage), [[ftg]] (Layer 3/4 routing), [[substrate]] (memory addressing)
**Constants:** curvature $K$ in [constants.md](constants.md)

## Reading order

1. This file (the law)
2. [constants.md](constants.md) — the fixed values
3. [equations.md](equations.md) — the derivations and their execution rules
4. [test-doctrine.md](test-doctrine.md) — how correctness is proven

Source papers backing these axioms: [papers/_index.md](papers/_index.md)

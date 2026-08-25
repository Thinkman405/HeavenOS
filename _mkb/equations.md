---
type: equation-set
layer: law
status: canonical
---

# Core Equations and Execution Rules

Each equation carries an **execution rule** — the thing that makes it code rather than notation. When implementing a subsystem, the `01_derive` stage copies the relevant equations and their execution rules into that subsystem's `math-contract.md`; it does not paraphrase them.

Constants referenced here are defined in [constants.md](constants.md) / [constants.json](constants.json).

## Quantum-Energy Quantization (The Howard Equation)

$$E = C_{H}\nu, \qquad C_H = \frac{h}{\sqrt{2\pi}}$$

**Definition:** The computational energy $E$ required to maintain any active thread, process, or network connection is directly proportional to its **ordinary frequency** $\nu$.

**Execution rule:** Garbage collection routines must monitor $\nu$. When $\nu \to 0$, $E \to 0$, and the memory vector is instantly unmapped.

**Consequence for scheduling:** high-priority tasks are assigned higher $\nu$ and therefore draw proportional computational energy. Priority is not a queue position — it is an energy state.

> **⚠ $\nu$, never $\omega$.** They differ by $2\pi$ and the units do not distinguish them. Keep them as separate types. $\omega_c$ (`baseline_carrier_frequency`) is angular and belongs to wave synthesis — it must never be substituted here. See [reconciliation R5a](reconciliation.md#r5a--c_h-redefined-supersedes-the-earlier-value).

**Binds:** [[symphony-kernel]] — `neos/symphony/kernel/src/quantization.rs`, `neos/symphony/kernel/src/scheduler.rs`

## Harmonic Force Equilibrium

$$\nabla \cdot \mathbf{E} = \frac{\rho}{\epsilon_0}$$

**Definition:** the self-stabilisation field relation. `ρ` is task density (per-core load **relative to mean**), `E` the processing capacity field over the core topology, `ε₀` the coupling governing how strongly load gradients drive migration.

**Execution rule:** with `E = −∇φ` this is Poisson's equation, discretised on the core-topology graph as `Lφ = −ρ/ε₀`. Load flows down the gradient of `φ`. Two constraints are mandatory and derive from the mathematics, not from choice:

- `Σρᵢ = 0` — mean-centre the load, or the system has no solution
- `α < 2/λ_max(L)` — derive the coupling from topology, or the balancer oscillates

At equilibrium `ρ → 0` and the field goes source-free. Full treatment and verified convergence in [resonance.md § Part 2](resonance.md#part-2--harmonic-force-equilibrium-as-load-balancing).

**Binds:** [[symphony-kernel]] — `neos/symphony/kernel/src/scheduler.rs`

## Resonance Correction

$$\xi(r) = \frac{\sinh(r/R)}{(r/R)\,\sinh(1)}\;e^{\,1-r/R}$$

**Definition:** the dimensionless clock-jitter damping factor, unity at the reference scale `r = R`. The `sinh(x)/x` factor removes the singularity at `r → 0`.

**Execution rule:** multiplies the nominal frequency of a timing source at observation scale `r`. Strictly decreasing and **bounded above by `e/sinh(1) = 2.3130352854993315`** — the correction can never diverge, which is what makes it safe in a clock path. Never gates correctness: failing to apply `ξ` degrades precision, it does not produce a wrong result.

**Health invariant:** `H(κ) = ∫γ(κ,t')dt' → 0`. If `|H(κ)|` grows instead of tending to zero, the clock domain has diverged. Monitor it; do not consume it as an input.

**Binds:** [[substrate]] — `neos/substrate/src/clock.rs`; [[symphony-kernel]] — `neos/symphony/kernel/src/resonance.rs`

## Hyperbolic Distance Function

$$d_{\mathbb{H}}(\mathbf{u}, \mathbf{v}) = \operatorname{arcosh}\left(1 + \frac{2\Vert{}\mathbf{u} - \mathbf{v}\Vert{}^2}{(1 - \Vert{}\mathbf{u}\Vert{}^2)(1 - \Vert{}\mathbf{v}\Vert{}^2)}\right)$$

**Definition:** Exact geodesic distance between two nodes $\mathbf{u}$ and $\mathbf{v}$ inside the Poincaré disk model of the $\{5,4\}$ hyperbolic tessellation.

**Execution rule:** Used exclusively by the Fourier Transform Gateway for Layer 3 network packet routing and memory address resolution.

**Validity:** holds as written only while $K = -1.0$.

**Binds:** [[ftg]] — `layers_3_4`; [[lattice]] — `metric`

## Curved Addressing (Modified Multiplication)

**Moved.** The abstract form `a ⊗ b = a × b + d(a,b)` that lived here was under-specified — with any metric it gives `1⊗1 = 1`, contradicting axiom [A1](axioms.md#a1--multiplicative-identity-override).

The reconciled operator, its pinned scale parameter, its enforced domain, and the non-associativity constraint now live in **[operators.md](operators.md)**. One home per fact.

**Binds:** [[lattice]] — `metric`, `tessellation`

## Standing Wave Superposition

$$f(t) = 2A \sin(k x) \cos(\omega_{sync} t)$$

**Definition:** Steady-state waveform of an established network session or inter-process communication pipe. This is what a "connection" *is* in NEOS — a synchronized oscillation, not a logical state record.

**Execution rule:** Used to verify link stability. Any phase variance exceeding $\pm\pi/4$ triggers automatic phase inversion and teardown.

**Binds:** [[ftg]] — Resonant Handshake Protocol (replaces SYN/ACK)

## Phase Inversion Teardown

$$f_{total} = f_{node A} + f_{node B} = 0$$

**Definition:** Intentional disconnection via absolute destructive interference. The node shifts transmission phase by exactly $\pi$ radians, forcing combined wave amplitude to zero.

**Execution rule:** Replaces FIN/ACK. Teardown is not a message — it is the amplitude reaching zero. Resource reclamation follows automatically from $E = C_H\nu \to 0$.

**Binds:** [[ftg]]; verified by Test Case 1 in [test-doctrine.md](test-doctrine.md)

## Carrier Synthesis (Layer 1/2 Transduction)

$$W(t) = \sum_{k=0}^{N-1} \left[ A \cos(\omega_c t + \phi_k) \right]$$

**Definition:** Incoming binary states $0$ and $1$ map to discrete phase shifts $-\pi/2$ and $+\pi/2$ (axiom [A2](axioms.md#a2--logic-gate-override)), then synthesize onto the carrier $\omega_c$ as one continuous waveform.

**Execution rule:** Frame validation uses destructive interference — corrupted frames collapse into dissonance and dissipate. Do **not** implement CRC; the geometry does the error checking.

**Binds:** [[ftg]] — `layers_1_2`

---
type: prd
layer: spec
status: canonical
scope: software architecture only — hardware configurations deliberately excluded
---

# Product Requirements Document: New Earth Operating System (NEOS)

> Precedence note: where this document and [`_mkb/axioms.md`](../_mkb/axioms.md) disagree, the axioms win. The math is upstream of the product definition.

## 1. Product Vision & Overview

NEOS is a paradigm-shifting translation layer and operating environment. It simulates a finite, scalable, wave-based geometric universe atop traditional discrete Boolean hardware. Instead of relying on binary logic and Cartesian coordinates, NEOS governs computation, networking, and user interaction through harmonic resonance, Tetryen curvilinear geometry, and Lynchpin Number Theory.

## 2. Core Architecture Mapping

Moved to its own file — see [architecture-map.md](architecture-map.md). It is the routing table from traditional OS concepts to NEOS subsystems and is referenced too often to live buried in a section here.

## 3. Language Stack (The Translation Architecture)

Two tiers interface continuous mathematical waves with binary silicon.

- **The Substrate Layer (Hypervisor)** — written in **Rust**. Strict memory safety and concurrency for raw binary computation. The foundational Virtual Machine, translating wave functions into optimized hardware instructions. → [[substrate]]
- **The Symphony Layer (Kernel Logic)** — written in a **custom DSL**. Discards Boolean operators (`AND`, `OR`, `NOT`) in favor of geometric rules: constructive/destructive interference, phase shifts, and scale modulation as logic gates. → [[symphony-lang]] (the language) and [[symphony-kernel]] (the logic it expresses)

> **Closed.** All three gates now have operational definitions in [`_mkb/gates.md`](../_mkb/gates.md). Interference was already law; **phase shift** and **scale modulation** were named here but undefined, and are now derived — the first from A2's orientations being exactly a teardown `π` apart, the second from `ξ(r)` composed with the standing-wave `±π/4` criterion, giving a resonance band of exactly `1/8`. Recorded as a **synthesis**, since no paper supplies them.

## 4. Kernel and Resource Management

The kernel is a localized physics engine, treating CPU processes as interfering waveforms rather than time-sliced threads.

- **Task Scheduling:** governed by Harmonic Force Equilibrium — balancing computational charge density against available harmonic fields.
- **Resource Allocation:** memory and CPU cycles quantized as energy states via the Howard equation. High-priority tasks get higher $\omega$, drawing proportional computational energy.
- **Process Bifurcation:** forking executes under Lynchpin Number Theory ($1 \times 1 = 2$). The kernel splits computational wave structures using bifurcation logic to prevent standard memory faults.

→ [[symphony-kernel]]

## 5. File System and Data Storage

Discards linear sectors and rigid Cartesian arrays for non-Euclidean storage.

- **Hyperbolic Lattices:** data structures stored within a $\{5,4\}$ pentagonal tessellation mapping to hyperbolic fractal geometries.
- **Curved Addressing:** read/write operations use $a \otimes b = a \times b + d(a,b)$ to traverse the non-linear directory tree.
- **Area Preservation:** scaling file sizes triggers geometric fractals preserving logical area, eliminating disk fragmentation entirely.

→ [[lattice]]

## 6. Networking: The Fourier Transform Gateway (FTG)

A real-time gateway converting incoming binary packets into continuous wave phenomena, so NEOS can talk to standard OSI/TCP-IP devices.

- **Layer 1/2 Transduction:** binary $0$/$1$ map to phase shifts $-\pi/2$/$+\pi/2$, synthesized onto a carrier frequency.
- **Geometric Error Checking:** frame validation by destructive interference. Corrupted frames collapse into dissonance and dissipate — no CRC.
- **Layer 3/4 Routing:** IPv4/IPv6 linear addresses map to hyperbolic 4D lattice coordinates via the Poincaré disk distance formula.
- **Harmonic Multiplexing:** TCP/UDP ports become harmonic overtones riding the fundamental wave established by the IP coordinate.

→ [[ftg]] · equations in [`_mkb/equations.md`](../_mkb/equations.md)

## 7. Network State Management

Connections are physical, energetic reality rather than logical software state.

- **Resonant Handshake Protocol** (replaces SYN/ACK): a connection is forged by synchronizing two independent oscillators into a shared standing wave.
- **Phase Inversion Teardown** (replaces FIN/ACK): disconnection via absolute destructive interference — shift phase by exactly $\pi$, forcing combined amplitude to zero.

→ [[ftg]]

## 8. Application Data Translation (Layer 7)

The FTG crystallizes linear 1D/2D data into 3D/4D resonant shapes.

- **Linguistic Crystallization (text/code):** character strings become sequential harmonic nodes. Line breaks and code structures trigger bifurcation events, rendering documents as navigable 3D polymer-like fractals.
- **Holographic Projection (images):** pixel grids pass through a Continuous Fourier Transform into spatial frequency maps, projected onto internal faces of scalable Tetryen geometry.
- **Resonant Chambers (audio/video):** media files act as localized oscillators or volumetric time-crystals, driving physical vibration and 4D spatial rotation.

→ [[crystallisation]] — spun off from `ftg`; a representation transform, not a transport concern

## 9. Graphical User Interface

The UI visually represents the wave mechanics and spatial geometry of the kernel.

- **Tetryen Rendering:** all boundaries, applications, and system monitors render as curvilinear Tetryen structures.
- **Fractal Navigation:** infinite resolution scaling — zoom into localized data nodes without pixelation.
- **Interference Visualization:** system load, memory, and network traffic render as real-time standing waves showing constructive and destructive energy states.

→ [[gui]]

> **Closed.** "Tetryen" now has a distilled definition at [`_mkb/tetryen.md`](../_mkb/tetryen.md) — a curved tetrahedral structure of four nodes at standing-wave positions, characterised as the minimiser of `E[Γ] = ∫(K(s) + H(s)²)ds` with geodesic edges. [[gui]] and [[crystallisation]] are both unblocked.

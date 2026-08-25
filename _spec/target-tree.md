---
type: target-tree
layer: spec
status: canonical
note: mkb/ was relocated out of neos/ to the workspace root as _mkb/ — see "Deviation" below
---

# Target Code Tree

What `neos/` will contain when built. **This describes the destination, not the present.** No code exists yet; each subsystem's `04_implement` stage creates its own paths when that subsystem is actually built.

```
neos/
├── Cargo.toml                  # Root workspace manifest for the Rust substrate
├── substrate/                  # Tier 1: The Rust Hypervisor & Hardware Abstraction
│   ├── src/
│   │   ├── main.rs             # Hypervisor entry point and virtual machine bootstrap
│   │   ├── memory.rs           # Non-Euclidean memory pool management
│   │   └── translation.rs      # Raw binary-to-wave translation pipelines
│   └── Cargo.toml
├── symphony/                   # Tier 2: The Custom DSL & Kernel Logic
│   ├── compiler/               # Parser and compiler for geometric wave logic
│   ├── interpreter/            # Executes phase-based conditional branches and bifurcation rules
│   └── kernel/
│       ├── scheduler.rs        # Harmonic Force Equilibrium process scheduler
│       └── quantization.rs     # Howard Comma energy allocation (E = C_H·ω) and garbage collection
├── ftg/                        # Fourier Transform Gateway (OSI bridge) — §6 + §7
│   ├── layers_1_2.rs           # Wave framing; geometric error checking by dissonance (NO CRC)
│   ├── layers_3_4.rs           # Poincaré disk hyperbolic routing & port overtone multiplexing
│   └── session.rs              # Resonant Handshake and Phase Inversion Teardown (§7)
├── crystallisation/            # Application data translation (Layer 7, §8) — deferred
│   ├── linguistic.rs           # Text/code → harmonic nodes; line breaks trigger bifurcation
│   ├── holographic.rs          # Pixel grids → CFT → spatial frequency maps → Tetryen faces
│   └── resonant.rs             # Audio/video as oscillators and volumetric time-crystals
├── lattice/                    # Hyperbolic 4D Geometry & Storage Engine
│   ├── tessellation.rs         # {5,4} pentagonal lattice mapping
│   └── metric.rs               # Distance functions (a⊗b and geodesic calculations)
├── gui/                        # Tetryen Rendering & Fractal UI Engine
│   ├── renderer.rs             # Curvilinear geometry graphics pipeline
│   ├── fractal.rs              # Infinite-resolution scaling engine
│   └── visualization.rs        # Standing wave interference and energy state visualizer
└── tests/                      # Physics-Based Test-Driven Development
    ├── interference_test.rs    # Phase inversion, destructive teardown, energy release
    └── bifurcation_test.rs     # Lynchpin multiplicative execution (1 × 1 = 2)
```

## Deviation from the original structure

The original draft nested `mkb/` inside `neos/`, holding `constants.json`, `schemas/`, and `papers/`.

**It now lives at the workspace root as [`_mkb/`](../_mkb/CONTEXT.md).** Reason: the MKB is the factory — the stable law configured once — while `neos/` is the product emitted from it. ICM keeps those structurally apart, and the MKB is read by build agents long before any Rust compiles.

**Consequence for the Rust build — now implemented.** Each crate carries its own `build.rs` that reads `_mkb/constants.json` and emits generated Rust constants into `OUT_DIR`. There is no workspace-level `build.rs`; the mechanism is per-crate:

- `neos/lattice/build.rs`
- `neos/substrate/build.rs`
- `neos/symphony/kernel/build.rs`

Generated constants are a build artifact, never a second hand-maintained source. **No numeric value from the MKB appears literally in any `.rs` file** — verified by grep in each subsystem's implementation log.

Two of those build scripts additionally *validate* the JSON and fail the build on inconsistency: `lattice` checks that `tessellation.schlafli`'s `q` matches `vertex_degree` (reconciliation R3), and `symphony-kernel` checks that `howard_comma.frequency_variable` is still `"nu"` (R5a).

## Mapping to subsystem records

Each path above is produced by a subsystem record under [`subsystems/`](../subsystems/):

| Record | Produces |
|---|---|
| [[substrate]] | `neos/substrate/**`, root `Cargo.toml` |
| [[symphony-kernel]] | `neos/symphony/kernel/**` — **built** |
| [[symphony-lang]] | `neos/symphony/{compiler,interpreter}/**` — deferred |
| [[ftg]] | `neos/ftg/**` |
| [[crystallisation]] | `neos/crystallisation/**` |
| [[lattice]] | `neos/lattice/**` |
| [[gui]] | `neos/gui/**` |

`neos/tests/` is written across records — each subsystem contributes the assertions its `03_tests` stage produced.

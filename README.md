<div align="center">

# HeavenOS — NEOS

### *New Earth Operating System*

A wave-based, finite, scalable, non-Euclidean operating system simulated atop Boolean hardware —
a Rust substrate driven by a quantized harmonic kernel.

[![Rust](https://img.shields.io/badge/Rust-2021-CE422B?style=for-the-badge&logo=rust&logoColor=white)](neos/Cargo.toml)
[![Tests](https://img.shields.io/badge/tests-472%20passing-2EC4B6?style=for-the-badge)](CONTEXT.md)
[![Subsystems](https://img.shields.io/badge/subsystems-7%2F7%20complete-6C5CE7?style=for-the-badge)](#subsystems)
[![Doctrine](https://img.shields.io/badge/doctrine-physics--TDD-00B4D8?style=for-the-badge)](#the-doctrine)
[![Status](https://img.shields.io/badge/status-active-2D3142?style=for-the-badge)](#status)

</div>

<br>

> No Boolean logic. No speculative math. No stage advances without a human reading the output.
> Every claim this workspace makes is either derived from law, or labelled honestly as a choice.

<br>

## Table of contents

- [What this is](#what-this-is)
- [The doctrine](#the-doctrine)
- [Workspace map](#workspace-map)
- [The build loop](#the-build-loop)
- [Subsystems](#subsystems)
- [Quickstart](#quickstart)
- [What `cargo run` actually does](#what-cargo-run-actually-does)
- [Status](#status)

<br>

## What this is

HeavenOS is the ICM (Instruction-Contract-Model) workspace building **NEOS** — an operating system
whose logic is geometric rather than Boolean, whose memory is curved rather than flat, and whose
scheduler balances load the way a diffusion equation balances a field, not the way a priority queue
does.

Seven subsystems, each carried through the same four-stage loop from raw physics to compiled,
tested Rust:

| | |
|---|---|
| **`lattice`** | the `{5,4}` hyperbolic tessellation, the `⊗` operator, curved addressing |
| **`substrate`** | the wave-translation floor: memory as standing waves on that lattice |
| **`symphony-kernel`** | scheduling, deadlock detection, diffusion load balancing |
| **`symphony-lang`** | the DSL — interference, phase shift, and scale modulation *as* logic gates |
| **`ftg`** | the Fourier Transform Gateway — routing, sessions, real packet delivery |
| **`gui`** | Tetryen geometry, navigation, live telemetry |
| **`crystallisation`** | flat media (text, image, audio, video) crystallised into native 3D/4D form |

Every artifact that leaves this workspace is **verified, working code in [`neos/`](neos/)** — not a
proposal, not a stub.

<br>

## The doctrine

> $$\text{Axioms} \;\succ\; \text{Equations} \;\succ\; \texttt{constants.json} \;\succ\; \text{Spec} \;\succ\; \text{Code}$$

If code contradicts [`_mkb/`](_mkb/) (the law), **the code is wrong**. If the spec contradicts the
law, **the spec is wrong**. Three rules follow directly from that precedence, and this codebase is
built to be checked against them, not just to claim them:

| Rule | What it actually means here |
|---|---|
| **No speculative math** | Every formula is verified step-by-step — often in a disposable scratch harness — *before* it's implemented, never after |
| **The sabotage gate** | No module is marked complete until it's deliberately broken, the suite is run, and it fails **for the right reason** |
| **Honest labelling** | A design decision with no law behind it (a deadlock victim policy, a privilege model) is built and stated as a choice — never smuggled in as physics |

<br>

## Workspace map

```
_mkb/            the law — axioms, equations, constants.json, source papers      (immutable)
_spec/           the product definition — PRD, architecture map, contracts      (derived)
subsystems/      each subsystem's own four-stage build loop                    (in progress → complete)
_templates/      boilerplate for initializing a new subsystem                  (schema)
_system/         status.py — generates the build report, audits integrity      (generated)
neos/            the production Rust workspace — all seven crates              (working code)
_archive/        historical drafts and retired specs                          (read-only)
```

<br>

## The build loop

Every subsystem runs the identical four stages against the identical factory:

```
_mkb/ + _spec/  ──the factory, configured once──┐
                                                 ▼
   01_derive → 02_design → 03_tests → 04_implement → neos/
        ▲            ▲          ▲            ▲
        └── a human reads the output at every arrow ──┘
```

| Stage | Job | Output | The human check |
|---|---|---|---|
| `01_derive` | extract the binding law | `math-contract.md` | are the open questions load-bearing? |
| `02_design` | types and interfaces | `design.md` | can any type hold an axiom-forbidden value? |
| `03_tests` | physics assertions, *first* | `test-plan.md` | would a classical implementation still pass? |
| `04_implement` | write the code | code + `implementation-log.md` | tests green, deviations written down |

<br>

## Subsystems

<div align="center">

| Subsystem | Tests | Status |
|:---|:---:|:---:|
| [`lattice`](subsystems/lattice/CONTEXT.md) | `73` | ✅ complete |
| [`substrate`](subsystems/substrate/CONTEXT.md) | `43` | ✅ complete |
| [`symphony-kernel`](subsystems/symphony-kernel/CONTEXT.md) | `79` | ✅ complete |
| [`symphony-lang`](subsystems/symphony-lang/CONTEXT.md) | `78` | ✅ complete |
| [`ftg`](subsystems/ftg/CONTEXT.md) | `62` | ✅ complete |
| [`gui`](subsystems/gui/CONTEXT.md) | `63` | ✅ complete |
| [`crystallisation`](subsystems/crystallisation/CONTEXT.md) | `67` | ✅ complete |
| *cross-cutting (geometric test bed)* | `7` | ✅ complete |
| **Total** | **472** | **7 / 7** |

</div>

Highlights that only exist because a subsystem's own limits were closed, not left as stated
boundaries: real OS-thread concurrency (`symphony_lang::concurrent`), a genuine multi-tenant
sandbox (`symphony_lang::sandbox::Sandbox`), a real radix-2 FFT replacing an `O((HW)²)` sum in
`crystallisation`, and lock-free parallel media crystallization across real threads. Full detail,
including every sabotage performed and what it caught, lives in each subsystem's own
`CONTEXT.md`.

<br>

## Quickstart

```bash
# generate the build report — reads every stage's output, runs every test,
# audits every wikilink and constant reference
python _system/status.py --check

# run the full workspace test suite directly
cd neos && cargo test --workspace

# boot the composed system: substrate boots, the kernel schedules real tasks,
# ftg delivers a real frame over a resonant session, crystallisation turns a
# real image/audio/video/text into native form, gui reads the result —
# rendered to neos/render*.ppm and neos/render*.gif
cargo run
```

<br>

## What `cargo run` actually does

Not a mock, not a placeholder — one real process:

- boots `substrate` and demonstrates real fault trapping and recovery
- schedules real tasks in `symphony-kernel`, builds and **resolves** a real deadlock
- runs real concurrent memory allocation across real OS threads
- runs three mutually untrusted `symphony-lang` programs **concurrently**, inside a genuine
  multi-tenant sandbox, each provably confined to its own memory
- delivers a real frame through `ftg` over a resonant, handshake-established session
- crystallises a real image, WAV tone, video sequence, and text document — **all four
  concurrently**, on real threads, with zero shared state
- evolves a Tetryen's real node amplitudes forward in discrete time
- renders the result to a real `.ppm` still and a real animated `.gif`

Full narration of every step: [`CONTEXT.md`](CONTEXT.md#status-is-whatever-exists).

<br>

## Status

The table above is a snapshot and can drift out of date. The generator cannot:

```bash
python _system/status.py --check
```

This reads every stage's real output on disk, runs every test suite, checks every law-ledger row
against `_mkb/`, and audits wikilinks, relative links, and file encodings — exiting non-zero if
anything is inconsistent. It is the only source of truth this workspace trusts about itself.

<br>

<div align="center">

*Built one verified stage at a time. No shortcuts through the law.*

</div>

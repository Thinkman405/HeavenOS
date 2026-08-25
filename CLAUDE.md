# HeavenOS — Building NEOS (New Earth Operating System)

A wave-based, finite, scalable, non-Euclidean operating system simulated atop Boolean hardware. Built with a Rust substrate and a quantized harmonic kernel. Every artifact leaving this workspace is verified, working code in `neos/`.

---

## The Veteran Software Engineer's Operating Doctrine

### 1. Precedence & Theoretical Grounding

The precedence cascade is absolute:


$$\text{Axioms} \succ \text{Equations} \succ \text{constants.json} \succ \text{Spec} \succ \text{Code}$$

* **Upstream Primacy:** If code contradicts `_mkb/`, the code is wrong. If the spec contradicts `_mkb/`, the spec is wrong.
* **No Speculative Math:** Independent step-by-step mathematical verification is required before implementing any formula. Do not substitute angular frequency $\omega$ for linear frequency $\nu$, or change normalization factors without explicit MKB law changes.

### 2. Sabotage & Testing Rigor

* **The Sabotage Gate:** No subsystem, module, or slice is marked `COMPLETE` until it undergoes intentional sabotage. Mutate the mechanism, run the test suite, and verify that the tests fail for the correct reason.
* **Exact Invariant Assertions:** Never loosen test tolerances or replace mathematical cancellation invariants with loose decay approximations ($H(\kappa) \to 0$ requires cross-scale cancellation, not arbitrary rounding).
* **Target Schema Accuracy:** Ensure test metadata and doctrine mappings point strictly to active records, never to retired or split module symbols.

### 3. Engineering & Encoding Discipline

* **BOM & Encoding Safety:** Config and build-critical files (such as `constants.json`) must remain strict ASCII or UTF-8 without BOM to prevent `serde_json` and compilation parser panics.
* **Topological Rigor:** Do not assume vertex-transitivity on bounded non-Euclidean patches. Measure local cell degrees dynamically (e.g., boundary valence dropping from 5 to 1) to prevent catastrophic load-balancer mis-weighting.
* **Closed-Form Performance:** Implement topology and adjacency operations via closed-form group operations over local coordinate vectors rather than runtime graph discovery or search algorithms.

### 4. Scope & Interface Boundaries

* **Surgical Record Splitting:** When a module encompasses distinct concerns (e.g., DSL vs. Kernel, Routing vs. Data Representation), split the record cleanly. Never silently drop layers or contract requirements during a split.
* **Explicit Contract Separation:** Maintain clear boundaries between detection and resolution (e.g., deadlock detection in kernel vs. application-level resolution).

---

## Workspace Map

| Folder | Core Purpose | Authority Level |
| --- | --- | --- |
| `_mkb/` | **The Law** — Axioms, equations, `constants.json`, test doctrine, source papers | Immutable Source of Truth |
| `_spec/` | **Product Definition** — PRD, architecture map, target code tree, contracts | Derived Functional Spec |
| `subsystems/` | Active subsystems executing the 4-stage build loop | Implementation Stage |
| `_templates/` | Standardized boilerplate stamps for new subsystem initialization | Operational Schema |
| `_system/` | Workspace tooling — `status.py` generates the build report and audits integrity | Generated, never hand-edited |
| `neos/` | Production Rust workspace. **All seven crates built:** `lattice`, `substrate`, `symphony/kernel`, `symphony/lang`, `ftg`, `gui`, `crystallisation`. A crate is created by its subsystem's `04_implement`, never ahead of it | Working Code Output |
| `_archive/` | Historical drafts and retired specifications | Read-Only Audit Trail |

---

## Operational Execution Routing

| Operation / Task | Primary Context Path | Verification Checkpoint |
| --- | --- | --- |
| Workspace Onboarding | `CONTEXT.md` | Build order sequence |
| Initializing Subsystem | `subsystems/{name}/CONTEXT.md` | Scope boundaries & open questions |
| Running Build Stage | `subsystems/{name}/{NN}_*/CONTEXT.md` | Human review of output artifacts |
| Status Audit | `python _system/status.py` | Generated from disk — never hand-edit a status summary |
| Constants & Law Check | `_mkb/CONTEXT.md` | Confirm $C_H$, $\xi(r)$, and metric invariants |
| Architectural Scope Check | `_spec/architecture-map.md` | Verify dependency topology |

---

## Human-in-the-Loop Gate Rule

> **The Absolute Gate:** No stage transition, record completion, or PR merging is permitted until a human operator has explicitly validated the execution output of the preceding stage. Zero unverified automated cascades.
---
type: subsystem
subsystem: "{slug}"
tier: "{1 | 2 | bridge | storage | presentation}"
language: "{Rust | custom DSL}"
stage: 01_derive
status: not-started
prd_sections: []
binds_axioms: []
---

# {Subsystem} — {what it is in five words}

One job: {the single responsibility, from the architecture map}.

## The build loop

Four stages, in order. Nothing advances until a person has read the previous output.

| Stage | Job | Output |
|---|---|---|
| `01_derive` | pull the exact law that binds this subsystem | `math-contract.md` |
| `02_design` | types and interfaces against that contract | `design.md` |
| `03_tests` | physics assertions, written before code | `test-plan.md` |
| `04_implement` | write the Rust/DSL into `neos/` | `implementation-log.md` |

## Status

Set `stage:` and `status:` in the frontmatter above as you advance. Real status is always derivable by scanning which `*/output/` folders hold files — the frontmatter is a convenience, and the filesystem wins if they disagree.

## Scope

**Owns:** `neos/{path}/**`
**PRD sections:** {list — from the architecture map ownership table}
**Axioms that bind it:** {A1 / A2 / A3}
**Equations that bind it:** {names — grep `Binds:` in `_mkb/equations.md`}

## Do not

Load other subsystems' records. They don't share state; they share the factory (`_mkb/`, `_spec/`). If you need something another subsystem produced, it is either in `neos/` as code or it belongs in the factory.

---
type: router
layer: L3
---

# _spec — what we are building

The factory layer for *product definition*, as [`_mkb/`](../_mkb/CONTEXT.md) is the factory layer for *law*. Stable across subsystem builds.

| File | Answers | Load when |
|---|---|---|
| [architecture-map.md](architecture-map.md) | which subsystem owns this concept? | orienting, or picking what to build next |
| [prd.md](prd.md) | what must this subsystem do? | `01_derive` — read only your subsystem's sections |
| [target-tree.md](target-tree.md) | where does the code go? | `04_implement` |

## Loading discipline

`01_derive` reads **only the PRD sections listed for its subsystem** in the architecture map's ownership table — not the whole PRD. The map exists so you can filter.

## Precedence

[`_mkb/axioms.md`](../_mkb/axioms.md) outranks everything here. The PRD describes intent; the axioms constrain what intent is expressible. A PRD clause that violates an axiom is a defect in the PRD.

## Human check

The PRD and architecture map both carry flagged open questions (the Tetryen definition gap; the Gauss law row with no execution rule). Before starting a subsystem, check whether its open question is still open — an unresolved one blocks `01_derive` for that subsystem specifically, not the whole project.

# 01_derive — extract the law that binds this subsystem

One job: assemble the exact axioms, constants, and equations this subsystem must obey into a single contract. No design, no code.

## Inputs

- Reference (every run): `../../../_mkb/axioms.md`
- Reference (every run): `../../../_mkb/constants.md`
- Reference (every run): `../../../_mkb/equations.md` — **only the entries whose `Binds:` line names this subsystem**
- Reference (every run): `../../../_spec/prd.md` — **only the sections listed in this record's `prd_sections:` frontmatter**
- Reference (every run): `../../../_spec/architecture-map.md`

Do NOT load: `_mkb/papers/` (unless a derivation is genuinely contested), other subsystems' records, the whole PRD, `_spec/target-tree.md`.

## Process

1. Read the axioms. List which of A1/A2/A3 bind this subsystem and how.
2. Copy — do not paraphrase — each binding equation with its execution rule.
3. Name every constant this subsystem reads, with the JSON key from `constants.json`.
4. State what classical construct each override forbids here (e.g. "no `bool` in branch evaluation", "no flat indexed arrays").
5. List open questions: anything the spec asserts that the MKB does not derive. Flag them; do not invent an answer.

## Outputs

- `math-contract.md` → `output/`

## Human check

Read the open-questions list first. If it contains anything load-bearing for this subsystem, stop and close it in `_mkb/` before advancing — not here. A gap papered over at this stage becomes wrong code at stage 04, and correction is cheapest right now.

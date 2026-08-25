# 04_implement — write the code

One job: implement the design so the test plan passes. Code lands in `neos/`; this folder holds only the log of what was written and why.

## Inputs

- Working (this run): `../02_design/output/design.md`
- Working (this run): `../03_tests/output/test-plan.md`
- Working (this run): `../01_derive/output/math-contract.md`
- Reference (every run): `../../../_spec/target-tree.md` — for the exact destination paths
- Reference (every run): `../../../_mkb/constants.json` — read values, never retype them

Do NOT load: the PRD, `_mkb/papers/`, other subsystems' code.

## Process

1. Write the tests from `test-plan.md` into `neos/tests/` **first**. Confirm they fail.
2. Create only the paths this record owns (see `../CONTEXT.md` → Scope). Do not scaffold other subsystems' folders.
3. Implement against `design.md` until the tests pass.
4. Read constants from `constants.json` via the build; never hardcode a numeric value that exists there.
5. Log every deviation from the design and every concession made to hardware reality, with the reason.

## Outputs

- `implementation-log.md` → `output/`
- Rust/DSL source → `neos/{owned paths}` (product, not artifact — it lives in the code tree, not here)

## Human check

Run the tests and read the deviation log. A deviation is acceptable when it is written down with a reason; it is a defect when it is silent. If a deviation contradicts the math contract rather than merely the design, it belongs upstream — fix `_mkb/` or the contract, then come back. Do not let code become the place where the law quietly changed.

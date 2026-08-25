# 03_tests — physics assertions, written before code

One job: write the test plan that will prove this subsystem obeys its math contract. Tests come before implementation; that is the doctrine, not a preference.

## Inputs

- Working (this run): `../02_design/output/design.md`
- Working (this run): `../01_derive/output/math-contract.md`
- Reference (every run): `../../../_mkb/test-doctrine.md`

Do NOT load: the PRD, `_mkb/papers/`, other subsystems' test plans.

## Process

1. For every rule in the math contract, write at least one assertion that would fail if the rule were violated.
2. Shape each assertion in wave terms — superposition sums, phase alignment, energy states. See the substitution table in `test-doctrine.md`. A `assert_eq!(x, true)` is a doctrine violation even when it passes.
3. For every tolerance flagged in `design.md`, pick an epsilon and **justify it against the amplitude scale in play**. Unjustified epsilons are how this doctrine quietly degrades back into conventional equality testing.
4. Name which file under `neos/tests/` each assertion will land in.
5. Mark any assertion you cannot yet write, and say what is missing.

## Outputs

- `test-plan.md` → `output/`

## Human check

Read the assertions and ask of each: *if the implementation regressed to classical Boolean or Euclidean logic, would this test catch it?* Any assertion that would still pass against a conventional implementation is not testing NEOS — rewrite it before code gets written against it.

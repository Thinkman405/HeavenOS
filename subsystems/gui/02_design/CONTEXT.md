# 02_design — types and interfaces, no implementation

One job: design the module's data structures and public interface so that the math contract is enforced by the type system wherever possible.

## Inputs

- Working (this run): `../01_derive/output/math-contract.md`
- Reference (every run): `../../../_mkb/schemas/tetryen-node.schema.json`
- Reference (every run): `../CONTEXT.md` (this record's scope + owned paths)

Do NOT load: the PRD (its content is already distilled into the math contract), `_mkb/papers/`, other subsystems.

## Process

1. Read the math contract. Everything below must trace to a line in it.
2. Define the types. Where an axiom forbids a classical construct, make the illegal state unrepresentable rather than merely checked at runtime — a phase value should not be a bare `f64` if `f64` permits values the axiom excludes.
3. Define the public interface: function signatures, no bodies.
4. State the error model in wave terms — what dissipates, what fails to resonate, what is unmappable.
5. Note every place a floating-point tolerance will be needed. `03_tests` will have to pick each one.

## Outputs

- `design.md` → `output/`

## Human check

For each type, ask: could this hold a value the axioms forbid? Every yes is either a deliberate, written-down concession to hardware reality or a design defect. There is no third category. Edit in place — `03_tests` reads whatever is here.

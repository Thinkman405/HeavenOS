---
type: design
subsystem: symphony-lang
stage: 02_design
status: complete
consumes: [symphony-kernel]
---

# Symphony-lang — Design

Types and signatures. No bodies.

## The shape

```
source ──lex──▶ [Spanned] ──parse──▶ [Stmt] ──execute──▶ Execution
         ▲                                                   │
         └─ A2 refusal fires here, before anything else       ▼
                                                     symphony_kernel::Task
                                                        → Scheduler::ingest
```

Three modules in one crate. The deviation from `_spec/target-tree.md`'s two-crate layout is recorded in the math contract, §"Deviation".

## The design question this stage exists to answer

*Can any type hold an axiom-forbidden value?*

For A2 the answer has to be stronger than "no type holds a `bool`". It has to be: **no source text containing a Boolean construct can become a value of any type in this crate.** That forces the refusal into `lex`, the first function the text meets, ahead of `Token` even existing.

```rust
const FORBIDDEN: &[(&str, &str)];   // construct → why, and what to write instead
pub fn lex(source: &str) -> Result<Vec<Spanned>, LangError>;
```

`FORBIDDEN` pairs each construct with a *reason*, not just a ban. `if` reports "conditionals are phase alignment: use `when X aligns Y`". A bare "syntax error" would be indistinguishable from a parser that had never heard of A2, so the message is part of the contract, and a test asserts it.

Word-shaped constructs (`true`, `if`, `bool`) are checked per token; punctuation-shaped ones (`&&`, `==`, `!`) are checked against the raw line **before** tokenisation, since whitespace splitting would otherwise let `a&&b` through as one identifier.

## Tokens

```rust
pub enum Token {
    Task, At, Hz, Phase, When, Aligns, Opposes, Fork, Emit,
    Positive, Negative,          // the two A2 orientations; no third literal
    OpenBrace, CloseBrace,
    Ident(String), Number(f64),
}
pub struct Spanned { pub token: Token, pub line: usize }
```

`Positive`/`Negative` are the *only* phase literals. There is deliberately no `Unknown`, no `Null`, and no numeric phase — `Phase::from_radians` exists in the kernel for measured angles and refuses anything not near the two permitted orientations, which is a different job from a source literal.

## Grammar

```text
program   := statement*
statement := task | branch | fork | emit
task      := "task" IDENT "at" NUMBER "hz" "phase" ("+" | "-")
branch    := "when" IDENT ("aligns" | "opposes") IDENT "{" statement* "}"
fork      := "fork" IDENT
emit      := "emit" IDENT
```

Note what is absent: no expression production, no assignment, no loop, no function. A program declares oscillators, asks how they interfere, and hands the survivors to the scheduler. Every one of those omissions is deliberate — see the contract's Forbidden list.

```rust
pub enum Alignment { Aligns, Opposes }

pub enum Stmt {
    Task { name: String, frequency: f64, phase: Phase },   // Phase is the KERNEL's
    Branch { left: String, alignment: Alignment, right: String, body: Vec<Stmt> },
    Fork { name: String },
    Emit { name: String },
}

pub fn parse(tokens: &[Spanned]) -> Result<Vec<Stmt>, LangError>;
```

`Stmt::Task` carries `symphony_kernel::Phase` directly rather than a local copy. A local `LangPhase` plus a conversion would be a second home for A2's two-valued set, and the conversion function would be exactly the place someone later adds a `bool` bridge.

`Branch.body` is `Vec<Stmt>`, so nesting is structural. A flat statement list with jump targets would make "did the outer branch cancel?" a runtime question instead of a tree walk.

## Runtime

```rust
pub struct RuntimeTask { pub name: String, /* private */ }

impl RuntimeTask {
    pub fn new(name: impl Into<String>, frequency: f64, phase: Phase)
        -> Result<Self, LangError>;      // refuses non-positive / non-finite ν
    pub fn with_fork_unit(self, unit: f64) -> Self;
}

impl symphony_kernel::bifurcation::TaskModel for RuntimeTask { … }
```

**This impl is the point of the record.** `TaskModel`'s doc comment says outright that nothing in the kernel depends on it yet and that it exists to fix the shape of the seam. `RuntimeTask` is the first implementor.

Fields are private and construction is fallible, so an unphysical frequency cannot exist as a value — §4.1 is enforced by the constructor, not checked later.

`with_fork_unit` is the one public way to build a task the surface syntax cannot. It exists so the non-integral-fork refusal is **reachable**. A guard nothing can trigger is decoration, and a test asserting one is asserting nothing — a lesson this workspace has already paid for once, in `crystallisation`'s `non_unitary_modulation_is_refused`.

## Execution

```rust
pub struct ForkEvent   { pub task: String, pub bifurcation: Bifurcation }
pub struct BranchEvent { pub left: String, pub right: String,
                         pub interference: Interference, pub taken: bool }

pub struct Execution {
    pub declared: BTreeMap<String, RuntimeTask>,
    pub emitted:  Vec<symphony_kernel::Task>,   // kernel type: no translation layer
    pub forks:    Vec<ForkEvent>,
    pub branches: Vec<BranchEvent>,
}
impl Execution { pub fn total_energy_joules(&self) -> f64; }

pub fn execute(program: &[Stmt]) -> Result<Execution, LangError>;
pub fn execute_with(program: &[Stmt], seed: impl IntoIterator<Item = RuntimeTask>)
    -> Result<Execution, LangError>;
pub fn run(source: &str) -> Result<Execution, LangError>;
```

`BranchEvent` records `interference` **and** `taken` separately. They are not the same thing: `taken` also depends on which form the programmer wrote. Collapsing them would make the record unable to distinguish "cancelled" from "cancelled, and that was what was wanted".

`declared` is a `BTreeMap` rather than a `HashMap` so `Execution` has a deterministic ordering and can derive `PartialEq` usefully in tests.

`execute_with` seeds the environment for a host embedding the language. Seeding **adds** to scope; it does not bypass it, so a source declaration colliding with a seeded name is still a `DuplicateTask`.

## Errors

```rust
pub enum LangError {
    ForbiddenBooleanConstruct { construct: String, reason: String, line: usize },
    UnexpectedCharacter { text: String, line: usize },
    UnexpectedToken { found: String, expected: String, line: usize },
    UnexpectedEnd { expected: String },
    UndeclaredTask { name: String },
    DuplicateTask { name: String },
    UnphysicalFrequency { name: String, frequency: f64 },
    ForkOutsideDomain { name: String, unit: f64 },
    NonIntegralFork { name: String, children: f64 },
}
```

Named for the failure, per `_mkb/test-doctrine.md`. `ForbiddenBooleanConstruct` is the star: it carries the construct, the reason, and the line, and its `Display` cites the axiom by name.

## Float tolerances

Exactly one place needs one: `program_energy_is_linear_in_declared_frequency` compares an energy **ratio**. Energies here are of order `1e-32` J, so an absolute threshold would be meaningless — **the fourth-occurrence lesson from `crystallisation`, applied in advance rather than discovered again.**

Everywhere else the comparisons are exact by construction: fork children are `2.0` bit-exact, phases are enum variants, and the path-independence test compares two sums of the *same* `f64` values.

---

# Slice 2 — the remaining two gates

PRD §3 names three geometric gates; slice 1 built one. The design question for this slice was *what shape does a gate have*, given that A2 forbids comparison.

## Two shapes, not three

The three gates do not have a uniform signature, and forcing one would have been wrong:

| Gate | Shape | Surface form |
|---|---|---|
| interference | `(Phase, Phase) -> Interference` | relation in `when` |
| **phase shift** | `Phase -> Phase` | **statement** |
| **scale modulation** | `(ν, r, ν, r) -> Interference` | relation in `when` |

Inversion *transforms* rather than *tests*, so it is a statement (`invert A`) rather than a `when` relation. Making it a relation would have required inventing a question it answers, and there isn't one — the π shift has no truth value, it has a result.

The other two both return `Interference`, so they share the two-polarity pattern: `aligns`/`opposes` and `resonates`/`detunes`.

```rust
pub enum Alignment { Aligns, Opposes, Resonates, Detunes }
impl Alignment { pub fn reads_phase(self) -> bool; }
```

`reads_phase` is the dispatch, and it is also the statement that the gates read **disjoint inputs**. That property is what makes them independent rather than two spellings of one test.

## Scale is optional, and the default is not a special case

```rust
pub const REFERENCE_SCALE: f64 = 1.0;

Stmt::Task { name, frequency, phase, scale }   // scale defaults to REFERENCE_SCALE
```

`ξ(R) = 1` exactly, so an omitted `scale` applies the identity correction. The parser fills in `REFERENCE_SCALE` and the interpreter has no branch for "no scale declared" — the default is the reference scale rather than the absence of one. A `None` here would have grown an `unwrap_or` at every use.

## The correction has one home

`RuntimeTask` stores the scale; `Runner::emit` passes it through `Task::with_scale`; the **kernel** applies `ξ` inside `energy_joules`. The language never multiplies by `ξ` itself.

That was a live temptation — pre-multiplying would have made `total_energy_joules` a one-liner — and it would have put the correction in two places, which is how the two copies drift.

## Errors added

```rust
UndefinedScale     { name: String, scale: f64 },   // outside xi's domain [0, inf)
UnresonatablePair  { left: String, right: String },// mean effective frequency collapsed
```

`UnresonatablePair` is the interesting one. `ξ(r) → 0` as `r → ∞`, so a large enough scale drives both effective frequencies to zero and the detuning ratio to `0/0`. The gate **refuses** rather than picking an answer — same discipline as `⊗` at its domain limit, and the third refusal-not-default in this crate.

Note there is deliberately **no error for inversion**. Gate 2 is total; the only way `invert` fails is an undeclared name, which is scoping rather than physics.

## Float tolerances — still one

Unchanged in kind. The gate comparisons are all against the derived band, which is dimensionless by construction — the derivation produces a ratio, so there was never an absolute threshold to get wrong. The one place a tolerance appears is still the energy-ratio assertion.

The boundary assertions use **exact** straddling values (`1.18922` resonant, `1.18923` detuned) rather than a tolerance around the boundary. A tolerance there would have hidden exactly the kind of off-by-a-rounding error that the first draft of the boundary test actually contained.

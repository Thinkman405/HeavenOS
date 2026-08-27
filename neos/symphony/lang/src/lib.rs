//! # Symphony-lang — the geometric wave-logic DSL
//!
//! The language half of the `symphony` split. `symphony-kernel` owns the
//! runtime physics; this crate owns **what a task is and what a condition is**,
//! which is exactly the gap `symphony_kernel::bifurcation::TaskModel` was left
//! open for.
//!
//! ## A2 is enforced by the lexer
//!
//! Axiom A2 deprecates Boolean truth values. Every other subsystem honours that
//! by *not defining* `bool`-shaped types — a discipline the compiler can help
//! with but cannot state. A language can do better: here `true`, `false`, `if`,
//! `else`, `&&`, `||`, `!` and `==` are **rejected at lex time** with an error
//! that names the axiom. There is no Boolean expression grammar to fall back
//! to, so A2 cannot be violated by a programmer in a hurry.
//!
//! ## The three gates
//!
//! PRD section 3 names three geometric replacements for Boolean logic:
//! *"constructive/destructive interference, phase shifts, and scale modulation
//! as logic gates."* All three have surface syntax, and all three are decided
//! by `symphony-kernel`. Law: `_mkb/gates.md`.
//!
//! | Gate | Syntax | Reads |
//! |---|---|---|
//! | interference | `when A aligns B` / `opposes` | phase |
//! | phase shift | `invert A` | phase -> phase |
//! | scale modulation | `when A resonates B` / `detunes` | frequency and scale |
//!
//! ```text
//! task carrier at 440 hz phase +
//! task guard   at 220 hz phase +
//! task probe   at 440 hz phase - scale 1.15
//!
//! when carrier aligns guard {      # gate 1: constructive - body runs
//!     fork carrier                 # A1: exactly 2 children
//! }
//!
//! when carrier resonates probe {   # gate 3: detuning 0.0999 <= 1/8 - runs
//!     invert probe                 # gate 2: the exact pi shift
//! }
//!
//! when carrier aligns probe {      # gate 1 again, now that probe flipped
//!     emit probe
//! }
//! ```
//!
//! The gates are **independent**: gate 1 ignores frequency and scale, gate 3
//! ignores phase, and they disagree on pairs. `carrier` and `probe` above
//! interfere destructively while resonating — neither gate is expressible
//! through the other.
//!
//! ## What this crate does not do
//!
//! No arithmetic, no expressions, no assignment, no loops. A program declares
//! oscillators, asks physical questions about them, and hands the surviving
//! work to the scheduler. Adding an expression grammar would immediately raise
//! the question of what `a == b` means, and the answer under A2 is "nothing" —
//! so the question is not raised.
//!
//! There is deliberately **no scale comparison**. A `when A above B` gate would
//! be a relational Boolean operator wearing a geometric name; gate 3 instead
//! asks whether a standing wave between the two would survive, which is
//! two-valued for a physical reason rather than by fiat.
//!
//! ## A real instruction-executing state machine, still inside A2
//!
//! [`vm`] compiles a program into a flat, program-counter-addressed
//! instruction sequence (a genuine dispatch loop, not a recursive tree walk)
//! and can run a **batch** of programs against real, shared
//! `substrate::MemoryPool` and `symphony_kernel::resources::ResourceTracker`
//! state — `store`/`load` move a task's own physical state into and out of
//! curved memory, `acquire`/`release` are real resource acquisition. A fault
//! in one program (an out-of-range cell, a resource another program holds)
//! traps that program alone; the rest of the batch keeps running against the
//! same, undamaged shared state. No new expression grammar, no arithmetic, no
//! privilege domains — see `_mkb/instruction_set.md` for exactly what was
//! composed and what was deliberately left out.
//!
//! ## A genuine multi-tenant sandbox
//!
//! [`sandbox::Sandbox`] composes [`concurrent::run_program`]'s real threads
//! with a per-tenant ownership map over cells, so several mutually
//! untrusted `symphony-lang` programs can run at the same time against one
//! real shared pool, each provably unable to touch another's admitted
//! memory. `Domain::Guest` alone only distinguishes trusted from untrusted;
//! `Sandbox` distinguishes whose untrusted memory is whose. See the module
//! docs for exactly what it does and does not isolate (memory: yes;
//! resource ids: a shared namespace, stated rather than hidden).

pub mod concurrent;
pub mod interpreter;
pub mod lexer;
pub mod parser;
pub mod sandbox;
pub mod vm;

use std::fmt;

pub use interpreter::{
    execute, execute_with, BranchEvent, Execution, ForkEvent, InversionEvent, RuntimeTask,
};
pub use lexer::lex;
pub use parser::{parse, Address, Alignment, Stmt, REFERENCE_SCALE};
pub use sandbox::{Owner, Sandbox, SandboxError};
pub use vm::{compile, Domain, Instruction, ProgramOutcome, TrapAction, Vm, VmFault};

/// Named for the failure, per `_mkb/test-doctrine.md`.
#[derive(Debug, Clone, PartialEq)]
pub enum LangError {
    /// A construct that would reintroduce Boolean logic. **The A2 refusal.**
    ForbiddenBooleanConstruct {
        construct: String,
        reason: String,
        line: usize,
    },
    /// Input that is not a token of this language.
    UnexpectedCharacter { text: String, line: usize },
    /// A token where the grammar wanted a different one.
    UnexpectedToken {
        found: String,
        expected: String,
        line: usize,
    },
    /// Source ran out mid-construct.
    UnexpectedEnd { expected: String },
    /// A name used before it was declared.
    UndeclaredTask { name: String },
    /// The same name declared twice.
    DuplicateTask { name: String },
    /// `E = C_H * nu` cannot price this: not finite, or not positive.
    UnphysicalFrequency { name: String, frequency: f64 },
    /// The fork unit is outside `(x)`'s domain.
    ForkOutsideDomain { name: String, unit: f64 },
    /// A1 returned a fractional child count. Refused, never rounded.
    NonIntegralFork { name: String, children: f64 },
    /// A scale outside `xi`'s domain, which is `[0, inf)`.
    UndefinedScale { name: String, scale: f64 },
    /// The resonance gate is undefined for this pair: `xi(r) -> 0` at large
    /// scale, so the mean effective frequency collapsed and the detuning ratio
    /// is `0/0`. Refused rather than answered.
    UnresonatablePair { left: String, right: String },
    /// A `store`/`load` cell ordinal that is negative, non-finite, or
    /// fractional — it indexes `MemoryPool::address_at`, not a physical
    /// quantity, so there is no meaning to round toward.
    InvalidCellOrdinal { name: String, value: f64, line: usize },
    /// An `acquire`/`release` resource id with the same defect.
    InvalidResourceId { value: f64, line: usize },
    /// `store`, `load`, `acquire`, `release`, or `halt` reached the
    /// tree-walking interpreter, which has no memory pool or resource
    /// tracker to act against. Real execution of these needs
    /// [`vm::compile`]/[`vm::Vm`] — see `_mkb/instruction_set.md`.
    RequiresVm { construct: &'static str },
}

impl fmt::Display for LangError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ForbiddenBooleanConstruct {
                construct,
                reason,
                line,
            } => write!(
                f,
                "line {line}: `{construct}` is not a construct of this language \
                 (axiom A2): {reason}"
            ),
            Self::UnexpectedCharacter { text, line } => {
                write!(f, "line {line}: `{text}` is not a token of this language")
            }
            Self::UnexpectedToken {
                found,
                expected,
                line,
            } => write!(f, "line {line}: expected {expected}, found {found}"),
            Self::UnexpectedEnd { expected } => {
                write!(f, "source ended while expecting {expected}")
            }
            Self::UndeclaredTask { name } => {
                write!(f, "task `{name}` is used but never declared")
            }
            Self::DuplicateTask { name } => write!(f, "task `{name}` is declared twice"),
            Self::UnphysicalFrequency { name, frequency } => write!(
                f,
                "task `{name}`: frequency {frequency} is unphysical; \
                 E = C_H*nu needs a finite positive nu"
            ),
            Self::ForkOutsideDomain { name, unit } => write!(
                f,
                "task `{name}`: fork unit {unit} is outside the domain of (x)"
            ),
            Self::NonIntegralFork { name, children } => write!(
                f,
                "task `{name}`: A1 gives {children} children, which is not a \
                 whole number of execution units"
            ),
            Self::UndefinedScale { name, scale } => write!(
                f,
                "task `{name}`: scale {scale} is outside xi's domain [0, inf)"
            ),
            Self::UnresonatablePair { left, right } => write!(
                f,
                "`{left}` and `{right}` have no resonance: their scale-corrected \
                 frequencies collapse to zero, so detuning is undefined"
            ),
            Self::InvalidCellOrdinal { name, value, line } => write!(
                f,
                "line {line}: `{name}`'s cell ordinal {value} must be a whole number >= 0"
            ),
            Self::InvalidResourceId { value, line } => write!(
                f,
                "line {line}: resource id {value} must be a whole number >= 0"
            ),
            Self::RequiresVm { construct } => write!(
                f,
                "`{construct}` has no meaning to the tree-walking interpreter \
                 (no memory pool or resource tracker); run it through vm::Vm instead"
            ),
        }
    }
}

impl std::error::Error for LangError {}

/// Lex, parse, and execute Symphony source in one call.
///
/// # Errors
/// Any [`LangError`]; the A2 refusal fires first, before parsing.
pub fn run(source: &str) -> Result<Execution, LangError> {
    let tokens = lex(source)?;
    let program = parse(&tokens)?;
    execute(&program)
}

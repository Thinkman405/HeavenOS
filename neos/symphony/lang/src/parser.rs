//! Parser for the Symphony DSL.
//!
//! Grammar:
//!
//! ```text
//! program   := statement*
//! statement := task | branch | fork | invert | emit
//!            | store | load | acquire | release | halt
//! task      := "task" IDENT "at" NUMBER "hz" "phase" ("+" | "-")
//!              [ "scale" NUMBER ]
//! branch    := "when" IDENT relation IDENT "{" statement* "}"
//! relation  := "aligns" | "opposes" | "resonates" | "detunes"
//! fork      := "fork" IDENT
//! invert    := "invert" IDENT
//! emit      := "emit" IDENT
//! store     := "store" IDENT "at" address
//! load      := "load" IDENT "at" address
//! address   := "cell" NUMBER | "path" NUMBER NUMBER*
//! acquire   := "acquire" NUMBER
//! release   := "release" NUMBER
//! halt      := "halt"
//! ```
//!
//! There is no `if`, no `else`, and no Boolean expression grammar. A branch
//! names two oscillators and asks a *physical* question about them — that is
//! the only conditional the language has.
//!
//! The four relations are the surface form of two of PRD section 3's three
//! gates: `aligns`/`opposes` test interference, `resonates`/`detunes` test
//! scale-corrected standing-wave survival. The third gate, phase inversion, is
//! a statement rather than a relation because it *transforms* rather than
//! *tests*. Law: `_mkb/gates.md`.
//!
//! `store`/`load`/`acquire`/`release`/`halt` are the instruction-executing
//! state machine's real memory and resource operations — see
//! `_mkb/instruction_set.md` and [`crate::vm`]. They parse into the same
//! [`Stmt`] tree as everything else, but only [`crate::vm::compile`] gives
//! them meaning: the tree-walking [`crate::interpreter`] refuses them
//! (`LangError::RequiresVm`), since it has no memory pool or resource tracker
//! to act against.
//!
//! `store`/`load` name a memory location one of two ways, both real
//! `LatticeAddress`es, never a flat offset — [`Address`]: `cell N` (an
//! ordinal into the pool's own ring order, `MemoryPool::address_at`) or
//! `path START S1 S2 ...` (a real `lattice::AddressPath`, the same `⊗`-fold
//! directory-style addressing `lattice::addressing` already defines,
//! resolved through `MemoryPool::resolve_path`). Both existed from the first
//! `vm` slice's cell-ordinal form; path addressing closes the deliberate
//! scope boundary that slice recorded in `_mkb/instruction_set.md`.

use crate::lexer::{Spanned, Token};
use crate::LangError;
use symphony_kernel::bifurcation::Phase;

/// The default observation scale.
///
/// `xi(R) = 1` exactly, so an undeclared scale applies no correction and the
/// effective frequency is the nominal one. Omitting `scale` is therefore the
/// same as writing the reference scale, not a special case in the interpreter.
pub const REFERENCE_SCALE: f64 = 1.0;

/// What a branch asks about two oscillators.
///
/// Two gates, two polarities each. Both gates return
/// `symphony_kernel::Interference`, so the polarities pair the same way: one
/// form runs on constructive, the other on destructive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Alignment {
    /// Gate 1. Body runs when the phases interfere constructively.
    Aligns,
    /// Gate 1. Body runs when they cancel.
    Opposes,
    /// Gate 3. Body runs when the two sustain a standing wave.
    Resonates,
    /// Gate 3. Body runs when they drift apart.
    Detunes,
}

impl Alignment {
    /// Whether this relation reads phase (gate 1) or frequency-and-scale
    /// (gate 3). The two gates are independent: neither reads what the other
    /// does, and they disagree on pairs — see `_mkb/gates.md` section 3.6.
    pub fn reads_phase(self) -> bool {
        matches!(self, Self::Aligns | Self::Opposes)
    }
}

/// How `store`/`load` name a memory location — always a real
/// `LatticeAddress` once resolved, never a flat offset.
#[derive(Debug, Clone, PartialEq)]
pub enum Address {
    /// An ordinal into the pool's own ring order (`MemoryPool::address_at`).
    Cell(usize),
    /// A `⊗`-fold directory-style path (`lattice::AddressPath`), resolved
    /// through `MemoryPool::resolve_path`. `steps` may be empty — a bare
    /// `path START` resolves to the cell nearest `START` on the tiling's
    /// reference geodesic, per `lattice::addressing`.
    Path { start: f64, steps: Vec<f64> },
}

/// A parsed statement.
#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    /// `task NAME at HZ hz phase ± [scale S]`
    Task {
        name: String,
        frequency: f64,
        phase: Phase,
        /// Observation scale. [`REFERENCE_SCALE`] when omitted.
        scale: f64,
    },
    /// `when A aligns|opposes B { .. }`
    Branch {
        left: String,
        alignment: Alignment,
        right: String,
        body: Vec<Stmt>,
    },
    /// `fork NAME`
    Fork { name: String },
    /// `invert NAME` — gate 2, the exact `pi` shift.
    Invert { name: String },
    /// `emit NAME`
    Emit { name: String },
    /// `store NAME at <address>` — write NAME's physical state into curved memory.
    Store { name: String, address: Address },
    /// `load NAME at <address>` — declare NAME by reading its state back.
    Load { name: String, address: Address },
    /// `acquire N` — acquire resource `N` for the running program.
    Acquire { resource: u64 },
    /// `release N` — release resource `N`.
    Release { resource: u64 },
    /// `halt` — end the running program.
    Halt,
}

struct Cursor<'a> {
    toks: &'a [Spanned],
    pos: usize,
}

impl<'a> Cursor<'a> {
    fn peek(&self) -> Option<&Token> {
        self.toks.get(self.pos).map(|s| &s.token)
    }
    fn line(&self) -> usize {
        self.toks
            .get(self.pos)
            .or_else(|| self.toks.last())
            .map_or(0, |s| s.line)
    }
    fn next(&mut self) -> Option<&Token> {
        let t = self.toks.get(self.pos).map(|s| &s.token);
        if t.is_some() {
            self.pos += 1;
        }
        t
    }
    fn expect(&mut self, want: &Token) -> Result<(), LangError> {
        let line = self.line();
        match self.next() {
            Some(t) if t == want => Ok(()),
            Some(t) => Err(LangError::UnexpectedToken {
                found: format!("{t:?}"),
                expected: format!("{want:?}"),
                line,
            }),
            None => Err(LangError::UnexpectedEnd {
                expected: format!("{want:?}"),
            }),
        }
    }
    fn ident(&mut self) -> Result<String, LangError> {
        let line = self.line();
        match self.next() {
            Some(Token::Ident(name)) => Ok(name.clone()),
            Some(t) => Err(LangError::UnexpectedToken {
                found: format!("{t:?}"),
                expected: "identifier".into(),
                line,
            }),
            None => Err(LangError::UnexpectedEnd {
                expected: "identifier".into(),
            }),
        }
    }
    fn number(&mut self) -> Result<f64, LangError> {
        let line = self.line();
        match self.next() {
            Some(Token::Number(n)) => Ok(*n),
            Some(t) => Err(LangError::UnexpectedToken {
                found: format!("{t:?}"),
                expected: "number".into(),
                line,
            }),
            None => Err(LangError::UnexpectedEnd {
                expected: "number".into(),
            }),
        }
    }

    /// A cell ordinal for `store`/`load` — must be a non-negative whole
    /// number, since it indexes `MemoryPool::address_at`, not a physical
    /// quantity. Refused rather than truncated, matching `NonIntegralFork`.
    fn cell_ordinal(&mut self, name: &str, line: usize) -> Result<usize, LangError> {
        let value = self.number()?;
        if value.is_finite() && value >= 0.0 && value.fract() == 0.0 {
            Ok(value as usize)
        } else {
            Err(LangError::InvalidCellOrdinal {
                name: name.to_string(),
                value,
                line,
            })
        }
    }

    /// A resource id for `acquire`/`release` — same whole-number discipline
    /// as [`Self::cell_ordinal`], for the same reason.
    fn resource_id(&mut self, line: usize) -> Result<u64, LangError> {
        let value = self.number()?;
        if value.is_finite() && value >= 0.0 && value.fract() == 0.0 {
            Ok(value as u64)
        } else {
            Err(LangError::InvalidResourceId { value, line })
        }
    }

    /// `"cell" NUMBER | "path" NUMBER NUMBER*` — the address form
    /// `store`/`load` name a memory location by. Unlike a cell ordinal, a
    /// path's `start`/steps are real `LatticeScalar` values, not indices —
    /// no whole-number discipline applies, and no upper bound: `⊗`'s own
    /// domain (checked when the path is resolved, not here) is the only
    /// limit.
    fn address(&mut self, name: &str, line: usize) -> Result<Address, LangError> {
        match self.next() {
            Some(Token::Cell) => Ok(Address::Cell(self.cell_ordinal(name, line)?)),
            Some(Token::Path) => {
                let start = self.number()?;
                let steps = self.number_list();
                Ok(Address::Path { start, steps })
            }
            Some(t) => Err(LangError::UnexpectedToken {
                found: format!("{t:?}"),
                expected: "cell or path".into(),
                line,
            }),
            None => Err(LangError::UnexpectedEnd {
                expected: "cell or path".into(),
            }),
        }
    }

    /// Greedily consume `Number` tokens. Safe because no statement's grammar
    /// ever starts with a bare number — every statement begins with a
    /// keyword — so there is no ambiguity about where the list ends.
    fn number_list(&mut self) -> Vec<f64> {
        let mut out = Vec::new();
        while let Some(Token::Number(n)) = self.peek() {
            let n = *n;
            self.next();
            out.push(n);
        }
        out
    }
}

/// Parse a token stream into statements.
pub fn parse(tokens: &[Spanned]) -> Result<Vec<Stmt>, LangError> {
    let mut c = Cursor { toks: tokens, pos: 0 };
    let mut out = Vec::new();
    while c.peek().is_some() {
        out.push(statement(&mut c)?);
    }
    Ok(out)
}

fn statement(c: &mut Cursor) -> Result<Stmt, LangError> {
    let line = c.line();
    match c.next().cloned() {
        Some(Token::Task) => {
            let name = c.ident()?;
            c.expect(&Token::At)?;
            let frequency = c.number()?;
            c.expect(&Token::Hz)?;
            c.expect(&Token::Phase)?;
            let phase = match c.next() {
                Some(Token::Positive) => Phase::Positive,
                Some(Token::Negative) => Phase::Negative,
                Some(t) => {
                    return Err(LangError::UnexpectedToken {
                        found: format!("{t:?}"),
                        expected: "+ or -".into(),
                        line,
                    })
                }
                None => {
                    return Err(LangError::UnexpectedEnd {
                        expected: "+ or -".into(),
                    })
                }
            };
            // `scale S` is optional; omitting it means the reference scale,
            // where xi == 1 and no correction applies.
            let scale = if matches!(c.peek(), Some(Token::Scale)) {
                c.next();
                c.number()?
            } else {
                REFERENCE_SCALE
            };
            Ok(Stmt::Task {
                name,
                frequency,
                phase,
                scale,
            })
        }
        Some(Token::When) => {
            let left = c.ident()?;
            let alignment = match c.next() {
                Some(Token::Aligns) => Alignment::Aligns,
                Some(Token::Opposes) => Alignment::Opposes,
                Some(Token::Resonates) => Alignment::Resonates,
                Some(Token::Detunes) => Alignment::Detunes,
                Some(t) => {
                    return Err(LangError::UnexpectedToken {
                        found: format!("{t:?}"),
                        expected: "aligns, opposes, resonates, or detunes".into(),
                        line,
                    })
                }
                None => {
                    return Err(LangError::UnexpectedEnd {
                        expected: "aligns, opposes, resonates, or detunes".into(),
                    })
                }
            };
            let right = c.ident()?;
            c.expect(&Token::OpenBrace)?;
            let mut body = Vec::new();
            loop {
                match c.peek() {
                    Some(Token::CloseBrace) => {
                        c.next();
                        break;
                    }
                    Some(_) => body.push(statement(c)?),
                    None => {
                        return Err(LangError::UnexpectedEnd {
                            expected: "}".into(),
                        })
                    }
                }
            }
            Ok(Stmt::Branch {
                left,
                alignment,
                right,
                body,
            })
        }
        Some(Token::Fork) => Ok(Stmt::Fork { name: c.ident()? }),
        Some(Token::Invert) => Ok(Stmt::Invert { name: c.ident()? }),
        Some(Token::Emit) => Ok(Stmt::Emit { name: c.ident()? }),
        Some(Token::Store) => {
            let name = c.ident()?;
            c.expect(&Token::At)?;
            let address = c.address(&name, line)?;
            Ok(Stmt::Store { name, address })
        }
        Some(Token::Load) => {
            let name = c.ident()?;
            c.expect(&Token::At)?;
            let address = c.address(&name, line)?;
            Ok(Stmt::Load { name, address })
        }
        Some(Token::Acquire) => Ok(Stmt::Acquire {
            resource: c.resource_id(line)?,
        }),
        Some(Token::Release) => Ok(Stmt::Release {
            resource: c.resource_id(line)?,
        }),
        Some(Token::Halt) => Ok(Stmt::Halt),
        Some(t) => Err(LangError::UnexpectedToken {
            found: format!("{t:?}"),
            expected: "task, when, fork, invert, emit, store, load, acquire, release, or halt"
                .into(),
            line,
        }),
        None => Err(LangError::UnexpectedEnd {
            expected: "a statement".into(),
        }),
    }
}

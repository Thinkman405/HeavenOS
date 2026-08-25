//! Executes a parsed Symphony program against the kernel.
//!
//! This module is where the [`TaskModel`] seam finally closes. The kernel has
//! carried that trait unimplemented since it was written — its doc comment says
//! outright that nothing depends on it yet and that it exists to fix the shape
//! of the seam. [`RuntimeTask`] is the first implementor.
//!
//! Everything decided here is decided by the **kernel**, not re-derived:
//!
//! | Question | Answered by |
//! |---|---|
//! | do two phases interfere? (gate 1) | [`symphony_kernel::evaluate_branch`] |
//! | what is a phase inverted? (gate 2) | [`symphony_kernel::Phase::invert`] |
//! | do two oscillators resonate? (gate 3) | [`symphony_kernel::resonates`] |
//! | how many children does a fork produce? | [`symphony_kernel::fork`] |
//! | what does a task cost? | `symphony_kernel::Task::energy_joules` |
//!
//! The language contributes syntax and scoping. It contributes no physics.

use std::collections::BTreeMap;

use symphony_kernel::bifurcation::TaskModel;
use symphony_kernel::{
    evaluate_branch, fork, resonates, Bifurcation, Interference, Phase, Task,
};

use crate::parser::{Alignment, Stmt, REFERENCE_SCALE};
use crate::LangError;

/// A task as the language declares it — and the kernel's [`TaskModel`].
#[derive(Debug, Clone, PartialEq)]
pub struct RuntimeTask {
    pub name: String,
    frequency: f64,
    phase: Phase,
    scale: f64,
    fork_unit: f64,
}

impl RuntimeTask {
    /// Declare a task at the reference scale.
    ///
    /// # Errors
    /// [`LangError::UnphysicalFrequency`] for a frequency that is not finite and
    /// positive. `E = C_H * nu` prices work by frequency; a negative frequency
    /// is a negative energy, and a task at exactly zero is born reclaimable.
    pub fn new(name: impl Into<String>, frequency: f64, phase: Phase) -> Result<Self, LangError> {
        Self::at_scale(name, frequency, phase, REFERENCE_SCALE)
    }

    /// Declare a task at an explicit observation scale.
    ///
    /// # Errors
    /// [`LangError::UnphysicalFrequency`] as above, and
    /// [`LangError::UndefinedScale`] for a scale outside `xi`'s domain — `xi`
    /// is defined on `[0, inf)`, so a negative scale is not an observation
    /// point at all.
    pub fn at_scale(
        name: impl Into<String>,
        frequency: f64,
        phase: Phase,
        scale: f64,
    ) -> Result<Self, LangError> {
        let name = name.into();
        if !frequency.is_finite() || frequency <= 0.0 {
            return Err(LangError::UnphysicalFrequency { name, frequency });
        }
        if !(scale >= 0.0) || scale.is_nan() {
            return Err(LangError::UndefinedScale { name, scale });
        }
        Ok(Self {
            name,
            frequency,
            phase,
            scale,
            fork_unit: 1.0,
        })
    }

    /// Observation scale. `xi(R) = 1`, so the default applies no correction.
    pub fn scale(&self) -> f64 {
        self.scale
    }

    /// Override the fork unit.
    ///
    /// Surface syntax has no way to set this — every *declared* task forks at
    /// the canonical `1`, the only unit A1 gives an exact answer for. A host
    /// embedding this language can seed a differently-shaped task through
    /// [`execute_with`], which is what makes the non-integral-fork refusal a
    /// reachable guard rather than decoration.
    pub fn with_fork_unit(mut self, unit: f64) -> Self {
        self.fork_unit = unit;
        self
    }

    /// Set this task's phase directly.
    ///
    /// `pub(crate)`, not `pub`: `Runner::run` (this module) mutates the
    /// `phase` field it already owns for `Stmt::Invert`; `vm::Vm` needs the
    /// same power from a different module in the same crate for
    /// `Instruction::Invert`. Not exposed outside the crate — a host embeds
    /// this language through `invert NAME`/`Instruction::Invert`, never by
    /// constructing an arbitrary orientation directly.
    pub(crate) fn set_phase(&mut self, phase: Phase) {
        self.phase = phase;
    }
}

impl TaskModel for RuntimeTask {
    fn frequency(&self) -> f64 {
        self.frequency
    }

    fn guard_phase(&self) -> Phase {
        self.phase
    }

    fn fork_unit(&self) -> f64 {
        self.fork_unit
    }
}

/// One `fork` statement, and what the axiom gave back.
#[derive(Debug, Clone, PartialEq)]
pub struct ForkEvent {
    pub task: String,
    pub bifurcation: Bifurcation,
}

/// One `invert` statement. Gate 2 is total, so there is nothing to refuse.
#[derive(Debug, Clone, PartialEq)]
pub struct InversionEvent {
    pub task: String,
    pub before: Phase,
    pub after: Phase,
}

/// One `when` statement, and how its gate answered.
#[derive(Debug, Clone, PartialEq)]
pub struct BranchEvent {
    pub left: String,
    pub right: String,
    /// Which gate was asked — `aligns`/`opposes` read phase, and
    /// `resonates`/`detunes` read frequency and scale.
    pub alignment: Alignment,
    pub interference: Interference,
    /// Whether the body ran. Each gate has two polarities, so this is not
    /// simply the interference.
    pub taken: bool,
}

/// The result of running a program.
#[derive(Debug, Clone, PartialEq)]
pub struct Execution {
    /// Tasks the program declared, by name.
    pub declared: BTreeMap<String, RuntimeTask>,
    /// Kernel tasks ready to hand to `Scheduler::ingest`, in emission order.
    pub emitted: Vec<Task>,
    pub forks: Vec<ForkEvent>,
    pub inversions: Vec<InversionEvent>,
    pub branches: Vec<BranchEvent>,
}

impl Execution {
    /// Total energy of everything emitted, priced by the kernel.
    pub fn total_energy_joules(&self) -> f64 {
        self.emitted.iter().map(Task::energy_joules).sum()
    }
}

struct Runner {
    declared: BTreeMap<String, RuntimeTask>,
    emitted: Vec<Task>,
    forks: Vec<ForkEvent>,
    inversions: Vec<InversionEvent>,
    branches: Vec<BranchEvent>,
    next_id: u64,
}

impl Runner {
    fn lookup(&self, name: &str) -> Result<&RuntimeTask, LangError> {
        self.declared
            .get(name)
            .ok_or_else(|| LangError::UndeclaredTask {
                name: name.to_string(),
            })
    }

    fn emit(&mut self, task: &RuntimeTask) {
        // `with_scale` carries the observation scale into the kernel, which
        // applies xi in `energy_joules`. The language does not pre-multiply it:
        // that would be a second home for the correction.
        self.emitted
            .push(Task::new(self.next_id, task.frequency()).with_scale(task.scale()));
        self.next_id += 1;
    }

    fn run(&mut self, stmts: &[Stmt]) -> Result<(), LangError> {
        for stmt in stmts {
            match stmt {
                Stmt::Task {
                    name,
                    frequency,
                    phase,
                    scale,
                } => {
                    if self.declared.contains_key(name) {
                        return Err(LangError::DuplicateTask { name: name.clone() });
                    }
                    let task = RuntimeTask::at_scale(name.clone(), *frequency, *phase, *scale)?;
                    self.declared.insert(name.clone(), task);
                }

                Stmt::Invert { name } => {
                    // Gate 2. Total by construction — A2's orientation set is
                    // closed under the pi shift, so there is no failure case
                    // and no domain check. See _mkb/gates.md section 2.
                    let task = self
                        .declared
                        .get_mut(name)
                        .ok_or_else(|| LangError::UndeclaredTask { name: name.clone() })?;
                    let before = task.phase;
                    task.phase = before.invert();
                    self.inversions.push(InversionEvent {
                        task: name.clone(),
                        before,
                        after: task.phase,
                    });
                }

                Stmt::Emit { name } => {
                    let task = self.lookup(name)?.clone();
                    self.emit(&task);
                }

                Stmt::Fork { name } => {
                    let task = self.lookup(name)?.clone();
                    let bifurcation =
                        fork(task.fork_unit()).map_err(|_| LangError::ForkOutsideDomain {
                            name: name.clone(),
                            unit: task.fork_unit(),
                        })?;

                    // A1 gives a child *count*. For the canonical unit it is
                    // exactly 2. Any other unit generally is not an integer, and
                    // a fractional child is not a thing the runtime can create —
                    // so it is refused rather than rounded.
                    let children = bifurcation.children;
                    if !children.is_finite() || children.fract() != 0.0 || children < 1.0 {
                        return Err(LangError::NonIntegralFork {
                            name: name.clone(),
                            children,
                        });
                    }

                    for _ in 0..(children as u64) {
                        self.emit(&task);
                    }
                    self.forks.push(ForkEvent {
                        task: name.clone(),
                        bifurcation,
                    });
                }

                Stmt::Branch {
                    left,
                    alignment,
                    right,
                    body,
                } => {
                    let a = self.lookup(left)?.clone();
                    let b = self.lookup(right)?.clone();

                    // Which gate runs is decided by the relation the programmer
                    // wrote. The two read disjoint inputs and can disagree on
                    // the same pair — see _mkb/gates.md section 3.6.
                    let interference = if alignment.reads_phase() {
                        // Gate 1: interference of phase orientations.
                        evaluate_branch(a.guard_phase(), b.guard_phase())
                    } else {
                        // Gate 3: scale-corrected standing-wave survival.
                        resonates(a.frequency(), a.scale(), b.frequency(), b.scale()).map_err(
                            |_| LangError::UnresonatablePair {
                                left: left.clone(),
                                right: right.clone(),
                            },
                        )?
                    };
                    // Written as an explicit pairing rather than `!aligns`.
                    //
                    // Those two are *currently* the same function: A2 admits
                    // exactly two phase orientations, so alignment is a
                    // two-valued predicate and its outcomes partition. Sabotage
                    // confirmed this — swapping to `!aligns` broke no test,
                    // because it is not a mutation.
                    //
                    // The pairing is kept because it depends on the *law*
                    // (constructive means aligned) rather than on `Interference`
                    // happening to have two variants. Where `opposes` genuinely
                    // differs from `else` is structural, not per-branch: see
                    // `branch_forms_are_independent_statements`.
                    let taken = match (alignment, interference) {
                        (Alignment::Aligns, Interference::Constructive) => true,
                        (Alignment::Opposes, Interference::Destructive) => true,
                        (Alignment::Resonates, Interference::Constructive) => true,
                        (Alignment::Detunes, Interference::Destructive) => true,
                        _ => false,
                    };
                    self.branches.push(BranchEvent {
                        left: left.clone(),
                        right: right.clone(),
                        alignment: *alignment,
                        interference,
                        taken,
                    });
                    if taken {
                        self.run(body)?;
                    }
                }

                // Real memory and resource operations have no meaning here:
                // this walker has no `&mut MemoryPool` or `ResourceTracker` to
                // act against, and taking one would give `execute`/`run` a
                // side-effectful signature every existing call site does not
                // expect. See `_mkb/instruction_set.md` and `crate::vm`.
                Stmt::Store { .. } => {
                    return Err(LangError::RequiresVm { construct: "store" })
                }
                Stmt::Load { .. } => return Err(LangError::RequiresVm { construct: "load" }),
                Stmt::Acquire { .. } => {
                    return Err(LangError::RequiresVm { construct: "acquire" })
                }
                Stmt::Release { .. } => {
                    return Err(LangError::RequiresVm { construct: "release" })
                }
                Stmt::Halt => return Err(LangError::RequiresVm { construct: "halt" }),
            }
        }
        Ok(())
    }
}

/// Execute a parsed program.
pub fn execute(program: &[Stmt]) -> Result<Execution, LangError> {
    execute_with(program, Vec::new())
}

/// Execute a parsed program against a pre-seeded environment.
///
/// The embedding seam: a host supplies tasks the source did not declare, and
/// the program may `emit`, `fork`, and branch on them exactly as if it had.
/// Redeclaring a seeded name is still a [`LangError::DuplicateTask`] — seeding
/// adds to the environment, it does not open a back door around scoping.
///
/// This is also what makes [`LangError::NonIntegralFork`] reachable: surface
/// syntax always forks at the canonical unit, so without a seeded task the
/// guard could never fire and a test asserting it would be asserting nothing.
pub fn execute_with(
    program: &[Stmt],
    seed: impl IntoIterator<Item = RuntimeTask>,
) -> Result<Execution, LangError> {
    let mut declared = BTreeMap::new();
    for task in seed {
        declared.insert(task.name.clone(), task);
    }
    let mut runner = Runner {
        declared,
        emitted: Vec::new(),
        forks: Vec::new(),
        inversions: Vec::new(),
        branches: Vec::new(),
        next_id: 0,
    };
    runner.run(program)?;
    Ok(Execution {
        declared: runner.declared,
        emitted: runner.emitted,
        forks: runner.forks,
        inversions: runner.inversions,
        branches: runner.branches,
    })
}

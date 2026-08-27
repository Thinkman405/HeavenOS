//! The instruction-executing state machine. Law: `_mkb/instruction_set.md`.
//!
//! Where [`crate::interpreter`] walks the parsed [`Stmt`] tree recursively,
//! this module [`compile`]s it into a flat, program-counter-addressed
//! [`Instruction`] sequence and runs it with a real dispatch loop. That
//! flattening is what makes a faulting program's execution a thing that can
//! *stop* — trap — rather than a Rust call stack that has to unwind: there is
//! a `pc` to abandon, not recursive frames to unwind through.
//!
//! `compile` preserves the tree-walker's exact dispatch semantics: a
//! `Branch`'s body becomes an `Eval` instruction immediately followed by its
//! flattened body, with `skip` recording exactly how far to jump when the
//! gate does not fire. `store`/`load`/`acquire`/`release`/`halt` have no
//! tree-walker equivalent at all — [`crate::interpreter`] refuses them
//! (`LangError::RequiresVm`) rather than pretending to run them without the
//! real memory pool and resource tracker this module actually threads
//! through.
//!
//! [`Vm::run_batch`] is the isolation boundary: several compiled programs run
//! against one shared [`MemoryPool`] and [`ResourceTracker`]/[`WaitForGraph`]
//! pair — the real kernel-lattice state — and a trap in one program stops
//! only that program's own dispatch loop. The rest of the batch keeps running
//! against the same, undamaged shared state.

use std::collections::{BTreeMap, HashSet};

use lattice::tessellation::CellId;
use lattice::AddressPath;
use substrate::{LatticeAddress, MemoryPool, SubstrateError};
pub use substrate::TrapAction;
use symphony_kernel::bifurcation::TaskModel;
use symphony_kernel::{
    evaluate_branch, fork, resonates, Acquired, Interference, Phase, ResourceError, ResourceId,
    ResourceTracker, Task, TaskId, WaitForGraph,
};

use crate::interpreter::{BranchEvent, ForkEvent, InversionEvent, RuntimeTask};
use crate::parser::{Address, Alignment, Stmt};
use crate::LangError;

/// One flat, program-counter-addressed instruction. See module docs.
#[derive(Debug, Clone, PartialEq)]
pub enum Instruction {
    Task {
        name: String,
        frequency: f64,
        phase: Phase,
        scale: f64,
    },
    Invert {
        name: String,
    },
    Fork {
        name: String,
    },
    Emit {
        name: String,
    },
    /// Gate 1 or gate 3, discriminated by `alignment` exactly as
    /// `Stmt::Branch` already is — `EVAL` and `RESONATE` in
    /// `_mkb/instruction_set.md` are this one instruction, not two.
    /// `skip` is how many instructions to jump over — the flattened
    /// body — when the gate does not fire.
    Eval {
        left: String,
        alignment: Alignment,
        right: String,
        skip: usize,
    },
    Store {
        name: String,
        address: Address,
    },
    Load {
        name: String,
        address: Address,
    },
    Acquire {
        resource: u64,
    },
    Release {
        resource: u64,
    },
    Halt,
}

/// Compile a parsed program into a flat instruction sequence.
///
/// A `Branch`'s body is flattened in place immediately behind its `Eval`,
/// which is what turns "walk into the body" into "fall through to the next
/// instruction, or skip past it" — the tree structure becomes control flow
/// over a flat address space instead of recursion.
pub fn compile(program: &[Stmt]) -> Vec<Instruction> {
    let mut out = Vec::new();
    compile_into(program, &mut out);
    out
}

fn compile_into(program: &[Stmt], out: &mut Vec<Instruction>) {
    for stmt in program {
        match stmt {
            Stmt::Task {
                name,
                frequency,
                phase,
                scale,
            } => out.push(Instruction::Task {
                name: name.clone(),
                frequency: *frequency,
                phase: *phase,
                scale: *scale,
            }),
            Stmt::Invert { name } => out.push(Instruction::Invert { name: name.clone() }),
            Stmt::Fork { name } => out.push(Instruction::Fork { name: name.clone() }),
            Stmt::Emit { name } => out.push(Instruction::Emit { name: name.clone() }),
            Stmt::Store { name, address } => out.push(Instruction::Store {
                name: name.clone(),
                address: address.clone(),
            }),
            Stmt::Load { name, address } => out.push(Instruction::Load {
                name: name.clone(),
                address: address.clone(),
            }),
            Stmt::Acquire { resource } => out.push(Instruction::Acquire { resource: *resource }),
            Stmt::Release { resource } => out.push(Instruction::Release { resource: *resource }),
            Stmt::Halt => out.push(Instruction::Halt),
            Stmt::Branch {
                left,
                alignment,
                right,
                body,
            } => {
                let eval_at = out.len();
                out.push(Instruction::Eval {
                    left: left.clone(),
                    alignment: *alignment,
                    right: right.clone(),
                    skip: 0, // patched once the body's flattened length is known
                });
                let body_start = out.len();
                compile_into(body, out);
                let body_len = out.len() - body_start;
                if let Instruction::Eval { skip, .. } = &mut out[eval_at] {
                    *skip = body_len;
                }
            }
        }
    }
}

/// One `acquire`/`release` and its real outcome.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceOutcome {
    Granted,
    Released { next: Option<TaskId> },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResourceEvent {
    pub resource: ResourceId,
    pub outcome: ResourceOutcome,
}

/// Why a program's dispatch loop stopped before reaching a `Halt` or the end
/// of its instructions. Every variant is a real value returned from a real
/// subsystem call — never a Rust panic. See `_mkb/instruction_set.md`.
#[derive(Debug, Clone, PartialEq)]
pub enum VmFault {
    /// A `store`/`load` hit a real fault from `substrate::MemoryPool`.
    Memory(SubstrateError),
    /// A `store`/`load` cell ordinal is past the end of this pool. Not a
    /// `SubstrateError`: no `CellId` was ever named, so it is not "unmapped"
    /// in the sense `substrate` uses that word for. Only reachable from
    /// `Address::Cell` — a `Path` that leaves `⊗`'s domain or names a cell
    /// outside this pool surfaces as `Memory` instead, since
    /// `MemoryPool::resolve_path` already gives those a real
    /// `SubstrateError`.
    CellOutOfRange { cell: usize, cells: usize },
    /// A `load` read back bytes that do not decode to a physical task state
    /// — a cell nothing ever `store`d into (zero-initialised memory), or a
    /// phase angle that survived the byte round-trip but is neither of A2's
    /// two orientations.
    CorruptState { address: Address },
    /// `acquire` would block. This VM runs one program to completion at a
    /// time within a batch — there is no scheduler able to suspend and later
    /// resume a blocked program — so blocking is a trap, a stated real limit
    /// rather than a hang or a silent "granted anyway."
    Blocked { resource: ResourceId, holder: TaskId },
    /// `release` on a resource this program does not hold.
    Resource(ResourceError),
    /// A language-level fault that would also stop the tree-walker —
    /// `UndeclaredTask`, `NonIntegralFork`, and so on.
    Lang(LangError),
    /// A [`Domain::Guest`] program's `store`/`load` resolved to a cell
    /// [`Vm::reserve_cells`] marked kernel-only. Checked against the
    /// **resolved** address, not the source `Address` a program wrote — the
    /// same place a real MMU checks a translated address, not a virtual
    /// one. Never offered to a `run_program_trapped` handler: retrying
    /// wouldn't change the domain, and a handler that could "fix" a
    /// privilege fault would make the boundary meaningless.
    PrivilegeViolation { address: Address, cell: CellId },
}

/// One program's complete run: what it produced before it halted or trapped.
#[derive(Debug, Clone, PartialEq)]
pub struct ProgramOutcome {
    pub task_id: TaskId,
    pub declared: BTreeMap<String, RuntimeTask>,
    pub emitted: Vec<Task>,
    pub forks: Vec<ForkEvent>,
    pub inversions: Vec<InversionEvent>,
    pub branches: Vec<BranchEvent>,
    pub stores: Vec<(String, Address)>,
    pub loads: Vec<(String, Address)>,
    pub resources: Vec<ResourceEvent>,
    /// `true` iff the program reached `Halt` or ran off the end of its
    /// instructions without a fault. `false` iff `trap` stopped it early.
    pub halted: bool,
    pub trap: Option<VmFault>,
}

/// Three `f64` (frequency, `Phase::radians()`, scale), IEEE-754
/// little-endian — a straight byte round-trip with no arithmetic performed
/// on the stored value, so reconstruction is bit-exact by construction. See
/// `_mkb/instruction_set.md`'s "What STORE/LOAD deliberately are not".
pub(crate) const STATE_BYTES: usize = 24;

pub(crate) fn encode_state(task: &RuntimeTask) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(STATE_BYTES);
    bytes.extend_from_slice(&task.frequency().to_le_bytes());
    bytes.extend_from_slice(&task.guard_phase().radians().to_le_bytes());
    bytes.extend_from_slice(&task.scale().to_le_bytes());
    bytes
}

pub(crate) fn decode_state(bytes: &[u8]) -> Option<(f64, f64, f64)> {
    if bytes.len() < STATE_BYTES {
        return None;
    }
    let frequency = f64::from_le_bytes(bytes[0..8].try_into().ok()?);
    let phase_radians = f64::from_le_bytes(bytes[8..16].try_into().ok()?);
    let scale = f64::from_le_bytes(bytes[16..24].try_into().ok()?);
    Some((frequency, phase_radians, scale))
}

/// Which domain a running program executes in — **a stated engineering
/// convention, not law**. Unlike everything else in this module, nothing in
/// `_mkb/` defines privilege, guest isolation, or kernel/guest separation;
/// there is no formula here to derive or compose, only a design decision to
/// state plainly, the same way the demo binary's deadlock victim policy is
/// stated as "a choice rather than a derived fact" rather than invented
/// physics. Two-valued deliberately, in the same spirit `Phase`/`Alignment`/
/// `Acquired` already are under A2 — a real domain distinction, not a `bool`
/// wearing a different name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Domain {
    /// Unrestricted: may `store`/`load` any cell this pool maps.
    Kernel,
    /// Restricted: a `store`/`load` resolving to a cell the `Vm` has marked
    /// reserved is refused — see [`Vm::reserve_cells`].
    Guest,
}

/// The shared state a batch of programs runs against — the real
/// kernel-lattice state the isolation guarantee in
/// `_mkb/instruction_set.md` is about. Borrowed, not owned: the same pool
/// and tracker a host (the demo binary, or a future scheduler integration)
/// already has real state in.
pub struct Vm<'a> {
    pool: &'a mut MemoryPool,
    tracker: &'a mut ResourceTracker,
    graph: &'a mut WaitForGraph,
    /// Cells no [`Domain::Guest`] program may `store`/`load` against — a
    /// "system-critical Tetryen patch" is just the set of cells it occupies,
    /// so this same mechanism protects a patch or a single cell alike.
    /// Empty by default: [`Vm::new`] grants full access, matching every
    /// existing caller's behaviour before this field existed.
    reserved: HashSet<CellId>,
}

impl<'a> Vm<'a> {
    pub fn new(
        pool: &'a mut MemoryPool,
        tracker: &'a mut ResourceTracker,
        graph: &'a mut WaitForGraph,
    ) -> Self {
        Self {
            pool,
            tracker,
            graph,
            reserved: HashSet::new(),
        }
    }

    /// Mark cells kernel-only: a [`Domain::Guest`] program's `store`/`load`
    /// resolving to any of them traps with [`VmFault::PrivilegeViolation`]
    /// rather than reaching the pool at all. [`Domain::Kernel`] programs are
    /// unaffected — reservation is a restriction on guests, not a lock on
    /// the cells themselves.
    pub fn reserve_cells(&mut self, cells: impl IntoIterator<Item = CellId>) {
        self.reserved.extend(cells);
    }

    /// Resolve a `store`/`load` [`Address`] to a real [`LatticeAddress`] —
    /// the one place both addressing modes converge, so `Instruction::Store`
    /// and `Instruction::Load` share exactly one resolution path rather than
    /// each re-implementing the `Cell`/`Path` split.
    fn resolve(&self, address: &Address) -> Result<LatticeAddress, VmFault> {
        match address {
            Address::Cell(n) => self.pool.address_at(*n).ok_or(VmFault::CellOutOfRange {
                cell: *n,
                cells: self.pool.cell_count(),
            }),
            Address::Path { start, steps } => {
                let path = AddressPath::new(*start, steps);
                self.pool.resolve_path(&path).map_err(VmFault::Memory)
            }
        }
    }

    /// Run one compiled program to completion or its first trap. Never
    /// retries a memory fault — equivalent to [`Self::run_program_trapped`]
    /// with `max_retries: 0` and a handler that always propagates.
    ///
    /// `task_id` is this program's identity for resource tracking — the
    /// "running thread" `_mkb/instruction_set.md` talks about isolating. It
    /// is unrelated to the oscillator names the program itself declares.
    pub fn run_program(&mut self, task_id: TaskId, instructions: &[Instruction]) -> ProgramOutcome {
        self.run_program_trapped(task_id, Domain::Kernel, instructions, 0, |_, _| {
            TrapAction::Propagate
        })
    }

    /// Run one compiled program with **dynamic fault routing** on its
    /// `store`/`load` memory faults — the direct counterpart to
    /// [`substrate::Hypervisor::allocate_trapped`], same shape: `handler` is
    /// called on every memory fault, unconditionally, with `&mut MemoryPool`
    /// so a real corrective action is actually possible (most concretely,
    /// seeding a cell a `load` found corrupt), and `max_retries` bounds how
    /// many times a fault this program *itself* hits gets retried, so a
    /// handler that never resolves anything cannot hang the caller.
    ///
    /// A retry re-attempts **the exact same instruction**, not the program
    /// from the start: `pc` does not advance, and nothing accumulated so far
    /// (`declared`, `emitted`, ...) is touched, so there is no restart
    /// semantics to reason about — this is `allocate_trapped`'s own "the
    /// retried operation" idea, applied to one instruction instead of one
    /// pool call.
    ///
    /// Scoped to `store`/`load` specifically, matching
    /// `allocate_trapped`'s own scoping discipline: `UndeclaredTask`/
    /// `DuplicateTask` (from a program naming a task it never declared, or
    /// declaring one twice) are the *caller's* logic errors, not a fault a
    /// handler can fix by acting on the pool, so they are never offered to
    /// `handler` — retrying an undeclared name is still undeclared no matter
    /// how many times it's asked. [`VmFault::PrivilegeViolation`] is
    /// likewise never offered to `handler`, for a stronger reason: a
    /// handler able to retry past a privilege boundary would make the
    /// boundary meaningless, not merely unhelpful.
    ///
    /// `domain` governs whether `store`/`load` may reach every cell this
    /// pool maps ([`Domain::Kernel`]) or only cells [`Vm::reserve_cells`]
    /// has not marked reserved ([`Domain::Guest`]) — see [`Domain`].
    pub fn run_program_trapped(
        &mut self,
        task_id: TaskId,
        domain: Domain,
        instructions: &[Instruction],
        max_retries: usize,
        mut handler: impl FnMut(&VmFault, &mut MemoryPool) -> TrapAction,
    ) -> ProgramOutcome {
        let mut declared: BTreeMap<String, RuntimeTask> = BTreeMap::new();
        let mut emitted = Vec::new();
        let mut forks = Vec::new();
        let mut inversions = Vec::new();
        let mut branches = Vec::new();
        let mut stores = Vec::new();
        let mut loads = Vec::new();
        let mut resources = Vec::new();
        let mut next_id: u64 = 0;
        let mut pc = 0usize;
        let mut retries = 0usize;

        // Calls `handler` on a real memory fault and either retries this
        // same instruction (same `pc`, loop again) or traps the program —
        // the one place `run_program`/`run_program_trapped` actually differ.
        // A local macro rather than a closure: it needs to `continue`/
        // `break` the enclosing loop directly, which a closure cannot do.
        macro_rules! recover_or_trap {
            ($fault:expr) => {{
                let fault = $fault;
                match handler(&fault, self.pool) {
                    TrapAction::Retry if retries < max_retries => {
                        retries += 1;
                        continue;
                    }
                    _ => break Some(fault),
                }
            }};
        }

        let trap = loop {
            let Some(instr) = instructions.get(pc) else {
                break None;
            };
            match instr {
                Instruction::Halt => break None,

                Instruction::Task {
                    name,
                    frequency,
                    phase,
                    scale,
                } => {
                    if declared.contains_key(name) {
                        break Some(VmFault::Lang(LangError::DuplicateTask {
                            name: name.clone(),
                        }));
                    }
                    match RuntimeTask::at_scale(name.clone(), *frequency, *phase, *scale) {
                        Ok(task) => {
                            declared.insert(name.clone(), task);
                            pc += 1;
                        }
                        Err(e) => break Some(VmFault::Lang(e)),
                    }
                }

                Instruction::Invert { name } => match declared.get_mut(name) {
                    Some(task) => {
                        let before = task.guard_phase();
                        task.set_phase(before.invert());
                        inversions.push(InversionEvent {
                            task: name.clone(),
                            before,
                            after: task.guard_phase(),
                        });
                        pc += 1;
                    }
                    None => {
                        break Some(VmFault::Lang(LangError::UndeclaredTask {
                            name: name.clone(),
                        }))
                    }
                },

                Instruction::Emit { name } => match declared.get(name) {
                    Some(task) => {
                        emitted.push(Task::new(next_id, task.frequency()).with_scale(task.scale()));
                        next_id += 1;
                        pc += 1;
                    }
                    None => {
                        break Some(VmFault::Lang(LangError::UndeclaredTask {
                            name: name.clone(),
                        }))
                    }
                },

                Instruction::Fork { name } => {
                    let task = match declared.get(name) {
                        Some(t) => t.clone(),
                        None => {
                            break Some(VmFault::Lang(LangError::UndeclaredTask {
                                name: name.clone(),
                            }))
                        }
                    };
                    match fork(task.fork_unit()) {
                        Ok(bifurcation) => {
                            let children = bifurcation.children;
                            if !children.is_finite() || children.fract() != 0.0 || children < 1.0 {
                                break Some(VmFault::Lang(LangError::NonIntegralFork {
                                    name: name.clone(),
                                    children,
                                }));
                            }
                            for _ in 0..(children as u64) {
                                emitted.push(
                                    Task::new(next_id, task.frequency()).with_scale(task.scale()),
                                );
                                next_id += 1;
                            }
                            forks.push(ForkEvent {
                                task: name.clone(),
                                bifurcation,
                            });
                            pc += 1;
                        }
                        Err(_) => {
                            break Some(VmFault::Lang(LangError::ForkOutsideDomain {
                                name: name.clone(),
                                unit: task.fork_unit(),
                            }))
                        }
                    }
                }

                Instruction::Eval {
                    left,
                    alignment,
                    right,
                    skip,
                } => {
                    let a = match declared.get(left) {
                        Some(t) => t.clone(),
                        None => {
                            break Some(VmFault::Lang(LangError::UndeclaredTask {
                                name: left.clone(),
                            }))
                        }
                    };
                    let b = match declared.get(right) {
                        Some(t) => t.clone(),
                        None => {
                            break Some(VmFault::Lang(LangError::UndeclaredTask {
                                name: right.clone(),
                            }))
                        }
                    };
                    let interference = if alignment.reads_phase() {
                        evaluate_branch(a.guard_phase(), b.guard_phase())
                    } else {
                        match resonates(a.frequency(), a.scale(), b.frequency(), b.scale()) {
                            Ok(i) => i,
                            Err(_) => {
                                break Some(VmFault::Lang(LangError::UnresonatablePair {
                                    left: left.clone(),
                                    right: right.clone(),
                                }))
                            }
                        }
                    };
                    let taken = matches!(
                        (alignment, interference),
                        (Alignment::Aligns, Interference::Constructive)
                            | (Alignment::Opposes, Interference::Destructive)
                            | (Alignment::Resonates, Interference::Constructive)
                            | (Alignment::Detunes, Interference::Destructive)
                    );
                    branches.push(BranchEvent {
                        left: left.clone(),
                        right: right.clone(),
                        alignment: *alignment,
                        interference,
                        taken,
                    });
                    pc += if taken { 1 } else { 1 + *skip };
                }

                Instruction::Store { name, address } => {
                    let task = match declared.get(name) {
                        Some(t) => t.clone(),
                        None => {
                            break Some(VmFault::Lang(LangError::UndeclaredTask {
                                name: name.clone(),
                            }))
                        }
                    };
                    let addr = match self.resolve(address) {
                        Ok(a) => a,
                        Err(fault) => recover_or_trap!(fault),
                    };
                    if domain == Domain::Guest && self.reserved.contains(&addr.cell()) {
                        break Some(VmFault::PrivilegeViolation {
                            address: address.clone(),
                            cell: addr.cell(),
                        });
                    }
                    match self.pool.write(addr, &encode_state(&task)) {
                        Ok(()) => {
                            stores.push((name.clone(), address.clone()));
                            pc += 1;
                        }
                        Err(e) => recover_or_trap!(VmFault::Memory(e)),
                    }
                }

                Instruction::Load { name, address } => {
                    if declared.contains_key(name) {
                        break Some(VmFault::Lang(LangError::DuplicateTask {
                            name: name.clone(),
                        }));
                    }
                    let addr = match self.resolve(address) {
                        Ok(a) => a,
                        Err(fault) => recover_or_trap!(fault),
                    };
                    if domain == Domain::Guest && self.reserved.contains(&addr.cell()) {
                        break Some(VmFault::PrivilegeViolation {
                            address: address.clone(),
                            cell: addr.cell(),
                        });
                    }
                    match self.pool.read(addr, STATE_BYTES) {
                        Ok(bytes) => {
                            let Some((frequency, phase_radians, scale)) = decode_state(&bytes)
                            else {
                                recover_or_trap!(VmFault::CorruptState {
                                    address: address.clone(),
                                });
                            };
                            // Exact match: encode/decode is a pure byte
                            // round-trip, never a computation — see
                            // `STATE_BYTES`'s doc comment.
                            let Ok(phase) = Phase::from_radians(phase_radians, 0.0) else {
                                recover_or_trap!(VmFault::CorruptState {
                                    address: address.clone(),
                                });
                            };
                            match RuntimeTask::at_scale(name.clone(), frequency, phase, scale) {
                                Ok(task) => {
                                    declared.insert(name.clone(), task);
                                    loads.push((name.clone(), address.clone()));
                                    pc += 1;
                                }
                                Err(_) => recover_or_trap!(VmFault::CorruptState {
                                    address: address.clone(),
                                }),
                            }
                        }
                        Err(e) => recover_or_trap!(VmFault::Memory(e)),
                    }
                }

                Instruction::Acquire { resource } => {
                    let rid = ResourceId(*resource);
                    match self.tracker.acquire(task_id, rid, self.graph) {
                        Ok(Acquired::Granted) => {
                            resources.push(ResourceEvent {
                                resource: rid,
                                outcome: ResourceOutcome::Granted,
                            });
                            pc += 1;
                        }
                        Ok(Acquired::Blocked { holder }) => {
                            break Some(VmFault::Blocked {
                                resource: rid,
                                holder,
                            })
                        }
                        Err(e) => break Some(VmFault::Resource(e)),
                    }
                }

                Instruction::Release { resource } => {
                    let rid = ResourceId(*resource);
                    match self.tracker.release(task_id, rid, self.graph) {
                        Ok(next) => {
                            resources.push(ResourceEvent {
                                resource: rid,
                                outcome: ResourceOutcome::Released { next },
                            });
                            pc += 1;
                        }
                        Err(e) => break Some(VmFault::Resource(e)),
                    }
                }
            }
        };

        ProgramOutcome {
            task_id,
            declared,
            emitted,
            forks,
            inversions,
            branches,
            stores,
            loads,
            resources,
            halted: trap.is_none(),
            trap,
        }
    }

    /// Run a batch of already-compiled programs in sequence, each against
    /// the same shared memory pool and resource tracker. Program `i` gets
    /// `TaskId(i)`.
    ///
    /// **This is the isolation demonstration**: a trap in program `k` is
    /// recorded in `results[k]` alone. Programs `k+1..` still run, against
    /// the same pool and tracker `k`'s fault did not damage — because every
    /// fault path above returns a `Result`/breaks the loop with a value,
    /// never panics or corrupts shared state mid-operation.
    pub fn run_batch(&mut self, programs: &[Vec<Instruction>]) -> Vec<ProgramOutcome> {
        programs
            .iter()
            .enumerate()
            .map(|(i, instructions)| self.run_program(TaskId(i as u64), instructions))
            .collect()
    }
}

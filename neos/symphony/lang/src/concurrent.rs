//! Real multithreaded execution — several `symphony-lang` programs on real
//! OS threads, contending over the same shared, real
//! [`symphony_kernel::ConcurrentPool`]/[`symphony_kernel::ConcurrentTracker`].
//!
//! [`vm::Vm`](crate::vm::Vm) is sequential by design and says so plainly:
//! `run_batch` runs one program to completion at a time, and a blocked
//! `acquire` traps rather than suspending, "a stated real limit... there is
//! no scheduler able to suspend a blocked program and resume it once the
//! holder releases." This module is that scheduler, built from real OS
//! thread primitives rather than simulated inside one call stack.
//!
//! # Why a second dispatch loop, not a generalised `Vm`
//!
//! `Vm<'a>` borrows its pool/tracker/graph exclusively for the run's
//! lifetime — the right shape for one thread owning them outright, and
//! exactly the shape that cannot be shared across real threads at the same
//! time. Sharing across threads needs owned, reference-counted, internally
//! synchronised handles (`Arc<ConcurrentPool>`, `Arc<ConcurrentTracker>`)
//! instead. Rather than force one type to serve both shapes through an
//! abstraction layer, [`run_program`] is a second, small dispatch loop
//! against the `Arc`'d forms — the same trade `ConcurrentPool` already made
//! against plain `MemoryPool` rather than unifying the two behind a trait.
//! `Vm` itself is untouched by this module; every one of its existing tests
//! is unaffected.
//!
//! # What's actually different in the loop
//!
//! Only two instructions change behaviour. `store`/`load` are identical —
//! `ConcurrentPool` already serialises them safely, real curved addressing
//! either way. `acquire` no longer has a `Blocked` outcome to trap on: it
//! calls [`symphony_kernel::ConcurrentTracker::blocking_acquire`], which
//! blocks the **calling OS thread** until granted. `run_program_trapped`'s
//! dynamic fault routing (a handler able to retry a memory fault) is not
//! reproduced here — this module is scoped to the concurrency question, not
//! a second copy of Phase 3's handler mechanism.
//!
//! # A real deadlock can now really happen — and this module does not
//! resolve it
//!
//! Two threads each holding what the other wants now genuinely block
//! forever on their own OS threads. Detecting and resolving that is left to
//! the caller, on purpose — the same detection/resolution boundary this
//! whole workspace already keeps everywhere else (`symphony_kernel::deadlock`
//! detects; `neos/src/main.rs` resolves, at application level, with a
//! policy stated as a choice). `ConcurrentTracker::detect_cycle`/
//! `force_release_all` are the pieces a caller's own watchdog needs; this
//! module does not run one itself.

use std::collections::{BTreeMap, HashSet};
use std::sync::Arc;

use lattice::tessellation::CellId;
use symphony_kernel::bifurcation::TaskModel;
use symphony_kernel::{evaluate_branch, fork, resonates, Interference, Phase, ResourceId, TaskId};
use symphony_kernel::{ConcurrentPool, ConcurrentTracker};

use crate::interpreter::{BranchEvent, ForkEvent, InversionEvent, RuntimeTask};
use crate::parser::{Address, Alignment};
use crate::vm::{
    decode_state, encode_state, Domain, Instruction, ProgramOutcome, ResourceEvent,
    ResourceOutcome, VmFault, STATE_BYTES,
};
use crate::LangError;

fn resolve(
    pool: &ConcurrentPool,
    address: &Address,
) -> Result<substrate::LatticeAddress, VmFault> {
    match address {
        Address::Cell(n) => pool.address_at(*n).ok_or(VmFault::CellOutOfRange {
            cell: *n,
            cells: pool.cell_count(),
        }),
        Address::Path { start, steps } => {
            let path = lattice::AddressPath::new(*start, steps);
            pool.resolve_path(&path).map_err(VmFault::Memory)
        }
    }
}

/// Run one compiled program on the **calling thread** against shared,
/// `Arc`'d state — see the module docs for exactly what differs from
/// [`crate::vm::Vm::run_program`]. Intended to be called from inside a
/// freshly spawned OS thread (see [`run_batch_concurrent`]), but is plain,
/// blocking, synchronous code itself — nothing here is async.
pub fn run_program(
    pool: &Arc<ConcurrentPool>,
    tracker: &Arc<ConcurrentTracker>,
    reserved: &Arc<HashSet<CellId>>,
    task_id: TaskId,
    domain: Domain,
    instructions: &[Instruction],
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
                    break Some(VmFault::Lang(LangError::DuplicateTask { name: name.clone() }));
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
                None => break Some(VmFault::Lang(LangError::UndeclaredTask { name: name.clone() })),
            },

            Instruction::Emit { name } => match declared.get(name) {
                Some(task) => {
                    emitted.push(
                        symphony_kernel::Task::new(next_id, task.frequency()).with_scale(task.scale()),
                    );
                    next_id += 1;
                    pc += 1;
                }
                None => break Some(VmFault::Lang(LangError::UndeclaredTask { name: name.clone() })),
            },

            Instruction::Fork { name } => {
                let task = match declared.get(name) {
                    Some(t) => t.clone(),
                    None => {
                        break Some(VmFault::Lang(LangError::UndeclaredTask { name: name.clone() }))
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
                                symphony_kernel::Task::new(next_id, task.frequency())
                                    .with_scale(task.scale()),
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
                        break Some(VmFault::Lang(LangError::UndeclaredTask { name: left.clone() }))
                    }
                };
                let b = match declared.get(right) {
                    Some(t) => t.clone(),
                    None => {
                        break Some(VmFault::Lang(LangError::UndeclaredTask { name: right.clone() }))
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
                        break Some(VmFault::Lang(LangError::UndeclaredTask { name: name.clone() }))
                    }
                };
                let addr = match resolve(pool, address) {
                    Ok(a) => a,
                    Err(fault) => break Some(fault),
                };
                if domain == Domain::Guest && reserved.contains(&addr.cell()) {
                    break Some(VmFault::PrivilegeViolation {
                        address: address.clone(),
                        cell: addr.cell(),
                    });
                }
                match pool.write(addr, &encode_state(&task)) {
                    Ok(()) => {
                        stores.push((name.clone(), address.clone()));
                        pc += 1;
                    }
                    Err(e) => break Some(VmFault::Memory(e)),
                }
            }

            Instruction::Load { name, address } => {
                if declared.contains_key(name) {
                    break Some(VmFault::Lang(LangError::DuplicateTask { name: name.clone() }));
                }
                let addr = match resolve(pool, address) {
                    Ok(a) => a,
                    Err(fault) => break Some(fault),
                };
                if domain == Domain::Guest && reserved.contains(&addr.cell()) {
                    break Some(VmFault::PrivilegeViolation {
                        address: address.clone(),
                        cell: addr.cell(),
                    });
                }
                match pool.read(addr, STATE_BYTES) {
                    Ok(bytes) => {
                        let Some((frequency, phase_radians, scale)) = decode_state(&bytes) else {
                            break Some(VmFault::CorruptState {
                                address: address.clone(),
                            });
                        };
                        let Ok(phase) = Phase::from_radians(phase_radians, 0.0) else {
                            break Some(VmFault::CorruptState {
                                address: address.clone(),
                            });
                        };
                        match RuntimeTask::at_scale(name.clone(), frequency, phase, scale) {
                            Ok(task) => {
                                declared.insert(name.clone(), task);
                                loads.push((name.clone(), address.clone()));
                                pc += 1;
                            }
                            Err(_) => {
                                break Some(VmFault::CorruptState {
                                    address: address.clone(),
                                })
                            }
                        }
                    }
                    Err(e) => break Some(VmFault::Memory(e)),
                }
            }

            // The one instruction whose meaning genuinely changes: a blocked
            // acquire suspends this real OS thread instead of trapping the
            // program. See the module docs.
            Instruction::Acquire { resource } => {
                let rid = ResourceId(*resource);
                match tracker.blocking_acquire(task_id, rid) {
                    Ok(()) => {
                        resources.push(ResourceEvent {
                            resource: rid,
                            outcome: ResourceOutcome::Granted,
                        });
                        pc += 1;
                    }
                    Err(e) => break Some(VmFault::Resource(e)),
                }
            }

            Instruction::Release { resource } => {
                let rid = ResourceId(*resource);
                match tracker.release(task_id, rid) {
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

/// Run several compiled programs concurrently, one real OS thread each,
/// against the same shared pool/tracker/reserved-cell set. Blocks the
/// calling thread until every program has run to completion or trapped —
/// which, per the module docs, **is not guaranteed to happen** if the
/// programs given can deadlock and nothing external resolves it. Callers
/// that want that guarantee run their own watchdog against the same
/// `tracker` concurrently with this call.
pub fn run_batch_concurrent(
    pool: &Arc<ConcurrentPool>,
    tracker: &Arc<ConcurrentTracker>,
    reserved: &Arc<HashSet<CellId>>,
    programs: Vec<(TaskId, Domain, Vec<Instruction>)>,
) -> Vec<ProgramOutcome> {
    let handles: Vec<_> = programs
        .into_iter()
        .map(|(task_id, domain, instructions)| {
            let pool = Arc::clone(pool);
            let tracker = Arc::clone(tracker);
            let reserved = Arc::clone(reserved);
            std::thread::spawn(move || {
                run_program(&pool, &tracker, &reserved, task_id, domain, &instructions)
            })
        })
        .collect();

    handles
        .into_iter()
        .map(|h| h.join().expect("a symphony-lang program thread must not panic"))
        .collect()
}

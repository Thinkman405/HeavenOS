//! Scheduler policy, deadlock detection, and axiom handlers.
//!
//! Doctrine: `_mkb/test-doctrine.md`.
//!
//! **[D]** marks assertions a conventional implementation could not pass — a
//! priority-queue scheduler, a Planck quantizer, or boolean branch logic.

use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};
use symphony_kernel::bifurcation::{superpose, Phase};
use symphony_kernel::{
    evaluate_branch, fork, fork_unit, Acquired, ConcurrentPool, ConcurrentTracker, CoreTopology,
    Interference, KernelError, ResourceError, ResourceId, ResourceTracker, Scheduler, Task,
    TaskId, WaitForGraph,
};

fn sched(cores: usize, tasks: usize, hz: f64) -> Scheduler {
    let mut s = Scheduler::new(cores);
    s.ingest((0..tasks as u64).map(|i| Task::new(i, hz)));
    s
}

// ------------------------------------------------- Group 1: arrival ingestion

/// Ingested tasks are all placed, none lost.
#[test]
fn ingestion_places_every_task() {
    let s = sched(16, 100, 2.0e9);
    assert_eq!(s.task_count(), 100);
    let total: usize = (0..s.core_count()).map(|c| s.tasks_on(c).len()).sum();
    assert_eq!(total, 100);
}

/// Load is the summed energy of resident tasks, not a task count.
#[test]
fn load_is_summed_energy_not_task_count() {
    let mut s = Scheduler::new(7);
    s.ingest([Task::new(0, 1.0e9), Task::new(1, 5.0e9)]);
    let total: f64 = s.load_per_core().iter().sum();
    let expected: f64 = s.all_tasks().iter().map(Task::energy_joules).sum();
    assert!((total - expected).abs() < 1e-40);
    // Two tasks of unequal frequency must not contribute equally.
    assert!(s.all_tasks()[0].energy_joules() != s.all_tasks()[1].energy_joules());
}

/// Task density is mean-centred — the solvability condition, end to end.
#[test]
fn ingested_density_sums_to_zero() {
    for cores in [7, 16, 31] {
        let s = sched(cores, 40, 3.0e9);
        let sum: f64 = s.task_density().iter().sum();
        assert!(sum.abs() < 1e-30, "sum(rho) = {sum} for {cores} cores");
    }
}

/// [D] Priority is an energy state: a higher-frequency task costs strictly more.
#[test]
fn higher_frequency_costs_more_energy() {
    let slow = Task::new(0, 1.0e9);
    let fast = Task::new(1, 4.0e9);
    assert!(fast.energy_joules() > slow.energy_joules());
    let ratio = fast.energy_joules() / slow.energy_joules();
    assert!((ratio - 4.0).abs() < 1e-9, "energy is linear in nu, got {ratio}");
}

/// The resonance correction is applied at the task's scale, and `xi(R) = 1`
/// leaves a reference-scale task untouched.
#[test]
fn resonance_correction_applies_at_task_scale() {
    let plain = Task::new(0, 2.0e9); // scale 1.0 == reference
    let base = symphony_kernel::energy(plain.frequency).0;
    assert!((plain.energy_joules() - base).abs() < 1e-40);

    let small = Task::new(1, 2.0e9).with_scale(0.1);
    assert!(small.energy_joules() > base, "xi > 1 below reference scale");
}

// ------------------------------------------------- Group 2: scheduling passes

/// [D] A pass reduces load spread — the field actually balances.
#[test]
fn scheduling_reduces_load_spread() {
    let mut s = Scheduler::new(16);
    s.ingest((0..60u64).map(|i| Task::new(i, 2.0e9)));
    let alpha = s.topology().stability_bound() * 0.9;

    let before = s.load_field().spread();
    let pass = s.schedule(alpha, 20_000).expect("alpha within bound");
    assert!(
        pass.spread_after <= before,
        "spread {before} -> {} should not grow",
        pass.spread_after
    );
    assert_eq!(s.task_count(), 60, "no task may be lost while balancing");
}

/// Tasks are conserved across repeated passes.
#[test]
fn tasks_are_conserved_across_passes() {
    let mut s = sched(31, 120, 2.5e9);
    let alpha = s.topology().stability_bound() * 0.9;
    for _ in 0..5 {
        s.schedule(alpha, 5_000).unwrap();
        assert_eq!(s.task_count(), 120);
    }
    assert!(s.reclaimed().is_empty(), "no live task should be reclaimed");
}

/// [D] Migration is edge-local: a task only ever moves to an adjacent core.
///
/// A scheduler that could relocate a task anywhere is a queue, not a field.
#[test]
fn migration_only_crosses_topology_edges() {
    let mut s = sched(31, 90, 2.0e9);
    let topo = s.topology().clone();
    let alpha = topo.stability_bound() * 0.9;

    let where_is = |s: &Scheduler| -> Vec<(TaskId, usize)> {
        let mut v = Vec::new();
        for c in 0..s.core_count() {
            for t in s.tasks_on(c) {
                v.push((t.id, c));
            }
        }
        v.sort();
        v
    };

    let before = where_is(&s);
    s.schedule(alpha, 5_000).unwrap();
    let after = where_is(&s);

    for ((id_a, core_a), (id_b, core_b)) in before.iter().zip(after.iter()) {
        assert_eq!(id_a, id_b, "task ordering changed unexpectedly");
        if core_a != core_b {
            assert!(
                topo.neighbors(*core_a).contains(core_b),
                "task {id_a:?} jumped from core {core_a} to non-adjacent {core_b}"
            );
        }
    }
}

/// An out-of-bound coupling is refused at the scheduler boundary too.
#[test]
fn scheduler_rejects_unstable_coupling() {
    let mut s = sched(16, 20, 2.0e9);
    let bad = s.topology().stability_bound() * 1.5;
    assert!(matches!(
        s.schedule(bad, 100),
        Err(KernelError::Unstable { .. })
    ));
}

/// Reclamation falls out of `E = C_H*nu`: a zero-frequency task holds no energy.
#[test]
fn zero_frequency_tasks_are_reclaimed() {
    let mut s = Scheduler::new(7);
    s.ingest([Task::new(0, 2.0e9), Task::new(1, 0.0), Task::new(2, 1.0e9)]);
    assert_eq!(s.task_count(), 3);
    let alpha = s.topology().stability_bound() * 0.9;
    let pass = s.schedule(alpha, 1_000).unwrap();
    assert_eq!(pass.reclaimed, 1);
    assert_eq!(s.task_count(), 2);
    assert_eq!(s.reclaimed(), &[TaskId(1)]);
}

// --------------------------------------------------- Group 3: deadlock

/// No cycle is a genuine result, not "none found".
#[test]
fn acyclic_wait_graph_has_no_deadlock() {
    let mut g = WaitForGraph::new();
    g.add_wait(TaskId(1), TaskId(2));
    g.add_wait(TaskId(2), TaskId(3));
    g.add_wait(TaskId(4), TaskId(3));
    assert_eq!(g.detect_cycle(), None);
    assert!(!g.has_deadlock());
}

/// The classic two-lock inversion.
#[test]
fn two_task_cycle_is_detected() {
    let mut g = WaitForGraph::new();
    g.add_wait(TaskId(1), TaskId(2));
    g.add_wait(TaskId(2), TaskId(1));
    let cycle = g.detect_cycle().expect("a 2-cycle is a deadlock");
    assert_eq!(cycle.len(), 2);
    assert!(cycle.contains(&TaskId(1)) && cycle.contains(&TaskId(2)));
}

/// Longer cycles, and a cycle reachable only through acyclic prefix edges.
#[test]
fn longer_and_buried_cycles_are_detected() {
    let mut g = WaitForGraph::new();
    for i in 1..=5u64 {
        g.add_wait(TaskId(i), TaskId(i % 5 + 1));
    }
    assert_eq!(g.detect_cycle().map(|c| c.len()), Some(5));

    let mut g = WaitForGraph::new();
    g.add_wait(TaskId(10), TaskId(11)); // acyclic prefix
    g.add_wait(TaskId(11), TaskId(1));
    g.add_wait(TaskId(1), TaskId(2));
    g.add_wait(TaskId(2), TaskId(3));
    g.add_wait(TaskId(3), TaskId(1));
    let c = g.detect_cycle().expect("buried cycle must be found");
    assert_eq!(c.len(), 3);
    assert!(!c.contains(&TaskId(10)), "prefix must not be in the cycle");
}

/// A task waiting on itself.
#[test]
fn self_wait_is_a_deadlock() {
    let mut g = WaitForGraph::new();
    g.add_wait(TaskId(7), TaskId(7));
    assert_eq!(g.detect_cycle(), Some(vec![TaskId(7)]));
}

/// A diamond has two paths but no cycle.
#[test]
fn diamond_dependency_is_not_a_deadlock() {
    let mut g = WaitForGraph::new();
    g.add_wait(TaskId(1), TaskId(2));
    g.add_wait(TaskId(1), TaskId(3));
    g.add_wait(TaskId(2), TaskId(4));
    g.add_wait(TaskId(3), TaskId(4));
    assert_eq!(g.detect_cycle(), None);
}

/// **[D] The scope boundary, made executable.**
///
/// Load equilibrium eliminates thrashing and bottlenecks. It does **not**
/// eliminate deadlock. Here the field is perfectly balanced and the system is
/// deadlocked anyway — which is exactly why contract §8 requires this module.
#[test]
fn perfectly_balanced_load_can_still_deadlock() {
    let mut s = Scheduler::new(16);
    s.ingest((0..64u64).map(|i| Task::new(i, 2.0e9)));
    let alpha = s.topology().stability_bound() * 0.9;
    s.schedule(alpha, 20_000).unwrap();

    let spread = s.load_field().spread();
    let total: f64 = s.load_per_core().iter().sum();
    assert!(
        spread / total < 0.35,
        "field should be broadly balanced, spread/total = {}",
        spread / total
    );

    // ... and yet:
    let mut g = WaitForGraph::new();
    g.add_wait(TaskId(0), TaskId(1));
    g.add_wait(TaskId(1), TaskId(0));
    assert!(
        g.has_deadlock(),
        "balanced load must not be mistaken for absence of deadlock"
    );
}

/// Releasing a wait breaks the cycle.
#[test]
fn clearing_waits_resolves_the_deadlock() {
    let mut g = WaitForGraph::new();
    g.add_wait(TaskId(1), TaskId(2));
    g.add_wait(TaskId(2), TaskId(1));
    assert!(g.has_deadlock());
    g.clear_waits(TaskId(2));
    assert!(!g.has_deadlock());
}

/// `remove_wait` deletes exactly the named edge, leaving any other edge the
/// same waiter holds untouched — the property `clear_waits` cannot give.
/// `ResourceTracker` never currently builds a waiter with two live edges (it
/// enforces at most one), so this exercises `WaitForGraph` directly rather
/// than through the tracker, closing what the doctrine check found was
/// otherwise a blind spot.
#[test]
fn remove_wait_drops_only_the_named_edge() {
    let mut g = WaitForGraph::new();
    g.add_wait(TaskId(1), TaskId(2));
    g.add_wait(TaskId(1), TaskId(3));
    g.add_wait(TaskId(3), TaskId(1));
    assert!(g.has_deadlock(), "1 -> 3 -> 1 is a cycle");

    g.remove_wait(TaskId(1), TaskId(2)); // an edge unrelated to the cycle
    assert!(
        g.has_deadlock(),
        "the real cycle must survive removing an unrelated edge"
    );

    g.remove_wait(TaskId(1), TaskId(3)); // the cycle's own edge
    assert!(
        !g.has_deadlock(),
        "removing the cycle's own edge must resolve it"
    );
}

// ------------------------------------- Group 3b: resource tracker feeds the graph
//
// `WaitForGraph` only ever detects cycles in edges someone else recorded.
// These tests build the edges by calling `ResourceTracker::acquire`/`release`
// directly — nobody hand-computes "who holds this."

/// A free resource is granted with no wait edge recorded at all.
#[test]
fn acquiring_a_free_resource_grants_immediately() {
    let mut t = ResourceTracker::new();
    let mut g = WaitForGraph::new();
    let r = ResourceId(1);
    assert_eq!(t.acquire(TaskId(1), r, &mut g), Ok(Acquired::Granted));
    assert_eq!(t.holder_of(r), Some(TaskId(1)));
    assert!(g.is_empty());
}

/// A held resource blocks the second acquirer and the block shows up as a
/// real edge in the graph, not just as the tracker's own return value.
#[test]
fn acquiring_a_held_resource_blocks_and_records_the_edge() {
    let mut t = ResourceTracker::new();
    let mut g = WaitForGraph::new();
    let r = ResourceId(1);
    t.acquire(TaskId(1), r, &mut g).unwrap();
    let outcome = t.acquire(TaskId(2), r, &mut g).unwrap();
    assert_eq!(outcome, Acquired::Blocked { holder: TaskId(1) });
    assert_eq!(t.is_waiting(TaskId(2)), Some(r));
    assert!(!g.is_empty());
    assert!(!g.has_deadlock(), "one waiter alone is not a cycle");
}

/// The holder re-acquiring its own resource is a no-op, not a self-wait.
#[test]
fn reentrant_acquire_by_the_holder_is_a_noop() {
    let mut t = ResourceTracker::new();
    let mut g = WaitForGraph::new();
    let r = ResourceId(1);
    t.acquire(TaskId(1), r, &mut g).unwrap();
    assert_eq!(t.acquire(TaskId(1), r, &mut g), Ok(Acquired::Granted));
    assert_eq!(t.is_waiting(TaskId(1)), None, "the holder is not a waiter");
    assert!(g.is_empty());
}

/// Repeating an acquire call while already blocked on that same resource is
/// idempotent, not a second edge or an error.
#[test]
fn repeated_acquire_of_the_same_pending_resource_is_idempotent() {
    let mut t = ResourceTracker::new();
    let mut g = WaitForGraph::new();
    let r = ResourceId(1);
    t.acquire(TaskId(1), r, &mut g).unwrap();
    let first = t.acquire(TaskId(2), r, &mut g).unwrap();
    let second = t.acquire(TaskId(2), r, &mut g).unwrap();
    assert_eq!(first, second);
    assert_eq!(t.is_waiting(TaskId(2)), Some(r));
}

/// Acquiring a second, different resource while already blocked is refused —
/// the tracker keeps at most one outstanding wait edge per task.
#[test]
fn acquiring_a_different_resource_while_blocked_is_refused() {
    let mut t = ResourceTracker::new();
    let mut g = WaitForGraph::new();
    let (r, s) = (ResourceId(1), ResourceId(2));
    t.acquire(TaskId(1), r, &mut g).unwrap(); // task 1 holds r
    t.acquire(TaskId(3), s, &mut g).unwrap(); // task 3 holds s
    t.acquire(TaskId(2), r, &mut g).unwrap(); // task 2 blocks on r

    let err = t.acquire(TaskId(2), s, &mut g).unwrap_err();
    assert_eq!(
        err,
        ResourceError::AlreadyWaiting {
            task: TaskId(2),
            already_waiting_on: r,
        }
    );
}

/// Releasing a resource you do not hold is refused, not silently accepted.
#[test]
fn releasing_a_resource_you_do_not_hold_is_refused() {
    let mut t = ResourceTracker::new();
    let mut g = WaitForGraph::new();
    let r = ResourceId(1);
    t.acquire(TaskId(1), r, &mut g).unwrap();
    let err = t.release(TaskId(2), r, &mut g).unwrap_err();
    assert_eq!(
        err,
        ResourceError::NotHolder {
            task: TaskId(2),
            resource: r,
        }
    );
}

/// **[D] Two tasks acquiring two resources in opposite orders deadlock** —
/// built entirely through `ResourceTracker::acquire`, the exact scope-boundary
/// scenario contract §8 names (two locks, opposite order), but composed by
/// the feeder instead of hand-built edges.
#[test]
fn two_tasks_contending_in_opposite_orders_deadlocks() {
    let mut t = ResourceTracker::new();
    let mut g = WaitForGraph::new();
    let (fork_left, fork_right) = (ResourceId(1), ResourceId(2));

    t.acquire(TaskId(1), fork_left, &mut g).unwrap(); // task 1 takes the left fork
    t.acquire(TaskId(2), fork_right, &mut g).unwrap(); // task 2 takes the right fork

    let a = t.acquire(TaskId(1), fork_right, &mut g).unwrap(); // 1 wants 2's fork
    let b = t.acquire(TaskId(2), fork_left, &mut g).unwrap(); // 2 wants 1's fork
    assert_eq!(a, Acquired::Blocked { holder: TaskId(2) });
    assert_eq!(b, Acquired::Blocked { holder: TaskId(1) });

    let cycle = g.detect_cycle().expect("classic two-lock inversion");
    assert_eq!(cycle.len(), 2);
}

/// Releasing hands the resource straight to the queued waiter and clears
/// exactly that waiter's edge — the graph reflects the handoff without the
/// caller doing anything beyond calling `release`.
#[test]
fn releasing_grants_the_next_queued_waiter_and_clears_its_edge() {
    let mut t = ResourceTracker::new();
    let mut g = WaitForGraph::new();
    let r = ResourceId(1);
    t.acquire(TaskId(1), r, &mut g).unwrap();
    t.acquire(TaskId(2), r, &mut g).unwrap(); // blocks

    let granted = t.release(TaskId(1), r, &mut g).unwrap();
    assert_eq!(granted, Some(TaskId(2)));
    assert_eq!(t.holder_of(r), Some(TaskId(2)));
    assert_eq!(t.is_waiting(TaskId(2)), None);
    assert!(g.is_empty(), "the granted waiter's edge must be gone, not stale");
}

/// A resource with nobody queued releases cleanly with no handoff.
#[test]
fn releasing_an_uncontended_resource_hands_off_to_nobody() {
    let mut t = ResourceTracker::new();
    let mut g = WaitForGraph::new();
    let r = ResourceId(1);
    t.acquire(TaskId(1), r, &mut g).unwrap();
    assert_eq!(t.release(TaskId(1), r, &mut g).unwrap(), None);
    assert_eq!(t.holder_of(r), None);
}

/// **The retargeting case.** A third task queued behind the one just granted
/// must end up waiting on the *new* holder, not the departed one — otherwise
/// a real cycle through the new holder would go undetected because the edge
/// still points at a task holding nothing.
#[test]
fn releasing_retargets_remaining_waiters_to_the_new_holder() {
    let mut t = ResourceTracker::new();
    let mut g = WaitForGraph::new();
    let (r, s) = (ResourceId(1), ResourceId(2));

    t.acquire(TaskId(1), r, &mut g).unwrap(); // 1 holds r
    t.acquire(TaskId(3), s, &mut g).unwrap(); // 3 holds s
    t.acquire(TaskId(2), r, &mut g).unwrap(); // 2 queues on r, behind nobody
    t.acquire(TaskId(3), r, &mut g).unwrap(); // 3 also queues on r, behind 2

    // 1 releases: 2 is granted r; 3's edge must move from "waits on 1" to
    // "waits on 2".
    assert_eq!(t.release(TaskId(1), r, &mut g).unwrap(), Some(TaskId(2)));
    assert_eq!(t.holder_of(r), Some(TaskId(2)));
    assert_eq!(t.is_waiting(TaskId(3)), Some(r));

    // Now close the loop: 2 (holding r, granted above) wants s, which 3
    // holds. If 3's edge were still stale on task 1 instead of retargeted
    // to 2, this could never cycle, since 1 holds nothing to wait back on.
    let outcome = t.acquire(TaskId(2), s, &mut g).unwrap();
    assert_eq!(outcome, Acquired::Blocked { holder: TaskId(3) });
    let cycle = g.detect_cycle().expect("2 <-> 3 must cycle through the retargeted edge");
    assert_eq!(cycle.len(), 2);
    assert!(cycle.contains(&TaskId(2)) && cycle.contains(&TaskId(3)));
}

// --------------------------------------------------- Group 4: A1 bifurcation

/// [D] A1: forking a unit yields exactly 2 — bit-exact, via `lattice`'s operator.
#[test]
fn unit_fork_yields_exactly_two() {
    let b = fork_unit();
    assert_eq!(b.children, 2.0, "axiom A1: 1 (x) 1 must be exactly 2");
    assert_eq!(b.address_scale, 2.0, "address space splits with the process");
}

/// [D] Forking is not scalar duplication.
#[test]
fn fork_is_not_scalar_duplication() {
    let b = fork_unit();
    assert_ne!(b.children, 1.0, "scalar duplication would give 1");
    assert_eq!(b.children, fork(1.0).unwrap().children);
}

/// Fork respects the operator's domain rather than returning infinity.
#[test]
fn fork_rejects_units_outside_the_operator_domain() {
    assert!(matches!(
        fork(1.0e9),
        Err(KernelError::UndefinedScale { .. })
    ));
}

// --------------------------------------------------- Group 5: A2 phase logic

/// [D] A2: phases are the two permitted orientations, from the MKB.
#[test]
fn phases_are_the_permitted_orientations() {
    use std::f64::consts::FRAC_PI_2;
    assert!((Phase::Positive.radians() - FRAC_PI_2).abs() < 1e-15);
    assert!((Phase::Negative.radians() + FRAC_PI_2).abs() < 1e-15);
}

/// A phase that is neither orientation is not a logic state and is rejected —
/// never silently rounded into one.
#[test]
fn off_axis_phase_is_rejected() {
    assert!(Phase::from_radians(0.0, 1e-9).is_err());
    assert!(Phase::from_radians(std::f64::consts::PI, 1e-9).is_err());
    assert!(Phase::from_radians(std::f64::consts::FRAC_PI_2, 1e-9).is_ok());
}

/// [D] Branch evaluation is phase alignment, not boolean comparison.
#[test]
fn branch_evaluation_is_interference() {
    assert_eq!(
        evaluate_branch(Phase::Positive, Phase::Positive),
        Interference::Constructive
    );
    assert_eq!(
        evaluate_branch(Phase::Positive, Phase::Negative),
        Interference::Destructive
    );
}

/// [D] Destructive cancellation is total — exactly zero, no tolerance.
///
/// This is what lets phase teardown work without an acknowledgement message.
#[test]
fn opposed_phases_cancel_exactly() {
    assert_eq!(superpose(Phase::Positive, Phase::Negative), 0.0);
    assert_eq!(superpose(Phase::Negative, Phase::Positive), 0.0);
    assert!(superpose(Phase::Positive, Phase::Positive).abs() > 1.9);
}

// --------------------------------------------------- Group 6: integration

/// A full cycle: arrivals, balance, deadlock check, fork.
#[test]
fn end_to_end_scheduling_cycle() {
    let mut s = Scheduler::new(31);
    s.ingest((0..120u64).map(|i| Task::new(i, 1.0e9 + (i as f64) * 1.0e7)));
    let alpha = s.topology().stability_bound() * 0.9;

    let pass = s.schedule(alpha, 20_000).expect("stable coupling");
    assert_eq!(s.task_count(), 120);
    assert!(pass.relaxation_steps > 0, "the field must actually relax");

    let g = WaitForGraph::new();
    assert!(!g.has_deadlock(), "a fresh system holds no waits");

    assert_eq!(fork_unit().children, 2.0);
}

/// Topology invariants survive scheduler construction.
#[test]
fn scheduler_topology_matches_lattice_guarantees() {
    let s = Scheduler::new(64);
    let t: &CoreTopology = s.topology();
    assert_eq!(t.len(), 64);
    assert!(t.is_connected());
    assert_eq!(t.max_degree(), 5);
    assert!(t.min_degree() < t.max_degree());
}

// ------------------------------------ Group 7: concurrent memory access
//
// `substrate::MemoryPool` is single-threaded by its own design; verified
// separately (a disposable scratch harness, not part of this suite) that an
// unsynchronised pool really does corrupt under real concurrent threads —
// 10/10 runs — while the identical workload through `ConcurrentPool`
// doesn't, 0/10. These tests exercise `ConcurrentPool` itself, with real
// `std::thread` threads, not a simulation of concurrency.

/// Many real threads sharing one pool, each writing and reading back its own
/// distinct fingerprint byte. If two threads' allocations ever aliased the
/// same bytes, at least one would read back a fingerprint that wasn't its
/// own — a plain `assert_eq!` inside the thread catches it directly, and a
/// failed assertion inside a spawned thread surfaces as a panic that
/// `join()` propagates.
#[test]
fn concurrent_allocations_never_corrupt_or_alias() {
    let pool = ConcurrentPool::new(2, 256);
    let threads = 32;
    let iters = 200;

    let handles: Vec<_> = (0..threads)
        .map(|tid| {
            let pool = Arc::clone(&pool);
            let fingerprint = (tid as u8).wrapping_add(1); // distinct per thread, never 0
            thread::spawn(move || {
                for _ in 0..iters {
                    if let Ok(alloc) = pool.allocate(64) {
                        pool.write(alloc.start(), &[fingerprint; 64]).unwrap();
                        thread::yield_now(); // widen the window for a race to land
                        let back = pool.read(alloc.start(), 64).unwrap();
                        pool.free(&alloc);
                        assert_eq!(
                            back,
                            vec![fingerprint; 64],
                            "thread {tid}'s allocation was corrupted by another thread"
                        );
                    }
                }
            })
        })
        .collect();

    for h in handles {
        h.join().expect("no thread should panic or fail its assertion");
    }

    assert_eq!(
        pool.available(),
        pool.total_capacity(),
        "every allocation was freed; nothing should remain marked used"
    );
}

// ---------------------------------------- Group: ConcurrentTracker — real
// ---------------------------------------- blocking, real deadlocks
//
// `ResourceTracker::acquire` answering `Blocked` is a fact recorded in a
// data structure — nothing about it makes a *thread* wait. `ConcurrentPool`
// only had to prove a `Mutex` prevents corruption of total operations;
// `ConcurrentTracker` has to prove something behavioural: that a blocked
// caller's own OS thread actually sleeps and actually wakes on release, not
// just that concurrent access is race-free.

/// The direct claim a `Condvar`-based wait exists to make: a second thread's
/// `blocking_acquire` for a resource thread one already holds returns only
/// *after* thread one releases — timed, not just check that it eventually
/// returns. A `Blocked`-then-immediately-return implementation (the
/// sequential `ResourceTracker` behaviour this wraps) would return in
/// microseconds regardless of when the release happens; a real wait tracks
/// it.
#[test]
fn blocking_acquire_really_suspends_the_calling_thread_until_released() {
    let tracker = ConcurrentTracker::new();
    let resource = ResourceId(1);
    let hold = Duration::from_millis(150);

    tracker.blocking_acquire(TaskId(1), resource).unwrap();

    let holder_tracker = Arc::clone(&tracker);
    let holder = thread::spawn(move || {
        thread::sleep(hold);
        holder_tracker.release(TaskId(1), resource).unwrap();
    });

    let start = Instant::now();
    tracker.blocking_acquire(TaskId(2), resource).unwrap();
    let elapsed = start.elapsed();

    holder.join().unwrap();
    assert!(
        elapsed >= hold - Duration::from_millis(20),
        "task 2 returned after {elapsed:?}, before task 1's {hold:?} hold — it did not really wait"
    );
}

/// **The real version of `two_tasks_contending_in_opposite_orders_deadlocks`
/// above** — the same classic two-lock inversion, but the two tasks are now
/// real OS threads that really block, not two sequential calls on one
/// thread recording what *would* happen. A third thread acts as the
/// watchdog this workspace's own detection/resolution boundary always
/// assigns to "application level": it polls `detect_cycle` (bounded, so a
/// bug here fails the test instead of hanging the suite) and resolves with
/// this workspace's existing stated policy — lowest `TaskId` in the cycle
/// is the victim, `force_release_all` releases everything it holds.
///
/// Each philosopher's program acquires *both* forks and then releases both
/// — not just acquires. This matters for what "resolved" can honestly mean:
/// `neos/src/main.rs`'s own resolution notes plainly that force-releasing
/// the victim's held resource does not cancel the victim's own still-
/// pending request — "whichever task didn't get force-released may simply
/// still be waiting, and correctly so." Stripping the victim's one held
/// fork lets the *other* philosopher finish its meal and put both forks
/// down, which is what actually frees the victim's own pending request —
/// not the force-release directly. A program that never gives its forks
/// back would leave the victim blocked forever no matter what the watchdog
/// does, which is a fact about the scenario, not a bug to route around.
///
/// Both philosophers' own release calls tolerate `NotHolder` rather than
/// unwrapping it — a real consequence of real preemption that a hand-built
/// sequential scenario never has to face: the victim's own thread keeps
/// running past the point its fork was taken, and will genuinely try to put
/// back a fork it no longer holds. A task built to survive preemption
/// checks for exactly this; one written with `.unwrap()` on every release
/// is a task that assumes it can never be preempted, which stops being true
/// the moment a watchdog exists at all.
#[test]
fn two_real_threads_deadlock_and_the_watchdog_resolves_it() {
    let tracker = ConcurrentTracker::new();
    let (fork_left, fork_right) = (ResourceId(1), ResourceId(2));
    let (chef_a, chef_b) = (TaskId(1), TaskId(2));

    // Each thread takes its own fork first, then reaches for the other's —
    // the exact opposite-order scenario the sequential test builds by hand —
    // eats, then puts both forks back down.
    let t1 = {
        let tracker = Arc::clone(&tracker);
        thread::spawn(move || {
            tracker.blocking_acquire(chef_a, fork_left).unwrap();
            thread::sleep(Duration::from_millis(20)); // widen the window
            tracker.blocking_acquire(chef_a, fork_right).unwrap();
            let _ = tracker.release(chef_a, fork_right); // may already be gone under preemption
            let _ = tracker.release(chef_a, fork_left);
        })
    };
    let t2 = {
        let tracker = Arc::clone(&tracker);
        thread::spawn(move || {
            tracker.blocking_acquire(chef_b, fork_right).unwrap();
            thread::sleep(Duration::from_millis(20));
            tracker.blocking_acquire(chef_b, fork_left).unwrap();
            let _ = tracker.release(chef_b, fork_left);
            let _ = tracker.release(chef_b, fork_right);
        })
    };

    // Watchdog: both threads are now genuinely, concurrently blocked on
    // real OS thread primitives — this loop runs alongside that, not after.
    let mut cycle = None;
    for _ in 0..500 {
        if let Some(c) = tracker.detect_cycle() {
            cycle = Some(c);
            break;
        }
        thread::sleep(Duration::from_millis(10));
    }
    let cycle = cycle.expect("a real two-thread opposite-order acquire must deadlock and be detected within 5s");
    assert_eq!(cycle.len(), 2);
    assert!(cycle.contains(&chef_a) && cycle.contains(&chef_b));

    let victim = *cycle.iter().min_by_key(|t| t.0).unwrap();
    let released = tracker.force_release_all(victim);
    assert!(!released.is_empty(), "the victim must have been holding something to release");

    // Both real threads must now be able to finish: the release unblocks
    // whichever philosopher wasn't the victim, who eats and puts both forks
    // back down, which is what frees the victim's own still-pending request.
    t1.join().unwrap();
    t2.join().unwrap();
    assert!(!tracker.has_deadlock(), "resolution must actually clear the cycle");
}

/// Requests that collectively exceed capacity, launched from many threads at
/// once: exactly as many succeed as capacity allows, never more. This is
/// only true if the availability check and the reservation happen as one
/// atomic step under the lock — a wrapper that checked, released the lock,
/// then reserved would let concurrent callers all see room that only one of
/// them could actually have.
#[test]
fn concurrent_admission_never_oversubscribes_capacity() {
    let pool = ConcurrentPool::new(4, 256); // 1024 bytes total
    let chunk = 100;
    let attempts = 20; // 2000 bytes requested; room for at most 10

    let handles: Vec<_> = (0..attempts)
        .map(|_| {
            let pool = Arc::clone(&pool);
            thread::spawn(move || pool.allocate(chunk))
        })
        .collect();

    let granted = handles
        .into_iter()
        .map(|h| h.join().unwrap())
        .filter(Result::is_ok)
        .count();
    let granted_bytes = granted * chunk;

    assert!(
        granted_bytes <= pool.total_capacity(),
        "granted {granted_bytes} bytes over a {}-byte pool",
        pool.total_capacity()
    );
    assert_eq!(
        pool.total_capacity() - pool.available(),
        granted_bytes,
        "accounting must match exactly what was actually granted, not merely stay within bounds"
    );
}

//! Concurrent access to [`ResourceTracker`]/[`WaitForGraph`], with **real**
//! blocking — the direct resource-side counterpart to [`crate::memory`]'s
//! `ConcurrentPool` for the memory side.
//!
//! # Why this is a genuinely different kind of wrapper than `ConcurrentPool`
//!
//! `ConcurrentPool` only had to prove *safety*: a `Mutex` around already-total
//! operations (`allocate` always returns promptly, `Ok` or `Err`). Blocking
//! on a resource is not total the same way — `ResourceTracker::acquire` can
//! legitimately answer "not yet," and something has to decide what a caller
//! does with that. Sequentially, `symphony_lang::vm::Vm` decides "not yet"
//! means "trap," a stated real limit: *"there is no scheduler able to
//! suspend a blocked program and resume it once the holder releases."*
//! [`ConcurrentTracker::blocking_acquire`] is that missing scheduler, built
//! from real OS thread primitives (`Condvar`) rather than simulated: a
//! blocked caller's *thread* actually sleeps, and actually wakes when
//! [`ConcurrentTracker::release`] (on any thread) makes the resource
//! available.
//!
//! # A real deadlock can now really happen
//!
//! Two threads wanting each other's already-held resource now genuinely
//! block forever on their own OS threads, not on a hand-sequenced call
//! order a single-threaded demo controls. Detecting it
//! ([`ConcurrentTracker::detect_cycle`], unchanged — it reads the same
//! [`WaitForGraph`] the sequential path already used) and resolving it
//! ([`ConcurrentTracker::force_release_all`], new — wakes every thread
//! blocked on any resource the victim held) both have to work while other
//! threads are genuinely, concurrently blocked, not paused for inspection.

use std::sync::{Arc, Condvar, Mutex, MutexGuard};

use crate::deadlock::WaitForGraph;
use crate::resources::{Acquired, ResourceError, ResourceId, ResourceTracker};
use crate::scheduler::TaskId;

struct State {
    tracker: ResourceTracker,
    graph: WaitForGraph,
}

/// [`ResourceTracker`]/[`WaitForGraph`] safe to share across threads via
/// [`Arc`], with real blocking on a resource another thread holds.
pub struct ConcurrentTracker {
    state: Mutex<State>,
    /// Signalled on every successful `release` — every blocked
    /// `blocking_acquire` wakes and re-checks, the same "wake everyone,
    /// let each re-ask" shape a condition variable is for; only the
    /// thread(s) actually granted continue.
    changed: Condvar,
}

impl ConcurrentTracker {
    /// Returns an `Arc` directly, matching `ConcurrentPool::new` — a
    /// tracker nobody else can see has no concurrency to speak of.
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            state: Mutex::new(State {
                tracker: ResourceTracker::new(),
                graph: WaitForGraph::new(),
            }),
            changed: Condvar::new(),
        })
    }

    /// Recovers from a poisoned lock rather than propagating the poison —
    /// same reasoning as `ConcurrentPool::lock`: neither `ResourceTracker`
    /// nor `WaitForGraph` leaves partial state behind on a panic mid-call,
    /// each checks before it mutates.
    fn lock(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Blocks the **calling OS thread** until `resource` is granted to
    /// `task`, or returns immediately on a genuine logic error
    /// (`AlreadyWaiting` — a task cannot usefully wait on a second resource
    /// while already queued for a first).
    ///
    /// Safe to call repeatedly while blocked: `ResourceTracker::acquire` is
    /// already idempotent for a task re-asking about the resource it's
    /// queued on (documented on `acquire` itself), which is exactly what
    /// happens on every spurious or real wake this loop re-checks.
    pub fn blocking_acquire(&self, task: TaskId, resource: ResourceId) -> Result<(), ResourceError> {
        let mut guard = self.lock();
        loop {
            let State { tracker, graph } = &mut *guard;
            match tracker.acquire(task, resource, graph) {
                Ok(Acquired::Granted) => return Ok(()),
                Ok(Acquired::Blocked { .. }) => {
                    guard = self.changed.wait(guard).unwrap_or_else(|p| p.into_inner());
                }
                Err(e) => return Err(e),
            }
        }
    }

    /// Releases `resource` from `task` and wakes every thread waiting on
    /// *any* resource — cheaper to reason about correctly than computing
    /// exactly who could now proceed, and `blocking_acquire`'s loop makes a
    /// spurious wake harmless: a thread not actually granted just re-blocks.
    pub fn release(&self, task: TaskId, resource: ResourceId) -> Result<Option<TaskId>, ResourceError> {
        let mut guard = self.lock();
        let granted = {
            let State { tracker, graph } = &mut *guard;
            tracker.release(task, resource, graph)?
        };
        drop(guard);
        self.changed.notify_all();
        Ok(granted)
    }

    /// Force-releases **every** resource `task` currently holds — the
    /// generic form of this workspace's existing deadlock resolution
    /// policy (`neos/src/main.rs`: lowest `TaskId` in the cycle is the
    /// victim, everything it holds is force-released). The sequential demo
    /// could hand-pick the one resource its two-task scenario's victim
    /// held; a resolver that doesn't know the scenario in advance has to
    /// ask what the victim actually holds first — see
    /// `ResourceTracker::resources_held_by`.
    pub fn force_release_all(&self, task: TaskId) -> Vec<ResourceId> {
        let held = {
            let guard = self.lock();
            guard.tracker.resources_held_by(task)
        };
        for &resource in &held {
            // A resource this exact call already confirmed `task` holds
            // cannot fail with `NotHolder` between that read and this
            // release — the lock stays held across neither call, but
            // nothing else can make `task` stop holding a resource except
            // `task` itself releasing it, which force-resolution is
            // standing in for.
            let _ = self.release(task, resource);
        }
        held
    }

    pub fn detect_cycle(&self) -> Option<Vec<TaskId>> {
        self.lock().graph.detect_cycle()
    }

    pub fn has_deadlock(&self) -> bool {
        self.lock().graph.has_deadlock()
    }

    pub fn holder_of(&self, resource: ResourceId) -> Option<TaskId> {
        self.lock().tracker.holder_of(resource)
    }
}

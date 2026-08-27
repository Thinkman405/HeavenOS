//! Resource acquisition/release, feeding the wait-for graph.
//!
//! [`deadlock::WaitForGraph`](crate::deadlock::WaitForGraph) detects cycles in
//! waits that "something else must record" — its own implementation log says
//! so directly: "Nothing yet acquires or releases resources." This is that
//! something: it tracks which task holds which named resource and turns
//! `acquire`/`release` calls into the graph's own edges, so a caller never
//! computes "who holds this" by hand.
//!
//! **Detection only**, per the subsystem's own boundary (contract §8: load
//! equilibrium is not deadlock *resolution*). Handing a freed resource to the
//! next waiter is bookkeeping the tracker must do to keep the graph accurate
//! — a released resource cannot still show a wait edge pointing at its old
//! holder — not resolution: nothing here ever breaks a cycle that already
//! exists. A held cycle stays held until whatever manages the tasks
//! themselves intervenes.

use crate::deadlock::WaitForGraph;
use crate::scheduler::TaskId;
use std::collections::HashMap;
use std::fmt;

/// Names a resource that tasks contend over. Opaque on purpose — the tracker
/// does not care what a resource *is* (a lock, a device, a cell), only who
/// holds it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ResourceId(pub u64);

/// Outcome of a successful [`ResourceTracker::acquire`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Acquired {
    /// The resource was free, or `task` already held it; granted immediately,
    /// no wait edge recorded.
    Granted,
    /// `holder` already has the resource; `task` now waits, recorded as a
    /// `task -> holder` edge in the graph.
    Blocked { holder: TaskId },
}

/// Named for the physical failure, per `_mkb/test-doctrine.md`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceError {
    /// `task` tried to release a resource it does not currently hold.
    NotHolder { task: TaskId, resource: ResourceId },
    /// `task` tried to acquire `resource` while already blocked on a
    /// *different* resource. The tracker keeps at most one outstanding wait
    /// edge per task — every deadlock the contract names (two locks taken in
    /// opposite orders) blocks on exactly one resource at a time — and a
    /// second live wait would break the one-edge-per-waiter invariant
    /// `release` relies on to retarget edges exactly rather than guess.
    AlreadyWaiting {
        task: TaskId,
        already_waiting_on: ResourceId,
    },
}

impl fmt::Display for ResourceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotHolder { task, resource } => write!(
                f,
                "task {task:?} does not hold resource {resource:?}, cannot release it"
            ),
            Self::AlreadyWaiting {
                task,
                already_waiting_on,
            } => write!(
                f,
                "task {task:?} is already waiting on resource {already_waiting_on:?}"
            ),
        }
    }
}

impl std::error::Error for ResourceError {}

/// Tracks who holds which resource and who is queued for it, and keeps a
/// caller-supplied [`WaitForGraph`] in exact sync with that state.
#[derive(Debug, Clone, Default)]
pub struct ResourceTracker {
    holders: HashMap<ResourceId, TaskId>,
    /// FIFO queue per resource, arrival order.
    waiters: HashMap<ResourceId, Vec<TaskId>>,
    /// Reverse index: the single resource (if any) a task is blocked on.
    waiting_on: HashMap<TaskId, ResourceId>,
}

impl ResourceTracker {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn holder_of(&self, resource: ResourceId) -> Option<TaskId> {
        self.holders.get(&resource).copied()
    }

    pub fn is_waiting(&self, task: TaskId) -> Option<ResourceId> {
        self.waiting_on.get(&task).copied()
    }

    /// Every resource `task` currently holds — the piece a generic deadlock
    /// resolver needs and single-victim call sites haven't: this workspace's
    /// existing demo resolution (`neos/src/main.rs`) hand-picks which one
    /// resource its two-task scenario's victim holds, since it already knows
    /// the scenario. A resolver that doesn't know the scenario in advance
    /// needs to ask, not assume a task holds exactly one thing.
    pub fn resources_held_by(&self, task: TaskId) -> Vec<ResourceId> {
        self.holders
            .iter()
            .filter(|&(_, &holder)| holder == task)
            .map(|(&resource, _)| resource)
            .collect()
    }

    /// Acquire `resource` for `task`, feeding `graph` if `task` has to wait.
    pub fn acquire(
        &mut self,
        task: TaskId,
        resource: ResourceId,
        graph: &mut WaitForGraph,
    ) -> Result<Acquired, ResourceError> {
        let Some(&holder) = self.holders.get(&resource) else {
            self.holders.insert(resource, task);
            return Ok(Acquired::Granted);
        };
        if holder == task {
            return Ok(Acquired::Granted); // already held, reentrant no-op
        }
        if let Some(&existing) = self.waiting_on.get(&task) {
            if existing == resource {
                // Same call repeated while still blocked: idempotent.
                return Ok(Acquired::Blocked { holder });
            }
            return Err(ResourceError::AlreadyWaiting {
                task,
                already_waiting_on: existing,
            });
        }
        self.waiters.entry(resource).or_default().push(task);
        self.waiting_on.insert(task, resource);
        graph.add_wait(task, holder);
        Ok(Acquired::Blocked { holder })
    }

    /// Release `resource` from `task`. If another task was queued for it,
    /// that task is granted the resource immediately and every remaining
    /// queued task's wait edge is retargeted from the old holder to the new
    /// one — otherwise their edges would keep pointing at a task that no
    /// longer holds anything, and the graph would be stale rather than
    /// accurate. Returns the task granted next, if any.
    pub fn release(
        &mut self,
        task: TaskId,
        resource: ResourceId,
        graph: &mut WaitForGraph,
    ) -> Result<Option<TaskId>, ResourceError> {
        if self.holders.get(&resource) != Some(&task) {
            return Err(ResourceError::NotHolder { task, resource });
        }
        self.holders.remove(&resource);

        let queue = self.waiters.entry(resource).or_default();
        if queue.is_empty() {
            return Ok(None);
        }
        let next = queue.remove(0);
        self.waiting_on.remove(&next);
        graph.remove_wait(next, task);

        for &still_waiting in queue.iter() {
            graph.remove_wait(still_waiting, task);
            graph.add_wait(still_waiting, next);
        }

        self.holders.insert(resource, next);
        Ok(Some(next))
    }
}

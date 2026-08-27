//! A genuine multi-tenant sandbox — composing [`crate::concurrent`]'s real
//! threads and [`crate::vm::Domain::Guest`]'s privilege check into something
//! that answers a question neither alone does: not just "is this program
//! trusted or not," but **"whose memory is this, among several mutually
//! untrusted tenants running at the same time."**
//!
//! Everything a `Sandbox` protects against is real, not simulated:
//! `symphony_kernel::ConcurrentPool` is a real `Mutex`-guarded pool real
//! threads share; `symphony_kernel::ConcurrentTracker` is real, thread-
//! blocking resource contention; every tenant's program runs on its own
//! real OS thread via [`crate::concurrent::run_program`]. A sandbox whose
//! isolation was only ever exercised on one thread would be a claim, not a
//! proof — this workspace already found a real bug (a preempted task's own
//! thread trying to release a resource already taken from it) that only a
//! second, genuinely concurrent thread could produce. `Sandbox` inherits
//! that same real-concurrency discipline rather than adding a second one.
//!
//! # What's actually new here, versus `Domain`/`reserve_cells` alone
//!
//! `vm::Domain` is a two-tier model: `Kernel` (trusted, unrestricted) or
//! `Guest` (restricted to whatever `reserve_cells` didn't mark off-limits).
//! That's privilege, not tenancy — every `Guest` program shares the *same*
//! restricted region as every other. `Sandbox` adds an ownership map,
//! `CellId -> Owner`, so **which cells are off-limits differs per tenant**:
//! tenant A's own admitted memory is off-limits to tenant B and vice versa,
//! not just to some anonymous "guest" category. No new privilege mechanism
//! was needed to build this — `Sandbox::run` computes, for the tenant about
//! to run, the set of cells *everyone else* owns (every other tenant's
//! memory, plus anything reserved for the kernel), and hands that to the
//! already-real, already-tested `Domain::Guest` check via
//! [`crate::concurrent::run_program`] unchanged. Composition, not a new
//! enforcement point.
//!
//! # A real, stated limit: resource ids are a shared namespace
//!
//! Memory is tenant-scoped; `ResourceId`s are not. Two tenants that happen
//! to `acquire` the same literal resource id genuinely contend with each
//! other through the same real `ConcurrentTracker` — which may be exactly
//! what a host wants (a resource meant to be shared across tenants) or a
//! genuine collision. `Sandbox` does not guess which and does not silently
//! remap resource ids per tenant; a host that wants per-tenant resource
//! namespaces has to arrange that itself (e.g. by convention, prefixing
//! ids). Recorded here rather than glossed over, the same discipline this
//! workspace already applies to every other real limit it ships with.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};

use lattice::tessellation::CellId;
use symphony_kernel::{ConcurrentPool, ConcurrentTracker, TaskId};

use crate::concurrent::run_program;
use crate::vm::{Domain, Instruction, ProgramOutcome};

/// Who a cell is admitted to. `Kernel` cells are off-limits to every
/// tenant; a `Tenant` cell is off-limits to every *other* tenant, but not
/// its own.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Owner {
    Kernel,
    Tenant(TaskId),
}

/// Named for the failure, per `_mkb/test-doctrine.md`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SandboxError {
    /// Admission refused: `cell` is already owned by someone else. Refusing
    /// rather than silently reassigning ownership is the point — a sandbox
    /// that let a later admission steal an earlier tenant's memory would
    /// not be a sandbox.
    CellAlreadyOwned { cell: CellId, by: Owner },
}

impl std::fmt::Display for SandboxError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CellAlreadyOwned { cell, by } => {
                write!(f, "cell {cell:?} is already owned by {by:?}")
            }
        }
    }
}

impl std::error::Error for SandboxError {}

/// A real, shared pool and resource tracker, plus a real ownership map over
/// them. Every program run through a `Sandbox` runs as [`Domain::Guest`] —
/// there is no trusted tier here; code that needs [`Domain::Kernel`] does
/// not belong in a sandbox meant to hold mutually untrusted tenants.
pub struct Sandbox {
    pool: Arc<ConcurrentPool>,
    tracker: Arc<ConcurrentTracker>,
    ownership: RwLock<HashMap<CellId, Owner>>,
}

impl Sandbox {
    pub fn new(cells: usize, cell_capacity: usize) -> Self {
        Self {
            pool: ConcurrentPool::new(cells, cell_capacity),
            tracker: ConcurrentTracker::new(),
            ownership: RwLock::new(HashMap::new()),
        }
    }

    /// The real, shared pool every tenant's `store`/`load` ultimately goes
    /// through — exposed so a host can pick real cells to admit tenants to
    /// (`sandbox.pool().address_at(n)`), the same way any other caller of
    /// `ConcurrentPool` would.
    pub fn pool(&self) -> &ConcurrentPool {
        &self.pool
    }

    /// Mark cells off-limits to every tenant, unconditionally. Refuses if
    /// any of them are already owned by anyone — including re-reserving a
    /// cell already reserved for the kernel, since a second reservation
    /// silently succeeding would hide a caller's own logic error.
    ///
    /// # Errors
    /// [`SandboxError::CellAlreadyOwned`] naming the first conflicting cell.
    pub fn reserve_kernel_cells(&self, cells: impl IntoIterator<Item = CellId>) -> Result<(), SandboxError> {
        self.admit(Owner::Kernel, cells)
    }

    /// Admit `tenant` to exclusive ownership of `cells`. Refuses — leaving
    /// ownership exactly as it was before the call, not partially applied —
    /// if any requested cell is already owned by the kernel or a *different*
    /// tenant. Admitting a tenant to cells it already owns is a no-op, not
    /// an error: a tenant re-admitted to its own memory hasn't taken
    /// anything from anyone.
    ///
    /// # Errors
    /// [`SandboxError::CellAlreadyOwned`] naming the first conflicting cell.
    pub fn admit_tenant(
        &self,
        tenant: TaskId,
        cells: impl IntoIterator<Item = CellId>,
    ) -> Result<(), SandboxError> {
        self.admit(Owner::Tenant(tenant), cells)
    }

    fn admit(&self, owner: Owner, cells: impl IntoIterator<Item = CellId>) -> Result<(), SandboxError> {
        let cells: Vec<CellId> = cells.into_iter().collect();
        let mut map = self.ownership.write().unwrap_or_else(|p| p.into_inner());
        for &cell in &cells {
            if let Some(&existing) = map.get(&cell) {
                if existing != owner {
                    return Err(SandboxError::CellAlreadyOwned { cell, by: existing });
                }
            }
        }
        for cell in cells {
            map.insert(cell, owner);
        }
        Ok(())
    }

    /// Every cell `tenant` must **not** touch: the kernel's, and every
    /// other tenant's. Cells nobody has been admitted to at all are shared,
    /// open ground — a sandbox that refused everything unadmitted by
    /// default would just be `Domain::Kernel` refusing everything, not a
    /// tenancy model.
    fn off_limits_for(&self, tenant: TaskId) -> HashSet<CellId> {
        self.ownership
            .read()
            .unwrap_or_else(|p| p.into_inner())
            .iter()
            .filter(|&(_, &owner)| owner != Owner::Tenant(tenant))
            .map(|(&cell, _)| cell)
            .collect()
    }

    /// Run one tenant's program on the calling thread. See [`Self::run_many`]
    /// for running several tenants concurrently, which is the actual
    /// multi-tenant claim — a single `run` call proves the privilege check,
    /// not the concurrency.
    pub fn run(&self, tenant: TaskId, instructions: &[Instruction]) -> ProgramOutcome {
        let reserved = Arc::new(self.off_limits_for(tenant));
        run_program(&self.pool, &self.tracker, &reserved, tenant, Domain::Guest, instructions)
    }

    /// Run several tenants' programs **concurrently**, one real OS thread
    /// each, all against this sandbox's one real shared pool and tracker —
    /// the actual multi-tenant claim: genuinely simultaneous, mutually
    /// untrusted execution, not isolation only ever exercised one tenant at
    /// a time.
    pub fn run_many(&self, programs: Vec<(TaskId, Vec<Instruction>)>) -> Vec<ProgramOutcome> {
        let handles: Vec<_> = programs
            .into_iter()
            .map(|(tenant, instructions)| {
                let pool = Arc::clone(&self.pool);
                let tracker = Arc::clone(&self.tracker);
                let reserved = Arc::new(self.off_limits_for(tenant));
                std::thread::spawn(move || {
                    run_program(&pool, &tracker, &reserved, tenant, Domain::Guest, &instructions)
                })
            })
            .collect();

        handles
            .into_iter()
            .map(|h| h.join().expect("a tenant's program thread must not panic"))
            .collect()
    }
}

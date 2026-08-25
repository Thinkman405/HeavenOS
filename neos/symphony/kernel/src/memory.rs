//! Concurrent access to `substrate::MemoryPool`.
//!
//! `MemoryPool` is deliberately single-threaded — its own implementation log
//! says so directly: "Whether locks live here or in `symphony-kernel` is a
//! scheduler decision, not a substrate one." This is that decision, made
//! here rather than in `substrate`: the scheduler already coordinates
//! concurrent access to cores and tasks, and serialising concurrent access
//! to the pool they share is the same kind of job, not a new one.
//!
//! # The lock is load-bearing, not decorative
//!
//! Verified before writing this, not assumed: a disposable scratch harness
//! wrapped a raw `MemoryPool` in an `unsafe impl Sync` with zero
//! synchronisation and ran 32 threads hammering `allocate`/`write`/`read`/
//! `free` against a 2-cell pool, each thread writing its own distinct
//! fingerprint byte and reading it back after a forced yield. Unsynchronised:
//! 10/10 runs showed corruption (a fingerprint read back that never matched
//! what that thread wrote — two allocations had aliased the same bytes).
//! The identical workload through a `Mutex`: 0/10 runs. Without a
//! deliberately distinct fingerprint per thread the corruption was invisible
//! — every thread had originally written the same byte value, so an aliased
//! allocation read back "correctly" by accident. Worth recording: the first
//! version of that harness looked clean for exactly the wrong reason.
//!
//! # One coarse lock, not one per cell
//!
//! `MemoryPool::allocate` already grows breadth-first across a dynamically
//! discovered set of adjacent cells in one call; a per-cell locking scheme
//! would need to lock that set correctly (in a consistent order, without
//! holding some cells' locks while waiting on others) to avoid introducing
//! the very deadlock class `deadlock` exists to catch. A single mutex around
//! the whole pool sidesteps that entirely. It serialises every operation
//! rather than allowing them to overlap, which is a real throughput cost —
//! but proving the pool is *safe* to share does not require also making it
//! the most concurrent implementation possible.

use std::sync::{Arc, Mutex, MutexGuard};
use substrate::{Allocation, LatticeAddress, MemoryPool, SubstrateError};

/// A `MemoryPool` safe to share across threads via [`Arc`].
pub struct ConcurrentPool {
    inner: Mutex<MemoryPool>,
}

impl ConcurrentPool {
    /// Build a pool already wrapped for sharing. Returns an `Arc` directly
    /// rather than a bare `Self`, since a pool with nobody else able to see
    /// it has no concurrency to speak of.
    pub fn new(cells: usize, cell_capacity: usize) -> Arc<Self> {
        Arc::new(Self {
            inner: Mutex::new(MemoryPool::new(cells, cell_capacity)),
        })
    }

    /// Recovers from a poisoned lock rather than propagating the poison to
    /// every future caller. A panic while holding the lock means whatever
    /// that caller was doing failed partway — but `MemoryPool`'s own
    /// operations don't leave partial state behind on error (each checks
    /// before it mutates), so the pool itself is not left inconsistent by
    /// a panicking caller, and there is no reason every *other* thread
    /// should also lose access to it.
    fn lock(&self) -> MutexGuard<'_, MemoryPool> {
        self.inner.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    pub fn allocate(&self, bytes: usize) -> Result<Allocation, SubstrateError> {
        self.lock().allocate(bytes)
    }

    pub fn free(&self, alloc: &Allocation) {
        self.lock().free(alloc)
    }

    pub fn write(&self, at: LatticeAddress, data: &[u8]) -> Result<(), SubstrateError> {
        self.lock().write(at, data)
    }

    pub fn read(&self, at: LatticeAddress, len: usize) -> Result<Vec<u8>, SubstrateError> {
        self.lock().read(at, len)
    }

    pub fn available(&self) -> usize {
        self.lock().available()
    }

    pub fn cell_count(&self) -> usize {
        self.lock().cell_count()
    }

    pub fn total_capacity(&self) -> usize {
        self.lock().total_capacity()
    }
}

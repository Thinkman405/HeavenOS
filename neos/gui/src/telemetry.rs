//! Reading real system state for visualisation.
//!
//! PRD §9: "System load, memory usage, and network traffic are rendered as
//! real-time standing waves representing constructive and destructive energy
//! states."
//!
//! This module is the seam between the running system and the renderer. It
//! reads `symphony-kernel`'s actual load field, `ftg`'s actual delivery
//! outcomes, and `substrate`'s actual pool usage - rather than taking a
//! number a caller made up.
//!
//! ## Memory needs no normalisation trick, and that is worth contrasting
//!
//! Core load is joules, of order `1e-25`, so
//! [`SystemSnapshot::normalised_load`] has to scale against the busiest core
//! before it means anything on screen - see below. Memory usage is already
//! bytes over bytes: [`SystemSnapshot::memory_utilisation`] is a ratio of two
//! same-unit quantities and lands in `[0, 1]` on its own. There
//! is no analogous trap here, and no analogous fix needed - the two readouts
//! differ because the underlying quantities do, not because one was built
//! more carefully than the other.
//!
//! ## Amplitude must be normalised, and this is not cosmetic
//!
//! A task at 2 GHz costs `E = C_H * nu ~ 5.3e-25 J`. Mapping that to amplitude
//! directly renders **every core as visually zero** - the display would show an
//! idle machine under full load.
//!
//! [`SystemSnapshot::normalised_load`] therefore scales against the busiest
//! core, giving `[0, 1]`. The raw joules stay available for readouts that want
//! a number, but nothing draws from them.
//!
//! This is the same absolute-vs-relative trap that produced a false
//! convergence in `symphony-kernel` and a flaky tolerance in `ftg`. Third
//! occurrence; designed for rather than discovered here.
//!
//! ## The traffic mapping is a stated assumption
//!
//! Delivered packets contribute constructively, failed ones destructively, so
//! net amplitude reads as network health and zero means as many failures as
//! successes. The PRD asks for "constructive and destructive energy states"
//! but does not define the mapping - this one is chosen, not derived.

use ftg::transport::Delivery;
use substrate::MemoryPool;
use symphony_kernel::Scheduler;

/// A point-in-time reading of the running system.
///
/// Deliberately a **snapshot**, not a live borrow: the renderer must not hold
/// the scheduler while drawing, and an explicit copy makes the data flow
/// testable.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SystemSnapshot {
    core_load: Vec<f64>,
    task_count: usize,
    delivered: usize,
    dissipated: usize,
    link_lost: usize,
    memory_total: usize,
    memory_available: usize,
}

impl SystemSnapshot {
    pub fn new() -> Self {
        Self::default()
    }

    /// Read the kernel's real per-core load.
    pub fn from_scheduler(scheduler: &Scheduler) -> Self {
        Self {
            core_load: scheduler.load_per_core(),
            task_count: scheduler.task_count(),
            ..Self::default()
        }
    }

    /// Read the pool's real memory usage.
    ///
    /// Mirrors [`observe`](Self::observe): an incremental read against a live
    /// snapshot, not a second constructor, since a caller building a full
    /// picture of the system needs load, memory, and traffic together and
    /// `..Self::default()` in a constructor would silently wipe whichever of
    /// those was read first.
    pub fn read_memory(&mut self, pool: &MemoryPool) {
        self.memory_total = pool.total_capacity();
        self.memory_available = pool.available();
    }

    /// Record what became of a packet.
    ///
    /// Takes `ftg`'s real [`Delivery`], so the three outcomes cannot drift out
    /// of step with transport.
    pub fn observe(&mut self, delivery: &Delivery) {
        match delivery {
            Delivery::Arrived { .. } => self.delivered += 1,
            Delivery::Dissipated { .. } => self.dissipated += 1,
            Delivery::LinkLost { .. } => self.link_lost += 1,
        }
    }

    pub fn core_count(&self) -> usize {
        self.core_load.len()
    }

    pub fn task_count(&self) -> usize {
        self.task_count
    }

    /// Raw per-core load in joules. For readouts, never for amplitude.
    pub fn raw_load(&self) -> &[f64] {
        &self.core_load
    }

    pub fn total_load(&self) -> f64 {
        self.core_load.iter().sum()
    }

    pub fn delivered(&self) -> usize {
        self.delivered
    }

    pub fn dissipated(&self) -> usize {
        self.dissipated
    }

    pub fn link_lost(&self) -> usize {
        self.link_lost
    }

    pub fn failed(&self) -> usize {
        self.dissipated + self.link_lost
    }

    /// Total pool capacity in bytes, across every cell.
    pub fn memory_total(&self) -> usize {
        self.memory_total
    }

    /// Free capacity in bytes.
    pub fn memory_available(&self) -> usize {
        self.memory_available
    }

    /// Bytes currently in use. `total - available`, never underflows even for
    /// an unread snapshot where both are zero.
    pub fn memory_used(&self) -> usize {
        self.memory_total.saturating_sub(self.memory_available)
    }

    /// Fraction of the pool in use, in `[0, 1]`.
    ///
    /// Already a ratio of two same-unit quantities - see the module docs for
    /// why this needs no peak-scaling trick the way [`normalised_load`]
    /// does. Zero for an unread snapshot (`total == 0`), which reads as "no
    /// activity" exactly like an idle load field does.
    ///
    /// [`normalised_load`]: Self::normalised_load
    pub fn memory_utilisation(&self) -> f64 {
        if self.memory_total == 0 {
            return 0.0;
        }
        self.memory_used() as f64 / self.memory_total as f64
    }

    /// Per-core load scaled to `[0, 1]` against the busiest core.
    ///
    /// **This is what gets drawn.** An idle system gives all zeros; a perfectly
    /// balanced one gives all ones. The scaling is what makes a `1e-25` J field
    /// visible at all - see the module docs.
    pub fn normalised_load(&self) -> Vec<f64> {
        let peak = self
            .core_load
            .iter()
            .cloned()
            .fold(f64::NEG_INFINITY, f64::max);
        if !(peak > 0.0) {
            return vec![0.0; self.core_load.len()];
        }
        self.core_load.iter().map(|l| l / peak).collect()
    }

    /// Spread of the normalised load: `0` when balanced, `1` when one core
    /// carries everything.
    ///
    /// Scale-free by construction, so it means the same thing whether loads are
    /// of order 1 or `1e-25`.
    pub fn imbalance(&self) -> f64 {
        let n = self.normalised_load();
        if n.is_empty() {
            return 0.0;
        }
        let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
        for v in n {
            lo = lo.min(v);
            hi = hi.max(v);
        }
        hi - lo
    }

    /// Net network health in `[-1, 1]`: `+1` all delivered, `-1` all failed,
    /// `0` when successes and failures cancel exactly.
    pub fn network_balance(&self) -> f64 {
        let total = self.delivered + self.failed();
        if total == 0 {
            return 0.0;
        }
        (self.delivered as f64 - self.failed() as f64) / total as f64
    }
}

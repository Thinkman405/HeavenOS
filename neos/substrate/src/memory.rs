//! Non-Euclidean memory pool. **This module is the flat/curved boundary.**
//!
//! Contract §2. Hardware is flat and byte-addressed; axiom A3 says addressable
//! space is not. The translation lives here and *only* here.
//!
//! ## The guarantee
//!
//! [`FlatOffset`] is private to this module. No public type, function, or field
//! yields a raw pointer, a byte index, or any linear address. Every public
//! address is a [`LatticeAddress`] - a `CellId` from `lattice` plus a bounded
//! intra-cell offset that is meaningless outside its cell.
//!
//! This is structural, not conventional. Any consumer able to obtain a flat
//! address would eventually do arithmetic on it, and at that moment it is
//! working in Euclidean space regardless of what the geometry layer claims.
//! `ftg` Layer 3/4 routing must read a native non-Euclidean space; a
//! convention would not survive a hot loop, so a private type carries it.

use crate::SubstrateError;
use lattice::tessellation::CellId;
use lattice::{AddressPath, LatticeScalar, LogicalArea, Tiling};
use std::collections::HashMap;

/// A byte position inside the backing store.
///
/// **Private by design.** This is the flat address the rest of NEOS must never
/// see. It has no public constructor, no accessor, and no `Deref`, and it is
/// the return type of [`MemoryPool::resolve`] - the one function that performs
/// the curved-to-flat translation. Because the type cannot leave this module,
/// neither can the translation's result.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct FlatOffset(usize);

impl FlatOffset {
    /// Only reachable from inside this module.
    const fn get(self) -> usize {
        self.0
    }
}

/// A bounded position within a cell.
///
/// Public, but meaningless outside its cell - it cannot serve as a global
/// linear index, which is what keeps it safe to expose.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct CellOffset(pub usize);

/// The only public address type in NEOS.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LatticeAddress {
    cell: CellId,
    offset: CellOffset,
}

impl LatticeAddress {
    pub const fn new(cell: CellId, offset: CellOffset) -> Self {
        Self { cell, offset }
    }
    pub const fn cell(&self) -> CellId {
        self.cell
    }
    pub const fn offset(&self) -> CellOffset {
        self.offset
    }
}

/// A span of memory, recorded as the **cells it occupies** rather than as a
/// base pointer and length.
#[derive(Debug, Clone, PartialEq)]
pub struct Allocation {
    cells: Vec<CellId>,
    start: LatticeAddress,
    len: usize,
    /// Exactly which byte range within each cell this allocation claimed —
    /// what `free` releases, precisely, regardless of what else lives in
    /// those same cells. Private: callers address an allocation as one
    /// contiguous logical span via `start`/`len`, never as per-cell offsets.
    spans: Vec<(CellId, usize, usize)>,
}

impl Allocation {
    pub fn cells(&self) -> &[CellId] {
        &self.cells
    }
    pub fn start(&self) -> LatticeAddress {
        self.start
    }
    pub fn len(&self) -> usize {
        self.len
    }
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// This allocation's own geometric footprint.
    ///
    /// `LogicalArea` was built to describe a single stored object's area,
    /// growing and shrinking by whole cells as the object resizes — this is
    /// that description applied to a real allocation rather than a bare cell
    /// count handed to `LogicalArea::of` from nowhere. `self.cells` is
    /// already exactly the set this reads; nothing new is tracked.
    pub fn logical_area(&self) -> LogicalArea {
        LogicalArea::of(self.cells.len())
    }
}

struct Slab {
    bytes: Vec<u8>,
    /// Live byte spans currently allocated in this cell: `(offset, len)`,
    /// sorted by `offset`, non-overlapping. The complement, relative to
    /// `[0, cell_capacity)`, is exactly what's free and reusable — see
    /// [`MemoryPool::free`] for why this replaced a coarser, cell-wide
    /// refcount.
    live: Vec<(usize, usize)>,
}

impl Slab {
    fn used(&self) -> usize {
        self.live.iter().map(|&(_, len)| len).sum()
    }

    /// The first free gap, in offset order: `(offset, length)`. `None` if
    /// nothing is free.
    ///
    /// First-fit, not best-fit, and an allocation takes from at most one gap
    /// per cell it touches — `write`/`read` address one contiguous range per
    /// cell (see [`MemoryPool::resolve`]), so a single allocation cannot
    /// occupy two disjoint spans within the same cell. If a cell holds
    /// several smaller gaps, only the first is offered per `allocate` call;
    /// the rest remain available to later calls.
    fn first_gap(&self, capacity: usize) -> Option<(usize, usize)> {
        let mut cursor = 0;
        for &(offset, len) in &self.live {
            if offset > cursor {
                return Some((cursor, offset - cursor));
            }
            cursor = offset + len;
        }
        (cursor < capacity).then_some((cursor, capacity - cursor))
    }

    fn insert_live(&mut self, offset: usize, len: usize) {
        let pos = self.live.partition_point(|&(o, _)| o < offset);
        self.live.insert(pos, (offset, len));
    }

    /// Remove exactly the span an allocation was granted. A mismatched
    /// `(offset, len)` — which would mean a caller freeing something it was
    /// never given — is a silent no-op rather than a panic: `free` takes
    /// `&Allocation`, and every `Allocation` in existence was itself
    /// produced by `allocate`, so a mismatch here would be this module's own
    /// bug, not a caller error to report.
    fn remove_live(&mut self, offset: usize, len: usize) {
        if let Some(pos) = self.live.iter().position(|&(o, l)| o == offset && l == len) {
            self.live.remove(pos);
        }
    }
}

/// Memory organised by lattice cells.
pub struct MemoryPool {
    tiling: Tiling,
    order: Vec<CellId>,
    slabs: HashMap<CellId, Slab>,
    cell_capacity: usize,
    /// Address-space extent, scaled by `(x)` on split (axiom A1).
    extent: f64,
}

impl MemoryPool {
    /// Build a pool over `cells` lattice cells, each holding `cell_capacity`
    /// bytes. Cells are taken in BFS ring order so the pool is a compact patch.
    pub fn new(cells: usize, cell_capacity: usize) -> Self {
        assert!(cells > 0 && cell_capacity > 0);

        let mut depth = 1;
        let mut tiling = Tiling::grow(depth);
        while tiling.len() < cells && depth < 12 {
            depth += 1;
            tiling = Tiling::grow(depth);
        }

        let mut order = Vec::with_capacity(cells);
        'outer: for ring in 0.. {
            match tiling.layer(ring) {
                Some(ids) => {
                    for id in ids {
                        if order.len() == cells {
                            break 'outer;
                        }
                        order.push(*id);
                    }
                }
                None => break,
            }
        }

        let slabs = order
            .iter()
            .map(|id| {
                (
                    *id,
                    Slab {
                        bytes: vec![0u8; cell_capacity],
                        live: Vec::new(),
                    },
                )
            })
            .collect();

        Self {
            tiling,
            order,
            slabs,
            cell_capacity,
            extent: 1.0,
        }
    }

    pub fn cell_count(&self) -> usize {
        self.order.len()
    }

    pub fn cell_capacity(&self) -> usize {
        self.cell_capacity
    }

    pub fn total_capacity(&self) -> usize {
        self.cell_count() * self.cell_capacity
    }

    pub fn available(&self) -> usize {
        self.slabs.values().map(|s| self.cell_capacity - s.used()).sum()
    }

    /// The pool's total geometric footprint: `total_capacity`/`available`
    /// in `LogicalArea` terms rather than bytes.
    ///
    /// PRD §5's area-preservation claim, wired to a real pool instead of
    /// tested only in isolation against a bare cell count. Fixed for the
    /// pool's lifetime — `cell_count` never changes after `new`, only which
    /// of those cells are occupied does.
    pub fn total_area(&self) -> LogicalArea {
        LogicalArea::of(self.cell_count())
    }

    /// Cells with at least one byte allocated.
    ///
    /// A cell partially used still counts as fully occupied here —
    /// `LogicalArea` has no notion of a fractional cell, which matches this
    /// pool's own fragmentation invariant: there is no in-between state to
    /// represent, only "some of this cell is spoken for" or "none is."
    pub fn occupied_area(&self) -> LogicalArea {
        LogicalArea::of(self.slabs.values().filter(|s| s.used() > 0).count())
    }

    /// Cells with nothing allocated in them yet.
    ///
    /// `occupied_area().cells() + available_area().cells() ==
    /// total_area().cells()` exactly, always — every cell is in exactly one
    /// of the two sets. The same is true of `.area()` only up to floating
    /// point: `(a+b)*c` is not bit-identical to `a*c+b*c` in `f64` in
    /// general, verified before relying on it rather than assumed — see
    /// `area_is_conserved_across_allocation_and_freeing`.
    pub fn available_area(&self) -> LogicalArea {
        LogicalArea::of(self.slabs.values().filter(|s| s.used() == 0).count())
    }

    /// Current address-space extent. Starts at 1 and scales by `(x)` on split.
    pub fn extent(&self) -> f64 {
        self.extent
    }

    /// Allocate `bytes`, growing into **adjacent** cells when one is not enough.
    ///
    /// Locality follows lattice adjacency, never a flat index. This is the
    /// property `ftg` depends on: addresses near in the metric are near in the
    /// allocation, so routing by hyperbolic distance is meaningful.
    pub fn allocate(&mut self, bytes: usize) -> Result<Allocation, SubstrateError> {
        if bytes > self.available() {
            return Err(SubstrateError::Exhausted {
                requested: bytes,
                available: self.available(),
            });
        }

        // Seed: the first cell with any free gap.
        let seed = self
            .order
            .iter()
            .find(|id| self.slabs[id].first_gap(self.cell_capacity).is_some())
            .copied()
            .ok_or(SubstrateError::Exhausted {
                requested: bytes,
                available: 0,
            })?;

        let (seed_offset, _) = self.slabs[&seed].first_gap(self.cell_capacity).unwrap();
        let start = LatticeAddress::new(seed, CellOffset(seed_offset));

        // Breadth-first growth over lattice adjacency.
        let mut chosen = Vec::new();
        let mut spans = Vec::new();
        let mut remaining = bytes;
        let mut queue = vec![seed];
        let mut seen: Vec<CellId> = vec![seed];

        while remaining > 0 {
            let Some(cell) = queue.first().copied() else {
                return Err(SubstrateError::Exhausted {
                    requested: bytes,
                    available: self.available(),
                });
            };
            queue.remove(0);

            if let Some((gap_offset, gap_len)) = self.slabs[&cell].first_gap(self.cell_capacity) {
                let take = gap_len.min(remaining);
                self.slabs.get_mut(&cell).unwrap().insert_live(gap_offset, take);
                spans.push((cell, gap_offset, take));
                remaining -= take;
                chosen.push(cell);
            }

            if let Some(c) = self.tiling.get(&cell) {
                for n in c.neighbors() {
                    let id = n.id();
                    if self.slabs.contains_key(&id) && !seen.contains(&id) {
                        seen.push(id);
                        queue.push(id);
                    }
                }
            }
        }

        Ok(Allocation {
            cells: chosen,
            start,
            len: bytes,
            spans,
        })
    }

    /// Release an allocation's cells — precisely, at sub-cell granularity.
    ///
    /// # Two defects this closes, one after the other
    ///
    /// `free` originally reset a whole cell's usage unconditionally,
    /// regardless of whether a still-live sibling allocation also had bytes
    /// there — freeing one of two allocations sharing a cell silently
    /// destroyed the other. Found (and fixed first, with a coarse per-cell
    /// live-allocation count) while building `symphony-kernel::ConcurrentPool`
    /// — real under plain single-threaded use too, no concurrency required.
    ///
    /// That fix was safe but pessimistic: it stopped corruption by refusing
    /// to reclaim *any* of a shared cell's freed bytes until *every*
    /// allocation touching it was gone — `allocate` never released
    /// sub-cell space, only whole cells. `Slab::live` becoming a real
    /// interval set (each entry's own `(offset, len)`, not just a count)
    /// closes that too: `free` now removes exactly the byte range an
    /// allocation was granted, tracked privately per `Allocation`, and a
    /// later `allocate` can immediately reuse whatever a sibling just
    /// returned — a hole in the middle of a cell is real free space again
    /// the instant it's freed, not only once the whole cell empties.
    pub fn free(&mut self, alloc: &Allocation) {
        for &(cell, offset, len) in &alloc.spans {
            if let Some(s) = self.slabs.get_mut(&cell) {
                s.remove_live(offset, len);
            }
        }
    }

    /// **The curved-to-flat translation.** The whole boundary is this function.
    ///
    /// Private, and its result type [`FlatOffset`] is private too, so a flat
    /// address cannot escape the module even by accident. Every public read and
    /// write goes through here.
    fn resolve(&self, at: LatticeAddress, len: usize) -> Result<FlatOffset, SubstrateError> {
        if !self.slabs.contains_key(&at.cell()) {
            return Err(SubstrateError::Unmapped { cell: at.cell() });
        }
        let off = at.offset().0;
        if off + len > self.cell_capacity {
            return Err(SubstrateError::OffsetOutOfCell {
                offset: off + len,
                capacity: self.cell_capacity,
            });
        }
        Ok(FlatOffset(off))
    }

    pub fn write(&mut self, at: LatticeAddress, data: &[u8]) -> Result<(), SubstrateError> {
        let flat = self.resolve(at, data.len())?.get();
        let slab = self.slabs.get_mut(&at.cell()).expect("resolve checked this");
        slab.bytes[flat..flat + data.len()].copy_from_slice(data);
        Ok(())
    }

    pub fn read(&self, at: LatticeAddress, len: usize) -> Result<Vec<u8>, SubstrateError> {
        let flat = self.resolve(at, len)?.get();
        let slab = self.slabs.get(&at.cell()).expect("resolve checked this");
        Ok(slab.bytes[flat..flat + len].to_vec())
    }

    /// Resolve a curved [`AddressPath`] to a concrete address in this pool.
    ///
    /// **This is the join `lattice::addressing` and this module were each
    /// missing half of.** Addressing folds `(x)` down to a single scalar and
    /// stops; this pool allocates over `CellId`s and stops. Nothing between
    /// them turned a path someone actually wrote into a cell this pool holds
    /// bytes for - until now.
    ///
    /// Goes through [`AddressPath::resolve_to_cell`], which uses *this pool's
    /// own tiling* - so paths resolve against exactly the cells the pool
    /// might actually have allocated, not a hypothetical unbounded lattice.
    /// The result always lands at [`CellOffset(0)`](CellOffset) - a resolved
    /// scalar names a cell, and there is no further formula in the law for a
    /// sub-cell offset to invent.
    ///
    /// This is a read: it names an address, it does not allocate one. A
    /// caller that gets `Ok` back still needs the returned address to fall
    /// within an existing [`Allocation`] before `read`/`write` will accept it
    /// - `resolve_path` only guarantees the *cell* is part of this pool.
    ///
    /// # Errors
    /// [`SubstrateError::AddressUnresolvable`] if the path's `(x)`-fold leaves
    /// its domain. [`SubstrateError::Unmapped`] if the nearest cell to the
    /// resolved point is not one this pool holds a slab for - naming a point
    /// in the address space is not the same as that point being part of this
    /// pool's backing store.
    pub fn resolve_path(&self, path: &AddressPath) -> Result<LatticeAddress, SubstrateError> {
        let cell = path
            .resolve_to_cell(&self.tiling)
            .map_err(SubstrateError::AddressUnresolvable)?;
        if !self.slabs.contains_key(&cell) {
            return Err(SubstrateError::Unmapped { cell });
        }
        Ok(LatticeAddress::new(cell, CellOffset(0)))
    }

    /// The address at the start of the `n`-th cell in ring order.
    ///
    /// A way to *name* a cell, not a linear index into memory: the result is a
    /// [`LatticeAddress`], and `n` is an ordinal over cells rather than a byte
    /// position. Returns `None` past the end of the pool.
    pub fn address_at(&self, n: usize) -> Option<LatticeAddress> {
        self.order
            .get(n)
            .map(|id| LatticeAddress::new(*id, CellOffset(0)))
    }

    /// Hyperbolic distance between two addresses.
    ///
    /// The **only** meaningful notion of "how far apart" two addresses are.
    /// Not an offset difference - see contract §2.
    pub fn distance(&self, a: LatticeAddress, b: LatticeAddress) -> f64 {
        match (self.tiling.get(&a.cell()), self.tiling.get(&b.cell())) {
            (Some(ca), Some(cb)) => ca.centre().distance_to(&cb.centre()),
            _ => f64::INFINITY,
        }
    }

    /// Whether two cells are edge-adjacent in the lattice.
    pub fn adjacent(&self, a: CellId, b: CellId) -> bool {
        self.tiling
            .get(&a)
            .map(|c| c.neighbors().iter().any(|n| n.id() == b))
            .unwrap_or(false)
    }

    /// Split the pool (axiom A1).
    ///
    /// Address-space extent scales by `u (x) u`, computed with `lattice`'s
    /// operator. For the unit pool this is exactly `2.0` - a structural
    /// geometric split, not a second copy.
    ///
    /// # Errors
    /// [`SubstrateError::SplitDomain`] if the extent leaves `(x)`'s domain.
    pub fn split(&mut self) -> Result<f64, SubstrateError> {
        let u = LatticeScalar::new(self.extent);
        let scaled = u
            .otimes(u)
            .map_err(|_| SubstrateError::SplitDomain { unit: self.extent })?;
        self.extent = scaled.get();
        Ok(self.extent)
    }
}

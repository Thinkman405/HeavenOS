//! NEOS — end-to-end boot-to-report pass.
//!
//! Wires the seven built subsystems into one running process: `substrate`
//! boots, `symphony-kernel` schedules real tasks, `ftg` delivers a real frame
//! over a resonant session, `crystallisation` decodes a real image, a real
//! WAV tone, and a real crystallised video sequence, and `gui` reads the
//! resulting state into standing-wave amplitudes.
//!
//! No new physics. Every call here is already exercised by the workspace's
//! own test suite (396 assertions at the time this was written) — this
//! binary only composes verified pieces into one process; it proves nothing
//! new about correctness that the tests do not already prove.
//!
//! `gui` itself has no rendering surface — its own module docs say so
//! directly: "There is no framebuffer, window, or GPU binding here," and
//! that boundary stays put. [`render`] is a small rasteriser living in this
//! binary instead, consuming `gui`'s geometry and amplitude types to produce
//! an actual image — see that module for why it belongs here and not there.

mod export;
mod gif;
mod render;

/// Shared by every `StandingWave` this binary constructs, so `render`'s
/// animation can sample the exact same waves the report's numbers come from
/// rather than reconstructing them from different constants.
const WAVE_K: f64 = 1.2;
const WAVE_OMEGA: f64 = 3.0;

use crystallisation::{takens_embed, FrequencyMap, PhaseSpaceVector, PixelGrid, VolumetricTimeCrystal};
use ftg::session::{Link, Oscillator};
use ftg::transport::{Delivery, Gateway, Packet};
use ftg::Frame;
use gui::telemetry::SystemSnapshot;
use gui::visualization::{LoadVisualisation, TetryenVisualisation};
use gui::Tetryen;
use std::collections::HashSet;
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use substrate::{Hypervisor, TrapAction};
use symphony_kernel::bifurcation::{Phase, TaskModel};
use symphony_kernel::{
    ConcurrentPool, ConcurrentTracker, ResourceId, ResourceTracker, Scheduler, Task, TaskId,
    WaitForGraph,
};
use symphony_lang::concurrent::run_batch_concurrent;
use symphony_lang::vm::{compile as lang_compile, Vm as LangVm};
use symphony_lang::{
    lex as lang_lex, parse as lang_parse, Domain as LangDomain, Sandbox, TrapAction as LangTrapAction,
};

/// A tiny real PPM (P5, greyscale), not a hand-built `FrequencyMap` — the
/// same 4x4, 16-pixel shape `gui_crystallisation_integration.rs` uses, since
/// 16 divides evenly across the Tetryen's four faces.
fn embedded_ppm() -> Vec<u8> {
    let pixels: &[u8] = &[1, 2, 3, 4, 5, 6, 7, 8, 9, 1, 2, 3, 4, 5, 6, 7];
    let mut bytes = b"P5\n4 4\n255\n".to_vec();
    bytes.extend_from_slice(pixels);
    bytes
}

/// A minimal real PCM WAV (mono, 8 kHz, 16-bit) — bytes on disk, not a
/// hand-built sample vector, matching the discipline `embedded_ppm` follows.
fn wav_pcm(channels: u16, rate: u32, bits: u16, samples: &[i32]) -> Vec<u8> {
    let bytes_per_sample = usize::from(bits / 8);
    let data_len = samples.len() * bytes_per_sample;
    let mut w = Vec::new();
    w.extend_from_slice(b"RIFF");
    w.extend_from_slice(&(36u32 + data_len as u32).to_le_bytes());
    w.extend_from_slice(b"WAVE");
    w.extend_from_slice(b"fmt ");
    w.extend_from_slice(&16u32.to_le_bytes());
    w.extend_from_slice(&1u16.to_le_bytes());
    w.extend_from_slice(&channels.to_le_bytes());
    w.extend_from_slice(&rate.to_le_bytes());
    let block_align = channels * bytes_per_sample as u16;
    w.extend_from_slice(&(rate * u32::from(block_align)).to_le_bytes());
    w.extend_from_slice(&block_align.to_le_bytes());
    w.extend_from_slice(&bits.to_le_bytes());
    w.extend_from_slice(b"data");
    w.extend_from_slice(&(data_len as u32).to_le_bytes());
    for &s in samples {
        w.extend_from_slice(&(s as i16).to_le_bytes());
    }
    w
}

/// A short tone (a few periods of a sine), long enough for `takens_embed` at
/// `tau = 3` to produce several real embedded phase-space nodes.
fn embedded_wav() -> Vec<u8> {
    let samples: Vec<i32> = (0..64).map(|i| ((i as f64 * 0.4).sin() * 10000.0) as i32).collect();
    wav_pcm(1, 8000, 16, &samples)
}

/// A handful of tiny decoded PPM frames, rescaled the way `_mkb/timecrystal.md`
/// §5.3 requires: a realistic frame's raw pixel energy sits ~50 orders of
/// magnitude past what `C_H` can exactly quantise, so a caller wanting
/// quantised video supplies frames already scaled down — the same
/// discipline the audio path gets for free from a small-amplitude signal.
/// Scale verified against `crystallisation_codec.rs`'s own `2x2`-frame case.
fn embedded_video_frames() -> Vec<PixelGrid> {
    const SCALE: f64 = 3e-9;
    (0..20)
        .map(|i| {
            let v = (64.0 + 40.0 * (i as f64 * 0.3).sin()) as u8;
            let mut file = b"P5\n2 2\n255\n".to_vec();
            file.extend_from_slice(&[v, v, v, v]);
            let decoded = crystallisation::decode_ppm(&file).expect("well-formed embedded PPM frame");
            let rescaled: Vec<f64> = decoded.pixels().iter().map(|p| p * SCALE).collect();
            PixelGrid::new(decoded.height(), decoded.width(), rescaled)
                .expect("rescaling keeps the frame's already-valid dimensions")
        })
        .collect()
}

/// The loudest moment across a set of phase-space nodes: whichever single
/// node's own `TetryenVisualisation` reaches the largest peak.
fn busiest_visualisation(nodes: &[PhaseSpaceVector], k: f64, omega: f64) -> TetryenVisualisation {
    nodes
        .iter()
        .map(|n| TetryenVisualisation::from_phase_vector(n, k, omega))
        .max_by(|a, b| a.peak().partial_cmp(&b.peak()).unwrap())
        .expect("takens_embed/crystallise_video both refuse to return an empty node list")
}

/// Write both a still (`t = 0`) and a full-period animation for one
/// `TetryenVisualisation`, under `stem.ppm`/`stem.gif`. The load strip is
/// shared across every render this binary produces — it's the same real
/// kernel state regardless of which media source the tetryen panel shows.
fn render_and_report(
    label: &str,
    stem: &str,
    geometry: &Tetryen,
    vis: &TetryenVisualisation,
    load: &LoadVisualisation,
) {
    let still = render::render_still(geometry, vis, load, WAVE_K);
    let still_path = format!("{stem}.ppm");
    render::write_ppm(&still_path, render::STATIC_SIZE, &still)
        .expect("writing the rendered report image");
    println!("  rendered       {still_path} (640x640 PPM, t = 0) — {label}");

    let frames = render::render_animation(geometry, vis, load, WAVE_K, WAVE_OMEGA);
    let gif_path = format!("{stem}.gif");
    gif::write_gif(
        &gif_path,
        render::ANIM_SIZE as u16,
        render::ANIM_SIZE as u16,
        &render::palette(),
        &frames,
        render::ANIM_DELAY_CS,
    )
    .expect("writing the rendered animation");
    println!(
        "  animated       {gif_path} ({0}x{0}, {1} frames, one full period) — {label}",
        render::ANIM_SIZE,
        render::ANIM_FRAMES
    );
}

fn main() {
    println!("NEOS — boot-to-report pass\n");

    // ---- substrate: boot the hypervisor --------------------------------
    let mut hv = Hypervisor::boot(31, 4096);
    println!("substrate");
    println!("  carrier        {:.6e} rad/s", hv.carrier().get());
    println!(
        "  memory         {} cells x {} bytes = {} total",
        hv.pool().cell_count(),
        hv.pool().cell_capacity(),
        hv.pool().total_capacity()
    );
    let alloc = hv.pool_mut().allocate(8192).expect("8 KiB fits in 31 x 4096");
    println!(
        "  allocation     {} bytes across {} adjacent cells",
        alloc.len(),
        alloc.cells().len()
    );
    for _ in 0..4 {
        hv.tick();
    }
    println!("  uptime         {:.6e} s after {} ticks", hv.uptime_seconds(), hv.ticks());

    // ---- substrate: real fault trapping and recovery --------------------
    //
    // Not guest isolation or privilege levels — nothing in this workspace
    // executes untrusted guest code for those to protect. This is the one
    // piece "virtualisation proper" had real substance to build: a genuine
    // trap dispatch where a fault actually transfers control to a handler
    // that can act on the same pool, and a retried operation can actually
    // succeed — not a closure that merely pretends to help. A separate,
    // small pool, deliberately, so this doesn't disturb `hv`'s own state.
    println!("\nsubstrate: fault trapping");
    let mut trap_hv = Hypervisor::boot(1, 128);
    let filler = trap_hv.pool_mut().allocate(128).unwrap(); // fills the only cell
    let mut freed = false;
    let recovered = trap_hv.allocate_trapped(64, 1, move |fault, pool| {
        println!("  trapped        {fault}");
        if !freed {
            pool.free(&filler);
            freed = true;
            TrapAction::Retry
        } else {
            TrapAction::Propagate
        }
    });
    assert!(
        recovered.is_ok(),
        "the handler's recovery must actually work, not just look like it does"
    );
    println!("  recovered      handler freed the filler; the retried allocation succeeded");

    // ---- symphony-kernel: schedule real tasks --------------------------
    let mut sched = Scheduler::new(16);
    // 64 live tasks at varied frequencies, so admission does not land them
    // perfectly balanced by construction, plus 3 already-decayed ones
    // (frequency at the reclamation threshold) — `schedule` sweeps those
    // before relaxing the field, per `E = C_H*nu`: as `nu -> 0`, `E -> 0`,
    // and the vector is unmapped.
    sched.ingest((0..64u64).map(|i| Task::new(i, 1.0e9 + (i % 7) as f64 * 0.7e9)));
    sched.ingest((64..67u64).map(|i| Task::new(i, 0.0)));
    let alpha = sched.topology().stability_bound() * 0.9;
    let pass = sched
        .schedule(alpha, 20_000)
        .expect("alpha is 0.9x the topology's own stability bound, by construction stable");
    println!("\nsymphony-kernel");
    println!(
        "  tasks          {} live ({} reclaimed this pass)",
        sched.task_count(),
        pass.reclaimed
    );
    println!("  cores          {}", sched.core_count());
    println!(
        "  relaxation     {} steps, spread {:.3e} -> {:.3e}",
        pass.relaxation_steps, pass.spread_before, pass.spread_after
    );
    println!("  migrations     {}", pass.migrations);

    // ---- symphony-kernel: a real deadlock, detected then resolved ------
    //
    // `WaitForGraph`/`ResourceTracker` are kernel-owned *detection* —
    // nothing in the `symphony-kernel` crate ever resolves a cycle once
    // found. That boundary is explicit and repeated throughout this
    // workspace (it's `CLAUDE.md`'s own worked example of "Explicit
    // Contract Separation": detection in kernel, resolution
    // application-level). This section *is* that application level: it
    // builds the exact scenario the contract itself names — two locks
    // taken in opposite order — using only the kernel's already-tested,
    // unmodified public API, and resolves it with a policy that lives
    // entirely here, never inside the kernel crate.
    let mut tracker = ResourceTracker::new();
    let mut wait_graph = WaitForGraph::new();
    let (fork_left, fork_right) = (ResourceId(1), ResourceId(2));
    let (chef_a, chef_b) = (TaskId(200), TaskId(201));

    tracker.acquire(chef_a, fork_left, &mut wait_graph).unwrap(); // a takes the left fork
    tracker.acquire(chef_b, fork_right, &mut wait_graph).unwrap(); // b takes the right fork
    tracker.acquire(chef_a, fork_right, &mut wait_graph).unwrap(); // a wants b's fork
    tracker.acquire(chef_b, fork_left, &mut wait_graph).unwrap(); // b wants a's fork

    println!("\nsymphony-kernel: deadlock");
    let cycle = wait_graph
        .detect_cycle()
        .expect("two tasks, two resources, opposite acquisition order must cycle");
    println!("  detected       cycle {cycle:?}");

    // Policy, stated plainly rather than derived: no law in this workspace
    // prescribes deadlock resolution, so this is one honest, transparent
    // heuristic among many valid ones — lowest TaskId in the cycle is the
    // victim, and everything it currently holds is force-released.
    let victim = *cycle.iter().min_by_key(|t| t.0).unwrap();
    let victim_holds = if victim == chef_a { fork_left } else { fork_right };
    let granted = tracker.release(victim, victim_holds, &mut wait_graph).unwrap();
    println!(
        "  resolved       task {victim:?} force-released {victim_holds:?} (victim policy: lowest TaskId); granted to {granted:?}"
    );

    let cleared = !wait_graph.has_deadlock();
    println!("  confirmed      deadlock cleared: {cleared}");
    assert!(cleared, "resolution must actually break the cycle, not just look like it did");
    // The victim's *own* pending request isn't cancelled by this — it was
    // never part of what deadlocked it, only what it held was. Whichever
    // task didn't get force-released may simply still be waiting, and
    // correctly so: a lone wait with nobody waiting back is not a deadlock.

    // ---- symphony-kernel: the same deadlock, for real -------------------
    //
    // The section above builds the classic two-lock inversion by calling
    // `acquire` in a hand-chosen sequence on one thread — real detection and
    // resolution, but a simulated *contention*: nothing was actually
    // fighting over anything at the same time. `ConcurrentTracker` (real
    // `Mutex` + `Condvar`, not `WaitForGraph`'s own re-derivation) makes the
    // contention real too: two real OS threads, each genuinely blocked on
    // its own `blocking_acquire` call, at the same time, on two different
    // cores — not narrated, not sequenced by this function.
    let real_tracker = ConcurrentTracker::new();
    let (real_left, real_right) = (ResourceId(101), ResourceId(102));
    let (real_a, real_b) = (TaskId(101), TaskId(102));
    let t_a = {
        let tracker = Arc::clone(&real_tracker);
        thread::spawn(move || {
            tracker.blocking_acquire(real_a, real_left).unwrap();
            thread::sleep(Duration::from_millis(20)); // widen the window for the real race
            tracker.blocking_acquire(real_a, real_right).unwrap();
            let _ = tracker.release(real_a, real_right);
            let _ = tracker.release(real_a, real_left);
        })
    };
    let t_b = {
        let tracker = Arc::clone(&real_tracker);
        thread::spawn(move || {
            tracker.blocking_acquire(real_b, real_right).unwrap();
            thread::sleep(Duration::from_millis(20));
            tracker.blocking_acquire(real_b, real_left).unwrap();
            let _ = tracker.release(real_b, real_left);
            let _ = tracker.release(real_b, real_right);
        })
    };

    println!("\nsymphony-kernel: the same deadlock, for real");
    let mut real_cycle = None;
    for _ in 0..500 {
        if let Some(c) = real_tracker.detect_cycle() {
            real_cycle = Some(c);
            break;
        }
        thread::sleep(Duration::from_millis(10));
    }
    let real_cycle = real_cycle.expect("two real threads in opposite acquisition order must cycle");
    println!("  detected       cycle {real_cycle:?} — two real OS threads, genuinely blocked");

    let real_victim = *real_cycle.iter().min_by_key(|t| t.0).unwrap();
    let real_released = real_tracker.force_release_all(real_victim);
    println!(
        "  resolved       task {real_victim:?} force-released {real_released:?} (same victim policy: lowest TaskId)"
    );

    t_a.join().expect("thread a must finish once the deadlock actually clears");
    t_b.join().expect("thread b must finish once the deadlock actually clears");
    println!(
        "  confirmed      both real threads finished; deadlock cleared: {}",
        !real_tracker.has_deadlock()
    );

    // ---- symphony-kernel: real concurrent allocation --------------------
    //
    // `substrate::MemoryPool` is deliberately single-threaded; its own
    // implementation log left synchronisation as "a scheduler decision, not
    // a substrate one." `ConcurrentPool` is that decision, made here in
    // `symphony-kernel` since it's the crate that already coordinates
    // concurrent access to cores and tasks. This spins up real OS threads —
    // not a simulation — sharing one pool, each writing and reading back
    // its own fingerprint, exactly the workload that caught a genuine,
    // pre-existing `MemoryPool::free` defect while this was being verified
    // (fixed in `substrate`, entirely unrelated to threading in the end).
    let concurrent_pool = ConcurrentPool::new(4, 256);
    let thread_count = 8usize;
    let iters_per_thread = 100usize;
    let handles: Vec<_> = (0..thread_count)
        .map(|tid| {
            let pool = Arc::clone(&concurrent_pool);
            let fingerprint = (tid as u8).wrapping_add(1);
            thread::spawn(move || {
                let mut completed = 0usize;
                for _ in 0..iters_per_thread {
                    if let Ok(alloc) = pool.allocate(64) {
                        pool.write(alloc.start(), &[fingerprint; 64]).unwrap();
                        thread::yield_now();
                        let back = pool.read(alloc.start(), 64).unwrap();
                        pool.free(&alloc);
                        assert_eq!(back, vec![fingerprint; 64], "thread {tid} was corrupted");
                        completed += 1;
                    }
                }
                completed
            })
        })
        .collect();
    let total_completed: usize = handles.into_iter().map(|h| h.join().unwrap()).sum();
    println!("\nsymphony-kernel: concurrent allocation");
    println!(
        "  threads        {thread_count} real OS threads, {total_completed} allocate/write/read/free cycles completed"
    );
    println!(
        "  confirmed      pool consistent after: available {} / {} bytes",
        concurrent_pool.available(),
        concurrent_pool.total_capacity()
    );
    assert_eq!(
        concurrent_pool.available(),
        concurrent_pool.total_capacity(),
        "every allocation was freed; nothing should remain marked used"
    );

    // ---- symphony-lang: the instruction-executing state machine ---------
    //
    // `_mkb/instruction_set.md`. A batch of three real programs shares this
    // hypervisor's own `MemoryPool` (at cells well clear of the 8192-byte
    // allocation above) and a fresh resource tracker: program 0 stores a
    // real task's physical state into curved memory, program 1 is built to
    // fault (a `load` from a cell nothing ever `store`d into), and program 2
    // loads program 0's state back and emits it, acquiring and releasing a
    // real resource along the way. Program 1's trap must not stop program 2
    // from running cleanly against the same, shared pool — the actual
    // "traps out the running thread, leaves the rest of the lattice running
    // cleanly" claim, demonstrated live rather than only in the test suite.
    let program_a = lang_compile(
        &lang_parse(
            &lang_lex("task probe at 660 hz phase + scale 1.25\nstore probe at cell 10\nemit probe")
                .unwrap(),
        )
        .unwrap(),
    );
    let program_b_faults =
        lang_compile(&lang_parse(&lang_lex("load ghost at cell 20").unwrap()).unwrap());
    let program_c = lang_compile(
        &lang_parse(&lang_lex("load probe at cell 10\nacquire 42\nemit probe\nrelease 42").unwrap())
            .unwrap(),
    );

    let mut lang_tracker = ResourceTracker::new();
    let mut lang_graph = WaitForGraph::new();
    let mut lang_vm = LangVm::new(hv.pool_mut(), &mut lang_tracker, &mut lang_graph);
    let batch = lang_vm.run_batch(&[program_a, program_b_faults, program_c]);

    println!("\nsymphony-lang: the instruction-executing state machine");
    for (i, outcome) in batch.iter().enumerate() {
        match &outcome.trap {
            None => println!(
                "  program {i}      halted cleanly, {} emitted, {} store(s), {} load(s)",
                outcome.emitted.len(),
                outcome.stores.len(),
                outcome.loads.len()
            ),
            Some(fault) => println!("  program {i}      trapped: {fault:?}"),
        }
    }
    assert!(batch[0].trap.is_none(), "program 0 must store cleanly");
    assert!(batch[1].trap.is_some(), "program 1 is built to fault");
    assert!(
        batch[2].trap.is_none() && batch[2].emitted.len() == 1,
        "program 2 must run cleanly against the same pool program 1 faulted against"
    );
    println!(
        "  isolation      confirmed: program 1's trap did not stop program 2 loading program 0's real stored state"
    );

    // ---- symphony-lang: dynamic fault routing (Vm::run_program_trapped) -
    //
    // The direct counterpart to `Hypervisor::allocate_trapped` above, for
    // `store`/`load`'s own memory faults rather than `allocate`'s. This
    // program's `load` faults on a cell nothing ever wrote to — the same
    // `CorruptState` fault program 1 hit above — but this time a real
    // handler is watching: it seeds the exact cell the fault names with real
    // encoded task state and asks for a retry. The retried `load`
    // instruction (not the whole program) then actually succeeds.
    let recoverable = lang_compile(&lang_parse(&lang_lex("load rescued at cell 25").unwrap()).unwrap());
    let mut recover_tracker = ResourceTracker::new();
    let mut recover_graph = WaitForGraph::new();
    let mut recover_vm = LangVm::new(hv.pool_mut(), &mut recover_tracker, &mut recover_graph);
    let mut handler_calls = 0usize;
    let recovered_outcome =
        recover_vm.run_program_trapped(TaskId(999), LangDomain::Kernel, &recoverable, 1, |fault, pool| {
            handler_calls += 1;
            let addr = pool.address_at(25).expect("cell 25 exists in a 31-cell pool");
            let mut bytes = Vec::with_capacity(24);
            bytes.extend_from_slice(&523.25_f64.to_le_bytes()); // C5, a real pitch
            bytes.extend_from_slice(&Phase::Positive.radians().to_le_bytes());
            bytes.extend_from_slice(&1.0_f64.to_le_bytes());
            pool.write(addr, &bytes).expect("cell 25 is untouched by every earlier demo section");
            println!("  trapped        {fault:?}");
            LangTrapAction::Retry
        });
    println!("\nsymphony-lang: dynamic fault routing");
    assert_eq!(handler_calls, 1, "the handler must run exactly once, on the first fault");
    assert!(
        recovered_outcome.trap.is_none(),
        "the retried load must succeed against the handler's corrected state"
    );
    println!(
        "  recovered      handler seeded cell 25; the retried load found `rescued` at {} hz",
        recovered_outcome.declared["rescued"].frequency()
    );

    // ---- symphony-lang: privilege domains (Vm::reserve_cells) -----------
    //
    // `_mkb/instruction_set.md` records this as a **stated engineering
    // convention, not law** — no axiom or PRD section defines privilege or
    // guest/kernel separation, unlike everything else this report
    // demonstrates. Protects the two real cells this hypervisor's own
    // 8192-byte allocation lives in (cell 0-1, from this report's very
    // first section) — genuinely system-critical, not a cell chosen only
    // for this demo.
    let protected: Vec<_> = alloc.cells().to_vec();
    let mut guard_tracker = ResourceTracker::new();
    let mut guard_graph = WaitForGraph::new();
    let mut guard_vm = LangVm::new(hv.pool_mut(), &mut guard_tracker, &mut guard_graph);
    guard_vm.reserve_cells(protected.iter().copied());

    let intruder =
        lang_compile(&lang_parse(&lang_lex("task x at 1 hz phase +\nstore x at cell 0").unwrap()).unwrap());
    let guest_outcome =
        guard_vm.run_program_trapped(TaskId(1000), LangDomain::Guest, &intruder, 0, |_, _| {
            LangTrapAction::Propagate
        });
    println!("\nsymphony-lang: privilege domains");
    match &guest_outcome.trap {
        Some(fault) => println!("  refused        Domain::Guest denied: {fault:?}"),
        None => panic!("a guest program must not be able to touch reserved cells"),
    }

    // ---- symphony-lang: real concurrency (symphony_lang::concurrent) ----
    //
    // `Vm::run_batch` is sequential — one program to completion at a time,
    // a stated real limit `_mkb/instruction_set.md` names outright.
    // `concurrent::run_batch_concurrent` is the real scheduler that limit
    // was missing: real OS threads sharing one real `ConcurrentPool`. Eight
    // threads each `acquire` the same resource, `store` their own distinct
    // frequency into the *same* shared cell, and `load` it straight back —
    // if the lock did not really exclude the others, at least one thread
    // would read back someone else's value instead of its own.
    let conc_pool = ConcurrentPool::new(4, 64);
    let conc_tracker = ConcurrentTracker::new();
    let conc_reserved: Arc<HashSet<lattice::tessellation::CellId>> = Arc::new(HashSet::new());
    let conc_programs: Vec<_> = (0..8u64)
        .map(|i| {
            let freq = 300.0 + i as f64;
            let source = format!(
                "task me at {freq} hz phase +\nacquire 7\nstore me at cell 1\nload echo at cell 1\nrelease 7"
            );
            let instructions = lang_compile(&lang_parse(&lang_lex(&source).unwrap()).unwrap());
            (TaskId(2000 + i), LangDomain::Kernel, instructions)
        })
        .collect();
    let conc_outcomes =
        run_batch_concurrent(&conc_pool, &conc_tracker, &conc_reserved, conc_programs);
    let all_consistent = conc_outcomes.iter().all(|o| {
        o.trap.is_none() && o.declared["echo"].frequency() == o.declared["me"].frequency()
    });
    println!("\nsymphony-lang: real concurrency");
    println!(
        "  contended      8 real OS threads, one shared cell, real acquire/release around each store+load"
    );
    println!(
        "  confirmed      every thread read back its own value: {all_consistent}"
    );
    assert!(
        all_consistent,
        "real mutual exclusion must prevent any thread's store/load from interleaving with another's"
    );

    // ---- symphony-lang: a genuine multi-tenant sandbox -------------------
    //
    // `Domain::Guest` alone only says "trusted or not" — every guest shares
    // the same restricted region. `Sandbox` adds a per-tenant ownership map
    // over `run_batch_concurrent`'s same real threads, so *which* memory is
    // off-limits differs per tenant: three mutually untrusted programs run
    // **at the same time**, each provably able to use only its own admitted
    // cells, and each provably refused when it reaches for another tenant's.
    let sandbox = Sandbox::new(8, 64);
    let tenants = 3usize;
    for t in 0..tenants {
        let cells = [
            sandbox.pool().address_at(2 * t).unwrap().cell(),
            sandbox.pool().address_at(2 * t + 1).unwrap().cell(),
        ];
        sandbox
            .admit_tenant(TaskId(3000 + t as u64), cells)
            .expect("each tenant is admitted to memory nobody else holds yet");
    }
    let sandbox_programs: Vec<_> = (0..tenants)
        .map(|t| {
            let own = 2 * t;
            let other = 2 * ((t + 1) % tenants);
            let freq = 400.0 + t as f64;
            let source = format!(
                "task mine at {freq} hz phase +\nstore mine at cell {own}\nload back at cell {own}\nstore mine at cell {other}"
            );
            let instructions = lang_compile(&lang_parse(&lang_lex(&source).unwrap()).unwrap());
            (TaskId(3000 + t as u64), instructions)
        })
        .collect();
    let sandbox_outcomes = sandbox.run_many(sandbox_programs);
    let each_kept_its_own = sandbox_outcomes
        .iter()
        .enumerate()
        .all(|(t, o)| o.declared["back"].frequency() == 400.0 + t as f64);
    let each_refused_from_the_others =
        sandbox_outcomes.iter().all(|o| {
            matches!(o.trap, Some(symphony_lang::VmFault::PrivilegeViolation { .. }))
        });
    println!("\nsymphony-lang: a genuine multi-tenant sandbox");
    println!(
        "  admitted       {tenants} tenants, 2 cells each, running concurrently on real threads"
    );
    println!("  own memory     each tenant read back its own store: {each_kept_its_own}");
    println!(
        "  isolation      each tenant refused from every other tenant's memory: {each_refused_from_the_others}"
    );
    assert!(each_kept_its_own, "a tenant must be able to use its own admitted memory");
    assert!(
        each_refused_from_the_others,
        "a tenant must never be able to touch another tenant's admitted memory"
    );

    // ---- ftg: deliver a real frame over a resonant session --------------
    //
    // `Gateway::deliver` (ungated) is the easy path; a real connection is a
    // shared standing wave, not a state record — `deliver_over` requires one
    // and re-checks it at every hop, not just at admission.
    let gateway = Gateway::new(4);
    let cells = gateway.router().cells();
    let (src, dst) = (cells[0], cells[cells.len() - 1]);
    let packet = Packet::new(Frame::encode(b"NEOS is running"), src, dst);

    let link = Link::attempt_handshake(Oscillator::new(0.0, 1.0), Oscillator::new(0.1, 1.0))
        .expect("phase variance 0.1 is well inside the lock bound");
    println!("\nftg");
    println!(
        "  session        {:?}, phase variance {:.4}",
        link.state(),
        link.phase_variance()
    );
    let delivery = gateway
        .deliver_over(&link, &packet, 200)
        .expect("a resonant session over the documented, fully connected patch delivers");
    match &delivery {
        Delivery::Arrived { payload, hops, .. } => println!(
            "  delivered      {:?} in {} hops, over the session",
            String::from_utf8_lossy(payload),
            hops
        ),
        Delivery::Dissipated { hop, amplitude, .. } => {
            println!("  dissipated     at hop {hop}, amplitude {amplitude}")
        }
        Delivery::LinkLost { hop, .. } => println!("  link lost      at hop {hop}"),
    }

    // ---- crystallisation: image, audio, and video, crystallised concurrently
    //
    // Three genuinely independent pipelines — nothing here shares state, so
    // there is nothing to synchronise, only real throughput to gain. See
    // `crystallisation::parallel`'s own module docs: every pipeline in that
    // crate is a pure function over owned data, which is what makes this
    // safe with zero locks. `crystallize_images`/`embed_audio`/
    // `crystallize_videos` handle the general batch case (many of one media
    // type); this demo's own three *different* media types are spawned
    // directly, one real OS thread each.
    let image_thread = thread::spawn(|| {
        let grid = crystallisation::decode_ppm(&embedded_ppm()).expect("well-formed embedded PPM");
        let faces = FrequencyMap::transform(&grid)
            .project_onto_faces()
            .expect("16 coefficients divide evenly across 4 faces");
        (grid, faces)
    });
    let audio_thread = thread::spawn(|| {
        let audio = crystallisation::decode_wav(&embedded_wav()).expect("well-formed embedded WAV");
        let audio_nodes = takens_embed(audio.samples(), 3).expect("64 samples embed at tau=3");
        (audio, audio_nodes)
    });
    let video_thread = thread::spawn(|| {
        let video_frames = embedded_video_frames();
        let frame_count = video_frames.len();
        let vtc = VolumetricTimeCrystal::crystallise_video(video_frames, 30.0, 3)
            .expect("rescaled embedded frames fit the quantisable ceiling");
        (frame_count, vtc)
    });

    let (grid, faces) = image_thread
        .join()
        .expect("the image crystallization thread must not panic");
    let (audio, audio_nodes) = audio_thread
        .join()
        .expect("the audio crystallization thread must not panic");
    let (frame_count, vtc) = video_thread
        .join()
        .expect("the video crystallization thread must not panic");

    println!("\ncrystallisation (image, audio, video — three real threads, not sequential)");
    println!(
        "  decoded        {}x{} image, {} bytes -> {} face(s)",
        grid.width(),
        grid.height(),
        grid.width() * grid.height(),
        faces.len()
    );
    println!(
        "  decoded        {} audio samples @ {} Hz -> {} phase-space node(s) (Takens tau=3)",
        audio.samples().len(),
        audio.sample_rate(),
        audio_nodes.len()
    );
    println!(
        "  crystallised   {} video frames -> {} phase-space node(s), energy {:.4e} J, conserving: {}",
        frame_count,
        vtc.nodes().len(),
        vtc.input_energy(),
        vtc.is_energy_conserving()
    );

    // ---- gui: read the running state into standing-wave amplitudes -----
    let mut snapshot = SystemSnapshot::from_scheduler(&sched);
    snapshot.read_memory(hv.pool());
    snapshot.observe(&delivery);

    let load = LoadVisualisation::from_snapshot(&snapshot, WAVE_K, WAVE_OMEGA);
    let tetryen = TetryenVisualisation::from_face_projections(&faces, WAVE_K, WAVE_OMEGA);
    println!("\ngui");
    println!(
        "  memory         {:.1}% used ({} / {} bytes)",
        snapshot.memory_utilisation() * 100.0,
        snapshot.memory_used(),
        snapshot.memory_total()
    );
    println!(
        "  load field     peak {:.4}, imbalance {:.4}",
        load.peak(),
        snapshot.imbalance()
    );
    println!(
        "  tetryen        peak node amplitude {:.4} (t = 0)",
        tetryen.peak()
    );
    let audio_vis = busiest_visualisation(&audio_nodes, WAVE_K, WAVE_OMEGA);
    let video_vis = busiest_visualisation(vtc.nodes(), WAVE_K, WAVE_OMEGA);
    println!(
        "  audio tetryen  peak node amplitude {:.4} (busiest embedded moment)",
        audio_vis.peak()
    );
    println!(
        "  video tetryen  peak node amplitude {:.4} (busiest embedded moment)",
        video_vis.peak()
    );

    // Larger than the 0.5 every test uses — nothing here is asserted against
    // that value, and a bigger shape reads better on a 640x640 canvas.
    let geometry = Tetryen::new(2.5).expect("2.5 is well inside the valid circumradius domain");

    // ---- gui: the Tetryen recurrence, evolving in real discrete time ----
    //
    // `_mkb/tetryen_recurrence.md` — a synthesis closing the undistilled
    // corpus's f(psi_n, psi_{n-1}) placeholder. Seeded from the image's own
    // real per-node amplitudes (not a hand-built state), then stepped
    // forward through real geometry-weighted coupling (node_amplitude at
    // the real geodesic distance between nodes), staying inside the
    // recurrence's own documented, measured-safe region.
    let seed: [f64; 4] = std::array::from_fn(|i| tetryen.wave(i).map_or(0.0, |w| w.amplitude()));
    let mut recurrence = gui::TetryenState::at_rest(seed);
    let recurrence_steps = 200;
    let mut diverged_at = None;
    for step in 0..recurrence_steps {
        if let Err(e) = recurrence.step(&geometry, WAVE_OMEGA, 0.01, 1.0) {
            diverged_at = Some((step, e));
            break;
        }
    }
    println!("\ngui: Tetryen recurrence");
    println!(
        "  seed           [{:.4}, {:.4}, {:.4}, {:.4}]",
        seed[0], seed[1], seed[2], seed[3]
    );
    match diverged_at {
        None => {
            let a = recurrence.amplitudes();
            println!(
                "  evolved        {recurrence_steps} steps -> [{:.4}, {:.4}, {:.4}, {:.4}]",
                a[0], a[1], a[2], a[3]
            );
        }
        Some((step, e)) => println!("  diverged       at step {step}: {e}"),
    }
    assert!(
        diverged_at.is_none(),
        "documented-safe parameters (gamma=1, dt=0.01) must not diverge"
    );

    // ---- crystallisation: the same recurrence, driven by real quantised
    // data ---------------------------------------------------------------
    //
    // `crystallisation` has no Tetryen geometry of its own — the coupling
    // weight is computed here from the same real geometry the panel above
    // already built, via the one shared law function both crates now call,
    // `lattice::tetryen_node_envelope`. The driving frequency is real too:
    // the video crystal's own Howard-Comma-derived fundamental, not an
    // arbitrary constant.
    let separation = geometry.nodes()[0].distance_to(&geometry.nodes()[1]);
    let weight = lattice::tetryen_node_envelope(separation);
    let crystal_seed = *vtc
        .nodes()
        .first()
        .expect("crystallise_video on real frames embeds at least one node");
    let mut crystal_recurrence = crystallisation::TetryenRecurrence::at_rest(crystal_seed);
    let crystal_steps = 200;
    let mut crystal_diverged_at = None;
    for step in 0..crystal_steps {
        if let Err(e) = crystal_recurrence.step(&vtc, 1e-5, 1.0, weight) {
            crystal_diverged_at = Some((step, e));
            break;
        }
    }
    println!("\ncrystallisation: Tetryen recurrence");
    println!("  seed           {:?}", crystal_seed.components());
    println!(
        "  driven by      real fundamental {:.4} Hz (Howard-Comma quantised)",
        vtc.fundamental().get()
    );
    match crystal_diverged_at {
        None => println!(
            "  evolved        {crystal_steps} steps -> {:?}",
            crystal_recurrence.state().components()
        ),
        Some((step, e)) => println!("  diverged       at step {step}: {e}"),
    }
    assert!(
        crystal_diverged_at.is_none(),
        "documented-safe parameters (gamma=1, dt=1e-5) must not diverge"
    );

    // The report above only ever states amplitude at t = 0. A standing wave
    // is a function of time (`StandingWave::at`) — each render below samples
    // a full period of its own real waves and writes the motion out as a
    // GIF too, rather than leaving "standing wave" a word the numbers don't
    // show. Three panels, one per media source that actually drove a
    // TetryenVisualisation this run.
    render_and_report("image", "render", &geometry, &tetryen, &load);
    render_and_report("audio", "render_audio", &geometry, &audio_vis, &load);
    render_and_report("video", "render_video", &geometry, &video_vis, &load);

    // ---- export: the same real data, as JSON -----------------------------
    //
    // Everything the interactive viewer draws is a second read of exactly
    // these already-computed values — not a reconstruction of them in a
    // second language. See `export.rs`.
    export::write_json_report(
        "report.json",
        &geometry,
        &[("image", &tetryen), ("audio", &audio_vis), ("video", &video_vis)],
        &load,
        snapshot.imbalance(),
        WAVE_K,
        WAVE_OMEGA,
    )
    .expect("writing the JSON report");
    println!("\nexport           report.json (real geometry + amplitudes, for the interactive viewer)");
}

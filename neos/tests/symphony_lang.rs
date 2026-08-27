//! Physics assertions for `symphony-lang`.
//!
//! Per `_mkb/test-doctrine.md`, the governing question for each test is: *would
//! a conventional implementation still pass?* For a DSL that question has a
//! sharp form — **would this test pass against a language with `if` and
//! `bool`?** If yes, it is testing a parser, not NEOS.

use std::collections::HashSet;
use std::sync::Arc;
use substrate::{MemoryPool, SubstrateError};
use symphony_kernel::bifurcation::TaskModel;
use symphony_kernel::{
    detuning, ConcurrentPool, ConcurrentTracker, Interference, Phase, ResourceTracker, Scheduler,
    TaskId, WaitForGraph,
};
use symphony_lang::concurrent::run_batch_concurrent;
use symphony_lang::vm::{compile, Instruction, ResourceOutcome, Vm, VmFault};
use symphony_lang::{
    execute, execute_with, lex, parse, run, Address, Alignment, Domain, LangError, RuntimeTask,
    Stmt, TrapAction, REFERENCE_SCALE,
};

// ---------------------------------------------------------------- A2: syntax

/// The central assertion of this record: Boolean logic is **unlexable**.
///
/// A conventional language passes this only by not existing.
#[test]
fn boolean_constructs_are_refused_at_lex_time() {
    // Every construct that would reintroduce classical logic.
    let cases = [
        "task a at 1 hz phase +\nemit true",
        "task a at 1 hz phase +\nemit false",
        "if a { emit a }",
        "else { emit a }",
        "when a aligns b && c { }",
        "when a aligns b || c { }",
        "emit !a",
        "when a == b { }",
        "when a != b { }",
        "task bool at 1 hz phase +",
        "when a and b { }",
        "when a or b { }",
        "emit not a",
    ];

    for source in cases {
        match lex(source) {
            Err(LangError::ForbiddenBooleanConstruct { construct, .. }) => {
                assert!(
                    !construct.is_empty(),
                    "the refusal must name the construct it refused"
                );
            }
            other => panic!("expected an A2 refusal for {source:?}, got {other:?}"),
        }
    }
}

/// The refusal is not a blanket ban on the letters — it names the axiom and
/// says what to write instead. An error that just said "syntax error" would be
/// indistinguishable from a parser that had never heard of A2.
#[test]
fn the_a2_refusal_explains_itself() {
    let err = lex("if a { }").unwrap_err();
    let message = err.to_string();
    assert!(message.contains("A2"), "message must cite the axiom: {message}");
    assert!(
        message.contains("when") && message.contains("aligns"),
        "message must point at the replacement construct: {message}"
    );
    assert_eq!(
        format!("{}", lex("emit true").unwrap_err()).contains("A2"),
        true
    );
}

/// A2 admits exactly two phase orientations. There is no third literal, and no
/// literal that stands for "unknown" — the type has two inhabitants and the
/// grammar has two spellings.
#[test]
fn only_two_phase_literals_exist() {
    let ok = run("task a at 1 hz phase +\ntask b at 1 hz phase -").unwrap();
    assert_eq!(ok.declared["a"].guard_phase(), Phase::Positive);
    assert_eq!(ok.declared["b"].guard_phase(), Phase::Negative);

    // Anything else in that slot is not a phase.
    for bad in ["task a at 1 hz phase 0", "task a at 1 hz phase maybe"] {
        assert!(
            run(bad).is_err(),
            "{bad:?} named a third phase and was accepted"
        );
    }
}

/// The lexer refuses Boolean punctuation *before* the parser ever sees the
/// program, so a syntactically broken program still fails for the A2 reason.
///
/// Order matters: if the parser reported first, a programmer would fix the
/// syntax and only then learn the construct was forbidden.
#[test]
fn a2_refusal_precedes_every_other_error() {
    // Undeclared task AND a forbidden construct in the same source.
    let err = run("when undeclared_thing == other { }").unwrap_err();
    assert!(
        matches!(err, LangError::ForbiddenBooleanConstruct { .. }),
        "A2 must fire before name resolution, got {err:?}"
    );
}

// ------------------------------------------------------- A2: branch semantics

/// Branching is phase interference. The kernel decides; the language only asks.
#[test]
fn a_branch_is_taken_by_interference_not_by_truth() {
    let source = "\
task carrier at 440 hz phase +
task guard   at 220 hz phase +
task veto    at 110 hz phase -

when carrier aligns guard {
    emit guard
}
when carrier aligns veto {
    emit veto
}
";
    let exec = run(source).unwrap();

    assert_eq!(exec.branches.len(), 2);
    assert_eq!(exec.branches[0].interference, Interference::Constructive);
    assert!(exec.branches[0].taken);
    assert_eq!(exec.branches[1].interference, Interference::Destructive);
    assert!(!exec.branches[1].taken);

    // Only the constructive body emitted.
    assert_eq!(exec.emitted.len(), 1);
}

/// Per operand pair, `opposes` **is** the complement of `aligns`. Stated
/// outright rather than left implicit.
///
/// A2 admits exactly two phase orientations, so alignment is a two-valued
/// predicate and its outcomes partition. Sabotage established this the hard
/// way: rewriting the interpreter's pairing as `!aligns_taken` broke no test,
/// because the two are the same function. Asserting a distinction here would
/// have been asserting something false.
///
/// What separates this language from `if`/`else` is structural, not per-branch
/// — see `branch_forms_are_independent_statements`.
#[test]
fn alignment_partitions_the_two_phase_orientations() {
    for (pa, pb) in [("+", "+"), ("+", "-"), ("-", "+"), ("-", "-")] {
        let aligns = run(&format!(
            "task a at 1 hz phase {pa}\ntask b at 1 hz phase {pb}\nwhen a aligns b {{ emit a }}"
        ))
        .unwrap();
        let opposes = run(&format!(
            "task a at 1 hz phase {pa}\ntask b at 1 hz phase {pb}\nwhen a opposes b {{ emit a }}"
        ))
        .unwrap();
        assert_eq!(
            aligns.branches[0].taken,
            !opposes.branches[0].taken,
            "the two branch forms must partition, at ({pa}, {pb})"
        );
    }
}

/// `opposes` is not `else`. The two forms are **independent statements** over
/// independently chosen operand pairs, so a program can take both, or neither.
///
/// A language with `if`/`else` cannot express either outcome: exactly one arm
/// runs, always. This — not per-branch complementarity — is the real difference.
#[test]
fn branch_forms_are_independent_statements() {
    let source = "\
task a at 100 hz phase +
task b at 100 hz phase +
task c at 100 hz phase -

when a aligns b { emit a }
when a opposes c { emit c }
";
    let exec = run(source).unwrap();

    // Both bodies ran. Under if/else exactly one could have.
    assert!(exec.branches.iter().all(|b| b.taken));
    assert_eq!(exec.emitted.len(), 2);

    // And the mirrored program takes neither.
    let neither = run("\
task a at 100 hz phase +
task b at 100 hz phase -
task c at 100 hz phase +

when a aligns b { emit a }
when a opposes c { emit c }
")
    .unwrap();
    assert!(neither.branches.iter().all(|b| !b.taken));
    assert_eq!(neither.emitted.len(), 0);
}

/// Interference is symmetric: swapping the operands cannot change the outcome.
/// Superposition does not care which wave you named first.
#[test]
fn interference_is_symmetric_in_its_operands() {
    for (pa, pb) in [("+", "+"), ("+", "-"), ("-", "+"), ("-", "-")] {
        let forward = run(&format!(
            "task a at 1 hz phase {pa}\ntask b at 1 hz phase {pb}\nwhen a aligns b {{ emit a }}"
        ))
        .unwrap();
        let reverse = run(&format!(
            "task a at 1 hz phase {pa}\ntask b at 1 hz phase {pb}\nwhen b aligns a {{ emit b }}"
        ))
        .unwrap();
        assert_eq!(
            forward.branches[0].interference, reverse.branches[0].interference,
            "interference changed when the operands were swapped ({pa}, {pb})"
        );
        assert_eq!(forward.emitted.len(), reverse.emitted.len());
    }
}

/// Nested branches compose: an inner body only runs if the outer one did.
#[test]
fn nested_branches_gate_on_the_outer_interference() {
    let taken = run("\
task a at 1 hz phase +
task b at 1 hz phase +
when a aligns b {
    when b aligns a { emit a }
}
")
    .unwrap();
    assert_eq!(taken.emitted.len(), 1);
    assert_eq!(taken.branches.len(), 2, "both branches were evaluated");

    let blocked = run("\
task a at 1 hz phase +
task b at 1 hz phase -
when a aligns b {
    when b aligns b { emit a }
}
")
    .unwrap();
    assert_eq!(blocked.emitted.len(), 0);
    assert_eq!(
        blocked.branches.len(),
        1,
        "the inner branch must not even be evaluated when the outer cancels"
    );
}

// ------------------------------------------------------------------- A1: fork

/// A1: `1 (x) 1 = 2`. The language never computes this — the kernel does.
#[test]
fn fork_yields_exactly_two_children() {
    let exec = run("task a at 50 hz phase +\nfork a").unwrap();

    assert_eq!(exec.forks.len(), 1);
    assert_eq!(
        exec.forks[0].bifurcation.children, 2.0,
        "A1 is bit-exact at the unit fork; anything else means (x) was reimplemented"
    );
    assert_eq!(exec.forks[0].bifurcation.address_scale, 2.0);
    assert_eq!(exec.emitted.len(), 2, "both children must be schedulable");
}

/// Forking is a *structural split*, not scalar duplication: the address space
/// scales by the same modified product as the child count.
#[test]
fn fork_scales_address_space_with_child_count() {
    let exec = run("task a at 50 hz phase +\nfork a").unwrap();
    let b = exec.forks[0].bifurcation;
    assert_eq!(b.address_scale, b.children);
}

/// Repeated forks compound. Three forks give six children, not four — each
/// fork is an independent structural split of the same declared oscillator.
#[test]
fn forks_accumulate_across_statements() {
    let exec = run("task a at 50 hz phase +\nfork a\nfork a\nfork a").unwrap();
    assert_eq!(exec.forks.len(), 3);
    assert_eq!(exec.emitted.len(), 6);
}

/// A fork inside an uncancelled branch runs; inside a cancelled one it does not.
/// This is the join of A1 and A2 in one program.
#[test]
fn a_cancelled_branch_forks_nothing() {
    let exec = run("\
task a at 1 hz phase +
task b at 1 hz phase -
when a aligns b { fork a }
")
    .unwrap();
    assert!(exec.forks.is_empty());
    assert!(exec.emitted.is_empty());
}

/// A1's arithmetic is exact only at the canonical unit. `2 (x) 2 = 20.97` — a
/// fractional child count, and a fraction of an execution unit does not exist.
/// Refused, never rounded.
///
/// Reached through the embedding seam, since surface syntax always forks at 1.
#[test]
fn a_fractional_child_count_is_refused() {
    let program = parse(&lex("fork a").unwrap()).unwrap();
    let odd = RuntimeTask::new("a", 1.0, Phase::Positive)
        .unwrap()
        .with_fork_unit(2.0);

    match execute_with(&program, [odd]) {
        Err(LangError::NonIntegralFork { children, .. }) => {
            assert!(
                children.fract() != 0.0 && children > 2.0,
                "expected a fractional count above the unit fork, got {children}"
            );
        }
        other => panic!("a fractional fork was not refused: {other:?}"),
    }
}

/// The (x) domain ceiling reaches this subsystem too — a fourth place, at a
/// fourth arity. A fork unit beyond it is refused as a language error, not a
/// panic and not a silently clamped value.
#[test]
fn a_fork_outside_the_otimes_domain_is_refused() {
    let program = parse(&lex("fork a").unwrap()).unwrap();
    let huge = RuntimeTask::new("a", 1.0, Phase::Positive)
        .unwrap()
        .with_fork_unit(1e6);

    assert!(
        matches!(
            execute_with(&program, [huge]),
            Err(LangError::ForkOutsideDomain { .. })
        ),
        "1e6 (x) 1e6 is outside the domain and must be refused"
    );
}

/// Seeding adds to the environment; it does not bypass scoping. A source
/// declaration colliding with a seeded name is still a redeclaration.
#[test]
fn seeding_does_not_open_a_hole_in_scoping() {
    let program = parse(&lex("task a at 5 hz phase +\nemit a").unwrap()).unwrap();
    let seeded = RuntimeTask::new("a", 1.0, Phase::Positive).unwrap();

    assert!(matches!(
        execute_with(&program, [seeded]),
        Err(LangError::DuplicateTask { .. })
    ));
}

// ------------------------------------------------- the TaskModel seam closes

/// `RuntimeTask` is the first implementor of the kernel's `TaskModel`. The seam
/// the kernel has carried open since it was written is now closed.
#[test]
fn runtime_task_implements_the_kernel_seam() {
    fn takes_a_model<T: TaskModel>(t: &T) -> (f64, Phase, f64) {
        (t.frequency(), t.guard_phase(), t.fork_unit())
    }

    let exec = run("task voice at 432 hz phase -").unwrap();
    let (nu, phase, unit) = takes_a_model(&exec.declared["voice"]);

    assert_eq!(nu, 432.0);
    assert_eq!(phase, Phase::Negative);
    assert_eq!(unit, 1.0, "the default fork unit is A1's canonical 1");
}

/// Emitted tasks are the kernel's own `Task`, so they schedule without
/// translation. The language hands work to the field; it does not run work.
#[test]
fn emitted_tasks_drive_the_real_scheduler() {
    let exec = run("\
task low  at 100 hz phase +
task high at 900 hz phase +
emit low
emit high
fork high
")
    .unwrap();
    assert_eq!(exec.emitted.len(), 4);

    let mut scheduler = Scheduler::new(8);
    scheduler.ingest(exec.emitted.iter().copied());
    assert_eq!(scheduler.task_count(), 4);

    let alpha = scheduler.topology().stability_bound() / 2.0;
    let pass = scheduler.schedule(alpha, 500).unwrap();
    assert!(
        pass.spread_after <= pass.spread_before,
        "ingesting a program must not destabilise the field: {pass:?}"
    );
}

/// Energy is priced by `E = C_H*nu`, and the language does not recompute it.
/// A doubled frequency is a doubled cost — linear, per the Howard Comma.
#[test]
fn program_energy_is_linear_in_declared_frequency() {
    let single = run("task a at 100 hz phase +\nemit a").unwrap();
    let double = run("task a at 200 hz phase +\nemit a").unwrap();

    let e1 = single.total_energy_joules();
    let e2 = double.total_energy_joules();

    assert!(e1 > 0.0, "a declared oscillator must carry energy");
    // Relative, not absolute: these are of order 1e-32 J.
    assert!(
        ((e2 / e1) - 2.0).abs() < 1e-12,
        "E = C_H*nu must be linear; got ratio {}",
        e2 / e1
    );
}

/// Two programs emitting the same oscillators cost the same, whatever route
/// through the branch structure produced them. Energy is a property of the
/// waves, not of the control flow that selected them.
#[test]
fn energy_is_independent_of_the_path_that_emitted_it() {
    let direct = run("task a at 300 hz phase +\nemit a\nemit a").unwrap();
    let branched = run("\
task a at 300 hz phase +
task k at 1 hz phase +
when a aligns k { emit a }
when k aligns a { emit a }
")
    .unwrap();

    assert_eq!(direct.emitted.len(), branched.emitted.len());
    assert_eq!(
        direct.total_energy_joules(),
        branched.total_energy_joules(),
        "control flow changed the energy of the same two oscillators"
    );
}

// ------------------------------------------------------------ well-formedness

#[test]
fn an_undeclared_task_is_refused() {
    assert!(matches!(
        run("emit ghost").unwrap_err(),
        LangError::UndeclaredTask { .. }
    ));
    assert!(matches!(
        run("task a at 1 hz phase +\nwhen a aligns ghost { }").unwrap_err(),
        LangError::UndeclaredTask { .. }
    ));
}

#[test]
fn a_redeclared_task_is_refused() {
    assert!(matches!(
        run("task a at 1 hz phase +\ntask a at 2 hz phase -").unwrap_err(),
        LangError::DuplicateTask { .. }
    ));
}

/// `E = C_H*nu` cannot price a non-positive frequency. A zero-frequency task is
/// born reclaimable and a negative one carries negative energy; both are
/// refused at declaration rather than emitted and then swept.
#[test]
fn an_unphysical_frequency_is_refused() {
    for bad in ["task a at 0 hz phase +", "task a at -5 hz phase +"] {
        assert!(
            matches!(run(bad), Err(LangError::UnphysicalFrequency { .. })),
            "{bad:?} was accepted"
        );
    }
}

#[test]
fn an_unterminated_branch_is_refused() {
    assert!(matches!(
        run("task a at 1 hz phase +\nwhen a aligns a { emit a").unwrap_err(),
        LangError::UnexpectedEnd { .. }
    ));
}

/// Comments and blank lines are ignored; braces need not be spaced apart.
#[test]
fn source_layout_does_not_change_the_program() {
    let spaced = run("\
task a at 1 hz phase +
task b at 1 hz phase +

when a aligns b {
    emit a
}
")
    .unwrap();

    let dense = run("task a at 1 hz phase +  # the carrier\ntask b at 1 hz phase +\nwhen a aligns b {emit a}\n")
        .unwrap();

    assert_eq!(spaced.emitted.len(), dense.emitted.len());
    assert_eq!(spaced.branches, dense.branches);
}

/// The parser produces the structure the grammar claims, with the body nested
/// inside the branch rather than flattened alongside it.
#[test]
fn the_parser_nests_branch_bodies() {
    let stmts = parse(&lex("task a at 7 hz phase -\nwhen a opposes a { fork a\nemit a }").unwrap())
        .unwrap();

    assert_eq!(stmts.len(), 2);
    match &stmts[0] {
        Stmt::Task {
            name,
            frequency,
            phase,
            scale,
        } => {
            assert_eq!(name, "a");
            assert_eq!(*frequency, 7.0);
            assert_eq!(*phase, Phase::Negative);
            assert_eq!(*scale, REFERENCE_SCALE, "an omitted scale is the reference");
        }
        other => panic!("expected a task declaration, got {other:?}"),
    }
    match &stmts[1] {
        Stmt::Branch {
            alignment, body, ..
        } => {
            assert_eq!(*alignment, Alignment::Opposes);
            assert_eq!(body.len(), 2, "the body must nest, not flatten");
        }
        other => panic!("expected a branch, got {other:?}"),
    }
}

/// An empty program is valid and does nothing. Worth pinning: the runner walks
/// a slice, and an empty slice must not be a special case.
#[test]
fn an_empty_program_is_valid_and_inert() {
    let exec = run("# nothing but a comment\n\n").unwrap();
    assert!(exec.declared.is_empty());
    assert!(exec.emitted.is_empty());
    assert_eq!(exec.total_energy_joules(), 0.0);
}

// ==================================================================
// Gate 2 — phase shift (`_mkb/gates.md` §2)
// ==================================================================

/// The π shift maps A2's orientation set onto itself. That closure is what
/// makes inversion a *gate* rather than an escape from the axiom.
#[test]
fn inversion_is_closed_on_the_two_orientations() {
    let exec = run("\
task up   at 1 hz phase +
task down at 1 hz phase -
invert up
invert down
")
    .unwrap();

    assert_eq!(exec.inversions.len(), 2);
    assert_eq!(exec.declared["up"].guard_phase(), Phase::Negative);
    assert_eq!(exec.declared["down"].guard_phase(), Phase::Positive);

    // No third orientation was reachable.
    for ev in &exec.inversions {
        assert_ne!(ev.before, ev.after, "the shift must actually move the phase");
    }
}

/// Inversion is its own inverse. A Boolean `!` also is — but this one is
/// asserted *through the exact π shift*, and the next test pins that.
#[test]
fn inversion_is_an_involution() {
    for start in ["+", "-"] {
        let once = run(&format!("task a at 1 hz phase {start}\ninvert a")).unwrap();
        let twice = run(&format!("task a at 1 hz phase {start}\ninvert a\ninvert a")).unwrap();

        assert_ne!(
            once.declared["a"].guard_phase(),
            twice.declared["a"].guard_phase()
        );
        assert_eq!(
            twice.declared["a"].guard_phase(),
            run(&format!("task a at 1 hz phase {start}")).unwrap().declared["a"].guard_phase(),
            "two inversions must return the original orientation"
        );
    }
}

/// The gate is the π shift of Phase Inversion Teardown, and A2's orientations
/// are separated by exactly π. Both facts asserted in radians, bit-exactly —
/// this is the step a Boolean `!` cannot reproduce.
#[test]
fn the_inversion_gate_is_exactly_the_teardown_shift() {
    let exec = run("task a at 1 hz phase -\ninvert a").unwrap();
    let ev = &exec.inversions[0];

    let delta = ev.after.radians() - ev.before.radians();
    assert_eq!(
        delta,
        std::f64::consts::PI,
        "inversion must be the exact pi shift, got {delta}"
    );

    // The teardown identity: a phase and its inversion superpose to exactly 0.
    let sum = ev.before.radians().sin() + ev.after.radians().sin();
    assert_eq!(sum, 0.0, "f_total = f_A + f_B must be exactly zero, got {sum}");
}

/// Inversion changes what the *interference* gate subsequently answers. The
/// two gates compose; neither is decorative.
#[test]
fn inverting_flips_a_later_interference_branch() {
    let before = run("\
task a at 1 hz phase +
task b at 1 hz phase +
when a aligns b { emit a }
")
    .unwrap();
    assert_eq!(before.emitted.len(), 1);

    let after = run("\
task a at 1 hz phase +
task b at 1 hz phase +
invert b
when a aligns b { emit a }
")
    .unwrap();
    assert_eq!(after.emitted.len(), 0, "the inversion must cancel the branch");
}

/// Gate 2 is total: A2's set is closed under the shift, so there is no failure
/// case. The only way `invert` can fail is a name that was never declared —
/// which is scoping, not physics.
#[test]
fn inversion_never_fails_on_a_declared_task() {
    let exec = run("task a at 1 hz phase +\ninvert a\ninvert a\ninvert a\ninvert a\ninvert a")
        .unwrap();
    assert_eq!(exec.inversions.len(), 5);
    assert_eq!(exec.declared["a"].guard_phase(), Phase::Negative, "odd count");

    assert!(matches!(
        run("invert ghost").unwrap_err(),
        LangError::UndeclaredTask { .. }
    ));
}

// ==================================================================
// Gate 3 — scale modulation (`_mkb/gates.md` §3)
// ==================================================================

/// The band is `1/8`, derived as `(π/4)/(2π)`. Asserted against the closed-form
/// boundary at equal scale: `|a−b|/mean = 1/8` solves to `b = a·17/15`.
#[test]
fn the_resonance_band_is_the_derived_one_eighth() {
    // 440 * 17/15 = 498.666... is exactly on the boundary.
    let inside = run("\
task a at 440 hz phase +
task b at 498 hz phase +
when a resonates b { emit a }
")
    .unwrap();
    assert_eq!(inside.emitted.len(), 1, "498 Hz is inside the band");

    let outside = run("\
task a at 440 hz phase +
task b at 500 hz phase +
when a resonates b { emit a }
")
    .unwrap();
    assert_eq!(outside.emitted.len(), 0, "500 Hz is outside the band");

    // And the ratio itself, measured through the kernel.
    let boundary = 440.0 * 17.0 / 15.0;
    let d = detuning(440.0, 1.0, boundary, 1.0).unwrap();
    assert!(
        (d - 0.125).abs() < 1e-12,
        "the closed-form boundary must land on 1/8, got {d}"
    );
}

/// **The gate reads scale, not just frequency.** Identical nominal frequencies
/// detune when observed at sufficiently different scales.
///
/// This is the assertion that makes it "scale modulation" rather than a
/// frequency comparison. Boundary verified at r ≈ 1.18922 against r = 1.
#[test]
fn identical_frequencies_detune_across_scales() {
    let near = run("\
task a at 440 hz phase +
task b at 440 hz phase + scale 1.15
when a resonates b { emit a }
")
    .unwrap();
    assert_eq!(near.emitted.len(), 1, "scale 1.15 is inside the band");

    let far = run("\
task a at 440 hz phase +
task b at 440 hz phase + scale 1.30
when a resonates b { emit a }
")
    .unwrap();
    assert_eq!(far.emitted.len(), 0, "scale 1.30 must detune");

    // The measured boundary from the law's own verification table.
    let inside = detuning(440.0, 1.0, 440.0, 1.18922).unwrap();
    let outside = detuning(440.0, 1.0, 440.0, 1.19).unwrap();
    assert!(inside <= 0.125, "r=1.18922 should be inside, got {inside}");
    assert!(outside > 0.125, "r=1.19 should be outside, got {outside}");
}

/// An omitted `scale` is the reference scale, where `ξ(R) = 1` exactly — so it
/// applies no correction and is not a special case in the interpreter.
#[test]
fn an_omitted_scale_is_the_reference_scale() {
    let implicit = run("task a at 440 hz phase +\nemit a").unwrap();
    let explicit = run("task a at 440 hz phase + scale 1\nemit a").unwrap();

    assert_eq!(implicit.declared["a"].scale(), REFERENCE_SCALE);
    assert_eq!(
        implicit.total_energy_joules(),
        explicit.total_energy_joules(),
        "declaring the reference scale must change nothing"
    );
}

/// Scale is not decorative: it changes what a task costs, through `ξ`.
/// `ξ` is strictly decreasing, so a larger scale is cheaper.
#[test]
fn scale_modulates_energy_through_xi() {
    let refr = run("task a at 440 hz phase +\nemit a").unwrap();
    let small = run("task a at 440 hz phase + scale 0.5\nemit a").unwrap();
    let large = run("task a at 440 hz phase + scale 2.0\nemit a").unwrap();

    let (e_ref, e_small, e_large) = (
        refr.total_energy_joules(),
        small.total_energy_joules(),
        large.total_energy_joules(),
    );

    assert!(
        e_small > e_ref && e_ref > e_large,
        "xi is strictly decreasing: expected {e_small} > {e_ref} > {e_large}"
    );

    // Relative, not absolute — these are of order 1e-32 J.
    let ratio = e_large / e_ref;
    assert!(
        (ratio - 0.5676676).abs() < 1e-6,
        "xi(2)/xi(1) should be ~0.56767, got {ratio}"
    );
}

/// `detunes` is gate 3's other polarity, and it is a separate statement rather
/// than an `else` — same structural property as `aligns`/`opposes`.
#[test]
fn detunes_is_the_other_polarity_of_the_same_gate() {
    let exec = run("\
task a at 440 hz phase +
task b at 900 hz phase +
task c at 445 hz phase +
when a detunes b { emit b }
when a resonates c { emit c }
")
    .unwrap();

    assert!(exec.branches.iter().all(|br| br.taken), "both must be taken");
    assert_eq!(exec.emitted.len(), 2);
}

/// **The gates are independent.** A pair that interferes destructively can
/// resonate — so neither gate is expressible through the other.
///
/// This is the witness recorded in `_mkb/gates.md` §3.6, asserted in code.
#[test]
fn the_interference_and_resonance_gates_disagree() {
    let exec = run("\
task a at 440 hz phase +
task b at 440 hz phase -
when a aligns b     { emit a }
when a resonates b  { emit b }
")
    .unwrap();

    assert_eq!(exec.branches.len(), 2);

    // Gate 1: opposed orientations cancel.
    assert_eq!(exec.branches[0].interference, Interference::Destructive);
    assert!(!exec.branches[0].taken);

    // Gate 3: identical frequency and scale, zero detuning.
    assert_eq!(exec.branches[1].interference, Interference::Constructive);
    assert!(exec.branches[1].taken);

    assert_eq!(
        exec.emitted.len(),
        1,
        "exactly the resonance branch should have fired"
    );
}

/// Each gate ignores what the other reads. Changing phase cannot move gate 3;
/// changing scale cannot move gate 1.
#[test]
fn each_gate_ignores_the_other_gates_inputs() {
    // Gate 3 is blind to phase.
    let same_phase = run(
        "task a at 440 hz phase +\ntask b at 440 hz phase +\nwhen a resonates b { emit a }",
    )
    .unwrap();
    let diff_phase = run(
        "task a at 440 hz phase +\ntask b at 440 hz phase -\nwhen a resonates b { emit a }",
    )
    .unwrap();
    assert_eq!(same_phase.branches[0].taken, diff_phase.branches[0].taken);

    // Gate 1 is blind to scale and frequency.
    let same_scale =
        run("task a at 440 hz phase +\ntask b at 440 hz phase +\nwhen a aligns b { emit a }")
            .unwrap();
    let diff_scale = run(
        "task a at 440 hz phase +\ntask b at 9 hz phase + scale 7.5\nwhen a aligns b { emit a }",
    )
    .unwrap();
    assert_eq!(same_scale.branches[0].taken, diff_scale.branches[0].taken);
    assert!(diff_scale.branches[0].taken, "gate 1 must still fire");
}

/// `ξ(r) → 0` as `r → ∞`, so a large enough scale collapses the mean effective
/// frequency and the detuning ratio becomes `0/0`. Refused, not answered —
/// the same discipline `⊗` applies at its domain limit.
#[test]
fn a_collapsed_resonance_pair_is_refused() {
    let err = run("\
task a at 1 hz phase + scale 1e308
task b at 1 hz phase + scale 1e308
when a resonates b { emit a }
")
    .unwrap_err();
    assert!(
        matches!(err, LangError::UnresonatablePair { .. }),
        "expected refusal, got {err:?}"
    );
}

/// A scale outside `ξ`'s domain is not an observation point at all.
#[test]
fn a_negative_scale_is_refused() {
    assert!(matches!(
        run("task a at 1 hz phase + scale -2").unwrap_err(),
        LangError::UndefinedScale { .. }
    ));
}

/// `ξ(0)` is the supremum, not an error — the expression is `0/0` there and
/// evaluates by limit. A zero scale is a legal observation point.
#[test]
fn a_zero_scale_is_legal_and_maximally_corrected() {
    let zero = run("task a at 440 hz phase + scale 0\nemit a").unwrap();
    let refr = run("task a at 440 hz phase +\nemit a").unwrap();

    let ratio = zero.total_energy_joules() / refr.total_energy_joules();
    assert!(
        (ratio - 2.3130352854993315).abs() < 1e-12,
        "xi(0) is the supremum e/sinh(1); got ratio {ratio}"
    );
}

// ==================================================================
// ξ's boundedness law, which the naive transcription violated
// ==================================================================

/// `ξ` is **bounded above by `e/sinh(1)`** — law, and a safety requirement in
/// the clock path. The literal transcription of the formula overflows `f64` at
/// `r ≈ 710.5` and returns `+inf`, then `NaN`.
///
/// Driven through the language so the whole path is covered.
#[test]
fn xi_stays_bounded_at_every_declarable_scale() {
    const SUPREMUM: f64 = 2.3130352854993315;
    let base = run("task a at 440 hz phase +\nemit a")
        .unwrap()
        .total_energy_joules();

    for scale in [
        0.0, 1e-9, 1.0, 10.0, 700.0, 710.0, 710.5, 711.0, 745.0, 1e3, 1e30, 1e300,
    ] {
        let e = run(&format!("task a at 440 hz phase + scale {scale}\nemit a"))
            .unwrap()
            .total_energy_joules();
        assert!(
            e.is_finite(),
            "energy at scale {scale} is not finite: {e} — xi diverged"
        );
        assert!(e >= 0.0, "energy at scale {scale} went negative: {e}");
        let correction = e / base;
        assert!(
            correction <= SUPREMUM,
            "xi({scale}) = {correction} exceeds its supremum {SUPREMUM}"
        );
    }
}

/// The gate must never answer from a poisoned `ξ`. At scales where the naive
/// transcription returned `+inf`, the gate still gives a real outcome.
#[test]
fn the_resonance_gate_survives_the_overflow_region() {
    // r = 711 is inside the region where sinh(r) overflows f64.
    let d = detuning(440.0, 1.0, 440.0, 711.0).unwrap();
    assert!(d.is_finite(), "detuning went non-finite: {d}");
    assert!(d > 0.125, "a scale that far out must detune, got {d}");

    let exec = run("\
task a at 440 hz phase +
task b at 440 hz phase + scale 711
when a detunes b { emit a }
")
    .unwrap();
    assert_eq!(exec.emitted.len(), 1, "the pair must register as detuned");
}

// ------------------------------------------------ the instruction-executing
// ------------------------------------------------ state machine (`vm`)
//
// Law: `_mkb/instruction_set.md`. The governing question is unchanged from
// the rest of this file, restated for a second execution engine: would this
// still pass against a `bool`-and-`if` VM? For the equivalence tests below
// the sharper form is: does the flat, program-counter-addressed engine
// produce the *same* answer as the tree-walker it replaces for real
// programs, and does it isolate a fault the way a recursive call stack
// structurally cannot?

fn fresh_pool() -> MemoryPool {
    // 64 bytes/cell comfortably exceeds `vm::STATE_BYTES` (24); small enough
    // that an out-of-range cell ordinal is easy to construct.
    MemoryPool::new(4, 64)
}

/// The core doctrine test for this engine: for real programs exercising
/// every construct the two engines share (declare, invert, branch — both
/// gates, nested — fork, emit), the flat bytecode dispatcher must produce
/// **exactly** the same `Execution`-shaped result as the recursive
/// tree-walker it was built to replace. If it didn't, the flattening in
/// `vm::compile` would not actually preserve the tree-walker's semantics —
/// it would just be a different language wearing the same syntax.
#[test]
fn bytecode_dispatch_matches_the_tree_walker_exactly() {
    let programs = [
        "task a at 440 hz phase +\ntask b at 440 hz phase +\nwhen a aligns b { emit a }",
        "task a at 440 hz phase +\ntask b at 440 hz phase -\nwhen a aligns b { emit a }\nwhen a opposes b { emit b }",
        "task a at 440 hz phase +\ntask b at 440 hz phase - scale 1.15\nwhen a resonates b { invert b }\nwhen a aligns b { fork a }",
        "task a at 1 hz phase +\ntask b at 1 hz phase +\ntask c at 1 hz phase -\nwhen a aligns b {\n  when b opposes c { emit c }\n  emit b\n}\nemit a",
    ];

    for source in programs {
        let tokens = lex(source).unwrap();
        let stmts = parse(&tokens).unwrap();

        let tree = execute(&stmts).unwrap();

        let instructions = compile(&stmts);
        let mut pool = fresh_pool();
        let mut tracker = ResourceTracker::new();
        let mut graph = WaitForGraph::new();
        let mut vm = Vm::new(&mut pool, &mut tracker, &mut graph);
        let outcome = vm.run_program(TaskId(0), &instructions);

        assert!(
            outcome.halted && outcome.trap.is_none(),
            "program {source:?} should run to completion, got {:?}",
            outcome.trap
        );
        assert_eq!(outcome.declared, tree.declared, "declared tasks diverged for {source:?}");
        assert_eq!(outcome.emitted, tree.emitted, "emitted tasks diverged for {source:?}");
        assert_eq!(outcome.forks, tree.forks, "forks diverged for {source:?}");
        assert_eq!(outcome.inversions, tree.inversions, "inversions diverged for {source:?}");
        assert_eq!(outcome.branches, tree.branches, "branches diverged for {source:?}");
    }
}

/// A second, more surgical angle on the same claim: `compile`'s `skip`
/// values must jump exactly past a flattened body, not one instruction short
/// or long — an off-by-one here would silently execute (or silently skip)
/// the *first* instruction after a branch, which the equivalence test above
/// would also catch, but this pins the compiled shape directly so a failure
/// here points straight at `compile` rather than requiring a diff against
/// the tree-walker to localise it.
#[test]
fn nested_branches_compile_with_correct_skip_targets() {
    let source = "\
task a at 1 hz phase +
task b at 1 hz phase +
when a aligns b {
    fork a
    when a aligns b { emit a }
}
emit b
";
    let stmts = parse(&lex(source).unwrap()).unwrap();
    let instructions = compile(&stmts);

    // [0]=Task a, [1]=Task b, [2]=Eval(outer, skip=?), [3]=Fork a,
    // [4]=Eval(inner, skip=?), [5]=Emit a, [6]=Emit b.
    assert_eq!(instructions.len(), 7, "unexpected compiled length: {instructions:?}");
    match &instructions[2] {
        // The outer body flattens to [Fork, Eval-inner, Emit a] — the inner
        // branch's own body is part of the outer body's flattened length,
        // not skipped separately.
        Instruction::Eval { skip, .. } => assert_eq!(*skip, 3, "outer body is [Fork, Eval-inner, Emit a]"),
        other => panic!("expected the outer Eval at index 2, got {other:?}"),
    }
    match &instructions[4] {
        Instruction::Eval { skip, .. } => assert_eq!(*skip, 1, "inner body is [Emit a]"),
        other => panic!("expected the inner Eval at index 4, got {other:?}"),
    }
}

/// `store`/`load` move a task's real physical state through real curved
/// memory — not a copy of the in-language value, the actual bytes a
/// `substrate::MemoryPool` holds.
#[test]
fn store_then_load_round_trips_a_tasks_real_state_through_curved_memory() {
    let source = "\
task a at 440 hz phase - scale 1.5
store a at cell 0
load b at cell 0
";
    let instructions = compile(&parse(&lex(source).unwrap()).unwrap());
    let mut pool = fresh_pool();
    let mut tracker = ResourceTracker::new();
    let mut graph = WaitForGraph::new();
    let mut vm = Vm::new(&mut pool, &mut tracker, &mut graph);
    let outcome = vm.run_program(TaskId(0), &instructions);

    assert!(outcome.trap.is_none(), "round trip should not fault: {:?}", outcome.trap);
    let a = &outcome.declared["a"];
    let b = &outcome.declared["b"];
    assert_eq!(b.frequency(), a.frequency());
    assert_eq!(b.guard_phase(), a.guard_phase());
    assert_eq!(b.scale(), a.scale());
    assert_eq!(outcome.stores, vec![("a".to_string(), Address::Cell(0))]);
    assert_eq!(outcome.loads, vec![("b".to_string(), Address::Cell(0))]);
}

/// Memory nothing ever `store`d into is zero bytes, per
/// `substrate::memory::MemoryPool::new`. Zero does not decode to a valid
/// phase (A2 admits exactly `+-pi/2`), so `load` from it must refuse —
/// **not** fabricate a task from garbage.
#[test]
fn loading_a_never_stored_cell_is_refused_not_fabricated() {
    let instructions = compile(&parse(&lex("load ghost at cell 1").unwrap()).unwrap());
    let mut pool = fresh_pool();
    let mut tracker = ResourceTracker::new();
    let mut graph = WaitForGraph::new();
    let mut vm = Vm::new(&mut pool, &mut tracker, &mut graph);
    let outcome = vm.run_program(TaskId(0), &instructions);

    assert!(
        matches!(outcome.trap, Some(VmFault::CorruptState { address: Address::Cell(1) })),
        "expected CorruptState, got {:?}",
        outcome.trap
    );
    assert!(!outcome.declared.contains_key("ghost"));
}

/// A cell ordinal past the pool's own size is refused with a fault named for
/// what it actually is — not a `SubstrateError::Unmapped`, since no `CellId`
/// was ever named to be unmapped.
#[test]
fn an_out_of_range_cell_traps_the_program() {
    let instructions =
        compile(&parse(&lex("task a at 1 hz phase +\nstore a at cell 999").unwrap()).unwrap());
    let mut pool = fresh_pool();
    let mut tracker = ResourceTracker::new();
    let mut graph = WaitForGraph::new();
    let mut vm = Vm::new(&mut pool, &mut tracker, &mut graph);
    let outcome = vm.run_program(TaskId(0), &instructions);

    match outcome.trap {
        Some(VmFault::CellOutOfRange { cell: 999, cells }) => assert_eq!(cells, pool.cell_count()),
        other => panic!("expected CellOutOfRange, got {other:?}"),
    }
}

/// **The central claim of `_mkb/instruction_set.md`'s fault-isolation
/// section, demonstrated directly.** A batch of three programs shares one
/// pool: the first stores real state, the second traps on a cell nothing
/// stored into, the third runs afterward against the *same* pool and
/// succeeds. The faulting program's trap must not touch the other two — no
/// panic, no corrupted pool, no skipped program.
#[test]
fn a_fault_isolates_only_the_faulting_program_in_a_batch() {
    let good_first = compile(&parse(&lex("task a at 1 hz phase +\nstore a at cell 0").unwrap()).unwrap());
    let faulting = compile(&parse(&lex("load ghost at cell 1").unwrap()).unwrap());
    let good_second =
        compile(&parse(&lex("load a at cell 0\nemit a").unwrap()).unwrap());

    let mut pool = fresh_pool();
    let mut tracker = ResourceTracker::new();
    let mut graph = WaitForGraph::new();
    let mut vm = Vm::new(&mut pool, &mut tracker, &mut graph);
    let results = vm.run_batch(&[good_first, faulting, good_second]);

    assert_eq!(results.len(), 3);
    assert!(results[0].trap.is_none(), "program 0 should succeed: {:?}", results[0].trap);
    assert!(
        matches!(results[1].trap, Some(VmFault::CorruptState { .. })),
        "program 1 should trap, got {:?}",
        results[1].trap
    );
    assert!(
        results[2].trap.is_none(),
        "program 2 must run cleanly against the pool program 1 faulted against: {:?}",
        results[2].trap
    );
    assert_eq!(results[2].declared["a"].frequency(), 1.0);
    assert_eq!(results[2].emitted.len(), 1, "program 2's own emit must have run");
}

/// `acquire`/`release` are real calls into `symphony_kernel::resources`, not
/// a language-level fiction — closing the gap `symphony-lang`'s own record
/// named: "nothing in the language declares resource acquisition."
#[test]
fn acquire_then_release_is_real_resource_tracking() {
    let instructions = compile(&parse(&lex("acquire 7\nrelease 7").unwrap()).unwrap());
    let mut pool = fresh_pool();
    let mut tracker = ResourceTracker::new();
    let mut graph = WaitForGraph::new();
    let mut vm = Vm::new(&mut pool, &mut tracker, &mut graph);
    let outcome = vm.run_program(TaskId(0), &instructions);

    assert!(outcome.trap.is_none(), "should not fault: {:?}", outcome.trap);
    assert_eq!(outcome.resources.len(), 2);
    assert_eq!(outcome.resources[0].outcome, ResourceOutcome::Granted);
    assert_eq!(outcome.resources[1].outcome, ResourceOutcome::Released { next: None });
}

/// This VM runs one program to completion at a time within a batch — there
/// is no scheduler able to suspend a blocked program and resume it once the
/// holder releases. `acquire` on a resource another program in the same
/// batch holds must therefore **trap**, not hang the batch and not proceed
/// as though it had been granted. Stated as a real limit in
/// `_mkb/instruction_set.md`, verified here rather than left to be
/// discovered as a hang.
#[test]
fn a_blocked_acquire_traps_rather_than_hanging_or_silently_granting() {
    let holds_it = compile(&parse(&lex("acquire 3").unwrap()).unwrap()); // never releases
    let wants_it = compile(&parse(&lex("acquire 3").unwrap()).unwrap());

    let mut pool = fresh_pool();
    let mut tracker = ResourceTracker::new();
    let mut graph = WaitForGraph::new();
    let mut vm = Vm::new(&mut pool, &mut tracker, &mut graph);
    let results = vm.run_batch(&[holds_it, wants_it]);

    assert!(results[0].trap.is_none(), "the holder should acquire cleanly");
    match &results[1].trap {
        Some(VmFault::Blocked { resource, holder }) => {
            assert_eq!(resource.0, 3);
            assert_eq!(*holder, TaskId(0), "program 1 is blocked on program 0");
        }
        other => panic!("expected Blocked, got {other:?}"),
    }
}

/// `halt` ends the program immediately — later instructions in the same
/// program must never run.
#[test]
fn halt_stops_the_program_before_later_instructions_run() {
    let instructions =
        compile(&parse(&lex("task a at 1 hz phase +\nhalt\ntask b at 1 hz phase +").unwrap()).unwrap());
    let mut pool = fresh_pool();
    let mut tracker = ResourceTracker::new();
    let mut graph = WaitForGraph::new();
    let mut vm = Vm::new(&mut pool, &mut tracker, &mut graph);
    let outcome = vm.run_program(TaskId(0), &instructions);

    assert!(outcome.halted && outcome.trap.is_none());
    assert!(outcome.declared.contains_key("a"));
    assert!(!outcome.declared.contains_key("b"), "halt must pre-empt the rest of the program");
}

/// A `store`/`load` cell ordinal indexes `MemoryPool::address_at`; it is not
/// a physical quantity, so a negative or fractional one is refused at parse
/// time — the same discipline `NonIntegralFork` already applies to fork
/// counts.
#[test]
fn invalid_cell_ordinal_is_refused_at_parse_time() {
    for bad in ["store a at cell -1", "store a at cell 1.5"] {
        match parse(&lex(bad).unwrap()) {
            Err(LangError::InvalidCellOrdinal { .. }) => {}
            other => panic!("expected InvalidCellOrdinal for {bad:?}, got {other:?}"),
        }
    }
}

/// `store` on a name the program never declared is a language-level fault —
/// the same `UndeclaredTask` the tree-walker would report, wrapped rather
/// than re-derived, so both engines name the same failure the same way.
#[test]
fn store_requires_a_declared_task() {
    let instructions = compile(&parse(&lex("store ghost at cell 0").unwrap()).unwrap());
    let mut pool = fresh_pool();
    let mut tracker = ResourceTracker::new();
    let mut graph = WaitForGraph::new();
    let mut vm = Vm::new(&mut pool, &mut tracker, &mut graph);
    let outcome = vm.run_program(TaskId(0), &instructions);

    assert!(matches!(
        outcome.trap,
        Some(VmFault::Lang(LangError::UndeclaredTask { .. }))
    ));
}

/// The tree-walking interpreter has no memory pool or resource tracker to
/// act against — real execution of the new instructions needs the VM, and
/// the interpreter says so by name rather than silently ignoring them.
#[test]
fn tree_walker_refuses_vm_only_instructions() {
    for source in ["store a at cell 0", "load a at cell 0", "acquire 1", "release 1", "halt"] {
        match run(source) {
            Err(LangError::RequiresVm { .. }) => {}
            other => panic!("expected RequiresVm for {source:?}, got {other:?}"),
        }
    }
}

// ------------------------------------- path addressing (`⊗`-fold, Phase 2)
//
// `store`/`load` also accept `path START S1 S2 ...` — the same `⊗`-fold
// directory-style addressing `lattice::addressing` and
// `substrate::MemoryPool::resolve_path` already define and test
// (`neos/tests/substrate.rs` Group 7). The pool sizes and path values below
// are taken directly from that suite's own proven cases rather than
// re-guessed, so what's actually new here is only the language-level
// plumbing: `vm::Vm::resolve` routing an `Address::Path` through the
// identical, already-verified `resolve_path`.

/// Mirrors `substrate.rs`'s `a_large_enough_pool_maps_an_ordinary_resolved_cell`
/// (`MemoryPool::new(200, 64)`, `AddressPath::new(1.0, &[1.0])`): a real,
/// non-origin, mapped cell — round-tripped here through the language rather
/// than the raw pool API.
#[test]
fn store_then_load_round_trips_through_a_path_address() {
    let source = "\
task a at 440 hz phase - scale 1.5
store a at path 1.0 1.0
load b at path 1.0 1.0
";
    let instructions = compile(&parse(&lex(source).unwrap()).unwrap());
    let mut pool = MemoryPool::new(200, 64);
    let mut tracker = ResourceTracker::new();
    let mut graph = WaitForGraph::new();
    let mut vm = Vm::new(&mut pool, &mut tracker, &mut graph);
    let outcome = vm.run_program(TaskId(0), &instructions);

    assert!(outcome.trap.is_none(), "round trip should not fault: {:?}", outcome.trap);
    let a = &outcome.declared["a"];
    let b = &outcome.declared["b"];
    assert_eq!(b.frequency(), a.frequency());
    assert_eq!(b.guard_phase(), a.guard_phase());
    assert_eq!(b.scale(), a.scale());
}

/// A path's steps are load-bearing, not decoration: `path 1.0 2.0 1.5` and
/// the bare `path 1.0` resolve to two different real cells in a 200-cell
/// pool (confirmed against the raw `lattice`/`substrate` APIs directly
/// before writing this test — not every extra step changes the resolved
/// cell, e.g. `path 1.0 1.0` and bare `path 1.0` land on the *same* cell in
/// this pool, so the values here are chosen, not assumed). Storing only at
/// the stepped path and then trying to load from the bare start must hit
/// real, untouched (zero-initialised) memory.
#[test]
fn a_paths_steps_resolve_to_a_different_real_cell_than_its_bare_start() {
    let source = "\
task a at 440 hz phase +
store a at path 1.0 2.0 1.5
load ghost at path 1.0
";
    let instructions = compile(&parse(&lex(source).unwrap()).unwrap());
    let mut pool = MemoryPool::new(200, 64);
    let mut tracker = ResourceTracker::new();
    let mut graph = WaitForGraph::new();
    let mut vm = Vm::new(&mut pool, &mut tracker, &mut graph);
    let outcome = vm.run_program(TaskId(0), &instructions);

    assert!(
        matches!(outcome.trap, Some(VmFault::CorruptState { .. })),
        "the bare start must be a different, untouched cell, got {:?}",
        outcome.trap
    );
}

/// Mirrors `substrate.rs`'s `dissonant_path_is_refused_not_panicked`
/// (`AddressPath::new(1.0, &[1.0, 1.0, 1.0, 1.0, 1.0])` — six-deep unit
/// steps leave `⊗`'s domain). Surfaces here as `VmFault::Memory`, the same
/// channel a `SubstrateError` from `store`/`load` always uses, not a new
/// fault kind for path addressing specifically.
#[test]
fn a_path_that_leaves_otimes_domain_traps_as_a_memory_fault() {
    let source = "task a at 1 hz phase +\nstore a at path 1.0 1.0 1.0 1.0 1.0 1.0";
    let instructions = compile(&parse(&lex(source).unwrap()).unwrap());
    let mut pool = MemoryPool::new(31, 64);
    let mut tracker = ResourceTracker::new();
    let mut graph = WaitForGraph::new();
    let mut vm = Vm::new(&mut pool, &mut tracker, &mut graph);
    let outcome = vm.run_program(TaskId(0), &instructions);

    match outcome.trap {
        Some(VmFault::Memory(SubstrateError::AddressUnresolvable(_))) => {}
        other => panic!("expected a wrapped domain refusal, got {other:?}"),
    }
}

/// Mirrors `substrate.rs`'s `a_resolvable_path_can_still_be_unmapped_in_a_small_pool`
/// (`MemoryPool::new(1, 64)`, `AddressPath::new(1.0, &[1.0])`): a path can
/// resolve to a real point that this particular pool does not back — again
/// `VmFault::Memory`, not `CellOutOfRange` (that variant is only reachable
/// through `Address::Cell`, which indexes ring order directly rather than
/// resolving a point).
#[test]
fn a_resolvable_path_outside_a_small_pool_traps_as_unmapped() {
    let source = "task a at 1 hz phase +\nstore a at path 1.0 1.0";
    let instructions = compile(&parse(&lex(source).unwrap()).unwrap());
    let mut pool = MemoryPool::new(1, 64);
    let mut tracker = ResourceTracker::new();
    let mut graph = WaitForGraph::new();
    let mut vm = Vm::new(&mut pool, &mut tracker, &mut graph);
    let outcome = vm.run_program(TaskId(0), &instructions);

    match outcome.trap {
        Some(VmFault::Memory(SubstrateError::Unmapped { .. })) => {}
        other => panic!("expected Unmapped, got {other:?}"),
    }
}

// -------------------------------------- dynamic fault routing (Phase 3)
//
// `Vm::run_program_trapped` — the direct counterpart to
// `substrate::Hypervisor::allocate_trapped`: a real handler, called on every
// memory fault, that can act on `&mut MemoryPool` and ask for a retry.
// `run_program` (used by every test above) is unchanged behaviourally — it
// delegates to this with a trivial always-`Propagate` handler — so every
// prior test is itself a regression guard confirming the new code path
// didn't alter old behaviour.

/// The direct analogue of `allocate_trapped`'s own demonstrated recovery:
/// a `CorruptState` fault (a `load` from a never-`store`d cell) is handed
/// real bytes by the handler, which seeds the exact cell the fault named and
/// asks for a retry — and the retried `load` actually succeeds against that
/// corrected state, not just returns a different error.
///
/// The program declares and emits a task *before* the faulting `load`, so a
/// retry that (incorrectly) restarted the whole program from the top would
/// either double the emitted count or hit `DuplicateTask` redeclaring
/// `seed`. Neither happens: a retry re-attempts only the faulting
/// instruction itself, at the same `pc`, with everything already
/// accumulated left untouched.
#[test]
fn a_corrupt_state_fault_can_be_recovered_by_seeding_the_cell_and_retrying() {
    let source = "\
task seed at 100 hz phase +
emit seed
load a at cell 3
emit a
";
    let instructions = compile(&parse(&lex(source).unwrap()).unwrap());
    let mut pool = fresh_pool();
    let mut tracker = ResourceTracker::new();
    let mut graph = WaitForGraph::new();
    let mut vm = Vm::new(&mut pool, &mut tracker, &mut graph);

    let mut handled = 0usize;
    let outcome = vm.run_program_trapped(TaskId(0), Domain::Kernel, &instructions, 1, |fault, pool| {
        handled += 1;
        assert!(
            matches!(fault, VmFault::CorruptState { address: Address::Cell(3) }),
            "unexpected fault: {fault:?}"
        );
        let addr = pool.address_at(3).unwrap();
        let mut bytes = Vec::with_capacity(24);
        bytes.extend_from_slice(&660.0_f64.to_le_bytes());
        bytes.extend_from_slice(&Phase::Positive.radians().to_le_bytes());
        bytes.extend_from_slice(&1.0_f64.to_le_bytes());
        pool.write(addr, &bytes).unwrap();
        TrapAction::Retry
    });

    assert_eq!(handled, 1, "handler must be called exactly once");
    assert!(outcome.trap.is_none(), "the retried load must succeed: {:?}", outcome.trap);
    assert_eq!(outcome.declared["a"].frequency(), 660.0);
    assert_eq!(outcome.declared["a"].guard_phase(), Phase::Positive);
    assert_eq!(
        outcome.emitted.len(),
        2,
        "seed's emit plus a's emit — not doubled, not lost, by the retry"
    );
}

/// A handler that returns `Retry` but never actually fixes anything cannot
/// hang the caller: the same bound `allocate_trapped` already enforces.
#[test]
fn retries_are_bounded_a_handler_that_never_helps_cannot_hang() {
    let instructions = compile(&parse(&lex("load ghost at cell 1").unwrap()).unwrap());
    let mut pool = fresh_pool();
    let mut tracker = ResourceTracker::new();
    let mut graph = WaitForGraph::new();
    let mut vm = Vm::new(&mut pool, &mut tracker, &mut graph);

    let mut attempts = 0usize;
    let outcome = vm.run_program_trapped(TaskId(0), Domain::Kernel, &instructions, 3, |_, _| {
        attempts += 1;
        TrapAction::Retry
    });

    assert_eq!(attempts, 4, "the original attempt plus exactly 3 retries, then stop");
    assert!(matches!(outcome.trap, Some(VmFault::CorruptState { .. })));
}

/// `Propagate` stops immediately — a handler declining to help does not
/// consume the rest of its retry budget pretending to.
#[test]
fn propagate_still_traps_immediately_even_with_retries_available() {
    let instructions = compile(&parse(&lex("load ghost at cell 1").unwrap()).unwrap());
    let mut pool = fresh_pool();
    let mut tracker = ResourceTracker::new();
    let mut graph = WaitForGraph::new();
    let mut vm = Vm::new(&mut pool, &mut tracker, &mut graph);

    let mut calls = 0usize;
    let outcome = vm.run_program_trapped(TaskId(0), Domain::Kernel, &instructions, 10, |_, _| {
        calls += 1;
        TrapAction::Propagate
    });

    assert_eq!(calls, 1, "Propagate must stop immediately, not exhaust the retry budget");
    assert!(matches!(outcome.trap, Some(VmFault::CorruptState { .. })));
}

/// Scoped to memory faults, matching `allocate_trapped`'s own scoping
/// discipline: a caller-logic error (a `store`/`load` naming a task the
/// program never declared) is never offered to the handler at all — no
/// amount of retrying fixes an undeclared name.
#[test]
fn language_level_faults_never_reach_the_handler() {
    let instructions = compile(&parse(&lex("store ghost at cell 0").unwrap()).unwrap());
    let mut pool = fresh_pool();
    let mut tracker = ResourceTracker::new();
    let mut graph = WaitForGraph::new();
    let mut vm = Vm::new(&mut pool, &mut tracker, &mut graph);

    let mut calls = 0usize;
    let outcome = vm.run_program_trapped(TaskId(0), Domain::Kernel, &instructions, 5, |_, _| {
        calls += 1;
        TrapAction::Retry
    });

    assert_eq!(calls, 0, "an UndeclaredTask fault is a caller logic error, not a memory fault");
    assert!(matches!(outcome.trap, Some(VmFault::Lang(LangError::UndeclaredTask { .. }))));
}

// ---------------------------------------- privilege domains (Phase 4)
//
// `_mkb/instruction_set.md` records these as a **stated engineering
// convention**, not law — no axiom or PRD section defines privilege or
// guest/kernel separation, so there is nothing here to compose the way the
// rest of this module's instructions compose real law. `Domain::Kernel`
// (used by every test above) is unaffected by any of this; these tests
// exercise `Domain::Guest` specifically.

/// The direct claim: a guest program's `store` resolving to a cell the `Vm`
/// marked reserved is refused before it ever reaches the pool.
#[test]
fn a_guest_program_is_refused_from_a_reserved_cell() {
    let instructions =
        compile(&parse(&lex("task a at 1 hz phase +\nstore a at cell 2").unwrap()).unwrap());
    let mut pool = fresh_pool();
    let reserved = pool.address_at(2).unwrap().cell();
    let mut tracker = ResourceTracker::new();
    let mut graph = WaitForGraph::new();
    let mut vm = Vm::new(&mut pool, &mut tracker, &mut graph);
    vm.reserve_cells([reserved]);

    let outcome = vm.run_program(TaskId(0), &instructions); // Domain::Kernel — must succeed
    assert!(outcome.trap.is_none(), "Domain::Kernel must be unaffected by reservation");

    let outcome = vm.run_program_trapped(TaskId(1), Domain::Guest, &instructions, 0, |_, _| {
        panic!("a privilege violation must never reach the handler")
    });
    match outcome.trap {
        Some(VmFault::PrivilegeViolation { cell, .. }) => assert_eq!(cell, reserved),
        other => panic!("expected PrivilegeViolation, got {other:?}"),
    }
}

/// Reservation protects the whole set of cells it's given at once — the
/// mechanism a "system-critical Tetryen patch" (several adjacent cells) is
/// protected by, not just a single named cell.
#[test]
fn reservation_protects_every_cell_in_a_patch() {
    let mut pool = fresh_pool();
    let patch = [pool.address_at(0).unwrap().cell(), pool.address_at(1).unwrap().cell()];
    let mut tracker = ResourceTracker::new();
    let mut graph = WaitForGraph::new();
    let mut vm = Vm::new(&mut pool, &mut tracker, &mut graph);
    vm.reserve_cells(patch);

    for cell in [0u64, 1] {
        let src = format!("task a at 1 hz phase +\nstore a at cell {cell}");
        let instructions = compile(&parse(&lex(&src).unwrap()).unwrap());
        let outcome = vm.run_program_trapped(TaskId(cell + 20), Domain::Guest, &instructions, 0, |_, _| {
            TrapAction::Propagate
        });
        assert!(
            matches!(outcome.trap, Some(VmFault::PrivilegeViolation { .. })),
            "cell {cell} is part of the reserved patch and must be refused, got {:?}",
            outcome.trap
        );
    }
}

/// A handler cannot be used to bypass a privilege boundary — the fault never
/// reaches it, unlike every other memory fault kind, no matter how large a
/// retry budget is offered.
#[test]
fn a_privilege_violation_is_never_offered_to_the_handler_even_with_retries() {
    let instructions =
        compile(&parse(&lex("task a at 1 hz phase +\nstore a at cell 3").unwrap()).unwrap());
    let mut pool = fresh_pool();
    let reserved = pool.address_at(3).unwrap().cell();
    let mut tracker = ResourceTracker::new();
    let mut graph = WaitForGraph::new();
    let mut vm = Vm::new(&mut pool, &mut tracker, &mut graph);
    vm.reserve_cells([reserved]);

    let mut calls = 0usize;
    let outcome = vm.run_program_trapped(TaskId(0), Domain::Guest, &instructions, 100, |_, _| {
        calls += 1;
        TrapAction::Retry
    });

    assert_eq!(calls, 0, "no retry budget can buy past a privilege boundary");
    assert!(matches!(outcome.trap, Some(VmFault::PrivilegeViolation { .. })));
}

/// A path address resolving to a reserved cell is refused exactly the same
/// way a cell-ordinal address is — the check is against the *resolved*
/// `LatticeAddress`, not the source syntax used to name it.
#[test]
fn reservation_applies_to_path_addresses_too() {
    let mut pool = MemoryPool::new(200, 64);
    let reserved = pool.resolve_path(&lattice::AddressPath::new(1.0, &[1.0])).unwrap().cell();
    let mut tracker = ResourceTracker::new();
    let mut graph = WaitForGraph::new();
    let mut vm = Vm::new(&mut pool, &mut tracker, &mut graph);
    vm.reserve_cells([reserved]);

    let instructions =
        compile(&parse(&lex("task a at 1 hz phase +\nstore a at path 1.0 1.0").unwrap()).unwrap());
    let outcome = vm.run_program_trapped(TaskId(0), Domain::Guest, &instructions, 0, |_, _| {
        TrapAction::Propagate
    });
    assert!(matches!(outcome.trap, Some(VmFault::PrivilegeViolation { .. })));
}

/// A path's `start`/steps are real numbers, not indices — a negative or
/// fractional value is not a parse error the way a cell ordinal's would be.
#[test]
fn path_addresses_accept_negative_and_fractional_values() {
    let stmts = parse(&lex("store a at path -1.5 0.25 -3.0").unwrap()).unwrap();
    match &stmts[0] {
        Stmt::Store {
            address: Address::Path { start, steps },
            ..
        } => {
            assert_eq!(*start, -1.5);
            assert_eq!(steps, &vec![0.25, -3.0]);
        }
        other => panic!("expected a Path address, got {other:?}"),
    }
}

// ------------------------------------------ real concurrency (`concurrent`)
//
// `vm::Vm` is sequential by design — `run_batch` runs one program to
// completion at a time, and `_mkb/instruction_set.md` states plainly that
// `acquire` blocking traps rather than suspending because "there is no
// scheduler able to suspend a blocked program and resume it once the
// holder releases." `symphony_lang::concurrent` is that scheduler: real OS
// threads, a real shared `ConcurrentPool`/`ConcurrentTracker`. Deeper
// blocking/deadlock-resolution behaviour is verified directly against
// `ConcurrentTracker` in `neos/tests/symphony_scheduler.rs`; these tests
// verify the *language* layer built on top of it.

/// The direct analogue of `ConcurrentPool`'s own original verification
/// (distinct fingerprint per thread, read back, must match): `N` real
/// threads each run a program that, while holding a shared resource,
/// stores its own distinct frequency into a shared cell and immediately
/// loads it back. If `acquire` did not really exclude other threads from
/// the critical section, at least one thread would sometimes read back a
/// different thread's just-written value instead of its own.
#[test]
fn real_threads_serialize_correctly_under_a_shared_resource() {
    let pool = ConcurrentPool::new(4, 64);
    let tracker = ConcurrentTracker::new();
    let reserved: Arc<HashSet<lattice::tessellation::CellId>> = Arc::new(HashSet::new());

    let threads = 8;
    let programs: Vec<_> = (0..threads)
        .map(|i| {
            let freq = 100.0 + i as f64;
            let source = format!(
                "task me at {freq} hz phase +\n\
                 acquire 1\n\
                 store me at cell 0\n\
                 load echo at cell 0\n\
                 release 1"
            );
            let instructions = compile(&parse(&lex(&source).unwrap()).unwrap());
            (TaskId(i as u64), Domain::Kernel, instructions)
        })
        .collect();

    let outcomes = run_batch_concurrent(&pool, &tracker, &reserved, programs);

    assert_eq!(outcomes.len(), threads);
    for (i, outcome) in outcomes.iter().enumerate() {
        assert!(outcome.trap.is_none(), "thread {i} faulted: {:?}", outcome.trap);
        let me = outcome.declared["me"].frequency();
        let echo = outcome.declared["echo"].frequency();
        assert_eq!(
            echo, me,
            "thread {i} read back {echo} hz instead of its own {me} hz — \
             another thread's store/load interleaved inside the critical section"
        );
    }
}

/// Real concurrent `store`/`load` to *distinct* cells, through the same
/// shared `ConcurrentPool` the language now drives — no resource contention
/// involved, just proving the language-level dispatch loop composes
/// correctly with real concurrent memory access end to end, not only in
/// isolated unit calls.
#[test]
fn store_load_stays_correct_across_distinct_cells_under_real_concurrency() {
    let pool = ConcurrentPool::new(8, 64);
    let tracker = ConcurrentTracker::new();
    let reserved: Arc<HashSet<lattice::tessellation::CellId>> = Arc::new(HashSet::new());

    let threads = 6;
    let programs: Vec<_> = (0..threads)
        .map(|i| {
            let freq = 200.0 + i as f64 * 10.0;
            let source = format!(
                "task mine at {freq} hz phase -\nstore mine at cell {i}\nload back at cell {i}"
            );
            let instructions = compile(&parse(&lex(&source).unwrap()).unwrap());
            (TaskId(i as u64), Domain::Kernel, instructions)
        })
        .collect();

    let outcomes = run_batch_concurrent(&pool, &tracker, &reserved, programs);
    for (i, outcome) in outcomes.iter().enumerate() {
        assert!(outcome.trap.is_none(), "thread {i} faulted: {:?}", outcome.trap);
        assert_eq!(outcome.declared["back"].frequency(), 200.0 + i as f64 * 10.0);
        assert_eq!(outcome.declared["back"].guard_phase(), Phase::Negative);
    }
}

/// Privilege domains (Phase 4) hold under real concurrency too: a
/// `Domain::Guest` program in the same batch as trusted `Domain::Kernel`
/// programs is still refused from a reserved cell, even though the check
/// now runs on its own real OS thread rather than the single sequential
/// dispatch loop it was originally verified against.
#[test]
fn privilege_domains_are_enforced_across_real_concurrent_threads() {
    let pool = ConcurrentPool::new(4, 64);
    let tracker = ConcurrentTracker::new();
    let reserved_cell = pool.address_at(2).unwrap().cell();
    let mut set = HashSet::new();
    set.insert(reserved_cell);
    let reserved = Arc::new(set);

    let guest = compile(
        &parse(&lex("task x at 1 hz phase +\nstore x at cell 2").unwrap()).unwrap(),
    );
    let kernel = compile(
        &parse(&lex("task y at 1 hz phase +\nstore y at cell 3").unwrap()).unwrap(),
    );

    let outcomes = run_batch_concurrent(
        &pool,
        &tracker,
        &reserved,
        vec![
            (TaskId(0), Domain::Guest, guest),
            (TaskId(1), Domain::Kernel, kernel),
        ],
    );

    assert!(
        matches!(outcomes[0].trap, Some(VmFault::PrivilegeViolation { .. })),
        "the guest program must be refused, got {:?}",
        outcomes[0].trap
    );
    assert!(outcomes[1].trap.is_none(), "the trusted program must be unaffected");
}

---
type: test-plan
subsystem: symphony-lang
stage: 03_tests
status: complete
suite: neos/tests/symphony_lang.rs
assertions: 27
---

# Symphony-lang — Test Plan

## The doctrine question, in this record's form

`_mkb/test-doctrine.md` asks: *would a conventional implementation still pass?* For a DSL that sharpens to:

> **Would this test pass against a language with `if` and `bool`?**

If yes, it is testing a parser, not NEOS. Every assertion below is classified against that question.

## Group 1 — A2 is syntactic (5)

| Assertion | Would a Boolean language pass? |
|---|---|
| `boolean_constructs_are_refused_at_lex_time` | **No** — it passes only by not existing |
| `the_a2_refusal_explains_itself` | **No** — requires the error to cite A2 and name the replacement |
| `only_two_phase_literals_exist` | **No** — a third literal must be rejected |
| `a2_refusal_precedes_every_other_error` | **No** — ordering claim: A2 beats name resolution |
| `source_layout_does_not_change_the_program` | Yes — a plain parser test, included for the comment/brace handling |

All thirteen forbidden constructs are enumerated in the first test rather than sampled. A blacklist that is only spot-checked will grow a hole the first time someone adds a token.

`a2_refusal_precedes_every_other_error` uses a source that is *both* Boolean-contaminated and name-broken, and asserts which error wins. Order is a real requirement: a programmer who fixes the syntax first and only then learns the construct was forbidden has been told the wrong thing twice.

## Group 2 — branching is interference (5)

| Assertion | Would a Boolean language pass? |
|---|---|
| `a_branch_is_taken_by_interference_not_by_truth` | **No** — asserts `Interference`, not a bool |
| `branch_forms_are_independent_statements` | **No** — asserts both bodies run, and separately that neither does |
| `alignment_partitions_the_two_phase_orientations` | Yes — and it is included *because* it is true |
| `interference_is_symmetric_in_its_operands` | Partly — symmetry over all four phase pairs |
| `nested_branches_gate_on_the_outer_interference` | Partly — also asserts the inner branch is not *evaluated* |

`branch_forms_are_independent_statements` is the one that `if`/`else` cannot survive in either direction: it builds a program where both forms are taken, and a mirrored one where neither is. Exactly one arm always runs in a classical conditional.

`alignment_partitions_the_two_phase_orientations` states the *weaker* true position deliberately. See the implementation log — a sabotage established that per-branch `opposes` really is the complement of `aligns`, so a test claiming otherwise would have been asserting something false.

`nested_branches_gate_on_the_outer_interference` checks `branches.len()`, not just emissions. A cancelled outer branch must leave the inner one **unevaluated**, not evaluated-and-discarded.

## Group 3 — bifurcation is A1 (5)

| Assertion | Would a Boolean language pass? |
|---|---|
| `fork_yields_exactly_two_children` | **No** — `1 ⊗ 1 = 2` bit-exact, `==` not approx |
| `fork_scales_address_space_with_child_count` | **No** — structural split, not duplication |
| `forks_accumulate_across_statements` | **No** — three forks give six, not four |
| `a_cancelled_branch_forks_nothing` | **No** — joins A1 and A2 in one program |
| `a_fractional_child_count_is_refused` | **No** — `2 ⊗ 2 = 20.97…`, refused not rounded |

`fork_yields_exactly_two_children` asserts `== 2.0` with no epsilon. That is not sloppiness — `sinh(arcsinh 1) = 1` identically, so the result is bit-exact, and an epsilon here would hide a reimplemented ⊗.

`a_fractional_child_count_is_refused` reaches the guard through `execute_with`, because surface syntax always forks at the canonical unit. Without that seam the assertion would be theatre.

## Group 4 — the ⊗ ceiling, fourth appearance (1)

`a_fork_outside_the_otimes_domain_is_refused`. The domain limit has now surfaced in four subsystems at four arities. Here it must arrive as a `LangError`, not a panic and not a clamped value.

## Group 5 — the seam and energy (4)

| Assertion | Would a Boolean language pass? |
|---|---|
| `runtime_task_implements_the_kernel_seam` | **No** — generic over `TaskModel` |
| `emitted_tasks_drive_the_real_scheduler` | **No** — runs a real relaxation pass |
| `program_energy_is_linear_in_declared_frequency` | **No** — `E = C_H·ν` |
| `energy_is_independent_of_the_path_that_emitted_it` | **No** — energy is a property of waves, not control flow |

`emitted_tasks_drive_the_real_scheduler` is the record's end-to-end assertion: source text → `Scheduler::ingest` → a relaxation pass that does not destabilise the field. It crosses `symphony-lang`, `symphony-kernel`, `substrate`, and `lattice`.

`program_energy_is_linear_in_declared_frequency` is the one tolerance in the suite, and it is **relative** (`|ratio − 2| < 1e-12`). Energies are of order `1e-32` J; an absolute threshold at any plausible magnitude would pass on nothing. Applied up front rather than discovered — this trap has now cost this workspace four separate debugging sessions.

## Group 6 — well-formedness (7)

`an_undeclared_task_is_refused`, `a_redeclared_task_is_refused`, `seeding_does_not_open_a_hole_in_scoping`, `an_unphysical_frequency_is_refused`, `an_unterminated_branch_is_refused`, `the_parser_nests_branch_bodies`, `an_empty_program_is_valid_and_inert`.

These would pass in a conventional language and are labelled as such. They are here because a language that accepts `emit ghost` is broken regardless of its axioms.

`an_unphysical_frequency_is_refused` is the exception in this group: `ν ≤ 0` is refused for a *physical* reason (`E = C_H·ν` gives negative energy; `ν = 0` is born reclaimable), not a syntactic one.

## Planned sabotages

| Mutation | Predicted failures |
|---|---|
| disable the A2 refusal in the lexer | Group 1's first four |
| treat `opposes` as `else` | Group 2 — **prediction recorded before running** |
| run every branch body regardless of interference | Groups 2 and 3 |
| truncate a fractional child count instead of refusing | Group 3's last |

The second prediction is recorded here deliberately so the log can report what actually happened.

## Human check

Read `boolean_constructs_are_refused_at_lex_time` and `branch_forms_are_independent_statements`. The first is the only place in NEOS where an axiom is enforced by refusing to *tokenise*. The second is the assertion `if`/`else` cannot satisfy in either direction.

---

# Slice 2 — gates 2 and 3

17 new assertions (44 total in this suite; 38 in `symphony_kernel`, which gained the kernel-side gate group and the `ξ` domain tests).

## Group 7 — phase shift (5)

| Assertion | Would a Boolean language pass? |
|---|---|
| `inversion_is_closed_on_the_two_orientations` | **No** — closure on A2's set |
| `the_inversion_gate_is_exactly_the_teardown_shift` | **No** — asserts `Δφ == π` bit-exactly |
| `inverting_flips_a_later_interference_branch` | **No** — gates 1 and 2 compose |
| `inversion_is_an_involution` | Yes — `!` is too |
| `inversion_never_fails_on_a_declared_task` | Yes |

`the_inversion_gate_is_exactly_the_teardown_shift` is the one that separates this from `!`. It asserts two things in radians: that the shift is exactly `π`, and that a phase and its inversion superpose to **exactly `0.0`** — the teardown identity `f_total = 0`. A Boolean negation has no radians to check.

`inversion_is_an_involution` is labelled as passable by a Boolean language on purpose. It is true and worth pinning, but it is not evidence of anything NEOS-specific — the same honesty applied to `alignment_partitions_the_two_phase_orientations` in slice 1.

## Group 8 — scale modulation (9)

| Assertion | Would a Boolean language pass? |
|---|---|
| `identical_frequencies_detune_across_scales` | **No** — the defining property of the gate |
| `the_resonance_band_is_the_derived_one_eighth` | **No** — hits the `17/15` closed form |
| `scale_modulates_energy_through_xi` | **No** — asserts `ξ(2)/ξ(1) ≈ 0.56767` |
| `the_interference_and_resonance_gates_disagree` | **No** — the independence witness |
| `each_gate_ignores_the_other_gates_inputs` | **No** — disjoint inputs |
| `a_zero_scale_is_legal_and_maximally_corrected` | **No** — `ξ(0)` is the supremum, not an error |
| `a_collapsed_resonance_pair_is_refused` | **No** — refusal, not a default answer |
| `an_omitted_scale_is_the_reference_scale` | Partly |
| `a_negative_scale_is_refused` | Yes |

**`identical_frequencies_detune_across_scales` is the assertion the whole gate exists for.** Two oscillators at the same nominal frequency, differing only in observation scale, must fall out of resonance. Anything that passes without reading `ξ` has built a frequency comparison and called it scale modulation.

**`the_interference_and_resonance_gates_disagree` is the independence witness** from `_mkb/gates.md` §3.6, asserted in code: `A(+, 440 Hz, r=1)` and `B(−, 440 Hz, r=1)` interfere destructively **and** resonate. Two gates, same pair, opposite answers — so neither is expressible through the other and neither is redundant.

Boundaries are asserted by **straddling with exact values** (`1.18922` resonant, `1.18923` detuned), not by a tolerance around the boundary. A tolerance there would have hidden the off-by-a-rounding error the first draft of this test actually contained — see the implementation log.

## Group 9 — `ξ`'s boundedness (3, split across two suites)

`xi_stays_bounded_at_every_declarable_scale` (lang), plus `xi_is_bounded_across_the_entire_representable_domain` and `both_algebraic_forms_of_xi_agree_where_both_are_valid` (kernel).

These exist because a shipped test was **too narrow rather than wrong**. `xi_is_bounded_everywhere` sweeps `r ∈ [0, 30]` and passed continuously while `ξ` returned `+inf` above `r ≈ 710.5`. The law says *bounded*; the test checked part of the domain.

The new range is the range the law claims — everything representable, up to and including `f64::INFINITY`.

## Sabotages performed

| Mutation | Failures | Predicted? |
|---|---|---|
| revert `ξ` to the naive single expression | **5** (3 kernel, 2 lang) | yes |
| gate 3 ignores observation scale | **6** (3 kernel, 3 lang) | yes |
| widen the band from `1/8` to `1/4` | **4** (2 kernel, 2 lang) | yes |
| `invert` becomes a no-op | **7** (2 kernel, 5 lang) | yes |

All four bite, and each fails the group it was aimed at without collateral. The band-widening sabotage failing only 4 is the expected shape: most gate-3 tests use pairs far from the boundary, and only the boundary assertions and the scale-discrimination tests notice a 2× change.

**Not attempted: implementing `invert` as `-φ`.** For A2's set that is numerically identical, so it is not a mutation — the same non-sabotage as slice 1's `opposes`-as-`else`. Recorded rather than run, since the lesson was already paid for once.

## Human check

Read `identical_frequencies_detune_across_scales` and `the_interference_and_resonance_gates_disagree`. The first is what makes gate 3 *scale* modulation rather than a frequency test. The second proves the three gates are three, not one gate with three spellings.

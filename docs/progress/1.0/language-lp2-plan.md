# Language LP2 plan: the headless core

Status: plan of 15 November 2026, written when LP2 started after the LP1 sign-off
([owner decisions of 15 November 2026](../../wiki/decisions/owner-decisions-2026-11-15-freeze.md)). It records the
integrator's selections, and the owner may overrule any of them. It is no ADR and freezes nothing.

Inputs: chapter 12 sections 4 and 15.3 (PHS.18 to PHS.21, PHS.55, PHS.62 to PHS.66); chapter 02 (LG2.5,
LG2.19 to LG2.22, LG3, LG4); chapter 07 (BND.39, BND.42 to BND.44); chapter 09 (DET.54 to DET.64); chapter 11
(RTE.18, RTE.70 and the phase table); the frozen [contract vectors](../../architecture/1.0/language/contract-vectors/README.md);
the owner decisions of 14 and 15 November 2026.

Evidence kind for everything this plan produces: source and fixture. Nothing here is Windows interaction, DirectX,
long-session or performance evidence.

## 1. What LP2 delivers

PHS.18's four products: the kernel API over both puzzles, the 600-cell-Full candidate adapter, the language core
(parser, checker, typed IR and execution contract, interpreter, budgets and cooperative cancellation) and the
conformance suite for the available products. LP2 exits only when PHS.19's nine acceptance items have evidence on one
candidate and the 248 tests that section 15.3 indexes run whole (PHS.62, PHS.63, PHS.66). The three-tool loop, renderer
integration, authoring services, the research corpus and the importer stay with later phases (PHS.21).

## 2. What may start now and what waits

| Work | State | Waits for |
| --- | --- | --- |
| Typed IR draft and checker | May start; LQ21 is answered | Nothing |
| Interpreter and the `lp1-exec-draft-1` meter | Designed in M2 below | The owner's answer on the big-integer dependency (RTE.18, RTE.70) |
| Parser | Waits | LQ13 (PHS.20): the owner picks the syntax directly from a one-page comparison of the three LX-8 candidates, with no human test, and CT-RTE-18 stays recorded as not run (owner, 16 November 2026) |
| Kernel API and candidate adapter, 600-cell-Full half | May start in Rust (LQ8) | Nothing beyond the frozen chapters |
| Jumbling half of the kernel API | Waits | EN3's review and approval of the jumbling interfaces (PHS.53) |
| Session store below the language | May start | Nothing beyond chapter 05's frozen core |

## 3. Integrator selections (the owner may overrule them)

1. **Exactness form.** LP2 builds form 1 of LG2.5, `ApproxBool` and `display_if`, as HANDOFF H6 and LG2.5 recommend.
   Form 2 (`lp1-exec-region-draft-1`) stays on record and is not built. The checker and the IR advertise the
   exactness and execution contract `lp1-exec-draft-1` (LG2.21, DET.54) and refuse a program pinned to another one
   with `LG4:input.ir_version_unsupported`. CT-LG2-05's form-2 half is therefore not applicable while form 2 is not
   built; this record says so, and non-applicability is no pass (PHS.63).
2. **Where the code lives.** One Cargo workspace in `rust/`, one crate per layer, named `c600-*`. The IR crate holds
   data only; the checker and the later interpreter depend on it and not on each other. No external crate enters
   without the owner's approval (RTE.70); each approved one is listed with its licence in the workspace README.
3. **Runner, directories and records (PHS.65).** The runner is `cargo test --locked --offline` in `rust/`, which CI
   runs through `tests/test_language_rust.py`, listed in `tools/ci/run_headless.py`. Tests live in each crate's
   `tests/` folder. A test that implements part of a conformance test names the ID and the part; the ID counts as run
   only when its whole definition runs (PHS.62, PHS.63). The evidence record is the CI run of the merged commit,
   named in the pull request. No new commit guard.
4. **IR status.** The IR is a draft, `c600-ir-draft-1`, and changes freely until the IR freeze (PHS.49). Until the
   parser exists, tests build IR by hand; a hand lowering is no evidence for the parser.

## 4. Milestones

One pull request per milestone; each names the checks that ran.

| Milestone | Content | Depends on |
| --- | --- | --- |
| M0, done | CI runs the eight language checkers ([pull request 148](https://github.com/KonomiYuzu01/Magic-600-Cell/pull/148)) | - |
| M1 | IR draft and checker: exactness, effects and capabilities, views, capability separation, grants (section 5) | - |
| M2 | Interpreter and `lp1-exec-draft-1` reference meter: the 40 frozen traces and the meter boundary vectors | Big-integer answer |
| M3 | Run-time boundary: stored provenance, the fixed acknowledgement of BND.43, the saved-word half of CT-LG3-07 | M2 |
| M4 | Kernel API and 600-cell-Full candidate adapter: O01 to O07 through `tools/oracle/` | Rust engine port |
| M5 | Jumbling half: public J1 API, differential comparisons, W-J fixtures and H6030 | EN3's approval |
| M6 | Session store below the language; budgets and cancellation end to end | M2, M4 |
| M7 | Parser and its lowering into the IR | LQ13 |
| M8 | PHS.19's acceptance on Windows and headless Linux; section 15.3 runnable | M1 to M7 |

## 5. M1: the IR draft and the checker

Crates: `rust/c600-ir` (the IR draft) and `rust/c600-check` (the checker). The checker implements:

- LG2.19, LG2.20 and LG2.22: the display qualifier; ordinary control and exact operands take exact inputs only;
  `display_if` takes an `ApproxBool` and pure display branches; function, closure, aggregate, storage and export
  boundaries keep the qualifier; display accounting never reaches exact code; `exactness_violation` takes precedence
  over `exact_required`.
- LG3.1 and LG3.2: effects and capability requirements are inferred through direct, recursive, higher-order, generic
  and imported calls; an annotation cannot erase an inferred effect; an unresolved callable is not pure.
- LG3.4: the IR has no commit operation and no commit effect.
- LG3.5 and LG3.6: a view has no workspace or preview effect, also through callees.
- LG3.13, BND.39 and BND.42: in the solving profile a program that prepares previews reads no state and no preview
  outcome; helpers and other modules do not launder this; no declassifier is enabled until LQ2's joint setting.
- LG3.8 and LG3.9: file, clock and randomness need the module's declaration and the run's grant; network, process and
  session-file access are never granted.

Acceptance:

1. `cargo test --locked --offline` passes in `rust/`, in CI through `tests/test_language_rust.py`.
2. The six cases of the frozen boundary vector `direct-and-helper-condition` and the three of
   `display-accounting-no-exact-continuation`, lowered to the IR by hand, give exactly the frozen codes with the
   vector's value type and its display-control and qualifier-erasure fields. The test reads `vectors.json`, so a change
   to the vector fails it. The accounting cases come from form-2 traces; the rule they check is LG2.20's, which binds
   both forms, and M1 lowers them to form 1.
3. The admissible counterparts pass: `display_if` choosing two colours gives a display colour, and storing or exporting
   it with its qualifier is admitted (LG2.22).
4. Effect, view, capability-separation and grant tests cover the M1 parts of CT-LG2-04, CT-LG2-05, CT-LG3-01,
   CT-LG3-02, CT-LG3-03, CT-LG3-04 and CT-LG3-07, each naming its part.
5. The checker's report lists diagnostics in IR order and is the same on every run (LG4.3).
6. No file under `docs/architecture/1.0/language/` changes, and the eight language checkers still pass.

Stop condition: M1 ends when acceptance items 1 to 6 pass in CI and its pull request is merged. It stops earlier, and
the owner is told, if a rule cannot be implemented without changing a frozen file or choosing an open contract
(LG2.18's word keys, LG4.8's production tariffs, LQ2's policy options, LQ13's syntax), or if it needs a product
dependency.

## 6. Stop condition for LP2

LP2 stops and reports to the owner when every remaining milestone waits for an owner answer or another line (LQ13,
the big-integer answer, EN3's approval), or when an implementation would change a frozen file (PHS.52). A failing
acceptance item blocks LP2's exit (PHS.63); nothing is cut.

## 7. Questions put to the owner on 15 November 2026

1. The big-integer dependency for M2: `num-bigint` 0.5.1 with `num-integer` 0.1.47 and `num-traits` 0.2.19 (each
   MIT OR Apache-2.0, as read from the verified archives in LX-5's crate run; exact on LX-5's rows and about 3.4 times
   faster in total than the experiment's own limbs), or own limbs with no dependency.
2. LX-8's human-test protocol, which LQ13 and the parser waited for. Moot since 16 November 2026: the owner picks LQ13
   directly from a one-page comparison that another session writes, and the parser waits for that choice.

Questions to the owner are batched (owner, 16 November 2026): at most one question card per milestone, each with a
recommended option. The big-integer card is M2's card; other M2 points follow recorded decisions or conventional
defaults.

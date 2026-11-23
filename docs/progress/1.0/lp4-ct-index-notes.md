# LP4 conformance index: notes

`tools/language/ct_index.py --phase LP4` computes the index of chapter 12 section 15.5 (each LP4 conformance test with
its state) from the table, the PHS.62 markers in the test sources and this file. This file holds what a marker cannot
say. LP4's line main is its only editor; each note cites the record it comes from, and a state with no record has no
note.

The tool refuses (exit 2) a note for an ID outside the phase's table, an unknown note kind and a second note for one
ID. This file uses two kinds:

- `whole by union`: the parts of the ID's tests together run the whole definition, so it counts as whole.
- `waiting`: the text says what the ID waits for. It is no pass (PHS.63).

For an ID with part markers, the index keeps the markers' not-run text beside the note. The tool's other kinds are not
used here.

The rows rest on the [LP4 plan](language-lp4-plan.md), its section 1 (what each product makes runnable) and section 4
(the work items), and on the owner's decisions of
[23 November 2026](../../wiki/decisions/owner-decisions-2026-11-23-lp4-lp5-start.md).

| ID | Kind | Note |
| --- | --- | --- |
| CT-EV1-08 | whole by union | the activation service, its refusals, a run suspended at a host call on the in-process executor and invocations racing activation in `c600-module/tests/ct_ev1_08.rs`, the same run suspended in the runtime's own process and bound jobs racing activation in `c600-runtime/tests/ct_ev1_08_process.rs`, and a program job's run suspended through the binding's boundary on either host in `c600-tools/tests/ev1_host.rs` together run the whole definition (LP4 plan, LP4-2) |
| CT-EV4-06 | whole by union | the key-map checks on the specs in `c600-input/tests/keymap.rs` and the same checks through `Shell::bind` on the shell's one map in `c600-command/tests/keymap.rs` together run the whole definition, the DT-M1 US and French counterexample included (LP4 plan, LP4-8) |
| CT-EV2-07 | waiting | a checker that writes KN6.4's per-claim trace: LP5's research checkers carry one in their reports (LP5 plan, `research.check.report`); until then the steps run over a trace the test builds (LP4 plan, LP4-6) |
| CT-EV4-05 | waiting | the owner's machine: the actual Windows IME, text undo and focus loss run in the owner-machine batch that runs LP4's frozen candidate, beside the editor bars (LP4 plan, evidence kinds; decision of 23 November 2026, item 2.1) |
| CT-EV5-03 | waiting | the non-author test form, which the owner sets at minimal scale from a one-page draft once LP4-10 and LP4-11 exist (decision of 23 November 2026, item 2.2; PHS.27) |
| CT-EV5-05 | waiting | the owner's comprehension protocol, set with the non-author form at minimal scale once LP4-10 and LP4-11 exist (decision of 23 November 2026, item 2.2; PHS.27) |
| CT-LB6-04 | waiting | the owner's human-test form, set with the non-author form at minimal scale once LP4-10 and LP4-11 exist (decision of 23 November 2026, item 2.2; PHS.27) |

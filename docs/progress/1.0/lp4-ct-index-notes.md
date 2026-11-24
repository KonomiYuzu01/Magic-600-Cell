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
| CT-EV1-04 | whole by union | the broken drafts and the module set's own routes in `c600-module/tests/ct_ev1_04.rs` and the host's routes in `c600-tools/tests/ev1_host.rs` (the program job from the palette and from a key on both hosts, the view source, the debugger's route of LP4-5 and a recording's route of LP4-9) together run the whole definition on every route (LP4 plan, LP4-2, LP4-5, LP4-9) |
| CT-EV4-08 | whole by union | in `c600-command/tests/recording.rs`, what a recording retains in `ct_ev4_11_capture_appends_one_step_per_resolved_invocation` and that playback has no rebinding operation in the CT-EV4-08 test, with a source conversion through the real checker and the module set's grants in `c600-durable/tests/ps1_44_carrier.rs`, together run the whole definition (LP4 plan, LP4-9) |
| CT-EV4-12 | whole by union | in `c600-command/tests/recording.rs`, the table enumerated in `ct_ev4_12_the_eligibility_table_enumerated_whole`, the unattended rows at play in `ct_ev4_12_playback_plays_only_the_unattended_rows`, a no-job query in lane none played live and the waiting file, app, `file_check` and exclusive rows in the CT-EV4-12 live test, and filter freshness at play in the two `ct_ev4_16` tests together run the whole definition (LP4 plan, LP4-9) |
| CT-EV4-16 | whole by union | in `c600-command/tests/recording.rs`, stale steps and identity refusals in `ct_ev4_16_stale_steps_stop_and_identities_refuse` and a kept filter preview gone stale, stopped without a retarget, in `ct_ev4_16_a_kept_filter_preview_gone_stale_stops_unretargeted` together run the whole definition (LP4 plan, LP4-9) |
| CT-EV4-19 | whole by union | in `c600-command/tests/recording.rs`, the play table's refusals outside it, the rewind with an empty map, W, rewind, W, undo, the play's end, `job.stop` and each child's new receipt binding in `ct_ev4_19_the_play_table_rewinds_and_job_stop`, a start while recording and a stop with none in `ct_ev4_11_capture_appends_one_step_per_resolved_invocation`, and each table row's outcome after reconciliation in `the_format_packets_playback_vectors` together run the whole definition (LP4 plan, LP4-9) |
| CT-ERR-18 | whole by union | in `c600-command/tests/recording.rs`, the codes' identities, their continuations (a stale step's is `after_refresh`) and a forwarded child failure in `ct_err_18_recording_codes_keep_their_identities`, stale stops and identity refusals before and during play in `ct_ev4_16_stale_steps_stop_and_identities_refuse`, unknown outcomes stopping play and conversion in `ct_ev4_13_every_historical_and_child_outcome`, conversion refusals in `ct_ev4_17_the_five_conversion_vectors_on_both_targets` and state refusals in `ct_ev4_19_the_play_table_rewinds_and_job_stop` together run the whole definition (LP4 plan, LP4-9) |
| CT-PS1-15 | whole by union | in `c600-command/tests/recording.rs`, the round trip of every `RecordedStep` field in `ct_ev4_11_capture_appends_one_step_per_resolved_invocation`, commit playback with a new explicit action and token in `ct_ev4_09_no_token_is_stored_or_replayed` and steps with a module digest and a word block in the CT-PS1-15 test, with the same steps through the session journal in `c600-session/tests/recorder.rs`, together run the whole definition (LP4 plan, LP4-9) |
| CT-BND-03 | whole by union | the workspace record and file routes in `c600-run/tests/boundary.rs`, a recording and both conversions, draft and source, in `c600-durable/tests/ct_bnd_03_recording_blocks.rs` and `c600-command/tests/recording.rs`, and the macro route, a macro of a stored word keeping the word's block through a reopen, in `c600-command/tests/rebind_and_macro_base.rs` together run the whole definition (LP4 plan, LP4-9; the macro route on the line, 24 November 2026) |
| CT-EV2-07 | waiting | a checker that writes KN6.4's per-claim trace: LP5's research checkers carry one in their reports (LP5 plan, `research.check.report`); until then the steps run over a trace the test builds (LP4 plan, LP4-6) |
| CT-EV4-05 | waiting | the owner's machine: the actual Windows IME, text undo and focus loss run in the owner-machine batch that runs LP4's frozen candidate, beside the editor bars (LP4 plan, evidence kinds; decision of 23 November 2026, item 2.1) |
| CT-EV5-03 | waiting | the non-author test form, which the owner sets at minimal scale from a one-page draft once LP4-10 and LP4-11 exist (decision of 23 November 2026, item 2.2; PHS.27) |
| CT-EV5-05 | waiting | the owner's comprehension protocol, set with the non-author form at minimal scale once LP4-10 and LP4-11 exist (decision of 23 November 2026, item 2.2; PHS.27) |
| CT-LB6-04 | waiting | the owner's human-test form, set with the non-author form at minimal scale once LP4-10 and LP4-11 exist (decision of 23 November 2026, item 2.2; PHS.27) |

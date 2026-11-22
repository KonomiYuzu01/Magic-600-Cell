# LP2 conformance index: notes

`tools/language/ct_index.py` computes the index of chapter 12 section 15.3 (each LP2 conformance test with its state)
from the table, the PHS.62 markers in the test sources and this file. This file holds what a marker cannot say. The LP2
integrator is its only editor; each note cites the record it comes from, and a state with no record has no note.

The tool refuses (exit 2) a note for an ID outside the phase's table and an unknown note kind. The kinds:

- `whole by union`: the parts of the ID's tests together run the whole definition, so it counts as whole.
- `not applicable`: the definition does not apply to the build; it is no pass (PHS.63).
- `waiting`: what does not run waits for a decision or a review; the text says for what. It applies to an ID that does
  not run whole, and it is no pass.
- `indexed elsewhere`: an owner-approved index change (PHS.52) places what LP2's products cannot run with the work
  that builds its piece; the text says where. It applies to an ID that does not run whole, and it is no pass.
- `missing despite name`: only documentation names the ID, so it counts as missing.

For an ID with part markers, the index keeps the markers' not-run text beside the note. The rows below the union rows
record the owner's readings of LP2's exit list
([22 November 2026](../../wiki/decisions/owner-decisions-2026-11-22-lx-bars-freeze.md), item 8; LP2 plan section 12).

After LP2's exit on `0260e6b1`, the owner's sign-off of C10 and LP2's exit
([22 November 2026](../../wiki/decisions/owner-decisions-2026-11-22-c10-lp2-exit.md); "the exit sign-off" below)
changes two things here:

- Its reading (a) moves reading 17's owner-machine halves (CT-KN4-01, CT-KN5-05, CT-LG4-08, CT-PS1-05, CT-ONE-20,
  CT-DET-05, CT-DET-07 and CT-DET-16) from LP3's owner-machine batch to the owner-machine sitting of the post-freeze
  on-screen line, the line that builds the device harnesses and RPI.77's quarantine after the freeze. They stay
  `indexed elsewhere`, and nothing in them counts as passed in LP2.
- Reading 18's undo fix, which LP3's single pull request (417) carries from pull request 416, makes CT-LG3-03 and
  CT-LB7-01 whole by union. Their rows close the union rows.

From that change on, the tool prints 128 whole (34 by union), 7 not applicable, 8 waiting and 105 indexed elsewhere. On
`0260e6b1` it still prints LP2's exit count: 126 whole (32 by union), 7 not applicable, 8 waiting and 107 indexed
elsewhere.

| ID | note kind | text |
| --- | --- | --- |
| CT-LG2-04 | whole by union | the `mismatch.rs` and `exactness.rs` tests together run the whole definition (`rust/README.md`, the count line) |
| CT-LG3-01 | whole by union | the effects test and the next test together run the whole row (`rust/README.md`, the count line) |
| CT-LG3-04 | whole by union | its parts together run the whole definition (`rust/README.md`, the count line) |
| CT-LG5-06 | whole by union | the fork half in `c600-module/tests/conformance_lg5.rs` and the evidence half in `c600-evidence/tests/sinks.rs` together run the whole definition (`rust/README.md`, the count line) |
| CT-KN1-05 | whole by union | M4's labels half and M5's poses half (`rust/README.md`, the count line; LP2 plan section 11) |
| CT-KN2-01 | whole by union | M4's half and M5's jumbling half (`rust/README.md`, the count line; LP2 plan section 11) |
| CT-KN2-04 | whole by union | with M5 its parts together run the whole definition (`rust/README.md`, the count line) |
| CT-KN2-05 | whole by union | with M5 its parts together run the whole definition (`rust/README.md`, the count line) |
| CT-KN1-04 | whole by union | M4's 600-cell-Full half (`c600-kernel/tests/kernel.rs`) and M8's jumbling half (`c600-jumble/tests/levels.rs`) (`rust/README.md`, the count line) |
| CT-PS1-11 | whole by union | the stored-word half and the journal half in `c600-session/tests/persist_jumble.rs` together run the whole definition (`rust/README.md`, the count line) |
| CT-PS1-22 | whole by union | the never-on half and the interrupted-crossing half in `c600-session/tests/persist_jumble.rs` together run the whole definition (`rust/README.md`, the count line) |
| CT-LG2-12 | whole by union | the language halves in `c600-syntax` and `c600-run`'s `collections.rs`, the kernel's literal-prefix half in `c600-kernel/tests/load.rs` and the jumbling fixtures under LG2.14's four hypotheses in `c600-jumble-tool/tests/lb3.rs` together run the whole definition (`rust/README.md`, the count line) |
| CT-LG2-13 | whole by union | the empty-word halves in `c600-syntax` and `c600-run`'s `conformance_lg2.rs` and the jumbling key at a straddling cap in `c600-jumble-tool/tests/lb3.rs` together run the whole definition (`rust/README.md`, the count line) |
| CT-LG2-14 | whole by union | the exact length and positions in `c600-syntax` and `c600-run`'s `collections.rs` and the kernel's streaming in `c600-kernel/tests/kernel.rs` together run the whole definition (`rust/README.md`, the count line) |
| CT-KN5-12 | whole by union | the preview lifecycle and the ended states in `c600-command/tests/layer.rs`, with the stale previews of `kn5_stages.rs` and `kn5_admission.rs`, together run the whole definition (`rust/README.md`, the count line) |
| CT-LI-04 | whole by union | the undo during a query, the superseded hint in `c600-command/tests/layer.rs` and the protection release through the solver's confirmation in `attack_protection.rs` together run the whole definition (`rust/README.md`, the count line) |
| CT-LI-13 | whole by union | the instance ceiling in `c600-view/tests/view_rows.rs` and the renderer loss in `c600-display/tests/publication.rs` together run the whole definition (`rust/README.md`, the count line) |
| CT-ONE-02 | whole by union | the runtime's half in `c600-run/tests/tables.rs`, the session's rows in `c600-session/tests/conformance_rows.rs` and every row of the command layer's table, overlay rows among them, in every profile in `c600-command/tests/profile.rs` together run the whole definition (`rust/README.md`, the count line) |
| CT-KN5-02 | whole by union | the runtime's half in `c600-run/tests/tables.rs`, the session's rows in `c600-session/tests/conformance_rows.rs` and every row of the command layer's table, overlay rows among them, in every profile in `c600-command/tests/profile.rs` together run the whole definition (`rust/README.md`, the count line) |
| CT-LG3-02 | whole by union | the program's effects in `c600-check`, `c600-syntax` and `c600-run` and the overlay and served rows in `c600-command/tests/layer.rs` and `served_rows.rs` together run the whole definition (`rust/README.md`, the count line) |
| CT-LI-07 | whole by union | the convergence by full reads, the optional and required rows and the absence of language code in `c600-command/tests/availability.rs` together run the whole definition (`rust/README.md`, the count line) |
| CT-KN1-01 | whole by union | the retained half in `c600-kernel/tests/kernel.rs` and `load.rs`, the jumbling half in `c600-jumble/tests/levels.rs` and the store's half in `c600-session/tests/persist_jumble.rs` refuse every value of another model, so together they run the whole definition (`rust/README.md`, the count line) |
| CT-BND-14 | whole by union | the engine and program half in `c600-run/tests/conformance_bnd.rs` and `boundary.rs` and the command layer's refusal of a run's preview with the session's digest and revision unchanged in `c600-command/tests/profile.rs` together run the whole definition (`rust/README.md`, the count line) |
| CT-ONE-01 | whole by union | the runtime's table in `c600-run/tests/tables.rs`, the session's rows in `c600-session/tests/conformance_rows.rs` and every H-08 row in `c600-command/tests/layer.rs`'s `no_row_takes_a_state_or_a_digest_to_adopt` together run the whole definition (`rust/README.md`, the count line) |
| CT-ONE-03 | whole by union | the token matrix of a word commit in `c600-command/tests/kn5.rs`, the commit with no presented preview in `layer.rs`, the live twists in `twist_apply.rs` and `jumble_twist_menu.rs` and the crossings in `jumble_on.rs` and `jumble_off.rs` together run the whole definition (`rust/README.md`, the count line) |
| CT-ERR-09 | whole by union | the six commit-step fixtures in `c600-command/tests/layer.rs` and `kn5.rs`, each with the state unchanged, and the codes' forms and retries in `tests/test_error_registry.py` together run the whole definition (`rust/README.md`, the count line) |
| CT-BND-27 | whole by union | the journal half in `c600-session/tests/part_blocks.rs`, the layer half in `c600-command/tests/part_blocks.rs`, the reader in `c600-tools/tests/workspace.rs` and the file-system disk in `c600-durable/tests/ct_rte_08_persisted_word.rs` together run the whole definition (`rust/README.md`, the count line) |
| CT-LG5-08 | whole by union | the syntax family, the stage checker and the three profiles in `c600-syntax/tests/stages.rs` and the digest, lock and grant checks of a stage module in `c600-module/tests/conformance_lg5.rs` together run the whole definition (`rust/README.md`, the count line) |
| CT-LB7-02 | whole by union | grants, profiles, storage and forks in `c600-run/tests/conformance_lg3.rs` and the key binding in `c600-input/tests/routing.rs` together run the whole definition (`rust/README.md`, the count line) |
| CT-LI-06 | whole by union | the presentation permission of a completed preview in `c600-input/tests/routing.rs`, atomic installation in `c600-view/tests/view_rows.rs` and the derived plan checked against the session head in `c600-tools/tests/views.rs` together run the whole definition (`rust/README.md`, the count line) |
| CT-RPI-01 | whole by union | the boundary enumeration in `c600-run/tests/tables.rs` and the backend switch with its comparison helper in `c600-render/tests/ct_rpi_01.rs` together run the whole definition (`rust/README.md`, the count line) |
| CT-EVD-21 | whole by union | the evidence library in `c600-evidence/tests/sinks.rs`, the policy, grants and admission in `c600-run/tests/tables.rs` and the commit paths of the command layer in `c600-command/tests/kn5_stages.rs` and of the session store in `c600-session/tests/commit.rs` together run the whole definition (`rust/README.md`, the count line) |
| CT-LG3-03 | whole by union | the static halves in source (`c600-syntax/tests/conformance_lg3.rs`) and in IR (`c600-check/tests/effects.rs` and `limits.rs`) and the command layer's halves in `c600-command`: a workspace row's completion as one durable action and a view row's job that writes nothing (`tests/job_output.rs`), and one undoable action per invoked workspace change, for a stored row (`tests/filter_apply.rs`) and for a language-defined row (`tests/job_output.rs`), together run the whole definition; the last is reading 18's undo fix, after LP2's exit (pull request 416, in LP3's pull request 417; the exit sign-off) |
| CT-LB7-01 | whole by union | views, save/load and import in `c600-run/tests/conformance_lg3.rs`, the missing engine preview of a research or authoring run in `c600-command/tests/profile.rs`, and in `c600-command` a view row's job that writes nothing and one undoable action per invoked workspace command, for a stored row (`tests/filter_apply.rs`) and for a language-defined row (`tests/job_output.rs`), together run the whole definition; the last is reading 18's undo fix, after LP2's exit (pull request 416, in LP3's pull request 417; the exit sign-off) |
| CT-KN1-06 | indexed elsewhere | indexed with the kernel or jumbling work that adds the call (reading 16) (owner, 22 November 2026, item 8) |
| CT-KN2-07 | indexed elsewhere | indexed with the kernel or jumbling work that adds the call (reading 16) (owner, 22 November 2026, item 8) |
| CT-KN3-01 | indexed elsewhere | indexed with the kernel or jumbling work that adds the call (reading 16) (owner, 22 November 2026, item 8) |
| CT-KN3-03 | indexed elsewhere | indexed with the kernel or jumbling work that adds the call (reading 16) (owner, 22 November 2026, item 8) |
| CT-KN3-04 | not applicable | a search for N ≥ 1 near the limit, which is research-only and has no budget: not applicable until the research profile budgets it (reading 4) (owner, 22 November 2026, item 8) |
| CT-KN3-06 | indexed elsewhere | indexed with LP3's host, shell, display and renderer (reading 9) (owner, 22 November 2026, item 8) |
| CT-KN4-01 | indexed elsewhere | the owner-machine halves are indexed in the owner-machine sitting of the post-freeze on-screen line (the exit sign-off's reading (a)), and other platforms come after 1.0 (reading 17) (owner, 22 November 2026, item 8) |
| CT-KN5-05 | indexed elsewhere | the owner-machine halves are indexed in the owner-machine sitting of the post-freeze on-screen line (the exit sign-off's reading (a)), and other platforms come after 1.0 (reading 17) (owner, 22 November 2026, item 8) |
| CT-KN5-13 | indexed elsewhere | indexed with the store work that builds each row, M6's follow-up and LP3's command table (reading 12) (owner, 22 November 2026, item 8) |
| CT-KN6-02 | indexed elsewhere | indexed in LP5 with its checkers, KN6.1's five kinds (reading 7) (owner, 22 November 2026, item 8) |
| CT-LG1-01 | not applicable | mixed supported dependency editions: not applicable while `6cs-draft-1` is the only supported edition, reopened when a second one is (reading 3) (owner, 22 November 2026, item 8) |
| CT-LG1-03 | indexed elsewhere | indexed in LP4 with its editor services (EV1, chapter 04) (reading 6) (owner, 22 November 2026, item 8) |
| CT-LG2-01 | indexed elsewhere | indexed with the IR's freeze, where the missing constructs land (reading 14) (owner, 22 November 2026, item 8) |
| CT-LG2-02 | indexed elsewhere | indexed with the IR's freeze, where the missing constructs land (reading 14) (owner, 22 November 2026, item 8) |
| CT-LG2-03 | indexed elsewhere | indexed with the IR's freeze, where the missing constructs land (reading 14) (owner, 22 November 2026, item 8) |
| CT-LG2-05 | indexed elsewhere | form 2 (LG2.21) stays out of LP2 until it is built, under section 15.3's PHS.52 amendment (reading 8) (owner, 22 November 2026, item 8) |
| CT-LG2-06 | indexed elsewhere | indexed with LP3's host, shell, display and renderer (reading 9) (owner, 22 November 2026, item 8) |
| CT-LG2-07 | indexed elsewhere | indexed with the IR's freeze, where the missing constructs land (reading 14) (owner, 22 November 2026, item 8) |
| CT-LG2-08 | indexed elsewhere | indexed with LP3's host, shell, display and renderer (reading 9) (owner, 22 November 2026, item 8) |
| CT-LG2-09 | indexed elsewhere | indexed with the IR's freeze, where the missing constructs land (reading 14) (owner, 22 November 2026, item 8) |
| CT-LG2-10 | indexed elsewhere | indexed with the IR's freeze, where the missing constructs land (reading 14) (owner, 22 November 2026, item 8) |
| CT-LG2-11 | indexed elsewhere | indexed with the IR's freeze, where the missing constructs land (reading 14) (owner, 22 November 2026, item 8) |
| CT-LG2-15 | indexed elsewhere | indexed with LP3's host, shell, display and renderer (reading 9) (owner, 22 November 2026, item 8) |
| CT-LG2-16 | indexed elsewhere | indexed with the IR's freeze, where the missing constructs land (reading 14) (owner, 22 November 2026, item 8) |
| CT-LG3-05 | indexed elsewhere | indexed in LP5 with its checkers, KN6.1's five kinds (reading 7) (owner, 22 November 2026, item 8) |
| CT-LG3-06 | waiting | waits for LQ2's joint setting (BND.41's flow rule, BND.55's label rule) and the built-in tool class review (BND.37), and runs when they land (reading 5) (owner, 22 November 2026, item 8) |
| CT-LG3-07 | waiting | waits for LQ2's joint setting (BND.41's flow rule, BND.55's label rule) and the built-in tool class review (BND.37), and runs when they land (reading 5) (owner, 22 November 2026, item 8) |
| CT-LG3-09 | indexed elsewhere | indexed with the store work that builds each row, M6's follow-up and LP3's command table (reading 12) (owner, 22 November 2026, item 8) |
| CT-LG4-01 | not applicable | a second admitted route: not applicable while RT1 is the only admitted route (LQ9), reopened if another route is admitted (reading 1) (owner, 22 November 2026, item 8) |
| CT-LG4-02 | indexed elsewhere | indexed with the kernel or jumbling work that adds the call (reading 16) (owner, 22 November 2026, item 8) |
| CT-LG4-03 | indexed elsewhere | indexed with the kernel or jumbling work that adds the call (reading 16) (owner, 22 November 2026, item 8) |
| CT-LG4-04 | indexed elsewhere | indexed with the IR's freeze, where the missing constructs land (reading 14) (owner, 22 November 2026, item 8) |
| CT-LG4-06 | indexed elsewhere | indexed with LP3's host, shell, display and renderer (reading 9) (owner, 22 November 2026, item 8) |
| CT-LG4-08 | indexed elsewhere | indexed with the kernel or jumbling work that adds the call (reading 16); the owner-machine halves are indexed in the owner-machine sitting of the post-freeze on-screen line (the exit sign-off's reading (a)), and other platforms come after 1.0 (reading 17) (owner, 22 November 2026, item 8) |
| CT-LG5-02 | indexed elsewhere | indexed in LP5 with its checkers, KN6.1's five kinds (reading 7) (owner, 22 November 2026, item 8) |
| CT-LG5-03 | indexed elsewhere | indexed with the store work that builds each row, M6's follow-up and LP3's command table (reading 12) (owner, 22 November 2026, item 8) |
| CT-LG5-04 | indexed elsewhere | indexed with the IR's freeze, where the missing constructs land (reading 14) (owner, 22 November 2026, item 8) |
| CT-LG5-05 | indexed elsewhere | indexed in LP4 with its editor services (EV1, chapter 04) (reading 6) (owner, 22 November 2026, item 8) |
| CT-LG5-07 | indexed elsewhere | indexed with the IR's freeze, where the missing constructs land (reading 14) (owner, 22 November 2026, item 8) |
| CT-LB6-02 | indexed elsewhere | indexed with chapter 03's library work (reading 15) (owner, 22 November 2026, item 8) |
| CT-PS1-02 | indexed elsewhere | indexed with the store work that builds each row, M6's follow-up and LP3's command table (reading 12) (owner, 22 November 2026, item 8) |
| CT-PS1-03 | indexed elsewhere | indexed with the store work that builds each row, M6's follow-up and LP3's command table (reading 12) (owner, 22 November 2026, item 8) |
| CT-PS1-04 | indexed elsewhere | indexed with the kernel or jumbling work that adds the call (reading 16) (owner, 22 November 2026, item 8) |
| CT-PS1-05 | indexed elsewhere | the owner-machine halves are indexed in the owner-machine sitting of the post-freeze on-screen line (the exit sign-off's reading (a)), and other platforms come after 1.0 (reading 17) (owner, 22 November 2026, item 8) |
| CT-PS1-06 | indexed elsewhere | indexed with the kernel or jumbling work that adds the call (reading 16) (owner, 22 November 2026, item 8) |
| CT-PS1-07 | indexed elsewhere | indexed with the store work that builds each row, M6's follow-up and LP3's command table (reading 12); indexed with the importer, once the owner places it (reading 13) (owner, 22 November 2026, item 8) |
| CT-PS1-13 | indexed elsewhere | indexed with the store work that builds each row, M6's follow-up and LP3's command table (reading 12) (owner, 22 November 2026, item 8) |
| CT-PS1-14 | indexed elsewhere | indexed with the store work that builds each row, M6's follow-up and LP3's command table (reading 12) (owner, 22 November 2026, item 8) |
| CT-PS1-18 | indexed elsewhere | indexed with the store work that builds each row, M6's follow-up and LP3's command table (reading 12) (owner, 22 November 2026, item 8) |
| CT-PS1-27 | indexed elsewhere | indexed with chapter 03's library work (reading 15) (owner, 22 November 2026, item 8) |
| CT-ONE-04 | indexed elsewhere | indexed with LP3's host, shell, display and renderer (reading 9) (owner, 22 November 2026, item 8) |
| CT-ONE-09 | indexed elsewhere | indexed with LP3's host, shell, display and renderer (reading 9); indexed with the store work that builds each row, M6's follow-up and LP3's command table (reading 12) (owner, 22 November 2026, item 8) |
| CT-ONE-12 | indexed elsewhere | indexed with LP3's host, which serves the frozen H-08 rows and carries ERR.8's envelope (reading 10) (owner, 22 November 2026, item 8) |
| CT-ONE-14 | indexed elsewhere | indexed with LP3's host, shell, display and renderer (reading 9) (owner, 22 November 2026, item 8) |
| CT-ONE-17 | indexed elsewhere | indexed with LP3's host, shell, display and renderer (reading 9) (owner, 22 November 2026, item 8) |
| CT-ONE-18 | indexed elsewhere | indexed with the kernel or jumbling work that adds the call (reading 16) (owner, 22 November 2026, item 8) |
| CT-ONE-19 | indexed elsewhere | indexed with the kernel or jumbling work that adds the call (reading 16) (owner, 22 November 2026, item 8) |
| CT-ONE-20 | indexed elsewhere | indexed with the store work that builds each row, M6's follow-up and LP3's command table (reading 12); the owner-machine halves are indexed in the owner-machine sitting of the post-freeze on-screen line (the exit sign-off's reading (a)), and other platforms come after 1.0 (reading 17) (owner, 22 November 2026, item 8) |
| CT-ONE-21 | indexed elsewhere | indexed with the IR's freeze, where the missing constructs land (reading 14) (owner, 22 November 2026, item 8) |
| CT-ONE-22 | indexed elsewhere | indexed with LP3's host, shell, display and renderer (reading 9) (owner, 22 November 2026, item 8) |
| CT-ONE-24 | indexed elsewhere | indexed with LP3's host, shell, display and renderer (reading 9) (owner, 22 November 2026, item 8) |
| CT-ONE-28 | indexed elsewhere | indexed with LP3's host, shell, display and renderer (reading 9); indexed with the store work that builds each row, M6's follow-up and LP3's command table (reading 12) (owner, 22 November 2026, item 8) |
| CT-DET-01 | indexed elsewhere | indexed in LP5 with its checkers, KN6.1's five kinds (reading 7) (owner, 22 November 2026, item 8) |
| CT-DET-02 | indexed elsewhere | indexed in LP5 with its checkers, KN6.1's five kinds (reading 7) (owner, 22 November 2026, item 8) |
| CT-DET-05 | indexed elsewhere | the owner-machine halves are indexed in the owner-machine sitting of the post-freeze on-screen line (the exit sign-off's reading (a)), and other platforms come after 1.0 (reading 17) (owner, 22 November 2026, item 8) |
| CT-DET-06 | indexed elsewhere | indexed with the IR's freeze, where the missing constructs land (reading 14); its RT2 and RT3 halves: not applicable while RT1 is the only admitted route (LQ9), reopened if another route is admitted (reading 1) (owner, 22 November 2026, item 8) |
| CT-DET-07 | indexed elsewhere | indexed with the store work that builds each row, M6's follow-up and LP3's command table (reading 12); the owner-machine halves are indexed in the owner-machine sitting of the post-freeze on-screen line (the exit sign-off's reading (a)), and other platforms come after 1.0 (reading 17) (owner, 22 November 2026, item 8) |
| CT-DET-08 | indexed elsewhere | indexed with the IR's freeze, where the missing constructs land (reading 14) (owner, 22 November 2026, item 8) |
| CT-DET-09 | indexed elsewhere | indexed with the store work that builds each row, M6's follow-up and LP3's command table (reading 12); its RT2 and RT3 halves: not applicable while RT1 is the only admitted route (LQ9), reopened if another route is admitted (reading 1) (owner, 22 November 2026, item 8) |
| CT-DET-10 | not applicable | its RT2 and RT3 halves: not applicable while RT1 is the only admitted route (LQ9), reopened if another route is admitted (reading 1) (owner, 22 November 2026, item 8) |
| CT-DET-11 | indexed elsewhere | indexed in LP5 with its checkers, KN6.1's five kinds (reading 7) (owner, 22 November 2026, item 8) |
| CT-DET-12 | indexed elsewhere | indexed with the IR's freeze, where the missing constructs land (reading 14) (owner, 22 November 2026, item 8) |
| CT-DET-13 | indexed elsewhere | indexed with the store work that builds each row, M6's follow-up and LP3's command table (reading 12); indexed with the IR's freeze, where the missing constructs land (reading 14) (owner, 22 November 2026, item 8) |
| CT-DET-17 | indexed elsewhere | indexed with LP3's host, shell, display and renderer (reading 9); its RT2 and RT3 halves: not applicable while RT1 is the only admitted route (LQ9), reopened if another route is admitted (reading 1) (owner, 22 November 2026, item 8) |
| CT-DET-18 | indexed elsewhere | indexed with LP3's host, shell, display and renderer (reading 9) (owner, 22 November 2026, item 8) |
| CT-DET-20 | indexed elsewhere | indexed with the store work that builds each row, M6's follow-up and LP3's command table (reading 12) (owner, 22 November 2026, item 8) |
| CT-DET-23 | indexed elsewhere | indexed with the store work that builds each row, M6's follow-up and LP3's command table (reading 12) (owner, 22 November 2026, item 8) |
| CT-DET-26 | not applicable | its RT2 and RT3 halves: not applicable while RT1 is the only admitted route (LQ9), reopened if another route is admitted (reading 1) (owner, 22 November 2026, item 8) |
| CT-DET-27 | indexed elsewhere | indexed with the store work that builds each row, M6's follow-up and LP3's command table (reading 12); its RT2 and RT3 halves: not applicable while RT1 is the only admitted route (LQ9), reopened if another route is admitted (reading 1) (owner, 22 November 2026, item 8) |
| CT-DET-29 | indexed elsewhere | indexed with the IR's freeze, where the missing constructs land (reading 14); its RT2 and RT3 halves: not applicable while RT1 is the only admitted route (LQ9), reopened if another route is admitted (reading 1) (owner, 22 November 2026, item 8) |
| CT-DET-30 | not applicable | its RT2 and RT3 halves, transport splits and cold caches among them, which RT1's in-process stream does not have: not applicable while RT1 is the only admitted route (LQ9), reopened if another route is admitted (reading 1) (owner, 22 November 2026, item 8) |
| CT-DET-31 | indexed elsewhere | indexed with LP3's host, shell, display and renderer (reading 9); its RT2 and RT3 halves: not applicable while RT1 is the only admitted route (LQ9), reopened if another route is admitted (reading 1) (owner, 22 November 2026, item 8) |
| CT-DET-32 | indexed elsewhere | form 2 (LG2.21) stays out of LP2 until it is built, under section 15.3's PHS.52 amendment (reading 8) (owner, 22 November 2026, item 8) |
| CT-DET-16 | indexed elsewhere | indexed with LP3's host, shell, display and renderer (reading 9); the owner-machine halves are indexed in the owner-machine sitting of the post-freeze on-screen line (the exit sign-off's reading (a)), and other platforms come after 1.0 (reading 17) (owner, 22 November 2026, item 8) |
| CT-DET-28 | indexed elsewhere | indexed with the IR's freeze, where the missing constructs land (reading 14); its RT2 and RT3 halves: not applicable while RT1 is the only admitted route (LQ9), reopened if another route is admitted (reading 1) (owner, 22 November 2026, item 8) |
| CT-RPI-20 | indexed elsewhere | indexed with LP3's host, shell, display and renderer (reading 9) (owner, 22 November 2026, item 8) |
| CT-EVD-17 | indexed elsewhere | indexed with the importer, once the owner places it (reading 13) (owner, 22 November 2026, item 8) |
| CT-EVD-29 | indexed elsewhere | indexed with chapter 03's library work (reading 15) (owner, 22 November 2026, item 8) |
| CT-EVD-30 | indexed elsewhere | indexed in LP5 with its checkers, KN6.1's five kinds (reading 7); indexed with LP3's host, shell, display and renderer (reading 9) (owner, 22 November 2026, item 8) |
| CT-EVD-10 | indexed elsewhere | indexed with the kernel or jumbling work that adds the call (reading 16) (owner, 22 November 2026, item 8) |
| CT-BND-02 | indexed elsewhere | indexed with the store work that builds each row, M6's follow-up and LP3's command table (reading 12) (owner, 22 November 2026, item 8) |
| CT-BND-05 | waiting | waits for LQ2's joint setting (BND.41's flow rule, BND.55's label rule) and the built-in tool class review (BND.37), and runs when they land (reading 5) (owner, 22 November 2026, item 8) |
| CT-BND-06 | waiting | waits for LQ2's joint setting (BND.41's flow rule, BND.55's label rule) and the built-in tool class review (BND.37), and runs when they land (reading 5) (owner, 22 November 2026, item 8) |
| CT-BND-08 | waiting | waits for LQ2's joint setting (BND.41's flow rule, BND.55's label rule) and the built-in tool class review (BND.37), and runs when they land (reading 5) (owner, 22 November 2026, item 8) |
| CT-BND-09 | waiting | waits for LQ2's joint setting (BND.41's flow rule, BND.55's label rule) and the built-in tool class review (BND.37), and runs when they land (reading 5) (owner, 22 November 2026, item 8) |
| CT-BND-10 | waiting | waits for LQ2's joint setting (BND.41's flow rule, BND.55's label rule) and the built-in tool class review (BND.37), and runs when they land (reading 5) (owner, 22 November 2026, item 8) |
| CT-BND-11 | not applicable | ONE.32's fields in the tool-authoring and research profiles, which never arise there because BND.60 sends every proposal in those profiles to a capture: not applicable while BND.60 stands; the solving profile's half runs (reading 2) (owner, 22 November 2026, item 8) |
| CT-BND-12 | indexed elsewhere | indexed with LP3's host, shell, display and renderer (reading 9) (owner, 22 November 2026, item 8) |
| CT-BND-22 | indexed elsewhere | indexed with the importer, once the owner places it (reading 13) (owner, 22 November 2026, item 8) |
| CT-BND-26 | indexed elsewhere | indexed with the store work that builds each row, M6's follow-up and LP3's command table (reading 12) (owner, 22 November 2026, item 8) |
| CT-BND-29 | waiting | waits for LQ2's joint setting (BND.41's flow rule, BND.55's label rule) and the built-in tool class review (BND.37), and runs when they land (reading 5) (owner, 22 November 2026, item 8) |
| CT-BND-30 | indexed elsewhere | indexed with the store work that builds each row, M6's follow-up and LP3's command table (reading 12) (owner, 22 November 2026, item 8) |
| CT-ERR-01 | indexed elsewhere | indexed with LP3's host, which serves the frozen H-08 rows and carries ERR.8's envelope (reading 10) (owner, 22 November 2026, item 8) |
| CT-ERR-05 | indexed elsewhere | indexed in LP4 with its editor services (EV1, chapter 04) (reading 6) (owner, 22 November 2026, item 8) |
| CT-ERR-06 | indexed elsewhere | indexed with LP3's host, which serves the frozen H-08 rows and carries ERR.8's envelope (reading 10) (owner, 22 November 2026, item 8) |
| CT-ERR-07 | indexed elsewhere | indexed with the store work that builds each row, M6's follow-up and LP3's command table (reading 12) (owner, 22 November 2026, item 8) |
| CT-ERR-08 | indexed elsewhere | indexed with LP3's host, shell, display and renderer (reading 9) (owner, 22 November 2026, item 8) |
| CT-ERR-10 | indexed elsewhere | indexed with the importer, once the owner places it (reading 13) (owner, 22 November 2026, item 8) |
| CT-ERR-12 | indexed elsewhere | indexed in LP4 with its editor services (EV1, chapter 04) (reading 6) (owner, 22 November 2026, item 8) |
| CT-ERR-13 | indexed elsewhere | indexed in LP4 with its editor services (EV1, chapter 04) (reading 6) (owner, 22 November 2026, item 8) |
| CT-ERR-14 | indexed elsewhere | indexed with LP3's host, which serves the frozen H-08 rows and carries ERR.8's envelope (reading 10) (owner, 22 November 2026, item 8) |
| CT-ERR-15 | indexed elsewhere | indexed with LP3's host, shell, display and renderer (reading 9) (owner, 22 November 2026, item 8) |
| CT-ERR-16 | indexed elsewhere | indexed with the IR's freeze, where the missing constructs land (reading 14) (owner, 22 November 2026, item 8) |
| CT-ERR-17 | indexed elsewhere | indexed with LP3's host, shell, display and renderer (reading 9) (owner, 22 November 2026, item 8) |
| CT-LI-02 | indexed elsewhere | indexed with LP3's host, which serves the frozen H-08 rows and carries ERR.8's envelope (reading 10) (owner, 22 November 2026, item 8) |
| CT-LI-03 | indexed elsewhere | indexed with LP3's host, which serves the frozen H-08 rows and carries ERR.8's envelope (reading 10) (owner, 22 November 2026, item 8) |
| CT-LI-05 | indexed elsewhere | indexed with the store work that builds each row, M6's follow-up and LP3's command table (reading 12) (owner, 22 November 2026, item 8) |
| CT-LI-09 | indexed elsewhere | indexed with the store work that builds each row, M6's follow-up and LP3's command table (reading 12) (owner, 22 November 2026, item 8) |
| CT-LI-12 | indexed elsewhere | indexed with LP3's host, which serves the frozen H-08 rows and carries ERR.8's envelope (reading 10) (owner, 22 November 2026, item 8) |
| CT-LI-14 | indexed elsewhere | an output-exhausted `jumble.observe`, whose row no crate serves: indexed with LP3's host, which serves the frozen H-08 rows (reading 10) (owner, 22 November 2026, item 8) |

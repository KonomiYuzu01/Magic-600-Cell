# LP2 conformance index: notes

`tools/language/ct_index.py` computes the index of chapter 12 section 15.3 (each LP2 conformance test with its state)
from the table, the PHS.62 markers in the test sources and this file. This file holds what a marker cannot say. The LP2
integrator is its only editor; each note cites the record it comes from, and a state with no record has no note.

The tool refuses (exit 2) a note for an ID outside the phase's table and an unknown note kind. The kinds:

- `whole by union`: the parts of the ID's tests together run the whole definition, so it counts as whole.
- `not applicable`: the definition does not apply to the build; it is no pass (PHS.63).
- `waiting`: no test runs it yet; the text says for what. It applies to an ID that no marker names.
- `missing despite name`: only documentation names the ID, so it counts as missing.

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
| CT-LI-14 | waiting | an output-exhausted `jumble.observe`, whose row no crate builds; LP3's command-table freeze builds or drops the row (`rust/README.md`, missing list) |

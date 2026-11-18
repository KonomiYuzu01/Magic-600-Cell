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
| CT-ONE-13 | waiting | LP3's lanes, views and shell (`rust/README.md`, missing list) |
| CT-ONE-14 | waiting | LP3's lanes, views and shell (`rust/README.md`, missing list) |
| CT-ONE-15 | waiting | LP3's lanes, views and shell (`rust/README.md`, missing list) |
| CT-ONE-16 | waiting | LP3's lanes, views and shell (`rust/README.md`, missing list) |
| CT-ONE-18 | waiting | LP3's lanes, views and shell (`rust/README.md`, missing list) |
| CT-ONE-19 | waiting | LP3's lanes, views and shell (`rust/README.md`, missing list) |
| CT-LI-01 | waiting | LP3's lanes, views and shell (`rust/README.md`, missing list) |
| CT-LI-02 | waiting | LP3's lanes, views and shell (`rust/README.md`, missing list) |
| CT-LI-03 | waiting | LP3's lanes, views and shell (`rust/README.md`, missing list) |
| CT-LI-05 | waiting | LP3's lanes, views and shell (`rust/README.md`, missing list) |
| CT-LI-06 | waiting | LP3's lanes, views and shell (`rust/README.md`, missing list) |
| CT-LI-07 | waiting | LP3's lanes, views and shell (`rust/README.md`, missing list) |
| CT-LI-11 | waiting | LP3's lanes, views and shell (`rust/README.md`, missing list) |
| CT-LI-12 | waiting | LP3's lanes, views and shell (`rust/README.md`, missing list) |
| CT-LI-13 | waiting | LP3's lanes, views and shell (`rust/README.md`, missing list) |
| CT-LI-14 | waiting | LP3's lanes, views and shell (`rust/README.md`, missing list) |
| CT-EV4-03 | waiting | LP3's lanes, views and shell (`rust/README.md`, missing list) |
| CT-PS1-13 | waiting | LP3's lanes, views and shell (`rust/README.md`, missing list) |

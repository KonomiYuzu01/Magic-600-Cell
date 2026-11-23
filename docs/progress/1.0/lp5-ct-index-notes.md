# LP5 conformance index: notes

`tools/language/ct_index.py --phase LP5` computes the index of chapter 12 section 15.6 (each LP5 conformance test with
its state) from the table, the PHS.62 markers in the test sources and this file. This file holds what a marker cannot
say. LP5's integrator is its only editor; each note cites the tests it rests on.

The tool refuses (exit 2) a note for an ID outside the phase's table, an unknown note kind and a second note for one
ID. This file uses one kind:

- `whole by union`: the parts of the ID's tests together run the whole definition, so it counts as whole.

No ID here has an owner reading yet, so the tool's other kinds (`indexed elsewhere`, `not applicable`, `waiting`,
`missing despite name`) are not used. The last section lists every ID that does not run whole, with what is missing;
the integrator turns it into the owner's readings.

The rows rest on a hand reading of each test against its definition (section 15.6 and the owning chapters 01, 03, 05,
06, 09, 11 and 13) on the LP5 line at `66b597ff`; the markers written with this file change comments only. Everything
here is source evidence (PHS.31). Two rules of reading apply throughout:

- An ignored test (`#[ignore]`) is not run by the runner, `cargo test --locked --offline` through
  `tests/test_language_rust.py` in the headless CI (LP5 plan, section 4, selection 9), so it counts as no run. Earlier
  commits record some of them passing on earlier candidates: R4's thirteen in a release build at `cae3407a`, R6's
  three at `3fdd711f`. Those runs are not on the exit candidate (PHS.4), and no record shows R3's two or R5's four
  ignored tests run.
- A part that runs on synthetic data counts towards a union only where the definition names no archived record set
  that the synthetic data would stand in for.

At `66b597ff`, before these notes and markers, the tool gave 0 whole, 15 part and 8 named only. With them it gives
11 whole, all by union, and 12 part.

| ID | note kind | text |
| --- | --- | --- |
| CT-LB1-04 | whole by union | the library half in `c600-library/tests/research_ct_lb1_04.rs` (a straight-line program's length equal to its streamed expansion on 60 seeded programs, 2^200·3 and B11's 6,775 digits computed without expansion, `admitted_expansion` answering `admitted: false` over its bound with no `expand` row, and a stream's chunks staying rows of the one run) and the kernel's half in `c600-checkers/tests/s18.rs` (`ct_lb1_04_expansion_half_stage18_streamed_lengths_equal_declared_lengths`, and the oversized lengths refused before anything is decoded in `ct_lb5_09_recorded_long_words_stream_through_length_and_admission`) together run the whole definition |
| CT-LB1-05 | whole by union | fixed points, matching witnesses and integral-flow witnesses against brute force on synthetic instances in `c600-library/tests/research_ct_lb1_05.rs`, and the matching half on stage 19's stored Γ(Y) certificate in `c600-checkers/tests/kn6_s19.rs` (`ct_lb1_05_matching_half_certificate_meets_every_edge_and_brute_force_agrees`) together run the whole definition |
| CT-LB5-02 | whole by union | the fast tests of `c600-checkers/tests/r4/s15.rs` together run the whole definition: R01 to R17 with their check records, the whole descents and the P* and DIS-1 certificates reproducing their supports, changes and digests; `s15.macro`'s gate failing every tampered claim; the beamed and unbeamed stops keeping different scopes; `pstar2/2` and the run wrappers recognized. The 7307 census, an ignored test, is a relation outside this definition, and CT-KN6-03 lists it |
| CT-LB5-03 | whole by union | the fast tests of `c600-checkers/tests/r4/s16.rs` together run the whole definition: the jam descents, releases and transports of section 2.2 and P**'s whole descents from their exact tables with the jam word as prefix; SJ-E1 as a release check outside the macro family; reduced transport at 7307 matching the unreduced case and not promoted to a lemma; the leaf, routing and alphabet negatives keeping their scope. The I-7304 and I-7306 traces, ignored tests, are literal records outside this definition, and CT-KN6-03 lists them |
| CT-LB5-06 | whole by union | IMPORT-MANIFEST's stored and uncompressed digests in `tests/test_lp5_archive.py` (`test_ct_lb5_06_manifest_part`, with every member's recheck not run in `test_corpus_import_inventory`); the claims in `c600-checkers/tests/kn6_s19.rs` (the full pose tables, the matching, witness, frontier and rank certificates, R3/S decrease and R_L's length premises, exits failing the sticker goal, indexed references not read as distinct fibres, Γ(Y) carrying no sticker goal, unknown synthetic reachability); and LB5.14 to LB5.17's preflight in `c600-archive/src/bindings.rs`'s unit tests, `c600-library/tests/research_ct_lb5_06_preflight.rs` and `c600-research/tests/preflight_bindings.rs`, where no answer adds an evidence leaf, together run the whole definition |
| CT-LB5-09 | whole by union | in `c600-checkers/tests/s18.rs`, `ct_lb5_09_recorded_long_words_stream_through_length_and_admission` (the 672,417, 871,582 and 2,352,984-letter cases through length and admission checks), `ct_lb5_05_gate3_completed_table_names_t6_steps_1_to_3_and_not_a_tampered_record` and `ct_lb5_05_gate3_tail_replay_mutation_gate_on_a_synthetic_three_segment_record` (a newly completed replay is a new check with its own report, never the archive's history) and `ct_lb5_05_gate3_historically_incomplete_steps_are_never_passed`, with `historical_check_incomplete` kept for T6's step 4 and the rule stages in `c600-archive/src/bindings.rs`'s unit test and every member's recheck not run at import in `tests/test_lp5_archive.py`, together run the whole definition |
| CT-PS3-08 | whole by union | `test_ct_ps3_08_tampered_gzip_decoded_and_manifest_digests_fail_separately` (on the corpus, a tampered stored gzip stream, decoded content and digest in either manifest each fail separately) and `test_ct_ps3_08_source_edition_digest_is_not_taken_for_public_bytes` in `tests/test_lp5_archive.py` together run the whole definition; `c600-archive/tests/import.rs` runs the same checks on synthetic members |
| CT-PS3-09 | whole by union | `test_ct_ps3_09_case_only_pairs_stay_distinct_by_digest` (the six stage 16 pairs) and `test_ct_ps3_09_missing_member_is_reported_and_stays_counted` in `tests/test_lp5_archive.py` together run the whole definition; `c600-archive/tests/import.rs` runs both clauses on synthetic names |
| CT-PS3-10 | whole by union | `test_ct_ps3_10_bundle_declares_missing_dependencies_and_runs_nothing` (a corpus bundle carrying a producer as data, its missing dependency declared, no replay asserted, no checker claimed, no producer run) and `test_nothing_runs_a_producer_or_reaches_git_processes_or_the_network` (no process, network, Git or directory walk in the importer's sources) in `tests/test_lp5_archive.py` together run the whole definition |
| CT-DET-24 | whole by union | the three tests of `c600-research/tests/r8/ct_det_24.rs` (stage 15's archived timed stop and stage 19's data-only archive, imported at the research host, keep their recorded outcome and limits and reproduce as `not run`; R01 counts only through a KN6 check job of its own, with that job's record and scope) together run the whole definition, with the import halves in `tests/test_lp5_archive.py` and `c600-archive/tests/import.rs` |
| CT-ERR-19 | whole by union | the codes, owning chapters, refusal form and retries in `c600-evidence/tests/obligation.rs` (`ct_err_19_refusals_keep_their_codes_chapters_and_retry`) and a refused `research.obligation.export` writing no file in `c600-research/tests/panel.rs` (`panel_driver_end_to_end`, which names no CT ID) together run the whole definition |

## LP2's seven halves

The LP5 plan's exit needs the LP5 halves of seven LP2 tests to run whole. They are the seven that
`lp2-ct-index-notes.md` indexes in LP5 under reading 7: CT-KN6-02, CT-LG3-05, CT-LG5-02, CT-DET-01, CT-DET-02,
CT-DET-11 and CT-EVD-30 (CT-DET-24 is LP5's own). LP2's index keeps them `indexed elsewhere`, and this file does not
change that.

- CT-KN6-02 runs: `tests/test_language_scans.py`'s scan reads every source of `c600-checkers`, its core, its four
  kinds and the six stage folders, and finds it reaching only `c600-kernel`, `c600-jumble` and `c600-digest`. Its
  marker still reads as LP2's part, with no not-run text.
- CT-LG3-05 does not run whole: `c600-research/tests/r8/ct_lg3_05.rs` shows a check job's pass refused at run time
  on a stale revision, another identity block or model, an unavailable checker and the protection gate. Not run: the
  mode refusal, since the session store holds JumbleOff only, and what LP2's halves leave, a program's proposal
  refused at `operation.preview` on a moved head, in the other mode or under another model.
- CT-LG5-02 does not run whole: `r8/ct_lg5_02.rs` shows a certificate's declared edition, requirements and grants
  granting nothing at a check job, an ungranted input read as absent, and reproduction bound to the sealed edition
  and grants. Not run: a declared boundary class checked against chapter 07's reviewed classes.
- CT-DET-01 runs its LP5 half: `r8/ct_det_01.rs` changes each key field alone, the checker identity, the cost table
  and the menu order among them, and runs the same job in a child process with another locale, time zone, user and
  host name and working directory. A second machine is only CI's Linux run.
- CT-DET-02 runs its LP5 half: `r8/ct_det_02.rs` stores a check job's reports and traces, reads the record back and
  reproduces the exact payload byte for byte against pinned digests. The comparison across Windows and headless Linux
  counts once both CI jobs pass on the exit candidate.
- CT-DET-11 runs its LP5 half: `r8/ct_det_11.rs` and `c600-checkers/tests/kn6_stop.rs` show checks nested in jobs and
  in stop scopes and a job's child checks minting no allowance, with the job's ledger the sum of its checks' and its
  memory their peak. A check nested in a running program is stood in for by an enclosing stop scope.
- CT-EVD-30 runs its LP5 half: `r8/ct_evd_30.rs` shows that only a job's own run or a matching reproduction reaches
  `admit_check_report`, that a changed stored payload reproduces to nothing and that no import rule sets `independent
  checker`. The half LP2's reading 9 indexes with LP3's host, shell, display and renderer is not LP5's.

## Not whole at the exit

Twelve IDs do not run whole. Four of them (CT-KN6-01, CT-LB5-01, CT-LB5-04 and CT-LB5-05) need only runs of ignored
tests on the exit candidate, and CT-KN6-03 needs those runs and more; the other seven need a test or a product part
that does not exist.

- CT-KN6-01: every kind's gate runs except the gates over their claims of three research-archive sub-checkers,
  `s15.pstar`, `s16.mobility` and `s17.rr`, which only the ignored tests `ct_kn6_03_stage15_pstar_gate`,
  `ct_kn6_03_stage16_mobility_gate` and `ct_kn6_03_stage17_rr_gate` run (about a minute, several minutes and half an
  hour in a debug build). By default those three fail only under a few tampered claims and a wrong start table.
- CT-KN6-03: the ignored record checks, stage 14's thirteen other level tables, stage 15's 7307 census, stage 16's
  I-7304 and I-7306 traces and the mobility records of the jams other than 7304, stage 17's RR gate records H2 and
  H2b, and stage 18's in-place gate 2 and 3 replays and lattice replay (together about 13 hours in a debug build, 11
  of them the lattice phase); stage 19's seven type-lex records, for which the archive defines no irregular-piece
  vector, and its cap-noninteraction claims, which wait for the definition of delta_Q5, have no sub-checker. No test
  tallies the groups that preflight answers `ready` against passing reports; the stage files check their records
  family by family.
- CT-LB5-01: the thirteen level tables other than the 7304 stop's, which only the ignored test
  `ct_lb5_01_levels_rebuilt_from_every_table_and_word` rebuilds (about 100 s). Everything else runs.
- CT-LB5-04: the RR canonical-word and five-tail gates against brute force at the recorded inputs H2 and H2b, which
  only the ignored tests `ct_lb5_04_stage17_rr_gate_h2` and `ct_lb5_04_stage17_rr_gate_h2b` run, with the label
  calculus checked against the engine in the ignored `ct_kn6_03_stage17_label_calculus_agrees_with_the_engine`. The
  bounds of the `s16-rr/1`, `s16-pxrr/1` and stop records run.
- CT-LB5-05: the archive's completed gates 2 and 3 rechecked in place, T6's four steps and the rule stages under
  gate 2 and T6's steps 1 to 3 under gate 3, which only the ignored tests of `c600-checkers/tests/s18/long.rs` replay
  (about an hour, half an hour and half an hour); no record shows them run. Gate 1 in place, the mutation controls,
  the missing third gate and the shortened chain run.
- CT-LB5-07: the search over the puzzle's states through KN2.4's rows, and the search run as a research job whose
  record's `ReproKey` holds LB5.2's input and module identities. The library's record contract runs on synthetic
  families; the research host has no search job.
- CT-LB5-08: the jumbling form of D[k] over KN2.4's branch predicates, which needs research host reads that are not
  built. The library's membership runs on synthetic actions.
- CT-LB5-11: at `research.obligation.export`, a check that an export changes no evidence grade and no revision, and
  a scan showing that the row's code in `c600-research` names no process or network API (the translator module's
  scan runs in `c600-evidence/tests/obligation.rs`). The translator half and the row's grant and refusals run.
- CT-PS1-26: the format packet's unknown-symbol and wrong-arity vectors refused through `research.obligation.export`
  with no file written (`c600-research/tests/panel.rs` refuses only statements with no resolvable body; a named
  statement cannot reach the row, which takes a typed digest), and a check at that row that an export promotes no
  evidence.
- CT-RTE-23: a check at `research.obligation.export` that an export promotes no claim, the evidence and revision
  unchanged after it. The translator's half and the row's file grant run.
- CT-EVD-12: E1 to E3's words of section 9 built from the facts that the import of their records gives through the
  label table. The import half in `tests/test_lp5_archive.py` checks the facts of R01, R01's check record, the stage
  19 receipt and the 7302 stop only, and `c600-evidence/tests/examples.rs` builds the words from a transcription. The
  forms come from the test's own statements, as chapter 06's open item allows.
- CT-ERR-11: the jumbling form of D[k] and the jumbling puzzle's own off-lattice results as research answers, which
  need research host reads that are not built. The library's answers and the preflight's unavailable answers run as
  values, and the research host's job records keep stops as values (`c600-research/tests/r8/stop.rs`).

The three obligation IDs, CT-LB5-11, CT-PS1-26 and CT-RTE-23, need only small additions to `c600-research`'s tests,
which this index did not edit. CT-EVD-12 needs a test that joins the import's facts to section 9's graphs, larger
than a small one. CT-LB5-07, CT-LB5-08 and CT-ERR-11 need research host reads or a search job, which are product
work.

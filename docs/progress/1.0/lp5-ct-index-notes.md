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
06, 09, 11 and 13) on the LP5 line at `66b597ff`; the markers written with this file change comments only. The rows
of CT-LB5-11 and CT-RTE-23 and the present text of CT-ERR-19 rest on the same reading of the row tests that branch
`lp5-ct-gaps` adds, `c600-research/tests/obligation_export.rs`, and of its markers in `c600-research/tests/panel.rs`.
Everything here is source evidence (PHS.31). Two rules of reading apply throughout:

- An ignored test (`#[ignore]`) is not run by the runner, `cargo test --locked --offline` through
  `tests/test_language_rust.py` in the headless CI (LP5 plan, section 4, selection 9), so it counts as no run. Earlier
  commits record some of them passing on earlier candidates: R4's thirteen in a release build at `cae3407a`, R6's
  three at `3fdd711f`. Those runs are not on the exit candidate (PHS.4), and no record shows R3's two or R5's four
  ignored tests run.
- A part that runs on synthetic data counts towards a union only where the definition names no archived record set
  that the synthetic data would stand in for.

At `66b597ff`, before these notes and markers, the tool gave 0 whole, 15 part and 8 named only. With them it gives
11 whole, all by union, and 12 part. With the row tests and markers of `lp5-ct-gaps` it gives 13 whole, all by union,
and 10 part.

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
| CT-LB5-11 | whole by union | the translator half in `c600-evidence/tests/obligation.rs` (`ct_lb5_11_translator_writes_only_the_vocabulary`: only the versioned vocabulary written, every untranslatable form refused with no text, the prelude not compiled in and no Lean dependency; `ct_evd_20_obligation_module_names_no_effect`: no process, network or file in the module; `ct_rte_23_export_promotes_nothing_and_has_no_return_path`: no return path and no check value added) and the row half: in `c600-research/tests/panel.rs`, the row's shape with no token, gate or preview and, in `panel_driver_end_to_end`, the supported statement written only to a granted file while an ungranted destination and statements with no body write nothing; in `c600-research/tests/obligation_export.rs`, the unknown-symbol and wrong-arity vectors refused through the row with no file written or changed, an export and its refusals changing no evidence word, claim, report, run, revision, workspace record or preview after a KN6 check gave the group an admitted leaf, and a scan finding no process, network or Lean in the crate's sources and the translator, file access only in the host's grant, no Lean package in the lock file and no row or import fact that returns a proof record; together they run the whole definition. LB5.18's translator takes a typed Statement, so its untranslatable statement is a typed one; a named StatementId is PS1.46's case and stays with CT-PS1-26 |
| CT-ERR-19 | whole by union | the codes, owning chapters, refusal form and retries in `c600-evidence/tests/obligation.rs` (`ct_err_19_refusals_keep_their_codes_chapters_and_retry`) and a refused `research.obligation.export` writing no file in `c600-research/tests/panel.rs` (`panel_driver_end_to_end`) and `c600-research/tests/obligation_export.rs` (`ct_ps1_26_refusal_vectors_refuse_through_the_row_and_write_nothing`) together run the whole definition |
| CT-RTE-23 | whole by union | the translator half in `c600-evidence/tests/obligation.rs` (`ct_rte_23_export_promotes_nothing_and_has_no_return_path`: PS1.45's format, no Lean dependency, no proof-record reader, import row or check value added, and `formal proof` refused to every writer) and the row half in `c600-research/tests/panel.rs` (LB5.18's file grant; the row's shape with no token, gate or preview) and `c600-research/tests/obligation_export.rs` (the export's format, vocabulary and pins at the row; no evidence word, claim, report, run, revision or workspace record changed by an export; no Lean package in the workspace's lock file, no row that reads a proof record back and no import fact asserting `formal proof` or a check mode outside EVD.47's) together run the whole definition |

## LP2's seven halves

The LP5 plan's exit needs the LP5 halves of seven LP2 tests to run whole. They are the seven that
`lp2-ct-index-notes.md` indexes in LP5 under reading 7: CT-KN6-02, CT-LG3-05, CT-LG5-02, CT-DET-01, CT-DET-02,
CT-DET-11 and CT-EVD-30 (CT-DET-24 is LP5's own). LP2's index keeps six of them `indexed elsewhere`, and this file
does not change that. CT-KN6-02's marker reads whole since `lp5-ct-gaps`, and a whole marker overrides an `indexed
elsewhere` note, so LP2's index counts it whole: 129 whole and 104 indexed elsewhere, where `lp2-ct-index-notes.md`
still states 128 and 105.

- CT-KN6-02 runs whole: `tests/test_language_scans.py`'s scan reads every source of `c600-checkers`, its core, its
  four kinds and the six stage folders, and finds it reaching only `c600-kernel`, `c600-jumble` and `c600-digest`; it
  also scans the evidence library's report reader and LP2's oracle comparison. The one scan is LP2's half and LP5's,
  and its marker reads whole.
- CT-LG3-05 does not run whole: `c600-research/tests/r8/ct_lg3_05.rs` shows a check job's pass refused at run time
  on a stale revision, another identity block or model, an unavailable checker and the protection gate. Not run: the
  mode refusal, since the session store holds JumbleOff only, and what LP2's halves leave, a program's proposal
  refused at `operation.preview` on a moved head, in the other mode or under another model.
- CT-LG5-02 does not run whole: `r8/ct_lg5_02.rs` shows a certificate's declared edition, requirements and grants
  granting nothing at a check job, an ungranted input read as absent, and reproduction bound to the sealed edition
  and grants. Not run: a declared boundary class checked against chapter 07's reviewed classes. No product code holds
  those classes as data (BND.57's rows by class do not exist and BND.59 is open): `c600-tools`' manifest declares its
  classes with no check, `c600-repr`'s `review_class` reviews `view_aid` only and is outside `c600-research`'s
  dependencies, and a check job's certificate carries no boundary class. Closing it needs the reviewed classes as data
  and a host check that reads a manifest's declared class against them, product work outside the research host.
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

Ten IDs do not run whole. Four of them (CT-KN6-01, CT-LB5-01, CT-LB5-04 and CT-LB5-05) need only runs of ignored
tests on the exit candidate, and CT-KN6-03 needs those runs and more; the other five need a test, a product part or
a product change that does not exist.

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
- CT-PS1-26: a named statement given to `research.obligation.export`. The row's `statement` argument takes only a
  typed StatementId's digest, so the named-statement vector's FP.19 form is refused with `KN5:input.argument_form`
  before translation and writes nothing (`c600-research/tests/obligation_export.rs`), where PS1.46 names
  `LB5:input.obligation_untranslatable`; the translator itself refuses it with that code. Everything else runs: the
  supported vector byte for byte with its pins through a granted file, the unknown-symbol and wrong-arity vectors
  refused through the row with nothing written, and no evidence promoted. Closing it needs the row to answer a named
  StatementId with `LB5:input.obligation_untranslatable` (a change in `c600-research/src/panel/host.rs`) or an owner
  reading. The marker in `c600-evidence/tests/obligation.rs` still names the row's vectors and evidence check as not
  run; they run now.
- CT-EVD-12: E1 to E3's words of section 9 built from the facts that the import of their records gives through the
  label table. The import half in `tests/test_lp5_archive.py` checks the facts of R01, R01's check record, the stage
  19 receipt and the 7302 stop only, and `c600-evidence/tests/examples.rs` builds the words from a transcription. The
  forms come from the test's own statements, as chapter 06's open item allows. A test of about 200 lines does not
  close it: no product code joins a record's rows into a leaf (EVD.50), since the import's facts are counted and
  read by nothing that builds a graph. The bases of E1's a₁ and a₂ are no imported facts (R01 has no label occurrence,
  and its check record gives check and mode only), and E1's Theorem MC and third replay path, E2's four other rows and
  E3's six other records have no imported facts checked yet, some of them kept recorded until a whole-record rule
  maps them (EVD.51).
- CT-ERR-11: the jumbling form of D[k] and the jumbling puzzle's own off-lattice results as research answers, which
  need research host reads that are not built. The library's answers and the preflight's unavailable answers run as
  values, and the research host's job records keep stops as values (`c600-research/tests/r8/stop.rs`).

CT-PS1-26 needs a product change at the row or an owner reading. CT-EVD-12 needs an envelope builder that joins a
record's facts into leaves, and the facts of the other records of E1 to E3, which is more than a small test.
CT-LB5-07, CT-LB5-08 and CT-ERR-11 need research host reads or a search job, which are product work.

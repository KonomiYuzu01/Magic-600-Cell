# Language LP5 plan: research expressiveness

Status: plan of 22 November 2026, written when LP2 and LP3 had exited
([LP2 and LP3 exit](../../wiki/decisions/owner-decisions-2026-11-22-c10-lp2-exit.md),
[LX bars and freeze](../../wiki/decisions/owner-decisions-2026-11-22-lx-bars-freeze.md)) and before any LP5 code;
updated 23 November 2026, when the owner started LP5 and took every recommended option
([LP4 and LP5 start](../../wiki/decisions/owner-decisions-2026-11-23-lp4-lp5-start.md), item 3; section 11). It
records the integrator's selections, and section 12 its exit record. It is no ADR, freezes nothing and edits no frozen
text: where a frozen clause and this plan meet awkwardly, section 9 records it for the integrator.

Inputs: [partition](../../architecture/1.0/language/ecosystem-partition.md) row LP5; chapter 12 section 7 (PHS.30 to
PHS.33) and section 15.6; chapter 03 section 6 and 7 (LB5.1 to LB5.18); chapter 01 section 7 (KN6.1 to KN6.6);
chapter 05 section 11 (PS3.10 to PS3.17) and PS1.31, PS1.45, PS1.46; chapter 09 DET.1 and DET.44 to DET.48; chapter 06
EVD.46 to EVD.53 and EVD.64; the [claim matrix](../../architecture/1.0/language/claim-matrix/README.md) (CX.1 to CX.23);
the [research handover](../../../research/jumbling/restoration/README.md); the owner's decision on the
[design and art path](../../wiki/decisions/owner-decisions-2026-11-22-design-art-path.md); the
[LP3 plan](language-lp3-plan.md), whose form this plan follows.

Evidence kind for everything LP5 produces: source (PHS.31 assigns every LP5 acceptance item that class). Nothing here is
Windows interaction, DirectX, input, long-session or performance evidence. The research stays stopped: LP5 checks and
imports what the repository holds and starts no search whose result could count (PHS.31, PHS.56).

## 1. What LP5 delivers

PHS.30's four products, with the clauses that define them, the conformance tests that check them (chapter 12 section
15.6 and the halves the owner indexed in LP5 on 22 November 2026, reading 7) and the pieces of section 5 that build them.

| Product | Clauses | Tests | Pieces |
| --- | --- | --- | --- |
| Research library: chapter 03's research entries, family recognition, ranks, conditional models, symbolic bounds | LB5.1 to LB5.7, LB5.9, LB5.12, LB5.13; LB1's big integers, straight-line programs, fixed points, matching and integral flows | CT-LB1-04, CT-LB1-05, CT-LB5-07, CT-LB5-08, CT-LB5-09, CT-ERR-11 | R7 |
| Archive import and independent checking: stage 14 to 19 formats, manifests and word members through PS3's versioned import table and KN6's checkers | PS3.10 to PS3.17; KN6.1 to KN6.6; LB5.8, LB5.10, LB5.11, LB5.14 to LB5.17; EVD.46 to EVD.53, EVD.64 | CT-PS3-08, CT-PS3-09, CT-PS3-10, CT-KN6-01, CT-KN6-03, CT-LB5-01 to CT-LB5-06, CT-EVD-12; LP2's halves of CT-KN6-02, CT-EVD-30 | R0 to R6 |
| Reproducible research work: engine-owned branches, derived states and run records | LB5.2, LB5.7; PS1.31; DET.1 to DET.6, DET.44 to DET.48; KN7.3, KN7.4 | CT-DET-24; LP2's halves of CT-DET-01, CT-DET-02, CT-DET-11, CT-LG3-05, CT-LG5-02 | R8 |
| Proof-obligation export: `research.obligation.export` in PS1.45's format through a granted file capability | LB5.18; PS1.45, PS1.46; RTE.28; EVD.47, EVD.64 | CT-LB5-11, CT-PS1-26, CT-RTE-23, CT-ERR-19 | R9 |

Section 15.6 indexes 23 tests in LP5, all source. The owner's reading 7 adds the LP5 halves of seven LP2 tests
(CT-KN6-02, CT-LG3-05, CT-LG5-02, CT-DET-01, CT-DET-02, CT-DET-11, CT-EVD-30; [LP2's index
notes](lp2-ct-index-notes.md)); each counts as run only when its whole definition runs (PHS.62). CT-LB5-10 is
LP1's and must keep passing (`check_matrix.py`); CT-LB5-12 is LP6's. The research output panel's data interface (section
6) is stage 2 work beside these products, not an LP5 product.

## 2. Acceptance (PHS.31)

| Acceptance item | Pieces and tests | Check |
| --- | --- | --- |
| Certificates and verdicts rechecked and matched | R2 to R6; CT-KN6-01, CT-KN6-03, CT-LB5-01 to CT-LB5-06 | Every claim group whose data the matrix marks self-contained (291 of 510, CX.14) gets a KN6 `CheckReport` from a checker that imports no producer code (CT-KN6-02); each report is matched with the record's own meaning, scope, original label and check mode through the import table; every checker's mutation gate (tampered claim, wrong start table, wrong input digest) fails |
| Missing dependencies stay visible | R0, R1; CT-LB5-06's preflight part, CT-LB5-10 (LP1), CT-PS3-10, CT-DET-24 | Every other group answers LB5.14's `unavailable(reasons, dependencies)` with the matrix's blockers; composed-only and incomplete replays (stage 18's gate 3) stay recorded as such; no unavailable check reads passed |
| A run record reproduced bit for bit | R8; CT-DET-24, LP2's halves of CT-DET-01, CT-DET-02 | A KN6 check of an archived certificate runs as a language job on an engine-owned branch at a digest-pinned input; its `RunRecord` is reproduced by `reproduce` with an exact payload equal byte for byte, on Windows and on headless Linux in CI; the archive's own producers supply none of this (DET.48) |
| No new search counted as acceptance | R0 to R9 | A scan of every LP5 acceptance command and its inputs against the Git-blob manifest and this plan: no acceptance input is produced by a search, a strategy or an enlarged admitted domain. The searches CT-LB5-07 and CT-LB5-08 run on small synthetic fixtures test the record contract only, and their outputs enter no research evidence (section 9, reading B) |
| Proof-obligation export keeps its limits | R9; CT-LB5-11, CT-PS1-26, CT-RTE-23, CT-ERR-19 | The supported statement of the format packet's `obligations` vectors exports byte for byte through a granted file; each refusal vector writes nothing; no process, network, evidence promotion, commit gate or return path appears |

LP5 exits when each item has evidence on one candidate (PHS.4), the 23 tests and the seven LP5 halves run whole, and
the owner has confirmed understanding of the archive claim scopes, missing producers, conditional bounds, bit-for-bit
reproduction and the global coverage still open (PHS.56, PHS.57).

## 3. The archive inventory

Baseline (PHS.32, default 13): the Git-blob manifest, not a checkout walk. It already exists:
`work/experiments/language-lx/envelope-facts/MANIFEST.json` (tracked) names every stage 14 to 19 member by Git blob,
and the frozen claim matrix pins it (CX.3, CX.4) with 697 file pins, all resolving, read as `git_blob_bytes`
independent of line-ending conversion. LP5 reads the archive only through that manifest; a member missing from a
checkout is reported missing and stays in the inventory (PS3.15). Stage 19's `IMPORT-MANIFEST.json` binds stored,
decoded and original-source digests separately (PS3.14); older stages keep their own record identities and have no
manifest of that form.

Totals from the matrix (CX.14): 497 records; 37 stage and family rows; 510 claim groups, of which 291 have
self-contained data, 213 need data or a binding, and 6 are record-only anchors. Fresh availability today: 504
`needs_input`, 6 `record_only`, none ready, since no KN6 checker exists and no claim has a typed binding. Of 681
declared reference occurrences, 211 are missing and 2 resolve to another edition (CX.16). PS3.11 counts 156 of 417 JSON
files with no top-level `format` key.

| Stage | Files, formats and manifest | Self-contained claim groups (LB5.8) | Producer-dependent or input-dependent groups, and what is missing |
| --- | --- | --- | --- |
| 14 Blocking structure | 81 files. Eight `t3p-*/1` formats (certificate, check, scan, levels, chain, pairs, plus, locality); 20 JSON records without a format key (chain replays, retained-beam records, `sources.json` as a bare list). No manifest of its own; `results/sources.json` catalogues the 16 stage 13 tables by digest | 44: T3P rounds and P0 to P4 (16), locality and fixed-point levels (19), literal replay rows (9). The level label caches are not stored but rebuild from a stored table; the locality summary omits its per-partner arrays, which recompute from model and table | 28 finite-search negatives: a complete coverage certificate (alphabet, bounds, every admitted edge, pruning lemmas, stop coverage). The beam searches and four "reported" facts depend on programs not committed. 2 input-catalogue anchors are record-only |
| 15 Lowering machine | 93 files. `macro-certificate/1`, `macro-check/1`, `pstar-census/1`, `pstar-step/1`, `pstar2-stop/1`, `pstar2/2`; nine run wrappers carry their format only inside (`run.format` is `pstar-run/1`), an absent-format case for PS3.12 | 57: macro certificates including R01 (55), relations (2) | 27 selector runs and 2 finite searches: per-family coverage and reduction certificates (per-candidate records and caches were not stored); 1 relation: the exact start table and prefix binding of its source record |
| 16 Six jams | 118 files. Only `macro-certificate/1` and `macro-check/1` carry a format key (48); 61 records do not, `jams.json` (the per-seed table, chain and end-digest catalogue) among them | 58: whole-descent macros (48), jam and release words (4), relations (6); releases such as SJ-E1 are literal words checked letter by letter | 41 reductions and 9 selector runs: complete finite coverage and matching reduction premises; the routing and release searches are beams, and some helper producers were never committed (LB5.15). 1 input catalogue is record-only |
| 17 Routing in the driver | 60 files. Every record has a format key: `s16-rr/1`, `s16-pxrr/1`, `s16-transport/1`, `pstar2-stop/1`, `macro-certificate/1`, `macro-check/1` | 20: chosen descent macros | 36 RR groups: the complete matching RR shard set, tail bounds and a reduction certificate; the shard set is complete for 7304 only, and the other stops exist only as run records |
| 18 Tail completion | 167 files. Only `runner-run/3` carries a format key; 97 records do not (`steps.json` in both forms, word, pose and step-tree records); 53 `ia16` word members. No manifest; `steps.json` binds each step's word, start and end digests | 93: expansion and hash (35), lattice and runner records (58) | 2 composed strict′ groups: actual preceding end tables and composed-contract inputs at every join; 2 tail-replay groups: actual-start tables and literal or chunk coverage; 5 static-tail plans: action and group witnesses, hole demands, endpoint joins. Gate 3 is stated complete for T6 steps 1 to 3 in the stage README only, with no stored replay record; T6 step 4 and the three rule stages keep `historical_check_incomplete` (LB5.12). The lattice run's `check.jsonl`, which LB5.15 cites, is not in the repository |
| 19 Height-depth framework | 113 files. `IMPORT-MANIFEST.json` (`sanitized-research-archive/1`) lists the 109 files under `records/` with stored, decoded and original digests; the four top-level files are not listed. 34 formats, each used once; 45 records without a format key (pose tables, `PASS` records). No executable producer or checker (LB5.16) | 19: matching and Γ(Y) (5), frontier exits (10), R3, S and R_L ranks (4); the 9-letter, 16-letter and 1,824-letter words are literal | 75 groups across admission, controlled bound, finite extras, high restoration, indexed reviews, lattice bound, profiles, pump, q11 and shortening. Named absences: `type-lex-successor-base-cache.npz` (q11); `type-height-weight-inputs.json` (a new derivation may replace it, CX.17); four low-tail array caches; three records cite source files at a pinned revision the repository does not hold. The remaining-obligations anchor and 2 input catalogues are record-only |

The restoration README's general claims that each stage has an import manifest and that each result has a checker
importing no construction code hold for stages 01 to 10 and 19, not for 14 to 18 (LB5.15 records the shared producer
chains). LP5 reads the matrix and the Git-blob manifest, not those README sentences (section 9, note D).

Mandatory checks and their dependencies. PHS.31's first item makes every checkable claim mandatory, which is the 291
self-contained groups; the 213 others are LB5.14's visible unavailable outcomes, not waived checks. One mandatory check
may lack its dependencies: CT-LB5-05 asks LP5 to "recheck the archive's completed gates", and the completed gate 2
(composed strict′) of T6 steps 1 to 4 and the rule stages and the completed gate 3 (literal replay) of T6 steps 1 to 3
sit in the 18/composed and 18/tail_replay rows, whose matrix entries name actual start and preceding end tables as
inputs not bound. The inventory points to a way through for T6: its start table is stored (stage 13's
`runs/dev/d1-acc-s7101/out4/tables/00113.json`), its literal words are stored with their digests in `steps.json`, and
each later step's start follows by replaying the earlier step's word, so the per-step tables can be rebuilt exactly
as a new derivation under the LP5 checker's identity (as CX.17 allows), not presented as the archive's own. The
rule stages' start tables were not traced. Piece R0 settles both before R5 starts. The owner decided on 23 November
2026 (LP5-3) that an unstored start table is rebuilt by exact replay as a new derivation with its own checker. Only a
table that can be neither found nor rebuilt from stored data makes LP5 wait on that group; the integrator then tells
the owner (PHS.33), and the rest of LP5 continues. The lattice phase's 13,231,266 twists can be replayed from T6's end as a fresh check,
but with `check.jsonl` absent there is no archived check to match it against, and the record says so. No other
section 15.6 test names a check over a row whose data the matrix marks as needing input; R0 confirms this per test
before the stage pieces start.

## 4. Integrator selections

1. **Existing crates first.** Research library entries are language code in `c600-library` (`.6cs` modules, LB1.1).
   Run records and branches stay in `c600-session` (`persist/run.rs`, ONE.75 branches). Statements, the evidence graph,
   EVD.52's import rules and `CheckReport` admission stay in `c600-evidence`, which also takes the obligation
   translator as pure code (it holds the typed `Statement`). LP2 and LP3 crates change only with their owners' small
   agreed API additions.
2. **Three new crates, all with no external crate.**
   - `c600-checkers` (KN6): kernel code (KN6.3) on the exact engines; it depends on `c600-kernel`, `c600-jumble` and
     `c600-digest` only, so it has no path to the interpreter, the library, any archive reader's interpretation or a
     producer. It emits KN6.1's `CheckReport` as its own type.
   - `c600-archive` (PS3 research import and exchange): the import table, the FP.22 discriminator for absent-format
     records, manifest and member checks, the import inventory and `export_check_bundle`; it depends on `c600-digest`,
     `c600-evidence` and `c600-kernel`, whose container readers (`archive`: DEFLATE and gzip; `json`) it reuses rather
     than writing its own, and calls no engine. It executes nothing (PS3.16, PS3.17).
   - `c600-research` (the research host): the overlay rows, the host reads the research entries need, the KN6 job
     adapter that hands a `CheckReport` to `c600-evidence`'s admission (EVD.64: only the kernel writes `independent
     checker`), and the research output panel's rows. It depends on `c600-tools`, `c600-command`, `c600-checkers`,
     `c600-archive`, `c600-evidence`, `c600-session` and `c600-library`, as `c600-jumble-tool` does on its side.
   These edges stay inside the frozen layering: the command layer gains no edge (ONE.48), the kernel crates gain no edge
   upward, and no checker reaches language or producer code. An edge outside this list that a piece finds it needs is
   not taken; it goes to the integrator and follows PHS.52 before any code. Product dependencies: none new.
3. **The Git-blob manifest is read by the test driver, not by product code.** A Python driver under `tests/` copies the
   manifest's blobs (`git cat-file`) into a fresh scoped directory and grants it to the importer as files, as PS3.17's
   `import_archive` takes them; product code never walks a checkout or runs Git. The driver verifies the blob identity
   and SHA-256 of every member before the grant.
4. **Checker identity.** KN6.1's fifth kind, research archives, is a family of sub-checkers (one per claim family of
   LB5.15 and LB5.16), each with its own `CheckerId`, source digest and mutation gate; the other four kinds are one
   checker each (section 9, reading A).
5. **Claim bindings.** The typed statement, scope and input binding that FR-B08 still owes (CX.22) is written by LP5 as
   versioned data in `c600-archive`, one row per claim group, from the matrix's anchors; an unbound group keeps
   `claim_unmapped` (owner, 23 November 2026, LP5-4).
6. **New reads and rows.** The research host needs reads and overlay rows that H-08 did not hold at the LP3 freeze
   (KN6's `check`, the import inventory, run records, `research.*` rows). Overlay rows are language-defined (KN5.3,
   ADR-L02.4); host reads extend CM.18's list. They enter as ordinary additions after the freeze, as CM.19 and CM.20,
   changing no frozen row (owner, 23 November 2026, LP5-1).
7. **No Lean run.** The export is checked against the format packet's exact Lean text; Lean stays off the allowlist
   and out of every check (LB5.18; owner, 23 November 2026, LP5-2).
8. **Critical path and review.** `c600-checkers` decides pass or fail for independent checks and `c600-research` writes
   through the command layer; each candidate change to them gets one read by a fresh Claude subagent on a smaller model
   before commit, with the diff and the piece's acceptance check (`AGENTS.md`, "Codex minimum"). No Codex call.
9. **Runner and records (PHS.65 as in LP3).** `cargo test --locked --offline` in `rust/` through
   `tests/test_language_rust.py`, and the archive driver `tests/test_lp5_archive.py`, both listed in
   `tools/ci/run_headless.py`, so CI runs them on Windows and Linux. A test naming a CT ID names its part.

## 5. Pieces

Each piece is one session in its own worktree with a seven-part packet naming its allowed files, acceptance check and
stop condition. Every piece's acceptance also includes: `cargo test --locked --offline` and the per-member `cargo fmt
--check` pass; `check_matrix.py`, the format packet's checks and the eight language checkers still pass; no file under
`docs/architecture/1.0/language/` or `research/` changes; nothing needs a renderer, a GPU, Lean or a timing. Each stops
early and reports if it needs a frozen-file change, a new crate edge outside section 4, an input the archive lacks, or a
product dependency.

| Piece | Crate and files | Content | Tests (part named) | Acceptance check |
| --- | --- | --- | --- | --- |
| R0 Import and inventory | `c600-archive` (new); `tests/test_lp5_archive.py` | PS3.10's import table edition 1 keyed by (format, field, value); FP.22's discriminator for absent-format records; PS3.14 manifest checks; PS3.15 case-only names; `ImportReport` with recognition, integrity, interpretation and recheck as separate results; `export_check_bundle`; the per-test dependency confirmation of section 3 | CT-PS3-08, CT-PS3-09, CT-PS3-10, CT-DET-24; CT-EVD-12's import half; CT-LB5-06's manifest part | All 497 records import from the Git-blob manifest with every (format, field, value) mapped or kept unmapped word for word; tampered gzip, decoded and manifest digests fail separately; the six case-only pairs stay distinct; a missing member is reported and stays counted; no producer runs; a written report lists, per section 15.6 test, whether its mandatory inputs are present |
| R1 Bindings and preflight | `c600-archive` (binding data); `c600-library` (`research_preflight.6cs`) | FR-B08's typed bindings per claim group; LB5.14's `check_availability` as library preflight data | CT-LB5-06's preflight part; CT-LB5-10 kept; CT-ERR-11's unavailable part | Every one of the 510 groups answers `ready` or `unavailable` with the matrix's reasons and dependencies; q11 answers `unavailable([dependency_missing, no_independent_checker], ...)` with its captured string unchanged (LB5.17); no answer adds an evidence leaf |
| R2 Checker core | `c600-checkers` (new); `tests/test_language_scans.py` | `check(certificate, kind) -> CheckReport`; cache-free letter-by-letter replay on both puzzles; the mutation-gate harness; the four non-archive kinds (admissibility and straddlers, macro certificates, compressed-word expansion, ν and label checkpoints); the import scan | CT-KN6-01's core part; LP2's half of CT-KN6-02; CT-KN6-03's harness | The scan finds no checker importing producer, search, construction, library or interpreter code; each kind's tampered record and wrong start table fail; reports carry checker digest, input digest and a per-claim trace |
| R3 Stage 14 checkers | `c600-checkers`, `src/archive/s14/` | T3P rounds, P0 to P4 and chain effects; locality and greatest-fixed-point levels from the model and table; literal replay rows | CT-LB5-01; CT-KN6-03's stage 14 part | Every self-contained stage 14 group passes and matches its record; changed helpers, precedence or start digest fail; scan negatives answer `unavailable` with their scope |
| R4 Stages 15 to 17 checkers | `c600-checkers`, `src/archive/s15/`, `s16/`, `s17/` | Theorem MC claim checks and literal replay for `macro-certificate/1`; jam and whole-descent chains from exact tables; SJ-E1 letter by letter (LB5.11); RR canonical words and five tails against brute force on the recorded gate inputs only | CT-LB5-02, CT-LB5-03, CT-LB5-04 | R01 and every corpus certificate reproduce supports, changes and digests; tampered claims fail; beamed and unbeamed stops keep different scopes; `pstar2/2` imports; reduced transport matches the recorded small unreduced cases and is not promoted to a lemma; bounded stops refute nothing beyond their bounds |
| R5 Stage 18 checkers | `c600-checkers`, `src/archive/s18/` | Streamed expansion and hash; composed strict′ contract; literal replay with chunk-chain validation; R22B-10 versions and runner records | CT-LB5-05, CT-LB5-09; CT-LB1-04's expansion half | Both `steps.json` forms import; gates 1 to 3 are rechecked where the archive completed them (or wait, section 3); T6 step 4 and the rule stages keep their missing gate 3; the 672,417, 871,582 and 2,352,984-letter cases stream through length and admission checks without full allocation |
| R6 Stage 19 checkers | `c600-checkers`, `src/archive/s19/` | Pose-table validity, Γ(Y) matching and gauges, the nine-letter forward and inverse witness, frontier exits, R3, S and R_L on stored blocks, finite rank arithmetic | CT-LB5-06; CT-LB1-05's matching half | Stored and decoded digests check; the matching certificate and frontier witnesses pass; frontier exits fail the sticker goal; R_L keeps its length premises; synthetic reachability stays unknown; the 73 indexed references are not read as distinct fibres |
| R7 Research library | `c600-library`, new `research_*.6cs` modules; built-in column rows | LB5's entries as language code: search declarations and records (LB5.2, LB5.3), admitted-domain membership (LB5.4), ranks and bounds (LB5.5), family recognition (LB5.6), conditional models; LB1's big integers, straight-line programs, fixed points, matching and integral flows | CT-LB1-04, CT-LB1-05, CT-LB5-07, CT-LB5-08, CT-ERR-11 | Straight-line length equals streamed expansion on small fixtures and B11's 6,775 digits compute without expansion; a cut-off membership enumeration never yields `OUTSIDE_CERTIFIED_DOMAIN(k)`; a budget, clock or error stop yields no negative; every searching entry is a research tier candidate refused in a solving session |
| R8 Reproducible research work | `c600-research` (new); `c600-session` (small agreed additions only) | Research jobs on engine-owned branches at a digest-pinned input, finishing after a commit (LB5.7); a KN6 check as a job with its `RunRecord`; `reproduce`; archived timed stops imported as recorded | CT-DET-24; LP2's halves of CT-DET-01, CT-DET-02, CT-DET-11, CT-LG3-05, CT-LG5-02, CT-EVD-30 | A complete record reproduces byte for byte on Windows and Linux; changing the checker identity changes the key; nested checks mint no allowance; only the kernel's report writes `independent checker`; static success never bypasses a stale revision at run time; the check job's budget reaches the checker as a cooperative stop, so a long expansion ends as the job's cut-off, never as a verdict (R2's review left the expansion kind's running time to this piece) |
| R9 Obligation export | `c600-evidence` (`obligation` module); `c600-research` (the `file` overlay row); the prelude and symbol table in the repository outside the shipped crates | The translator over `c600-obligation-vocabulary-draft-1`, its versioned symbol table and pins; `research.obligation.export` through a granted file | CT-LB5-11, CT-PS1-26, CT-RTE-23, CT-ERR-19 | The format packet's supported vector exports byte for byte with `by sorry`; a named ID, an unknown symbol, a wrong arity and a form outside the vocabulary refuse with `LB5:input.obligation_untranslatable` and write nothing; no process or network grant; no grade or gate changes |
| R10 Research panel rows | `c600-research` | Section 6's data interface through the command layer | none indexed (stage 2 work) | Each panel control invokes a row through the command layer and the palette; every read is revision-bound with its D2 state; a headless driver runs the panel's flow end to end with fresh data |

## 6. The research output panel: data interface

The stage 2 greybox runs the research output panel in `renderer/shell`, each control invoking a row (owner, 22 November
2026). Its look, layout, text and motion belong to ADR-D02 and K2; this section fixes only the data the panel reads and
the rows it invokes, all through the command layer, none answered by running language code (KN5.5).

| Panel need | Row (draft ID) | Kind | Answer or effect |
| --- | --- | --- | --- |
| Archive inventory | `research.archive.inventory` | query | Per record: Git blob and SHA-256, stage, format or its absence, recognition, integrity and interpretation results, import-table edition |
| Claims and their state | `research.claims` | query | Per claim group: family, statement and scope phrase (EVD.36), original label word for word, check mode, data status, preflight `ready` or `unavailable` with LB5.14's reasons, latest check verdict |
| One check's report | `research.check.report` | query | `CheckReport`: verdict, checker digest, input digest, per-claim trace by reference, steppable without the producer (KN6.4) |
| Evidence word | `evidence.word` | query | `c600-evidence`'s `Word` for a statement: kind, claim, row, scope phrase, named premises, source label |
| Runs | `research.runs` | query | Per run: branch, `ReproKey` digest, outcome, stop reason, declared coverage, DET.30 reading, output digests, reproduction status |
| Open obligations | `research.obligations` | query | Record-only anchors and stage 19's remaining obligations, with export status |
| Import an archive | `research.archive.import` | file | Reads granted files; publishes envelopes only through this invoked command (PS3.17) |
| Run a check | `research.check.run` | workspace, job | A KN6 check job at a digest-pinned input; writes a check record |
| Reproduce a run | `research.run.reproduce` | workspace, job | `reproduce` of a stored record; refusal or exact-payload comparison |
| Export a check bundle | `research.bundle.export` | file | PS3.17's bundle with missing dependencies declared |
| Export an obligation | `research.obligation.export` | file | R9 |
| Select a branch, stop a job | `session.branch.select`, `job.stop` | H-08 rows | As in H-08 |

Each answer carries the revision it was computed at, so the panel shows ONE.12's stale and pending states rather than
an old answer as current. A research row reads and checks; none chooses, ranks or applies a solving word, and every
searching entry stays a research tier candidate that a solving session refuses (PHS.7, the human-solve boundary).
Row IDs are drafts until the owner's answer to item 1.

## 7. Order of work and parallel sessions

```text
R0 -> R1 -> { R3, R4, R5, R6 } -> R8 -> exit
R2 --------^                       ^
R7 (independent) ------------------|
R9 (independent) ------------------|
R10 after R0, R1 and R8's job adapter
```

- Wave 1, in parallel, with disjoint files: R0 (`c600-archive`, the driver), R2 (`c600-checkers` core and the scan
  test), R7 (new `.6cs` modules in `c600-library` and their tests only), R9 (`c600-evidence`'s new `obligation` module
  and the prelude folder). Before wave 1, the line main adds the three new crates as empty members in one scaffold
  commit (`rust/Cargo.toml`, `rust/Cargo.lock`, `rust/README.md`), so no piece edits those files; R0 alone registers
  its driver in `tools/ci/run_headless.py`. Each piece's worktree is cut from the line branch, and the line main
  merges the pieces back into it.
- Wave 2, after R0 and R2 merge: R1, then R3, R4, R5 and R6 in parallel, each in its own `src/archive/sNN/` folder.
  R5 starts only once R0's report settles stage 18's start tables.
- Wave 3: R8 and R10 in `c600-research`, one session, since both write that crate; R9's overlay row joins there.
- At most four sessions run at once. Under the landing train, each piece verifies once and queues; the integrator
  merges.

## 8. What stays open at LP5's exit

- Global coverage: every universal or global research claim that was open stays open (PHS.33), stage 19's remaining
  obligations included; an admitted-domain rejection never becomes unreachability (LB5.4).
- What the archive cannot supply: the 213 groups needing data or a binding stay `unavailable` with their reasons;
  absent producers and histories, the q11 successor cache and stage 19's four low-tail array caches, sources pinned at
  a revision the repository does not hold, uncommitted helper programs behind the "reported" facts of stages 14 to 16,
  the R22B-34 negative-control words, stage 18's missing gate 3 and the lattice run's absent `check.jsonl` stay
  recorded as such. Stage 19's "independent" reviews stay model reviews (EVD.52, V14), never KN6 checks. A new derivation that replaces an absent input says so
  and carries its own checker identity (CX.17); nothing is presented as a rerun of a missing producer.
- LP6's items: product import, packaging, CT-LB5-12, final documentation and machine gates (PHS.33).
- LQ2's joint setting: whether the shipped product exposes the research profile, to be set jointly before LP6.
- The panel's look (ADR-D02, K2) and its human checks.

## 9. Readings and contradictions for the integrator

No frozen text is edited. These are recorded for the integrator to rule on or route.

The integrator adopted readings A and B, and the owner's record of 23 November 2026 routes C1 and C2 through LP5-1.

- **Reading A (KN6.1), adopted.** KN6.1 says one small checker per certificate kind and lists research archives as
  one kind, while LB5.15 and LB5.16 plan many claim-family checkers. This plan reads the research-archive kind as a
  family of sub-checkers, each with its own identity and mutation gate.
- **Reading B (PHS.31 and the brute-force tests), adopted.** PHS.31 counts no search as acceptance, while CT-LB5-04
  compares RR with brute force and CT-LB5-07 runs a search declaration. This plan reads both as checks of a recorded
  gate input or of the record contract on a synthetic fixture, whose outputs enter no research evidence.
- **Contradiction candidate C1 (H-08's reads after the freeze).** CM.18 lists the reads at the LP3 freeze, and
  PHS.30's products need host reads it does not hold (KN6's `check` among them). LP5-1 makes them ordinary additions
  (section 4, selection 6).
- **Contradiction candidate C2 (CT-EVD-12's form facts).** Chapter 06's open items record that the label table has no
  producer for `form`, which CT-EVD-12 needs. R0 takes the forms from the test's own statements, as that open item
  allows, and records the choice.
- **Note D (the restoration README).** The research README, frozen with the research, says each stage has an import
  manifest and each result a checker that imports no construction code; stages 14 to 18 have neither. This is no
  conflict with frozen language text, and LP5 edits nothing under `research/`; the integrator may route a correction
  of that README separately if the owner wants one.

## 10. Estimates

Agent-days of model work, rough; owner hours apart. They bind nothing (PHS.1).

| Piece | Agent-days |
| --- | --- |
| R0 import and inventory | 3 |
| R1 bindings and preflight | 2 |
| R2 checker core | 2 |
| R3 stage 14 | 2 |
| R4 stages 15 to 17 | 4 |
| R5 stage 18 | 3 |
| R6 stage 19 | 3 |
| R7 research library | 3 |
| R8 reproducible work | 2 |
| R9 obligation export | 1.5 |
| R10 panel rows | 2 |
| Integration, exit record, reviews | 2 |
| Total | about 29.5; about 10 to 12 working days with up to four parallel sessions |

Owner hours: about 2. The milestone card was answered on 23 November 2026; what remains is the PHS.56 explanation and
understanding confirmation at exit (about 1.5 hours), and a decision only if a stage 18 start table can be neither
found nor rebuilt.

## 11. Owner decisions

The owner answered the milestone card on 23 November 2026 with every recommended option
([LP4 and LP5 start](../../wiki/decisions/owner-decisions-2026-11-23-lp4-lp5-start.md), item 3).

1. **LP5-1, new research reads and overlay rows:** ordinary additions after the LP3 freeze, as CM.19 and CM.20,
   changing no frozen row. Not chosen: a versioned PHS.52 change of H-08.
2. **LP5-2, Lean:** no Lean run in LP5; the export is checked against the format packet's exact text. Not chosen: an
   owner-machine Lean check.
3. **LP5-3, stage 18's start tables:** an unstored table is rebuilt by exact replay of stored words from a stored
   table, as a new derivation with its own checker identity. Not chosen: waiting with the gates recorded only.
4. **LP5-4, typed claim bindings (FR-B08):** LP5 writes them as versioned data from the matrix's anchors. Not chosen:
   waiting for a separate EN4 pass.

## 12. Exit record

Written on 23 November 2026 on the line's final candidate, the head of LP5's pull request, which names the checks that
ran on it. Nothing here is the owner's: the readings below and the PHS.56 understanding confirmation are the owner's, on
one exit card. Evidence kind throughout: source (PHS.31).

### Acceptance items

1. **Certificates and verdicts rechecked and matched.** `c600-research/tests/lp5_exit_sweep.rs` drives the check from
   edition 1 of the claim bindings: preflight answers 286 groups `ready` and 224 `unavailable`, and each ready group's
   record goes through each sub-checker its binding names, with its stored inputs by role and pin, and each report is
   matched with the record's meaning, scope, original labels and check mode through R0's import. 291 self-contained
   groups less 7 with no claim checker plus stage 18's 2 composed groups made ready by rebuilt start tables give 286
   (the test's module doc names them). The 287 checks of the ready groups: 286 pass and one is not run. The fast sweep
   passes its 116 checks in a normal `cargo test`, the stage 14 to 17 sweep its 168, and the two gate 2 replays pass in
   place in `c600-checkers/tests/s18/long.rs` with the same sub-checker, record and inputs (below); the lattice replay's
   check is not run. The 114 scope-only reads answer `not_applicable` with the record's own bounds and count as no
   check. Every sub-checker's mutation gate, structural mutations included, fails on every tampered fixture (with the
   checkers' ignored stage 14, 15 to 17 and 19 tests run in release).
2. **Missing dependencies stay visible.** The 224 unavailable groups answer LB5.14's `unavailable(reasons,
   dependencies)` with the matrix's blockers word for word (`c600-archive/tests/import.rs`,
   `c600-research/tests/preflight_bindings.rs`, `c600-library/tests/research_ct_lb5_06_preflight.rs`); the sweep asserts
   that no unavailable group runs a check. Stage 18's gate 3 stays `historical_check_incomplete` and step 4 is never
   replayed for it. CT-LB5-10 passes (`check_matrix.py`).
3. **A run record reproduced bit for bit.** `c600-research/tests/r8/` (CT-DET-24 and LP2's halves of CT-DET-01,
   CT-DET-02 and CT-DET-11): a KN6 check runs as a job on an engine-owned branch at a pinned input, and `reproduce`
   returns the exact payload byte for byte. It passes on Windows here; the Windows and headless Linux comparison counts
   once both CI jobs pass on the candidate. No archive producer runs (DET.48).
4. **No new search counted as acceptance.** `tests/test_lp5_acceptance_inputs.py`: every `research/` path the acceptance
   tests read is a file of the frozen commit, unchanged since; the line changes nothing under `research/`; no acceptance
   test starts a process on a research file; CT-LB5-07's and CT-LB5-08's synthetic searches reach no evidence writer and
   read no archived record.
5. **The proof-obligation export keeps its limits.** CT-LB5-11, CT-PS1-26, CT-RTE-23 and CT-ERR-19 run whole
   (`c600-evidence/tests/obligation.rs`, `c600-research/tests/obligation_export.rs` and `tests/panel.rs`): the supported
   vector exports byte for byte through a granted file, each refusal writes nothing and stays on record, and no process,
   network, evidence promotion, commit gate or return path appears.

### Long runs and their source identity

The in-place replays ran in release from commit `b4dc6770` (gate 3's rerun from `7a0d7dd1`, which changed only its
test's assertion); no source of `c600-checkers`, `c600-kernel`, `c600-jumble` or `c600-digest` and no `Cargo.lock` entry
they use changed between that commit and the candidate; only test comments, markers and gate 3's assertion did.

| Run | Result |
| --- | --- |
| Gate 3, T6's steps 1 to 3 (639,174 letters) | pass, 2272.5 s |
| Gate 2, the rule's three stages (688,686 letters) | pass, 5973.4 s |
| Gate 2, T6's four steps (1,510,756 letters) | pass, 8831.2 s |
| The lattice phase (13,231,266 letters) | not run: 11 hours or more in release |
| The checkers' ignored stage 14, 15 to 17 and 19 tests | pass: `kn6_r4` 60 tests, `kn6_s14` 24 and `kn6_s19` 19, ignored ones included |

### Conformance index

`ct_index.py --phase LP5`: 23 IDs, 14 whole (each by union) and 9 part. [Index notes](lp5-ct-index-notes.md) name what
each part leaves. On the candidate:

- CT-KN6-01, CT-LB5-01 and CT-LB5-04 lack only ignored tests, which ran and passed in release (`kn6_r4` 60 tests,
  `kn6_s14` 24 and `kn6_s19` 19, ignored ones included). Recommended reading: whole.
- CT-LB5-05 lacks only the in-place gate 2 and 3 replays above, which ran and passed. Recommended reading: whole.
- CT-KN6-03 also lacks the lattice replay and has no sub-checker for stage 19's seven type-lex records (the archive
  defines no irregular-piece vector I_t for them) or for the cap-noninteraction claims (delta_Q5 is not defined).
  Recommended reading: part, with those records and claims kept `unavailable` and the lattice replay run once at the
  owner-machine sitting.
- CT-LB5-07, CT-LB5-08, CT-ERR-11 and CT-EVD-12 need product parts that do not exist (a research search job, host reads
  over KN2.4's branch predicates, the envelope builder). Recommended reading: indexed with the work that builds them.

LP2's seven halves: CT-KN6-02 runs whole; CT-DET-01, CT-DET-02, CT-DET-11 and CT-EVD-30 run their LP5 halves; CT-LG3-05
(the mode refusal, as the session store holds JumbleOff only) and CT-LG5-02 (no reviewed boundary classes as data) do
not. Recommended reading for those two: indexed with the work that builds them.

### Drafts for the owner to ratify

- R8: the cost rule, the metering rule, the report format and edition `6cs-draft-1`.
- R10: the row IDs, kinds, lanes and argument types of section 6, and the research-profile serve.
- R1: a binding names, besides the matrix's `required_files` and stage 18's stored stop table, step members and end
  table, the stored inputs its named claim checker reads, each a matrix file with its pin.
- Stage 19's uncovered records and claims stay `not_applicable` or `unavailable` until the archive defines I_t and
  delta_Q5.

### Deferred

Minor review findings, recorded and not fixed at the exit: R5's string per letter; R6's unrecertified
`tail_sticker_addend`, the joined cleanup table's unused pose 24 and a record with half or fewer keys read as another
record; R8's constant `ArchivedRun::reproduction()` assertion, `bound_by` taking any one matching start state and a
thread-local borrow per letter outside stop scopes; R0's missing r22b10 records, the inputs pinned only in R5's tests
and the matrix's count of a relative reference as missing; the shared mutation gate not emptying the parent list of an
element claim; the ambiguous `letters` key in gate 3's test.

### Not run on the candidate

The lattice replay (above); Look Lab's Godot checks (`tools/looklab/check.py --godot`), since the count sync changes no
Godot scene; the Windows-only native host regression, which LP5 does not touch.

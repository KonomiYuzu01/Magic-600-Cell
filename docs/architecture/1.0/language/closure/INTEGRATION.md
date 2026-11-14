# Closure integration record

The integrator's record for the closure of the freeze-readiness findings, required by plan finding PLAN-06. It is a working
record. Nothing here accepts an ADR, freezes a format or stands in for either Astra gate ruling or the owner's sign-off
(PHS.47).

## Order

PLAN-06 fixes the order: integrate, check, review, resolve blocking findings, then commit. No closure text is committed
before the review below has run on the candidate it names.

## Inputs

- Seven closure drafts `p1` to `p6` (with `p2a` and `p2b`), written in parallel against [HANDOFF.md](HANDOFF.md). Their
  manifests are in [manifests/](manifests/).
- Three reconciliation audits of the merged drafts: 11 blockers and 20 majors, all adopted. They are listed with the
  integrator's own items in [reconciliation-findings.json](reconciliation-findings.json).
- Six reconciliation packets `p7-table`, `p7-layers`, `p7-data`, `p7-lang`, `p7-render` and `p7-reg`, which corrected the
  text in draft under HANDOFF sections H15 and H16.
- The kernel interface annex in [../kernel-interface/](../kernel-interface/) and the claim matrix in
  [../claim-matrix/](../claim-matrix/).

## What the integrator changed after the packets

| Item | Change | Where |
| --- | --- | --- |
| X-PIN | Alias pin moved to `en4-2026-11-05.2`; every inventory digest refreshed; FP.15's last sentence states the pin's meaning | `format-packet/candidate.json`, `vectors.json`, `README.md` |
| X-REG | 17 requirements missing from the index added; totals and PHS.71 rewritten after comparing every defining table with the index: 418 imported, 428 with chapter 12's own rows, no ID defined twice, no class changed | `architecture/12-phases-and-conformance.md`, `closure/check_p7.py` |
| H11 rules 11 and 12 (from `p7-layers`) | `session.checkpoint.save` could not be encoded because `Event.inverse` had no null. FP.31 now gives `inverse:Ref\|null`, null exactly on a non-undoable durable action, and states that `action` classifies the effect, not the row kind. Three vectors added: the positive chain, an undo that names such a target, and a corrupted null on an ordinary workspace action | `format-packet/README.md`, `check_vectors.py`, `vectors.json`, `closure/HANDOFF.md` |
| Atlas | The banner reads the reconciled table; the atlas was rebuilt | `docs/progress/1.0/atlas/template.html` |

## Full review and fix wave

One full review of the assembled candidate ran in five read-only shards: four Astra shards (state, execution, data, route)
and one Sol shard on the mechanical checks. It reported 23 findings (4 blockers, 17 majors, 2 minors), all adopted. Four
Astra fix packets, `p8-data`, `p8-layers`, `p8-lang` and `p8-route`, made the corrections, and the integrator read each
patch. The rows below are the integrator's own changes on top of those packets. They need a separate Codex review.

| Item | Change | Where |
| --- | --- | --- |
| S3-F01 | The integrator's packet wrongly extended the exhaustive-refutation guard to `mixed(exists)` and placed it before I5. The mathematician's ruling governs: a completed exhaustive search of exactly the declared finite set refutes a plain `existence` statement only; the order is I5, EVD.41, then I6 on live leaves only; row 1 reached this way prints its scope phrase. The integrator rewrote the clauses, the reference check and the vectors to match | `architecture/06-evidence.md` (EVD.30, I6, EVD.41, EVD.42, E3's rows, CT-EVD-11, CT-EVD-22, EVD.62, CT-EVD-24), `closure/HANDOFF.md` H15.12, `format-packet/check_vectors.py`, `vectors.json` |
| Pins | Alias pin moved to `en4-2026-11-05.3`; every inventory digest refreshed | `format-packet/candidate.json`, `vectors.json`, `README.md` |
| ADR sentences | ADR-L03's open item on the wire encoding names the drafts and the freeze; ADR-L05 and ADR-L10 describe PS1.38's source experiment as sampled by one Sol review | `adr/` |
| Atlas | Rebuilt from the corrected table | `docs/progress/1.0/atlas/` |

Scoped verification round 1 confirmed 19 of the 21 blocker and major findings fixed. MC-02 and MC-04 stayed open, and it
found three new majors: S3V1-F01, S3V1-F02 and S3V1-F03 (SV1-EVD-01 is the same defect as S3V1-F01). Two Astra fix packets,
`p9-data` and `p9-table`, made those corrections. The rows below are the integrator's own changes on top of them.

| Item | Change | Where |
| --- | --- | --- |
| EVD.42 sentence | Sentence boundary and line wrap corrected after `p9-data`'s edit | `architecture/06-evidence.md` |
| Entry root | Each entry or redo with an entrance clears the observed root, so a solved-witness entry reports none. Control `s3v1-f03-witness-entry-clears-root` | `format-packet/check_vectors.py`, `vectors.json` |
| No-form reading | `flagged_without_form` in the evidence check and the absent-form reading in the exhaustive refutation. Controls `s3v1-f02-no-form-flagged-covered` and `s3v1-f02-no-form-identical-covered` | `format-packet/check_vectors.py`, `vectors.json` |

## Readings recorded here

- **X3.** `p1` names EN2 and `p3` names EN3 for the numeric word-length bound. Both hold: ENG-0J's step 5 and the 22 October
  replan give EN2 the measurement (DT-J1's experiment X3); EN3 sets the value in the interface (chapter 05's limits table,
  PS1.21). Neither has happened, so the bound stays open.
- **`p7-reg` X-REG stays `partial`** in its manifest because the packet could not make the final count. The row above is that
  count. A later requirement needs a recount.

## What this candidate does not settle

- The owner answered LQ1 to LQ21 on 6 November 2026. LQ8, LQ9, LQ13 and LQ21 wait for their evidence, and LQ2's option set
  waits for the joint setting. No ADR is accepted.
- State contract amendment A6 stays proposed until EN1's Astra check and the owner. A1 stays in force until then. The
  owner chose the nearest point of A1's grid for free mode, which settles that detail of A6's text.
- KI.15's binding holes and FR-B03 wait for LQ8 and LQ9.
- The typed corpus interpretations and the numeric X3 bound (FR-C02). `format-packet/candidate.json` stays `frozen: false`
  until the pins are refreshed for the freeze.
- FR-K01, FR-K05, FR-B03, FR-B04, FR-B08, FR-C01 and FR-C02 stay `partial` in at least one manifest. In these records
  "closed" means specified in draft: nothing has been implemented, and no conformance test exists to run.
- KN5.12 and ONE.42 stay with the owner and the design line. ONE.78 is answered (owner, 6 November 2026, decision 10).

**Correction, 14 November 2026:** The owner chose LQ8, LQ9 and LQ21: Rust for the engine and the interpreter's host,
route RT1 with the runtime in a process of its own, and capability separation (RTE.17, RTE.21, RTE.24;
[decision page](../../../../wiki/decisions/owner-decisions-2026-11-14-language-route.md)). KI.15 now names RT1's binding
as the one to write, and FR-B03 stays partial until that binding exists. LQ13 waits for LX-8, and LQ2's option set waits
for the joint setting. The list above stays as written for this candidate.

## Review of the earlier candidate

This section records the review of the candidate as it stood before the owner's answers. Its review allowance ended with
three major findings open. `AGENTS.md` then sent the decision to the owner, whose choice is at the end of this section. The
final candidate has its own record below.

Each digest is the SHA-256 of the sorted lines `<sha256 of file>  <path>` over the uncommitted documents, this file excepted.

| Stage | Candidate digest | Files |
| --- | --- | --- |
| Full review | `01dd42f59615885d2e7b6687addb4dbdc27b72681219713e14c2e8446f8c65b7` | 65 |
| Verification round 1 | `bca8cf829975b70b347ad1ad9e833eb83734577bf7fc37b50355a28e8e6f195d` | 69 |
| Verification round 2 | `f3b5ed37f4047fa215466b33112d34003f67a191a8de908fdd6c38d2f6999e63` | 69 |

| Stage | Shard | Model | Call | Result |
| --- | --- | --- | --- | --- |
| Full | State | Astra | `20261105T101810Z-d2d2c76d` | S1-01 to S1-05, all adopted |
| Full | Execution | Astra | `20261105T101812Z-f20a8e92` | LANG-01, adopted |
| Full | Data | Astra | `20261105T101814Z-ed48e814` | S3-F01 to S3-F04, all adopted |
| Full | Route | Astra | `20261105T101816Z-16536bc0` | S4-01 to S4-06, all adopted |
| Full | Mechanical checks | Sol | `20261105T101818Z-92590270` | MC-01 to MC-07, all adopted |
| Round 1 | State | Astra | `20261105T113453Z-a0403f7c` | Pass: S1-01 to S1-04 fixed |
| Round 1 | Data | Astra | `20261105T113454Z-c6e99cd5` | S3-F01 to S3-F04 fixed; new majors S3V1-F01, S3V1-F02, S3V1-F03, adopted |
| Round 1 | Route | Astra | `20261105T113456Z-fea55f8a` | Pass: LANG-01 and S4-01 to S4-05 fixed |
| Round 1 | Mechanical checks | Sol | `20261105T113458Z-a679aff2` | MC-01, MC-03, MC-05, MC-06, MC-07 fixed; MC-02 and MC-04 not fixed; new major SV1-EVD-01, adopted |
| Round 2 | Data | Astra | `20261105T122633Z-62b453ec` | S3V1-F01, S3V1-F02, S3V1-F03 fixed; new major S3V2-D01, adopted |
| Round 2 | Mechanical checks | Sol | `20261105T122635Z-1489f132` | MC-02, MC-04 and SV1-EVD-01 fixed; new majors SV2-CHK-01 and SV2-CHK-02, adopted |

Other-model review: Sol's shard covered the mechanical checks of the Astra-authored drafts in all three stages; it did not
read every clause. The integrator read each Astra patch. Astra and Sol both read the integrator's own rows above in round 2,
and Sol confirmed that the no-form and solved-witness controls detect the removal of their fixes.

Astra listed the defect of SV2-CHK-02 as a verified minor and Sol as a major. The integrator adopted it as a major, because
the packet's claim is stronger than its evidence.

Every full-review and round 1 finding is closed. The three that remain are in the draft reference program
`format-packet/check_vectors.py`. No clause was faulted, but `format-packet/README.md` lines 626 and 709 claim more than the
program checks.

| Finding | Model | Defect | Integrator's check |
| --- | --- | --- | --- |
| S3V2-D01 | Astra | A leaf made inert by EVD.41 skips the structural I2 checks: an unrelated `negation` reads row 7 where I2 requires row 0 | Reproduced |
| SV2-CHK-01 | Sol | Recovery derives the retained suffix without checking event parents against the recovered Positions, so a suffix that omits a retained move is accepted | Confirmed by reading the code |
| SV2-CHK-02 | Sol | Undo leaves `pose_root` bound to an undone entry; the root should follow the resulting Position | Reproduced |

The owner's options were: fix each with a stored control and commit under an owner exception; fix and run a third scoped
round, which also needs an owner exception; or commit nothing until the final candidate is reviewed. The owner chose the
third on 5 November 2026. This text stays uncommitted. The integrator fixes the three findings in the final candidate, each
with a stored control that fails before and passes after, and that candidate gets its own full review.

## Final candidate

The owner answered LQ1 to LQ21 on 6 November 2026 and gave the follow-up answers the same day
(`docs/wiki/decisions/owner-decisions-2026-11-06.md`). The owner returned the team to the original workflow, so the
integrator writes and integrates, Sol does the routine parts and Astra does what `AGENTS.md` assigns.

### Plan

| Stage | Model | Call | Result |
| --- | --- | --- | --- |
| Plan check | Astra | `20261106T133219Z-9e7e9796` | PF-01 to PF-09, all adopted |
| Scoped re-check 1 | Sol | `20261106T142806Z-5e417b48` | PF-02 not answered; new major NF-01; both adopted |
| Scoped re-check 2 | Sol | `20261106T164209Z-53c36418` | PF-02 and NF-01 answered; new majors NF-02 and NF-03, both adopted |

No third plan re-check ran. `AGENTS.md` allows one plan check per task and a scoped re-check of a changed plan, the two new
findings were narrow, and their fixes are text that the candidate's full review reads directly. NF-02: RPI.72 step 2, RM.40's
line-only vector, RTE.83 and CT-RTE-22. NF-03: PS1.44's `actions` map, EV4.18 and CT-EV4-20.

### Who wrote what

| Part | Writer | Call or note |
| --- | --- | --- |
| `00-overview.md` to `03-libraries.md` | Sol, patch read by the integrator | `20261106T145519Z-e01f9ee2` |
| `04-environment.md`, `05-persistence.md` | Sol, patch read by the integrator | `20261106T150010Z-eede975d` |
| `06-evidence.md` to `08-one-state.md` | Sol, patch read by the integrator | `20261106T150015Z-116605e4` |
| `09-determinism.md`, `11-route-and-spikes.md`, `13-errors.md` | Sol, patch read by the integrator | `20261106T164141Z-5afd99b3` |
| ADR status notes, `adr/README.md`, `language/README.md`, the partition, DT-L1's note | Sol, patch read by the integrator | `20261106T164148Z-89662214` |
| `10-renderer-platform.md`, `renderer-feature-matrix.md`, `12-phases-and-conformance.md`, ADR-L11 | Integrator | Stage contract RPI.68 to RPI.80, further views RPI.81 to RPI.83, RM.38 to RM.40, withdrawn clauses and test rows |
| `jumbling-interfaces.md`, `layer-interfaces.md`, H-11 in the protocol and the experiment plan, A6 in `research/jumbling/state-contract.md` | Integrator | A6 is a proposal, not in force |
| `command-rows-language.json`, `build_command_table.py`, `test_command_table.py`, `check_p7.py`, `command-table.md`, the atlas banner | Integrator | Six added rows and two amended notes; the builder takes both deltas and refuses a field amended twice |
| `kernel-interface/` | Integrator | Sources, BND.84's provenance step, one more rejection control |
| `format-packet/` | Integrator | FP.2, FP.3, FP.13, FP.19, FP.20, FP.29, FP.37; playback vectors; pins refreshed |
| Every fix after the full review | Integrator | Document edits; PS1.46, RM.41 and FP.38; the seven new vector families; the three checkers. The owner asked to save Codex quota, so no implementation call ran, and the scoped rounds below are the separate reading |

Chapter edits by the integrator on top of the Sol patches: clause tags, withdrawn clauses, the wrap of a few long lines and
PHS.69's sentence about the deltas.

### The three findings the owner kept for this candidate

Each has stored controls in `format-packet/vectors.json`, each fails before the fix and passes after it, and the integrator
read the change. The full review below read these fixes and confirmed them closed within their synthetic scope.

| Finding | Fix | Controls |
| --- | --- | --- |
| S3V2-D01 | Structural I2 checks run on a leaf that EVD.41 made inert | Five vectors `s3v2-d01-*` |
| SV2-CHK-01 | Recovery checks event parents against the recovered Positions | `sv2-chk-01-rebound-retained-parent` |
| SV2-CHK-02 | Undo, redo and a branch change bind `pose_root` to the resulting Position | Five vectors `sv2-chk-02-*` |

### Readings recorded here

- The tag check `work/p8/accept_tags.py` (private) reported five baseline test IDs gone from four ADR drafts. They are range
  ends that moved when chapters gained rows: every `CT-X-01 to CT-X-n` range in `adr/` now ends at the family's highest
  defined number, and all five IDs stay defined and indexed in chapter 12.
- The manifests `manifests/p2a.json` and `manifests/p7-table.json` keep the old digest of `command-rows-jumbling.json`. They are
  records of what those packets saw, so they stay unchanged. The new table is pinned by the checks, not by those manifests.
- A reading note on the decision page says that 1.0 has the export of proof obligations only. A Lean result has no route
  into a record in 1.0, and `formal proof` has no writer in 1.0 (PS1.45, EVD.64). The owner confirmed the wider reading in
  decision 9.
- Check runs after the last text change: see the review record below.

### Review of the final candidate

The full review read `ade59fee` in a private clone. The candidate digest is the SHA-256 of the sorted lines
`<sha256 of file>  <path>` over the uncommitted files, leaving out this page:
`b2d9e42c92e25ee24a97ce94b0b304ae09517346aeeb2d39906f3223ca0937f4`, 83 files.

| Shard | Model | Call | Findings | Blocking |
| --- | --- | --- | ---: | --- |
| State and interfaces | Astra | `20261106T233208Z-e1e69781` | 3 | S1-01, S1-02 |
| Execution | Astra | `20261106T233211Z-9c1da8c0` | 5 | S2-F01 to S2-F05 |
| Data and formats | Astra | `20261106T233214Z-19bd1fd0` | 7 | S3-FINAL-01 to S3-FINAL-06 |
| Route and phases | Astra | `20261106T233217Z-85f74caf` | 4 | S4-01 to S4-04 |
| EN1's check of A6 | Astra | `20261106T233220Z-695aec17` | 3 | A6-001, A6-002 |
| Reference checkers | Sol | `20261106T233223Z-9943aad2` | 4 | FC-MC-01 to FC-MC-04 |
| The owner's answers | Sol | `20261106T233226Z-09941f86` | 2 | OWNER-F01 |

The review gave 28 findings, 24 of them blocking, and every one was answered `adopt`. Six are blockers: S2-F01, S2-F02,
S3-FINAL-01, S3-FINAL-02, S4-02 and S4-01. The four minor findings S1-03, S3-FINAL-07, OWNER-F02 and A6-003 were fixed in
the same pass. Three pairs are one defect each: S3-FINAL-02 with S4-02, S2-F03 with S3-FINAL-03, and S3-FINAL-06 with
FC-MC-03. The reviewers found no further departure from the owner's answers.

| Scoped round | Model | Call | Candidate | Result |
| --- | --- | --- | --- | --- |
| 1 | Sol | `20261107T025133Z-a718ef54` | `6825fe2b` | The 22 listed findings fixed; new major SV-F01, adopted |
| 1, second reading of A6 | Astra | `20261107T025136Z-00533242` | `6825fe2b` | A6-001, A6-002 and A6-003 fixed; new major A6-004, adopted |
| 2, the last allowed | Sol | `20261107T034317Z-43f3b33f` | `5eb760dd` | SV-F01 and A6-004 fixed; new blocker SV2-F01, adopted |

Round 1's candidate digest is `ab22635574e7498e8709ac099adce8707f60533adbcfe24565b7ac561d8d7ac4`. Round 2's is
`bf7b49c34138751f0c6dbf97efdd0414786d19a6df6f5d73b23b8ed10c01931c`. Both cover 83 files.

SV-F01: `pick_result` and the `last` branch of `playback_storage_result` checked the Ref but never compared the resolved
record's header with it. Both now compare `format` and `version`, and FP.11, FP.37 and FP.38 say so. A6-004: "every distance
is exact in Q" was false for the plain Frobenius distance, so free mode now compares and records the squared distance, and
CT-KN3-04 holds the counterexample at 8/5. Sol's round 2 confirmed that fix, which is the sentence Astra suggested. To save
the owner's quota no separate Astra call read it. The architecture-freeze ruling reads A6 again, and EN1's Astra check of A6
ends there. A6 stays proposed.

SV2-F01: the sentences added to FP.37 and FP.38 for SV-F01 gave a version mismatch another refusal code than FP.10's
`PS2:input.persistence_version`. The checkers were already right, because `record_header` runs first, so only the text
changed: FP.37 and FP.38 now say that FP.10's refusal comes first, as the reviewer suggested. That fix is `a2a547e3`, with
digest `b51e0ac22f9abe71c495fb4790eda97afa77492acc81bb24f806a692ad5fa682`. Both scoped rounds were used, so no model has read it. The integrator offered the owner an exception
or one more narrow Sol call, and on 7 November 2026 the owner chose the exception (decision 16 of
`docs/wiki/decisions/owner-decisions-2026-11-06.md`). The architecture-freeze ruling reads these clauses again.

The owner's choice of A1's grid for free mode was given on 6 November 2026 in a side session and reached the integrator after
the last round. It is written into KN3.7, A6's text in `research/jumbling/state-contract.md`, ADR-L03 and the chapters that
cite them (`d2ef99b7`). The text already used the grid, so the changes are who made the choice and the removal of the
alternative sentence. These lines record an owner decision, and the freeze ruling reads A6 again.

The digest of the text committed here, by the same rule, is `1ecee1a4026c30e741d9ba4f3a54089e5bc442149dfb9bfbc715d8b384920ba2`, 83 files. The same commit carries this page,
`docs/wiki/index.md` and `docs/wiki/log.md`.

Controls, each stored and run on the old logic first:

| Control | Old logic | Now |
| --- | --- | --- |
| `durable_invalid` `corrupt-revision-head`, `corrupt-revision-workspace` | `accepted` | `PS3:input.checkpoint_replay` |
| `playback` `playback-undo-no-hint`, `playback-redo-no-hint` | undone with no hint | `EV4:stale.recording_step` |
| `playback_trace` `skipped-step-stale` | undone | `EV4:stale.recording_step` |
| Annex rejection controls: a tool block on the walk's word, directly and in a part | Control failed, twice | Detected; 34 controls pass |
| `generator-returned-owner` | `KeyError` | Draw 0 of the stream |
| `generator-nested-same-name` | A draw of the inner stream | Draw 0 of the outer stream |
| `picks` `pick-wrong-header` | Accepted | `PS1:capability.provenance_incompatible` |
| `playback_storage` `playback-last-wrong-header` | Accepted | `PS1:input.persistence_format` |

`pick-version-first` and `playback-last-version-first` are pins, not failing controls: both give
`PS2:input.persistence_version` on the logic before SV-F01's fix and after it.

The 38 earlier contract traces gave the same results after the checker changed. The expected values of the new format
vectors came from separate generators with `hashlib`, not from the checker.

Noted and left unchanged:

- `sv2-chk-02-redo-later-crossings` still passes one last-entry-root mutation that the four other `sv2-chk-02-*` controls
  detect. The defect stays covered by those four.
- `check_matrix.py`'s main creates, writes and deletes `build/`. It leaves nothing behind.

Checks on the committed text: `test_command_table.py` 37 tests; format vectors 377; contract vectors 40 traces and 13
boundary vectors; `check_closure.py --all` and `check_p7.py --all` pass; annex 21 operations, 2 walks, 58 steps and 34
rejection controls; claim matrix 497 records and 10 negative controls; test index 491 rows defined, 1 withdrawn, 481
indexed; wiki lint ok. These are mechanical checks, and they cannot show that the text is right. `tests/test_oracle.py`
ended with 13 tests and 2 environment errors (WinError 1314) on the Windows machine. No Windows, DirectX or performance
claim follows from any of this.

Nothing is accepted or frozen: no ADR, no A6 and no format. The freeze path is the two gate rulings and the owner's
sign-off.

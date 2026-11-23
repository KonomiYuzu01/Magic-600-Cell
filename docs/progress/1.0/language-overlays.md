# Overlay records on frozen language text

Status: register started on 23 November 2026 by the LP4 line. An overlay record says how a frozen clause is read or
completed after an owner decision, where editing the clause would change bytes that must stay as they are, such as a
file of the frozen format candidate (`docs/architecture/1.0/language/format-packet/candidate.json`). An overlay record:
- edits no frozen text;
- accepts no ADR;
- changes no format.

The command layer's readings of 23 November 2026 are recorded with their rows in [the command table](command-table.md).

## OV-1. What a run record holds for replay (chapter 05; EV2.9)

- **Authority.**
  - Owner, [23 November 2026](../../wiki/decisions/owner-decisions-2026-11-23-lp4-lp5-start.md), item 2.3: replay
    inspects recorded effects and never re-executes them, as a PHS.52 revision of chapters 05 and 09.
  - The integrator's ruling of the same day: chapter 05 is a file of the frozen format candidate, so its bytes stay
    as they are, and its part goes here. Chapter 09's part is DET.7, amended in place.
- **Clauses.**
  - Chapter 05's row for the run record: `c600-run-draft`, reserved under FP.4, in the table of record kinds.
  - Chapter 04's EV2.9.
  - Chapter 09's DET.7 as amended.
- **Reading.** Beside its reproducibility key (`ReproKey`, DET.1), a run record that is to be replayed holds the run's
  host trace in order:
  - each read, with the input it was answered with;
  - each tariff charged, since EV2.4 compares the charges;
  - each preview or sink effect, with its row, its request and the bytes it produced;
  - the outcome.

  Replay answers reads from those captured inputs and compares each effect at the same trace position. Nothing outside
  the record is captured or consulted: there is no isolated replay sink, ambient file or clock.
- **What it changes.** Nothing in the frozen format. `c600-run-draft` stays reserved, and its form is written within
  FP.4's reservation when the record producer is built. A producer whose effectful runs should be replayable captures
  the trace above.
- **A record without the trace** stays inspectable as recorded evidence. Replay refuses an effect the record does not
  hold with `EV2:capability.debug_write_forbidden` and writes nothing.
- **Tests.** CT-EV2-10 (`rust/c600-debug/tests/replay.rs`).

## OV-2. The 1.0 log file's record (chapter 05's catalogue; ADR-D01.80 to .84)

- **Authority.**
  - ADR-D01.80, .81 and .84 (`docs/architecture/1.0/design/adr/ADR-D01-rev5-documents.md`, frozen at A7): LP4
    builds the 1.0 log format and its compiler, and names the extension.
  - The rule of [22 November 2026](../../wiki/decisions/owner-decisions-2026-11-22-adr-d01-overrules.md): where
    ADR-D01 conflicts with a frozen clause, ADR-D01 wins.
  - The integrator's adoption of this record on 23 November 2026.
- **Clauses.** Chapter 05's PS1.12 and its catalogue of formats, and FP.4's list of format identifiers.
- **Reading.**
  - PS1.12 says that nothing in the catalogue freezes a format. The log's record is `c600-log-draft`, version 1.
  - Like the descriptors PS1.12 names, it keeps `-draft` until its own packet.
  - The file is plain canonical JSON, or stored gzip above 250,000 bytes, with the extension `.c600log`.
- **What it changes.**
  - The log is a product exchange file written from the session store. It adds no session-store record kind.
  - It changes no byte of the frozen migration format or of chapter 05.
  - Its FP.4 row and its catalogue row follow at the format's next versioned change.
  - The extension, the gzip threshold and the MPUlt cap (0.4's: 10,000 native moves, 20,000 sequence entries,
    16 MiB) are the integrator's defaults.
  - `c600-log` has path dependencies only. A real compressor would be a product dependency and goes to the owner.
- **Tests.** `rust/c600-log/tests/` (compile gates, dependencies, interchange).

## OV-3. The learning path's step order (chapter 04, EV5.6 and EV5.18)

- **Authority.** The integrator's ruling of 23 November 2026: record the change here, and do not edit EV5.6. The
  owner chose the manual as the source of first use on 23 November 2026
  ([decision](../../wiki/decisions/owner-decisions-2026-11-23-lp4-lp5-start.md), item 2.5). The mathematician's
  pass checks this record together with the learning path. Where the mathematician disagrees with a move, the concept
  keeps its frozen step and the conflict goes to the integrator.
- **Clauses.**
  - Chapter 04's EV5.6, the seven steps and their concepts.
  - EV5.10's term check: a definition uses terms at its own level or lower.
  - EV5.18's qualified names.
- **Reading.** A concept cannot be taught after a concept whose definition uses it. Where a definition taught at an
  earlier step uses a concept that EV5.6 places at a later step, the concept is taught at the earlier step and
  revisited at its frozen step. The path is `docs/progress/1.0/glossary/learning-path.json`, with its rows in
  `terms.json`.

  | Concept (row) | EV5.6 step | Step in the path | Used by (definition at the earlier step) |
  | --- | --- | --- | --- |
  | Theorem (`theorem`) | 5 | 1, revisited at 5 | Basis, Reachability claim |
  | Certificate (`certificate`) | 7 | 1, revisited at 7 | Basis |
  | Digest (`digest`) | 7 | 1, revisited at 7 | Job result |
  | Identity level (`identity-level`) | 6 | 1, revisited at 6 | Specification, Digest |
  | Word (`word`) | 4 | Taken from the manual; step 4 revisits its named alphabet and identity | The manual's chapter 1 teaches it first, with jumbling words only |
  | Flow rule (`flow-rule`) | 6 | Waits at 6 | Not learner-facing until LQ2 decides (EV5.9) |

  Eleven concepts that EV5.6 does not list are taught where a definition first needs them:

  | Concept (row) | Step | Used by |
  | --- | --- | --- |
  | View (`view`) | 1 | Base revision |
  | Statement (`statement`) | 1 | Claim, Standing, Statement form, Negation pair, Evidence graph, Theorem |
  | Evidence (`evidence`) | 1 | Claim, Evidence words, Verdict |
  | Admitted domain (`admitted-domain`) | 1 | Covered set |
  | Sticker (`sticker`) | 1 | Identity level |
  | Source range (`source-range`) | 2 | Diagnostic |
  | Keyboard key (`keyboard-key`) | 3 | Binding |
  | Menu letter (`menu-letter`) | 4 | Cost alphabet |
  | Revision reference (`revision-reference`) | 4 | Recording |
  | State derivation (`state-derivation`) | 6 | Derived state |
  | Gauge (`gauge`) | 7 | Exact transport |

  Specification and Not meant stay at step 1, as EV5.6 places them. Each records a forward use of Principle record,
  which step 5 teaches, as EV5.6's own note allows. Assistance provenance keeps its frozen step 4. Workspace record,
  taught at step 3, records a forward use of it, because its "where required" applies only to stored words (PS1.32,
  LB7.3), which step 4 teaches. The step numbers do not change. The dependency order is:
  - steps 2, 3 and 4 after step 1;
  - step 5 after step 4;
  - step 6 after steps 2, 3 and 5;
  - step 7 after step 6.
- **Other readings in the glossary.** These are recorded in `terms.json` and rendered into `glossary.md`.
  - "Twist" is a homonym with no bare sense: `twist-full` and `twist-jumbling`.
  - Bare words read as follows. Key is the keyboard key and block the achievement block (EV5.18). Macro is a macro.
    Witness is the witness word. Chamber is the cut chamber, LP4's reading, since EV5.18 sets only cut chamber against
    flag chamber. Every other homonym is refused when bare.
  - "Protection level" reads as identity level, and the flow label's "provenance" as assistance provenance.
  - Census rank: EV5.18's row says what the rank counts is not established. The mathematician cites Theory Lemma 6,
    r(A) = |M(A)| - 1, which holds on all 35 orbits of `census.json`. The row now states it and is learner-facing.
  - Cap cover uses manual 2.2's definition. That adds an eighth recorded first-use exception, because chapter 2's
    introduction mentions cover before section 2.1 names cut cells.
  - Section 6's ambiguous citations are qualified in each row's `source` field, under partition default 19:
    - "Q7 (a)" becomes "DT-M1 Q7 (a)";
    - "default 10", "default 9" and "default 7" become "partition default" 10, 9 and 7;
    - "access policy: LQ14" becomes "LQ14 (access policy)".
- **What it changes.** No frozen text. The term check (`build.py --check`) and `tests/test_glossary.py` enforce the
  order.
- **Tests.** CT-EV5-02 in part (`tests/test_glossary.py`). BS3's branch waits for LQ2. The review clause, and the
  check of the flow goals against the disclosure data, wait for LP4-11's `c600-command` part.
- **The mathematician's pass, 23 November 2026.** The mathematician agrees with every move above and with the eleven
  additions. They disagreed with moving assistance provenance to step 3, so it keeps its frozen step, as stated above.
  They asked that step 4 revisit Word, also stated above. Their findings on the learning path, the principle records
  and the glossary rows are fixed in the same pull request.

## OV-4. Where recordings and playback states are kept (chapter 05, PS1.44; FP.37)

- **Authority.** The integrator's ruling of 23 November 2026 for LP4-9: if the payloads' carrier departs from chapter
  05's catalogue, the departure is recorded here and no frozen text is edited. PS1.12 says that nothing in the
  catalogue freezes a format.
- **Clauses.**
  - Chapter 05's PS1.16 (the workspace record's carrier), PS1.30 (the recording) and PS1.44 (the playback state).
  - The catalogue's rows for a recording (`c600-recording-draft`, reserved under FP.4) and a playback state
    (`c600-playback-draft`, FP.37).
  - The format packet's FP.12 (a workspace record's fields) and FP.37 (the playback state's wire form).
- **Reading.**
  - Recordings and playback states are records of the command layer's journal. That journal has the draft format
    `c600-command-journal-draft-1` (`rust/c600-session/src/command_store.rs`).
  - Its record kind `command_workspace` holds `id`, `kind` and `value`. Such a record takes no action number, no
    transaction cites it, and it creates no undo target, as PS1.44 requires.
  - A recording's value is a record object under FP.9, of kind `c600-recording-draft`. A playback state's value is
    inline, of kind `c600-playback-draft`.
  - The value bytes are draft JSON, not FP.37's wire form. FP.12's `dependencies`, `action` and `revision` fields are
    not written yet.
- **What it changes.**
  - Nothing in the frozen migration format: the command journal is a draft.
  - FP.37's wire form and its `playback_storage` vectors are written when the command journal moves to the migration
    format. Until then, a reader built before LP4-9 stops at the new record kind.
- **Open.** Reset and import are not built on the command layer's path: they answer `KN5:input.row_not_built` and
  write nothing. A reset or an import that keeps the session must say what happens to a playback state's `actions`
  map. LP4-14 builds `file.log.import` and states its reading.
- **Tests.**
  - CT-EV4-14 and CT-PS1-25 on the journal (`rust/c600-durable/tests/ct_ev4_14_playback_crash.rs`).
  - The carrier (`rust/c600-durable/tests/ps1_44_carrier.rs`, `rust/c600-session/tests/recorder.rs`).

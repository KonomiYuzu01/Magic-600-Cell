# H-08 Command table and layer boundaries

Status: **frozen at LP3 (chapter 12, PHS.53)**. The merge of the command-table freeze pull request is LP3's frozen candidate, on which the owner signs off C10 ([LP3 exit decisions](../../wiki/decisions/owner-decisions-2026-11-20-lp3-exit.md)). An item marked OPEN is not frozen, and a later change goes through PHS.52. The table joins the signed 3 October dispositions and carries DT-M1's fix-first items 2 to 4 and the [22 October answers](../../wiki/decisions/owner-decisions-2026-10-22-dt-m1.md). The owner answered LQ1 to LQ21 on [6 November 2026](../../wiki/decisions/owner-decisions-2026-11-06.md). The owner accepted the language ADRs and froze the language architecture on [15 November 2026](../../wiki/decisions/owner-decisions-2026-11-15-freeze.md). The design line remains closed.

- **CM.1 [D]** [build_command_table.py](build_command_table.py) builds [command-table.json](command-table.json) from its retained rows, the signed join, [command-rows-jumbling.json](command-rows-jumbling.json) and [command-rows-language.json](command-rows-language.json). The stored table has 175 commands: 152 retained-purpose and DT-M1 rows, 16 jumbling additions and seven language additions (the six of 6 November and LP3's record write `workspace.set`, CM.17), with six existing rows amended by the jumbling delta and two notes by the language delta. It has `final: true` and equals a fresh default build. Nine service registrations (CM.16) and 30 bridge reads (CM.18) are separate. `--draft-without-jumbling` builds the 152-row base with `final: false` for isolated checks. `final: true` marks a non-draft build; equality with a fresh default build and the delta checks show completeness. It is not an architecture approval. The working forms are HANDOFF H15.

```text
python -B docs/progress/1.0/build_command_table.py
python -B docs/progress/1.0/test_command_table.py
python -B docs/architecture/1.0/language/closure/check_closure.py --packet p2a
```

The builder also accepts `--delta <path>`, repeatable and applied in order in place of the default pair, and `--out <path>`. It validates everything in memory before writing one output file. It writes UTF-8 LF bytes. Importing the builder and running the tests write nothing, including bytecode.

- **CM.2 [D]** [command-join.json](command-join.json) has one record for each of the 556 signed decisions, including each numbered split part. It records source rows and a digest of the signed decision, the commands serving its purpose, or `outcome: none` with a reason. No `delete` or `automate` decision contributes a command. A coarse inventory row can occur in several split parts; its other commands do not automatically belong to each part. A command carries every contributing decision and the sorted set of their dispositions. An empty disposition list denotes a new DT-M1 row with its own source, not an undecided 0.4 disposition.

The source census reproduces DT-M1 E1: 377 keep/redesign decisions have a key, button, menu, command, mouse or control entry. Of those, 268 carry legacy command IDs and 109 are mapped individually. Six of the 377 are routing or reporting obligations rather than invocable commands: U4-010, U4-032, U4-063, U4-064, U6-047 and U6-048. Each has its own `none` reason; none is silently dropped or reclassified. Across all 556 decisions, 493 map to commands and 63 have a stated non-command host or a signed delete/automate disposition.

- **CM.3 [D]** [command-aliases.json](command-aliases.json) is the versioned candidate alias table for H12 and PS3.3 to PS3.8. Version `en4-2026-11-05.3` covers the 270 raw legacy IDs carried by retained user-entry decisions, plus the deleted planner ID. Its 211 pattern groups implement E1's nine normalization patterns; these are the measured groups for this draft, rather than a claim that the earlier estimate of about 205 was exact. Every raw spelling remains explicit, including `NativeWorkbench.KeyAction:` spellings. Alias targets are purposes with arguments, not newly invented commands per legacy key.

The content digest is SHA-256 over the entire alias object except `content_digest`: UTF-8 JSON, recursively sorted object keys, array order retained, compact comma/colon separators, `ensure_ascii=false`, no trailing newline. Migration pins both `version` and `content_digest` from this file. An alias with missing captured object data remains unresolved. A legacy parameter such as an axis number or list row cannot be invoked before it is resolved to its absolute object. Action hints, previews, checked files and user-action tokens are fresh invocation guards; none is migrated as permission. Undo/redo aliases name only `session.undo` or `session.redo` and acquire a fresh `ActionHint` and token. Dispatch by action class is internal (CM.5). Twist bindings migrate by the exact old word, never by the spelling of T3 or its inverse.

Old bank-scoped bindings have no target scope under DT-M1 K6. Preserve their raw bank, command and arguments as unresolved, install no active binding, and require an explicit checked rebind. Bank-switch aliases also have no target; the command join records their retained purposes under the new anchor or command access. `functions-toggle` reaches the palette, not a Functions bank. The alias candidate still requires the integrator's semantic review and migration vectors; its checksum proves identity, not correctness.

Every required target argument must have a literal entry in `arguments`, a named `name: ...` resolution requirement in `requires`, or a nonempty acquisition rule in `invoke_requires`. A trailing `?` on the row's type marks an optional input. General prose supplies no binding; the builder rejects any missing required argument for every alias. `review` binds a resolved draft or pinned word, explicit requested `fields` and positive `max_chunk` from the checked binding or solver invocation, and acquires a fresh snapshot with `kernel.snapshot.open(current)` under LI.11 for each invocation. Missing fields or chunk limit remain unresolved, with no implicit defaults. `filter-apply` submits a `FilterPreviewRef` under KN0.9/LI.41, without presentation or pending word-preview dependence. Undo/redo hint acquisition supplies the corresponding direction under LI.9.

- **CM.4 [D]** A build refuses unknown inventory IDs, duplicate command IDs, values outside a vocabulary, a missing decision or split part, a changed signed-source record, a command on a delete/automate decision, a missing non-command reason, a missing alias, or an invalid alias digest or target. It refuses repeat outside CM.6's allowlist, a commit outside the commit lane, inconsistent preview/gate fields and a live/draft twist or timer row that violates CM.7. H2's delta refusals apply: adding an existing ID, amending a missing ID, an incompatible `expect`, two amendments of one field, an invalid value, or a missing delta without the draft flag. An addition supplies every H2 field and its explicit argument schema; an amendment supplies nonempty `expect`, `set` and `source`, includes its full argument schema and cannot rename an ID (CM.15). Lane targets, service registrations and bridge reads are checked under CM.13, CM.16 and CM.18. The draft flag and a delta cannot be combined. Removing U4-069/3 fails coverage even while U4-069/1 and /2 remain.

## 1. The six layers

[LI.3 and LI.4](layer-interfaces.md) define ownership and call paths: shell to command layer to engine, with engine-coordinated store transactions. Results pass through the view model and renderer adapter. The right-hand column names the boundary output.

| Layer | Owns | Must not | Boundary output |
|---|---|---|---|
| **Engine** | Mathematics: the model, legal moves, effects, protection checks, previews, names and residuals. It is proven equal to 0.4 by the differential oracle. | Hold UI state, read the clock for decisions, or write files. | Results with an evidence state (`verified`, `recorded`, `unknown`, `unchecked`), typed errors, and the availability of each command |
| **Session store** | The journal, checkpoints, workspace (drafts, macros, worksheets, keymaps, filters) and preferences, plus migration import. One transaction per commit (R-06). | Interpret mathematics; write a response while holding the session lock (R-15). | Revisions (`state`, `workspace`, `protection`), durable command receipts, recovery after a crash |
| **Command layer** | Command IDs, gates, preview rule, undo class, lanes, routing and job handling. It is the only entry for anything that changes state, workspace or files. | Grant permission from a cached availability hint, or have a second path that skips a gate (R-05, R-07). | One `invoke(command_id, args)` with a receipt, and revision-bound availability under KN5.8 and H5 |
| **View model** | What every panel shows: selections, the forecast in view, filter results and inspection. It is built from engine snapshots and revisions. | Call the engine synchronously on the UI thread (R-14); keep its own copy of the puzzle state. | Observable view state; it updates only when its inputs change (R-19) |
| **Shell** | Layout, panels, motion, focus and input routing (keys, pointer, onscreen keyboard), following the layout specification and keyboard model from the design track. | Change state except through the command layer; enable a control the engine says is unavailable. | Command invocations; view and camera state for the renderer |
| **Renderer** | Drawing of 259,800 stickers, the frame and overlays; picking from a screen point to a sticker. The renderer line owns backend selection. | Change or relabel mechanical state; present an uncommitted value as the session state. Identified previews and derived display plans are allowed under CM.8. | Frames, pick results, device-loss reports (R-17) |

Rules that cross the layers (R numbers refer to [requirements-from-screening](requirements-from-screening.md)):
- **Commit outcome is separate from refresh.** A committed command is never reported as failed. "Applied, display out of date" is its own state (R-02).
- **Durable command identity.** A retried command keeps its ID and is committed at most once ([1.0 architecture V03.5](../../architecture/1.0/10_V1_ARCHITECTURE.md), proposal).
- **Typed errors.** The error categories are those of the architecture V06 proposal: input, unverified reference, protection, stale, cancelled, resource limit, consistency, unknown outcome, auth and busy. Each category has its own continuation. A hard failure is never shown as a low score.
- **Three things the shell never merges:** evidence state, score and permission.

## 2. Command row format

| Field | Meaning |
|---|---|
| `id` | A stable dotted ID, for example `operation.execute`. Keys, buttons, menus and scripts all name this ID. |
| `label` | A working name; the final wording comes from the design track. |
| `kind` | `commit` uses the atomic commit/history path, including named undo/redo. `workspace` changes drafts, protection, macros or preferences. `view` changes presentation. `query` is read-only. `file` writes an export. `app` controls the process, a job or clipboard output. |
| `inventory` | The top-down inventory rows it serves. |
| `gates` | `protection`, `fresh_preview`, `fresh_filter_preview`, `confirm` (explicit scope confirmation), `model_match`, `file_check` (the same checked file digest). Word and filter freshness use their separate predicates (CM.14). `not_busy` is absent. |
| `lane` | One of H5's `commit`, `exclusive`, `latest_wins`, `program`, `view`, `none`. Admission and revision-change behavior belong to H5 and KN7.2, not another definition here. |
| `lane_target` | Required on `view` and `latest_wins` lanes: exactly one of `arg`, `view` or `control`, resolved before admission (CM.13). |
| `preview` | `none`, `required` for a word preview, or `filter` for a filter result (CM.14). |
| `undo` | `journal`, `workspace` or `none` describes the recorded action class. The shared undo/redo entry dispatches the inverse by its target action (CM.5). |
| `contexts` | Coarse reachability: `any`, `puzzle`, `solve`, `operation`, `keyboard`, `filter`, `macro_base`, `editor`. These do not override C2's input routing or grant permission. |
| `repeat` | True only for the named camera, list-movement and preview-scrub rows in CM.6. |
| `job` | Runs as a cancellable job; the shell stays responsive. |
| `key_0_4` | The 0.4 binding, for reference only. 1.0 keys come from the keyboard model (G6). |
| `disposition` | Sorted distinct dispositions of all contributing decisions; only `keep` and `redesign`, or an empty list for a new row. |
| `decisions` | Generated `{decision, disposition}` contributors from the many-to-many join. |
| `arguments` | Named invocation inputs. Object references are absolute and model-bound. Type names are row-level requirements for EN4's signatures, not a new frozen type schema. Every commit requires a fresh `UserActionToken` under ONE.4. |
| `slot_map` | `preserve`, `rebind`, or `by_target`. Only `grip.rebind` installs a new map. Only shared session undo/redo uses `by_target`: a journal inverse preserves the map; a workspace inverse restores one only if the recorded target changed it. |
| `action_guard` | On undo/redo, `action_hint`: both fields of LI.9/LI.26's `ActionHint` enter the request and token binding. |
| `source`, `note` | Authority for the purpose and any row-specific constraint. Delta amendments also retain `amendment_sources`. |

H15 additions and amendments carry explicit argument schemas (CM.15). The builder defaults only `slot_map` to `preserve` on added rows. No commit token or mathematical input is inferred from an omitted schema.

- **CM.5 [D]** `session.undo` and `session.redo` are the only undo/redo entries. Both are ungated commit rows with `slot_map: by_target`, `action_guard: action_hint` and arguments `hint: ActionHint`, `token: UserActionToken`. LI.9 and LI.26 own the hint and inverse protocol. `undo_dispatch.entry` names those two rows, `classes` is `[journal, workspace]`, and dispatch by the recorded target class is internal. Both hint fields enter the request and token binding: order 42 may target action 41; if action 43 lands first, that hint is refused as `KN5:stale.undo_superseded`. Undo of a recorded rebind restores its recorded map, returns `committed`, preserves puzzle head/hash and advances workspace versions. A journal inverse preserves the map. R-05's original permission path still applies, including confirmation before releasing protection. A running query or program is not a busy gate. Commit/undo cancels jobs that read replaced dependencies; file jobs finish (LI.25, LI.26). A second commit in flight is refused as `KN5:busy.commit_in_flight`, without queueing or replay (Q6(a)). These are table contracts, not measured admission or inverse execution.

The old busy-bearing rows now have lanes. Commit rows use `commit`; `session.resume` uses `exclusive` for recovery. `session.scramble` and `operation.phase.edit` use `none` for atomic workspace edits, with reasons in their notes. No running-job exclusion is restored through a renamed gate. Availability consumes KN5.8 and H5: the hint grants nothing and invoke rechecks it. The overlay, program/view lane and terminal receipt protocol is in the layer interfaces.

- **CM.6 [D]** The repeat allowlist is exactly `camera.rotate`, `camera.zoom`, `list.move`, `view.preview.scrub`. In particular `macro.add`, `operation.preview`, `object.inspect` and all toggles do not repeat. `view.preview.scrub` is the single scrub entry for retained and jumbling solver-word previews. It takes the preview ID, literal `WordPosition` and revision pin, and runs as a cancellable view job. A newer request for its lane target supersedes unfinished scrubs (LI.23); hidden scramble prefixes remain inaccessible (JI.31). `validate_bindings` consumes explicit `routing`, `command` and optional `repeat` fields from a supplied binding table: capture/text entries reach no command; list entries reach only `list.move`, `object.inspect` or `list.row.copy`; global entries use the same row and gates. A binding cannot enable repeat on a non-repeating row. G6 has not supplied actual key letters or a binding table, so the tests exercise synthetic bindings and make no claim about a shipped key map, IME or focus handling.

- **CM.7 [D]** C7 is settled: timer start/stop are workspace rows with no journal entry or undo; a live twist is `twist.apply`, and a draft twist is `operation.phase.append_twist`, with workspace undo and no live protection gate. The complete draft's preview reports protection and execute enforces it (CM.14). `bank.select` becomes `grip.anchor.set`, `grip.capture` becomes `grip.rebind`, and `bank.functions.toggle` has no row. Structure and saved-set commands remain because the named 0.4 units are redesign decisions, including U4-065, U4-066, U4-069/1 to /3, U4-070, U4-072, U4-074, U4-037 and U5-051.

- **CM.8 [B]** The renderer never adopts an uncommitted value as authoritative session state. ONE.62's two named exceptions remain: an engine preview object, identified as a preview; and a derived state, identified by a display plan naming its derivation and base. EV3.4 additionally keeps the plan's base and the value's lineage distinct, and retains the derived digest, level and reachability reference. Neither display assigns a new session revision or grants commit permission. For example, a plan based on revision 7 displaying a derived value leaves the authoritative head and revision at 7. Its appearance remains for the design line and p6; this clause does not choose a layout.

- **CM.13 [D]** Every `view` or `latest_wins` command lane has a nonempty `lane_target` with exactly one key. `arg` names a declared `ViewId` argument; `view` names a fixed ViewId; `control` names a session control target. LI.22 and LI.23 own resolution, request binding and supersession. Rows with a `view` argument use it. Other built-in view rows use their own command ID as the stable session control target, including phase selection, scrub and toggles; the release check uses `jumble.release.check`. No layout or physical control is selected. Camera rotate and zoom for session S and view V resolve the same target and supersede unfinished work; a request for view W has a different target and can run independently. Missing targets, multiple keys, empty values and arguments that do not name a ViewId fail the build.

- **CM.14 [D]** `operation.preview` has no protection admission gate. It can complete with a protection-conflict verdict; `operation.execute` retains its protection gate and refuses that commit. An inadmissible word prefix still creates no preview (ONE.33, KN5.9). `preview: required` holds exactly when `fresh_preview` is a gate. `preview: filter` holds exactly when `fresh_filter_preview` is a gate, and both belong only to `filter.apply`. Filter freshness uses the submitted `FilterPreviewRef` and KN0.9's predicate through LI.41; it requires no pending word preview or presentation. At unchanged filter dependencies it is current even with no word preview; a changed dependency yields `KN5:stale.filter_preview`. The builder refuses mismatched gate/preview forms or a filter gate on another row. These checks verify metadata, not engine freshness evaluation.

- **CM.15 [D]** Every delta addition and amendment supplies a full, nonempty `arguments` object of named type descriptors. An omitted or empty schema fails; `apply_delta` cannot turn it into an empty schema or token-only row. Commit schemas explicitly include `UserActionToken`. JI.32 and JI.33 own the jumbling inputs: immutable snapshots, named caps, pinned letters, correction IDs and preview positions. `snapshot: StateRef` consumes JI.4; `PinnedMenuLetter` refers to JI.3/JI.35's identity/order/contract-pinned exact letter, not a key or unpinned index. Correction references carry JI.13's catalogue pin and explicit construction ID. The `macro` input of `macro.add` accepts one pinned macro, word or correction reference. The `draft` input of check/preview accepts one resolved draft or pinned word. `operation.check` also names its snapshot, requested fields, chunk limit and optional unit/cap; JI.7, JI.11 and JI.14 define those inputs. Budgets and grants remain in LI.5's common envelope. Two `jumble.straddlers` bindings at snapshot S7, chunk limit 32 and caps 0 and 1 have different argument digests and request bindings under LI.5/LI.19. The builder keeps the required argument names independently of the generating delta. For every addition and amendment, the tests remove each required argument while keeping its schema nonempty; each mutant must be refused, including `jumble.on` reduced to only `token`. These are row-level type requirements, not a new byte format or a runtime validator.

- **CM.16 [D]** Top-level `services` contains nine registrations with exactly `id`, `kind`, `caller`, `lane`, `clauses` and `note`. `kind` is `service` or `continuation`; `caller` is `runtime`, `host` or `both`; `lane` uses the table vocabulary. A service has no key, alias, availability or undo entry, grants nothing and does not enter C1's command count. IDs are unique and cannot collide with commands. The registrations are `kernel.snapshot.open`, `kernel.snapshot.close`, `kernel.stream.next`, `kernel.value.retain`, `kernel.value.release`, `kernel.outcome`, `kernel.job_result`, `kernel.undo_hint` and `language.publish_view`. The stream continuation keeps its admitted invocation and lane without new admission. Scheduled view publication finalizes its original invocation; standalone publication uses the resolved ViewId under LI.25. The annex's `requires` resolves these as services or continuations and `jumble.observe` as a command (KI.1, KI.16); naming a missing ID in `requires` fails its check.

  `kernel.undo_hint` takes the session/epoch and `direction: undo | redo`, returning one `ActionHint` or `none`
  for that direction as LI.9 defines. The annex's signature and walks consume that service; no extra command is added.

- **CM.17 [D]** H-08's workspace rows at the LP3 freeze (owner, 20 November 2026:
  [LP3 exit decisions](../../wiki/decisions/owner-decisions-2026-11-20-lp3-exit.md)).
  - `workspace.set {record: Text, payload: Json, provenance: ProvenanceBlock}` is M6's record write, a language
    addition: lane `none`, no gate or preview, any context, undo `workspace`. It writes the named record's payload and
    provenance block as one durable action; the workspace revision and the record's version advance, and the state,
    the preview slot and the jobs stay (KN5.18, KN5.23). Its undo and redo are workspace inverses (LI.26). The host's
    provenance reader checks the block at the write (PS1.16), and a program may not invoke the row
    (`LG3:capability.not_granted`). The row has the form `c600-command`'s draft row has, so the command layer keeps
    it unchanged. OPEN: `payload: Json` is not frozen; the record payload's typed forms (a work sheet, a saved set, a
    layer predicate, a filter reference) are named where they are built, and a typed payload is a later change under
    PHS.52.
  - Undo (LB7.2: a workspace-changing operation forms an undoable action). The rows that save or write a named object
    the workspace holds (LB7.1) have undo `workspace`, as their siblings `keymap.save`, `filter.edit`, `filter.preset`
    and `structure.filter.compose` have: `worksheet.save` (CT-LB7-01), `macro.save`, `set.save`, `filter.save` and
    `structure.layer`. Eight workspace rows keep undo `none`, each for a stated reason: `session.timer.start` and
    `session.timer.stop` (CM.7, EV4.21); `recording.start`, `recording.stop` and `recording.play`, whose capture and
    playback state is outside the undo order while each played step keeps its own row's class (EV4.20 to EV4.22);
    `operation.preview.cancel`, since a preview slot is no workspace object and a reuse needs a fresh preview
    (LB7.2); and `grip.select` and `input.destination.set`, input routing that the shell owns (section 1) and the
    workspace does not hold, which give a receipt and no undo target, as a view row does (EV4.21).
  - `set.save` takes the optional `replace: Bool?` beside `members`, `kind` and `name`, an amendment in the
    language delta (U1-057: replacing a named set stays explicit; LP2's pull request 337). A save under a name
    that holds a saved set is refused with `KN5:input.saved_set_exists`, and the saved set is kept, unless the
    invocation carries `replace: true` (chapter 13 section 9).

- **CM.18 [D]** Top-level `reads` lists the rows a program reads through a host bridge at the LP3 freeze (KN2.3,
  KN2.4, LG2.8; owner item 7 of the [LP3 exit decisions](../../wiki/decisions/owner-decisions-2026-11-20-lp3-exit.md)).
  A program writes `read_snapshot<T>("<id>")` or `read_snapshot<T>("<id>", argument)`. A read is not a command or a
  service: it has no key, alias, lane, availability or undo entry, grants nothing and does not enter C1's count.
  - Each row has exactly `id`, `puzzle`, `level`, `argument`, `answer`, `fields`, `tariff`, `codes`, `clauses` and
    `note`. `puzzle` is `Full`, `jumbling` or `both`, and `level` follows KN2.4. Every Full read is at `labels`
    (`effect` reads no state; chapter 03 types the orbit catalogue as Full labels). The jumbling reads `nu`,
    `correction_set_check`, `correction_lookup`, `sticker_test` and `label_checkpoint` are at `labels`, the other
    fourteen at `poses` (JI.6 to JI.13). `journal` answers in both modes at level `none` (KN4.3).
  - `replay` and `protection` keep one name each, with one entry per puzzle and level: Full at `labels`, jumbling at
    `poses`. No other name repeats, and a name for both puzzles has no single-puzzle entry. The reads
    `structure.compare`, `filter.preview` and `structure.layer` share a name with their command rows and stay
    separate entries; a read ID cannot collide with a service.
  - `answer` is the type a program declares. `fields: all` means `T` has exactly the answer's fields, in any order:
    `journal`, every Full read except `effect`, and the three jumbling end tests. `fields: subset` means `T` declares a
    nonempty subset of the served fields: `effect` and the other jumbling reads, whose `answer` is the type LP3's
    library (C8) reads them with from a wider served record. On the Full bridge a `Nat` field may be declared `Int`,
    and no field carries a display qualifier. `types` names the shared argument form `Recipe`.
  - Order: the host prices the read first and refuses a malformed argument there, and on the Full bridge a declared
    type it does not serve, at the cost of the read's step only. The run then charges the tariff and the host
    answers; a refusal after the charge keeps it. A jumbling tariff counts passes of 177,120 units, one per piece.
  - `codes` lists once the refusals every read shares (`every`) and those of each puzzle's bridge: a retained read
    with Jumble on, a jumbling read with Jumble off, and the jumbling bridge's identity and state-hash checks. A row
    lists only its own codes, and a kernel refusal a read passes through is named in its note.
    `LG3:capability.not_granted` refuses a read the run's bridge does not serve; `setups` is served only with a
    certified table (BND.1). Every code is registered in chapter 13 section 5.
  - The rows are table contracts for the bridges' draft rows at the freeze, not a bridge, a measured cost or a
    profile grant. OPEN: where a jumbling bridge serves a field under another name than JI.6 to JI.13 give (for
    `straddlers`, `certificates` and `status`), the row keeps the served name, and a rename is a later change under
    PHS.52.

## 3. What the table shows now

| Kind | Commands | Examples |
|---|---|---|
| workspace | 67 | phases, buffers, intents, blocks, protection, macros, keymaps, timer, recordings, record writes |
| view | 45 | panels, cycle modes, inspection, display controls, camera, further views |
| query | 38 | check, preview, buffer analyser, recommendations, structure, filter counts |
| commit | 14 | execute, live twist, shared undo and redo, mode crossings, menu twist, new solve, reset, log import, checkpoint save/restore, branch selection, practice copy |
| file, app | 11 | exports, backup, resume, stop a job, exit, clipboard |

These counts come from the stored final table and exclude the nine services and the 30 bridge reads. The row list below is checked against that same table. A commit row identifies an admission and transaction path; it does not imply that every such row changes puzzle labels. All labels and collateral remain present regardless of view filters.

Eleven top-down inventory rows remain in `not_commands`. That historical index does not replace the per-decision coverage check. The retained units behind old browser controls are mapped by purpose even where the separate browser infrastructure is deleted.

| Command ID | Kind | Lane |
| --- | --- | --- |
| `session.new` | commit | commit |
| `session.resume` | app | exclusive |
| `session.timer.start` | workspace | none |
| `session.timer.stop` | workspace | none |
| `session.scramble` | workspace | none |
| `session.reset` | commit | commit |
| `session.undo` | commit | commit |
| `session.redo` | commit | commit |
| `session.checkpoint.save` | commit | commit |
| `session.checkpoint.restore` | commit | commit |
| `session.report` | query | none |
| `file.log.export` | file | exclusive |
| `file.log.check` | query | exclusive |
| `file.log.import` | commit | commit |
| `file.keymap.export` | file | none |
| `file.keymap.check` | query | none |
| `file.keymap.apply` | workspace | none |
| `solve.open` | view | view |
| `operation.phase.select` | view | view |
| `operation.phase.edit` | workspace | none |
| `operation.cleanup.from_prepare` | workspace | none |
| `operation.check` | query | exclusive |
| `operation.preview` | query | exclusive |
| `operation.execute` | commit | commit |
| `operation.preview.cancel` | workspace | none |
| `job.stop` | app | none |
| `operation.new` | workspace | none |
| `operation.reuse` | workspace | none |
| `view.forecast.select` | view | view |
| `view.cycle.mode` | view | view |
| `view.effect.scope` | view | view |
| `view.local` | view | view |
| `view.global` | view | view |
| `target.next.activate` | workspace | none |
| `target.next.set` | workspace | none |
| `object.inspect` | view | view |
| `object.actions` | view | view |
| `object.inspect.clear` | view | view |
| `view.entry_slots` | view | view |
| `buffer.assign` | workspace | none |
| `buffer.analyse` | query | exclusive |
| `block.add_current` | workspace | none |
| `block.add_home` | workspace | none |
| `intent.set` | workspace | none |
| `view.residuals` | query | none |
| `view.findings` | view | view |
| `orbit.switch` | workspace | none |
| `view.recommendations` | query | exclusive |
| `view.names` | view | view |
| `macro.base.open` | view | view |
| `macro.select` | view | view |
| `macro.add` | workspace | none |
| `worksheet.save` | workspace | none |
| `worksheet.load` | workspace | none |
| `macro.compare` | query | exclusive |
| `endgame.configure` | workspace | none |
| `macro.save` | workspace | none |
| `macro.variant.create` | workspace | none |
| `protection.policy.set` | workspace | none |
| `protection.position.protect` | workspace | none |
| `protection.position.release` | workspace | none |
| `protection.orbit.unlock` | workspace | none |
| `keyboard.toggle` | view | view |
| `grip.select` | workspace | none |
| `twist.apply` | commit | commit |
| `twist.inverse.toggle` | view | view |
| `input.destination.set` | workspace | none |
| `grip.anchor.set` | workspace | none |
| `grip.rebind` | workspace | none |
| `keymap.edit` | workspace | none |
| `command.find` | view | view |
| `camera.rotate` | view | view |
| `filter.open` | view | view |
| `filter.preview` | query | none |
| `filter.apply` | workspace | none |
| `display.frame.toggle` | view | view |
| `display.adaptive.toggle` | view | view |
| `display.adjust` | view | view |
| `view.reset` | view | view |
| `structure.find` | query | none |
| `structure.compare` | query | none |
| `structure.layer` | workspace | none |
| `view.cell_views` | view | view |
| `camera.center` | view | view |
| `set.save` | workspace | none |
| `operation.phase.append_twist` | workspace | none |
| `workspace.reset` | workspace | none |
| `session.branch.select` | commit | commit |
| `session.history` | query | none |
| `session.completion` | query | none |
| `session.practice` | commit | commit |
| `file.session.backup` | file | exclusive |
| `file.proof.export` | file | exclusive |
| `file.certificate.export` | file | exclusive |
| `file.macro.export` | file | exclusive |
| `file.macro.check` | query | exclusive |
| `file.macro.import` | workspace | none |
| `app.exit` | app | none |
| `app.reconnect` | app | exclusive |
| `help.open` | view | view |
| `work.surface.show` | view | view |
| `work.surface.close` | view | view |
| `work.layout.set` | view | view |
| `work.layout.reset` | view | view |
| `work.fullscreen.toggle` | view | view |
| `transient.close` | view | view |
| `list.move` | view | view |
| `list.row.copy` | workspace | none |
| `camera.zoom` | view | view |
| `view.preview.scrub` | view | view |
| `view.local.configure` | view | view |
| `display.references.set` | view | view |
| `display.mode.set` | view | view |
| `object.copy` | app | none |
| `object.locate` | view | view |
| `target.current.set` | workspace | none |
| `target.next.clear` | workspace | none |
| `target.frame.set` | workspace | none |
| `block.reference.set` | workspace | none |
| `block.remove` | workspace | none |
| `protection.orbit.set` | workspace | none |
| `grip.behavior.set` | workspace | none |
| `grip.frames` | query | none |
| `grip.rotations` | query | none |
| `grip.members` | query | none |
| `keymap.defaults` | workspace | none |
| `keymap.save` | workspace | none |
| `keymap.name` | workspace | none |
| `keyboard.disclosure.set` | workspace | none |
| `filter.edit` | workspace | none |
| `filter.preset` | workspace | none |
| `filter.save` | workspace | none |
| `filter.load` | workspace | none |
| `structure.select` | workspace | none |
| `structure.adjacency` | query | none |
| `structure.vertex` | query | none |
| `structure.filter.compose` | workspace | none |
| `macro.replace` | workspace | none |
| `macro.metadata.set` | workspace | none |
| `macro.filter` | query | none |
| `macro.library.check` | query | exclusive |
| `macro.compose` | workspace | none |
| `macro.operand.set` | workspace | none |
| `macro.reference.check` | query | exclusive |
| `endgame.choices` | query | none |
| `endgame.check` | query | exclusive |
| `reference.set` | workspace | none |
| `worksheet.compare` | query | exclusive |
| `view.cycle.select` | view | view |
| `view.cycle.page` | view | view |
| `view.reason` | view | view |
| `view.journal.delta` | query | none |
| `jumble.on` | commit | commit |
| `jumble.off` | commit | commit |
| `jumble.twist.menu` | commit | commit |
| `jumble.twist.append` | workspace | none |
| `jumble.survey` | query | exclusive |
| `jumble.observe` | query | exclusive |
| `jumble.straddlers` | query | exclusive |
| `jumble.certificate.inspect` | query | none |
| `jumble.release.check` | query | latest_wins |
| `jumble.covers` | query | exclusive |
| `jumble.repair.list` | query | exclusive |
| `jumble.nu` | query | exclusive |
| `jumble.corrections.check` | query | exclusive |
| `jumble.correction.lookup` | query | none |
| `jumble.checkpoint.check` | query | exclusive |
| `jumble.scramble` | workspace | exclusive |
| `view.open` | view | view |
| `view.close` | view | view |
| `recording.start` | workspace | none |
| `recording.stop` | workspace | none |
| `recording.play` | workspace | program |
| `recording.convert` | workspace | none |
| `workspace.set` | workspace | none |

The bridge reads (CM.18), in the table's order:

| Read | Puzzle | Level | Fields | Clauses |
| --- | --- | --- | --- | --- |
| `positions` | Full | labels | all | KN2.4 |
| `labels` | Full | labels | all | KN2.4 |
| `effect` | Full | labels | subset | KN2.4 |
| `protection` | Full | labels | all | KN2.4, LB2.1 |
| `replay` | Full | labels | all | KN2.4, KN2.7 |
| `setups` | Full | labels | all | BND.1, LB2.1 |
| `structure.compare` | Full | labels | all | KN2.3, LG2.8 |
| `filter.preview` | Full | labels | all | KN2.3, LG2.8 |
| `structure.layer` | Full | labels | all | KN2.3, LG2.8 |
| `orbits` | Full | labels | all | KN2.3, LB2.1 |
| `journal` | both | none | all | KN4.3 |
| `heights` | jumbling | poses | subset | KN2.4, JI.6 |
| `height_counts` | jumbling | poses | subset | KN2.4, JI.6 |
| `replay` | jumbling | poses | subset | KN2.4, JI.10 |
| `word_check` | jumbling | poses | subset | KN2.4, JI.11 |
| `protection` | jumbling | poses | subset | KN2.4, KN1.11 |
| `nu` | jumbling | labels | subset | KN2.4, JI.13 |
| `correction_set_check` | jumbling | labels | subset | KN2.4, JI.13 |
| `seam_facts` | jumbling | poses | subset | JI.7, JI.8 |
| `rim_summary` | jumbling | poses | subset | KN2.4, JI.6 |
| `covers` | jumbling | poses | subset | KN2.4, JI.12 |
| `correction_lookup` | jumbling | labels | subset | JI.13 |
| `classify` | jumbling | poses | subset | KN2.4, JI.6 |
| `survey` | jumbling | poses | subset | KN2.4, LB3.5 |
| `straddlers` | jumbling | poses | subset | KN2.4, JI.6 |
| `units` | jumbling | poses | subset | JI.9 |
| `repair_list` | jumbling | poses | subset | KN2.4, JI.12, BND.2 |
| `sticker_test` | jumbling | labels | all | KN2.4, LB3.2 |
| `retained_checkpoint` | jumbling | poses | all | KN2.4, LB3.2 |
| `label_checkpoint` | jumbling | labels | all | KN2.4, LB3.2 |

## 4. Use by the design track

- **Greybox layouts (G1):** every control in a greybox names a command ID. A layout passes the reachability check when every kept command is reachable in it.
- **Keyboard model (G6):** keys bind to command IDs and contexts. A key in `editor` context never reaches a puzzle command. Repeat is allowed only where `repeat` is true.
- **Flow scripts:** a core flow is a sequence of command IDs. The script measures steps, pointer travel and time on each layout.
- **Pointing needs (H-05)** return to the engineering track as the commands that need a pick target (`object.inspect`, `grip.select`, `twist.apply`, `camera.rotate`, `camera.center`).

## 5. Open points

The old points 1 to 4 are settled in this draft: code-only purposes have explicit join records; timers have no journal/undo; structure stays; live and draft twists have separate rows (CM.2, CM.7). Point 5 belongs to p2b's availability contract and its independent plan check. H3 to H5 are consumed as the integrator's working forms, not new owner decisions.

- **CM.9 [B]** `twist.apply` keeps protection and has no preview. Charter S4's preview applies to a draft step, a macro and any word (ONE.78; owner, 6 November 2026, decision 10).
- **CM.10 [D]** The 22 October answers remain in force for now: Q1(a)'s physical grip block, fixed chords and prefixes; Q2(b)'s position bindings except office-character undo/redo; Q3(a)'s transported slots and frames; Q4(a)'s shared action order; Q5(a)'s revision-job cancellation; Q6(a)'s busy refusal; Q7(a)'s four seats pending XM1; Q8(a)'s first in-place refusal message; Q9(a)'s learning-step disclosure; and Q10(a)'s additional H-03 motion rows. Other options remain recorded in DT-M1. This packet selects no key letters, motion timing or UI design. If XM1 reopens Q7, the owner decides and p1's input delta changes; resolved absolute rotations and the retained join still apply.
- **CM.11 [D]** XM2 to XM5 have only the table-level evidence below. They do not claim a running engine, a key router, a journal replay or measured latency. Workspace undo of a rebind restores its recorded map under Q4(a); it cannot compute a new map from camera or focus. LI.26 supplies the workspace action protocol.

| Experiment | Headless table check now | Still required; owner of the work |
| --- | --- | --- |
| XM2 | Camera, filter, twist and focus-command rows preserve `slot_map`; only rebind installs a new map; shared session undo/redo can restore the target workspace action's recorded one. An unauthorized map-writing row is refused. | Digest trace through real camera/filter/twist/focus events and undo of rebind; EN4 router and session implementation. |
| XM3 | Undo and redo carry `ActionHint`, the `action_hint` guard and `KN5:stale.undo_superseded`; removing the guard or either hint binding fails. | Land action 43 after hint {order:42,target:41}, invoke that hint, observe the stale error and unchanged state/workspace; p2b and the session implementation. |
| XM4 | Live commit and both shared undo/redo entries use `commit`, preview uses `exclusive`, undo has no gates, and held mutation keys cannot repeat. | B-004 atomic admission with a running query, concurrent pointer/key submissions and job cancellation (file jobs finishing); candidate engine. Real burst gaps and commit latency require the owner's machine. No thresholds or timings measured here. |
| XM5 | Live/draft rows take the same absolute cap and exact rotation types; aliases retain source spellings and require resolution of relative parameters. | All input routes over all 59 menu items per grip, and equal journal hash under two key maps; p1's exact menu delta, G6 routing and the engine's replay fixture. |

- **CM.12 [D]** The final table includes the real jumbling and language deltas under H15. Integration still reconciles the other packets' layer signatures, overlay/availability protocol, migration vectors and derived-display trace. The integrator refreshes the format packet's alias pin to `en4-2026-11-05.3` and its current digest under CM.3's profile. The two base rows without 0.4 contributors (`list.row.copy`, `view.preview.scrub`) all 16 jumbling additions and all seven language additions cite their explicit newer sources. Independent semantic review of the handwritten join and aliases remains required. G6's real binding table must be passed to `validate_bindings`; K6's prefix, duplicate and layout-conflict checks and Windows IME/focus-release tests require that implementation. These are open verification inputs, not new design-table topics.

After integration the integrator runs the following. The final-table test always reads the real deltas and requires `final: true`; it never overwrites the table. The example fixture is never used to build the stored final table.

```text
python -B docs/progress/1.0/build_command_table.py
python -B docs/progress/1.0/test_command_table.py
python -B docs/architecture/1.0/language/closure/check_closure.py --packet p2a
```

The atlas reads the retained fields. Its banner states this draft, and it does not display the join or lane fields. After a table change run `python -B docs/progress/1.0/atlas/build.py`.

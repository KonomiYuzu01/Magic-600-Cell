# Chapter 04: the environment

Status: **draft for the owner. Not accepted.** Conventions are in [the overview](00-overview.md), section 3.

The environment connects source, checked programs, results and the solver's actions. This chapter defines services and
data for an editor host and the shell. The editor's appearance waits for the design line. Chapter 02 owns the language
core, chapter 03 the libraries and principle records, chapter 05 the stored forms, and chapter 10 the display objects.
The interfaces below consume those contracts. They prescribe no concrete syntax.

Sources: [the partition](../ecosystem-partition.md), especially sections 6, 8.3, 9 and 11, INV-12 and defaults 1 to 20;
[the kernel](01-kernel.md); [DT-L1](../../../../progress/1.0/design-table/topics/language-ecosystem.md), especially E2,
E6, M5, M6, R2-E1 to R2-E4 and its Outcome;
[DT-M1](../../../../progress/1.0/design-table/topics/command-keyboard-model.md) and its owner answers;
[DT-J1](../../../../progress/1.0/design-table/topics/jumble-solver-aids.md) and its word-storage rules;
[H-08](../../../../progress/1.0/command-table.md);
[the charter](../../../../progress/1.0/charter-2.0-draft.md), bars E5, E6, M1 to M6, D1 to D8 and S1 to S6;
[the owner design decisions of 2 October](../../../../wiki/decisions/owner-decisions-2026-10-02-design.md);
[the owner decisions of 3 November](../../../../wiki/decisions/owner-decisions-2026-11-03.md);
[requirements R-01 to R-19](../../../../progress/1.0/requirements-from-screening.md);
[the glossary](../../../../progress/1.0/glossary/glossary.md),
[its terms](../../../../progress/1.0/glossary/terms.json) and
[its build rules](../../../../progress/1.0/glossary/build.py);
[the solving workflow](../../../../progress/1.0/solving-workflow.md); and
[V06](../../10_V1_ARCHITECTURE.md), for the error categories reused by partition default 5.

## 1. EV1: editor services

- **EV1.1 [D]** Each editor request names immutable source bytes by digest, their edition, and the manifest and
  lockfile context of chapter 02. Its response names the same input. A host never attaches an older response to a newer
  draft. This follows from partition LG1, LG5 and default 13: a file name alone cannot identify the program checked.
  A source range identifies its source digest and its start and end positions; the coordinate convention is EV1.10.

- **EV1.2 [D]** Parsing returns a recovered structure and diagnostics even when part of the draft is incomplete
  (partition EV1, LG1). Recovery identifies gaps and invalid spans. It gives completion and navigation a structure to
  read; it supplies no executable meaning for a gap. A recovered draft never passes the execution check merely because
  its surrounding structure parsed.

  ```text
  operation parse(source, edition) -> { recovered_structure, diagnostics, source_digest }
    effect:  pure
    errors:  LG1:syntax.recovery_gap
  operation complete(source, position, module_context) -> { candidates, source_digest }
    effect:  pure
    errors:  EV1:stale.editor_source, KN1:input.identity_mismatch
  operation diagnose(source, module_context) -> { diagnostics, static_verdict, source_digest }
    effect:  pure
    errors:  LG1:syntax.recovery_gap, LG2:type.mismatch, LG2:type.exact_required,
             LG3:effect.undeclared, EV1:evidence.obligation_unmet, KN1:input.identity_mismatch
  ```

- **EV1.3 [D]** Completion uses the recovered scope, the chapter 02 type and effect information, and the selected
  module context. Each candidate carries the source range it would replace and the relevant type, effect, identity
  and documentation reference (partition EV1, LG2, LG3). Suggesting an entry neither grants its capabilities nor makes
  its command available. A draft's missing context remains explicit instead of borrowing the active tool's context.

- **EV1.4 [D]** Diagnostics distinguish ordinary type errors, effect errors, exactness violations, unmet evidence
  obligations and model or menu identity mismatches (partition EV1, default 5). Each diagnostic has a stable typed
  code, a source range and the relevant typed context. `LG2:type.exact_required` identifies an approximate value at an
  exact decision site; `EV1:evidence.obligation_unmet` identifies a required claim or check that is absent. An open claim
  that the program is allowed to inspect is a value, not an error (INV-5, KN0.5). Chapter 02 defines the checks, and
  chapter 06 defines the evidence they inspect.

- **EV1.5 [D]** Hover returns the checked type, effects and required capabilities, exactness, model and level, docs,
  and evidence references for the selected expression or declaration (partition EV1). It preserves the distinction
  between a static check, a result's evidence and permission to invoke a command. For an unresolved expression, it
  returns the known fields and their unresolved dependencies; it never invents a successful check.

  ```text
  operation hover(source, position, module_context) -> { information, references, source_digest }
    effect:  pure
    errors:  EV1:stale.editor_source
  operation definition(source, position, module_context) -> List<SourceRange>
    effect:  pure
    errors:  EV1:stale.editor_source
  operation rename(source, declaration, proposed_name, module_context) -> { edits, diagnostics, source_digest }
    effect:  pure
    errors:  EV1:stale.editor_source, LG2:type.mismatch
  ```

- **EV1.6 [D]** Go to definition resolves a declaration in the digest-pinned dependency graph. Rename proposes edits
  to the resolved declaration and its references, with the source digest for each edit. It does not replace equal
  text in unrelated scopes or rewrite a dependency under its old digest (partition LG5, default 13). Applying edits
  to changed bytes is refused as `EV1:stale.editor_source`. A resulting draft is checked again before it can run.

- **EV1.7 [D]** A running tool is pinned to a checked module digest and its locked dependencies. Editing makes another
  draft; it does not replace the bytes being executed. While the draft is broken, the last valid version keeps running
  and its results and source links continue to name that version (partition EV1, LG5). An unchecked draft never runs,
  including through a view, a binding, a recording or a debugger. This guarantee does not override cancellation or
  availability when the tool's input revision or capabilities change (KN5, KN7).

- **EV1.8 [D]** A valid static check is a prerequisite for activating a new module digest, not an execution grant.
  Activation and each invocation still use the chapter 09 profile grants and KN5.7 availability. Editing, saving or
  renaming does not inherit another digest's evidence or permissions (defaults 17 and 18; DT-L1 R2-E4). The promotion
  event remains EV1.11.

- **EV1.9 [D]** The shell may link the checker as a library to provide the data services of EV1: parse, complete,
  diagnose, hover, definition and rename (partition section 9; DT-L1 E1). Such a service reads source and checker data;
  it runs no user program or user callback. Program execution remains outside the UI thread and off the kernel-key
  dispatch path. Whether a linked service meets the latency bars is LX-4 in chapter 11. No latency has been measured
  here, and this chapter supplies no substitute timing target.

- **EV1.10 [O]** LP1's editor protocol and chapter 10's source linkage must choose one source-position convention and
  its conversion at host boundaries. Options include UTF-8 byte offsets, Unicode scalar positions and host code-unit
  positions. The choice fixes range endpoints, insertion ranges and conversion around multi-unit characters. Under
  every option a range is bound to exact source bytes, and a host never guesses its coordinate unit.

- **EV1.11 [O]** No source chooses whether a newly checked draft replaces the active version automatically or through
  an explicit activation command. The integrator's LP1 service contract must settle the switch, including a run already
  using the old digest, subject to LQ9's placement and the later design-line interaction. Both options preserve EV1.7:
  each run has one pinned version, and failure to check leaves the previous valid version available.

- **EV1.12 [B]** A diagnostic or refused editor action provides what happened, its reason and an available next action
  as typed data for the host (charter E6). Its code decides handling; prose is never parsed to decide a continuation
  (H-08 typed errors; partition default 5). Final messages and their presentation wait for the design line.

## 2. EV2: debugger, replay and profiler

- **EV2.1 [D]** A debugging session names checked source and module digests, the chapter 02 IR version and the pinned
  input it inspects (partition EV2, INV-10). Stepping advances a program's evaluation or a recorded trace. It never
  advances the authoritative session, commits a preview, executes an undo or grants a capability. A debugger exposes no
  write path to the engine or store (INV-2, INV-3). An inspected proposal is data; stepping over it cannot obtain the
  shell's user-action token.

  ```text
  operation debug_step(run, ir_position) -> { ir_position, values, source_ranges, outcome }
    effect:  read a snapshot
    cost:    interpreter steps and declared kernel costs, under KN7
    errors:  EV2:input.replay_dependency_missing, EV2:capability.debug_write_forbidden, KN2:stale.revision_gone
  operation inspect_value(run, value_reference) -> { value, type, identity, level, origin }
    effect:  read a snapshot
    errors:  EV2:input.replay_dependency_missing
  ```

- **EV2.2 [D]** Value inspection preserves the chapter 02 type and KN1 identity and level. It distinguishes exact
  values, approximate display values with their precision, KN0.4 partial results and KN0.5 verdicts. A result on a
  `Derived` value retains its derivation and reachability claim (KN2.11 to KN2.14; DT-L1 R2-E2, R2-E4). A rendered
  coincidence, a truncated inspection or a missing certificate cannot become equality or a proved claim
  (INV-4, INV-5, INV-6).

- **EV2.3 [D]** Deterministic replay preflights the `ReproKey` of chapter 09 from the `RunRecord` of chapter 05. It
  resolves every required source and module digest, edition and IR version, identity block, input digest and level,
  parameters, seeds, cost units and deterministic budgets before evaluation (INV-7, INV-10; partition 8.4; DT-L1 R2-E2).
  A missing module returns `EV2:input.replay_dependency_missing`, naming the missing digest. A different model, menu
  identity, menu order or contract revision returns `KN1:input.identity_mismatch`, with expected and available identities.
  A similarly named local file is never a replacement. The record remains inspectable as recorded evidence without a
  successful replay claim.

  ```text
  operation replay_run(record: RunRecord) -> { outcome, comparison, profile, evidence_references }
    effect:  read a snapshot
    cost:    the deterministic budget and declared costs of the recorded run
    errors:  EV2:input.replay_dependency_missing, KN1:input.identity_mismatch, EV2:evidence.replay_mismatch,
             EV2:capability.debug_write_forbidden
  ```

- **EV2.4 [D]** A replay matches the record's exact result bytes, level-qualified digests, outcome, coverage and
  deterministic budget usage under chapter 09's comparison rules. A difference is
  `EV2:evidence.replay_mismatch`, with the differing result or trace position where available. Display floats are
  compared at their declared precision, as partition 8.4 requires; elapsed time and new session receipts are not exact
  results to reproduce. A wall-clock safety stop remains incomplete and not reproducible (KN7.10). Replay cannot
  upgrade that record into a completed negative, or label a missing dependency as a refutation (INV-5, default 6).

- **EV2.5 [D]** The result's provenance view follows evidence and computation references to their inputs, library
  entries, kernel primitives and cited lemmas (partition EV2, INV-12). Expansion is on demand. A long step is cited by
  its literal-word digest and positions, with its identity block and cost alphabet, so a reader can request a part
  without expanding the whole word (DT-L1 R2-E4). A citation never substitutes the equality key for the executed
  letters (KN2.7). Chapter 06 owns the graph's evidence meaning; its presentation adds no evidence edge.

- **EV2.6 [D]** Checker stepping consumes KN6.4's per-claim trace. Each step identifies the certificate input digest,
  checker source digest, claim, checked inputs and outcome, and links to the word positions when relevant. The trace
  is the checker's output, stored or re-derived; the debugger imports no producer search or construction code
  (KN6.2, KN6.4; DT-L1 R2-E4). A missing trace is unavailable, not a passed check. Running a checker to re-derive it
  remains budgeted work and cannot modify the puzzle.

- **EV2.7 [D]** A cost profile reports KN7's granted, used and remaining interpreter steps, declared kernel cost,
  logical memory and logical output units, with the named budget and stop reason. Word costs name their alphabet:
  retained generators, menu letters or UI actions (partition LG2). Kernel costs use KN7.8's charges even on cache
  hits; recomputation after eviction adds no extra charge. Elapsed seconds may accompany the profile as display data.
  They neither alter the budget nor establish product performance evidence (KN7.7 to KN7.11, overview 3.5).

- **EV2.8 [B]** Journal travel reads `journal`, `revision_info`, `checkpoints` and, when needed, `workspace_records`
  and `receipt` through KN4.3 query rows (INV-2). Opening an old snapshot changes only what the debugger inspects. It
  does not select an active journal line, reset the session, apply an inverse word or write session files. A revision
  that is unavailable reports `KN2:stale.revision_gone`; the debugger never silently opens the current revision instead
  (KN0.6). Chapter 08 defines snapshot and branch lifecycles.

- **EV2.9 [O]** Replay of a run with workspace, file or preview effects needs a chapter 09 replay contract agreed with
  chapter 05. Options are inspection of recorded effects without re-executing them, or evaluation against isolated
  replay sinks whose inputs are captured and digest-pinned. No source settles those sinks or which external inputs a
  record must capture. Both options keep the debugger read-only, grant no ambient files or clock, and distinguish
  trace inspection from a complete deterministic re-execution. Unsupported effects report
  `EV2:capability.debug_write_forbidden` instead of touching the live workspace.

- **EV2.10 [P LQ4]** A research run pinned by digest may finish while the solver commits or undoes. Its debugger and
  result keep the old input; reaching the new revision requires a re-run or exact transport justified by a lemma
  (KN7.5; DT-L1 R2-E1). Under the cancellation alternative the run ends as cancelled. Under the no-solving-branches
  alternative the journal viewer opens research sessions separately. No option makes an old result current merely
  because the debugger has moved its cursor.

- **EV2.11 [O]** Chapter 05's word references and KN6.4's trace contract must settle whether word positions use
  zero-based half-open intervals or one-based inclusive intervals, and how a compressed part cites its expansion.
  Each reference must make its alphabet and position convention unambiguous. Until that contract is fixed, EV2.5 and
  EV2.6 specify the required linkage without prescribing stored endpoints.

- **EV2.12 [O]** The executable IR stop points and their mapping back to source depend on chapter 02 and LQ9. RT1 can
  expose its evaluator steps; RT3 needs a mapping through its target VM; RT2 or RT4 needs an equivalent host mapping
  for its admitted subset. LP1 must define step granularity, suspended-call inspection and missing-map results for the
  chosen route. No route may turn a missing mapping into an invented source location or an unchecked execution.

## 3. EV3: source and UI linkage

- **EV3.1 [D]** The linkage chain is source digest and range, checked program, `ViewNode`, `DisplayPlan`,
  `DisplayInstance`, canonical object ID and source range through `SourceMap` (partition EV3, default 7). Chapter 10
  defines these display types and their multiplicities. The editor uses the source side; the shell uses the view,
  picking and command side. Multiple instances may link to one object or declaration. A screen point is only input to
  picking; it never becomes an object identity or a source reference (INV-6).

- **EV3.2 [D]** Declarative views, panels, controls and overlays publish chapter 10 view-model nodes. Controls name
  command rows and typed arguments, and nodes link to their producing source (partition EV3, KN5.3 to KN5.5). A
  renderer adapter consumes display plans without calling a program. Evaluating a view grants no workspace-write
  effect; a persistent edit requires a workspace command the solver invokes (default 16). Chapter 10 owns `Chart` and
  the chart subset; this chapter defines no renderer feature or shader.

- **EV3.3 [B]** Output carries the base `RevisionRef` it read, and remains identified as stale after that base is
  replaced until recomputation supplies current output (charter D2; partition 8.3). The shell adopts the engine's new
  revision without waiting for a program. Its output state distinguishes empty, loading, error, stale and busy without
  passing stale facts off as current ones. A cancelled query result is not a retained current answer (KN7.3, DT-M1
  Q5 (a)); a stale program output is identified with its old base rather than attached to the new revision.

- **EV3.4 [D]** A derived state appears only through a chapter 10 display plan naming the kernel derivation and its
  base revision (INV-1; partition EV3, 8.3; DT-L1 E2). The base is the snapshot revision in the derivation's ancestry;
  it is not a new session revision assigned to the derived value. The plan retains the derived state's own digest,
  level and reachability reference (DT-L1 R2-E2). Neither the shell nor the renderer can adopt it as the authoritative
  state (KN2.11 to KN2.14). An imported value with no snapshot ancestor raises the open dependency EV3.12.

- **EV3.5 [D]** A pick returns the `DisplayInstance` of the adopted plan. Chapter 10's `SourceMap` resolves it to the
  canonical ID at its model and level and, for a language-produced instance, to the producing source location.
  The shell checks the plan's base revision and the editor checks the source digest before using the result
  (KN0.6, INV-6). A stale pick or obsolete source link returns `EV3:stale.source_link`. Missing source is explicit; the
  host never substitutes a nearby declaration. The chain uses instance IDs without inverting a chart (partition 8.5).

  ```text
  operation follow_pick(instance: DisplayInstance, map: SourceMap, shown_base: RevisionRef)
      -> { canonical_id, level, source_location?, producing_module }
    effect:  pure
    errors:  EV3:stale.source_link, KN1:input.identity_mismatch
  operation edit_declared_part(source, declaration, field, value) -> { edits, source_digest }
    effect:  pure
    errors:  EV3:input.round_trip_undeclared, EV1:stale.editor_source, LG2:type.mismatch
  ```

- **EV3.6 [D]** Source navigation for a running tool opens the digest that produced the selected output, including
  when its editable draft is broken (EV1.7). Navigating from a declared source node finds the associated view-model
  nodes and instances for that version. A source edit invalidates that correspondence until the checked version is
  recomputed; it does not relabel old instances as products of the new source. This follows from EV1.1 and INV-6.

- **EV3.7 [D]** Round trip is limited to parameters, layouts and defaults explicitly declared editable by their source
  (partition EV3). An edit identifies the declaration, its digest, its typed field and proposed value. It returns
  source edits for checking; it does not infer arbitrary code from a drawing. An undeclared field returns
  `EV3:input.round_trip_undeclared`. Persisting a layout or parameter follows the workspace command and undo rules; an
  expression with workspace effects cannot run just because the view has recomputed (default 16, KN4.4).

- **EV3.8 [D]** The relationship strip consumes typed relations in the grammar shared by code and the theory book
  (partition EV3; DT-L1 M6, R2-E4). Its data contract carries a typed statement reference, its relation and operands,
  canonical IDs for the roles it names, its model and level, its base revision, its result and evidence references,
  and any producing source location. Roles include destination, current occupant, wanted piece, buffers, frame and
  residual. Chapter 06 defines statement identity and evidence; chapter 02 defines the types. These fields can join
  EN3's view model now, without deciding the strip's design. Three scopes hold (DT-L1 R3-M4). The relation "frame" is the
  frame group R_A/S_A, at label level. "Frame" now names four things: reference stickers, that frame group, the C₃
  component of restoration theory that no sticker shows, and A3's F_f. The strip must tell them apart, and the word it
  uses for each is a preference. "Current occupant" is partial: off the lattice it has no value.

- **EV3.9 [D]** A relation with no position off the lattice returns the distinct value "none: off the lattice";
  insufficient knowledge returns `unknown(reason)` (KN0.4, KN0.5; DT-L1 R2-E4). It never returns an old occupant, slot
  zero or false for either case. A partially defined relation retains its coverage and missing part. Relations do not
  lift labels into poses, and off the lattice they read poses, classes or regions, never labels (KN1.9, KN1.10, KN1.15).

- **EV3.10 [B]** Relations and other program view data are recomputed for changed inputs at a named revision, never
  per frame (R-19; partition EV3). Frame drawing reuses those results. Latest-wins evaluation skips superseded
  revisions (KN7.2, partition LG4); it cannot queue every missed revision or delay a commit. Camera changes do not
  trigger exact state analysis unless a declared analysis input actually changed. Per-frame chart evaluation remains
  chapter 10's display operation, outside this relation computation.

- **EV3.11 [P LQ6]** In the solving profile program charts and overlays live inside Global and Local. Research views
  may open in a research workspace, each with its own gate run (partition LQ6; charter D8). Under strict D8 everywhere,
  research content must also fit those two views. Free views everywhere require an owner change to D8. In every
  option, canonical identity, base revisions, source linkage and the absence of a program commit path stay the same.
  No additional view is authorized by this proposal.

- **EV3.12 [O]** KN2.14 permits a derived value imported from raw data without a session ancestor, while partition EV3
  requires a displayed derived state's base revision. Chapters 08 and 10 must settle this case: an explicit comparison
  base with no claim that it produced the imported value, or a separate research-reference plan whose input base is a
  digest rather than a session revision. The latter requires clarification of the partition's display contract.
  Nothing here fabricates a derivation or a revision to fill the gap.

- **EV3.13 [D]** LQ7 determines which user charts can produce plans; LQ20 determines the renderer-line handoff.
  Neither changes the linkage or the one-state rule (INV-1, INV-8; partition 8.5). Built-in parameterised charts can
  use the same service contracts if generated user charts are deferred. The handoff describes data primitives only;
  layout, appearance, motion and interface wording wait under the owner's 3 November design-line hold.

## 4. EV4: commands, bindings, macros and recording

- **EV4.1 [D]** A language-defined command registers a KN5.3 overlay row with the H-08 row fields and chapter 02's
  checked module reference. It carries the static verdict, declared capabilities and kernel rows it may invoke;
  optional rows are identified as in KN5.4. Its kind is `view`, `workspace`, `query`, or `file` with a grant. It cannot
  register a `commit` or `app` row (KN5.3 to KN5.5). The engine receives data and computes availability under KN5.7;
  it never calls language code to ask whether the row is available (default 17).

- **EV4.2 [B]** The palette includes every command, including a language overlay row (charter E5, DT-M1 Outcome C3).
  An unavailable entry keeps its place in the matches with its typed reason. Search uses labels, glossary terms,
  command IDs and key names; it exposes a command's binding before disclosure. Its availability input is KN5.6's table
  per revision, with no engine call for each search keystroke. Palette invocation uses the same gates and preview
  rule as a key. Searching and showing a command never grants permission to run it.

- **EV4.3 [D]** A `Binding` maps a key sequence and routing context to a stable command ID with absolute arguments
  (DT-M1 Outcome, Absolute arguments and Terms). Its distinction from invocation identity follows from KN5.21:
  the binding names H-08's stable dotted row ID. A submitted invocation also gets a distinct durable command identity;
  reusing a binding is a new invocation, not a retry of a previous receipt. Absolute arguments name canonical objects,
  an exact rotation or a pinned word. A relative name such as a current grip, menu index or row number is resolved with
  the context that fixes it before dispatch;
  once that context changes it cannot be reinterpreted silently.

  ```text
  Binding = {
    sequence: List<PhysicalChord>
    context:  RoutingClass
    command:  RowId
    args:     AbsoluteArguments
    layout:   LayoutReference?     # needed for layout-dependent resolution
  }
  operation resolve_binding(key_event, routing_context, bindings, layout) -> Binding | unhandled
    effect:  pure
    errors:  EV4:input.binding_conflict, EV4:input.binding_prefix_conflict, EV4:input.binding_layout_mismatch
  ```

- **EV4.4 [B]** Routing follows DT-M1 Outcome C2, in order: key capture; text input; a focused list or view taking only
  its navigation keys; then the global map for other keys. Text input, IME composition, clipboard keys and text undo
  stay with the editor; a character entered there never twists the puzzle. List navigation includes arrows, Enter to
  inspect, Tab and the settled repair-row number keys. Focus, hover, filters and the camera do not change a global
  binding's command or arguments. The explicit live or draft destination remains the input mode DT-M1 allows.

- **EV4.5 [D]** A key bound to a kernel row dispatches through KN5 without the interpreter. A key bound to an overlay
  row invokes the checked program as a job, receives its receipt and obeys its lane and budget (partition section 9,
  KN7.2). A slow or failed program cannot block kernel keys, commits or undo. A view publishes its newest result per
  revision; it cannot make workspace actions as a side effect of publication (default 16).

- **EV4.6 [B]** Only camera, list-movement and preview-scrub keys may repeat (DT-M1 Outcome K4, fix-first item 2).
  Repeat never repeats a puzzle or workspace state change (R-18). Grip, twist, preview-job submission, commit and
  toggles do not repeat. Held-key state is released on focus loss. A commit key-down is non-repeating and binds the
  shown preview under KN5.14; while another commit is in flight it is refused as busy under DT-M1 Q6 (a), never queued
  for later execution.

- **EV4.7 [D]** Portable layers store physical key codes (DT-M1 Outcome K5). The platform layer resolves labels and
  handles IME. The settled DT-M1 Q2 (b) exception keeps undo and redo's office-software characters: its
  layout-dependent intent is recorded with the layout and resolved to physical chords at that boundary. It does not
  put character handling into the engine. Key-map checks test duplicates and the prefix-code property separately in
  each routing class and for every declared layout. No complete binding may also be a prefix group. A layout conflict
  is reported before that map is applied. Chapter 05 owns the persisted intent and resolved-map forms.

- **EV4.8 [B]** The applied grip-slot map, including its absolute caps and transported frames, is workspace state
  changed only by the solver's rebind command (DT-M1 Outcome K2, Q3 (a)). Camera, filter, twist, focus and restart do
  not derive a new applied map. One default map with the solver's edits and one macro base serve both puzzles (DT-M1
  K6; DT-J1 Outcome E1). Themes change neither commands nor bindings (owner design decisions, section 3).

- **EV4.9 [D]** A macro contains a word with its KN1 identity block, literal letters or named parts, start digest,
  exact expanded length and provenance field (partition section 13, BM2; DT-J1 Outcome E1 and E3; DT-L1 R2-E3).
  Each word is typed by its alphabet. A jumble letter is refused with Jumble off. Named-part expansion is checked
  against DT-J1 X3's eventual bound, with the exact length reported before check, preview or run; this chapter sets
  no bound. Equality uses the equality key; protection and execution use literal prefixes
  (KN2.7, default 10; DT-L1 R2-E2).
  Flat storage versus references belongs to chapter 05 with EN3, without losing literal order or provenance.

- **EV4.10 [D]** A `Recording` stores semantic steps: stable command IDs, resolved arguments, canonical IDs with their
  identities, and the revisions on which the steps depend (partition EV4, LQ11). It can refer to receipts and outcomes
  to describe what actually happened. An overlay step also identifies the module digest needed to give its command
  meaning (LG5, INV-10). Pixels, screen coordinates and key labels cannot identify a recorded action (INV-6). A
  recording stores no user-action token or old execution permission. Chapter 05 defines its bytes and versioning.

- **EV4.11 [O]** The recording contract must choose between capturing every resolved invocation with its outcome,
  refusals included, and capturing only accepted semantic actions. Chapter 05 and the integrator must settle the
  capture point against the receipt lifecycle, including an unknown outcome. Under either choice the record cannot
  describe a refused or unresolved command as a committed action, and reconstructing it cannot silently retry one.

- **EV4.12 [P LQ11]** A recording becomes either a macro with one complete preview and one user commit, or a
  step-by-step playback with one user action for each commit. It never plays commits unattended. The alternative
  permits unattended view and workspace steps only; those still use the command layer, workspace undo and gates, and
  never acquire the shell's commit token. The execution gate of INV-3 holds under both choices. Recording is not
  deterministic run replay: EV2 reproduces computations without playing actions into the session.

- **EV4.13 [D]** Conversion to source names the recorded commands, absolute arguments, module and word digests, and
  revision guards. Generated source passes the same chapter 02 checks and chapter 09 grants as typed source. A word
  assembled from it receives a fresh whole-word preview and protection check; no old receipt or certificate supplies
  permission (INV-3, defaults 14 and 18). New invocations receive new durable identities. Playback that cannot resolve
  a step reports `EV4:input.recording_step_unresolved`, preserving the record for inspection. It never replaces a missing
  canonical ID with a visual pick.

- **EV4.14 [O]** Mapping recorded revisions into a new playback session is not specified by LQ11. Chapters 05 and 08
  must choose exact historical-input playback only, or an explicit rebinding operation that checks the identity,
  input digests, absolute objects and fresh previews in a new session. Both retain the recorded revisions and report
  mismatches; neither silently retargets an old command. Converting steps into editable source does not settle this
  playback mapping.

- **EV4.15 [B]** Puzzle and workspace actions share DT-M1 Q4 (a)'s durable undo order. An undo carries the action its
  hint named and is refused if another action superseded it; a text editor keeps its own text undo. Commit and undo
  go ahead during revision-bound jobs, cancelling those jobs as in DT-M1 Q5 (a). The debugger's journal cursor is
  outside this action order because it writes nothing. KN5.20's EN4 fix remains a prerequisite for these services.

- **EV4.16 [D]** Saving a word, recording it or turning it into source preserves the chapter 07 provenance reference;
  it cannot make research output into a solving preview or change an imported word into an own word by format
  conversion (BM2; DT-L1 R2-E3, fix-first item 18). Flow-label detail versus capability separation stays LQ21. Under
  the interim policy of partition section 11 no solver-tool solving preview is enabled before the joint setting.
  Built-in tools retain the 2 October boundary and the 17 October named-cap repair-list exception (INV-11). Showing
  a result or recording a solver's pick grants no new declassifier.

## 5. EV5: docs, explanations and the learning path

- **EV5.1 [B]** The mathematician oversees this component (DT-L1 owner input; partition EV5). Everything here is a
  draft for that review. Understanding is a deliverable: the owner's 3 November principle presumes that a solver
  understands the principle of their tool (INV-12). Source visibility, a competence result or a model's agreement
  alone cannot establish that understanding or decide the joint boundary.

- **EV5.2 [D]** Reference documentation is generated from source declarations and their checked metadata, with the
  source and module digests, edition, types, effects, capabilities, supported identities and documentation references
  needed to interpret each entry (partition EV5, LG5). An entry links to its declaration and evidence rather than
  keeping a second handwritten signature. A broken draft's reference output remains distinguished from the active
  tool's documentation (EV1.7). Native built-ins under an LQ5 alternative still owe INV-12's viewable source and
  principle record; the source language and generation adapter change, not that obligation.

- **EV5.3 [B]** A tool explanation consumes the `PrincipleRecord` defined in chapter 03 (INV-12). It exposes the exact
  function and domain, the meaning of an empty or negative answer, the supporting lemma or certificate with its
  evidence, a small hand-checkable instance on the real geometry, the decomposition into base parts, and the parts a
  person writes by hand (LB6.2): the map from decomposition to lemma steps, the reason, a failure example, what an answer
  does not mean, the hand derivation, the prerequisites and the leaf specifications. A completed empty repair list
  concerns the named cap and the admitted repair shape; it never asserts that no solving step exists (DT-J1 Outcome, Out
  by the filter). A cut-off job has no completed empty answer (KN7.4, KN7.11). This section supplies the means of
  understanding. It cannot certify understanding (DT-L1 R3-M4, BND.72).

- **EV5.4 [D]** Explanation links use EV2's lazy provenance expansion down to primitives and stage-qualified,
  source-commit-pinned lemmas. A long word is a digest and positions until expanded. The decomposition is generated
  from the checked IR before lowering, from its call graph and kernel calls, and the hand-checkable instance has a
  matching headless fixture (partition INV-12, LG4; DT-L1 M6, R2-E4). A generated decomposition gives structure, not
  reasons: the reasons are the hand-written parts of LB6.2 (DT-L1 R3-M4). A fixture pass is source evidence for that
  instance. It does not prove a universal lemma or record a human comprehension result. KN6 trace links use the
  checker's own output.

- **EV5.5 [D]** A fork inherits no evidence (default 18). An explanation of a fork retains links to cited statements
  and their scope, but it cannot label the changed module as certified because its parent was checked. Reusing an
  unchanged lemma needs the chapter 06 identity and scope rules; executing a changed tool still needs checking and
  current grants. Author class, evidence, competence and permission stay separate (INV-5, BM2; DT-L1 R2-E4).

- **EV5.6 [D]** The language learning path follows partition EV5: use tools; change a parameter; save layouts and
  bindings; compose recipes and macros; write tools; maintain modules and evidence. It builds on the orbit-first
  solving workflow and the glossary's L1 to L6 rather than introducing another numbered key scale (charter D7, M6;
  DT-M1 Q9 (a)). Each step names its prerequisite concepts, command routes and fixture task. Proposed term levels are
  listed in section 6; they are not new learning gates or an approved lesson order.

- **EV5.7 [B]** Competence checks use fresh isolated fixture tasks and record independent completion, critical
  mistakes, help use and transfer to a new target. Time is recorded separately from correctness (owner design
  decisions, section 5). Local in-situ metrics support the same learning questions and stay local. Checking a lesson
  never changes the solver's personal session, uploads metrics or equates fast work with correctness. The chapter's
  human conformance tests need the owner's protocol before they can run.

- **EV5.8 [P LQ14]** Competence checks guide and do not lock functions. Keys open by soft disclosure; the palette
  still lists every command, an expert can open all keys, and showing all keys changes no disclosure state (charter
  E5; DT-M1 Outcome K3). A goal flow's needed keys are disclosed by its first use, with their concept terms already
  introduced. LQ14's hard-gates alternative changes this access rule and requires the owner to reconcile it with the
  2 October soft-disclosure decision. Capability and evidence checks continue under either choice.

- **EV5.9 [O]** If LQ2 selects BS3, the competence check on a principle record gates the capability to prepare solving
  previews for that principle (partition section 11). That is a capability decision, distinct from showing a key or
  listing a command. BS1, BS2 and BS4 add no such understanding gate. The owner and integrator must settle this with
  LQ14, including what changing a principle record does to an earlier check; no source settles that validity rule.
  Until then no learning result grants a solving-preview capability.

- **EV5.10 [D]** Documentation and library terms use the glossary, checked by script (default 8, charter bar M1).
  A term reference must resolve to one concept at its declared learning level; stored citations name their source
  prefix, including a stage for a lemma or a chapter component for a contract (default 19, DT-L1 M6, R2-E4).
  An unknown term or ambiguous citation fails that check. A formatter or documentation generator cannot repair the
  meaning by guessing a similarly named concept.

- **EV5.11 [B]** The theory book defines the grammar used by code and the relationship strip and gives the proofs or
  cited proofs on which its correctness relies (charter M2, M4, M5; owner design decisions, section 2). Its 1.0 scope
  is what the interface shows. Figures use real geometry with reproducible local production evidence. This chapter's
  data contract provides relation and evidence references; it chooses no appearance, encoding token or interface
  wording. The design line remains on hold under the 3 November decision.

- **EV5.12 [O]** The owner sets the human-test form and success criteria for comprehension, fixture competence and
  transfer; the mathematician checks their mathematical content. Rubrics and the treatment of help, repeated attempts
  and changed tools are not fixed here. LQ13's LX-8 learnability test feeds syntax choice, and chapter 11 owns its
  protocol dependency. No invented pass rate, timing threshold or claimed participant result fills this gap.

- **EV5.13 [P LQ3, LQ10]** Under the recommended evidence model and facts envelope, explanation labels are derived
  by chapter 06's versioned functions over stored facts (DT-L1 R2-E1). The view exposes statement, scope, basis,
  checks and job outcome separately. Under a judgements envelope it reads the settled stored judgements and their
  version; under LQ3's other models the label mapping changes. Source labels and check references remain inspectable
  in every option. The environment neither sets the evidence order nor supplies its own stronger summary.

## 6. Glossary entries the language adds

The inventory below is source evidence from `terms.json`, the overview, chapter 01 and the partition's named concept
families, with repository research sources named for the proposed jumbling summaries. Existing entries whose meaning
works unchanged are omitted. A row marked "change" corrects or scopes an existing entry. The definitions are proposed
glossary summaries of the owning chapter, not competing contracts.

- **EV5.14 [O]** The mathematician must review these additions and scope changes against the theory and the completed
  chapters. Their proposed L1 to L6 levels use the existing glossary scale; their final placement follows the owner's
  learning path and DT-M1's first-use rule. Evidence words also depend on LQ3 and chapter 06, and the understanding
  gate depends on LQ2 and LQ14. The inventory carries round 3's corrections to DT-L1 R2-E1 (evidence), R2-E2 (identity
  levels), R2-E3 (flow control) and R2-E4 (principles and partial values). No entry or level is accepted by this
  inventory, and `terms.json` is unchanged.

- **EV5.15 [D]** Each integrated entry needs its own diagram under the existing glossary build rules (charter M6,
  default 8, `glossary/build.py`). This includes a changed concept whose old diagram no longer states its meaning.
  Integration supplies the term fields, editable `diagrams/<id>.mmd`, locally rendered SVG and the manifest's source
  and output digests, tool version, configuration digest and production command. The generated glossary must match
  `terms.json`. These diagrams and the build check are subsequent glossary work; none is created or run by this
  chapter-writing packet.

| Term | Proposed level | One-line definition | Source |
| --- | --- | --- | --- |
| Cap (scope change) | L1 | A world-fixed grip and cut; which pieces lie inside it depends on the state. | Chapter 01 KN1.5; partition KN1 |
| Label (scope change) | L1 | A sticker's identity at a lattice slot; the label array is the state of record for 600-cell-Full. | Chapter 01 KN1; partition LG2 |
| Piece (scope change) | L1 | A rigid labelled object with stickers; slot occupancy is defined only on the lattice in the jumbling puzzle. | Chapter 01 KN1; DT-L1 M3, M6 |
| Generator (scope change) | L1 | One of 600-cell-Full's retained basic generators; its signed inverse is distinct from a menu-letter count. | Chapter 01 KN1.5; DT-M1 Terms |
| Twist, 600-cell-Full (change) | L1 | A turn of one grip by a non-identity retained menu rotation, which can be a word in generators. | Overview 3.6; DT-M1 Terms |
| Twist, jumbling puzzle (change) | L1 | A turn of one grip by a non-identity active menu element, represented by a pinned menu letter. | Overview 3.6; DT-M1 Terms |
| Retained twist | L1 | A twist in the cap's retained rotation subgroup, with the same rotation when Jumble is on. | DT-M1 Outcome C6, Terms |
| Jumble twist | L1 | An active-menu twist outside that retained subgroup, refused when Jumble is off. | DT-M1 Outcome C6, Terms |
| Orbit (scope change) | L2 | A retained label-position orbit; a centre's fixed labels do not imply a fixed jumbling pose. | Chapter 01 KN1; DT-L1 M3 |
| Net and Strict checks (scope change) | L2 | Protection on the whole end state or on every executed literal prefix, at the puzzle's protection level. | Chapter 01 KN2.7; partition default 10 |
| Complete effect (scope change) | L4 | The whole operation's effect at its declared puzzle and level, including every hidden mechanical part. | Chapter 01 KN5.10; charter S4 |
| Orientation (scope change) | L5 | A piece's orientation relative to its lattice position and reference frame, distinct from a full jumbling pose. | Chapter 01 KN1; DT-L1 M3 |
| Frame (scope change) | L4 | Unqualified, the ordered reference stickers. The other three uses take their own names: frame group, the C₃ frame component no sticker shows, and state contract A3's transported frame F_f. | Chapter 01 KN1.7; DT-L1 R3-M4 |
| Position | L1 | The chamber occupied by a piece, derived from labels and undefined off the lattice. | Chapter 01 KN1.5, KN1.9 |
| Pose | L1 | A piece's exact rotation in the jumbling puzzle, independent of whether its stickers occupy lattice slots. | Chapter 01 KN1.5, KN1.6 |
| Sticker | L1 | The identified region attached to a piece and its home host facet. | Chapter 01 KN1.5 |
| Model identity | L1 | The immutable identity fixing the mechanical objects and interpretation of their data. | Chapter 01 KN1; INV-7 |
| Canonical ID | L1 | An object identifier bound to a model and identity level, independent of its display. | Chapter 01 KN1.5; INV-6 |
| Identity block | L2 | Chapter 01's bundle of puzzle, model, menu, menu order and contract references. | Chapter 01 KN0.1 |
| Identity tower | L6 | The puzzle's levels, a partial order, and the exact projections from finer information to coarser information. | Chapter 01 KN1.7; DT-L1 R2-E2 |
| Sticker class | L6 | The class g_A·S_A of a pose: poses that show every labelled sticker in the same place. Earlier name: visible pose class. | Chapter 01 KN1.7; DT-L1 N1 |
| Region class | L6 | The class g_A·R_A of a pose, which is the piece's posed region. | Chapter 01 KN1.7; DT-L1 N1 |
| Frame group | L6 | The group R_A/S_A of a piece: the turns that keep its region and permute its sticker regions. | Chapter 01 KN1.7; DT-L1 N1 |
| Gauge | L6 | A pair (σ, t) that renames pieces and changes their pose representatives exactly; by Theorem 1 of note N1 it keeps every word's admissibility. | Chapter 01 KN1.7, KN1.14; DT-L1 N1 |
| Shape | L6 | The set of posed regions of a state, forgetting which piece fills which region. | Chapter 01 KN1.7; DT-L1 N1 |
| Exact transport | L6 | Reuse of a claim along a level's gauge subgroup with no re-run, under a checked relation. | Chapter 01 KN1.16; chapter 06 |
| Level-qualified digest | L2 | A digest of normalised bytes naming the identity level it covers. | Chapter 01 KN1.8 |
| Menu | L1 | The pinned set of rotations available to a grip in the named puzzle. | Chapter 01 KN1; partition LG2 |
| Menu letter | L1 | A grip and menu-item reference meaningful under its menu identity, order digest and contract revision. | Chapter 01 KN1.3 |
| Menu identity | L1 | The order-independent identity of the menu's rotations. | Chapter 01 KN1.3 |
| Menu-order digest | L1 | The digest fixing the ordered menu that gives an item index its meaning. | Chapter 01 KN1.3 |
| Seat | L1 | DT-M1's pinned right-coset input for the additional menu twists, with its base rotation fixed. | DT-M1 Outcome Seats; Q7 (a) |
| Jumble on and Jumble off | L1 | The two session modes, without merging the two puzzles' model identities. | Overview 3.6; chapter 01 KN0.2 |
| Word | L2 | A chronological list of letters in a named alphabet and identity. | Overview 3.6; chapter 01 KN2 |
| Literal word | L2 | The word as written before normalisation, whose prefixes are checked and executed. | Chapter 01 KN2.7 |
| Equality key | L2 | A word's normal form used for equality, without replacing its literal execution path. | Chapter 01 KN2.7; default 10 |
| Recipe | L3 | An operation composed of word and star steps with their complete chronological expansion. | Partition 4.1, 10.2; chapter 05 |
| Compressed word (straight-line program) | L6 | A word represented by a finite composition of named parts with an exact expanded length. | Chapter 01 KN2.10; chapter 05 |
| Macro | L2 | A reusable named word with its identity, exact length, start reference and provenance. | EV4.9; DT-J1 Outcome E1, E3 |
| Macro base | L2 | The shared collection of typed macros for both puzzles, with per-entry identities and provenance. | EV4.8, EV4.9; DT-J1 Outcome E1 |
| Cost alphabet | L2 | The named unit of a word or action count, such as retained generators or menu letters. | Chapter 02; partition LG2 |
| Admissible | L1 | The jumbling-twist outcome that permits the exact proposed rotation on its input state. | Chapter 01 KN2.8 |
| Blocked | L1 | The jumbling-twist refusal caused by a cut-straddling piece, with the state unchanged. | Chapter 01 KN2.8; DT-J1 fix-first item 3 |
| Invalid | L1 | The jumbling-twist outcome for invalid input, distinct from a blocked cut. | Chapter 01 KN2.8 |
| Uncertain | L1 | The distinct unresolved jumbling-twist outcome that permits no state change. | Chapter 01 KN2.8 |
| Lattice | L1 | The configurations where the label projection and sticker-slot placement are defined. | Chapter 01 KN1.9; partition LG2 |
| Straddler set | L1 | The complete set of pieces straddling a named cap's cut at a named revision. | Chapter 01 KN2.4; DT-J1 fix-first item 3 |
| N | L1 | The total misalignment count of (domain, cut-cell) pairs covered on only one side by that domain. | Chapter 03; [restoration section 2](../../../../../research/jumbling/theory/restoration.md#2-misalignment-and-the-rim-test) |
| Lowering supply D_x(h) | L6 | The number of inside pieces that a named item lowers at a grip, under the source's height and branch conditions. | Chapter 03; [card 7, Terms](../../../../../research/jumbling/theory/cards/card-7-lowering-machine.md) |
| Depth Φ | L6 | Under I_a, the sum over pieces of their left height above two, counting no negative contribution. | Chapter 03; [card 6, Model](../../../../../research/jumbling/theory/cards/card-6-blocking-structure.md) |
| Height | L6 | The tree distance of a jumbling pose from the lattice root, with the tree side and menu stated. | Chapter 03; [restoration T5 to T7](../../../../../research/jumbling/theory/restoration.md) |
| Domain ID | L6 | The exact identifier of a pose's alignment class, supplied with the height batch at its input revision. | Chapter 01 KN2.4; chapter 03 |
| Height counts | L1 | The per-revision counts of pieces at the declared exact height values. | Chapter 01 KN2.4; DT-J1 Outcome A1 |
| Rim summary | L1 | The named cap's per-revision cut facts, without selecting a twist or a target cap. | Chapter 01 KN2.4; DT-J1 rule 6 |
| Lifted (working term) | L1 | The lattice pieces taken off the lattice by a preview, distinct from its full moving set. | DT-J1 M22, fix-first item 4; chapter 03 |
| Jumbling block | L6 | Pieces that no cut separates and therefore move together, distinct from a solver-declared achievement block. | Chapter 03; [tail section 3.1](../../../../../research/jumbling/manual/03-tail.md) |
| Jumbling frame domain | L6 | A height-two alignment domain in the jumbling tree, distinct from an ordered sticker reference frame. | Chapter 03; [card 6, Model](../../../../../research/jumbling/theory/cards/card-6-blocking-structure.md) |
| Clean-step unit | L6 | A jumbling block or one frame part of a block, as used by the tail's clean-step contract. | Chapter 03; [tail section 3.2](../../../../../research/jumbling/manual/03-tail.md) |
| Pose unit | L6 | A set of pieces with one exact pose, as used in the blocking-structure card. | Chapter 03; [card 6, Terms](../../../../../research/jumbling/theory/cards/card-6-blocking-structure.md) |
| Clean step | L6 | A legal word landing its unit, keeping prior lattice pieces on the lattice and preserving other off pieces outside its free set. | Chapter 03; [tail section 3.2](../../../../../research/jumbling/manual/03-tail.md) |
| Excursion | L6 | The largest rise of the named depth measure along a literal word relative to its starting value. | Chapter 03; [card 7](../../../../../research/jumbling/theory/cards/card-7-lowering-machine.md) |
| ν | L5 | The lattice invariant queried only where its label-level domain is defined. | Chapter 01 KN2.4; DT-J1 Outcome A3 |
| ν-correction | L5 | A solver-selected named construction with a fixed effect on the ν coordinates. | DT-J1 O7 (a); chapter 03 |
| Repair list | L1 | The neutral-order repair pairs at a cap the solver names under the 17 October exception. | Chapter 01 KN2.4; DT-J1 |
| Retained checkpoint | L5 | The retained end predicate of chapter 01, kept distinct from the label checkpoint and sticker test. | Chapter 01 KN2.4; default 9 |
| Label checkpoint | L5 | The label-level checkpoint predicate permitting the certified lattice crossing. | Chapter 01 KN2.4; DT-J1 O8 (1) |
| Sticker test | L5 | The label-based completion test, distinct from equality of every exact pose. | Chapter 01 KN2.4; DT-J1 O8 (1) |
| Revision reference | L2 | The session, epoch, identity and state, workspace and protection revisions bound to a result. | Chapter 01 KN0.2 |
| State reference | L6 | The typed reference to a snapshot or derived value used by kernel reads. | Chapter 01 KN0.3 |
| Snapshot | L2 | A read-only handle pinned to one revision for its lifetime. | Chapter 01 KN2.2 |
| Derived state | L6 | An immutable kernel-computed value with a derivation and reachability claim, without a session revision. | Chapter 01 KN2.11 to KN2.14 |
| Derived model | L6 | A quotient, restriction or simplified model represented by a derivation and claim-preserving maps. | Chapter 01 KN1.12, KN1.13 |
| Difference | L2 | A comparison of named states at a declared model and level. | Partition 4.1; chapters 02, 03 |
| Typed partial result | L1 | A result that identifies its covered and missing parts or why it is undefined. | Chapter 01 KN0.4 |
| Verdict | L2 | Yes or no with evidence, or unknown with a reason, for a stated decision. | Chapter 01 KN0.5 |
| None: off the lattice | L1 | A known absence of a lattice value, distinct from insufficient knowledge. | EV3.9; chapter 01 KN0.4; DT-L1 R2-E4 |
| Reachability claim | L2 | Evidence about reachability in a named puzzle, kept with the state it describes. | Chapter 01 KN2.13; chapter 06 |
| Witnessed reachability | L2 | Reachability accompanied by a finite legal witness in the named puzzle. | Chapter 01 KN2.13 |
| Reachable by theorem without a witness | L6 | A reachability claim supported by the cited theorem without a stored legal word. | Chapter 01 KN2.13 |
| Promised reachability | L6 | A reachability assumption supplied by the source and not upgraded by input validation. | Chapter 01 KN2.13, KN2.14 |
| Proposal | L2 | A word and provenance submitted for an engine-owned preview at a base revision. | Chapter 01 KN5.9 |
| Preview | L2 | The engine's immutable witness and full-effect result at one base revision. | Chapter 01 KN5.9 |
| User-action token | L2 | The shell's single-use authority bound to the displayed preview and its literal letters. | Chapter 01 KN5.14, KN5.15 |
| Receipt | L2 | The command's durable identity, request digest and typed execution outcome. | Chapter 01 KN5.21 |
| Availability | L1 | The per-revision command availability and typed reasons supplied through KN5. | Chapter 01 KN5.6, KN5.7 |
| Claim | L2 | A typed statement with an evidence state, without conflating score or execution permission. | Chapter 06; INV-5 |
| Statement ID | L6 | A typed-statement digest or stage-qualified, commit-pinned statement name. | Chapter 06; DT-L1 R2-E1 |
| Key | L6 | A statement ID with its scope: what evidence discharges, refutes or transports. | Chapter 06 EVD.7 |
| Scope | L2 | The declared inputs, identities and set for which a claim is asserted, every field immutable. | Chapter 06; partition 8.1 |
| Basis | L2 | The proof, certificate, exact computation or sample on which a claim rests. | Chapter 06; partition 8.1 |
| Check | L2 | A recorded confirmation with its checker or reviewer identity and mode. | Chapter 06; partition 8.1 |
| Premise | L2 | A named obligation on which a conditional claim depends. | Chapter 06; partition 8.1 |
| Domain | L6 | The declared inputs on which an operation or claim is defined or admitted. | Chapters 02, 06; partition 4.1 |
| Goal | L2 | A predicate typed by puzzle and identity level, with its completion meaning stated. | Chapter 01 KN1.11; chapter 02 |
| Certificate | L2 | Data supporting stated claims for an independent checker of a named kind. | Chapter 01 KN6; chapter 06 |
| Check report | L2 | The checker's verdict, identity, input digest and per-claim trace reference. | Chapter 01 KN6.1 |
| Evidence | L2 | The support for a scoped claim, separate from author class, score and permission. | INV-5; chapter 06 |
| Evidence envelope | L6 | Chapter 05's stored evidence carrier, whose facts or judgements depend on LQ10. | Chapters 05, 06; partition LQ10 |
| Provenance graph | L2 | The linked arguments and inputs through which a result can be traced to its support. | Chapter 06; EV2.5 |
| Per-claim trace | L2 | The independent checker's own steps and outcomes for each checked claim. | Chapter 01 KN6.4; EV2.6 |
| Evidence words | L2 | The versioned display vocabulary for evidence and run status, interpreted by source and field. | Chapter 06; EV5.13 |
| Proved | L2 | The evidence model's label for an established claim at its stated scope. | Chapter 06; partition LQ3 |
| Conditional | L2 | A claim whose stated premises have not all been discharged. | Chapter 06; partition 8.1 |
| Finite instance | L2 | A display label limited to the declared finite inputs, without a universal claim. | Chapter 06; partition 8.1 |
| Not found within budget | L2 | A bounded run found no witness and supplies no unscoped negative claim. | Chapter 01 KN7.11; chapter 06 |
| Incomplete | L2 | The record does not describe a completed run or full declared check. | Chapter 01 KN7; chapter 06 |
| Refuted | L2 | A claim contradicted by evidence of the kind required for that statement. | Chapter 06; partition 8.1 |
| Open | L2 | The stated claim is unresolved by the available evidence. | Chapter 06; partition 8.1 |
| Verified | L2 | The command-layer evidence label interpreted in its check context, without granting execution. | Chapter 06; H-08 |
| Recorded | L2 | A source's claim or result retained without treating it as a fresh independent check. | Chapter 06; partition PS3 |
| Unknown | L1 | Insufficient evidence for the requested conclusion, never false or a numeric zero. | Chapter 01 KN0.5; INV-5 |
| Unchecked | L2 | A declared check has not been performed for the stated input and context. | Chapter 06; H-08 |
| Computed | L6 | A research label whose meaning is mapped from its format, field and value. | Chapter 06; partition PS3 |
| Measured | L6 | In the research evidence vocabulary, exact computation on finite inputs, not a performance timing. | Chapter 06; partition 8.1 |
| Lead | L6 | A source-qualified research indication retained without promotion to a proof. | Chapter 06; partition 8.1 |
| Reported | L6 | A source-qualified result whose producer or replayability status remains explicit. | Chapter 06; partition 8.1 |
| Exact transport | L6 | A lemma-justified transfer of a claim between inputs that preserves the claim's subject. | Chapter 01 KN1.13, KN7.5; chapter 06 |
| Check mode | L6 | The recorded distinction among reused transport, fresh exact audit and actual replay. | Chapter 06; partition 8.1 |
| Grade | L6 | A (basis, check) pair in chapter 06's partial order. | Chapter 06 EVD.19 |
| Alternative | L6 | One undominated argument for a statement; a statement's summary is a set of them. | Chapter 06 EVD.23 |
| Standing | L6 | Whether the evidence graph carries a statement at the threshold: first with no premise assumed, then with every unrefuted premise assumed. | Chapter 06 EVD.30 |
| Principle record | L2 | The tool's specification, support, hand-checkable real instance and decomposition. | Chapter 03; INV-12 |
| Library entry | L6 | A source-addressed operation with its checked domain, effects and supporting references. | Chapter 03 |
| Effect | L6 | A declared and checked kind of interaction a program can perform. | Chapter 02; partition LG3 |
| Capability declaration | L6 | A module's request for authority that the host must grant for the profile and run. | Chapters 02, 09 |
| Typed IR | L6 | The checked intermediate representation consumed by execution and environment services. | Chapter 02; partition LG4, LQ9 |
| Module | L6 | A digest-addressed source unit with its edition, manifest and dependencies. | Chapter 02; partition LG5 |
| Module manifest | L6 | The module's declared supported identities, capabilities, edition and dependencies. | Chapter 02 |
| Lockfile | L6 | The dependency resolution that pins the digests used by a run. | Chapter 02 |
| Edition | L6 | A named source or semantic version whose changes preserve recorded saved meaning through migration. | Chapters 02, 05 |
| Profile | L6 | The chapter 07 capability and labelling context, without deciding a separate product entrance. | Chapters 07, 09; partition LQ1 |
| Author class | L6 | The recorded source class of a tool, distinct from mathematical evidence and understanding. | Chapter 07; partition BM2 |
| Provenance block | L6 | Chapter 07's assistance-origin information carried by previews and stored words. | Chapter 07; partition BM2 |
| Boundary class | L6 | A shipped tool's reviewed class under the human-solve boundary. | Chapter 07; partition BM5 |
| Declassifier | L6 | A fixed policy-enabled entry that permits a named information flow; programs cannot declare one. | Chapter 07; partition BM2 |
| Flow label | L6 | The provenance information following permitted data and control dependencies if LQ21 selects labels. | Chapter 07; partition LQ21 |
| Capability separation | L6 | LQ21's alternative separating state-reading programs from preview-preparing programs. | Chapter 07; partition BM2 |
| Solving session | L2 | A session in the solving profile, where the human-solve boundary applies. | Chapter 07 BND.14 |
| Sink | L6 | A place a labelled value can reach: a preview, a draft, a display, a workspace, a file or a record. | Chapter 07 BND.26 |
| Transparent entry | L6 | A shipped function on the policy's list whose output carries only the join of its inputs. | Chapter 07 BND.25 |
| Policy table | L6 | The versioned data that fills the boundary mechanisms with a joint setting's choices. | Chapter 07 BND.28 |
| Assistance summary | L2 | The per-solve account of which tools assisted, derived from provenance blocks. | Chapter 07 BND.35 |
| Pick record | L6 | The record of a solver's choice among listed options, with the list that offered them. | Chapter 07 BND.54 |
| Job | L2 | Admitted cancellable work with a pinned input, budget and typed outcome. | Chapter 01 KN7 |
| Job result | L2 | A value with outcome, input digest, parameters, coverage and budget use. | Chapter 01 KN7.3 |
| Budget | L2 | Deterministic limits on steps, declared kernel cost, logical memory and output. | Chapter 01 KN7.7 |
| Lane | L6 | A named admission rule for concurrent commands or jobs. | Chapter 01 KN7.2 |
| Kernel row | L6 | A command-layer row invoking kernel behavior through its gates without language execution. | Chapter 01 KN5 |
| Language overlay | L6 | The data-only command-table rows registered by checked language modules. | Chapter 01 KN5.3 to KN5.5 |
| Latest wins | L2 | Admission or recomputation that supersedes an older request with the newest applicable input. | Chapter 01 KN7.2 |
| Reproducibility key | L6 | The complete identity of a deterministic run's source, inputs, seed, units and budgets. | Chapter 09 |
| Run record | L6 | The stored run inputs, reproducibility references, stop reason and results. | Chapter 05 |
| Branch | L6 | An engine-owned journal line whose availability in solving sessions remains LQ4. | Chapters 01, 08 |
| Journal | L2 | The session store's parent-linked durable events, distinct from the debugger's inspection cursor. | Chapter 01 KN4.1; chapter 08 |
| Checkpoint | L2 | A stored session point with the replay path needed to reconstruct and validate it. | Chapter 01 KN4; chapter 05 |
| Workspace record | L2 | Persistent work around the puzzle changed only through workspace commands, with provenance where required. | Chapter 01 KN4.4; chapter 05 |
| Filter | L1 | A declared query limiting display or interaction without changing the mechanical state. | Chapter 05; solving workflow section 4.3 |
| Preference | L1 | A persisted user setting whose stored meaning is versioned by chapter 05. | Chapter 05; partition PS1 |
| Search space | L6 | The declared set and alphabet over which a research search ranges. | Chapter 03; partition LB5 |
| Search record | L6 | The inputs, coverage and outcome of a search, including a bounded failure to find a witness. | Chapters 03, 05, 06 |
| Strategy | L6 | A research procedure composed over a declared search space and budget. | Chapter 03; partition LB5 |
| Rank | L6 | A declared quantity with stated decrease obligations on the admitted operations. | Chapter 03; partition 10.3 |
| View-model node | L6 | The declarative node by which program content reaches the host. | Chapter 10; EV3.2 |
| View | L1 | A revision-bound presentation of information, subject to charter D8 and LQ6. | Chapter 10; EV3.11 |
| Panel | L6 | A declared host container for view-model content and its command references. | Chapter 10; EV3.2 |
| Control | L6 | A view-model element naming a typed command interaction. | Chapter 10; EV3.2 |
| Overlay | L1 | Declarative information attached to a view through a display plan. | Chapter 10; partition EV3 |
| Display plan | L6 | The renderer-independent description consumed by the adapter, with its base and derivation references. | Chapter 10 |
| Display instance | L1 | A particular drawn occurrence that maps back to a canonical object. | Chapter 10; default 7 |
| Source map | L6 | Chapter 10's linkage between produced display instances, canonical IDs and source locations. | Chapter 10 |
| Source range | L6 | A digest-bound source span under the editor protocol's declared position convention. | EV1.1, EV1.10 |
| Diagnostic | L6 | A typed code, source range and context explaining a failed or incomplete source check. | EV1.4 |
| Typed error | L2 | A stable category and code with typed context and a defined continuation. | Chapter 01 KN5.24; chapter 13 |
| Recovered parse | L6 | A source structure retaining invalid or missing spans for editor services without executable gaps. | EV1.2 |
| Editor service | L6 | A source-bound operation for parsing, completion, diagnostics, hover, definition or rename. | EV1.1 to EV1.6 |
| Debugger | L6 | The read-only service for stepping checked evaluation and checker traces and inspecting their values. | EV2.1, EV2.6 |
| Deterministic replay | L6 | Re-evaluation from a complete run record under its reproducibility key, with an explicit result comparison. | EV2.3, EV2.4 |
| Cost profile | L6 | A report of declared deterministic costs, budget usage and stop reason in KN7 units. | EV2.7 |
| Round trip | L6 | A checked source edit derived from an explicitly editable parameter, layout or default. | EV3.7 |
| Command | L1 | A stable command-table row invoked with resolved arguments through the shared gates. | Chapter 01 KN5; EV4.1 |
| Binding | L1 | The map from a key sequence and routing context to a command ID with absolute arguments. | EV4.3; DT-M1 |
| Recording | L2 | A sequence of semantic commands and resolved objects with revision references. | EV4.10; chapter 05 |
| Routing class | L1 | The key-routing context that separates text, list or view navigation, and global dispatch. | EV4.4; DT-M1 Outcome C2 |
| Portable key layer | L6 | The physical-code binding data below platform character labels and IME handling. | EV4.7; DT-M1 Outcome K5 |
| Layout | L6 | A declared placement of view-model content, with round trip only where source permits it. | Chapters 05, 10; EV3.7 |
| Chart | L6 | A display map governed by chapter 10's declared domain and precision. | Chapter 10; partition LQ7 |
| Projection | L1 | A stated map used to display the geometry, without altering its canonical objects. | Chapter 10; charter M2 |
| Section | L6 | A declared representation of a geometric section with its exact mathematical basis. | Chapters 03, 10; partition LB4 |
| Atlas | L6 | A collection of linked charts with their domains and object correspondences. | Chapters 03, 10; partition LB4 |
| Action | L6 | The declared mathematical action on the typed objects of a representation. | Chapter 03; partition LB4 |
| Embedding | L6 | A representation map with its source objects and stated mathematical properties. | Chapter 03; partition LB4 |
| Relation | L1 | A typed statement about named operands at a declared identity level and revision. | EV3.8; chapters 02, 06 |
| Graph | L6 | A representation with typed vertices and edges whose meanings are declared. | Chapter 03; partition LB4 |
| Tree | L6 | A tree representation with declared node identities and edge relations. | Chapter 03; partition LB1, LB4 |
| Cell complex | L6 | A representation of cells and their incidence relations at a stated dimension. | Chapter 03; partition LB4 |
| State graph | L6 | A graph of named states and declared transitions, without making its nodes authoritative sessions. | Chapters 03, 10; partition LB4 |
| Relationship strip | L1 | The consumer of the shared typed-relation grammar inside Global and Local. | EV3.8; owner design decisions section 2 |
| Competence check | L1 | An isolated task recording correctness, mistakes, help and transfer, with access policy left to LQ14. | EV5.7 to EV5.9 |
| Soft disclosure | L1 | Gradual introduction of key information while the palette continues to offer every command. | EV5.8; owner design decisions section 5 |

## 7. Error codes added by this chapter

These are the codes used by the clauses above. Chapter 13 owns the final spelling. `KN1:input.identity_mismatch` and
`KN2:stale.revision_gone` are reused from chapter 01 rather than added here. Cancellation, budget exhaustion and command
receipts keep their KN7 and KN5 outcomes; an open or unknown evidence value is not another error category.

| Code | Meaning | Clauses |
| --- | --- | --- |
| `LG1:syntax.recovery_gap` | A missing or invalid source span was recovered for editing; it supplies no executable value. | EV1.2, EV1.4 |
| `LG2:type.mismatch` | A source expression, edit or declared parameter has an incompatible type. | EV1.4, EV1.6, EV3.7 |
| `LG2:type.exact_required` | An approximate value reaches an exact decision site. | EV1.4 |
| `LG3:effect.undeclared` | An inferred effect is outside the declaration or the allowed view effects. | EV1.4, EV3.2 |
| `EV1:evidence.obligation_unmet` | A required claim or check is absent for a checked operation's obligation. | EV1.4 |
| `EV1:stale.editor_source` | An editor response or proposed edit names source bytes that are no longer its target. | EV1.1, EV1.6 |
| `EV2:input.replay_dependency_missing` | A required digest, source edition, IR version or trace input cannot be resolved. | EV2.1, EV2.3, EV2.6 |
| `EV2:evidence.replay_mismatch` | Replay under the stated key differs from the record's required exact comparison. | EV2.4 |
| `EV2:capability.debug_write_forbidden` | Debugging or replay would require a live write or an unsupported effect. | EV2.1, EV2.9 |
| `EV3:stale.source_link` | A pick or source link does not match the adopted plan's base or producing source. | EV3.5, EV3.6 |
| `EV3:input.round_trip_undeclared` | The selected source part does not declare the requested round-trip edit. | EV3.7 |
| `EV4:input.binding_conflict` | Bindings resolve to a duplicate or conflicting physical chord in a routing class. | EV4.3, EV4.7 |
| `EV4:input.binding_prefix_conflict` | A complete command binding is also a prefix of another sequence in that class. | EV4.7 |
| `EV4:input.binding_layout_mismatch` | Layout-dependent resolution is missing or conflicts under a declared layout. | EV4.7 |
| `EV4:input.recording_step_unresolved` | A recorded semantic step lacks a required command, module, object or input mapping. | EV4.13, EV4.14 |

## 8. Conformance tests

These are test requirements for chapter 12's phases. No conformance test listed here exists or has been run. A source
or synthetic test cannot establish native input behavior or a performance result. Performance and human tests retain
the unresolved protocols named in their checks.

| Test | Checks | Class |
| --- | --- | --- |
| CT-EV1-01 | EV1.1, EV1.2: an incomplete declaration yields recovered spans and completion context but no runnable module. | source |
| CT-EV1-02 | EV1.4: fixtures for types, effects, exactness, evidence obligations and model or menu mismatches produce distinct codes and correct source ranges. | source |
| CT-EV1-03 | EV1.5, EV1.6: hover and definition resolve pinned dependencies; rename changes resolved references only and refuses stale edit targets. | source |
| CT-EV1-04 | EV1.7, EV1.8: breaking a draft leaves the old digest running; every route refuses an unchecked new digest. | source |
| CT-EV1-05 | EV1.10: multi-unit characters, insertion ranges and end positions round-trip through the chosen coordinate convention. | synthetic |
| CT-EV1-06 | EV1.9: parse, diagnose and completion meet the chapter 11 LX-4 bars on realistic modules once the bars and protocol are set. | performance |
| CT-EV1-07 | EV1.12: each editor refusal exposes the event, reason and next-action data without handling that depends on English text. | source |
| CT-EV2-01 | EV2.1, EV2.8: enumeration of debugger operations and stepping over effect sites changes no journal, workspace or session digest and emits no commit. | source |
| CT-EV2-02 | EV2.2: exact values, declared display precision, partial results and unknown verdicts stay distinct in inspection. | source |
| CT-EV2-03 | EV2.3, EV2.4: a complete reproducible record matches its exact bytes, outcome, coverage and charges on repeated headless replays. | source |
| CT-EV2-04 | EV2.3: a missing module and another model or reversed menu order report their specific blockers without evaluating a replacement file. | source |
| CT-EV2-05 | EV2.4: a planted result difference reports a replay mismatch; a safety-stopped record remains incomplete without a negative claim. | synthetic |
| CT-EV2-06 | EV2.5, EV2.11: a compressed long step resolves the cited literal positions lazily under the chosen interval convention and retains its alphabet. | source |
| CT-EV2-07 | EV2.6: checker trace steps bind the certificate and checker digests; the debugger imports no producer construction code. | source |
| CT-EV2-08 | EV2.7: cached and uncached runs have the same deterministic charges; elapsed seconds never change the stop step. | source |
| CT-EV2-09 | EV2.8, EV2.10: journal travel uses query rows and leaves the active line unchanged; unavailable revisions and LQ4 cancellation remain explicit. | source |
| CT-EV2-10 | EV2.9: effectful replay follows the settled contract, refuses live writes and distinguishes trace inspection from complete re-execution. | source |
| CT-EV3-01 | EV3.1, EV3.5, EV3.6: source to node to instance to canonical ID to source returns the producing digest; duplicated instances retain one canonical identity. | synthetic |
| CT-EV3-02 | EV3.3, EV3.10: a burst of revisions adopts the newest engine bundle at once, marks old program output stale and skips obsolete view evaluations. | synthetic |
| CT-EV3-03 | EV3.4, EV3.12: an anchored derived display names its derivation and base without acquiring a revision; the unanchored case follows the settled display contract. | source |
| CT-EV3-04 | EV3.5: a delayed pick from an old revision or old source returns a stale link and never selects a replacement object. | synthetic |
| CT-EV3-05 | EV3.7: declared parameter, layout and default edits recheck source; an arbitrary drawn part cannot be round-tripped. | source |
| CT-EV3-06 | EV3.8 to EV3.10: an off-lattice relation is distinct from unknown; unchanged frame draws perform no relation computation. | source |
| CT-EV3-07 | EV3.2, EV4.15: a view cannot write workspace state or make undo stale; an explicit workspace command joins the durable action order. | source |
| CT-EV3-08 | EV3.11: the selected LQ6 policy admits only its permitted view hosts while every plan keeps identity, base and source linkage. | source |
| CT-EV4-01 | EV4.1, EV4.2: overlay commands appear in the palette; forbidden kinds are refused and availability uses stored tables without running programs. | source |
| CT-EV4-02 | EV4.3, EV4.4: key capture, text, list navigation and global dispatch run in the fixed order; resolved absolute arguments survive later camera and focus changes. | synthetic |
| CT-EV4-03 | EV4.5: a blocked interpreter leaves kernel-row key dispatch independent and undo available through KN5. | source |
| CT-EV4-04 | EV4.6: held-key events repeat only the allowlist; a workspace append, preview job, twist or commit is admitted once and busy input is not queued. | synthetic |
| CT-EV4-05 | EV4.4, EV4.6, EV4.7: actual Windows IME, text undo and focus loss suppress puzzle dispatch and clear held keys. | windows |
| CT-EV4-06 | EV4.7: duplicate and prefix conflicts fail per routing class; the DT-M1 US and French layout counterexample fails under its conflicting layout. | synthetic |
| CT-EV4-07 | EV4.8, EV4.9: rebind alone changes the applied map; both puzzles use one macro base with identity, alphabet, literal order and exact expansion length preserved. | source |
| CT-EV4-08 | EV4.10 to EV4.14: a recording contains semantic commands, canonical IDs and revisions, converts to checked source and follows the chosen capture and rebinding contracts. | source |
| CT-EV4-09 | EV4.12: playback under the recommendation pauses for every user commit; the alternative automates view and workspace steps only; neither stores or replays a token. | source |
| CT-EV4-10 | EV4.13, EV4.16: recording and source conversion preserve provenance, cannot revive old permission and cannot launder research output into a solving preview. | source |
| CT-EV5-01 | EV5.2 to EV5.5: generated docs and principle records resolve their pinned source, domains, empty-answer meanings, instances, decomposition and evidence; a fork inherits no check. | source |
| CT-EV5-02 | EV5.6, EV5.10, EV5.14, EV5.15: the integrated glossary covers chapter terms, proposed changes have been reviewed, every entry has build evidence, and flow goals need no undisclosed concept. | source |
| CT-EV5-03 | EV5.7, EV5.8: non-authors perform isolated fixture and transfer tasks under the owner's rubric; correctness, time and help use are reported separately. | human |
| CT-EV5-04 | EV5.8, EV5.9: ordinary soft disclosure never locks commands; if the owner chooses BS3, only the settled per-principle capability gate changes preview access. | source |
| CT-EV5-05 | EV5.1, EV5.3, EV5.11: the mathematician reviews the principle explanations and shared grammar; people can follow a real example to its primitives under the owner's comprehension protocol. | human |
| CT-EV5-06 | EV5.13: each selected evidence-model and envelope option is displayed through chapter 06's rules without merging job outcome, evidence, score or permission. | source |

## 9. Open items

The partition's owner questions remain open. This table records their effect on this chapter; it supplies no answers.

| Question | What stays fixed and what the answer changes | Clauses |
| --- | --- | --- |
| LQ1, one language or several | Digest-bound source and records stay fixed; shared versus separate research tooling changes editor coverage and the final learning steps. Capability profiles do not by themselves authorize separate product entrances. | EV1.1, EV5.2, EV5.6 |
| LQ2, solver-developed tools | The execution gate and built-in boundary stay; the joint policy decides admitted preview and display flows and any BS3 competence gate. | EV4.16, EV5.1, EV5.9 |
| LQ3, evidence model | Source labels and references remain traceable; diagnostic obligations and explanation summaries use the selected evidence model. | EV1.4, EV2.5, EV5.13, EV5.14 |
| LQ4, branches and pinned jobs | Journal inspection remains read-only; branch hosts and cancellation of pinned runs follow the selected option. | EV2.8, EV2.10 |
| LQ5, built-in implementation | Every built-in retains source and a principle record; the source language, debugger mapping and documentation adapter can differ for native tools. | EV1.7, EV2.12, EV5.2 |
| LQ6, views and D8 | Identity, revisions and source linkage stay fixed; allowed research and solving view hosts change. | EV3.11 |
| LQ7, user charts | Picking remains instance-based; generated charts, built-in parameterised charts or a user shader route change chapter 10's admitted plan producers. | EV3.5, EV3.13 |
| LQ8, engine language | Service meaning and KN7 units remain; library adapters, the checker host and LX-4's implementation depend on this choice. | EV1.9, EV2.7 |
| LQ9, route and placement | Checked versions and read-only debugging remain; transport, fault containment, IR stepping and source mappings need the selected route. | EV1.9, EV1.11, EV2.12, EV4.5 |
| LQ10, persistence freeze | Semantic identities and provenance remain; chapter 05 fixes stored recordings, bindings and evidence facts or judgements in the chosen freeze. | EV4.7, EV4.9 to EV4.14, EV5.13 |
| LQ11, recording playback | No unattended puzzle commit is admitted; the alternative adds unattended view and workspace playback. | EV4.10 to EV4.14 |
| LQ12, module sharing | Modules resolve by digest; the accepted sharing scope changes dependency acquisition, not permission or replay identity. No network capability is granted here. | EV1.1, EV2.3, EV5.2 |
| LQ13, syntax | Service operations and source identity remain; recovery structures, ranges and rename syntax follow the selected edition and LX-8. | EV1.2, EV1.6, EV1.10, EV5.12 |
| LQ14, learning gates | Competence facts stay apart from evidence and progress; guidance versus hard access gates needs the owner's decision with LQ2. | EV5.7 to EV5.9 |
| LQ15, formal proofs | Existing checker traces and provenance remain; a proof-assistant choice adds evidence links and learning material under chapter 06. | EV2.6, EV5.3, EV5.11 |
| LQ16, the 1.0 cut | The named services retain their contracts; the shipped library and research coverage changes which docs, fixtures and learning material must be delivered. | EV5.2, EV5.6 |
| LQ17, name | Neutral service and data meanings remain; language name and source extension stay working names. | EV1.1, EV5.2 |
| LQ18, evidence spikes | No measurement is claimed here; permission and timing of LX-4 and LX-8 work follow chapter 11. | EV1.9, EV5.12 |
| LQ19, freeze order | No gate is relaxed; when authoring services and the editor's design can be built depends on the charter decision and chapter 12. | EV1.11, EV3.13, EV5.11 |
| LQ20, renderer handoff | The declarative plan boundary stays; a handoff before renderer freeze or fitting the existing feature set changes its integration work. | EV3.13 |
| LQ21, flow control | Provenance survives storage and conversion; labels versus capability separation changes diagnostics, inspectable dependencies and preview grants. | EV1.4, EV2.2, EV4.16 |

The clauses tagged `[O]` leave these specific contracts unsettled:

| Clause | Open mechanism or decision | Decides it |
| --- | --- | --- |
| EV1.10 | Source-range units, endpoints and host conversion | LP1 editor protocol with chapter 10 |
| EV1.11 | Automatic promotion versus explicit activation of a checked version, including running old jobs | Integrator's LP1 service contract; LQ9; later design-line interaction |
| EV2.9 | Captured inputs and recorded-effect inspection versus isolated effect sinks for replay | Chapters 05 and 09 |
| EV2.11 | Word interval convention and compressed expansion positions | Chapter 05 and KN6.4 trace contract |
| EV2.12 | Executable step granularity and mappings for the selected execution route | Chapter 02, LQ9 and LP1 |
| EV3.12 | Display base for an imported derived value without a snapshot ancestor | Chapters 08 and 10; partition clarification if required |
| EV4.11 | Recording capture point, refusals and unresolved outcomes | Chapter 05 and the integrator's receipt contract |
| EV4.14 | Historical-only playback versus explicitly checked revision rebinding | Chapters 05 and 08 |
| EV5.9 | BS3's competence gate and validity after a principle-record change | Joint setting of LQ2 with LQ14 |
| EV5.12 | Human-test rubrics, comprehension and transfer criteria | Owner, with the mathematician; chapter 11 for LX-8 |
| EV5.14 | Term additions, scope corrections and their proposed learning levels | Mathematician and the owner's learning path; LQ3 for evidence words |

Other dependencies and differences in the sources remain visible:

| Source or dependency | Difference or unfinished obligation | Affected clauses |
| --- | --- | --- |
| H-08 versus DT-M1 | H-08 retains the draft contexts and commit-only repeat validation. DT-M1 fixes routing classes, the repeat allowlist, the single undo order and the missing workspace-undo path. EN4's final table and builder checks must carry those changes, including removal of undo's `not_busy` gate. | EV4.1 to EV4.8, EV4.15 |
| H-08 availability versus the overlay | H-08 says the command layer asks the engine; KN5.7 also needs the overlay's static table. KN5.8 leaves their protocol for the architecture plan check. The environment consumes the resulting table rather than deciding availability. | EV1.8, EV4.1, EV4.2 |
| H-08's renderer rule versus derived-state display | The older table forbids drawing an uncommitted state. The partition and KN2.12 require explicitly identified derived display plans. Chapters 08 and 10 and the renderer handoff must distinguish these plans from adoption of the session state. | EV3.3 to EV3.5, EV3.12, EV3.13 |
| Solving workflow's historical key sets versus DT-M1 | The workflow records 0.4's orbit sets as historical requirements; DT-M1 selects one default map, explicit applied anchors and solver rebinds for 1.0. Its settled rule governs bindings here. | EV4.7, EV4.8, EV5.6 |
| Glossary versus DT-M1 and KN1 | The current twist entry equates a twist with a generator. Cap, label, piece, generator, orbit, orientation, protection-prefix and complete-effect entries need their puzzle and level scope; label-fixed centres can rotate in pose. Section 6 proposes corrections without changing the glossary. | EV2.2, EV3.9, EV4.9, EV5.14 |
| Research block, frame and unit vocabulary | The tail's block and frame-part unit differ from a solver-declared block and an ordered sticker frame; card 6 uses unit for pieces with one exact pose. Proposed entries qualify those meanings instead of merging them. The mathematician must settle their language names. | EV5.10, EV5.14 |
| KN2.14 versus the display base requirement | An imported derived value need not have the snapshot ancestry from which EV3's required base revision would come. The kernel remains unchanged; EV3.12 carries the missing display contract. | EV3.4, EV3.12 |
| DT-L1 rounds 2 and 3 | Round 3 answered R2-E1 to R2-E4 (overview section 8). Every use here names its card, and EV3.8, EV3.9, EV5.3 and EV5.4 carry round 3's corrections. | EV2.5, EV2.6, EV2.10, EV3.8, EV3.9, EV4.9, EV4.16, EV5.3, EV5.4, EV5.10, EV5.13, EV5.14 |
| EN3 and EN4 | Relationship data joins EN3's pending view-model fields; overlay registration, query rows, lanes, receipt and availability transport, bindings and undo require EN4's final H-08. | EV1.8, EV2.8, EV3.8, EV4.1 to EV4.15 |
| Other chapters | Chapter 02 owns checker and IR services; 03 principle records; 05 bytes and versioning; 06 evidence meaning; 07 and 09 policy and grants; 08 lifecycles; 10 display linkage; 13 error spellings. | EV1 to EV5 |
| LX-4, LX-8 and chapter 12 | Editor latency bars, human-test form and phased acceptance remain future requirements, with no result claimed here. | EV1.9, EV5.12; CT-EV1-06, CT-EV5-03, CT-EV5-05 |
| Design line | The owner's 3 November hold still covers the editor's appearance, the strip's design and DT-J2 to DT-J4. This data chapter reopens none of them. | EV1.12, EV3.13, EV5.11 |

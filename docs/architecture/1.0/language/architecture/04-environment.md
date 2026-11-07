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
[EN4 layer interfaces](../../../../progress/1.0/layer-interfaces.md);
[the charter](../../../../progress/1.0/charter-2.0-draft.md), bars E5, E6, M1 to M6, D1 to D8 and S1 to S6;
[the owner design decisions of 2 October](../../../../wiki/decisions/owner-decisions-2026-10-02-design.md);
[the owner decisions of 3 November](../../../../wiki/decisions/owner-decisions-2026-11-03.md);
[the owner decisions of 6 November](../../../../wiki/decisions/owner-decisions-2026-11-06.md);
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
    errors:  LG1:syntax.recovery_gap, LG2:type.mismatch, LG2:type.exactness_violation, LG2:type.exact_required,
             LG3:effect.undeclared, EV1:evidence.obligation_unmet, KN1:input.identity_mismatch
  ```

- **EV1.3 [D]** Completion uses the recovered scope, the chapter 02 type and effect information, and the selected
  module context. Each candidate carries the source range it would replace and the relevant type, effect, identity
  and documentation reference (partition EV1, LG2, LG3). Suggesting an entry neither grants its capabilities nor makes
  its command available. A draft's missing context remains explicit instead of borrowing the active tool's context.

- **EV1.4 [D]** Diagnostics distinguish ordinary type errors, effect errors, exactness violations, unmet evidence
  obligations and model or menu identity mismatches (partition EV1, default 5). Each diagnostic has a stable typed
  code, a source range and the relevant typed context, except for ERR.10's encoding-error diagnostic, whose `span`
  is absent. LG2.22 owns exactness diagnostic precedence and the delimited meaning of `LG2:type.exact_required`.
  `diagnose` forwards `LG2:type.exactness_violation` unchanged for ApproxBool
  misuse and display-dependent control/result escape, including the direct condition and helper-return cases.
  `EV1:evidence.obligation_unmet` identifies a required claim or check that is absent. An open claim
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
  dispatch path. Whether a linked service meets the latency bars is LX-4 in chapter 11. Its first-pass headless
  function timings are leads only (`work/experiments/language-lx/lx4-editor/RESULT.md`), with no presented-frame
  measurement or gate result. Decisive foreground runs wait for the owner's window (owner, 6 November 2026,
  decision 15), and this chapter supplies no substitute timing target.

- **EV1.10 [D]** The proposed editor protocol consumes `SourceRange` from [the working handoff](../closure/HANDOFF.md),
  H7: absolute UTF-8 byte offsets on scalar boundaries, half-open, bound to the exact source digest. Check
  `0 <= start <= end <= byte_length`; an empty range is an insertion, including at byte_length. The bytes are the
  validated UTF-8/LF source of LG1.3, with no Unicode normalization. Both endpoints must be scalar boundaries.
  A host declares `utf8`, `utf16` or `scalar` when it opens the document. EV1 converts both endpoints by decoding
  those same bytes and counting the declared units from the start, without rounding a position inside a multibyte
  scalar or surrogate pair. Coordinates in a CRLF buffer must first be mapped to the pinned LF document; hashing
  LF bytes and counting CRLF units is refused. A digest mismatch is `EV1:stale.editor_source`; an invalid endpoint,
  undeclared unit or non-bijective conversion is `EV1:input.source_range_invalid`.
  Invalid UTF-8 has no `SourceRange`, including no empty range in an otherwise valid prefix. Its diagnostic uses
  ERR.10's raw input context, outside this conversion protocol; replacement decoding cannot create a valid range.

  For the scalars `a`, U+1F600, U+00E9, LF, `z`, the boundary triples `(utf8, utf16, scalar)` are
  `(0,0,0), (1,1,1), (5,3,2), (7,4,3), (8,5,4), (9,6,5)`. Thus UTF-8 [1,5) selects U+1F600 and converts to UTF-16
  [1,3); [5,5) inserts after it at [3,3); [9,9) inserts at EOF [6,6). UTF-8 offset 2 and UTF-16 offset 2 refuse.
  Chapter 10's `SourceMap` consumes these ranges without a second coordinate convention.

- **EV1.11 [D]** Checking a draft creates a candidate, without changing the active module. The service
  `activate_module(module, expected_active_digest, candidate_digest, check_reference)` is the only switch. It checks
  that the reference covers exactly the candidate's source, manifest, lock closure, editions, execution and checker
  versions and that the host's activation grants still hold. An unchecked or mismatched candidate returns
  `EV1:input.module_not_checked`. The switch atomically compares expected_active_digest (or none for an initial
  install) and installs the whole candidate closure; a changed active digest returns `EV1:stale.activation_base`.
  Every refusal leaves the active version intact. Success returns the previous and new digests; subsequent invocations
  bind the new one and still check their own grants. An invocation racing activation binds wholly before or after
  that switch, never to a mixture. Save, rename and a successful diagnostic request do not perform this switch.

  A run already bound to A continues on A after activation of B. Its closure, source ranges, debugger and result stay
  A; B does not inherit its evidence. The host retains A's required immutable bytes while the run can resume. Module
  activation alone does not cancel it. Revision replacement can stop revision-bound runs, and revoked grants can
  stop a run under KN7; a digest-pinned research run follows EV2.10. Restarting or rerunning creates a new invocation;
  it is not hot replacement of A's stack.

- **EV1.12 [B]** A diagnostic or refused editor action provides what happened, its reason and an available next action
  as typed data for the host (charter E6). Its code decides handling; prose is never parsed to decide a continuation
  (H-08 typed errors; partition default 5). Final messages and their presentation wait for the design line.

- **EV1.13 [O]** Whether the host requests EV1.11 automatically after a check or only through an explicit user action
  is an interaction choice for the owner and the resumed design line. Neither trigger is selected here. Both call
  the same atomic service and preserve old runs; widgets, prompts and editor appearance remain open. LQ9 changes
  where the service is hosted, not its version-binding contract.

## 2. EV2: debugger, replay and profiler

- **EV2.1 [D]** A debugging session names checked source and module digests, the chapter 02 IR version and the pinned
  input it inspects (partition EV2, INV-10). Stepping advances a program's evaluation or a recorded trace. It never
  advances the authoritative session, commits a preview, executes an undo or grants a capability. A debugger exposes no
  write path to the engine or store (INV-2, INV-3). An inspected proposal is data; stepping over it cannot obtain the
  shell's user-action token.

  ```text
  operation debug_step(run, expected_stop, request_id) -> StepResult
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
  letters (KN2.7). Chapter 06 owns the graph's evidence meaning; its presentation adds no evidence edge. A computation
  edge says where a value came from, and that is not support. The view shows computation edges and evidence edges
  separately, counts only evidence edges, and shows the edition with the word (EVD.14).

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
  through KN4.3 and LI.14 query rows (INV-2). Invocation recovery uses `outcome(command, request)` under KN5.23 and
  LI.30, without invoking or retrying the command. Opening an old snapshot changes only what the debugger inspects. It
  does not select an active journal line, reset the session, apply an inverse word or write session files. A revision
  that is unavailable reports `KN2:stale.revision_gone`; the debugger never silently opens the current revision instead
  (KN0.6). Chapter 08 defines snapshot and branch lifecycles.

- **EV2.9 [O]** Replay of a run with workspace, file or preview effects needs a chapter 09 replay contract agreed with
  chapter 05. Options are inspection of recorded effects without re-executing them, or evaluation against isolated
  replay sinks whose inputs are captured and digest-pinned. No source settles those sinks or which external inputs a
  record must capture. Both options keep the debugger read-only, grant no ambient files or clock, and distinguish
  trace inspection from a complete deterministic re-execution. Unsupported effects report
  `EV2:capability.debug_write_forbidden` instead of touching the live workspace.

- **EV2.10 [B]** A research run pinned by digest may finish while the solver commits or undoes (owner, 6 November 2026,
  LQ4). Its debugger and result keep the old input; reaching the new revision requires a re-run or exact transport
  justified by a lemma (KN7.5; DT-L1 R2-E1). Moving the debugger's cursor does not make an old result current.

- **EV2.11 [D]** Word references consume H7's zero-based half-open positions in the expanded literal word. They name
  its literal digest, identity block, alphabet and exact expanded length, with `0 <= start <= end <= length`.
  A letter at i is [i,i+1); [length,length) is the completed cursor. These are letter positions, never source bytes,
  chunks or equality-key tokens. A compressed reference also carries a path to the particular expansion occurrence
  and an offset within that occurrence. The path distinguishes child occurrences, repeat iterations and inverse
  traversals in the carrier's expansion order. Reusing one named part twice therefore gives two paths. Offsets count
  emitted chronological letters, including the emitted order of an inverse part. The resolver checks the path and
  offset against the absolute expanded position and literal digest, without materializing the whole word. With
  `repeat([a,b],3)`, iteration 1, offset 1 cites b at [3,4); it does not cite the first b at [1,2).
  Flat H10 storage has the absolute range directly; named annotations do not change it. Missing expansion content
  gives `EV2:input.replay_dependency_missing`; an inconsistent path or endpoint gives `EV2:input.word_range_invalid`.
  Chapter 05 owns the carrier and stored bytes, and KN6.4 supplies the checker trace rather than a fabricated path.

- **EV2.12 [D]** A step consumes one next semantic operation under the run's versioned LG4/DET.20 contract, stopping
  at its next declared boundary. The service consumes that contract's position; it defines no second step counter or
  tariff. For ordinary calls the next boundary is the callee's entry; return reaches the caller's continuation.
  A kernel call is opaque to language stepping: inspect its call site and KN6 trace references, never its native
  instructions or partially constructed result. DET.23 still owns admission and cost. The request names the current
  stop and a run-local request ID. A duplicate ID returns its latest result without another advance; a different ID
  with a stale stop returns `EV2:stale.debug_stop`. Only one advance is in flight per run.
  A different ID naming the current awaiting stop joins that call and returns its waiting/completed result without
  another advance. Reusing an ID with a different run or expected stop is `EV2:stale.debug_stop`.

  ```text
  StepResult = { run, module_digest, execution_contract, position, stop_id, values,
                 source: mapped(List<SourceRange>) | unavailable(source_map_missing | source_bytes_missing),
                 state: stopped | waiting(call_id) | terminal(run_outcome) | unavailable(step_mapping_missing) }
  ```

  `position` and any deterministic cut-off are the actual execution-contract positions. `values` contains only
  completed immutable locals; inspection runs no getter or program callback. `source` names the executed module's
  bytes through EV1.10. A missing source map permits inspection of the actual position with an explicit unavailable
  source, never a range in the current draft. A missing executor-to-contract step mapping gives
  `unavailable(step_mapping_missing)` before advancing; that route fails stepping admission. A budget refusal,
  cancellation, safety stop or runtime failure is `terminal` with the actual KN7 outcome and coverage, not a
  successful step or an empty negative. Repeating a terminal request changes nothing.

  On dispatch of a suspended kernel call k7, return `waiting(call_id)` at k7's awaiting boundary with the call-site
  source and no return value. Repeating the request while pending returns the same waiting call and charges nothing
  further. The one in-flight request remains attached to that call. When it finishes, that request yields
  `stopped` at continuation k8 with its completed result, or `terminal` if the run stopped there. It never dispatches
  k7 again. The host reports completion without executing k8; a new request is needed to advance k8. Cancellation
  remains bounded by the kernel contract's safe boundaries and reports the real completed coverage.

  RT1 exposes these boundaries in its evaluator. RT2 and RT3 preserve them through a checked host/VM map, including
  operations optimized away or fused at the native level. RT4 with IR uses the same map; non-IR RT4 owes equivalent
  declared host boundaries and mappings under RTE.6. Host bytecode instructions, VM fuel and source lines cannot stand
  in for semantic steps. This is a route admission requirement, not evidence that a candidate already supplies it.

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
  the stage safety contract; this chapter defines no renderer feature or shader.

- **EV3.3 [B]** Output carries the base `RevisionRef` it read, and remains identified as stale after that base is
  replaced until recomputation supplies current output (charter D2; partition 8.3). The shell adopts the engine's new
  revision without waiting for a program. Its output state distinguishes empty, loading, error, stale and busy without
  passing stale facts off as current ones. A cancelled query result is not a retained current answer (KN7.3, DT-M1
  Q5 (a)); a stale program output is identified with its old base rather than attached to the new revision.

- **EV3.4 [D]** A derived state appears only through a chapter 10 display plan naming the kernel derivation and its
  base revision (INV-1; partition EV3, 8.3; DT-L1 E2). A plan names two things: its own base, the revision the display
  computation read, and the derived value's lineage (ONE.21). Neither is a new session revision assigned to the derived
  value. The plan retains the derived state's own digest, level and reachability reference (DT-L1 R2-E2). Neither the
  shell nor the renderer can adopt it as the authoritative state (KN2.11 to KN2.14).

- **EV3.5 [D]** A pick returns the `DisplayInstance` of the adopted plan. Chapter 10's `SourceMap` resolves it to the
  canonical ID at its model and level and, for a language-produced instance, to the producing source location.
  Check RPI.24's pick bindings first, including installed plan/map and view generations, under RPI.25 and RM.10.
  A mismatch returns `EV3:stale.display_pick` before any source action, even if the source also differs. Only after
  those checks pass can applying a source range to a different producing digest return `EV3:stale.source_link`.
  A newer editable draft alone does not invalidate a pick: navigation opens the producing version (EV3.6).
  Missing source stays explicit under RM.10; the host never substitutes a nearby declaration. In RM.30's
  generation-change case, an instance from generation 7 consumed after generation 8 returns `EV3:stale.display_pick`
  even with unchanged source. With bindings unchanged, applying its range from `a😀b` to `a😀c` returns
  `EV3:stale.source_link`.
  The chain uses instance IDs without inverting a chart (partition 8.5; KN0.6, INV-6).

  ```text
  operation follow_pick(instance: DisplayInstance, map: SourceMap, dependencies: RPI.24 pick dependencies)
      -> { canonical_id, level, source_location?, producing_module }
    effect:  pure
    errors:  EV3:stale.display_pick, EV3:stale.source_link, KN1:input.identity_mismatch
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
  and any producing source location. The statement reference travels in `claims` of `ViewNode` (RPI.5, RPI.65). Roles
  include destination, current occupant, wanted piece, buffers, frame and residual. Chapter 06 defines statement
  identity and evidence; chapter 02 defines the types. These fields can join EN3's view model now, without deciding
  the strip's design. Three scopes hold (DT-L1 R3-M4). The role "frame" takes a value in the frame group under chosen
  reference frames, at label level, on the lattice; changing the frames conjugates the value. Unqualified, "frame"
  means reference stickers, and every other use carries a qualifier: the frame domain and frame part of the jumbling
  research, frame bits and frame sums, the C₃ and C₂ components of restoration theory that no sticker shows, and A3's
  F_f. The strip must tell them apart, and the word it uses for each is a preference. "Current occupant" is partial:
  off the lattice it has no value.

- **EV3.9 [D]** A relation with no position off the lattice returns the distinct value "none: off the lattice";
  insufficient knowledge returns `unknown(reason)` (KN0.4, KN0.5; DT-L1 R2-E4). It never returns an old occupant, slot
  zero or false for either case. A partially defined relation retains its coverage and missing part. Relations do not
  lift labels into poses, and off the lattice they read poses, classes or regions, never labels (KN1.9, KN1.10, KN1.15).

- **EV3.10 [B]** Relations and other program view data are recomputed for changed inputs at a named revision, never
  per frame (R-19; partition EV3). Frame drawing reuses those results. Latest-wins evaluation skips superseded
  revisions (KN7.2, partition LG4); it cannot queue every missed revision or delay a commit. Camera changes do not
  trigger exact state analysis unless a declared analysis input actually changed. Per-frame chart evaluation remains
  chapter 10's display operation, outside this relation computation.

- **EV3.11 [B]** Further views are allowed in every profile (owner, 6 November 2026, LQ6). Charter D8's limit to
  Global and Local is lifted (owner, 6 November 2026, decision 1). Look and placement stay with the design line
  under the owner's 3 November hold.

- **EV3.12 [D]** A value imported under KN2.14 has no session ancestor. Its plan states lineage `imported` with the
  import's digest, and the plan's own base is an explicit comparison base with no claim that it produced the value
  (ONE.21, RPI.7). Nothing fabricates a derivation or a revision. This reading of partition EV3 is the integrator's.

- **EV3.13 [D]** User stages produce display output under chapter 10's contract, RPI.68 to RPI.80 (owner,
  6 November 2026, LQ7). Further views keep canonical identity, base revisions and source linkage under RPI.81 to
  RPI.83; no view gains a program commit path. Handoff row H-11 gives the renderer line RPI.15's display-plan
  primitives, the user-stage safety contract and the free-view workload before the renderer feature-list freeze
  (owner, 6 November 2026, LQ20). The linkage and the one-state rule remain (INV-1, INV-8; partition 8.5);
  layout, appearance, motion and interface wording wait under the owner's 3 November design-line hold.

## 4. EV4: commands, bindings, macros and recording

- **EV4.1 [D]** A language-defined command registers a KN5.3 overlay row with the H-08 row fields and chapter 02's
  checked module reference. It carries the static verdict, declared capabilities and kernel rows it may invoke;
  optional rows are identified as in KN5.4. Its kind is `view`, `workspace`, `query`, or `file` with a grant. It cannot
  register a `commit` or `app` row (KN5.3 to KN5.5). The engine receives data and computes availability under KN5.7;
  it never calls language code to ask whether the row is available (default 17).

- **EV4.2 [B]** The palette includes every command, including a language overlay row (charter E5, DT-M1 Outcome C3).
  An unavailable entry keeps its place in the matches with its typed reason. Search uses labels, glossary terms,
  command IDs and key names; it exposes a command's binding before disclosure. Its availability input is KN5.6's table,
  read per revision and at each new generation (KN5.25), with no engine call for each search keystroke. Palette
  invocation uses the same gates and preview rule as a key. Searching and showing a command never grants permission to
  run it.

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
  no bound. The macro comparison's word-equivalence result uses LG2.18's versioned `WordEqualityKey`; a separately
  requested trace comparison uses LB1.10's `TraceEqualityKey` and reports its independence contract. Neither is whole
  macro-record equality or a decision of arbitrary partial-map equality. Unequal compatible keys mean only that the
  selected rule did not recognize equivalence; incompatible versions refuse as `LB1:input.key_contract_mismatch`.
  Protection and execution use literal prefixes
  (KN2.7, default 10; DT-L1 R2-E2).
  Flat storage versus references belongs to chapter 05 with EN3, without losing literal order or provenance.

- **EV4.10 [D]** A `Recording` stores semantic steps: stable command IDs, resolved arguments, canonical IDs with their
  identities, and the revisions on which the steps depend (partition EV4; owner, 6 November 2026, LQ11). Each step
  has the following shape. An overlay step identifies the module digest needed to give its command meaning
  (LG5, INV-10). Pixels, screen coordinates and key labels cannot identify a recorded action (INV-6). A recording
  stores no user-action token, operation preview ID (KN5.9) or old execution permission. Chapter 05 defines its bytes and versioning.

  ```text
  RecordedStep = {
    row:       stable command ID
    arguments: resolved canonical arguments, with no token and no operation preview ID
    word:      WordRef?                       # a word step (EV4.18)
    depends:   List<RevisionRef>              # the revisions the step read
    module:    Digest<source>?                # an overlay step
    action:    ActionNumber?                  # the durable action it took (LI.9); EV4.23 matches undo and redo by it
    command:   CommandId
    request:   Digest<request>
    outcome:   terminal receipt reference, or none
  }
  ```

- **EV4.11 [D]** While recording is on, the command layer appends one step for every resolved invocation, with its
  outcome, refusals included. `recording.*` rows are not captured. Input that never resolved to a row leaves nothing.
  The outcome is filled from the invocation's terminal receipt under KN5.21 and KN5.23; until it is known, the step
  has no terminal reference. The record cannot describe a refused or unresolved command as a committed action,
  and reconstructing it never retries one. EV4.19 governs playback and conversion of these outcomes.

- **EV4.12 [B]** A recording may play view and workspace steps unattended; a commit never plays unattended (owner,
  6 November 2026, LQ11; owner, 6 November 2026, decision 3). The execution gate of INV-3 holds. The three uses
  follow EV4.13's mechanism. Recording is not deterministic run replay: EV2 reproduces computations without playing
  actions into the session.

- **EV4.13 [D]** The three uses are conversion to a word draft or source for one complete preview and one user
  commit, stepwise playback with one user action per commit, and unattended playback after one starting user action,
  stopping for every commit. EV4.17 to EV4.23 define eligibility, outcomes, recovery, undo and conversion.
  Conversion to source names the recorded commands, absolute arguments, module and word digests, and
  revision guards. Generated source passes the same chapter 02 checks and chapter 09 grants as typed source. A word
  assembled from it receives a fresh whole-word preview and protection check; no old receipt or certificate supplies
  permission (INV-3, defaults 14 and 18). New invocations receive new durable identities. Playback that cannot resolve
  a step reports `EV4:input.recording_step_unresolved`, preserving the record for inspection. It never replaces a missing
  canonical ID with a visual pick.

- **EV4.14 [D]** 1.0 has no rebinding operation. Playback re-invokes each step with its recorded canonical arguments
  against the current revision, after checking model identity and input digests. A mismatch refuses playback before
  it starts, or stops it during playback, with `EV4:input.recording_identity_mismatch`. An unattended step whose
  arguments name a revision that is not the current RevisionRef, every field equal (KN0.2), or a filter preview that is
  not fresh, stops with `EV4:stale.recording_step`; a named revision that is still current plays. Recorded revisions remain dependencies;
  they are never silently retargeted, and a recorded undo or redo target follows EV4.18's action map. Manual
  continuation uses EV4.18's fresh guards and solver action.

- **EV4.15 [B]** Puzzle and workspace actions share DT-M1 Q4 (a)'s durable undo order. An undo carries the action its
  hint named and is refused if another action superseded it; a text editor keeps its own text undo. Commit and undo
  go ahead during revision-bound jobs, cancelling those jobs as in DT-M1 Q5 (a). The debugger's journal cursor is
  outside this action order because it writes nothing. KN5.20 and LI.26 supply the shared `session.undo` entry;
  LI.9 supplies its action-order guard. Their implementation and conformance remain owed.

- **EV4.16 [D]** Saving a word, recording it or turning it into source preserves the chapter 07 provenance reference;
  it cannot make research output into a solving preview or change an imported word into an own word by format
  conversion (BM2; DT-L1 R2-E3, fix-first item 18). Flow-label detail versus capability separation stays LQ21. Under
  the interim policy of partition section 11 no solver-tool solving preview is enabled before the joint setting.
  Built-in tools retain the 2 October boundary and the 17 October named-cap repair-list exception (INV-11). Showing
  a result or recording a solver's pick grants no new declassifier.

- **EV4.17 [B]** Recorded view and workspace steps may play unattended (owner, 6 November 2026, decision 3).
  EV4.18 and EV4.22 give the integrator's row and lane eligibility rules, including the query default beyond the
  owner's words. That default supplies no effect permission.

- **EV4.18 [D]** Playback stops and waits for one user action at commit-lane rows, rows with `confirm` or
  `file_check`, `file` and `app` rows, exclusive-lane rows and job rows. `session.checkpoint.save` stays manual:
  its lane is `commit` even though its durable action tag is `workspace` (LI.9). For a user step the shell fills
  the current `revision`, the solver's `token`, a fresh preview where required and, for undo and redo, a current
  direction-specific hint (LI.9, LI.26). A recorded `operation.execute` commit is a word step: its stored word
  gets a fresh `operation.preview`, and the solver's one action commits it. A recorded preview ID is never reused.
  The live one-letter word steps of EV4.23 keep ONE.78's live-twist rule.

  Before an undo or redo user step the shell reads `kernel.undo_hint`. If the hint's target is not the played
  action that PS1.44's `actions` map gives for the recorded `hint.target`, or the map has no entry for that target,
  playback stops with `EV4:stale.recording_step` and undoes nothing. A redo's target is the original action number
  (LI.9), so the same map serves redo.

- **EV4.19 [D]** Historical outcomes control whether a step can be played or converted:

  | Recorded outcome | Playback | Conversion |
  | --- | --- | --- |
  | `completed` | Plays if eligible under EV4.22; otherwise a user step | By EV4.23 |
  | `committed` | User step | By EV4.23 |
  | `refused`, `failed`, `cancelled` | Stays visible, never plays, is passed over | Left out |
  | `accepted`, `running`, no terminal record, `unknown_outcome` | Stops at this step with `EV4:input.recording_step_outcome_unknown` | Stops at this step with the same code |

  New child outcomes during playback follow this table. Each advance is reconciled at most once under EV4.20.

  | New child outcome | Unattended step | User step |
  | --- | --- | --- |
  | `accepted`, `running` | Pending; no advance | Pending; no advance |
  | `completed` | Advances | Advances |
  | `committed` | Not an unattended step under EV4.18; no automatic advance | Advances |
  | `refused`, `failed`, `cancelled`, `unknown_outcome` | Stops with that typed outcome; no automatic retry | Stops with that typed outcome; no automatic retry |

  A child's code keeps its identity (ERR.7). A refusal or lost outcome never becomes a successful empty step.

- **EV4.20 [D]** PS1.44's `PlaybackState` is a durable workspace record outside the undo order. Its `step` is the
  next recorded step. The command layer writes `pending` before each dispatch. Recovery first calls
  `kernel.outcome(command, request)` under KN5.23 and reconciles the receipt, advancing at most once. An
  `unknown_outcome` stops playback and never makes a fresh retry. The cursor, cleared pending entry, any new
  `actions` entry and `last` receipt are saved together when the step advances.

  A manual step goes through the shell's step action: the shell chooses the command ID (KN5.21), the command layer
  writes `pending`, then the ordinary row is submitted with the solver's token. Its receipt reconciles like a
  played step. An ordinary invocation outside that action does not move the cursor. The solver moves the cursor
  explicitly with `recording.play {recording, from}`, under EV4.22's table, after pending reconciliation.

- **EV4.21 [D]** Each played step keeps its row's undo classification. An undoable workspace step is its own
  action. An `undo: none` row, such as `camera.rotate` or `session.timer.start`, gives a receipt and no invented
  undo target. A query is read-only. The cursor is never an undo target. LI.17 stages one overlay invocation as
  one action; playback is a sequence of commands, so its completed actions remain separate, including if a later
  step stops or fails.

- **EV4.22 [D]** Capture and playback sit at the command layer: an overlay program cannot observe other invocations
  and has no generic dispatch. `recording.start {name}` and `recording.stop {recording}` control capture.
  Start while recording, stop with no recording, or a play or `from` that the table below does not allow is refused
  with `EV4:input.recording_state`. `recording.play {recording, from?}` runs from the durable cursor, or the explicitly
  recorded `from`, to the next user step or the end, and finishes. `job.stop` halts its job. Each played step is a
  new invocation with a new durable identity (EV4.13) and its own receipt. Playback never holds the commit token.
  RPI.14 stays true: playback is solver-invoked, not a view effect.

  The eligibility table applies after EV4.19 and subject to EV4.14. Every ordinary gate still applies. In particular,
  `filter.apply` plays only while its fresh filter preview holds; eligibility never recreates or retargets one.

  | Row and lane | Unattended eligibility |
  | --- | --- |
  | Kind `view` or `workspace`, lane `view` or `none`, no `confirm` or `file_check` gate, no job | Eligible under the owner's rule (EV4.17), with all remaining gates checked |
  | Kind `query`, lane `none`, no job and no `confirm` or `file_check` gate | Eligible as the integrator's default: it changes nothing |
  | Commit lane, including `session.checkpoint.save`; `confirm` or `file_check`; `file` or `app`; exclusive lane; any job | User step under EV4.18 |
  | Any other row/lane combination | User step; no unattended permission |

  `recording.play` reads PS1.44's state for that recording after pending reconciliation. `from` = f needs
  0 ≤ f < the number of steps. Every refusal here is `EV4:input.recording_state`.

  | State when `recording.play` runs | No `from` | `from` = f |
  | --- | --- | --- |
  | No state for this recording | New playback at step 0 | New playback at f |
  | A child still `accepted` or `running`, or status `running` | Refused | Refused |
  | `waiting(step)`, or `stopped` with a code or with `refused`, `failed` or `cancelled` | Same playback, continues at `step` | f ≥ `step`: same playback, cursor to f, map kept. f < `step`: new playback at f, empty map |
  | `stopped(outcome(unknown_outcome))` | Refused | New playback at f, empty map |
  | `finished` | Refused | New playback at f, empty map |

  A new playback replaces the recording's state record; the kernel keeps every receipt (KN5.23). Writing an unknown
  outcome clears `pending` and leaves that receipt in `last`. A skipped step has no map entry, so a later undo of it
  stops under EV4.18. For example, W recorded as action 10 plays as 100. `from = 0` opens a new playback with an empty
  map, W plays as 101, the undo step's hint targets 101 and undoes it, and 100 stays applied.

- **EV4.23 [D]** `recording.convert {recording, target: draft | source}` first applies EV4.19, then checks every
  remaining step against the target. It writes nothing unless every step passes. A step with no form in that target
  refuses the whole conversion with `EV4:input.recording_not_convertible`, naming the step and target. A word step
  is a `committed` `operation.execute` (its stored word), `twist.apply` or `jumble.twist.menu` (its one letter).
  The step's `action: ActionNumber?` (LI.9) matches undo and redo to their recorded `hint.target`.

  | Step | Target `draft` | Target `source` |
  | --- | --- | --- |
  | Word step | Adds its letters to the word | Adds its letters to the word |
  | `session.undo` or `session.redo` targeting a word step of this recording | Removes or restores that step, keeping its recorded place | The same |
  | `session.undo` or `session.redo` targeting a non-word step of this recording | Contributes nothing | Refuses |
  | `session.undo` or `session.redo` targeting an action outside this recording | Refuses | Refuses |
  | `session.checkpoint.save` | Contributes nothing: its action tag is `workspace` (LI.9) | Refuses |
  | Any other `commit` row | Refuses: the position changed in a way no letter states | Refuses |
  | Completed `view`, `query` or `workspace` step | Contributes nothing, listed in the conversion report | A call in recorded order if an overlay row may invoke it (KN5.4) and it has no `confirm` or `file_check` gate; otherwise refuses |
  | Completed `file` or `app` step | Contributes nothing, listed in the report | Refuses |

  Workspace staging adds no letters. `macro.add(W)` followed by `operation.execute(W)` gives W exactly once.
  Target `draft` refuses an empty word with the same code, so `camera.rotate` alone refuses. Success creates one
  operation draft whose Macro phase holds the literal word, as one undoable workspace action. Its provenance block
  lists each contributing word's block in `parts` (BND.24), so BND.84 reads them at preview. Target `source` keeps
  EV4.13's fresh whole-word preview. A program-produced word carries `producer: program(...)` and is refused at
  preview in a solving session under BND.84. A solver who wants their own word back there uses target `draft`.

## 5. EV5: docs, explanations and the learning path

- **EV5.1 [B]** The mathematician oversees this component (DT-L1 owner input; partition EV5). Everything here is a
  draft for that review.

- **EV5.16 [D]** The means of understanding are a deliverable (INV-12). The owner's 3 November principle presumes that a
  solver understands the principle of their tool. Nothing the software records, alone or together, establishes that a
  person understands a principle, and none of it decides the joint boundary.

- **EV5.2 [D]** Reference documentation is generated from source declarations and their checked metadata, with the
  source and module digests, edition, types, effects, capabilities, supported identities and documentation references
  needed to interpret each entry (partition EV5, LG5). An entry links to its declaration and evidence rather than
  keeping a second handwritten signature. A broken draft's reference output remains distinguished from the active
  tool's documentation (EV1.7). Every tool above the kernel is written in the language (owner, 6 November 2026,
  LQ5), with INV-12's viewable source and principle record.

- **EV5.3 [D]** A tool explanation consumes the `PrincipleRecord` defined in chapter 03 (INV-12). It exposes the exact
  function and domain, the meaning of an empty or negative answer, the supporting lemma or certificate with its
  evidence, a small instance on the real geometry whose steps a person can check from named givens, the decomposition
  into base parts, and the parts a person writes by hand (LB6.2): the relation between decomposition and lemma steps, the
  reason, a failure example, what an answer does not mean, the givens and steps of the derivation, the prerequisites and
  the leaf specifications. The word of a cited statement is the word of that statement only and never reads as the word
  of the shown answer. A completed empty repair list concerns the named cap and the admitted repair shape; it never
  asserts that no solving step exists (DT-J1 Outcome, Out by the filter). A cut-off job has no completed empty answer
  (KN7.4, KN7.11). This section supplies the means of understanding. It cannot certify understanding (DT-L1 R3-M4,
  BND.72).

- **EV5.4 [D]** Explanation links use EV2's lazy provenance expansion down to primitives and stage-qualified,
  source-commit-pinned lemmas. An imported result traces to its stored bytes and source label, and the trace says that
  its producer is absent. A long word is a digest and positions until expanded. The decomposition is generated from the
  checked IR before executor lowering, or RTE.80's checked host decomposition for a non-IR RT4 tool, from its
  transitive call graph and kernel calls, and the worked instance has a matching headless
  fixture (partition INV-12, LG4; DT-L1 M6, R2-E4). A generated decomposition gives structure, not reasons: the reasons
  are the hand-written parts of LB6.2 (DT-L1 R3-M4). A fixture pass is source evidence for that instance. It does not
  prove a universal lemma or record a human comprehension result. KN6 trace links use the checker's own output.

- **EV5.5 [D]** A fork inherits no evidence (default 18). An explanation of a fork retains links to cited statements
  and their scope, but it cannot label the changed module as certified because its parent was checked. Reusing an
  unchanged lemma needs the chapter 06 identity and scope rules; executing a changed tool still needs checking and
  current grants. Author class, evidence, competence and permission stay separate (INV-5, BM2; DT-L1 R2-E4).

- **EV5.6 [D]** The language learning path follows partition EV5: use tools and read their results; change a
  parameter; save layouts and bindings; compose recipes and macros; read a principle record and state your tool's
  principle; write tools; maintain modules and evidence. It builds on the orbit-first solving workflow and the
  glossary's L1 to L6 rather than introducing another numbered key scale (charter D7, M6; DT-M1 Q9 (a)). Each step
  names its prerequisite concepts, command routes and fixture task. The table below is the mathematician's proposal
  (DT-L1 R4-M44 to R4-M48). Every cited H-08 ID exists in the command table, but those rows are draft (`pending-2.2`).
  No fixture task exists yet, so every fixture is owed and the candidates are proposals. Placing conjugate and
  commutator at step 4 is the mathematician's inference. Proposed term levels are listed in section 6; they are not
  new learning gates or an approved lesson order.

| Step | Prerequisite concepts | Command routes | Fixture |
| --- | --- | --- | --- |
| 1 Use tools and read results | Command, Availability, Revision, Base revision, Stale; Claim, Scope, Covered set, Basis, Check, evidence words, Standing, Threshold, Premise; every, some and negation (Statement form, Negation pair), Witness; empty, not found, not run (EVD.33); Unknown against None: off the lattice; Typed partial result, Verdict; Job, Budget, Job result; Reachability claim; Evidence graph on request; record parts `specification` and `not meant` | H-08 `command.find`, `object.inspect`, `view.findings`, `view.residuals`, `operation.check`, `job.stop`. `invoke` on an overlay row (chapter 01), `word` (chapter 06), `follow_pick` (EV3). Explanation and provenance view: owed | Owed. Candidate: read E3's 7302 and 7303 records (chapter 06, section 9) |
| 2 Change a parameter | Editable parameter, new version, Fork, "a fork inherits no evidence", Diagnostic, Typed error | `edit_declared_part` (EV3), `diagnose` (EV1). Workspace command that persists it, host fork operation: owed | Owed |
| 3 Save layouts and bindings | Layout, Workspace record, Binding with absolute arguments, Routing class, Preference, workspace undo | H-08 `keymap.edit`, `file.keymap.export`, `file.keymap.check`, `file.keymap.apply`, `grip.capture`. `edit_declared_part`. Save layout: owed | Owed |
| 4 Compose recipes and macros | Word, Literal word, Equality key, Cost alphabet, Setup, Star, conjugate and commutator, Recipe, Macro, Macro base, Proposal, Preview, User-action token, Protection, Net and Strict, Complete effect, assistance provenance, Recording, fresh preview on reuse (LB7.2) | H-08 `macro.base.open`, `macro.select`, `macro.add`, `macro.save`, `macro.variant.create`, `macro.compare`, `operation.new`, `operation.phase.edit`, `operation.reuse`, `operation.check`, `operation.preview`, `operation.execute`, `worksheet.save`, `worksheet.load`. `propose` (chapter 01), `word_key` (chapter 02). `recording.start`, `recording.stop`, `recording.play`, `recording.convert` (EV4) | Owed |
| 5 Read a record, state your principle | Principle, Principle record, Hypothesis, Lemma, Theorem as a proof, Invariant, Counterexample, mixed statements, Decomposition, Base part, Step map, Leaf specification, Worked instance, Hand derivation, Fixture, a domain inside its lemma's scope | `hover`, `definition` (EV1). Reading a record: owed. Stating your own: owed; no data route exists, and EV5.9 leaves the map to the joint setting | Owed. Candidate: the floor instance in LB6.2 |
| 6 Write tools | Module, Module manifest, Lockfile, language edition; exact against approximate; Identity level typing; Snapshot, State reference, Derived state; partial results and verdicts in code; budgets, jobs, Cost profile; Effect, Capability declaration, Profile, Author class, Provenance block; view nodes and overlays; boundary: KN2.5, Flow rule, Declassifier, why a preview is refused; stabiliser, coset, quotient, homomorphism, F2, F5, Q(√5), with library explanations under LB6 and EV5.17 | `parse`, `complete`, `diagnose`, `hover`, `definition`, `rename` (EV1), `debug_step`, `inspect_value` (EV2), `open_snapshot`, `project`, `derive`, `propose` (chapter 01), `publish_view` (chapter 10). Create a module: owed | Owed |
| 7 Maintain modules and evidence | Digest, Reproducibility key, Run record, Deterministic replay, Certificate, Check report, Per-claim trace, Grade, Alternative, Check mode, Statement ID, Statement key, Evidence envelope, Exact transport, evidence edition | `replay_run` (EV2), `reproduce` (chapter 09), `check` (chapter 01), `word` (chapter 06), `import_archive`, `export_check_bundle`, `migrate_record` (chapter 05) | Owed |

  Four placements may not be obvious. `specification` and `not meant` are read at step 1, because empty against not
  found cannot be read without them; the whole record is step 5. Basis and Check sit at step 1, because EV5.13 shows
  the check beside the word. Partial results, verdicts, budgets and jobs are read at 1 and written at 6. Mixed
  statements and Invariant sit at 5.

- **EV5.7 [B]** Competence checks use fresh isolated fixture tasks and record independent completion, critical
  mistakes, help use and transfer to a new target. Time is recorded separately from correctness (owner design
  decisions, section 5). Local in-situ metrics support the same learning questions and stay local. Checking a lesson
  never changes the solver's personal session, uploads metrics or equates fast work with correctness. The chapter's
  human conformance tests need the owner's protocol before they can run.

- **EV5.8 [B]** Competence checks guide and do not lock functions (owner, 6 November 2026, LQ14). Keys open by soft
  disclosure; the palette still lists every command, an expert can open all keys, and showing all keys changes no
  disclosure state (charter E5; DT-M1 Outcome K3). A goal flow's needed keys are disclosed by its first use, with their
  concept terms already
  introduced. Capability and evidence checks continue.

- **EV5.9 [O]** If LQ2 selects BS3, the competence check on a principle record gates the capability to prepare solving
  previews for that principle (partition section 11). That is a capability decision, distinct from showing a key or
  listing a command. BS1, BS2 and BS4 add no such competence gate. The owner and integrator must settle this in
  LQ2's joint setting, within LQ14's soft-disclosure rule, including what changing a principle record does to an earlier
  check; no source settles that validity rule.
  No source defines which principle a solver's tool embodies, and any such map belongs to the joint setting. Until then
  no learning result grants a solving-preview capability.

- **EV5.10 [D]** Documentation and library terms use the glossary, checked by script (default 8, charter bar M1).
  A term reference must resolve to one concept at its declared learning level; stored citations name their source
  prefix, including a stage for a lemma or a chapter component for a contract (default 19, DT-L1 M6, R2-E4).
  An unknown term or ambiguous citation fails that check. A formatter or documentation generator cannot repair the
  meaning by guessing a similarly named concept. No such check exists yet: `glossary/build.py` checks duplicates only.
  LP4 builds it, with two rules proposed here: a definition uses only terms at its own level or lower, and a row that
  never reaches the interface carries a "not learner-facing" flag.

- **EV5.11 [B]** The theory book defines the grammar used by code and the relationship strip and gives the proofs or
  cited proofs on which its correctness relies (charter M2, M4, M5; owner design decisions, section 2). Its 1.0 scope
  is what the interface shows. Figures use real geometry with reproducible local production evidence. This chapter's
  data contract provides relation and evidence references; it chooses no appearance, encoding token or interface
  wording. The design line remains on hold under the 3 November decision.

- **EV5.17 [B]** The theory book does not extend to the language's libraries. The theory book, proofs and stage
  research records live in the repository and stay out of the shipped software; the research library and archive
  checkers stay in 1.0 (owner, 6 November 2026, decision 5; owner, 6 November 2026, decision 9).

- **EV5.12 [O]** The owner sets the human-test form and success criteria for comprehension, fixture competence and
  transfer; the mathematician checks their mathematical content. Rubrics and the treatment of help, repeated attempts
  and changed tools are not fixed here. LQ13's LX-8 learnability test feeds syntax choice, and chapter 11 owns its
  protocol dependency. No invented pass rate, timing threshold or claimed participant result fills this gap.

- **EV5.13 [D]** Explanation labels are derived by chapter 06's versioned functions over stored facts (owner,
  6 November 2026, LQ3; owner, 6 November 2026, LQ10; DT-L1 R2-E1). The view exposes statement, scope, basis,
  checks and job outcome separately. Source labels and check references remain inspectable. The environment neither
  sets the evidence order nor supplies its own stronger summary. An explanation shows the check beside the word,
  and says so where no human review or formal proof is recorded. EVD.64 assigns no writer to `formal proof` in 1.0.

## 6. Glossary entries the language adds

The inventory below is source evidence from `terms.json`, the overview, chapter 01 and the partition's named concept
families, with repository research sources named for the proposed jumbling summaries. Existing entries whose meaning
works unchanged are omitted. A row marked "change" corrects or scopes an existing entry. The definitions are proposed
glossary summaries of the owning chapter, not competing contracts.

The View, Chart and Research tier rows reflect the answers of 6 November 2026. The other proposed entries retain
their proposal wording; their settled-question references record the proposal's dependencies. Current contracts are
in the owning chapters.

- **EV5.14 [O]** The mathematician reviewed these additions and scope changes in round 4 against the theory and the
  completed chapters, and this inventory carries the corrections (DT-L1 round 4). That review read the rows owned by
  chapters 02, 05, 09 and 10 for internal consistency only. The proposed L1 to L6 levels use the existing glossary
  scale; their final placement follows the owner's learning path and DT-M1's first-use rule. Jumbling and language
  terms need a first-use order before their levels can be checked, and using the manual or the shelved tutorial for it
  is the owner's call. Evidence words use LQ3's separate fields and chapter 06; any competence capability gate waits
  for LQ2's option set under LQ14's soft disclosure. The inventory carries round 3's corrections to DT-L1 R2-E1
  (evidence), R2-E2 (identity levels), R2-E3 (flow control) and R2-E4 (principles and partial values). The mathematician's
  follow-up applied EV5.10's level rule to every row by hand, and its level moves and new rows are proposals too.
  Some rows are only partly grounded. Principle
  is the integrator's working definition (LB6.2), with no owner text. Base tool has no row, because what counts as one
  is LQ2's question. Base part's meaning is inferred. Solve record is used and never defined. Fixture is generalised
  from its uses. Revision and Statement are the mathematician's wording. Lemma, Theorem, Hypothesis, Invariant and
  Counterexample are standard mathematics the sources use without defining. Foreign load, Clean grip and Tame face
  tool writers only, and the L1 word for a grip is blocked or not. No entry or level is accepted by this inventory,
  and `terms.json` is unchanged.

- **EV5.15 [D]** Each integrated entry needs its own diagram under the existing glossary build rules (charter M6,
  default 8, `glossary/build.py`). This includes a changed concept whose old diagram no longer states its meaning.
  Integration supplies the term fields, editable `diagrams/<id>.mmd`, locally rendered SVG and the manifest's source
  and output digests, tool version, configuration digest and production command. The generated glossary must match
  `terms.json`. These diagrams and the build check are subsequent glossary work; none is created or run by this
  chapter-writing packet.

| Term | Proposed level | One-line definition | Source |
| --- | --- | --- | --- |
| Cap (scope change) | L1 | A world-fixed grip and cut; which pieces lie inside it depends on the state. | Chapter 01 KN1.5; partition KN1 |
| Grip (scope change) | L1 | A world-fixed place, the pole of one cell. Choosing the grip for the next twist changes nothing in the puzzle state. | Chapter 01 KN1.5; state contract section 1 |
| Label (scope change) | L1 | A sticker's identity at a lattice slot; the label array is the state of record for 600-cell-Full. | Chapter 01 KN1; partition LG2 |
| Piece (scope change) | L1 | A rigid labelled object with stickers; slot occupancy is defined only on the lattice in the jumbling puzzle. | Chapter 01 KN1; DT-L1 M3, M6 |
| Generator (scope change) | L1 | One of the 1,200 generators of 600-cell-Full; a negative number is its inverse. Every count says what it counts. | Chapter 01 KN1.5; DT-M1 Terms |
| Twist, 600-cell-Full (change) | L1 | A turn of one grip by a non-identity retained menu rotation, which can be a sequence of generators. | Overview 3.6; DT-M1 Terms |
| Twist, jumbling puzzle (change) | L1 | A turn of one grip by a non-identity active menu element, represented by a pinned menu letter. | Overview 3.6; DT-M1 Terms |
| Retained twist | L1 | A twist in the cap's retained rotation subgroup, with the same rotation when Jumble is on. | DT-M1 Outcome C6, Terms |
| Jumble twist | L1 | An active-menu twist outside that retained subgroup, refused when Jumble is off. | DT-M1 Outcome C6, Terms |
| Orbit (scope change) | L2 | A retained label-position orbit; a centre's fixed labels do not imply a fixed jumbling pose. | Chapter 01 KN1; DT-L1 M3 |
| Stage order (scope change) | L2 | The definition stays. The entry's interface line changes: "next-orbit suggestions" are not admitted under the interim reading. | BND.65; LB2.5 |
| Target (scope change) | L3 | The definition stays. The entry's interface line changes: a "candidate list with reasons" is not admitted under the interim reading. | BND.65; LB2.5 |
| Net and Strict checks (scope change) | L2 | Protection on the whole end state or on every executed literal prefix, at the puzzle's protection level. | Chapter 01 KN2.7; partition default 10 |
| Complete effect (scope change) | L4 | The whole operation's effect at its declared puzzle and level, including every hidden mechanical part. | Chapter 01 KN5.10; charter S4 |
| Orientation (scope change) | L5 | How a piece sits in its position as its labels show it; a piece can be at Home and wrongly oriented. A3's pose-level orientation is a separate quantity that labels do not read, and takes its own name. | Chapter 01 KN1; DT-L1 M3 |
| Frame (scope change) | L4 | Unqualified, an ordered choice of reference stickers that fixes how a piece or block is read. Every other use carries a qualifier. | Theory Lemma 7; DT-L1 R3-M4 |
| Position | L1 | The chamber occupied by a piece, derived from labels and undefined off the lattice. | Chapter 01 KN1.5, KN1.9 |
| Chamber | L1 | One of the 177,120 fixed places a piece can fill. A position names one. | State contract section 2; [card 6, Model](../../../../../research/jumbling/theory/cards/card-6-blocking-structure.md); [theory, cut chambers](../../../../../research/Full_600cell_Puzzle_Theory.tex) |
| Occupant (scope change) | L3 | The piece currently sitting in a position; partial off the lattice, where it has no value. | Chapter 01 KN1.9; EV3.8 |
| Pose | L1 | A piece's exact rotation in the jumbling puzzle, independent of whether its stickers occupy lattice slots. | Chapter 01 KN1.5, KN1.6 |
| Sticker | L1 | The identified region attached to a piece and its home host facet. | Chapter 01 KN1.5 |
| Model identity | L1 | The immutable identity fixing the mechanical objects and interpretation of their data. | Chapter 01 KN1; INV-7 |
| Canonical ID | L1 | An object's fixed identifier in its model, independent of how it is drawn. | Chapter 01 KN1.5; INV-6 |
| Identity block | L2 | Chapter 01's bundle of puzzle, model, menu, menu order and contract references. | Chapter 01 KN0.1 |
| Identity level | L2 | How finely a value describes a state: from each piece's exact pose down to what the stickers show and coarser. Every stored value names its level. | Chapter 01 KN1.7, KN1.8 |
| Identity tower | L6 | The puzzle's levels, a partial order, and the exact projections from finer information to coarser information. | Chapter 01 KN1.7; DT-L1 R2-E2 |
| Sticker class | L6 | The class g_A·S_A of a pose: poses that show every labelled sticker in the same place. Earlier name: visible pose class. | Chapter 01 KN1.7; DT-L1 N1 |
| Region class | L6 | The class g_A·R_A of a pose, which is the piece's posed region. | Chapter 01 KN1.7; DT-L1 N1 |
| Frame group (change: replaces Orientation group) | L5 | The group of permutations of a piece's sticker regions that turns keeping its region can make; trivial, C2, C5, D5 or A5 by orbit. | Chapter 01 KN1.7 (R_A/S_A); Theory Lemma 7; DT-L1 N1 |
| Gauge | L6 | A pair (σ, t): σ permutes the pieces, and each t_A is a rotation carrying A's reference region onto that of σA. Piece A takes the place σA had, so the same regions stay occupied. By Theorem 1 of note N1 (status: theorem, producer) every word stays admissible and the two runs stay related by the same gauge. A gauge is not equality and does not carry reachability. | Chapter 01 KN1.7, KN1.14; DT-L1 N1 |
| Shape | L6 | The multiset of posed regions of a state, forgetting which piece fills which region. On a state without collisions, the set. | Chapter 01 KN1.7; DT-L1 N1 |
| Digest | L2 | A short value computed from exact bytes, used to name them. Digests compare only at the same level. | Chapter 01 KN1.8 |
| Level-qualified digest | L2 | A digest of normalised bytes naming the identity level it covers. | Chapter 01 KN1.8 |
| Menu | L1 | The pinned set of rotations available to a grip in the named puzzle. | Chapter 01 KN1; partition LG2 |
| Menu letter | L1 | A grip and a menu-item number, meaningful only under its named menu and that menu's order. | Chapter 01 KN1.3 |
| Menu identity | L2 | The order-independent identity of the menu's rotations. | Chapter 01 KN1.3 |
| Menu-order digest | L2 | The digest fixing the ordered menu that gives an item index its meaning. | Chapter 01 KN1.3 |
| Seat | L1 | One of the four sets of twelve jumble twists at a cap; hold a seat, then press a twist key. | DT-M1 Outcome Seats; Q7 (a), only if Q7 picks seats |
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
| Invalid | L1 | The outcome of a request that is not an exact element of the active menu; the state is not tested. | Chapter 01 KN2.8 |
| Uncertain | L1 | The distinct unresolved jumbling-twist outcome that permits no state change. | Chapter 01 KN2.8 |
| Lattice | L1 | Configurations where every piece's pose is a rotation symmetry of the 600-cell, so each piece fills a chamber and its stickers sit on slots. | Chapter 01 KN1.9; state contract section 2 (K⁺); [card 6, Model](../../../../../research/jumbling/theory/cards/card-6-blocking-structure.md) |
| Straddler set | L1 | The complete set of pieces straddling a named cap's cut at a named revision. | Chapter 01 KN2.4; DT-J1 fix-first item 3 |
| N | L1 | The total misalignment: how many times a cut cell, on any rotated cut, is covered on one side and not by the same alignment on the other. | Chapter 03; [restoration section 2](../../../../../research/jumbling/theory/restoration.md#2-misalignment-and-the-rim-test) |
| N_c | L1 | A cap's misalignment: In_c + Out_c − 2·A_c. | [Descent manual](../../../../../research/jumbling/manual/02-descent.md) |
| In_c, Out_c, A_c | L1 | At cap c's cut: the cut cells inside pieces cover, those outside pieces cover, and those covered on both sides by one alignment. | [Descent manual](../../../../../research/jumbling/manual/02-descent.md) |
| Floor | L1 | A lower bound on a cap's misalignment: the size of the difference between In_c and Out_c. Twists of that cap alone keep both, so they cannot go below it. | [Descent manual](../../../../../research/jumbling/manual/02-descent.md); [open problems](../../../../../research/jumbling/theory/open-problems.md); BND.10 |
| Cut cell | L1 | A pair of neighbouring chambers on either side of a cap's cut, sharing one face on it. | [Restoration section 2](../../../../../research/jumbling/theory/restoration.md#2-misalignment-and-the-rim-test); [jumbling observation](../../../../progress/1.0/jumbling-observation.md) |
| Seam | L1 | A misaligned cut cell. An off-grip seam lies on a rotated cut that is no cap's cut, so no single twist closes it. | [Restoration section 2](../../../../../research/jumbling/theory/restoration.md#2-misalignment-and-the-rim-test); [jumbling observation](../../../../progress/1.0/jumbling-observation.md) |
| u | L1 | The number of pieces that straddle a cap's cut and so block it. | Chapter 01 KN2.4; chapter 03; [engineering handoff](../../../../progress/1.0/jumbling-handoff/engineering.md) |
| Lowering count D_x(h) | L6 | The number of inside pieces of height at least 4 and branch CLS(x) that item h lowers at grip x. | Chapter 03; [card 7, Terms](../../../../../research/jumbling/theory/cards/card-7-lowering-machine.md) |
| Depth Φ | L6 | Under I_a, the sum over pieces of their left height above two, counting no negative contribution. | Chapter 03; [card 6, Model](../../../../../research/jumbling/theory/cards/card-6-blocking-structure.md) |
| Height | L1 | How far a piece's pose lies from the lattice, on the stated side and menu. A jumble twist changes it by at most 2; a retained twist never does. | Chapter 03; [card 6, Model and T5](../../../../../research/jumbling/theory/cards/card-6-blocking-structure.md), for menu I_a |
| Alignment domain | L6 | A coset γK⁺: pieces whose poses fit one rotated copy of the lattice. | [Card 6, Model](../../../../../research/jumbling/theory/cards/card-6-blocking-structure.md) |
| Free mode | L6, not learner-facing | The simulator's infinite field-valued alphabet in place of a finite menu. On Γ(Λ) left and right height 0 together mean the lattice; in free mode they do not. | State contract; chapter 02 |
| Foreign load | L6 | The number of pieces in grip x's cap of height at least 2 and tree branch other than the grip's class. | [Card 6, Model](../../../../../research/jumbling/theory/cards/card-6-blocking-structure.md); chapter 03 |
| Clean grip | L6 | A grip with no straddlers, so all its twists are admissible. The opposite of blocked. | [Card 6, Model](../../../../../research/jumbling/theory/cards/card-6-blocking-structure.md) |
| Tame | L6 | Of a word: no letter raises Φ. Stronger than every prefix staying at or below the start. | [Card 6, Model](../../../../../research/jumbling/theory/cards/card-6-blocking-structure.md) |
| Domain ID | L6 | The exact identifier of a pose's alignment class, supplied with the height batch at its input revision. | Chapter 01 KN2.4; chapter 03 |
| Height counts | L1 | The per-revision counts of pieces at the declared exact height values. | Chapter 01 KN2.4; DT-J1 Outcome A1 |
| Rim summary | L1 | Per revision, for every cap: N_c, In_c, Out_c, A_c, blocked, and whether it is at its floor. It selects no twist and no cap. | Chapter 01 KN2.4; DT-J1 rule 6 |
| Lifted (working term) | L1 | The lattice pieces a proposed operation takes off the lattice, distinct from its full moving set. | DT-J1 M22, fix-first item 4; chapter 03 |
| Jumbling block | L6 | Pieces that no cut separates and therefore move together, distinct from a solver-declared achievement block. | Chapter 03; [tail section 3.1](../../../../../research/jumbling/manual/03-tail.md) |
| Jumbling frame domain | L6 | A height-two alignment domain in the jumbling tree, distinct from an ordered sticker reference frame. | Chapter 03; [card 6, Model](../../../../../research/jumbling/theory/cards/card-6-blocking-structure.md) |
| Clean-step unit | L6 | A jumbling block or one frame part of a block, as used by the tail's clean-step contract. | Chapter 03; [tail section 3.2](../../../../../research/jumbling/manual/03-tail.md) |
| Pose unit | L6 | A set of pieces with one exact pose, as used in the blocking-structure card. | Chapter 03; [card 6, Terms](../../../../../research/jumbling/theory/cards/card-6-blocking-structure.md) |
| Clean step | L6 | A legal word landing its unit, keeping prior lattice pieces on the lattice and preserving other off pieces outside its free set. | Chapter 03; [tail section 3.2](../../../../../research/jumbling/manual/03-tail.md) |
| Excursion | L6 | The largest rise of the named depth measure along a literal word relative to its starting value. | Chapter 03; [card 7](../../../../../research/jumbling/theory/cards/card-7-lowering-machine.md) |
| ν | L5 | The 45 coordinates read from the labels of a lattice configuration: 35 sign bits, 8 frame bits and two values modulo 5. Retained twists do not change them, and jumble words can. Undefined off the lattice. | Chapter 01 KN2.4; DT-J1 Outcome A3 |
| ν-correction | L5 | One of the 36 named constructions in the static catalogue, with a fixed effect on ν. | DT-J1 O7 (a); chapter 03 |
| Repair list | L1 | The neutral-order repair pairs at a cap the solver names under the 17 October exception. | Chapter 01 KN2.4; DT-J1 |
| Retained checkpoint | L5 | A lattice configuration with a witnessed legal word in the 1,200 retained generators from solved, or certified equal to such a state. Every retained checkpoint is a label checkpoint. | Chapter 01 KN2.4; default 9 |
| Label checkpoint | L5 | A reachable lattice configuration with ν = 0 (state contract A5). Its reachability clause reads the pose and the journal, so labels alone cannot establish it. | Chapter 01 KN2.4; DT-J1 O8 (1) |
| Sticker test | L5 | The label-based completion test, distinct from equality of every exact pose. | Chapter 01 KN2.4; DT-J1 O8 (1) |
| Revision | L1 | One numbered moment of a session; each change makes a new revision. | Chapter 01 KN0.2 |
| Revision reference | L2 | The session, epoch, identity and state, workspace and protection revisions bound to a result. | Chapter 01 KN0.2 |
| Base revision | L2 | The revision a result or view read. Once it is replaced, the output is stale until recomputed. | EV3.3; RPI.5 |
| Stale | L2 | Output read at a revision whose dependencies no longer match the current one. It stays marked with that revision and is never passed off as current. | EV3.3; RPI.12, RPI.13 |
| State reference | L6 | The typed reference to a snapshot or derived value used by kernel reads. | Chapter 01 KN0.3 |
| Snapshot | L2 | A read-only handle pinned to one revision for its lifetime. | Chapter 01 KN2.2 |
| Derived state | L6 | An immutable kernel-computed value with a derivation and reachability claim, without a session revision. | Chapter 01 KN2.11 to KN2.14 |
| Derived model | L6 | A quotient, restriction or simplified model represented by a derivation and claim-preserving maps. | Chapter 01 KN1.12, KN1.13 |
| Difference | L2 | A comparison of named states at a declared model and level. | Partition 4.1; chapters 02, 03 |
| Typed partial result | L1 | A result that says which part it covers, which part is missing, or why it is undefined. | Chapter 01 KN0.4 |
| Verdict | L2 | Yes or no with evidence, or unknown with a reason, for a stated decision. | Chapter 01 KN0.5 |
| None: off the lattice | L1 | A known absence of a lattice value, distinct from insufficient knowledge. | EV3.9; chapter 01 KN0.4; DT-L1 R2-E4 |
| Reachability claim | L2 | What is on record about whether a state can be reached in a named puzzle: witnessed, by theorem without a witness, promised, refuted or open. | Chapter 01 KN2.13; chapter 06 |
| Witnessed reachability | L2 | Reachability accompanied by a finite legal witness in the named puzzle. | Chapter 01 KN2.13 |
| Reachable by theorem without a witness | L2 | A reachability claim supported by the cited theorem without a stored legal word. | Chapter 01 KN2.13 |
| Promised reachability | L2 | A reachability assumption supplied by the source and not upgraded by input validation. | Chapter 01 KN2.13, KN2.14 |
| Proposal | L2 | A word, with a record of where it came from, submitted for a preview at a base revision. | Chapter 01 KN5.9 |
| Preview | L2 | What the engine says will happen if you commit: the exact word and what it does, fixed at one base revision. | Chapter 01 KN5.9 |
| User-action token | L2 | The shell's single-use authority bound to the displayed preview and its literal letters. | Chapter 01 KN5.14, KN5.15 |
| Receipt | L2 | An invocation's command identity, request digest and typed execution outcome; retention follows its kind. | Chapter 01 KN5.21, KN5.23; LI.30 |
| Availability | L1 | Whether a command can run now, and the reason when it cannot. | Chapter 01 KN5.6, KN5.7 |
| Claim | L2 | A typed statement with an evidence state, without conflating score or execution permission. | Chapter 06; INV-5 |
| Statement | L2 | A precise sentence about named objects that records can support or refute. | Chapter 06 section 2 |
| Statement form | L2 | Whether a statement concerns one finite case, every case, some case, or a mix such as every case having some answer. | Chapter 06 EVD.9, section 2 |
| Negation pair | L2 | A statement and its exact opposite, recorded together, so that one record can settle both. | Chapter 06 EVD.9 |
| Statement ID | L6 | A typed-statement digest or stage-qualified, commit-pinned statement name. | Chapter 06; DT-L1 R2-E1 |
| Statement key | L6 | A statement ID with its scope: what evidence discharges, refutes or transports. | Chapter 06 EVD.7 |
| Scope | L2 | The declared inputs, identities and set for which a claim is asserted, every field immutable. | Chapter 06; partition 8.1 |
| Covered set | L2 | What a claim is about: one instance, a declared search set, an admitted domain or every reachable state. The field name `covers` is not learner-facing. | Chapter 06 section 2; EVD.32. Not the kernel row `covers` |
| Admitted domain | L2 | A named set of inputs with a test for membership, such as D[k]. | Chapter 06 section 2; [stage 19 README](../../../../../research/jumbling/restoration/19-height-depth-framework/README.md) |
| Basis | L2 | The theorem, certificate, exhaustive computation, exact computation or sample on which a claim rests. | Chapter 06; partition 8.1 |
| Check | L2 | A recorded confirmation with its checker or reviewer identity and mode. | Chapter 06; partition 8.1 |
| Premise | L2 | A named obligation on which a conditional claim depends. | Chapter 06; partition 8.1 |
| Hypothesis | L2 | A condition a statement assumes; the statement says nothing about cases where it fails. | Chapter 03 LB6.2; chapter 06 |
| Lemma | L2 | A statement with a written proof, used as a step towards other statements; cited by stage and source commit. | Chapter 06 EVD.2 |
| Theorem | L2 | A statement with a written proof, cited by stage and source commit. As a basis it says a proof is cited; who checked it is separate. | Chapter 06 EVD.1, EVD.2 |
| Invariant | L2 | A quantity or property that a named set of twists leaves unchanged. | Chapter 03 LB6.2 |
| Counterexample | L2 | A checked case that lies in a statement's scope, meets its hypotheses and breaks its conclusion. | Chapter 06 EVD.42; chapter 03 LB6.2 |
| Witness | L2 | A concrete object that proves something exists; for reachability, a finite legal word from solved. | Chapter 01 KN2.13; chapter 06 EVD.44 |
| Declared domain | L6 | The inputs on which an operation or claim is defined. | Chapters 02, 06; partition 4.1 |
| Goal | L2 | A predicate typed by puzzle and identity level, with its completion meaning stated. | Chapter 01 KN1.11; chapter 02 |
| Certificate | L2 | Data supporting stated claims for an independent checker of a named kind. | Chapter 01 KN6; chapter 06 |
| Check report | L2 | The checker's verdict, identity, input digest and per-claim trace reference. | Chapter 01 KN6.1 |
| Evidence | L2 | The support for a scoped claim, separate from who wrote the tool, score and permission. | INV-5; chapter 06 |
| Evidence envelope | L6 | Chapter 05's stored evidence carrier, whose facts or judgements depend on LQ10. | Chapters 05, 06; partition LQ10 |
| Result trace | L2 | The linked computation and evidence edges through which a result can be traced; only evidence edges are support. | Chapter 06; EV2.5 |
| Per-claim trace | L2 | The independent checker's own steps and outcomes for each checked claim. | Chapter 01 KN6.4; EV2.6 |
| Evidence words | L2 | The versioned display vocabulary for evidence and run status, interpreted by source and field. | Chapter 06; EV5.13 |
| Proved | L2 | The word shown when a statement stands at the threshold with no premise assumed, over an admitted domain or every reachable state. It always carries its scope phrase. It says the recorded argument passes the threshold, not that a person has reviewed it. | Chapter 06; partition LQ3 |
| Conditional | L2 | A statement that stands once every premise not refuted is assumed; the word names the premises. | Chapter 06; partition 8.1 |
| Finite instance | L2 | The word for a statement that stands on one input or on a declared search set. Nothing outside that set is claimed. | Chapter 06; partition 8.1 |
| Not found within budget | L2 | A search stopped at its budget or declared limit without covering its set, and found no witness. It gives no negative over that set; only a part it examined completely can carry a narrower claim of its own. | Chapter 01 KN7.11; chapter 06 |
| Incomplete | L2 | The job was cancelled, failed, or hit a resource stop, so it gives no negative. A search stopped by its own declared budget reads "not found within budget". | Chapter 01 KN7; chapter 06 |
| Refuted | L2 | A claim contradicted by evidence of the kind required for that statement. | Chapter 06; partition 8.1 |
| Open | L2 | The stated claim is unresolved by the available evidence. | Chapter 06; partition 8.1 |
| Verified | L2 | The command-layer evidence label interpreted in its check context, without granting execution. | Chapter 06; H-08 |
| Recorded | L2 | A source's claim or result retained without treating it as a fresh independent check. | Chapter 06; partition PS3 |
| Unknown | L1 | Not enough is known for the requested conclusion; never false or a numeric zero. | Chapter 01 KN0.5; INV-5 |
| Unchecked | L2 | A declared check has not been performed for the stated input and context. | Chapter 06; H-08 |
| Computed | L6 | A research label whose meaning is mapped from its format, field and value. | Chapter 06; partition PS3 |
| Measured | L6 | In the research evidence vocabulary, exact computation on finite inputs, not a performance timing. | Chapter 06; partition 8.1 |
| Lead | L6 | A source-qualified research indication retained without promotion to a proof. | Chapter 06; partition 8.1 |
| Reported | L6 | Computed by a program that is not committed, so the repository cannot replay it. | Chapter 06; partition 8.1 |
| Exact transport | L6 | A lemma-justified move of a claim to another input that preserves its subject, with no re-run. Along a gauge, the lemma is Theorem 1 of note N1 and the checked relation is Z = X·γ. | Chapter 01 KN1.16, KN7.5; chapter 06 EVD.18 |
| Check mode | L6 | The recorded distinction among reused transport, fresh exact audit and actual replay. | Chapter 06; partition 8.1 |
| Grade | L6 | A (basis, check) pair in chapter 06's partial order. | Chapter 06 EVD.19 |
| Alternative | L6 | One argument's summary: its weakest grades and its premises. A statement's summary keeps the undominated ones. | Chapter 06 EVD.23 |
| Standing | L2 | Whether a statement's recorded arguments clear the threshold: first with no premise assumed, then with every premise assumed except those refuted in the first pass. It describes records, not truth. | Chapter 06 EVD.30 |
| Threshold | L2 | The versioned bar a recorded argument must clear before a word such as proved or conditional appears. | Chapter 06 EVD.29 (raising it is LQ3) |
| Typed inconsistency | L2 | The word shown when the records behind a statement contradict each other or break a record rule. It states no claim. | Chapter 06 EVD.32 row 0, EVD.40 |
| Evidence graph | L2 | The record of how supports combine: an argument needs all its parts, and any one argument that clears the threshold carries the statement. | Chapter 06 EVD.15, EVD.39 |
| Principle record | L2 | A tool's specification, justification, worked instance, generated decomposition and the parts a person writes: step map, reason, failure, what an answer does not mean, hand derivation, prerequisites and leaf specifications. | Chapter 03; INV-12 |
| Principle | L2 | The statement named as a tool's reason in its principle record, with that statement's hypotheses: why an answer means what the specification says. | Chapter 03 LB6.2, a working definition with no owner text |
| Decomposition | L2 | The generated list of the parts a tool calls, down to base parts. It shows how the tool is built, not why its answer is right. | Chapter 03 LB6.2; EV5.4 |
| Base part | L2 | A shipped function at which a tool's decomposition stops. | Chapter 03 LB6.1, LB6.2 (meaning inferred) |
| Step map | L2 | A stated relation between a decomposition's parts and the steps of a proof, reporting what is unmapped on both sides. | Chapter 03 LB6.2 |
| Hand derivation | L2 | The written working of a worked instance: the givens read from named parts, and the steps a person checks from them, with the step count. | Chapter 03 LB6.2 |
| Leaf specification | L2 | A written statement of what an opaque base part computes, naming the test that covers it. | Chapter 03 LB6.2 |
| Worked instance | L2 | A small instance on the real geometry whose steps a person can check by hand from named givens. | Chapter 03 LB6.1 |
| Fixture | L2 | A fixed input with its expected results, kept apart from the solver's session. A pass shows the tool reproduced those results at that input, nothing wider. | Chapter 03 LB6.2; EV5.7 |
| Fork | L2 | Your own copy of a built-in tool. It keeps a link to its origin and inherits none of the origin's evidence. | Chapter 03 LB6.3; EV5.5 |
| Editable parameter | L2 | A value a tool's source declares editable. Changing it makes a new version of the tool with no inherited evidence. | EV3.7; chapter 03 LB6.3 |
| Library entry | L6 | A source-addressed operation with its checked domain, effects and supporting references. | Chapter 03 |
| Research tier | L6 | The entries that search, minimise, rank or select over twists, words or caps. They ship in 1.0 and stay out of the solving profile. | Chapter 03 LB1.5; EV5.17; owner, 6 November 2026, LQ16 |
| Effect | L6 | A declared and checked kind of interaction a program can perform. | Chapter 02; partition LG3 |
| Capability declaration | L6 | A module's request for authority that the host must grant for the profile and run. | Chapters 02, 09 |
| Typed IR | L6 | The checked intermediate representation consumed by execution and environment services. | Chapter 02; partition LG4, LQ9 |
| Module | L6 | A digest-addressed source unit with its edition, manifest and dependencies. | Chapter 02; partition LG5 |
| Module manifest | L6 | The module's declared supported identities, capabilities, edition and dependencies. | Chapter 02 |
| Lockfile | L6 | The dependency resolution that pins the digests used by a run. | Chapter 02 |
| Language edition | L6 | A named source or semantic version whose changes preserve recorded saved meaning through migration. | Chapters 02, 05 |
| Profile | L6 | The chapter 07 capability and labelling context, without deciding a separate product entrance. | Chapters 07, 09; partition LQ1 |
| Author class | L6 | The recorded source class of a tool, distinct from mathematical evidence and understanding. | Chapter 07; partition BM2 |
| Provenance block | L6 | Chapter 07's assistance-origin information carried by previews and stored words. | Chapter 07; partition BM2 |
| Boundary class | L6 | A shipped tool's reviewed class under the human-solve boundary. | Chapter 07; partition BM5 |
| Declassifier | L6 | A fixed policy-enabled entry that permits a named information flow; programs cannot declare one. | Chapter 07; partition BM2 |
| Flow rule | L6, not learner-facing unless LQ2 turns it on | A policy switch: when on, in the solving profile, a preview's word and target may depend on the state or computed results only through declassifiers. | Chapter 07 BND.33 |
| Flow label | L6 | The provenance information following permitted data and control dependencies if LQ21 selects labels. | Chapter 07; partition LQ21 |
| Capability separation | L6 | LQ21's alternative separating state-reading programs from preview-preparing programs. | Chapter 07; partition BM2 |
| Solving session | L2 | A session whose journal can stand as a solve record; every ordinary session is one. | Chapter 07 BND.14 |
| Solve record | L2 | The journal of a solve, read as its record, with the assistance the system recorded. | Chapter 07 BND.14, BND.35 (the mathematician's reading; the term is used and never defined) |
| Sink | L6 | A place a labelled value can reach: a preview, a draft, a display, a workspace, a file or a record. | Chapter 07 BND.26 |
| Transparent entry | L6 | A shipped function on the policy's list whose output carries only the join of its inputs. | Chapter 07 BND.25 |
| Policy table | L6 | The versioned data that fills the boundary mechanisms with a joint setting's choices. | Chapter 07 BND.28 |
| Assistance summary | L2 | The per-solve account of the assistance the system recorded. | Chapter 07 BND.35 |
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
| Journal branch | L6 | An engine-owned journal line whose availability in solving sessions remains LQ4. | Chapters 01, 08 |
| Journal | L2 | The session store's parent-linked durable events, distinct from the debugger's inspection cursor. | Chapter 01 KN4.1; chapter 08 |
| Saved checkpoint | L2 | A stored session point with the replay path needed to reconstruct and validate it. | Chapter 01 KN4; chapter 05 |
| Workspace record | L2 | Persistent work around the puzzle changed only through workspace commands, with provenance where required. | Chapter 01 KN4.4; chapter 05 |
| Filter | L1 | A declared query limiting display or interaction without changing the mechanical state. | Chapter 05; solving workflow section 4.3 |
| Preference | L1 | A persisted user setting whose stored meaning is versioned by chapter 05. | Chapter 05; partition PS1 |
| Search space | L6 | The declared set and alphabet over which a research search ranges. | Chapter 03; partition LB5 |
| Search record | L6 | The inputs, coverage and outcome of a search, including a bounded failure to find a witness. | Chapters 03, 05, 06 |
| Strategy | L6 | A research procedure composed over a declared search space and budget. | Chapter 03; partition LB5 |
| Descent rank | L6 | A declared quantity with stated decrease obligations on the admitted operations. | Chapter 03; partition 10.3 |
| View-model node | L6 | The declarative node by which program content reaches the host. | Chapter 10; EV3.2 |
| View | L1 | A presentation of information at one revision; further views are allowed in every profile. | Chapter 10 RPI.81 to RPI.83; EV3.11; owner, 6 November 2026, LQ6 |
| Panel | L6 | A declared host container for view-model content and its command references. | Chapter 10; EV3.2 |
| Control | L6 | A view-model element naming a typed command interaction. | Chapter 10; EV3.2 |
| Overlay | L1 | Declarative information attached to a view. | Chapter 10; partition EV3 |
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
| Layout | L2 | A saved arrangement of what a view shows; a program's layout can change only where its source declares it editable. | Chapters 05, 10; EV3.7 |
| Chart | L6 | A display map governed by chapter 10's declared domain and precision, with a user vertex stage checked under its stage safety contract. | Chapter 10 RPI.68 to RPI.80; owner, 6 November 2026, LQ7 |
| Display projection | L1 | A stated map used to display the geometry, without altering its canonical objects. | Chapter 10; charter M2 |
| Section | L6 | A declared representation of a geometric section with its exact mathematical basis. | Chapters 03, 10; partition LB4 |
| Atlas | L6 | A collection of linked charts with their domains and object correspondences. | Chapters 03, 10; partition LB4 |
| Action | L6 | The declared mathematical action on the typed objects of a representation. | Chapter 03; partition LB4 |
| Embedding | L6 | A representation map with its source objects and stated mathematical properties. | Chapter 03; partition LB4 |
| Relation | L1 | A stated connection between named objects in one state, such as which piece occupies a position. | EV3.8; chapters 02, 06 |
| Graph | L6 | A representation with typed vertices and edges whose meanings are declared. | Chapter 03; partition LB4 |
| Tree | L6 | A tree representation with declared node identities and edge relations. | Chapter 03; partition LB1, LB4 |
| Cell complex | L6 | A representation of cells and their incidence relations at a stated dimension. | Chapter 03; partition LB4 |
| State graph | L6 | A graph of named states and declared transitions, without making its nodes authoritative sessions. | Chapters 03, 10; partition LB4 |
| Relationship strip | L1 | The strip that shows stated relations, in the grammar shared by the code and the theory book, inside Global and Local. | EV3.8; owner design decisions section 2 |
| Competence check | L1 | An isolated task recording correctness, mistakes, help and transfer. | EV5.7 to EV5.9; access policy: LQ14 |
| Soft disclosure | L1 | Gradual introduction of key information while the palette continues to offer every command. | EV5.8; owner design decisions section 5 |

- **EV5.18 [D]** The word chosen for a sense is notation preference (owner design decisions, section 8). Three things
  are correctness: one reference resolves to one concept (EV5.10); each sense has its own row, so a script can tell
  them apart; a bare word is never used where two senses can occur. The qualified names below are proposals. Applying
  them to prose, and the script check, come with LP4 under EV5.10, and a sense without a row gets one when it is
  integrated (EV5.15).

| Word | Qualified names |
| --- | --- |
| Domain | **alignment domain**: a coset γK⁺, pieces whose poses fit one rotated copy of the lattice (card 6, Model). **declared domain**: the inputs on which an operation or claim is defined. **admitted domain**: a declared domain with a membership test, such as D[k] (chapter 06, section 2). **chart domain**: where a chart's map is defined (chapter 10). Jumbling frame domain stays: an alignment domain at height 2 |
| Projection | **display projection**: a chart stage that draws the 4D geometry (RPI.31). **level projection**: the exact, possibly partial map to a coarser identity level (`project`, KN1.9); the label projection π is one |
| Key | **key**: on the keyboard. **statement key**: statement ID with scope (EVD.7). **equality key**: a word's normal form. **reproducibility key**: chapter 09. **source key**: the DisplayInstance field (chapter 10) |
| Checkpoint | **saved checkpoint**: a stored session point (`session.checkpoint.save`, KN4.1). **retained checkpoint** and **label checkpoint**: predicates on a state (A5) |
| Branch | **journal branch**: an engine-owned journal line. **tree branch**: the class b(A) of a piece's first tree step (card 6, Model). Stage 19's "complete branch" stays research prose |
| Edition | **language edition**: a source or semantic version. **evidence edition**: the four versioned tables (EVD.14) |
| Rank | **descent rank**: a declared quantity with decrease obligations (chapter 03). **matrix rank**: linear algebra (chapter 03). **census rank**: the orbit field `rank` (`assets/census.json`; what it counts is not established) |
| F symbols | F_x stays foreign load (card 6, Model; card 7 writes F_c for it). The floor takes the word floor or another symbol, and chapters 01 and 07 now use the word. Which symbol is preference; the collision is correctness. F_f, F2 and F5 keep their own meanings |
| Clean | **clean grip**: S_d empty (card 6, Model). **clean step**: a legal word landing a unit. Bare "clean" is never used. "Unblocked grip" is an option (preference) |
| Block | **achievement block**: solver-declared (`terms.json`). **jumbling block**: pieces no cut separates. Identity block and Provenance block keep their names, as bundles of fields. Bare "block" means achievement block only. Blocked is unrelated to all of these |
| Provenance | **assistance provenance**: the origin of a word (BM2, PS1.32). **result trace**: how a result traces to its inputs and support (EV2.5). **attribution**: author, reproducer, scope setter, review class (EVD.6; PS1.35 already says Attribution) |
| Macro | **macro**: a reusable named word. **lowering macro**: a word ending in a jumble that lowers Φ (stage 15 README). **macro certificate**: Theorem MC's event-by-event certificate of a word (stage 15 README) |
| Covers | **covered set** (chapter 06, section 2). **cap cover**: "d covers c", DT-J1 M4's predicate at a named cap (kernel row `covers`). **helper cover**: helpers whose inside sets cover a target's straddlers (card 7; stage 14) |
| Level | **identity level**, **learning level** and **grip level** lev(c) (card 7) |
| Witness | **witness word** against **witness piece** (chapter 10) |
| Chamber | **cut chamber** against **flag chamber** (the theory) |
| Base part | **decomposition leaf** (LB6.1) against **base-tier function** (BS2) |
| Derivation | **hand derivation**: the written working of a worked instance (LB6.2). **state derivation**: how a derived state was computed (KN2.11) |

## 7. Error codes added by this chapter

These are the codes used by the clauses above. Chapter 13 owns the final spelling. `KN1:input.identity_mismatch` and
`KN2:stale.revision_gone` are reused from chapter 01 rather than added here. Cancellation, budget exhaustion and command
receipts keep their KN7 and KN5 outcomes; an open or unknown evidence value is not another error category.

| Code | Meaning | Clauses |
| --- | --- | --- |
| `LG1:syntax.recovery_gap` | A missing or invalid source span was recovered for editing; it supplies no executable value. | EV1.2, EV1.4 |
| `LG2:type.mismatch` | A source expression, edit or declared parameter has an incompatible type. | EV1.4, EV1.6, EV3.7 |
| `LG2:type.exactness_violation` | Forwarded unchanged under LG2.22's precedence. | EV1.4; LG2.22 |
| `LG2:type.exact_required` | The direct scalar case delimited by LG2.22. | EV1.4; LG2.22 |
| `LG3:effect.undeclared` | An inferred effect is outside the declaration or the allowed view effects. | EV1.4, EV3.2 |
| `EV1:evidence.obligation_unmet` | A required claim or check is absent for a checked operation's obligation. | EV1.4 |
| `EV1:stale.editor_source` | An editor response or proposed edit names source bytes that are no longer its target. | EV1.1, EV1.6 |
| `EV1:input.source_range_invalid` | A source endpoint, host unit or conversion is invalid for the pinned scalar boundaries. | EV1.10 |
| `EV1:input.module_not_checked` | Activation lacks a successful check of exactly the proposed module and dependency closure. | EV1.11 |
| `EV1:stale.activation_base` | Another active module digest replaced the activation request's expected digest. | EV1.11 |
| `EV2:input.replay_dependency_missing` | A required digest, source edition, IR version or trace input cannot be resolved. | EV2.1, EV2.3, EV2.6 |
| `EV2:input.word_range_invalid` | A literal interval or compressed occurrence path disagrees with the word's exact expansion. | EV2.11 |
| `EV2:stale.debug_stop` | A step request names an obsolete stop or reuses its request ID with different inputs. | EV2.12 |
| `EV2:evidence.replay_mismatch` | Replay under the stated key differs from the record's required exact comparison. | EV2.4 |
| `EV2:capability.debug_write_forbidden` | Debugging or replay would require a live write or an unsupported effect. | EV2.1, EV2.9 |
| `EV3:stale.display_pick` | A pick binding is stale; checked before source navigation under RPI.24, RPI.25 and RM.10. | EV3.5 |
| `EV3:stale.source_link` | After pick bindings pass, a source range is applied to a different producing digest under RM.10. | EV3.5, EV3.6 |
| `EV3:input.round_trip_undeclared` | The selected source part does not declare the requested round-trip edit. | EV3.7 |
| `EV4:input.binding_conflict` | Bindings resolve to a duplicate or conflicting physical chord in a routing class. | EV4.3, EV4.7 |
| `EV4:input.binding_prefix_conflict` | A complete command binding is also a prefix of another sequence in that class. | EV4.7 |
| `EV4:input.binding_layout_mismatch` | Layout-dependent resolution is missing or conflicts under a declared layout. | EV4.7 |
| `EV4:input.recording_step_unresolved` | A recorded semantic step lacks a required command, module, object or input mapping. | EV4.13, EV4.14 |
| `EV4:stale.recording_step` | An unattended step's arguments name a revision or preview that is not current, or a current undo or redo hint does not target the played action mapped from the recorded target; no silent retargeting is allowed. | EV4.14, EV4.18 |
| `EV4:input.recording_identity_mismatch` | Model identity or an input digest differs from the recording's; refusal before playback, stop during it. | EV4.14 |
| `EV4:input.recording_step_outcome_unknown` | A recorded step has no known terminal outcome; playback and conversion stop at that step. | EV4.19 |
| `EV4:input.recording_state` | Start while recording, stop with none, or a play or `from` that EV4.22's table does not allow. | EV4.22 |
| `EV4:input.recording_not_convertible` | A step has no form in the requested target, or the draft word is empty; nothing is written. | EV4.23 |

## 8. Conformance tests

These are test requirements for chapter 12's phases. No conformance test listed here exists or has been run. A source
or synthetic test cannot establish native input behavior or a performance result. Performance and human tests retain
the unresolved protocols named in their checks.

| Test | Checks | Class |
| --- | --- | --- |
| CT-EV1-01 | EV1.1, EV1.2: an incomplete declaration yields recovered spans and completion context but no runnable module. | source |
| CT-EV1-02 | EV1.4: fixtures for types, effects, exactness, evidence obligations and model or menu mismatches produce distinct codes and correct source ranges; LG2.22's direct and helper condition refusals pass through unchanged, distinct from its direct scalar exact_required case. | source |
| CT-EV1-03 | EV1.5, EV1.6: hover and definition resolve pinned dependencies; rename changes resolved references only and refuses stale edit targets. | source |
| CT-EV1-04 | EV1.7, EV1.8: breaking a draft leaves the old digest running; every route refuses an unchecked new digest. | source |
| CT-EV1-05 | EV1.10: multi-unit characters, insertion ranges and end positions round-trip through the chosen coordinate convention. Malformed UTF-8 (61 FF 62 and truncated 61 F0 9F 98) uses ERR.10's raw context without a span or host conversion; even [0,0) is not a valid SourceRange for either input. | synthetic |
| CT-EV1-06 | EV1.9: parse, diagnose and completion meet the chapter 11 LX-4 bars on realistic modules once the bars and protocol are set. | performance |
| CT-EV1-07 | EV1.12: each editor refusal exposes the event, reason and next-action data without handling that depends on English text. | source |
| CT-EV1-08 | LP4 (PHS.26, PHS.62), EV1.11: activation atomically changes A to B for later invocations while a suspended run stays on A. A stale expected digest and an unchecked candidate refuse without changing the active version. | source |
| CT-EV2-01 | EV2.1, EV2.8: enumeration of debugger operations and stepping over effect sites changes no journal, workspace or session digest and emits no commit. | source |
| CT-EV2-02 | EV2.2: exact values, declared display precision, partial results and unknown verdicts stay distinct in inspection. | source |
| CT-EV2-03 | EV2.3, EV2.4: a complete reproducible record matches its exact bytes, outcome, coverage and charges on repeated headless replays. | source |
| CT-EV2-04 | EV2.3: a missing module and another model or reversed menu order report their specific blockers without evaluating a replacement file. | source |
| CT-EV2-05 | EV2.4: a planted result difference reports a replay mismatch; a safety-stopped record remains incomplete without a negative claim. | synthetic |
| CT-EV2-06 | EV2.5, EV2.11: a compressed long step resolves the cited literal positions lazily under the chosen interval convention and retains its alphabet. | source |
| CT-EV2-07 | EV2.6: checker trace steps bind the certificate and checker digests; the debugger imports no producer construction code. | source |
| CT-EV2-08 | EV2.7: cached and uncached runs have the same deterministic charges; elapsed seconds never change the stop step. | source |
| CT-EV2-09 | EV2.8, EV2.10: journal travel uses query rows and leaves the active line unchanged; unavailable revisions stay explicit and a digest-pinned research run finishes on its old input across commit or undo. | source |
| CT-EV2-10 | EV2.9: effectful replay follows the settled contract, refuses live writes and distinguishes trace inspection from complete re-execution. | source |
| CT-EV2-11 | LP4 (PHS.26, PHS.62), EV2.12: duplicate step requests across a suspended kernel call do not redispatch or recharge it; completion stops at the continuation. Missing source and step mappings, stale stops and every terminal reason stay distinct. | source |
| CT-EV3-01 | EV3.1, EV3.5, EV3.6: source to node to instance to canonical ID to source returns the producing digest; duplicated instances retain one canonical identity. | synthetic |
| CT-EV3-02 | EV3.3, EV3.10: a burst of revisions adopts the newest engine bundle at once, marks old program output stale and skips obsolete view evaluations. | synthetic |
| CT-EV3-03 | EV3.4, EV3.12: an anchored derived display names its derivation and base without acquiring a revision; an imported value's plan states lineage `imported` and claims nothing with its comparison base. | source |
| CT-EV3-04 | EV3.5: a stale pick binding returns EV3:stale.display_pick before source navigation, including when source also differs; current bindings with a mismatched source digest return EV3:stale.source_link, while navigation to the producing version survives an editable-source change. | synthetic |
| CT-EV3-05 | EV3.7: declared parameter, layout and default edits recheck source; an arbitrary drawn part cannot be round-tripped. | source |
| CT-EV3-06 | EV3.8 to EV3.10: an off-lattice relation is distinct from unknown; unchanged frame draws perform no relation computation. | source |
| CT-EV3-07 | EV3.2, EV4.15: a view cannot write workspace state or make undo stale; an explicit workspace command joins the durable action order. | source |
| CT-EV3-08 | EV3.11, EV3.13: further views are admitted in every profile under RPI.81 to RPI.83, with identity, one base revision, source linkage and no program commit path; Global and Local remain linked and cannot be closed. Hidden allocations remain charged and opening/showing a view obeys RM.39's measured ceilings. | source |
| CT-EV3-09 | EV3.13: H-11 carries RPI.15's display primitives, the RPI.68 to RPI.80 user-stage contract and the free-view workload into the renderer feature list before its freeze; layout, appearance, motion and wording remain under the design-line hold. | source |
| CT-EV4-01 | EV4.1, EV4.2: overlay commands appear in the palette; forbidden kinds are refused and availability uses stored tables without running programs. | source |
| CT-EV4-02 | EV4.3, EV4.4: key capture, text, list navigation and global dispatch run in the fixed order; resolved absolute arguments survive later camera and focus changes. | synthetic |
| CT-EV4-03 | EV4.5: a blocked interpreter leaves kernel-row key dispatch independent and undo available through KN5. | source |
| CT-EV4-04 | EV4.6: held-key events repeat only the allowlist; a workspace append, preview job, twist or commit is admitted once and busy input is not queued. | synthetic |
| CT-EV4-05 | EV4.4, EV4.6, EV4.7: actual Windows IME, text undo and focus loss suppress puzzle dispatch and clear held keys. | windows |
| CT-EV4-06 | EV4.7: duplicate and prefix conflicts fail per routing class; the DT-M1 US and French layout counterexample fails under its conflicting layout. | synthetic |
| CT-EV4-07 | EV4.8, EV4.9: rebind alone changes the applied map; both puzzles use one macro base with identity, alphabet, literal order and exact expansion length preserved. | source |
| CT-EV4-08 | EV4.10 to EV4.14: a recording retains semantic commands, canonical arguments, identities, word and action references, revisions and receipt bindings; playback has no rebinding operation and source conversion passes the ordinary checks and grants. | source |
| CT-EV4-09 | EV4.12, EV4.18: conversion, stepwise play and unattended play never store or replay a token; every commit, including checkpoint save, waits for a fresh solver action; operation.execute uses a new whole-word preview, and a recorded live twist uses no preview (ONE.78). | source |
| CT-EV4-10 | EV4.13, EV4.16, EV4.23: recording and both conversions preserve provenance through parts, cannot revive old permission and cannot launder research output into a solving preview; a source-produced word is refused under BND.84 in a solving session. | source |
| CT-EV4-11 | EV4.10, EV4.11: resolved completed, committed and refused invocations each append one semantic step; unresolved input and recording.* append none. Pending outcomes never masquerade as commits; no token or operation preview ID enters the stored arguments. | source |
| CT-EV4-12 | EV4.17, EV4.18, EV4.22: enumerate the eligibility table across kinds, lanes, jobs and gates. Camera rotation and an eligible workspace edit play; a no-job query in lane none plays only by the default; confirm, file_check, file, app, exclusive and job rows wait. Filter freshness remains checked without retargeting. | source |
| CT-EV4-13 | EV4.19: cover every historical and new-child outcome in both tables. Historical refused/failed/cancelled steps remain visible and are skipped; missing terminals stop play and conversion with recording_step_outcome_unknown. Only completed advances an unattended child, while completed/committed advances a user child; new refusals, failures, cancellations and unknown outcomes stop without retry. | source |
| CT-EV4-14 | EV4.20, PS1.44: crash after pending is durable but before dispatch; after durable child completion but before cursor advance; and during a manual continuation. Outcome lookup precedes continuation, each known success advances at most once, unknown_outcome never dispatches a fresh retry, and an unrelated invocation never moves the cursor. | synthetic |
| CT-EV4-15 | EV4.21: play an undoable workspace edit, camera.rotate and a query. Only the workspace action becomes an undo target; camera/timer rows with undo none invent none, and the cursor is never undoable. A later failure retains earlier completed actions. | source |
| CT-EV4-16 | EV4.14: an unattended step naming a revision that is still current plays, and one naming a revision that is not current, or a filter preview that is not fresh, stops as stale; a changed model or input digest refuses before play or stops during it with recording_identity_mismatch. No canonical argument is silently retargeted. | source |
| CT-EV4-17 | EV4.23: test both targets on the five vectors: camera only refuses an empty draft and becomes a permitted source call; macro.add(W), operation.execute(W) yields W once; W1, W2, undo W2, W3 yields W1 W3; undo of an older action refuses; jumble.on between word steps refuses. Every refusal writes nothing. | source |
| CT-EV4-18 | EV4.23: a redo restores a word at its recorded place; undo/redo of a recorded non-word action and checkpoint save add no draft letters but refuse source. Completed non-word steps enter the draft report; source accepts only KN5.4-invocable calls without confirm/file_check and refuses file/app. A successful draft conversion is one undoable Macro-phase action retaining every contributing block. | source |
| CT-EV4-19 | EV4.22: start while recording, stop with none and a play or from outside the table refuse with recording_state. Each table row gives its outcome after reconciliation, a rewind opens a new playback with an empty map, and W, rewind, W, undo undoes the second W only; play ends at the next user step or end, job.stop halts it, and every child has a new command/request receipt binding. | source |
| CT-EV4-20 | EV4.18, PS1.44: record W then undo of W. In playback W takes action 100, then an ordinary macro.add(X) outside playback takes 101 and leaves the cursor. The undo step reads a hint targeting 101, so playback stops with stale.recording_step, X is not undone and W stays applied. A recorded target with no actions entry stops the same way, and redo resolves through the same map. | source |
| CT-EV5-01 | EV5.2 to EV5.5: generated docs and principle records resolve their pinned source, domains, empty-answer meanings, instances, decomposition, evidence and the parts a person writes; a fork inherits no check. | source |
| CT-EV5-02 | EV5.6, EV5.10, EV5.14, EV5.15, EV5.18: the integrated glossary covers chapter terms, proposed changes have been reviewed, every entry has build evidence, each sense of a homonym has its own row, flow goals need no undisclosed concept, each step's prerequisite concepts are taught before that step's task, and prerequisites form a DAG. | source |
| CT-EV5-03 | EV5.7, EV5.8: non-authors perform isolated fixture and transfer tasks under the owner's rubric; correctness, time and help use are reported separately. | human |
| CT-EV5-04 | EV5.8, EV5.9: ordinary soft disclosure never locks commands; if the owner chooses BS3, only the settled per-principle capability gate changes preview access. | source |
| CT-EV5-05 | EV5.1, EV5.3, EV5.11: the mathematician reviews the principle explanations and shared grammar; people can follow a real example to its primitives and its reasons under the owner's comprehension protocol. | human |
| CT-EV5-06 | EV5.13: chapter 06 derives explanation words from the facts envelope, with statement, scope, basis, check and job outcome separate; source labels remain visible, no writer asserts formal proof in 1.0, and evidence, score and permission never merge. | source |
| CT-EV5-07 | EV5.17: the 1.0 inventory includes the research library and archive checkers, excludes the repository's theory book, proofs and stage research records, and retains library explanations without extending the theory book to them. | source |

## 9. Open items

The owner answered LQ1 to LQ21 on 6 November 2026. LQ8, LQ9, LQ13 and LQ21 wait for their evidence, and LQ2's option set
waits for the joint setting. This table records the remaining choices and what decides them.

| Question | What stays fixed and what the answer changes | Clauses |
| --- | --- | --- |
| LQ2, solver-developed tools | The execution gate and built-in boundary stay; the joint policy decides admitted preview and display flows and any BS3 competence gate. | EV4.16, EV5.9, EV5.16 |
| LQ8, engine language | LX-5 and LX-6 decide the engine language; library adapters, the checker host and LX-4's implementation follow it. Decisive runs wait for the owner's window (decision 15). | EV1.9, EV2.7 |
| LQ9, route and placement | LX-1 to LX-5 feed the choice together with LQ21; transport, fault containment, IR stepping and source mappings need that choice. First-pass records are leads, not a ruling. | EV1.9, EV1.11, EV2.12, EV4.5 |
| LQ13, syntax | LX-8's human learnability test decides the edition's syntax before LP2 builds the parser; source preparation has run, but the human test has not. | EV1.2, EV1.6, EV1.10, EV5.12 |
| LQ21, flow control | LX-1's label cost and LX-3's refusal counts decide labels versus capability separation with LQ9; provenance survives either choice. | EV1.4, EV2.2, EV4.16 |

The clauses tagged `[O]` leave these specific contracts unsettled:

| Clause | Open mechanism or decision | Decides it |
| --- | --- | --- |
| EV1.13 | Automatic versus user-triggered requests to the specified activation service; editor appearance | Owner and resumed design line; no trigger chosen |
| EV2.9 | Captured inputs and recorded-effect inspection versus isolated effect sinks for replay | Chapters 05 and 09 |
| EV5.9 | BS3's competence gate and validity after a principle-record change | Joint setting of LQ2 under soft disclosure |
| EV5.12 | Human-test rubrics, comprehension and transfer criteria | Owner, with the mathematician; chapter 11 for LX-8 |
| EV5.14 | Proposed learning levels and a first-use order for jumbling and language terms | The owner's learning path, with chapter 06's evidence vocabulary |

Other dependencies and differences in the sources remain visible:

| Source or dependency | Difference or unfinished obligation | Affected clauses |
| --- | --- | --- |
| H-08 and DT-M1 | KN5.20, LI.9 and LI.26 specify the single durable undo order and shared entry during jobs. The table reconciliation and builder checks must match those clauses, DT-M1 routing and the repeat allowlist; runtime conformance is still owed. | EV4.1 to EV4.8, EV4.15 |
| Availability and overlays | KN5.8 cites LI.18 to LI.21's argument-aware table, overlay conjunction and generation event. The environment consumes that draft protocol; its implementation and assembled-candidate review remain owed. | EV1.8, EV4.1, EV4.2 |
| H-08's renderer rule versus derived-state display | The older table forbids drawing an uncommitted state. The partition and KN2.12 require explicitly identified derived display plans. ONE.62 and RPI.7 keep these plans apart from adoption of the session state, and the renderer handoff carries that line. | EV3.3 to EV3.5, EV3.12, EV3.13 |
| Solving workflow's historical key sets versus DT-M1 | The workflow records 0.4's orbit sets as historical requirements; DT-M1 selects one default map, explicit applied anchors and solver rebinds for 1.0. Its settled rule governs bindings here. | EV4.7, EV4.8, EV5.6 |
| Glossary versus DT-M1 and KN1 | The current twist entry equates a twist with a generator. Cap, label, piece, generator, orbit, orientation, protection-prefix and complete-effect entries need their puzzle and level scope; label-fixed centres can rotate in pose. Section 6 proposes corrections without changing the glossary. | EV2.2, EV3.9, EV4.9, EV5.14 |
| Research block, frame and unit vocabulary | The tail's block and frame-part unit differ from a solver-declared block and an ordered sticker frame; card 6 uses unit for pieces with one exact pose. Proposed entries qualify those meanings instead of merging them, and EV5.18 lists the proposed names. | EV5.10, EV5.14, EV5.18 |
| KN2.14 versus the display base requirement | An imported derived value need not have the snapshot ancestry from which EV3's required base revision would come. The kernel remains unchanged; ONE.21 and EV3.12 give the display contract. | EV3.4, EV3.12 |
| DT-L1 rounds 2 and 3 | Round 3 answered R2-E1 to R2-E4 (overview section 8). Every use here names its card, and EV3.8, EV3.9, EV5.3 and EV5.4 carry round 3's corrections. | EV2.5, EV2.6, EV2.10, EV3.8, EV3.9, EV4.9, EV4.16, EV5.3, EV5.4, EV5.10, EV5.13, EV5.14 |
| EN3 and EN4 | Relationship data uses RPI.5/RPI.65; LI.3 and LI.31 specify layer ownership and publication. LI.14, LI.18 to LI.30 and KN5.21/KN5.23 supply query, availability, lane, undo and outcome contracts. Final row reconciliation, implementation and conformance remain owed. | EV1.8, EV2.8, EV3.8, EV4.1 to EV4.15 |
| Other chapters | Chapter 02 owns checker and IR services; 03 principle records; 05 bytes and versioning; 06 evidence meaning; 07 and 09 policy and grants; 08 lifecycles; 10 display linkage; 13 error spellings. | EV1 to EV5 |
| LX-4, LX-8 and chapter 12 | First-pass headless function timings and syntax preparation are recorded in work/experiments/language-lx/lx4-editor/RESULT.md and lx8-surfaces/RESULT.md. They establish neither presented-frame latency nor human comprehension; decisive foreground runs wait for decision 15's window and the human test needs the owner's protocol. | EV1.9, EV5.12; CT-EV1-06, CT-EV5-03, CT-EV5-05 |
| Design line | The owner's 3 November hold still covers the editor's appearance, the strip's design and DT-J2 to DT-J4. This data chapter reopens none of them. | EV1.12, EV3.13, EV5.11 |

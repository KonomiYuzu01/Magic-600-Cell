# EN4 layer interfaces

Status: **draft for review, `en4-layer-interfaces/1-draft`. Not accepted or frozen.**

This is the main-line contract requested by PHS.13. It accompanies [H-08](command-table.md),
[chapter 01](../../architecture/1.0/language/architecture/01-kernel.md) and
[chapter 08](../../architecture/1.0/language/architecture/08-one-state.md).
The common working forms are in [the closure handoff](../../architecture/1.0/language/closure/HANDOFF.md).
They are the integrator's proposals. The [DT-M1 owner record](../../wiki/decisions/owner-decisions-2026-10-22-dt-m1.md)
supplies the settled undo, concurrency and routing rules. No product implementation or timing is reported here.

## 1. Scope and ownership

- **LI.1 [D]** This draft supplies the six layer boundaries, common invocation lifecycle and kernel service obligations.
  It does not supply p2a's final command rows, p1's jumbling transition table, p3's byte formats, p6's renderer adapter,
  or the later language-kernel annex. A cited parallel input is a dependency, not a claim that it is delivered.
- **LI.2 [D]** The following definitions have one owner. Consumers cite them rather than copying their fields.

| Definition | Defining clause | Use here |
| --- | --- | --- |
| `IdentityBlock`, `RevisionRef`, `PreviewDeps` | KN0.1, KN0.2, KN0.8 | Identity, observed revision and preview freshness |
| `FilterPreviewDeps` | KN0.9 | Freshness of the submitted filter result, independent of word previews |
| `ActionHint` | LI.9 | Current action order and undo/redo target |
| `StateRef`, partial result, verdict | KN0.3 to KN0.5 | Immutable reads and exact answers |
| `Preview` and token binding | KN5.9, KN5.14, ONE.4, ONE.41 | Engine-owned witness and explicit solver action |
| `Receipt`, `CommitRecord`, `Completion`, `ResultRef`, `CancelReason` | KN5.21 | Invocation outcomes, including non-commit success |
| `Availability` | KN5.6 | Cached eligibility with revision and generation |
| `JobResult`, `Budget`, cost and memory rules | KN7.3, KN7.7 to KN7.10 | Completion coverage and deterministic charges |
| Display nodes, plans, instances and source maps | RPI.5 to RPI.10 | Publication through the view model |
| Exit-record validity; `Position` and `Context` | H11, JI.25 to JI.30; durable encoding FP.30 to FP.32 | Journal-linked crossing, workspace and history operations |

- **LI.3 [D]** These are ownership boundaries, not a claim that each component may call only its neighbour in a list.

| Layer | Input and output | Authority and boundary |
| --- | --- | --- |
| Shell | Resolved command row and arguments to `invoke`; receives receipts, availability and view state | Routes input under DT-M1; mints tokens for explicit commit actions; never decides mechanics or executes an engine call on the UI thread |
| Command layer | Invocation envelope to engine services; receipts and availability back | Owns dispatch, gates, identity deduplication and lane admission; all mutation enters here |
| Engine | Immutable inputs to exact operations; validated candidates and complete revision bundles out | Owns the state of record, previews, kernels, value store and mathematical gate verdicts; never runs language code or writes files itself |
| Session store | Engine-validated transaction to durable action, state and receipt records; immutable query pages back | Owns persistence, recovery and action order; interprets no mathematics; replies only after releasing its lock |
| View model | Complete engine bundles and separately validated program publications; observable view state out | Adopts a matching bundle atomically, without waiting for a program; never changes puzzle state or computes a permission |
| Renderer | Adopted bundles and adapter-validated plans in; frames, presented-preview reports, picks and device reports out | Draws all mechanical identities unchanged; uncommitted preview/derived subjects never become the session state |

- **LI.4 [D]** The path is shell to command layer to engine, with the engine coordinating a store transaction when
  needed. Results go to the view model and through the renderer adapter. Store queries still enter through the command
  layer. The runtime is a client of those same services (ONE.67); it cannot open session files or invoke commit rows.
  A shell-side scheduler runs an admitted checked overlay program off the UI thread. The engine receives its data
  requests, never a callback into that program (KN0.7). This holds in every LQ9 placement.

## 2. Submission and the publication boundary

- **LI.5 [D]** Every new invocation returning `Receipt` carries a common envelope: session, originating engine
  epoch, unique `CommandId`, stable `RowId`, absolute typed arguments and request digest. The digest covers the row,
  resolved arguments and their guards, pinned source/module/word identities, and any declared budget or grant
  references. Computed results are output, not a change to the submitted request. A token's binding is covered;
  its secret authenticator is not persisted. The engine checks the digest against the received content. Byte
  encoding belongs to p3. A new press has a new command ID; a transport retry keeps
  the entire envelope. A key, a relative grip or an unbound menu index is resolved before this point (EV4.3).
  Outcome lookup returns an existing record and creates no invocation. Finalization of an admitted job keeps
  that invocation's input envelope and identifies its output separately.
- **LI.6 [D]** A commit token binds the session, epoch, invocation, row, absolute arguments and identity. A word commit
  also binds the presented preview ID, its literal digest and `PreviewDeps`. A no-preview commit binds the state and
  protection revisions used to resolve its action; undo/redo also bind their ActionHint, and restore binds its checked
  input digest. Final checks use those bindings. No token grants permission after its epoch, and an identical
  delivery retry is answered from the receipt before token-use checks. A new invocation with a used token is refused
  as `KN5:stale.token_used`. Programs never receive the token or its signing secret.
- **LI.7 [D]** Dispatch validates identity, caller rights, input form, known gates and atomic lane admission before
  acceptance. A rejection is `refused(Error)` and has no effect. Long validation and computation proceed after an
  `accepted` or `running` receipt; their errors are `failed(Error)`, with the original typed code and no publication.
  A standalone view publication whose payload already exists validates that payload before acceptance; refusal
  supersedes nothing. A scheduled view validates its generated payload on its original invocation after acceptance.
  A final validation error fails that invocation, without another admission or reviving work it superseded (LI.25).
  A request accepted against a hint still rechecks its effective grants, dependencies and gates at publication.
  Expensive work uses an immutable snapshot. It never runs under the session write lock.
- **LI.8 [D]** Final dependency checks and the store write share one serialization point across all session mutations,
  including workspace/protection writes. A workspace write cannot race between a preview recheck and commit. The
  commit lane is held from admission to established durable fate; background readers hold no lock it needs. Under
  the short store transaction the engine checks the expected current inputs, records the complete validated change
  and terminal receipt, and releases the lock. Only then may it adopt/publish the new bundle and send a response.
  A proved rollback publishes nothing. An unproved durable fate follows KN5.23, never a guessed rollback.
- **LI.9 [D]** `RevisionRef` counters follow KN0.2. Durable solver actions also have one increasing action-order number
  across journal and workspace records, incremented by one per durable action. The shared hint is:

  ```text
  ActionHint = { order: ActionNumber, target: ActionNumber }
  ```

  `order` is the current durable action-order number; `target` is the action to invert. The shell acquires it
  through `kernel.undo_hint(session, direction: undo | redo)`, with the session's engine epoch. The service reads
  the current order and eligible target for that direction atomically and returns one `ActionHint`, or `none`
  when that direction has no eligible target. `session.undo` and `session.redo` carry the hint for their own
  direction and the shell token. Both fields enter the request digest and token binding and
  are rechecked at the write boundary. A changed order or target gives `KN5:stale.undo_superseded`. If undoable
  action 41 is followed by non-undoable action 42, the hint is `{order:42,target:41}`; once action 43 lands, that
  hint is stale even if target 41 remains eligible. Querying a hint never reserves it.
  Direction vector: journal move 40, workspace rebind 41 and non-undoable checkpoint save 42 are followed by
  undo 43 of action 41. At order 43, `kernel.undo_hint(..., undo)` returns `{order:43,target:40}` and
  `kernel.undo_hint(..., redo)` returns `{order:43,target:41}`; checkpoint 42 is eligible for neither direction.
  Both hints become stale when any durable action advances the order to 44, even if their targets remain
  eligible. Separate reads can observe different orders if an action intervenes; each invocation rechecks its hint.
  A workspace action joins H11's common chain with `action=workspace`, its inverse, provenance and changed-record
  references/versions. Its Position before and after are equal; its actual Context change is recorded, and its
  state revision stays fixed. This action tag describes the effect, not the command kind. A non-undoable durable
  action records the absence of an inverse and still advances the order; checkpoint save is such a commit row
  with a non-puzzle effect (JI.27). The next transition starts at that actual Position and Context, including after an
  undo. Views, queries and availability reads add no action. No constant-time status request enumerates checkpoints
  or the full journal (R-13).
- **LI.10 [D]** Successful publication has the following meaning. A row's kind, not its lane or duration, selects it.

| Kind or service | Successful terminal | Publication point | Terminal retention |
| --- | --- | --- | --- |
| `commit` | `committed(CommitRecord)` | Durable state/action/receipt transaction | Durable, including admission and request binding |
| `workspace` | `completed(kind=workspace)` | Durable workspace/action/receipt transaction | Durable, including admission and request binding |
| `query` | `completed(kind=query)` | Complete typed result made available | Through the originating engine epoch |
| `propose` (a query service) | `completed(kind=preview_ready)` | Immutable preview installed in the pending slot | Through the originating engine epoch |
| `view`, including `publish_view` | `completed(kind=view_published)` | Complete validated view change installed atomically | Through the originating engine epoch |
| `file` | `completed(kind=file)` | Validated output atomically published to the granted destination | Through the originating engine epoch; the file is not a receipt |
| `app` | `completed(kind=app)` | Requested control action registered | Through the originating engine epoch |

  `Completion.revision` is the revision read or the revision at publication, as applicable; it is not a promise that
  this revision remains current. Result-bearing successes include a `ResultRef`; control actions without a result
  may omit it. A file job stages output until publication; interrupted staging is never a successful export. The
  file publication mechanism and bytes belong to the format/platform contracts, not a new session-state path.

## 3. Kernel services supplied to the command layer

- **LI.11 [D]** `open_snapshot` and `close_snapshot` use KN2.2 and ONE.14 to ONE.19. Opening `current` captures all
  revision fields atomically. A handle is bound to its owning run and epoch and pins that immutable input; closing
  it, ending the run or ending the epoch releases it. Historical opening resolves the stored session, event and
  digests, including the requested workspace/protection version, or returns `KN2:stale.revision_gone`. It cannot
  substitute current workspace data for an old observation. Reads of two fields through one handle agree even
  while the head changes. A saved `RevisionRef` can be reopened; a saved handle cannot.
- **LI.12 [D]** KN2.4's batch calls take a `StateRef`, puzzle/level, canonical selection and explicit parameters.
  Results name the input reference and digest, parameters and coverage. A whole-table result uses canonical ID
  order; a selected result names each selected ID, so chunking cannot change object identity. Partial or undefined
  answers use KN0.4, and verdicts use KN0.5. Batch metadata declares exact input sizes, cost-table revision, cost
  function and cancellation chunk boundaries before admission. No per-sticker interpreter callback is permitted.
  p1 supplies the exact jumbling row fields; this draft supplies their common envelope and lifecycle.
- **LI.13 [D]** `derive`, `compare` and `project` use KN2.11 and ONE.20 to ONE.30. The value store holds immutable
  values and their dependency closure by typed digest. Open references pin the inputs needed to reconstruct them,
  charged under KN7.9. Eviction can remove materialized arrays, not the meaning of a live reference. Recomputing
  costs nothing extra beyond the logical call charge. A saved derivation whose inputs are absent returns
  `KN2:stale.value_unavailable`, never a guessed state or a changed reachability claim. Raw imported values keep
  their validated source bytes/digest and open or promised reachability. No value-store operation adopts a state.
- **LI.14 [D]** The KN4.3 query rows have the following logical results. Each response carries the captured revision;
  pages additionally carry an opaque cursor fixing that read version, ordering and next position. A continuation
  cannot silently move to newer data; an unavailable captured version gives `KN2:stale.revision_gone`. A page limit
  and output budget bound the response. These are service fields, not storage encodings or new H-08 row definitions.

| Query | Required arguments | Result fields and scope |
| --- | --- | --- |
| `journal` | Session, starting event, page limit, optional cursor | Event and parent IDs, action order, kind, before/after references, literal-witness references and episode/exit links; chronological entries on the named line |
| `revision_info` | `RevisionRef` | Resolved identity, head, counters, level/digest and available checkpoint/episode references; exact historical input or an error |
| `checkpoints` | Session, page limit, optional cursor | Checkpoint ID, recorded revision/digests, witness/certificate references and recovery role; listing does not restore |
| `workspace_records` | Kind, typed filter, page limit, optional cursor | Record IDs, monotone versions, content digests, provenance and undo metadata at one captured revision |
| `outcome` | Command ID and request digest | KN5.21 receipt under LI.30; this lookup neither invokes nor retries the command |
| `job_result` | Command ID and job ID | KN7.3 result with matching identities, input, parameters, coverage, budget and value |

- **LI.15 [D]** Independent `check` uses KN6.1's certificate reference and checker kind. Its report names the checker
  source digest, certificate-byte digest and per-claim trace. The checker resolves immutable inputs before checking,
  recomputes without the producer's search/construction code and never adopts a producer verdict. Missing required
  input gives `not_applicable(missing_dependency(digest))`, not `pass` or a refutation. A malformed supplied input
  follows the checker's typed evidence error. The complete claim/checker/dependency matrix and archive coverage
  remain p5's input; this service rule does not establish that every promised checker can run.
- **LI.16 [D]** Every job declares KN7.7's deterministic budget. Each batch is admitted against its quoted cost
  before execution, charged as if uncached, and checks cancellation at its declared chunk boundaries. Charges,
  retained-value lifetimes and child accounting use chapter 09 and p4's contract; a nested call cannot reset the
  parent's allowance. H8 supplies canonical uncompressed output bytes and a separate terminal-status reserve,
  which carries no user output. p3 owns those bytes; p4 owns tariffs and stopping positions. No numeric tariff,
  throughput, latency or program-slot performance claim has been measured by this draft. An implementation needs
  the versioned tariff before admitting the corresponding call. Safety-stop seconds never enter a reproducibility key.
- **LI.17 [D]** An effectful overlay invocation stages its workspace edits as one action until it can publish them
  with `completed(workspace)`; a failure or cancellation before that point discards them. Preview construction and
  view publication similarly expose only complete results. Publication settles that invocation: work after it is
  notification/refresh work and cannot turn success into failure. A runtime fault leaves the engine's already
  completed preview and all durable actions intact. This defines the service transaction boundary; the later
  language-kernel annex must map language effects to it without introducing early partial writes.

## 4. Availability without language callbacks

- **LI.18 [D]** The command layer asks the engine for `availability(row: RowId, args, revision)` and receives
  `available` or `unavailable(Code)`. The supplied revision must identify the observation being queried; if it is no
  longer current, the answer is `unavailable(KN5:stale.revision_changed)` and the client reads the current table.
  This is an obsolete hint request, not a preview failure. For current inputs the answer uses the row's gates,
  caller context, effective grants, lane capacity and the row's freshness predicate (LI.19, LI.41). A table entry
  is eligibility on the resolved inputs available now; it does not certify a future program-computed word or
  replace expensive exact validation. `invoke` always checks the actual arguments and current inputs again.
- **LI.19 [D]** The engine supplies KN5.6's table for the shell's registered row/argument bindings and context. The
  command layer stamps its revision, context digest and generation atomically with those answers. Entries are
  keyed by row and absolute-argument digest, so two bindings of one row cannot share the wrong hint. A read for
  another set of arguments uses LI.18 and caches that binding, not an answer for another argument. Search keystrokes only filter
  this local table. `fresh_preview` uses KN0.8 in both this table and commit step 6. For a preview at state 7,
  workspace 10, protection 2, saving an unread keymap at workspace 11 leaves `fresh(p) = true`; the generation
  changes, but the commit is still eligible if its other gates pass. With no pending presented preview the gate
  returns `KN5:stale.no_fresh_preview`; with one whose dependencies changed it returns
  `KN5:stale.revision_changed`. At invocation, token and pending-ID checks still precede the dependency check.
- **LI.41 [D]** `filter.apply` alone uses `fresh_filter_preview` and `preview=filter`, with KN0.9's
  `filter_fresh(f)` bound to its submitted `FilterPreviewRef`. It uses no pending word-preview slot and requires no
  presentation. A stale filter reference returns `KN5:stale.filter_preview`. A current filter at state 7 and
  resolved filter F applies with `completed(workspace)` when the other gates pass, even with no word preview.
  Changing the state to 8 or a record used to resolve F makes that reference stale and application is refused with
  that code. An unread workspace edit alone does not. Row metadata uses `preview=required` exactly for
  `fresh_preview`, and `preview=filter` exactly for `fresh_filter_preview`; the predicates are not interchangeable.
  Alias vector: resolve legacy `filter-apply` and direct `filter.apply` to the same submitted `{preview:f}`.
  With no presented result or pending word preview, both return `completed(workspace)` at unchanged KN0.9
  dependencies, including after an unread workspace edit. In separate runs from that same input, advance state
  from 7 to 8 or change a record read to resolve F before invocation: both return
  `refused(KN5:stale.filter_preview)` and publish no workspace change. Alias resolution adds no presentation gate.
- **LI.20 [D]** Overlay availability is a data-only conjunction: the stored static verdict passes, its grants and
  own row gates/lane permit admission, and every required kernel dependency has the hint below (KN5.7). Required
  bindings are stored as:

  ```text
  requires: [ { row: RowId, args: { static: Digest } | computed } ]
  ```

  A static entry uses the captured `(row, argument digest)` availability answer. A computed entry uses an
  argument-independent hint from that row's gates only; it does not select a cached argument binding. Gates that
  need the computed arguments wait for `invoke`, where all actual arguments and lane admission are checked.
  Optional rows do not participate. Availability never runs the program or reserves a child lane. Report the first
  failed static diagnostic, then a missing grant (`LG3:capability.not_granted`), then an own-row gate failure, then
  the first required-row failure in sorted RowId order (ties use stored dependency order). A missing required row gives
  `KN5:input.overlay_dependency_missing`.
  For a passing overlay whose required menu-word row returns `KN5:input.menu_word_needs_jumble_on`, the overlay
  returns that same code. A busy optional repair-list row alone leaves it available. Registration of `commit` or
  `app` is still refused as `KN5:input.overlay_row_kind`; no availability calculation can admit it later.
  With all other gates passing, `(K,A)=available` and `(K,B)=unavailable(KN2:input.named_object_invalid)` give an
  available overlay for static A and that refusal for static B. A computed dependency on K is available if K's
  argument-independent gates pass; computing B later still gets the input refusal at `invoke`.
- **LI.21 [D]** The advisory event is `availability_changed(session, epoch, generation, revision)`. A generation
  increases whenever an input changes: revision, pending-preview lifecycle/presentation, lane occupancy, registered
  bindings/context, overlay/static-verdict table or effective grants. A full table read returns the newest complete
  generation. Clients discard older events; after a gap, reconnect or an epoch change they read the full table.
  Events may coalesce. The generation is not a token, a state revision or a durable action. Missing an event can
  leave a stale display hint but cannot admit an operation. A renderer presentation or device-loss report updates
  the preview's presentation gate through the adapter; it runs no program (RPI.10, ONE.41).

## 5. Lanes and undo

- **LI.22 [D]** KN7.2 defines all six lane admissions, including the result for a second request. Atomic admission
  occurs after duplicate-ID lookup, so a retry consumes no second slot. The scope of commit, exclusive and program
  capacity is a session. Every `view` or `latest_wins` row declares `lane_target`, one of
  `{"arg":"<argument naming the ViewId>"}`, `{"view":"<fixed ViewId>"}` or
  `{"control":"<session control target>"}`. The command layer resolves it before admission and includes the
  resolved target in the request binding. View arguments and fixed ViewIds resolve to the same view namespace;
  control targets have a separate namespace. Requests share a replacement lane exactly when their session and
  resolved target are equal, even across row IDs. Runtime publication uses the scheduled invocation's target.
  Thus B for view V supersedes unfinished A for V under LI.23, while C for view W can run independently. A control
  target named V is distinct from view V. The program capacity is a positive integer declared by the host's
  admission configuration and returned with availability. It is fixed
  for admitted jobs, not derived from measured seconds. For a declared capacity of 2, two running programs are
  admitted and a third new invocation is `refused(KN7:busy.lane_full)`; undo can still enter the commit lane.
- **LI.23 [D]** A latest-wins or view lane holds at most one runner and one newest pending request. A newer admitted
  request fences off every older unfinished request in that lane and settles each as
  `cancelled(superseded(by=new_command))`. Their results cannot publish after the fence. Cooperative cleanup stops
  the old runner at a chunk boundary; a replacement starts only after its runner slot is released. The terminal
  descriptor keeps the last acknowledged coverage at the fence; later cleanup adds no output. Another arrival
  replaces the pending request, not a growing queue. Already completed invocations retain their success receipts.
  A refused new request supersedes nothing. Logical cancellation does not erase cleanup memory or grant extra capacity.
- **LI.24 [D]** A solver-invoked non-view overlay program uses `program`; an overlay of kind `view`, including
  revision-driven views and their publication, uses `view`. Kernel child operations retain their own row gates and
  applicable admission rules, with the same run's budget accounting. A program slot cannot confer a commit slot.
  Short `none` operations still use the common
  validation and transaction boundary. `job.stop` uses `none`: its `completed(app)` acknowledges the stop request,
  not the target's outcome. The target later reports `cancelled(by_user)`, or keeps its success if publication won.
- **LI.25 [D]** At a state commit or puzzle undo, the coordinator immediately fences jobs on replaced dependencies
  and signals cancellation; it never waits for their cleanup to commit. File jobs finish from their captured
  inputs. Pinned jobs finish on their pinned inputs (KN7.5; owner, 6 November 2026, LQ4). A view records only the
  newest invalidation, recomputes only while visible (RPI.11), and rechecks dependencies plus its lane generation
  at atomic installation. An obsolete
  publication is `cancelled(replaced_revision)` or `cancelled(superseded(by=...))`, according to the first fence.
  A hidden view retains the invalidation and runs no program. Its old output never attaches to new session geometry.
  For a scheduled view, `publish_view` finalizes that same command, job, request and lane generation; it does not
  acquire a second view slot or supersede itself. Its generated plans are output under LI.5. A separate host
  publication names its target ViewId in the request and carries already prepared plans. It validates before admission
  under LI.7, then uses ordinary replacement admission for that view only. With an instance ceiling of 4, standalone B carrying 5 instances is
  `refused(EV3:resource_limit.display_plan_ceiling)` and pending A is untouched. If scheduled B was accepted and
  superseded A before producing 5 instances, B ends `failed(EV3:resource_limit.display_plan_ceiling)` on its original
  command/job with `JobResult.outcome=error`; A stays `cancelled(superseded(by=B))`. There is no second admission,
  installation or resurrection. An invocation already fenced by a newer request keeps its cancellation instead
  of accepting a late validation result.
  Successful installation returns `completed(kind=view_published)` for that command/job, with the captured revision
  and a result reference naming the ViewId and installed publication digest.
- **LI.26 [D]** `session.undo` and `session.redo` are the sole solver entries for their directions in the durable
  action order. The shell first calls LI.9's `kernel.undo_hint` with `direction=undo` or `direction=redo`
  respectively; `none` supplies no invocable target. Each takes `hint: ActionHint` (LI.9) and `token: UserActionToken`, uses `action_guard=action_hint`,
  the commit lane and no `not_busy` gate. A changed order or target returns
  `KN5:stale.undo_superseded`. The engine dispatches the journal or workspace inverse under that action's original
  scope and the normal permission path. Releasing protection by undo needs the same explicit `confirm` as releasing
  it directly; absent confirmation gives `KN5:input.confirmation_required`. No separate workspace key or focus
  dependent undo class is introduced. The durable inverse, counter changes and `committed` receipt are atomic;
  a workspace inverse keeps Position and puzzle hashes equal, including head and state revision, while advancing
  workspace and affected record/protection versions. Both rows use `slot_map: by_target`: a journal action preserves
  the current slot map; a workspace action that recorded one restores that map. Undoing a `grip.rebind` from map A
  to B at workspace 10/record version 4 therefore restores A at workspace 11/record version 5, returns `committed`
  with equal pre/post puzzle hashes, and leaves head/state unchanged. Redo restores B with another workspace/version
  increment. Other current context stays under JI.27; no whole historical Context is assigned.
  It cancels jobs whose declared inputs it replaced, preserves unrelated drafts and file jobs, and invalidates
  affected preview dependencies. Text undo remains the editor's. The row-table dispatch is internal and introduces
  no extra command per class.

## 6. Results, loss and display

- **LI.27 [D]** KN5.21's success forms cover every invocation kind. The receipt, result and any job have the same
  command/request binding. `running(job)` and the terminal receipt carry the same `JobId`; `job_result` checks both
  identities. Preview summaries name the preview ID and literal digest; query results name the exact input and
  coverage; workspace results name the action and changed records; view results name the ViewId and installed
  publication digest; file results name the output digest and granted destination identity. None is a CommitRecord.
- **LI.28 [D]** A job settles according to this table. A partial value or checked witness is not a completed negative
  answer, and no partial display plan is installed. The fixed terminal reserve survives output-budget exhaustion.
  This is the only job-outcome-to-receipt mapping. `completed(query)` may carry a partial typed value only if its
  result type declares partial coverage as a normal outcome within the grant. That is `JobResult.outcome=completed`,
  not a cut-off of the invocation; it claims nothing about uncovered inputs. An invocation that exhausts its own
  deterministic budget is `cancelled(budget)`, even when it retains useful observations or checked positive witnesses.

| JobResult outcome | Receipt status | Meaning |
| --- | --- | --- |
| `completed` | `completed(Completion)` for non-commit work | Complete result or publication of the stated kind |
| `cancelled` | `cancelled(by_user / superseded / replaced_revision)` | No publication by this invocation |
| `cut_off(KN7:budget.steps / kernel_cost / memory / output)` | `cancelled(budget)` | Deterministic incomplete coverage, with the exact code in JobResult |
| `cut_off(KN7:resource_limit.safety_stop)` | `failed` with that error | Incomplete and not reproducible; not a deterministic budget cancellation |
| `error(Error)` | `failed(Error)` | Runtime or host error; for runtime death use `KN7:resource_limit.runtime_lost` |

  A commit row that finishes asynchronous preparation still has only `committed(CommitRecord)` as success. A
  cancellation racing its durable transaction is resolved at LI.8: rollback permits cancellation, durability
  requires the committed receipt, and uncertainty requires recovery. No success becomes cancellation afterwards.
  Error forms follow the occurrence: `EV3:busy.renderer_unavailable` before acceptance has `form=refusal` and a
  refused receipt. Loss during accepted preparation has `form=stop` inside `JobResult.outcome=error`, with
  `Receipt.status=failed` and the same code. Neither installs a view. Loss after installation leaves the original
  `completed(view_published)` intact and is a separate refresh/device report. The registry's primary form does not
  override these contextual forms (H15.9).
  For `jumble.observe` that exhausts output after seam coverage `[0,2)`, the receipt is `cancelled(budget)` and
  `JobResult.outcome=cut_off(KN7:budget.output)`, with `[0,2)` retained and total_N undefined. Same-epoch
  `outcome(command,request)` returns that cancellation unchanged; after epoch loss LI.30 returns `unknown_outcome`
  if no transient terminal was retained. Partial coverage never becomes a complete empty answer.
- **LI.29 [D]** Non-durable receipts and job-result descriptors remain queryable until their epoch ends. A large
  result payload may be released independently; reading it then gives `KN5:stale.result_gone`, without changing the
  terminal receipt or re-running the command. A durable receipt likewise proves its action even when its payload
  is not resident. `completed(preview_ready)` never implies that its preview is still pending, presented or fresh.
  `completed(view_published)` proves installation in the view model, not that the renderer presented a frame.
- **LI.30 [D]** Outcome lookup has no side effects and takes no job lane. It follows this order:

| Lookup condition | Answer and continuation |
| --- | --- |
| Command exists with another request digest | `refused(KN5:input.command_id_reused)` for the lookup; original receipt unchanged |
| Matching terminal exists | Return it unchanged, including after undo or a refresh failure |
| Matching active invocation in this epoch | Return its `accepted` or `running` receipt; no duplicate work |
| Durable admission survived a crash, effects and terminal are proven committed atomically | Return recovered `committed` or `completed(workspace)` |
| Durable admission survived, complete recovery proves no publication and rollback | `failed(KN5:resource_limit.engine_lost)`; no automatic retry |
| Terminal/admission proof is missing or recovery is incomplete | `unknown_outcome`; possible session mutation puts that session in read-only recovery |
| Transient invocation's epoch ended and no terminal is retained | `unknown_outcome`; query/view loss alone does not freeze session mutation |

  A never-observed ID also looks up as `unknown_outcome`, not success or proof that nothing happened. An invocation
  from an ended epoch with no durable record is never executed by resubmitting it. A new action after recovery must
  have a new identity and current guards. An uncertain file outcome is inspected through its granted destination;
  it is never automatically exported again. Current-epoch live receipts are authoritative even if their notification
  was lost. The store is the authority for durable terminals, not the existence of a frame, a file or an equal hash.
  Every durable transition links its command, request, admission, receipt and resulting RevisionRef under JI.27.
  Undo appends its own action and terminal; it never edits the original receipt. If crossing C committed at state
  revision 32, its notification was lost, and undo U restored its prior puzzle state at revision 33, lookup of C
  after restart still returns C's original `committed` receipt at revision 32, not U's revision or a new crossing.
- **LI.31 [D]** Session-bundle adoption and program publication are separate. RPI.10 installs matching nodes, plans
  and source maps as one set; an invalid or incomplete set replaces none. Preview and derived-state plans retain
  their distinct subject, base and derivation (ONE.21, ONE.62); drawing them never changes the session head. After
  device loss, lack of a presented preview disables the commit chord, but a committed receipt stays committed.
  The adapter's details and feature support belong to p6. No layout, dwell time, colour or new UI is selected here.

## 7. Main-line contract traces

These are specified outcomes, not executed product tests. Symbolic P and L below are typed pose and label digests.

- **LI.32 [D]** Live twist: at state 7 and protection 2, with no commit in flight, the shell resolves a permitted
  retained letter and invokes `twist.apply` with a fresh action token. The engine validates the letter and full
  start hash under active protection, the store records one event/receipt, and state becomes 8. The receipt is
  `committed`; the view model adopts that result before the renderer draws it as the session state. An active
  analysis job at state 7 does not refuse the twist; it is fenced and cancelled. A live twist has no preview
  (ONE.78; owner, 6 November 2026, decision 10).
- **LI.33 [D]** Word path: proposal command Q (no token) freezes and replays the literal word at state 7,
  workspace 10, protection 2. Q returns `completed(preview_ready)` with preview p and its KN0.8 key. The adapter
  reports p presented. Unread keymap-save command K (no token) raises workspace to 11 only; the engine's table
  and commit step 6 still accept p's key. While p is presented, two distinct explicit solver actions mint
  `operation.execute` envelopes C with token TC and D with token TD, each bound to its own command and p.
  Hold D undelivered, so TD stays unused. Deliver C/TC: it commits one event at state 8 and discards pending p.
  Undo command U with fresh token TU and its current undo ActionHint restores the start hash at state 9.
  An identical-envelope retry C/TC returns C's original committed receipt before token checks (LI.6).
  A new invocation N using consumed TC fails step 3 with `KN5:stale.token_used`. First delivery of D/TD,
  still in the same epoch with the commit lane free, passes the unused-token check but fails step 5 with
  `KN5:stale.preview_replaced`: p was discarded. Equal hashes at state 9 cannot restore KN0.8 freshness.
  In a separate branch before C or D is delivered, draft-edit command E (no token) changes a read record, or
  protection-edit command P (no token) changes protection to 3, while p remains pending. Delivery of D/TD then
  fails step 6 with `KN5:stale.revision_changed`. These are gate-order outcomes, not two definitions of freshness.
  Every refused invocation leaves the current state unchanged.
- **LI.34 [D]** Long query: a named-cap request gets `accepted`, then `running(j1)` in the exclusive lane. A second
  new exclusive request is `refused(KN7:busy.lane_full)`. If j1 completes, its result and
  `completed(query)` receipt name the same input digest, parameters, coverage, command and j1. Only a completed job
  covering its declared set can report an empty answer. A runtime failure instead returns `failed(KN7:resource_limit.runtime_lost)`
  with `JobResult.outcome = error`. Neither branch changes the puzzle head or writes an undo action.
- **LI.35 [D]** Concurrent undo: with a query j1 reading state 8 and the hint naming action-order 12's puzzle move,
  `session.undo` enters the free commit lane, validates that hint and the inverse, and records its receipt with
  state 9 and the prior puzzle digest. j1 is `cancelled(replaced_revision)`; a file export keeps running. No job
  cleanup is awaited. If action-order 13 landed before the undo check, the result is instead
  `refused(KN5:stale.undo_superseded)` with no inverse written. If another commit holds the lane, it is
  `refused(KN5:busy.commit_in_flight)`, not queued.
- **LI.36 [D]** Crossing: for witnessed pose digest P at a label checkpoint, Jumble off commits the transition
  validated under JI.27 to labels L and stores its exact exit poses and certificate with the episode. The row follows
  ONE.80's token, lane, checks, atomic transaction and bundle path. Undoing that crossing follows ONE.81 back to P, not a lift
  from L. Entry is a separate action: after retained w at h9, `exit_root(E,e,digest(w))` enters at replay(P,w),
  and undo of this entry restores the off Position at h9. Redo restores the entry's own after Position. A
  `solved_witness(v)` entry after import uses its own off/on sides and needs no E (JI.29). Neither path assigns a
  historical Context: current preferences and protection stay except the crossing's recorded protection-level
  change, reversed/reapplied with it. Branch navigation uses JI.28's proved journal steps, keeping current context.
  The common-chain vector starts with exit action 40 at head e, state revision 32, workspace 10, protection 2 and
  context protection P0/preferences B. Workspace action 41 adds protection P1: Position stays equal, workspace
  becomes 11 and protection 3. Retained move 42 changes head to m and state to 33; E's root remains P. With current
  hints, valid tokens and required confirmation, undo 43 of move 42 restores head e at state 34, keeping P1/B and
  workspace 11/protection 3. Undo 44 of workspace action 41 restores protection content P0 at workspace 12/protection
  4, keeping head e/state 34 and preferences B. Both undo receipts are `committed`; the workspace inverse has equal
  puzzle hashes. All five transitions link actual Position and Context without a gap; recovery gives that final
  tuple, the same unvoided E/root P and an empty retained suffix. Each action has its own command/request,
  admission/terminal references and resulting RevisionRef. Action 41's original `completed(workspace)` remains
  queryable after the undo and restart, as does the crossing's original committed receipt (LI.30).
  A restore of L at an earlier retained head instead follows ONE.82 and voids every earlier exit, active or inactive,
  even when labels agree. Undo of that break or branch selection cannot erase the void. Recovery keeps it;
  Jumble on without a valid retained witness from solved is refused as `KN5:input.jumble_on_labels_only`.
  A proved rollback restores the prior complete tuple; uncertain durable fate enters read-only recovery. These
  traces consume JI.25 to JI.30 and H11; FP.30 to FP.32 encode the same bindings and both field groups.

## 8. Freeze-input walk and open choices

- **LI.37 [D]** This is the walk of every PHS.48 input row on this packet's candidate. A partial row cannot support
  an architecture-freeze ruling merely because these layer clauses exist.

| PHS.48 input | What this draft supplies | What it still lacks and who supplies it |
| --- | --- | --- |
| Authority and scope | Explicit draft status; DT-M1 source; the owner's answers of 6 November 2026 (cut, LQ18 permission, LQ19 order and amendment); LI.38's open choices | Accepted/conditional ADR disposition: owner and integrator |
| Architecture and interfaces | Main-line six-layer contract; RevisionRef, PreviewDeps and Receipt; availability, jobs, undo, program/view lanes and reconciled chapters 01/08 | Final EN3 interfaces and transition rows (p1); built final H-08 and coverage join (p2a); p3 format packet; second-wave language-kernel annex and integrated trace review (integrator) |
| Renderer feature list | Data boundary, distinct derived/preview subjects, publication terminal and presented-preview dependency | p6 support/disposition matrix and adapter contract; renderer-line presented-frame evidence; handoff row H-11 and selection evidence |
| Decisions and evidence | Source-level contract vectors in the p2b manifest and requirements below; no measurements | p4 exact accounting vectors/tariffs, p5 language/checker contracts; applicable spike results; evidence-led owner decisions and product conformance remain open |
| Review and limitations | This packet's manifest, explicit partial closures and scope limits | Current assembled-candidate review/dispositions, evidence-class audit and N1 status from integrator; neither N1 review nor a freeze ruling is supplied here |

  PHS.13's snapshot, batch, value-store, preview/token, receipt, availability, store-query, checker and job/budget
  interfaces are covered by LI.5 to LI.31 with their chapter-01 definitions. The exact per-row language binding,
  scheduler tariffs, byte encodings and missing checker dependencies are explicitly separate inputs. FR-K01 and
  FR-C01 therefore remain partial as whole findings; this packet does not certify final EN3/EN4 delivery.
- **LI.38 [O]** The owner answered LQ1 to LQ21 on 6 November 2026 and chose LQ8, LQ9 and LQ21 on 14 November 2026:
  Rust, route RT1 with the runtime in a process of its own, and capability separation (language chapter 11, RTE.17,
  RTE.21, RTE.24). LQ13 waits for its evidence, and LQ2's option set waits for the joint setting. The rows below
  mark what stays open. Working forms support the traces, never replace owner answers.

| Question | Working form used here | Change under the alternative |
| --- | --- | --- |
| KN5.12 / ONE.42, shown-preview replacement | Traces use a solver-initiated proposal and no unsolicited replacement of a shown preview | Solver-only replacement refuses unsolicited replacement; if timed replacement is chosen, the selected presentation delay is another eligibility gate. No delay value or default is chosen here; token and freshness contracts are identical |
| LQ9, runtime placement (answered: a process of its own, RTE.21) | Interfaces stay placement-neutral | The runtime's process must still isolate tokens, UI and the commit path and pass LX-2's fault/latency checks, now an LP3 acceptance check; a failure reopens LQ9 |
| LQ21, provenance and boundary policy (form answered: capability separation, RTE.24) | Carry opaque versioned provenance and current grants; the enabled rules wait for LQ2's joint setting | The data core serializes these references losslessly (owner, 6 November 2026, LQ10); the selected policy changes grant/row eligibility. Neither turns stored evidence into a token |

  ADR acceptance remains an owner item in LI.37. This draft starts no experiment, product implementation,
  design-table topic or UI/UX work.

## 9. Conformance obligations and integration

- **LI.39 [D]** These are later test requirements, not passing test results. The packet's closure check validates
  clause references and concrete vector fields only.

| Test | Checks | Class |
| --- | --- | --- |
| CT-LI-01 | Availability and commit share KN0.8 for unread-record edit, read-draft edit, protection change and undo to equal labels; versions never revive permission | source |
| CT-LI-02 | Every invocation kind has the terminal in LI.10; a lost notification returns the same outcome, and transient epoch loss returns unknown | source |
| CT-LI-03 | Concurrent requests obey all six lane admissions; a superseded runner cannot publish and a replaced pending request cannot run | source |
| CT-LI-04 | Undo during a query succeeds, fences that query, preserves a file job, rejects an obsolete hint, and checks protection release through the usual gate | source |
| CT-LI-05 | Crossing, crossing undo and equal-label restore commit complete tuples or none; restore's durable void survives recovery | source |
| CT-LI-06 | A completed preview is not presentation permission; view installation is atomic, and derived plans never change the session head | source |
| CT-LI-07 | Lost/coalesced availability events converge by full reads; optional overlay rows do not block and required rows do, without executing language code | source |
| CT-LI-08 | Every PHS.48 row identifies supplied and missing artifacts on the actual candidate; a missing parallel input cannot count as delivered | source |
| CT-LI-09 | Shared undo/redo take ActionHint order and target; order 42/target 41 is stale after 43 lands; rebind inverse restores its map with committed/equal hashes and advanced workspace versions | source |
| CT-LI-10 | A complete protection-conflicting word gets completed(preview_ready), while its otherwise valid commit is refused; a mechanically refused prefix still builds no preview | source |
| CT-LI-11 | Current FilterPreviewRef applies without a pending word preview or presentation; state/filter dependency changes give KN5:stale.filter_preview, while unread workspace edits preserve freshness | source |
| CT-LI-12 | Overlay static A/B bindings use their own answers; computed dependencies use only argument-independent gates; resolved lane targets make same-target requests supersede and different targets independent | source |
| CT-LI-13 | Ceiling 4/five instances refuses standalone publication without supersession, but fails an accepted scheduled invocation without reviving its predecessor; renderer loss uses refusal/stop by occurrence and preserves installed receipts | source |
| CT-LI-14 | Output-exhausted jumble.observe keeps coverage and cancelled(budget) through same-epoch lookup; typed partial success stays within the grant; original durable outcome survives undo/restart with its original revision | source |

- **LI.40 [D]** Integration must pin this draft with the p1/p2a/p3/p4/p5/p6 candidates, reconcile chapter 13's changed
  receipt/error meanings and register the manifest's new codes, and add these requirements to chapter 12's phase
  index (CT-LI-08 at LP1; the other rows at LP2). ADR-L01/L02/L08 and chapter 04 still contain historical
  pending-EN4 wording and need the integrator's reconciliation; none is edited by p2b. Then build and test the real
  command-table join, run the all-packet closure check and H13's second-wave traces, and obtain the required plan
  check/current-candidate review. The supplied packet check is:

  `python docs/architecture/1.0/language/closure/check_closure.py --packet p2b`

  After the other drafts land, the integrator runs `python docs/progress/1.0/build_command_table.py`,
  `python -B docs/progress/1.0/test_command_table.py` and
  `python docs/architecture/1.0/language/closure/check_closure.py --all`.
  No final-table build, independent review, renderer run, performance measurement or owner ruling is claimed here.

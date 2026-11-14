# Language-kernel interface annex

Status: **draft architecture input for FR-B03; not accepted or frozen**. This annex selects no route,
engine language, placement, syntax, ABI or wire format, answers no LQ, and makes no zero-copy claim.

- **KI.1 [D]** [operations.json](operations.json) is the typed catalogue for the two traces in
  [walks.json](walks.json) and the observations left open by LB3.9. Names describe semantic operations,
  not callable product APIs. Existing types keep their defining clauses. A catalogue entry's `rows`
  names its dispatch row, supporting service registration, or consuming command, as `row_use` states.
  Library helpers and batch continuations do not acquire another command lane. The nine service or
  continuation entries in `requires` resolve in H-08's `services` list (CM.16); `jumble.observe` resolves
  in its `commands` list. A `requires` entry cannot supply a missing registration. Services have no key,
  alias, availability or undo entry and grant no permission; they are outside DT-M1 C1's command count.

## Forms relied on

- **KI.2 [D]** These are working forms, not accepted decisions. `sources` in the catalogue names the
  repository inputs read. The checker resolves clause definitions by ID, without freezing moving
  chapters, decisions or tables by digest.

| Form | Definition used | Integration condition |
| --- | --- | --- |
| Identity, revision, state reference, partial result, verdict | KN0.1, KN0.2, KN0.3, KN0.4, KN0.5 | Preserve puzzle and level; unavailable is never zero or false |
| Preview freshness and resolved draft | KN0.8, KN5.9, LI.5, LI.6 | Keep record versions and the actual read set; unrelated workspace edits do not stale a preview |
| Invocation and result identity | KN5.21, KN5.23, LI.27, LI.30 | One command/request binding; host IDs are outside exact payload equality (DET.6) |
| Snapshot and value ownership | KN2.2, LI.11, LI.13 | Epoch/run-scoped handles; historical workspace/protection must match too |
| Terminal outcomes | KN5.24, LI.28, LI.29 | Use the explicit cut-off-to-cancelled mapping; JI.15's older completed-query wording needs reconciliation |
| Lanes and undo | KN7.2, LI.22, LI.26, CM.5 | Shared session undo/redo dispatches by the target action; both ActionHint fields bind the request and token |
| Jumbling payloads | JI.6, JI.7, JI.8, JI.9, JI.10, JI.11, JI.14, CM.15 | Named batches use the final table's delta and amended argument schemas; binding and tariff work remains |
| Meter and lifetimes | DET.54, DET.55, DET.56, DET.57, DET.58, DET.59, DET.60, DET.63; LG4.7, LG4.9 | No production tariff, size ceiling or terminal capacity is supplied by this annex |
| Semantic chunking | DET.63, LG4.6 | JI.14's maximum chunk size is an admission constraint; the row's pinned semantic schedule determines stops |
| Provenance and grants | LG3.2, LG3.4, LG3.12, LG3.13, LG3.16; RTE.24, RTE.26, RTE.27 | Capability separation is selected (LQ21; RTE.24); keep the interim restrictions; a typed signature grants nothing |
| Presentation and publication | RPI.10, RPI.13, RM.14, RM.18, RM.19, RM.26 | Confirmed presentation differs from installation; renderer resources stay host-owned |

## Common contract

- **KI.3 [D]** `Context` means the admitted session/epoch, command/request/job binding where applicable,
  run owner, effective grants, reproducibility inputs, API/row identity, cost-table identity and finite
  `Budget`. Invocation envelopes use LI.5; a continuation keeps that invocation rather than creating
  another one. Library calls use the current run. Every read result has `Binding { identity, level,
  input_digest, origin: RevisionRef | derived lineage, parameters_digest, coverage }`. The immutable
  input digest belongs to the exact payload; epoch-local handles and receipts belong to the host
  envelope (DET.1, DET.4, DET.6). A result is consumed only with matching pins. Exact values never narrow
  to a host integer or float. `Partial<T>` and `Verdict` retain their alternatives, including unknown.
  Live receipt polling is host work. A reproducible language run consumes a permitted captured terminal
  record as a declared immutable input, or its own serial child's result at a fixed continuation;
  notification timing and changing host status cannot choose its computation (DET.4, DET.6, DET.55).

- **KI.4 [D]** The catalogue uses these neutral resource types:

  - `Owner<T>` is a run/scope-owned reference to immutable T. It pins T and its reconstruction inputs.
    A borrowed argument creates no owner and cannot outlive its named owner. An explicit retained alias
    charges the entire payload; a move invalidates the sender and transfers its existing charge (DET.57,
    DET.58). A digest alone is neither an owner nor a permission.
  - `Stream<T, Summary>` owns a cursor, pinned inputs and the semantic schedule. Its cursor alone is
    mutable bookkeeping, inaccessible as puzzle data. Each `stream_next` returns a new immutable
    `Chunk<T> { binding, index, covered: [start,end), items }` and, only when appropriate, a final Summary.
    Chunks are ordered and non-overlapping. References to derived values inside a chunk borrow it until
    `retain_value` makes a charged owner. Releasing a chunk cannot invalidate a separately retained owner.
  - `ReleaseAck { resource, invalidated }` confirms that the logical owner can no longer be used.
    It does not assert physical deallocation or GPU completion. Release is idempotent for the same
    owner identity; it cannot release an alias. Normal release uses `release_value`, or `close_snapshot`.
    Run/epoch teardown performs the same invalidation without executing user destructors.

  In the walk notation, an `Owner<T>`, `Borrow<T>`, `StateRef` or borrowed engine `PreviewId` input
  names its tracked owner directly, or a field/index path rooted in that owner. For example,
  `pc.items[0].prefix_state` borrows `pc`, not an independently live derived owner. The checker resolves
  that root from the actual input and requires it to be live before the step and present in `uses`.
  `uses` may additionally record other retained lifetimes, but cannot substitute a different owner.
  A newly created alias becomes live only after the step; dropping the source leaves that alias live.
  Unretained host context/status values follow the walk's ledger rule and create no tracked owner.

- **KI.5 [D]** Before starting async work, the service pins borrowed inputs for its whole use. They are
  declared job-owned reservations, charged in the same ancestor meters; a caller cannot free memory by
  dropping its reference while the service still needs a distinct owner. Results become usable only
  after admission, complete production and validation. Their bytes remain immutable until every owner
  and in-flight transfer has finished. A transfer identifies owner, epoch, binding, chunk and request;
  the receiver validates them before acknowledging adoption. Lost acknowledgements may cause identical
  delivery, never a second owning alias, output charge, chunk advance or command. No native pointer,
  allocator object, callback or mutable state crosses the logical interface (KN0.7, LI.12).

  For a move, the bridge holds the sender's immutable bytes until receiver adoption or established
  failure, and the logical owner moves exactly once. For a copy/alias, reserve a new logical owner first.
  The bridge owns transfer staging and counts it against its finite physical limits; transport copies
  do not alter deterministic payload charges. A release acknowledgement invalidates remote access;
  physical reclamation waits for the last use or proved process/device teardown. A broken connection
  alone proves neither. Bounded quarantine and admission refusal replace unsafe reuse. Actual transport,
  acknowledgements, handle validation and failure detection remain binding work in KI.15.

- **KI.6 [D]** Each entry supplies `transfer.payload` and a profile below. This expands to all three
  placements of RTE.21; it does not choose the remaining shell/engine topology.

| Profile | Engine-process isolate | Separate runtime process | Shell-process runtime |
| --- | --- | --- | --- |
| `kernel` | Runtime/kernel exchange typed references inside the process; a checked language boundary may still need copying | Requests carry pins, arguments and scoped IDs; replies carry descriptors and requested immutable payloads; retain/release/cancel acknowledgements cross back | Same request/result and lifetime traffic crosses the shell/engine boundary; runtime remains off the UI thread |
| `local` | No runtime/kernel transfer for this helper | No runtime/kernel transfer for this helper | No runtime/kernel transfer for this helper |
| `host` | Shell/adapter and engine exchange the entry's host payload independently of the isolate | Same host path bypasses the runtime process | Host path remains privileged and separate from runtime access even when both share a process |
| `resource` | Local owners release locally; engine owners use the kernel boundary | Local owners release locally; remote owner IDs, retain/release requests and acknowledgements cross | Same rule across the shell/engine boundary |
| `stream` | Local expansion stays local; kernel continuation uses the checked isolate boundary | Local expansion stays local; kernel stream IDs/indices cross out and immutable chunks/summaries cross back | Same distinction across shell/engine, off the UI thread |
| `publication` | Runtime submits typed data to command service; the complete publication crosses to the view-model host if separate | Complete publication and pins cross from runtime to command/view-model host; receipt returns | Command admission still applies; content transfers only if the command/view-model host is separate; adapter data stays host-owned |

  In a `kernel` reply a snapshot itself crosses as a scoped reference and metadata, never an eagerly
  copied mutable state. Only requested arrays/chunks cross. An explicitly proved read-only sharing
  binding may replace copying while preserving KI.5; co-location proves neither sharing nor zero-copy
  (RTE.7, RTE.19). Host commit tokens never enter a runtime payload in any placement (KN5.15).

- **KI.7 [D]** `accounting` in each entry identifies its step events, kernel work, live memory and
  output units. DET.56 orders interruption, step debit and atomic kernel/memory/output admission.
  Equality to a limit succeeds. A denied secondary charge keeps only the admitted step and exposes
  no new value. Variable-size work, hashing, expansion and encoding must have metered continuations or
  a pinned input-size tariff; this annex supplies none. A native aggregate includes its internal work
  once, or declares its decomposition, never both (DET.23, DET.54). A library aggregate charges its
  actual constituent operations, not an extra native aggregate tariff. Children share ancestor
  meters (DET.60). Cache hits, transport packets and reconstruction cannot change charges.

  Live and peak logical memory include inputs, output owners, aliases and declared scratch, independent
  of physical copies. Host-owned previews/publications acquire their own bounded service reservations
  before acceptance; transfer to that owner does not erase the producing job's peak or cumulative use.
  No user value lives free in the terminal reserve. Complete output units use the pinned canonical
  uncompressed byte profile of DET.59; internal returns are moves, and identical delivery adds no charge.
  Top-level returned values, emitted chunks, preview summaries and retained partial results require
  ordinary output charges. Publication ends only after its whole output unit is admitted. Shell key
  dispatch uses no interpreter steps (LG4.13); host service work still has declared limits and costs.

- **KI.8 [D]** A streaming operation pins its semantic chunk rule in its row identity. A caller's
  positive `max_chunk` limits admitted chunks; it cannot silently change the schedule for the same key.
  Word schedules use literal positions. Seam schedules use canonical (domain, cut-cell) order and
  metered enumeration continuations. When discovery cannot quote a finite whole-stream tariff from
  sealed metadata, each growing continuation is separately quoted before work, as DET.57 and DET.63
  allow; it must not pretend to have an upfront total. Geometry safe points and bounds still require
  a production descriptor before admission. Transport split/coalescing does not add semantic events.

- **KI.9 [D]** `stop_job` acknowledges only a cancellation request. The target settles at an
  interpreter continuation or declared kernel safe point; an incomplete chunk gives no complete
  coverage. Before publication cancellation discards staged work, releases job owners and retains only
  previously admitted output owners and bounded status. Explicit caller-owned snapshots/aliases remain
  charged until dropped or their scope ends. At terminal stop record counters and live charge before
  teardown, then clean up without another budget debit (DET.58). Cut-off maps to `cancelled(budget)`
  with its specific code and covered range in `JobResult`; a safety stop is a non-reproducible failure
  (LI.28). No partial enumeration supplies total N, a whole-word conclusion or a negative claim.

  A replaced revision fences ordinary jobs immediately; commit/undo never waits for cleanup. Pinned
  jobs finish on their pinned inputs (KN7.5; owner, 6 November 2026, LQ4). Runtime death releases its run
  owners and errors unfinished jobs,
  but an installed engine preview survives and a durable commit stays committed (RTE.22). A commit
  racing cancellation is settled by the durable transaction: committed, proved rollback, or unknown
  pending recovery, never a guessed failure (LI.8, LI.30). Outcomes and payload lifetimes are separate.

## Observation packaging

- **KI.10 [D]** The following choices consume JI.7 without changing LB3.9. All reads use one pose-level
  `StateRef`; joins require its full binding. Exactness is not a claim that a raw input is reachable.

| Observation | Catalogue operation and result field | Choice and reason |
| --- | --- | --- |
| Total N, including off-grip mismatches | `seam_facts` / `SeamTotals.total_N` | Explicit exact batch under `jumble.observe`; cap summaries omit off-grip cells |
| Exact seams | `seam_facts` / `Chunk<SeamCell>.items` | Same batch; complete domain/cut-cell identities, occupancies, planes and holding caps are required to count and inspect the same objects |
| D | `prefix_observations` / `PrefixObservation.D`, `lowered_high` | Explicit replay observation batch under `operation.check`; evaluated at the supplied letter's actual prefix, not inferred from an endpoint |
| Units | `units` / `UnitTable.groups` | Library aggregation of `pose_facts` and `height_facts`; exact-pose grouping needs no additional geometric decision |
| F | `prefix_observations` / `PrefixObservation.F`, `foreign_pieces` | Same replay batch; exact inside classification and branches before the letter acts |
| Phi | `height_potential` / `phi` | Library sum of max(0,h-2) over pieces using `height_facts`; pose multiplicity cannot be lost |
| Prefix observations | `prefix_observations` / `Chunk<PrefixObservation>.items`, `WordObservation` | Same new batch; replay, seam and height facts share each checked prefix; no interpreter callback per piece or sticker |

- **KI.11 [D]** `SeamCell` and `SeamTotals` have JI.8's fields. Each unique mismatched
  (domain, cut-cell) pair counts once, even when multiple caps hold it. All-scope completion alone gives
  `total_N = on_grip_mismatches + off_grip_mismatches`; this is not a sum over caps. Named-cap results
  give `N_c`, with `total_N = undefined(named_cap_scope)` unless an independently completed all-scope
  result at the identical input is joined. Interrupted totals are `undefined(incomplete_coverage)`.

- **KI.12 [D]** `PrefixObservation` and `WordObservation` use every field of JI.10 and JI.11.
  Optional unrequested fields are `undefined(not_requested)`, not absent zeros. Counts are unbounded
  naturals, deltas signed integers, sets canonical piece IDs, poses exact keys over Q(sqrt(5)). N at each
  side uses complete all-scope seam facts. F and D are counts of their returned sets; on the stated
  admissible I_a domain `delta_phi = 2*(F-D)`. Retained letters give D undefined and actual delta-phi
  zero; outside the branch premises the accounting fields are undefined. Blocked/uncertain/invalid
  letters give their verdict and evidence without applying the letter; after-fields are undefined and
  the suffix is unchecked. Coverage distinguishes examined letters from applied letters. The last
  admissible prefix reference is not a reference to an unplayed endpoint. Whole-word aggregates require
  complete replay. A false step flag remains data, not an execution refusal.

## Retained path, host authority and walks

- **KI.13 [D]** `word_open` and `stream_next` expand a caller-supplied pinned recipe chronologically,
  preserving exact length, alphabet, dependency closure and source positions. At complete expansion
  they return an immutable `LiteralWord` owner and its literal digest; an incomplete expansion cannot
  masquerade as that word. Large words may retain an immutable compressed representation and verified
  content references instead of one flat allocation. `propose` independently freezes/replays it into
  the engine's own preview, with all collateral and Net/Strict coverage. It does not adopt a supplied
  endpoint. The runtime sees only results its current grants allow: under capability separation a
  preview-only program cannot read preview outcomes. The built-in shell workflow in the retained walk
  therefore reads the preview; it is not permission for a solver program to combine read and preview.
  At `operation.preview` admission in a solving session the host's provenance adapter reads the block of
  the word and of every part (KN5.26, BND.84, BND.85). A word that a solver's own tool computed refuses
  with `LG3:capability.not_granted` and installs nothing. The retained walk's manual macro passes;
  `check_annex.py` holds the rejection control.

  `read_presented` is the existing host adapter service of RM.14/RM.26, catalogued so the walk has no
  unstated presentation interface. Its row association is a commit-gate dependency, not a language row
  or proof that a frame was presented here. `commit_preview` accepts a fresh shell token only.
  `outcome` retrieves the same receipt after a lost notification. `undo_hint` takes session/epoch and
  `direction: undo | redo`, returning LI.9's `ActionHint` or `none` for that direction. The retained walk
  acquires an undo hint before the inverse and a redo hint afterwards; LI.9 supplies the distinct-target
  move/rebind/checkpoint vector and the invalidation of both hints on any order change. `undo` binds both
  fields and uses the shell action and normal gate path; a redo invocation binds its direction's hint
  through `session.redo` under LI.26. CM.5 governs
  journal and workspace inverses, including restoration of a recorded rebind. Equal labels after undo never
  revive a preview. Host-only operations are explicitly excluded from runtime registration.

- **KI.14 [D]** The two walks are specified source/synthetic contract traces, not puzzle-engine,
  renderer, performance or platform test results. The retained macro expands to `[+1,-1]`: it tests
  literal prefix preservation even when the net effect is identity. The research input is JI.8's
  synthetic complete three-mismatch table with two off-grip entries, attached to a symbolic exact
  pose input. It does not assert that those records realize a particular geometry. A supplied admissible
  letter x gives a symbolic exact next pose S1; the walk traces the result fields and exact equations
  without inventing numerical geometry results. Both walks release every owned resource. Separate
  interruption traces cover expansion cancellation, partial seam enumeration, prefix cancellation,
  budget cut-off and commit/cancellation races through catalogue operations.

## Binding holes

- **KI.15 [O]** The logical operations and release obligations above leave these concrete holes.
  The owner chose route RT1 and Rust (owner, 14 November 2026, LQ8, LQ9; RTE.17, RTE.21) and capability separation
  (LQ21; RTE.24). The RT1 row is the binding to write; RT2, RT3 and RT4 are not built and their rows record the
  unselected alternatives.

| Route | Binding still required |
| --- | --- |
| RT1 | Chosen engine-language types, checked isolate entry, exact-number representation, owning/borrowed references, scheduler continuations and failure containment; map each operation to the typed IR and meter |
| RT2 | Chosen VM's approved subset and domain adapter; lossless integers/partial results, handle rooting and deterministic release independent of VM GC, metering and interrupt hooks, dynamic-code refusal and executed-byte approval; modified VM if run-time labels are selected |
| RT3 | Typed-IR lowering and VM imports; exact-number and target-memory transfer, bounds/handle checks, ownership across memory growth, charge/source-map preservation and interrupt mapping; compiled labels if selected, with fuel equivalence proved |
| RT4 | Checked host subset plus common IR or RTE.6's versioned replacement; generated transitive decomposition, exact types, effects, deterministic meter/drop points and source identity; host-runtime modification if run-time labels are selected |

  Every chosen placement additionally needs the concrete ABI or messages, canonical transfer encoding,
  immutable-sharing mechanism if any, acknowledgement/retry protocol, process-loss reclamation,
  resource bounds, production tariffs/safe points, and matching oracle/fault/transfer evidence. The
  annex defines no encoding and adopts none of the test tariffs in `contract-vectors/`. H-08 resolves
  `requires`; EN3/EN4 still review the observation fields and forms table together. FR-B03 remains
  **partial** until that agreement and the selected-route binding exist; a passing reference check is
  not closure or freeze.

## Check and integration

- **KI.16 [D]** Run `python -B docs/architecture/1.0/language/kernel-interface/check_annex.py`.
  It resolves command and service references separately and checks their kind and disjoint IDs. Missing
  registrations fail even when named in `requires`. It validates clause references, catalogue fields, all walk operation references,
  direction-specific hint acquisition, actual resource-input owners and release, the observation mapping, route holes
  and rejection controls entirely in memory. Controls include r7 proposing the already released `letters` owner
  while `uses` still names `w`, borrowing a path from a released chunk, omitting a real input from `uses`,
  borrowing an alias before its creation, and a structured tool block at r7, direct or in a part, that does not refuse (BND.84, BND.85). It writes
  nothing, creates no `build/` or bytecode, refreshes no digest and runs no engine. The public files
  are the editable sources; no chapter or decision digest is pinned. A complete walk is fully specified,
  not experimentally verified.

  After reconciliation the integrator reruns that command and
  `python docs/architecture/1.0/language/closure/check_closure.py --all` (not run here).
  Independent review of this Codex-authored candidate remains with the integrator. The packet's
  no-network/owned-folder constraints prevent a review-wrapper call here. Concrete route conformance,
  Windows presentation, Linux equivalence and performance commands cannot be specified before a
  binding/harness exists; none was run or is implied by this check.

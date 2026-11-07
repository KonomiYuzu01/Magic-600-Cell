# Renderer feature matrix and portable interface

Status: **draft for the owner. Not accepted.** Packets p6 and p7-render, closure and reconciliation waves.
No renderer is selected, no ADR is accepted, and no gate result or new measurement is supplied here. The selection gate in
[renderer-candidates](../../../wiki/decisions/renderer-candidates.md) is unchanged.

Read with [chapter 10](architecture/10-renderer-platform.md), [ADR-L11](adr/ADR-L11-display-plans.md),
[the working handoff](closure/HANDOFF.md), [EV3](architecture/04-environment.md),
[ONE.41 and ONE.62](architecture/08-one-state.md),
[jumbling interfaces](../../../progress/1.0/jumbling-interfaces.md), JI.40 to JI.46,
[layer interfaces](../../../progress/1.0/layer-interfaces.md), LI.7, LI.16 and LI.25 to LI.30, and
[`magic600-jumbling-render/1`](../../../../research/jumbling/render-contract.md).
The handoff's forms are the integrator's proposals, not owner decisions. This document consumes them by reference.

## 1. Disposition and open choices

- **RM.1 [D]** The feature table is the proposed support contract for RPI.15, RPI.17, RPI.18, RPI.50 and RPI.55.
  A row identifies a semantic feature, not a candidate implementation. Required means that a backend claiming the
  applicable feature surface must deliver it. Reporting an unsupported required row is a truthful refusal and an
  unmet delivery requirement; it is not a conforming omission. Optional means that its absence is conforming.
  Neither status is established by a probe drawing a similar picture.
- **RM.2 [B]** The working form follows the owner's answers of 6 November 2026: user stages (LQ7) and a handoff before
  the renderer feature freeze (LQ20). [D] Every row's last column applies these marks:

| Mark | How the row enters the feature list |
| --- | --- |
| `A` | Enters the feature list with its data, ownership and required support; only a referenced chart's admitted evaluator changes |
| `B` | Parameterised built-in stages stay required and enter the feature list |
| `G` | Required; enters the feature list through H-11 with RPI.68 to RPI.83 |
| `C` | Required, including ONE.41's report |
| `J` | JI.43 preserves the exact producers, referenced charts use the admitted evaluator only, and JI.41's rows enter this crosswalk |
| `N` | Optional and off by default, outside the correctness path and the selection pass |

- **RM.3 [O]** RPI.27 still leaves pick disambiguation to the design line. Nothing here chooses placement, appearance,
  motion timing, wording or a DT-J2 to DT-J4 topic. Further views follow RPI.81 to RPI.83 and RM.39.
- **RM.4 [D]** RM.34 crosswalks every JI.41 producer row to J01 to J13, using JI.40's enclosing binding and JI.42's
  immutable bundle lifetime. J01 to J10 keep their IDs; J11 to J13 cover release checks, nu/correction observations
  and picks explicitly. The producer contracts are supplied as drafts. They are not backend implementations or
  new render/1 assets. J10 preserves the unavailable-geometry outcome; production sizes, layout bindings and
  backend evidence remain open under RM.33. FR-K05 stays partial for those inputs, not for an absent JI.41 table.

## 2. Data production and layout

- **RM.5 [D]** Every payload inherits RPI.6's base, subject, dimension, geometry identity, precision and dependencies.
  Engine producers supply exact state, poses, sets, certificates and preview effects. LB4 or a checked program
  computes display positions and incidences on the CPU once per changed input revision. The adapter validates and
  packs those values; it does no exact analysis or user-program evaluation in a frame loop. Display values, including
  H6's `display_if` results under that working form, reach it only through `publish_view`. The alternative replaces
  `ApproxBool`/`display_if` admission with a separately checked display region and checked exits; it changes neither
  this sink nor the prohibition on display-to-exact conversion.
- **RM.6 [D]** These are portable, immutable input layouts, not a disk format, a C ABI or a shader constant-buffer ABI.
  Chapter 05 and LI.16 own canonical stored/output bytes. An array descriptor contains its element type, shape, byte
  length and content digest. Numeric arrays are tightly packed, little-endian, with the last index contiguous;
  f32 means IEEE binary32. Indices are u32 with checked range and shape products before allocation. Matrices are
  row-major acting on column vectors unless an existing asset or render-contract field explicitly says otherwise.
  The adapter transposes or pads on its side and records that conversion; it never changes the input digest.
  Canonical object IDs and exact references remain typed KN1 values, not native pointers or floats.

| Payload name | Producer and portable layout | Validation before use |
| --- | --- | --- |
| `GeometryInput` | Asset loader: model and manifest digests plus the unchanged arrays of render-contract section 2 and S-B SPEC section 2; in particular `mesh_vertices.f32` [30480,4], `mesh_sticker.u32` [30480], `cell_frames.f32` [600,4,4] **column-major**, and offsets [434] | Check each asset digest and dimension; area-weighted shrink anchors follow SPEC section 3, never numbering centres; no generated replacement asset |
| `RetainedInput` | Engine: full labels u32 [259800], styles/token references, separate visibility and interaction u8 [259800] with values 0 or 1, canonical object associations; one complete RPI.9 bundle | Labels and all mutable arrays bind the same revision and full state; a display filter never shrinks the engine state |
| `TransformInput` | Engine-certified retained motion: grip, plane vectors f32 [2,d], angle f32, sorted moving-piece IDs u32 [m] and exact-set digest, committed endpoints; shell supplies only the display phase | RPI.16 and render-contract section 4; no inside-set inference; no swept half-turn without the engine's plane; J03 cites JI.41's jumbling binding |
| `LayoutInput` | LB4/program: typed object/instance table [n], positions f32 [n,k] with k = 2 or 3, optional chart references, edges u32 [e,2], triangles u32 [t,3]; all positions finite | Indices address the supplied instance table; repeated objects have distinct instances; coverage is explicit; no object ID is inferred from a position |
| `IncidenceInput` | LB4/program: `LayoutInput` plus cell dimension u32 [c], boundary offsets u32 [c+1], boundary indices u32 [b] and orientation i32 [b] in {-1,1}; canonical cell references | Offsets start at 0, are nondecreasing and end at b; each boundary reference names a lower-dimensional supplied cell; this is supplied incidence, not a proof |
| `GraphInput` | LB4/program: `LayoutInput` plus a typed state/reference table and directed edges u32 [e,2] carrying supplied transition/word references | Each node retains its own subject; graph-local nodes without a canonical object stay ViewNodes, not fictitious puzzle IDs; drawing performs no search |
| `OverlayInput` | Engine/library: typed canonical-object table [n], exact set/relation digest and coverage; display positions f32 [v,k], line indices u32 [e,2] and triangles u32 [t,3] as needed, plus token references | Same subject and revision throughout; a relation carries its claim Key under RPI.65; no absent, off-lattice or unknown result becomes an empty completed set |
| `StageInput` | Checked library/compiler: ordered Chart references, evaluator definition digest/version, parameter values matching that definition's typed signature, domain/coverage and RPI.28 precision/check references and, for a user stage, stage kind, stage digest, compile record and declared bounds | Exact property checks stay separate from approximate evaluation; every stage and definition must appear in the support report; non-finite coordinates never draw or pick |
| `TokenInput` | Host token table plus program token-name references: ordered typed name/value records bound to a table digest | Names resolve before installation; colour never identifies an object, certifies evidence or chooses a palette in this draft |
| `PublicationInput` | Runtime: RPI.5 nodes, RPI.6 plans, RPI.20 instances and RPI.21 maps as typed lists and keyed records, plus immutable input leases | References resolve inside the publication or its pinned inputs; duplicate IDs, dangling references and source mismatches invalidate the whole publication under RM.20 |

- **RM.7 [D]** Sheets and atlases use `LayoutInput`; an atlas additionally names each chart and each occurrence's
  chart membership in the instance table. A tree supplies its root indices and parent/child edges; its declared tree
  property is checked by the producer, never deduced by the adapter. A dimension-tagged complex uses `IncidenceInput`.
  A state graph uses `GraphInput`. These are distinct row kinds even when buffers can be shared. Abstract layout
  annotations remain ViewNodes under RPI.20. Section, slab and clip geometry arrives as `OverlayInput` with the exact
  predicate/result reference; rendering the same triangles does not identify those three predicates with each other.
- **RM.8 [D]** A derived display is a separate plan subject under EV3.4 and ONE.62. Its host-owned presentation
  metadata says `subject_kind = derived`, and exposes the subject digest, level, derivation or imported lineage,
  reachability reference and display comparison base. The host identifies the output as a derived state, including
  in stale/error output and pick replies. Appearance stays with the design line. Its arrays come from the kernel
  value store for that digest; the adopted session bundle continues to name the committed subject. An imported value
  uses ONE.21 and EV3.12, with no fabricated session ancestor or derivation. A program cannot label its own arrays
  `session` or `preview`. A derived display supplies no preview ID for a commit token and is never adopted by a commit.
- **RM.9 [D]** A preview uses the engine's immutable preview object and its certified effect, literal digest and base;
  the host marks it `subject_kind = preview`. The session subject remains separate. Program publication cannot create
  or substitute this object. Rendering reports the preview actually presented under RM.26, and the shell still
  performs ONE.41 and the command layer's freshness checks. A newly received preview grants nothing by itself.
- **RM.10 [D]** RPI.21's source linkage consumes the `SourceRange` contract of EV1.10, with no new coordinate system.
  Every present range names the producing source digest and resolves to pinned immutable source bytes. Admission
  validates its bounds and scalar boundaries against those bytes. If source is unavailable, the entry explicitly
  carries the producing digest and `source_missing(reason)` with no invented range; object resolution can still work.
  A declared range with no matching bytes, or an invalid range, is `EV3:input.display_plan_invalid` under RM.20.
  When consuming a pick, check all RPI.24/RM.24 bindings first. An explicit stale pick returns
  `EV3:stale.display_pick` before object resolution or source navigation, even if the source digest also mismatches;
  passive stale replies are dropped under RPI.25. Only after those bindings pass can a later source-digest mismatch
  return `EV3:stale.source_link`. A source edit or module activation never retargets old instances: navigation opens
  the producing version, or reports missing source. EV1.10's host-unit conversion follows the digest check. These
  checks do not invalidate an old run merely because a new checked module exists.

## 3. Limits and accounting

- **RM.11 [D]** A versioned `LimitProfile` supplies finite ceilings before a feature can be advertised as supported.
  Missing fields or units make the support report invalid, rather than meaning unlimited. Limits are inputs pinned
  for a run and its support report; no measured value is asserted here. The per-publication count rules are:

| Ceiling field | Unit and counting rule |
| --- | --- |
| `nodes`, `plans`, `instances`, `maps`, `map_entries` | Number of supplied records of that kind; duplicate occurrences count separately even if their canonical object is the same |
| `charts`, `stages`, `stage_ops` | Chart records, total stage occurrences after chain expansion, and maximum weighted checked scalar operations for one vertex evaluation of the expanded chain (RM.38's table); bounded loops count their full bound |
| `fragment_ops`, `overdraw`, `frame_work` | Maximum weighted checked scalar operations for one fragment invocation; allowed fragment invocations per surface pixel, as RPI.72 counts them; and the combined weighted frame ceiling charged at admission (RPI.71, RM.38) |
| `view_vertex_evals` | Per-frame vertex evaluations summed over visible views; a hidden view adds nothing (RPI.82, RM.39) |
| `layout_vertices`, `layout_indices`, `overlay_items` | Supplied position rows, all index entries including incidence boundaries and graph edges, and supplied exact-set/relation/certificate records; multiplicity counts |
| `input_bytes`, `source_bytes` | Sum of descriptor byte lengths, and pinned source byte lengths; arithmetic is checked before allocation; deduplication cannot excuse a larger logical publication |
| `canonical_output_bytes` | Full publication's canonical uncompressed byte length under LI.16; metadata and delivered payload values count, and cache hits or compression do not discount it |
| `adapter_cpu_bytes`, `adapter_gpu_bytes` | Physical resident-byte ceilings for the adapter, including staging, padding, the installed generation, the pending generation and resources awaiting safe retirement; separate from KN7 logical units |

- **RM.12 [D]** The fixed admission ceiling is the componentwise minimum of the host/profile ceilings and backend
  support ceilings. A payload at the ceiling passes this check; one above it yields
  `EV3:resource_limit.display_plan_ceiling`, naming the field, requested count and ceiling. For a standalone
  publication whose payload exists at admission, this is `refused(Error)` before acceptance. For an already accepted
  scheduled view, final validation is `failed(original Error)` on its original invocation under LI.7/LI.25 and RM.20.
  It neither enters the lane again nor resurrects superseded work. A malformed descriptor is
  `EV3:input.display_plan_invalid`; a valid unsupported kind/definition is `EV3:input.display_unsupported`.
  Preflight uses checked counts before transfer-buffer allocation or GPU submission. Backend allocation failure
  after acceptance ends preparation with `failed(KN7:resource_limit.host_exhausted)`; it is not a deterministic
  budget cut-off. No allocation, driver or zero-copy success is assumed from a logical size.
- **RM.13 [D]** The declared run's output allowance is separately charged under LI.16 and KN7.7 to KN7.10 before
  publication data is emitted to the adapter. Its terminal reserve cannot carry user output. If 100 canonical bytes remain and the next complete
  publication requires 101, its job returns `cut_off(KN7:budget.output)` and an accepted publication invocation ends
  `cancelled(budget)` under LI.28, with no installed part. The 101-byte value is not emitted; previously charged output
  is not refunded. At 100 required bytes it can proceed, subject to other checks. Transport framing, compression,
  cache reuse and GPU packing do not alter that charge. Logical memory and its release follow KN7.9 and chapter 09,
  not the GPU retirement clock. This draft uses fixed plan ceilings and the existing output budget, so it does not activate
  the reserved `KN7:budget.display_plan_limit` or add a new deterministic budget dimension. That code belongs only to
  a separately declared producer plan budget under ERR.25's pending alternative, never to a fixed renderer ceiling;
  fixed ceilings always use RM.12/RM.20, including JI.41's complete straddler payload when producer budgets fit.
  Numeric production ceilings, logical tariffs and H-09 resource measurements remain unmeasured and must be pinned
  before enablement.
- **RM.38 [D]** The stage fields of RM.11 use one weight table. Each operation of RPI.70's tiers has a weight, its
  measured cost relative to `+`, and a bounded loop counts at its bound. `stage_ops` and `fragment_ops` are sums of
  those weights over the expanded chain. `overdraw` is a ratio: fragment invocations allowed per surface pixel, as
  RPI.72 counts them over every pass and shading sample. `frame_work` is a weighted operation count: the sum over
  visible views of vertex evaluations times `stage_ops`, plus surface pixels times `overdraw` times `fragment_ops`.
  Admission charges it from declared values before the first frame, and it is charged again before a change of surface
  size or visibility takes effect. Showing a view over the ceiling refuses under RM.39. A resize cannot be refused, so
  a view that a resize takes over the ceiling falls back to the built-in chain with visible status before the first
  over-ceiling frame, with no quarantine, and its stage returns when a recharge fits. The table and the values come
  from measurement (RPI.79).
- **RM.39 [D]** `view_vertex_evals` is the ceiling on per-frame vertex evaluations summed over visible views; a hidden
  view adds nothing (RPI.11, R-19). Memory stays under `adapter_cpu_bytes` and `adapter_gpu_bytes`: hiding stops
  evaluation and rebuilding and frees nothing, and hidden allocations stay charged until safely released, including
  eviction and delayed retirement (RM.19). RM.25 permits retained hidden data under the same charge. Opening or showing
  a view past `view_vertex_evals`, `adapter_cpu_bytes`, `adapter_gpu_bytes` or `frame_work` is refused with `EV3:resource_limit.display_plan_ceiling` under RM.12, naming field,
  requested size and ceiling. The value is the largest stated workload with a passing run of the unrelaxed gate; a
  failed workload lowers it and never moves the gate (RPI.60, RPI.82).

## 4. Portable adapter operations

- **RM.14 [D]** This is a data interface between host and adapter. It is never a capability granted to a program.
  KN5.21 owns receipts, their statuses, `Completion`, `ResultRef` and `CancelReason`; KN5.23/LI.30 own outcome lookup.
  EV1.10 owns the source-coordinate contract; chapter 01 owns identities and revisions. The route-specific ABI or
  transport stays with the integrator's binding annex. No promise of zero-copy or a candidate framework API follows from these signatures.
  The interface version and the non-jumbling layouts in RM.6 have draft version 1; this is not a frozen storage
  format. JI.41 supplies the jumbling payload contracts crosswalked in RM.34. Concrete layout versions for its
  additions remain an EN2/renderer binding input before those rows can be advertised as supported.

  ```text
  SupportReport = { interface_version, backend_build, device_epoch,
                    features: Map<feature_id, supported(layout_version, LimitProfile, stages)
                                              | unsupported(Error)>,
                    evidence: List<scoped evidence references> }
  PublicationStamp = { command: CommandId, view: ViewId, view_generation, publication_generation,
                       device_epoch, base: RevisionRef, dependencies, input_digest }
  Preparation = { stamp: PublicationStamp, payload: PublicationInput, limits: LimitProfile }
  describe_backend() -> SupportReport | unavailable(Error)
  adopt_engine(bundle: complete RPI.9 bundle, inputs: ImmutableLease)
      -> adopted(RevisionRef) | refresh_required(Error)
  prepare(request: Preparation, inputs: ImmutableLease) -> Ticket | refused(Error)
  poll(ticket: Ticket) -> pending | ready(ResourceSet) | cancelled(CancelReason) | failed(Error)
  activate(resources: ResourceSet, stamp: PublicationStamp) -> installed | refused(Error)
  cancel(ticket: Ticket, reason: CancelReason) -> acknowledged
  retire(resources: ResourceSet) -> acknowledged
  submit_frame(input: FrameInput) -> FrameId | unavailable(Error)
  read_presented() -> List<PresentedFrame> | unavailable(Error)
  read_frame_reports() -> List<FrameReport> | unavailable(Error)
  request_pick(frame: FrameStamp, point, requested_kind_and_level) -> PickTicket | refused(Error)
  poll_pick(ticket: PickTicket) -> pending | stamped_instances | refused(Error)
  recover() -> SupportReport | unavailable(Error)
  ```

- **RM.15 [D]** Feature IDs are the table's stable row names. A support report lists every applicable row and every
  admitted stage definition/version, including built-in alternatives; omission means unsupported. `supported` is an
  implementation declaration, not evidence of conformance. `unsupported` names `EV3:input.display_unsupported` and
  the exact row or stage. A report with no evidence says `no evidence yet`. Each device epoch and backend build gets
  a fresh report, immutable for that epoch; changing support or limits starts another epoch. Required and optional
  are the contract's dispositions; a backend cannot rewrite them.
- **RM.16 [D]** `adopt_engine` installs the complete CPU-side revision image independently of any program lane,
  adapter preparation or recovery. It never waits for program work or a GPU fence. GPU drawing resumes only with a
  complete uploaded bundle; a frame cannot claim the new revision while still using old arrays. Missing or mismatched
  engine inputs return `refresh_required(EV3:input.display_bundle_mismatch)`. Program resources have their own caps
  and cannot consume the resources reserved for this path. Engine adoption during device loss still updates this
  authoritative read-only image; it makes no claim that a frame was displayed.
- **RM.17 [D]** `prepare` is host-scheduled off the UI and engine-adoption paths. A backend can finish synchronously
  there or use a worker; both expose the same ticket/poll protocol. The host owns tickets and resource tokens, which
  contain no application-visible device handle. Preflight checks the full input before admission to backend work.
  After the command layer has accepted publication, a backend refusal becomes KN5.21 `failed(Error)`, not a retrospective
  pre-acceptance refusal. After cancellation is acknowledged, polling returns that cancellation; a late completion
  cannot yield usable resources. `activate` is a bounded host-side
  compare-and-swap of the prepared generation and its maps/nodes; it does no GPU wait or program evaluation.
- **RM.18 [D]** The producer seals its payload and gives the host an immutable lease or a copy before standalone
  admission, or before final validation of an already accepted scheduled view. LI.25 keeps that view's invocation.
  The host pins those bytes and the producing source versions for the installed or pending publication. A program
  cannot mutate them, and ending the producing run cannot invalidate them. The adapter owns all staging and device
  copies. `retire` removes a resource set from future submissions immediately; physical release waits for the last
  transfer, draw or readback using it. `cancel` is idempotent and acknowledged without waiting for that release.
  Publication termination, input-latch release and GPU retirement are separate events. Reuse requires a matching
  immutable digest and binding; no lease survives as permission to use a stale subject.
- **RM.19 [D]** The physical ceilings count retired resources until the backend proves completion or completes safe
  device teardown. Failure to prove a fence complete never permits early free or reuse. When a new request cannot
  fit beside live/retiring resources it receives the resource-limit outcome of RM.12; there is no unbounded queue.
  Lost-device work is detached from the host's input and publication state promptly and quarantined on the renderer
  side until safe disposal. Recovery either supplies a fresh epoch or reports unavailable; it does not hold a receipt
  pending forever. RPI.46 remains the fault obligation, with no invented timeout or recovery measurement.

## 5. Publication, replacement and presentation

- **RM.20 [D]** `publish_view` returns KN5.21's `Receipt` under RPI.10 and LI.7/LI.25. A standalone publication has
  its payload at admission: check the target view and generation, full bindings, source-map validity, support and
  fixed limits then. A bad standalone request is `refused(Error)` and supersedes no valid pending work. A scheduled
  view has already been accepted: final validation of its generated payload keeps the same command, job, request
  and lane generation. Any validation error is `failed(original Error)` on that invocation, with no new admission,
  no self-supersession and no resurrection of previously cancelled work. LI.28 alone maps its job outcome to a
  receipt. Accepted work owns one pending generation per view and
  charges output under RM.13 before adapter transfer. The host installs nodes, plans, instances, maps and prepared
  resources together, only while the command is still the newest accepted one and its dependencies/device epoch
  still match. Success is `completed` with KN5.21 Completion fields `kind = view_published`, `revision = base`, and a
  ResultRef whose `digest` identifies the canonical publication and whose `covered` identifies the installed view
  generation (`job` names the job when one exists). It is never `committed`, and it does not mean a frame was presented.
- **RM.21 [D]** If accepted publication A is pending when B is accepted for the same view, A ends once as
  `cancelled(superseded(by: B.command))`. A's ticket is cancelled and its late completion cannot install. The currently
  installed generation remains whole until B successfully replaces it, or remains separately identified stale under
  RPI.13 if its state dependencies moved. If A already installed and completed before B's admission, A keeps that
  terminal receipt; B retires A's resources on installation and does not change A's outcome to cancelled. The view
  lane serializes this race. A rejected B cancels nothing. A B that fails after acceptance does not resurrect an
  already cancelled A. No intermediate or incomplete publication becomes a negative result.
- **RM.22 [D]** A dependency change before installation ends accepted work as `cancelled(replaced_revision)`;
  explicit user cancellation ends it as `cancelled(by_user)`. An output cut-off follows RM.13. A device failure before
  acceptance is `refused(EV3:busy.renderer_unavailable)`, using the code's primary `refusal` form. After acceptance
  but before installation it is `failed(EV3:busy.renderer_unavailable)`. For an associated job the error uses the
  contextual `stop` form inside `JobResult.outcome = error(Error)`; LI.28 maps it to `Receipt.status = failed(Error)`.
  This follows H15.9's contextual forms for the existing code, without a new spelling. A failure after installation
  changes display availability, never its completed receipt or any mechanical receipt. The first serialized terminal wins; later cancel/failure notifications
  release resources only. KN5.23 and LI.29/LI.30's lookup and epoch-lifetime rules apply after a lost notification.
  A missing report is never reconstructed as success from pixels or a hash.
- **RM.23 [D]** Device loss invalidates the device epoch, all in-flight pick/presentation reports and the shell's
  presented-preview latch. Input latches are released. Pending publications take RM.22's failure unless already
  terminal. Installed CPU data may be retained, marked unavailable, but old resource tokens cannot draw on a new
  device. Recovery rebinds immutable geometry and the latest complete engine bundle, then separately revalidates
  retained publications and pinned source data. Valid retained data gets new device resources and fresh frame/pick
  stamps; stale data waits for recomputation. Recovery neither retries a commit nor changes a terminal receipt.
- **RM.24 [D]** A `FrameInput` contains the complete engine bundle reference, selected resource sets, camera/viewport
  inputs and the host's subject labels from RM.8/RM.9. Its `FrameStamp` binds the device epoch, surface and surface
  generation, increasing frame sequence, engine revision, each view/plan/map generation, subject kind and digest,
  camera version, viewport, visibility and interaction revisions. `submit_frame` returns submission identity only.
  An ID pass uses the same stamp, camera and clipping. Picking resolves returned instances through the corresponding
  maps, never through a chart inverse or colour. RPI.23 to RPI.25 decide partial and stale results.
- **RM.25 [D]** A hidden or unchanged view triggers no program evaluation or resource rebuild. Hiding a view cancels
  its pending publication as `cancelled(by_user)` and clears its presented-preview latch; showing it revalidates
  retained data before use. Close/reopen changes the view and surface generation. The host may retain identified old
  data for RPI.13's stale presentation, but cannot attach its overlays to a new subject or resolve an old pick through
  a reused instance ID. Engine adoption continues regardless of those view states.
- **RM.26 [D]** `PresentedFrame` contains its `FrameStamp` and, for each displayed preview, the engine preview ID,
  literal digest and base; absence is explicit `no_preview`. The adapter emits it only for a frame whose presentation
  it has confirmed, not merely on receipt of a preview, upload completion, frame submission or a Present-call return.
  A dropped/unconfirmed frame produces no positive report. If confirmation is unavailable the host clears the
  presented-preview latch and treats presentation as unavailable, never guessing the newest received preview.
  Reports are ordered by frame sequence within one surface/device epoch; a delayed older report cannot replace the
  newest confirmed one. A `no_preview` report clears the latch. A new-device or reopened surface requires a fresh
  report. ONE.41 binds a token only to the reported preview, still subject to engine freshness checks. A derived
  display, even with identical labels, cannot populate this field, and neither can a preview drawn through a user
  stage (RPI.76). This report is required (RPI.66); each candidate still needs an implementation and actual
  presentation evidence.
- **RM.41 [D]** `read_frame_reports` returns each frame that stopped a user stage, as
  `FrameReport = { stamp: FrameStamp, view: ViewId, publication_generation, error: Error, fallback: built_in_chain | error_status }`.
  `error.form` is `status`. The report arrives after the publication is terminal, so it changes no receipt and no job
  outcome (RM.22). `completed(view_published)` stays, and the view's visible status shows the fallback.

## 6. Per-feature disposition

- **RM.27 [D]** Every row inherits RM.5/RM.6 binding and RM.11 to RM.13 limits. The lifecycle column names the concrete
  rules rather than granting a separate publication path: `P` is immutable host leases, atomic replacement,
  supersession, cancellation and refusal under RM.17 to RM.23; `E` is independent complete engine adoption and safe
  retirement under RM.16/RM.18/RM.23; `F` is the stamped frame/pick lifetime under RM.24 to RM.26. All require a typed
  error for unsupported input. Layout/index overflow, bad hashes and malformed references follow RM.20, never truncate.
  `Required` follows the product scope the owner set (owner, 6 November 2026, LQ1, LQ16). A cut in that scope requires
  an owner decision and a changed feature list. RM.34 supplies the jumbling crosswalk;
  missing geometry, unmeasured sizes and absent backend evidence stay explicit in its rows.

| Feature ID / primitive | Producer; payload and layout | Adapter operation | Additional limit / refusal | Ownership, replacement, cancellation | Disposition | Evidence at exact scope | Mark |
| --- | --- | --- | --- | --- | --- | --- | --- |
| F01 Retained sticker objects and home shrink | Asset loader + engine; `GeometryInput`, `RetainedInput` | `adopt_engine`, `prepare`, `submit_frame` | d = 4; all 259800 labels; reject identity/hash mismatch, never reindex | E for engine; P for program copies | Required | E1/E2/E3: retained W3 geometry/labels only; no language binding evidence yet | C |
| F02 Visibility and interaction sets | Engine exact filter result; separate u8 arrays in `RetainedInput`, set digest | `prepare`, `request_pick` | Both arrays retain full subject; no excluded object enters the ID pass | E + P + F; never changes mechanical protection | Required | Source: RPI.3/RPI.17; no evidence yet for this interface | C |
| F03 Retained certified transform | Engine + shell phase; `TransformInput` | `adopt_engine`, `submit_frame` | Certified set/endpoints; absent half-turn plane gives no swept preview | E + F; motion sampling never commits or relabels | Required | E1/E2/E3: their W3 turn only; no general motion-interface evidence yet | C |
| F04 Built-in projection chain | Checked library; `StageInput` for rotate, perspective or stereographic, camera3, and admitted LB4 local/Hopf/two-hemisphere definitions | `prepare`, `submit_frame` | Per-vertex stage/operation ceilings; unknown definition refused; RPI.34/RPI.36 checks before enablement | P + F; per-frame projection consumes inputs, not analysis | Required for admitted definitions | E1/E2/E3: fixed W3 rotation/perspective/camera only; no evidence yet for other stages or this interface | B |
| F05 User vertex stage | Stage checker + CPU reference; `StageInput`, checked definition, compile record and portable HLSL artifact digest | `prepare`, `submit_frame` | RPI.68 to RPI.72 admission, `stage_ops` and `frame_work`; raster bound before any fragment work | P + F; programs cannot upload shader/device handles; RPI.77 quarantine | Required | Compute subset only: E5; no drawing-pipeline evidence | G |
| F06 Object sheet | LB4/program; `LayoutInput` | `prepare`, `submit_frame` | Layout vertices/indices/instances; unsupported layout refused | P; CPU once per changed input revision | Required | Source: LB4/RPI.38; no evidence yet | A |
| F07 Atlas and repeated chart copies | LB4/program; `LayoutInput` + membership of each chart occurrence | `prepare`, `request_pick` | Count every instance and expanded chart; no collapse of canonical copies | P + F; each copy keeps its own instance/map entry | Required | Source: LB4/RPI.20; no evidence yet | A |
| F08 Tree layout | LB4/program; `LayoutInput` + roots and parent/child edges | `prepare`, `submit_frame` | Edge/index bounds; producer supplies any checked tree claim | P; no per-frame tree computation | Required | Source: LB4/RPI.38; no evidence yet | A |
| F09 Dimension-tagged cell complex | LB4/program; `IncidenceInput` | `prepare`, `submit_frame` | Incidence counts and dimensions; dimension beyond admitted d refused | P; no incidence inferred by GPU | Required | Source: LB4/RPI.15; no evidence yet | A |
| F10 State graph | LB4/program; `GraphInput` | `prepare`, `submit_frame` | Node/edge counts; subject on each node; no word search in adapter | P; graph-local nodes remain ViewNodes | Required | Source: LB4; no evidence yet | A |
| F11 Exact section | Engine/KN3 + LB4 conversion; `OverlayInput` and exact section predicate | `prepare`, `submit_frame` | Overlay counts/indices and explicit coverage; empty concerns that predicate only | P; exact producer owns predicate | Required | Source: LB4; no evidence yet | A |
| F12 Slab | Engine/KN3 + LB4 conversion; `OverlayInput` and slab predicate | `prepare`, `submit_frame` | Same bounds; never substitutes a section | P; exact producer owns predicate | Required | Source: LB4; no evidence yet | A |
| F13 Clip | Engine/KN3 + LB4 conversion; `OverlayInput` and clip predicate | `prepare`, `submit_frame` | Same bounds; singular-crossing treatment waits for RPI.36 | P; drawing cannot certify membership | Required | Source: LB4/RPI.36; no evidence yet | A |
| F14 Exact-set and relation overlay | Engine/library; `OverlayInput`, claim Keys for relations, coverage and missing parts | `prepare`, `submit_frame` | Overlay/instance limits; relation without a Key refused; no false negative from partial data | P; exact relations once per input revision | Required | Source: EV3.8/RPI.65; no evidence yet; jumbling extension rows below | A |
| F15 Token colours | Host + token references from program; `TokenInput` | `prepare`, `submit_frame` | Token/source bytes; unresolved name refused; colour never identity | P; a table change replaces the bound token generation | Required | No evidence yet for host token mapping; W3 colours do not establish it | A |
| F16 Derived-state display | Kernel value store + runtime; `PublicationInput` and RM.8 metadata | `prepare`, `activate`, `submit_frame` | Derivation/import binding mandatory; subject cannot enter session/preview channel | P + F; picks include the derived subject | Required | Source: EV3.4/ONE.62; no evidence yet | C |
| F17 Engine preview display | Engine only; preview object, certified effect, literal digest and base under RM.9 | `prepare`, `submit_frame`, `read_presented` | No program-supplied preview; engine freshness still applies | E + F; old preview cannot mint a fresh token | Required | Source: ONE.41/ONE.62; no evidence yet | C |
| F18 Canonical picking and source linkage | Host/runtime; RPI.20/RPI.21 records, EV1.10 ranges, stamped ID pass | `request_pick`, `poll_pick`; host `resolve_pick` | Instance/map/source counts; bindings checked before source digest under RM.10, no inverse lookup | P + F; source versions pinned through map lifetime | Required | Source: EV3.1 to EV3.6; no evidence yet | C |
| F19 Nodes, controls, claims and view statuses | Runtime + host; `PublicationInput`, RPI.65 kinds, base and D2 status | `prepare`, `activate`; shell consumes nodes | Node/source/output limits; no functions, workspace effects or invented evidence | P; drawing never invokes a command | Required | Source: EV3/RPI.5/RPI.13; no evidence yet | A |
| F20 Atomic publication and latest-wins replacement | Command layer + host; `PublicationInput`, `PublicationStamp`, KN5.21 Receipt | `prepare`, `poll`, `activate`, `cancel`, `retire` | One pending generation per view; RM.12 ceilings and RM.13 output charge | P; pending superseded command cancels; completed one stays completed | Required | No evidence yet; E4 is only a fenced texture handoff | C |
| F21 Hidden/unchanged view scheduling | Host invalidations and declared dependencies; no new payload | `cancel`; revalidate before `prepare` on visibility | Zero program evaluations/rebuilds while hidden or unchanged | P + F; RM.25, newest data only | Required | Source: RPI.11/RPI.38; no evidence yet | C |
| F22 Device loss and recovery | Host + backend; device epoch, immutable inputs, complete engine bundle | `recover`, `adopt_engine`, `prepare`, `retire` | Physical caps include retiring resources; no wait on engine/input path | E + P + F; RM.22/RM.23 | Required | No evidence yet for product recovery; E4 covers its own drain protocol only | C |
| F23 Presented-frame/preview report | Backend confirmation + host immutable frame metadata; `PresentedFrame` | `read_presented` | Ordered surface/device epoch; unconfirmed/dropped frame supplies no positive report | F; RM.26 clears absent/unavailable preview | Required | No evidence yet; W3 capture/label readback is not this report | C |
| F24 Headless validation and portable HLSL boundary | Host validator/library reference; same payloads and chart definitions | `describe_backend`, host validation/CPU reference | No device result from headless work; unsupported stage explicit | P data lifetime; no GPU lease in a headless run | Required | Source: chapter 10; no conformance evidence yet for this interface | C |
| F25 NVIDIA enhancements | Optional backend only; no new program payload or capability | Backend-private operation only if separately enabled | Off by default; no correctness/gate dependency or state-inspection evidence from reconstruction | Backend-owned; P/F isolation still required | Optional | No evidence yet for this interface; no feature enabled here | N |
| F26 User fragment stage | Stage checker + CPU reference; `StageInput`, checked definition, compile record and portable HLSL artifact digest | `prepare`, `submit_frame` | RPI.68 to RPI.72 admission, `fragment_ops`, `overdraw` and `frame_work`; opaque RGB only (RPI.75); raster bound before any fragment work | P + F; RPI.76 status order; RPI.77 quarantine; no presented-preview identity | Required | No evidence: LX-7 tested no fragment stage | G |
| F27 Further views | Host + rows `view.open`, `view.close`; `PublicationInput` per view; RM.39's fields | `prepare`, `activate`, `retire` | `view_vertex_evals`; hidden allocations stay charged; over a ceiling refused under RM.12 | P + F; RPI.81 to RPI.83; Global and Local stay linked and cannot be closed | Required | No evidence yet | G |

- **RM.34 [D]** This is the clause-level crosswalk to all 17 rows of JI.41 in
  `docs/progress/1.0/jumbling-interfaces.md`. The quoted row names identify the payload definitions there; they
  are not new renderer-owned definitions. JI.40 supplies identity, menu order, epoch, revision, source digest and
  source role; JI.42 supplies the bundle ownership obligation implemented by E/P/F. Each row inherits RM.27 and
  JI.46's CT-JI-05 obligation. The codes below use H15.16's final names while JI.41/JI.45 remain their rule sources.
  Declared producer budget exhaustion follows KN7.10/LI.28; RM.13/ERR.25 keep `KN7:budget.display_plan_limit`
  reserved for a separately specified producer plan budget. Fixed renderer ceilings use
  `EV3:resource_limit.display_plan_ceiling` under RM.12/RM.20 even when every producer allowance fits.
  The crosswalk supplies no numeric production limit, new geometry, candidate support or backend evidence.

| Feature ID / primitive | Producer; payload and defining clauses | Adapter operation | Additional limit / refusal | Ownership, replacement, cancellation | Disposition | Evidence at exact scope | Mark |
| --- | --- | --- | --- | --- | --- | --- | --- |
| J01 Jumbling static geometry and shrink | Model assets; JI.41 "Static geometry and rigid sticker shrink", JI.40 and render/1 section 2: frames, triangles/offsets, `slot_piece`, normals/R and area-weighted triangle centroids | `adopt_engine`, `prepare` | d = 4; 600 facets, 433 stickers each, 259800 slots; invalid shapes/digests give `EV3:input.display_bundle_mismatch`; numbering centres are not shrink anchors | E + P; immutable geometry identity; home shrink precedes piece pose | Required jumbling row | Source payload contract supplied; no matching candidate evidence here | J |
| J02 Posed sticker objects | Engine `pose_table`/`pose_ids`; JI.6 and JI.41 "Exact state drawn as floats", render/1 section 3: `pose_index.i32`, `poses.f32`, `pose_lattice.i32`, header and JI.40 binding | `adopt_engine`, `submit_frame` | Exactly 177120 piece indices; matching finite row-major pose matrices and lattice rows, valid indices, identity at pose 0; shape/digest/source mismatch gives `EV3:input.display_bundle_mismatch` | E + F; atomic three-array adoption; hidden pieces remain in state | Required jumbling row | Source payload contract supplied; no evidence yet for this interface | J |
| J03 Certified jumbling transform | Engine-certified letter/inside set; JI.5/JI.6 and JI.41 "Swept twist motion", render/1 section 4: grip, plane_u/v, angle, sorted moving IDs/digest and endpoints or explicit preview origin | `adopt_engine`, `submit_frame` | Verify source and moving-set pins; a half-turn without an engine-named plane has no swept preview; no inferred inside set or legality | E + F; sampling changes no state; preview keeps its origin | Required jumbling row | Source payload contract supplied; no general motion-interface evidence yet | J |
| J04 Grip, certificate and cut overlays | Engine `survey`, full `straddlers(s, cap)` and named cut geometry; JI.6 and all three JI.41 rows "Grip map", "Blocked-grip certificates and points", "Named cut hyperplane", render/1 section 5 | `prepare`, `submit_frame`, `request_pick` | 600 certified a/b statuses with coverage; uncertain survey stays pending. Full named-cap set carries every piece and both exactly checked points plus converted vectors; a cap-map witness is not that set. Named cut carries exact pole/plane and alpha=121/125. Pins and malformed payload fail validation; budgets never justify one-witness truncation | P + F; points target their witness piece and named cap, need not lie on a sticker and are never inferred from a mesh | Required jumbling row | Source payload contract supplied; no backend evidence for full sets or cut overlays | J |
| J05 Jumbling projection chain | Shell view parameters and engine frames; JI.41 "Projection chain", render/1 section 6, RPI.28 charts in RM.6 `StageInput`: ordered rotate, perspective or stereographic, camera3 | `prepare`, `submit_frame` | d = 4, compatible stages and finite inputs; unsupported chain gives `EV3:input.display_unsupported`; no substitute stage; chart admission stays RPI.28 to RPI.41 and RPI.68 to RPI.80 | E + P + F; poses precede the chain; exact data unchanged by evaluator choice | Required jumbling row | Source contract supplied; retained W3 does not establish this binding | J |
| J06 Jumbling revision adoption and readback | View model's complete bundle and adapter's bound-array check record; JI.40/JI.42 and JI.41 "Revision adoption and readback", render/1 section 7 | `adopt_engine`, `submit_frame`; backend readback produces the check record | First frame of each revision compares all three bound state arrays by digest and element; failure leaves display out of date, never changes a committed receipt | E + F; checks carry revision/device/frame stamps and are discarded if stale; this is not RM.26 presentation confirmation | Required jumbling row | Source acceptance obligation supplied; actual backend readback evidence absent | J |
| J07 Domain/height and rim summaries | `heights`, `pose_table`, `pose_ids`, `rim_summary`, `height_counts`, `seam_facts(all)`; JI.6/JI.8 and JI.41 "Domain and lattice masks" and "Rim map and counts" | `prepare`, `submit_frame` | Input-bound 177120-bit piece masks; exact domains, N_c/floor, blocked, total N, n2/n4/n6, above-6 count/maximum and definedness. Missing full coverage leaves total N undefined; no cap rank or helpful-twist field | P; exact results at one input; no float membership or stale-pose reuse | Proposed addition under RM.27 | Draft producer binding supplied; sizes unmeasured; no backend evidence | J |
| J08 Cut-cell and off-grip seams | Engine `seam_facts`; JI.8 and JI.41 "Cut-cell and off-grip seam overlays": domain/cell identity, exact hyperplanes, occupancies, certified holding caps and derived display geometry | `prepare`, `submit_frame` | Include off-grip cells and preserve distinct domain/cell identities; count/byte limits before publication; no complete total N from omitted cells or interrupted enumeration | P; exact subject/coverage retained; geometry never inferred from pixels | Proposed addition under RM.27 | Draft producer binding supplied; concrete geometry layouts/sizes and backend evidence outstanding | J |
| J09 Named filters, covers and repair | Engine caller-named straddler/covered/lifted/moved sets and JI.12 completed tables; JI.6/JI.11/JI.12 and JI.41 "Named filters and highlight/dim masks" and "Covers and repair rows" | `prepare`, `submit_frame`, `request_pick` | Piece masks 177120 bits, grip masks 600 bits with input/mask digest; mismatch discarded. Only caller thresholds; no u ordering/filter or state-marked seat. Covers/repair list answers require complete coverage; empty is cap/input scoped. Only solver-selected row masks and resolved literal word/full collateral; no default focus, rank, prefetch or execution | P + F; supplied cap-ID or pinned x/d order preserved; view grants no solving choice | Proposed addition under RM.27 | Draft producer binding supplied; limits unmeasured; no backend evidence | J |
| J10 Exposed-face geometry | Exact region/cut-cell producer contract; JI.41 "Exposed-face meshes" and JI.45: face identity, exact-source digest, converted vertices/topology | `prepare`, `submit_frame` only when certified inputs and support exist | Missing certified geometry or size profile gives `KN2:input.exposed_geometry_unavailable`; a backend lacking this feature gives `EV3:input.display_unsupported`. No fabricated mesh, sticker ID or support claim | P; immutable-model process applies; EN2/renderer geometry production and measured size still owed | Proposed addition under RM.27 | Unavailable-geometry contract supplied; geometry production, size measurement and backend evidence absent | J |
| J11 Release and word-check observations | Engine `release_check`/`word_check`; JI.6/JI.11 and JI.41 "Release/word check data": supplied chronological `WordObservation` and named-cap checks | `prepare`, `activate`; shell consumes bound records | Only actual solver releases; newest in-flight release wins. Completed checks stay in tried order and clear on state change; unchecked suffix stays uncovered, no score or execution permission from a step flag | P; one revision and input per record; no generated candidate-letter table | Proposed addition under RM.27 | Draft producer binding supplied; limits unmeasured; no backend evidence | J |
| J12 Nu and correction observations | Engine `nu`, caller-supplied correction-set check and shipped catalogue; JI.13 and JI.41 "Nu and correction-set observations": `Partial<NuVector>` and the predicted vector after the supplied signed multiplicities | `prepare`, `activate`; shell consumes bound records | Off lattice is `undefined(off_lattice)`, never zero/old nu. `KN2:consistency.fixed_nu_nonzero` cannot be displayed as a valid state. Keep all 36 fixed catalogue rows in ID order; only caller-specified multiplicities, no suggested combination, state filtering, lighting or reordering | P; exact observation/undefined result retained; selected words still need fresh preview | Proposed addition under RM.27 | Draft producer binding supplied; limits unmeasured; no backend evidence | J |
| J13 Jumbling picks | Adapter stamped hit; JI.39/JI.41 "Picks for H-05", RPI.20 to RPI.26 and RM.10/RM.24: grip, piece or certificate-point ID tied to its source role | `request_pick`, `poll_pick`; host `resolve_pick` | Check bindings first: explicit stale work gives `EV3:stale.display_pick`, passive stale hits drop; only then check source digest. Resolve IDs before invocation; off-lattice piece has no slot; a hit is no inside/outside certificate | E + P + F; source role and witness identity stay bound; no inverse lookup, capability or commit token | Required; jumbling binding of F18 | Draft identity contract supplied; no backend pick evidence | J |

## 7. Evidence boundary

- **RM.28 [D]** Evidence labels in the table refer only to the following records. They do not declare support for
  this proposed interface. Source/fixture results, synthetic geometry, actual Windows/DirectX results and performance
  records retain their separate scopes. No new timings, feature costs, tariffs or candidate comparison are derived.

| Evidence | Recorded source and exact scope | Does not establish |
| --- | --- | --- |
| E1 | `work/experiments/renderer-sb/RESULT.md`, anchor re-acceptance of 10 October; build `d41020f63b69ddaa480286c81ac9cbe505df9a5ea7d7f9622a93421774aaf804`; `results/w3-20261010T164807115Z-summary.json` under that directory (SHA-256 `5d3752b3543b443ad29339ae16771bd56d8044876f503c65ae7d6650a39c2cac`). Actual Windows/DirectX geometry/label checks and the recorded W3 performance runs on the recorded machine/conditions | Language plans, general chart admission, jumbling additions, picking, presented-preview semantics, device recovery, W5 or product latency |
| E2 | `work/experiments/renderer-l2/RESULT.md`, 10 October S-A2 Godot 4.7.2 .NET; build `06d14019c937ed55e90ad3a78e28e9988139a3fdd946e3c9f20495e9ee034106`; `results/sa2-w3-20261010T175043422Z-summary.json` there (SHA-256 `339f629cc1197f5cd35a17337a3166f69f6c5769523e073f8af718bec07d43ed`). Actual Windows/DirectX checks and recorded W3 performance under the loaded-file guard's stated limits | New build/adapter support, user stages, device recovery, W5 or selection |
| E3 | Same level-2 card, 10 October S-D Qt 6.10.3; build `befc441f2b5715c3f0c60d173700fc2eef22ad3eac18c6f91beac23f58f088c5`; `results/sd-w3-20261010T172910231Z-summary.json` there (SHA-256 `e990333279f6fe70ecc122059d247036945933b5f3f8ce7b98c6c921909ce52e`). Actual Windows/DirectX checks and recorded W3 performance under the guard's stated limits | GPU headroom from its vblank-capped result; any new feature/adapter, recovery, W5 or selection |
| E4 | `work/experiments/renderer-sb/RESULT.md`, D3D12 resource handoff of 3 October; `results/handoff-20261003T063841Z-summary.json` there and its recorded executable identity. Actual Windows/DirectX fenced texture-pattern handoff, resize and injected drain failures; no model or window | Atomic language publication, a Godot/Qt handoff, presented frames or product device-loss recovery |
| E5 | `work/experiments/language-lx/lx7-charts/RESULT.md`, frozen `lx7-r1`: 4 accepted and 18 refused sources, 496 GPU readback records of a compute subset on the RTX 4070 Laptop. Actual Windows/DirectX compute readback only | Drawing-pipeline stages, fragment stages, the raster bound, W3 performance, arbitrary-chart correctness, an exact chart property or renderer selection |

- **RM.29 [D]** The evidence trail also includes `docs/wiki/evidence/s-b-probe-results.md`,
  `docs/wiki/evidence/level-2-framework-results.md` and `docs/wiki/decisions/day7-go-no-go-ruling.md`.
  The 10 October ruling allowed all three candidates to continue at its recorded scope. It selected none. Historical
  pre-anchor runs retain their old identities. The superseded stack and schedule text in
  `docs/architecture/1.0/10_V1_ARCHITECTURE.md` is not an approval; only its surviving revision/identity obligations
  are consumed. The waiting questions and the design-line hold do not change any existing gate threshold.

## 8. Contract vectors and remaining work

- **RM.30 [D]** These are specified outcomes, not executed conformance evidence. For a valid source `a😀b`, its UTF-8
  bytes have digest `6fba5b2ea783ded096fc2444d540ffbdf49168df30993b155b7efb683313f110`. EV1.10 range [5,6) selects `b`;
  the EV1 conversion gives UTF-16 [3,4) or scalar [2,3). Insertion [5,5) stays empty at UTF-16 [3,3).
  [2,5) splits a scalar: standalone admission refuses `EV3:input.display_plan_invalid`, while a scheduled view's
  final validation fails that invocation with the same code under RM.20. With pick bindings current, applying the
  valid range to `a😀c` (digest `f2f6acee05793ab28e37f3562f442906e607a9347977cad09a2d7d682241c97a`) returns
  `EV3:stale.source_link`. Consuming a generation-7 instance after generation 8 replaces the map returns
  `EV3:stale.display_pick` before any source action, whether the requested source is still `a😀b` or is now `a😀c`.
  A new module alone does not stale a still-bound occurrence; following its pinned `a😀b` still selects `b`.
- **RM.31 [D]** With session revision 12 adopted and a valid derived subject D computed against it, installation
  returns KN5.21 `completed(view_published)` and a frame names `subject_kind = derived`, digest D and comparison base 12.
  The session subject stays revision 12, the derived value gets no session revision, and `no_preview` is reported
  for that derived view. A later session revision 13 makes a state-dependent D stale; it cannot overlay revision 13.
- **RM.32 [D]** With A pending, B accepted, then A's preparation arriving late, A has exactly
  `cancelled(superseded(by: B.command))` and only B can install. With A already completed before B is accepted, both
  can have their own `completed(view_published)` in order and A's terminal never changes. With a device lost while
  B is pending, B instead ends `failed(EV3:busy.renderer_unavailable)` and no old pick/report can be accepted after
  recovery. A mechanical command already committed remains committed. If preview P1 was confirmed in frame 40 while
  P2 was only received/submitted in frame 41, ONE.41 still names P1, subject to freshness; it never names P2 until a
  valid presentation report for P2 arrives. Pending confirmation is distinct from the adapter reporting confirmation
  unavailable; the latter and device loss clear that eligibility entirely.
- **RM.35 [D]** Paired ceiling vectors use `instances = 4` as a declared fixture limit, not a measured production
  value. All other counts, bindings, support and output allowance fit. With A pending, standalone B submits five
  instances already available at admission: B is `refused(EV3:resource_limit.display_plan_ceiling)`, with field
  `instances`, requested 5 and ceiling 4. A stays pending and nothing from B installs. In the separate scheduled
  case, B is accepted first and supersedes A, then generates five instances. B ends
  `failed(EV3:resource_limit.display_plan_ceiling)` with the same field/counts on its original command, job, request
  and lane generation; its job's `error(original Error)` maps through LI.28. A stays
  `cancelled(superseded(by: B.command))`, no second admission occurs and nothing from B installs. Four instances
  pass the ceiling in either case. The original code is also preserved for malformed (`EV3:input.display_plan_invalid`)
  or unsupported (`EV3:input.display_unsupported`) scheduled output, without reclassifying it as a refusal.
- **RM.36 [D]** Device-loss vectors use the same existing code in three contexts. With A pending and the device
  unavailable when standalone B is submitted, B is `refused(EV3:busy.renderer_unavailable)` with form `refusal`;
  B's refusal does not supersede A. Any loss affecting A itself is handled separately under RM.22. In a separate
  run where B has `running(job)` and no earlier terminal, loss before installation gives
  `JobResult.outcome = error(Error(code = EV3:busy.renderer_unavailable, form = stop))` and
  `Receipt.status = failed(Error)` for that same command/job via LI.28. In a third run where B has already installed,
  loss marks drawing unavailable and clears old picks/reports, while B stays `completed(view_published)`.
  `outcome(B.command, B.request)` returns that same terminal while retained under KN5.23/LI.30; it creates no retry.
- **RM.37 [D]** These jumbling vectors are specified source-contract outcomes, not new geometry or backend runs.
  Unless a row says otherwise, bindings are current, budgets fit and the producer has validated its exact data.

| Given | Expected result | Crosswalk |
| --- | --- | --- |
| A revision-12 pose index is paired with revision-13 pose/lattice tables | `refresh_required(EV3:input.display_bundle_mismatch)`; no mixed bundle is adopted | J01/J02/J06; JI.40/JI.41, RM.16 |
| A certified half-turn has no engine-named plane | No swept preview; no guessed plane, moving set or legality | J03; JI.41 |
| A full named-cap straddler set has pieces 7 and 9 and two verified point pairs; the cap-map witness names only piece 7 | The full-set payload retains both pieces, both pairs and all four points. The one witness cannot replace it; admission still checks the entire payload under RM.12/RM.20 | J04; JI.6/JI.41 |
| The same full straddler payload fits its declared output and memory allowances but exceeds a fixed renderer count ceiling; standalone B submits it while A is pending | `refused(EV3:resource_limit.display_plan_ceiling)` with form `refusal`, field, requested count and ceiling; A stays pending. No part of B installs, and no one-witness truncation or `KN7:budget.display_plan_limit` substitutes for that outcome | J04; RM.12/RM.20; CT-ERR-08 |
| In a separate run, scheduled B is already accepted and has superseded A before generating that same complete over-ceiling payload; producer budgets fit | `failed(EV3:resource_limit.display_plan_ceiling)` on B's original invocation, with the same field/counts and form `stop` in its job's `error(Error)` through LI.28. A stays cancelled; no second admission, resurrection or partial installation | J04; RM.12/RM.20; CT-ERR-08 |
| A 600-grip survey has 599 certified a/b entries and one uncertain entry; the named cut is supplied with alpha=121/125 | Survey remains incomplete/pending with one uncertain entry, never 600 certified statuses; the cut consumes the supplied exact plane, not a mesh-derived predicate | J04; JI.41 |
| A valid jumbling publication names an unsupported projection stage | `EV3:input.display_unsupported` under RM.20; no substitute projection | J05; JI.41 |
| A complete all-scope seam table has one on-grip and two off-grip mismatches; a separate enumeration stops before full coverage | Complete total N is 3, not 1; the interrupted result has undefined total N. Distinct domain/cell identities and off-grip entries stay in the payload | J07/J08; JI.8/JI.41 |
| A completed cover table at cap 0 has all 80 meeting caps; a completed repair list at that input is empty | Preserve all 80 cover rows in cap-ID order and the cap/input-scoped empty repair result; no global no-repair claim or default selection. A cut-off list supplies no completed negative | J09; JI.12/JI.41 |
| Certified exposed-face geometry or its size profile is missing | `KN2:input.exposed_geometry_unavailable`; no fabricated mesh, sticker ID or supported-feature claim | J10; JI.41/JI.45 with H15.16 |
| Release A is in flight when actual solver release B is accepted at revision 12; completed checks C then D already exist | A is superseded and cannot publish late; C then D keep their tried order and B may append only after completion. A change to revision 13 clears the revision-12 checks; no step flag supplies execution permission | J11; JI.6/JI.11/JI.41 |
| `nu` is requested off lattice while the fixed correction catalogue has 36 shipped rows | `undefined(off_lattice)`, no zero or old nu; all 36 catalogue rows keep fixed ID order with no state filtering, lighting, reordering or proposed combination | J12; JI.13/JI.41 |
| A valid lattice input returns all-zero nu and the caller supplies zero multiplicity for every correction row | The predicted nu vector remains all zero under JI.13's finite-group sum; all 36 rows remain in fixed order, and no multiplicity or word is selected by the check | J12; JI.13/JI.41 |
| A lattice nu check reports a nonzero fixed I_a coordinate | `KN2:consistency.fixed_nu_nonzero`; this failure cannot be displayed as a valid state | J12; JI.13/JI.41 |
| A current certificate-point hit targets off-lattice piece 7 and a named grip | Resolve the point's witness piece 7 and retain its grip and source role; no slot is invented and the hit certifies no inside/outside fact. A stale binding instead follows RM.10 before invocation or source navigation | J13; JI.39/JI.41, RPI.23/RPI.26 |

- **RM.40 [D]** These stage and view vectors use declared fixture limits, not measured production values. They are
  specified outcomes, not executed conformance evidence. A named allowance and a pixel count stand for one surface.

| Given | Expected result | Clause |
| --- | --- | --- |
| `adapter_gpu_bytes` ceiling 100; a hidden view retains 70; opening another view needs 70 | Refused with `EV3:resource_limit.display_plan_ceiling`, field `adapter_gpu_bytes`, requested 140 resident bytes (70 retained and 70 new) and ceiling 100, until retirement completes; hiding freed nothing | RPI.82; RM.39 |
| A view is hidden, evicted and waits for delayed retirement, then reopens before retirement ends | The retiring bytes stay charged; fresh resources must fit beside them or the open is refused; no early free or reuse | RM.19; RM.39 |
| Two plans with equal vertex counts, stage operations and surface and `overdraw` 4; one spreads (U = 3 × pixels), the other overlaps (U = 5 × pixels) | The first runs. The second stops before any user-stage fragment work with `EV3:resource_limit.stage_work_exceeded`, falls back and quarantines every user-stage digest of its chain | RPI.72, RPI.77 |
| A plan passes at one sample and one pass (U = 3 × pixels), `overdraw` 4 | At two samples with the ID pass U = 12 × pixels and the frame stops | RPI.72 |
| A triangle crosses the near plane | It counts the whole surface; with `overdraw` 1 and any other coverage the frame stops | RPI.72 |
| A line-only layout: 1,000 repeats of one diagonal edge across the surface, `overdraw` 4, a user stage, zero triangles in the plan | Each edge expands to counted triangles, U is about 2,000 × pixels and the frame stops before fragment work; U is never 0 | RPI.72 |
| View V has pending publication A and view W has pending publication C; a standalone publication B names V | B supersedes A and C runs on, because each publication binds its own target | RPI.10, LI.25 |
| A publication ends `completed(view_published)`; a later frame's raster bound is over the allowance | No user-stage fragment work runs, the view falls back and the chain's digests quarantine. `read_frame_reports` gives `EV3:resource_limit.stage_work_exceeded` with form `status`, and the receipt stays | RPI.72, RM.41 |
| `frame_work` ceiling 600, vertex term 100, `fragment_ops` 1, `overdraw` 4; the surface grows from 100 to 200 pixels | The charge goes from 500 to 900 before the first larger frame. The view falls back with visible status and no quarantine, and its stage returns when a recharge fits | RM.38, RM.39 |
| A chain with vertex stage V and fragment stage F has a device fault | Both digests quarantine. Later publications naming V alone or F alone refuse with `EV3:input.stage_quarantined` | RPI.77 |

- **RM.33 [O]** Remaining freeze inputs have named owners: the integrator verifies the JI.41/RM.34 crosswalk,
  KN5.21 to KN5.23, EV1.10 and LI.16/LI.28 consumers against the assembled H-08 and error registry. EN2 and the
  renderer line still supply J10's certified geometry, concrete added-layout versions and size profiles.
  LP1 and the renderer line pin finite production ceilings and resource accounting
  evidence; LB4/LP1 and the renderer line settle RPI.34/RPI.36's checker and precision inputs before enabling affected
  charts; the renderer line implements and verifies RM.26. View look and placement stay with the design line, and the
  owner answers any model/geometry change. No missing input is an implicit approval. The interface here is a
  draft contract, not a completed adapter or a claim that any candidate meets these added requirements.

The integrator adds these requirement rows to chapter 12. All are unrun; this packet's closure check checks manifest
structure and clause references only.

| Test | Checks | Class |
| --- | --- | --- |
| CT-RM-01 | Every applicable feature has a producer, typed payload, limit, lifecycle and support row; all 17 JI.41 rows crosswalk to J01 to J13, including all three J04 inputs; missing geometry, measured limits and backend evidence stay explicit (RM.1, RM.4, RM.27, RM.34) | source |
| CT-RM-02 | Ceiling 4 passes 4 instances; 5 refuses standalone admission but fails the original scheduled invocation, preserving field/counts and supersession; 100 output bytes admit 100 and cut off 101 without installation or terminal-reserve payload; compression changes neither outcome (RM.11 to RM.13, RM.35) | source |
| CT-RM-03 | Pending/completed supersession, rejected replacement, dependency change and budget exhaustion each produce exactly one KN5.21 terminal and no partial installation; renderer loss has refusal/stop forms on the two sides of acceptance and preserves installed success (RM.20 to RM.22, RM.32, RM.36) | source |
| CT-RM-04 | Derived/imported subjects retain identity and label; EV1.10 ranges and pinned old source survive editing; reused instance IDs fail binding checks before source-digest checks, including when both are stale (RM.8, RM.10, RM.30, RM.31) | source |
| CT-RM-05 | Actual device-loss injection during upload/draw/readback invalidates stamps, releases input state, safely retires resources and rebinds current data without changing receipts or session hashes (RM.18, RM.19, RM.23) | windows |
| CT-RM-06 | An actual candidate binds a report to the presented preview, withholds it for unconfirmed/dropped frames and handles reordered reports, no-preview frames, recovery and reopen; submit/Present return alone cannot pass (RM.24 to RM.26, RM.32) | windows |
| CT-RM-07 | JI.41 crosswalk vectors preserve complete point pairs, uncertain grips, off-grip seams/undefined totals, complete scoped lists, release order, undefined nu and unfiltered correction rows, unavailable geometry and canonical picks without granting authority (RM.34, RM.37) | source |
| CT-RM-08 | RM.38's weights, units and count rules and RM.39's ceilings are stated in a `LimitProfile`; a missing field makes the support report invalid (RM.11, RM.38, RM.39) | source |
| CT-RM-09 | RM.40's vectors give their stated outcomes: hidden bytes stay charged, retiring bytes block early reuse, and spread and overlapping plans, samples and the ID pass, a near-plane crossing and a line-only plan stop or run as stated; a publication to one view leaves another view's work alone, a later-frame stop keeps its receipt, a resize recharges `frame_work` and a chain's stages quarantine together (RM.38, RM.40, RM.41, RPI.77) | source |
| CT-RM-10 | On an actual backend the raster bound agrees with the formula on the same positions, a stopped frame runs no user-stage fragment work and audit counts stay at most U (RM.38, RM.40; RPI.72) | windows |

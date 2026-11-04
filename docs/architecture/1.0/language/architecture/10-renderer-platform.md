# Chapter 10: renderer and platform independence

Status: **draft for the owner. Not accepted.** Conventions are in [the overview](00-overview.md), section 3.

The language produces descriptions of views. An adapter turns those descriptions into drawing work for whichever
renderer the owner selects. This chapter defines the descriptions, their revision and identity rules, and the handoff
to the renderer line. It defines no syntax or implementation. LQ6, LQ7 and LQ20 remain choices, with their alternatives
kept below. Other language concepts retain their definitions in their owning chapters.

Sources: [partition](../ecosystem-partition.md), sections 8.5, 9 and 15; [overview](00-overview.md), sections 3 and 8;
[kernel](01-kernel.md); [DT-L1](../../../../progress/1.0/design-table/topics/language-ecosystem.md), E5, M4 and the
outcome; [render contract](../../../../../research/jumbling/render-contract.md), `magic600-jumbling-render/1`;
[renderer candidates and gate](../../../../wiki/decisions/renderer-candidates.md);
[renderer experiment plan](../../../../progress/1.0/renderer-experiment-plan.md);
[experiment protocol](../../../../progress/1.0/stage-2-experiment-protocol.md);
[V03.3, V08 and V09](../../10_V1_ARCHITECTURE.md), as proposals where marked;
[screening requirements](../../../../progress/1.0/requirements-from-screening.md), R-14, R-17 and R-19;
[charter](../../../../progress/1.0/charter-2.0-draft.md), D2, D8 and E2;
[owner, 2 October 2026](../../../../wiki/decisions/owner-decisions-2026-10-02.md);
[jumbling observation](../../../../progress/1.0/jumbling-observation.md).

## 1. Boundary and scope

- **RPI.1 [B]** No language feature depends on the renderer candidate (INV-8). A program publishes view-model content
  and display plans. It never calls a renderer API, supplies a device handle or receives a renderer callback. A backend
  change preserves the meaning of accepted plans.
- **RPI.2 [D]** The renderer adopts each engine revision bundle without waiting for program output (partition section
  9). A missing, slow or failed program view cannot hold that adoption. Kernel command dispatch also bypasses the
  runtime. The end-to-end turn path remains the one in the overview and chapter 01.
- **RPI.3 [B]** Drawing changes no mechanical state, canonical identity, legality, protection or evidence (INV-4,
  INV-6 and INV-8). The full mechanical analysis remains independent of visibility. For 600-cell-Full this preserves
  all 259,800 labelled sticker slots and all 1,200 legal generators (`AGENTS.md`, source requirements), even when only
  a subset is drawn or pickable.
- **RPI.4 [B]** Dimension is a parameter in headers, geometry, charts and adapter inputs. A reader refuses an
  unsupported dimension. Only d = 4 is implemented and accepted in 1.0 (render contract sections 1 and 8). Dimension
  parameters do not authorize another model identity or changes to the retained assets (INV-7).

## 2. View-model content and publication

- **RPI.5 [D]** A `ViewNode` is a declarative part of a program view (partition EV3). It describes an object, a
  relation, a container or a control, with references to its display plans and source linkage. It contains data and
  command references, never a function for the renderer to execute. Its semantic fields are:

  ```text
  ViewNode = {
    node:         NodeId
    base:         RevisionRef                 # KN0.2; the revision the output read
    kind:         object | relation | container | control
    content:      typed data or references to typed data
    plans:        List<DisplayPlanId>
    children:     List<NodeId>
    source:       source linkage of chapter 04
    commands:     command and Binding references of chapters 01 and 04
  }
  ```

  References preserve the types and levels of the data they name. A relation uses the grammar and evidence of chapters
  03 and 06. The node does not assign those meanings itself.
- **RPI.6 [D]** A `DisplayPlan` describes objects, charts, overlays and token colours for one declared subject, following
  partition section 8.5. Its semantic fields are independent of the adapter's buffers and pipelines:

  ```text
  DisplayPlan = {
    plan:         DisplayPlanId
    base:         RevisionRef
    subject:      StateRef                    # KN0.3; snapshot or derived-state reference
    derivation:   reference to KN2.11's derivation, when subject is derived
    dimension:    Nat
    geometry:     model identity and immutable geometry content digests
    primitives:   List<primitive from section 3>
    charts:       List<Chart>
    instances:    List<DisplayInstance>
    source_map:   SourceMapId
    precision:    declared display precision, as in chapter 09
    dependencies: revisions and immutable input digests actually read
  }
  ```

  The plan's base uses KN0.6. A plan does not introduce a second revision scheme or mutable puzzle state. The fields
  `base` and `dependencies` together realise chapter 08's `Base` (ONE.8): `base` is the revision read, and
  `dependencies` names which of its parts the output depends on.
- **RPI.7 [B]** A derived state is displayed only by a plan naming its derivation and base revision (INV-1, KN0.6,
  partition EV3 and KN2.12). The base is the display computation's session context; it does not give the derived state
  a session revision. The subject keeps its own digest, identity, level, lineage (ONE.21) and reachability claim from
  chapter 01. It is never substituted for the renderer's adopted session subject or offered as a state to commit.
- **RPI.8 [D]** Static geometry binds to model identity and content hash, following INV-7 and V03.3. Mutable labels,
  poses, exact sets and annotations bind to the revision or derived subject that produced them. The adapter validates
  those bindings before drawing; it cannot repair a mismatch by relabelling objects or recomputing engine predicates.
  Jumbling arrays come from `magic600-jumbling-render/1`. Its header lacks the menu-order field; the surrounding
  language binding supplies KN0.1's complete `IdentityBlock`. That header alone cannot replace the full binding.
- **RPI.9 [D]** An engine bundle is installed atomically, following partition section 9 and V03.3. Snapshot metadata,
  mutable labels and styles, and, for jumbling, pose indices, pose table and lattice data belong to the same revision.
  Missing or mismatched data requests a validated refresh. No frame combines arrays from different revisions. Advisory
  notifications do not replace reading the current revision or delta.

  ```text
  operation validate_engine_bundle(candidate: revision-bound bundle) -> complete bundle | refresh_required(Error)
    effect:  pure
    cost:    declared validation cost for its metadata and arrays
    errors:  EV3:input.display_bundle_mismatch
  ```
- **RPI.10 [D]** Program publication is a separate atomic installation of matching nodes, plans, instances and source
  maps, following EV3 and KN0.6. Validation happens before installation. An incomplete publication replaces none of
  that set. It never delays RPI.9. The command layer admits the view change through the existing path (INV-2):

  ```text
  operation publish_view(base: RevisionRef, nodes: List<ViewNode>,
                         plans: List<DisplayPlan>, maps: List<SourceMap>) -> Receipt
    effect:  change the view
    lane:    view, latest wins (KN7.2)
    cost:    declared construction and validation cost, plus logical output size
    errors:  EV3:input.display_plan_invalid, EV3:input.display_unsupported, EV3:stale.display_plan,
             KN7:budget.display_plan_limit
  ```

  The adapter consumes the installed data. This operation is not a language entry into the backend.
- **RPI.11 [B]** Program views do work only when visible and only for changed data, checking caches before rebuilding
  (R-19). Hidden views retain invalidation information without running their program or rebuilding their display
  resources. On becoming visible, they validate their dependencies against the current revision before reusing output.
- **RPI.12 [D]** Recomputations use the view lane's newest revision (KN7.2). A burst replaces pending work instead of
  building a queue for every intermediate revision. An old completion cannot overwrite a newer one. A result whose
  state dependencies no longer match stays stale; exact transport, if applicable, belongs to chapters 01 and 08 and
  is not accomplished by changing a plan's revision field (DT-L1 R2-E1).
- **RPI.13 [B]** A program view exposes its base revision and the host's empty, loading, error, stale and busy status
  (charter D2, partition EV3). The host derives staleness by comparing dependencies, independently of the program's
  assertions. Old state-bound overlays never attach to newly adopted geometry. Retained output, if shown, remains
  identifiable as output for its old subject until a matching publication arrives. It appears only in the program
  view's own stale presentation, naming its base revision, and is never drawn against the new state (ONE.11, ONE.12).
- **RPI.14 [D]** A view has no workspace-write effect (partition LG3, default 16 and DT-L1 fix-first item 3). A control
  refers to a command or binding defined in chapters 01 and 04; drawing or recomputing it invokes nothing. A workspace
  change still needs the solver-invoked command, and a commit still needs the shell's token. Program output therefore
  adds no actions to the durable undo order merely by refreshing.

## 3. Display primitives

- **RPI.15 [D]** The table is the semantic inventory handed to the renderer line under LQ20. Every row inherits the
  plan's base, subject, dimension, precision and identity binding. Object-bearing elements also carry the instance and
  source-map fields of section 4. The last column separates existing contract fields from language requirements and
  open dependencies; it makes no claim that an adapter exists.

| Primitive | Fields in addition to the shared binding | What the renderer line provides | Existing contract or dependency |
| --- | --- | --- | --- |
| Retained sticker objects | Geometry digests, canonical sticker and slot references, label data, visibility and interaction sets, token-name references | Geometry binding, per-object token lookup, separate draw and interaction eligibility | Static geometry in render contract section 2; label revision binding in V03.3; language instance mapping is additional |
| Posed sticker objects | `pose_index`, `poses`, `pose_lattice`, exact state digest and array hashes | Adopt the arrays together and apply the engine-supplied pose to each sticker's piece | Render contract sections 2, 3 and 7, including `slot_piece` and the canonical pose table |
| Certified transform data | `grip`, `plane_u`, `plane_v`, `angle`, certified `moving` piece set and hash, `from_revision`, `to_revision` | Consume the certified set and revision endpoints without deriving an inside set from floats | Render contract section 4; this chapter supplies no motion values or design rules |
| Chart stage | `Chart` reference, ordered stage position, parameters, dimension and precision | Validate supported stages; bind their parameters; evaluate per-vertex stages as section 5 requires | Render contract section 6 supplies `rotate`, perspective or stereographic, and `camera3`; generated stages depend on LQ7 |
| Object layout | Canonical object references, revision-bound CPU output positions and incidences, chart references where used | Draw the supplied sheets, atlases, trees, dimension-tagged cell complexes or state graphs; retain object mappings across copies | Partition LB4; these general layout fields are not in `magic600-jumbling-render/1` |
| Certified geometric overlay | Grip and witness-piece IDs, `grip_status`, certificate points as exact references and float vectors, `cut` | Draw the supplied geometric data independently of sticker meshes; map a certificate instance to its witness subject | Render contract section 5; points need not lie on any sticker |
| Exact-set or relation overlay | Revision-bound canonical object set and digest, typed relation or check references, missing coverage and token-name references | Bind supplied sets and relation data without redoing their exact tests per frame | Partition EV3, LB4 and jumbling observation sections 2 and 8; additional EN3 fields remain a dependency |
| Token colours | Token names associated with object classes or overlay data | Resolve names through the host's token table without using colour as identity or evidence | Charter D1; token references are language data, with no palette specified here |
| Derived-state display | Derived subject reference, derivation, lineage (ONE.21), reachability reference and a nested primitive set | Keep its subject distinct from the adopted session bundle, including picking and source mapping | INV-1 and EV3; not a session revision or a new state-adoption path |
| Preview display | Preview ID, base revision and the engine preview's witness and effect, as a nested primitive set | Draw it from the engine's preview object, keep it distinct from the adopted session bundle, and report which preview the newest presented frame shows | ONE.41, ONE.62 and KN5.9; the program supplies no preview data; a preview is not a session revision |

- **RPI.16 [B]** Jumbling drawing respects the geometry order and area-weighted shrink anchors in render contract
  section 2. Numbering centres are not those anchors. Certified points are read from their fields, never inferred from
  a sticker mesh (render contract section 5). These are geometry requirements, with no choice of shrink parameters.
- **RPI.17 [D]** A backend reports support or a typed refusal for each primitive and projection-chain stage, following
  INV-8 and render contract section 6. An unsupported primitive never silently becomes a different chart or subject.
  A display limitation does not reduce the mechanical model. Render visibility and interaction eligibility remain
  separate from exact analysis and protection (jumbling observation section 2).
- **RPI.18 [O]** Exposed-face geometry and cut-cell or off-grip-seam data are not supplied by the current render
  contract. Jumbling observation section 8 proposes additions. EN3 and the renderer line must settle their exact data,
  geometry identity binding and measured size before extending the contract. Options include engine-supplied geometry
  or references to separately specified derived geometry. Neither option permits changes to the immutable model
  boundary without its identity and migration process. No size or availability is assumed here.
- **RPI.19 [D]** A headless run produces nodes, plans, chart descriptors, instances, source maps, bindings and typed
  diagnostics as test data (partition section 8.5). It can validate geometry references, execute CPU layout and chart
  reference evaluations, and compare semantic outputs without a window or GPU. It produces no evidence of native
  frames, device recovery, input or performance. Stored byte forms belong to chapter 05; reproducible comparison
  belongs to chapter 09.

## 4. Display instances, source maps and picking

- **RPI.20 [D]** A `DisplayInstance` is one occurrence of a canonical object in one installed plan (default 7 and
  INV-6). Several occurrences can refer to the same object, for example in separate charts. Instance IDs distinguish
  those occurrences only within their installed view and plan generation:

  ```text
  DisplayInstance = {
    instance:     InstanceId
    view:         ViewId and generation
    plan:         DisplayPlanId
    base:         RevisionRef
    subject:      StateRef
    object:       canonical ID with its model and level, from KN1
    primitive:    primitive occurrence in the plan
    source_key:   entry in SourceMap
    interaction:  eligibility from the view's declared interaction set
  }
  ```

  A container or control without a canonical puzzle object remains a `ViewNode`. It does not acquire a fictitious
  puzzle ID to enter the picking system. The same applies to an abstract graph element without an object target.
- **RPI.21 [D]** A `SourceMap` maps display occurrences to canonical objects and source linkage, following EV3. It binds
  the plan generation and base revision, so an ID reused by a later publication cannot resolve through an old map:

  ```text
  SourceMap = {
    map:          SourceMapId
    plan:         DisplayPlanId and installed generation
    base:         RevisionRef
    entries:      Map<InstanceId, { object: canonical ID, source: chapter 04 source linkage }>
  }
  ```

  The object in each entry agrees with its `DisplayInstance`. Source linkage names the producing source version using
  chapter 04's contract. It does not silently retarget an older result to a newly edited module.
- **RPI.22 [D]** Every display instance maps back to a canonical ID (INV-6). Picking first obtains an instance ID and
  then resolves its source map (default 7 and EV3). Pixels, token colours, pose-table indices and chart inverses never
  identify an object. A many-to-one chart or a chart with no inverse remains pickable through this path.
- **RPI.23 [D]** Resolution names the requested object kind and identity level, following KN1 and default 7. The table
  states the permitted interpretation. Resolving a display occurrence does not change a selection role.

| Picked occurrence or requested level | Resolved subject | Constraint |
| --- | --- | --- |
| Sticker | Canonical sticker `(A, f)` in the plan's model and subject | Its home identity survives changes of pose or chart |
| Piece | Canonical piece ID, through the instance's declared sticker-to-piece association where needed | No inference from coincident projected vertices |
| Slot or position | The explicit slot target, or the kernel's exact projection for the bound state | Jumbling projections can be partial off the lattice; no slot is invented |
| Pose information | Exact pose reference obtained for the canonical piece at the bound subject | A pose-table index is meaningful only with that subject's canonical table |
| Cell, grip, cap or orbit | The canonical object explicitly targeted by that occurrence | A certificate point targets its witness piece, with its grip recorded separately; it is not a sticker hit |
| Layout copy or relation endpoint | The same canonical target as the original occurrence | Its layout position and graph index add no identity level |
| Derived-state occurrence | Canonical object together with its derived subject and derivation | It is not a selection in the live session until a command separately validates that request |

- **RPI.24 [D]** A pick request and reply bind the installed plan and source map, view generation, relevant state
  dependencies, camera version, viewport size, visibility revision and interaction revision. This follows KN0.6 and
  default 7 using V08's proposed invalidation fields. The integer-ID pass uses the drawing pass's camera, size and
  clipping. A reopened view, changed camera or replaced map cannot consume an old reply. Unrelated layout edits do not
  invalidate a query whose declared dependencies are unchanged (V03.3).
- **RPI.25 [D]** Stale hover and pick replies are discarded quietly, following KN0.6 and the V03.3 proposal. Explicitly
  requested work that is stale returns its typed reason for a visible explanation. Neither case silently retries a
  state-changing command. A resolution interface exposes the distinction:

  ```text
  operation resolve_pick(instance: InstanceId, map: SourceMapId,
                         dependencies: declared pick dependencies,
                         requested: object kind and level) -> Partial<canonical target and source linkage>
    effect:  read a snapshot, if an exact level projection is needed
    cost:    map lookup plus the declared kernel projection cost
    errors:  EV3:input.display_plan_invalid, EV3:stale.display_pick
  ```

  A valid partial answer off the lattice is not a stale answer. It uses KN0.4 and never invokes a chart inverse.
- **RPI.26 [D]** Linked representations carry the same canonical target and keep their separate instance IDs (LB4).
  The shell routes an explicit pick to the appropriate command or source-link action of chapter 04. A picked object
  grants no capability, supplies no commit token and certifies no claim (INV-3 and INV-5). Off the lattice a pick
  resolves to a piece, and no slot or position exists for it (KN1.7).
- **RPI.27 [O]** Multi-layer pick disambiguation remains V08's open product decision D-5. Front layer, working object
  and explicit depth choice are alternatives for the owner when the design line resumes. This chapter fixes the
  identity and staleness of any returned candidate, without selecting the policy or reopening that design work.

## 5. Charts and evaluation

- **RPI.28 [D]** A `Chart` declares its mathematical domain, map, inverse and loss (LB4). The declaration separates
  mathematical meaning from the approximate evaluator used to draw it:

  ```text
  Chart = {
    chart:        content identity of the definition and its parameters
    dimension:    input and output dimensions
    domain:       declared mathematical domain and its exact membership specification
    map:          mathematical definition
    inverse:      partial inverse with its domain, or none with a reason
    loss:         information and relations not preserved, and singular sets
    properties:   individually scoped claims and check references from chapter 06
    evaluator:    built-in stage, or the evaluator admitted under LQ7
    precision:    declared display precision
  }
  ```

  Loss includes any declared restriction or identification, such as clipping or a many-to-one projection. An inverse
  is mathematical data for declared uses, never an alternative picking implementation.
- **RPI.29 [B]** A float evaluation decides no exact predicate (INV-4). A drawn section, slab, clip or domain does not
  prove membership, equality, legality, admissibility, protection or reachability. Where those facts are needed,
  kernel or library exact results arrive with their subject, revision and evidence.
- **RPI.30 [D]** Checks of declared properties run at exact camera poses, the identity or an element of K⁺ (LB4.2; DT-L1
  R2-E2, R3-M2). Domain is decided by an exact region test, injectivity by a theorem or an exact region test, and section
  membership on the posed vertices. A vertex test can refute a region property and cannot establish one. Between those
  poses domain membership is a display decision. A check records the regions, vertices and poses it covers; a finite
  check does not prove an unstated claim over all points or all cameras (INV-5). Approximate camera matrices are not
  rounded into exact poses for a decision.
- **RPI.31 [D]** A projection chart receives frame-dependent vertex inputs during four-dimensional camera rotation or
  turn animation (partition section 8.5, DT-L1 E5). The gate's combined rotation and turn workload gives every vertex
  new input every frame. At full detail this is 18,288,000 vertex evaluations per frame: 30,480 base vertices times
  600 cells, counted from [`assets/mesh.json`](../../../../../assets/mesh.json), source evidence. This is a workload
  count, not measured throughput. Evaluating a chart once per session revision cannot serve that workload.
- **RPI.32 [P LQ7]** The recommended chart subset is pure and total, uses float32, has bounded loops, and permits no
  recursion, allocation or big integers. It compiles to portable HLSL. The CPU interpreter is its reference evaluator.
  The subset has no effects, engine calls, files, ambient inputs or renderer handles, following INV-4 and INV-8.
  Under the built-in-only alternative, users supply parameters to admitted built-in charts instead of subset programs.
  Under the user shader-language alternative, a separate safety and checking contract is needed (RPI.39).
- **RPI.33 [P LQ7]** Admission checks the subset and its bounds before shader generation. An evaluator outside it is
  refused before reaching a device. CPU reference evaluation and generated HLSL use the same checked definition,
  parameters and float32 meaning. Total evaluation includes a typed coverage result where the mathematical map is
  undefined; it does not require a finite point at a pole. This reference tests translation; exact mathematical
  property checks remain separate. Under the built-in-only alternative, the equivalent reference and checks belong
  to each built-in stage.
- **RPI.34 [O]** LB4 and LP1 must settle which domain specifications, inverse definitions and property claims have exact
  checkers. Options are a restricted family with known checkers, or additional chart definitions with independently
  checkable obligations. Total float32 execution alone establishes none of those properties. An unestablished property
  remains unknown and is not advertised as checked. The repertoire and obligation format must be settled before the
  corresponding charts are enabled; this chapter does not assume an exact decision procedure for arbitrary programs.
- **RPI.35 [D]** The shader correctness test compares GPU output against an exact or interval evaluation on the declared
  domain, excluding a stated neighbourhood of singular sets (partition section 8.5, DT-L1 M4 and R2-E2). It also compares
  the generated stage with its CPU reference within declared display precision. The test records the definition,
  parameters, inputs, excluded set and error rule. Agreement proves neither a chart property nor performance.
- **RPI.36 [O]** The precision rule and singular-set exclusion need the LB4, chapter 09 and renderer contracts before
  LX-7 is judged. Absolute, relative or ulp bounds are options; the metric, its units and any mixed rule must be stated.
  Clipping, omitting affected primitives or using a declared second chart are options for singular coverage. The
  exclusion neighbourhood and handling of triangles crossing it must be specified together. No numerical tolerance,
  exclusion radius or unmeasured cost is chosen here.
- **RPI.37 [D]** Outside the display domain, at a singular set or when evaluation cannot return finite coordinates, the
  result carries missing coverage and a reason, following KN0.4 and INV-4. Undefined coordinates enter neither drawing
  nor hit testing. The affected projection is incomplete; its underlying objects, exact state and claims remain
  unchanged. RPI.36 decides how to recover display coverage. The adapter does not fabricate a finite point or infer a
  canonical target by inversion.
- **RPI.38 [D]** Per-object sheets, atlases, trees and state graphs are evaluated on the CPU per revision (partition
  section 8.5). Their outputs then become instance data. Camera changes can redraw that data without rerunning exact
  relations or a layout program. Whole-state relations are computed once per relevant revision, never inside a frame
  loop (EV3 and DT-L1 R2-E4). R-19 still applies: only visible views with changed inputs rebuild. Projection stages
  consume changed frame inputs without turning into per-frame state-analysis jobs.
- **RPI.39 [O]** LQ7's alternative of a shader language for users has no safety contract in the sources. Its allowed
  operations, termination checks, memory access, precision and checking route must be specified and reviewed before
  enabling it. A constrained shader language or a validated subset are options. It must still meet the common
  requirements of RPI.40 and the renderer selection gate; this chapter grants no arbitrary shader upload capability.
- **RPI.40 [D]** Every LQ7 option keeps declarative plans, canonical instance picking, declared domain and loss, exact
  property checks, stated precision, device-fault isolation and platform separation (INV-4, INV-6, INV-8 and INV-9;
  LB4). Built-in parameterised charts are each measured in H-06 under that alternative. No option moves projection
  work into a per-revision CPU path that claims to meet the moving-camera workload without evidence.
- **RPI.41 [P LQ7]** User charts are enabled only when LX-7 passes. Until then the drawing path uses parameterised
  built-in charts, as the partition's recommendation states. A generated projection stage enters the proposed handoff
  under RPI.58. The built-in-only alternative has no user-chart activation step; the shader-language alternative
  requires its own settled contract and evidence, without weakening the gate.

## 6. Views, profiles and charter D8

- **RPI.42 [B]** Charter D8 remains in force as written unless the owner changes it: Global and Local are the only two
  views, visible together and linked to one context and revision. A chart or overlay is display content, not permission
  to create another product view. This chapter chooses no placement, appearance or interaction design within them.
- **RPI.43 [P LQ6]** Under the recommendation, solving-profile charts and overlays live inside Global and Local.
  Research-profile views may open in a research workspace, and each has its own gate run. Under strict D8 everywhere,
  those research displays also fit inside Global and Local. Under free views everywhere, the owner must reopen D8 and
  the renderer line must test the resulting workload. Profile definitions and grants remain in chapters 07 and 09.
- **RPI.44 [O]** LQ6 does not specify a separate placement rule for the tool-authoring profile. The owner must clarify
  whether it keeps D8 throughout or uses a research workspace for authoring tests. Until an answer, D8 provides the
  existing constraint. Neither editor services nor this chapter imply a new product viewport.
- **RPI.45 [D]** Display content retains the policy and provenance of its producing profile (partition section 11;
  chapters 07 and 09). Built-in tools keep the 2 October boundary and the 17 October exception until the joint setting.
  A chart or overlay cannot become a way to rank caps, choose targets or show forbidden solving output. Which display
  sinks flow control checks remains LQ2; its form remains LQ21 (DT-L1 R2-E3). The adapter does not decide authorship,
  understanding, declassification or evidence labels.

## 7. Device loss, invalid charts and limits

- **RPI.46 [B]** Device loss during any input releases the renderer's publication and input state; the renderer recovers
  or reports the failure (R-17). A program view becomes unavailable for drawing through that device, with the error
  status required by charter D2. Pending GPU transfers and pick readbacks cannot keep input latched or a publication
  held indefinitely. No mechanical operation depends on recovery (INV-8).
- **RPI.47 [D]** Recovery rebinds immutable geometry and reads a complete current engine bundle before resuming session
  drawing, following RPI.8 and RPI.9. Program publications are revalidated separately. Old device resources and replies
  cannot resolve through the new instance generation. A committed receipt remains committed even if its display failed
  (R-02, KN5.22). Recovery never retries a commit or adopts program-supplied state.
- **RPI.48 [D]** A chart that fails a subset, property or precision check cannot supply a checked display stage,
  following INV-4, LB4 and the LQ7 admission rule where applicable. The failure identifies the chart and check; affected
  output has error or incomplete coverage. Other valid display data can continue. No failure changes the session,
  protection, evidence for another claim or a receipt. Unknown check results remain unknown, not false or passes.

  ```text
  operation admit_chart(chart: Chart, checks: references to required check results)
      -> admitted | not_established(reason) | refused(Error)
    effect:  pure
    cost:    declared validation cost for the definition and check references
    errors:  LG2:type.chart_subset_violation, EV3:evidence.chart_check_failed
  ```

  `not_established` retains an unknown verdict. The subset error applies only under recommended LQ7. This operation
  consumes checks; it does not define the exact checker repertoire left open in RPI.34.
- **RPI.49 [D]** View construction, reference evaluation and output use the deterministic budgets of KN7 and chapter 09.
  Exceeding a plan limit refuses that publication before installation, following atomic publication and default 6.
  A cut-off computation does not publish a partial result as a complete view or a negative claim. The host can report
  the cut-off and retain separately identified old output, under RPI.13's rule; engine revision adoption proceeds.
- **RPI.50 [O]** LP1's budget contract and the renderer feature list must set the plan limits and their units, with
  H-09's constraints. Limits need to cover nodes, instances, output, chart expansion and overlay data before allocation
  or GPU submission. Fixed per-profile limits or declared per-run limits within host ceilings are options. Their values
  and backend resource accounting are unmeasured here. KN7's logical charges stay independent of allocator or driver.

## 8. Platform boundary and backend

- **RPI.51 [B]** 1.0 is Windows first. The engine, session store, command layer and view model contain no Windows-only
  code or APIs (owner, 2 October 2026). The runtime, libraries and checkers are headless and portable too (INV-9).
  Everything below the renderer runs headless on Linux. 1.0 includes no macOS or Linux product build, test or release.
- **RPI.52 [B]** Windowing, file locking, process ownership, platform paths and Direct3D 12 sit behind narrow platform
  interfaces (owner, 2 October 2026). A Windows-only dependency outside that layer requires a recorded reason and named
  replacement path for macOS and Linux. The language's public values and capabilities expose none of those APIs.
- **RPI.53 [B]** Shader source is written once in portable HLSL and compiled for Direct3D 12 now. The small renderer
  backend interface leaves later SPIR-V and Metal targets possible without rewriting higher layers (owner, 2 October
  2026). This is a source constraint and port path, not a claim that another platform backend exists or has passed.
- **RPI.54 [D]** The adapter validates a plan and compiles its data into backend work, following INV-8. The backend
  boundary covers immutable geometry binding, revision-bound resource adoption, admitted projection stages, drawing,
  stamped picking and device recovery. Framework objects, queues, fences and device handles stay on its renderer side.
  Engine and runtime interfaces accept only the portable data already defined by their owning chapters.

  ```text
  operation compile_display_plan(plan: DisplayPlan) -> adapter work | refused(Error)
    caller:  renderer adapter
    effect:  change the view, through the host
    cost:    declared plan validation and preparation cost
    errors:  EV3:input.display_plan_invalid, EV3:input.display_unsupported, EV3:busy.renderer_unavailable
  ```

  This is an adapter responsibility, not an operation granted to a program. The returned work stays inside the adapter.
- **RPI.55 [O]** LP1 and the renderer line must fix the small backend interface's signatures, support-report format,
  resource lifetimes and cancellation or completion rules before the renderer feature list freezes. Host-scheduled
  synchronous preparation and asynchronous preparation are options; neither may block engine bundle adoption on
  program output. This chapter fixes the responsibilities in RPI.54 without selecting a candidate's API or ABI.
- **RPI.56 [D]** LX-3 compares the same plans on Windows and headless Linux at their declared precision, under chapter
  09 (partition section 8.4 and DT-L1 R2-E2). Exact identities, source mappings, subjects and revisions agree exactly;
  approximate coordinates agree under the stated precision rule. Pixel equality, identical driver output or bitwise
  equality of floats across maths libraries is not promised. A plan never changes a mechanical answer to obtain a
  closer picture.

## 9. Handoff and evidence

- **RPI.57 [P LQ20]** The handoff enters the renderer feature list before its freeze. It contains RPI.15's primitive
  table; revision and derived-subject binding; atomic program publication; instance and source-map picking; the report
  of the preview that the newest presented frame shows (ONE.41); status and fault behaviour; precision and resource
  requirements; and the unresolved dependencies in section 12. Support, limits and evidence are recorded per feature.
  Sending the handoff does not claim acceptance or authorize implementation.
- **RPI.58 [P LQ7, LQ20]** If both recommendations are accepted, the handoff also includes a generated projection stage
  from RPI.32's subset, its CPU reference and the LX-7 check and measurement requirements. LQ7 alone does not answer
  LQ20. With built-in-only LQ7, the handoff lists those stages and their H-06 measurements instead. With the shader
  alternative, its unsettled safety contract is an explicit dependency before any generated user stage can be enabled.
- **RPI.59 [D]** Under LQ20's no-handoff alternative, display plans fit the frozen renderer as it is (partition LQ20).
  RPI.17 reports unsupported content. A user-chart proposal cannot reopen that freeze implicitly. If LQ7 and the frozen
  feature list cannot both be satisfied, the integrator returns the conflict and evidence to the owner; this chapter
  selects neither a renderer nor an exception.
- **RPI.60 [B]** The renderer remains unselected. Its selection gate stays the one in renderer-candidates: all 259,800
  sticker slots at full detail, the most complex turn animation and camera workload, average at least 30 fps and
  p99 frame time at most 33.3 ms, native resolution and no frame generation, on the owner's RTX 4070 Laptop GPU with
  8 GB VRAM and peak use at most about 7 GB. These are decision thresholds from that source, not measurements in this
  chapter. The experiment plan's W3 and W5 rules and correctness readbacks are not relaxed by a chart option.
- **RPI.61 [D]** LX-7 retains PresentMon on the owner's machine at W3, three cold runs per candidate comparing a
  generated chart stage with the built-in perspective stage (partition LX-7, DT-L1 E5). Captures and gate judgement
  follow the renderer experiment plan. GPU numerical correctness needs actual GPU output; the expected exact or
  interval values and CPU reference fixtures can be prepared headless. Headless preparation supplies no GPU or
  performance result. Additional research views admitted by LQ6 each require the prescribed gate run.
- **RPI.62 [D]** Active program views also enter the end-to-end charter E2 test (partition LP3 and DT-L1 E1, E4). A
  renderer frame-rate pass or a short chart timer cannot establish key-to-correct-frame latency. H-06 chart costs,
  LX-7, fault runs and E2 remain unmeasured by this chapter. Earlier bare-probe results establish none of them for a
  new generated stage or product workload.
- **RPI.63 [D]** Section 11 lists conformance obligations for chapter 12's phases, following the overview's convention.
  No conformance test in that table exists yet. An option-dependent test applies only after its named decision; its
  absence is not a pass. Synthetic or source checks never substitute for windows, performance or human evidence.

## 10. Added error codes

- **RPI.64 [D]** These proposed codes use default 5's categories and chapter 01's error convention. Chapter 13 owns
  final spelling. A display failure is separate from a command's mechanical outcome. Missing coverage, a partial level
  projection and an unknown verdict remain typed results, not new error categories.

| Code | Meaning and continuation |
| --- | --- |
| `EV3:input.display_plan_invalid` | Malformed plan, inconsistent subject or geometry binding, or invalid instance/source-map reference; refuse the publication before installation |
| `EV3:input.display_unsupported` | Unsupported dimension, primitive or projection-chain stage; report the unsupported part without silently changing its meaning |
| `EV3:stale.display_plan` | Publication no longer matches the dependencies of the view it would replace; recompute at the newest revision |
| `EV3:stale.display_pick` | Pick dependency or installed-generation mismatch; discard passive replies quietly and explain explicitly requested stale work |
| `LG2:type.chart_subset_violation` | Under recommended LQ7, the evaluator violates the subset or its checked bounds; no shader is submitted |
| `EV3:evidence.chart_check_failed` | A declared chart property or precision requirement fails its check; identify the failed obligation and withhold checked status |
| `KN7:budget.display_plan_limit` | Reserved until RPI.50 settles the limit (ERR.25): view construction or plan output exceeds its admitted limit; cut off the job and install no incomplete publication |
| `EV3:input.display_bundle_mismatch` | Metadata and mutable arrays do not form one complete matching engine bundle; request a validated refresh |
| `EV3:busy.renderer_unavailable` | Device loss or recovery prevents drawing or picking; report display availability separately from committed receipts |

## 11. Conformance tests

| Test | Checks | Class |
| --- | --- | --- |
| CT-RPI-01 | Boundary enumeration finds no language renderer calls, backend handles or callbacks; switching a mock backend leaves plan meaning unchanged (RPI.1, RPI.54) | source |
| CT-RPI-02 | Wrong model or geometry hash, unsupported dimension and unknown stage are refused before installation (RPI.4, RPI.8, RPI.17) | source |
| CT-RPI-03 | Delayed and reordered engine arrays never form a mixed bundle; a stalled program view does not block engine adoption (RPI.2, RPI.9) | synthetic |
| CT-RPI-04 | A publication with a missing source map installs none of its nodes, plans or instances; an older completion cannot replace a newer one (RPI.10, RPI.12) | source |
| CT-RPI-05 | A derived plan without derivation or base is refused; a valid one keeps its derived subject and never becomes session state (RPI.7) | source |
| CT-RPI-06 | Counters show no program work or resource rebuild for a hidden or unchanged view; becoming visible validates dependencies and skips to the newest revision (RPI.11, RPI.12, RPI.38) | source |
| CT-RPI-07 | Engine adoption makes mismatched program output stale; its overlays cannot attach to new geometry, and all D2 statuses have typed data (RPI.13) | synthetic |
| CT-RPI-08 | Headless fixtures produce and validate every admitted primitive row, with unsupported and missing-data cases explicit (RPI.15, RPI.19) | source |
| CT-RPI-09 | Every instance in duplicate charts and layouts resolves to the declared canonical object and producing source version (RPI.20 to RPI.23) | source |
| CT-RPI-10 | A many-to-one chart with no inverse is picked by instance ID; a planted inverse that throws is never called (RPI.22, RPI.25) | synthetic |
| CT-RPI-11 | Reused instance IDs, reopened views, camera changes and replaced maps discard delayed picks; an unrelated layout edit preserves an unchanged query (RPI.24, RPI.25) | synthetic |
| CT-RPI-12 | Explicitly requested stale resolution carries a visible-explanation reason; passive stale picking is quiet; off-lattice partial answers stay distinct (RPI.23, RPI.25) | source |
| CT-RPI-13 | The native ID pass and drawing pass share camera, viewport and clipping; non-interactive reference objects cannot be selected (RPI.24) | windows |
| CT-RPI-14 | At exact camera poses an exact region test or a theorem decides declared chart properties, and a vertex test only refutes them; intervening float poses cannot create exact verdicts (RPI.29, RPI.30, RPI.34) | synthetic |
| CT-RPI-15 | Recommended LQ7 refuses recursion, unbounded loops, allocation, big integers and effects before shader generation (RPI.32, RPI.33) | source |
| CT-RPI-16 | CPU reference fixtures test float32 evaluation and stated precision against exact or interval values outside the declared singular neighbourhood (RPI.33 to RPI.36) | synthetic |
| CT-RPI-17 | GPU readbacks agree with exact or interval expectations and the CPU reference on the admitted domain, using the recorded exclusion and error rule (RPI.35) | windows |
| CT-RPI-18 | Singular and domain-boundary fixtures produce missing coverage, no non-finite draw or pick coordinates, and unchanged exact subjects (RPI.36, RPI.37) | synthetic |
| CT-RPI-19 | One reproducibility key on Windows and headless Linux gives equal exact plan fields and approximate values within declared precision, including maths-library-sensitive cases (RPI.56, LX-3) | source |
| CT-RPI-20 | Dependency and interface inspection finds Windows APIs only behind platform interfaces and one portable HLSL source for each stage (RPI.51 to RPI.55) | source |
| CT-RPI-21 | Device loss injected during input, transfer and pick readback releases publication/input state, recovers or reports, and preserves session hashes and receipts (RPI.46, RPI.47, R-17) | windows |
| CT-RPI-22 | One-over-limit plans install nothing; failed chart checks affect only their display output; neither failure changes the session or certifies a negative (RPI.48 to RPI.50) | source |
| CT-RPI-23 | Conditional LQ6 profile fixtures admit only authorized view locations; built-in output keeps the interim boundary under either chart evaluator (RPI.42 to RPI.45) | source |
| CT-RPI-24 | First frames using a jumbling revision read back the bound pose indices, pose table and lattice arrays and compare them with that revision's engine data (RPI.9; render contract section 7) | windows |
| CT-RPI-25 | LX-7's three cold W3 runs per candidate use PresentMon and the unrelaxed renderer gate, comparing the generated stage with built-in perspective (RPI.60, RPI.61) | performance |
| CT-RPI-26 | End-to-end charter E2 holds with program views active, and R-19 gate captures confirm that hidden and unchanged views add no rebuilds (RPI.11, RPI.62) | performance |

## 12. Open items

| Item | What remains open and where it is used |
| --- | --- |
| LQ6 | Whether research workspaces may host views outside Global and Local, or D8 stays strict everywhere, or free views reopen D8 (RPI.42, RPI.43). Tool-authoring placement is not specified by the recommendation (RPI.44 [O]). |
| LQ7 | Built-in parameterised charts, the recommended compiled subset, or a user shader language (RPI.28 to RPI.41). Exact checker coverage is an LP1/LB4 dependency (RPI.34 [O]); the shader-language alternative needs a safety contract (RPI.39 [O]). |
| LQ20 | Whether to add the renderer handoff before its freeze or fit the frozen renderer unchanged (RPI.57 to RPI.59). The existing handoff tables are not amended by this chapter. |
| LQ1, LQ16 | The language/profile arrangement and final product cut remain open. The portable data boundary still applies to any included producer; no research viewport is promised independently of LQ6 (RPI.1, RPI.19, RPI.43). |
| LQ2, LQ21 | The joint boundary policy, checked display sinks and form of flow control, owned by chapter 07. No new permission follows from display or picking (RPI.14, RPI.26, RPI.45; DT-L1 R2-E3). |
| LQ3, LQ10 | Evidence and stored forms remain in chapters 05 and 06. Chart and reachability references retain their fields and scope without settling either question (RPI.5 to RPI.7, RPI.28, RPI.48). |
| LQ4 | Branch display and pinned-job lifetime remain in chapters 01 and 08. Reuse at another revision needs the existing rerun or exact-transport rule (RPI.7, RPI.12; DT-L1 R2-E1). |
| LQ5, LQ8, LQ9 | Built-in implementation, engine language and runtime placement do not change this boundary. Each chosen placement must preserve the independent adoption and fault rules (RPI.1, RPI.2, RPI.46 to RPI.54). |
| LQ18, LQ19 | Authorization of evidence spikes and the order of freeze gates are still owner questions. LX-3 and LX-7 are obligations, not runs authorized or performed here (RPI.56 to RPI.63). |
| EN3 and geometry additions | Exact overlays and new geometry need data, identity binding and measured sizes; exposed faces are absent from the current render contract (RPI.15, RPI.18 [O]). Geometry changes still need the immutable-model process. |
| EN4 and LP1 | Final RevisionRef fields, the view lane and command-layer publication contract remain dependencies from chapter 01 (RPI.6, RPI.10, RPI.12). Source linkage and editable parameters remain in chapter 04; their stored forms remain in chapter 05. |
| LB4, chapter 09 and LX-7 | Error metrics, singular neighbourhoods and crossing-primitive handling need one agreed precision contract before shader acceptance (RPI.36 [O]). Exact checks and shader agreement remain separate (RPI.30, RPI.35). |
| Renderer interface and H-09 | Plan ceilings and accounting are unsettled (RPI.50 [O]); backend signatures, support reports and resource lifetime rules are unsettled (RPI.55 [O]). The renderer line fixes them before its feature freeze. |
| Picking policy D-5 | Multi-layer disambiguation awaits the owner when design work resumes (RPI.27 [O]). Identity resolution and stale-reply handling do not depend on its answer. |
| Renderer selection and evidence | No candidate is selected. H-06 chart costs, LX-7, product fault tests and E2 with program views still need evidence for the matching candidate (RPI.60 to RPI.63). |
| DT-L1 obligations | Round 3 cleared them (overview section 8). Dependencies on R2-E1, R2-E2, R2-E3 and R2-E4 are named at RPI.12, RPI.30, RPI.35, RPI.38, RPI.45 and RPI.56, and RPI.30 carries round 3's correction. |
| EV3.12 | Settled as a default: the plan states lineage `imported` with the import's digest and uses an explicit comparison base with no claim that it produced the value (ONE.21). The reading of partition EV3 is the integrator's. |
| Source differences to reconcile | The old stack and process choices in V02 and V11 and the dated schedule/design work in the experiment documents are superseded as the briefing records. Their surviving revision and gate contracts are used here (RPI.9, RPI.24, RPI.51 to RPI.63). The render header lacks KN0.1's menu-order field; the language envelope retains the full IdentityBlock without changing that format (RPI.6, RPI.8). Partition 4.1's derived-state base revision is the value's lineage, and the plan's base is the display context; ONE.21 separates them, and neither is a session revision on the derived value (RPI.7). |

# Chapter 10: renderer and platform independence

Status: **draft for the owner. Not accepted.** Conventions are in [the overview](00-overview.md), section 3.

The language produces descriptions of views. An adapter turns those descriptions into drawing work for whichever
renderer the owner selects. This chapter defines the descriptions, their revision and identity rules, and the handoff
to the renderer line. It defines no syntax or implementation. The owner answered LQ6, LQ7 and LQ20 on 6 November 2026:
free views, a shader language for users and a handoff before the renderer freeze. Other language concepts retain
their definitions in their owning chapters.

Sources: [partition](../ecosystem-partition.md), sections 8.5, 9 and 15; [overview](00-overview.md), sections 3 and 8;
[kernel](01-kernel.md); [DT-L1](../../../../progress/1.0/design-table/topics/language-ecosystem.md), E5, M4, R4-M43 and the
outcome; [render contract](../../../../../research/jumbling/render-contract.md), `magic600-jumbling-render/1`;
[renderer candidates and gate](../../../../wiki/decisions/renderer-candidates.md);
[renderer experiment plan](../../../../progress/1.0/renderer-experiment-plan.md);
[experiment protocol](../../../../progress/1.0/stage-2-experiment-protocol.md);
[V03.3, V08 and V09](../../10_V1_ARCHITECTURE.md), as proposals where marked;
[screening requirements](../../../../progress/1.0/requirements-from-screening.md), R-14, R-17 and R-19;
[charter](../../../../progress/1.0/charter-2.0-draft.md), D2, D8 and E2;
[owner, 2 October 2026](../../../../wiki/decisions/owner-decisions-2026-10-02.md);
[owner, 6 November 2026](../../../../wiki/decisions/owner-decisions-2026-11-06.md);
[jumbling observation](../../../../progress/1.0/jumbling-observation.md);
[renderer feature matrix](../renderer-feature-matrix.md); [layer interfaces](../../../../progress/1.0/layer-interfaces.md),
LI.7, LI.16 and LI.25 to LI.30; [jumbling interfaces](../../../../progress/1.0/jumbling-interfaces.md), JI.40 to JI.46;
[closure working forms](../closure/HANDOFF.md), H15.8 to H15.10 and H15.16.

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
  command references, never a function for the renderer to execute. A stage module is the one exception: it reaches
  the adapter as a checked definition by digest, compiled by the host, never as a callable (RPI.68). The node's
  semantic fields are:

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
    claims:       List<Key>                   # chapter 06; empty when the node states no claim
  }
  ```

  References preserve the types and levels of the data they name. A relation uses the grammar and evidence of chapters
  03 and 06. The node does not assign those meanings itself.
- **RPI.65 [D]** Content shown in a view is one of three kinds. *State data* is what a kernel row returned for the
  subject at `base`: labels, poses, exact sets, counts, partial results and verdicts, with any H-08 evidence state as
  the engine gave it. Its support is that revision and that row, so it needs no statement reference. A *claim* is a
  relation (EV3.8), a declared property, or anything shown with a word of EVD.32. It carries its `Key` in `claims`.
  *Program content* is everything else a program publishes. It is shown as the output of its producing module and
  carries no word of EVD.32. A word of EVD.32 reaches a display only through `word()` on a key, with the phrase of
  EVD.36, and a view asks for it only on a key that has a record: with none, row 7 would call a kernel read open. A
  node of kind relation with empty `claims` is invalid with `EV3:input.display_plan_invalid`; RM.20 maps that error
  to standalone refusal or scheduled failure. Text a program writes
  stays the program's text, whatever it says: the host does not read it for evidence. How the three kinds look is not
  set here.
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
  An imported value uses EV3.12's imported lineage and comparison base without inventing a derivation. RM.8 specifies
  the host-owned `subject_kind = derived` identification and its fields, including for stale output and picks. It is
  drawn from that subject's immutable kernel data, alongside the separately bound session subject (ONE.62), and can
  supply no presented-preview identity for a commit token. Its appearance remains with the design line.
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
  operation publish_view(target: ViewId, base: RevisionRef, nodes: List<ViewNode>,
                         plans: List<DisplayPlan>, maps: List<SourceMap>) -> Receipt
    effect:  change the view
    lane:    view, latest wins (KN7.2)
    cost:    declared construction and validation cost, plus canonical uncompressed output bytes under LI.16
    errors:  EV3:input.display_plan_invalid, EV3:input.display_unsupported, EV3:stale.display_plan,
             EV3:resource_limit.display_plan_ceiling, KN7:budget.output, EV3:busy.renderer_unavailable
  ```

  `target` names the view. A standalone publication binds it into the request before admission (LI.5, LI.22); a
  scheduled publication keeps its invocation's target and generation (LI.25). KN5.21 supplies the Receipt and terminal forms; LI.7/LI.25 and RM.20 to RM.23 specify their publication transitions.
  Success is
  `completed(view_published)`, never `committed`; a pending superseded publication ends as
  `cancelled(superseded(by: CommandId))`. An already completed publication keeps its terminal when replaced. The
  adapter consumes installed data. Installation is not presentation (ONE.41), and this operation is not a language
  entry into the backend. RPI.67 binds the output cut-off to the same lifecycle.
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

- **RPI.15 [D]** The table is the semantic inventory handed to the renderer line through row H-11 (owner, 6 November
  2026, LQ20). Every row inherits the plan's base, subject, dimension, precision and identity binding. Object-bearing
  elements also carry the instance and source-map fields of section 4. The
  [feature matrix](../renderer-feature-matrix.md), RM.5 to RM.27, supplies the per-feature producer, payload layout,
  operation, limits, ownership, replacement, cancellation, refusal and required or optional disposition. Its mark
  column states how each row enters the feature list. The table below remains the semantic index, not a support
  claim. RM.4/RM.34 crosswalk JI.41's jumbling producers to matrix rows J01 to J13.

| Primitive | Fields in addition to the shared binding | What the renderer line provides | Existing contract or dependency |
| --- | --- | --- | --- |
| Retained sticker objects | Geometry digests, canonical sticker and slot references, label data, visibility and interaction sets, token-name references | Geometry binding, per-object token lookup, separate draw and interaction eligibility | Static geometry in render contract section 2; label revision binding in V03.3; language instance mapping is additional |
| Posed sticker objects | `pose_index`, `poses`, `pose_lattice`, exact state digest and array hashes | Adopt the arrays together and apply the engine-supplied pose to each sticker's piece | Render contract sections 2, 3 and 7, including `slot_piece` and the canonical pose table |
| Certified transform data | `grip`, `plane_u`, `plane_v`, `angle`, certified `moving` piece set and hash, `from_revision`, `to_revision` | Consume the certified set and revision endpoints without deriving an inside set from floats | Render contract section 4; this chapter supplies no motion values or design rules |
| Chart stage | `Chart` reference, ordered stage position, parameters, dimension and precision | Validate supported stages; bind their parameters; evaluate per-vertex stages as section 5 requires | Render contract section 6 supplies `rotate`, perspective or stereographic, and `camera3`; user stages follow RPI.68 to RPI.80 |
| User stage | Stage digest and compile record, kind `vertex` or `fragment`, declared inputs and bounds | Install the host-compiled stage; run the raster bound before any fragment work; fall back with a visible status | RPI.68 to RPI.80; no user stage draws a presented preview (RPI.76) |
| Object layout | Canonical object references, revision-bound CPU output positions and incidences, chart references where used | Draw the supplied sheets, atlases, trees, dimension-tagged cell complexes or state graphs; retain object mappings across copies | Partition LB4; these general layout fields are not in `magic600-jumbling-render/1` |
| Certified geometric overlay | Grip and witness-piece IDs, `grip_status`, certificate points as exact references and float vectors, `cut` | Draw the supplied geometric data independently of sticker meshes; map a certificate instance to its witness subject | Render contract section 5; JI.41 and matrix J04 distinguish a cap-map witness from every point pair in the full named-cap set |
| Exact-set or relation overlay | Revision-bound canonical object set and digest, typed relation or check references, missing coverage and token-name references | Bind supplied sets and relation data without redoing their exact tests per frame | Partition EV3, LB4 and JI.41's observation payloads; matrix J07 to J12 preserve their coverage and undefined results |
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
  The report and applicable required list are RM.14/RM.15 and RM.27. Omitting a required row is an unmet delivery
  requirement even when the refusal itself is correct; optional rows alone permit conforming omission. A
  required-feature conflict goes back to the owner with its evidence; this chapter selects neither a renderer nor an
  exception.
- **RPI.18 [O]** The current render/1 format has no exposed-face meshes or cut-cell/off-grip-seam additions.
  JI.8/JI.41 now supply the draft exact seam and geometry payload contracts, crosswalked in matrix J08 and J10.
  J10 preserves `KN2:input.exposed_geometry_unavailable` when certified geometry or its size profile is absent;
  the interface text supplies neither a mesh nor a measurement. EN2 and the renderer line still supply geometry
  production, concrete layout bindings and measured sizes before support can be advertised. Engine-supplied geometry
  and references to separately specified derived geometry remain alternatives. A model/geometry identity change
  remains an owner item under the immutable-model process. No missing mesh or unresolved binding counts as implemented.
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
  the plan generation, base revision and producing source digests, so an ID reused by a later publication cannot
  resolve through an old map:

  ```text
  SourceMap = {
    map:          SourceMapId
    plan:         DisplayPlanId and installed generation
    base:         RevisionRef
    sources:      producing digests and pinned immutable source references, or explicit missing source
    entries:      Map<InstanceId, { object: canonical ID, source: chapter 04 linkage using EV1.10 SourceRange }>
  }
  ```

  The object in each entry agrees with its `DisplayInstance`. EV1.10 supplies the coordinate contract; RM.10 specifies
  digest/range validation, missing-source data and EV1 conversion. A pick checks its bindings first and returns
  `EV3:stale.display_pick` on mismatch. Only a subsequent source action against another producing digest returns
  `EV3:stale.source_link`; malformed publication ranges return `EV3:input.display_plan_invalid`. Old output
  keeps its producing source version across editing or activation, never a nearby declaration in the new source.
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
  dependencies, camera version, viewport size, visibility revision and interaction revision, plus RM.24's device
  epoch and surface generation. This follows KN0.6 and default 7 using V08's proposed invalidation fields. The integer-ID
  pass uses the drawing pass's camera, size and clipping. A reopened view, changed camera, replaced map or recovered
  device cannot consume an old reply. Unrelated layout edits do not invalidate a query whose declared dependencies
  are unchanged (V03.3).
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
    evaluator:    built-in stage, or a checked vertex stage (RPI.68)
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
- **RPI.32 [B] Withdrawn.** LQ7's answer replaces the compiled chart subset with stage modules under RPI.68 to RPI.80
  (owner, 6 November 2026).
- **RPI.33 [B] Withdrawn.** Admission and the CPU reference now sit in RPI.69 and RPI.74 (owner, 6 November 2026, LQ7).
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
- **RPI.39 [B] Withdrawn.** RPI.68 to RPI.80 are the safety contract this clause asked for (owner, 6 November 2026,
  LQ7).
- **RPI.40 [D]** User stages keep declarative plans, canonical instance picking, declared domain and loss, exact
  property checks, stated precision, device-fault isolation and platform separation (INV-4, INV-6, INV-8 and INV-9;
  LB4). Built-in parameterised charts stay, each measured in H-06. No projection work moves into a per-revision CPU
  path that claims to meet the moving-camera workload without evidence.
- **RPI.41 [B] Withdrawn.** User stages are enabled under RPI.79 and RPI.80, with parameterised built-in charts as the
  drawing path until then (owner, 6 November 2026, LQ7).

### User stages

- **RPI.68 [B]** Users write shader stages (owner, 6 November 2026, LQ7). The project's own pipeline checks a stage and
  compiles it to the one portable HLSL source of RPI.53, and the upload of raw shader code has no path (reading of
  decision 2). [D] A stage module is a module kind of 600 Cell Script with its own checker and only the `pure` effect
  (LG2.24, LG3.17, LG5.12). There is no fourth profile; it uses the syntax family LQ13 selects, and any profile may
  use it. A stage has one of two kinds. A `vertex` stage is a chart evaluator with a declared domain that returns a
  position or a coverage code, as in LX-7's subset. A `fragment` stage returns an opaque RGB value. There is no
  compute stage, because scatter writes need memory rules this contract does not give. A stage reaches the adapter as
  a checked definition by digest, compiled by the host, never as a callable or as shader text from a program.
- **RPI.69 [D]** The stage checker refuses anything outside these limits with `LG2:type.stage_rule_violation`, before
  anything reaches a device. A stage computes in binary32 only. It has no recursion (calls are inlined), no loop
  without a static bound, no allocation, big integers, pointers, writes, atomics, barriers or readback, and no
  effect, engine call, file or renderer handle. An array index into a declared immutable input is checked statically.
  Frame time is a declared host input, never an ambient read. Evaluation is total: where the mathematical map is
  undefined a vertex stage returns a typed coverage code, and it need not produce a finite point at a pole. CPU
  reference evaluation and generated HLSL use the same checked definition, parameters and binary32 meaning.
- **RPI.70 [D]** The operation table has tiers: `+ - * /`, unary minus and `abs`, then `sqrt`, then `sin`, `cos`,
  `exp2` and `log2`. A tier is enabled only when its error rule is stated. Those values stay under RPI.36 [O].
- **RPI.71 [D]** Every stage declares static costs, and admission charges them against RM.38's fields. `stage_ops` is
  the maximum weighted checked scalar operations for one vertex evaluation of the expanded chain. `fragment_ops` is
  the same for one fragment invocation. A bounded loop counts at its bound, and weights come from RM.38's measured
  table. `overdraw` is the allowed fragment invocations per surface pixel, counted by RPI.72's raster bound over every
  pass and shading sample. `frame_work` is the combined ceiling charged at admission: the sum over visible views of
  vertex evaluations times `stage_ops`, plus surface pixels times `overdraw` times `fragment_ops`. It is a real bound
  because of RPI.72. A stage over a fixed checker bound gives `LG2:type.stage_rule_violation`; a publication over a
  `LimitProfile` field gives RM.12's `EV3:resource_limit.display_plan_ceiling`.
- **RPI.72 [D]** Static bounds cannot bound fragment invocations. A depth pre-pass does not help, because a user
  vertex stage can put every triangle at one depth, and a vertex stage can inflate coverage even with the built-in
  fragment stage. So every frame with a user stage in its chain, vertex or fragment, runs a raster bound before any
  fragment work:

  1. The adapter evaluates the vertex stage into a charged buffer (`view_vertex_evals` times `stage_ops`, under
     `adapter_gpu_bytes`). The draw consumes these positions, so the bound and the draw see the same triangles.
  2. Every raster primitive of such a frame is a triangle when the bound is computed. The adapter expands each line
     and point into counted triangles, a screen-space quad of the primitive's drawn width, from the evaluated
     positions, in the same charged buffer, and the draw consumes those triangles. No native line or point primitive
     is drawn in such a frame.
  3. On the device, before any fragment runs, it computes U: the sum over triangles of the triangle's bounding
     rectangle clipped to the surface and widened to whole 2×2 quads (helper lanes), times the shading samples per
     pixel, summed over every pass including the ID pass. A triangle wholly outside the clip volume counts zero and
     draws nothing. One that crosses the near plane counts the whole surface. Undefined coordinates enter neither the
     sum nor the draw (RPI.37). U is pinned to this formula, with integer pixel arithmetic after rounding the
     rectangle outward, so that backends working from the same positions agree on which frames pass.
  4. If U exceeds surface pixels times `overdraw`, no user-stage rasterization or fragment work of that frame runs.
     The view falls back to the built-in chain with visible status, every user-stage digest of the chain quarantines
     under RPI.77, and RM.41's frame report carries `EV3:resource_limit.stage_work_exceeded`.

  How the device skips the work (predication, indirect arguments or an equivalent) is the backend's, and each
  candidate states it. A backend that cannot skip cannot advertise user stages (RPI.80). The whole chain, bound pass
  included, passes the unchanged gate together (RPI.79). Post-frame counts stay an audit only: a frame counted above
  U is an adapter fault (CT-RPI-31). Quarantine answers a frame that was stopped; it is not what keeps the charge.
- **RPI.73 [D]** A stage is display only. No stage result returns to any program (INV-4, RPI.29). A pick returns an
  instance ID to the host. Stage inputs pass the same `display` sink as `publish_view` (RPI.10; chapter 07).
- **RPI.74 [D]** The CPU interpreter evaluates the same checked definition and is the stage's reference. This tests
  translation; exact mathematical property checks remain separate. Admission separates safe to run from properties
  established: `admit_chart` keeps `not_established` (RPI.48), and RPI.30 and RPI.34 stay. A GPU agreement check
  (RPI.35) is available and its result is recorded; it proves no chart property.
- **RPI.75 [D]** The ID pass uses the same vertex stage as drawing (RPI.24), and undefined coordinates enter neither
  (RPI.37). A fragment stage writes opaque RGB only: it has no discard and no depth, stencil or coverage change.
  Transparency is a later row. Colour is never identity or evidence (charter D1).
- **RPI.76 [D]** Host statuses (preview, stale, protection breakthrough and selection) are composited after user
  stages. User stages never draw the preview that a commit binds to. ONE.41's and RM.26's presented-preview report
  covers only frames drawn by the built-in chain. A preview shown through a user stage is an extra display with no
  presented-preview identity, as RPI.7 says of derived subjects.
- **RPI.77 [D]** Static bounds admit a stage; they cannot prove a driver correct. A device fault in a frame that used
  a user stage quarantines every user-stage digest of its expanded chain for the session, and so does a frame stopped
  under RPI.72. The view falls
  back to the built-in chain with visible status (RPI.46 to RPI.48, R-17); a view with no built-in chain for that plan
  shows RPI.13's error status instead. A later publication naming any of those digests ends with
  `EV3:input.stage_quarantined` under RM.20. No TDR or security setting changes.
- **RPI.78 [D]** The same source and compiler version give the same HLSL bytes. The stage digest and compile record
  join the plan's identity. Compilation runs off the frame path under a budget, with `loading` status and the
  built-in fallback. A host compile failure gives `EV3:input.stage_compile_failed`, and the built-in chain stays.
- **RPI.79 [B]** No gate is relaxed: RPI.60's selection gate applies unchanged with user stages in the chain (owner,
  3 November 2026; reading of decision 1). [D] Ceilings come from measurement. The advertised combination of views,
  vertex stage, fragment stage and overdraw allowance must pass the unchanged gate together in one run, with the
  bound pass included; separately measured ceilings do not establish the combination. Values stay unset until such a
  run exists, and no support is advertised until then (RM.12). LX-7's `lx7-r1` results (4 accepted sources, 18
  refused sources, 496 readback records) apply to the compute subset they tested. This contract needs a new safety
  test set and vertex and fragment stages in a drawing pipeline at W3 (RTE.59 to RTE.61, RTE.83; RM.40).
- **RPI.80 [D]** The feature-matrix rows for user stages are required (mark `G`, RM.2). Each renderer candidate states
  how a stage compiles at run time and how the device skips a stopped frame. A candidate with no run-time compile
  path or no way to skip fails that row, and RPI.17's rule returns the conflict to the owner. DXC is not on the
  toolchain allowlist, so shipping a compiler is an "Ask the owner" item.

## 6. Views, profiles and charter D8

- **RPI.42 [B]** Charter D8 stays in force with its limit on the number of views lifted (owner, 6 November 2026, LQ6).
  Global and Local stay, each answers its own question, and the two are visible together and linked to one context
  and revision. Further views are allowed in every profile (RPI.81 to RPI.83). A chart or overlay is display content.
  This chapter chooses no placement, appearance or interaction design for any view.
- **RPI.43 [B] Withdrawn.** LQ6's answer removes the profile split of view locations; RPI.81 to RPI.83 replace it
  (owner, 6 November 2026).
- **RPI.44 [B] Withdrawn.** Free views everywhere cover the tool-authoring profile too; look and placement stay with
  the design line (owner, 6 November 2026, LQ6).
- **RPI.45 [D]** Display content retains the policy and provenance of its producing profile (partition section 11;
  chapters 07 and 09). Built-in tools keep the 2 October boundary and the 17 October exception until the joint setting.
  A chart or overlay cannot become a way to rank caps, choose targets or show forbidden solving output. Which display
  sinks flow control checks stays in LQ2's option set; its form waits for LQ21 (DT-L1 R2-E3). The adapter does not
  decide authorship, understanding, declassification or evidence labels.
- **RPI.81 [D]** A further view has a `ViewId` in LI.22's view namespace. It shows one revision (`base`, RPI.6), keeps
  instance identity and source linkage (RPI.20 to RPI.22) and shows the host statuses of RPI.13. A further view
  widens nothing the session may show: display keeps its policy (RPI.45, BND.26), built-in tools keep the boundary,
  and BND.59 stays open under LQ2's option set. No view gains a program commit path (RPI.14). Look and placement stay
  with the design line.
- **RPI.82 [D]** Free means no structural limit on views. The only limit is a stated resource budget, in RM.39's
  fields. `view_vertex_evals` caps per-frame vertex evaluations summed over visible views; a hidden view adds nothing
  (RPI.11, R-19). Hiding stops evaluation and rebuilding and frees nothing: hidden allocations stay charged under
  `adapter_cpu_bytes` and `adapter_gpu_bytes` until safely released, including eviction and delayed retirement
  (RM.19). Opening or showing a view past either ceiling is refused with `EV3:resource_limit.display_plan_ceiling`,
  and there is no unbounded queue. The ceiling value comes from measurement: the largest stated workload for which
  the renderer has a passing run of the unrelaxed gate. A failed workload lowers the ceiling and never moves the gate
  (RPI.60). Until such a run exists the value is unset and no support is advertised (RM.12).
- **RPI.83 [D]** The command table has two rows for further views (H-08). `view.open {view: ViewId, source:
  ViewSourceRef}` opens a view and `view.close {view: ViewId}` closes one. Both are kind `view` on the `view` lane with
  `lane_target {"arg":"view"}`, undo `none` and no gates. `ViewSourceRef` names what fills the view:

  ```text
  ViewSourceRef = built_in(name: Text) | overlay(row, module: Digest<source>, arguments: typed data)
  ```

  `row` names an overlay row of kind `view` (KN5.3). Either row runs only inside a command the solver invoked. A
  recomputation started by a revision change holds no permission for either row, so a view cannot open views (compare
  BND.30). Output reaches an open view through `language.publish_view`. Global and Local cannot be closed. Naming an
  open view gives `EV3:input.view_id_in_use`; closing an unknown view, Global or Local gives
  `EV3:input.view_not_closable`.

## 7. Device loss, invalid charts and limits

- **RPI.46 [B]** Device loss during any input releases the renderer's publication and input state; the renderer recovers
  or reports the failure (R-17). A program view becomes unavailable for drawing through that device, with the error
  status required by charter D2. Pending GPU transfers and pick readbacks cannot keep input latched or a publication
  held indefinitely. No mechanical operation depends on recovery (INV-8).
  RM.18/RM.19 distinguish prompt cancellation and input release from safe physical resource retirement. A lost
  fence never authorizes freeing in-use memory. RM.22/RM.23 determine pending receipts and invalidate the old epoch.
- **RPI.47 [D]** Recovery rebinds immutable geometry and reads a complete current engine bundle before resuming session
  drawing, following RPI.8 and RPI.9. Program publications are revalidated separately. Old device resources and replies
  cannot resolve through the new instance generation. A committed receipt remains committed even if its display failed
  (R-02, KN5.22). Recovery never retries a commit or adopts program-supplied state.
- **RPI.48 [D]** A chart that fails a stage rule, property or precision check cannot supply a checked display stage,
  following INV-4, LB4 and RPI.69. The failure identifies the chart and check; affected output has error or incomplete
  coverage. Other valid display data can continue. No failure changes the session, protection, evidence for another
  claim or a receipt. Unknown check results remain unknown, not false or passes.

  ```text
  operation admit_chart(chart: Chart, checks: references to required check results)
      -> admitted | not_established(reason) | refused(Error)
    effect:  pure
    cost:    declared validation cost for the definition and check references
    errors:  LG2:type.stage_rule_violation, EV3:evidence.chart_check_failed
  ```

  `not_established` retains an unknown verdict, and a stage may run as `not_established`: safe to run is a separate
  fact from properties established (RPI.74). This operation consumes checks; it does not define the exact checker
  repertoire left open in RPI.34.
- **RPI.49 [D]** View construction, reference evaluation and output use the deterministic budgets of KN7 and chapter 09.
  For a standalone publication whose payload exists at admission, exceeding a fixed plan ceiling returns
  `refused(EV3:resource_limit.display_plan_ceiling)` before acceptance. For a scheduled view already accepted under
  LI.25, final validation returns `failed(original Error)` on that invocation, including the same ceiling code.
  There is no second lane admission and no resurrection of superseded work. Exhausting a declared output allowance
  cuts off the job under LI.16/LI.28 and RPI.67. Neither installs a partial publication or turns incomplete output
  into a negative claim. The host may retain separately identified old output under RPI.13; engine adoption proceeds.
- **RPI.50 [D]** RM.11 to RM.13 supply the draft plan-limit fields, units, count rules and outcomes. This working
  contract uses finite fixed host/profile/backend ceilings and KN7.7 to KN7.10's existing budget contract, with
  canonical output charging under LI.16 and job-to-receipt mapping only under LI.28. KN5.21 and LI.7/LI.25 determine
  whether a validation error is a refusal or failure; RM.35 supplies the paired ceiling vector. Preflight checks
  nodes, plans, instances, maps, source bytes, expanded charts, layout/overlay data and physical adapter reservations
  before buffer allocation or GPU submission. It adds no deterministic budget dimension and does not activate the
  reserved `KN7:budget.display_plan_limit`. Production ceiling values and H-09's resource measurements remain
  unmeasured and must be pinned before advertising support. Logical memory, canonical output and physical CPU/GPU
  bytes remain distinct. A later per-run plan allowance would require LP1's versioned contract and ERR.25 reconciliation.

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

  This is an adapter responsibility, not an operation granted to a program. The returned work stays inside the adapter
  as RM.14's prepared resource set, and its host validates support and limits before preparing it.
- **RPI.55 [D]** RM.14 to RM.26 supply the draft portable signatures, support-report fields, immutable input leases,
  cancellation, atomic replacement, safe retirement and device epochs. These consume KN5.21 to KN5.23's receipt
  definitions, LI.7/LI.25's admission and finalization, and LI.27 to LI.30's result and lookup rules. Source maps consume
  EV1.10's coordinate contract, and output consumes LI.16, without new definitions here. Host-scheduled synchronous
  work and worker
  preparation share one ticket/poll interface off the UI and engine-adoption paths. Candidate handles never cross
  that boundary. The integrator's route-binding annex still supplies an ABI/transport after the route choices; this
  data contract selects neither a candidate nor zero-copy transfer. Required support and evidence remain per row,
  and the pending inputs in RM.33 must be closed before the applicable feature freeze.
- **RPI.56 [D]** LX-3 compares the same plans on Windows and headless Linux at their declared precision, under chapter
  09 (partition section 8.4 and DT-L1 R2-E2). Exact identities, source mappings, subjects and revisions agree exactly;
  approximate coordinates agree under the stated precision rule. Pixel equality, identical driver output or bitwise
  equality of floats across maths libraries is not promised. A plan never changes a mechanical answer to obtain a
  closer picture.

## 9. Handoff and evidence

- **RPI.57 [B]** The handoff enters the renderer feature list before its freeze (owner, 6 November 2026, LQ20), as row
  H-11 of the experiment protocol and the renderer experiment plan. [D] It contains RPI.15's primitive table; revision
  and derived-subject binding; atomic program publication; instance and source-map picking; the report of the preview
  that the newest presented frame shows (ONE.41); status and fault behaviour; precision and resource requirements; and
  the unresolved dependencies in section 12. Support, limits and evidence are recorded per feature. The draft is
  `docs/architecture/1.0/language/renderer-feature-matrix.md`; RM.34 crosswalks the supplied JI.41 rows, with geometry,
  limits and backend evidence still open under RM.33. Sending it claims no acceptance or implementation authority.
- **RPI.58 [B]** The handoff carries the user stage contract and the free-view workload in place of the generated
  projection stage (owner, 6 November 2026, reading of LQ20). [D] It names RPI.68 to RPI.83, the CPU reference, and
  LX-7's checks and measurement requirements. Built-in stages keep their H-06 measurements.
- **RPI.59 [B] Withdrawn.** LQ20's answer removes the no-handoff alternative (owner, 6 November 2026). Its conflict
  rule moved to RPI.17, and ONE.41's report stays open under RPI.66.
- **RPI.66 [O]** The presented-frame report goes to the renderer line with the rest of H-11: LP3's acceptance needs to
  bind the presented preview (ONE.41). RM.24/RM.26 now specify its frame stamp, preview binding, ordering,
  no-preview/unavailable outcomes and recovery invalidation. A submission or Present-call return is insufficient.
  Each candidate's implementation and actual presentation evidence remain open; defining the report supplies neither.
- **RPI.60 [B]** The renderer remains unselected. Its selection gate stays the one in renderer-candidates: all 259,800
  sticker slots at full detail, the most complex turn animation and camera workload, average at least 30 fps and
  p99 frame time at most 33.3 ms, native resolution and no frame generation, on the owner's RTX 4070 Laptop GPU with
  8 GB VRAM and peak use at most about 7 GB. These are decision thresholds from that source, not measurements in this
  chapter. The experiment plan's W3 and W5 rules and correctness readbacks are not relaxed by a user stage or a further
  view.
- **RPI.61 [D]** LX-7 retains PresentMon on the owner's machine at W3: three cold runs of each advertised combination
  of RPI.79, with three cold built-in-chain runs for comparison and the raster-bound pass included (RTE.60; partition
  LX-7, DT-L1 E5). Captures and gate judgement follow the renderer experiment plan. GPU numerical correctness needs
  actual GPU output; the expected exact or interval values and CPU reference fixtures can be prepared headless.
  Headless preparation supplies no GPU or performance result. A view workload gets its ceiling under RPI.82.
- **RPI.62 [D]** Active program views also enter the end-to-end charter E2 test (partition LP3 and DT-L1 E1, E4). A
  renderer frame-rate pass or a short chart timer cannot establish key-to-correct-frame latency. H-06 chart costs,
  LX-7, fault runs and E2 remain unmeasured by this chapter. Earlier bare-probe results establish none of them for a
  new user stage or product workload.
- **RPI.63 [D]** Section 11 lists conformance obligations for chapter 12's phases, following the overview's convention.
  No conformance test in that table exists yet. A test that depends on a waiting decision applies only after that
  decision; its absence is not a pass. Synthetic or source checks never substitute for windows, performance or human evidence.

## 10. Added error codes

- **RPI.67 [D]** Publication consumes KN5.21's terminal outcomes and LI.16/LI.28's output accounting and result mapping.
  A fixed plan ceiling uses `EV3:resource_limit.display_plan_ceiling`: refusal at standalone admission, or failure
  on the already accepted scheduled invocation under RPI.49/RM.20. Exhausting the declared output
  allowance returns the job's `cut_off(KN7:budget.output)` and an accepted publication's `cancelled(budget)`; its
  terminal reserve carries no user output. RM.13 and RM.20 to RM.23 determine these outcomes and the replacement
  races. No error changes a completed publication or committed mechanical receipt. Chapter 13 registers these forms.

- **RPI.64 [D]** These proposed codes use default 5's categories and chapter 01's error convention. Chapter 13 owns
  final spelling. A display failure is separate from a command's mechanical outcome. Missing coverage, a partial level
  projection and an unknown verdict remain typed results, not new error categories.

| Code | Meaning and continuation |
| --- | --- |
| `EV3:input.display_plan_invalid` | Malformed plan, inconsistent subject or geometry binding, invalid instance/source-map reference, or a relation node with no statement reference; standalone refusal or scheduled failure under RM.20, with no installation |
| `EV3:input.display_unsupported` | Unsupported dimension, primitive or projection-chain stage; report the unsupported part without silently changing its meaning |
| `EV3:stale.display_plan` | Publication no longer matches the dependencies of the view it would replace; recompute at the newest revision |
| `EV3:stale.display_pick` | Pick dependency or installed-generation mismatch; discard passive replies quietly and explain explicitly requested stale work |
| `LG2:type.stage_rule_violation` | A stage module breaks a stage rule or bound; nothing reaches a device (RPI.69, RPI.71). It replaces `LG2:type.chart_subset_violation`, withdrawn before first use |
| `EV3:evidence.chart_check_failed` | A declared chart property or precision requirement fails its check; identify the failed obligation and withhold checked status |
| `KN7:budget.display_plan_limit` | Remains reserved under ERR.25; this draft uses fixed plan ceilings and the existing output allowance, not a new declared plan budget |
| `EV3:resource_limit.display_plan_ceiling` | Fixed count/byte ceiling exceeded; refuse standalone admission or fail final validation of an accepted scheduled invocation with the original error; name field, requested size and ceiling, and replace no data (RM.12/RM.20) |
| `KN7:budget.output` | LI.16 canonical output allowance exhausted; LI.28 maps the job cut-off to an accepted publication cancelled with reason budget; install no incomplete output (RM.13) |
| `KN7:resource_limit.host_exhausted` | Physical allocation fails after acceptance; publication fails without changing the installed generation or deterministic charges (RM.12) |
| `EV3:stale.source_link` | After pick bindings pass, a source action names a digest other than the producing source; no retargeting of the range (RM.10) |
| `EV3:input.display_bundle_mismatch` | Metadata and mutable arrays do not form one complete matching engine bundle; request a validated refresh |
| `EV3:busy.renderer_unavailable` | Device loss or recovery prevents drawing or picking; refusal form before acceptance, contextual stop form in an accepted job's error outcome with Receipt.status = failed; completed publications and committed receipts stay unchanged (RM.22) |
| `EV3:input.stage_compile_failed` | The host compiler refused a checked stage; refusal before acceptance, stop after acceptance; the built-in chain stays (RPI.78) |
| `EV3:input.stage_quarantined` | A publication names a stage digest quarantined in this session; refusal before acceptance, stop after acceptance (RPI.77, RM.20) |
| `EV3:resource_limit.stage_work_exceeded` | A frame with a user stage had a raster bound over its allowance; no user-stage fragment work of that frame ran; context names every user-stage digest of the chain, the bound and the allowance; form `status` in RM.41's frame report (RPI.72, RPI.77) |
| `EV3:input.view_id_in_use` | `view.open` names an open view (RPI.83) |
| `EV3:input.view_not_closable` | `view.close` names an unknown view, Global or Local (RPI.83) |

## 11. Conformance tests

| Test | Checks | Class |
| --- | --- | --- |
| CT-RPI-01 | Boundary enumeration finds no language renderer calls, backend handles or callbacks; switching a mock backend leaves plan meaning unchanged (RPI.1, RPI.54) | source |
| CT-RPI-02 | Wrong model or geometry hash, unsupported dimension and unknown stage prevent installation; RM.20 distinguishes standalone refusal from scheduled failure (RPI.4, RPI.8, RPI.17) | source |
| CT-RPI-03 | Delayed and reordered engine arrays never form a mixed bundle; a stalled program view does not block engine adoption (RPI.2, RPI.9) | synthetic |
| CT-RPI-04 | A publication with a missing source map installs none of its nodes, plans or instances; an older completion cannot replace a newer one (RPI.10, RPI.12) | source |
| CT-RPI-05 | A derived plan without derivation or base fails validation under RM.20; a valid one keeps its derived subject and never becomes session state (RPI.7) | source |
| CT-RPI-06 | Counters show no program work or resource rebuild for a hidden or unchanged view; becoming visible validates dependencies and skips to the newest revision (RPI.11, RPI.12, RPI.38) | source |
| CT-RPI-07 | Engine adoption makes mismatched program output stale; its overlays cannot attach to new geometry, and all D2 statuses have typed data (RPI.13) | synthetic |
| CT-RPI-08 | Headless fixtures produce and validate every admitted primitive row, with unsupported and missing-data cases explicit (RPI.15, RPI.19) | source |
| CT-RPI-09 | Every instance in duplicate charts and layouts resolves to the declared canonical object and producing source version (RPI.20 to RPI.23) | source |
| CT-RPI-10 | A many-to-one chart with no inverse is picked by instance ID; a planted inverse that throws is never called (RPI.22, RPI.25) | synthetic |
| CT-RPI-11 | Reused instance IDs, reopened views, camera changes and replaced maps discard delayed picks; an unrelated layout edit preserves an unchanged query (RPI.24, RPI.25) | synthetic |
| CT-RPI-12 | Explicitly requested stale resolution carries a visible-explanation reason; passive stale picking is quiet; off-lattice partial answers stay distinct (RPI.23, RPI.25) | source |
| CT-RPI-13 | The native ID pass and drawing pass share camera, viewport and clipping; non-interactive reference objects cannot be selected (RPI.24) | windows |
| CT-RPI-14 | At exact camera poses an exact region test or a theorem decides declared chart properties, and a vertex test only refutes them; intervening float poses cannot create exact verdicts (RPI.29, RPI.30, RPI.34) | synthetic |
| CT-RPI-15 | The stage checker refuses recursion, unbounded loops, allocation, big integers, pointers, writes, atomics, barriers, readback and effects before compilation (RPI.69, RPI.70) | source |
| CT-RPI-16 | CPU reference fixtures test float32 evaluation and stated precision against exact or interval values outside the declared singular neighbourhood (RPI.34 to RPI.36, RPI.74) | synthetic |
| CT-RPI-17 | GPU readbacks agree with exact or interval expectations and the CPU reference on the admitted domain, using the recorded exclusion and error rule (RPI.35) | windows |
| CT-RPI-18 | Singular and domain-boundary fixtures produce missing coverage, no non-finite draw or pick coordinates, and unchanged exact subjects (RPI.36, RPI.37) | synthetic |
| CT-RPI-19 | One reproducibility key on Windows and headless Linux gives equal exact plan fields and approximate values within declared precision, including maths-library-sensitive cases (RPI.56, LX-3) | source |
| CT-RPI-20 | Dependency and interface inspection finds Windows APIs only behind platform interfaces and one portable HLSL source for each stage (RPI.51 to RPI.55) | source |
| CT-RPI-21 | Device loss injected during input, transfer and pick readback releases publication/input state, recovers or reports, and preserves session hashes and receipts (RPI.46, RPI.47, R-17) | windows |
| CT-RPI-22 | One-over-limit plans install nothing; failed chart checks affect only their display output; neither failure changes the session or certifies a negative (RPI.48 to RPI.50) | source |
| CT-RPI-23 | Profile fixtures show Global and Local stay linked and further views open under every profile; built-in output keeps the interim boundary under either chart evaluator (RPI.42, RPI.45, RPI.81) | source |
| CT-RPI-24 | First frames using a jumbling revision read back the bound pose indices, pose table and lattice arrays and compare them with that revision's engine data (RPI.9; render contract section 7) | windows |
| CT-RPI-25 | LX-7's three cold W3 runs per advertised combination use PresentMon and the unrelaxed renderer gate, with built-in-chain comparisons and the raster bound included (RPI.60, RPI.61, RPI.79) | performance |
| CT-RPI-26 | End-to-end charter E2 holds with program views active, and R-19 gate captures confirm that hidden and unchanged views add no rebuilds (RPI.11, RPI.62) | performance |
| CT-RPI-27 | A word of EVD.32 reaches a display only through `word()` on a key with its phrase; a relation node with empty `claims` gives `EV3:input.display_plan_invalid` under RM.20; state data needs no reference; text a program writes stays the program's text (RPI.5, RPI.65) | source |
| CT-RPI-28 | The same source and compiler version give the same HLSL bytes; the stage digest and compile record join the plan's identity; a host compile failure gives `EV3:input.stage_compile_failed` and the built-in chain stays (RPI.78) | source |
| CT-RPI-29 | A stage module has only `pure`; no stage result reaches a program; stage inputs pass the `display` sink (RPI.68, RPI.73) | source |
| CT-RPI-30 | RM.40's raster vectors: spread versus overlapping plans with equal vertex counts, a plan failing only when samples or the ID pass multiply U, a near-plane crossing and a line-only plan; frames over the bound run no user-stage fragment work (RPI.71, RPI.72; RM.40) | synthetic |
| CT-RPI-31 | On an actual device a stopped frame runs no user-stage rasterization or fragment work, and audit counts of a frame that ran are at most U (RPI.72, RPI.80) | windows |
| CT-RPI-32 | A fragment stage has no discard, depth, stencil or coverage output; host statuses composite after user stages; a preview shown through a user stage has no presented-preview identity and binds no commit token (RPI.75, RPI.76) | source |
| CT-RPI-33 | With a user vertex stage the ID pass and the drawing pass agree on positions and coverage, and undefined coordinates enter neither (RPI.37, RPI.75) | windows |
| CT-RPI-34 | An injected device fault in a frame with a user stage quarantines every user-stage digest of the chain, the view falls back with visible status, a later publication naming any of them gives `EV3:input.stage_quarantined`, and no TDR or security setting changes (RPI.77) | windows |
| CT-RPI-35 | A candidate report with no run-time compile path or no way to skip a stopped frame fails the user-stage row, and the conflict returns to the owner (RPI.80, RPI.17) | source |
| CT-RPI-36 | `view.open` and `view.close` run only inside a solver-invoked command; a revision-started recomputation holds no permission; Global and Local cannot be closed; both EV3 codes (RPI.81, RPI.83) | source |
| CT-RPI-37 | RM.40's view vectors: hidden views add no vertex evaluations, hidden allocations stay charged until safe release, opening past a ceiling refuses with `EV3:resource_limit.display_plan_ceiling`, no unbounded queue (RPI.82, RM.39) | synthetic |
| CT-RPI-38 | An unset ceiling advertises no support; a failed workload lowers the ceiling and never moves the gate (RPI.79, RPI.82) | source |

## 12. Open items

The owner answered LQ1 to LQ21 on 6 November 2026. LQ8, LQ9, LQ13 and LQ21 wait for their evidence, and LQ2's option set
waits for the joint setting.

| Item | What remains open and where it is used |
| --- | --- |
| LQ2, LQ21 | The joint boundary policy, checked display sinks and form of flow control, owned by chapter 07. No new permission follows from display or picking (RPI.14, RPI.26, RPI.45; DT-L1 R2-E3). |
| LQ8, LQ9 | Engine language and runtime placement do not change this boundary. Each chosen placement must preserve the independent adoption and fault rules (RPI.1, RPI.2, RPI.46 to RPI.54). |
| LQ13 | The surface syntax of stage modules follows the choice made after LX-8 (RPI.68). |
| Stage numerics and weights | Error rules, the tiers they enable and RM.38's weight table (RPI.36 [O], RPI.70, RPI.71). |
| Stage and view ceilings | Unset until a passing gate run exists (RPI.79, RPI.82). |
| Run-time compiler | DXC is not on the toolchain allowlist, so shipping one is an "Ask the owner" item (RPI.80). |
| EN3 and geometry additions | RM.34 crosswalks JI.41 into J01 to J13, including release checks, nu/correction observations and picks. EN2 and the renderer line still owe certified exposed-face geometry, concrete layout bindings, measured sizes and backend evidence (RPI.18 [O], RM.4). Geometry changes still need the immutable-model process. |
| EN4 and LP1 | The integrator verifies KN5.21 to KN5.23 receipts, EV1.10 source ranges and LI.16/LI.28 accounting against the assembled H-08 table and chapter 13 (RPI.10, RPI.21, RPI.67). Editable parameters and stored forms remain in chapters 04 and 05. |
| LB4, chapter 09 and LX-7 | Error metrics, singular neighbourhoods and crossing-primitive handling need one agreed precision contract before shader acceptance (RPI.36 [O]). Exact checks and shader agreement remain separate (RPI.30, RPI.35). |
| Renderer interface and H-09 | RM.11 to RM.26 supply draft count rules, signatures, support reports and lifetimes (RPI.50, RPI.55). Finite production ceilings, resource measurements, route binding and candidate implementations/evidence remain open. RM.26's presented-frame report remains required (RPI.66). |
| Picking policy D-5 | Multi-layer disambiguation awaits the owner when design work resumes (RPI.27 [O]). Identity resolution and stale-reply handling do not depend on its answer. |
| Renderer selection and evidence | No candidate is selected. H-06 chart costs, LX-7, product fault tests and E2 with program views still need evidence for the matching candidate (RPI.60 to RPI.63). |
| DT-L1 obligations | Round 3 cleared them (overview section 8). Dependencies on R2-E1, R2-E2, R2-E3 and R2-E4 are named at RPI.12, RPI.30, RPI.35, RPI.38, RPI.45 and RPI.56, and RPI.30 carries round 3's correction. |
| EV3.12 | Settled as a default: the plan states lineage `imported` with the import's digest and uses an explicit comparison base with no claim that it produced the value (ONE.21). The reading of partition EV3 is the integrator's. |
| Source differences to reconcile | The old stack and process choices in V02 and V11 and the dated schedule/design work in the experiment documents are superseded as the briefing records. Their surviving revision and gate contracts are used here (RPI.9, RPI.24, RPI.51 to RPI.63). The render header lacks KN0.1's menu-order field; the language envelope retains the full IdentityBlock without changing that format (RPI.6, RPI.8). Partition 4.1's derived-state base revision is the value's lineage, and the plan's base is the display context; ONE.21 separates them, and neither is a session revision on the derived value (RPI.7). |

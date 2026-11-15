# Chapter 11: implementation routes and spikes

Status: **frozen with the LP1 candidate** (owner sign-off, 15 November 2026;
[decision](../../../../wiki/decisions/owner-decisions-2026-11-15-freeze.md)). Conventions are in
[the overview](00-overview.md), section 3, and its status says what the freeze covers.

This chapter gives the obligations of each implementation route and the evidence needed to choose a route, an engine
language and a runtime placement. It supplies experiment cards and decision rules. LX-1 to LX-9 ran as far as the
owner's allowance permits; RTE.82 records their evidence and limits. LX-5 and LX-6 ran decisively on 14 November 2026,
and the owner chose LQ8, LQ9 and LQ21 that day: the engine and the interpreter's host in Rust (RTE.17), route RT1 with
the runtime in a process of its own (RTE.21), and capability separation (RTE.24). The decisive LX-1 to LX-4 checks
are LP3 acceptance checks. LQ13 still waits for LX-8. This chapter starts no run. No ADR is accepted and nothing is
frozen.

Sources: [the partition](../ecosystem-partition.md), especially INV-1 to INV-12, sections 8.4, 11, 12, 14 and defaults
1 to 20; [DT-L1](../../../../progress/1.0/design-table/topics/language-ecosystem.md), E1, E3, E4, R2-E3 and Round 3,
engineer advisor, items 3 and 4; [the kernel](01-kernel.md); [the language core](02-language-core.md);
[libraries](03-libraries.md), LB5.13; [environment](04-environment.md), EV1 to EV3;
[persistence](05-persistence.md); [one state](08-one-state.md); [determinism](09-determinism.md);
[renderer and platform independence](10-renderer-platform.md);
[engine language question](../../../../wiki/questions/engine-language-rust.md);
[stage 2 experiment protocol](../../../../progress/1.0/stage-2-experiment-protocol.md), sections 1 and 2;
[charter](../../../../progress/1.0/charter-2.0-draft.md), E2, E3 and the stage 2.5 exit;
[renderer selection gate](../../../../wiki/decisions/renderer-candidates.md);
[renderer experiment plan](../../../../progress/1.0/renderer-experiment-plan.md);
[B4-12 measurement decisions](../../../../wiki/decisions/b4-12-measurement-decisions.md); and
[oracle cases](../../../../../tools/oracle/make_cases.py) and
[comparator](../../../../../tools/oracle/compare.py), for their names and cases only.

## 1. What every route delivers

- **RTE.1 [B]** The owner requested ecosystem and architecture before implementation, with subscription quota only
  (DT-L1, Owner input; owner, 3 November 2026, as recorded in the briefing). This text chooses no route, engine language,
  placement, syntax or form of flow control; it records the owner's choices of 14 November 2026 (RTE.17, RTE.21,
  RTE.24). The owner allowed LX-1 to LX-9 as experiment code (owner,
  6 November 2026, LQ18). That allowance supplies no permission for an API-team run, new cost or tool installation.
- **RTE.2 [D]** Every route is a client of the same chapter 01 kernel interface and chapter 08 state lifecycles
  (partition sections 9 and 12). It preserves model and level identity, literal chronological execution, exact
  decisions, complete collateral effects and independent checking. It has no mechanics of its own. No route receives
  a commit row, a shell token or a path to adopt program-supplied state (ONE.3, ONE.44, ONE.67).
- **RTE.3 [D]** A route change preserves the language-neutral data core and the meaning of saved work, following
  partition sections 12 and 13 and PS2's recorded migrations. It changes neither a saved literal word nor the meaning
  of a stored evidence reference. RT1 to RT3 retain LG4.1's versioned typed IR; an executor replacement below that
  contract preserves its semantics and budget stopping points. RT4 has the replacement obligation in RTE.6. A new
  execution contract has a new version and compatibility treatment; it cannot claim an old run's key silently.
- **RTE.4 [D]** The following table is an obligation inventory, from partition LG2 to LG4 and section 12. Each cell
  uses one of three terms. **By construction** means the route's definition includes the component, with its
  implementation and conformance still to prove. **Must be built** includes an adapter or restriction around a
  borrowed component. **Needs a modified host** identifies the executor ownership required by BM2; a library wrapper
  around ordinary values and branches does not supply it (DT-L1 R2-E3). No cell is a result or a cost estimate. LQ9
  selected RT1 and LQ21 capability separation (RTE.21, RTE.24): the RT1 column and the capability-separation row
  apply, and the other columns and the label rows record the alternatives.

| Requirement | Contract | RT1 | RT2 | RT3 | RT4 |
| --- | --- | --- | --- | --- | --- |
| Checker for types, effects and exactness | LG2.1 to LG2.9, LG3.2 | by construction | must be built | by construction | must be built |
| Numeric tower, unbounded integers and exact operations | KN3.1 to KN3.6, LG1.6 | must be built | must be built | must be built | must be built |
| Deterministic evaluation and canonical ordering | LG4.3, DET.5 to DET.17 | by construction | must be built | must be built | must be built |
| Step metering and cancellation points | KN7.6 to KN7.8, LG4.7 to LG4.9, DET.20 to DET.23 | by construction | must be built | must be built | must be built |
| Logical memory and recursion limits | KN7.9, DET.24, DET.28 | must be built | must be built | must be built | must be built |
| No ambient authority | LG3.8, DET.35 to DET.42 | must be built | must be built | must be built | must be built |
| Typed IR as the versioned contract, or RT4's explicit replacement | LG4.1, LG4.14; RTE.6 | by construction | must be built | by construction | must be built |
| Executed-source maps for debugging | LG4.12, EV2.1, EV2.12 | must be built | must be built | must be built | must be built |
| Output and pick linkage to canonical IDs and source | EV3.1, EV3.5, RPI.20 to RPI.25 | must be built | must be built | must be built | must be built |
| Editor services for domain types, effects and evidence | EV1.1 to EV1.9 | must be built | must be built | must be built | must be built |
| Zero-copy access to immutable snapshots and kernels where the binding permits it | Partition section 12; KN2.2, KN2.3; RTE.7 | must be built | must be built | must be built | must be built |
| Streaming of literal and compressed long words | KN2.10, LG4.6, LB5.13 | must be built | must be built | must be built | must be built |
| Static flow labels, if selected | LG3.12, BM2 | by construction | must be built | by construction | must be built |
| Run-time flow labels, if selected | LG3.12, BM2 | by construction | needs a modified host | by construction | needs a modified host |
| Capability separation (selected, RTE.24) | LG3.13, BM2 | must be built | must be built | must be built | must be built |

- **RTE.5 [D]** Host tooling, a numeric library, VM fuel or a sandbox supplies only the part demonstrated against this
  inventory (LG4.14, DET.21, partition section 12). A candidate's report identifies the implementation of each row,
  the admitted source subset and the remaining gaps. In particular, a host editor does not establish EV1's domain
  diagnostics, and host integers do not establish the whole KN3 tower. The independent kernel gates remain the same.
- **RTE.6 [D]** Conditional on LQ9 selecting RT4, admission requires either lowering the approved host subset into
  the common validated typed IR, including LB6.2's generated decomposition before lowering to an executor, or a
  checked host-program contract satisfying RTE.80 and RTE.81. The latter owes compatible types, exactness, effects,
  accounting, stops and source linkage under LG4.14, and a generated decomposition for every tool's PrincipleRecord.
  Host annotations, metering and source locations without that decomposition are insufficient. RT4 admission remains
  unproved until its validator and conformance evidence exist. LQ9 selected RT1, an IR route (RTE.21), so the
  non-IR replacement clause is inapplicable and records the unselected alternative; the ordinary IR decomposition
  obligation remains.

- **RTE.80 [B]** LQ9 selected RT1 (owner, 14 November 2026, LQ9; RTE.21), so no non-IR RT4 candidate is admitted.
  This clause records the unselected alternative and binds nothing. A non-IR RT4 candidate supplies a versioned host
  decomposition contract. Its validator consumes
  exactly the admitted source bytes, generated bodies if any, locked dependency closure, host/subset and adapter
  versions and executable artifact digest. It generates a graph of entry points, transitive library calls, kernel
  leaves, effects and required capabilities. Every node and edge names its source digest and EV1.10 range or an
  explicit generated origin; no edge silently borrows the current editor draft. A higher-order or generic call has
  every possible target in the admitted subset, with over-approximations marked `may call`. Targets outside that
  checked set, unresolved imports, dynamic code and unmodelled native calls refuse admission.

  The validator derives calls and effects from the body, rather than trusting an author's graph. It checks that
  each declared edge set contains the derived targets and that effects include the transitive union, including
  recursive components. An opaque kernel leaf names its specification and oracle gate; it is not a way to hide
  tool code. It binds the resulting graph digest to the approval and verifies the same artifact at load. Changed
  source, dependencies, call resolution or effects invalidate approval and mark LB6.2's hand-written mapping stale.
  Stored source and build digests follow chapter 05. Generation supplies structure only, never a lemma or a human
  understanding result. RT4 with common-IR lowering uses LB6.2 directly instead of this replacement.

- **RTE.81 [D]** Under RT1 (RTE.21) only this gate's IR arm applies; its non-IR RT4 arm records the unselected
  alternative. Admission evidence includes a tool `T` calling a function parameter whose sole admitted target
  is helper `H`; `H` reads a snapshot and invokes a kernel leaf `K`. The generated graph contains `T -> H -> K`,
  marking the indirect edge `may call`, and both T and H carry the snapshot effect. Removing H, K or that effect from
  the submitted decomposition yields LB6.6's `LB6:evidence.decomposition_mismatch`. Replacing H's bytes without a new checked
  graph yields the same refusal. A complete matching graph permits only this admission gate to pass; run grants and
  boundary checks still apply. A host unable to enumerate the possible target set is refused, not admitted with an
  unknown edge. CT-LB6-05 is the required mutation gate. Under RT1, RT2 or RT3 its non-IR RT4 arm returns
  `not_applicable(route_uses_ir)` and the IR arm still checks transitive calls. No candidate has run this gate.
- **RTE.7 [O]** The sources motivate a common host by zero-copy access but give no binding, ABI or buffer-lifetime
  protocol. LQ8 and LQ9 are answered (RTE.17, RTE.21): engine and runtime share the Rust language but not a process,
  so no language boundary is needed, and the runtime reaches snapshots through transfer or a separately specified
  read-only sharing mechanism. LP1's interfaces specify which. Either preserves snapshot immutability, logical memory
  charges and kernel-only mutation. LX-1 and LX-2 report actual bytes and transfers; nothing is presumed zero-copy.

## 2. RT1: own language

- **RTE.8 [D]** LQ9 selected RT1 (RTE.21). RT1 owns the parser, checker, typed IR and interpreter, normally in the
  engine language (partition section 12), which is Rust (RTE.17). It supplies direct control over domain types,
  exactness, effects, determinism, budget checks and source diagnostics. Under capability separation, the selected form
  (RTE.24), it enforces LG3.13; under flow labels, the recorded alternative, it would also own the checker and executor
  values. It adds the largest implementation effort, including the editor and debugger services. Its speed remains
  unmeasured. LG4.4 starts with an interpreter and introduces no own VM, JIT or machine-code backend before
  measurement supports one.
- **RTE.9 [D]** RT1's front end is the difficult choice to reverse: source, diagnostics, module contracts and source
  associations become saved-work dependencies (partition section 12, DT-L1 E4). Choosing a borrowed surface syntax
  under LQ13 still leaves this route an own front end and interpreter. LX-1 and LX-4 bear directly on its cost and
  authoring path; LX-2 and LX-3 test placement and enforcement, and LX-5 and LX-6 inform its engine host. An experiment
  slice supplies effort evidence; the partition supplies no effort figure.

## 3. RT2: embedded runtime

LQ9 selected RT1 (RTE.21). Sections 3 to 5 record the alternatives and bind nothing.

- **RTE.10 [D]** RT2 embeds an existing scripting runtime with a domain library (partition section 12). Its candidates
  are **Luau, Lua 5.4, a JavaScript engine, CPython or a restricted subset, Rhai, a WebAssembly VM**. The partition
  identifies existing VMs, docs and editors as possible reuse and a faster start as an expectation. Domain types,
  effects, exactness and evidence discipline still need the checker and adapter of LG4.14. Host numeric semantics
  cannot introduce float decisions or integer overflow. No candidate here is selected or demonstrated compliant.
- **RTE.11 [D]** RTE.82 records stand-in results for some named candidates; none is admitted as a product host, and
  LQ9 selected RT1 (RTE.21). The questions below bind only a later route change. For each named candidate, LX-2,
  LX-3 and LX-5 must answer: which host facilities can the approved subset reach, can
  dynamic code be refused with approval bound to executed bytes, can all limits and interrupts be enforced, do exact
  results repeat, and what are the dependency and modification licence obligations? The partition calls CPython's
  sandboxing and determinism hard; this is a risk to test, not a demonstrated escape or impossibility. No licence,
  speed, sandbox or determinism property is supplied from memory.
- **RTE.12 [D]** A borrowed VM does not remove the front-end commitment (partition section 12, DT-L1 E4). The host
  syntax or approved subset, its checker and its mapping to the domain IR constrain saved tools. Run-time labels need
  a modified VM; static labels need the separate subset checker; capability separation keeps its coarse restriction
  without per-value labels (DT-L1 R2-E3, LG3.12, LG3.13). LX-2, LX-3 and LX-5 are the partition's direct evidence;
  LX-1 and LX-4 still apply to the common route comparison and editor path. Existing host tooling proves none of them.

## 4. RT3: hybrid

- **RTE.13 [D]** RT3 owns syntax, checker and typed IR and lowers them to an existing VM, such as WebAssembly with
  fuel metering (partition section 12). It keeps the domain discipline in its front end and reuses an executor. It
  adds a compiler boundary, target memory and big-integer integration, and source maps through that boundary. Static
  labels belong to the front end; run-time labels can be compiled as explicit fields (DT-L1 R2-E3). Target fuel counts
  as the logical budget only after DET.21's mapping is verified.
- **RTE.14 [D]** RT3's own front end remains the hard choice to reverse; the lower executor can change while LG4.1's
  contract holds (partition section 12). The compiler must retain literal order, effects, source linkage and charges
  across that change (LG4.6 to LG4.14). LX-1 to LX-4 are its direct route evidence. LX-5 tests its arithmetic
  integration and target limits; LX-6 informs its host. Two layers give no measured speed or effort advantage here.

## 5. RT4: host-language library

- **RTE.15 [D]** RT4 supplies the domain as a typed library in the engine language or Python (partition section 12).
  It offers host tooling and the partition's anticipated fastest start. Its costs are learning the host, dependence
  on the engine choice, a separate checker for transitive effects and exactness, and sandbox enforcement for admitted
  user code. Static flow labels require an approved-subset checker; run-time labels require a modified host runtime;
  capability separation is available without that modification (DT-L1 R2-E3, LG4.14). Library types alone discharge
  none of the missing checks.
- **RTE.16 [D]** RT4 makes the admitted host syntax and semantics the front-end commitment. Changing them must preserve
  the explicit saved-program contract of RTE.6, with recorded migrations where meaning changes (LG4.14, PS2). LX-3 is
  the partition's direct evidence for this route; LX-1, LX-2 and LX-4 still test the common cost, placement and editor
  requirements, and LX-5 its exact arithmetic. A host-language library receives no additional kernel or commit rights.

## 6. LQ8: engine language and interpreter host

- **RTE.17 [B]** LQ8 is answered: the engine, the session store, the command layer and the interpreter's host are
  written in Rust (owner, 14 November 2026, LQ8). The owner first chose to wait for LX-5 and LX-6 (owner, 6 November
  2026, LQ8) and chose Rust on their decisive runs of 14 November 2026 (RTE.82). The table keeps the recorded options;
  the third binds nothing. The portability boundary is invariant: engine, session store, command layer and view
  model remain free of Windows-only code, with runtime, libraries and checkers runnable headless on Linux (INV-9,
  RPI.51 to RPI.53).

| LQ8 option | What changes | What remains to prove |
| --- | --- | --- |
| Decide after LX-5 and LX-6 (chosen on 6 November 2026; it led to Rust) | Compare exact arithmetic dependencies and the engine hot path before choosing the host; no language is presumed | Oracle equality, arithmetic correctness, dependency terms and owner-machine timings for the selected candidate |
| Rust (selected on 14 November 2026) | Adopt the wiki question's proposed portable engine, store and command layer, with a narrow boundary to the GUI | Its expected benefits and costs remain proposals; correctness and measured performance still need their checks |
| Keep the exact Python and NumPy reference as the 1.0 engine | Wrap the reference behind the new interfaces rather than presuming a port | The new interface, lifecycle, limits and end-to-end bars still need verification; 0.4 timings do not prove them |

- **RTE.18 [D]** LX-5 and LX-6 fed the engine decision before the route was selected (RTE.82). On the same inputs in
  the pinned engine environment, the Rust port is 2.3 to 7.2 times faster than NumPy at applying a generator, applying
  a long word and composing macros, with equal output digests. NumPy is 15 times faster at the net effect, where it
  uses a warmed star cache the port lacks, so that row is no like-for-like comparison; the port adds the cache before
  any net-effect claim. LP2 chooses the big-integer implementation on LX-5's rows: `num-bigint` was exact and about
  3.4 times faster in total than the experiment's own limbs, with gcd and sign hot spots on random 22,500-bit inputs,
  and the own limbs are no audited library. A product dependency needs the owner's approval (RTE.70). The wiki's Rust
  proposal also expects memory safety, type-level invariants and a PyO3 transition path, and records costs in the
  extra GUI boundary, compilation and reading burden; those remain its expectations. Whole-engine correctness and
  product performance still need their checks.
- **RTE.19 [D]** The interpreter's host follows the engine language (partition section 12): both are Rust (RTE.17).
  Because the runtime runs in a process of its own (RTE.21), snapshot and kernel access crosses a process boundary,
  not a language boundary. The binding preserves the same identity, immutable lifetimes, exact values, declared
  charges and typed results through RTE.7's boundary, and the report separates transfer overhead from interpreter and
  kernel work. Neither choice makes the runtime depend on the renderer's language. The GUI language follows renderer
  selection in the wiki proposal; engine and runtime portability remain separate obligations.
- **RTE.20 [D]** The wiki question proposes comparison on "the same headless machine". DT-L1 E4 and Round 3 item 4,
  LX-6, narrow its use: oracle equality is source evidence, cloud or headless timing is a lead only, and the decisive
  speed comparison uses the same inputs against NumPy in the pinned engine environment on the owner's machine, by the
  B4-12 method. There is no speed threshold. The decisive run took place on 14 November 2026, and both results went
  to LQ8 with LX-5 (RTE.82).

## 7. LQ9: runtime placement

- **RTE.21 [B]** LQ9 is answered: route RT1, an own interpreter over the typed IR, with the runtime in a process of
  its own (owner, 14 November 2026, LQ9). The engine-process and shell-process placements are not admitted, and RT2,
  RT3 and RT4 are not built; the table below and the route rows elsewhere keep them as the record of the
  alternatives, and they bind nothing. The owner chose on RTE.82's leads, together with LQ21, instead of waiting for
  decisive LX-1, LX-2 and LX-4 runs, which need the Rust host, experiment versions of the LP3 tools and a renderer
  candidate. Those runs are LP3 acceptance checks with their bars unchanged (RTE.38 to RTE.53); if one fails, LQ9
  reopens with that evidence. The placement must satisfy KN7.12, KN7.13 and every rule of chapter 08. LX-2's
  decisive run judges its fault containment and responsiveness; a process boundary alone is no proof.

| Placement | What it gives and costs | What KN7.13 and chapter 08 require it to demonstrate |
| --- | --- | --- |
| Isolate inside the engine process | Direct access can avoid IPC; user execution shares the engine's physical resource and failure domain | Limits, no-kernel-call loops and the forced runtime kill cannot end the engine session or change its journal; snapshots cannot hold commit locks |
| Process of its own (selected) | A separate runtime failure domain; extra startup, transfer and round trips at LX-1's call mix | Runtime death ends jobs with typed outcomes, while the engine, store, pending engine preview and shell continue; no second process opens session files |
| Shell process | Can share the authoring host; user execution shares the window and renderer's crash and memory domain | Execution stays off the UI thread and kernel-key path; faults and forced kill leave the session and shell usable; admitted only if LX-2's fault run passes |

- **RTE.22 [D]** A runtime fault ends its jobs, never the session (KN7.13, ONE.70, DET.43). A previously durable
  command keeps its receipt; the fault cannot change it to a failed commit. An engine-owned pending preview stays
  available after runtime death, and runtime handles end with the run. Program output becomes stale under ONE.11 and
  ONE.12; the adopted engine bundle continues without waiting for it (ONE.64). No placement permits old output over a
  new state, a raw state to commit, direct session-file access or token access.
- **RTE.23 [D]** The owner decided placement and route together with LQ21 on RTE.82's leads, before LP1's freeze
  gates (owner, 14 November 2026, LQ9 and LQ21). The choice does not waive KN7.13 for the selected placement. A
  placement that cannot survive the forced runtime kill fails the proposed LX-2 rule, even if ordinary cooperative
  cancellation works. LX-2's first pass met conditions 1 and 2 for seven injected faults in the separate process on
  its stand-in candidate; that is a lead, not a pass (RTE.82).

## 8. Flow control across routes

- **RTE.24 [B]** LQ21 is answered: capability separation (owner, 14 November 2026, LQ21). A program either reads
  state or prepares previews, apart from the fixed declassifiers that LQ2's joint setting enables; a tool that needs
  both uses such a declassifier or is split into a reading tool and a previewing tool. The owner first chose to wait
  for LX-1's cost leads and LX-3's refusal counts, together with LQ9 (owner, 6 November 2026, LQ21). On LX-3's RT1
  stand-in, capability separation refused all 20 must-refuse cases with the expected code and falsely refused 2 of
  the 35 allowed tools, A25 and A30, which read state and prepare previews from independent manual or fixed setup
  data. Both label forms refused 16 of 20 with no false refusal and left MR13, MR14, MR19 and MR20 unrefused
  (RTE.82). LX-3's remaining checks are LP3 acceptance checks: admitted-executor and platform coverage, the
  execution-contract mapping and product sandbox admission. Chapter 07 owns the rule, policy, provenance and
  declassifiers. This table records the implementation consequences in partition section 12 and LG3.12 to LG3.14
  (DT-L1 R2-E3). Only the RT1 row's capability-separation cell binds; the other cells record the alternatives.

| Route | Static labels | Run-time labels in addition to static checks | Capability separation |
| --- | --- | --- | --- |
| RT1 | Own checker over exactly the source that runs | Own executor values and branch context | Enforce the profile's exclusive read-or-preview capabilities |
| RT2 | Add a checker over an approved host subset; refuse dynamic code and bind approval to executed bytes | Modify the borrowed VM; ordinary native branches are invisible to a library wrapper | Same exclusive capabilities; no per-value VM labels required |
| RT3 | Own front end retains label analysis through lowering | Compile labels as explicit fields and preserve branch dependence | Enforce the same capabilities through front end and VM binding |
| RT4 | Add an approved-host-subset checker with the same byte binding and dynamic-code refusal | Modify the host runtime | Same exclusive capabilities, including indirect calls and persisted inputs |

- **RTE.25 [D]** Static-only labels trade more conservative refusals for no run-time label propagation cost; adding
  run-time labels can reduce those refusals at a cost per step (BM2, LG3.12). Capability separation works in every
  route, but a harmless own tool cannot both read state-derived data and prepare previews, apart from fixed enabled
  declassifiers, and a previewing program cannot read preview outcomes (LG3.13). Flow labels record dependencies more
  finely; separation records only whether the program read state. Neither proves authorship or understanding or
  closes termination, preview-emission or manual-retyping channels (DT-L1 R2-E3).
- **RTE.26 [D]** LQ21 precedes LP1's freeze gates and LP2's checker and IR (owner, 6 November 2026, LQ21); the
  owner answered it on 14 November 2026, before both. Adding the checks afterwards would refuse saved tools that had
  been allowed to run (DT-L1 R2-E3). Stored words, macros and workspace records keep BM2 provenance under capability
  separation as under any form (KN4.4; PS1.32 to PS1.34). The choice changes checking and execution, not the
  kernel's user-action gate or independent evidence checking.
- **RTE.27 [O]** LQ2's joint setting decides which author classes and display sinks the flow rule covers and which
  fixed declassifiers it enables. LQ21 chose the mechanism (RTE.24), not those permissions. Until that setting,
  partition section 11's interim reading and LG3.16 apply: built-in tools keep the 2 October boundary and 17 October
  repair-list exception, research output never becomes a solving-session preview, and solver-tool solving previews
  remain disabled. These are the conditions for constructing LX-3's allowed-tool and must-refuse lists; an experiment
  does not answer LQ2.

## 9. Proof-obligation export to Lean

- **RTE.28 [D]** Proof obligations export to Lean in 1.0 (owner, 6 November 2026, LQ15 and decisions 4 and 9).
  LB5.18's `research.obligation.export` writes Lean 4 source through a granted file capability, using a restricted,
  versioned vocabulary and the `c600-proof-obligation-draft` format (PS1.45). The translation prelude lives in the
  repository, outside the shipped software. Lean is neither shipped nor a runtime dependency and is not on the
  allowlist; a project check that runs it needs the owner. There is no proof-record import row, proof-record format
  or return path in 1.0, and no new check mode. Chapter 06 owns the meaning of a formal-proof check: `formal proof`
  remains in EVD.3 with no writer in 1.0 (EVD.64). A later edition names a certificate the kernel can check. Export
  promotes no claim and changes no commit gate; a failed Lean run is no refutation and enters no record.

## 10. Order of decisions and spikes

- **RTE.29 [B]** The permitted spikes are experiment code under `work/experiments/`
  and never product code. They may start in LP0 and supply decisions inside LP1 before its freeze gates. This is the
  acyclic order of partition sections 12 and 14; a listed independent start needs its own frozen experiment inputs.
  LX-2 and LX-4 need a renderer candidate that presents frames for the renderer gate and E2 in the same run. Under
  this order LQ9 waited for a renderer candidate; the owner chose it on RTE.82's leads instead and made the decisive
  LX-1, LX-2 and LX-4 runs LP3 acceptance checks (RTE.21). LX-5 and LX-6 ran decisively in the owner's window of
  14 November 2026. LX-2 and LX-4 use the existing renderer candidates first and run again if the renderer changes
  (owner, 6 November 2026, LQ8, LQ9, LQ18, LQ21 and decision 15). RTE.37 prevents relabelling earlier results for a
  changed candidate.

| Work or decision | Waits for | Feeds or constrains |
| --- | --- | --- |
| Owner's LQ18 allowance and each complete experiment card | Allowance granted; each run retains its frozen card, scope and time box | Permission for the named experiments only |
| LX-5 and LX-6 | Their allowed cards and frozen arithmetic/oracle fixtures; decisive runs on 14 November 2026 | LQ8, answered (RTE.17); LP2's big-integer choice |
| LX-3 | Its frozen candidate, key, policy and corpora; does not wait for LQ8 | LQ9 and LQ21, answered (RTE.21, RTE.24); LP3 acceptance; later changes of candidate require matching evidence |
| LX-7 | Its stage and renderer candidates, safety contract and precision contract; does not wait for LQ8 | User-stage and free-view feature handoff before renderer freeze (RPI.79, RTE.83) |
| LX-8 | LQ13's owner-set human protocol and equivalent IR candidates; does not wait for LQ8 | LQ13 before LP2's parser |
| LX-9 reference comparisons | Frozen requests and exact bisectors; do not wait for LQ8 | INV-10 and any EN1 contract-amendment packet |
| LX-9 engine-candidate comparison | LX-6 candidate | Same contract question; no assumed outcome |
| LQ8, answered: Rust (RTE.17) | LX-5 and LX-6, decisive on 14 November 2026 | Host for LX-1, LX-2 and LX-4 |
| LX-1 | LQ8 and frozen experiment versions of the three LP3 tools | LP3 acceptance for LQ9 and LQ21 (RTE.21, RTE.24) and LX-2's call mix |
| LX-2 | LQ8, the selected placement (RTE.21), LX-1's call mix and a renderer candidate presenting frames for RTE.42 conditions 3 and 4 | LP3 acceptance for LQ9; uses LX-4's echo bar without waiting for an LX-4 pass |
| LX-4 | LQ8, its generated module/edit fixtures and a renderer candidate presenting frames for RTE.51 step 8 and RTE.53 | LP3 acceptance for LQ9 and the later EV1 acceptance |
| LQ9 and LQ21 together, answered: RT1 in a process of its own, and capability separation (RTE.21, RTE.24) | RTE.82's leads and the decisive LX-5 and LX-6 runs | Form of flow control in ADR 6, ADR 7 and ADR 12; LP1's route interfaces |
| LP1 freeze gates and sign-off | Their full packets, including route decisions and LQ10's data-core prerequisites | Chapter 12's conditions before LP2 |
| LP2 parser and checker/IR | Applicable freeze exit, LQ13 for parser and LQ21 for checker/IR | LP3's implemented closed loop |
| LP3 end-to-end E2 measurement | LP2, final H-08 and renderer freeze for its on-screen half | Decisive product responsiveness with the three tools' views active |
| LP3 acceptance: decisive LX-1, LX-2 and LX-4 runs and LX-3's remaining checks | The Rust host, LP2's implemented tools and a renderer candidate presenting frames | A failing LX-1, LX-2 or LX-4 run reopens LQ9 (RTE.21); a failing LX-3 check goes to the owner (RTE.24) |

- **RTE.30 [D]** The pre-LP2 cards use experiment versions of the LP3 tools and interfaces. They do not require
  LP3's product implementation to exist first (partition LP1 and LP3, stage 2 protocol's cheapest-artifact principle).
  Their results are scoped to those frozen candidates. Completing a spike neither completes LP2 nor supplies LP3's
  end-to-end evidence. The rule and numeric echo bars can be shared before a measurement, so LX-2 and LX-4 form no cycle.
- **RTE.31 [B] Withdrawn.** The owner allowed pre-freeze experiment code, so deferred spikes no longer define an order (owner, 6 November 2026, LQ18).
- **RTE.32 [B]** The charter is amended: both freeze rulings sit at LP1's exit, and LP2 to LP5 may start after those
  rulings and the owner's sign-off (owner, 6 November 2026, LQ19 and decision 7). The language's development is complete
  when LP2 to LP5 are done apart from LP4's editor look. The design line resumes then, and its design-system ADR
  conditions LP4's editor look and LP6. The earlier schedule stays set aside (owner, 3 November 2026); no gate is relaxed.

## 11. Figures the experiments use

- **RTE.33 [D]** The following sizes and counts come from DT-L1 Round 3, engineer advisor, item 3. They are source
  evidence or source arithmetic, not timings. The table names each alphabet and the spike use. Stage paths in the
  source record resolve under `research/jumbling/restoration/`; no figure here implies a new literal replay or a
  selected product length limit (KN2.10, LB5.12, LB5.13).

| Figure | Value and unit | Class and source | Use and limit of the inference |
| --- | --- | --- | --- |
| Full labels | 1,039,200 bytes, 259,800 int32 labels | source; `core.py:18`, `assets/model.npz`; Round 3 item 3 | LX-2 transfer; no working-set or throughput claim |
| Pose-ID array alone | 708,480 bytes, 177,120 int32 indices | source; `research/jumbling/sim/state.py:103`; Round 3 item 3 | Only one component of a pose bundle |
| Pose bundle at P = 2,049 with one mask | 869,952 bytes = 708,480 pose IDs + 131,136 `poses.f32` + 8,196 `pose_lattice.i32` + 22,140 mask | source arithmetic; Round 3 item 3, `research/jumbling/render-contract.md`, stage 18 lattice-d1 records | LX-2 transfer; P is the recorded table-entry count, not a capacity ceiling |
| Earlier E2 literal step | 672,417 jumbling menu letters; 1,344,834 raw `ia16` bytes | source; Round 3 item 3 and `docs/progress/1.0/packets/engineering/ENG-0J-jumbling-backend.md` | LX-5 and long-word regression; an earlier example, not the largest literal word |
| Longest literal word stored flat | 2,352,984 I_a menu letters; 4,705,968 raw `ia16` bytes | source; Round 3 item 3, `research/jumbling/restoration/18-tail-completion/results/lattice-d1/run/run.json`, step 14, and `words/00014.ia16.gz` | LX-5 literal fixture and KN2.10 streaming |
| Whole lattice phase | 13,231,266 I_a menu letters in 53 word files; 26,462,532 raw `ia16` bytes | source; same lattice-d1 phase, Round 3 item 3 | LX-5 as one stream; no claim that all prefixes have been newly replayed |
| Stage 18 admission bound | 10⁷ menu letters per word; 20,000,000 raw `ia16` bytes | source; `research/jumbling/restoration/18-tail-completion/README.md:169`, Round 3 item 3 | LX-5 boundary fixture; distinct from DT-J1 X3's open product bound |
| Composed lattice word | 166,788,405 menu letters | source; `docs/progress/1.0/jumbling-handoff/research-data.md:125`, Round 3 item 3 | LX-5 exact length arithmetic; checked by composition, never played |
| Integer stress sizes | 2^64, 2^256 and about 22,500 bits | synthetic inputs proposed in Round 3 item 4, LX-5; KN3.2's B11 has 6,775 decimal digits | Exact arithmetic fixtures; the bit size is a proposed stress size, not an observed pose entry |
| Pose-entry stress | Beyond 2^64, built from the J1 negative control | synthetic; Round 3 item 4, LX-5; KN3.2 and ENG-0J section 4 | Unbounded heights motivate it; recorded tables have at most three decimal digits |

- **RTE.34 [D]** Transfer size, logical memory charge and stored size stay separate (KN7.9, PS1.6, PS1.7). The pose-ID
  array's `u32le-base64` carrier has 944,640 characters before its other fields (source arithmetic, Round 3 item 3);
  neither that text nor the render bundle is the logical exact-state charge. KN7.9 counts KN1 array bytes and each
  pose entry's canonical exact-key length, and LG4.7, DET.18 and DET.24 use that unit. No route may substitute allocator
  bytes, JSON bytes or physical sharing for it.

## 12. Experiment records and common conditions

- **RTE.35 [D]** Each section below is an experiment card under stage 2 protocol sections 1 and 2. Before a run,
  its ledger entry receives that protocol's experiment ID as well as its LX identifier, owner-set time box, frozen
  candidate/build identities, source and input digests, method and kill criteria. Each run records its result and the
  adopted, rejected or open decision with its decider. RTE.82 records the first-pass results; the choices waiting on
  their evidence remain open. A reached time box leaves the observed evidence and unfinished items; it is no pass.
- **RTE.36 [B]** Evidence classes stay separate under `AGENTS.md` and overview 3.4. Source or synthetic runs never
  establish actual Windows/DirectX, native input, long-session or performance results. Performance figures count only
  from the owner's machine. Where the cards require the renderer gate, they retain all 259,800 slots at full detail,
  the most complex animation and camera workload, average at least 30 fps, p99 at most 33.3 ms, native resolution,
  no frame generation and peak VRAM at most about 7 GB on the owner's RTX 4070 Laptop GPU (8 GB). These are thresholds
  from renderer-candidates and RPI.60, not measured results. Its recorded power, discrete-GPU, driver, display and
  repeat-run conditions remain part of the gate.
- **RTE.37 [D]** A spike leaves its frozen card, inputs or digest manifest, editable experiment source, production
  commands, tool versions, raw observations and report, including negative results (stage 2 protocol). A report names
  its evidence class and exact source/build scope. Results on another candidate do not become evidence for a changed
  one by relabelling the report. The card lists excluded timing, host and path fields where canonical payloads are
  compared. Fresh isolated fixture data is used; experiment code remains under `work/experiments/` and is never product
  code. This is a requirement on a permitted run; this chapter starts no run.
- **RTE.70 [B]** Each card names the tools and dependencies it uses. Already installed tools are used at pinned
  versions and recorded with the run. RTE.1 grants no installation; adding a third-party source outside
  `tools/toolchain.lock.json` needs the owner's approval first (`AGENTS.md`, "Ask the owner").

- **RTE.82 [D]** LX-1 to LX-9 ran as far as they can now under the owner's allowance (owner, 6 November 2026,
  LQ18). The records below are first passes and leads, limited to their frozen candidates, except LX-5's and LX-6's
  timing. Those ran in the owner's window of 14 November 2026, and the owner accepted them as decisive with the input
  recorded during the runs (`work/experiments/language-lx/quiet-window/2026-11-14T1818Z/RESULT.md`). The owner chose
  LQ8, LQ9 and LQ21 that day (RTE.17, RTE.21, RTE.24); no record decides LQ13. The decisive LX-1, LX-2 and LX-4 runs
  are LP3 acceptance checks; LX-2 and LX-4 use the existing renderer candidates first and run again if the renderer
  changes (owner, 6 November 2026, decision 15). RTE.36's evidence classes and RTE.37's source/build scope apply to
  every row.

| Spike | Record | Evidence class | What it showed | Still not run or established |
| --- | --- | --- | --- | --- |
| LX-1 | `work/experiments/language-lx/lx1-cost/RESULT.md` | source, synthetic; sandbox timing leads | 39 runnable prototype workload/mode cells, six declared not-run cells; comparable summary and cross-mode observable hashes agree. RT3 labels-off timing includes a different artifact-loading path and cannot measure label cost. | The Rust host (RTE.17), frozen real LP3 tools, real pose traffic, scale contrast and decisive E2, now LP3 acceptance checks (RTE.21). |
| LX-2 | `work/experiments/language-lx/lx2-placement/RESULT.md` | source, synthetic; sandbox timing leads | The separate process met conditions 1 and 2 for seven injected faults on this candidate. The shared-host native allocation, recursion, loop and forced-kill cases fail. | Conditions 3 and 4, first-frame startup, Linux and the decisive call-mix/renderer integration, now LP3 acceptance checks (RTE.21); the shell placement is not admitted. |
| LX-3 | `work/experiments/language-lx/lx3-sandbox-flow/corpus/RESULT.md`, `work/experiments/language-lx/lx3-sandbox-flow/part-b/RESULT.md`, its five `candidates/*/RESULT.md` records, and `work/experiments/language-lx/lx3-sandbox-flow/recheck/RESULT.md` | source, synthetic | Frozen corpus, controls and five stand-in candidates ran; failed flow forms, false refusals, unsupported cells and bench defects remain recorded. Recheck reproduced additional persisted-taint escapes outside the frozen counts. | Complete admitted-executor/platform coverage, the current execution-contract mapping and product sandbox admission, now LP3 acceptance checks; the owner chose capability separation (RTE.24). |
| LX-4 | `work/experiments/language-lx/lx4-editor/RESULT.md` | source, synthetic; function timing leads | Python/Rust front ends agree on the frozen stand-in inputs; 50 function jobs completed smoke and three 100-sample series. No input-to-frame bar was judged. | Frozen three-tool corpus, actual editor/renderer integration, foreground input/frame correlation, W3, E2 and the full decisive series, now LP3 acceptance checks (RTE.21). |
| LX-5 | `work/experiments/language-lx/lx5-arithmetic/RESULT.md`, `work/experiments/language-lx/lx5-crate/RESULT.md` and `work/experiments/language-lx/quiet-window/2026-11-14T1818Z/RESULT.md` | source, synthetic; decisive owner-machine timing (14 November 2026) | Node, .NET, own Rust limbs and `num-bigint` 0.5.1 each matched all 3,374 exact cases. Large words were streamed; 166,788,405 was length arithmetic only. Sums of the 190 row medians: .NET 136.6 ms, `num-bigint` 340.7 ms, own limbs 1,145.8 ms, Node 1,410.1 ms. `num-bigint`'s gcd and sign rows on random 22,500-bit inputs are its hot spots. | Linux leg and dependency approval; no speed threshold; the experiment's own limbs are no audited general-purpose library. |
| LX-6 | `work/experiments/language-lx/lx6-engine-port/RESULT.md` and `work/experiments/language-lx/quiet-window/2026-11-14T1818Z/RESULT.md` | source; synthetic controls; decisive owner-machine timing (14 November 2026) | All 3,201 required O01 to O07 steps equal the frozen oracle; the corrupted-trace control fails as expected. With equal output digests, the Rust port is 2.3, 5.1 and 7.2 times faster than NumPy at applying a generator, applying a long word and composing macros; NumPy is 15.2 times faster at the net effect with a warmed star cache the port lacks. | A net-effect comparison with the cache in the port and the Linux correctness leg; no whole-engine or end-to-end E2 result. |
| LX-7 | `work/experiments/language-lx/lx7-charts/RESULT.md` | source, synthetic; windows for the later named-device comparison; no performance | `lx7-r1` accepted four sources and refused 18; its compute readback has 496 records. Finite device/CPU differences are at most one binary32 ULP, with exact/float coverage disagreements retained. | Product precision verdict, the re-scoped safety set, vertex and fragment stages in a drawing pipeline and W3's combined gate (RTE.83). |
| LX-8 | `work/experiments/language-lx/lx8-surfaces/RESULT.md` | source | 17 programs in three spellings give 51 matching IR comparisons; 48 broken drafts and 45 semantic controls were checked. No syntax is ranked. | Owner-set protocol and human run; source equivalence is no learnability result. |
| LX-9 | `work/experiments/language-lx/lx9-input-map/RESULT.md`, its `followup/NOTE.md`, and `work/experiments/language-lx/lx9-exact-request/RESULT.md` | source, synthetic | Six signed-axis requests share one differing float event across platforms. The exact-request follow-up reproduced its stored cases, exact ties and menu bindings; its two free-mode meanings differ. | A6's EN1/owner amendment process, full default-grid Linux replay and a specified/tested gesture producer; no native-input or universal agreement claim. |

## 13. LX-1: interpreter cost

- **RTE.38 [D]** LX-1's experiment card uses DT-L1 E4 and Round 3 item 4. Its decision questions are LQ9 and LQ21.

| Field | Card |
| --- | --- |
| Question | What work and boundary traffic do the three LP3 tools require in each route and flow-label mode? |
| Decision it feeds | LQ9 route and placement; LQ21 label cost; LX-2's kernel-call mix |
| Hypothesis | None established; the partition expects bulk work in kernel batches, not per-sticker interpreter calls |
| Method | Freeze experiment versions of the Full, jumbling and representations tools and one input set by digest; run labels off, static only and with run-time labels; name the executed form and report the counters below |
| Effort report | Work to build each slice, in person or agent hours, with what was reused (RTE.9); a lead that sets no estimate |
| Time box | Owner sets it before the run |
| Kill criteria | Stop and retain a candidate run whose output, identity, accounting or source binding fails its frozen checks; reaching the time box leaves the comparison incomplete; a slow observation alone does not reject a route |
| Evidence class | source for work counts and headless cost leads; timing outside the owner-machine protocol is only a lead |
| Cannot show | Product responsiveness, a route pass, equal instruction counts across routes or LP3's charter E2 result |
| Product left behind | Frozen three-tool corpus, per-row call mix, transfer counts and the series report for LQ9/LQ21 and LX-2 |

- **RTE.39 [D]** Proposed decision rule, DT-L1 Round 3 item 4, LX-1: for each route and flow-label mode, run the
  three LP3 tools on one frozen input set listed by digest. Report per tool steps, kernel calls by row, bytes crossing
  the kernel boundary, and time per step and per call as median and p99 of three series of 100 runs. A step is one
  instruction of the route's executed form, which the report names. Step counts are never compared across routes.
  The result is a lead for LQ9 and LQ21. No LX-1 figure passes or fails a route or stands in for charter E2. The
  decisive check is LP3's end-to-end E2 measurement on the owner's machine.
- **RTE.40 [D]** LX-1's executed-instruction counter is distinct from DET.20's abstract budget meter and LG4.8's
  charge schedule. The draft `lp1-exec-draft-1` contract and its reference traces are in DET.54 to DET.65 and
  `contract-vectors/`. They do not reinterpret LX-1 counts or the frozen LX-3 analysis meter as semantic charges.
  RTE.71 requires a mapping for each admitted route; none is established here. A route that cannot preserve the
  common meter supplies one, is declined, or proposes a different versioned contract for the owner's LQ9 decision
  and chapter 05's compatibility treatment. The weighted alternative stays recorded in LG4.8.

## 14. LX-2: placement and faults

- **RTE.41 [D]** LX-2's experiment card uses DT-L1 E1, E4, R2-E3 and Round 3 item 4. It feeds LQ9.

| Field | Card |
| --- | --- |
| Question | Which placements preserve the session and responsiveness through runtime faults at the required call mix and transfer sizes? |
| Decision it feeds | LQ9 placement, including admission of the shell process |
| Hypothesis | E1 identifies shared shell/window/renderer failure and memory domains as a risk; it establishes no winning placement |
| Method | Compare in-process and separate-process kernel round trips at LX-1's call mix; carry the selected form's provenance on preview requests (DT-L1 R2-E3); transfer RTE.33's full label and pose bundles; run faults and cold startups under RTE.42 |
| Time box | Owner sets it before the run |
| Kill criteria | Failure of any fault condition or inability to survive forced kill fails that placement; preserve the counterexample and control; startup misses are reported against the proposed E3 bar; timeout leaves remaining cases unverified |
| Evidence class | performance on the owner's machine, with native Windows fault/input observations identified as windows evidence |
| Cannot show | Containment for untested hosts or faults, a general process-isolation proof, or an accepted E3 budget |
| Product left behind | Fault/control traces, journal and hash comparisons, call/transfer measurements, three startup records and the placement report |

- **RTE.42 [D]** Proposed decision rule, DT-L1 Round 3 item 4, LX-2: on the owner's machine, on a frozen
  candidate with fresh isolated data and a session open, inject allocation to its limit, recursion to its limit, a
  loop with no kernel call and a forced kill. A placement passes only if every fault satisfies all four conditions:
  (1) journal rows and state hash afterwards equal a control run of the same commands without the fault;
  (2) the job ends with a typed outcome and the session stays open;
  (3) the renderer gate and LX-4 echo bar hold during the fault window;
  (4) at least 100 commits issued during fault windows meet charter E2, repeating the fault until 100 are observed.
  A placement that cannot survive forced kill fails. Transfer is measured at 1,039,200 bytes of labels and 869,952
  bytes for the pose bundle at P = 2,049 with one mask. Startup runs from process start to the first interactive
  frame, with an existing session and runtime loaded, in each of three cold runs, within charter E3's proposed 3 s.
  E3 remains a proposal until the owner sets it. This rule feeds LQ9.
- **RTE.43 [D]** "Unchanged" in KN7.13 concerns the fault's effect, not the absence of legitimate solver commits
  during the test (ONE.70, DET.43). RTE.42 therefore compares the same command sequence with and without the fault.
  Each durable receipt still means committed. Charter E2 uses the full key-to-first-correct-frame interval; LP3
  specifies p95 at most 100 ms over at least 100 observations. A kernel round trip, an interpreter timer or renderer
  FPS alone cannot replace it (partition LP3; DT-L1 E1, E4).

## 15. LX-3: sandbox, determinism and refusals

- **RTE.44 [D]** LX-3's experiment card uses partition 8.4 and LX-3, DET.49 to DET.51 and DT-L1 Round 3 item 4.

| Field | Card |
| --- | --- |
| Question | Does each candidate enforce its sandbox and selected flow mechanism while reproducing exact results and reporting harmless-tool refusals? |
| Decision it feeds | LQ9 route admission and LQ21 form of flow control |
| Hypothesis | None established; deterministic inputs and conservative flow rules are requirements to test |
| Method | Freeze keys, exact payload profile, display precision, policy, allowed-tool list and must-refuse corpus by digest; test repeated Windows/Linux runs, each budget, interruption, cache changes and planted clock, file, network and process access without grants under DET.49; exercise the forms and variants in RTE.45 |
| Time box | Owner sets it before the run |
| Kill criteria | A reproducibility mismatch, successful planted read, unrefused prohibited case, crash, hang or wrong refusal code fails that form or variant; preserve the case; timeout gives no passing result for omitted cases |
| Evidence class | source; synthetic cases are identified separately where inputs are constructed |
| Cannot show | All-program safety, authorship, understanding, absence of covert channels, product latency or an acceptable false-refusal threshold |
| Product left behind | Digest-frozen refusal and allowed-tool corpora, canonical result hashes, per-form refusal lists and reproducibility/enforcement counterexamples |

- **RTE.45 [D]** The must-refuse corpus is frozen before a run. It includes selection by largest N_c, search
  followed by rebuilding from base parts, the F2 and F5 solve over ν and shipped corrections, workspace laundering,
  state-based choice between two words, flow through a module call, a stop-selected preview, refusal-driven selection, a
  one-item pick and accumulation across runs (partition LX-3, DT-L1 R2-E3, R3-M3, DET.50, BND.52). It runs
  under capability separation, static labels and additional run-time labels. RT2 and RT4 also identify the variant
  whose approved-subset checker refuses dynamic code and binds approval to executed bytes. A label mode off in LX-1
  is a cost baseline, not a successful flow-control variant. Expected registry codes and the allowed-tool denominator
  are frozen with the corpus, subject to the interim boundary; no case is excused after observing its result.
- **RTE.46 [D]** Proposed decision rule, DT-L1 Round 3 item 4, LX-3: cases and keys are frozen by digest before
  the run. Each key runs ten times on Windows and ten on headless Linux; all twenty runs give one SHA-256 over the
  canonical result bytes, comprising result value and stop reason in canonical JSON. Timing, host and path fields
  are excluded and listed on the experiment card. A display plan agrees when non-numeric fields are equal and each
  numeric field is within its declared precision. A case counts as refused only with its expected registry code,
  no state change, and no data returned by a planted read. A crash, hang or another code is a failure of that form.
  A false refusal refuses a program on the frozen list of built-in and solving-workflow tools the interim boundary
  allows. For each form and variant the report gives the count and each refused program with its code. No threshold
  is set; the count goes to the owner with LQ21. The resulting enforcement and determinism evidence also feeds LQ9.
  The result comparison retains DET.6's exact payload, including coverage and used deterministic counters. Declared
  precision applies only to approximate display fields; numeric exact fields remain exact under DET.32 and RPI.56.

## 16. LX-4: editor latency

- **RTE.47 [D]** LX-4's experiment card uses EV1.1 to EV1.9 and all of DT-L1 Round 3 item 4, LX-4.

| Field | Card |
| --- | --- |
| Question | Does a route's editor path meet the proposed feedback bars while turns and rendering meet their gates? |
| Decision it feeds | LQ9 and EV1's later acceptance; no choice of syntax, appearance or activation interaction |
| Hypothesis | The source corpus supports representative module sizes; no route latency is predicted |
| Method | Use RTE.48's sizes, RTE.49's bars and the nine-step protocol of RTE.51 on the owner's machine |
| Time box | Owner sets it and agrees the foreground measurement window before the run |
| Kill criteria | An S/M bar miss or any renderer/E2 miss fails as RTE.53 states; preserve complete series rather than stopping slow series; invalid correctness runs are retained and repeated whole; timeout leaves an incomplete record |
| Evidence class | performance on the owner's machine; corpus sizes are source evidence, not performance |
| Cannot show | Latency of arbitrary user views, learnability, design quality, or a pass for another candidate/build |
| Product left behind | Deterministic module generator and digest manifest, seeded edit script, frame/input correlations, all series and a bar-by-bar report |

- **RTE.48 [D]** Module sizes follow Round 3 item 4's source census: 33 research scripts range from 55 to
  1,034 lines, median 164, total 7,723, about 47 bytes per line. These are source counts reported in
  `docs/progress/1.0/design-table/topics/language-ecosystem.md`, not editor measurements. The sizes below are proposed
  fixtures generated from the three LP3 tools; no source syntax is chosen by their line counts.

| Size | Module/workspace size | Reason in Round 3 |
| --- | --- | --- |
| S | 200 lines | Near the median research script |
| M | 1,000 lines | Near the largest research script |
| L | 5,000 lines | Stress at about five times the largest script |
| W | 50 modules, about 20,000 lines | About 2.6 times the total of the 33 scripts |

- **RTE.49 [D]** The proposed bars are Round 3 item 4's p95 / p99 in milliseconds, measured from the key
  message except for the stated burst endpoint. None is a measured result. Every debounce counts against its bar.

| Interval | S | M | L |
| --- | --- | --- | --- |
| Typed echo | 50 / 67 | 50 / 67 | 50 / 67 |
| Syntax feedback | 67 / 100 | 67 / 100 | 100 / 150 |
| Completion list shown | 100 / 150 | 100 / 150 | 150 / 250 |
| Type and effect diagnostics, from the last key of a burst | 150 / 250 | 200 / 300 | 500 / 750 |
| Stale state shown on a view, charter D2 | 100 / 150 | 100 / 150 | 100 / 150 |
| Reference view re-run, updated view presented | 250 / 400 | 250 / 400 | 500 / 750 |

  At W an edit to an exported signature has its dependents re-checked within 1,000 / 1,500 ms, p95 / p99.

- **RTE.50 [D]** The reasoning is the one recorded with those bars. At the gate's 30 fps floor a key can land
  just after input sampling; two 33.3 ms frames give about 66.7 ms (arithmetic on the gate, not a timing). Syntax
  feedback is intended before the next key; the roughly 125 ms fast-typist gap is a common estimate, not project
  evidence. Completion uses charter E2's existing 100 ms sense of response. Diagnostics sit outside the keystroke
  path, between the usual 0.1 s and 1 s human-factors limits, which are also not project evidence. Round 3 uses p99
  about 1.5 times p95. The re-run bar applies only to a reference view of about 200 lines, one batch read of labels
  or pose IDs and a per-orbit count. Other views have only the stale-state bar; chapter 08 still governs stale data.
- **RTE.51 [D]** The proposed protocol has these nine steps, from DT-L1 Round 3 item 4. All run on the owner's
  machine.

  1. Freeze one candidate and its build identity. Generate the module set deterministically from the three LP3 tools
     and freeze it by digest.
  2. Use a fixed, seeded edit script with four kinds: inside a body, exported-signature change, completion trigger and
     syntax break.
  3. Run with Jumble off and Jumble on, at a fixture's end state, on fresh isolated data.
  4. Record B4-12 D6 conditions exactly, with frame generation off, PresentMon pinned under B4-12 D7 and a quiet
     machine. D6 records power source, Windows power mode, vendor performance mode, driver V-Sync and overlays;
     renderer-gate conditions also remain required for a gate result (RTE.36).
  5. Timed input is an owned-window message posted to the window's queue. Start at the post; end at the first
     presented frame showing the result, matched by frame ID to PresentMon.
  6. Keep automatic rendering on, unlike the M1 series, because the gate must hold in the same run.
  7. Take one smoke run, then three series of 100 samples per metric, size and puzzle. Never stop a series early for
     being slow. A correctness or state-hash failure invalidates the run; retain it and repeat it whole.
  8. In the same runs capture PresentMon at W3 and script at least 100 turns per series for E2.
  9. Report median, p95, p99 and largest value for each series, showing all three series.

- **RTE.52 [D]** Round 3's count is **10,800** samples: six metrics times three sizes S/M/L times two puzzles
  times three series times 100. At one second per sample this is about three hours of foreground time, source
  arithmetic from DT-L1 Round 3 item 4, not measured duration or a schedule. W's dependent re-check, the smoke run,
  setup and repeated invalid runs are additional work. The owner agrees the window and time box; none is fixed here.
- **RTE.53 [D]** Proposed decision rule, DT-L1 Round 3 item 4, LX-4: on the owner's machine, with one frozen
  candidate per route, the fixed module set and fixed edit script, an editor path passes when every bar holds in each
  of three series at S, M and L on both puzzles, and the renderer gate (259,800 slots, average at least 30 fps, p99
  at most 33.3 ms, peak VRAM at most about 7 GB) and charter E2 hold in the same runs. A bar missed at S or M fails the
  editor path until fixed. A bar missed only at L or W goes to the owner with the numbers. If editor work makes a
  run miss the gate or E2, the route fails at any size. No bar is relaxed without the owner. This rule feeds LQ9;
  an L-only miss is reported under the specific escalation sentence and cannot be called an all-bars pass.

## 17. LX-5: exact arithmetic

- **RTE.54 [D]** LX-5's experiment card uses KN3, LB5.13 and DT-L1 Round 3 items 3 and 4.

| Field | Card |
| --- | --- |
| Question | Which exact-arithmetic candidates meet KN3 at the recorded and synthetic sizes, with what dependency obligations and measured costs? |
| Decision it feeds | LQ8; arithmetic integration obligations for LQ9 |
| Hypothesis | Unbounded pose heights and recorded large lengths require arbitrary precision; no library speed or licence advantage is established |
| Method | Freeze arithmetic operations and reference answers, Q(√5) fixtures and the RTE.33 size corpus; construct large pose entries from the J1 negative control; compare exactly with Python's integer-backed reference and record licences |
| Time box | Owner sets it before the run |
| Kill criteria | Any exact-result difference removes that candidate; preserve operands and operation; timeout leaves untested sizes open; any proposed copyleft dependency goes to the owner |
| Evidence class | source for recorded fixtures and licence documents; synthetic for constructed large entries; performance only for owner-machine timings |
| Cannot show | That synthetic entries occurred in a saved state, that the largest length was played, a general speed threshold, or owner approval of a dependency |
| Product left behind | Exact-answer fixtures, synthetic-entry construction, per-size operation report and dependency/licence list |

- **RTE.55 [D]** Proposed decision rule, DT-L1 Round 3 item 4, LX-5: use integers at 2^64, 2^256 and about
  22,500 bits; synthetic pose entries built from the J1 negative control because no recorded table exceeds three
  decimal digits; literal words of 672,417 and 2,352,984 menu letters and the 10⁷ bound; 13,231,266 menu letters as
  one stream; and the 166,788,405-menu-letter length as exact length arithmetic. Every candidate's results equal
  Python's exact integers on every fixture; a difference removes the candidate. Report speed from the owner's machine
  as median and p99 per operation and size, with no speed threshold at this stage. A licence list goes to the owner;
  a copyleft dependency needs the owner. This rule feeds LQ8, not a new dependency authorization.

## 18. LX-6: engine language

- **RTE.56 [D]** LX-6's experiment card is the deferred engine-language question, narrowed by DT-L1 E4 and
  Round 3 item 4. The owner's LQ18 allowance covers experiment code only.

| Field | Card |
| --- | --- |
| Question | Does the proposed engine hot path match the 0.4 oracle, and what is its cost against NumPy on the owner's machine? |
| Decision it feeds | LQ8 together with LX-5; the candidate leg of LX-9 |
| Hypothesis | The wiki proposes Rust benefits and costs; RTE.82 records first-pass oracle equality and the decisive speed comparison of 14 November 2026, after which the owner chose Rust (RTE.17) |
| Method | Port permutation application and macro composition as experiment code; freeze O01 to O07 and compare through `tools/oracle/compare.py`; compare identical inputs against NumPy in the pinned engine environment by the B4-12 method |
| Time box | Owner sets it before the run; the wiki's earlier allocation does not bind |
| Kill criteria | Any oracle difference stops the spike until explained; retain the first differing case/step and digests; timeout leaves a partial port and report, not an accepted engine |
| Evidence class | source for oracle equality; performance for owner-machine comparison; cloud or headless timing is only a lead |
| Cannot show | Whole-engine correctness, persistence or input correctness, end-to-end E2, or a language choice from a speed threshold |
| Product left behind | Scoped hot-path port, oracle traces and comparison report, pinned inputs and NumPy timing comparison |

- **RTE.57 [D]** Proposed decision rule, DT-L1 Round 3 item 4, LX-6: the port covers applying a permutation
  and composing macros. O01 to O07 of `tools/oracle/` give the same state hash, net digest and expansion digest as the
  0.4 reference through `tools/oracle/compare.py`. Any difference is a demonstrated failure and stops the spike until
  explained. Speed compares the same inputs against NumPy in the pinned engine environment on the owner's machine,
  by the B4-12 method. Cloud or headless timing is a lead only. There is no speed threshold; both results went to
  LQ8 with LX-5 on 14 November 2026 (RTE.17, RTE.82).
- **RTE.58 [D]** The named oracle cases are the source cases in `tools/oracle/make_cases.py`: O01 every legal
  generator and inverse from solved; O02 a seeded chronological generator chain; O03 first, middle and last stars of
  every orbit after a seeded scramble; O04 mixed word/star recipes; O05 each mixed recipe followed by its chronological
  inverse; O06 collateral/target protection and net-only cancellation; O07 invalid inputs and suffixes preserving a
  seeded initial state. Their frozen case bytes, not just names, identify an experiment. Equality is checked at the
  required trace steps, not merely the final state (KN2.1; stage 2 protocol section 3). No oracle is run by this text.

## 19. LX-7: user stages

- **RTE.59 [D]** LX-7's experiment card tests the user stage contract of RPI.68 to RPI.80, following the owner's
  shader-language direction (owner, 6 November 2026, LQ7 and decision 2). RPI.35, RPI.36 and RPI.60 still govern
  precision and the renderer gate. The `lx7-r1` compute subset is first-pass evidence only (RTE.82, RTE.83).

| Field | Card |
| --- | --- |
| Question | Does the user stage contract admit safe vertex and fragment stages, preserve display and pick rules, and meet the renderer gate for the advertised combined workload? |
| Decision it feeds | Enablement under RPI.79 and RPI.80; the user-stage, safety-contract and free-view renderer handoff |
| Hypothesis | The compute subset supplies translation and device leads; it establishes no drawing-pipeline safety, raster bound or combined-workload timing |
| Method | Freeze checked vertex and fragment definitions, safety cases, built-in chain, exact vertices/camera poses, precision and singular exclusions; run the safety set, named-device reference comparisons and W3 drawing pipeline of RTE.83 |
| Time box | Owner sets it before the run |
| Kill criteria | A failed safety, raster-bound, pick or status check withholds user-stage enablement; a precision failure withholds chart admission; any cold run missing the whole gate fails that combination; invalid correctness runs provide no gate result; timeout leaves unfinished comparisons open |
| Evidence class | performance for owner-machine gate runs; windows for actual Windows GPU output; source/synthetic for reference fixtures, never for a GPU result inferred without a device |
| Cannot show | Arbitrary-chart correctness, an exact chart property from float agreement, renderer selection, or a performance pass from a no-window comparison |
| Product left behind | Frozen stage/reference and safety fixtures, drawing-pipeline experiment source, named-device comparisons and three-cold-run reports per advertised combination with a built-in comparison |

- **RTE.60 [D]** At W3 on the owner's machine, each advertised combination of views, vertex stage, fragment stage
  and overdraw allowance makes three cold runs (RPI.79), with three cold built-in-chain runs for comparison.
  A combination passes when each of its three runs meets the whole selection gate: all 259,800 slots at full detail,
  the most complex animation, average at least
  30 fps, p99 at most 33.3 ms, PresentMon at native resolution without frame generation and peak VRAM at most about
  7 GB. The raster-bound pass is included. Separately measured ceilings do not establish the combination. Report the
  difference from the built-in chain. For exact vertices and camera poses fixed before the run, GPU and CPU
  evaluations agree within the chart's declared precision outside its declared singular neighbourhoods. That
  comparison runs without a window, names its device and is never a gate result. The rule conditions stage enablement;
  it does not reconsider the owner's shader-language direction.
- **RTE.61 [D]** RPI.35 also requires GPU comparison with exact or interval expectations; CPU agreement alone does
  not prove the chart's exact properties. RPI.36's error metric and exclusions must be settled before judging LX-7.
  The safety contract is RPI.68 to RPI.80; a no-window compute comparison establishes none of its drawing-pipeline
  checks or combined gate. A bare "headless" claim cannot establish GPU evaluation; Round 3's device-named comparison
  resolves that wording in partition LX-7.

- **RTE.83 [D]** The re-scoped LX-7 still owes a safety test set for RPI.69 to RPI.78 and vertex and fragment stages
  in a drawing pipeline at W3. `lx7-r1`'s four accepted sources, 18 refusals and 496 readback records apply only to
  its compute subset. They establish neither rasterization nor the user stage contract. The new safety set covers
  forbidden operations and static bounds, checked-definition/compile identity, display-only outputs, drawing and
  picking with the same vertex stage, coverage, host statuses, built-in commit-preview isolation, quarantine and
  fallback. Raster-bound cases include spread versus overlapping triangles, shading samples and the ID pass, a
  near-plane crossing and a line-only plan with repeated edges (RPI.72; RM.40). A frame over the bound runs no
  user-stage fragment work. The advertised combination of views, vertex stage, fragment stage and overdraw allowance
  must pass the unchanged gate together in one run, with the bound pass included (RPI.79); RTE.60 requires three cold
  runs. No such W3 command exists yet.
  `work/experiments/language-lx/lx7-charts/W3-PLAN.md` records the missing renderer integration; repeating its existing
  built-in-stage command supplies no user-stage result.

## 20. LX-8: syntax learnability

- **RTE.62 [D]** LX-8's experiment card uses partition LQ13 and LX-8, LG1.8 and DT-L1 Round 3 item 4.

| Field | Card |
| --- | --- |
| Question | Which of two or three syntax candidates over one typed IR can non-authors learn and use under the owner's protocol? |
| Decision it feeds | LQ13 waits for LX-8 before LP2 builds the parser (owner, 6 November 2026, LQ13); the name is settled: 600 Cell Script, `.6cs` (LQ17 and decision 6) |
| Hypothesis | None supplied; author or model preferences are not human evidence |
| Method | Prepare equivalent typed operations, effects, literal order and charges under LG1.8; participants, tasks, observations and comparison are set only by the owner's protocol |
| Time box | Owner sets it with the protocol before a run |
| Kill criteria | No human run before that protocol exists; semantic mismatch prevents comparison as equivalent candidates; other early-stop criteria await the owner |
| Evidence class | human; equivalence of the semantic fixtures is separate source evidence |
| Cannot show | General learnability, product design sign-off, or a syntax decision by authors' or models' agreement |
| Product left behind | Equivalent candidate descriptions and source/IR fixtures; if allowed, the protocol's observations and owner decision |

- **RTE.63 [O]** No LX-8 decision rule exists until the owner sets the protocol (LQ13, DT-L1 Round 3 item 4).
  Agreement between authors or models does not replace it. Participant selection, tasks, sample plan, evaluation,
  stopping rules and treatment of inconclusive results remain open. Semantic, recovery and edition obligations in
  LG1 remain unchanged. No concrete syntax or comparison protocol is chosen here.

## 21. LX-9: input-map determinism

- **RTE.64 [D]** LX-9 follows the exact-request direction (owner, 6 November 2026, decision 8), written as proposed
  amendment A6 in KN3.4 and KN3.7 to KN3.9. It is not in force until EN1's Astra check and then the owner. DET.33
  and DET.34 separate the exact kernel request, shell producer and recorded replay. The card retains DT-L1 R2-E2's
  fresh-choice obligation with exact bytes as its input.

| Field | Card |
| --- | --- |
| Question | Do identical exact request bytes, operation, bounds and menu binding produce the same exact rotation and recorded choice data across reference and candidate runtimes? |
| Decision it feeds | INV-10 conformance; EN1's Astra check and the owner's ruling on proposed amendment A6 |
| Hypothesis | The float-path follow-up found different exact choices; the exact-request prototype supplies finite source/synthetic checks, not universal agreement or a gesture producer |
| Method | Freeze C/H/M bytes, free-mode bounds, exact bisectors and ordered-menu digests; compare exact free and menu choices, complete tie lists, distances and request digests across runtimes; test malformed, noncanonical, oversize and misbound requests and shell-producer controls separately |
| Time box | Owner sets it before the run |
| Kill criteria | Different exact choices or recorded choice data for identical bound inputs, repaired invalid bytes or an unrefused menu-binding mismatch fail that candidate; timeout leaves untested requests open |
| Evidence class | synthetic, on both platforms and the named candidate; no native-input or performance result |
| Cannot show | Equality for every request from a finite sample, input latency, or an approved change to state contract A1 |
| Product left behind | Preserved float-path findings, frozen exact-request and tie fixtures, exact-choice/refusal comparisons, producer controls and proposed amendment A6 evidence |

- **RTE.65 [D]** The former two-step protocol is history: step 1 compared target bits for 13,600 requests, and
  step 2 compared exact choices near certified bisectors. The Windows-only report,
  `work/experiments/language-lx/lx9-input-map/RESULT.md`, found no differing exact choice in its sample. The later
  `work/experiments/language-lx/lx9-input-map/followup/NOTE.md` records six signed-axis requests with different exact
  choices on Linux, all sharing one float computation.
  That finding fed decision 8. `work/experiments/language-lx/lx9-exact-request/RESULT.md` records the exact-request
  follow-up; it establishes its tested cases only. Under proposed amendment A6 the kernel reads C/H/M exactly,
  refuses malformed bytes without repair, and compares menu-drag candidates exactly in Q(√5) under the recorded
  order. Free mode takes the nearest point of A1's grid, the owner's choice (KN3.7). The shell produces
  bytes from gestures (KN3.9); identical gestures need not give identical bytes. Candidate conformance requires equal
  exact choices for identical bound inputs, not equal gestures. No experiment result itself amends A1.
- **RTE.66 [D]** Replaying the already recorded exact rotation or menu letter remains DET.33's different test. It
  does not recompute the chooser and cannot establish fresh-request agreement. LX-9 reads no labels off the lattice.
  Its R2-E2 dependency is named, and R3-M2 leaves LX-9 standing.

## 22. Source differences and limits

- **RTE.67 [D]** The following reconciliations delimit this chapter's use of its sources. Round 3 item 4 remains
  the source of experiment detail under the owner's LQ18 allowance; correcting a figure or identifying a missing
  mechanism decides no waiting question.

| Source difference | Treatment and clauses |
| --- | --- |
| Partition LX-2 and DT-L1 E4 use 708,480 bytes as pose IDs; Round 3 distinguishes the full pose bundle | RTE.33 and RTE.42 use 869,952 bytes at P = 2,049 with one mask. The smaller figure remains the ID array alone. |
| Earlier E3/E4 and ENG-0J cite 672,417 as the long-word example; Round 3 R3-E4 gives longer literals and compositions | RTE.33 and RTE.55 retain the earlier regression and use 2,352,984 literal letters plus the 13,231,266-letter stream; 166,788,405 is length arithmetic. KN2.10 and LB5.13 already carry that correction. |
| The engine-language wiki proposes headless speed comparison; partition and Round 3 count speed only on the owner's machine | RTE.20 and RTE.57 keep oracle equality as source evidence and headless timing as a lead. RTE.82 records first-pass oracle equality and the decisive owner-machine speed comparison of 14 November 2026. |
| Partition LX-7 calls GPU/CPU comparison headless; Round 3 requires an actual named device | RTE.60 and RTE.61 separate no-window device correctness from owner-machine gate measurements. CPU fixture preparation supplies no GPU result. |
| Round 3 LX-1 defines executed instructions; DET.20/DET.21 require a common logical meter | RTE.39 and RTE.40 keep separate counters. A mapping remains open; raw step counts cannot establish route-equivalent budgets. |
| KN7.9 specifies arrays plus exact keys; LG4.7 and DET.18/DET.24 use canonical-encoding language | RTE.34 uses KN7.9. The integrator must align the broader wording; render or JSON sizes do not determine logical memory. |
| Section 12's IR promise expressly covers RT1 to RT3; the task's common obligations also include RT4 | LG4.14 and RTE.6 require a separately admitted equivalent contract or common-IR lowering. No RT4 exemption or replacement validator is assumed. |
| B4-12 D6 records everyday conditions; the renderer gate also prescribes power and device conditions | RTE.36 and RTE.51 record conditions exactly and require the full gate for a gate claim. A differently labelled condition cannot replace it. |
| Older experiment documents and the wiki carry a schedule; LQ19 amends part of the charter exit | RTE.32 applies the answered order (owner, 6 November 2026, LQ19 and decision 7). The 3 November schedule hold stands. The owner set the timing window of 14 November 2026; other decisive windows and time boxes await the owner. |

## 23. Errors

- **RTE.68 [D]** This chapter adds no error codes; chapter 13 owns the registry used by candidate checks and refusals.

## 24. Conformance tests

- **RTE.69 [D]** These are requirements on route candidates and spike records under overview 3.4, collected by phase
  in chapter 12. RTE.82 scopes the existing first-pass observations; no product conformance pass follows from them.
  A document structure check supplies no experiment result. Checks involving an open mechanism use its selected,
  versioned contract and cannot decide it by themselves.

| Test | Checks | Class |
| --- | --- | --- |
| CT-RTE-01 | Every route inventory identifies the implementation and evidence for each RTE.4 row; missing checks, host conventions and unmeasured reuse are not reported as compliance | source |
| CT-RTE-02 | Candidate IR from RT1 to RT3 passes the same versioned validator and semantic fixtures; RT4 supplies that lowering or its explicitly admitted equivalent contract and compatibility evidence (RTE.6) | source |
| CT-RTE-03 | Enumerate every route/placement's capabilities and command rows: none opens session files, obtains a token, invokes commit or supplies state for adoption; preview swap remains refused | source |
| CT-RTE-04 | One frozen abstract trace preserves exact results, charges, stopping points and source associations across admitted executors; raw LX-1 instruction counts never substitute for the logical mapping | source |
| CT-RTE-05 | Each spike record names its LX card, experiment ID, owner allowance, time box, frozen candidate and input digests, commands/tool versions, evidence class, kill criteria, actual result and decision with decider | source |
| CT-RTE-06 | Before LX-3 starts, digest manifests freeze keys, expected registry codes, policy, must-refuse cases and allowed-tool denominator; later changes require another identified candidate/run | source |
| CT-RTE-07 | Each LX-3 form/variant runs all frozen keys ten times per platform; canonical exact hashes agree, display precision checks pass, expected refusals return no planted data, and every false refusal is listed | source |
| CT-RTE-08 | Static subset approval remains bound to executed dependency bytes; substitution or dynamic code is refused, and module, branch, preview-outcome and persisted-word cases retain the selected BM2 discipline | source |
| CT-RTE-09 | Cold/warm caches, eviction, aliasing and host allocation do not change KN7.9 charges; synthetic large entries and deep recursion end at the specified logical boundaries without host wraparound | synthetic |
| CT-RTE-10 | LX-2's native fault/control command sequences leave equal journal rows and hashes, preserve the open session and typed outcome, and retain the engine-owned pending preview after runtime death | windows |
| CT-RTE-11 | Every LX-2 fault window meets renderer and echo bars and contains at least 100 E2 commits; each of three cold startups is measured against the explicitly proposed E3 bar | performance |
| CT-RTE-12 | LX-4 freezes deterministic S/M/L/W fixtures, records all nine protocol steps and all three series per puzzle, accounts for 10,800 S/M/L samples separately from W/additional work and applies the stated S/M versus L/W rule | performance |
| CT-RTE-13 | LX-5 uses every RTE.55 size, obtains exact Python-reference agreement, streams literal letters without silent splitting and computes the 166,788,405-letter length without claiming literal replay | source |
| CT-RTE-14 | LX-6 identifies frozen O01 to O07 bytes and compares required state, net and expansion digests through the oracle comparator; a difference stops the spike with its case and step | source |
| CT-RTE-15 | Report inspection rejects headless/cloud timings as performance, sampled no-difference claims as universal proofs and no-window GPU comparison as a renderer gate pass | source |
| CT-RTE-16 | Each advertised LX-7 combination of views, vertex stage, fragment stage and overdraw allowance passes the whole unrelaxed gate together in each of three cold W3 drawing-pipeline runs, with the raster-bound pass included and built-in-chain comparisons; separate ceilings never establish the combination (RTE.60, RTE.83; RPI.79) | performance |
| CT-RTE-17 | LX-7's drawing pipeline names the device, checked vertex/fragment definitions, frozen exact inputs, error metric and exclusions; reference comparisons, draw/pick agreement, coverage and host statuses pass without claiming chart-property proof. The commit-bound preview uses the built-in chain (RTE.61, RTE.83; RPI.74 to RPI.76) | windows |
| CT-RTE-18 | LX-8 has the owner-set protocol and equivalent IR corpus before testing non-authors; its outcome is reported only under that protocol | human |
| CT-RTE-19 | Under proposed amendment A6, identical C/H/M bytes, operation, free-mode bounds and menu binding give equal exact choices, tie lists, distances and request digests across runtimes; malformed/noncanonical/oversize bytes and mismatched menu bindings refuse without repair. Producer controls stay separate from recorded replay and promise no gesture-to-byte equality (RTE.64 to RTE.66) | synthetic |
| CT-RTE-20 | Withdrawn: LQ18 allowed pre-freeze experiments, so the deferred-spike case no longer exists (owner, 6 November 2026, LQ18). | source |
| CT-RTE-21 | Each RTE.82 summary resolves to its recorded RESULT.md, keeps its evidence class and frozen scope and retains negative/unrun cases. Only the LX-5 and LX-6 rows cite decisive timing; no summary decides LQ13 or passes the LP3 acceptance checks that LQ9 and LQ21 carry | source |
| CT-RTE-22 | LX-7's safety set covers static stage refusals and bounds, deterministic compilation, display-only outputs, draw/pick and preview isolation, quarantine/fallback and pre-fragment raster limits, including overlapping triangles, samples, ID passes, near-plane crossings and line-only plans (RTE.83; RPI.69 to RPI.78; RM.40) | synthetic |
| CT-RTE-23 | The 1.0 obligation export uses LB5.18's file grant and PS1.45's format; no shipped Lean dependency, proof-record import, new check mode or formal-proof writer exists, and export promotes no claim or commit authority (RTE.28; EVD.64) | source |

## 25. Open items

The owner answered LQ1 to LQ21 on 6 November 2026 and chose LQ8, LQ9 and LQ21 on 14 November 2026: the engine and the
interpreter's host in Rust, route RT1 with the runtime in a process of its own, and capability separation (RTE.17,
RTE.21, RTE.24). LQ13 waits for its evidence, and LQ2's option set waits for the joint setting.

| Owner question | Clauses and dependency |
| --- | --- |
| LQ2 Option set and joint boundary setting | RTE.24 to RTE.27, RTE.45. Chapter 07 and the joint setting decide author classes, declassifiers and display sinks; each frozen refusal corpus names its policy identity. |
| LQ8 Engine language | RTE.17 to RTE.20, RTE.54 to RTE.58. Answered: Rust (owner, 14 November 2026), on LX-5 and LX-6's decisive runs. LP2 chooses the big-integer implementation, and the port adds the net-effect star cache. |
| LQ9 Route and placement | RTE.4 to RTE.23, RTE.29, RTE.30, RTE.38 to RTE.58. Answered: RT1 with the runtime in a process of its own (owner, 14 November 2026). RT2 to RT4 and the other two placements are records of the alternatives. The decisive LX-1, LX-2 and LX-4 runs are LP3 acceptance checks with unchanged bars; a failure reopens LQ9. |
| LQ13 Syntax | RTE.9, RTE.29, RTE.62, RTE.63. The choice follows LX-8 before LP2's parser; the human protocol and run remain owed, and the owner decides from their evidence. |
| LQ21 Flow-control form | RTE.4, RTE.12 to RTE.16, RTE.24 to RTE.27, RTE.39, RTE.45, RTE.46. Answered: capability separation (owner, 14 November 2026). LX-3's two false refusals are met by fixed declassifiers or split tools; its remaining checks are LP3 acceptance checks. |

Open mechanisms and dependencies:

- **RT4 admission, RTE.6 and RTE.80.** Closed: LQ9 selected RT1 (RTE.21), so no RT4 candidate is admitted and these
  clauses record the unselected alternative.
- **Bindings and zero-copy access [O], RTE.7.** LQ8 and LQ9 are answered: engine and runtime are both Rust, in
  separate processes. LP1 settles transfer or read-only sharing, immutable buffer lifetimes and exact-value transfer.
  No size table demonstrates an implemented binding.
- **Engine choice, RTE.17.** Answered: Rust. Owed: LP2's big-integer choice with any dependency approval, the port's
  net-effect star cache, the Linux legs and whole-engine correctness.
- **Placement, RTE.21.** Answered: a process of its own. LX-2's decisive run at LP3 acceptance judges containment; a
  process name alone supplies no pass, and a failure reopens LQ9.
- **Flow mechanism, RTE.24.** Answered: capability separation, before checker/IR construction; its policy stays
  separate (RTE.27).
- **Policy and sinks [O], RTE.27.** LQ2 and chapter 07 decide enabled flows, classes and exceptions; LX-3 cannot freeze
  an authoritative corpus without naming the policy it tests.
- **Candidate-specific facts, RTE.11.** RTE.82 records stand-in experiments, including failures. RT1 is selected,
  so no RT2 host is admitted; RT1's own dependencies still owe their licence list and any approval (RTE.70).
- **Metering [D], RTE.40; mapping and limits [O], DET.22/DET.28.** LG4.8 and DET.54 to DET.64 specify uniform
  semantic steps, input-size tariff forms, owning-handle release, canonical-byte output, serial child accounting and
  logical recursion frames. LG2.21 specifies the alternative display-region lowering and identity. Weighted steps,
  conservative retention, an output-size table, disjoint child allowances and a separate depth guard remain distinct
  alternatives under LG4.8 and DET.24/DET.26/DET.28. Executor mappings, production tariffs and limits, evidence and
  the owner freeze remain owed. Instruction timing supplies none of those contract choices.
- **Human protocol [O], RTE.63.** The owner fixes LX-8's participants, tasks, samples, stopping and decision rules.
- **Shared fixtures and byte profiles, RTE.6, RTE.35 to RTE.46.** DET.54 to DET.65 and the contract vectors supply
  draft accounting forms and synthetic examples; EV1.10/EV2.12 specify coordinates and stepping. Candidate validators,
  source/step mappings, the pinned production byte profile and error registry, and digest-frozen execution corpora
  remain prerequisites for the corresponding runs. A reference-model pass supplies no executor evidence.
- **EN3 and EN4, RTE.2, RTE.21, RTE.29.** Chapter 01's final batch/query rows, program/view lanes, availability,
  undo during jobs and provenance-bearing proposals are still handoffs, not delivered runtime interfaces.
- **Renderer precision and measurement, RTE.36, RTE.42, RTE.51, RTE.59 to RTE.61.** RPI.36's metric and singular exclusions,
  selected candidate/device, frozen W3 inputs, owner-pinned PresentMon and recorded conditions precede judgement.
  RTE.83's stage safety set and combined drawing-pipeline gate remain owed. Charter E3's 3 s remains proposed.
  No installation or renderer selection is authorized here.
- **Exact requests, RTE.64 to RTE.66.** Proposed amendment A6 waits for EN1's Astra check and then the owner. The
  owner chose the free-mode grid (KN3.7); KN3.9's gesture producer detail remains subject to that process. RTE.82's
  finite prototype results supply no universal agreement, native-input result or accepted amendment.
- **Source wording, RTE.34 and RTE.67.** The pose payload, old long-word example, wiki timing venue, GPU/headless
  wording and instruction/logical-meter distinction are reconciled explicitly. KN7.9's precise logical-memory unit
  is consumed by LG4.7 and DET.18/DET.24, with language layout and handle lifetimes in DET.57/DET.58. Physical
  allocation and stored/output byte counts remain separate; candidate mappings and measurements remain owed.
- **DT-L1 obligations, RTE.12 to RTE.16, RTE.24 to RTE.27, RTE.45, RTE.61, RTE.64 to RTE.66.** Uses of R2-E2 and
  R2-E3 are named. Round 3 cleared the obligations (overview section 8), and RTE.45 carries its added cases.
- **Content review and phase collection, RTE.69.** The integrator and Astra still review this candidate's content;
  chapter 12 collects conformance by phase and chapter 13 owns final error spelling. Passing a structure check proves
  neither content acceptance nor any experiment result.

- **RTE.71 [D]** Before claiming the same reproducibility key, a route supplies a checked mapping from each
  DET.55 operation and dynamic continuation to its executed form, preserving order, source identity, charge phases,
  logical lifetimes, child caps, semantic chunks and output units. RT1 implements the schedule in its interpreter.
  RT2 validates the admitted host subset and instruments the schedule at its boundaries. RT3 preserves those
  boundaries through optimization and lowering, including elided calls/allocations and final false loop tests.
  VM fuel and native instruction counts are enforcement aids, never a substitute for this mapping. RT4 either
  lowers to the common IR or brings RTE.6/LG4.14's checked host decomposition contract: every transitive/indirect
  call, effect and executed-source identity is covered, including the decomposition needed by LB6.2 and EV5.4.
  The non-IR mapping must detect an omitted helper or changed executed module and refuse admission. Equivalent
  signatures or successful final values alone cannot establish equal budget stops. If the owner selects a different
  contract, its distinct identity, vectors and chapter 05 compatibility treatment precede saved-run use.

- **RTE.72 [D]** LP1's reference model is source evidence for a proposed schedule, not admission of any executor.
  Integration runs `python -B docs/architecture/1.0/language/contract-vectors/check_vectors.py` and
  `python docs/architecture/1.0/language/closure/check_closure.py --packet p4`. Subsequent LX-3 evidence must replay
  the traces on each candidate and platform, including caches, operand-size tariffs, alias release, child ledgers,
  both display branches and transport splits. The frozen experiment profile remains untouched. LQ8 and LQ9 were
  answered on other evidence (RTE.17, RTE.21); these checks select neither an engine nor a route. The owner's LQ18
  allowance and LQ19's amended order apply (owner, 6 November 2026, LQ18, LQ19 and decision 7); no check result
  supplies either freeze ruling or sign-off.

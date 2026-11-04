# Chapter 11: implementation routes and spikes

Status: **draft for the owner. Not accepted.** Conventions are in [the overview](00-overview.md), section 3.

This chapter gives the obligations of each implementation route and the evidence needed to choose a route, an engine
language and a runtime placement. It supplies proposed experiment cards and decision rules. Nothing has been measured,
no language spike has run, and this chapter authorizes no run. The owner questions remain open.

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
  placement, syntax or form of flow control. LQ18 is the permission boundary for every spike below. It supplies no
  permission for an API-team run, new cost or tool installation.
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
  around ordinary values and branches does not supply it (DT-L1 R2-E3). No cell is a result or a cost estimate.

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
| Capability separation, if selected | LG3.13, BM2 | must be built | must be built | must be built | must be built |

- **RTE.5 [D]** Host tooling, a numeric library, VM fuel or a sandbox supplies only the part demonstrated against this
  inventory (LG4.14, DET.21, partition section 12). A candidate's report identifies the implementation of each row,
  the admitted source subset and the remaining gaps. In particular, a host editor does not establish EV1's domain
  diagnostics, and host integers do not establish the whole KN3 tower. The independent kernel gates remain the same.
- **RTE.6 [O]** RT4 does not contain RT1 to RT3's typed IR by definition. LG4.14 requires an equally versioned
  contract over its admitted host programs. LP1 and the owner's LQ9 decision must settle its admission: either lower
  the approved subset into the common validated IR, or specify and verify an equivalent host-program contract and
  validator. The latter needs its own compatibility and metering evidence; host annotations and conventions alone
  are insufficient. The common tests below apply to the shared IR where one exists, and to the stated equivalent
  obligations under a separately admitted replacement. This text defines neither validator.
- **RTE.7 [O]** The sources motivate a common host by zero-copy access but give no binding, ABI or buffer-lifetime
  protocol. LP1's interfaces, LQ8 and LQ9 settle it. Same-process read-only access can avoid a transport boundary;
  another process needs transfer or a separately specified read-only sharing mechanism. A different host language
  needs a checked language boundary as well. Each option preserves snapshot immutability, logical memory charges and
  kernel-only mutation. LX-1 and LX-2 report actual bytes and transfers; no route or placement is presumed zero-copy.

## 2. RT1: own language

- **RTE.8 [D]** RT1 owns the parser, checker, typed IR and interpreter, normally in the engine language (partition
  section 12). It supplies direct control over domain types, exactness, effects, determinism, budget checks and source
  diagnostics. Under flow labels it also owns the checker and executor values; under capability separation it enforces
  LG3.13 instead. It adds the largest implementation effort, including the editor and debugger services. Its speed
  remains unmeasured. LG4.4 starts with an interpreter and introduces no own VM, JIT or machine-code backend before
  measurement supports one.
- **RTE.9 [D]** RT1's front end is the difficult choice to reverse: source, diagnostics, module contracts and source
  associations become saved-work dependencies (partition section 12, DT-L1 E4). Choosing a borrowed surface syntax
  under LQ13 still leaves this route an own front end and interpreter. LX-1 and LX-4 bear directly on its cost and
  authoring path; LX-2 and LX-3 test placement and enforcement, and LX-5 and LX-6 inform its engine host. An experiment
  slice supplies effort evidence; the partition supplies no effort figure.

## 3. RT2: embedded runtime

- **RTE.10 [D]** RT2 embeds an existing scripting runtime with a domain library (partition section 12). Its candidates
  are **Luau, Lua 5.4, a JavaScript engine, CPython or a restricted subset, Rhai, a WebAssembly VM**. The partition
  identifies existing VMs, docs and editors as possible reuse and a faster start as an expectation. Domain types,
  effects, exactness and evidence discipline still need the checker and adapter of LG4.14. Host numeric semantics
  cannot introduce float decisions or integer overflow. No candidate here is selected or demonstrated compliant.
- **RTE.11 [O]** The repository supplies no candidate-specific licence, speed or enforcement result for this list.
  For each named candidate, LX-2, LX-3 and LX-5 must answer: which host facilities can the approved subset reach, can
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

- **RTE.17 [O]** LQ8 chooses the engine language. The following are its options, not outcomes. The portability
  boundary is invariant under all of them: engine, session store, command layer and view model remain free of
  Windows-only code, with runtime, libraries and checkers runnable headless on Linux (INV-9, RPI.51 to RPI.53).

| LQ8 option | What changes | What remains to prove |
| --- | --- | --- |
| Decide after LX-5 and LX-6 | Compare exact arithmetic dependencies and the engine hot path before choosing the host; no language is presumed | Oracle equality, arithmetic correctness, dependency terms and owner-machine timings for the selected candidate |
| Decide Rust now | Adopt the wiki question's proposed portable engine, store and command layer, with a narrow boundary to the GUI | Its expected benefits and costs remain proposals; correctness and measured performance still need their checks |
| Keep the exact Python and NumPy reference as the 1.0 engine | Wrap the reference behind the new interfaces rather than presuming a port | The new interface, lifecycle, limits and end-to-end bars still need verification; 0.4 timings do not prove them |

- **RTE.18 [P LQ8]** Under LQ8's recommendation, LX-5 and LX-6 feed the engine decision before the route is selected.
  Choosing Rust now or keeping Python and NumPy resolves the host earlier; it supplies no passing spike or product
  evidence. The wiki's Rust proposal expects speed, memory safety, type-level invariants and a PyO3 transition path,
  and records costs in the extra GUI boundary, compilation and reading burden. Those are the proposal's expectations.
  Its deferred spike has not run, and none is treated as a measured benefit here.
- **RTE.19 [D]** The interpreter's host normally follows the engine language to permit direct snapshot and kernel
  access (partition section 12). With another host, the binding must preserve the same identity, immutable lifetimes,
  exact values, declared charges and typed results through RTE.7's boundary. The report separates binding overhead
  from interpreter and kernel work. Neither choice makes the runtime depend on the renderer's language. The GUI
  language follows renderer selection in the wiki proposal; engine and runtime portability remain separate obligations.
- **RTE.20 [D]** The wiki question proposes comparison on "the same headless machine". DT-L1 E4 and Round 3 item 4,
  LX-6, narrow its use: oracle equality is source evidence, cloud or headless timing is a lead only, and the decisive
  speed comparison uses the same inputs against NumPy in the pinned engine environment on the owner's machine, by the
  B4-12 method. There is no speed threshold. Both results go to LQ8 with LX-5. The earlier schedule sets no time box.

## 7. LQ9: runtime placement

- **RTE.21 [O]** LQ9 chooses placement with the route. Each option must satisfy KN7.12, KN7.13 and every rule of
  chapter 08. LX-2 decides fault containment and responsiveness; co-location or a process boundary is no proof.

| Placement | What it gives and costs | What KN7.13 and chapter 08 require it to demonstrate |
| --- | --- | --- |
| Isolate inside the engine process | Direct access can avoid IPC; user execution shares the engine's physical resource and failure domain | Limits, no-kernel-call loops and the forced runtime kill cannot end the engine session or change its journal; snapshots cannot hold commit locks |
| Process of its own | A separate runtime failure domain; extra startup, transfer and round trips at LX-1's call mix | Runtime death ends jobs with typed outcomes, while the engine, store, pending engine preview and shell continue; no second process opens session files |
| Shell process | Can share the authoring host; user execution shares the window and renderer's crash and memory domain | Execution stays off the UI thread and kernel-key path; faults and forced kill leave the session and shell usable; admitted only if LX-2's fault run passes |

- **RTE.22 [D]** A runtime fault ends its jobs, never the session (KN7.13, ONE.70, DET.43). A previously durable
  command keeps its receipt; the fault cannot change it to a failed commit. An engine-owned pending preview stays
  available after runtime death, and runtime handles end with the run. Program output becomes stale under ONE.11 and
  ONE.12; the adopted engine bundle continues without waiting for it (ONE.64). No placement permits old output over a
  new state, a raw state to commit, direct session-file access or token access.
- **RTE.23 [P LQ9]** Under LQ9's recommendation, placement and route follow LX-1 to LX-5 and are decided with LQ21
  inside LP1. Choosing an own or embedded interpreter now reduces the route alternatives; it does not waive LX-2's
  condition on shell placement or KN7.13 for any placement. A placement that cannot survive the forced runtime kill
  fails the proposed LX-2 rule, even if ordinary cooperative cancellation works.

## 8. Flow control across routes

- **RTE.24 [O]** LQ21 selects flow labels or capability separation, including static-only or additional run-time
  labels. Chapter 07 owns the rule, policy, provenance and declassifiers. This table records the implementation
  consequences in partition section 12 and LG3.12 to LG3.14 (DT-L1 R2-E3); it selects none.

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
- **RTE.26 [D]** LQ21 precedes LP2's checker and IR under either LQ18 order (partition sections 12 and 14). Adding
  the checks afterwards would refuse saved tools that had been allowed to run (DT-L1 R2-E3). Stored words, macros and
  workspace records keep BM2 provenance under either form (KN4.4; PS1.32 to PS1.34). The choice changes checking and
  execution, not the kernel's user-action gate or independent evidence checking.
- **RTE.27 [O]** LQ2's joint setting decides which author classes and display sinks the flow rule covers and which
  fixed declassifiers it enables. LQ21 decides the mechanism, not those permissions. Until that setting, partition
  section 11's interim reading and LG3.16 apply: built-in tools keep the 2 October boundary and 17 October repair-list
  exception, research output never becomes a solving-session preview, and solver-tool solving previews remain disabled.
  These are the conditions for constructing LX-3's allowed-tool and must-refuse lists; an experiment does not answer LQ2.

## 9. LQ15: formal proofs

- **RTE.28 [P LQ15]** The recommendation uses certificates and independent checkers in 1.0, with no proof assistant
  as the language and a proof-assistant link left to 1.x (partition LQ15). Lean 4 as the research language would add
  a separate research front end and an adapter to the common identities, exact kernels and stored evidence. Exporting
  proof obligations to Lean instead would add an export and checked-return path. Either alternative needs its own
  specification and evidence; neither changes the commit gate or promotes a claim merely because it was exported.
  Chapter 06 owns the meaning of a formal-proof check. This chapter chooses no alternative or proof protocol.

## 10. Order of decisions and spikes

- **RTE.29 [P LQ18]** Under LQ18's recommendation, the permitted spikes are experiment code under `work/experiments/`
  and never product code. They may start in LP0 and supply decisions inside LP1 before its freeze gates. This is the
  acyclic order of partition sections 12 and 14; a listed independent start needs its own frozen experiment inputs.

| Work or decision | Waits for | Feeds or constrains |
| --- | --- | --- |
| Owner's LQ18 allowance and each complete experiment card | LP0 direction; owner-set time box and scope | Permission for the named experiments only |
| LX-5 and LX-6 | Their allowed cards and frozen arithmetic/oracle fixtures | LQ8 |
| LX-3 | Its frozen candidate, key, policy and corpora; does not wait for LQ8 | LQ9 and LQ21; later changes of candidate require matching evidence |
| LX-7 | Its chart and renderer candidates and precision contract; does not wait for LQ8 | LQ7 and LQ20's feature handoff before renderer freeze |
| LX-8 | LQ13's owner-set human protocol and equivalent IR candidates; does not wait for LQ8 | LQ13 before LP2's parser |
| LX-9 reference comparisons | Frozen requests and exact bisectors; do not wait for LQ8 | INV-10 and any EN1 contract-amendment packet |
| LX-9 engine-candidate comparison | LX-6 candidate | Same contract question; no assumed outcome |
| LQ8 | LX-5 and LX-6 under its recommendation | Host for LX-1, LX-2 and LX-4 |
| LX-1 | LQ8 and frozen experiment versions of the three LP3 tools | LQ9, LQ21 and LX-2's call mix |
| LX-2 | LQ8, its placements and LX-1's call mix | LQ9; uses LX-4's echo bar without waiting for an LX-4 pass |
| LX-4 | LQ8 and its generated module/edit fixtures | LQ9 and the later EV1 acceptance |
| LQ9 and LQ21 together | LX-1 to LX-5 under the recommendations | Form of flow control in ADR 6, ADR 7 and ADR 12; LP1's route interfaces |
| LP1 freeze gates and sign-off | Their full packets, including route decisions and LQ10's data-core prerequisites | Chapter 12's conditions before LP2 |
| LP2 parser and checker/IR | Applicable freeze exit, LQ13 for parser and LQ21 for checker/IR | LP3's implemented closed loop |
| LP3 end-to-end E2 measurement | LP2, final H-08 and renderer freeze for its on-screen half | Decisive product responsiveness with the three tools' views active |

- **RTE.30 [P LQ18]** The pre-LP2 cards use experiment versions of the LP3 tools and interfaces. They do not require
  LP3's product implementation to exist first (partition LP1 and LP3, stage 2 protocol's cheapest-artifact principle).
  Their results are scoped to those frozen candidates. Completing a spike neither completes LP2 nor supplies LP3's
  end-to-end evidence. The rule and numeric echo bars can be shared before a measurement, so LX-2 and LX-4 form no cycle.
- **RTE.31 [P LQ18]** Under LQ18's alternative, no spike runs before the architecture freeze. The same spike dependency
  order moves after LP1's gates and before LP2, with LQ8, LQ9 and LQ21 then decided from it; LQ13 still precedes the
  parser. The frozen interfaces must hold under every RT1 to RT4 route and both forms of flow control, including
  RTE.6's RT4 obligation, provenance carriers, immutable snapshot lifetimes and deterministic budgets. Unverified
  compatibility remains an explicit freeze-packet dependency. If a later candidate requires an incompatible interface,
  it needs the applicable reviewed owner decision; deferral supplies no implicit amendment or passing evidence.
- **RTE.32 [B]** The signed charter's stage 2.5 exit includes the architecture-freeze ruling, migration-format-freeze
  ruling and accepted design-system ADR before implementation. The earlier schedule is set aside by the 3 November
  owner decision; no gate is relaxed. LQ19's recommendation would allow LP2 to LP5 after the two rulings through an
  owner-approved charter amendment. Its alternatives change that order. Until the owner decides, chapter 12 must keep
  the charter's whole exit as a prerequisite. This chapter grants no early product implementation.

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

- **RTE.35 [P LQ18]** Each section below is an experiment card under stage 2 protocol sections 1 and 2. Before a run,
  its ledger entry receives that protocol's experiment ID as well as its LX identifier, owner-set time box, frozen
  candidate/build identities, source and input digests, method and kill criteria. Each run records its result and the
  adopted, rejected or open decision with its decider. The present result of every card is **not run** and its decision
  is **open**. A reached time box leaves the observed evidence and unfinished items; it is no pass. Under LQ18's
  alternative, these cards remain specifications until the post-freeze allowance.
- **RTE.36 [B]** Evidence classes stay separate under `AGENTS.md` and overview 3.4. Source or synthetic runs never
  establish actual Windows/DirectX, native input, long-session or performance results. Performance figures count only
  from the owner's machine. Where the cards require the renderer gate, they retain all 259,800 slots at full detail,
  the most complex animation and camera workload, average at least 30 fps, p99 at most 33.3 ms, native resolution,
  no frame generation and peak VRAM at most about 7 GB on the owner's RTX 4070 Laptop GPU (8 GB). These are thresholds
  from renderer-candidates and RPI.60, not measured results. Its recorded power, discrete-GPU, driver, display and
  repeat-run conditions remain part of the gate.
- **RTE.37 [P LQ18]** A spike leaves its frozen card, inputs or digest manifest, editable experiment source, production
  commands, tool versions, raw observations and report, including negative results (stage 2 protocol). A report names
  its evidence class and exact source/build scope. Results on another candidate do not become evidence for a changed
  one by relabelling the report. The card lists excluded timing, host and path fields where canonical payloads are
  compared. Fresh isolated fixture data is used; experiment code remains under `work/experiments/` and is never product
  code. This is a requirement on a permitted future run, not a request to create those files in this stage.

## 13. LX-1: interpreter cost

- **RTE.38 [P LQ18]** LX-1's experiment card uses DT-L1 E4 and Round 3 item 4. Its decision questions are LQ9 and LQ21.

| Field | Card |
| --- | --- |
| Question | What work and boundary traffic do the three LP3 tools require in each route and flow-label mode? |
| Decision it feeds | LQ9 route and placement; LQ21 label cost; LX-2's kernel-call mix |
| Hypothesis | None established; the partition expects bulk work in kernel batches, not per-sticker interpreter calls |
| Method | Freeze experiment versions of the Full, jumbling and representations tools and one input set by digest; run labels off, static only and with run-time labels; name the executed form and report the counters below |
| Time box | Owner sets it before the run |
| Kill criteria | Stop and retain a candidate run whose output, identity, accounting or source binding fails its frozen checks; reaching the time box leaves the comparison incomplete; a slow observation alone does not reject a route |
| Evidence class | source for work counts and headless cost leads; timing outside the owner-machine protocol is only a lead |
| Cannot show | Product responsiveness, a route pass, equal instruction counts across routes or LP3's charter E2 result |
| Product left behind | Frozen three-tool corpus, per-row call mix, transfer counts and the series report for LQ9/LQ21 and LX-2 |

- **RTE.39 [P LQ18]** Proposed decision rule, DT-L1 Round 3 item 4, LX-1: for each route and flow-label mode, run the
  three LP3 tools on one frozen input set listed by digest. Report per tool steps, kernel calls by row, bytes crossing
  the kernel boundary, and time per step and per call as median and p99 of three series of 100 runs. A step is one
  instruction of the route's executed form, which the report names. Step counts are never compared across routes.
  The result is a lead for LQ9 and LQ21. No LX-1 figure passes or fails a route or stands in for charter E2. The
  decisive check is LP3's end-to-end E2 measurement on the owner's machine.
- **RTE.40 [O]** LX-1's executed-instruction counter is distinct from DET.20's abstract budget meter and LG4.8's
  charge schedule. LP1 must fix the latter and verify each admitted route's mapping (DET.21, DET.22). The options
  remain operation counts or a fixed weight table; a route that cannot preserve the common meter supplies one, is
  declined, or proposes a different versioned contract to LQ9. This chapter compares no raw instruction counts as
  equivalent work and claims no established cross-route budget mapping.

## 14. LX-2: placement and faults

- **RTE.41 [P LQ18]** LX-2's experiment card uses DT-L1 E1, E4, R2-E3 and Round 3 item 4. It feeds LQ9.

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

- **RTE.42 [P LQ18]** Proposed decision rule, DT-L1 Round 3 item 4, LX-2: on the owner's machine, on a frozen
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

- **RTE.44 [P LQ18]** LX-3's experiment card uses partition 8.4 and LX-3, DET.49 to DET.51 and DT-L1 Round 3 item 4.

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

- **RTE.45 [P LQ18]** The must-refuse corpus is frozen before a run. It includes selection by largest N_c, search
  followed by rebuilding from base parts, the F2 and F5 solve over ν and shipped corrections, workspace laundering,
  state-based choice between two words, flow through a module call, a stop-selected preview, refusal-driven selection, a
  one-item pick and accumulation across runs (partition LX-3, DT-L1 R2-E3, R3-M3, DET.50, BND.52). It runs
  under capability separation, static labels and additional run-time labels. RT2 and RT4 also identify the variant
  whose approved-subset checker refuses dynamic code and binds approval to executed bytes. A label mode off in LX-1
  is a cost baseline, not a successful flow-control variant. Expected registry codes and the allowed-tool denominator
  are frozen with the corpus, subject to the interim boundary; no case is excused after observing its result.
- **RTE.46 [P LQ18]** Proposed decision rule, DT-L1 Round 3 item 4, LX-3: cases and keys are frozen by digest before
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

- **RTE.47 [P LQ18]** LX-4's experiment card uses EV1.1 to EV1.9 and all of DT-L1 Round 3 item 4, LX-4.

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

- **RTE.48 [P LQ18]** Module sizes follow Round 3 item 4's source census: 33 research scripts range from 55 to
  1,034 lines, median 164, total 7,723, about 47 bytes per line. These are source counts reported in
  `docs/progress/1.0/design-table/topics/language-ecosystem.md`, not editor measurements. The sizes below are proposed
  fixtures generated from the three LP3 tools; no source syntax is chosen by their line counts.

| Size | Module/workspace size | Reason in Round 3 |
| --- | --- | --- |
| S | 200 lines | Near the median research script |
| M | 1,000 lines | Near the largest research script |
| L | 5,000 lines | Stress at about five times the largest script |
| W | 50 modules, about 20,000 lines | About 2.6 times the total of the 33 scripts |

- **RTE.49 [P LQ18]** The proposed bars are Round 3 item 4's p95 / p99 in milliseconds, measured from the key
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

- **RTE.50 [P LQ18]** The reasoning is the one recorded with those bars. At the gate's 30 fps floor a key can land
  just after input sampling; two 33.3 ms frames give about 66.7 ms (arithmetic on the gate, not a timing). Syntax
  feedback is intended before the next key; the roughly 125 ms fast-typist gap is a common estimate, not project
  evidence. Completion uses charter E2's existing 100 ms sense of response. Diagnostics sit outside the keystroke
  path, between the usual 0.1 s and 1 s human-factors limits, which are also not project evidence. Round 3 uses p99
  about 1.5 times p95. The re-run bar applies only to a reference view of about 200 lines, one batch read of labels
  or pose IDs and a per-orbit count. Other views have only the stale-state bar; chapter 08 still governs stale data.
- **RTE.51 [P LQ18]** The proposed protocol has these nine steps, from DT-L1 Round 3 item 4. All run on the owner's
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

- **RTE.52 [P LQ18]** Round 3's count is **10,800** samples: six metrics times three sizes S/M/L times two puzzles
  times three series times 100. At one second per sample this is about three hours of foreground time, source
  arithmetic from DT-L1 Round 3 item 4, not measured duration or a schedule. W's dependent re-check, the smoke run,
  setup and repeated invalid runs are additional work. The owner agrees the window and time box; none is fixed here.
- **RTE.53 [P LQ18]** Proposed decision rule, DT-L1 Round 3 item 4, LX-4: on the owner's machine, with one frozen
  candidate per route, the fixed module set and fixed edit script, an editor path passes when every bar holds in each
  of three series at S, M and L on both puzzles, and the renderer gate (259,800 slots, average at least 30 fps, p99
  at most 33.3 ms, peak VRAM at most about 7 GB) and charter E2 hold in the same runs. A bar missed at S or M fails the
  editor path until fixed. A bar missed only at L or W goes to the owner with the numbers. If editor work makes a
  run miss the gate or E2, the route fails at any size. No bar is relaxed without the owner. This rule feeds LQ9;
  an L-only miss is reported under the specific escalation sentence and cannot be called an all-bars pass.

## 17. LX-5: exact arithmetic

- **RTE.54 [P LQ18]** LX-5's experiment card uses KN3, LB5.13 and DT-L1 Round 3 items 3 and 4.

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

- **RTE.55 [P LQ18]** Proposed decision rule, DT-L1 Round 3 item 4, LX-5: use integers at 2^64, 2^256 and about
  22,500 bits; synthetic pose entries built from the J1 negative control because no recorded table exceeds three
  decimal digits; literal words of 672,417 and 2,352,984 menu letters and the 10⁷ bound; 13,231,266 menu letters as
  one stream; and the 166,788,405-menu-letter length as exact length arithmetic. Every candidate's results equal
  Python's exact integers on every fixture; a difference removes the candidate. Report speed from the owner's machine
  as median and p99 per operation and size, with no speed threshold at this stage. A licence list goes to the owner;
  a copyleft dependency needs the owner. This rule feeds LQ8, not a new dependency authorization.

## 18. LX-6: engine language

- **RTE.56 [P LQ18]** LX-6's experiment card is the deferred engine-language question, narrowed by DT-L1 E4 and
  Round 3 item 4. It starts no port without LQ18's allowance.

| Field | Card |
| --- | --- |
| Question | Does the proposed engine hot path match the 0.4 oracle, and what is its cost against NumPy on the owner's machine? |
| Decision it feeds | LQ8 together with LX-5; the candidate leg of LX-9 |
| Hypothesis | The wiki proposes Rust benefits and costs; none has a measured result and the engine language remains open |
| Method | Port permutation application and macro composition as experiment code; freeze O01 to O07 and compare through `tools/oracle/compare.py`; compare identical inputs against NumPy in the pinned engine environment by the B4-12 method |
| Time box | Owner sets it before the run; the wiki's earlier allocation does not bind |
| Kill criteria | Any oracle difference stops the spike until explained; retain the first differing case/step and digests; timeout leaves a partial port and report, not an accepted engine |
| Evidence class | source for oracle equality; performance for owner-machine comparison; cloud or headless timing is only a lead |
| Cannot show | Whole-engine correctness, persistence or input correctness, end-to-end E2, or a language choice from a speed threshold |
| Product left behind | Scoped hot-path port, oracle traces and comparison report, pinned inputs and NumPy timing comparison |

- **RTE.57 [P LQ18]** Proposed decision rule, DT-L1 Round 3 item 4, LX-6: the port covers applying a permutation
  and composing macros. O01 to O07 of `tools/oracle/` give the same state hash, net digest and expansion digest as the
  0.4 reference through `tools/oracle/compare.py`. Any difference is a demonstrated failure and stops the spike until
  explained. Speed compares the same inputs against NumPy in the pinned engine environment on the owner's machine,
  by the B4-12 method. Cloud or headless timing is a lead only. There is no speed threshold; both results go to LQ8
  with LX-5.
- **RTE.58 [D]** The named oracle cases are the source cases in `tools/oracle/make_cases.py`: O01 every legal
  generator and inverse from solved; O02 a seeded chronological generator chain; O03 first, middle and last stars of
  every orbit after a seeded scramble; O04 mixed word/star recipes; O05 each mixed recipe followed by its chronological
  inverse; O06 collateral/target protection and net-only cancellation; O07 invalid inputs and suffixes preserving a
  seeded initial state. Their frozen case bytes, not just names, identify an experiment. Equality is checked at the
  required trace steps, not merely the final state (KN2.1; stage 2 protocol section 3). No oracle is run by this text.

## 19. LX-7: user charts

- **RTE.59 [P LQ18]** LX-7's experiment card uses chapter 10, particularly RPI.32 to RPI.41 and RPI.60 to RPI.62,
  and DT-L1 Round 3 item 4. The generated chart path is LQ7's recommendation, not an accepted feature.

| Field | Card |
| --- | --- |
| Question | Does a generated projection stage agree with its declared reference precision and meet the renderer selection gate? |
| Decision it feeds | LQ7; LQ20's generated-stage feature handoff |
| Hypothesis | The partition identifies per-frame vertex work as requiring GPU evaluation; it supplies no generated-stage timing |
| Method | Freeze the admitted subset candidate, generated stage, built-in perspective stage, exact vertices/camera poses, precision and singular exclusions; run W3 and the no-window named-device comparison below |
| Time box | Owner sets it before the run |
| Kill criteria | A precision failure withholds chart admission; any cold run missing the whole gate fails that candidate; invalid correctness runs provide no gate result; timeout leaves unfinished comparisons open |
| Evidence class | performance for owner-machine gate runs; windows for actual Windows GPU output; source/synthetic for reference fixtures, never for a GPU result inferred without a device |
| Cannot show | Arbitrary-chart correctness, an exact chart property from float agreement, renderer selection, or a performance pass from a no-window comparison |
| Product left behind | Frozen chart/reference fixtures, generated-stage experiment source, named-device comparison and three-cold-run reports per stage |

- **RTE.60 [P LQ18]** Proposed decision rule, DT-L1 Round 3 item 4, LX-7: at W3 on the owner's machine the generated
  chart stage and built-in perspective stage each make three cold runs. A candidate passes when each of its three
  runs meets the whole selection gate: all 259,800 slots at full detail, the most complex animation, average at least
  30 fps, p99 at most 33.3 ms, PresentMon at native resolution without frame generation and peak VRAM at most about
  7 GB. Report the difference from the built-in stage. For exact vertices and camera poses fixed before the run, GPU
  and CPU evaluations agree within the chart's declared precision outside its declared singular neighbourhoods. That
  comparison runs without a window, names its device and is never a gate result. The rule feeds LQ7.
- **RTE.61 [D]** RPI.35 also requires GPU comparison with exact or interval expectations; CPU agreement alone does
  not prove the chart's exact properties. RPI.36's error metric and exclusions must be settled before judging LX-7.
  Under built-in-only LQ7, H-06 measures those built-in stages and no user chart is enabled. A user shader-language
  alternative needs RPI.39's separate safety contract. A bare "headless" claim cannot establish GPU evaluation;
  Round 3's device-named comparison resolves that wording in partition LX-7.

## 20. LX-8: syntax learnability

- **RTE.62 [P LQ18]** LX-8's experiment card uses partition LQ13 and LX-8, LG1.8 and DT-L1 Round 3 item 4.

| Field | Card |
| --- | --- |
| Question | Which of two or three syntax candidates over one typed IR can non-authors learn and use under the owner's protocol? |
| Decision it feeds | LQ13 before LP2 builds the parser; working name LQ17 remains the owner's choice |
| Hypothesis | None supplied; author or model preferences are not human evidence |
| Method | Prepare equivalent typed operations, effects, literal order and charges under LG1.8; participants, tasks, observations and comparison are set only by the owner's protocol |
| Time box | Owner sets it with the protocol before a run |
| Kill criteria | No human run before that protocol exists; semantic mismatch prevents comparison as equivalent candidates; other early-stop criteria await the owner |
| Evidence class | human; equivalence of the semantic fixtures is separate source evidence |
| Cannot show | General learnability, product design sign-off, or a syntax decision by authors' or models' agreement |
| Product left behind | Equivalent candidate descriptions and source/IR fixtures; if allowed, the protocol's observations and owner decision |

- **RTE.63 [O]** No LX-8 decision rule exists until the owner sets the protocol (LQ13, DT-L1 Round 3 item 4).
  Agreement between authors or models does not replace it. Participant selection, tasks, sample plan, evaluation,
  stopping rules and treatment of inconclusive results remain open. Choosing a syntax now removes LX-8 as a
  prerequisite to that choice; semantic, recovery and edition obligations in LG1 remain unchanged. No concrete syntax
  or comparison protocol is chosen here.

## 21. LX-9: input-map determinism

- **RTE.64 [P LQ18]** LX-9's experiment card uses KN3.4, DET.33, DET.34, DT-L1 R2-E2 and Round 3 item 4.

| Field | Card |
| --- | --- |
| Question | Do fresh A1 input requests and menu-mode drag choices select the same exact rotation across the reference and candidate runtimes? |
| Decision it feeds | INV-10 conformance; EN1's Astra check and owner decision on any state-contract amendment |
| Hypothesis | R2-E2 found non-correctly-rounded trigonometric targets on its synthetic corpus; it did not establish different chosen rotations |
| Method | Freeze requests and exact bisectors by digest; compare target bits on Windows CPython, Linux CPython and the LX-6 engine candidate, then exact chosen-rotation keys near the bisectors |
| Time box | Owner sets it before the run |
| Kill criteria | Any different chosen rotation stops an unqualified cross-runtime claim and triggers the amendment packet; target-bit differences alone continue to step 2; timeout leaves untested requests open |
| Evidence class | synthetic, on both platforms and the named candidate; no native-input or performance result |
| Cannot show | Equality for every request from a finite sample, input latency, or an approved change to state contract A1 |
| Product left behind | Frozen target-bit and bisector fixtures, exact-choice comparisons and, if triggered, the proposed amendment packet |

- **RTE.65 [P LQ18]** Proposed decision rule, DT-L1 Round 3 item 4, LX-9: step 1 compares 64-bit patterns of target
  coordinates for R2-E2's 13,600 requests on Windows CPython, Linux CPython and the LX-6 engine candidate. Step 2
  places at least 1,000 requests within one ulp of exactly computed bisectors, including those step 1 found to differ,
  and compares the exact key of the chosen rotation. If any chosen rotation differs between any two runtimes, the
  request is recorded as a dyadic Cayley vector read exactly and a drag compares its 60 candidates exactly in Q(√5).
  That amends the state contract: EN1's Astra check, then the owner. A difference in target bits alone is reported
  and feeds step 2. No difference is a sample result for the tested requests only, which the report says; the amendment
  stays open to the owner. This rule feeds the contract decision, not an automatic amendment.
- **RTE.66 [D]** Replaying the already recorded exact rotation or menu letter remains DET.33's different test. It
  does not recompute the chooser and cannot establish fresh-request agreement. LX-9 reads no labels off the lattice.
  Its R2-E2 dependency is named, and R3-M2 leaves LX-9 standing.

## 22. Source differences and limits

- **RTE.67 [D]** The following reconciliations delimit this chapter's use of its sources. Round 3 item 4 remains
  proposal under LQ18; correcting a figure or identifying a missing mechanism accepts no owner option.

| Source difference | Treatment and clauses |
| --- | --- |
| Partition LX-2 and DT-L1 E4 use 708,480 bytes as pose IDs; Round 3 distinguishes the full pose bundle | RTE.33 and RTE.42 use 869,952 bytes at P = 2,049 with one mask. The smaller figure remains the ID array alone. |
| Earlier E3/E4 and ENG-0J cite 672,417 as the long-word example; Round 3 R3-E4 gives longer literals and compositions | RTE.33 and RTE.55 retain the earlier regression and use 2,352,984 literal letters plus the 13,231,266-letter stream; 166,788,405 is length arithmetic. KN2.10 and LB5.13 already carry that correction. |
| The engine-language wiki proposes headless speed comparison; partition and Round 3 count speed only on the owner's machine | RTE.20 and RTE.57 keep oracle equality as source evidence and headless timing as a lead. The deferred proposal has no result. |
| Partition LX-7 calls GPU/CPU comparison headless; Round 3 requires an actual named device | RTE.60 and RTE.61 separate no-window device correctness from owner-machine gate measurements. CPU fixture preparation supplies no GPU result. |
| Round 3 LX-1 defines executed instructions; DET.20/DET.21 require a common logical meter | RTE.39 and RTE.40 keep separate counters. A mapping remains open; raw step counts cannot establish route-equivalent budgets. |
| KN7.9 specifies arrays plus exact keys; LG4.7 and DET.18/DET.24 use canonical-encoding language | RTE.34 uses KN7.9. The integrator must align the broader wording; render or JSON sizes do not determine logical memory. |
| Section 12's IR promise expressly covers RT1 to RT3; the task's common obligations also include RT4 | LG4.14 and RTE.6 require a separately admitted equivalent contract or common-IR lowering. No RT4 exemption or replacement validator is assumed. |
| B4-12 D6 records everyday conditions; the renderer gate also prescribes power and device conditions | RTE.36 and RTE.51 record conditions exactly and require the full gate for a gate claim. A differently labelled condition cannot replace it. |
| Older experiment documents and the wiki carry a schedule; LQ19 also proposes moving part of the charter exit | RTE.32 keeps the current whole exit and the 3 November schedule hold. Time boxes await the owner; no proposed charter amendment is treated as accepted. |

## 23. Errors

- **RTE.68 [D]** This chapter adds no error codes; chapter 13 owns the registry used by candidate checks and refusals.

## 24. Conformance tests

- **RTE.69 [D]** These are requirements on route candidates and spike records under overview 3.4, collected by phase
  in chapter 12. No conformance test in this table exists yet. A document structure check supplies none of these
  results. Checks involving an open mechanism use its selected, versioned contract and cannot decide it by themselves.

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
| CT-RTE-16 | Each LX-7 stage's three cold W3 runs independently satisfy the whole unrelaxed selection gate; the report includes its built-in-stage comparison | performance |
| CT-RTE-17 | LX-7 names the actual GPU device, frozen exact inputs, error metric and singular exclusions; device output agrees with CPU and exact/interval expectations without claiming chart-property proof | windows |
| CT-RTE-18 | LX-8 has the owner-set protocol and equivalent IR corpus before testing non-authors; its outcome is reported only under that protocol | human |
| CT-RTE-19 | LX-9 retains 13,600 target-bit comparisons and at least 1,000 near-bisector exact-choice comparisons; target-bit-only differences continue to step 2 and rotation differences produce an unaccepted amendment packet | synthetic |
| CT-RTE-20 | Under deferred spikes, the freeze packet explicitly checks every route and both flow forms against its interfaces, data/provenance carriers and metering obligations; unresolved gaps are not marked verified | source |

## 25. Open items

All LQ1 to LQ21 remain open. Their contact with this chapter is listed here without answering them.

| Owner question | Clauses and dependency |
| --- | --- |
| LQ1 Language/profile arrangement | RTE.2 to RTE.6, RTE.24 to RTE.27, RTE.44. A separate research language changes adapter and corpus coverage; common identity, exactness and commit boundaries stay fixed. |
| LQ2 Joint boundary setting | RTE.24 to RTE.27, RTE.45. Chapter 07 supplies author classes, declassifiers, display sinks and the policy identity for frozen refusal corpora; no solving preview is enabled by this chapter. |
| LQ3 Evidence model | RTE.3, RTE.29, RTE.46. Chapter 06 and chapter 05 supply canonical evidence output; the data-freeze dependency changes with LQ10's facts/judgements choice. |
| LQ4 Branches and pinned jobs | RTE.2, RTE.21, RTE.22. Chapter 08 owns their lifecycles; either finish/cancel option keeps results on their original immutable inputs and permits no program commit. |
| LQ5 Built-in implementation | RTE.8 to RTE.16, RTE.30, RTE.38. The three-tool experiment must identify the candidate implementation; native/per-tool alternatives change its path, not its kernel obligations. |
| LQ6 Views | RTE.22, RTE.59 to RTE.61. Any admitted research-view workload needs its prescribed gate evidence; no extra product view or interface design is chosen here. |
| LQ7 Charts | RTE.29, RTE.59 to RTE.61. Generated subset, built-in-only and shader-language alternatives retain their separate admission and evidence needs from chapter 10. |
| LQ8 Engine language | RTE.17 to RTE.20, RTE.54 to RTE.58. Choose after arithmetic/engine evidence, Rust now, or the exact Python/NumPy reference; no spike has resolved the choice. |
| LQ9 Route and placement | RTE.4 to RTE.23, RTE.29 to RTE.31, RTE.38 to RTE.58. RT1 to RT4 and all three placements remain candidates subject to the common obligations and shell fault condition. |
| LQ10 Data core and freeze | RTE.3, RTE.26, RTE.29 to RTE.34. Chapter 05 owns formats, word encoding and lossless provenance under both flow forms; no syntax or IR is frozen here. |
| LQ11 Recording | RTE.2, RTE.3, RTE.6. A recording uses chapter 04's command semantics and saved meaning; no route adds unattended commit. |
| LQ12 Sharing | RTE.3, RTE.11, RTE.35 to RTE.37. Module distribution choices supply no program network/process grant; candidate dependencies and licences still need recorded review. |
| LQ13 Syntax | RTE.9, RTE.29, RTE.62, RTE.63. LX-8 awaits the owner's human protocol, or the owner chooses now; the same semantic contract remains required. |
| LQ14 Learning gates | RTE.27, RTE.44 to RTE.46, RTE.63. Any competence-dependent permission changes chapter 07's policy and allowed-tool corpus; neither LX-4 nor LX-8 settles it. |
| LQ15 Formal proofs | RTE.28. No assistant, Lean 4 research language or exported obligations remain options; proof protocols and adapters are not supplied. |
| LQ16 Product cut | RTE.2, RTE.30, RTE.38, RTE.44. Experiment coverage does not decide which tools ship or reopen the research scope. |
| LQ17 Name | RTE.62, RTE.63. C600 Script remains a working name; no spelling, extension or concrete syntax is selected. |
| LQ18 Spike allowance | RTE.1, RTE.29 to RTE.31, RTE.35 to RTE.65. Every card, time box and decision rule remains proposed; no experiment is run by this chapter. |
| LQ19 Freeze order | RTE.29 to RTE.32. Chapter 12 applies the current charter exit unless the owner accepts an amendment; no earlier schedule or gate waiver is implied. |
| LQ20 Renderer handoff | RTE.29, RTE.59 to RTE.61. Chapter 10 owns the primitives and generated-stage handoff before renderer freeze; the no-handoff alternative fits the frozen feature list. |
| LQ21 Flow-control form | RTE.4, RTE.12 to RTE.16, RTE.24 to RTE.27, RTE.39, RTE.45, RTE.46. Static-only labels, additional run-time labels and capability separation have different costs and refusal sets; LX-1/LX-3 evidence is absent. |

Open mechanisms and dependencies:

- **RT4 admission [O], RTE.6.** LP1 and LQ9 choose common-IR lowering or an equally versioned host contract with an
  equivalent validator and compatibility proof. No source supplies that validator or its accepted subset.
- **Bindings and zero-copy access [O], RTE.7.** LP1, LQ8 and LQ9 settle ABI, immutable buffer lifetimes, transport or
  read-only sharing and cross-host exact-value transfer. No size table demonstrates an implemented binding.
- **Engine choice [O], RTE.17.** LX-5 and LX-6 or an explicit owner choice decide; the wiki proposal remains deferred.
- **Placement [O], RTE.21.** LX-2 and LQ9 decide containment. A process name alone supplies no pass.
- **Flow mechanism [O], RTE.24.** LQ21 decides the form before checker/IR construction; its policy stays separate.
- **Policy and sinks [O], RTE.27.** LQ2 and chapter 07 decide enabled flows, classes and exceptions; LX-3 cannot freeze
  an authoritative corpus without naming the policy it tests.
- **Candidate-specific facts [O], RTE.11.** The named RT2 candidates have no verified licence, speed, sandbox or
  determinism comparison here. Their scoped evidence and any dependency approval remain outstanding.
- **Metering [O], RTE.40.** LG4.8 and DET.20 to DET.28 leave abstract charges, mapping, logical resource lifetimes,
  output units and recursion rules to LP1. Instruction timing is a separate lead. Numeric presets are not selected.
- **Human protocol [O], RTE.63.** The owner fixes LX-8's participants, tasks, samples, stopping and decision rules.
- **Shared fixtures and byte profiles, RTE.6, RTE.35 to RTE.46.** LP1, chapters 02 and 05 and chapter 13 must supply
  the validator, canonical result encoding, source-map convention, exact error codes and digest-frozen corpora before
  the corresponding runs. Chapter 04's EV1.10/EV2.12 coordinate and stepping choices remain its dependencies.
- **EN3 and EN4, RTE.2, RTE.21, RTE.29.** Chapter 01's final batch/query rows, program/view lanes, availability,
  undo during jobs and provenance-bearing proposals are still handoffs, not delivered runtime interfaces.
- **Renderer precision and measurement, RTE.36, RTE.51, RTE.59 to RTE.61.** RPI.36's metric and singular exclusions,
  selected candidate/device, frozen W3 inputs, owner-pinned PresentMon and recorded conditions precede judgement.
  Charter E3's 3 s remains proposed. No installation or renderer selection is authorized here.
- **Source wording, RTE.34 and RTE.67.** The pose payload, old long-word example, wiki timing venue, GPU/headless
  wording and instruction/logical-meter distinction are reconciled explicitly. KN7.9's precise logical-memory unit
  still needs alignment in the broader LG4.7 and DET.18/DET.24 wording; no other chapter is edited here.
- **DT-L1 obligations, RTE.12 to RTE.16, RTE.24 to RTE.27, RTE.45, RTE.61, RTE.64 to RTE.66.** Uses of R2-E2 and
  R2-E3 are named. Round 3 cleared the obligations (overview section 8), and RTE.45 carries its added cases.
- **Content review and phase collection, RTE.69.** The integrator and Astra still review this candidate's content;
  chapter 12 collects conformance by phase and chapter 13 owns final error spelling. Passing a structure check proves
  neither content acceptance nor any experiment result.

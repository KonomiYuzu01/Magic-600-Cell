# Chapter 09: determinism, budgets and sandbox

Status: **draft for the owner. Not accepted.** Conventions are in [the overview](00-overview.md), section 3.

This chapter defines the identity of a reproducible run and the limits on its execution. It uses `Budget`, `JobResult`,
snapshots and kernel costs from [chapter 01](01-kernel.md). Chapter 02 owns types, effects and the IR; chapter 05 owns
stored bytes and `RunRecord`; chapter 07 owns profiles and boundary policy. No runtime, generator algorithm or budget
preset is selected here.

Sources: [the partition](../ecosystem-partition.md), especially INV-4, INV-9, INV-10, sections 8.4, 9, 11 and 12 and
defaults 2 and 6; [chapter 01](01-kernel.md), especially KN2.15, KN3.4 and KN7;
[DT-L1](../../../../progress/1.0/design-table/topics/language-ecosystem.md), cards E3, E4, R2-E2 to R2-E4 and Fix first;
[restoration](../../../../../research/jumbling/restoration/README.md) and the stage
[13](../../../../../research/jumbling/restoration/13-tutorial-runner/README.md),
[14](../../../../../research/jumbling/restoration/14-blocking-structure/README.md),
[15](../../../../../research/jumbling/restoration/15-lowering-machine/README.md),
[17](../../../../../research/jumbling/restoration/17-routing-in-driver/README.md),
[18](../../../../../research/jumbling/restoration/18-tail-completion/README.md) and
[19](../../../../../research/jumbling/restoration/19-height-depth-framework/README.md) READMEs;
[owner, 2 October](../../../../wiki/decisions/owner-decisions-2026-10-02.md); and
[requirements R-03, R-08, R-14 and R-16](../../../../progress/1.0/requirements-from-screening.md).

## 1. The reproducibility key

- **DET.1 [D]** `ReproKey` names the complete fixed input of a run, as partition 8.4 requires. Its fields below are
  semantic fields. Chapter 05 defines their canonical encoding and digest; chapter 02 defines edition, module and IR
  identities. A record containing a key also retains, or resolves by digest, the bytes needed to use it.

  ```text
  ReproKey = {
    source:            source digests
    modules:           resolved module digests
    edition:           language edition
    ir_version:        typed IR version
    identity:          IdentityBlock
    input:             Digest<level>
    parameters:        typed parameter values and further immutable input digests
    seeds:             sealed seeds, named streams and generator algorithm identities
    budgets:           deterministic fields of Budget, with their unit identities
    kernel_cost_table: declared cost table revision and digest
    kernel_api:        API revision and identity of the exact kernel implementation it exposes
    checkers:          identities of the checkers on which the result relies
  }
  ```

  The fields have these suppliers and purposes. The last rows are justified separately in DET.2 and DET.3.

| Field | Supplied by | Why it is part of the key |
| --- | --- | --- |
| `source` | The loader and checker, from the checked source bytes | An edit can change the program. Physical source locations do not identify it. |
| `modules` | The loader, from the manifest and lockfile resolution of chapter 02 | The complete executed dependency closure, including libraries, can change the result. A module name alone cannot pin it. |
| `edition` | The source declaration, validated by the checker | The same source can have different meanings in different editions. |
| `ir_version` | The checker and executor | The operation meanings and their metering contract are versioned. |
| `identity` | The kernel registry, checked against the input | KN0.1 supplies model identity and, for jumbling, menu identity, menu-order digest and contract revision. An order-free menu identity cannot interpret indexed menu letters. |
| `input` | The engine or the validated import, from the pinned snapshot or derived input | The digest names the puzzle and level through KN1; a label digest cannot stand in for a pose digest. |
| `parameters` | The caller, validated by the checker and host | Goals, bounds, alphabet, protection inputs and other consulted data affect the result. Effective grants and policy inputs are covered by DET.4. |
| `seeds` | The caller and the runtime's generator declaration, sealed before execution | Random choices depend on the seed, stream assignment and algorithm. |
| `budgets` | The host, from the explicit limits accepted for this invocation | A different deterministic limit can stop at a different operation. `safety_stop` is excluded by KN7.7. |
| `kernel_cost_table` | The kernel | A cost change can change a cut-off even when the mathematical answer is unchanged. |
| `kernel_api` | The kernel's versioned declaration | Matching signatures alone do not identify the engine semantics used to compute the answer. |
| `checkers` | KN6, with each checker's source digest | A different checker can produce a different verdict or trace for the same certificate. |

- **DET.2 [D]** The declared cost table revision is an addition to partition 8.4's list. It follows from INV-10 and
  KN7.8: if one kernel revision charges differently, the same remaining budget can admit a call in one run and refuse it
  in another. Every table used by the run is pinned before execution. Changing a declared charge changes the key.
- **DET.3 [D]** Kernel API and checker identities are a second addition to partition 8.4's list. They follow from
  INV-10: a key that ignores the engine cannot promise the same bits. The kernel identity covers the exact algorithms
  and dependencies exposed under that API revision; a behavioural change cannot retain the identity. KN6.3 supplies a
  checker's source digest. The host pins the available checker set before sealing the key; a run cannot use an
  unrecorded checker. A replacement implementation establishes agreement before claiming an existing identity.
  Platform executable paths and executable container metadata do not identify these semantics.
- **DET.4 [D]** Complete inputs follow from INV-10's ban on ambient authority. Every datum a program can consult is
  fixed by the key: directly, through the module closure, or through `parameters` and its immutable input digests. This
  includes consulted workspace records, file contents, solver picks, provenance, the effective `Profile`, grants and
  policy revision from chapter 07. Reading a snapshot's puzzle digest does not pin its workspace or protection data.
  Such data are captured separately when used. A later live read is a new input and starts a new run.
- **DET.5 [B]** For an admitted run that completes or reaches a deterministic cut-off, the same key gives the same
  exact result bytes on Windows and on Linux (INV-9, INV-10, partition 8.4). Equality means byte equality, including a
  cut-off's coverage and stop position. Equality of a final state digest alone is insufficient.
- **DET.6 [D]** The promise of DET.5 concerns the exact payload, following INV-4 and partition 8.4's separate display
  precision rule. This covers the outcome, stable error code and exact context if any, input identities, coverage,
  granted and used deterministic budgets, exact returned values, literal words, certificates and checker reports or
  traces included in the result. Evidence output uses the versioned rules of chapters 05 and 06. Approximate display
  coordinates are covered by section 4. Wall-clock time, machine, user name, physical paths and locale are absent from
  the key and exact payload. Timings, diagnostics, transport framing, compression container metadata, per-launch
  handles, job IDs, preview IDs and command receipts belong to the host envelope. They cannot influence computation.
  KN0.6 still attaches the source revision to a delivered result; such session-local references do not replace its
  immutable input digest. Reproduction does not promise the same host envelope or repeat a workspace command, file
  publication or commit.
- **DET.7 [D]** Loading and reproduction validate the key's identities and input digests before execution, following
  INV-7, INV-10 and default 13. Missing bytes yield `not run`, with a typed reason. A mismatch yields a refusal. Neither
  substitutes a current module, current snapshot, nearby file or newer checker. Replaying a record grants no authority
  to write; any external action still needs the current host grant and command gates (INV-2, INV-3).

  ```text
  operation reproduce(record: RunRecord) -> JobResult | not_run(reason: Error) | refused(Error)
    effect:  read a snapshot; seeded randomness; use a granted file, as required by the recorded computation
    cost:    the recorded deterministic budget under its recorded cost table
    errors:  LG4:input.repro_input_missing, LG4:input.repro_key_mismatch, LG3:capability.grant_mismatch,
             KN7:budget.*, LG4:evidence.reproducibility_mismatch
  ```

## 2. Deterministic evaluation

- **DET.8 [D]** One run has one execution thread, following partition 8.4's builder rule and INV-10. This includes the
  kernel computation and numerical library work performed for that run. No parallel reduction or race chooses a
  result. The host can execute independent runs concurrently, but changing their relative speed, admission order or
  completion order changes no admitted run's exact payload. Admission refusals remain KN7's host results, outside
  DET.5's promise for an admitted run.
- **DET.9 [D]** Traversal and tie breaking use canonical keys, following INV-6 and INV-10. Collection order never comes
  from addresses, hash-table layout, allocator state or a host locale. An unordered collection is ordered by its
  declared canonical key before iteration can affect a result, random draw, charge, coverage or output order. A tie has
  an explicit secondary key. The pinned menu order applies to menu letters; a host sort never replaces it.
- **DET.10 [B]** A run reads no ambient clock, file, random source, network, process state or other input (INV-10,
  default 2). Environment variables, working directory, directory enumeration and host module discovery provide no
  program input. A granted resource is mediated as in section 5. A clock grant creates a non-reproducible run; it does
  not extend DET.5 to time-dependent answers.
- **DET.11 [D]** Seeded randomness is deterministic input, following INV-10 and LG3. Before execution the record seals
  each seed, its named stream, the generator algorithm and revision, and any stream-derivation rule. No seed comes from
  the clock or host entropy during execution. The generator's reference vectors cover initialization and draws; library
  operations that consume draws have a pinned order. A child receives its recorded stream, rather than a stream chosen
  by scheduling.
- **DET.12 [O]** The generator algorithm and the stream-derivation rule have no selected source. LP1's language and
  library specifications choose and publish them, with reference vectors. Options include a stateful generator with
  a recorded draw order and a generator addressed by stream and counter. Both retain DET.11; the algorithm identities,
  stream mapping and vectors change. Naming a host's default random function does not settle this item.
- **DET.13 [B]** Integer decisions use KN3's exact integers and explicit size-based costs (INV-4, KN3.1, KN3.2,
  KN3.6). Host word size cannot cause wraparound or silent narrowing. Typed fixed-width data follows KN1's byte
  conventions and chapter 02's range rules. Approximate values never convert implicitly into exact decisions.
- **DET.14 [O]** Text equality, normalization, case handling and ordering need the language contract in chapter 02 and
  byte rules in chapter 05. The sources specify UTF-8 source but select no Unicode comparison algorithm. Options are
  scalar-value equality and order without implicit normalization, or explicitly versioned normalization and comparison
  tables. Under either option there is no locale dependence, invalid input has a typed rejection, and any Unicode
  tables affecting a result are pinned. LX-3 checks the selected rules with non-ASCII and normalization-sensitive data.
- **DET.15 [D]** Content digests identify artifacts, and hashed sources and assets use pinned bytes (default 13,
  R-03). Chapter 05 owns normalization; a host text reader cannot change the bytes being hashed. A granted file's
  physical path serves only the host's resource lookup. Program-visible resource names are logical names. Case-only
  file-name differences neither merge distinct contents nor create a platform-specific import order; the importer
  resolves records by digest as default 13 requires.
- **DET.16 [D]** Cache state and value-store eviction are unobservable to the run, following KN2.15 and KN7.8. A cache
  hit and a cold computation return the same value and consume the same declared charge. Recomputing an evicted value
  adds no charge. Physical sharing or another run's residency cannot change the logical memory charge. KN6.2's
  independent checkers continue to recompute without producer caches.
- **DET.17 [B]** Deterministic cut-off is reproducible; solver cancellation and a wall-clock safety stop are not
  (INV-10, KN7.6, KN7.10). Cooperative cancellation retains its actual stop reason and coverage. A replay with the same
  key and no cancellation need not stop where the cancelled run did. Host exhaustion or runtime death is an error
  under KN7.13, and supplies no promise about an identical failure position on another machine.

## 3. Budgets and accounting

- **DET.18 [B]** Every job has an explicit deterministic `Budget` (default 6, KN7.7). This chapter uses its four
  deterministic fields and its separate optional safety stop unchanged. Seconds are a display unit. CPU seconds,
  elapsed seconds and a VM's measured speed cannot supply a deterministic limit.

| Field from KN7.7 | Unit and charge | Independent of |
| --- | --- | --- |
| `steps` | Abstract interpreter work under the versioned metering contract of DET.20 | Native instruction count, optimization and processor speed |
| `kernel_cost` | The exact integer cost each admitted kernel call declares from its inputs, under the pinned cost table | Cache hits, eviction and transport cost |
| `memory` | KN7.9's logical units: array bytes in the KN1 digest convention, plus each pose entry's canonical exact-key length | Resident memory, pointer size, allocator layout, stored text form and physical sharing |
| `output` | Logical size of results, records and plans; the precise counting rule is open in DET.24 | Compression ratio, terminal speed and transport packet sizes |
| `safety_stop` | Optional host deadline, outside the deterministic key | No exact replay promise applies when it stops a run |

- **DET.19 [D]** A budget record identifies its units, following LG2 and INV-10 (DT-L1 R2-E4). Length and puzzle cost
  retain their alphabet: signed retained generators, jumbling menu letters and UI actions are different quantities.
  A kernel cost table states how these typed inputs and operand bit lengths produce its declared integer charge.
  Interpreter steps and declared kernel work are separate counters. A change of spelling cannot convert one alphabet
  or budget dimension into another.
- **DET.20 [O]** LP1's metering specification, shared with chapter 02, defines an abstract step and its charge points.
  The sources do not choose between one charge per IR operation and a fixed, versioned weight table over IR operations.
  Either includes loop tests, calls, pointwise contracts and applicable run-time flow checks; none can perform
  unbounded work between charges. Pointwise contracts are charged under LG4 (DT-L1 R2-E4). Whole-state contracts keep
  LG4's once-per-revision or agreement-test rule. The specification also fixes exhaustion boundaries, equal-to-limit
  admission and precedence when several limits would fail together. Both options preserve DET.5; their tables and
  recorded stop positions differ. Tests at each boundary become part of the IR version's conformance fixtures.
- **DET.21 [D]** RT1 to RT3 claiming the same key use the same abstract charges, following the typed IR contract in
  partition section 12 and INV-10. RT1 meters the abstract execution; RT2 must expose equivalent metering for its
  admitted language; RT3 preserves the abstract charge points through compilation. Optimized code cannot delete
  charges that change a cut-off. Borrowed VM instructions or fuel are enforcement tools only after their mapping to
  the abstract charges is established. KN7.8's kernel charges stay the same in every route.
- **DET.22 [O]** Whether RT2 can provide that mapping, and how RT3 preserves it, awaits LX-1, LX-3 and LQ9 in chapter
  11. No source establishes equal step budgets across those executors. RT4 needs an admitted subset and a specified
  meter before it can make an equivalent promise. If a route cannot preserve the common contract, its budgeted results
  cannot claim the same key. The choices are to supply the meter, decline that route, or propose a different versioned
  execution contract for the owner's LQ9 decision. A different contract changes keys and needs chapter 05's explicit
  compatibility treatment; it cannot silently change saved runs.
- **DET.23 [D]** Kernel costs are declared before admission and depend only on inputs, as KN7.8 requires. The run
  accounts for each logical call once, including nested library calls that reach it. Eviction, internal retries used
  only to reconstruct values and IPC do not buy or consume extra kernel work. Chunking retains the same literal-letter
  work and total declared cost. Cancellation boundaries keep KN7.6's declared batch chunks; they do not define a new
  deterministic budget in seconds.
- **DET.24 [O]** LP1's accounting specification fixes logical memory lifetimes and output units. KN7.9 fixes the
  logical unit, including pinned snapshots and derived states, but leaves alias accounting and release points
  unstated. Options are deterministic logical ownership or a conservative lifetime extending to a declared scope;
  host garbage collection cannot decide exhaustion. For output the choices include canonical uncompressed byte size
  or another explicitly versioned logical-size table. Both cover results, records, traces and plans, including output
  retained before a cut-off. The specification defines peak live memory, cumulative output, framing overhead and the
  bounded space needed to emit a terminal result when an output budget ends. Both options preserve cache-independent
  charges and produce the same stop on repeated runs under their own recorded contract.
- **DET.25 [D]** Work performed on behalf of a run remains within that run's limits, following the explicit per-job
  budget of default 6 and INV-10. A call, recursive call, checker invocation or child job cannot reset its parent's
  counters or mint an allowance. A parent that uses child results records their budgets and coverage. A genuinely
  separate, host-authorized run has its own key; consuming its result makes that immutable result a declared input.
- **DET.26 [O]** The sources do not choose hierarchical accounting. LP1 chooses between serial children sharing a
  meter and disjoint child allowances reserved deterministically from the parent before launch. Both conserve steps,
  kernel work and output, count aggregate live logical memory, and record the child order or reservations. Parallel
  child completion cannot select a winner or redistribute unused allowance by timing. Under reservations, memory
  stays charged until a fixed logical join, and allocation, return and aggregation rules are fixed inputs. Under a
  shared meter, the logical execution order is fixed. LX-3 compares the parent and child ledgers. This item does not
  change R-16's admission lanes.
- **DET.27 [D]** A run starts only with finite, explicit deterministic limits accepted for it, following default 6.
  The host exposes the granted limits, units, cost table identity and amount used as data. The exact expanded length
  and puzzle cost of a word are available before a run (KN5.10). Presets resolve to concrete limits in the record;
  neither a machine-speed estimate nor a later preset edit changes an existing key. A program that reads the amount
  used after reading state reads a state-derived value (BND.44).
- **DET.28 [O]** No source supplies numeric presets, profile ceilings or a logical recursion rule. LP1 specifies them
  with chapter 02 and the host; chapter 07 adds no capability-dependent ceiling (BND.62). Recursion can use a separate
  logical depth guard or a bounded logical stack charged to memory. Either records its limit and counting rule in the
  run's parameters, retains step metering, and refuses before the host stack fails. Preset values and any overhead
  figures require evidence under overview 3.5; none is measured here. A host may impose a lower physical safety limit,
  whose exhaustion ends the run as an incomplete host error.
- **DET.29 [D]** The run record retains the resolved budget, unit and metering revisions, used counters, child ledger
  if present, deterministic stop boundary, coverage and actual stop reason, following partition PS1, 8.4 and KN7.3.
  Reproduction starts from the original inputs and counters under the recorded contract. A continuation after a
  cut-off records new limits and a new key; it never rewrites the old run as completed. A changed deterministic
  accounting rule changes its version identity and key.
- **DET.30 [B]** `Empty`, `not found within budget` and `not run` are distinct results (default 6, KN7.11). `Empty`
  describes a completed enumeration of its declared set. `not found within budget` records that the deterministic
  budget ended without a checked witness; it retains the covered set and is open for the unexamined part. `not run`
  has no examined set. A job that did not complete yields no negative claim, and a witness already found counts once
  checked (KN7.4). Only a deterministic budget gives coverage a reproducible scope. A clock-stopped run reads open;
  cancellation and errors also establish no absence. Chapter 06 owns claims and display words under every LQ3 option.

## 4. Display precision and the input map

- **DET.31 [B]** Exact decisions remain exact at every display boundary (INV-4, KN3.1). A display coordinate or a
  comparison of approximate coordinates cannot decide legality, identity, protection, equality or evidence. Chapter 10
  owns `DisplayPlan` and its connection to canonical IDs.
- **DET.32 [D]** Display plans depend on the maths library and are compared at their declared precision, as partition
  8.4 adopts from DT-L1 R2-E2. Their exact fields still obey DET.5: object identities, ordering, source references and
  exact declared properties. Approximate fields carry chapter 10's precision and domain contract. LX-3 compares those
  fields across platforms at that precision. It neither requires identical float bits nor uses tolerances to weaken
  an exact field. The precision contract is pinned with the plan's producing module and inputs.
- **DET.33 [B]** The A1 input map's float request is the site defined by KN3.4 and state contract A1. A run consuming
  such input records the chosen exact rotation, with its identity block and base input, before using it as an exact
  input. A menu-mode choice records the actual menu letter under its menu identity and order (INV-4). Replay uses the
  recorded choice. It never recomputes trigonometry, nearest candidates or the choice from a recorded drag.
- **DET.34 [D]** Fresh input-map choice remains an LX-9 obligation under KN3.4 (DT-L1 R2-E2). Replaying a recorded exact
  choice does not establish cross-platform agreement for fresh float requests. If LX-9 finds differing choices, KN3.4
  gives the proposed dyadic Cayley request and exact menu-candidate comparison; EN1's Astra check and then the owner
  decide the contract amendment. This chapter uses no assumed outcome, no new rounding rule and no off-lattice visible
  pose equivalence. Chapter 11 owns the experiment and its decision rule.

## 5. Sandbox and grants

- **DET.35 [B]** The runtime has only the interfaces in partition section 9 (INV-1 to INV-3). A snapshot is read-only;
  derived states remain kernel values; view and workspace changes use command rows. Programs can prepare proposals
  only where the boundary permits. They cannot open session files, write engine state, obtain a shell token or commit
  in any profile or placement. Static approval never replaces the engine's gates.
- **DET.36 [D]** The sandbox rules apply in an engine isolate, a separate runtime process and the shell process,
  following partition section 9, INV-10 and KN7.13. They also apply in every implementation route. The host mediates
  resource access, meters steps, kernel work, logical memory and output, and enforces the recursion bound of DET.28.
  Host-language imports, foreign calls, reflection or dynamic code cannot supply an ungranted access path. A route
  whose admitted subset cannot enforce these rules remains unverified; placement inside a trusted process supplies
  no extra authority. The approved-subset variant and approval bound to executed bytes are DT-L1 R2-E3 dependencies.
- **DET.37 [D]** A module declares capabilities and the host grants them per profile and per run (LG3, BM3). A grant is
  scoped to the operation and resource; a callee or child receives no broader grant than the caller can pass. The
  following table separates fixed sandbox rules from chapter 07's policy. `grant` means an explicit scoped host grant,
  subject to declared effects; it supplies no ambient access. Chapter 07 section 8 fills the boundary-sensitive rows, and
  it splits the solving column by author class.

| Capability | Solving | Tool authoring | Research |
| --- | --- | --- | --- |
| Snapshot read | chapter 07 section 8 | chapter 07 section 8 | grant for a pinned input; rows as in chapter 07 section 8 |
| Derived state | chapter 07 section 8 | chapter 07 section 8 | grant for immutable kernel values |
| View change | chapter 07 section 8 | chapter 07 section 8 | chapter 07 section 8 |
| Workspace change | chapter 07 section 8 | chapter 07 section 8 | chapter 07 section 8 |
| Prepare a preview | chapter 07 section 8 | chapter 07 section 8 | chapter 07 section 8; interim solving-session transfer barred |
| Granted file | grant with named resource and rights | grant with named resource and rights | grant with named resource and rights |
| Clock | explicit grant; non-reproducible | explicit grant; non-reproducible | explicit grant; non-reproducible |
| Seeded randomness | grant with sealed seed and pinned generator | grant with sealed seed and pinned generator | grant with sealed seed and pinned generator |
| Network | no | no | no |
| Process | no | no | no |

- **DET.38 [D]** The table does not answer LQ2, LQ14 or LQ21. Chapter 07 supplies `Profile`, provenance, boundary classes
  and declassifiers. Under flow labels a grant is checked with the permitted data and control flows; under capability
  separation, reading state-derived data and preparing previews cannot coexist except through policy-enabled
  declassifiers, and preview outcomes are state-derived (DT-L1 R2-E3). Until the joint setting, the partition's section
  11 interim reading applies: research output never becomes a solving-session preview, solver-tool solving previews
  remain disabled, and built-in tools keep the 2 October boundary and the 17 October exception. A file grant preserves
  stored provenance; it cannot launder a word through storage.
- **DET.39 [P LQ1]** With the recommended single language, solving, tool authoring and research use the same types,
  exact semantics and key format; profiles change grants and labels. If research uses a separate language, its engine
  adapter and imported evidence meet the same identity, input and checking requirements before claiming reproduction;
  this chapter does not confer sandbox guarantees on an external notebook. With a product-only language, the research
  column describes no shipped language profile and research stays outside it. The product's sandbox, exactness and
  no-network/no-process rules stay the same under all alternatives.
- **DET.40 [D]** The run record stores the effective grant set, following INV-10, LG3 and partition PS1. For each grant
  it records the capability kind, rights, logical resource scope, grantee module digest, effective profile and the
  policy identity or decision used by the host. File inputs name captured content digests; randomness names DET.11's
  seed and algorithm; a clock grant marks the run non-reproducible. These descriptors are fixed parameters under
  DET.4. OS handles and physical paths remain host lookup data. A saved descriptor is an audit fact, never a bearer
  grant that a program can redeem or widen.
- **DET.41 [D]** A reproducible file read consumes captured, digest-checked bytes, following INV-10, default 13 and
  R-08. Mutable files are captured before the key is sealed; changing them during capture invalidates admission.
  Directory contents, file timestamps and the existence of another output are unavailable as implicit inputs. A
  granted clock read or uncaptured mutable input puts the run outside DET.5; it cannot produce reproducible negative
  evidence. For an evidence builder, such access is refused (section 6).
- **DET.42 [B]** Network and process access are `no` in every profile in 1.0 (default 2, INV-10). A granted file cannot
  enable either. The host's placement, isolated-interpreter startup and engine process ownership do not give programs
  a process capability. Local module lookup remains digest-pinned under every LQ12 option; a possible later registry
  is no present network permission.
- **DET.43 [D]** A runtime fault ends its jobs as `error` under KN7.13. The fault itself changes neither journal nor
  state hash, and the shell stays responsive. Any previously committed engine command keeps its receipt under KN5.22;
  a fault never rewrites a committed outcome as failure. R-14 keeps engine work off the UI thread and R-16 retains atomic
  admission, including concurrent runs and held keys. Logical exhaustion is reported with its budget name; physical
  exhaustion and a killed runtime remain incomplete errors. LX-2 owns the placement fault experiment and responsiveness
  measurement. This chapter asserts no measured containment or latency result.

## 6. Runs that produce evidence

- **DET.44 [D]** A language evidence builder uses sealed seeds, no clock, one thread and an isolated interpreter, as
  partition 8.4 requires. Its dependency set and inputs are fixed by `ReproKey`; host startup excludes user-site modules
  and ambient interpreter configuration. The restoration README's isolated Python commands and stages 14, 17 and 18's
  single-thread settings are source precedents. They do not select Python as the language executor.
- **DET.45 [D]** Builder outputs and scratch data stay outside the source tree, under a scoped host output grant
  (partition 8.4; restoration stages 14, 15, 17 and 18). The output location does not enter the key or select a branch of
  computation. The builder emits bounded output through the host; it gains no general filesystem or process access.
  Checkers remain independent under KN6.2.
- **DET.46 [D]** Builder caches are keyed by all inputs needed to validate their contents and fail closed on a
  mismatch (partition 8.4, R-08). The host checks recorded inputs against the actual captured inputs after construction
  before reuse or publication. A mismatched cache value never reaches the computation as a valid value. Rebuilding
  from verified inputs treats the rejected entry as a miss and follows DET.16's unchanged charge. The cache diagnostic
  belongs to the host envelope. A cache cannot change the search order, random stream, termination reason or evidence.
  The final checked result is compared with the canonical input identities even when intermediate data were cached.
- **DET.47 [D]** Builder publication uses atomic writes, following partition 8.4 and the stage 13 README's atomic run
  records. Chapter 05 owns storage and recovery details. Interruption exposes a complete previous artifact or a
  complete new artifact; a partial output is never a completed certificate. Digests identify the canonical content,
  separately from stored compression bytes, as the stage 19 README distinguishes. Compression metadata supplies no
  input to exact results or evidence.
- **DET.48 [D]** These are requirements on new language evidence runs, not a claim that every archived research
  producer already satisfies them. Stage 13's README, under Clocks, retains child deadlines and a fixed CPU cap;
  stages 14 and 15 also describe timed searches. Those historical stops cannot supply DET.5's repeatable coverage.
  Stage 18 describes a fixed-seed, fixed-order construction with letter budgets, while stage 19 omits executable
  producers and complete histories. An import retains those limits as recorded facts through chapters 05 and 06. A
  checked archived witness can still count under KN7.4. The archive supplies neither a new run nor a portable replay
  result, and this chapter starts no research.

## 7. What LX-3 checks

- **DET.49 [D]** LX-3 exercises DET.1 to DET.30 and DET.35 to DET.42 for each candidate route, placement and admitted
  profile, following partition section 12 and DT-L1 E4. It repeats exact runs on Windows and headless Linux, varies
  cache residency, eviction, concurrent load and physical locations, and compares exact payload bytes, used counters,
  coverage and stop boundaries. It tests each budget alone and interacting limits, denied ambient access and the
  approved-subset variants. Resource enforcement in a host and equality between routes are separate assertions.
- **DET.50 [D]** LX-3 also compares the approximate display fields of DET.32 at chapter 10's declared precision and
  checks refusal and false-refusal cases under the forms of flow control (partition LX-3, DT-L1 R2-E2, R2-E3). Its
  must-refuse corpus includes selection by largest cap value, search followed by rebuilding from base parts, the F2 and
  F5 correction solve, workspace laundering, a state-based choice between two words, flow through a module call, a
  stop-selected preview, refusal-driven selection, a one-item pick and accumulation across runs (BND.52).
  The host-subset variant refuses dynamic code and binds static approval to the executed bytes. Counts of harmless
  tools refused are returned to LQ21; this chapter sets no acceptable count or policy ruling.
- **DET.51 [D]** Chapter 11 owns LX-3's decision rule, experiment card and relation to LQ9, LQ18 and LQ21. No LX-3 or
  conformance result exists yet (overview 3.4; partition section 12). Exact portability, repeatable cut-offs, sandbox
  containment, route-equivalent metering and display agreement remain unverified. LX-1 supplies cost leads; LX-2 tests
  placement faults; LX-9 tests fresh input-map choice. None is replaced by a document structure check. Windows-first
  product scope stays the 2 October decision: headless Linux checks of platform-independent layers promise no Linux
  product build, native input, DirectX, long-session or performance result.

## 8. Added errors

- **DET.52 [D]** The following codes extend default 5's typed categories. They distinguish admission refusals,
  deterministic cut-offs and incomplete host errors; no English diagnostic is used as a discriminator. A budget code
  accompanies KN7's `cut_off`, rather than changing it to a completed negative. Chapter 13 owns final spelling and the
  shared error envelope.

| Code | Meaning |
| --- | --- |
| `LG4:input.repro_input_missing` | Required source, module, snapshot, file bytes or a recorded implementation identity is unavailable; reproduction is not run. |
| `LG4:input.repro_key_mismatch` | Supplied bytes, versions, identity or parameters disagree with the requested key; admission is refused. |
| `LG4:evidence.reproducibility_mismatch` | Two admitted executions of a supported key disagree in exact payload bytes or deterministic coverage. |
| `KN7:budget.steps` | The abstract step allowance ends at the recorded boundary. |
| `KN7:budget.kernel_cost` | The declared kernel cost allowance cannot admit further work. |
| `KN7:budget.memory` | The deterministic logical memory allowance ends. |
| `KN7:budget.output` | The deterministic logical output allowance ends. |
| `LG3:capability.not_granted` | The requested operation has no effective grant or is barred by the fixed sandbox rules or chapter 07 policy. |
| `LG3:capability.grant_mismatch` | A requested resource, right, module or profile differs from its effective grant. |
| `KN7:resource_limit.recursion_exhausted` | Reserved: the recorded logical recursion or stack bound ends under DET.28's selected rule (ERR.26). |
| `KN7:resource_limit.host_exhausted` | Physical host resources or a host safety mechanism end the run; deterministic coverage is not promised. |
| `LG4:input.cache_entry_mismatch` | A cached value fails its recorded input or content checks and is refused as a reusable entry. |
| `KN7:resource_limit.runtime_lost` | The runtime dies; KN7.13's error and isolation rules apply. |

## 9. Conformance tests

- **DET.53 [D]** These tests are requirements for the conformance suite of chapter 12, under overview 3.4. No test in
  this table exists yet. LX-3 uses the relevant source and synthetic cases; their inclusion here records no execution
  or pass. Tests depending on an open accounting or text rule use that rule's selected, versioned fixtures.

| Test | Checks | Class |
| --- | --- | --- |
| CT-DET-01 | Change each key field alone, including menu order, cost table and checker identity; key equality fails. Change machine, user name, physical location or locale; the key is unchanged (DET.1 to DET.6). | source |
| CT-DET-02 | Repeat completed exact runs on Windows and headless Linux; compare canonical payload bytes, evidence and included checker traces, excluding the host envelope (DET.5, DET.6). | source |
| CT-DET-03 | Missing or substituted dependencies and inputs produce not run or refusal; no current-version fallback or external action occurs during reproduction (DET.7). | source |
| CT-DET-04 | A workspace, protection input, grant or file changes while the puzzle digest stays fixed; captured input identity changes or admission is refused (DET.4, DET.40, DET.41). | source |
| CT-DET-05 | Perturb hash seeds, insertion order and allocation; canonical traversal, tie breaking, draw order and output remain equal (DET.8, DET.9). | source |
| CT-DET-06 | Execute the generator reference vectors and child stream assignments across routes and platforms; host entropy and a clock-derived seed are refused (DET.11, DET.12). | source |
| CT-DET-07 | Integers beyond host word size, invalid fixed-width inputs and Unicode normalization or case pairs follow the selected contract in every host locale (DET.13 to DET.15). | source |
| CT-DET-08 | Repeat with cold caches, warm caches, value eviction and other runs retaining shared values; result bytes and all declared charges match (DET.16, DET.23, DET.24). | source |
| CT-DET-09 | Fixed keys with each deterministic budget stop at the same recorded boundary and coverage; equal-to-limit and simultaneous-limit cases follow the metering specification (DET.18 to DET.24, DET.29). | source |
| CT-DET-10 | Compare one abstract trace across RT1 to RT3, including optimized execution and kernel chunking; charges and cut-offs match or the mapping is reported unverified (DET.20 to DET.23). | source |
| CT-DET-11 | Recursive calls, nested checks and child jobs cannot mint allowance; parent and child ledgers reconcile, including aggregate live memory (DET.25, DET.26, DET.28). | source |
| CT-DET-12 | Vary other runs' scheduling and child completion order; fixed inputs keep exact payloads and budget coverage, while one-job admission remains atomic (DET.8, DET.26, DET.43). | source |
| CT-DET-13 | Empty completed enumeration, deterministic cut-off without a witness and not run remain distinct; cancellation, clock stop and host errors yield no negative claim (DET.17, DET.30). | source |
| CT-DET-14 | A checked witness survives an interrupted search; the unexamined domain remains open and a continuation has a new key (DET.29, DET.30). | source |
| CT-DET-15 | Compare exact display fields byte for byte and approximate fields at their declared precision on both platforms; perturb a float without changing an exact decision (DET.31, DET.32). | source |
| CT-DET-16 | Replay recorded exact A1 rotations and menu letters without invoking the float chooser; fresh requests near decision boundaries remain LX-9's separate obligation (DET.33, DET.34). | synthetic |
| CT-DET-17 | In each profile, route and placement, planted clock, environment, file, network and process access without grants is refused; no interface yields session files or a commit token (DET.10, DET.35 to DET.42). | source |
| CT-DET-18 | File grants cannot escape their resource or rights; imports, calls and children cannot widen grants, and a saved descriptor supplies no replay authority (DET.37, DET.40 to DET.42). | source |
| CT-DET-19 | All LX-3 must-refuse compositions and host-subset substitutions are refused under the tested policy and flow form; harmless-tool refusals are counted separately (DET.36, DET.38, DET.50). | source |
| CT-DET-20 | Endless work, deep recursion, logical exhaustion and killed-runtime fixtures preserve journal and state hash under fault alone; the job outcome is typed (DET.28, DET.43). | source |
| CT-DET-21 | A builder cannot consult a clock, add a computation thread or import ambient modules; its output and scratch grant points outside the source tree (DET.44, DET.45). | source |
| CT-DET-22 | Tamper cache contents or change captured construction inputs before publication; reuse fails closed and a verified rebuild has unchanged charges (DET.46). | source |
| CT-DET-23 | Interrupt atomic publication at every boundary; no partial certificate becomes completed, and canonical content identity survives different compression metadata (DET.47). | source |
| CT-DET-24 | Import an archived timed stop and a data-only archive without producers; preserve those limits without claiming deterministic replay, while independently checked witnesses retain their scope (DET.48). | source |

## 10. Open items

- **LQ1.** One language and three profiles, a separate research language, or a product-only language (DET.37, DET.39,
  DET.44). Exact imported evidence and product sandbox rules stay fixed; executor coverage and shipped profiles change.
- **LQ2, LQ14, LQ21.** Chapter 07's capability policy, any competence-dependent grant and the form of flow control
  (DET.4, DET.20, DET.37, DET.38, DET.40, DET.50). Labels and capability separation need different enforcement and can
  refuse different harmless tools; neither changes the commit gate or permits ambient authority.
- **LQ3, LQ10.** Evidence output and its stored form in chapters 05 and 06 (DET.5, DET.29, DET.30, DET.48). The key pins
  the producing rules under each option; the envelope's facts or judgements and their bytes can differ. No cut-off
  becomes a proved negative under either option.
- **LQ4.** Pinned research jobs after a commit (KN7.5; DET.4, DET.17, DET.29). Finishing and cancellation are the two
  alternatives; immutable-input identity stays fixed. Cancellation adds no reproducible stop promise. Chapter 08 owns
  result transport and branch lifecycles.
- **LQ5, LQ16.** Built-in implementation and the shipped research cut (DET.3, DET.21, DET.39, DET.44). Native and language
  tools can use different execution paths; each path's declared charges and identity remain recorded. These clauses
  do not settle which tools or research profiles ship.
- **LQ7, LQ20.** User charts and the renderer handoff (DET.31, DET.32). Chapter 10 supplies the precision contracts;
  a generated chart stage and built-in charts can produce different plans. Exact identity and the declared-precision
  comparison remain the same.
- **LQ8, LQ9.** Engine, route and placement (DET.3, DET.8, DET.21, DET.22, DET.36, DET.43). The semantic key, sandbox
  boundary and exactness promise stay fixed. Metering adapters, algorithm dependencies and fault containment need
  evidence for the selected host. No source establishes equivalence of borrowed VM fuel and abstract steps.
- **LQ12.** Module sharing (DET.7, DET.15, DET.42). Local file exchange and possible later distribution options retain
  content identity; default 2 supplies no network access in 1.0.
- **LQ18, LQ19.** Permission and ordering of spikes and freeze gates (DET.12, DET.20, DET.22, DET.24, DET.26, DET.28,
  DET.51). Chapter 11 owns decision rules and chapter 12 owns phase gates. Deferring experiments leaves these contracts
  unverified; it supplies no pass or architecture-freeze authorization.
- **Generator and streams [O].** DET.12: stateful draws or counter-addressed generation; LP1's library specification
  selects the algorithm, derivation rule and reference vectors.
- **Unicode semantics [O].** DET.14: scalar-value rules or pinned normalization and comparison tables; chapters 02 and
  05 fix equality, order, invalid-input handling and byte identity.
- **Abstract meter [O].** DET.20: operation counts or weighted IR charges, including boundary and exhaustion rules;
  LP1 fixes the contract. DET.22: LX-1, LX-3 and LQ9 establish whether each executor can preserve it.
- **Logical resources [O].** DET.24: ownership and release rules, alias accounting, output units and terminal-result
  space; LP1's accounting specification makes the repeatable charge fixtures possible.
- **Child accounting [O].** DET.26: serial shared meter or deterministic reservations; LP1 fixes conservation,
  aggregation and return rules before concurrent children claim reproducibility.
- **Limits and recursion [O].** DET.28: explicit depth or a logical stack, numeric presets and ceilings; LP1, chapter
  02 and chapter 07 supply their respective rules. There are no measured preset values in this chapter.
- **Kernel and storage specifications.** DET.1 to DET.7, DET.19, DET.29 and DET.40 need LP1's kernel API and cost table
  identities, chapter 02's metering and module contract, and chapter 05's byte rules. The field obligations are fixed;
  their final encodings and compatibility mechanisms belong to those chapters. EN4 supplies the command rows and
  admission integration used by DET.35 and DET.43.
- **Input map.** DET.33, DET.34 and DET.51: LX-9 and any A1 amendment remain open. Round 3 cleared DT-L1's obligations
  (overview section 8), and clauses citing R2-E2, R2-E3 or R2-E4 name those dependencies.
- **Historical builders.** DET.48 records the gap between the partition's no-clock builder requirement and archived
  timed producers. Their clocks and missing producer code remain reproduction limits; importing them changes no rule.
- **Conformance and error registry.** DET.52 and DET.53: chapter 13 collects final code spelling; chapter 12 collects
  the required tests. LX-3, LX-2 and LX-9 have no result here.

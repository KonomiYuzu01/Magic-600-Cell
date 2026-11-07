# Chapter 09: determinism, budgets and sandbox

Status: **draft for the owner. Not accepted.** Conventions are in [the overview](00-overview.md), section 3.

This chapter defines the identity of a reproducible run and the limits on its execution. It uses `Budget`, `JobResult`,
snapshots and kernel costs from [chapter 01](01-kernel.md). Chapter 02 owns types, effects and the IR; chapter 05 owns
stored bytes and `RunRecord`; chapter 07 owns profiles and boundary policy. Section 11 proposes a versioned execution
contract and generator. It selects no runtime or budget preset, and accepts no owner question or ADR.

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
- **DET.12 [D]** The draft recommends the counter-addressed generator `sha256-counter-draft-1` in DET.61, with
  explicit stream paths and reference vectors. Its purpose here is specified reproducibility, not a measured speed
  or statistical-quality claim. A stateful generator with a recorded draw order remains the alternative; it needs
  its own algorithm identity, stream mapping and vectors and cannot reinterpret saved seeds under this identity.
- **DET.13 [B]** Integer decisions use KN3's exact integers and explicit size-based costs (INV-4, KN3.1, KN3.2,
  KN3.6). Host word size cannot cause wraparound or silent narrowing. Typed fixed-width data follows KN1's byte
  conventions and chapter 02's range rules. Approximate values never convert implicitly into exact decisions.
- **DET.14 [D]** The draft consumes HANDOFF H8's scalar `Text` comparison rule; DET.62 supplies its metered
  operations and invalid-input result. There is no implicit case folding or normalization. Explicitly versioned
  normalization/comparison tables remain an alternative language contract, with a different version and key.
  Chapter 05 alone defines canonical stored bytes; scalar comparison does not re-encode or normalize hashed source.
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
| `memory` | KN7.9's state payload units plus DET.57's versioned language layout; DET.58 fixes handle lifetimes | Resident memory, pointer size, allocator layout, stored text form and physical sharing |
| `output` | Canonical uncompressed bytes under the pinned data-core profile (DET.59; HANDOFF H8) | Compression ratio, terminal speed and transport packet sizes |
| `safety_stop` | Optional host deadline, outside the deterministic key | No exact replay promise applies when it stops a run |

- **DET.19 [D]** A budget record identifies its units, following LG2 and INV-10 (DT-L1 R2-E4). Length and puzzle cost
  retain their alphabet: signed retained generators, jumbling menu letters and UI actions are different quantities.
  A kernel cost table states how these typed inputs and operand bit lengths produce its declared integer charge.
  Interpreter steps and declared kernel work are separate counters. A change of spelling cannot convert one alphabet
  or budget dimension into another.
- **DET.20 [D]** The draft meter is `lp1-exec-draft-1`: one step for each semantic operation or continuation in
  DET.55, plus a pinned input-size tariff for work delegated to a kernel. DET.56 specifies checks and exhaustion;
  DET.57 to DET.64 specify resources, children, generators, text, streams and display branches. LG4.8 records why
  this form is recommended and retains the weighted alternative. LG2.21 gives form 2 its separate execution identity
  and lowering, charged by DET.64. The step schedule is semantic, not a performance
  estimate. Pointwise predicates, loop tests, calls and applicable run-time flow checks are included; whole-state
  contracts retain LG4.10's separate revision/agreement-test rule.
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
- **DET.23 [D]** Each logical kernel operation declares its input-dependent charge before admission (KN7.8).
  It is charged once. An unchunked call admits its entire tariff; a declared streaming call admits the fixed semantic
  chunks of DET.63 in order. Completed chunks partition that tariff without a per-transport-chunk surcharge.
  A cache hit, eviction, reconstruction retry or IPC split changes no charge, continuation or coverage. A nested
  language call exposes its constituent kernel operations; a native row either includes internal work in its tariff
  or exposes a pinned decomposition, never both. KN7.6's cooperative interruption is checked at those boundaries.
- **DET.24 [D]** The draft chooses deterministic owning handles with explicit release (DET.57, DET.58) and
  HANDOFF H8's canonical uncompressed output bytes (DET.59). Each live alias is charged in full, independent of
  physical sharing; dropping a handle releases its charge at the specified operation. Used memory is peak live
  charge, while steps, kernel work and output are cumulative. The alternative is conservative retention to a
  declared scope, including run end, and an alternative output-size table could replace canonical bytes. Either
  alternative needs a distinct version and key. Host garbage collection never supplies a release boundary.
- **DET.25 [D]** Work performed on behalf of a run remains within that run's limits, following the explicit per-job
  budget of default 6 and INV-10. A call, recursive call, checker invocation or child job cannot reset its parent's
  counters or mint an allowance. A parent that uses child results records their budgets and coverage. A genuinely
  separate, host-authorized run has its own key; consuming its result makes that immutable result a declared input.
- **DET.26 [D]** The draft chooses serial children sharing ancestor meters, with optional child caps and
  deterministic unwind/return rules in DET.60. A child cannot mint an allowance, refund completed work or choose a
  winner by completion time. The alternative is disjoint allowances reserved before launch, with memory retained
  to fixed joins and deterministic return of unused allowance. That alternative requires another version and its
  own vectors; it is not enabled by this draft and does not change KN7's host admission lanes.
- **DET.27 [D]** A run starts only with finite, explicit deterministic limits accepted for it, following default 6.
  The host exposes the granted limits, units, cost table identity and amount used as data. The exact expanded length
  and puzzle cost of a word are available before a run (KN5.10). Presets resolve to concrete limits in the record;
  neither a machine-speed estimate nor a later preset edit changes an existing key. A program that reads the amount
  used after reading state reads a state-derived value (BND.44).
- **DET.28 [O]** Numeric presets and ceilings remain unselected and unmeasured. Every run supplies finite
  limits. For the draft meter, recursion uses DET.57's logical frames charged to `memory`; frame entry checks before
  a callee executes, and DET.58 releases the frame on return. There is no independent depth budget and the reserved
  `KN7:resource_limit.recursion_exhausted` code stays inactive. A separate explicit depth guard is the alternative,
  requiring a versioned counting rule and recorded limit. A lower physical host limit remains an incomplete host
  error, not a deterministic recursion stop. Chapter 07 adds no capability-dependent ceiling (BND.62).
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
  cancellation and errors also establish no absence. Chapter 06 owns claims and display words.

## 4. Display precision and exact input requests

- **DET.31 [B]** Exact decisions remain exact at every display boundary (INV-4, KN3.1). A display coordinate or a
  comparison of approximate coordinates cannot decide legality, identity, protection, equality or evidence. Chapter 10
  owns `DisplayPlan` and its connection to canonical IDs.
- **DET.32 [D]** Display plans depend on the maths library and are compared at their declared precision, as partition
  8.4 adopts from DT-L1 R2-E2. Their exact fields still obey DET.5: object identities, ordering, source references and
  exact declared properties. Approximate fields carry chapter 10's precision and domain contract. LX-3 compares those
  fields across platforms at that precision. It neither requires identical float bits nor uses tolerances to weaken
  an exact field. The precision contract is pinned with the plan's producing module and inputs.
- **DET.33 [D]** Under proposed amendment A6, a request is exact bytes: C, a dyadic Cayley vector; H, a primitive
  integer half-turn axis; or M, a menu letter bound to its set and order digests (KN3.4, KN3.7, KN3.8; owner,
  6 November 2026, decision 8). A run consuming it records the bytes and chosen exact rotation, with its identity
  block and base input, before using it as an exact input. A menu-mode choice records the actual menu letter under
  its menu identity and order (INV-4). Replay uses the recorded choice. It never recomputes trigonometry, nearest
  candidates or the choice from a recorded drag. A6 is not in force until EN1's Astra check and then the owner.
- **DET.34 [D]** The float site is outside the kernel, in the shell's producer (KN3.9). It reads binary64 values
  bit-exactly, refuses non-finite values and canonicalises signed zero before producing request bytes. Identical
  gestures need not give identical bytes. Identical bytes, operation, bounds and menu binding must give the same exact
  choice under KN3.7 and KN3.8. Menu drags compare every ordered candidate by exact squared Frobenius distance in
  Q(√5), with the smallest index winning an exact tie. Free mode takes the nearest point of A1's grid, the owner's
  choice (KN3.7). LX-9's follow-up finding fed decision 8; its exact-request prototype is source and synthetic
  evidence for its tested cases only (RTE.64 to RTE.66). Chapter 11 owns the remaining checks and amendment
  dependency. Replaying a recorded choice establishes neither a gesture producer nor fresh-request agreement.

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

- **DET.38 [D]** The table does not answer LQ2's option set or LQ21. Chapter 07 supplies `Profile`, provenance,
  boundary classes and declassifiers. Under flow labels a grant is checked with the permitted data and control flows;
  under capability separation, reading state-derived data and preparing previews cannot coexist except through policy-enabled
  declassifiers, and preview outcomes are state-derived (DT-L1 R2-E3). Until the joint setting, the partition's section
  11 interim reading applies: research output never becomes a solving-session preview, solver-tool solving previews
  remain disabled, and built-in tools keep the 2 October boundary and the 17 October exception. A file grant preserves
  stored provenance; it cannot launder a word through storage.
- **DET.39 [D]** One language has three profiles: solving, tool authoring and research. They share types, exact
  semantics and the key format; profiles change grants and labels (owner, 6 November 2026, LQ1; partition LQ1).
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
  a process capability. Module sharing is local through user-chosen files with digests (owner, 6 November 2026, LQ12).
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
- **DET.51 [D]** Chapter 11 owns LX-3's decision rule, experiment card and relation to LQ9 and LQ21. RTE.82 records
  the first-pass spike evidence under the owner's LQ18 allowance. It supplies no product conformance result
  (overview 3.4; partition section 12). Exact portability, repeatable cut-offs, sandbox containment, route-equivalent
  metering and display agreement for admitted product executors remain unverified. LX-1 supplies cost leads; LX-2
  tests placement faults; LX-9's follow-up fed the exact-request direction in decision 8. None is replaced by a
  document structure check. Windows-first product scope stays the 2 October decision: headless Linux checks of
  platform-independent layers promise no Linux product build, native input, DirectX, long-session or performance result.

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
| `LG4:input.pointwise_failed` | An admitted pointwise assertion is false; its guarded operation has not run (DET.55). |
| `LG4:input.generator_input` | Seed size, scalar stream path or path count/length is invalid under DET.61. |
| `LG4:resource_limit.generator_exhausted` | An admitted draw requests a counter after the last defined block; no wraparound (DET.61). |
| `LG4:input.text_scalar_invalid` | Invalid UTF-8 or a surrogate would enter a `Text` value (DET.62). |

## 9. Conformance tests

- **DET.53 [D]** These tests are requirements for the conformance suite of chapter 12, under overview 3.4. No row in
  this table is a product conformance pass. The pure-Python draft vectors in `contract-vectors/` exercise the meter only.
  LX-3 still needs admitted executors and matching source and synthetic cases on both platforms.

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
| CT-DET-16 | Under proposed amendment A6, identical exact request bytes, operation, bounds and menu binding give identical choices, including exact menu ties. The shell producer refuses non-finite values and canonicalises signed zero; gesture equality promises no byte equality. Replay reads the recorded rotation or menu letter without invoking the producer or chooser (DET.33, DET.34). | synthetic |
| CT-DET-17 | In each profile, route and placement, planted clock, environment, file, network and process access without grants is refused; no interface yields session files or a commit token (DET.10, DET.35 to DET.42). | source |
| CT-DET-18 | File grants cannot escape their resource or rights; imports, calls and children cannot widen grants, and a saved descriptor supplies no replay authority (DET.37, DET.40 to DET.42). | source |
| CT-DET-19 | All LX-3 must-refuse compositions and host-subset substitutions are refused under the tested policy and flow form; harmless-tool refusals are counted separately (DET.36, DET.38, DET.50). | source |
| CT-DET-20 | Endless work, deep recursion, logical exhaustion and killed-runtime fixtures preserve journal and state hash under fault alone; the job outcome is typed (DET.28, DET.43). | source |
| CT-DET-21 | A builder cannot consult a clock, add a computation thread or import ambient modules; its output and scratch grant points outside the source tree (DET.44, DET.45). | source |
| CT-DET-22 | Tamper cache contents or change captured construction inputs before publication; reuse fails closed and a verified rebuild has unchanged charges (DET.46). | source |
| CT-DET-23 | Interrupt atomic publication at every boundary; no partial certificate becomes completed, and canonical content identity survives different compression metadata (DET.47). | source |
| CT-DET-24 | Import an archived timed stop and a data-only archive without producers; preserve those limits without claiming deterministic replay, while independently checked witnesses retain their scope (DET.48). | source |

## 10. Open items

The owner answered LQ1 to LQ21 on 6 November 2026. LQ8, LQ9, LQ13 and LQ21 wait for their evidence, and LQ2's option set
waits for the joint setting.

- **LQ2's option set and LQ21.** Chapter 07's capability policy and the form of flow control
  (DET.4, DET.20, DET.37, DET.38, DET.40, DET.50). Labels and capability separation need different enforcement and can
  refuse different harmless tools; neither changes the commit gate or permits ambient authority.
- **LQ8, LQ9.** Engine, route and placement (DET.3, DET.8, DET.21, DET.22, DET.36, DET.43). The semantic key, sandbox
  boundary and exactness promise stay fixed. Metering adapters, algorithm dependencies and fault containment need
  evidence for the selected host. No source establishes equivalence of borrowed VM fuel and abstract steps.
- **Draft execution contract.** DET.12, DET.14, DET.20, DET.24 and DET.26 now have recommended rules and
  reference vectors in section 11. The alternatives remain recorded. Review and owner freeze are pending;
  executor mappings, production tariffs and resource descriptors still need implementation and validation.
- **Limits and recursion [O].** DET.28 keeps presets and ceilings open. The draft uses logical frames; a separate
  depth guard is an alternative contract, not an additional silently enabled limit.
- **Kernel and storage specifications.** DET.1 to DET.7, DET.19, DET.29 and DET.40 need LP1's kernel API and cost table
  identities, chapter 02's metering and module contract, and chapter 05's byte rules. The field obligations are fixed;
  their final encodings and compatibility mechanisms belong to those chapters. EN4 supplies the command rows and
  admission integration used by DET.35 and DET.43.
- **Exact requests and the producer.** DET.33, DET.34 and DET.51: proposed amendment A6 waits for EN1's Astra check
  and then the owner. The owner chose the free-mode grid (KN3.7), and KN3.9's producer detail remains subject to that
  review and the owner's ruling. RTE.82 records the finite experiment evidence; product and native-input checks remain
  owed. Round 3 cleared DT-L1's obligations (overview section 8), and clauses citing R2-E2, R2-E3 or R2-E4 name those
  dependencies.
- **Historical builders.** DET.48 records the gap between the partition's no-clock builder requirement and archived
  timed producers. Their clocks and missing producer code remain reproduction limits; importing them changes no rule.
- **Conformance and error registry.** DET.52 and DET.53: chapter 13 collects final code spelling; chapter 12 collects
  the required tests. RTE.82 records the first-pass LX-3, LX-2 and LX-9 results and what they leave unrun.

## 11. LP1 draft execution contract

- **DET.54 [D]** `lp1-exec-draft-1` names the following semantic contract, subject to review and owner freeze. It is
  neither source syntax nor a serialized IR. A checked module pins its lowering to these operations, the operation
  order, source map, logical layouts and all native row declarations. A native row declares its exact meaning,
  input-size tariff, logical result/scratch bounds, continuations and coverage rule. A row lacking any of these is
  not admitted. A library body uses its constituent operations; an opaque native row uses its declared tariff.
  Executors cannot choose between the two decompositions for one key. The unit identities in DET.1's `budgets` pin
  this contract, layout and output profile; `ir_version` pins the lowering and `kernel_cost_table` pins every tariff.
  These are values of existing semantic fields, not new KN1 identities or wire fields. Chapter 05 (PS1.31, PS2.1 to
  PS2.9) owns encoding, unsupported-version refusal and recorded conversion. A changed stop rule creates a new key;
  old bytes and old run meaning are retained. No old or LX-3 experiment key is silently assigned this contract.

- **DET.55 [D]** Evaluate operands left to right in the checked operation order, then attempt their consumer.
  Each operation below costs one step. A failed step executes nothing. Successful step admission precedes DET.56's
  remaining checks. A literal loads one scalar or one reference; it cannot copy an arbitrarily large value for one
  step. A variable-size operation either expands into these continuations or invokes a pinned size-priced row.
  There is no unmetered loop, recursive call, collection walk, hash, comparison, encoding or big-integer operation.
  Input sizes are retained metadata constructed with the value; computing new size metadata uses the same rules.

| IR family (LG4.2) | Semantic operations and continuations | Additional charge and effect boundary |
| --- | --- | --- |
| Values and exact computation | `value` constructs a scalar/header, `visit` reads or fills one declared slot, `alias` retains a value, `drop` releases it; arithmetic/comparison invokes a named row or checked library body | A handle's logical payload uses DET.57. Native variable-size work has the input-size kernel tariff; all exact integers, including small ones, use the same pinned rule |
| Calls and control flow | `call`, `return`, `branch`/tag match, `loop_test`, and one `visit` per yielded element; generic specialization itself is static | Argument operations precede `call`; frame memory is checked at call. Every loop test, including the final false test, costs one. `return` costs one, including an implicit return. Tail-call elimination preserves both events |
| State and checks | `kernel` for each snapshot open/query, projection, derivation, independent check or batch; `drop` for a close | Charge the row once, including its result/scratch reservation, before entry. A separately opened snapshot has a separate full charge. Nested native work is included or decomposed as DET.23 states |
| Words and recipes | `value`/`visit` construct parts, `word_open` creates the expansion cursor, `chunk` advances a declared semantic batch | Compressed structure and cursor stay live. DET.63 prices expansion and replay, preserves chronological positions, and meters size-dependent key computation separately |
| Commands and receipts | `invoke` one permitted row, `read_receipt` one immutable result | Argument construction, any kernel work and canonical output unit are charged before dispatch. Polling is another operation; host notifications never choose computational order or supply an ambient result |
| View output | Ordinary construction operations, `display_if`, then `publish_view`; under LG2.21's alternative, `display_enter`, `display_branch`, `display_exit`, then publication | DET.64 charges the selected branch and the selected form's boundary operations. Publication admits the complete plan's bytes under DET.59 and has no engine adoption authority |
| Granted input and output | `read` one declared captured-input chunk, `emit` one complete output unit, `rng_init`, `rng_draw` | Input transfer/decoding and output encoding use declared size tariffs or visits. Captured bytes have logical memory charges. A clock read remains outside reproducible execution |
| Resource control | `assert`, `flow_check`, `child`, `scope_exit` | A predicate is evaluated normally before its one assertion/check step. Budget preflight itself adds no recursive charge. Scope exit releases that scope's handles; DET.60 governs children |

  A pointwise assertion runs after its predicate and before the guarded operation. Failure is
  `LG4:input.pointwise_failed`. An applicable run-time `flow_check` occurs after arguments and before each call,
  branch, retained binding or sink required by the selected checked flow form. One check tests one fixed descriptor;
  walking a variable label set uses visits or a size tariff. Failure retains the check's admitted charge and uses
  chapter 07's selected flow refusal. Static checks cost no run-time step. The chosen flow form, policy and checked
  insertion sites are pinned by DET.4; this clause activates neither reserved flow spelling nor an LQ21 choice.
  Whole-state contracts remain once per revision or agreement tests (LG4.10); a run-requested check is still charged.

- **DET.56 [D]** At each continuation, first observe an already pending solver/host interruption. Such a stop is
  outside deterministic reproduction (DET.17). With no interruption, apply the following admission algorithm:

  1. Check `used.steps + 1 <= limit.steps` in every active meter, from root to innermost child. If a check fails,
     report `KN7:budget.steps` at this operation; debit nothing. Otherwise debit one step in all active meters.
  2. Obtain the input-derived requests `(kernel_cost, memory_reservation, output_bytes)`. Preflight these dimensions
     in that order, and within each dimension test root before children. Test cumulative kernel/output use and live
     memory plus the reservation. Equality is admitted; only a strict excess fails. The first failing dimension
     and outermost failing meter determine the code and exhausted scope.
  3. On failure, retain the step but debit none of those three requests. Execute no work, retain no new handle and
     expose no output. On success, debit all three together before the operation's effect. Releases occur after
     admission, so dropping or replacing an old value cannot fund its replacement early. A conservative result
     reservation is reduced to its actual logical size after success; its high-water use remains in the peak.

  A fully admitted operation that returns a domain error keeps its declared charges; no cache, error or cancellation
  refund is available. Zero remaining allowance is not itself a stop: completion and a zero additional resource
  request can succeed at equality. Stop position names the checked operation, dynamic call/child and loop path,
  continuation/chunk index, failed phase and exhausted scope. Source location consumes HANDOFF H7's `SourceRange`
  against the executed bytes, including generated operations' originating range. Never substitute current source.
  No partially performed operation counts as a completed result or coverage increment.
  Tariff/reservation descriptors are bounded expressions over retained size metadata and pinned constants, using
  nonnegative addition, multiplication, minimum, maximum and ceiling division. Admission may saturate a comparison
  at the relevant limit plus one only if the final exact comparison is preserved, including later divisions;
  it never evaluates an unpriced user function. A more complex size computation
  must appear as earlier metered operations. Bookkeeping is not a recursive call to its own meter.

- **DET.57 [D]** `lp1-logical-layout-draft-1` extends KN7.9 to language values without changing state sizes or digest
  bytes. These are logical payload units, not stored JSON lengths or physical memory measurements. Every owning
  handle charges its full payload even when another handle has the same value; aliasing does not receive a discount.
  A borrowed operand names an existing live owner and creates no owner. Retaining it, including in a closure or a
  returned object, requires a charged owner or a move. A run's captured inputs and open snapshots are charged at
  explicit entry operations before user computation; source and contract tables are admission metadata.

| Payload | Logical layout |
| --- | --- |
| Boolean, fixed-width scalar, display scalar | 1 byte for Boolean; declared width in bytes for a fixed-width/display scalar |
| Unbounded integer `n` | One sign byte plus `max(1, ceil(bit_length(abs(n))/8))` magnitude bytes |
| Rational or Q(√5) value | Sum of its canonical exact integer components; chapter 05 owns their canonical values |
| `Text` | Four bytes per scalar; this is neither UTF-8 storage nor output length |
| Array/state/pose payload | KN7.9 and the declared element widths; canonical pose-key length stays KN1's convention |
| Finite algebraic record or sequence | Four-byte variant tag, eight-byte length per variable sequence, and the sum of owned field/element payloads; repeated owned values count repeatedly |
| Closure | Its checked code reference (32 bytes) and the owned payloads of captures in declaration order |
| Function frame | 16 bytes plus eight bytes per argument/local/temporary slot in the checked frame layout; payload owners are charged separately |
| Generator | 41 bytes: the 32-byte stream key, eight-byte counter and exhausted flag (DET.61) |

  A kernel row also declares an input-derived logical workspace and result bound. Both coexist with its live inputs
  at admission. Workspace is released at its declared continuation; excess result reservation is released after the
  result size is known. If there is no finite bound computable from metered input metadata, the row must expose
  growing/chunked continuations. Backend scratch choices do not change this declared logical reservation. Immutable
  recursive module references are code identities, not infinitely sized value payloads. No unbounded or cyclic
  owned payload is admitted. A checker cannot silently erase an allocation, frame or alias from this schedule.

- **DET.58 [D]** Handles belong to a logical scope. `drop` releases exactly that handle after its step, while an
  alias remains separately charged and valid. `scope_exit` and `return` release the scope's remaining owners and
  frame after their steps. A returned owner moves to the caller without a second payload charge; returning a borrow
  first creates a charged owner. A replacement acquires its new payload before releasing the old one. Releases have
  no user destructor or computation; the scope maintains its charge total as owners enter and leave. Host allocation,
  GC, cache eviction and last-use optimization are irrelevant. Ordinary calls borrow their supplied arguments and
  share the caller's counters. Return never refunds steps, kernel cost or output.

  Each meter records cumulative steps/kernel/output, peak live memory and current live charge. On any terminal
  stop, capture counters and live charge before cleanup; then release abandoned scopes without further program
  evaluation or charge. Abandoned scopes include admitted result/scratch reservations for which no owner was
  installed; release those reservations in the scope and every charged ancestor too. Used `memory` is the peak,
  not the final live charge. Cleanup cannot rescue the denied operation. The terminal record may retain a charged
  immutable output by reference; it cannot keep an uncharged user value alive inside the reserve.

- **DET.59 [D]** Output consumes HANDOFF H8's unit: the length of the complete canonical uncompressed value under
  the pinned data-core profile of chapter 05. That profile owns number, Unicode, escaping, key order and terminal-LF
  rules. A result, record, trace item, plan, granted-file record or top-level returned value is an output unit.
  Nested returns are internal moves, not output. Construction/encoding work and temporary storage are metered
  separately from the cumulative byte charge. Counts use the canonical bytes including the value's own delimiters;
  transport framing, compression and repeated delivery of the same admitted unit add nothing. Two program emissions
  are two units even if equal. A complete unit is admitted before publication; failure emits none of that unit and
  keeps previously emitted units charged. A streaming API pins its sequence of complete units before execution;
  changing transport packet sizes cannot split a unit or change a stopping point.
  The byte length is retained metadata from metered construction, or comes from a preceding metered encoding pass.
  Publication never performs an unpriced length-discovery pass. A fused encode/publication row is possible only
  when its exact byte length is already available; its input-size tariff still covers the encoding work.

  Before admission the host reserves a fixed terminal capacity under the pinned output profile, separate from the
  program's output and memory limits. Its bounded schema permits only outcome/code, counters, stop locator and
  digests of already charged coverage/results/child ledgers. It contains no arbitrary message, user text, trace,
  plan or new coverage list. The profile supplies a capacity sufficient for the largest such status at the accepted
  numeric/source-locator bounds; these bounds are checked before admission and included in the unit identity.
  Rich coverage and ledger records use ordinary charged output units or deterministic re-derivation references.
  Thus an output limit of zero still permits a terminal status and cannot be bypassed to return user data. This
  specifies the reserve's purpose and admission rule, not a measured product byte capacity or a second byte codec.

- **DET.60 [D]** A `child` operation costs one step in its ancestors, then starts a serial subrun while its parent
  is suspended. The effective child caps are the componentwise minimum of its explicit requested caps and all
  ancestors' remaining cumulative allowances/free live memory after that step. Record requested and effective caps.
  An omitted cap resolves to the ancestors' remaining allowance in that dimension; the resolved cap is explicit
  before entering the child. Omitting it creates no extra allowance.
  No speculative or concurrent child work is admitted under this contract. Child entry/return use ordinary
  `call`/`return` operations inside those caps, with the same frame and payload rules as any call. The child borrows
  parent inputs; existing parent owners remain charged in the parent and root. Its local memory cap counts new
  allocations, while each ancestor counts its own live owners plus all descendants' new allocations.

  Every accepted child debit updates each ancestor once, including use by nested children; merging a ledger adds
  nothing again. Ordinary function calls have no new allowance. On normal child return, close its frame and move
  declared result owners to the parent without duplicating the root charge. Its ledger records local used/peak,
  transferred live payload, outcome and stop. Unused capacity becomes available automatically; completed cumulative
  use never returns. A failure of a child cap unwinds that child and its descendants, releases their non-returned
  owners and resumes its parent with a typed `cut_off` result. A root limit stops the whole run. If an outer child
  cap and an inner cap fail together, the outer child stops. Dimension precedence is DET.56, before scope precedence.
  Emitted child output remains charged and has the same visibility rule as any output. A cut-off returns no
  unadmitted value and no negative claim. The child's counter/ledger observation is state-derived where BND.44 applies.
  Reading status fields uses ordinary operations. Retaining a status or ledger as a language value requires its
  normal owner charge, and publishing it requires its output charge; the terminal carrier is no free value store.

- **DET.61 [D]** The proposed generator is fully addressed; it has no host entropy or scheduling input. For
  `sha256-counter-draft-1`, a seed is exactly 32 bytes. A stream path is a finite sequence of scalar `Text` segments.
  Encode its segment count as unsigned 32-bit big-endian, then each segment's UTF-8 byte length as unsigned 32-bit
  big-endian followed by those bytes, without normalization. Counts/lengths outside that range are invalid input.
  The stream key is `SHA256(ASCII("C600-LP1-RNG") || 00 || seed || encoded_path)`. `rng_init` charges the input-byte
  tariff and its 41-byte generator owner; its counter starts at zero. Draw `i` is the unsigned big-endian integer
  `SHA256(stream_key || u64be(i))`, for `0 <= i < 2^64`. Each admitted `rng_draw` costs one step, the declared fixed
  block tariff and any retained result reservation. Only a successful draw advances the counter; a denied charge
  consumes no draw. After the last draw set the exhausted flag; another admitted request returns
  `LG4:resource_limit.generator_exhausted`, never wraparound. Invalid seed/path data uses `LG4:input.generator_input`.
  Each generator owner holds its own cursor. An alias copies key/counter/flag into an independent cursor at the full
  41-unit charge; subsequent draws advance only the named owner. A borrowed argument refers to that owner's cursor,
  and a move transfers it. Fixed-size cursor updates require no additional live memory. Repeated draws from copied
  cursors intentionally repeat blocks; they are not automatically assigned fresh streams.

  Stream assignment is fixed before execution: append the deterministic child invocation index as a decimal path
  segment to its parent's assigned path, recursively, without drawing a new seed. Named sibling streams are
  separate sealed paths. Re-initializing the same seed/path reproduces the same stream deliberately. A bounded
  random-choice library uses a pinned rejection algorithm and meters every block, comparison and retry; modulo
  reduction is not an implicit bounded draw. This block contract promises no cryptographic or statistical fitness.

- **DET.62 [D]** Following HANDOFF H8, compare `Text` lexicographically by scalar value; the first unequal scalar
  decides, and a proper prefix sorts first. Equality compares the scalar sequence without normalization or case
  folding. A native text comparison is one step plus a tariff on both complete scalar lengths, even when an early
  mismatch or a cache makes it cheaper physically. A lowered comparison instead uses its pinned visits and tests;
  these are different decompositions and cannot be selected opportunistically. Invalid UTF-8 or a surrogate is
  `LG4:input.text_scalar_invalid` before the invalid value is bound. Decoding captured bytes uses its declared
  input-byte tariff and logical result bound. Explicit normalization/case conversion, if supplied as a library,
  pins its tables, effects and tariffs and never changes the meaning of this comparison or source digests.

- **DET.63 [D]** A streaming row pins a semantic chunk schedule determined only by immutable inputs and its row
  revision. `word_open` costs one step and declares expanded length, total kernel tariff, expansion tariff and
  logical cursor/workspace bounds. It does not pre-debit the chunks. Each `chunk` costs one step and preflights its
  share of kernel work and live workspace before expanding or replaying any of its letters. The sum of chunk
  tariffs equals the declared full tariff; setup cost, if any, belongs to `word_open`, never every transport packet.
  A zero-length stream has only its open event. A final short chunk has its actual declared share. Expansion work
  is included once in the chunk tariff or decomposed into explicit visits, as pinned by the row, never free.

  The full tariff and its shares must be computable from the sealed call inputs before `word_open`. A later state
  cannot change an announced share. Adaptive work without such a bound is a sequence of separately declared calls,
  not a stream claiming that upfront total. Chunk coverage advances only on completion, in chronological literal
  positions. A denied chunk does not expand,
  draw randomness, allocate or advance coverage. Scratch is released after completion; input/cursor and declared
  retained results stay live. Logical result bounds may use the current exact state's retained size metadata; the
  schedule and bound expressions are pinned. A mid-chunk domain failure reports the row's exact prefix;
  an asynchronous cancellation reports only its verified coverage and has no deterministic stop promise. Transport
  may split/coalesce chunks but cannot alter this schedule, the tariff partition, literal digest, protection scope
  or cancellation points (LG4.6, LG4.9). An executor changing semantic chunk size needs a new row identity and key.

- **DET.64 [D]** Under HANDOFF H6's working `ApproxBool` form, `display_if` costs one step plus only the selected
  branch's ordinary operations. The condition is evaluated first and charged normally. Branches remain pure display
  expressions; chapter 02's exactness discipline applies through calls, closures, storage and exports. LG2.20 owns
  the accounting-isolation and reproduction rule for both forms. This selects no LQ21 form.

  For LG2.21's alternative `lp1-exec-region-draft-1`, `display_enter`, `display_branch` and `display_exit` each cost
  one step under DET.56. Those boundary operations have no additional kernel tariff, allocation or output charge;
  entry borrows inputs and exit moves/releases owners as LG2.21 specifies. Predicate evaluation and every taken
  continuation keep their ordinary charges; the untaken branch costs nothing. Calls and closures retain DET.57's
  charges, with no extra implicit frame for the region. A denied entry, branch or exit has DET.56's code, counters
  and originating source locator. An admitted exit with no remaining steps does not fund the separate publication
  step. The form-2 equality and one-short vectors are separate from form 1; no old trace or key is relabelled.

  A predicate that selects a charged branch must yield the same display condition for a fixed key (`ApproxBool`
  in form 1, LG2.21's region-local condition in form 2). Its producing library
  must pin and implement the evaluation/rounding/comparison algorithm for that predicate across admitted executors.
  DET.32's tolerance for rendered coordinates does not permit a different branch, charge, exact field or stop.
  A route that establishes only approximate-coordinate tolerance has not established this predicate agreement and
  cannot claim the same reproducibility key for the branch. This requirement neither converts approximate values
  to exact decisions nor claims that a floating-point implementation has passed it.
  The same restriction applies to accounting sizes: allowable coordinate variation must preserve logical sizes,
  canonical output byte counts and unit boundaries. The pinned producer/profile must establish that property, for
  example through a common display quantization contract supplied by chapters 05 and 10. A variable-length number
  encoding plus coordinate tolerance alone does not establish it. No new encoding, padding rule or tolerance is
  invented here; a candidate without that agreement remains unverified for this key.

- **DET.65 [D]** The reference traces in [contract-vectors](../contract-vectors/README.md) are small synthetic
  accounting examples with declared test tariffs, input sizes and byte profiles. The checker recomputes charges,
  stops, counters and values without writing files. It is not a product executor, benchmark or cross-platform pass.
  The frozen LX-3 input `work/experiments/language-lx/lx3-sandbox-flow/corpus/profile.json` remains experiment evidence,
  not authority. This draft differs from its analysis meter: explicit handle release replaces retention to run end;
  logical language layout replaces JSON-sized bound values; variable arithmetic uses a separate input-size kernel
  tariff rather than limb-weighted steps; applicable flow checks are separate events rather than included
  bookkeeping; secondary resource requests are atomic after the step; children and semantic chunks have explicit
  rules. H7 supplies digest-bound source ranges rather than only neutral stop paths. The new vectors' declared
  retained-JSON test profile also differs from LX-3's neutral UTF-8 profile; neither selects p3's production profile.
  Neither that experiment's 2048-byte reserve nor its default limits are adopted as production values. Its
  instruction, memory and output counters must not be compared as if these were the same contract (RTE.40).

| Test | Checks | Class |
| --- | --- | --- |
| CT-DET-25 | Recompute all p4 finding-tag traces, charges, stop phases and final used counters with declared test tariffs (DET.54 to DET.65) | source |
| CT-DET-26 | Alias/drop, replacement peaks, frame unwind, moved child results and cache-independent owning handles agree across executors (DET.57, DET.58, DET.60) | source |
| CT-DET-27 | Exact equality admission, simultaneous failures, child/root precedence and denied secondary requests preserve the specified counters (DET.56, DET.60) | source |
| CT-DET-28 | Oversized arithmetic and text work use operand sizes; loop tests, assertions, flow checks, calls and returns retain their operations (DET.55, DET.62) | synthetic |
| CT-DET-29 | Generator initialization/draws and child stream paths match; denied draws do not consume the stream (DET.61) | source |
| CT-DET-30 | Fixed semantic streaming chunks keep coverage and charges under transport splits, cold caches and cancellation boundaries (DET.63) | source |
| CT-DET-31 | Canonical output equality boundaries, nested returns, retained partial output and terminal reserve cannot bypass byte accounting (DET.59) | source |
| CT-DET-32 | Both display forms charge only executed branches plus their specified boundary operations; form-2 equality and one-short limits determine the exit stop and source range. LG2.20 isolation, predicate/size agreement and exact accounting reproduction hold across routes (DET.56, DET.64) | source |

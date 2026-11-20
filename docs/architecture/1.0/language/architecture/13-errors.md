# Chapter 13: the error registry

Status: **frozen with the LP1 candidate** (owner sign-off, 15 November 2026;
[decision](../../../../wiki/decisions/owner-decisions-2026-11-15-freeze.md)). Conventions are in
[the overview](00-overview.md), section 3, and its status says what the freeze covers. Amended once under PHS.52 at
LP3's command-table freeze (owner, [16 November 2026](../../../../wiki/decisions/owner-decisions-2026-11-16-phs52.md)
and [20 November 2026](../../../../wiki/decisions/owner-decisions-2026-11-20-lp3-exit.md)); section 9 records it.

This chapter defines error identity, the error envelope and its categories. It collects the codes proposed by the
drafted chapters and reconciles their spellings with DT-L1 Round 3. It defines no implementation, concrete syntax or
interface presentation. Chapters 01 and 08 retain ownership of kernel results, commit checks and recovery.

Sources: [the partition](../ecosystem-partition.md), especially INV-1 to INV-12 and defaults 5 and 19;
[DT-L1 Round 3](../../../../progress/1.0/design-table/topics/language-ecosystem.md#round-3), engineer advisor item 2;
[V06, V03.3 and V05](../../10_V1_ARCHITECTURE.md), as proposals where marked;
[01 Kernel](01-kernel.md), [02 Language core](02-language-core.md), [03 Libraries](03-libraries.md),
[04 Environment](04-environment.md), [05 Persistence](05-persistence.md), [06 Evidence](06-evidence.md),
[07 Boundary](07-boundary.md), [08 One state](08-one-state.md), [09 Determinism](09-determinism.md),
[10 Renderer and platform independence](10-renderer-platform.md), [11 Route and spikes](11-route-and-spikes.md),
[12 Phases and conformance](12-phases-and-conformance.md);
[ENG-0J](../../../../progress/1.0/packets/engineering/ENG-0J-jumbling-backend.md), acceptance item 3 and section 4;
[DT-J1](../../../../progress/1.0/design-table/topics/jumble-solver-aids.md), A5 and its Outcome; and
[charter E6 and M3](../../../../progress/1.0/charter-2.0-draft.md);
[closure handoff](../closure/HANDOFF.md), H7 and H15;
[jumbling interfaces](../../../../progress/1.0/jumbling-interfaces.md),
[layer interfaces](../../../../progress/1.0/layer-interfaces.md) and
[renderer feature matrix](../renderer-feature-matrix.md).

## 1. Codes and ownership

- **ERR.1 [D]** This is the registry required by partition default 5. Its code form follows default 19 and DT-L1
  Round 3, item 2. The tables describe contracts for later implementation. They establish no conformance result or
  Windows, DirectX, input, long-session or performance evidence. The owner answered LQ1 to LQ21 on 6 November 2026 and
  chose LQ8, LQ9 and LQ21 on 14 November 2026: the engine and the interpreter's host in Rust, route RT1 with the
  runtime in a process of its own, and capability separation (RTE.17, RTE.21, RTE.24). LQ13 waits for its evidence,
  and LQ2's option set waits for the joint setting.
- **ERR.2 [D]** A stored code has the following form. Its source is the component that raises the failure, or
  `EVD` for the evidence pillar. A charter bar, contract amendment or research item belongs in Rule source, since
  several rules can support one code. The category equals the envelope's category.

  ```text
  Code = <source>:<category>.<slug>
  example: KN5:stale.revision_changed
  ```

- **ERR.3 [D]** A slug is lower-case snake case, starts with a letter, contains only letters, digits and separating
  underscores, and has at most 40 characters. It names the failed fact, never a remedy, selected twist or correction
  (DT-L1 Round 3; DT-J1 A5). The length is an unmeasured naming convention proposed in that repository record, not a
  measured capacity. A redundant `lbN_` part is removed when the source already identifies that library.
- **ERR.4 [D]** The short form `category.slug` is legal only in an `errors:` line in the raising component's
  section. There it expands with that component's prefix. `category.*` denotes all registered codes of that
  category from that component and needs no registry row of its own. With a prefix, as in `KN7:budget.*`, it denotes the
  same for the named component. An interface returning another component's error uses its full code or such a family,
  and a wildcard reaches at least one registered code. Section 6 records what replaced the drafts' provisional short
  spellings, in prose and code tables too (default 19; DT-L1 Round 3).
- **ERR.5 [D]** A code keeps its meaning for ever and is never reused. Retirement leaves the row, its meaning and
  its retirement edition in the registry. A proposal withdrawn before first use is marked as such, with no invented
  edition. It cannot become a different failure later (default 5; DT-L1 Round 3; V06).
- **ERR.6 [D]** Tests and handling assert `code`, never `summary` or `diagnostic`. Summaries may change and may
  be translated without changing a code. Text never selects a continuation (default 5, V06).
- **ERR.7 [D]** Forwarding preserves the originating code and its typed context. The prefix identifies the check
  that failed, rather than the outer service returning it. Thus an editor can return an LG2 diagnostic and a library
  can return a KN2 refusal without minting synonyms. A library's own contract check keeps its LB prefix. This follows
  from default 19's source identity and the component boundaries in partition sections 2 to 7.

## 2. The envelope

- **ERR.8 [D]** The envelope retains V06's fields below. Default 5 adopts that typed contract; V06's proposed wire
  presentation is not a frozen schema. Context contains typed facts appropriate to the row, such as the field, gate,
  identity and level, revision, capability and profile, or limit and actual amount. The prefix must agree with the
  reported raising component, and the code's category must agree with `category`.

  ```text
  Error = {
    code:        Code
    category:    Category
    recoverable: Bool
    retry:       never | after_refresh | after_user_change | after_reconnect | after_user_request
    context:     typed facts for this failure
    summary:     human-readable account
    diagnostic:  internal detail
    form:        diagnostic | refusal | stop | freeze | status
    span:        source range for a diagnostic, except ERR.10's encoding-error case
  }
  ```

- **ERR.9 [D]** `form` is an addition to V06 proposed by DT-L1 Round 3. It follows from default 5's language
  diagnostics and KN7.3's distinct job outcomes. It distinguishes where the code occurs, without redefining those
  outcomes. A registry row gives the primary form and may name contextual forms for the same failed fact. The
  envelope uses the form for the actual occurrence, with the code and meaning unchanged. For
  `EV3:busy.renderer_unavailable`, RM.22 gives `refusal` before acceptance and `stop` after acceptance but before
  installation, in `failed(Error)` and an associated job's `error(Error)`. Loss after installation changes display
  availability only: `completed(view_published)` and committed mechanical receipts remain terminal. A user stage
  stopped in a later frame has form `status` in RM.41's frame report, for the same reason.

| Form | Contract |
| --- | --- |
| `diagnostic` | Attached to source; the diagnosed candidate did not execute. The last valid module remains separate. |
| `refusal` | Returned to the caller; the refused operation changes no authoritative state or active durable record. |
| `stop` | A reason inside a job outcome or an accepted invocation's `failed(Error)`. KN7.3 owns job outcomes; LI.28 and LI.30 own receipt mapping and recovery. A reason never implies completion. |
| `freeze` | A refusal that also stops execution pending recovery. For an unknown durable outcome, reads continue in read-only recovery. |
| `status` | A reason in a frame or display report after a publication is terminal (RM.41). It changes no receipt, job outcome or authoritative state. |

- **ERR.10 [D]** `span` is a second addition to V06 from DT-L1 Round 3. It follows from partition EV1 and LG1:
  diagnostics identify the immutable source digest and the affected range. A diagnostic for an absent edition header
  can identify an insertion range. A service never attaches a diagnostic from the executed version to different draft
  bytes (EV1.1, EV1.7). A run-time refusal without a source position does not fabricate one.
  The sole encoding-error exception is `LG1:syntax.invalid_encoding`: it remains a diagnostic, but `span` is absent.
  Its typed context contains `raw_input_digest: Digest<bytes>`, `byte_start: Nat` and `byte_end: Nat`. The digest binds
  the exact received bytes before decoding or LF pinning; `[byte_start, byte_end)` is a nonempty, zero-based,
  half-open offending byte interval with `0 <= byte_start < byte_end <= raw_byte_length`. This context is not a
  `SourceRange`, requires no scalar boundaries and is never converted to host units. Neither replacement characters
  nor a fabricated validated-source range may stand for the rejected bytes.
- **ERR.11 [D]** `after_user_request` is an addition to V06's retry values from DT-L1 Round 3. It follows from V06's
  busy and cancellation continuations: a fresh user action may send the request again, with current guards and any
  newly required token. Nothing retries automatically. A retry value describes eligibility after its condition, not
  an instruction to replay a durable command. KN5.21 to KN5.23 and ONE.71 still govern its identity and receipt.
- **ERR.12 [D]** `span` consumes H7's `SourceRange` through EV1.10, including its pinned source digest, UTF-8
  byte offsets on scalar boundaries and half-open endpoints. EV1.10 owns validation and conversion from a declared
  host unit; chapter 10 consumes the same range. ERR.10's encoding-error exception has no `span`; its raw context
  never enters this range validation or conversion. For EV1.10's `a`, U+1F600, U+00E9, LF, `z` example, UTF-8 [1,5)
  maps to UTF-16 [1,3), and [9,9) is the EOF insertion at [6,6). Offset 2 in either unit is invalid. A different
  source digest returns `EV1:stale.editor_source`; an invalid endpoint or undeclared host unit returns
  `EV1:input.source_range_invalid`. This adopts the draft protocol, not a second coordinate definition or an LQ answer.
- **ERR.13 [B]** The data for a message states what happened, why, and what the solver can do (charter E6). It keeps
  evidence, score and permission separate (charter M3). A Jumble-off refusal names the failed fact and the kind of
  action available, never which correction or twist to choose (DT-J1 A5, Outcome Refusals; ENG-0J A5). Display rules
  below constrain this content only. They prescribe no appearance, layout or final wording.

## 3. Categories and their boundaries

- **ERR.14 [D]** The following table retains Round 3's sixteen categories: V06's ten and the six additions of
  default 5. Raising layers include the contract adapters used by the later chapters. An adapter's local type,
  capability or evidence refusal applies the owning rule; it gains no kernel authority. The protection row includes
  LB2.3's missing assurance, and the busy row includes RPI.64's renderer availability. Recoverable describes whether
  a corrected or renewed request is possible. It never authorizes an automatic retry.

| Category | Meaning | Recoverable; default retry | Required message data | Raising layer |
| --- | --- | --- | --- | --- |
| `input` | The request or file fails a requirement the caller can correct. The requested operation did not run. | yes; `after_user_change` | Failed field or fact; retained draft. | Engine, store, command layer, persistence; language/service input adapters |
| `unverified_reference` | References do not determine one correspondence. This establishes no mathematical impossibility. | yes; `after_user_change` | Missing disambiguating frame and the unresolved correspondence. | Engine |
| `protection` | A protected set would move, or requested protection assurance lacks checked coverage. It is a hard constraint. | yes; `after_user_change` | Protected set, level and failed or unchecked coverage; explicit protection change or a different operation as action kinds. | Engine; LB2 assurance entry |
| `stale` | A revision, preview, token, handle or result binding is no longer current. | yes; `after_refresh` | Changed binding and preserved inputs for explicit work. Passive hover/pick results are discarded without a message. | Command layer, engine; editor and display binding adapters |
| `cancelled` | A request or revision replacement stopped work. The cancelled work changed no state. | yes; `after_user_request` | Stop reason and coverage; no claim of emptiness. | KN7 |
| `resource_limit` | A product ceiling, machine limit or other limit outside the job's declared deterministic budget. | yes; `after_user_change` | Limit, unit and actual value where known; missing machine measurements remain absent. A machine stop is incomplete and not reproducible. | Engine, store, persistence; host admission adapters |
| `consistency` | The kernel's own invariants disagree. | no; `never` | Internal failure, recovery/report requirement and retained draft. Hashes belong only in `diagnostic`. | Kernel components KN1 to KN7 only |
| `unknown_outcome` | The fate of a durable command or publication cannot be established. | no; `never` | Unknown fate, read-only recovery and retained draft; no assertion that the operation failed. | Command layer, store; PS3 publication through the store |
| `auth` | A connection or token's authenticity is not established. | yes; `after_reconnect` | Authentication failure and the need to renew the engine link. | Command layer, platform boundary |
| `busy` | Another job/commit holds a lane, or the renderer service is unavailable during recovery. | yes; `after_user_request` | Active work or unavailable service; available wait/cancel action kinds. No blind replay. | Command layer, engine; renderer adapter |
| `syntax` | Source cannot be parsed, or its edition header is absent or unsupported. | yes; `after_user_change` | Source range (or ERR.10's encoding-error context) and parse/edition fact; identity of the retained valid version. | LG1, delivered through EV1 |
| `type` | A value violates puzzle, level, exactness, unit or mode requirements, or a partial result/verdict is mishandled. | yes; `after_user_change` | Expected and supplied types with levels, at the source range when available. | LG2; library/chart contract checks |
| `effect` | A performed or inherited effect is forbidden by the program's allowed context. | yes; `after_user_change` | Source range, forbidden effect and the call path that introduces it. | LG3 |
| `capability` | A required grant is absent or does not cover the requested use. | yes; `after_user_change` | Capability, effective profile and failed scope; any grant change belongs to the solver. | LG3 host, command layer; module, replay and provenance admission adapters |
| `budget` | A limit the job declared; the same key stops at the same place. | yes; `after_user_change` | Budget identity/unit, amount used and coverage. No unexamined domain is reported empty or negative. | KN7 and LG4; view work uses the same accounting |
| `evidence` | A required support/check fails or is absent, a premise is unmet, scope/digests differ, or stored evidence fields disagree. | yes; `after_user_change` | Failed support and its scope; the claim's status without that support. A failed check alone is no refutation. | KN6, evidence pillar; consuming contract checks in LG, LB, EV and PS |

- **ERR.15 [D]** Drafts survive every category (V06, DT-L1 Round 3). Input, frame selection and valid context are
  retained where still valid; a stale binding grants no permission. `summary` is understandable without internal
  hashes, indices or tokens. Those may appear in `diagnostic` under V06. No category supplies a solving suggestion,
  weakens protection, silently splits an operation or changes an alphabet (V05.2, default 10, DT-J1 A5).
- **ERR.16 [D]** A `budget` code requires both a limit declared for that job and a deterministic stopping point
  under the same key (INV-10, default 6, Round 3). A deterministic product ceiling remains `resource_limit`.
  A wall-clock safety stop is `KN7:resource_limit.safety_stop`, inside `cut_off(by: Code)`, incomplete and not
  reproducible (KN7.3, KN7.7, KN7.10). Physical exhaustion and runtime loss end jobs as error under KN7.13 and DET.43.
- **ERR.17 [D]** `capability` concerns authority granted to a program; `auth` concerns the authenticity of a
  connection or token. A valid program declaration is no grant. `effect` concerns a forbidden transitive effect
  found statically; `capability` checks the effective grant at load or run time (LG3.2, LG3.8 to LG3.10; Round 3).
  A static pass never replaces the engine's current checks.
- **ERR.18 [D]** A moved binding is `stale`. Equal authoritative revisions with unequal full-state digests are
  `consistency` when the kernel establishes that its own invariants disagree (KN5.16, ONE section 7, step 7).
  An invalid received file, cache entry or assembled bundle does not establish that fact. Its input is refused or
  refreshed under its own row. Only KN1 to KN7 may raise `consistency`; every such row has form `freeze`
  (DT-L1 Round 3; overview section 7).
- **ERR.19 [D]** A runtime fault is `KN7:resource_limit.runtime_lost`, a stop inside an error job outcome. It
  leaves the journal and state hash unchanged, the shell responsive and an engine-owned pending preview intact
  (KN7.13, ONE.70). It never freezes the session. `unknown_outcome` separately enters read-only recovery when
  durable fate is unresolved; a further forbidden request has form refusal, without establishing non-execution
  (KN5.23, ONE.70, PS3.9).
- **ERR.20 [D]** Replay and reproduction disagreements are evidence findings about the compared records or runs.
  They withhold the claimed match and freeze nothing in the session. EV2.4 compares replay against a record;
  DET.5, DET.6 and DET.52 compare admitted runs under a supported key. These remain separate codes because their
  comparison subjects differ. A kernel invariant disagreement discovered during either process independently uses
  the appropriate KN consistency code. This follows from Round 3's kernel-only boundary and INV-5.

## 4. Results without error codes

- **ERR.21 [D]** The following results remain on the registry's exclusion list, following Round 3 and the cited
  owning clauses. A stop code can explain an enclosing incomplete job; it does not turn its value into an error
  or an empty answer. A claim allowed to remain open is no missing-obligation diagnostic.

| Result | Owning rule and boundary |
| --- | --- |
| Off-lattice projection, partial coverage, `undefined(reason)` | KN0.4, KN1.9; DT-L1 R2-E4. These are typed partial results. Ignoring the partial case is `LG2:type.partial_ignored`. |
| `unknown(reason)` | KN0.5, INV-5. Insufficient evidence remains distinct from false and from an exception. |
| Admissible, blocked, invalid, uncertain; `derive`'s `Refused` value | KN2.8, KN2.11. These outcomes carry their certificates and change no authoritative state. An invalid request can separately have an input refusal. |
| Empty, not found within budget, not run | KN7.11, DET.30, default 6. Empty requires completed declared coverage; an incomplete search supplies no negative over its unexamined part. |
| Applied, display out of date | KN5.22, ONE.70, R-02. A committed receipt stays committed; display refresh failure is separate. |
| False step flag, lifted lattice pieces and other shown word facts | ENG-0J acceptance item 3; LB3.8. They are shown before commit. Admissibility and protection decide permission. |
| Missing chart coverage and `not_established` | RPI.37, RPI.48. Undefined display coordinates and an unknown property are typed results; falsely claiming a required check can be an evidence refusal. |
| Unmapped research label | PS3.10, default 20; DT-L1 R2-E1. Original text remains recorded, with no invented failed check. |
| `OUTSIDE_CERTIFIED_DOMAIN(k)` | LB5.4; partition 8.1 and 10.3. Completed rejection of membership in the stated D[k], with that scope. It says nothing about reachability or solvability. |

- **ERR.22 [D]** `input.lb5_outside_certified_domain`, in chapter 03's first text, misclassified its own completed
  membership answer. It has left the error registry. Its withdrawn prefixed spelling,
  `LB5:input.outside_certified_domain`, is recorded here only to prevent future allocation as an error. Section 6
  records that withdrawal as a removal, not a code substitution. A cut-off membership job returns unknown membership,
  never this completed rejection (LB5.4, INV-5, default 6).

## 5. Registry by source

- **ERR.23 [D]** Each table below has one raising source: its heading names the raising layer, and the code
  carries that component and its category (ERR.2). A blank Retry cell inherits section 3's category default.
  An explicit value overrides only retry eligibility, never the commit or recovery gates. Used by names the drafted
  chapters and clauses, including interfaces that forward the code. `Round 3; not yet used` retains an entry from
  the advisor's first list without pretending a chapter already emits it. Reserved and retired rows are not active.
  The retired flow spelling is marked retired before first use; this is no fifth envelope form. Rule source always
  identifies a requirement; a chapter's code table alone is not its justification (default 5). The 23 closure
  additions follow H15.16's final names and the originating clauses in the closure manifests. JI clauses stay rule
  sources, with the raising KN component in each code. JI.41's payload and projection failures reuse the existing EV3
  rows.
  The branch-path, void-exit and exposed-geometry input refusals identify their failed input fact under ERR.14;
  none asserts an unresolved frame correspondence or requests a disambiguating frame.
  KN3 supplies arithmetic rule clauses, with no registered KN3 raising family. Rotation-request codes belong to KN2.

- **ERR.24 [B]** The owner selected capability separation (owner, 14 November 2026, LQ21; RTE.24).
  `LG3:effect.flow_refused` is retired before first use and is never emitted, and `LG3:capability.flow_refused` is the
  flow-refusal code. Chapter 07 still owns the rule (BND.33). Capability separation rejects the combined authority.
  LQ2 still decides permitted flows, sinks and exceptions (BM2; LG3.13, LG3.14; DT-L1 R2-E3). Flow labels, static-only
  or with run-time labels, were not selected; the retired spelling records that unselected alternative and binds
  nothing.
- **ERR.25 [O]** Numeric plan ceilings, production tariffs and resource measurements remain unset here. The
  working draft uses RM.11 and RM.12's fixed ceilings with `EV3:resource_limit.display_plan_ceiling`, and RM.13's
  separate existing output allowance with `KN7:budget.output`. A count equal to the fixed ceiling passes that
  check; an excess carries the field, requested count and ceiling before allocation, with standalone refusal or
  already-accepted scheduled failure as RM.12/RM.20 specify. With 100 canonical output bytes remaining, a complete
  101-byte publication instead gives `cut_off(KN7:budget.output)`
  and `cancelled(budget)` under LI.28; none of it is emitted or installed. Physical exhaustion after acceptance is
  `failed(KN7:resource_limit.host_exhausted)`, not a budget cut-off. Engine adoption stays independent (RM.16).
  `KN7:budget.display_plan_limit` stays reserved for a separately declared deterministic producer plan budget if that
  alternative is later specified and approved; it never denotes a fixed renderer ceiling. RM.13 introduces no such
  allowance. No numeric limit is selected here.
- **ERR.26 [O]** `KN7:resource_limit.recursion_exhausted` is reserved for a recursion/stack guard outside the
  declared budget. DET.28 and LP1 choose the guard and counting rule. A logical stack charged solely to the declared
  memory allowance instead stops with `KN7:budget.memory`; a lower physical host limit is a host error. Both
  choices retain step metering and no negative from an incomplete job. No new recursion budget is introduced here.
- **ERR.27 [D]** Round 3's `KN7:budget.not_declared` and `KN7:budget.over_ceiling` remain as retired
  pre-use reservations. A missing declaration is `KN7:input.budget_not_declared`; a rejected host/profile ceiling
  is `KN7:resource_limit.budget_ceiling`. Neither has an admitted job's exhausted allowance, so neither can pass
  ERR.16's budget rule. No retirement edition has been assigned because none has been used.

### KN1: model registry and identity

| Code | Form | Meaning | Retry | Rule source | Used by |
| --- | --- | --- | --- | --- | --- |
| `KN1:input.unknown_model` | refusal | Identity is absent from the model registry. | | KN1.1, KN1.4 | 01 KN1 model_info |
| `KN1:input.identity_mismatch` | refusal | Model, menu, order digest or contract revision differs; context identifies the field. No reinterpretation or empty answer. | | INV-7; KN1.4; ENG-0J item 3 | 01 KN2.11; 02 LG2.13; 03 LB1.3, LB3.8; 04 EV1.2, EV2.3, EV3.5; 05 PS1.18, PS2.2, PS3.8; 08 ONE.22, ONE.25 |
| `KN1:input.level_not_held` | refusal | Requested level is not held by this puzzle/input. | | KN1.7, KN1.9 | 01 KN1.9; 08 ONE.25 |
| `KN1:consistency.identity_disagrees` | freeze | Kernel parts of the same running session name different identities. | | INV-7; Round 3 | Round 3; not yet used |
| `KN1:input.unsupported_dimension` | refusal | Header/request dimension is not 4 in this edition; refuse before reading arrays or changing state. | `after_user_change` | JI.3, JI.45; H15.16 | JI.3 |

### KN2: exact engines

| Code | Form | Meaning | Retry | Rule source | Used by |
| --- | --- | --- | --- | --- | --- |
| `KN2:stale.revision_gone` | refusal | The requested journal event/revision is no longer held. | | KN2.2; ONE.17 | 01 KN2.2, KN4.3; 04 EV2.1, EV2.8; 08 ONE.17 |
| `KN2:stale.epoch_changed` | refusal | A transient handle or preview belongs to an ended engine epoch. A durable RevisionRef can still be resolved. | | ONE.17, ONE.43 | 08 section 15 |
| `KN2:input.rotation_not_representable` | refusal | Requested rotation has no permitted exact representation. | | ENG-0J item 3; INV-4 | Round 3; not yet used |
| `KN2:input.twist_off_menu` | refusal | Requested twist is outside the signed menu. | | ENG-0J item 3; KN3.8 | 01 KN3.8 |
| `KN2:input.rotation_request_malformed` | refusal | Rotation request bytes fail the exact request format, canonical form or size bound. Nothing is repaired or rounded. | `after_user_change` | KN3.7 | 01 KN3.7 |
| `KN2:input.menu_binding_mismatch` | refusal | A menu letter or drag names a menu set or order digest other than the pinned menu's, or an M request arrives in free mode, which pins no menu. | `after_user_change` | KN3.7, KN3.8 | 01 KN3.7, KN3.8 |
| `KN2:resource_limit.rotation_search_bound` | refusal | The bounds N and D of a free-mode request are outside the pinned search limit. No search runs and no choice is made. | `after_user_change` | KN3.7 | 01 KN3.7 |
| `KN2:input.menu_invalid` | refusal | Menu fails inverse closure or A4 invariance. | | ENG-0J item 3 | Round 3; not yet used |
| `KN2:input.word_blocked_prefix` | refusal | A stored word has a blocked prefix at the current input; context gives the prefix length. | | ENG-0J item 3; ONE.33 | Round 3; M5's jumbling adapter for a refused word; M8's derive for a refused letter |
| `KN2:input.recipe_shape` | refusal | Recipe violates the retained word/star step shape or step range. | | Partition section 13; PS1.22 | Round 3; LP2's kernel recipe admission; the command layer's inline recipe |
| `KN2:unverified_reference.frame_ambiguous` | refusal | Ordered references do not fix one proper correspondence. | | V06 | Round 3; not yet used |
| `KN2:protection.orbit_would_move` | refusal | Label-level protected orbit would move under the active protection contract. | | V06; KN1.11 | Round 3; 01 KN5.16 and 08 commit step 8 use the family |
| `KN2:protection.piece_would_move` | refusal | Pose-level protected piece would move. No label-level substitution. | | KN1.11; LB3.4 | Round 3; 01 KN5.16 and 08 commit step 8 use the family |
| `KN2:resource_limit.primitive_ceiling` | refusal | Total retained-generator primitive count exceeds the compatibility ceiling, regardless of chunks. | | V05.1; LB2.4; PS1.10 | Round 3; LP2's kernel recipe expansion |
| `KN2:resource_limit.word_length_bound` | refusal | Menu-word replay exceeds the X3 admission bound; exact length retained. Bound remains unset here. | | ENG-0J X3; KN2.10 | Round 3; not yet used |
| `KN2:consistency.fixed_nu_nonzero` | freeze | A reachable lattice result under signed I_a has a nonzero fixed ν coordinate. | | ENG-0J item 3 and A5; DT-J1 Outcome fix-first 5 | 03 LB3.1 catalogue, section 11 |
| `KN2:input.named_object_invalid` | refusal | Named cap/piece is missing or out of range, or a unit is empty, mixed-pose or contains a lattice piece. No alternate object is selected. | `after_user_change` | JI.9, JI.45; H15.16 | JI named-object queries |
| `KN2:input.exposed_geometry_unavailable` | refusal | Certified exposed-face geometry or its size profile is absent. Context names the feature, source binding and missing certified data/profile; report it unavailable and fabricate no mesh. | `after_user_change` | JI.41, JI.45; H15.16 | JI.41; renderer feature matrix |
| `KN2:stale.value_unavailable` | refusal | A saved derived-value reference cannot resolve its immutable reconstruction inputs; no guessed state or changed reachability claim. | `after_refresh` | LI.13 | LI value-store reads |
| `KN2:consistency.star_certificate` | freeze | A star certificate of the loaded model fails one of 0.4's checks (seed definition, guarded buffer position, transported star frame); no result is published from that star. | `never` | KN2.1, KN2.4 | 01 KN2.4 rows; 08 preview and commit |
| `KN2:input.named_cap_blocked_or_closed` | refusal | The cap a repair list names is blocked or uncertain, or closed (N_c = 0), so ENG-0J's unblocked or nonclosed premise fails (JI.12). No other cap is taken. | `after_user_change` | JI.12, JI.45 | JI repair list; 01 KN2.4 `repair_list` |
| `KN2:input.menu_order_unpinned` | refusal | A repair list is asked for in free mode, which pins no menu, so its rows have no pinned order (JI.12, JI.15). No order is supplied in its place. | `after_user_change` | JI.12, JI.15, JI.45 | JI repair list; 01 KN2.4 `repair_list` |

### KN4: session store

| Code | Form | Meaning | Retry | Rule source | Used by |
| --- | --- | --- | --- | --- | --- |
| `KN4:input.unknown_session` | refusal | No such session exists. | | KN2.2, KN4.1 | 01 KN2.2 |
| `KN4:input.checkpoint_unwitnessed` | refusal | Checkpoint has neither a legal witness nor a label certificate. | | ENG-0J item 3; PS1.29 | Round 3; not yet used |
| `KN4:resource_limit.disk_full` | refusal | A durable write could not be completed; no half-published state. | | R-06; KN4.2 | Round 3; not yet used |
| `KN4:input.branch_path_unproved` | refusal | Target branch lacks a proved journal navigation path. Context names the target and missing path evidence; retain the current binding. An explicit validated restore instead uses the exit-void rule. | `after_user_change` | JI.27, JI.28, JI.45; H15.16 | JI branch navigation |
| `KN4:input.session_exists` | refusal | A new session is begun in a store that already holds one; the held journal is kept and nothing is written. | `after_user_change` | KN4.1, KN4.2 | Command layer's session start; M6 session store |
| `KN4:busy.session_locked` | refusal | Another opener holds the session lock, so the store does not open the session; nothing is read or written. | `after_user_request` | KN4.1, KN4.2; owner, 18 November 2026, item 3 | M6 session store open |

### KN5: command layer

| Code | Form | Meaning | Retry | Rule source | Used by |
| --- | --- | --- | --- | --- | --- |
| `KN5:stale.revision_changed` | refusal | Preview base no longer equals the current revision at commit. | | KN5.16; ONE commit step 6 | 08 section 15 |
| `KN5:stale.preview_replaced` | refusal | Genuine token names a preview other than the pending one. | | KN5.11; ONE commit step 5 | 08 section 15 |
| `KN5:stale.token_mismatch` | refusal | Genuine token's row, arguments, literal digest or identity differ from its binding. | | INV-3; ONE commit step 5 | 08 section 15 |
| `KN5:stale.token_used` | refusal | A single-use token has already been seen. | | KN5.15; ONE commit step 3 | 08 section 15 |
| `KN5:stale.no_fresh_preview` | refusal | Commit action has no fresh presented preview. | | ONE.39, ONE.41; commit step 1 | 08 section 15 |
| `KN5:stale.undo_superseded` | refusal | Another durable action superseded the action named by the undo hint. | | KN5.19 | Round 3; not yet used |
| `KN5:auth.token_invalid` | refusal | Token authenticator fails; replaying the action cannot repair authenticity. | | KN5.15; V06; ONE commit step 3 | 01 KN5 commit; 08 section 15 |
| `KN5:auth.session_credential` | refusal | Local engine credential is invalid. | | V06 | Round 3; not yet used |
| `KN5:busy.commit_in_flight` | refusal | Another commit holds the commit lane. | | KN5.17 | 01 KN5 commit; 08 commit step 4 |
| `KN5:input.command_id_reused` | refusal | A durable command ID is reused with different request content. | | KN5.21 | Round 3; not yet used |
| `KN5:input.overlay_row_kind` | refusal | Language overlay attempts to register a forbidden row kind. | | KN5.3 | Round 3; not yet used |
| `KN5:input.jumble_off_pieces_off_lattice` | refusal | Jumble off requested while pieces are off the lattice. | | ENG-0J A5; DT-J1 A5 | Round 3; not yet used |
| `KN5:input.jumble_off_nu_nonzero` | refusal | Jumble off requested with nonzero ν. No correction is selected. | | ENG-0J A5; DT-J1 A5 | Round 3; not yet used |
| `KN5:input.jumble_off_not_shown_reachable` | refusal | Required journal/reachability premise for the crossing is absent. This establishes no unreachability claim. | | ENG-0J A5; ONE.54 | Round 3; not yet used |
| `KN5:input.jumble_on_labels_only` | refusal | Labels supplied without the required pose witness/replay chain. | | ENG-0J A5; ONE.54 | Round 3; not yet used |
| `KN5:input.retained_tool_needs_jumble_off` | refusal | Retained workflow tool requested with Jumble on. | | ENG-0J item 7; LG2.8 | Round 3; not yet used |
| `KN5:input.menu_word_needs_jumble_on` | refusal | Session menu-word operation requested with Jumble off. | | ENG-0J item 7; PS1.21 | Round 3; not yet used |
| `KN5:input.menu_change_in_episode` | refusal | Attempt to change the pinned menu inside an episode. | | ENG-0J section 4 | Round 3; not yet used |
| `KN5:capability.commit_row_refused` | refusal | Runtime attempts to invoke a commit row. | | ONE.3, ONE.4 | 07 BND.83; 08 ONE.67, section 15 |
| `KN5:consistency.state_hash_mismatch` | freeze | Equal authoritative revisions disagree in their full-state digests. | | KN5.16; ONE commit step 7 | 08 section 15 |
| `KN5:unknown_outcome.commit_fate` | freeze | No authoritative receipt/continuity/recovery record establishes durable fate. | | KN5.23; V03.5 | Round 3; 01 KN5 commit and 08 commit step 9 use the family |
| `KN5:unknown_outcome.read_only_recovery` | refusal | Proposal or commit requested while durable-outcome recovery allows only reads. | | KN5.23; ONE.70 | 08 section 15 |
| `KN5:input.confirmation_required` | refusal | The normal confirm gate lacks explicit scope confirmation, including undo that releases protection; perform no action. | `after_user_change` | LI.26 | LI undo and confirmation gates |
| `KN5:input.overlay_dependency_missing` | refusal | A required kernel row is absent from the supplied row table; overlay availability names that missing dependency. | `after_user_change` | LI.20 | LI overlay availability |
| `KN5:input.scramble_policy_unset` | refusal | No owner-approved product scramble policy reference exists; keep the command unavailable and construct no scramble. | `after_user_change` | JI.38, JI.45; H15.16 | JI jumble.scramble availability |
| `KN5:input.exit_record_voided` | refusal | A history action would reactivate a known void exit binding. Context names the exit and durable void fact; refuse that redo. A fresh validated crossing or solved-witness entry is a separate action. | `after_user_change` | JI.27, JI.45; H11, H15.16 | JI crossing/entry redo |
| `KN5:stale.filter_preview` | refusal | The submitted FilterPreviewRef fails H15.4's filter_fresh predicate. Retain inputs for an explicit fresh filter preview; no pending word-preview slot or presentation is required. | `after_refresh` | H15.4; H15.16; KN0.9 | filter.apply fresh_filter_preview gate |
| `KN5:stale.result_gone` | refusal | A known terminal's result payload is no longer available. Keep the receipt unchanged and do not rerun the command. | `after_refresh` | LI.29 | LI result retrieval |
| `KN5:resource_limit.engine_lost` | stop | After an engine crash, complete recovery proves an accepted durable invocation rolled back without publication; lookup returns failed(Error). Missing proof remains unknown_outcome. | `after_user_request` | LI.30 | LI durable outcome lookup; a new action needs a new ID and current guards |
| `KN5:input.unknown_row` | refusal | The request names a row that the command table does not hold. Nothing runs. | `after_user_change` | KN5.1; LI.5 | KN5.1 invoke; row serving |
| `KN5:input.argument_form` | refusal | The request's arguments do not have the row's argument form: a name, type or value differs. Nothing runs. | `after_user_change` | KN5.1; LI.5 | KN5.1 invoke; 01 KN2.4 host bridge reads |
| `KN5:input.row_not_built` | refusal | The table holds the row, but this build does not serve it or lacks what it needs, such as the host's pinned twist menu; the row stays unavailable. Nothing runs. | `after_user_change` | KN5.6; KN3.8 | KN5.1 invoke; KN5.6 availability |
| `KN5:input.overlay_row_conflict` | refusal | A served or overlay row takes an ID that a row or service already holds, that the layer builds itself, or that another entry already serves. The table is unchanged. | `after_user_change` | KN5.3, KN5.4 | KN5.3 overlay and row serving |
| `KN5:input.saved_set_exists` | refusal | `set.save` names a saved set without `replace: true`; the saved set is kept. | `after_user_change` | U1-057 | `set.save` |
| `KN5:consistency.unadmitted_journal_action` | freeze | A reopen's replay meets a journal action that no solver action could have made: a commit whose caller is not the shell, a commit that moves a protected orbit under the protection and policy then in force, or a protection release without its confirmation. The session opens in read-only recovery before that action. | `never` | ONE.3, ONE.4; KN1.11; LI.26; KN5.23; owner, 18 November 2026, item 3 | Session reopen replay |
| `KN5:capability.preview_shown` | refusal | A run asks for `operation.preview` or `operation.preview.cancel` while the solver is shown a preview; only the solver's own action ends a shown preview. | `after_user_change` | KN5.12; ONE.42; owner, 18 November 2026, item 1 | KN5 run admission |
| `KN5:capability.outside_solver_command` | refusal | A run asks for `operation.preview` outside a command that the solver invoked; a preview is prepared only inside one. | `after_user_change` | ONE.34; owner, 18 November 2026, item 1 | KN5 run admission |

### KN6: independent checkers

| Code | Form | Meaning | Retry | Rule source | Used by |
| --- | --- | --- | --- | --- | --- |
| `KN6:evidence.certificate_rejected` | refusal | Certificate fails to support its claim for the stated inputs. | | KN6.1, KN6.6 | Round 3; 01 KN6.1 uses the family |
| `KN6:evidence.no_checker` | refusal | No checker is available for this certificate kind. | | KN6.1 | Round 3; 01 KN6.1 uses the family |
| `KN6:evidence.start_table_mismatch` | refusal | Certificate/check request supplies the wrong start table. | | KN6.5 | Round 3; 01 KN6.1 uses the family |

### KN7: jobs and budgets

| Code | Form | Meaning | Retry | Rule source | Used by |
| --- | --- | --- | --- | --- | --- |
| `KN7:budget.steps` | stop | Declared interpreter step allowance ends; job is cut off. | | KN7.7, KN7.10; DET.18 to DET.20 | 01 KN2.11, KN7.10; 09 DET.7, DET.52 |
| `KN7:budget.kernel_cost` | stop | Declared kernel allowance cannot admit further work. | | KN7.7, KN7.8, KN7.10 | 01 KN2.11, KN7.10; 09 DET.7, DET.52 |
| `KN7:budget.memory` | stop | Declared logical memory allowance ends. | | KN7.7, KN7.9, KN7.10 | 01 KN2.11, KN7.10; 09 DET.7, DET.52 |
| `KN7:budget.output` | stop | Declared logical output allowance ends: job cut_off with this code and accepted invocation cancelled(budget). Emit/install none of the denied complete unit; retain prior charges and the terminal reserve. | | KN7.7, KN7.10; DET.59; LI.28; RM.13 | 01 KN2.11, KN7.10; 09 DET.7, DET.52; renderer publication |
| `KN7:budget.display_plan_limit` | stop | Reserved: a separately declared deterministic producer plan budget ends; install no incomplete publication. It never reports a fixed renderer ceiling, which uses EV3:resource_limit.display_plan_ceiling under RM.12/RM.20. The fixed-ceiling/output-budget draft does not emit this code. | | ERR.25; RPI.49, RPI.50; RM.13 | Declared-plan-allowance alternative; activation pending |
| `KN7:budget.not_declared` | refusal | Retired before first use: Round 3's missing-budget proposal. Use the input successor below. | | Round 3; ERR.16, ERR.27 | Round 3; never active; edition unassigned |
| `KN7:budget.over_ceiling` | refusal | Retired before first use: Round 3's budget-admission ceiling proposal. Use the resource-limit successor below. | | Round 3; ERR.16, ERR.27 | Round 3; never active; edition unassigned |
| `KN7:input.budget_not_declared` | refusal | Admission has no explicit deterministic budget. | | KN7.7; ERR.27 | Round 3 successor; no chapter caller yet |
| `KN7:resource_limit.budget_ceiling` | refusal | Requested allowance exceeds a host/profile ceiling before admission. | | BM3; ERR.16, ERR.27 | Round 3 successor; no chapter caller yet |
| `KN7:resource_limit.safety_stop` | stop | Wall-clock safety stop; cut off, incomplete and not reproducible. | | INV-10; KN7.10 | 01 KN7.10; 09 DET.17, DET.18 |
| `KN7:resource_limit.runtime_lost` | stop | Runtime died; job outcome error, session and pending engine preview preserved. | | KN7.13; ONE.70 | 09 DET.43, DET.52 |
| `KN7:resource_limit.host_exhausted` | stop | Physical resources or a host safety mechanism ended the job as error; accepted preparation returns failed(Error). No deterministic cut-off claim. | | DET.17, DET.43; LI.28; RM.12 | 09 DET.52; renderer preparation |
| `KN7:resource_limit.recursion_exhausted` | stop | Reserved: non-budget recursion/stack guard reached, under the selected DET.28 contract. | | ERR.26; DET.28 | 09 DET.36, DET.52; guard pending |
| `KN7:cancelled.by_user` | stop | Solver requested cancellation before durable commit. | | V06; KN7.6 | Round 3; not yet used |
| `KN7:cancelled.revision_replaced` | stop | Commit or undo replaced the input revision. | | KN5.18; ONE.49 | Round 3; not yet used |
| `KN7:busy.lane_full` | refusal | Lane has no admission slot. | | KN7.1, KN7.2 | Round 3; not yet used |
| `KN7:stale.job_input_digest` | refusal | Job result is requested against a different input digest. | | KN7.3; ENG-0J item 3 | Round 3; not yet used |

### LG1: syntax and editions

| Code | Form | Meaning | Retry | Rule source | Used by |
| --- | --- | --- | --- | --- | --- |
| `LG1:syntax.unexpected_token` | diagnostic | Parsing failed at a token or required structure; recovery retains locations. | | LG1.2, LG1.5 | 02 parse, section 6; Round 3 |
| `LG1:syntax.edition_missing` | diagnostic | No source edition header selects the rules. | | LG1.1, LG1.2 | 02 parse, section 6; Round 3 |
| `LG1:syntax.edition_unsupported` | diagnostic | Source requests an unsupported edition. | | LG1.2 | 02 parse, section 6; Round 3 |
| `LG1:syntax.invalid_encoding` | diagnostic | Source bytes violate the UTF-8/pinned-byte input contract. ERR.10 supplies the raw input context with span absent. | | LG1.3; ERR.10, ERR.12 | 02 parse, section 6 |
| `LG1:syntax.invalid_literal` | diagnostic | Literal has no value of its declared kind. | | LG1.6, LG1.7 | 02 parse, section 6 |
| `LG1:syntax.recovery_gap` | diagnostic | Recovered missing/invalid source supplies no executable value. | | LG1.5; EV1.2 | 04 EV1.2, EV1.4 |

### LG2: types

| Code | Form | Meaning | Retry | Rule source | Used by |
| --- | --- | --- | --- | --- | --- |
| `LG2:type.mismatch` | diagnostic | Expression, edit or declared parameter has an incompatible type. | | LG2.1; EV1.4, EV3.7 | 04 EV1.2, EV1.6, EV3.7; Round 3 |
| `LG2:type.identity_mismatch` | diagnostic | Argument/comparison uses a different puzzle or model identity. | | LG2.2 | 02 section 6 |
| `LG2:type.puzzle_mismatch` | diagnostic | A Full piece is supplied as a jumbling piece, or conversely. | | LG2.2; Round 3 | Round 3; not yet used as a concrete code |
| `LG2:type.level_mismatch` | diagnostic | Wrong identity-tower level or partial projection treated as total. | | LG2.3 | 02 section 6; Round 3 |
| `LG2:type.exact_required` | diagnostic | An approximate value is supplied at an exact value site. | | LG2.4; EV1.4; Round 3 | 04 EV1.2, EV1.4 |
| `LG2:type.exactness_violation` | diagnostic | Approximate data or a dependent condition reaches exact computation, including indirect control flow. | | LG2.4, LG2.5 | 02 section 6 |
| `LG2:type.cost_unit_mix` | diagnostic | Puzzle or cost-unit indices of quantities do not agree. | | LG2.6 | 02 section 6; Round 3 |
| `LG2:type.reachability_missing` | diagnostic | A state lacks its required named-puzzle reachability claim. | | LG2.7 | 02 section 6; Round 3 |
| `LG2:type.mode_mismatch` | diagnostic | Session operation requires the other mode. | | LG2.8 | 02 section 6; Round 3 |
| `LG2:type.verdict_as_boolean` | diagnostic | Evidence-bearing verdict is read as a boolean. | | LG2.9; Round 3 | Round 3; not yet used as a concrete code |
| `LG2:type.verdict_required` | diagnostic | A decision drops unknown or omits required verdict handling, including misuse as a boolean. | | LG2.9 | 02 section 6 |
| `LG2:type.partial_ignored` | diagnostic | Partial projection is used without its off-lattice/undefined case. | | KN0.4; LG2.3, LG2.9 | Round 3; not yet used |
| `LG2:type.word_key_not_executable` | diagnostic | An equality key is supplied where literal execution data is required. | | LG2.13, LG2.17 | 02 section 6 |
| `LG2:type.chart_subset_violation` | diagnostic | Withdrawn before first use: chart evaluator violates the former checked subset/bounds. Replaced by `LG2:type.stage_rule_violation`; this code keeps its old meaning. | | Former RPI.32, RPI.33, RPI.48; ERR.5 | 10 former chart admission; never active; edition unassigned |
| `LG2:type.stage_rule_violation` | diagnostic | A stage module breaks a stage rule or bound. | `after_user_change` | LG2.24; RPI.69 to RPI.71 | 02 LG2.24; 10 stage admission |

### LG3: effects and capability host

| Code | Form | Meaning | Retry | Rule source | Used by |
| --- | --- | --- | --- | --- | --- |
| `LG3:effect.undeclared` | diagnostic | Transitive effect exceeds the allowed effect set of its declaration/context. | | LG3.2, LG3.5, LG3.6; Round 3 | 02 section 6; 04 EV1.2, EV1.4, EV3.2; 07 BND.83 |
| `LG3:effect.view_writes_workspace` | diagnostic | View has a workspace-write effect through its body or callees. | | Default 16; LG3.5 | Round 3; not yet used as a concrete code |
| `LG3:effect.flow_refused` | refusal | Retired before first use: the flow-label spelling, not selected (LQ21; RTE.24). Never emitted; use `LG3:capability.flow_refused`. | | BM2; ERR.24; Round 3 | 02 LG3.12, LG3.14, section 6; 07 BND.83, CT-BND-24; never active; edition unassigned |
| `LG3:capability.flow_refused` | refusal | The flow-refusal code under capability separation (LQ21; RTE.24). The rule (BND.33) is chapter 07's; LQ2 decides which flows it refuses. | | BM2; ERR.24; Round 3 | 02 LG3.13, LG3.14, section 6; 07 BND.83, CT-BND-24 |
| `LG3:capability.not_granted` | refusal | Requested authority is absent or barred by the effective sandbox/profile. | | LG3.8, LG3.9; BND.84, BND.85; DET.36, DET.37, DET.42 | 01 KN5.26; 02 section 6; 07 BND.61, BND.83 to BND.85; 09 DET.37, DET.52; Round 3 |
| `LG3:capability.ambient_read` | refusal | Clock, file or network read requested without a grant; network remains barred in 1.0. | | INV-10; default 2; LX-3 | Round 3; the checker's admission, its built-in column and LP3's approval of a built-in tool (owner, 16 November 2026); LX-3 P01 to P03 |
| `LG3:capability.grant_mismatch` | refusal | Resource, right, module or profile differs from the effective grant. | | DET.37, DET.40 | 07 BND.83, CT-BND-23; 09 DET.7, DET.52 |

### LG4: interpreter and run host

| Code | Form | Meaning | Retry | Rule source | Used by |
| --- | --- | --- | --- | --- | --- |
| `LG4:input.ir_rejected` | refusal | IR fails validation at load. | | LG4.1, LG4.2; Round 3 | Round 3; not yet used |
| `LG4:input.ir_version_unsupported` | refusal | Executor cannot preserve the requested version's IR contract. | | LG4.1 | 02 section 6 |
| `LG4:input.repro_input_missing` | refusal | Required source, module, snapshot, file bytes or implementation identity is unavailable; reproduction is not run. | | DET.1, DET.7 | 09 DET.7, DET.52 |
| `LG4:input.repro_key_mismatch` | refusal | Supplied bytes, versions, identity or parameters disagree with the requested key. | | DET.7 | 09 DET.7, DET.52 |
| `LG4:evidence.reproducibility_mismatch` | refusal | Admitted executions of one supported key differ in exact payload or deterministic coverage; no session freeze. | | DET.5, DET.6; ERR.20 | 09 DET.52 |
| `LG4:input.cache_entry_mismatch` | refusal | Candidate cache entry fails its recorded input/content checks; reject reuse and treat as a miss with unchanged charges. | `after_refresh` | DET.16, DET.46 | 09 DET.46, DET.52; host envelope only |
| `LG4:input.pointwise_failed` | stop | An admitted pointwise assertion is false; end the run as error before its guarded operation and retain admitted charges. | `after_user_change` | DET.55, DET.56 | LG4 assertions |
| `LG4:input.generator_input` | refusal; stop in an admitted run | Seed size or scalar stream path/count/length violates the declared generator contract; no generator is bound. | `after_user_change` | DET.61 | LG4 rng_init |
| `LG4:resource_limit.generator_exhausted` | stop | An admitted draw requests a counter after the last defined block; end as error without wrapping or repeating a block. | `after_user_change` | DET.61 | LG4 rng_draw |
| `LG4:input.text_scalar_invalid` | refusal; stop in an admitted run | Invalid UTF-8 or a surrogate would enter Text; no invalid value is bound. | `after_user_change` | DET.62 | LG4 Text input/decoding |

### LG5: modules

| Code | Form | Meaning | Retry | Rule source | Used by |
| --- | --- | --- | --- | --- | --- |
| `LG5:input.module_digest_mismatch` | refusal | Module bytes differ from the expected content/lockfile digest. | | LG5.1, LG5.3 | 02 section 6; Round 3 |
| `LG5:input.model_not_supported` | refusal | Manifest does not list the model identity. | | LG5.2, LG5.5 | Round 3; not yet used |
| `LG5:input.module_dependency_missing` | refusal | Content required by the locked dependency closure is unavailable. | | LG5.3 | 02 section 6 |
| `LG5:input.module_contract_mismatch` | refusal | Manifest, source editions or dependency contracts disagree. | | LG5.2, LG5.3, LG5.7 | 02 section 6 |

### LB1: base library

| Code | Form | Meaning | Retry | Rule source | Used by |
| --- | --- | --- | --- | --- | --- |
| `LB1:input.incompatible_alphabet` | refusal | Word composition/application mixes puzzle alphabets or pinned letter meanings. | | LB1.3, LB1.9 | 03 LB1 literal words catalogue, LB3.8 |
| `LB1:input.key_contract_mismatch` | refusal | Key kind, rewrite version, alphabet/identity/order or independence contract differs; return no equality or ordering result. | `after_user_change` | LG2.23 | LB1 word-key comparison |

### LB2: retained protection assurance

| Code | Form | Meaning | Retry | Rule source | Used by |
| --- | --- | --- | --- | --- | --- |
| `LB2:protection.incomplete_coverage` | refusal | Requested protection assurance lacks full executed-prefix coverage; issue no safe verdict and do not claim motion was proved. | | LB2.3 | 03 LB2 catalogue, section 11 |

### LB4: exact representation checks

| Code | Form | Meaning | Retry | Rule source | Used by |
| --- | --- | --- | --- | --- | --- |
| `LB4:type.approximate_property_input` | refusal | Approximate display data supplied to an exact chart-property check. | | LB4.1, LB4.2 | 03 LB4 catalogue, section 11 |

### LB5: research contracts

| Code | Form | Meaning | Retry | Rule source | Used by |
| --- | --- | --- | --- | --- | --- |
| `LB5:evidence.reduction_scope_mismatch` | refusal | Reduction lemma has another family, alphabet, hypotheses or conclusion; no negative transport. | | LB5.3 | 03 LB5 catalogue, section 11 |
| `LB5:input.obligation_untranslatable` | refusal | A statement has no typed body or uses something outside the versioned obligation vocabulary. Nothing is written. | `after_user_change` | LB5.18, PS1.46 | 03 LB5.18 |

### LB6: built-in tool admission

| Code | Form | Meaning | Retry | Rule source | Used by |
| --- | --- | --- | --- | --- | --- |
| `LB6:evidence.principle_record_incomplete` | refusal | Required PrincipleRecord part or resolving evidence/fixture reference is absent. | | INV-12; LB6.1, LB6.2; DT-L1 R2-E4 | 03 LB6.2, section 11 |
| `LB6:evidence.decomposition_mismatch` | refusal | Submitted tool graph omits a derived transitive call/effect or differs from its bound source/build inputs; withhold tool admission. | `after_user_change` | LB6.6; RTE.80 | 03 LB6.6; 11 RTE.80 (non-IR RT4 admission under LQ9) |

### LB7: module entry admission

| Code | Form | Meaning | Retry | Rule source | Used by |
| --- | --- | --- | --- | --- | --- |
| `LB7:capability.module_grant_missing` | refusal | A module entry's required effect lacks its host grant. | | LB7.1, LB7.5 | 03 LB7.1, section 11 |

### EV1: editor services

| Code | Form | Meaning | Retry | Rule source | Used by |
| --- | --- | --- | --- | --- | --- |
| `EV1:stale.editor_source` | refusal | Response or proposed edit names source bytes no longer equal to its target. | | EV1.1, EV1.6 | 04 EV1.2, EV1.5, EV1.6, EV3.5 |
| `EV1:evidence.obligation_unmet` | diagnostic | Operation requires a claim/check that is absent; an inspectable open claim alone is no error. | | EV1.4 | 04 EV1.2, EV1.4 |
| `EV1:input.source_range_invalid` | refusal | Invalid endpoint, undeclared host unit or invalid conversion on the pinned UTF-8/LF scalar boundaries; no rounded range. | `after_user_change` | EV1.10; ERR.12 | EV1 source services; SourceMap consumers |
| `EV1:input.module_not_checked` | refusal | Activation lacks a successful check of exactly the candidate and its dependency closure; keep the active version. | `after_user_change` | EV1.11 | EV1 activate_module |
| `EV1:stale.activation_base` | refusal | Atomic activation finds an active digest other than expected_active_digest; keep the active version. | `after_refresh` | EV1.11 | EV1 activate_module |

### EV2: debugger and recorded replay

| Code | Form | Meaning | Retry | Rule source | Used by |
| --- | --- | --- | --- | --- | --- |
| `EV2:input.replay_dependency_missing` | refusal | Required replay digest, edition, IR version or trace input cannot be resolved. | | EV2.1, EV2.3, EV2.6 | 04 EV2.1, EV2.3, EV2.6 |
| `EV2:evidence.replay_mismatch` | refusal | Replay disagrees with the record's required comparison; retain differing result/trace position where available. | | EV2.4; ERR.20 | 04 EV2.3, EV2.4 |
| `EV2:capability.debug_write_forbidden` | refusal | Debug/replay would require a live write or an unsupported effect. | | EV2.1, EV2.9 | 04 EV2.1, EV2.3, EV2.9 |
| `EV2:input.word_range_invalid` | refusal | Literal endpoints or compressed occurrence path/offset disagree with the exact expanded word position; resolve no substitute range. | `after_user_change` | EV2.11 | EV2 word reference resolution |
| `EV2:stale.debug_stop` | refusal | Step names an obsolete stop or reuses its request ID with different run/stop inputs; do not advance again. | `after_refresh` | EV2.12 | EV2 step service |

### EV3: view publication and linkage

| Code | Form | Meaning | Retry | Rule source | Used by |
| --- | --- | --- | --- | --- | --- |
| `EV3:stale.source_link` | refusal | After pick-binding checks succeed, the navigation target's source digest differs from the producing source digest. Binding failures use EV3:stale.display_pick first, even if source also differs. | | EV3.5, EV3.6; RM.10 | 04 EV3.5, EV3.6; renderer source navigation |
| `EV3:input.round_trip_undeclared` | refusal | Source part does not declare the requested editable field. | | EV3.7 | 04 EV3.5, EV3.7 |
| `EV3:input.display_plan_invalid` | refusal | Malformed plan, inconsistent subject/geometry binding, invalid instance/map reference, or a relation node with no statement reference. | | RPI.8, RPI.10, RPI.21, RPI.65 | 10 RPI.10, RPI.25, RPI.64, RPI.65 |
| `EV3:input.display_unsupported` | refusal | Dimension, primitive or chart stage is unsupported; no silent substitution. | | RPI.4, RPI.17; JI.41 | 10 RPI.10, RPI.64; JI projection chain |
| `EV3:stale.display_plan` | refusal | Publication no longer matches the dependencies of its target view. | | RPI.10, RPI.12 | 10 RPI.10, RPI.64 |
| `EV3:stale.display_pick` | refusal | Pick dependencies or installed generation no longer match. | | RPI.24, RPI.25 | 10 RPI.25, RPI.64 |
| `EV3:evidence.chart_check_failed` | refusal | Required chart property/precision check failed; withhold checked status without changing another claim. | | RPI.30, RPI.35, RPI.48 | 10 RPI.48, RPI.64 |
| `EV3:input.display_bundle_mismatch` | refusal | Received metadata/arrays are incomplete or do not form one matching engine bundle; install none and request validated refresh. | `after_refresh` | V03.3; ONE.60; RPI.9; JI.41 | 10 RPI.9, RPI.64; JI render payload |
| `EV3:busy.renderer_unavailable` | refusal before acceptance; stop after acceptance before installation | Device loss/recovery prevents drawing or picking. Before acceptance return refused(Error); accepted preparation returns failed(Error), with error(Error) for its job. Installed publications keep completed(view_published); committed receipts remain committed. | | RPI.46, RPI.47; RM.22; LI.28; ERR.9 | 10 RPI.64; renderer publication/recovery |
| `EV3:resource_limit.display_plan_ceiling` | refusal; stop on accepted preparation failure | Publication exceeds a fixed host/profile/backend plan ceiling under RM.11/RM.12, or opening or showing a view exceeds RM.39's ceilings. Context names field, requested count and ceiling. Admit equality; refuse excess before acceptance. A scheduled publication's final validation failure stays failed(Error) on its original invocation (H15.8). | | ERR.25; RPI.50, RPI.82; RM.11, RM.12, RM.17, RM.39 | Renderer publication; numeric ceilings still required before enablement |
| `EV3:input.stage_compile_failed` | refusal before acceptance; stop after acceptance | The host compiler refused a checked stage. The built-in chain stays. | `after_user_change` | RPI.78 | 10 RPI.78 |
| `EV3:input.stage_quarantined` | refusal before acceptance; stop after acceptance | A publication names a stage digest quarantined in this session. | `after_user_change` | RPI.77, RM.20 | 10 RPI.77 |
| `EV3:resource_limit.stage_work_exceeded` | status | A frame with a user stage had a raster bound over its allowance. No user-stage fragment work of that frame ran. Context names every user-stage digest of the chain, the bound and the allowance. | `after_user_change` | RPI.72, RPI.77 | 10 RPI.72, RPI.77 |
| `EV3:input.view_id_in_use` | refusal | `view.open` names an open view. | `after_user_change` | RPI.83 | 10 RPI.83 |
| `EV3:input.view_not_closable` | refusal | `view.close` names an unknown view, Global or Local. | `after_user_change` | RPI.83 | 10 RPI.83 |

### EV4: bindings and recording

| Code | Form | Meaning | Retry | Rule source | Used by |
| --- | --- | --- | --- | --- | --- |
| `EV4:input.binding_conflict` | refusal | Duplicate/conflicting resolved chord within a routing class. | | EV4.3, EV4.7 | 04 EV4.3, EV4.7 |
| `EV4:input.binding_prefix_conflict` | refusal | A complete binding is also a prefix of another sequence in that class. | | EV4.7 | 04 EV4.3, EV4.7 |
| `EV4:input.binding_layout_mismatch` | refusal | Layout-dependent resolution is absent or conflicting for a declared layout. | | EV4.7 | 04 EV4.3, EV4.7 |
| `EV4:input.recording_step_unresolved` | refusal | Recorded step lacks a required command, module, object or input mapping. | | EV4.13, EV4.14 | 04 EV4.13, EV4.14 |
| `EV4:stale.recording_step` | stop | An unattended step's arguments name a revision or preview that is not current, or a current undo or redo hint does not target the played action mapped from the recorded target. | `after_refresh` | EV4.14, EV4.18 | 04 EV4.14, EV4.18 |
| `EV4:input.recording_identity_mismatch` | refusal; stop during playback | Model identity or an input digest differs from the recording's. | `after_user_change` | EV4.14 | 04 EV4.14 |
| `EV4:input.recording_step_outcome_unknown` | stop | A recorded step has no known terminal outcome. Playback and conversion stop at that step. | `after_user_change` | EV4.19 | 04 EV4.19 |
| `EV4:input.recording_state` | refusal | Start while recording, stop with no recording, or a play or `from` that EV4.22's table does not allow. | `after_user_change` | EV4.22 | 04 EV4.22 |
| `EV4:input.recording_not_convertible` | refusal | The recording cannot become this target: a step has no form in it, or the target is `draft` and the word is empty. Nothing is written. Context names the step and the target. | `after_user_change` | EV4.23 | 04 EV4.23 |

### PS1: format validation

| Code | Form | Meaning | Retry | Rule source | Used by |
| --- | --- | --- | --- | --- | --- |
| `PS1:input.digest_mismatch` | refusal | Supplied stored/decoded bytes, members or recorded input/state/dependency data disagree with their claimed digests. | | PS1.2, PS1.6, PS1.37; Round 3 | 05 read_record, migrate_record, import_archive, export_check_bundle; PS3.14 |
| `PS1:input.persistence_format` | refusal | Carrier or required field structure/dimension is invalid for an active read. | | PS1.12, PS1.13, PS1.41 | 05 read_record, import_archive, PS1.42 |
| `PS1:input.noncanonical_value` | refusal | Exact value, part graph or byte profile violates the selected format. | | PS1.3 to PS1.5, PS1.19 | 05 read_record, expand_stored_word, PS1.42 |
| `PS1:resource_limit.record_too_large` | refusal | A record exceeds the applicable shared limits table. | | R-01; PS1.10; Round 3 | Round 3; not yet used as a concrete code |
| `PS1:resource_limit.persistence_size` | refusal | Stored, decoded, cumulative, parsing or expansion resource bound would be exceeded. | | PS1.10, PS1.11 | 05 read_record, write_record, expand_stored_word, migrate_record, import_archive, export_check_bundle; PS1.42 |
| `PS1:resource_limit.word_expansion` | refusal | Exact expanded length exceeds the consuming operation's admission bound; retain that length. | | PS1.19, PS1.21 | 05 expand_stored_word, PS1.42 |
| `PS1:evidence.scope_not_immutable` | refusal | Persisted claim has revision-bound context in its scope. | | PS1.35; EVD.10; partition 8.1; DT-L1 R2-E1, R3-M1 | 05 write_record, export_check_bundle, PS1.42; 06 EVD.58 |
| `PS1:capability.provenance_incompatible` | refusal | Required provenance form, label or dependency cannot justify a gated use. | | PS1.32, PS1.33; BM2; DT-L1 R2-E3 | 05 expand_stored_word, migrate_record, PS1.42; 07 BND.83, CT-BND-22 |

### PS2: editions and migration admission

| Code | Form | Meaning | Retry | Rule source | Used by |
| --- | --- | --- | --- | --- | --- |
| `PS2:input.edition_unsupported` | refusal | Stored record requires an unsupported source edition. Distinct from a source diagnostic. | | PS2.1, PS2.2; Round 3 | Round 3; not yet used as a concrete code |
| `PS2:input.persistence_version` | refusal | Data version/source edition or required migration path is unsupported/unavailable. Original can remain recorded. | | PS2.1 to PS2.4 | 05 read_record, write_record, migrate_record, PS1.42 |

### PS3: import and export

| Code | Form | Meaning | Retry | Rule source | Used by |
| --- | --- | --- | --- | --- | --- |
| `PS3:input.export_off_lattice` | refusal | Requested retained export requires an unavailable off-lattice projection. | | ENG-0J item 3; Round 3 | Round 3; not yet used |
| `PS3:input.checkpoint_replay` | refusal | Checkpoint's own replay path, complete labels or pre/post checks cannot be verified. | | PS1.29; PS3.5, PS3.8 | 05 import_archive, PS1.42 |
| `PS3:input.source_preservation_unverified` | refusal | Locked-copy preservation conditions fail or watch coverage is inconclusive; publish nothing. It asserts no attribution or proven source change. | | PS3.1, PS3.2 | 05 import_archive, PS1.42 |
| `PS3:busy.legacy_session_locked` | refusal | Required existing 0.4 lock cannot be acquired under the source-preservation contract. | | PS3.1, PS3.2 | 05 import_archive, PS1.42 |
| `PS3:unknown_outcome.import_publication` | freeze | Active-pointer switch cannot be authoritatively reconciled; recover before any retry/publication. | | PS3.9 | 05 import_archive, PS1.42 |

### EVD: the evidence pillar

| Code | Form | Meaning | Retry | Rule source | Used by |
| --- | --- | --- | --- | --- | --- |
| `EVD:evidence.premise_unmet` | refusal | A discharge, or an operation that needs the premise established, finds it not held for the required statement and scope. An unresolved premise stored or evaluated is no error (EVD.26). | | Partition 8.1; Round 3 | 06 EVD.17, EVD.58 |
| `EVD:evidence.scope_mismatch` | refusal | Claim is offered outside its immutable scope. | | Partition 8.1; Round 3 | 06 EVD.18, EVD.58 |
| `EVD:evidence.inconsistent_fields` | refusal | Stored evidence fields admit no display-word result under the selected model. | | Partition section 13; Round 3 | 06 EVD.40, EVD.58 |
| `EVD:evidence.negative_without_completion` | refusal | Negative conclusion offered from a job that did not complete its declared coverage. | | KN7.4; Round 3 | 06 EVD.40, EVD.43, EVD.58 |
| `EVD:capability.field_not_writable` | refusal | A record write names a field value its writer may not set. | `after_user_change` | EVD.64, EVD.65 | 06 EVD.64, EVD.65 |

## 6. Reconciliation

- **ERR.28 [D]** The following table has one row per concrete spelling the drafts first used, including prefix-only
  changes. Each of its first two cells holds one code. The chapters now use the registry codes, so the table is a
  record of that migration, not a second set of runtime aliases. Reserved rows wait for their named decisions. The
  outside-domain row records a withdrawn spelling, removed from `errors:` as ERR.22 specifies. Wildcards require no
  individual rows. Generic Round 3 codes retained alongside more specific chapter codes keep their original scopes;
  their existence does not merge distinct failures.

| Spelling first used | Registry code | What changed |
| --- | --- | --- |
| `auth.token_invalid` | `KN5:auth.token_invalid` | Prefix; authenticator failure only, reconnect required. Reuse of a genuine token is separately stale. |
| `budget.display_plan_limit` | `KN7:budget.display_plan_limit` | Reserved declared-job case only, ERR.25. The current draft uses the separate EV3 fixed-ceiling row and KN7 output allowance (RM.12, RM.13). |
| `budget.kernel_cost_exhausted` | `KN7:budget.kernel_cost` | Same declared allowance as Round 3; exhaustion suffix removed. |
| `budget.memory_exhausted` | `KN7:budget.memory` | Same declared logical memory allowance as Round 3. |
| `budget.output_exhausted` | `KN7:budget.output` | Same declared logical output allowance as Round 3. |
| `budget.steps_exhausted` | `KN7:budget.steps` | Same declared step allowance as Round 3. |
| `busy.commit_in_flight` | `KN5:busy.commit_in_flight` | Prefix; no queue or automatic replay. |
| `busy.legacy_session_locked` | `PS3:busy.legacy_session_locked` | Prefix; source-preserving lock admission. |
| `busy.renderer_unavailable` | `EV3:busy.renderer_unavailable` | Prefix; display-service availability. ERR.9 and RM.22 give contextual forms; an installed publication's terminal receipt is unchanged. |
| `capability.commit_row_refused` | `KN5:capability.commit_row_refused` | Prefix; runtime cannot invoke any commit row. |
| `capability.debug_write_forbidden` | `EV2:capability.debug_write_forbidden` | Prefix; replay/debug sink authority is separate from general missing grants. |
| `capability.denied` | `LG3:capability.not_granted` | Chapters 02 and 09 report the same absence of effective authority as Round 3; policy/sandbox can make the grant unavailable. |
| `capability.flow_not_allowed` | `LG3:capability.flow_refused` | The flow-refusal code under capability separation (LQ21; RTE.24); the effect spelling is retired, ERR.24. Chapter 07 and LQ2 set when it is emitted. |
| `capability.grant_mismatch` | `LG3:capability.grant_mismatch` | Prefix; a present grant has the wrong scope/right/grantee/profile, distinct from absence. |
| `capability.lb7_module_grant_missing` | `LB7:capability.module_grant_missing` | LB7 moves to the prefix; retain the module-entry admission fact. |
| `capability.provenance_incompatible` | `PS1:capability.provenance_incompatible` | Prefix; stored provenance cannot justify a gated use under its recorded form. |
| `consistency.cache_entry_mismatch` | `LG4:input.cache_entry_mismatch` | Invalid cache reuse candidate; reject as a miss in the host envelope (DET.46). No kernel invariant or session freeze; moved current bindings still use stale. |
| `consistency.display_bundle_mismatch` | `EV3:input.display_bundle_mismatch` | Incomplete/incompatible received bundle is refused before installation and refreshed (RPI.9); it establishes no authoritative invariant failure. |
| `consistency.lb3_fixed_nu_coordinate` | `KN2:consistency.fixed_nu_nonzero` | Same Round 3 engine invariant. Detection belongs to KN2, not LB3; valid only on reachable lattice I_a inputs, never other-menu comparisons or arbitrary raw poses. |
| `consistency.persistence_digest` | `PS1:input.digest_mismatch` | Supplied record/member/recorded-state digest failed validation, matching Round 3. Kernel state disagreement remains a separate KN failure. |
| `consistency.replay_mismatch` | `EV2:evidence.replay_mismatch` | Replay-versus-record finding (EV2.4); withhold match, no session freeze. Kept distinct from run-versus-run reproduction. |
| `consistency.reproducibility_mismatch` | `LG4:evidence.reproducibility_mismatch` | Supported-key executions disagree in exact payload/coverage (DET.52); evidence finding, not kernel freeze. |
| `consistency.runtime_failed` | `KN7:resource_limit.runtime_lost` | Round 3 code; KN7.13 ends the job as error and leaves the session and engine preview intact. |
| `consistency.source_changed` | `PS3:input.source_preservation_unverified` | PS3.2 also covers inconclusive watches. Refuse publication without falsely asserting a change or freezing a kernel session. |
| `consistency.state_hash_mismatch` | `KN5:consistency.state_hash_mismatch` | Prefix; authoritative equal-revision disagreement retains freeze. |
| `effect.not_allowed` | `LG3:effect.undeclared` | Same forbidden transitive effect-set membership as Round 3. Allowed context includes view restrictions, not just written annotations. |
| `effect.undeclared_effect` | `LG3:effect.undeclared` | Same inferred effect outside the declaration/allowed context as chapter 02; merge the synonym. |
| `evidence.chart_check_failed` | `EV3:evidence.chart_check_failed` | Prefix; admission consumes a failed property/precision check, without inventing a checker or a refutation. |
| `evidence.lb5_reduction_scope_mismatch` | `LB5:evidence.reduction_scope_mismatch` | Source carries LB5; scope mismatch blocks only the proposed transport. |
| `evidence.lb6_principle_record_incomplete` | `LB6:evidence.principle_record_incomplete` | Source carries LB6; admission checks the required record/dependencies. |
| `evidence.obligation_unmet` | `EV1:evidence.obligation_unmet` | Prefix; absent required support, distinct from an allowed unknown value. |
| `evidence.scope_not_immutable` | `PS1:evidence.scope_not_immutable` | Prefix; persistence cannot convert transient context into evidence. |
| `input.binding_conflict` | `EV4:input.binding_conflict` | Prefix; duplicate/conflicting chord in the same routing class. |
| `input.binding_layout_mismatch` | `EV4:input.binding_layout_mismatch` | Prefix; recorded layout resolution failed. |
| `input.binding_prefix_conflict` | `EV4:input.binding_prefix_conflict` | Prefix; prefix-code property failed, distinct from a duplicate. |
| `input.checkpoint_replay` | `PS3:input.checkpoint_replay` | Prefix; checkpoint path/checks failed, distinct from a byte-digest failure. |
| `input.display_plan_invalid` | `EV3:input.display_plan_invalid` | Prefix; plan structure/subject/instance references invalid. |
| `input.display_unsupported` | `EV3:input.display_unsupported` | Prefix; unsupported content stays explicit. |
| `input.identity_mismatch` | `KN1:input.identity_mismatch` | Round 3 origin retained through callers; model/menu/order/contract field named. Distinct from a static type mismatch. |
| `input.lb1_incompatible_alphabet` | `LB1:input.incompatible_alphabet` | Source carries LB1. A word-alphabet contract is separate from a state identity mismatch. |
| `input.lb5_outside_certified_domain` | `LB5:input.outside_certified_domain` | Withdrawn spelling only, no registry row. Remove the errors entry; return the completed OUTSIDE_CERTIFIED_DOMAIN(k) value in section 4. This is not a rename to an active code. |
| `input.level_not_held` | `KN1:input.level_not_held` | Prefix; unavailable puzzle/input level, distinct from partial off-lattice data. |
| `input.module_contract_mismatch` | `LG5:input.module_contract_mismatch` | Prefix; incompatible manifest/edition/dependency contract. |
| `input.module_dependency_missing` | `LG5:input.module_dependency_missing` | Prefix; unavailable locked content at module loading. |
| `input.module_digest_mismatch` | `LG5:input.module_digest_mismatch` | Prefix; retains Round 3's module byte/lockfile meaning. |
| `input.noncanonical_value` | `PS1:input.noncanonical_value` | Prefix; selected byte/value/part-graph profile failed. |
| `input.persistence_format` | `PS1:input.persistence_format` | Prefix; carrier/field structure failed. |
| `input.persistence_version` | `PS2:input.persistence_version` | Prefix; data version or migration unavailable. Round 3's narrower edition code remains separately registered. |
| `input.recording_step_unresolved` | `EV4:input.recording_step_unresolved` | Prefix; semantic playback mapping, distinct from computation replay. |
| `input.replay_dependency_missing` | `EV2:input.replay_dependency_missing` | Keep replay/trace resolution (EV2.1, EV2.3, EV2.6), including a missing edition/trace input. Broader reproduction admission has its own code. |
| `input.repro_input_missing` | `LG4:input.repro_input_missing` | Keep general ReproKey input admission (DET.7), including captured file/snapshot/implementation identity. It need not involve debugger replay. |
| `input.repro_key_mismatch` | `LG4:input.repro_key_mismatch` | Prefix; supplied inputs differ from the requested sealed key. |
| `input.round_trip_undeclared` | `EV3:input.round_trip_undeclared` | Prefix; source did not declare that field editable. |
| `input.unknown_model` | `KN1:input.unknown_model` | Prefix; absent registered identity. |
| `input.unknown_session` | `KN4:input.unknown_session` | Prefix; absent session. |
| `protection.lb2_incomplete_coverage` | `LB2:protection.incomplete_coverage` | Source carries LB2. Missing assurance issues no safe verdict and does not assert proved protected motion. |
| `resource_limit.host_exhausted` | `KN7:resource_limit.host_exhausted` | Prefix; physical host error, without deterministic failure-position promise. |
| `resource_limit.persistence_size` | `PS1:resource_limit.persistence_size` | Prefix; includes decoded/cumulative/parser bounds. Retain Round 3's narrower record-size code. |
| `resource_limit.recursion_exhausted` | `KN7:resource_limit.recursion_exhausted` | Reserved non-budget guard, ERR.26; declared logical-memory exhaustion uses the memory budget code. |
| `resource_limit.safety_stop` | `KN7:resource_limit.safety_stop` | Prefix; already corrected in KN7.10 to the machine-limit category. |
| `resource_limit.word_expansion` | `PS1:resource_limit.word_expansion` | Prefix; stored-word consuming operation's admission bound, separate from KN2 replay and retained primitive ceilings. |
| `stale.display_pick` | `EV3:stale.display_pick` | Prefix; installed generation/dependency mismatch. |
| `stale.display_plan` | `EV3:stale.display_plan` | Prefix; superseded publication dependencies. |
| `stale.editor_source` | `EV1:stale.editor_source` | Prefix; source response/edit no longer matches its target. |
| `stale.epoch_changed` | `KN2:stale.epoch_changed` | Prefix; transient handles/permissions only, not a durable event reference. |
| `stale.no_fresh_preview` | `KN5:stale.no_fresh_preview` | Prefix; commit lacks a fresh presented preview. |
| `stale.preview_replaced` | `KN5:stale.preview_replaced` | Prefix; token names another pending-preview identity. |
| `stale.revision_changed` | `KN5:stale.revision_changed` | Prefix; moved commit base, distinct from equal-revision kernel disagreement. |
| `stale.revision_gone` | `KN2:stale.revision_gone` | Prefix; durable event cannot be resolved, not merely an ended epoch. |
| `stale.source_link` | `EV3:stale.source_link` | Prefix; source-digest mismatch after successful pick-binding checks (RM.10). |
| `stale.token_mismatch` | `KN5:stale.token_mismatch` | Prefix; genuine token's bound content differs. |
| `stale.token_used` | `KN5:stale.token_used` | Prefix; reuse stays separate from failed authentication. |
| `syntax.invalid_encoding` | `LG1:syntax.invalid_encoding` | Prefix; malformed pinned source bytes. |
| `syntax.invalid_literal` | `LG1:syntax.invalid_literal` | Prefix; exact/approximate literal-kind validation. |
| `syntax.missing_edition` | `LG1:syntax.edition_missing` | Same missing source header as Round 3; word order normalized. |
| `syntax.recovery_gap` | `LG1:syntax.recovery_gap` | Grammar diagnostic returned by EV1; no recovered hole executes. |
| `syntax.unexpected_token` | `LG1:syntax.unexpected_token` | Prefix; retains Round 3. |
| `syntax.unsupported_edition` | `LG1:syntax.edition_unsupported` | Same unsupported source edition as Round 3; distinct from stored-record refusal. |
| `type.chart_subset_violation` | `LG2:type.chart_subset_violation` | Prefix retained as history; withdrawn before first use under ERR.5. The replacement rule uses `LG2:type.stage_rule_violation`, with no reuse or retirement edition invented. |
| `type.cost_unit_mismatch` | `LG2:type.cost_unit_mix` | Same wrong puzzle/unit indices as Round 3. |
| `type.exactness_required` | `LG2:type.exact_required` | EV1.4's direct approximate value at an exact site is Round 3's narrow fact. |
| `type.exactness_violation` | `LG2:type.exactness_violation` | Keep LG2.4/LG2.5's broader data/control-dependence violation. A dependent branch need not supply an approximate argument, so merging into the narrow code would lose meaning. |
| `type.identity_mismatch` | `LG2:type.identity_mismatch` | Model/puzzle typing; broader than Round 3's retained puzzle-only row and distinct from run-time input identity. |
| `type.incompatible_value` | `LG2:type.mismatch` | Same general incompatible-type fact as Round 3. |
| `type.lb4_approximate_property_input` | `LB4:type.approximate_property_input` | LB4 moves to source; retain exact property-check input refusal. |
| `type.level_mismatch` | `LG2:type.level_mismatch` | Prefix; retain static discipline and explicit partial handling. |
| `type.mode_mismatch` | `LG2:type.mode_mismatch` | Prefix; retains Round 3. |
| `type.reachability_required` | `LG2:type.reachability_missing` | Same absent required reachability claim as Round 3. |
| `type.unsupported_ir_version` | `LG4:input.ir_version_unsupported` | LG4.1 refuses an unsupported execution contract at load; this is no LG2 source type error. |
| `type.verdict_required` | `LG2:type.verdict_required` | Preserve LG2.9's broader missing-unknown/verdict handling; Round 3's boolean-only diagnostic remains narrower. |
| `type.word_key_not_executable` | `LG2:type.word_key_not_executable` | Prefix; equality keys cannot provide literal execution data. |
| `unknown_outcome.import_publication` | `PS3:unknown_outcome.import_publication` | Prefix; unresolved durable pointer switch requires recovery, never blind retry. |
| `unknown_outcome.read_only_recovery` | `KN5:unknown_outcome.read_only_recovery` | Prefix; refusal while recovery is already active. |

- **ERR.31 [D]** The closure rows below index component-local spellings under ERR.4. The three earlier reference
  spellings record H15.16's category corrections. Only the full codes in section 5 identify the errors; these are
  not runtime aliases. JI remains a rule family, not a raising component.

| Local or earlier spelling | Registry code | What changed |
| --- | --- | --- |
| `evidence.decomposition_mismatch` | `LB6:evidence.decomposition_mismatch` | Component prefix; keep the owning clause's failed fact and continuation. |
| `input.confirmation_required` | `KN5:input.confirmation_required` | Component prefix; keep the owning clause's failed fact and continuation. |
| `input.generator_input` | `LG4:input.generator_input` | Component prefix; keep the owning clause's failed fact and continuation. |
| `input.key_contract_mismatch` | `LB1:input.key_contract_mismatch` | Component prefix; keep the owning clause's failed fact and continuation. |
| `input.module_not_checked` | `EV1:input.module_not_checked` | Component prefix; keep the owning clause's failed fact and continuation. |
| `input.named_object_invalid` | `KN2:input.named_object_invalid` | Component prefix; keep the owning clause's failed fact and continuation. |
| `input.overlay_dependency_missing` | `KN5:input.overlay_dependency_missing` | Component prefix; keep the owning clause's failed fact and continuation. |
| `input.pointwise_failed` | `LG4:input.pointwise_failed` | Component prefix; keep the owning clause's failed fact and continuation. |
| `input.projection_chain` | `EV3:input.display_unsupported` | JI.41 forwards the existing unsupported-stage code; no new synonym. |
| `input.render_payload` | `EV3:input.display_bundle_mismatch` | JI.41 forwards the existing bundle-validation code; no new synonym. |
| `input.scramble_policy_unset` | `KN5:input.scramble_policy_unset` | Component prefix; keep the owning clause's failed fact and continuation. |
| `input.source_range_invalid` | `EV1:input.source_range_invalid` | Component prefix; keep the owning clause's failed fact and continuation. |
| `input.text_scalar_invalid` | `LG4:input.text_scalar_invalid` | Component prefix; keep the owning clause's failed fact and continuation. |
| `input.unsupported_dimension` | `KN1:input.unsupported_dimension` | Component prefix; keep the owning clause's failed fact and continuation. |
| `input.word_range_invalid` | `EV2:input.word_range_invalid` | Component prefix; keep the owning clause's failed fact and continuation. |
| `resource_limit.engine_lost` | `KN5:resource_limit.engine_lost` | Component prefix; keep the owning clause's failed fact and continuation. |
| `resource_limit.generator_exhausted` | `LG4:resource_limit.generator_exhausted` | Component prefix; keep the owning clause's failed fact and continuation. |
| `stale.activation_base` | `EV1:stale.activation_base` | Component prefix; keep the owning clause's failed fact and continuation. |
| `stale.debug_stop` | `EV2:stale.debug_stop` | Component prefix; keep the owning clause's failed fact and continuation. |
| `stale.filter_preview` | `KN5:stale.filter_preview` | Component prefix; keep the owning clause's failed fact and continuation. |
| `stale.result_gone` | `KN5:stale.result_gone` | Component prefix; keep the owning clause's failed fact and continuation. |
| `stale.value_unavailable` | `KN2:stale.value_unavailable` | Component prefix; keep the owning clause's failed fact and continuation. |
| `unverified_reference.branch_path` | `KN4:input.branch_path_unproved` | Input/path proof, raised by the store; no missing frame is asserted. |
| `unverified_reference.exit_record` | `KN5:input.exit_record_voided` | Input/known void binding, raised by the command layer; no frame can revive it. |
| `unverified_reference.exposed_geometry` | `KN2:input.exposed_geometry_unavailable` | Input/certified producer data or size profile absent, raised by the engine; not an ambiguous correspondence. |

- **ERR.29 [D]** Round 3's three corrections are already present in chapters 01 and 08 and are preserved here:
  authenticator failure and used token are separate (ONE commit step 3, section 15); a moved revision/binding is
  stale while authoritative equal-revision digest disagreement freezes (KN5.16, ONE commit steps 5 to 7); and a
  safety stop uses a resource-limit code inside `cut_off(by: Code)` (KN7.3, KN7.10). A full-state hash check
  compares the preview's start digest with the head, never a caller-supplied end digest. No registry change repairs
  or redefines those chapter contracts.

## 7. Conformance tests

- **ERR.30 [D]** These are requirements for chapter 12's conformance suite, not existing tests or results
  (overview section 3.4; default 5). Lints cover the integrated chapters after the reconciliation is applied.
  Retired/reserved rows are checked for identity preservation and cannot authorize emission. All following tests
  use source or fixture evidence; none establishes native or performance behavior.

| Test | Checks | Class |
| --- | --- | --- |
| CT-ERR-01 | Parse every active code: registered source prefix, registered category, category equal to envelope.category, prefix equal to the reported origin; forwarding preserves both. | source |
| CT-ERR-02 | Slugs match lower-case snake case and the 40-character bound; reject empty parts, redundant LB prefixes and names of remedies or selected twists/corrections. | source |
| CT-ERR-03 | Compare registry editions: no code is reused or silently changes meaning; retired rows and their retirement editions remain, with unassigned editions explicit for pre-use withdrawals. | source |
| CT-ERR-04 | Expand each component's errors lines, including wrapped lines and category wildcards, qualified or not; every concrete active code has a row, every wildcard reaches a code, every row has Rule source and Used by, and forwarded codes retain full prefixes. | source |
| CT-ERR-05 | Every envelope has V06 fields, an admitted form and retry value; diagnostics use EV1.10/ERR.12 ranges: [1,5) maps to UTF-16 [1,3), [9,9) to EOF [6,6), and interior offset 2 refuses. For invalid UTF-8 61 FF 62 and truncated 61 F0 9F 98, ERR.10 keeps LG1:syntax.invalid_encoding diagnostic with span absent, raw input digest and offending byte intervals [1,2) and [1,4), no replacement and no host conversion. Renderer loss before acceptance has refusal form, during accepted preparation stop with failed/error, and after installation preserves completed(view_published). Invalid draft bytes never replace the active version. | source |
| CT-ERR-06 | Change and translate summaries/diagnostics while keeping code/context; handling and test outcomes remain the same without text matching. | source |
| CT-ERR-07 | Every active consistency row originates in KN1 to KN7 and freezes only on a kernel invariant failure. Replay, cache, persistence, bundle and runtime-fault fixtures freeze no session. | source |
| CT-ERR-08 | Every active budget row names an allowance the job declared and stops reproducibly for the same key. Missing declarations, host/profile ceilings and wall-clock stops cannot use budget codes. For a fixed count ceiling 100, admit 100 and refuse 101 as EV3:resource_limit.display_plan_ceiling; RM.37's full-straddler payload over a fixed ceiling uses that code for standalone refusal or already-accepted scheduled failure even when producer budgets fit, never KN7:budget.display_plan_limit. For 100 output bytes remaining and 101 required, use cut_off(KN7:budget.output) with cancelled(budget); physical allocation failure uses failed(KN7:resource_limit.host_exhausted). | source |
| CT-ERR-09 | Commit-step fixtures separately fail authenticator, token reuse, replaced preview, bound content, moved revision and equal-revision head hash; codes/forms/retries remain distinct and state is unchanged on refusal. | source |
| CT-ERR-10 | Unknown durable commit/publication fate enters read-only recovery; a new forbidden request returns the recovery refusal, reads work, and no retry is inferred from the absence of a receipt. | source |
| CT-ERR-11 | Completed outside-domain membership, unknown, partial/off-lattice results, false step flags and empty completed lists stay values. A cut-off membership enumeration cannot emit OUTSIDE_CERTIFIED_DOMAIN(k). | source |
| CT-ERR-12 | Reconciliation covers every concrete chapter spelling once; equal-meaning merges retain typed context and distinct comparisons/authority facts remain distinct. Withdrawn outside-domain entries are removed, not renamed into active errors. Check H15.16's 23 closure additions, including the six final KN origins and filter freshness, plus both JI.41 forwards; branch-path, void-exit and geometry failures carry input facts, never invented missing frames. Apply RM.30 to the EV3 registry rows: old pick bindings return stale.display_pick with unchanged or changed source; only current bindings followed by a source-digest mismatch return stale.source_link, and navigation to the pinned producing bytes succeeds. | source |
| CT-ERR-13 | Exercise all category content rules: reason and next action data, retained drafts, typed scope/level/coverage and limits; no unchecked part is safe, no failed support automatically refutes a claim, and no message selects a solving twist/correction. | source |
| CT-ERR-14 | Kill the runtime: jobs end as error with KN7:resource_limit.runtime_lost, state and journal remain equal, and the engine preview remains usable through the normal shell gate. | source |
| CT-ERR-15 | `LG3:effect.flow_refused` is retired and never emitted, `LG3:capability.flow_refused` cannot be emitted before LQ2 sets the rule, and plan/recursion reservations use only their settled contracts. RM.13's fixed-ceiling/output-budget draft never emits KN7:budget.display_plan_limit. | source |
| CT-ERR-16 | Exact-request format, menu-binding and free-search-cap failures retain their three KN2 codes, refusal form, typed failed fact and after_user_change retry. KN3 remains a rule source, never a registered raising family (ERR.23; KN3.7, KN3.8). | source |
| CT-ERR-17 | Stage-rule diagnostics use LG2:type.stage_rule_violation; the withdrawn chart-subset row keeps its meaning and cannot emit. EV3 stage compile failures have refusal before acceptance and stop after it, quarantine refuses, raster-bound excess reports with form status before user-stage fragment work, and view open/close refusals retain their two input codes and category retries. | source |
| CT-ERR-18 | Recording stale-step stops inherit after_refresh; identity mismatches refuse before playback and stop during it. Unknown recorded terminal outcomes stop playback and conversion, state and conversion refusals keep their own codes, and forwarded child failures retain their original identities (EV4.14, EV4.19, EV4.22, EV4.23; ERR.7, ERR.14). | source |
| CT-ERR-19 | An untranslatable obligation refuses with LB5:input.obligation_untranslatable and writes nothing; an unauthorized evidence field refuses with EVD:capability.field_not_writable. Each keeps its owning chapter, refusal form and capability/input category retry (LB5.18; EVD.64, EVD.65; ERR.14). | source |

## 8. Open items

The owner answered LQ1 to LQ21 on 6 November 2026 and chose LQ8, LQ9 and LQ21 on 14 November 2026: the engine and the
interpreter's host in Rust, route RT1 with the runtime in a process of its own, and capability separation (RTE.17,
RTE.21, RTE.24). LQ13 waits for its evidence, and LQ2's option set waits for the joint setting.
The remaining choices and mechanisms never silently change an allocated code's meaning.

| Item | What stays fixed and what remains to decide | Clauses |
| --- | --- | --- |
| LQ2 option set | Execution gates and built-in boundary remain. The joint setting decides allowed flows, sinks, author classes and exceptions, so no policy refusal is enabled by this registry alone. | ERR.13, ERR.17, ERR.24 |
| LQ8 answered | The engine is written in Rust (owner, 14 November 2026, LQ8; RTE.17). The implementation must preserve error identity and kernel invariants. | ERR.1, ERR.7, ERR.19 |
| LQ9 answered | Route RT1 with the runtime in a process of its own (owner, 14 November 2026, LQ9; RTE.21). Runtime loss never freezes a session; unsupported execution contracts remain explicit. | ERR.9, ERR.17, ERR.19, ERR.24 |
| LQ13 | Syntax waits for LX-8's owner-set human protocol and evidence before LP2's parser. Diagnostics keep digest-bound ranges and stable codes; source and stored-edition failures stay distinct. | ERR.6, ERR.10, ERR.12; LG1 and PS2 tables |
| LQ21 answered | Capability separation (owner, 14 November 2026, LQ21; RTE.24). `LG3:effect.flow_refused` is retired before use; chapter 07 decides the rule under the selected mechanism. | ERR.17, ERR.24 |
| Envelope additions the owner may overrule | ERR.9's form, ERR.10's span and ERR.11's after_user_request are new D additions to V06 from Round 3. Removing one requires an explicit replacement for the diagnostic, job-stop or fresh-action distinction it serves. | ERR.8 to ERR.11 |
| Span protocol implementation | H7 and EV1.10 now supply the draft convention and conversions; implementation and conformance remain owed. No coordinate choice is left to this registry. | ERR.12 |
| Flow rule [O] | LQ21 settled the form (ERR.24). Chapter 07 and LQ2 settle when `LG3:capability.flow_refused` is emitted. | ERR.24 |
| Plan-limit amounts [O] | RM.11 to RM.13 draft fixed ceilings plus the existing output allowance. Numeric production limits and measurements remain owed; a separately declared plan allowance would need its own contract before activating the reserved budget row. | ERR.25 |
| Recursion guard [O] | DET.28 and LP1 choose the non-budget guard or logical stack/memory treatment and its count. | ERR.26 |
| Chapters 06 and 07 | EVD.64 and EVD.65 add the field-writer refusal. Chapter 07 uses BND.83's rows and the existing not_granted code at BND.84/BND.85's preview check. Owning implementations and conformance remain owed. | ERR.21, ERR.23, ERR.24 |
| Source differences and integration | Section 6 records the spellings the chapters first used, and the chapters now use the registry codes. Round 3's two budget-admission rows are retained but retired before use. Chapters 01 and 08's current commit and safety-stop corrections are preserved. | ERR.16 to ERR.29 |
| LP1, EN3 and EN4 | Final envelope encoding, typed context, command/revision transport, shared limits and the declared charge schedule still need their owning contracts. No new numerical limit is supplied here. The retained primitive ceiling is V05.1's source compatibility fact; fixed ν coordinates are ENG-0J A5's I_a-only invariant. | ERR.8, ERR.16, ERR.23, ERR.25, ERR.26 |
| DT-L1 obligations | Round 3 cleared them (overview section 8). R2-E1 supports stored evidence/scope, R2-E3 supports provenance/flow, and R2-E4 supports partial-result and principle-record references where cited, each as round 3 corrected it. | ERR.21, ERR.24; LB6 and PS1 tables |
| Design hold and evidence | Built-in tools retain the 2 October boundary and 17 October exception until the joint setting. No UI design or DT-J2 to DT-J4 work is opened. Required tests have no results here; the writing check proves structure only. | ERR.1, ERR.13, ERR.30 |

## 9. The amendment at LP3's command-table freeze

One change under PHS.52, made with H-08's freeze (PHS.53), carries items 1 to 4 of the last section of the owner's
[16 November 2026 page](../../../../wiki/decisions/owner-decisions-2026-11-16-phs52.md) and item 6 of the
[20 November 2026 page](../../../../wiki/decisions/owner-decisions-2026-11-20-lp3-exit.md). No code is renamed,
reused or retired, and no earlier row changes its meaning, form or retry (ERR.5).

- Section 5 registers the eleven codes that the command layer and the session store used as drafts. Six are from
  16 November: `KN2:consistency.star_certificate`, `KN5:input.unknown_row`, `KN5:input.argument_form`,
  `KN5:input.row_not_built`, `KN5:input.overlay_row_conflict` and `KN4:input.session_exists`. Five are from
  20 November: `KN5:consistency.unadmitted_journal_action`, `KN5:capability.preview_shown`,
  `KN5:capability.outside_solver_command`, `KN5:input.saved_set_exists` and `KN4:busy.session_locked`.
- M8's four drafts, as section 3 (d) of the 16 November page settles them:
  - two new rows, `KN2:input.named_cap_blocked_or_closed` and `KN2:input.menu_order_unpinned`;
    `KN2:input.named_object_invalid` and `KN2:input.menu_binding_mismatch` are not widened;
  - a repair list's `max_pairs` takes `KN7:budget.kernel_cost` once JI declares it part of that allowance, and
    until then it has no code;
  - a cancelled repair list needs no code (KN7.3's `cancelled`).
- Errata:
  - The LB6 closure row `evidence.decomposition_mismatch` heads ERR.31's table. ERR.31 keeps its number,
    although it stands before ERR.29 and ERR.30.
  - Rule sources: `KN5:stale.filter_preview` adds KN0.9, and `LG3:capability.not_granted` adds DET.36.
  - The Used-by cells of `KN2:input.word_blocked_prefix`, `KN2:input.recipe_shape`,
    `KN2:resource_limit.primitive_ceiling` and `LG3:capability.ambient_read` now name their users.
- The store raises `KN4:busy.session_locked` for its session lock (KN4.1). Persistence likewise raises
  `PS3:busy.legacy_session_locked`, and section 3's busy row lists neither layer. This amendment does not change
  that row.
- Items 5 and 6 of the same list change LI.5 and LI.33 in the layer interfaces, outside this chapter.

# Chapter 13: the error registry

Status: **draft for the owner. Not accepted.** Conventions are in [the overview](00-overview.md), section 3.

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
[charter E6 and M3](../../../../progress/1.0/charter-2.0-draft.md).

## 1. Codes and ownership

- **ERR.1 [D]** This is the registry required by partition default 5. Its code form follows default 19 and DT-L1
  Round 3, item 2. The tables describe contracts for later implementation. They establish no conformance result or
  Windows, DirectX, input, long-session or performance evidence. Owner questions LQ1 to LQ21 remain open.
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
  category from that component and needs no registry row of its own. An interface returning another component's
  error uses its full code. Section 6 records what replaced the drafts' provisional short spellings, in prose and code
  tables too (default 19; DT-L1 Round 3).
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
    form:        diagnostic | refusal | stop | freeze
    span:        source range, when the error is a diagnostic
  }
  ```

- **ERR.9 [D]** `form` is an addition to V06 proposed by DT-L1 Round 3. It follows from default 5's language
  diagnostics and KN7.3's distinct job outcomes. It distinguishes where the code occurs, without redefining those
  outcomes:

| Form | Contract |
| --- | --- |
| `diagnostic` | Attached to source; the diagnosed candidate did not execute. The last valid module remains separate. |
| `refusal` | Returned to the caller; the refused operation changes no authoritative state or active durable record. |
| `stop` | A reason inside a job outcome. KN7.3 decides completed, cancelled, cut off or error; a reason never implies completion. |
| `freeze` | A refusal that also stops execution pending recovery. For an unknown durable outcome, reads continue in read-only recovery. |

- **ERR.10 [D]** `span` is a second addition to V06 from DT-L1 Round 3. It follows from partition EV1 and LG1:
  diagnostics identify the immutable source digest and the affected range. A diagnostic for an absent edition header
  can identify an insertion range. A service never attaches a diagnostic from the executed version to different draft
  bytes (EV1.1, EV1.7). A run-time refusal without a source position does not fabricate one.
- **ERR.11 [D]** `after_user_request` is an addition to V06's retry values from DT-L1 Round 3. It follows from V06's
  busy and cancellation continuations: a fresh user action may send the request again, with current guards and any
  newly required token. Nothing retries automatically. A retry value describes eligibility after its condition, not
  an instruction to replay a durable command. KN5.21 to KN5.23 and ONE.71 still govern its identity and receipt.
- **ERR.12 [O]** The coordinate unit and endpoints of `span` await EV1.10's LP1 editor protocol with chapter 10.
  UTF-8 byte offsets, Unicode scalar positions and host code-unit positions are the recorded alternatives. Each binds
  the range to the same source bytes; their conversion and endpoints differ. The error envelope adopts that contract
  rather than choosing another position convention.
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
| `syntax` | Source cannot be parsed, or its edition header is absent or unsupported. | yes; `after_user_change` | Source range and parse/edition fact; identity of the retained valid version. | LG1, delivered through EV1 |
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
  membership answer. It has left the error registry. Its withdrawn prefixed spelling, `LB5:input.outside_certified_domain`,
  is recorded here only to prevent future allocation as an error. Section 6 records that withdrawal as a removal,
  not a code substitution. A cut-off membership job returns unknown membership, never this completed rejection
  (LB5.4, INV-5, default 6).

## 5. Registry by source

- **ERR.23 [D]** Each table below has one raising source. A blank Retry cell inherits section 3's category default.
  An explicit value overrides only retry eligibility, never the commit or recovery gates. Used by names the drafted
  chapters and clauses, including interfaces that forward the code. `Round 3; not yet used` retains an entry from
  the advisor's first list without pretending a chapter already emits it. Reserved and retired rows are not active.
  An unassigned flow form is marked pending LQ21; this is no fifth envelope form. Rule source always identifies a
  requirement; a chapter's code table alone is not its justification (default 5).

- **ERR.24 [O]** Both `LG3:effect.flow_refused` and `LG3:capability.flow_refused` remain reserved. LQ21 retires
  one before first use, and chapter 07 owns the rule (BND.33) and the form. Flow labels can reject a forbidden source
  flow; capability separation rejects the combined authority. Static and run-time label variants can detect the
  refusal at different points. LQ2 still decides permitted flows, sinks and exceptions under either mechanism
  (BM2; LG3.12 to LG3.14; DT-L1 R2-E3). The reconciliation target for the current capability spelling is a reservation,
  not an answer to LQ21.
- **ERR.25 [O]** `budget.display_plan_limit` cannot be active until RPI.50 and LP1 settle its limit and units.
  The reserved budget row below applies only to a declared deterministic job allowance. A fixed profile/backend
  ceiling instead uses the reserved EV3 resource-limit row. Both withhold the whole incomplete publication under
  RPI.49 and leave engine adoption independent. The second spelling supplies the fixed-ceiling alternative; it
  selects no limit, amount or additional budget field.
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

### KN2: exact engines

| Code | Form | Meaning | Retry | Rule source | Used by |
| --- | --- | --- | --- | --- | --- |
| `KN2:stale.revision_gone` | refusal | The requested journal event/revision is no longer held. | | KN2.2; ONE.17 | 01 KN2.2, KN4.3; 04 EV2.1, EV2.8; 08 ONE.17 |
| `KN2:stale.epoch_changed` | refusal | A transient handle or preview belongs to an ended engine epoch. A durable RevisionRef can still be resolved. | | ONE.17, ONE.43 | 08 section 15 |
| `KN2:input.rotation_not_representable` | refusal | Requested rotation has no permitted exact representation. | | ENG-0J item 3; INV-4 | Round 3; not yet used |
| `KN2:input.twist_off_menu` | refusal | Requested twist is outside the signed menu. | | ENG-0J item 3 | Round 3; not yet used |
| `KN2:input.menu_invalid` | refusal | Menu fails inverse closure or A4 invariance. | | ENG-0J item 3 | Round 3; not yet used |
| `KN2:input.word_blocked_prefix` | refusal | A stored word has a blocked prefix at the current input; context gives the prefix length. | | ENG-0J item 3; ONE.33 | Round 3; not yet used |
| `KN2:input.recipe_shape` | refusal | Recipe violates the retained word/star step shape or step range. | | Partition section 13; PS1.22 | Round 3; not yet used |
| `KN2:unverified_reference.frame_ambiguous` | refusal | Ordered references do not fix one proper correspondence. | | V06 | Round 3; not yet used |
| `KN2:protection.orbit_would_move` | refusal | Label-level protected orbit would move under the active protection contract. | | V06; KN1.11 | Round 3; 01 KN5.16 and 08 commit step 8 use the family |
| `KN2:protection.piece_would_move` | refusal | Pose-level protected piece would move. No label-level substitution. | | KN1.11; LB3.4 | Round 3; 01 KN5.16 and 08 commit step 8 use the family |
| `KN2:resource_limit.primitive_ceiling` | refusal | Total retained-generator primitive count exceeds the compatibility ceiling, regardless of chunks. | | V05.1; LB2.4; PS1.10 | Round 3; not yet used |
| `KN2:resource_limit.word_length_bound` | refusal | Menu-word replay exceeds the X3 admission bound; exact length retained. Bound remains unset here. | | ENG-0J X3; KN2.10 | Round 3; not yet used |
| `KN2:consistency.fixed_nu_nonzero` | freeze | A reachable lattice result under signed I_a has a nonzero fixed ν coordinate. | | ENG-0J item 3 and A5; DT-J1 Outcome fix-first 5 | 03 LB3.1 catalogue, section 11 |

### KN4: session store

| Code | Form | Meaning | Retry | Rule source | Used by |
| --- | --- | --- | --- | --- | --- |
| `KN4:input.unknown_session` | refusal | No such session exists. | | KN2.2, KN4.1 | 01 KN2.2 |
| `KN4:input.checkpoint_unwitnessed` | refusal | Checkpoint has neither a legal witness nor a label certificate. | | ENG-0J item 3; PS1.29 | Round 3; not yet used |
| `KN4:resource_limit.disk_full` | refusal | A durable write could not be completed; no half-published state. | | R-06; KN4.2 | Round 3; not yet used |

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
| `KN7:budget.output` | stop | Declared logical output allowance ends. | | KN7.7, KN7.10; DET.24 | 01 KN2.11, KN7.10; 09 DET.7, DET.52 |
| `KN7:budget.display_plan_limit` | stop | Reserved: a declared deterministic plan allowance ends; install no incomplete publication. | | ERR.25; RPI.49, RPI.50 | 10 RPI.10, RPI.64; activation pending |
| `KN7:budget.not_declared` | refusal | Retired before first use: Round 3's missing-budget proposal. Use the input successor below. | | Round 3; ERR.16, ERR.27 | Round 3; never active; edition unassigned |
| `KN7:budget.over_ceiling` | refusal | Retired before first use: Round 3's budget-admission ceiling proposal. Use the resource-limit successor below. | | Round 3; ERR.16, ERR.27 | Round 3; never active; edition unassigned |
| `KN7:input.budget_not_declared` | refusal | Admission has no explicit deterministic budget. | | KN7.7; ERR.27 | Round 3 successor; no chapter caller yet |
| `KN7:resource_limit.budget_ceiling` | refusal | Requested allowance exceeds a host/profile ceiling before admission. | | BM3; ERR.16, ERR.27 | Round 3 successor; no chapter caller yet |
| `KN7:resource_limit.safety_stop` | stop | Wall-clock safety stop; cut off, incomplete and not reproducible. | | INV-10; KN7.10 | 01 KN7.10; 09 DET.17, DET.18 |
| `KN7:resource_limit.runtime_lost` | stop | Runtime died; job outcome error, session and pending engine preview preserved. | | KN7.13; ONE.70 | 09 DET.43, DET.52 |
| `KN7:resource_limit.host_exhausted` | stop | Physical resources or a host safety mechanism ended the job as error. | | DET.17, DET.43 | 09 DET.52 |
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
| `LG1:syntax.invalid_encoding` | diagnostic | Source bytes violate the UTF-8/pinned-byte input contract. | | LG1.3 | 02 parse, section 6 |
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
| `LG2:type.chart_subset_violation` | diagnostic | Under recommended LQ7, chart evaluator violates the checked subset/bounds. | | RPI.32, RPI.33, RPI.48 | 10 RPI.48, RPI.64; conditional on LQ7 |

### LG3: effects and capability host

| Code | Form | Meaning | Retry | Rule source | Used by |
| --- | --- | --- | --- | --- | --- |
| `LG3:effect.undeclared` | diagnostic | Transitive effect exceeds the allowed effect set of its declaration/context. | | LG3.2, LG3.5, LG3.6; Round 3 | 02 section 6; 04 EV1.2, EV1.4, EV3.2; 07 BND.83 |
| `LG3:effect.view_writes_workspace` | diagnostic | View has a workspace-write effect through its body or callees. | | Default 16; LG3.5 | Round 3; not yet used as a concrete code |
| `LG3:effect.flow_refused` | pending LQ21 | Reserved flow-label spelling; the rule (BND.33) and the form await LQ2 and LQ21. | | BM2; ERR.24; Round 3 | 02 LG3.12, LG3.14, section 6; 07 BND.83, CT-BND-24 |
| `LG3:capability.flow_refused` | pending LQ21 | Reserved capability-separation spelling; LQ21 retires one alternative before use. | | BM2; ERR.24; Round 3 | 02 LG3.13, LG3.14, section 6; 07 BND.83, CT-BND-24 |
| `LG3:capability.not_granted` | refusal | Requested authority is absent or barred by the effective sandbox/profile. | | LG3.8, LG3.9; DET.37, DET.42 | 02 section 6; 07 BND.61, BND.83; 09 DET.37, DET.52; Round 3 |
| `LG3:capability.ambient_read` | refusal | Clock, file or network read requested without a grant; network remains barred in 1.0. | | INV-10; default 2; LX-3 | Round 3; not yet used as a concrete code |
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

### LB6: built-in tool admission

| Code | Form | Meaning | Retry | Rule source | Used by |
| --- | --- | --- | --- | --- | --- |
| `LB6:evidence.principle_record_incomplete` | refusal | Required PrincipleRecord part or resolving evidence/fixture reference is absent. | | INV-12; LB6.1, LB6.2; DT-L1 R2-E4 | 03 LB6.2, section 11 |

### LB7: module entry admission

| Code | Form | Meaning | Retry | Rule source | Used by |
| --- | --- | --- | --- | --- | --- |
| `LB7:capability.module_grant_missing` | refusal | A module entry's required effect lacks its host grant. | | LB7.1, LB7.5 | 03 LB7.1, section 11 |

### EV1: editor services

| Code | Form | Meaning | Retry | Rule source | Used by |
| --- | --- | --- | --- | --- | --- |
| `EV1:stale.editor_source` | refusal | Response or proposed edit names source bytes no longer equal to its target. | | EV1.1, EV1.6 | 04 EV1.2, EV1.5, EV1.6, EV3.5 |
| `EV1:evidence.obligation_unmet` | diagnostic | Operation requires a claim/check that is absent; an inspectable open claim alone is no error. | | EV1.4 | 04 EV1.2, EV1.4 |

### EV2: debugger and recorded replay

| Code | Form | Meaning | Retry | Rule source | Used by |
| --- | --- | --- | --- | --- | --- |
| `EV2:input.replay_dependency_missing` | refusal | Required replay digest, edition, IR version or trace input cannot be resolved. | | EV2.1, EV2.3, EV2.6 | 04 EV2.1, EV2.3, EV2.6 |
| `EV2:evidence.replay_mismatch` | refusal | Replay disagrees with the record's required comparison; retain differing result/trace position where available. | | EV2.4; ERR.20 | 04 EV2.3, EV2.4 |
| `EV2:capability.debug_write_forbidden` | refusal | Debug/replay would require a live write or an unsupported effect. | | EV2.1, EV2.9 | 04 EV2.1, EV2.3, EV2.9 |

### EV3: view publication and linkage

| Code | Form | Meaning | Retry | Rule source | Used by |
| --- | --- | --- | --- | --- | --- |
| `EV3:stale.source_link` | refusal | Pick/source link no longer matches the installed plan base or producing source. | | EV3.5, EV3.6 | 04 EV3.5, EV3.6 |
| `EV3:input.round_trip_undeclared` | refusal | Source part does not declare the requested editable field. | | EV3.7 | 04 EV3.5, EV3.7 |
| `EV3:input.display_plan_invalid` | refusal | Malformed plan, inconsistent subject/geometry binding or invalid instance/map reference. | | RPI.8, RPI.10, RPI.21 | 10 RPI.10, RPI.25, RPI.64 |
| `EV3:input.display_unsupported` | refusal | Dimension, primitive or chart stage is unsupported; no silent substitution. | | RPI.4, RPI.17 | 10 RPI.10, RPI.64 |
| `EV3:stale.display_plan` | refusal | Publication no longer matches the dependencies of its target view. | | RPI.10, RPI.12 | 10 RPI.10, RPI.64 |
| `EV3:stale.display_pick` | refusal | Pick dependencies or installed generation no longer match. | | RPI.24, RPI.25 | 10 RPI.25, RPI.64 |
| `EV3:evidence.chart_check_failed` | refusal | Required chart property/precision check failed; withhold checked status without changing another claim. | | RPI.30, RPI.35, RPI.48 | 10 RPI.48, RPI.64 |
| `EV3:input.display_bundle_mismatch` | refusal | Received metadata/arrays are incomplete or do not form one matching engine bundle; install none and request validated refresh. | `after_refresh` | V03.3; ONE.60; RPI.9 | 10 RPI.9, RPI.64 |
| `EV3:busy.renderer_unavailable` | refusal | Device loss/recovery prevents drawing or picking; committed receipts remain committed. | | RPI.46, RPI.47 | 10 RPI.64 |
| `EV3:resource_limit.display_plan_ceiling` | refusal | Reserved: publication exceeds a fixed host/profile/backend plan ceiling. | | ERR.25; RPI.50 | Fixed-ceiling alternative; no active caller yet |

### EV4: bindings and recording

| Code | Form | Meaning | Retry | Rule source | Used by |
| --- | --- | --- | --- | --- | --- |
| `EV4:input.binding_conflict` | refusal | Duplicate/conflicting resolved chord within a routing class. | | EV4.3, EV4.7 | 04 EV4.3, EV4.7 |
| `EV4:input.binding_prefix_conflict` | refusal | A complete binding is also a prefix of another sequence in that class. | | EV4.7 | 04 EV4.3, EV4.7 |
| `EV4:input.binding_layout_mismatch` | refusal | Layout-dependent resolution is absent or conflicting for a declared layout. | | EV4.7 | 04 EV4.3, EV4.7 |
| `EV4:input.recording_step_unresolved` | refusal | Recorded step lacks a required command, module, object or input mapping. | | EV4.13, EV4.14 | 04 EV4.13, EV4.14 |

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
| `EVD:evidence.premise_unmet` | refusal | Cited premise is not held for the required statement and scope. | | Partition 8.1; Round 3 | 06 EVD.17, EVD.58 |
| `EVD:evidence.scope_mismatch` | refusal | Claim is offered outside its immutable scope. | | Partition 8.1; Round 3 | 06 EVD.18, EVD.58 |
| `EVD:evidence.inconsistent_fields` | refusal | Stored evidence fields admit no display-word result under the selected model. | | Partition section 13; Round 3 | 06 EVD.40, EVD.58 |
| `EVD:evidence.negative_without_completion` | refusal | Negative conclusion offered from a job that did not complete its declared coverage. | | KN7.4; Round 3 | 06 EVD.40, EVD.43, EVD.58 |

## 6. Reconciliation

- **ERR.28 [D]** The following table has one row per concrete spelling the drafts first used, including prefix-only
  changes. Each of its first two cells holds one code. The chapters now use the registry codes, so the table is a record of
  that migration, not a second set of runtime aliases. Reserved rows wait for their named decisions. The outside-domain
  row records a withdrawn spelling, removed from `errors:` as ERR.22 specifies. Wildcards require no individual rows. Generic Round 3 codes
  retained alongside more specific chapter codes keep their original scopes; their existence does not merge distinct
  failures.

| Spelling first used | Registry code | What changed |
| --- | --- | --- |
| `auth.token_invalid` | `KN5:auth.token_invalid` | Prefix; authenticator failure only, reconnect required. Reuse of a genuine token is separately stale. |
| `budget.display_plan_limit` | `KN7:budget.display_plan_limit` | Reserved declared-job case only, ERR.25. A fixed ceiling uses the separate EV3 resource-limit reservation. No unconditional emission yet. |
| `budget.kernel_cost_exhausted` | `KN7:budget.kernel_cost` | Same declared allowance as Round 3; exhaustion suffix removed. |
| `budget.memory_exhausted` | `KN7:budget.memory` | Same declared logical memory allowance as Round 3. |
| `budget.output_exhausted` | `KN7:budget.output` | Same declared logical output allowance as Round 3. |
| `budget.steps_exhausted` | `KN7:budget.steps` | Same declared step allowance as Round 3. |
| `busy.commit_in_flight` | `KN5:busy.commit_in_flight` | Prefix; no queue or automatic replay. |
| `busy.legacy_session_locked` | `PS3:busy.legacy_session_locked` | Prefix; source-preserving lock admission. |
| `busy.renderer_unavailable` | `EV3:busy.renderer_unavailable` | Prefix; display-service availability, independent of durable receipts. |
| `capability.commit_row_refused` | `KN5:capability.commit_row_refused` | Prefix; runtime cannot invoke any commit row. |
| `capability.debug_write_forbidden` | `EV2:capability.debug_write_forbidden` | Prefix; replay/debug sink authority is separate from general missing grants. |
| `capability.denied` | `LG3:capability.not_granted` | Chapters 02 and 09 report the same absence of effective authority as Round 3; policy/sandbox can make the grant unavailable. |
| `capability.flow_not_allowed` | `LG3:capability.flow_refused` | Reserved capability spelling only. Defer activation to LQ21 and chapter 07; the effect spelling also stays reserved, ERR.24. |
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
| `stale.source_link` | `EV3:stale.source_link` | Prefix; source/plan correspondence no longer current. |
| `stale.token_mismatch` | `KN5:stale.token_mismatch` | Prefix; genuine token's bound content differs. |
| `stale.token_used` | `KN5:stale.token_used` | Prefix; reuse stays separate from failed authentication. |
| `syntax.invalid_encoding` | `LG1:syntax.invalid_encoding` | Prefix; malformed pinned source bytes. |
| `syntax.invalid_literal` | `LG1:syntax.invalid_literal` | Prefix; exact/approximate literal-kind validation. |
| `syntax.missing_edition` | `LG1:syntax.edition_missing` | Same missing source header as Round 3; word order normalized. |
| `syntax.recovery_gap` | `LG1:syntax.recovery_gap` | Grammar diagnostic returned by EV1; no recovered hole executes. |
| `syntax.unexpected_token` | `LG1:syntax.unexpected_token` | Prefix; retains Round 3. |
| `syntax.unsupported_edition` | `LG1:syntax.edition_unsupported` | Same unsupported source edition as Round 3; distinct from stored-record refusal. |
| `type.chart_subset_violation` | `LG2:type.chart_subset_violation` | Prefix; conditional static subset diagnostic under recommended LQ7 only. |
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
| CT-ERR-04 | Expand each component's errors lines, including wrapped lines and category wildcards; every concrete active code has a row, every row has Rule source and Used by, and forwarded codes retain full prefixes. | source |
| CT-ERR-05 | Every envelope has V06 fields, an admitted form and retry value; diagnostics bind a digest/range using the selected EV1.10 convention. Invalid draft bytes never replace the active version. | source |
| CT-ERR-06 | Change and translate summaries/diagnostics while keeping code/context; handling and test outcomes remain the same without text matching. | source |
| CT-ERR-07 | Every active consistency row originates in KN1 to KN7 and freezes only on a kernel invariant failure. Replay, cache, persistence, bundle and runtime-fault fixtures freeze no session. | source |
| CT-ERR-08 | Every active budget row names an allowance the job declared and stops reproducibly for the same key. Missing declarations, host/profile ceilings and wall-clock stops cannot use budget codes. | source |
| CT-ERR-09 | Commit-step fixtures separately fail authenticator, token reuse, replaced preview, bound content, moved revision and equal-revision head hash; codes/forms/retries remain distinct and state is unchanged on refusal. | source |
| CT-ERR-10 | Unknown durable commit/publication fate enters read-only recovery; a new forbidden request returns the recovery refusal, reads work, and no retry is inferred from the absence of a receipt. | source |
| CT-ERR-11 | Completed outside-domain membership, unknown, partial/off-lattice results, false step flags and empty completed lists stay values. A cut-off membership enumeration cannot emit OUTSIDE_CERTIFIED_DOMAIN(k). | source |
| CT-ERR-12 | Reconciliation covers every concrete chapter spelling once; equal-meaning merges retain typed context and distinct comparisons/authority facts remain distinct. Withdrawn outside-domain entries are removed, not renamed into active errors. | source |
| CT-ERR-13 | Exercise all category content rules: reason and next action data, retained drafts, typed scope/level/coverage and limits; no unchecked part is safe, no failed support automatically refutes a claim, and no message selects a solving twist/correction. | source |
| CT-ERR-14 | Kill the runtime: jobs end as error with KN7:resource_limit.runtime_lost, state and journal remain equal, and the engine preview remains usable through the normal shell gate. | source |
| CT-ERR-15 | Reserved flow codes cannot be emitted before LQ2 and LQ21 set the rule and retire one, and plan/recursion reservations use only their settled contracts. | source |

## 8. Open items

All owner questions remain open. The following table identifies their contact with this registry. Alternatives change
the named producers, checks or payloads; they do not silently change an allocated code's meaning.

| Item | What stays fixed and what remains to decide | Clauses |
| --- | --- | --- |
| LQ1 | Common kernel error meanings remain. One language/profiles, a separate research language or product-only scope changes which adapters and run producers use them. | ERR.1, ERR.7, ERR.20 |
| LQ2 | Execution gates and built-in boundary remain. The joint setting decides allowed flows, sinks, author classes and exceptions, so no policy refusal is enabled by this registry alone. | ERR.13, ERR.17, ERR.24 |
| LQ3 | Unknown, failed support and permission remain separate. Chapter 06 defines the selected evidence model's obligations and the EVD rows. | ERR.14, ERR.20, ERR.21, ERR.23 |
| LQ4 | Results keep their input identity. Finish/cancel choices for pinned jobs and the allowed branch hosts change job lifetimes, not cancellation or stale meanings. | ERR.16, ERR.18, ERR.21 |
| LQ5 | Built-ins retain principle records and exact contracts. Language/native/per-tool producers can need different diagnostic adapters and decomposition checks. | ERR.7, ERR.23; LB6 table |
| LQ6 | Canonical bindings and independent adoption remain. Allowed view hosts change where display errors are consumed. | ERR.18, ERR.23; EV3 table |
| LQ7 | Exact property evidence and missing coverage remain distinct. The selected chart route decides whether the subset diagnostic applies and which checks are required. | ERR.21, ERR.23, ERR.25; LG2 and EV3 tables |
| LQ8 | Error identity and kernel invariants remain. Engine choice changes the implementation reporting them, with no result inferred here. | ERR.1, ERR.7, ERR.19 |
| LQ9 | Runtime loss never freezes a session. Route and placement decide fault containment and static/run-time adapters; unsupported execution contracts remain explicit. | ERR.9, ERR.17, ERR.19, ERR.24 |
| LQ10 | Stored error identities remain stable. Envelope facts/judgements and freeze scope change evidence validation and byte dependencies in chapters 05 and 06. | ERR.5, ERR.8, ERR.20, ERR.23 |
| LQ11 | No recorded token or unattended commit permission exists. Recording/playback alternatives change unresolved-step contexts while computation replay remains separate. | ERR.11, ERR.23; EV2 and EV4 tables |
| LQ12 | Content identity and present no-network rule remain. Sharing options change module acquisition, not grants or local dependency/digest refusals. | ERR.7, ERR.17; LG5 table |
| LQ13 | Source diagnostics keep digest-bound ranges and stable codes. Syntax/edition choice changes parser details, not source versus stored-edition distinctions. | ERR.6, ERR.10, ERR.12; LG1 and PS2 tables |
| LQ14 | Evidence and permission remain separate. Soft guidance or an owner-selected competence gate changes capability eligibility only under chapter 07's policy. | ERR.13, ERR.17, ERR.24 |
| LQ15 | Failed checks establish no automatic refutation. A proof-assistant option adds support/check producers; chapter 06 confirms their obligations before codes are added. | ERR.20, ERR.21, ERR.23 |
| LQ16 | Included components keep their error contracts. The shipped cut determines which library/research/service entries are active. | ERR.1, ERR.7, ERR.23 |
| LQ17 | Prefixes identify components. The working language name and extension remain the owner's, without changing registry identity. | ERR.1 to ERR.5 |
| LQ18 | Tests and spikes remain requirements. The owner decides whether experiments may run in this stage; this chapter authorizes none. | ERR.1, ERR.30 |
| LQ19 | No gate is relaxed. Chapter 12 records the owner-selected freeze order; no acceptance here substitutes for either freeze ruling or sign-off. | ERR.1, ERR.30 |
| LQ20 | Plans and picking retain typed refusals. The renderer handoff or frozen-feature alternative changes supported content and adapter work. | ERR.18, ERR.23, ERR.25; EV3 table |
| LQ21 | Both flow spellings stay reserved until one is retired before use; chapter 07 decides the rule/form under the owner's selected mechanism. | ERR.17, ERR.24 |
| Envelope additions the owner may overrule | ERR.9's form, ERR.10's span and ERR.11's after_user_request are new D additions to V06 from Round 3. Removing one requires an explicit replacement for the diagnostic, job-stop or fresh-action distinction it serves. | ERR.8 to ERR.11 |
| Span convention [O] | EV1.10 and LP1 choose byte/scalar/host units and endpoint conversion. | ERR.12 |
| Flow rule and form [O] | Chapter 07, LQ2 and LQ21 settle the active flow row and retire the other; no selection is implied by the reconciliation column. | ERR.24 |
| Plan-limit classification [O] | RPI.50 and LP1 choose declared allowances versus fixed ceilings, their units and amounts. Reserved rows cover the alternatives without choosing one. | ERR.25 |
| Recursion guard [O] | DET.28 and LP1 choose the non-budget guard or logical stack/memory treatment and its count. | ERR.26 |
| Chapters 06 and 07 | Chapter 06 confirms the four EVD rows and adds none (EVD.58). Chapter 07 adds none and uses the rows of BND.83. | ERR.21, ERR.23, ERR.24 |
| Source differences and integration | Section 6 records the spellings the chapters first used, and the chapters now use the registry codes. Round 3's two budget-admission rows are retained but retired before use. Chapters 01 and 08's current commit and safety-stop corrections are preserved. | ERR.16 to ERR.29 |
| LP1, EN3 and EN4 | Final envelope encoding, typed context, command/revision transport, shared limits and the declared charge schedule still need their owning contracts. No new numerical limit is supplied here. The retained primitive ceiling is V05.1's source compatibility fact; fixed ν coordinates are ENG-0J A5's I_a-only invariant. | ERR.8, ERR.16, ERR.23, ERR.25, ERR.26 |
| DT-L1 obligations | Round 3 cleared them (overview section 8). R2-E1 supports stored evidence/scope, R2-E3 supports provenance/flow, and R2-E4 supports partial-result and principle-record references where cited, each as round 3 corrected it. | ERR.21, ERR.24; LB6 and PS1 tables |
| Design hold and evidence | Built-in tools retain the 2 October boundary and 17 October exception until the joint setting. No UI design or DT-J2 to DT-J4 work is opened. Required tests have no results here; the writing check proves structure only. | ERR.1, ERR.13, ERR.30 |

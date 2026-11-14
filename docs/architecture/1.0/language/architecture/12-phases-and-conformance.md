# Chapter 12: phases and conformance

Status: **draft for the owner. Not accepted.** Conventions are in [the overview](00-overview.md), section 3.

A phase ends when its acceptance is evidenced. This chapter makes the partition's phase table checkable, connects it
to the other engineering lines, and assigns the existing chapters' test requirements to their first runnable phase.
It records no completed language phase, implemented conformance test or new measurement. The owner answered LQ1 to
LQ21 on 6 November 2026 and chose LQ8, LQ9 and LQ21 on 14 November 2026: the engine and the interpreter's host in
Rust, route RT1 with the runtime in a process of its own, and capability separation (RTE.17, RTE.21, RTE.24). LQ13
waits for its evidence, and LQ2's option set waits for the joint setting.

Sources: [the partition](../ecosystem-partition.md), sections 12 to 17 and INV-1 to INV-12;
[DT-L1](../../../../progress/1.0/design-table/topics/language-ecosystem.md), owner input, E4, E6, Outcome and Round 3;
[chapters 01](01-kernel.md), [02](02-language-core.md), [03](03-libraries.md), [04](04-environment.md),
[05](05-persistence.md), [06](06-evidence.md), [07](07-boundary.md), [08](08-one-state.md), [09](09-determinism.md),
[10](10-renderer-platform.md), [11](11-route-and-spikes.md) and [13](13-errors.md);
[EN1 to EN4](../../../../progress/1.0/replan-2026-10-22.md);
[the charter](../../../../progress/1.0/charter-2.0-draft.md), sections 5 and 8;
[H-08](../../../../progress/1.0/command-table.md);
[the experiment protocol](../../../../progress/1.0/stage-2-experiment-protocol.md);
[the renderer experiment plan](../../../../progress/1.0/renderer-experiment-plan.md);
[renderer selection](../../../../wiki/decisions/renderer-candidates.md);
[owner, 3 November](../../../../wiki/decisions/owner-decisions-2026-11-03.md);
[owner, 6 November](../../../../wiki/decisions/owner-decisions-2026-11-06.md);
[path B](../../../../wiki/decisions/owner-decisions-2026-10-02-migration.md) and
[its amended design](../../../../progress/1.0/migration-exporter-proposal.md);
[the research handover](../../../../../research/jumbling/restoration/README.md);
[the 600-cell-Full oracle](../../../../../tools/oracle/) and
[the J1 harness](../../../../../research/jumbling/differential/README.md).

## 1. Rules for every phase

- **PHS.1 [B]** There are no phase day counts or target days here. The owner set the earlier schedule aside on
  3 November 2026. Nothing is automatically cut, moved or relaxed when work or an input is late. The owner receives the
  blocked acceptance item, the missing input and the choices; the dependent phase waits.
- **PHS.2 [D]** The product, acceptance and dependency tables below expand partition section 14 under the owner's
  answers of 6 November 2026. A product is delivered only when its named content is available for inspection. An acceptance item
  needs its stated check and evidence class. A missing check, an invalid run and an unresolved prerequisite are open,
  never a pass. Sections 9 and 10 give the answer state and the order.
- **PHS.3 [B]** Every phase preserves the mechanical identities, exact decisions, protection and single mutation path
  of partition INV-1 to INV-11 and `AGENTS.md`. Chapter 01 owns the kernel vocabulary and chapter 08 its lifecycles.
  A phase cannot make a failed oracle comparison acceptable or substitute a normal form for literal execution.
- **PHS.4 [D]** Each exit record identifies its candidate, products, applicable owner decisions, acceptance items,
  checks and evidence references, with class and outcome for each. It lists what stays open. This follows from
  partition section 14's acceptance gates and the experiment protocol's result card. A source revision, fixture or
  build change requires checking which evidence still applies; a pass on another candidate is not transferred by name.
- **PHS.5 [B]** At each phase the mathematician and engineer advisor explain its key concepts, mathematical principles
  and trade-offs in plain words, and the owner confirms understanding before anything is finalised (DT-L1 owner input).
  Their agreement is not acceptance evidence. The owner decides scope and sign-off; Astra gives the gate rulings.
- **PHS.6 [B]** Evidence retains overview section 3.4's classes and `AGENTS.md`'s limits. A headless result cannot
  establish Windows/DirectX, actual input, long-session or performance behaviour. A human check needs people and the
  owner's test form. A model review is recorded as review; it cannot stand in for an oracle, measurement or human test.
- **PHS.7 [D]** The interim tool boundary applies throughout development until the joint setting of LQ2 (partition
  section 11). Built-in tools keep the 2 October boundary and the 17 October named-cap repair-list exception. No
  solver-tool solving preview is enabled in advance of that setting. LP3's closed loop uses admitted built-in
  operations and the solver's action, not an assumed permission for solver-authored assistance.
- **PHS.8 [B]** This architecture stage authorises no product implementation (DT-L1 owner input). LQ18 allowed
  experiment code as LX-1 to LX-9, with cards, subscription quota only and no product code (owner, 6 November 2026).
  Two allowances of that day add one fetch of a big-integer crate for LX-5 after a licence check and one installation
  of Rust, Node and .NET in WSL (decisions 13 and 14). Nothing else here authorises an API-team run, new cost, tool
  installation or research restart. The design line stays closed under the 3 November decision; DT-J2 to DT-J4 remain
  closed. References to later design deliverables prescribe no interface appearance.

## 2. LP0: contracts

- **PHS.9 [D]** LP0 produces the following inventory, following partition LP0 and section 17. The current-state column
  is source inspection of this branch, not an LP0 exit declaration. This chapter is the packet's new draft.

| Product | Required at exit | Current source state |
| --- | --- | --- |
| Partition | Direction, boundaries, defaults and owner-option record | `ecosystem-partition.md` exists; direction confirmed; its text stays the record of the questions the owner answered on 6 November 2026 |
| Architecture 00 | Overview, conventions, walk-throughs and trust table | `00-overview.md` exists as a draft |
| Architecture 01 | Kernel | `01-kernel.md` exists as a draft |
| Architecture 02 | Language core | `02-language-core.md` exists as a draft |
| Architecture 03 | Libraries | `03-libraries.md` exists as a draft |
| Architecture 04 | Environment | `04-environment.md` exists as a draft |
| Architecture 05 | Persistence | `05-persistence.md` exists as a draft |
| Architecture 06 | Evidence and certificates | `06-evidence.md` exists as a draft |
| Architecture 07 | Boundary mechanisms and option sets | `07-boundary.md` exists as a draft |
| Architecture 08 | One state | `08-one-state.md` exists as a draft |
| Architecture 09 | Determinism | `09-determinism.md` exists as a draft |
| Architecture 10 | Renderer/platform independence | `10-renderer-platform.md` exists as a draft |
| Architecture 11 | Routes and spikes | `11-route-and-spikes.md` exists as a draft |
| Architecture 12 | This phase and conformance chapter | This draft |
| Architecture 13 | Error registry | `13-errors.md` exists as a draft |
| ADR 1 | One mechanics and one authoritative state: derived states and preview exception | [`ADR-L01`](../adr/ADR-L01-one-state.md) exists as a proposal |
| ADR 2 | Runtime as command-layer client, off the critical path | [`ADR-L02`](../adr/ADR-L02-runtime-as-client.md) exists as a proposal |
| ADR 3 | Exactness, numeric tower and algebras | [`ADR-L03`](../adr/ADR-L03-exactness.md) exists as a proposal |
| ADR 4 | Canonical identity, identity tower and source maps | [`ADR-L04`](../adr/ADR-L04-canonical-identity.md) exists as a proposal |
| ADR 5 | Evidence model and certificates | [`ADR-L05`](../adr/ADR-L05-evidence.md) exists as a proposal |
| ADR 6 | Effects, capabilities and flow control | [`ADR-L06`](../adr/ADR-L06-effects-and-flow.md) exists as a proposal; its flow-control form is capability separation (owner, 14 November 2026, LQ21; RTE.24) |
| ADR 7 | Typed IR as stable contract | [`ADR-L07`](../adr/ADR-L07-typed-ir.md) exists as a proposal; follows the route choice |
| ADR 8 | Jobs and deterministic budgets | [`ADR-L08`](../adr/ADR-L08-jobs-and-budgets.md) exists as a proposal |
| ADR 9 | Boundary mechanisms, policy left to joint setting | [`ADR-L09`](../adr/ADR-L09-boundary-mechanisms.md) exists as a proposal |
| ADR 10 | Persistence formats, data core and editions | [`ADR-L10`](../adr/ADR-L10-persistence.md) exists as a proposal |
| ADR 11 | Display plans and user stages | [`ADR-L11`](../adr/ADR-L11-display-plans.md) exists as a proposal |
| ADR 12 | Implementation route and runtime placement | [`ADR-L12`](../adr/ADR-L12-route.md) exists as a proposal; follows the owner's 14 November 2026 answer (RTE.21) |
| Evidence import table | Default 20's versioned table, enumeration rule and recorded inferences | [`evidence-import/`](../evidence-import/README.md) exists with `corpus-labels.json`, `corpus-labels.md`, `extract_labels.py`, `rules.txt`, `refs.txt`, `ext.txt`, `rule.txt`; draft, with V14, P13, K1 and H1 ruled on by EVD.52 |
| Glossary | Proposed entries for every language term in chapter 04 section 6; integrate into `terms.json` at LP1 before any LP2 parser or library text uses a term (default 8) | Existing [`terms.json`](../../../../progress/1.0/glossary/terms.json), [`glossary.md`](../../../../progress/1.0/glossary/glossary.md), diagrams and [`build.py`](../../../../progress/1.0/glossary/build.py); chapter 04 section 6 holds EV5.14's proposed additions and changes, not yet integrated |
| Owner answers | Answers or explicitly accepted decision procedures as specified in section 9 | [Answered on 6 November 2026](../../../../wiki/decisions/owner-decisions-2026-11-06.md); LQ8, LQ9 and LQ21 were chosen on 14 November 2026 (RTE.17, RTE.21, RTE.24), LQ13 waits for its evidence, and LQ2's option set waits for the joint setting |

- **PHS.10 [D]** LP0's acceptance decomposes partition LP0 as follows. The inventory check supports the review; it
  does not replace the review or the owner's acceptance.

| Acceptance item | How checked | Evidence class |
| --- | --- | --- |
| Complete contracts ready for review | Inspect every inventory entry, cross-chapter owner, glossary reference and default-20 mapping or explicit unmapped row; inspect that no clause relies on more than overview section 8 allows | source |
| Astra review with no open blocker or major | At least two independent concurrent shards review the current full proposal; the integrator records every finding's disposition and verifies closure under `AGENTS.md` | source, review record only |
| Owner answers at their proper scope | Compare the answer record with every row of section 9, including accepted procedures and the deferred LQ2 joint setting; obtain understanding confirmation under PHS.5 | human |
| ADR proposals ready for acceptance | Every proposal is reviewed and ready; ADR 6, ADR 7 and ADR 12 are completed from the owner's 14 November 2026 answers (RTE.17, RTE.21, RTE.24). The owner accepts ADRs only after both LP1 rulings (PHS.15) | human |

- **PHS.11 [D]** LP0 has no preceding language-phase dependency (partition LP0). Sources in force still bind it.
  LQ18 allowed LX-1 to LX-9 during LP0 (owner, 6 November 2026); the absence of an earlier phase gives no other
  permission.
- **PHS.12 [D]** At LP0 exit, execution evidence and the conformance implementations remain open. LQ8, LQ9 and LQ21
  are answered (owner, 14 November 2026; RTE.17, RTE.21, RTE.24), LQ13 awaits LX-8 before the parser, and LQ2's option
  set awaits the joint setting before LP6 (owner, 6 November 2026). The answer procedures are settled, not those
  last two choices. Note N1 keeps the status overview
  section 8 states, and no clause reads labels off the lattice.

## 3. LP1: interfaces and evidence

- **PHS.13 [D]** These products preserve partition LP1's scope. Chapters 01 to 10 specify their meaning; LP1 turns
  their open interface choices into the specification supplied to the gates.

| Product | Inspectable content |
| --- | --- |
| Allowed evidence spikes | LX-1 to LX-9 experiment cards, candidate/input identities, negative results and class-limited reports, in chapter 11's order; allowed by LQ18 |
| Decisions | LQ8, LQ9 and LQ21 answered on 14 November 2026 (RTE.17, RTE.21, RTE.24); LQ13 after LX-8 before LP2's parser |
| Remaining ADR material | ADR 6's flow-control form, ADR 7's execution contract and ADR 12's route/placement, settled by the owner's 14 November 2026 answers (RTE.17, RTE.21, RTE.24), for the owner's acceptance after both rulings |
| Kernel interface specification | Snapshot handles, batch queries, derived states/value store, preview/token binding, receipts, availability, store query rows, independent checkers, jobs and deterministic budgets; chapter 01 and chapter 08 |
| Identity and accounting specification | Canonical IDs and level-qualified digests, normalised bytes, metering and logical resource units; KN1.8, KN7.7 to KN7.10, LG4.8, chapter 09 |
| Language-neutral data core | Chapter 05's formats and complete path B coverage: the data core freezes and the evidence envelope stores facts (owner, 6 November 2026, LQ10) |
| Final H-08 input | Runtime needs reconciled with EN4's command rows, program/view lanes and undo during jobs; KN5 and KN7.2; the view and recording rows of the language delta |
| Renderer handoff | Handoff row H-11 (owner, 6 November 2026, LQ20): display-plan primitives and their support/evidence requirements under RPI.57, the user-stage contract and the free-view workload; ONE.41's presented-preview report goes to the renderer line (RPI.66) |
| User-stage contract | LG2.24, LG3.17, LG5.12 and RPI.68 to RPI.80: the stage module's checker and effects, raster bound, quarantine, CPU reference and compile path, with RTE.83's safety test set |
| Recording and playback | EV4.10 to EV4.23 and PS1.30, PS1.44: recorded steps, unattended view and workspace playback and their stored forms (owner, 6 November 2026, LQ11). A commit never plays unattended |
| Glossary integration | Chapter 04 section 6's reviewed entries integrated into `terms.json` with EV5.15's build evidence before any LP2 parser or library text uses a term |

- **PHS.14 [B]** The owner allowed the spikes on 6 November 2026 (LQ18). Each starts with the experiment protocol's
  card, including the question, decision, method, kill criteria and evidence class. Chapter 11 owns the routes, spike
  protocols and decision rules; DT-L1 Round 3 supersedes earlier figures and sketches. LX-1 is a lead and cannot pass
  charter E2. LX-2's fault controls compare the same commands with and without the fault, rather than requiring an
  unchanged journal while intended commits occur. LX-7's GPU comparison names its actual device even when it runs
  without a window. Its correctness comparison is separate from its PresentMon gate run. [D] First-pass records of
  LX-1 to LX-9 exist, summarised by RTE.82 at their frozen scope. LX-5 and LX-6's decisive runs ran on 14 November
  2026, and the owner accepted them (RTE.82). The decisive LX-1, LX-2, LX-3 and LX-4 checks are LP3 acceptance checks
  with their bars unchanged (RTE.21, RTE.24). The remaining decisive runs, LX-7 to LX-9 and LQ13's LX-8 among them,
  wait for the owner's window (owner, 6 November 2026, decision 15).
- **PHS.15 [D]** LP1's acceptance makes partition LP1 checkable without requiring the unbuilt LP2 product to pass its
  future tests. Contract vectors, reference evidence and experiment results identify which assertions they support.

| Acceptance item | How checked | Evidence class |
| --- | --- | --- |
| LQ8 answered (owner, 14 November 2026; RTE.17) | LX-5's exact-equality/licence report and LX-6's oracle comparison ran on the owner's machine and were accepted as decisive (RTE.82); LP2 chooses the big-integer implementation on LX-5's data | source; performance for speed; human for decision |
| LQ9 and LQ21 answered (owner, 14 November 2026; RTE.21, RTE.24) | LX-1, LX-2 and LX-4 become LP3 acceptance checks with their bars unchanged, and if one fails LQ9 reopens; LX-3's decisive check is part of LP3 acceptance; false refusals are presented without inventing a threshold | source; windows for native fault/input observations; performance for timings; human for decision |
| Architecture-freeze gate | Astra rules on the current packet, including final command table, layer interfaces and renderer feature list, under section 11 | source, gate ruling only; referenced runs keep their own classes |
| Migration-format-freeze gate | Astra rules on EN4's complete format packet under section 11 and LQ10's answer | source, gate ruling only; referenced runs keep their own classes |
| ADRs accepted | After both rulings the owner accepts the language ADRs, the deferred material against the actual route, placement and flow-control evidence | human |
| Owner sign-off | Record the owner's sign-off after both rulings and understanding confirmation, with the charter amendment named | human |

- **PHS.16 [D]** LP1 depends on LP0, on LQ18's allowance for the spikes and on LQ10's answer, both given on
  6 November 2026; on EN3's command rows, view-model fields and jumbling interfaces; and on EN4's final H-08, layer
  interfaces and migration-format packet (partition LP1). Undo while a job runs and the program/view lanes are required
  EN4 inputs, not optional follow-ups.
- **PHS.17 [D]** At LP1 exit the two rulings and owner sign-off are evidenced; LP2 product conformance, the importer,
  on-screen integration and release measurements remain open. LQ13 may still await its choice before the parser; LQ2's
  policy may still await the joint setting. The design-system ADR remains open under the charter amendment of
  6 November 2026 (LQ19). No statement here declares either freeze accepted.

## 4. LP2: headless core

- **PHS.18 [D]** These are partition LP2's products. The API tests use a candidate behind the boundary, not the
  reference engine on both sides as a substitute for candidate equality.

| Product | Inspectable content |
| --- | --- |
| Kernel API over both puzzles | Chapter 01's interfaces, with a public J1 API replacing adapters' private-member access before binding (default 12) |
| 600-cell-Full candidate adapter | Existing oracle cases O01 to O07 run through the kernel API, plus the language's new cases |
| Language core | Parser, checker, typed IR/execution contract, interpreter, budgets and cooperative cancellation; chapter 02 and chapter 09 |
| Conformance suite | Runnable checks for available core products and the index of later obligations; section 15, with no later-phase test marked passed merely because it is indexed |

- **PHS.19 [D]** Each acceptance item below is a separate check from partition LP2. Source results on both operating
  systems establish exact bytes, not native Windows interaction.

| Acceptance item | How checked | Evidence class |
| --- | --- | --- |
| Both oracles pass on the API | Compare the candidate against the 0.4 reference and against J1 step by step, including refusals and full collateral; first difference fails | source |
| Cross-platform conformance and determinism | Run identical pinned inputs/keys on Windows and headless Linux; compare canonical exact payloads and stop reasons; apply the declared precision only to display fields | source |
| Exact decision paths | Inspect call/type paths and use refusal fixtures to exclude floats outside INV-4's named sites | source |
| Preview swap refused | Replace the pending preview after presentation and before commit; token binding refuses it and no state changes | source |
| Repeatable budget exhaustion | Repeat each declared-budget case, including cache/eviction variants, and compare stop step, coverage and charges | source |
| Centre-0 protection | Refuse the centre-0 pose-level case inside Jumble and pass its label-level case after the crossing (DT-L1 R2-E2) | source |
| Heights and domain IDs agree | Compare the kernel with the library reference and `trees.py` on ENG-0J acceptance item 8's states and H6030, and check the free-mode case of CT-KN2-02 (DT-L1 R2-E2, note N1) | source |
| R-11's jumbling property | Under LG2.14's four hypotheses, check that the equality key preserves admissible starts and end digests; execution and prefix protection still use literal letters (default 10; DT-L1 R2-E2, note N1) | source |
| Gauge transfer | Run CT-KN2-06 with S, R and σ ≠ id cases, on and off the lattice. It checks the engine against note N1, which stands at (theorem, producer) under chapter 06's grades; a pass adds no proof (KN1.14; overview section 8) | source |

- **PHS.20 [D]** LP2 depends on LP1's gates and the owner's sign-off; under the charter amendment of 6 November 2026
  it does not wait for the design-system ADR (LQ19, partition LP2). The syntax choice LQ13 precedes the parser. An
  unresolved approximate-control-flow, word-key or metering contract cannot be guessed by its checker implementation;
  PHS.55 lists the owning specifications.
- **PHS.21 [D]** LP2 exit leaves the three-tool loop, native renderer integration, authoring services, full research
  corpus checking and path B importer open. Its headless pass supplies none of LP3's performance, LP4's human or LP6's
  product evidence. Section 9's LP2 column records the required owner-answer state.

## 5. LP3: first closed loop

- **PHS.22 [D]** LP3 delivers the three built-in tools of partition LP3: one for 600-cell-Full, one for jumbling and
  one for representations, written in the language above the kernel (owner, 6 November 2026, LQ5). Each demonstrates
  the complete loop below.

| Product | Inspectable content |
| --- | --- |
| Three-tool vertical path | Source, check, run, view nodes, engine preview, explicit user commit, durable receipt and undo |
| Matching command/key routes | The same absolute arguments, identity and literal operation through source, command and key; kernel-row keys retain ONE.48's direct path |
| Active program views | Revision-bound output, latest-wins recomputation and stale-state data under ONE.11, ONE.12 and ONE.64; no view blocks engine adoption |
| Further views and user stages | `view.open`, `view.close` and RPI.82's budget; user vertex and fragment stages under RPI.68 to RPI.80 on the selected renderer. Support is advertised only after RPI.79's run |

- **PHS.23 [D]** LP3's acceptance checks the result and the whole turn interval, following partition LP3 and DT-L1 E4.

| Acceptance item | How checked | Evidence class |
| --- | --- | --- |
| Same result through all routes | Replay the three tools through source, command and key with pinned absolute arguments; compare literal digests, outcomes, receipts and undo results | source; windows for the actual key route |
| Oracle equality at every committed step | Compare every state transition and refusal against the matching puzzle oracle, retaining identity and protection level | source |
| Presented-preview binding in the selected renderer | Exercise the gap between received and presented previews under actual frame pacing; the chord binds the presented preview (ONE.41) | windows |
| Charter E2 with the tools' views active | Measure key to first correct full frame, including admission, apply, durable store transaction and adoption; p95 at most 100 ms over at least 100 observations, on one frozen candidate | performance |
| Stage and view contract on the selected renderer | On the actual device a stopped frame runs no user-stage fragment work, picks agree with the drawing pass and a device fault quarantines the chain's user-stage digests; the advertised combination passes the unchanged gate in one run (RPI.72, RPI.77, RPI.79; RM.40) | windows; performance for the gate run |

- **PHS.24 [D]** LP3 depends on LP2, frozen H-08 and the renderer freeze for its on-screen half (partition LP3).
  Headless integration work does not pass the on-screen acceptance early. The E2 figures are decision thresholds,
  not measurements here: charter E2 and the B4-12 table of
  [`10_V1_ARCHITECTURE.md`](../../10_V1_ARCHITECTURE.md), with DT-L1 E4's interval clarification.
- **PHS.25 [D]** At LP3 exit, the complete authoring environment, archive expressiveness and importer remain open.
  Passing this small loop does not pass every tool, the full shipped workload or final human comprehension. LQ2's
  joint setting can remain open; built-ins continue under PHS.7. The design-system ADR stays outstanding under the
  charter amendment (LQ19).

## 6. LP4: authoring environment

- **PHS.26 [D]** LP4's products and checks follow partition LP4. Functional authoring services are separable from
  the editor's appearance, which remains with the closed design line.

| Product | Inspectable content |
| --- | --- |
| Editor services | Recovery, completion, diagnostics, navigation and version separation of EV1 |
| Debugger and replay | EV2's read-only stepping, replay, checker traces and budget profile |
| Linkage and authoring actions | EV3 source linkage and declared editable data; EV4 bindings, macros, semantic recording and unattended playback of view and workspace steps (owner, 6 November 2026, LQ11) |
| First edition of docs and learning | EV5 reference generation, principle explanations, learning path and the glossary entries integrated at LP1, under the mathematician's oversight |

- **PHS.27 [O]** The owner must set the non-author protocol and success criteria before LP4's human acceptance
  (EV5.12). The LX-4/editor bars from DT-L1 Round 3 are unmeasured proposals whose adoption belongs to the owner and
  chapter 11. No source supplies a comprehension pass rate or settles BS3's changed-principle validity rule
  (EV5.9). Possible rubrics distinguish fixture correctness, transfer, help and time; this chapter chooses none.

| Acceptance item | How checked | Evidence class |
| --- | --- | --- |
| Editor latency bars | Run the adopted LX-4 protocol on the owner's machine, with pinned modules/edit script, both puzzles, a quiet machine and frozen candidate; compare each required series with the adopted bars | performance |
| Non-author tests | People use the authoring and learning tasks in the owner's form; record correctness, transfer, help and time separately, with mathematical content checked by the mathematician | human |

- **PHS.28 [D]** LP4 depends on LP3 and the shell (partition LP4). Under the charter amendment of 6 November 2026 (LQ19)
  its functional products can be developed while the design-system ADR waits; the editor's appearance waits for that ADR after the
  language's development is complete. This split cannot be reported as a fully completed LP4 while its deferred part
  is still open.
- **PHS.29 [D]** At the functional LP4 exit, research and migration acceptance can still be open, as can that
  deferred editor part. Human protocols or latency bars that remain unset prevent their acceptance items passing.
  The owner's understanding confirmation and section 9's LP4 decision state are required before anything is finalised.

## 7. LP5: research expressiveness

- **PHS.30 [D]** LP5's products exercise the recorded research, following partition LP5. The material is data and
  proof/certificate records with their original review status; bringing it into the language resumes no research.

| Product | Inspectable content |
| --- | --- |
| Research library | Chapter 03's research entries, including family recognition, ranks, conditional models and symbolic bounds |
| Archive import and independent checking | Stage 14 to 19 formats, manifests and word members through PS3's versioned import table and KN6 checkers |
| Reproducible research work | Engine-owned branches (owner, 6 November 2026, LQ4), derived states and run records |
| Proof-obligation export | LB5.18's `research.obligation.export` in PS1.45's format through a granted file capability (owner, 6 November 2026, LQ15). Lean does not ship, and formal proofs come after 1.0 |

- **PHS.31 [D]** LP5's acceptance is the following decomposition of partition LP5. A finite archive check does not
  settle its global coverage obligations or manufacture missing producer code.

| Acceptance item | How checked | Evidence class |
| --- | --- | --- |
| Certificates and verdicts rechecked and matched | Independently check every checkable claim against its immutable inputs; compare with the format-specific recorded meaning, including scope, original label and check mode; mutation controls must fail | source |
| Missing dependencies stay visible | Separate self-contained checks from LB5.8's producer-dependent claims; preserve composed-only and incomplete literal replays as such, without marking an unavailable required check passed | source |
| A run record reproduced bit for bit | Reproduce a complete record with available pinned inputs, modules, exact payload and stop reason under chapter 09; an archive lacking its producer cannot supply this demonstration by itself | source |
| No new search counted as acceptance | Inspect the acceptance inputs/commands against the frozen archive and replay plan; searches, new strategy results and expanded admitted domains supply no LP5 pass | source |
| Proof-obligation export keeps its limits | Export a supported statement with its pinned vocabulary through a granted file capability; an untranslatable statement writes nothing; the export promotes no claim or commit authority (LB5.18, PS1.45) | source |

- **PHS.32 [D]** LP5 depends on LP2, LP3 and the research results in the repository (partition LP5). It does not
  depend on LP4's editor appearance. The import baseline is the Git-blob manifest, not a checkout walk (default 13;
  DT-L1 R3-E2). The mapping and envelope depend explicitly on DT-L1 R2-E1; provenance on R2-E3; trace/principle
  linkage on R2-E4, each as round 3 corrected it (R3-M1, R3-M3, R3-M4).
- **PHS.33 [D]** At LP5 exit, product import, packaging, final documentation and machine gates remain open. Global
  research claims that were open remain open; an admitted-domain rejection does not become unreachability. LQ2's
  policy must still be set jointly before LP6. If a mandatory archived check lacks its dependencies, LP5 waits and
  the owner is told; preserving the record as recorded is not a replacement acceptance.

## 8. LP6: product

- **PHS.34 [D]** LP6's product list is partition LP6's list, with each item exposed for the owner's decision.

| Product | Inspectable content |
| --- | --- |
| Path B importer | Every target of partition section 13 and PS3.3: recipes, macro sets, protection, filters, preferences, checkpoints with their own replay paths, twist-key words and command aliases |
| Packaging | Traceable product build, declared files and dependencies, recovery and edition compatibility |
| Complete documentation | All shipped modules, principle records, errors, glossary, supported data and migration outcomes |
| Boundary policy as decided | Joint LQ2 policy, chosen LQ21 mechanism, profile grants, labelled records and reviewed built-in classes |
| Owner-machine gates | Whole-product performance and applicable native, input, long-session and human requirements for the frozen release candidate |

- **PHS.35 [D]** Partition LP6's acceptance is owner sign-off. The following checks provide its inputs; none
  delegates that sign-off to a model or a structural checker.

| Acceptance item | How checked | Evidence class |
| --- | --- | --- |
| Import coverage and preservation ready for sign-off | Run path B fixtures against the product importer: recipe equality, all offered checkpoint paths, protection, preferences, key conversions, source preservation and transactional recovery | source; windows for the actual lock/watch/publication path |
| Product gates ready for sign-off | Run the unrelaxed renderer gate, charter E2 and applicable charter E3/E4/E7 and human checks on the matching candidate; retain unrun/failed requirements visibly | windows, performance, human as required by each check |
| Owner accepts the product | Present the product inventory, complete acceptance/evidence record, joint policy and open issues after PHS.5's explanations; record the owner's decision | human |

- **PHS.36 [D]** LP6 depends on the session store/migration line, a selected and frozen renderer, the design-system
  ADR after the design line, and owner gates (partition LP6; owner, 6 November 2026, LQ19). Complete docs and LQ16's cut
  require the applicable earlier products. The remaining LP4 editor part follows the design-system ADR before product
  sign-off.
- **PHS.37 [D]** An LP6 exit record still lists research conjectures, unsupported future editions and features
  outside the chosen cut; it does not present them as passed. Every requirement inside the cut has its acceptance or
  an explicit new owner disposition, never an automatic omission. This draft asserts no such exit or disposition.

## 9. Owner-answer state at each exit

- **PHS.38 [B]** The matrix records the answer state after the owner's answers of 6 November 2026. **A** means the
  owner has answered that question at the stated scope; later columns retain that answer. **Procedure** means the owner
  has accepted how it will be decided, with the choice still open. **Joint pending** means that the owner accepted the
  joint-setting process and interim boundary, but the option set is still open. **Syntax pending/A** allows a choice
  after LX-8 up to the parser prerequisite. LP4 means its functional exit.

| Question | LP0 | LP1 | LP2 | LP3 | LP4 | LP5 | LP6 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| LQ1 Language/profile split | A | A | A | A | A | A | A |
| LQ2 Boundary policy | Joint pending | Joint pending | Joint pending | Joint pending | Joint pending | Joint pending | A, joint setting before entry |
| LQ3 Evidence model | A | A | A | A | A | A | A |
| LQ4 Branches/pinned jobs | A | A | A | A | A | A | A |
| LQ5 Built-in backing | A | A | A | A | A | A | A |
| LQ6 Views | A | A | A | A | A | A | A |
| LQ7 User stages | A | A | A | A | A | A | A |
| LQ8 Engine language | Procedure | A, answered 14 November 2026 | A | A | A | A | A |
| LQ9 Route/placement | Procedure | A, answered 14 November 2026 | A | A | A | A | A |
| LQ10 Data freeze | A | A | A | A | A | A | A |
| LQ11 Recording | A | A | A | A | A | A | A |
| LQ12 Sharing | A | A | A | A | A | A | A |
| LQ13 Syntax | Procedure | Syntax pending/A after LX-8 | A before parser | A | A | A | A |
| LQ14 Learning gates | A | A | A | A | A | A | A |
| LQ15 Formal proofs | A | A | A | A | A | A | A |
| LQ16 The 1.0 cut | A | A | A | A | A | A | A |
| LQ17 Name | A | A | A | A | A | A | A |
| LQ18 Spike permission | A | A | A | A | A | A | A |
| LQ19 Charter/freeze order | A | A | A | A | A | A | A |
| LQ20 Renderer handoff | A | A | A | A | A | A | A |
| LQ21 Flow-control form | Procedure | A, answered 14 November 2026 | A | A | A | A | A |

- **PHS.39 [D]** A joint LQ2 choice must also settle its edge rulings; merely naming BS1, BS2, BS3 or BS4 does not
  answer them.
- **PHS.40 [B]** LQ10's facts answer removes a *format* dependency on LQ3 (DT-L1 R2-E1). LQ17 is settled: the language
  is 600 Cell Script and its source files use `.6cs` (owner, 6 November 2026). Neither a passed test nor an ADR draft
  answers a question the owner has not answered.

## 10. Order

- **PHS.41 [B]** The following dependency order is acyclic (owner, 6 November 2026, LQ18, LQ19). Parallel branches
  denote separable work, not waived dependencies. Language development is complete when LP2 to LP5 are done apart from
  LP4's editor look (decision 7); no part of this graph resumes design by itself.

```text
LP0 -> LP1 interfaces and allowed spikes -> both Astra rulings -> owner sign-off -> LP2 -> LP3
LP3 -> LP4 functional work
LP2 + LP3 -> LP5
LP4 functional work + LP5 -> design line -> accepted design-system ADR -> remaining LP4 editor part
completed applicable language products + design-system ADR + migration/renderer/owner gates -> LP6
```

- **PHS.42 [B]** Within the evidence branch, LX-5 and LX-6 feed LQ8 first. LX-3, LX-7 and LX-8 need not wait
  for LQ8. LX-9's reference-platform part is independent; its engine-candidate part follows LX-6. LX-1, LX-2 and LX-4
  follow LQ8's host choice. Then the applicable LX-1 to LX-5 evidence feeds LQ9 and LQ21 together, before the LP1
  gates; ADR acceptance follows both rulings. LX-8 feeds LQ13 before the parser. This is partition section 12's
  order; LQ8, LQ9 and LQ21 are now answered (owner, 14 November 2026; RTE.17, RTE.21, RTE.24), and LX-1, LX-2 and
  LX-4 became LP3 acceptance checks. Chapter 11
  defines the spike decision rules, including no speed threshold where Round 3 supplies none. LX-2's conditions 3 and 4
  (RTE.42) and LX-4's protocol (RTE.51 step 8, RTE.53) require a renderer candidate that presents frames, with the
  renderer gate and E2 measured in the same run. Their LP3 acceptance checks wait for a renderer candidate. The
  existing candidates run first, and a changed candidate requires matching reruns under RTE.37 (owner, 6 November
  2026, decision 15).
- **PHS.43 [B] Withdrawn.** LQ18 allowed the spikes before the freeze (owner, 6 November 2026).
- **PHS.44 [B]** The charter is amended to allow LP2 to LP5 after both LP1 rulings and sign-off (owner, 6 November
  2026, LQ19). The design line resumes when LP2 to LP5 are done except LP4's editor appearance. The accepted
  design-system ADR then conditions the deferred LP4 part and LP6. The whole stage 2.5 exit keeps all three parts and
  remains incomplete until the design-system ADR is accepted; only their order changes for the language line.
- **PHS.45 [B] Withdrawn.** Rejected alternative of LQ19 (owner, 6 November 2026).
- **PHS.46 [B] Withdrawn.** Rejected alternative of LQ19 (owner, 6 November 2026).

## 11. The freeze rulings and the stage 2.5 exit

- **PHS.47 [B]** The signed charter, section 8, requires architecture-freeze and migration-format-freeze rulings and
  an accepted design-system ADR for the stage 2.5 exit. `AGENTS.md` assigns both rulings to Codex Astra. The owner
  accepts ADRs and signs off. A review with no blocking findings, a renderer selection or owner understanding alone
  is not either gate ruling. The charter amendment of 6 November 2026 (LQ19) sets the implementation order of PHS.44,
  and no gate is relaxed.
- **PHS.48 [D]** The architecture-freeze input packet contains the items below, following partition LP1, DT-L1 E6
  and the experiment protocol's freeze scope. The gate applies to the candidate named in the packet, not an earlier
  chapter or H-08 first draft. The current inventory distinguishes supplied drafts from the review, evidence and
  owner approvals still required; it declares no freeze readiness.

| Input | What the ruling must be able to inspect | Current draft input and outstanding work |
| --- | --- | --- |
| Authority and scope | Owner-answer record, cut, LQ18 permission, LQ19 order/amendment, proposed ADR material | The [decision record](../../../../wiki/decisions/owner-decisions-2026-11-06.md), the [partition](../ecosystem-partition.md), PHS.9's ADR proposals and PHS.38's matrix are supplied. LQ8, LQ9 and LQ21 are answered (RTE.17, RTE.21, RTE.24); LQ13, LQ2's option set and the ADR acceptances remain open |
| Architecture and interfaces | Current chapters/ADR drafts, EN3 jumbling interfaces, EN4 layer contracts, final H-08 with its availability protocol, undo and lanes, the new view and recording rows | PHS.9's chapters/ADRs and PHS.53's identified [EN3](../../../../progress/1.0/jumbling-interfaces.md), [EN4](../../../../progress/1.0/layer-interfaces.md) and [H-08 CM.1](../../../../progress/1.0/command-table.md) drafts are supplied, including the integrated 174-command final table candidate. Review, implementation evidence and approval remain open |
| Renderer feature list | Feature support and unresolved requirements, handoff row H-11 with the user-stage contract and the free-view workload, ONE.41's presented-preview report, existing renderer evidence at its exact scope | [Renderer feature matrix](../renderer-feature-matrix.md), RM.1 to RM.3, RM.26 and RM.38 to RM.40, supplies the proposed feature crosswalk, presented-preview protocol and stage and view fields. Adapter implementation, matching device/performance evidence and selection/sign-off remain open |
| Decisions and evidence | Required spike cards/results; exactness/determinism/boundary contracts, the recording rules and the relevant test vectors | Chapters 07/09/11, [contract vectors](../contract-vectors/README.md) and the [format packet](../format-packet/README.md) supply draft contracts and reference vectors. RTE.82 summarises the first-pass spike records. PS1.38/FP.33 report partial source evidence only; complete typed coverage, applicable spike/product evidence and owner decisions remain open |
| Review and limitations | Current-candidate findings and dispositions, evidence classes, open dependencies and note N1's status, with no reading of labels off the lattice | [Closure handoff H15/H16](../closure/HANDOFF.md), [packet manifests](../closure/manifests/) and [claim matrix](../claim-matrix/README.md) supply finding responses and explicit limitations. Current-candidate review and gate rulings remain open; PHS.6/PHS.12 retain evidence limits and N1's status |

- **PHS.49 [B]** The migration-format-freeze input is EN4's packet, expanded in PS2.13: complete path B target
  inventory, identifiers/versions, byte/digest/gzip profiles, shared limits, provenance compatibility, evidence
  envelope, lossless label/import mapping, reference replay/conversion vectors, source-preservation and publication
  contracts, and recorded editions/recovery. It freezes the language-neutral data core of PS2.10, and the envelope
  stores facts (owner, 6 November 2026, LQ10). It explicitly relies on DT-L1 R2-E1 as R3-M1 corrected it: the envelope's
  added facts, and scope fields that are all immutable with none revision-bound (PS1.35, EVD.10, EVD.13). Syntax and
  IR freeze later. No part of this permits loss of a path B target or provenance by default.
- **PHS.50 [D]** Format review uses the original and decoded digests of archived inputs, immutable scopes and each
  digest's level; it keeps permission tokens outside evidence (PS1.35 to PS1.38; INV-5). Current draft vectors can
  specify later importer/API tests. They cannot be recorded as results from that unbuilt importer or API. Actual
  Windows probes remain evidence for their matching source/build and method only (chapter 05, section 13).
- **PHS.51 [B]** Both Astra rulings are LP1 acceptance, followed by the owner's sign-off (owner, 6 November 2026, LQ19).
  That sign-off records the candidate and the charter amendment; the outstanding design-system ADR is named. There is
  no freeze declared by this chapter.
- **PHS.52 [D]** No source here imposes a new serial order between the two Astra rulings. Each waits for its own
  complete inputs; product implementation waits for both and the applicable charter conditions. A changed format,
  layer boundary or renderer feature after a conditional freeze follows its recorded revision/review and owner
  acceptance process. Calling it a later phase does not bypass that process (partition sections 13 to 15).

## 12. Dependencies on the other lines

- **PHS.53 [D]** Each cell lists separate inputs and their first dependent phase, following partition LP1 to LP6,
  chapters 01/05/08/10 and DT-L1 E6. The linked draft identities record supplied specifications, not acceptance of
  another line. Required review, implementation evidence and approval remain dependencies. Every missing-input
  outcome is the same: the dependent phase waits and the owner is told; nothing is cut.

| Line | What the language needs, by item and phase | What the line needs from the language | If not ready |
| --- | --- | --- | --- |
| EN1, input-map amendment | LP1: state-contract amendment A6 for exact rotation requests (owner, 6 November 2026, decision 8) through EN1's Astra check and then the owner, before anything relies on the changed input map | Chapter 11's LX-9 result and the exact request/drag representation of KN3.4 and KN3.7 to KN3.9; sampled agreement is no proof of universal agreement | Dependent phase waits; tell the owner; nothing cut or contract amended implicitly |
| EN3 | LP1: [jumbling interfaces](../../../../progress/1.0/jumbling-interfaces.md), draft 5 November 2026 for ENG-0J revision 2, supplies the four layer contracts, view-model/relation fields and command delta integrated by CM.1. Current-candidate review and approval remain open. LP2: implementation and evidence for the specified exact observation rows | KN2.4's batch fields and level/identity contracts; EV3.8's relation data; ONE.8/ONE.25's dependency/comparison needs, without prescribing interface appearance | Dependent LP1/LP2 waits for the outstanding review, evidence and approval; tell the owner; nothing cut |
| EN4 | LP1: [H-08 CM.1](../../../../progress/1.0/command-table.md) supplies the 174-command integrated candidate with `final: true`; [layer interfaces](../../../../progress/1.0/layer-interfaces.md), `en4-layer-interfaces/1-draft`, supplies six-layer contracts, undo, lanes and availability (LI.18 to LI.26). PS2.13 identifies [format candidate](../format-packet/README.md) `en4-data-core-draft-1`, with its pinned inventory in `candidate.json`. Review, complete typed coverage and matching implementation/evidence remain open; these drafts grant no approval. LP3: command-table freeze still required | Kernel query/overlay rows; revision/dependency fields; preview/token and receipt rules; provenance field; byte, limit and migration requirements from PS2.13 | Dependent LP1/LP3 waits for the outstanding review, evidence and approval; tell the owner; nothing cut |
| Renderer line | LP1: feature support/constraints and handoff row H-11 (owner, 6 November 2026, LQ20); actual device for LX-7 correctness and owner-machine timings in the owner's window; a renderer candidate presenting frames for LX-2 (RTE.42 conditions 3 and 4) and LX-4 (RTE.51 step 8, RTE.53), with the renderer gate and E2 in the same run and reruns under RTE.37 when the candidate changes. LP3's acceptance checks for LX-2 and LX-4 wait for that candidate (RTE.21). LP3: selected/frozen renderer for the on-screen half, with a run-time stage compile path and a way to skip a stopped frame (RPI.80). LP6: matching product gate evidence under renderer-candidates | RPI.57/RPI.58 primitives, source maps and atomic publication; the user-stage contract of RPI.68 to RPI.80 and the free-view workload of RPI.81 to RPI.83 through H-11 before its feature freeze; ONE.41 presented-preview report (RPI.66) | Dependent LP1/on-screen LP3/LP6 waits; tell the owner; nothing cut |
| Mathematical research | LP0/LP1: recorded definitions, proofs, certificates, input identities and open obligations; the mathematician's oversight of EV5. LP5: repository stage 14 to 19 results and checkable archive inputs. Nothing new is searched | Expressible exact types, scopes, conditional claims, independent checker/import contracts, derived states and run records; reports of missing producer dependencies | Dependent phase waits; tell the owner; nothing cut or research restarted |
| Session and migration | LP1: path B targets/contracts and data core. LP2: session store below the language, never opened by its runtime. LP6: transactional importer, locked-copy/WAL handling, source preservation, replay/publication/recovery | KN4 query/command boundary; literal words and identity/digest conventions; PS1 data/provenance/envelope fields; complete checkpoint, key and preference conversion coverage | Dependent LP1/LP2/LP6 waits; tell the owner; nothing cut |
| Design line | Its accepted design-system ADR conditions the deferred LP4 part and LP6 (owner, 6 November 2026, LQ19). Closed until language development is complete, which is when LP2 to LP5 are done apart from LP4's editor look | Completed language behaviour, command/data contracts, principle and learning requirements; no DT-J2 to DT-J4 work is opened here | Dependent phase waits; tell the owner; nothing cut and design not resumed by default |
| Glossary | LP0: proposed entries in chapter 04 section 6 under default 8; mathematician review of EV5.14. LP1: one meaning per puzzle/alphabet before the data core freezes; integration into `terms.json` with EV5.15 build evidence before any LP2 parser or library text uses a term. LP4: docs and learning use the integrated glossary | Owning definitions, learning prerequisites, reviewed term changes and locally reproducible diagram inputs; existing `glossary/build.py` tooling | Dependent LP0/LP1/LP4 waits; tell the owner; nothing cut |
| 600-cell-Full and J1 oracles | LP1 allowed LX-6: O01 to O07 reference observations. LP2: candidate adapter through `tools/oracle/`, public J1 API and `research/jumbling/differential/` comparisons, W-J fixtures, journals and H6030; LP3/LP6: matching step/replay comparisons | API-bound candidate observations, full collateral, refusal stability, identity/menu-order binding and new language cases; no changed reference to fit the candidate | Dependent LP1/LP2/LP3/LP6 waits; tell the owner; nothing cut |

- **PHS.54 [D]** The existing oracle case names are O01 every generator/inverse, O02 chronological chain, O03 stars,
  O04 mixed recipes, O05 their inverses, O06 protection/collateral and O07 invalid-input preservation
  (`tools/oracle/make_cases.py`, source evidence). The J1 harness compares status, moved count/set, exact digest,
  lattice and requested surveys, with separate straddler-certificate checks. Its J1-against-J1 fixture runs exercise
  the reference/harness; they establish no new engine's equality (the differential README). LP2 needs the candidate.
- **PHS.55 [D]** Open contract choices stay with their owners before the consuming product can be accepted:
  KN5.8/EN4 availability; LG2.5 approximate control flow; LG2.18/LB1.10 word keys; LG4.8 and DET.12/DET.14/DET.20/
  DET.22/DET.24/DET.26/DET.28 deterministic generators, text and accounting; PS1.3/PS1.5/PS1.9/PS1.11/PS1.13/
  PS1.20/PS1.34 and PS3.12/PS3.16 bytes, limits, framing, provenance and import dispatch; LB3.9 exact observation
  packaging; LB5.8 archive dependencies; LB6.6 native-tool decomposition if chosen; EV1/EV2/EV3/EV4 open linkage,
  replay and recording conventions; RPI.18/RPI.34/RPI.36/RPI.50/RPI.55 geometry, chart, stage value, profile and
  backend contracts. This list routes dependencies already open in those chapters, not new mechanisms or resolved choices.

## 13. What the owner sees

- **PHS.56 [D]** The following phase-specific explanation and decision record makes PHS.5 checkable. It follows
  DT-L1's kickoff and partition LP0 to LP6. The advisors show the unresolved option, what every option preserves,
  what it changes and what evidence is missing; the owner confirms understanding and takes the indicated decisions.

| Phase | Concepts and evidence explained | Owner action at or before exit |
| --- | --- | --- |
| LP0 | One state, exactness, identity levels, evidence versus permission, boundary limits, saved meaning, full phase order | Section 9's answers, given on 6 November 2026; review the proposed ADRs, which stay proposed; confirm understanding |
| LP1 | Exact arithmetic and host trade-offs, route/placement faults, flow forms and false refusals, data-core losslessness, both freeze packets | LQ8, LQ9 and LQ21 were answered on 14 November 2026; decide LQ13 by its prerequisite; after both Astra rulings accept the ADRs and sign off, naming the charter amendment |
| LP2 | Oracle equality, typed partial results, literal-prefix checks, deterministic cut-offs, a conformance test versus a proof | Confirm understanding of the core evidence and remaining limits; no new scope decision is inferred from a pass |
| LP3 | The three complete loops, presented-preview binding, receipts/undo, source/key equivalence and full E2 interval | Review the measured result and remaining trade-offs; a missed bar waits for correction or an explicit owner decision |
| LP4 | How source, principles and evidence connect; replay limits; learning and transfer evidence; adopted editor bars | Set the non-author protocol and any learning-gate decision, judge its results, confirm understanding; design stays closed until the language's development is complete |
| LP5 | Archive claim scopes, missing producers, conditional bounds, bit-for-bit reproduction and global coverage still open | Confirm the research handover's meaning and limits; resolve LQ2 jointly before LP6 if still pending; no new search authorised |
| LP6 | Complete importer/product evidence, release cut, boundary policy and remaining out-of-scope work | Accept the design-system ADR and other owner gates when due; sign off the product only on the matching evidence |

- **PHS.57 [D]** Understanding confirmation is a recorded owner response, not an inference from silence, elapsed
  time or agreement between advisors. It remains separate from a comprehension-test result for other people, an
  accepted ADR and a mathematical check (INV-12; DT-L1 owner input; EV5.1). A phase with a pending owner response
  cannot finalise the decision that needs it.

## 14. The 1.0 cut

- **PHS.58 [B]** 1.0 ships KN, LG, LB, EV and PS, including the research library and research-archive checkers (owner,
  6 November 2026, LQ16). The theory book, proofs and stage research records stay out of the shipped software and live
  in the repository (decisions 5 and 9). LP0/LP1 specify all five; LP2 supplies the core; LP3 the three-tool loop; LP4
  the environment; LP5 research expressiveness; LP6 import, packaging and product gates. Modules are shared locally
  through user-chosen files (LQ12), and local content-addressed modules remain in 1.0. The export of proof obligations
  is in 1.0; Lean does not ship with the software, and formal proofs themselves come after 1.0 (LQ15, decision 9).
  Whether the shipped product exposes the research profile is still an LQ2 joint-setting question, distinct from
  implementing the in-scope research library.
- **PHS.59 [B] Withdrawn.** Rejected alternative of LQ16 (owner, 6 November 2026).
- **PHS.60 [D]** A later change to this cut needs an explicit owner decision that names the changed products, their
  checks and every phase they touch. It adds no network or process capability, changes no model identity and relaxes
  no gate by implication. The shared kernel, evidence identities and acceptance meaning stay.

## 15. The conformance suite by first phase

- **PHS.61 [D]** A test's definition belongs with the component or pillar whose contract it checks; the index below
  supplies its phase (overview section 3.4). A cross-layer test names the clauses it checks and one owning definition,
  rather than creating duplicate IDs. Chapter 13 owns error spelling. This chapter owns the suite's coverage checks.
  No conformance test named here exists yet. Existing oracle/harness code and the evidence-table generator are
  reference tools, not implemented CT tests.
- **PHS.62 [D]** The index records the first phase whose listed products can run the whole test, not the first
  possible stub or a promise to run it later. Direct spike checks appear in LP1. Core codecs and fixture models run in
  LP2; three-tool/view integration in LP3; editor/debugger/recording and principle learning in LP4; full research
  algorithms, archive-checker coverage and the obligation export in LP5; product migration, the shipped inventory and
  final designed human distinction in LP6. CT-EV1-08
  and CT-EV2-11 first belong to LP4: their full activation/suspended-run and debugger/suspended-call checks need
  PHS.26's authoring services. LP2 stubs cannot discharge either requirement. Earlier partial
  checker mutation checks do not discharge the all-kinds gate indexed in LP5. Results are rechecked on later changed
  candidates under PHS.4; first assignment is not a lifetime exemption.
  The tables follow the owner's answers of 6 November 2026. An approved order or scope change requires the same
  readiness check and an updated index; nothing is reassigned through an assumed answer.
- **PHS.63 [D]** A failed required test blocks its owning acceptance and every dependent exit that needs it
  (partition section 14). A failure of a lead blocks reporting that lead as obtained; it is not itself a refutation
  of the proposed theorem. A test that depends on a waiting question waits for its decision. If that decision removes
  the tested feature, the decision record identifies the alternative check or explicit non-applicability; neither a
  skip nor non-applicability counts as a pass. No test result answers a waiting question.
- **PHS.64 [D]** Owner-machine tests use fresh isolated fixtures, one frozen candidate/build identity and a quiet
  machine. Input/interaction tests run in the foreground window they exercise. Timings retain their specific
  protocol: charter E2 uses the B4-12 whole interval; LX-4 uses Round 3's adopted edit-to-presented-frame protocol;
  LX-7 and renderer gates use the renderer plan, native resolution, PresentMon and no frame generation. Record
  correctness controls with the run. A run that violates its protocol is invalid and retained as such, never pooled
  into a pass. Long-session evidence covers its specified full interval; a short run proves none of it.
- **PHS.65 [O]** The sources do not select the suite's runner, physical test directories, evidence-record file
  format or an automated commit guard. The integrator must choose them in the implementation plan: reuse existing
  harnesses with reviewed exit records, or add a checked index/evidence validator where needed. Either must enforce
  PHS.4/PHS.63. No new script, schema, hook or directory is established by this chapter.
- **PHS.66 [D]** An integrator must not commit a claim that a phase passed without evidence for every acceptance
  item on that candidate. Review compares the claimed exit with PHS.4's record, required tests and owner/gate
  decisions; missing evidence leaves the exit open. This follows from partition section 14's acceptance and
  `AGENTS.md`'s matching-source evidence rule. A structural check of this document cannot authorise that claim.
- **PHS.67 [D]** The following tables preserve each test's class exactly as its owning conformance table states
  it. Each ID occurs in one phase table. The index includes chapters 01 to 11 and 13,
  the jumbling and layer interfaces, the format packet and the renderer feature matrix. PHS.71 identifies the closure
  additions and their sources. Totals count the indexed requirements, not executed tests. A source-class held-key
  or fault fixture supplies no actual input or native isolation result; the separate windows/performance tests still
  apply (PHS.6).

### 15.1. LP0

LP0 has no executable component test in the supplied chapters. Its contract checks are in section 18.

| Component | source | synthetic | windows | performance | human |
| --- | --- | --- | --- | --- | --- |
| None | none | none | none | none | none |

### 15.2. LP1

| Component | source | synthetic | windows | performance | human |
| --- | --- | --- | --- | --- | --- |
| KN7 | none | none | none | CT-KN7-06 | none |
| LG1 | CT-LG1-06 | none | none | none | none |
| LG3 | CT-LG3-08 | none | none | none | none |
| EV1 | none | none | none | CT-EV1-06 | none |
| EV3 | CT-EV3-09 | none | none | none | none |
| PS1 | CT-PS1-20 | none | none | none | none |
| PS2 | CT-PS2-07 | none | none | none | none |
| PS3 | CT-PS3-07 | none | none | none | none |
| DET | CT-DET-19, CT-DET-25 | none | none | none | none |
| RPI | CT-RPI-15, CT-RPI-28, CT-RPI-29, CT-RPI-35, CT-RPI-38 | CT-RPI-16, CT-RPI-30 | CT-RPI-17 | CT-RPI-25 | none |
| EVD | CT-EVD-14, CT-EVD-15, CT-EVD-16, CT-EVD-23, CT-EVD-25 | CT-EVD-24 | none | none | none |
| BND | CT-BND-25 | none | none | none | none |
| RTE | CT-RTE-01, CT-RTE-02, CT-RTE-03, CT-RTE-04, CT-RTE-05, CT-RTE-06, CT-RTE-07, CT-RTE-08, CT-RTE-13, CT-RTE-14, CT-RTE-15, CT-RTE-21 | CT-RTE-09, CT-RTE-19, CT-RTE-22 | CT-RTE-10, CT-RTE-17 | CT-RTE-11, CT-RTE-12, CT-RTE-16 | CT-RTE-18 |
| JI | CT-JI-01, CT-JI-02, CT-JI-03, CT-JI-04, CT-JI-05, CT-JI-06, CT-JI-07, CT-JI-08, CT-JI-09, CT-JI-10, CT-JI-11 | none | none | none | none |
| LI | CT-LI-08 | none | none | none | none |
| FP | CT-FP-01, CT-FP-02, CT-FP-03, CT-FP-05, CT-FP-07, CT-FP-08, CT-FP-09 | CT-FP-04, CT-FP-06 | none | none | none |
| LB5 | CT-LB5-10 | none | none | none | none |
| LB6 | CT-LB6-05 | none | none | none | none |
| RM | CT-RM-01, CT-RM-08 | none | none | none | none |

### 15.3. LP2

| Component | source | synthetic | windows | performance | human |
| --- | --- | --- | --- | --- | --- |
| KN1 | CT-KN1-01, CT-KN1-02, CT-KN1-03, CT-KN1-04, CT-KN1-05, CT-KN1-06 | none | none | none | none |
| KN2 | CT-KN2-01, CT-KN2-02, CT-KN2-03, CT-KN2-04, CT-KN2-05, CT-KN2-06, CT-KN2-07 | none | none | none | none |
| KN3 | CT-KN3-01, CT-KN3-03, CT-KN3-04, CT-KN3-05, CT-KN3-06 | CT-KN3-02 | none | none | none |
| KN4 | CT-KN4-01, CT-KN4-02, CT-KN4-03, CT-KN4-04 | none | none | none | none |
| KN5 | CT-KN5-01, CT-KN5-02, CT-KN5-03, CT-KN5-04, CT-KN5-05, CT-KN5-06, CT-KN5-07, CT-KN5-08, CT-KN5-09, CT-KN5-10, CT-KN5-11, CT-KN5-12, CT-KN5-13 | none | none | none | none |
| KN6 | CT-KN6-02 | none | none | none | none |
| KN7 | CT-KN7-01, CT-KN7-02, CT-KN7-03, CT-KN7-04, CT-KN7-05, CT-KN7-07 | none | none | none | none |
| LG1 | CT-LG1-01, CT-LG1-02, CT-LG1-03, CT-LG1-04, CT-LG1-05 | none | none | none | none |
| LG2 | CT-LG2-01, CT-LG2-02, CT-LG2-03, CT-LG2-04, CT-LG2-05, CT-LG2-06, CT-LG2-07, CT-LG2-08, CT-LG2-09, CT-LG2-10, CT-LG2-11, CT-LG2-12, CT-LG2-13, CT-LG2-14, CT-LG2-15, CT-LG2-16 | none | none | none | none |
| LG3 | CT-LG3-01, CT-LG3-02, CT-LG3-03, CT-LG3-04, CT-LG3-05, CT-LG3-06, CT-LG3-07, CT-LG3-09, CT-LG3-10 | none | none | none | none |
| LG4 | CT-LG4-01, CT-LG4-02, CT-LG4-03, CT-LG4-04, CT-LG4-05, CT-LG4-06, CT-LG4-08 | none | none | none | none |
| LG5 | CT-LG5-01, CT-LG5-02, CT-LG5-03, CT-LG5-04, CT-LG5-05, CT-LG5-06, CT-LG5-07, CT-LG5-08 | none | none | none | none |
| LB6 | CT-LB6-02 | none | none | none | none |
| LB7 | CT-LB7-01, CT-LB7-02 | none | none | none | none |
| EV4 | CT-EV4-03 | none | none | none | none |
| PS1 | CT-PS1-01, CT-PS1-02, CT-PS1-03, CT-PS1-04, CT-PS1-05, CT-PS1-06, CT-PS1-07, CT-PS1-08, CT-PS1-09, CT-PS1-10, CT-PS1-11, CT-PS1-12, CT-PS1-13, CT-PS1-14, CT-PS1-16, CT-PS1-18, CT-PS1-19, CT-PS1-21, CT-PS1-22, CT-PS1-23, CT-PS1-27 | none | none | none | none |
| PS2 | CT-PS2-01, CT-PS2-02, CT-PS2-03, CT-PS2-04, CT-PS2-05, CT-PS2-06 | none | none | none | none |
| ONE | CT-ONE-01, CT-ONE-02, CT-ONE-03, CT-ONE-04, CT-ONE-05, CT-ONE-06, CT-ONE-07, CT-ONE-08, CT-ONE-09, CT-ONE-10, CT-ONE-11, CT-ONE-12, CT-ONE-13, CT-ONE-14, CT-ONE-15, CT-ONE-16, CT-ONE-17, CT-ONE-18, CT-ONE-19, CT-ONE-20, CT-ONE-21, CT-ONE-22, CT-ONE-23, CT-ONE-24, CT-ONE-28 | none | none | none | none |
| DET | CT-DET-01, CT-DET-02, CT-DET-03, CT-DET-04, CT-DET-05, CT-DET-06, CT-DET-07, CT-DET-08, CT-DET-09, CT-DET-10, CT-DET-11, CT-DET-12, CT-DET-13, CT-DET-14, CT-DET-15, CT-DET-17, CT-DET-18, CT-DET-20, CT-DET-21, CT-DET-22, CT-DET-23, CT-DET-26, CT-DET-27, CT-DET-29, CT-DET-30, CT-DET-31, CT-DET-32 | CT-DET-16, CT-DET-28 | none | none | none |
| RPI | CT-RPI-01, CT-RPI-20 | none | none | none | none |
| EVD | CT-EVD-01, CT-EVD-02, CT-EVD-03, CT-EVD-04, CT-EVD-05, CT-EVD-06, CT-EVD-07, CT-EVD-08, CT-EVD-09, CT-EVD-11, CT-EVD-13, CT-EVD-17, CT-EVD-18, CT-EVD-19, CT-EVD-20, CT-EVD-21, CT-EVD-22, CT-EVD-26, CT-EVD-27, CT-EVD-28, CT-EVD-29, CT-EVD-30 | CT-EVD-10 | none | none | none |
| BND | CT-BND-01, CT-BND-02, CT-BND-04, CT-BND-05, CT-BND-06, CT-BND-07, CT-BND-08, CT-BND-09, CT-BND-10, CT-BND-11, CT-BND-12, CT-BND-13, CT-BND-14, CT-BND-22, CT-BND-23, CT-BND-24, CT-BND-26, CT-BND-27, CT-BND-28, CT-BND-29, CT-BND-30 | none | none | none | none |
| ERR | CT-ERR-01, CT-ERR-02, CT-ERR-03, CT-ERR-04, CT-ERR-05, CT-ERR-06, CT-ERR-07, CT-ERR-08, CT-ERR-09, CT-ERR-10, CT-ERR-12, CT-ERR-13, CT-ERR-14, CT-ERR-15, CT-ERR-16, CT-ERR-17 | none | none | none | none |
| LI | CT-LI-01, CT-LI-02, CT-LI-03, CT-LI-04, CT-LI-05, CT-LI-06, CT-LI-07, CT-LI-09, CT-LI-10, CT-LI-11, CT-LI-12, CT-LI-13, CT-LI-14 | none | none | none | none |

### 15.4. LP3

| Component | source | synthetic | windows | performance | human |
| --- | --- | --- | --- | --- | --- |
| LG4 | none | none | none | CT-LG4-09 | none |
| LB1 | CT-LB1-01, CT-LB1-02, CT-LB1-03, CT-LB1-06 | none | none | none | none |
| LB2 | CT-LB2-01, CT-LB2-02, CT-LB2-03, CT-LB2-04 | none | none | none | none |
| LB3 | CT-LB3-01, CT-LB3-02, CT-LB3-03, CT-LB3-04, CT-LB3-05, CT-LB3-06, CT-LB3-07 | none | none | none | none |
| LB4 | CT-LB4-01, CT-LB4-03, CT-LB4-04 | CT-LB4-02 | none | none | none |
| LB6 | CT-LB6-03 | none | none | none | none |
| EV3 | CT-EV3-03, CT-EV3-06, CT-EV3-07, CT-EV3-08 | CT-EV3-01, CT-EV3-02, CT-EV3-04 | none | none | none |
| EV4 | none | CT-EV4-02, CT-EV4-04 | none | none | none |
| ONE | CT-ONE-29, CT-ONE-30 | none | CT-ONE-26 | CT-ONE-25 | none |
| RPI | CT-RPI-02, CT-RPI-04, CT-RPI-05, CT-RPI-06, CT-RPI-08, CT-RPI-09, CT-RPI-12, CT-RPI-19, CT-RPI-22, CT-RPI-23, CT-RPI-27, CT-RPI-32, CT-RPI-36 | CT-RPI-03, CT-RPI-07, CT-RPI-10, CT-RPI-11, CT-RPI-14, CT-RPI-18, CT-RPI-37 | CT-RPI-13, CT-RPI-21, CT-RPI-24, CT-RPI-31, CT-RPI-33, CT-RPI-34 | CT-RPI-26 | none |
| BND | CT-BND-15, CT-BND-17, CT-BND-18, CT-BND-19 | none | CT-BND-20 | none | none |
| RM | CT-RM-02, CT-RM-03, CT-RM-04, CT-RM-07, CT-RM-09 | none | CT-RM-05, CT-RM-06, CT-RM-10 | none | none |

### 15.5. LP4

| Component | source | synthetic | windows | performance | human |
| --- | --- | --- | --- | --- | --- |
| LG4 | CT-LG4-07 | none | none | none | none |
| LB6 | CT-LB6-01 | none | none | none | CT-LB6-04 |
| EV1 | CT-EV1-01, CT-EV1-02, CT-EV1-03, CT-EV1-04, CT-EV1-07, CT-EV1-08 | CT-EV1-05 | none | none | none |
| EV2 | CT-EV2-01, CT-EV2-02, CT-EV2-03, CT-EV2-04, CT-EV2-06, CT-EV2-07, CT-EV2-08, CT-EV2-09, CT-EV2-10, CT-EV2-11 | CT-EV2-05 | none | none | none |
| EV3 | CT-EV3-05 | none | none | none | none |
| EV4 | CT-EV4-01, CT-EV4-07, CT-EV4-08, CT-EV4-09, CT-EV4-10, CT-EV4-11, CT-EV4-12, CT-EV4-13, CT-EV4-15, CT-EV4-16, CT-EV4-17, CT-EV4-18, CT-EV4-19, CT-EV4-20 | CT-EV4-06, CT-EV4-14 | CT-EV4-05 | none | none |
| EV5 | CT-EV5-01, CT-EV5-02, CT-EV5-04, CT-EV5-06 | none | none | none | CT-EV5-03, CT-EV5-05 |
| PS1 | CT-PS1-15, CT-PS1-17, CT-PS1-24 | CT-PS1-25 | none | none | none |
| BND | CT-BND-03 | none | none | none | none |
| ERR | CT-ERR-18 | none | none | none | none |

### 15.6. LP5

| Component | source | synthetic | windows | performance | human |
| --- | --- | --- | --- | --- | --- |
| KN6 | CT-KN6-01, CT-KN6-03 | none | none | none | none |
| LB1 | CT-LB1-04, CT-LB1-05 | none | none | none | none |
| LB5 | CT-LB5-01, CT-LB5-02, CT-LB5-03, CT-LB5-04, CT-LB5-05, CT-LB5-06, CT-LB5-07, CT-LB5-08, CT-LB5-09, CT-LB5-11 | none | none | none | none |
| PS1 | CT-PS1-26 | none | none | none | none |
| PS3 | CT-PS3-08, CT-PS3-09, CT-PS3-10 | none | none | none | none |
| DET | CT-DET-24 | none | none | none | none |
| EVD | CT-EVD-12 | none | none | none | none |
| ERR | CT-ERR-11, CT-ERR-19 | none | none | none | none |
| RTE | CT-RTE-23 | none | none | none | none |

### 15.7. LP6

| Component | source | synthetic | windows | performance | human |
| --- | --- | --- | --- | --- | --- |
| PS3 | CT-PS3-01, CT-PS3-02, CT-PS3-03, CT-PS3-04, CT-PS3-05, CT-PS3-06, CT-PS3-11, CT-PS3-12 | none | none | none | none |
| ONE | none | none | none | none | CT-ONE-27 |
| BND | CT-BND-16, CT-BND-21 | none | none | none | none |
| EV5 | CT-EV5-07 | none | none | none | none |
| LB5 | CT-LB5-12 | none | none | none | none |

Source counts from the index above, for the integrated candidate (PHS.71):

| Phase | source | synthetic | windows | performance | human | Total |
| --- | --- | --- | --- | --- | --- | --- |
| LP0 | 0 | 0 | 0 | 0 | 0 | 0 |
| LP1 | 54 | 8 | 3 | 6 | 1 | 72 |
| LP2 | 244 | 4 | 0 | 0 | 0 | 248 |
| LP3 | 47 | 13 | 11 | 3 | 0 | 74 |
| LP4 | 42 | 5 | 1 | 0 | 3 | 51 |
| LP5 | 23 | 0 | 0 | 0 | 0 | 23 |
| LP6 | 12 | 0 | 0 | 0 | 1 | 13 |
| Total | 422 | 30 | 15 | 9 | 5 | 481 |

The imported index contains 481 requirements: 422 source, 30 synthetic, 15 windows, 9 performance and 5 human.
Section 18 adds ten source test requirements for this chapter. Including them gives 491 requirements, with 432 source;
the other class counts are unchanged. Those ten are excluded from the imported phase totals. No number is a pass count.

- **PHS.71 [D]** The closure adds 55 indexed requirements from the [closure manifests](../closure/manifests/):
  `p1.json`, `p2b.json`, `p3.json`, `p4.json`, `p5.json`, `p6.json`, `p7-data.json`, `p7-layers.json` and
  `p7-render.json`. Their owning definitions remain in the jumbling/layer interfaces, format packet, chapters 03, 04,
  06 and 09, and renderer feature matrix. The manifests supply phases and classes except that
  CT-EV1-08 and CT-EV2-11 use LP4 under PHS.26/PHS.62 and reconciliation finding RS3-12. The additions are 27 in LP1
  (24 source, 3 synthetic), 20 in LP2 (19 source, 1 synthetic), 6 in LP3 (4 source, 2 windows) and 2 source in LP4.
  The integration of the owner's answers of 6 November 2026 adds 64 more and withdraws CT-RTE-20: 10 in LP1 (8 source,
  2 synthetic), 24 source in LP2, 11 in LP3 (6 source, 1 synthetic, 4 windows), 13 in LP4 (11 source, 2 synthetic),
  4 source in LP5 and 2 source in LP6. The integrator compared this index with every defining table of the combined
  candidate on 6 November 2026: each defined ID occurs once with its class unchanged. A later addition needs a recount.
  The totals count requirements only: none of these tests has run.

## 16. Source differences and evidence limits

- **PHS.68 [D]** The older day tables in the charter, re-plan, briefing, experiment protocol and renderer plan are
  historical schedule text under the 3 November owner decision. Their acceptance requirements remain. The protocol
  and renderer plan allow continuing from a last delivered handoff for experiments; that does not satisfy a missing
  language-phase dependency. PHS.1/PHS.53 require the dependent exit to wait and the owner to be told.
- **PHS.69 [D]** The owner settled the conflict between the signed charter's whole stage 2.5 exit and the design hold
  on 6 November 2026 (LQ19, PHS.44); partition section 14 keeps its record.
  [H-08 CM.1](../../../../progress/1.0/command-table.md) now supplies an integrated final table candidate: 174 commands
  with `final: true`, using H15's reconciled forms for undo, gates, lanes and renderer consumers. The flag means the
  real jumbling and language deltas were supplied, not that the contract is accepted or frozen. Review of the current candidate,
  adapter/runtime implementation and matching evidence, and the required owner decisions and freeze rulings remain
  open.
- **PHS.70 [D]** Source figures and method corrections use DT-L1 Round 3: the evidence corpus is 497 records and
  13,412,769 stored bytes from Git blobs, as recorded in the evidence-import README, not the earlier checkout total;
  long-word regressions use KN2.10's recorded lengths, not 672,417 as a maximum. These are source evidence, not
  replay or speed results. Round 3 also separates GPU correctness from headless preparation. The 2 October migration
  record's no-file-copy wording is superseded for the locked database/WAL copy by the owner's amendment recorded in
  the migration proposal and chapter 05, section 13; the proposal's old source-connection pipeline wording is not the
  amended method. This chapter claims no new probe result. Renderer peak memory remains a recorded budget/risk under
  renderer-candidates; RPI.60 and Round 3 express it as part of their proposed complete gate run. No new memory
  threshold or authority to weaken that budget is selected here.

- **PHS.72 [D]** ADR-L01, ADR-L02, ADR-L07, ADR-L08 and ADR-L11 describe the current one-state, client/receipt,
  execution/accounting and display-interface drafts by reference to their owning clauses. All five remain Proposed
  and not accepted; ADR-L07 remains held for LP1 and the route choice. Their open lists retain review, implementation,
  measurements and the waiting questions. The owner's answers of 6 November 2026 accepted no ADR, and a supplied draft
  protocol is no accepted ADR or freeze.

## 17. Errors

This chapter adds no error code; chapter 13 owns the registry.

## 18. This chapter's conformance tests

These are requirements for later suite tooling and exit review. None exists or has run. Their earliest structural
checks belong with LP0's contract inventory; evidence-claim checks also apply whenever an exit is asserted. They are
separate from the imported-component totals above.

| Test | Checks | Class |
| --- | --- | --- |
| CT-PHS-01 | Compare defining chapter tables with the phase index: every imported CT ID has exactly one first-phase assignment, no unknown ID, unchanged class and exact per-phase/per-class totals | source |
| CT-PHS-02 | Every phase has products, acceptance and dependencies; every acceptance item names a check and evidence class, and every exit lists what remains open | source |
| CT-PHS-03 | PHS.41's graph is acyclic; LP1 proceeds with every ADR still proposed and no ADR is accepted before both rulings; LP2 cannot start before both rulings and the owner's sign-off, and the design line cannot resume before LP2 to LP5 are done apart from LP4's editor look | source |
| CT-PHS-04 | Each LQ1 to LQ21 has an exit-state row; procedure acceptance cannot be mistaken for the final choice; LQ2 is settled jointly before LP6 and LQ13 before the parser | source |
| CT-PHS-05 | A proposed phase-pass commit with a missing check, wrong candidate identity, invalid run or missing owner/gate decision is refused as an unsupported exit claim | source |
| CT-PHS-06 | Substitute headless, sampled, source-only or model-agreement material for required native, input, long-session, performance or human evidence; the corresponding acceptance stays open | source |
| CT-PHS-07 | LP1 freeze inputs include H-08 with the new command rows, layer interfaces, renderer features with the stage contract and free-view workload, the recording rules and the complete path B data core under LQ10's answer; an absent dependency cannot pass by a previous draft's name | source |
| CT-PHS-08 | A missing input in each external-line row blocks the dependent exit and produces an owner report, without an automatic cut, research restart or design-line resumption | source |
| CT-PHS-09 | Skips that depend on a waiting question retain their disposition and alternative obligations; a skipped test is never included in passed totals | source |
| CT-PHS-10 | Owner-machine evidence binds protocol, fixtures and frozen build; failed correctness controls or missing foreground/input conditions invalidate the run rather than count as a pass | source |

## 19. Open items

The owner answered LQ1 to LQ21 on 6 November 2026 and chose LQ8, LQ9 and LQ21 on 14 November 2026: the engine and the
interpreter's host in Rust, route RT1 with the runtime in a process of its own, and capability separation (RTE.17,
RTE.21, RTE.24). LQ13 waits for its evidence, and LQ2's option set waits for the joint setting.

| Owner question | Clauses and remaining decision |
| --- | --- |
| LQ2 | PHS.7, PHS.38/PHS.39, PHS.56/PHS.58: joint policy, author classes, declassifiers, display sinks, shipped research-profile exposure and any competence gate, before LP6 |
| LQ8 (answered) | PHS.15, PHS.38, PHS.42: the engine is written in Rust (owner, 14 November 2026, LQ8; RTE.17); LX-5 and LX-6 ran and were accepted as decisive (RTE.82) |
| LQ9 (answered) | PHS.15, PHS.38, PHS.42: route RT1, runtime in a process of its own (owner, 14 November 2026, LQ9; RTE.21); LX-1, LX-2 and LX-4 are LP3 acceptance checks on a renderer candidate presenting frames, and a failure reopens LQ9 |
| LQ13 | PHS.17/PHS.20, PHS.38/PHS.42: syntax and learnability protocol before the parser |
| LQ21 (answered) | PHS.15, PHS.38, PHS.42: capability separation (owner, 14 November 2026, LQ21; RTE.24); LX-3's decisive check is part of LP3 acceptance |

| Open dependency or mechanism | Clauses and what settles it |
| --- | --- |
| Content review | PHS.9/PHS.10: integrator supplies proposed glossary entries in chapter 04 section 6 at LP0 and integrates them into `terms.json` at LP1 before any LP2 parser or library text uses a term; Astra reviews the assembled current candidate |
| Status of note N1 | PHS.12/PHS.19/PHS.32/PHS.48: N1 stands at (theorem, producer) until a model review reads it, and its transfer step is the reviewed stage 19 lemma; no person has reviewed either |
| Import-table dispatch | PHS.9/PHS.31/PHS.49: EVD.52 rules on V14, P13, K1 and H1 with their limits. Records with no format key stay wholly recorded until a discriminator table exists (EVD.53, PS3.12) |
| EN3/EN4 and H-08 | PHS.16/PHS.20/PHS.53/PHS.55/PHS.69: [EN3's 5 November draft](../../../../progress/1.0/jumbling-interfaces.md), [EN4 `en4-layer-interfaces/1-draft`](../../../../progress/1.0/layer-interfaces.md), [H-08 CM.1's integrated final candidate](../../../../progress/1.0/command-table.md) and [format candidate `en4-data-core-draft-1`](../format-packet/README.md) are supplied. Current-candidate review, complete typed coverage, implementation/evidence and owner/gate approvals remain open |
| Specification choices routed to existing chapters | PHS.55: byte and numeric profiles, limits, provenance version, exactness control flow, word keys, metering/resources, archive dependencies, replay/linkage/recording and chart/backend contracts remain with those owners |
| Human protocols and editor bars [O] | PHS.27: owner sets/adopts protocols and criteria; mathematician checks content; chapter 11 supplies adopted LX-4 detail; no participant or timing result exists |
| Runner and evidence enforcement [O] | PHS.65: integrator chooses implementation/reuse and the reviewable enforcement of PHS.66; no hook, schema or runner is created here |
| Renderer selection, handoff and measurements | PHS.24/PHS.48/PHS.53/PHS.64: owner closes/selects/freezes under the existing gate; H-11's support, the stage and view ceilings, LX-7's W3 runs and E2 remain unevidenced for this language candidate |
| Decisive runs | PHS.14/PHS.42: the owner says when the machine is free (decision 15); first-pass records decide nothing |
| State-contract amendment A6 | PHS.53: A6's text passes EN1's Astra check and then the owner; the owner has chosen its free-mode grid (KN3.7) |
| Session and migration product evidence | PHS.35/PHS.36/PHS.49/PHS.53/PHS.70: amended path B contracts and matching product importer/native recovery checks, without transferring historical probe passes |
| Design-system ADR | PHS.28/PHS.36/PHS.44/PHS.47: the design line stays closed until the language's development is complete; the ADR then conditions LP4's editor look and LP6; no DT-J2 to DT-J4 work opened |
| Research availability and scope | PHS.31 to PHS.33: archive inputs and independent checks; missing required dependencies wait; research remains stopped and global coverage remains open |
| Glossary integration and production evidence | PHS.9/PHS.26/PHS.53: chapter 04 section 6's proposed entries integrated into `terms.json` at LP1 before any LP2 parser or library text uses a term, with reviewed additions, one term per concept and puzzle/alphabet cost units, existing tooling and EV5.15 diagram evidence |
| Test implementation | PHS.61 to PHS.67: future implementation supplies every CT test and matching evidence |

# Chapter 03: libraries and built-in tools

Status: **draft for the owner. Not accepted.** Conventions are in [the overview](00-overview.md), section 3.

This chapter gives the library entries, their contracts and their backing. It defines `PrincipleRecord` and lists
candidate built-in tools. It grants no solving capability. Chapter 07 owns the policy table, chapter 02 the types and
effects, and chapter 01 the shared kernel vocabulary. Names below fix concepts, not concrete syntax.

Sources: [the partition](../ecosystem-partition.md), sections 5, 10.2, 10.3 and 11;
[DT-L1](../../../../progress/1.0/design-table/topics/language-ecosystem.md),
M3 to M6, E6, R2-E1 to R2-E4 and its Outcome; [DT-J1](../../../../progress/1.0/design-table/topics/jumble-solver-aids.md),
rule 6 and its Outcome; [the command table](../../../../progress/1.0/command-table.md);
[the solving workflow](../../../../progress/1.0/solving-workflow.md); [the state contract](../../../../../research/jumbling/state-contract.md);
[ENG-0J revision 2](../../../../progress/1.0/packets/engineering/ENG-0J-jumbling-backend.md);
[research data](../../../../progress/1.0/jumbling-handoff/research-data.md);
[restoration theory](../../../../../research/jumbling/theory/restoration.md);
[the restoration archive](../../../../../research/jumbling/restoration/README.md);
[retained puzzle theory](../../../../../research/PUZZLE_THEORY.md);
[core.py](../../../../../core.py) and [session.py](../../../../../session.py).
Section 7 links the individual stage archives.

## 1. Rules for every library

- **LB1.1 [D]** Libraries are language code over kernel calls (partition section 5, default 1). A decision-path
  function promoted for speed becomes a kernel batch with an oracle agreement test. Its language function remains the
  documented, viewable entry. Promotion changes the backing and declared cost, while preserving its domain, result,
  level and evidence. The placement of per-revision decisions in KN2.4 follows DT-L1 R2-E2.
- **LB1.2 [D]** Each entry has the five fields used in the catalogues below: Entry, Level, Effect, Backing and Notes.
  Level identifies the puzzle and identity-tower level read, or says that the entry operates on values without a state.
  Effect uses chapter 02's effects. Backing names a KN2.4 row or library code, with its lower-level calls. Notes state
  preconditions, coverage and the meaning of an empty or negative answer. This makes partition LB1 to LB5 and INV-5
  testable without treating a library name as evidence.
- **LB1.3 [D]** State inputs are chapter 01's `StateRef` values, and state results retain their identity, level and
  input reference, following INV-1, INV-6 and INV-7. A library never creates a mutable puzzle copy, lifts labels to
  poses, or supplies a committed state. Projections use chapter 01's `Partial<T>`. In particular, off-lattice labels are
  undefined; unknown evidence is a different result.
- **LB1.4 [D]** An empty mathematical collection means exactly that collection has no members. An empty job answer
  means this only after completion and only for the named input and declared coverage. Cancellation, error, a budget
  stop and an operation not run remain distinct (default 6, KN7). An intermediate witness may be checked independently;
  it does not make the rest of the search exhaustive. Chapter 06 interprets the resulting evidence.
- **LB1.5 [D]** The catalogues enumerate entries and make no boundary ruling (partition section 11). Under its interim
  reading, no shipped function searches, minimises, ranks or selects over twists, words or caps in a solving session.
  Every such entry below carries the exact marker **research tier candidate**. The two exceptions are setup search for
  a target the solver names and the repair list at a named cap. Enumeration here neither admits a function to a base
  tier nor promises that the research tier ships; chapter 07 records the joint setting under LQ2 and LQ21. BND.64 states
  the rule the marker serves.
- **LB1.6 [D]** Pure algebra is not a boundary exemption. Generic elimination over F2 and F5, the current ν and the
  correction rows compose into a correction selector (DT-L1 R2-E3, fix-first 17). Calls, branches, stored words and
  preview outcomes use chapter 07's provenance mechanism, with its form left to LQ21. Under flow labels the mechanism
  follows the dependencies; under capability separation a previewing program cannot read state-derived data or its
  preview outcomes, apart from the policy's fixed declassifiers. A library cannot declare a declassifier.
- **LB1.7 [D]** Library contracts use exact values for decisions and chapter 01's verdicts for incomplete evidence
  (INV-4, INV-5). Pointwise contracts consume the interpreter's step budget; whole-state contracts run per revision or
  as agreement tests in CI (DT-L1 R2-E4). Chapter 09 owns budgets and grants. Chapter 05 owns all stored forms, field
  keys, versions, digest bytes and import tables; chapter 06 owns statement identity and label interpretation.

## 2. LB1: base library

- **LB1.8 [D]** The following entries cover partition LB1. Exact scalar operations come from KN3; the algorithms over
  them are library code (DT-L1 M4, R2-E2). An algebraic answer establishes its stated algebraic property, with no
  implied reachability, admissibility or permission.

| Entry | Level | Effect | Backing | Notes |
| --- | --- | --- | --- | --- |
| Collections and relations | Values; IDs retain their model and level | pure | Library code | Set operations, maps, joins and relation composition; an empty join means no matching tuples in the supplied relations. |
| Graphs, trees and AND/OR graphs | Values | pure | Library code | Incidence, paths supplied by the caller, dependencies and release obligations; no solution follows merely from a graph node or edge. |
| Finite fixed points | Values in a declared finite lattice | pure, budgets | Library code | Least and greatest fixed points with the operator and order stated; a converged empty set is a fact about that operator. |
| Exact linear algebra | Z, Q, Q(√5), F2 or F5 values | pure, budgets | Library code over KN3 | Rows, kernels, rank and equation solving in the declared domain. Inconsistency is about the supplied equations. Puzzle-word selection through these equations is a research tier candidate, including correction combination. |
| Quaternion and icosian algebra | Exact pose matrices or algebraic values | pure, budgets | Library code over KN3 | Quaternion factors over Q(√5), the icosian order, its 2-adic valuation and 5-regular tree; heights, branches and cosets γK⁺. Per-revision decisions use LB3's batch entries. |
| Permutation groups | Permutations of an explicit finite set | pure, budgets | Library code | Orbits, stabilisers, cycle types, Schreier tables and primitivity tests. A failed group-membership test concerns this group. Extracting a puzzle word by a Schreier search is a research tier candidate. |
| Literal words | Full signed generators; jumbling menu letters | pure | Library code | Concatenation, inverse, conjugate, commutator and powers construct words. Construction alone checks no state. `errors: LB1:input.incompatible_alphabet`. |
| Word equality keys | Same alphabet and identity as the word | pure | Library code | Free reduction for Full; domain-preserving merging for jumbling. Keys do not replace executed letters (LB1.9). |
| Trace equality keys | Words and a stated independence relation | pure, budgets | Library code | A global canonical key for interchangeable orders; a search for a least word is a research tier candidate. The precise convention is open in LB1.10. |
| Straight-line programs | Words | pure, budgets | Library code; application through KN2.4 `replay` | Exact expanded length and cost without full expansion, lazy expansion and chunk iteration. Empty expansion is a zero-letter word, subject to the word's alphabet contract. |
| Exact matching and integral flows | Explicit bipartite graphs and networks | pure, budgets | Library code over KN3 integers | Matching and integral feasible flows, with the matched objects, demands and capacities. Failure concerns this graph or network. Using a flow to choose a solving plan is a research tier candidate. |
| Big integers and symbolic bounds | Exact scalar values | pure, budgets | Library code over KN3 | Binomials, products, recurrences and digit counts; a bound does not imply that its word was expanded or replayed. |
| Budgeted enumeration | Declared finite or enumerable family | pure, budgets; seeded randomness if used | Library code | Coverage and completeness report. Enumeration over twists, words or caps is a research tier candidate; an unfinished enumeration has no negative conclusion. |

- **LB1.9 [D]** Word operations are total on well-formed words and partial on states (partition LB1, default 10;
  DT-L1 M4, R2-E2). For jumbling, a grip with a straddler refuses both `(c,a)` and the literal word
  `(c,a),(c,a⁻¹)`, while the empty word is admissible. A merged identity letter keeps the grip's domain restriction;
  deleting it would lose that restriction. A key must preserve admissibility and the end digest at every tested state.
  Preview, Strict protection, peak height and commit use the literal word, including intermediate states. The inverse
  of an admitted word is admitted from its end; a constructed conjugate gets no admissibility guarantee.
- **LB1.10 [O]** The global convention for trace equality keys needs the LP1 interface specification. Options include
  a least topological word under a fixed alphabet order or a canonical dependency graph. The independence relation and
  the equality represented must be stated with either option. Stage 16's local commuting rule retains representatives
  of traces and is explicitly not a normal form (its README, section 2.6); it cannot supply this convention. Neither
  option changes literal execution or turns minimality and dynamic-target reductions into word identities.
- **LB1.11 [D]** Expanded length and cost are available before checking, previewing or running a compressed word
  (default 10, KN2.10). Exceeding an admission bound refuses that operation with its exact length. Chunking transports
  one operation and creates no new commit or protection boundary. Admission limits are scoped to the consuming entry;
  the stage 18 limit is not a mathematical bound on possible words.

## 3. LB2: 600-cell-Full library

- **LB2.1 [D]** These entries cover partition LB2 and the orbit-first workflow. They read labels and derived
  positions through KN2.4. Retained workflow tools require Jumble off (state contract A5; DT-J1 O8 (1)). A label
  checkpoint permits these tools on its labels without constructing a retained word to the exit poses.

| Entry | Level | Effect | Backing | Notes |
| --- | --- | --- | --- | --- |
| Orbit catalogue and collateral stage order | Full labels; model data | pure; read a snapshot for progress | Library code over KN2.4 `labels`, `positions`, `effect` | Stage dependencies follow full seed collateral, not orbit ID order. A cycle in a custom dependency graph does not authorise dropping an effect. |
| Seeds and full seed analysis | Full labels | pure for named words; read a snapshot for context | Library code over KN2.4 `effect`; KN6 checks | Legal seed witness, target action, exact frames and all collateral. No certificate means unverified, never pure by omission. |
| Guarded setups and buffers | Full labels and positions | read a snapshot, budgets | Library code over KN2.4 `labels`, `positions`, `effect` | Buffers are positions with changing occupants. Setup search for a target the solver names is the exception in LB1.5; its empty result names the guarded setup family examined. |
| Star and recipe construction | Full words and exact slot-tuple frames | pure, budgets | Library code; KN2.4 `effect`, `replay` on application | Chronological setup⁻¹, relocation⁻¹, seed, relocation, setup. Recipe steps retain their full witnesses. Frame checks use ordered slot tuples. |
| Collateral and support | Full labels | pure for an effect; read a snapshot for its consequences | KN2.4 `effect`; library code for reports | Support per orbit and the full source-to-destination map. Empty net support says identity at the end; it says nothing about intermediate support. |
| Net and Strict protection | Full labels | read a snapshot, budgets | KN2.4 `protection`; library code for lock predicates | Captured protected state, block, identity and position locks. Unknown or unchecked coverage cannot produce a safe verdict. Requested assurance with missing coverage has `errors: LB2:protection.incomplete_coverage`. |
| Residuals and exact completion | Full labels and positions | read a snapshot | Library code over KN2.4 `labels`, `positions` | Non-buffer positions, buffer occupants, frames, orientations and home labels. Empty candidate lists or matching colours never establish completion. |
| Blocks and work sheets | Full labels and positions; stored words | read a snapshot; change the workspace only in a command | Library code over KN2.4 reads and `effect`; KN4 commands for storage | Home and referenced relations, named roles, Prepare/Macro/Cleanup and gains/losses. Reuse resolves roles again and needs a fresh check. |
| Named macro analysis and comparison | Full labels | read a snapshot, budgets | KN2.4 `effect`, `protection`, `replay`; library code | Reports effects of the caller's named macros without choosing one. Ranking or selecting macros, generated endgame corrections and automatic next-target selection are research tier candidate entries. |

- **LB2.2 [B]** Analysis preserves every source-to-destination permutation in chronological order and every collateral
  effect (`AGENTS.md`, mechanics; `core.py`'s `net` and `expand`). A filter, selected orbit, work sheet or displayed
  target cycle cannot reduce mechanical support. The star's guarded setup does not certify the seed's other orbits.
- **LB2.3 [D]** Protection reports include their checked coverage, input and level. Net tests the end of the complete
  operation; Strict is a predicate over every executed prefix. An unchecked part is never safe, and a program is never
  split silently (partition 10.2, default 10; solving workflow sections 2 and 4.1). A word can have identity net effect
  and fail Strict. The step flag of a jumbling word is a separate predicate, used by LB3.
- **LB2.4 [D]** The retained recipe compatibility entry keeps the limits listed by partition 10.2: 1 to 256 recipe
  steps, a total of at most 3,000,000 retained primitives, and word chunks of at most 100,000 retained primitives
  (source evidence: `core.py`, `Model.normalize`; architecture V03's recipe contract). A chunk cannot raise the total.
  The library reports the length and the refusal; it does not turn `core.py`'s historical suggestion to split a batch
  into an automatic split. Chapter 05 defines the imported representation.
- **LB2.5 [D]** An admissible, protection-preserving operation need not improve the named goal (solving workflow
  section 4.1; INV-5). Inspection, analysis, goal satisfaction and execution permission remain separate. The workflow's
  historical candidate and next-orbit suggestions do not admit automatic target selection under the interim reading
  (partition section 11). That entry stays a research tier candidate under the interim reading (BND.65).

## 4. LB3: jumbling library

- **LB3.1 [D]** The following entries cover partition LB3. The level is the state level actually read, including when
  a derived scalar is returned. Off the lattice an entry reads poses, classes or regions, never labels (KN1.15).
  Heights, branches and domain IDs use exact poses and the icosian tree; their per-revision batch must agree with LB1
  and `trees.py` (KN2.4, DT-L1 R2-E2). A lattice test by heights holds only on Γ(Λ). In free mode, domain IDs go by exact
  coset tests, because T6 uses T3 (DT-L1 R3-M2). Heights can batch per coset.

| Entry | Level | Effect | Backing | Notes |
| --- | --- | --- | --- | --- |
| Menus and pinned letters | Jumbling model and exact rotations | pure | Library code over KN1 and KN3 | A4₀ inclusion, inverse closure, conjugation invariance and transport; I_a is the signed menu. Menu identity, order digest and contract revision accompany every letter sequence. |
| Admissibility and blocked-twist check | Jumbling poses | read a snapshot, budgets | KN2.4 `classify`, `survey` | Admissible, blocked, invalid or uncertain for the named twist; certificates for inside/outside or straddling. A refusal changes nothing. |
| Straddlers and u at a named cap | Jumbling poses | read a snapshot, budgets | KN2.4 `straddlers` | The complete set, each with its exact certificate pair. Empty after completion means no straddler at that cap and input, not a useful lowering. |
| Rim and seam facts; N | Jumbling poses | read a snapshot, budgets | KN2.4 `rim_summary`; library code over exact pose/region data | N_c, In_c, Out_c, A_c and floor at every cap; seam cells including off-grip seams. Total N includes those seams and is not the sum of the displayed N_c. Batch packaging is open in LB3.9. |
| Heights, branches, domains and Φ | Jumbling poses | read a snapshot, budgets | KN2.4 `height_counts`, `heights`; LB1 tree code and exact sums | Sparse heights with n₂, n₄, n₆ and above-6 data; Φ = Σ max(0,h−2). A zero Φ permits height-2 pieces and is not the lattice or sticker test. |
| Units and clean-step contract | Jumbling poses | read a snapshot, budgets | Library code over KN2.4 `pose_table`, `pose_ids`, `word_check` | Named unit and free set; landings, lifted lattice pieces and unchanged other off poses. A false contract is an observed failure of that contract, not by itself a refusal to preview. |
| D and release accounting | Jumbling poses | read a snapshot, budgets | Library code over exact heights, branches and the caller's named word; KN2.4 `word_check` | D_x(h), the high-piece lowering count for a named item, is distinct from an admitted domain D[k]. F_x, D_x(h) and ΔΦ are read when the letter acts. Listing or maximising over lowerings is a research tier candidate. |
| Release and whole-word checks | Jumbling poses | read a snapshot, budgets | KN2.4 `release_check`, `word_check`, `replay` | Literal prefixes, digests, moved sets, ΔN, Δn_h and Δu at the named cap; step flag, tameness, excursion and peak. An unplayed suffix stays unchecked. |
| ν and drafted correction-set check | Jumbling labels, on the lattice only | read a snapshot, budgets | KN2.4 `nu`, `correction_set_check` | Uses the fixed C5 convention of A5. Off the lattice returns undefined, never stale coordinates, zeros or partial ν. The set is drafted by the solver. `errors: KN2:consistency.fixed_nu_nonzero` for a violated fixed coordinate on a reachable I_a result. |
| Correction catalogue | Jumbling labelled action, with literal menu words | pure; read a snapshot for a word check | Library code over the shipped constructions; KN2.4 `word_check`, `replay`; KN6 | The 36 ν-corrections in fixed ID order with their fixed rows (source: DT-J1 O7 (a), A5). Lookup by solver-supplied ID chooses no combination. |
| Correction-combination solve | Jumbling labels | read a snapshot, budgets | LB1 exact linear algebra over ν and the correction rows | research tier candidate; includes the operation exemplified by `solve(b2,b5)` (DT-L1 R2-E3). It cannot be inferred safe from its algebraic implementation. |
| Retained checkpoint, label checkpoint, sticker test | Jumbling poses and explicit label projection; Full labels as applicable | read a snapshot, budgets | KN2.4 `retained_checkpoint`, `label_checkpoint`, `sticker_test` | Three independently named tests, with different premises and conclusions (LB3.2). |
| Mode crossing prerequisites | Jumbling poses; Full labels with journal derivation | read a snapshot, budgets | Library code over the end tests and KN4 queries | Reports A5's crossing prerequisites. Actual Jumble on/off commands, stored exit poses and revisions belong to chapters 01 and 08. |
| Protection | Jumbling poses; Full labels after crossing | read a snapshot, budgets | KN2.4 `protection` | Verdict carries its level. A label verdict grants no pose protection. |
| Covers at a named cap | Jumbling poses | read a snapshot, budgets | KN2.4 `covers` | DT-J1's exact face-piece definition, both directions and the meeting-cap domain; no top-cap verdict or method-selected target. |
| Repair list at a named cap | Jumbling poses | read a snapshot, budgets | KN2.4 `repair_list` | Exception in LB1.5; completed list of (x,d,y,z,ΔN) under the pinned order and tie rule. Empty means no repair of this stated shape at this cap. |
| Scramble construction and research descent | Jumbling poses and declared menu words | read a snapshot, budgets; seeded randomness where declared | Library code over KN2.4 `classify`, `replay` and observations | research tier candidate for selection over twists. ENGINE scramble admission and its separate record stay with ENG-0J; product depth and mix are open there. This entry grants no solving capability. |

- **LB3.2 [B]** End tests retain the separate conclusions of state contract A5 and partition default 9. A retained
  checkpoint in the jumbling puzzle has a retained legal witness to its exact pose state. A label checkpoint has the
  reachable-lattice premises and ν = 0 under the pinned C5 convention, certified by G = 𝒦, without generating a witness
  word. The sticker test requires N = 0 and
  every labelled slot at home. A retained checkpoint implies a label checkpoint; the sticker test implies a label
  checkpoint for a reachable input. Lattice placement or low height alone establishes neither. A pose table without
  the required reachability record is not a label checkpoint.
- **LB3.3 [B]** The sticker test ends a jumbled solve, whatever its invisible stabiliser rotations (state contract A5;
  DT-J1 O8 (1)). Pose equality stays the kernel's equality for digests and replay and is not another completion fact.
  Jumble off needs a label checkpoint; Jumble on uses journal replay from solved or the stored exit poses, never a lift
  of labels. The retained tools run after the crossing. Chapter 08 owns these lifecycles.
- **LB3.4 [B]** Protection is pose-based inside Jumble and label-based in Full (state contract A3, A5). A protection
  verdict carries its level and never crosses it. In particular, a retained generator can rotate centre piece 0 with
  unchanged labels. Its label-level protection pass after crossing cannot replace the pose-level check inside Jumble.
- **LB3.5 [D]** Shown facts follow DT-J1 rule 6 as stated in partition section 8.2: they are defined at one revision,
  with no quantifier, minimum or argmax over twists or words, and are supplied per cap or for the solver's named
  objects. There is no largest/top/first selection by the method, unlock list, existence answer, prefetch for an unnamed
  cap or automatic correction combination. The per-revision rim entry excludes the research survey's all-items loop
  (DT-J1 fix-first 6). A static domain or floor fact can imply something about twists; those edges wait for the joint
  setting (BND.66).
- **LB3.6 [B]** The repair list uses the exception of 17 October and DT-J1 G3, O10 and O11. For each nonidentity x at
  the solver's cap c, it tests the meeting misaligned caps d in neutral cap order. After x, y minimises N_d; after x,y,
  z minimises N_c; ties use the smaller pinned menu index. The row exists only under ENG-0J's unblocked, nonclosed and
  strict-lowering conditions and when the whole repair lowers N. Rows remain in x-then-d order with none ranked best.
  The solver selects a row and gets a fresh whole-word preview. Identity z gives a shorter repair. No empty list says
  that no other lowering word exists.
- **LB3.7 [D]** Excursion, peak height and tameness are chronological word observations (partition 10.3; stages 14 and
  15). Excursion is the greatest rise of Φ over its start, including the last letter. Tameness requires no letter to
  raise Φ. A word with zero excursion may fail tameness. A composed endpoint alone cannot certify prefix peak,
  admissibility or Strict protection; chapter 01's agreement requirement applies to any fast path.
- **LB3.8 [B]** A failed step flag is a check result, separate from execution permission (ENG-0J acceptance item 3;
  DT-J1 D2). The flag checks every prefix's admissibility and its stated end conditions: the unit lands, no lattice
  piece is lifted, and every other off piece lands or keeps its exact pose. It is separate from Net and Strict
  protection, a general clean step's declared free set, and any optimisation rank. The library never silently edits a
  word to make the flag pass.

  ```text
  operation check_named_word(input: StateRef, literal: LiteralWord, unit?: PieceSet, cap?: CapId) -> JobResult
    effect:  read a snapshot, budgets
    cost:    chapter 01's literal replay cost and the declared observation batches
    errors:  LB1:input.incompatible_alphabet, KN1:input.identity_mismatch, stale.*, budget.*
  ```

  This names LB3's whole-word check. A blocked prefix and a false step flag retain their distinct kernel results.
- **LB3.9 [O]** EN3 and LP1 must fix how total N, exact seam data, D, units, Φ and prefix observations are packaged.
  KN2.4 names the rim, pose, height and word-check rows but does not enumerate all these result fields. Options are
  fields on those rows with library aggregation, or additional batch rows for the remaining decisions. Either keeps
  the entries above, exact pose-level inputs and oracle agreement. Summing N_c to stand in for total N is excluded by
  restoration theory M2/M3. This chapter adds no unnamed kernel interface as an assumed dependency.

## 5. LB4: representations library

- **LB4.1 [D]** These entries cover partition LB4. They consume exact model/state data and produce chapter 10's
  representation values and display plans. Geometry used to decide a property remains exact; drawing values carry
  their declared precision (INV-4). Layout and projection are different evaluations.

| Entry | Level | Effect | Backing | Notes |
| --- | --- | --- | --- | --- |
| Exact geometry and display conversion | Model geometry; poses or labels explicitly | pure; read a snapshot for state geometry | Library code over KN3 exact polytope operations and KN2.4 pose/label reads | Explicit exact-to-display conversion; no approximate value decides a mechanical predicate. |
| Chart contract and property checks | Exact vertices at exact camera poses | pure, budgets | Library code over KN3 | Declared domain, inverse where defined and information loss; exact domain, injectivity and section-membership checks on the stated inputs. `errors: LB4:type.approximate_property_input`. |
| Projection families | Geometry, with precision-tagged display values | pure; change the view when published | Library code; chapter 10 evaluator | Perspective, stereographic, local chart, Hopf rings and two hemispheres; singular and excluded points retain their domain status. |
| Sections, slabs and clips | Exact model geometry and named exact predicates | pure, budgets; change the view when published | Library code over KN3 exact polytope operations | A true section, a slab and a clip have separate predicates; an empty result concerns that predicate and input geometry. |
| Layouts and atlases | Canonical objects at a stated revision | read a snapshot; change the view | Library code over exact KN2.4 reads | Per-object layouts, atlas membership, trees and dimension-tagged cell-complex incidence; evaluated per revision. |
| State-graph representation | Explicit states and transitions | pure; change the view when published | Library code over declared graph data | Rendering a supplied graph chooses no word. Expanding a graph by puzzle-word search is a research tier candidate. |
| Map property checks | Explicit domain, codomain, metric and map | pure, budgets | Library code over exact scalar/geometry operations | Homomorphism, injectivity, embedding and isometry checked separately. One pass implies none of the other properties. |
| Linked selection | Canonical IDs with level and source relation | change the view | Library code; chapter 10 source maps | Carries the solver's pick between representations. A missing image means no image under that map, distinct from an unknown relation. |

- **LB4.2 [D]** A chart's declared properties are decided exactly at exact camera poses, the identity or an element of
  K⁺ (partition LB4; DT-L1 M4, R2-E2, R3-M2). Domain is decided by an exact region test over KN3's polytope operations,
  injectivity by a theorem or an exact region test, and section membership on the posed vertices, to which convexity
  reduces it. A per-vertex test can refute a region property and cannot establish one: with a stereographic pole on the
  axis of cell c, the centre piece's 8 vertices stay at least 0.2296 from the axis while its centroid lies on it. Between
  exact poses, domain membership is a display decision that decides nothing. A precision agreement test does not
  establish injectivity, domain validity or an inverse. A property checked on a finite set is scoped to that set.
- **LB4.3 [P LQ7]** If LQ7's recommendation is accepted, user projection charts use chapter 10's chart subset and its
  CPU reference evaluator and portable HLSL path. This chapter defines no subset of its own. Under built-in-only
  charts, the same entry contracts apply to the parameterised built-ins and user projection evaluation is absent.
  Under a user shader language, chapter 10 must supply its separate admission and precision contract. Exact property
  checks remain required in each case.
- **LB4.4 [D]** Per-object layouts evaluate per revision, and picking maps a display instance to its canonical ID
  through chapter 10's source map (default 7; partition section 8.5, DT-L1 R2-E2). A chart inverse never identifies the
  picked object. A layout filter cannot alter the mechanically analysed set (INV-6, INV-8).

## 6. LB5: research library

- **LB5.1 [D]** These entries cover partition LB5. A computed state is a kernel-derived value with its reachability
  claim in a named puzzle (INV-1). Each entry which searches or selects over twists, words or caps is a research tier
  candidate. Its implementation or a theorem about its termination does not admit it in a solving session.

| Entry | Level | Effect | Backing | Notes |
| --- | --- | --- | --- | --- |
| Restricted word search | Poses or labels, with typed goal | read a snapshot, budgets; seeded randomness if used | Library code over KN2.4 `classify`, `effect`, `replay` and observations | research tier candidate; finite families, exhaustive prefixes and beams recorded separately. Empty means only the searched set has no witness. |
| Search reductions and gates | Word families with their puzzle and level | pure, budgets; read a snapshot for instance tests | Library code; KN6 for certificates | research tier candidate; minimality, dynamic targets, symmetry and commuting reductions cite lemmas preserving the required conclusion. Brute-force agreement is a scoped test, not the lemma. `errors: LB5:evidence.reduction_scope_mismatch`. |
| Strategies, selectors and ranks | Exact states and typed progress predicates | read a snapshot, budgets | Library code over KN2.4 observations and `replay` | research tier candidate; P*, P**, P***, T3P chains, routing and WC-PLAN. Availability, closure and strict-decrease obligations stay separate. |
| Release structures and accounting | Jumbling poses; explicit AND/OR graph | read a snapshot, budgets | LB1 graphs/fixed points; LB3 exact observations | A supplied graph and ledger can be checked. Searching schedules, enablers, conjugated releases or a viable controller is a research tier candidate. |
| Theorem-schema instance check | Stated puzzle, level and named word | read a snapshot, budgets | Library code for hypotheses; KN2.4 `replay`; KN6 | Decides hypotheses and checks the supplied conclusion. Constructing or selecting its solving word is a research tier candidate. An unmet premise leaves the implication inapplicable. |
| Conditional models and partial actions | Named finer/coarser levels and derivation | pure; read a snapshot for a model instance | Library code; KN1 maps and exact KN2.4 reads | Regional profiles, quotients and restrictions state their maps and preserved subjects. Recognition of a supplied instance does not assert its reachability. |
| Regional-family recognition | Jumbling poses | read a snapshot, budgets | LB1 exact bipartite matching over full reference pose tables | Γ(Y), type-preserving bijections and K⁺ gauges; transfers legality and shape-level claims (KN1.16). No automatic sticker-goal, distance or reachability transport. |
| Admitted-domain membership | Jumbling poses, declared D[k] | read a snapshot, budgets | Library code over KN2.4 `classify`, `replay` and branch predicates | research tier candidate when it enumerates legal words. Complete rejection is `OUTSIDE_CERTIFIED_DOMAIN(k)` with that domain's scope (LB5.4). It is a completed answer and a value, not an error (ERR.21, ERR.22). |
| Closure-set negative certificates | Explicit alphabet, state set and ceiling | pure; read a snapshot, budgets for checking | Library certificate builder; KN6 and KN2.4 `replay` | Includes stage 14's finite neutral-plateau certificates and stage 15's excursion closure sets. Constructing the closure by word search is a research tier candidate. A negative requires all admitted edges, including raising letters within the ceiling. |
| Bound calculation and compressed witnesses | Exact scalar/word values | pure, budgets | LB1 integers and straight-line programs | Fixed/controlled costs, lexicographic ranks and symbolic bounds with named premises; no expansion claim follows. Selecting a witness is a research tier candidate. |
| Experiment runs and branches | Kernel-derived states and their inputs | read a snapshot, budgets; change the workspace only through commands | Library code; KN4 queries and commands, KN7 jobs | Chapter 08 owns branch lifecycles; chapter 05 owns `RunRecord`; chapter 09 owns `ReproKey`. No branch or record becomes an authoritative puzzle copy. |
| Certificate builders, import and replay | Certificate-specific puzzle and level | pure, budgets; use a granted file on import | Library builders; KN6; KN2.4 `replay`; chapter 05 import | Builders do not confer a pass. Checks use their own exact inputs and mutation gates. Importing a recorded label is not fresh verification. |

- **LB5.2 [D]** A search declaration records its family, alphabet, bounds, beam, seed and stop reason, with input and
  module identities, deterministic budget and cost unit (partition LB5, INV-10). An unused beam or seed is explicitly
  inapplicable, not an omitted source of nondeterminism. The result states which parts were exhaustive. Stops by a
  clock, error or budget give no negative. A completed bounded search can refute existence in its searched set even
  when the larger family remains open. Chapter 05 owns the encoding, chapter 06 the claim.
- **LB5.3 [D]** A reduced search's negative reaches its unreduced family only through a lemma with the matching
  family, alphabet, hypotheses and conclusion (partition LB5; DT-L1 M4). A local commuting reduction is not a trace
  normal form. A dynamic target may be absent from the initial own holders. A beam which discarded a representative
  cannot support an exhaustive-family negative. Stage 15's beamed R and later unbeamed stop search remain different
  coverage records; the replay certificate of their chosen macro certifies neither search's completeness.
- **LB5.4 [D]** `OUTSIDE_CERTIFIED_DOMAIN(k)` refutes membership in the stated D[k] and says nothing about
  reachability or solvability (partition 8.1, 10.3; stage 19 README). The membership enumeration and the promised
  reachability needed by sticker synthesis are separate contracts. Successful geometric admission does not prove that
  promise. A cut-off enumeration gives unknown membership, never this rejection. Increasing k indefinitely is an
  eventual search, not a new quantitative bound.
- **LB5.5 [D]** Ranks cite their transition contract. For stage 19, fixed-length and controlled-cost bounds retain
  the selector, availability and invariant-domain premises; checked decrease on stored blocks does not discharge them
  globally. The 6,775-digit B11 is a theoretical bound (source evidence: stage 19 README), not an expanded word or a
  measured run. A frontier exit, low height and a restored sticker state remain different goals.
- **LB5.6 [D]** Family recognition and claim transport use the preserved subject of the map (KN1.13; DT-L1 M3).
  Γ(Y) matches the full background, including collateral, through exact matching. It transfers legal-word languages,
  legality along prefixes and shape-level claims such as q, n, Φ and H (N1, B.7). It can permute which physical piece
  occupies a region, so it supplies no sticker-goal or reachability transport. Reference indices are counted separately from distinct fibres. Exact transport, fresh
  audit and actual replay remain distinct check modes (DT-L1 R2-E1); chapter 06 defines their evidence reading.
- **LB5.7 [P LQ4]** With LQ4's recommendation, experiment branches are engine-owned journal branches and a job on a
  digest-pinned input may finish after a commit. Under its alternatives, research uses separate sessions or the pinned
  job is cancelled. All options retain immutable derived states. A result reaches a newer revision only by rerunning
  it or by an exact transport with a matching lemma (KN7.5, DT-L1 R2-E1).
- **LB5.8 [O]** The producer-dependent checks in the stage 19 archive need a per-claim implementation plan from LP1's
  KN6 and import specifications. Options are a new independent exact checker for a self-contained recorded claim, or
  retaining a claim as recorded where an unbundled dependency prevents that check. A captured review is never renamed
  as a new check. Section 7 retains all archive rows and section 10 requires checkable certificates to keep checking;
  no absent producer is assumed to exist. This dependency does not authorise restarting the stopped research.

## 7. Coverage of restoration and the research

- **LB5.9 [D]** The following tables map every row of partition 10.2 and 10.3 to named entries above. Each row keeps
  its original subject even where the archive narrows it. Figures here are source evidence from the file named in the
  row or the linked stage README, not measurements made for this chapter. File keys, import versions and byte rules
  belong to chapter 05; the evidence reading of a source label belongs to chapter 06 (default 20, DT-L1 R2-E1).

### 7.1 Restoration: partition 10.2

| Partition row | Named entries | Contract and source |
| --- | --- | --- |
| Chronological signed-generator words and word/star recipes | LB1 Literal words; LB2 Star and recipe construction | `core.py`'s `normalize`, `net`, `expand`; LB2.4 keeps the retained recipe's step, total and chunk bounds. |
| Stars, certified seeds, guarded buffers, exact frames | LB2 Seeds and full seed analysis; Guarded setups and buffers; Star and recipe construction | Full seed action and ordered slot-tuple frames, with the guarded setup predicate; `core.py`, retained theory and solving workflow section 3.1. |
| Collateral, Net/Strict, block and position locks | LB2 Collateral and support; Net and Strict protection; Blocks and work sheets | Full operation and all literal prefixes for Strict, with unchecked coverage explicit; `session.py`, solving workflow sections 2 and 4.1. |
| Exact length and cost before a run | LB1 Straight-line programs; LB2 Star and recipe construction; LB3 Release and whole-word checks | KN2.10, KN5.10; no expansion or silent splitting to evade admission. |
| Orbit-first blocks, work sheets and residuals | LB2 Orbit catalogue and collateral stage order; Residuals and exact completion; Blocks and work sheets | Solving workflow sections 2 to 4; chosen roles and full collateral remain visible as data. Target-selection entry stays a research tier candidate. |
| Pinned menu letters | LB3 Menus and pinned letters | State contract A4 and ENG-0J: both menu identity and order digest, plus contract revision. |
| Four outcomes, no change on refusal, checked straddlers | LB3 Admissibility and blocked-twist check; Straddlers and u at a named cap | State contract section 3, KN2.8; set and certificates rather than one producer-chosen witness. |
| Partial word actions and domain-preserving keys | LB1 Literal words; Word equality keys | LB1.9, state contract section 3; identity letters keep a grip's domain condition. |
| Poses, stickers, protection and the solve end | LB3 Protection; Retained checkpoint, label checkpoint, sticker test | State contract A3/A5; no label-to-pose protection transport and no pose completion fact. |
| N, D, Φ, heights, units, clean steps, crossings | LB3 Rim and seam facts; Heights, branches, domains and Φ; Units and clean-step contract; D and release accounting; Mode crossing prerequisites | Research data section 1, restoration theory M2/M3, A5; N includes off-grip seams and Φ = 0 does not imply N = 0. |
| ν, corrections and the icosian tree | LB1 Exact linear algebra; Quaternion and icosian algebra; LB3 ν and drafted correction-set check; Correction catalogue | A5 and restoration theory T1 to T6. Correction-combination solve is a separate research tier candidate. |
| Repair list at a named cap | LB3 Repair list at a named cap | LB3.6; owner, 17 October; DT-J1 O9 to O11. The exception does not select a cap. |
| Long words and scramble jobs | LB1 Straight-line programs; LB3 Release and whole-word checks; Scramble construction and research descent | The figure 672,417 is an earlier recorded step, not a maximum; LB5.13 lists the recorded lengths. Scramble rejection sampling is job work under ENG-0J, with no inferred solving grant. |

### 7.2 Stage 19: partition 10.3

Source for these rows: [stage 19](../../../../../research/jumbling/restoration/19-height-depth-framework/README.md).
Its mathematical configurations and captured reviews establish only their stated scopes.

| Partition row | Named entries | Contract and archive scope |
| --- | --- | --- |
| Full pose tables, Γ(Y), gauges and exact recognition | LB1 Quaternion and icosian algebra; Exact matching and integral flows; LB5 Regional-family recognition | Full reference background and type-preserving matching. The finite recognition algorithm is mathematical; a general practical matcher was not built. Legal-word languages and shape-level claims transfer; sticker goals do not (KN1.14, KN1.17). |
| Chronological blocks, continuations and frontier exits | LB1 Literal words; LB3 Release and whole-word checks; LB5 Theorem-schema instance check | The 5/9-letter continuations finish at a frontier with q16 remaining. Their stated height ceiling and earlier-type progress do not make them restored states. |
| R3, S, R_L and lexicographic descent | LB5 Strategies, selectors and ranks; Bound calculation and compressed witnesses | Each rank retains its block contract. The length-3 rank lemma does not cover the four-letter D block; S covers its stated cleanup progress. |
| Conditional bounds, obligations and reachability promise | LB5 Theorem-schema instance check; Conditional models and partial actions | Availability, invariant-domain and reachability premises accumulate. Chapter 06 owns discharge by identical statement and scope (DT-L1 R2-E1). No finite check proves global availability. |
| D[k], finite legal enumeration and rejection | LB5 Admitted-domain membership; Restricted word search | Complete membership test over the declared neighborhood of the three branches. `OUTSIDE_CERTIFIED_DOMAIN(k)` has LB5.4's narrow meaning. |
| Large theoretical bounds | LB1 Big integers and symbolic bounds; Straight-line programs | B11's 6,775 digits are exact theoretical data; neither an expanded word nor a practical runtime was claimed. |
| Peak height and no overshoot | LB3 Release and whole-word checks; LB5 Theorem-schema instance check | Prefix peak over every collateral piece. The no-overshoot theorem covers the guarded pump-prefix regional domain with its promise, not arbitrary high states or shortest word length. |
| Synthetic inputs with unknown reachability | LB5 Experiment runs and branches; Conditional models and partial actions | The synthetic cycle's forward/inverse checks keep reachability unknown. They do not certify reachable sticker restoration. |
| Indexed references, fibres and check modes | LB1 Collections and relations; LB5 Regional-family recognition; Certificate builders, import and replay | The 73 indexed references are not asserted distinct fibres or fresh replays. Reused checks, fresh exact audits and actual replay stay separate. |
| IMPORT-MANIFEST and compressed/public/source digests | LB5 Certificate builders, import and replay | `IMPORT-MANIFEST.json` binds stored and uncompressed public bytes separately from the captured source digest. Chapter 05 checks both relevant public digests and deterministic gzip; historical hashes are not hashes of sanitised bytes. |
| Absent executable producers | LB5 Certificate builders, import and replay; Theorem-schema instance check | Data and checkable witnesses are importable; complete histories and producers are absent. LB5.8 keeps dependencies explicit. No portable end-to-end harness is claimed to exist. |

### 7.3 Stages 14 to 18: partition 10.3

| Partition row and archive | Named entries | Formats and exact scope |
| --- | --- | --- |
| [14 Blocking structure](../../../../../research/jumbling/restoration/14-blocking-structure/README.md) | LB1 Finite fixed points; LB3 D and release accounting; Release and whole-word checks; LB5 Release structures and accounting; Theorem-schema instance check; Restricted word search; Closure-set negative certificates | The `t3p-*/1` family includes `t3p-certificate/1`, `t3p-check/1`, `t3p-scan/1`, `t3p-pairs/1`, `t3p-plus/1`, `t3p-levels/1`, `t3p-locality/1`, `t3p-chain/1`. Exact Δ(c,d), greatest-fixed-point levels, P0 to P4, plateau certificates and lemma chains; replay rows: letter, applied, n₂, n₄, n₆, maximum height, Φ, off count and digest. A T3P scan's empty family does not exclude other descents. |
| [15 Lowering machine](../../../../../research/jumbling/restoration/15-lowering-machine/README.md) | LB1 Graphs, trees and AND/OR graphs; LB3 Release and whole-word checks; LB5 Strategies, selectors and ranks; Closure-set negative certificates; Certificate builders, import and replay | `macro-certificate/1`, `macro-check/1`, `pstar-census/1`, `pstar-step/1`, `pstar-run/1`, `pstar2-stop/1`, and `pstar2/2`. Macro claims cover chronological supports, delta, phi, digests, excursion, F and D. Theorem MC supplies the event calculus; the serialised macro format requires descending terminal-jumble macros. P* records its five families, exhaustive depth and beam; the good-set controller and AND/OR obligations retain their hypotheses. |
| [16 Six jams](../../../../../research/jumbling/restoration/16-six-jams/README.md) | LB5 Search reductions and gates; D and release accounting through LB3; Release structures and accounting; Theorem-schema instance check; Restricted word search | `s16-*/1`, `jams.json`, whole-descent `macro-certificate/1` and formatless captured records. Theorem X, minimality and dynamic targets retain their reduction lemmas; B = D − F is evaluated along the route. Cover tests exclude helpers outside their declared class. Conjugated release needs the dynamic inverse-step condition and the full chronological excursion. Alphabet-restricted negatives and leaf-schema bounds remain restricted. |
| [17 Routing in the driver](../../../../../research/jumbling/restoration/17-routing-in-driver/README.md) | LB1 Literal words; LB5 Restricted word search; Search reductions and gates; Strategies, selectors and ranks | `s16-rr/1`, `s16-pxrr/1`, `s16-transport/1`, and the stage 15 stop-search record. RR records carrier actions, helpers, releases and all five tail forms, with its reductions checked against brute force. P*** adds RR at stops and retains the Φ/2 macro bound's premises. A stop within its carrier/helper/tail bounds is not a full-word negative. |
| [18 Tail completion](../../../../../research/jumbling/restoration/18-tail-completion/README.md) | LB1 Exact matching and integral flows; Permutation groups; Straight-line programs; LB5 Theorem-schema instance check; Strategies, selectors and ranks; Certificate builders, import and replay | `steps.json` with per-step words and both recorded forms; `r22b10/1`, `r22b10/2`, `runner-run/3`, `literal-lattice/1`, `lattice-phase-play/1`. Hole profiles, layered integral flow, independent group factors, Schreier tables and Jordan witnesses support static plans. WC-PLAN selects by stage count then (cap,item) order and is a research tier candidate. Its 10⁷-literal-letter admission is a recorded rule, not proof that every plan meets it. |

- **LB5.10 [D]** A fresh certificate pass requires claim-by-claim checking through KN6's independent path, without
  producer search or construction code; mutation gates include wrong input tables (partition 10.3, KN6). A producer's
  check record remains importable even when no fresh check can run. Its label is preserved through chapter 05's table
  keyed by format, field and value, with unmapped values retained word for word as recorded (default 20, DT-L1 R2-E1).
  Formatless records keep their source identity; no fabricated format is inserted to make a claim checkable.
- **LB5.11 [D]** The mathematical event calculus of stage 15 applies to admissible words, but its stored
  `macro-certificate/1` checker accepts descending macros ending in a jumble (stage 16 README, section 2.6). A release
  alone, such as SJ-E1, uses letter-by-letter checking and is not forced into that format. The partition's stage 15
  row states this restriction. The format inventory also includes `pstar2/2`, written by
  `15-lowering-machine/scripts/pstar2.py`, and the row names it.
- **LB5.12 [D]** Stage 18 checks retain three separate gates: exact admitted length and expanded hash; composed
  strict′ check; complete twist-by-twist replay from the actual prior end (stage 18 README, section 3.3). A chunked
  replay counts as complete only when all word spans concatenate, their boundary digests chain and the final digest
  matches. The archive records T6 steps 1 to 3 through all gates; T6 step 4 and the rule's three stages passed only the
  first two when frozen. Import never fills the unfinished gate from a composed pass. Dropping V′⁻¹ changes a step's
  end, so a later stored word must be rebuilt or separately justified; the two forms are not interchangeable.
- **LB5.13 [D]** The long-word coverage takes in every recorded length (source evidence; DT-L1 R3-E4). The longest
  literal word stored flat has 2,352,984 letters (stage 18 `results/lattice-d1/run/words/00014.ia16.gz`, rule
  `lattice/orbit-10`), and the 53 words of that phase hold 13,231,266 letters. The stage 18 T6 step of 871,582 literal
  I_a menu letters (its README, sections 2.2 and 3.3, and `results/d1-tail/steps.json`) and the 672,417 of ENG-0J's
  earlier observation stay regression cases. Longer words exist only composed: 166,788,405 letters checked by
  composition and never played, and 224,683,850 in the largest retained cascade. The T6 word's incomplete third gate
  is a fact of its own, and no length here implies a new replay result. Chapter 01's streaming contract covers all of
  them (KN2.10).

## 8. LB6: built-in tools and PrincipleRecord

- **LB6.1 [D]** Every built-in tool ships viewable source and a `PrincipleRecord` (INV-12). The record states the exact
  function and domain, the meaning of empty and negative answers, its lemma or certificate with evidence, one small
  instance on the real geometry whose steps a person can check by hand from named givens, and the decomposition into
  base parts. A tool's displayed claims can be traced to those primitives and evidence. An abstract toy geometry alone
  cannot be this instance. A digest is never a step.
- **LB6.2 [D]** The record is data in the module manifest. Its instance runs as a headless fixture, the decomposition
  is generated from the typed IR before lowering, from the call graph and kernel calls, and its justification resolves
  to `EvidenceEnvelope` references where evidence exists (partition LB6; DT-L1 R2-E4). The record is bound to the module
  digest, its lockfile closure, the model identity and its statement IDs, and a changed decomposition marks the
  hand-written parts stale. Chapter 02 defines the manifest and IR; chapter 05 their stored forms; chapter 06 the
  envelope's meaning. A generated decomposition gives structure, not reasons: the floor's call graph is two set sizes and
  a subtraction, and its reason is the invariance of In_c and Out_c (DT-L1 R3-M4). So the record also holds what a person
  writes by hand. The conceptual content is:

  ```text
  PrincipleRecord = {
    specification:  function or declared procedure; inputs, parameters and budget; output type; puzzle, menu and
                    level; domain; tie rule; empty-answer meaning; negative-answer scope
    justification:  a list of arguments, each a lemma, a per-output certificate kind, a replay or "by definition",
                    with an evidence-envelope reference where one exists
    instance:       real-geometry input, expected claims, headless fixture reference
    decomposition:  generated IR call graph with library entries and kernel calls
    by hand:
      step map:     a relation between nodes and (statement ID, step label), reporting the unmapped items on both sides
      reason:       the statement ID and a short proof idea in glossary terms at or below the record's level
      failure:      per hypothesis: a concrete counterexample, a weaker conclusion or "necessity open"
      not meant:    what an empty or negative answer does not mean
      givens:       the facts the instance reads from named leaf calls or certificates
      steps:        what a person verifies from the givens, with the step count
      prerequisites: glossary terms and earlier records (bar M6)
      leaf specs:   a specification for each opaque kernel leaf, naming the oracle test that covers it
  }
  ```

  These are content requirements, not storage keys. Every part admits a typed "none, because ...": a check has no empty
  answer, and a definition has no hypothesis. The domain lies inside the cited statement's scope, or the record names the
  part outside it and what the answer means there. As a working definition, the principle of a tool is the statement
  named in `reason`, with its hypotheses. Prerequisite records form a DAG, and each prerequisite term sits at or below
  the record's level. The fixture names model identity and the level it checks. Its claims and checker trace resolve
  through their recorded digests. The decomposition names transitive calls and their effects; it cannot conceal a kernel
  call behind a hand-written description. Higher-order calls over-approximate, and such an edge is marked "may call". A
  fixture pass shows that the module reproduced the expected claims at that input. The expected claims come from the
  written derivation, never from the module's own output. Any ceiling on the step count is the owner's (EV5.12).

  The floor shows the split. After 40 twists of seed 7101, cap 30 has 840 cut cells covered inside, 737 outside and none
  aligned (`research/jumbling/manual/02-descent.md`). Those are givens, read from named leaves. A person checks
  840 + 737 = 1,577, 840 − 737 = 103 and, for the twist that brings the cap to its floor, 1,577 − 103 = 1,474 and
  30,286 − 1,474 = 28,812. Nobody checks 840 itself. The floor statement in `research/jumbling/theory/open-problems.md`
  has no stage-qualified name yet, and it needs one before it serves as evidence (EVD.2).

  Admission of an incomplete record has `errors: LB6:evidence.principle_record_incomplete`.
- **LB6.3 [D]** A fork inherits no evidence (default 18). It can retain source references as references, but its new
  module digest has no inherited pass, cost approval or record. Any claim asserted for it needs its own scope and check
  under chapter 06, and its statement counts as unchanged only under an identical statement ID. Every built-in declares
  chapter 07's `BoundaryClass` under BM5, reviewed when it ships. A class declaration and a `PrincipleRecord` are separate
  obligations.
- **LB6.4 [P LQ5]** Under LQ5's recommendation every tool above the kernel is written in the language, using LB1 to
  LB5, while exact engines and independent checkers remain kernel code. Under the native-tool or per-tool alternatives,
  the implementation backing changes; viewable source, the exact specification, real-geometry instance, evidence and
  reviewed boundary declaration remain required by INV-12 and BM5. The generation of a decomposition for any native
  body needs the contract in LB6.6.
- **LB6.5 [D]** The following are candidate built-in tools, derived from H-08's named rows, DT-J1's Outcome and the
  solving workflow. No candidate has a `PrincipleRecord` yet. The class is a candidate BM5 class, not an admission
  ruling. A row that combines several commands gives their common tool data; it does not let language code execute
  `operation.execute`, `session.undo` or another shell/kernel-owned command. ENG-0J's new row names remain EN3's work.

| Candidate tool and source rows | Libraries it uses | Candidate boundary class | PrincipleRecord exists |
| --- | --- | --- | --- |
| Orbit residual and work-sheet inspector: `view.residuals`, `view.findings`, `orbit.switch`, `worksheet.save/load` | LB2 Orbit catalogue; Residuals and exact completion; Blocks and work sheets | tracking | No |
| Piece, buffer and frame inspector: `object.inspect`, `view.entry_slots`, `buffer.assign/analyse` | LB2 Guarded setups and buffers; Residuals and exact completion; LB4 Linked selection | tracking, analysis | No |
| Named-target guarded setup tool: solving workflow section 3.1 and `buffer.analyse`'s preparation data | LB2 Guarded setups and buffers; Star and recipe construction | setup search for a named target | No |
| Word/recipe composer and full-effect inspector: `operation.phase.*`, `operation.cleanup.from_prepare`, `operation.check/preview` | LB1 Literal words; LB2 Star and recipe construction; Collateral and support; LB3 Release and whole-word checks | analysis, checking | No |
| Protection and block-condition checker: `block.add_current/home`, `intent.set`, `protection.*` | LB2 Net and Strict protection; Blocks and work sheets; LB3 Protection | protection | No |
| Solver's macro and work-sheet reuse: `macro.base.open`, `macro.select/add/save`, `macro.variant.create`, `operation.reuse` | LB2 Blocks and work sheets; Named macro analysis and comparison; LB7 workspace modules | tracking, analysis | No |
| Named-macro comparison and residual inspection: `macro.compare`, `endgame.configure` | LB2 Named macro analysis and comparison; Residuals and exact completion | analysis, checking | No |
| Representation, work-set and filter tools: `filter.preview/apply`, `view.effect.scope`, `set.save`, `structure.compare/layer` | LB4 Layouts and atlases; Map property checks; Linked selection | view aid | No |
| Jumbling facts and named straddlers: DT-J1 A1/A2 and ENG-0J observation rows | LB3 Rim and seam facts; Heights, branches, domains and Φ; Straddlers and u at a named cap | tracking, analysis | No |
| Named twist/word checker and kept checks: DT-J1 D1 to D3 and O4 | LB3 Release and whole-word checks; Units and clean-step contract | checking | No |
| Named-cap covers query: DT-J1 O3 (b) | LB3 Covers at a named cap | analysis | No |
| ν, drafted-set check and static correction catalogue: DT-J1 O6 (b), O7 (a) | LB3 ν and drafted correction-set check; Correction catalogue | analysis, checking | No |
| Named-cap repair list: DT-J1 O9 to O11 | LB3 Repair list at a named cap | repair list | No |
| Checkpoint and sticker-test inspector: DT-J1 O8 (1) and ENG-0J crossing prerequisites | LB3 Retained checkpoint, label checkpoint, sticker test; Mode crossing prerequisites | checking | No |

  The candidate list supplies no completed fixture, timing, human understanding result or product implementation.

- **LB6.6 [O]** LQ5's native-tool alternative has no specified way to generate a tool's decomposition from its body.
  The owner and LP1 must choose language wrappers whose meaningful work appears in the typed IR, or an audited native
  call description with a defined check against the implementation. INV-12 still requires decomposition; the IR-only
  mechanism of DT-L1 R2-E4 cannot describe an opaque native body. This dependency is conditional on that alternative
  and settles neither LQ5 nor the implementation route.
- **LB6.7 [D]** H-08's `view.recommendations`, `target.next.activate`, `object.actions`, `structure.find` and any
  macro-comparison mode that selects a solving target or word require a separate content definition and a ruling of the
  joint setting (partition section 11; BND.65; DT-L1 M5, R2-E3). Such a selector is a research tier candidate, with no
  admitted BM5 class or `PrincipleRecord` at this stage. The static inspector versions in the table above do not
  include that selection. A research runner, correction solver or WC-PLAN is also a research tier candidate, outside
  this solving-tool table. Competence results do not themselves grant a preview capability; LQ2 and LQ14 decide any gate.

## 9. LB7: user modules

- **LB7.1 [D]** The local workspace holds the solver's tools, macros, recipes, layouts and bindings, and forks of
  built-in tools (partition LB7). Module identity, dependencies, manifests and lockfiles use chapter 02; stored forms
  use chapter 05. A binding names the command it invokes under chapter 04. Keeping an object in the workspace grants
  no effect, capability or evidence. Entry admission has `errors: LB7:capability.module_grant_missing` when the host
  grant is absent.
- **LB7.2 [D]** Workspace-changing operations run only inside commands the solver invokes and form undoable actions
  (default 16). Program views cannot write the workspace. A macro or work sheet reused at another revision, target or
  reference gets fresh exact analysis and a fresh engine preview; its stored result never replaces that preview
  (default 14, KN5; solving workflow section 4.1).
- **LB7.3 [D]** Stored words, macros and workspace records retain the provenance field through storage and import
  (BM2, DT-L1 R2-E3). A fork is a forked built-in for chapter 07's author-class mechanism and inherits no evidence
  (default 18). A local author class is recorded provenance, not proof that the solver understood or independently
  invented an algorithm. Chapter 07 decides the assistance classes and flows; chapter 09 decides what each module can
  reach.
- **LB7.4 [P LQ12]** Under LQ12's recommendation, sharing uses local files the solver chooses, with content digests,
  and has no registry or network. A later curated registry or signed-package option changes distribution, not the
  need to check identity, evidence, provenance and granted capabilities. Network/process access is not granted to
  running programs by this chapter (default 2).
- **LB7.5 [D]** No solver-tool solving preview is enabled before the joint boundary setting (partition section 11's
  interim reading). Forking a tool, composing its base calls, using a research record or passing a principle fixture
  cannot bypass that setting. Under either LQ21 mechanism, chapter 07 controls preview and display sinks and chapter 09
  controls grants. No module obtains the shell's commit token (INV-3).

## 10. Conformance tests

- **LB7.6 [D]** The following tests are requirements for the library conformance corpus (partition 10.3), alongside
  chapter 01's oracle and checker gates. No conformance test exists yet and none is reported run here. The corpus keeps
  existing checkable certificates passing, altered claims failing and unfinished checks unfinished. A captured label
  alone is not a passing test. Chapter 12 assigns the tests to phases; source fixtures do not establish Windows,
  DirectX, input, long-session or performance evidence.

| Test | Checks | Class |
| --- | --- | --- |
| CT-LB1-01 | F2/F5 equation ranks and solutions, exact Q(√5) signs, quaternion factor reconstruction and tree outputs agree with the pinned research references. A scalar solve asserts no puzzle reachability. | source |
| CT-LB1-02 | At a blocked cap, literal a,a⁻¹ and its domain-preserving key are both refused; empty is admitted. On admitted fixtures, keys preserve the end digest. Prefix protection still reads literal letters. | source |
| CT-LB1-03 | Inverse, conjugate, commutator and powers construct words without state access; application checks each prefix. An inverse returns an admitted path to its start. | source |
| CT-LB1-04 | Straight-line length equals streamed expansion length on small fixtures; huge symbolic bounds compute without expansion. Admission refuses an oversized expansion before allocation and creates no new operation boundary. | source |
| CT-LB1-05 | Fixed points satisfy their finite operator; matching and integral-flow witnesses satisfy every edge/demand constraint; small brute-force instances agree on infeasibility. | source |
| CT-LB1-06 | Under the convention selected in LB1.10, equivalent traces receive one key, including stage 16's bca/cab counterexample. A local commuting representative is not accepted as the key. | source |
| CT-LB2-01 | Retained word/star recipes from the oracle preserve chronological full-label effects, exact frames and collateral; filtering out another orbit changes no reported mechanical support. | source |
| CT-LB2-02 | A word with identity net effect and moving intermediate prefixes passes Net and fails Strict for the relevant captured lock. An incomplete prefix check never returns safe. | source |
| CT-LB2-03 | Recipe chunking preserves the total admission bound and the whole operation; an oversized recipe is refused without silent splitting. Named work-sheet reuse invalidates the old preview. | source |
| CT-LB2-04 | Exact completion reads identities, slots and orientations. Empty candidates, a permitted residual or unchanged colours cannot pass it. | source |
| CT-LB3-01 | Separate fixtures distinguish retained checkpoint, label checkpoint and sticker test. A reachable nonzero-ν lattice state fails the label checkpoint; an imported table without reachability stays unestablished. | source |
| CT-LB3-02 | Centre piece 0 under retained generator 1 distinguishes pose and label protection. A pose refusal cannot be replaced by the label pass at a crossing. | source |
| CT-LB3-03 | Heights and domain IDs agree with LB1 and `trees.py` on ENG-0J item 8's states and H6030. Φ = 0 with height-2 pieces does not pass the sticker test. In free mode, heights (0,0) do not imply the lattice, and domain IDs come from exact coset tests. | source |
| CT-LB3-04 | Off-lattice ν is undefined; the pinned C5 convention round-trips. A planted nonzero fixed I_a coordinate is a consistency error. A drafted correction set is checked without selecting one. | source |
| CT-LB3-05 | A named-cap repair list agrees with the pinned y/z tie rule and neutral order; empty is possible only after completion and is scoped to that repair family. No unnamed-cap job is started. | source |
| CT-LB3-06 | On exact word fixtures, prefix peak, excursion, tameness and the step flag are recomputed separately. A false flag with no protection conflict does not itself refuse an admissible word. | source |
| CT-LB3-07 | Per-revision fact results contain no all-items outcome table. Named-word checks answer only their named word; no recommendation, next-cap skip or correction combination leaks through the inspector entries. | source |
| CT-LB4-01 | At exact camera poses, declared domain is decided by an exact region test and section membership on the posed vertices, including boundary cases. A chart whose vertices all pass and whose domain fails at an interior point is refused. Homomorphism, injectivity, embedding and isometry each require their own check. | source |
| CT-LB4-02 | Between exact camera poses, approximate domain decisions reach only display data. Replacing them changes no admissibility, protection, digest or exact claim. | synthetic |
| CT-LB4-03 | Multiple display instances map to the correct canonical IDs and levels; chart inverse values cannot identify a pick. Layout filters preserve the mechanically analysed set. | source |
| CT-LB5-01 | Stage 14: import every `t3p-*/1` kind and check W3/W5 certificates, P0 to P4, locality and fixed-point levels; changed helper sets, precedence or input digest fail; replay rows preserve chronological observables. | source |
| CT-LB5-02 | Stage 15: the R01 `macro-certificate/1` and other corpus certificates reproduce their supports, changes and digests. Tampered claims fail. P*'s beam stop and the unbeamed stop retain different scopes; `pstar2/2` imports. | source |
| CT-LB5-03 | Stage 16: jam words and whole-descent certificates chain from their exact tables. Reduced transport matches small unreduced cases without promoting that test to a lemma; SJ-E1 remains a release check. Leaf/alphabet negatives retain scope. | source |
| CT-LB5-04 | Stage 17: RR canonical-word and five-tail gates agree with brute force on its recorded gate inputs. `s16-rr/1`, `s16-pxrr/1` and separate stop records retain every bound; bounded stops do not refute all words. | source |
| CT-LB5-05 | Stage 18: both `steps.json` forms, R22B-10 versions and lattice/runner records import. Recheck the archive's completed gates and mutation controls; T6 step 4 and incomplete rule replays remain missing their third gate. A shortened stored chain is not accepted without rechecking its changed input. | source |
| CT-LB5-06 | Stage 19: stored/uncompressed manifest digests, full reference tables and checkable certificates still check. Check R3/S block decrease and R_L's length premises; frontier exits fail the sticker goal. Indexed references are not asserted distinct fibres; Γ(Y) preserves legal prefixes without transporting sticker goals. The synthetic cycle retains unknown reachability. Missing producer dependencies are reported. | source |
| CT-LB5-07 | One search key repeats coverage and stop reason; a budget, clock or error stop yields no negative. A reduced negative cannot leave its family without a matching reduction lemma. | source |
| CT-LB5-08 | A complete D[k] rejection yields `OUTSIDE_CERTIFIED_DOMAIN(k)` only. Changing k or a reachability promise changes the applicable scope; neither rejection nor a budget stop means unreachable. | source |
| CT-LB5-09 | Stream the recorded 672,417, 871,582 and 2,352,984-letter cases through length/admission checks. Any newly completed replay has its own check record; import does not claim the historical third gate completed. | source |
| CT-LB6-01 | Each shipped tool has all PrincipleRecord parts, resolving source/evidence digests and a runnable real-geometry fixture. A part that does not exist reads a typed none. A domain outside the cited scope is named. Regeneration of the decomposition accounts for transitive library and kernel calls, and a changed decomposition marks the hand parts stale. Expected claims come from the written derivation, not from the module's output. | source |
| CT-LB6-02 | A changed module digest or fork receives no inherited evidence pass. Original references remain references and cannot certify the fork's behavior. | source |
| CT-LB6-03 | The tool's declared BM5 class is reviewed against its exact entry definition. LX-3's correction solve, state-based target selection and storage-laundering cases are refused according to chapter 07's selected mechanism and sinks. | source |
| CT-LB6-04 | A person can check the steps of the tool's small real-geometry instance by hand from its named givens and relate them to the generated decomposition; the owner sets the human-test form and any learning gate under LQ14. | human |
| CT-LB7-01 | Workspace views cannot write records; an invoked workspace command is undoable. Save/load and import preserve provenance and cannot turn research output into an interim solving preview. | source |
| CT-LB7-02 | The same module runs only with chapter 09's grants and chapter 07's profile policy; local storage, a fork or a binding grants no token or extra capability. | source |

## 11. Added error codes

- **LB7.7 [D]** The following are this chapter's proposed codes in default 5's categories, for collection and final
  spelling by chapter 13. Their meanings follow the contracts cited below. Existing kernel identity, stale, budget,
  cancellation and admissibility results retain chapter 01's codes. Unknown, a false check and an unfinished search
  are result states, not generic failures.

| Code | Meaning and clause |
| --- | --- |
| `LB1:input.incompatible_alphabet` | Word composition/application uses incompatible puzzle alphabets or pinned letter meanings; LB1.3, LB1.9. |
| `LB2:protection.incomplete_coverage` | The requested protection assurance cannot cover the whole executed prefix; no safe verdict is issued; LB2.3. |
| `KN2:consistency.fixed_nu_nonzero` | A reachable I_a result has a nonzero coordinate that A5 fixes at zero; LB3's ν entry. |
| `LB4:type.approximate_property_input` | An approximate display value is supplied where an exact chart-property input is required; LB4.1, LB4.2. |
| `LB5:evidence.reduction_scope_mismatch` | A reduction lemma is offered for another family, alphabet, hypothesis set or conclusion; it cannot transport this negative; LB5.3. |
| `LB6:evidence.principle_record_incomplete` | A proposed shipped tool lacks a required record part or a resolving evidence/fixture reference; LB6.1, LB6.2. |
| `LB7:capability.module_grant_missing` | A module's requested entry effect lacks the host grant required by chapter 09; LB7.1, LB7.5. |

## 12. Open items

Every LQ remains open. The following are the interfaces each answer touches here; alternatives are still those in the
partition. No candidate list or conformance requirement is an owner answer.

| Owner question | Clauses | What stays; what can change |
| --- | --- | --- |
| LQ1 Language/profile split | LB1.1, LB5.1, LB7.1 | Exact model identities and evidence stay shared. A separate or excluded research language changes LB5's host and integration; this text does not decide that split. |
| LQ2 Human-solve boundary | LB1.5, LB1.6, LB3.5, LB6.7, LB7.5 | Entry definitions, execution gates and principle records stay. The joint setting chooses permitted calls, author classes, sinks and option set. |
| LQ3 Evidence model | LB1.4, LB5.5, LB6.2 | Scope and the distinction from permission stay. Chapter 06 supplies the selected representation, evidence order and labels. |
| LQ4 Branches and pinned jobs | LB5.7 | Immutable derived values stay. Branch placement and whether pinned jobs survive a commit change. |
| LQ5 Built-in implementation | LB6.4, LB6.6 | Viewable source, principles and boundary review stay. Native/per-tool backing changes the decomposition mechanism. |
| LQ6 Research views | LB4.1, LB7.1 | Representation data and canonical IDs stay. Where a research representation may be hosted belongs to chapters 07 and 10. |
| LQ7 User charts | LB4.2, LB4.3 | Exact property scope stays. Built-ins alone, the chart subset or user shaders change projection evaluation and its gate. |
| LQ8 Engine language | LB1.1, LB3.1 | Exact API behavior and oracle agreement stay. Numeric implementations and batch backing can change. |
| LQ9 Runtime route/placement | LB1.6, LB6.2, LB7.5 | Library contracts and no commit token stay. Chapter 11 chooses the host and supported provenance implementation with LQ21. |
| LQ10 Data-core freeze | LB1.7, LB5.9 to LB5.12, LB7.3 | Source bytes, scope and labels remain distinguishable. Chapter 05 decides envelope storage and freeze timing; DT-L1 R2-E1 is explicitly relied on for the import mechanism. |
| LQ11 Recording | LB7.1, LB7.2 | Reused words need fresh checks and explicit commits. Chapter 04 decides recording/playback form; this chapter consumes those objects. |
| LQ12 Module sharing | LB7.4 | Identity, provenance and grant checks stay. Distribution may change. |
| LQ13 Syntax | LB1.2, LB6.2 | Conceptual contracts stay. Concrete function/module syntax is chapter 02's work after the decision. |
| LQ14 Learning gates | LB6.1, LB6.7, CT-LB6-04 | A principle record stays required. Whether competence guides or gates capabilities remains open. |
| LQ15 Formal proofs | LB5.5, LB6.2 | A certificate check establishes only its actual scope. A proof-assistant link can add evidence under chapter 06; none is assumed here. |
| LQ16 Product cut | LB5.1, LB6.5 | The full catalogue records the requested scope. Shipping extent, including research, remains the owner's decision. |
| LQ17 Name | Introduction, LB1.2 | Entry concepts stay. Working language name and extension are not fixed interfaces here. |
| LQ18 Spikes | LB1.1, LB4.3, LB6.7 | Requirements remain unmeasured. Whether and when LX experiments run belongs to chapter 11; none is authorised by this chapter. |
| LQ19 Gates | LB5.8, LB6.6 | These are draft interface dependencies. Implementation timing and the charter amendment remain the owner's decision (DT-L1 E6). |
| LQ20 Renderer handoff | LB4.1, LB4.3 | Renderer-independent representation contracts stay. Chapter 10 and the renderer line decide the handoff and available primitives. |
| LQ21 Flow-control form | LB1.6, LB6.7, LB7.3, LB7.5 | The correction-solve composition and storage provenance problem remain under either form. Labels and capability separation change which combinations can run. |

The clauses tagged [O] identify unresolved mechanisms or dependencies:

| Clause | Open dependency and deciding work |
| --- | --- |
| LB1.10 | LP1 chooses and specifies the global trace-key convention and its independence relation. |
| LB3.9 | EN3/LP1 packages total N, seams, D, units, Φ and chronological observations into named exact kernel results or library aggregation. |
| LB5.8 | LP1's KN6/PS3 specification separates self-contained stage 19 checks from claims with unbundled producer dependencies. |
| LB6.6 | If the owner chooses native built-ins under LQ5, the owner and LP1 specify their checkable generated decomposition. |

Further dependencies and source differences:

| Item | Clauses and disposition in this chapter |
| --- | --- |
| DT-L1 obligations | Round 3 cleared them (overview section 8). LB1.1, LB1.6, LB1.7, LB3.1, LB4.2, LB5.6, LB5.9 and LB6.2 cite the relevant cards, and LB3.1, LB4.2, LB5.6, LB6.2 and LB6.3 carry round 3's corrections. |
| EN3, EN4 and LP1 | Public J1 API before binding (default 12), named observation and mode rows, workspace queries/commands, lanes and manifest/effect contracts; LB1.2, LB3.9, LB6.5, LB7.2. Chapter 01's pending interfaces are not asserted delivered. |
| Recorded word lengths | LB5.13 and KN2.10 carry the figures of DT-L1 R3-E4, and the partition is corrected. ENG-0J's "672,417 on record" stays as written in that packet; EN3 should cite the longer figures when it next changes it. Streaming behaviour is unchanged. |
| Stage 15 certificate scope and format name | LB5.11: the partition's stage 15 row now states the terminal-jumble/descent restriction for `macro-certificate/1` and names `pstar2/2`. The coverage rows retain both subjects. |
| Stage 18 replay claims | LB5.12: the archive's section 3.3 corrects its earlier blanket twist-by-twist description. Composition and complete replay stay separate; four words' third gate is unfinished. The shortened-chain totals withdrawn in its section 3.4 are not reused. |
| Historical restoration theory | LB3.2, LB3.3, LB5.5: `theory/restoration.md` still lists retained-group sufficiency and phase-L membership as open and earlier lens-word content as a product take-away; A5 and DT-J1 now govern checkpoints and shipped correction content. Its early height-boundedness question is superseded by stage 10's unboundedness, retained in the archive and research data. Its float centroid cover selector is replaced by DT-J1's exact covers query. Its method-selected caps and words remain research tier candidate entries. |
| Historical workflow/command recommendations | LB2.5, LB6.7: `solving-workflow.md` describes suggested next targets and H-08 contains recommendation rows; the interim policy admits no method-selected target. They stay catalogue leads under the interim reading (BND.65), not shipped solving behavior. |
| Principle instances and conformance corpus | LB6.5, LB7.6: no candidate record or new test exists yet. LP1 and later phases choose real-geometry instances and implement the tests. Human-check form is the owner's; measured cost and native evidence remain absent. |

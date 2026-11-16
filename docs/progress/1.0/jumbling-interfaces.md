# EN3: jumbling interfaces

Status: **draft, 5 November 2026, revised 16 November 2026 for LP2's M5 (JI.52 to JI.54); not a freeze or an owner
decision.** This is the EN3 part of
ENG-0J revision 2. It supplies four portable layer contracts and the H2 command delta in
`docs/progress/1.0/command-rows-jumbling.json`. It contains no implementation or new measurement.

- **JI.1 [D]** Read this draft with `docs/architecture/1.0/language/closure/HANDOFF.md` (H2 to H15),
  ENG-0J at `docs/progress/1.0/packets/engineering/ENG-0J-jumbling-backend.md`, state contract A1 to A6 at
  `research/jumbling/state-contract.md`, and the settled outcomes of DT-J1 and DT-M1 in
  `docs/progress/1.0/design-table/topics/`. A5 governs crossings and A6 fresh rotation requests. DT-J1's adopted aids
  and exclusions stay in force. The working forms on the handoff sheet are integration proposals; the chapters frozen
  on 15 November 2026 consume them, and the command rows stay drafts until the command table freezes at LP3 (PHS.53).
- **JI.2 [B]** This draft consumes, without redefining, chapter 01's `IdentityBlock`, `RevisionRef`, `StateRef`,
  `Partial`, `Verdict`, `Preview`, `JobResult` and snapshot operations; H3's `PreviewDeps`; H4's terminal receipts;
  H5's lanes and availability; H7's word positions; H8's accounting; and chapter 07's `ProvenanceBlock` with H9's
  no-run form. It does not introduce another mutable state, permission path, receipt protocol or provenance policy.
- **JI.3 [D]** Interfaces take `dimension`; this edition admits only 4. Unsupported dimensions return
  `KN1:input.unsupported_dimension`. The full geometry, all 177,120 pieces, 259,800 sticker slots and 1,200 retained
  generators remain unchanged. For the current I_a inputs, use ENG-0J's menu identity
  `fddc8532b9184bd1b413c1032b5a7a305dd9fd1f10385d87ba6ec4fd06eb958c` and separate order digest
  `5a0429d2946fbf58b495d8f300e24a3fa164b1d9518ce4a5c3f4ef0a40625c3c`.
  The contract pin is `state-contract 2026-11-15 A1-A6`: the owner accepted A6 on 15 November 2026, and it replaces
  A1's input map only. J1 (`state-contract 2026-10-09 A1-A4`) stays the reference for sections 1 to 4 and for menu
  letters, and the nu reference (`state-contract 2026-10-22 A1-A5`) for A5. A request's decoding, binding and drag
  scoring are checked against KN3.7 to KN3.9, for which no J1 reference exists. A reference's own stamp names the
  revision it implements; this rule, not equal stamps, decides which reference checks a result. These consume
  existing pins, not a new model or menu choice.
  Comparison-menu evidence stays separately identified. A menu change within an episode is refused by
  `KN5:input.menu_change_in_episode`.

## 1. Engine

- **JI.4 [D]** Every read below takes one immutable `StateRef` and returns its identity, level and input digest.
  Session reads also keep the originating `RevisionRef`. Caps, pieces and words are absolute arguments, resolved
  before invocation. A changed identity returns `KN1:input.identity_mismatch`; an unavailable level returns
  `KN1:input.level_not_held`. A missing state returns `KN2:stale.revision_gone`. None is an empty result.
- **JI.5 [B]** Poses and classifications follow state contract sections 2 and 3 and A1 to A4. Pose entries use exact
  Q(sqrt(5)) triples with arbitrary-size integers, or checked arithmetic with an exact fallback. There is no pose
  height ceiling. Pose equality is exact matrix equality; pose IDs are local to one canonical table. The optional
  A2 filter must agree with the exact test. A blocked or uncertain twist changes nothing. Every prefix of a word
  uses that same test; Net and Strict protection use poses inside Jumble. Retained workflow tools require Jumble off.
- **JI.6 [D]** The following are payload contracts for the existing KN2.4 calls, not parallel engine entry points.
  `Budget` and `LimitsRef` name the admitted profiles in KN7 and the data core. A long call returns H4's running
  receipt and then a KN7.3 `JobResult` containing the named result. An immediate read completes under H4.

| Call and arguments | Typed result | Contract and refusal |
| --- | --- | --- |
| `pose_table(s)`, `pose_ids(s)` | Exact canonical poses, K+ membership per pose, full piece-to-pose map | State section 2, A3; a table and map from different inputs cannot join |
| `protection(s, pieces, word, mode)` | Net or Strict `Verdict` at level poses | KN1.11: inside Jumble the verdict is pose-level, m_A(w) = 1; it carries its level and never crosses to labels (A3, A5) |
| `classify(s, letter)` | `admissible(inside, outside, certificates)`, `blocked(certificates)`, `invalid(code)` or `uncertain(reason)` | State section 3; exact A1 rotation, A4 menu membership; no live change on non-admissible outcomes |
| `survey(s)` | One certified status per grip; coverage identifies any still uncertain grips | State section 3; early exit may prove blocked, but does not claim the full straddler set |
| `rim_summary(s)` | Per cap: `N_c`, `In_c`, `Out_c`, `A_c`, blocked status and floor, with each field's definedness | ENG-0J section 4 and `rim.py`; no loop over menu outcomes; undefined is never zero |
| `heights(s)`, `height_counts(s)` | Per exact pose: domain ID (JI.53), tree positions, branches, left/right heights; per piece sparse height histogram, off count, n2/n4/n6, above-6 count and maximum | LB3.1 and `trees.py`; count multiplicities of pieces, not distinct poses; a zero above-6 count has no maximum |
| `straddlers(s, cap)` | `StraddlerSet { cap, pieces, u, certificate_pairs }` | Full set at the solver's named cap; `u = size(pieces)`; each pair is verified against every region constraint |
| `release_check(s, letter, cap?)` | One-letter `WordObservation`, plus that letter's cap rim after and floor | DT-J1 D1; only a released letter, latest request wins; optional cap supplies delta-u |
| `word_check(s, word, unit?, cap?)` | `WordObservation` of JI.11 | EH-25, LB3.8; the solver supplies word/unit/cap; not an existence test |
| `covers(s, cap)` | Completed `CoverTable` of JI.12 | Exact face-piece definition, not float centroids |
| `repair_list(s, cap)` | Completed ordered `RepairTable` of JI.12 | Only the owner-approved three-twist exception |
| `nu(s)`, `correction_set_check(s, ids_and_multiplicities)` | `Partial<NuVector>` and the predicted vector after the supplied corrections | A5, JI.13; undefined off lattice; no correction selection |
| `retained_checkpoint(s)`, `label_checkpoint(s)`, `sticker_test(s)` | Three distinct `Verdict`s and their scoped evidence | A5, LB3.2; a raw pose table with no reachability chain is not a label checkpoint |
| `replay(s, word)`, `derive(s, word)` | Exact chronological result or first refused prefix, with coverage | KN2.7 to KN2.15; neither adopts a derived value as the session state |
| `derive_from_pose_table(identity, poses, pose_ids, reachability)` | Derived state with reachability open or promised, as its source says | KN2.14: form and identity are validated; validation says nothing about reachability or collisions, and the state is no label checkpoint (A5) |

- **JI.7 [D]** LB3.9 is served by two additional exact batches and explicit library aggregation over existing reads.
  `Batch<T>` here means a logically complete ordered result streamed in bounded chunks under JI.14, not a new
  shared envelope. `observe_jumbling` selects the listed facts; it never selects twists or caps for the caller.

| Observation | Named operation and result | Responsible layer and source |
| --- | --- | --- |
| Total N and seams | `seam_facts(s, scope: all or named_cap) -> Batch<SeamCell>` with `SeamTotals` | New exact engine batch, JI.8; restoration theory section 2 |
| Rim and height facts | `observe_jumbling(s, fields) -> ObservationSet` | Command query aggregates KN2.4 `rim_summary`, `heights`, `height_counts` and the explicitly requested `units`, `height_potential` or `seam_facts` results at one input |
| Units | `units(s, pieces: all_off or named_set) -> UnitTable` | LB3 library grouping over `pose_table`, `pose_ids` and lattice membership, JI.9 |
| D, F, delta-Phi and prefix observations | `prefix_observations(s, word, requested_fields, unit?, cap?) -> Batch<PrefixObservation>` with `WordObservation` | New exact engine batch over KN2.4 replay, JI.10 and JI.11; no caller callbacks |
| Phi | `height_potential(s) -> Nat` | LB3 exact sum over piece-weighted `heights`; the definition is LB3.1, not a new potential |

- **JI.8 [D]** `SeamCell` carries the exact domain ID (JI.53), domain-frame cut-cell signature, its exact transported
  hyperplane, inside/outside occupancy with piece IDs or the fixed core, aligned/mismatched status, and holding
  caps certified to contain the cell. A cell's identity is the pair (domain, cut cell), as in restoration theory
  section 2; coincident geometry does not merge different domains. `seam_facts(all)` enumerates every occupied-side
  pair and includes off-grip hyperplanes. `SeamTotals` contains `total_N`, `on_grip_mismatches`,
  `off_grip_mismatches`, and exact coverage. A mismatched pair contributes one. Total N is the complete count,
  not the sum of displayed N_c. Named-cap scope returns that cap's cells and N_c, with total_N undefined unless a
  completed all-scope result at the same input is joined. Interrupted enumeration has no total_N. For a supplied
  complete table with one on-grip and two off-grip mismatch entries, total_N is 3, not 1. At solved it is 0.
- **JI.9 [D]** `UnitTable` lists every exact-pose group of the requested off pieces in canonical piece-ID order,
  with its members, pose key, domain, heights and branch. It carries no preferred group. A caller may name a
  nonempty subset sharing one pose as a unit; a lattice piece, empty set or mixed-pose set in that argument returns
  `KN2:input.named_object_invalid`. The general clean-step free set is a separately supplied piece set. No unit or
  free set is inferred from which proposal would succeed. DT-J1's step flag uses its narrower conditions in LB3.8,
  not the general free-set contract. `units(solved, all_off)` is an empty completed table.
- **JI.10 [D]** `PrefixObservation` has zero-based letter index, exact letter, before/after pose digests,
  application verdict and certificates, moved set, N before/after, delta-N, histogram delta, optional named-cap
  delta-u, Phi before/after, maximum piece height, and the following optional fields. These are observations of
  the supplied chronological word, never a table over candidate letters.

| Field | Exact meaning and domain |
| --- | --- |
| `foreign_pieces`, `F` | The foreign-load set and count F_x at this letter's cap before it acts, by stage 14 `README.md` section 1; computed from exact inside classification and branches |
| `lowered_high`, `D` | D_x(h) for this supplied jumble letter, by stage 14 Lemma Phi; the exact inside pieces of height at least 4 on its lowering branch; a retained letter returns `undefined(retained_letter)` for D |
| `delta_phi` | Actual Phi after minus before; for an admissible I_a jumble letter it must equal `2*(F-D)`; retained letters have zero change |
| `prefix_state` | Optional immutable derived reference to the checked prefix, charged at its logical size; not a mutable array or an execution permission |

  These F and D are not a free set or a domain D[k]. Their theory predicates are consumed from
  `research/jumbling/restoration/14-blocking-structure/README.md`, not estimated by screen position. Where the
  I_a branch premises do not hold the accounting fields return `undefined(outside_ia_domain)`; actual replay
  and its other defined fields remain available. No result applies the identity to an unplayed or blocked letter.
- **JI.11 [D]** `WordObservation` contains start/end pose digests, exact expanded length, processed prefix range,
  first refused prefix if any, off counts before/after, delta-N, delta histogram, optional delta-u, unit size and
  unit pieces landed, lifted starting-lattice pieces, and other starting-off pieces partitioned into landed,
  exact-pose-unchanged, or changed-and-still-off. Compare exact keys, never pose-table indices across revisions.
  The step flag is LB3.8's conjunction; without a named unit it is `undefined(unit_not_named)`. A false flag or
  positive delta-N is not a protection or admissibility refusal. The word check does not produce a commit token.
  A blocked or uncertain prefix reports its index and leaves the suffix unchecked; no end-state assertion covers it.
  Peak height includes the start and every played prefix. Excursion and tameness use LB3.7; all-letter aggregates
  require complete replay. The one-letter release and word checks agree in every shared field at identical inputs.
- **JI.12 [B]** Covers and repair lists use ENG-0J section 4 and LB3.6 unchanged. A completed cover result lists all
  80 meeting caps for the named cap, in cap-ID order, with the answer in both directions and any undefined
  prerequisite. Its exact face-piece side counts must be equal and at least ten, with every selected cell aligned;
  a straddler is not inside. There is no top-cap answer or map mark. A repair row is `(x,d,y,z,delta_N)`, ordered by
  pinned x then d. y minimises N_d after x and z minimises N_c after x,y, ties to the smaller pinned index; the
  unblocked, nonclosed and strict-lowering premises of ENG-0J must hold and the full repair lowers N. Identity z
  stays visible as data but is omitted from the two-letter draft; no identity action is requested. Selection by
  the solver resolves the row into literal letters before preview. A completed empty list means only no such
  repair at this cap and input. A cancelled or cut-off list cannot supply that negative claim.
- **JI.13 [B]** A5 defines the NuVector and its C5 convention. `nu` off lattice is `undefined(off_lattice)`, never
  zeros, partial coordinates or an old value. Nonzero fixed I_a coordinates raise `KN2:consistency.fixed_nu_nonzero`.
  The correction catalogue returns the 36 shipped constructions in fixed ID order with literal words and fixed
  nu rows. A drafted set check adds only the caller's specified rows with their signed multiplicities in the
  appropriate finite groups. It neither solves for the multiplicities nor lights, filters or reorders the catalogue
  by nu. Every selected construction still needs a fresh whole-word preview; its catalogue check grants no execution.
- **JI.14 [D]** Batches own immutable result buffers. Each request declares a positive maximum chunk size and an
  output/memory/work budget before admission; a seam chunk ends after at most that many cells, and a prefix chunk
  after at most that many literal letters. Exact geometry costs and cancellation safe points inside classification
  remain part of the future binding's tariff, not a timing promise. The engine checks cancellation between chunks.
  Chunks carry input/parameter pins and half-open coverage, and only the final completed result certifies full
  coverage. Close, cancellation, failure or end of the run releases the job's handles and buffers; retained derived
  references require separately charged ownership. KN2.2 and ONE.14 to ONE.19 govern snapshots. Copying versus
  shared buffers, ABI, tariff numbers and buffer-release acknowledgements remain later work; LQ8 and LQ9 are answered
  (owner, 14 November 2026), and LP2's M5 records the chunk sizes it implements (JI.54).
  There is no zero-copy or responsiveness claim here.
- **JI.15 [D]** All job results also pin menu order, contract, algorithm/tie-rule edition, library digest and requested
  parameters. A mismatch is `KN7:stale.job_input_digest`, not an empty list. Read-cache reuse follows ONE.10 only
  when every declared input matches. Preview authority uses H3 instead: equal digests after undo do not revive it.
  LI.28 is the sole mapping from KN7 job outcomes to terminal receipts, including deterministic cut-off and typed
  partial coverage. A partial observation is not permission to relabel an exhausted invocation as completed.
  Queries write no puzzle state. A durable commit stays `committed` even if its subsequent refresh fails.

### One jumbling research operation through these results

- **JI.16 [D]** The following is a contract vector, not a newly run research experiment. The requested operation is
  to inspect the supplied round-trip word `[(0,12),(0,45)]` from solved in the pinned I_a menu. The fixture's exact
  matrices make item 45 the inverse of item 12. No operation chooses a letter.

| Step | Named call and ownership | Determined result |
| --- | --- | --- |
| 1 | `open_snapshot` of solved jumbling state r; caller owns handle s | Poses-level immutable input, with r and its digest |
| 2 | `observe_jumbling(s, {rim, heights, Phi, total_N, seams})`, including `height_potential` and `seam_facts(all)` | total_N = 0, mismatches empty, Phi = 0, all piece heights zero; N was not inferred by adding N_c |
| 3 | `units(s, all_off)` | Completed empty table, not a failure to find a step |
| 4 | `prefix_observations(s, supplied_word, {N, heights, Phi, F, D, prefix_state})`, chunk size 1 | Two chronological rows. Both letters admissible. Each has F = 0, D = 0 and delta-Phi = 0. Peak left height = 2; excursion = 0; tame = true; end pose digest equals solved |
| 5 | `units(first_prefix_state, all_off)` | One exact-pose unit: the pieces moved by item 12; no preferred target field |
| 6 | `word_check(first_prefix_state, [(0,45)], that_unit, cap=0)` | Unit lands, lifted set empty, no other off pieces, step flag true; end digest solved. It is a read of a derived state, not a session commit |
| 7 | `seam_facts(final_prefix_state, all)` | total_N = 0, on-grip and off-grip mismatch counts 0 |
| 8 | Release derived references, close result streams, `close_snapshot(s)` | No live handles or preview; authoritative session still at r |

  If the job is cancelled after row 0, coverage is `[0,1)` and row 1 and the final solved claim are absent. A commit
  during this revision-bound job cancels it under H5; its late output is not installed on the new revision. A
  separately pinned research job finishes on its old input, with no adoption into the new state (KN7.5; owner, 6
  November 2026, LQ4). FR-B03 remains partial until the second wave supplies the route
  binding, release protocol, tariffs and the retained-workflow walk.

### Additions for LP2's M5 (16 November 2026)

- **JI.52 [D]** M5 of LP2 gives J1 a public API before the language binds to J1 (default 12 of the ecosystem
  partition; LP2's products). It is one added module, `research/jumbling/sim/api.py`, whose functions call J1 as it
  stands: no accepted J1 file changes, and J1 stays the accepted reference while the research is stopped. The module
  is oracle engineering for the engineering line, not research. The differential harness under
  `research/jumbling/differential/` then reaches J1 only through this module and J1's public methods (`apply`,
  `classify`, `survey`, `undo`, `replay`, `digest`, `snapshot`, `checkpoint`, `pose`, `lattice_flags`). The names
  below are drafts that M5 may change; no entry is dropped, and none changes a result.

| Private use in the harness today | Public entry point | Contract |
| --- | --- | --- |
| A `State` subclass that captures `inside` from `_commit(twist, inside, journal)` | `apply_with_inside(state, twist, budget)`, returning the outcome and the inside set | The set J1 committed, with no second classification; a refusal returns the empty set and changes nothing |
| `_poses`, `_lookup`, `_kpose()` | `pose_entries(state)` | Per pose ID, its K+ index or exact matrix and its key; a read-only copy, local to the state's canonical table |
| `_pose_matrix(pid)` | `pose_matrix(state, pid)` | The exact matrix of a pose ID |
| `_groups(off)`, `_anchors(pid, members)` | `pose_groups(state, pieces)`, `anchors(state, pid, members)` | The private methods' values |
| `_eval(...)`, `_straddle_certificate(...)` | `evaluate(state, ...)`, `straddle_certificate(state, piece, pid, grip, below, above)` | The same exact evaluation, and the certificate after its exact re-check |
| `_intern_k`, `_intern_matrix`, `_compact()`, `_anchor_cache` | `from_pose_table(ctx, entries, pose_ids, menu)` | A state from a raw pose table, interned and compacted canonically, with empty caches (KN2.14) |
| `seams.SeamRestorer._dom` and `_cls` (`research/jumbling/theory/seams.py`) | `seam_restorer(state, menu, domains, inside_sets)` | The restorer over the supplied domains and inside sets |

  The harness's own private names, such as `census.seams._fixture_state`, are not J1's and stay. Acceptance:
  `python research/jumbling/sim/accept.py` and `python research/jumbling/fixtures/wj.py check` give the results
  recorded before the module; `tests/test_jumbling_*.py` pass; a source scan finds no private member of J1 or of the
  theory code in `research/jumbling/differential/`; and outside that directory the module is the only change under
  `research/jumbling/`.
- **JI.53 [D]** A domain ID names the coset g K+ of a pose g. It is the reference `domain_key` in
  `research/jumbling/differential/references/observations.py`: the SHA-256 of the compact JSON of the sorted images
  g v of the 120 unit icosians, each a canonical reduced Q(sqrt(5)) triple. K+ is the rotation group of that vertex
  set, so two poses g and h have equal domain IDs exactly when g^-1 h lies in K+, and every lattice pose has the
  identity's ID. The ID is defined for every pose, including poses outside the menu's group and cosets its tree never
  reaches. Tree positions and heights are functions of the coset that determine it only inside the menu's group
  (KN1.7, LB3.1); a free-mode pose takes its domain ID from this exact key, never from tree positions. Vector, from
  CT-KN2-02: the Cayley rotation omega = (2,0,0) at cap 0 reads heights (0,0), lies outside K+, and its domain ID
  differs from the identity's.

## 2. Session store

- **JI.17 [B]** The jumbling part is optional. A never-jumbled session stores exactly the Full bytes it would store
  without jumbling support. An episode stores its start derivation, identity pins, exact pose journal and ending
  witness or label certificate plus exit poses. Exact numbers are lossless; PS1 owns canonical number encoding.
  A state delta carries a complete pose-ID remap or full next map, as KN4.8 requires. Preferences and the recovery
  checkpoint survive transactional failures. Path B still imports no jumbling state from 0.4.
- **JI.18 [D]** Choose **flat expanded ia16 storage with optional named span annotations** for PS1.20, following
  H10. It gives KN4.7's journal digest one literal sequence, avoids a replay dependency graph and streams the long
  recorded words without materialising all letters. It costs more stored bytes than shared references. A language
  straight-line program may still expand lazily into this carrier; its exact big-integer length is computed before
  expansion. Changing the carrier never changes letters, chronological prefixes or protection semantics. The
  word encoding belongs to the language-neutral data core (owner, 6 November 2026, LQ10). The chunk encoding is an
  integrator default within that scope (KN4.7), and nothing is frozen yet.
- **JI.19 [D]** The payload digest profile is `ia16-expanded-le/1`: for pinned I_a menu letters only, encode
  `60*grip + item` as unsigned little-endian 16-bit values in chronological order (`0 <= grip < 600`,
  `0 <= item < 60`). SHA-256 covers the concatenation of all uncompressed payload bytes, with no header, newline,
  padding, chunk boundary or gzip metadata. This consumes PS3.13's ia16 meaning. It never decodes retained signed
  generator IDs. A different alphabet needs its own declared carrier, not an inferred interpretation of integers.
  Literal identity is the alphabet, IdentityBlock, big-integer expanded length and this payload digest together;
  equal payload bytes under different menu orders are not the same word. The enclosing canonical metadata and
  compression profile are p3's, not a second JSON or gzip profile here.
- **JI.20 [D]** A stored word has PS1.18's fields, an ordered list of digest-pinned chunk references and total
  expanded length. Chunk references include letter count and uncompressed digest; the sum must equal the declared
  length and the decoded size must equal twice that length. The payload digest is independent of chunking. Missing
  content returns `PS1:input.persistence_format`; changed bytes return `PS1:input.digest_mismatch`; noncanonical
  codes or counts return `PS1:input.noncanonical_value`. Check identities and the complete dependency closure
  before publishing a stored word. The execution dependencies are the pinned alphabet/model and literal chunks;
  origin, provenance and annotations keep their own content references without supplying substitute letters.
- **JI.21 [D]** Named annotations give unique IDs and half-open spans over the flat payload. For `same_as` and
  `inverse_of`, FP.13's shared `relation.word` form is exactly one tagged target: `{"local": <annotation id>}`,
  `{"part": <part id>}` or `{"ref": Ref}` (H15.13). A local target must be an earlier annotation in the same
  ordered annotation list. A part target resolves an immutable digest-pinned included part definition; a ref
  target resolves the included immutable word record by its content reference. Parts and referenced words must
  be present in the dependency closure, in dependency order, before the referring annotation is admitted.
  Reject missing targets, forward local targets, self-reference and cycles before storage; never fetch during
  replay. Verify the relation against the target's literal sequence and the actual annotated span, with matching
  alphabet and identity pins. `inverse_of` reverses the target letters and inverts each by the pinned menu.
  Neither names nor relations generate replay letters or replace the flat payload. Editing an annotation changes
  metadata identity, not literal execution identity. PS1.19 still checks every declared relation.
- **JI.22 [D]** `expand_stored_word` (PS1.24's signature) opens an immutable `LiteralWordStream` only after identity,
  closure, length and budget admission. `read_word_chunk(stream, max_letters)` returns the next contiguous
  half-open range and bytes, at most `max_letters`, which must be positive and within `LimitsRef`; the consumer
  never splits a 16-bit letter. `close_word(stream)` releases its read ownership; EOF is at exactly the declared
  length. Final payload-digest verification precedes publishing a successful check or ready preview. Corruption or
  early EOF returns `PS1:input.digest_mismatch` and discards the scratch result. Cancellation is checked between
  chunks and has no partial commit. One
  word remains one KN4.7 commit even across many chunks. Storage limits and X3's execution bound are distinct;
  crossing a storage limit returns `PS1:resource_limit.persistence_size`, and exceeding admitted expansion returns
  `PS1:resource_limit.word_expansion` (or `KN2:resource_limit.word_length_bound` at replay admission), each with the
  actual big-integer length. No mathematical length cap, numeric X3 bound, elapsed-time
  estimate or silent subdivision is supplied here.
- **JI.23 [D]** A byte vector uses spans A=`[(0,12)]`, B=`same_as A`, C=`inverse_of A`. The flat payload is
  `[(0,12),(0,12),(0,45)]`, length 3, hex `0c000c002d00`, SHA-256
  `c2ae6cf7df4e002e34b55017aa22a79d7b385830a0cf513f170586410380d309`.
  In annotation order A, B, C the spans are `[0,1)`, `[1,2)`, `[2,3)`. A has no relation;
  B has `relation={kind:same_as,word:{"local":"A"}}` and C has
  `relation={kind:inverse_of,word:{"local":"A"}}`. Both relations resolve to A's literal item 12.
  Chunks of one and two letters give the same literal digest as one chunk of three. Replacing C by item 12 fails
  the inverse annotation check; changing B's target to a missing ID fails closure. Making A target C is a forward
  local reference and is refused, including a cycle A to C to A. A missing pinned part or ref target likewise
  refuses annotation admission. This is an encoding example, not evidence of replay at an arbitrary state.
- **JI.24 [D]** Source scale for this choice is the stage 18 lattice construction recorded in
  `research/jumbling/restoration/18-tail-completion/results/lattice-d1/run/run.json`: its 53 lattice words total
  13,231,266 letters. Step index 14 (`lattice/orbit-10`, `words/00014.ia16.gz`) contains 2,352,984 letters,
  hence 4,705,968 uncompressed ia16 bytes by arithmetic. These are that run's recorded literal lattice phase,
  not a general bound, a measured human phase, or a product timing. Other EH-23 construction lengths are not
  reused as limits here. Human lattice-phase time and actions remain unmeasured in this packet.

### Exit records and history

- **JI.25 [D]** An `ExitRecord` binds an exit ID to its `exit` crossing event, episode, exact exit pose content and
  digest, label digest, A5 certificate, source journal derivation and identity/order/contract pins. The certificate
  includes nu = 0, C5 convention, frame agreement and certificate-program digest as A5 requires. H11 defines
  Position, Context and the transition shape; FP.30 to FP.32 encode them. An exit records its own before and after
  Position. A separate `entry` records both of its own sides and its entrance binding:
  `exit_root(exit, crossing, suffix digest)` or `solved_witness(word)`. The suffix is the chronological literal
  retained path in effect at the selected head after undo/redo; its digest pins that exact path's word. When no
  active exit applies, the solved witness is a complete retained word from solved whose replay agrees with current
  labels. No witness is generated from labels. The new episode retains this entrance derivation, including an exit-root binding's
  exact exit and crossing, rather than selecting a root by label digest.
- **JI.26 [D]** Validity is a function of the selected parent path and durable void facts, never labels. An exit
  is active exactly when its crossing is on that path, the suffix from crossing to head holds only retained moves,
  and no void fact names it. These are FP.31's `retained_move` actions; a `jumble_move` cannot supply an exit-root
  suffix, even when its word uses retained letters at pose level. Otherwise the exit is inactive or void;
  inactivity is not deletion. Workspace actions leave
  the puzzle path unchanged and cannot break that retained suffix. A reset, import or checkpoint restore writes
  `void_exit` facts for every earlier exit in the session lineage, active or inactive, with the break event and
  reason in the same transaction. Voids are monotone durable facts, outside the undoable Position and Context.
  Undoing the break, selecting an old branch or returning to equal labels never removes them. A new witnessed
  entry and later exit may create a new exit ID. This is H11's conservative refinement of A5, not permission to
  lift labels or restore an old exit's authority.
- **JI.27 [D]** Every durable transition uses H11's common action chain, distinct from the parent-linked puzzle
  journal. It records its action tag, sequence, previous transition, actual before/after Position and Context,
  event/target and exit/entrance/void references. Its sequence is LI.9's action-order number, increasing by one
  across both action classes. It also carries `command: CommandId`, `request: Digest`,
  `admission: Ref`, `receipt: Ref` and the new `RevisionRef`. Accepted commit and workspace invocations durably
  bind their admission to the command and request (KN5.21, KN5.23). One transaction writes the validated change,
  transition and terminal receipt. Admission without a committed change follows LI.30's recovery rules.
  Each transition's before and context_before equal the preceding transition's after and context_after; undo
  appends a transition and never rewrites that chain. Its recorded sides describe the actual invocation, not an
  assignment of an earlier transaction's entire context. Changed counters advance under KN0.2 even when content
  digests recur. Workspace-only actions and their inverses have equal Position and unchanged state revision.
  Crossings, branch navigation and journal undo/redo restore Position fields only: head, mode, episode and exact
  state at its level. Current preferences and protection stay (U1-015, dispositions S1, solving-workflow 4.2).
  The sole crossing exception is a protection change required at the new level: record its changed fields and
  inverse, validate it through the normal protection path, and reverse/reapply only that change with the crossing.
  Unrelated current protection and preferences never come from a historical Context. A workspace action changes
  only its recorded scope. Reset/import preserve current preferences; restore applies only its validated explicit
  context scope, never incidental context from an exit. All actual context changes have changed-record references
  and fresh versions. A refused action writes no transition, and a durable success stays successful after refresh
  failure. The rows below state field restoration as well as lineage; they are not a second wire schema.

| Action | Valid replay root and resulting mode/head | Position and Context fields restored or changed | Required durable evidence | Refusal or fallback |
| --- | --- | --- | --- | --- |
| Exit, `jumble.off` (`exit`) | Off; new crossing e; new E rooted at current exact X; suffix empty | New head, off mode, episode's exit and projected label state; current Context except a recorded protection-level change under JI.27 | Reachable journal to X, complete A5 certificate, exact X, labels, both exit sides and common JI.27 fields | N > 0: `KN5:input.jumble_off_pieces_off_lattice`; nu != 0: `KN5:input.jumble_off_nu_nonzero`; absent/wrong chain or displaced centre: `KN5:input.jumble_off_not_shown_reachable` |
| Retained move after exit | Off; new move event; same E and X; append literal word to effective suffix | Head and label state change; mode, episode and current Context stay | Parent link, exact retained letters, before/after label digests and common transition/receipt | Refuse stale/protected/invalid move; label replacement instead uses the break rows |
| Mechanical word while Jumble is on (`jumble_move`, FP.31) | On; new pose head in the same episode; no retained exit suffix | Head and exact poses change; episode and current Context stay | Parent link, exact literal word, pose digests and common transition/admission/receipt; includes `jumble.twist.menu` and any word commit at pose level | Normal admissibility, stale and protection gates; never tag it as a retained off-mode move |
| Undo Jumble move | On; head and exact poses before the move, in its recorded episode | Restore the move's before Position with a new state revision; current Context stays | ActionHint, target transition, exact inverse replay and pose digests | Changed hint or missing replay link is refused; checkpoint save supplies no inverse of its own |
| Redo Jumble move | On; move's saved head and exact poses, in its recorded episode | Restore the move's after Position with a new state revision; current Context stays | ActionHint, original literal word, target chain and forward replay digests | Changed target or failed current protection/admissibility gate is refused |
| Undo retained move | Off; head before that move; same E and X; remove that move from suffix | Restore target's before Position with a new state revision; current Context stays | ActionHint, target links, exact inverse replay and digests | `KN5:stale.undo_superseded` for a changed hint; no fallback to labels for a missing replay link |
| Redo retained move | Off; saved move's head; same E and X; restore original chronological suffix | Restore target's after Position with a new state revision; current Context stays | ActionHint, saved word, chain to target, replay digests | Missing or changed target is refused; matching labels never select an exit |
| Undo the exit crossing | On; pre-exit head and original episode at exact X; E inactive | Restore that exit transition's before Position; keep current Context except reversing its recorded protection-level change | Exit's exact before/after, target links and a new inverse receipt/revision | Missing exact content enters read-only recovery; never lift labels |
| Redo the exit crossing | Off; that crossing's head; same unvoided E and X; suffix empty | Restore that exit transition's after Position; keep current Context except reapplying its recorded protection-level change | Same exit identity, certificate, forward validation and new action/receipt | Voided E: `KN5:input.exit_record_voided`; a fresh validated exit needs a new crossing |
| Enter again, `jumble.on` (`entry`, exit root) | On; new entry head/episode at replay(X,w), where w is the effective suffix | Record actual off before and on after Position; current Context except recorded protection-level change | Active E, complete retained path, `exit_root(E,e,digest(w))`, label agreement and exact resulting poses | With no valid E use the solved-witness entry row; without either binding, `KN5:input.jumble_on_labels_only` |
| Entry from solved witness (`entry`) | When no active exit applies, on at new entry head/episode and replay(solved,v); no exit required | Record actual off before and on after Position, including mode/episode/head; current Context except recorded protection-level change | `solved_witness(v)`, complete retained v, pins, replay agreement with current labels and exact resulting poses | No valid retained witness: `KN5:input.jumble_on_labels_only`; do not construct v from labels |
| Undo/redo that entry | Undo returns to its exact off pre-entry state and binding; redo returns to its recorded entrance replay result | Undo restores the entry's own before Position; redo its own after Position; never an exit's sides. Current Context stays except reversing/reapplying that entry's protection change | Both entry sides, entrance derivation, ActionHint and new action/receipt/revision for each inverse/forward step | Redo through a now-void exit: `KN5:input.exit_record_voided`; missing durable evidence enters recovery; no old preview/token repairs it |
| Branch creation or change | Creation at head keeps E; proved journal navigation uses last active unvoided exit on target path; same-exit forks keep X | Creation preserves Position; navigation restores target Position through ordered inverse/forward steps. Keep current Context, except each crossed transition's recorded protection-level change | Common ancestor, ordered journal path, each crossing's own sides/entrance, target digests and one branch action/receipt | Missing path: `KN4:input.branch_path_unproved`; a path that would reactivate a void exit: `KN5:input.exit_record_voided`; checkpoint assignment instead is restore |
| Workspace action | Same root, mode, head, episode and effective suffix | Position before = after; only named workspace/Context records change; state revision stays | `action=workspace`, inverse, provenance, changed-record references/versions and common admission/receipt fields | Normal workspace gates; no hidden puzzle move or replay-root substitution |
| Undo/redo workspace action | Same exit path and root | Position before = after; apply target's scoped inverse/forward edit, advance workspace/affected versions, keep state revision | Target action, changed records, ActionHint and `committed` receipt with equal puzzle hashes (LI.26) | Same scope/confirmation gates as the original operation; obsolete hint refuses |
| Checkpoint save | Mode/head and exit validity unchanged; historical reference only | Position and Context stay; save changes the named checkpoint record | Complete checkpoint/lineage references; `action=workspace` for this non-puzzle effect, no eligible inverse, durable admission and `committed` terminal for the existing commit row (LI.9/LI.10) | Saved reference alone cannot bypass restore |
| Checkpoint restore | New restore action; every earlier exit void, even with equal labels or an E reference; off-state has no active root | Validated target Position with new state revision; only explicitly requested validated context scope, with other current context kept | Complete checkpoint and recovery checkpoint, exact state/lineage, all void facts, context scope and common receipt fields | Later off-to-on entry needs solved witness; labels-only refuses. An on-checkpoint needs its own reachable exact pose journal |
| Import or reset | New replacement action; every earlier exit void; a supplied solved witness can seed later entry | Validated replacement Position (these retained-history rows stay off); keep current preferences and record any explicit validated protection change | Complete input validation before write, source identity/digest, recovery checkpoint, voids and common receipt fields | Imported exit IDs/equal labels cannot restore local authority; invalid input changes nothing; solved reset has empty retained witness |
| Recovery | Last complete transition's Position/Context; recompute active root from selected journal path and all voids; new epoch | Recover actual durable after fields, or preceding fields after proved rollback; never a half-crossed Position or a historical Context chosen by label match | Complete chain with workspace actions, exact entry/exit derivations, admission/terminal bindings and monotone voids | Before durability recover the prior transaction; after durability recover the new one. Missing fate/evidence: `KN5:unknown_outcome.read_only_recovery`, never guessed success |

- **JI.28 [D]** Branch navigation in JI.27 is a journal operation, not checkpoint assignment or replay of workspace
  history. On an off-state fork E then a then undo(a) then b, the root remains E's X and the effective suffix is b.
  If a proved target path ends after E2, it uses E2. Every entry or exit on that navigation uses its own stored
  inverse/forward Position and entrance evidence. Suppose E2 recorded preferences A and protection P0, but current
  preferences are B and protection is P1. If the crossed transitions changed no protection fields, navigation to
  E2 restores its target Position with B and P1, not A and P0. A required crossing-level protection change follows
  only JI.27's exception and gates. The branch action records the resulting actual Context, preserving continuity.
  This draft supplies no branch UI or new branch command.
- **JI.29 [D]** Equal-label counterexample: let reachable exits E1 and E2 store X and Y, with `pi(X) = pi(Y)`
  and different pose digests. Navigation proved to E2 returns Y as replay root, even if E1's label digest matches.
  A retained suffix w entered from E2 produces `replay(Y,w)`, not `replay(X,w)`. Entry records
  `exit_root(E2,crossing2,digest(w))`, its off before Position after w and its on after Position. Undo restores that
  off Position, not E2's pre-exit Y; redo validates and restores the entry's own on Position. Recovery after either
  returns its last complete side. Restoring those same labels after E2 voids it; without a retained witness,
  Jumble on returns `KN5:input.jumble_on_labels_only`. If a retained witness v from solved is present, a new entry
  records `solved_witness(v)` and produces exactly `replay(solved,v)`, without asserting equality to X or Y. Its
  undo/redo/recovery use its own recorded sides too, with no ExitRecord required. A5's label-lift lemma does not
  remove these pose-digest and journal distinctions.
- **JI.30 [D]** Recovery reconstructs the same durable journal, action chain and void facts; it is not a checkpoint
  restore. Restoring a backup as a new session state is. A crash before an import/restore transaction is durable
  leaves old Position/Context and exits; a crash after durability leaves the replacement, actual Context and all
  void facts. Undo of that break cannot erase those facts. For entry, recovery uses the actual entry transition's
  before/after and its exit-root or solved-witness binding, never the referenced exit's sides.
  Durable admissions and terminals remain queryable by command and request after undo and epoch loss (LI.30).
  A proved rollback after durable admission returns `failed(KN5:resource_limit.engine_lost)` without another action;
  missing proof returns `unknown_outcome` and read-only recovery for a possible mutation. Recovery never infers
  outcome or exit validity from a state hash, and it revives no preview or token.
- **JI.31 [B]** The scramble witness is stored separately from solver-authored words and actions. It is available
  to engine replay, independent checking and recovery, but no solving view or query supplies its per-twist data.
  Generic KN4 journal/episode reads respect this separation too; an opaque scramble event is not a public word
  reference. Entrance, transition and receipt queries preserve that restriction when a binding cites a hidden
  witness. A public endpoint observation grants no access to its hidden prefixes. Import and archive permissions
  remain chapter 07's. Corrections selected by the solver retain their construction provenance; research word files
  do not become shipped content through this store choice.

## 3. Command layer

- **JI.32 [D]** H2's delta adds whole rows and amends five existing word rows plus the shared `view.preview.scrub`
  row, with `expect.kind` as the stable precondition. The amendments include full argument schemas under CM.15,
  preview gates and the shared scrub job metadata, as applicable. EN4 owns the final table vocabularies, gates,
  lanes, dispositions, unified undo and builder. The five word-row amendments specify these required mode extensions;
  JI.33 specifies the shared scrub amendment:

| Existing row | EN3 amendment and result |
| --- | --- |
| `macro.add` | Add a pinned menu word or a solver-selected correction to the current draft, preserving alphabet, length, parts and provenance; one workspace action, no execution |
| `macro.save` | Store a pinned menu word by JI.18 to JI.23; saving neither selects, inserts nor executes it |
| `operation.check` | Accept a menu word and optional unit/cap; return JI.11's observation without a commit permission |
| `operation.preview` | Replay the whole menu word at current dependencies into one immutable preview; no protection admission gate, but retain the computed conflict verdict (KN5.9); blocked prefix makes no preview; false step flag does not refuse it |
| `operation.execute` | Commit that presented fresh preview as one journal action, after H3 and full-state/protection checks; use its engine-owned end state, not an imported endpoint |

- **JI.33 [D]** The jumbling rows and shared scrub row below use the same invoke/availability/receipt path as H5 and
  KN5. There is no `not_busy` gate. All commit rows use lane `commit`, require a shell action token and do not repeat.
  A job in an exclusive or latest-wins lane never blocks undo; a state change cancels its revision-bound work. Every result
  below includes JI.4/JI.15 pins. Labels in the JSON are working labels, not UI sign-off.
  The existing `view.preview.scrub` carries jumbling through its preview and position arguments; it is not another
  command or delta addition. Prefix scrubbing runs as a cancellable view job against the immutable preview;
  LI.22/LI.23 resolve its target and cancel a superseded request. It does not synchronously replay a long prefix
  on the UI thread or confer a commit permission.

| Row | Kind / lane / undo | Arguments and result | Boundary and contract |
| --- | --- | --- | --- |
| `jumble.on` | commit / commit / journal | Current revision and action; entered episode and CommitRecord | A5 replay root only, JI.27; no confirmation key |
| `jumble.off` | commit / commit / journal | Current revision and action; E plus CommitRecord | A5 label checkpoint only; no correction advice or confirmation key |
| `jumble.twist.menu` | commit / commit / journal | Resolved absolute cap and menu item with pins; live one-letter CommitRecord | A1/A4, DT-M1 C6, JI.35; not a freely approximated rotation |
| `jumble.twist.append` | workspace / none / workspace | Same resolved letter and explicit draft phase; new draft revision | DT-M1 C7; no live protection gate; one fixed undo class |
| `jumble.survey` | query / exclusive / none | Snapshot; certified grip-status coverage | State section 3; no candidate twist values |
| `jumble.observe` | query / exclusive / none | Snapshot, requested facts and optional named cap/sets; ObservationSet | JI.7; complete facts or explicit incomplete coverage, no method-selected object |
| `jumble.straddlers` | query / exclusive / none | Named cap; full StraddlerSet and u | DT-J1 A2; naming, not hover or prefetch |
| `jumble.certificate.inspect` | query / none / none | Pinned certificate, cap and piece; exact points and bounds | State section 3; already validated result, stale inputs refused |
| `jumble.release.check` | query / latest_wins / none | Released preview letter and optional named cap; release observation | DT-J1 D1; superseded release ends cancelled, no earlier late answer |
| `jumble.covers` | query / exclusive / none | Named cap; completed CoverTable | DT-J1 O3(b), JI.12; both directions without a top verdict |
| `jumble.repair.list` | query / exclusive / none | Named cap; completed RepairTable | 17 October exception, DT-J1 O9 to O11; neutral order, no default selection |
| `jumble.nu` | query / exclusive / none | Snapshot; Partial<NuVector> | A5, JI.13; undefined off lattice |
| `jumble.corrections.check` | query / exclusive / none | Solver-drafted IDs and multiplicities; predicted nu | DT-J1 O6(b); not a combination solver |
| `jumble.correction.lookup` | query / none / none | Explicit construction ID; pinned word and fixed row | DT-J1 O7(a); lookup chooses nothing for the solver; `macro.add` inserts it |
| `jumble.checkpoint.check` | query / exclusive / none | Snapshot and named checkpoint test; Verdict and evidence | A5, LB3.2; separate tests, never a manufactured witness |
| `view.preview.scrub` | view / view / none | Preview ID, literal position; inspected prefix | DT-J1 O10(b), H7; may repeat, does not commit or expose scramble prefixes |
| `jumble.scramble` | workspace / exclusive / workspace | Owner-approved policy reference and explicit action; opaque staged scramble reference | JI.31/JI.38; cancellable construction, no puzzle mutation until normal preview/execute |

- **JI.34 [D]** Existing `session.undo`/`session.redo` perform JI.27's mode-aware transitions. Existing
  `session.checkpoint.save` stores a checked checkpoint and `session.checkpoint.restore` performs the restore row;
  `jumble.checkpoint.check` is their read-side evidence, not a second save. `jumble.off` itself is the handoff to
  the retained workflow; there is no second crossing command. `operation.check` is also the named word-check row;
  no duplicate command is added. Filters use existing `filter.preview`/`filter.apply` with engine results at one
  revision. Repair-row copying resolves a solver-selected result to a pinned word for `macro.add`; the focused
  row or list index alone cannot enter a draft. `job.stop` and preview cancellation keep their existing purposes.
- **JI.35 [D]** `jumble.twist.menu` accepts only a request that resolves to an item of the exact pinned menu table.
  It rejects an off-menu item with `KN2:input.twist_off_menu`; it does not run A1's rational search to approximate any
  of I_a's 48 jumble items. Axis/angle, drag and seat routes all resolve to that table under A6, in force since the
  owner accepted it on 15 November 2026 (KN3.7 to KN3.9): the shell's producer turns a gesture into request bytes, the
  request and its binding `{request, cap, operation, menu}` are named, journalled arguments, and the kernel scores a
  menu drag exactly, with pinned-index ties, and records the winner. Exact free-mode requests belong to research, not
  a new product input row. The twist rows of `command-rows-jumbling.json` still take a resolved letter; they take A6's
  request and binding when the command table freezes at LP3 (PHS.53). With Jumble off, a retained item dispatches to the equivalent retained action;
  a jumble item returns `KN5:input.menu_word_needs_jumble_on`. With Jumble on, every item uses pose classification
  and pose protection.
  `jumble.twist.append` resolves the same letters into a draft; it never inherits live journal undo. No key, seat,
  current-grip name or unpinned item index is persisted as the mathematical argument.
- **JI.36 [D]** Name the four logical seat bindings `jumble.seat.1`, `jumble.seat.2`, `jumble.seat.3`,
  `jumble.seat.4`. They are binding IDs, not four commands or chosen physical key letters. They consume DT-M1's
  Q7(a) form, pending XM1: a pinned geometric representative s for each right coset A4_c*s; tap s alone or combine
  with retained t to resolve t*s, with s first. The binding record pins representative rotations, geometric names,
  transported frame and coset side together. This draft uses DT-M1 D8's construction: at the base cap,
  `s_k = g^k` for k=1 to 4, where g is the exact matrix at item 12 of the identity/order-pinned fixture
  `research/jumbling/fixtures/wj-I_a.json`; store that matrix, not an unpinned index. Transport it using the
  recorded cap frame. Physical key letters and glyph design stay with G6. A held seat changes only the resolved
  input and displayed
  rotation glyph, never a state-derived ranking, snap or highlight. If XM1 reopens Q7 and the owner chooses axis
  then angle or palette/drag alone, only these bindings change; the resolved menu row and stored words remain.
- **JI.37 [B]** An explicit live menu twist has no preview, while a draft turn or word requires a presented fresh
  preview (ONE.78; owner, 6 November 2026, decision 10). Program replacement of a presented preview remains
  ONE.42/KN5.12's design-line question; this draft enables no automatic replacement policy.
- **JI.38 [O]** Product scramble depth and retained/jumble mix remain EH-30's owner choice. No test depth, seed
  family or stage 18 length sets a product default. Until a policy is approved, `jumble.scramble` is unavailable
  with `KN5:input.scramble_policy_unset`. The proposed staging follows the existing staged scramble model: it holds
  an opaque engine-owned word, stores the witness apart, and exposes only permitted endpoint/effect information
  for a whole-operation preview. `operation.execute` commits that preview with the usual token. Scrubbing,
  word-export and general solver journal queries cannot reveal its hidden letters. Alternate approved depths/mixes
  change the policy and cost admission, not the hidden-record or transaction rules. No scramble is generated here.
- **JI.39 [B]** Each row stays inside ENG-0J's boundary. There is no row that lists closing/lowering twists except
  the repair list, ranks caps, marks or prefetches a next cap, chooses a target or unlock letters, or answers whether
  a step exists. Per-revision facts and checks of the solver's own named word remain distinct from permission to
  execute it. A5 refusals name the fact and action kind only. H-05 pointing targets are grips, pieces and certificate
  points with identity and revision, never a screen point used as a mathematical predicate.

## 4. View model and renderer input

- **JI.40 [D]** The view model owns one immutable adopted bundle and a mode, as ONE.59 to ONE.65 specify. It
  switches the Full label path and jumbling pose path only at a committed revision boundary. A header/array
  mismatch triggers a validated refresh; it never mixes revisions. The enclosing binding carries identity,
  menu order, epoch, revision, source digest and source role (authoritative, preview, or derived). This supplies
  pins absent from the older render/1 file header without rewriting that file format. Preview and derived geometry
  retain their names and origins; neither is adopted as the authoritative state. Global and Local share the bundle.
- **JI.41 [D]** The table below covers `research/jumbling/render-contract.md` sections 2 to 7 and ENG-0J's added
  observation payloads. The engine or named library is the producer; the view model only packages validated results.
  Each row requires its typed feature support from the future renderer adapter. `LimitsRef` covers declared producer
  budgets such as canonical output bytes and memory (H8, KN7); exhausting those budgets returns the corresponding
  KN7 budget code with no partial authoritative adoption. Fixed renderer ceilings follow RM.12 and RM.20,
  including `EV3:resource_limit.display_plan_ceiling` and its refusal before acceptance or failure during accepted
  preparation; they are not a KN7 draw-plan budget. Dynamic geometry sizes, timings and tariffs are unmeasured.
  This table chooses no renderer.

| Render feature | Producer and payload | Structural limits, refusal or permitted fallback |
| --- | --- | --- |
| Static geometry and rigid sticker shrink (render 2) | Model assets: facet frames, base triangles and sticker offsets, slot-to-piece map, normals/R; mesh-derived area-weighted triangle centroids | d=4; 600 facets, 433 stickers per facet, 259,800 slots; refuse invalid shapes/digests with `EV3:input.display_bundle_mismatch`. Numbering centres are not shrink anchors |
| Exact state drawn as floats (render 3) | Engine `pose_table`/`pose_ids`: all piece indices, row-major float matrices converted from exact poses, lattice index or -1 per pose; header hashes/shapes | Exactly 177,120 indices and matching table/lattice rows; finite drawing conversions, valid indices and identity at pose 0; refuse mixed tables or nonfinite payload; no hidden pieces removed from state |
| Swept twist motion (render 4) | Engine-certified letter and inside set: grip, plane_u/v, angle, sorted moving IDs and digest, from/to revision or explicit preview origin | Verify source and moving-set pins; a half-turn without an engine-named plane is drawn without a swept preview, as render 4 permits; never invent a direction or legality |
| Grip map (render 5) | Engine `survey`: certified a/b per grip and full coverage | 600 statuses; uncertain entries cannot be encoded as a or b. Incomplete survey remains pending; no invented admissible flag or all-items outcome table |
| Blocked-grip certificates and points (render 5) | Engine full named-cap straddler set: piece IDs, both exact points and converted vectors; cap-map witness remains a witness only | Every point pair checked exactly; full named set obeys producer budgets and fixed renderer ceilings (RM.12/RM.20), with no one-witness truncation; wrong input pins refused |
| Named cut hyperplane (render 5) | Engine cut geometry: pole, exact plane and alpha=121/125, converted display values | Only the named cap; no mesh-derived predicate; malformed plane refused |
| Projection chain (render 6) | Shell's view parameters: ordered rotate, perspective or stereographic, camera3 stages; model frames supplied by engine | d=4, dimension-compatible stages and finite inputs; unsupported stage returns `EV3:input.display_unsupported`; no substitute projection silently selected |
| Revision adoption and readback (render 7) | View model complete bundle; adapter's first-frame bound-array digests and element-check record | Compare the exact revision's three state arrays as render 7 requires; failure leaves display out of date, not a failed commit. Actual backend readback evidence remains renderer work |
| Domain and lattice masks | `heights`, `pose_table`, `pose_ids`: exact domain IDs (JI.53) and per-piece membership, with masks | Each mask exactly 177,120 bits and input digest; no membership inferred from floats or stale pose IDs |
| Rim map and counts | `rim_summary`, `height_counts`, `seam_facts(all)`: N_c/floor, blocked, total N, n2/n4/n6 and above-6 count/maximum | No default sort, cap rank or helpful-twist value; preserve undefined fields, including total N when full coverage is missing |
| Cut-cell and off-grip seam overlays | `seam_facts`: exact cells, domains, occupancies, hyperplanes and certified holding caps, plus derived display geometry | Include off-grip cells; count/byte limits stated before publication; refuse budget overrun, never remove them while claiming a complete N |
| Exposed-face meshes | Exact region/cut-cell geometry producer: face identity, exact-source digest, converted vertices and topology | Size measurement and geometry production are still outstanding in EN2/renderer line. Missing certified geometry returns `KN2:input.exposed_geometry_unavailable`; no fabricated mesh or support claim |
| Named filters and highlight/dim masks | Engine results for caller-named piece/cap criteria: straddlers, covered, lifted, moved sets and mask SHA-256 | Piece masks 177,120 bits, grip masks 600 bits; threshold only when solver supplies it; no u ordering/filter or state-marked seat; mismatch discarded |
| Release/word check data | JI.11 fields and chronological kept checks at the named cap for this revision | Only actual solver releases; newest in-flight release wins. Completed checks kept in tried order, cleared on state change; no score or permission inferred from step flag |
| Covers and repair rows | Completed JI.12 tables; optional masks for the solver's selected row, resolved literal word and full collateral | Coverage must be complete for list answers; empty is cap/input scoped. No row focused by default, no rank, no prefetch and no automatic execution |
| Nu and correction-set observations | JI.13 vector/undefined result and fixed correction rows | Off lattice has no nu values; fixed-coordinate consistency failure cannot be displayed as a state; no suggested combination or state-filtered catalogue |
| Picks for H-05 | Adapter hit record tied to adopted source: grip, piece or certificate-point ID | Stale pick dropped under ONE.13; resolve IDs before invocation. A hit is never an inside/outside certificate |

- **JI.42 [D]** Bundle buffers stay immutable until the renderer acknowledges replacement or device loss releases
  its references. Replacement cancels obsolete view production under H4/H5 and publishes one complete newer
  bundle. Device loss releases device resources and rebuilds them from the same engine bundle; it changes no head,
  pose, label, episode or receipt. Failure of a required feature is an explicit unsupported-feature disposition for
  p6, not a silent omission. The portable adapter signatures, support registry, release acknowledgements, limits
  and device-loss vectors remain p6's part of FR-K05. No backend pass is claimed by these producer rows.
- **JI.43 [D]** This draft uses chapter 10's renderer-independent publication boundary. Further views and user
  stages reach the renderer line as handoff row H-11 (owner, 6 November 2026, LQ6, LQ7, LQ20), and p6 owns the
  adapters and support matrix. Exact producers, identity pins, full-state adoption and refusals above remain. Layout,
  motion timing, look, entry filters and the repair-list presentation wait for the owner to resume DT-J2 to DT-J4.
  No such topic opens here.

## EH answers and evidence limits

- **JI.44 [D]** This is the EN3 answer to every current handoff item (EH-01 to EH-35), including the later updates.
  "Specified" means contained in this draft, not implemented or measured. The handoff log itself is outside p1's files.

| EH | EN3 answer | Work still owned elsewhere |
| --- | --- | --- |
| EH-01 | Four portable layers, JI.4 to JI.43 | EN4 reconciliation and freeze review |
| EH-02 | Interface and delta supplied; JI.32 | EN2 harness, store tests and cost leads |
| EH-03 | Current I_a identity consumed, JI.3 | Final menu sign-off remains owner; EN2 compares fixtures |
| EH-04 | Exact arbitrary-size entries, JI.5 | EN2 H6030 replay; no bounded-height assumption |
| EH-05 | Observation batches and render rows, JI.6 to JI.8, JI.41 | EN2 reference agreement and exposed-face sizing |
| EH-06 | Named release and word checks, JI.6/JI.11 | EN2 reference comparisons and X5 |
| EH-07 | Neutral repair job, JI.12/JI.33 | EN2 all recorded list comparisons, X1 |
| EH-08 | Exact bidirectional covers, JI.12 | EN2 X4 disagreement traces |
| EH-09 | Command IDs supplied; settled list behavior retained | Entry defaults DT-J2 and presentation DT-J4, when owner resumes them |
| EH-10 | Nu defined only on lattice, JI.13 | EN2 nu-coordinate comparisons |
| EH-11 | Fixed 36-construction catalogue with current preview, JI.13 | Product packaging and CI comparison of each correction's row |
| EH-12 | No combination solver; caller-drafted set check only, JI.13 | Closed owner boundary in DT-J1 O6(b), not reopened |
| EH-13 | Stored words and crossing supply interface support, JI.18/JI.27 | Human lattice phase in vertical slice is unmeasured |
| EH-14 | Menu-word storage and existing macro rows, JI.18 to JI.23/JI.32 | EN2 long-word replay and round trip |
| EH-15 | Current whole-word replay and full lifted set, JI.11/JI.32 | EN2 b14-after-b12 negative and admissible-commit case |
| EH-16 | Unit, lifted and unchanged-pose fields, JI.9/JI.11 | EN2 exact reference agreement |
| EH-17 | No existence-test row, JI.39 | Closed by DT-J1 O5(a) |
| EH-18 | A5 crossings and all history actions, JI.25 to JI.30 | p2b command lifecycle, p3 durability, second-wave trace |
| EH-19 | Separate hidden scramble witness, JI.31/JI.38 | EN2 store round trip; product policy is owner item |
| EH-20 | Research is data only; no restart or tutorial acceptance claimed | Historical outcomes remain their source records; EH-35 updates the handoff |
| EH-21 | Interface is now supplied; KN2.4/JI.6 cite the oracle | EN2 harness independence scan and first-difference reporting |
| EH-22 | Flat payload, optional verified named spans, JI.18 to JI.23 | p3 envelope integration, EN2 replay round trip |
| EH-23 | Every lattice figure used here is tied to stage 18's specific record, JI.24 | No historical construction bound used as product or human measurement |
| EH-24 | Current replay, exact pose keys and fresh dependencies, JI.5/JI.11/JI.15 | EN2 b03 and wrong-inverse/frame negative controls |
| EH-25 | Whole-word fields and chronological observations, JI.10/JI.11 | EN2 recorded b20 field comparison; flag is not permission |
| EH-26 | Sparse histogram, fixed counts and named u, JI.6/JI.41 | EN2 six height-4 stop comparisons, X2 |
| EH-27 | No unlock listing, target selection or u ranking, JI.39 | Closed by the settled DT-J1 boundary |
| EH-28 | Covers uses exact whole-piece classification, JI.12 | EN2 X4 traces blocked-d and rounding differences |
| EH-29 | No global bound; I_a reachability may assert right height 0 and left height <= start height + 2 per played letter | EN2 X1 witness check; a sample height does not bound storage |
| EH-30 | Scramble policy intentionally unset, JI.38 | Owner chooses depth and mix |
| EH-31 | Adopted DT-J1 choices consumed, JI.12/JI.13/JI.39/JI.41 | Remaining look/placement stays with paused design line |
| EH-32 | Revision 2 used, JI.1 | EN2 checks and EN3 independent candidate review remain separate |
| EH-33 | Order digest separate from menu identity, JI.3/JI.15/JI.19 | EN2 recomputes both order encodings |
| EH-34 | A5 applied without a label lift, JI.25 to JI.30 | EN2 negative reachability and crossing round trips |
| EH-35 | Archived research data motivates scale only, JI.24 | No research-derived solving feature, tutorial approval or research restart |

## Errors and conformance obligations

- **JI.45 [D]** H15.16 names the raising component in each code; JI clauses remain the rule sources. The two EV3
  codes already exist; the other six are proposals for p7-reg's registry update. All eight have category `input`,
  not `unverified_reference`: none asks for a missing disambiguating frame. ERR.8's envelope is shared, not redefined
  here. At admission these are `form=refusal`, `recoverable=true`, `retry=after_user_change`; an error discovered
  after acceptance keeps its code and context through LI.7/LI.28. Every input error identifies the failed fact
  and retained draft, or explicit absence of a draft. No continuation retries automatically. KN7 codes still
  apply to budget/cancellation; undefined observation fields remain typed results, not errors or false booleans.

| Code | Raising component and rule source | Exact trigger and required context | Continuation |
| --- | --- | --- | --- |
| `KN1:input.unsupported_dimension` | KN1 model registry; JI.3 | Header/request dimension differs from 4; supplied and supported dimension, identity/header reference | Correct the dimension before arrays are read; no state change |
| `KN2:input.named_object_invalid` | KN2 engine validation; JI.9 | Missing/out-of-range cap/piece or invalid unit; argument name, supplied IDs and failed unit condition | Correct that argument; no alternate object selected |
| `EV3:input.display_bundle_mismatch` | EV3 display adapter; JI.41 | Shape, digest, index, finite-conversion or source-binding failure; bundle/source reference and failed field | Refuse the bundle, retain the last valid view and request validated refresh |
| `EV3:input.display_unsupported` | EV3 display adapter; JI.41 | Unsupported or dimension-incompatible projection stage; stage ID, dimensions and supported feature declaration | Keep the last valid view; require a supported explicit chain |
| `KN5:input.scramble_policy_unset` | KN5 command availability; JI.38 | No owner-approved product policy reference; row and missing policy fact | Keep command unavailable; await owner's policy |
| `KN4:input.branch_path_unproved` | KN4 store navigation; JI.28 | No proved journal navigation path; current/target heads and missing or unproved path link, not a missing frame | Keep current head/context/root; supply a proved path or explicitly request a validated restore, which voids old exits |
| `KN5:input.exit_record_voided` | KN5 command reactivation check; JI.26/JI.27 | Target would reactivate a void exit; exit ID, crossing, target action and durable void/break reference | Refuse redo/navigation; a fresh validated crossing or solved-witness entry is a separate action; never clear the void |
| `KN2:input.exposed_geometry_unavailable` | KN2 exact geometry producer; JI.41 | Certified geometry or admitted size profile is absent; named feature, exact input reference and which prerequisite is missing | Report feature unavailable; request it only after certified data/profile exists, or explicitly choose a supported feature; invent no mesh or size |

- **JI.46 [D]** These are new requirement rows, not passing tests. The manifest vectors are source-level contract
  examples. Chapter 12 lists these rows under JI. Each runs in the first phase that builds every component it names:
  CT-JI-03 with M5 of LP2, and the others at LP3, where the command table freezes (PHS.53). The draft named LP1, which
  the owner signed on 15 November 2026 without these rows.

| ID | Phase | Acceptance obligation | Class |
| --- | --- | --- | --- |
| CT-JI-01 | LP3 | Exit, retained move, undo, redo, branch navigation, restore/import and recovery preserve exactly the roots/void facts of JI.27, including unequal poses with equal labels | source |
| CT-JI-02 | LP3 | ia16 chunking and named inverse/same-as spans preserve JI.23's bytes/digest; missing, cyclic or unequal parts are refused; exact length precedes admission | source |
| CT-JI-03 | LP2 (M5) | All-scope seam counts include off-grip cells; JI.16's literal prefix observations and unit results agree with exact replay; cancellation yields only covered prefixes | source |
| CT-JI-04 | LP3 | H2 delta joins once per ID; all five stable-kind amendments apply; no forbidden query or repeating state/draft change exists | source |
| CT-JI-05 | LP3 | Every JI.41 feature has a producer, validated payload and explicit size/support refusal; mismatched revision arrays never form a bundle | source |
| CT-JI-06 | LP3 | Live and draft menu routes resolve all intended exact items under pins; wrong order is refused; alternate keymaps replay the same letters; Q7 remains conditional on XM1 | source |
| CT-JI-07 | LP3 | No solving view, generic journal query or `view.preview.scrub` leaks per-twist scramble data; the shared row carries jumbling by arguments; staged scramble cannot commit without a fresh presented preview and token | source |
| CT-JI-08 | LP3 | Both entry bindings in JI.25 survive entry/undo/redo/recovery using the entry's own sides; branch navigation preserves current preferences/protection except recorded crossing-level changes | source |
| CT-JI-09 | LP3 | Exit, protection workspace edit, retained move and both inverse classes form one continuous action chain; monotone voids survive undo and branch selection; original outcome lookup survives undo/restart | source |
| CT-JI-10 | LP3 | JI.23 local relations resolve earlier A; part/ref targets require closed pinned inputs; missing targets, forward references, cycles and changed inverses refuse annotation admission | source |
| CT-JI-11 | LP3 | Branch-path, void-exit and exposed-geometry envelopes use JI.45's component, input category, typed facts and continuation; no missing-frame remedy is invented | source |

## Open decisions and integration

- **JI.47 [O]** The owner retains menu sign-off at the freeze. This draft assumes the current I_a pins already
  used by ENG-0J; a different approved menu requires versioned identity/order, carrier/input conventions and new
  affected evidence, not relabelling existing words. EH-30, DT-M1 Q7 pending XM1 and the paused design
  questions remain owner items as JI.36, JI.38 and JI.43 state. No change to A5 is needed by this draft. Allowing
  labels-only entry or reviving an exit through restore would change A5 and requires an owner decision; both stay refused.
- **JI.48 [O]** Built-in rows retain DT-J1's interim boundary and the repair/correction exceptions already
  recorded. Language callers still pass chapter 07's profile and author-class checks. LQ21 is answered with
  capability separation (owner, 14 November 2026): a previewing solving program cannot read those observations,
  including refusal details. H9 supplies `profile:none` only for manual/imported
  no-run records and grants nothing. If LQ2's joint setting changes callable tiers or sinks, update the policy
  table; these signatures grant no new solving permission. The research library ships in 1.0 (owner, 6 November
  2026, LQ16). Exposure of the research profile stays a joint-setting question (PHS.58), and the research walk
  remains a conformance input, not a shipping decision.
- **JI.49 [B]** Engine journal branches stand, and a pinned job finishes on its pinned input (JI.16, JI.28; owner, 6
  November 2026, LQ4). Void facts, snapshot immutability and labels-only refusal stay.
- **JI.50 [D]** Open engineering work: p2a builds the final H-08 table with the real delta; p2b supplies its final
  availability, receipts, lanes and history actions; p3 supplies canonical envelopes, chunk framing/limits and
  durable exit/void encoding; p6 supplies final adapter support and resource lifetimes. The second wave performs
  H13's traces, finishes the FR-B03 language-kernel annex (LQ8 and LQ9 are answered, owner, 14 November 2026), and
  reconciles every public error and
  requirement. Libraries above the kernel are written in the language (owner, 6 November 2026, LQ5). These typed
  results do not change, and the principle/decomposition contract still applies. EN2 owns oracle agreement, X1 to X5, XM1 and mesh
  sizing. No missing measurement or ABI detail is replaced by a default tariff in this draft.
- **JI.51 [D]** Mechanical acceptance for p1 is
  `python docs/architecture/1.0/language/closure/check_closure.py --packet p1`.
  After the other drafts land, the integrator must run
  `python docs/progress/1.0/build_command_table.py`,
  `python -B docs/progress/1.0/test_command_table.py`, and
  `python docs/architecture/1.0/language/closure/check_closure.py --all` on the combined candidate. The final table
  must use this delta without `--draft-without-jumbling`. ENG-0J also requires the finished EN3 candidate's Astra
  review in at least two concurrent shards, coordinated with Claude's review of these Codex-authored changes,
  through `python tools/agents/codex_review.py --kind review --packet <scoped-packet> --model gpt-6-astra
  --effort max --speed fast`. The integrator supplies the review packets and dispositions. Under `AGENTS.md`
  "Codex minimum" (owner, 14 November 2026) a fresh Claude read replaces that review unless the owner asks for the
  Astra call; JI.54 records the read for M5. No review, integration check, runtime test, Windows/DirectX result or
  owner freeze is claimed by the closure check.
- **JI.54 [D]** LP2's M5 consumes the engine half: JI.1 to JI.16, JI.52, JI.53 and JI.45's codes
  `KN1:input.unsupported_dimension` and `KN2:input.named_object_invalid`; CT-JI-03 is among its tests. The EH answers
  of JI.44 bind only through the clauses they cite. The session-store half (JI.17 to JI.31) stays a draft for the
  jumbling episode store, bound where it consumes the frozen format packet. The command layer, the view model and
  `command-rows-jumbling.json` (JI.32 to JI.43) stay drafts until LP3 and LP4 (PHS.53). M5's pull request records
  what this draft leaves to it: what a Net-protection refusal reports, the identity blocks of the S4 and I_b
  fixtures, the batch budgets and chunk sizes, the internal free-mode entry, and whether JI.16's vector holds as
  written. On 16 November 2026 a fresh Claude read of this draft for M5's start found four major problems: the
  contract pin, A6 in JI.35, the public J1 API and the domain ID. JI.3, JI.35, JI.52 and JI.53 fix them. The read's
  minor points are fixed in JI.1, JI.6, JI.14, JI.46, JI.48, JI.50 and JI.51, except three that are deferred:
  JI.18's "nothing is frozen yet" and the twist rows and live-preview alternative of `command-rows-jumbling.json`
  go to LP3, and the relationship-strip fields of the view model go to LP4.

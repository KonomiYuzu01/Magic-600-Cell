# Chapter 01: the kernel

Status: **draft for the owner. Not accepted.** Conventions are in [the overview](00-overview.md), section 3.

The kernel is the lower half of the charter's six layers: the engine (KN1, KN2, KN3, KN6, KN7), the session store (KN4)
and the command layer (KN5). This chapter states what the language needs from it. It is the contract that LP1's kernel
interface specification makes final. The command table's final form belongs to EN4, and the jumbling rows to EN3; section
9 lists what this chapter asks of them.

Sources: [command table H-08](../../../../progress/1.0/command-table.md), [requirements R-01 to
R-19](../../../../progress/1.0/requirements-from-screening.md), [ENG-0J revision
2](../../../../progress/1.0/packets/engineering/ENG-0J-jumbling-backend.md), [state contract A1 to
A5](../../../../../research/jumbling/state-contract.md), [DT-M1](../../../../progress/1.0/design-table/topics/command-keyboard-model.md),
[DT-J1](../../../../progress/1.0/design-table/topics/jumble-solver-aids.md) and
[`10_V1_ARCHITECTURE.md`](../../10_V1_ARCHITECTURE.md) sections V03 to V06, as proposals where that document marks them so.

## 1. Shared vocabulary

- **KN0.1 [B]** Every value that crosses the kernel boundary names what it is about. Programs, words, records and
  evidence carry the model identity (INV-7). In this chapter that is the *identity block*:

  ```text
  IdentityBlock = {
    puzzle:       full | jumbling
    model_id:     Digest          # assets/manifest.json; both puzzles share the retained geometry
    menu_id:      Digest?         # jumbling only: the order-free menu identity
    menu_order:   Digest?         # jumbling only: the menu-order digest
    contract_rev: Text?           # jumbling only: the state contract revision
  }
  ```

- **KN0.2 [D]** A *revision reference* binds a result to one moment of one session. It extends the proposal of V03.3 with
  the three revisions that H-08 names. EN4 fixes its final fields.

  ```text
  RevisionRef = {
    session:     SessionId
    epoch:       EngineEpoch      # distinguishes process lifetimes
    identity:    IdentityBlock
    mode:        jumble_off | jumble_on
    head:        EventId
    state:       Nat              # state revision
    workspace:   Nat              # workspace revision
    protection:  Nat              # protection revision
    digest:      Digest<level>    # labels with Jumble off, poses with Jumble on
  }
  ```

- **KN0.3 [D]** A *state reference* is a typed choice between a revision of a session and a derived state:
  `StateRef = Snapshot | DerivedRef`. Every read call takes a `StateRef`, so a program never holds raw mutable state.
- **KN0.4 [D]** A call that can be undefined on part of its input returns a *typed partial result*:
  `Partial<T> = total(T) | partial(covered: T, missing: Missing)`, or `undefined(reason)` when nothing is defined. A
  sentinel value is never used. "None: off the lattice" and "unknown" are different values (DT-L1 R2-E4).
- **KN0.5 [B]** A decision that can lack evidence returns a verdict, never a boolean: `yes(evidence)`, `no(evidence)` or
  `unknown(reason)`. Unknown is never false (INV-5), and it is a property of a result, never an error category (V06).
- **KN0.6 [B]** Every result names the revision or derived state it read. Old previews, async results and delayed
  picking results are never applied to a mismatched state (V03.3).
- **KN0.7 [D]** The kernel never calls the language (partition section 2). No kernel operation takes a callback, and no
  table the kernel reads holds code. What the language needs at run time, it asks for.

## 2. KN1: model registry and identity

### Identities

- **KN1.1 [B]** The registry holds the 600-cell-Full identity (the `model_id` of `assets/manifest.json`) and, for each
  signed menu, the jumbling identity: the menu identity, the menu-order digest and the contract revision (ENG-0J section
  4). A session always records the 600-cell-Full identity. While Jumble is on it also records the rest.
- **KN1.2 [B]** Both puzzles read the retained geometry from `assets/` unchanged. A geometry change gives both puzzles new
  identities (ENG-0J section 4) and needs a migration (`AGENTS.md`).
- **KN1.3 [B]** The menu identity ignores item order, while stored (grip, index) letters depend on it. The reversed I_a
  record keeps the identity and moves all 60 indices (DT-J1 fix-first 1). So a menu letter is meaningful only under its
  menu-order digest, and every stored word, list result and job result cites both.
- **KN1.4 [B]** On an identity mismatch, a value is refused or marked as recorded. It is never reinterpreted (INV-7).

```text
operation model_info(identity: IdentityBlock) -> ModelInfo
  effect:  pure
  errors:  KN1:input.unknown_model
```

### Canonical IDs

- **KN1.5 [B]** Objects are addressed by canonical IDs bound to a model and a level (INV-6). Screen positions, pose-table
  indices and display instances are never identities.

| Object | Canonical form | Puzzle | Source and note |
| --- | --- | --- | --- |
| Cell, grip | The index in the model files; a grip is the pole of one cell | both | State contract section 1: grips are world-fixed |
| Cap | A grip together with its cut | both | Membership of a cap is a fact of a state, not of the ID |
| Generator | A signed integer, ±1 to ±1200 | 600-cell-Full | A negative number is the inverse |
| Menu letter | (grip, item index), under the identity block | jumbling | State contract A4; ENG-0J section 4 |
| Piece | The index among the 177,120 retained pieces | both | State contract section 2 |
| Sticker | (A, f): the piece and its home host facet | both | State contract A3 |
| Slot | The index among the 259,800 labelled slots | both | Position of one sticker on the lattice |
| Position | Derived: the chamber a piece occupies | both | Defined on the lattice only in the jumbling puzzle |
| Pose | The exact key of a 4×4 rotation over Q(√5) | jumbling | Compared by exact key, never by a pose-table index of another revision (ENG-0J) |
| Orbit | The index among the 600-cell-Full orbits | 600-cell-Full | |

- **KN1.6 [B]** Equality of two jumbling configurations is exact pose equality of every labelled piece, with no symmetry
  quotient (state contract section 2). Label equality is weaker (A3).

### The identity tower

- **KN1.7 [D]** Each puzzle has identity levels, joined by exact maps with named kernels (DT-L1 M3, R2-E2, note N1). The
  maps go one way, from a finer level to a coarser one. The name "tower" stays, but the levels form a partial order, not
  a chain: pose → sticker class → region class → {coset → tree data and heights; shape}. Labels stand beside the sticker
  class and position beside the region class, on the lattice.

  The definitions come from note N1:

  - R_A is the set of s in SO(4) with s(Ā₀) = Ā₀. S_A is the set of s in R_A that map each sticker region of A onto
    itself. S_A is normal in R_A, and R_A/S_A is the **frame group** of A. This chapter's first draft called S_A
    "Stab_A" and the sticker class "visible pose class". S_A must not be read as R_A: with R_A, 39,530 labels of a solved
    state differ.
  - A **gauge** is a pair γ = (σ, t), where σ permutes the pieces and t_A in SO(4) carries Ā₀ onto the reference region
    of σA. It acts on the right: (X·γ)[A] = X[σA]·t_A. 𝒢 is the group of all gauges. 𝒢_R is its subgroup with σ = id and
    t_A in R_A, and 𝒢_S the subgroup with σ = id and t_A in S_A.
  - **Shape** counts multiplicity. Note N1 writes the set of posed regions, and on a collision-free configuration the
    two agree. A twist keeps a configuration collision-free (state contract section 3, "No collisions"), but a raw table
    can collide (KN2.14). Then the set forgets a count: for three pieces with congruent home regions, R a lattice region
    and QR an off-lattice copy, the tables (R, R, QR) and (R, QR, QR) give one set, different off counts and no gauge
    between them. Equal multisets always give a gauge: pick σ that matches the regions, and t_A = X[σA]⁻¹·Y[A] carries
    Ā₀ onto the reference region of σA, so Y = X·γ.

| Level | Jumbling puzzle | Equal when | Defined | 600-cell-Full |
| --- | --- | --- | --- | --- |
| Pose | g_A for every piece: the state of record | X = Y | Always | Not held |
| Sticker class | g_A·S_A | Y = X·γ with γ in 𝒢_S | Always | Not held |
| Labels | π of state contract A3 | As the sticker class (N1, Corollary 3) | On the lattice only | The state of record |
| Region class | g_A·R_A, which is A's posed region | Y = X·γ with γ in 𝒢_R | Always | Not held |
| Position | The chamber, derived from labels | As the region class | On the lattice only | Derived from labels |
| Coset | g_A·K⁺ | Equal cosets; a quotient, not a gauge | Always | Not held |
| Tree data, heights | Functions of the coset | Equal values; a quotient, not a gauge | Always; they determine the coset and the lattice test only on Γ(Λ) | Not held |
| Shape | The multiset of posed regions | Y = X·γ with γ in 𝒢 | Always | Not held |

| Map | Defined on | Kernel | What it preserves |
| --- | --- | --- | --- |
| Pose to sticker class | Every pose | S_A | Labelled posed stickers at every later prefix, and everything at a coarser level (KN1.15) |
| Sticker class to labels | Lattice configurations only | None (N1, Corollary 3) | ν, the sticker test. A3's orientation is pose-level, and labels do not read it |
| Sticker class to region class | Every class | The frame group R_A/S_A | Each piece's posed region |
| Labels to position | Every labelled state | The frame group R_A/S_A | Which piece sits where |
| Region class to coset | Every class | The place within the domain | The domain. No transfer: two states with equal cosets can have different shapes |
| Coset to tree data and heights | Every coset | Depends on the menu: an injection only on Γ(Λ) | Tree positions, heights |
| Region class to shape | Every class | Which piece fills which region | Occupied regions, admissibility of every word, N, Φ (KN1.15) |

- **KN1.8 [D]** Digests name their level. `Digest<labels>` is the digest of the int32 label array, the existing
  `state_hash`. `Digest<poses>` is J1's digest over poses. KN1 normalises the byte conventions once (default 4). A
  digest of one level never compares with a digest of another. Off the lattice, pose is the only stored level. A
  sticker-class digest, the least exact key in g_A·S_A with at most 12 products per piece, is optional and does not exist
  yet.
- **KN1.9 [D]** A projection to a coarser level is an explicit call. Off the lattice it returns a typed partial result
  that says how many pieces are off:

  ```text
  operation project(state: StateRef, to: Level) -> Partial<LevelValue>
    effect:  read a snapshot
    cost:    linear in pieces
    errors:  KN1:input.level_not_held
  ```

- **KN1.10 [D]** No operation lifts a value from a coarser level to a finer one. The pose digest of a lift is not
  determined (ENG-0J section 7).
- **KN1.11 [D]** Predicates, goals and protection checks are typed by the level they read (LG2). Protection is two
  predicates. Inside Jumble the verdict is pose-level: m_A(w) = 1. In 600-cell-Full it is label-level, which on a lattice
  configuration reads X[A]⁻¹·m_A(w)·X[A] ∈ S_A. Both are invariant under 𝒢_R, because S_A is normal in R_A (N1, B.5). A
  verdict carries its level and never crosses over (A3, A5, DT-L1 M3). A comparison with a stored literal pose is
  pose-level only.

### Derived models and claim transport

- **KN1.12 [B]** A derived model (a quotient, a restriction, a simplified model) is a value with a derivation. It is never
  a new authoritative puzzle (INV-7).
- **KN1.13 [D]** The registry records a derived model's map and the claims the map preserves. A claim moves to or from it
  only along a map that preserves the claim's subject. For example, Lemma 13 of the retained theory needs cost-preserving
  automorphisms that keep the goal, the protection and the data that lifts a path, and the family maps Γ(Y) of stage 19
  transfer legality and shape-level claims (N1, B.7, which corrects DT-L1 M3's "legality claims only").

### Transfer between classes

- **KN1.14 [D]** Gauge transfer (N1, Theorem 1). Let X be any configuration, on or off the lattice, γ = (σ, t) a gauge,
  Z = X·γ, and classes exact. Then a word w is admissible from X exactly when it is admissible from Z, w(Z) = w(X)·γ, and
  each piece's left multiplier satisfies m_A(w; Z) = m_{σA}(w; X). Two corollaries follow:

  - For γ in 𝒢_S the labelled posed sticker regions of X and Z agree at every later prefix.
  - On lattice configurations, π(X) = π(Y) exactly when X[A] lies in Y[A]·S_A for every piece.

  A5's label-lift lemma is the case σ = id on the lattice. Stage 19's transfer is the same step off the lattice. Note N1
  stands at (theorem, producer) under chapter 06 until a model review reads it, and the engine test is CT-KN2-06.
- **KN1.15 [D]** What reads at which level (N1, B.4). A function has a level when it is constant on that level's classes.
  Piece-indexed functions transform by σ at shape level.

| Reads at this level | Functions |
| --- | --- |
| Pose only | The pose table; the pose digest; the moved count; the orientation index; the journal; `v`; reachability; label and retained checkpoints, through their reachability clause; budgeted `uncertain`; pose groups |
| Sticker class | Labelled posed stickers. On the lattice: labels, ν, the sticker test and the label digest |
| Region class | Each piece's posed region, lattice flag, domain, tree positions and heights; inside sets and straddlers as sets of pieces; left multipliers |
| Shape | Occupied regions with their multiplicity; admissibility of every word; N, N_c, In_c, Out_c, the floor; Φ, q_t, H; the off count; multisets of heights by type; anything defined only from these, such as the repair list |

- **KN1.16 [D]** Exact transport. A claim at a level moves along that level's subgroup with no re-run. The reused leaf is
  the conjunction of the source leaf, Theorem 1 and the checked relation Z = X·γ (N1, B.7; chapter 06). Legality and
  shape-level claims (q, n, Φ, H) transfer. If a goal set is invariant under a subgroup of 𝒢, the distance in letters to
  that goal is constant on the subgroup's orbits (N1, Corollary 4).
- **KN1.17 [D]** What does not transfer. Reachability, and label and retained checkpoints through their reachability
  clause, do not move along any gauge: a solved state with one centre turned by a third-turn of A4₀ passes the sticker
  test and is unreachable in J(A4₀) (N1, B.5). Digest, journal, moved count and orientation can differ inside a class. A
  different class does not mean a later twist separates two states. Only the three gauge groups give transfer: equal
  cosets or equal heights give none. A class is never equality (KN1.6).

## 3. KN2: exact engines

### Snapshots and reads

- **KN2.1 [B]** The engine is the source of truth. Floats serve drawing only (partition section 1.1). The differential
  oracles gate every kernel: the J1 harness in `research/jumbling/differential/` and the 600-cell-Full oracle in
  `tools/oracle/`.
- **KN2.2 [D]** A program reads through a *snapshot*: a read-only handle bound to one revision.

  ```text
  operation open_snapshot(at: RevisionRef | current) -> Snapshot
    effect:  read a snapshot
    cost:    declared from the journal distance to the nearest checkpoint, resident or not (KN7.8)
    errors:  KN2:stale.revision_gone, KN4:input.unknown_session
  operation close_snapshot(s: Snapshot) -> nothing
  ```

  A handle pins its revision for as long as it is open. Each open handle is charged to the run's memory budget at the
  logical size of its state. A snapshot's data is immutable, so a commit never waits for a reader.
- **KN2.3 [D]** Reads are batches. A program asks for a whole table, or for a named set of objects, in one call. No
  interface walks the stickers one at a time through the interpreter (LG4).
- **KN2.4 [D]** The calls that decide something are kernel calls typed by puzzle and level, each with an oracle agreement
  test (DT-L1 R2-E2). The small exact algebra above them is library code (LB1).

| Call | Puzzle, level | Result | Source |
| --- | --- | --- | --- |
| `labels`, `positions` | Full, labels | The label array; the position of named pieces | `core.py`; oracle |
| `effect` | Full, labels | A word's or recipe's source-to-destination map, support per orbit, exact length and cost | `core.py`; partition 10.2 |
| `protection` | Full, labels; jumbling, poses | A verdict for a protected set, Net or Strict, with its level | `session.py`; A3 |
| `retained_checkpoint`, `label_checkpoint`, `sticker_test` | By puzzle | Three separate end tests (default 9) | A5 |
| `pose_table`, `pose_ids` | Jumbling, poses | The canonical table and per-piece IDs of one revision | render contract |
| `classify` | Jumbling, poses | The outcome of a twist, with certificates | State contract section 3 |
| `survey` | Jumbling, poses | Admissible and blocked grips | J1 `state.py` |
| `rim_summary` | Jumbling, poses | Per cap: N_c, In_c, Out_c, A_c, blocked, at its floor | ENG-0J section 4 |
| `height_counts`, `heights` | Jumbling, poses | n₂, n₄, n₆ and the slot above 6; heights and domain IDs per distinct pose | ENG-0J section 4 |
| `straddlers` | Jumbling, poses | Straddlers and u of a cap the solver names, each with its certificate pair | ENG-0J section 4 |
| `release_check`, `word_check` | Jumbling, poses | The fields of EH-25, with the step flag | ENG-0J section 4 |
| `covers` | Jumbling, poses | Covers at a cap the solver names (a job) | ENG-0J section 4 |
| `nu`, `correction_set_check` | Jumbling, labels | ν at N = 0; "undefined" off the lattice; ν plus a drafted set's rows | A5; ENG-0J section 4 |
| `repair_list` | Jumbling, poses | The pairs at a cap the solver names (a job) | Owner, 17 October 2026 |
| `replay` | By puzzle | Whole-word replay, prefix by prefix | ENG-0J section 4 |

- **KN2.5 [B]** Arguments of the jumbling rows name the solver's object, and each answer is about that object only. The
  per-revision facts and the repair list at its named cap are the only exceptions (ENG-0J section 4, DT-J1). No row lists
  twists that close a cut or lower N except the repair list, ranks caps, marks a next cap, picks a target or answers
  whether a step exists.
- **KN2.6 [D]** Which profile may call which row is a table outside the kernel (BM3, chapter 07). The kernel exposes rows.
  It does not know who wrote a program.

### Words

- **KN2.7 [B]** A word is executed by its literal letters. A normal form is only an equality key (default 10). Preview,
  protection, commit and the token all use the literal letters, because merging deletes the states between letters, and
  Strict protection and the peak height are evaluated on those states.
- **KN2.8 [B]** The outcome of a jumbling twist is admissible, blocked, invalid or uncertain. A refusal changes nothing
  (state contract section 3). Admissibility of a whole word is a verdict per revision, from a replay of every prefix
  (ENG-0J section 4).
- **KN2.9 [B]** Nothing is reused from an earlier revision: no certificate, seed, hole count or pose-table index. A
  composed fast path is trusted only after an agreement test against per-twist replay, and every replay test states which
  check it ran (ENG-0J section 4). This is a rule on what an engine computation takes as input. A cached read result with
  equal inputs is a separate case, and a permission is never revived (R-08, ONE.10).
- **KN2.10 [D]** Words of any recorded length must work. The longest literal word stored flat has 2,352,984 letters
  (stage 18 lattice phase, step 14), and the 53 words of that phase hold 13,231,266 letters in one stream. Stage 18's
  admission bound is 10⁷ letters per word (partition 10.3). Composed words go further: 166,788,405 letters checked by
  composition and never played, and 224,683,850 in the largest retained cascade. ENG-0J's 672,417 twists and the T6
  step of 871,582 letters stay regression cases (LB5.13). These are source figures, and none of them is a limit (DT-L1
  R3-E4). Words stream through kernel calls in chunks. A compressed word (a straight-line program) expands lazily, its
  exact length is a big integer, and that length is known before any expansion. The length bound on what the kernel
  will replay comes from DT-J1's experiment X3.

### Derived states and the value store

- **KN2.11 [B]** A derived state is an immutable value that the same exact kernels compute (INV-1).

  ```text
  operation derive(base: StateRef, word: Word) -> Derived | Refused
    effect:  read a snapshot
    cost:    the word's exact length times the per-letter cost of the puzzle
    errors:  KN7:budget.*, KN1:input.identity_mismatch

  Derived = {
    ref:          DerivedRef       # digest, level, identity block
    derivation:   { base: StateRef, word: Digest<literal word> }
    reachability: ReachabilityClaim
  }
  Refused = { first_refused_prefix: Nat, outcome, certificates }
  ```

- **KN2.12 [D]** A derived state has no session revision. It lives in the kernel's value store by digest, and it is
  charged to the program's memory budget at its logical size (KN7.9). It can be compared, and displayed through a display
  plan that names its derivation. It is never adopted as the session state (INV-1, chapter 08).
- **KN2.13 [D]** A reachability claim names a puzzle and takes one of five values: witnessed, by theorem without a
  witness, promised, refuted or open (DT-L1 M3). The value is an evidence envelope reference (chapter 06), not a flag.
  For example, a label checkpoint is reachable by theorem with no word, and a lattice state reached by one of the 36
  corrections is witnessed in the jumbling puzzle while no retained word reaches its labels.
- **KN2.14 [D]** A state a program builds from raw data, such as an imported pose table, enters as a derived state with
  reachability open or promised, as its source says. The kernel validates its form and identity. Validation says nothing
  about reachability (A5: a pose state read without its journal is not a label checkpoint), and it does not test for
  collisions: two pieces of a raw table can overlap, and shape keeps the count (KN1.7).
- **KN2.15 [D]** The value store may evict a value and recompute it from its derivation. Eviction and recomputation never
  change a result or a charge (KN7.8).

## 4. KN3: exact arithmetic

- **KN3.1 [B]** Every decision is exact (INV-4). Nothing rounds silently.
- **KN3.2 [D]** The kernel provides these exact number kinds, each with stated operations and a cost by operand size:

| Kind | Use | Note |
| --- | --- | --- |
| Integers, unbounded | Pose entries, lengths, costs, bounds | Under I_a entry heights are unbounded (ENG-0J section 4), so pose entries can exceed 2^64; no table on record holds an integer above three decimal digits; B11 has 6,775 digits |
| Rationals | Cayley parameters, exact locality values | |
| Q(√5) | Poses, poles, cut offsets, region vertices | Signs are decided exactly (state contract section 3) |
| F2, F5 | ν and the 36 corrections | ν has 35 sign bits, 8 frame bits and two values modulo 5 (A5) |
| Exact matrices | Cayley rotations, half-turn branches | State contract A1 |

- **KN3.3 [D]** Exact polytope operations live here, with the exact double description of the state contract. The small
  exact algebra above this (quaternions, the icosian order and its tree, rows over F2 and F5) is library code (LB1).
- **KN3.4 [B]** The A1 input map turns a float request into an exact rotation, and the journal records the exact result.
  Replay uses the recorded rotation (state contract A1). The float site stays inside this one call. Whether the choice is
  the same under every runtime is spike LX-9. If it differs, the request becomes a dyadic Cayley vector read exactly, and
  a drag compares its 60 candidates exactly in Q(√5); that amends the state contract (EN1's Astra check, then the owner).
- **KN3.5 [B]** The A2 filter is an optional accelerator whose result never differs from the exact one, and the
  certificate records what it used (state contract A2, ENG-0J section 4).
- **KN3.6 [D]** The cost of an exact operation is a function of its operands' sizes. Words of the cost unit (LG2) and bit
  lengths are both exact, so a cost never depends on the machine.

## 5. KN4: session store

- **KN4.1 [B]** The store owns the journal (parent-linked events, branches kept), revisions, snapshots and checkpoints,
  jumble episodes, recovery, atomic writes and workspace records. Only the engine opens the session files (partition KN4).
- **KN4.2 [B]** State that must agree is written in one transaction (R-06). No response is written while the session lock
  is held (R-15). The cost of status does not grow with history (R-13).
- **KN4.3 [B]** The runtime reads the store only through query rows of the command layer (INV-2).

```text
query journal(session, from: EventId?, limit: Nat) -> Page<JournalEntry>
query revision_info(ref: RevisionRef) -> RevisionInfo
query checkpoints(session) -> List<CheckpointInfo>
query workspace_records(kind, filter) -> Page<WorkspaceRecord>
query receipt(command: CommandId) -> Receipt
  effect:  read a snapshot
  cost:    linear in the page returned, flat in history (R-13)
  errors:  KN2:stale.revision_gone, input.*
```

- **KN4.4 [D]** Workspace records change only through workspace commands. Each stored word, macro and workspace record
  carries its provenance field (BM2, chapter 05), so a word laundered through the workspace keeps its label.
- **KN4.5 [B]** A session without jumbling stores exactly what it would store without jumbling support, and an interrupted
  episode recovers as on or rolls back to its last committed revision (ENG-0J section 4).
- **KN4.6 [P LQ4]** An experiment branch is a journal branch the engine owns. Exactly one line per session is active.
  Other branches can be viewed and compared, and programs cannot change them. Under LQ4's alternative, solving sessions
  have no branches and research uses separate sessions; the query rows above stay the same.
- **KN4.7 [P LQ10]** One word commit is one journal row that cites the literal letters by digest, and the letters are
  stored as `ia16` chunks. As J1 rows a word of 2,352,984 letters would take about 649 million characters at 276 per
  row, against 4,705,968 raw bytes (arithmetic, DT-L1 R3-E4). The word encoding belongs to the data core, so the owner
  decides it under LQ10.
- **KN4.8 [D]** A stored delta between two jumbling revisions carries the pose-ID remap, or the revision carries its
  full pose-ID array. Moved pairs alone do not rebuild the next revision, because `_compact` reorders the pose table
  after a commit and remaps every pose ID, so unmoved pieces can change ID (`research/jumbling/sim/state.py:385-397`,
  KN2.9).

## 6. KN5: command layer

### The table and its rows

- **KN5.1 [B]** Every change to a session goes through the command layer and its gates (INV-2). It has one entry,
  `invoke`, for keys, buttons, menus and scripts (H-08), and no second path skips a gate (R-05, R-07).

  ```text
  operation invoke(command: CommandId, row: RowId, args: Args) -> Receipt
    lane:    the row's lane (KN7.2)
    errors:  input.*, stale.*, KN2:protection.*, busy.*, consistency.*, auth.*, capability.*
  ```

- **KN5.2 [B]** `invoke` returns a receipt at once (DT-M1). No engine call runs on the UI thread (R-14).
- **KN5.3 [D]** A command the language defines is an entry in a *language overlay* of the command table (EV4). It has the
  same row fields as a kernel row. Its kind is `view`, `workspace` or `query`, or `file` with a granted file capability.
  Its kind is never `commit` or `app`. Registering any other kind is refused.
- **KN5.4 [D]** An overlay row carries what the checker found: its static verdict, its capabilities, and the kernel rows
  it may invoke. A row the manifest marks as optional does not count towards availability.
- **KN5.5 [D]** Overlay rows reach the engine as data. The engine never runs language code to answer for them (KN0.7).

### Availability

- **KN5.6 [B]** The engine computes one availability table per revision, and the shell shows a control as unavailable if
  and only if the table says so (V04.3, proposal; the shell never computes availability). A palette reads the table once
  per revision, and again at each new generation (KN5.25), with no engine call per keystroke, and an unavailable entry
  keeps its place with a typed reason (DT-M1).

  ```text
  Availability = {
    revision:   RevisionRef
    generation: Nat        # KN5.25
    rows:       Map<RowId, available | unavailable(Error)>
  }
  ```

- **KN5.25 [D]** Availability reads inputs that no revision counts. A preview reaching `presented` makes a commit key
  live (chapter 08, section 6), and a lane's admission or release changes a `not_busy` gate (KN7.1), with every revision
  unchanged. DT-M1's sketch has the case: a check ends and Preview becomes available. So the engine numbers the tables of
  one epoch by a *generation*, which advances whenever an input of the table changes: a revision, a preview's lifecycle
  state or a lane's occupancy. A new generation replaces the last table, so one table stands for a revision at any
  moment (KN5.6). The engine sends an event when the generation advances, and the palette's read follows that event, so
  a keystroke still costs no engine call. Like every table it grants nothing: each row's gates run at invocation (KN5.7),
  so a table one generation old can mislead a display and never a commit. The event's form belongs to KN5.8's protocol.
- **KN5.7 [D]** A language-defined command is available when its static verdict passes and every non-optional kernel row
  it may invoke is available (default 17). The command layer computes this from two stored tables, so it needs no language
  code. Availability grants nothing: each row's gates still run when the program invokes it.
- **KN5.8 [O]** The availability protocol between the command layer and the engine needs an Astra plan check before the
  architecture freeze (H-08 open point 5).

### Proposals and previews

- **KN5.9 [B]** A preview owns an immutable copy of its witness (R-10). The engine builds it and owns it. Changes by the
  caller cannot change what is committed or journalled.

  ```text
  operation propose(base: RevisionRef, word: LiteralWord | RecipeRef, provenance: ProvenanceBlock) -> Receipt
    effect:  prepare a preview
    lane:    the exclusive lane for a long word, else the row's
    cost:    the exact length of the word
    errors:  stale.*, input.*, capability.*, KN2:resource_limit.*

  Preview = {
    id:         PreviewId
    base:       RevisionRef
    literal:    Digest<literal word>
    identity:   IdentityBlock
    witness:    engine-owned, immutable
    effect:     per puzzle: end digest, support, N and its change, length and cost in their units
    protection: verdict with its level
    provenance: ProvenanceBlock
  }
  ```

  A word that conflicts with protection still gets a preview, and its `protection` field says so, because charter S4
  shows that effect before a commit. The commit refuses it (KN5.16, ONE.33).

- **KN5.10 [B]** The preview replays the literal word at the current revision, prefix by prefix, and shows the full effect.
  Exact length and cost are shown before any run. A program is never split silently (default 10).
- **KN5.11 [B]** A session holds at most one pending preview. A new preview replaces it under a new ID (0.4 behaviour,
  kept). A replaced preview cannot be committed.
- **KN5.12 [O]** Whether a program may replace a shown preview at all is a design-line question (DT-L1 E2). The token
  binding keeps the commit safe either way.
- **KN5.13 [B]** A proposal never carries a state or a digest for the engine to adopt (INV-1).

### Commit

- **KN5.14 [B]** The shell mints a user-action token from a non-repeating key-down (or the equivalent explicit pointer
  action). It binds the token to the ID of the preview on screen and the digest of its literal letters, with the model
  identity, the menu identity, the menu-order digest and the contract revision. The token never binds a normal form
  (INV-3). The commit takes one action on a fresh preview (charter S4).

  ```text
  operation commit(preview: PreviewId, token: UserActionToken) -> Receipt
    caller:  the shell only
    lane:    commit
    errors:  KN5:auth.token_invalid, stale.*, KN2:protection.*, KN5:busy.commit_in_flight, consistency.*,
             unknown_outcome.*
  ```

- **KN5.15 [D]** The token is single-use and never persisted (V03.5). It carries an authenticator from a per-launch secret
  that the runtime never receives. No operation available to the runtime returns a token, in any placement of the
  runtime (LQ9).
- **KN5.16 [B]** At commit the engine rechecks the revision, the literal-letter digest, the full-state hash and protection
  (partition KN5; `AGENTS.md`). A moved revision, or a digest that differs from what the token binds, is refused as
  stale. A protection conflict is refused as `protection`. Equal revisions with unequal digests mean the engine is
  wrong, and they stop as `consistency` (chapter 08, section 7). The engine then adopts the end state of its own preview
  object. One durable transaction writes the journal event, the receipt and the workspace changes that must agree
  (R-06).
- **KN5.17 [B]** A commit typed while another commit is in flight is refused as busy (DT-M1 Q6 (a)). The commit chord is one
  non-repeating chord, and key repeat never repeats a state change (R-18, DT-M1).
- **KN5.18 [B]** A commit or undo goes ahead at once and cancels the jobs that read the revision it replaces. File jobs
  finish (DT-M1 Q5 (a)).

### Undo

- **KN5.19 [B]** There is one undo key and one durable order of actions across the journal and the workspace. An undo
  carries the action number its hint named, and it is refused as stale if another action landed first (DT-M1 Q4 (a)).
- **KN5.20 [B]** Undo is available while a job or a program runs (charter E4; DT-M1 fix-first 3, still open in EN4).
  `session.undo` must lose its `not_busy` gate before the runtime binds to H-08.

### Receipts

- **KN5.21 [D]** Every invocation has a durable command ID, chosen before submission, and a request digest. The same ID
  with different content is refused. One command ID produces at most one commit (V03.5).

  ```text
  Receipt = {
    command:  CommandId
    request:  Digest<request>
    status:   accepted | running(JobId) | refused(Error)
            | committed(CommitRecord) | cancelled | unknown_outcome
  }
  CommitRecord = { event: EventId, pre: Digest<level>, post: Digest<level>, revision: RevisionRef }
  ```

- **KN5.22 [B]** A committed command is never reported as failed. Commit outcome and display refresh are reported apart,
  and a refresh failure after a commit shows "applied, display out of date" (R-02).
- **KN5.23 [D]** No receipt, broken journal continuity or an incomplete recovery record means `unknown_outcome`, not proof
  of non-execution. The session enters read-only recovery (V03.5).
- **KN5.24 [B]** Errors are typed with stable codes, and each category has its own continuation (H-08, V06; chapter 13).
  The shell never merges evidence state, score and permission (charter bar M3).

## 7. KN6: independent checkers

- **KN6.1 [B]** There is one small checker per certificate kind: admissibility and straddlers, macro certificates,
  compressed-word expansion, ν and label checkpoints, research archives (partition KN6).

  ```text
  operation check(certificate: CertificateRef, kind: CheckerKind) -> CheckReport
    effect:  read a snapshot
    cost:    declared by the checker from the certificate's size
    errors:  evidence.*

  CheckReport = {
    verdict:  pass | fail(claim: ClaimId, reason) | not_applicable(reason)
    checker:  CheckerId and its source digest
    input:    Digest of the certificate bytes
    trace:    a per-claim trace, by reference
  }
  ```

- **KN6.2 [B]** A checker recomputes every claim letter by letter without caches. It shares no search or construction code
  with the producer. It uses the kernel's exact engines and model files, which the oracles test separately. It never
  trusts a producer's verdict.
- **KN6.3 [D]** Checkers are kernel code, so users cannot change them. A checker's own result is a fact in an evidence
  graph (chapter 06), and its identity is its source digest.
- **KN6.4 [D]** The trace is the checker's own output, so stepping through it adds no import of the producer (DT-L1
  R2-E4). A trace is stored or re-derived on demand, and a long step is cited by word digest and positions.
- **KN6.5 [B]** Each checker has a mutation gate: tampered records and wrong start tables must fail.
- **KN6.6 [D]** A checker never prints a judgement beyond its kind. It says that a certificate's claims hold for its stated
  inputs. What that means for a statement is the evidence model's work (chapter 06).

## 8. KN7: jobs and budgets

### Jobs and lanes

- **KN7.1 [B]** Long work runs as a job, never on the UI thread (R-14). Admission is atomic and is one compare-and-set per
  lane, keyed by the durable command ID, and a one-job limit cannot be bypassed by concurrent submissions (R-16, DT-M1).
- **KN7.2 [D]** A lane is a named admission rule. DT-M1 names the commit, exclusive and latest-wins lanes. The language
  needs two more, which EN4 confirms.

| Lane | Admission | Used by | On a commit or undo |
| --- | --- | --- | --- |
| Commit | One at a time; a second is refused as busy | Commit rows, undo | Not applicable |
| Exclusive | One job at a time | Covers, the repair list, long checks | Cancelled if it reads the replaced revision |
| Latest-wins | The newest request replaces an older one | Release checks | Cancelled |
| Program (new) | A bounded number of program jobs | Language-defined commands | Cancelled if it reads the replaced revision, unless pinned (KN7.5) |
| View (new) | One per view, latest wins, skip to the newest revision | Program views | Recomputes at the newest revision |

- **KN7.3 [B]** A job's outcome is completed, cancelled, cut off or error. Each result is bound to its input digest, its
  parameters and what it covered. The view model reads a result only after its digest matches the shown revision. An
  equal digest after an undo is a match (R-08, ONE.10). An empty list comes only from a completed job and names its cap
  and revision (ENG-0J section 4).

  ```text
  JobResult = {
    outcome:  completed | cancelled | cut_off(by: Code) | error(Error)
    input:    Digest<level>
    params:   Digest
    covered:  a typed description of what was examined
    budget:   the budget granted and the amount used
    value:    the result, if any
  }
  ```

- **KN7.4 [B]** A job that did not complete yields no negative claim, and a witness it found counts once it is checked
  (partition 8.1, default 6).
- **KN7.5 [P LQ4]** A job whose input is pinned by digest finishes after a commit, as file jobs do. Its result reaches a
  newer revision only by a re-run at that revision or by an exact transport that a lemma justifies. Under LQ4's
  alternative, pinned jobs are cancelled like the others.
- **KN7.6 [B]** Cancellation is cooperative and phase-aware. Acknowledging a request does not guarantee it. Before commit
  the result is `cancelled` with the state unchanged; after a durable commit it is the committed receipt (V03.5). Kernel
  batches check for cancellation at chunk boundaries, and the chunk size is a declared part of each batch.

### Budgets

- **KN7.7 [B]** Every job has an explicit deterministic budget (default 6, INV-10). Seconds are a display unit, never a
  budget unit.

  ```text
  Budget = {
    steps:        Nat       # interpreter steps
    kernel_cost:  Nat       # declared cost of kernel calls
    memory:       Nat       # logical units
    output:       Nat       # logical units of results, records and plans
    safety_stop:  Seconds?  # a machine limit, not a budget unit; never part of the reproducibility key
  }
  ```

- **KN7.8 [D]** Every kernel call declares its cost before admission, as a function of its inputs only. A cache hit is
  charged as a miss, and a recomputation after an eviction is charged as nothing extra. Otherwise one reproducibility
  key would stop at different steps depending on what the cache held.
- **KN7.9 [D]** Memory is counted in logical units: the byte length of a value's arrays in the KN1 digest convention,
  plus, for a pose table, the length of each entry's canonical exact key. The text form in a file is never the unit
  (DT-L1 round 3). What the kernel shares or the allocator does changes nothing, because Windows and Linux allocators
  differ (INV-9). A 600-cell-Full state counts as 1,039,200 bytes of labels (source). A pose state counts as its pose IDs
  (708,480 bytes, source) plus its table entries.
- **KN7.10 [B]** A budget that ends cuts the job off, with that budget's code. A wall-clock safety stop also cuts it off,
  with the code `KN7:resource_limit.safety_stop`, because a `budget` code promises that the same key stops at the same place
  and a clock cannot promise that. That run reads incomplete and not reproducible (INV-10).
- **KN7.11 [B]** "Empty", "not found within budget" and "not run" are distinct results (default 6).

### Isolation of the critical path

- **KN7.12 [D]** A running job never delays a commit, an undo or a key bound to a kernel row. Reads work on immutable
  snapshots and take no lock that a commit needs. How this is met in each placement is LX-2's question.
- **KN7.13 [D]** A runtime that dies ends its jobs as `error`. The journal and the state hash are unchanged, and the shell
  stays responsive (LX-2's pass rule).

## 9. What this chapter asks of EN3 and EN4

EN3 delivers the jumbling command rows, view-model fields and `jumbling-interfaces.md`. EN4 delivers the final H-08, the
layer interfaces and the migration-format packet (re-plan of 22 October 2026). Neither has delivered yet. The language adds
these needs:

| Item | Owner | Clause |
| --- | --- | --- |
| Undo without the `not_busy` gate; a workspace-undo command; undo of a protection row through R-05's path | EN4 | KN5.20 |
| The revision reference's final fields | EN4 | KN0.2 |
| The program and view lanes | EN4 | KN7.2 |
| The language overlay of the command table and its checks in `build_command_table.py` | EN4 | KN5.3 |
| The availability protocol, with the overlay rule and the generation event | EN4 | KN5.7, KN5.8, KN5.25 |
| Query rows for the journal, revisions, checkpoints, workspace records and receipts | EN4 | KN4.3 |
| A provenance block on proposals and on stored words | EN4 | KN5.9, KN4.4 |
| The token's binding fields | EN4 | KN5.14 |
| The rows of KN2.4 for jumbling, with heights and domain IDs as a kernel batch | EN3 | KN2.4 |
| The view-model fields of the relationship strip's data contract | EN3 | Chapter 04 |

## 10. Conformance tests

| Test | Checks | Class |
| --- | --- | --- |
| CT-KN1-01 | A value with another model identity is refused or marked as recorded | source |
| CT-KN1-02 | A (grip, index) word recorded under the reversed I_a order is refused: same menu identity, other order digest | source |
| CT-KN1-03 | A projection off the lattice returns a typed partial result with the count of pieces off | source |
| CT-KN1-04 | An enumeration of the API finds no lift from a coarser level to a finer one | source |
| CT-KN1-05 | `Digest<labels>` equals the oracle's state hash and `Digest<poses>` equals J1's digest on every fixture | source |
| CT-KN1-06 | Equal shape means an equal multiset of posed regions: on the raw tables (R, R, QR) and (R, QR, QR), with Q the Cayley rotation ω = (2,0,0) at cap 0, the shapes and the off counts differ; no fixture has equal shapes with unequal off counts; a gauge with σ ≠ id keeps the multiset | source |
| CT-KN2-01 | The J1 harness and the 600-cell-Full oracle (O01 to O07) pass through the kernel API | source |
| CT-KN2-02 | Heights and domain IDs equal the library and `trees.py` on ENG-0J item 8's states and the H6030 witness. A free-mode case: the Cayley rotation ω = (2,0,0) at cap 0 reads heights (0,0) and lies outside K⁺, and its domain ID comes from an exact coset test, not from tree positions | source |
| CT-KN2-03 | Protecting centre piece 0: refused at pose level inside Jumble, passes at label level after the crossing | source |
| CT-KN2-04 | A blocked, invalid or uncertain word leaves every digest unchanged | source |
| CT-KN2-05 | A scan finds no float on a decision path outside the sites of INV-4 | source |
| CT-KN2-06 | Gauge transfer (KN1.14) as a conformance test of the engine on J1 fixtures, with S, R and σ ≠ id cases, on and off the lattice. Note N1 proves the statement; this test checks the engine against it | source |
| CT-KN2-07 | The recorded literal words of 672,417, 871,582 and 2,352,984 letters, and the 13,231,266 letters of the stage 18 lattice phase as one stream, pass through the replay interface in chunks (KN2.10, LB5.13, CT-LB5-09) | source |
| CT-KN3-01 | Sign tests at the state contract's small margins decide exactly | source |
| CT-KN3-02 | Synthetic pose entries beyond 2^64 and a 6,775-digit integer round-trip exactly. No record reaches 2^64, so the pose entries are built for the test | synthetic |
| CT-KN3-03 | The A2 filter's result equals the exact result on adversarial cases | source |
| CT-KN4-01 | A runtime in any placement cannot open a session file | source |
| CT-KN4-02 | R-06: injected failures at each write leave all of the state as before | source |
| CT-KN4-03 | R-13: status cost is flat at 0, 50, 200 and 400 checkpoints | source |
| CT-KN5-01 | Preview swap: a preview replaced between its display and the commit is refused | source |
| CT-KN5-02 | No operation in the runtime's capability table returns a token or calls `commit` | source |
| CT-KN5-03 | R-18: a held key commits once | source |
| CT-KN5-04 | The same command ID with other content is refused; a retried delivery commits at most once | source |
| CT-KN5-05 | R-02: a refresh failure injected after a commit reports "applied, display out of date" | source |
| CT-KN5-06 | R-10: a caller's mutation after a preview changes nothing committed or journalled | source |
| CT-KN5-07 | Registering an overlay row of kind `commit` or `app` is refused | source |
| CT-KN5-08 | The availability of a language-defined command is computed with no language code running | source |
| CT-KN5-09 | Undo succeeds while a program job runs | source |
| CT-KN5-10 | R-05: an enumeration of every entry point against the restriction table | source |
| CT-KN5-11 | An undo that names a superseded action is refused as stale | source |
| CT-KN5-12 | With `RevisionRef` held constant, a preview's creation, presentation and cancellation and a lane's admission and release each advance the generation, the commit row's availability follows chapter 08's lifecycle table, and no keystroke reaches the engine | source |
| CT-KN6-01 | Every checker kind's mutation gate: tampered records and wrong start tables fail | source |
| CT-KN6-02 | An import scan: no checker imports a producer's search or construction code | source |
| CT-KN6-03 | The conformance corpus: the existing certificates pass under the kernel's checkers | source |
| CT-KN7-01 | Repeated runs of one key stop at the same step | source |
| CT-KN7-02 | A cache hit and a recomputation after eviction leave the charges unchanged | source |
| CT-KN7-03 | A cut-off job yields no negative claim | source |
| CT-KN7-04 | DT-M1's XM4 with a long program job: a commit cancels jobs on the old revision, and file jobs finish | source |
| CT-KN7-05 | R-16: concurrent invocations and a held key cannot pass a one-job limit | source |
| CT-KN7-06 | A commit completes within charter E2 while a program job runs at full load | performance |

## 11. Open items

- **LQ3, LQ10.** The form of a reachability claim and a check report inside the evidence envelope (chapters 05, 06).
- **LQ4.** Branches (KN4.6) and pinned jobs (KN7.5).
- **LQ8.** The engine language. Nothing in this chapter depends on it, except the exact arithmetic libraries it can use
  (LX-5).
- **LQ9.** Where the runtime runs. KN5.15, KN7.12 and KN7.13 must hold in each placement.
- **LQ21.** What a provenance block holds (KN5.9): chapter 07.
- **EN3, EN4.** Section 9.
- **H-08 open point 5.** The availability protocol's plan check (KN5.8).
- **Open after note N1.** Whether the same word language implies the same shape; reachability of sticker classes in
  J(I_a); an exact certificate for Lemma R; a bridge lemma with its own ID, if a goal wants "some member of Z's sticker
  class is reachable". Nothing in this chapter depends on them.

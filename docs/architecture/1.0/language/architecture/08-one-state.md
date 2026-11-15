# Chapter 08: one state

Status: **frozen with the LP1 candidate** (owner sign-off, 15 November 2026;
[decision](../../../../wiki/decisions/owner-decisions-2026-11-15-freeze.md)). Conventions are in
[the overview](00-overview.md), section 3, and its status says what the freeze covers.

A session has one authoritative puzzle state, and nothing a program does can make a second one. This chapter follows that
promise through the system. It gives the life of everything that can look like a state, the two paths to a commit, what
happens to each thing when the state changes, and how the runtime meets the view model and the renderer.

Chapter 01 holds the signatures. This chapter holds the order of events.

Sources: the partition's INV-1 to INV-3 and sections 8.3 and 9, [ENG-0J revision
2](../../../../progress/1.0/packets/engineering/ENG-0J-jumbling-backend.md) section 4, [state contract
A5](../../../../../research/jumbling/state-contract.md), the [render contract](../../../../../research/jumbling/render-contract.md),
[DT-M1](../../../../progress/1.0/design-table/topics/command-keyboard-model.md), [command table
H-08](../../../../progress/1.0/command-table.md), [requirements R-01 to R-19](../../../../progress/1.0/requirements-from-screening.md),
[the observation requirements](../../../../progress/1.0/jumbling-observation.md), [the solving
workflow](../../../../progress/1.0/solving-workflow.md) section 4.1, DT-L1's cards E1 to E3 and
[`10_V1_ARCHITECTURE.md`](../../10_V1_ARCHITECTURE.md) sections V03.3 and V03.5, as proposals where that document marks
them so.

## 1. The rule

- **ONE.1 [B]** Each session has exactly one state of record: the state at the journal head of its active line. The engine
  owns it, and the session store records it (INV-1). Its level is labels with Jumble off and poses with Jumble on (KN1.7).
- **ONE.2 [B]** Only a row of kind `commit` changes the state of record, through the command layer and its gates (INV-2,
  H-08). These include `twist.apply`, `operation.execute` and `session.undo`. EN3 supplies Jumble on, Jumble off and
  the menu twist; p2a owns the final row inventory. The first draft's count is not the final command set.
- **ONE.3 [D]** No row of kind `commit` is available to the runtime, in any profile or placement. This is wider than
  KN5.3, which stops a program defining such a row. A program can neither define one nor invoke one. A checkpoint restore,
  a log import and an undo are solver actions too (INV-3).
- **ONE.4 [D]** Every row of kind `commit` reaches the command layer with a user-action token from the shell.
  - For `operation.execute` the token binds as KN5.14 says.
  - For a row with no preview, such as `twist.apply` or `session.undo`, the token binds the row and its absolute arguments.

  This extends KN5.14 from previews to every commit row. EN4's draft supplies the common envelope and binding in
  LI.5 and LI.6, without admitting a token or a commit row to the runtime.

## 2. Everything that can look like a state

- **ONE.5 [D]** Eight things can look like a puzzle state. Only the first is one.

| Thing | What it is | Owner | Can it change? | Session revision | Can a commit adopt it? |
| --- | --- | --- | --- | --- | --- |
| State of record | The state at the head | Engine | Only through a commit row | It is one | A commit replaces it |
| Snapshot | A read-only handle to one revision | Engine; a run holds the handle | Never | Names one | No |
| Derived state | An immutable value with a derivation | Kernel value store | Never | None | No |
| Preview | The engine's own dry run of a literal word | Engine | Never; it is replaced whole | Names its base | Its end state, when every recheck passes (section 7) |
| Draft | Letters in a phase; it holds no state | Workspace | Through workspace rows | Workspace revision only | No; it reaches the state only through a preview |
| Adopted bundle | The published image of one revision, for drawing | View model | Replaced whole | Names one | No |
| Stored or imported state | Data in a record | The record | Never | None | No; it enters as a derived state (KN2.14) |
| A program's own table | Data a program copied or built | The program | As the program likes | Only what the program kept | No |

- **ONE.6 [D]** A program may copy whatever it can read into its own data and compute with it. The design does not stop
  that, and it need not. Such a copy has no identity block, and no kernel call accepts it as a state. It can enter only
  through KN2.14, as a derived state with reachability open or promised. So a program can hold a wrong picture of the
  state, and it can mislead only through what it says (overview section 7). It cannot make that picture authoritative.

## 3. Revisions and freshness

### What a result is bound to

- **ONE.7 [B]** Every result names the revision or derived state it read (KN0.6). Old previews, async results and delayed
  picking results are never applied to a mismatched state (V03.3).
- **ONE.8 [D]** A result also declares which parts of the revision it depends on, so that an unrelated edit does not
  invalidate it (V03.3: declared dependency revisions).

  ```text
  Base = {
    revision: RevisionRef
    depends:  Set<state | workspace | protection>
  }
  ```

  This is a read-result binding. Preview permission instead uses KN0.8's `PreviewDeps`; neither `Base` nor equality
  of a full `RevisionRef` supplies another preview freshness rule.

- **ONE.9 [D]** Within one engine epoch a revision number never goes back. Undo, redo and a restore each make a new
  revision, as `Session.rev` does in 0.4 ([`session.py`](../../../../../session.py)). So a digest can recur and a revision
  cannot.
- **ONE.10 [D]** State equality does not identify a transaction: a legal no-op can have equal hashes, and undo can revisit
  an earlier state (V03.5). Two kinds of bound value must therefore stay apart:
  - A **read result** is a function of its declared inputs. It is valid wherever those inputs are equal: the same digest at
    the named level, the same parameters and the same dependencies. After an undo returns to an equal digest, a cached
    result with equal inputs may be shown again (R-08). It is still charged as a miss (KN7.8).
  - A **permission** is bound to a moment. It is the right to commit a preview, or a token. A later equal digest never
    revives it, and nothing is reused from an earlier revision (KN2.9).

### What shows when something is out of date

- **ONE.11 [B]** A filter result, a count and a tint belong to one revision. After a commit they are recomputed or discarded,
  never shown against a newer state (observation requirements rule 3, R-19). This holds for program output too. DT-M1
  ruled out the option of letting old jobs finish and showing their results as stale (filter, E4 (b)).
- **ONE.12 [D]** Three out-of-date conditions look alike and differ. Each has its own display, and none stands in for
  another (DT-M1: the preview's pattern is distinct from "applied, display out of date").

| Condition | Meaning | What shows | Source |
| --- | --- | --- | --- |
| Applied, display out of date | A commit succeeded and a refresh failed | R-02's message; no retry is offered | R-02 |
| View stale | A view's output lags the adopted revision | The view's stale state, naming the base revision of its last output, until new output arrives. The old output is not drawn against the new state (ONE.11) | Charter D2, rule 3 |
| Preview stale | Its draft was edited, or a revision it depends on moved | The stale pattern; the commit key is not live | DT-M1 |

  The partition's section 8.3 says that a result "shows the stale state" after a commit. Rule 3 is stricter, so this chapter
  reads the stale state as the view's own designed state, never as old content over a new state. How the state looks
  belongs to the design line.
- **ONE.13 [D]** A stale hover or pick is dropped quietly. Stale work the solver explicitly asked for gets a visible
  explanation (V03.3).

## 4. Snapshots

```text
open_snapshot ──► open ──► closed      closed by: close_snapshot, the end of the run, the end of the engine epoch
```

- **ONE.14 [D]** A snapshot is a capability. Only `open_snapshot` makes one, and the handle works only in the run that opened
  it. What a program stores is the `RevisionRef`, never the handle.
- **ONE.15 [B]** A snapshot's data is immutable. A commit never waits for a reader (KN2.2, KN7.12), and a reader never sees a
  half-applied state. Reads through one handle give the same bytes however the head moves.
- **ONE.16 [D]** A run that needs two reads to agree takes both from one snapshot. Two calls of `open_snapshot(current)` can
  return different revisions.
- **ONE.17 [D]** A handle does not outlive its engine epoch. A stored reference can be opened again: `open_snapshot` resolves
  it by session, head event and digest, and a different epoch does not invalidate a read (V03.3: a restart invalidates
  transient permissions). `KN2:stale.revision_gone` means that the store no longer holds that event.
- **ONE.18 [D]** A commit does not close a snapshot. It cancels the jobs that read the replaced revision (KN5.18), and a
  cancelled job's handles close. A handle held outside a job, by an interactive evaluation in the editor, stays readable.
  Its results keep naming their revision, and ONE.11 applies to them.
- **ONE.19 [D]** A pinned job keeps its input after a commit (owner, 6 November 2026, LQ4; KN7.5). The handle stays open
  until the job ends.

## 5. Derived states

### Making one

- **ONE.20 [B]** `derive` applies a word with the same exact kernels and returns an immutable value, or the first refused
  prefix with its outcome and certificates (KN2.11). A refusal changes nothing.
- **ONE.21 [D]** The base of a derivation is a snapshot or another derived state. A stored derivation names its base by
  `RevisionRef` or by digest, never by handle, so it can be recomputed later. Following the chain back gives the value's
  lineage: the revision of its root snapshot, or, for a value imported under KN2.14, that import's digest and no
  revision. Lineage is a fact about the value and gives it no session revision. A display plan has its own base, the
  revision the display computation read (RPI.6), and the two can differ. A plan showing a value with no session ancestor
  uses its base as an explicit comparison base, which claims nothing about how the value was produced (EV3.12).
- **ONE.22 [D]** A derived state keeps the level and identity block of its base. A letter of another puzzle is refused with
  `KN1:input.identity_mismatch`. A value moves to a coarser level only by `project` (KN1.9), and nothing lifts it to a finer
  one (KN1.10).
- **ONE.23 [D]** Which profile may call `derive`, and how often, is the profile table's matter (KN2.6, BND.57). A search
  over twists is built from `derive`, so this row is where the boundary policy acts.

### Using one

- **ONE.24 [B]** A derived state can be compared, and it can be displayed through a display plan that names its derivation.
  It has no session revision, and it is never adopted as the session state (INV-1, KN2.12).
- **ONE.25 [D]** Comparison is one kernel call at a named level. Label equality is weaker than pose equality (KN1.6), so the
  level is part of the question.

  ```text
  operation compare(a: StateRef, b: StateRef, at: Level) -> Partial<Verdict>
    effect:  read a snapshot
    cost:    linear in pieces
    errors:  KN1:input.level_not_held, KN1:input.identity_mismatch
  ```

  Off the lattice a comparison at the labels level returns a typed partial result, as `project` does. Digests of two levels
  never compare (KN1.8).
- **ONE.26 [B]** To make a derived state real, the solver plays its word through the normal path. The word goes through
  `propose`, and the engine replays it at the *current* revision, prefix by prefix (KN5.10). The derived state is not
  consulted. If the head has moved since the derivation, the end state can differ, or the word can be refused. The preview
  shows what the word does now. A proposal carries no state and no digest to adopt (KN5.13).
- **ONE.27 [D]** A derived state with no word, such as an imported one, cannot be proposed: it has no word to replay. It
  can be read, compared and displayed, and it can be the base of `derive` (ONE.21). A value derived from it keeps the
  import's reachability claim, because a word cannot settle whether its start is reachable, and like every derived state
  it is never adopted (ONE.24). That value's own word can be proposed: it replays at the current revision like any word
  (ONE.26), and it says nothing about the imported value.
- **ONE.28 [D]** A predicate that depends on reachability reads the state's claim (KN2.13). A5 gives the case: a pose state
  read without its journal is not a label checkpoint, and a lattice state that is not reachable can have ν = 0 and displaced
  centres. So `label_checkpoint` on a derived state answers yes or no only when its reachability is witnessed back to a
  state the journal reached, or holds by a theorem that covers it. Otherwise it answers `unknown(reason)`. A promise of
  reachability stays a promise (chapter 06).

### Ending one

- **ONE.29 [B]** A derived state is charged to the program's memory budget at its logical size. The value store may evict it
  and recompute it from its derivation, and that never changes a result or a charge (KN2.12, KN2.15, KN7.8).
- **ONE.30 [D]** A record stores the derivation and the digest. It never needs the value itself to stay meaningful, and the
  value store is not durable.

## 6. Previews

### What a preview is

- **ONE.31 [B]** The engine builds a preview and owns it. It holds an immutable copy of its witness, so nothing a caller does
  afterwards changes what is committed or journalled (R-10, KN5.9).
- **ONE.32 [D]** A program receives a preview's public fields: its ID, base, dependencies, literal digest, identity block, effect,
  protection verdict and provenance. It never receives the witness as something it could hand back, and it never receives
  a token. This differs from 0.4, where `preview()` returned a token to its caller. In the solving profile chapter 07
  narrows the fields a program may use (BND.42, BND.43).
- **ONE.33 [B]** A preview that conflicts with protection is still built, with a verdict that says so, because the charter's S4
  shows the effect on protection before a commit. `operation.preview` has no `protection` admission gate; a complete
  admissible replay returns `completed(preview_ready)` with the conflict verdict. `operation.execute` keeps the
  protection gate and refuses that commit. For a word that moves a protected labelled orbit, the preview succeeds
  and the otherwise valid commit is refused with `KN2:protection.orbit_would_move`, leaving head and state unchanged.
  A word mechanically refused at a prefix builds no preview: the receipt gives the first refused prefix (ENG-0J section 4).
- **ONE.34 [D]** A preview is prepared only inside a command the solver invokes, and a view has no preview effect (LG3.6).
  This follows from BM1 and default 16. It is a new integrator default, and the owner may overrule it.
- **ONE.35 [B]** Draft input appends to a phase and leaves the committed state alone. A draft reaches the state only through a
  preview (solving workflow 4.1). Neither a commit nor an undo of another class drops a draft, and Escape never drops one
  (DT-M1).
- **ONE.36 [D]** A word a program hands to the solver goes to a draft or to a preview, and it keeps its provenance either way
  (KN4.4). A write to a draft is a workspace change: it happens only inside a command the solver invokes, as one undoable
  action (default 16). The draft keeps the word's block (BND.58); `operation.preview` applies BND.84 before a solving
  preview exists.
- **ONE.37 [D]** The float motion of a drag or a turn preview is display (INV-4). It carries no witness and cannot be committed.
  The letter it realises goes through `twist.apply`.

### Its life

```text
              ┌───────────────► refused
propose ─► building ─► ready ─► presented ─► committed
              │          │         │  ▲
              │          │         ▼  │ (re-proposed under a new ID)
              ▼          ▼       stale
          cancelled   replaced, cancelled, discarded   (also from presented and stale)
```

| State | Meaning | Is the commit key live? |
| --- | --- | --- |
| building | The engine replays the word. A long word runs as a job | No |
| ready | The preview exists and has not reached the screen | No |
| presented | The renderer adapter has reported a frame that shows it | Yes, while fresh and the other gates, including protection and ONE.42's open policy, pass. The availability generation advances (KN5.25) |
| stale | Its resolved-draft dependencies or protection revision changed (KN0.8) | No |
| committed | Its end state was adopted | Ended |
| replaced | A newer preview took the single slot (KN5.11) | Ended |
| cancelled | The solver cancelled it, or the job that built it was cancelled | Ended |
| discarded | The state revision moved, or the engine epoch ended | Ended |
| refused | A prefix was refused; no preview exists | Ended |

- **ONE.38 [B]** A session holds at most one pending preview. A successfully built and admitted replacement takes
  its place under a new ID, subject to ONE.42. A replaced preview cannot be committed (KN5.11).
- **ONE.39 [D]** Freshness is exactly `fresh(p)` of KN0.8, for the H-08 `fresh_preview` gate, availability and commit
  step 6 alike. An unrelated workspace edit preserves it. An edit to a read draft record, a protection change or an
  undo that moves the state revision defeats it, even when the labels return to the same digest. Freshness alone
  grants no commit: the preview must also remain pending, be presented and pass the token and protection checks.
- **ONE.40 [B]** A preview of an older state is a per-revision result. When the state revision moves, the engine discards the
  preview, as 0.4 does, and rule 3 forbids showing it against the new state. The draft stays. A commit key pressed then
  shows the reason and offers a fresh preview (DT-M1 C3).
- **ONE.41 [D]** **On screen means presented.** The shell binds the token to the preview of the newest frame the renderer
  adapter has reported as presented. It never binds the newest preview it has received. Between those two moments the
  screen still shows the older preview, so binding the newer one would commit a word the solver has not seen. E2's fix binds
  "the preview it adopted"; this clause closes the gap between adoption and the frame. The report belongs to the
  renderer handoff (owner, 6 November 2026, LQ20), and LP3 acceptance still needs its delivery (RPI.66). A presented
  preview is drawn by the built-in chain, so a commit token never binds to a user stage's frame (RPI.76).
- **ONE.42 [O]** The token proves which preview was on screen. It cannot prove that the solver had time to read it. A program
  that replaces a shown preview one frame before the chord leaves that gap open. This is KN5.12's question, for the design
  line. The options are: (a) only a solver action ends a shown preview; (b) a program may replace it, and the commit key
  stays dead for a stated time after a new preview is presented. The token binding keeps the commit correct either way.
- **ONE.43 [D]** A preview is not durable. It dies with the engine epoch, and restart never revives it (V03.5). The draft
  persists. Its preview invocation has an epoch-retained receipt under KN5.23; after that epoch an unavailable
  receipt gives `unknown_outcome`. A saved preview summary is only a record. Neither that record nor a retained
  receipt for the later commit can revive the preview or its token.

## 7. Commits

### A whole word

`operation.execute` commits a word from a fresh preview. The steps run in this order. The order is part of the contract, and
each step's failure leaves the state unchanged, except where the table says otherwise.

| Step | Who | Check or action | On failure |
| --- | --- | --- | --- |
| 1 | Shell | The key-down is non-repeating and outside a text field; a preview is presented and the engine's latest availability permits the chord (DT-M1 C4, ONE.42). The engine rechecks at invocation | The state is untouched and the reason shows: `KN5:stale.no_fresh_preview` |
| 2 | Shell | Mints the token, bound to the presented preview (KN5.14, ONE.41) | |
| 3 | Command layer | The token's authenticator is valid, and the token is unused (KN5.15) | `KN5:auth.token_invalid` if the authenticator fails; `KN5:stale.token_used` if the token was seen before |
| 4 | Command layer | Admission to the commit lane (KN5.17) | `KN5:busy.commit_in_flight` |
| 5 | Engine | The token's preview is the pending preview, with the same literal digest | `KN5:stale.preview_replaced` if it names another preview; `KN5:stale.token_mismatch` if a genuine token's row, arguments, digest or identity differ |
| 6 | Engine | `fresh(p)` holds: compare `PreviewDeps` as KN0.8 defines, not the full base `RevisionRef` | `KN5:stale.revision_changed` |
| 7 | Engine | The head's full-state hash equals the preview's start digest | `KN5:consistency.state_hash_mismatch`, because equal revisions with unequal digests mean the engine is wrong |
| 8 | Engine | Protection is rechecked, at its level (KN1.11) | `protection.*` |
| 9 | Engine, store | One durable transaction records the engine preview's end state, journal event, receipt and workspace changes that must agree. Final checks and the write share LI.8's serialization point (KN5.16, R-06) | A proved rollback fails with no effect; `unknown_outcome` if durable fate cannot be established (KN5.23) |
| 10 | Engine | Adopts the durable result, fences and cancels jobs on replaced dependencies without waiting for cleanup, publishes the new bundle, returns the receipt | Never reported as failed once step 9 is durable (R-02) |

- **ONE.44 [B]** At step 9 the engine adopts its own object. No value from a program takes part, so step 9 needs no trust in the
  language (INV-1, KN5.13).
- **ONE.45 [B]** A commit goes ahead at once (KN5.18). `not_busy` is not a gate. The commit lane refuses a second
  commit as `KN5:busy.commit_in_flight`, never a commit because a program job is running (DT-M1 Q5 (a), R-16, KN7.12).

### A live twist

- **ONE.46 [B]** A live twist is an immediate complete operation under the active protection (solving workflow 4.1). `twist.apply`
  has no preview object. The key names the literal letter, the engine decides its outcome at the current revision, and a
  refusal changes nothing (KN2.8).
- **ONE.47 [B]** A live turn cannot borrow the cleanup of a future draft (solving workflow 4.1). Temporary motion of protected
  pieces is allowed only inside a reviewed complete draft, so only inside a preview.
- **ONE.48 [D]** No interpreter stands on this path. A key bound to a kernel row dispatches without the runtime (default 15), and a
  key bound to a language-defined command can at most reach a preview.
- **ONE.78 [B]** Charter S4's preview applies to a turn entered as a draft step, to a macro and to any word (owner,
  6 November 2026, decision 10). A live twist shows nothing first, as solving workflow 4.1 and DT-M1 C4 say. The solver's
  key names the one letter and the active protection still gates it. `twist.apply` gains no preview, and ONE.46 stands.

### What a commit does to everything else

- **ONE.49 [D]** A commit, an undo and any other row that moves the state revision act on the things of section 2 as follows.

| Thing | At a state revision change |
| --- | --- |
| Open snapshot | Stays readable and unchanged (ONE.18) |
| Job reading the replaced revision, in the program or exclusive lane | Cancelled, with no negative claim (KN7.4, KN5.18) |
| Job building a preview, and latest-wins jobs | Cancelled (DT-M1 Q5 (a)) |
| File job | Finishes (DT-M1 Q5 (a)) |
| Pinned job | Finishes. Its result reaches a newer revision only by a re-run or an exact transport that a lemma justifies (owner, 6 November 2026, LQ4; KN7.5) |
| View job | Recomputes at the newest revision, skipping any between (KN7.2) |
| Pending preview | Discarded, unless it is the one being committed (ONE.40) |
| Unused token | Dies with the preview it binds |
| Derived state | Unchanged. It keeps its derivation and its old base |
| Draft | Stays (ONE.35) |
| Per-revision fact on screen | Recomputed or discarded (ONE.11) |
| Adopted bundle | Replaced whole (section 10) |

## 8. Undo and redo

- **ONE.50 [B]** There is one undo key and one durable order of actions across the journal and the workspace. An undo carries the
  `ActionHint` of LI.9, binding current order and target separately, and is refused as stale if either no longer matches
  (KN5.19). `session.undo` and `session.redo` dispatch both action classes internally. Undo is available while a job
  or a program runs (KN5.20).
- **ONE.51 [D]** Undo and redo are rows of kind `commit`, so ONE.3 and ONE.4 apply. A program cannot undo. It can offer the solver
  a command, and the solver acts.
- **ONE.52 [B]** A view has no workspace-write effect, so a view that runs at every revision cannot make undo stale (default 16,
  DT-L1 E3). A workspace change by a program happens inside a command the solver invokes, as one undoable action.
- **ONE.53 [D]** A puzzle undo makes a new state revision with an earlier digest. Read results may match it again under
  ONE.10, and permissions may not: a preview built before the commit stays ended. A workspace undo advances workspace
  and any affected protection or record versions under KN0.2 and KN0.8. It never rolls counters back. The one undo
  command still returns `committed`, with equal puzzle hashes when it undoes only workspace data (LI.26).

## 9. Crossing between the puzzles

- **ONE.54 [B]** Jumble on and Jumble off are plain commit rows with typed refusals (DT-M1 C6). Jumble off is allowed exactly at
  label checkpoints (A5). Jumble on replays, through the jumbling engine, the retained moves that 600-cell-Full's journal
  holds since the last crossing, starting from the stored exit poses. Labels alone are refused (A5).
- **ONE.55 [D]** To a program, a crossing is a commit. It makes a new revision with another `mode` and another level. The rules of
  ONE.49 apply, and a snapshot opened before it keeps its own level.
- **ONE.56 [B]** The stored exit poses serve the digest, the journal and replay, never a visible fact (A5). A program reads them only
  as an episode record, through query rows.
- **ONE.57 [D]** A value crosses only by `project` to a coarser level. A labels-level value never becomes a poses-level one, because
  the pose digest of a lift is not determined (KN1.10, ENG-0J section 7).
- **ONE.58 [B]** Recovery restores an interrupted episode as on, or rolls back to its last committed revision, never to a half-crossed
  state (ENG-0J section 4, KN4.5).

- **ONE.79 [D]** The exit-validity rule is H11 and JI.25 to JI.30 in
  `docs/progress/1.0/jumbling-interfaces.md`. Active validity needs the selected parent path, a retained-only suffix
  and no durable void; inactive is not deleted. Workspace actions join the durable chain without moving the puzzle
  path or state revision. The command resolves the exact episode, crossing and exit on that path before replay.
  Undo/redo and branch selection use the path and all void facts, never an equal label digest. H11's Position and
  Context are separate: journal navigation restores Position, keeping current preferences/protection except
  JI.27's recorded crossing-level protection change. FP.30 to FP.32 own the durable representation.
- **ONE.80 [D]** A crossing follows LI.36's non-preview commit path: shell token; commit-lane admission; current
  identity, mode, revision and full-state checks; A5's exact crossing/protection checks; one durable transaction;
  then publication. `exit` and `entry` are separate actions, each with its own before/after Position and actual
  Context. Entry records `exit_root(exit, crossing, suffix digest)` or `solved_witness(word)` under JI.25; no exit
  is required for the latter. The transaction writes JI.27's transition, identity pins, state/episode/head, changed
  records, void facts, counters and action order, plus command, request, admission, receipt and new RevisionRef.
  Off-lattice, nonzero-nu and unwitnessed refusals write none of these. No program supplies an end state. The receipt
  binding remains queryable after undo and epoch loss (LI.30); current state equality cannot replace it.
- **ONE.81 [D]** Undo resolves LI.9's ActionHint and that target transition's own before Position; redo validates its
  own after Position. Under the same gates, each restores head, mode, episode and exact state atomically under the
  pinned identity, with a new state revision and inverse/forward receipt. Current Context follows JI.27, never a
  wholesale historical assignment. Undoing exit poses P at h7 to labels L at h8 returns on at h7/P. If retained w
  then led to h9 and an entry at h10 replayed P,w, undo of that entry returns off at h9 after w, not on at h7/P;
  redo restores that entry's h10/episode/replay result. A solved-witness entry uses its own sides in the same way.
  Redo preserves the exit identity when one is used and refuses a now-void exit. No inverse lifts labels, restores
  old counters or revives a preview. Recovery follows JI.30 for whichever side was last durable. Branch navigation
  follows these journal steps with current Context (JI.28).
- **ONE.82 [D]** A reset, import or checkpoint restore goes through the commit lane after complete input validation.
  It records void facts for every earlier exit of the session lineage, active or inactive, in the same transaction
  as the replacement, actual Context, command/request/admission/receipt binding and recovery checkpoint;
  no response or bundle exposes a half-write. Restoring labels L at an earlier head while a valid exit has the same
  labels still voids that exit. A later Jumble-on request without a retained witness from solved gets
  `KN5:input.jumble_on_labels_only` under A5. Recovery reads the durable journal and void facts and preserves that
  refusal; equal labels never repair the chain. Voids survive undo of the break and selection of an old branch.
  Recovery checks continuity of Position and Context through workspace actions too. Before durability it recovers
  the preceding transaction, after durability the complete new one, for entry as well as exit; no half-crossed state
  exists. A durable admission with proved rollback and one with unknown fate follow LI.30's distinct outcomes.
  The reset/import preference-preservation rules still apply.

These clauses are about lattice configurations only, where labels are defined (KN1.7).

## 10. The view model and the adopted bundle

- **ONE.59 [B]** There is one view model with a mode. With Jumble off it serves the label path of 600-cell-Full unchanged, and with
  Jumble on it serves the pose path. The switch happens at a revision boundary on a lattice state, where both paths give the
  same sticker positions (ENG-0J section 4).
- **ONE.60 [B]** The engine publishes each revision as one bundle, and the view model installs a complete matching bundle atomically.
  Missing, mismatched or out-of-order data triggers a validated refresh, never a mixed frame (V03.3, render contract rule
  "One adoption per revision"). Change notification is advisory, so a client converges by reading the revision delta
  (V03.2).
- **ONE.61 [D]** The view model holds no puzzle copy. The partition's sentence means this: an adopted bundle is a read-only image of
  one revision, replaced whole. Nothing edits it, and nothing decides from it. Legality, protection and every verdict come from
  the engine. A drawing needs the arrays in that bundle, so a rule against holding them would be false.
- **ONE.62 [B]** The renderer draws no uncommitted state as the session state (H-08). It has two named exceptions, each drawn as what
  it is: a preview, from the engine's preview object, and a derived state, through a display plan that names its derivation
  and base. How they look belongs to the design line.
- **ONE.63 [D]** The runtime publishes view nodes, display plans and source maps, each with its base and declared dependencies
  (RPI.6, RPI.10). The view model attaches them only to an adopted revision whose declared dependencies match. Otherwise the
  view shows its stale state (ONE.12).
- **ONE.64 [D]** A bundle never waits for program output (DT-L1 E1). Program views skip to the newest revision, so a burst of twists
  queues no view work (KN7.2).
- **ONE.65 [B]** Global and Local are linked to one context and revision (charter D8). A program cannot make them show two revisions.
  Further views are allowed in every profile (owner, 6 November 2026, LQ6). Where a derived state appears inside Global
  and Local, and how it looks, stays with the design line.
- **ONE.66 [D]** A pick resolves against the revision it was made on, through a source map (chapter 10). A pick of an older revision
  falls under ONE.13.

## 11. The runtime at each layer

- **ONE.67 [D]** The runtime uses each layer only as this table says (partition section 9).

| Layer | The runtime uses | The runtime never |
| --- | --- | --- |
| Engine | Snapshot handles; batch queries; derived states; certificates; checkers | Calls the engine from the UI thread; holds its mutable state |
| Session store | Journal, revisions and checkpoints through query rows; workspace records through workspace commands | Opens the session files; writes directly |
| Command layer | `propose` with provenance; receipts; typed errors; availability; workspace and view rows | Bypasses a gate; invokes a commit row; supplies a state or digest to commit |
| View model | Publishes view nodes, display plans and source maps with their base | Holds a puzzle copy; delays the adoption of a revision |
| Shell | Is hosted by it for the editor and bindings; may share the checker as a library for editor latency | Runs a program on the UI thread; receives a token |
| Renderer | Nothing directly; plans reach it through the adapter | Decides legality; is called by a program; holds up a frame |

- **ONE.68 [B]** A turn's interval (charter E2) runs from the key through the command layer, the engine's apply, the store's durable
  transaction and the bundle's adoption to the frame. The runtime is not on it (default 15, DT-L1 E1).
- **ONE.69 [B]** The runtime runs in a process of its own (owner, 14 November 2026, LQ9; RTE.21). The engine-process
  and shell-process placements are not admitted. KN5.15, KN7.12, KN7.13 and every clause of this chapter hold in that
  placement. LX-2's fault-injection run is an LP3 acceptance check with its bar unchanged; if it fails, LQ9 reopens.

## 12. Failure and recovery

- **ONE.70 [D]** Each failure leaves the things of section 2 as this table says.

| Failure | Handles and derived values | Pending preview | Jobs | State and journal | The solver sees |
| --- | --- | --- | --- | --- | --- |
| The runtime dies | End with the runtime | Stays: the engine owns it, so the solver can still commit or cancel it | End as `error` (KN7.13) | Unchanged | A responsive shell, and the views' stale state |
| The engine restarts | End; the value store is not durable | Gone (ONE.43) | End; a new epoch begins | Recovered from the journal | A fresh preview is needed; drafts remain |
| A commit's outcome is unknown | Reads go on | Refused | Admitted only for reads | Read-only recovery (KN5.23) | The recovery state; never an automatic retry |
| A refresh fails after a commit | Unaffected | Discarded, as after any commit | As after any commit | Committed | "Applied, display out of date" (R-02) |
| The device is lost (R-17) | Unaffected | Stays, but nothing is presented, so the commit key is not live | Accepted view preparation follows LI.28; installed publications keep their completed receipts | Unchanged | The renderer's own recovery |

- **ONE.71 [B]** One command ID produces at most one commit. A receipt that says committed means already committed, even if a later
  undo changed the state, and it is never replayed automatically (V03.5, KN5.21).
- **ONE.72 [B]** A run a budget cut off, a cancelled job and a dead runtime cannot mutate the puzzle journal or state
  (INV-2). Unpublished workspace edits are discarded. A workspace action already durably completed keeps its success
  receipt; a later runtime fault does not undo or relabel it as failed (KN5.22, LI.17).

## 13. Branches

- **ONE.73 [B]** An experiment branch is a journal branch the engine owns (owner, 6 November 2026, LQ4; KN4.6). Exactly
  one line per session is active, and that line's head is the state of record (ONE.1).
- **ONE.74 [D]** A branch comes from the solver's own commits: the journal keeps a line when the solver commits after an undo
  (KN4.1). No program can create, switch or change a branch.
- **ONE.75 [D]** Switching the active line is a row of kind `commit`, and it makes a new revision like any other. ONE.49 applies to it.
- **ONE.76 [D]** A program reads another branch through `open_snapshot` and compares it through `compare`. Those are the only ways it
  sees one.
- **ONE.77 [D]** A branch and a derived state differ in what they prove. Every revision on a branch passed the gates, so the journal
  witnesses its reachability. A derived state has only its claim (KN2.13). Replaying a derived state's word on a branch is the
  solver's act, through the normal path.

## 14. What this chapter asks of other lines

Chapter 01's section 9 lists the kernel's needs. This chapter adds these:

| Item | Owner | Clause |
| --- | --- | --- |
| Apply the supplied token contract to every final commit row | EN4 table / p2a | ONE.4, LI.6 |
| Use the supplied dependency freshness predicate in the final `fresh_preview` gate | EN4 table / p2a | ONE.39, KN0.8 |
| `Base` with declared dependencies on every result | EN3, EN4 | ONE.8 |
| `compare` as a kernel row | EN3, EN4 | ONE.25 |
| A row that switches the active line | EN4 | ONE.75 |
| A report of the preview shown by the newest presented frame, drawn by the built-in chain | Renderer line, through the LQ20 handoff | ONE.41, RPI.66, RPI.76 |

## 15. Errors this chapter adds

Chapter 13 owns the final spelling. `KN1:input.level_not_held` and `KN5:busy.commit_in_flight` are chapter 01's codes; this
chapter reuses them.

| Code | Raised when | Continuation |
| --- | --- | --- |
| `KN5:stale.revision_changed` | A preview's `PreviewDeps` no longer match at commit | Preview again |
| `KN5:stale.preview_replaced` | The token names a preview that is no longer the pending one | Review the preview on screen |
| `KN5:stale.no_fresh_preview` | The commit key is pressed with no fresh preview presented | Preview the draft |
| `KN2:stale.epoch_changed` | A handle or a preview of an earlier engine epoch is used | Open a new snapshot or preview again |
| `KN5:stale.token_used` | A single-use token is seen a second time | Press the commit key once on the preview on screen |
| `KN5:stale.token_mismatch` | A genuine token's row, arguments, digest or identity differ from what it names | Review the preview on screen |
| `KN5:auth.token_invalid` | A token fails its authenticator | Renew the link to the engine, then preview again |
| `KN5:capability.commit_row_refused` | A call from the runtime names a commit row | Offer the action to the solver |
| `KN5:consistency.state_hash_mismatch` | Equal revisions have unequal digests | Internal failure: stop and report |
| `KN5:unknown_outcome.read_only_recovery` | A proposal or a commit arrives during read-only recovery | Wait for the recovery to end |

## 16. Conformance tests

| Test | Checks | Class |
| --- | --- | --- |
| CT-ONE-01 | An enumeration of the runtime's table finds no operation that takes a state or a digest to adopt | source |
| CT-ONE-02 | An enumeration of the runtime's table finds no row of kind `commit`, in any profile | source |
| CT-ONE-03 | A commit row without a token, or with one bound to other arguments, is refused. A live twist and a crossing commit with no preview object; a word commit with no presented preview is refused with `KN5:stale.no_fresh_preview` | source |
| CT-ONE-04 | Presented-frame binding: with B received and A still presented, the chord's token binds A, and the commit is refused | source |
| CT-ONE-05 | Revisited digest: commit, then undo; a token for a preview built before the commit is refused though the digest is equal | source |
| CT-ONE-06 | Read results: a cached result with equal inputs is shown after an undo, and it is charged as a miss | source |
| CT-ONE-07 | A long read keeps giving the same bytes while commits land | source |
| CT-ONE-08 | A derived state's word, proposed after the head moved, shows the effect at the current revision, and the derived state is not read | source |
| CT-ONE-09 | A raw table enters only through KN2.14, with reachability open or promised | source |
| CT-ONE-10 | `label_checkpoint` on a derived state with open reachability answers unknown, at ν = 0 | source |
| CT-ONE-11 | A commit row refused at each step of section 7 leaves every digest unchanged | source |
| CT-ONE-12 | Every state change of ONE.49, driven headless, acts on each thing as the table says | source |
| CT-ONE-13 | A blocked view leaves bundle adoption undelayed, and the view shows its stale state with its base revision | source |
| CT-ONE-14 | After a commit, no per-revision output of the old revision is drawn against the new one | source |
| CT-ONE-15 | A burst of twists queues no view work: the view lane holds at most one pending request | source |
| CT-ONE-16 | A mixed or out-of-order bundle is never installed and triggers a refresh | source |
| CT-ONE-17 | A crossing cancels jobs on the replaced revision, and a snapshot opened before it keeps its level | source |
| CT-ONE-18 | A comparison across levels with no projection is refused | source |
| CT-ONE-19 | The view model's switch at a crossing gives the same sticker positions on both paths (ENG-0J, E-2.4-0J item 2) | source |
| CT-ONE-20 | Runtime death leaves the pending preview committable | source |
| CT-ONE-21 | A restart ends handles and previews, keeps drafts and durable mutation receipts, returns unknown for lost transient outcomes, and revives no permission | source |
| CT-ONE-22 | Read-only recovery refuses proposals and commits, and answers reads | source |
| CT-ONE-23 | A program cannot create, switch or change a branch (ONE.74) | source |
| CT-ONE-24 | A pinned job finishes after a commit, and its result is not shown against the newer revision (ONE.19) | source |
| CT-ONE-25 | Charter E2 end to end, with the three LP3 tools' views active: p95 at most 100 ms over at least 100 observations | performance |
| CT-ONE-26 | The chord binds the presented preview under real frame pacing, in the selected renderer | windows |
| CT-ONE-27 | People can tell a derived state on screen from the session state, in a form the owner sets | human |
| CT-ONE-28 | A wordless import: reads, `compare`, display and `derive` work; the derived value keeps the import's open reachability claim, and no row proposes or adopts either value | source |
| CT-ONE-29 | A preview drawn through a user stage is an extra display with no presented-preview identity; the shell binds a commit token only to a presented frame drawn by the built-in chain (ONE.41, RPI.76) | source |
| CT-ONE-30 | A further view keeps its one-revision base, identity and source linkage, and opening it never makes Global and Local show different contexts or revisions (ONE.65, RPI.81) | source |

CT-KN5-01 is the preview-swap test itself. CT-ONE-04 extends it to the gap between receiving a preview and presenting it.

## 17. Open items

The owner answered LQ1 to LQ21 on 6 November 2026 and chose LQ8, LQ9 and LQ21 on 14 November 2026: the engine and the
interpreter's host in Rust, route RT1 with the runtime in a process of its own, and capability separation (RTE.17,
RTE.21, RTE.24). LQ13 waits for its evidence, and LQ2's option set waits for the joint setting.

- **The design line.** Where a derived state may appear inside Global and Local, and how it looks (ONE.62, ONE.65).
- **LQ9.** Answered: the runtime runs in a process of its own (owner, 14 November 2026, LQ9; RTE.21; ONE.69).
- **Renderer-line delivery.** The handoff row is settled under LQ20; the presented-frame report still needs its matching
  renderer evidence, using the built-in chain (ONE.41, RPI.66, RPI.76).
- **KN5.12.** Whether a program may replace a shown preview, and the options of ONE.42.
- **EN3, EN4.** Section 14.
- **New integrator rules the owner may overrule.** ONE.3 (no commit row for the runtime at all), ONE.4 (a token on every commit
  row), ONE.34 (previews only inside a solver's command), ONE.40 (the engine discards a preview when the state revision moves)
  and ONE.41 (on screen means presented).
- **Readings of the sources.** ONE.12 reads section 8.3's "shows the stale state" under rule 3, ONE.21 and EV3.12 read
  partition EV3's base revision for an imported value, and ONE.61 reads "holds no puzzle copy". If the owner reads any of
  them differently, the source or the clause changes.

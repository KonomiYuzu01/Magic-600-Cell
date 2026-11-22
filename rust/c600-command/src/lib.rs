//! The command layer of LP3 (chapter 01 KN5; H-08; EN4's layer 3), headless.
//!
//! Every change to a session goes through [`CommandLayer::invoke`] and its gates (KN5.1). The layer reads H-08's rows
//! from the stored table ([`table`]), returns a receipt for every invocation ([`receipt`]), runs the word preview as a
//! job, commits a presented fresh preview through the ten commit steps of chapter 08 section 7, and undoes and redoes
//! through `session.undo` and `session.redo` with LI.9's action hint. The shell ([`shell`]) mints the user-action
//! tokens from a per-launch secret the runtime never receives (KN5.14, KN5.15) and resolves key bindings before the
//! invocation envelope (EV4.3), so a key bound to a kernel row reaches the engine with no interpreter step (ONE.48).
//! This crate depends on the kernel only, `c600-kernel` and its jumbling half `c600-jumble` for KN3's exact rotations
//! and Jumble on's poses: it has no path to the language runtime at all.
//!
//! Step 9's durable transaction sits behind [`store::Store`], and [`CommandLayer::open`] replays a store's history
//! through the kernel ([`store::Reopen`]) to restore the revision, the protected set, the undo and redo history and the
//! durable receipts. LP2's session store implements both over KN4's journal (`c600_session::JournalStore`, which
//! depends on this crate and never the reverse); [`store::MemoryStore`] keeps the same contract in memory for the
//! tests and the oracle loop.
//!
//! # Owner decisions
//!
//! The owner decided these readings of the frozen text under PHS.52 on 16 November 2026
//! (`docs/wiki/decisions/owner-decisions-2026-11-16-phs52.md`). The frozen text itself changed with the amendment LP3
//! made at its command-table freeze (PHS.53).
//!
//! - Ruling 1: a press whose preview ended is refused `KN5:stale.no_fresh_preview` when the slot is empty and
//!   `KN5:stale.preview_replaced` when it holds another preview; a token for a preview never presented is
//!   `no_fresh_preview`. LI.33's vector, which answers `preview_replaced` after the pressed preview was committed, is
//!   amended to follow this ruling.
//! - Ruling 4 (KN5.21, KN5.23): the retry lookup keeps its frozen place before authentication (chapter 08 section 7),
//!   and the host stamps the caller from the channel the envelope came on. The envelope ([`Invocation`]) names no
//!   caller; [`CommandLayer::invoke`] takes the channel's caller beside it. Command IDs are scoped per caller (the
//!   shell, and each run on its own since finding G1's reading below; no host caller is built). A record is keyed by
//!   the scope and the ID, and the request digest
//!   covers the stamped caller (LI.5's amendment says so). A refusal of an envelope that does not authenticate (a bad
//!   authenticator, a missing token, or a caller that may not use the row or hold a token) is retained for the epoch
//!   under the envelope's full digest, authenticator included, which is its receipt's `request`; it never attaches to
//!   the command ID. Every other answer attaches to the ID. An envelope whose request digest equals a recorded one is
//!   answered as its retry whatever its authenticator, and commits nothing.
//! - EN4's layer interfaces (LI) are frozen with the architecture (owner, 15 November 2026), so a change to LI follows
//!   PHS.52. Where LI is silent, the layer's choices below are drafts.
//!
//! On 18 November 2026 the owner answered the readings that the KN5 boundary attack left open
//! (`docs/wiki/decisions/owner-decisions-2026-11-18.md`):
//!
//! - Finding B7: a reopen checks a stored commit and a stored release again ([`CommandLayer::open`]). A commit replays
//!   only from the shell's caller and derives under the protected set and the policy in force at its place in the
//!   journal; a protection change that releases an orbit, or the undo or redo of one, replays only with the
//!   confirmation stored with it. A journal action that fails either check leaves the layer in read-only recovery
//!   before it with `KN5:consistency.unadmitted_journal_action`, so the session does not open as an
//!   intact writable one. The journal carries no authentication: an edit that rewrites a program-made action as the
//!   solver's own is not detected.
//! - Findings B1 and L17 (KN5.12, ONE.42 option (a)): a run cannot replace or cancel a preview the solver is shown;
//!   only the solver's action ends it. While a frame presents the pending preview, a run's preview and a run's cancel
//!   of it are refused with `KN5:capability.preview_shown`, also for the run of a command the solver
//!   invoked.
//! - Finding B2 (ONE.34): a run prepares a preview only inside a command the solver invoked, and the one check is in
//!   the layer, not in each host. The run must be the run of a program job ([`ProgramJob::run`], `job-<id>`) that the
//!   host has taken and not yet reported, invoked by the shell or by a run that is itself inside such a command.
//!   Otherwise its preview is refused with `KN5:capability.outside_solver_command`. The check is on, so
//!   a host, LP3's included, must start its runs through a tool row the shell invokes.
//! - Finding G1: a command ID lives in its caller's namespace. Each run has its own scope (`runtime/<run>`), so one run
//!   cannot use up another run's IDs. The scope is derived from the stored caller, so the journal format is unchanged.
//! - Finding LD1 (EV4.22): a change of the protection policy needs the solver's own confirmed press, as a release of
//!   protection does, so no playback or run changes it (below).
//! - ONE.52: a run invokes no row that writes a workspace record, even a run that a solver command started (below).
//!   The records a workspace row's own job returns are written in that row's completion, and only the solver invokes
//!   that row.
//! - Finding B5 (KN1.11, JI.5; JI.27's alternative is not taken): protection holds inside Jumble, so a round trip
//!   through Jumble cannot move a protected orbit. A pose move is derived with the jumbling engine's pieces that carry
//!   the protected orbits as pose-protected pieces, and one that would move such a piece is refused with the engine's
//!   `KN2:protection.piece_would_move` (below). The verdict is the engine's own pose verdict, so no label verdict
//!   crosses into Jumble (chapter 03).
//!
//! # Drafts
//!
//! The command table is frozen at LP3 (PHS.53) and the IR freezes later; a choice below stays a draft until a frozen
//! text settles it.
//!
//! - One session per layer, 600-cell-Full only. Of the jumbling rows, `jumble.on`'s entry from solved, `jumble.off`,
//!   `jumble.twist.menu` and `jumble.repair.list` are built (below); the rest is refused with
//!   `KN5:input.row_not_built`.
//! - `WordRef` is an inline literal recipe in 0.4's JSON form, `{"recipe": [...]}`, and `ResolvedDraftRef` is
//!   `{"record": name}`, a workspace record that holds a word's letters (M6's form) or a work sheet (below); a draft
//!   argument in any other form is refused with `KN5:input.argument_form`. A record draft's digest is M6's
//!   `{"letters", "source", "reads"}`, with the record's content digest and version in `reads` (KN0.8).
//! - The workspace store ([`workspace`]) holds M6's records: `workspace.set` ([`RECORD_ROW`]) takes `record`, `payload`
//!   and `provenance`, the provenance block's canonical JSON as text. H-08 holds the row since the freeze (CM.17), and
//!   the layer refuses it to the runtime with `LG3:capability.not_granted`, as M6 does. The layer cannot
//!   read a provenance block: the host's [`Provenance`] reader checks it at the write (PS1.16) and, in a solving
//!   session, admits a record's word at `operation.preview` (KN5.26); without a reader both are refused. A record
//!   write and a workspace row's completion keep the part blocks the reader holds for the blocks they write
//!   ([`Provenance::parts`]) with the action, and admission gives them back ([`Provenance::admit_kept`]), so a restored
//!   workspace keeps a word's recursive parts (BND.24, BND.85, CT-BND-27; draft).
//! - LB7.2's work sheets ([`worksheet`]; CT-LB2-03's work-sheet part), three rows of H-08:
//!   - A work sheet is a workspace record whose payload is `{"worksheet": {...}}` ([`Worksheet::to_payload`]): the
//!     phases Prepare, Macro and Cleanup, the named roles (a piece, named by its home position, or a position), the
//!     reference as the draft gave it, and the piece roles' placements. A draft `{"record": name}` of a sheet previews
//!     its complete operation ([`RecordValue::operation`]).
//!   - `worksheet.save` ([`WORKSHEET_SAVE_ROW`]) takes a draft `{"record": r}` that holds letters or a sheet, and a
//!     `name`. A recipe draft is refused with `KN5:input.argument_form`: the sheet keeps the draft record's provenance
//!     block, and the layer cannot make one. The save is undoable as a workspace inverse, as LB7.2 orders for every
//!     workspace change and as H-08's `undo: workspace` reads since the command table's freeze (CM.17).
//!   - `worksheet.load` ([`WORKSHEET_LOAD_ROW`]) takes H-08's `WorksheetRevisionRef` as `{"record": name, "version":
//!     n}` ([`WorksheetRef`]); a record at another version is `KN5:stale.revision_changed`. It places the piece roles
//!     again and reads each position role's occupant again at the head, keeps where each piece was and what each
//!     position held at the save, and writes the sheet again with its phases unchanged, so a reuse makes every preview
//!     that read the sheet stale, also at the same state, and the reuse needs a fresh preview. A changed occupant
//!     ([`Occupancy::changed`]) is shown beside a moved piece ([`Placement::moved`]): neither silent nor a refusal,
//!     and nothing is retargeted (U1-057, U4-037, LB7.2).
//!   - A sheet saved from a record of letters keeps the record's name, version and content digest as its macro
//!     version ([`MacroVersion`]); a sheet saved from a sheet keeps the sheet's.
//!   - `worksheet.compare` ([`WORKSHEET_COMPARE_ROW`]) takes the `WorksheetRevisionRef` and a draft `{"record": r}`,
//!     the current work. Its job runs in the layer on the exclusive lane ([`CommandLayer::run_jobs`]), since no entry
//!     can serve that lane: at the head it compares the sheet's phases, roles, reference and macro version with the
//!     draft's ([`compare_inputs`]), places the piece roles and reads the position roles' occupants again, names the
//!     moved pieces and the changed occupants, reports the macro version as current, changed or
//!     missing, and analyses the sheet's complete operation afresh under the protection and policy in force, with
//!     LB2's gains and losses (the positions whose home piece becomes solved, and the reverse). The comparison is the
//!     completion's result ([`CommandLayer::comparison`]); it grants nothing, and a reuse still needs `worksheet.load`
//!     and a fresh engine preview (LB7.2).
//!   - The three rows need the host's provenance reader (`KN5:input.row_not_built` without it) and Jumble off, and the
//!     runtime may invoke none (`LG3:capability.not_granted`). No entry can serve them.
//! - `view.open` and `view.close` ([`view`]; EV3.2, RPI.81 to RPI.83; draft). The layer admits them and the host's
//!   view model ([`Views`], given with [`CommandLayer::set_views`]) holds the views and RPI.82's budget. The argument
//!   forms, an overlay source's row and the caller come first (`KN5:input.argument_form`,
//!   `LG3:capability.not_granted` for a run not inside a command the solver invoked); an admitted open or close is
//!   `accepted` with no job. Without a view model both rows are `KN5:input.row_not_built`.
//! - `layout.undo`, `layout.redo` and `explore.set` ([`layer`]; ADR-D01.14, ADR-D01.39; owner, 22 November 2026, item
//!   4; draft): the layout history beside the move history, and the explore state that pauses every other commit row.
//! - `session.undo.partial` ([`layer`]; ADR-D01.15; draft): a commit that steps back inside the head's journal entry
//!   to one of its moves, journaled as a retained move that names the entry and the move
//!   ([`CommandLayer::head_entry`] reads its target).
//! - Every row this crate does not build yet is refused with `KN5:input.row_not_built`. The loop uses the operation
//!   rows, which are kernel rows too.
//! - The session's profile ([`Profile`]: `solving`, `tool_authoring` or `research`; BND.14, BND.15, BND.17, KN5.26):
//!   - The host sets it when it creates the session ([`CommandLayer::new_with_profile`]; [`CommandLayer::new`] is
//!     `solving`). Every ordinary session is `solving`, and so is the practice copy until LQ2 answers BND.15. No
//!     program or row sets or changes it, and a host sets a run's profile from [`CommandLayer::profile`].
//!   - The session's start records it once ([`Genesis::profile`]), and a reopen reads it back. A journal's session
//!     record without it reads as `solving`, so journals written before it stay valid.
//!   - KN5.26 applies only in a solving session: there the host's reader admits a record's word at
//!     `operation.preview`, and a saved sheet's word at `worksheet.compare`. Elsewhere no reader is asked, and
//!     BND.61's check comes when such a record is previewed in a solving session. The reader's check at
//!     `workspace.set` (PS1.16) runs in every profile.
//!   - BND.16's table of the rows a run may invoke by profile follows chapter 07 section 8. In `tool_authoring` and
//!     `research` a run may not invoke `operation.preview` (`LG3:capability.not_granted`; BND.60: a run's `propose`
//!     there goes to a capture). The cells that grant a run more outside a solving session, records of the authoring
//!     workspace and research records, are not built. The shell's rows are the same in every profile.
//!   - M6's `Session` (`c600-session`) keeps no profile.
//! - `twist.apply` ([`TWIST_ROW`]; A6, ONE.46, ONE.78), a live twist with no preview.
//!   - The argument form. `cap` is KN3.7's cap, an integer from 0 to 599. `rotation` is KN3.7's binding without its
//!     cap: exactly `request`, `operation` and `menu`. The row's `cap` completes it to the canonical binding, so the
//!     request digest and the token's argument digest cover the whole binding. A malformed binding, a `rotation` that
//!     is not an object of exactly those fields included, is refused with `KN2:input.rotation_request_malformed`.
//!   - The host pins the twist menu with its exact model ([`CommandLayer::pin_menu`]). Without a pin the row is
//!     refused with `KN5:input.row_not_built` before its arguments are read. The pin is not journaled.
//!   - The request resolves against the pinned menu (KN3.8). A free binding is refused with
//!     `KN2:input.menu_binding_mismatch`, since the session pins a menu (free mode is research-only, JI.35), as is a
//!     menu pin other than the pinned menu's. A direct rotation off the menu is `KN2:input.twist_off_menu`. A jumble
//!     item is `KN5:input.menu_word_needs_jumble_on`, and the identity item, which moves no piece,
//!     `KN2:input.recipe_shape`. These checks come before the token, so they spend no press.
//!   - A retained item commits its shortest retained word (`c600_jumble::rotation::retained_word`) as a one-word
//!     recipe, derived under the protection policy in force, so Strict judges every prefix of the word. The token binds
//!     the state and protection revisions (LI.6): a twist whose token names an older revision is refused with
//!     `KN5:stale.revision_changed`, so a press queued behind another commits nothing.
//!   - The commit is a node of the move tree like a word commit: undo and redo derive the word's inverse or the word
//!     again. It ends the pending preview and cancels the jobs that read the replaced revision, except file jobs
//!     (LI.32).
//!   - The journal keeps the resolution's record as `rotation` (`Transaction::rotation`) beside the recipe, on a twist
//!     and only there. A reopen replays the recipe and runs neither the producer nor the chooser again (KN3.9), so it
//!     needs no pinned menu.
//!   - Drafts for LP3's amendment list (PHS.52): the argument form, the input refusals before the token, the identity
//!     item's code and the journal key `rotation`.
//! - `jumble.on` ([`JUMBLE_ON_ROW`]; A5, ONE.54, ONE.55, ONE.80, JI.25, JI.27), the entry into Jumble on ([`jumble`]).
//!   - The row takes `revision`, KN0.2's `RevisionRef`, and a token that binds the state and protection revisions
//!     (LI.6), as `twist.apply`'s does. It needs the pinned menu: without one it is refused with
//!     `KN5:input.row_not_built` before its arguments are read. The `revision` argument's form comes next, before the
//!     token (`KN5:input.argument_form`).
//!   - ONE.80's order follows: the token, the commit lane, the token's binding to the invocation and the identity block;
//!     the mode; the named revision and the token's revisions (`KN5:stale.revision_changed`); the head hash; A5's
//!     witness; one durable transaction; the adoption. With Jumble on already, `jumble.on` is refused with
//!     `KN5:input.retained_tool_needs_jumble_off`, since an already-on position needs no new entry.
//!   - The witness (JI.25, JI.26). An exit is active while the move tree's path from the head climbs through retained
//!     commits only to it; a pose move cannot supply that path, and no void is built. With an active exit the
//!     witness is its root: the retained suffix, the kernel's literal expansion of every commit from the exit to the
//!     head, replayed from the exit's own poses (FP.32; the empty suffix FP.31 admits keeps them). Poses made
//!     under a menu with another identity block are refused with `KN1:input.identity_mismatch`, and a suffix whose
//!     replay does not reach the current labels with `KN5:input.jumble_on_labels_only`, with no fall-back to the
//!     solved witness, which applies only when no exit is active. With no active exit the witness is JI.25's solved
//!     witness: the chronological literal retained path from the session's start to the head. The start must be
//!     solved: a session that started elsewhere holds no retained word from solved, even at a solved head, and is
//!     refused with `KN5:input.jumble_on_labels_only`, as is a word whose replay does not reach the current labels.
//!     Either replay runs in the jumbling engine under the pinned menu, with Net protection and no protected piece,
//!     and the entry enters at its poses. A suffix's derivation nests on the exit's poses, so at the 64th chained
//!     derivation the layer replays the jumbling journal flat from solved instead, as for a pose move. No protection
//!     gate applies: H-08 gives the row `model_match` only, and a crossing moves no piece.
//!   - The crossing is a node of the move tree that holds no retained recipe. The revision's mode becomes `jumble_on`
//!     ([`layer::JUMBLE_ON_MODE`]), its state revision advances and its digest is the pose digest
//!     ([`CommandLayer::jumbled`] is the head's value). Like a commit, it answers `committed`, ends the pending preview
//!     and cancels the jobs that read the replaced revision, except file jobs.
//!   - With Jumble on, `operation.preview`, `operation.execute` and `twist.apply` are refused with
//!     `KN5:input.retained_tool_needs_jumble_off` (LG2.8): a preview before its draft is read, a commit after its token's
//!     binding. `jumble.twist.menu` and the protection, policy and record rows stay; their `pre` and `post` are the
//!     pose digest. A program job's revision names the mode, so a built-in tool decides for itself.
//!   - An undo of the entry returns to the exact Jumble off state before it, after any retained suffix and not at the
//!     exit's labels (JI.29), and a redo makes the same entry again from the same witness, which must reach the
//!     entry's poses and record, under a pinned menu with the identity block the entry was made under
//!     (`KN1:input.identity_mismatch` otherwise; ONE.81, JI.27, FP.31). Each advances the state revision. While Jumble
//!     is on, a pin of a menu with another identity block is refused with `KN1:input.identity_mismatch`.
//!   - The journal names the action's kind `crossing`. It keeps the crossing's record (`Transaction::crossing`): from
//!     the solved witness, the entrance `solved_witness`, the jumbling identity block, and the witness's letter count
//!     and `Digest<literal word>`; from an exit root, the entrance `exit_root`, the exit's event as `crossing`, the
//!     SHA-256 of the exit's record in canonical JSON as `exit`, the jumbling identity block, and the suffix's letter
//!     count and `Digest<literal word>` (FP.31's `exit:Ref`, `crossing:Text` and suffix digest). The letters are the
//!     move tree's. A reopen with [`CommandLayer::open_with_menu`] replays the witness and must make the same record;
//!     [`CommandLayer::open`] leaves the layer in read-only recovery before a crossing.
//!   - Drafts for LP3's amendment list (PHS.52): the journal kind and key `crossing` and both record forms, the
//!     refusal of `jumble.on` with Jumble on, the order of the mode check before the revision checks, the pin rule,
//!     the identity refusal of an exit root made under another menu, and no fall-back to the solved witness while an
//!     exit is active.
//! - `jumble.off` ([`JUMBLE_OFF_ROW`]; A5, ONE.54, ONE.80, JI.27), the exit from Jumble on ([`jumble`]).
//!   - The row takes `revision` and a token, as `jumble.on` does, needs the pinned menu (`KN5:input.row_not_built`)
//!     and checks the `revision` argument's form before the token (`KN5:input.argument_form`). ONE.80's order follows
//!     as for `jumble.on`; with Jumble off already, `jumble.off` is refused with `KN5:input.menu_word_needs_jumble_on`.
//!   - A5's label checkpoint decides (`c600_jumble::levels::label_checkpoint`): every piece on the lattice, every
//!     centre in its own chamber, ν = 0, and the head's poses witnessed from solved by the jumbling journal. ν comes
//!     from the identity labels or a retained witness where they show it, and from the ν row otherwise. A refusal
//!     names the fact only and selects no correction: `KN5:input.jumble_off_pieces_off_lattice` for a piece off the
//!     lattice, `KN5:input.jumble_off_nu_nonzero` for a nonzero ν, and `KN5:input.jumble_off_not_shown_reachable`
//!     for a displaced centre, two pieces in one chamber or poses not shown reachable.
//!   - The exit is a node of the move tree that holds no retained recipe and has the exit's label digest. The poses'
//!     labels become the retained state, the revision's mode becomes `jumble_off` ([`layer::MODE`]), its state
//!     revision advances and its digest is the label digest. Like a commit, it answers `committed`, ends the pending
//!     preview and cancels the jobs that read the replaced revision, except file jobs. No protection gate applies:
//!     H-08 gives the row `model_match` only. The protected set and the policy stay.
//!   - The retained rows work from the exit's state, and the jumbling journal runs on through the exit, which moves
//!     no piece. While only retained commits follow it, the exit stays active, and `jumble.on` enters from its root.
//!   - An undo of the exit returns to the exact poses before it, Jumble on, with the retained state the entry left;
//!     a redo checks the checkpoint again, which must reach the exit's recorded labels and record (JI.27). Each
//!     advances the state revision. Reset, import and restore, which would void exits, are not built.
//!   - The journal names the action's kind `exit` (FP.31; [`TransactionKind::Exit`]) and keeps the exit's record as
//!     `crossing`: the episode (the entry's event), the jumbling identity block, the jumbling journal's letter count
//!     and `Digest<literal word>`, the pose and label digests, and A5's certificate (ν, its source, the C5
//!     convention, the frame agreement, the certificate program's SHA-256 and the contract revision). A reopen with
//!     [`CommandLayer::open_with_menu`] replays the journal to the exit, checks the checkpoint again and must make
//!     the same record.
//!   - Drafts for LP3's amendment list (PHS.52): the journal kind `exit` and the record's form under the key
//!     `crossing`, the three refusal codes, the code of `jumble.off` with Jumble off, and the absent protection check.
//! - `jumble.twist.menu` ([`JUMBLE_TWIST_ROW`]; JI.35, FP.31, JI.27, ONE.78), a live menu twist with no preview.
//!   - The argument form. `cap` is KN3.7's cap, as for `twist.apply`. `letter`, H-08's `PinnedMenuLetter`, is KN3.7's
//!     binding without its cap, exactly `request`, `operation` and `menu`, the form of `twist.apply`'s `rotation`, so
//!     the request digest and the token's argument digest cover the whole binding. `revision` is KN0.2's
//!     `RevisionRef`, as for `jumble.on`, and the token binds the state and protection revisions (LI.6).
//!   - Before the token, in this order: the pinned menu (`KN5:input.row_not_built`), the `revision` argument's form
//!     (`KN5:input.argument_form`), the binding (`KN2:input.rotation_request_malformed`) and its resolution against
//!     the pinned menu (`KN2:input.*`, as for `twist.apply`), the identity item in either mode
//!     (`KN2:input.recipe_shape`), and with Jumble off a jumble item (`KN5:input.menu_word_needs_jumble_on`). Then the
//!     token, the commit lane and the token's binding; the named revision and the token's revisions
//!     (`KN5:stale.revision_changed`); the head hash (`KN5:consistency.state_hash_mismatch`); the move; one durable
//!     transaction; the adoption, which ends the pending preview and the jobs that read the replaced revision, except
//!     file jobs (LI.32).
//!   - With Jumble off a retained item dispatches to the retained action: its retained word is derived under the
//!     protection set and policy in force (`KN2:protection.orbit_would_move`) and committed as `twist.apply` commits
//!     it, a `commit` in the journal with the row's name and its `rotation`. Its undo and redo are a word commit's.
//!   - With Jumble on every item, a retained one too, is a pose move (JI.35): the item's exact twist is derived in the
//!     jumbling engine from the head's value (`KN2:input.word_blocked_prefix` for a blocked twist). The journal names
//!     its kind `jumble_move` (FP.31; [`TransactionKind::JumbleMove`]) and keeps its `rotation`, its literal is the
//!     engine's `Digest<literal word>` of the twist, and its `pre` and `post` are pose digests. The move is a node of
//!     the move tree that holds the twist; the state revision advances and the mode stays.
//!   - Pose protection (finding B5, the owner's reading of 18 November 2026). A pose move is derived under the policy
//!     in force with the jumbling engine's pieces that carry the protected orbits as pose-protected pieces
//!     (`jumble::protected_pieces`: both models read `slot_piece` and `orbit_id` from `assets/model.npz`, so the pieces
//!     of orbit o are those whose `orbit_id` is o). One that would move such a piece is the engine's
//!     `KN2:protection.piece_would_move` and changes nothing. The engine judges the pieces at the pose level
//!     (`m_A(w) = 1`, KN1.11), which is stricter than the label verdict, so no label verdict crosses into Jumble
//!     (chapter 03: no label-to-pose transport) and a round trip through Jumble moves no protected orbit. From solved,
//!     each item at grips 0 and 1 of the pinned menu moves pieces of all 35 orbits, so while an orbit is protected such
//!     a pose move commits only after the solver releases the orbit, with a confirmation (LI.26). A replay of poses the
//!     session already holds (an entry's witness, an exit's suffix, the flat replay of the journal) judges no piece:
//!     each move in it was judged when it was made. A reopen derives a stored pose move as the live move does, under
//!     the protection in force at its place in the journal, so one that would move a protected piece leaves the layer
//!     in read-only recovery before it with the engine's code.
//!   - The jumbling engine nests each derivation's reachability evidence in the next, and a chain of some hundreds of
//!     derivations overflows a thread's stack. After 64 chained moves the layer replays the jumbling journal flat from
//!     solved instead (the retained letters of the path's commits, then the pose moves' twists), which must reach the
//!     chained poses. Such a replay costs time linear in the journal.
//!   - An undo applies the exact inverse of the move's twist when the menu holds it and replays the journal to the
//!     move's parent otherwise; a redo applies the twist again. Each must reach the recorded poses of its target
//!     (JI.27), and each advances the state revision.
//!   - A reopen with [`CommandLayer::open_with_menu`] replays each jumble move's recorded exact twist (KN3.9), which
//!     must be an item of the pinned menu, and must reach its recorded poses; [`CommandLayer::open`] stops before the
//!     crossing already.
//!   - Drafts for LP3's amendment list (PHS.52): the `letter` form, the journal kind `jumble_move`, the chain bound
//!     and the flat replay, the undo by the exact inverse, the identity item's code in both modes, and the order of
//!     the checks.
//! - `jumble.repair.list` ([`REPAIR_LIST_ROW`]; BND.2, the 17 October exception; JI.12, JI.33), the repair list at a
//!   cap the solver names ([`repair`]; CT-LB3-05's and CT-BND-17's command-layer half).
//!   - H-08's arguments: `cap` is a `CapId`, 0 to 599, and not optional, so an invocation without one is refused with
//!     `KN5:input.argument_form` and no job starts for an unnamed cap (BND.64). `max_chunk` is a `PositiveNat`; the
//!     headless layer has no stream, so it bounds nothing yet and the list is delivered whole. `snapshot` is KN0.2's
//!     `RevisionRef` and must name the current revision; a derived state reference is not built.
//!   - Admission, queuing nothing on a refusal: the pinned menu (`KN5:input.row_not_built`), Jumble on
//!     (`KN5:input.menu_word_needs_jumble_on`: the registry has no code of its own for a jumbling query with Jumble
//!     off), the forms, the snapshot's revision (`KN5:stale.revision_changed`), in a solving session a run that is not
//!     inside a command the solver invoked (`LG3:capability.not_granted`; the checker's repair-list exception holds the
//!     run's cap to the one the solver named), and the exclusive lane (`KN7:busy.lane_full`).
//!   - The job runs in the layer on the exclusive lane ([`CommandLayer::run_jobs`]) and completes as a query. Its
//!     result is the kernel's list (`c600_jumble::observe::cuts::repair_list`) whole, with one field added,
//!     `statement`: a completed list with no rows says that no pair at its cap lowers N, as BND.2 words it, and a
//!     cancelled or cut-off list says that it makes no claim. A blocked or closed cap is the kernel's `error` status
//!     with no outcomes, passed on as data. The list is kept ([`CommandLayer::repair_list`]) until an action moves the
//!     state, and a state change cancels a queued list (JI.33), which never blocks undo. Nothing chooses, ranks or
//!     reorders a pair, and H-08's row has no key and does not repeat.
//! - KN4.3's journal query ([`CommandLayer::journal`], served as the `journal` read row; LP3's C8): a page of the move
//!   tree's events on one line, newest first, from a named event (`e<order>`) or the head toward the start
//!   ([`journal`]). Each entry gives its kind (`start`, `commit`, `crossing`, `pose` or `exit`), a commit's
//!   retained letters, whether the start is solved and an exit's mark, the SHA-256 of its exit record's canonical
//!   JSON, which `jumble.off`'s node keeps since its change leaves `changes` with the redo stack. A page holds 1 to
//!   [`JOURNAL_PAGE_LIMIT`] events; its cursor binds the session, the walk's `from` and the next event by an HMAC
//!   under the launch's secret and grants nothing. It answers with Jumble on or off and for every epoch, and an
//!   event the move tree does not hold is `KN2:stale.revision_gone`, as M6's store refuses a revision it does not
//!   hold.
//! - Codes chapter 13 does not register yet: `KN5:input.unknown_row`, `KN5:input.argument_form`,
//!   `KN5:input.row_not_built`, `KN5:input.overlay_row_conflict`, `KN4:input.session_exists`,
//!   `KN5:input.saved_set_exists` and `KN2:consistency.star_certificate`.
//! - The token travels beside the arguments in the envelope, not inside them, so it can bind the absolute arguments
//!   (LI.6). Its binding enters the request digest; its authenticator does not (LI.5).
//! - Jobs run when the host calls [`CommandLayer::run_jobs`]; `invoke` answers `running(job)` at once (KN5.2). The
//!   exclusive lane holds one job; a program lane's capacity is a host setting (LI.22), one by default. A view
//!   overlay's newer request supersedes a queued older one.
//! - A token is spent when it authenticates, so a refused commit consumes its press.
//! - The envelope carries its originating epoch (LI.5). A command ID the layer knows is answered by its outcome
//!   lookup, across a restart too; an unknown envelope from another epoch is refused with `KN2:stale.epoch_changed`,
//!   and when it authenticates its command ID stays used, so a new press takes a new ID.
//! - [`CommandLayer::present`] reads the adapter's RM.26 reports ([`frame`]) in the order of their stamps, by device
//!   epoch, then sequence. A report that is not newer than the newest is refused and changes nothing. The pending
//!   preview is presented while the newest frame shows it through the built-in chain, and a frame that shows no
//!   preview, another preview or a user stage's drawing (RPI.76) ends the presentation. A word commit's token binds the
//!   stamp of the frame that presented its preview, and the commit is refused with `KN5:stale.no_fresh_preview` unless
//!   that frame belongs to the preview's current presentation: on one device epoch, surface and surface generation,
//!   from the first frame that showed it to the newest. A restart forgets the frame reports. [`HeadlessFrames`] stamps
//!   the frames of a headless adapter.
//! - A program may replace the pending preview: its proposal runs inside a command the solver invoked (KN5.12 stays
//!   open; LI.38's working form).
//! - `operation.preview.cancel` changes no revision.
//! - Undo and redo walk M6's move tree through the original permission path (LI.26): a move's inverse is derived
//!   through the kernel under the protected set in force, from a head whose hash matches its node; the undo of a
//!   protection change restores the set before it and needs `confirm` when it releases an orbit. After an undo the
//!   revision's head is the node the undo returns to, not the undo's own event (KN5.21).
//! - Draft ruling 2: an undo or redo checks its hint before its bound revisions, so a press raced by any durable action
//!   is `KN5:stale.undo_superseded`.
//! - Draft ruling 3: no receipt and no lookup answers `accepted`. A record is written once the gates answered, and a
//!   refusal never overwrites a stored record.
//! - Draft ruling 5: a token whose authenticator fails is `KN5:auth.token_invalid`; one that verifies but names another
//!   session or epoch is `KN5:stale.token_mismatch`.
//! - Draft ruling 7: `session.checkpoint.save` follows `command-table.json`, a commit row that takes a token; it is not
//!   built yet. M6's `session.checkpoint` takes no token.
//! - A commit token binds the SHA-256 of the kernel's identity block ([`identity_json`]) and the preview's
//!   dependencies (KN0.8); an undo or redo token binds the state and protection revisions and the hint (LI.6).
//! - `confirm` is an optional boolean argument of a row with the `confirm` gate or an action guard.
//! - A new session starts in epoch 1. Every open and every restart draws a fresh random epoch of at least 2 that the
//!   session never had, also under an injected secret (L10). The shell's command IDs are `prefix/n` in epoch 1 and
//!   `prefix/epoch/n` after it, so a new shell never meets an earlier epoch's IDs; two shells with one prefix in one
//!   epoch would collide. Preview and job IDs restart after a reopen; the epoch in every binding keeps them apart.
//! - [`CommandLayer::outcome`] is the shell's lookup and answers `None` for an ID the shell never used in this layer's
//!   life; [`CommandLayer::kernel_outcome`] is `kernel.outcome` for any caller.
//! - `protection.orbit.set` is a workspace row with no token; the runtime may not invoke it.
//! - Strict protection in the command layer (KN2.4, KN2.7, LB2.3). `protection.policy.set` ([`POLICY_ROW`]) takes
//!   `policy`, `net` or `strict`, as H-08 has it; the session starts Net ([`CommandLayer::policy`]). Under Strict a
//!   preview is built with the kernel's `derive_strict`, step 8 judges the preview's recipe again with
//!   `strict_protection`, and an undo or redo derives its path with `derive_strict`, so every executed literal prefix
//!   is judged; a Strict preview's summary adds `mode` and `first_refused_prefix`, and a Net preview's summary is
//!   unchanged. The row is a workspace row with no token. A change in either direction needs `confirm: true`
//!   (`KN5:input.confirmation_required` otherwise; finding LD1's reading). H-08 gives the row no gate, so the layer asks
//!   for the confirmation itself; an undo or redo of a policy change needs none. The runtime may not invoke it. A policy
//!   change advances the workspace and protection revisions, so a
//!   pending preview built under the old policy is stale (H-08's note), and it is undoable as a workspace inverse. The
//!   journal names it `policy`, and it and its undo or redo carry the policy after the action
//!   (`Transaction::policy`).
//! - An action that does not move the puzzle (a protection or policy change, a record write, the undo or redo of any
//!   of them, a workspace completion) keeps the state revision, the preview slot and the jobs (KN5.18).
//! - A record write answers `completed(workspace)`, and its undo and redo answer `committed` with equal puzzle hashes,
//!   advancing the workspace revision and the record's version (KN5.20). An undo of a record's first write leaves the
//!   record with no value. The journal keeps the record's value after each of these actions (`Transaction::record`).
//! - A workspace overlay's completion is one durable action with its receipt (KN5.23). Only a stored row whose undo
//!   class is `workspace` makes it undoable (filter previews, below); a language-defined row's is not. A rolled-back
//!   transaction's `failed` receipt is transient.
//! - A run reaches no workspace-kind row (findings B3 and B4 of the KN5 boundary attack). Of the rows an entry
//!   runs, an overlay row or a served row, the runtime may invoke a query or view row only. A workspace or file row is
//!   refused with `LG3:capability.not_granted`, as `workspace.set` is, before read-only recovery, the arguments and
//!   the row's own gates are checked: no job is admitted, nothing is journaled and no history is cut. So a run alone
//!   causes no durable action, and a run's write never takes the solver's record writes out of the undo order. A run
//!   that a solver command started reaches no workspace row either: a program prepares previews and views, and the
//!   solver saves (ONE.52; the owner's reading of 18 November 2026). The records a workspace row's job returns are
//!   written in that row's completion, the solver's own action (job output, below), not on the run's channel.
//! - A built-in tool's entry may serve a stored row ([`CommandLayer::serve_row`]; LP3's C6, its first common need).
//!   - An invocation of a served row admits a program job for the entry, as for an overlay row.
//!   - The row keeps H-08's kind, lane, undo class, gates and argument types. The runtime may invoke it only when it
//!     is a query or view row, as for an overlay row.
//!   - A row on the `none` lane has no occupancy limit (KN7.2), so a full program lane does not hold it back, and
//!     nothing in the layer bounds how many of its jobs wait. A commit still ends its job, as it ends an overlay
//!     row's (KN5.18), although KN7.2 gives the `none` lane no background work to cancel. Whether such a row should
//!     count against the program lane, and whether a workspace entry that reads no snapshot should keep its job
//!     across a commit as file jobs do, is LP3's call.
//!   - H-08 marks the representations tool's rows `job: false`. A served row still answers `running` first, since
//!     its entry runs in the runtime's process (LQ9), and completes when the host reports the entry's result.
//!   - The layer checks the arguments' forms, as below.
//!   - The layer does not read a row's `contexts`, for a served row as for any other: the shell's key route resolves
//!     a binding in the focused context.
//!   - The layer serves only a view, workspace, query or file row on the `none`, `view` or `program` lane, with no
//!     action guard or token, whose undo class is `none` or, on a workspace row, `workspace`, and with no gate or
//!     preview rule except `filter.apply`'s pair (filter previews, below). It can check such a row's admission and
//!     completion without the entry.
//!   - Any other row is refused with `KN5:input.overlay_row_kind`. A row the layer builds itself, or one already
//!     served, is refused with `KN5:input.overlay_row_conflict`.
//!   - A served workspace row completes like a workspace overlay, as above.
//!   - Serving is not journaled. After a reopen the host serves its rows again, as it registers its overlays again.
//! - Argument forms (LP3's C6, its third common need).
//!   - An argument of an optional type, one ending in `?` (H-08), may be left out of any row. Every other argument
//!     must be given, and no other name may be. None of the layer's own rows takes an optional argument. The names
//!     are checked before the row's arm, so a row the layer does not build is refused with `KN5:input.row_not_built`
//!     once its names fit, with an optional argument left out or given.
//!   - For a row it admits as a program job, a served row or an overlay row, the layer checks the forms of the types
//!     it knows before it admits the job. `Nat` is an integer of at least 0. `Text` is a string. A choice of words,
//!     such as `body|complete`, is one of its words. `CellId` is an integer from 0 to 599, as the kernel's model
//!     numbers the cells, and `CellIdPair` is a list of two. `RevisionRef` is KN0.2's nine-key object
//!     ([`Revision::to_json`]): its text fields are text and its counters naturals, in any mode. An optional type has
//!     its type's form when given.
//!   - A malformed argument is refused with `KN5:input.argument_form`. After every form check, a well-formed
//!     `RevisionRef` that is not the layer's current revision in every field is refused with
//!     `KN5:stale.revision_changed` (EV4.14), so a malformed request is never answered as stale. Both refusals come
//!     before the lane's capacity is checked and take no job.
//!   - `FilterRef` is a result reference `{"job": Nat, "digest": Text}` and `FilterPreviewRef` `{"epoch": Nat, "job":
//!     Nat}`, a job and the epoch it ran in (draft). After the forms and the revisions, a `FilterRef` must resolve
//!     (filter previews, below).
//!   - `CanonicalObjectRefs`, which `set.save` takes, is a list of pieces or positions, each 0 to 177,119 (the
//!     kernel's `PIECES`); an empty list and repeated members pass (draft). `set.save` also takes the optional boolean
//!     `replace` (`Bool?` since the freeze, CM.17), and its completion writes the saved set's own record ([`sets`]).
//!   - The forms of the other types are not written down yet, and they pass unchecked: among them a union of types
//!     such as `ResolvedDraftRef|WordRef`. The layer's own rows check their arguments where they are built.
//! - Overlay availability (KN5.7, LI.18 to LI.21; CT-KN5-08). [`CommandLayer::declare_overlay`] stores a
//!   language-defined row's [`Metadata`]: the static check's first failed diagnostic, and the kernel rows its program
//!   may invoke, each required or optional and bound [`Binding::Static`] with captured arguments or
//!   [`Binding::Computed`]. [`CommandLayer::availability`] answers such a row from that data and the layer's state
//!   alone, in LI.20's order. No language code runs, no program starts and no lane is reserved. Draft choices:
//!   - A required row that the runtime may not invoke is the missing grant, `LG3:capability.not_granted`, although an
//!     invocation of a commit or app row is refused with `KN5:capability.commit_row_refused`. The program's other
//!     grants (chapter 09's profile) are the host's until they are stored as data; the host can declare their failure
//!     as the diagnostic.
//!   - A required row that is not a kernel row, absent or itself language-defined, is
//!     `KN5:input.overlay_dependency_missing`.
//!   - A static binding keeps the captured arguments themselves, where LI.20 keeps their digest, so that the layer can
//!     check them. It answers as [`CommandLayer::availability_for`] does, LI.18's argument-aware hint without its
//!     revision: the row's argument-independent gates first, then an invocation's argument checks (an object,
//!     `confirm`, the names, the forms and a named revision's currency). It is computed when asked, not cached.
//!   - KN5.6's table (L13 (d) of the KN5 attack leads): [`CommandLayer::availability_table`] answers the shell's
//!     bindings in one [`Availability`], keyed by row and argument digest (LI.19) and stamped with the revision, the
//!     generation and the digest of the caller and its profile. The shell passes its bindings with each read and the
//!     layer keeps no registration, so a new set of bindings needs no new generation. A table is computed when read,
//!     never cached, and grants nothing.
//!   - Only a language-defined row takes metadata; a stored row an entry serves answers by its own gates. Metadata is
//!     not journaled: after a reopen the host declares it again, as it registers its overlays again.
//!   - The generation advances when an overlay row is registered, a stored row served or metadata declared (LI.21).
//! - The report after a command (KN5.22, R-02, ONE.70; CT-KN5-05's command-layer half). [`report`] pairs a receipt
//!   with the display's status, [`DisplayStatus`]. When the receipt says that its command applied, the display
//!   refreshes to the head through the host's [`Refresh`]; when that fails, the status is `OutOfDate`, whose message is
//!   [`DISPLAY_OUT_OF_DATE`], and the receipt stays as the layer returned it. [`Shell::send_reported`] sends and
//!   reports. Draft choices:
//!   - A command applied when it committed or completed as a workspace write, a view publication or a file
//!     publication, as KN5.22 lists them. An app row's completion, a query's answer and a preview's creation are not
//!     followed by a refresh; a preview reaches the display through the frames.
//!   - The refresh asks for the head when the report is made, not the revision the receipt names, since a later
//!     action may have moved the head; RPI.47's recovery reads the current bundle in the same way.
//!   - The continuation is [`refresh_display`], which sends no command: no retry is offered (ONE.12), and a press
//!     sent again returns its first receipt (ONE.71), whose report refreshes the display to the head at that time.
//!   - The host's [`Refresh`] adopts the engine's bundle into LP3's view model and presents it; the view model's
//!     refusal (`refresh_required`, RPI.9) is its error. The message on screen is LP3's.
//! - Job output (LP3's C6, its second common need). The host reports a program job's output as the runtime's sinks
//!   gave it ([`CommandLayer::finish_program_job_with`], [`JobOutput`]): the value the entry returned, the views it
//!   published (`publish_view`) and the workspace records it wrote (`write_workspace`), each record with its
//!   provenance block. [`CommandLayer::finish_program_job`] reports a value alone, as before.
//!   - The completion's result reference carries [`JobOutput::digest`]: the value's own digest when the job published
//!     no view and wrote no record, so a value alone keeps its digest, and otherwise the digest of the records, the
//!     value and the views together. The layer keeps no view or value; the host keeps the output.
//!   - Only a workspace row's job writes records. A record from any other row's job fails the job with
//!     `LG3:capability.not_granted`. Views pass from every row.
//!   - The host's provenance reader checks every record's block, as for `workspace.set` (PS1.16). A block it refuses
//!     fails the job with the reader's error, and without a reader the job fails with `KN5:input.row_not_built`.
//!   - A job whose output fails a check writes nothing.
//!   - A workspace row's records are written in its completion's one durable action, in the order of their names:
//!     each record's version advances, and the workspace revision advances once. The journal keeps them
//!     (`Transaction::records`), so a reopen writes them again.
//!   - No undo reaches the writes of a job of a language-defined row or of a stored row whose undo class is `none`.
//!     So that an undo or redo of an earlier write of the same record cannot write over them, the earlier actions that
//!     wrote one leave the undo and redo stacks. The cut runs on: walked from each stack's top, an action that leaves
//!     adds the records it wrote, so the actions beneath it that wrote one of them leave too, while those above it keep
//!     their place.
//!   - Drafts for LP3's amendment list (PHS.52): the digest's form, the code for a record from another row, the
//!     history cut and the journal key `records`.
//! - Argument blocks (BND.22; LP3's C6, step 1 item 4). The host fills every provenance block, and a program sets no
//!   field of one. [`CommandLayer::invoke_with_inputs`] takes the blocks of an invocation's arguments beside the
//!   envelope; for a running program's invocation the host builds them from the run's context. Draft choices:
//!   - Blocks change none of the checks before a row is dispatched (epoch, row, caller, token, recovery, argument
//!     names); their refusals come first. Only a row an entry serves takes blocks. Any other row that passed those
//!     checks refuses them with `KN5:input.argument_form`.
//!   - For a served row, the host's reader checks each block's form after the arguments' forms, the `FilterRef`s and
//!     the gate, as for a record's block (PS1.16). A block it refuses refuses the invocation with the reader's error.
//!     Without a reader, an invocation with blocks is refused with `KN5:input.row_not_built`. The layer reads nothing
//!     else in a block and sets no limit of its own on their number or size.
//!   - The job holds the blocks as given, in their order ([`ProgramJob::inputs`]). The host runs the entry with them
//!     as its run's inputs. [`CommandLayer::invoke`] passes none.
//!   - The request digest covers each block's SHA-256 under the key `inputs`, only when there are blocks. An
//!     invocation without blocks keeps its digest, and a retry with other blocks is other content
//!     (`KN5:input.command_id_reused`).
//!   - A run still reaches no workspace or file row (ruling of 18 November 2026). Blocks change no admission.
//! - Budgets (KN7.7, BM3, DET.27, ERR.27; [`budget`]; CT-RTE-07's B13 and B14; draft):
//!   - An invocation of a row an entry serves declares its budget in the envelope ([`Invocation::budget`]), beside
//!     the epoch and the token.
//!   - The layer admits it after the row's arguments, `FilterRef`s, gate and blocks, and before the lane and before a
//!     job ID exists, so a refusal uses nothing. A missing budget is `KN7:input.budget_not_declared`. A dimension over
//!     the ceiling is `KN7:resource_limit.budget_ceiling`, naming the dimension, the request and the ceiling.
//!   - The ceiling is the session profile's ([`Profile::budget_ceiling`], [`Budget::CEILING`] for every profile, the
//!     owner's values of 21 November 2026). A host may set a lower one
//!     ([`CommandLayer::set_budget_ceiling`]); the lower applies in each dimension. The safety stop is not checked.
//!   - The job carries the admitted budget ([`ProgramJob::budget`]). The request digest covers a declared budget
//!     under the key `budget`, so an invocation without one keeps its digest, and a retry with another budget is other
//!     content (`KN5:input.command_id_reused`). A workspace row's completion keeps the budget in its journal entry.
//!   - Another row runs no program and reads no budget. [`Shell::prepare`] declares the shell's budget on an
//!     invocation of a row an entry serves ([`Shell::budget`], [`Budget::CEILING`] by default) and none on another.
//! - Filter previews (KN0.9, H15.4, LI.41; LP3's C6, [`filter`]). Draft choices:
//!   - A `FilterRef` names a completed job of a row that prepares a predicate (`structure.layer`) by its result
//!     reference. It resolves while every record that job wrote still holds the job's write, to those records, each
//!     read with its content digest and version; otherwise it is refused with `KN2:input.named_object_invalid` when a
//!     job is admitted, and [`CommandLayer::availability_for`] answers the same. A reopen rebuilds the predicates from
//!     the journal's workspace completions.
//!   - A completed `filter.preview` job's result is a filter preview, kept by its job ID
//!     ([`CommandLayer::filter_preview`]) with KN0.9's dependencies ([`FilterPreviewDeps`]): the epoch, identity, mode
//!     and state of the revision the job read, and the resolved filter's digest. The protection and workspace
//!     revisions are not dependencies. The layer keeps fresh filter previews only, and none across a reopen.
//!   - A `FilterPreviewRef` names a filter preview by its job and the epoch it was made in ([`filter::preview_ref`],
//!     [`FilterPreview::reference`]). Job IDs start again after a reopen and no epoch comes back, so a reference from
//!     before a reopen never names a later filter preview with the same job ID.
//!   - `filter.apply`'s gate `fresh_filter_preview` holds while the filter preview its `FilterPreviewRef` names is
//!     kept, made in the reference's epoch, and fresh ([`CommandLayer::filter_fresh`]): its records are read again, so
//!     a write of one, its undo included, makes it stale, and an edit of another record does not. It reads neither the
//!     word-preview slot nor a presentation. Missing or stale, the row is refused with `KN5:stale.filter_preview` when
//!     the job is admitted; a preview that goes stale while the job runs fails the job with that code when its output
//!     is published, and nothing is written.
//!   - A workspace completion of a stored row whose undo class is `workspace` is one undoable action: `filter.apply`'s,
//!     and since the command table's freeze `set.save`'s, `filter.save`'s and `structure.layer`'s too (LB7.2, CM.17).
//!     `session.undo` and `session.redo` write each of its records' values before or after it again as one action,
//!     answering `committed` with equal puzzle hashes and advancing the workspace revision and the records' versions
//!     (LI.26). The journal keeps the writes in `records`, where an undo's value is `null` when it leaves the record
//!     empty. A replay decides undoability from H-08's table, before the host serves its rows. A completion that writes
//!     no record changes nothing an undo could restore, so it takes no undo entry. A predicate job that later writes
//!     one of an applied filter's records is an undoable action of its own above the apply, which keeps its place; a
//!     job of a language-defined row that does takes the apply off both stacks, as above.
//!   - The gate is answered by [`CommandLayer::availability_for`] too, after the `FilterRef`s.
//! - The test hooks (`hold_commit_lane`, `fault_replace_head`, `store_mut`) exist only in builds with debug assertions
//!   or the `test-hooks` feature (L15).
//! - The per-launch secret comes from the standard library's random hasher keys, the clock and the process ID, hashed
//!   with SHA-256. A product build needs an operating-system random source.

#![forbid(unsafe_code)]

pub mod availability;
pub mod budget;
pub mod filter;
mod forms;
pub mod frame;
pub mod journal;
pub mod jumble;
pub mod layer;
pub mod receipt;
pub mod repair;
pub mod sets;
pub mod shell;
pub mod store;
pub mod table;
pub mod token;
pub mod twist;
pub mod view;
pub mod worksheet;
pub mod workspace;

pub use availability::{Availability, AvailabilityKey};
pub use budget::Budget;
pub use filter::{FilterPreview, FilterPreviewDeps, ResolvedFilter, FILTER_PREVIEW_ROW, FRESH_FILTER_PREVIEW};
pub use frame::{FrameChain, FrameStamp, HeadlessFrames, PresentedFrame, ShownPreview};
pub use journal::{EntryKind, JournalEntry, JournalPage, JOURNAL_PAGE_LIMIT, JOURNAL_ROW};
pub use jumble::{JUMBLE_OFF_ROW, JUMBLE_ON_ROW, JUMBLE_TWIST_ROW};
pub use layer::{
    Caller, CommandLayer, Invocation, JobOutput, Preview, PreviewState, ProgramJob, EXPLORE_ROW, LAYOUT_REDO_ROW,
    LAYOUT_UNDO_ROW, PARTIAL_UNDO_ROW, POLICY_ROW,
};
pub use receipt::{
    code, digest, ActionHint, CancelReason, CommitRecord, Completion, CompletionKind, Direction, Error, PreviewDeps,
    Receipt, ResultRef, Revision, Status,
};
pub use repair::REPAIR_LIST_ROW;
pub use sets::{saved_set_record, SavedSet, REPLACE, SAVED_SET_PREFIX, SET_SAVE_RECORD, SET_SAVE_ROW};
pub use shell::{
    draft_args, refresh_display, report, word_draft, ArgSource, DisplayStatus, KeyBinding, Refresh, Report, Shell,
    DISPLAY_OUT_OF_DATE,
};
pub use store::{
    identity_from_json, identity_json, policy_from_name, policy_name, Fate, Genesis, History, MemoryStore, Profile,
    Reopen, Store, Transaction, TransactionKind,
};
pub use table::{Binding, Dependency, Kind, Lane, Metadata, Overlay, PreviewRule, Row, Table, UndoClass};
pub use token::{PreviewBinding, Secret, Token, TokenBinding};
pub use twist::TWIST_ROW;
pub use view::{ViewSourceRef, Views, VIEW_CLOSE_ROW, VIEW_OPEN_ROW};
pub use worksheet::{
    compare_inputs, MacroVersion, Occupancy, Placement, Resolution, Role, Worksheet, WorksheetRef,
    WORKSHEET_COMPARE_ROW, WORKSHEET_LOAD_ROW, WORKSHEET_SAVE_ROW,
};
pub use workspace::{Provenance, RecordRead, RecordValue, RecordWrite, ResolvedDraft, WorkspaceRecord, RECORD_ROW};

/// The sources of the command layer and its loop runner, with the stored command table, hashed into [`build`].
pub const SOURCES: &[(&str, &[u8])] = &[
    ("c600-command/Cargo.toml", include_bytes!("../Cargo.toml")),
    ("c600-command/src/availability.rs", include_bytes!("availability.rs")),
    ("c600-command/src/bin/c600-jumble-route.rs", include_bytes!("bin/c600-jumble-route.rs")),
    ("c600-command/src/bin/c600-lp3-oracle.rs", include_bytes!("bin/c600-lp3-oracle.rs")),
    ("c600-command/src/budget.rs", include_bytes!("budget.rs")),
    ("c600-command/src/filter.rs", include_bytes!("filter.rs")),
    ("c600-command/src/forms.rs", include_bytes!("forms.rs")),
    ("c600-command/src/frame.rs", include_bytes!("frame.rs")),
    ("c600-command/src/journal.rs", include_bytes!("journal.rs")),
    ("c600-command/src/jumble.rs", include_bytes!("jumble.rs")),
    ("c600-command/src/layer.rs", include_bytes!("layer.rs")),
    ("c600-command/src/lib.rs", include_bytes!("lib.rs")),
    ("c600-command/src/receipt.rs", include_bytes!("receipt.rs")),
    ("c600-command/src/repair.rs", include_bytes!("repair.rs")),
    ("c600-command/src/sets.rs", include_bytes!("sets.rs")),
    ("c600-command/src/shell.rs", include_bytes!("shell.rs")),
    ("c600-command/src/store.rs", include_bytes!("store.rs")),
    ("c600-command/src/table.rs", include_bytes!("table.rs")),
    ("c600-command/src/token.rs", include_bytes!("token.rs")),
    ("c600-command/src/twist.rs", include_bytes!("twist.rs")),
    ("c600-command/src/view.rs", include_bytes!("view.rs")),
    ("c600-command/src/worksheet.rs", include_bytes!("worksheet.rs")),
    ("c600-command/src/workspace.rs", include_bytes!("workspace.rs")),
    ("docs/progress/1.0/command-table.json", table::H08.as_bytes()),
];

/// The build string: the crate version, the SHA-256 of a `sha256sum`-style listing of [`SOURCES`], and the kernel's
/// build string.
pub fn build() -> String {
    let mut listing = String::new();
    for (name, bytes) in SOURCES {
        listing.push_str(&format!("{}  {name}\n", c600_digest::sha256_hex(bytes)));
    }
    let sources = c600_digest::sha256_hex(listing.as_bytes());
    format!("c600-command {} sources-sha256:{sources}; {}", env!("CARGO_PKG_VERSION"), c600_kernel::build())
}

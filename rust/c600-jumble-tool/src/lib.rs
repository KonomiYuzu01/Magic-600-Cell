//! The built-in jumbling tool of 600 Cell Script, headless half (LP3, C8; `docs/progress/1.0/language-lp3-plan.md`,
//! sections 8 and 9): PHS.22's built-in tool for jumbling, the counterpart of the 600-cell-Full tool in `c600-tools`.
//!
//! - [`tool`] holds the tool: its `.6cs` source in edition `6cs-draft-1` (syntax candidate A), its entries, its module
//!   manifest with its declared class (BND.27, BND.37) and its admission under the built-in column of chapter 07
//!   section 8, through `c600_tools::tool`. Each entry prepares a preview of a literal word the solver names and reads
//!   no state (LG3.13, BND.39). The tool never chooses or outputs a word by itself.
//! - [`word`] reads the `Word` a solver names as literal jumbling letters for `c600_jumble`'s observations, and builds
//!   LB1.8's literal words of one pinned menu from their letters alone: concatenation, inverse, conjugate, commutator
//!   and powers, refused with `LB1:input.incompatible_alphabet` across alphabets.
//! - [`level`] names the level a result reads: poses inside Jumble, labels in 600-cell-Full (KN1.11).
//! - [`relation`] is one typed relation of the relationship strip at pose level, the current occupant of a sticker
//!   slot, on and off the lattice (EV3.8, EV3.9).
//! - [`protection`] holds protection verdicts that carry their level and answer only at it (LB3.4, KN1.11): M8's
//!   protection of a whole word at pose level, and the kernel's protection of named pieces at label level.
//! - `tests/tool.rs` runs the jumbling tool's parts of CT-LB6-03 and CT-BND-15.
//! - `tests/lb3.rs` runs the kernel halves of CT-LB1-02, CT-LB3-01, CT-LB3-03, CT-LB3-05 with CT-BND-17, CT-LB3-06 and
//!   CT-LB3-07 on `c600_jumble`'s public API, with milestone M8's end tests (`levels`) and repair list
//!   (`observe::cuts::repair_list`), and with LP2's ν row (`c600_jumble::nu`) and correction catalogue
//!   (`c600_jumble::corrections`) for CT-LB3-01's reachable nonzero-ν state and the correction part of CT-LB3-07. The
//!   LB1 and LB3 library entries are language code the IR draft cannot express yet, so each test checks the facts the
//!   library entry would return; the library entries wait for the IR draft's collections.
//! - `tests/ct_lb3_04.rs` runs CT-LB3-04 whole at this crate's level, on LP2's `nu` and `correction_set_check` rows:
//!   off-lattice ν is undefined, the pinned C5 convention round-trips, a planted nonzero fixed I_a coordinate is a
//!   consistency error, and a drafted correction set is checked as drafted. No 600 Cell Script program calls the rows,
//!   since the IR draft has no F2 or F5 number kinds.
//! - `tests/ct_lb1_03.rs` runs CT-LB1-03's jumbling kernel half, `tests/ct_lb3_02.rs` CT-LB3-02 whole (LB3.4),
//!   `tests/ct_ev3_06.rs` CT-EV3-06's off-lattice half and `tests/ct_lb4_03.rs` CT-LB4-03's jumbling half, the last
//!   over `c600-display`'s posed row (J02) and pose projection (J13), which places a sticker at a lattice pose by M8's
//!   slot action of K⁺, and `c600-repr`'s layouts. Each test's documentation says what runs and what does not. C6's
//!   `c600-repr` runs the 600-cell-Full half of CT-LB4-03 and the unchanged-frame half of CT-EV3-06 (EV3.10).
//!
//! The tool runs in-process, as `c600-tools` runs its tool. Nothing here is a command-layer row: no state of the
//! jumbled puzzle is previewed, committed or undone through `c600-command`.
//!
//! # Drafts
//!
//! Each choice freezes later with the IR, the command table or BM5's review.
//!
//! - **Entries (LB6.5, LB3.8, LG3.6).** The module `jumble_checker` exports two commands, `check_named_twist(t: Word)`
//!   and `check_named_word(w: Word)`, for LB6.5's candidate "Named twist/word checker and kept checks". Each proposes
//!   its `Word` parameter as given. The IR draft has no list types and no word operations, so an entry does not test
//!   that a named twist is one letter, and the two entries differ in name only; [`word::twist`] reads one letter, in
//!   Rust. The kept checks (DT-J1 O4) are no entry: a kept check proposed again from storage is the storage-laundering
//!   case capability separation refuses (BND.39), so the solver names the word again, and the table of kept checks is
//!   the shell's.
//! - **Target and row set (BND.57, LB6.5).** Every entry's target is `operation.preview`, the only preview row the
//!   command layer on main has, so the row set is that one row. The draft command table's jumbling rows
//!   (`jumble.release.check`, `operation.check`) are not in the command layer, and LB6.5 leaves ENG-0J's row names to
//!   EN3. BND.57's list of rows by class does not exist as data, so the row set is not tested against one.
//! - **Result (BND.42).** An entry returns `record { base, id }`, read with `receipt_base` and `receipt_id`, as the
//!   600-cell-Full tool does. A previewing program does not read its preview's outcome, so the check's answer reaches
//!   the solver through the shell, not through the program.
//! - **Manifest and class (BND.27, BND.37, LB6.5).** The IR and the syntax carry no module manifest, so the manifest
//!   is Rust data: one `c600_tools::tool::Manifest` per entry, sharing the module, the class and the row set. The
//!   declared class is `checking`, LB6.5's candidate class for the row. No `PrincipleRecord` and no review record
//!   exist (LB6.3, BM5); the manifest names the SHA-256 of the module's source, the digest a review record will name
//!   (CT-BND-15).
//! - **Built-in admission (chapter 07 section 8; LG3.13).** Each entry is loaded with `c600_tools::tool::Tool::new`,
//!   which admits it with `c600_check::admit_built_in` and the class rows of `c600_tools::tool::class_rows` (the
//!   preview row of the manifest's row, a draft of that crate), under its draft capability contract, checker pin and
//!   run limits. This crate adds no admission rule.
//! - **Letters (LB1.9, LB3.8).** [`word::letters`] reads a `Word` as pairs of integers, cap then item, the item an
//!   index in the pinned order of the state's menu, as the reference's letters `[grip, item]`. The reading is Rust for
//!   the engine side of the tests. How the command layer reads a jumbling word is LP2's KN5 implementation and may
//!   differ. A word that is no sequence of letters is a [`word::WordError`] with no chapter 13 code, since that code
//!   belongs to the command layer's row; only a mix of alphabets carries one, `LB1:input.incompatible_alphabet`.
//! - **Alphabet (LB1.3, LB1.9, LB1.10).** A literal word's [`word::Alphabet`] is the jumbling identity block of its
//!   pinned menu: puzzle, model, order-free menu identity, menu-order digest and contract revision. Two words, or a
//!   word and a state, share an alphabet when their blocks are equal, since only then does an item index name the same
//!   rotation; free mode has no menu and so no alphabet. The refusal names the first field that differs, in KN1.4's
//!   order.
//! - **Word constructors (LB1.8, LB1.9).** The inverse of a letter is the letter at the same cap whose item's matrix is
//!   the transpose of its own, the inverse rotation, which an inverse-closed menu has; a menu without one is broken
//!   (`WordError::NoInverse`), never a property of a word. The frozen text fixes no convention, so the conjugate by `s`
//!   is `s w s⁻¹` and the commutator of `a` and `b` is `a b a⁻¹ b⁻¹`. A power is expanded letter by letter: the empty
//!   word at 0 and powers of the inverse below 0. No constructor cancels, reduces or reorders letters.
//! - **Levels (KN1.7, KN1.11).** [`level::Level`] has the two levels a protection verdict or a relation of this crate
//!   reads: poses inside Jumble and labels in 600-cell-Full, named `poses` and `labels` as `c600-jumble`'s protection
//!   report names them. The rest of KN1's identity tower is not needed here.
//! - **Current occupant (EV3.8, EV3.9, KN0.4 to KN0.6).** The occupant of slot t, with home piece A, is the lattice
//!   piece in A's chamber, placed by M8's slot action of K⁺ (`c600_jumble::levels::Tables::image`); "none: off the
//!   lattice" when no lattice piece is there; and `unknown(reason)` only on a raw pose table whose lattice pieces
//!   collide (KN2.14), which no twist reaches. The rule reads poses, the chambers of K⁺ and home slots, no label; where
//!   M8's labels cover a slot, they name the same piece (CT-EV3-06). The two missing values are
//!   [`relation::Missing`]'s, an undefined result's reason is its missing part slot by slot, the empty request is a
//!   total empty result, a repeated slot is answered once, and a slot outside the model refuses the whole call with
//!   `KN2:input.named_object_invalid`. The result names the relation, its level, the identity block and the pose digest
//!   it read.
//! - **Protection verdicts (LB3.4, KN1.11, KN2.4, KN2.7).** A [`protection::Verdict`] holds its level, its mode, its
//!   protected pieces, its finding, its first refused prefix under Strict and the digest it read. It copies an
//!   engine's report and judges nothing itself: at pose level M8's protection of a whole word (`levels::derive`, its
//!   `WordProtection`), at label level the kernel's protection of named pieces, fixed cell centres included
//!   (`Kernel::piece_protection` under Net, `Kernel::strict_piece_protection` under Strict). The kernel names a piece
//!   by its home position, the number `c600_jumble` gives it, which CT-LB3-02 checks for the centre piece of cell 0.
//!   Asked for a puzzle of the other level, a verdict is refused with `KN1:input.level_not_held`, both ways;
//!   `LG2:type.level_mismatch` stays the checker's static diagnostic. CT-LB3-02's crossing at solved is M8's end
//!   tests (`label_checkpoint`, `retained_checkpoint`) and its projection to labels (`levels::project`), from which
//!   the kernel's label pass starts; the crossing as a command-layer commit row waits for LP2's KN5.
//! - **CT-LB4-03's jumbling fixture (LB4.4, RPI.15).** `c600-repr`'s layouts carry no posed row, so the test adds
//!   J02's posed row of the base state as primitive 2 of each plan: the plan then draws a pose-level subject, which
//!   the validator checks, and a filter, which rebuilds primitive 0 only, keeps it. The mechanically analysed set is
//!   the certified inside set of the state's last twist, J03's moving set: a set the engine certified, not one a
//!   display computed.
//! - **Kernel halves (LB3.1, LB3.5, LB3.6, LB3.7, LB3.8).** The tests take the kernel's answers as the library
//!   entries' facts: `word_check` for LB3.8's `check_named_word`, `prefix_observations` for the chronological
//!   observations, `heights` for heights and domain IDs, M8's `levels::sticker_test`, `retained_checkpoint` and
//!   `label_checkpoint` for A5's end tests, and `observe::cuts::repair_list` for LB3.6's repair list, each of whose
//!   deltas the test reads again from the pair's own word, and whose y and z it recomputes for one pair with JI.6's
//!   release check. "No protection conflict" in CT-LB3-06 is a run with no protected piece, and prefix protection in
//!   CT-LB1-02 is the kernel's pose-level Net protection of one letter (`State::apply`).
//! - **ν and drafted correction sets (chapter 03's "ν and drafted correction-set check" and "Correction catalogue";
//!   A5).** The tests read ν with LP2's `c600_jumble::nu::nu` and check drafted sets with
//!   `c600_jumble::corrections::correction_set_check`, and add no rule of their own. A drafted set is a list of pairs
//!   (correction id, signed multiplicity) fixed in a test in the solver's place, and a correction's word is taken only
//!   by an id the test names (`Catalogue::lookup`, `Catalogue::twists`). The check answers ν's 45 values after the
//!   set; nothing here chooses, ranks, reduces or proposes a set. This settles nothing in the chapters.
//! - **The C5 round trip (A5, DT-J1 M13).** CT-LB3-04 reads "the pinned C5 convention round-trips" as three facts: in
//!   each C5 frame group, r0 is the least element of order 5 and r0^i has class i; correction 27's ν, read in r0, is
//!   its row and comes back unchanged through its 45 values, its JSON list and the label checkpoint's evidence; and
//!   the manual's rule "make correction 27 v times", with v that value, checks to 0, which a value read in another
//!   generator does not. This settles nothing in the chapters.
//! - **CT-LB3-04's plant (A5, DT-J1 A17).** No reachable I_a state has a nonzero fixed coordinate, so the test plants
//!   one: the pinned I_b menu with I_a's order-free menu identity written over its own, and a six-letter I_b word
//!   fixed in the test (`MIRROR`) whose lattice configuration has b5 = 1. The ν row and the drafted check refuse it
//!   with `KN2:consistency.fixed_nu_nonzero`; under the true I_b menu the ν row gives the same configuration's value,
//!   with no consistency error. This settles nothing in the chapters.
//! - **CT-LB3-07's correction part.** The inspector library entries are not written, so the test reads as their
//!   output what the tool's word entry and LP2's rows answer on the kernel: the whole-word check of a correction's
//!   literal word named by its id, the ν row, the drafted check and the label checkpoint's evidence. A leak is a field
//!   whose name names a recommendation, a next cap, a ranking or a correction, or a field holding a list of lists; the
//!   test reads names and shapes, not values. This settles nothing in the chapters.
//!
//! # What waits
//!
//! - Jumble on in the command layer, with `jumble.twist.menu`, `jumble.off` and the oracle route through it, is built
//!   (LP2's pull requests 212, 217, 218 and 220) and not taken up here yet; CT-LB4-03's layout evaluated again at a
//!   later revision of a jumbled session waits for that.
//! - ν and the correction catalogue in 600 Cell Script: the IR draft has no F2 or F5 number kinds, so the tests call
//!   LP2's `nu` and `correction_set_check` rows in Rust, and LB3's entry "ν and drafted correction-set check" waits as
//!   language code. The label checkpoint computes no ν itself; the tests supply the `nu` row's value. CT-LB3-01's
//!   checker half, the end tests' typed verdicts, is LP2's (pull request 213).
//! - The repair list through the command layer: a job that names its cap (KN5), the built-in column's repair-list
//!   exception in `c600-check`, an empty list worded as BND.2 states, and no key that skips caps with an empty list
//!   (CT-LB3-05, CT-BND-17): LP2.
//! - Not taken up yet on M8: the whole-word protection of `levels::derive` for CT-LB3-06, and the rim and survey rows
//!   of CT-LB3-07.
//! - The relationship strip's data contract (statement reference, claims, evidence and source location; chapter 06,
//!   RPI.65) and EV3.8's other roles: the occupant relation carries none of them.
//! - CT-LB1-02's domain-preserving key and "keys preserve the end digest": an owner answer on LG2.18's word keys.
//! - The LB1 and LB3 library entries as language code, not written yet. The IR draft has collection types and word
//!   operations (pull request 176) and menu words (pull request 210); LB3's ν entry also needs F2 and F5.
//! - The review of the class against the entries and its record (CT-LB6-03, CT-BND-15): BM5.

#![forbid(unsafe_code)]

pub mod level;
pub mod protection;
pub mod relation;
pub mod tool;
pub mod word;

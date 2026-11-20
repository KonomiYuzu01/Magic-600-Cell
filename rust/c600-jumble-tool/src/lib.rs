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
//! - [`library`] holds the jumbling library entries as 600 Cell Script on the IR draft's menu words (LP2's pull request
//!   210), each module with its catalogue row: LB1.8's literal words of one pinned menu, LB1.10's trace equality keys
//!   of menu words, and LB3.1's heights, whole-word checks and protection at pose level. The entries track, check and
//!   protect the solver's named word or state; none chooses, outputs or previews a word.
//! - [`locality`] builds the relation `jumble-locality/1` that a jumbling run pins beside its menu alphabet, from the
//!   table `c600_jumble::locality` regenerates from the loaded model (LB1.10; LP2's pull request 251).
//! - [`level`] names the level a result reads: poses inside Jumble, labels in 600-cell-Full (KN1.11).
//! - [`relation`] is one typed relation of the relationship strip at pose level, the current occupant of a sticker
//!   slot, on and off the lattice (EV3.8, EV3.9).
//! - [`protection`] holds protection verdicts that carry their level and answer only at it (LB3.4, KN1.11): M8's
//!   protection of a whole word at pose level, and the kernel's protection of named pieces at label level.
//! - [`rows`] is the jumbling rows' host bridge: KN2.4's jumbling rows that [`library`]'s entries read, answered by
//!   `c600_jumble` and served to a run as snapshot reads at the command layer's head with Jumble on
//!   ([`rows::Rows::at_head`]), on a witnessed base or at a state. It grants those rows only, refuses another argument
//!   form with `KN5:input.argument_form` and prices each pass over the state in a draft tariff.
//! - `tests/tool.rs` runs the jumbling tool's parts of CT-LB6-03 and CT-BND-15.
//! - `tests/lb3.rs` runs the kernel halves of CT-LB1-02, CT-LB3-01, CT-LB3-03, CT-LB3-05 with CT-BND-17, CT-LB3-06 and
//!   CT-LB3-07 on `c600_jumble`'s public API, with milestone M8's end tests (`levels`) and repair list
//!   (`observe::cuts::repair_list`), and with LP2's ν row (`c600_jumble::nu`) and correction catalogue
//!   (`c600_jumble::corrections`) for CT-LB3-01's reachable nonzero-ν state and the correction part of CT-LB3-07. Each
//!   test checks the facts the library entry would return. CT-LB1-02's key half runs there too: LG2.18's word key
//!   under I_a's item matrices, which [`word::Alphabet::pinned`] pins, at a blocked cap and on random words from three
//!   states, through the library entry `menu_word_keys`, whose source, purity and misuse refusal are checked there.
//! - `tests/library.rs` checks [`library`]'s modules with LP2's checker and runs them with LP2's interpreter:
//!   CT-LB1-03's jumbling half in the language, and the library entries of CT-LB3-03 and CT-LB3-06, the latter with a
//!   whole word's Net and Strict protection at pose level from M8's `levels::derive` (pull request 182). The bridge
//!   [`rows`] serves the KN2.4 rows on witnessed snapshots and states, and each result is compared with the kernel
//!   halves' answers.
//! - `tests/ct_lb3_04.rs` runs CT-LB3-04 whole at this crate's level, on LP2's `nu` and `correction_set_check` rows:
//!   off-lattice ν is undefined, the pinned C5 convention round-trips, a planted nonzero fixed I_a coordinate is a
//!   consistency error, and a drafted correction set is checked as drafted. Three of its tests run LB3's entry
//!   [`library::NU`] in the language on the IR draft's F2 and F5 (LP2's pull request 222), through [`rows`] on
//!   witnessed bases.
//! - `tests/ct_lb3_07.rs` runs CT-LB3-07 on the inspector library entries in the language: LB3.1's "Rim and seam
//!   facts; N" ([`library::RIM`]), "Covers at a named cap" ([`library::COVERS`]) and "Correction catalogue"
//!   ([`library::CORRECTIONS`]), with the named-word checks and the drafted check, through [`rows`] on witnessed
//!   bases. Each answer is compared with `c600_jumble`'s and each entry's reads are recorded with their arguments: the
//!   rim entry answers every cap in cap-ID order and nothing of a twist, the covers and catalogue entries read the
//!   solver's named cap or id only, and no answer names a recommendation, a next cap, a ranking or a selection. The
//!   crossing entry ([`library::CROSSING`]), the Jumble-off half of "Mode crossing prerequisites", answers A5's
//!   prerequisites of Jumble off; its Jumble-on half waits for a KN4 journal query served to programs (KN4.3), so
//!   CT-LB3-07 stays part.
//! - `tests/rows.rs` runs the library entries of CT-LB1-03, CT-LB3-03, CT-LB3-04 and CT-LB3-06 through [`rows`] at the
//!   command layer's head with Jumble on, in I_a, S4 and I_b sessions: each run equals, byte for byte, the same run on
//!   the same state witnessed apart from the layer, which the two tests above compare with the kernel. With Jumble off
//!   every read stops with `KN5:input.menu_word_needs_jumble_on`. It also checks what the bridge grants and its
//!   tariff, and that the `seam_facts` row answers the engine's seam facts for the named cap.
//! - `tests/ct_rte_07_jumble.rs` runs CT-RTE-07's six jumbling keys of LX-3's frozen corpus (MR01, MR02, MR05, MR06,
//!   A31 and A32) with literal words and again with menu words of the I_a alphabet, pinned with its item matrices,
//!   through `c600-module`'s pipeline under the jumbling model identity, with [`rows`] as the engine host: ten runs
//!   and one SHA-256 for each key, the solving keys refused at the preview point by capability separation, a manual
//!   proposal as the control, and the accounting against the 600-cell-Full leg.
//! - `tests/ct_lb1_03.rs` runs CT-LB1-03's jumbling kernel half, `tests/ct_lb3_02.rs` CT-LB3-02 whole (LB3.4),
//!   `tests/ct_ev3_06.rs` CT-EV3-06's off-lattice half and `tests/ct_lb4_03.rs` CT-LB4-03's jumbling half, the last
//!   over `c600-display`'s posed row (J02) and pose projection (J13), which places a sticker at a lattice pose by M8's
//!   slot action of K⁺, and `c600-repr`'s layouts. Each test's documentation says what runs and what does not. C6's
//!   `c600-repr` runs the 600-cell-Full half of CT-LB4-03 and the unchanged-frame half of CT-EV3-06 (EV3.10).
//! - `tests/jumble_on_layer.rs` takes up Jumble on in the command layer (LP2's pull requests 212, 217, 218 and 220):
//!   oracle equality against M5's W-J fixtures, every record of each journal pressed through `jumble.on` and
//!   `jumble.twist.menu` beside M5's engine, with J1's references at its stages. CT-LB4-03's layout evaluated again at
//!   a later revision of a jumbled session (`tests/ct_lb4_03.rs`) presses, and undoes, through the same layer;
//!   `tests/common/mod.rs` holds the session both use.
//! - `tests/ct_lb1_06.rs` runs CT-LB1-06's jumbling half on the model in `assets/`: the relation [`locality`] pins,
//!   the keys of bca, cab and abc, identity letters and an item beside its inverse kept, equal keys with equal
//!   admissibility and end digests on M5's states, the library entry `menu_traces` in the language, refusals of a
//!   key under another menu or version, and word keys separate from trace keys: in one run that pins I_a's alphabet
//!   with its item matrices and the relation, `word_key` and `trace_key` key the same menu words each by its own rule.
//!
//! The tool runs in-process, as `c600-tools` runs its tool. The tool itself is not a command-layer row; [`rows`]
//! reads the command layer's head, and the tests above drive the layer.
//!
//! # Drafts
//!
//! Each choice freezes later with the IR, the command table or BM5's review.
//!
//! - **Entries (LB6.5, LB3.8, LG3.6).** The module `jumble_checker` exports two commands, `check_named_twist(t: Word)`
//!   and `check_named_word(w: Word)`, for LB6.5's candidate "Named twist/word checker and kept checks". Each proposes
//!   its `Word` parameter as given. The IR draft has list types and word operations since LP2's pull request 176,
//!   but these entries do not take them up yet, so no entry tests yet that a named twist is one letter, and the two
//!   entries differ in name only; [`word::twist`] reads one letter, in Rust. The kept checks (DT-J1 O4) are no entry:
//!   a kept check proposed again from storage is the storage-laundering case capability separation refuses (BND.39),
//!   so the solver names the word again, and the table of kept checks is the shell's.
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
//!   release check. "No protection conflict" in CT-LB3-06 is a run with no protected piece in `tests/lb3.rs`, and in
//!   `tests/library.rs` also a run whose protected pieces the word leaves in place, clear under Net and Strict. Prefix
//!   protection in CT-LB1-02 is the kernel's pose-level Net protection of one letter (`State::apply`).
//! - **Library entries (LB1.8, LB3.1, KN2.4).** The row answers, the arguments, what the entries recompute and the
//!   refusals are [`library`]'s drafts.
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
//! - **CT-LB3-07's correction part.** The kernel half in `tests/lb3.rs` reads as the inspector entries' output what
//!   the tool's word entry and LP2's rows answer on the kernel: the whole-word check of a correction's
//!   literal word named by its id, the ν row, the drafted check and the label checkpoint's evidence. A leak is a field
//!   whose name names a recommendation, a next cap, a ranking or a correction, or a field holding a list of lists; the
//!   test reads names and shapes, not values. `tests/ct_lb3_07.rs` runs the entries themselves with the same names
//!   and, for the rim and covers entries, the further names of LB3.5's selections and of a twist's outcome. This
//!   settles nothing in the chapters.
//! - **Jumble on through the command layer (LP2's pull requests 212, 217, 218, 220 and 254).**
//!   `tests/jumble_on_layer.rs` compares the layer's jumbling journal with the fixtures' references, and
//!   `tests/jumble_layer_verdicts.rs` runs `jumble.off`, the refusals with Jumble on, redo, protected pieces and free
//!   mode beside the tool's own entries at the layer's head (`rows::Rows::at_head`): the label checkpoint predicts each
//!   exit, with the crossing entry's prerequisites agreeing, and the twist check and a word's protection predict each
//!   press. The tests read the layer's decisions and add no rule of their own. This settles nothing in the chapters.
//! - **The repair list through the command layer (LP2's pull request 326; CT-LB3-05, CT-BND-17).** LP2's
//!   `jumble.repair.list` is a job that names its cap (KN5): the layer passes M8's list on whole and adds its
//!   `statement`, BND.2's wording of an empty list, and no key skips caps with an empty list (`c600-command`'s
//!   `tests/repair_list.rs`). `tests/jumble_layer_repair.rs` runs the job beside the language entry
//!   ([`library::REPAIR`]) at the layer's head on the I_a walk: at a named misaligned cap, at closed caps and at a cap
//!   whose completed list is empty, after a press that drops the kept list and after its undo, the two agree on
//!   status, input digest, parameters, message and rows, the job alone words the list, and each run reads the
//!   `repair_list` row once with the named cap. The tests read the layer's and the tool's answers and add no rule of
//!   their own. This settles nothing in the chapters.
//!
//! # What waits
//!
//! - ν: LB3's entry "ν and drafted correction-set check" is language code ([`library::NU`]) and reads its rows through
//!   [`rows`], and so does the correction catalogue's entry ([`library::CORRECTIONS`]). M8's label checkpoint computes
//!   no ν itself; the `label_checkpoint` row takes the `nu` row's value when the identity labels and a retained witness
//!   leave ν unknown, as the command layer's exit reads it ([`library::END_TESTS`]). CT-LB3-01's checker half, the end
//!   tests' typed verdicts, is LP2's (pull request 213).
//! - The survey's all-items loop: not served. LB3.5 keeps it out of the inspector entries, so the `survey` row answers
//!   the solver's named grip only ([`library::ADMISSIBILITY`]), and [`library::RIM`] reads the rim summary and one
//!   whole seam scan only.
//! - A producer of the relationship strip's records. The strip's data contract (statement reference, claims, evidence
//!   and source location; chapter 06, RPI.65) and EV3.8's roles are `c600_display::strip`'s (LP3 C6, pull request
//!   372). The occupant relation carries none of them itself; CT-EV3-06 (`c600-repr`'s `tests/ct_ev3.rs`) writes it
//!   as strip records, and no code here does yet.
//! - The jumbling rows in the command table: [`rows`] serves them read-only beside the layer, in [`library`]'s draft
//!   answer types and a draft tariff; the command table names and prices them when it freezes (KN5, PHS.52), and no
//!   program runs here as a job of the layer. CT-LB3-03's agreement with `trees.py` stays M5's Python test on the
//!   same kernel API.
//! - LB3's other entries as language code, not written yet; also the named cap of "Release and whole-word checks",
//!   with ΔN, Δn_h, Δu, the moved sets and the prefix digests (LB3.8), and the label half of "Protection" after
//!   crossing.
//! - The review of the class against the entries and its record (CT-LB6-03, CT-BND-15): BM5.

#![forbid(unsafe_code)]

pub mod level;
pub mod library;
pub mod locality;
pub mod protection;
pub mod relation;
pub mod rows;
pub mod tool;
pub mod word;

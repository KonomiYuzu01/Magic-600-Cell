//! The built-in jumbling tool of 600 Cell Script, headless half (LP3, C8; `docs/progress/1.0/language-lp3-plan.md`,
//! sections 8 and 9): PHS.22's built-in tool for jumbling, the counterpart of the 600-cell-Full tool in `c600-tools`.
//!
//! - [`tool`] holds the tool: its `.6cs` source in edition `6cs-draft-1` (syntax candidate A), its entries, its module
//!   manifest with its declared class (BND.27, BND.37) and its admission under the built-in column of chapter 07
//!   section 8, through `c600_tools::tool`. Each entry prepares a preview of a literal word the solver names and reads
//!   no state (LG3.13, BND.39). The tool never chooses or outputs a word by itself.
//! - [`word`] reads the `Word` a solver names as literal jumbling letters for `c600_jumble`'s observations.
//! - `tests/tool.rs` runs the jumbling tool's parts of CT-LB6-03 and CT-BND-15.
//! - `tests/lb3.rs` runs the kernel halves of CT-LB1-02, CT-LB3-03, CT-LB3-06 and CT-LB3-07 on `c600_jumble`'s public
//!   API. The LB1 and LB3 library entries are language code the IR draft cannot express yet, so each test checks the
//!   facts the library entry would return; the library entries wait for the IR draft's collections.
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
//! - **Built-in admission (chapter 07 section 8; LG3.13).** Each entry is admitted with
//!   `c600_tools::tool::admit_built_in` and loaded with `c600_tools::tool::Tool::new`, under that crate's draft
//!   capability contract, checker pin and run limits. This crate adds no admission rule.
//! - **Letters (LB1.9, LB3.8).** [`word::letters`] reads a `Word` as pairs of integers, cap then item, the item an
//!   index in the pinned order of the state's menu, as the reference's letters `[grip, item]`. The reading is Rust for
//!   the engine side of the tests. How the command layer reads a jumbling word is LP2's KN5 implementation and may
//!   differ. A word that is no sequence of letters is a [`word::WordError`] with no chapter 13 code: the code belongs
//!   to the command layer's row.
//! - **Kernel halves (LB3.1, LB3.7, LB3.8).** The tests take the kernel's answers as the library entries' facts:
//!   `word_check` for LB3.8's `check_named_word`, `prefix_observations` for the chronological observations and
//!   `heights` for heights and domain IDs. "No protection conflict" in CT-LB3-06 is a run with no protected piece, and
//!   prefix protection in CT-LB1-02 is the kernel's pose-level Net protection of one letter (`State::apply`): M5 has
//!   no protection of a whole word.
//!
//! # What waits
//!
//! - Jumble on in the command layer and oracle equality through it: LP2's KN5 implementation. LP3 changes no
//!   `c600-command` layer code until it lands.
//! - ν, the checkpoints, the sticker test, the repair list (CT-LB3-05, CT-BND-17) and the correction catalogue: M5 left
//!   them to milestone M8 (the LP2 plan, section 11). With them wait CT-LB3-01, CT-LB3-04, CT-LB3-03's sticker-test
//!   sentence and the correction and rim parts of CT-LB3-07.
//! - LB3.4's crossing (CT-LB3-02): M5's `tests/crossing.rs` runs its kernel facts as CT-KN2-03; LP3's own test is C8's
//!   next piece.
//! - CT-LB1-02's domain-preserving key and "keys preserve the end digest": an owner answer on LG2.18's word keys.
//! - The LB1 and LB3 library entries as language code: collection types and word operations in the IR draft (LP2).
//! - The pose levels of CT-LB4-03 and the off-lattice relations of CT-EV3-06: `c600-repr` (C6), not yet on main.
//! - The review of the class against the entries and its record (CT-LB6-03, CT-BND-15): BM5.

#![forbid(unsafe_code)]

pub mod tool;
pub mod word;

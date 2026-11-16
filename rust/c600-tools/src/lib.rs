//! The built-in tools of 600 Cell Script and the LP3 source route (LP3, C5; `docs/progress/1.0/language-lp3-plan.md`,
//! section 7).
//!
//! - [`tool`] holds the built-in 600-cell-Full tool: its `.6cs` source in edition `6cs-draft-1` (syntax candidate A),
//!   its module manifest with its declared class (BND.27, BND.37), its admission in the solving profile (it prepares
//!   previews and reads no state: LG3.13, BND.39) and one run of it on `c600-run` through the run-time boundary, which
//!   gives the program the fixed acknowledgement (BND.42, BND.43).
//! - [`expand`] is the source route's recipe expansion: a case step's recipe to its literal word, through the kernel,
//!   outside the program and without a state read.
//! - [`case`] reads the frozen oracle cases O01 to O07.
//!
//! The tool runs in-process; the runtime's own process (C3) is not on main yet.
//!
//! # Drafts
//!
//! Each choice freezes later with the IR or the command table.
//!
//! - **Entry.** `full_600_cell.preview_word(w: Word)` is an exported command (LG3.6) that proposes `w` as given. The IR
//!   draft has no list types and no word operations, so the tool proposes the word it is given; the library entries of
//!   chapter 03 (LB1 to LB4) wait for the IR.
//! - **Target.** The proposal's target text names the command-layer row the proposal reaches, `operation.preview`.
//! - **Result.** The entry returns `record { base, id }`, read with `receipt_base` and `receipt_id`: the two receipt
//!   fields a previewing program may read (BND.42). A receipt itself has no canonical output form.
//! - **Manifest and class.** The IR and the syntax carry no module manifest yet, so [`tool::Manifest`] is Rust data.
//!   The declared class is LB6.5's candidate for the word/recipe composer and full-effect inspector, `analysis` and
//!   `checking`; the rows are `operation.preview`. No `PrincipleRecord` and no review record exist (LB6.3, BM5), so
//!   the review of the class against the entry (CT-BND-15, CT-LB6-03) is not done.
//! - **Built-in admission.** `c600_check::admit` with `Profile::Solving` is the column of a solver's own, forked or
//!   imported tool, which prepares no preview until the joint setting, so it refuses this tool. [`tool::admit_built_in`]
//!   applies the built-in column of chapter 07 section 8 over the checker's report and its capability-separation
//!   facts: no checker diagnostic; never both a state read and a preview (LG3.13); a previewing entry is an exported
//!   command (LG3.6) that calls no named function and proposes only a `Word` parameter as given, unshadowed (a word the
//!   solver named). The built-in column's two exceptions are not built. The API `c600-check` would need: an author or
//!   column parameter on `admit`, or an `admit_built_in` of its own.
//! - **Provenance.** The blocks name a draft capability contract, pinned as `c600-tools-draft-1`, and the checker
//!   pin `c600-check-lp2-draft` over `c600-check`'s source. The word argument enters with a manual block: the solver
//!   named it. The producer is the tool's module digest and entry, author `built_in`, profile `solving`.
//! - **Limits.** One run gets [`tool::LIMITS`]; the run's charges are information only.
//! - **Expansion.** A recipe the kernel accepts expands to its letters. A recipe the kernel refuses that is exactly one
//!   word step of integer moves fitting `i32` passes unchanged, so the command layer refuses the literal word as it
//!   refuses the draft on the command route. Any other refused recipe has no literal word: the tool does not run, and
//!   the step's record is the kernel's refusal. In O07 that is steps 3 to 7 and 9 (a fractional move, an empty
//!   recipe, three malformed stars and a recipe whose second step is a malformed star); steps 0, 1, 2 and 8 pass.

#![forbid(unsafe_code)]

pub mod case;
pub mod expand;
pub mod tool;

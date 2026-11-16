//! Magic 600 Cell evidence library: chapter 06 (`docs/architecture/1.0/language/architecture/06-evidence.md`) as
//! pure code. Statements, keys and scopes (section 2), the AND/OR evidence graph (section 3), grades, scenarios,
//! summaries and the two standing passes (section 4), the word function with its decision list and admission
//! conditions (sections 5 and 6), `CheckReport` admission (section 7), EVD.52's import rules (section 8) and EVD.64's
//! writer check at the `record` sink.
//!
//! Everything here is effect-free (EVD.54): no file, clock, process or session is read or written, and a word is a
//! function of the graph's facts and a pinned [`Edition`] (EVD.14). No word converts to a boolean (EVD.57), and nothing
//! in this crate grants, permits or commits anything (EVD.56).
//!
//! # Draft choices
//!
//! Where chapter 06 leaves an encoding or a detail open, this crate takes a draft, named here. None of these is a
//! decision of the chapter.
//!
//! - Word encoding (EVD.55, R3-E4): [`Word`] with a [`WordKind`], a [`Claim`], the row number, the scope phrase, the
//!   named premises, the source label and diagnostics. Row 0 is returned as an [`EvidenceError`] with its code and the
//!   EVD.40 cases found. [`Word::display`] is a draft display; EV5 owns teaching.
//! - Scope phrase wording (EVD.36): one instance "on one instance"; a search set "{bounds} of family {family} only"; an
//!   admitted domain "on {name}" (with parameters in brackets when given); every reachable state "for every reachable
//!   state of {puzzle}"; each guard appends ", for reachable inputs", ", given {name}" or ", under {name}". Row 3
//!   requires the phrase (EVD.36), and so does row 1 reached through EVD.42's finite exhaustive guard, read as: neg S
//!   stands at ω₀ through an OR child that holds a live exhaustive-computation leaf.
//! - Edition (EVD.14): `evidence-draft-1` holds all 30 pairs, the product order, an empty invalid-pairs table and the
//!   trial threshold 𝒰₀. The threshold stays the owner's (EVD.29); tests use 𝒰₀ and its up-subsets only.
//! - KN1.7 levels: labels stand beside the sticker class and position beside the region class, both read as mutual
//!   coarsenings for EVD.11; a decoded level outside the enumeration is comparable with nothing but itself.
//! - Negation (EVD.9): neg K is the root with the negated statement and the identical scope, otherwise the only root
//!   with the negated statement; several such roots read I2.
//! - Negative leaf (EVD.42, I5): a leaf in an argument of a statement of form universal. Exhaustive leaf: basis
//!   exhaustive computation.
//! - A leaf's own job: the EVD.61 binding when it states an outcome, else its search record's job.
//! - Search records and jobs for a key with no leaf (E3's truncated beam, E3's cancelled runs) attach to the key's OR
//!   node; FP.20 carries a search record inside a leaf only (lead for the integrator).
//! - Row 6: a search job cut off reads row 6 when its stop is `other(reason)`, the wall-clock, resource or
//!   product-ceiling stop (EVD.35, ERR.16); a search job cut off with stop closed matches no row of its own.
//! - Row 4's named premises: the premises of the canonical alternatives satisfied at ω₁ whose own claim does not stand
//!   at ω₀.
//! - I6: an argument of neg X offers a refutation of X when it holds a live leaf whose basis is not sample. A complete
//!   certificate is a certificate leaf flagged complete by its checker ([`Leaf::complete`]); a mixed(forall) witness is
//!   a leaf with role [`LeafRole::Witness`], and the witness and the complete negative at it must be live leaves of the
//!   same argument of neg X. The check also runs for an X with no OR node of its own.
//! - Leaf roles: chapter 06 names witnesses and transport lemmas but stores no role field, so the library keeps a
//!   [`LeafRole`] beside the leaf. A transport lemma leaf carries its checked relation Z = X·γ.
//! - Entry check (EVD.18): a target without a `form` fact counts as universal for I7 (a) at entry too.
//! - Writers (EVD.64): an import writes as the host does, under the same conditions; a commit pin is 40 or 64 lowercase
//!   hexadecimal digits and a review digest 64; the kernel writes `independent checker` only through
//!   [`admit_check_report`]; a value outside the enumeration has no writer.
//! - P13's key-name vocabulary and H1's refusal of a row that is not one instance (as I7 (c)).
//!
//! # Not implemented
//!
//! The label-table importer and its corpus walk (EVD.48 to EVD.51; CT-EVD-14, 15's mapping half, 16, 23), the FP.19 and
//! FP.20 wire encoding, KN6's checkers (this crate only admits their reports), EV5's display and teaching (EVD.39),
//! the display of independence and common failure points (EVD.28), PS3.12's dispatch of absent-format records
//! (EVD.53), obligation export (LB5.18), storage of the envelope (chapter 05, the M6 session store), the command layer,
//! the runtime process and the jumbling half.

#![forbid(unsafe_code)]

pub mod edition;
pub mod error;
pub mod graph;
pub mod import;
pub mod model;
pub mod sink;
pub mod word;

pub use edition::{basis_le, check_le, product_le, u0, Edition, EditionError};
pub use error::{EvidenceError, Inconsistency};
pub use graph::{Graph, NegResolution, Node, NodeId, OrNode};
pub use model::*;
pub use sink::{
    admit_check_report, check_write, entry_check, offer, record, reuse, CheckReport, Citation, Verdict, Writer,
};
pub use word::{
    discharge, dominates, effective_job, reduce, scope_phrase, standing, summaries_in_order, summary, val_at, word,
    Alternative, Claim, Diagnostic, Standing, Word, WordKind,
};

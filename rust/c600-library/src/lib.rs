//! The base library's first entries as 600 Cell Script (LP3, C5), written as language code over the IR draft (LB1.1):
//! chapter 03's row "Literal words" (LB1.8), over the IR draft's word operations; the F2 and F5 part of LB1.8's row
//! "Exact linear algebra"; and the exact-completion half of LB2's row "Residuals and exact completion", over KN2.4's
//! `positions` and `labels` rows.
//!
//! # Literal words
//!
//! The source [`SOURCE`], `src/words.6cs`, is the module `words` in edition `6cs-draft-1` (syntax candidate A), in the
//! formatter's canonical layout. It exports one pure function, `power(w: Word, n: Int) -> Word`:
//! for `n > 0` the word `w` written out `n` times, for `n < 0` the word `inverse(w)` written out `-n` times, and
//! `word[]` for `n = 0`. The words are literal: `power` concatenates and never cancels, so `power(word[+1, -1], 2)` has
//! four letters. Concatenation, inverse, conjugate (`x y x⁻¹`) and commutator (`x y x⁻¹ y⁻¹`) are the IR draft's own
//! operations (LG2.15), so the library adds powers and wraps none of them. A client module imports the library by the
//! SHA-256 of its source ([`source_sha256`]) and is lowered together with it ([`lower_sources`]).
//!
//! [`literal_words`] is the row's catalogue entry with LB1.2's five fields. Construction reads no state: the checker
//! infers no effect, capability or state read for `power`, and the words it builds are applied, prefix by prefix, by
//! the kernel's `derive`, `derive_strict` and `replay` rows (KN2.4), never by the library.
//!
//! # Exact completion
//!
//! The source [`COMPLETION_SOURCE`], `src/completion.6cs`, is the module `completion` in edition `6cs-draft-1`, in the
//! formatter's canonical layout. It exports one function,
//! `exact_completion(pieces: List<Nat>, home_slots: List<Nat>) -> Bool`, with the `snapshot` effect. `pieces` are the
//! candidates, named by their home positions, and `home_slots` are their home slots in the candidates' order, which
//! the host names with them: model data, the same in every state. The entry reads KN2.4's `positions` row for the
//! pieces, then KN2.4's `labels` row at the named home slots, each through `read_snapshot` with its parameter as given,
//! and answers `true` exactly when the list is not empty, the named home slots are the candidates' own and every
//! candidate is complete. A piece is complete when every one of its stickers sits in its own home slot, read from both
//! rows: `positions` answers its `slots` equal to its `home` slots, sticker for sticker (identity, slots and
//! orientation), and `labels` answers the label of each of its home slots as that slot itself (its home labels). A
//! label names a sticker by its home slot, so both are identity facts: the entry reads no colour, and stickers of one
//! colour in each other's slots are not complete.
//!
//! [`residuals_and_exact_completion`] is the row's catalogue entry with LB1.2's five fields, for exact completion only.
//! The checker infers the effect `snapshot`, a state read and the rows `positions` and `labels` for the entry, so under
//! capability separation it reads state and prepares no preview. The entry runs on a host that serves both rows:
//! `c600-tools`' host bridge does (`c600_tools::kernel_rows`), and `c600-tools/tests/exact_completion.rs` runs it there
//! on the command layer (CT-LB2-04).
//!
//! # Exact linear algebra over F2 and F5
//!
//! The source [`LINALG_SOURCE`], `src/linalg.6cs`, is the module `linalg` in edition `6cs-draft-1`, in the formatter's
//! canonical layout. It exports four pure functions over the IR draft's F2 and F5 (LP2's pull request 222):
//! `rank_f2(rows: List<List<Int>>) -> Nat` and `rank_f5`, the rank of the rows in the field, and
//! `solve_f2(rows: List<List<Int>>, rhs: List<Int>) -> record { rank: Nat, solution: List<Nat>, solvable: Bool }` and
//! `solve_f5`, for the equations `rows · x = rhs` in the field. Each entry of `rows` and `rhs` is an integer reduced into
//! the field with `to_f2` or `to_f5`, so negative entries and entries of any size are allowed. A solve answers the rank
//! of `rows`, whether the equations are consistent, and when they are, one solution as residues: Gauss-Jordan
//! elimination with every free unknown 0. Inconsistent equations answer `solvable: false` and an empty solution.
//!
//! [`exact_linear_algebra`] is the F2 and F5 part of LB1.8's row "Exact linear algebra". The checker infers no effect,
//! capability or state read for the four functions, so a rank or a solution says something about the supplied equations
//! only, never about a puzzle state or its reachability (CT-LB1-01). `tests/ct_lb1_01.rs` checks them against the
//! pinned research references: the 36-macro correction library's ν rows have rank 34 over F2 and 2 over F5, and the
//! F5 cycle criterion of the general method's senior review.
//!
//! The crate draws nothing and owns no state. It depends on the IR, syntax and digest crates only; its tests also run
//! the checker, the interpreter (`c600-run`) and the 600-cell-Full kernel, and `c600-tools`' tests run the
//! exact-completion entry through the tool's host bridge. Every result of its tests is source and fixture evidence.
//!
//! # Drafts
//!
//! Each choice below is this crate's own and settles nothing in the chapters it names.
//!
//! - LB1.8: `power`'s exponent is an `Int`, and a negative power is the inverse written out: `power(w, -n)` is
//!   `power(inverse(w), n)`. The IR draft has no subtraction or negation, and it had no loop when `power` was written
//!   (loops came with LP2's pull request 234), so `power` recurses on `exact_add(n, 1)` or `exact_add(n, -1)` under
//!   exact control (LG2.20), and the run's budget bounds the exponent.
//! - LB1.8: a power is a flat `Word` written out in full. A compressed power (LG2.16) is not built.
//! - LB1.2: the catalogue row is Rust data in [`literal_words`], because the IR draft and the syntax carry no module
//!   manifest yet (LG5.2, PS1.15), as in `c600-tools` and `c600-repr`. Its Effect field holds the IR draft's effects,
//!   and an empty list reads as the catalogue's `pure`, so a test compares it with the checker's inferred summary.
//! - LB1.8, LB1.9: the library builds no `LB1:input.incompatible_alphabet` refusal of its own. `power` takes a Full
//!   `Word`, and the checker refuses one of the IR draft's menu words (LP2's pull request 210) where a `Word` is
//!   expected, with that code.
//! - LB2, exact completion: the entry is a function of a candidate list, with its home slots, that answers a `Bool`,
//!   `true` only when completion is established. `false` covers both a candidate that is not complete and an empty
//!   list: an empty list establishes nothing, so the entry never answers `true` for it vacuously, and it does not stop
//!   the run, because chapter 13 registers no code for an empty candidate list. A piece outside 0 to 177,119 stops the
//!   run with the `positions` row's refusal, and a home slot outside 0 to 259,799 with the `labels` row's, both
//!   `KN2:input.named_object_invalid`.
//! - LB2, exact completion: completion is identity, read from both rows, and the entry passes only when both agree.
//!   From `positions`, the candidates' `slots` equal their `home` slots, compared as lists of lists in the candidates'
//!   order; from `labels`, the labels at the candidates' home slots, as named, equal those slots. The other fields of
//!   the `positions` record (`piece`, `orbit`, `position`) are read and not compared: a piece whose slots are its home
//!   slots is in its home position. No colour is read.
//! - LB2, LB3.5: the entry takes the candidates' home slots as a second parameter because the built-in column reads
//!   only for what the solver named. A snapshot read's argument is a parameter of the entry as given, a literal, or a
//!   record or collection built from those alone (LB3.5), so the entry cannot read `labels` at home slots it took from
//!   the `positions` answer: the checker refuses that form with `LG3:capability.not_granted`, as
//!   `c600-tools/tests/exact_completion.rs` shows. The host names the home slots with the pieces, and the entry answers
//!   `false` unless they equal the candidates' `home` lists from `positions`, flattened in order, so slots that are not
//!   the candidates' own never give a `true`.
//! - LB2, exact completion: the entry names no buffer and permits no residual. A candidate list that names a buffer
//!   whose position holds another piece answers `false`, whatever a residual check would permit.
//! - LB2, exact completion: the IR draft has no list map or flattening, so the entry collects the candidates' `home`
//!   and `slots` lists with `repeat`, taking the row index from the length of the list built so far, and flattens the
//!   home slots for the comparison with the named ones with the recursive helper `flatten` under exact control
//!   (LG2.20). The run's budget bounds both. `flatten` copies the rest of the list at every level, so its cost grows
//!   with the square of the number of candidates, and a list of all 177,120 pieces may stop on the step budget
//!   rather than answer; a single pass that builds the list with `push` would avoid that (deferred). The `labels`
//!   read is one request for all named home slots.
//! - LB2: the catalogue row is Rust data in [`residuals_and_exact_completion`], as for the literal words. Its Level is
//!   the chapter's "Full labels and positions", and its Effect field is the checker's `ReadSnapshot`.
//! - LB1.8, exact linear algebra: the IR draft has no function generic over a number kind, so the F2 and F5
//!   eliminations are two copies of one algorithm, each with its own field's types. Matrices cross the entry as
//!   `List<List<Int>>` and solutions as `List<Nat>` residues, because a field element has no canonical stored form
//!   (FP.6). The number of unknowns is the length of the first row, and an empty list of rows has none; rows of other
//!   lengths and an `rhs` shorter than `rows` stop the run at an `index` past a list's end. The IR draft has no
//!   mutable list, so each elimination step rebuilds the rows it changes with `push`; the run's budget bounds the work.
//! - LB1.5, LB1.6: the entry is generic elimination on supplied equations. Composing it with ν and the correction rows
//!   to choose a combination is the research tier's correction-combination solve; the entry reads neither, and its
//!   catalogue row carries the marker.
//!
//! # Not built
//!
//! - The rest of CT-LB1-01: exact Q(√5) signs, quaternion factor reconstruction and tree outputs; and the rest of
//!   "Exact linear algebra": kernels (null spaces) and the Z, Q and Q(√5) domains.
//! - Jumbling words in this crate: the jumbling library entries, on the IR draft's menu words (LP2's pull request
//!   210), are C8's, in `c600_jumble_tool::library`; `c600_jumble_tool::word` builds the kernel half's words in Rust.
//! - Library entries over kernel rows other than exact completion, such as LB2's "Star and recipe construction" and
//!   "Net and Strict protection": the tool's host bridge serves the `positions` and `labels` rows only
//!   (`c600_tools::kernel_rows`), so CT-LB2-02's tests apply a library-built word through the kernel from Rust.
//!   CT-LB2-02's Strict half also runs through the tool's route (`c600-tools`), on the command layer's Strict policy
//!   (LP2's pull request 198).
//! - The residual half of LB2's row "Residuals and exact completion": non-buffer positions, buffer occupants and
//!   frames.
//! - The other entries of LB1.8's table, among them the word and trace keys (LB1.9, LB1.10).

#![forbid(unsafe_code)]

use c600_digest::sha256_hex;
use c600_ir::{Effect, Program};
use c600_syntax::{lower, parse};

/// The library's source: edition `6cs-draft-1`, syntax candidate A, in the formatter's canonical layout.
pub const SOURCE: &str = include_str!("words.6cs");
/// The library's module.
pub const MODULE: &str = "words";
/// The module's exported functions.
pub const EXPORTS: [&str; 1] = ["power"];

/// The exact-completion source: edition `6cs-draft-1`, syntax candidate A, in the formatter's canonical layout.
pub const COMPLETION_SOURCE: &str = include_str!("completion.6cs");
/// The exact-completion module.
pub const COMPLETION_MODULE: &str = "completion";
/// The exact-completion module's exported functions.
pub const COMPLETION_EXPORTS: [&str; 1] = ["exact_completion"];
/// The KN2.4 rows the exact-completion entry reads with `read_snapshot`, in name order.
pub const COMPLETION_ROWS: [&str; 2] = ["labels", "positions"];

/// The exact linear algebra source over F2 and F5: edition `6cs-draft-1`, syntax candidate A, in the formatter's
/// canonical layout.
pub const LINALG_SOURCE: &str = include_str!("linalg.6cs");
/// The exact linear algebra module.
pub const LINALG_MODULE: &str = "linalg";
/// The exact linear algebra module's exported functions.
pub const LINALG_EXPORTS: [&str; 4] = ["rank_f2", "solve_f2", "rank_f5", "solve_f5"];

/// The SHA-256 of [`SOURCE`], in hex: the digest a client's `import` names.
pub fn source_sha256() -> String {
    sha256_hex(SOURCE.as_bytes())
}

/// The SHA-256 of [`COMPLETION_SOURCE`], in hex.
pub fn completion_sha256() -> String {
    sha256_hex(COMPLETION_SOURCE.as_bytes())
}

/// The SHA-256 of [`LINALG_SOURCE`], in hex.
pub fn linalg_sha256() -> String {
    sha256_hex(LINALG_SOURCE.as_bytes())
}

/// One row of chapter 03's catalogue with LB1.2's five fields, and the module that backs it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CatalogueRow {
    /// Entry: the row's name in the catalogue.
    pub entry: &'static str,
    /// Level: the puzzle and level the entry reads, or that it operates on values without a state.
    pub level: &'static str,
    /// Effect: chapter 02's effects as the IR draft names them. An empty list is the catalogue's `pure`.
    pub effects: &'static [Effect],
    /// Backing: library code or a KN2.4 row, with its lower-level calls.
    pub backing: &'static str,
    /// Notes: preconditions, coverage and the meaning of an empty or negative answer.
    pub notes: &'static str,
    /// The module whose exported functions back the row.
    pub module: &'static str,
    pub functions: &'static [&'static str],
    /// The SHA-256 of the module's source, in hex.
    pub source_sha256: String,
}

/// LB1.8's row "Literal words" as far as this crate builds it.
pub fn literal_words() -> CatalogueRow {
    CatalogueRow {
        entry: "Literal words",
        level: "600-cell-Full signed generators. C8's `c600_jumble_tool::library` has the jumbling entries.",
        effects: &[],
        backing: "Library code: `words.power` over the IR draft's `concat` and `inverse`. The IR draft's own \
                  operations give concatenation, inverse, conjugate and commutator (LG2.15).",
        notes: "Concatenation, inverse, conjugate, commutator and powers construct literal words; nothing cancels. \
                Construction alone checks no state; application checks each prefix in the kernel (KN2.4). \
                A menu word where `power` takes a `Word` is the checker's `LB1:input.incompatible_alphabet`.",
        module: MODULE,
        functions: &EXPORTS,
        source_sha256: source_sha256(),
    }
}

/// The exact-completion half of LB2's row "Residuals and exact completion", as far as this crate builds it.
pub fn residuals_and_exact_completion() -> CatalogueRow {
    CatalogueRow {
        entry: "Residuals and exact completion",
        level: "Full labels and positions. Exact completion only; the residual half is not built.",
        effects: &[Effect::ReadSnapshot],
        backing: "Library code: `completion.exact_completion` over KN2.4's `positions` and `labels`, each read with \
                  `read_snapshot` at what the host names, the candidates and their home slots; a host serves both \
                  rows (`c600_tools::kernel_rows`).",
        notes: "Exact completion of the named pieces: identities, slots and orientations from `positions` and home \
                labels from `labels`; a piece is complete when every sticker sits in its own home slot. `false` is \
                not established: empty candidate lists, matching colours or a buffer whose occupant is not its home \
                piece never establish completion, and home slots that are not the candidates' own answer `false`.",
        module: COMPLETION_MODULE,
        functions: &COMPLETION_EXPORTS,
        source_sha256: completion_sha256(),
    }
}

/// The F2 and F5 part of LB1.8's row "Exact linear algebra", as far as this crate builds it.
pub fn exact_linear_algebra() -> CatalogueRow {
    CatalogueRow {
        entry: "Exact linear algebra",
        level: "F2 or F5 values. Z, Q and Q(√5) are not built.",
        effects: &[],
        backing: "Library code over KN3's F2 and F5: `linalg.rank_f2`, `linalg.solve_f2`, `linalg.rank_f5` and \
                  `linalg.solve_f5`, Gauss-Jordan elimination with the IR draft's field operations and loops.",
        notes: "Rank and one solution of the supplied equations in the declared field; free unknowns are 0. \
                `solvable: false` is inconsistency of the supplied equations and nothing else. A solution asserts no \
                puzzle reachability. Puzzle-word selection through these equations is a research tier candidate, \
                including correction combination.",
        module: LINALG_MODULE,
        functions: &LINALG_EXPORTS,
        source_sha256: linalg_sha256(),
    }
}

/// Parses and lowers one module's source to the IR draft. A parse diagnostic, a recovery hole or a lowering error is
/// an `Err`. The contract is that of `c600_tools::tool::lower_source`; the library does not depend on the tools
/// crate, which depends on the command, runtime, display and view crates.
pub fn lower_source(source: &str) -> Result<Program, String> {
    lower_sources(&[source])
}

/// Parses and lowers several modules' sources, one module each, into one program. An import names a module by the
/// SHA-256 of its source among them, so a client lowered with [`SOURCE`] that imports [`source_sha256`] calls
/// `words.power`.
pub fn lower_sources(sources: &[&str]) -> Result<Program, String> {
    let mut files = Vec::with_capacity(sources.len());
    for source in sources {
        let file = parse(source.as_bytes());
        if !file.diagnostics.is_empty() || !file.holes.is_empty() {
            return Err(format!("a source does not parse: {:?}", file.diagnostics));
        }
        files.push(file);
    }
    lower(&files).map(|lowered| lowered.program).map_err(|error| format!("the sources do not lower: {error}"))
}

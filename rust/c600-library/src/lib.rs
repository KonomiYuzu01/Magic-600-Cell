//! The base library's first entries as 600 Cell Script (LP3, C5): chapter 03's row "Literal words" (LB1.8), written
//! as language code over the IR draft's word operations (LB1.1).
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
//! The crate draws nothing and owns no state. It depends on the IR, syntax and digest crates only; its tests also run
//! the checker, the interpreter (`c600-run`) and the 600-cell-Full kernel. Every result of its tests is source and
//! fixture evidence.
//!
//! # Drafts
//!
//! Each choice below is this crate's own and settles nothing in the chapters it names.
//!
//! - LB1.8: `power`'s exponent is an `Int`, and a negative power is the inverse written out: `power(w, -n)` is
//!   `power(inverse(w), n)`. The IR draft has no loop, subtraction or negation, so `power` recurses on
//!   `exact_add(n, 1)` or `exact_add(n, -1)` under exact control (LG2.20), and the run's budget bounds the exponent.
//! - LB1.8: a power is a flat `Word` written out in full. A compressed power (LG2.16) is not built.
//! - LB1.2: the catalogue row is Rust data in [`literal_words`], because the IR draft and the syntax carry no module
//!   manifest yet (LG5.2, PS1.15), as in `c600-tools` and `c600-repr`. Its Effect field holds the IR draft's effects,
//!   and an empty list reads as the catalogue's `pure`, so a test compares it with the checker's inferred summary.
//! - LB1.8, LB1.9: no `LB1:input.incompatible_alphabet` refusal is built. The IR draft has one alphabet, 600-cell-Full's
//!   signed generators: a `menu[...]` word does not lower (M5), so a mix of alphabets cannot arise yet.
//!
//! # Not built
//!
//! - Jumbling menu letters in the language: they wait for menu words in the IR draft (M5, LP2). C8's
//!   `c600_jumble_tool::word` builds literal jumbling words of one pinned menu in Rust.
//! - Library entries over kernel rows, such as LB2's "Star and recipe construction" and "Net and Strict protection":
//!   the IR draft cannot pass a run's input as a query's argument (a snapshot read names a literal query only), so the
//!   tests apply a library-built word through the kernel from Rust.
//! - The command layer's preview in Strict mode (KN5, LP2): CT-LB2-02's Strict half runs at the kernel only.
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

/// The SHA-256 of [`SOURCE`], in hex: the digest a client's `import` names.
pub fn source_sha256() -> String {
    sha256_hex(SOURCE.as_bytes())
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
        level: "600-cell-Full signed generators. Jumbling menu letters wait for menu words in the IR draft (M5, LP2).",
        effects: &[],
        backing: "Library code: `words.power` over the IR draft's `concat` and `inverse`. The IR draft's own \
                  operations give concatenation, inverse, conjugate and commutator (LG2.15).",
        notes: "Concatenation, inverse, conjugate, commutator and powers construct literal words; nothing cancels. \
                Construction alone checks no state; application checks each prefix in the kernel (KN2.4). \
                `LB1:input.incompatible_alphabet` cannot arise while the IR draft has one alphabet.",
        module: MODULE,
        functions: &EXPORTS,
        source_sha256: source_sha256(),
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

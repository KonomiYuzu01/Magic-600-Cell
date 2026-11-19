//! The jumbling library entries as 600 Cell Script (LP3, C8; chapter 03's LB1.8 and LB3.1 catalogues), on the IR
//! draft's menu words (LP2's pull request 210). Four modules, each in edition `6cs-draft-1` (syntax candidate A) and
//! in the formatter's canonical layout, checked and run by LP2's checker and interpreter:
//!
//! - `menu_words` ([`MENU_WORDS`]): LB1.8's literal words of one pinned menu, the jumbling half. `power` repeats a menu
//!   word, or its inverse for a negative power; the IR draft's own operations concatenate, invert, conjugate and
//!   commute menu words. It is pure: it reads no state and names no proposal target.
//! - `jumble_heights` ([`HEIGHTS`]): LB3.1's "Heights, branches, domains and Φ". `height_facts` reads KN2.4's `heights`
//!   and `height_counts` and adds, by exact sums, Φ = Σ max(0, h − 2) over the left heights, the largest left height,
//!   and the pieces at heights (0,0) and on the lattice's domain.
//! - `jumble_word_checks` ([`WORD_CHECKS`]): LB3.1's "Release and whole-word checks". `word_observations` reads KN2.4's
//!   `replay` of a literal word and recomputes LB3.7's chronological observations from each prefix's Φ;
//!   `step_flag` reads KN2.4's `word_check` for a named unit and recomputes LB3.8's step flag from its end facts.
//! - `jumble_protection` ([`PROTECTION`]): LB3.1's "Protection" at pose level. `word_protection` reads KN2.4's
//!   `protection` of a literal word in both modes, Net and Strict, each verdict with its level.
//!
//! Every entry tracks, analyses or checks the solver's named word or state; none chooses or outputs a word, and none
//! previews or executes anything (no `propose`). A run pins the word's alphabet with [`crate::word::Alphabet::pinned`].
//!
//! # Drafts
//!
//! Each choice freezes later with the IR, the command table or LB3.9's packaging, which is open.
//!
//! - **Row answers (KN2.4, KN2.5, LB3.9).** No host bridge serves the jumbling rows yet, so the rows' answer types are
//!   this crate's drafts: each is a record of fields named as `c600_jumble`'s answers name them (`observe::heights`,
//!   `observe::prefix_observations`, `observe::word_check`, `levels::WordProtection`), a JSON object read as a `Map`,
//!   and a field that may be undefined, such as an end fact after a refused prefix, declared as a list that is empty
//!   when the field is undefined and holds one value otherwise, since the IR draft has no option type. A program
//!   declares the fields it reads, in any order. Every answer writes each of its fields, null when undefined: a
//!   refused letter's row in `replay`, which `observe::prefix_observations` writes without the after-state facts,
//!   carries them as null. A declared field that the answer does not write, such as a misspelled one, is
//!   `LG2:type.mismatch` and is never read as undefined. The tests' host (`tests/library.rs`) serves the rows from the
//!   kernel API this way and refuses another declared shape with `LG2:type.mismatch`.
//! - **Arguments (KN2.5).** The word rows take the solver's literal word: `replay` takes the menu word itself,
//!   `word_check` a record `{ unit, word }` with the named unit, and `protection` a record `{ mode, pieces, word }`
//!   with the mode as the text `net` or `strict`. The heights rows take none: they are per-revision facts.
//! - **Recomputed separately (LB3.7, LB3.8).** `word_observations` reads each admitted prefix's Φ and largest left
//!   height and none of the kernel's final claims: the peak includes the start, the excursion is the greatest rise of
//!   Φ over the start with the last letter included, and the word is tame when no letter raises Φ. A word with a
//!   refused prefix gets no claim, and its count of admitted prefixes is the refused letter's index. `step_flag` reads
//!   the end facts and not the kernel's flag: the word admitted, every piece of the unit on the lattice, no lattice
//!   piece lifted, and every other off piece landed or at its exact pose.
//! - **Units and caps (LB3.8).** `check_named_word`'s optional unit is a required argument of `step_flag`, since an
//!   empty list would be a named empty unit; `word_observations` names none. The optional named cap, with ΔN, Δn_h,
//!   Δu, the moved sets and the prefix digests, is not taken by these entries.
//! - **Refusals (LB1.8, JI.45, KN2.4).** A row refuses where the alphabet check or `c600_jumble` refuses, and the run
//!   then stops with the row's code and no partial answer: a word whose alphabet is not the state's is
//!   `LB1:input.incompatible_alphabet`, as [`crate::word::LiteralWord::letters`] refuses it; a named unit that is
//!   empty, holds a piece outside the model or on the lattice, or spans two exact poses, and a protected piece outside
//!   the model, are `KN2:input.named_object_invalid`. A word refused at a blocked or invalid prefix has no protection
//!   report in `levels::derive`, so `protection` refuses it with `KN2:input.word_blocked_prefix`, while `replay`
//!   answers its admitted prefixes. A protection conflict refuses no row: its verdict `moves` is the answer.

use c600_digest::sha256_hex;
use c600_ir::{Effect, Program};
use c600_library::{lower_sources, CatalogueRow};

/// One module of the library: its source, its exported functions and the KN2.4 rows it reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Module {
    pub name: &'static str,
    /// Edition `6cs-draft-1`, syntax candidate A, in the formatter's canonical layout.
    pub source: &'static str,
    pub exports: &'static [&'static str],
    /// The KN2.4 rows its functions read with `read_snapshot`, in name order; none for pure code.
    pub rows: &'static [&'static str],
}

impl Module {
    /// The SHA-256 of the source, in hex: the digest a client's `import` names.
    pub fn sha256(&self) -> String {
        sha256_hex(self.source.as_bytes())
    }
}

/// LB1.8's literal words of one pinned menu.
pub const MENU_WORDS: Module =
    Module { name: "menu_words", source: include_str!("menu_words.6cs"), exports: &["power"], rows: &[] };

/// LB3.1's heights, branches, domains and Φ.
pub const HEIGHTS: Module = Module {
    name: "jumble_heights",
    source: include_str!("jumble_heights.6cs"),
    exports: &["height_facts"],
    rows: &["height_counts", "heights"],
};

/// LB3.1's release and whole-word checks: the chronological observations and the step flag.
pub const WORD_CHECKS: Module = Module {
    name: "jumble_word_checks",
    source: include_str!("jumble_word_checks.6cs"),
    exports: &["word_observations", "step_flag"],
    rows: &["replay", "word_check"],
};

/// LB3.1's protection at pose level.
pub const PROTECTION: Module = Module {
    name: "jumble_protection",
    source: include_str!("jumble_protection.6cs"),
    exports: &["word_protection"],
    rows: &["protection"],
};

/// The library's modules.
pub const MODULES: [Module; 4] = [MENU_WORDS, HEIGHTS, WORD_CHECKS, PROTECTION];

/// Parses and lowers the library's modules and `clients`, one module each, into one program
/// (`c600_library::lower_sources`). A client imports `menu_words` by [`Module::sha256`].
pub fn program(clients: &[&str]) -> Result<Program, String> {
    let sources: Vec<&str> = MODULES.iter().map(|m| m.source).chain(clients.iter().copied()).collect();
    lower_sources(&sources)
}

fn row(entry: &'static str, level: &'static str, effects: &'static [Effect], module: Module) -> CatalogueRow {
    CatalogueRow {
        entry,
        level,
        effects,
        backing: "",
        notes: "",
        module: module.name,
        functions: module.exports,
        source_sha256: module.sha256(),
    }
}

/// LB1.8's row "Literal words", the jumbling half.
pub fn literal_words() -> CatalogueRow {
    CatalogueRow {
        backing:
            "Library code: `menu_words.power` over the IR draft's `concat` and `inverse` and the empty menu word. \
                  The IR draft's own operations give concatenation, inverse, conjugate and commutator of menu words \
                  (LG2.15; LP2's pull request 210).",
        notes: "Concatenation, inverse, conjugate, commutator and powers construct literal words of the run's pinned \
                alphabet; nothing cancels, and an inverse takes the pinned menu's inverse items. Construction alone \
                checks no state; application checks each prefix in the kernel (KN2.4 `replay`). An argument of \
                another alphabet, or a menu word in a run with none pinned, is `LB1:input.incompatible_alphabet`, as \
                is a `Word` where `power` takes a `MenuWord`, which the checker gives.",
        ..row("Literal words", "Jumbling menu letters of one pinned menu.", &[], MENU_WORDS)
    }
}

/// LB3.1's row "Heights, branches, domains and Φ".
pub fn heights_branches_domains_and_phi() -> CatalogueRow {
    CatalogueRow {
        backing: "Library code: `jumble_heights.height_facts` over KN2.4's `heights` and `height_counts`, each read \
                  with `read_snapshot`, and exact sums.",
        notes: "Heights, branches and domain IDs per distinct pose; n₀, n₂, n₄, n₆ and above-6 data; the largest left \
                height; Φ = Σ max(0, h − 2) over the left heights. A zero Φ permits height-2 pieces and is not the \
                lattice or sticker test. The pieces on the lattice are counted by domain ID, which the kernel decides \
                by exact coset tests, never by heights.",
        ..row("Heights, branches, domains and Φ", "Jumbling poses.", &[Effect::ReadSnapshot], HEIGHTS)
    }
}

/// LB3.1's row "Release and whole-word checks", as far as this crate builds it.
pub fn release_and_whole_word_checks() -> CatalogueRow {
    CatalogueRow {
        backing: "Library code: `jumble_word_checks.word_observations` over KN2.4's `replay` and \
                  `jumble_word_checks.step_flag` over KN2.4's `word_check`, each read with `read_snapshot` for the \
                  solver's literal word.",
        notes: "Admitted prefixes and, for a word admitted whole, its end digest, peak, excursion and tameness, \
                recomputed letter by letter from each prefix's Φ; the step flag for a named unit, recomputed from the \
                check's end facts. A false flag refuses nothing. An unplayed suffix stays unchecked: a refused prefix \
                gives no claim. Not built: the named cap, ΔN, Δn_h, Δu, moved sets and prefix digests.",
        ..row("Release and whole-word checks", "Jumbling poses.", &[Effect::ReadSnapshot], WORD_CHECKS)
    }
}

/// LB3.1's row "Protection", at pose level.
pub fn protection() -> CatalogueRow {
    CatalogueRow {
        backing: "Library code: `jumble_protection.word_protection` over KN2.4's `protection`, read once in each \
                  mode for the solver's literal word and protected pieces (M8's `levels::derive`, pull request 182).",
        notes: "Net and Strict verdicts, each with its level `poses`, its mode, the protected pieces, the pieces the \
                judged operation moves and Strict's first refused prefix. The verdict is separate from the step flag \
                (LB3.8). A label verdict grants no pose protection (LB3.4).",
        ..row(
            "Protection",
            "Jumbling poses; the label half after crossing is not built here.",
            &[Effect::ReadSnapshot],
            PROTECTION,
        )
    }
}

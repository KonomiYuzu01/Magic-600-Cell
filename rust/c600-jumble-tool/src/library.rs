//! The jumbling library entries as 600 Cell Script (LP3, C8; chapter 03's LB1.8, LB1.10 and LB3.1 catalogues), on
//! the IR draft's menu words (LP2's pull request 210) and its F2 and F5 (LP2's pull request 222). Twelve modules, each in
//! edition `6cs-draft-1` (syntax candidate A) and in the formatter's canonical layout, checked and run by LP2's checker
//! and interpreter:
//!
//! - `menu_words` ([`MENU_WORDS`]): LB1.8's literal words of one pinned menu, the jumbling half. `power` repeats a menu
//!   word, or its inverse for a negative power; the IR draft's own operations concatenate, invert, conjugate and
//!   commute menu words. It is pure: it reads no state and names no proposal target.
//! - `menu_traces` ([`MENU_TRACES`]): LB1.10's trace equality keys, the jumbling half. `same_trace` compares the keys
//!   of two menu words under the relation `jumble-locality/1` the run pins ([`crate::locality::relation`]), and
//!   `trace_commute` asks whether xy and yx have one key. It is pure, like `menu_words`.
//! - `jumble_heights` ([`HEIGHTS`]): LB3.1's "Heights, branches, domains and Φ". `height_facts` reads KN2.4's `heights`
//!   and `height_counts` and adds, by exact sums, Φ = Σ max(0, h − 2) over the left heights, the largest left height,
//!   and the pieces at heights (0,0) and on the lattice's domain.
//! - `jumble_word_checks` ([`WORD_CHECKS`]): LB3.1's "Release and whole-word checks". `word_observations` reads KN2.4's
//!   `replay` of a literal word and recomputes LB3.7's chronological observations from each prefix's Φ;
//!   `step_flag` reads KN2.4's `word_check` for a named unit and recomputes LB3.8's step flag from its end facts.
//! - `jumble_protection` ([`PROTECTION`]): LB3.1's "Protection" at pose level. `word_protection` reads KN2.4's
//!   `protection` of a literal word in both modes, Net and Strict, each verdict with its level.
//! - `jumble_nu` ([`NU`]): LB3.1's "ν and drafted correction-set check". `nu_facts` reads KN2.4's `nu` and lifts ν's
//!   45 values into F2 (43 bits) and F5 (the two C5 values); `drafted_check` also reads `correction_set_check` for
//!   the solver's drafted set and computes, in F2 and F5, the change the set makes: ν after the set less ν.
//! - `jumble_rim` ([`RIM`]): LB3.1's "Rim and seam facts; N". `rim_facts` reads KN2.4's `rim_summary` and one whole
//!   `seam_facts` scan, passes every cap's row through in cap-ID order, and counts by exact sums the admissible,
//!   blocked, closed, misaligned and at-floor caps and the sum of N_c. Total N is the scan's, not that sum.
//! - `jumble_covers` ([`COVERS`]): LB3.1's "Covers at a named cap". `covers_at` reads KN2.4's `covers` for the
//!   solver's cap and answers both directions for every meeting cap in cap-ID order, with no top-cap verdict.
//! - `jumble_corrections` ([`CORRECTIONS`]): LB3.1's "Correction catalogue". `correction` reads `correction_lookup`
//!   for the solver's id and answers that correction's letters, length, ν row and support; it chooses no combination.
//! - `jumble_admissibility` ([`ADMISSIBILITY`]): LB3.1's "Admissibility and blocked-twist check". `twist_check` reads
//!   KN2.4's `classify` for the solver's named twist, a menu word of one letter, and `grip_status` reads `survey` for
//!   the solver's named grip only.
//! - `jumble_straddlers` ([`STRADDLERS`]): LB3.1's "Straddlers and u at a named cap". `straddlers_at` reads KN2.4's
//!   `straddlers` for the solver's cap and answers the complete set, each straddler with its exact certificate pair,
//!   and u, the count of straddlers.
//! - `jumble_units` ([`UNITS`]): LB3.1's "Units and clean-step contract". `off_units` reads JI.9's `units` of every
//!   piece off the lattice and `named_unit` the solver's named unit; `clean_step` reads KN2.4's `word_check` for the
//!   solver's word and unit and answers the contract's parts: the unit landed, the lattice pieces lifted off and the
//!   other off pieces kept at their exact pose or landed, and whether the contract holds.
//!
//! Every entry tracks, analyses or checks the solver's named word or state; none chooses or outputs a word, and none
//! previews or executes anything (no `propose`). A run pins the word's alphabet with [`crate::word::Alphabet::pinned`].
//!
//! # Drafts
//!
//! Each choice freezes later with the IR, the command table or LB3.9's packaging, which is open.
//!
//! - **Row answers (KN2.4, KN2.5, LB3.9).** The rows' answer types are this crate's drafts, as the host bridge
//!   [`crate::rows`] serves them: each is a record of fields named as `c600_jumble`'s answers name them
//!   (`observe::heights`, `observe::prefix_observations`, `observe::word_check`, `levels::WordProtection`,
//!   `observe::cuts::rim_summary`, `observe::cuts::covers`, `corrections::Catalogue::lookup`, `state::Outcome`,
//!   `state::StraddlerSet`, `observe::units`), a JSON
//!   object read as a `Map`, and a field that may be undefined, such as an end fact after a refused prefix, declared as
//!   a list that is empty when the field is undefined and holds one value otherwise, since the IR draft has no option
//!   type. A program declares the fields it reads, in any order. Every answer writes each of its fields, null when
//!   undefined: a refused letter's row in `replay`, which `observe::prefix_observations` writes without the after-state
//!   facts, carries them as null. A declared field that the answer does not write, such as a misspelled one, is
//!   `LG2:type.mismatch` and is never read as undefined. The bridge serves the rows from the kernel API this way and
//!   refuses another declared shape with `LG2:type.mismatch`.
//! - **Arguments (KN2.5).** The word rows take the solver's literal word: `replay` takes the menu word itself,
//!   `word_check` a record `{ unit, word }` with the named unit, and `protection` a record `{ mode, pieces, word }`
//!   with the mode as the text `net` or `strict`. The heights rows and `nu` take none: they are per-revision facts.
//!   `correction_set_check` takes the drafted set as a list of records `{ id, m }`, the correction's id and its signed
//!   multiplicity, in the solver's order. `rim_summary` takes none; `covers` takes the solver's cap ID,
//!   `correction_lookup` the solver's correction id, and `seam_facts` a cap ID or the text `all` for one whole scan.
//!   `classify` takes the solver's named twist as a menu word of one letter, `survey` and `straddlers` the solver's
//!   cap ID, and `units` the named unit as a list of pieces or the text `all` for every unit off the lattice.
//! - **Exact numbers (KN2.4).** A straddle certificate's points and cut heights are numbers of Q(√5), declared `Q5`:
//!   the bridge reads the engine's `[a, b, d]` for (a + b√5)/d, never a float.
//! - **ν's answer (KN2.4, A5).** `nu` and `correction_set_check` answer a record `{ reason, values }`: ν's 45 values
//!   as integers and no reason when ν is defined, no values and the reason (`off_lattice`) when it is not. A field
//!   value has no canonical form, so the entries return residues, each value's least non-negative representative.
//! - **Recomputed separately (LB3.7, LB3.8).** `word_observations` reads each admitted prefix's Φ and largest left
//!   height and none of the kernel's final claims: the peak includes the start, the excursion is the greatest rise of
//!   Φ over the start with the last letter included, and the word is tame when no letter raises Φ. A word with a
//!   refused prefix gets no claim, and its count of admitted prefixes is the refused letter's index. `step_flag` reads
//!   the end facts and not the kernel's flag: the word admitted, every piece of the unit on the lattice, no lattice
//!   piece lifted, and every other off piece landed or at its exact pose. `covers_at` recomputes each direction's
//!   face-piece flag from its counts and the answer's threshold (at least the threshold inside, and the inside,
//!   outside and aligned counts equal), and `rim_facts` counts the caps by kind from their rows.
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

/// LB1.10's trace equality keys of menu words, the jumbling half.
pub const MENU_TRACES: Module = Module {
    name: "menu_traces",
    source: include_str!("menu_traces.6cs"),
    exports: &["same_trace", "trace_commute"],
    rows: &[],
};

/// LB3.1's ν and drafted correction-set check, on F2 and F5.
pub const NU: Module = Module {
    name: "jumble_nu",
    source: include_str!("jumble_nu.6cs"),
    exports: &["nu_facts", "drafted_check"],
    rows: &["correction_set_check", "nu"],
};

/// LB3.1's rim and seam facts and N, at every cap.
pub const RIM: Module = Module {
    name: "jumble_rim",
    source: include_str!("jumble_rim.6cs"),
    exports: &["rim_facts"],
    rows: &["rim_summary", "seam_facts"],
};

/// LB3.1's covers at a named cap.
pub const COVERS: Module = Module {
    name: "jumble_covers",
    source: include_str!("jumble_covers.6cs"),
    exports: &["covers_at"],
    rows: &["covers"],
};

/// LB3.1's correction catalogue, looked up by the solver's id.
pub const CORRECTIONS: Module = Module {
    name: "jumble_corrections",
    source: include_str!("jumble_corrections.6cs"),
    exports: &["correction"],
    rows: &["correction_lookup"],
};

/// LB3.1's admissibility and blocked-twist check, for the solver's named twist or grip.
pub const ADMISSIBILITY: Module = Module {
    name: "jumble_admissibility",
    source: include_str!("jumble_admissibility.6cs"),
    exports: &["twist_check", "grip_status"],
    rows: &["classify", "survey"],
};

/// LB3.1's straddlers and u at a named cap.
pub const STRADDLERS: Module = Module {
    name: "jumble_straddlers",
    source: include_str!("jumble_straddlers.6cs"),
    exports: &["straddlers_at"],
    rows: &["straddlers"],
};

/// LB3.1's units and clean-step contract.
pub const UNITS: Module = Module {
    name: "jumble_units",
    source: include_str!("jumble_units.6cs"),
    exports: &["off_units", "named_unit", "clean_step"],
    rows: &["units", "word_check"],
};

/// The library's modules.
pub const MODULES: [Module; 12] = [
    MENU_WORDS,
    MENU_TRACES,
    HEIGHTS,
    WORD_CHECKS,
    PROTECTION,
    NU,
    RIM,
    COVERS,
    CORRECTIONS,
    ADMISSIBILITY,
    STRADDLERS,
    UNITS,
];

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

/// LB1's row "Trace equality keys", the jumbling half (LB1.10).
pub fn trace_equality_keys() -> CatalogueRow {
    CatalogueRow {
        backing: "Library code: `menu_traces.same_trace` and `menu_traces.trace_commute` over the IR draft's \
                  `trace_key` on menu words and `exact_eq` (LP2's pull request 251), under the relation \
                  `jumble-locality/1` that the run pins beside its menu alphabet (`locality::relation`).",
        notes: "Two literal words of one pinned menu have one key exactly when adjacent swaps of letters at \
                independent grips reach one from the other; letters at one grip never swap. Every occurrence stays, \
                identity letters and an item beside its inverse among them, so a key keeps each grip's domain test. \
                The key recognizes commuting swaps only and searches for no word. A key under another menu, model, \
                key or relation version is `LB1:input.key_contract_mismatch`, and a word of another alphabet is \
                `LB1:input.incompatible_alphabet`. Word keys are separate (LG2.18's jumbling word key is not built).",
        ..row("Trace equality keys", "Jumbling menu words under `jumble-locality/1`.", &[], MENU_TRACES)
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

/// LB3.1's row "ν and drafted correction-set check".
pub fn nu_and_drafted_correction_set_check() -> CatalogueRow {
    CatalogueRow {
        backing: "Library code: `jumble_nu.nu_facts` over KN2.4's `nu` and `jumble_nu.drafted_check` over KN2.4's \
                  `nu` and `correction_set_check`, each read with `read_snapshot`, on the IR draft's F2 and F5 \
                  (LP2's pull request 222).",
        notes: "ν's 45 values in the fixed C5 convention of A5: 43 bits in F2, then the two C5 values in F5. Off the \
                lattice ν is undefined, with its reason and no values, never stale coordinates, zeros or a partial \
                ν. A drafted set is the solver's list of (id, signed multiplicity); the check answers ν after the set \
                and the change the set makes, computed in F2 and F5, and selects, ranks or reduces no set. A nonzero \
                fixed I_a coordinate on a reachable I_a result is `KN2:consistency.fixed_nu_nonzero`, an id the \
                catalogue does not ship is `KN2:input.named_object_invalid`, and a base under another menu is \
                `KN2:input.menu_binding_mismatch`.",
        ..row(
            "ν and drafted correction-set check",
            "Jumbling labels, on the lattice only.",
            &[Effect::ReadSnapshot],
            NU,
        )
    }
}

/// LB3.1's row "Rim and seam facts; N".
pub fn rim_and_seam_facts() -> CatalogueRow {
    CatalogueRow {
        backing: "Library code: `jumble_rim.rim_facts` over KN2.4's `rim_summary` (M8's `observe::cuts::rim_summary`, \
                  pull request 176) and `seam_facts` with scope `all` (`observe::seam_facts`), each read with \
                  `read_snapshot`, and exact sums.",
        notes: "N_c, In_c, Out_c, A_c, F_c and whether N_c is at the floor at every cap, in cap-ID order, with each \
                cap's status; a blocked cap's rim is undefined, never zero. Counts of the admissible, blocked, closed, \
                misaligned and at-floor caps, and grip N, the kernel's and the exact sum of N_c over the admissible \
                caps. Total N comes from the seam scan of every domain, with its on-grip and off-grip mismatches, and \
                is not the sum of the displayed N_c. Facts of one revision: no twist or word is evaluated, no cap is \
                skipped, ranked or selected, and the entry holds no table of outcomes over menu items (LB3.5).",
        ..row("Rim and seam facts; N", "Jumbling poses.", &[Effect::ReadSnapshot], RIM)
    }
}

/// LB3.1's row "Covers at a named cap".
pub fn covers_at_a_named_cap() -> CatalogueRow {
    CatalogueRow {
        backing: "Library code: `jumble_covers.covers_at` over KN2.4's `covers` (M8's `observe::cuts::covers`, pull \
                  request 176), read with `read_snapshot` for the solver's named cap.",
        notes: "Every cap d meeting the named cap c, in cap-ID order, in both directions: the face-piece counts, the \
                straddlers of a blocked d, and DT-J1's exact face-piece definition recomputed from the counts and the \
                kernel's threshold; \"c covers d\" is not applicable, with its reason, at a blocked or closed d. A \
                blocked or closed named cap answers its status and message and no answers; a cap outside the model \
                is `KN2:input.named_object_invalid`. No top-cap verdict, mark, ranking or method-selected target.",
        ..row("Covers at a named cap", "Jumbling poses.", &[Effect::ReadSnapshot], COVERS)
    }
}

/// LB3.1's row "Correction catalogue".
pub fn correction_catalogue() -> CatalogueRow {
    CatalogueRow {
        backing: "Library code: `jumble_corrections.correction` over the shipped constructions (LP2's \
                  `corrections::catalogue`, pull request 206), read through the bridge's `correction_lookup` with \
                  `read_snapshot` for the solver's id.",
        notes: "The correction the solver names by id: its family, its literal word as (grip, item) letters of the \
                pinned I_a menu, its length, its fixed ν row in A5's convention and the row's support. Lookup by id \
                chooses no combination: the entry reads one id and returns that correction only. An id the catalogue \
                does not ship is `KN2:input.named_object_invalid`. The letters number I_a's items in its recorded \
                order, so a state under another menu or order, or in free mode, is \
                `KN2:input.menu_binding_mismatch`. The effect is `snapshot` where LB3.1 says pure, \
                since the IR draft has no static data and the catalogue is read through the host (a draft). The word \
                check of a selected correction is the whole-word check's.",
        ..row(
            "Correction catalogue",
            "Jumbling labelled action, with literal menu words.",
            &[Effect::ReadSnapshot],
            CORRECTIONS,
        )
    }
}

/// LB3.1's row "Admissibility and blocked-twist check".
pub fn admissibility_and_blocked_twist_check() -> CatalogueRow {
    CatalogueRow {
        backing: "KN2.4's `classify` and `survey`: `jumble_admissibility.twist_check` reads `classify` for the \
                  solver's named twist and `jumble_admissibility.grip_status` reads `survey` for the solver's named \
                  grip, each with `read_snapshot` (M8's `State::apply` on a copy and `State::survey`).",
        notes:
            "Admissible, blocked, invalid or uncertain for the named twist, with the count of pieces it would move, \
                the straddle certificate of a blocked twist, the pieces left uncertain and the refusal's code; the \
                status of the named grip. A refusal changes nothing: the twist acts on a copy of the state read. The \
                survey row answers one named grip only: the research survey's all-items loop is not served (LB3.5).",
        ..row("Admissibility and blocked-twist check", "Jumbling poses.", &[Effect::ReadSnapshot], ADMISSIBILITY)
    }
}

/// LB3.1's row "Straddlers and u at a named cap".
pub fn straddlers_and_u_at_a_named_cap() -> CatalogueRow {
    CatalogueRow {
        backing: "KN2.4's `straddlers`: `jumble_straddlers.straddlers_at` reads it with `read_snapshot` for the \
                  solver's named cap (M8's `State::straddler_set`).",
        notes: "The complete set of the named cap's straddlers, ascending, each with its exact certificate pair: a \
                point of the posed piece region on either side of the cut, each coordinate in Q(√5), with both cut \
                heights. u is the count of straddlers; complete means no piece is left uncertain. Empty after \
                completion means no straddler at that cap and input, not a useful lowering.",
        ..row("Straddlers and u at a named cap", "Jumbling poses.", &[Effect::ReadSnapshot], STRADDLERS)
    }
}

/// LB3.1's row "Units and clean-step contract".
pub fn units_and_clean_step_contract() -> CatalogueRow {
    CatalogueRow {
        backing: "Library code: `jumble_units.off_units` and `jumble_units.named_unit` over JI.9's `units` (the \
                  exact-pose groups of the off pieces, from the pose table and pose IDs) and `jumble_units.clean_step` \
                  over KN2.4's `word_check`, each read with `read_snapshot` for the solver's unit and word.",
        notes: "The units off the lattice, or the named unit, each with its members, exact pose key, domain, heights \
                and branches; for a named word and unit, the contract's parts: whether the unit landed, the lattice \
                pieces lifted off, and the other off pieces kept at their exact pose or landed, with the ones changed. \
                A false contract is an observed failure of that contract, not by itself a refusal to preview; a \
                refused prefix leaves the parts undefined and the contract false. Not built: a general clean step's \
                declared free set, which `word_check` does not take.",
        ..row("Units and clean-step contract", "Jumbling poses.", &[Effect::ReadSnapshot], UNITS)
    }
}

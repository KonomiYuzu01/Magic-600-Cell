//! The research library's entries as 600 Cell Script (LP5, piece R7), language code over the IR draft (LB1.1): LB1's
//! rows "Straight-line programs", "Big integers and symbolic bounds", "Finite fixed points" and "Exact matching and
//! integral flows", and LB5's rows "Restricted word search", "Search reductions and gates", "Admitted-domain
//! membership", "Strategies, selectors and ranks" (the ranks), "Bound calculation and compressed witnesses",
//! "Regional-family recognition" and "Conditional models and partial actions" (chapter 03, LB1.8 and LB5.1 to LB5.6).
//!
//! Each module is a source in edition `6cs-draft-1` (syntax candidate A), in the formatter's canonical layout, with
//! its module name, its exports in source order and the SHA-256 of its source, and each row is a [`CatalogueRow`]
//! with LB1.2's five fields. The modules read no state: the checker infers no state read, row or capability for any
//! export, so every answer says something about the supplied values only. The searching entries take a synthetic
//! permutation action as data (a letter's images of the points), never a puzzle state, and their outputs enter no
//! research evidence (LP5 plan, section 9, reading B; PHS.31).
//!
//! # Modules
//!
//! - [`SLP_SOURCE`], `research_slp`: straight-line programs as the IR draft's compressed words (LG2.16). A program is
//!   a list of rules, each a leaf of literal letters or the concatenation of two earlier rules, and the last rule is
//!   the word; `program_word` builds it for a client module. A compressed word has no canonical form, so a run cannot
//!   hand one to its host, and the other entries take the rules: the exact length and cost come without expansion;
//!   `streamed_length` expands chunk by chunk and counts the letters it expanded, and reads the literal digest with
//!   `word_stream`; `admitted_expansion` refuses an expansion over its bound with the exact length before any letter
//!   is expanded (LB1.11).
//! - [`BIGINT_SOURCE`], `research_bigint`: binomials, products, an affine recurrence, sums of binomial terms and
//!   decimal digit counts of integers of any size, and a symbolic bound that keeps its named premises and says it was
//!   neither expanded nor replayed. Stage 19's B11 has 6,775 digits (LB5.5).
//! - [`FIXPOINTS_SOURCE`], `research_fixpoints`: least and greatest fixed points of the monotone operator of an AND/OR
//!   graph, iterated from the empty and the full set, each with the check that the operator maps it to itself.
//! - [`FLOWS_SOURCE`], `research_flows`: a maximum bipartite matching by augmenting paths, an integral feasible flow
//!   with supplies and demands by shortest augmenting paths over a source and a sink, with the residual cut, and the
//!   two witness checks.
//! - [`SEARCH_SOURCE`], `research_search`: the restricted word search with its record (LB5.2), the transport of a
//!   reduced negative through a cited lemma (LB5.3) and admitted-domain membership D\[k\] (LB5.4).
//! - [`RANKS_SOURCE`], `research_ranks`: stage 19's rank R_L, the lexicographic order of typed vectors, decrease on
//!   stored blocks, and the fixed-length and conditional cost bounds with their named premises (LB5.5).
//! - [`FAMILIES_SOURCE`], `research_families`: recognition of an instance by a type-preserving bijection onto a full
//!   reference table, through `research_flows.max_matching`, which it imports by the SHA-256 of its source (LB5.6).
//! - [`MODELS_SOURCE`], `research_models`: restrictions and quotients of a permutation action that state their maps and
//!   preserved subjects, recognition of a supplied instance, and conditional bounds whose premises accumulate and are
//!   discharged only by an identical statement and scope.
//!
//! # The human-solve boundary
//!
//! Every row whose notes begin with [`MARKER`] is a research tier candidate (LB1.5, BND.64): the searching entries and
//! the rows chapter 03 marks. [`research_tier_rows`] lists them, the data a host's shipped-manifest scan reads
//! (CT-BND-16). No entry chooses, ranks or outputs a solving word: the search and the membership count witnesses and
//! give the least witness length, and carry no witness letters.
//!
//! # Drafts
//!
//! Each choice below is this module's own and settles nothing in the chapters it names.
//!
//! - LB5.2, the search: the family is every word of at most `max_length` letters over the action's letters, enumerated
//!   by length, then by letter order. The deterministic budget is a number of examined candidates, the record's cost
//!   unit `candidates`; a declaration with another unit, an applicable seed (the enumeration draws nothing) or an
//!   unknown reduction is not run (DET.30's `not run`, stop `unsupported`). The search runs in a child scope with no
//!   caps of its own (DET.60), so an error in it answers outcome `error` with no coverage. The first check in it is
//!   the input's: at least one letter, rows of one length, and the images, the start point and every goal point among
//!   the action's points; malformed input is such an error. A stop of the run's own budget or an interruption (a
//!   clock or the solver) ends the run with no record at all. Neither gives a negative.
//! - LB5.2, coverage: `complete_lengths` counts the lengths 0, 1, ... whose words were all examined. A level after
//!   the first beam discard is not complete, so a beamed run that discarded a candidate never reads as full coverage;
//!   `discarded` counts the discards.
//! - LB5.2, outcomes: `found`, `empty` (a completed enumeration of the whole declared, or reduced, family),
//!   `not_found_in_beam` (completed, but the beam discarded candidates), `not_found_within_budget`, `not_run`,
//!   `error` and `cut_off`. Only `empty` is a negative, for the declared family or, with a reduction, for the reduced
//!   family; `negative_family` names which.
//! - LB5.3, reductions: the one reduction built is the local commuting reduction: a candidate whose last two letters
//!   are declared independent and out of letter order is skipped. Its negative stays reduced until
//!   `transport_negative` is given a lemma whose family, alphabet size, hypotheses, reduction and conclusion
//!   (`no_witness_in_unreduced_family`) equal the record's; a mismatch keeps it reduced with
//!   `LB5:evidence.reduction_scope_mismatch` in `errors`. The entry checks the lemma's scope only; whether the lemma is
//!   true is the lemma's.
//! - LB5.4, membership: D\[k\] is "within k letters of a target point" under the action; the answer is `inside` with
//!   the least depth, `OUTSIDE_CERTIFIED_DOMAIN` only after every word of at most k letters was examined, and `unknown`
//!   after a budget stop or an error. The scope is k, the domain's name and the reachability promise, and every answer
//!   says `reachability: not_asserted`. Its input is checked as the search's, with the targets as goals and the
//!   input as start, so a target or an input outside the action's points answers `unknown` with stop `error`.
//! - LB1.11, admission: no LB1 code is registered for an expansion over a library entry's bound, so
//!   `admitted_expansion` answers the refusal as a value with `admitted: false` and the exact length; chapter 05's
//!   `PS1:resource_limit.word_expansion` stays the stored-word path's.
//! - LB5.5, bounds: the premises are named `availability`, `invariant_domain`, `selector` and `strict_decrease`, after
//!   the stage 19 README's condition that every admitted nonterminal state has a legal selected macro that stays
//!   admitted and strictly decreases R_L. The conditional cost is a rational, because max(H-2,0)/2 need not be whole.
//! - LB5.6, recognition: two poses fit when they are equal or the gauge relation pairs them, as given; the gauge is
//!   data and the entry computes no group closure.
//! - LB1.2: the rows are Rust data, as for the base library, because the IR draft carries no module manifest yet.
//! - LB1.2, effects: each row's Effect field is the set of effects the checker infers for its functions, and it is a
//!   subset of chapter 03's Effect column for the row. Where the chapter lists `pure, budgets` (LB1's four rows and
//!   the bounds), `budgets` is the most an entry of the row may use; these entries use none and are pure. A `read a
//!   snapshot` in the column belongs to the puzzle forms, which are not built.
//!
//! # Not built
//!
//! - The puzzle forms of the searching entries over KN2.4's `classify`, `effect`, `replay` and observations, and the
//!   jumbling form of D\[k\] over branch predicates: they wait for the research host's reads (LP5 plan, section 4,
//!   selection 6).
//! - Strategies and selectors (P*, P**, P***, T3P chains, routing, WC-PLAN), release structures, theorem-schema
//!   instance checks, closure-set negative certificates, experiment runs and branches, certificate builders and the
//!   preflight `check_availability` (R1).
//! - Seeded randomness in a search, beams other than a fixed width, and reductions other than the local commuting one.
//! - The host's refusal of a research tier candidate in a solving session: the marker is data here, and the scan that
//!   reads it is the host's (BND.64, CT-BND-16).

use c600_digest::sha256_hex;
use c600_ir::Effect;

use crate::CatalogueRow;

/// The exact marker chapter 03 gives a research tier candidate (LB1.5); a row is one when its notes begin with it.
pub const MARKER: &str = "research tier candidate";

/// `LB5:evidence.reduction_scope_mismatch` (LB5.3): the code `research_search.transport_negative` answers in `errors`
/// when the cited lemma has another family, alphabet, hypotheses, reduction or conclusion.
pub const REDUCTION_SCOPE_MISMATCH: &str = "LB5:evidence.reduction_scope_mismatch";

/// The straight-line programs source: edition `6cs-draft-1`, syntax candidate A, in the formatter's canonical layout.
pub const SLP_SOURCE: &str = include_str!("research_slp.6cs");
/// The straight-line programs module.
pub const SLP_MODULE: &str = "research_slp";
/// The straight-line programs module's exported functions, in source order.
pub const SLP_EXPORTS: [&str; 5] =
    ["program_word", "program_length", "program_cost", "streamed_length", "admitted_expansion"];

/// The big integers source: edition `6cs-draft-1`, syntax candidate A, in the formatter's canonical layout.
pub const BIGINT_SOURCE: &str = include_str!("research_bigint.6cs");
/// The big integers and symbolic bounds module.
pub const BIGINT_MODULE: &str = "research_bigint";
/// The big integers module's exported functions, in source order.
pub const BIGINT_EXPORTS: [&str; 6] =
    ["binomial", "product", "affine_recurrence", "binomial_sum", "digit_count", "symbolic_bound"];

/// The finite fixed points source: edition `6cs-draft-1`, syntax candidate A, in the formatter's canonical layout.
pub const FIXPOINTS_SOURCE: &str = include_str!("research_fixpoints.6cs");
/// The finite fixed points module.
pub const FIXPOINTS_MODULE: &str = "research_fixpoints";
/// The finite fixed points module's exported functions, in source order.
pub const FIXPOINTS_EXPORTS: [&str; 4] =
    ["least_fixed_point", "greatest_fixed_point", "apply_operator", "is_fixed_point"];

/// The matching and flows source: edition `6cs-draft-1`, syntax candidate A, in the formatter's canonical layout.
pub const FLOWS_SOURCE: &str = include_str!("research_flows.6cs");
/// The exact matching and integral flows module.
pub const FLOWS_MODULE: &str = "research_flows";
/// The matching and flows module's exported functions, in source order.
pub const FLOWS_EXPORTS: [&str; 4] = ["max_matching", "check_matching", "feasible_flow", "check_flow"];

/// The search source: edition `6cs-draft-1`, syntax candidate A, in the formatter's canonical layout.
pub const SEARCH_SOURCE: &str = include_str!("research_search.6cs");
/// The search, reductions and membership module.
pub const SEARCH_MODULE: &str = "research_search";
/// The search module's exported functions, in source order.
pub const SEARCH_EXPORTS: [&str; 3] = ["search", "transport_negative", "membership"];
/// The exported function behind LB5's row "Restricted word search".
pub const SEARCH_WORD_SEARCH: [&str; 1] = ["search"];
/// The exported function behind LB5's row "Search reductions and gates".
pub const SEARCH_REDUCTIONS: [&str; 1] = ["transport_negative"];
/// The exported function behind LB5's row "Admitted-domain membership".
pub const SEARCH_MEMBERSHIP: [&str; 1] = ["membership"];

/// The ranks and bounds source: edition `6cs-draft-1`, syntax candidate A, in the formatter's canonical layout.
pub const RANKS_SOURCE: &str = include_str!("research_ranks.6cs");
/// The ranks and bounds module.
pub const RANKS_MODULE: &str = "research_ranks";
/// The ranks module's exported functions, in source order.
pub const RANKS_EXPORTS: [&str; 5] = ["rank", "lex_less", "block_decrease", "fixed_length_bound", "conditional_cost"];
/// The exported functions behind the rank part of LB5's row "Strategies, selectors and ranks".
pub const RANKS_RANKS: [&str; 3] = ["rank", "lex_less", "block_decrease"];
/// The exported functions behind LB5's row "Bound calculation and compressed witnesses".
pub const RANKS_BOUNDS: [&str; 2] = ["fixed_length_bound", "conditional_cost"];

/// The family recognition source: edition `6cs-draft-1`, syntax candidate A, in the formatter's canonical layout. It
/// imports [`FLOWS_SOURCE`] by its SHA-256, so it lowers together with it.
pub const FAMILIES_SOURCE: &str = include_str!("research_families.6cs");
/// The regional-family recognition module.
pub const FAMILIES_MODULE: &str = "research_families";
/// The family recognition module's exported functions, in source order.
pub const FAMILIES_EXPORTS: [&str; 1] = ["recognize"];

/// The conditional models source: edition `6cs-draft-1`, syntax candidate A, in the formatter's canonical layout.
pub const MODELS_SOURCE: &str = include_str!("research_models.6cs");
/// The conditional models and partial actions module.
pub const MODELS_MODULE: &str = "research_models";
/// The conditional models module's exported functions, in source order.
pub const MODELS_EXPORTS: [&str; 6] =
    ["restriction", "quotient", "recognize_instance", "conditional_bound", "combine", "discharge"];

/// The SHA-256 of [`SLP_SOURCE`], in hex.
pub fn slp_sha256() -> String {
    sha256_hex(SLP_SOURCE.as_bytes())
}

/// The SHA-256 of [`BIGINT_SOURCE`], in hex.
pub fn bigint_sha256() -> String {
    sha256_hex(BIGINT_SOURCE.as_bytes())
}

/// The SHA-256 of [`FIXPOINTS_SOURCE`], in hex.
pub fn fixpoints_sha256() -> String {
    sha256_hex(FIXPOINTS_SOURCE.as_bytes())
}

/// The SHA-256 of [`FLOWS_SOURCE`], in hex: the digest `research_families` imports.
pub fn flows_sha256() -> String {
    sha256_hex(FLOWS_SOURCE.as_bytes())
}

/// The SHA-256 of [`SEARCH_SOURCE`], in hex.
pub fn search_sha256() -> String {
    sha256_hex(SEARCH_SOURCE.as_bytes())
}

/// The SHA-256 of [`RANKS_SOURCE`], in hex.
pub fn ranks_sha256() -> String {
    sha256_hex(RANKS_SOURCE.as_bytes())
}

/// The SHA-256 of [`FAMILIES_SOURCE`], in hex.
pub fn families_sha256() -> String {
    sha256_hex(FAMILIES_SOURCE.as_bytes())
}

/// The SHA-256 of [`MODELS_SOURCE`], in hex.
pub fn models_sha256() -> String {
    sha256_hex(MODELS_SOURCE.as_bytes())
}

/// LB1.8's row "Straight-line programs".
pub fn straight_line_programs() -> CatalogueRow {
    CatalogueRow {
        entry: "Straight-line programs",
        level: "Words: 600-cell-Full words as the IR draft's compressed words (LG2.16).",
        effects: &[],
        backing: "Library code over the IR draft's compressed words: `research_slp.program_word` builds one from its \
                  rules with `compress` and `concat`; `research_slp.program_length` and \
                  `research_slp.program_cost` read the exact length; `research_slp.streamed_length` expands chunk by \
                  chunk with `expand` and reads the literal digest with `word_stream`; \
                  `research_slp.admitted_expansion` expands only within its bound. Application stays KN2.4's \
                  `replay`.",
        notes: "Exact expanded length and cost come before any expansion. A streamed expansion holds one chunk at a \
                time, and the letters it expands number exactly the exact length. An expansion over its admission \
                bound is refused with its exact length before anything is expanded; the bound is the consuming \
                entry's, not a bound on possible words. Empty expansion is a zero-letter word.",
        module: SLP_MODULE,
        functions: &SLP_EXPORTS,
        source_sha256: slp_sha256(),
    }
}

/// LB1.8's row "Big integers and symbolic bounds".
pub fn big_integers_and_symbolic_bounds() -> CatalogueRow {
    CatalogueRow {
        entry: "Big integers and symbolic bounds",
        level: "Exact scalar values: integers of any size.",
        effects: &[],
        backing: "Library code over KN3's integers: `research_bigint.binomial`, `research_bigint.product`, \
                  `research_bigint.affine_recurrence`, `research_bigint.binomial_sum`, \
                  `research_bigint.digit_count` and `research_bigint.symbolic_bound`, with the IR draft's \
                  `exact_add`, `exact_mul`, `q5_of` and `q5_parts`.",
        notes: "Binomials, products, recurrences and decimal digit counts, exact. A symbolic bound keeps its name and \
                named premises and says `expanded: false` and `replayed: false`: a bound does not imply that its \
                word was expanded or replayed. Stage 19's B11 has 6,775 digits and is a theoretical bound.",
        module: BIGINT_MODULE,
        functions: &BIGINT_EXPORTS,
        source_sha256: bigint_sha256(),
    }
}

/// LB1.8's row "Finite fixed points".
pub fn finite_fixed_points() -> CatalogueRow {
    CatalogueRow {
        entry: "Finite fixed points",
        level: "Values in a declared finite lattice: the subsets of an AND/OR graph's nodes.",
        effects: &[],
        backing: "Library code: `research_fixpoints.least_fixed_point`, `research_fixpoints.greatest_fixed_point`, \
                  `research_fixpoints.apply_operator` and `research_fixpoints.is_fixed_point`, iterating the graph's \
                  monotone operator from the empty and the full set.",
        notes: "The operator is stated with the graph: an `and` node holds when all its children hold, an `or` node \
                when one does. Least and greatest fixed points answer `holds`, the check that the operator maps the \
                answer to itself. A converged empty set is a fact about that operator.",
        module: FIXPOINTS_MODULE,
        functions: &FIXPOINTS_EXPORTS,
        source_sha256: fixpoints_sha256(),
    }
}

/// LB1.8's row "Exact matching and integral flows".
pub fn exact_matching_and_integral_flows() -> CatalogueRow {
    CatalogueRow {
        entry: "Exact matching and integral flows",
        level: "Explicit bipartite graphs and networks.",
        effects: &[],
        backing: "Library code over KN3's integers: `research_flows.max_matching` by augmenting paths, \
                  `research_flows.check_matching`, `research_flows.feasible_flow` by shortest augmenting paths over a \
                  source and a sink for the supplies and demands, and `research_flows.check_flow`.",
        notes: "A maximum matching with its matched pairs, and an integral feasible flow with its flows, demands and \
                capacities and the residual cut. The checks test a supplied witness against every edge, capacity \
                and demand. Infeasibility concerns this graph or network. Using a flow to plan a solve is a research \
                tier candidate.",
        module: FLOWS_MODULE,
        functions: &FLOWS_EXPORTS,
        source_sha256: flows_sha256(),
    }
}

/// LB5's row "Restricted word search", over a synthetic permutation action.
pub fn restricted_word_search() -> CatalogueRow {
    CatalogueRow {
        entry: "Restricted word search",
        level: "Values: a declared finite family of words over a synthetic permutation action, with its start point \
                and goal points; no puzzle state.",
        effects: &[Effect::Budgets],
        backing: "Library code: `research_search.search` enumerates the declared family by length, then letter order, \
                  in a child scope, with its deterministic budget in examined candidates. The puzzle form over \
                  KN2.4's `classify`, `effect`, `replay` and observations is not built.",
        notes: "research tier candidate; a solving session refuses it. The record keeps the declaration (family, \
                alphabet, bounds, beam, seed, budget and cost unit, input and module identities), the outcome, the \
                stop reason and the coverage: examined candidates, the lengths covered completely, beam discards, the \
                witness count and the least witness length. It carries no witness word. Empty means only that the \
                searched set has no witness; a beam that discarded a candidate, a budget stop, a clock stop and an \
                error give no negative.",
        module: SEARCH_MODULE,
        functions: &SEARCH_WORD_SEARCH,
        source_sha256: search_sha256(),
    }
}

/// LB5's row "Search reductions and gates".
pub fn search_reductions_and_gates() -> CatalogueRow {
    CatalogueRow {
        entry: "Search reductions and gates",
        level: "Word families with their declared alphabet, hypotheses and reduction.",
        effects: &[],
        backing: "Library code: `research_search.transport_negative` over a search record and a cited lemma; the \
                  local commuting reduction is the search's own.",
        notes: "research tier candidate; a solving session refuses it. A reduced negative reaches its unreduced \
                family only through a lemma with the same family, alphabet, hypotheses, reduction and conclusion; \
                otherwise it stays reduced with `errors: LB5:evidence.reduction_scope_mismatch`. A local commuting \
                reduction is not a trace normal form. Brute-force agreement is a scoped test, not the lemma.",
        module: SEARCH_MODULE,
        functions: &SEARCH_REDUCTIONS,
        source_sha256: search_sha256(),
    }
}

/// LB5's row "Admitted-domain membership", over a synthetic permutation action.
pub fn admitted_domain_membership() -> CatalogueRow {
    CatalogueRow {
        entry: "Admitted-domain membership",
        level: "Values: a declared D[k] over a synthetic permutation action, its target points, k and the \
                reachability promise; no puzzle state.",
        effects: &[Effect::Budgets],
        backing: "Library code: `research_search.membership` enumerates every word of at most k letters from the \
                  input in a child scope, with its budget in examined candidates, after checking the action. The \
                  jumbling form over KN2.4's `classify`, `replay` and branch predicates is not built.",
        notes: "research tier candidate when it enumerates legal words; a solving session refuses it. A complete \
                rejection is the value `OUTSIDE_CERTIFIED_DOMAIN` with k and that domain's scope (LB5.4): a completed \
                answer and a value, not an error. A cut-off or failed enumeration answers `unknown`, never that \
                rejection. No answer asserts reachability or unreachability, and changing k or the promise changes \
                the scope.",
        module: SEARCH_MODULE,
        functions: &SEARCH_MEMBERSHIP,
        source_sha256: search_sha256(),
    }
}

/// The rank part of LB5's row "Strategies, selectors and ranks".
pub fn strategies_selectors_and_ranks() -> CatalogueRow {
    CatalogueRow {
        entry: "Strategies, selectors and ranks",
        level: "Values: typed progress vectors q and the class sizes N_t of stage 19's bounded transitions.",
        effects: &[],
        backing: "Library code over KN3's integers: `research_ranks.rank` (R_L by Horner's rule), \
                  `research_ranks.lex_less` and `research_ranks.block_decrease`.",
        notes: "research tier candidate; a solving session refuses it. Ranks only: R_L = sum(q_t product_(s>t)(1+L \
                N_s)), the lexicographic order of typed vectors and the decrease of R_L across stored blocks, which \
                answers `global: false`: checked decrease on stored blocks does not discharge the selector, \
                availability and invariant-domain premises. No strategy or selector is built.",
        module: RANKS_MODULE,
        functions: &RANKS_RANKS,
        source_sha256: ranks_sha256(),
    }
}

/// LB5's row "Bound calculation and compressed witnesses", as far as this module builds it.
pub fn bound_calculation_and_compressed_witnesses() -> CatalogueRow {
    CatalogueRow {
        entry: "Bound calculation and compressed witnesses",
        level: "Exact scalar values: stage 19's fixed-length and conditional cost bounds.",
        effects: &[],
        backing: "Library code: `research_ranks.fixed_length_bound` and `research_ranks.conditional_cost` over \
                  `research_ranks.rank`; big integers and compressed witnesses through the `research_bigint` and \
                  `research_slp` modules.",
        notes: "Fixed-length cost L R_L and peak 2 R_L + 2 + 2L, and the conditional cost max(H-2,0)/2 \
                (product(1+L N_t) - 1), each with its named premises (availability, invariant domain, selector, \
                strict decrease) and `discharged: false`, `expanded: false`, `measured: false` and \
                `restores_stickers: false`. No expansion claim follows, and a frontier exit, low height and a \
                restored sticker state stay different goals. No witness is picked here.",
        module: RANKS_MODULE,
        functions: &RANKS_BOUNDS,
        source_sha256: ranks_sha256(),
    }
}

/// LB5's row "Regional-family recognition", over supplied pose tables.
pub fn regional_family_recognition() -> CatalogueRow {
    CatalogueRow {
        entry: "Regional-family recognition",
        level: "Values: a full reference table and an instance of typed poses with a gauge relation, supplied as \
                data.",
        effects: &[],
        backing: "Library code: `research_families.recognize` over LB1's exact bipartite matching, \
                  `research_flows.max_matching`, imported by the SHA-256 of its source.",
        notes: "A type-preserving bijection of the whole instance onto the reference, poses equal or paired by the \
                gauge. A recognized instance transfers the legal-word language, legality along prefixes and \
                shape-level claims, never a sticker goal, a distance or reachability. The reference pieces are \
                counted; distinct fibres are not asserted. The check mode is exact transport, not a fresh audit or a \
                replay.",
        module: FAMILIES_MODULE,
        functions: &FAMILIES_EXPORTS,
        source_sha256: families_sha256(),
    }
}

/// LB5's row "Conditional models and partial actions", over a synthetic permutation action.
pub fn conditional_models_and_partial_actions() -> CatalogueRow {
    CatalogueRow {
        entry: "Conditional models and partial actions",
        level: "Values: a synthetic permutation action, a region or a block partition, and bounds with named \
                premises.",
        effects: &[],
        backing: "Library code: `research_models.restriction`, `research_models.quotient`, \
                  `research_models.recognize_instance`, `research_models.conditional_bound`, \
                  `research_models.combine` and `research_models.discharge`.",
        notes: "A restriction and a quotient state their map and preserved subjects; a restriction is a partial \
                action, -1 where a point leaves the region, and an incompatible quotient answers its conflicts. \
                Recognition of a supplied instance does not assert its reachability. The premises of conditional \
                bounds accumulate, and one is discharged only by a statement of the identical premise and scope.",
        module: MODELS_MODULE,
        functions: &MODELS_EXPORTS,
        source_sha256: models_sha256(),
    }
}

/// Every row of the research library, LB1's first, in chapter 03's order.
pub fn rows() -> Vec<CatalogueRow> {
    vec![
        finite_fixed_points(),
        straight_line_programs(),
        exact_matching_and_integral_flows(),
        big_integers_and_symbolic_bounds(),
        restricted_word_search(),
        search_reductions_and_gates(),
        strategies_selectors_and_ranks(),
        conditional_models_and_partial_actions(),
        regional_family_recognition(),
        admitted_domain_membership(),
        bound_calculation_and_compressed_witnesses(),
    ]
}

/// The rows that are research tier candidates: those whose notes begin with [`MARKER`].
pub fn research_tier_rows() -> Vec<CatalogueRow> {
    rows().into_iter().filter(|row| row.notes.starts_with(MARKER)).collect()
}

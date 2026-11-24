//! The research library's catalogue rows (LB1.2, LB1.5): each row has LB1.2's five fields, names one of chapter 03's
//! LB1 or LB5 rows, and is backed by exported functions of a module whose source is canonical, LF-only and pinned by
//! its SHA-256; each row's effects are the checker's, and no entry reads state. Exactly the rows chapter 03 marks are
//! research tier candidates, and every entry that enumerates words is in one. These are supporting tests of piece
//! R7, not rows of a conformance table. A pass is source evidence.
//!
//! Not tested here: the refusal of a research tier candidate in a solving session. The checker's solving profile
//! refuses own-tool previews and the built-in column's unlisted rows; these entries prepare no preview and use no
//! row, so the refusal is the host's shipped-manifest scan over the marker (BND.64, CT-BND-16), which is not built.

mod research_common;

use std::collections::{BTreeMap, BTreeSet};

use c600_check::check;
use c600_digest::sha256_hex;
use c600_ir::Effect;
use c600_library::lower_sources;
use c600_library::research::*;
use c600_syntax::{format, parse};
use research_common::repository;

/// Every module: (name, source, exports in source order).
fn modules() -> Vec<(&'static str, &'static str, Vec<&'static str>)> {
    vec![
        (SLP_MODULE, SLP_SOURCE, SLP_EXPORTS.to_vec()),
        (BIGINT_MODULE, BIGINT_SOURCE, BIGINT_EXPORTS.to_vec()),
        (FIXPOINTS_MODULE, FIXPOINTS_SOURCE, FIXPOINTS_EXPORTS.to_vec()),
        (FLOWS_MODULE, FLOWS_SOURCE, FLOWS_EXPORTS.to_vec()),
        (SEARCH_MODULE, SEARCH_SOURCE, SEARCH_EXPORTS.to_vec()),
        (RANKS_MODULE, RANKS_SOURCE, RANKS_EXPORTS.to_vec()),
        (FAMILIES_MODULE, FAMILIES_SOURCE, FAMILIES_EXPORTS.to_vec()),
        (MODELS_MODULE, MODELS_SOURCE, MODELS_EXPORTS.to_vec()),
        (PREFLIGHT_MODULE, PREFLIGHT_SOURCE, PREFLIGHT_EXPORTS.to_vec()),
    ]
}

/// Each source is edition `6cs-draft-1` in the formatter's canonical layout, LF-only, names its module on its second
/// line and exports exactly its listed functions in source order. `research_families` imports `research_flows` by the
/// SHA-256 of its source and is the only module that imports.
#[test]
fn research_sources_are_canonical_and_export_their_listed_functions() {
    for (module, source, exports) in modules() {
        assert!(source.starts_with("edition \"6cs-draft-1\";\n"), "{module}");
        assert!(!source.contains('\r'), "{module} is LF-only");
        assert_eq!(source.lines().nth(1), Some(format!("module {module} requires [];").as_str()));
        let file = parse(source.as_bytes());
        assert!(file.diagnostics.is_empty() && file.holes.is_empty(), "{module}: {:#?}", file.diagnostics);
        assert_eq!(format(&file).as_deref(), Some(source), "{module} is canonical");
        let listed: Vec<&str> = source
            .lines()
            .filter_map(|line| line.strip_prefix("export fn "))
            .map(|rest| &rest[..rest.find('(').expect("a parameter list")])
            .collect();
        assert_eq!(listed, exports, "{module}");
        let imports: Vec<&str> = source.lines().filter(|line| line.starts_with("import ")).collect();
        if module == FAMILIES_MODULE {
            assert_eq!(imports, [format!("import {FLOWS_MODULE} \"{}\";", flows_sha256()).as_str()]);
        } else {
            assert!(imports.is_empty(), "{module}");
        }
    }
}

/// Each row has LB1.2's five fields filled, names a row of chapter 03's LB1 or LB5 tables, and is backed by exported
/// functions of its module with that module's digest. Together the rows cover every export once.
#[test]
fn research_rows_have_lb1_2_fields_and_cover_every_export_once() {
    let chapter =
        std::fs::read_to_string(repository().join("docs/architecture/1.0/language/architecture/03-libraries.md"))
            .expect("chapter 03");
    let digests: BTreeMap<&str, (String, Vec<&str>)> = modules()
        .into_iter()
        .map(|(module, source, exports)| (module, (sha256_hex(source.as_bytes()), exports)))
        .collect();
    let rows = rows();
    assert_eq!(rows.len(), 12);
    let mut covered: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    let mut names = BTreeSet::new();
    for row in &rows {
        assert!(names.insert(row.entry), "one row per entry: {}", row.entry);
        assert!(chapter.contains(&format!("| {} |", row.entry)), "chapter 03 has a row {:?}", row.entry);
        for (field, value) in [("level", row.level), ("backing", row.backing), ("notes", row.notes)] {
            assert!(!value.trim().is_empty(), "{}: {field}", row.entry);
            assert!(!value.contains("  "), "{}: {field} has no doubled space", row.entry);
        }
        let (digest, exports) = &digests[row.module];
        assert_eq!(&row.source_sha256, digest, "{}", row.entry);
        assert!(!row.functions.is_empty());
        for function in row.functions {
            assert!(exports.contains(function), "{}: {function} is exported", row.entry);
            assert!(
                row.backing.contains(&format!("`{}.{function}`", row.module)),
                "{}: backing names {function}",
                row.entry
            );
            covered.entry(row.module).or_default().push(function);
        }
    }
    for (module, (_, exports)) in &digests {
        let mut seen = covered.get(module).cloned().unwrap_or_default();
        seen.sort();
        let mut all = exports.clone();
        all.sort();
        assert_eq!(seen, all, "{module}: every export in exactly one row");
    }
}

/// Each row's effects are the union of the checker's inferred effects of its functions, so `pure` rows are pure and
/// the enumerating rows declare budgets only; no function of any module reads state, uses a capability, a row or a
/// host call, and none prepares a preview or changes the workspace.
#[test]
fn research_row_effects_are_the_checkers_and_no_entry_reads_state() {
    let sources: Vec<&str> = modules().iter().map(|(_, source, _)| *source).collect();
    let program = lower_sources(&sources).expect("the research sources lower together");
    let report = check(&program).expect("well-formed IR");
    assert!(report.diagnostics.is_empty(), "{:#?}", report.diagnostics);
    for row in rows() {
        let mut inferred = BTreeSet::new();
        for function in row.functions {
            let summary = report.summary(row.module, function).expect("a summary");
            assert!(summary.capabilities.is_empty(), "{}.{function}", row.module);
            assert!(summary.state_reads.is_empty(), "{}.{function}", row.module);
            assert!(summary.rows.is_empty(), "{}.{function}", row.module);
            assert!(summary.unresolved.is_empty() && summary.vars.is_empty(), "{}.{function}", row.module);
            inferred.extend(summary.effects.iter().copied());
        }
        let declared: BTreeSet<Effect> = row.effects.iter().copied().collect();
        assert_eq!(declared, inferred, "{}", row.entry);
        assert!(declared.iter().all(|effect| *effect == Effect::Budgets), "{}", row.entry);
    }
}

/// Chapter 03's Effect column for `entry`, as IR effects: `pure` adds none, a qualified phrase ("read a snapshot
/// for instance tests", "seeded randomness if used") counts as its effect, and "use a granted file on import" (the row
/// "Certificate builders, import and replay") adds none, since a file grant is the host's and no IR effect.
fn chapter_effects(chapter: &str, entry: &str) -> BTreeSet<Effect> {
    let line = chapter
        .lines()
        .find(|line| line.starts_with(&format!("| {entry} |")))
        .unwrap_or_else(|| panic!("chapter 03 has a row {entry:?}"));
    let column = line.split('|').nth(3).expect("an Effect column");
    let mut effects = BTreeSet::new();
    for phrase in column.split([',', ';']).map(str::trim) {
        if phrase == "pure" || phrase == "use a granted file on import" {
            continue;
        } else if phrase == "budgets" {
            effects.insert(Effect::Budgets);
        } else if phrase.starts_with("read a snapshot") {
            effects.insert(Effect::ReadSnapshot);
        } else if phrase.starts_with("seeded randomness") {
            effects.insert(Effect::SeededRandomness);
        } else {
            panic!("{entry}: unknown effect phrase {phrase:?}");
        }
    }
    effects
}

/// Each row's effects, the checker's, are a subset of chapter 03's Effect column for the row: the column is the most
/// an entry of the row may use. LB1's four rows list `pure, budgets` and their entries use no budget, so they are
/// pure; the enumerating rows use budgets; no row reads a snapshot, which only the unbuilt puzzle forms would.
#[test]
fn research_row_effects_are_within_chapter_03s_effect_column() {
    let chapter =
        std::fs::read_to_string(repository().join("docs/architecture/1.0/language/architecture/03-libraries.md"))
            .expect("chapter 03");
    for row in rows() {
        let allowed = chapter_effects(&chapter, row.entry);
        let declared: BTreeSet<Effect> = row.effects.iter().copied().collect();
        assert!(declared.is_subset(&allowed), "{}: {declared:?} within {allowed:?}", row.entry);
    }
    for entry in [
        "Straight-line programs",
        "Big integers and symbolic bounds",
        "Finite fixed points",
        "Exact matching and integral flows",
    ] {
        assert_eq!(chapter_effects(&chapter, entry), BTreeSet::from([Effect::Budgets]), "{entry}: pure, budgets");
    }
}

/// Exactly chapter 03's marked rows are research tier candidates: restricted word search, search reductions and
/// gates, strategies, selectors and ranks, and admitted-domain membership. Every exported function that enumerates
/// words, the ones with a budget, is in one of them. What is scanned besides: the search's and the membership's result
/// types hold no word type, so no record carries a witness word; and no row text or source uses a move-selection
/// phrase from the list below.
#[test]
fn research_tier_candidates_are_exactly_the_marked_rows() {
    for function in ["search", "membership"] {
        let signature = SEARCH_SOURCE
            .lines()
            .find(|line| line.starts_with(&format!("export fn {function}(")))
            .expect("an exported function");
        let result = &signature[signature.find(") -> ").expect("a result type")..signature.rfind(" effects ").unwrap()];
        assert!(!result.contains("Word"), "{function}'s result holds no word");
    }
    let marked: Vec<&str> = research_tier_rows().iter().map(|row| row.entry).collect();
    assert_eq!(
        marked,
        [
            "Restricted word search",
            "Search reductions and gates",
            "Strategies, selectors and ranks",
            "Admitted-domain membership"
        ]
    );
    for row in rows() {
        if row.effects.contains(&Effect::Budgets) {
            assert!(row.notes.starts_with(MARKER), "{} enumerates words", row.entry);
        }
        if !row.notes.starts_with(MARKER) {
            assert!(!row.notes.contains(MARKER) || row.entry == "Exact matching and integral flows", "{}", row.entry);
        }
        for text in [row.backing, row.notes] {
            let lower = text.to_lowercase();
            for phrase in ["best move", "next move", "suggested move", "recommended", "solution word"] {
                assert!(!lower.contains(phrase), "{}: {phrase}", row.entry);
            }
        }
    }
    for (_, source, _) in modules() {
        assert!(!source.contains("best") && !source.contains("suggest"), "no source selects a move");
    }
}

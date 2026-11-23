//! LP5 piece R1: the research library's availability preflight, `research_preflight.check_availability` (LB5.14), on
//! `c600-run` with no host.
//!
//! Test names carry the CT ID and part (PHS.62):
//!
//! - CT-LB5-06 (source, part: LB5.14's preflight in the language: every claim group of `c600-archive`'s binding
//!   edition, read in place as the host would supply it, answers `ready` or `unavailable` with the matrix's blockers,
//!   and q11 answers LB5.17's example with its captured string unchanged; not run: the stage 19 manifest digests,
//!   which `tests/test_lp5_archive.py` checks, and the stage 19 claims, which `c600-checkers/tests/kn6_s19.rs` checks).
//! - CT-ERR-11 (source, part: an unavailable preflight answer is a completed value, never a pass, a failure or a
//!   negative; not run: the membership, search and other research answers, which `research_ct_err_11.rs` runs).
//!
//! The module's own source and catalogue row are checked as R7's catalogue test checks the other research modules. A
//! pass is source evidence.

mod research_common;

use std::collections::{BTreeMap, BTreeSet};

use c600_check::check;
use c600_digest::sha256_hex;
use c600_ir::Effect;
use c600_kernel::json::{self, Value as Json};
use c600_library::lower_sources;
use c600_library::research::{
    availability_preflight, preflight_sha256, MARKER, PREFLIGHT_EXPORTS, PREFLIGHT_MODULE, PREFLIGHT_SOURCE,
};
use c600_run::value::Value;
use c600_syntax::{format, parse};
use research_common::*;

/// The binding edition `c600-archive` embeds, read in place as host data.
const EDITION: &str = "rust/c600-archive/data/claim-bindings-1.json";

/// q11's group (LB5.17).
const Q11: &str = "19/q11:research/jumbling/restoration/19-height-depth-framework/records/\
                   type-lex-trimmed-independent-results.json#/q11_depth_certificate";

fn edition() -> Json {
    let bytes = std::fs::read(repository().join(EDITION)).expect("the binding edition");
    json::parse(&bytes).expect("strict JSON")
}

fn jtext(v: &Json, key: &str) -> String {
    v.get(key).and_then(Json::as_str).unwrap_or_else(|| panic!("{key}")).to_string()
}

fn jtexts(v: &Json) -> Vec<String> {
    v.as_array().expect("a list").iter().map(|x| x.as_str().expect("text").to_string()).collect()
}

fn jarray<'v>(v: &'v Json, key: &str) -> &'v [Json] {
    v.get(key).and_then(Json::as_array).unwrap_or_else(|| panic!("{key}"))
}

fn texts_of(values: &[String]) -> Value {
    list(values.iter().map(|s| text(s)).collect())
}

/// How the host answers: which named checkers are registered, and the state of each stored input by path.
struct HostAnswers<'a> {
    registered: &'a dyn Fn(&str) -> bool,
    state: &'a dyn Fn(&str) -> &'static str,
}

/// One edition row as the host supplies it to the library: the row's own data, the checkers that check its claim
/// (none for a scope-only reading) with the host's registration answer, and its stored files (the record, its inputs
/// and its `via` record, once each, as `c600-archive` lists them) with the host's state for each.
fn plan(edition: &Json, row: &Json, host: &HostAnswers) -> Value {
    let files = jarray(edition, "files");
    let file = |v: &Json| &files[v.as_i64().expect("a file index") as usize];
    let checks = row.get("checked_as").and_then(Json::as_str).is_some_and(|how| how != "scope_only");
    let checkers: Vec<Value> = if checks {
        jtexts(row.get("checkers").unwrap())
            .iter()
            .map(|id| {
                let registered = (host.registered)(id);
                let digest = if registered { sha256_hex(id.as_bytes()) } else { String::new() };
                record(vec![("id", text(id)), ("registered", boolean(registered)), ("source_digest", text(&digest))])
            })
            .collect()
    } else {
        Vec::new()
    };
    let mut stored: Vec<(i64, String)> = vec![(row.get("source").unwrap().as_i64().unwrap(), "claim_record".into())];
    for pair in jarray(row, "inputs") {
        let pair = pair.as_array().unwrap();
        stored.push((pair[0].as_i64().unwrap(), pair[1].as_str().unwrap().to_string()));
    }
    if let Some(via) = row.get("via").and_then(Json::as_i64) {
        stored.push((via, "via".into()));
    }
    let mut seen = BTreeSet::new();
    stored.retain(|(i, _)| seen.insert(*i));
    let inputs: Vec<Value> = stored
        .iter()
        .map(|(i, role)| {
            let f = &files[*i as usize];
            let path = jtext(f, "path");
            let state = (host.state)(&path);
            record(vec![
                ("path", text(&path)),
                ("role", text(role)),
                ("sha256", text(&jtext(f, "sha256"))),
                ("state", text(state)),
            ])
        })
        .collect();
    let derived: Vec<Value> = jarray(row, "derived")
        .iter()
        .map(|d| {
            record(vec![
                ("digest", text(&jtext(d, "digest"))),
                ("pointer", text(&jtext(d, "pointer"))),
                ("role", text(&jtext(d, "role"))),
            ])
        })
        .collect();
    let missing: Vec<Value> = jarray(row, "missing")
        .iter()
        .map(|m| record(vec![("cites", texts_of(&jtexts(m.get("cites").unwrap()))), ("name", text(&jtext(m, "name")))]))
        .collect();
    let resolved: Vec<String> = row.get("resolved").and_then(Json::as_object).unwrap().keys().cloned().collect();
    let label = match row.get("label") {
        Some(Json::Object(_)) => jtext(row.get("label").unwrap(), "text"),
        _ => String::new(),
    };
    record(vec![
        ("bound", boolean(true)),
        ("checkers", list(checkers)),
        ("derived", list(derived)),
        ("historical", texts_of(&jtexts(row.get("historically_incomplete").unwrap()))),
        ("inputs", list(inputs)),
        ("label", text(&label)),
        ("matrix_reasons", texts_of(&jtexts(row.get("matrix_reasons").unwrap()))),
        ("missing", list(missing)),
        ("resolved", texts_of(&resolved)),
        ("source", text(&jtext(file(row.get("source").unwrap()), "path"))),
    ])
}

/// A plan for a claim with no binding row.
fn unbound() -> Value {
    record(vec![
        ("bound", boolean(false)),
        ("checkers", list(vec![])),
        ("derived", list(vec![])),
        ("historical", list(vec![])),
        ("inputs", list(vec![])),
        ("label", text("")),
        ("matrix_reasons", list(vec![])),
        ("missing", list(vec![])),
        ("resolved", list(vec![])),
        ("source", text("research/elsewhere.json")),
    ])
}

fn preflight(loaded: &c600_run::interp::Loaded, plan: Value) -> Value {
    call(loaded, PREFLIGHT_MODULE, "check_availability", vec![plan])
}

/// The answer's constant fields: no check ran, no evidence leaf, nothing refuted.
fn assert_no_check(answer: &Value) {
    assert!(!as_bool(field(answer, "checked")));
    assert!(!as_bool(field(answer, "evidence_leaf")));
    assert!(!as_bool(field(answer, "refutes")));
}

/// The module is edition `6cs-draft-1` in the formatter's canonical layout, LF-only, imports nothing and exports
/// `check_availability`; its row has LB1.2's five fields, names chapter 03's row "Certificate builders, import and
/// replay", is backed by its export with the source's digest, is pure as the checker infers (within the row's Effect
/// column, whose "use a granted file on import" is no IR effect), reads no state and is no research tier candidate.
#[test]
fn research_preflight_source_and_row() {
    let source = PREFLIGHT_SOURCE;
    assert!(source.starts_with("edition \"6cs-draft-1\";\n"));
    assert!(!source.contains('\r'));
    assert_eq!(source.lines().nth(1), Some(format!("module {PREFLIGHT_MODULE} requires [];").as_str()));
    let file = parse(source.as_bytes());
    assert!(file.diagnostics.is_empty() && file.holes.is_empty(), "{:#?}", file.diagnostics);
    assert_eq!(format(&file).as_deref(), Some(source), "canonical layout");
    let exports: Vec<&str> = source
        .lines()
        .filter_map(|line| line.strip_prefix("export fn "))
        .map(|rest| &rest[..rest.find('(').expect("a parameter list")])
        .collect();
    assert_eq!(exports, PREFLIGHT_EXPORTS);
    assert!(!source.lines().any(|line| line.starts_with("import ")));
    assert!(!source.contains("best") && !source.contains("suggest"));

    let row = availability_preflight();
    let chapter =
        std::fs::read_to_string(repository().join("docs/architecture/1.0/language/architecture/03-libraries.md"))
            .expect("chapter 03");
    let line = chapter.lines().find(|l| l.starts_with(&format!("| {} |", row.entry))).expect("chapter 03's row");
    let column = line.split('|').nth(3).expect("an Effect column");
    let phrases: Vec<&str> = column.split([',', ';']).map(str::trim).collect();
    assert_eq!(phrases, ["pure", "budgets", "use a granted file on import"]);
    for (name, value) in [("level", row.level), ("backing", row.backing), ("notes", row.notes)] {
        assert!(!value.trim().is_empty() && !value.contains("  "), "{name}");
    }
    assert_eq!(row.source_sha256, preflight_sha256());
    assert_eq!(row.source_sha256, sha256_hex(source.as_bytes()));
    assert_eq!(row.module, PREFLIGHT_MODULE);
    assert_eq!(row.functions, PREFLIGHT_EXPORTS);
    assert!(row.backing.contains("`research_preflight.check_availability`"));
    assert!(!row.notes.starts_with(MARKER) && !row.notes.contains(MARKER));
    for text in [row.backing, row.notes] {
        let lower = text.to_lowercase();
        for phrase in ["best move", "next move", "suggested move", "recommended", "solution word"] {
            assert!(!lower.contains(phrase), "{phrase}");
        }
    }
    let program = lower_sources(&[source]).expect("the source lowers");
    let report = check(&program).expect("well-formed IR");
    assert!(report.diagnostics.is_empty(), "{:#?}", report.diagnostics);
    let summary = report.summary(PREFLIGHT_MODULE, "check_availability").expect("a summary");
    assert!(summary.capabilities.is_empty() && summary.state_reads.is_empty() && summary.rows.is_empty());
    assert!(summary.unresolved.is_empty() && summary.vars.is_empty());
    let inferred: BTreeSet<Effect> = summary.effects.iter().copied().collect();
    assert_eq!(inferred, row.effects.iter().copied().collect::<BTreeSet<Effect>>());
    assert!(inferred.is_empty(), "pure");
}

/// LB5.14's reasons, in its order.
const REASONS: [&str; 6] = [
    "claim_unmapped",
    "dependency_missing",
    "dependency_mismatch",
    "no_independent_checker",
    "unsupported_claim",
    "historical_check_incomplete",
];

/// The reasons LB5.14's rule gives a row when every named claim checker is registered and every input held: the
/// matrix's, less what the binding resolves, less `no_independent_checker` when a checker checks the claim, in LB5.14's
/// order (the matrix lists them alphabetically).
fn expected_reasons(row: &Json) -> Vec<String> {
    let resolved = row.get("resolved").and_then(Json::as_object).unwrap();
    let checks = row.get("checked_as").and_then(Json::as_str).is_some_and(|how| how != "scope_only");
    let kept: Vec<String> = jtexts(row.get("matrix_reasons").unwrap())
        .into_iter()
        .filter(|r| !resolved.contains_key(r) && !(checks && r == "no_independent_checker"))
        .collect();
    REASONS.iter().filter(|r| kept.iter().any(|k| k == *r)).map(|r| r.to_string()).collect()
}

/// One row's answers: with every named claim checker registered and every input held, the expected reasons (and
/// `ready` exactly when there are none, naming its checkers); with no checker registered, never `ready`, and
/// `no_independent_checker` for every mathematical group. Returns the first answer's reasons, or none for `ready`.
fn answers_for(loaded: &c600_run::interp::Loaded, edition: &Json, row: &Json) -> Option<Vec<String>> {
    let group = jtext(row, "group");
    let matrix: Vec<String> = jtexts(row.get("matrix_reasons").unwrap());
    let all = HostAnswers { registered: &|_| true, state: &|_| "present" };
    let none = HostAnswers { registered: &|_| false, state: &|_| "present" };
    let answer = preflight(loaded, plan(edition, row, &all));
    assert_no_check(&answer);
    let reasons = strings(field(&answer, "reasons"));
    assert_eq!(reasons, expected_reasons(row), "{group}");
    assert!(reasons.iter().all(|r| matrix.contains(r)), "{group}: {reasons:?} within {matrix:?}");
    let unregistered = preflight(loaded, plan(edition, row, &none));
    assert_eq!(as_text(field(&unregistered, "kind")), "unavailable", "{group}");
    let record_only = jtext(row, "data_status") == "record_only";
    let no_checker = strings(field(&unregistered, "reasons")).contains(&"no_independent_checker".to_string());
    assert_eq!(no_checker, !record_only, "{group}");
    match as_text(field(&answer, "kind")).as_str() {
        "ready" => {
            assert!(reasons.is_empty() && items(field(&answer, "dependencies")).is_empty(), "{group}");
            assert!(!items(field(&answer, "checkers")).is_empty(), "{group}");
            None
        }
        "unavailable" => {
            assert!(!reasons.is_empty(), "{group}");
            Some(reasons)
        }
        other => panic!("{group}: {other}"),
    }
}

/// CT-LB5-06's preflight part, one group of each of the matrix's 37 claim families, supplied as host data from the
/// binding edition: each answers `ready` or `unavailable` with the matrix's blockers less what the binding resolves.
#[test]
fn ct_lb5_06_preflight_one_group_per_family_from_host_data() {
    let loaded = loaded(&[PREFLIGHT_SOURCE]);
    let edition = edition();
    let mut families = BTreeSet::new();
    for row in jarray(&edition, "rows") {
        if families.insert(jtext(row, "family")) {
            answers_for(&loaded, &edition, row);
        }
    }
    assert_eq!(families.len(), 37);
}

/// CT-LB5-06's preflight part: every row of the binding edition, supplied as host data, answers `ready` or
/// `unavailable` with the matrix's blockers less what the binding resolves: 286 ready, and the unavailable reasons
/// counted as `c600-archive`'s preflight counts them. With no checker registered, none is ready.
#[test]
#[ignore = "about two minutes in a debug build (1,020 interpreter runs); ct_lb5_06_preflight_one_group_per_family_from_host_data is the fast representative"]
fn ct_lb5_06_preflight_every_group_from_host_data() {
    let loaded = loaded(&[PREFLIGHT_SOURCE]);
    let edition = edition();
    let rows = jarray(&edition, "rows");
    assert_eq!(rows.len(), 510);
    let mut ready = 0;
    let mut per_reason: BTreeMap<String, usize> = BTreeMap::new();
    for row in rows {
        match answers_for(&loaded, &edition, row) {
            None => ready += 1,
            Some(reasons) => {
                for r in reasons {
                    *per_reason.entry(r).or_default() += 1;
                }
            }
        }
    }
    assert_eq!(ready, 286);
    let expected: BTreeMap<String, usize> = [
        ("dependency_missing", 210),
        ("historical_check_incomplete", 2),
        ("no_independent_checker", 216),
        ("unsupported_claim", 6),
    ]
    .into_iter()
    .map(|(r, n)| (r.to_string(), n))
    .collect();
    assert_eq!(per_reason, expected);
}

/// CT-LB5-06's preflight part, LB5.17's example: q11 answers `unavailable([dependency_missing,
/// no_independent_checker], ...)` with its source reference to the absent successor cache, and its captured string
/// unchanged, whatever the host registers.
#[test]
fn ct_lb5_06_preflight_q11_from_host_data() {
    let loaded = loaded(&[PREFLIGHT_SOURCE]);
    let edition = edition();
    let row = jarray(&edition, "rows").iter().find(|r| jtext(r, "group") == Q11).expect("q11's row");
    for registered in [true, false] {
        let host = HostAnswers { registered: &move |_| registered, state: &|_| "present" };
        let answer = preflight(&loaded, plan(&edition, row, &host));
        assert_no_check(&answer);
        assert_eq!(as_text(field(&answer, "kind")), "unavailable");
        assert_eq!(strings(field(&answer, "reasons")), ["dependency_missing", "no_independent_checker"]);
        assert_eq!(as_text(field(&answer, "label")), "PASS_SHORTENED_THREE_BLOCK_WITNESS_AND_Q11_DEPTH3");
        let dependencies = items(field(&answer, "dependencies"));
        assert_eq!(dependencies.len(), 1);
        let d = &dependencies[0];
        assert_eq!(as_text(field(d, "kind")), "missing_input");
        assert!(as_text(field(d, "path"))
            .ends_with("19-height-depth-framework/records/type-lex-trimmed-independent-results.json"));
        assert!(as_text(field(d, "name")).starts_with("type-lex-successor-base-cache.npz"));
        assert_eq!(strings(field(d, "cites")), ["/inputs_sha256/type-lex-successor-base-cache.npz"]);
        assert_eq!(as_text(field(d, "sha256")), "", "a missing input is never given a digest");
    }
}

/// CT-ERR-11's unavailable part: an unavailable answer is a completed run with no code and a value that is neither a
/// pass nor a failure nor a negative: a claim with no binding (`claim_unmapped`, no dependency), a missing and a
/// mismatched input with an unregistered checker (each reason once, in LB5.14's order, the unregistered checker never
/// named), and stage 18's unfinished gate 3. The ready answer for the same group is still no pass.
#[test]
fn ct_err_11_preflight_unavailable_is_a_value() {
    let loaded = loaded(&[PREFLIGHT_SOURCE]);
    let edition = edition();
    let answer = preflight(&loaded, unbound());
    assert_no_check(&answer);
    assert_eq!(as_text(field(&answer, "kind")), "unavailable");
    assert_eq!(strings(field(&answer, "reasons")), ["claim_unmapped"]);
    assert!(items(field(&answer, "dependencies")).is_empty());

    let rows = jarray(&edition, "rows");
    let t3p = rows
        .iter()
        .find(|r| jtext(r, "group").starts_with("14/t3p:") && jtext(r, "group").contains("s7304-w3-certificate.json"))
        .expect("a stage 14 row");
    let table = "research/jumbling/restoration/13-tutorial-runner/runs/dev/d2-s7304/component/tables/00005.json";
    let broken = HostAnswers {
        registered: &|_| false,
        state: &|path| match path {
            p if p == table => "missing",
            "assets/manifest.json" => "mismatch",
            _ => "present",
        },
    };
    let answer = preflight(&loaded, plan(&edition, t3p, &broken));
    assert_no_check(&answer);
    assert_eq!(as_text(field(&answer, "kind")), "unavailable");
    assert_eq!(
        strings(field(&answer, "reasons")),
        ["dependency_missing", "dependency_mismatch", "no_independent_checker"]
    );
    assert!(items(field(&answer, "checkers")).is_empty() && items(field(&answer, "inputs")).is_empty());
    let states: Vec<(String, String)> = items(field(&answer, "dependencies"))
        .iter()
        .map(|d| (as_text(field(d, "path")), as_text(field(d, "state"))))
        .collect();
    assert_eq!(states, [("assets/manifest.json".into(), "mismatch".into()), (table.into(), "missing".into())]);
    let fine = HostAnswers { registered: &|_| true, state: &|_| "present" };
    let ready = preflight(&loaded, plan(&edition, t3p, &fine));
    assert_eq!(as_text(field(&ready, "kind")), "ready");
    assert_no_check(&ready);

    let replay = rows
        .iter()
        .find(|r| jtext(r, "group").starts_with("18/tail_replay:") && jtext(r, "group").contains("/d1-tail/"))
        .expect("T6's tail replay row");
    let answer = preflight(&loaded, plan(&edition, replay, &fine));
    assert_no_check(&answer);
    assert_eq!(strings(field(&answer, "reasons")), ["historical_check_incomplete"]);
    let gates: Vec<String> =
        items(field(&answer, "dependencies")).iter().map(|d| as_text(field(d, "pointer"))).collect();
    assert_eq!(gates, ["/steps/3"]);
}

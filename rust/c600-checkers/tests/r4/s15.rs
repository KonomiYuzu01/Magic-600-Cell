//! Stage 15 (lowering machine), read in place.
//!
//! - CT-LB5-02 (source, part: R01 to R17 with their check records under the chain binding, the whole descents at
//!   the six chain ends and the P* and DIS-1 certificates reproducing their supports, changes and digests; tampered
//!   claims failing in `s15.macro`'s gate; P*'s beamed reference stop and the unbeamed pruned stop at 7304 answering
//!   with different scopes; `pstar2/2` records and the run wrappers recognized; not run: nothing the definition
//!   names; the 7307 census, which only the ignored test `ct_lb5_02_stage15_census_7307` checks, is a relation, not a
//!   certificate of this definition).
//! - CT-KN6-01 (source, part: `s15.macro`'s gate on a certificate and a check record, with `gate::run`'s structural
//!   changes; `s15.pstar` failing under structural tampering of the 7304 census and under a wrong start table; not
//!   run: `s15.pstar`'s gate over its claims and a wrong input digest, which only the ignored test
//!   `ct_kn6_03_stage15_pstar_gate` runs).
//! - CT-KN6-03 (source, part: stage 15's sub-checkers registered and its self-contained records passing in place;
//!   not run: the 7307 census, an ignored test).

use c600_checkers::archive::{self, s15};
use c600_checkers::{codes, CheckerKind, Verdict};
use c600_kernel::json::Value;

use super::support::*;
use crate::common::*;

const MACRO: &str = "s15.macro";
const PSTAR: &str = "s15.pstar";

#[test]
fn ct_kn6_03_stage15_registry_is_clean() {
    assert!(archive::registry_problems(archive::registry()).is_empty());
    let ids: Vec<&str> = s15::SUBCHECKERS.iter().map(|s| s.id()).collect();
    assert_eq!(ids, [MACRO, PSTAR]);
    for sub in s15::SUBCHECKERS {
        assert!(std::ptr::eq(archive::find(sub.id()).unwrap(), *sub));
    }
}

fn rows(range: std::ops::RangeInclusive<usize>) {
    for n in range {
        let row = format!("R{n:02}");
        certificate_and_record(
            MACRO,
            &format!("{S15}/certificates/{row}.json"),
            &format!("{S15}/certificates/{row}-check.json"),
        );
    }
}

/// R01 to R17 with their check records, under the stage's chain binding (the certificates alone are R2's harness).
#[test]
fn ct_lb5_02_stage15_rows_and_check_records_r01_r06() {
    rows(1..=6);
}

#[test]
fn ct_lb5_02_stage15_rows_and_check_records_r07_r12() {
    rows(7..=12);
}

#[test]
fn ct_lb5_02_stage15_rows_and_check_records_r13_r17() {
    rows(13..=17);
}

fn descent(seed: &str) {
    certificate_and_record(
        MACRO,
        &format!("{S15}/runs/s{seed}-descent.json"),
        &format!("{S15}/runs/s{seed}-descent-check.json"),
    );
    // The whole descent ends at the jam, recomputed: the catalogue's end digest and Φ.
    let cert = read(&format!("{S15}/runs/s{seed}-descent.json"));
    let report = check(MACRO, &cert, &inputs(seed));
    assert!(report.trace.iter().any(|s| s.claim == "jam.end_digest" && s.held));
    assert!(report.trace.iter().any(|s| s.claim == "jam.end_phi" && s.held));
}

#[test]
fn ct_lb5_02_stage15_whole_descents_7302_7303() {
    descent("7302");
    descent("7303");
}

#[test]
fn ct_lb5_02_stage15_whole_descents_7304_7305() {
    descent("7304");
    descent("7305");
}

#[test]
fn ct_lb5_02_stage15_whole_descents_7306_7307() {
    descent("7306");
    descent("7307");
}

/// The P* certificates of the declared runs and the DIS-1 certificates (no check record of their own).
#[test]
fn ct_lb5_02_stage15_pstar_and_dis1_certificates() {
    let mut paths: Vec<String> = SEEDS.iter().map(|s| format!("{S15}/runs/s{s}-pstar.certificate.json")).collect();
    paths.extend(["r04", "r05", "r06"].map(|r| format!("{S15}/runs/dis1-{r}.certificate.json")));
    for path in paths {
        let cert = read(&path);
        let report = check(MACRO, &cert, &inputs(&seed_of(&cert)));
        assert_eq!(report.verdict, Verdict::Pass, "{path}");
        assert!(report.complete());
    }
}

fn census(seed: &str) -> c600_checkers::CheckReport {
    let record = read(&format!("{S15}/census-{seed}.json"));
    let with = vec![
        ("chain", read(&format!("{S14}/s{seed}-t3p-chain.json"))),
        ("table", table(seed)),
        ("menu", menu_record("I_a")),
    ];
    check(PSTAR, &record, &with)
}

/// The clean-pair census at the 7304 chain end: 41 clean grips and no high piece in two clean caps, recomputed.
#[test]
fn ct_lb5_02_stage15_census_7304() {
    let report = census("7304");
    assert_eq!(report.verdict, Verdict::Pass);
    assert!(report.complete());
    assert!(report.trace.iter().any(|s| s.claim == "clean_grips" && s.computed == "41"));
    assert!(report.trace.iter().any(|s| s.claim == "chain.end_digest" && s.held));
}

#[test]
#[ignore = "about 20 s in a debug build; ct_lb5_02_stage15_census_7304 is the fast representative"]
fn ct_lb5_02_stage15_census_7307() {
    let report = census("7307");
    assert_eq!(report.verdict, Verdict::Pass);
    assert!(report.trace.iter().any(|s| s.claim == "clean_grips" && s.computed == "40"));
}

/// Every stop and run record of P* is `not_applicable`, never a pass or a failure, and its reason states its own
/// bounds: the beamed reference stop and the unbeamed pruned stop at 7304 keep different scopes.
#[test]
fn ct_lb5_02_stage15_stops_keep_their_scopes() {
    let reason = |path: &str| not_applicable(&check(PSTAR, &read(path), &[])).to_owned();
    let beamed = reason(&format!("{S15}/runs/s7304-stop-reference.json"));
    let unbeamed = reason(&format!("{S15}/runs/s7304-stop-pruned.json"));
    assert_ne!(beamed, unbeamed);
    assert!(beamed.contains("beamed stop") && beamed.contains("beam of 1000000"), "{beamed}");
    assert!(beamed.contains("exhaustive through 2"), "{beamed}");
    assert!(unbeamed.contains("unbeamed stop") && unbeamed.contains("exhaustive through 3"), "{unbeamed}");
    assert!(unbeamed.contains("not for all words"), "{unbeamed}");
    for name in [
        "s7307-stop-reference",
        "s7307-stop-pruned",
        "dis1-r05-stop",
        "dis1-r06-stop",
        "jam-7302-E200",
        "jam-7303-E200",
        "jam-7304-E200",
        "jam-7305-E200",
        "jam-7306-E200",
        "jam-7307-E200",
    ] {
        let reason = reason(&format!("{S15}/runs/{name}.json"));
        assert!(reason.contains("no KN6 checker of the search's coverage"), "{name}: {reason}");
    }
    // The E₀ = 200 steps are negatives for family E only.
    let e200 = reason(&format!("{S15}/runs/jam-7304-E200.json"));
    assert!(e200.contains("families [\"E\"]") && e200.contains("E0 200"), "{e200}");
}

/// `pstar2/2` records and the run wrappers (format only inside, `run.format`) are recognized: each answers with its
/// scope, never with an unknown-format refusal or a failure.
#[test]
fn ct_lb5_02_stage15_pstar2_and_run_records_are_recognized() {
    for seed in SEEDS {
        let report = check(PSTAR, &read(&format!("{S15}/runs/s{seed}-pstar2.json")), &[]);
        let reason = not_applicable(&report);
        assert!(reason.starts_with("a two-level P* run"), "{seed}: {reason}");
        let wrapper = check(PSTAR, &read(&format!("{S15}/runs/s{seed}-pstar.json")), &[]);
        assert!(not_applicable(&wrapper).starts_with("a declared P* run record"), "{seed}");
    }
    for r in ["r04", "r05", "r06"] {
        let wrapper = check(PSTAR, &read(&format!("{S15}/runs/dis1-{r}.json")), &[]);
        assert!(not_applicable(&wrapper).starts_with("a declared P* run record"), "{r}");
    }
}

/// The chain binding: a certificate whose prefix is not its jam's chain word fails, and one whose table is no jam's
/// is the wrong start table.
#[test]
fn ct_lb5_02_stage15_chain_binding() {
    let cert = read(&format!("{S15}/runs/dis1-r05.certificate.json"));
    let mut doc = parse(&cert);
    let Value::Object(top) = &mut doc else { unreachable!() };
    let Value::Array(prefix) = top.get_mut("prefix").unwrap() else { unreachable!() };
    prefix.pop();
    let report = check(MACRO, doc.canonical().as_bytes(), &inputs("7304"));
    assert_eq!(report.verdict.claim(), Some("prefix"));
    // Without the catalogue the certificate cannot be bound.
    let report = check(MACRO, &cert, &[("table", table("7304")), ("menu", menu_record("I_a"))]);
    assert_eq!(report.verdict.claim(), Some("jams"));
}

/// s15.macro's fixtures: a short certificate and a check record.
fn macro_fixtures() -> [c600_checkers::gate::Fixture; 2] {
    let certificate = archive_fixture(
        "dis1-r05",
        MACRO,
        read(&format!("{S15}/runs/dis1-r05.certificate.json")),
        inputs("7304"),
        &[
            "table_digest",
            "start.digest",
            "start.phi",
            "macros[0].claims.supports[1].size",
            "macros[0].claims.delta[2]",
            "macros[0].claims.digests[2]",
            "macros[0].claims.F",
            "macros[0].claims.D",
        ],
        table("7305"),
    );
    let mut with = inputs("7302");
    with.push(("certificate", read(&format!("{S15}/certificates/R01.json"))));
    let record = archive_fixture(
        "R01-check",
        MACRO,
        read(&format!("{S15}/certificates/R01-check.json")),
        with,
        &["digest", "phi", "passed", "replay.events", "replay.digest", "macros[0].passed", "macros[0].claims.D"],
        table("7303"),
    );
    [certificate, record]
}

/// s15.macro's mutation gate on a short certificate and on a check record.
#[test]
fn ct_kn6_03_stage15_macro_gate() {
    let [certificate, record] = macro_fixtures();
    assert_eq!(assert_gate(&certificate).tampered(), 8);
    assert_eq!(assert_gate(&record).tampered(), 7);
}

/// s15.macro under the structural mutations (a key deleted, renamed or added, an array emptied, an element dropped)
/// at the gate's claim paths: each fails.
#[test]
#[ignore = "about two minutes in a debug build (one check per mutation); ct_kn6_03_stage15_macro_gate is the fast gate"]
fn ct_kn6_03_stage15_macro_structural_mutations() {
    for f in macro_fixtures() {
        assert_fixture_structure(&f);
    }
}

/// s15.pstar's mutation gate on the 7304 census: digest, Φ, the clean grips, the pair count and exhaustion.
#[test]
#[ignore = "about a minute in a debug build (five full censuses); ct_lb5_02_stage15_census_7304 runs the census"]
fn ct_kn6_03_stage15_pstar_gate() {
    assert_eq!(assert_gate(&census_fixture()).tampered(), 5);
}

/// s15.pstar under the structural mutations at the census gate's claim paths: each fails at the record's shape.
#[test]
fn ct_kn6_03_stage15_pstar_structural_mutations() {
    assert_fixture_structure(&census_fixture());
}

/// A record of no known form fails at `format` rather than answer `not_applicable`.
#[test]
fn ct_kn6_03_stage15_pstar_unknown_records_fail() {
    let base = parse(&read(&format!("{S15}/runs/s7304-stop-pruned.json")));
    let (mut wrong, mut none) = (base.clone(), base);
    if let Value::Object(top) = &mut wrong {
        top.insert("format".into(), Value::String("pstar2-stop/9".into()));
    }
    if let Value::Object(top) = &mut none {
        top.remove("format");
    }
    let report = check(PSTAR, wrong.canonical().as_bytes(), &[]);
    assert_eq!(report.verdict.claim(), Some("format"));
    let report = check(PSTAR, none.canonical().as_bytes(), &[]);
    assert_eq!(report.verdict.claim(), Some("format"));
    let report = check(MACRO, none.canonical().as_bytes(), &inputs("7304"));
    assert_eq!(report.verdict.claim(), Some("format"));
}

fn census_fixture() -> c600_checkers::gate::Fixture {
    let with = vec![
        ("chain", read(&format!("{S14}/s7304-t3p-chain.json"))),
        ("table", table("7304")),
        ("menu", menu_record("I_a")),
    ];
    archive_fixture(
        "census-7304",
        PSTAR,
        read(&format!("{S15}/census-7304.json")),
        with,
        &["digest", "phi", "clean_grips", "pair_count", "exhausted"],
        table("7307"),
    )
}

/// The census's binding: a table that is not the chain's start table fails as the wrong start table, before any
/// grip is classified.
#[test]
fn ct_kn6_03_stage15_pstar_wrong_table_fails_first() {
    let record = read(&format!("{S15}/census-7304.json"));
    let with = vec![
        ("chain", read(&format!("{S14}/s7304-t3p-chain.json"))),
        ("table", table("7303")),
        ("menu", menu_record("I_a")),
    ];
    let report = check(PSTAR, &record, &with);
    assert_eq!(report.verdict.code(), Some(codes::START_TABLE_MISMATCH));
    let _ = CheckerKind::Archive(PSTAR);
}

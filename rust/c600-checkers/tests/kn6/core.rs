//! The check entry, the report and the archive family's registration point.

use c600_checkers::archive::{self, SubChecker};
use c600_checkers::gate;
use c600_checkers::kinds::{self, Checker, Needs};
use c600_checkers::report::Sources;
use c600_checkers::{check, codes, CertificateRef, CheckerKind, Engines, Verdict};

use super::common::*;

const KINDS: [CheckerKind; 4] =
    [CheckerKind::Admissibility, CheckerKind::Macro, CheckerKind::Expansion, CheckerKind::Checkpoints];

fn hex64(s: &str) -> bool {
    s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit())
}

#[test]
fn ct_kn6_01_core_every_report_carries_its_identities_and_trace() {
    let mut identities = Vec::new();
    for kind in KINDS {
        // A certificate of another format: not applicable, with the checker's identity, the input digest and the
        // trace of the input check.
        let bytes = br#"{"format": "another/1"}"#;
        let report = run(kind, bytes, &[]);
        assert!(matches!(report.verdict, Verdict::NotApplicable { .. }), "{kind:?}");
        assert!(hex64(&report.checker.source), "{kind:?}");
        assert_eq!(report.input, sha(bytes));
        assert_eq!(report.trace.first().map(|s| (s.claim.as_str(), s.held)), Some(("input", true)));
        // The identity is stable from call to call.
        assert_eq!(run(kind, bytes, &[]).checker, report.checker);
        identities.push(report.checker);
    }
    let mut ids: Vec<_> = identities.iter().map(|c| c.id).collect();
    let mut sources: Vec<_> = identities.iter().map(|c| c.source.clone()).collect();
    ids.dedup();
    sources.sort();
    sources.dedup();
    assert_eq!((ids.len(), sources.len()), (4, 4), "each kind has its own identity");
}

#[test]
fn ct_kn6_01_core_wrong_input_digest_and_refusals() {
    let bytes = br#"{"format": "macro-certificate/1"}"#;
    let wrong = sha(b"another");
    let report = check(&engines(), &CertificateRef { digest: &wrong, bytes, inputs: &[] }, CheckerKind::Macro).unwrap();
    assert_eq!(report.verdict.claim(), Some("input"));
    assert_eq!(report.verdict.code(), Some(codes::CERTIFICATE_REJECTED));
    assert_eq!(report.input, sha(bytes));
    // A kind whose engine was not supplied, and an archive ID that no stage registers, are refusals, not reports.
    let digest = sha(bytes);
    let cert = CertificateRef { digest: &digest, bytes, inputs: &[] };
    let none = Engines { kernel: Some(kernel()), jumble: None };
    assert_eq!(check(&none, &cert, CheckerKind::Macro).unwrap_err().code, codes::NO_CHECKER);
    assert_eq!(check(&engines(), &cert, CheckerKind::Archive("s15.none")).unwrap_err().code, codes::NO_CHECKER);
    // A malformed certificate fails as a claim and never panics.
    for kind in KINDS {
        let report = run(kind, b"not json", &[]);
        assert_eq!(report.verdict.claim(), Some("certificate"), "{kind:?}");
    }
}

static EXAMPLE_SOURCES: Sources = Sources::new(
    "s15.example-macro-certificate",
    &[
        ("c600-checkers/Cargo.toml", include_bytes!("../../Cargo.toml")),
        ("c600-checkers/src/claims.rs", include_bytes!("../../src/claims.rs")),
        ("c600-checkers/src/kinds/macros.rs", include_bytes!("../../src/kinds/macros.rs")),
        ("c600-checkers/src/kinds/mod.rs", include_bytes!("../../src/kinds/mod.rs")),
        ("c600-checkers/src/lib.rs", include_bytes!("../../src/lib.rs")),
        ("c600-checkers/src/replay.rs", include_bytes!("../../src/replay.rs")),
        ("c600-checkers/src/report.rs", include_bytes!("../../src/report.rs")),
        ("c600-checkers/tests/kn6/core.rs", include_bytes!("core.rs")),
    ],
);

/// A sub-checker that covers only its own file.
static NARROW_SOURCES: Sources =
    Sources::new("s15.narrow", &[("c600-checkers/tests/kn6/core.rs", include_bytes!("core.rs"))]);
static NARROW: SubChecker = SubChecker {
    checker: Checker { sources: &NARROW_SOURCES, needs: Needs::JUMBLE, run: run_macro },
    stage: 15,
    family: "S15 macro-certificate/1",
};

#[test]
fn ct_kn6_01_core_identities_cover_the_pass_logic() {
    // Every kind's identity covers the shared files that decide a pass, and its own file.
    for (sources, own) in [
        (&kinds::admissibility::SOURCES, "c600-checkers/src/kinds/admissibility.rs"),
        (&kinds::macros::SOURCES, "c600-checkers/src/kinds/macros.rs"),
        (&kinds::expansion::SOURCES, "c600-checkers/src/kinds/expansion.rs"),
        (&kinds::checkpoints::SOURCES, "c600-checkers/src/kinds/checkpoints.rs"),
    ] {
        let named: Vec<&str> = sources.files.iter().map(|(name, _)| *name).collect();
        for shared in archive::SHARED_SOURCES.iter().chain([&own]) {
            assert!(named.contains(shared), "{} does not cover {shared}", sources.id);
        }
    }
    // A sub-checker whose identity misses the shared files is a broken registration.
    let problems = archive::registry_problems([&NARROW]);
    assert_eq!(problems.len(), archive::SHARED_SOURCES.len() + 1, "{problems:#?}");
}

/// A sub-checker registered nowhere: stage 15's macro certificates checked by the macro kind's run, under its own
/// identity, as a stage piece may register one.
static EXAMPLE: SubChecker = SubChecker {
    checker: Checker { sources: &EXAMPLE_SOURCES, needs: Needs::JUMBLE, run: run_macro },
    stage: 15,
    family: "S15 macro-certificate/1",
};

fn run_macro(engines: &Engines, run: &mut c600_checkers::Run) -> c600_checkers::Step<()> {
    (kinds::macros::CHECKER.run)(engines, run)
}

#[test]
fn ct_kn6_01_core_archive_registration_point() {
    // The registry holds no sub-checker yet and its invariants hold; a broken registration is reported by name.
    assert_eq!(archive::registry().count(), 0);
    assert!(archive::registry_problems(archive::registry()).is_empty());
    assert!(archive::registry_problems([&EXAMPLE]).is_empty());
    let problems = archive::registry_problems([&EXAMPLE, &EXAMPLE]);
    assert_eq!(problems, ["s15.example-macro-certificate is registered twice"]);
    // A sub-checker runs through the same report and the same mutation gate as every kind.
    let cert = read("research/jumbling/restoration/15-lowering-machine/results/certificates/R05.json");
    let table = read("research/jumbling/restoration/13-tutorial-runner/runs/dev/d2-s7304/component/tables/00005.json");
    let other = read("research/jumbling/restoration/13-tutorial-runner/runs/dev/d2-s7305/component/tables/00010.json");
    let mut f = fixture(
        "R05 through a sub-checker",
        CheckerKind::Archive(EXAMPLE.id()),
        cert,
        vec![("table", table), ("menu", menu_record("I_a"))],
        &["table_digest", "macros[0].claims.D", "macros[0].claims.digests[2]"],
        Some(("table", other)),
    );
    f.skip.clear();
    let report = gate::run_on(&engines(), &f, &EXAMPLE.checker).unwrap();
    assert!(report.passed(), "{:#?}", report.problems);
    assert_eq!(report.baseline.checker.id, "s15.example-macro-certificate");
    assert_ne!(report.baseline.checker.source, kinds::macros::SOURCES.identity().source);
}

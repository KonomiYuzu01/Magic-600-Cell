//! CT-DET-24 (part: the archive's runs at the research host, and the KN6-checked witness of the definition's last
//! clause). The LP2 and R0 halves (`c600-archive/tests/import.rs` on a synthetic archive, `tests/test_lp5_archive.py`
//! on the corpus) ran the import of an archived timed stop and of stage 19's data-only archive; they waited for R8's
//! checked witnesses. Here the import of the corpus's own members, granted by the Git-blob manifest, gives archived
//! runs that keep their recorded outcome and limit and supply no new run, and stage 15's stored certificate R01 counts
//! only through a KN6 check job of its own, with that job's record and scope.

use c600_archive::import::{import_archive, GrantedFile, GrantedFiles, ImportReport};
use c600_archive::{Discriminator, ImportTable, ImportTableRef, Limits, LimitsRef};
use c600_checkers::{CheckerKind, Verdict};
use c600_evidence::{Covered, Covers, DigestRef, Form, Graph, Key, Scope, Statement, StatementId};
use c600_research::archived::archived_runs;
use c600_research::evidence::{admit, covered};
use c600_research::{Check, CheckJob, Det30, Granted, Pinned};
use c600_session::persist::record::identity_json;
use c600_session::persist::run::{codes, Record, Reproduction};

use crate::common::*;

const MANIFEST: &str = "work/experiments/language-lx/envelope-facts/MANIFEST.json";
const STOP: &str = "research/jumbling/restoration/15-lowering-machine/results/runs/s7305-pstar.json";
const R01: &str = "research/jumbling/restoration/15-lowering-machine/results/certificates/R01.json";
const S19_MANIFEST: &str = "research/jumbling/restoration/19-height-depth-framework/IMPORT-MANIFEST.json";
/// R01's start table: stage 13's checkpoint table, granted by the host (it is outside the stage 14 to 19 corpus).
const TABLE: &str =
    "research/jumbling/restoration/13-tutorial-runner/runs/dev/d2-s7302/component-checkpoint/tables/00021.json";
/// The digest of the start table R01 states (`table_digest`).
const R01_TABLE_DIGEST: &str = "2430f4cc99ea121a6d3476ec21b66fc401e277d13809ad5e1c979e6e0abe1472";
/// The pinned exact payload of R01's check job (SHA-256 of its FP.5 bytes).
const R01_PAYLOAD: &str = "217108fd11590f7fd679421315790d767ffef3ec5807fefa8527412344a06c75";

fn import() -> (ImportReport, ImportTable) {
    let granted = GrantedFiles {
        manifest: GrantedFile { name: MANIFEST.into(), bytes: read(MANIFEST) },
        members: [STOP, R01, S19_MANIFEST]
            .into_iter()
            .map(|name| GrantedFile { name: name.into(), bytes: read(name) })
            .collect(),
    };
    let table = ImportTable::edition_1();
    let discriminator = Discriminator::edition_1();
    let report = import_archive(
        granted,
        ImportTableRef { table: &table, discriminator: &discriminator },
        LimitsRef(&Limits::research_archive()),
    )
    .expect("the corpus manifest reads");
    (report, table)
}

fn member(report: &ImportReport, name: &str) -> usize {
    report.member(name).unwrap_or_else(|| panic!("{name} is listed"))
}

/// CT-DET-24 (part: an archived timed stop at the research host): stage 15's cut-off P* run imports with its status and
/// reason as run facts that keep their recorded outcome and claim and read as stops, and with its exhausted families
/// as steps completed within their declared search set only. Every one reproduces as `not run` for want of a key; the
/// member's bytes are no run record, so neither the record reader nor the job's reproduction accepts them.
#[test]
fn ct_det_24_archived_timed_stop_supplies_no_new_run() {
    let (report, table) = import();
    let stop = member(&report, STOP);
    assert_eq!(report.members[stop].integrity(), "pass");
    let runs: Vec<_> = archived_runs(&report, &table).into_iter().filter(|run| run.member == STOP).collect();
    let (stops, steps): (Vec<_>, Vec<_>) = runs.iter().partition(|run| run.is_stop());
    let mut fields: Vec<&str> = stops.iter().map(|run| run.field.as_str()).collect();
    fields.sort_unstable();
    assert_eq!(fields, ["run.reason", "run.status"]);
    for run in &stops {
        assert_eq!(run.job_outcome, "cut off");
        assert_eq!((run.claim.as_deref(), run.check.as_deref()), (Some("open"), Some("producer")));
    }
    assert!(!steps.is_empty());
    for run in &steps {
        assert!(run.field.starts_with("run.steps[].families.") && run.field.ends_with(".exhausted"), "{run:?}");
        assert_eq!((run.job_outcome.as_str(), run.scope.as_deref()), ("completed", Some("declared search set")));
    }
    for run in &runs {
        assert_eq!(run.reproduction(), Reproduction::NotRun(codes::INPUT_MISSING.into()));
    }
    let bytes = report.stored_bytes(stop).expect("granted bytes");
    assert!(Record::read(bytes).is_err(), "an archived run is not a run record");
    let attempt = c600_research::reproduce(&engines(), bytes, &Default::default(), &c600_research::Cancel::new());
    assert!(matches!(attempt.answer, Reproduction::Refused(_)) && attempt.run.is_none());
}

/// CT-DET-24 (part: stage 19's data-only archive): its granted member is data, its recheck is not run, and the import
/// runs no producer.
#[test]
fn ct_det_24_data_only_archive_is_not_rechecked() {
    let (report, table) = import();
    let s19 = member(&report, S19_MANIFEST);
    assert_eq!(report.members[s19].integrity(), "pass");
    assert!(report.members[s19].recheck.starts_with("not run"));
    assert_eq!(report.totals()["producers_run"], 0);
    // No archived run, from stage 19 or elsewhere, is a run this host can repeat.
    assert!(archived_runs(&report, &table).iter().all(|run| matches!(run.reproduction(), Reproduction::NotRun(_))));
}

/// CT-DET-24 (part: the definition's last clause, a KN6-checked witness): stage 15's stored certificate R01, as the
/// import holds its bytes and the manifest states their digest, passes a KN6 check job pinned at the start table it
/// names under the I_a identity block. The job's record reproduces byte for byte, its leaf covers the pinned table, and
/// a report about another input is refused.
#[test]
fn ct_det_24_r01_counts_only_through_a_check_job() {
    let (report, _) = import();
    let r01 = member(&report, R01);
    let certificate = report.stored_bytes(r01).expect("granted bytes").to_vec();
    let stated = report.members[r01].listed.stored_sha256.clone();
    let doc = c600_checkers::claims::document(&certificate).ok().expect("JSON");
    assert_eq!(doc.get("table_digest").and_then(|v| v.as_str()), Some(R01_TABLE_DIGEST));

    let menu = menu_record();
    let block = loaded().jumbling_identity(&menu).expect("the I_a menu");
    let check = Check {
        kind: CheckerKind::Macro,
        certificate,
        stated,
        inputs: vec![
            Granted { role: "table".into(), bytes: read(TABLE) },
            Granted { role: "menu".into(), bytes: menu },
        ],
        cap: None,
    };
    let job = CheckJob {
        checks: vec![check],
        input: Pinned::stated("poses", R01_TABLE_DIGEST, identity_json(&block)),
        budget: WIDE,
    };
    let run = run_job(job);
    assert_eq!(run.reading(), Det30::Complete);
    let report = run.children()[0].report.clone().expect("a report");
    assert_eq!(report.verdict, Verdict::Pass);
    let table = report.inputs.iter().find(|input| input.role == "table").expect("the table is bound");
    assert!(table.identity.iter().any(|(name, value)| name == "digest" && value == R01_TABLE_DIGEST), "{table:?}");
    assert_eq!(covered(&run), Covered::new(Covers::OneInstance, Some(DigestRef::poses(R01_TABLE_DIGEST))));

    let again = reproduce(&run);
    assert_eq!(again.answer, Reproduction::Same(run.payload().clone()));
    assert_eq!(payload_digest(&run), R01_PAYLOAD, "the pinned payload");

    let statement = StatementId::named("r8", "0123456789abcdef0123456789abcdef01234567", "R01 lowers");
    let scope = |input: &str| {
        Scope::new(Covers::OneInstance, &run.input().identity_text(), "contract:r8", Some(DigestRef::poses(input)))
    };
    let mut g = Graph::new();
    g.add_statement(Statement::new(statement.clone()).form(Form::Finite)).expect("a statement");
    let here = Key::new(statement.clone(), scope(R01_TABLE_DIGEST));
    g.root(&here);
    assert!(admit(&mut g, &here, &run).expect("admitted")[0].is_some());
    let elsewhere = Key::new(statement, scope(&hex_of("another table")));
    assert!(admit(&mut g, &elsewhere, &run).is_err(), "a report about R01's table is about that table only");
}

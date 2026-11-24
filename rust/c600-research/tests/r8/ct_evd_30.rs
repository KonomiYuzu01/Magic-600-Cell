//! CT-EVD-30 (part: only a KN6 checker in the kernel produces the report that writes check `independent checker`).
//! The LP2 half (`c600-evidence/tests/sinks.rs`) ran the library's entry paths: no writer sets the value at the
//! `record` sink, and a `CheckReport` enters through `admit_check_report`. Its "not run" part runs here: the research
//! host's only path to `admit_check_report` takes a `CheckRun`, which only a job's run or a matching reproduction
//! makes, so the report is the kernel checker's own; a stored record whose payload was changed reproduces to nothing,
//! and no import-table rule sets the value.

use c600_archive::{ImportTable, Outcome};
use c600_checkers::CheckerKind;
use c600_evidence::{
    record, Basis, Check as EvidenceCheck, Citation, Covered, Covers, DigestRef, Form, Grade, Graph, Key, Leaf, Scope,
    Statement, StatementId, Writer,
};
use c600_research::evidence::{admit, covered};
use c600_research::{Check, CheckJob, CheckRun, Pinned};
use c600_session::persist::codec::Json;
use c600_session::persist::record::identity_json;
use c600_session::persist::run::{codes, Record, Reproduction};

use crate::common::*;

const COMMIT: &str = "0123456789abcdef0123456789abcdef01234567";
const FIELD_NOT_WRITABLE: &str = "EVD:capability.field_not_writable";
const SCOPE_MISMATCH: &str = "EVD:evidence.scope_mismatch";

fn independent() -> Grade {
    Grade::new(Basis::Certificate, EvidenceCheck::IndependentChecker)
}

/// A target statement about the job's pinned input, in a graph that holds it.
fn target(run: &CheckRun, input: DigestRef) -> (Graph, Key) {
    let id = StatementId::named("r8", COMMIT, "the word's expansion");
    let key =
        Key::new(id.clone(), Scope::new(Covers::OneInstance, &run.input().identity_text(), "contract:r8", Some(input)));
    let mut g = Graph::new();
    g.add_statement(Statement::new(id).form(Form::Finite)).expect("a statement");
    g.root(&key);
    (g, key)
}

fn pinned_digest(run: &CheckRun) -> DigestRef {
    DigestRef::labels(&run.input().digest)
}

const WORD: [i32; 3] = [44, -1000, 7];

/// A session after `before`, its head opened read-only, and the start record of that revision.
fn pinned_after(before: &[i32]) -> (c600_session::SnapshotHandle, Vec<u8>) {
    let s = session();
    if !before.is_empty() {
        committed(&commit(&s, "before", before));
    }
    let pin = snapshot(&s);
    let start = labels_record(&pin);
    (pin, start)
}

/// An expansion check of `WORD` with its replay from `start` (at digest `from`) to the stated `end`.
fn replay(start: &[u8], from: &str, end: &str) -> Check {
    let certificate = expansion(&WORD, Some((start, from, end)));
    Check::new(CheckerKind::Expansion, certificate, vec![("start", start.to_vec())])
}

/// A job pinned at the solved revision of a session with four checks: a pass and a fail (a wrong end) that both read
/// the pinned start, a fail that read no state (a wrong word digest, no replay) and a not-applicable one (an expansion
/// certificate read as a macro certificate).
fn four_verdicts() -> CheckRun {
    let (pin, start) = pinned_after(&[]);
    let end = end_digest(&[], &WORD);
    let checks = vec![
        replay(&start, &pin.revision.digest, &end),
        replay(&start, &pin.revision.digest, &hex_of("not the end")),
        Check::new(CheckerKind::Expansion, long_program(1, 2, &hex_of("wrong")), Vec::new()),
        Check::new(CheckerKind::Macro, expansion(&[1], None), Vec::new()),
    ];
    run_job(CheckJob { checks, input: Pinned::snapshot(&pin), budget: WIDE })
}

/// CT-EVD-30 (part: a job's reports enter as the checker's own): a pass whose checker read the pinned start is a live
/// leaf (certificate, independent checker) identified by the checker's source digest and the certificate's digest and
/// covering the pinned input; a fail that read it is a failed leaf; a fail that read no state and a not-applicable
/// verdict add none.
#[test]
fn ct_evd_30_job_reports_enter_as_independent_checker_leaves() {
    let run = four_verdicts();
    let (mut g, key) = target(&run, pinned_digest(&run));
    let leaves = admit(&mut g, &key, &run).expect("admitted");
    assert_eq!(leaves.len(), 4);
    let pass = g.leaf(leaves[0].expect("a pass leaf")).expect("a leaf").clone();
    let report = run.children()[0].report.clone().expect("a report");
    assert_eq!(run.input().bound_by(&report), Some(true));
    assert_eq!(pass.grade, independent());
    assert_eq!(pass.support, vec![report.checker.source.clone(), report.input.clone()]);
    assert_eq!(pass.covered, Some(covered(&run)));
    assert_eq!(covered(&run), Covered::new(Covers::OneInstance, Some(pinned_digest(&run))));
    assert!(!pass.failed);
    let fail = g.leaf(leaves[1].expect("a fail leaf")).expect("a leaf");
    assert!(fail.failed, "a failed check is kept for display and never live");
    assert_eq!(leaves[2], None, "a fail that read no start state says nothing about the pinned input");
    assert_eq!(leaves[3], None, "a not-applicable verdict adds no leaf");
}

/// CT-EVD-30 (part: the covered input is the checker's, not the caller's): a pass whose checker read another start
/// than the pinned revision is refused, by `result_at` as stale and by the admission as about another input, and a
/// pass that read no start state covers no pinned input.
#[test]
fn ct_evd_30_a_report_about_another_state_is_refused() {
    let s = session();
    committed(&commit(&s, "before", &[3, 4]));
    let earlier = snapshot(&s);
    let start = labels_record(&earlier);
    committed(&commit(&s, "next", &[5]));
    let later = snapshot(&s);
    let end = end_digest(&[3, 4], &WORD);
    let check = replay(&start, &earlier.revision.digest, &end);
    let run = run_job(CheckJob { checks: vec![check], input: Pinned::snapshot(&later), budget: WIDE });
    let report = run.children()[0].report.clone().expect("a report");
    assert_eq!(report.verdict, c600_checkers::Verdict::Pass, "the certificate replays from its own start");
    assert_eq!(run.input().bound_by(&report), Some(false));
    assert_eq!(run.result_at(&later.revision).expect_err("read another state").code, "KN5:stale.revision_changed");
    let (mut g, key) = target(&run, pinned_digest(&run));
    let before = g.clone();
    assert_eq!(admit(&mut g, &key, &run).expect_err("about another state").code, SCOPE_MISMATCH);
    assert_eq!(g, before);

    let unbound =
        run_job(CheckJob { checks: vec![short_check(&[1, 2])], input: Pinned::snapshot(&later), budget: WIDE });
    assert_eq!(unbound.input().bound_by(unbound.children()[0].report.as_ref().expect("a report")), None);
    assert_eq!(unbound.result_at(&later.revision).expect("a state-free report reads at its revision").len(), 1);
    let (mut g, key) = target(&unbound, pinned_digest(&unbound));
    assert_eq!(admit(&mut g, &key, &unbound).expect_err("covers no input").code, SCOPE_MISMATCH);
}

/// CT-EVD-30 (part: a stop is no report): a check that was cut off adds nothing, and a report about another input or
/// under another identity block is refused whole.
#[test]
fn ct_evd_30_a_stop_adds_nothing_and_another_scope_is_refused() {
    let mut capped = long_check(10, 10);
    capped.cap = Some(c600_research::Budget { kernel_cost: 5, ..WIDE });
    let (pin, start) = pinned_after(&[9]);
    let end = end_digest(&[9], &WORD);
    let checks = vec![capped, replay(&start, &pin.revision.digest, &end)];
    let run = run_job(CheckJob { checks, input: Pinned::snapshot(&pin), budget: WIDE });
    let (mut g, key) = target(&run, pinned_digest(&run));
    let leaves = admit(&mut g, &key, &run).expect("admitted");
    assert_eq!(leaves[0], None, "a cut-off check has no report");
    assert!(leaves[1].is_some());

    let (mut g, key) = target(&run, DigestRef::labels(&hex_of("another start")));
    let before = g.clone();
    assert_eq!(admit(&mut g, &key, &run).expect_err("another input").code, SCOPE_MISMATCH);
    assert_eq!(g, before, "a refusal leaves the graph unchanged");

    let mut other = run_job(stated_job(vec![short_check(&[8])], WIDE)).input().clone();
    other.identity = identity_json(&loaded().jumbling_identity(&menu_record()).expect("a menu"));
    let id = StatementId::named("r8", COMMIT, "s");
    let elsewhere =
        Key::new(id, Scope::new(Covers::OneInstance, &other.identity_text(), "contract:r8", Some(pinned_digest(&run))));
    let mut g = Graph::new();
    assert_eq!(admit(&mut g, &elsewhere, &run).expect_err("another identity").code, SCOPE_MISMATCH);
}

/// CT-EVD-30 (part: no other path): a stored record whose payload was changed to claim another report, rewritten as a
/// well-formed record, reproduces to a mismatch and gives no run, so it never reaches evidence; the `record` sink
/// refuses the value to the host; and no rule of the import table sets check `independent checker`.
#[test]
fn ct_evd_30_stored_bytes_imports_and_writers_cannot_write_independent_checker() {
    let run = run_job(stated_job(
        vec![Check::new(CheckerKind::Expansion, long_program(1, 2, &hex_of("wrong")), Vec::new())],
        WIDE,
    ));
    let mut forged: Record = run.record().clone();
    if let Json::Object(payload) = &mut forged.result {
        payload
            .insert("outputs".into(), Json::List(vec![c600_session::persist::codec::digest_ref("bytes", b"a pass")]));
    }
    let bytes = forged.write().expect("a well-formed record");
    let answer = c600_research::reproduce(&engines(), &bytes, &run.available(), &c600_research::Cancel::new());
    assert_eq!(answer.answer, Reproduction::Refused(codes::REPRODUCIBILITY.into()));
    assert!(answer.run.is_none(), "a mismatch gives no run, so nothing to admit");

    let (mut g, key) = target(&run, pinned_digest(&run));
    let leaf = Leaf::new(&key, independent()).support(&hex_of("checker")).support(&hex_of("certificate"));
    for writer in [Writer::Host, Writer::Kernel, Writer::Import, Writer::UserProgram] {
        assert_eq!(
            record(&mut g, writer, &key, leaf.clone(), &Citation::none()).expect_err("refused").code,
            FIELD_NOT_WRITABLE
        );
    }

    let table = ImportTable::edition_1();
    for rule in table.rules() {
        if let Outcome::Facts(facts) = &rule.outcome {
            assert_ne!(facts.check.as_deref(), Some("independent checker"), "an import rule sets independent checker");
        }
    }
}

/// The pinned input of a session job is a labels digest; of a stated start, the level the caller states.
#[test]
fn covered_set_names_the_pinned_level() {
    let mut job = stated_job(vec![short_check(&[1])], WIDE);
    job.input = Pinned::stated("poses", &hex_of("a table"), job.input.identity.clone());
    let run = run_job(job);
    assert_eq!(covered(&run), Covered::new(Covers::OneInstance, Some(DigestRef::poses(&hex_of("a table")))));
}

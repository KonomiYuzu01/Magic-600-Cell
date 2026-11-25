//! The cooperative stop of a check (`check_until`, `claims::scoped`; KN7.6, KN7.10) and the owned engines
//! (`engines::Loaded`), LP5 piece R8's additions to the checker core.
//!
//! A stop scope's hook reaches a running checker at its next letter: a long expansion or a long replay stops there with
//! the hook's code and the units charged, and gives no verdict. A scope inside another is charged in both, so a nested
//! check cannot run past what an enclosing scope has left; an inner scope's own stop ends it alone. A check outside
//! every scope charges nothing and reports as before.
//!
//! Test names carry the CT ID and part: CT-DET-11's nested-check part (the job's child scopes are in
//! `c600-research/tests/r8/ct_det_11.rs`).

#[path = "kn6/common.rs"]
mod common;

use std::cell::Cell;
use std::rc::Rc;

use c600_checkers::claims::{self, scoped};
use c600_checkers::engines::Loaded;
use c600_checkers::{check, check_until, codes, CertificateRef, Checked, CheckerKind, Verdict};
use c600_kernel::json::Value;

use common::*;

const CUT: &str = "KN7:budget.kernel_cost";
const OUTER: &str = "KN7:budget.steps";

/// A program of `2 · a · b` letters, `(7, -8)` repeated, with `digest` as its claimed SHA-256.
fn long_program(a: u64, b: u64, digest: &str) -> Vec<u8> {
    let rule = |items: Vec<Value>| Value::Array(items);
    let reference = |r: i64, times: u64| obj(vec![("rule", int(r)), ("times", int(times as i64))]);
    obj(vec![
        ("format", text("kn6-expansion/1")),
        ("alphabet", text("full")),
        ("inputs", obj(vec![])),
        (
            "rules",
            Value::Array(vec![rule(vec![int(7), int(-8)]), rule(vec![reference(0, a)]), rule(vec![reference(1, b)])]),
        ),
        ("root", int(2)),
        ("claims", obj(vec![("length", int((2 * a * b) as i64)), ("sha256", text(digest))])),
    ])
    .canonical()
    .into_bytes()
}

/// The SHA-256 of `(7, -8)` repeated `n` times as little-endian int32.
fn pairs_digest(n: u64) -> String {
    let mut hash = c600_digest::Sha256::new();
    for _ in 0..n {
        hash.update(&7i32.to_le_bytes());
        hash.update(&(-8i32).to_le_bytes());
    }
    c600_digest::hex(&hash.finish())
}

fn budget(limit: u64, code: &'static str) -> impl FnMut(u64) -> Option<&'static str> + 'static {
    move |total| (total > limit).then_some(code)
}

fn until(certificate: &[u8], limit: u64) -> Checked {
    let digest = sha(certificate);
    let cert = CertificateRef { digest: &digest, bytes: certificate, inputs: &[] };
    check_until(&engines(), &cert, CheckerKind::Expansion, budget(limit, CUT)).expect("a checker")
}

/// A 2·10^12-letter expansion under a budget of 10,000 units stops at exactly that unit with the budget's code, twice
/// alike, and gives no verdict, though its claimed digest is wrong and a completed check would fail.
#[test]
fn ct_det_11_long_expansion_stops_at_its_budget_without_a_verdict() {
    let cert = long_program(1_000_000, 1_000_000, &sha(b"not the word"));
    let first = until(&cert, 10_000);
    assert_eq!(first, Checked::Stopped { code: CUT, charged: 10_000 });
    assert_eq!(until(&cert, 10_000), first, "the stop position repeats");
    assert_eq!(claims::stopped(), None, "no scope is left open");
}

/// A short expansion within its budget reports as a plain check does, and charges one unit per letter produced.
#[test]
fn ct_det_11_short_expansion_reports_and_charges_its_letters() {
    let cert = long_program(10, 50, &pairs_digest(500));
    let plain = run(CheckerKind::Expansion, &cert, &[]);
    assert_eq!(plain.verdict, Verdict::Pass);
    match until(&cert, 1_000) {
        Checked::Report { report, charged } => {
            assert_eq!(report, plain, "a scope changes no report");
            assert_eq!(charged, 1_000);
        }
        other => panic!("not a report: {other:?}"),
    }
    // One unit short of the word stops it at the last letter but one.
    assert_eq!(until(&cert, 999), Checked::Stopped { code: CUT, charged: 999 });
}

/// CT-DET-11 (part: a check nested in a scope cannot mint allowance; the job's child checks are in
/// `c600-research/tests/r8/ct_det_11.rs`): a nested check that asks for no limit is stopped where the enclosing scope
/// runs out, with that scope's code; both scopes charge the same units. A nested check's own smaller cap stops it alone,
/// and the enclosing scope goes on.
#[test]
fn ct_det_11_nested_check_cannot_mint_allowance() {
    let cert = long_program(1_000, 1_000, &sha(b"x"));
    let outer = scoped(budget(500, OUTER), || until(&cert, u64::MAX)).expect("a scope");
    assert_eq!(outer.value, Checked::Stopped { code: OUTER, charged: 500 });
    assert_eq!((outer.charged, outer.stopped), (500, Some(OUTER)));

    let outer = scoped(budget(10_000, OUTER), || {
        let first = until(&cert, 300);
        let second = until(&long_program(10, 10, &pairs_digest(100)), u64::MAX);
        (first, second, claims::stopped())
    })
    .expect("a scope");
    let (first, second, stopped_inside) = outer.value;
    assert_eq!(first, Checked::Stopped { code: CUT, charged: 300 });
    assert!(matches!(second, Checked::Report { charged: 200, .. }), "{second:?}");
    assert_eq!(stopped_inside, None, "the inner scope's stop ended the inner scope only");
    assert_eq!((outer.charged, outer.stopped), (500, None), "the enclosing scope was charged every unit");
}

/// A plain check inside a stopped scope gives no report: it is refused with the scope's code.
#[test]
fn ct_det_11_plain_check_inside_a_stopped_scope_has_no_verdict() {
    let cert = long_program(1_000, 1_000, &sha(b"x"));
    let digest = sha(&cert);
    let outer = scoped(budget(100, OUTER), || {
        check(&engines(), &CertificateRef { digest: &digest, bytes: &cert, inputs: &[] }, CheckerKind::Expansion)
    })
    .expect("a scope");
    let refusal = outer.value.expect_err("no report from a stopped scope");
    assert_eq!(refusal.code, OUTER);
}

/// CT-DET-11 (part: a check started inside a scope that is already stopped): it is stopped with that scope's code and
/// gives no report, even when it fails before its first letter and so never reaches a stop of its own.
#[test]
fn ct_det_11_check_inside_a_stopped_scope_is_stopped() {
    let malformed = b"{}".to_vec();
    let outer = scoped(budget(0, OUTER), || {
        assert!(claims::poll(1).is_err(), "the enclosing scope stops at its first unit");
        until(&malformed, u64::MAX)
    })
    .expect("a scope");
    assert_eq!(outer.value, Checked::Stopped { code: OUTER, charged: 0 });
    assert_eq!((outer.charged, outer.stopped), (0, Some(OUTER)));
}

/// A hook that starts a check, with or without a scope of its own, is refused, and nothing panics: inside a hook a check
/// could be neither charged nor stopped.
#[test]
fn a_check_inside_a_hook_is_refused() {
    let short = long_program(1, 1, &pairs_digest(1));
    let answer: Rc<Cell<Option<&'static str>>> = Rc::new(Cell::new(None));
    let plain: Rc<Cell<Option<&'static str>>> = Rc::new(Cell::new(None));
    let (seen, seen_plain) = (answer.clone(), plain.clone());
    let digest = sha(&short);
    let cert = CertificateRef { digest: &digest, bytes: &short, inputs: &[] };
    let inner = short.clone();
    let result = check_until(&engines(), &cert, CheckerKind::Expansion, move |_| {
        let digest = sha(&inner);
        let cert = CertificateRef { digest: &digest, bytes: &inner, inputs: &[] };
        let nested = check_until(&engines(), &cert, CheckerKind::Expansion, |_| None);
        seen.set(nested.err().map(|refusal| refusal.code));
        seen_plain.set(check(&engines(), &cert, CheckerKind::Expansion).err().map(|refusal| refusal.code));
        None
    });
    assert!(matches!(result, Ok(Checked::Report { .. })));
    assert_eq!(answer.get(), Some(codes::NO_CHECKER));
    assert_eq!(plain.get(), Some(codes::NO_CHECKER));
}

/// A jumbling replay stops too: stage 15's R01 under a budget below its replay stops with the budget's code, and with
/// enough units it passes as the plain check does, having charged one unit per letter applied.
#[test]
fn ct_det_11_replay_stops_at_its_budget() {
    let cert = read("research/jumbling/restoration/15-lowering-machine/results/certificates/R01.json");
    let table = read(
        "research/jumbling/restoration/13-tutorial-runner/runs/dev/d2-s7302/component-checkpoint/tables/00021.json",
    );
    let menu = menu_record("I_a");
    let inputs =
        [c600_checkers::Input { role: "table", bytes: &table }, c600_checkers::Input { role: "menu", bytes: &menu }];
    let digest = sha(&cert);
    let r01 = CertificateRef { digest: &digest, bytes: &cert, inputs: &inputs };
    let plain = check(&engines(), &r01, CheckerKind::Macro).expect("a checker");
    assert_eq!(plain.verdict, Verdict::Pass);
    let full = check_until(&engines(), &r01, CheckerKind::Macro, budget(u64::MAX, CUT)).expect("a checker");
    let Checked::Report { report, charged } = full else { panic!("not a report: {full:?}") };
    assert_eq!(report, plain);
    // R01 replays its four-letter prefix and its three-letter macro.
    assert_eq!(charged, 7);
    let stopped = check_until(&engines(), &r01, CheckerKind::Macro, budget(charged - 1, CUT)).expect("a checker");
    assert_eq!(stopped, Checked::Stopped { code: CUT, charged: charged - 1 });
}

/// The owned engines load from `assets/`, give the engines a check reads and the identity blocks a host pins; a
/// folder with no model is refused with `KN6:evidence.no_checker`.
#[test]
fn owned_engines_load_and_name_their_identities() {
    let loaded = Loaded::load(&repository().join("assets")).expect("the model loads");
    let short = long_program(1, 1, &pairs_digest(1));
    let digest = sha(&short);
    let report = check(
        &loaded.engines(),
        &CertificateRef { digest: &digest, bytes: &short, inputs: &[] },
        CheckerKind::Expansion,
    )
    .expect("a checker");
    assert_eq!(report.verdict, Verdict::Pass);
    assert_eq!(loaded.full_identity(), kernel().identity());
    let menu = menu("I_a");
    assert_eq!(loaded.jumbling_identity(&menu_record("I_a")).expect("the menu reads"), jumble().identity(Some(&menu)));
    assert_eq!(loaded.jumbling_identity(b"{}").expect_err("no menu").code, codes::CERTIFICATE_REJECTED);
    let missing = Loaded::load(&repository().join("no-such-model")).err().expect("no model");
    assert_eq!(missing.code, codes::NO_CHECKER);
}

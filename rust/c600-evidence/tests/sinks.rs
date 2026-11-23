//! Chapter 06 sections 1, 3, 7, 8 and 10: the import rules (EVD.52), reuse and entry (EVD.4, EVD.18), `CheckReport`
//! admission (EVD.31, EVD.46), purity and editions (EVD.14, EVD.54), no grant from a word (EVD.56, EVD.57) and the
//! writer check at the `record` sink (EVD.64, EVD.65); with the evidence halves of CT-LB6-02 (LB6.3) and CT-PS1-27
//! (PS1.36). Expected codes are the chapter's literal strings, and expected rows come from the clauses cited.

mod common;

use std::collections::BTreeSet;

use c600_evidence::import::{h1_reachability_row, k1_check, p13_check_fact, v14, HistoricalCheck};
use c600_evidence::*;
use common::*;

/// EVD.65 and EVD.58's table.
const FIELD_NOT_WRITABLE: &str = "EVD:capability.field_not_writable";
/// EVD.18 and EVD.58's table.
const SCOPE_MISMATCH: &str = "EVD:evidence.scope_mismatch";
/// EVD.17 and EVD.58's table.
const PREMISE_UNMET: &str = "EVD:evidence.premise_unmet";
/// EVD.40 and EVD.58's table.
const INCONSISTENT_FIELDS: &str = "EVD:evidence.inconsistent_fields";

/// The crate's source files, for the scans of CT-EVD-20 and CT-EVD-21.
const SOURCES: [(&str, &str); 10] = [
    ("edition.rs", include_str!("../src/edition.rs")),
    ("error.rs", include_str!("../src/error.rs")),
    ("graph.rs", include_str!("../src/graph.rs")),
    ("import.rs", include_str!("../src/import.rs")),
    ("lib.rs", include_str!("../src/lib.rs")),
    ("model.rs", include_str!("../src/model.rs")),
    ("obligation/json.rs", include_str!("../src/obligation/json.rs")),
    ("obligation/mod.rs", include_str!("../src/obligation/mod.rs")),
    ("sink.rs", include_str!("../src/sink.rs")),
    ("word.rs", include_str!("../src/word.rs")),
];

const WRITERS: [Writer; 4] = [Writer::Kernel, Writer::Host, Writer::Import, Writer::UserProgram];

/// The `.rs` files under `dir`, as paths relative to `root` with `/` separators, folders included.
fn rust_files(root: &std::path::Path, dir: &std::path::Path, out: &mut Vec<String>) {
    for entry in std::fs::read_dir(dir).expect("source directory") {
        let path = entry.expect("entry").path();
        if path.is_dir() {
            rust_files(root, &path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            let rel = path.strip_prefix(root).expect("under src");
            out.push(rel.components().map(|c| c.as_os_str().to_string_lossy()).collect::<Vec<_>>().join("/"));
        }
    }
}

/// The scanned list must be the whole crate, module folders included: a new module would otherwise escape the scans.
fn assert_sources_complete() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut found = Vec::new();
    rust_files(&dir, &dir, &mut found);
    found.sort();
    let listed: Vec<String> = SOURCES.iter().map(|(n, _)| n.to_string()).collect();
    assert_eq!(found, listed, "the scans must list every source file of the crate");
}

fn review_digest() -> String {
    c600_digest::sha256_hex(b"review record: a recorded model review")
}

/// Every citation EVD.64 names at once: a commit-pinned record and a digested review record.
fn full_citation() -> Citation {
    Citation::theorem("stage 16 PROOFS.md", COMMIT).with_review(&review_digest())
}

fn one_target() -> (Graph, Key) {
    let t = key("T", Covers::OneInstance);
    let mut g = Graph::new();
    g.add_statement(finite("T")).unwrap();
    g.root(&t);
    (g, t)
}

/// A refused write: its code, and the graph unchanged.
fn refused(writer: Writer, grade: &Grade, citation: &Citation) -> &'static str {
    let (mut g, t) = one_target();
    let before = g.clone();
    let err =
        record(&mut g, writer, &t, Leaf::new(&t, grade.clone()), citation).expect_err("the write must be refused");
    assert_eq!(g, before, "a refused write adds no leaf");
    err.code
}

/// A permitted write: one new leaf under the target's OR node, stored as offered.
fn passes(writer: Writer, grade: &Grade, citation: &Citation) {
    let (mut g, t) = one_target();
    let before = g.len();
    let leaf = Leaf::new(&t, grade.clone());
    let id = record(&mut g, writer, &t, leaf.clone(), citation).expect("the write must pass the writer check");
    assert_eq!(g.len(), before + 1);
    assert_eq!(g.leaf(id), Some(&leaf));
}

// --- CT-EVD-17 ------------------------------------------------------------------------------------------------

/// CT-EVD-17 (part: the four rule functions with their limits; not run: the rules applied by the label-table importer
/// to the rows of the corpus, EVD.48 to EVD.51, which this crate does not implement): EVD.52. V14 maps a historical
/// `*-independent*` reviewer record to model review with a fresh audit and not a program checker; a mode read from a key
/// name creates no transport edge; K1 without a covering check reads none; an open reach(Z) row discharges nothing.
#[test]
fn ct_evd_17_import_rules_hold_with_their_limits() {
    let e = ed();

    // V14, and its limit: reviewer records only, never independent checker.
    assert_eq!(
        v14(&HistoricalCheck::Reviewer("stage17-independent-review".into())),
        Some((Check::ModelReview, CheckMode::FreshExactAudit))
    );
    assert_eq!(v14(&HistoricalCheck::Program("replay_word.py".into())), None);
    assert_eq!(v14(&HistoricalCheck::Program("replay_word-independent.py".into())), None);
    assert_eq!(v14(&HistoricalCheck::Reviewer("stage17-review".into())), None);

    // P13: a mode set from a key's name. Its limit: no transport edge. The vocabulary is a draft (crate docs); the
    // limit holds for every mode the draft can set.
    let x = poses("X");
    let z = poses("Z");
    let sx = Key::new(sid("S"), scope_in(Covers::OneInstance, x.clone()));
    let sz = Key::new(sid("S"), scope_in(Covers::OneInstance, z.clone()));
    for name in ["replay_checked", "reused_transport", "verified"] {
        let fact = p13_check_fact(name, "tool:checker").expect("the draft vocabulary sets a mode for this name");
        assert_eq!(fact.transport, None, "{name}: a mode from a key name creates no transport edge");
        let mut g = Graph::new();
        g.add_statement(finite("S")).unwrap();
        let plain = Leaf::new(&sx, exact()).covered(Covered::new(Covers::OneInstance, Some(x.clone())));
        let mut g_plain = g.clone();
        offer(&mut g_plain, &sx, plain.clone()).unwrap();
        let id = offer(&mut g, &sx, plain.clone().check_fact(fact)).unwrap();
        // No word reads the mode (EVD.4).
        assert_eq!(word(&g, &sx, &e), word(&g_plain, &sx, &e));
        assert_eq!(row(&g, &sx, &e), 2);
        // The leaf about X still reaches Z only as AND(source, transport lemma): offered for Z it is refused.
        let mut moved = g.leaf(id).cloned().unwrap();
        moved.key = sz.clone();
        let before = g.clone();
        assert_eq!(offer(&mut g, &sz, moved).unwrap_err().code, SCOPE_MISMATCH);
        assert_eq!(g, before);
        assert!(g.nodes().iter().all(|n| !matches!(n, Node::And(_))), "{name}: no AND node, so no reuse edge");
        assert_eq!(row(&g, &sz, &e), 7);
    }

    // K1: the enclosing record's check covers the row, or the check reads none.
    assert_eq!(k1_check(None, Some((&Check::ModelReview, true))), Check::ModelReview);
    assert_eq!(k1_check(None, Some((&Check::ModelReview, false))), Check::None);
    assert_eq!(k1_check(None, None), Check::None);
    assert_eq!(k1_check(Some(&Check::Producer), Some((&Check::ModelReview, false))), Check::Producer);
    // A check that reads none lies outside the trial threshold (EVD.29: k != none), so it moves no word.
    let t = key("T", Covers::OneInstance);
    let mut g = Graph::new();
    g.add_statement(finite("T")).unwrap();
    let k = k1_check(None, Some((&Check::ModelReview, false)));
    g.support(&t, Leaf::new(&t, gr(Basis::ExactComputation, k)));
    assert_eq!(row(&g, &t, &e), 7);

    // H1: a reachability row is open on one finite instance, as a claim of its own, and adds no OR child.
    let reach_z = Key::new(sid("reach(Z)"), scope_in(Covers::OneInstance, z.clone()));
    let mut g = Graph::new();
    let root = h1_reachability_row(&mut g, Statement::new(sid("reach(Z)")), &reach_z).unwrap();
    match g.node(root) {
        Some(Node::Or(or)) => assert!(or.children.is_empty(), "an open reach row adds no OR child"),
        other => panic!("expected the OR node of reach(Z), got {other:?}"),
    }
    let w = word(&g, &reach_z, &e).unwrap();
    assert_eq!((w.row, w.claim), (7, Claim::Open));
    // It discharges nothing: T(Z) resting on the premise reach(Z) stays conditional on it, and a requested discharge
    // of reach(Z) is refused.
    let tz = Key::new(sid("T(Z)"), scope_in(Covers::OneInstance, z.clone()));
    g.add_statement(finite("T(Z)")).unwrap();
    let thm = g.add_leaf(Leaf::new(&tz, th()));
    let p = g.add_premise(&reach_z, PremiseClass::ReachabilityPromise);
    let and = g.add_and(&[thm, p]).unwrap();
    g.add_or_child(&tz, and).unwrap();
    let w = word(&g, &tz, &e).unwrap();
    assert_eq!((w.row, w.claim), (4, Claim::Conditional));
    assert_eq!(w.premises, vec![reach_z.clone()]);
    assert_eq!(discharge(&g, &reach_z, &reach_z, &e).unwrap_err().code, PREMISE_UNMET);
    // Draft: H1 refuses a row whose scope is not one instance.
    let wide = key("reach(D)", Covers::admitted_domain("D[7]", ""));
    let mut g = Graph::new();
    assert_eq!(
        h1_reachability_row(&mut g, Statement::new(sid("reach(D)")), &wide).unwrap_err().code,
        INCONSISTENT_FIELDS
    );
    assert!(g.is_empty());
}

// --- CT-EVD-18 ------------------------------------------------------------------------------------------------

/// CT-EVD-18 (whole): EVD.4 and EVD.18. A claim about input X offered for input Z is refused with
/// `EVD:evidence.scope_mismatch`; reused evidence appears as AND(source leaf, transport lemma leaf) under Z's OR node,
/// and the transport along a gauge names its checked relation Z = X·γ, after which I7 (a) compares the source leaf.
#[test]
fn ct_evd_18_reuse_is_visible_and_names_its_relation() {
    let e = ed();
    let x = poses("X");
    let z = poses("Z = X.gamma");
    let sx = Key::new(sid("S"), scope_in(Covers::OneInstance, x.clone()));
    let sz = Key::new(sid("S"), scope_in(Covers::OneInstance, z.clone()));
    let mut g = Graph::new();
    g.add_statement(finite("S")).unwrap();
    let source = Leaf::new(&sx, exact()).covered(Covered::new(Covers::OneInstance, Some(x.clone())));
    offer(&mut g, &sx, source.clone()).unwrap();

    // Offered elsewhere, as recorded or rekeyed, the claim is refused and the graph is unchanged.
    let before = g.clone();
    assert_eq!(offer(&mut g, &sz, source.clone()).unwrap_err().code, SCOPE_MISMATCH);
    let mut rekeyed = source.clone();
    rekeyed.key = sz.clone();
    assert_eq!(offer(&mut g, &sz, rekeyed).unwrap_err().code, SCOPE_MISMATCH);
    assert_eq!(g, before);
    assert_eq!(row(&g, &sz, &e), 7);

    // A transport lemma without its relation, or with a relation that does not map X to Z, is refused.
    assert_eq!(reuse(&mut g, &sz, source.clone(), Leaf::new(&sz, th())).unwrap_err().code, SCOPE_MISMATCH);
    let wrong = Relation { from: poses("Y"), to: z.clone(), gauge: "gamma".into() };
    assert_eq!(
        reuse(&mut g, &sz, source.clone(), Leaf::new(&sz, th()).transport(wrong)).unwrap_err().code,
        SCOPE_MISMATCH
    );
    assert_eq!(g, before);

    // Exact transport along the gauge: AND(source leaf, transport leaf), the relation named on the lemma.
    let rel = Relation { from: x.clone(), to: z.clone(), gauge: "gamma".into() };
    let and = reuse(&mut g, &sz, source.clone(), Leaf::new(&sz, th()).transport(rel.clone())).unwrap();
    match g.node(and) {
        Some(Node::And(children)) => {
            assert_eq!(children.len(), 2);
            assert_eq!(g.leaf(children[0]), Some(&source), "the source leaf is kept as recorded");
            assert_eq!(g.leaf(children[1]).map(|l| &l.role), Some(&LeafRole::Transport(rel.clone())));
        }
        other => panic!("expected AND(source, transport), got {other:?}"),
    }
    match g.node(g.root_of(&sz).unwrap()) {
        Some(Node::Or(or)) => assert_eq!(or.children, vec![and], "the reuse is one new OR child of Z"),
        other => panic!("expected Z's OR node, got {other:?}"),
    }
    let w = word(&g, &sz, &e).unwrap();
    assert_eq!((w.row, w.claim), (2, Claim::Proved));
    assert_eq!(row(&g, &sx, &e), 2);

    // I7 (a) compares a reused source leaf after the checked relation (EVD.40): an exhaustive negative over F at X
    // covers the universal's scope at Z once transported, and the same leaf stored directly for Z reads row 0.
    let f = Covers::search_set("R", "retained letters", "three letters");
    let ux = Key::new(sid("U"), scope_in(f.clone(), x.clone()));
    let uz = Key::new(sid("U"), scope_in(f.clone(), z.clone()));
    let neg = Leaf::new(&ux, exhaustive()).covered(Covered::new(f.clone(), Some(x.clone()))).search(SearchRecord::new(
        &sid("U"),
        &f,
        Complete::Yes,
        Stop::Closed,
        JobOutcome::Completed,
    ));
    let mut g = Graph::new();
    g.add_statement(universal("U")).unwrap();
    offer(&mut g, &ux, neg.clone()).unwrap();
    reuse(&mut g, &uz, neg.clone(), Leaf::new(&uz, th()).transport(rel.clone())).unwrap();
    assert_eq!(row(&g, &uz, &e), 2);
    let mut stored = Graph::new();
    stored.add_statement(universal("U")).unwrap();
    stored.support(&uz, neg.clone());
    let err = word(&stored, &uz, &e).unwrap_err();
    assert_eq!(err.code, INCONSISTENT_FIELDS);
    assert!(err.has(Inconsistency::I7));
}

// --- CT-EVD-19 ------------------------------------------------------------------------------------------------

/// CT-EVD-19 (whole): EVD.31 and EVD.46. A `CheckReport` pass adds one leaf, basis certificate and check independent
/// checker, carrying the checker's source digest and the certificate's byte digest; a fail removes it from val and
/// refutes nothing, so the claim reads as it would without it; `not_applicable` adds no leaf.
#[test]
fn ct_evd_19_check_reports_enter_as_one_leaf() {
    let e = ed();
    let x = poses("start pose");
    let s = Key::new(sid("S"), scope_in(Covers::OneInstance, x.clone()));
    let n = Key::new(sid("N"), scope_in(Covers::OneInstance, x.clone()));
    let base = || {
        let mut g = Graph::new();
        pair(&mut g, finite("S"), finite("N"));
        g.root(&s);
        g.root(&n);
        g
    };
    let checker = c600_digest::sha256_hex(b"checker source");
    let certificate = c600_digest::sha256_hex(b"certificate bytes");
    let report = |verdict: Verdict| CheckReport {
        verdict,
        checker_id: "kn6:macro-certificate".into(),
        checker_source: checker.clone(),
        certificate: certificate.clone(),
        stated_inputs: Covered::new(Covers::OneInstance, Some(x.clone())),
        complete: true,
    };

    // Pass: exactly one leaf, with its grade and digests, as one OR child.
    let mut g = base();
    let len = g.len();
    let id = admit_check_report(&mut g, &s, &report(Verdict::Pass)).unwrap().expect("a pass adds a leaf");
    assert_eq!(g.len(), len + 1);
    let leaf = g.leaf(id).unwrap();
    assert_eq!(leaf.grade, gr(Basis::Certificate, Check::IndependentChecker));
    assert!(leaf.support.contains(&checker), "the checker's source digest");
    assert!(leaf.support.contains(&certificate), "the certificate's byte digest");
    assert_eq!(leaf.covered, Some(Covered::new(Covers::OneInstance, Some(x.clone()))));
    match g.node(g.root_of(&s).unwrap()) {
        Some(Node::Or(or)) => assert_eq!(or.children, vec![id]),
        other => panic!("expected S's OR node, got {other:?}"),
    }
    assert_eq!(row(&g, &s, &e), 2);

    // Fail: the claim and its negation read as they would without that support, and no scenario lets it hold.
    let without = base();
    let mut g = base();
    let fail = Verdict::Fail { claim: "end pose".into(), reason: "digest mismatch".into() };
    admit_check_report(&mut g, &s, &report(fail.clone())).unwrap();
    assert_eq!(word(&g, &s, &e), word(&without, &s, &e));
    assert_eq!(word(&g, &n, &e), word(&without, &n, &e));
    assert_eq!(row(&g, &s, &e), 7);
    assert_eq!(row(&g, &n, &e), 7, "a failed check refutes nothing");
    assert!(!val_at(&g, &s, &e, &BTreeSet::new(), &BTreeSet::new()).unwrap(), "the failed leaf is out of val");
    // Beside another argument, a fail removes only its own leaf.
    let mut g = base();
    g.support(&s, Leaf::new(&s, th()));
    let mut g_fail = g.clone();
    admit_check_report(&mut g_fail, &s, &report(fail)).unwrap();
    assert_eq!(word(&g_fail, &s, &e), word(&g, &s, &e));

    // Not applicable: no leaf.
    let mut g = base();
    let before = g.clone();
    assert_eq!(
        admit_check_report(&mut g, &s, &report(Verdict::NotApplicable { reason: "kind".into() })).unwrap(),
        None
    );
    assert_eq!(g, before);
}

// --- CT-EVD-20 ------------------------------------------------------------------------------------------------

/// CT-EVD-20 (whole): EVD.14 and EVD.54. The word function, the passes and the summaries read the graph and a pinned
/// edition and change neither, give the same result on every call, and the crate names no file, clock, process,
/// network, environment, thread or shared mutable state; its only dependency is the digest crate. Changing an edition
/// changes words only through that edition's tables.
#[test]
fn ct_evd_20_words_are_pure_and_edition_bound() {
    assert_sources_complete();
    for (name, text) in SOURCES {
        for banned in [
            "std::fs",
            "std::io",
            "std::net",
            "std::process",
            "std::env",
            "std::time",
            "std::thread",
            "std::sync",
            "SystemTime",
            "Instant::",
            "File::",
            "Command::",
            "TcpStream",
            "env!(",
            "env::",
            "print!(",
            "println!(",
            "eprintln!(",
            "static mut",
            "thread_local!",
            "RefCell",
            "Cell<",
            "unsafe {",
            "unsafe fn",
        ] {
            assert!(!text.contains(banned), "{name} names {banned:?}");
        }
    }
    let manifest = include_str!("../Cargo.toml");
    let deps: Vec<&str> = manifest
        .split("[dependencies]")
        .nth(1)
        .unwrap_or("")
        .lines()
        .take_while(|l| !l.trim_start().starts_with('['))
        .filter_map(|l| l.split('=').next().map(str::trim).filter(|n| !n.is_empty() && !n.starts_with('#')))
        .collect();
    assert_eq!(deps, vec!["c600-digest"]);
    assert!(!manifest.contains("[build-dependencies]") && !manifest.contains("build ="), "no build script");
    assert!(!std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("build.rs").exists(), "no build script");

    // A graph whose keys read row 3, row 2 and row 4 under the trial edition.
    let e = ed();
    let d7 = Covers::admitted_domain("D[7]", "k = 7");
    let u = key("U", d7.clone());
    let t = key("T", Covers::OneInstance);
    let c = key("C", Covers::OneInstance);
    let p = key("P", Covers::OneInstance);
    let mut g = Graph::new();
    g.add_statement(universal("U")).unwrap();
    g.add_statement(finite("T")).unwrap();
    g.add_statement(finite("C")).unwrap();
    g.add_statement(finite("P")).unwrap();
    g.support(&u, Leaf::new(&u, th()));
    g.support(&t, Leaf::new(&t, exact()));
    let l = g.add_leaf(Leaf::new(&c, th()));
    let pr = g.add_premise(&p, PremiseClass::OpenHypothesis);
    let and = g.add_and(&[l, pr]).unwrap();
    g.add_or_child(&c, and).unwrap();
    let keys = [u.clone(), t.clone(), c.clone(), p.clone()];
    let rows: Vec<u8> = keys.iter().map(|k| row(&g, k, &e)).collect();
    assert_eq!(rows, vec![3, 2, 4, 7]);

    // No effect on the graph or the edition, and the same result on every call.
    let g0 = g.clone();
    let e0 = e.clone();
    let first: Vec<_> = keys.iter().map(|k| (word(&g, k, &e), standing(&g, k, &e), summary(&g, k, &e))).collect();
    for _ in 0..3 {
        let again: Vec<_> = keys.iter().map(|k| (word(&g, k, &e), standing(&g, k, &e), summary(&g, k, &e))).collect();
        assert_eq!(again, first);
    }
    assert_eq!(g, g0);
    assert_eq!(e, e0);

    // The edition's name and its import-table pin are no table a word reads: no word changes.
    let renamed = e.with_name("evidence-draft-1, renamed").with_import_table("another pin");
    for k in &keys {
        assert_eq!(word(&g, k, &renamed), word(&g, k, &e));
    }
    // A threshold that requires a check above producer changes exactly the words resting on producer checks.
    let later: BTreeSet<Grade> = u0().into_iter().filter(|gr| gr.check != Check::Producer).collect();
    let raised = e.with_threshold(&later).expect("an up-set inside U0");
    let rows: Vec<u8> = keys.iter().map(|k| row(&g, k, &raised)).collect();
    assert_eq!(rows, vec![3, 7, 4, 7]);
    // Listing (exact computation, producer) as an invalid pair makes T's record read I1 and leaves the others alone.
    let invalid = e.with_invalid_pairs(&[exact()]);
    let err = word(&g, &t, &invalid).unwrap_err();
    assert_eq!(err.code, INCONSISTENT_FIELDS);
    assert!(err.has(Inconsistency::I1));
    for k in [&u, &c, &p] {
        assert_eq!(word(&g, k, &invalid), word(&g, k, &e));
    }
    assert_eq!(g, g0);
}

// --- CT-EVD-21 ------------------------------------------------------------------------------------------------

/// The parameter list of every `fn` in a source text: from the name to the return arrow or the body.
fn fn_parameters(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    for (i, _) in text.match_indices("fn ") {
        let rest = &text[i..];
        let end = [rest.find("->"), rest.find('{'), rest.find(';')].into_iter().flatten().min().unwrap_or(rest.len());
        out.push(rest[..end].to_string());
    }
    out
}

/// CT-EVD-21 (part: the evidence library's half; the policy, grants and admission run in `c600-run/tests/tables.rs`,
/// and the commit paths in `c600-command/tests/kn5_stages.rs` and `c600-session/tests/commit.rs`): EVD.56 and EVD.57. No function of this crate
/// takes a word or a claim, neither converts to a boolean, no function grants, permits, commits or issues a token, and
/// the `record` sink decides a write the same way whatever the target's word.
#[test]
fn ct_evd_21_no_path_reads_a_word() {
    assert_sources_complete();
    // The scan is not vacuous: it sees the crate's functions and would catch a function that reads a word.
    assert!(SOURCES.iter().map(|(_, t)| fn_parameters(t).len()).sum::<usize>() > 50);
    assert!(fn_parameters("pub fn allow_preview(w: &Word) -> bool { true }")[0].contains(": &Word"));
    for (name, text) in SOURCES {
        for banned in ["for bool", "PartialEq<bool>", "Into<bool>", "Not for"] {
            assert!(!text.contains(banned), "{name}: a word or claim must not convert to a boolean ({banned:?})");
        }
        for banned in ["fn grant", "fn permit", "fn commit", "fn token", "fn preview", "fn capability", "fn authori"] {
            assert!(!text.contains(banned), "{name} defines {banned:?}");
        }
        for params in fn_parameters(text) {
            for ty in ["Word", "WordKind", "Claim"] {
                let reads = params.contains(&format!(": {ty}"))
                    || params.contains(&format!(": &{ty}"))
                    || params.contains(&format!("<{ty}>"));
                assert!(!reads, "{name}: {params:?} takes a {ty}");
            }
        }
    }

    // The record sink: the same outcome for every writer, grade and citation whether T reads open, finite instance
    // or refuted.
    let e = ed();
    let t = key("T", Covers::OneInstance);
    let n = key("not T", Covers::OneInstance);
    let base = || {
        let mut g = Graph::new();
        pair(&mut g, finite("T"), finite("not T"));
        g.root(&t);
        g.root(&n);
        g
    };
    let open = base();
    let mut proved = base();
    proved.support(&t, Leaf::new(&t, exact()));
    let mut refuted = base();
    refuted.support(&n, Leaf::new(&n, exact()));
    let mut outcomes = Vec::new();
    for (g, expected_row) in [(&open, 7), (&proved, 2), (&refuted, 1)] {
        assert_eq!(row(g, &t, &e), expected_row);
        let mut these = Vec::new();
        for writer in WRITERS {
            for grade in all_grades() {
                for citation in [Citation::none(), full_citation()] {
                    let mut h = g.clone();
                    let r = record(&mut h, writer, &t, Leaf::new(&t, grade.clone()), &citation);
                    these.push(r.map(|_| ()).map_err(|err| err.code));
                }
            }
        }
        outcomes.push(these);
    }
    assert_eq!(outcomes[0], outcomes[1]);
    assert_eq!(outcomes[0], outcomes[2]);
}

// --- CT-EVD-26 ------------------------------------------------------------------------------------------------

/// CT-EVD-26 (whole): EVD.64 and EVD.65. A user program writes, through the `record` sink, at most basis exact
/// computation with check producer, in EVD.19's order. Check independent checker, basis theorem, self-asserted
/// certificate or exhaustive-computation bases and model, human or formal checks are refused with
/// `EVD:capability.field_not_writable`, whatever the program cites, and add no leaf; (exact computation, producer)
/// passes.
#[test]
fn ct_evd_26_user_program_writes_at_most_exact_producer() {
    assert_eq!(c600_evidence::error::FIELD_NOT_WRITABLE, FIELD_NOT_WRITABLE);
    let ceiling = gr(Basis::ExactComputation, Check::Producer);
    let mut permitted = Vec::new();
    for grade in all_grades() {
        for citation in [Citation::none(), full_citation()] {
            if t_le(&grade, &ceiling) {
                passes(Writer::UserProgram, &grade, &citation);
            } else {
                assert_eq!(refused(Writer::UserProgram, &grade, &citation), FIELD_NOT_WRITABLE, "{grade:?}");
            }
        }
        if t_le(&grade, &ceiling) {
            permitted.push(grade);
        }
    }
    // The four grades at most (exact computation, producer): sample or exact computation, none or producer.
    assert_eq!(permitted.len(), 4);
    for named in [
        gr(Basis::ExactComputation, Check::IndependentChecker),
        gr(Basis::Theorem, Check::Producer),
        gr(Basis::Certificate, Check::Producer),
        gr(Basis::ExhaustiveComputation, Check::Producer),
        gr(Basis::ExactComputation, Check::ModelReview),
        gr(Basis::ExactComputation, Check::HumanReview),
        gr(Basis::ExactComputation, Check::FormalProof),
    ] {
        assert_eq!(refused(Writer::UserProgram, &named, &full_citation()), FIELD_NOT_WRITABLE, "{named:?}");
    }
    passes(Writer::UserProgram, &ceiling, &Citation::none());
    // A membership check is a check the program writes too.
    let (mut g, t) = one_target();
    let before = g.clone();
    let leaf = Leaf::new(&t, ceiling.clone()).member_of(&t, "t", gr(Basis::ExactComputation, Check::ModelReview));
    assert_eq!(record(&mut g, Writer::UserProgram, &t, leaf, &full_citation()).unwrap_err().code, FIELD_NOT_WRITABLE);
    assert_eq!(g, before);
}

// --- CT-EVD-27 ------------------------------------------------------------------------------------------------

/// CT-EVD-27 (whole): EVD.64. The host writes basis theorem only when it cites a record pinned to a commit: no cited
/// record, a record with no commit, or a pin that is no commit is refused with `EVD:capability.field_not_writable`,
/// and a cited commit-pinned record passes the writer check.
#[test]
fn ct_evd_27_host_theorem_cites_a_commit_pinned_record() {
    for check in [Check::None, Check::Producer] {
        let grade = gr(Basis::Theorem, check);
        assert_eq!(refused(Writer::Host, &grade, &Citation::none()), FIELD_NOT_WRITABLE);
        let unpinned = Citation { record: Some("stage 16 PROOFS.md".into()), commit: None, review_digest: None };
        assert_eq!(refused(Writer::Host, &grade, &unpinned), FIELD_NOT_WRITABLE);
        let no_record = Citation { record: None, commit: Some(COMMIT.into()), review_digest: None };
        assert_eq!(refused(Writer::Host, &grade, &no_record), FIELD_NOT_WRITABLE);
        // A branch name is no commit. Draft: a commit pin is a Git object name of 40 or 64 lowercase hex digits.
        assert_eq!(refused(Writer::Host, &grade, &Citation::theorem("stage 16 PROOFS.md", "main")), FIELD_NOT_WRITABLE);
        passes(Writer::Host, &grade, &Citation::theorem("stage 16 PROOFS.md", COMMIT));
    }
}

// --- CT-EVD-28 ------------------------------------------------------------------------------------------------

/// CT-EVD-28 (whole): EVD.64. The host writes check model review or human review only when it cites a digested review
/// record; without one the write is refused with `EVD:capability.field_not_writable`, and with one it passes.
#[test]
fn ct_evd_28_host_review_cites_a_digested_record() {
    for check in [Check::ModelReview, Check::HumanReview] {
        let grade = gr(Basis::ExactComputation, check);
        assert_eq!(refused(Writer::Host, &grade, &Citation::none()), FIELD_NOT_WRITABLE);
        // A review named but not digested. Draft: a digested review record is named by its SHA-256.
        assert_eq!(refused(Writer::Host, &grade, &Citation::review("review.md")), FIELD_NOT_WRITABLE);
        passes(Writer::Host, &grade, &Citation::review(&review_digest()));
    }
}

// --- CT-EVD-29 ------------------------------------------------------------------------------------------------

/// CT-EVD-29 (part: the record sink; not run: the obligation export's `file` row, LB5.18, which is `c600-research`'s;
/// this crate's translator, `obligation`, returns text and writes no evidence field): EVD.3, EVD.47 and EVD.64. No writer, an import included, can assert check formal proof in 1.0:
/// every such write gets `EVD:capability.field_not_writable` and adds no leaf, whatever it cites.
#[test]
fn ct_evd_29_formal_proof_has_no_writer() {
    for writer in WRITERS {
        for basis in Basis::all() {
            for citation in [Citation::none(), full_citation()] {
                let grade = gr(basis.clone(), Check::FormalProof);
                assert_eq!(refused(writer, &grade, &citation), FIELD_NOT_WRITABLE, "{writer:?} {basis:?}");
            }
        }
    }
    // K1 keeps a row's own check, and an imported "formal proof" still stops at the sink.
    let k = k1_check(Some(&Check::FormalProof), Some((&Check::ModelReview, true)));
    assert_eq!(refused(Writer::Import, &gr(Basis::Theorem, k), &full_citation()), FIELD_NOT_WRITABLE);
}

// --- CT-EVD-30 ------------------------------------------------------------------------------------------------

/// CT-EVD-30 (part: the library's entry paths; not run: that only a KN6 checker in the kernel produces a
/// `CheckReport`, which needs KN6's checkers and the runtime's capability boundary, LP3): EVD.46, EVD.64 and EVD.65.
/// No writer sets check independent checker at the `record` sink, a user-supplied assertion with the checker's own
/// digests included; a kernel-checked certificate enters through `admit_check_report` with its own basis and check.
#[test]
fn ct_evd_30_independent_checker_comes_from_the_report() {
    for writer in WRITERS {
        for basis in Basis::all() {
            for citation in [Citation::none(), full_citation()] {
                let grade = gr(basis.clone(), Check::IndependentChecker);
                assert_eq!(refused(writer, &grade, &citation), FIELD_NOT_WRITABLE, "{writer:?} {basis:?}");
            }
        }
    }
    let x = poses("start pose");
    let s = Key::new(sid("S"), scope_in(Covers::OneInstance, x.clone()));
    let checker = c600_digest::sha256_hex(b"checker source");
    let certificate = c600_digest::sha256_hex(b"certificate bytes");
    let mut g = Graph::new();
    g.add_statement(finite("S")).unwrap();
    g.root(&s);
    let before = g.clone();
    let forged = Leaf::new(&s, gr(Basis::Certificate, Check::IndependentChecker))
        .covered(Covered::new(Covers::OneInstance, Some(x.clone())))
        .support(&checker)
        .support(&certificate);
    assert_eq!(record(&mut g, Writer::UserProgram, &s, forged, &full_citation()).unwrap_err().code, FIELD_NOT_WRITABLE);
    assert_eq!(g, before);
    let report = CheckReport {
        verdict: Verdict::Pass,
        checker_id: "kn6:macro-certificate".into(),
        checker_source: checker.clone(),
        certificate: certificate.clone(),
        stated_inputs: Covered::new(Covers::OneInstance, Some(x.clone())),
        complete: true,
    };
    let id = admit_check_report(&mut g, &s, &report).unwrap().unwrap();
    assert_eq!(g.leaf(id).map(|l| l.grade.clone()), Some(gr(Basis::Certificate, Check::IndependentChecker)));
    assert_eq!(row(&g, &s, &ed()), 2);
}

// --- CT-LB6-02 ------------------------------------------------------------------------------------------------

/// CT-LB6-02 (part: the evidence half, LB6.3 read through chapter 06; not run: chapter 03's fork operation, module
/// manifest and `PrincipleRecord`, and the inheritance of cost approvals and records, which this crate does not
/// build): a changed module digest or fork receives no inherited evidence pass, and original references remain
/// references that certify nothing about the fork.
///
/// CT-LG5-06 (part: the evidence half of LG5.8: the fork inherits no evidence, the parent's pass offered for the
/// fork's subject is refused, and a citation of the parent stays a citation that checks nothing of the fork; not run:
/// the fork's new digests and its run grants, which `c600-module/tests/conformance_lg5.rs` runs as the other part).
#[test]
fn ct_lb6_02_fork_inherits_no_evidence() {
    let e = ed();
    let original = DigestRef::of_source_bytes(b"tool m: version 1");
    let fork = DigestRef::of_source_bytes(b"tool m: version 1, edited");
    assert_ne!(original, fork);
    let behaves =
        |src: &DigestRef| Key::new(sid("tool m behaves"), scope(Covers::OneInstance).with_sources(vec![src.clone()]));
    let ko = behaves(&original);
    let kf = behaves(&fork);
    assert_ne!(ko, kf, "the module digest is part of the scope, so the fork's claim has its own key");
    let mut g = Graph::new();
    g.add_statement(finite("tool m behaves")).unwrap();
    let pass = Leaf::new(&ko, exact());
    offer(&mut g, &ko, pass.clone()).unwrap();
    assert_eq!(row(&g, &ko, &e), 2);

    // No inherited pass: the fork's key reads open, the original's leaf offered for it is refused, and the
    // original's claim discharges no premise of the fork.
    assert_eq!(row(&g, &kf, &e), 7);
    let before = g.clone();
    assert_eq!(offer(&mut g, &kf, pass.clone()).unwrap_err().code, SCOPE_MISMATCH);
    assert_eq!(g, before);
    assert_eq!(discharge(&g, &kf, &ko, &e).unwrap_err().code, PREMISE_UNMET);

    // References stay references: a fork statement that cites the original gains no edge, no Reach and no word.
    let kr = Key::new(sid("fork of tool m behaves"), scope(Covers::OneInstance).with_sources(vec![fork.clone()]));
    g.add_statement(finite("fork of tool m behaves").reference(&ko)).unwrap();
    g.root(&kr);
    assert_eq!(row(&g, &kr, &e), 7);
    let original_root = g.root_of(&ko).unwrap();
    assert!(!g.reach(&kr).contains(&original_root));
    assert!(!g.reach(&kf).contains(&original_root));
    // Theorem D (a): removing the original's record changes no word of the fork.
    let mut without = Graph::new();
    without.add_statement(finite("tool m behaves")).unwrap();
    without.add_statement(finite("fork of tool m behaves").reference(&ko)).unwrap();
    without.root(&kr);
    assert_eq!(word(&without, &kr, &e), word(&g, &kr, &e));
    assert_eq!(word(&without, &kf, &e), word(&g, &kf, &e));
}

// --- CT-PS1-27 ------------------------------------------------------------------------------------------------

/// CT-PS1-27 (part: the `record` sink that the envelope writer resolves to; the storage round trip of permitted facts
/// is `c600-session`'s `tests/persist_ps1.rs`; not run: the envelope writer's own call of the sink, so a forbidden
/// basis, check or formal proof written through storage keeps EVD.65's code, which the store does not check): a program
/// or an imported record cannot assert a forbidden basis or check or formal proof through the sink, a violation keeps
/// EVD.65's code and adds nothing, and a permitted fact keeps its source and check dependencies.
#[test]
fn ct_ps1_27_record_sink_keeps_writers_and_dependencies() {
    let e = ed();
    let forbidden = [
        (Writer::UserProgram, gr(Basis::Theorem, Check::Producer), full_citation()),
        (Writer::UserProgram, gr(Basis::ExactComputation, Check::IndependentChecker), full_citation()),
        (Writer::UserProgram, gr(Basis::ExactComputation, Check::FormalProof), full_citation()),
        (Writer::Import, gr(Basis::Theorem, Check::Producer), Citation::none()),
        (Writer::Import, gr(Basis::ExactComputation, Check::ModelReview), Citation::none()),
        (Writer::Import, gr(Basis::ExactComputation, Check::FormalProof), full_citation()),
        (Writer::Import, gr(Basis::Certificate, Check::IndependentChecker), full_citation()),
    ];
    for (writer, grade, citation) in &forbidden {
        assert_eq!(refused(*writer, grade, citation), FIELD_NOT_WRITABLE, "{writer:?} {grade:?}");
    }

    // Permitted facts keep their source and check dependencies, field by field, and read the same from a copy.
    let x = poses("start pose");
    let s = Key::new(sid("S"), scope_in(Covers::OneInstance, x.clone()));
    let mut g = Graph::new();
    g.add_statement(finite("S")).unwrap();
    let source = c600_digest::sha256_hex(b"program source bytes");
    let check = CheckFact {
        tool: c600_digest::sha256_hex(b"replay tool"),
        common_base: vec![c600_digest::sha256_hex(b"shared construction code")],
        kind: "replay".into(),
        mode: CheckMode::ActualReplay,
        transport: None,
    };
    let program = Leaf::new(&s, exact())
        .covered(Covered::new(Covers::OneInstance, Some(x.clone())))
        .support(&source)
        .check_fact(check.clone())
        .job(ProducingJob::new("job 1", JobOutcome::Completed));
    let a = record(&mut g, Writer::UserProgram, &s, program.clone(), &Citation::none()).unwrap();
    let imported = Leaf::new(&s, th()).support(&c600_digest::sha256_hex(b"PROOFS.md at the commit"));
    let b = record(&mut g, Writer::Import, &s, imported.clone(), &full_citation()).unwrap();
    assert_eq!(g.leaf(a), Some(&program));
    assert_eq!(g.leaf(b), Some(&imported));
    let copy = g.clone();
    assert_eq!(copy, g);
    assert_eq!(word(&copy, &s, &e), word(&g, &s, &e));
    assert_eq!(row(&g, &s, &e), 2);
}

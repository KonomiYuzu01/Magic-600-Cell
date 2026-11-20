//! CT-RTE-07, the admission leg (RTE.46; LX-3's frozen corpus `lx3-neutral-v2`): MR13, MR14, MR19 and MR20, which
//! `c600-module/tests/ct_rte_07_full.rs` leaves, refused at the built-in column's admission on both executors.
//!
//! In each of these keys a state-derived read decides whether a later preview is emitted at all: an early stop that
//! the state chooses (LX-3's code gap `stop_emission`; the families `stop_selected_preview` and
//! `accumulation_across_runs`). `c600-module`'s interim split refuses a proposal at the preview point when its block
//! may read state. A stop that the state chooses comes before that point, so under the split the runs of MR19 and MR20
//! on the corpus's `bits` [1, 0, 1] would end differently by bit, and the accumulation across runs would read the bits
//! from the endings. Capability separation at the entry (LG3.13, BND.39) refuses the program before its first
//! operation: `c600_check::admit_built_in`, LP2's draft of the built-in column of chapter 07 section 8, which both
//! executors apply to a job that names its class rows (`JobOptions::built_in`). Every run is then refused alike with
//! the corpus's code, whatever the bits, and the host is never called: no row is read and no preview is proposed.
//!
//! The mapping follows the Full leg's (CT-RTE-07's fixture mapping in `c600-module`'s crate documentation): the letters
//! `a`, `b` and `c` are `+1`, `+2` and `+3`, `bits` is the record `{i0, i1, i2}`, and the loop over the corpus's word
//! list is unrolled. The built-in column sets three choices of this leg's own:
//!
//! - The words are the entry's `Word` parameters, given `a`, `b` and `c`: the column previews only a word the solver
//!   named, a parameter of the entry as given (chapter 07 section 8). MR13 and MR14 take their words from the corpus's
//!   manual inputs. MR19 writes `a` and `b` as literals and MR20 writes `b`, which the column refuses besides
//!   separation, with the same code, at each preview (`ct_rte_07_admission_literal_words_add_a_second_finding`).
//! - `score` of a word is the snapshot read `score` with the word as its argument (LP2 draft). The corpus's manual
//!   target `c0` is the literal `target("c0")`: a built-in tool's rows are listed by its class, and a proposal to a
//!   target that is not a literal is a row the checker cannot name (BND.57).
//! - The requested stop is an assertion (`assert`, LP2 draft, DET.55), the IR draft's one way to end a run early. Such
//!   a run would end with `LG4:input.pointwise_failed` where the corpus stops on request; none gets that far.
//!
//! Each key's class lists the rows the key uses, so separation is the admission's only finding: the key with its
//! previews or its reads left out is admitted under the same class, which shows that the class covers them.
//!
//! Each key runs ten times on both executors, each run against fresh sessions; the ten runs give one SHA-256 over the
//! refusals and the sessions. Evidence kind: source and synthetic. The host is `support`'s fake session, not Full's
//! mechanics; nothing here measures time.

mod support;

use c600_check::{Profile, Row};
use c600_digest::sha256;
use c600_run::interp::RowTariff;
use c600_run::meter::code::NOT_GRANTED;
use c600_run::provenance::{self, hex, Json};
use c600_run::value::Value;
use c600_runtime::{Ended, JobOptions, Refused, Stage};
use support::{compare, header, job, runtime, Session, WIDE};

const CORPUS: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/../../work/experiments/language-lx/lx3-sandbox-flow/corpus/");

const BITS: &str = "record { i0: Int, i1: Int, i2: Int }";

/// Which half of a key a line of its body belongs to: a state read and what the read decides, or a proposal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Side {
    Read,
    Preview,
}

use Side::{Preview, Read};

/// A key as this leg writes it: the entry's parameters and arguments, the rows its class lists, and one body per run
/// of the corpus.
struct Key {
    id: &'static str,
    params: String,
    args: Vec<Value>,
    rows: Vec<Row>,
    runs: Vec<Vec<(Side, String)>>,
}

fn line(side: Side, text: &str) -> (Side, String) {
    (side, text.to_string())
}

/// A proposal of `word`, a parameter or a literal, to the corpus's target `c0`.
fn propose(name: &str, word: &str) -> (Side, String) {
    line(Preview, &format!("let {name} = propose({word}, target(\"c0\"));"))
}

/// The entry's `Word` parameters with these names, and their arguments: the letters `+1`, `+2`, ... in order.
fn words(names: &[&str]) -> (String, Vec<Value>) {
    let params = names.iter().map(|n| format!("{n}: Word")).collect::<Vec<_>>().join(", ");
    (params, (1..=names.len() as i32).map(|letter| Value::word(&[letter])).collect())
}

/// The requested stop when `test` holds, written as an assertion that it does not.
fn stop_if(name: &str, test: &str) -> (Side, String) {
    line(Read, &format!("let {name} = assert(not({test}));"))
}

fn read_bits() -> (Side, String) {
    line(Read, &format!("let bits = read_snapshot<{BITS}>(\"bits\");"))
}

/// MR13: each word of the solver's list `a`, `b`, `c` is previewed before its score is read, and a zero score stops
/// the loop, which leaves the word previewed last pending.
fn mr13() -> Key {
    let mut body = Vec::new();
    for name in ["a", "b", "c"] {
        body.push(propose(&format!("r{name}"), &format!("w{name}")));
        body.push(line(Read, &format!("let s{name} = read_snapshot<Int>(\"score\", w{name});")));
        body.push(stop_if(&format!("k{name}"), &format!("exact_eq(s{name}, 0)")));
    }
    let (params, args) = words(&["wa", "wb", "wc"]);
    Key { id: "MR13", params, args, rows: vec![Row::Read("score".into()), Row::Preview("c0".into())], runs: vec![body] }
}

/// MR14: a stop chosen by `bits[0]` decides whether the constant preview of the solver's word `a`, which lies outside
/// the branch, is emitted.
fn mr14() -> Key {
    let (params, args) = words(&["w"]);
    Key {
        id: "MR14",
        params,
        args,
        rows: vec![Row::Read("bits".into()), Row::Preview("c0".into())],
        runs: vec![vec![read_bits(), stop_if("k", "exact_eq(bits.i0, 1)"), propose("r", "w")]],
    }
}

/// MR19 and MR20: run `i` stops when `bits[i]` is 1 and otherwise previews `b`, so the runs together would show the
/// bits; MR19 previews `a` first in every run. The words are the entry's parameters `wa` and `wb`, or with `literal`
/// the corpus's literal words.
fn across_runs(id: &'static str, first: bool, literal: bool) -> Key {
    let (a, b) = if literal { ("word[1]", "word[2]") } else { ("wa", "wb") };
    let run = |i: usize| {
        let mut body = Vec::new();
        if first {
            body.push(propose("ra", a));
        }
        body.push(read_bits());
        body.push(stop_if("k", &format!("exact_eq(bits.i{i}, 1)")));
        body.push(propose("rb", b));
        body
    };
    let (params, args) = if literal { (String::new(), Vec::new()) } else { words(&["wa", "wb"]) };
    Key {
        id,
        params,
        args,
        rows: vec![Row::Read("bits".into()), Row::Preview("c0".into())],
        runs: (0..3).map(run).collect(),
    }
}

/// The source of one run, keeping the lines of the sides `keep` names. The declared effects are those of the kept
/// lines.
fn source(key: &Key, body: &[(Side, String)], keep: &[Side]) -> String {
    let kept: Vec<&str> = body.iter().filter(|(s, _)| keep.contains(s)).map(|(_, l)| l.as_str()).collect();
    let mut effects = Vec::new();
    if kept.iter().any(|l| l.contains("read_snapshot")) {
        effects.push("snapshot");
    }
    if kept.iter().any(|l| l.contains("propose")) {
        effects.push("preview");
    }
    let mut text =
        format!("{}export command go({}) -> Text effects [{}] {{\n", header(""), key.params, effects.join(", "));
    for l in kept {
        text.push_str(&format!("    {l}\n"));
    }
    text.push_str("    return \"requested\";\n}\n");
    text
}

/// The built-in column with the key's class rows, in the solving profile.
fn options(key: &Key) -> JobOptions {
    JobOptions {
        profile: Profile::Solving,
        built_in: Some(key.rows.iter().cloned().collect()),
        ..JobOptions::new(WIDE)
    }
}

/// A session that also answers the corpus's rows `bits` ([1, 0, 1]) and `score` (1 for every word), which only the
/// controls read.
fn fixture(session: &mut Session) {
    let tariff = RowTariff { kernel_cost: 3, result_bound: 4096 };
    let bits = Value::record(vec![("i0", Value::int(1)), ("i1", Value::int(0)), ("i2", Value::int(1))]);
    session.snapshots.push(("bits", tariff, bits));
    session.snapshots.push(("score", tariff, Value::int(1)));
}

/// The corpus's case of a key in `must-refuse.json`.
fn case(id: &str) -> Json {
    let corpus = provenance::parse(&std::fs::read(format!("{CORPUS}must-refuse.json")).unwrap()).expect("it reads");
    let Some(Json::List(cases)) = corpus.get("cases") else { panic!("the corpus's cases") };
    cases.iter().find(|c| c.get("id").and_then(Json::as_text) == Some(id)).expect("the key").clone()
}

/// The index constant of each run's `index` operation, if it has one.
fn indices(case: &Json) -> Vec<Option<i64>> {
    let Some(Json::List(runs)) = case.get("program").and_then(|p| p.get("runs")) else { panic!("the runs") };
    let index = |run: &Json| {
        let Some(Json::List(body)) = run.get("body") else { panic!("a body") };
        body.iter().find(|op| op.get("fn").and_then(Json::as_text) == Some("index")).map(|op| match op.get("args") {
            Some(Json::List(args)) => match args[1].get("value") {
                Some(Json::Int(n)) => *n,
                other => panic!("{other:?}"),
            },
            other => panic!("{other:?}"),
        })
    };
    runs.iter().map(index).collect()
}

/// Ten runs of each of the key's runs on both executors: each is refused at the admission stage with the corpus's code
/// and the one separation finding, alike in every run, and the host is never called. Returns the refusal.
fn refused_at_admission(key: &Key, expected: &str) -> Refused {
    let mut rt = runtime();
    let fresh = Session::default();
    let mut digests = Vec::new();
    let mut seen: Vec<Refused> = Vec::new();
    for _ in 0..10 {
        let mut round = Vec::new();
        for body in &key.runs {
            let source = source(key, body, &[Read, Preview]);
            let (end, session) = compare(&mut rt, &job(&source, "go", key.args.clone(), options(key)), fixture);
            let Ended::Refused(refused) = end else { panic!("{}: admitted: {end:#?}", key.id) };
            assert_eq!((refused.stage, refused.codes.as_slice()), (Stage::Admit, [expected.to_string()].as_slice()));
            assert!(refused.detail.contains("capability separation"), "{}", refused.detail);
            assert!(session.log.is_empty(), "{}: the host was called: {:?}", key.id, session.log);
            assert!(session.proposals.is_empty() && session.views.is_empty() && session.workspace.is_empty());
            assert_eq!((&session.journal, &session.preview), (&fresh.journal, &fresh.preview));
            round.push(format!("{refused:?} {:?} {:?} {:?}", session.log, session.journal, session.preview));
            seen.push(refused);
        }
        digests.push(hex(&sha256(round.join("\n").as_bytes())));
    }
    assert!(digests.iter().all(|d| *d == digests[0]), "{}: {digests:?}", key.id);
    assert!(seen.iter().all(|r| *r == seen[0]), "{}: the runs are refused alike", key.id);
    assert_eq!(rt.spawns(), 1);
    seen.swap_remove(0)
}

/// The key with its previews left out, and with its reads left out, is admitted under the same class on both
/// executors: the class covers every row the key uses. The read half reads its rows; with the fixture's bits its
/// assertion may end it early, which is still a run. The preview half proposes each of its words.
fn halves_are_admitted(key: &Key) {
    let mut rt = runtime();
    for body in &key.runs {
        let reads = source(key, body, &[Read]);
        let (end, session) = compare(&mut rt, &job(&reads, "go", key.args.clone(), options(key)), fixture);
        assert!(matches!(end, Ended::Ran(_)), "{}: {end:#?}", key.id);
        assert!(!session.log.is_empty() && session.proposals.is_empty(), "{}: {:?}", key.id, session.log);

        let previews = source(key, body, &[Preview]);
        let (end, session) = compare(&mut rt, &job(&previews, "go", key.args.clone(), options(key)), fixture);
        assert!(matches!(end, Ended::Ran(_)), "{}: {end:#?}", key.id);
        let proposed = body.iter().filter(|(s, _)| *s == Preview).count();
        assert_eq!(session.proposals.len(), proposed, "{}", key.id);
    }
}

/// The corpus's case: its code, separation's expected catch and its runs' index constants, which the key's runs use.
fn corpus_code(key: &Key, expected_indices: &[Option<i64>]) -> String {
    let case = case(key.id);
    assert_eq!(indices(&case), expected_indices, "{}: the corpus's runs", key.id);
    assert_eq!(key.runs.len(), expected_indices.len());
    let catch = case.get("forms").and_then(|f| f.get("capability_separation")).and_then(|s| s.get("expected_to_catch"));
    assert_eq!(catch, Some(&Json::Bool(true)), "{}", key.id);
    let code = case.get("expected_code").and_then(Json::as_text).expect("its code").to_string();
    assert_eq!(code, NOT_GRANTED);
    code
}

fn realised(key: Key, expected_indices: &[Option<i64>]) {
    let code = corpus_code(&key, expected_indices);
    let refused = refused_at_admission(&key, &code);
    assert!(refused.detail.contains("m.go at entry"), "{}", refused.detail);
    halves_are_admitted(&key);
}

#[test]
fn ct_rte_07_admission_mr13_scores_stop_the_loop() {
    realised(mr13(), &[None]);
}

#[test]
fn ct_rte_07_admission_mr14_stop_before_a_constant_preview() {
    realised(mr14(), &[Some(0)]);
}

#[test]
fn ct_rte_07_admission_mr19_accumulation_after_a_preview() {
    realised(across_runs("MR19", true, false), &[Some(0), Some(1), Some(2)]);
}

#[test]
fn ct_rte_07_admission_mr20_accumulation_across_runs() {
    realised(across_runs("MR20", false, false), &[Some(0), Some(1), Some(2)]);
}

/// MR19 and MR20 with the corpus's literal words: every run is refused alike at admission with the corpus's code, on
/// two kinds of finding, separation's at `entry` and one at each preview of a word the solver did not name; the host
/// is never called.
#[test]
fn ct_rte_07_admission_literal_words_add_a_second_finding() {
    let mut rt = runtime();
    for (id, first) in [("MR19", true), ("MR20", false)] {
        let key = across_runs(id, first, true);
        let previews = if first { 2 } else { 1 };
        let mut seen = Vec::new();
        for body in &key.runs {
            let source = source(&key, body, &[Read, Preview]);
            let (end, session) = compare(&mut rt, &job(&source, "go", Vec::new(), options(&key)), fixture);
            let Ended::Refused(refused) = end else { panic!("{id}: admitted: {end:#?}") };
            assert_eq!((refused.stage, refused.codes.len()), (Stage::Admit, 1 + previews), "{id}");
            assert!(refused.codes.iter().all(|c| c == NOT_GRANTED), "{id}: {:?}", refused.codes);
            assert!(refused.detail.contains("capability separation"), "{}", refused.detail);
            assert!(refused.detail.contains("m.go at entry"), "{}", refused.detail);
            let unnamed = refused.detail.matches("a preview of a word the solver did not name").count();
            assert_eq!(unnamed, previews, "{}", refused.detail);
            assert!(session.log.is_empty() && session.proposals.is_empty(), "{id}: {:?}", session.log);
            seen.push(refused);
        }
        assert!(seen.iter().all(|r| *r == seen[0]), "{id}: the runs are refused alike");
    }
    assert_eq!(rt.spawns(), 1);
}

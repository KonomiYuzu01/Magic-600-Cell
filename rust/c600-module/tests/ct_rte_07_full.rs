//! CT-RTE-07, the 600-cell-Full subset, source leg (RTE.46; LX-3's frozen corpus `lx3-neutral-v2`).
//!
//! Each realised key is re-expressed in `.6cs` under the fixture mapping that the crate documentation records
//! (`Drafts`, CT-RTE-07's fixture mapping), and runs ten times from a fresh fixture: verified, approved through
//! `c600-check`, loaded and run through `c600-run`'s boundary. The ten runs give one SHA-256 over the canonical results
//! and stop reasons, the engine side and the store. An allowed key completes with the corpus's value; a refused key
//! gives the corpus's expected code at the refused operation, installs no preview, keeps the session fields and the
//! existing workspace provenance, and reads no planted data. A priced budget key (B01 to B07) runs under the price
//! mapping that the crate documentation pins (CT-RTE-07's price mapping): it gives the corpus's outcome and code, in
//! the corpus's phase, at the counterpart of the corpus's refused instruction, with the corpus's used amounts mapped
//! through that table. An interruption key (B08, B09) is the solver's cancel, which the synthetic engine raises
//! inside a host row of a `repeat` loop: it gives the corpus's outcome and code at the next operation, and the
//! corpus's examined items are the rows the engine served. A pick key (MR17, MR18, A29, A30) is two runs, because the
//! draft language has no pick inside a run: the first builds the list, the host writes the solver's `c600-pick` record
//! of option 0 (FP.38, BND.54), and the second previews the picked value, whose block records the pick and keeps the
//! list's block in `parts` (BND.23, BND.85).
//! [`FALSE_REFUSALS`] and [`READ_AS_FIXTURE_INVERSES`] list where this leg differs from the corpus; every refused key
//! gives the corpus's code (P01 to P03 since the owner decision of 16 November 2026 under PHS.52). [`LEFT`] lists the
//! keys not realised here, with the reason: B10 and B11 need a runtime process and are realised in `c600-tools`, as
//! are B13 and B14, whose budget the command layer admits on the tool's row; MR13, MR14, MR19 and MR20, which the
//! built-in column's admission refuses, are realised in `c600-runtime`.
//!
//! Evidence kind: source and synthetic. The fixture is LX-3's fake data, not Full's mechanics; nothing here measures
//! time or performance.
//!
//! CT-RTE-07 (source, part: the 600-cell-Full subset's keys except those in `LEFT`, each run ten times; not run here:
//! the keys of `LEFT` other than B12: B10 and B11 in `c600-tools/tests/ct_rte_07_c5.rs`, B13 and B14 in its
//! `ct_rte_07_budget.rs`, MR13, MR14, MR19 and MR20 in `c600-runtime/tests/ct_rte_07_admission.rs`, and the six
//! jumbling keys on M5's rows in `c600-jumble-tool/tests/ct_rte_07_jumble.rs`; not run: B12, whose cold run has no
//! test on this commit and whose other five cache states are not applicable by the owner's decision of 20 November
//! 2026, item 1, which is no pass).

mod support;

use std::collections::BTreeSet;
use std::sync::Arc;

use c600_digest::sha256;
use c600_ir::{Callee, Expr, Lit, QTy, Ty};
use c600_module::code::{DIGEST_MISMATCH, NOT_GRANTED};
use c600_module::{approve, load, verify, Closure, Identity, Outcome, Refusal, RunRequest};
use c600_run::boundary::{Answer, Boundary, Context, Place, Store};
use c600_run::interp::{CancelFlag, Options};
use c600_run::meter::{frame_size, Amounts, Outcome as Ended};
use c600_run::provenance::{self, field, hex, Author, Json, Producer, Profile, INCOMPATIBLE};
use c600_run::value::Value;
use support::*;

const CORPUS: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/../../work/experiments/language-lx/lx3-sandbox-flow/corpus/");
/// The raw SHA-256 of the frozen corpus's `FREEZE.json` (`lx3-neutral-v2`).
const FREEZE_SHA256: &str = "d7eff17503ee95d45ddca0efe38ec3bb09418b0248a6279afe2dce0a90ea6627";

/// The keys realised here, one test each.
const REALISED: [&str; 60] = [
    "A01", "A02", "A03", "A04", "A05", "A06", "A07", "A08", "A09", "A10", "A11", "A12", "A13", "A14", "A15", "A16",
    "A17", "A18", "A19", "A20", "A21", "A22", "A23", "A24", "A25", "A26", "A27", "A28", "A29", "A30", "A33", "A34",
    "A35", "B01", "B02", "B03", "B04", "B05", "B06", "B07", "B08", "B09", "MR03", "MR04", "MR07", "MR08", "MR09",
    "MR10", "MR11", "MR12", "MR15", "MR16", "MR17", "MR18", "P01", "P02", "P03", "P04", "S01", "S02",
];

const M5: &str = "a jumbling key: realised on M5's rows in `c600-jumble-tool/tests/ct_rte_07_jumble.rs`";
const RUNTIME: &str = "realised with a runtime process in `c600-tools/tests/ct_rte_07_c5.rs`";
const ADMISSION: &str = "realised as the built-in column's refusal at admission in \
                         `c600-runtime/tests/ct_rte_07_admission.rs`";
const COMMAND: &str = "realised at the command layer on the 600-cell-Full tool's row in \
                       `c600-tools/tests/ct_rte_07_budget.rs`";

/// The keys left, with the reason.
const LEFT: [(&str, &str); 15] = [
    ("MR01", M5),
    ("MR02", M5),
    ("MR05", M5),
    ("MR06", M5),
    ("A31", M5),
    ("A32", M5),
    ("MR13", ADMISSION),
    ("MR14", ADMISSION),
    ("MR19", ADMISSION),
    ("MR20", ADMISSION),
    ("B10", RUNTIME),
    ("B11", RUNTIME),
    ("B12", "not realised: the host's row cache and its variants are not part of this crate"),
    ("B13", COMMAND),
    ("B14", COMMAND),
];

/// Allowed keys this leg refuses. A25 reads the counts and previews a manually named word in one entry; separation
/// refuses the preview, which the corpus allows ("Separation may falsely refuse the combination").
const FALSE_REFUSALS: [&str; 1] = ["A25"];

/// Allowed keys whose value has other literal letters than the corpus's and matches it only under the fixture's
/// inverse letters ([`named`]). A17's inverse of `D a` is `-1, +4`, because the IR writes a letter's inverse as its
/// negation; the fixture names `a` its own inverse, so `-1` reads as `a` and the value as the corpus's `a d`, whose
/// literal letters are `+1, +4`.
const READ_AS_FIXTURE_INVERSES: [&str; 1] = ["A17"];

const INTS: &str = "record { c0: Int, c1: Int, c2: Int }";
const BOOLS: &str = "record { c0: Bool, c1: Bool, c2: Bool }";
const STATE: &str = "record { s0: Int, s1: Int, s2: Int, s3: Int }";
const NU: &str = "record { i0: Int, i1: Int }";
const BITS: &str = "record { i0: Int, i1: Int, i2: Int }";
const PAIR: &str = "record { count: Int, target: Target }";
/// The row `word_check` of a word (`kernel-stub.md`).
const CHECK: &str = "record { coverage: record { complete: Bool, letters: Int }, end_labels: $STATE, \
                     moved_slots: List<Int>, protected: Bool, word: Word }";

/// The search of MR03, MR04 and A33, unrolled over the corpus's words `a`, `b`, `c`: their scores and the index of
/// the first minimum.
const SEARCH: &str = "    let sa = read_snapshot<Int>(\"score/a\");
    let sb = read_snapshot<Int>(\"score/b\");
    let sc = read_snapshot<Int>(\"score/c\");
    let i = if(exact_less(sb, sa), if(exact_less(sc, sb), 2, 1), if(exact_less(sc, sa), 2, 0));
";

/// The word at index `i` of the corpus's words `a`, `b`, `c`, rebuilt from literal letters.
const REBUILD: &str = "if(exact_eq(i, 0), word[1], if(exact_eq(i, 1), word[2], word[3]))";

/// A module source with the type and code placeholders filled in.
fn src(name: &str, requires: &[&str], deps: &[(&str, &str)], body: &str) -> String {
    let body = body
        .replace("$CHECK", CHECK)
        .replace("$INTS", INTS)
        .replace("$BOOLS", BOOLS)
        .replace("$STATE", STATE)
        .replace("$NU", NU)
        .replace("$BITS", BITS)
        .replace("$PAIR", PAIR)
        .replace("$SEARCH", SEARCH)
        .replace("$REBUILD", REBUILD);
    module(name, requires, deps, &body)
}

fn root(body: &str) -> String {
    src("m", &[], &[], body)
}

/// One key's runs on one fixture.
struct Trial {
    package: Package,
    engine: Engine,
    store: Store,
    results: Vec<Result<Outcome, Refusal>>,
}

impl Trial {
    fn of(root: &str, deps: &[&str]) -> Trial {
        Trial { package: package(root, deps), engine: Engine::fixture(), store: Store::default(), results: Vec::new() }
    }

    /// Runs `m.<function>` with the corpus's inputs: the target `c0` and the word `a`, both entered by hand.
    fn run(self, function: &str, author: Author, profile: Profile) -> Trial {
        self.run_with(function, author, profile, vec![manual(), manual()])
    }

    fn run_with(self, function: &str, author: Author, profile: Profile, inputs: Vec<Vec<u8>>) -> Trial {
        self.exec(function, author, profile, inputs, WIDE)
    }

    /// Runs `m.<function>` under the given limits.
    fn exec(self, function: &str, author: Author, profile: Profile, inputs: Vec<Vec<u8>>, limits: Amounts) -> Trial {
        self.call(function, author, profile, vec![Value::target("c0"), Value::word(&[A])], inputs, limits)
    }

    /// Runs `m.<function>` with `args`, whose blocks are `inputs`, under the given limits.
    fn call(
        self,
        function: &str,
        author: Author,
        profile: Profile,
        args: Vec<Value>,
        inputs: Vec<Vec<u8>>,
        limits: Amounts,
    ) -> Trial {
        self.call_with(function, author, profile, args, inputs, Options::new(limits))
    }

    /// Runs `m.<function>` with `args`, whose blocks are `inputs`, under the given options.
    fn call_with(
        mut self,
        function: &str,
        author: Author,
        profile: Profile,
        args: Vec<Value>,
        inputs: Vec<Vec<u8>>,
        options: Options<'_>,
    ) -> Trial {
        let id = format!("r{}", self.results.len() + 1);
        let result = ready(&self.package, entry(function, author, profile)).and_then(|r| {
            let request = RunRequest { run: &id, base: "rev-7", args, inputs, pins: pins(), options };
            r.run(request, &mut self.engine, &mut self.store)
        });
        self.results.push(result);
        self
    }

    /// Runs `m.<function>`, an own tool in tool authoring with no arguments, under `limits`. The engine cancels the
    /// run by the solver's request inside the first row `row` it serves; the run observes the cancel before its next
    /// operation.
    fn cancelled_after(mut self, function: &str, row: &str, limits: Amounts) -> Trial {
        let flag = Arc::new(CancelFlag::default());
        self.engine.cancel_after = Some((row.to_string(), flag.clone()));
        let mut options = Options::new(limits);
        options.cancel = Some(&*flag);
        let mut trial = self.call_with(function, Author::Own, Profile::ToolAuthoring, Vec::new(), Vec::new(), options);
        trial.engine.cancel_after = None;
        trial
    }

    /// RTE.46's digest: the canonical results and stop reasons (a refusal by its code), the engine side and the store.
    fn digest(&self) -> [u8; 32] {
        let runs = self.results.iter().map(|r| match r {
            Ok(o) => Json::Text(String::from_utf8_lossy(&o.canonical()).into_owned()),
            Err(e) => Json::object([("refused", Json::text(e.code))]),
        });
        sha256(&provenance::canonical(&Json::object([
            ("engine", Json::Text(hex(&self.engine.digest()))),
            ("runs", Json::List(runs.collect())),
            ("store", store_json(&self.store)),
        ])))
    }

    fn last(&self) -> &Outcome {
        match self.results.last().expect("a run") {
            Ok(o) => o,
            Err(r) => panic!("{r:?}"),
        }
    }
}

/// RTE.46: ten runs of a key, each from a fresh fixture, give one SHA-256.
fn ten(key: impl Fn() -> Trial) -> Trial {
    let first = key();
    let digest = hex(&first.digest());
    for i in 2..=10 {
        assert_eq!(hex(&key().digest()), digest, "run {i} of 10");
    }
    first
}

/// One solving run of a built-in entry `go`, the corpus's usual command.
fn solving(source: &str) -> Trial {
    Trial::of(source, &[]).run("go", Author::BuiltIn, Profile::Solving)
}

/// One solving run of a built-in entry `go` whose arguments are the key's own words, each entered by hand.
fn solving_on(source: &str, words: &[&[i32]]) -> Trial {
    let args = words.iter().map(|w| Value::word(w)).collect();
    Trial::of(source, &[]).call("go", Author::BuiltIn, Profile::Solving, args, vec![manual(); words.len()], WIDE)
}

/// The fixture letter `d`; `D` is its inverse, `-4`.
const D: i32 = 4;

/// The fixture's names of the letters `+1` to `+4`.
const NAMES: [&str; 4] = ["a", "b", "c", "d"];

/// A word's letters as the corpus names them. The IR writes the inverse of a letter as its negation (LG2.15), and
/// `fixture.json` names each letter's inverse (`inverses`): `a`, `b` and `c` are their own inverses and `D` is `d`'s.
/// So `+n` reads as the fixture's letter `n`, and `-n` as the inverse the fixture names for it. It checks the tests'
/// letters against the corpus's names; the runs' outputs are compared as exact bytes.
fn named(letters: &[i32]) -> Vec<String> {
    let fixture = provenance::parse(&std::fs::read(format!("{CORPUS}fixture.json")).unwrap()).expect("it reads");
    let inverses = fixture.get("inverses").expect("the fixture's inverse letters");
    let name = |l: i32| NAMES[l.unsigned_abs() as usize - 1];
    let read = |l: i32| match l > 0 {
        true => name(l),
        false => inverses.get(name(l)).and_then(Json::as_text).expect("an inverse letter"),
    };
    letters.iter().map(|&l| read(l).to_string()).collect()
}

/// No planted data anywhere, no trap call, and the session fields of the fixture.
fn clean(trial: &Trial) {
    assert!(!leaks(&trial.results, &trial.store) && !trial.engine.holds_planted(), "planted data");
    assert_eq!(trial.engine.traps, 0, "a trap was called");
    let fixture = Engine::fixture();
    assert_eq!((trial.engine.revision, &trial.engine.labels), (fixture.revision, &fixture.labels));
}

/// An allowed key: completed with the expected output.
fn completed<'a>(trial: &'a Trial, output: &[u8]) -> &'a Outcome {
    let out = trial.last();
    assert_eq!((out.run.summary.outcome, out.run.summary.code), (Ended::Completed, None), "{:?}", out.run.summary);
    let given = out.run.output.as_deref().map(String::from_utf8_lossy);
    assert_eq!(given.as_deref(), Some(&*String::from_utf8_lossy(output)));
    clean(trial);
    out
}

/// An allowed key that prepares no preview and writes nothing.
fn quiet(trial: &Trial, output: &[u8]) {
    let out = completed(trial, output);
    assert!(out.shell.is_empty() && out.captures.is_empty() && trial.engine.previews.is_empty());
    assert!(trial.store.records.is_empty() && trial.engine.views.is_empty());
}

/// The engine side's digest, views aside, is the fixture's: no preview installed, the pending preview, labels and
/// revision kept, and no trap called.
fn unchanged(trial: &Trial) {
    let mut side = trial.engine.clone();
    side.views.clear();
    assert_eq!(hex(&side.digest()), hex(&Engine::fixture().digest()), "the engine side is unchanged");
}

/// A refused key, refused at its first proposal: the shell answer has the code, the split refused it or not, and
/// the engine installed no preview and kept its pending one.
fn refused_preview(trial: &Trial, code: &str, by_split: bool) {
    let out = trial.last();
    let shown = &out.shell[0];
    assert!(matches!(&shown.answer, Answer::Refused(r) if r.code == code), "{:?}", shown.answer);
    assert_eq!(out.separated.contains(&shown.id), by_split, "{:?}", out.separated);
    assert!(trial.engine.previews.is_empty(), "no forbidden preview is installed");
    assert_eq!(trial.engine.pending, Engine::fixture().pending);
    unchanged(trial);
    clean(trial);
}

/// The corpus's expected code for a refused case: the first `expected_code` after the case's `id`.
fn corpus_code(file: &str, id: &str) -> String {
    let text = std::fs::read_to_string(format!("{CORPUS}{file}")).unwrap();
    let at = text.find(&format!("\"id\": \"{id}\"")).expect("the case");
    let key = "\"expected_code\": \"";
    let start = at + text[at..].find(key).expect("its code") + key.len();
    text[start..start + text[start..].find('"').unwrap()].to_string()
}

fn ints(fields: &[(&str, i64)]) -> Value {
    Value::record(fields.iter().map(|(n, v)| (*n, Value::int(*v))).collect())
}

fn text(s: &str) -> Vec<u8> {
    encoded(&Value::text(s))
}

/// The frozen corpus is the one LX-3 froze, and every key of `keys.json` is realised or listed as left.
#[test]
fn ct_rte_07_full_corpus_is_frozen_and_every_key_is_accounted_for() {
    let read = |name: &str| std::fs::read(format!("{CORPUS}{name}")).unwrap();
    let freeze = read("FREEZE.json");
    assert_eq!(hex(&sha256(&freeze)), FREEZE_SHA256);
    let freeze = provenance::parse(&freeze).expect("FREEZE.json reads");
    let Some(Json::Object(files)) = freeze.get("files") else { panic!("the frozen file list") };
    for (name, digest) in files.iter().filter(|(name, _)| name.as_str() != "FREEZE.json") {
        assert_eq!(Some(hex(&sha256(&read(name))).as_str()), digest.as_text(), "{name}");
    }

    let keys = String::from_utf8(read("keys.json")).unwrap();
    let marker = "\"case_id\": \"";
    let ids: BTreeSet<&str> = keys
        .match_indices(marker)
        .map(|(at, _)| {
            let rest = &keys[at + marker.len()..];
            &rest[..rest.find('"').unwrap()]
        })
        .collect();
    let realised: BTreeSet<&str> = REALISED.into_iter().collect();
    let left: BTreeSet<&str> = LEFT.iter().map(|(k, _)| *k).collect();
    assert_eq!(realised.len() + left.len(), REALISED.len() + LEFT.len(), "no key is listed twice");
    assert!(realised.is_disjoint(&left));
    assert_eq!(&realised | &left, ids, "every key is realised or left");
    assert!(FALSE_REFUSALS.iter().chain(READ_AS_FIXTURE_INVERSES.iter()).all(|k| realised.contains(k)));
}

/// A01: the counts, in fixed order.
#[test]
fn ct_rte_07_full_a01_counts() {
    let source = root(
        "export fn go(t: Target, w: Word) -> $INTS effects [snapshot] {\n    \
         return read_snapshot<$INTS>(\"counts\");\n}\n",
    );
    quiet(&ten(|| solving(&source)), &encoded(&ints(&[("c0", 2), ("c1", 6), ("c2", 6)])));
}

/// A02: the floors, in fixed order.
#[test]
fn ct_rte_07_full_a02_floors() {
    let source = root(
        "export fn go(t: Target, w: Word) -> $INTS effects [snapshot] {\n    \
         return read_snapshot<$INTS>(\"floors\");\n}\n",
    );
    quiet(&ten(|| solving(&source)), &encoded(&ints(&[("c0", 0), ("c1", 2), ("c2", 2)])));
}

/// A03: the blocked flags, in fixed order.
#[test]
fn ct_rte_07_full_a03_blocked() {
    let source = root(
        "export fn go(t: Target, w: Word) -> $BOOLS effects [snapshot] {\n    \
         return read_snapshot<$BOOLS>(\"blocked\");\n}\n",
    );
    let flags = Value::record(vec![("c0", Value::Bool(false)), ("c1", Value::Bool(true)), ("c2", Value::Bool(false))]);
    quiet(&ten(|| solving(&source)), &encoded(&flags));
}

/// A04: a copy of the labels.
#[test]
fn ct_rte_07_full_a04_snapshot() {
    let source = root(
        "export fn go(t: Target, w: Word) -> $STATE effects [snapshot] {\n    \
         return read_snapshot<$STATE>(\"snapshot\");\n}\n",
    );
    quiet(&ten(|| solving(&source)), &encoded(&ints(&[("s0", 1), ("s1", 0), ("s2", 2), ("s3", 3)])));
}

/// A05: the caps zipped with their counts, in menu order, equal values included.
#[test]
fn ct_rte_07_full_a05_rows_in_menu_order() {
    let source = root(
        "export fn go(t: Target, w: Word) -> record { c0: $PAIR, c1: $PAIR, c2: $PAIR } effects [snapshot] {\n    \
         let n = read_snapshot<$INTS>(\"counts\");\n    \
         return record { c0 = record { count = n.c0, target = target(\"c0\") }, \
         c1 = record { count = n.c1, target = target(\"c1\") }, \
         c2 = record { count = n.c2, target = target(\"c2\") } };\n}\n",
    );
    let pair = |t: &str, n: i64| Value::record(vec![("count", Value::int(n)), ("target", Value::target(t))]);
    let rows = Value::record(vec![("c0", pair("c0", 2)), ("c1", pair("c1", 6)), ("c2", pair("c2", 6))]);
    quiet(&ten(|| solving(&source)), &encoded(&rows));
}

/// A06: the exact total of the counts.
#[test]
fn ct_rte_07_full_a06_exact_total() {
    let source = root(
        "export fn go(t: Target, w: Word) -> Int effects [snapshot] {\n    \
         let n = read_snapshot<$INTS>(\"counts\");\n    \
         return exact_add(exact_add(n.c0, n.c1), n.c2);\n}\n",
    );
    quiet(&ten(|| solving(&source)), &encoded(&Value::int(14)));
}

/// A07: how many floors are zero.
#[test]
fn ct_rte_07_full_a07_count_equal() {
    let source = root(
        "export fn go(t: Target, w: Word) -> Int effects [snapshot] {\n    \
         let f = read_snapshot<$INTS>(\"floors\");\n    \
         return exact_add(exact_add(if(exact_eq(f.c0, 0), 1, 0), if(exact_eq(f.c1, 0), 1, 0)), \
         if(exact_eq(f.c2, 0), 1, 0));\n}\n",
    );
    quiet(&ten(|| solving(&source)), &encoded(&Value::int(1)));
}

/// A08: the length of the solver's word `a a b`, counted letter by letter with no simplification.
#[test]
fn ct_rte_07_full_a08_word_length() {
    let source = root("export fn go(w: Word) -> Int effects [] {\n    return length(w);\n}\n");
    quiet(&ten(|| solving_on(&source, &[&[A, A, B]])), &encoded(&Value::int(3)));
    assert_eq!(named(&[A, A, B]), ["a", "a", "b"], "the corpus's input");
}

/// A09: exact integer arithmetic beyond binary64's exact range.
#[test]
fn ct_rte_07_full_a09_beyond_binary64() {
    let source = root(
        "export fn go(t: Target, w: Word) -> Int effects [] {\n    \
         return exact_add(9007199254740993, 7);\n}\n",
    );
    let out = encoded(&Value::int(9_007_199_254_741_000_i64));
    assert_eq!(out, b"{\"int\":\"9007199254741000\"}\n");
    quiet(&ten(|| solving(&source)), &out);
}

/// A10: LX-3's `mod_dot` of `[1, 2]` and `[3, 1]` modulo 5, in F5's exact field arithmetic: `1*3 + 2*1 = 5`, whose
/// residue is 0.
#[test]
fn ct_rte_07_full_a10_dot_modulo_five() {
    let source = root(
        "export fn go(t: Target, w: Word) -> Nat effects [] {\n    \
         return residue(field_add(field_mul(to_f5(1), to_f5(3)), field_mul(to_f5(2), to_f5(1))));\n}\n",
    );
    quiet(&ten(|| solving(&source)), &encoded(&Value::int(0)));
}

/// The chart of A11 and A12: the row's values as indexed points, with their exact values.
fn points(row: &str) -> String {
    root(&format!(
        "export fn go(t: Target, w: Word) -> Text effects [snapshot, view] {{\n    \
         let v = read_snapshot<$INTS>(\"{row}\");\n    \
         let shown = publish_view(record {{ coordinates = record {{ p0 = record {{ x = 0, y = v.c0 }}, \
         p1 = record {{ x = 1, y = v.c1 }}, p2 = record {{ x = 2, y = v.c2 }} }}, \
         exact = record {{ exact_properties = record {{ values = v }}, kind = \"points\", \
         object_ids = record {{ i0 = \"c0\", i1 = \"c1\", i2 = \"c2\" }}, \
         order = record {{ i0 = 0, i1 = 1, i2 = 2 }} }} }});\n    \
         return \"published\";\n}}\n"
    ))
}

/// The view a chart key publishes, built from its exact values. The corpus's display also carries host-added
/// `source_digest` and `input_digest` fields; the draft view sink adds none, so they are not compared.
fn chart(values: &[(&str, i64)], coordinates: Vec<(&str, Value)>, ids: &[&str]) -> Vec<u8> {
    let indexed = |items: Vec<Value>| {
        let names = ["i0", "i1", "i2"];
        Value::record(names.iter().copied().zip(items).collect())
    };
    let exact = Value::record(vec![
        (
            "exact_properties",
            Value::record(vec![(if values.len() == 1 { "value" } else { "values" }, ints_or_int(values))]),
        ),
        ("kind", Value::text("points")),
        ("object_ids", indexed(ids.iter().map(|i| Value::text(i)).collect())),
        ("order", indexed((0..ids.len() as i64).map(Value::int).collect())),
    ]);
    encoded(&Value::record(vec![("coordinates", Value::record(coordinates)), ("exact", exact)]))
}

fn ints_or_int(values: &[(&str, i64)]) -> Value {
    match values {
        [(_, v)] => Value::int(*v),
        _ => ints(values),
    }
}

fn point(x: i64, y: i64) -> Value {
    Value::record(vec![("x", Value::int(x)), ("y", Value::int(y))])
}

/// A11: the counts as indexed points.
#[test]
fn ct_rte_07_full_a11_counts_chart() {
    let source = points("counts");
    let trial = ten(|| solving(&source));
    completed(&trial, &text("published"));
    let coordinates = vec![("p0", point(0, 2)), ("p1", point(1, 6)), ("p2", point(2, 6))];
    let view = chart(&[("c0", 2), ("c1", 6), ("c2", 6)], coordinates, &["c0", "c1", "c2"]);
    assert_eq!(trial.engine.views, [view]);
}

/// A12: the floors as indexed points.
#[test]
fn ct_rte_07_full_a12_floors_chart() {
    let source = points("floors");
    let trial = ten(|| solving(&source));
    completed(&trial, &text("published"));
    let coordinates = vec![("p0", point(0, 0)), ("p1", point(1, 2)), ("p2", point(2, 2))];
    let view = chart(&[("c0", 0), ("c1", 2), ("c2", 2)], coordinates, &["c0", "c1", "c2"]);
    assert_eq!(trial.engine.views, [view]);
}

/// A13: one count as a point. The corpus indexes the counts at the literal 1, the cap `c1` the solver names; the run's
/// target argument stays `c0`, which the key does not read.
#[test]
fn ct_rte_07_full_a13_one_point() {
    let source = root(
        "export fn go(t: Target, w: Word) -> Text effects [snapshot, view] {\n    \
         let n = read_snapshot<$INTS>(\"counts\");\n    \
         let shown = publish_view(record { coordinates = record { p0 = record { x = 1, y = n.c1 } }, \
         exact = record { exact_properties = record { value = n.c1 }, kind = \"points\", \
         object_ids = record { i0 = \"c1\" }, order = record { i0 = 0 } } });\n    \
         return \"published\";\n}\n",
    );
    let trial = ten(|| solving(&source));
    completed(&trial, &text("published"));
    assert_eq!(trial.engine.views, [chart(&[("c1", 6)], vec![("p0", point(1, 6))], &["c1"])]);
}

/// A14: a view aid over the solver's word `a`. It reads the row `word_check` with the word as its argument, then
/// publishes the corpus's word path from literal values.
#[test]
fn ct_rte_07_full_a14_word_path() {
    let source = root(
        "export fn go(w: Word) -> Text effects [snapshot, view] {\n    \
         let check = read_snapshot<$CHECK>(\"word_check\", w);\n    \
         let shown = publish_view(record { coordinates = record { p0 = record { x = 0, y = 1 }, \
         p1 = record { x = 1, y = 0 } }, exact = record { exact_properties = record { word = word[1] }, \
         kind = \"word_path\", object_ids = record { i0 = \"slot0\", i1 = \"slot1\" }, \
         order = record { i0 = 0, i1 = 1 } } });\n    \
         return \"published\";\n}\n",
    );
    let trial = ten(|| solving_on(&source, &[&[A]]));
    completed(&trial, &text("published"));
    let exact = Value::record(vec![
        ("exact_properties", Value::record(vec![("word", Value::word(&[A]))])),
        ("kind", Value::text("word_path")),
        ("object_ids", Value::record(vec![("i0", Value::text("slot0")), ("i1", Value::text("slot1"))])),
        ("order", Value::record(vec![("i0", Value::int(0)), ("i1", Value::int(1))])),
    ]);
    let coordinates = Value::record(vec![("p0", point(0, 1)), ("p1", point(1, 0))]);
    assert_eq!(trial.engine.views, [encoded(&Value::record(vec![("coordinates", coordinates), ("exact", exact)]))]);
    assert_eq!(trial.engine.served, ["word_check/[1]"]);
    assert_eq!(named(&[A]), ["a"], "the corpus's input");
}

/// A15: a fixed rational reference chart. The exact fraction stays exact; the coordinates are shown at 53 bits, and
/// each lies within that precision of its exact value (RTE.46's display precision).
#[test]
fn ct_rte_07_full_a15_display_precision() {
    let source = root(
        "export fn go(t: Target, w: Word) -> Text effects [view] {\n    \
         let shown = publish_view(record { coordinates = record { \
         p0 = record { x = approx53(0), y = approx53(0) }, p1 = record { x = approx53(1/3), y = approx53(1) } }, \
         exact = record { exact_properties = record { fraction = 1/3 }, kind = \"reference\", \
         object_ids = record { i0 = \"origin\", i1 = \"third\" }, order = record { i0 = 0, i1 = 1 } } });\n    \
         return \"published\";\n}\n",
    );
    let trial = ten(|| solving(&source));
    completed(&trial, &text("published"));
    let view = String::from_utf8(trial.engine.views[0].clone()).unwrap();
    let zero = "{\"x\":{\"f64\":\"0000000000000000\"},\"y\":{\"f64\":\"0000000000000000\"}}";
    let third = "{\"x\":{\"f64\":\"3fd5555555555555\"},\"y\":{\"f64\":\"3ff0000000000000\"}}";
    assert!(view.contains(&format!("\"coordinates\":{{\"p0\":{zero},\"p1\":{third}}}")), "{view}");
    assert!(view.contains("\"exact_properties\":{\"fraction\":{\"rat\":[\"1\",\"3\"]}}"), "{view}");
    // x = m * 2^k exactly; |x - 1/3| <= 2^-53 * 1/3 holds when |3m - 2^-k| <= 2^(-k-53).
    let bits = u64::from_str_radix("3fd5555555555555", 16).unwrap();
    let (m, k) = (i128::from((bits & ((1 << 52) - 1)) | (1 << 52)), ((bits >> 52) & 0x7ff) as i32 - 1075);
    assert!((3 * m - (1i128 << -k)).abs() <= 1i128 << (-k - 53));
}

/// The corpus's `word_check` keys: the row read with the solver's word as its argument, returned as it is.
const WORD_CHECK: &str = "export fn go(w: Word) -> $CHECK effects [snapshot] {\n    \
                          return read_snapshot<$CHECK>(\"word_check\", w);\n}\n";

/// The row `word_check` the corpus expects for a word: its end labels, the slots it moves, and whether the
/// protected slot stays.
fn word_check(word: &[i32], end: [i64; 4], moved: &[i64], protected: bool) -> Vec<u8> {
    let labels = ["s0", "s1", "s2", "s3"].into_iter().zip(end).map(|(s, l)| (s, Value::int(l))).collect();
    let coverage = vec![("complete", Value::Bool(true)), ("letters", Value::int(word.len() as i64))];
    encoded(&Value::record(vec![
        ("coverage", Value::record(coverage)),
        ("end_labels", Value::record(labels)),
        ("moved_slots", Value::list(moved.iter().map(|&i| Value::int(i)).collect())),
        ("protected", Value::Bool(protected)),
        ("word", Value::word(word)),
    ]))
}

/// A `word_check` key on the solver's word: the corpus's value, and one row served with the word as its argument.
fn checked(word: &[i32], names: &[&str], expected: Vec<u8>) {
    let source = root(WORD_CHECK);
    let trial = ten(|| solving_on(&source, &[word]));
    quiet(&trial, &expected);
    let letters: Vec<String> = word.iter().map(i32::to_string).collect();
    assert_eq!(trial.engine.served, [format!("word_check/[{}]", letters.join(","))]);
    assert_eq!(named(word), names, "the corpus's input");
}

/// A16: the check of the solver's word `a`, in a run that also has the corpus's target.
#[test]
fn ct_rte_07_full_a16_word_check() {
    let source = root(
        "export fn go(t: Target, w: Word) -> $CHECK effects [snapshot] {\n    \
         return read_snapshot<$CHECK>(\"word_check\", w);\n}\n",
    );
    let trial = ten(|| solving(&source));
    quiet(&trial, &word_check(&[A], [0, 1, 2, 3], &[0, 1], true));
    assert_eq!(trial.engine.served, ["word_check/[1]"]);
}

/// A19: `a a` moves no slot; the letters are counted with no simplification.
#[test]
fn ct_rte_07_full_a19_word_check_no_simplification() {
    checked(&[A, A], &["a", "a"], word_check(&[A, A], [1, 0, 2, 3], &[], true));
}

/// A21: `a` swaps slots 0 and 1.
#[test]
fn ct_rte_07_full_a21_word_check_a() {
    checked(&[A], &["a"], word_check(&[A], [0, 1, 2, 3], &[0, 1], true));
}

/// A22: `b` moves slots 1 and 2 and keeps the protected slot.
#[test]
fn ct_rte_07_full_a22_word_check_b() {
    checked(&[B], &["b"], word_check(&[B], [1, 2, 0, 3], &[1, 2], true));
}

/// A23: `c` moves the protected slot 3; a false check is a value, not a refusal.
#[test]
fn ct_rte_07_full_a23_word_check_c() {
    checked(&[C], &["c"], word_check(&[C], [1, 0, 3, 2], &[2, 3], false));
}

/// The engine's rows match `fixture.json`: its permutations, the IR letters' fixture names and the setups.
#[test]
fn ct_rte_07_full_engine_rows_are_the_fixture() {
    let fixture = provenance::parse(&std::fs::read(format!("{CORPUS}fixture.json")).unwrap()).expect("it reads");
    let ints = |j: &Json| match j {
        Json::List(xs) => {
            xs.iter().map(|x| if let Json::Int(n) = x { *n } else { panic!("{x:?}") }).collect::<Vec<_>>()
        }
        other => panic!("{other:?}"),
    };
    let Some(Json::Object(permutations)) = fixture.get("permutations") else { panic!("the permutations") };
    assert_eq!(permutations.len(), PERMUTATIONS.len());
    for (name, p) in PERMUTATIONS {
        assert_eq!(ints(&permutations[name]), p.map(|i| i as i64), "{name}");
    }
    for (letter, name) in LETTERS {
        assert_eq!(named(&[letter]), [name], "{letter}");
    }
    let Some(Json::Object(setups)) = fixture.get("setups") else { panic!("the setups") };
    assert_eq!(setups.len(), SETUPS.len());
    for (target, words) in SETUPS {
        let Json::List(listed) = &setups[target] else { panic!("{target}") };
        let named: Vec<Vec<String>> = words.iter().map(|w| named(w)).collect();
        let listed: Vec<Vec<String>> = listed
            .iter()
            .map(|w| match w {
                Json::List(ls) => ls.iter().map(|l| l.as_text().expect("a letter").to_string()).collect(),
                other => panic!("{other:?}"),
            })
            .collect();
        assert_eq!(named, listed, "{target}");
    }
    let Some(Json::Object(session)) = fixture.get("session") else { panic!("the session") };
    assert_eq!(ints(&session["protected_slots"]), [PROTECTED_SLOT as i64]);
}

/// A17: the inverse of the solver's word `D a`: its letters reversed, each replaced by its inverse. The IR writes the
/// inverse of `a` as `-1`, which the fixture names `a` ([`named`]), so the corpus's `a d` is `-1, +4`: a listed
/// difference ([`READ_AS_FIXTURE_INVERSES`]).
#[test]
fn ct_rte_07_full_a17_word_inverse() {
    let source = root("export fn go(w: Word) -> Word effects [] {\n    return inverse(w);\n}\n");
    quiet(&ten(|| solving_on(&source, &[&[-D, A]])), &encoded(&Value::word(&[-A, D])));
    assert_eq!(named(&[-D, A]), ["D", "a"], "the corpus's input");
    assert_eq!(named(&[-A, D]), ["a", "d"], "the corpus's value");
    assert!(READ_AS_FIXTURE_INVERSES.contains(&"A17"));
}

/// A18: the solver's words `a b` and `c`, concatenated in chronological order.
#[test]
fn ct_rte_07_full_a18_word_concatenation() {
    let source =
        root("export fn go(left: Word, right: Word) -> Word effects [] {\n    return concat(left, right);\n}\n");
    quiet(&ten(|| solving_on(&source, &[&[A, B], &[C]])), &encoded(&Value::word(&[A, B, C])));
    assert_eq!(named(&[A, B, C]), ["a", "b", "c"], "the corpus's value");
}

/// A20: a static explanation; texts compare by scalars, with no locale normalization.
#[test]
fn ct_rte_07_full_a20_scalar_text() {
    let source = root(
        "export fn go(t: Target, w: Word) -> Text effects [] {\n    \
         return \"Named setup: c1; labels: \\u00e9 / e\\u0301 / \\u03bd.\";\n}\n",
    );
    let expected = text("Named setup: c1; labels: \u{e9} / e\u{301} / \u{3bd}.");
    assert_ne!(expected, text("Named setup: c1; labels: \u{e9} / \u{e9} / \u{3bd}."));
    quiet(&ten(|| solving(&source)), &expected);
}

/// A24: a preview of the manually named word only; it never commits.
#[test]
fn ct_rte_07_full_a24_manual_preview() {
    let source = root(
        "export fn go(t: Target, w: Word) -> Text effects [preview] {\n    \
         let r = propose(w, t);\n    return \"requested\";\n}\n",
    );
    let trial = ten(|| solving(&source));
    let out = completed(&trial, &text("requested"));
    assert!(matches!(out.shell[0].answer, Answer::Previewed(_)) && out.separated.is_empty());
    assert_eq!(trial.engine.previews, [(vec![A], "c0".to_string())]);
    assert!(trial.store.records.is_empty());
}

/// A25: a facts report and an independent manual preview. Separation refuses the preview: a false refusal, which
/// the corpus allows and counts ([`FALSE_REFUSALS`]).
#[test]
fn ct_rte_07_full_a25_false_refusal_is_listed() {
    let source = root(
        "export fn go(t: Target, w: Word) -> Text effects [snapshot, view, preview] {\n    \
         let n = read_snapshot<$INTS>(\"counts\");\n    let shown = publish_view(n);\n    \
         let r = propose(w, t);\n    return \"requested\";\n}\n",
    );
    let trial = ten(|| solving(&source));
    completed(&trial, &text("requested"));
    assert_eq!(trial.engine.views, [encoded(&ints(&[("c0", 2), ("c1", 6), ("c2", 6)]))]);
    refused_preview(&trial, NOT_GRANTED, true);
    assert!(FALSE_REFUSALS.contains(&"A25"));
}

/// A26 to A28: the setup search for the target the solver names, entered by hand. The row `setup` is read with the
/// target as its argument and returns the fixture's ordered setups for it.
fn setups(target: &str, names: &[&[&str]]) {
    let source = root(
        "export fn go(t: Target) -> List<Word> effects [snapshot] {\n    \
         return read_snapshot<List<Word>>(\"setup\", t);\n}\n",
    );
    let words = SETUPS.iter().find(|(t, _)| *t == target).expect("the fixture's setups").1;
    let named: Vec<Vec<String>> = words.iter().map(|w| named(w)).collect();
    assert_eq!(named, names, "the corpus's value");
    let solve = || {
        let args = vec![Value::target(target)];
        Trial::of(&source, &[]).call("go", Author::BuiltIn, Profile::Solving, args, vec![manual()], WIDE)
    };
    let trial = ten(solve);
    quiet(&trial, &encoded(&Value::list(words.iter().map(|w| Value::word(w)).collect())));
    assert_eq!(trial.engine.served, [format!("setup/\"{target}\"")]);
}

#[test]
fn ct_rte_07_full_a26_setups_c0() {
    setups("c0", &[&[], &["a"]]);
}

#[test]
fn ct_rte_07_full_a27_setups_c1() {
    setups("c1", &[&["b"], &["a", "b"]]);
}

#[test]
fn ct_rte_07_full_a28_setups_c2() {
    setups("c2", &[&["a", "b"], &["b", "a"]]);
}

/// MR17, MR18, A29 and A30, run 2: the built-in solving entry `go` proposes `w` at `t`.
const GO_PICKED: &str = "export fn go(t: Target, w: Word) -> Text effects [preview] {\n    \
                         let r = propose(w, t);\n    return \"requested\";\n}\n";

/// The block `c600-run`'s boundary records for what a built-in solving run of `m.<function>` returns when it takes
/// the stored blocks `inputs` (BND.22, BND.24): the producer `c600-module` builds from the approval, whether the entry
/// may read state, and the inputs as parts. The host keeps it beside the list it shows.
fn returned_block(package: &Package, function: &str, inputs: &[Vec<u8>]) -> Vec<u8> {
    let ready = ready(package, entry(function, Author::BuiltIn, Profile::Solving)).expect("the entry is approved");
    let a = ready.approval();
    let producer = Producer {
        contract: a.contract.clone(),
        checker: Some(a.checker.clone()),
        author: a.entry.author,
        module: a.module,
        entry: a.entry.function.clone(),
        profile: a.entry.profile,
    };
    let reads_state = a.admitted.separation.reads_state;
    let context = Context {
        run: "r1".into(),
        base: "rev-7".into(),
        producer,
        reads_state,
        inputs: inputs.to_vec(),
        pins: pins(),
    };
    let (mut engine, mut store) = (Engine::fixture(), Store::default());
    Boundary::new(&mut engine, &mut store, context).block()
}

/// Runs the built-in solving entry `m.offered` with `args`, each entered by hand, and gives the solver's pick of its
/// option 0 (BND.54): the host keeps the list's block, writes FP.38's `c600-pick` record with `fixed` and `target`,
/// and gives the picked value with its block, a hand-entered value (BND.23) that records the pick and keeps the list's
/// block in `parts` (BND.85).
fn listed_and_picked(trial: Trial, args: Vec<Value>, fixed: bool, target: &str) -> (Trial, Value, Vec<u8>) {
    let inputs = vec![manual(); args.len()];
    let list = returned_block(&trial.package, "offered", &inputs);
    let mut trial = trial.call("offered", Author::BuiltIn, Profile::Solving, args, inputs, WIDE);
    let (options, chosen) = {
        let out = trial.last();
        let Some(Value::List(items)) = &out.run.value else { panic!("a list: {:?}", out.run.value) };
        (out.run.output.clone().expect("the list's bytes"), items.items()[0].clone())
    };
    let content = |bytes: &[u8]| provenance::digest_ref("content", &sha256(bytes));
    let identity = Json::object([
        ("puzzle", Json::text("full")),
        ("model_id", Json::text("58c83f336e451aa64147fe1b8db2d60d7d43806aa3a35122f8403d5af51253f9")),
        ("menu_id", Json::Null),
        ("menu_order", Json::Null),
        ("contract_rev", Json::Null),
    ]);
    let payload = Json::object([
        ("list", content(&list)),
        ("fixed", Json::Bool(fixed)),
        ("options", content(&options)),
        ("chosen", content(&encoded(&chosen))),
        ("target", Json::text(target)),
    ]);
    let record = Json::object([
        ("format", Json::text("c600-pick")),
        ("version", Json::Int(1)),
        ("identity", identity),
        ("provenance", Json::Null),
        ("payload", payload),
    ]);
    trial.store.keep(&list);
    let digest = trial.store.keep_pick(&provenance::canonical(&record));
    let reference = Json::object([
        ("format", Json::text("c600-pick")),
        ("version", Json::Int(1)),
        ("content", provenance::digest_ref("content", &digest)),
    ]);
    let pick = Json::object([("record", reference), ("chosen", content(&encoded(&chosen)))]);
    let Json::Object(mut block) = provenance::manual(&contract()) else { panic!("a block is an object") };
    block.insert("parts".into(), Json::List(vec![content(&list)]));
    block.insert("pick".into(), pick);
    (trial, chosen, provenance::canonical(&Json::Object(block)))
}

/// A refused pick key: the picked value's block is refused at preview admission, before the split, because the list
/// is no fixed entry (BND.54, BND.55).
fn refused_pick(trial: &Trial, key: &str, word: &[i32], target: &str) {
    refused_preview(trial, &corpus_code("must-refuse.json", key), false);
    let shown = &trial.last().shell[0];
    assert_eq!((shown.word.as_slice(), shown.target.as_str()), (word, target));
    let Answer::Refused(refusal) = &shown.answer else { panic!("a refusal") };
    assert!(refusal.detail.contains("BND.55"), "the pick is refused: {}", refusal.detail);
}

/// MR17: a solving run computes a one-item target list from the counts, the first largest of the corpus's targets,
/// unrolled over them; the solver picks its only option, and a later run proposes the manual word at the picked
/// target. The list is no fixed entry, so the picked target is refused at preview admission while BND.55 is open (LP2
/// draft; it fails closed), and no preview is installed.
#[test]
fn ct_rte_07_full_mr17_one_item_target_pick() {
    let list = "export fn offered() -> List<Target> effects [snapshot] {\n    \
        let n = read_snapshot<$INTS>(\"counts\");\n    \
        let i = if(exact_less(n.c0, n.c1), if(exact_less(n.c1, n.c2), 2, 1), if(exact_less(n.c0, n.c2), 2, 0));\n    \
        return list<Target>(if(exact_eq(i, 0), target(\"c0\"), if(exact_eq(i, 1), target(\"c1\"), target(\"c2\"))));\n\
        }\n\n";
    let source = root(&[list, GO_PICKED].concat());
    let trial = ten(|| {
        let (trial, target, block) = listed_and_picked(Trial::of(&source, &[]), vec![], false, "c1");
        let args = vec![target, Value::word(&[A])];
        trial.call("go", Author::BuiltIn, Profile::Solving, args, vec![block, manual()], WIDE)
    });
    assert_eq!(trial.engine.served, ["counts"]);
    refused_pick(&trial, "MR17", &[A], "c1");
}

/// MR18: a solving run builds a one-item word list from `nu2`'s corrections; the solver picks its only option, and a
/// later run proposes it at the manual target. The list is no fixed entry, so the picked word is refused at preview
/// admission while BND.55 is open (LP2 draft; it fails closed), and no preview is installed.
#[test]
fn ct_rte_07_full_mr18_one_item_word_pick() {
    let list = "export fn offered() -> List<Word> effects [snapshot] {\n    \
        let nu = read_snapshot<$NU>(\"nu2\");\n    \
        let w = if(exact_eq(nu.i0, 1), if(exact_eq(nu.i1, 1), word[1, 2], word[1]), \
        if(exact_eq(nu.i1, 1), word[2], word[]));\n    \
        return list<Word>(w);\n}\n\n";
    let source = root(&[list, GO_PICKED].concat());
    let trial = ten(|| {
        let (trial, word, block) = listed_and_picked(Trial::of(&source, &[]), vec![], false, "c0");
        let args = vec![Value::target("c0"), word];
        trial.call("go", Author::BuiltIn, Profile::Solving, args, vec![manual(), block], WIDE)
    });
    assert_eq!(trial.engine.served, ["nu2"]);
    refused_pick(&trial, "MR18", &[A], "c0");
}

/// A29 and A30: the setup search for the target the solver names (BND.1), a fixed entry; the solver picks row 0, and
/// a later run proposes it at that target. The picked word keeps the setup list's block, which may read state, and is
/// admitted: the proposal's own block reads no state, so the split passes it too, and the engine installs it.
fn picked_setup(list: &str, served: &[&str]) -> Trial {
    let source = root(&[list, GO_PICKED].concat());
    let trial = ten(|| {
        let (trial, word, block) = listed_and_picked(Trial::of(&source, &[]), vec![Value::target("c1")], true, "c1");
        let args = vec![Value::target("c1"), word];
        trial.call("go", Author::BuiltIn, Profile::Solving, args, vec![manual(), block], WIDE)
    });
    let out = completed(&trial, &text("requested"));
    assert!(matches!(out.shell[0].answer, Answer::Previewed(_)) && out.separated.is_empty());
    assert_eq!(named(&[B]), ["b"], "the corpus's word");
    assert_eq!(trial.engine.previews, [(vec![B], "c1".to_string())]);
    assert_eq!(trial.engine.served, served);
    assert!(trial.store.records.is_empty(), "no workspace write");
    trial
}

/// A29: row 0 of the setups for `c1`, picked and previewed at `c1`.
#[test]
fn ct_rte_07_full_a29_picked_setup() {
    let trial = picked_setup(
        "export fn offered(t: Target) -> List<Word> effects [snapshot] {\n    \
         return read_snapshot<List<Word>>(\"setup\", t);\n}\n\n",
        &["setup/\"c1\""],
    );
    assert!(trial.engine.views.is_empty());
}

/// A30: A29 after an unrelated facts report in the same run, which publishes the counts. The report does not choose
/// the setup or the target, and the pick is admitted: no false refusal here.
#[test]
fn ct_rte_07_full_a30_picked_setup_after_a_report() {
    let trial = picked_setup(
        "export fn offered(t: Target) -> List<Word> effects [snapshot, view] {\n    \
         let n = read_snapshot<$INTS>(\"counts\");\n    let shown = publish_view(n);\n    \
         return read_snapshot<List<Word>>(\"setup\", t);\n}\n\n",
        &["counts", "setup/\"c1\""],
    );
    assert_eq!(trial.engine.views, [encoded(&ints(&[("c0", 2), ("c1", 6), ("c2", 6)]))]);
    assert!(!FALSE_REFUSALS.contains(&"A30"));
}

/// A33: a research search over the fixed words returns the first word of least score.
#[test]
fn ct_rte_07_full_a33_research_search() {
    let source =
        root("export fn go(t: Target, w: Word) -> Word effects [snapshot] {\n$SEARCH    return $REBUILD;\n}\n");
    let trial = ten(|| Trial::of(&source, &[]).run("go", Author::BuiltIn, Profile::Research));
    quiet(&trial, &encoded(&Value::word(&[A])));
}

/// A34: research records are readable and keep their provenance.
#[test]
fn ct_rte_07_full_a34_research_record() {
    let source = root(
        "export fn go(t: Target, w: Word) -> $NU effects [snapshot, workspace] {\n    \
         let nu = read_snapshot<$NU>(\"nu2\");\n    let put = write_workspace<$NU>(\"research_nu\", nu);\n    \
         return read_snapshot<$NU>(\"research_nu\");\n}\n",
    );
    let trial = ten(|| Trial::of(&source, &[]).run("go", Author::BuiltIn, Profile::Research));
    completed(&trial, &encoded(&ints(&[("i0", 1), ("i1", 0)])));
    let block = provenance::read(&trial.store.records[&Place::Workspace("research_nu".into())].block).unwrap();
    assert_eq!((field::author(&block), field::profile(&block)), ("built_in", "research"));
    assert!(field::may_read_state(&block));
}

/// A35: a research proposal is captured for inspection, with no engine preview (BND.60).
#[test]
fn ct_rte_07_full_a35_research_capture() {
    let source = root(
        "export fn go(t: Target, w: Word) -> Text effects [snapshot, preview] {\n    \
         let nu = read_snapshot<$NU>(\"nu2\");\n    \
         let combined = if(exact_eq(nu.i0, 1), if(exact_eq(nu.i1, 1), word[1, 2], word[1]), \
         if(exact_eq(nu.i1, 1), word[2], word[]));\n    \
         let r = propose(combined, t);\n    return \"captured\";\n}\n",
    );
    let trial = ten(|| Trial::of(&source, &[]).run("go", Author::BuiltIn, Profile::Research));
    let out = completed(&trial, &text("captured"));
    assert!(out.shell.is_empty() && trial.engine.previews.is_empty());
    let capture = &out.captures[0];
    assert_eq!((capture.word.as_slice(), capture.target.as_str()), (&[A][..], "c0"));
}

/// MR03: a solving search rebuilds the selected word from literal letters; the rebuilt word keeps the dependence.
#[test]
fn ct_rte_07_full_mr03_search_rebuild() {
    let source = root(
        "export fn go(t: Target, w: Word) -> Text effects [snapshot, preview] {\n$SEARCH    \
         let rebuilt = $REBUILD;\n    let r = propose(rebuilt, t);\n    return \"requested\";\n}\n",
    );
    let trial = ten(|| solving(&source));
    assert_eq!(trial.last().shell[0].word, [A]);
    refused_preview(&trial, &corpus_code("must-refuse.json", "MR03"), true);
}

/// MR04: research stores the index it found; a solving run reads it and rebuilds the word from the fixed word list,
/// unrolled over its literal words. The index keeps its research provenance, and preview admission refuses it
/// (BND.61) before the split.
#[test]
fn ct_rte_07_full_mr04_research_index() {
    let source = root(
        "export fn search(t: Target, w: Word) -> Unit effects [snapshot, workspace] {\n$SEARCH    \
         return write_workspace<Int>(\"selected_index\", i);\n}\n\n\
         export fn go(t: Target, w: Word) -> Text effects [snapshot, preview] {\n    \
         let i = read_snapshot<Int>(\"selected_index\");\n    let rebuilt = $REBUILD;\n    \
         let r = propose(rebuilt, t);\n    return \"requested\";\n}\n",
    );
    let trial = ten(|| {
        Trial::of(&source, &[]).run("search", Author::BuiltIn, Profile::Research).run(
            "go",
            Author::BuiltIn,
            Profile::Solving,
        )
    });
    refused_preview(&trial, &corpus_code("must-refuse.json", "MR04"), false);
}

/// MR07: tool authoring saves a word chosen from the counts; a solving run copies it to another record and proposes
/// it. Every part stays in the provenance; the proposal is refused, and the saved record's block is unchanged.
#[test]
fn ct_rte_07_full_mr07_workspace_laundering() {
    let source = root(
        "export fn save(t: Target, w: Word) -> Unit effects [snapshot, workspace] {\n    \
         let n = read_snapshot<$INTS>(\"counts\");\n    \
         let i = if(exact_less(n.c0, n.c1), if(exact_less(n.c1, n.c2), 2, 1), if(exact_less(n.c0, n.c2), 2, 0));\n    \
         return write_workspace<Word>(\"saved\", $REBUILD);\n}\n\n\
         export fn go(t: Target, w: Word) -> Text effects [snapshot, workspace, preview] {\n    \
         let saved = read_snapshot<Word>(\"saved\");\n    let put = write_workspace<Word>(\"renamed\", saved);\n    \
         let renamed = read_snapshot<Word>(\"renamed\");\n    let r = propose(renamed, t);\n    \
         return \"requested\";\n}\n",
    );
    let saved = |trial: &Trial| trial.store.records[&Place::Workspace("saved".into())].clone();
    let first = Trial::of(&source, &[]).run("save", Author::BuiltIn, Profile::ToolAuthoring);
    let before = saved(&first);
    assert!(matches!(before.value, Value::Word(ref w) if **w == [B]), "the first largest count");
    let trial = ten(|| {
        Trial::of(&source, &[]).run("save", Author::BuiltIn, Profile::ToolAuthoring).run(
            "go",
            Author::BuiltIn,
            Profile::Solving,
        )
    });
    refused_preview(&trial, &corpus_code("must-refuse.json", "MR07"), true);
    assert_eq!(saved(&trial).block, before.block, "the existing record's provenance is unchanged");
    let renamed = provenance::read(&trial.store.records[&Place::Workspace("renamed".into())].block).unwrap();
    assert!(field::parts(&renamed).contains(&sha256(&before.block)));
}

/// MR08: a word whose block does not read is unknown; storing and loading it never makes it admissible.
#[test]
fn ct_rte_07_full_mr08_unknown_provenance() {
    let source = root(
        "export fn keep(t: Target, legacy: Word) -> Unit effects [workspace] {\n    \
         return write_workspace<Word>(\"legacy\", legacy);\n}\n\n\
         export fn go(t: Target, w: Word) -> Text effects [snapshot, preview] {\n    \
         let l = read_snapshot<Word>(\"legacy\");\n    let r = propose(l, t);\n    return \"requested\";\n}\n",
    );
    let legacy = b"a legacy word without a readable block".to_vec();
    let trial = ten(|| {
        Trial::of(&source, &[])
            .run_with("keep", Author::BuiltIn, Profile::ToolAuthoring, vec![manual(), legacy.clone()])
            .run("go", Author::BuiltIn, Profile::Solving)
    });
    let expected = corpus_code("must-refuse.json", "MR08");
    assert_eq!(expected, INCOMPATIBLE);
    refused_preview(&trial, &expected, false);
}

/// MR09: a state-conditioned branch chooses between two constant words.
#[test]
fn ct_rte_07_full_mr09_state_branch() {
    let source = root(
        "export fn go(t: Target, w: Word) -> Text effects [snapshot, preview] {\n    \
         let n = read_snapshot<$INTS>(\"counts\");\n    \
         let r = if(exact_less(0, n.c0), propose(word[1], t), propose(word[2], t));\n    return \"requested\";\n}\n",
    );
    let trial = ten(|| solving(&source));
    refused_preview(&trial, &corpus_code("must-refuse.json", "MR09"), true);
}

/// MR10: a branch-dependent assignment survives the join and reaches a later preview.
#[test]
fn ct_rte_07_full_mr10_branch_join() {
    let source = root(
        "export fn go(t: Target, w: Word) -> Text effects [snapshot, preview] {\n    \
         let bits = read_snapshot<$BITS>(\"bits\");\n    let choice = if(exact_eq(bits.i1, 1), word[1], word[2]);\n    \
         let r = propose(choice, t);\n    return \"requested\";\n}\n",
    );
    let trial = ten(|| solving(&source));
    refused_preview(&trial, &corpus_code("must-refuse.json", "MR10"), true);
}

/// The selector module of MR11 and MR12: it reads the state inside the callee.
fn selector() -> String {
    src(
        "selector",
        &[],
        &[],
        "export fn select() -> Word effects [snapshot] {\n    let n = read_snapshot<$NU>(\"nu2\");\n    \
         return if(exact_eq(n.i0, 1), word[1], word[2]);\n}\n",
    )
}

/// MR11: the read happens inside a callee in another module.
#[test]
fn ct_rte_07_full_mr11_module_flow() {
    let dep = selector();
    let source = src(
        "m",
        &[],
        &[("selector", dep.as_str())],
        "export fn go(t: Target, w: Word) -> Text effects [snapshot, preview] {\n    \
         let chosen = selector.select();\n    let r = propose(chosen, t);\n    return \"requested\";\n}\n",
    );
    let trial = ten(|| Trial::of(&source, &[&dep]).run("go", Author::BuiltIn, Profile::Solving));
    refused_preview(&trial, &corpus_code("must-refuse.json", "MR11"), true);
}

/// MR12: an identity wrapper in a second module neither hides the dependence nor drops the callee's effects.
#[test]
fn ct_rte_07_full_mr12_identity_wrapper() {
    let dep = selector();
    let copy = src("copy", &[], &[], "export fn same(x: Word) -> Word effects [] {\n    return x;\n}\n");
    let source = src(
        "m",
        &[],
        &[("selector", dep.as_str()), ("copy", copy.as_str())],
        "export fn go(t: Target, w: Word) -> Text effects [snapshot, preview] {\n    \
         let selected = selector.select();\n    let copied = copy.same(selected);\n    \
         let r = propose(copied, t);\n    return \"requested\";\n}\n",
    );
    let trial = ten(|| Trial::of(&source, &[&dep, &copy]).run("go", Author::BuiltIn, Profile::Solving));
    refused_preview(&trial, &corpus_code("must-refuse.json", "MR12"), true);
}

/// MR15 and MR16: a probe's error or protection would choose a later word. The probe is refused at the preview point,
/// and the outcome read stops the run with the same code; no later proposal is made.
fn refusal_driven(probe: i32, read: &str, pattern: &str, key: &str) {
    let source = root(&format!(
        "export fn go(t: Target, w: Word) -> Text effects [snapshot, preview] {{\n    \
         let probe = propose(word[{probe}], t);\n    \
         return match {read}(probe) {{\n        tag(\"{pattern}\") => receipt_id(propose(word[1], t));\n        \
         _ => receipt_id(propose(word[2], t));\n    }};\n}}\n"
    ));
    let trial = ten(|| solving(&source));
    let expected = corpus_code("must-refuse.json", key);
    refused_preview(&trial, &expected, true);
    let out = trial.last();
    assert_eq!((out.run.summary.outcome, out.run.summary.code), (Ended::Error, Some(expected.as_str())));
    assert_eq!(out.shell.len(), 1, "no later proposal");
}

#[test]
fn ct_rte_07_full_mr15_error_driven() {
    refusal_driven(4, "receipt_error", PROTECTION, "MR15");
}

#[test]
fn ct_rte_07_full_mr16_protection_driven() {
    refusal_driven(C, "receipt_protection", "clear", "MR16");
}

/// The planted probes: an own tool in tool authoring with no grant reads an ambient resource, shows it and returns it.
/// Approval refuses it, so no trap is called and no planted data appears.
fn probe(requires: &str, body: &str, key: &str) {
    let source = src("m", &[requires], &[], body);
    let trial = ten(|| Trial::of(&source, &[]).run("go", Author::Own, Profile::ToolAuthoring));
    let refusal = trial.results[0].as_ref().map(|_| ()).unwrap_err();
    let expected = corpus_code("planted.json", key);
    assert_eq!(refusal.code, expected);
    assert!(refusal.detail.to_lowercase().contains(requires), "the refusal names the resource: {}", refusal.detail);
    unchanged(&trial);
    clean(&trial);
    assert!(trial.store.records.is_empty() && trial.engine.views.is_empty());
}

/// P01: the clock probe.
#[test]
fn ct_rte_07_full_p01_clock() {
    let body =
        "export fn go(t: Target, w: Word) -> Display<Nat> effects [clock, view] {\n    let leak = read_clock();\n    \
                let shown = publish_view(leak);\n    return leak;\n}\n";
    probe("clock", body, "P01");
}

/// P02: the file probe.
#[test]
fn ct_rte_07_full_p02_file() {
    let body = "export fn go(t: Target, w: Word) -> Text effects [file, view] {\n    \
                let leak = file_read<Text>(\"file_probe\");\n    let shown = publish_view(leak);\n    return leak;\n}\n";
    probe("file", body, "P02");
}

/// P03: the network probe. The IR draft has no network read, so the module only declares `network`; approval refuses
/// the declaration.
#[test]
fn ct_rte_07_full_p03_network() {
    probe("network", "export fn go(t: Target, w: Word) -> Text effects [] {\n    return \"none\";\n}\n", "P03");
}

/// P04: the process probe. The IR draft has no process call, so the module only declares `process`; approval refuses
/// the declaration.
#[test]
fn ct_rte_07_full_p04_process() {
    probe("process", "export fn go(t: Target, w: Word) -> Text effects [] {\n    return \"none\";\n}\n", "P04");
}

/// S01: dynamic code is refused at approval, before anything runs.
#[test]
fn ct_rte_07_full_s01_dynamic_code() {
    let source = root("export fn go(t: Target, w: Word) -> Int effects [] {\n    return 2;\n}\n");
    let trial = ten(|| {
        let mut trial = Trial::of(&source, &[]);
        let closure = trial.package.verify().unwrap();
        let mut program = closure.program.clone();
        let go = program.modules.iter_mut().flat_map(|m| m.functions.iter_mut()).find(|f| f.name == "go").unwrap();
        go.body = Expr::Call {
            callee: Callee::Unresolved { name: "evaluate".into(), result: QTy { ty: Ty::Int, display: false } },
            args: vec![Expr::Lit(Lit::Text("constant arithmetic".into()))],
        };
        let dynamic = Closure { program, ..closure };
        let approval = approve(&dynamic, entry("go", Author::Own, Profile::ToolAuthoring), contract(), checker());
        trial.results.push(approval.map(|_| unreachable!("dynamic code is never approved")));
        trial
    });
    let refusal = trial.results[0].as_ref().map(|_| ()).unwrap_err();
    assert_eq!(refusal.code, corpus_code("approval.json", "S01"));
    assert!(refusal.detail.contains("dynamic code"), "{}", refusal.detail);
    unchanged(&trial);
    clean(&trial);
}

/// S02: the dependency's bytes are replaced after approval; the locked digest refuses them.
#[test]
fn ct_rte_07_full_s02_dependency_substitution() {
    let constant = |n: i64| {
        module("constant", &[], &[], &format!("export fn value() -> Int effects [] {{\n    return {n};\n}}\n"))
    };
    let dep = constant(1);
    let source = module(
        "m",
        &[],
        &[("constant", dep.as_str())],
        "export fn go() -> Int effects [] {\n    return constant.value();\n}\n",
    );
    let trial = ten(|| {
        let mut trial = Trial::of(&source, &[&dep]);
        let closure = trial.package.verify().unwrap();
        let approval =
            approve(&closure, entry("go", Author::Own, Profile::ToolAuthoring), contract(), checker()).unwrap();
        let files = trial.package.files.clone().with("src/dep0.6cs", constant(2).as_bytes());
        let swapped = verify(&trial.package.manifest, &trial.package.lock, &files, &Identity::full());
        let result = swapped.and_then(|c| load(&c, &approval)).map(|_| unreachable!("substituted bytes never load"));
        trial.results.push(result);
        trial
    });
    let refusal = trial.results[0].as_ref().map(|_| ()).unwrap_err();
    assert_eq!((refusal.code, corpus_code("approval.json", "S02").as_str()), (DIGEST_MISMATCH, DIGEST_MISMATCH));
    unchanged(&trial);
    clean(&trial);
}

/// A budget key's case in `budgets.json`: its limits, its host event if any, and its expected outcome, code, used
/// amounts (none for an interruption), stop and examined items.
struct BudgetCase {
    limits: Amounts,
    event: Option<String>,
    outcome: String,
    code: String,
    used: Option<Amounts>,
    stop: String,
    examined: Option<Vec<usize>>,
}

fn budget_case(id: &str) -> BudgetCase {
    let json = provenance::parse(&std::fs::read(format!("{CORPUS}budgets.json")).unwrap()).expect("budgets.json reads");
    let Some(Json::List(cases)) = json.get("cases") else { panic!("the cases") };
    let case = cases.iter().find(|c| c.get("id").and_then(Json::as_text) == Some(id)).expect("the case");
    let amounts = |o: &Json| {
        let n = |k: &str| match o.get(k) {
            Some(Json::Int(v)) => *v as u128,
            other => panic!("{k}: {other:?}"),
        };
        Amounts { steps: n("steps"), kernel_cost: n("kernel_cost"), memory: n("memory"), output: n("output") }
    };
    let expected = case.get("expected").expect("its expectation");
    let text = |k: &str| expected.get(k).and_then(Json::as_text).expect(k).to_string();
    let examined = match expected.get("coverage").and_then(|c| c.get("examined")) {
        Some(Json::List(items)) => {
            Some(items.iter().map(|i| if let Json::Int(n) = i { *n as usize } else { panic!("{i:?}") }).collect())
        }
        _ => None,
    };
    BudgetCase {
        limits: amounts(case.get("limits").unwrap()),
        event: case.get("host").and_then(|h| h.get("event")).and_then(Json::as_text).map(str::to_string),
        outcome: text("outcome"),
        code: text("code"),
        used: match expected.get("used") {
            None | Some(Json::Null) => None,
            Some(used) => Some(amounts(used)),
        },
        stop: text("stop"),
        examined,
    }
}

/// Amounts in the meter's order: steps, kernel cost, memory, output.
fn spent(steps: u128, kernel_cost: u128, memory: u128, output: u128) -> Amounts {
    Amounts { steps, kernel_cost, memory, output }
}

/// A priced budget key (B01 to B07): `m.go`, an own tool in tool authoring with no argument, runs under the corpus's
/// limits moved by the price mapping (`Drafts`, CT-RTE-07's price mapping). `prefix` is the draft's charge of the
/// prefix the corpus admits, the entry's `call` included and memory at its peak. The corpus's charge of that prefix is
/// its used amounts less the step of its refused instruction `stop`, which stays charged unless the step is what
/// fails. The run gives the corpus's outcome and code, in the corpus's phase, stopped at `at`, the counterpart of
/// `stop`, and has used `used`, the corpus's used amounts mapped through the table. It installs no preview, writes
/// nothing and keeps the session fields.
fn priced(id: &str, source: &str, stop: &str, prefix: Amounts, at: &str, used: Amounts) -> Trial {
    let case = budget_case(id);
    assert_eq!(case.stop, stop, "the corpus's refused instruction");
    let phase = case.code.strip_prefix("KN7:budget.").expect("a budget code");
    let mut admitted = case.used.expect("the corpus's used amounts");
    if phase != "steps" {
        admitted.steps -= 1;
    }
    let limit = |corpus: u128, draft: u128, admitted: u128| corpus + draft - admitted;
    let l = case.limits;
    let limits = Amounts {
        steps: limit(l.steps, prefix.steps, admitted.steps),
        kernel_cost: limit(l.kernel_cost, prefix.kernel_cost, admitted.kernel_cost),
        memory: limit(l.memory, prefix.memory, admitted.memory),
        output: limit(l.output, prefix.output, admitted.output),
    };
    let trial =
        ten(|| Trial::of(source, &[]).call("go", Author::Own, Profile::ToolAuthoring, Vec::new(), Vec::new(), limits));
    let s = &trial.last().run.summary;
    assert_eq!((s.outcome.name(), s.code), (case.outcome.as_str(), Some(case.code.as_str())));
    let refused = s.stop.as_ref().expect("a stop");
    assert_eq!((refused.at.as_str(), refused.phase), (at, phase));
    assert_eq!(s.used, used, "the corpus's {:?}, mapped", case.used);
    assert!(trial.engine.previews.is_empty() && trial.store.records.is_empty());
    unchanged(&trial);
    clean(&trial);
    trial
}

/// B01: three `tick`s, the literals `()` of a record, under two steps. The entry's frame has a slot per field.
#[test]
fn ct_rte_07_full_b01_steps() {
    let source = root(
        "export fn go() -> record { a: Unit, b: Unit, c: Unit } effects [] {\n    \
         return record { a = (), b = (), c = () };\n}\n",
    );
    // The entry's `call` and the two literals the corpus admits as `tick`s; the third literal's step fails.
    let admitted = spent(3, 0, frame_size(3), 0);
    let trial = priced("B01", &source, "r1/body/2:before_step", admitted, "m.go:2#value", admitted);
    assert!(trial.engine.views.is_empty());
}

/// B02: two reads of the counts, 3 each (`fixture.json`), under a kernel cost of 5. A read reserves the host's
/// result bound, 4096 in the synthetic engine, so the memory peaks at the frame and that bound. The engine serves
/// the corpus's examined row only.
#[test]
fn ct_rte_07_full_b02_kernel_cost() {
    let source = root(
        "export fn go() -> record { a: $INTS, b: $INTS } effects [snapshot] {\n    \
         return record { a = read_snapshot<$INTS>(\"counts\"), b = read_snapshot<$INTS>(\"counts\") };\n}\n",
    );
    let peak = frame_size(2) + 4096;
    let trial =
        priced("B02", &source, "r1/body/1:before_kernel", spent(2, 3, peak, 0), "m.go:1#kernel", spent(3, 3, peak, 0));
    assert_eq!(trial.engine.served, ["counts"]);
    assert!(trial.engine.views.is_empty());
}

/// B03: two retained 16-byte texts, the literals `"abcd"` of four scalars (DET.57), under a memory of 31.
#[test]
fn ct_rte_07_full_b03_memory() {
    let source = root(
        "export fn go() -> record { a: Text, b: Text } effects [] {\n    \
         return record { a = \"abcd\", b = \"abcd\" };\n}\n",
    );
    let peak = frame_size(2) + 16;
    priced("B03", &source, "r1/body/1:before_memory", spent(2, 0, peak, 0), "m.go:1#value", spent(3, 0, peak, 0));
}

/// B04: two emissions of the text "abcd" under an output of 11. A publication's unit is the JSON and a line feed,
/// 7 bytes (FP.5): output, encoding work and a buffer released after it, while the literal holds 16 bytes. The first
/// view is published and the second refused. The draft's five steps are the corpus's two, one for the entry's `call`
/// and one for each literal.
#[test]
fn ct_rte_07_full_b04_output() {
    let source = root(
        "export fn go() -> record { a: Unit, b: Unit } effects [view] {\n    \
         return record { a = publish_view(\"abcd\"), b = publish_view(\"abcd\") };\n}\n",
    );
    let peak = frame_size(2) + 16 + 7;
    let trial = priced(
        "B04",
        &source,
        "r1/body/1:before_output",
        spent(3, 7, peak, 7),
        "m.go:1#publish_view",
        spent(5, 7, peak, 7),
    );
    assert_eq!(trial.engine.views, [text("abcd")]);
}

/// B05 and B06: one read of the counts, the run's first operation after the entry's `call`.
const COUNTS: &str = "export fn go() -> $INTS effects [snapshot] {\n    return read_snapshot<$INTS>(\"counts\");\n}\n";

/// B05: the step passes and stays charged; the kernel cost of 3 fails under 2 (DET.56). No row is served.
#[test]
fn ct_rte_07_full_b05_kernel_cost_after_step() {
    let entry = spent(1, 0, frame_size(0), 0);
    let trial = priced(
        "B05",
        &root(COUNTS),
        "r1/body/0:before_kernel",
        entry,
        "m.go:root#kernel",
        spent(2, 0, frame_size(0), 0),
    );
    assert!(trial.engine.served.is_empty() && trial.engine.views.is_empty());
}

/// B06: no step for the read; a failed step debits nothing. No row is served.
#[test]
fn ct_rte_07_full_b06_no_step() {
    let entry = spent(1, 0, frame_size(0), 0);
    let trial = priced("B06", &root(COUNTS), "r1/body/0:before_step", entry, "m.go:root#kernel", entry);
    assert!(trial.engine.served.is_empty() && trial.engine.views.is_empty());
}

/// B07: a recursion under a memory of 48; the corpus refuses its fourth 16-byte frame. A function with no slot has a
/// 16-byte frame here too, the entry's included, so the draft refuses the same call.
#[test]
fn ct_rte_07_full_b07_recursion() {
    let source = root(
        "fn recurse() -> Int effects [] {\n    return recurse();\n}\n\n\
         export fn go() -> Int effects [] {\n    return recurse();\n}\n",
    );
    let peak = 4 * frame_size(0);
    let at = "m.go:root > m.recurse:root > m.recurse:root > m.recurse:root#call";
    let trial = priced("B07", &source, "r1/recurse:before_frame_4", spent(4, 0, peak, 0), at, spent(5, 0, peak, 0));
    assert!(trial.engine.views.is_empty());
}

/// B08 and B09: an own tool in tool authoring, under the corpus's limits, whose loop reads one host row per item; the
/// solver cancels inside the first row. The run gives the corpus's outcome and code, stopped in the `interruption`
/// phase at the operation after that row, `at`. The corpus's examined items are the rows the engine served, and none
/// after the cancel. The run installs no preview, publishes and writes nothing, and keeps the session fields. As the
/// corpus's repetition rule asks of a cancel, this checks the typed reason and the truthful partial coverage.
fn interrupted(id: &str, source: &str, row: &str, served: &[&str], at: &str) {
    let case = budget_case(id);
    assert_eq!((case.event.as_deref(), case.used), (Some("cancel"), None));
    let trial = ten(|| Trial::of(source, &[]).cancelled_after("go", row, case.limits));
    let s = &trial.last().run.summary;
    assert_eq!((s.outcome.name(), s.code), (case.outcome.as_str(), Some(case.code.as_str())));
    let stop = s.stop.as_ref().expect("a stop");
    assert_eq!((stop.at.as_str(), stop.phase), (at, "interruption"), "{:?}", case.stop);
    assert_eq!(trial.engine.served, served);
    assert_eq!(case.examined, Some((0..served.len()).collect()));
    assert!(trial.engine.views.is_empty() && trial.store.records.is_empty());
    unchanged(&trial);
    clean(&trial);
}

/// B08: the loop over ten items, one `tick` each; the cancel comes inside the first tick. The run stops at the loop's
/// next visit after that body, the corpus's "first loop continuation after body tick".
#[test]
fn ct_rte_07_full_b08_cancel_in_a_loop() {
    let source = root(
        "export fn go() -> Nat effects [snapshot] {\n    \
         return repeat(10, 0, fn(done: Nat) -> Nat {\n        return read_snapshot<Nat>(\"tick\");\n    });\n}\n",
    );
    interrupted("B08", &source, "tick", &["tick"], "m.go:root#body:0 > m.go:root#visit");
}

/// B09: the batch check of the words `a`, `b` and `c`, one word per chunk; the cancel comes inside the first chunk's
/// read. The run stops at the `push` that would keep that chunk's check, the corpus's "after first kernel word chunk".
#[test]
fn ct_rte_07_full_b09_cancel_in_a_batch() {
    let source = root(
        "export fn go() -> List<$CHECK> effects [snapshot] {\n    \
         let words = push(push(push(list<Word>(), word[1]), word[2]), word[3]);\n    \
         return repeat(3, list<$CHECK>(), fn(done: List<$CHECK>) -> List<$CHECK> {\n        \
         return push(done, read_snapshot<$CHECK>(\"word_check\", index(words, length(done))));\n    });\n}\n",
    );
    assert_eq!(named(&[A, B, C]), ["a", "b", "c"], "the corpus's words");
    interrupted("B09", &source, "word_check", &["word_check/[1]"], "m.go:1#body:0 > m.go:1.2#push");
}

//! CT-RTE-07, the 600-cell-Full subset, source leg (RTE.46; LX-3's frozen corpus `lx3-neutral-v2`).
//!
//! Each realised key is re-expressed in `.6cs` under the fixture mapping that the crate documentation records
//! (`Drafts`, CT-RTE-07's fixture mapping), and runs ten times from a fresh fixture: verified, approved through
//! `c600-check`, loaded and run through `c600-run`'s boundary. The ten runs give one SHA-256 over the canonical results
//! and stop reasons, the engine side and the store. An allowed key completes with the corpus's value; a refused key
//! gives the corpus's expected code at the refused operation, installs no preview, keeps the session fields and the
//! existing workspace provenance, and reads no planted data. A realised budget key (B05, B06) gives the corpus's
//! outcome, code and used amounts; its stop is the draft's operation path. [`FALSE_REFUSALS`] lists where this leg
//! differs from the corpus; every refused key gives the corpus's code (P01 to P03 since the owner decision of 16
//! November 2026 under PHS.52). [`LEFT`] lists the keys not realised, with the reason.
//!
//! Evidence kind: source and synthetic. The fixture is LX-3's fake data, not Full's mechanics; nothing here measures
//! time or performance.

mod support;

use std::collections::BTreeSet;

use c600_digest::sha256;
use c600_ir::{Callee, Expr, Lit, QTy, Ty};
use c600_module::code::{DIGEST_MISMATCH, NOT_GRANTED};
use c600_module::{approve, load, verify, Closure, Identity, Outcome, Refusal, RunRequest};
use c600_run::boundary::{Answer, Place, Store};
use c600_run::interp::Options;
use c600_run::meter::{Amounts, Outcome as Ended};
use c600_run::provenance::{self, field, hex, Author, Json, Profile, INCOMPATIBLE};
use c600_run::value::Value;
use support::*;

const CORPUS: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/../../work/experiments/language-lx/lx3-sandbox-flow/corpus/");
/// The raw SHA-256 of the frozen corpus's `FREEZE.json` (`lx3-neutral-v2`).
const FREEZE_SHA256: &str = "d7eff17503ee95d45ddca0efe38ec3bb09418b0248a6279afe2dce0a90ea6627";

/// The keys realised here, one test each.
const REALISED: [&str; 36] = [
    "A01", "A02", "A03", "A04", "A05", "A06", "A07", "A09", "A11", "A12", "A13", "A15", "A20", "A24", "A25", "A33",
    "A34", "A35", "B05", "B06", "MR03", "MR04", "MR07", "MR08", "MR09", "MR10", "MR11", "MR12", "MR15", "MR16", "P01",
    "P02", "P03", "P04", "S01", "S02",
];

const M5: &str = "waits for M5: a jumbling key";
const M6: &str = "waits for M6: a recorded pick or accumulation across runs needs the session store";
const ROW_ARGUMENT: &str = "not expressible: a row whose argument is the run's input; a snapshot query is a literal";
const HOST_EVENT: &str = "not realised: a host event (a cancel, a deadline or a lost runtime) inside a loop or a \
                          batch; the IR draft has no loop, and this leg runs no timing and no runtime process";
const COMMAND: &str =
    "not realised: the budget declaration and the host ceiling belong to the command layer, a non-goal";

/// The keys left, with the reason. The amounts of B01 to B04 and B07 follow LX-3's experiment-only counting fixture
/// (`profile.json`); lp1-exec-draft-1 prices operations, encodings and memory with its own table, and the corpus asks
/// for a candidate's mapping to be pinned before comparison (`kernel-stub.md`).
const LEFT: [(&str, &str); 39] = [
    ("MR01", M5),
    ("MR02", M5),
    ("MR05", M5),
    ("MR06", M5),
    ("A31", M5),
    ("A32", M5),
    ("MR17", M6),
    ("MR18", M6),
    ("MR19", M6),
    ("MR20", M6),
    ("A29", M6),
    ("A30", M6),
    ("MR13", "not expressible: the IR draft has no early stop (LX-3 code gap `stop_emission`)"),
    ("MR14", "not expressible: the IR draft has no early stop (LX-3 code gap `stop_emission`)"),
    ("A08", "not expressible: the IR draft has no word length"),
    ("A10", "not expressible: the IR draft has no exact multiplication or remainder"),
    ("A14", ROW_ARGUMENT),
    ("A16", ROW_ARGUMENT),
    ("A17", "not expressible: the IR draft has no word inverse"),
    ("A18", "not expressible: the IR draft has no word concatenation"),
    ("A19", ROW_ARGUMENT),
    ("A21", ROW_ARGUMENT),
    ("A22", ROW_ARGUMENT),
    ("A23", ROW_ARGUMENT),
    ("A26", ROW_ARGUMENT),
    ("A27", ROW_ARGUMENT),
    ("A28", ROW_ARGUMENT),
    ("B01", "not realised: LX-3's `tick` has no IR counterpart; the draft charges a step to every operation it runs"),
    ("B02", "not realised: the draft also prices the encoding and a row's memory, so the used amounts differ"),
    ("B03", "not realised: LX-3's `retain` of a logical byte array has no IR counterpart"),
    ("B04", "not realised: LX-3 charges an emitted `\"abcd\"` as 6 bytes; the draft's encoding adds a line feed"),
    ("B07", "not realised: LX-3's probe-only 16-byte frame per call; the draft prices a frame with its own table"),
    ("B08", HOST_EVENT),
    ("B09", HOST_EVENT),
    ("B10", HOST_EVENT),
    ("B11", HOST_EVENT),
    ("B12", "not realised: the host's row cache and its variants are not part of this crate"),
    ("B13", COMMAND),
    ("B14", COMMAND),
];

/// Allowed keys this leg refuses. A25 reads the counts and previews a manually named word in one entry; separation
/// refuses the preview, which the corpus allows ("Separation may falsely refuse the combination").
const FALSE_REFUSALS: [&str; 1] = ["A25"];

const INTS: &str = "record { c0: Int, c1: Int, c2: Int }";
const BOOLS: &str = "record { c0: Bool, c1: Bool, c2: Bool }";
const STATE: &str = "record { s0: Int, s1: Int, s2: Int, s3: Int }";
const NU: &str = "record { i0: Int, i1: Int }";
const BITS: &str = "record { i0: Int, i1: Int, i2: Int }";
const PAIR: &str = "record { count: Int, target: Target }";

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
    fn exec(
        mut self,
        function: &str,
        author: Author,
        profile: Profile,
        inputs: Vec<Vec<u8>>,
        limits: Amounts,
    ) -> Trial {
        let id = format!("r{}", self.results.len() + 1);
        let args = vec![Value::target("c0"), Value::word(&[A])];
        let result = ready(&self.package, entry(function, author, profile)).and_then(|r| {
            let request =
                RunRequest { run: &id, base: "rev-7", args, inputs, pins: pins(), options: Options::new(limits) };
            r.run(request, &mut self.engine, &mut self.store)
        });
        self.results.push(result);
        self
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
    assert!(FALSE_REFUSALS.iter().all(|k| realised.contains(k)));
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

/// A budget key's case in `budgets.json`: its limits, and its expected outcome, code, used amounts and stop.
struct BudgetCase {
    limits: Amounts,
    outcome: String,
    code: String,
    used: Amounts,
    stop: String,
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
    BudgetCase {
        limits: amounts(case.get("limits").unwrap()),
        outcome: text("outcome"),
        code: text("code"),
        used: amounts(expected.get("used").unwrap()),
        stop: text("stop"),
    }
}

/// B05 and B06: one read of the counts by an own tool in tool authoring, under the corpus's interacting limits. The
/// read is the run's first operation and the row costs 3 (`fixture.json`). The outcome, code and used amounts are the
/// corpus's: a failed step debits nothing, and a step that passes stays charged when the kernel cost fails (DET.56).
/// The stop is the draft's path of that operation, `entry#read`, with the phase the corpus's `before_*` names; the
/// corpus's coverage is not part of the draft's result.
fn interacting(id: &str, phase: &str) {
    let source = root(
        "export fn go(t: Target, w: Word) -> $INTS effects [snapshot] {\n    \
         return read_snapshot<$INTS>(\"counts\");\n}\n",
    );
    let case = budget_case(id);
    let inputs = || vec![manual(), manual()];
    let trial = ten(|| Trial::of(&source, &[]).exec("go", Author::Own, Profile::ToolAuthoring, inputs(), case.limits));
    let s = &trial.last().run.summary;
    assert_eq!((s.outcome.name(), s.code), (case.outcome.as_str(), Some(case.code.as_str())));
    assert_eq!(s.used, case.used);
    let stop = s.stop.as_ref().expect("a stop");
    assert_eq!((stop.at.as_str(), stop.phase), ("entry#read", phase));
    let before = match phase {
        "steps" => "step",
        "kernel_cost" => "kernel",
        other => other,
    };
    assert_eq!(case.stop, format!("r1/body/0:before_{before}"));
    assert!(trial.engine.views.is_empty() && trial.store.records.is_empty());
    unchanged(&trial);
    clean(&trial);
}

#[test]
fn ct_rte_07_full_b05_kernel_cost_after_step() {
    interacting("B05", "kernel_cost");
}

#[test]
fn ct_rte_07_full_b06_no_step() {
    interacting("B06", "steps");
}

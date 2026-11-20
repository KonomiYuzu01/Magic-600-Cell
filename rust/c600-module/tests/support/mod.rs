//! Shared fixtures of the CT-RTE-07 and CT-RTE-08 tests: the synthetic engine, the policy pins, and helpers that
//! write a manifest, resolve and verify a closure, approve one entry and run it through the boundary.
//!
//! The engine serves LX-3's fixed fake data (`fixture.json` of the frozen corpus) under the fixture mapping that the
//! crate documentation records: letters `a, b, c, d, D` are `+1, +2, +3, +4, -4`, fixed-order lists are records with
//! fields `c0..c2`, `i0..i2` and `s0..s3`, a row with a literal argument is the query `row/argument`, and the rows
//! `word_check` and `setup` take their argument, the run's input, through `read_snapshot<T>("row", x)`. It is
//! synthetic data, not Full's mechanics.

#![allow(dead_code)]

use std::collections::BTreeMap;
use std::sync::Arc;

use c600_check::{Grant, Policy};
use c600_digest::sha256;
use c600_module::{
    approve, load, resolve, verify, Approval, Closure, Dependency, Entry, Files, Identity, Manifest, Outcome, Ready,
    Refusal, RunRequest,
};
use c600_run::boundary::{Pins, Place, Store};
use c600_run::interp::{CancelFlag, Host, Options, Request, RowTariff, Sink};
use c600_run::meter::code::NOT_GRANTED;
use c600_run::meter::{Amounts, Interruption};
use c600_run::provenance::{self, hex, Author, Json, Pin, Profile};
use c600_run::value::Value;
use c600_syntax::ast::Item;

pub const WIDE: Amounts = Amounts { steps: 1 << 40, kernel_cost: 1 << 40, memory: 1 << 40, output: 1 << 40 };
pub const EDITION: &str = "6cs-draft-1";
/// The engine's refusal of a word that moves the protected slot 3 (fixture `protected_slots`).
pub const PROTECTION: &str = "KN2:protection.piece_would_move";
/// The fixture letters: `a, b, c, d, D`.
pub const A: i32 = 1;
pub const B: i32 = 2;
pub const C: i32 = 3;

/// The fixture's permutations (`fixture.json`, `permutations`): letter `x` sends slot `i` to `x[i]`.
pub const PERMUTATIONS: [(&str, [usize; 4]); 5] =
    [("a", [1, 0, 2, 3]), ("b", [0, 2, 1, 3]), ("c", [0, 1, 3, 2]), ("d", [1, 2, 3, 0]), ("D", [3, 0, 1, 2])];
/// The IR's letters under the fixture mapping: `+1..+4` are `a..d`, and `-n` is the inverse the fixture names for
/// letter `n` (`inverses`).
pub const LETTERS: [(i32, &str); 8] =
    [(1, "a"), (2, "b"), (3, "c"), (4, "d"), (-1, "a"), (-2, "b"), (-3, "c"), (-4, "D")];
/// The fixture's setups for the targets the solver names (`setups`), in the IR's letters.
pub const SETUPS: [(&str, &[&[i32]]); 3] =
    [("c0", &[&[], &[A]]), ("c1", &[&[B], &[A, B]]), ("c2", &[&[A, B], &[B, A]])];
/// The fixture's protected slot (`protected_slots`).
pub const PROTECTED_SLOT: usize = 3;

/// The synthetic engine side of a session: the fixture's revision, labels and pending preview, the previews it
/// installed, the views it received, the rows it served, and a trap on every ambient read that counts the call and
/// would hand out planted data. A test may have it cancel the run inside a row it serves.
#[derive(Clone, Debug)]
pub struct Engine {
    pub rows: BTreeMap<String, Value>,
    pub revision: u64,
    pub labels: Vec<u32>,
    pub pending: (Vec<i32>, String),
    pub previews: Vec<(Vec<i32>, String)>,
    /// The menu-word previews it built (LB1.8; LP2 draft): the letters `[cap, item]` and the target.
    pub menu_previews: Vec<(Vec<[u32; 2]>, String)>,
    pub views: Vec<Vec<u8>>,
    pub traps: u32,
    /// The rows it served, in order: a literal row by its query, a row with an argument as `row/argument`.
    pub served: Vec<String>,
    /// A row inside which it cancels the run by the solver's request, and the flag the run observes.
    pub cancel_after: Option<(String, Arc<CancelFlag>)>,
}

fn row(fields: &[(&str, i64)]) -> Value {
    Value::record(fields.iter().map(|(name, n)| (*name, Value::int(*n))).collect())
}

impl Engine {
    /// The fixture's state (`fixture.json`): its rows, revision 7, labels `[1, 0, 2, 3]` and the pending preview
    /// `b` at `c1`.
    pub fn fixture() -> Engine {
        let mut rows = BTreeMap::new();
        rows.insert("counts".into(), row(&[("c0", 2), ("c1", 6), ("c2", 6)]));
        rows.insert("floors".into(), row(&[("c0", 0), ("c1", 2), ("c2", 2)]));
        let blocked = [("c0", false), ("c1", true), ("c2", false)];
        rows.insert("blocked".into(), Value::record(blocked.iter().map(|(n, b)| (*n, Value::Bool(*b))).collect()));
        rows.insert("snapshot".into(), row(&[("s0", 1), ("s1", 0), ("s2", 2), ("s3", 3)]));
        rows.insert("nu2".into(), row(&[("i0", 1), ("i1", 0)]));
        rows.insert("nu5".into(), row(&[("i0", 0), ("i1", 3)]));
        rows.insert("bits".into(), row(&[("i0", 1), ("i1", 0), ("i2", 1)]));
        // `score` of the literal words `a`, `b`, `c` on labels [1, 0, 2, 3]: the end labels [0, 1, 2, 3], [1, 2, 0, 3]
        // and [1, 0, 3, 2] differ from their indexes in 0, 3 and 4 places (`kernel-stub.md`).
        for (word, score) in [("a", 0), ("b", 3), ("c", 4)] {
            rows.insert(format!("score/{word}"), Value::int(score));
        }
        Engine {
            rows,
            revision: 7,
            labels: vec![1, 0, 2, 3],
            pending: (vec![B], "c1".into()),
            previews: Vec::new(),
            menu_previews: Vec::new(),
            views: Vec::new(),
            traps: 0,
            served: Vec::new(),
            cancel_after: None,
        }
    }

    /// Serves a row: logs it, and cancels the run when it is the row the test named.
    fn serve(&mut self, row: String) {
        if let Some((after, flag)) = &self.cancel_after {
            if row == *after || row.starts_with(&format!("{after}/")) {
                flag.cancel(Interruption::ByUser);
            }
        }
        self.served.push(row);
    }

    /// The row `word_check` of a word on the fixture's labels (`kernel-stub.md`): the word applied to the slots in
    /// chronological order, its end labels, the slots it moves, whether the protected slot stays, and the word.
    pub fn word_check(&self, word: &[i32]) -> Option<Value> {
        let mut q = [0usize, 1, 2, 3];
        for letter in word {
            let name = LETTERS.iter().find(|(l, _)| l == letter)?.1;
            let p = PERMUTATIONS.iter().find(|(n, _)| *n == name)?.1;
            q = [q[p[0]], q[p[1]], q[p[2]], q[p[3]]];
        }
        let labels = ["s0", "s1", "s2", "s3"].iter().zip(q).map(|(s, i)| (*s, Value::int(i64::from(self.labels[i]))));
        let moved = (0..4).filter(|&i| q[i] != i).map(|i| Value::int(i as i64)).collect();
        let coverage = vec![("complete", Value::Bool(true)), ("letters", Value::int(word.len() as i64))];
        Some(Value::record(vec![
            ("coverage", Value::record(coverage)),
            ("end_labels", Value::record(labels.collect())),
            ("moved_slots", Value::list(moved)),
            ("protected", Value::Bool(q[PROTECTED_SLOT] == PROTECTED_SLOT)),
            ("word", Value::word(word)),
        ]))
    }

    /// A digest of everything a run could change on the engine side.
    pub fn digest(&self) -> [u8; 32] {
        let word = |w: &[i32]| Json::List(w.iter().map(|l| Json::Int(i64::from(*l))).collect());
        let preview = |(w, t): &(Vec<i32>, String)| Json::object([("target", Json::text(t)), ("word", word(w))]);
        let state = Json::object([
            ("labels", Json::List(self.labels.iter().map(|l| Json::Int(i64::from(*l))).collect())),
            ("pending", preview(&self.pending)),
            ("previews", Json::List(self.previews.iter().map(preview).collect())),
            ("revision", Json::Int(self.revision as i64)),
            ("traps", Json::Int(i64::from(self.traps))),
            ("views", Json::List(self.views.iter().map(|v| Json::Text(String::from_utf8_lossy(v).into())).collect())),
        ]);
        sha256(&provenance::canonical(&state))
    }

    /// Whether a planted value appears anywhere on the engine side.
    pub fn holds_planted(&self) -> bool {
        self.views.iter().any(|v| planted_in(v))
    }
}

/// The word a row's argument encodes, when it is the canonical encoding of a word.
pub fn word_of(arg: &[u8]) -> Option<Vec<i32>> {
    let text = std::str::from_utf8(arg).ok()?.trim_end_matches('\n');
    let inner = text.strip_prefix('[')?.strip_suffix(']')?;
    let word: Vec<i32> =
        if inner.is_empty() { Vec::new() } else { inner.split(',').map(|l| l.parse().ok()).collect::<Option<_>>()? };
    (encoded(&Value::word(&word)) == arg).then_some(word)
}

/// The setups of the target a row's argument encodes.
pub fn setups_of(arg: &[u8]) -> Option<Value> {
    let (_, setups) = SETUPS.iter().find(|(t, _)| encoded(&Value::target(t)) == arg)?;
    Some(Value::list(setups.iter().map(|w| Value::word(w)).collect()))
}

/// A row with an argument as the engine logs it.
fn served_name(query: &str, arg: &[u8]) -> String {
    format!("{query}/{}", String::from_utf8_lossy(arg).trim_end_matches('\n'))
}

/// The planted data of LX-3's probes (`planted.json`); a trap read would return it.
pub const PLANTED: [&str; 4] =
    ["LX3_CLOCK_90123", "LX3_FILE_NEVER_RETURN", "LX3_NETWORK_NEVER_RETURN", "LX3_PROCESS_NEVER_RETURN"];

pub fn planted_in(bytes: &[u8]) -> bool {
    let text = String::from_utf8_lossy(bytes);
    PLANTED.iter().any(|p| text.contains(p))
}

impl Host for Engine {
    /// A row costs its `fixture.json` cost (`score` and `word_check` of a word cost one plus the word's length; LX-3's
    /// `tick` costs 1); a request the fixture prices nowhere costs 9. A `word_check` whose argument is no word, or a row
    /// with an argument the fixture does not have, is not granted.
    fn tariff(&mut self, request: &Request) -> Result<RowTariff, &'static str> {
        let kernel_cost = match request {
            Request::Snapshot { query, .. } => match *query {
                "counts" | "floors" | "blocked" | "bits" => 3,
                "nu2" | "nu5" => 2,
                "snapshot" => 4,
                "tick" => 1,
                q if q.starts_with("score/") => 1 + (q.len() - "score/".len()) as u128,
                _ => 9,
            },
            Request::SnapshotWith { query: "word_check", arg, .. } => {
                1 + word_of(arg).ok_or(NOT_GRANTED)?.len() as u128
            }
            Request::SnapshotWith { query: "setup", .. } => 4,
            Request::SnapshotWith { .. } => return Err(NOT_GRANTED),
            _ => 9,
        };
        Ok(RowTariff { kernel_cost, result_bound: 4096 })
    }

    fn perform(&mut self, request: &Request) -> Result<Value, &'static str> {
        match request {
            Request::Snapshot { query: "tick", .. } => {
                let ticks = self.served.iter().filter(|r| *r == "tick").count();
                self.serve("tick".into());
                Ok(Value::int(ticks as i64 + 1))
            }
            Request::Snapshot { query, .. } => {
                let value = self.rows.get(*query).cloned().ok_or(NOT_GRANTED)?;
                self.serve(query.to_string());
                Ok(value)
            }
            Request::SnapshotWith { query, arg, .. } => {
                let value = match *query {
                    "word_check" => word_of(arg).and_then(|w| self.word_check(&w)),
                    "setup" => setups_of(arg),
                    _ => None,
                };
                let value = value.ok_or(NOT_GRANTED)?;
                self.serve(served_name(query, arg));
                Ok(value)
            }
            Request::Propose { word, target, .. } => {
                if word.iter().any(|l| l.abs() >= C) {
                    return Err(PROTECTION);
                }
                self.pending = (word.to_vec(), target.to_string());
                self.previews.push(self.pending.clone());
                Ok(Value::receipt(vec![
                    ("base", Value::text("rev-7")),
                    ("effect", Value::text("moves 2 slots")),
                    ("id", Value::text(&format!("preview-{}", self.previews.len()))),
                    ("protection", Value::tag("clear")),
                ]))
            }
            Request::ProposeMenu { letters, target, .. } => {
                self.menu_previews.push((letters.to_vec(), target.to_string()));
                Ok(Value::receipt(vec![
                    ("base", Value::text("rev-7")),
                    ("effect", Value::text("moves 2 slots")),
                    ("id", Value::text(&format!("menu-preview-{}", self.menu_previews.len()))),
                    ("protection", Value::tag("clear")),
                ]))
            }
            Request::Clock => {
                self.traps += 1;
                Ok(Value::text(PLANTED[0]))
            }
            Request::FileRead { .. } | Request::Declassified { .. } => {
                self.traps += 1;
                Ok(Value::text(PLANTED[1]))
            }
        }
    }

    fn sink(&mut self, sink: &Sink, _: &Value, bytes: &[u8]) -> Result<(), &'static str> {
        if matches!(sink, Sink::View) {
            self.views.push(bytes.to_vec());
        }
        Ok(())
    }
}

/// FP.18's fixture capability contract, held by the policy in these tests.
pub fn contract() -> Pin {
    Pin::of("fixture-1", b"p3 fixture capability contract 1\n")
}

pub fn checker() -> Pin {
    Pin::of("m1-draft", b"c600-check fixture\n")
}

pub fn pins() -> Pins {
    Pins { contracts: vec![contract()], checkers: vec![checker()] }
}

/// A hand-entered value's block.
pub fn manual() -> Vec<u8> {
    provenance::canonical(&provenance::manual(&contract()))
}

pub fn unhex(s: &str) -> [u8; 32] {
    let mut out = [0u8; 32];
    for (i, byte) in out.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&s[2 * i..2 * i + 2], 16).expect("hex");
    }
    out
}

/// The source of a module named `name` with a header that requires `requires`, the imports of `deps` (alias and
/// source text), and a body.
pub fn module(name: &str, requires: &[&str], deps: &[(&str, &str)], body: &str) -> String {
    let mut text = format!("edition \"{EDITION}\";\nmodule {name} requires [{}];\n\n", requires.join(", "));
    for (alias, source) in deps {
        text.push_str(&format!("import {alias} \"{}\";\n", hex(&sha256(source.as_bytes()))));
    }
    if !deps.is_empty() {
        text.push('\n');
    }
    text.push_str(body);
    text
}

/// The manifest a root source declares: its name, edition, `requires`, direct imports and Full's identity.
pub fn manifest_for(root: &[u8]) -> Manifest {
    let parsed = c600_syntax::parse(root);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let header = parsed.module.as_ref().expect("a module header");
    let dependencies = parsed
        .items
        .iter()
        .filter_map(|item| match item {
            Item::Import(i) => Some(Dependency { alias: i.alias.text.clone(), source: unhex(&i.digest) }),
            _ => None,
        })
        .collect();
    Manifest {
        module: header.name.text.clone(),
        edition: EDITION.into(),
        source: sha256(root),
        dependencies,
        capabilities: header.requires.iter().map(|r| r.text.clone()).collect(),
        boundary: None,
        principle: None,
        supports: vec![Identity::full()],
    }
}

/// A root module with its local files, its manifest bytes and the lock resolved for them.
#[derive(Clone, Debug)]
pub struct Package {
    pub manifest: Vec<u8>,
    pub lock: Vec<u8>,
    pub files: Files,
}

/// Writes the manifest of `root`, places it and its dependencies under `src/`, and resolves the lock.
pub fn package(root: &str, deps: &[&str]) -> Package {
    let mut files = Files::new().with("src/root.6cs", root.as_bytes());
    for (i, dep) in deps.iter().enumerate() {
        files = files.with(&format!("src/dep{i}.6cs"), dep.as_bytes());
    }
    let manifest = manifest_for(root.as_bytes()).bytes();
    let lock = resolve(&manifest, &files).unwrap_or_else(|r| panic!("{r:?}")).bytes();
    Package { manifest, lock, files }
}

impl Package {
    pub fn verify(&self) -> Result<Closure, Refusal> {
        verify(&self.manifest, &self.lock, &self.files, &Identity::full())
    }
}

/// The entry `m.<function>` under an author class and profile, with no grant and the interim policy.
pub fn entry(function: &str, author: Author, profile: Profile) -> Entry {
    Entry {
        module: "m".into(),
        function: function.into(),
        author,
        profile,
        grant: Grant::none(),
        policy: Policy::interim(),
    }
}

/// Verifies a package and approves one entry.
pub fn approved(package: &Package, entry: Entry) -> Result<(Closure, Approval), Refusal> {
    let closure = package.verify()?;
    let approval = approve(&closure, entry, contract(), checker())?;
    Ok((closure, approval))
}

/// Verifies, approves and loads one entry.
pub fn ready(package: &Package, entry: Entry) -> Result<Ready, Refusal> {
    let (closure, approval) = approved(package, entry)?;
    load(&closure, &approval)
}

/// Runs a loaded entry as run `run` on base `rev-7`.
pub fn run(
    ready: &Ready,
    run: &str,
    args: Vec<Value>,
    inputs: Vec<Vec<u8>>,
    engine: &mut Engine,
    store: &mut Store,
) -> Result<Outcome, Refusal> {
    let request = RunRequest { run, base: "rev-7", args, inputs, pins: pins(), options: Options::new(WIDE) };
    ready.run(request, engine, store)
}

/// The canonical encoding of a value, as the run's output bytes.
pub fn encoded(value: &Value) -> Vec<u8> {
    let mut bytes = Vec::new();
    c600_run::encode::encode(value, &mut bytes).expect("a canonical value");
    bytes
}

/// The store's records as canonical JSON: place, value bytes and block digest.
pub fn store_json(store: &Store) -> Json {
    let records = store.records.iter().map(|(place, s)| {
        let place = match place {
            Place::Workspace(n) => Json::object([("workspace", Json::text(n))]),
            Place::File(n) => Json::object([("file", Json::text(n))]),
        };
        Json::object([
            ("block", Json::Text(hex(&sha256(&s.block)))),
            ("bytes", Json::Text(String::from_utf8_lossy(&s.bytes).into())),
            ("place", place),
        ])
    });
    Json::List(records.collect())
}

/// Whether a planted value appears in a run's result, output, shell, captures or in the store.
pub fn leaks(results: &[Result<Outcome, Refusal>], store: &Store) -> bool {
    let in_results = results.iter().any(|r| match r {
        Ok(o) => planted_in(&o.canonical()),
        Err(e) => planted_in(e.detail.as_bytes()),
    });
    in_results || planted_in(&provenance::canonical(&store_json(store)))
}

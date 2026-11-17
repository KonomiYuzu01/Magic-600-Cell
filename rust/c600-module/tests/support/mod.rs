//! Shared fixtures of the CT-RTE-07 and CT-RTE-08 tests: the synthetic engine, the policy pins, and helpers that
//! write a manifest, resolve and verify a closure, approve one entry and run it through the boundary.
//!
//! The engine serves LX-3's fixed fake data (`fixture.json` of the frozen corpus) under the fixture mapping that the
//! crate documentation records: letters `a, b, c, d, D` are `+1, +2, +3, +4, -4`, fixed-order lists are records with
//! fields `c0..c2`, `i0..i2` and `s0..s3`, and a row with a literal argument is the query `row/argument`. It is
//! synthetic data, not Full's mechanics.

#![allow(dead_code)]

use std::collections::BTreeMap;

use c600_check::{Grant, Policy};
use c600_digest::sha256;
use c600_module::{
    approve, load, resolve, verify, Approval, Closure, Dependency, Entry, Files, Identity, Manifest, Outcome, Ready,
    Refusal, RunRequest,
};
use c600_run::boundary::{Pins, Place, Store};
use c600_run::interp::{Host, Options, Request, RowTariff, Sink};
use c600_run::meter::code::NOT_GRANTED;
use c600_run::meter::Amounts;
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

/// The synthetic engine side of a session: the fixture's revision, labels and pending preview, the previews it
/// installed, the views it received, and a trap on every ambient read that counts the call and would hand out
/// planted data.
#[derive(Clone, Debug)]
pub struct Engine {
    pub rows: BTreeMap<String, Value>,
    pub revision: u64,
    pub labels: Vec<u32>,
    pub pending: (Vec<i32>, String),
    pub previews: Vec<(Vec<i32>, String)>,
    pub views: Vec<Vec<u8>>,
    pub traps: u32,
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
            views: Vec::new(),
            traps: 0,
        }
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

/// The planted data of LX-3's probes (`planted.json`); a trap read would return it.
pub const PLANTED: [&str; 4] =
    ["LX3_CLOCK_90123", "LX3_FILE_NEVER_RETURN", "LX3_NETWORK_NEVER_RETURN", "LX3_PROCESS_NEVER_RETURN"];

pub fn planted_in(bytes: &[u8]) -> bool {
    let text = String::from_utf8_lossy(bytes);
    PLANTED.iter().any(|p| text.contains(p))
}

impl Host for Engine {
    /// A row costs its `fixture.json` cost (`score` of a word costs one plus the word's length); a request the
    /// fixture prices nowhere costs 9.
    fn tariff(&mut self, request: &Request) -> Result<RowTariff, &'static str> {
        let kernel_cost = match request {
            Request::Snapshot { query, .. } => match *query {
                "counts" | "floors" | "blocked" | "bits" => 3,
                "nu2" | "nu5" => 2,
                "snapshot" => 4,
                q if q.starts_with("score/") => 1 + (q.len() - "score/".len()) as u128,
                _ => 9,
            },
            _ => 9,
        };
        Ok(RowTariff { kernel_cost, result_bound: 4096 })
    }

    fn perform(&mut self, request: &Request) -> Result<Value, &'static str> {
        match request {
            Request::Snapshot { query, .. } => self.rows.get(*query).cloned().ok_or(NOT_GRANTED),
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
            Request::Clock => {
                self.traps += 1;
                Ok(Value::text(PLANTED[0]))
            }
            Request::FileRead { .. }
            | Request::Declassified { .. }
            | Request::SnapshotWith { .. }
            | Request::ProposeMenu { .. } => {
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

//! Shared fixtures of the record-layer conformance tests (`persist_ps1.rs`, `persist_ps2.rs`): the frozen format packet
//! read in place, JSON editing, the packet's JI23 menu word, and a facts payload written as FP.19 stores it.
//!
//! Every value here is transcribed from the packet or the chapter, or written by the test; none comes from the library
//! under test.

#![allow(dead_code)]

#[path = "../../../c600-check/tests/support/json.rs"]
pub mod json;

use std::collections::{BTreeMap, BTreeSet};

use c600_digest::sha256_hex;
use c600_evidence::model::{Check, Grade};
use c600_evidence::{u0, Edition};
use c600_session::persist::codec::{self, Json};
use c600_session::persist::word::Menu;
use c600_session::persist::Refusal;

pub const ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../");
pub const PACKET: &str = "docs/architecture/1.0/language/format-packet/";
pub const CHAPTER: &str = "docs/architecture/1.0/language/architecture/05-persistence.md";

// ------------------------------------------------------------------------------------------------- packet access

pub fn read_text(path: &str) -> String {
    let full = format!("{ROOT}{path}");
    std::fs::read_to_string(&full).unwrap_or_else(|e| panic!("cannot read {full}: {e}"))
}

pub fn packet(name: &str) -> json::Json {
    json::Json::parse(&read_text(&format!("{PACKET}{name}"))).unwrap_or_else(|e| panic!("{name} does not parse: {e}"))
}

pub fn vectors(group: &str) -> Vec<json::Json> {
    packet("vectors.json").get(group).and_then(json::Json::arr).unwrap_or_else(|| panic!("group {group}")).to_vec()
}

pub fn text<'a>(j: &'a json::Json, key: &str) -> &'a str {
    j.get(key).and_then(json::Json::str).unwrap_or_else(|| panic!("text field `{key}`"))
}

/// A packet value in the store's JSON model. The packet's JSON numbers are integer tokens.
pub fn block(j: &json::Json) -> Json {
    match j {
        json::Json::Null => Json::Null,
        json::Json::Bool(b) => Json::Bool(*b),
        json::Json::Num(n) => Json::Int(n.parse().unwrap_or_else(|e| panic!("`{n}` is no integer token: {e}"))),
        json::Json::Str(s) => Json::Text(s.clone()),
        json::Json::Arr(items) => Json::List(items.iter().map(block).collect()),
        json::Json::Obj(fields) => Json::Object(fields.iter().map(|(k, v)| (k.clone(), block(v))).collect()),
    }
}

/// `accepted`, or the refusal's code.
pub fn outcome<T>(result: Result<T, Refusal>) -> String {
    match result {
        Ok(_) => "accepted".to_string(),
        Err(refusal) => refusal.code.to_string(),
    }
}

pub fn bytes_of_hex(text: &str) -> Vec<u8> {
    (0..text.len()).step_by(2).map(|i| u8::from_str_radix(&text[i..i + 2], 16).expect("hex digit")).collect()
}

pub fn hex_of(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

// ------------------------------------------------------------------------------------------------- JSON editing

pub fn at<'a>(value: &'a mut Json, path: &[&str]) -> &'a mut Json {
    let mut v = value;
    for key in path {
        v = match v {
            Json::Object(map) => map.get_mut(*key).unwrap_or_else(|| panic!("no field `{key}`")),
            _ => panic!("not an object at `{key}`"),
        };
    }
    v
}

pub fn item(value: &mut Json, index: usize) -> &mut Json {
    match value {
        Json::List(items) => &mut items[index],
        _ => panic!("not a list"),
    }
}

pub fn set(value: &mut Json, key: &str, new: Json) {
    match value {
        Json::Object(map) => {
            map.insert(key.to_string(), new);
        }
        _ => panic!("not an object"),
    }
}

pub fn remove(value: &mut Json, key: &str) {
    match value {
        Json::Object(map) => {
            map.remove(key);
        }
        _ => panic!("not an object"),
    }
}

pub fn push(value: &mut Json, new: Json) {
    match value {
        Json::List(items) => items.push(new),
        _ => panic!("not a list"),
    }
}

/// The FP.5 text of a value without its final line feed.
pub fn compact(value: &Json) -> String {
    let mut bytes = codec::canonical(value);
    bytes.pop();
    String::from_utf8(bytes).expect("UTF-8")
}

pub fn parse(text: &str) -> Json {
    codec::parse(text.as_bytes()).unwrap_or_else(|| panic!("test JSON does not parse: {text}"))
}

/// FP.8's content digest of a record: SHA-256 of its FP.5 bytes.
pub fn content_of(record: &Json) -> Json {
    Json::object([("role", Json::text("content")), ("sha256", Json::Text(sha256_hex(&codec::canonical(record))))])
}

/// FP.11's Ref, written by the test.
pub fn ref_of(record: &Json) -> Json {
    Json::object([
        ("format", record.get("format").cloned().expect("a format")),
        ("version", record.get("version").cloned().expect("a version")),
        ("content", content_of(record)),
    ])
}

// ------------------------------------------------------------------------------------------------- JI23

/// The packet's JI23 fixture: a three-letter menu word with local annotations, an included part and the pinned menu.
pub struct Ji23 {
    pub record: Json,
    pub part_ref: Json,
    pub records: BTreeMap<String, Json>,
    pub blobs: BTreeMap<String, Vec<u8>>,
    pub inverse: BTreeMap<u16, u16>,
    pub menu: Menu,
}

pub fn ji23() -> Ji23 {
    let examples = packet("examples.json");
    let fixture = examples.get("ji23").expect("ji23");
    let json::Json::Obj(records) = fixture.get("records").expect("records") else { panic!("records") };
    let json::Json::Obj(blobs) = fixture.get("blobs").expect("blobs") else { panic!("blobs") };
    let json::Json::Obj(inverse) = fixture.get("inverse").expect("inverse") else { panic!("inverse") };
    let menu = fixture.get("menu").expect("menu");
    Ji23 {
        record: block(fixture.get("record").expect("record")),
        part_ref: block(fixture.get("part_ref").expect("part_ref")),
        records: records.iter().map(|(k, v)| (k.clone(), block(v))).collect(),
        blobs: blobs.iter().map(|(k, v)| (k.clone(), bytes_of_hex(v.str().expect("hex")))).collect(),
        inverse: inverse
            .iter()
            .map(|(k, v)| (k.parse().expect("an item"), v.num().expect("an item").parse().expect("an item")))
            .collect(),
        menu: Menu { id: text(menu, "menu_id").to_string(), order: text(menu, "menu_order").to_string() },
    }
}

/// A word's `parts` list with one included definition.
pub fn part(id: &str, reference: &Json) -> Json {
    Json::List(vec![Json::object([("id", Json::text(id)), ("word", reference.clone())])])
}

// ------------------------------------------------------------------------------------------------- facts payloads

pub const COMMIT: &str = "0123456789abcdef0123456789abcdef01234567";
pub const FULL_IDENTITY: &str = r#"{"puzzle":"full","model_id":"58c83f336e451aa64147fe1b8db2d60d7d43806aa3a35122f8403d5af51253f9","menu_id":null,"menu_order":null,"contract_rev":null}"#;
pub const JUMBLING_IDENTITY: &str = r#"{"puzzle":"jumbling","model_id":"58c83f336e451aa64147fe1b8db2d60d7d43806aa3a35122f8403d5af51253f9","menu_id":"989ce7bb3d9e703874af7b1fe11b388b7db927fbfd094a4b4596c2bba4d93979","menu_order":"e660bb57e0596f901c8b41967c038c1ab4ad41d30eccc96f60041c52950d00ae","contract_rev":"JI23-fixture-1"}"#;

pub const RECORD: &[u8] = b"fixture result record";
pub const CAPTURED: &[u8] = b"fixture captured job log";

pub fn sid_text(name: &str) -> String {
    format!(r#"{{"kind":"named","stage":"test","commit":"{COMMIT}","name":"{name}"}}"#)
}

/// A raw FP.8 BlobRef, written by the test.
pub fn raw_blob(data: &[u8]) -> String {
    let sha = sha256_hex(data);
    let n = data.len();
    format!(
        r#"{{"stored":{{"role":"bytes","sha256":"{sha}"}},"decoded":{{"role":"bytes","sha256":"{sha}"}},"stored_bytes":"{n}","decoded_bytes":"{n}","encoding":"raw"}}"#
    )
}

pub fn input_sha() -> String {
    sha256_hex(b"fixture start pose")
}

/// An evidence record around a facts payload.
pub fn evidence_record(payload: Json) -> Json {
    Json::object([
        ("format", Json::text("c600-evidence")),
        ("version", Json::Int(1)),
        ("identity", parse(FULL_IDENTITY)),
        ("payload", payload),
        ("provenance", Json::Null),
    ])
}

/// The two recorded function editions of the packet's editions vector: the display pin differs.
pub fn edition_pins() -> (Json, Json) {
    let given = vectors("editions")[0].get("given").cloned().expect("given");
    (block(given.get("old").unwrap()), block(given.get("new").unwrap()))
}

/// Edition 1 with a threshold that requires a check above producer: a second, differently pinned function edition.
pub fn stricter() -> Edition {
    let later: BTreeSet<Grade> =
        u0().into_iter().filter(|g| !matches!(g.check, Check::None | Check::Producer)).collect();
    Edition::draft_1().with_threshold(&later).expect("a threshold")
}

/// The graph shapes the facts payloads store.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shape {
    /// L under the root R.
    Leaf,
    /// R over A, an AND of L and the premise P.
    Premise,
    /// R over the leaves L and M.
    TwoLeaves,
}

/// One stored-field combination of a facts payload.
#[derive(Clone, Copy, Debug)]
pub struct Case {
    pub basis: &'static str,
    pub check: &'static str,
    pub job: Option<&'static str>,
    pub search: Option<bool>,
    pub covered: bool,
    pub same_identity: bool,
    pub complete: bool,
    pub shape: Shape,
}

impl Case {
    /// A complete single leaf: an exact computation an independent checker confirmed, covering its scope's input.
    pub fn plain() -> Case {
        Case {
            basis: "exact_computation",
            check: "independent_checker",
            job: Some("completed"),
            search: None,
            covered: true,
            same_identity: true,
            complete: true,
            shape: Shape::Leaf,
        }
    }
}

pub fn search_text(complete: bool) -> String {
    let (complete, stop, job) =
        if complete { ("yes", "closed", "completed") } else { ("no", "deterministic_budget", "cut_off") };
    format!(
        r#"{{"target":{},"declared":{{"family":"macros","alphabet":["a","b"],"bounds":{{"length":"3"}}}},"complete":"{complete}","stop":{{"kind":"{stop}"}},"job":"{job}"}}"#,
        sid_text("S")
    )
}

pub fn leaf_text(id: &str, c: &Case, basis: &str, check: &str) -> String {
    let covered = if c.covered {
        format!(r#"{{"covers":{{"kind":"one_instance"}},"input":{{"role":"poses","sha256":"{}"}}}}"#, input_sha())
    } else {
        "null".to_string()
    };
    let search = c.search.map_or("null".to_string(), search_text);
    let job = c.job.map_or("null".to_string(), |o| {
        format!(
            r#"{{"source":{{"role":"bytes","sha256":"{}"}},"pointer":"/jobs/0","id":"job-1","outcome":"{o}"}}"#,
            sha256_hex(CAPTURED)
        )
    });
    let identity = if c.same_identity { FULL_IDENTITY } else { JUMBLING_IDENTITY };
    format!(
        r#"{{"id":"{id}","kind":"leaf","key":{{"statement":{sid},"scope":"s"}},"grade":{{"basis":"{basis}","check":"{check}"}},"identity":{identity},"covered":{covered},"record":{record},"checks":[],"membership":[],"search":{search},"producing_job":{job},"reachability":null}}"#,
        sid = sid_text("S"),
        record = raw_blob(RECORD),
    )
}

/// The facts payload of a case, as FP.19 and FP.20 store it.
pub fn payload_text(c: &Case, editions: &Json) -> String {
    let mut statements = vec![format!(
        r#"{{"id":{},"form":{{"kind":"finite"}},"negation":null,"subject":"pose","premises":[],"guards":[]}}"#,
        sid_text("S")
    )];
    let leaf = leaf_text("L", c, c.basis, c.check);
    let key = format!(r#"{{"statement":{},"scope":"s"}}"#, sid_text("S"));
    let nodes = match c.shape {
        Shape::Leaf => vec![leaf, format!(r#"{{"id":"R","kind":"or","key":{key},"children":["L"]}}"#)],
        Shape::Premise => {
            statements.push(format!(
                r#"{{"id":{},"form":{{"kind":"finite"}},"negation":null,"subject":"pose","premises":[],"guards":[]}}"#,
                sid_text("T")
            ));
            vec![
                r#"{"id":"A","kind":"and","children":["L","P"]}"#.to_string(),
                leaf,
                format!(
                    r#"{{"id":"P","kind":"premise","key":{{"statement":{},"scope":"s"}},"class":"open_hypothesis"}}"#,
                    sid_text("T")
                ),
                format!(r#"{{"id":"R","kind":"or","key":{key},"children":["A"]}}"#),
            ]
        }
        Shape::TwoLeaves => vec![
            leaf,
            leaf_text("M", c, "theorem", "model_review"),
            format!(r#"{{"id":"R","kind":"or","key":{key},"children":["L","M"]}}"#),
        ],
    };
    let source = format!(
        r#"{{"bytes":{},"format":"text","pointer":"","labels":[{{"pointer":"/jobs/0/grade","value":"(exhaustive computation, producer)"}}],"stage":"stage 15","source_version":"1","attribution":null,"engineering":"source"}}"#,
        raw_blob(CAPTURED)
    );
    format!(
        r#"{{"form":"facts","status":"{status}","statements":[{statements}],"scopes":[{{"id":"s","covers":{{"kind":"one_instance"}},"identity":{FULL_IDENTITY},"contract":"contract:test-r1","input":{{"role":"poses","sha256":"{input}"}},"sources":[]}}],"nodes":[{nodes}],"roots":["R"],"sources":[{source}],"editions":{editions},"caches":[],"missing":[]}}"#,
        status = if c.complete { "complete" } else { "recorded" },
        statements = statements.join(","),
        input = input_sha(),
        nodes = nodes.join(","),
        editions = compact(editions),
    )
}

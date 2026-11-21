//! LP2 conformance tests for chapter 05's record layer (`c600_session::persist`, M6's remaining rows): CT-PS1-08,
//! CT-PS1-09, CT-PS1-10, CT-PS1-19, CT-PS1-21 and CT-PS1-23.
//!
//! Vectors and fixtures are read in place from the frozen format packet; catalogue rows come from FP.4's and PS1.12's
//! tables. Expected values come from the packet, the chapter, the test's own transcription or an independent
//! computation with 0.4's canonical form (pinned below), never from the library under test.
//!
//! Evidence kind: source and fixture. One run covers one platform; nothing here is Windows/DirectX evidence.

mod persist_support;

use std::collections::{BTreeMap, BTreeSet};

use c600_digest::sha256_hex;
use c600_evidence::model::{
    Basis, Check, CheckMode, Complete, Covered, Covers, Declared, DigestRef, DigestRole, Form, Grade, JobOutcome, Key,
    Leaf, Level, PremiseClass, ProducingJob, Scope, SearchRecord, Statement, StatementId, Stop,
};
use c600_evidence::{word as display_word, Edition, Graph};
use c600_kernel::json::{self as legacy_json, Value};
use c600_session::persist::codec::{self, Json};
use c600_session::persist::codes::{CANON, DIGEST, EXPANSION, FORMAT, IDENTITY, SCOPE, SIZE, VERSION};
use c600_session::persist::facts::{self, Derived, Functions};
use c600_session::persist::legacy::{self, State};
use c600_session::persist::record::{self, Entry, Read};
use c600_session::persist::word::{self, Letter, Store};
use persist_support::*;

/// The backticked tokens of a table cell.
fn ticked(cell: &str) -> Vec<String> {
    cell.split('`').skip(1).step_by(2).map(str::to_string).collect()
}

/// The cells of each body row of the Markdown table whose header line starts with `header`.
fn table(source: &str, header: &str) -> Vec<Vec<String>> {
    let mut lines = source.lines().skip_while(|l| !l.starts_with(header));
    assert!(lines.next().is_some(), "table `{header}`");
    lines
        .skip(1)
        .take_while(|l| l.starts_with('|'))
        .map(|l| l.split('|').map(|c| c.trim().to_string()).collect())
        .collect()
}

// ------------------------------------------------------------------------------------------------- CT-PS1-08

/// CT-PS1-08 (source, whole: FP.4's and PS1.12's catalogue tables against the reader's dispatch, FP.10's nine
/// unknown-field vectors through the header check and `read_record`, a nested provenance version and a Ref of a newer
/// version): the catalogue keeps every existing identifier, a known optional field survives read and write byte for
/// byte, and unknown or newer data stays recorded with its original bytes and never activates.
#[test]
fn ct_ps1_08_catalogue_keeps_identifiers_and_never_activates_unknown_data() {
    // FP.4: frozen rows have no -draft suffix and their former draft identifier is unknown; rows marked No keep it.
    let readme = read_text(&format!("{PACKET}README.md"));
    let mut frozen = BTreeSet::new();
    for row in table(&readme, "| Record | Exact `format`") {
        let in_freeze = row[4].as_str();
        for token in ticked(&row[2]) {
            match record::entry(&token) {
                Entry::Frozen(kind) => {
                    assert_ne!(in_freeze, "No", "{token} is outside the freeze");
                    assert!(!token.ends_with("-draft"), "{token}");
                    assert_eq!(record::ident(kind), token);
                    let former = format!("{token}-draft");
                    assert_eq!(record::entry(&former), Entry::Unknown, "{former} is no alias (FP.4)");
                    assert!(!record::known_format(&former));
                    frozen.insert(kind.to_string());
                }
                Entry::Reserved | Entry::Draft(_) => {
                    assert_eq!(in_freeze, "No", "{token}");
                    assert!(token.ends_with("-draft"), "{token} keeps its suffix");
                }
                Entry::Retained => assert!(!token.starts_with("c600-"), "{token} is an existing identifier"),
                Entry::Unknown => panic!("FP.4 row {token} is not in the catalogue"),
            }
        }
    }
    let catalogue: BTreeSet<String> = record::FROZEN_KINDS.iter().map(|k| k.to_string()).collect();
    assert_eq!(frozen, catalogue, "the frozen kinds are exactly FP.4's");

    // PS1.12: existing identifiers are reused without renaming; the c600 reader keeps their bytes recorded.
    let chapter = read_text(CHAPTER);
    let not_identifiers =
        ["word", "star", ".6cs", "session/checkpoint-paths.json", ".c600migrate", "jams.json", "steps.json"];
    let mut retained = 0;
    for row in table(&chapter, "| Format | Identifier | Version |") {
        for token in ticked(&row[2]) {
            if token.contains('*') || not_identifiers.contains(&token.as_str()) {
                continue;
            }
            if token.starts_with("c600-") {
                assert_ne!(record::entry(&token), Entry::Unknown, "{token} is catalogued");
                continue;
            }
            assert_eq!(record::entry(&token), Entry::Retained, "{token} is an existing identifier");
            let header = Json::object([
                ("format", Json::text(&token)),
                ("version", Json::Int(1)),
                ("identity", parse(FULL_IDENTITY)),
                ("payload", Json::object([])),
                ("provenance", Json::Null),
            ]);
            let bytes = codec::canonical(&header);
            assert_eq!(
                record::read_record(&bytes, &token),
                Ok(Read::Recorded { bytes: bytes.clone(), code: VERSION }),
                "{token} stays recorded"
            );
            assert_eq!(outcome(record::write_record(&header)), VERSION, "{token} is not written as a c600 record");
            retained += 1;
        }
    }
    assert!(retained >= 30, "PS1.12's retained identifiers were read ({retained})");

    // FP.10's vectors on the packet's specimen record.
    let examples = packet("examples.json");
    let specimen = block(examples.get("records").and_then(|r| r.get("manual_flow")).expect("manual_flow"));
    let unknown = vectors("unknown_fields");
    assert_eq!(unknown.len(), 9);
    for vector in &unknown {
        let id = text(vector, "id");
        let expected = text(vector, "expected");
        let mut record = specimen.clone();
        if let Json::Object(given) = block(vector.get("given").expect("given")) {
            for (key, value) in given {
                set(&mut record, &key, value);
            }
        }
        for field in vector.get("remove").and_then(json::Json::arr).unwrap_or(&[]) {
            remove(&mut record, field.str().expect("a field name"));
        }
        assert_eq!(outcome(record::record_header(&record)), expected, "{id}");
        let bytes = codec::canonical(&record);
        let read = record::read_record(&bytes, "c600-word");
        match expected {
            "accepted" => {
                assert_eq!(codec::read_canonical(&bytes), Ok(record.clone()), "{id}: unknown optional retained");
                assert_eq!(read, Ok(Read::Active(record.clone())), "{id}");
                assert_eq!(record::write_record(&record), Ok(bytes.clone()), "{id}: written back byte for byte");
            }
            VERSION => assert_eq!(read, Ok(Read::Recorded { bytes: bytes.clone(), code: VERSION }), "{id}"),
            _ => assert_eq!(read.map_err(|r| r.code), Err(expected), "{id}"),
        }
    }

    // A newer nested provenance version cannot hide inside a known record (FP.10).
    let mut nested = specimen.clone();
    set(at(&mut nested, &["provenance", "block"]), "version", Json::Int(2));
    let bytes = codec::canonical(&nested);
    assert_eq!(record::read_record(&bytes, "c600-word"), Ok(Read::Recorded { bytes: bytes.clone(), code: VERSION }));

    // A Ref is dispatched before it is followed: a newer version is refused before its digest is looked up.
    let mut newer = record::reference(&specimen);
    set(&mut newer, "version", Json::Int(2));
    assert_eq!(outcome(record::follow(&newer, &BTreeMap::new(), FORMAT)), VERSION);
    let known = record::reference(&specimen);
    let store = BTreeMap::from([(codec::sha_of(known.get("content").unwrap()).unwrap().to_string(), specimen.clone())]);
    assert_eq!(record::follow(&known, &store, FORMAT), Ok(&specimen));
}

// ------------------------------------------------------------------------------------------------- CT-PS1-09

/// The packet's `relation_result`, transcribed: one mutation of the fixture, then admission.
fn relation(case: &str, f: &Ji23) -> String {
    let mut record = f.record.clone();
    let mut blobs = f.blobs.clone();
    let mut records = f.records.clone();
    match case {
        "local" => {}
        "part" | "ref" => {
            if case == "part" {
                set(at(&mut record, &["payload"]), "parts", part("A-part", &f.part_ref));
            }
            let Json::List(annotations) = at(&mut record, &["payload", "annotations"]) else { panic!("annotations") };
            for annotation in annotations.iter_mut().skip(1) {
                let target = if case == "part" {
                    Json::object([("part", Json::text("A-part"))])
                } else {
                    Json::object([("ref", f.part_ref.clone())])
                };
                set(at(annotation, &["relation"]), "word", target);
            }
        }
        "missing" => {
            let annotation = item(at(&mut record, &["payload", "annotations"]), 1);
            set(at(annotation, &["relation"]), "word", Json::object([("local", Json::text("missing"))]));
        }
        "cycle" => {
            let annotation = item(at(&mut record, &["payload", "annotations"]), 0);
            let relation =
                Json::object([("kind", Json::text("same_as")), ("word", Json::object([("local", Json::text("B"))]))]);
            set(annotation, "relation", relation);
        }
        "changed_inverse" => {
            let raw = bytes_of_hex("0c000c000c00");
            set(at(&mut record, &["payload"]), "chunks", Json::List(vec![codec::blob(&raw)]));
            set(at(&mut record, &["payload"]), "letters", codec::digest_ref("bytes", &raw));
            blobs.insert(sha256_hex(&raw), raw);
        }
        "missing_chunks" => remove(at(&mut record, &["payload"]), "chunks"),
        "missing_part" => {
            set(at(&mut record, &["payload"]), "parts", part("A-part", &f.part_ref));
            records.clear();
        }
        "part_future_version" => {
            let sha = codec::sha_of(f.part_ref.get("content").unwrap()).unwrap().to_string();
            let mut target = records[&sha].clone();
            set(&mut target, "version", Json::Int(2));
            let content = codec::content_ref(&target);
            let mut reference = f.part_ref.clone();
            set(&mut reference, "version", Json::Int(2));
            set(&mut reference, "content", content.clone());
            records = BTreeMap::from([(codec::sha_of(&content).unwrap().to_string(), target)]);
            set(at(&mut record, &["payload"]), "parts", part("A-part", &reference));
        }
        "menu_id" | "menu_order" => set(at(&mut record, &["identity"]), case, Json::Text("0".repeat(64))),
        other => panic!("unknown relation case {other}"),
    }
    let store = Store { blobs: &blobs, records: &records, inverse: &f.inverse, menu: &f.menu };
    outcome(word::admit(&record, &store))
}

/// The letters of `ia16` bytes, transcribed from FP.13: little-endian `60 * grip + item`.
fn letters_of(raw: &[u8]) -> Vec<Letter> {
    raw.chunks(2).map(|p| u16::from_le_bytes([p[0], p[1]])).map(|code| (code / 60, code % 60)).collect()
}

/// CT-PS1-09 (source, whole: FP.13's eleven relation vectors on the JI23 fixture, the `ia16` vector and its real chunk
/// boundary, a written two-chunk word read back, and annotations removed or overrun): a flat menu word keeps its
/// literal letters in mandatory chunks; `same_as` and `inverse_of` must match the pinned span they name and generate
/// no letter; a dangling or cyclic target is refused.
#[test]
fn ct_ps1_09_flat_words_keep_literal_chunks_and_relations_generate_nothing() {
    let f = ji23();
    let relations = vectors("relations");
    assert_eq!(relations.len(), 11);
    for vector in &relations {
        let case = text(vector.get("given").expect("given"), "case");
        assert_eq!(relation(case, &f), text(vector, "expected"), "{}", text(vector, "id"));
    }

    // The admitted word is the literal chunk, and its annotations add nothing.
    let store = Store { blobs: &f.blobs, records: &f.records, inverse: &f.inverse, menu: &f.menu };
    let literal = letters_of(&bytes_of_hex("0c000c002d00"));
    assert_eq!(literal, vec![(0, 12), (0, 12), (0, 45)]);
    assert_eq!(word::admit(&f.record, &store), Ok(literal.clone()));
    let mut bare = f.record.clone();
    set(at(&mut bare, &["payload"]), "annotations", Json::List(Vec::new()));
    assert_eq!(word::admit(&bare, &store), Ok(literal.clone()), "annotations generate no letter");
    let mut overrun = f.record.clone();
    let annotation = item(at(&mut overrun, &["payload", "annotations"]), 0);
    set(annotation, "end", Json::text("4"));
    assert_eq!(outcome(word::admit(&overrun, &store)), FORMAT, "a span past the literal word");

    // The ia16 vector, and a real chunk boundary.
    let vector = packet("vectors.json").get("word").cloned().expect("word vector");
    let given: Vec<Letter> = vector
        .get("given")
        .and_then(json::Json::arr)
        .expect("letters")
        .iter()
        .map(|pair| {
            let pair = pair.arr().expect("a pair");
            (pair[0].num().unwrap().parse().unwrap(), pair[1].num().unwrap().parse().unwrap())
        })
        .collect();
    let raw = word::encode_ia16(&given).expect("in range");
    assert_eq!(hex_of(&raw), text(&vector, "expected_hex"));
    assert_eq!(sha256_hex(&raw), text(&vector, "expected_sha256"));
    assert_eq!(word::decode_ia16(&raw), Ok(given));
    let mut letters = vec![(0u16, 0u16); 524_288 + 1];
    letters.push((599, 59));
    let raw = word::encode_ia16(&letters).expect("in range");
    let chunks = word::word_chunks(&raw).expect("even");
    let sizes: Vec<String> = chunks.iter().map(|c| c.len().to_string()).collect();
    let expected: Vec<String> = vector
        .get("chunk_expected_bytes")
        .and_then(json::Json::arr)
        .expect("chunk sizes")
        .iter()
        .map(|n| n.num().expect("a size").to_string())
        .collect();
    assert_eq!(sizes, expected);
    let n = letters.len() as i128;
    let digest = sha256_hex(&raw);
    assert_eq!(word::read_chunks(&chunks, n, &digest, "order-a", "order-a"), Ok(letters.clone()));
    assert_eq!(outcome(word::read_chunks(&chunks, n, &digest, "order-a", "order-b")), IDENTITY);
    assert_eq!(outcome(word::read_chunks(&chunks, n - 1, &digest, "a", "a")), CANON);
    assert_eq!(outcome(word::read_chunks(&chunks, i128::MAX, &digest, "a", "a")), CANON, "a length past i128::MAX / 2");
    assert_eq!(outcome(word::decode_ia16(&36_000u16.to_le_bytes())), CANON);
    assert_eq!(outcome(word::decode_ia16(&[0])), CANON);

    // A written word keeps the literal letters across its mandatory chunks; a dropped chunk is refused.
    let identity = f.record.get("identity").cloned().unwrap();
    let start = f.record.get("payload").and_then(|p| p.get("start")).cloned().unwrap();
    let provenance = f.record.get("provenance").cloned().unwrap();
    let (written, blobs) = word::write(&identity, &start, &letters, &[], &[], &provenance).expect("a word");
    let Some(Json::List(stored)) = written.get("payload").and_then(|p| p.get("chunks")) else { panic!("chunks") };
    assert_eq!(stored.len(), 2, "two mandatory chunks");
    let none = BTreeMap::new();
    let store = Store { blobs: &blobs, records: &none, inverse: &f.inverse, menu: &f.menu };
    assert_eq!(word::admit(&written, &store), Ok(letters));
    let mut short = written.clone();
    let Json::List(chunks) = at(&mut short, &["payload", "chunks"]) else { panic!("chunks") };
    chunks.pop();
    assert_eq!(outcome(word::admit(&short, &store)), CANON, "a dropped chunk shortens the literal word");
    let mut huge = written.clone();
    set(at(&mut huge, &["payload"]), "length", Json::text(&"9".repeat(50)));
    assert_eq!(outcome(word::admit(&huge, &store)), CANON, "a length no literal word has, refused without overflow");
}

// ------------------------------------------------------------------------------------------------- CT-PS1-10

/// CT-PS1-10 (source, whole: the packet's storage vector, the expanded word's charge through `stored_bytes`, and
/// replay admission through `expand` below, at and above the X3 bound): a small source graph is charged against its
/// own record budget, the stored expanded word against `bundle_decoded` for every literal byte, and a word above X3 is
/// refused with its exact length before any chunk is read, never truncated.
#[test]
fn ct_ps1_10_storage_charges_literal_bytes_and_x3_refuses_before_replay() {
    for vector in vectors("storage") {
        let given = vector.get("given").expect("given");
        let graph: u128 = given.get("graph_bytes").and_then(json::Json::num).unwrap().parse().unwrap();
        let letters = codec::int_value(&Json::text(text(given, "letters"))).expect("a Nat") as u128;
        let result = BTreeMap::from([
            ("source_graph", outcome(codec::charge("record_decoded", graph))),
            ("expanded_word", outcome(codec::charge("bundle_decoded", 2 * letters))),
        ]);
        let expected = vector.get("expected").expect("expected");
        for (name, got) in result {
            assert_eq!(got, text(expected, name), "{} {name}", text(&vector, "id"));
        }
    }

    let f = ji23();
    let store = Store { blobs: &f.blobs, records: &f.records, inverse: &f.inverse, menu: &f.menu };
    let small = codec::canonical(&f.record);
    assert_eq!(outcome(codec::charge("record_decoded", small.len() as u128)), "accepted", "the graph's own budget");

    // The stored expansion charges every literal byte: 2,147,483,649 letters are 4,294,967,298 bytes.
    let mut long = f.record.clone();
    set(at(&mut long, &["payload"]), "length", Json::text("2147483649"));
    let refusal = word::stored_bytes(&long).unwrap_err();
    assert_eq!(refusal.code, SIZE);
    let limit = refusal.limit.expect("PS1.11's row");
    assert_eq!((limit.row, limit.max, limit.attempted.as_str()), ("bundle_decoded", 4_294_967_296, "4294967298"));

    // X3 comes first: no chunk is read (the store below has none), and the refusal carries the exact length.
    let empty = BTreeMap::new();
    let no_records = BTreeMap::new();
    let bare = Store { blobs: &empty, records: &no_records, inverse: &f.inverse, menu: &f.menu };
    let refusal = word::expand(&long, &bare, 2_147_483_648).unwrap_err();
    assert_eq!((refusal.code, refusal.length.as_deref()), (EXPANSION, Some("2147483649")));
    let refusal = word::expand(&long, &bare, 2_147_483_649).unwrap_err();
    assert_eq!(refusal.code, SIZE, "within X3, the storage charge refuses next");
    assert_eq!(word::expand(&f.record, &store, 3).map(|w| w.len()), Ok(3));
    let refusal = word::expand(&f.record, &store, 2).unwrap_err();
    assert_eq!((refusal.code, refusal.length.as_deref()), (EXPANSION, Some("3")), "refused whole, not truncated");
}

// ------------------------------------------------------------------------------------------------- evidence model

fn sid(name: &str) -> StatementId {
    StatementId::named("test", COMMIT, name)
}

const BASES: [&str; 6] = ["theorem", "certificate", "exhaustive_computation", "exact_computation", "sample", "oracle"];
const CHECKS: [&str; 7] =
    ["none", "producer", "independent_checker", "model_review", "human_review", "formal_proof", "peer"];
const JOBS: [Option<&str>; 6] =
    [None, Some("completed"), Some("cut_off"), Some("cancelled"), Some("error"), Some("paused")];

fn job_outcome(name: &str) -> JobOutcome {
    match name {
        "completed" => JobOutcome::Completed,
        "cut_off" => JobOutcome::CutOff,
        "cancelled" => JobOutcome::Cancelled,
        "error" => JobOutcome::Error,
        other => JobOutcome::Other(other.to_string()),
    }
}

fn identity_text(identity: &str) -> String {
    compact(&parse(identity))
}

fn direct_leaf(key: &Key, c: &Case, basis: &str, check: &str) -> Leaf {
    let identity = identity_text(if c.same_identity { FULL_IDENTITY } else { JUMBLING_IDENTITY });
    let mut leaf = Leaf::new(key, Grade::new(Basis::from_wire(basis), Check::from_wire(check)))
        .identity(&identity)
        .support(&sha256_hex(RECORD));
    if c.covered {
        leaf = leaf.covered(Covered::new(Covers::OneInstance, Some(DigestRef::poses(&input_sha()))));
    }
    if let Some(complete) = c.search {
        leaf = leaf.search(SearchRecord {
            target: sid("S"),
            declared: Declared {
                family: "macros".to_string(),
                alphabet: r#"["a","b"]"#.to_string(),
                bounds: r#"{"length":"3"}"#.to_string(),
            },
            complete: if complete { Complete::Yes } else { Complete::No },
            stop: if complete { Stop::Closed } else { Stop::DeterministicBudget },
            job: if complete { JobOutcome::Completed } else { JobOutcome::CutOff },
        });
    }
    if let Some(name) = c.job {
        leaf = leaf.job(ProducingJob {
            source: sha256_hex(CAPTURED),
            pointer: "/jobs/0".to_string(),
            id: "job-1".to_string(),
            outcome: job_outcome(name),
        });
    }
    leaf
}

/// The same facts built directly in the evidence model, in the order admission builds them (node IDs sorted).
fn direct(c: &Case) -> (Graph, Key) {
    let scope = Scope::new(
        Covers::OneInstance,
        &identity_text(FULL_IDENTITY),
        "contract:test-r1",
        Some(DigestRef::poses(&input_sha())),
    );
    let key = Key::new(sid("S"), scope.clone());
    let mut g = Graph::new();
    g.add_statement(Statement::new(sid("S")).form(Form::Finite).subject(Level::Pose)).unwrap();
    match c.shape {
        Shape::Leaf => {
            let l = c.complete.then(|| g.add_leaf(direct_leaf(&key, c, c.basis, c.check)));
            g.root(&key);
            if let Some(l) = l {
                g.add_or_child(&key, l).unwrap();
            }
        }
        Shape::Premise => {
            g.add_statement(Statement::new(sid("T")).form(Form::Finite).subject(Level::Pose)).unwrap();
            let other = Key::new(sid("T"), scope);
            let l = c.complete.then(|| g.add_leaf(direct_leaf(&key, c, c.basis, c.check)));
            let p = g.add_premise(&other, PremiseClass::OpenHypothesis);
            g.root(&key);
            if let Some(l) = l {
                let a = g.add_and(&[l, p]).unwrap();
                g.add_or_child(&key, a).unwrap();
            }
        }
        Shape::TwoLeaves => {
            let leaves = c.complete.then(|| {
                let l = g.add_leaf(direct_leaf(&key, c, c.basis, c.check));
                let m = g.add_leaf(direct_leaf(&key, c, "theorem", "model_review"));
                (l, m)
            });
            g.root(&key);
            if let Some((l, m)) = leaves {
                g.add_or_child(&key, l).unwrap();
                g.add_or_child(&key, m).unwrap();
            }
        }
    }
    (g, key)
}

fn direct_derived(g: &Graph, key: &Key, edition: &Edition) -> Derived {
    match display_word(g, key, edition) {
        Ok(w) => Derived::Word(w),
        Err(e) => Derived::Inconsistent(e),
    }
}

// ------------------------------------------------------------------------------------------------- CT-PS1-19

/// CT-PS1-19 (source, whole: an envelope with labelled, sticker-class, source and unnamed digest roles, two check facts
/// and an original source label set, written, read and admitted; each revision-bound scope field and each mutable
/// input refused at write and at admission): the facts envelope rejects a scope that holds transient context and keeps
/// every digest's level, every original source label and every check mode.
#[test]
fn ct_ps1_19_facts_envelope_keeps_levels_labels_and_modes() {
    let (pins, _) = edition_pins();
    let labels_sha = sha256_hex(b"fixture labels");
    let report = raw_blob(b"fixture check report");
    let tool = sha256_hex(b"fixture checker source");
    let base = sha256_hex(b"fixture common base");
    let class_sha = sha256_hex(b"fixture sticker classes");
    let region_sha = sha256_hex(b"fixture region classes");
    let note = r#"{"checked":"2026-09-01","original":"Grade B+"}"#;
    let check = |mode: &str| {
        format!(
            r#"{{"tool":{{"role":"source","sha256":"{tool}"}},"common_base":[{{"role":"source","sha256":"{base}"}}],"report":{report},"kind":"replay","result":"pass","mode":"{mode}","transport":null}}"#
        )
    };
    let text = format!(
        r#"{{"form":"facts","status":"complete",
        "statements":[{{"id":{sid},"form":{{"kind":"finite"}},"negation":null,"subject":"labels","premises":[],"guards":[]}}],
        "scopes":[{{"id":"s","covers":{{"kind":"one_instance"}},"identity":{FULL_IDENTITY},"contract":"contract:test-r1",
            "input":{{"role":"labels","sha256":"{labels_sha}"}},
            "sources":[{{"role":"sticker_class","sha256":"{class_sha}"}},{{"role":"region_class","sha256":"{region_sha}"}}]}}],
        "nodes":[{{"id":"L","kind":"leaf","key":{{"statement":{sid},"scope":"s"}},"grade":{{"basis":"exact_computation","check":"independent_checker"}},
            "identity":{FULL_IDENTITY},"covered":{{"covers":{{"kind":"one_instance"}},"input":{{"role":"labels","sha256":"{labels_sha}"}}}},
            "record":{record},"checks":[{fresh},{unknown}],"membership":[],"search":null,
            "producing_job":{{"source":{{"role":"bytes","sha256":"{captured}"}},"pointer":"/rows/3","id":"t3p-check-7","outcome":"completed"}},"reachability":null}},
            {{"id":"R","kind":"or","key":{{"statement":{sid},"scope":"s"}},"children":["L"]}}],
        "roots":["R"],
        "sources":[{{"bytes":{captured_blob},"format":"t3p-check/1","pointer":"/rows/3","labels":[
            {{"pointer":"/rows/3/grade","value":"(exact computation, independent checker)"}},{{"pointer":"/rows/3/note","value":{note}}}],
            "stage":"stage 14","source_version":"t3p-check/1","attribution":"archived checker","engineering":"source"}}],
        "editions":{editions},"caches":[],"missing":[]}}"#,
        sid = sid_text("S"),
        record = raw_blob(RECORD),
        fresh = check("fresh_exact_audit"),
        unknown = check("replayed_twice"),
        captured = sha256_hex(CAPTURED),
        captured_blob = raw_blob(CAPTURED),
        editions = compact(&pins),
    );
    let payload = parse(&text);
    let record = evidence_record(payload.clone());
    let bytes = record::write_record(&record).expect("an immutable envelope is written");
    assert_eq!(record::read_record(&bytes, "c600-evidence"), Ok(Read::Active(record.clone())));
    let facts = facts::admit(&payload).expect("admitted");

    // Digest levels: the scope's input and coverage stay at the labels level; the scope's sources keep their roles,
    // an unnamed role by its stored name.
    let (_, key) = &facts.roots[0];
    let labels = DigestRef::new(DigestRole::Labels, &labels_sha);
    assert_eq!(key.scope.input.as_ref(), Some(&labels));
    assert_eq!(labels.level(), Some(Level::Labels));
    assert_eq!(
        key.scope.sources,
        vec![
            DigestRef::new(DigestRole::StickerClass, &class_sha),
            DigestRef::new(DigestRole::Other("region_class".to_string()), &region_sha)
        ]
    );
    let leaves: Vec<&Leaf> = (0..facts.graph.len()).filter_map(|n| facts.graph.leaf(n)).collect();
    assert_eq!(leaves.len(), 1);
    let leaf = leaves[0];
    assert_eq!(leaf.covered.as_ref().and_then(|c| c.input.as_ref()), Some(&labels));
    // Check modes: a known mode by name, an unknown one kept as read.
    let modes: Vec<&CheckMode> = leaf.checks.iter().map(|c| &c.mode).collect();
    assert_eq!(modes, [&CheckMode::FreshExactAudit, &CheckMode::Other("replayed_twice".to_string())]);
    assert_eq!(leaf.checks[0].tool, tool);
    assert_eq!(leaf.checks[0].common_base, vec![base.clone()]);
    // The original source labels, word for word, with the source's version and engineering kind.
    let source = &facts.sources[0];
    assert_eq!(
        source.labels,
        vec![
            ("/rows/3/grade".to_string(), Json::text("(exact computation, independent checker)")),
            ("/rows/3/note".to_string(), parse(note)),
        ]
    );
    assert_eq!(
        (
            source.format.as_deref(),
            source.stage.as_str(),
            source.source_version.as_deref(),
            source.engineering.as_deref()
        ),
        (Some("t3p-check/1"), "stage 14", Some("t3p-check/1"), Some("source"))
    );
    assert_eq!(source.bytes, sha256_hex(CAPTURED));
    assert_eq!(
        leaf.producing_job.as_ref().map(|j| (j.pointer.as_str(), j.id.as_str())),
        Some(("/rows/3", "t3p-check-7"))
    );

    // Revision-bound context never enters a stored scope: each such field, and an input that is not a digest.
    for field in ["revision", "epoch", "role", "goal", "protection", "orbit"] {
        let mut transient = payload.clone();
        set(item(at(&mut transient, &["scopes"]), 0), field, Json::text("r12"));
        assert_eq!(outcome(record::write_record(&evidence_record(transient.clone()))), SCOPE, "{field} at write");
        assert_eq!(outcome(facts::admit(&transient)), SCOPE, "{field} at read");
    }
    let revision_input = Json::text("revision 12");
    let mut stamped = parse(&format!(r#"{{"role":"labels","sha256":"{labels_sha}"}}"#));
    set(&mut stamped, "revision", Json::text("12"));
    for input in [revision_input, stamped] {
        let mut transient = payload.clone();
        set(item(at(&mut transient, &["scopes"]), 0), "input", input);
        assert_eq!(outcome(record::write_record(&evidence_record(transient.clone()))), SCOPE);
        assert_eq!(outcome(facts::admit(&transient)), SCOPE);
    }
}

// ------------------------------------------------------------------------------------------------- CT-PS1-21

fn check_one(c: &Case, pins: &[(Json, Edition)], functions: &Functions, unknown: &Json) {
    let payload = parse(&payload_text(c, &pins[0].0));
    let stored = facts::admit(&payload).unwrap_or_else(|r| panic!("{c:?} is admitted: {r}"));
    let record = evidence_record(payload);
    let bytes = record::write_record(&record).unwrap_or_else(|r| panic!("{c:?} is written: {r}"));
    let Ok(Read::Active(read)) = record::read_record(&bytes, "c600-evidence") else { panic!("{c:?} is read") };
    let reread = facts::admit(read.get("payload").unwrap()).expect("admitted after the round trip");
    let (g, key) = direct(c);
    for (editions, edition) in pins {
        let derived = facts::derive(&stored, editions, functions);
        assert_eq!(derived.len(), 1, "{c:?}: one root, one judgement");
        let (root, judgement) = &derived[0];
        assert_eq!(root, "R");
        match judgement {
            Derived::Word(w) => assert!((1..=7).contains(&w.row), "{c:?}: row {}", w.row),
            Derived::Inconsistent(e) => assert!(e.code.starts_with("EVD:"), "{c:?}: {}", e.code),
            Derived::NoVerdict => panic!("{c:?}: a recorded edition gives a verdict"),
        }
        assert_eq!(judgement, &direct_derived(&g, &key, edition), "{c:?}: no second evidence order");
        assert_eq!(facts::derive(&reread, editions, functions), derived, "{c:?}: the round trip keeps the verdict");
    }
    assert_eq!(facts::derive(&stored, unknown, functions), vec![("R".to_string(), Derived::NoVerdict)], "{c:?}");
}

/// CT-PS1-27 (part: permitted facts with their source and check dependencies round-trip through the FP.19 and FP.20
/// envelope and derive the same word before and after; not run: a forbidden basis, check or formal proof written
/// through storage, which `admit` does not check, and the envelope writer's call of the `record` sink).
/// CT-PS1-21 (source, whole: the leaf facts crossed exhaustively, six bases, seven checks, six job outcomes, three
/// search records and coverage, and four graph shapes with both identities and both statuses, each stored, written,
/// read and admitted, then judged under two recorded function editions and one unavailable edition, and compared
/// with the word of the same facts built directly in the evidence model): every admitted stored-field combination
/// gives exactly one derived display word or one typed inconsistency under each recorded function edition, the same
/// before and after the round trip, and no verdict under an edition the build cannot run.
#[test]
fn ct_ps1_21_each_stored_combination_derives_one_word_or_inconsistency() {
    let (old, new) = edition_pins();
    let pins = [(old.clone(), Edition::draft_1()), (new.clone(), stricter())];
    let functions = Functions::new().with(old.clone(), Edition::draft_1()).with(new, stricter());
    let mut unknown = old;
    set(at(&mut unknown, &["summary"]), "version", Json::text("fixture-2"));

    let mut cases = 0;
    for basis in BASES {
        for check in CHECKS {
            for job in JOBS {
                for search in [None, Some(true), Some(false)] {
                    for covered in [false, true] {
                        let c = Case {
                            basis,
                            check,
                            job,
                            search,
                            covered,
                            same_identity: true,
                            complete: true,
                            shape: Shape::Leaf,
                        };
                        check_one(&c, &pins, &functions, &unknown);
                        cases += 1;
                    }
                }
            }
        }
    }
    for shape in [Shape::Leaf, Shape::Premise, Shape::TwoLeaves] {
        for complete in [true, false] {
            for same_identity in [true, false] {
                for (basis, check) in [
                    ("theorem", "model_review"),
                    ("exact_computation", "producer"),
                    ("sample", "none"),
                    ("oracle", "peer"),
                ] {
                    for job in [None, Some("cut_off")] {
                        let c = Case { basis, check, job, search: None, covered: true, same_identity, complete, shape };
                        check_one(&c, &pins, &functions, &unknown);
                        cases += 1;
                    }
                }
            }
        }
    }
    assert_eq!(cases, 6 * 7 * 6 * 3 * 2 + 3 * 2 * 2 * 4 * 2);
}

// ------------------------------------------------------------------------------------------------- CT-PS1-23

/// A 0.4 preview certificate as `session.py` returns it, built with 0.4's own canonical form
/// (`json.dumps(sort_keys=True, separators=(',', ':'))`): the identifier digests the eleven initial fields only.
const CERTIFICATE: &str = r#"{"assistance":"manual","certificate_id":"6900ae4f98417f500b11b319e919cce05053e65c7ae0555a87e57a446be6db8f","conflicts":[{"orbit":4,"pieces":2}],"evidence":"Legal seed replay plus exact conjugation/composition over the retained full model","format":"C600-STUDIO-CERTIFICATE-v1","model_id":"58c83f336e451aa64147fe1b8db2d60d7d43806aa3a35122f8403d5af51253f9","native_windows_equivalence":false,"net_moved_pieces":3,"net_moved_stickers":12,"note":"fixture","post_state":"0b404c31261398e638b705af23ba8c8de24f6eda2c9a4ef70e1d03ae3949d3f9","pre_state":"2bb5baadcd361d5ea5137c2c2a115b162bf9077feda3dbec2d94d38adc979348","preview_ms":41.5,"primitive_count":"3","recipe":[{"kind":"word","moves":[3,-7,3]}],"source_to_destination_sha256":"9dcb7fdaf4e39c06335ae39b341115d9617c0e67465cae373e9ccdb064a18cb4","star_count":0,"support":[{"orbit":4,"pieces":2}],"token":"fixture-token"}"#;
/// The identifier 0.4 computes over the initial public object.
const CERTIFICATE_ID: &str = "6900ae4f98417f500b11b319e919cce05053e65c7ae0555a87e57a446be6db8f";
/// What a producer that digested every field, diagnostics and token included, would have recorded instead.
const ALL_FIELDS_ID: &str = "e727bb3811c8bf1d469d73da264422958c6a6d3c8fc92ee32be2ef22f19786b7";

fn certificate_with(change: impl FnOnce(&mut BTreeMap<String, Value>)) -> Vec<u8> {
    let Ok(Value::Object(mut fields)) = legacy_json::parse(CERTIFICATE.as_bytes()) else { panic!("the fixture") };
    change(&mut fields);
    Value::Object(fields).canonical().into_bytes()
}

/// CT-PS1-23 (source, whole: a 0.4 certificate built with 0.4's canonical form and digest, imported as returned, with
/// its timing, context, state-C and state-D fields changed, added or removed, and with identifiers computed over other
/// field sets): the identifier uses the original pre-diagnostic field set; imported timing or context fields never
/// upgrade the recorded state B or its "stated: not replayed" mode, and never recreate state C or D.
#[test]
fn ct_ps1_23_legacy_certificates_keep_their_identifier_and_state() {
    let imported = legacy::import_certificate(CERTIFICATE.as_bytes()).expect("a 0.4 certificate");
    assert_eq!(imported.id, CERTIFICATE_ID);
    assert_eq!((imported.state, imported.mode.clone()), (State::RecordedNetAction, CheckMode::StatedNotReplayed));
    let Value::Object(initial) = &imported.initial else { panic!("an object") };
    let initial: Vec<&str> = initial.keys().map(String::as_str).collect();
    let mut expected = vec![
        "format",
        "model_id",
        "recipe",
        "source_to_destination_sha256",
        "primitive_count",
        "star_count",
        "support",
        "pre_state",
        "post_state",
        "evidence",
        "native_windows_equivalence",
    ];
    expected.sort_unstable();
    assert_eq!(initial, expected, "verify.py's HASH_FIELDS");
    let context: Vec<&str> = imported.context.keys().map(String::as_str).collect();
    assert_eq!(context, ["assistance", "net_moved_pieces", "net_moved_stickers", "note", "preview_ms"]);
    assert_eq!(imported.dropped, ["conflicts", "token"]);

    // Changed or added timing and context fields: the same identifier and the same recorded claim.
    let upgraded = certificate_with(|f| {
        f.insert("preview_ms".into(), Value::Number("0.5".into()));
        f.insert("note".into(), Value::String("verified again in 1.0".into()));
        f.insert("assistance".into(), Value::String("independent_checker".into()));
        f.insert("net_moved_stickers".into(), Value::Int(0));
        f.insert("check_mode".into(), Value::String("fresh_exact_audit".into()));
        f.insert("state".into(), Value::String("A".into()));
        f.insert("verified".into(), Value::Bool(true));
    });
    let again = legacy::import_certificate(&upgraded).expect("still the same certificate");
    assert_eq!(again.id, CERTIFICATE_ID);
    assert_eq!((again.state, again.mode.clone()), (State::RecordedNetAction, CheckMode::StatedNotReplayed));
    assert_eq!(again.initial, imported.initial);
    assert!(["check_mode", "state", "verified"].iter().all(|k| again.context.contains_key(*k)), "context only");

    // State C and D are never recreated: an empty or a different conflict list, a missing token.
    for conflicts in [Value::Array(Vec::new()), Value::String("orbit 9".into())] {
        let changed = certificate_with(|f| {
            f.insert("conflicts".into(), conflicts);
            f.remove("token");
        });
        let c = legacy::import_certificate(&changed).expect("imported");
        assert_eq!(c.dropped, ["conflicts"]);
        assert!(!c.context.contains_key("conflicts") && !c.context.contains_key("token"));
        assert_eq!(c.state, State::RecordedNetAction);
    }

    // The identifier is the original field set's: an identifier over every field, or a changed initial field, is
    // refused; so is a missing initial field or another format.
    let all = certificate_with(|f| {
        f.insert("certificate_id".into(), Value::String(ALL_FIELDS_ID.into()));
    });
    assert_eq!(outcome(legacy::import_certificate(&all)), DIGEST);
    let moved = certificate_with(|f| {
        f.insert("primitive_count".into(), Value::String("4".into()));
    });
    assert_eq!(outcome(legacy::import_certificate(&moved)), DIGEST);
    let missing = certificate_with(|f| {
        f.remove("evidence");
    });
    assert_eq!(outcome(legacy::import_certificate(&missing)), FORMAT);
    let other = certificate_with(|f| {
        f.insert("format".into(), Value::String("C600-STUDIO-SESSION-v1".into()));
    });
    assert_eq!(outcome(legacy::import_certificate(&other)), FORMAT);
}

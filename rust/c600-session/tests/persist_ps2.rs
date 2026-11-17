//! LP2 conformance tests for chapter 05's editions and recorded migration (`c600_session::persist::migrate`, M6's
//! remaining rows): CT-PS2-01 to CT-PS2-05.
//!
//! Migrations run over the packet's JI23 word and over facts envelopes written as FP.19 stores them; publication runs
//! over a `MemDisk` that fails, tears and crashes on request. Expected values come from the packet, the chapter or the
//! test's own construction, never from the library under test.
//!
//! Evidence kind: source and fixture. One run covers one platform. Nothing here is Windows/DirectX evidence or a
//! power-loss proof (FP.28: publication metadata does not prove power-loss safety).

mod persist_support;

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;

use c600_digest::sha256_hex;
use c600_evidence::Edition;
use c600_session::disk::{Fault, MemDisk};
use c600_session::persist::codec::{self, Json};
use c600_session::persist::codes::{DIGEST, FORMAT, IDENTITY, PROVENANCE, VERSION};
use c600_session::persist::facts::{self, Derived, Functions};
use c600_session::persist::migrate::{
    self, Conversion, Converter, Migration, Outcome, Package, PublishError, Publisher, RecordedInput, Status,
};
use c600_session::persist::record::{self, Read};
use c600_session::persist::word::{self, Store};
use c600_session::persist::Refusal;
use persist_support::*;

const SOURCE_EDITION: &str = "ji23-edition-1";
const CONVERTER: &[u8] = b"fixture converter, edition 2";
const MAPPING: &[u8] = b"fixture mapping table, edition 2";
const MAPPING_3: &[u8] = b"fixture mapping table, edition 3";

// ------------------------------------------------------------------------------------------------- fixtures

/// FP.8's DigestRef, written by the test.
fn digest(role: &str, bytes: &[u8]) -> Json {
    Json::object([("role", Json::text(role)), ("sha256", Json::Text(sha256_hex(bytes)))])
}

/// FP.11's Pin, written by the test.
fn pin(version: &str, source: &[u8]) -> Json {
    Json::object([("version", Json::text(version)), ("source", digest("source", source))])
}

fn with(record: &Json, change: impl FnOnce(&mut Json)) -> Json {
    let mut changed = record.clone();
    change(&mut changed);
    changed
}

/// The JI23 word with its part included, as the packet's `part` relation case stores it.
fn source_word(f: &Ji23) -> Json {
    with(&f.record, |r| set(at(r, &["payload"]), "parts", part("A-part", &f.part_ref)))
}

/// Edition 2's change: a named span over the whole word, which generates no letter (FP.13).
fn name_whole_word(record: &mut Json) {
    let span = Json::object([
        ("name", Json::text("W")),
        ("start", Json::text("0")),
        ("end", Json::text("3")),
        ("relation", Json::Null),
    ]);
    push(at(record, &["payload", "annotations"]), span);
}

/// The included part's newer definition: one more named span, so new content under a new digest.
fn newer_part(f: &Ji23) -> Json {
    let sha = text_of(f.part_ref.get("content").and_then(|c| c.get("sha256")));
    let span = Json::object([
        ("name", Json::text("Z")),
        ("start", Json::text("0")),
        ("end", Json::text("1")),
        ("relation", Json::Null),
    ]);
    with(&f.records[&sha], |r| push(at(r, &["payload", "annotations"]), span))
}

fn text_of(value: Option<&Json>) -> String {
    match value {
        Some(Json::Text(t)) => t.clone(),
        other => panic!("not a text: {other:?}"),
    }
}

fn mapping() -> Vec<Json> {
    vec![digest("bytes", MAPPING)]
}

fn word_input<'a>(source: &Json, bytes: &'a [u8]) -> RecordedInput<'a> {
    RecordedInput {
        format: "c600-word",
        version: Json::Int(1),
        edition: Some(SOURCE_EDITION),
        bytes,
        identity: source.get("identity").cloned().expect("an identity"),
    }
}

/// `migrate_record` on a word with the fixture converter's pin.
fn run(
    source: &Json,
    convert: &dyn Fn(&[u8]) -> Result<Conversion, Refusal>,
    inputs: &mut dyn FnMut() -> Vec<Json>,
) -> Result<Migration, Refusal> {
    let bytes = codec::canonical(source);
    let mut converter = Converter { pin: pin("converter-2", CONVERTER), inputs, convert };
    migrate::migrate_record(&word_input(source, &bytes), &mut converter)
}

/// A conversion that applies `change` to the source and declares `notes` as its semantic changes.
fn migrate_word(
    source: &Json,
    change: &dyn Fn(&mut Json),
    notes: &[&str],
    checks: &[Json],
) -> Result<Migration, Refusal> {
    let convert = |old: &[u8]| -> Result<Conversion, Refusal> {
        let mut target = codec::read_canonical(old)?;
        change(&mut target);
        Ok(Conversion {
            target: Some(target),
            outcomes: vec![Outcome::new("word", Status::Converted, "edition 2 annotations")],
            checks: checks.to_vec(),
            semantic_changes: notes.iter().map(|n| n.to_string()).collect(),
        })
    };
    run(source, &convert, &mut mapping)
}

fn replayed(_: &Package) -> Result<(), Refusal> {
    Ok(())
}

/// What a publication call answered.
fn status<T>(result: &Result<T, PublishError>) -> String {
    match result {
        Ok(_) => "ok".to_string(),
        Err(PublishError::Refused(r)) => r.code.to_string(),
        Err(PublishError::NotPublished(_)) => "not published".to_string(),
        Err(PublishError::UnknownOutcome) => migrate::UNKNOWN_OUTCOME.to_string(),
    }
}

/// The active publication as a reader that reopens the store sees it, validated with its generation.
fn active(disk: &MemDisk) -> Option<Json> {
    Publisher::new(disk.clone()).active().unwrap_or_else(|e| panic!("the active generation validates: {e:?}"))
}

/// A store whose active generation holds `head` alone.
fn first_generation(head: &Json) -> (MemDisk, Json) {
    let disk = MemDisk::new();
    let package =
        Package { records: vec![head.clone()], originals: Vec::new(), head: head.clone(), validation: head.clone() };
    let meta = Publisher::new(disk.clone()).publish("generation-1", &package, &replayed).expect("the first generation");
    (disk, meta)
}

fn object(disk: &MemDisk, bytes: &[u8]) -> Option<Vec<u8>> {
    disk.durable(&format!("objects/{}", sha256_hex(bytes)))
}

/// FP.19's Cache.
fn cache(inputs: &[Json], functions: Vec<Json>) -> Json {
    Json::object([
        ("inputs", Json::List(inputs.to_vec())),
        ("functions", Json::List(functions)),
        ("output", parse(&raw_blob(b"fixture cached display word"))),
    ])
}

fn pins_of(editions: &Json, names: &[&str]) -> Vec<Json> {
    names.iter().map(|n| editions.get(n).cloned().expect("a pin")).collect()
}

/// A facts payload without its function editions and caches: what an edition change leaves unchanged (PS2.6).
fn without_editions(payload: &Json) -> Json {
    with(payload, |p| {
        remove(p, "editions");
        remove(p, "caches");
    })
}

fn payload_of(record: &Json) -> &Json {
    record.get("payload").expect("a payload")
}

// ------------------------------------------------------------------------------------------------- CT-PS2-01

/// CT-PS2-01 (source, whole: a word migration's record compared field by field with the test's own transcription of
/// FP.29, source fields the bytes do not carry, targetless and fabricated-target conversions, then its publication over
/// a `MemDisk` with every write failing in turn as an error, a partial write or a write that took effect, an outage
/// from each write on with torn and complete pointer tails kept at the crash, a torn pointer line under each failure
/// of the next publication, a damaged pointer line, and refused replays and packages): the migration record binds
/// source, target, converter, declared inputs and every version; a failed validation or write leaves the old bytes and
/// the active generation recoverable, and a retry publishes.
#[test]
fn ct_ps2_01_migration_binds_its_inputs_and_failures_keep_the_old_generation() {
    let f = ji23();
    let source = source_word(&f);
    let original = codec::canonical(&source);
    let (pins, _) = edition_pins();
    let check = evidence_record(parse(&payload_text(&Case::plain(), &pins)));

    // PS2.3 and FP.29: the record binds the source's format, version, edition and bytes, the target's Ref, the
    // converter's pin, the inputs it declared, each item's outcome, the check records and the semantic changes.
    let migration =
        migrate_word(&source, &name_whole_word, &[], std::slice::from_ref(&check)).expect("a preserved conversion");
    let target = migration.target.clone().expect("a converted target");
    assert_eq!(target, with(&source, name_whole_word));
    let payload = Json::object([
        (
            "source",
            Json::object([
                ("format", Json::text("c600-word")),
                ("version", Json::Int(1)),
                ("edition", Json::text(SOURCE_EDITION)),
                ("bytes", digest("bytes", &original)),
            ]),
        ),
        ("target", ref_of(&target)),
        ("converter", pin("converter-2", CONVERTER)),
        ("inputs", Json::List(mapping())),
        ("outcomes", parse(r#"[{"item":"word","status":"converted","reason":"edition 2 annotations"}]"#)),
        ("checks", Json::List(vec![ref_of(&check)])),
        ("semantic_changes", Json::List(Vec::new())),
    ]);
    let expected = Json::object([
        ("format", Json::text("c600-migration")),
        ("version", Json::Int(1)),
        ("identity", source.get("identity").cloned().unwrap()),
        ("payload", payload),
        ("provenance", Json::Null),
    ]);
    assert_eq!(migration.record, expected);
    assert_eq!(migration.checks, vec![check.clone()]);
    let bytes = record::write_record(&migration.record).expect("a migration record is written");
    assert_eq!(record::read_record(&bytes, "c600-migration"), Ok(Read::Active(expected)));

    // A converter pin without its version binds nothing.
    let convert = |old: &[u8]| -> Result<Conversion, Refusal> {
        Ok(Conversion {
            target: Some(codec::read_canonical(old)?),
            outcomes: vec![Outcome::new("word", Status::Converted, "unchanged")],
            checks: Vec::new(),
            semantic_changes: Vec::new(),
        })
    };
    let mut unpinned = Converter {
        pin: Json::object([("source", digest("source", CONVERTER))]),
        inputs: &mut mapping,
        convert: &convert,
    };
    assert_eq!(outcome(migrate::migrate_record(&word_input(&source, &original), &mut unpinned)), FORMAT);

    // PS2.3: the source fields describe the bytes. A format, version or identity the bytes do not carry, bytes that
    // claim a record format and are no record, or a record given as an older format, are refused before the converter
    // runs.
    let calls = Cell::new(0);
    let counting = |old: &[u8]| -> Result<Conversion, Refusal> {
        calls.set(calls.get() + 1);
        convert(old)
    };
    let identity = source.get("identity").cloned().expect("an identity");
    let other_identity = with(&identity, |i| set(i, "contract_rev", Json::text("JI23-fixture-2")));
    let newer = codec::canonical(&with(&source, |r| set(r, "version", Json::Int(2))));
    let no_record = codec::canonical(&Json::object([("note", Json::text("no record"))]));
    let claims: [(&str, &[u8], &Json, &str); 5] = [
        ("c600-word", &newer, &identity, FORMAT),
        ("c600-macro", &original, &identity, FORMAT),
        ("c600-word", &original, &other_identity, IDENTITY),
        ("c600-word", &no_record, &identity, FORMAT),
        ("magic600-session-proto/1", &original, &identity, FORMAT),
    ];
    for (format, bytes, identity, code) in claims {
        let input = RecordedInput {
            format,
            version: Json::Int(1),
            edition: Some(SOURCE_EDITION),
            bytes,
            identity: identity.clone(),
        };
        let mut converter = Converter { pin: pin("converter-2", CONVERTER), inputs: &mut mapping, convert: &counting };
        assert_eq!(outcome(migrate::migrate_record(&input, &mut converter)), code, "{format} {identity:?}");
    }
    assert_eq!(calls.get(), 0, "no converter ran");

    // FP.29: a conversion that converts nothing has no target, and nothing activates; a target with no converted item,
    // or a converted item without a target, is refused.
    let recorded = |_: &[u8]| -> Result<Conversion, Refusal> {
        Ok(Conversion {
            target: None,
            outcomes: vec![Outcome::new("word", Status::Recorded, "no edition-2 reading")],
            checks: Vec::new(),
            semantic_changes: Vec::new(),
        })
    };
    let kept = run(&source, &recorded, &mut mapping).expect("a recorded item");
    assert_eq!((kept.target.as_ref(), payload_of(&kept.record).get("target")), (None, Some(&Json::Null)));
    assert_eq!(kept.package(&original), None, "nothing activates; the original stays as it was");
    let fabricated = |old: &[u8]| -> Result<Conversion, Refusal> {
        Ok(Conversion {
            target: Some(codec::read_canonical(old)?),
            outcomes: vec![Outcome::new("word", Status::Refused, "letters outside the menu")],
            checks: Vec::new(),
            semantic_changes: Vec::new(),
        })
    };
    assert_eq!(outcome(run(&source, &fabricated, &mut mapping)), FORMAT);
    let missing = |_: &[u8]| -> Result<Conversion, Refusal> {
        Ok(Conversion {
            target: None,
            outcomes: vec![Outcome::new("word", Status::Converted, "converted")],
            checks: Vec::new(),
            semantic_changes: Vec::new(),
        })
    };
    assert_eq!(outcome(run(&source, &missing, &mut mapping)), FORMAT);

    // FP.28: the clean publication retains the original beside the new generation, which names the old one.
    let package = migration.package(&original).expect("a package");
    assert_eq!(package.originals, vec![original.clone()]);
    let (clean, first) = first_generation(&source);
    let start = clean.writes();
    let second =
        Publisher::new(clean.clone()).publish("generation-2", &package, &replayed).expect("a clean publication");
    let span = clean.writes() - start;
    assert_eq!(active(&clean), Some(second.clone()));
    assert_eq!(second.get("previous"), Some(&digest("content", &codec::canonical(&first))));
    assert_eq!(second.get("next"), Some(&content_of(&target)));
    assert_eq!(second.get("validation"), Some(&ref_of(&migration.record)));
    let mut reader = Publisher::new(clean.clone());
    let records = migrate::records_of(&mut reader, &second).expect("the new generation's records");
    let mut stored: Vec<String> = records.keys().cloned().collect();
    let mut wanted: Vec<String> =
        [&target, &migration.record, &check, &source].iter().map(|r| sha256_hex(&codec::canonical(r))).collect();
    stored.sort();
    wanted.sort();
    assert_eq!(stored, wanted, "the target, the migration, its check and the retained original");
    assert_eq!(reader.object(&sha256_hex(&original)).ok(), Some(original.clone()));

    // Each write of the publication fails in turn. A switch is reported only when a reopened reader sees it; after a
    // crash the reader validates the old generation unless the switch was reported, the original bytes stay, and a
    // retry publishes.
    let mut seen = BTreeMap::new();
    for index in 0..span {
        for fault in [Fault::Error, Fault::Partial(10), Fault::Persisted] {
            let (disk, first) = first_generation(&source);
            disk.fail_write(disk.writes() + index, fault);
            let result = Publisher::new(disk.clone()).publish("generation-2", &package, &replayed);
            let label = status(&result);
            let reported = result.as_ref().ok().cloned();
            match &result {
                Ok(meta) => assert_eq!(active(&disk).as_ref(), Some(meta), "{index} {fault:?}: visible"),
                Err(PublishError::NotPublished(_)) => {
                    assert_eq!(active(&disk), Some(first.clone()), "{index} {fault:?}")
                }
                Err(e) => panic!("{index} {fault:?}: one failed write is reconciled from the pointer, not {e:?}"),
            }
            disk.crash();
            assert_eq!(active(&disk), Some(reported.clone().unwrap_or_else(|| first.clone())), "{index} {fault:?}");
            assert_eq!(object(&disk, &original), Some(original.clone()), "{index} {fault:?}: the old bytes");
            if reported.is_none() {
                let retried = Publisher::new(disk.clone()).publish("generation-2", &package, &replayed);
                assert_eq!(retried.ok(), Some(second.clone()), "{index} {fault:?}: the retry publishes");
                disk.crash();
                assert_eq!(active(&disk), Some(second.clone()));
            }
            *seen.entry(label).or_insert(0) += 1;
        }
    }
    assert!(seen.contains_key("ok") && seen.contains_key("not published"), "{seen:?}");

    // An outage from each write on. Before the pointer nothing switches; at the pointer the outcome is unknown and the
    // pointer the crash leaves decides it: a torn line is no switch, a complete line names the durable new generation.
    let (mut uncertain, mut switched) = (0, 0);
    for index in 0..span {
        for kept in [0, 10, 64, 65] {
            let (disk, first) = first_generation(&source);
            disk.fail_from(disk.writes() + index);
            let result = Publisher::new(disk.clone()).publish("generation-2", &package, &replayed);
            let label = status(&result);
            assert!(label == "not published" || label == migrate::UNKNOWN_OUTCOME, "{index}: {label}");
            uncertain += usize::from(label == migrate::UNKNOWN_OUTCOME);
            disk.crash_keeping("active", kept);
            let after = active(&disk);
            if after != Some(first.clone()) {
                assert_eq!(
                    label,
                    migrate::UNKNOWN_OUTCOME,
                    "{index} {kept}: only an uncertain switch may have happened"
                );
                assert_eq!(after, Some(second.clone()), "{index} {kept}");
                switched += 1;
                continue;
            }
            assert_eq!(object(&disk, &original), Some(original.clone()), "{index} {kept}");
            let retried = Publisher::new(disk.clone()).publish("generation-2", &package, &replayed);
            assert_eq!(retried.ok(), Some(second.clone()), "{index} {kept}: the retry publishes");
        }
    }
    assert!(switched > 0 && uncertain > switched, "both sides of an uncertain switch: {uncertain} {switched}");

    // A torn pointer line is never completed later. This publication's line, torn after its 64 digits, is no switch;
    // then no write of a later publication, failed, torn after one byte or taking effect unreported, makes it active.
    let later =
        Package { records: vec![check.clone()], originals: Vec::new(), head: check.clone(), validation: check.clone() };
    let torn = || {
        let (disk, first) = first_generation(&source);
        disk.fail_write(disk.writes() + span - 2, Fault::Partial(64));
        let result = Publisher::new(disk.clone()).publish("generation-2", &package, &replayed);
        assert_eq!(status(&result), "not published");
        disk.crash();
        assert_eq!(active(&disk), Some(first.clone()));
        (disk, first)
    };
    let (probe, _) = torn();
    let start = probe.writes();
    let third = Publisher::new(probe.clone()).publish("generation-3", &later, &replayed).expect("published");
    let later_span = probe.writes() - start;
    probe.crash();
    assert_eq!(active(&probe), Some(third.clone()));
    for index in 0..later_span {
        for fault in [Fault::Error, Fault::Partial(1), Fault::Persisted] {
            let (disk, first) = torn();
            disk.fail_write(disk.writes() + index, fault);
            let result = Publisher::new(disk.clone()).publish("generation-3", &later, &replayed);
            disk.crash_keeping("active", 1);
            let after = active(&disk);
            assert_ne!(after, Some(second.clone()), "{index} {fault:?}: the torn generation stays unpublished");
            assert!(after == Some(first.clone()) || after == Some(third.clone()), "{index} {fault:?}: {after:?}");
            assert!(after == Some(first) || status(&result) != "not published", "{index} {fault:?}");
        }
    }

    // A damaged last pointer line is refused, never skipped for the line before it, and nothing is published over it.
    let (disk, _) = first_generation(&source);
    Publisher::new(disk.clone()).publish("generation-2", &package, &replayed).expect("published");
    let mut log = disk.durable("active").expect("a pointer");
    log.remove(log.len() - 2);
    disk.overwrite("active", &log);
    assert_eq!(status(&Publisher::new(disk.clone()).active()), FORMAT);
    let before = disk.writes();
    assert_eq!(status(&Publisher::new(disk.clone()).publish("generation-3", &later, &replayed)), FORMAT);
    assert_eq!(disk.writes(), before);

    // A refused replay or an inconsistent package writes nothing.
    let (disk, first) = first_generation(&source);
    let before = disk.writes();
    let mismatch = |_: &Package| -> Result<(), Refusal> { Err(Refusal::new(DIGEST)) };
    assert_eq!(status(&Publisher::new(disk.clone()).publish("generation-2", &package, &mismatch)), DIGEST);
    let orphan = Package { originals: Vec::new(), ..package.clone() };
    assert_eq!(status(&Publisher::new(disk.clone()).publish("generation-2", &orphan, &replayed)), FORMAT);
    let elsewhere = Package { head: source.clone(), ..package.clone() };
    assert_eq!(status(&Publisher::new(disk.clone()).publish("generation-2", &elsewhere, &replayed)), FORMAT);
    let unchecked =
        Package { records: package.records.iter().filter(|r| **r != check).cloned().collect(), ..package.clone() };
    assert_eq!(unchecked.records.len() + 1, package.records.len());
    assert_eq!(status(&Publisher::new(disk.clone()).publish("generation-2", &unchecked, &replayed)), FORMAT);
    assert_eq!(disk.writes(), before, "nothing was written");
    assert_eq!(active(&disk), Some(first));
}

// ------------------------------------------------------------------------------------------------- CT-PS2-02

/// CT-PS2-02 (source, whole: the JI23 word with its included part, converted with only a new named span, then with
/// each preserved field changed alone, without and with a declared semantic change, and its part definition changed
/// under and beside the pinned digest): an edition change keeps the literal word, the alphabet, the identity, the part
/// pins, the length, the start digest and the provenance; changed referenced content is never relinked silently, and
/// a declared relink is a different word with its own content identity.
#[test]
fn ct_ps2_02_edition_changes_keep_the_literal_word_and_never_relink_silently() {
    let f = ji23();
    let source = source_word(&f);
    let store = Store { blobs: &f.blobs, records: &f.records, inverse: &f.inverse, menu: &f.menu };
    let letters = word::admit(&source, &store).expect("the source word");

    // A new span generates no letter: the converted word keeps every preserved field.
    let kept = migrate_word(&source, &name_whole_word, &[], &[]).expect("a preserved conversion");
    let target = kept.target.expect("a target");
    assert_eq!(word::admit(&target, &store), Ok(letters.clone()));
    let preserved = |r: &Json| {
        let payload = payload_of(r);
        (
            r.get("identity").cloned(),
            r.get("provenance").cloned(),
            ["alphabet", "start", "length", "letters", "parts"].map(|field| payload.get(field).cloned()),
        )
    };
    assert_eq!(preserved(&target), preserved(&source));

    // Each preserved field changed alone, in a record that is itself well formed, is refused unless the conversion
    // declares the change; an identity change is refused even then (PS2.4).
    let newer = newer_part(&f);
    let newer_ref = ref_of(&newer);
    let raw = bytes_of_hex("0c000c000c00");
    let changes: [(&str, &dyn Fn(&mut Json), &str); 7] = [
        ("identity", &|r| set(at(r, &["identity"]), "contract_rev", Json::text("JI23-fixture-2")), IDENTITY),
        ("provenance", &|r| set(at(r, &["provenance", "block"]), "form", Json::text("flow_runtime")), PROVENANCE),
        ("alphabet", &|r| set(at(r, &["payload"]), "alphabet", Json::text("signed_generators")), DIGEST),
        ("start", &|r| set(at(r, &["payload"]), "start", digest("poses", b"another start pose")), DIGEST),
        ("length", &|r| set(at(r, &["payload"]), "length", Json::text("4")), DIGEST),
        (
            "letters",
            &|r| {
                set(at(r, &["payload"]), "chunks", Json::List(vec![parse(&raw_blob(&raw))]));
                set(at(r, &["payload"]), "letters", digest("bytes", &raw));
            },
            DIGEST,
        ),
        ("parts", &|r| set(at(r, &["payload"]), "parts", part("A-part", &newer_ref)), DIGEST),
    ];
    for (field, change, code) in changes {
        let changed = with(&source, change);
        assert!(record::write_record(&changed).is_ok(), "{field}: the changed record is itself well formed");
        assert_eq!(outcome(migrate_word(&source, change, &[], &[])), code, "{field}: silent change");
        let note = format!("{field} changed in edition 2");
        let declared = migrate_word(&source, change, &[note.as_str()], &[]);
        if field == "identity" {
            assert_eq!(outcome(declared), IDENTITY, "a new reader never authorises an identity change");
        } else {
            let declared = declared.unwrap_or_else(|r| panic!("{field}: a declared change: {r}"));
            let target = declared.target.expect("a target");
            assert_ne!(content_of(&target), content_of(&source), "{field}: a changed word has its own identity");
            let notes = payload_of(&declared.record).get("semantic_changes");
            assert_eq!(notes, Some(&Json::List(vec![Json::text(&note)])), "{field}");
        }
    }

    // The old word never resolves to the newer definition: changed content under the pinned digest is refused, and a
    // store that holds only the newer definition has nothing under the pin.
    let pinned = text_of(f.part_ref.get("content").and_then(|c| c.get("sha256")));
    let swapped = BTreeMap::from([(pinned, newer.clone())]);
    let newer_only = BTreeMap::from([(text_of(newer_ref.get("content").and_then(|c| c.get("sha256"))), newer.clone())]);
    let under_pin = Store { blobs: &f.blobs, records: &swapped, inverse: &f.inverse, menu: &f.menu };
    assert_eq!(outcome(word::admit(&source, &under_pin)), DIGEST);
    let beside_pin = Store { blobs: &f.blobs, records: &newer_only, inverse: &f.inverse, menu: &f.menu };
    assert_eq!(outcome(word::admit(&source, &beside_pin)), FORMAT);
    // The declared relink resolves against the newer definition, with the same literal word.
    let relink: &dyn Fn(&mut Json) = &|r| set(at(r, &["payload"]), "parts", part("A-part", &newer_ref));
    let relinked =
        migrate_word(&source, relink, &["A-part relinked to its edition-2 definition"], &[]).expect("declared");
    let relinked = relinked.target.expect("a target");
    assert_eq!(word::admit(&relinked, &beside_pin), Ok(letters));
    assert_eq!(outcome(word::admit(&relinked, &store)), FORMAT, "and never against the old one");
}

// ------------------------------------------------------------------------------------------------- CT-PS2-03

/// The expected migration outcome of one root's new judgement.
fn root_outcome(root: &str, derived: &Derived) -> Json {
    let (status, reason) = match derived {
        Derived::Word(w) => ("converted", format!("row {}: {}", w.row, w.display())),
        Derived::Inconsistent(e) => ("converted", format!("row 0: {}", e.code)),
        Derived::NoVerdict => ("recorded", "no verdict: function edition unavailable".to_string()),
    };
    Json::object([
        ("item", Json::Text(format!("root {root}"))),
        ("status", Json::text(status)),
        ("reason", Json::Text(reason)),
    ])
}

/// CT-PS2-03 (source, whole: the packet's two editions vectors, and the display-function change of two stored facts
/// envelopes, a producer-checked word and an inconsistent leaf, each with a cache of the old display edition, a cache
/// of unchanged functions and a cache of an unknown function, then the change published; and a change to an edition
/// the build cannot run): a facts-function edition change keeps the facts and the old interpretation, records each
/// root's new output, inconsistency or missing verdict with the edition note, and invalidates every derived cache that
/// does not pin exactly the new functions.
#[test]
fn ct_ps2_03_function_edition_change_keeps_facts_and_the_old_interpretation() {
    // The packet's editions vectors.
    for vector in vectors("editions") {
        let got = facts::editions_result(&block(vector.get("given").expect("given")));
        match vector.get("expected").expect("expected") {
            json::Json::Str(code) => assert_eq!(&outcome(got), code, "{}", text(&vector, "id")),
            expected => assert_eq!(got, Ok(block(expected)), "{}", text(&vector, "id")),
        }
    }

    let (old, new) = edition_pins();
    let functions = Functions::new().with(old.clone(), Edition::draft_1()).with(new.clone(), stricter());
    let converter = pin("edition-converter-1", b"fixture edition converter");
    let fact_ref = digest("content", b"fixture fact input");
    let display_cache =
        cache(std::slice::from_ref(&fact_ref), pins_of(&old, &["order", "summary", "display", "import"]));
    let order_cache = cache(std::slice::from_ref(&fact_ref), pins_of(&old, &["order", "summary"]));
    let foreign_cache = cache(std::slice::from_ref(&fact_ref), vec![pin("fixture-9", b"another display function")]);
    let producer = Case { check: "producer", ..Case::plain() };
    let foreign_leaf = Case { same_identity: false, ..Case::plain() };
    for (name, case) in [("producer", producer), ("foreign leaf", foreign_leaf)] {
        let payload = with(&parse(&payload_text(&case, &old)), |p| {
            let caches = vec![display_cache.clone(), order_cache.clone(), foreign_cache.clone()];
            set(p, "caches", Json::List(caches));
        });
        let record = evidence_record(payload.clone());
        let old_bytes = record::write_record(&record).expect("written");
        let change = migrate::change_editions(&old_bytes, &new, &functions, &converter).expect("an edition change");

        // The old record and its interpretation stay reproducible from the unchanged bytes.
        let Ok(Read::Active(reread)) = record::read_record(&old_bytes, "c600-evidence") else { panic!("{name}") };
        assert_eq!(reread, record, "{name}");
        let stored = facts::admit(payload_of(&reread)).expect("the old facts");
        assert_eq!(change.old, facts::derive(&stored, &old, &functions), "{name}");
        assert_eq!(change.new, facts::derive(&stored, &new, &functions), "{name}");
        match name {
            "producer" => assert_ne!(change.old, change.new, "the stricter display edition judges anew"),
            _ => assert!(matches!(change.new[0].1, Derived::Inconsistent(_)), "{name}: {:?}", change.new),
        }

        // The new record differs only in its editions and surviving caches.
        let new_payload = payload_of(&change.record);
        assert_eq!(without_editions(new_payload), without_editions(&payload), "{name}: the facts are unchanged");
        assert_eq!(new_payload.get("editions"), Some(&new));
        assert_eq!(new_payload.get("caches"), Some(&Json::List(vec![order_cache.clone()])), "{name}");
        assert_eq!(change.stale, vec![display_cache.clone(), foreign_cache.clone()], "{name}");
        for field in ["format", "version", "identity", "provenance"] {
            assert_eq!(change.record.get(field), record.get(field), "{name} {field}");
        }

        // PS2.6's note and FP.29's record: old and new editions, the unchanged fact input and each root's new output.
        let facts_digest = sha256_hex(&codec::canonical(&without_editions(&payload)));
        let note = Json::object([
            ("changed", Json::List(vec![Json::text("display")])),
            ("facts", Json::Text(facts_digest)),
            ("old", Json::Text(sha256_hex(&codec::canonical(&old)))),
            ("new", Json::Text(sha256_hex(&codec::canonical(&new)))),
        ]);
        assert_eq!(change.note, note, "{name}");
        let migration = payload_of(&change.migration.record);
        let source = Json::object([
            ("format", Json::text("c600-evidence")),
            ("version", Json::Int(1)),
            ("edition", Json::Null),
            ("bytes", digest("bytes", &old_bytes)),
        ]);
        assert_eq!(migration.get("source"), Some(&source), "{name}");
        assert_eq!(migration.get("target"), Some(&ref_of(&change.record)), "{name}");
        assert_eq!(migration.get("converter"), Some(&converter), "{name}");
        let inputs = vec![
            digest("content", &codec::canonical(&without_editions(&payload))),
            digest("content", &codec::canonical(&old)),
            digest("content", &codec::canonical(&new)),
        ];
        assert_eq!(migration.get("inputs"), Some(&Json::List(inputs)), "{name}");
        let mut outcomes =
            vec![parse(r#"{"item":"record","status":"converted","reason":"new function editions; facts unchanged"}"#)];
        outcomes.extend(change.new.iter().map(|(root, derived)| root_outcome(root, derived)));
        assert_eq!(migration.get("outcomes"), Some(&Json::List(outcomes)), "{name}");
        let semantic = Json::List(vec![Json::text("display function edition changed")]);
        assert_eq!(migration.get("semantic_changes"), Some(&semantic), "{name}");

        // Published, the new generation keeps the old record beside the new one.
        let (disk, _) = first_generation(&record);
        let package = change.migration.package(&old_bytes).expect("a package");
        let meta = Publisher::new(disk.clone()).publish("editions-2", &package, &replayed).expect("published");
        let records = migrate::records_of(&mut Publisher::new(disk.clone()), &meta).expect("records");
        assert_eq!(records.get(&sha256_hex(&old_bytes)), Some(&record), "{name}: the old record stays");
        assert_eq!(records.get(&sha256_hex(&codec::canonical(&change.record))), Some(&change.record), "{name}");
    }

    // A change to an edition the build cannot run keeps the facts and records each root without a verdict.
    let unknown = with(&new, |e| set(at(e, &["summary"]), "version", Json::text("fixture-2")));
    let record = evidence_record(parse(&payload_text(&Case::plain(), &old)));
    let old_bytes = record::write_record(&record).expect("written");
    let change = migrate::change_editions(&old_bytes, &unknown, &functions, &converter).expect("recorded");
    assert_eq!(change.new, vec![("R".to_string(), Derived::NoVerdict)]);
    assert!(matches!(change.old[0].1, Derived::Word(_)), "the old interpretation stays: {:?}", change.old);
    let outcomes = payload_of(&change.migration.record).get("outcomes").cloned();
    let expected = vec![
        parse(r#"{"item":"record","status":"converted","reason":"new function editions; facts unchanged"}"#),
        root_outcome("R", &Derived::NoVerdict),
    ];
    assert_eq!(outcomes, Some(Json::List(expected)));
}

// ------------------------------------------------------------------------------------------------- CT-PS2-04

/// CT-PS2-04 (source, whole: a word conversion whose mapping table changes while it runs, against the same conversion
/// with an unchanged table; cache reuse under changed, extra, missing, reordered and unrecorded inputs and functions;
/// and a display-function edition that computes the same words as the old one): an R-08 input that changed during the
/// build refuses the result even though the output is the same, the inputs are read again only after the build, and a
/// cache is reused only on exact equality of every recorded input and function edition.
#[test]
fn ct_ps2_04_changed_inputs_refuse_reuse_after_the_build() {
    let f = ji23();
    let source = source_word(&f);
    let reads = Cell::new(0u32);
    let reads_at_build = Cell::new(0u32);
    let outputs = RefCell::new(Vec::new());
    let convert = |old: &[u8]| -> Result<Conversion, Refusal> {
        reads_at_build.set(reads.get());
        let target = with(&codec::read_canonical(old)?, name_whole_word);
        outputs.borrow_mut().push(ref_of(&target));
        Ok(Conversion {
            target: Some(target),
            outcomes: vec![Outcome::new("word", Status::Converted, "edition 2 annotations")],
            checks: Vec::new(),
            semantic_changes: Vec::new(),
        })
    };

    // The mapping table is replaced while the conversion runs.
    let mut changing = || {
        reads.set(reads.get() + 1);
        vec![digest("bytes", if reads.get() == 1 { MAPPING } else { MAPPING_3 })]
    };
    assert_eq!(outcome(run(&source, &convert, &mut changing)), DIGEST);
    assert_eq!((reads_at_build.get(), reads.get()), (1, 2), "read once before the build and again after it");
    // The same conversion over an unchanged table is accepted, with the same output.
    reads.set(0);
    let mut stable = || {
        reads.set(reads.get() + 1);
        mapping()
    };
    let accepted = run(&source, &convert, &mut stable).expect("unchanged inputs");
    assert_eq!((reads_at_build.get(), reads.get()), (1, 2));
    let outputs = outputs.borrow();
    assert_eq!(outputs.len(), 2);
    assert_eq!(outputs[0], outputs[1], "both builds made the same target: equal output did not excuse the input");
    assert_eq!(payload_of(&accepted.record).get("target"), Some(&outputs[1]));

    // FP.19 and PS2.7: a cache is reused only on exact equality of every recorded input and function edition.
    let (old, new) = edition_pins();
    let inputs = vec![digest("content", b"fixture fact input"), digest("bytes", MAPPING)];
    let functions = pins_of(&old, &["order", "summary", "display"]);
    let cached = cache(&inputs, functions.clone());
    assert!(migrate::cache_reusable(&cached, &inputs, &functions));
    let changed = vec![inputs[0].clone(), digest("bytes", MAPPING_3)];
    assert!(!migrate::cache_reusable(&cached, &changed, &functions), "a changed input, the same output");
    assert!(!migrate::cache_reusable(&cached, &inputs, &pins_of(&new, &["order", "summary", "display"])));
    let extra = [inputs.clone(), vec![digest("bytes", b"fixture extra table")]].concat();
    assert!(!migrate::cache_reusable(&cached, &extra, &functions), "an extra input");
    assert!(!migrate::cache_reusable(&cached, &inputs[..1], &functions), "a missing input");
    let reordered = vec![inputs[1].clone(), inputs[0].clone()];
    assert!(!migrate::cache_reusable(&cached, &reordered, &functions), "a reordered input");
    let unrecorded = with(&cached, |c| remove(c, "functions"));
    assert!(!migrate::cache_reusable(&unrecorded, &inputs, &functions), "caches fail closed");

    // A new display edition that computes the same words still invalidates a cache of the old one.
    let same = Functions::new().with(old.clone(), Edition::draft_1()).with(new.clone(), Edition::draft_1());
    let display_cache = cache(&inputs[..1], pins_of(&old, &["display"]));
    let payload = with(&parse(&payload_text(&Case::plain(), &old)), |p| {
        set(p, "caches", Json::List(vec![display_cache.clone()]));
    });
    let old_bytes = record::write_record(&evidence_record(payload)).expect("written");
    let converter = pin("edition-converter-1", b"fixture edition converter");
    let change = migrate::change_editions(&old_bytes, &new, &same, &converter).expect("an edition change");
    assert_eq!(change.old, change.new, "the new display function gives the same word");
    assert_eq!(change.stale, vec![display_cache]);
    assert_eq!(payload_of(&change.record).get("caches"), Some(&Json::List(Vec::new())));
}

// ------------------------------------------------------------------------------------------------- CT-PS2-05

/// CT-PS2-05 (source, whole: a converted word made newer, given an unknown required extension, a newer provenance
/// block, a provenance block or contract without its version, or no provenance, each read, produced by a conversion
/// and published beside a valid record; a generation whose metadata, head or validation record is newer; and facts
/// under an edition the build cannot run): a newer schema or edition, unavailable extension rules and missing
/// provenance versions never activate
/// a record or part of one: the reader keeps the bytes recorded or refuses them, no conversion produces such a target,
/// and a publication that holds one writes nothing at all.
#[test]
fn ct_ps2_05_newer_or_unreadable_records_never_activate_in_part() {
    let f = ji23();
    let source = source_word(&f);
    let target = with(&source, name_whole_word);
    let variants: [(&str, Json, &str); 6] = [
        ("a newer version", with(&target, |r| set(r, "version", Json::Int(2))), VERSION),
        (
            "an unknown required extension",
            with(&target, |r| {
                set(r, "extensions", parse(r#"[{"name":"future.rule","required":true,"value":"replay twice"}]"#))
            }),
            VERSION,
        ),
        (
            "a newer provenance block",
            with(&target, |r| set(at(r, &["provenance", "block"]), "version", Json::Int(2))),
            VERSION,
        ),
        (
            "a provenance block without its version",
            with(&target, |r| remove(at(r, &["provenance", "block"]), "version")),
            PROVENANCE,
        ),
        (
            "a provenance contract without its version",
            with(&target, |r| remove(at(r, &["provenance", "block", "contract"]), "version")),
            PROVENANCE,
        ),
        ("a word without provenance", with(&target, |r| set(r, "provenance", Json::Null)), FORMAT),
    ];
    for (name, variant, code) in &variants {
        // Read: the bytes stay recorded, or are refused; never active.
        let bytes = codec::canonical(variant);
        let read = record::read_record(&bytes, "c600-word");
        if *code == VERSION {
            assert_eq!(read, Ok(Read::Recorded { bytes: bytes.clone(), code: VERSION }), "{name}");
        } else {
            assert_eq!(read.map_err(|r| r.code), Err(*code), "{name}");
        }
        // Convert: no migration produces it.
        let produce: &dyn Fn(&mut Json) = &|r| *r = variant.clone();
        assert_eq!(outcome(migrate_word(&source, produce, &["edition 3"], &[])), *code, "{name}");
        // Publish beside a valid record: refused before any write, so neither record is stored.
        let (disk, first) = first_generation(&source);
        let before = disk.writes();
        let package = Package {
            records: vec![target.clone(), variant.clone()],
            originals: Vec::new(),
            head: target.clone(),
            validation: target.clone(),
        };
        assert_eq!(status(&Publisher::new(disk.clone()).publish("generation-2", &package, &replayed)), *code, "{name}");
        assert_eq!(disk.writes(), before, "{name}: nothing was written");
        assert_eq!(object(&disk, &codec::canonical(&target)), None, "{name}: not even the valid record");
        assert_eq!(object(&disk, &bytes), None, "{name}");
        assert_eq!(active(&disk), Some(first), "{name}");
    }

    // A generation whose metadata is newer is refused, never replaced by the older generation, and nothing is
    // published over it.
    let (disk, first) = first_generation(&source);
    let newer = with(&first, |m| set(m, "version", Json::Int(2)));
    let bytes = codec::canonical(&newer);
    let sha = sha256_hex(&bytes);
    disk.overwrite(&format!("objects/{sha}"), &bytes);
    let mut log = disk.durable("active").expect("a pointer");
    log.extend_from_slice(format!("{sha}\n").as_bytes());
    disk.overwrite("active", &log);
    assert_eq!(status(&Publisher::new(disk.clone()).active()), VERSION);
    let before = disk.writes();
    let package = Package {
        records: vec![target.clone()],
        originals: Vec::new(),
        head: target.clone(),
        validation: target.clone(),
    };
    assert_eq!(status(&Publisher::new(disk.clone()).publish("generation-3", &package, &replayed)), VERSION);
    assert_eq!(disk.writes(), before);

    // A generation that names a newer record as its head or as its validation record never activates either, and
    // the older generation is not read in its place.
    let newer_word = with(&target, |r| set(r, "version", Json::Int(2)));
    for (head, validation) in [(&newer_word, &target), (&target, &newer_word)] {
        let (disk, first) = first_generation(&source);
        let mut listed = Vec::new();
        for record in [head, validation] {
            let bytes = codec::canonical(record);
            disk.overwrite(&format!("objects/{}", sha256_hex(&bytes)), &bytes);
            listed.push(digest("bytes", &bytes));
        }
        listed.sort_by_key(|d| text_of(d.get("sha256")));
        let archive = codec::canonical(&Json::object([("objects", Json::List(listed))]));
        disk.overwrite(&format!("objects/{}", sha256_hex(&archive)), &archive);
        let meta = Json::object([
            ("format", Json::text("c600-publication")),
            ("version", Json::Int(1)),
            ("id", Json::text("generation-2")),
            ("previous", content_of(&first)),
            ("next", content_of(head)),
            ("archive", digest("bytes", &archive)),
            ("validation", ref_of(validation)),
        ]);
        let bytes = codec::canonical(&meta);
        let sha = sha256_hex(&bytes);
        disk.overwrite(&format!("objects/{sha}"), &bytes);
        let mut log = disk.durable("active").expect("a pointer");
        log.extend_from_slice(format!("{sha}\n").as_bytes());
        disk.overwrite("active", &log);
        let name = if head == &newer_word { "a newer head" } else { "a newer validation record" };
        assert_eq!(status(&Publisher::new(disk.clone()).active()), VERSION, "{name}");
        let before = disk.writes();
        assert_eq!(
            status(&Publisher::new(disk.clone()).publish("generation-3", &package, &replayed)),
            VERSION,
            "{name}"
        );
        assert_eq!(disk.writes(), before, "{name}");
    }

    // Facts under an edition the build cannot run stay stored without a derived verdict.
    let (old, _) = edition_pins();
    let unknown = with(&old, |e| set(at(e, &["display"]), "version", Json::text("fixture-9")));
    let record = evidence_record(parse(&payload_text(&Case::plain(), &unknown)));
    let bytes = record::write_record(&record).expect("archived");
    assert_eq!(record::read_record(&bytes, "c600-evidence"), Ok(Read::Active(record.clone())));
    let stored = facts::admit(payload_of(&record)).expect("admitted for the archive");
    let functions = Functions::new().with(old, Edition::draft_1());
    assert_eq!(facts::derive(&stored, &unknown, &functions), vec![("R".to_string(), Derived::NoVerdict)]);
}

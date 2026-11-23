//! `import_archive` and `export_check_bundle` on small synthetic archives. The corpus halves of these tests run in
//! `tests/test_lp5_archive.py` over the Git-blob manifest; these show each check and refusal on bytes built here.

use c600_archive::import::{CheckResult, OccurrenceOutcome, Recognition, INVALID_FORMAT};
use c600_archive::{
    export_check_bundle, import_archive, Discrimination, Discriminator, GrantedFile, GrantedFiles, ImportReport,
    ImportTable, ImportTableRef, Limits, LimitsRef, DIGEST_MISMATCH, FORMAT, SCOPE_NOT_IMMUTABLE, SIZE_LIMIT,
};
use c600_digest::sha256_hex;
use c600_kernel::archive::{crc32, Zip};
use c600_kernel::json::{parse, Value};

/// A gzip member with one stored DEFLATE block (RFC 1951 block type 0), enough for these fixtures.
fn gzip(data: &[u8]) -> Vec<u8> {
    let mut out = vec![0x1f, 0x8b, 8, 0, 0, 0, 0, 0, 0, 0xff, 1];
    let len = data.len() as u16;
    out.extend_from_slice(&len.to_le_bytes());
    out.extend_from_slice(&(!len).to_le_bytes());
    out.extend_from_slice(data);
    out.extend_from_slice(&crc32(data).to_le_bytes());
    out.extend_from_slice(&(data.len() as u32).to_le_bytes());
    out
}

struct Fixture {
    members: Vec<(String, Vec<u8>, Option<Vec<u8>>)>,
    auxiliary: Vec<(String, Vec<u8>)>,
}

impl Fixture {
    fn new() -> Fixture {
        Fixture { members: Vec::new(), auxiliary: Vec::new() }
    }

    fn record(mut self, name: &str, json: &str, compressed: bool) -> Fixture {
        let decoded = json.as_bytes().to_vec();
        let stored = if compressed { gzip(&decoded) } else { decoded.clone() };
        self.members.push((name.to_string(), stored, Some(decoded)));
        self
    }

    fn auxiliary(mut self, name: &str, bytes: &[u8]) -> Fixture {
        self.auxiliary.push((name.to_string(), bytes.to_vec()));
        self
    }

    /// The corpus manifest, in the `ps1.38-git-manifest/1` shape, with its own digests right.
    fn manifest(&self) -> Value {
        let s = |x: &str| Value::String(x.to_string());
        let n = |x: usize| Value::Int(x as i64);
        let mut records: Vec<Value> = self
            .members
            .iter()
            .map(|(name, stored, decoded)| {
                let decoded = decoded.as_ref().unwrap();
                let canonical = parse(decoded).unwrap().canonical();
                c600_kernel::json::object([
                    ("path", s(name)),
                    ("git_blob", s(&"a".repeat(40))),
                    ("stored_sha256", s(&sha256_hex(stored))),
                    ("stored_bytes", n(stored.len())),
                    ("decoded_sha256", s(&sha256_hex(decoded))),
                    ("decoded_bytes", n(decoded.len())),
                    ("canonical_sha256", s(&sha256_hex(canonical.as_bytes()))),
                    ("canonical_bytes", n(canonical.len())),
                ])
            })
            .collect();
        records.sort_by_key(|r| r.get("path").unwrap().as_str().unwrap().to_string());
        let auxiliary: Vec<Value> = self
            .auxiliary
            .iter()
            .map(|(name, bytes)| {
                c600_kernel::json::object([
                    ("path", s(name)),
                    ("git_blob", s(&"b".repeat(40))),
                    ("stored_sha256", s(&sha256_hex(bytes))),
                    ("stored_bytes", n(bytes.len())),
                ])
            })
            .collect();
        let mut all: Vec<(String, String)> = records
            .iter()
            .chain(&auxiliary)
            .map(|e| {
                (e.get("path").unwrap().as_str().unwrap().into(), e.get("git_blob").unwrap().as_str().unwrap().into())
            })
            .collect();
        all.sort();
        let full: String = all.iter().map(|(p, b)| format!("{p}\0{b}\n")).collect();
        c600_kernel::json::object([
            ("format", s("ps1.38-git-manifest/1")),
            ("record_manifest_sha256", s(&sha256_hex(Value::Array(records.clone()).canonical().as_bytes()))),
            ("full_corpus_manifest_sha256", s(&sha256_hex(full.as_bytes()))),
            ("full_corpus_entries", n(all.len())),
            ("records", Value::Array(records)),
            ("auxiliary_corpus", Value::Array(auxiliary)),
        ])
    }

    fn granted(&self) -> GrantedFiles {
        GrantedFiles {
            manifest: GrantedFile { name: "MANIFEST.json".into(), bytes: self.manifest().canonical().into_bytes() },
            members: self
                .members
                .iter()
                .map(|(n, b, _)| GrantedFile { name: n.clone(), bytes: b.clone() })
                .chain(self.auxiliary.iter().map(|(n, b)| GrantedFile { name: n.clone(), bytes: b.clone() }))
                .collect(),
        }
    }
}

fn run(input: GrantedFiles) -> ImportReport {
    run_with(input, &Limits::research_archive()).unwrap()
}

fn run_with(input: GrantedFiles, limits: &Limits) -> Result<ImportReport, c600_archive::ArchiveError> {
    let table = ImportTable::edition_1();
    let discriminator = Discriminator::edition_1();
    import_archive(input, ImportTableRef { table: &table, discriminator: &discriminator }, LimitsRef(limits))
}

fn failed(report: &ImportReport) -> Vec<(String, &'static str, String)> {
    let mut out: Vec<(String, &'static str, String)> = report
        .members
        .iter()
        .flat_map(|m| {
            m.checks
                .iter()
                .filter(|c| c.result == CheckResult::Fail)
                .map(move |c| (m.name.clone(), c.check, c.against.split(' ').next().unwrap_or("").to_string()))
        })
        .collect();
    out.sort();
    out
}

const RUN: &str =
    r#"{"kind":"pstar","run":{"format":"pstar-run/1","status":"unknown","reason":"checkpoint","cpu_s":12.5}}"#;
const SCAN: &str = r#"{"format":"t3p-scan/1","rows":[{"status":"budget"},{"status":"a word nobody mapped"}]}"#;
const STAGE: &str = "s19/IMPORT-MANIFEST.json";

/// A stage manifest naming one compressed and one plain member, with original digests unlike the public ones.
fn stage_fixture() -> Fixture {
    let table = br#"{"pose_id":[0,1],"poses":[]}"#.to_vec();
    let (stored, decoded) = (gzip(&table), table.clone());
    let text = b"# a sanitized note\n".to_vec();
    let manifest = format!(
        r#"{{"format":"sanitized-research-archive/1","source_commit":"c","archive_date":"2026-11-03","files":[{{"path":"records/t.json.gz","sha256":"{}","stored_bytes":{},"uncompressed_sha256":"{}","uncompressed_bytes":{},"original_name":"T.json","original_sha256":"{}","transformation":"sanitized"}},{{"path":"records/n.md","sha256":"{}","stored_bytes":{},"uncompressed_sha256":"{}","uncompressed_bytes":{},"original_name":"N.md","original_sha256":"{}","transformation":"sanitized"}}]}}"#,
        sha256_hex(&gzip(&table)),
        gzip(&table).len(),
        sha256_hex(&table),
        table.len(),
        "9".repeat(64),
        sha256_hex(&text),
        text.len(),
        sha256_hex(&text),
        text.len(),
        "8".repeat(64),
    );
    let mut f = Fixture::new().record(STAGE, &manifest, false).auxiliary("s19/records/n.md", &text);
    f.members.push(("s19/records/t.json.gz".into(), stored, Some(decoded)));
    f.auxiliary.push(("s19/README.md".into(), b"readme".to_vec()));
    f
}

#[test]
fn ct_ps3_08_synthetic_tampered_gzip_decoded_and_manifest_digests_fail_separately() {
    // CT-PS3-08 (part: synthetic members; the corpus part runs in tests/test_lp5_archive.py).
    let clean = run(stage_fixture().granted());
    assert!(failed(&clean).is_empty(), "{:?}", failed(&clean));
    assert!(clean.refusal().is_none());
    let sm = &clean.stage_manifests[0];
    assert_eq!(sm.rows.iter().map(|r| r.status).collect::<Vec<_>>(), ["checked", "checked"]);
    assert_eq!(sm.not_listed, ["s19/README.md"]);
    assert_eq!(sm.rows[0].original_sha256.as_deref(), Some("9".repeat(64).as_str()));

    // A gzip stream corrupted after the stage manifest pinned it, in its body or in its signature bytes: the stream
    // check fails against both manifests, and the stage manifest's stored digest fails. The `.gz` name declares the
    // stream, so damaged signature bytes do not let the member pass as plain bytes.
    for signature in [false, true] {
        let mut f = stage_fixture();
        let corrupt = {
            let (_, stored, _) = &mut f.members[1];
            if signature {
                stored[0] ^= 0xff;
                stored[1] ^= 0xff;
            } else {
                let at = stored.len() - 6;
                stored[at] ^= 1;
            }
            stored.clone()
        };
        f.members[1].2 = Some(br#"{"pose_id":[0,1],"poses":[]}"#.to_vec());
        let r = run(f.granted());
        let name = "s19/records/t.json.gz".to_string();
        assert_eq!(
            failed(&r),
            vec![
                (name.clone(), "gzip_stream", "MANIFEST.json".into()),
                (name.clone(), "gzip_stream", STAGE.into()),
                (name.clone(), "stored_sha256", STAGE.into()),
            ],
            "signature {signature}"
        );
        assert_eq!(r.refusal(), Some(DIGEST_MISMATCH));
        let m = &r.members[r.member(&name).unwrap()];
        assert_eq!(m.stored_sha256.as_deref(), Some(sha256_hex(&corrupt).as_str()));
    }

    // Gzip bytes under a name that declares none are not decoded: they fail as plain bytes against the decoded pin.
    let r = run(Fixture::new().record("plain.json", SCAN, true).granted());
    assert!(failed(&r).iter().all(|(_, c, _)| *c != "gzip_stream"));
    assert!(failed(&r).iter().any(|(_, c, _)| *c == "decoded_sha256"));
    assert!(matches!(r.members[0].recognition, Recognition::Unparsed(_)));

    // A changed decoded content behind a valid stream: the decoded checks fail against both manifests, with the
    // canonical digest that derives from the decoded bytes, and no stream check fails. The stage manifest here still
    // pins the old stored bytes; the corpus test moves every stored pin as well.
    let mut f = stage_fixture();
    let other = br#"{"pose_id":[0,2],"poses":[]}"#.to_vec();
    f.members[1].1 = gzip(&other);
    let r = run(f.granted());
    let got: Vec<_> = failed(&r).into_iter().map(|(_, c, a)| (c, a)).collect();
    assert!(got.contains(&("decoded_sha256", "MANIFEST.json".into())));
    assert!(got.contains(&("decoded_sha256", STAGE.into())));
    assert!(got.iter().all(|(c, _)| matches!(*c, "decoded_sha256" | "canonical_sha256" | "stored_sha256")));
    assert!(!got.contains(&("gzip_stream", STAGE.into())));

    // A tampered digest in the corpus manifest: its own record digest fails and so does that member.
    let f = Fixture::new().record("a.json", SCAN, false);
    let mut m = f.manifest();
    if let Value::Object(top) = &mut m {
        if let Some(Value::Array(rs)) = top.get_mut("records") {
            if let Value::Object(e) = &mut rs[0] {
                e.insert("stored_sha256".into(), Value::String("f".repeat(64)));
            }
        }
    }
    let mut g = f.granted();
    g.manifest.bytes = m.canonical().into_bytes();
    let r = run(g);
    assert_eq!(failed(&r), vec![("a.json".into(), "stored_sha256", "MANIFEST.json".into())]);
    assert!(r.manifest_checks.iter().any(|c| c.check == "record_manifest_sha256" && c.result == CheckResult::Fail));

    // The original digest is metadata: no check ever expects it.
    let all: Vec<_> = clean.members.iter().flat_map(|m| m.checks.iter()).collect();
    assert!(all.iter().all(|c| c.expected != "9".repeat(64) && c.expected != "8".repeat(64)));
}

#[test]
fn ct_ps3_09_synthetic_case_only_names_and_a_missing_member() {
    // CT-PS3-09 (part: synthetic names; the six stage 16 pairs run in tests/test_lp5_archive.py).
    let f = Fixture::new()
        .record("runs/PX-1.json", r#"{"format":"x/1","status":"A"}"#, false)
        .record("runs/px-1.json", r#"{"format":"x/1","status":"B"}"#, false)
        .record("runs/other.json", r#"{"format":"x/1"}"#, false);
    let r = run(f.granted());
    assert_eq!(r.case_only_names.len(), 1);
    assert!(r.case_only_names[0].distinct_digests());
    assert_eq!(r.totals()["present"], 3);

    // Missing: listed, not granted; reported, counted, and never a digest failure.
    let mut g = f.granted();
    g.members.retain(|m| m.name != "runs/px-1.json");
    let r = run(g);
    let m = &r.members[r.member("runs/px-1.json").unwrap()];
    assert!(!m.present);
    assert_eq!((m.integrity(), &m.recognition), ("not_available", &Recognition::NotAvailable));
    assert_eq!((r.totals()["records"], r.totals()["missing"]), (3, 1));
    assert!(r.refusal().is_none());
    // A missing member still takes part in the case-only comparison by its listed digest.
    assert_eq!(r.case_only_names.len(), 1);

    // An unlisted granted file is reported and not read; two granted files with one exact name refuse the grant.
    let mut g = f.granted();
    g.members.push(GrantedFile { name: "runs/extra.json".into(), bytes: b"{}".to_vec() });
    assert_eq!(run(g).unlisted.len(), 1);
    let mut g = f.granted();
    let first = g.members[0].clone();
    g.members.push(first);
    assert_eq!(run_with(g, &Limits::research_archive()).unwrap_err().code, FORMAT);
}

#[test]
fn ct_ps3_10_synthetic_bundle_declares_missing_dependencies_and_refuses_bad_input() {
    // CT-PS3-10 (part: synthetic envelopes; the corpus bundle runs in tests/test_lp5_archive.py).
    let script = b"print('a producer, carried as data and never run')\n";
    let f = Fixture::new().record("a.json.gz", SCAN, true).record("b.json", RUN, false).auxiliary("make.py", script);
    let r = run(f.granted());
    let limits = Limits::research_archive();
    let a = r
        .envelope("a.json.gz")
        .unwrap()
        .with_dependency("make.py", Some(&sha256_hex(script)), Some(script.to_vec()), "the producer, as text")
        .with_dependency("cache.npz", None, None, "absent from the archive")
        .with_dependency("table.json", Some(&"7".repeat(64)), None, "cited with a digest, absent");
    let b = r.envelope("b.json").unwrap();
    assert_eq!(b.format, None);
    let bundle = export_check_bundle(&[a.clone(), b.clone()], LimitsRef(&limits)).unwrap();
    assert_eq!(bundle.sha256, sha256_hex(&bundle.zip));
    let zip = Zip::parse(&bundle.zip).unwrap();
    let names = zip.names();
    assert_eq!(names[0], "manifest.json");
    let objects: Vec<&str> = names[1..].to_vec();
    let mut sorted = objects.clone();
    sorted.sort();
    assert_eq!(objects, sorted);
    for n in &objects {
        let bytes = zip.read(n, 1 << 20).unwrap();
        assert_eq!(*n, format!("objects/{}", sha256_hex(&bytes)));
    }
    let manifest = parse(&zip.read("manifest.json", 1 << 20).unwrap()).unwrap();
    assert_eq!(manifest, bundle.manifest);
    let payload = manifest.get("payload").unwrap();
    assert_eq!(payload.get("checkers"), Some(&Value::Array(Vec::new())));
    assert_eq!(payload.get("missing").unwrap().as_array().unwrap().len(), 1);
    // FP.10's extension list: unique names in scalar order, every entry optional.
    let ext = manifest.get("extensions").unwrap().as_array().unwrap();
    let names: Vec<&str> = ext.iter().map(|x| x.get("name").unwrap().as_str().unwrap()).collect();
    let mut sorted_names = names.clone();
    sorted_names.sort();
    sorted_names.dedup();
    assert_eq!(names, sorted_names);
    assert!(ext.iter().all(|x| x.get("required") == Some(&Value::Bool(false)) && x.as_object().unwrap().len() == 3));
    let value = |name: &str| ext.iter().find(|x| x.get("name").unwrap().as_str() == Some(name)).unwrap().get("value");
    assert_eq!(value("missing_without_digest").unwrap().as_array().unwrap().len(), 1);
    assert!(!value("checker_status").unwrap().as_str().unwrap().contains("exists"));
    assert_eq!(manifest.get("provenance"), Some(&Value::Null));
    assert_eq!(objects.len(), 3);

    // Refusals: bytes that disagree with their digest, a revision-bound scope, and the caller's limits.
    let mut bad = a.clone();
    bad.bytes.push(b' ');
    assert_eq!(export_check_bundle(&[bad], LimitsRef(&limits)).unwrap_err().code, DIGEST_MISMATCH);
    // The digest compared is the corpus manifest's, not one recomputed from the bytes in hand.
    let mut forged = a.clone();
    forged.bytes = b"{}".to_vec();
    forged.stored_sha256 = sha256_hex(&forged.bytes);
    assert_eq!(export_check_bundle(&[forged], LimitsRef(&limits)).unwrap_err().code, DIGEST_MISMATCH);
    // A record whose stored bytes match their pin but whose decoded content fails its pins is refused, and so is a
    // dependency carried from such a member.
    let mut f = Fixture::new().record("c.json.gz", SCAN, true);
    f.members[0].2 = Some(br#"{"format":"t3p-scan/1","rows":[]}"#.to_vec());
    let damaged = run(f.granted());
    assert_eq!(damaged.refusal(), Some(DIGEST_MISMATCH));
    let c = damaged.envelope("c.json.gz").unwrap();
    assert_eq!(c.listed_sha256, c.stored_sha256);
    assert!(c.failed_checks.iter().any(|x| x.starts_with("decoded_sha256")));
    assert!(c.failed_checks.iter().any(|x| x.starts_with("canonical_sha256")));
    assert_eq!(export_check_bundle(&[c.clone()], LimitsRef(&limits)).unwrap_err().code, DIGEST_MISMATCH);
    let dep = damaged.dependency("c.json.gz", "c.json.gz", Some(&c.stored_sha256), "a damaged member").unwrap();
    assert!(!dep.failed_checks.is_empty());
    let with_damaged = b.clone().with(dep);
    assert_eq!(export_check_bundle(&[with_damaged], LimitsRef(&limits)).unwrap_err().code, DIGEST_MISMATCH);
    // A clean member dependency travels.
    let clean = r.dependency("make.py", "make.py", Some(&sha256_hex(script)), "the producer, as text").unwrap();
    assert!(export_check_bundle(&[b.clone().with(clean)], LimitsRef(&limits)).is_ok());
    let wrong = b.clone().with_dependency("make.py", Some(&"0".repeat(64)), Some(script.to_vec()), "wrong digest");
    assert_eq!(export_check_bundle(&[wrong], LimitsRef(&limits)).unwrap_err().code, DIGEST_MISMATCH);
    let mut bound = b.clone();
    bound.revision_bound = true;
    assert_eq!(export_check_bundle(&[bound], LimitsRef(&limits)).unwrap_err().code, SCOPE_NOT_IMMUTABLE);
    let small = Limits { max_members: 2, ..Limits::research_archive() };
    assert_eq!(export_check_bundle(&[a, b], LimitsRef(&small)).unwrap_err().code, SIZE_LIMIT);
}

#[test]
fn ct_det_24_synthetic_timed_stop_keeps_its_limits() {
    // CT-DET-24 (part: a synthetic archived timed stop; the corpus records run in tests/test_lp5_archive.py).
    let r = run(Fixture::new().record("s7305-pstar.json", RUN, false).granted());
    let m = &r.members[0];
    assert!(matches!(m.recognition, Recognition::Absent(Discrimination::Undiscriminated)));
    assert!(m.recheck.starts_with("not run"));
    let table = ImportTable::edition_1();
    let mut outcomes = Vec::new();
    for g in &r.occurrences {
        if let OccurrenceOutcome::Facts { rule } = g.outcome {
            let c600_archive::Outcome::Facts(f) = &table.rules()[rule].outcome else { unreachable!() };
            outcomes.push((g.field.clone(), f.job_outcome.clone()));
        }
    }
    outcomes.sort();
    assert_eq!(
        outcomes,
        vec![("run.reason".into(), Some("cut off".to_string())), ("run.status".into(), Some("cut off".to_string()))]
    );
    // The leaves outside the inner format stay recorded with the record's absence.
    assert!(r
        .occurrences
        .iter()
        .any(|g| g.format.is_none() && matches!(g.outcome, OccurrenceOutcome::Recorded { .. })));
    assert_eq!(r.totals()["producers_run"], 0);
}

#[test]
fn unmapped_values_stay_word_for_word_and_limits_bound_the_import() {
    let r = run(Fixture::new().record("scan.json", SCAN, false).granted());
    let unmapped: Vec<_> =
        r.occurrences.iter().filter(|g| matches!(g.outcome, OccurrenceOutcome::Unmapped { .. })).collect();
    assert_eq!(unmapped.len(), 1);
    assert_eq!(unmapped[0].value, Value::String("a word nobody mapped".into()));
    assert_eq!(unmapped[0].pointers, ["/rows/1/status"]);
    let mapped: Vec<_> =
        r.occurrences.iter().filter(|g| matches!(g.outcome, OccurrenceOutcome::Facts { .. })).collect();
    assert_eq!(mapped.len(), 1);

    let f = Fixture::new().record("scan.json.gz", SCAN, true);
    for limits in [
        Limits { max_members: 0, ..Limits::research_archive() },
        Limits { max_member_stored_bytes: 10, ..Limits::research_archive() },
        Limits { max_total_stored_bytes: 10, ..Limits::research_archive() },
        Limits { max_member_decoded_bytes: 10, ..Limits::research_archive() },
        Limits { max_total_decoded_bytes: 10, ..Limits::research_archive() },
        Limits { max_occurrences: 1, ..Limits::research_archive() },
        Limits { max_manifest_bytes: 10, ..Limits::research_archive() },
    ] {
        assert_eq!(run_with(f.granted(), &limits).unwrap_err().code, SIZE_LIMIT, "{limits:?}");
    }
    let mut g = f.granted();
    g.manifest.bytes = b"{\"format\":\"something else\"}".to_vec();
    assert_eq!(run_with(g, &Limits::research_archive()).unwrap_err().code, FORMAT);
}

#[test]
fn a_non_string_or_empty_format_is_malformed_not_absent() {
    // PS3.11's absence is a missing `format` key; a present key with another value is an invalid format, kept word for
    // word, and no discriminator is consulted for it.
    let f = Fixture::new()
        .record("n.json", r#"{"format":7,"status":"budget"}"#, false)
        .record("e.json", r#"{"format":"","status":"budget"}"#, false)
        .record("x.json", r#"{"status":"budget"}"#, false);
    let r = run(f.granted());
    let recognition = |n: &str| r.members[r.member(n).unwrap()].recognition.clone();
    assert_eq!(recognition("n.json"), Recognition::InvalidFormat(Value::Int(7)));
    assert_eq!(recognition("e.json"), Recognition::InvalidFormat(Value::String(String::new())));
    assert!(matches!(recognition("x.json"), Recognition::Absent(_)));
    assert_eq!((r.totals()["invalid_format"], r.totals()["absent_format"]), (2, 1));
    let absent = r.member("x.json").unwrap();
    let malformed: Vec<_> = r.occurrences.iter().filter(|g| g.member != absent).collect();
    assert!(!malformed.is_empty());
    for g in malformed {
        assert_eq!(g.outcome, OccurrenceOutcome::Recorded { rule: None, reason: INVALID_FORMAT.to_string() });
    }
    assert_eq!(r.totals()["occurrences_absent_format"], 1);
    assert!(r.envelope("n.json").is_none());
    let json = r.to_json();
    let kinds: Vec<String> = json
        .get("members")
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .map(|m| m.get("recognition").unwrap().get("kind").unwrap().as_str().unwrap().to_string())
        .collect();
    assert_eq!(kinds, ["invalid_format", "invalid_format", "absent"]);
}

#[test]
fn a_run_record_letter_count_that_overflows_refuses_the_import() {
    let word = b"\x01\x00".to_vec();
    let fixture = |letters: &str| {
        let record = format!(
            r#"{{"format":"runner-run/3","steps":[{{"word":"w.ia16.gz","encoding":"ia16","word_sha256":"{}","letters":{letters}}}]}}"#,
            sha256_hex(&word)
        );
        Fixture::new().record("run.json", &record, false).auxiliary("w.ia16.gz", &gzip(&word))
    };
    let r = run(fixture("1").granted());
    assert!(failed(&r).is_empty(), "{:?}", failed(&r));
    assert!(matches!(r.members[r.member("w.ia16.gz").unwrap()].recognition, Recognition::Referenced { .. }));
    let refused = run_with(fixture(&i64::MAX.to_string()).granted(), &Limits::research_archive()).unwrap_err();
    assert_eq!(refused.code, FORMAT);
}

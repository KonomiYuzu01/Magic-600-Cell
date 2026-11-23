//! Stage 18's records read in place: registration, gate 1 on both `steps.json` forms, the lattice run record, the
//! groups left `not_applicable`, and the long words through length and admission checks.

use c600_checkers::archive::{self, s18, SHARED_SOURCES};
use c600_checkers::gate;
use c600_checkers::{codes, Verdict};
use c600_kernel::json::Value;

use super::fixtures::*;
use crate::common::*;

const STAGE12: &str = "research/jumbling/restoration/12-low-height/runs/round22/line-a/literal/run3";

#[test]
fn ct_lb5_05_registration_five_stage18_subcheckers_each_with_its_own_identity() {
    let subs: Vec<_> = archive::registry().filter(|sub| sub.stage == 18).collect();
    let ids: Vec<&str> = subs.iter().map(|sub| sub.id()).collect();
    assert_eq!(ids, ["s18.expansion", "s18.composed", "s18.tail_replay", "s18.lattice_run", "s18.lattice_replay"]);
    assert!(archive::registry_problems(archive::registry()).is_empty());
    let mut digests: Vec<String> = subs.iter().map(|sub| sub.checker.sources.identity().source).collect();
    digests.sort();
    digests.dedup();
    assert_eq!(digests.len(), 5, "each sub-checker has its own source digest");
    for sub in &subs {
        let named: Vec<&str> = sub.checker.sources.files.iter().map(|(name, _)| *name).collect();
        let own = format!("c600-checkers/src/archive/s18/{}.rs", sub.id().trim_start_matches("s18."));
        assert!(named.contains(&own.as_str()), "{} does not cover {own}", sub.id());
        for shared in SHARED_SOURCES {
            assert!(named.contains(&shared), "{} does not cover {shared}", sub.id());
        }
        // Every s18 file a sub-checker's code reaches is in its identity.
        for module in ["words", "record"] {
            let file = format!("c600-checkers/src/archive/s18/{module}.rs");
            assert!(named.contains(&file.as_str()), "{} does not cover {file}", sub.id());
        }
    }
}

#[test]
fn ct_lb5_05_gate1_both_steps_json_forms_pass_in_place() {
    for (dir, sha256, end, form) in
        [(T6, T6_STEPS_SHA256, true, "four-segment"), (RULE, RULE_STEPS_SHA256, false, "three-segment")]
    {
        let record = pinned(&format!("{dir}/steps.json"), sha256);
        let report = check_owned(expansion(), &record, &chain_inputs(dir, end));
        assert_eq!(report.verdict, Verdict::Pass, "{dir}: {:#?}", report.trace.last());
        assert!(report.complete());
        assert_eq!(report.input, sha256);
        assert_eq!(report.trace.iter().find(|t| t.claim == "form").map(|t| t.computed.as_str()), Some(form));
        // The start table is bound by its bytes and its recomputed digest.
        let start = report.inputs.iter().find(|i| i.role == "start").unwrap();
        assert_eq!(start.sha256, STOP_TABLE_SHA256);
        assert_eq!(start.identity, [("digest".to_owned(), STOP_DIGEST.to_owned())]);
    }
}

#[test]
fn ct_lb5_05_gate1_mutation_gate_on_both_records_in_place() {
    let wrong = pinned(BEFORE_STOP_TABLE, BEFORE_STOP_TABLE_SHA256);
    // T6 with its stored end table: every leaf of the record is a gate-1 claim.
    let f = gate_fixture(
        "T6 steps.json, gate 1",
        expansion(),
        pinned(&format!("{T6}/steps.json"), T6_STEPS_SHA256),
        chain_inputs(T6, true),
        &[],
        &[],
        ("start", wrong.clone()),
    );
    let report = gate::run(&engines(), &f).expect("a checker");
    assert!(report.passed(), "{:#?}", report.problems);
    assert_eq!(report.tampered(), 1 + 4 * 14 + 2);
    let record = pinned(&format!("{T6}/steps.json"), T6_STEPS_SHA256);
    structural_gate("T6 steps.json, gate 1", expansion(), &record, &chain_inputs(T6, true), &[], &[]);
    // The rule's record: its end table is not stored, so its last end digest is a gate-2 claim only.
    let f = gate_fixture(
        "rule-stop steps.json, gate 1",
        expansion(),
        pinned(&format!("{RULE}/steps.json"), RULE_STEPS_SHA256),
        chain_inputs(RULE, false),
        &[],
        &["steps[2].end_digest"],
        ("start", wrong),
    );
    let report = gate::run(&engines(), &f).expect("a checker");
    assert!(report.passed(), "{:#?}", report.problems);
    let record = pinned(&format!("{RULE}/steps.json"), RULE_STEPS_SHA256);
    let inputs = chain_inputs(RULE, false);
    structural_gate("rule-stop steps.json, gate 1", expansion(), &record, &inputs, &[], &["steps[2].end_digest"]);
}

#[test]
fn ct_lb5_05_lattice_run_record_and_members_pass_in_place_with_a_mutation_gate() {
    let record = pinned(&format!("{RUN}/run.json"), RUN_SHA256);
    let inputs = run_inputs();
    let report = check_owned(lattice_run(), &record, &inputs);
    assert_eq!(report.verdict, Verdict::Pass, "{:#?}", report.trace.last());
    let scope = report.trace.iter().find(|t| t.claim == "scope").unwrap();
    assert!(scope.computed.contains("no state after a letter"), "{}", scope.computed);
    // The tampered leaves: the record's phases, totals and acceptance, and the first, a middle and the last step.
    // Not claims: the producer's configuration, the timings and the labels flag.
    let skip: Vec<String> = (0..53).map(|k| format!("steps[{k}].seconds")).collect();
    let mut skip: Vec<&str> = skip.iter().map(String::as_str).collect();
    skip.extend(["config", "totals.seconds", "steps[52].labels_identity"]);
    let claims = [
        "format",
        "menu",
        "start",
        "phases",
        "totals",
        "acceptance",
        "result",
        "error",
        "stuck",
        "lattice",
        "steps[0]",
        "steps[35]",
        "steps[52]",
    ];
    let f = gate_fixture(
        "lattice-d1 run.json",
        lattice_run(),
        record.clone(),
        inputs.clone(),
        &claims,
        &skip,
        ("start-table.json", read(&format!("{RUN}/tables/00052.json"))),
    );
    let report = gate::run(&engines(), &f).expect("a checker");
    assert!(report.passed(), "{:#?}", report.problems);
    // The structural mutations of the same paths, and of the whole record's top level.
    structural_gate("lattice-d1 run.json", lattice_run(), &record, &inputs, &claims, &skip);
    let mut doc = parse(&record);
    if let Value::Object(map) = &mut doc {
        map.insert("unknown~".to_owned(), int(0));
    }
    assert_eq!(check_owned(lattice_run(), doc.canonical().as_bytes(), &inputs).verdict.claim(), Some("certificate"));
    // A tampered member fails too: the report, a tree and a word.
    for role in ["lattice/report.json", "step-trees/00003.json.gz", "words/00002.ia16.gz"] {
        let bytes = inputs.iter().find(|(r, _)| r == role).unwrap().1.clone();
        let mut tampered = bytes.clone();
        let middle = tampered.len() / 2;
        tampered[middle] ^= 1;
        let report = check_owned(lattice_run(), &read(&format!("{RUN}/run.json")), &replace(&inputs, role, tampered));
        assert!(report.verdict.is_fail(), "{role}: {:?}", report.verdict);
    }
}

#[test]
fn ct_lb5_05_static_tail_and_wholecap_records_answer_not_applicable_with_their_scope() {
    let kinds = [expansion(), composed(), tail_replay(), lattice_run(), lattice_replay()];
    let mut records: Vec<String> =
        ["112734", "16314", "60085", "81223", "9013"].iter().map(|u| format!("{S18}/stop-units/{u}.json")).collect();
    records.extend(
        ["d1-stop-158", "d1-t6-104", "d1-t6-129", "e2-71", "e2-737"].iter().map(|w| format!("{S18}/wholecap/{w}.txt")),
    );
    for path in &records {
        for kind in kinds {
            let report = check_owned(kind, &read(path), &[]);
            match &report.verdict {
                Verdict::NotApplicable { reason } => {
                    assert!(
                        reason.contains("18/static_tail") && reason.contains("no independent checker"),
                        "{path}: {reason}"
                    );
                }
                other => panic!("{path} under {kind:?}: {other:?}"),
            }
        }
    }
    // Only those: a record of one sub-checker departs from the others' form and fails at its format, and a stop unit
    // with a field more or less is no stop unit.
    let run = read(&format!("{RUN}/run.json"));
    for kind in [expansion(), composed(), tail_replay()] {
        assert_eq!(check_owned(kind, &run, &[]).verdict.claim(), Some("format"));
    }
    let steps = read(&format!("{T6}/steps.json"));
    for kind in [lattice_run(), lattice_replay()] {
        assert_eq!(check_owned(kind, &steps, &[]).verdict.claim(), Some("format"));
    }
    let unit = parse(&read(&format!("{S18}/stop-units/9013.json")));
    let mut changed = 0;
    for mutant in structural_mutants(&unit, &[], &[]).into_iter().map(|(_, doc)| doc) {
        let fields: Vec<&str> = mutant.as_object().unwrap().keys().map(String::as_str).collect();
        if fields != s18::record::STOP_UNIT_KEYS {
            changed += 1;
            for kind in kinds {
                assert!(check_owned(kind, mutant.canonical().as_bytes(), &[]).verdict.is_fail(), "{kind:?}");
            }
        }
    }
    assert_eq!(changed, 2 * 9 + 1);
}

/// A trace step's span length.
fn span_len(report: &c600_checkers::CheckReport, claim: &str) -> u64 {
    let span = report.trace.iter().find(|t| t.claim == claim).and_then(|t| t.span.clone()).expect(claim);
    span.to - span.from
}

#[test]
fn ct_lb5_09_recorded_long_words_stream_through_length_and_admission() {
    // 871,582: T6's step 4, X 21,592 + W + V′ 424,994 + W⁻¹ + V′⁻¹ 424,994, admitted before any word is read.
    let report = check_owned(expansion(), &read(&format!("{T6}/steps.json")), &chain_inputs(T6, true));
    assert_eq!(report.verdict, Verdict::Pass);
    let admission = report.trace.iter().find(|t| t.claim == "steps[3].admission").unwrap();
    assert!(admission.computed.starts_with("871582 letters"), "{}", admission.computed);
    let position = |claim: &str| report.trace.iter().position(|t| t.claim == claim).unwrap();
    assert!(position("steps[3].admission") < position("steps[0].v.sha256"), "admission comes before any word");
    assert_eq!((span_len(&report, "steps[3].x.sha256"), span_len(&report, "steps[3].v.sha256")), (21_592, 424_994));

    // 2,352,984: the lattice phase's step 14, stored flat.
    let report = check_owned(lattice_run(), &read(&format!("{RUN}/run.json")), &run_inputs());
    assert_eq!(report.verdict, Verdict::Pass);
    assert_eq!(span_len(&report, "steps[14].word"), 2_352_984);
    let total: u64 = (0..53).map(|k| span_len(&report, &format!("steps[{k}].word"))).sum();
    assert_eq!(total, 13_231_266);

    // 672,417: stage 12's E2 step (outside the Git-blob manifest, pinned here): X 3 + W + V′ 336,206 + W⁻¹ + V′⁻¹,
    // the length and hash t1.out records, each part streamed.
    let t1 = parse(&pinned(
        &format!("{STAGE12}/t1.out"),
        "7c52425324695d8042948aeb116d38a1019805ab569be70a4d517c6cca4e847a",
    ));
    let number = |key: &str| t1.get(key).and_then(Value::as_i64).unwrap() as u64;
    let (x, v, letters) = (number("X"), number("V_letters"), number("letters"));
    assert_eq!((x, v, letters), (3, 336_206, 672_417));
    s18::words::admit(x + 2 + 2 * v, "letters").unwrap();
    assert_eq!(x + 2 + 2 * v, letters);
    let parts = [
        (
            "prep2/x-word.json",
            "e4cdf39f19f1bade77904fcfcac05101b968088f80450e86fd8607dbbaf26ea6",
            x,
            &["input_digest", "output_digest"][..],
        ),
        (
            "t1/b06-14-W.json",
            "d5dd3155912c7218445e44e16c819f01594caefd5a8a040c0a3060293386b290",
            1,
            &["input_digest", "output_digest"][..],
        ),
        (
            "t1/b06-14-V-word.json.gz",
            "9b4d864ceb88d8b62ec20c8fde07b20c6ac49b00f71be384310599b692fd418c",
            v,
            &["input_digest"][..],
        ),
    ];
    let mut hashes = Vec::new();
    for (file, pin, declared, digests) in parts {
        let bytes = pinned(&format!("{STAGE12}/{file}"), pin);
        let limit = s18::words::text_limit(declared, file).unwrap();
        let text = s18::words::decode(file, &bytes, limit, file).unwrap();
        let read = s18::words::stream_word_file(&text, file, declared, digests, |_, _, _| Ok(())).unwrap();
        assert_eq!(read.letters, declared, "{file}");
        hashes.push(read.sha256);
    }
    assert_eq!(Some(hashes[2].as_str()), t1.get("V_sha256").and_then(Value::as_str));
    let winv = pinned(
        &format!("{STAGE12}/t1/b06-14-winv.json"),
        "132ef1019379e624eb7680a2a84f4d923fe72c3bc81679520bad3aeb1e7c3110",
    );
    let (p, q, _) = s18::words::read_core(&winv, "winv").unwrap();
    assert_eq!(p.len() * 2 + q.len(), 1);

    // An oversized declared length is refused before anything is decoded: the stored word is replaced by bytes that are
    // no word at all, and the failure is still the admission.
    let mut doc = parse(&read(&format!("{T6}/steps.json")));
    if let Value::Object(map) = &mut doc {
        if let Some(Value::Array(steps)) = map.get_mut("steps") {
            if let Value::Object(step) = &mut steps[0] {
                step.insert("V".into(), int(5_000_000));
                step.insert("letters".into(), int(10_000_002));
            }
        }
    }
    let record = doc.canonical().into_bytes();
    let inputs = replace(&chain_inputs(T6, true), "01-353-22/v.json.gz", b"no word".to_vec());
    let report = check_owned(expansion(), &record, &inputs);
    assert_eq!(report.verdict.claim(), Some("steps[0].letters"));
    assert!(matches!(&report.verdict, Verdict::Fail { reason, .. } if reason.contains("admission bound")));
    let mut run = parse(&read(&format!("{RUN}/run.json")));
    if let Value::Object(map) = &mut run {
        if let Some(Value::Array(steps)) = map.get_mut("steps") {
            if let Value::Object(step) = &mut steps[14] {
                step.insert("letters".into(), int(10_000_001));
            }
        }
    }
    let inputs = replace(&run_inputs(), "words/00014.ia16.gz", b"no word".to_vec());
    for kind in [lattice_run(), lattice_replay()] {
        let report = check_owned(kind, &run.canonical().into_bytes(), &inputs);
        assert_eq!(report.verdict.claim(), Some("steps[14].letters"), "{kind:?}");
        assert_ne!(report.verdict.code(), Some(codes::START_TABLE_MISMATCH));
    }
}

#[test]
fn ct_lb1_04_expansion_half_stage18_streamed_lengths_equal_declared_lengths() {
    // Stage 18's 35 expansion groups: both records and their 33 step files. Every word file's streamed length is its
    // declared length (X, 1, V or the core's one letter), every unit has its declared size, and each step's exact
    // length X + 2 + copies·V is computed and admitted before any of its words is read.
    let mut files = 0;
    for (dir, end, copies) in [(T6, true, 2), (RULE, false, 1)] {
        let record = parse(&read(&format!("{dir}/steps.json")));
        let report = check_owned(expansion(), &read(&format!("{dir}/steps.json")), &chain_inputs(dir, end));
        assert_eq!(report.verdict, Verdict::Pass);
        files += 1;
        for (k, step) in record.get("steps").and_then(Value::as_array).unwrap().iter().enumerate() {
            let n = |key: &str| step.get(key).and_then(Value::as_i64).unwrap() as u64;
            let held =
                |claim: &str| report.trace.iter().find(|t| t.claim == claim && t.held).map(|t| t.computed.clone());
            assert_eq!(held(&format!("steps[{k}].letters")), Some((n("X") + 2 + copies * n("V")).to_string()));
            if n("X") > 0 {
                assert_eq!(held(&format!("steps[{k}].x.letters")), Some(n("X").to_string()));
                files += 1;
            }
            assert_eq!(held(&format!("steps[{k}].w.letters")), Some("1".into()));
            assert_eq!(held(&format!("steps[{k}].v.letters")), Some(n("V").to_string()));
            assert!(held(&format!("steps[{k}].winv.letter")).is_some());
            assert_eq!(held(&format!("steps[{k}].unit_size")), Some(n("unit_size").to_string()));
            files += 4;
        }
    }
    assert_eq!(files, 35);
}

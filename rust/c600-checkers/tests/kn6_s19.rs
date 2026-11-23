//! Stage 19's research-archive sub-checkers (LP5 piece R6) on the archive, read in place and read-only.
//!
//! Test names carry the CT ID and part (PHS.62): CT-LB5-06's claim part (stored and decoded digests, full tables, the
//! checkable certificates, R3/S block decrease and R_L's length premises, frontier exits failing the sticker goal,
//! indexed references not read as distinct fibres, Γ(Y) carrying legality without sticker goals, unknown synthetic
//! reachability; the manifest part is R0's and the preflight part R1's) and CT-LB1-05's matching half (the matching
//! witness satisfies every edge constraint and brute force agrees on the infeasible bijections). Every sub-checker
//! passes the mutation gate on an archived record. The slowest gates are `#[ignore]`d with their reason.

#[path = "kn6/common.rs"]
mod common;

use c600_checkers::archive::{self, s19};
use c600_checkers::gate::{self, Fixture};
use c600_checkers::{check, CertificateRef, CheckReport, CheckerKind, Input, Verdict};
use c600_kernel::json::Value;

use common::{engines, menu_record, parse, read, sha};

const STAGE: &str = "research/jumbling/restoration/19-height-depth-framework";

const Y: &str = "records/irregularity-descent-source-end-table.json.gz";
const Z: &str = "records/profile-family-validation-input-table.json.gz";
const Z_END: &str = "records/profile-family-validation-end-table.json.gz";
const POST_A: &str = "records/type-lex-independent-282-13-end-table.json.gz";
const COMPACT: &str = "records/type-lex-trimmed-macro-end-table.json.gz";
const CLEANUP: &str = "records/type-lex-recovery-transfer-joined-table.json.gz";
const MATCHING: &str = "records/profile-family-validation-matching-certificate.json";
const RESULTS: &str = "records/profile-family-validation-results.json.gz";
const MACRO: &str = "records/type-lex-trimmed-macro.json";
const TRIMMED: &str = "records/type-lex-trimmed-independent-results.json";
const WEIGHTS: &str = "records/type-height-weight-results.json";
const COSETS: &str = "records/type-lex-282-results.json";

/// A stage 19 member's stored bytes.
fn stored(member: &str) -> Vec<u8> {
    read(&format!("{STAGE}/{member}"))
}

/// A stage 19 member's decoded bytes.
fn decoded(member: &str) -> Vec<u8> {
    let bytes = stored(member);
    if member.ends_with(".gz") {
        c600_kernel::archive::gunzip(&bytes, 1 << 26).expect("a gzip member")
    } else {
        bytes
    }
}

/// The member's entry in `IMPORT-MANIFEST.json`.
fn manifest(member: &str) -> Value {
    let doc = parse(&read(&format!("{STAGE}/IMPORT-MANIFEST.json")));
    let files = doc.get("files").and_then(Value::as_array).expect("files");
    files.iter().find(|f| f.get("path").and_then(Value::as_str) == Some(member)).expect("a manifest entry").clone()
}

fn text(v: &Value, key: &str) -> String {
    v.get(key).and_then(Value::as_str).expect(key).to_owned()
}

/// Checks an archived record under its manifest's decoded digest.
fn check_member(id: &'static str, member: &str, inputs: &[(&str, Vec<u8>)]) -> CheckReport {
    let bytes = decoded(member);
    let stated = text(&manifest(member), "uncompressed_sha256");
    assert_eq!(sha(&bytes), stated, "{member}: the decoded bytes are the manifest's");
    check_bytes(id, &bytes, &stated, inputs)
}

fn check_bytes(id: &'static str, bytes: &[u8], digest: &str, inputs: &[(&str, Vec<u8>)]) -> CheckReport {
    let inputs: Vec<Input> = inputs.iter().map(|(role, bytes)| Input { role, bytes }).collect();
    check(&engines(), &CertificateRef { digest, bytes, inputs: &inputs }, CheckerKind::Archive(id)).expect("a checker")
}

fn assert_pass(report: &CheckReport) {
    assert!(report.complete(), "{:?}\n{:#?}", report.verdict, report.trace.iter().rev().take(3).collect::<Vec<_>>());
}

/// The computed value of a trace step.
fn step<'r>(report: &'r CheckReport, claim: &str) -> &'r str {
    &report.trace.iter().find(|s| s.claim == claim).unwrap_or_else(|| panic!("no trace step {claim}")).computed
}

/// The `kn6-s19-pose-table/1` certificate of a stored table: its manifest entry and its own digest.
fn pose_certificate(member: &str) -> Vec<u8> {
    let entry = manifest(member);
    let table = parse(&decoded(member));
    let field = |key: &str| entry.get(key).expect(key).clone();
    let doc = Value::Object(
        [
            ("format", Value::String("kn6-s19-pose-table/1".into())),
            ("path", Value::String(member.into())),
            ("sha256", field("sha256")),
            ("stored_bytes", field("stored_bytes")),
            ("uncompressed_sha256", field("uncompressed_sha256")),
            ("uncompressed_bytes", field("uncompressed_bytes")),
            ("digest", table.get("digest").expect("a digest").clone()),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_owned(), v))
        .collect(),
    );
    doc.canonical().into_bytes()
}

fn fixture(
    name: &str,
    id: &'static str,
    certificate: Vec<u8>,
    inputs: Vec<(&str, Vec<u8>)>,
    claims: &[&str],
    wrong: (&str, Vec<u8>),
) -> Fixture {
    Fixture {
        name: name.to_owned(),
        kind: CheckerKind::Archive(id),
        certificate,
        inputs: inputs.into_iter().map(|(r, b)| (r.to_owned(), b)).collect(),
        claims: claims.iter().map(|c| (*c).to_owned()).collect(),
        skip: Vec::new(),
        wrong_start: Some((wrong.0.to_owned(), wrong.1)),
        optional: Vec::new(),
    }
}

/// The mutation gate, whose structural mutations (a claim's key deleted, renamed or joined by an unknown key, a
/// claim list emptied or shortened) every fixture must also fail.
fn assert_gate(fixture: &Fixture) {
    let report = gate::run(&engines(), fixture).expect("a checker");
    assert!(report.passed(), "{}: {:#?}", fixture.name, report.problems);
    assert!(report.tampered() > 0, "{}", fixture.name);
}

fn matching_inputs() -> Vec<(&'static str, Vec<u8>)> {
    vec![("reference", stored(Y)), ("input", stored(Z))]
}

fn witness_inputs() -> Vec<(&'static str, Vec<u8>)> {
    vec![("menu", menu_record("I_a")), ("reference", stored(Y)), ("input", stored(Z)), ("end", stored(Z_END))]
}

fn macro_inputs() -> Vec<(&'static str, Vec<u8>)> {
    vec![("menu", menu_record("I_a")), ("start", stored(POST_A)), ("end", stored(COMPACT))]
}

fn exits_inputs() -> Vec<(&'static str, Vec<u8>)> {
    vec![
        ("menu", menu_record("I_a")),
        ("blocks", stored(RESULTS)),
        ("origin", stored(Y)),
        ("start", stored(POST_A)),
        ("end", stored(COMPACT)),
        ("long", stored(CLEANUP)),
    ]
}

fn block_rank_inputs() -> Vec<(&'static str, Vec<u8>)> {
    vec![("menu", menu_record("I_a")), ("blocks", stored(RESULTS)), ("origin", stored(Y)), ("start", stored(POST_A))]
}

fn weight_inputs() -> Vec<(&'static str, Vec<u8>)> {
    vec![("source", stored(COSETS)), ("start", stored(Y))]
}

/// A decoded record with one field replaced, re-serialized.
fn edited(member: &str, edit: impl FnOnce(&mut Value)) -> Vec<u8> {
    let mut doc = parse(&decoded(member));
    edit(&mut doc);
    doc.canonical().into_bytes()
}

fn member_mut<'v>(doc: &'v mut Value, path: &[&str]) -> &'v mut Value {
    path.iter().fold(doc, |v, key| match v {
        Value::Object(map) => map.get_mut(*key).unwrap_or_else(|| panic!("no {key}")),
        _ => panic!("not an object at {key}"),
    })
}

// --------------------------------------------------------------------------------------------- registration

#[test]
fn ct_lb5_06_claims_stage19_registry_is_clean() {
    assert!(archive::registry_problems(archive::registry()).is_empty());
    let ids: Vec<&str> = s19::SUBCHECKERS.iter().map(|s| s.id()).collect();
    assert_eq!(ids, ["s19.pose-table", "s19.gamma-matching", "s19.gamma-witness", "s19.frontier", "s19.ranks"]);
    let mut sources: Vec<String> = s19::SUBCHECKERS.iter().map(|s| s.checker.sources.identity().source).collect();
    sources.sort();
    sources.dedup();
    assert_eq!(sources.len(), ids.len(), "each sub-checker has its own source digest");
    for sub in s19::SUBCHECKERS {
        assert_eq!(sub.stage, 19);
        assert!(archive::find(sub.id()).is_some());
        let named: Vec<&str> = sub.checker.sources.files.iter().map(|(n, _)| *n).collect();
        assert!(named.contains(&"c600-checkers/src/archive/s19/common.rs"), "{}", sub.id());
    }
}

// --------------------------------------------------------------------------------------------- pose tables

#[test]
fn ct_lb5_06_claims_stored_and_decoded_digests_and_full_tables_check() {
    for member in [Y, Z, Z_END, POST_A, COMPACT, CLEANUP] {
        let certificate = pose_certificate(member);
        let report = check_bytes("s19.pose-table", &certificate, &sha(&certificate), &[("table", stored(member))]);
        assert_pass(&report);
        let table = parse(&decoded(member));
        let bound = report.inputs.iter().find(|i| i.role == "table").expect("the table is bound");
        assert_eq!(bound.sha256, text(&manifest(member), "sha256"), "{member}");
        let digest = bound.identity.iter().find(|(k, _)| k == "digest").map(|(_, v)| v.clone());
        assert_eq!(digest.as_deref(), table.get("digest").and_then(Value::as_str), "{member}");
    }
}

#[test]
fn ct_lb5_06_claims_pose_table_rejects_altered_and_inexact_tables() {
    let entry = manifest(Y);
    let with = |table: &Value| {
        let bytes = table.canonical().into_bytes();
        let certificate = Value::Object(
            [
                ("format", Value::String("kn6-s19-pose-table/1".into())),
                ("path", Value::String(Y.into())),
                ("sha256", Value::String(sha(&bytes))),
                ("stored_bytes", Value::Int(bytes.len() as i64)),
                ("uncompressed_sha256", Value::String(sha(&bytes))),
                ("uncompressed_bytes", Value::Int(bytes.len() as i64)),
                ("digest", table.get("digest").unwrap().clone()),
            ]
            .into_iter()
            .map(|(k, v)| (k.to_owned(), v))
            .collect(),
        )
        .canonical()
        .into_bytes();
        check_bytes("s19.pose-table", &certificate, &sha(&certificate), &[("table", bytes)])
    };
    let original = parse(&decoded(Y));
    // Re-encoded as plain JSON the same table passes: the check is of its content, not its gzip stream.
    assert_pass(&with(&original));
    assert_ne!(text(&entry, "sha256"), text(&entry, "uncompressed_sha256"));
    // A pose ID moved to another pose no longer gives the stated digest.
    let mut moved = original.clone();
    if let Value::Array(ids) = member_mut(&mut moved, &["pose_id"]) {
        let other = ids.iter().find(|v| **v != ids[0]).unwrap().clone();
        ids[0] = other;
    }
    let report = with(&moved);
    assert_eq!(report.verdict.code(), Some(c600_checkers::codes::START_TABLE_MISMATCH), "{:?}", report.verdict);
    // A pose entry out of lowest terms, and one that is no rotation.
    for (entry, why) in [
        (vec![Value::Int(2), Value::Int(0), Value::Int(2)], "lowest terms"),
        (vec![Value::Int(2), Value::Int(0), Value::Int(1)], "rotation"),
    ] {
        let mut bad = original.clone();
        if let Value::Array(poses) = member_mut(&mut bad, &["poses"]) {
            if let Value::Array(rows) = &mut poses[1] {
                if let Value::Array(row) = &mut rows[0] {
                    row[0] = Value::Array(entry.clone());
                }
            }
        }
        let report = with(&bad);
        assert!(report.verdict.is_fail(), "{why}: {:?}", report.verdict);
        assert_eq!(report.verdict.claim(), Some("table"), "{why}");
    }
}

#[test]
fn ct_lb5_06_claims_pose_table_mutation_gate() {
    let f = fixture(
        "S19 source pose table",
        "s19.pose-table",
        pose_certificate(Y),
        vec![("table", stored(Y))],
        &["sha256", "stored_bytes", "uncompressed_sha256", "uncompressed_bytes", "digest"],
        ("table", stored(Z)),
    );
    assert_gate(&f);
}

// --------------------------------------------------------------------------------------------- Γ(Y) matching

#[test]
fn ct_lb1_05_matching_half_certificate_meets_every_edge_and_brute_force_agrees() {
    let report = check_member("s19.gamma-matching", MATCHING, &matching_inputs());
    assert_pass(&report);
    // Three changed rows: nine edge tests, six bijections, one valid.
    assert_eq!(step(&report, "admission.tested_bijections"), "6");
    assert_eq!(step(&report, "admission.valid_bijections"), "1");
    assert!(step(&report, "scope.indexed_references").contains("not 73 distinct fibres"));
    assert!(step(&report, "scope.sticker_goal").starts_with("not transported"));
    // A bijection through an invalid edge breaks its edge constraint.
    let bad = edited(MATCHING, |doc| {
        let pairs = member_mut(doc, &["admission", "recovered_pi_overrides"]);
        if let Value::Array(pairs) = pairs {
            let (a, b) = (pairs[0].clone(), pairs[1].clone());
            if let (Value::Array(a), Value::Array(b)) = (a, b) {
                pairs[0] = Value::Array(vec![a[0].clone(), b[1].clone()]);
                pairs[1] = Value::Array(vec![b[0].clone(), a[1].clone()]);
            }
        }
    });
    let report = check_bytes("s19.gamma-matching", &bad, &sha(&bad), &matching_inputs());
    assert_eq!(report.verdict.claim(), Some("admission.recovered_pi_overrides[0]"), "{:?}", report.verdict);
    // Brute force disagrees with a second valid bijection.
    let bad = edited(MATCHING, |doc| *member_mut(doc, &["admission", "valid_bijections"]) = Value::Int(2));
    let report = check_bytes("s19.gamma-matching", &bad, &sha(&bad), &matching_inputs());
    assert_eq!(report.verdict.claim(), Some("admission.valid_bijections"));
}

#[test]
fn ct_lb1_05_matching_half_mutation_gate() {
    let f = fixture(
        "S19 Γ(Y) matching certificate",
        "s19.gamma-matching",
        decoded(MATCHING),
        matching_inputs(),
        &[
            "reference_digest",
            "input_digest",
            "admission.changed_rows",
            "admission.edge_tests[2].right_K_index",
            "admission.edge_tests[4].valid_edge",
            "admission.tested_bijections",
            "admission.valid_bijections",
            "admission.recovered_pi_overrides[1][1]",
            "admission.recovered_gauges[0].matrix[1][2][0]",
            "admission.all177120_bijection_and_type_preservation",
            "construction_home_maps[0].initial_physical_height",
            "construction_home_maps[1].home_vertices_target[2][1][0]",
            "construction_home_maps[2].complete_home_vertex_set_map",
            "construction_home_maps[2].actual_world_region_equality",
            "all177120_initial_region_bijection",
        ],
        ("reference", stored(Z)),
    );
    assert_gate(&f);
}

#[test]
fn ct_lb5_06_claims_synthetic_reachability_stays_unknown() {
    let claims = ["REACHABLE", "reachable from solved", "KNOWN", "UNKNOWN; reachable from solved by a stated word"];
    for value in claims {
        let bad = edited(MATCHING, |doc| *member_mut(doc, &["reachability"]) = Value::String(value.into()));
        let report = check_bytes("s19.gamma-matching", &bad, &sha(&bad), &matching_inputs());
        assert!(matches!(report.verdict, Verdict::NotApplicable { .. }), "{value}: {:?}", report.verdict);
    }
    for value in ["REACHABLE", "UNKNOWN; reachable from solved by a stated word"] {
        let bad = edited(RESULTS, |doc| *member_mut(doc, &["reachability"]) = Value::String(value.into()));
        let report = check_bytes("s19.gamma-witness", &bad, &sha(&bad), &witness_inputs());
        assert!(matches!(report.verdict, Verdict::NotApplicable { .. }), "{value}: {:?}", report.verdict);
    }
}

// --------------------------------------------------------------------------------------------- the witness

#[test]
fn ct_lb5_06_claims_nine_letter_witness_carries_legality_not_sticker_goals() {
    let report = check_member("s19.gamma-witness", RESULTS, &witness_inputs());
    assert_pass(&report);
    assert_eq!(step(&report, "full_State_apply_letters"), "18");
    assert_eq!(step(&report, "all18_prefix_Gamma_joins"), "true");
    assert_eq!(step(&report, "forward[8].support_equals_decoded_pi_inverse_of_reference"), "true");
    assert!(step(&report, "goal.sticker").starts_with("fails"));
    assert!(step(&report, "premise.length").contains("D: 4 letters, R unchanged"));
    assert!(step(&report, "scope.indexed_references").contains("not 73 distinct fibres"));
    assert!(step(&report, "reachability").starts_with("unknown"));
    // Claiming the endpoint restored fails.
    let bad = edited(RESULTS, |doc| *member_mut(doc, &["endpoint", "not_restored"]) = Value::Bool(false));
    let report = check_bytes("s19.gamma-witness", &bad, &sha(&bad), &witness_inputs());
    assert_eq!(report.verdict.claim(), Some("endpoint.not_restored"));
}

#[test]
fn ct_lb5_06_claims_witness_mutation_gate() {
    let f = fixture(
        "S19 Γ(Y) nine-letter witness",
        "s19.gamma-witness",
        decoded(RESULTS),
        witness_inputs(),
        &[
            "synthetic_input_digest",
            "physical_type_counts[2]",
            "weights_decimal[30]",
            "initial_metrics.S3",
            "forward[0].actual_support_ids[7]",
            "forward[0].reference_digest",
        ],
        ("input", stored(Y)),
    );
    assert_gate(&f);
}

#[test]
#[ignore = "slow: each tampered late claim replays up to eighteen letters on two full states; run with --ignored"]
fn ct_lb5_06_claims_witness_mutation_gate_late_claims() {
    let f = fixture(
        "S19 Γ(Y) nine-letter witness, late claims",
        "s19.gamma-witness",
        decoded(RESULTS),
        witness_inputs(),
        &[
            "forward[4].metrics.n",
            "forward[8].all177120_Gamma_join",
            "endpoint.not_restored",
            "endpoint.digest",
            "max_forward_prefix_height",
            "inverse[8].actual_digest",
            "all177120_exact_inverse_return",
            "boundaries[1].delta_S3",
            "boundaries[1].first_changed_type",
            "four_boundary_rank_audit",
        ],
        ("input", stored(Y)),
    );
    assert_gate(&f);
}

// --------------------------------------------------------------------------------------------- frontier

#[test]
fn ct_lb5_06_claims_frontier_exits_check_and_fail_the_sticker_goal() {
    let report = check_member("s19.frontier", MACRO, &macro_inputs());
    assert_pass(&report);
    let goal = step(&report, "goal.sticker");
    assert!(goal.contains("q0 to q15 are 0, q16 = 6") && goal.contains("sticker goal fails"), "{goal}");
    let report = check_member("s19.frontier", TRIMMED, &exits_inputs());
    assert_pass(&report);
    for exit in ["goal.sticker.compact", "goal.sticker.cleanup"] {
        let goal = step(&report, exit);
        assert!(goal.contains("q16 = 6") && goal.contains("sticker goal fails"), "{exit}: {goal}");
    }
    assert_eq!(step(&report, "D_exact_q_neutral_off_drop"), "1777");
    assert!(step(&report, "scope.q11_depth_certificate").contains("type-lex-successor-base-cache.npz"));
    // The compact and cleanup backgrounds stay distinct.
    let mut inputs = exits_inputs();
    inputs.iter_mut().find(|(r, _)| *r == "long").unwrap().1 = stored(COMPACT);
    let report = check_member("s19.frontier", TRIMMED, &inputs);
    assert!(report.verdict.is_fail(), "{:?}", report.verdict);
}

#[test]
fn ct_lb5_06_claims_frontier_mutation_gate() {
    let f = fixture(
        "S19 compact frontier suffix",
        "s19.frontier",
        decoded(MACRO),
        macro_inputs(),
        &[
            "input_digest",
            "prefixes[1].inside_count",
            "prefixes[2].metrics.n",
            "prefixes[0].allN_matrix_contract",
            "after.q[16]",
            "delta_q[11]",
            "first_changed_type",
            "strict_q_descent",
            "fixed_L4_rank.drop",
            "endpoint.digest",
            "all_applied_prefix_inverse_exact",
            "full_letters",
        ],
        ("start", stored(Y)),
    );
    assert_gate(&f);
}

#[test]
#[ignore = "slow: each tampered claim replays A, C, E and D on full states; run with --ignored"]
fn ct_lb5_06_claims_frontier_exits_mutation_gate() {
    let f = fixture(
        "S19 frontier exits",
        "s19.frontier",
        decoded(TRIMMED),
        exits_inputs(),
        &[
            "prefixes[0].digest",
            "prefixes[2].target114169_height",
            "all177120_short_endpoint_match",
            "canonical_short_metrics.n",
            "all_five_short_forward_prefixes_H4",
            "independent_inverse_all177120_exact",
            "all177120_long_equals_short_plus_recorded_D",
            "canonical_long_metrics.q[16]",
            "D_exact_q_neutral_off_drop",
        ],
        ("start", stored(Y)),
    );
    assert_gate(&f);
}

// --------------------------------------------------------------------------------------------- ranks

#[test]
fn ct_lb5_06_claims_ranks_on_stored_blocks_keep_their_premises() {
    let report = check_member("s19.ranks", WEIGHTS, &weight_inputs());
    assert_pass(&report);
    assert!(step(&report, "scope.counts").contains("derived anew"));
    let report = check_member("s19.ranks", TRIMMED, &block_rank_inputs());
    assert_pass(&report);
    assert!(step(&report, "premise.length").contains("R3 covers the compact blocks A, C, E"));
    // R_L's length premise: the four-letter D cannot join the length-3 rank's blocks.
    let bad = edited(TRIMMED, |doc| {
        *member_mut(doc, &["one_fixed_L3", "macro_lengths"]) =
            Value::Array([1, 4, 3, 1].into_iter().map(Value::Int).collect());
    });
    let report = check_bytes("s19.ranks", &bad, &sha(&bad), &block_rank_inputs());
    assert_eq!(report.verdict.claim(), Some("one_fixed_L3.macro_lengths"));
    assert!(matches!(&report.verdict, Verdict::Fail { reason, .. } if reason.contains("length premise")));
    // A discharged premise and an unconditional bound are global claims, never passes.
    let bad = edited(TRIMMED, |doc| {
        let key = "selector_closure_and_complete_low_tail_availability_remain_hypotheses";
        *member_mut(doc, &["generic_mixed_rank_proof_check", key]) = Value::Bool(false);
    });
    let report = check_bytes("s19.ranks", &bad, &sha(&bad), &block_rank_inputs());
    assert!(matches!(report.verdict, Verdict::NotApplicable { .. }), "{:?}", report.verdict);
    for formula in [
        "max(H-2,0)/2 * coefficient",
        "UNCONDITIONAL: max(H-2,0)/2 * coefficient",
        "max(H-2,0)/2 * coefficient, NOT CONDITIONAL on a terminating <=L lex-q-descending selector",
    ] {
        let bad =
            edited(WEIGHTS, |doc| *member_mut(doc, &["linear_high_cost_formula"]) = Value::String(formula.into()));
        let report = check_bytes("s19.ranks", &bad, &sha(&bad), &weight_inputs());
        assert!(matches!(report.verdict, Verdict::NotApplicable { .. }), "{formula}: {:?}", report.verdict);
    }
    // A premise flag of the wrong type is malformed, not discharged.
    let bad = edited(TRIMMED, |doc| {
        let key = "reused_sticker_theorem_not_recertified";
        *member_mut(doc, &["generic_mixed_rank_proof_check", key]) = Value::String("true".into());
    });
    let report = check_bytes("s19.ranks", &bad, &sha(&bad), &block_rank_inputs());
    assert!(report.verdict.is_fail(), "{:?}", report.verdict);
    // A list claim is compared element by element: the right boundary ranks merged into one string fail.
    let bad = edited(TRIMMED, |doc| {
        let list = member_mut(doc, &["one_fixed_L3", "boundary_R"]);
        let Value::Array(items) = list else { panic!("a list") };
        let merged = items.iter().map(|v| v.as_str().unwrap().to_owned()).collect::<Vec<_>>().join(",");
        *list = Value::Array(vec![Value::String(merged)]);
    });
    let report = check_bytes("s19.ranks", &bad, &sha(&bad), &block_rank_inputs());
    assert_eq!(report.verdict.claim(), Some("one_fixed_L3.boundary_R"), "{:?}", report.verdict);
}

#[test]
fn ct_lb5_06_claims_hostile_numbers_fail_and_never_panic() {
    let huge = Value::Int(i64::MAX);
    let cases: [(&str, &[&str], Value); 4] = [
        ("L", &["L"], huge.clone()),
        ("counterexample L", &["synthetic_unbounded_difference_counterexample", "L"], huge.clone()),
        ("counterexample delta", &["synthetic_unbounded_difference_counterexample", "delta"], {
            Value::Array(vec![Value::Int(-1), Value::Int(i64::MIN)])
        }),
        ("row delta", &["rows"], Value::Null),
    ];
    for (name, path, value) in cases {
        let bad = edited(WEIGHTS, |doc| {
            if name == "row delta" {
                let Value::Array(rows) = member_mut(doc, path) else { panic!("rows") };
                let Value::Object(row) = &mut rows[0] else { panic!("a row") };
                let Some(Value::Array(delta)) = row.get_mut("delta_q_by_type") else { panic!("a delta") };
                delta[35] = Value::Int(i64::MAX);
                delta[34] = Value::Int(i64::MAX);
            } else {
                *member_mut(doc, path) = value;
            }
        });
        let report = check_bytes("s19.ranks", &bad, &sha(&bad), &weight_inputs());
        assert!(report.verdict.is_fail(), "{name}: {:?}", report.verdict);
    }
    let bad = edited(RESULTS, |doc| *member_mut(doc, &["fixed_L"]) = Value::Int(i64::MAX));
    let report = check_bytes("s19.gamma-witness", &bad, &sha(&bad), &witness_inputs());
    assert_eq!(report.verdict.claim(), Some("fixed_L"), "{:?}", report.verdict);
}

#[test]
fn ct_lb5_06_claims_ranks_mutation_gate() {
    let f = fixture(
        "S19 fixed weights (L = 1)",
        "s19.ranks",
        decoded(WEIGHTS),
        weight_inputs(),
        &[
            "N",
            "L",
            "physical_type_counts[5]",
            "weights_decimal[0]",
            "all36_tail_weight_identities_exact",
            "product_all_factors_decimal",
            "sum_Nt_wt_decimal",
            "all_type_sum_identity_exact",
            "linear_high_cost_coefficient_decimal",
            "rows[0].rank_before",
            "rows[1].rank_after",
            "rows[2].first_delta_q",
            "rows[3].delta_q_by_type[20]",
            "rows[3].strict_descent",
            "synthetic_unbounded_difference_counterexample.weighted_delta",
            "synthetic_unbounded_difference_counterexample.violated_later_growth_bound",
        ],
        ("start", stored(POST_A)),
    );
    assert_gate(&f);
}

#[test]
#[ignore = "slow: each tampered claim replays thirteen letters from the source table; run with --ignored"]
fn ct_lb5_06_claims_block_ranks_mutation_gate() {
    let f = fixture(
        "S19 R3 and S on the stored blocks",
        "s19.ranks",
        decoded(TRIMMED),
        block_rank_inputs(),
        &[
            "one_fixed_L3.macro_lengths[1]",
            "one_fixed_L3.weights[4]",
            "one_fixed_L3.boundary_R[2]",
            "one_fixed_L3.first_changes[2].type",
            "one_fixed_L3.all_strict",
            "one_fixed_S.boundary_S[3]",
            "one_fixed_S.order[1]",
            "one_fixed_S.all_strict",
            "generic_mixed_rank_proof_check.literal_cost",
            "generic_mixed_rank_proof_check.peak",
            "generic_mixed_rank_proof_check.high_blocks_at_most_R0",
            "generic_mixed_rank_proof_check.S_strict_high_drop_dominates_n",
        ],
        ("origin", stored(POST_A)),
    );
    assert_gate(&f);
}

// --------------------------------------------------------------------------------------------- outside the families

#[test]
fn ct_lb5_06_claims_groups_outside_the_families_never_pass() {
    let q11 = {
        let doc = parse(&decoded(TRIMMED));
        doc.get("q11_depth_certificate").unwrap().canonical().into_bytes()
    };
    let mut certificates: Vec<(&str, Vec<u8>)> = vec![("q11 lower bound", q11)];
    for member in [
        "records/tail-coverage-completion-manifest.json",
        "records/tail-coverage-short-core-chain.json",
        "records/entry-profile-transfer-independent-check.json",
        "records/continuous-framework-independent-verified.json",
        "records/continuous-framework-independent-initial-result.json",
        "records/profile-family-review-sol-receipt.json",
        "records/profile-family-review-astra-results.json",
    ] {
        certificates.push((member, decoded(member)));
    }
    for sub in s19::SUBCHECKERS {
        for (name, bytes) in &certificates {
            let report = check_bytes(sub.id(), bytes, &sha(bytes), &[]);
            let Verdict::NotApplicable { reason } = &report.verdict else {
                panic!("{} on {name}: {:?}", sub.id(), report.verdict);
            };
            if *name == "q11 lower bound" {
                assert!(reason.contains("type-lex-successor-base-cache.npz"), "{reason}");
            } else {
                assert!(reason.contains("D[k] admission") && reason.contains("remaining"), "{reason}");
            }
        }
    }
}

#[test]
fn ct_lb5_06_claims_malformed_records_fail_by_field() {
    // A record of a sub-checker's form with a key removed, renamed or added fails and names the field.
    let removed = edited(MATCHING, |doc| {
        if let Value::Object(map) = doc {
            map.remove("input_digest");
        }
    });
    let report = check_bytes("s19.gamma-matching", &removed, &sha(&removed), &matching_inputs());
    assert_eq!(report.verdict.claim(), Some("certificate"), "{:?}", report.verdict);
    let added = edited(MACRO, |doc| {
        if let Value::Object(map) = doc {
            map.insert("restored".into(), Value::Bool(true));
        }
    });
    let report = check_bytes("s19.frontier", &added, &sha(&added), &macro_inputs());
    assert_eq!(report.verdict.claim(), Some("certificate"), "{:?}", report.verdict);
    let renamed = edited(WEIGHTS, |doc| {
        if let Value::Object(map) = doc {
            let v = map.remove("rows").unwrap();
            map.insert("row".into(), v);
        }
    });
    let report = check_bytes("s19.ranks", &renamed, &sha(&renamed), &weight_inputs());
    assert_eq!(report.verdict.claim(), Some("certificate"), "{:?}", report.verdict);
}

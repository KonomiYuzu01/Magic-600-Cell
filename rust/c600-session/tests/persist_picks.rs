//! LP2 conformance tests for FP.38's pick record (`c600_session::persist::pick`): the format packet's `picks` vectors,
//! read in place, the payload rules a writer keeps, and the vectors' picks at `c600-run`'s preview admission.
//!
//! Expected values come from the packet or from the test's own edits of the packet's record, never from the library
//! under test.
//!
//! Evidence kind: source and fixture. One run covers one platform; nothing here is Windows/DirectX evidence.

mod persist_support;

use std::collections::BTreeMap;

use c600_digest::sha256_hex;
use c600_run::boundary::{admit_preview, Pins, Store};
use c600_run::provenance::{field, INCOMPATIBLE};
use c600_session::persist::codec::{self, Json};
use c600_session::persist::codes::{FORMAT, PROVENANCE};
use c600_session::persist::{pick, record};
use persist_support::*;

/// The records of a vector, by the SHA-256 of their FP.5 bytes.
fn records_of(given: &json::Json) -> BTreeMap<String, Json> {
    let records = given.get("records").and_then(json::Json::arr).expect("records");
    records.iter().map(block).map(|r| (sha256_hex(&codec::canonical(&r)), r)).collect()
}

/// A vector's block, read back from its FP.5 bytes as the packet's checker reads it.
fn block_of(given: &json::Json) -> Json {
    let raw = block(given.get("block").expect("block"));
    codec::read_canonical(&codec::canonical(&raw)).expect("the block is FP.5 JSON")
}

/// FP.38 (source, whole: the packet's fourteen `picks` vectors): a built-in pick resolves to its record under each
/// LQ21 form and answers the record's `chosen`; a missing record, a Ref to another kind, a differing header or a
/// differing `chosen` is `PS1:capability.provenance_incompatible`; FP.10's version refusal comes before the Ref's shape
/// and its kind, and an added Ref field is `PS1:input.persistence_format`.
#[test]
fn fp38_picks_resolve_in_fp10_order() {
    let picks = vectors("picks");
    assert_eq!(picks.len(), 14);
    let mut resolved = 0;
    for vector in &picks {
        let id = text(vector, "id");
        let given = vector.get("given").expect("given");
        let (block, records) = (block_of(given), records_of(given));
        let answer = pick::resolve(&block, &records);
        match vector.get("expected").expect("expected") {
            json::Json::Str(code) => assert_eq!(outcome(answer), *code, "{id}"),
            expected => {
                let answer = answer.unwrap_or_else(|refusal| panic!("{id}: {refusal}"));
                assert_eq!(answer.chosen, text(expected, "chosen"), "{id}");
                assert_eq!(sha256_hex(&codec::canonical(&block)), text(expected, "block"), "{id}");
                assert!(answer.fixed, "{id}: the vectors' built-in pick names a fixed entry");
                resolved += 1;
            }
        }
    }
    assert_eq!(resolved, 3, "a built-in pick under each LQ21 form");
}

/// FP.38 (source, part: the writer's side): the packet's pick record is written; a payload field added or missing, a
/// `fixed` that is no Bool, a `chosen` that is no `DigestRef(content)` or a non-null `provenance` is
/// `PS1:input.persistence_format` at write and at resolution.
#[test]
fn fp38_a_pick_record_keeps_its_payload() {
    let vector = vectors("picks").into_iter().find(|v| text(v, "id") == "built-in-pick-capability_separation");
    let given = vector.expect("the vector").get("given").cloned().expect("given");
    let good = block(&given.get("records").and_then(json::Json::arr).expect("records")[0]);
    assert!(record::write_record(&good).is_ok());
    let block = block_of(&given);
    let mut broken = Vec::new();
    let mut added = good.clone();
    set(at(&mut added, &["payload"]), "rank", Json::Int(1));
    broken.push(("an added field", added));
    let mut missing = good.clone();
    remove(at(&mut missing, &["payload"]), "options");
    broken.push(("a missing field", missing));
    let mut fixed = good.clone();
    set(at(&mut fixed, &["payload"]), "fixed", Json::text("yes"));
    broken.push(("a text `fixed`", fixed));
    let mut list = good.clone();
    set(at(&mut list, &["payload", "list"]), "role", Json::text("bytes"));
    broken.push(("a `list` of another role", list));
    let mut provenance = good.clone();
    set(&mut provenance, "provenance", block.clone());
    broken.push(("a provenance block", provenance));
    for (what, record) in broken {
        assert_eq!(outcome(record::write_record(&record)), FORMAT, "{what}");
        assert_eq!(outcome(pick::check_payload(&record)), FORMAT, "{what}");
    }

    // At resolution: a record whose payload breaks FP.38 and whose digest the block names.
    let mut odd = good.clone();
    set(at(&mut odd, &["payload"]), "fixed", Json::Null);
    let mut named = block.clone();
    set(at(&mut named, &["pick"]), "record", record::reference(&odd));
    let records = BTreeMap::from([(sha256_hex(&codec::canonical(&odd)), odd)]);
    assert_eq!(outcome(pick::resolve(&named, &records)), FORMAT);

    // A block without a pick names no record.
    let mut none = block;
    set(&mut none, "pick", Json::Null);
    assert_eq!(outcome(pick::resolve(&none, &records)), PROVENANCE);
}

/// CT-FP-02 (part: a non-null built-in pick under each LQ21 form at preview admission, which `c600-run`'s
/// `tests/boundary.rs` leaves to this file) with FP.38 and BND.85: with the vector's records kept and the
/// capability-form vector's contract pin held, the capability-form built-in pick resolves and passes admission, and
/// the flow-form ones stay in their form (CT-BND-22). Each refusing vector's pick, moved into the capability-form
/// block, is refused at the pick with `PS1:capability.provenance_incompatible`, and the resolving picks pass there.
#[test]
fn fp38_a_built_in_pick_resolves_at_preview_admission() {
    let picks = vectors("picks");
    let base = picks.iter().find(|v| text(v, "id") == "built-in-pick-capability_separation").expect("the vector");
    let base = block_of(base.get("given").expect("given"));
    let contracts = field::contract(&base).into_iter().collect();
    let pins = Pins { contracts, checkers: field::checker(&base).into_iter().collect() };
    for vector in &picks {
        let id = text(vector, "id");
        let given = vector.get("given").expect("given");
        let mut store = Store::default();
        for record in records_of(given).values() {
            store.keep_pick(&codec::canonical(record));
        }
        let own = block_of(given);
        let admitted = admit_preview(&codec::canonical(&own), &store, &pins).map_err(|refusal| refusal.code);
        let mut moved = base.clone();
        set(&mut moved, "pick", own.get("pick").cloned().expect("a pick"));
        let answer = admit_preview(&codec::canonical(&moved), &store, &pins);
        if let Some(json::Json::Str(_)) = vector.get("expected") {
            assert_eq!(admitted, Err(INCOMPATIBLE), "{id}");
            let refusal = answer.expect_err(id);
            assert_eq!(refusal.code, INCOMPATIBLE, "{id}");
            assert!(refusal.detail.contains("pick"), "{id}: {}", refusal.detail);
        } else {
            assert_eq!(answer, Ok(()), "{id}: moved");
            let own_form = field::form(&own) == "capability_separation";
            assert_eq!(admitted, if own_form { Ok(()) } else { Err(INCOMPATIBLE) }, "{id}");
        }
    }
}

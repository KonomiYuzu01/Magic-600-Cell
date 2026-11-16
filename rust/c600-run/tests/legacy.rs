//! Legacy and unsupported provenance blocks at the run-time boundary (chapter 05, PS1.32 to PS1.34; chapter 07,
//! BND.21 and BND.23; format packet FP.17 and FP.18).
//!
//! CT-PS1-18 (part: the run-time store and preview admission; not run: saving and reloading such a block through the
//! session store, which M6 builds in `rust/c600-session`; the store here is M3's in-memory one). The run-level join, in
//! which a legacy or damaged part never makes a run's output read `read_state: no`, is in `boundary.rs`
//! (`damaged_and_legacy_parts_never_read_as_no`).

use c600_digest::sha256;
use c600_run::boundary::{admit_preview, Pins, Place, Store, Stored};
use c600_run::provenance::{self, field, Author, Json, Pin, Producer, Profile, INCOMPATIBLE};
use c600_run::value::{equal, Value};

/// The captured bytes of a 0.4 saved recipe, as FP.18's legacy example records them.
const RECIPE: &[u8] = b"[{\"kind\":\"word\",\"moves\":[1,-1]}]";

/// FP.18's fixture capability contract, held by the policy in these tests.
fn contract() -> Pin {
    Pin::of("fixture-1", b"p3 fixture capability contract 1\n")
}

fn pins() -> Pins {
    Pins { contracts: vec![contract()], checkers: Vec::new() }
}

fn edit(json: &Json, key: &str, value: Option<Json>) -> Json {
    let Json::Object(mut map) = json.clone() else { panic!("an object") };
    match value {
        Some(v) => map.insert(key.into(), v),
        None => map.remove(key),
    };
    Json::Object(map)
}

/// The block of a built-in solving tool's output that took in `parts` and read no state itself.
fn solving_output(parts: &[[u8; 32]]) -> Vec<u8> {
    let producer = Producer {
        contract: contract(),
        checker: None,
        author: Author::BuiltIn,
        module: sha256(b"built-in solving tool"),
        entry: "go".into(),
        profile: Profile::Solving,
    };
    provenance::canonical(&provenance::program(&producer, false, parts))
}

/// CT-PS1-18 (part, above): an imported 0.4 macro enters as the solver's own, imported, with its captured source digest
/// and the legacy detail it lacks; its dependencies are `not_recorded`, which never reads as `no`.
#[test]
fn ct_ps1_18_imported_0_4_origin_stays_own_and_imported() {
    let block = provenance::import(&contract(), RECIPE);
    let bytes = provenance::canonical(&block);
    assert_eq!(provenance::read(&bytes), Ok(block.clone()));
    assert_eq!((field::author(&block), field::producer(&block), field::profile(&block)), ("own", "import", "none"));
    let origin = provenance::digest_ref("bytes", &sha256(RECIPE));
    assert_eq!(block.get("producer").and_then(|p| p.get("origin")), Some(&origin));
    let legacy = block.get("legacy").expect("a legacy field");
    assert_eq!(legacy.get("source"), Some(&origin));
    let missing = Json::List(["producing_run", "dependency_history"].map(Json::text).into());
    assert_eq!(legacy.get("missing"), Some(&missing));
    assert_eq!(block.get("depends"), Some(&Json::object([("kind", Json::text("not_recorded"))])));
    assert!(field::may_read_state(&block), "`not_recorded` never reads as `read_state: no` (FP.17)");
    assert!(!field::may_read_state(&provenance::manual(&contract())), "the control: a typed word reads `no`");

    // The import origin grants nothing beyond the solver's own: admitted under the policy's pin, refused without it.
    let store = Store::default();
    assert_eq!(admit_preview(&bytes, &store, &pins()), Ok(()));
    assert_eq!(admit_preview(&bytes, &store, &Pins::default()).unwrap_err().code, INCOMPATIBLE);
}

/// CT-PS1-18 (part, above): a block of an unsupported version, an unknown form, another form, with an unknown field
/// or in non-canonical bytes is kept byte for byte, reads `unknown` or stays in its own form, and is never admitted:
/// neither alone nor as a part of a solving tool's word.
#[test]
fn ct_ps1_18_unsupported_blocks_stay_recorded_and_are_never_admitted() {
    let good = provenance::import(&contract(), RECIPE);
    let meta = good.get("block").expect("block metadata");
    let canonical = provenance::canonical(&good);
    let spaced = String::from_utf8(canonical.clone()).unwrap().replace(':', ": ").into_bytes();
    let with_meta =
        |key: &str, value: Json| provenance::canonical(&edit(&good, "block", Some(edit(meta, key, Some(value)))));
    let cases = [
        ("version 2", with_meta("version", Json::Int(2))),
        ("unknown form", with_meta("form", Json::text("flow_v0"))),
        ("unknown field", provenance::canonical(&edit(&good, "origin_v0", Some(Json::text("0.3"))))),
        ("non-canonical bytes", spaced),
        ("another form", with_meta("form", Json::text("flow_static"))),
    ];
    for (name, bytes) in cases {
        assert_ne!(bytes, canonical, "{name}");
        let reads = provenance::read(&bytes);
        if name == "another form" {
            // A valid FP.17 block of the unselected flow form: it reads, keeps its form and is not converted.
            assert_eq!(reads.as_ref().map(field::form), Ok("flow_static"), "{name}");
        } else {
            assert_eq!(reads, Err(INCOMPATIBLE), "{name}: reads `unknown` (BND.21)");
        }

        // Stays recorded: the store keeps the bytes as written, beside the value, and a later write of the same
        // block changes nothing (PS1.33).
        let mut store = Store::default();
        let place = Place::Workspace("imported".into());
        let stored = Stored { value: Value::word(&[1, -1]), bytes: b"[1,-1]\n".to_vec(), block: bytes.clone() };
        store.records.insert(place.clone(), stored);
        let digest = store.keep(&bytes);
        assert_eq!(digest, sha256(&bytes), "{name}");
        assert_eq!(store.keep(&bytes), digest, "{name}");
        assert_eq!(store.blocks.get(&digest), Some(&bytes), "{name}");
        assert_eq!(store.records[&place].block, bytes, "{name}");
        assert!(equal(&store.records[&place].value, &Value::word(&[1, -1])), "{name}");

        // Never an unlabelled solving word: refused alone, and a solving tool's word citing it is refused at the part.
        let alone = admit_preview(&bytes, &store, &pins()).unwrap_err();
        assert_eq!(alone.code, INCOMPATIBLE, "{name}: {}", alone.detail);
        let cited = admit_preview(&solving_output(&[digest]), &store, &pins()).unwrap_err();
        assert_eq!(cited.code, INCOMPATIBLE, "{name}: {}", cited.detail);
        assert!(cited.detail.starts_with(&format!("part {}", provenance::hex(&digest))), "{name}: {}", cited.detail);
    }

    // The control: the same solving word citing the supported import block is admitted.
    let mut store = Store::default();
    let digest = store.keep(&canonical);
    assert_eq!(admit_preview(&solving_output(&[digest]), &store, &pins()), Ok(()));
}

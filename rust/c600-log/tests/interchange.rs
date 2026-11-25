//! LP4-13: 0.4 proof logs and MPUlt logs as interchange. The fixtures were made by 0.4's own code
//! (`tests/fixtures/make_fixtures.py`), with `expected.json` holding what 0.4 returned for each. They import to the
//! states 0.4 reaches; export then import is the identity on them; an MPUlt log over a cap or of another puzzle is
//! refused with its own code; `log_io.py` and the other 0.4 sources are unchanged.

mod common;

use std::sync::OnceLock;

use c600_kernel::archive::gunzip;
use c600_kernel::json::Value;
use c600_log::c600::{self, Compression};
use c600_log::mpult::{self, NativeProfile};
use c600_log::{code, compile, write, Commit, Gate, Interchange, Log, MoveAction, Origin, Problem, Recorder};

use common::{expected, fixture, int, kernel, repository, sha256, text, walk};

const C600: [&str; 2] = ["c600-session.c600.json.gz", "c600-scramble.c600.json"];
const MPULT: [&str; 3] = ["mpult-negative.log.gz", "mpult-partial-redo.log.gz", "mpult-export.log.gz"];

/// An MPUlt fixture's own bytes: the fixtures keep them gzipped.
fn mpult_fixture(name: &str) -> Vec<u8> {
    gunzip(&fixture(name), 64 << 20).expect("the fixture is gzip")
}

fn profile_bytes() -> Vec<u8> {
    mpult::profile_bytes(&fixture("mpult-profile.json.gz")).expect("the profile fixture reads")
}

/// 0.4's synthetic native profile (`test_native_bridge.make_fixture` through `native_bridge.verify_native`).
fn profile() -> &'static NativeProfile {
    static PROFILE: OnceLock<NativeProfile> = OnceLock::new();
    PROFILE.get_or_init(|| NativeProfile::read(kernel(), &profile_bytes()).expect("the 0.4 profile passes"))
}

fn commits(log: &Log) -> Vec<&Commit> {
    log.moves
        .iter()
        .filter_map(|e| match &e.action {
            MoveAction::Commit(c) => Some(c),
            _ => None,
        })
        .collect()
}

/// The imported log without the digest of the file it came from.
fn without_source(mut log: Log) -> Log {
    match &mut log.origin {
        Origin::C600 { source } => source.clear(),
        Origin::MPUlt(origin) => origin.source.clear(),
        Origin::Live => {}
    }
    log
}

fn compiled(log: &Log) -> c600_log::Compiled {
    let compiled = compile(kernel(), &write(log).unwrap());
    assert!(compiled.passed(), "{:#?}", compiled.problems());
    compiled
}

/// Each transaction against what 0.4 returned: recipe, counts, states, note and assistance.
fn assert_events(log: &Log, events: &[Value], what: &str) {
    let commits = commits(log);
    assert_eq!(commits.len(), events.len(), "{what}");
    for (i, (commit, event)) in commits.iter().zip(events).enumerate() {
        assert_eq!(&commit.recipe, walk(event, &["recipe"]), "{what} {i}");
        assert_eq!(commit.primitive_count as i64, int(event, &["length"]), "{what} {i}");
        assert_eq!(commit.stars as i64, int(event, &["stars"]), "{what} {i}");
        assert_eq!(commit.pre, text(event, &["pre"]), "{what} {i}");
        assert_eq!(commit.post, text(event, &["post"]), "{what} {i}");
        assert_eq!(commit.note, text(event, &["note"]), "{what} {i}");
        assert_eq!(commit.assistance, text(event, &["assistance"]), "{what} {i}");
    }
}

#[test]
fn a_04_fixture_session_written_as_a_log_compiles_to_the_sessions_hash() {
    let k = kernel();
    for name in C600 {
        let log = c600::import(k, &fixture(name)).unwrap();
        let bytes = write(&log).unwrap();
        let compiled = compile(k, &bytes);
        assert!(compiled.passed(), "{name}: {:#?}", compiled.problems());
        assert!(!compiled.read_only());
        assert_eq!(compiled.state().unwrap().digest(), text(expected(), &["c600", name, "session"]), "{name}");
    }
}

#[test]
fn the_04_sources_and_fixtures_are_unchanged() {
    let sources = walk(expected(), &["sources"]).as_object().unwrap();
    assert!(sources.contains_key("log_io.py") && sources.contains_key("mpult_log.py"));
    for (name, digest) in sources {
        let bytes = std::fs::read(repository().join(name)).unwrap();
        assert_eq!(sha256(&bytes), digest.as_str().unwrap(), "{name} changed since the fixtures were made");
    }
    for name in C600 {
        assert_eq!(sha256(&fixture(name)), text(expected(), &["c600", name, "sha256"]), "{name}");
    }
    for name in MPULT {
        assert_eq!(sha256(&mpult_fixture(name)), text(expected(), &["mpult", name, "sha256"]), "{name}");
    }
    assert_eq!(sha256(&profile_bytes()), text(expected(), &["profile_sha256"]));
    assert_eq!(text(expected(), &["model_id"]), kernel().identity().model_id);
}

#[test]
fn c600_fixtures_import_to_the_states_04_reaches() {
    let k = kernel();
    for name in C600 {
        let want = walk(expected(), &["c600", name]);
        let plan = c600::check(k, &fixture(name)).unwrap();
        assert_eq!(plan.state.digest(), text(want, &["state"]), "{name}");
        assert_eq!(plan.primitive_count as i64, int(want, &["primitive_count"]), "{name}");
        assert_eq!(plan.events.len() as i64, int(want, &["transactions"]), "{name}");
        let log = c600::import(k, &fixture(name)).unwrap();
        assert_events(&log, walk(want, &["events"]).as_array().unwrap(), name);
        let compiled = compiled(&log);
        assert_eq!(compiled.state().unwrap().digest(), text(want, &["session"]), "{name}");
        assert!(log.preferences.is_some(), "{name}: the preferences are recorded");
    }
}

#[test]
fn c600_export_then_import_is_the_identity_on_the_fixtures() {
    let k = kernel();
    // The raw fixture exports to its own bytes.
    let raw = fixture("c600-scramble.c600.json");
    let log = c600::import(k, &raw).unwrap();
    let exported = c600::export(k, &compiled(&log), Compression::None).unwrap();
    assert_eq!(exported, raw);
    // The gzip fixture (0.4 compresses at level 5) exports to the same JSON in a stored member, which 0.4 reads.
    let packed = fixture("c600-session.c600.json.gz");
    let log = c600::import(k, &packed).unwrap();
    let exported = c600::export(k, &compiled(&log), Compression::Gzip).unwrap();
    assert_eq!(gunzip(&exported, 1 << 26).unwrap(), gunzip(&packed, 1 << 26).unwrap());
    for name in C600 {
        let log = c600::import(k, &fixture(name)).unwrap();
        for compression in [Compression::None, Compression::Gzip] {
            let exported = c600::export(k, &compiled(&log), compression).unwrap();
            let again = c600::import(k, &exported).unwrap();
            assert_eq!(without_source(again), without_source(log.clone()), "{name} {compression:?}");
        }
    }
}

#[test]
fn mpult_fixtures_import_to_the_states_04_reaches() {
    let k = kernel();
    for name in MPULT {
        let want = walk(expected(), &["mpult", name]);
        let bytes = mpult_fixture(name);
        let plan = mpult::check(k, profile(), &bytes).unwrap();
        assert_eq!(plan.state.digest(), text(want, &["state"]), "{name}");
        assert_eq!(plan.selected_count as i64, int(want, &["selected_count"]), "{name}");
        assert_eq!(plan.primitive_count as i64, int(want, &["primitive_count"]), "{name}");
        let native = walk(want, &["native_log"]);
        assert_eq!(plan.sequence_entries as i64, int(native, &["sequence_entries"]), "{name}");
        assert_eq!(plan.pointer as i64, int(native, &["pointer"]), "{name}");
        assert_eq!(plan.shuffle as i64, int(native, &["shuffle_boundary"]), "{name}");
        assert_eq!(plan.timer as i64, int(native, &["timer_milliseconds"]), "{name}");
        assert_eq!((plan.events.len() - plan.selected_count) as i64, int(native, &["redo_transactions"]), "{name}");
        let log = mpult::import(k, profile(), &bytes).unwrap();
        assert_events(&log, walk(want, &["events"]).as_array().unwrap(), name);
        let compiled = compiled(&log);
        assert_eq!(compiled.state().unwrap().digest(), text(want, &["state"]), "{name}");
        if let Some(session) = want.get("session") {
            assert_eq!(compiled.state().unwrap().digest(), session.as_str().unwrap(), "{name}");
        }
    }
    // The partial-redo log keeps its redo tail after the saved cursor.
    let log = mpult::import(k, profile(), &mpult_fixture("mpult-partial-redo.log.gz")).unwrap();
    assert_eq!(commits(&log).len(), 3);
    assert_eq!(log.saved.moves.as_deref(), Some("1"));
    assert!(matches!(log.moves.last().unwrap().action, MoveAction::Cursor(_)));
}

#[test]
fn mpult_export_then_import_is_the_identity_on_the_fixtures() {
    let k = kernel();
    for name in MPULT {
        let bytes = mpult_fixture(name);
        let log = mpult::import(k, profile(), &bytes).unwrap();
        let exported = mpult::export(k, profile(), &compiled(&log), None).unwrap();
        assert_eq!(String::from_utf8_lossy(&exported), String::from_utf8_lossy(&bytes), "{name}");
        let again = mpult::import(k, profile(), &exported).unwrap();
        assert_eq!(again, log, "{name}");
    }
}

#[test]
fn a_session_crosses_between_the_formats() {
    let k = kernel();
    let parse = |text: &str| c600_kernel::json::parse(text.as_bytes()).unwrap();
    // The session 0.4 exported to the MPUlt fixture: one transaction of a word and a star, written letter by letter
    // through the profile's translations, gives 0.4's bytes exactly.
    let mut r = Recorder::new(k);
    let recipe = parse(r#"[{"kind":"word","moves":[1,2,-4]},{"kind":"star","orbit":33,"node":0,"sign":1}]"#);
    r.commit(&recipe, "hand", "", None).unwrap();
    let live = compiled(&r.finish());
    let want = text(expected(), &["mpult", "mpult-export.log.gz", "session"]);
    assert_eq!(live.state().unwrap().digest(), want);
    let native = mpult::export(k, profile(), &live, Some(12_000)).unwrap();
    assert_eq!(String::from_utf8_lossy(&native), String::from_utf8_lossy(&mpult_fixture("mpult-export.log.gz")));
    // A 0.4 scramble crosses to MPUlt with its shuffle boundary after the scramble's moves, and back to its state.
    let log = c600::import(k, &fixture("c600-scramble.c600.json")).unwrap();
    let native = mpult::export(k, profile(), &compiled(&log), None).unwrap();
    let plan = mpult::check(k, profile(), &native).unwrap();
    assert_eq!(plan.state.digest(), text(expected(), &["c600", "c600-scramble.c600.json", "session"]));
    assert_eq!((plan.shuffle, plan.events.len()), (30, 32));
    assert!(plan.events[..30].iter().all(|e| e.assistance == "recorded-scramble-native-import"));
    // An MPUlt import exports as a 0.4 proof log that 0.4's checks accept with the same state.
    let log = mpult::import(k, profile(), &mpult_fixture("mpult-negative.log.gz")).unwrap();
    let proof = c600::export(k, &compiled(&log), Compression::Gzip).unwrap();
    let plan = c600::check(k, &proof).unwrap();
    assert_eq!(plan.state.digest(), text(expected(), &["mpult", "mpult-negative.log.gz", "state"]));
    // A live log with a partial undo exports to both formats with the state at its cursor.
    let mut r = Recorder::new(k);
    r.commit(&parse(r#"[{"kind":"word","moves":[1,2,-4]}]"#), "hand", "", None).unwrap();
    r.commit(&parse(r#"[{"kind":"word","moves":[7,9,11]}]"#), "hand", "", None).unwrap();
    r.partial_undo(1, None).unwrap();
    let state = r.state().digest().to_owned();
    let live = compiled(&r.finish());
    let proof = c600::export(k, &live, Compression::None).unwrap();
    assert_eq!(c600::check(k, &proof).unwrap().state.digest(), state);
    let native = mpult::export(k, profile(), &live, None).unwrap();
    assert_eq!(mpult::check(k, profile(), &native).unwrap().state.digest(), state);
}

fn refusal(result: Result<impl std::fmt::Debug, Problem>) -> &'static str {
    let problem = result.expect_err("refused");
    assert_eq!(problem.gate, Gate::Interchange);
    problem.code
}

/// An MPUlt fixture with its text edited.
fn edited(name: &str, edit: impl FnOnce(String) -> String) -> Vec<u8> {
    edit(String::from_utf8(mpult_fixture(name)).unwrap()).into_bytes()
}

#[test]
fn mpult_logs_over_a_cap_or_of_another_puzzle_are_refused_with_their_own_codes() {
    let k = kernel();
    let p = profile();
    assert_eq!((mpult::MAX_MOVES, mpult::MAX_ENTRIES, mpult::MAX_LOG_BYTES), (10_000, 20_000, 16 << 20));
    // More sequence entries than 20,000.
    let entries = edited("mpult-negative.log.gz", |t| t.replacen("600-cell-Full 3 0 3", "600-cell-Full 20001 0 3", 1));
    assert_eq!(refusal(mpult::check(k, p, &entries)), code::PERSISTENCE_SIZE);
    // More native moves than 10,000.
    let moves = edited("mpult-negative.log.gz", |t| {
        let head = t.split("\n*\n").next().unwrap().replacen("600-cell-Full 3 0 3", "600-cell-Full 10001 0 10001", 1);
        format!("{head}\n*\n{} .\n", vec!["0:0:1:1"; 10_001].join(" "))
    });
    assert_eq!(refusal(mpult::check(k, p, &moves)), code::PERSISTENCE_SIZE);
    // More than 16 MiB.
    let mut large = mpult_fixture("mpult-negative.log.gz");
    large.resize((16 << 20) + 1, b' ');
    assert_eq!(refusal(mpult::check(k, p, &large)), code::PERSISTENCE_SIZE);
    // Another puzzle, by name or by description.
    let other = edited("mpult-negative.log.gz", |t| t.replace("600-cell-Full", "120-cell"));
    assert_eq!(refusal(mpult::check(k, p, &other)), code::IDENTITY_MISMATCH);
    let geometry = edited("mpult-negative.log.gz", |t| t.replacen("Cuts 0.968 -0.968", "Cuts 0.9 -0.968", 1));
    assert_eq!(refusal(mpult::check(k, p, &geometry)), code::IDENTITY_MISMATCH);
    let dimensions = edited("mpult-negative.log.gz", |t| t.replacen("\n2 259800\n", "\n2 120\n", 1));
    assert_eq!(refusal(mpult::check(k, p, &dimensions)), code::IDENTITY_MISMATCH);
    // Another MPUltimate version.
    let version = edited("mpult-negative.log.gz", |t| t.replacen("MPUltimate v1", "MPUltimate v2", 1));
    assert_eq!(refusal(mpult::check(k, p, &version)), code::PERSISTENCE_VERSION);
    // Each code is its own.
    let codes = [code::PERSISTENCE_SIZE, code::IDENTITY_MISMATCH, code::PERSISTENCE_VERSION];
    assert_eq!(codes.iter().collect::<std::collections::BTreeSet<_>>().len(), 3);
}

#[test]
fn mpult_checks_follow_04() {
    let k = kernel();
    let p = profile();
    // Description numbers compare as exact decimals: trailing zeros are the same geometry.
    let zeros =
        edited("mpult-negative.log.gz", |t| t.replacen("Faces 4.236068,1,1,1", "Faces  4.236068000000,1.0,1,+1", 1));
    let want = text(expected(), &["mpult", "mpult-negative.log.gz", "state"]);
    assert_eq!(mpult::check(k, p, &zeros).unwrap().state.digest(), want);
    // The CRC covers the counters, the sequence and the colors.
    let crc = edited("mpult-negative.log.gz", |t| t.replacen("#timer 12345", "#timer 12346", 1));
    assert_eq!(refusal(mpult::check(k, p, &crc)), code::DIGEST_MISMATCH);
    // A changed color fails the CRC; colors with a valid CRC that disagree with the replay at the pointer fail it.
    let mut colors = mpult_fixture("mpult-negative.log.gz");
    let start = colors.windows(9).position(|w| w == b"\n2 259800").unwrap() + 10;
    colors[start] = if colors[start] == b'0' { b'1' } else { b'0' };
    assert_eq!(refusal(mpult::check(k, p, &colors)), code::DIGEST_MISMATCH);
    let solved = p.colors(&k.solved());
    let one = mpult::serialize(&solved, &["0:0:1:1".to_owned()], 1, 1, 0);
    assert_eq!(refusal(mpult::check(k, p, &one)), code::CHECKPOINT_REPLAY);
    let none = mpult::serialize(&solved, &["0:0:1:1".to_owned()], 1, 0, 0);
    assert_eq!(mpult::check(k, p, &none).unwrap().state.digest(), k.solved().digest());
    // A move token the bridge does not know.
    let unknown = edited("mpult-negative.log.gz", |t| t.replacen("0:0:1:1", "0:0:1:2", 1));
    assert_eq!(refusal(mpult::check(k, p, &unknown)), code::PERSISTENCE_FORMAT);
    // A profile of another model, and a profile whose digest is not its own.
    let mut value = c600_kernel::json::parse(&profile_bytes()).unwrap();
    if let Value::Object(fields) = &mut value {
        fields.insert("seconds".to_owned(), Value::Int(1));
    }
    assert_eq!(refusal(NativeProfile::read(k, value.canonical().as_bytes())), code::DIGEST_MISMATCH);
}

#[test]
fn c600_logs_follow_04s_limits_and_codes() {
    let k = kernel();
    let record = c600::decode(&fixture("c600-scramble.c600.json")).unwrap();
    let with = |key: &str, value: Value| {
        let Value::Object(mut fields) = record.clone() else { panic!() };
        fields.insert(key.to_owned(), value);
        Value::Object(fields).canonical().into_bytes()
    };
    let model = with("model_id", Value::String("0".repeat(64)));
    assert_eq!(refusal(c600::check(k, &model)), code::IDENTITY_MISMATCH);
    let format = with("format", Value::String("C600-STUDIO-SESSION-v2".to_owned()));
    assert_eq!(refusal(c600::check(k, &format)), code::PERSISTENCE_VERSION);
    let events = walk(&record, &["events"]).as_array().unwrap()[1].clone();
    let many = with("events", Value::Array(vec![events; 10_001]));
    assert_eq!(refusal(c600::check(k, &many)), code::PERSISTENCE_SIZE);
    let final_state = with("final_state", Value::String(k.solved().digest().to_owned()));
    assert_eq!(refusal(c600::check(k, &final_state)), code::CHECKPOINT_REPLAY);
    let mut bom = b"\xef\xbb\xbf".to_vec();
    bom.extend_from_slice(&fixture("c600-scramble.c600.json"));
    assert_eq!(
        c600::check(k, &bom).unwrap().state.digest(),
        text(expected(), &["c600", "c600-scramble.c600.json", "state"])
    );
    assert_eq!(refusal(c600::check(k, &vec![b' '; (16 << 20) + 1])), code::PERSISTENCE_SIZE);
    assert_eq!(refusal(c600::check(k, b"")), code::PERSISTENCE_SIZE);
    // Interchange keeps 0.4's caps on export too.
    assert_eq!((c600::MAX_TRANSACTIONS, c600::MAX_PRIMITIVES), (10_000, 2_000_000));
    let log = c600::import(k, &fixture("c600-scramble.c600.json")).unwrap();
    assert!(matches!(commits(&log)[0].interchange, Interchange::C600 { id: Some(ref id) } if id == "1"));
}

#[test]
fn the_description_is_mpults_block_for_600_cell_full() {
    let text = std::fs::read_to_string(repository().join("native/runtime/MPUlt_puzzles.txt")).unwrap();
    let text = text.strip_prefix('\u{feff}').unwrap_or(&text);
    let lines: Vec<&str> = text.lines().collect();
    let start = lines.iter().position(|l| *l == format!("Puzzle {}", mpult::NAME)).unwrap() + 1;
    let block: Vec<&str> = lines[start..]
        .iter()
        .take_while(|l| !l.starts_with("Puzzle "))
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect();
    assert_eq!(block, mpult::DESCRIPTION);
}

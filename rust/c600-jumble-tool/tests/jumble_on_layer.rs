//! Jumble on in the command layer, taken up by C8 (LP3 plan section 8; LP2's pull requests 212, 217, 218 and 220):
//! oracle equality against M5's W-J fixtures through `jumble.on` and `jumble.twist.menu` (PHS.23's "Oracle equality
//! at every committed step", its jumbling half), with the jumbling tool's own relation and `c600-display`'s posed row
//! read from the layer's head.
//!
//! For each fixture (I_a, I_b and S4), one session enters Jumble on at solved under the fixture's menu and presses
//! J1's journal records in order, one `jumble.twist.menu` press each. M5's engine replays the same records beside it
//! from solved (`State::apply`, flat, with no protected piece). At every step: the press commits; the state revision
//! advances by one; the revision's digest is the jumbling engine's digest of the head and M5's digest of its prefix;
//! the pieces whose pose changed are M5's inside set, and their count is J1's recorded `moved`. At each of J1's
//! `start`, `mid` and `end` stages the run reaches, the layer's head has J1's digest, its count of pieces off the
//! lattice, its pose-table size and its count of poses in K⁺, and `c600-display`'s posed row (J02) of the head has
//! J1's three array hashes. At the end of the run the head's posed row equals the one of M5's state, and the jumbling
//! tool's current-occupant relation (CT-EV3-06) read from the head answers every slot as it does on M5's state. The
//! layer replays its jumbling journal flat from solved each time the chain of derived pose moves reaches 64, so the
//! default run, the first 130 records of each journal, crosses two flat replays; the whole journals cross them many
//! times and run on request (`--ignored`, about twelve minutes in a dev build, since every press derives the head and
//! every flat replay derives the whole journal).
//!
//! LP2's `tests/test_jumbling_command_route.py` compares the same layer with J1 itself, step by step, on the first 70
//! records with undo and redo and the refusals; this test runs every record in Rust against the fixtures' recorded
//! references. `jumble.off`, the refusals, undo and redo, protected pieces and free mode run beside the tool's own
//! entries in `tests/jumble_layer_verdicts.rs`. A pass is source and fixture evidence.

mod common;

use c600_digest::sha256_hex;
use c600_display::jumble;
use c600_jumble::state::{State, Status};
use c600_jumble_tool::relation::current_occupants;
use c600_kernel::json::Value;

use common::{committed, engine, moved, Fixture, Session};

fn number(block: &Value, field: &str) -> u64 {
    block.get(field).and_then(Value::as_i64).unwrap_or_else(|| panic!("a recorded {field}")) as u64
}

/// The layer's head at a J1 stage against J1's references: digest, counts and the posed row's array hashes.
fn check_stage(fixture: &Fixture, stage: &str, session: &Session, steps: usize) {
    let engine = engine();
    let block = fixture.stage(stage);
    let what = format!("{} at stage {stage}", fixture.name);
    assert_eq!(number(block, "records") as usize, steps, "{what}: the stage's record count");
    let state = session.state();
    assert_eq!(session.layer.revision().digest, block.get("digest").and_then(Value::as_str).unwrap(), "{what}");
    let kplus = state.poses().iter().filter(|pose| pose.kidx().is_some()).count();
    let counts = (state.off_lattice_count() as u64, state.poses().len() as u64, kplus as u64);
    let recorded = (number(block, "off_lattice"), number(block, "poses"), number(block, "kplus_poses"));
    assert_eq!(counts, recorded, "{what}: off the lattice, pose-table entries and poses in K⁺");
    let posed = jumble::posed(engine, state).expect("finite poses");
    let arrays = block.get("arrays").expect("the stage's arrays");
    for (array, name) in
        [(&posed.pose_index, "pose_index.i32"), (&posed.poses, "poses.f32"), (&posed.pose_lattice, "pose_lattice.i32")]
    {
        let recorded = arrays.get(name).and_then(|a| a.get("sha256")).and_then(Value::as_str).expect("a hash");
        assert_eq!(sha256_hex(&array.bytes), recorded, "{what}: {name}");
    }
}

/// Every slot's current occupant read from the layer's head equals the one read from M5's own state.
fn check_occupants(fixture: &Fixture, session: &Session, oracle: &State) {
    let engine = engine();
    let slots: Vec<u32> = (0..engine.model.slot_piece.len() as u32).collect();
    let through = current_occupants(engine, session.state(), &slots).expect("no engine error").expect("model slots");
    let direct = current_occupants(engine, oracle, &slots).expect("no engine error").expect("model slots");
    assert_eq!(through, direct, "{}: the current-occupant relation", fixture.name);
}

/// The records each journal's default run presses: twice past the 64 chained pose moves after which the layer once
/// replayed the jumbling journal from solved, where it now keeps each move's flat claim flat. The whole journals take
/// about twelve minutes in a dev build, so they run on request.
const BOUNDED: usize = 130;

/// At the end of a run: J02's posed row and the current-occupant relation of the layer's head equal M5's.
fn check_head(fixture: &Fixture, session: &Session, oracle: &State) {
    let engine = engine();
    let (through, direct) = (jumble::posed(engine, session.state()), jumble::posed(engine, oracle));
    let (through, direct) = (through.expect("finite poses"), direct.expect("finite poses"));
    for (a, b, array) in [
        (&through.pose_index, &direct.pose_index, "pose_index"),
        (&through.poses, &direct.poses, "poses"),
        (&through.pose_lattice, &direct.pose_lattice, "pose_lattice"),
    ] {
        assert!(a.bytes == b.bytes, "{}: the posed row's {array} array", fixture.name);
    }
    check_occupants(fixture, session, oracle);
}

/// Presses the first `records` records of the journal `name` through the layer beside M5's engine.
fn oracle_equality(name: &'static str, records: Option<usize>) {
    let engine = engine();
    let fixture = Fixture::load(name);
    let journal = fixture.journal();
    let mid = number(fixture.stage("mid"), "records") as usize;
    let count = records.map_or(fixture.twists.len(), |records| records.min(fixture.twists.len()));
    let mut session = Session::entered(&fixture);
    let mut oracle = State::new(Some(std::sync::Arc::new(fixture.menu())));
    assert_eq!(session.layer.revision().digest, oracle.digest(engine), "{name}: the entry at solved");
    check_stage(&fixture, "start", &session, 0);

    for (index, twist) in fixture.twists[..count].iter().enumerate() {
        let what = format!("{name}, record {index}");
        let (before, state) = (session.state().clone(), session.layer.revision().state);
        let outcome = oracle.apply(engine, twist, None, &[]).expect("no engine error");
        assert_eq!(outcome.status, Status::Admissible, "{what}: J1 applied every record");
        let receipt = session.press(twist);
        assert!(committed(&receipt), "{what}: {:?}", receipt.status);
        let revision = session.layer.revision();
        assert_eq!(revision.state, state + 1, "{what}: one state revision per press");
        let head = session.layer.jumbled().expect("Jumble is on").digest(engine);
        assert_eq!((&revision.digest, &head), (&oracle.digest(engine), &oracle.digest(engine)), "{what}: digests");
        let pieces = moved(&before, session.state());
        assert_eq!(pieces, outcome.inside, "{what}: the moved set");
        assert_eq!(pieces.len() as u64, number(&journal[index], "moved"), "{what}: J1's moved count");
        if index + 1 == mid {
            check_stage(&fixture, "mid", &session, mid);
        }
    }
    if count == fixture.twists.len() {
        check_stage(&fixture, "end", &session, count);
    }
    check_head(&fixture, &session, &oracle);
}

/// Oracle equality through the command layer on the first 130 records of J1's I_a journal.
#[test]
fn jumble_on_through_the_command_layer_equals_m5_on_the_i_a_fixture_bounded() {
    oracle_equality("I_a", Some(BOUNDED));
}

/// Oracle equality through the command layer on the first 130 records of J1's I_b journal.
#[test]
fn jumble_on_through_the_command_layer_equals_m5_on_the_i_b_fixture_bounded() {
    oracle_equality("I_b", Some(BOUNDED));
}

/// Oracle equality through the command layer on the first 130 records of J1's S4 journal.
#[test]
fn jumble_on_through_the_command_layer_equals_m5_on_the_s4_fixture_bounded() {
    oracle_equality("S4", Some(BOUNDED));
}

/// Oracle equality through the command layer on J1's whole I_a journal (841 records), with J1's three stages.
#[test]
#[ignore = "about twelve minutes in a dev build; run with --ignored"]
fn jumble_on_through_the_command_layer_equals_m5_on_the_whole_i_a_fixture() {
    oracle_equality("I_a", None);
}

/// Oracle equality through the command layer on J1's whole I_b journal (885 records), with J1's three stages.
#[test]
#[ignore = "about twelve minutes in a dev build; run with --ignored"]
fn jumble_on_through_the_command_layer_equals_m5_on_the_whole_i_b_fixture() {
    oracle_equality("I_b", None);
}

/// Oracle equality through the command layer on J1's whole S4 journal (640 records), with J1's three stages.
#[test]
#[ignore = "about twelve minutes in a dev build; run with --ignored"]
fn jumble_on_through_the_command_layer_equals_m5_on_the_whole_s4_fixture() {
    oracle_equality("S4", None);
}

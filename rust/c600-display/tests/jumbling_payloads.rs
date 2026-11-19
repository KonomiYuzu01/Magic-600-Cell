//! CT-RPI-08's producers of the jumbling rows J01, J06, J07, J09, J11 and J12 (LP3 C8; RM.34, JI.41): J01's shrink
//! anchors against render/1's pin, J06's first-frame readback check, and the J07, J09, J11 and J12 payloads read from
//! M5's and M8's engine on the pinned I_a walk (`research/jumbling/fixtures/wj-I_a.json`). The rows stay unsupported in
//! the fixture report (RM.15): these are the payloads a backend's own rows would carry, each the engine's exact answer
//! bound to its state's pose digest. Every result is a source fixture outcome, not a backend run.

mod common;

use std::sync::{Arc, OnceLock};

use c600_display::array::{Array, Elem};
use c600_display::codes;
use c600_display::crosswalk::{NuObservation, ReleaseLog, RepairList};
use c600_display::jumble::{self, observations as obs, Missing};
use c600_display::pick::Partial;
use c600_jumble::corrections;
use c600_jumble::levels::{self, Base, Snapshot};
use c600_jumble::model::{NP, POLES};
use c600_jumble::nu;
use c600_jumble::observe::{self, cuts, Batch, Rows, Scope};
use c600_jumble::state::{State, Status};
use c600_jumble::twist::{Menu, Twist};
use c600_jumble::Context as Engine;
use c600_kernel::json::{self, Value};

use common::*;

/// render/1 section 2's pin: the SHA-256 of the 433 × 4 f32 little-endian shrink anchors.
const J01_PIN: &str = "0b6ead284d3f62719e3e6ec893d6c199d49698d246a59164bc362446e39faca6";

struct World {
    engine: Engine,
    menu: Arc<Menu>,
    records: Vec<Value>,
}

fn world() -> &'static World {
    static WORLD: OnceLock<World> = OnceLock::new();
    WORLD.get_or_init(|| {
        let engine = Engine::load(&assets()).expect("the jumbling model loads");
        let fixture = std::fs::read(assets().join("../research/jumbling/fixtures/wj-I_a.json")).expect("the fixture");
        let fixture = json::parse(&fixture).expect("the fixture is JSON");
        let menu = Menu::from_record(&engine, fixture.get("menu").expect("a menu")).expect("the pinned I_a menu");
        let records = fixture.get("journal").and_then(|j| j.get("records")).and_then(Value::as_array);
        World { engine, menu: Arc::new(menu), records: records.expect("the journal's records").to_vec() }
    })
}

impl World {
    fn solved(&self) -> State {
        State::new(Some(self.menu.clone()))
    }

    /// The twists of the walk's first `n` records.
    fn journal(&self, n: usize) -> Vec<Twist> {
        self.records[..n].iter().map(|r| Twist::from_record(&self.engine, r).expect("a menu record")).collect()
    }

    /// The state after the walk's first `n` records, every one admissible.
    fn after(&self, n: usize) -> State {
        let mut state = self.solved();
        for (i, twist) in self.journal(n).iter().enumerate() {
            let outcome = state.apply(&self.engine, twist, None, &[]).expect("no engine error");
            assert_eq!(outcome.status, Status::Admissible, "record {i}");
        }
        state
    }

    /// The letter (grip, item) of the pinned I_a menu.
    fn letter(&self, grip: usize, item: usize) -> ([i64; 2], Twist) {
        let twists = self.menu.for_cap(&self.engine, grip).expect("the menu's twists");
        ([grip as i64, item as i64], twists[item].clone())
    }

    /// The letter of record `n`: its grip and the item of its `I_a[k]` label.
    fn record_letter(&self, n: usize) -> ([i64; 2], Twist) {
        let record = &self.records[n];
        let grip = record.get("grip").and_then(Value::as_i64).expect("a grip") as usize;
        let label = record.get("params").and_then(|p| p.get("label")).and_then(Value::as_str).expect("a label");
        let item = label.strip_prefix("I_a[").and_then(|l| l.strip_suffix(']')).expect("an I_a label");
        self.letter(grip, item.parse().expect("an item index"))
    }
}

/// The state after three records and its rim map (complete seam batch, no classification budget), computed once.
struct Walk {
    state: State,
    rims: obs::RimMap,
}

const WHOLE: Batch = Batch { chunk: 1 << 12, budget: None, cancel_after: None };

fn walk() -> &'static Walk {
    static WALK: OnceLock<Walk> = OnceLock::new();
    WALK.get_or_init(|| {
        let w = world();
        let mut state = w.after(3);
        let rims = obs::rim_map(&w.engine, &mut state, &WHOLE, None).expect("a rim map");
        Walk { state, rims }
    })
}

/// A cap of the walk's state with this status (`admissible` with N_c > 0 for "misaligned").
fn cap_where(status: &str) -> u32 {
    let caps = &walk().rims.caps;
    let found = match status {
        "misaligned" => caps.iter().find(|cap| cap.rim.is_some_and(|rim| rim.n > 0)),
        _ => caps.iter().find(|cap| cap.status == status),
    };
    found.expect("the walk's state has such a cap").cap
}

fn int(value: &Value, key: &str) -> Option<i64> {
    value.get(key).and_then(Value::as_i64)
}

fn text(value: &Value, key: &str) -> Option<String> {
    value.get(key).and_then(Value::as_str).map(str::to_owned)
}

// ------------------------------------------------------------------------------------------------------------ J01

/// CT-RPI-08, J01 (render/1 section 2, SPEC section 3): the shrink anchors of the shipped mesh are the area-weighted
/// triangle centroids render/1 pins, as f32 [433, 4]; mesh offsets that do not cut the mesh into triangles, a sticker
/// of zero area and a mesh of the wrong shape refuse the assets with `EV3:input.display_bundle_mismatch` and no anchors.
#[test]
fn ct_rpi_08_j01_shrink_anchors_are_render_1s_pinned_centroids() {
    let anchors = jumble::shrink_anchors(geometry()).expect("the shipped mesh has anchors");
    assert_eq!((anchors.desc.elem, anchors.desc.shape.as_slice()), (Elem::F32, &[433, 4][..]));
    assert_eq!(anchors.desc.digest, J01_PIN, "render/1's pinned anchors");

    let refused = |input: &c600_display::produce::GeometryInput| {
        let error = jumble::shrink_anchors(input).expect_err("refused");
        assert_eq!((error.code, error.context["feature"].as_str()), (codes::BUNDLE_MISMATCH, "J01"), "{error:?}");
    };
    let offsets = geometry().offsets.as_u32();
    let mut cut = geometry().clone();
    let mut shifted = offsets.clone();
    shifted[1] += 1;
    cut.offsets = Array::u32s(vec![shifted.len() as u64], &shifted);
    refused(&cut);
    let mut flat = geometry().clone();
    let mut vertices = geometry().vertices.as_f32();
    let (begin, end) = (offsets[0] as usize, offsets[1] as usize);
    let first: Vec<f32> = vertices[begin * 4..begin * 4 + 4].to_vec();
    for k in begin..end {
        vertices[k * 4..k * 4 + 4].copy_from_slice(&first);
    }
    flat.vertices = Array::f32s(geometry().vertices.desc.shape.clone(), &vertices);
    refused(&flat);
    let mut wrong = geometry().clone();
    wrong.vertices = Array::f32s(vec![vertices.len() as u64], &vertices);
    refused(&wrong);
}

// ------------------------------------------------------------------------------------------------------------ J06

/// CT-RPI-08, J06 (render/1 section 7): the readback of a revision's three bound arrays passes only when each is read
/// back once, at the bundle's revision, with its element type, shape, digest and elements; any other readback names
/// the array that failed and nothing else. The check carries its revision, device and frame, and a check of another
/// revision or device is stale.
#[test]
fn ct_rpi_08_j06_readback_compares_every_bound_array() {
    let w = world();
    let state = w.after(1);
    let posed = jumble::posed(&w.engine, &state).expect("finite poses");
    let bundle = jumble::bundle(&jumble::revision(&w.engine, &state, 13), &posed);
    let check = jumble::readback_check(&bundle, &bundle.arrays, 2, 40);
    assert!(check.passed(), "{:?}", check.failed);
    assert_eq!((check.revision, check.device, check.frame), (13, 2, 40));
    assert!(check.current(13, 2) && !check.current(14, 2) && !check.current(13, 3), "another revision or device");

    let failed = |read: &[c600_display::validate::BoundArray]| jumble::readback_check(&bundle, read, 2, 40).failed;
    for (i, name) in ["pose_index", "poses", "pose_lattice"].into_iter().enumerate() {
        let mut flipped = bundle.arrays.clone();
        let mut bytes = (*flipped[i].array.bytes).clone();
        bytes[0] ^= 1;
        flipped[i].array = Array::from_bytes(flipped[i].array.desc.elem, flipped[i].array.desc.shape.clone(), bytes);
        assert_eq!(failed(&flipped), [name], "one element of {name} differs");
        let mut forged = bundle.arrays.clone();
        let mut bytes = (*forged[i].array.bytes).clone();
        bytes[0] ^= 1;
        forged[i].array.bytes = Arc::new(bytes);
        assert_eq!(failed(&forged), [name], "{name}'s descriptor claims the engine's digest over other bytes");
        let mut old = bundle.arrays.clone();
        old[i].revision = 12;
        assert_eq!(failed(&old), [name], "{name} of the previous revision");
        let missing: Vec<_> = bundle.arrays.iter().filter(|a| a.name != name).cloned().collect();
        assert_eq!(failed(&missing), [name], "{name} not read back");
        let mut twice = bundle.arrays.clone();
        twice.push(bundle.arrays[i].clone());
        assert_eq!(failed(&twice), [name], "{name} read back twice");
    }
    let mut reshaped = bundle.arrays.clone();
    let poses = &bundle.arrays[1].array;
    reshaped[1].array = Array::from_bytes(Elem::F32, vec![poses.desc.byte_len / 4], (*poses.bytes).clone());
    assert_eq!(failed(&reshaped), ["poses"], "the same bytes in another shape");
}

// ------------------------------------------------------------------------------------------------------------ J07

/// CT-RPI-08, J07 (JI.41 "Domain and lattice masks"): one bit per piece. The lattice mask holds exactly the pieces whose
/// pose the engine places in K⁺ (J02's lattice entries), the domains partition the pieces in domain-ID order, every
/// member of an engine unit is in its unit's domain, and the lattice domain holds every lattice piece. The solved state
/// has one domain with every piece.
#[test]
fn ct_rpi_08_j07_masks_follow_the_engines_exact_domains() {
    let w = world();
    let state = &walk().state;
    let masks = obs::domain_masks(&w.engine, state).expect("complete units");
    assert_eq!(masks.input, state.digest(&w.engine));
    assert_eq!(masks.lattice.words().len(), NP.div_ceil(64));
    assert_eq!(masks.lattice.words().last().copied().unwrap_or(0) >> (NP % 64), 0, "no bit past the last piece");
    let posed = jumble::posed(&w.engine, state).expect("finite poses");
    let (index, lattice) = (posed.pose_index.as_i32(), posed.pose_lattice.as_i32());
    for (piece, &pose) in (0u32..).zip(&index) {
        assert_eq!(masks.lattice.contains(piece), lattice[pose as usize] >= 0, "piece {piece}");
    }
    assert_eq!(masks.lattice.count(), (NP - state.off_lattice_count()) as u64);
    assert!(state.off_lattice_count() > 0, "the walk's state is off the lattice");

    let ids: Vec<&str> = masks.domains.iter().map(|(id, _)| id.as_str()).collect();
    assert!(ids.windows(2).all(|w| w[0] < w[1]), "domain-ID order, no repeats");
    assert!(masks.domains.iter().all(|(_, mask)| mask.count() > 0), "only domains that hold a piece");
    let total: u64 = masks.domains.iter().map(|(_, mask)| mask.count()).sum();
    let union = masks.domains.iter().fold(vec![0u64; NP.div_ceil(64)], |mut union, (_, mask)| {
        union.iter_mut().zip(mask.words()).for_each(|(u, &w)| *u |= w);
        union
    });
    let covered: u64 = union.iter().map(|w| u64::from(w.count_ones())).sum();
    assert_eq!((total, covered), (NP as u64, NP as u64), "every piece in exactly one domain");
    let (_, home) = masks.domains.iter().find(|(id, _)| *id == masks.lattice_domain).expect("the lattice domain");
    assert!(home.words().iter().zip(masks.lattice.words()).all(|(&d, &l)| d & l == l), "lattice pieces are home");
    let units = observe::units(&w.engine, state, None).expect("no engine error");
    for unit in units.get("units").and_then(Value::as_array).expect("units") {
        let domain = text(unit, "domain").expect("a domain");
        let (_, mask) = masks.domains.iter().find(|(id, _)| *id == domain).expect("the unit's domain");
        for member in unit.get("members").and_then(Value::as_array).expect("members") {
            assert!(mask.contains(member.as_i64().expect("a piece") as u32), "{member:?} in domain {domain}");
        }
    }

    let solved = obs::domain_masks(&w.engine, &w.solved()).expect("complete units");
    assert_eq!(solved.domains.len(), 1);
    assert_eq!((solved.domains[0].0.as_str(), solved.lattice.count()), (solved.lattice_domain.as_str(), NP as u64));
}

/// CT-RPI-08, J07 (JI.41 "Rim map and counts"): the rim map copies `rim_summary` for all 600 caps in cap-ID order (an
/// admissible cap's counts; no counts for a blocked cap, with the engine's reason); `grip_n` is the sum of N_c, total N
/// is the complete seam batch's, and X5's counts are the engine's. With no exact evaluation left, a cap is uncertain
/// and `grip_n` is undefined, never a sum over fewer caps; a cut-off seam batch leaves total N undefined.
#[test]
fn ct_rpi_08_j07_rim_map_copies_every_cap_and_its_counts() {
    let w = world();
    let walk = walk();
    let map = &walk.rims;
    let mut state = walk.state.clone();
    assert_eq!(map.input, state.digest(&w.engine));
    let summary = cuts::rim_summary(&w.engine, &mut state, None, None).expect("no engine error");
    let rows = summary.get("rows").and_then(Value::as_array).expect("rows");
    assert_eq!((map.caps.len(), rows.len()), (POLES, POLES));
    let mut sum = 0;
    for (cap, row) in map.caps.iter().zip(rows) {
        assert_eq!((Some(i64::from(cap.cap)), Some(cap.status.clone())), (int(row, "cap"), text(row, "status")));
        assert_eq!(cap.undefined, text(row, "undefined"), "cap {}", cap.cap);
        match cap.rim {
            Some(rim) => {
                let counts = [rim.n, rim.inside, rim.outside, rim.aligned, rim.floor];
                let engine = ["N_c", "In_c", "Out_c", "A_c", "F_c"].map(|key| int(row, key).expect("a count"));
                assert_eq!(counts, engine, "cap {}", cap.cap);
                assert_eq!(Some(&Value::Bool(rim.at_floor)), row.get("at_floor"));
                assert_eq!((cap.blocked, cap.undefined.as_deref()), (Some(false), None));
                sum += rim.n;
            }
            None => assert_eq!(row.get("N_c"), Some(&Value::Null), "cap {}", cap.cap),
        }
    }
    assert!(map.caps.iter().any(|cap| cap.status == "blocked" && cap.rim.is_none() && cap.blocked == Some(true)));
    assert_eq!(map.grip_n, Partial::Total(sum));
    assert_eq!(Some(sum), int(&summary, "grip_n"));
    let facts = observe::seam_facts(&w.engine, &mut state, Scope::All, &WHOLE, Rows::Mismatched, None).expect("facts");
    let total = facts.get("totals").and_then(|t| int(t, "total_N")).expect("a complete batch has totals");
    assert_eq!(map.total_n, Partial::Total(total as u64));
    let heights = observe::heights(&w.engine, &state).expect("no engine error");
    let counts = heights.get("height_counts").expect("height counts");
    for key in ["n_0", "n_2", "n_4", "n_6", "n_above_6"] {
        assert_eq!(Some(map.heights.bin(key)), int(counts, key), "{key}");
    }
    assert_eq!(map.heights.max_above_6, int(counts, "max_above_6"));

    let mut state = walk.state.clone();
    let cut = Batch { chunk: 1, budget: Some(1), cancel_after: None };
    let short = obs::rim_map(&w.engine, &mut state, &cut, Some(0)).expect("a rim map");
    assert_eq!(short.grip_n, Partial::Undefined("uncertain_caps".to_owned()));
    assert!(matches!(short.total_n, Partial::Undefined(_)), "no total N from a cut-off batch");
    let uncertain = short.caps.iter().find(|cap| cap.status == "uncertain").expect("an uncertain cap");
    assert_eq!((uncertain.blocked, uncertain.rim), (None, None));
    assert_eq!(uncertain.undefined.as_deref(), Some("uncertain_under_budget"));
}

// ------------------------------------------------------------------------------------------------------------ J09

/// CT-RPI-08, J09 (JI.41 "Covers and repair rows"): a completed cover table at a misaligned cap is every meeting cap in
/// cap-ID order with the engine's answers copied; a blocked cap, a closed cap, a cap outside the model and a zero
/// classification budget give no table.
#[test]
fn ct_rpi_08_j09_cover_tables_are_complete_or_missing() {
    let w = world();
    let mut state = walk().state.clone();
    let c = cap_where("misaligned");
    let covers = obs::covers(&w.engine, &mut state, c, None).expect("a completed table");
    let answer = cuts::covers(&w.engine, &mut state, i64::from(c), None).expect("no engine error");
    assert_eq!(covers.input, state.digest(&w.engine));
    assert_eq!(covers.table.cap, c);
    let domain: Vec<u32> = answer
        .get("domain")
        .and_then(Value::as_array)
        .expect("a domain")
        .iter()
        .map(|d| d.as_i64().unwrap() as u32)
        .collect();
    assert_eq!((covers.table.rows(), covers.table.rows().len()), (domain.as_slice(), 80));
    assert_eq!(Some(&covers.answers), answer.get("answers").and_then(Value::as_array).map(|a| a.to_vec()).as_ref());

    let incomplete = |status: &str| Missing::Incomplete { row: "J09", status: status.to_owned() };
    let blocked = cap_where("blocked");
    assert_eq!(obs::covers(&w.engine, &mut state, blocked, None).err(), Some(incomplete("error")));
    assert_eq!(obs::covers(&w.engine, &mut state, c, Some(0)).err(), Some(incomplete("cut_off")));
    assert_eq!(obs::covers(&w.engine, &mut state, 600, None).err(), Some(incomplete("refused")));
    assert_eq!(obs::covers(&w.engine, &mut w.solved(), 7, None).err(), Some(incomplete("error")), "a closed cap");
}

/// CT-RPI-08, J09: a completed repair list keeps the engine's rows (x, d, y, z, ΔN) in its order, each ΔN negative, and
/// supports only the negative scoped to its cap and input; a pair budget or a cancellation gives a cut-off list with
/// no rows and no negative; a blocked cap and free mode give no list.
#[test]
fn ct_rpi_08_j09_repair_lists_are_completed_or_cut_off() {
    let w = world();
    let state = walk().state.clone();
    let digest = state.digest(&w.engine);
    let c = cap_where("misaligned");
    let full = obs::repair_list(&w.engine, &state, c, None, None, None).expect("a list");
    let answer = cuts::repair_list(&w.engine, &state, i64::from(c), None, None, None).expect("no engine error");
    let engine: Vec<Vec<i64>> = answer
        .get("rows")
        .and_then(Value::as_array)
        .expect("a completed list has rows")
        .iter()
        .map(|row| row.as_array().expect("a row").iter().map(|v| v.as_i64().expect("an integer")).collect())
        .collect();
    let rows: Vec<Vec<i64>> =
        full.rows.iter().map(|r| vec![r.x.into(), r.d.into(), r.y.into(), r.z.into(), r.delta]).collect();
    assert_eq!(rows, engine);
    let RepairList::Completed { cap, input, repairs } = &full.list else { panic!("completed: {:?}", full.list) };
    assert_eq!((*cap, input.as_str()), (c, digest.as_str()));
    assert_eq!(repairs, &full.rows.iter().map(|r| (r.x, r.d)).collect::<Vec<_>>());
    assert_eq!(Some(full.pairs_tried as i64), answer.get("coverage").and_then(|c| int(c, "pairs_tried")));
    assert_eq!(full.list.negative().is_some(), repairs.is_empty(), "a negative only for a completed empty list");

    for (max_pairs, cancel_after, tried) in [(Some(1), None, 1), (Some(0), None, 0), (None, Some(0), 0)] {
        let stopped = obs::repair_list(&w.engine, &state, c, max_pairs, cancel_after, None).expect("a list");
        assert_eq!(stopped.list, RepairList::CutOff { cap: c, input: digest.clone() });
        assert_eq!((stopped.rows.len(), stopped.pairs_tried, stopped.list.negative()), (0, tried, None));
    }
    let incomplete = |status: &str| Some(Missing::Incomplete { row: "J09", status: status.to_owned() });
    let blocked = cap_where("blocked");
    assert_eq!(obs::repair_list(&w.engine, &state, blocked, None, None, None).err(), incomplete("error"));
    assert_eq!(obs::repair_list(&w.engine, &State::new(None), 7, None, None, None).err(), incomplete("error"));
}

// ------------------------------------------------------------------------------------------------------------ J11

/// CT-RPI-08, J11 (JI.41 "Release/word check data"): a word check copies the engine's answer bound to its start and
/// word digests; a complete word ends at the state the letters reach, and without a named unit the step flag is
/// undefined. A refused letter ends the check: the suffix stays unchecked and the end state undefined. A release check
/// is the one-letter check. Checks are kept in tried order at their revision, and a new revision clears them.
#[test]
fn ct_rpi_08_j11_word_checks_and_their_release_log() {
    let w = world();
    let solved = w.solved();
    let word = [w.record_letter(0), w.record_letter(1)];
    let checked = obs::word_check(&w.engine, &solved, &word, None, None, None).expect("an observation");
    let answer = observe::word_check(&w.engine, &solved, &word, None, None, None).expect("no engine error");
    assert_eq!(checked.answer, answer);
    assert_eq!(
        (checked.start.clone(), Some(checked.word.clone())),
        (solved.digest(&w.engine), text(&answer, "word_digest"))
    );
    assert_eq!((checked.letters, checked.processed, checked.refused.clone()), (2, 2, None));
    assert_eq!(checked.end, Partial::Total(w.after(2).digest(&w.engine)));
    assert!(matches!(checked.step, Partial::Undefined(_)), "no unit, no step flag: {:?}", checked.step);

    let state = &walk().state;
    let blocked = cap_where("blocked") as usize;
    let word = [w.letter(blocked, 1), w.record_letter(3)];
    let refused = obs::word_check(&w.engine, state, &word, None, None, None).expect("an observation");
    let (index, status) = refused.refused.clone().expect("the first letter is refused");
    assert_eq!((index, refused.processed, refused.letters), (0, 1, 2), "the suffix stays unchecked");
    assert_eq!(Some(status), text(&refused.answer, "failing_status"));
    assert!(matches!(refused.end, Partial::Undefined(_)), "no end state after a refusal");

    let letter = w.record_letter(3);
    let release = obs::release_check(&w.engine, state, &letter, None, None).expect("an observation");
    let answer = cuts::release_check(&w.engine, state, &letter, None, None).expect("no engine error");
    assert_eq!(release.answer, answer);
    assert_eq!((release.letters, release.processed, release.refused.clone()), (1, 1, None));
    assert_eq!(release.end, Partial::Total(w.after(4).digest(&w.engine)));

    let mut log = ReleaseLog::new(7);
    assert!(checked.keep(&mut log, 7) && release.keep(&mut log, 7));
    assert_eq!(log.entries(), [checked.word.clone(), release.word.clone()], "tried order");
    assert!(!checked.keep(&mut log, 6), "a check of another revision is not kept");
    log.set_revision(8);
    assert!(log.entries().is_empty(), "a new revision clears the old checks");
    let outside = obs::word_check(&w.engine, &solved, &word, None, Some(600), None).err();
    assert_eq!(outside, Some(Missing::Incomplete { row: "J11", status: "refused".to_owned() }), "a cap outside");
}

// ------------------------------------------------------------------------------------------------------------ J12

/// The state correction `id` reaches from solved under I_a, played literally, witnessed by its word.
fn played(id: u32) -> Snapshot {
    let w = world();
    let catalogue = corrections::catalogue().expect("the catalogue reads");
    let word = catalogue.twists(&w.engine, &w.menu, id).expect("no engine error").expect("the word reads");
    let mut state = w.solved();
    for twist in &word {
        let outcome = state.apply(&w.engine, twist, None, &[]).expect("no engine error");
        assert_eq!(outcome.status, Status::Admissible, "correction {id}");
    }
    Snapshot::witnessed(&w.engine, state, &word, None).expect("no engine error").expect("the word witnesses it")
}

/// The engine's `correction_set_check` of `set` on `base`, as values.
fn engine_check(base: Base, set: &[(u32, i64)]) -> Partial<Vec<i64>> {
    let checked = corrections::correction_set_check(&world().engine, base, set).expect("no engine error");
    match checked.expect("not refused") {
        levels::Partial::Total(value) => Partial::Total(value.values().to_vec()),
        levels::Partial::Undefined(reason) => Partial::Undefined(reason),
        other => panic!("a partial ν: {other:?}"),
    }
}

/// CT-RPI-08, J12 (JI.41 "Nu and correction-set observations"): the catalogue is the shipped one, all 36 rows in ID
/// order over ν's moduli. ν of a lattice state is the engine's value, with I_a's fixed coordinates named only under a
/// claim witnessed at that state, and the predicted ν of a drafted set equals the engine's `correction_set_check` on
/// the same base. Off the lattice both are `undefined(off_lattice)`, never zeros or an older value.
#[test]
fn ct_rpi_08_j12_nu_and_the_catalogue_agree_with_the_engine() {
    let w = world();
    let catalogue = obs::catalogue().expect("the shipped catalogue");
    let shipped = corrections::catalogue().expect("the catalogue reads");
    assert_eq!((catalogue.rows().len(), catalogue.moduli(), catalogue.modulus), (36, &nu::MODULI[..], 10));
    for (row, correction) in catalogue.rows().iter().zip(shipped.rows()) {
        assert_eq!((row.id, row.effect.as_slice()), (correction.id(), &correction.row().values()[..]));
    }

    let mut multiplicities = vec![0i64; 36];
    let set = [(3u32, 2i64), (17, -1), (35, 4)];
    for &(id, m) in &set {
        multiplicities[id as usize] = m;
    }
    let solved = Snapshot::solved(&w.engine, Some(w.menu.clone()));
    let observation = obs::nu_observation(&w.engine, Base::Snapshot(&solved)).expect("ν");
    assert_eq!(observation, NuObservation::Lattice { nu: vec![0; 45], fixed: nu::I_A_FIXED.to_vec() });
    let free = Snapshot::solved(&w.engine, None);
    let unbound = obs::nu_observation(&w.engine, Base::Snapshot(&free)).expect("ν");
    assert_eq!(unbound, NuObservation::Lattice { nu: vec![0; 45], fixed: Vec::new() }, "free mode binds nothing");

    let snapshot = played(0);
    let observation = obs::nu_observation(&w.engine, Base::Snapshot(&snapshot)).expect("ν");
    let NuObservation::Lattice { nu: value, fixed } = &observation else { panic!("on the lattice: {observation:?}") };
    assert_eq!((value.as_slice(), fixed.as_slice()), (&shipped.rows()[0].row().values()[..], &nu::I_A_FIXED[..]));
    assert!(value.iter().any(|&v| v != 0), "correction 0 changes ν");
    let predicted = obs::predicted(&observation, &catalogue, &multiplicities).expect("not refused");
    assert_eq!(predicted, engine_check(Base::Snapshot(&snapshot), &set));
    let mut back = vec![0i64; 36];
    back[0] = -1;
    let predicted = obs::predicted(&observation, &catalogue, &back).expect("not refused");
    assert_eq!(predicted, Partial::Total(vec![0; 45]), "taking correction 0 back predicts 0");

    let mut off = w.solved();
    let journal = w.journal(1);
    off.apply(&w.engine, &journal[0], None, &[]).expect("no engine error");
    let snapshot = Snapshot::witnessed(&w.engine, off, &journal, None).expect("no engine error").expect("witnessed");
    let observation = obs::nu_observation(&w.engine, Base::Snapshot(&snapshot)).expect("ν");
    assert_eq!(observation, NuObservation::OffLattice);
    let predicted = obs::predicted(&observation, &catalogue, &multiplicities).expect("not refused");
    assert_eq!(predicted, Partial::Undefined(nu::OFF_LATTICE.to_owned()));
    assert_eq!(engine_check(Base::Snapshot(&snapshot), &set), predicted);
}

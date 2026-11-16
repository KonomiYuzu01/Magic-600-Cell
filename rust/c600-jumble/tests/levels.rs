//! LP2, M8: the level crossings, end tests, shapes and gauges of the jumbling puzzle (`c600_jumble::levels`; chapter
//! 01, KN1.7 to KN1.10, KN1.14 and KN1.17; state contract A3 and A5). `tests/test_jumbling_rust_m8_levels.py` runs
//! the same calls through the adapter against J1. Nothing here chooses a twist: every word below is fixed in the test.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use c600_jumble::kplus::ORDER;
use c600_jumble::levels::{self, Gauge, GaugeClass, RegionKey, Shape};
use c600_jumble::levels::{Level, LevelValue, Partial, Projection, ReachKind, Reachability, Verdict, Witness};
use c600_jumble::model::{NP, NS};
use c600_jumble::q5::{matmul, Mat4, Q5};
use c600_jumble::state::{Pose, State, Status};
use c600_jumble::twist::{Menu, Twist};
use c600_jumble::{observe, Context};
use c600_kernel::json::{self, Value};

fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..")
}

struct Setup {
    ctx: Context,
    menu: Arc<Menu>,
}

/// The model and the pinned I_a menu, loaded once for every test of this file.
fn setup() -> &'static Setup {
    static SETUP: OnceLock<Setup> = OnceLock::new();
    SETUP.get_or_init(|| {
        let ctx = Context::load(&repository().join("assets")).expect("the jumbling model loads");
        let fixture = std::fs::read(repository().join("research/jumbling/fixtures/wj-I_a.json")).expect("the fixture");
        let fixture = json::parse(&fixture).expect("the I_a fixture is JSON");
        let menu = Menu::from_record(&ctx, fixture.get("menu").expect("a menu")).expect("the pinned I_a menu");
        Setup { ctx, menu: Arc::new(menu) }
    })
}

/// Free mode, one exact Cayley rotation ω = (2, 0, 0) at grip 0 from solved: Q of CT-KN1-06, off the lattice.
fn cayley_state(ctx: &Context) -> State {
    let omega = [Q5::int(2), Q5::zero(), Q5::zero()];
    let twist = Twist::from_record(ctx, &observe::cayley_record(ctx, 0, &omega)).expect("the Cayley record reads");
    let mut state = State::new(None);
    let outcome = state.apply(ctx, &twist, None, &[]).expect("no engine error");
    assert_eq!(outcome.status, Status::Admissible);
    state
}

/// A fixed retained word from solved under the I_a menu, with its twists.
fn retained_state(s: &Setup, word: &[i64]) -> (State, Vec<Twist>) {
    let twists: Vec<Twist> = word.iter().map(|&mid| levels::primitive(&s.ctx, mid).expect("a primitive id")).collect();
    let mut state = State::new(Some(s.menu.clone()));
    for twist in &twists {
        let outcome = state.apply(&s.ctx, twist, None, &[]).expect("no engine error");
        assert_eq!(outcome.status, Status::Admissible);
    }
    (state, twists)
}

fn fact(verdict: &Verdict) -> Option<&str> {
    match verdict {
        Verdict::No(evidence) => evidence.get("fact").and_then(Value::as_str),
        _ => None,
    }
}

fn evidence_text<'a>(verdict: &'a Verdict, key: &str) -> Option<&'a str> {
    match verdict {
        Verdict::Yes(evidence) | Verdict::No(evidence) => evidence.get(key).and_then(Value::as_str),
        Verdict::Unknown(_) => None,
    }
}

fn stated(kind: ReachKind) -> Reachability {
    Reachability::stated(kind, Value::Null).expect("a claim that is not witnessed can be stated")
}

/// CT-KN1-03 (whole: a projection of a state off the lattice to a level defined on lattice configurations only, labels
/// or position, is a typed partial result whose missing part counts the pieces off, and every other level the state
/// holds is total; tests/test_jumbling_rust_m8_levels.py checks the off counts and the covered chambers against J1).
#[test]
fn ct_kn1_03_a_projection_off_the_lattice_is_partial_with_the_off_count() {
    let ctx = &setup().ctx;
    let state = cayley_state(ctx);
    let off = state.off_lattice_count();
    assert!(off > 0 && off < NP, "the Cayley rotation takes some pieces off the lattice: {off}");
    let on_lattice = |piece: usize| state.pose_of(piece).kidx().is_some();
    for level in Level::ALL {
        assert_eq!(level.lattice_only(), matches!(level, Level::Labels | Level::Position));
        match (level, levels::project(ctx, &state, level).expect("no engine error")) {
            (Level::StickerClass, Projection::Refused(refusal)) => assert_eq!(refusal.code, levels::LEVEL_NOT_HELD),
            (Level::Labels, Projection::Value(Partial::Partial { covered: LevelValue::Labels(labels), missing })) => {
                assert_eq!((missing.off, missing.collisions), (off, 0));
                assert_eq!(labels.slots.len() + missing.uncovered, NS);
                // Each covered slot shows a sticker of a lattice piece, and no sticker is shown twice.
                let mut shown = labels.labels.clone();
                shown.sort_unstable();
                shown.dedup();
                assert_eq!(shown.len(), labels.labels.len());
                assert!(labels.labels.iter().all(|&l| on_lattice(ctx.model.slot_piece[l as usize] as usize)));
            }
            (Level::Position, Projection::Value(Partial::Partial { covered: LevelValue::Position(p), missing })) => {
                assert_eq!((missing.off, missing.collisions, missing.uncovered), (off, 0, off));
                assert_eq!(p.pieces.len(), NP - off);
                assert!(p.pieces.iter().all(|&piece| on_lattice(piece as usize)));
            }
            (Level::RegionClass, Projection::Value(Partial::Total(LevelValue::RegionClass(keys)))) => {
                assert_eq!(keys.len(), NP);
                assert_eq!(keys.iter().filter(|k| !k.is_chamber()).count(), off);
            }
            (Level::Shape, Projection::Value(Partial::Total(LevelValue::Shape(shape)))) => {
                assert_eq!((shape.off_regions, shape.off_poses, shape.collisions), (off, off, 0));
                assert_eq!(shape.regions.iter().map(|(_, n)| *n as usize).sum::<usize>(), NP);
            }
            (Level::Poses, Projection::Value(Partial::Total(LevelValue::Poses { digest }))) => {
                assert_eq!(digest, state.digest(ctx))
            }
            (Level::Coset, Projection::Value(Partial::Total(LevelValue::Coset(domains)))) => {
                assert_eq!(domains.len(), NP)
            }
            (Level::TreeData, Projection::Value(Partial::Total(LevelValue::TreeData(rows)))) => {
                assert_eq!(rows.len(), NP)
            }
            (level, other) => panic!("{}: {other:?}", level.name()),
        }
    }
}

/// At solved every level the state holds is total, π is the identity and every piece is in its own chamber.
#[test]
fn solved_projects_totally() {
    let ctx = &setup().ctx;
    let state = State::new(None);
    match levels::labels(ctx, &state).expect("no engine error") {
        Partial::Total(labels) => {
            assert_eq!(labels.displaced(), 0);
            assert_eq!(labels.labels.len(), NS);
        }
        other => panic!("{other:?}"),
    }
    match levels::positions(ctx, &state).expect("no engine error") {
        Partial::Total(p) => assert!(p.pieces.iter().zip(&p.chambers).all(|(a, b)| a == b) && p.pieces.len() == NP),
        other => panic!("{other:?}"),
    }
    let shape = levels::shape(ctx, &state).expect("no engine error");
    assert_eq!((shape.regions.len(), shape.off_regions, shape.collisions), (NP, 0, 0));
    assert!(matches!(
        levels::project(ctx, &state, Level::StickerClass).expect("no engine error"),
        Projection::Refused(refusal) if refusal.code == levels::LEVEL_NOT_HELD
    ));
}

/// The text up to the first `{` or `;` of every `pub fn` in the crate's sources, as (file, name, inputs, output) with
/// whitespace collapsed.
fn public_functions() -> Vec<(String, String, String, String)> {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let (mut files, mut dirs) = (Vec::new(), vec![src.clone()]);
    while let Some(dir) = dirs.pop() {
        for entry in std::fs::read_dir(&dir).expect("src/ reads") {
            let path = entry.expect("a directory entry").path();
            if path.is_dir() {
                dirs.push(path);
            } else if path.extension().is_some_and(|e| e == "rs") {
                files.push(path);
            }
        }
    }
    files.sort();
    let collapse = |text: &str| text.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut out = Vec::new();
    for path in files {
        let text = std::fs::read_to_string(&path).expect("a source file reads");
        let file = path.strip_prefix(&src).expect("under src/").to_string_lossy().replace('\\', "/");
        let mut rest = text.as_str();
        while let Some(at) = rest.find("pub fn ") {
            rest = &rest[at + "pub fn ".len()..];
            let name: String = rest.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
            let open = rest.find('(').expect("a parameter list");
            let mut depth = 0usize;
            let mut close = open;
            for (i, c) in rest[open..].char_indices() {
                match c {
                    '(' => depth += 1,
                    ')' => {
                        depth -= 1;
                        if depth == 0 {
                            close = open + i;
                            break;
                        }
                    }
                    _ => {}
                }
            }
            let tail = &rest[close + 1..];
            let end = tail.find(|c| c == '{' || c == ';').unwrap_or(tail.len());
            out.push((file.clone(), name, collapse(&rest[open + 1..close]), collapse(&tail[..end])));
        }
    }
    out
}

fn identifiers(text: &str) -> Vec<String> {
    text.split(|c: char| !(c.is_alphanumeric() || c == '_')).filter(|t| !t.is_empty()).map(str::to_owned).collect()
}

/// CT-KN1-04 (part: the jumbling crate's public functions and KN1.7's maps; not run: the 600-cell-Full half, which is
/// rust/c600-kernel/tests/kernel.rs's part). Every map of KN1.7 goes one way, from poses down; and no public function
/// of `c600-jumble` that returns a state or a pose reads a value of a coarser level: a state comes from solved, from a
/// pose table, from poses or from twists, never from labels, positions, regions, shapes, cosets or tree data.
#[test]
fn ct_kn1_04_no_public_call_lifts_a_coarser_level() {
    // A new level makes this match fail to compile, so that the maps and the names below are read again.
    let coarser = |level: Level| -> &'static [&'static str] {
        match level {
            Level::Poses => &[],
            Level::StickerClass => &["sticker", "stickers"],
            Level::Labels => &["Labels", "labels", "label"],
            Level::RegionClass => &["RegionKey", "region", "regions"],
            Level::Position => &["Positions", "positions", "position", "chamber", "chambers"],
            Level::Coset => &["coset", "domain", "domains"],
            Level::TreeData => &["Facts", "facts", "heights"],
            Level::Shape => &["Shape", "shape"],
        }
    };
    for level in Level::ALL {
        assert!(Level::Poses.reaches(level), "poses reach {}", level.name());
        assert_eq!(level == Level::Poses, level.reaches(Level::Poses), "{} reaches poses", level.name());
        for &to in level.maps() {
            assert_ne!(to, level);
            assert!(!to.reaches(level), "{} and {} map both ways", level.name(), to.name());
        }
    }
    let mut names: Vec<&str> = Level::ALL.iter().flat_map(|&level| coarser(level).iter().copied()).collect();
    names.extend(["LevelValue", "Partial", "Projection", "Missing", "Level"]);

    let functions = public_functions();
    assert!(functions.len() >= 100, "the enumeration found {} public functions", functions.len());
    let mut builders = Vec::new();
    for (file, name, inputs, output) in &functions {
        let out = identifiers(output);
        if !out.iter().any(|t| t == "State" || t == "Pose") {
            continue;
        }
        let read = identifiers(inputs);
        let lifted: Vec<&str> = names.iter().copied().filter(|n| read.iter().any(|t| t == n)).collect();
        assert!(lifted.is_empty(), "{file}: {name}({inputs}) {output} reads {lifted:?}");
        builders.push(format!("{file}:{name}"));
    }
    for expected in ["state.rs:new", "state.rs:from_record", "levels.rs:state_of", "levels.rs:replay"] {
        assert!(builders.iter().any(|b| b == expected), "{expected} is enumerated: {builders:?}");
    }
}

/// KN1.17 and A5: a centre piece turned by a third-turn of A4₀ passes the sticker test, which claims no reachability;
/// a retained checkpoint and a label checkpoint never answer yes for it.
#[test]
fn a_turned_centre_passes_the_sticker_test_and_no_checkpoint() {
    let s = setup();
    let ctx = &s.ctx;
    let kp = &ctx.kplus;
    let third = kp.a4(0).into_iter().find(|&k| k != 0 && kp.compose(k, kp.compose(k, k)) == 0).expect("a third-turn");
    let centre = *ctx.model.centre_pieces.iter().find(|&&p| ctx.model.signature(p as usize) == [0]).expect("a centre");
    let mut poses = vec![Pose::K(0); NP];
    poses[centre as usize] = Pose::K(third);
    let state = levels::state_of(ctx, Some(s.menu.clone()), &poses).expect("the raw table reads");
    assert_ne!(state.digest(ctx), State::new(None).digest(ctx), "the turned centre is not solved at pose level");

    let sticker = levels::sticker_test(ctx, &state).expect("no engine error");
    assert!(matches!(sticker, Verdict::Yes(_)), "{sticker:?}");
    for witness in [Witness::Word(&[]), Witness::Journal(&[])] {
        let verdict = levels::retained_checkpoint(ctx, &state, witness).expect("no engine error");
        assert!(matches!(verdict, Verdict::Unknown(_)), "{verdict:?}");
    }
    assert!(levels::witness(ctx, &state, Some(s.menu.clone()), &[], None).expect("no engine error").is_err());
    for kind in [ReachKind::Open, ReachKind::Promised, ReachKind::ByTheorem, ReachKind::Refuted] {
        let verdict = levels::label_checkpoint(ctx, &state, &stated(kind), None).expect("no engine error");
        assert!(matches!(verdict, Verdict::Unknown(_)), "{}: {verdict:?}", kind.name());
    }
}

/// A5's end tests on a retained word: the sticker test answers no, the retained checkpoint replays the supplied word or
/// the journal's reduction and answers yes, and the label checkpoint answers yes only under the journal's witnessed
/// claim, with ν = 0 from the retained witness; open and promised claims answer unknown.
#[test]
fn a_retained_word_is_a_checkpoint_under_its_witness_only() {
    let s = setup();
    let ctx = &s.ctx;
    let word = [1, 3, -5];
    let (state, twists) = retained_state(s, &word);
    let digest = state.digest(ctx);

    assert_eq!(fact(&levels::sticker_test(ctx, &state).expect("no engine error")), Some("labels_differ"));
    let supplied = levels::retained_checkpoint(ctx, &state, Witness::Word(&word)).expect("no engine error");
    assert_eq!(evidence_text(&supplied, "pose_digest"), Some(digest.as_str()), "{supplied:?}");
    assert_eq!(evidence_text(&supplied, "source"), Some("supplied"));
    let journal = levels::retained_checkpoint(ctx, &state, Witness::Journal(&twists)).expect("no engine error");
    assert_eq!(evidence_text(&journal, "source"), Some("journal"), "{journal:?}");
    let other = levels::retained_checkpoint(ctx, &state, Witness::Word(&[1, 3])).expect("no engine error");
    assert!(matches!(other, Verdict::Unknown(_)), "a witness of another configuration: {other:?}");

    let reach = levels::witness(ctx, &state, Some(s.menu.clone()), &twists, None)
        .expect("no engine error")
        .expect("the journal witnesses the state");
    assert!(reach.retained());
    let verdict = levels::label_checkpoint(ctx, &state, &reach, None).expect("no engine error");
    assert_eq!(evidence_text(&verdict, "nu_source"), Some("retained_witness"), "{verdict:?}");
    assert_eq!(evidence_text(&verdict, "pose_digest"), Some(digest.as_str()));
    assert_eq!(evidence_text(&verdict, "contract_revision"), Some(levels::A5_REVISION));
    let zero = vec![0i64; levels::NU_LEN];
    assert_eq!(levels::label_checkpoint(ctx, &state, &reach, Some(zero.as_slice())).expect("no engine error"), verdict);
    let mut one = zero.clone();
    one[0] = 1;
    assert!(
        levels::label_checkpoint(ctx, &state, &reach, Some(one.as_slice())).is_err(),
        "a supplied ν that disagrees"
    );
    for kind in [ReachKind::Open, ReachKind::Promised] {
        let verdict =
            levels::label_checkpoint(ctx, &state, &stated(kind), Some(zero.as_slice())).expect("no engine error");
        assert!(matches!(verdict, Verdict::Unknown(_)), "{}: {verdict:?}", kind.name());
        // Without a witness ν is only what the caller supplies: a nonzero ν answers no whatever the claim.
        let verdict =
            levels::label_checkpoint(ctx, &state, &stated(kind), Some(one.as_slice())).expect("no engine error");
        assert_eq!(fact(&verdict), Some("nu_nonzero"), "{verdict:?}");
    }
    // A journal that does not reach the state witnesses nothing.
    assert!(levels::witness(ctx, &state, Some(s.menu.clone()), &twists[..2], None).expect("no engine error").is_err());
}

/// Off the lattice every end test answers no with the off count, whatever the reachability claim.
#[test]
fn off_the_lattice_every_end_test_answers_no() {
    let ctx = &setup().ctx;
    let state = cayley_state(ctx);
    let off = state.off_lattice_count() as i64;
    let verdicts = [
        levels::sticker_test(ctx, &state).expect("no engine error"),
        levels::retained_checkpoint(ctx, &state, Witness::Word(&[])).expect("no engine error"),
        levels::label_checkpoint(ctx, &state, &stated(ReachKind::Open), None).expect("no engine error"),
        levels::label_checkpoint(ctx, &state, &Reachability::solved(), None).expect("no engine error"),
    ];
    for verdict in &verdicts {
        assert_eq!(fact(verdict), Some("off_lattice"), "{verdict:?}");
        let Verdict::No(evidence) = verdict else { unreachable!() };
        assert_eq!(evidence.get("off").and_then(Value::as_i64), Some(off));
    }
}

/// A rotation of the whole puzzle by a K⁺ element that moves poles displaces centres: no checkpoint, at any claim.
#[test]
fn a_displaced_centre_is_no_checkpoint() {
    let s = setup();
    let ctx = &s.ctx;
    let k = ctx.kplus.gen_idx[0];
    let state = levels::state_of(ctx, None, &vec![Pose::K(k); NP]).expect("the raw table reads");
    let displaced = levels::displaced_centres(ctx, &state).expect("no engine error");
    assert!(!displaced.is_empty());
    let verdicts = [
        levels::retained_checkpoint(ctx, &state, Witness::Word(&[1])).expect("no engine error"),
        levels::label_checkpoint(ctx, &state, &stated(ReachKind::Open), None).expect("no engine error"),
    ];
    for verdict in &verdicts {
        assert_eq!(fact(verdict), Some("centre_displaced"), "{verdict:?}");
    }
    // Every piece is in some chamber and π is a permutation, but not the identity.
    assert_eq!(fact(&levels::sticker_test(ctx, &state).expect("no engine error")), Some("labels_differ"));
}

/// Stated claims are never witnessed: a witnessed claim comes only from a replay or a derivation.
#[test]
fn a_witnessed_claim_cannot_be_stated() {
    assert!(Reachability::stated(ReachKind::Witnessed, Value::Null).is_none());
    for kind in [ReachKind::Open, ReachKind::Promised, ReachKind::ByTheorem, ReachKind::Refuted] {
        let claim = stated(kind);
        assert_eq!((claim.kind(), claim.retained()), (kind, false));
        assert_eq!(ReachKind::parse(kind.name()), Some(kind));
    }
    assert!(Reachability::solved().retained());
}

// ------------------------------------------------------------------------------------------- shapes and gauges

/// Q of CT-KN1-06: the exact Cayley rotation ω = (2, 0, 0) at cap 0.
fn cayley_q(ctx: &Context) -> Arc<Mat4> {
    let omega = [Q5::int(2), Q5::zero(), Q5::zero()];
    Twist::from_record(ctx, &observe::cayley_record(ctx, 0, &omega)).expect("the Cayley record reads").matrix
}

/// Piece 4 and the next two pieces of its orbit, each with the K⁺ element that carries its home region onto piece 4's.
fn congruent_pieces(ctx: &Context) -> [(usize, u16); 3] {
    let (kp, regions) = (&ctx.kplus, &ctx.regions);
    let (orbit, to_4) = regions.rep_of(4);
    let onto = |p: usize| kp.compose(to_4, kp.inv[regions.rep_of(p).1 as usize]);
    let mut others = (0..NP).filter(|&p| p != 4 && regions.rep_of(p).0 == orbit);
    let (b, c) = (others.next().expect("a second piece"), others.next().expect("a third piece"));
    [(4, onto(4)), (b, onto(b)), (c, onto(c))]
}

/// Solved except for the listed poses: a raw pose table, read for form and identity only (KN2.14).
fn raw_table(ctx: &Context, posed: &[(usize, Pose)]) -> State {
    let mut poses = vec![Pose::K(0); NP];
    for (piece, pose) in posed {
        poses[*piece] = pose.clone();
    }
    levels::state_of(ctx, None, &poses).expect("the raw table reads")
}

/// σA = g(A) and t_A = g for a K⁺ element g: piece A takes the pose of the piece whose home region is g(Ā₀).
fn relabel(ctx: &Context, g: u16) -> Gauge {
    let table = levels::tables(ctx).expect("the level tables");
    let sigma = (0..NP).map(|a| table.image(ctx, a, g)).collect();
    Gauge::new(ctx, sigma, vec![g; NP]).expect("a relabel is a gauge")
}

/// σ swaps two pieces of one orbit. Each comes with the element that carries its home region onto one common region,
/// so tc⁻¹·tb carries b's home region onto c's.
fn swap(ctx: &Context, (b, tb): (usize, u16), (c, tc): (usize, u16)) -> Gauge {
    let kp = &ctx.kplus;
    let mut sigma: Vec<u32> = (0..NP as u32).collect();
    sigma.swap(b, c);
    let mut t = vec![0u16; NP];
    t[b] = kp.compose(kp.inv[tc as usize], tb);
    t[c] = kp.compose(kp.inv[tb as usize], tc);
    Gauge::new(ctx, sigma, t).expect("a swap within an orbit is a gauge")
}

/// A gauge of the class. S turns every piece of each orbit whose S_A is more than {id} by an element of S_A other than
/// id; R turns every piece of each orbit with R_A ≠ S_A by an element of R_A outside S_A; `All` is a relabel. The
/// element is picked at the orbit's representative and carried to each piece by its transport.
fn class_gauge(ctx: &Context, class: GaugeClass) -> Gauge {
    let (kp, regions) = (&ctx.kplus, &ctx.regions);
    if class == GaugeClass::All {
        return relabel(ctx, kp.gen_idx[0]);
    }
    let picks: Vec<Option<u16>> = regions
        .reps
        .iter()
        .map(|&rep| {
            let r = levels::region_stabiliser(ctx, rep as usize).expect("R_A");
            let s = levels::sticker_stabiliser(ctx, rep as usize).expect("S_A");
            match class {
                GaugeClass::S => s.iter().copied().find(|&k| k != 0),
                _ => r.iter().copied().find(|k| !s.contains(k)),
            }
        })
        .collect();
    let t = (0..NP)
        .map(|a| {
            let (orbit, tr) = regions.rep_of(a);
            picks[orbit].map_or(0, |k| kp.compose(tr, kp.compose(k, kp.inv[tr as usize])))
        })
        .collect();
    Gauge::new(ctx, (0..NP as u32).collect(), t).expect("a turn of each piece within its region is a gauge")
}

/// Each twist from a copy of `from`, with its status and the pieces it moved.
fn run(ctx: &Context, from: &State, twists: &[Twist]) -> (State, Vec<(Status, Vec<u32>)>) {
    let mut state = from.clone();
    let mut steps = Vec::with_capacity(twists.len());
    for twist in twists {
        let outcome = state.apply(ctx, twist, None, &[]).expect("no engine error");
        steps.push((outcome.status, outcome.inside));
    }
    (state, steps)
}

/// KN1.7's stabilisers: S_A lies in R_A, both are groups, the stabilisers of a piece are its representative's carried
/// by the transport, and the census of the pieces by (|R_A|, |S_A|, stickers) is the one J1's regions give
/// (`research/jumbling/sim/regions.py`, counted with the same definitions).
#[test]
fn region_and_sticker_stabilisers() {
    let ctx = &setup().ctx;
    let (kp, regions) = (&ctx.kplus, &ctx.regions);
    let table = levels::tables(ctx).expect("the level tables");
    assert_eq!(levels::region_stabiliser(ctx, 4).expect("R_A"), [0, 834]);
    assert_eq!(levels::sticker_stabiliser(ctx, 4).expect("S_A"), [0]);
    assert_eq!(levels::region_stabiliser(ctx, 1).expect("R_A"), [0, 5, 25, 80, 208, 209]);
    assert_eq!(levels::sticker_stabiliser(ctx, 1).expect("S_A"), [0, 5, 25]);
    assert!(levels::region_stabiliser(ctx, NP).is_err());

    let mut members = vec![0usize; regions.reps.len()];
    let mut last = vec![0usize; regions.reps.len()];
    for p in 0..NP {
        let orbit = regions.rep_of(p).0;
        members[orbit] += 1;
        last[orbit] = p;
    }
    let closed =
        |group: &[u16]| group.iter().all(|&i| group.iter().all(|&j| group.binary_search(&kp.compose(i, j)).is_ok()));
    let mut census: BTreeMap<(usize, usize, usize), usize> = BTreeMap::new();
    for (orbit, &rep) in regions.reps.iter().enumerate() {
        let r = levels::region_stabiliser(ctx, rep as usize).expect("R_A");
        let s = levels::sticker_stabiliser(ctx, rep as usize).expect("S_A");
        assert!(r.contains(&0) && s.contains(&0) && s.iter().all(|k| r.contains(k)), "orbit {orbit}");
        assert!(closed(&r) && closed(&s), "orbit {orbit}: the stabilisers are groups");
        let (r2, s2) = (
            levels::region_stabiliser(ctx, last[orbit]).expect("R_A"),
            levels::sticker_stabiliser(ctx, last[orbit]).expect("S_A"),
        );
        assert_eq!((r2.len(), s2.len()), (r.len(), s.len()), "orbit {orbit}");
        *census.entry((r.len(), s.len(), table.slots(rep as usize).len())).or_default() += members[orbit];
    }
    let j1 = BTreeMap::from([
        ((1, 1, 1), 93600),
        ((1, 1, 2), 43200),
        ((2, 1, 2), 21600),
        ((2, 2, 1), 3600),
        ((3, 3, 1), 9600),
        ((5, 1, 5), 2880),
        ((6, 3, 2), 1200),
        ((10, 1, 5), 720),
        ((12, 12, 1), 600),
        ((60, 1, 20), 120),
    ]);
    assert_eq!(census, j1);
}

/// CT-KN1-06 (part: the raw tables and gauges with σ ≠ id; not run: the fixture clause, which
/// tests/test_jumbling_rust_m8_levels.py runs at sampled fixture prefixes). On the raw tables (R, R, QR) and
/// (R, QR, QR) of three pieces with congruent home regions, R piece 4's home region and Q the Cayley rotation
/// ω = (2, 0, 0) at cap 0, the sets of occupied regions are equal while the shapes and the off counts differ, and no
/// gauge relates the tables. A gauge with σ ≠ id keeps the multiset; on a table that collides, one may even fix it.
#[test]
fn ct_kn1_06_equal_shape_is_an_equal_multiset_of_posed_regions() {
    let ctx = &setup().ctx;
    let kp = &ctx.kplus;
    let q = cayley_q(ctx);
    let [(a, ta), (b, tb), (c, tc)] = congruent_pieces(ctx);
    let table = levels::tables(ctx).expect("the level tables");
    assert_eq!(ta, 0);
    assert_eq!((table.image(ctx, b, tb), table.image(ctx, c, tc)), (a as u32, a as u32));
    let off = |k: u16| Pose::M(Arc::new(matmul(&q, kp.matrix(k, &ctx.model))));
    // Piece a stays at home in R.
    let t1 = raw_table(ctx, &[(b, Pose::K(tb)), (c, off(tc))]);
    let t2 = raw_table(ctx, &[(b, off(tb)), (c, off(tc))]);
    let s1 = levels::shape(ctx, &t1).expect("no engine error");
    let s2 = levels::shape(ctx, &t2).expect("no engine error");
    assert_eq!((s1.off_regions, s1.off_poses, s1.collisions), (1, 1, 1));
    assert_eq!((s2.off_regions, s2.off_poses, s2.collisions), (2, 2, 1));
    let keys = |s: &Shape| s.regions.iter().map(|(key, _)| key.clone()).collect::<Vec<RegionKey>>();
    assert_eq!(keys(&s1), keys(&s2), "one set of occupied regions");
    assert_ne!(s1.digest(), s2.digest(), "two multisets");
    assert_eq!(levels::gauge_between(ctx, &t1, &t2).expect("no engine error"), None);
    assert_eq!(levels::gauge_between(ctx, &t2, &t1).expect("no engine error"), None);

    let relabelled = relabel(ctx, kp.gen_idx[0]);
    let swapped = swap(ctx, (b, tb), (c, tc));
    for (x, sx) in [(&t1, &s1), (&t2, &s2)] {
        for gauge in [&relabelled, &swapped] {
            assert_eq!(gauge.class(ctx).expect("no engine error"), GaugeClass::All);
            assert!(gauge.moved() > 0);
            let z = levels::act(ctx, x, gauge).expect("no engine error");
            assert_eq!(levels::shape(ctx, &z).expect("no engine error").digest(), sx.digest());
            // Swapping the two pieces that share QR fixes (R, QR, QR); every other image is another table.
            assert_eq!(z.digest(ctx) == x.digest(ctx), std::ptr::eq(x, &t2) && std::ptr::eq(gauge, &swapped));
            // With a collision the gauge is not unique: the one found carries X onto Z.
            let found = levels::gauge_between(ctx, x, &z).expect("no engine error").expect("equal shapes");
            assert_eq!(levels::act(ctx, x, &found).expect("no engine error").digest(ctx), z.digest(ctx));
        }
    }
}

/// CT-KN2-06 (part: the engine against Theorem 1 of KN1.14 with S, R and σ ≠ id gauges, on a retained word on the
/// lattice and on the Cayley state off it; not run: J1's fixtures, which tests/test_jumbling_rust_m8_levels.py runs
/// against J1). For Z = X·γ: every surveyed grip has one status from X and from Z, each twist of a fixed word has one
/// status from both, piece A moves from Z exactly when σA moves from X, and w(Z) = w(X)·γ. The class's level is kept:
/// labels under 𝒢_S, posed regions under 𝒢_R, the shape under 𝒢; and the gauge between X and Z is γ.
#[test]
fn ct_kn2_06_a_gauge_transfers_admissibility_moves_and_images() {
    let s = setup();
    let ctx = &s.ctx;
    let table = levels::tables(ctx).expect("the level tables");
    let grips = [0usize, 1, 2, 13, 77, 120, 300, 599];
    let twists: Vec<Twist> = [1, 3, -5].iter().map(|&m| levels::primitive(ctx, m).expect("a primitive")).collect();
    let gauges = [GaugeClass::S, GaugeClass::R, GaugeClass::All].map(|class| (class, class_gauge(ctx, class)));
    for (name, x) in [("a retained word", retained_state(s, &[2, -7, 11]).0), ("the Cayley state", cayley_state(ctx))] {
        let x_survey = x.clone().survey(ctx, &grips, None).expect("no engine error");
        let (wx, x_steps) = run(ctx, &x, &twists);
        assert!(x_steps.iter().any(|(status, _)| *status == Status::Admissible), "{name}");
        for (class, gauge) in &gauges {
            assert_eq!(gauge.class(ctx).expect("no engine error"), *class);
            let z = levels::act(ctx, &x, gauge).expect("no engine error");
            assert_ne!(z.digest(ctx), x.digest(ctx), "{name}, {}", class.name());
            assert_eq!(z.clone().survey(ctx, &grips, None).expect("no engine error"), x_survey);
            let (wz, z_steps) = run(ctx, &z, &twists);
            for ((sx, ix), (sz, iz)) in x_steps.iter().zip(&z_steps) {
                assert_eq!(sx, sz, "{name}, {}", class.name());
                let mut mapped: Vec<u32> = iz.iter().map(|&p| gauge.sigma()[p as usize]).collect();
                mapped.sort_unstable();
                assert_eq!(&mapped, ix, "{name}, {}", class.name());
            }
            assert_eq!(levels::act(ctx, &wx, gauge).expect("no engine error").digest(ctx), wz.digest(ctx));

            let labels = |state: &State| levels::labels(ctx, state).expect("no engine error");
            let regions = |state: &State| levels::region_keys(ctx, &table, state).expect("no engine error");
            let shape = |state: &State| levels::shape(ctx, state).expect("no engine error").digest();
            match class {
                GaugeClass::S => assert_eq!(labels(&z), labels(&x), "{name}"),
                GaugeClass::R => {
                    assert_eq!(regions(&z), regions(&x), "{name}");
                    assert_ne!(labels(&z), labels(&x), "{name}: R_A turns stickers");
                }
                GaugeClass::All => {
                    assert_eq!(shape(&z), shape(&x), "{name}");
                    assert_ne!(regions(&z), regions(&x), "{name}");
                }
            }
            assert_eq!(levels::gauge_between(ctx, &x, &z).expect("no engine error").as_ref(), Some(gauge));
        }
    }
}

/// A gauge is checked piece by piece, and the identity acts trivially.
#[test]
fn a_gauge_is_checked_piece_by_piece() {
    let ctx = &setup().ctx;
    let id: Vec<u32> = (0..NP as u32).collect();
    assert!(Gauge::new(ctx, id[1..].to_vec(), vec![0; NP - 1]).is_err(), "too short");
    let mut twice = id.clone();
    twice[1] = 0;
    assert!(Gauge::new(ctx, twice, vec![0; NP]).is_err(), "σ is no permutation");
    let mut far = vec![0u16; NP];
    far[0] = ORDER as u16;
    assert!(Gauge::new(ctx, id.clone(), far).is_err(), "t names no K+ element");
    let g = *ctx.kplus.gen_idx.iter().find(|&&g| g != 0 && g != 834).expect("a generator outside R_4");
    let mut turned = vec![0u16; NP];
    turned[4] = g;
    assert!(Gauge::new(ctx, id.clone(), turned).is_err(), "t carries piece 4's region onto another");

    let identity = Gauge::identity();
    assert_eq!(Gauge::new(ctx, id, vec![0; NP]).expect("the identity"), identity);
    assert_eq!(
        (identity.class(ctx).expect("no engine error"), identity.moved(), identity.turned()),
        (GaugeClass::S, 0, 0)
    );
    let x = cayley_state(ctx);
    assert_eq!(levels::act(ctx, &x, &identity).expect("no engine error").digest(ctx), x.digest(ctx));
    assert_eq!(levels::gauge_between(ctx, &x, &x).expect("no engine error"), Some(identity));
}

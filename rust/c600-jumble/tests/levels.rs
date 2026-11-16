//! LP2, M8: the level crossings, end tests, shapes, gauges and derived states of the jumbling puzzle
//! (`c600_jumble::levels`; chapter 01, KN1.7 to KN1.10, KN1.14, KN1.17, KN2.7, KN2.8 and KN2.11 to KN2.14; chapter
//! 08, ONE.21 to ONE.28; state contract A3 and A5). `tests/test_jumbling_rust_m8_levels.py` runs the same calls
//! through the adapter against J1. Nothing here chooses a twist: every word below is fixed in the test or read from a
//! fixture.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use c600_jumble::codes;
use c600_jumble::kplus::ORDER;
use c600_jumble::levels::{self, Gauge, GaugeClass, RegionKey, Shape};
use c600_jumble::levels::{Base, Comparison, Derivation, Derived, Lineage, Missing, Refused, Snapshot, WordProtection};
use c600_jumble::levels::{Level, LevelValue, Partial, Projection, ReachKind, Reachability, Verdict, Witness};
use c600_jumble::model::{NP, NS};
use c600_jumble::q5::{matmul, transpose, Mat4, Q5};
use c600_jumble::state::{Pose, State, Status};
use c600_jumble::twist::{Menu, Twist};
use c600_jumble::{observe, Context};
use c600_kernel::json::{self, Value};
use c600_kernel::{IdentityBlock, ProtectionMode, Verdict as ProtectionVerdict};

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
    // The solved state's witnessed claim speaks of the solved state, not of this one (A4, A5).
    let verdict = levels::label_checkpoint(ctx, &state, &Reachability::solved(ctx), None).expect("no engine error");
    assert!(matches!(verdict, Verdict::Unknown(_)), "{verdict:?}");
}

/// A4 and A5: a witnessed claim speaks only of the state it reached. Used for another state it answers unknown, with ν
/// supplied or not, and its retained witness shows no ν there; on its own state it answers yes.
#[test]
fn a_witnessed_claim_speaks_only_of_its_own_state() {
    let s = setup();
    let ctx = &s.ctx;
    let (a, twists) = retained_state(s, &[1, 3, -5]);
    let (b, _) = retained_state(s, &[2, -7, 11]);
    let claim = levels::witness(ctx, &a, Some(s.menu.clone()), &twists, None)
        .expect("no engine error")
        .expect("the journal witnesses a");
    assert_eq!(claim.at(), Some(a.digest(ctx).as_str()));
    let zero = vec![0i64; levels::NU_LEN];
    for nu in [None, Some(zero.as_slice())] {
        let verdict = levels::label_checkpoint(ctx, &b, &claim, nu).expect("no engine error");
        assert!(matches!(verdict, Verdict::Unknown(_)), "{verdict:?}");
    }
    let verdict = levels::label_checkpoint(ctx, &a, &claim, None).expect("no engine error");
    assert_eq!(evidence_text(&verdict, "nu_source"), Some("retained_witness"), "{verdict:?}");
}

/// KN2.14 reads a raw table for form and identity only, so a lattice table with its centres home may hold two pieces
/// in one chamber. No word reaches it: the label checkpoint answers no with the collision count under any claim, as
/// the sticker test answers no, and never with an engine error.
#[test]
fn a_collided_table_is_no_checkpoint() {
    let ctx = &setup().ctx;
    let [_, (b, tb), _] = congruent_pieces(ctx);
    let state = raw_table(ctx, &[(b, Pose::K(tb))]);
    assert_eq!(state.off_lattice_count(), 0);
    assert!(levels::displaced_centres(ctx, &state).expect("no engine error").is_empty());
    assert_eq!(fact(&levels::sticker_test(ctx, &state).expect("no engine error")), Some("labels_differ"));
    for claim in [stated(ReachKind::Open), stated(ReachKind::Promised), Reachability::solved(ctx)] {
        let verdict = levels::label_checkpoint(ctx, &state, &claim, None).expect("a verdict, not an engine error");
        assert_eq!(fact(&verdict), Some("pieces_collide"), "{verdict:?}");
        let Verdict::No(evidence) = &verdict else { unreachable!() };
        assert!(evidence.get("collisions").and_then(Value::as_i64).is_some_and(|n| n > 0), "{verdict:?}");
    }
    let retained = levels::retained_checkpoint(ctx, &state, Witness::Word(&[])).expect("no engine error");
    assert!(!matches!(retained, Verdict::Yes(_)), "{retained:?}");
}

/// KN1.11 and JI.4: a verdict carries its level. The sticker test and the label checkpoint read the labels, the
/// retained checkpoint the poses, and every verdict kind names its level.
#[test]
fn a_verdict_carries_its_level() {
    assert_eq!(
        (levels::STICKER_TEST_LEVEL, levels::RETAINED_CHECKPOINT_LEVEL, levels::LABEL_CHECKPOINT_LEVEL),
        (Level::Labels, Level::Poses, Level::Labels)
    );
    for verdict in [Verdict::Yes(Value::Null), Verdict::No(Value::Null), Verdict::Unknown("a reason".into())] {
        for level in Level::ALL {
            let json = verdict.to_json_at(level);
            assert_eq!(json.get("level").and_then(Value::as_str), Some(level.name()));
            assert_eq!(json.get("verdict").and_then(Value::as_str), Some(verdict.name()));
        }
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
    assert_eq!(evidence_text(&verdict, "contract_revision"), Some(c600_jumble::CONTRACT_REVISION));
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
        levels::label_checkpoint(ctx, &state, &Reachability::solved(ctx), None).expect("no engine error"),
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
    assert!(Reachability::solved(&setup().ctx).retained());
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

// ---------------------------------------------------------------------------------------------- derived states

/// The twists of a fixed word of signed 1-based primitive ids.
fn primitives(ctx: &Context, word: &[i64]) -> Vec<Twist> {
    word.iter().map(|&mid| levels::primitive(ctx, mid).expect("a primitive id")).collect()
}

/// The Cayley twist of [`cayley_state`] and its inverse, the transposed exact matrix at the same grip.
fn cayley_twists(ctx: &Context) -> (Twist, Twist) {
    let omega = [Q5::int(2), Q5::zero(), Q5::zero()];
    let twist = Twist::from_record(ctx, &observe::cayley_record(ctx, 0, &omega)).expect("the Cayley record reads");
    let inverse = Twist::new(ctx, twist.grip, transpose(&twist.matrix)).expect("the inverse fixes the same pole");
    (twist, inverse)
}

/// The Cayley state as a snapshot that its one-letter journal from solved witnesses.
fn cayley_snapshot(ctx: &Context) -> (Snapshot, Twist) {
    let (twist, inverse) = cayley_twists(ctx);
    let journal = [twist];
    let (state, steps) = run(ctx, &State::new(None), &journal);
    assert_eq!(steps[0].0, Status::Admissible);
    let snapshot = Snapshot::witnessed(ctx, state, &journal, None)
        .expect("no engine error")
        .expect("the journal witnesses the Cayley state");
    (snapshot, inverse)
}

/// The pinned I_a fixture's journal at prefix 8 as a snapshot that the journal witnesses: off the lattice, with
/// primitive 3 blocked there and primitive 1 admissible.
fn fixture_snapshot(s: &Setup) -> Snapshot {
    let ctx = &s.ctx;
    let fixture = std::fs::read(repository().join("research/jumbling/fixtures/wj-I_a.json")).expect("the fixture");
    let fixture = json::parse(&fixture).expect("the I_a fixture is JSON");
    let records = fixture.get("journal").and_then(|j| j.get("records")).and_then(Value::as_array).expect("records");
    let journal: Vec<Twist> =
        records[..8].iter().map(|r| Twist::from_record(ctx, r).expect("a record the candidate reads")).collect();
    let (state, steps) = run(ctx, &State::new(Some(s.menu.clone())), &journal);
    assert!(steps.iter().all(|(status, _)| *status == Status::Admissible));
    assert!(state.off_lattice_count() > 0);
    Snapshot::witnessed(ctx, state, &journal, None).expect("no engine error").expect("the journal witnesses it")
}

/// `derive` with no engine error and no refusal of the request.
fn derivation(
    ctx: &Context,
    base: Base,
    identity: &IdentityBlock,
    twists: &[Twist],
    mode: ProtectionMode,
    protect: &[u32],
) -> Derivation {
    levels::derive(ctx, base, identity, twists, mode, protect)
        .expect("no engine error")
        .expect("the request is not refused")
}

fn derived(answer: Derivation) -> Derived {
    match answer {
        Derivation::Derived(value) => value,
        Derivation::Refused(refused) => panic!("refused: {refused:?}"),
    }
}

fn refused(answer: Derivation) -> Refused {
    match answer {
        Derivation::Refused(refused) => refused,
        Derivation::Derived(value) => panic!("derived: {value:?}"),
    }
}

/// A protection certificate: the piece with its K⁺ poses before and after the judged operation.
fn moved_piece(piece: u32, from: u16, to: u16) -> Value {
    json::object([
        ("piece", Value::Int(piece as i64)),
        ("from", json::object([("k", Value::Int(from as i64))])),
        ("to", json::object([("k", Value::Int(to as i64))])),
    ])
}

/// KN1.4 and chapter 13: a derivation names its base's identity block and protects pieces of the model; otherwise the
/// request is refused before any letter runs.
#[test]
fn a_derivation_refuses_another_identity_or_a_piece_outside_the_model() {
    let s = setup();
    let ctx = &s.ctx;
    let base = Snapshot::solved(ctx, Some(s.menu.clone()));
    let ia = ctx.identity(Some(&*s.menu));
    let twists = primitives(ctx, &[1]);
    let refusal = levels::derive(ctx, Base::Snapshot(&base), &ctx.identity(None), &twists, ProtectionMode::Net, &[])
        .expect("no engine error")
        .expect_err("another identity block");
    assert_eq!(refusal.code, codes::IDENTITY_MISMATCH);
    let refusal = levels::derive(ctx, Base::Snapshot(&base), &ia, &twists, ProtectionMode::Strict, &[3, NP as u32])
        .expect("no engine error")
        .expect_err("a piece outside the model");
    assert_eq!(refusal.code, codes::NAMED_OBJECT_INVALID);
    // A protected piece outside the letter's cap keeps its pose, so Strict clears the word.
    let (_, steps) = run(ctx, base.state(), &twists);
    let still = (0..NP as u32).find(|p| !steps[0].1.contains(p)).expect("a piece outside the cap");
    let value = derived(derivation(ctx, Base::Snapshot(&base), &ia, &twists, ProtectionMode::Strict, &[still]));
    assert_eq!(value.identity(), &ia);
    assert_eq!(value.protection().map(|r| (r.verdict, r.pieces.len())), Some((ProtectionVerdict::Clear, 0)));
}

/// KN2.8 and KN2.11: a word is refused whole at its first blocked or invalid letter, with the prefix's length, the
/// letter's outcome and, for a blocked letter, its straddle certificate; the base keeps its digest and no value is
/// made. The letters before it derive, and their value is witnessed by a journal that is no retained word.
#[test]
fn a_word_is_refused_whole_at_its_first_blocked_letter() {
    let s = setup();
    let ctx = &s.ctx;
    let base = fixture_snapshot(s);
    let ia = ctx.identity(Some(&*s.menu));
    let before = base.state().digest(ctx);
    let twists = primitives(ctx, &[1, -1, 3]);
    for mode in [ProtectionMode::Net, ProtectionMode::Strict] {
        let answer = refused(derivation(ctx, Base::Snapshot(&base), &ia, &twists, mode, &[]));
        assert_eq!((answer.first_refused_prefix, answer.outcome), (Some(3), Status::Blocked));
        assert_eq!(answer.refusal.code, codes::WORD_BLOCKED_PREFIX);
        assert_eq!((&answer.base, &answer.word, answer.letters), (&before, &levels::word_digest(&twists), 3));
        assert_eq!(answer.certificates.len(), 1, "{answer:?}");
        assert_eq!(answer.certificates[0].get("grip").and_then(Value::as_i64), Some(twists[2].grip as i64));
        assert_eq!(answer.protection, None);
    }
    assert_eq!(base.state().digest(ctx), before);
    let back = derived(derivation(ctx, Base::Snapshot(&base), &ia, &twists[..2], ProtectionMode::Strict, &[]));
    assert_eq!(back.digest(ctx), before);
    assert_eq!((back.reachability().kind(), back.reachability().retained()), (ReachKind::Witnessed, false));

    // A letter off the I_a menu is invalid: the word is refused at its prefix with the letter's reason.
    let (cayley, _) = cayley_twists(ctx);
    assert!(!s.menu.contains(ctx, &cayley), "the Cayley rotation is not an I_a item");
    let word = [twists[0].clone(), cayley];
    let answer = refused(derivation(ctx, Base::Snapshot(&base), &ia, &word, ProtectionMode::Net, &[]));
    assert_eq!((answer.first_refused_prefix, answer.outcome), (Some(2), Status::Invalid));
    assert_eq!(answer.refusal.code, codes::WORD_BLOCKED_PREFIX);
    assert!(answer.certificates.is_empty() && answer.protection.is_none(), "{answer:?}");
}

/// KN2.7 at pose level: Strict judges every prefix and refuses at the first letter that changes a protected piece's
/// pose; Net judges the end of the whole word. A word whose net map keeps the piece passes Net and fails Strict; a
/// word that leaves it moved fails both, Net with no prefix. Each refusal certifies the piece's pose before and after.
#[test]
fn strict_judges_every_prefix_and_net_the_end_of_the_word() {
    let s = setup();
    let ctx = &s.ctx;
    let ia = ctx.identity(Some(&*s.menu));
    let base = Snapshot::solved(ctx, Some(s.menu.clone()));
    let solved = base.state().digest(ctx);
    let word = primitives(ctx, &[1, 3, -3, -1]);
    let (_, steps) = run(ctx, base.state(), &word);
    assert!(steps.iter().all(|(status, _)| *status == Status::Admissible));
    let p = *steps[1].1.iter().find(|q| !steps[0].1.contains(q)).expect("a piece the second letter moves alone");
    let (two, _) = run(ctx, base.state(), &word[..2]);
    let to = two.pose_of(p as usize).kidx().expect("retained letters keep the piece on the lattice");
    let report = |mode, pieces: Vec<u32>, verdict, first_refused_prefix| WordProtection {
        mode,
        protected: vec![p],
        pieces,
        verdict,
        first_refused_prefix,
    };

    let strict = refused(derivation(ctx, Base::Snapshot(&base), &ia, &word, ProtectionMode::Strict, &[p, p]));
    assert_eq!((strict.first_refused_prefix, strict.outcome), (Some(2), Status::Protected));
    assert_eq!(strict.refusal.code, codes::PIECE_WOULD_MOVE);
    assert_eq!(strict.certificates, vec![moved_piece(p, 0, to)]);
    assert_eq!(strict.protection, Some(report(ProtectionMode::Strict, vec![p], ProtectionVerdict::Moves, Some(2))));

    let net = derived(derivation(ctx, Base::Snapshot(&base), &ia, &word, ProtectionMode::Net, &[p]));
    assert_eq!(net.digest(ctx), solved);
    assert_eq!(net.protection(), Some(&report(ProtectionMode::Net, vec![], ProtectionVerdict::Clear, None)));
    assert!(net.reachability().retained());

    let open = refused(derivation(ctx, Base::Snapshot(&base), &ia, &word[..2], ProtectionMode::Net, &[p]));
    assert_eq!((open.first_refused_prefix, open.outcome), (None, Status::Protected));
    assert_eq!(open.refusal.code, codes::PIECE_WOULD_MOVE);
    assert_eq!(open.certificates, vec![moved_piece(p, 0, to)]);
    assert_eq!(open.protection, Some(report(ProtectionMode::Net, vec![p], ProtectionVerdict::Moves, None)));

    // A piece in the cap of a letter and its inverse from solved: Strict refuses at prefix 1, and Net passes the word,
    // which brings the piece back.
    let there_and_back = [word[1].clone(), word[2].clone()];
    let q = run(ctx, base.state(), &there_and_back).1[0].1[0];
    let first = refused(derivation(ctx, Base::Snapshot(&base), &ia, &there_and_back, ProtectionMode::Strict, &[q]));
    assert_eq!((first.first_refused_prefix, first.outcome), (Some(1), Status::Protected));
    let back = derived(derivation(ctx, Base::Snapshot(&base), &ia, &there_and_back, ProtectionMode::Net, &[q]));
    assert_eq!(back.digest(ctx), solved);
    // Strict with nothing protected judges nothing.
    let free = derived(derivation(ctx, Base::Snapshot(&base), &ia, &word, ProtectionMode::Strict, &[]));
    assert_eq!(free.protection().map(|r| (r.verdict, r.protected.len())), Some((ProtectionVerdict::Clear, 0)));
}

/// KN2.11, KN2.12, ONE.21 and ONE.22: a derived value keeps its base's identity block and lineage, records its base and
/// word, and carries the base's claim along the word; a value derived from it continues the lineage. The base is
/// unchanged, and nothing is adopted.
#[test]
fn a_derived_value_keeps_its_identity_lineage_and_claim() {
    let s = setup();
    let ctx = &s.ctx;
    let ia = ctx.identity(Some(&*s.menu));
    let base = Snapshot::solved(ctx, Some(s.menu.clone()));
    let solved = base.state().digest(ctx);
    let first = derived(derivation(ctx, Base::Snapshot(&base), &ia, &primitives(ctx, &[1]), ProtectionMode::Net, &[]));
    let tail = primitives(ctx, &[3, -5]);
    let second = derived(derivation(ctx, Base::Derived(&first), &ia, &tail, ProtectionMode::Net, &[]));
    assert_eq!(second.digest(ctx), retained_state(s, &[1, 3, -5]).0.digest(ctx));
    assert_eq!(base.state().digest(ctx), solved, "the base is unchanged");
    for value in [&first, &second] {
        assert_eq!(value.identity(), &ia);
        assert_eq!(value.lineage(), &Lineage::Snapshot(solved.clone()));
        assert!(value.reachability().retained());
    }
    let from = second.derivation().expect("a derivation");
    assert_eq!((&from.base, &from.word, from.letters), (&first.digest(ctx), &levels::word_digest(&tail), 2));
    let verdict = levels::label_checkpoint(ctx, second.state(), second.reachability(), None).expect("no engine error");
    assert_eq!(evidence_text(&verdict, "nu_source"), Some("retained_witness"), "{verdict:?}");

    // The empty word derives the base's configuration.
    let same = derived(derivation(ctx, Base::Derived(&second), &ia, &[], ProtectionMode::Strict, &[]));
    assert_eq!(same.digest(ctx), second.digest(ctx));
    let record = second.to_json(ctx);
    let reference = record.get("ref").expect("a reference");
    assert_eq!(reference.get("digest").and_then(Value::as_str), Some(second.digest(ctx).as_str()));
    assert_eq!(reference.get("level").and_then(Value::as_str), Some("poses"));
    assert_eq!(record.get("lineage").and_then(|l| l.get("revision")), Some(&Value::Null));
}

/// CT-ONE-09 (part: KN2.14's import in c600-jumble, which takes a raw pose table only with an open or promised claim,
/// validates its form and identity, and makes a derived value with no derivation, no protection report and its own
/// digest as its lineage; not run: the runtime's entry points and table rows, LP3, and the session store's import, M6).
#[test]
fn ct_one_09_a_raw_table_enters_with_an_open_or_promised_claim() {
    let s = setup();
    let ctx = &s.ctx;
    let ia = ctx.identity(Some(&*s.menu));
    let (state, _) = retained_state(s, &[2, -7, 11]);
    let record = state.record(ctx);
    let digest = state.digest(ctx);
    let import = |record: &Value, identity: &IdentityBlock, claim| {
        levels::import(ctx, Some(s.menu.clone()), record, identity, claim)
    };
    for claim in [stated(ReachKind::ByTheorem), stated(ReachKind::Refuted), Reachability::solved(ctx)] {
        assert!(import(&record, &ia, claim).is_err());
    }
    for kind in [ReachKind::Open, ReachKind::Promised] {
        let value = import(&record, &ia, stated(kind)).expect("the table reads").expect("the identity agrees");
        assert_eq!(value.digest(ctx), digest);
        assert_eq!((value.reachability().kind(), value.reachability().retained()), (kind, false));
        assert_eq!(value.lineage(), &Lineage::Import(digest.clone()));
        assert!(value.derivation().is_none() && value.protection().is_none());
        assert_eq!(value.identity(), &ia);
    }
    let refusal = import(&record, &ctx.identity(None), stated(ReachKind::Open))
        .expect("the table reads")
        .expect_err("another identity block");
    assert_eq!(refusal.code, codes::IDENTITY_MISMATCH);
    assert!(import(&Value::Null, &ia, stated(ReachKind::Open)).is_err(), "no pose table");
}

/// CT-ONE-10 (whole: `label_checkpoint` on a derived value with an open claim answers unknown at ν = 0, for the import
/// of the solved table, whose identity labels show ν = 0, and for a value derived from it, with ν = 0 supplied or not;
/// the same word derived from the solved snapshot keeps a retained witness and answers yes).
#[test]
fn ct_one_10_a_derived_state_with_an_open_claim_answers_unknown_at_nu_zero() {
    let s = setup();
    let ctx = &s.ctx;
    let ia = ctx.identity(Some(&*s.menu));
    let solved = State::new(Some(s.menu.clone()));
    let import = levels::import(ctx, Some(s.menu.clone()), &solved.record(ctx), &ia, stated(ReachKind::Open))
        .expect("the table reads")
        .expect("the identity agrees");
    let word = primitives(ctx, &[1, 3, -5]);
    let on = derived(derivation(ctx, Base::Derived(&import), &ia, &word, ProtectionMode::Net, &[]));
    assert_eq!(on.reachability().kind(), ReachKind::Open);
    let zero = vec![0i64; levels::NU_LEN];
    for value in [&import, &on] {
        for nu in [None, Some(zero.as_slice())] {
            let verdict =
                levels::label_checkpoint(ctx, value.state(), value.reachability(), nu).expect("no engine error");
            assert!(matches!(verdict, Verdict::Unknown(_)), "{verdict:?}");
        }
    }
    let snapshot = Snapshot::solved(ctx, Some(s.menu.clone()));
    let retained = derived(derivation(ctx, Base::Snapshot(&snapshot), &ia, &word, ProtectionMode::Net, &[]));
    assert_eq!(retained.digest(ctx), on.digest(ctx));
    let verdict = levels::label_checkpoint(ctx, retained.state(), retained.reachability(), Some(zero.as_slice()))
        .expect("no engine error");
    assert!(matches!(verdict, Verdict::Yes(_)), "{verdict:?}");
}

/// CT-ONE-28 (part: a wordless import reads, compares and is a base for `derive`, whose value keeps the import's open
/// claim and lineage; not run: display, LP3's views, and that no row proposes or adopts either value, LP3's command
/// table and M6).
#[test]
fn ct_one_28_a_wordless_import_reads_compares_and_derives() {
    let s = setup();
    let ctx = &s.ctx;
    let ia = ctx.identity(Some(&*s.menu));
    let (state, _) = retained_state(s, &[2, -7, 11]);
    let import = levels::import(ctx, Some(s.menu.clone()), &state.record(ctx), &ia, stated(ReachKind::Open))
        .expect("the table reads")
        .expect("the identity agrees");
    for level in Level::ALL {
        match (level, levels::project(ctx, import.state(), level).expect("no engine error")) {
            (Level::StickerClass, Projection::Refused(refusal)) => assert_eq!(refusal.code, levels::LEVEL_NOT_HELD),
            (_, Projection::Value(Partial::Total(_))) => {
                let comparison = levels::compare(ctx, import.state(), &state, level).expect("no engine error");
                assert!(matches!(comparison, Comparison::Total(Verdict::Yes(_))), "{}: {comparison:?}", level.name());
            }
            (level, other) => panic!("{}: {other:?}", level.name()),
        }
    }
    let sticker = levels::sticker_test(ctx, import.state()).expect("no engine error");
    assert_eq!(fact(&sticker), Some("labels_differ"));

    let word = primitives(ctx, &[1]);
    let on = derived(derivation(ctx, Base::Derived(&import), &ia, &word, ProtectionMode::Strict, &[]));
    assert_eq!((on.reachability().kind(), on.lineage()), (ReachKind::Open, import.lineage()));
    assert_eq!(on.derivation().map(|d| d.base.clone()), Some(import.digest(ctx)));
    assert!(import.derivation().is_none());
    assert_eq!(on.digest(ctx), run(ctx, &state, &word).0.digest(ctx));
    let comparison = levels::compare(ctx, on.state(), import.state(), Level::Poses).expect("no engine error");
    assert!(matches!(comparison, Comparison::Total(Verdict::No(_))), "{comparison:?}");
}

/// CT-LG2-07 (part: the jumbled puzzle's engine half, where a raw table enters only by `import` with a claim that is
/// not witnessed and with validation, and a jumbling witness does not assert retained reachability; not run: the
/// language's state types and their static guard, which wait in c600-syntax for the domain families, and
/// 600-cell-Full's import, which c600-kernel does not build). A journal with a jumble twist witnesses its state but is
/// no retained word: a value derived from it by retained letters keeps a witnessed claim that is not retained and
/// answers unknown at the label checkpoint, while the same configuration derived from solved by retained letters
/// answers yes.
#[test]
fn ct_lg2_07_a_jumbling_witness_does_not_assert_retained_reachability() {
    let ctx = &setup().ctx;
    let free = ctx.identity(None);
    let (snapshot, inverse) = cayley_snapshot(ctx);
    assert_eq!((snapshot.reachability().kind(), snapshot.reachability().retained()), (ReachKind::Witnessed, false));
    let back = derived(derivation(ctx, Base::Snapshot(&snapshot), &free, &[inverse], ProtectionMode::Net, &[]));
    let solved = State::new(None);
    assert_eq!(back.digest(ctx), solved.digest(ctx));
    assert_eq!((back.reachability().kind(), back.reachability().retained()), (ReachKind::Witnessed, false));
    let word = primitives(ctx, &[1]);
    let on = derived(derivation(ctx, Base::Derived(&back), &free, &word, ProtectionMode::Net, &[]));
    assert!(!on.reachability().retained());
    let verdict = levels::label_checkpoint(ctx, on.state(), on.reachability(), None).expect("no engine error");
    assert!(matches!(verdict, Verdict::Unknown(_)), "{verdict:?}");

    let retained =
        derived(derivation(ctx, Base::Snapshot(&Snapshot::solved(ctx, None)), &free, &word, ProtectionMode::Net, &[]));
    assert_eq!(retained.digest(ctx), on.digest(ctx));
    assert!(retained.reachability().retained());
    let verdict =
        levels::label_checkpoint(ctx, retained.state(), retained.reachability(), None).expect("no engine error");
    assert!(matches!(verdict, Verdict::Yes(_)), "{verdict:?}");
    // A raw table never enters witnessed.
    assert!(levels::import(ctx, None, &solved.record(ctx), &free, Reachability::solved(ctx)).is_err());
}

/// ONE.25: two values compare at one level. Off the lattice the comparison is made on the keys that both cover, with
/// each side's missing part; an S-gauge image has equal labels and other poses; the sticker class is not held, and two
/// identity blocks that differ are refused.
#[test]
fn compare_answers_partially_off_the_lattice() {
    let s = setup();
    let ctx = &s.ctx;
    let cayley = cayley_state(ctx);
    let solved = State::new(None);
    let off = cayley.off_lattice_count();
    for level in [Level::Labels, Level::Position] {
        match levels::compare(ctx, &cayley, &solved, level).expect("no engine error") {
            Comparison::Partial { covered: Verdict::Yes(evidence), a, b } => {
                assert_eq!((a.off, a.collisions, b), (off, 0, Missing::default()), "{}", level.name());
                let count = |key: &str| evidence.get(key).and_then(Value::as_i64);
                assert_eq!(
                    (count("differing"), count("a_only"), count("b_only")),
                    (Some(0), Some(0), Some(a.uncovered as i64)),
                    "{}",
                    level.name()
                );
            }
            other => panic!("{}: {other:?}", level.name()),
        }
        let itself = levels::compare(ctx, &cayley, &cayley, level).expect("no engine error");
        assert!(matches!(itself, Comparison::Partial { covered: Verdict::Yes(_), .. }), "{itself:?}");
    }
    let not_held = levels::compare(ctx, &cayley, &solved, Level::StickerClass).expect("no engine error");
    assert!(matches!(&not_held, Comparison::Refused(r) if r.code == levels::LEVEL_NOT_HELD), "{not_held:?}");
    let other =
        levels::compare(ctx, &State::new(Some(s.menu.clone())), &solved, Level::Poses).expect("no engine error");
    assert!(matches!(&other, Comparison::Refused(r) if r.code == codes::IDENTITY_MISMATCH), "{other:?}");

    let x = retained_state(s, &[2, -7, 11]).0;
    let z = levels::act(ctx, &x, &class_gauge(ctx, GaugeClass::S)).expect("no engine error");
    let labels = levels::compare(ctx, &x, &z, Level::Labels).expect("no engine error");
    assert!(matches!(labels, Comparison::Total(Verdict::Yes(_))), "{labels:?}");
    let poses = levels::compare(ctx, &x, &z, Level::Poses).expect("no engine error");
    assert!(matches!(poses, Comparison::Total(Verdict::No(_))), "{poses:?}");
}

//! LP2, M8: the level crossings and end tests of the jumbling puzzle (`c600_jumble::levels`; chapter 01, KN1.7 to
//! KN1.10 and KN1.17; state contract A3 and A5). `tests/test_jumbling_rust_m8_levels.py` runs the same calls through
//! the adapter against J1. Nothing here chooses a twist: every word below is fixed in the test.

use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use c600_jumble::levels::{self, Level, LevelValue, Partial, Projection, ReachKind, Reachability, Verdict, Witness};
use c600_jumble::model::{NP, NS};
use c600_jumble::q5::Q5;
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

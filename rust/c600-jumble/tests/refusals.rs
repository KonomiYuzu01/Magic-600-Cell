//! JI.45 and JI.11 at the candidate's observation API (LP2, M5; the Fable solver's leads L1, L2 and L16 of
//! 16 November 2026). A named unit or cap outside the model is refused with `KN2:input.named_object_invalid`, which
//! names the argument, the supplied IDs and the failed condition, and nothing changes; a word check whose prefix is
//! refused makes no end-state assertion. CT-KN2-04 (part: a word check with a blocked letter and refused named objects
//! leave the state unchanged; not run here: the invalid and uncertain letters, which
//! `tests/test_jumbling_rust_candidate.py` runs). `tests/test_jumbling_rust_attacks.py` runs these rules and the other
//! leads through the adapter. Render feature J10's exposed faces are refused with
//! `KN2:input.exposed_geometry_unavailable` while no certified geometry or admitted size profile exists (JI.41).

use std::path::{Path, PathBuf};
use std::sync::Arc;

use c600_jumble::model::NP;
use c600_jumble::observe::{self, Batch, Fields, PrefixRequest, Rows, Scope};
use c600_jumble::state::{State, Status};
use c600_jumble::twist::{Menu, Twist};
use c600_jumble::{codes, faces, Context};
use c600_kernel::json::{self, Value};

fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..")
}

struct Setup {
    ctx: Context,
    menu: Arc<Menu>,
}

impl Setup {
    fn load() -> Setup {
        let assets = repository().join("assets");
        let ctx = Context::load(&assets).expect("the jumbling model loads");
        let fixture = std::fs::read(repository().join("research/jumbling/fixtures/wj-I_a.json")).expect("the fixture");
        let fixture = json::parse(&fixture).expect("the I_a fixture is JSON");
        let menu = Menu::from_record(&ctx, fixture.get("menu").expect("a menu")).expect("the pinned I_a menu");
        Setup { ctx, menu: Arc::new(menu) }
    }

    fn letter(&self, grip: usize, item: usize) -> ([i64; 2], Twist) {
        let twists = self.menu.for_cap(&self.ctx, grip).expect("the menu's twists");
        ([grip as i64, item as i64], twists[item].clone())
    }

    /// The state after the letters, every one admissible.
    fn at(&self, letters: &[(usize, usize)]) -> State {
        let mut state = State::new(Some(self.menu.clone()));
        for &(grip, item) in letters {
            let (_, twist) = self.letter(grip, item);
            let outcome = state.apply(&self.ctx, &twist, None, &[]).expect("no engine error");
            assert_eq!(outcome.status, Status::Admissible, "letter ({grip}, {item})");
        }
        state
    }
}

fn text(value: &Value, key: &str) -> Option<String> {
    value.get(key).and_then(Value::as_str).map(str::to_owned)
}

fn assert_invalid(answer: &Value, argument: &str, supplied: &[i64]) {
    assert_eq!(text(answer, "status").as_deref(), Some("refused"));
    assert_eq!(text(answer, "code").as_deref(), Some(codes::NAMED_OBJECT_INVALID));
    assert_eq!(text(answer, "argument").as_deref(), Some(argument));
    assert_eq!(answer.get("supplied"), Some(&Value::Array(supplied.iter().map(|&p| Value::Int(p)).collect())));
    assert!(text(answer, "condition").is_some_and(|c| !c.is_empty()), "{argument}: no condition");
}

#[test]
fn named_objects_outside_the_model_are_refused_and_change_nothing() {
    let s = Setup::load();
    let mut state = s.at(&[(0, 12)]);
    let before = state.record(&s.ctx);
    let undo = [s.letter(0, 45)];
    let lattice = (0..NP).find(|&p| state.is_lattice_piece(p)).expect("a lattice piece") as i64;
    // JI.9: an empty unit, a negative ID, an ID past the model and a lattice piece.
    for unit in [vec![], vec![-1], vec![NP as i64], vec![lattice]] {
        let answer =
            observe::word_check(&s.ctx, &state, &undo, Some(unit.as_slice()), None, None).expect("no engine error");
        assert_invalid(&answer, "unit", &unit);
        let answer = observe::units(&s.ctx, &state, Some(unit.as_slice())).expect("no engine error");
        assert_invalid(&answer, "pieces", &unit);
    }
    // JI.45: a cap outside 0..599, never another cap or an empty result.
    for cap in [600, -1] {
        let answer = observe::word_check(&s.ctx, &state, &undo, None, Some(cap), None).expect("no engine error");
        assert_invalid(&answer, "cap", &[cap]);
        let batch = Batch { chunk: 1 << 16, budget: Some(1 << 24), cancel_after: None };
        let answer = observe::seam_facts(&s.ctx, &mut state, Scope::Cap(cap), &batch, Rows::None, None)
            .expect("no engine error");
        assert_invalid(&answer, "scope", &[cap]);
        let request = PrefixRequest {
            letters: &undo,
            fields: Fields { n: false, heights: false, fd: false, sets: false },
            cap: Some(cap),
            batch: Batch { chunk: 1, budget: Some(1), cancel_after: None },
            keep: false,
            eval_budget: None,
        };
        let (answer, kept) = observe::prefix_observations(&s.ctx, &state, &request).expect("no engine error");
        assert_invalid(&answer, "cap", &[cap]);
        assert!(kept.is_empty());
    }
    assert_eq!(state.record(&s.ctx), before);
}

#[test]
fn a_refused_prefix_makes_no_end_state_assertion() {
    let s = Setup::load();
    let mut after_first = s.at(&[(0, 12)]);
    let grips: Vec<usize> = (0..600).collect();
    let statuses = after_first.survey(&s.ctx, &grips, None).expect("no engine error");
    let blocked = grips.iter().zip(&statuses).find(|(_, status)| **status == Status::Blocked).expect("a blocked grip");
    let identity = s.letter(*blocked.0, 0);
    assert_eq!(identity.1.kidx, Some(0), "item 0 of the pinned I_a menu is the identity");
    let solved = State::new(Some(s.menu.clone()));
    let before = solved.record(&s.ctx);
    let word = [s.letter(0, 12), identity];
    let check = observe::word_check(&s.ctx, &solved, &word, None, None, None).expect("no engine error");
    assert_eq!(check.get("admissible"), Some(&Value::Bool(false)));
    assert_eq!(check.get("failing_letter_index"), Some(&Value::Int(1)));
    assert_eq!(text(&check, "failing_status").as_deref(), Some("blocked"));
    assert_eq!(text(&check, "end_state_undefined").as_deref(), Some("refused_prefix"));
    assert_eq!(text(&check, "step_flag_undefined").as_deref(), Some("unit_not_named"));
    for field in [
        "end_digest",
        "collateral_ids",
        "others_landed",
        "off_after",
        "delta_N",
        "heights_after",
        "delta_n_h",
        "peak_left_height",
        "excursion",
        "tame",
        "step_flag",
    ] {
        assert_eq!(check.get(field), Some(&Value::Null), "{field}");
    }
    assert_eq!(solved.record(&s.ctx), before);
}

/// JI.41 and JI.45 for render feature J10: with no certified exposed-face geometry and no admitted size profile, the
/// producer refuses with `KN2:input.exposed_geometry_unavailable`, names the feature, the exact input and both missing
/// prerequisites, retains no draft, invents no mesh, and changes nothing.
#[test]
fn exposed_faces_are_unavailable_and_change_nothing() {
    let s = Setup::load();
    let state = s.at(&[(0, 12)]);
    let before = state.record(&s.ctx);
    let answer = faces::exposed_faces(&s.ctx, &state);
    assert_eq!(text(&answer, "status").as_deref(), Some("refused"));
    assert_eq!(text(&answer, "code").as_deref(), Some(codes::EXPOSED_GEOMETRY_UNAVAILABLE));
    assert_eq!(text(&answer, "feature").as_deref(), Some("exposed_face_meshes"));
    assert_eq!(text(&answer, "render_row").as_deref(), Some("J10"));
    let missing = answer.get("missing").and_then(Value::as_array).expect("the missing prerequisites");
    assert_eq!(missing, [Value::String("certified_geometry".into()), Value::String("size_profile".into())]);
    assert_eq!(answer.get("draft"), Some(&Value::Null), "no draft is retained");
    assert!(text(&answer, "continuation").is_some_and(|c| c.contains("unavailable")));
    for key in ["mesh", "faces", "vertices", "size"] {
        assert!(answer.get(key).is_none(), "no fabricated `{key}`");
    }
    let source = answer.get("source").expect("the exact input reference");
    assert_eq!(text(source, "state_digest"), Some(state.digest(&s.ctx)));
    assert_eq!(text(source, "model_id").as_deref(), Some(s.ctx.model.model_id.as_str()));
    assert_eq!(text(source, "menu_id").as_deref(), Some(s.menu.identity.as_str()));
    // Another configuration is another exact input.
    let other = faces::exposed_faces(&s.ctx, &State::new(Some(s.menu.clone())));
    assert_ne!(other.get("source"), answer.get("source"));
    assert_eq!(state.record(&s.ctx), before);
}

//! The M8 observations of the jumbled puzzle (LP2 plan, section 11; JI.6 and JI.7) at the candidate's observation API:
//! their refusals, their undefined fields and their agreement with the M5 observations they share facts with. The
//! comparison with J1 runs through the adapter in `tests/test_jumbling_rust_m8_observations.py`.
//! CT-KN2-01 (part: the candidate's M8 observations agree with its own seam facts; not run here: the comparison with
//! J1, which `tests/test_jumbling_rust_m8_observations.py` runs).

use std::path::{Path, PathBuf};
use std::sync::Arc;

use c600_jumble::model::POLES;
use c600_jumble::observe::{self, cuts, Batch, Rows, Scope};
use c600_jumble::state::{State, Status};
use c600_jumble::twist::{Menu, Twist};
use c600_jumble::{codes, Context};
use c600_kernel::json::{self, Value};

fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..")
}

struct Setup {
    ctx: Context,
    menu: Arc<Menu>,
    records: Vec<Value>,
}

impl Setup {
    fn load() -> Setup {
        let assets = repository().join("assets");
        let ctx = Context::load(&assets).expect("the jumbling model loads");
        let fixture = std::fs::read(repository().join("research/jumbling/fixtures/wj-I_a.json")).expect("the fixture");
        let fixture = json::parse(&fixture).expect("the I_a fixture is JSON");
        let menu = Menu::from_record(&ctx, fixture.get("menu").expect("a menu")).expect("the pinned I_a menu");
        let records = fixture
            .get("journal")
            .and_then(|j| j.get("records"))
            .and_then(Value::as_array)
            .expect("the journal's records")
            .to_vec();
        Setup { ctx, menu: Arc::new(menu), records }
    }

    /// The letter (grip, item) of the pinned I_a menu.
    #[allow(dead_code)]
    fn letter(&self, grip: usize, item: usize) -> ([i64; 2], Twist) {
        let twists = self.menu.for_cap(&self.ctx, grip).expect("the menu's twists");
        ([grip as i64, item as i64], twists[item].clone())
    }

    /// The state after the first `n` records of the I_a fixture's journal, every one admissible.
    fn after(&self, n: usize) -> State {
        let mut state = State::new(Some(self.menu.clone()));
        for (i, record) in self.records[..n].iter().enumerate() {
            let twist = Twist::from_record(&self.ctx, record).expect("a menu record");
            let outcome = state.apply(&self.ctx, &twist, None, &[]).expect("no engine error");
            assert_eq!(outcome.status, Status::Admissible, "record {i}");
        }
        state
    }
}

fn text(value: &Value, key: &str) -> Option<String> {
    value.get(key).and_then(Value::as_str).map(str::to_owned)
}

fn int(value: &Value, key: &str) -> Option<i64> {
    value.get(key).and_then(Value::as_i64)
}

fn rows(answer: &Value) -> &[Value] {
    answer.get("rows").and_then(Value::as_array).expect("rows")
}

fn assert_invalid(answer: &Value, argument: &str, supplied: &[i64]) {
    assert_eq!(text(answer, "status").as_deref(), Some("refused"));
    assert_eq!(text(answer, "code").as_deref(), Some(codes::NAMED_OBJECT_INVALID));
    assert_eq!(text(answer, "argument").as_deref(), Some(argument));
    assert_eq!(answer.get("supplied"), Some(&Value::Array(supplied.iter().map(|&p| Value::Int(p)).collect())));
    assert!(text(answer, "condition").is_some_and(|c| !c.is_empty()), "{argument}: no condition");
}

const RIM_FIELDS: [&str; 6] = ["N_c", "In_c", "Out_c", "A_c", "F_c", "at_floor"];

#[test]
fn rim_summary_refuses_a_cap_outside_the_model_and_changes_nothing() {
    let s = Setup::load();
    let mut state = s.after(1);
    let before = state.record(&s.ctx);
    // JI.45: a cap outside 0..599 and an empty selection, never another cap or an empty answer.
    for caps in [vec![600], vec![-1], vec![3, 600], vec![]] {
        let answer = cuts::rim_summary(&s.ctx, &mut state, Some(caps.as_slice()), None).expect("no engine error");
        assert_invalid(&answer, "caps", &caps);
    }
    assert_eq!(state.record(&s.ctx), before);
}

#[test]
fn the_solved_state_has_every_cap_aligned() {
    let s = Setup::load();
    let mut state = State::new(Some(s.menu.clone()));
    let answer = cuts::rim_summary(&s.ctx, &mut state, None, None).expect("no engine error");
    assert_eq!(text(&answer, "status").as_deref(), Some("ok"));
    assert_eq!(answer.get("complete"), Some(&Value::Bool(true)));
    assert_eq!(int(&answer, "grip_n"), Some(0));
    assert_eq!(answer.get("grip_n_undefined"), Some(&Value::Null));
    assert_eq!(rows(&answer).len(), POLES);
    for (c, row) in rows(&answer).iter().enumerate() {
        assert_eq!(int(row, "cap"), Some(c as i64));
        assert_eq!(text(row, "status").as_deref(), Some("admissible"), "cap {c}");
        assert_eq!(row.get("blocked"), Some(&Value::Bool(false)), "cap {c}");
        let (n_in, n_out, aligned) = (int(row, "In_c").unwrap(), int(row, "Out_c").unwrap(), int(row, "A_c").unwrap());
        assert!(n_in > 0 && n_in == n_out && n_out == aligned, "cap {c}: every cut cell is held on both sides");
        assert_eq!((int(row, "N_c"), int(row, "F_c")), (Some(0), Some(0)), "cap {c}");
        assert_eq!(row.get("at_floor"), Some(&Value::Bool(true)), "cap {c}");
    }
}

#[test]
fn rims_agree_with_seam_facts_and_blocked_caps_have_none() {
    let s = Setup::load();
    let mut state = s.after(3);
    let before = state.record(&s.ctx);
    let answer = cuts::rim_summary(&s.ctx, &mut state, None, None).expect("no engine error");
    assert_eq!(answer.get("complete"), Some(&Value::Bool(true)));
    let grips: Vec<usize> = (0..POLES).collect();
    let statuses = state.survey(&s.ctx, &grips, None).expect("no engine error");
    let (mut grip_n, mut misaligned, mut blocked, mut aligned_checked) = (0, 0, 0, 0);
    let batch = Batch { chunk: 1 << 16, budget: Some(1 << 24), cancel_after: None };
    for (c, row) in rows(&answer).iter().enumerate() {
        assert_eq!(int(row, "cap"), Some(c as i64));
        assert_eq!(text(row, "status").as_deref(), Some(statuses[c].as_str()), "cap {c}");
        if statuses[c] != Status::Admissible {
            blocked += 1;
            assert_eq!(row.get("blocked"), Some(&Value::Bool(true)), "cap {c}");
            assert_eq!(text(row, "undefined").as_deref(), Some("cap_not_admissible"), "cap {c}");
            for field in RIM_FIELDS {
                assert_eq!(row.get(field), Some(&Value::Null), "cap {c}: {field} is undefined, never zero");
            }
            continue;
        }
        let n = int(row, "N_c").unwrap();
        let (n_in, n_out, aligned) = (int(row, "In_c").unwrap(), int(row, "Out_c").unwrap(), int(row, "A_c").unwrap());
        assert_eq!(n, n_in + n_out - 2 * aligned, "cap {c}");
        assert_eq!(int(row, "F_c"), Some((n_in - n_out).abs()), "cap {c}");
        assert_eq!(row.get("at_floor"), Some(&Value::Bool(n == (n_in - n_out).abs())), "cap {c}");
        grip_n += n;
        // The seam facts of the cap's cut (M5) count the same cells: every misaligned cap and a few aligned ones.
        if n > 0 || aligned_checked < 8 {
            misaligned += (n > 0) as i64;
            aligned_checked += (n == 0) as i64;
            let facts = observe::seam_facts(&s.ctx, &mut state, Scope::Cap(c as i64), &batch, Rows::None, None)
                .expect("no engine error");
            let totals = facts.get("totals").expect("totals");
            assert_eq!((int(totals, "N_c"), int(totals, "aligned")), (Some(n), Some(aligned)), "cap {c}");
        }
    }
    assert!(misaligned > 0 && blocked > 0, "the state has misaligned and blocked caps");
    assert_eq!(int(&answer, "grip_n"), Some(grip_n));
    assert_eq!(state.record(&s.ctx), before);
}

#[test]
fn named_caps_and_an_exhausted_budget_claim_no_grip_n() {
    let s = Setup::load();
    let mut state = s.after(1);
    let named = cuts::rim_summary(&s.ctx, &mut state, Some(&[5, 3, 5]), None).expect("no engine error");
    let caps: Vec<i64> = rows(&named).iter().map(|row| int(row, "cap").unwrap()).collect();
    assert_eq!(caps, vec![3, 5]);
    assert_eq!(named.get("grip_n"), Some(&Value::Null));
    assert_eq!(text(&named, "grip_n_undefined").as_deref(), Some("caps_selected"));
    // A classification budget of zero evaluations leaves every cap uncertain while pieces are off the lattice: the
    // rows claim no blocked flag and no rim.
    let answer = cuts::rim_summary(&s.ctx, &mut state, None, Some(0)).expect("no engine error");
    assert_eq!(answer.get("complete"), Some(&Value::Bool(false)));
    assert_eq!(int(&answer, "uncertain_caps"), Some(POLES as i64));
    assert_eq!(answer.get("grip_n"), Some(&Value::Null));
    assert_eq!(text(&answer, "grip_n_undefined").as_deref(), Some("uncertain_caps"));
    for row in rows(&answer) {
        assert_eq!(text(row, "status").as_deref(), Some("uncertain"));
        assert_eq!(row.get("blocked"), Some(&Value::Null));
        assert_eq!(text(row, "undefined").as_deref(), Some("uncertain_under_budget"));
        for field in RIM_FIELDS {
            assert_eq!(row.get(field), Some(&Value::Null), "{field}");
        }
    }
}

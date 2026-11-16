//! The M8 observations of the jumbled puzzle (LP2 plan, section 11; JI.6 and JI.7) at the candidate's observation API:
//! their refusals, their undefined fields and their agreement with the M5 observations they share facts with. The
//! comparison with J1 runs through the adapter in `tests/test_jumbling_rust_m8_observations.py`.
//! CT-KN2-01 (part: the candidate's M8 observations agree with its own seam facts, word check and rim summary; not run
//! here: the comparison with J1, which `tests/test_jumbling_rust_m8_observations.py` runs).
//! CT-KN2-04 (part: the release check of a blocked, an off-menu and an uncertain letter leaves the start's digest
//! unchanged; not run here: words of more than one letter, which the M5 word-check tests cover).

use std::path::{Path, PathBuf};
use std::sync::Arc;

use c600_jumble::model::POLES;
use c600_jumble::observe::{self, cuts, Batch, Rows, Scope};
use c600_jumble::q5::Q5;
use c600_jumble::state::{State, Status};
use c600_jumble::twist::{self, Menu, Twist};
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
    fn letter(&self, grip: usize, item: usize) -> ([i64; 2], Twist) {
        let twists = self.menu.for_cap(&self.ctx, grip).expect("the menu's twists");
        ([grip as i64, item as i64], twists[item].clone())
    }

    /// The letter of journal record n: its grip and the item of its `I_a[k]` label, the same rotation as the record.
    fn record_letter(&self, n: usize) -> ([i64; 2], Twist) {
        let record = &self.records[n];
        let grip = int(record, "grip").expect("a grip") as usize;
        let label = record.get("params").and_then(|p| p.get("label")).and_then(Value::as_str).expect("a label");
        let item = label.strip_prefix("I_a[").and_then(|l| l.strip_suffix(']')).expect("an I_a label");
        let letter = self.letter(grip, item.parse().expect("an item index"));
        let twist = Twist::from_record(&self.ctx, record).expect("a menu record");
        assert_eq!(letter.1.matrix, twist.matrix, "record {n}: the letter is the record's rotation");
        letter
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

/// The rim summary's row of cap c.
fn rim_row(s: &Setup, state: &mut State, c: usize) -> Value {
    let answer = cuts::rim_summary(&s.ctx, state, Some(&[c as i64]), None).expect("no engine error");
    rows(&answer)[0].clone()
}

/// The release check's rim fields, all undefined, with their reason.
fn assert_no_rim(answer: &Value, reason: &str) {
    for field in ["rim_before", "rim_after", "floor", "delta_N_c"] {
        assert_eq!(answer.get(field), Some(&Value::Null), "{field} is undefined, never zero");
    }
    assert_eq!(text(answer, "rim_undefined").as_deref(), Some(reason));
    assert_eq!(answer.get("admissible"), Some(&Value::Bool(false)));
    assert_eq!(text(answer, "end_state_undefined").as_deref(), Some("refused_prefix"));
    assert_eq!(answer.get("end_digest"), Some(&Value::Null));
}

#[test]
fn release_check_is_the_one_letter_word_check_plus_the_rim_of_its_cap() {
    let s = Setup::load();
    for n in [0, 3] {
        let mut start = s.after(n);
        let letter = s.record_letter(n);
        let c = letter.1.grip;
        let before = start.record(&s.ctx);
        let answer = cuts::release_check(&s.ctx, &start, &letter, None, None).expect("no engine error");
        // JI.11: the one-letter release and word checks agree in every shared field at identical inputs.
        let word = observe::word_check(&s.ctx, &start, std::slice::from_ref(&letter), None, None, None)
            .expect("no engine error");
        for (key, value) in word.as_object().expect("a word check is an object") {
            assert_eq!(answer.get(key), Some(value), "record {n}: {key}");
        }
        assert_eq!(answer.get("admissible"), Some(&Value::Bool(true)), "record {n}");
        assert_eq!(int(&answer, "letter_cap"), Some(c as i64), "record {n}");
        assert_eq!(answer.get("rim_undefined"), Some(&Value::Null), "record {n}");
        // The rims are the rim summary's rows of the letter's cap before the letter and on the journal's next state.
        let mut end = s.after(n + 1);
        assert_eq!(text(&answer, "end_digest"), Some(end.digest(&s.ctx)), "record {n}");
        let (row_before, row_after) = (rim_row(&s, &mut start, c), rim_row(&s, &mut end, c));
        let (rim_before, rim_after) = (answer.get("rim_before").unwrap(), answer.get("rim_after").unwrap());
        for field in RIM_FIELDS {
            assert_eq!(rim_before.get(field), row_before.get(field), "record {n}: {field} before");
            assert_eq!(rim_after.get(field), row_after.get(field), "record {n}: {field} after");
        }
        let (n_before, n_after) = (int(rim_before, "N_c").unwrap(), int(rim_after, "N_c").unwrap());
        assert_eq!(int(&answer, "delta_N_c"), Some(n_after - n_before), "record {n}");
        assert_eq!(int(&answer, "floor"), int(rim_before, "F_c"), "record {n}");
        assert_eq!(int(&answer, "floor"), int(rim_after, "F_c"), "record {n}: the floor is the same after");
        // A twist at an admissible cap changes N only on the cap's cut, so total delta N is delta N_c.
        assert_eq!(int(&answer, "delta_N"), Some(n_after - n_before), "record {n}");
        assert_eq!(start.record(&s.ctx), before, "record {n}");
    }
}

#[test]
fn release_check_of_a_refused_letter_has_no_rim_and_changes_nothing() {
    let s = Setup::load();
    let mut start = s.after(3);
    let before = start.record(&s.ctx);
    let grips: Vec<usize> = (0..POLES).collect();
    let statuses = start.survey(&s.ctx, &grips, None).expect("no engine error");
    let blocked = statuses.iter().position(|&status| status == Status::Blocked).expect("a blocked cap");
    // A blocked, invalid or uncertain letter leaves the start's digest unchanged.
    let answer = cuts::release_check(&s.ctx, &start, &s.letter(blocked, 1), None, None).expect("no engine error");
    assert_eq!(text(&answer, "failing_status").as_deref(), Some("blocked"));
    assert_eq!(int(&answer, "letter_cap"), Some(blocked as i64));
    assert_no_rim(&answer, "cap_not_admissible");
    let omega = [Q5::int(2), Q5::int(0), Q5::int(0)];
    let grip = s.record_letter(3).1.grip;
    let off_menu = Twist::new(&s.ctx, grip, twist::cayley_matrix(&s.ctx.model.poles[grip], &omega, &s.ctx.model.nn))
        .expect("an exact rotation fixing the grip's pole");
    assert!(!s.menu.contains(&s.ctx, &off_menu), "the Cayley rotation is off the menu");
    let off_menu = ([grip as i64, 0], off_menu);
    let answer = cuts::release_check(&s.ctx, &start, &off_menu, None, None).expect("no engine error");
    assert_eq!(text(&answer, "failing_status").as_deref(), Some("invalid"));
    assert_no_rim(&answer, "letter_refused");
    // A budget of zero exact evaluations leaves the admissible letter of record 3 uncertain.
    let answer = cuts::release_check(&s.ctx, &start, &s.record_letter(3), None, Some(0)).expect("no engine error");
    assert_eq!(text(&answer, "failing_status").as_deref(), Some("uncertain"));
    assert_no_rim(&answer, "uncertain_under_budget");
    assert_eq!(start.record(&s.ctx), before);
}

#[test]
fn release_check_refuses_a_cap_outside_the_model() {
    let s = Setup::load();
    let start = s.after(1);
    let letter = s.record_letter(1);
    for cap in [600, -1] {
        let answer = cuts::release_check(&s.ctx, &start, &letter, Some(cap), None).expect("no engine error");
        assert_invalid(&answer, "cap", &[cap]);
        assert!(answer.get("rim_before").is_none(), "a refusal reads nothing");
    }
    // A named cap supplies delta-u as in the word check.
    let answer = cuts::release_check(&s.ctx, &start, &letter, Some(17), None).expect("no engine error");
    let word = observe::word_check(&s.ctx, &start, std::slice::from_ref(&letter), None, Some(17), None)
        .expect("no engine error");
    for key in ["cap", "u_before", "u_after", "delta_u"] {
        assert_eq!(answer.get(key), word.get(key), "{key}");
    }
    assert_eq!(int(&answer, "cap"), Some(17));
}

fn ints(value: &Value, key: &str) -> Vec<i64> {
    value.get(key).and_then(Value::as_array).expect(key).iter().map(|v| v.as_i64().expect("an integer")).collect()
}

fn answers(answer: &Value) -> &[Value] {
    answer.get("answers").and_then(Value::as_array).expect("answers")
}

/// A cover table that is not completed: its status and message, every meeting cap in the domain and no answers.
fn assert_no_answers(answer: &Value, status: &str, message: &str, by: Option<&str>) {
    assert_eq!(text(answer, "status").as_deref(), Some(status));
    assert_eq!(text(answer, "message").as_deref(), Some(message));
    assert_eq!(text(answer, "by").as_deref(), by);
    assert_eq!(ints(answer, "domain").len(), 80);
    assert!(answers(answer).is_empty(), "{message}: no answer about any meeting cap");
}

#[test]
fn covers_refuses_a_cap_outside_the_model_and_answers_no_blocked_or_closed_cap() {
    let s = Setup::load();
    let mut state = s.after(3);
    let before = state.record(&s.ctx);
    for cap in [600, -1] {
        let answer = cuts::covers(&s.ctx, &mut state, cap, None).expect("no engine error");
        assert_invalid(&answer, "cap", &[cap]);
        assert!(answer.get("answers").is_none(), "a refusal reads nothing");
    }
    let grips: Vec<usize> = (0..POLES).collect();
    let statuses = state.survey(&s.ctx, &grips, None).expect("no engine error");
    let blocked = statuses.iter().position(|&status| status == Status::Blocked).expect("a blocked cap");
    let answer = cuts::covers(&s.ctx, &mut state, blocked as i64, None).expect("no engine error");
    assert_no_answers(&answer, "error", &format!("cap {blocked}: blocked"), None);
    assert_eq!(state.record(&s.ctx), before);
    // Every cap of the solved state is closed.
    let mut solved = State::new(Some(s.menu.clone()));
    let answer = cuts::covers(&s.ctx, &mut solved, 7, None).expect("no engine error");
    assert_no_answers(&answer, "error", "cap 7: closed (not misaligned)", None);
}

#[test]
fn covers_answers_every_meeting_cap_in_both_directions() {
    let s = Setup::load();
    let mut state = s.after(3);
    let before = state.record(&s.ctx);
    let summary = cuts::rim_summary(&s.ctx, &mut state, None, None).expect("no engine error");
    let rim: Vec<&Value> = rows(&summary).iter().collect();
    let c = rim.iter().position(|row| int(row, "N_c").is_some_and(|n| n > 0)).expect("a misaligned cap");
    let answer = cuts::covers(&s.ctx, &mut state, c as i64, None).expect("no engine error");
    assert_eq!(text(&answer, "status").as_deref(), Some("completed"), "{:?}", text(&answer, "message"));
    assert_eq!((answer.get("message"), answer.get("by")), (Some(&Value::Null), Some(&Value::Null)));
    assert_eq!(text(&answer, "input_digest"), Some(state.digest(&s.ctx)));
    let parameters = answer.get("parameters").expect("parameters");
    assert_eq!((int(parameters, "cap"), int(parameters, "threshold")), (Some(c as i64), Some(10)));
    assert_eq!(text(parameters, "menu_identity"), Some(s.menu.identity.clone()));
    assert_eq!(text(parameters, "menu_order_digest"), Some(s.menu.order.clone()));
    // The domain is the 80 other caps meeting c, in cap-ID order, and the answers follow it with nothing added.
    let domain = ints(&answer, "domain");
    assert_eq!(domain.len(), 80);
    assert!(domain.windows(2).all(|w| w[0] < w[1]) && !domain.contains(&(c as i64)));
    let caps: Vec<i64> = answers(&answer).iter().map(|row| int(row, "cap").unwrap()).collect();
    assert_eq!(caps, domain);
    let covers_flag = |cover: &Value| {
        let counts = cover.get("counts").expect("counts");
        let n: Vec<i64> = ["inside", "outside", "aligned_inside", "aligned_outside"]
            .iter()
            .map(|key| int(counts, key).expect("a count"))
            .collect();
        n[0] >= 10 && n.iter().all(|&x| x == n[0])
    };
    let (mut blocked, mut answered, mut closed) = (0, 0, 0);
    let mut reverse = None;
    for row in answers(&answer) {
        let d = int(row, "cap").unwrap() as usize;
        let (d_covers_c, c_covers_d) = (row.get("d_covers_c").unwrap(), row.get("c_covers_d").unwrap());
        assert_eq!(d_covers_c.get("covers"), Some(&Value::Bool(covers_flag(d_covers_c))), "cap {d}");
        let is_blocked = text(rim[d], "status").as_deref() == Some("blocked");
        if !is_blocked {
            assert!(ints(d_covers_c, "straddlers").is_empty(), "cap {d}: only a blocked cap has straddlers");
        }
        // "c covers d" is not applicable at a blocked or closed d, and answered with counts otherwise.
        let reason = text(c_covers_d, "reason");
        if is_blocked {
            blocked += 1;
            assert_eq!(reason.as_deref(), Some("blocked"), "cap {d}");
        } else if int(rim[d], "N_c") == Some(0) {
            closed += 1;
            assert_eq!(reason.as_deref(), Some("closed"), "cap {d}");
        } else {
            answered += 1;
            assert_eq!(reason, None, "cap {d}");
            assert_eq!(c_covers_d.get("covers"), Some(&Value::Bool(covers_flag(c_covers_d))), "cap {d}");
            assert!(ints(c_covers_d, "straddlers").is_empty(), "cap {d}: c is not blocked");
            reverse.get_or_insert((d, c_covers_d.clone()));
        }
        if reason.is_some() {
            assert_eq!(text(c_covers_d, "status").as_deref(), Some("not_applicable"), "cap {d}");
        }
    }
    assert!(blocked > 0 && answered > 0 && closed > 0, "{blocked} blocked, {answered} answered, {closed} closed");
    // "c covers d" in c's table is "d covers c" in d's table: the same face of d, the same inside set of c.
    let (d, c_covers_d) = reverse.expect("an answered meeting cap");
    let table = cuts::covers(&s.ctx, &mut state, d as i64, None).expect("no engine error");
    let row = answers(&table).iter().find(|row| int(row, "cap") == Some(c as i64)).expect("c meets d");
    assert_eq!(row.get("d_covers_c"), Some(&c_covers_d));
    assert_eq!(state.record(&s.ctx), before);
}

#[test]
fn covers_is_cut_off_when_the_classification_budget_ends() {
    let s = Setup::load();
    let mut state = s.after(3);
    let before = state.record(&s.ctx);
    let summary = cuts::rim_summary(&s.ctx, &mut state, None, None).expect("no engine error");
    let c = rows(&summary).iter().position(|row| int(row, "N_c").is_some_and(|n| n > 0)).expect("a misaligned cap");
    // KN7.4 and KN7.10: a budget of zero exact evaluations leaves the named cap uncertain, so the job makes no claim.
    let answer = cuts::covers(&s.ctx, &mut state, c as i64, Some(0)).expect("no engine error");
    let message = format!("cap {c}: the classification budget ends before the cap is classified");
    assert_no_answers(&answer, "cut_off", &message, Some("KN7:budget.kernel_cost"));
    assert_eq!(state.record(&s.ctx), before);
}

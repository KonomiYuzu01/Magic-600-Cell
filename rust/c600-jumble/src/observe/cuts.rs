//! The JI.6 and JI.7 observations that M5 left to M8 (LP2 plan, section 11): the rim summary of every cap and the
//! release check of one letter. Each call reads a jumbled state exactly and changes nothing.
//!
//! The rim of an admissible cap c follows rim.py's `rim_sets`: in every domain whose frame carries n_c to a pole e,
//! the cut cells at e that the domain holds on at least one side ([`super::cap_cells`]). A cell's upper chamber is
//! held by an inside piece and its lower chamber by an outside piece or the core, so In_c counts the cells whose
//! upper side is held, Out_c those whose lower side is held, A_c those held on both sides, N_c = In_c + Out_c − 2·A_c
//! the mismatched ones, and the floor is F_c = |In_c − Out_c|. A blocked cap has no rim and an uncertain one (the
//! classification budget ran out) claims none: their fields are undefined, never zero.

use super::*;

/// The rim counts of one admissible cap.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Rim {
    n_in: i64,
    n_out: i64,
    aligned: i64,
}

/// The rim fields of a cap, in the order every answer lists them.
const RIM_FIELDS: [&str; 6] = ["N_c", "In_c", "Out_c", "A_c", "F_c", "at_floor"];

impl Rim {
    /// Counts one cut cell by its occupancy (0 both sides held, 1 only the lower, 2 only the upper).
    fn add(&mut self, occ: u8) {
        self.n_in += (occ != 1) as i64;
        self.n_out += (occ != 2) as i64;
        self.aligned += (occ == 0) as i64;
    }

    fn n(&self) -> i64 {
        self.n_in + self.n_out - 2 * self.aligned
    }

    fn floor(&self) -> i64 {
        (self.n_in - self.n_out).abs()
    }

    fn fields(&self) -> Vec<(&'static str, Value)> {
        vec![
            ("N_c", int(self.n())),
            ("In_c", int(self.n_in)),
            ("Out_c", int(self.n_out)),
            ("A_c", int(self.aligned)),
            ("F_c", int(self.floor())),
            ("at_floor", Value::Bool(self.n() == self.floor())),
        ]
    }

    fn undefined() -> Vec<(&'static str, Value)> {
        RIM_FIELDS.iter().map(|&name| (name, Value::Null)).collect()
    }
}

/// The rims of every cap in one scan of the occupied cut cells: a cell of domain d at pole e lies on the cut of cap
/// c when d's frame carries n_e to n_c (the lattice domain's frame is the identity). Only an admissible cap's rim is
/// read, and it equals the count of that cap's [`cap_cells`].
fn all_rims(ctx: &Context, t: &Tables, dom: &Domains) -> Vec<Rim> {
    let mut rims = vec![Rim::default(); POLES];
    let mut cap_of: HashMap<(u32, u16), Option<usize>> = HashMap::new();
    seam_scan(t, dom, |d, ti, occ| {
        let e = t.pole[ti as usize];
        let c = *cap_of.entry((d, e)).or_insert_with(|| {
            if d == 0 {
                Some(e as usize)
            } else {
                ctx.model.pole_of(&matvec(&dom.reps[d as usize], &ctx.model.poles[e as usize]))
            }
        });
        if let Some(c) = c {
            rims[c].add(occ);
        }
    });
    rims
}

/// JI.6 `rim_summary`: every cap, or the named caps ascending and without repeats, each with its status (J1's
/// `observations.observe`: admissible, blocked or uncertain), whether it is blocked, and its rim when admissible.
/// The answer carries grip N, the sum of N_c over the admissible caps, only when all 600 caps are summarised and none
/// is uncertain; otherwise grip N is undefined with its reason, so a partial sum is never labelled grip N. No menu
/// outcome is evaluated. A named cap outside 0..599, or an empty selection, is refused (JI.45) and nothing is read.
pub fn rim_summary(
    ctx: &Context,
    state: &mut State,
    caps: Option<&[i64]>,
    eval_budget: Option<u64>,
) -> Result<Value, String> {
    let t = tables(ctx)?;
    let selected: Vec<usize> = match caps {
        None => (0..POLES).collect(),
        Some(named) => {
            if named.is_empty() || named.iter().any(|&c| !(0..POLES as i64).contains(&c)) {
                let condition = "a selection names at least one cap, each a pole ID in 0..599";
                return Ok(invalid_object("caps", named, condition));
            }
            let mut set: Vec<usize> = named.iter().map(|&c| c as usize).collect();
            set.sort_unstable();
            set.dedup();
            set
        }
    };
    let facts = pose_facts(ctx, t, state)?;
    let dom = domains(ctx, t, state, &facts)?;
    let rims = all_rims(ctx, t, &dom);
    let (mut rows, mut grip_n, mut uncertain) = (Vec::with_capacity(selected.len()), 0i64, 0i64);
    for &c in &selected {
        let status = state.classify(ctx, c, false, true, eval_budget).map_err(|e| e.0)?.status();
        let mut row = vec![("cap", int(c as i64)), ("status", text(status.as_str()))];
        match status {
            Status::Admissible => {
                row.push(("blocked", Value::Bool(false)));
                row.extend(rims[c].fields());
                row.push(("undefined", Value::Null));
                grip_n += rims[c].n();
            }
            Status::Blocked => {
                row.push(("blocked", Value::Bool(true)));
                row.extend(Rim::undefined());
                row.push(("undefined", text("cap_not_admissible")));
            }
            Status::Uncertain => {
                uncertain += 1;
                row.push(("blocked", Value::Null));
                row.extend(Rim::undefined());
                row.push(("undefined", text("uncertain_under_budget")));
            }
            other => return Err(format!("cap {c}: a classification has status {}", other.as_str())),
        }
        rows.push(obj(row));
    }
    let grip_n_undefined = if selected.len() < POLES {
        Some("caps_selected")
    } else if uncertain > 0 {
        Some("uncertain_caps")
    } else {
        None
    };
    Ok(obj(vec![
        ("status", text("ok")),
        ("digest", text(&state.digest(ctx))),
        ("rows", Value::Array(rows)),
        ("complete", Value::Bool(uncertain == 0)),
        ("uncertain_caps", int(uncertain)),
        ("grip_n", if grip_n_undefined.is_none() { int(grip_n) } else { Value::Null }),
        ("grip_n_undefined", grip_n_undefined.map_or(Value::Null, text)),
    ]))
}

/// The rim of cap c on a state, counted over [`cap_cells`]; it is the cap's rim when c is admissible there.
fn cap_rim(ctx: &Context, t: &Tables, state: &State, c: usize) -> Result<Rim, String> {
    let facts = pose_facts(ctx, t, state)?;
    let dom = domains(ctx, t, state, &facts)?;
    let mut rim = Rim::default();
    for (_, _, occ) in cap_cells(ctx, t, &dom, c) {
        rim.add(occ);
    }
    Ok(rim)
}

/// JI.6 `release_check`: the one-letter [`word_check`] of a released letter, whose fields it carries unchanged
/// (JI.11), plus the rim of the letter's cap (its grip) before and after the letter, the cap's floor and the change
/// of N_c. The optional `cap` names the cap whose straddler count supplies delta-u, as in the word check. A refused
/// letter (its cap blocked, or uncertain under the budget) leaves the end state and both rims undefined, never zero,
/// with the reason in `rim_undefined`. A twist at an admissible cap moves its inside pieces rigidly, so the cap stays
/// admissible and its floor F_c is the same after the letter. A `cap` outside 0..599 is refused (JI.45) and nothing is
/// read; the start state never changes.
pub fn release_check(
    ctx: &Context,
    start: &State,
    letter: &([i64; 2], Twist),
    cap: Option<i64>,
    eval_budget: Option<u64>,
) -> Result<Value, String> {
    let checked = word_check(ctx, start, std::slice::from_ref(letter), None, cap, eval_budget)?;
    let Value::Object(mut fields) = checked else {
        return Err("a word check answers with an object".into());
    };
    if fields.get("status").and_then(Value::as_str) != Some("ok") {
        return Ok(Value::Object(fields));
    }
    let twist = &letter.1;
    let c = twist.grip;
    let rim: Vec<(&str, Value)> = match fields.get("failing_status").and_then(Value::as_str) {
        None => {
            let t = tables(ctx)?;
            let before = cap_rim(ctx, t, start, c)?;
            let mut end = start.clone();
            let outcome = end.apply(ctx, twist, eval_budget, &[]).map_err(|e| e.0)?;
            if outcome.status != Status::Admissible {
                let status = outcome.status.as_str();
                return Err(format!("cap {c}: the admissible letter is {status} on a second application"));
            }
            let after = cap_rim(ctx, t, &end, c)?;
            if after.floor() != before.floor() {
                return Err(format!("cap {c}: the floor changed under a twist at the cap"));
            }
            vec![
                ("rim_before", obj(before.fields())),
                ("rim_after", obj(after.fields())),
                ("floor", int(before.floor())),
                ("delta_N_c", int(after.n() - before.n())),
                ("rim_undefined", Value::Null),
            ]
        }
        Some(status) => {
            let reason = match status {
                "blocked" => "cap_not_admissible",
                "uncertain" => "uncertain_under_budget",
                _ => "letter_refused",
            };
            vec![
                ("rim_before", Value::Null),
                ("rim_after", Value::Null),
                ("floor", Value::Null),
                ("delta_N_c", Value::Null),
                ("rim_undefined", text(reason)),
            ]
        }
    };
    fields.insert("letter_cap".to_owned(), int(c as i64));
    fields.extend(rim.into_iter().map(|(name, value)| (name.to_owned(), value)));
    Ok(Value::Object(fields))
}

//! The JI.6 and JI.7 observations that M5 left to M8 (LP2 plan, section 11): the rim summary of every cap, the
//! release check of one letter and the cover table of a named cap (JI.12). Each call reads a jumbled state exactly
//! and changes nothing.
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

// ------------------------------------------------------------------------------------------------ covers (JI.12)

/// A cover needs at least this many face pieces on each side (JI.12; J1's `THRESHOLD`).
const THRESHOLD: i64 = 10;

/// The number of caps that meet a cap: the 81st largest product n_0·n_x (the cap itself is the first) bounds them.
const MEETING: usize = 80;

/// KN7.10: a job whose declared allowance cannot admit further work is cut off with this budget's code.
const KERNEL_COST: &str = "KN7:budget.kernel_cost";

/// The caps meeting cap c (J1's `meeting_caps`), in cap-ID order: every other cap d whose product n_c·n_d is at least
/// the 81st largest product n_0·n_x, decided in Q(√5). Every cap meets 80.
fn meeting_caps(ctx: &Context, c: usize) -> Result<Vec<usize>, String> {
    let poles = &ctx.model.poles;
    let mut dots: Vec<Q5> = poles.iter().map(|n| dot(&poles[0], n)).collect();
    dots.sort_by(|x, y| cmp_q5(y, x));
    let threshold = &dots[MEETING];
    let caps: Vec<usize> =
        (0..POLES).filter(|&d| d != c && cmp_q5(&dot(&poles[c], &poles[d]), threshold) != Ordering::Less).collect();
    if caps.len() != MEETING {
        return Err(format!("cap {c} meets {} caps, not {MEETING}", caps.len()));
    }
    Ok(caps)
}

/// The piece of every domain's chamber: (domain, chamber) to piece.
fn holders(dom: &Domains) -> HashMap<(u32, u32), u32> {
    let mut holder = HashMap::with_capacity(NP);
    for (d, members) in dom.members.iter().enumerate() {
        for &p in members {
            holder.insert((d as u32, dom.cham[p as usize]), p);
        }
    }
    holder
}

/// One face piece of a cut (J1's `seams.face`): the piece, whether it holds its cell's upper side (inside the cap) or
/// its lower side (outside), and whether its domain holds the cell on both sides.
struct FacePiece {
    piece: u32,
    inside: bool,
    aligned: bool,
}

/// The face of an admissible cap from its cut cells: the holder of every held side of every cell. The core holds a
/// lattice cell's lower side and is no piece.
fn face(t: &Tables, holder: &HashMap<(u32, u32), u32>, cells: &[(u32, u32, u8)]) -> Result<Vec<FacePiece>, String> {
    let held = |d: u32, chamber: u32| {
        holder.get(&(d, chamber)).copied().ok_or_else(|| format!("no piece of domain {d} holds chamber {chamber}"))
    };
    let mut face = Vec::new();
    for &(d, ti, occ) in cells {
        let (lo, up) = (t.lo[ti as usize], t.up[ti as usize]);
        if occ != 1 {
            face.push(FacePiece { piece: held(d, up)?, inside: true, aligned: occ == 0 });
        }
        if occ != 2 && lo != CORE {
            face.push(FacePiece { piece: held(d, lo)?, inside: false, aligned: occ == 0 });
        }
    }
    Ok(face)
}

/// One direction of a cover (J1's `exact_covers`): the face pieces of the covered cap's cut that lie inside the
/// covering cap, counted by side and by alignment, and the face pieces among the covering cap's straddlers (a
/// straddler is not inside). The covering cap covers when the four counts are equal and at least [`THRESHOLD`].
fn cover_answer(face: &[FacePiece], inside: &[u32], straddlers: &[u32]) -> Value {
    let (mut n_in, mut n_out, mut a_in, mut a_out) = (0i64, 0i64, 0i64, 0i64);
    for f in face.iter().filter(|f| inside.binary_search(&f.piece).is_ok()) {
        if f.inside {
            n_in += 1;
            a_in += f.aligned as i64;
        } else {
            n_out += 1;
            a_out += f.aligned as i64;
        }
    }
    let mut on_face: Vec<u32> = face.iter().map(|f| f.piece).filter(|p| straddlers.binary_search(p).is_ok()).collect();
    on_face.sort_unstable();
    let counts = vec![
        ("inside", int(n_in)),
        ("outside", int(n_out)),
        ("aligned_inside", int(a_in)),
        ("aligned_outside", int(a_out)),
    ];
    obj(vec![
        ("covers", Value::Bool(n_in >= THRESHOLD && n_in == n_out && n_out == a_in && a_in == a_out)),
        ("counts", obj(counts)),
        ("straddlers", ids(&on_face)),
    ])
}

/// The rim of an admissible cap from its cut cells.
fn rim_of(cells: &[(u32, u32, u8)]) -> Rim {
    let mut rim = Rim::default();
    for &(_, _, occ) in cells {
        rim.add(occ);
    }
    rim
}

/// JI.12 `covers` (J1's `covers_query`): every cap d meeting the named cap c, in cap-ID order, with the exact answer in
/// both directions. "d covers c" counts the face pieces of c's cut that lie inside d, and names the face pieces that
/// straddle d when d is blocked; "c covers d" counts d's face pieces inside c and is not applicable, with its reason,
/// when d is blocked or closed (N_d = 0). The named cap must be admissible and misaligned: a blocked or closed cap
/// is an error with no answers. Nothing ranks, marks or orders the caps beyond their IDs. A cap outside 0..599 is
/// refused (JI.45). When the classification budget leaves a cap uncertain, the job is cut off with
/// `KN7:budget.kernel_cost` and has no answers (KN7.4). The state never changes.
pub fn covers(ctx: &Context, state: &mut State, cap: i64, eval_budget: Option<u64>) -> Result<Value, String> {
    let c = match cap_index("cap", cap) {
        Ok(c) => c,
        Err(refused) => return Ok(refused),
    };
    let t = tables(ctx)?;
    let menu = state.menu_arc();
    let pin = |value: Option<&String>| value.map_or(Value::Null, |s| text(s));
    let parameters = obj(vec![
        ("cap", int(c as i64)),
        ("threshold", int(THRESHOLD)),
        ("menu_identity", pin(menu.as_ref().map(|m| &m.identity))),
        ("menu_order_digest", pin(menu.as_ref().map(|m| &m.order))),
    ]);
    let domain = meeting_caps(ctx, c)?;
    let digest = state.digest(ctx);
    let result = |status: &str, answers: Vec<Value>, message: Option<String>, by: Option<&str>| {
        obj(vec![
            ("status", text(status)),
            ("input_digest", text(&digest)),
            ("parameters", parameters.clone()),
            ("domain", Value::Array(domain.iter().map(|&d| int(d as i64)).collect())),
            ("answers", Value::Array(answers)),
            ("message", message.map_or(Value::Null, |m| text(&m))),
            ("by", by.map_or(Value::Null, text)),
        ])
    };
    let cut_off = |x: usize| {
        let message = format!("cap {x}: the classification budget ends before the cap is classified");
        result("cut_off", Vec::new(), Some(message), Some(KERNEL_COST))
    };
    let named = state.classify(ctx, c, true, false, eval_budget).map_err(|e| e.0)?;
    if named.uncertain > 0 {
        return Ok(cut_off(c));
    }
    if named.straddle.is_some() {
        return Ok(result("error", Vec::new(), Some(format!("cap {c}: blocked")), None));
    }
    let facts = pose_facts(ctx, t, state)?;
    let dom = domains(ctx, t, state, &facts)?;
    let cells_c = cap_cells(ctx, t, &dom, c);
    if rim_of(&cells_c).n() == 0 {
        return Ok(result("error", Vec::new(), Some(format!("cap {c}: closed (not misaligned)")), None));
    }
    let inside_c = named.inside.unwrap_or_default();
    let holder = holders(&dom);
    let face_c = face(t, &holder, &cells_c)?;
    let mut answers = Vec::with_capacity(domain.len());
    for &d in &domain {
        let cl = state.classify(ctx, d, true, false, eval_budget).map_err(|e| e.0)?;
        if cl.uncertain > 0 {
            return Ok(cut_off(d));
        }
        let blocked = cl.straddle.is_some();
        let straddlers = if blocked {
            let set = state.straddler_set(ctx, d, eval_budget).map_err(|e| e.0)?;
            if set.uncertain > 0 {
                return Ok(cut_off(d));
            }
            set.pieces
        } else {
            Vec::new()
        };
        let inside_d = cl.inside.unwrap_or_default();
        let c_covers_d = if blocked {
            obj(vec![("reason", text("blocked")), ("status", text("not_applicable"))])
        } else {
            let cells_d = cap_cells(ctx, t, &dom, d);
            if rim_of(&cells_d).n() == 0 {
                obj(vec![("reason", text("closed")), ("status", text("not_applicable"))])
            } else {
                cover_answer(&face(t, &holder, &cells_d)?, &inside_c, &[])
            }
        };
        answers.push(obj(vec![
            ("cap", int(d as i64)),
            ("d_covers_c", cover_answer(&face_c, &inside_d, &straddlers)),
            ("c_covers_d", c_covers_d),
        ]));
    }
    Ok(result("completed", answers, None, None))
}

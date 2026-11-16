//! The JI.6 and JI.7 observations that M5 left to M8 (LP2 plan, section 11): the rim summary of every cap, the
//! release check of one letter, and the cover table and the repair list of a named cap (JI.12). Each call reads a
//! jumbled state exactly and changes nothing; the repair list plays its trial twists on copies of the state.
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

// ------------------------------------------------------------------------------------------- repair list (JI.12)

/// The tie rule of a repair row's y and z (JI.12; J1's `TIE_RULE`).
const TIE_RULE: &str = "lowest value, then smaller pinned index";

/// The cut cells of pole 0 and the stabiliser of pole 0 in K⁺, which carry the cells of one domain's cut to the cells
/// they land on in another domain under a twist ([`score`]).
struct CellIndex {
    /// The four poles nearest pole 0, ascending.
    near0: [u16; 4],
    /// The index of every cut cell of pole 0, by its upper chamber.
    local0: HashMap<u32, u32>,
    /// For each element s of the stabiliser, in `stab0` order, the index of the cell s carries each cell to.
    sigma: Vec<Vec<u32>>,
    /// The position in `stab0` of the element carrying near0[0] and near0[1] to near0[j1] and near0[j2].
    pair: [[usize; 4]; 4],
}

impl CellIndex {
    /// Builds the index and checks what [`score`] relies on: every frame F_e carries pole 0 to pole e, pole 0 has
    /// exactly four nearest poles, and its stabiliser acts on them sharply 2-transitively, so an element of the
    /// stabiliser is known by the images of the first two.
    fn build(ctx: &Context, t: &Tables) -> Result<CellIndex, String> {
        let (kp, poles) = (&ctx.kplus, &ctx.model.poles);
        if (0..POLES).any(|e| kp.perms[kp.frame_idx[e] as usize][0] as usize != e) {
            return Err("a frame does not carry pole 0 to its pole".into());
        }
        let mut by_dot: Vec<(Q5, usize)> = (1..POLES).map(|e| (dot(&poles[0], &poles[e]), e)).collect();
        by_dot.sort_by(|x, y| cmp_q5(&y.0, &x.0).then(x.1.cmp(&y.1)));
        if cmp_q5(&by_dot[0].0, &by_dot[3].0) != Ordering::Equal
            || cmp_q5(&by_dot[3].0, &by_dot[4].0) != Ordering::Greater
        {
            return Err("pole 0 does not have exactly four nearest poles".into());
        }
        let near0: [u16; 4] = std::array::from_fn(|j| by_dot[j].1 as u16);
        let cells = &t.by_pole[0];
        let local0: HashMap<u32, u32> =
            cells.iter().enumerate().map(|(i, &ti)| (t.up[ti as usize], i as u32)).collect();
        if local0.len() != cells.len() {
            return Err("two cut cells of pole 0 share an upper chamber".into());
        }
        let mut sigma = Vec::with_capacity(kp.stab0.len());
        let mut pair = [[usize::MAX; 4]; 4];
        for (pos, &s) in kp.stab0.iter().enumerate() {
            let perm = &kp.perms[s as usize];
            let at = |j: usize| near0.iter().position(|&n| n == perm[near0[j] as usize]);
            let (Some(j1), Some(j2)) = (at(0), at(1)) else {
                return Err(format!("stabiliser element {s} moves a nearest pole of pole 0 away"));
            };
            if pair[j1][j2] != usize::MAX {
                return Err("the stabiliser of pole 0 is not sharply 2-transitive on its nearest poles".into());
            }
            pair[j1][j2] = pos;
            let row = cells
                .iter()
                .map(|&ti| {
                    let image = t.image(ctx, t.up[ti as usize] as usize, s)?;
                    local0
                        .get(&image)
                        .copied()
                        .ok_or_else(|| format!("stabiliser element {s} moves a cut cell of pole 0"))
                })
                .collect::<Result<Vec<u32>, String>>()?;
            sigma.push(row);
        }
        if (0..4).any(|j1| (0..4).any(|j2| j1 != j2 && pair[j1][j2] == usize::MAX)) {
            return Err("the stabiliser of pole 0 is not transitive on pairs of its nearest poles".into());
        }
        Ok(CellIndex { near0, local0, sigma, pair })
    }
}

/// One domain's side of a cap's cut (an entry of rim.py's `rim_sets`): the world images of the four poles nearest the
/// cut's pole in the domain's frame, the cells the domain holds on the upper side (inside the cap) by their index at
/// pole 0, and a mark for each cell it holds on the lower side.
struct Side {
    near: [Vec4; 4],
    ins: Vec<u32>,
    outs: Vec<bool>,
}

/// The sides of an admissible cap's cut, one for each domain with cut cells ([`cap_cells`] lists them domain by
/// domain). A cell at pole e of a domain's frame has the index of the cell at pole 0 that F_e⁻¹ carries it to.
fn sides(
    ctx: &Context,
    t: &Tables,
    ix: &CellIndex,
    dom: &Domains,
    cells: &[(u32, u32, u8)],
) -> Result<Vec<Side>, String> {
    let (kp, poles) = (&ctx.kplus, &ctx.model.poles);
    let mut sides: Vec<Side> = Vec::new();
    let mut open: Option<(u32, u16)> = None;
    for &(d, ti, occ) in cells {
        let back = match open {
            Some((current, back)) if current == d => back,
            _ => {
                let frame = kp.frame_idx[t.pole[ti as usize] as usize] as usize;
                let near = std::array::from_fn(|j| {
                    let n = &poles[kp.perms[frame][ix.near0[j] as usize] as usize];
                    if d == 0 {
                        n.clone()
                    } else {
                        matvec(&dom.reps[d as usize], n)
                    }
                });
                sides.push(Side { near, ins: Vec::new(), outs: vec![false; ix.local0.len()] });
                open = Some((d, kp.inv[frame]));
                kp.inv[frame]
            }
        };
        let up = t.up[ti as usize];
        let at0 = if back == 0 { up } else { t.image(ctx, up as usize, back)? };
        let i = *ix.local0.get(&at0).ok_or_else(|| format!("cut cell {ti} has no cell at pole 0"))?;
        let side = sides.last_mut().ok_or("a cut cell without a side")?;
        if occ != 1 {
            side.ins.push(i);
        }
        if occ != 2 {
            side.outs[i as usize] = true;
        }
    }
    Ok(sides)
}

/// What J1's `Restorer.scores` reads at a cap: blocked, uncertain under the classification budget, or open with N_c
/// and N_c after each of the cap's twists in menu order (none when N_c = 0, as in J1).
enum Score {
    Blocked,
    Uncertain,
    Open { now: i64, after: Vec<i64> },
}

/// A trial state of the repair list, with its domains once they are read.
struct Trial {
    state: State,
    dom: Option<Domains>,
}

impl Trial {
    fn new(state: State) -> Trial {
        Trial { state, dom: None }
    }
}

/// J1's `Restorer.scores(c)` on a trial state: c's status and, when c is admissible, N_c and N_c after each twist g of
/// `twists` (rim.py's `misalignment`). Under g an inside cell of a domain D lands in the domain D' = g·D on the cell
/// that k carries it to (R_D' k = g R_D, k in K⁺), and it is aligned there when D' holds that cell's lower side; so N_c
/// after g is In_c + Out_c − 2·(the inside cells landing on a held lower side), and a cell whose domain g·D has no
/// side at c lands on none. No coset is searched: k = F_e' s F_e⁻¹ with s in the stabiliser of pole 0, and g carries
/// the world images of two nearest poles of D's side to those of D' exactly when s carries them, which names s.
fn score(
    ctx: &Context,
    t: &Tables,
    ix: &CellIndex,
    trial: &mut Trial,
    c: usize,
    twists: &[Twist],
    budget: Option<u64>,
) -> Result<Score, String> {
    match trial.state.classify(ctx, c, true, true, budget).map_err(|e| e.0)?.status() {
        Status::Admissible => {}
        Status::Blocked => return Ok(Score::Blocked),
        Status::Uncertain => return Ok(Score::Uncertain),
        other => return Err(format!("cap {c}: a classification has status {}", other.as_str())),
    }
    if trial.dom.is_none() {
        let facts = pose_facts(ctx, t, &trial.state)?;
        trial.dom = Some(domains(ctx, t, &trial.state, &facts)?);
    }
    let dom = trial.dom.as_ref().ok_or("a trial state without domains")?;
    let cells = cap_cells(ctx, t, dom, c);
    let rim = rim_of(&cells);
    let now = rim.n();
    if now == 0 {
        return Ok(Score::Open { now, after: Vec::new() });
    }
    let sides = sides(ctx, t, ix, dom, &cells)?;
    let mut targets: HashMap<&Vec4, Vec<(usize, usize)>> = HashMap::new();
    for (s, side) in sides.iter().enumerate().filter(|(_, side)| side.outs.contains(&true)) {
        for (j, v) in side.near.iter().enumerate() {
            targets.entry(v).or_default().push((s, j));
        }
    }
    let identity = identity4();
    let mut after = Vec::with_capacity(twists.len());
    for g in twists {
        if *g.matrix == identity {
            after.push(now);
            continue;
        }
        let mut matches = 0i64;
        for side in sides.iter().filter(|side| !side.ins.is_empty()) {
            let Some(found) = targets.get(&matvec(&g.matrix, &side.near[0])) else {
                continue;
            };
            let u2 = matvec(&g.matrix, &side.near[1]);
            for &(s2, j1) in found {
                let target = &sides[s2];
                let Some(j2) = target.near.iter().position(|v| *v == u2) else {
                    continue;
                };
                let sigma = ix.sigma.get(ix.pair[j1][j2]).ok_or("a twist carries two nearest poles to one")?;
                matches += side.ins.iter().filter(|&&i| target.outs[sigma[i as usize] as usize]).count() as i64;
                break;
            }
        }
        after.push(rim.n_in + rim.n_out - 2 * matches);
    }
    Ok(Score::Open { now, after })
}

/// The item of lowest value, the smaller index on a tie ([`TIE_RULE`]).
fn lowest(values: &[i64]) -> Result<usize, String> {
    values.iter().enumerate().min_by_key(|&(k, &v)| (v, k)).map(|(k, _)| k).ok_or_else(|| "a cap without twists".into())
}

/// Why a repair list stopped before it completed.
enum Stop {
    /// The caller's cancellation, observed at a check.
    Cancelled,
    /// The pair budget is spent.
    Pairs,
    /// The classification budget leaves this cap uncertain.
    Uncertain(usize),
}

/// A repair list's input, its meeting caps once found, its budgets, and what it has tried so far.
struct RepairJob {
    digest: String,
    parameters: Value,
    meeting: Vec<usize>,
    max_pairs: Option<u64>,
    cancel_after: Option<u64>,
    checks: u64,
    outcomes: Vec<Value>,
    rows: Vec<Value>,
    done: Vec<Value>,
}

impl RepairJob {
    /// J1's check before an item is opened and before each later meeting cap: the cancellation first, observed at
    /// check number `cancel_after` + 1, then the pair budget.
    fn check(&mut self) -> Option<Stop> {
        self.checks += 1;
        if self.cancel_after.is_some_and(|n| self.checks > n) {
            Some(Stop::Cancelled)
        } else if self.max_pairs.is_some_and(|m| self.outcomes.len() as u64 >= m) {
            Some(Stop::Pairs)
        } else {
            None
        }
    }

    /// The answer: only a completed list has rows; the outcomes of a stopped one are coverage, never a negative claim.
    fn answer(self, status: &str, message: Option<String>, by: Option<&str>) -> Value {
        let coverage = obj(vec![
            ("pairs_tried", int(self.outcomes.len() as i64)),
            ("items_done", Value::Array(self.done)),
            ("meeting_caps", Value::Array(self.meeting.iter().map(|&d| int(d as i64)).collect())),
        ]);
        obj(vec![
            ("status", text(status)),
            ("input_digest", text(&self.digest)),
            ("parameters", self.parameters),
            ("coverage", coverage),
            ("rows", if status == "completed" { Value::Array(self.rows) } else { Value::Null }),
            ("outcomes", Value::Array(self.outcomes)),
            ("message", message.map_or(Value::Null, |m| text(&m))),
            ("by", by.map_or(Value::Null, text)),
        ])
    }

    fn stopped(self, stop: Stop) -> Value {
        match stop {
            Stop::Cancelled => self.answer("cancelled", None, None),
            Stop::Pairs => self.answer("cut_off", None, Some(KERNEL_COST)),
            Stop::Uncertain(x) => {
                let message = format!("cap {x}: the classification budget ends before the cap is classified");
                self.answer("cut_off", Some(message), Some(KERNEL_COST))
            }
        }
    }
}

/// Plays a twist on a trial state: `None` once it is applied, the stop when the classification budget leaves its cap
/// uncertain. Any other refusal is an engine error, since the cap was just read admissible on this state.
fn play(ctx: &Context, state: &mut State, twist: &Twist, budget: Option<u64>) -> Result<Option<Stop>, String> {
    let outcome = state.apply(ctx, twist, budget, &[]).map_err(|e| e.0)?;
    match outcome.status {
        Status::Admissible => Ok(None),
        Status::Uncertain => Ok(Some(Stop::Uncertain(twist.grip))),
        other => Err(format!("cap {}: a twist read admissible is {} when played", twist.grip, other.as_str())),
    }
}

/// JI.12 `repair_list` (J1's `repair_list`, ENG-0J section 4) at the named cap c, which must be admissible and
/// misaligned. Its meeting caps are the caps meeting c that are admissible and misaligned at the input. For each
/// non-identity item x in the pinned menu order, and each meeting cap d in cap-ID order, the outcome of the pair
/// (x, d) is read on copies of the state after x: y minimises N_d after x and z minimises N_c after x and y, ties to
/// the smaller pinned index; delta is the sum of the three own-cut changes, which is the exact change of N; and the
/// pair is a row (x, d, y, z, delta) when delta < 0. A pair has no delta when d is blocked or closed after x
/// (`d_blocked_or_closed`), when no item lowers N_d (`d_no_drop`), or when c is blocked or closed after x and y
/// (`c_blocked_or_closed`). Nothing chooses, ranks or scores a twist, and the input never changes.
///
/// Only a completed list has rows. Before each item is opened and before each later meeting cap, the job checks the
/// caller's cancellation (`cancel_after`: observed at check `cancel_after` + 1) and then the pair budget
/// (`max_pairs`): a cancelled or cut-off list keeps its outcomes as coverage and supplies no negative claim. A
/// classification budget that leaves a cap uncertain cuts the job off with `KN7:budget.kernel_cost` (KN7.10). A
/// blocked or closed named cap is an error with no outcomes, and so is free mode, which pins no menu order. A cap
/// outside 0..599 is refused (JI.45).
pub fn repair_list(
    ctx: &Context,
    state: &State,
    cap: i64,
    max_pairs: Option<u64>,
    cancel_after: Option<u64>,
    eval_budget: Option<u64>,
) -> Result<Value, String> {
    let c = match cap_index("cap", cap) {
        Ok(c) => c,
        Err(refused) => return Ok(refused),
    };
    let t = tables(ctx)?;
    let menu = state.menu_arc();
    let pin = |value: Option<&String>| value.map_or(Value::Null, |s| text(s));
    let parameters = obj(vec![
        ("cap", int(c as i64)),
        ("menu_identity", pin(menu.as_ref().map(|m| &m.identity))),
        ("order_digest", pin(menu.as_ref().map(|m| &m.order))),
        ("tie_rule", text(TIE_RULE)),
        ("contract_revision", text(crate::CONTRACT_REVISION)),
    ]);
    let mut job = RepairJob {
        digest: state.digest(ctx),
        parameters,
        meeting: Vec::new(),
        max_pairs,
        cancel_after,
        checks: 0,
        outcomes: Vec::new(),
        rows: Vec::new(),
        done: Vec::new(),
    };
    let Some(menu) = menu else {
        let message = "free mode pins no menu, so a repair list has no pinned order".to_owned();
        return Ok(job.answer("error", Some(message), None));
    };
    let ix = CellIndex::build(ctx, t)?;
    let twists_c = menu.for_cap(ctx, c).map_err(|e| e.to_string())?;
    let mut base = Trial::new(state.clone());
    let (now_c, after_c) = match score(ctx, t, &ix, &mut base, c, &twists_c, eval_budget)? {
        Score::Uncertain => return Ok(job.stopped(Stop::Uncertain(c))),
        Score::Blocked => return Ok(job.answer("error", Some(format!("cap {c} is blocked or uncertain")), None)),
        Score::Open { now: 0, .. } => {
            let message = format!("cap {c} is closed (N_c = 0), not misaligned");
            return Ok(job.answer("error", Some(message), None));
        }
        Score::Open { now, after } => (now, after),
    };
    let mut neighbours = Vec::new();
    for d in meeting_caps(ctx, c)? {
        match base.state.classify(ctx, d, true, true, eval_budget).map_err(|e| e.0)?.status() {
            Status::Admissible => {}
            Status::Blocked => continue,
            Status::Uncertain => return Ok(job.stopped(Stop::Uncertain(d))),
            other => return Err(format!("cap {d}: a classification has status {}", other.as_str())),
        }
        let dom = base.dom.as_ref().ok_or("the input state without domains")?;
        if rim_of(&cap_cells(ctx, t, dom, d)).n() > 0 {
            neighbours.push((d, menu.for_cap(ctx, d).map_err(|e| e.to_string())?));
        }
    }
    job.meeting = neighbours.iter().map(|(d, _)| *d).collect();
    let identity = identity4();
    let mut stop = None;
    'items: for x in (0..twists_c.len()).filter(|&x| *twists_c[x].matrix != identity) {
        if neighbours.is_empty() {
            job.done.push(int(x as i64));
            continue;
        }
        if let Some(s) = job.check() {
            stop = Some(s);
            break;
        }
        let mut sx = Trial::new(base.state.clone());
        if let Some(s) = play(ctx, &mut sx.state, &twists_c[x], eval_budget)? {
            stop = Some(s);
            break;
        }
        for (index, (d, twists_d)) in neighbours.iter().enumerate() {
            if index > 0 {
                if let Some(s) = job.check() {
                    stop = Some(s);
                    break 'items;
                }
            }
            let (delta, info) = match score(ctx, t, &ix, &mut sx, *d, twists_d, eval_budget)? {
                Score::Uncertain => {
                    stop = Some(Stop::Uncertain(*d));
                    break 'items;
                }
                Score::Blocked | Score::Open { now: 0, .. } => (None, text("d_blocked_or_closed")),
                Score::Open { now: n_d, after: a_d } => {
                    let y = lowest(&a_d)?;
                    if a_d[y] >= n_d {
                        (None, text("d_no_drop"))
                    } else {
                        let mut sxy = Trial::new(sx.state.clone());
                        if let Some(s) = play(ctx, &mut sxy.state, &twists_d[y], eval_budget)? {
                            stop = Some(s);
                            break 'items;
                        }
                        match score(ctx, t, &ix, &mut sxy, c, &twists_c, eval_budget)? {
                            Score::Uncertain => {
                                stop = Some(Stop::Uncertain(c));
                                break 'items;
                            }
                            Score::Open { now: n_c, after: a_c } if n_c > 0 => {
                                let z = lowest(&a_c)?;
                                let delta = (after_c[x] - now_c) + (a_d[y] - n_d) + (a_c[z] - n_c);
                                if delta < 0 {
                                    let row = [x as i64, *d as i64, y as i64, z as i64, delta];
                                    job.rows.push(Value::Array(row.into_iter().map(int).collect()));
                                }
                                (Some(delta), pair(y as i64, z as i64))
                            }
                            _ => (None, text("c_blocked_or_closed")),
                        }
                    }
                }
            };
            job.outcomes.push(Value::Array(vec![int(x as i64), int(*d as i64), delta.map_or(Value::Null, int), info]));
        }
        job.done.push(int(x as i64));
    }
    Ok(match stop {
        None => job.answer("completed", None, None),
        Some(stop) => job.stopped(stop),
    })
}

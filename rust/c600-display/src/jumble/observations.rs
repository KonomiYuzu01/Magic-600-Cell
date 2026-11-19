//! Producers for the jumbling payloads J07, J09, J11 and J12 (RM.34; JI.41's "Domain and lattice masks", "Rim map
//! and counts", "Covers and repair rows", "Release/word check data" and "Nu and correction-set observations"). Each
//! asks M5's and M8's engine (`c600_jumble::observe`, `observe::cuts`, `nu` and `corrections`) about one state and
//! copies its exact answer, bound to that state's pose digest. Like J08's seam table they are payloads, not supported
//! rows: no plan for them is produced or published, and the fixture report keeps them unsupported (RM.15). They rank,
//! filter, select, default and execute nothing. An answer that did not complete, or that lacks a field, is a
//! [`Missing`] value, never a shortened or filled-in payload.

use std::collections::BTreeMap;

use c600_jumble::corrections;
use c600_jumble::levels::{Base, Partial as Exact, ReachKind};
use c600_jumble::model::NP;
use c600_jumble::nu;
use c600_jumble::observe::{self, cuts, Batch, Rows, Scope};
use c600_jumble::state::State;
use c600_jumble::twist::Twist;
use c600_jumble::Context as Engine;
use c600_kernel::json::Value;

use super::{seam_table, Missing};
use crate::crosswalk::{self, Catalogue, Correction, CoverTable, NuObservation, ReleaseLog, RepairList};
use crate::error::Error;
use crate::pick::Partial;

// ------------------------------------------------------------------------------------------------ answer fields

fn field<'a>(answer: &'a Value, row: &'static str, name: &'static str) -> Result<&'a Value, Missing> {
    answer.get(name).ok_or(Missing::Malformed { row, field: name })
}

fn text<'a>(answer: &'a Value, row: &'static str, name: &'static str) -> Result<&'a str, Missing> {
    field(answer, row, name)?.as_str().ok_or(Missing::Malformed { row, field: name })
}

fn int(answer: &Value, row: &'static str, name: &'static str) -> Result<i64, Missing> {
    field(answer, row, name)?.as_i64().ok_or(Missing::Malformed { row, field: name })
}

/// An integer field that may be `null`.
fn opt_int(answer: &Value, row: &'static str, name: &'static str) -> Result<Option<i64>, Missing> {
    match field(answer, row, name)? {
        Value::Null => Ok(None),
        value => value.as_i64().map(Some).ok_or(Missing::Malformed { row, field: name }),
    }
}

/// A text field that may be `null`.
fn opt_text(answer: &Value, row: &'static str, name: &'static str) -> Result<Option<String>, Missing> {
    match field(answer, row, name)? {
        Value::Null => Ok(None),
        value => value.as_str().map(|s| Some(s.to_owned())).ok_or(Missing::Malformed { row, field: name }),
    }
}

fn array<'a>(answer: &'a Value, row: &'static str, name: &'static str) -> Result<&'a [Value], Missing> {
    field(answer, row, name)?.as_array().ok_or(Missing::Malformed { row, field: name })
}

/// The answer's status must be `complete`; any other status (`cut_off`, `cancelled`, `error`, a refusal) is
/// [`Missing::Incomplete`].
fn completed(answer: &Value, row: &'static str, complete: &str) -> Result<(), Missing> {
    match answer.get("status").and_then(Value::as_str) {
        Some(status) if status == complete => Ok(()),
        Some(status) => Err(Missing::Incomplete { row, status: status.to_owned() }),
        None => Err(Missing::Malformed { row, field: "status" }),
    }
}

/// A cap or piece ID below `bound`.
fn id(value: &Value, row: &'static str, name: &'static str, bound: usize) -> Result<u32, Missing> {
    value
        .as_i64()
        .and_then(|v| u32::try_from(v).ok())
        .filter(|&v| (v as usize) < bound)
        .ok_or(Missing::Malformed { row, field: name })
}

// ----------------------------------------------------------------------------------------------- J07: masks

/// A piece mask of J07 (JI.41 "Domain and lattice masks"): exactly one bit per piece, 177,120 bits, bound to the
/// pose digest of the state it was read on. Membership comes from the engine's exact answer, never from floats or a
/// pose index of another revision.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PieceMask {
    pub input: String,
    words: Vec<u64>,
}

impl PieceMask {
    /// The number of bits: one per piece of the model.
    pub const BITS: usize = NP;

    fn empty(input: &str) -> PieceMask {
        PieceMask { input: input.to_owned(), words: vec![0; NP.div_ceil(64)] }
    }

    fn insert(&mut self, piece: u32) {
        self.words[piece as usize / 64] |= 1 << (piece % 64);
    }

    pub fn contains(&self, piece: u32) -> bool {
        (piece as usize) < NP && self.words[piece as usize / 64] & (1 << (piece % 64)) != 0
    }

    /// The number of pieces in the mask.
    pub fn count(&self) -> u64 {
        self.words.iter().map(|w| u64::from(w.count_ones())).sum()
    }

    /// The bits, piece p at bit p % 64 of word p / 64; the bits past piece 177,119 are 0.
    pub fn words(&self) -> &[u64] {
        &self.words
    }
}

/// The domain and lattice masks of one state (J07): the pieces on the lattice, and the pieces of each exact domain
/// (JI.53's coset key), in domain-ID order. Only domains that hold a piece are listed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DomainMasks {
    pub input: String,
    /// The ID of the lattice domain, the domain of every pose in K⁺.
    pub lattice_domain: String,
    /// The pieces whose pose is in K⁺.
    pub lattice: PieceMask,
    pub domains: Vec<(String, PieceMask)>,
}

/// J07's domain and lattice masks of `state`, from the engine's `units` (JI.9: every piece off the lattice, grouped
/// by exact pose, with its pose's domain) and `heights` (the lattice domain's ID and the off count). A piece no unit
/// names is on the lattice, in the lattice domain; the off count of `heights` must agree with the units, or the
/// answer is [`Missing::Malformed`].
pub fn domain_masks(engine: &Engine, state: &State) -> Result<DomainMasks, Missing> {
    const ROW: &str = "J07";
    let input = state.digest(engine);
    let units = observe::units(engine, state, None).map_err(Missing::Engine)?;
    completed(&units, ROW, "ok")?;
    if units.get("complete") != Some(&Value::Bool(true)) {
        return Err(Missing::Incomplete { row: ROW, status: "incomplete".to_owned() });
    }
    let heights = observe::heights(engine, state).map_err(Missing::Engine)?;
    let lattice_domain = text(&heights, ROW, "lattice_key")?.to_owned();
    let mut off = PieceMask::empty(&input);
    let mut domains: BTreeMap<String, PieceMask> = BTreeMap::new();
    for unit in array(&units, ROW, "units")? {
        let domain = text(unit, ROW, "domain")?;
        let mask = domains.entry(domain.to_owned()).or_insert_with(|| PieceMask::empty(&input));
        for member in array(unit, ROW, "members")? {
            let piece = id(member, ROW, "members", NP)?;
            if off.contains(piece) {
                return Err(Missing::Malformed { row: ROW, field: "members" });
            }
            off.insert(piece);
            mask.insert(piece);
        }
    }
    if int(&heights, ROW, "off")? != off.count() as i64 {
        return Err(Missing::Malformed { row: ROW, field: "off" });
    }
    let mut lattice = PieceMask::empty(&input);
    for piece in (0..NP as u32).filter(|&piece| !off.contains(piece)) {
        lattice.insert(piece);
    }
    if lattice.count() > 0 {
        let mask = domains.entry(lattice_domain.clone()).or_insert_with(|| PieceMask::empty(&input));
        for (word, &bits) in mask.words.iter_mut().zip(&lattice.words) {
            *word |= bits;
        }
    }
    Ok(DomainMasks { input, lattice_domain, lattice, domains: domains.into_iter().collect() })
}

// ------------------------------------------------------------------------------------------ J07: rims, counts

/// The rim counts of an admissible cap (JI.6 `rim_summary`): N_c, In_c, Out_c, A_c, the floor F_c and whether N_c is at
/// the floor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RimCounts {
    pub n: i64,
    pub inside: i64,
    pub outside: i64,
    pub aligned: i64,
    pub floor: i64,
    pub at_floor: bool,
}

/// One cap of the rim map (J07, JI.41 "Rim map and counts"). Only an admissible cap has a rim: a blocked cap has none
/// and an uncertain one claims none, so their counts are `None` with the engine's reason, never zero.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CapRim {
    pub cap: u32,
    pub status: String,
    /// Whether the cap is blocked; `None` when its classification is uncertain.
    pub blocked: Option<bool>,
    pub rim: Option<RimCounts>,
    /// Why the counts are undefined (`cap_not_admissible`, `uncertain_under_budget`).
    pub undefined: Option<String>,
}

/// X5's left-height counts (J07): every bin the engine keeps (`n_0`, `n_2`, `n_4`, `n_6`, `n_above_6`, and a sparse
/// odd bin where one occurs), in key order, and the largest left height above 6, `None` when no piece has one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HeightCounts {
    pub bins: Vec<(String, i64)>,
    pub max_above_6: Option<i64>,
}

impl HeightCounts {
    /// The count of one bin, 0 when the engine kept no such bin.
    pub fn bin(&self, key: &str) -> i64 {
        self.bins.iter().find(|(k, _)| k == key).map_or(0, |&(_, n)| n)
    }
}

/// The rim map and counts of one state (J07): every cap in cap-ID order, the sum of N_c over the caps (`grip_n`),
/// total N from a seam enumeration of scope `all`, and X5's counts. The sums are undefined, with the reason, when
/// their coverage is not full.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RimMap {
    pub input: String,
    pub caps: Vec<CapRim>,
    pub grip_n: Partial<i64>,
    pub total_n: Partial<u64>,
    pub heights: HeightCounts,
}

/// J07's rim map of `state`: `rim_summary` over all 600 caps, `seam_facts` of scope `all` run as `batch` for total N,
/// and `heights` for X5's counts. Caps keep the engine's cap-ID order; there is no sort, cap rank or helpful-twist
/// field. Total N is undefined unless the seam batch completed (J08's [`seam_table`]), and `grip_n` unless every cap
/// was classified. `eval_budget` bounds each classification. The state is read, never changed.
pub fn rim_map(engine: &Engine, state: &mut State, batch: &Batch, eval_budget: Option<u64>) -> Result<RimMap, Missing> {
    const ROW: &str = "J07";
    let input = state.digest(engine);
    let summary = cuts::rim_summary(engine, state, None, eval_budget).map_err(Missing::Engine)?;
    completed(&summary, ROW, "ok")?;
    if text(&summary, ROW, "digest")? != input {
        return Err(Missing::Malformed { row: ROW, field: "digest" });
    }
    let mut caps = Vec::new();
    for row in array(&summary, ROW, "rows")? {
        let blocked = match field(row, ROW, "blocked")? {
            Value::Null => None,
            Value::Bool(b) => Some(*b),
            _ => return Err(Missing::Malformed { row: ROW, field: "blocked" }),
        };
        let rim = match opt_int(row, ROW, "N_c")? {
            None => None,
            Some(n) => Some(RimCounts {
                n,
                inside: int(row, ROW, "In_c")?,
                outside: int(row, ROW, "Out_c")?,
                aligned: int(row, ROW, "A_c")?,
                floor: int(row, ROW, "F_c")?,
                at_floor: match field(row, ROW, "at_floor")? {
                    Value::Bool(b) => *b,
                    _ => return Err(Missing::Malformed { row: ROW, field: "at_floor" }),
                },
            }),
        };
        caps.push(CapRim {
            cap: id(field(row, ROW, "cap")?, ROW, "cap", c600_jumble::model::POLES)?,
            status: text(row, ROW, "status")?.to_owned(),
            blocked,
            rim,
            undefined: opt_text(row, ROW, "undefined")?,
        });
    }
    if caps.len() != c600_jumble::model::POLES || caps.iter().zip(0u32..).any(|(cap, c)| cap.cap != c) {
        return Err(Missing::Malformed { row: ROW, field: "rows" });
    }
    let grip_n = match (opt_int(&summary, ROW, "grip_n")?, opt_text(&summary, ROW, "grip_n_undefined")?) {
        (Some(n), None) => Partial::Total(n),
        (None, Some(reason)) => Partial::Undefined(reason),
        _ => return Err(Missing::Malformed { row: ROW, field: "grip_n" }),
    };
    let facts = observe::seam_facts(engine, state, Scope::All, batch, Rows::Mismatched, eval_budget)
        .map_err(Missing::Engine)?;
    let total_n = seam_table(&facts).ok_or(Missing::Malformed { row: ROW, field: "seam_facts" })?.total();
    let heights = observe::heights(engine, state).map_err(Missing::Engine)?;
    let counts = field(&heights, ROW, "height_counts")?
        .as_object()
        .ok_or(Missing::Malformed { row: ROW, field: "height_counts" })?;
    let mut bins = Vec::new();
    for (key, value) in counts.iter().filter(|(key, _)| key.as_str() != "max_above_6") {
        bins.push((key.clone(), value.as_i64().ok_or(Missing::Malformed { row: ROW, field: "height_counts" })?));
    }
    let max_above_6 = opt_int(field(&heights, ROW, "height_counts")?, ROW, "max_above_6")?;
    Ok(RimMap { input, caps, grip_n, total_n, heights: HeightCounts { bins, max_above_6 } })
}

// --------------------------------------------------------------------------------------- J09: covers, repair

/// A completed cover table of J09 (JI.12 `covers`): the [`CoverTable`] of every cap meeting the named cap, in cap-ID
/// order, and the engine's answer for each, copied in the same order (both directions, their counts and the
/// straddlers on the face). It marks, ranks and focuses nothing.
#[derive(Clone, Debug, PartialEq)]
pub struct Covers {
    pub input: String,
    pub table: CoverTable,
    pub answers: Vec<Value>,
}

/// J09's cover table at `cap` on `state`. Only a completed answer is a table: a job the classification budget cut off,
/// and an error (a blocked or closed cap), are [`Missing::Incomplete`] with the answer's status; a cap outside the
/// model is the engine's refusal, also incomplete. The state is read, never changed.
pub fn covers(engine: &Engine, state: &mut State, cap: u32, eval_budget: Option<u64>) -> Result<Covers, Missing> {
    const ROW: &str = "J09";
    let answer = cuts::covers(engine, state, i64::from(cap), eval_budget).map_err(Missing::Engine)?;
    completed(&answer, ROW, "completed")?;
    let input = text(&answer, ROW, "input_digest")?.to_owned();
    let answers = array(&answer, ROW, "answers")?.to_vec();
    let rows = answers
        .iter()
        .map(|row| id(field(row, ROW, "cap")?, ROW, "cap", c600_jumble::model::POLES))
        .collect::<Result<Vec<u32>, Missing>>()?;
    let domain = array(&answer, ROW, "domain")?;
    if domain.len() != rows.len() || domain.iter().zip(&rows).any(|(d, &r)| d.as_i64() != Some(i64::from(r))) {
        return Err(Missing::Malformed { row: ROW, field: "answers" });
    }
    let table = CoverTable::new(cap, rows).map_err(|_| Missing::Malformed { row: ROW, field: "answers" })?;
    Ok(Covers { input, table, answers })
}

/// One row of a completed repair list (J09; JI.12 `repair_list`): the item x, the meeting cap d, the items y and z,
/// and the change of N, which is negative. The owner's reading of 17 October 2026 lets the program list these pairs
/// in a neutral order for the solver to pick from; nothing here ranks or picks one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RepairRow {
    pub x: u32,
    pub d: u32,
    pub y: u32,
    pub z: u32,
    pub delta: i64,
}

/// A repair list of J09: the [`RepairList`] of RM.37 (its completed pairs (x, d), or cut off), the rows of a completed
/// list in the engine's order (pinned item order, then cap-ID order), and the number of pairs tried, which is coverage
/// and never a negative claim when the list stopped early.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Repairs {
    pub list: RepairList,
    pub rows: Vec<RepairRow>,
    pub pairs_tried: u64,
}

/// J09's repair list at `cap` on `state`, with the engine's pair budget `max_pairs`, cancellation point `cancel_after`
/// and classification budget. A completed answer is [`RepairList::Completed`]; a cut-off or cancelled one is
/// [`RepairList::CutOff`] with no rows, so it supplies no negative; an error (a blocked or closed cap, free mode) or a
/// refusal is [`Missing::Incomplete`].
pub fn repair_list(
    engine: &Engine,
    state: &State,
    cap: u32,
    max_pairs: Option<u64>,
    cancel_after: Option<u64>,
    eval_budget: Option<u64>,
) -> Result<Repairs, Missing> {
    const ROW: &str = "J09";
    let answer = cuts::repair_list(engine, state, i64::from(cap), max_pairs, cancel_after, eval_budget)
        .map_err(Missing::Engine)?;
    let status = text(&answer, ROW, "status")?;
    let input = text(&answer, ROW, "input_digest")?.to_owned();
    let pairs_tried = int(field(&answer, ROW, "coverage")?, ROW, "pairs_tried")?;
    let pairs_tried = u64::try_from(pairs_tried).map_err(|_| Missing::Malformed { row: ROW, field: "coverage" })?;
    match status {
        "completed" => {
            let mut rows = Vec::new();
            for row in array(&answer, ROW, "rows")? {
                let values =
                    row.as_array().filter(|v| v.len() == 5).ok_or(Missing::Malformed { row: ROW, field: "rows" })?;
                let item = |v: &Value| id(v, ROW, "rows", u32::MAX as usize);
                let delta =
                    values[4].as_i64().filter(|&d| d < 0).ok_or(Missing::Malformed { row: ROW, field: "rows" })?;
                rows.push(RepairRow {
                    x: item(&values[0])?,
                    d: id(&values[1], ROW, "rows", c600_jumble::model::POLES)?,
                    y: item(&values[2])?,
                    z: item(&values[3])?,
                    delta,
                });
            }
            let repairs = rows.iter().map(|row| (row.x, row.d)).collect();
            Ok(Repairs { list: RepairList::Completed { cap, input, repairs }, rows, pairs_tried })
        }
        "cut_off" | "cancelled" => {
            if field(&answer, ROW, "rows")? != &Value::Null {
                return Err(Missing::Malformed { row: ROW, field: "rows" });
            }
            Ok(Repairs { list: RepairList::CutOff { cap, input }, rows: Vec::new(), pairs_tried })
        }
        other => Err(Missing::Incomplete { row: ROW, status: other.to_owned() }),
    }
}

// ------------------------------------------------------------------------------------------- J11: word checks

/// A word or release check of J11 (JI.11's `WordObservation`), bound to its start digest and the word's digest. The
/// engine's answer is kept whole; the fields the shell binds on are read out. A refused letter ends the check: the
/// suffix after it is unchecked and stays uncovered, and every end-state field is undefined. The step flag is LB3.8's
/// conjunction, undefined without a named unit; it is no score and grants no permission to execute.
#[derive(Clone, Debug, PartialEq)]
pub struct WordObservation {
    pub start: String,
    pub word: String,
    pub letters: u64,
    /// The processed prefix [0, processed): the letters checked, a refused letter last.
    pub processed: u64,
    /// The first refused letter and its status.
    pub refused: Option<(u64, String)>,
    pub end: Partial<String>,
    pub step: Partial<bool>,
    pub answer: Value,
}

impl WordObservation {
    /// Keeps this completed check in `log` at `revision`, under the word's digest, after the checks tried before it.
    /// A check of another revision is not kept (J11; RM.37's [`ReleaseLog`]).
    pub fn keep(&self, log: &mut ReleaseLog, revision: u64) -> bool {
        log.check(revision, &self.word)
    }
}

fn word_observation(answer: Value) -> Result<WordObservation, Missing> {
    const ROW: &str = "J11";
    completed(&answer, ROW, "ok")?;
    let count = |name: &'static str| -> Result<u64, Missing> {
        u64::try_from(int(&answer, ROW, name)?).map_err(|_| Missing::Malformed { row: ROW, field: name })
    };
    let processed = array(&answer, ROW, "processed")?;
    let processed = match processed {
        [start, end] if start.as_i64() == Some(0) => end
            .as_i64()
            .and_then(|e| u64::try_from(e).ok())
            .ok_or(Missing::Malformed { row: ROW, field: "processed" })?,
        _ => return Err(Missing::Malformed { row: ROW, field: "processed" }),
    };
    let refused = match (opt_int(&answer, ROW, "failing_letter_index")?, opt_text(&answer, ROW, "failing_status")?) {
        (None, None) => None,
        (Some(index), Some(status)) if index >= 0 => Some((index as u64, status)),
        _ => return Err(Missing::Malformed { row: ROW, field: "failing_status" }),
    };
    let end = match (answer.get("end_digest").and_then(Value::as_str), opt_text(&answer, ROW, "end_state_undefined")?) {
        (Some(digest), None) => Partial::Total(digest.to_owned()),
        (None, Some(reason)) => Partial::Undefined(reason),
        _ => return Err(Missing::Malformed { row: ROW, field: "end_digest" }),
    };
    let step = match (field(&answer, ROW, "step_flag")?, opt_text(&answer, ROW, "step_flag_undefined")?) {
        (Value::Bool(flag), None) => Partial::Total(*flag),
        (Value::Null, Some(reason)) => Partial::Undefined(reason),
        _ => return Err(Missing::Malformed { row: ROW, field: "step_flag" }),
    };
    Ok(WordObservation {
        start: text(&answer, ROW, "start_digest")?.to_owned(),
        word: text(&answer, ROW, "word_digest")?.to_owned(),
        letters: count("letters")?,
        processed,
        refused,
        end,
        step,
        answer,
    })
}

/// J11's word check (JI.11 `word_check`) of the solver's `letters` from `start`, with the solver's optional unit and
/// cap. A unit or cap outside the model is the engine's refusal, [`Missing::Incomplete`]. `start` never changes.
pub fn word_check(
    engine: &Engine,
    start: &State,
    letters: &[([i64; 2], Twist)],
    unit: Option<&[i64]>,
    cap: Option<i64>,
    eval_budget: Option<u64>,
) -> Result<WordObservation, Missing> {
    word_observation(observe::word_check(engine, start, letters, unit, cap, eval_budget).map_err(Missing::Engine)?)
}

/// J11's release check (JI.6 `release_check`) of one released letter: its one-letter word check with the rim of the
/// letter's cap before and after, which stay in [`WordObservation::answer`].
pub fn release_check(
    engine: &Engine,
    start: &State,
    letter: &([i64; 2], Twist),
    cap: Option<i64>,
    eval_budget: Option<u64>,
) -> Result<WordObservation, Missing> {
    word_observation(cuts::release_check(engine, start, letter, cap, eval_budget).map_err(Missing::Engine)?)
}

// ------------------------------------------------------------------------------------------------ J12: nu

/// J12's correction catalogue: the shipped catalogue's 36 rows in ID order, each row's ν effect as its 45 values, over
/// ν's coordinate moduli (2 for the bits, 5 for C5_17 and C5_28). Nothing is filtered, lit or reordered by state. An
/// engine error reading the catalogue is [`Missing::Engine`].
pub fn catalogue() -> Result<Catalogue, Missing> {
    let shipped = corrections::catalogue().map_err(|error| Missing::Engine(error.0))?;
    let rows =
        shipped.rows().iter().map(|row| Correction { id: row.id(), effect: row.row().values().to_vec() }).collect();
    Catalogue::new(rows, nu::MODULI.to_vec()).map_err(|error| Missing::Engine(error.summary))
}

/// J12's ν observation of a base (KN2.4's `nu` row): the 45 values on a lattice configuration, with the nine
/// coordinates A5 fixes at 0 (`nu::I_A_FIXED`) named only when the engine's claim covers them: the state's menu is I_a
/// by its order-free identity and its claim is witnessed at that state. Off the lattice it is
/// [`NuObservation::OffLattice`], never zeros or an older value; a raw table with two pieces in one chamber is
/// [`Missing::Undefined`] with `pieces_collide`. [`crosswalk::nu`] then refuses a nonzero fixed coordinate in form
/// freeze, exactly when the engine's row refuses it.
pub fn nu_observation(engine: &Engine, base: Base) -> Result<NuObservation, Missing> {
    let (state, claim) = match base {
        Base::Snapshot(snapshot) => (snapshot.state(), snapshot.reachability()),
        Base::Derived(derived) => (derived.state(), derived.reachability()),
    };
    let claimed = state.menu().map(|menu| menu.identity.as_str()) == Some(nu::I_A_MENU_IDENTITY)
        && claim.kind() == ReachKind::Witnessed
        && claim.at().is_some_and(|at| at == state.digest(engine));
    match nu::value(engine, state).map_err(|error| Missing::Engine(error.0))? {
        Exact::Total(value) => Ok(NuObservation::Lattice {
            nu: value.values().to_vec(),
            fixed: if claimed { nu::I_A_FIXED.to_vec() } else { Vec::new() },
        }),
        Exact::Undefined(reason) if reason == nu::OFF_LATTICE => Ok(NuObservation::OffLattice),
        Exact::Undefined(reason) => Err(Missing::Undefined { row: "J12", reason }),
        Exact::Partial { .. } => Err(Missing::Malformed { row: "J12", field: "nu" }),
    }
}

/// J12's predicted ν after the caller's signed multiplicities, one per catalogue row in ID order (JI.13): ν of the
/// observation plus [`Catalogue::predict`], each coordinate modulo its own modulus. It is undefined off the lattice, as
/// ν is, and refused as ν is when a fixed coordinate is nonzero. It solves, suggests and filters nothing.
pub fn predicted(
    observation: &NuObservation,
    catalogue: &Catalogue,
    multiplicities: &[i64],
) -> Result<Partial<Vec<i64>>, Error> {
    Ok(match crosswalk::nu(observation)? {
        Partial::Total(value) => {
            if value.len() != catalogue.moduli().len() {
                return Err(Error::invalid("the ν observation and the catalogue have different widths"));
            }
            let delta = catalogue.predict(multiplicities);
            let sums = value.iter().zip(delta).zip(catalogue.moduli());
            Partial::Total(sums.map(|((&v, d), &m)| (v + d).rem_euclid(m)).collect())
        }
        other => other,
    })
}

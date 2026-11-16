//! The JSON-lines adapter of the jumbling candidate for the differential harness (CT-KN2-01, jumbling half).
//!
//! Usage: `c600-jumble-adapter <assets directory>`. After loading, the adapter writes one ready line, then answers one
//! request per line on standard input with one line on standard output: `{"ok":true,"result":...}` or
//! `{"ok":false,"error":{"kind":"input"|"engine","message":...}}`. A refusal is a result, never an error. An error is
//! an input the candidate does not read (selection 2 of the LP2 plan, section 10) or a broken invariant.
//!
//! Requests (`op`): `start` (`menu`, a menu record or null for free mode), `apply` (`step`, as the harness gives it,
//! with optional `budget` and `protect`), `survey` (`grips`, optional `budget`), `straddlers` (`caps`), `certify`
//! (`cap`, `piece`, `certificate`), `record`, `load` (`record`), `identity`, `word` (`identity`, `letters`, optional
//! `budget`), `model` and `quit`.
//!
//! Observations ([`c600_jumble::observe`]; a batch names a positive `chunk` and a positive `budget`, with an optional
//! `cancel_after` chunk count, and `eval_budget` bounds classification): `heights`; `units` (optional `pieces`, a
//! named unit); `seam_facts` (`scope` "all" or a cap, `rows` "none", "mismatched" or "all"); `seam_totals` (`cells`,
//! a supplied table); `prefix_observations` (`letters`, optional `fields` from "N", "heights", "fd" and "sets",
//! optional `cap` and `keep`); `word_check` (`letters`, optional `unit` and `cap`). Stored states: `keep` (`name`),
//! `use` (`name`) and `release` (`name`); a kept prefix state is named `prefix/<row index>`. `cayley` (`grip`,
//! `omega`) returns the record of an exact Cayley rotation for free-mode tests.

use std::collections::HashMap;
use std::io::{self, BufRead, Write};
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

use c600_jumble::observe::{self, moved_set_hash};
use c600_jumble::q5::{Vec4, Q5};
use c600_jumble::state::{Outcome, State, Status, StraddlerSet};
use c600_jumble::twist::{Menu, RecordError, Twist};
use c600_jumble::{codes, Context};
use c600_kernel::json::{self, Value};
use c600_kernel::{IdentityBlock, Puzzle};

/// SHA-256 of no bytes: the moved set of a refused step.
const EMPTY: &str = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
/// `keep` stores one state per row, so a prefix observation that keeps its states is short.
const KEEP_LIMIT: usize = 64;

enum Fail {
    Input(String),
    Engine(String),
}

fn input(message: impl Into<String>) -> Fail {
    Fail::Input(message.into())
}

struct Adapter {
    ctx: Context,
    state: Option<State>,
    cap_twists: HashMap<usize, Vec<Twist>>,
    /// Stored states by name (`keep`, kept prefix states) for `use`.
    states: HashMap<String, State>,
}

fn int(value: i64) -> Value {
    Value::Int(value)
}

fn string(text: &str) -> Value {
    Value::String(text.to_owned())
}

/// A positive integer: a chunk size or a batch budget.
fn positive(value: Option<&Value>, what: &str) -> Result<u64, Fail> {
    match value {
        Some(Value::Int(n)) if *n >= 1 => Ok(*n as u64),
        _ => Err(input(format!("{what} must be a positive integer"))),
    }
}

/// Selection 5: a batch declares a positive chunk size and a positive budget; `cancel_after` cancels the batch
/// after that many chunks (a test of the cancellation point between chunks).
fn batch(request: &Value) -> Result<observe::Batch, Fail> {
    let cancel_after = match request.get("cancel_after") {
        None | Some(Value::Null) => None,
        Some(Value::Int(n)) if *n >= 0 => Some(*n as u64),
        _ => return Err(input("cancel_after must be a non-negative integer or null")),
    };
    Ok(observe::Batch {
        chunk: positive(request.get("chunk"), "chunk")?,
        budget: Some(positive(request.get("budget"), "budget")?),
        cancel_after,
    })
}

/// Piece IDs as given (a named unit is checked by the observation, which refuses an ID outside the model).
fn pieces(value: Option<&Value>, what: &str) -> Result<Option<Vec<u32>>, Fail> {
    match value {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Array(items)) => items
            .iter()
            .map(|v| match v {
                Value::Int(n) if (0..=u32::MAX as i64).contains(n) => Ok(*n as u32),
                _ => Err(input(format!("{what} must list non-negative integers"))),
            })
            .collect::<Result<Vec<u32>, Fail>>()
            .map(Some),
        _ => Err(input(format!("{what} must be a list or null"))),
    }
}

fn optional_cap(value: Option<&Value>) -> Result<Option<usize>, Fail> {
    match value {
        None | Some(Value::Null) => Ok(None),
        cap => Ok(Some(small_int(cap, "cap", 600)? as usize)),
    }
}

fn name(request: &Value) -> Result<String, Fail> {
    match request.get("name") {
        Some(Value::String(s)) if !s.is_empty() => Ok(s.clone()),
        _ => Err(input("a stored state needs a non-empty name")),
    }
}

fn ids(values: &[u32]) -> Value {
    Value::Array(values.iter().map(|&v| int(v as i64)).collect())
}

fn small_int(value: Option<&Value>, what: &str, below: i64) -> Result<i64, Fail> {
    match value {
        Some(Value::Int(n)) if (0..below).contains(n) => Ok(*n),
        _ => Err(input(format!("{what} must be an integer in 0..{}", below - 1))),
    }
}

fn budget(value: Option<&Value>) -> Result<Option<u64>, Fail> {
    match value {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Int(n)) if *n >= 0 => Ok(Some(*n as u64)),
        _ => Err(input("budget must be a non-negative integer or null")),
    }
}

fn int_list(value: Option<&Value>, what: &str, below: i64) -> Result<Vec<i64>, Fail> {
    match value {
        None | Some(Value::Null) => Ok(Vec::new()),
        Some(Value::Array(items)) => items.iter().map(|v| small_int(Some(v), what, below)).collect(),
        _ => Err(input(format!("{what} must be a list"))),
    }
}

fn identity_json(block: &IdentityBlock) -> Value {
    let opt = |v: &Option<String>| v.as_deref().map_or(Value::Null, string);
    json::object([
        ("puzzle", string(if block.puzzle == Puzzle::Full { "full" } else { "jumbling" })),
        ("model_id", string(&block.model_id)),
        ("menu_id", opt(&block.menu_id)),
        ("menu_order", opt(&block.menu_order)),
        ("contract_rev", opt(&block.contract_rev)),
    ])
}

fn parse_identity(value: &Value) -> Result<IdentityBlock, Fail> {
    let fields = value.as_object().ok_or_else(|| input("an identity block must be a mapping"))?;
    let text = |key: &str| -> Result<Option<String>, Fail> {
        match fields.get(key) {
            None | Some(Value::Null) => Ok(None),
            Some(Value::String(s)) => Ok(Some(s.clone())),
            _ => Err(input(format!("identity field {key} must be a string or null"))),
        }
    };
    let puzzle = match text("puzzle")?.as_deref() {
        Some("full") => Puzzle::Full,
        Some("jumbling") => Puzzle::Jumbling,
        _ => return Err(input("identity puzzle must be full or jumbling")),
    };
    Ok(IdentityBlock {
        puzzle,
        model_id: text("model_id")?.ok_or_else(|| input("identity needs a model_id"))?,
        menu_id: text("menu_id")?,
        menu_order: text("menu_order")?,
        contract_rev: text("contract_rev")?,
    })
}

fn point(value: Option<&Value>) -> Option<Vec4> {
    let items = value?.as_array().filter(|items| items.len() == 4)?;
    let parsed: Vec<Q5> = items.iter().map(Q5::from_json_any).collect::<Result<_, _>>().ok()?;
    Some([parsed[0].clone(), parsed[1].clone(), parsed[2].clone(), parsed[3].clone()])
}

fn straddlers_json(set: &StraddlerSet) -> Value {
    json::object([
        ("blocked", Value::Bool(!set.pieces.is_empty())),
        ("pieces", ids(&set.pieces)),
        ("status", string(set.status())),
        ("uncertain_pieces", int(set.uncertain as i64)),
        (
            "certificates",
            Value::Array(
                set.certificates.iter().map(|c| Value::Array(vec![int(c.piece as i64), c.to_json()])).collect(),
            ),
        ),
        ("exact_evaluations", int(set.exact_evaluations as i64)),
    ])
}

impl Adapter {
    fn state(&mut self) -> Result<&mut State, Fail> {
        self.state.as_mut().ok_or_else(|| input("start must come first"))
    }

    fn step_json(&self, status: &str, moved: usize, moved_set: &str) -> Result<Value, Fail> {
        let state = self.state.as_ref().ok_or_else(|| input("start must come first"))?;
        Ok(json::object([
            ("status", string(status)),
            ("moved", int(moved as i64)),
            ("moved_set", string(moved_set)),
            ("digest", string(&state.digest(&self.ctx))),
            ("lattice", string(&state.lattice_digest())),
        ]))
    }

    fn outcome_json(&self, outcome: &Outcome) -> Result<Value, Fail> {
        let admissible = outcome.status == Status::Admissible;
        let moved_set = if admissible { moved_set_hash(&outcome.inside) } else { EMPTY.to_owned() };
        let mut result = self.step_json(outcome.status.as_str(), outcome.moved(), &moved_set)?;
        if let Value::Object(fields) = &mut result {
            if let Some((code, message)) = &outcome.refusal {
                fields.insert("code".into(), string(code));
                fields.insert("message".into(), string(message));
            }
            if let Some(protection) = &outcome.protection {
                fields.insert("protection".into(), protection.to_json());
            }
            if let Some(cert) = &outcome.straddle {
                fields.insert("certificate".into(), cert.to_json());
            }
            fields.insert("uncertain_pieces".into(), int(outcome.uncertain as i64));
            fields.insert("exact_evaluations".into(), int(outcome.exact_evaluations as i64));
        }
        Ok(result)
    }

    fn refused_json(&self, status: &str, code: Option<&str>, message: &str) -> Result<Value, Fail> {
        let mut result = self.step_json(status, 0, EMPTY)?;
        if let Value::Object(fields) = &mut result {
            fields.insert("code".into(), code.map_or(Value::Null, string));
            fields.insert("message".into(), string(message));
        }
        Ok(result)
    }

    /// The twist a harness step names; `Ok(Err(message))` for a record the reference refuses as invalid.
    fn twist(&mut self, step: &Value) -> Result<Result<Twist, String>, Fail> {
        match step.get("kind").and_then(Value::as_str) {
            Some("record") => {
                let record = step.get("record").ok_or_else(|| input("a record step needs a record"))?;
                match Twist::from_record(&self.ctx, record) {
                    Ok(twist) => Ok(Ok(twist)),
                    Err(RecordError::Invalid(message)) => Ok(Err(message)),
                    Err(RecordError::Unsupported(message)) => Err(input(message)),
                }
            }
            Some("letter") => {
                let menu = self.state()?.menu_arc().ok_or_else(|| input("a letter needs a started menu"))?;
                let grip = small_int(step.get("grip"), "letter grip", 600)? as usize;
                let item = small_int(step.get("item"), "letter item", menu.items.len() as i64)? as usize;
                if !self.cap_twists.contains_key(&grip) {
                    let twists = menu.for_cap(&self.ctx, grip).map_err(|e| Fail::Engine(e.to_string()))?;
                    self.cap_twists.insert(grip, twists);
                }
                Ok(Ok(self.cap_twists[&grip][item].clone()))
            }
            _ => Err(input("a step kind must be record or letter")),
        }
    }

    fn handle(&mut self, request: &Value) -> Result<Option<Value>, Fail> {
        let op = request.get("op").and_then(Value::as_str).ok_or_else(|| input("a request needs an op"))?;
        let result = match op {
            "quit" => return Ok(None),
            "start" => {
                let menu = match request.get("menu") {
                    None | Some(Value::Null) => None,
                    Some(record) => Some(Arc::new(Menu::from_record(&self.ctx, record).map_err(Fail::Input)?)),
                };
                self.state = Some(State::new(menu));
                self.cap_twists.clear();
                self.step_json("start", 0, EMPTY)?
            }
            "apply" => {
                // Every step needs a started state, a record step too: a refusal reports the state's digests.
                self.state()?;
                let step = request.get("step").ok_or_else(|| input("apply needs a step"))?;
                let budget = budget(request.get("budget"))?;
                let protect: Vec<u32> =
                    int_list(request.get("protect"), "a protected piece", c600_jumble::model::NP as i64)?
                        .into_iter()
                        .map(|p| p as u32)
                        .collect();
                let twist = match self.twist(step)? {
                    Ok(twist) => twist,
                    Err(message) => return Ok(Some(self.refused_json("invalid", None, &message)?)),
                };
                let Adapter { ctx, state, .. } = self;
                let state = state.as_mut().ok_or_else(|| input("start must come first"))?;
                let outcome = state.apply(ctx, &twist, budget, &protect).map_err(|e| Fail::Engine(e.0))?;
                self.outcome_json(&outcome)?
            }
            "survey" => {
                let grips: Vec<usize> =
                    int_list(request.get("grips"), "a grip", 600)?.into_iter().map(|g| g as usize).collect();
                let budget = budget(request.get("budget"))?;
                let Adapter { ctx, state, .. } = self;
                let state = state.as_mut().ok_or_else(|| input("start must come first"))?;
                let statuses = state.survey(ctx, &grips, budget).map_err(|e| Fail::Engine(e.0))?;
                let rows =
                    grips.iter().zip(statuses).map(|(&g, s)| Value::Array(vec![int(g as i64), string(s.letter())]));
                json::object([("grips", Value::Array(rows.collect()))])
            }
            "straddlers" => {
                let caps: Vec<usize> =
                    int_list(request.get("caps"), "a cap", 600)?.into_iter().map(|c| c as usize).collect();
                let budget = budget(request.get("budget"))?;
                let Adapter { ctx, state, .. } = self;
                let state = state.as_mut().ok_or_else(|| input("start must come first"))?;
                let mut rows = Vec::with_capacity(caps.len());
                for &cap in &caps {
                    let set = state.straddler_set(ctx, cap, budget).map_err(|e| Fail::Engine(e.0))?;
                    rows.push(Value::Array(vec![int(cap as i64), straddlers_json(&set)]));
                }
                json::object([("caps", Value::Array(rows))])
            }
            "certify" => {
                let cap = small_int(request.get("cap"), "cap", 600)? as usize;
                let piece = small_int(request.get("piece"), "piece", c600_jumble::model::NP as i64)? as usize;
                let cert = request.get("certificate");
                let state = self.state.as_ref().ok_or_else(|| input("start must come first"))?;
                let valid = match (
                    point(cert.and_then(|c| c.get("point_below"))),
                    point(cert.and_then(|c| c.get("point_above"))),
                ) {
                    (Some(below), Some(above)) => state.check_certificate(&self.ctx, cap, piece, &below, &above),
                    _ => false,
                };
                json::object([("valid", Value::Bool(valid))])
            }
            "record" => {
                let ctx = &self.ctx;
                self.state.as_ref().ok_or_else(|| input("start must come first"))?.record(ctx)
            }
            "load" => {
                let record = request.get("record").ok_or_else(|| input("load needs a record"))?;
                let menu = self.state()?.menu_arc();
                let loaded = State::from_record(&self.ctx, menu, record).map_err(Fail::Input)?;
                self.state = Some(loaded);
                self.step_json("start", 0, EMPTY)?
            }
            "identity" => {
                let menu = self.state()?.menu_arc();
                identity_json(&self.ctx.identity(menu.as_deref()))
            }
            "word" => self.word(request)?,
            "model" => self.model(),
            "heights" => {
                let state = self.state.as_ref().ok_or_else(|| input("start must come first"))?;
                observe::heights(&self.ctx, state).map_err(Fail::Engine)?
            }
            "units" => {
                let named = pieces(request.get("pieces"), "pieces")?;
                let state = self.state.as_ref().ok_or_else(|| input("start must come first"))?;
                observe::units(&self.ctx, state, named.as_deref()).map_err(Fail::Engine)?
            }
            "seam_facts" => {
                let scope = match request.get("scope") {
                    Some(Value::String(s)) if s == "all" => observe::Scope::All,
                    Some(cap @ Value::Int(_)) => observe::Scope::Cap(small_int(Some(cap), "scope cap", 600)? as usize),
                    _ => return Err(input("scope must be all or a cap")),
                };
                let rows = match request.get("rows") {
                    None | Some(Value::Null) => observe::Rows::None,
                    Some(Value::String(s)) if s == "none" => observe::Rows::None,
                    Some(Value::String(s)) if s == "mismatched" => observe::Rows::Mismatched,
                    Some(Value::String(s)) if s == "all" => observe::Rows::All,
                    _ => return Err(input("rows must be none, mismatched or all")),
                };
                let batch = batch(request)?;
                let eval_budget = budget(request.get("eval_budget"))?;
                let Adapter { ctx, state, .. } = self;
                let state = state.as_mut().ok_or_else(|| input("start must come first"))?;
                observe::seam_facts(ctx, state, scope, &batch, rows, eval_budget).map_err(Fail::Engine)?
            }
            "seam_totals" => {
                let cells =
                    request.get("cells").and_then(Value::as_array).ok_or_else(|| input("seam_totals needs cells"))?;
                observe::seam_totals(cells).map_err(Fail::Input)?
            }
            "prefix_observations" => self.prefix_observations(request)?,
            "word_check" => {
                let letters = self.letters(request.get("letters"))?;
                let unit = pieces(request.get("unit"), "unit")?;
                let cap = optional_cap(request.get("cap"))?;
                let eval_budget = budget(request.get("eval_budget"))?;
                let state = self.state.as_ref().ok_or_else(|| input("start must come first"))?;
                observe::word_check(&self.ctx, state, &letters, unit.as_deref(), cap, eval_budget)
                    .map_err(Fail::Engine)?
            }
            "keep" => {
                let name = name(request)?;
                let state = self.state()?.clone();
                self.states.insert(name.clone(), state);
                json::object([("kept", string(&name))])
            }
            "use" => {
                let name = name(request)?;
                let state = self.states.get(&name).ok_or_else(|| input(format!("no stored state {name}")))?.clone();
                self.state = Some(state);
                self.cap_twists.clear();
                self.step_json("start", 0, EMPTY)?
            }
            "release" => {
                let name = name(request)?;
                json::object([("released", Value::Bool(self.states.remove(&name).is_some()))])
            }
            "cayley" => {
                let grip = small_int(request.get("grip"), "grip", 600)? as usize;
                let omega = request
                    .get("omega")
                    .and_then(Value::as_array)
                    .filter(|w| w.len() == 3)
                    .ok_or_else(|| input("omega must be three numbers"))?;
                let omega: Vec<Q5> =
                    omega.iter().map(Q5::from_json_any).collect::<Result<_, _>>().map_err(Fail::Input)?;
                json::object([("record", observe::cayley_record(&self.ctx, grip, &omega))])
            }
            other => return Err(input(format!("unknown op {other}"))),
        };
        Ok(Some(result))
    }

    /// The letters of a word ([grip, item] pairs) with their twists under the started menu.
    fn letters(&mut self, value: Option<&Value>) -> Result<Vec<([i64; 2], Twist)>, Fail> {
        let items = value.and_then(Value::as_array).ok_or_else(|| input("letters must be a list of [grip, item]"))?;
        let mut out = Vec::with_capacity(items.len());
        for letter in items {
            let pair =
                letter.as_array().filter(|p| p.len() == 2).ok_or_else(|| input("a letter must be [grip, item]"))?;
            let step = json::object([("kind", string("letter")), ("grip", pair[0].clone()), ("item", pair[1].clone())]);
            let twist = self.twist(&step)?.map_err(Fail::Engine)?;
            // twist() has read both entries as small integers.
            out.push(([pair[0].as_i64().unwrap_or(-1), pair[1].as_i64().unwrap_or(-1)], twist));
        }
        Ok(out)
    }

    fn prefix_observations(&mut self, request: &Value) -> Result<Value, Fail> {
        let letters = self.letters(request.get("letters"))?;
        let fields = match request.get("fields") {
            None | Some(Value::Null) => observe::Fields { n: true, heights: true, fd: true, sets: false },
            Some(Value::Array(names)) => {
                let mut fields = observe::Fields { n: false, heights: false, fd: false, sets: false };
                for field in names {
                    match field.as_str() {
                        Some("N") => fields.n = true,
                        Some("heights") => fields.heights = true,
                        Some("fd") => fields.fd = true,
                        Some("sets") => {
                            fields.fd = true;
                            fields.sets = true;
                        }
                        _ => return Err(input("a field must be N, heights, fd or sets")),
                    }
                }
                fields
            }
            _ => return Err(input("fields must be a list")),
        };
        let keep = match request.get("keep") {
            None | Some(Value::Null) => false,
            Some(Value::Bool(keep)) => *keep,
            _ => return Err(input("keep must be a boolean")),
        };
        if keep && letters.len() > KEEP_LIMIT {
            return Err(input(format!("keep stores one state per row: at most {KEEP_LIMIT} letters")));
        }
        let req = observe::PrefixRequest {
            letters: &letters,
            fields,
            cap: optional_cap(request.get("cap"))?,
            batch: batch(request)?,
            keep,
            eval_budget: budget(request.get("eval_budget"))?,
        };
        let state = self.state.as_ref().ok_or_else(|| input("start must come first"))?;
        let (result, kept) = observe::prefix_observations(&self.ctx, state, &req).map_err(Fail::Engine)?;
        if keep {
            // The kept states are those of rows 0, 1, ... in order: the batch stops at its first refused row.
            self.states.retain(|name, _| !name.starts_with("prefix/"));
            for (index, state) in kept.into_iter().enumerate() {
                self.states.insert(format!("prefix/{index}"), state);
            }
        }
        Ok(result)
    }

    /// A stored word under its identity block: refused whole on another identity (KN1.4); otherwise applied letter
    /// by letter, and refused whole, with nothing changed, at its first refused letter. Every letter is read and
    /// resolved to its twist before any is applied, and an engine error restores the saved state, so a malformed or
    /// failing word changes nothing either.
    fn word(&mut self, request: &Value) -> Result<Value, Fail> {
        let block = parse_identity(request.get("identity").ok_or_else(|| input("a word needs its identity block"))?)?;
        let menu = self.state()?.menu_arc();
        if let Err(refusal) = self.ctx.check_identity(menu.as_deref(), &block) {
            let mut result = self.refused_json("refused", Some(refusal.code), &refusal.message)?;
            if let Value::Object(fields) = &mut result {
                fields.insert("applied".into(), int(0));
                fields.insert("refused_at".into(), Value::Null);
            }
            return Ok(result);
        }
        let letters = self.letters(request.get("letters"))?;
        let budget = budget(request.get("budget"))?;
        let saved = self.state()?.clone();
        let mut moved: Vec<u32> = Vec::new();
        for (index, (_, twist)) in letters.iter().enumerate() {
            let Adapter { ctx, state, .. } = self;
            let state = state.as_mut().ok_or_else(|| input("start must come first"))?;
            let outcome = match state.apply(ctx, twist, budget, &[]) {
                Ok(outcome) => outcome,
                Err(error) => {
                    self.state = Some(saved);
                    return Err(Fail::Engine(error.0));
                }
            };
            if outcome.status != Status::Admissible {
                self.state = Some(saved);
                let message = format!("letter {index} of the word is {}", outcome.status.as_str());
                let mut result =
                    self.refused_json(outcome.status.as_str(), Some(codes::WORD_BLOCKED_PREFIX), &message)?;
                if let Value::Object(fields) = &mut result {
                    fields.insert("applied".into(), int(0));
                    fields.insert("refused_at".into(), int(index as i64));
                }
                return Ok(result);
            }
            moved.extend_from_slice(&outcome.inside);
        }
        moved.sort_unstable();
        moved.dedup();
        let mut result = self.step_json("admissible", moved.len(), &moved_set_hash(&moved))?;
        if let Value::Object(fields) = &mut result {
            fields.insert("applied".into(), int(letters.len() as i64));
            fields.insert("refused_at".into(), Value::Null);
        }
        Ok(result)
    }

    fn model(&self) -> Value {
        let model = &self.ctx.model;
        let mut centres = vec![0i64; c600_jumble::model::POLES];
        for &p in &model.centre_pieces {
            centres[model.signature(p as usize)[0] as usize] = p as i64;
        }
        let files = Value::Object(model.identity.iter().map(|(k, v)| (k.clone(), string(v))).collect());
        let regions = Value::Object(self.ctx.regions.report.iter().map(|(k, v)| (k.clone(), string(v))).collect());
        json::object([
            ("model_id", string(&model.model_id)),
            ("files", files),
            ("regions", regions),
            ("centre_of_cell", Value::Array(centres.into_iter().map(int).collect())),
            ("contract_revision", string(c600_jumble::CONTRACT_REVISION)),
        ])
    }
}

fn error_line(kind: &str, message: &str) -> String {
    json::object([
        ("ok", Value::Bool(false)),
        ("error", json::object([("kind", string(kind)), ("message", string(message))])),
    ])
    .canonical()
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let stdout = io::stdout();
    let mut out = stdout.lock();
    if args.len() != 2 {
        let _ = writeln!(out, "{}", error_line("input", "usage: c600-jumble-adapter <assets directory>"));
        std::process::exit(2);
    }
    let started = Instant::now();
    let ctx = match Context::load(Path::new(&args[1])) {
        Ok(ctx) => ctx,
        Err(e) => {
            let _ = writeln!(out, "{}", error_line("engine", &e.0));
            std::process::exit(1);
        }
    };
    let ready = json::object([
        ("ok", Value::Bool(true)),
        (
            "result",
            json::object([
                ("ready", Value::Bool(true)),
                ("model_id", string(&ctx.model.model_id)),
                ("load_ms", int(started.elapsed().as_millis() as i64)),
            ]),
        ),
    ]);
    let _ = writeln!(out, "{}", ready.canonical());
    let _ = out.flush();
    let mut adapter = Adapter { ctx, state: None, cap_twists: HashMap::new(), states: HashMap::new() };
    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        if line.trim().is_empty() {
            continue;
        }
        let response = match json::parse(line.as_bytes()) {
            Err(e) => error_line("input", &format!("request is not JSON: {e}")),
            Ok(request) => match adapter.handle(&request) {
                Ok(None) => break,
                Ok(Some(result)) => json::object([("ok", Value::Bool(true)), ("result", result)]).canonical(),
                Err(Fail::Input(message)) => error_line("input", &message),
                Err(Fail::Engine(message)) => error_line("engine", &message),
            },
        };
        if writeln!(out, "{response}").and_then(|_| out.flush()).is_err() {
            break;
        }
    }
}

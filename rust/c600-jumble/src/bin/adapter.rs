//! The JSON-lines adapter of the jumbling candidate for the differential harness (CT-KN2-01, jumbling half).
//!
//! Usage: `c600-jumble-adapter <assets directory>`. After loading, the adapter writes one ready line, then answers one
//! request per line on standard input with one line on standard output: `{"ok":true,"result":...}` or
//! `{"ok":false,"error":{"kind":"input"|"engine","message":...}}`. A refusal is a result, never an error. An error is
//! an input the candidate does not read (selection 2 of the LP2 plan, section 11) or a broken invariant.
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
//!
//! The end tests and derived values of `c600_jumble::levels` are listed where the adapter answers them. ν (A5) is
//! `nu` and `nu_constants`, and the shipped correction catalogue is `corrections`, `correction_lookup` and
//! `correction_set_check`; both are listed at the end of this file.
//!
//! Every request, and the record that `load` reads, may name its `dimension` (JI.3). This edition admits only 4, and
//! a request or record without one means 4 (a draft: the harness's requests carry none). Any other value is refused
//! with `KN1:input.unsupported_dimension` before any other field is read, and nothing changes (JI.45).

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
    /// Derived values by name (`derive`, `import`); none becomes the current state (KN2.12).
    derived: HashMap<String, c600_jumble::levels::Derived>,
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

/// Piece IDs as given: the observation checks a named unit and refuses an ID outside the model, a negative one too,
/// with `KN2:input.named_object_invalid` (JI.45). Only a value that is no list of integers is an input error.
fn pieces(value: Option<&Value>, what: &str) -> Result<Option<Vec<i64>>, Fail> {
    match value {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Array(items)) => items
            .iter()
            .map(|v| match v {
                Value::Int(n) => Ok(*n),
                _ => Err(input(format!("{what} must list integers"))),
            })
            .collect::<Result<Vec<i64>, Fail>>()
            .map(Some),
        _ => Err(input(format!("{what} must be a list or null"))),
    }
}

/// JI.3 and JI.45: the refusal of a `dimension` other than 4 at `argument` (the request's or a record header's),
/// with the supplied and the supported dimension; `None` when the dimension is 4 or not given.
fn dimension_refusal(value: Option<&Value>, argument: &str) -> Option<Value> {
    match value {
        None | Some(Value::Null) => None,
        Some(Value::Int(d)) if *d == c600_jumble::DIMENSION => None,
        Some(supplied) => Some(json::object([
            ("status", string("refused")),
            ("code", string(codes::UNSUPPORTED_DIMENSION)),
            ("message", string(&format!("{argument}: this edition admits only dimension {}", c600_jumble::DIMENSION))),
            ("argument", string(argument)),
            ("supplied", supplied.clone()),
            ("supported", Value::Array(vec![int(c600_jumble::DIMENSION)])),
        ])),
    }
}

/// A named cap of an observation as given: the observation refuses one outside 0..599 (JI.45).
fn optional_cap(value: Option<&Value>) -> Result<Option<i64>, Fail> {
    match value {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Int(n)) => Ok(Some(*n)),
        _ => Err(input("cap must be an integer or null")),
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
        if op == "quit" {
            return Ok(None);
        }
        // JI.3: the dimension comes first, before any array or other field is read.
        if let Some(refused) = dimension_refusal(request.get("dimension"), "dimension") {
            return Ok(Some(refused));
        }
        if let Some(result) = self.m8_observation(op, request)? {
            return Ok(Some(result));
        }
        let result = match op {
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
                if let Some(refused) = dimension_refusal(record.get("dimension"), "record.dimension") {
                    return Ok(Some(refused));
                }
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
                    Some(Value::Int(cap)) => observe::Scope::Cap(*cap),
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
            "project" | "sticker_test" | "retained_checkpoint" | "label_checkpoint" | "retained_word" => {
                self.levels(op, request)?
            }
            "gauge" | "gauge_between" => self.levels(op, request)?,
            "derive" | "import" | "compare" => self.derived_op(op, request)?,
            "nu" => self.nu(request)?,
            "nu_constants" => self.nu_constants()?,
            "corrections" => self.corrections()?,
            "correction_lookup" => self.correction_lookup(request)?,
            "correction_set_check" => self.correction_set_check(request)?,
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
    let mut adapter =
        Adapter { ctx, state: None, cap_twists: HashMap::new(), states: HashMap::new(), derived: HashMap::new() };
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

/// The observations of milestone M8 ([`observe::cuts`]; JI.6 and JI.7). Each reads the current state and changes
/// nothing, and `eval_budget` bounds each classification: `rim_summary` (optional `caps`, the named caps),
/// `release_check` (`letter` as [grip, item] of the started menu, optional `cap` for delta-u), `covers` (`cap`) and
/// `repair_list` (`cap`, optional `max_pairs`, and `cancel_after`, a test hook for a cancellation observed at check
/// `cancel_after` + 1).
impl Adapter {
    /// The answer to an M8 observation, or `None` when `op` names none.
    fn m8_observation(&mut self, op: &str, request: &Value) -> Result<Option<Value>, Fail> {
        let result = match op {
            "rim_summary" => {
                let caps = pieces(request.get("caps"), "caps")?;
                let eval_budget = budget(request.get("eval_budget"))?;
                let Adapter { ctx, state, .. } = self;
                let state = state.as_mut().ok_or_else(|| input("start must come first"))?;
                observe::cuts::rim_summary(ctx, state, caps.as_deref(), eval_budget).map_err(Fail::Engine)?
            }
            "release_check" => {
                let letter = request.get("letter").cloned().unwrap_or(Value::Null);
                let letter = self.letters(Some(&Value::Array(vec![letter])))?.remove(0);
                let cap = optional_cap(request.get("cap"))?;
                let eval_budget = budget(request.get("eval_budget"))?;
                let state = self.state.as_ref().ok_or_else(|| input("start must come first"))?;
                observe::cuts::release_check(&self.ctx, state, &letter, cap, eval_budget).map_err(Fail::Engine)?
            }
            "covers" => {
                let cap = optional_cap(request.get("cap"))?.ok_or_else(|| input("covers needs a cap"))?;
                let eval_budget = budget(request.get("eval_budget"))?;
                let Adapter { ctx, state, .. } = self;
                let state = state.as_mut().ok_or_else(|| input("start must come first"))?;
                observe::cuts::covers(ctx, state, cap, eval_budget).map_err(Fail::Engine)?
            }
            "repair_list" => {
                let cap = optional_cap(request.get("cap"))?.ok_or_else(|| input("repair_list needs a cap"))?;
                let max_pairs = count(request.get("max_pairs"), "max_pairs")?;
                let cancel_after = count(request.get("cancel_after"), "cancel_after")?;
                let eval_budget = budget(request.get("eval_budget"))?;
                let state = self.state.as_ref().ok_or_else(|| input("start must come first"))?;
                observe::cuts::repair_list(&self.ctx, state, cap, max_pairs, cancel_after, eval_budget)
                    .map_err(Fail::Engine)?
            }
            _ => return Ok(None),
        };
        Ok(Some(result))
    }
}

/// A non-negative count or null: a repair list's pair budget or its cancellation point.
fn count(value: Option<&Value>, what: &str) -> Result<Option<u64>, Fail> {
    match value {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Int(n)) if *n >= 0 => Ok(Some(*n as u64)),
        _ => Err(input(format!("{what} must be a non-negative integer or null"))),
    }
}

// ----------------------------------------------------------------------------- levels and end tests (LP2, M8)

/// A boolean flag, false when absent.
fn flag(value: Option<&Value>, what: &str) -> Result<bool, Fail> {
    match value {
        None | Some(Value::Null) => Ok(false),
        Some(Value::Bool(b)) => Ok(*b),
        _ => Err(input(format!("{what} must be a boolean"))),
    }
}

/// A word of signed 1-based primitive ids (generator k is id k + 1), as J1's `primitive` reads them.
fn signed_ids(value: Option<&Value>, what: &str) -> Result<Vec<i64>, Fail> {
    let items = value.and_then(Value::as_array).ok_or_else(|| input(format!("{what} must be a list")))?;
    items
        .iter()
        .map(|v| match v {
            Value::Int(n) if (1..=1200u64).contains(&n.unsigned_abs()) => Ok(*n),
            _ => Err(input(format!("{what} lists signed 1-based primitive ids, 1 to 1200 in absolute value"))),
        })
        .collect()
}

/// ν as the ν row gives it: 45 values, the 43 bits in 0..1 and the two values modulo 5 in 0..4.
fn nu_values(value: Option<&Value>) -> Result<Option<Vec<i64>>, Fail> {
    let items = match value {
        None | Some(Value::Null) => return Ok(None),
        Some(Value::Array(items)) if items.len() == c600_jumble::levels::NU_LEN => items,
        _ => return Err(input("nu must list 45 values")),
    };
    let mut out = Vec::with_capacity(items.len());
    for (index, item) in items.iter().enumerate() {
        let modulus = if index < 43 { 2 } else { 5 };
        match item {
            Value::Int(n) if (0..modulus).contains(n) => out.push(*n),
            _ => return Err(input(format!("nu value {index} must be an integer in 0..{}", modulus - 1))),
        }
    }
    Ok(Some(out))
}

/// σ or t of a gauge: one value per piece as base64 little-endian u32, or null for the identity.
fn gauge_part(value: Option<&Value>, what: &str) -> Result<Option<Vec<u32>>, Fail> {
    let np = c600_jumble::model::NP;
    match value {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(text)) => {
            let bytes = c600_jumble::b64::decode(text).ok_or_else(|| input(format!("{what} is no base64")))?;
            if bytes.len() != np * 4 {
                return Err(input(format!("{what} lists {np} little-endian u32 values")));
            }
            Ok(Some(bytes.chunks_exact(4).map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]])).collect()))
        }
        _ => Err(input(format!("{what} must be base64 or null"))),
    }
}

/// The result of an end test, naming the state it read (KN0.6); its verdict names the level (KN1.11).
fn naming(mut result: Value, state_digest: String) -> Value {
    if let Value::Object(fields) = &mut result {
        fields.insert("state".into(), Value::String(state_digest));
    }
    result
}

/// The requests of `c600_jumble::levels`. Each reads the current state, or the stored state that `state` names (see
/// `keep`), and changes nothing, except `retained_word`:
///
/// - `project` (`level`, optional `values`): KN1.9's projection, total, partial with what is missing (the pieces off
///   the lattice first), or refused with `KN1:input.level_not_held`; `values` adds the label or chamber arrays as
///   base64 little-endian 32-bit integers;
/// - `sticker_test`, `retained_checkpoint` (`witness`, a word of signed primitive ids, or `journal`, records from
///   solved) and `label_checkpoint` (`reachability` `{kind, evidence}`, where a witnessed claim names its `journal`
///   from solved instead of evidence, and optional `nu`, 45 values from the ν row): the end tests of A5;
/// - `retained_word` (`word`, optional `budget`): the primitive twists of a retained word, one after another, refused
///   whole, with nothing changed, at its first refused letter;
/// - `gauge` (`sigma` and `t`, each a base64 little-endian u32 array over the pieces, null for the identity): X·γ of
///   KN1.7 becomes the current state, with the `source` digest and the gauge's class; an invalid gauge is an input
///   error;
/// - `gauge_between` (`other`, a stored state): a gauge γ with X·γ equal to the other state, or null, with whether the
///   two shape digests are equal. It changes nothing.
impl Adapter {
    fn read_state(&self, request: &Value) -> Result<&State, Fail> {
        match request.get("state") {
            None | Some(Value::Null) => self.state.as_ref().ok_or_else(|| input("start must come first")),
            Some(Value::String(name)) => self.states.get(name).ok_or_else(|| input(format!("no stored state {name}"))),
            _ => Err(input("state must name a stored state or be null")),
        }
    }

    /// Journal records from solved, each read as `apply` reads a record step, with the moved count it states.
    fn journal(&self, value: Option<&Value>) -> Result<Option<(Vec<Twist>, Vec<Option<usize>>)>, Fail> {
        let records = match value {
            None | Some(Value::Null) => return Ok(None),
            Some(Value::Array(records)) => records,
            _ => return Err(input("a journal must be a list of records")),
        };
        let (mut twists, mut moved) = (Vec::with_capacity(records.len()), Vec::with_capacity(records.len()));
        for (index, record) in records.iter().enumerate() {
            match Twist::from_record(&self.ctx, record) {
                Ok(twist) => twists.push(twist),
                Err(RecordError::Invalid(message) | RecordError::Unsupported(message)) => {
                    return Err(input(format!("journal record {index}: {message}")))
                }
            }
            moved.push(match record.get("moved") {
                None | Some(Value::Null) => None,
                Some(Value::Int(n)) if *n >= 0 => Some(*n as usize),
                _ => return Err(input(format!("journal record {index}: moved must be a non-negative integer"))),
            });
        }
        Ok(Some((twists, moved)))
    }

    fn levels(&mut self, op: &str, request: &Value) -> Result<Value, Fail> {
        use c600_jumble::levels::{self, Level, ReachKind, Reachability, Verdict, Witness};
        if names_derived(request) {
            return self.derived_read(op, request);
        }
        let engine = |e: c600_jumble::state::EngineError| Fail::Engine(e.0);
        match op {
            "project" => {
                let name =
                    request.get("level").and_then(Value::as_str).ok_or_else(|| input("project needs a level"))?;
                let level = Level::parse(name).ok_or_else(|| input(format!("{name} is no level of KN1.7")))?;
                let values = flag(request.get("values"), "values")?;
                let state = self.read_state(request)?;
                let projection = levels::project(&self.ctx, state, level).map_err(engine)?;
                Ok(projection.to_json(level, &state.digest(&self.ctx), values))
            }
            "sticker_test" => {
                let state = self.read_state(request)?;
                let verdict = levels::sticker_test(&self.ctx, state).map_err(engine)?;
                Ok(naming(verdict.to_json_at(levels::STICKER_TEST_LEVEL), state.digest(&self.ctx)))
            }
            "retained_checkpoint" => {
                let word = match request.get("witness") {
                    None | Some(Value::Null) => None,
                    value => Some(signed_ids(value, "witness")?),
                };
                let journal = self.journal(request.get("journal"))?;
                let state = self.read_state(request)?;
                let witness = match (&word, &journal) {
                    (Some(word), None) => Witness::Word(word.as_slice()),
                    (None, Some((twists, _))) => Witness::Journal(twists.as_slice()),
                    _ => return Err(input("a retained checkpoint needs a witness word or a journal, not both")),
                };
                let verdict = levels::retained_checkpoint(&self.ctx, state, witness).map_err(engine)?;
                Ok(naming(verdict.to_json_at(levels::RETAINED_CHECKPOINT_LEVEL), state.digest(&self.ctx)))
            }
            "label_checkpoint" => {
                let claim = request.get("reachability").ok_or_else(|| input("a label checkpoint needs a claim"))?;
                let kind = claim.get("kind").and_then(Value::as_str).and_then(ReachKind::parse).ok_or_else(|| {
                    input("reachability kind must be witnessed, by_theorem, promised, refuted or open")
                })?;
                let journal = self.journal(claim.get("journal"))?;
                let nu = nu_values(request.get("nu"))?;
                let state = self.read_state(request)?;
                let digest = state.digest(&self.ctx);
                let reach = match (kind, journal) {
                    (ReachKind::Witnessed, Some((twists, moved))) => {
                        let shown =
                            levels::witness(&self.ctx, state, state.menu_arc(), &twists, Some(moved.as_slice()))
                                .map_err(engine)?;
                        match shown {
                            Ok(reach) => reach,
                            Err(reason) => {
                                let reason = format!("the journal does not witness this state: {reason}");
                                return Ok(naming(
                                    Verdict::Unknown(reason).to_json_at(levels::LABEL_CHECKPOINT_LEVEL),
                                    digest,
                                ));
                            }
                        }
                    }
                    (ReachKind::Witnessed, None) => {
                        return Err(input("a witnessed claim names its journal from solved"))
                    }
                    (_, Some(_)) => return Err(input("only a witnessed claim names a journal")),
                    (kind, None) => Reachability::stated(kind, claim.get("evidence").cloned().unwrap_or(Value::Null))
                        .ok_or_else(|| input("a witnessed claim names its journal from solved"))?,
                };
                let verdict = levels::label_checkpoint(&self.ctx, state, &reach, nu.as_deref()).map_err(engine)?;
                Ok(naming(verdict.to_json_at(levels::LABEL_CHECKPOINT_LEVEL), digest))
            }
            "retained_word" => {
                let word = signed_ids(request.get("word"), "word")?;
                let budget = budget(request.get("budget"))?;
                let twists: Vec<Twist> = word
                    .iter()
                    .map(|&mid| levels::primitive(&self.ctx, mid))
                    .collect::<Result<_, _>>()
                    .map_err(engine)?;
                let saved = self.state()?.clone();
                let mut moved: Vec<u32> = Vec::new();
                for (index, twist) in twists.iter().enumerate() {
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
                        let message = format!("letter {index} of the retained word is {}", outcome.status.as_str());
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
                    fields.insert("applied".into(), int(twists.len() as i64));
                    fields.insert("refused_at".into(), Value::Null);
                }
                Ok(result)
            }
            "gauge" => {
                let np = c600_jumble::model::NP;
                let sigma = gauge_part(request.get("sigma"), "sigma")?.unwrap_or_else(|| (0..np as u32).collect());
                let t: Vec<u16> = match gauge_part(request.get("t"), "t")? {
                    None => vec![0; np],
                    Some(values) => values
                        .into_iter()
                        .map(|k| u16::try_from(k).map_err(|_| input("t lists K+ indices")))
                        .collect::<Result<_, _>>()?,
                };
                let gauge = levels::Gauge::new(&self.ctx, sigma, t).map_err(Fail::Input)?;
                let source = self.read_state(request)?;
                let image = levels::act(&self.ctx, source, &gauge).map_err(engine)?;
                let source = source.digest(&self.ctx);
                let summary = gauge.to_json(&self.ctx, false).map_err(engine)?;
                self.state = Some(image);
                self.cap_twists.clear();
                let mut result = self.step_json("start", 0, EMPTY)?;
                if let Value::Object(fields) = &mut result {
                    fields.insert("source".into(), string(&source));
                    fields.insert("gauge".into(), summary);
                }
                Ok(result)
            }
            "gauge_between" => {
                let name = request
                    .get("other")
                    .and_then(Value::as_str)
                    .ok_or_else(|| input("gauge_between needs the name of a stored state"))?;
                let other = self.states.get(name).ok_or_else(|| input(format!("no stored state {name}")))?;
                let state = self.read_state(request)?;
                let shape = |s: &State| levels::shape(&self.ctx, s).map(|shape| shape.digest()).map_err(engine);
                let shapes_equal = shape(state)? == shape(other)?;
                let gauge = match levels::gauge_between(&self.ctx, state, other).map_err(engine)? {
                    Some(gauge) => gauge.to_json(&self.ctx, true).map_err(engine)?,
                    None => Value::Null,
                };
                Ok(json::object([
                    ("state", string(&state.digest(&self.ctx))),
                    ("other", string(&other.digest(&self.ctx))),
                    ("shapes_equal", Value::Bool(shapes_equal)),
                    ("gauge", gauge),
                ]))
            }
            other => Err(input(format!("unknown op {other}"))),
        }
    }
}

// ------------------------------------------------------------------------------------ derived states (LP2, M8)

/// Whether a request, or an operand of `compare`, names a derived value (`derived`) in place of a state.
fn names_derived(request: &Value) -> bool {
    !matches!(request.get("derived"), None | Some(Value::Null))
}

/// A refused request on derived values: `{result: "refused", code, message}`; nothing changes.
fn derived_refusal(refusal: &c600_kernel::Refusal) -> Value {
    json::object([("result", string("refused")), ("code", string(refusal.code)), ("message", string(&refusal.message))])
}

/// The requests on derived values (KN2.11 to KN2.14, ONE.25). A derived value is named by the caller and kept apart
/// from the stored states; it never becomes the current state (KN2.12), and none of these requests changes the
/// current state or a stored one. A new value under a name in use replaces the old one, as `keep` does for a stored
/// state; a refusal stores nothing. Input errors come before refusals.
///
/// - `derive` (`name`, `identity`, `base`, `letters` or `records`, optional `mode` and `protect`): KN2.11's
///   `derive(base, word)`. `base` is `{derived}`, an earlier derived value, or `{state, journal}`: the current state
///   (`state` null) or a stored state, with its journal of records from solved (`[]` for solved), which must witness
///   it. The word is `letters`, [grip, item] pairs under the base's menu, or `records`, twist records. `mode` is `net`
///   (the default) or `strict`, and `protect` lists the protected pieces. The answer is the derived value, stored
///   under `name`, or the refusal with its first refused prefix, its outcome and its certificates.
/// - `import` (`name`, `record`, `identity`, `reachability` `{kind, evidence}`, open or promised): KN2.14's raw pose
///   table, read under the current state's menu and stored under `name`.
/// - `compare` (`a`, `b`, `level`): ONE.25's comparison of two operands, each `{state}` (a stored state, or null for
///   the current state) or `{derived}`.
/// - `project`, `sticker_test`, `retained_checkpoint` and `label_checkpoint` with `derived` in place of `state` read
///   the derived value; `label_checkpoint` then uses the value's own reachability claim and takes none.
impl Adapter {
    /// The derived value that the request's `derived` names.
    fn derived_value(&self, request: &Value) -> Result<&c600_jumble::levels::Derived, Fail> {
        match request.get("derived") {
            Some(Value::String(name)) => {
                self.derived.get(name).ok_or_else(|| input(format!("no derived value {name}")))
            }
            _ => Err(input("derived must name a derived value")),
        }
    }

    /// The twists of a word's letters, [grip, item] pairs, under `menu`, the menu of the derivation's base.
    fn derive_letters(&self, menu: Option<Arc<Menu>>, value: &Value) -> Result<Vec<Twist>, Fail> {
        let menu = menu.ok_or_else(|| input("letters need a base with a menu"))?;
        let items = value.as_array().ok_or_else(|| input("letters must be a list of [grip, item]"))?;
        let mut caps: HashMap<usize, Vec<Twist>> = HashMap::new();
        let mut out = Vec::with_capacity(items.len());
        for letter in items {
            let pair =
                letter.as_array().filter(|p| p.len() == 2).ok_or_else(|| input("a letter must be [grip, item]"))?;
            let grip = small_int(Some(&pair[0]), "letter grip", 600)? as usize;
            let item = small_int(Some(&pair[1]), "letter item", menu.items.len() as i64)? as usize;
            if !caps.contains_key(&grip) {
                let twists = menu.for_cap(&self.ctx, grip).map_err(|e| Fail::Engine(e.to_string()))?;
                caps.insert(grip, twists);
            }
            out.push(caps[&grip][item].clone());
        }
        Ok(out)
    }

    /// An operand of `compare`: `{state}`, read as `read_state` reads a request, or `{derived}`.
    fn operand(&self, value: Option<&Value>, what: &str) -> Result<&State, Fail> {
        let value =
            value.filter(|v| v.as_object().is_some()).ok_or_else(|| input(format!("{what} must be a mapping")))?;
        if !names_derived(value) {
            return self.read_state(value);
        }
        if !matches!(value.get("state"), None | Some(Value::Null)) {
            return Err(input(format!("{what} names a state or a derived value, not both")));
        }
        Ok(self.derived_value(value)?.state())
    }

    fn derived_op(&mut self, op: &str, request: &Value) -> Result<Value, Fail> {
        use c600_jumble::levels::{self, Base, Derivation, Level, ReachKind, Reachability, Snapshot};
        use c600_kernel::ProtectionMode;
        let engine = |e: c600_jumble::state::EngineError| Fail::Engine(e.0);
        let given = |key: &str| request.get(key).filter(|v| !matches!(v, Value::Null));
        let named = |mut result: Value, name: &str| {
            if let Value::Object(fields) = &mut result {
                fields.insert("name".into(), string(name));
            }
            result
        };
        match op {
            "derive" => {
                let name = name(request)?;
                let identity =
                    parse_identity(request.get("identity").ok_or_else(|| input("derive needs an identity block"))?)?;
                let mode = match given("mode") {
                    None => ProtectionMode::Net,
                    Some(Value::String(s)) if s == "net" => ProtectionMode::Net,
                    Some(Value::String(s)) if s == "strict" => ProtectionMode::Strict,
                    _ => return Err(input("mode must be net or strict")),
                };
                let protect = pieces(request.get("protect"), "protect")?.unwrap_or_default();
                let base = given("base")
                    .filter(|b| b.as_object().is_some())
                    .ok_or_else(|| input("derive needs a base mapping: {derived} or {state, journal}"))?;
                // The base's state, and for a state base its journal from solved.
                let (state, history) = if names_derived(base) {
                    if !matches!(base.get("state"), None | Some(Value::Null))
                        || !matches!(base.get("journal"), None | Some(Value::Null))
                    {
                        return Err(input("a derived base names no state and no journal"));
                    }
                    (self.derived_value(base)?.state(), None)
                } else {
                    let history = self
                        .journal(base.get("journal"))?
                        .ok_or_else(|| input("a state base names its journal from solved ([] for solved)"))?;
                    (self.read_state(base)?, Some(history))
                };
                let twists = match (given("letters"), given("records")) {
                    (Some(letters), None) => self.derive_letters(state.menu_arc(), letters)?,
                    (None, Some(records)) => self.journal(Some(records))?.map(|(twists, _)| twists).unwrap_or_default(),
                    _ => return Err(input("a derivation names its word as letters or as records, not both")),
                };
                let snapshot = match history {
                    None => None,
                    Some((journal, moved)) => Some(
                        Snapshot::witnessed(&self.ctx, state.clone(), &journal, Some(moved.as_slice()))
                            .map_err(engine)?
                            .map_err(|reason| input(format!("the journal does not witness the base: {reason}")))?,
                    ),
                };
                if let Err(refusal) = self.ctx.check_identity(state.menu(), &identity) {
                    return Ok(derived_refusal(&refusal));
                }
                let np = c600_jumble::model::NP;
                if let Some(p) = protect.iter().find(|&&p| !(0..np as i64).contains(&p)) {
                    let message = format!("Protected piece {p} is outside 0..{}", np - 1);
                    return Ok(derived_refusal(&c600_kernel::Refusal {
                        code: codes::NAMED_OBJECT_INVALID,
                        message,
                        fact: None,
                    }));
                }
                let protect: Vec<u32> = protect.into_iter().map(|p| p as u32).collect();
                let base = match &snapshot {
                    Some(snapshot) => Base::Snapshot(snapshot),
                    None => Base::Derived(self.derived_value(base)?),
                };
                match levels::derive(&self.ctx, base, &identity, &twists, mode, &protect).map_err(engine)? {
                    Err(refusal) => Ok(derived_refusal(&refusal)),
                    Ok(Derivation::Refused(refused)) => Ok(refused.to_json()),
                    Ok(Derivation::Derived(derived)) => {
                        let result = named(derived.to_json(&self.ctx), &name);
                        self.derived.insert(name, derived);
                        Ok(result)
                    }
                }
            }
            "import" => {
                let name = name(request)?;
                let record = request.get("record").ok_or_else(|| input("import needs a record"))?;
                if let Some(refused) = dimension_refusal(record.get("dimension"), "record.dimension") {
                    return Ok(refused);
                }
                let identity =
                    parse_identity(request.get("identity").ok_or_else(|| input("import needs an identity block"))?)?;
                let claim = request.get("reachability").ok_or_else(|| input("import needs a reachability claim"))?;
                let kind = claim.get("kind").and_then(Value::as_str).and_then(ReachKind::parse).ok_or_else(|| {
                    input("reachability kind must be witnessed, by_theorem, promised, refuted or open")
                })?;
                if !matches!(claim.get("journal"), None | Some(Value::Null)) {
                    return Err(input("a raw table names no journal: it is never witnessed (KN2.14)"));
                }
                let reach = Reachability::stated(kind, claim.get("evidence").cloned().unwrap_or(Value::Null))
                    .ok_or_else(|| input("a raw table is never witnessed (KN2.14)"))?;
                let menu = self.state()?.menu_arc();
                match levels::import(&self.ctx, menu, record, &identity, reach).map_err(Fail::Input)? {
                    Err(refusal) => Ok(derived_refusal(&refusal)),
                    Ok(derived) => {
                        let result = named(derived.to_json(&self.ctx), &name);
                        self.derived.insert(name, derived);
                        Ok(result)
                    }
                }
            }
            "compare" => {
                let name = given("level").and_then(Value::as_str).ok_or_else(|| input("compare needs a level"))?;
                let level = Level::parse(name).ok_or_else(|| input(format!("{name} is no level of KN1.7")))?;
                let a = self.operand(request.get("a"), "a")?;
                let b = self.operand(request.get("b"), "b")?;
                let comparison = levels::compare(&self.ctx, a, b, level).map_err(engine)?;
                let mut result = comparison.to_json(level);
                if let Value::Object(fields) = &mut result {
                    let digests = vec![string(&a.digest(&self.ctx)), string(&b.digest(&self.ctx))];
                    fields.insert("states".into(), Value::Array(digests));
                }
                Ok(result)
            }
            other => Err(input(format!("unknown op {other}"))),
        }
    }

    /// `project`, `sticker_test`, `retained_checkpoint` and `label_checkpoint` on the derived value that `derived`
    /// names, as `levels` reads a state; they change nothing. Another request does not read a derived value.
    fn derived_read(&self, op: &str, request: &Value) -> Result<Value, Fail> {
        use c600_jumble::levels::{self, Level, Witness};
        let engine = |e: c600_jumble::state::EngineError| Fail::Engine(e.0);
        if !matches!(op, "project" | "sticker_test" | "retained_checkpoint" | "label_checkpoint") {
            return Err(input(format!("{op} does not read a derived value")));
        }
        if !matches!(request.get("state"), None | Some(Value::Null)) {
            return Err(input("a request names a state or a derived value, not both"));
        }
        match op {
            "project" => {
                let name =
                    request.get("level").and_then(Value::as_str).ok_or_else(|| input("project needs a level"))?;
                let level = Level::parse(name).ok_or_else(|| input(format!("{name} is no level of KN1.7")))?;
                let values = flag(request.get("values"), "values")?;
                let state = self.derived_value(request)?.state();
                let projection = levels::project(&self.ctx, state, level).map_err(engine)?;
                Ok(projection.to_json(level, &state.digest(&self.ctx), values))
            }
            "sticker_test" => {
                let state = self.derived_value(request)?.state();
                let verdict = levels::sticker_test(&self.ctx, state).map_err(engine)?;
                Ok(naming(verdict.to_json_at(levels::STICKER_TEST_LEVEL), state.digest(&self.ctx)))
            }
            "retained_checkpoint" => {
                let word = match request.get("witness") {
                    None | Some(Value::Null) => None,
                    value => Some(signed_ids(value, "witness")?),
                };
                let journal = self.journal(request.get("journal"))?;
                let state = self.derived_value(request)?.state();
                let witness = match (&word, &journal) {
                    (Some(word), None) => Witness::Word(word.as_slice()),
                    (None, Some((twists, _))) => Witness::Journal(twists.as_slice()),
                    _ => return Err(input("a retained checkpoint needs a witness word or a journal, not both")),
                };
                let verdict = levels::retained_checkpoint(&self.ctx, state, witness).map_err(engine)?;
                Ok(naming(verdict.to_json_at(levels::RETAINED_CHECKPOINT_LEVEL), state.digest(&self.ctx)))
            }
            _ => {
                if !matches!(request.get("reachability"), None | Some(Value::Null)) {
                    return Err(input("a derived value carries its own reachability claim"));
                }
                let nu = nu_values(request.get("nu"))?;
                let derived = self.derived_value(request)?;
                let state = derived.state();
                let verdict = levels::label_checkpoint(&self.ctx, state, derived.reachability(), nu.as_deref())
                    .map_err(engine)?;
                Ok(naming(verdict.to_json_at(levels::LABEL_CHECKPOINT_LEVEL), state.digest(&self.ctx)))
            }
        }
    }
}

// ------------------------------------------------------------------------------------------------------ ν (A5)

/// ν ([`c600_jumble::nu`]). Neither request changes anything.
///
/// - `nu` reads the current state, a stored state (`state`) or a derived value (`derived`). With a derived value, or
///   with `journal`, records from solved that witness the state, it is KN2.4's row ([`c600_jumble::nu::nu`]): under
///   I_a a nonzero fixed coordinate is refused with `KN2:consistency.fixed_nu_nonzero`. Without either it is the bare
///   value ([`c600_jumble::nu::value`]), which makes no claim. The answer is `{result: "total", nu, nonzero, state}`
///   with the 45 values and the nonzero ones by name, `{result: "undefined", reason, state}` or `{result: "refused",
///   code, message, state}`. A journal that does not witness the state is an input error.
/// - `nu_constants`: the coordinates' names and moduli, the fixed coordinates under I_a, the orbits of the frame bits
///   and of the values modulo 5, the C5 convention, the certificate program's SHA-256, the I_a menu's identity and
///   order digest, the contract revision, and each orbit's frame group (`groups`: orbit, kind, order, slots, and the
///   generator r0 of a C5 group).
impl Adapter {
    fn nu(&self, request: &Value) -> Result<Value, Fail> {
        use c600_jumble::levels::{Base, Snapshot};
        use c600_jumble::nu;
        let engine = |e: c600_jumble::state::EngineError| Fail::Engine(e.0);
        let journal = self.journal(request.get("journal"))?;
        let (result, state) = if names_derived(request) {
            if !matches!(request.get("state"), None | Some(Value::Null)) || journal.is_some() {
                return Err(input("a derived value names no state and no journal"));
            }
            let derived = self.derived_value(request)?;
            (nu::nu(&self.ctx, Base::Derived(derived)).map_err(engine)?, derived.state())
        } else {
            let state = self.read_state(request)?;
            match journal {
                None => (Ok(nu::value(&self.ctx, state).map_err(engine)?), state),
                Some((twists, moved)) => {
                    let snapshot = Snapshot::witnessed(&self.ctx, state.clone(), &twists, Some(moved.as_slice()))
                        .map_err(engine)?
                        .map_err(|reason| input(format!("the journal does not witness the state: {reason}")))?;
                    (nu::nu(&self.ctx, Base::Snapshot(&snapshot)).map_err(engine)?, state)
                }
            }
        };
        nu_answer(result, &state.digest(&self.ctx))
    }

    fn nu_constants(&self) -> Result<Value, Fail> {
        use c600_jumble::nu;
        let tables = nu::tables(&self.ctx).map_err(|e| Fail::Engine(e.0))?;
        let mut groups = Vec::new();
        for orbit in 0..35 {
            let group = tables.group(orbit).ok_or_else(|| Fail::Engine(format!("orbit {orbit} has no group")))?;
            let generator = match group.generator() {
                Some(r0) => Value::Array(r0.iter().map(|&x| int(x as i64)).collect()),
                None => Value::Null,
            };
            groups.push(json::object([
                ("orbit", int(orbit as i64)),
                ("kind", string(group.kind().name())),
                ("order", int(group.order() as i64)),
                ("slots", int(group.slots() as i64)),
                ("generator", generator),
            ]));
        }
        let ints = |values: &[usize]| Value::Array(values.iter().map(|&v| int(v as i64)).collect());
        Ok(json::object([
            ("names", Value::Array(nu::NAMES.iter().map(|name| string(name)).collect())),
            ("moduli", Value::Array(nu::MODULI.iter().map(|&m| int(m)).collect())),
            ("i_a_fixed", Value::Array(nu::I_A_FIXED.iter().map(|&i| string(nu::NAMES[i])).collect())),
            ("frame_bit_orbits", ints(&nu::FRAME_BIT_ORBITS)),
            ("c5_orbits", ints(&nu::C5_ORBITS)),
            ("c5_convention", string(nu::C5_CONVENTION)),
            ("certificate_program_sha256", string(nu::CERTIFICATE_PROGRAM_SHA256)),
            ("i_a_menu_identity", string(nu::I_A_MENU_IDENTITY)),
            ("i_a_order_digest", string(nu::I_A_ORDER_DIGEST)),
            ("contract_revision", string(c600_jumble::CONTRACT_REVISION)),
            ("groups", Value::Array(groups)),
        ]))
    }
}

/// The answer of `nu` and `correction_set_check`: `{result: "total", nu, nonzero, state}` with the 45 values and the
/// nonzero ones by name, `{result: "undefined", reason, state}` or `{result: "refused", code, message, state}`.
fn nu_answer(
    result: Result<c600_jumble::levels::Partial<c600_jumble::nu::NuVector>, c600_kernel::Refusal>,
    digest: &str,
) -> Result<Value, Fail> {
    use c600_jumble::levels::Partial;
    Ok(match result {
        Err(refusal) => json::object([
            ("result", string("refused")),
            ("code", string(refusal.code)),
            ("message", string(&refusal.message)),
            ("state", string(digest)),
        ]),
        Ok(Partial::Total(value)) => {
            let nonzero = value.nonzero().into_iter().map(|(name, v)| (name.to_owned(), int(v))).collect();
            json::object([
                ("result", string("total")),
                ("nu", value.to_json()),
                ("nonzero", Value::Object(nonzero)),
                ("state", string(digest)),
            ])
        }
        Ok(Partial::Undefined(reason)) => {
            json::object([("result", string("undefined")), ("reason", string(&reason)), ("state", string(digest))])
        }
        Ok(Partial::Partial { .. }) => return Err(Fail::Engine("ν is never partial".into())),
    })
}

// ------------------------------------------------------------------------------------------- correction catalogue

/// The shipped correction catalogue ([`c600_jumble::corrections`]; DT-J1 O7 (a), JI.13, EH-11). None of these
/// requests previews, applies or changes anything, and none chooses a correction: every id is the caller's.
///
/// - `corrections`: the format, the binding (model ID, I_a menu identity and order digest, contract revision), the
///   source (the research library's path and SHA-256), the coordinate names and the 36 corrections in id order, each
///   `{id, family, letters, row}` with the letters as [grip, item] pairs.
/// - `correction_lookup` (`id`, optional `twists`): `{result: "found", correction}`, or `{result: "refused", code,
///   message}` for an unknown id. With `twists: true`, `word` holds the literal word as twists of the started state's
///   menu, `{result: "word", twists}` with each twist `{grip, matrix}`, or the refusal of free mode or of a menu that
///   is not I_a by identity and order digest (`KN2:input.menu_binding_mismatch`).
/// - `correction_set_check` (`corrections`, [id, m] pairs with m any integer, and a base): KN2.4's row on a derived
///   value (`derived`), or on the current or a stored state (`state`) with `journal`, records from solved that witness
///   it. The answer is the `nu` answer for ν plus the drafted set's rows. A journal that does not witness the state,
///   and a state without a journal, are input errors.
impl Adapter {
    fn corrections(&self) -> Result<Value, Fail> {
        use c600_jumble::corrections;
        let catalogue = corrections::catalogue().map_err(|e| Fail::Engine(e.0))?;
        let binding = catalogue.binding();
        Ok(json::object([
            ("format", string(corrections::FORMAT)),
            (
                "binding",
                json::object([
                    ("model_id", string(&binding.model_id)),
                    ("menu_identity", string(&binding.menu_identity)),
                    ("menu_order", string(&binding.menu_order)),
                    ("contract_revision", string(&binding.contract_revision)),
                ]),
            ),
            (
                "source",
                json::object([
                    ("library", string(catalogue.library())),
                    ("sha256", string(corrections::LIBRARY_SHA256)),
                ]),
            ),
            ("names", Value::Array(c600_jumble::nu::NAMES.iter().map(|name| string(name)).collect())),
            ("corrections", Value::Array(catalogue.rows().iter().map(correction_json).collect())),
        ]))
    }

    fn correction_lookup(&self, request: &Value) -> Result<Value, Fail> {
        use c600_jumble::corrections;
        let id = correction_id(request.get("id"))?;
        // With twists, the started state's menu: None in free mode.
        let menu = match request.get("twists") {
            None | Some(Value::Null) | Some(Value::Bool(false)) => None,
            Some(Value::Bool(true)) => {
                Some(self.state.as_ref().ok_or_else(|| input("start must come first"))?.menu_arc())
            }
            _ => return Err(input("twists must be true, false or null")),
        };
        let catalogue = corrections::catalogue().map_err(|e| Fail::Engine(e.0))?;
        let correction = match catalogue.lookup(id) {
            Ok(correction) => correction,
            Err(refusal) => return Ok(derived_refusal(&refusal)),
        };
        let mut answer = json::object([("result", string("found")), ("correction", correction_json(correction))]);
        if let Some(menu) = menu {
            let twists = match &menu {
                Some(menu) => catalogue.twists(&self.ctx, menu, id).map_err(|e| Fail::Engine(e.0))?,
                // A word needs a menu, and free mode has none: refused as the check refuses free mode.
                None => Err(c600_kernel::Refusal {
                    code: codes::MENU_BINDING_MISMATCH,
                    message: "The corrections are words of the I_a menu; a state in free mode has no menu".into(),
                    fact: None,
                }),
            };
            let word = match twists {
                Ok(word) => {
                    let twists = word.iter().map(|twist| {
                        json::object([
                            ("grip", int(twist.grip as i64)),
                            ("matrix", c600_jumble::q5::matrix_json(&twist.matrix)),
                        ])
                    });
                    json::object([("result", string("word")), ("twists", Value::Array(twists.collect()))])
                }
                Err(refusal) => derived_refusal(&refusal),
            };
            if let Value::Object(fields) = &mut answer {
                fields.insert("word".into(), word);
            }
        }
        Ok(answer)
    }

    fn correction_set_check(&self, request: &Value) -> Result<Value, Fail> {
        use c600_jumble::corrections;
        use c600_jumble::levels::{Base, Snapshot};
        let engine = |e: c600_jumble::state::EngineError| Fail::Engine(e.0);
        let set = correction_set(request.get("corrections"))?;
        let journal = self.journal(request.get("journal"))?;
        if names_derived(request) {
            if !matches!(request.get("state"), None | Some(Value::Null)) || journal.is_some() {
                return Err(input("a derived value names no state and no journal"));
            }
            let derived = self.derived_value(request)?;
            let result = corrections::correction_set_check(&self.ctx, Base::Derived(derived), &set).map_err(engine)?;
            return nu_answer(result, &derived.state().digest(&self.ctx));
        }
        let state = self.read_state(request)?;
        let (twists, moved) =
            journal.ok_or_else(|| input("the check needs a derived value or a journal that witnesses the state"))?;
        let snapshot = Snapshot::witnessed(&self.ctx, state.clone(), &twists, Some(moved.as_slice()))
            .map_err(engine)?
            .map_err(|reason| input(format!("the journal does not witness the state: {reason}")))?;
        let result = corrections::correction_set_check(&self.ctx, Base::Snapshot(&snapshot), &set).map_err(engine)?;
        nu_answer(result, &state.digest(&self.ctx))
    }
}

/// One correction as `{id, family, letters, row}`.
fn correction_json(correction: &c600_jumble::corrections::Correction) -> Value {
    let letters =
        correction.letters().iter().map(|&(grip, item)| Value::Array(vec![int(grip as i64), int(item as i64)]));
    json::object([
        ("id", int(correction.id() as i64)),
        ("family", string(correction.family())),
        ("letters", Value::Array(letters.collect())),
        ("row", correction.row().to_json()),
    ])
}

/// A correction id: an integer in 0..2^32. An id outside the catalogue is the library's refusal, not an input error.
fn correction_id(value: Option<&Value>) -> Result<u32, Fail> {
    value
        .and_then(Value::as_i64)
        .and_then(|id| u32::try_from(id).ok())
        .ok_or_else(|| input("a correction id must be an integer in 0..2^32"))
}

/// A drafted correction set: a list of [id, m] pairs, m any integer.
fn correction_set(value: Option<&Value>) -> Result<Vec<(u32, i64)>, Fail> {
    let items = value.and_then(Value::as_array).ok_or_else(|| input("corrections must be a list of [id, m] pairs"))?;
    items
        .iter()
        .map(|item| {
            let pair = item.as_array().filter(|p| p.len() == 2).ok_or_else(|| input("a correction must be [id, m]"))?;
            let id = correction_id(Some(&pair[0]))?;
            let m = pair[1].as_i64().ok_or_else(|| input("a multiplicity must be an integer"))?;
            Ok((id, m))
        })
        .collect()
}

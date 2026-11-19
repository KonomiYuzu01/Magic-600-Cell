//! The jumbling rows' host bridge (LP3, C8): KN2.4's jumbling rows that the library entries read ([`crate::library`]),
//! answered by `c600_jumble` and served to a run as snapshot reads, in the library's draft answer types (KN5, LP2 draft
//! until the command table freezes, PHS.52).
//!
//! [`Rows::at_head`] reads at the command layer's head with Jumble on: the head's derived value under the pinned menu
//! ([`CommandLayer::jumbled`]) is the state every row reads and the base that `protection`, `nu` and
//! `correction_set_check` derive from. With Jumble off every read refuses with `KN5:input.menu_word_needs_jumble_on` (a
//! draft: no code names the jumbling rows' mode yet). [`Rows::on`] reads on a witnessed base apart from a layer, and
//! [`Rows::at_state`] at a state with no witnessed claim, such as a free-mode state that no menu press reaches; there
//! the three rows that need a base are not granted. The bridge is the same code in each case, so a test of the rows at
//! a state tests what the head serves.
//!
//! # Rows
//!
//! - `heights`, no argument: `observe::heights` of the state.
//! - `height_counts`, no argument: the `height_counts` field of `heights`.
//! - `replay`, a menu word: JI.10's prefix observations of the word at the state (`observe::prefix_observations`),
//!   which is not changed. A refused letter's row writes the after-state facts as null, undefined, so that every row
//!   has an admitted row's fields.
//! - `word_check`, a record `{ word, unit }`: the kernel's whole-word check of the word with the unit
//!   (`observe::word_check`, JI.11). A unit piece outside the model is `KN2:input.named_object_invalid` (JI.45).
//! - `protection`, a record `{ word, mode, pieces }` with `mode` `"net"` or `"strict"`: M8's derivation of the word
//!   from the base under the base's own identity, with protection at pose level (`levels::derive`, KN2.7), answered
//!   with its protection report, for a derived value or for a refusal that has one. A refusal without one is its code.
//! - `nu`, no argument: KN2.4's ν of the base (`nu::nu`), as a record `{ reason, values }`: ν's 45 values in A5's fixed
//!   C5 convention and no reason when ν is defined, the reason and no values when it is not (off the lattice).
//! - `correction_set_check`, a list of records `{ id, m }`, the solver's drafted set: ν after the set as the catalogue
//!   predicts it (`corrections::correction_set_check`), in the same record. The bridge keeps each set it was asked to
//!   check ([`Rows::drafts`]); it never chooses, ranks or reduces a set.
//!
//! A menu word is read as [`crate::word::LiteralWord::letters`] reads it: a word of another alphabet than the state's
//! is `LB1:input.incompatible_alphabet`. Every field of an answer is written, null when undefined; a declared type that
//! the answer does not have, or a declared field that it does not write, is `LG2:type.mismatch` ([`convert`]).
//!
//! # Grants and refusals
//!
//! The bridge grants snapshot reads of the rows above only: no preview, file, clock, workspace write or sink, and no
//! other row (`LG3:capability.not_granted`). A row read without the argument it takes, or with one it does not take, or
//! an argument of another shape, is `KN5:input.argument_form`, refused at the tariff, at the cost of the read's step. A
//! row's own refusal (an unshipped correction id, a base under another menu, a nonzero fixed coordinate, a protected
//! piece that would move) stops the run with its code once the read is charged. An error of the jumbling engine is
//! `KN5:consistency.state_hash_mismatch`, as the command layer reports one (`c600-command`'s `jumble` module).
//!
//! # Tariff (draft; the command table prices the rows when it freezes)
//!
//! The kernel cost is one unit for each piece of the model ([`NP`]) for each pass over a state: one pass for `heights`,
//! `height_counts` and `nu`, and one more for each letter of a named word or each entry of a drafted set. The result
//! bound is [`RESULT_BOUND`] for every row; an answer above it stops the run with the interpreter's
//! `KN7:resource_limit.host_exhausted`, never a cut answer.

use c600_check::TYPE_MISMATCH;
use c600_command::{code, CommandLayer, Store};
use c600_ir::{QTy, Ty};
use c600_jumble::codes::{IDENTITY_MISMATCH, MENU_WORD_NEEDS_JUMBLE_ON, NAMED_OBJECT_INVALID};
use c600_jumble::corrections;
use c600_jumble::levels::{self, Base, Derivation, Partial};
use c600_jumble::model::NP;
use c600_jumble::nu::{self, NuVector};
use c600_jumble::observe::{heights, prefix_observations, word_check, Batch, Fields, PrefixRequest};
use c600_jumble::state::State;
use c600_jumble::Context;
use c600_kernel::json::{self, Value};
use c600_kernel::{ProtectionMode, Refusal};
use c600_run::interp::{Host, Request, RowTariff, Sink};
use c600_run::menu::MenuIdentity;
use c600_run::meter::code::NOT_GRANTED;
use c600_run::provenance::{self, Json};
use c600_run::value::Value as RunValue;

use crate::word::{self, menu_identity, Letter, INCOMPATIBLE_ALPHABET};

/// The rows the bridge serves, each with whether it takes an argument.
pub const ROWS: [(&str, bool); 7] = [
    ("heights", false),
    ("height_counts", false),
    ("replay", true),
    ("word_check", true),
    ("protection", true),
    ("nu", false),
    ("correction_set_check", true),
];

/// The result bound of every row's answer (a draft): 16 MiB of logical size.
pub const RESULT_BOUND: u128 = 1 << 24;

/// The after-state facts that JI.10's prefix observations write in an admitted letter's row, with no optional field
/// requested.
const AFTER_FACTS: [&str; 4] = ["phi_before", "phi_after", "delta_phi", "max_left_height"];

/// What the rows read: a witnessed base, a state with no witnessed claim, or nothing, with the code every read gets.
#[derive(Clone, Copy)]
enum At<'a> {
    Base(Base<'a>),
    State(&'a State),
    Refused(&'static str),
}

/// The bridge for one run: the rows at one state, and its record of what the run read.
pub struct Rows<'a> {
    ctx: &'a Context,
    at: At<'a>,
    asked: Vec<String>,
    drafts: Vec<Vec<(u32, i64)>>,
}

impl<'a> Rows<'a> {
    /// The rows at `layer`'s head with Jumble on, under the engine `ctx` the layer's menu is pinned with. With Jumble
    /// off every read refuses with `KN5:input.menu_word_needs_jumble_on`; under another engine than the head's
    /// identity, with `KN1:input.identity_mismatch`.
    pub fn at_head<S: Store>(ctx: &'a Context, layer: &'a CommandLayer<S>) -> Rows<'a> {
        let at = match layer.jumbled() {
            None => At::Refused(MENU_WORD_NEEDS_JUMBLE_ON),
            Some(head) if ctx.identity(head.state().menu()) != *head.identity() => At::Refused(IDENTITY_MISMATCH),
            Some(head) => At::Base(Base::Derived(head)),
        };
        Rows::new(ctx, at)
    }

    /// The rows on a witnessed base: a snapshot or a derived value.
    pub fn on(ctx: &'a Context, base: Base<'a>) -> Rows<'a> {
        Rows::new(ctx, At::Base(base))
    }

    /// The rows at a state with no witnessed claim: `protection`, `nu` and `correction_set_check` are not granted.
    pub fn at_state(ctx: &'a Context, state: &'a State) -> Rows<'a> {
        Rows::new(ctx, At::State(state))
    }

    fn new(ctx: &'a Context, at: At<'a>) -> Rows<'a> {
        Rows { ctx, at, asked: Vec::new(), drafts: Vec::new() }
    }

    /// The names of the rows the run read, in order, one for each read the bridge answered or refused after its charge.
    pub fn asked(&self) -> &[String] {
        &self.asked
    }

    /// Each drafted set the run asked `correction_set_check` to check, as drafted, in order.
    pub fn drafts(&self) -> &[Vec<(u32, i64)>] {
        &self.drafts
    }

    fn state(&self) -> Result<&'a State, &'static str> {
        match self.at {
            At::Base(Base::Snapshot(snapshot)) => Ok(snapshot.state()),
            At::Base(Base::Derived(derived)) => Ok(derived.state()),
            At::State(state) => Ok(state),
            At::Refused(code) => Err(code),
        }
    }

    fn base(&self) -> Result<Base<'a>, &'static str> {
        match self.at {
            At::Base(base) => Ok(base),
            At::State(_) => Err(NOT_GRANTED),
            At::Refused(code) => Err(code),
        }
    }

    /// The literal letters of a menu word in the run's canonical bytes, refused with `LB1:input.incompatible_alphabet`
    /// when its alphabet is not the state's, as `LiteralWord::letters` refuses them.
    fn letters(&self, word: &Json) -> Result<Vec<Letter>, &'static str> {
        let state = self.state()?;
        let named = word.get("menu").ok_or(code::ARGUMENT_FORM)?;
        let part = |key: &str| named.get(key).and_then(Json::as_text).map(str::to_owned).ok_or(code::ARGUMENT_FORM);
        let identity = MenuIdentity {
            puzzle: part("puzzle")?,
            model_id: part("model_id")?,
            menu_id: part("menu_id")?,
            menu_order: part("menu_order")?,
            contract_rev: part("contract_rev")?,
        };
        if identity.differing(&menu_identity(&self.ctx.identity(state.menu()))).is_some() {
            return Err(INCOMPATIBLE_ALPHABET);
        }
        let menu = state.menu().ok_or(INCOMPATIBLE_ALPHABET)?;
        word::letters(self.ctx, menu, &flat_letters(word)?).map_err(|_| code::ARGUMENT_FORM)
    }

    /// The answer of row `query` with `arg`, before its conversion to the declared type.
    pub fn answer(&mut self, query: &str, arg: Option<&Json>) -> Result<Value, &'static str> {
        let ctx = self.ctx;
        match (query, arg) {
            ("heights", None) => heights(ctx, self.state()?).map_err(engine),
            ("height_counts", None) => {
                let all = heights(ctx, self.state()?).map_err(engine)?;
                all.get("height_counts").cloned().ok_or(code::STATE_HASH_MISMATCH)
            }
            ("replay", Some(word)) => {
                let letters = self.letters(word)?;
                Ok(served_replay(observed(ctx, self.state()?, &letters)?))
            }
            ("word_check", Some(arg)) => {
                let letters = self.letters(arg.get("word").ok_or(code::ARGUMENT_FORM)?)?;
                let unit = naturals(arg.get("unit").ok_or(code::ARGUMENT_FORM)?)?;
                let check = word_check(ctx, self.state()?, &letters, Some(&unit), None, None).map_err(engine)?;
                match check.get("status").and_then(Value::as_str) {
                    Some("ok") => Ok(check),
                    _ if check.get("code").and_then(Value::as_str) == Some(NAMED_OBJECT_INVALID) => {
                        Err(NAMED_OBJECT_INVALID)
                    }
                    _ => Err(code::STATE_HASH_MISMATCH),
                }
            }
            ("protection", Some(arg)) => {
                let base = self.base()?;
                let letters = self.letters(arg.get("word").ok_or(code::ARGUMENT_FORM)?)?;
                let mode = match arg.get("mode").and_then(Json::as_text) {
                    Some("net") => ProtectionMode::Net,
                    Some("strict") => ProtectionMode::Strict,
                    _ => return Err(code::ARGUMENT_FORM),
                };
                let protect = naturals(arg.get("pieces").ok_or(code::ARGUMENT_FORM)?)?;
                let protect: Vec<u32> = protect
                    .iter()
                    .map(|&p| u32::try_from(p).map_err(|_| NAMED_OBJECT_INVALID))
                    .collect::<Result<_, _>>()?;
                let identity = ctx.identity(self.state()?.menu());
                let twists: Vec<_> = letters.into_iter().map(|letter| letter.1).collect();
                match levels::derive(ctx, base, &identity, &twists, mode, &protect).map_err(engine)? {
                    Err(refusal) => Err(refusal.code),
                    Ok(Derivation::Derived(derived)) => {
                        derived.protection().map(|p| p.to_json()).ok_or(code::STATE_HASH_MISMATCH)
                    }
                    Ok(Derivation::Refused(refused)) => match &refused.protection {
                        Some(protection) => Ok(protection.to_json()),
                        None => Err(refused.refusal.code),
                    },
                }
            }
            ("nu", None) => served_nu(nu::nu(ctx, self.base()?).map_err(engine)?),
            ("correction_set_check", Some(arg)) => {
                let base = self.base()?;
                let draft = drafted(arg)?;
                let checked = corrections::correction_set_check(ctx, base, &draft).map_err(engine)?;
                self.drafts.push(draft);
                served_nu(checked)
            }
            _ => Err(code::ARGUMENT_FORM),
        }
    }
}

impl Host for Rows<'_> {
    fn tariff(&mut self, request: &Request) -> Result<RowTariff, &'static str> {
        let (query, arg) = match *request {
            Request::Snapshot { query, .. } => (query, None),
            Request::SnapshotWith { query, arg, .. } => {
                (query, Some(provenance::parse(arg).ok_or(code::ARGUMENT_FORM)?))
            }
            _ => return Err(NOT_GRANTED),
        };
        let takes = ROWS.iter().find(|(row, _)| *row == query).map(|&(_, takes)| takes).ok_or(NOT_GRANTED)?;
        let passes = match (takes, &arg) {
            (false, None) => 1,
            (true, Some(arg)) => 1 + extent(query, arg)?,
            _ => return Err(code::ARGUMENT_FORM),
        };
        Ok(RowTariff { kernel_cost: (NP as u128).saturating_mul(passes as u128), result_bound: RESULT_BOUND })
    }

    fn perform(&mut self, request: &Request) -> Result<RunValue, &'static str> {
        let (query, arg, ty) = match *request {
            Request::Snapshot { query, ty } => (query, None, ty),
            Request::SnapshotWith { query, arg, ty } => {
                (query, Some(provenance::parse(arg).ok_or(code::ARGUMENT_FORM)?), ty)
            }
            _ => return Err(NOT_GRANTED),
        };
        self.asked.push(query.to_owned());
        convert(&self.answer(query, arg.as_ref())?, ty)
    }

    fn sink(&mut self, _: &Sink, _: &RunValue, _: &[u8]) -> Result<(), &'static str> {
        Err(NOT_GRANTED)
    }
}

/// An error of the jumbling engine, as the command layer reports one.
fn engine(_: impl std::fmt::Debug) -> &'static str {
    code::STATE_HASH_MISMATCH
}

/// The passes a row's argument adds to its tariff: the letters of a named word, or the entries of a drafted set.
fn extent(query: &str, arg: &Json) -> Result<usize, &'static str> {
    let word = match query {
        "replay" => arg,
        "word_check" | "protection" => arg.get("word").ok_or(code::ARGUMENT_FORM)?,
        "correction_set_check" => return drafted(arg).map(|set| set.len()),
        _ => return Err(code::ARGUMENT_FORM),
    };
    Ok(flat_letters(word)?.len() / 2)
}

/// The integers of a menu word's letters, in order: two for each letter.
fn flat_letters(word: &Json) -> Result<Vec<i32>, &'static str> {
    let Some(Json::List(pairs)) = word.get("letters") else { return Err(code::ARGUMENT_FORM) };
    let mut flat = Vec::with_capacity(2 * pairs.len());
    for pair in pairs {
        let Json::List(pair) = pair else { return Err(code::ARGUMENT_FORM) };
        for x in pair {
            let Json::Int(x) = x else { return Err(code::ARGUMENT_FORM) };
            flat.push(i32::try_from(*x).map_err(|_| code::ARGUMENT_FORM)?);
        }
    }
    Ok(flat)
}

/// The integers of a list in a run's canonical bytes, each written `{"int": "n"}`.
fn naturals(list: &Json) -> Result<Vec<i64>, &'static str> {
    let Json::List(items) = list else { return Err(code::ARGUMENT_FORM) };
    items.iter().map(|item| integer(item).ok_or(code::ARGUMENT_FORM)).collect()
}

fn integer(item: &Json) -> Option<i64> {
    item.get("int").and_then(Json::as_text).and_then(|n| n.parse().ok())
}

/// A drafted set in a run's canonical bytes: a list of records `{ id, m }`.
fn drafted(arg: &Json) -> Result<Vec<(u32, i64)>, &'static str> {
    let Json::List(set) = arg else { return Err(code::ARGUMENT_FORM) };
    let entry = |item: &Json| {
        let id = item.get("id").and_then(integer).and_then(|id| u32::try_from(id).ok());
        Some((id?, item.get("m").and_then(integer)?))
    };
    set.iter().map(|item| entry(item).ok_or(code::ARGUMENT_FORM)).collect()
}

/// JI.10's prefix observations of a literal word at `start`, which is not changed, with no optional field.
fn observed(ctx: &Context, start: &State, letters: &[Letter]) -> Result<Value, &'static str> {
    let request = PrefixRequest {
        letters,
        fields: Fields { n: false, heights: false, fd: false, sets: false },
        cap: None,
        batch: Batch { chunk: 16, budget: None, cancel_after: None },
        keep: false,
        eval_budget: None,
    };
    Ok(prefix_observations(ctx, start, &request).map_err(engine)?.0)
}

/// The prefix observations as the `replay` row serves them: `prefix_observations` writes no after-state facts in the
/// row of a refused letter, and the row writes them as null, undefined, so that every row has an admitted row's fields.
fn served_replay(mut replay: Value) -> Value {
    if let Value::Object(fields) = &mut replay {
        if let Some(Value::Array(rows)) = fields.get_mut("rows") {
            for row in rows {
                if let Value::Object(row) = row {
                    for name in AFTER_FACTS {
                        row.entry(name.to_string()).or_insert(Value::Null);
                    }
                }
            }
        }
    }
    replay
}

/// ν's draft answer: its 45 values and no reason when ν is defined, no values and the reason when not.
fn served_nu(result: Result<Partial<NuVector>, Refusal>) -> Result<Value, &'static str> {
    match result {
        Err(refusal) => Err(refusal.code),
        Ok(Partial::Total(value)) => Ok(json::object([("reason", Value::Null), ("values", value.to_json())])),
        Ok(Partial::Undefined(reason)) => {
            Ok(json::object([("reason", Value::String(reason)), ("values", Value::Null)]))
        }
        Ok(_) => Err(code::STATE_HASH_MISMATCH),
    }
}

/// A row's answer as a value of the declared type, by the library's draft answer types: a record takes the declared
/// fields of a JSON object, each of which the answer writes, null when undefined, so that a declared field that it does
/// not write, such as a misspelled one, is refused and never read as undefined; a list takes an array, or holds nothing
/// for null and one value otherwise; a map from text takes an object. Any other shape is `LG2:type.mismatch`.
pub fn convert(value: &Value, ty: &QTy) -> Result<RunValue, &'static str> {
    Ok(match (&ty.ty, value) {
        (Ty::Bool, Value::Bool(b)) => RunValue::Bool(*b),
        (Ty::Nat, Value::Int(n)) if *n >= 0 => RunValue::int(*n),
        (Ty::Int, Value::Int(n)) => RunValue::int(*n),
        (Ty::Text, Value::String(s)) => RunValue::text(s),
        (Ty::List(item), Value::Array(values)) => {
            RunValue::list(values.iter().map(|v| convert(v, item)).collect::<Result<_, _>>()?)
        }
        (Ty::List(_), Value::Null) => RunValue::list(Vec::new()),
        (Ty::List(item), other) => RunValue::list(vec![convert(other, item)?]),
        (Ty::Map(key, item), Value::Object(entries)) if key.ty == Ty::Text => RunValue::map(
            entries.iter().map(|(k, v)| Ok((RunValue::text(k), convert(v, item)?))).collect::<Result<_, &str>>()?,
        ),
        (Ty::Record(fields), Value::Object(entries)) => {
            let mut out = Vec::with_capacity(fields.len());
            for (name, field_ty) in fields {
                out.push((name.as_str(), convert(entries.get(name).ok_or(TYPE_MISMATCH)?, field_ty)?));
            }
            RunValue::record(out)
        }
        _ => return Err(TYPE_MISMATCH),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bytes(text: &str) -> Json {
        provenance::parse(text.as_bytes()).expect("canonical bytes")
    }

    #[test]
    fn drafted_sets_and_naturals_read_the_canonical_integers() {
        let set = bytes(r#"[{"id":{"int":"2"},"m":{"int":"-1"}},{"id":{"int":"35"},"m":{"int":"7"}}]"#);
        assert_eq!(drafted(&set), Ok(vec![(2, -1), (35, 7)]));
        assert_eq!(drafted(&bytes(r#"[{"id":{"int":"-1"},"m":{"int":"1"}}]"#)), Err(code::ARGUMENT_FORM));
        assert_eq!(drafted(&bytes(r#"[{"id":{"int":"2"}}]"#)), Err(code::ARGUMENT_FORM));
        assert_eq!(naturals(&bytes(r#"[{"int":"0"},{"int":"9"}]"#)), Ok(vec![0, 9]));
        assert_eq!(naturals(&bytes(r#"{"int":"0"}"#)), Err(code::ARGUMENT_FORM));
    }

    #[test]
    fn a_word_extent_counts_its_letters() {
        let word = bytes(r#"{"letters":[[1,2],[3,4],[5,6]],"menu":{}}"#);
        assert_eq!(extent("replay", &word), Ok(3));
        assert_eq!(extent("word_check", &bytes(r#"{"unit":[],"word":{"letters":[[1,2]]}}"#)), Ok(1));
        assert_eq!(extent("protection", &bytes(r#"{"word":{"letters":[[1]]}}"#)), Ok(0));
        assert_eq!(extent("replay", &bytes(r#"{"letters":[[1,"a"]]}"#)), Err(code::ARGUMENT_FORM));
    }
}

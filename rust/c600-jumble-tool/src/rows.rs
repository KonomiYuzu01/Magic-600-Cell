//! The jumbling rows' host bridge (LP3, C8): KN2.4's jumbling rows that the library entries ([`crate::library`]) and
//! CT-RTE-07's jumbling keys read, answered by `c600_jumble` and served to a run as snapshot reads, in the library's
//! draft answer types (KN5, LP2 draft until the command table freezes, PHS.52).
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
//! - `seam_facts`, a cap ID or the text `"all"`: KN2.4's seam facts of the cap's cut at the state
//!   (`observe::seam_facts` with the cap as its scope, a whole batch and no cell rows), as a record `{ scope, status,
//!   complete, cells, domains, totals, undefined }`. `totals` holds N_c and the aligned count when the cap is
//!   admissible, and is null with the reason in `undefined` when it is not (`cap_not_admissible`). A cap outside 0..599
//!   is `KN2:input.named_object_invalid` (JI.45). The row answers for the cap it is asked about; it never surveys,
//!   ranks or selects caps. With `"all"` it scans every seam once (`Scope::All`), and `totals` holds total N with its
//!   on-grip and off-grip mismatches and the aligned count: one total, not a row for each cap.
//! - `rim_summary`, no argument: KN2.4's rim summary at the state (`observe::cuts::rim_summary` of every cap), as a
//!   record `{ status, digest, rows, complete, uncertain_caps, grip_n, grip_n_undefined }`. `rows` holds one row for
//!   each cap in cap-ID order, `{ cap, status, blocked, N_c, In_c, Out_c, A_c, F_c, at_floor, undefined }`; a blocked
//!   cap keeps its row with null rim facts and `cap_not_admissible`. Grip N is the sum of N_c over the admissible caps.
//!   The row answers every cap alike; it never ranks, selects or skips caps.
//! - `covers`, a cap ID: KN2.4's covers at the named cap (`observe::cuts::covers`), as a record `{ status,
//!   input_digest, parameters, domain, answers, message, by }`: for each of the 80 caps that meet it, in cap-ID order,
//!   both directions, each with its counts, its face-piece flag and its straddlers, or not applicable with the reason
//!   (`blocked`, `closed`). Each direction writes every field, null when not applicable. A blocked or closed named cap
//!   answers status `error` with its message and no answers; a cap outside the model is
//!   `KN2:input.named_object_invalid`. The row marks no covering cap and gives no top-cap verdict.
//! - `correction_lookup`, a correction id: the catalogue's correction with that id (`corrections::Catalogue::lookup`),
//!   as a record `{ family, id, letters, row }`: its literal letters `[grip, item]` and its fixed ν row. The letters
//!   number the I_a menu's items in its recorded order, so a state under another menu, or under I_a's items in
//!   another order, or in free mode, is `KN2:input.menu_binding_mismatch`, as `Catalogue::twists` refuses it. An id
//!   the catalogue does not ship is `KN2:input.named_object_invalid`. The lookup answers the named correction only; it
//!   checks no set and chooses no combination.
//! - `classify`, a menu word of one letter, the solver's named twist: the twist's outcome at the state
//!   (`State::apply` on a copy, with no budget and no protected piece), which is not changed, as a record
//!   `{ grip, status, moved, certificate, uncertain, code }`: `admissible`, `blocked`, `invalid` or `uncertain`, the
//!   count of pieces the twist would move (none unless admissible), the straddle certificate of a blocked twist, the
//!   pieces left uncertain and the refusal's code. A word of any other length is `KN5:input.argument_form`.
//! - `survey`, a cap ID: the status of the named grip only (`State::survey` of that grip), as a record
//!   `{ cap, status }`. `"all"` or a list is `KN5:input.argument_form`, so the research survey's all-items loop is not
//!   served (LB3.5); a cap outside 0..599 is `KN2:input.named_object_invalid`.
//! - `straddlers`, a cap ID: every certified straddler of the named cap (`State::straddler_set`), as a record
//!   `{ cap, status, pieces, certificates, uncertain }`, the pieces ascending and each certificate with its exact pair
//!   of points, each coordinate a `Q5`. A cap outside 0..599 is `KN2:input.named_object_invalid`.
//! - `units`, a list of pieces or the text `"all"`: JI.9's units (`observe::units`), as a record `{ status, units,
//!   complete }`: the named unit, or with `"all"` the exact-pose groups of the pieces off the lattice ordered by
//!   smallest member, each `{ members, pose, domain, heights, branches }`. A named unit that is empty, holds a piece
//!   outside the model or on the lattice, or spans two exact poses is `KN2:input.named_object_invalid` (JI.45).
//! - `repair_list`, a cap ID: JI.12's repair list at the named cap (`observe::cuts::repair_list`, with no pair budget
//!   and no cancellation), as a record `{ status, input_digest, parameters, rows, message }`. `parameters` names the
//!   cap, the pinned menu and its order (null in free mode), the tie rule and the contract revision. `rows` holds the
//!   completed list's pairs (x, d) whose repair lowers N, each `{ x, d, y, z, delta }`, in the engine's neutral order:
//!   x in the menu's pinned order, then d in cap-ID order. A blocked or closed named cap, or free mode, answers status
//!   `error` with its message and no rows; a cap outside the model is `KN2:input.named_object_invalid`. The engine's
//!   outcome of every pair it tried and its coverage are not served (CT-LB3-07: no all-items outcome table), and no
//!   field ranks, marks or recommends a pair; the solver picks one (the owner's decision of 17 October 2026, BND.2).
//! - `sticker_test`, no argument: A5's sticker test of the state (`levels::sticker_test`).
//! - `retained_checkpoint`, a word of 600-cell-Full's letters, the retained witness the solver names: section 5's
//!   retained-state checkpoint of the state with that word as its witness (`levels::retained_checkpoint`,
//!   `Witness::Word`). The bridge never builds a witness.
//! - `label_checkpoint`, no argument: A5's label checkpoint of the base under the base's own reachability claim
//!   (`levels::label_checkpoint`). As the command layer's exit from Jumble on reads it, ν comes from the identity
//!   labels or a retained witness where they show it, and otherwise from the ν row.
//!
//! The three end tests answer `{ level, verdict, reason }` and the fields of their evidence ([`STICKER_FIELDS`],
//! [`RETAINED_FIELDS`], [`LABEL_FIELDS`]), each written, null when the verdict has none: a verdict names the level it
//! was reached at and never crosses it (KN1.11), and unknown is never no. A label checkpoint's reachability claim is
//! served as its kind. A program reads an end test's row declared as that test's own verdict type (LB3.2, LG2.9; the
//! checker refuses any other), and [`convert`] gives it the verdict's outcome only: yes `holds`, no `fails` and unknown
//! `unestablished`. The IR draft's verdict carries no evidence, reason or level, so those stay in the row's answer.
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
//! piece that would move, a cap outside the model) stops the run with its code once the read is charged. An error of
//! the jumbling engine is `KN5:consistency.state_hash_mismatch`, as the command layer reports one (`c600-command`'s
//! `jumble` module).
//!
//! # Tariff (draft; the command table prices the rows when it freezes)
//!
//! The kernel cost is one unit for each piece of the model ([`NP`]) for each pass over a state: one pass for `heights`,
//! `height_counts`, `nu` and `correction_lookup`, and one more for each letter of a named word or each entry of a
//! drafted set; two for `seam_facts`, the cap's classification and its cut, or the whole scan with `"all"`; one more
//! for each cap ([`POLES`]) for `rim_summary`; one more for the named cap and each of its [`MEETING_CAPS`] for
//! `covers`; two for `classify`, `survey`, `straddlers` and `units`, the named twist's, grip's, cap's or unit's pass
//! and the state's; one for `sticker_test`, one more for each letter of the witness for `retained_checkpoint`, and
//! two for `label_checkpoint`, the labels and ν. A `repair_list` read is priced at its worst case under the state's
//! menu ([`repair_passes`]). The result bound is [`RESULT_BOUND`] for every row; an answer above it stops the run
//! with the interpreter's `KN7:resource_limit.host_exhausted`, never a cut answer.

use std::sync::Arc;

use c600_check::TYPE_MISMATCH;
use c600_command::{code, CommandLayer, Store};
use c600_ir::{EndTestOutcome, QTy, Ty};
use c600_jumble::codes::{IDENTITY_MISMATCH, MENU_BINDING_MISMATCH, MENU_WORD_NEEDS_JUMBLE_ON, NAMED_OBJECT_INVALID};
use c600_jumble::corrections;
use c600_jumble::levels::{self, Base, Derivation, Level, Partial, Verdict, Witness};
use c600_jumble::model::{NP, POLES};
use c600_jumble::nu::{self, NuVector};
use c600_jumble::observe::cuts::{covers, repair_list, rim_summary};
use c600_jumble::observe::{
    heights, prefix_observations, seam_facts, units, word_check, Batch, Fields, PrefixRequest, Rows as Cells, Scope,
};
use c600_jumble::state::{Certificate, State};
use c600_jumble::Context;
use c600_kernel::json::{self, Value};
use c600_kernel::{ProtectionMode, Refusal};
use c600_run::interp::{Host, Request, RowTariff, Sink};
use c600_run::menu::MenuIdentity;
use c600_run::meter::code::NOT_GRANTED;
use c600_run::provenance::{self, Json};
use c600_run::value::{parse_integer, Quad, Value as RunValue};

use crate::word::{self, menu_identity, Letter, INCOMPATIBLE_ALPHABET};

/// The rows the bridge serves, each with whether it takes an argument.
pub const ROWS: [(&str, bool); 19] = [
    ("heights", false),
    ("height_counts", false),
    ("replay", true),
    ("word_check", true),
    ("protection", true),
    ("nu", false),
    ("correction_set_check", true),
    ("seam_facts", true),
    ("rim_summary", false),
    ("covers", true),
    ("correction_lookup", true),
    ("classify", true),
    ("survey", true),
    ("straddlers", true),
    ("units", true),
    ("repair_list", true),
    ("sticker_test", false),
    ("retained_checkpoint", true),
    ("label_checkpoint", false),
];

/// The caps that meet one cap: the classifications a `covers` read adds to the named cap's own.
pub const MEETING_CAPS: usize = 80;

/// The result bound of every row's answer (a draft): 16 MiB of logical size.
pub const RESULT_BOUND: u128 = 1 << 24;

/// The after-state facts that JI.10's prefix observations write in an admitted letter's row, with no optional field
/// requested.
const AFTER_FACTS: [&str; 4] = ["phi_before", "phi_after", "delta_phi", "max_left_height"];

/// The fields of the `seam_facts` row's answer, each written, null when the engine's answer has none.
const SEAM_FIELDS: [&str; 7] = ["cells", "complete", "domains", "scope", "status", "totals", "undefined"];

/// The fields of each direction of a `covers` answer, each written, null when the engine's answer has none: an answered
/// direction has no reason or status, and one that is not applicable has no counts, flag or straddlers.
const COVER_FIELDS: [&str; 5] = ["counts", "covers", "reason", "status", "straddlers"];

/// The fields of a repair list that the `repair_list` row serves: not the engine's `outcomes`, its table of every pair
/// it tried, nor its `coverage` and `by`, which only a stopped list needs and a list with no budget never is.
const REPAIR_FIELDS: [&str; 5] = ["input_digest", "message", "parameters", "rows", "status"];

/// The names of a repair-list row's five integers, in the engine's order.
const REPAIR_ROW: [&str; 5] = ["x", "d", "y", "z", "delta"];

/// The evidence fields of the sticker test's verdicts: a yes's digests and off count, a no's fact with its count.
pub const STICKER_FIELDS: [&str; 6] = ["collisions", "fact", "labels_digest", "off", "pose_digest", "slots"];

/// The evidence fields of the retained checkpoint's verdicts: a yes's witness and digest, a no's fact with its count or
/// its centres.
pub const RETAINED_FIELDS: [&str; 6] = ["centres", "fact", "off", "pose_digest", "source", "witness_word"];

/// The evidence fields of the label checkpoint's verdicts: a yes's digests, ν with its source, the reachability claim's
/// kind and the contract stamp; a no's fact with its count, centres or ν.
pub const LABEL_FIELDS: [&str; 11] = [
    "centres",
    "collisions",
    "contract_revision",
    "fact",
    "frame_agrees",
    "labels_digest",
    "nu",
    "nu_source",
    "off",
    "pose_digest",
    "reachability",
];

/// A whole batch: the engine's cells of one cap's cut in chunks, with no budget or cancellation.
const WHOLE: Batch = Batch { chunk: 1 << 12, budget: None, cancel_after: None };

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
            ("seam_facts", Some(arg)) => {
                let scope = match arg.as_text() {
                    Some("all") => Scope::All,
                    _ => Scope::Cap(integer(arg).ok_or(code::ARGUMENT_FORM)?),
                };
                let mut state = self.state()?.clone();
                served_seam(seam_facts(ctx, &mut state, scope, &WHOLE, Cells::None, None).map_err(engine)?)
            }
            ("rim_summary", None) => {
                let mut state = self.state()?.clone();
                let summary = rim_summary(ctx, &mut state, None, None).map_err(engine)?;
                match summary.get("status").and_then(Value::as_str) {
                    Some("ok") => Ok(summary),
                    _ => Err(code::STATE_HASH_MISMATCH),
                }
            }
            ("covers", Some(cap)) => {
                let cap = integer(cap).ok_or(code::ARGUMENT_FORM)?;
                let mut state = self.state()?.clone();
                served_covers(covers(ctx, &mut state, cap, None).map_err(engine)?)
            }
            ("correction_lookup", Some(id)) => {
                let menu = self.state()?.menu();
                let id = u32::try_from(integer(id).ok_or(code::ARGUMENT_FORM)?).map_err(|_| NAMED_OBJECT_INVALID)?;
                let catalogue = corrections::catalogue().map_err(engine)?;
                // The letters number I_a's items in its recorded order, so they are answered under that menu only.
                let binding = catalogue.binding();
                if ctx.model.model_id != binding.model_id {
                    return Err(IDENTITY_MISMATCH);
                }
                match menu {
                    Some(menu) if menu.identity == binding.menu_identity && menu.order == binding.menu_order => {}
                    _ => return Err(MENU_BINDING_MISMATCH),
                }
                let correction = catalogue.lookup(id).map_err(|r| r.code)?;
                let letters = correction
                    .letters()
                    .iter()
                    .map(|&(grip, item)| Value::Array(vec![Value::Int(i64::from(grip)), Value::Int(i64::from(item))]));
                Ok(json::object([
                    ("family", Value::String(correction.family().to_owned())),
                    ("id", Value::Int(i64::from(correction.id()))),
                    ("letters", Value::Array(letters.collect())),
                    ("row", correction.row().to_json()),
                ]))
            }
            ("classify", Some(word)) => {
                let letters = self.letters(word)?;
                let [(_, twist)] = &letters[..] else { return Err(code::ARGUMENT_FORM) };
                // The named twist acts on a copy: the state read is not changed, whatever the outcome.
                let outcome = self.state()?.clone().apply(ctx, twist, None, &[]).map_err(engine)?;
                let code = outcome.refusal.as_ref().map_or(Value::Null, |(code, _)| Value::String((*code).to_owned()));
                Ok(json::object([
                    ("certificate", outcome.straddle.as_ref().map_or(Value::Null, Certificate::to_json)),
                    ("code", code),
                    ("grip", Value::Int(outcome.grip as i64)),
                    ("moved", Value::Int(outcome.moved() as i64)),
                    ("status", Value::String(outcome.status.as_str().to_owned())),
                    ("uncertain", Value::Int(outcome.uncertain as i64)),
                ]))
            }
            ("survey", Some(cap)) => {
                let cap = named_cap(cap)?;
                let statuses = self.state()?.clone().survey(ctx, &[cap], None).map_err(engine)?;
                let [status] = statuses[..] else { return Err(code::STATE_HASH_MISMATCH) };
                Ok(json::object([
                    ("cap", Value::Int(cap as i64)),
                    ("status", Value::String(status.as_str().to_owned())),
                ]))
            }
            ("straddlers", Some(cap)) => {
                let cap = named_cap(cap)?;
                let set = self.state()?.clone().straddler_set(ctx, cap, None).map_err(engine)?;
                Ok(json::object([
                    ("cap", Value::Int(cap as i64)),
                    ("certificates", Value::Array(set.certificates.iter().map(Certificate::to_json).collect())),
                    ("pieces", Value::Array(set.pieces.iter().map(|&p| Value::Int(i64::from(p))).collect())),
                    ("status", Value::String(set.status().to_owned())),
                    ("uncertain", Value::Int(set.uncertain as i64)),
                ]))
            }
            ("units", Some(arg)) => {
                let named = match arg.as_text() {
                    Some("all") => None,
                    _ => Some(naturals(arg)?),
                };
                let answer = units(ctx, self.state()?, named.as_deref()).map_err(engine)?;
                match answer.get("status").and_then(Value::as_str) {
                    Some("ok") => Ok(answer),
                    _ if answer.get("code").and_then(Value::as_str) == Some(NAMED_OBJECT_INVALID) => {
                        Err(NAMED_OBJECT_INVALID)
                    }
                    _ => Err(code::STATE_HASH_MISMATCH),
                }
            }
            ("repair_list", Some(cap)) => {
                let cap = named_cap(cap)?;
                served_repair(repair_list(ctx, self.state()?, cap as i64, None, None, None).map_err(engine)?)
            }
            ("sticker_test", None) => {
                let verdict = levels::sticker_test(ctx, self.state()?).map_err(engine)?;
                Ok(served_verdict(&verdict, levels::STICKER_TEST_LEVEL, &STICKER_FIELDS))
            }
            ("retained_checkpoint", Some(word)) => {
                let word = primitive_ids(word)?;
                let verdict = levels::retained_checkpoint(ctx, self.state()?, Witness::Word(&word)).map_err(engine)?;
                Ok(served_verdict(&verdict, levels::RETAINED_CHECKPOINT_LEVEL, &RETAINED_FIELDS))
            }
            ("label_checkpoint", None) => {
                let base = self.base()?;
                let (state, reach) = match base {
                    Base::Snapshot(snapshot) => (snapshot.state(), snapshot.reachability()),
                    Base::Derived(derived) => (derived.state(), derived.reachability()),
                };
                let checkpoint = |nu: Option<&[i64]>| levels::label_checkpoint(ctx, state, reach, nu).map_err(engine);
                let mut verdict = checkpoint(None)?;
                if matches!(verdict, Verdict::Unknown(_)) {
                    // Neither the identity labels nor a retained witness show ν here: the ν row computes it, as the
                    // command layer's exit reads it.
                    if let Partial::Total(row) = nu::nu(ctx, base).map_err(engine)?.map_err(|r| r.code)? {
                        verdict = checkpoint(Some(&row.values()[..]))?;
                    }
                }
                Ok(served_verdict(&verdict, levels::LABEL_CHECKPOINT_LEVEL, &LABEL_FIELDS))
            }
            _ => Err(code::ARGUMENT_FORM),
        }
    }

    /// The passes of a `repair_list` read at a named cap under the state's menu, at its worst case: the named cap's
    /// score and the classification of each of its [`MEETING_CAPS`]; then for each item other than the identity, its
    /// play, and for each meeting cap that cap's score, the play of y and the named cap's score again. A state with no
    /// menu, or that the bridge does not read, adds none: the read is then refused or an error.
    fn repair_passes(&self, arg: &Json) -> Result<usize, &'static str> {
        integer(arg).ok_or(code::ARGUMENT_FORM)?;
        let items = self.state().ok().and_then(State::menu).map_or(0, |menu| menu.items.len());
        Ok(repair_passes(items))
    }
}

/// The worst-case passes of a repair list under a menu of `items` items ([`Rows::repair_passes`]).
pub fn repair_passes(items: usize) -> usize {
    if items == 0 {
        return 0;
    }
    1 + MEETING_CAPS + (items - 1) * (1 + 3 * MEETING_CAPS)
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
            (false, None) => match query {
                "rim_summary" => 1 + POLES,
                "label_checkpoint" => 2,
                _ => 1,
            },
            (true, Some(arg)) if query == "repair_list" => 1 + self.repair_passes(arg)?,
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
        if matches!(ty.ty, Ty::EndTest(test) if test.row() != query) {
            // Only the test's own row answers its verdict (LB3.2): the checker refuses such a read, and so does
            // the bridge.
            return Err(TYPE_MISMATCH);
        }
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

/// The passes a row's argument adds to its tariff: the letters of a named word, the entries of a drafted set, a named
/// cap's classification (with its meeting caps' for `covers`), the named twist's, grip's or unit's pass, or none for a
/// correction id.
fn extent(query: &str, arg: &Json) -> Result<usize, &'static str> {
    let cap = |passes: usize| integer(arg).map(|_| passes).ok_or(code::ARGUMENT_FORM);
    let word = match query {
        "replay" => arg,
        "word_check" | "protection" => arg.get("word").ok_or(code::ARGUMENT_FORM)?,
        "correction_set_check" => return drafted(arg).map(|set| set.len()),
        "seam_facts" if arg.as_text() == Some("all") => return Ok(1),
        "seam_facts" | "survey" | "straddlers" => return cap(1),
        "covers" => return cap(1 + MEETING_CAPS),
        "correction_lookup" => return cap(0),
        "classify" if flat_letters(arg)?.len() == 2 => return Ok(1),
        "units" if arg.as_text() == Some("all") => return Ok(1),
        "units" => return naturals(arg).map(|_| 1),
        "retained_checkpoint" => return primitive_ids(arg).map(|word| word.len()),
        _ => return Err(code::ARGUMENT_FORM),
    };
    Ok(flat_letters(word)?.len() / 2)
}

/// A cap the solver names, in a run's canonical bytes: a cap ID of the model, or `KN2:input.named_object_invalid`.
fn named_cap(cap: &Json) -> Result<usize, &'static str> {
    let cap = integer(cap).ok_or(code::ARGUMENT_FORM)?;
    usize::try_from(cap).ok().filter(|&cap| cap < POLES).ok_or(NAMED_OBJECT_INVALID)
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

/// The letters of a 600-cell-Full word in a run's canonical bytes, a list of signed integers: the signed 1-based
/// primitive ids that a retained witness is written in (`levels::primitive`). A letter of magnitude 0 or above 1200 is
/// `KN5:input.argument_form`, as a run's word never holds one.
fn primitive_ids(word: &Json) -> Result<Vec<i64>, &'static str> {
    let Json::List(letters) = word else { return Err(code::ARGUMENT_FORM) };
    let letter = |l: &Json| match l {
        Json::Int(l) if (1..=1200).contains(&l.unsigned_abs()) => Ok(*l),
        _ => Err(code::ARGUMENT_FORM),
    };
    letters.iter().map(letter).collect()
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

/// The seam facts of one cap as the `seam_facts` row serves them: [`SEAM_FIELDS`], null where the engine's answer
/// writes none (a cap that is not admissible has no cells or domains; an admissible one has no reason). The engine's
/// JI.45 refusal of a cap outside the model is its code.
fn served_seam(facts: Value) -> Result<Value, &'static str> {
    if facts.get("status").and_then(Value::as_str) == Some("refused") {
        return match facts.get("code").and_then(Value::as_str) {
            Some(NAMED_OBJECT_INVALID) => Err(NAMED_OBJECT_INVALID),
            _ => Err(code::STATE_HASH_MISMATCH),
        };
    }
    Ok(json::object(SEAM_FIELDS.map(|name| (name, facts.get(name).cloned().unwrap_or(Value::Null)))))
}

/// The cover table of a named cap as the `covers` row serves it: the engine's answer with each direction of each
/// meeting cap's answer written in [`COVER_FIELDS`], null where the engine's answer writes none. A table that is not
/// completed (a blocked or closed named cap) is served as it is, with its status, its message and no answers. The
/// engine's JI.45 refusal of a cap outside the model is its code.
fn served_covers(mut table: Value) -> Result<Value, &'static str> {
    if table.get("status").and_then(Value::as_str) == Some("refused") {
        return match table.get("code").and_then(Value::as_str) {
            Some(NAMED_OBJECT_INVALID) => Err(NAMED_OBJECT_INVALID),
            _ => Err(code::STATE_HASH_MISMATCH),
        };
    }
    if let Value::Object(fields) = &mut table {
        if let Some(Value::Array(answers)) = fields.get_mut("answers") {
            for answer in answers {
                let Value::Object(answer) = answer else { continue };
                for direction in ["c_covers_d", "d_covers_c"] {
                    if let Some(Value::Object(facts)) = answer.get_mut(direction) {
                        for name in COVER_FIELDS {
                            facts.entry(name.to_string()).or_insert(Value::Null);
                        }
                    }
                }
            }
        }
    }
    Ok(table)
}

/// A repair list as the `repair_list` row serves it: [`REPAIR_FIELDS`] of the engine's answer, each row of a completed
/// list as a record `{ x, d, y, z, delta }` in the engine's order, and null rows for a list that is not completed. The
/// engine's JI.45 refusal of a cap outside the model is its code.
fn served_repair(list: Value) -> Result<Value, &'static str> {
    if list.get("status").and_then(Value::as_str) == Some("refused") {
        return match list.get("code").and_then(Value::as_str) {
            Some(NAMED_OBJECT_INVALID) => Err(NAMED_OBJECT_INVALID),
            _ => Err(code::STATE_HASH_MISMATCH),
        };
    }
    let record = |row: &Value| match row.as_array() {
        Some(row) if row.len() == REPAIR_ROW.len() => Ok(Value::Object(
            REPAIR_ROW.iter().zip(row).map(|(name, value)| (name.to_string(), value.clone())).collect(),
        )),
        _ => Err(code::STATE_HASH_MISMATCH),
    };
    let rows = match list.get("rows") {
        Some(Value::Array(rows)) => Value::Array(rows.iter().map(record).collect::<Result<_, _>>()?),
        Some(Value::Null) => Value::Null,
        _ => return Err(code::STATE_HASH_MISMATCH),
    };
    Ok(json::object(REPAIR_FIELDS.map(|name| match name {
        "rows" => (name, rows.clone()),
        _ => (name, list.get(name).cloned().unwrap_or(Value::Null)),
    })))
}

/// An end test's verdict as its row serves it: the level it was reached at, the verdict, an unknown's reason, and each
/// of `fields` from a yes's or a no's evidence, null where the verdict writes none. A reachability claim is served as
/// its kind.
fn served_verdict(verdict: &Verdict, level: Level, fields: &[&str]) -> Value {
    let read = verdict.to_json_at(level);
    let evidence = read.get("evidence");
    let mut served = vec![
        ("level".to_owned(), read.get("level").cloned().unwrap_or(Value::Null)),
        ("verdict".to_owned(), read.get("verdict").cloned().unwrap_or(Value::Null)),
        ("reason".to_owned(), read.get("reason").cloned().unwrap_or(Value::Null)),
    ];
    for &name in fields {
        let value = evidence.and_then(|evidence| evidence.get(name)).cloned().unwrap_or(Value::Null);
        let value = match name {
            "reachability" => value.get("kind").cloned().unwrap_or(Value::Null),
            _ => value,
        };
        served.push((name.to_owned(), value));
    }
    Value::Object(served.into_iter().collect())
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
/// for null and one value otherwise; a map from text takes an object. An integer may be one that does not fit `i64`, and
/// a `Q5` takes the engine's `[a, b, d]` for (a + b√5)/d with d nonzero, in lowest terms as a run's `Q5` keeps it. An
/// end test's verdict takes its row's answer and keeps the outcome: yes `holds`, no `fails`, unknown `unestablished`.
/// Any other shape is `LG2:type.mismatch`.
pub fn convert(value: &Value, ty: &QTy) -> Result<RunValue, &'static str> {
    Ok(match (&ty.ty, value) {
        (Ty::EndTest(test), Value::Object(_)) => RunValue::EndTest(*test, outcome(value)?),
        (Ty::Bool, Value::Bool(b)) => RunValue::Bool(*b),
        (Ty::Nat, Value::Int(n)) if *n >= 0 => RunValue::int(*n),
        (Ty::Int, Value::Int(n)) => RunValue::int(*n),
        (Ty::Nat, Value::Number(lexeme)) if !lexeme.starts_with('-') => {
            RunValue::int(parse_integer(lexeme).ok_or(TYPE_MISMATCH)?)
        }
        (Ty::Int, Value::Number(lexeme)) => RunValue::int(parse_integer(lexeme).ok_or(TYPE_MISMATCH)?),
        (Ty::Q5, Value::Array(triple)) => q5(triple)?,
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

/// The outcome of an end test's answer, KN0.5's yes, no or unknown, as the IR draft's verdict names it (LG2.9).
fn outcome(answer: &Value) -> Result<EndTestOutcome, &'static str> {
    match answer.get("verdict").and_then(Value::as_str) {
        Some("yes") => Ok(EndTestOutcome::Holds),
        Some("no") => Ok(EndTestOutcome::Fails),
        Some("unknown") => Ok(EndTestOutcome::Unestablished),
        _ => Err(TYPE_MISMATCH),
    }
}

/// A number of Q(√5) as the jumbling engine writes one, `[a, b, d]` for (a + b√5)/d, as a run's `Q5`.
fn q5(triple: &[Value]) -> Result<RunValue, &'static str> {
    let exact = |v: &Value| match v {
        Value::Int(n) => parse_integer(&n.to_string()),
        Value::Number(lexeme) => parse_integer(lexeme),
        _ => None,
    };
    let [a, b, d] = triple else { return Err(TYPE_MISMATCH) };
    if matches!(d, Value::Int(0)) {
        return Err(TYPE_MISMATCH);
    }
    let (a, b, d) = (exact(a).ok_or(TYPE_MISMATCH)?, exact(b).ok_or(TYPE_MISMATCH)?, exact(d).ok_or(TYPE_MISMATCH)?);
    Ok(RunValue::Q5(Arc::new(Quad::new(a, b, d))))
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

    #[test]
    fn named_twists_grips_caps_and_units_take_one_object() {
        assert_eq!(extent("classify", &bytes(r#"{"letters":[[1,2]],"menu":{}}"#)), Ok(1));
        assert_eq!(extent("classify", &bytes(r#"{"letters":[[1,2],[3,4]],"menu":{}}"#)), Err(code::ARGUMENT_FORM));
        assert_eq!(extent("classify", &bytes(r#"{"letters":[],"menu":{}}"#)), Err(code::ARGUMENT_FORM));
        for query in ["survey", "straddlers"] {
            assert_eq!(extent(query, &bytes(r#"{"int":"7"}"#)), Ok(1), "{query}");
            assert_eq!(extent(query, &bytes(r#""all""#)), Err(code::ARGUMENT_FORM), "{query}: not every grip");
            assert_eq!(extent(query, &bytes(r#"[{"int":"7"}]"#)), Err(code::ARGUMENT_FORM), "{query}: one cap");
        }
        assert_eq!(named_cap(&bytes(r#"{"int":"599"}"#)), Ok(599));
        assert_eq!(named_cap(&bytes(r#"{"int":"600"}"#)), Err(NAMED_OBJECT_INVALID));
        assert_eq!(named_cap(&bytes(r#"{"int":"-1"}"#)), Err(NAMED_OBJECT_INVALID));
        assert_eq!(extent("units", &bytes(r#""all""#)), Ok(1));
        assert_eq!(extent("units", &bytes(r#"[{"int":"3"},{"int":"9"}]"#)), Ok(1));
        assert_eq!(extent("units", &bytes(r#"{"int":"3"}"#)), Err(code::ARGUMENT_FORM));
    }

    #[test]
    fn an_exact_number_reads_the_engines_form() {
        let ty = |ty: Ty| QTy { ty, display: false };
        let n = |s: &str| parse_integer(s).expect("an integer");
        let quad = |text: &[u8]| match convert(&json::parse(text).expect("JSON"), &ty(Ty::Q5)) {
            Ok(RunValue::Q5(q)) => Ok(Quad::new(q.a.clone(), q.b.clone(), q.d.clone())),
            Ok(other) => panic!("no Q5: {other:?}"),
            Err(code) => Err(code),
        };
        assert_eq!(quad(b"[1, 1, 2]"), Ok(Quad::new(n("1"), n("1"), n("2"))), "(1 + √5)/2");
        assert_eq!(quad(b"[2, 2, -4]"), Ok(Quad::new(n("-1"), n("-1"), n("2"))), "in lowest terms, d > 0");
        assert_eq!(
            quad(b"[123456789012345678901234567890, 0, 1]"),
            Ok(Quad::new(n("123456789012345678901234567890"), n("0"), n("1")))
        );
        for refused in ["[1, 1, 0]", "[1, 1]", "[1, \"1\", 2]", "[1.5, 1, 2]"] {
            assert_eq!(quad(refused.as_bytes()), Err(TYPE_MISMATCH), "{refused}");
        }
        let integer = |text: &[u8], t: Ty| match convert(&json::parse(text).expect("JSON"), &ty(t)) {
            Ok(RunValue::Int(i)) => Ok(i.to_string()),
            Ok(other) => panic!("no integer: {other:?}"),
            Err(code) => Err(code),
        };
        let big = b"123456789012345678901234567890";
        assert_eq!(integer(big, Ty::Int), Ok("123456789012345678901234567890".to_owned()));
        assert_eq!(integer(big, Ty::Nat), Ok("123456789012345678901234567890".to_owned()));
        assert_eq!(integer(b"-123456789012345678901234567890", Ty::Nat), Err(TYPE_MISMATCH));
        assert_eq!(integer(b"1.5", Ty::Int), Err(TYPE_MISMATCH));
    }
}

//! Stage 18's `steps.json` records in both forms, the files of each step, and the scope of what stage 18 holds
//! (shared by the stage 18 sub-checkers).
//!
//! A record (stage 18 `results/d1-tail/steps.json`, T6, and `results/rule-stop/steps.json`, the rule's three stages) is
//! `{"start_digest", "form"?, "steps": [{"dir", "grip", "item", "start_digest", "end_digest", "off_before",
//! "off_after", "X", "V", "letters", "unit_size", "x_sha256"?, "w_sha256", "v_sha256", "winv_sha256"}, ...]}`. The form
//! is `four-segment` (X·W·V′·W⁻¹·V′⁻¹, the default when the field is absent) or `three-segment` (X·W·V′·W⁻¹). Each step
//! names a directory holding `x.json.gz` (exactly when X > 0), `w.json`, `v.json.gz`, `winv.json` and `unit.json`; a
//! check receives each file as the input whose role is `<dir>/<file>`, the I_a menu as `menu`, the start table as
//! `start` and, optionally, a stored table of the chain's end as `end`. No other input is accepted.
//!
//! The record's own claims, checked before any input is read:
//!
//! - every directory is a plain name, and no two steps share one;
//! - X ≥ 0, V ≥ 1, the unit size ≥ 1, a preparation exactly when X > 0 and then with its hash;
//! - the exact length X + 2 + copies·V (copies 2 in the four-segment form, 1 in the three-segment form) is the
//!   stated `letters` and at most 10⁷, before any word is expanded (LB5.12, gate 1);
//! - the off count drops by exactly the unit size, and the steps chain: the first starts at the record's start, every
//!   later one at the end of the one before and with its off count.
//!
//! Each step's files ([`read_step`]): every word file has the recorded SHA-256 of the JSON text of its word and the
//! recorded length (X, 1 or V); W is the record's (grip, item); W⁻¹, a core P·Q·P⁻¹, expands to the one letter that
//! inverts W in the exact menu and has the recorded hash; the digests a word file names chain (X starts at the step's
//! start, W at X's end or the step's start, V′ at W's end); the unit has the recorded size. These are consistency
//! claims between stored values; a state digest is only established by replaying the letters (gates 2 and 3).

use std::sync::Arc;

use c600_jumble::model::POLES;
use c600_jumble::nu::{I_A_MENU_IDENTITY, I_A_ORDER_DIGEST};
use c600_jumble::state::State;
use c600_jumble::twist::Menu;
use c600_jumble::Context;
use c600_kernel::json::{self, Value};

use super::words::{self, admit, code, decode, read_core, read_unit, stream_word_file, text_limit, ITEMS};
use crate::claims::{
    array, at, count, dot, field, hex64, keys, not_applicable, object, reject, start_mismatch, string, Run, Step, Stop,
};
use crate::replay::{inverse_item, read_menu, read_state};
use crate::report::{codes, Span};
use crate::Engines;

/// What the `steps.json` sub-checkers open with: the record and its own claims, the inputs it names, the I_a menu
/// (by identity and order) and the inverse of each item.
pub struct Opened<'e> {
    pub ctx: &'e Context,
    pub chain: Chain,
    pub menu: Arc<Menu>,
    pub inverse: Vec<usize>,
}

/// Opens a `steps.json` record. A record the plan leaves unchecked is `not_applicable` with its scope ([`unchecked`]);
/// anything else that departs from the record's form fails, naming the field.
pub fn open<'e>(engines: &Engines<'e>, run: &mut Run, scope: &str) -> Step<Opened<'e>> {
    let ctx = engines.jumble.ok_or_else(|| reject("certificate", "the jumbling engine is absent"))?;
    let doc = parse_record(run.cert.bytes, scope)?;
    if doc.get("format").and_then(Value::as_str) == Some(RUN_FORMAT) {
        return Err(reject(
            "format",
            "the lattice run record (runner-run/3) is checked by s18.lattice_run and s18.lattice_replay; this checker \
             reads steps.json records",
        ));
    }
    let chain = read_chain(run, &doc)?;
    check_roles(run, &chain, &["menu", "start"], &["end"])?;
    let menu = read_menu(ctx, run, "menu")?;
    run.claim("menu.identity", &I_A_MENU_IDENTITY.to_owned(), &menu.identity)?;
    run.claim("menu.order", &I_A_ORDER_DIGEST.to_owned(), &menu.order)?;
    let inverse = inverses(&menu)?;
    Ok(Opened { ctx, chain, menu, inverse })
}

/// The stored start table: present, reproducing its own digest, and the record's start; otherwise
/// `KN6:evidence.start_table_mismatch`.
pub fn start_state(opened: &Opened, run: &mut Run) -> Step<State> {
    if run.optional_input("start").is_none() {
        return Err(start_mismatch("start", "the record's start table is absent"));
    }
    let state = read_state(opened.ctx, run, &opened.menu, "start")?;
    let digest = state.digest(opened.ctx);
    run.compare("start_digest", &opened.chain.start, &digest, codes::START_TABLE_MISMATCH)?;
    Ok(state)
}

/// The optional stored table of the chain's end: its digest and off count are the last step's, and, when the chain was
/// replayed, the digest reached.
pub fn end_table(opened: &Opened, run: &mut Run, reached: Option<&str>) -> Step<()> {
    if run.optional_input("end").is_none() {
        return Ok(());
    }
    let last = opened.chain.steps.last().expect("a record has steps");
    let state = read_state(opened.ctx, run, &opened.menu, "end")?;
    let digest = state.digest(opened.ctx);
    run.claim("end.digest", &last.end, &digest)?;
    run.claim("end.off", &last.off_after, &(state.off_lattice_count() as u64))?;
    if let Some(reached) = reached {
        run.claim("end.reached", &digest, &reached.to_owned())?;
    }
    Ok(())
}

/// The two recorded forms of a step.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Form {
    /// X·W·V′·W⁻¹·V′⁻¹, T6's form and the default.
    FourSegment,
    /// X·W·V′·W⁻¹, the rule's form.
    ThreeSegment,
}

impl Form {
    /// How often a step plays V′.
    pub fn copies(self) -> u64 {
        match self {
            Form::FourSegment => 2,
            Form::ThreeSegment => 1,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Form::FourSegment => "four-segment",
            Form::ThreeSegment => "three-segment",
        }
    }
}

/// One step as the record states it.
#[derive(Clone, Debug)]
pub struct StepRecord {
    /// The claim path, `steps[k]`.
    pub path: String,
    pub dir: String,
    pub grip: usize,
    pub item: usize,
    pub start: String,
    pub end: String,
    pub off_before: u64,
    pub off_after: u64,
    pub x: u64,
    pub v: u64,
    pub letters: u64,
    pub unit_size: u64,
    pub x_sha: Option<String>,
    pub w_sha: String,
    pub v_sha: String,
    pub winv_sha: String,
}

impl StepRecord {
    /// The input role of one of the step's files.
    pub fn role(&self, file: &str) -> String {
        format!("{}/{file}", self.dir)
    }

    /// The input roles of the step's files.
    pub fn roles(&self) -> Vec<String> {
        let mut roles = Vec::new();
        if self.x > 0 {
            roles.push(self.role(X_FILE));
        }
        for file in [W_FILE, V_FILE, WINV_FILE, UNIT_FILE] {
            roles.push(self.role(file));
        }
        roles
    }
}

pub const X_FILE: &str = "x.json.gz";
pub const W_FILE: &str = "w.json";
pub const V_FILE: &str = "v.json.gz";
pub const WINV_FILE: &str = "winv.json";
pub const UNIT_FILE: &str = "unit.json";

/// A record: its start, its form and its steps.
#[derive(Clone, Debug)]
pub struct Chain {
    pub start: String,
    pub form: Form,
    pub steps: Vec<StepRecord>,
}

/// The lattice run record's format (stage 13's runner, `runner-run/3`).
pub const RUN_FORMAT: &str = "runner-run/3";

/// The fields of a stop unit (`results/stop-units/*.json`), the static tail's records.
pub const STOP_UNIT_KEYS: [&str; 9] =
    ["block", "digest", "frames", "inside", "movers", "pieces", "size", "state", "straddled"];

/// A record the plan leaves unchecked, as KN6.6's `not_applicable` with its scope (nothing is judged beyond the
/// kind): a whole-cap output (`results/wholecap/*.txt`, a text whose first line is `ELIGIBILITY <digest> off ...`) or a
/// stop unit (a JSON object with exactly [`STOP_UNIT_KEYS`]). Static whole-cap plans, whole-cap negatives and plan
/// tables have no independent checker (LB5.15). Every other certificate is read as the sub-checker's own form, and a
/// departure from it fails, naming the field.
pub fn unchecked(bytes: &[u8], scope: &str) -> Option<Stop> {
    const STATIC: &str = "static whole-cap plans, whole-cap negatives and plan tables have no independent checker: a \
        finite flow, group and plan checker needs every action and group witness, hole demand and exact endpoint join \
        of the plan (matrix family 18/static_tail, LB5.15), and static existence is neither a built word nor a \
        gate-3 replay";
    let wholecap = bytes
        .strip_prefix(b"ELIGIBILITY ")
        .and_then(|rest| rest.get(..69))
        .is_some_and(|head| head[..64].iter().all(u8::is_ascii_hexdigit) && &head[64..] == b" off ");
    if wholecap {
        return Some(not_applicable(format!(
            "a whole-cap output of results/wholecap/: {STATIC}; this checker covers {scope}"
        )));
    }
    let doc = json::parse(bytes).ok()?;
    let fields = doc.as_object()?;
    if fields.len() == STOP_UNIT_KEYS.len() && STOP_UNIT_KEYS.iter().all(|key| fields.contains_key(*key)) {
        return Some(not_applicable(format!(
            "a unit of the stop (results/stop-units/): {STATIC}; this checker covers {scope}"
        )));
    }
    None
}

/// The certificate as a JSON record of the sub-checker's form: a record the plan leaves unchecked stops as
/// `not_applicable`, anything that is not JSON fails.
pub fn parse_record(bytes: &[u8], scope: &str) -> Step<Value> {
    if let Some(stop) = unchecked(bytes, scope) {
        return Err(stop);
    }
    json::parse(bytes).map_err(|error| reject("certificate", format!("not a JSON record: {error}")))
}

/// Reads a record and checks its own claims. No input is read.
pub fn read_chain(run: &mut Run, doc: &Value) -> Step<Chain> {
    keys(doc, "certificate", &["start_digest", "steps"], &["form"])?;
    let start = hex64(field(doc, "start_digest", "start_digest")?, "start_digest")?.to_owned();
    let form = match doc.get("form") {
        None => Form::FourSegment,
        Some(value) => match string(value, "form")? {
            "four-segment" => Form::FourSegment,
            "three-segment" => Form::ThreeSegment,
            other => return Err(reject("form", format!("the form {other} is neither four-segment nor three-segment"))),
        },
    };
    run.held("form", form.name());
    let list = array(field(doc, "steps", "steps")?, "steps")?;
    if list.is_empty() {
        return Err(reject("steps", "the record has no step"));
    }
    let mut steps: Vec<StepRecord> = Vec::new();
    for (k, value) in list.iter().enumerate() {
        let path = at("steps", k);
        let step = read_step_record(value, &path)?;
        let names = |key: &str| dot(&path, key);
        if steps.iter().any(|s| s.dir == step.dir) {
            return Err(reject(names("dir"), format!("the directory {} names two steps", step.dir)));
        }
        let stated = step.letters;
        let computed = step.x + 2 + form.copies() * step.v;
        run.claim(&names("letters"), &stated, &computed)?;
        admit(computed, &names("letters"))?;
        run.held(names("admission"), format!("{computed} letters, at most 10^7, before expansion"));
        if step.off_before < step.unit_size {
            return Err(reject(names("unit_size"), "the unit is larger than the off count"));
        }
        run.claim(&names("off_after"), &step.off_after, &(step.off_before - step.unit_size))?;
        let (previous_end, previous_off) = match steps.last() {
            Some(before) => (before.end.clone(), Some(before.off_after)),
            None => (start.clone(), None),
        };
        run.claim(&names("start_digest"), &step.start, &previous_end)?;
        if let Some(off) = previous_off {
            run.claim(&names("off_before"), &step.off_before, &off)?;
        }
        steps.push(step);
    }
    Ok(Chain { start, form, steps })
}

fn read_step_record(value: &Value, path: &str) -> Step<StepRecord> {
    let required = [
        "dir",
        "grip",
        "item",
        "start_digest",
        "end_digest",
        "off_before",
        "off_after",
        "X",
        "V",
        "letters",
        "unit_size",
        "w_sha256",
        "v_sha256",
        "winv_sha256",
    ];
    keys(value, path, &required, &["x_sha256"])?;
    let get = |key: &str| field(value, key, path);
    let name = |key: &str| dot(path, key);
    let dir = string(get("dir")?, &name("dir"))?.to_owned();
    if dir.is_empty() || dir == "." || dir == ".." || dir.contains(['/', '\\']) {
        return Err(reject(name("dir"), "a step directory is a plain name"));
    }
    let grip = count(get("grip")?, &name("grip"))?;
    let item = count(get("item")?, &name("item"))?;
    if grip >= POLES as u64 || item >= ITEMS as u64 {
        return Err(reject(name("grip"), "W's grip is 0 to 599 and its item 0 to 59"));
    }
    let x = count(get("X")?, &name("X"))?;
    let v = count(get("V")?, &name("V"))?;
    let unit_size = count(get("unit_size")?, &name("unit_size"))?;
    if v == 0 {
        return Err(reject(name("V"), "V′ has at least one letter"));
    }
    if unit_size == 0 {
        return Err(reject(name("unit_size"), "the unit has at least one piece"));
    }
    let x_sha = match object(value, path)?.get("x_sha256") {
        Some(sha) => Some(hex64(sha, &name("x_sha256"))?.to_owned()),
        None => None,
    };
    if x_sha.is_some() != (x > 0) {
        return Err(reject(name("X"), "a step has a preparation exactly when X > 0, and then its hash"));
    }
    let hex = |key: &str| -> Step<String> { Ok(hex64(get(key)?, &name(key))?.to_owned()) };
    Ok(StepRecord {
        path: path.to_owned(),
        dir,
        grip: grip as usize,
        item: item as usize,
        start: hex("start_digest")?,
        end: hex("end_digest")?,
        off_before: count(get("off_before")?, &name("off_before"))?,
        off_after: count(get("off_after")?, &name("off_after"))?,
        x,
        v,
        letters: count(get("letters")?, &name("letters"))?,
        unit_size,
        x_sha,
        w_sha: hex("w_sha256")?,
        v_sha: hex("v_sha256")?,
        winv_sha: hex("winv_sha256")?,
    })
}

/// Checks that the supplied inputs are exactly the record's files and the named extra roles (`required` must be
/// present, `optional` may be).
pub fn check_roles(run: &Run, chain: &Chain, required: &[&str], optional: &[&str]) -> Step<()> {
    let mut named: Vec<String> = chain.steps.iter().flat_map(StepRecord::roles).collect();
    named.extend(required.iter().map(|r| (*r).to_owned()));
    named.extend(optional.iter().map(|r| (*r).to_owned()));
    for supplied in run.cert.inputs {
        if !named.iter().any(|n| n == supplied.role) {
            return Err(reject("inputs", format!("the input {} is not named by the record", supplied.role)));
        }
    }
    Ok(())
}

/// The trace text of a step's expansion: its parts and exact length.
pub fn expansion_note(step: &StepRecord, form: Form) -> String {
    match form {
        Form::FourSegment => format!(
            "X {} + W 1 + V' {} + W^-1 1 + V'^-1 {} = {} letters; each stored part streamed once, V'^-1 counted from V'",
            step.x, step.v, step.v, step.letters
        ),
        Form::ThreeSegment => {
            format!("X {} + W 1 + V' {} + W^-1 1 = {} letters; each stored part streamed once", step.x, step.v, step.letters)
        }
    }
}

/// The inverse item of every I_a item, from the exact menu.
pub fn inverses(menu: &Menu) -> Step<Vec<usize>> {
    (0..menu.items.len())
        .map(|item| inverse_item(menu, item))
        .collect::<Option<Vec<_>>>()
        .ok_or_else(|| reject("menu", "the menu is not inverse-closed"))
}

/// A step's words, as codes (60·cap + item), and the digests its files name.
#[derive(Clone, Debug, Default)]
pub struct StepWords {
    pub x: Vec<u16>,
    /// X's input and output digests, with a preparation.
    pub x_digests: Option<(String, String)>,
    pub w: u16,
    pub w_digests: (String, String),
    pub v: Vec<u16>,
    pub v_input: String,
    pub winv: u16,
    pub unit: Vec<u32>,
}

impl StepWords {
    /// The step's expanded word, X·W·V′·W⁻¹ and, in the four-segment form, V′⁻¹, as codes.
    pub fn expanded(&self, form: Form, inverse: &[usize]) -> Vec<u16> {
        let mut word = Vec::with_capacity(self.x.len() + 2 + self.v.len() * form.copies() as usize);
        word.extend_from_slice(&self.x);
        word.push(self.w);
        word.extend_from_slice(&self.v);
        word.push(self.winv);
        if form == Form::FourSegment {
            word.extend(self.v.iter().rev().map(|&c| invert(c, inverse)));
        }
        word
    }
}

/// The inverse letter of a code.
pub fn invert(code_: u16, inverse: &[usize]) -> u16 {
    let (cap, item) = words::letter_of(code_);
    code(cap, inverse[item])
}

/// The SHA-256 of a word's codes as little-endian uint16 (the digest an `ia16` file of the word has).
pub fn word_digest(codes: &[u16]) -> String {
    let mut hash = c600_digest::Sha256::new();
    for chunk in codes.chunks(1 << 15) {
        let bytes: Vec<u8> = chunk.iter().flat_map(|c| c.to_le_bytes()).collect();
        hash.update(&bytes);
    }
    c600_digest::hex(&hash.finish())
}

/// Reads and checks one step's files against its record (see the module documentation). With `keep`, the letters
/// are kept as codes for a replay; without it every word is only streamed through its length and hash checks.
pub fn read_step(run: &mut Run, step: &StepRecord, inverse: &[usize], keep: bool) -> Step<StepWords> {
    let path = &step.path;
    let mut words = StepWords::default();

    // X, when the step is prepared.
    let mut before_w = step.start.clone();
    if let Some(x_sha) = &step.x_sha {
        let claim = dot(path, "x");
        let file = word_file(
            run,
            step,
            X_FILE,
            &claim,
            step.x,
            x_sha,
            &["input_digest", "output_digest"],
            keep,
            &mut words.x,
        )?;
        let (input, output) = (file.input_digest.unwrap_or_default(), file.output_digest.unwrap_or_default());
        run.claim(&dot(&claim, "input_digest"), &input, &step.start)?;
        before_w = output.clone();
        words.x_digests = Some((input, output));
    }

    // W, one letter: the record's (grip, item).
    let claim = dot(path, "w");
    let mut w = Vec::new();
    let file = word_file(run, step, W_FILE, &claim, 1, &step.w_sha, &["input_digest", "output_digest"], true, &mut w)?;
    let stated = code(step.grip, step.item);
    if w.first() != Some(&stated) {
        return Err(reject(
            dot(&claim, "word"),
            format!("W is not the record's letter [{}, {}]", step.grip, step.item),
        ));
    }
    run.held(dot(&claim, "letter"), format!("[{}, {}]", step.grip, step.item));
    words.w = stated;
    let (input, output) = (file.input_digest.unwrap_or_default(), file.output_digest.unwrap_or_default());
    run.claim(&dot(&claim, "input_digest"), &input, &before_w)?;
    words.w_digests = (input, output.clone());

    // V′, read once; the four-segment form plays it a second time inverted.
    let claim = dot(path, "v");
    let file = word_file(run, step, V_FILE, &claim, step.v, &step.v_sha, &["input_digest"], keep, &mut words.v)?;
    let input = file.input_digest.unwrap_or_default();
    run.claim(&dot(&claim, "input_digest"), &input, &output)?;
    words.v_input = input;

    // W⁻¹, a core P·Q·P⁻¹ expanding to the inverse letter of W.
    let claim = dot(path, "winv");
    let bytes = run.input(&step.role(WINV_FILE))?;
    let (p, q, sha) = read_core(bytes, &claim)?;
    run.bind(&step.role(WINV_FILE), bytes, vec![("sha256", sha.clone())]);
    run.claim(&dot(&claim, "sha256"), &step.winv_sha, &sha)?;
    let mut expanded: Vec<u16> = p.iter().map(|&(c, i)| code(c, i)).collect();
    expanded.extend(q.iter().map(|&(c, i)| code(c, i)));
    expanded.extend(p.iter().rev().map(|&(c, i)| invert(code(c, i), inverse)));
    let wanted = code(step.grip, inverse[step.item]);
    if expanded != [wanted] {
        return Err(reject(
            dot(&claim, "word"),
            format!("W⁻¹ expands to {} letters, not the one letter inverting W", expanded.len()),
        ));
    }
    run.held(dot(&claim, "letter"), format!("[{}, {}]", step.grip, inverse[step.item]));
    words.winv = wanted;

    // The unit.
    let claim = dot(path, "unit");
    let bytes = run.input(&step.role(UNIT_FILE))?;
    let unit = read_unit(bytes, &claim)?;
    run.bind(&step.role(UNIT_FILE), bytes, vec![("pieces", unit.len().to_string())]);
    run.claim(&dot(path, "unit_size"), &step.unit_size, &(unit.len() as u64))?;
    words.unit = unit;
    Ok(words)
}

/// One word file of a step, streamed: its length and hash checked against the record, its letters kept as codes when
/// `keep` is set.
#[allow(clippy::too_many_arguments)]
fn word_file(
    run: &mut Run,
    step: &StepRecord,
    file: &str,
    claim: &str,
    declared: u64,
    stated: &str,
    digests: &[&str],
    keep: bool,
    out: &mut Vec<u16>,
) -> Step<words::WordFile> {
    let role = step.role(file);
    let bytes = run.input(&role)?;
    let text = decode(file, bytes, text_limit(declared, claim)?, claim)?;
    if keep {
        out.reserve(declared as usize);
    }
    let read = stream_word_file(&text, claim, declared, digests, |_, cap, item| {
        if keep {
            out.push(code(cap, item));
        }
        Ok(())
    })?;
    drop(text);
    run.bind(&role, bytes, vec![("letters", read.letters.to_string()), ("sha256", read.sha256.clone())]);
    run.claim(&dot(claim, "letters"), &declared, &read.letters)?;
    for key in digests {
        let value = match *key {
            "input_digest" => &read.input_digest,
            _ => &read.output_digest,
        };
        if let Some(text) = value {
            if !(text.len() == 64 && text.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))) {
                return Err(reject(dot(claim, key), "expected a lowercase SHA-256 digest"));
            }
        }
    }
    if read.sha256 != stated {
        let stop = reject(dot(claim, "sha256"), format!("stated {stated}, computed {}", read.sha256));
        return Err(run.failed(stop, &read.sha256));
    }
    run.held_span(dot(claim, "sha256"), &read.sha256, Span { word: read.sha256.clone(), from: 0, to: read.letters });
    Ok(read)
}

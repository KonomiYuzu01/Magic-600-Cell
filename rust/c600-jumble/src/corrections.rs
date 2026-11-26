//! The shipped correction catalogue and the drafted correction-set check (LP2: DT-J1 O7 (a), JI.13, EH-11; KN2.4's
//! `correction_set_check` row).
//!
//! The 36 corrections of the pinned research library ship as constructions that the solver selects by id (DT-J1 O7
//! (a), owner, 22 October 2026, the one named exception to "no research word becomes program content"). Each carries
//! its family, its literal word as (grip, item) letters of the pinned I_a menu, and its ν row ([`crate::nu`]): the
//! change the word makes to ν from any reachable lattice configuration under I_a (EH-11). The data is
//! `data/corrections-I_a.json`, written from the research library (decompressed SHA-256 [`LIBRARY_SHA256`]) by
//! `research/jumbling/differential/references/catalogue.py` and embedded in the crate, so the engine still reads only
//! `assets/` at run time. It is bound to the model ID, the I_a menu identity and order digest, and the engine's
//! contract revision.
//!
//! - [`catalogue`]: the 36 corrections in fixed id order, checked once when first read.
//! - [`Catalogue::lookup`]: one correction by id. An unknown id is refused with `KN2:input.named_object_invalid`, a
//!   draft: chapter 13's row names caps and pieces, and no row names correction ids.
//! - [`Catalogue::twists`]: the literal word as twists of a menu whose identity and order digest are I_a's, since the
//!   letters number the items in I_a's recorded order. It previews and executes nothing: each selection still needs a
//!   fresh whole-word preview (JI.13).
//! - [`Catalogue::predict`]: ν + Σ m·row for a drafted set of (id, signed multiplicity), in F2 for the 43 bits and F5
//!   for the two C5 values.
//! - [`correction_set_check`]: KN2.4's row: ν of the base ([`crate::nu::nu`]), then [`Catalogue::predict`]. It takes
//!   a base under I_a by the menu's order-free identity, as ν's own I_a gate does: it uses rows, not letters.
//!
//! Nothing here solves, lights, filters, ranks or reorders corrections, or proposes a set (JI.13).

use std::collections::hash_map::{Entry, HashMap};
use std::sync::OnceLock;

use c600_kernel::json::{self, Value};
use c600_kernel::Refusal;

use crate::levels::{Base, Partial, NU_LEN};
use crate::model::POLES;
use crate::nu::{self, NuVector, I_A_FIXED, I_A_MENU_IDENTITY, I_A_ORDER_DIGEST, MODULI, NAMES};
use crate::state::EngineError;
use crate::twist::{Menu, Twist};
use crate::{codes, Context, CONTRACT_REVISION};

/// The catalogue file's format.
pub const FORMAT: &str = "c600-jumble-corrections-v1";
/// The decompressed SHA-256 of the research library the catalogue is written from.
pub const LIBRARY_SHA256: &str = "8a13eb6c4f4359ce4dd58a0018cf86eed3089111e11f20b7c00476f6393a661a";
/// The number of shipped corrections.
pub const COUNT: usize = 36;
/// The research library's path in the repository, as the catalogue records it.
const LIBRARY_PATH: &str = "research/jumbling/restoration/09-general-method/data/library-merged.json.gz";

static DATA: &[u8] = include_bytes!("../data/corrections-I_a.json");
static CATALOGUE: OnceLock<Result<Catalogue, String>> = OnceLock::new();

/// What the catalogue is bound to: the model, the I_a menu by identity and order digest, and the contract revision.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Binding {
    pub model_id: String,
    pub menu_identity: String,
    pub menu_order: String,
    pub contract_revision: String,
}

/// One shipped correction.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Correction {
    id: u32,
    family: String,
    letters: Vec<(u16, u8)>,
    row: NuVector,
}

impl Correction {
    pub fn id(&self) -> u32 {
        self.id
    }

    /// The research family the construction comes from.
    pub fn family(&self) -> &str {
        &self.family
    }

    /// The literal word: (grip, item) letters of the I_a menu, item in the menu's recorded order.
    pub fn letters(&self) -> &[(u16, u8)] {
        &self.letters
    }

    /// The change the word makes to ν (EH-11).
    pub fn row(&self) -> &NuVector {
        &self.row
    }
}

/// The shipped catalogue.
#[derive(Debug)]
pub struct Catalogue {
    binding: Binding,
    library: String,
    rows: Vec<Correction>,
}

/// The catalogue, read and checked once. A file that does not hold the 36 corrections in id order, bound to I_a and
/// this contract revision, with A5's coordinate names and rows whose fixed coordinates are 0, is an engine error.
pub fn catalogue() -> Result<&'static Catalogue, EngineError> {
    CATALOGUE.get_or_init(|| read(DATA)).as_ref().map_err(|e| EngineError(format!("correction catalogue: {e}")))
}

fn read(bytes: &[u8]) -> Result<Catalogue, String> {
    let doc = json::parse(bytes).map_err(|e| e.0.to_owned())?;
    let text = |v: Option<&Value>, what: &str| v.and_then(Value::as_str).map(str::to_owned).ok_or(format!("no {what}"));
    if doc.get("format").and_then(Value::as_str) != Some(FORMAT) {
        return Err("the format is not c600-jumble-corrections-v1".into());
    }
    let binding = doc.get("binding").ok_or("no binding")?;
    let binding = Binding {
        model_id: text(binding.get("model_id"), "model_id")?,
        menu_identity: text(binding.get("menu_identity"), "menu_identity")?,
        menu_order: text(binding.get("menu_order"), "menu_order")?,
        contract_revision: text(binding.get("contract_revision"), "contract_revision")?,
    };
    if binding.menu_identity != I_A_MENU_IDENTITY || binding.menu_order != I_A_ORDER_DIGEST {
        return Err("the catalogue is not bound to the I_a menu".into());
    }
    if binding.contract_revision != CONTRACT_REVISION {
        return Err(format!("the catalogue is bound to {}, not {CONTRACT_REVISION}", binding.contract_revision));
    }
    let source = doc.get("source").ok_or("no source")?;
    let library = text(source.get("library"), "library")?;
    if source.get("sha256").and_then(Value::as_str) != Some(LIBRARY_SHA256) || library != LIBRARY_PATH {
        return Err("the catalogue names another research library".into());
    }
    let names: Option<Vec<&str>> =
        doc.get("names").and_then(Value::as_array).map(|n| n.iter().filter_map(Value::as_str).collect());
    if names.as_deref() != Some(&NAMES[..]) {
        return Err("the coordinate names are not A5's".into());
    }
    let entries = doc.get("corrections").and_then(Value::as_array).ok_or("no corrections")?;
    if entries.len() != COUNT {
        return Err(format!("{} corrections, not {COUNT}", entries.len()));
    }
    let mut rows = Vec::with_capacity(COUNT);
    for (index, entry) in entries.iter().enumerate() {
        let id = entry.get("id").and_then(Value::as_i64).ok_or(format!("correction {index} has no id"))?;
        if id != index as i64 {
            return Err(format!("correction {index} has id {id}: ids run 0 to 35 in order"));
        }
        let family = text(entry.get("family"), "family")?;
        let values: Option<Vec<i64>> =
            entry.get("row").and_then(Value::as_array).map(|r| r.iter().filter_map(Value::as_i64).collect());
        let values = values.filter(|v| v.len() == NU_LEN).ok_or(format!("correction {id}: the row needs 45 values"))?;
        if values.iter().zip(MODULI).any(|(&v, m)| !(0..m).contains(&v)) {
            return Err(format!("correction {id}: a row value is out of its field"));
        }
        if I_A_FIXED.iter().any(|&i| values[i] != 0) {
            return Err(format!("correction {id}: a coordinate that A5 fixes is nonzero"));
        }
        let row = NuVector::from_values(&values).ok_or(format!("correction {id}: the row is not a ν vector"))?;
        let letters = entry.get("letters").and_then(Value::as_array).ok_or(format!("correction {id}: no letters"))?;
        let mut word = Vec::with_capacity(letters.len());
        for letter in letters {
            let pair = letter.as_array().filter(|p| p.len() == 2);
            let pair = pair.and_then(|p| Some((p[0].as_i64()?, p[1].as_i64()?)));
            let (grip, item) = pair.ok_or(format!("correction {id}: a letter is not [grip, item]"))?;
            let grip = u16::try_from(grip)
                .ok()
                .filter(|&g| (g as usize) < POLES)
                .ok_or(format!("correction {id}: grip {grip} is not a pole index 0..599"))?;
            let item = u8::try_from(item).map_err(|_| format!("correction {id}: item {item} is out of range"))?;
            word.push((grip, item));
        }
        if word.is_empty() {
            return Err(format!("correction {id}: the word is empty"));
        }
        rows.push(Correction { id: id as u32, family, letters: word, row });
    }
    Ok(Catalogue { binding, library, rows })
}

fn unknown(id: u32) -> Refusal {
    Refusal {
        code: codes::NAMED_OBJECT_INVALID,
        message: format!("No shipped correction has id {id}; the ids run 0 to {}", COUNT - 1),
        fact: None,
    }
}

/// I_a by its order-free identity, and with `order` also by its order digest; free mode is refused.
fn menu_binding(menu: Option<&Menu>, order: bool) -> Result<(), Refusal> {
    let refused =
        |message: &str| Err(Refusal { code: codes::MENU_BINDING_MISMATCH, message: message.into(), fact: None });
    match menu {
        None => refused("The corrections are words of the I_a menu; a state in free mode has no menu"),
        Some(menu) if menu.identity != I_A_MENU_IDENTITY => {
            refused("The corrections are words of the I_a menu; this menu is another")
        }
        Some(menu) if order && menu.order != I_A_ORDER_DIGEST => {
            refused("The letters number the I_a menu's items in its recorded order; this menu orders them otherwise")
        }
        Some(_) => Ok(()),
    }
}

impl Catalogue {
    /// The 36 corrections in fixed id order.
    pub fn rows(&self) -> &[Correction] {
        &self.rows
    }

    pub fn binding(&self) -> &Binding {
        &self.binding
    }

    /// The research library's path in the repository, as the catalogue records it.
    pub fn library(&self) -> &str {
        &self.library
    }

    /// One correction by id; an unknown id is refused with `KN2:input.named_object_invalid` (a draft).
    pub fn lookup(&self, id: u32) -> Result<&Correction, Refusal> {
        self.rows.get(id as usize).ok_or_else(|| unknown(id))
    }

    fn model(&self, ctx: &Context) -> Result<(), Refusal> {
        if ctx.model.model_id == self.binding.model_id {
            return Ok(());
        }
        Err(Refusal {
            code: codes::IDENTITY_MISMATCH,
            message: "The correction catalogue is bound to another model_id".into(),
            fact: None,
        })
    }

    /// The literal word of correction `id` as twists of `menu`, which must be I_a by identity and order digest
    /// (`KN2:input.menu_binding_mismatch`), on the catalogue's model (`KN1:input.identity_mismatch`). It neither
    /// previews nor executes anything.
    pub fn twists(&self, ctx: &Context, menu: &Menu, id: u32) -> Result<Result<Vec<Twist>, Refusal>, EngineError> {
        let checked = self.model(ctx).and_then(|()| menu_binding(Some(menu), true));
        let correction = match checked.and_then(|()| self.lookup(id)) {
            Ok(correction) => correction,
            Err(refusal) => return Ok(Err(refusal)),
        };
        // Each grip's transported menu is built once: the longest word has 5,629 letters on at most 600 grips.
        let mut caps: HashMap<u16, Vec<Twist>> = HashMap::new();
        let mut twists = Vec::with_capacity(correction.letters.len());
        for &(grip, item) in &correction.letters {
            let at_cap = match caps.entry(grip) {
                Entry::Occupied(entry) => entry.into_mut(),
                Entry::Vacant(entry) => entry.insert(
                    menu.for_cap(ctx, grip as usize)
                        .map_err(|e| EngineError(format!("correction {id}: grip {grip}: {e:?}")))?,
                ),
            };
            let twist = at_cap.get(item as usize).cloned();
            twists.push(twist.ok_or_else(|| EngineError(format!("correction {id}: menu item {item} does not exist")))?);
        }
        Ok(Ok(twists))
    }

    /// ν + Σ m·row over the drafted set, with m signed, in F2 for the bits and F5 for the C5 values. It checks the
    /// caller's set only: it solves, filters and reorders nothing.
    pub fn predict(&self, value: &NuVector, set: &[(u32, i64)]) -> Result<NuVector, Refusal> {
        let mut out = *value;
        for &(id, m) in set {
            out = out.add_times(self.lookup(id)?.row(), m);
        }
        Ok(out)
    }
}

/// KN2.4's `correction_set_check` row: ν of the base ([`nu::nu`]), then the drafted set's prediction
/// ([`Catalogue::predict`]). The base must be on the catalogue's model (`KN1:input.identity_mismatch`) under I_a by
/// the menu's order-free identity (`KN2:input.menu_binding_mismatch`, also for free mode), and every id must be
/// shipped (`KN2:input.named_object_invalid`). Off the lattice the result is undefined, as ν is; ν's own refusal
/// passes through.
pub fn correction_set_check(
    ctx: &Context,
    base: Base,
    set: &[(u32, i64)],
) -> Result<Result<Partial<NuVector>, Refusal>, EngineError> {
    let catalogue = catalogue()?;
    let state = match base {
        Base::Snapshot(snapshot) => snapshot.state(),
        Base::Derived(derived) => derived.state(),
    };
    let checked = catalogue
        .model(ctx)
        .and_then(|()| menu_binding(state.menu(), false))
        .and_then(|()| set.iter().try_for_each(|&(id, _)| catalogue.lookup(id).map(|_| ())));
    if let Err(refusal) = checked {
        return Ok(Err(refusal));
    }
    Ok(match nu::nu(ctx, base)? {
        Err(refusal) => Err(refusal),
        Ok(Partial::Total(value)) => catalogue.predict(&value, set).map(Partial::Total),
        Ok(other) => Ok(other),
    })
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::sync::Arc;

    use super::*;
    use crate::levels::Snapshot;

    fn document() -> Value {
        json::parse(DATA).expect("the shipped catalogue is JSON")
    }

    /// The value at `path`: mapping keys, or list indices as decimal text.
    fn at<'a>(doc: &'a mut Value, path: &[&str]) -> &'a mut Value {
        path.iter().fold(doc, |value, key| match value {
            Value::Object(fields) => fields.get_mut(*key).expect("a planted key exists"),
            Value::Array(items) => &mut items[key.parse::<usize>().expect("a list index")],
            other => panic!("cannot step into {other:?}"),
        })
    }

    /// The shipped document with the value at `path` replaced, read back.
    fn planted(path: &[&str], value: Value) -> Result<Catalogue, String> {
        let mut doc = document();
        *at(&mut doc, path) = value;
        read(doc.canonical().as_bytes())
    }

    fn text(s: &str) -> Value {
        Value::String(s.into())
    }

    #[test]
    fn the_shipped_file_reads() {
        let catalogue = read(DATA).expect("the shipped catalogue reads");
        assert_eq!(catalogue.rows().len(), COUNT);
        assert!(catalogue.rows().iter().enumerate().all(|(i, c)| c.id() as usize == i));
        assert_eq!(catalogue.library(), LIBRARY_PATH);
        let binding = catalogue.binding();
        assert_eq!(binding.menu_identity, I_A_MENU_IDENTITY);
        assert_eq!(binding.menu_order, I_A_ORDER_DIGEST);
        assert_eq!(binding.contract_revision, CONTRACT_REVISION);
        // The canonical rewrite of the same document reads to the same catalogue.
        let again = read(document().canonical().as_bytes()).expect("the canonical text reads");
        assert_eq!(again.rows(), catalogue.rows());
        assert_eq!(again.binding(), catalogue.binding());
    }

    #[test]
    fn planted_defects_are_engine_errors() {
        let zeros = "0".repeat(64);
        let mut short_row = document();
        let Value::Array(row) = at(&mut short_row, &["corrections", "5", "row"]) else { panic!("a row is a list") };
        row.pop();
        let short_row = at(&mut short_row, &["corrections", "5", "row"]).clone();
        let mut fewer = document();
        let Value::Array(entries) = at(&mut fewer, &["corrections"]) else { panic!("the corrections are a list") };
        entries.pop();
        let fewer = at(&mut fewer, &["corrections"]).clone();
        let cases: Vec<(&[&str], Value)> = vec![
            (&["format"], text("c600-jumble-corrections-v2")),
            (&["binding", "menu_identity"], text(&zeros)),
            (&["binding", "menu_order"], text(&zeros)),
            (&["binding", "contract_revision"], text("state-contract 2026-10-09 A1-A4")),
            (&["binding", "model_id"], Value::Int(7)),
            (&["source", "sha256"], text(&zeros)),
            (&["source", "library"], text("research/jumbling/restoration/other.json.gz")),
            (&["names", "4"], text("b35")),
            (&["corrections"], fewer),
            (&["corrections", "0", "id"], Value::Int(1)),
            (&["corrections", "5", "family"], Value::Int(7)),
            (&["corrections", "5", "row"], short_row),
            (&["corrections", "5", "row", "4"], Value::Int(2)),
            (&["corrections", "5", "row", "43"], Value::Int(5)),
            (&["corrections", "5", "row", "44"], Value::Int(-1)),
            (&["corrections", "5", "row", "0"], Value::Int(1)),
            (&["corrections", "5", "row", "35"], Value::Int(1)),
            (&["corrections", "5", "row", "37"], Value::Int(1)),
            (&["corrections", "5", "letters"], Value::Array(Vec::new())),
            (&["corrections", "5", "letters", "0"], Value::Array(vec![Value::Int(1)])),
            (&["corrections", "5", "letters", "0", "0"], Value::Int(600)),
            (&["corrections", "5", "letters", "0", "0"], Value::Int(-1)),
            (&["corrections", "5", "letters", "0", "1"], Value::Int(256)),
            (&["corrections", "5", "letters", "0", "1"], text("0")),
        ];
        for (path, value) in cases {
            assert!(planted(path, value).is_err(), "{path:?}");
        }
        // A planted value that is still in its field and off the fixed coordinates reads.
        assert!(planted(&["corrections", "5", "row", "4"], Value::Int(1)).is_ok());
        assert!(planted(&["corrections", "5", "row", "44"], Value::Int(4)).is_ok());
        assert!(planted(&["binding", "model_id"], text(&zeros)).is_ok());
    }

    fn model() -> Context {
        Context::load(&Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..").join("assets")).expect("the model")
    }

    #[test]
    fn another_model_and_menu_bindings_are_refused() {
        let ctx = model();
        let zeros = "0".repeat(64);
        let other = planted(&["binding", "model_id"], text(&zeros)).expect("another model's catalogue reads");
        assert_eq!(other.model(&ctx).unwrap_err().code, codes::IDENTITY_MISMATCH);
        let shipped = catalogue().expect("the catalogue reads");
        assert_eq!(shipped.binding().model_id, ctx.model.model_id);
        assert_eq!(shipped.model(&ctx), Ok(()));

        let fixture =
            std::fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../research/jumbling/fixtures/wj-I_a.json"))
                .expect("the I_a fixture");
        let fixture = json::parse(&fixture).expect("the fixture is JSON");
        let menu = Menu::from_record(&ctx, fixture.get("menu").expect("a menu")).expect("I_a");
        assert_eq!(other.twists(&ctx, &menu, 0).expect("no engine error").unwrap_err().code, codes::IDENTITY_MISMATCH);
        assert_eq!(menu_binding(Some(&menu), true), Ok(()));
        assert_eq!(menu_binding(Some(&menu), false), Ok(()));
        for order in [true, false] {
            assert_eq!(menu_binding(None, order).unwrap_err().code, codes::MENU_BINDING_MISMATCH);
        }

        // The shipped catalogue gives the literal word under the menu of an I_a state.
        let solved = Snapshot::solved(&ctx, Some(Arc::new(menu)));
        let word = shipped.twists(&ctx, solved.state().menu().expect("I_a"), 1).expect("no engine error");
        assert_eq!(word.expect("the word reads").len(), shipped.rows()[1].letters().len());
    }
}

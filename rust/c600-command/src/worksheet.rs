//! LB7.2's work sheets (draft; CT-LB2-03's work-sheet part). A work sheet is a workspace record that holds a saved
//! operation: its three phases (Prepare, Macro and Cleanup), its named roles and its reference, with its piece roles as
//! they were placed when it was saved and when it was last reused (solving workflow section 4).
//!
//! `worksheet.save` ([`WORKSHEET_SAVE_ROW`]) writes a sheet from a draft record. `worksheet.load`
//! ([`WORKSHEET_LOAD_ROW`]) reuses one: it places the piece roles again at the head and writes the sheet again with its
//! phases unchanged, so a piece that moved is shown, never silently retargeted (section 4.1). The write advances the
//! record's version, so a preview that read the sheet goes stale (KN0.8) and the reuse needs a fresh engine preview
//! with its fresh exact analysis; a sheet stores no analysis that could stand in for it (LB7.2). A sheet's record
//! previews as its complete operation through the draft `{"record": name}` ([`crate::RecordValue::operation`]).
//!
//! A sheet saved from a record of letters keeps that record's name, version and content digest as its macro version
//! ([`MacroVersion`]); a sheet saved from a sheet keeps the sheet's (solving workflow section 4.1: a saved work sheet
//! keeps its macro versions). `worksheet.compare` ([`WORKSHEET_COMPARE_ROW`]) compares a saved sheet with the current
//! work, a draft record ([`compare_inputs`]), and analyses the sheet's complete operation afresh at the head; the layer
//! builds its job.

use std::collections::BTreeMap;

use c600_kernel::json::{self, Value};
use c600_kernel::PIECES;

/// The row that saves a work sheet (H-08).
pub const WORKSHEET_SAVE_ROW: &str = "worksheet.save";

/// The row that reuses a saved work sheet (H-08).
pub const WORKSHEET_LOAD_ROW: &str = "worksheet.load";

/// The row that compares a saved work sheet with the current work (H-08: a query row with a job).
pub const WORKSHEET_COMPARE_ROW: &str = "worksheet.compare";

/// The names of the three phases in the stored form, in the order the complete operation runs them.
const PHASES: [&str; 3] = ["prepare", "macro", "cleanup"];

/// A piece or a position: an ID from 0 to 177,119.
fn id(value: &Value) -> Option<i64> {
    value.as_i64().filter(|&id| usize::try_from(id).is_ok_and(|id| id < PIECES))
}

/// A list of letters: integers that fit a letter, as [`crate::RecordValue::letters`] reads them.
fn letters(value: &Value) -> Option<Vec<i64>> {
    value.as_array()?.iter().map(|item| item.as_i64().filter(|&n| i32::try_from(n).is_ok())).collect()
}

/// A named role of a work sheet.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Role {
    /// A piece the operation acts on, named by its home position as KN2.4's `positions` row names it. A reuse places it
    /// again.
    Piece { name: String, piece: i64 },
    /// A position, such as the target or a buffer, which stays where it is.
    Position { name: String, position: i64 },
}

impl Role {
    pub fn name(&self) -> &str {
        match self {
            Role::Piece { name, .. } | Role::Position { name, .. } => name,
        }
    }

    pub fn to_json(&self) -> Value {
        match self {
            Role::Piece { name, piece } => {
                json::object([("name", Value::String(name.clone())), ("piece", Value::Int(*piece))])
            }
            Role::Position { name, position } => {
                json::object([("name", Value::String(name.clone())), ("position", Value::Int(*position))])
            }
        }
    }

    /// Reads [`Role::to_json`]'s form strictly: a name that is not empty and one ID from 0 to 177,119.
    pub fn from_json(value: &Value) -> Option<Role> {
        let fields = value.as_object().filter(|fields| fields.len() == 2)?;
        let name = fields.get("name")?.as_str().filter(|name| !name.is_empty())?.to_owned();
        match (fields.get("piece").and_then(id), fields.get("position").and_then(id)) {
            (Some(piece), None) => Some(Role::Piece { name, piece }),
            (None, Some(position)) => Some(Role::Position { name, position }),
            _ => None,
        }
    }
}

/// Where a piece role's piece was when the sheet was saved and where it is at the sheet's latest save or reuse: the
/// position its stickers are together in, or `None` while they are apart (KN2.4's `positions` row).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Placement {
    pub name: String,
    pub piece: i64,
    pub saved: Option<i64>,
    pub now: Option<i64>,
}

impl Placement {
    /// Whether the piece is not where it was when the sheet was saved: a mismatch that a reuse shows and never
    /// retargets.
    pub fn moved(&self) -> bool {
        self.saved != self.now
    }

    pub fn to_json(&self) -> Value {
        let at = |position: Option<i64>| position.map_or(Value::Null, Value::Int);
        json::object([
            ("name", Value::String(self.name.clone())),
            ("piece", Value::Int(self.piece)),
            ("saved", at(self.saved)),
            ("now", at(self.now)),
        ])
    }

    /// Reads [`Placement::to_json`]'s form strictly.
    pub fn from_json(value: &Value) -> Option<Placement> {
        let fields = value.as_object().filter(|fields| fields.len() == 4)?;
        let at = |key: &str| match fields.get(key)? {
            Value::Null => Some(None),
            other => id(other).map(Some),
        };
        Some(Placement {
            name: fields.get("name")?.as_str()?.to_owned(),
            piece: id(fields.get("piece")?)?,
            saved: at("saved")?,
            now: at("now")?,
        })
    }
}

/// A sheet's placements: the state digests at the save and at the latest save or reuse, and one placement for each
/// piece role, in the roles' order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Resolution {
    pub saved: String,
    pub now: String,
    pub pieces: Vec<Placement>,
}

impl Resolution {
    pub fn to_json(&self) -> Value {
        json::object([
            ("saved", Value::String(self.saved.clone())),
            ("now", Value::String(self.now.clone())),
            ("pieces", Value::Array(self.pieces.iter().map(Placement::to_json).collect())),
        ])
    }

    /// Reads [`Resolution::to_json`]'s form strictly.
    pub fn from_json(value: &Value) -> Option<Resolution> {
        let fields = value.as_object().filter(|fields| fields.len() == 3)?;
        let pieces = fields.get("pieces")?.as_array()?.iter().map(Placement::from_json).collect::<Option<_>>()?;
        Some(Resolution {
            saved: fields.get("saved")?.as_str()?.to_owned(),
            now: fields.get("now")?.as_str()?.to_owned(),
            pieces,
        })
    }
}

/// The macro a sheet's Macro phase was saved from: a workspace record of letters, its version and its content digest
/// at the save ([`crate::RecordValue::digest`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MacroVersion {
    pub record: String,
    pub version: u64,
    pub digest: String,
}

impl MacroVersion {
    pub fn to_json(&self) -> Value {
        json::object([
            ("record", Value::String(self.record.clone())),
            ("version", Value::Int(self.version as i64)),
            ("digest", Value::String(self.digest.clone())),
        ])
    }

    /// Reads [`MacroVersion::to_json`]'s form strictly.
    pub fn from_json(value: &Value) -> Option<MacroVersion> {
        let fields = value.as_object().filter(|fields| fields.len() == 3)?;
        Some(MacroVersion {
            record: fields.get("record")?.as_str()?.to_owned(),
            version: u64::try_from(fields.get("version")?.as_i64()?).ok()?,
            digest: fields.get("digest")?.as_str()?.to_owned(),
        })
    }
}

/// A work sheet: its phases, its roles, its reference, its macro version and, once saved, its placements.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Worksheet {
    /// Prepare, Macro and Cleanup, each a list of letters.
    pub phases: [Vec<i64>; 3],
    /// The named roles; each name occurs once.
    pub roles: Vec<Role>,
    /// The reference the sheet was built in, as its draft gave it; `null` for none. The layer keeps it and does not
    /// read it.
    pub reference: Value,
    /// `None` in a draft that has not been saved.
    pub resolution: Option<Resolution>,
    /// The macro its Macro phase was saved from; `None` for a sheet that names none.
    pub macro_version: Option<MacroVersion>,
}

impl Worksheet {
    /// The complete operation: Prepare, Macro and Cleanup in that order (LB2: the complete operation is what counts).
    pub fn letters(&self) -> Vec<i64> {
        self.phases.concat()
    }

    /// The piece roles' names and pieces, in the roles' order.
    pub fn pieces(&self) -> Vec<(String, i64)> {
        let piece = |role: &Role| match role {
            Role::Piece { name, piece } => Some((name.clone(), *piece)),
            Role::Position { .. } => None,
        };
        self.roles.iter().filter_map(piece).collect()
    }

    /// The stored form: `{"worksheet": {"phases": {"prepare", "macro", "cleanup"}, "roles", "reference",
    /// "resolution", "macro_version"}}`, the resolution left out while the sheet has none and the macro version while
    /// it names none.
    pub fn to_payload(&self) -> Value {
        let list = |phase: &Vec<i64>| Value::Array(phase.iter().map(|&letter| Value::Int(letter)).collect());
        let phases = PHASES.iter().zip(&self.phases).map(|(name, phase)| ((*name).to_owned(), list(phase)));
        let mut fields = BTreeMap::from([
            ("phases".to_owned(), Value::Object(phases.collect())),
            ("roles".to_owned(), Value::Array(self.roles.iter().map(Role::to_json).collect())),
            ("reference".to_owned(), self.reference.clone()),
        ]);
        if let Some(resolution) = &self.resolution {
            fields.insert("resolution".to_owned(), resolution.to_json());
        }
        if let Some(version) = &self.macro_version {
            fields.insert("macro_version".to_owned(), version.to_json());
        }
        json::object([("worksheet", Value::Object(fields))])
    }

    /// Reads [`Worksheet::to_payload`]'s form strictly: every letter fits a letter, every role names a piece or a
    /// position from 0 to 177,119 under a name of its own, and a resolution places exactly the piece roles, in their
    /// order. Any other payload is not a work sheet.
    pub fn from_payload(payload: &Value) -> Option<Worksheet> {
        let outer = payload.as_object().filter(|outer| outer.len() == 1)?;
        let fields = outer.get("worksheet")?.as_object()?;
        let resolution = match fields.get("resolution") {
            None => None,
            Some(value) => Some(Resolution::from_json(value)?),
        };
        let macro_version = match fields.get("macro_version") {
            None => None,
            Some(value) => Some(MacroVersion::from_json(value)?),
        };
        (fields.len() == 3 + usize::from(resolution.is_some()) + usize::from(macro_version.is_some())).then_some(())?;
        let phases = fields.get("phases")?.as_object().filter(|phases| phases.len() == PHASES.len())?;
        let phase = |name: &str| letters(phases.get(name)?);
        let roles: Vec<Role> = fields.get("roles")?.as_array()?.iter().map(Role::from_json).collect::<Option<_>>()?;
        let mut names: Vec<&str> = roles.iter().map(Role::name).collect();
        names.sort_unstable();
        names.windows(2).all(|pair| pair[0] != pair[1]).then_some(())?;
        let sheet = Worksheet {
            phases: [phase(PHASES[0])?, phase(PHASES[1])?, phase(PHASES[2])?],
            roles,
            reference: fields.get("reference")?.clone(),
            resolution,
            macro_version,
        };
        let placed = |r: &Resolution| {
            let placed: Vec<(String, i64)> = r.pieces.iter().map(|p| (p.name.clone(), p.piece)).collect();
            placed == sheet.pieces()
        };
        sheet.resolution.as_ref().is_none_or(placed).then_some(sheet)
    }

    /// The sheet a draft record's payload saves as: a list of letters is a Macro phase alone, with no role, no
    /// reference and no macro version (the layer names the record as the macro version); a work sheet keeps its
    /// phases, roles, reference and macro version, and its placements are made again.
    pub fn from_draft(payload: &Value) -> Option<Worksheet> {
        if let Some(sheet) = Worksheet::from_payload(payload) {
            return Some(Worksheet { resolution: None, ..sheet });
        }
        let phases = [Vec::new(), letters(payload)?, Vec::new()];
        Some(Worksheet { phases, roles: Vec::new(), reference: Value::Null, resolution: None, macro_version: None })
    }
}

/// How a saved sheet's inputs stand against the current work (`worksheet.compare`; U2-065, U5-055, U6-040; draft):
/// `{"phases": {"prepare", "macro", "cleanup"}, "roles", "reference", "macro_version"}`. Each phase, the reference and
/// the macro version is `true` when the draft's is the same. `roles` lists every role name of either sheet, in order of
/// name, as `{"name", "saved", "draft", "status"}`: the role in each sheet (`null` where it has none) and `same`,
/// `changed`, `missing` (only the saved sheet has it) or `added` (only the draft has it). A comparison is a report: it
/// binds, selects and retargets nothing (solving workflow section 4.1).
pub fn compare_inputs(saved: &Worksheet, draft: &Worksheet) -> Value {
    let phases = PHASES.iter().zip(saved.phases.iter().zip(&draft.phases));
    let phases = phases.map(|(name, (saved, draft))| ((*name).to_owned(), Value::Bool(saved == draft)));
    let by_name = |sheet: &Worksheet| -> BTreeMap<String, Role> {
        sheet.roles.iter().map(|role| (role.name().to_owned(), role.clone())).collect()
    };
    let (old, new) = (by_name(saved), by_name(draft));
    let mut names: Vec<&String> = old.keys().chain(new.keys()).collect();
    names.sort_unstable();
    names.dedup();
    let roles = names.into_iter().map(|name| {
        let (before, after) = (old.get(name), new.get(name));
        let status = match (before, after) {
            (Some(before), Some(after)) if before == after => "same",
            (Some(_), Some(_)) => "changed",
            (Some(_), None) => "missing",
            _ => "added",
        };
        json::object([
            ("name", Value::String(name.clone())),
            ("saved", before.map_or(Value::Null, Role::to_json)),
            ("draft", after.map_or(Value::Null, Role::to_json)),
            ("status", Value::String(status.to_owned())),
        ])
    });
    json::object([
        ("phases", Value::Object(phases.collect())),
        ("roles", Value::Array(roles.collect())),
        ("reference", Value::Bool(saved.reference == draft.reference)),
        ("macro_version", Value::Bool(saved.macro_version == draft.macro_version)),
    ])
}

/// H-08's `WorksheetRevisionRef` in its draft form, `{"record": name, "version": n}`: a sheet's record and the record
/// version at which the solver chose it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorksheetRef {
    pub record: String,
    pub version: u64,
}

impl WorksheetRef {
    pub fn to_json(&self) -> Value {
        json::object([("record", Value::String(self.record.clone())), ("version", Value::Int(self.version as i64))])
    }

    /// Reads [`WorksheetRef::to_json`]'s form strictly: a record name that is text and a version that is a natural.
    pub fn from_json(value: &Value) -> Option<WorksheetRef> {
        let fields = value.as_object().filter(|fields| fields.len() == 2)?;
        Some(WorksheetRef {
            record: fields.get("record")?.as_str()?.to_owned(),
            version: u64::try_from(fields.get("version")?.as_i64()?).ok()?,
        })
    }
}

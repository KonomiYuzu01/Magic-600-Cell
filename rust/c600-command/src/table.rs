//! H-08's rows and the language's overlay rows (ADR-L02; KN5.3).
//!
//! The stored table is `docs/progress/1.0/command-table.json`, compiled in. It is frozen at LP3 (PHS.53), and
//! this module reads only the fields the command layer uses and ignores the rest. A language-defined command is an
//! overlay row of kind view, workspace, query or file, never commit or app. A view overlay runs on the `view` lane and
//! every other overlay on the `program` lane. A stored row that a built-in tool's entry serves ([`Table::serve_row`])
//! keeps H-08's lane. A language-defined row's stored [`Metadata`] is what its availability reads (KN5.7, LI.20).

use std::collections::BTreeMap;

use c600_kernel::json::{self, Value};

use crate::receipt::{code, Error};

/// The stored H-08 table.
pub const H08: &str = include_str!("../../../docs/progress/1.0/command-table.json");

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    App,
    Commit,
    File,
    Query,
    View,
    Workspace,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lane {
    Commit,
    Exclusive,
    LatestWins,
    None,
    Program,
    View,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UndoClass {
    Journal,
    None,
    Workspace,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PreviewRule {
    None,
    Required,
    Filter,
}

/// The module entry a language-defined row runs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Overlay {
    pub module: String,
    pub entry: String,
}

/// One row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Row {
    pub id: String,
    pub kind: Kind,
    pub lane: Lane,
    pub undo: UndoClass,
    pub preview: PreviewRule,
    pub gates: Vec<String>,
    pub contexts: Vec<String>,
    pub repeat: bool,
    pub job: bool,
    pub key_0_4: Option<String>,
    /// Argument names and their H-08 types.
    pub arguments: BTreeMap<String, String>,
    pub action_guard: Option<String>,
    /// Set for a language-defined row, and for a stored row that a built-in tool's entry serves
    /// ([`Table::serve_row`]).
    pub overlay: Option<Overlay>,
}

impl Row {
    pub fn has_gate(&self, gate: &str) -> bool {
        self.gates.iter().any(|g| g == gate)
    }

    /// Whether the row takes a user-action token.
    pub fn takes_token(&self) -> bool {
        self.arguments.values().any(|t| t == "UserActionToken")
    }

    /// The argument names other than the token, which travels beside the arguments in the envelope.
    pub fn plain_arguments(&self) -> impl Iterator<Item = &str> {
        self.arguments.iter().filter(|(_, t)| *t != "UserActionToken").map(|(name, _)| name.as_str())
    }
}

/// How a language-defined row binds the arguments of a kernel row it may invoke (LI.20).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Binding {
    /// Arguments the module's check captured: the row's availability for exactly these arguments
    /// ([`crate::CommandLayer::availability_for`]).
    Static(Value),
    /// Arguments the program computes: the row's argument-independent gates only
    /// ([`crate::CommandLayer::availability`]). Gates that need the arguments wait for the invocation.
    Computed,
}

/// A kernel row that a language-defined row's program may invoke (LI.20).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Dependency {
    pub row: String,
    pub binding: Binding,
    /// An optional row takes no part in availability.
    pub optional: bool,
}

/// A language-defined row's stored metadata (draft; KN5.5, KN5.7, LI.20): data that availability reads with no
/// language code running. The default is a passing static verdict and no kernel row.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Metadata {
    /// The first failed diagnostic of the module's static check; `None` when the static verdict passes.
    pub diagnostic: Option<Error>,
    /// The kernel rows the program may invoke, in stored order.
    pub requires: Vec<Dependency>,
}

/// The rows by ID.
#[derive(Clone, Debug)]
pub struct Table {
    rows: BTreeMap<String, Row>,
    services: Vec<String>,
    /// The language-defined rows' metadata: a key for each row [`Table::register_overlay`] registered.
    metadata: BTreeMap<String, Metadata>,
}

fn word<T: Copy>(value: Option<&Value>, names: &[(&str, T)]) -> Result<T, String> {
    let text = value.and_then(Value::as_str).ok_or("a vocabulary field is not text")?;
    names.iter().find(|(name, _)| *name == text).map(|&(_, v)| v).ok_or_else(|| format!("unknown value `{text}`"))
}

fn texts(value: Option<&Value>) -> Result<Vec<String>, String> {
    match value {
        None => Ok(Vec::new()),
        Some(value) => value
            .as_array()
            .ok_or("a list field is not a list")?
            .iter()
            .map(|v| v.as_str().map(str::to_owned).ok_or_else(|| "a list item is not text".to_owned()))
            .collect(),
    }
}

fn row(value: &Value) -> Result<Row, String> {
    let id = value.get("id").and_then(Value::as_str).ok_or("a row has no id")?.to_owned();
    let fail = |e: String| format!("row `{id}`: {e}");
    let kinds = [
        ("app", Kind::App),
        ("commit", Kind::Commit),
        ("file", Kind::File),
        ("query", Kind::Query),
        ("view", Kind::View),
        ("workspace", Kind::Workspace),
    ];
    let lanes = [
        ("commit", Lane::Commit),
        ("exclusive", Lane::Exclusive),
        ("latest_wins", Lane::LatestWins),
        ("none", Lane::None),
        ("program", Lane::Program),
        ("view", Lane::View),
    ];
    let undos = [("journal", UndoClass::Journal), ("none", UndoClass::None), ("workspace", UndoClass::Workspace)];
    let previews = [("none", PreviewRule::None), ("required", PreviewRule::Required), ("filter", PreviewRule::Filter)];
    let mut arguments = BTreeMap::new();
    if let Some(args) = value.get("arguments") {
        for (name, t) in args.as_object().ok_or_else(|| fail("arguments is not an object".into()))? {
            let t = t.as_str().ok_or_else(|| fail(format!("argument `{name}` has no type")))?;
            arguments.insert(name.clone(), t.to_owned());
        }
    }
    let flag = |name: &str| value.get(name).is_some_and(|v| matches!(v, Value::Bool(true)));
    Ok(Row {
        kind: word(value.get("kind"), &kinds).map_err(fail)?,
        lane: word(value.get("lane"), &lanes).map_err(fail)?,
        undo: word(value.get("undo"), &undos).map_err(fail)?,
        preview: word(value.get("preview"), &previews).map_err(fail)?,
        gates: texts(value.get("gates")).map_err(fail)?,
        contexts: texts(value.get("contexts")).map_err(fail)?,
        repeat: flag("repeat"),
        job: flag("job"),
        key_0_4: value.get("key_0_4").and_then(Value::as_str).map(str::to_owned),
        arguments,
        action_guard: value.get("action_guard").and_then(Value::as_str).map(str::to_owned),
        overlay: None,
        id,
    })
}

impl Table {
    /// The stored H-08 table.
    pub fn h08() -> Table {
        Table::parse(H08.as_bytes()).expect("the stored H-08 table reads")
    }

    pub fn parse(bytes: &[u8]) -> Result<Table, String> {
        let document = json::parse(bytes).map_err(|e| e.to_string())?;
        let commands = document.get("commands").and_then(Value::as_array).ok_or("the table has no command list")?;
        let mut rows = BTreeMap::new();
        for value in commands {
            let row = row(value)?;
            if rows.insert(row.id.clone(), row).is_some() {
                return Err("the table repeats a row id".into());
            }
        }
        let services = match document.get("services").and_then(Value::as_array) {
            None => Vec::new(),
            Some(list) => list.iter().filter_map(|s| s.get("id").and_then(Value::as_str).map(str::to_owned)).collect(),
        };
        Ok(Table { rows, services, metadata: BTreeMap::new() })
    }

    pub fn row(&self, id: &str) -> Option<&Row> {
        self.rows.get(id)
    }

    pub fn rows(&self) -> impl Iterator<Item = &Row> {
        self.rows.values()
    }

    /// The `kernel.*` and `language.*` services, which are not rows (LI.10).
    pub fn services(&self) -> &[String] {
        &self.services
    }

    /// The table with the command layer's draft rows that it does not hold: `workspace.set`
    /// ([`crate::workspace::RECORD_ROW`]), M6's record write, a workspace row whose undo is a workspace inverse. A row
    /// the table already holds is kept as it is; H-08 holds `workspace.set` in this shape since the command table's
    /// freeze (CM.17).
    pub fn with_drafts(mut self) -> Table {
        let id = crate::workspace::RECORD_ROW;
        let arguments = [("payload", "Json"), ("provenance", "ProvenanceBlock"), ("record", "Text")];
        self.rows.entry(id.to_owned()).or_insert(Row {
            id: id.to_owned(),
            kind: Kind::Workspace,
            lane: Lane::None,
            undo: UndoClass::Workspace,
            preview: PreviewRule::None,
            gates: Vec::new(),
            contexts: vec!["any".to_owned()],
            repeat: false,
            job: false,
            key_0_4: None,
            arguments: arguments.iter().map(|(n, t)| ((*n).to_owned(), (*t).to_owned())).collect(),
            action_guard: None,
            overlay: None,
        });
        self
    }

    /// Registers a language-defined row. Its lane follows its kind, it takes no token, it has no gates and it is
    /// available in every context. Its metadata starts as the default until [`Table::declare`] stores it. A commit or
    /// app row is refused with `KN5:input.overlay_row_kind`; an ID that a row or service already holds is refused with
    /// `KN5:input.overlay_row_conflict`.
    pub fn register_overlay(
        &mut self,
        id: &str,
        kind: Kind,
        arguments: &[(&str, &str)],
        overlay: Overlay,
    ) -> Result<&Row, Error> {
        if matches!(kind, Kind::Commit | Kind::App) {
            return Err(Error::new(code::OVERLAY_ROW_KIND, format!("overlay row `{id}` may not be a {kind:?} row")));
        }
        if self.rows.contains_key(id) || self.services.iter().any(|s| s == id) {
            return Err(Error::new(code::OVERLAY_ROW_CONFLICT, format!("`{id}` is already a row or service")));
        }
        if arguments.iter().any(|(_, t)| *t == "UserActionToken") {
            return Err(Error::new(code::OVERLAY_ROW_KIND, "an overlay row never takes a user-action token (KN5.15)"));
        }
        let row = Row {
            id: id.to_owned(),
            kind,
            lane: if kind == Kind::View { Lane::View } else { Lane::Program },
            undo: if kind == Kind::Workspace { UndoClass::Workspace } else { UndoClass::None },
            preview: PreviewRule::None,
            gates: Vec::new(),
            contexts: vec!["any".to_owned()],
            repeat: false,
            job: true,
            key_0_4: None,
            arguments: arguments.iter().map(|(n, t)| ((*n).to_owned(), (*t).to_owned())).collect(),
            action_guard: None,
            overlay: Some(overlay),
        };
        self.metadata.insert(id.to_owned(), Metadata::default());
        Ok(self.rows.entry(id.to_owned()).or_insert(row))
    }

    /// Whether `id` is a language-defined row: one that [`Table::register_overlay`] registered. A stored row that an
    /// entry serves is not one.
    pub fn language_defined(&self, id: &str) -> bool {
        self.metadata.contains_key(id)
    }

    /// A language-defined row's stored metadata; `None` for any other row.
    pub fn metadata(&self, id: &str) -> Option<&Metadata> {
        self.metadata.get(id)
    }

    /// Stores language-defined row `id`'s metadata, replacing what it held (draft; KN5.5, LI.20): the host declares
    /// it from the module's static check and the kernel rows the program may invoke. Refused: an ID that names no row,
    /// with `KN5:input.unknown_row`, and a stored row, with `KN5:input.overlay_row_conflict`. A refusal
    /// leaves the table as it was.
    pub fn declare(&mut self, id: &str, metadata: Metadata) -> Result<(), Error> {
        if !self.rows.contains_key(id) {
            return Err(Error::new(code::UNKNOWN_ROW, format!("no row `{id}`")));
        }
        let Some(stored) = self.metadata.get_mut(id) else {
            return Err(Error::new(
                code::OVERLAY_ROW_CONFLICT,
                format!("`{id}` is a stored row, not a language-defined one"),
            ));
        };
        *stored = metadata;
        Ok(())
    }

    /// Serves a stored row with a built-in tool's entry (draft; LP3's C6). An invocation of the row admits a program
    /// job that runs `overlay`'s entry, as for a language-defined row. The row keeps H-08's kind, lane, undo class,
    /// gates and argument types.
    ///
    /// Only a row whose admission and completion the layer can check without the entry is served: a view, workspace,
    /// query or file row on the `none`, `view` or `program` lane, with no action guard or token. Its undo class is
    /// `none`, or `workspace` on a workspace row, whose completion the layer inverts as a workspace inverse. It has no
    /// gate and no preview rule, except a workspace row with the undo class `workspace`, the gate `fresh_filter_preview`
    /// and the `filter` preview rule together (LI.41: `filter.apply`'s shape), whose gate the layer checks against the
    /// filter previews it keeps ([`crate::filter`]).
    ///
    /// Refused:
    /// - an ID that names no row, with `KN5:input.unknown_row`;
    /// - a row of any other shape, with `KN5:input.overlay_row_kind`;
    /// - a row the layer builds itself, or one an entry already serves, with
    ///   `KN5:input.overlay_row_conflict`.
    ///
    /// A refusal leaves the row as it was.
    pub fn serve_row(&mut self, id: &str, overlay: Overlay) -> Result<&Row, Error> {
        let Some(row) = self.rows.get_mut(id) else {
            return Err(Error::new(code::UNKNOWN_ROW, format!("no row `{id}`")));
        };
        if LAYER_ROWS.contains(&id) {
            return Err(Error::new(code::OVERLAY_ROW_CONFLICT, format!("the command layer builds `{id}` itself")));
        }
        if row.overlay.is_some() {
            return Err(Error::new(code::OVERLAY_ROW_CONFLICT, format!("an entry already serves `{id}`")));
        }
        let inverse = row.kind == Kind::Workspace && row.undo == UndoClass::Workspace;
        let filter = row.gates == [crate::filter::FRESH_FILTER_PREVIEW] && row.preview == PreviewRule::Filter;
        let checkable = !matches!(row.kind, Kind::Commit | Kind::App)
            && matches!(row.lane, Lane::None | Lane::View | Lane::Program)
            && (row.undo == UndoClass::None || inverse)
            && ((row.preview == PreviewRule::None && row.gates.is_empty()) || (inverse && filter))
            && row.action_guard.is_none()
            && !row.takes_token();
        if !checkable {
            return Err(Error::new(code::OVERLAY_ROW_KIND, format!("no entry can serve `{id}` in this layer")));
        }
        row.overlay = Some(overlay);
        Ok(row)
    }
}

/// The stored rows the command layer builds itself; no entry serves them.
const LAYER_ROWS: [&str; 11] = [
    "operation.preview",
    "operation.execute",
    "operation.preview.cancel",
    "session.undo",
    "session.redo",
    "protection.orbit.set",
    crate::layer::POLICY_ROW,
    crate::workspace::RECORD_ROW,
    crate::worksheet::WORKSHEET_SAVE_ROW,
    crate::worksheet::WORKSHEET_LOAD_ROW,
    crate::worksheet::WORKSHEET_COMPARE_ROW,
];

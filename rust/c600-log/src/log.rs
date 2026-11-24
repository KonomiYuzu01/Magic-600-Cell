//! What a log holds (ADR-D01.80, revision 6's R-3) and its record: an FP.11 record of format `c600-log-draft`,
//! version 1, with null provenance, outside the data core like FP.37's playback state.
//!
//! The payload is `{log, seal}`. `seal` is the `DigestRef(content)` of the FP.5 bytes of the record with `payload`
//! reduced to `{log}`, so a changed byte anywhere in the record that still reads as canonical JSON fails the seal
//! with `PS1:input.digest_mismatch`.
//!
//! Both histories are append-only lists of actions (ADR-D01.14 to .16). A node action (a commit, a partial undo, a
//! workspace action, a layout step) adds a node whose `parent` is where the cursor stood; a cursor action (undo, redo,
//! roll, select) moves the cursor to a node or to the start (`to: null`). Nothing is rewritten: an abandoned branch is
//! the nodes no later cursor returns to, and a partial undo is its own node, naming the commit and how many of its
//! literal letters stay. Its parent is the commit's parent, since its state is that commit's state with a shorter
//! word, and the cursor must stand on the commit or on another partial undo of it.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use c600_kernel::json::Value;
use c600_kernel::{IdentityBlock, Puzzle};

use crate::problem::{code, Gate, Problem};
use crate::wire::{self, digest_ref, labels_value, nat, object, text_or_null, Fields};

/// The record's `format`.
pub const FORMAT: &str = "c600-log-draft";
/// The record's `version`.
pub const VERSION: i64 = 1;
/// The file extension of a 1.0 log, without its dot: `name.c600log`. The bytes are FP.7's (raw FP.5 JSON, or stored
/// gzip above 250,000 bytes), so the extension names the format, not a compression.
pub const EXTENSION: &str = "c600log";
/// The origin of a log imported from a 0.4 proof log: chapter 05's retained session exchange format.
pub(crate) const C600_ORIGIN: &str = "C600-STUDIO-SESSION-v1";
/// The origin of a log imported from an MPUlt log, as 0.4 names it in `native_log.format`.
pub(crate) const MPULT_ORIGIN: &str = "MPUltimate-v1";

/// A whole log.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Log {
    pub identity: IdentityBlock,
    pub start: Start,
    pub moves: Vec<MoveEntry>,
    pub layout_history: Vec<LayoutEntry>,
    pub saved: Saved,
    /// The view, the layout and the workspace at save: recorded values that LP4-14 and LP5 interpret.
    pub view: Value,
    pub layout: Value,
    pub workspace: Value,
    pub documents: Vec<OpenDocument>,
    pub macros: Vec<Macro>,
    pub library: Vec<LibraryEntry>,
    pub scripts: Vec<Script>,
    /// The preferences at save as canonical JSON text, recorded for reference and never applied (ADR-D01.80).
    pub preferences: Option<String>,
    pub origin: Origin,
    /// Optional extensions this reader does not know, kept as read (FP.10).
    pub extensions: Vec<Extension>,
}

impl Log {
    /// An empty log of a model: solved start, no history, the saved state the start's.
    pub fn new(identity: IdentityBlock, start_digest: &str) -> Log {
        Log {
            identity,
            start: Start::Solved,
            moves: Vec::new(),
            layout_history: Vec::new(),
            saved: Saved { moves: None, layout: None, state: start_digest.to_owned() },
            view: Value::Null,
            layout: Value::Null,
            workspace: Value::Null,
            documents: Vec::new(),
            macros: Vec::new(),
            library: Vec::new(),
            scripts: Vec::new(),
            preferences: None,
            origin: Origin::Live,
            extensions: Vec::new(),
        }
    }

    /// The node of an id: a commit, a partial undo or a workspace action.
    pub fn node(&self, id: &str) -> Option<&MoveEntry> {
        self.moves.iter().find(|entry| entry.id == id && entry.action.is_node())
    }

    /// The nodes from the start to `head`, first to last.
    pub fn line(&self, head: Option<&str>) -> Vec<&MoveEntry> {
        let nodes: HashMap<&str, &MoveEntry> =
            self.moves.iter().filter(|e| e.action.is_node()).map(|e| (e.id.as_str(), e)).collect();
        let mut line = Vec::new();
        let mut at = head;
        while let Some(entry) = at.and_then(|id| nodes.get(id)) {
            line.push(*entry);
            at = entry.action.parent().flatten();
        }
        line.reverse();
        line
    }
}

/// Where the move history starts.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Start {
    Solved,
    /// A state's labels, carried as PS1.7 `u32le-base64`.
    Labels(Vec<i32>),
}

/// One action of the move history.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MoveEntry {
    pub id: String,
    /// When it happened, as the host wrote it; null when unknown (an import).
    pub time: Option<String>,
    pub action: MoveAction,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MoveAction {
    Commit(Commit),
    PartialUndo(PartialUndo),
    Workspace(WorkspaceStep),
    Cursor(Cursor),
}

impl MoveAction {
    pub fn kind(&self) -> &'static str {
        match self {
            MoveAction::Commit(_) => "commit",
            MoveAction::PartialUndo(_) => "partial_undo",
            MoveAction::Workspace(_) => "workspace",
            MoveAction::Cursor(_) => "cursor",
        }
    }

    pub fn is_node(&self) -> bool {
        !matches!(self, MoveAction::Cursor(_))
    }

    /// A node's parent: `Some(None)` for a node at the start, `None` for a cursor action.
    pub fn parent(&self) -> Option<Option<&str>> {
        match self {
            MoveAction::Commit(c) => Some(c.parent.as_deref()),
            MoveAction::PartialUndo(p) => Some(p.parent.as_deref()),
            MoveAction::Workspace(w) => Some(w.parent.as_deref()),
            MoveAction::Cursor(_) => None,
        }
    }
}

/// A committed transaction: one hand, macro or script commit (ADR-D01.14), with its legal witness.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Commit {
    pub parent: Option<String>,
    /// How it was made: `hand`, `macro`, `script` or `import`.
    pub origin: String,
    pub name: String,
    pub note: String,
    pub assistance: String,
    /// The legal witness: PS1.22 word and star steps with small JSON integers, as 0.4 normalizes them.
    pub recipe: Value,
    pub primitive_count: u64,
    pub stars: u64,
    /// The full-state hashes before and after (PS1.6).
    pub pre: String,
    pub post: String,
    pub interchange: Interchange,
}

/// Where a commit came from in an interchange file, so that export writes it back the same way.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Interchange {
    None,
    /// A 0.4 transaction; `id` is the canonical JSON text of its `id` field, when it had one.
    C600 {
        id: Option<String>,
    },
    /// The MPUlt sequence entries that produced the transaction: its move token, after any macro markers and moves
    /// of no letter that came before it.
    MPUlt {
        tokens: Vec<String>,
    },
}

/// Stepping back inside a commit: the state is the commit's pre state with the first `keep` letters of its literal
/// word (ADR-D01.15).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PartialUndo {
    pub parent: Option<String>,
    pub entry: String,
    pub keep: u64,
    pub post: String,
}

/// A puzzle-related workspace action of the move history (a macro, an operation draft, a record); the state stays.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkspaceStep {
    pub parent: Option<String>,
    pub what: String,
    pub value: Value,
}

/// A cursor move: one solver action that may cover several steps.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cursor {
    pub action: CursorAction,
    /// The node the cursor moves to; `None` is the start.
    pub to: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CursorAction {
    /// To a node before the cursor on its line.
    Undo,
    /// To a node after the cursor.
    Redo,
    /// "Roll to here": to a node before or after the cursor on one line.
    Roll,
    /// `session.branch.select`: to any node.
    Select,
}

impl CursorAction {
    pub const ALL: [CursorAction; 4] =
        [CursorAction::Undo, CursorAction::Redo, CursorAction::Roll, CursorAction::Select];

    pub fn name(self) -> &'static str {
        match self {
            CursorAction::Undo => "undo",
            CursorAction::Redo => "redo",
            CursorAction::Roll => "roll",
            CursorAction::Select => "select",
        }
    }
}

/// One action of the layout history.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LayoutEntry {
    pub id: String,
    pub time: Option<String>,
    pub action: LayoutAction,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LayoutAction {
    Step(LayoutStep),
    Cursor(Cursor),
}

/// A camera, filter, layout, arrangement or design-profile change, by hand or by a script.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LayoutStep {
    pub parent: Option<String>,
    pub what: String,
    pub origin: String,
    pub value: Value,
}

/// Where both cursors stood at save, and the state there.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Saved {
    pub moves: Option<String>,
    pub layout: Option<String>,
    /// The full-state hash at save.
    pub state: String,
}

/// An open document at save (revision 6, R-3).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OpenDocument {
    pub identity: DocumentIdentity,
    /// `script` or `log`.
    pub kind: String,
    /// The digest of the last saved bytes; `None` for an untitled document never saved.
    pub saved: Option<String>,
    pub dirty: bool,
    /// The full contents at save; for a clean document they are the saved bytes.
    pub contents: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DocumentIdentity {
    Path(String),
    Untitled(String),
}

/// The provenance mark of an embedded entry: chapter 07's author class (BND.18), or `unknown`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Author {
    BuiltIn,
    Own,
    ForkedBuiltIn,
    Imported,
    Unknown,
}

impl Author {
    pub const ALL: [Author; 5] =
        [Author::BuiltIn, Author::Own, Author::ForkedBuiltIn, Author::Imported, Author::Unknown];

    pub fn name(self) -> &'static str {
        match self {
            Author::BuiltIn => "built_in",
            Author::Own => "own",
            Author::ForkedBuiltIn => "forked_built_in",
            Author::Imported => "imported",
            Author::Unknown => "unknown",
        }
    }
}

/// An embedded macro with its witness (FP.12's recipe fields).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Macro {
    pub id: String,
    pub name: String,
    pub author: Author,
    pub steps: Value,
    pub primitive_count: u64,
    /// `DigestRef(recipe_expansion)`: signed int32-LE letters (FP.8).
    pub expansion: String,
}

/// An embedded Library entry, recorded as its value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LibraryEntry {
    pub id: String,
    pub name: String,
    pub author: Author,
    pub kind: String,
    pub value: Value,
}

/// An embedded script: its source text and that text's `DigestRef(source)`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Script {
    pub name: String,
    pub author: Author,
    pub source: String,
    pub text: String,
}

impl Script {
    pub fn new(name: &str, author: Author, text: &str) -> Script {
        Script {
            name: name.to_owned(),
            author,
            source: c600_digest::sha256_hex(text.as_bytes()),
            text: text.to_owned(),
        }
    }
}

/// Where the log came from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Origin {
    /// Written by 1.0.
    Live,
    /// Imported from a 0.4 `C600-STUDIO-SESSION-v1` file of these bytes.
    C600 {
        source: String,
    },
    MPUlt(MpultOrigin),
}

/// An imported MPUlt v1 log: its bytes and the counters 0.4 keeps (`native_log`), so that export writes them back.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MpultOrigin {
    pub source: String,
    pub timer: u64,
    pub shuffle: u64,
    pub pointer: u64,
    /// Sequence entries after the last transaction: macro markers and moves of no letter.
    pub trailing: Vec<String>,
    /// How many transactions the import made.
    pub transactions: u64,
}

/// An extension this reader does not know (FP.10).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Extension {
    pub name: String,
    pub required: bool,
    pub value: Value,
}

// ---------------------------------------------------------------------------------------------------------------
// Writing

/// The KN0.1 identity block as FP.11 writes it.
pub fn identity_value(identity: &IdentityBlock) -> Value {
    object(vec![
        ("contract_rev", text_or_null(identity.contract_rev.as_deref())),
        ("menu_id", text_or_null(identity.menu_id.as_deref())),
        ("menu_order", text_or_null(identity.menu_order.as_deref())),
        ("model_id", Value::String(identity.model_id.clone())),
        (
            "puzzle",
            Value::String(
                match identity.puzzle {
                    Puzzle::Full => "full",
                    Puzzle::Jumbling => "jumbling",
                }
                .to_owned(),
            ),
        ),
    ])
}

fn labels(sha256: &str) -> Value {
    digest_ref("labels", sha256)
}

fn parent(parent: &Option<String>) -> Value {
    text_or_null(parent.as_deref())
}

fn cursor_fields(cursor: &Cursor) -> Vec<(&'static str, Value)> {
    vec![("action", Value::String(cursor.action.name().to_owned())), ("to", text_or_null(cursor.to.as_deref()))]
}

fn move_value(entry: &MoveEntry) -> Value {
    let mut fields = vec![
        ("id", Value::String(entry.id.clone())),
        ("kind", Value::String(entry.action.kind().to_owned())),
        ("time", text_or_null(entry.time.as_deref())),
    ];
    match &entry.action {
        MoveAction::Commit(c) => fields.extend([
            ("parent", parent(&c.parent)),
            ("origin", Value::String(c.origin.clone())),
            ("name", Value::String(c.name.clone())),
            ("note", Value::String(c.note.clone())),
            ("assistance", Value::String(c.assistance.clone())),
            ("recipe", c.recipe.clone()),
            ("primitive_count", nat(c.primitive_count)),
            ("stars", nat(c.stars)),
            ("pre", labels(&c.pre)),
            ("post", labels(&c.post)),
            ("interchange", interchange_value(&c.interchange)),
        ]),
        MoveAction::PartialUndo(p) => fields.extend([
            ("parent", parent(&p.parent)),
            ("entry", Value::String(p.entry.clone())),
            ("keep", nat(p.keep)),
            ("post", labels(&p.post)),
        ]),
        MoveAction::Workspace(w) => fields.extend([
            ("parent", parent(&w.parent)),
            ("what", Value::String(w.what.clone())),
            ("value", w.value.clone()),
        ]),
        MoveAction::Cursor(cursor) => fields.extend(cursor_fields(cursor)),
    }
    object(fields)
}

fn interchange_value(interchange: &Interchange) -> Value {
    match interchange {
        Interchange::None => Value::Null,
        Interchange::C600 { id } => {
            object(vec![("kind", Value::String("c600".to_owned())), ("id", text_or_null(id.as_deref()))])
        }
        Interchange::MPUlt { tokens } => object(vec![
            ("kind", Value::String("mpult".to_owned())),
            ("tokens", Value::Array(tokens.iter().cloned().map(Value::String).collect())),
        ]),
    }
}

fn layout_value(entry: &LayoutEntry) -> Value {
    let mut fields = vec![("id", Value::String(entry.id.clone())), ("time", text_or_null(entry.time.as_deref()))];
    match &entry.action {
        LayoutAction::Step(step) => fields.extend([
            ("kind", Value::String("step".to_owned())),
            ("parent", parent(&step.parent)),
            ("what", Value::String(step.what.clone())),
            ("origin", Value::String(step.origin.clone())),
            ("value", step.value.clone()),
        ]),
        LayoutAction::Cursor(cursor) => {
            fields.push(("kind", Value::String("cursor".to_owned())));
            fields.extend(cursor_fields(cursor));
        }
    }
    object(fields)
}

fn document_value(document: &OpenDocument) -> Value {
    let identity = match &document.identity {
        DocumentIdentity::Path(path) => {
            object(vec![("kind", Value::String("path".to_owned())), ("path", Value::String(path.clone()))])
        }
        DocumentIdentity::Untitled(id) => {
            object(vec![("kind", Value::String("untitled".to_owned())), ("id", Value::String(id.clone()))])
        }
    };
    let contents = match std::str::from_utf8(&document.contents) {
        Ok(text) => {
            object(vec![("encoding", Value::String("utf8".to_owned())), ("text", Value::String(text.to_owned()))])
        }
        Err(_) => object(vec![
            ("encoding", Value::String("base64".to_owned())),
            ("data", Value::String(wire::base64(&document.contents))),
        ]),
    };
    object(vec![
        ("identity", identity),
        ("kind", Value::String(document.kind.clone())),
        ("saved", document.saved.as_deref().map_or(Value::Null, |sha| digest_ref("bytes", sha))),
        ("dirty", Value::Bool(document.dirty)),
        ("contents", contents),
    ])
}

fn author(author: Author) -> Value {
    Value::String(author.name().to_owned())
}

fn origin_value(origin: &Origin) -> Value {
    match origin {
        Origin::Live => Value::Null,
        Origin::C600 { source } => object(vec![
            ("kind", Value::String("c600".to_owned())),
            ("format", Value::String(C600_ORIGIN.to_owned())),
            ("source", digest_ref("bytes", source)),
        ]),
        Origin::MPUlt(m) => object(vec![
            ("kind", Value::String("mpult".to_owned())),
            ("format", Value::String(MPULT_ORIGIN.to_owned())),
            ("source", digest_ref("bytes", &m.source)),
            ("timer", nat(m.timer)),
            ("shuffle", nat(m.shuffle)),
            ("pointer", nat(m.pointer)),
            ("trailing", Value::Array(m.trailing.iter().cloned().map(Value::String).collect())),
            ("transactions", nat(m.transactions)),
        ]),
    }
}

fn log_value(log: &Log) -> Value {
    let start = match &log.start {
        Start::Solved => object(vec![("kind", Value::String("solved".to_owned()))]),
        Start::Labels(l) => object(vec![("kind", Value::String("labels".to_owned())), ("labels", labels_value(l))]),
    };
    object(vec![
        ("start", start),
        (
            "history",
            object(vec![
                ("moves", Value::Array(log.moves.iter().map(move_value).collect())),
                ("layout", Value::Array(log.layout_history.iter().map(layout_value).collect())),
            ]),
        ),
        (
            "saved",
            object(vec![
                ("moves", text_or_null(log.saved.moves.as_deref())),
                ("layout", text_or_null(log.saved.layout.as_deref())),
                ("state", labels(&log.saved.state)),
            ]),
        ),
        ("view", log.view.clone()),
        ("layout", log.layout.clone()),
        ("workspace", log.workspace.clone()),
        ("documents", Value::Array(log.documents.iter().map(document_value).collect())),
        (
            "macros",
            Value::Array(
                log.macros
                    .iter()
                    .map(|m| {
                        object(vec![
                            ("id", Value::String(m.id.clone())),
                            ("name", Value::String(m.name.clone())),
                            ("author", author(m.author)),
                            ("steps", m.steps.clone()),
                            ("primitive_count", nat(m.primitive_count)),
                            ("expansion", digest_ref("recipe_expansion", &m.expansion)),
                        ])
                    })
                    .collect(),
            ),
        ),
        (
            "library",
            Value::Array(
                log.library
                    .iter()
                    .map(|e| {
                        object(vec![
                            ("id", Value::String(e.id.clone())),
                            ("name", Value::String(e.name.clone())),
                            ("author", author(e.author)),
                            ("kind", Value::String(e.kind.clone())),
                            ("value", e.value.clone()),
                        ])
                    })
                    .collect(),
            ),
        ),
        (
            "scripts",
            Value::Array(
                log.scripts
                    .iter()
                    .map(|s| {
                        object(vec![
                            ("name", Value::String(s.name.clone())),
                            ("author", author(s.author)),
                            ("source", digest_ref("source", &s.source)),
                            ("text", Value::String(s.text.clone())),
                        ])
                    })
                    .collect(),
            ),
        ),
        ("preferences", text_or_null(log.preferences.as_deref())),
        ("origin", origin_value(&log.origin)),
    ])
}

/// The record without its seal; the seal is the content digest of these FP.5 bytes.
fn unsealed(log: &Log) -> Value {
    let mut fields = vec![
        ("format", Value::String(FORMAT.to_owned())),
        ("version", Value::Int(VERSION)),
        ("identity", identity_value(&log.identity)),
        ("payload", object(vec![("log", log_value(log))])),
        ("provenance", Value::Null),
    ];
    if !log.extensions.is_empty() {
        let extensions = log
            .extensions
            .iter()
            .map(|e| {
                object(vec![
                    ("name", Value::String(e.name.clone())),
                    ("required", Value::Bool(e.required)),
                    ("value", e.value.clone()),
                ])
            })
            .collect();
        fields.push(("extensions", Value::Array(extensions)));
    }
    object(fields)
}

fn seal(unsealed: &Value) -> Result<String, Problem> {
    Ok(c600_digest::sha256_hex(&wire::json_bytes(unsealed)?))
}

/// The record of a log as a JSON value, sealed.
pub fn record(log: &Log) -> Result<Value, Problem> {
    let mut record = unsealed(log);
    let sealed = seal(&record)?;
    if let Value::Object(fields) = &mut record {
        if let Some(Value::Object(payload)) = fields.get_mut("payload") {
            payload.insert("seal".to_owned(), digest_ref("content", &sealed));
        }
    }
    Ok(record)
}

/// The stored bytes of a log (FP.5, or FP.7 above 250,000 bytes). Writing checks nothing about the moves: compile is
/// the check.
pub fn write(log: &Log) -> Result<Vec<u8>, Problem> {
    wire::encode(&record(log)?)
}

// ---------------------------------------------------------------------------------------------------------------
// Reading: the format gate

/// Reads stored bytes as a log: the FP.9 limits, FP.5 and FP.7, FP.10's dispatch, FP.11's fields, the seal and the
/// structure of both histories. It checks no move: that is compile's `moves` gate.
pub fn read(bytes: &[u8]) -> Result<Log, Problem> {
    let record = wire::decode(bytes)?;
    let Value::Object(top) = &record else {
        return Err(Problem::format("record", "a record is an object"));
    };
    // FP.10: dispatch on format and version before any other field.
    let (Some(format), Some(version)) = (top.get("format"), top.get("version")) else {
        return Err(Problem::format("record", "a record names its format and version"));
    };
    if format.as_str() != Some(FORMAT) || version.as_i64() != Some(VERSION) {
        return Err(Problem::new(
            code::PERSISTENCE_VERSION,
            Gate::Format,
            "format",
            "the format and version are not a known log's",
        ));
    }
    let mut fields = Fields::of(&record, "")?;
    fields.value("format")?;
    fields.value("version")?;
    let identity = read_identity(fields.value("identity")?)?;
    let payload = fields.value("payload")?;
    if fields.value("provenance")? != &Value::Null {
        return Err(Problem::format("provenance", "a log record's provenance is null"));
    }
    let extensions = match fields.optional("extensions") {
        Some(value) => read_extensions(value)?,
        None => Vec::new(),
    };
    fields.finish()?;

    let mut payload_fields = Fields::of(payload, "payload")?;
    let log_value = payload_fields.value("log")?;
    let seal_claim = payload_fields.digest("seal", "content")?;
    payload_fields.finish()?;
    let mut unsealed_record = record.clone();
    if let Value::Object(fields) = &mut unsealed_record {
        if let Some(Value::Object(payload)) = fields.get_mut("payload") {
            payload.remove("seal");
        }
    }
    if seal(&unsealed_record)? != seal_claim {
        return Err(Problem::new(
            code::DIGEST_MISMATCH,
            Gate::Format,
            "payload.seal",
            "the record's content differs from its seal",
        ));
    }
    let log = read_log(log_value, identity, extensions)?;
    check_structure(&log)?;
    Ok(log)
}

fn read_identity(value: &Value) -> Result<IdentityBlock, Problem> {
    let mut f = Fields::of(value, "identity")?;
    let puzzle = match f.text("puzzle")?.as_str() {
        "full" => Puzzle::Full,
        "jumbling" => Puzzle::Jumbling,
        _ => return Err(Problem::format("identity.puzzle", "the puzzle is full or jumbling")),
    };
    let identity = IdentityBlock {
        puzzle,
        model_id: f.text("model_id")?,
        menu_id: f.text_or_null("menu_id")?,
        menu_order: f.text_or_null("menu_order")?,
        contract_rev: f.text_or_null("contract_rev")?,
    };
    f.finish()?;
    let jumbling = [&identity.menu_id, &identity.menu_order, &identity.contract_rev];
    let shape = match puzzle {
        Puzzle::Full => jumbling.iter().all(|f| f.is_none()),
        Puzzle::Jumbling => jumbling.iter().all(|f| f.is_some()),
    };
    if !shape {
        return Err(Problem::format("identity", "the jumbling-only fields are null for Full and set for jumbling"));
    }
    Ok(identity)
}

fn read_extensions(value: &Value) -> Result<Vec<Extension>, Problem> {
    let items = value.as_array().ok_or_else(|| Problem::format("extensions", "a list is required here"))?;
    let mut out: Vec<Extension> = Vec::new();
    for (i, item) in items.iter().enumerate() {
        let at = format!("extensions[{i}]");
        let mut f = Fields::of(item, &at)?;
        let extension =
            Extension { name: f.text("name")?, required: f.boolean("required")?, value: f.value("value")?.clone() };
        f.finish()?;
        if out.last().is_some_and(|last| last.name >= extension.name) {
            return Err(Problem::new(code::NONCANONICAL, Gate::Format, at, "extension names are unique and sorted"));
        }
        if extension.required {
            return Err(Problem::new(code::PERSISTENCE_VERSION, Gate::Format, at, "an unknown required extension"));
        }
        out.push(extension);
    }
    Ok(out)
}

fn read_author(f: &mut Fields<'_>) -> Result<Author, Problem> {
    let at = f.path("author");
    let name = f.text("author")?;
    Author::ALL.into_iter().find(|a| a.name() == name).ok_or_else(|| Problem::format(at, "an unknown author class"))
}

fn read_cursor(f: &mut Fields<'_>) -> Result<Cursor, Problem> {
    let at = f.path("action");
    let name = f.text("action")?;
    let action = CursorAction::ALL
        .into_iter()
        .find(|a| a.name() == name)
        .ok_or_else(|| Problem::format(at, "a cursor action is undo, redo, roll or select"))?;
    Ok(Cursor { action, to: f.text_or_null("to")? })
}

fn read_tokens(value: &Value, at: &str) -> Result<Vec<String>, Problem> {
    let items = value.as_array().ok_or_else(|| Problem::format(at, "a list of texts is required here"))?;
    items
        .iter()
        .map(|t| t.as_str().map(str::to_owned).ok_or_else(|| Problem::format(at, "a text is required")))
        .collect()
}

fn read_move(value: &Value, at: &str) -> Result<MoveEntry, Problem> {
    let mut f = Fields::of(value, at)?;
    let id = f.text("id")?;
    let time = f.text_or_null("time")?;
    let kind_at = f.path("kind");
    let action = match f.text("kind")?.as_str() {
        "commit" => {
            let interchange_at = f.path("interchange");
            let interchange = match f.value("interchange")? {
                Value::Null => Interchange::None,
                value => {
                    let mut i = Fields::of(value, &interchange_at)?;
                    let interchange = match i.text("kind")?.as_str() {
                        "c600" => Interchange::C600 { id: i.text_or_null("id")? },
                        "mpult" => {
                            let tokens_at = i.path("tokens");
                            Interchange::MPUlt { tokens: read_tokens(i.value("tokens")?, &tokens_at)? }
                        }
                        _ => return Err(Problem::format(interchange_at, "an unknown interchange kind")),
                    };
                    i.finish()?;
                    interchange
                }
            };
            MoveAction::Commit(Commit {
                parent: f.text_or_null("parent")?,
                origin: f.text("origin")?,
                name: f.text("name")?,
                note: f.text("note")?,
                assistance: f.text("assistance")?,
                recipe: f.value("recipe")?.clone(),
                primitive_count: f.nat("primitive_count")?,
                stars: f.nat("stars")?,
                pre: f.digest("pre", "labels")?,
                post: f.digest("post", "labels")?,
                interchange,
            })
        }
        "partial_undo" => MoveAction::PartialUndo(PartialUndo {
            parent: f.text_or_null("parent")?,
            entry: f.text("entry")?,
            keep: f.nat("keep")?,
            post: f.digest("post", "labels")?,
        }),
        "workspace" => MoveAction::Workspace(WorkspaceStep {
            parent: f.text_or_null("parent")?,
            what: f.text("what")?,
            value: f.value("value")?.clone(),
        }),
        "cursor" => MoveAction::Cursor(read_cursor(&mut f)?),
        _ => return Err(Problem::format(kind_at, "an unknown move history action")),
    };
    f.finish()?;
    Ok(MoveEntry { id, time, action })
}

fn read_layout(value: &Value, at: &str) -> Result<LayoutEntry, Problem> {
    let mut f = Fields::of(value, at)?;
    let id = f.text("id")?;
    let time = f.text_or_null("time")?;
    let kind_at = f.path("kind");
    let action = match f.text("kind")?.as_str() {
        "step" => LayoutAction::Step(LayoutStep {
            parent: f.text_or_null("parent")?,
            what: f.text("what")?,
            origin: f.text("origin")?,
            value: f.value("value")?.clone(),
        }),
        "cursor" => LayoutAction::Cursor(read_cursor(&mut f)?),
        _ => return Err(Problem::format(kind_at, "an unknown layout history action")),
    };
    f.finish()?;
    Ok(LayoutEntry { id, time, action })
}

fn read_document(value: &Value, at: &str) -> Result<OpenDocument, Problem> {
    let mut f = Fields::of(value, at)?;
    let identity_at = f.path("identity");
    let mut i = Fields::of(f.value("identity")?, &identity_at)?;
    let identity = match i.text("kind")?.as_str() {
        "path" => DocumentIdentity::Path(i.text("path")?),
        "untitled" => DocumentIdentity::Untitled(i.text("id")?),
        _ => return Err(Problem::format(identity_at, "a document is named by a path or an untitled id")),
    };
    i.finish()?;
    let kind = f.text("kind")?;
    let saved = f.digest_or_null("saved", "bytes")?;
    let dirty = f.boolean("dirty")?;
    let contents_at = f.path("contents");
    let mut c = Fields::of(f.value("contents")?, &contents_at)?;
    let contents = match c.text("encoding")?.as_str() {
        "utf8" => c.text("text")?.into_bytes(),
        "base64" => {
            let data = c.text("data")?;
            let bytes = wire::unbase64(&data)
                .ok_or_else(|| Problem::format(contents_at.clone(), "the contents are not strict base64"))?;
            if std::str::from_utf8(&bytes).is_ok() {
                return Err(Problem::new(
                    code::NONCANONICAL,
                    Gate::Format,
                    contents_at,
                    "UTF-8 contents are written as text",
                ));
            }
            bytes
        }
        _ => return Err(Problem::format(contents_at, "contents are utf8 text or base64 data")),
    };
    c.finish()?;
    f.finish()?;
    match (&identity, &saved) {
        (DocumentIdentity::Path(_), None) => {
            return Err(Problem::format(at, "a document with a path has the digest of its last saved bytes"))
        }
        (DocumentIdentity::Untitled(_), Some(_)) => {
            return Err(Problem::format(at, "an untitled document has never been saved"))
        }
        _ => {}
    }
    if !dirty {
        if let Some(saved) = &saved {
            if c600_digest::sha256_hex(&contents) != *saved {
                return Err(Problem::new(
                    code::DIGEST_MISMATCH,
                    Gate::Format,
                    at,
                    "a clean document's contents differ from its saved bytes",
                ));
            }
        }
    }
    Ok(OpenDocument { identity, kind, saved, dirty, contents })
}

fn read_origin(value: &Value) -> Result<Origin, Problem> {
    if value == &Value::Null {
        return Ok(Origin::Live);
    }
    let mut f = Fields::of(value, "payload.log.origin")?;
    let kind = f.text("kind")?;
    let format = f.text("format")?;
    let origin = match (kind.as_str(), format.as_str()) {
        ("c600", C600_ORIGIN) => Origin::C600 { source: f.digest("source", "bytes")? },
        ("mpult", MPULT_ORIGIN) => {
            let source = f.digest("source", "bytes")?;
            let timer = f.nat("timer")?;
            let shuffle = f.nat("shuffle")?;
            let pointer = f.nat("pointer")?;
            let trailing_at = f.path("trailing");
            let trailing = read_tokens(f.value("trailing")?, &trailing_at)?;
            let transactions = f.nat("transactions")?;
            Origin::MPUlt(MpultOrigin { source, timer, shuffle, pointer, trailing, transactions })
        }
        _ => return Err(Problem::format("payload.log.origin", "an unknown origin")),
    };
    f.finish()?;
    Ok(origin)
}

fn read_log(value: &Value, identity: IdentityBlock, extensions: Vec<Extension>) -> Result<Log, Problem> {
    let mut f = Fields::of(value, "payload.log")?;
    let start_at = f.path("start");
    let mut s = Fields::of(f.value("start")?, &start_at)?;
    let start = match s.text("kind")?.as_str() {
        "solved" => Start::Solved,
        "labels" => {
            let labels_at = s.path("labels");
            Start::Labels(wire::read_labels(s.value("labels")?, &labels_at)?)
        }
        _ => return Err(Problem::format(start_at, "a start is solved or labels")),
    };
    s.finish()?;

    let history_at = f.path("history");
    let mut h = Fields::of(f.value("history")?, &history_at)?;
    let moves_at = h.path("moves");
    let moves = h
        .array("moves")?
        .iter()
        .enumerate()
        .map(|(i, v)| read_move(v, &format!("{moves_at}[{i}]")))
        .collect::<Result<Vec<_>, _>>()?;
    let layout_at = h.path("layout");
    let layout_history = h
        .array("layout")?
        .iter()
        .enumerate()
        .map(|(i, v)| read_layout(v, &format!("{layout_at}[{i}]")))
        .collect::<Result<Vec<_>, _>>()?;
    h.finish()?;

    let saved_at = f.path("saved");
    let mut sv = Fields::of(f.value("saved")?, &saved_at)?;
    let saved = Saved {
        moves: sv.text_or_null("moves")?,
        layout: sv.text_or_null("layout")?,
        state: sv.digest("state", "labels")?,
    };
    sv.finish()?;

    let view = f.value("view")?.clone();
    let layout = f.value("layout")?.clone();
    let workspace = f.value("workspace")?.clone();
    let documents_at = f.path("documents");
    let documents = f
        .array("documents")?
        .iter()
        .enumerate()
        .map(|(i, v)| read_document(v, &format!("{documents_at}[{i}]")))
        .collect::<Result<Vec<_>, _>>()?;

    let macros_at = f.path("macros");
    let mut macros = Vec::new();
    for (i, v) in f.array("macros")?.iter().enumerate() {
        let mut m = Fields::of(v, &format!("{macros_at}[{i}]"))?;
        macros.push(Macro {
            id: m.text("id")?,
            name: m.text("name")?,
            author: read_author(&mut m)?,
            steps: m.value("steps")?.clone(),
            primitive_count: m.nat("primitive_count")?,
            expansion: m.digest("expansion", "recipe_expansion")?,
        });
        m.finish()?;
    }
    let library_at = f.path("library");
    let mut library = Vec::new();
    for (i, v) in f.array("library")?.iter().enumerate() {
        let mut e = Fields::of(v, &format!("{library_at}[{i}]"))?;
        library.push(LibraryEntry {
            id: e.text("id")?,
            name: e.text("name")?,
            author: read_author(&mut e)?,
            kind: e.text("kind")?,
            value: e.value("value")?.clone(),
        });
        e.finish()?;
    }
    let scripts_at = f.path("scripts");
    let mut scripts = Vec::new();
    for (i, v) in f.array("scripts")?.iter().enumerate() {
        let mut s = Fields::of(v, &format!("{scripts_at}[{i}]"))?;
        scripts.push(Script {
            name: s.text("name")?,
            author: read_author(&mut s)?,
            source: s.digest("source", "source")?,
            text: s.text("text")?,
        });
        s.finish()?;
    }
    let preferences = f.text_or_null("preferences")?;
    let origin = read_origin(f.value("origin")?)?;
    f.finish()?;
    Ok(Log {
        identity,
        start,
        moves,
        layout_history,
        saved,
        view,
        layout,
        workspace,
        documents,
        macros,
        library,
        scripts,
        preferences,
        origin,
        extensions,
    })
}

// ---------------------------------------------------------------------------------------------------------------
// Structure

/// A history's ids, parents and cursor, checked action by action.
struct Tree<'a> {
    parents: HashMap<&'a str, Option<&'a str>>,
    head: Option<&'a str>,
    ids: BTreeSet<&'a str>,
}

impl<'a> Tree<'a> {
    fn new() -> Tree<'a> {
        Tree { parents: HashMap::new(), head: None, ids: BTreeSet::new() }
    }

    fn id(&mut self, id: &'a str, at: &str) -> Result<(), Problem> {
        if id.is_empty() || !self.ids.insert(id) {
            return Err(Problem::format(at, "an action's id is a nonempty text used once"));
        }
        Ok(())
    }

    fn known(&self, node: Option<&str>) -> bool {
        node.is_none_or(|id| self.parents.contains_key(id))
    }

    /// Whether `ancestor` is on the line from the start to `node`, `node` itself excluded.
    fn before(&self, ancestor: Option<&str>, mut node: Option<&'a str>) -> bool {
        while let Some(id) = node {
            node = self.parents[id];
            if node == ancestor {
                return true;
            }
        }
        false
    }

    fn node(&mut self, id: &'a str, parent: Option<&'a str>, at: &str) -> Result<(), Problem> {
        if !self.known(parent) {
            return Err(Problem::format(at, "a node's parent is an earlier node or the start"));
        }
        self.parents.insert(id, parent);
        self.head = Some(id);
        Ok(())
    }

    fn cursor(&mut self, cursor: &'a Cursor, at: &str) -> Result<(), Problem> {
        let to = cursor.to.as_deref();
        if !self.known(to) || to == self.head {
            return Err(Problem::format(at, "a cursor moves to another earlier node or to the start"));
        }
        let fits = match cursor.action {
            CursorAction::Undo => self.before(to, self.head),
            CursorAction::Redo => self.before(self.head, to),
            CursorAction::Roll => self.before(to, self.head) || self.before(self.head, to),
            CursorAction::Select => true,
        };
        if !fits {
            return Err(Problem::format(
                at,
                "an undo goes back on the cursor's line, a redo forward, a roll either way",
            ));
        }
        self.head = to;
        Ok(())
    }
}

fn check_structure(log: &Log) -> Result<(), Problem> {
    let mut tree = Tree::new();
    let mut commits: BTreeMap<&str, (Option<&str>, u64)> = BTreeMap::new();
    let mut partial_of: HashMap<&str, &str> = HashMap::new();
    for (i, entry) in log.moves.iter().enumerate() {
        let at = format!("payload.log.history.moves[{i}]");
        tree.id(&entry.id, &at)?;
        let parent_at = format!("{at}.parent");
        match &entry.action {
            MoveAction::Commit(c) => {
                if c.parent.as_deref() != tree.head {
                    return Err(Problem::format(parent_at, "a commit's parent is where the cursor stood"));
                }
                tree.node(&entry.id, c.parent.as_deref(), &parent_at)?;
                commits.insert(&entry.id, (c.parent.as_deref(), c.primitive_count));
            }
            MoveAction::PartialUndo(p) => {
                let Some(&(parent, length)) = commits.get(p.entry.as_str()) else {
                    return Err(Problem::format(format!("{at}.entry"), "a partial undo names an earlier commit"));
                };
                let on_entry = tree.head == Some(p.entry.as_str())
                    || tree.head.is_some_and(|head| partial_of.get(head) == Some(&p.entry.as_str()));
                if !on_entry {
                    return Err(Problem::format(
                        format!("{at}.entry"),
                        "a partial undo steps back inside the commit where the cursor stands",
                    ));
                }
                if p.parent.as_deref() != parent {
                    return Err(Problem::format(parent_at, "a partial undo's parent is its commit's parent"));
                }
                if p.keep >= length {
                    return Err(Problem::format(
                        format!("{at}.keep"),
                        "a partial undo keeps fewer letters than the commit",
                    ));
                }
                tree.node(&entry.id, parent, &parent_at)?;
                partial_of.insert(&entry.id, &p.entry);
            }
            MoveAction::Workspace(w) => {
                if w.parent.as_deref() != tree.head {
                    return Err(Problem::format(parent_at, "a workspace action's parent is where the cursor stood"));
                }
                tree.node(&entry.id, w.parent.as_deref(), &parent_at)?;
            }
            MoveAction::Cursor(cursor) => tree.cursor(cursor, &format!("{at}.to"))?,
        }
    }
    if log.saved.moves.as_deref() != tree.head {
        return Err(Problem::format("payload.log.saved.moves", "the saved move cursor is where the history ends"));
    }

    let mut layout = Tree::new();
    for (i, entry) in log.layout_history.iter().enumerate() {
        let at = format!("payload.log.history.layout[{i}]");
        layout.id(&entry.id, &at)?;
        match &entry.action {
            LayoutAction::Step(step) => {
                let parent_at = format!("{at}.parent");
                if step.parent.as_deref() != layout.head {
                    return Err(Problem::format(parent_at, "a layout step's parent is where the cursor stood"));
                }
                layout.node(&entry.id, step.parent.as_deref(), &parent_at)?;
            }
            LayoutAction::Cursor(cursor) => layout.cursor(cursor, &format!("{at}.to"))?,
        }
    }
    if log.saved.layout.as_deref() != layout.head {
        return Err(Problem::format("payload.log.saved.layout", "the saved layout cursor is where the history ends"));
    }

    let mut names = BTreeSet::new();
    for (i, script) in log.scripts.iter().enumerate() {
        if !names.insert(script.name.as_str()) {
            return Err(Problem::format(format!("payload.log.scripts[{i}].name"), "script names are unique"));
        }
    }
    let mut ids = BTreeSet::new();
    for (i, m) in log.macros.iter().enumerate() {
        if !ids.insert(m.id.as_str()) {
            return Err(Problem::format(format!("payload.log.macros[{i}].id"), "macro ids are unique"));
        }
    }
    Ok(())
}

//! The host's document service over the session store (LP4-14; ADR-D01.79 to .84 as revision 6's R-2 and R-3 amend
//! them; draft): [`DocumentStore`] answers the command layer's document and log rows ([`c600_command::Documents`]).
//!
//! - The live session is the first document, [`SESSION_DOCUMENT`]: always open, never closed, and saved as a log at
//!   `app.exit` without a prompt, with every open document's contents (ADR-D01.79).
//! - [`session_log`] converts the session's move history into a [`Log`] through `c600-log` (whose manifest keeps no
//!   path to `c600-run`): each commit with its legal witness, each undo and redo as a cursor move, each record,
//!   protection, policy and workspace completion as a workspace node, each layout-history step as a layout step.
//! - Opening, saving, compiling and checking change no live state: the service holds documents and files, never the
//!   layer's state. A compile result is bound to the SHA-256 of the bytes it compiled, and a log binds a state only
//!   when its compile passed (revision 6, R-2).
//! - `file.log.check` records its result by the file and the SHA-256 of the bytes it read; `file.log.import`'s plan
//!   comes only from a passed check of the file's current bytes (ADR-D01.82). The service keeps the current
//!   preferences: an import records the log's preferences and never applies them (ADR-D01.80).
//!
//! Not built: a session with Jumble on (a crossing, a jumble move or an exit) converts to no log
//! (`KN5:input.row_not_built`); a partial undo and a rollback step convert as what the journal holds, a commit of the
//! retained word and cursor moves, not as the log's `partial_undo` form; MPUlt check and export need the host's native
//! profile ([`DocumentStore::set_native_profile`]).

use std::collections::BTreeMap;
use std::path::{Component, Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use c600_command::documents::{CheckedFile, DocumentKind, Documents, ImportPlan, LogFormat, SESSION_DOCUMENT};
use c600_command::{code as layer_code, Error, History, Table, TransactionKind, UndoClass};
use c600_digest::sha256_hex;
use c600_kernel::json::{self, Value};
use c600_kernel::{Kernel, Snapshot, Step};
use c600_log::c600::{self, Compression};
use c600_log::mpult::{self, NativeProfile};
use c600_log::{
    compile, recipe_value, Commit, Compiled, Cursor, CursorAction, DocumentIdentity, Interchange, LayoutAction,
    LayoutEntry, LayoutStep, Log, MoveAction, MoveEntry, OpenDocument, Problem, Saved, Start, WorkspaceStep,
};

use crate::command_store::HistoryReader;
use crate::types::code;

/// The log file's extension with its dot (`c600_log::EXTENSION`).
const LOG_SUFFIX: &str = ".c600log";
/// A `.6cs` script.
const SCRIPT_SUFFIX: &str = ".6cs";
/// 0.4's proof log, raw or in a gzip member (`c600_log::c600::EXTENSION`).
const C600_SUFFIXES: [&str; 2] = [".c600.json.gz", ".c600.json"];
/// An MPUlt log (`c600_log::mpult::EXTENSION`).
const MPULT_SUFFIX: &str = ".log";
/// A commit's origin in the log: made by the import row, or by the solver's hand otherwise (draft).
const IMPORT_ORIGIN: &str = "import";
const HAND_ORIGIN: &str = "hand";

fn text(value: &str) -> Value {
    Value::String(value.to_owned())
}

fn from_problem(problem: Problem) -> Error {
    Error::new(problem.code, format!("{}: {}", problem.at, problem.message))
}

fn problems_json(problems: &[Problem]) -> Value {
    Value::Array(
        problems
            .iter()
            .map(|p| {
                json::object([
                    ("code", text(p.code)),
                    ("gate", text(p.gate.name())),
                    ("at", text(&p.at)),
                    ("message", text(&p.message)),
                ])
            })
            .collect(),
    )
}

// ----- the files -----

/// Where documents are read and written: a write is whole or absent.
pub trait Files: Send {
    fn read(&mut self, file: &str) -> Result<Vec<u8>, Error>;
    fn write(&mut self, file: &str, bytes: &[u8]) -> Result<(), Error>;
}

/// Files in memory. Clones share them, so a test keeps a handle after the service took one.
#[derive(Clone, Debug, Default)]
pub struct MemFiles(Arc<Mutex<BTreeMap<String, Vec<u8>>>>);

impl MemFiles {
    pub fn new() -> MemFiles {
        MemFiles::default()
    }

    fn files(&self) -> MutexGuard<'_, BTreeMap<String, Vec<u8>>> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// The bytes of `file`, if it exists.
    pub fn get(&self, file: &str) -> Option<Vec<u8>> {
        self.files().get(file).cloned()
    }

    /// Puts `bytes` at `file`, as a file another program wrote.
    pub fn put(&self, file: &str, bytes: &[u8]) {
        self.files().insert(file.to_owned(), bytes.to_vec());
    }
}

impl Files for MemFiles {
    fn read(&mut self, file: &str) -> Result<Vec<u8>, Error> {
        self.get(file).ok_or_else(|| Error::new(layer_code::ARGUMENT_FORM, format!("no file `{file}`")))
    }

    fn write(&mut self, file: &str, bytes: &[u8]) -> Result<(), Error> {
        self.put(file, bytes);
        Ok(())
    }
}

/// Files under a root folder of the file system. A name is a relative path without `..`; a write goes to a sibling
/// file first, is synced, and is renamed over the target.
pub struct FsFiles {
    root: PathBuf,
}

impl FsFiles {
    pub fn new(root: &Path) -> FsFiles {
        FsFiles { root: root.to_owned() }
    }

    fn path(&self, file: &str) -> Result<PathBuf, Error> {
        let relative = Path::new(file);
        let plain = relative.components().all(|c| matches!(c, Component::Normal(_)));
        if file.is_empty() || !plain {
            return Err(Error::new(layer_code::ARGUMENT_FORM, format!("`{file}` is not a relative file name")));
        }
        Ok(self.root.join(relative))
    }
}

impl Files for FsFiles {
    fn read(&mut self, file: &str) -> Result<Vec<u8>, Error> {
        let path = self.path(file)?;
        std::fs::read(&path).map_err(|e| Error::new(layer_code::ARGUMENT_FORM, format!("`{file}` does not read: {e}")))
    }

    fn write(&mut self, file: &str, bytes: &[u8]) -> Result<(), Error> {
        use std::io::Write;
        let path = self.path(file)?;
        let failed = |e: std::io::Error| Error::new(code::DISK_FULL, format!("`{file}` was not written: {e}"));
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(failed)?;
        }
        // The file's own name with `.partial` appended, so no two targets share a sibling.
        let mut name = path.file_name().unwrap_or_default().to_owned();
        name.push(".partial");
        let partial = path.with_file_name(name);
        let mut out = std::fs::File::create(&partial).map_err(failed)?;
        out.write_all(bytes).and_then(|()| out.sync_all()).map_err(failed)?;
        drop(out);
        std::fs::rename(&partial, &path).map_err(failed)
    }
}

// ----- the conversion -----

/// Converts a session's history into a log (LP4-14): the session's start and identity, each commit with its legal
/// witness (its normalized recipe, from its pre to its post state), each undo and redo of the move history as a cursor
/// move to the node it returns to or repeats, each record write, protection change, policy change and workspace
/// completion of the move history as a workspace node, and each layout-history step and its undo and redo in the
/// layout history. Both cursors are saved where the session stands. A history that broke, a session of another
/// model and a session with Jumble on are refused.
pub fn session_log(kernel: &Kernel, table: &Table, history: &History) -> Result<Log, Error> {
    if let Some(damaged) = &history.damaged {
        return Err(damaged.clone());
    }
    let genesis = &history.genesis;
    if genesis.identity != kernel.identity() {
        return Err(Error::new(c600_kernel::codes::IDENTITY_MISMATCH, "the session belongs to another model"));
    }
    let mut log = Log::new(genesis.identity.clone(), &genesis.digest);
    if genesis.digest != kernel.solved().digest() {
        log.start = Start::Labels(genesis.labels.clone());
    }
    let mut c = Conversion::default();
    for t in &history.transactions {
        let workspace = |what: &str, value: Value| (what.to_owned(), value);
        let step = match t.kind {
            TransactionKind::Commit => {
                let recipe = kernel.recipe(&t.recipe)?;
                let stars = recipe.steps().iter().filter(|step| matches!(step, Step::Star(_))).count() as u64;
                let origin = if t.row == c600_command::LOG_IMPORT_ROW { IMPORT_ORIGIN } else { HAND_ORIGIN };
                let commit = Commit {
                    parent: c.head.clone(),
                    origin: origin.to_owned(),
                    name: t.row.clone(),
                    note: String::new(),
                    assistance: "manual".to_owned(),
                    recipe: recipe_value(&recipe),
                    primitive_count: recipe.primitive_count() as u64,
                    stars,
                    pre: t.pre.clone(),
                    post: t.post.clone(),
                    interchange: Interchange::None,
                };
                c.node(&mut log, t.order, MoveAction::Commit(commit), t.post.clone());
                continue;
            }
            TransactionKind::Undo | TransactionKind::Redo => {
                let undo = t.kind == TransactionKind::Undo;
                c.cursor(&mut log, t.target, undo)?;
                continue;
            }
            TransactionKind::Workspace if table.row(&t.row).is_some_and(|row| row.undo == UndoClass::Layout) => {
                let records = Value::Array(t.records.iter().map(|r| r.to_json()).collect());
                c.layout(&mut log, t.order, &t.row, json::object([("records", records)]));
                continue;
            }
            TransactionKind::Workspace => {
                let records = Value::Array(t.records.iter().map(|r| r.to_json()).collect());
                let result = t.result.clone().map_or(Value::Null, Value::String);
                workspace("workspace", json::object([("row", text(&t.row)), ("records", records), ("result", result)]))
            }
            TransactionKind::Record => {
                let record = t.record.as_ref().map_or(Value::Null, |r| r.to_json());
                workspace("record", json::object([("row", text(&t.row)), ("record", record)]))
            }
            TransactionKind::Protection => {
                let orbits = Value::Array(t.orbits.iter().map(|&o| Value::Int(o as i64)).collect());
                workspace("protection", json::object([("orbits", orbits)]))
            }
            TransactionKind::Policy => {
                let policy = t.policy.map_or(Value::Null, |p| text(c600_command::policy_name(p)));
                workspace("policy", json::object([("policy", policy)]))
            }
            TransactionKind::Crossing | TransactionKind::JumbleMove | TransactionKind::Exit => {
                return Err(Error::new(
                    layer_code::ROW_NOT_BUILT,
                    "a session with Jumble on converts to no log (LP4-14: not built)",
                ));
            }
        };
        let state = c.state(&genesis.digest);
        let (what, value) = step;
        let node = WorkspaceStep { parent: c.head.clone(), what, value };
        c.node(&mut log, t.order, MoveAction::Workspace(node), state);
    }
    let state = c.state(&genesis.digest);
    log.saved = Saved { moves: c.head.clone(), layout: c.layout_head.clone(), state };
    Ok(log)
}

/// The conversion's place in both histories.
#[derive(Default)]
struct Conversion {
    next: u64,
    head: Option<String>,
    layout_head: Option<String>,
    /// Each action order's node in the move history, with the node's parent and state.
    nodes: BTreeMap<u64, String>,
    parents: BTreeMap<String, Option<String>>,
    states: BTreeMap<String, String>,
    /// Each action order's step in the layout history, with the step's parent.
    layouts: BTreeMap<u64, String>,
    layout_parents: BTreeMap<String, Option<String>>,
}

impl Conversion {
    fn id(&mut self) -> String {
        self.next += 1;
        self.next.to_string()
    }

    fn state(&self, start: &str) -> String {
        self.head.as_ref().and_then(|head| self.states.get(head)).cloned().unwrap_or_else(|| start.to_owned())
    }

    fn node(&mut self, log: &mut Log, order: u64, action: MoveAction, state: String) {
        let id = self.id();
        self.parents.insert(id.clone(), self.head.clone());
        self.states.insert(id.clone(), state);
        self.nodes.insert(order, id.clone());
        log.moves.push(MoveEntry { id: id.clone(), time: None, action });
        self.head = Some(id);
    }

    fn layout(&mut self, log: &mut Log, order: u64, row: &str, value: Value) {
        let id = self.id();
        let step = LayoutStep {
            parent: self.layout_head.clone(),
            what: row.to_owned(),
            origin: HAND_ORIGIN.to_owned(),
            value,
        };
        self.layout_parents.insert(id.clone(), self.layout_head.clone());
        self.layouts.insert(order, id.clone());
        log.layout_history.push(LayoutEntry { id: id.clone(), time: None, action: LayoutAction::Step(step) });
        self.layout_head = Some(id);
    }

    /// An undo returns to the parent of the node of the action it inverts; a redo goes to that node. When the cursor
    /// does not stand on that node's line, as after a node that left the session's undo and redo stacks alone (a
    /// workspace completion of undo class `none`), the move is a `select`, which may go anywhere; a move to where the
    /// cursor stands records nothing.
    fn cursor(&mut self, log: &mut Log, target: Option<u64>, undo: bool) -> Result<(), Error> {
        let unknown = || Error::new(code::PERSISTENCE_FORMAT, "an undo or redo of an action the history does not hold");
        let target = target.ok_or_else(unknown)?;
        let (moves, node) = match (self.nodes.get(&target), self.layouts.get(&target)) {
            (Some(node), _) => (true, node.clone()),
            (None, Some(step)) => (false, step.clone()),
            (None, None) => return Err(unknown()),
        };
        let parents = if moves { &self.parents } else { &self.layout_parents };
        let to = if undo { parents.get(&node).cloned().flatten() } else { Some(node) };
        let head = if moves { self.head.clone() } else { self.layout_head.clone() };
        if to == head {
            return Ok(());
        }
        let fits = if undo {
            before(parents, to.as_deref(), head.as_deref())
        } else {
            before(parents, head.as_deref(), to.as_deref())
        };
        let action = match (fits, undo) {
            (false, _) => CursorAction::Select,
            (true, true) => CursorAction::Undo,
            (true, false) => CursorAction::Redo,
        };
        let id = self.id();
        let cursor = Cursor { action, to: to.clone() };
        if moves {
            log.moves.push(MoveEntry { id, time: None, action: MoveAction::Cursor(cursor) });
            self.head = to;
        } else {
            log.layout_history.push(LayoutEntry { id, time: None, action: LayoutAction::Cursor(cursor) });
            self.layout_head = to;
        }
        Ok(())
    }
}

/// Whether `ancestor` is on the line from the start to `node`, `node` excluded (c600-log's own reading of a cursor
/// move).
fn before(parents: &BTreeMap<String, Option<String>>, ancestor: Option<&str>, node: Option<&str>) -> bool {
    let mut node = node.map(str::to_owned);
    while let Some(id) = node {
        node = parents.get(&id).cloned().flatten();
        if node.as_deref() == ancestor {
            return true;
        }
    }
    false
}

// ----- the service -----

/// One open document other than the live session.
struct Doc {
    kind: DocumentKind,
    /// Its file; `None` while untitled.
    file: Option<String>,
    contents: Vec<u8>,
    /// The SHA-256 of the bytes last saved or read; `None` while untitled.
    saved: Option<String>,
}

impl Doc {
    fn dirty(&self) -> bool {
        self.saved.as_deref() != Some(sha256_hex(&self.contents).as_str())
    }
}

/// A recorded `file.log.check`.
struct Check {
    passed: bool,
    /// Its first problem, when it did not pass.
    problem: Option<(&'static str, String)>,
    plan: Option<ImportPlan>,
}

/// The document service of one session (LP4-14; draft).
pub struct DocumentStore {
    kernel: Arc<Kernel>,
    table: Table,
    history: HistoryReader,
    files: Box<dyn Files>,
    /// Where the live session's log is saved.
    session_file: String,
    preferences: Option<String>,
    profile: Option<NativeProfile>,
    documents: BTreeMap<String, Doc>,
    untitled: u64,
    checks: BTreeMap<(String, String), Check>,
    checkpoints: u64,
}

impl DocumentStore {
    /// The service of the session whose history `history` reads, saving the live session's log at `session_file`.
    /// `table` is the layer's, which names each row's undo class.
    pub fn new(
        kernel: Arc<Kernel>,
        table: Table,
        history: HistoryReader,
        files: Box<dyn Files>,
        session_file: &str,
    ) -> DocumentStore {
        DocumentStore {
            kernel,
            table,
            history,
            files,
            session_file: session_file.to_owned(),
            preferences: None,
            profile: None,
            documents: BTreeMap::new(),
            untitled: 0,
            checks: BTreeMap::new(),
            checkpoints: 0,
        }
    }

    /// The current preferences as canonical JSON text, which every saved log of the session records.
    pub fn set_preferences(&mut self, preferences: Option<String>) {
        self.preferences = preferences;
    }

    pub fn preferences(&self) -> Option<&str> {
        self.preferences.as_deref()
    }

    /// The native profile that MPUlt check and export need.
    pub fn set_native_profile(&mut self, profile: NativeProfile) {
        self.profile = Some(profile);
    }

    /// Where the live session's log is saved.
    pub fn session_file(&self) -> &str {
        &self.session_file
    }

    /// The open documents other than the live session, by id.
    pub fn open_documents(&self) -> Vec<&str> {
        self.documents.keys().map(String::as_str).collect()
    }

    /// The host editor's change of a document's contents: it becomes dirty until it is saved.
    pub fn edit(&mut self, document: &str, contents: &[u8]) -> Result<(), Error> {
        self.doc(document)?.contents = contents.to_vec();
        Ok(())
    }

    fn doc(&mut self, document: &str) -> Result<&mut Doc, Error> {
        self.documents
            .get_mut(document)
            .ok_or_else(|| Error::new(layer_code::ARGUMENT_FORM, format!("no open document `{document}`")))
    }

    /// The live session's log as the service saves it: the history's conversion, the current preferences and every
    /// open document.
    pub fn session(&self) -> Result<Log, Error> {
        let mut log = session_log(&self.kernel, &self.table, &self.history.history()?)?;
        log.preferences = self.preferences.clone();
        log.documents = self
            .documents
            .iter()
            .map(|(id, doc)| OpenDocument {
                identity: match &doc.file {
                    Some(file) => DocumentIdentity::Path(file.clone()),
                    None => DocumentIdentity::Untitled(id.clone()),
                },
                kind: doc.kind.name().to_owned(),
                saved: doc.saved.clone(),
                dirty: doc.dirty(),
                contents: doc.contents.clone(),
            })
            .collect();
        Ok(log)
    }

    fn session_bytes(&self) -> Result<Vec<u8>, Error> {
        c600_log::write(&self.session()?).map_err(from_problem)
    }

    fn write_session(&mut self, file: &str) -> Result<Value, Error> {
        let bytes = self.session_bytes()?;
        self.files.write(file, &bytes)?;
        Ok(json::object([
            ("document", text(SESSION_DOCUMENT)),
            ("file", text(file)),
            ("sha256", text(&sha256_hex(&bytes))),
        ]))
    }

    /// A compiled log's answer: bound to its bytes' SHA-256, and bound to its saved state only when it passed.
    fn compiled_json(&self, document: &str, compiled: &Compiled) -> Value {
        let bound = compiled.passed();
        let state = compiled.state().filter(|_| bound).map_or(Value::Null, |s| text(s.digest()));
        json::object([
            ("document", text(document)),
            ("kind", text(DocumentKind::Log.name())),
            ("sha256", text(compiled.bytes_sha256())),
            ("passed", Value::Bool(bound)),
            ("bound", Value::Bool(bound)),
            ("state", state),
            ("problems", problems_json(compiled.problems())),
        ])
    }

    /// A log in any format this service reads, as `c600-log` compiles it.
    fn compile_file(&self, file: &str, bytes: &[u8]) -> Result<(LogFormat, Compiled), Problem> {
        let format = format_of(file).ok_or_else(|| {
            Problem::new(code::PERSISTENCE_FORMAT, c600_log::Gate::Format, file, "not a log, 0.4 or MPUlt file name")
        })?;
        let log = match format {
            LogFormat::Log => return Ok((format, compile(&self.kernel, bytes))),
            LogFormat::C600 => c600::import(&self.kernel, bytes)?,
            LogFormat::Mpult => match &self.profile {
                Some(profile) => mpult::import(&self.kernel, profile, bytes)?,
                None => {
                    return Err(Problem::new(
                        layer_code::ROW_NOT_BUILT,
                        c600_log::Gate::Interchange,
                        file,
                        "an MPUlt log needs the host's native profile",
                    ))
                }
            },
        };
        Ok((format, compile(&self.kernel, &c600_log::write(&log)?)))
    }

    /// The import's plan from a passed compile: the line from the log's start to its saved cursor in the kernel's
    /// steps, a partial undo as the kept letters of its entry.
    fn plan(&self, checked: CheckedFile, log: &Log, state: &Snapshot) -> Result<ImportPlan, Error> {
        let start = match &log.start {
            Start::Solved => self.kernel.solved(),
            Start::Labels(labels) => self.kernel.state_from_labels(labels.clone()).ok_or_else(|| {
                Error::new(code::PERSISTENCE_FORMAT, "the log's start labels are not a state of the model")
            })?,
        };
        let mut line = Vec::new();
        for entry in log.line(log.saved.moves.as_deref()) {
            match &entry.action {
                MoveAction::Commit(commit) => line.extend(commit.recipe.as_array().into_iter().flatten().cloned()),
                MoveAction::PartialUndo(undo) => {
                    let Some(MoveAction::Commit(commit)) = log.node(&undo.entry).map(|e| &e.action) else {
                        return Err(Error::new(code::PERSISTENCE_FORMAT, "a partial undo names no commit"));
                    };
                    let letters = self.kernel.letters(&self.kernel.recipe(&commit.recipe)?)?;
                    let kept = letters.iter().take(undo.keep as usize).map(|&m| Value::Int(i64::from(m))).collect();
                    line.push(json::object([("kind", text("word")), ("moves", Value::Array(kept))]));
                }
                _ => {}
            }
        }
        Ok(ImportPlan {
            checked,
            model: log.identity.model_id.clone(),
            start: start.digest().to_owned(),
            line: Value::Array(line),
            state: state.digest().to_owned(),
            preferences: log.preferences.clone(),
        })
    }
}

fn kind_of(file: &str) -> Option<DocumentKind> {
    if file.ends_with(LOG_SUFFIX) {
        Some(DocumentKind::Log)
    } else if file.ends_with(SCRIPT_SUFFIX) {
        Some(DocumentKind::Script)
    } else {
        None
    }
}

fn format_of(file: &str) -> Option<LogFormat> {
    if file.ends_with(LOG_SUFFIX) {
        Some(LogFormat::Log)
    } else if C600_SUFFIXES.iter().any(|suffix| file.ends_with(suffix)) {
        Some(LogFormat::C600)
    } else if file.ends_with(MPULT_SUFFIX) {
        Some(LogFormat::Mpult)
    } else {
        None
    }
}

impl Documents for DocumentStore {
    fn create(&mut self, kind: DocumentKind) -> Result<Value, Error> {
        let contents = match kind {
            DocumentKind::Log => {
                let empty = Log::new(self.kernel.identity(), self.kernel.solved().digest());
                c600_log::write(&empty).map_err(from_problem)?
            }
            DocumentKind::Script => Vec::new(),
        };
        self.untitled += 1;
        let id = format!("untitled-{}", self.untitled);
        self.documents.insert(id.clone(), Doc { kind, file: None, contents, saved: None });
        Ok(json::object([("document", text(&id)), ("kind", text(kind.name())), ("untitled", Value::Bool(true))]))
    }

    fn open(&mut self, file: &str) -> Result<Value, Error> {
        let Some(kind) = kind_of(file) else {
            return Err(Error::new(layer_code::ARGUMENT_FORM, format!("`{file}` is not a log or a script")));
        };
        if !self.documents.contains_key(file) {
            let contents = self.files.read(file)?;
            let saved = Some(sha256_hex(&contents));
            self.documents.insert(file.to_owned(), Doc { kind, file: Some(file.to_owned()), contents, saved });
        }
        let doc = &self.documents[file];
        Ok(json::object([
            ("document", text(file)),
            ("kind", text(kind.name())),
            ("file", text(file)),
            ("sha256", text(&sha256_hex(&doc.contents))),
            ("dirty", Value::Bool(doc.dirty())),
        ]))
    }

    fn save(&mut self, document: &str) -> Result<Value, Error> {
        if document == SESSION_DOCUMENT {
            let file = self.session_file.clone();
            return self.write_session(&file);
        }
        let doc = self.doc(document)?;
        let Some(file) = doc.file.clone() else {
            return Err(Error::new(layer_code::ARGUMENT_FORM, "an untitled document is saved with `save_as`"));
        };
        let bytes = doc.contents.clone();
        self.files.write(&file, &bytes)?;
        let sha256 = sha256_hex(&bytes);
        self.doc(document)?.saved = Some(sha256.clone());
        Ok(json::object([("document", text(document)), ("file", text(&file)), ("sha256", text(&sha256))]))
    }

    fn save_as(&mut self, document: &str, destination: &str) -> Result<Value, Error> {
        if document == SESSION_DOCUMENT {
            if kind_of(destination) != Some(DocumentKind::Log) {
                return Err(Error::new(layer_code::ARGUMENT_FORM, "the live session is saved as a `.c600log`"));
            }
            let answer = self.write_session(destination)?;
            self.session_file = destination.to_owned();
            return Ok(answer);
        }
        let doc = self.doc(document)?;
        if kind_of(destination) != Some(doc.kind) {
            return Err(Error::new(layer_code::ARGUMENT_FORM, "the destination names another kind of document"));
        }
        let bytes = doc.contents.clone();
        self.files.write(destination, &bytes)?;
        let sha256 = sha256_hex(&bytes);
        let doc = self.doc(document)?;
        doc.file = Some(destination.to_owned());
        doc.saved = Some(sha256.clone());
        Ok(json::object([("document", text(document)), ("file", text(destination)), ("sha256", text(&sha256))]))
    }

    fn compile(&mut self, document: &str) -> Result<Value, Error> {
        if document == SESSION_DOCUMENT {
            let bytes = self.session_bytes()?;
            return Ok(self.compiled_json(document, &compile(&self.kernel, &bytes)));
        }
        let doc = self.doc(document)?;
        let (kind, contents) = (doc.kind, doc.contents.clone());
        if kind == DocumentKind::Log {
            return Ok(self.compiled_json(document, &compile(&self.kernel, &contents)));
        }
        // A script is diagnosed by the editor services and never run; it binds no state.
        let source = c600_editor::Document::open(contents.clone(), c600_editor::Units::Utf8);
        let context = c600_editor::ModuleContext::new();
        let (passed, problems) = match c600_editor::diagnose(&source, source.digest(), &context) {
            Ok(diagnosis) => {
                let passed =
                    diagnosis.diagnostics.is_empty() && matches!(diagnosis.verdict, c600_editor::Verdict::Checked(_));
                let problems = diagnosis.diagnostics.iter().map(|e| (e.code, e.summary.clone())).collect::<Vec<_>>();
                (passed, problems)
            }
            Err(envelope) => (false, vec![(envelope.code, envelope.summary)]),
        };
        let problems = problems.iter().map(|(c, m)| json::object([("code", text(c)), ("message", text(m))])).collect();
        Ok(json::object([
            ("document", text(document)),
            ("kind", text(DocumentKind::Script.name())),
            ("sha256", text(&sha256_hex(&contents))),
            ("passed", Value::Bool(passed)),
            ("bound", Value::Bool(false)),
            ("state", Value::Null),
            ("problems", Value::Array(problems)),
        ]))
    }

    fn close(&mut self, document: &str, discard: bool) -> Result<Value, Error> {
        if document == SESSION_DOCUMENT {
            return Err(Error::new(layer_code::ARGUMENT_FORM, "the live session does not close"));
        }
        let dirty = self.doc(document)?.dirty();
        if dirty && !discard {
            return Err(Error::new(
                layer_code::CONFIRMATION_REQUIRED,
                "a document with unsaved changes closes only with `discard`",
            ));
        }
        self.documents.remove(document);
        Ok(json::object([("document", text(document)), ("discarded", Value::Bool(dirty))]))
    }

    fn check(&mut self, file: &str) -> Result<Value, Error> {
        let bytes = self.files.read(file)?;
        let sha256 = sha256_hex(&bytes);
        let checked = CheckedFile { file: file.to_owned(), sha256: sha256.clone() };
        let (format, passed, problems, state, plan) = match self.compile_file(file, &bytes) {
            Ok((format, compiled)) => {
                let passed = compiled.passed();
                let plan = match (passed, compiled.log(), compiled.state()) {
                    (true, Some(log), Some(state)) => Some(self.plan(checked.clone(), log, state)?),
                    _ => None,
                };
                let state = plan.as_ref().map(|plan| plan.state.clone());
                (Some(format), passed && plan.is_some(), compiled.problems().to_vec(), state, plan)
            }
            Err(problem) => (format_of(file), false, vec![problem], None, None),
        };
        let problem = problems.first().map(|p| (p.code, format!("{}: {}", p.at, p.message)));
        let problem = match (passed, problem) {
            (false, None) => Some((code::PERSISTENCE_FORMAT, "the check did not pass".to_owned())),
            (_, problem) => problem,
        };
        self.checks.insert((file.to_owned(), sha256.clone()), Check { passed, problem, plan });
        Ok(json::object([
            ("file", text(file)),
            ("sha256", text(&sha256)),
            ("format", format.map_or(Value::Null, |f| text(f.name()))),
            ("passed", Value::Bool(passed)),
            ("state", state.map_or(Value::Null, Value::String)),
            ("problems", problems_json(&problems)),
        ]))
    }

    fn export(&mut self, format: LogFormat, destination: &str) -> Result<Value, Error> {
        let bytes = self.session_bytes()?;
        let written = match format {
            LogFormat::Log => bytes,
            LogFormat::C600 => {
                let compression = if destination.ends_with(".gz") { Compression::Gzip } else { Compression::None };
                c600::export(&self.kernel, &compile(&self.kernel, &bytes), compression).map_err(from_problem)?
            }
            LogFormat::Mpult => {
                let Some(profile) = &self.profile else {
                    return Err(Error::new(
                        layer_code::ROW_NOT_BUILT,
                        "an MPUlt export needs the host's native profile",
                    ));
                };
                mpult::export(&self.kernel, profile, &compile(&self.kernel, &bytes), None).map_err(from_problem)?
            }
        };
        self.files.write(destination, &written)?;
        Ok(json::object([
            ("format", text(format.name())),
            ("file", text(destination)),
            ("sha256", text(&sha256_hex(&written))),
        ]))
    }

    fn import_plan(&mut self, checked: &CheckedFile) -> Result<ImportPlan, Error> {
        let Some(check) = self.checks.get(&(checked.file.clone(), checked.sha256.clone())) else {
            return Err(Error::new(layer_code::RESULT_GONE, "no check of this file and digest"));
        };
        if !check.passed {
            let (code, message) = check.problem.clone().unwrap_or((code::PERSISTENCE_FORMAT, String::new()));
            return Err(Error::new(code, format!("the file's check did not pass: {message}")));
        }
        let plan = check.plan.clone().expect("a passed check has its plan");
        let bytes = self.files.read(&checked.file)?;
        if sha256_hex(&bytes) != checked.sha256 {
            return Err(Error::new(code::DIGEST_MISMATCH, "the file's bytes changed since its check"));
        }
        Ok(plan)
    }

    fn checkpoint(&mut self) -> Result<String, Error> {
        let bytes = self.session_bytes()?;
        let stem = self.session_file.strip_suffix(LOG_SUFFIX).unwrap_or(&self.session_file).to_owned();
        // The first number no file holds, so a service started again overwrites no earlier checkpoint.
        let name = loop {
            self.checkpoints += 1;
            let name = format!("{stem}.checkpoint-{}{LOG_SUFFIX}", self.checkpoints);
            if self.files.read(&name).is_err() {
                break name;
            }
        };
        self.files.write(&name, &bytes)?;
        Ok(name)
    }

    fn exit(&mut self) -> Result<Value, Error> {
        let file = self.session_file.clone();
        let mut answer = self.write_session(&file)?;
        if let Value::Object(fields) = &mut answer {
            fields.insert("documents".to_owned(), Value::Int(self.documents.len() as i64));
        }
        Ok(answer)
    }
}

//! The document rows and the redesigned log rows (ADR-D01.79 to .84 as revision 6's R-2 and R-3 amend them; LP4-14;
//! draft): `file.document.new`, `.open`, `.save`, `.save_as`, `.compile` and `.close`, `file.log.export`, `.check` and
//! `.import`, and `app.exit`, which saves the live session.
//!
//! The command layer depends on no language crate (CT-LI-07), so it never reads, compiles or writes a document: the
//! host's document service ([`Documents`]) does, over the log format and its compiler (`c600-log`) and the editor
//! services; `c600_session::documents::DocumentStore` is one. The layer admits the rows, runs the compile, check and
//! export jobs on the exclusive lane, and commits an import itself: its two gates, its recovery checkpoint, the
//! playback states it stops and its one undoable journal entry ([`crate::CommandLayer`]). Without a service every one of
//! these rows is `KN5:input.row_not_built`, as before.
//!
//! - A document is named by the service's id ([`DOCUMENT_REF`]), a nonempty text. The live session is the first
//!   document, [`SESSION_DOCUMENT`]; it is always open and does not close.
//! - Opening, saving and compiling change no live state: none takes a token, a gate, a preview or an undo entry, and the
//!   layer's state, revision and journal stay as they were.
//! - A compile result is bound to the SHA-256 of the bytes it compiled (revision 6, R-2). Only a log whose compile
//!   passed is bound to its saved state.
//! - `file.log.check`'s result names the file and the SHA-256 of the bytes it checked ([`CheckedFile`]), which
//!   `file.log.import` names as its `checked` argument (the `file_check` gate: the same checked file digest).

use c600_kernel::json::{self, Value};

use crate::receipt::Error;

/// `file.document.new {kind}`: a new untitled document, a log or a script.
pub const DOCUMENT_NEW_ROW: &str = "file.document.new";
/// `file.document.open {file}`: a file read into a tab.
pub const DOCUMENT_OPEN_ROW: &str = "file.document.open";
/// `file.document.save {document}`.
pub const DOCUMENT_SAVE_ROW: &str = "file.document.save";
/// `file.document.save_as {document, destination}`.
pub const DOCUMENT_SAVE_AS_ROW: &str = "file.document.save_as";
/// `file.document.compile {document}`: a query job on the exclusive lane.
pub const DOCUMENT_COMPILE_ROW: &str = "file.document.compile";
/// `file.document.close {document, discard?}`.
pub const DOCUMENT_CLOSE_ROW: &str = "file.document.close";
/// `file.log.export {history, format, destination}`: a file job on the exclusive lane.
pub const LOG_EXPORT_ROW: &str = "file.log.export";
/// `file.log.check {file}`: a query job on the exclusive lane, the import's file check.
pub const LOG_CHECK_ROW: &str = "file.log.check";
/// `file.log.import {checked, token}`: the explicit import commit (ADR-D01.82).
pub const LOG_IMPORT_ROW: &str = "file.log.import";
/// `app.exit`: saves the live session without a prompt (ADR-D01.79).
pub const EXIT_ROW: &str = "app.exit";
/// The live session's document id: the first document, always open (ADR-D01.79).
pub const SESSION_DOCUMENT: &str = "session";
/// The one journal range `file.log.export` takes (`JournalRangeRef`; draft): the live session's whole journal, at its
/// cursors. A range inside it is not built.
pub const WHOLE_SESSION: &str = "session";
/// H-08's type of a document argument.
pub const DOCUMENT_REF: &str = "DocumentRef";

/// The rows this module's service answers, `app.exit` included.
pub const DOCUMENT_ROWS: [&str; 10] = [
    DOCUMENT_NEW_ROW,
    DOCUMENT_OPEN_ROW,
    DOCUMENT_SAVE_ROW,
    DOCUMENT_SAVE_AS_ROW,
    DOCUMENT_COMPILE_ROW,
    DOCUMENT_CLOSE_ROW,
    LOG_EXPORT_ROW,
    LOG_CHECK_ROW,
    LOG_IMPORT_ROW,
    EXIT_ROW,
];

/// Whether `row` is one of [`DOCUMENT_ROWS`]: no run invokes one (the documents are the solver's).
pub fn document_row(row: &str) -> bool {
    DOCUMENT_ROWS.contains(&row)
}

/// The two kinds of document (ADR-D01.79).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DocumentKind {
    Log,
    /// A `.6cs` script.
    Script,
}

impl DocumentKind {
    pub fn name(self) -> &'static str {
        match self {
            DocumentKind::Log => "log",
            DocumentKind::Script => "script",
        }
    }

    pub fn named(name: &str) -> Option<DocumentKind> {
        [DocumentKind::Log, DocumentKind::Script].into_iter().find(|kind| kind.name() == name)
    }
}

/// H-08's `LogFormat`: the 1.0 log, or 0.4's C600 proof log or an MPUlt v1 log as interchange (ADR-D01.84; draft).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LogFormat {
    /// The 1.0 log (`.c600log`).
    Log,
    /// 0.4's `C600-STUDIO-SESSION-v1`.
    C600,
    /// MPUltimate v1 of 600-cell-Full.
    Mpult,
}

impl LogFormat {
    pub const ALL: [LogFormat; 3] = [LogFormat::Log, LogFormat::C600, LogFormat::Mpult];

    pub fn name(self) -> &'static str {
        match self {
            LogFormat::Log => "c600log",
            LogFormat::C600 => "c600",
            LogFormat::Mpult => "mpult",
        }
    }

    pub fn named(name: &str) -> Option<LogFormat> {
        LogFormat::ALL.into_iter().find(|format| format.name() == name)
    }
}

/// H-08's `CheckedFileRef` (draft): `{"file": FileRef, "sha256": 64 hexadecimal digits}`, the file a passed
/// `file.log.check` read and the SHA-256 of the bytes it checked.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CheckedFile {
    pub file: String,
    pub sha256: String,
}

impl CheckedFile {
    pub fn from_json(value: &Value) -> Option<CheckedFile> {
        let fields = value.as_object().filter(|fields| fields.len() == 2)?;
        let file = fields.get("file")?.as_str().filter(|file| !file.is_empty())?;
        let sha256 = fields.get("sha256")?.as_str().filter(|digest| hex64(digest))?;
        Some(CheckedFile { file: file.to_owned(), sha256: sha256.to_owned() })
    }

    pub fn to_json(&self) -> Value {
        json::object([("file", Value::String(self.file.clone())), ("sha256", Value::String(self.sha256.clone()))])
    }
}

fn hex64(text: &str) -> bool {
    text.len() == 64 && text.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

/// What a passed check lets `file.log.import` commit (ADR-D01.82): the log's line from its start to its saved cursor,
/// which is the legal witness of its saved state, and what the layer checks before it commits. The service builds it
/// from a check of the file's current bytes only.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImportPlan {
    pub checked: CheckedFile,
    /// The KN0.1 `model_id` the log was written under.
    pub model: String,
    /// The label digest of the log's start state.
    pub start: String,
    /// The log's line from its start to its saved cursor, in the kernel's JSON steps.
    pub line: Value,
    /// The full-state hash at the log's saved cursor.
    pub state: String,
    /// The log's preferences as canonical JSON text, recorded and never applied (ADR-D01.80).
    pub preferences: Option<String>,
}

/// The host's document service (ADR-D01.79 to .84; draft). Every method answers the value the layer digests into its
/// receipt's result ([`crate::CommandLayer::document_result`]), or an error that the layer answers as a refusal
/// (`Refused`) for a row that runs no job and as a failure (`Failed`) for a job. A refusal changes nothing; a write is
/// whole or absent.
pub trait Documents: Send {
    /// `file.document.new`: a new untitled document, never saved, of `kind`.
    fn create(&mut self, kind: DocumentKind) -> Result<Value, Error>;
    /// `file.document.open`: reads `file` into a tab. It runs no script and binds a log to no state.
    fn open(&mut self, file: &str) -> Result<Value, Error>;
    /// `file.document.save`: writes the document's bytes to its file; the live session writes the session's log.
    fn save(&mut self, document: &str) -> Result<Value, Error>;
    /// `file.document.save_as`: writes the document's bytes to `destination`, which becomes its file.
    fn save_as(&mut self, document: &str, destination: &str) -> Result<Value, Error>;
    /// `file.document.compile`'s job: the compile result, bound to the SHA-256 of the bytes it compiled.
    fn compile(&mut self, document: &str) -> Result<Value, Error>;
    /// `file.document.close`. The live session does not close; a dirty document closes only with `discard`.
    fn close(&mut self, document: &str, discard: bool) -> Result<Value, Error>;
    /// `file.log.check`'s job: checks `file`'s current bytes, records the result for the import's gate, and answers
    /// it with the file and the SHA-256 it checked ([`CheckedFile`]) and whether it passed.
    fn check(&mut self, file: &str) -> Result<Value, Error>;
    /// `file.log.export`'s job: writes the live session in `format` to `destination`.
    fn export(&mut self, format: LogFormat, destination: &str) -> Result<Value, Error>;
    /// `file.log.import`'s file check: the plan of a passed check of `checked`, refused when there is none for that
    /// file and digest, when it did not pass (with its first problem's code), or when the file's bytes are no longer
    /// the checked ones (`PS1:input.digest_mismatch`). Reading it changes nothing.
    fn import_plan(&mut self, checked: &CheckedFile) -> Result<ImportPlan, Error>;
    /// Writes the recovery checkpoint of the live session before an import commits, with the current preferences;
    /// answers its name.
    fn checkpoint(&mut self) -> Result<String, Error>;
    /// `app.exit`: saves the live session, with every open document's contents, without a prompt.
    fn exit(&mut self) -> Result<Value, Error>;
}

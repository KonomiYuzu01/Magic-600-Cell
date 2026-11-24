//! The command layer's side of the document rows and the log rows (LP4-14; ADR-D01.79 to .84; draft). The data and the
//! host's service are in [`crate::documents`].
//!
//! - Without the host's document service every row here, `app.exit` included, is `KN5:input.row_not_built`, as it was
//!   before LP4-14, and writes nothing.
//! - `file.document.new`, `.open`, `.save`, `.save_as` and `.close` and `app.exit` are answered at once from the
//!   service: a transient `completed(app)` or `completed(file)` whose result digests the service's answer
//!   ([`CommandLayer::document_result`]). They take no action number and change no live state.
//! - `file.document.compile`, `file.log.check` and `file.log.export` queue a job on the exclusive lane, which
//!   [`CommandLayer::run_jobs`] runs through the service. A commit does not cancel it: it reads documents and files,
//!   not the state the commit replaced (LI.25's file jobs finish).
//! - `file.log.import` is the explicit commit (ADR-D01.82): its confirm gate and its file check come before the token
//!   is spent, then the commit steps run as `twist.apply`'s do. Before the commit the recovery checkpoint is written
//!   and every playback state is stopped; the commit is one retained move whose word is the legal witness from the
//!   head back to the session's start and along the log's line.

use std::collections::BTreeMap;

use c600_kernel::json::Value;
use c600_kernel::Derivation;

use super::{kernel_error, refused, steps_json, Caller, CommandLayer, Invocation, Job, JobKind, Key, Meta};
use crate::documents::{
    CheckedFile, DocumentKind, Documents, LogFormat, DOCUMENT_CLOSE_ROW, DOCUMENT_COMPILE_ROW, DOCUMENT_NEW_ROW,
    DOCUMENT_OPEN_ROW, DOCUMENT_SAVE_AS_ROW, DOCUMENT_SAVE_ROW, EXIT_ROW, LOG_CHECK_ROW, LOG_EXPORT_ROW,
    LOG_IMPORT_ROW,
};
use crate::forms;
use crate::receipt::{code, digest, Completion, CompletionKind, Error, ResultRef, Status};
use crate::store::Store;
use crate::table::{Lane, Row};

/// No proved path joins the session's head and the log's saved state (chapter 13: a target branch lacks a proved
/// journal navigation path): the log does not start at this session's start, or the line from the head back to the start
/// crosses a node that holds no retained word.
const BRANCH_PATH_UNPROVED: &str = "KN4:input.branch_path_unproved";

/// The layer's document state: the host's service, the answers its rows gave and the last import's checkpoint.
#[derive(Default)]
pub(super) struct Docs {
    service: Option<Box<dyn Documents>>,
    /// Each answer by the digest its receipt's result names (draft): transient, as a comparison is.
    results: BTreeMap<String, Value>,
    /// The recovery checkpoint the last import wrote before it committed (ADR-D01.82).
    checkpoint: Option<String>,
}

fn text<'a>(args: &'a BTreeMap<String, Value>, name: &str) -> &'a str {
    args.get(name).and_then(Value::as_str).unwrap_or_default()
}

/// Whether `row` runs as a job on the exclusive lane.
pub(super) fn document_job(row: &str) -> bool {
    matches!(row, DOCUMENT_COMPILE_ROW | LOG_CHECK_ROW | LOG_EXPORT_ROW)
}

impl<S: Store> CommandLayer<S> {
    // ----- the host's interface -----

    /// Installs the host's document service (ADR-D01.79 to .84; LP4-14). Without one, the document rows, the log rows
    /// and `app.exit` are refused with `KN5:input.row_not_built`.
    pub fn set_documents(&mut self, documents: Box<dyn Documents>) {
        self.docs.service = Some(documents);
        self.generation += 1;
    }

    /// The service's answer that a document or log row's result digest names, while this layer runs (draft).
    pub fn document_result(&self, digest: &str) -> Option<&Value> {
        self.docs.results.get(digest)
    }

    /// The recovery checkpoint the last `file.log.import` wrote before it committed (ADR-D01.82), while this layer runs.
    pub fn import_checkpoint(&self) -> Option<&str> {
        self.docs.checkpoint.as_deref()
    }

    // ----- availability and admission -----

    /// The argument-independent gates of a document or log row: the host's service, the exclusive lane for a job, and
    /// for the import the commit lane and Jumble off.
    pub(super) fn document_gates(&self, row: &Row) -> Result<(), Error> {
        if self.docs.service.is_none() {
            return Err(Error::new(code::ROW_NOT_BUILT, format!("`{}` needs the host's document service", row.id)));
        }
        if document_job(&row.id) {
            return self.exclusive_free();
        }
        if row.id == LOG_IMPORT_ROW {
            if self.commit_held {
                return Err(Error::new(code::COMMIT_IN_FLIGHT, "another commit is in flight"));
            }
            return self.retained_mode();
        }
        Ok(())
    }

    /// Admits a document or log row. Refused, writing nothing, in this order: no service (`KN5:input.row_not_built`),
    /// an argument of another form (`KN5:input.argument_form`), a held exclusive lane for a job (`KN7:busy.lane_full`),
    /// and the service's own refusal.
    pub(super) fn document_row(
        &mut self,
        caller: &Caller,
        invocation: &Invocation,
        request: &str,
        row: &Row,
        args: &BTreeMap<String, Value>,
    ) -> Result<(Option<u64>, Status), Error> {
        if self.docs.service.is_none() {
            return refused(code::ROW_NOT_BUILT, format!("`{}` needs the host's document service", row.id));
        }
        forms::check_forms(row, args)?;
        if row.id == LOG_IMPORT_ROW {
            return self.import(caller, invocation, request);
        }
        if document_job(&row.id) {
            self.exclusive_free()?;
            let id = self.next_job();
            let kind = JobKind::Document { row: row.id.clone(), args: invocation.args.clone() };
            let (command, scope) = (invocation.command.clone(), caller.scope());
            self.jobs.push_back(Job { id, command, scope, lane: Lane::Exclusive, kind });
            self.generation += 1;
            return Ok((Some(id), Status::Running(id)));
        }
        let service = self.docs.service.as_mut().expect("checked above");
        let (kind, answer) = match row.id.as_str() {
            DOCUMENT_NEW_ROW => {
                let Some(kind) = DocumentKind::named(text(args, "kind")) else {
                    return refused(code::ARGUMENT_FORM, "`kind` is `log` or `script`");
                };
                (CompletionKind::App, service.create(kind)?)
            }
            DOCUMENT_OPEN_ROW => (CompletionKind::File, service.open(text(args, "file"))?),
            DOCUMENT_SAVE_ROW => (CompletionKind::File, service.save(text(args, "document"))?),
            DOCUMENT_SAVE_AS_ROW => {
                (CompletionKind::File, service.save_as(text(args, "document"), text(args, "destination"))?)
            }
            DOCUMENT_CLOSE_ROW => {
                let discard = args.get("discard") == Some(&Value::Bool(true));
                (CompletionKind::App, service.close(text(args, "document"), discard)?)
            }
            EXIT_ROW => (CompletionKind::App, service.exit()?),
            other => return refused(code::ROW_NOT_BUILT, format!("`{other}` is not a document row")),
        };
        Ok((None, self.document_answer(kind, None, answer)))
    }

    /// A completion whose result digests `answer`, which the layer keeps for [`CommandLayer::document_result`].
    fn document_answer(&mut self, kind: CompletionKind, job: Option<u64>, answer: Value) -> Status {
        let result = ResultRef { job, digest: digest(&answer) };
        self.docs.results.insert(result.digest.clone(), answer);
        Status::Completed(Completion { kind, revision: self.revision.clone(), result: Some(result) })
    }

    /// Runs a document job and settles its receipt: `completed(query)` for a compile or a check, `completed(file)` for
    /// an export, or `failed` with the service's error.
    pub(super) fn run_document_job(&mut self, id: u64, key: &Key, row: &str, args: &Value) {
        let field = |name: &str| args.get(name).and_then(Value::as_str).unwrap_or_default().to_owned();
        let Some(service) = self.docs.service.as_mut() else {
            let error = Error::new(code::ROW_NOT_BUILT, format!("`{row}` needs the host's document service"));
            self.settle(key, Status::Failed(error));
            return;
        };
        let (kind, answer) = match row {
            DOCUMENT_COMPILE_ROW => (CompletionKind::Query, service.compile(&field("document"))),
            LOG_CHECK_ROW => (CompletionKind::Query, service.check(&field("file"))),
            _ => match LogFormat::named(&field("format")) {
                Some(format) => (CompletionKind::File, service.export(format, &field("destination"))),
                None => (CompletionKind::File, Err(Error::new(code::ARGUMENT_FORM, "`format` is a LogFormat"))),
            },
        };
        let status = match answer {
            Ok(answer) => self.document_answer(kind, Some(id), answer),
            Err(error) => Status::Failed(error),
        };
        self.settle(key, status);
        self.generation += 1;
    }

    // ----- file.log.import -----

    /// `file.log.import {checked, token}` (ADR-D01.82; draft): makes a checked log the current session as one
    /// explicit commit.
    ///
    /// Before the token is spent: the solver's confirmation, `confirm: true` (`KN5:input.confirmation_required`), then
    /// the file check: the service's plan of a passed `file.log.check` of `checked`, the same checked file digest
    /// (refused with the service's code: no such check, a check that did not pass, or bytes that changed since). Then
    /// the commit steps run as `twist.apply`'s do: the token, the commit lane, its binding, Jumble off, the state and
    /// protection revisions, the head hash; the log's model identity (`KN1:input.identity_mismatch`); a proved path
    /// from the head to the log's state (`KN4:input.branch_path_unproved` when the log does not start at this session's
    /// start, or the line back to the start crosses a node that holds no retained word); and the kernel's derivation of
    /// that word under the protected set and the policy in force (`KN2:protection.orbit_would_move`), which must reach
    /// the log's saved state. A refusal changes nothing.
    ///
    /// Then, in this order: the recovery checkpoint of the live session ([`Documents::checkpoint`]); every playback
    /// state stopped with `KN7:cancelled.revision_replaced` and its action map emptied; the durable transaction. A write
    /// that fails answers `failed` and commits nothing; the checkpoint and the stopped states stay, which loses
    /// nothing. The transaction is FP.31's `retained_move` under this row, so the journal and the migration format do
    /// not change, `session.undo` returns to the state before the import, and a reopen replays it as any commit. The
    /// log's preferences are not applied: the layer holds none, and the service keeps the current ones.
    fn import(
        &mut self,
        caller: &Caller,
        invocation: &Invocation,
        request: &str,
    ) -> Result<(Option<u64>, Status), Error> {
        let Some(checked) = invocation.args.get("checked").and_then(CheckedFile::from_json) else {
            return refused(code::ARGUMENT_FORM, "`checked` is a CheckedFileRef `{\"file\", \"sha256\"}`");
        };
        if invocation.args.get("confirm") != Some(&Value::Bool(true)) {
            return refused(code::CONFIRMATION_REQUIRED, "an import needs the solver's confirmation (ADR-D01.82)");
        }
        let service = self.docs.service.as_mut().expect("checked at admission");
        let plan = service.import_plan(&checked)?;
        if plan.checked != checked {
            return refused(code::STATE_HASH_MISMATCH, "the service's plan is for another checked file");
        }
        let binding = self.authenticate(invocation)?;
        self.commit_lane()?;
        self.bound(&binding, invocation)?;
        self.retained_mode()?;
        if binding.revision != Some((self.revision.state, self.revision.protection)) {
            return refused(code::REVISION_CHANGED, "the token's state or protection revision changed");
        }
        if self.current.digest() != self.nodes[&self.head].digest {
            return refused(code::STATE_HASH_MISMATCH, "the head hash differs from the head's recorded state");
        }
        if plan.model != self.kernel.identity().model_id {
            return refused(c600_kernel::codes::IDENTITY_MISMATCH, "the log was written under another model");
        }
        if plan.start != self.nodes[&0].digest {
            return refused(BRANCH_PATH_UNPROVED, "the log does not start at this session's start state");
        }
        let mut steps = match self.head {
            0 => Vec::new(),
            _ => steps_json(&self.path(0).map_err(|e| Error::new(BRANCH_PATH_UNPROVED, e.detail))?, false),
        };
        match &plan.line {
            Value::Array(line) => steps.extend(line.iter().cloned()),
            _ => return refused(code::ARGUMENT_FORM, "the plan's line is a list of steps"),
        }
        let recipe = self.kernel.recipe(&Value::Array(steps))?;
        let derived = match self.derive(&self.current, &recipe, &self.protected).map_err(kernel_error)? {
            Derivation::Derived(derived) => derived,
            Derivation::Refused(refusal) => return Err(refusal.refusal.into()),
        };
        if derived.state.digest() != plan.state {
            return refused(code::STATE_HASH_MISMATCH, "the log's line does not reach its saved state");
        }
        let service = self.docs.service.as_mut().expect("checked at admission");
        let checkpoint = match service.checkpoint() {
            Ok(name) => name,
            Err(error) => return Ok((None, Status::Failed(error))),
        };
        self.docs.checkpoint = Some(checkpoint);
        if let Err(error) = self.stop_playbacks_for_import() {
            return Ok((None, Status::Failed(error)));
        }
        let meta = Meta::of(caller, invocation, request);
        let adoption = self.commit_adoption(meta, recipe, derived.state, derived.word, None);
        self.transact(adoption)
    }
}

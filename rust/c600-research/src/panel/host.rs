//! The research host that serves the panel's rows: it takes each row's program job from the command layer, answers it
//! from its own data and reports the output, and it gives the panel a row's answer with charter D2's state.

use std::collections::{BTreeMap, BTreeSet};

use c600_archive::bindings::{
    Availability as Preflight, Bindings, CheckedAs, FilePin, Host as PreflightHost, InputState, Reason,
};
use c600_archive::bundle::export_check_bundle;
use c600_archive::import::{import_archive, GrantedFile, GrantedFiles, ImportReport};
use c600_archive::{Discriminator, ImportTable, ImportTableRef, Limits, LimitsRef};
use c600_checkers::engines::Loaded;
use c600_command::{
    code, CancelReason, CommandLayer, Error, JobOutput, Overlay, Profile, ProgramJob, Receipt, RecordValue, Revision,
    Status, Store,
};
use c600_evidence::obligation::{translate, SymbolTable};
use c600_evidence::{Statement, StatementId};
use c600_session::persist::codec::{canonical, Json};
use c600_session::persist::record::identity_json;
use c600_session::persist::run::{codes as run_codes, Reproduction};

use super::files::Grant;
use super::rows::{self, MODULE, ROWS};
use super::wire::{
    arg_map, arg_opt_text, arg_sha, arg_text, arg_texts, int, json_of, list, object, revision_json, sha, utf8,
};
use crate::job::{codes as job_codes, Budget, Cancel, Check, CheckJob, CheckRun, Granted, Pinned};
use crate::record::{kind_from, provenance};

/// A published import: its report, which holds the granted members' bytes.
pub(super) struct Import {
    pub(super) report: ImportReport,
}

/// A check run the host made, with the claim group it was asked for.
pub(super) struct RunEntry {
    /// The SHA-256 of the run record's bytes, the run's name on the panel.
    pub(super) sha: String,
    pub(super) group: Option<String>,
    pub(super) run: CheckRun,
}

/// A reproduction of a run the host made.
pub(super) struct ReproEntry {
    pub(super) run: String,
    pub(super) answer: Reproduction,
}

/// An export the host wrote, or refused.
pub(super) struct ExportEntry {
    pub(super) row: &'static str,
    /// The exported statement's SHA-256, or the bundle's record names.
    pub(super) subject: Json,
    pub(super) destination: String,
    /// The SHA-256 of the bytes written, or the refusal's code.
    pub(super) result: Result<String, &'static str>,
}

/// A row's answer as a job reported it.
struct Stored {
    research: String,
    value: Json,
    digest: String,
}

/// What a row's job did: its answer, and the workspace records it writes (a workspace row's only).
pub(super) struct Done {
    pub(super) answer: Json,
    pub(super) records: Vec<(String, Json)>,
}

impl Done {
    pub(super) fn answer(answer: Json) -> Done {
        Done { answer, records: Vec::new() }
    }
}

/// Charter D2's state of a row's answer on the panel (RPI.13; ONE.12's stale and pending displays).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum D2 {
    /// The answer was computed at the head revision from the research data the host holds now.
    Current,
    /// As `Current`, with nothing to list.
    Empty,
    /// The row's job is queued or running.
    Loading { job: Option<u64> },
    /// The row was refused because its lane or the session was busy.
    Busy { code: &'static str },
    /// The answer was computed at revision `at`, which is no longer the head, or from research data the host has
    /// changed since (`research_moved`). A job a commit cancelled before it answered is stale with no answer.
    Stale { at: Option<Revision>, research_moved: bool },
    /// The row was refused or failed with `code`, or its answer is gone (`KN5:stale.result_gone`).
    Error { code: &'static str },
}

/// A row's answer as the panel reads it: the receipt's status name, its D2 state, and the answer the host reported,
/// `{row, revision, research, answer}`, when the row completed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Read {
    pub status: &'static str,
    pub state: D2,
    pub answer: Option<Json>,
}

/// The research host of the panel's rows.
pub struct Host<'a> {
    pub(super) loaded: &'a Loaded,
    pub(super) grant: Grant,
    pub(super) bindings: Bindings,
    pub(super) table: ImportTable,
    pub(super) imports: Vec<Import>,
    pub(super) runs: Vec<RunEntry>,
    pub(super) reproductions: Vec<ReproEntry>,
    pub(super) exports: Vec<ExportEntry>,
    answers: BTreeMap<u64, Stored>,
    cancel: Cancel,
}

/// The lengths of the host's lists before a job, to drop what a job that did not complete added. Exports are not
/// among them: a file written or refused is on record whatever the receipt.
struct Mark {
    imports: usize,
    runs: usize,
    reproductions: usize,
}

impl<'a> Host<'a> {
    /// A host with the owned engines and the user's file grant, holding no import, run or export yet; its claim
    /// bindings are `c600-archive`'s edition 1.
    pub fn new(loaded: &'a Loaded, grant: Grant) -> Host<'a> {
        Host {
            loaded,
            grant,
            bindings: Bindings::edition_1(),
            table: ImportTable::edition_1(),
            imports: Vec::new(),
            runs: Vec::new(),
            reproductions: Vec::new(),
            exports: Vec::new(),
            answers: BTreeMap::new(),
            cancel: Cancel::new(),
        }
    }

    /// Serves the rows of [`ROWS`] on `layer`, each with the entry of its own ID in [`MODULE`]. A session of another
    /// profile than `research` is refused with `LG3:capability.grant_mismatch` (draft), and a table without one of
    /// the rows with `KN5:input.unknown_row`; either refusal serves nothing.
    pub fn serve<S: Store>(&self, layer: &mut CommandLayer<S>) -> Result<(), Error> {
        if layer.profile() != Profile::Research {
            let detail = "the research panel's rows are served only in a session of the research profile";
            return Err(Error::new(run_codes::GRANT_MISMATCH, detail));
        }
        if let Some(row) = ROWS.iter().find(|row| layer.table().row(row.id).is_none()) {
            return Err(Error::new(code::UNKNOWN_ROW, format!("the session's table has no row `{}`", row.id)));
        }
        for row in &ROWS {
            layer.serve_row(row.id, Overlay { module: MODULE.to_owned(), entry: row.id.to_owned() })?;
        }
        Ok(())
    }

    /// A handle that stops the check or reproduction the host runs next or is running, at its next charge (KN7.6).
    /// After any job that ran while the handle was set, the host takes a fresh one: a stop set while a read ran stops
    /// nothing, and a handle taken before that is spent.
    pub fn stopper(&self) -> Cancel {
        self.cancel.clone()
    }

    /// The SHA-256 of the host's research data: its grant, the imports it published, the runs and reproductions it
    /// made and the exports it wrote or refused. An answer made from other research data is stale.
    pub fn research(&self) -> String {
        let imports = self.imports.iter().map(|import| {
            let members = import.report.members.iter().filter(|m| m.present).map(|m| Json::text(&m.name));
            object(vec![("manifest", Json::text(&import.report.manifest_sha256)), ("members", list(members))])
        });
        let reproductions = self.reproductions.iter().map(|entry| {
            object(vec![("run", Json::text(&entry.run)), ("answer", super::answers::reproduction_json(&entry.answer))])
        });
        let exports = self.exports.iter().map(super::answers::export_json);
        sha(&object(vec![
            ("grant", self.grant.json()),
            ("imports", list(imports)),
            ("runs", list(self.runs.iter().map(|entry| Json::text(&entry.sha)))),
            ("reproductions", list(reproductions)),
            ("exports", list(exports)),
        ]))
    }

    /// Takes every queued program job from `layer` and runs those of [`MODULE`]; the jobs of other entries are
    /// returned untouched, for their own host to run.
    pub fn run_pending<S: Store>(&mut self, layer: &mut CommandLayer<S>) -> Vec<ProgramJob> {
        let mut foreign = Vec::new();
        while let Some(job) = layer.take_program_job() {
            if let Err(job) = self.run(layer, job) {
                foreign.push(job);
            }
        }
        foreign
    }

    /// Runs one program job of [`MODULE`] and reports its output to `layer`; a job of another entry is given back.
    /// The answer is `{row, revision, research, answer}`, and a workspace row's records are written in its
    /// completion. An import, run or reproduction a job added to the host's research data is kept only when the layer
    /// completed the job; an export stays on record as written or refused whatever the receipt, since its file was
    /// written or not.
    pub fn run<S: Store>(
        &mut self,
        layer: &mut CommandLayer<S>,
        job: ProgramJob,
    ) -> Result<Option<Receipt>, ProgramJob> {
        if job.overlay.module != MODULE {
            return Err(job);
        }
        let mark = Mark { imports: self.imports.len(), runs: self.runs.len(), reproductions: self.reproductions.len() };
        let args = json_of(&job.args.canonical()).unwrap_or(Json::Null);
        let done = self.dispatch(&job, &args);
        let research = self.research();
        let mut kept: Option<(Json, String)> = None;
        let output = done.and_then(|done| {
            let value = object(vec![
                ("row", Json::text(&job.row)),
                ("revision", revision_json(&job.revision)),
                ("research", Json::text(&research)),
                ("answer", done.answer),
            ]);
            let mut records = BTreeMap::new();
            let block = String::from_utf8(canonical(&provenance())).unwrap_or_default();
            for (name, payload) in done.records {
                records.insert(name, RecordValue { payload: kernel_value!(payload)?, provenance: block.clone() });
            }
            let output = JobOutput { value: kernel_value!(value)?, views: Vec::new(), records };
            kept = Some((value, output.digest()));
            Ok(output)
        });
        let receipt = layer.finish_program_job_with(job.job, output);
        match (receipt.as_ref().map(|receipt| &receipt.status), kept) {
            (Some(Status::Completed(_)), Some((value, digest))) => {
                self.answers.insert(job.job, Stored { research, value, digest });
            }
            _ => self.rollback(&mark),
        }
        if self.cancel.is_cancelled() {
            self.cancel = Cancel::new();
        }
        Ok(receipt)
    }

    fn rollback(&mut self, mark: &Mark) {
        self.imports.truncate(mark.imports);
        self.runs.truncate(mark.runs);
        self.reproductions.truncate(mark.reproductions);
    }

    fn dispatch(&mut self, job: &ProgramJob, args: &Json) -> Result<Done, Error> {
        match job.row.as_str() {
            rows::INVENTORY => self.inventory(args).map(Done::answer),
            rows::CLAIMS => self.claims(args).map(Done::answer),
            rows::REPORT => self.report(args).map(Done::answer),
            rows::WORD => self.word(args).map(Done::answer),
            rows::RUNS => self.runs_answer(args).map(Done::answer),
            rows::OBLIGATIONS => self.obligations(args).map(Done::answer),
            rows::IMPORT => self.import(args),
            rows::CHECK => self.check(args, job),
            rows::REPRODUCE => self.reproduce(args),
            rows::BUNDLE => self.bundle(args),
            rows::OBLIGATION => self.obligation(args),
            other => Err(Error::new(code::ROW_NOT_BUILT, format!("the research host has no entry for `{other}`"))),
        }
    }

    /// A row's answer as the panel reads it: `receipt`'s current outcome in `layer` (the shell's outcome lookup),
    /// with the answer the host reported for it. A completed answer is current while its revision is the head and the
    /// research data is unchanged; a result reference whose digest is not the host's answer is gone.
    pub fn read<S: Store>(&self, layer: &CommandLayer<S>, receipt: &Receipt) -> Read {
        let now = layer.outcome(&receipt.command, &receipt.request).unwrap_or_else(|| receipt.clone());
        let status = now.status.name();
        let (state, answer) = match &now.status {
            Status::Accepted | Status::Running(_) => (D2::Loading { job: now.job }, None),
            Status::Completed(completion) => {
                let stored = completion
                    .result
                    .as_ref()
                    .and_then(|result| Some((result, self.answers.get(&result.job?)?)))
                    .filter(|(result, stored)| stored.digest == result.digest);
                match stored {
                    None => (D2::Error { code: code::RESULT_GONE }, None),
                    Some((_, stored)) => {
                        let research_moved = stored.research != self.research();
                        let state = if completion.revision != *layer.revision() || research_moved {
                            D2::Stale { at: Some(completion.revision.clone()), research_moved }
                        } else if empty(stored.value.get("answer")) {
                            D2::Empty
                        } else {
                            D2::Current
                        };
                        (state, Some(stored.value.clone()))
                    }
                }
            }
            Status::Refused(error) | Status::Failed(error) if error.code.contains(":busy.") => {
                (D2::Busy { code: error.code }, None)
            }
            Status::Refused(error) | Status::Failed(error) => (D2::Error { code: error.code }, None),
            Status::Cancelled(CancelReason::ReplacedRevision | CancelReason::Superseded(_)) => {
                (D2::Stale { at: None, research_moved: false }, None)
            }
            Status::Cancelled(CancelReason::ByUser) => (D2::Error { code: code::BY_USER }, None),
            Status::Cancelled(CancelReason::Budget) => (D2::Error { code: code::BUDGET_CEILING }, None),
            Status::Committed(_) | Status::UnknownOutcome => (D2::Error { code: code::RESULT_GONE }, None),
        };
        Read { status, state, answer }
    }

    // ------------------------------------------------------------------------------------------------ resolution

    /// A certificate's or input's bytes by name, with the digest they are stated to have: a granted member of the
    /// newest published import that holds it, stated by its corpus manifest, or else a granted file, stated by its
    /// own bytes.
    fn resolve(&self, name: &str) -> Result<(Vec<u8>, String), Error> {
        for import in self.imports.iter().rev() {
            let report = &import.report;
            let Some(i) = report.member(name) else { continue };
            if let Some(bytes) = report.stored_bytes(i) {
                return Ok((bytes.to_vec(), report.members[i].listed.stored_sha256.clone()));
            }
        }
        let bytes = self.grant.bytes(name)?;
        Ok((bytes.to_vec(), c600_digest::sha256_hex(bytes)))
    }

    /// The bytes of a stored file the claim bindings pin, when the host holds them with the pinned SHA-256: a granted
    /// file or a granted member of a published import.
    pub(super) fn held(&self, pin: &FilePin) -> Option<&[u8]> {
        if self.grant.digest(&pin.path) == Some(pin.sha256.as_str()) {
            return self.grant.bytes(&pin.path).ok();
        }
        self.imports.iter().rev().find_map(|import| {
            let report = &import.report;
            let i = report.member(&pin.path)?;
            (report.members[i].stored_sha256.as_deref() == Some(pin.sha256.as_str()))
                .then(|| report.stored_bytes(i))
                .flatten()
        })
    }

    /// LB5.14 before a group's check: the group's preflight must be `ready`, the checker one of its claim checkers,
    /// and the certificate the group's record (its `via` record when the record is the check's input or table).
    fn resolve_claim(&self, group: &str, checker: &str, certificate: &str) -> Result<(), Error> {
        let row = self.bindings.find(group).ok_or_else(|| {
            Error::new(code::ARGUMENT_FORM, format!("`{group}` is no claim group of the claim bindings"))
        })?;
        if let Preflight::Unavailable { reasons, .. } = self.bindings.availability(row, self) {
            let words: Vec<&str> = reasons.iter().map(Reason::word).collect();
            let code = match reasons.first() {
                Some(Reason::DependencyMissing) => code::NOT_GRANTED,
                Some(Reason::DependencyMismatch) => c600_archive::DIGEST_MISMATCH,
                _ => job_codes::NO_CHECKER,
            };
            return Err(Error::new(code, format!("`{group}`'s preflight is unavailable: {}", words.join(", "))));
        }
        if !row.claim_checkers().iter().any(|id| id == checker) {
            return Err(Error::new(job_codes::NO_CHECKER, format!("`{checker}` checks no claim of `{group}`")));
        }
        let record = match row.checked_as {
            Some(CheckedAs::Certificate) | None => row.source,
            Some(_) => row.via.unwrap_or(row.source),
        };
        if self.bindings.file(record).sha256 != certificate {
            return Err(Error::new(code::ARGUMENT_FORM, format!("the certificate is not `{group}`'s record")));
        }
        Ok(())
    }

    // ------------------------------------------------------------------------------------------------ job rows

    /// `research.archive.import`: the granted manifest and members through `c600-archive`'s import; an import with a
    /// failed integrity check publishes nothing.
    fn import(&mut self, args: &Json) -> Result<Done, Error> {
        let manifest = arg_text(args, "manifest")?;
        let manifest = GrantedFile { name: manifest.to_owned(), bytes: self.grant.bytes(manifest)?.to_vec() };
        let mut members = Vec::new();
        for name in arg_texts(args, "members")? {
            let bytes = self.grant.bytes(&name)?.to_vec();
            members.push(GrantedFile { name, bytes });
        }
        let discriminator = Discriminator::edition_1();
        let report = import_archive(
            GrantedFiles { manifest, members },
            ImportTableRef { table: &self.table, discriminator: &discriminator },
            LimitsRef(&Limits::research_archive()),
        )
        .map_err(|error| Error::new(error.code, error.detail))?;
        if let Some(code) = report.refusal() {
            let failed: Vec<String> = (0..report.members.len()).flat_map(|i| report.failed_checks(i)).collect();
            return Err(Error::new(code, format!("the import failed integrity and publishes nothing: {failed:?}")));
        }
        let totals = report.totals().into_iter().map(|(name, n)| (name.to_owned(), int(n))).collect();
        let answer = object(vec![
            (
                "manifest",
                object(vec![
                    ("name", Json::text(&report.manifest_name)),
                    ("sha256", Json::text(&report.manifest_sha256)),
                ]),
            ),
            (
                "table",
                object(vec![
                    ("edition", Json::text(&report.table_edition)),
                    ("sha256", Json::text(&report.table_sha256)),
                ]),
            ),
            ("totals", Json::Object(totals)),
        ]);
        self.imports.push(Import { report });
        Ok(Done::answer(answer))
    }

    /// `research.check.run`: one KN6 check through the job adapter at the stated input; its run record is the row's
    /// workspace record.
    fn check(&mut self, args: &Json, job: &ProgramJob) -> Result<Done, Error> {
        let checker = arg_text(args, "checker")?;
        let kind = kind_from(checker, None)
            .or_else(|| kind_from("archive", Some(checker)))
            .ok_or_else(|| Error::new(job_codes::NO_CHECKER, format!("no checker `{checker}` is registered")))?;
        let (certificate, stated) = self.resolve(arg_text(args, "certificate")?)?;
        let mut inputs = Vec::new();
        for (role, name) in arg_map(args, "inputs")? {
            inputs.push(Granted { role, bytes: self.resolve(&name)?.0 });
        }
        let group = arg_opt_text(args, "group")?.map(str::to_owned);
        if let Some(group) = &group {
            self.resolve_claim(group, checker, &c600_digest::sha256_hex(&certificate))?;
        }
        let identity = match arg_text(args, "puzzle")? {
            "full" => self.loaded.full_identity(),
            "jumbling" => {
                let menu = inputs.iter().find(|input| input.role == "menu").ok_or_else(|| {
                    Error::new(code::ARGUMENT_FORM, "a check of the jumbling puzzle names its `menu` input")
                })?;
                self.loaded
                    .jumbling_identity(&menu.bytes)
                    .map_err(|refusal| Error::new(refusal.code, refusal.message))?
            }
            other => {
                return Err(Error::new(code::ARGUMENT_FORM, format!("`puzzle` is `{other}`, not full or jumbling")))
            }
        };
        let pinned = Pinned::stated(arg_text(args, "level")?, arg_sha(args, "digest")?, identity_json(&identity));
        let b = job.budget;
        let check = CheckJob {
            checks: vec![Check { kind, certificate, stated, inputs, cap: None }],
            input: pinned,
            budget: Budget::new(b.steps, b.kernel_cost, b.memory, b.output),
        };
        let refusal = |refusal: c600_checkers::Refusal| Error::new(refusal.code, refusal.message);
        let run = check.admit().map_err(refusal)?.run(&self.loaded.engines(), &self.cancel).map_err(refusal)?;
        let sha = c600_digest::sha256_hex(run.bytes());
        let record = format!("research.run/{sha}");
        let payload = object(vec![
            ("record", utf8(run.bytes())),
            ("reports", list(run.outputs().iter().map(|bytes| utf8(bytes)))),
        ]);
        let entry = RunEntry { sha, group, run };
        let answer = super::answers::run_json(self, &entry);
        self.runs.push(entry);
        Ok(Done { answer, records: vec![(record, payload)] })
    }

    /// `research.run.reproduce`: DET.7's reproduce of a run the host holds; its answer is the row's workspace record.
    fn reproduce(&mut self, args: &Json) -> Result<Done, Error> {
        let run = arg_sha(args, "run")?;
        let entry =
            self.runs.iter().find(|entry| entry.sha == run).ok_or_else(|| {
                Error::new(code::NAMED_OBJECT_INVALID, format!("the research host holds no run `{run}`"))
            })?;
        let again =
            crate::record::reproduce(&self.loaded.engines(), entry.run.bytes(), &entry.run.available(), &self.cancel);
        let answer =
            object(vec![("run", Json::text(run)), ("answer", super::answers::reproduction_json(&again.answer))]);
        self.reproductions.push(ReproEntry { run: run.to_owned(), answer: again.answer });
        Ok(Done { answer: answer.clone(), records: vec![(format!("research.reproduction/{run}"), answer)] })
    }

    /// `research.bundle.export`: PS3.17's check bundle of records of published imports, written to a granted file.
    fn bundle(&mut self, args: &Json) -> Result<Done, Error> {
        let destination = arg_text(args, "destination")?;
        self.grant.writable(destination)?;
        let names = arg_texts(args, "records")?;
        let mut envelopes = Vec::new();
        for name in &names {
            let report = self
                .imports
                .iter()
                .rev()
                .map(|import| &import.report)
                .find(|report| report.member(name).and_then(|i| report.stored_bytes(i)).is_some())
                .ok_or_else(|| {
                    Error::new(
                        code::NAMED_OBJECT_INVALID,
                        format!("`{name}` is no granted record of a published import"),
                    )
                })?;
            let mut envelope = report.envelope(name).ok_or_else(|| {
                Error::new(code::PERSISTENCE_FORMAT, format!("`{name}` is not a record with a format or none"))
            })?;
            let mut seen: BTreeSet<String> = BTreeSet::from([name.clone()]);
            for row in self.bindings.rows().iter().filter(|row| self.bindings.file(row.source).path == *name) {
                for (file, role) in self.bindings.stored(row) {
                    let pin = self.bindings.file(file);
                    if seen.insert(pin.path.clone()) {
                        let bytes = self.held(pin).map(<[u8]>::to_vec);
                        envelope = envelope.with_dependency(&pin.path, Some(&pin.sha256), bytes, role);
                    }
                }
                for missing in &row.missing {
                    if seen.insert(missing.name.clone()) {
                        envelope = envelope.with_dependency(&missing.name, None, None, &missing.name);
                    }
                }
            }
            envelopes.push(envelope);
        }
        let identity = kernel_value!(json_of(&self.bindings.identity).unwrap_or(Json::Null))?;
        let bundle = export_check_bundle(&envelopes, &identity, LimitsRef(&Limits::research_archive()))
            .map_err(|error| Error::new(error.code, error.detail))?;
        self.grant.put(destination, &bundle.zip)?;
        let subject = list(names.iter().map(|name| Json::text(name)));
        self.exports.push(ExportEntry {
            row: rows::BUNDLE,
            subject: subject.clone(),
            destination: destination.to_owned(),
            result: Ok(bundle.sha256.clone()),
        });
        let answer = object(vec![
            ("destination", Json::text(destination)),
            ("sha256", Json::text(&bundle.sha256)),
            ("bytes", int(bundle.zip.len())),
            ("records", subject),
            ("manifest", json_of(&bundle.manifest.canonical()).unwrap_or(Json::Null)),
        ]);
        Ok(Done::answer(answer))
    }

    /// `research.obligation.export`: LB5.18's export of a typed statement through `c600-evidence`'s translator,
    /// written to a granted file; a refusal writes nothing and stays on record.
    fn obligation(&mut self, args: &Json) -> Result<Done, Error> {
        let destination = arg_text(args, "destination")?;
        self.grant.writable(destination)?;
        let statement = arg_sha(args, "statement")?;
        // The body is a granted file with the statement's SHA-256. A claim group's statement content names a record's
        // claim by its pointers and is no body in the obligation vocabulary, so it is never passed as one.
        let body = self.grant.by_digest(statement);
        let translated = SymbolTable::draft_1()
            .and_then(|table| translate(&Statement::new(StatementId::typed(statement)), body, &table));
        let mut entry = ExportEntry {
            row: rows::OBLIGATION,
            subject: Json::text(statement),
            destination: destination.to_owned(),
            result: Err(code::NOT_GRANTED),
        };
        let obligation = match translated {
            Ok(obligation) => obligation,
            Err(error) => {
                entry.result = Err(error.code);
                self.exports.push(entry);
                return Err(Error::new(error.code, error.detail));
            }
        };
        self.grant.put(destination, obligation.source.as_bytes())?;
        entry.result = Ok(obligation.sha256.clone());
        self.exports.push(entry);
        let pins = object(vec![
            ("statement", Json::text(&obligation.pins.statement)),
            ("symbols", Json::text(&obligation.pins.symbols)),
            ("prelude", super::wire::opt_text(obligation.pins.prelude.as_deref())),
        ]);
        Ok(Done::answer(object(vec![
            ("statement", Json::text(statement)),
            ("destination", Json::text(destination)),
            ("sha256", Json::text(&obligation.sha256)),
            ("format", Json::text(obligation.format)),
            ("vocabulary", Json::text(obligation.vocabulary)),
            ("pins", pins),
        ])))
    }
}

/// LB5.14's host questions, answered from the KN6 registry and the files the host holds.
impl PreflightHost for Host<'_> {
    fn checker(&self, id: &str) -> Option<String> {
        c600_checkers::archive::find(id).map(|sub| sub.checker.sources.identity().source)
    }

    fn input(&self, file: &FilePin) -> InputState {
        if self.held(file).is_some() {
            return InputState::Present;
        }
        let other = self.grant.digest(&file.path).is_some()
            || self.imports.iter().any(|import| {
                import.report.member(&file.path).is_some_and(|i| import.report.stored_bytes(i).is_some())
            });
        if other {
            InputState::Mismatch
        } else {
            InputState::Missing
        }
    }
}

/// Whether an answer has nothing to list: null, an empty list, or an object with at least one list field whose list
/// fields are all empty (`research.runs`' `rows` and `archived`, `research.obligations`' `rows` and `exports`).
fn empty(answer: Option<&Json>) -> bool {
    match answer {
        None | Some(Json::Null) => true,
        Some(Json::List(items)) => items.is_empty(),
        Some(Json::Object(fields)) => {
            let mut lists = fields.values().filter_map(|value| match value {
                Json::List(items) => Some(items),
                _ => None,
            });
            let first = lists.next();
            first.is_some_and(|items| items.is_empty()) && lists.all(|items| items.is_empty())
        }
        Some(_) => false,
    }
}

//! The six reads' answers, built from the research host's own data. Lists keep the order of their source: claim groups
//! in the claim matrix's order, records in the corpus manifest's, runs in the order they ran. Nothing is sorted by a
//! score, and no answer names a solving word to apply.

use c600_archive::bindings::{Availability as Preflight, Binding, CheckedAs, Dependency, ReadyInput};
use c600_archive::import::{MemberReport, Recognition, Role};
use c600_checkers::CheckReport;
use c600_command::{code, Error};
use c600_evidence::word::{word, Claim};
use c600_evidence::{DigestRole, Edition, Form, Graph, Key, Scope, StatementId};
use c600_session::persist::codec::Json;
use c600_session::persist::run::Reproduction;

use super::host::{ExportEntry, Host, RunEntry};
use super::rows;
use super::wire::{arg_nat, arg_opt_nat, arg_opt_text, arg_sha, arg_text, int, list, object, opt_text, sha, texts};
use crate::archived::archived_runs;
use crate::evidence::{admit, covered};
use crate::job::Det30;
use crate::record::{kind_token, report_json};

/// The restoration stage a corpus path names (`.../restoration/15-lowering-machine/...`), if any.
fn stage_of(path: &str) -> Option<u64> {
    let rest = path.split("restoration/").nth(1)?;
    (rest.get(2..3) == Some("-")).then_some(())?;
    rest.get(..2)?.parse().ok()
}

fn recognition(member: &MemberReport) -> (&'static str, Json) {
    match &member.recognition {
        Recognition::NotAvailable => ("not_available", Json::Null),
        Recognition::Declared(format) => ("declared", Json::text(format)),
        Recognition::Absent(_) => ("absent", Json::Null),
        Recognition::InvalidFormat(_) => ("invalid_format", Json::Null),
        Recognition::Unparsed(_) => ("unparsed", Json::Null),
        Recognition::Referenced { encoding, .. } => ("referenced", Json::text(encoding)),
        Recognition::Unrecognised => ("unrecognised", Json::Null),
    }
}

fn form(form: &Form) -> Json {
    Json::text(match form {
        Form::Finite => "finite",
        Form::Universal => "universal",
        Form::Existence => "existence",
        Form::Mixed(_) => "mixed",
        Form::Other(other) => other,
    })
}

fn role(role: &DigestRole) -> &str {
    match role {
        DigestRole::Poses => "poses",
        DigestRole::Labels => "labels",
        DigestRole::StickerClass => "sticker_class",
        DigestRole::Bytes => "bytes",
        DigestRole::Content => "content",
        DigestRole::Source => "source",
        DigestRole::Other(other) => other,
    }
}

fn claim(claim: Claim) -> Json {
    Json::text(match claim {
        Claim::Proved => "proved",
        Claim::Conditional => "conditional",
        Claim::Refuted => "refuted",
        Claim::Open => "open",
    })
}

fn reading(run: &crate::job::CheckRun) -> Json {
    match run.reading() {
        Det30::Complete => object(vec![("kind", Json::text("complete"))]),
        Det30::NotRun => object(vec![("kind", Json::text("not_run"))]),
        Det30::CutOff { covered } => {
            object(vec![("kind", Json::text("cut_off")), ("covered", c600_session::persist::codec::nat_text(covered))])
        }
        Det30::Open(code) => object(vec![("kind", Json::text("open")), ("code", Json::text(&code))]),
    }
}

fn verdict(report: Option<&CheckReport>) -> Json {
    report.map_or(Json::Null, |report| report_json(report).get("verdict").cloned().unwrap_or(Json::Null))
}

/// DET.7's answer as the panel shows it: `same` with the payload's digest, or `not_run` or `refused` with its code.
pub(super) fn reproduction_json(answer: &Reproduction) -> Json {
    match answer {
        Reproduction::Same(payload) => {
            object(vec![("kind", Json::text("same")), ("payload", Json::text(&sha(payload)))])
        }
        Reproduction::NotRun(code) => object(vec![("kind", Json::text("not_run")), ("code", Json::text(code))]),
        Reproduction::Refused(code) => object(vec![("kind", Json::text("refused")), ("code", Json::text(code))]),
    }
}

pub(super) fn export_json(export: &ExportEntry) -> Json {
    let result = match &export.result {
        Ok(sha256) => object(vec![("kind", Json::text("written")), ("sha256", Json::text(sha256))]),
        Err(code) => object(vec![("kind", Json::text("refused")), ("code", Json::text(code))]),
    };
    object(vec![
        ("row", Json::text(export.row)),
        ("subject", export.subject.clone()),
        ("destination", Json::text(&export.destination)),
        ("result", result),
    ])
}

/// One run as `research.runs` and `research.check.run` show it: its branch (the pinned input), key, outcome, DET.30
/// reading, each check's ledger entry with its verdict, its reports by digest and its latest reproduction.
pub(super) fn run_json(host: &Host<'_>, entry: &RunEntry) -> Json {
    let run = &entry.run;
    let input = run.input();
    let checks = run.children().iter().map(|child| {
        let (token, archive) = kind_token(child.kind);
        object(vec![
            ("at", Json::text(&child.at)),
            ("kind", Json::text(token)),
            (
                "checker",
                object(vec![
                    ("id", opt_text(archive.or(Some(child.checker.id)))),
                    ("source", Json::text(&child.checker.source)),
                ]),
            ),
            ("certificate", Json::text(&child.certificate)),
            ("outcome", Json::text(child.outcome)),
            ("code", opt_text(child.code)),
            ("stop", opt_text(child.stop.as_deref())),
            ("verdict", verdict(child.report.as_ref())),
        ])
    });
    let reproduction = host
        .reproductions
        .iter()
        .rev()
        .find(|repro| repro.run == entry.sha)
        .map_or(Json::Null, |repro| reproduction_json(&repro.answer));
    object(vec![
        ("run", Json::text(&entry.sha)),
        ("record", Json::text(&format!("research.run/{}", entry.sha))),
        ("group", opt_text(entry.group.as_deref())),
        (
            "branch",
            object(vec![
                ("level", Json::text(&input.level)),
                ("digest", Json::text(&input.digest)),
                ("identity", Json::text(&sha(&input.identity))),
            ]),
        ),
        ("key", Json::text(&run.key().digest())),
        ("outcome", Json::text(run.outcome())),
        ("code", opt_text(run.code())),
        ("reading", reading(run)),
        ("checks", list(checks)),
        ("outputs", list(run.outputs().iter().map(|bytes| Json::text(&c600_digest::sha256_hex(bytes))))),
        ("reproduction", reproduction),
    ])
}

fn preflight(answer: &Preflight) -> Json {
    match answer {
        Preflight::Ready { checkers, inputs } => {
            let checkers =
                checkers.iter().map(|c| object(vec![("id", Json::text(&c.id)), ("source", Json::text(&c.digest))]));
            let inputs = inputs.iter().map(|input| match input {
                ReadyInput::Stored { role, path, sha256 } => {
                    object(vec![("role", Json::text(role)), ("path", Json::text(path)), ("sha256", Json::text(sha256))])
                }
                ReadyInput::Derived { role, pointer, digest } => object(vec![
                    ("role", Json::text(role)),
                    ("pointer", Json::text(pointer)),
                    ("digest", Json::text(digest)),
                ]),
            });
            object(vec![("kind", Json::text("ready")), ("checkers", list(checkers)), ("inputs", list(inputs))])
        }
        Preflight::Unavailable { reasons, dependencies } => {
            let reasons = reasons.iter().map(|reason| Json::text(reason.word()));
            let dependencies = dependencies.iter().map(|dependency| match dependency {
                Dependency::MissingInput { source, name, cites } => object(vec![
                    ("kind", Json::text("missing_input")),
                    ("source", Json::text(source)),
                    ("name", Json::text(name)),
                    ("cites", texts(cites)),
                ]),
                Dependency::Input { path, sha256, state } => object(vec![
                    ("kind", Json::text("input")),
                    ("path", Json::text(path)),
                    ("sha256", Json::text(sha256)),
                    ("state", Json::text(&format!("{state:?}").to_lowercase())),
                ]),
                Dependency::HistoricalGate { source, pointer } => object(vec![
                    ("kind", Json::text("historical_gate")),
                    ("source", Json::text(source)),
                    ("pointer", Json::text(pointer)),
                ]),
            });
            object(vec![
                ("kind", Json::text("unavailable")),
                ("reasons", list(reasons)),
                ("dependencies", list(dependencies)),
            ])
        }
    }
}

fn find_group<'b>(host: &'b Host<'_>, group: &str) -> Result<&'b Binding, Error> {
    host.bindings
        .find(group)
        .ok_or_else(|| Error::new(code::ARGUMENT_FORM, format!("`{group}` is no claim group of the claim bindings")))
}

/// The claim bindings' rows a read selects: one group, the groups of one stage, or all, in the matrix's order.
fn selected<'b>(host: &'b Host<'_>, group: Option<&str>, stage: Option<u64>) -> Result<Vec<&'b Binding>, Error> {
    if let Some(group) = group {
        return Ok(vec![find_group(host, group)?]);
    }
    Ok(host.bindings.rows().iter().filter(|row| stage.is_none_or(|stage| u64::from(row.stage) == stage)).collect())
}

impl Host<'_> {
    /// `research.archive.inventory`: per record of the newest import, or of the import whose manifest has the SHA-256
    /// `manifest`, its Git blob and SHA-256, stage, format or its absence, and recognition, integrity and
    /// interpretation as separate results, with the import table's edition (PS3.14, PS3.17). Null before any import.
    pub(super) fn inventory(&self, args: &Json) -> Result<Json, Error> {
        let stage = arg_opt_nat(args, "stage")?;
        let import = match arg_opt_text(args, "manifest")? {
            Some(manifest) => {
                self.imports.iter().rev().find(|import| import.report.manifest_sha256 == manifest).ok_or_else(|| {
                    Error::new(code::NAMED_OBJECT_INVALID, format!("no published import has manifest `{manifest}`"))
                })?
            }
            None => match self.imports.last() {
                Some(import) => import,
                None => return Ok(Json::Null),
            },
        };
        let report = &import.report;
        let rows = report
            .members
            .iter()
            .enumerate()
            .filter(|(_, member)| stage.is_none_or(|stage| stage_of(&member.name) == Some(stage)))
            .map(|(i, member)| {
                let (recognised, format) = recognition(member);
                let interpretation = member.interpretation.as_ref().map_or(Json::Null, |n| {
                    object(vec![
                        ("occurrences", int(n.occurrences)),
                        ("facts", int(n.facts)),
                        ("recorded", int(n.recorded)),
                        ("unmapped", int(n.unmapped)),
                        ("absent_format", int(n.absent_format)),
                        ("prose", int(n.prose)),
                        ("payload_booleans", int(n.payload_booleans)),
                    ])
                });
                let failed = member
                    .checks
                    .iter()
                    .filter(|check| check.result == c600_archive::import::CheckResult::Fail)
                    .map(|check| Json::text(&format!("{} against {}", check.check, check.against)));
                object(vec![
                    ("name", Json::text(&member.name)),
                    ("role", Json::text(if member.role == Role::Record { "record" } else { "auxiliary" })),
                    ("stage", stage_of(&member.name).map_or(Json::Null, |s| int(s as usize))),
                    ("git_blob", Json::text(&member.listed.git_blob)),
                    ("sha256", Json::text(&member.listed.stored_sha256)),
                    ("bytes", Json::Int(i64::try_from(member.listed.stored_bytes).unwrap_or(i64::MAX))),
                    ("present", Json::Bool(member.present && report.stored_bytes(i).is_some())),
                    ("recognition", Json::text(recognised)),
                    ("format", format),
                    ("integrity", Json::text(member.integrity())),
                    ("failed", list(failed)),
                    ("interpretation", interpretation),
                    ("recheck", Json::text(member.recheck)),
                ])
            });
        Ok(object(vec![
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
            ("discriminator", Json::text(&report.discriminator_edition)),
            ("rows", list(rows)),
        ]))
    }

    /// `research.claims`: per claim group, its family, statement and scope phrase, the original label word for word,
    /// check mode, data status, LB5.14's preflight and the latest check verdict beside it. Preflight is no verdict.
    pub(super) fn claims(&self, args: &Json) -> Result<Json, Error> {
        let rows = selected(self, arg_opt_text(args, "group")?, arg_opt_nat(args, "stage")?)?;
        let rows = rows.into_iter().map(|row| {
            let latest =
                self.runs.iter().rev().find(|entry| entry.group.as_deref() == Some(row.group.as_str())).map_or(
                    Json::Null,
                    |entry| {
                        let verdicts = entry.run.children().iter().map(|child| verdict(child.report.as_ref()));
                        object(vec![
                            ("run", Json::text(&entry.sha)),
                            ("outcome", Json::text(entry.run.outcome())),
                            ("reading", reading(&entry.run)),
                            ("verdicts", list(verdicts)),
                        ])
                    },
                );
            object(vec![
                ("group", Json::text(&row.group)),
                ("family", Json::text(&row.family)),
                ("stage", int(usize::from(row.stage))),
                (
                    "statement",
                    object(vec![
                        ("id", Json::text(&row.statement.id)),
                        ("form", row.statement.form.as_ref().map_or(Json::Null, form)),
                        ("input", Json::text(&row.statement.input)),
                    ]),
                ),
                ("scope_phrase", Json::text(&row.scope_phrase)),
                ("label", row.label.as_ref().map_or(Json::Null, |label| Json::text(&label.text))),
                ("checked_as", opt_text(row.checked_as.as_ref().map(CheckedAs::word))),
                ("checkers", texts(&row.checkers)),
                ("data_status", Json::text(row.data_status.word())),
                ("preflight", preflight(&self.bindings.availability(row, self))),
                ("latest_check", latest),
            ])
        });
        Ok(object(vec![("bindings", Json::text(&self.bindings.sha256)), ("rows", list(rows))]))
    }

    /// `research.check.report`: check `check` of run `run`, with its KN6 report when it reached one (KN6.4).
    pub(super) fn report(&self, args: &Json) -> Result<Json, Error> {
        let run = arg_sha(args, "run")?;
        let entry =
            self.runs.iter().find(|entry| entry.sha == run).ok_or_else(|| {
                Error::new(code::NAMED_OBJECT_INVALID, format!("the research host holds no run `{run}`"))
            })?;
        let index = arg_nat(args, "check")?;
        let child = usize::try_from(index)
            .ok()
            .and_then(|i| entry.run.children().get(i))
            .ok_or_else(|| Error::new(code::ARGUMENT_FORM, format!("run `{run}` has no check {index}")))?;
        let (token, _) = kind_token(child.kind);
        Ok(object(vec![
            ("run", Json::text(run)),
            ("check", Json::Int(i64::try_from(index).unwrap_or(i64::MAX))),
            ("at", Json::text(&child.at)),
            ("kind", Json::text(token)),
            (
                "checker",
                object(vec![("id", Json::text(child.checker.id)), ("source", Json::text(&child.checker.source))]),
            ),
            ("outcome", Json::text(child.outcome)),
            ("code", opt_text(child.code)),
            ("report", child.report.as_ref().map_or(Json::Null, report_json)),
        ]))
    }

    /// `evidence.word`: the group's word at the binding's key, and at each key a run of the group covers, with that
    /// run's reports admitted through the job (EVD.46). A key's refusal answers its code.
    pub(super) fn word(&self, args: &Json) -> Result<Json, Error> {
        let row = find_group(self, arg_text(args, "group")?)?;
        let statement = self.bindings.statement(row);
        let mut graph = Graph::new();
        graph.add_statement(statement).map_err(|error| Error::new(error.code, error.detail))?;
        let key = self.bindings.key(row);
        graph.root(&key);
        let mut keys = vec![(Json::Null, key, Json::Null)];
        for entry in self.runs.iter().filter(|entry| entry.group.as_deref() == Some(row.group.as_str())) {
            let covered = covered(&entry.run);
            let scope = Scope::new(
                covered.covers.clone(),
                &entry.run.input().identity_text(),
                &self.bindings.contract,
                covered.input.clone(),
            );
            let key = Key::new(StatementId::typed(&row.statement.id), scope);
            graph.root(&key);
            let admitted = match admit(&mut graph, &key, &entry.run) {
                Ok(leaves) => object(vec![("leaves", int(leaves.iter().flatten().count()))]),
                Err(error) => object(vec![("code", Json::text(error.code))]),
            };
            keys.push((Json::text(&entry.sha), key, admitted));
        }
        let edition = Edition::draft_1();
        let words = keys.into_iter().map(|(run, key, admitted)| {
            let scope = &key.scope;
            let at = object(vec![
                (
                    "covers",
                    Json::text(if scope.covers == c600_evidence::Covers::OneInstance {
                        "one_instance"
                    } else {
                        "other"
                    }),
                ),
                ("identity", Json::text(&c600_digest::sha256_hex(scope.identity.as_bytes()))),
                ("contract", Json::text(&scope.contract)),
                (
                    "input",
                    scope.input.as_ref().map_or(Json::Null, |input| {
                        object(vec![("role", Json::text(role(&input.role))), ("sha256", Json::text(&input.sha256))])
                    }),
                ),
            ]);
            let shown = match word(&graph, &key, &edition) {
                Ok(word) => object(vec![
                    ("word", Json::text(word.word.text())),
                    ("display", Json::text(&word.display())),
                    ("claim", claim(word.claim)),
                    ("row", int(usize::from(word.row))),
                    ("scope_phrase", Json::text(&word.scope_phrase)),
                    ("phrase_required", Json::Bool(word.phrase_required)),
                    ("premises", int(word.premises.len())),
                    ("source_label", opt_text(word.source_label.as_deref())),
                ]),
                Err(error) => object(vec![("code", Json::text(error.code))]),
            };
            object(vec![("run", run), ("scope", at), ("admitted", admitted), ("word", shown)])
        });
        Ok(object(vec![
            ("group", Json::text(&row.group)),
            ("statement", Json::text(&row.statement.id)),
            ("rows", list(words)),
        ]))
    }

    /// `research.runs`: the host's runs in the order they ran, and the archive's runs as recorded, which supply no new
    /// run (DET.48), each reproducing as `not run`.
    pub(super) fn runs_answer(&self, args: &Json) -> Result<Json, Error> {
        let group = match arg_opt_text(args, "group")? {
            Some(group) => Some(find_group(self, group)?),
            None => None,
        };
        let source = group.map(|row| self.bindings.file(row.source).path.as_str());
        let runs = self
            .runs
            .iter()
            .filter(|entry| group.is_none_or(|row| entry.group.as_deref() == Some(row.group.as_str())))
            .map(|entry| run_json(self, entry));
        let archived = self
            .imports
            .iter()
            .flat_map(|import| archived_runs(&import.report, &self.table))
            .filter(|run| source.is_none_or(|source| run.member == source));
        let archived = archived.map(|run| {
            object(vec![
                ("member", Json::text(&run.member)),
                ("field", Json::text(&run.field)),
                ("outcome", Json::text(&run.job_outcome)),
                ("claim", opt_text(run.claim.as_deref())),
                ("check", opt_text(run.check.as_deref())),
                ("scope", opt_text(run.scope.as_deref())),
                ("stop", Json::Bool(run.is_stop())),
                ("reproduction", reproduction_json(&run.reproduction())),
            ])
        });
        Ok(object(vec![("rows", list(runs)), ("archived", list(archived))]))
    }

    /// `research.obligations`: the claim bindings' record-only anchors, each with its export status, and every
    /// obligation export the host wrote or refused (LB5.18).
    pub(super) fn obligations(&self, args: &Json) -> Result<Json, Error> {
        let group = arg_opt_text(args, "group")?;
        let exports: Vec<&ExportEntry> = self.exports.iter().filter(|export| export.row == rows::OBLIGATION).collect();
        let anchors = selected(self, group, None)?
            .into_iter()
            .filter(|row| row.data_status == c600_archive::bindings::DataStatus::RecordOnly)
            .map(|row| {
                let exported = exports
                    .iter()
                    .filter(|export| export.subject.as_text() == Some(row.statement.id.as_str()))
                    .map(|export| export_json(export));
                let missing = row.missing.iter().map(|missing| Json::text(&missing.name));
                object(vec![
                    ("group", Json::text(&row.group)),
                    ("family", Json::text(&row.family)),
                    ("stage", int(usize::from(row.stage))),
                    ("statement", Json::text(&row.statement.id)),
                    ("label", row.label.as_ref().map_or(Json::Null, |label| Json::text(&label.text))),
                    ("missing", list(missing)),
                    ("exports", list(exported)),
                ])
            });
        Ok(object(vec![("rows", list(anchors)), ("exports", list(exports.into_iter().map(export_json)))]))
    }
}

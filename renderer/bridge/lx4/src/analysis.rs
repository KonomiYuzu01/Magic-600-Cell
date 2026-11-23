//! The language work, on a worker thread of its own so that the shell's frame loop never waits on it. A job is the
//! buffer at one revision. The worker takes the newest job waiting, and between stages it drops a job that a newer
//! revision has replaced. Each stage's answer goes back as soon as it is ready. The first three stages are EV1's
//! editor services (`c600-editor`, LP4-1; re-pointed by LP4-3), on the buffer opened as a document in UTF-8 units:
//!
//! 1. syntax: `c600_editor::parse` of the buffer under the edition (its diagnostics and recovery holes);
//! 2. completion: `c600_editor::complete` at the caret, when the caret follows an identifier ([`prefix`]): the labels
//!    of its candidates that start with that identifier (keywords, built-ins, the buffer's declarations and its
//!    import aliases, as the position's context admits them);
//! 3. diagnostics: `c600_editor::diagnose` of the module under a context that holds the W modules' bytes as its
//!    dependencies, when its syntax is clean;
//! 4. dependents, for W00 only: W re-pinned to the buffer's digest by `c600_lx4::repin` and all 50 modules through
//!    `c600_lx4::front_end`. This is LX-4's model of the candidate closure (LG5.1, LG5.3, LG5.10, EV1.11), not a
//!    product fact, and it checks every W module, more than W00's dependents.

use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Arc;
use std::thread::{self, JoinHandle};

use c600_editor::{complete, diagnose, parse, Document, Envelope, ModuleContext, Parse, Stage, Units, Verdict};
use c600_lx4::{front_end, repin, Module, Outcome};

/// The stages, as indices into the endpoints of the crate root.
pub const SYNTAX: usize = 1;
pub const COMPLETION: usize = 2;
pub const DIAGNOSTICS: usize = 3;
pub const DEPENDENTS: usize = 6;

/// The most names a completion list shows.
pub const LIST: usize = 12;

/// The buffer at one revision.
#[derive(Clone, Debug)]
pub struct Job {
    pub revision: u64,
    pub text: String,
    /// The caret's byte offset.
    pub caret: usize,
    /// W00's index when the buffer is W00, so that the dependents are checked.
    pub w00: Option<usize>,
}

/// One stage's answer for one revision.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Answer {
    pub revision: u64,
    pub stage: usize,
    /// What the editor shows for the stage.
    pub text: String,
    /// Whether showing it reaches the stage's endpoint: a parse, a non-empty list, a check that ran. A check not run
    /// for a syntax error, an empty list or an internal failure does not.
    pub endpoint: bool,
}

/// The worker thread and its two channels.
pub struct Worker {
    jobs: Option<Sender<Job>>,
    answers: Receiver<Answer>,
    /// The newest revision sent; a job older than it stops at its next stage.
    newest: Arc<AtomicU64>,
    thread: Option<JoinHandle<()>>,
}

impl Worker {
    pub fn start(workspace: Arc<Vec<Module>>) -> std::io::Result<Worker> {
        let (jobs, inbox) = mpsc::channel::<Job>();
        let (outbox, answers) = mpsc::channel();
        let newest = Arc::new(AtomicU64::new(0));
        let seen = Arc::clone(&newest);
        let thread = thread::Builder::new().name("c600-shell-lx4 analysis".into()).spawn(move || {
            let context = context(&workspace);
            while let Ok(mut job) = inbox.recv() {
                while let Ok(newer) = inbox.try_recv() {
                    job = newer;
                }
                let current = || seen.load(Ordering::SeqCst) == job.revision;
                let send = |stage: usize, text: String, endpoint: bool| {
                    let _ = outbox.send(Answer { revision: job.revision, stage, text, endpoint });
                };
                if catch_unwind(AssertUnwindSafe(|| analyse(&job, &workspace, &context, &current, &send))).is_err() {
                    send(SYNTAX, "the analysis panicked; see the shell's stderr".to_owned(), false);
                }
            }
        })?;
        Ok(Worker { jobs: Some(jobs), answers, newest, thread: Some(thread) })
    }

    pub fn send(&self, job: Job) {
        self.newest.store(job.revision, Ordering::SeqCst);
        if let Some(jobs) = &self.jobs {
            let _ = jobs.send(job);
        }
    }

    /// The answers that arrived since the last call, oldest first.
    pub fn drain(&self) -> Vec<Answer> {
        self.answers.try_iter().collect()
    }
}

impl Drop for Worker {
    /// Stops the worker at its next stage and waits for it.
    fn drop(&mut self) {
        self.newest.store(u64::MAX, Ordering::SeqCst);
        self.jobs = None;
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn analyse(
    job: &Job,
    workspace: &[Module],
    context: &ModuleContext,
    current: &dyn Fn() -> bool,
    send: &dyn Fn(usize, String, bool),
) {
    let document = Document::open(job.text.as_bytes(), Units::Utf8);
    let parsed = parse(&document, &context.edition);
    let clean = parsed.is_complete();
    send(SYNTAX, syntax(&parsed, &job.text), true);
    if !current() {
        return;
    }
    let names = completion(&document, context, job.caret, prefix(&job.text, job.caret));
    let shown = !names.is_empty();
    send(COMPLETION, names.join("\n"), shown);
    if !current() {
        return;
    }
    if clean {
        let (text, ran) = diagnostics(&document, context, &job.text);
        send(DIAGNOSTICS, text, ran);
    } else {
        send(DIAGNOSTICS, "diagnostics: not run (syntax)".to_owned(), false);
    }
    let Some(w00) = job.w00 else { return };
    if !current() {
        return;
    }
    if clean {
        let (text, ran) = dependents(workspace, w00, &job.text);
        send(DEPENDENTS, text, ran);
    } else {
        send(DEPENDENTS, "dependents: not run (syntax)".to_owned(), false);
    }
}

/// The module context of the editor services: this parser's edition with every W module's bytes as a dependency, so
/// that an import by digest resolves.
fn context(workspace: &[Module]) -> ModuleContext {
    let mut context = ModuleContext::new();
    for module in workspace {
        context.add_dependency(module.source.as_bytes(), None);
    }
    context
}

/// The line and column (1-based, the column in scalars) of a byte offset of `text`.
fn line_column(text: &str, byte: usize) -> (usize, usize) {
    let head = text.get(..byte).unwrap_or(text);
    let line = head.matches('\n').count() + 1;
    let column = head.rsplit('\n').next().map_or(0, |l| l.chars().count()) + 1;
    (line, column)
}

/// ` at L:C` for an envelope with a source range, or nothing.
fn at(text: &str, envelope: &Envelope) -> String {
    envelope.span.as_ref().map_or_else(String::new, |span| {
        let (line, column) = line_column(text, span.start);
        format!(" at {line}:{column}")
    })
}

fn syntax(parsed: &Parse, text: &str) -> String {
    if parsed.is_complete() {
        return "syntax: clean".to_owned();
    }
    let mut out = format!("syntax: {} diagnostics, {} holes", parsed.diagnostics.len(), parsed.file.holes.len());
    if let Some(first) = parsed.diagnostics.first() {
        let _ = write!(out, "; first {}{}: {}", first.code, at(text, first), first.summary);
    }
    out
}

/// The identifier that ends at the caret, or nothing when the caret follows no identifier.
pub fn prefix(text: &str, caret: usize) -> &str {
    let head = text.get(..caret).unwrap_or(text);
    let start = head
        .char_indices()
        .rev()
        .take_while(|&(_, c)| c.is_alphanumeric() || c == '_')
        .last()
        .map_or(head.len(), |(i, _)| i);
    let word = &head[start..];
    if word.starts_with(|c: char| c.is_ascii_digit()) {
        ""
    } else {
        word
    }
}

/// The labels `c600_editor::complete` gives at the caret (a byte offset) that start with `prefix`, sorted, at most
/// [`LIST`]; none when the caret follows no identifier or the service refuses the position.
pub fn completion(document: &Document, context: &ModuleContext, caret: usize, prefix: &str) -> Vec<String> {
    if prefix.is_empty() {
        return Vec::new();
    }
    let Ok(answer) = complete(document, document.digest(), caret, context) else { return Vec::new() };
    let names: BTreeSet<&str> =
        answer.candidates.iter().map(|c| c.label.as_str()).filter(|n| n.starts_with(prefix)).collect();
    names.into_iter().take(LIST).map(str::to_owned).collect()
}

/// `c600_editor::diagnose`'s verdict as the status shows it, and whether the check ran.
fn diagnostics(document: &Document, context: &ModuleContext, text: &str) -> (String, bool) {
    let diagnosis = match diagnose(document, document.digest(), context) {
        Ok(diagnosis) => diagnosis,
        Err(refusal) => return (format!("diagnostics: refused: {}", refusal.code), false),
    };
    let first = diagnosis
        .diagnostics
        .first()
        .map_or_else(String::new, |first| format!("; first {}{}: {}", first.code, at(text, first), first.summary));
    match diagnosis.verdict {
        Verdict::Checked(_) => ("diagnostics: clean".to_owned(), true),
        Verdict::Refused => (format!("diagnostics: {}{first}", diagnosis.diagnostics.len()), true),
        Verdict::NotChecked(Stage::Lowering) => (format!("diagnostics: does not lower{first}"), true),
        Verdict::NotChecked(Stage::Dependencies) => (format!("diagnostics: not run (dependencies){first}"), false),
        Verdict::NotChecked(_) => ("diagnostics: not run (syntax)".to_owned(), false),
    }
}

fn dependents(workspace: &[Module], w00: usize, text: &str) -> (String, bool) {
    let sources = repin(workspace, w00, text.to_owned());
    let sources: Vec<&str> = sources.iter().map(String::as_str).collect();
    match front_end(&sources) {
        Outcome::Clean => (format!("dependents: clean ({} modules checked)", sources.len()), true),
        Outcome::Refused(found) => {
            let mut modules: Vec<&str> = Vec::new();
            for (module, _, _) in &found {
                if !modules.contains(&module.as_str()) {
                    modules.push(module);
                }
            }
            let text = format!(
                "dependents: {} diagnostics, refused in {} modules: {} ({} modules checked)",
                found.len(),
                modules.len(),
                modules.join(", "),
                sources.len()
            );
            (text, true)
        }
        Outcome::Syntax => ("dependents: syntax".to_owned(), false),
        Outcome::Lowering(error) => (format!("dependents: do not lower: {error}"), true),
        Outcome::Internal(error) => (format!("dependents: the checker failed: {error}"), false),
    }
}

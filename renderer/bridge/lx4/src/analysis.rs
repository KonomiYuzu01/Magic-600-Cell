//! The language work, on a worker thread of its own so that the shell's frame loop never waits on it. A job is the
//! buffer at one revision. The worker takes the newest job waiting, and between stages it drops a job that a newer
//! revision has replaced. Each stage's answer goes back as soon as it is ready:
//!
//! 1. syntax: `c600_syntax::parse` of the buffer (its diagnostics and recovery holes);
//! 2. completion: the names that start with the identifier before the caret, from `c600_syntax::keywords()` and
//!    `builtins()`, the buffer's declarations and its import aliases;
//! 3. diagnostics: the module alone through `lower` and `c600_check::check`, when its syntax is clean;
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

use c600_check::check;
use c600_lx4::{front_end, repin, Module, Outcome};
use c600_syntax::ast::Item;
use c600_syntax::{builtins, keywords, lower, parse, ParsedFile};

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
            while let Ok(mut job) = inbox.recv() {
                while let Ok(newer) = inbox.try_recv() {
                    job = newer;
                }
                let current = || seen.load(Ordering::SeqCst) == job.revision;
                let send = |stage: usize, text: String, endpoint: bool| {
                    let _ = outbox.send(Answer { revision: job.revision, stage, text, endpoint });
                };
                if catch_unwind(AssertUnwindSafe(|| analyse(&job, &workspace, &current, &send))).is_err() {
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

fn analyse(job: &Job, workspace: &[Module], current: &dyn Fn() -> bool, send: &dyn Fn(usize, String, bool)) {
    let parsed = parse(job.text.as_bytes());
    let clean = parsed.diagnostics.is_empty() && parsed.holes.is_empty();
    send(SYNTAX, syntax(&parsed), true);
    if !current() {
        return;
    }
    let names = completion(&parsed, prefix(&job.text, job.caret));
    let shown = !names.is_empty();
    send(COMPLETION, names.join("\n"), shown);
    if !current() {
        return;
    }
    if clean {
        let (text, ran) = diagnostics(&parsed);
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

fn syntax(parsed: &ParsedFile) -> String {
    if parsed.diagnostics.is_empty() && parsed.holes.is_empty() {
        return "syntax: clean".to_owned();
    }
    let mut text = format!("syntax: {} diagnostics, {} holes", parsed.diagnostics.len(), parsed.holes.len());
    if let Some(first) = parsed.diagnostics.first() {
        let at = first.span.map_or_else(String::new, |s| format!(" at {}:{}", s.line, s.column));
        let _ = write!(text, "; first {}{at}: {}", first.code, first.message);
    }
    text
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

/// The names in scope that start with `prefix`, sorted, at most [`LIST`].
pub fn completion(parsed: &ParsedFile, prefix: &str) -> Vec<String> {
    if prefix.is_empty() {
        return Vec::new();
    }
    let mut names: BTreeSet<&str> = keywords().iter().chain(builtins()).copied().collect();
    names.extend(parsed.declarations().map(|d| d.name.text.as_str()));
    for item in &parsed.items {
        if let Item::Import(import) = item {
            names.insert(&import.alias.text);
        }
    }
    names.into_iter().filter(|n| n.starts_with(prefix)).take(LIST).map(str::to_owned).collect()
}

fn diagnostics(parsed: &ParsedFile) -> (String, bool) {
    let lowered = match lower(std::slice::from_ref(parsed)) {
        Ok(lowered) => lowered,
        Err(error) => return (format!("diagnostics: does not lower: {error}"), true),
    };
    match check(&lowered.program) {
        Ok(report) if report.diagnostics.is_empty() => ("diagnostics: clean".to_owned(), true),
        Ok(report) => {
            let first = &report.diagnostics[0];
            let at = lowered
                .source(&first.module)
                .and_then(|s| s.span(&first.function, &first.path))
                .map_or_else(String::new, |s| format!(" at {}:{}", s.line, s.column));
            let text = format!(
                "diagnostics: {}; first {} in {}.{}{at}",
                report.diagnostics.len(),
                first.code,
                first.module,
                first.function
            );
            (text, true)
        }
        Err(error) => (format!("diagnostics: the checker failed: {error:?}"), false),
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

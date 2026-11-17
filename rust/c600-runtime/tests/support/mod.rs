//! Shared test doubles: a fake session that serves as the caller's host, and the comparison of a run in process with
//! the same run in the child.

#![allow(dead_code)]

use c600_digest::sha256;
use c600_run::interp::{Host, Request, RowTariff, Sink};
use c600_run::meter::code::NOT_GRANTED;
use c600_run::meter::Amounts;
use c600_run::value::Value;
use c600_runtime::codec::same_value;
use c600_runtime::{run_in_process, Config, Ended, Job, JobOptions, Runtime};

pub const WIDE: Amounts = Amounts { steps: 1 << 40, kernel_cost: 1 << 40, memory: 1 << 40, output: 1 << 40 };

/// The header of module `m`.
pub fn header(requires: &str) -> String {
    format!("edition \"6cs-draft-1\";\nmodule m requires [{requires}];\n\n")
}

pub fn job(source: &str, entry: &str, args: Vec<Value>, options: JobOptions) -> Job {
    Job { sources: vec![source.as_bytes().to_vec()], module: "m".into(), entry: entry.into(), args, options }
}

/// A handle on the child binary this package builds.
pub fn runtime() -> Runtime {
    Runtime::new(Config::new(env!("CARGO_BIN_EXE_c600-runtime")))
}

/// A user-action token. Only the shell holds one and only [`Session::confirm`] takes one; no runtime message can
/// carry it (KN5.15).
pub struct Token(());

impl Token {
    pub fn from_the_shell() -> Token {
        Token(())
    }
}

/// A fake session: committed state (a journal), an engine-owned pending preview, the snapshot it answers, and what
/// runs published, wrote and proposed. A proposal replaces the pending preview; only [`Session::confirm`] changes the
/// journal. Every host call is logged in order.
pub struct Session {
    pub journal: Vec<Vec<i32>>,
    pub preview: Option<Vec<i32>>,
    pub snapshots: Vec<(&'static str, RowTariff, Value)>,
    pub files: Vec<(&'static str, Value)>,
    pub views: Vec<Vec<u8>>,
    pub workspace: Vec<(String, Vec<u8>)>,
    pub written: Vec<(String, Vec<u8>)>,
    pub proposals: Vec<(Vec<i32>, String)>,
    pub log: Vec<String>,
    /// Refuses every `perform` with this code.
    pub refuse: Option<&'static str>,
    /// Runs at the start of every `perform`, for tests that act from inside a host call.
    pub on_perform: Option<Box<dyn FnMut()>>,
}

impl Default for Session {
    fn default() -> Session {
        Session {
            journal: vec![vec![1, -2], vec![3]],
            preview: Some(vec![5, -7, 11]),
            snapshots: vec![
                ("pieces", RowTariff { kernel_cost: 5, result_bound: 64 }, Value::int(42)),
                ("small", RowTariff { kernel_cost: 5, result_bound: 1 }, Value::int(42)),
                ("best_word", RowTariff { kernel_cost: 9, result_bound: 4096 }, Value::word(&[1, 2, -3])),
            ],
            files: vec![("saved.6cw", Value::word(&[4, -4]))],
            views: Vec::new(),
            workspace: Vec::new(),
            written: Vec::new(),
            proposals: Vec::new(),
            log: Vec::new(),
            refuse: None,
            on_perform: None,
        }
    }
}

impl Session {
    pub fn state_hash(&self) -> [u8; 32] {
        sha256(format!("{:?}", self.journal).as_bytes())
    }

    /// The shell gate: commits the pending preview, with a token the runtime never sees.
    pub fn confirm(&mut self, _token: Token) -> Result<(), &'static str> {
        let preview = self.preview.take().ok_or("no pending preview")?;
        self.journal.push(preview);
        Ok(())
    }

    /// What a run may change: everything except the log.
    pub fn assert_same(&self, other: &Session) {
        assert_eq!(self.log, other.log, "the host calls differ");
        assert_eq!(self.journal, other.journal);
        assert_eq!(self.preview, other.preview);
        assert_eq!(self.views, other.views);
        assert_eq!(self.workspace, other.workspace);
        assert_eq!(self.written, other.written);
        assert_eq!(self.proposals, other.proposals);
    }
}

impl Host for Session {
    fn tariff(&mut self, request: &Request) -> Result<RowTariff, &'static str> {
        self.log.push(format!("tariff {request:?}"));
        match request {
            // A read with an argument (LP2 draft) answers from the same rows; the log keeps its bytes.
            Request::Snapshot { query, .. } | Request::SnapshotWith { query, .. } => {
                self.snapshots.iter().find(|s| s.0 == *query).map(|s| s.1).ok_or(NOT_GRANTED)
            }
            Request::Propose { .. } => Ok(RowTariff { kernel_cost: 11, result_bound: 4096 }),
            Request::FileRead { .. } => Ok(RowTariff { kernel_cost: 3, result_bound: 4096 }),
            _ => Err(NOT_GRANTED),
        }
    }

    fn perform(&mut self, request: &Request) -> Result<Value, &'static str> {
        self.log.push(format!("perform {request:?}"));
        if let Some(hook) = &mut self.on_perform {
            hook();
        }
        if let Some(code) = self.refuse {
            return Err(code);
        }
        match request {
            Request::Snapshot { query, .. } | Request::SnapshotWith { query, .. } => {
                self.snapshots.iter().find(|s| s.0 == *query).map(|s| s.2.clone()).ok_or(NOT_GRANTED)
            }
            Request::FileRead { path, .. } => {
                self.files.iter().find(|f| f.0 == *path).map(|f| f.1.clone()).ok_or(NOT_GRANTED)
            }
            Request::Propose { word, target, .. } => {
                self.proposals.push((word.to_vec(), target.to_string()));
                self.preview = Some(word.to_vec());
                Ok(Value::receipt(vec![
                    ("base", Value::text("rev-7")),
                    ("effect", Value::text("moves 12 stickers")),
                    ("id", Value::text("preview-1")),
                    ("protection", Value::tag("clear")),
                ]))
            }
            _ => Err(NOT_GRANTED),
        }
    }

    fn sink(&mut self, sink: &Sink, value: &Value, bytes: &[u8]) -> Result<(), &'static str> {
        self.log.push(format!("sink {sink:?} {value:?} {bytes:?}"));
        match sink {
            Sink::View => self.views.push(bytes.to_vec()),
            Sink::Workspace { record, .. } => self.workspace.push((record.to_string(), bytes.to_vec())),
            Sink::File { path } => self.written.push((path.to_string(), bytes.to_vec())),
            Sink::Exact { .. } => {}
        }
        Ok(())
    }
}

/// Holds two ends of the same job to each other: summary (charges, rows, stop), value and output, or the refusal.
pub fn assert_same_end(here: &Ended, there: &Ended) {
    match (here, there) {
        (Ended::Ran(a), Ended::Ran(b)) => {
            assert_eq!(a.summary, b.summary, "the summaries differ");
            assert_eq!(a.output, b.output, "the outputs differ");
            match (&a.value, &b.value) {
                (None, None) => {}
                (Some(x), Some(y)) => assert!(same_value(x, y), "{x:?} / {y:?}"),
                (x, y) => panic!("one run has a value and the other none: {x:?} / {y:?}"),
            }
        }
        (Ended::Refused(a), Ended::Refused(b)) => assert_eq!(a, b),
        _ => panic!("the runs ended differently:\n{here:#?}\n{there:#?}"),
    }
}

/// Runs `job` in process and in the child against two fresh sessions prepared by `setup`, and holds them to each
/// other. Returns the child's end and session.
pub fn compare(runtime: &mut Runtime, job: &Job, setup: impl Fn(&mut Session)) -> (Ended, Session) {
    let mut here = Session::default();
    setup(&mut here);
    let mut there = Session::default();
    setup(&mut there);
    let a = run_in_process(job, &mut here, None);
    let b = runtime.run(job, &mut there, None).unwrap_or_else(|f| panic!("{f}\n{}", f.stderr));
    assert_same_end(&a, &b);
    here.assert_same(&there);
    (b, there)
}

pub fn ran(ended: &Ended) -> &c600_run::interp::Run {
    match ended {
        Ended::Ran(run) => run,
        Ended::Refused(r) => panic!("refused: {r:#?}"),
    }
}

pub fn output(ended: &Ended) -> String {
    String::from_utf8(ran(ended).output.clone().expect("a completed run")).expect("UTF-8")
}

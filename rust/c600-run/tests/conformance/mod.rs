//! Shared helpers for `c600-run`'s LP2 conformance tests (`conformance_lg2.rs`, `conformance_lg3.rs`,
//! `conformance_lg4.rs`): checked surface programs, a fake engine with a committed state and a journal, and runs
//! through the capability-separation boundary (chapter 07, M3).

#![allow(dead_code)]

use std::collections::BTreeMap;

use c600_check::{check, separation, Policy};
use c600_digest::sha256;
use c600_run::boundary::{Boundary, Capture, Context, Pins, Place, Shown, Store};
use c600_run::interp::{load, Host, Loaded, Options, Request, RowTariff, Run, Sink};
use c600_run::meter::code::NOT_GRANTED;
use c600_run::meter::Amounts;
use c600_run::provenance::{self, Author, Json, Pin, Producer, Profile};
use c600_run::value::Value;
use c600_syntax::{lower, parse, Lowered};

pub const WIDE: Amounts = Amounts { steps: 1 << 40, kernel_cost: 1 << 40, memory: 1 << 40, output: 1 << 40 };

/// The run-time rechecks of chapter 13 that a static success cannot bypass (LG3.10).
pub const STALE: &str = "KN5:stale.revision_changed";
pub const NEEDS_JUMBLE_OFF: &str = "KN5:input.retained_tool_needs_jumble_off";
pub const NEEDS_JUMBLE_ON: &str = "KN5:input.menu_word_needs_jumble_on";
pub const IDENTITY_MISMATCH: &str = "KN1:input.identity_mismatch";
pub const OVERLAY_MISSING: &str = "KN5:input.overlay_dependency_missing";
pub const PIECE_WOULD_MOVE: &str = "KN2:protection.piece_would_move";

/// One module `m` with the given declarations and capability list.
pub fn m_requiring(requires: &str, declarations: &str) -> String {
    format!("edition \"6cs-draft-1\";\nmodule m requires [{requires}];\n\n{declarations}")
}

/// One module `m` that requires nothing.
pub fn m(declarations: &str) -> String {
    m_requiring("", declarations)
}

pub fn lowered(sources: &[&str]) -> Lowered {
    let files: Vec<_> = sources
        .iter()
        .map(|s| {
            let file = parse(s.as_bytes());
            assert!(file.diagnostics.is_empty(), "{s}\n{:#?}", file.diagnostics);
            file
        })
        .collect();
    lower(&files).unwrap_or_else(|e| panic!("{e}"))
}

/// Parses, lowers, checks and loads source files; the checker must admit them.
pub fn build(sources: &[&str]) -> (Lowered, Loaded) {
    let lowered = lowered(sources);
    let report = check(&lowered.program).unwrap_or_else(|e| panic!("{e}"));
    assert!(report.is_admitted(), "{:#?}", report.diagnostics);
    let loaded = load(&lowered.program).unwrap_or_else(|r| panic!("{r:?}"));
    (lowered, loaded)
}

/// The engine side of a run: snapshot values by query, the previews it built, the session's committed actions, what
/// it published, and the run-time rechecks it applies to every snapshot read or preview.
#[derive(Default)]
pub struct Engine {
    pub snapshot: BTreeMap<String, Value>,
    pub previews: Vec<Vec<i32>>,
    /// The session's committed actions. No request a program can make appends to it (LG3.4).
    pub journal: Vec<String>,
    pub published: Vec<Vec<u8>>,
    pub refuse_previews: Option<&'static str>,
    pub refuse_reads: Option<&'static str>,
}

impl Engine {
    pub fn with(snapshot: &[(&str, Value)]) -> Engine {
        Engine { snapshot: snapshot.iter().map(|(k, v)| (k.to_string(), v.clone())).collect(), ..Engine::default() }
    }

    /// A digest of the committed state: the snapshot and the journal.
    pub fn state_hash(&self) -> [u8; 32] {
        let mut bytes = Vec::new();
        for (query, value) in &self.snapshot {
            bytes.extend_from_slice(format!("{query}={value:?};").as_bytes());
        }
        for action in &self.journal {
            bytes.extend_from_slice(format!("{action};").as_bytes());
        }
        sha256(&bytes)
    }
}

impl Host for Engine {
    fn tariff(&mut self, _: &Request) -> Result<RowTariff, &'static str> {
        Ok(RowTariff { kernel_cost: 9, result_bound: 1 << 20 })
    }

    fn perform(&mut self, request: &Request) -> Result<Value, &'static str> {
        match request {
            Request::Snapshot { query, .. } => match self.refuse_reads {
                Some(code) => Err(code),
                None => self.snapshot.get(*query).cloned().ok_or(NOT_GRANTED),
            },
            Request::Propose { word, .. } => {
                self.previews.push(word.to_vec());
                match self.refuse_previews {
                    Some(code) => Err(code),
                    None => Ok(Value::receipt(vec![
                        ("base", Value::text("rev-7")),
                        ("effect", Value::text("moves 12 stickers")),
                        ("id", Value::text("preview-1")),
                        ("protection", Value::tag("clear")),
                    ])),
                }
            }
            _ => Err(NOT_GRANTED),
        }
    }

    fn sink(&mut self, sink: &Sink, _: &Value, bytes: &[u8]) -> Result<(), &'static str> {
        match sink {
            Sink::View => {
                self.published.push(bytes.to_vec());
                Ok(())
            }
            Sink::Exact { .. } => Ok(()),
            _ => Err(NOT_GRANTED),
        }
    }
}

/// FP.18's fixture capability contract, held by the policy in these tests.
pub fn contract() -> Pin {
    Pin::of("fixture-1", b"p3 fixture capability contract 1\n")
}

pub fn checker() -> Pin {
    Pin::of("m1-draft", b"c600-check fixture\n")
}

pub fn pins() -> Pins {
    Pins { contracts: vec![contract()], checkers: vec![checker()] }
}

pub fn producer(source: &str, entry: &str, author: Author, profile: Profile) -> Producer {
    Producer {
        contract: contract(),
        checker: Some(checker()),
        author,
        module: sha256(source.as_bytes()),
        entry: entry.into(),
        profile,
    }
}

/// The block of a hand-entered word.
pub fn manual() -> Vec<u8> {
    provenance::canonical(&provenance::manual(&contract()))
}

/// The block of an imported macro file.
pub fn imported() -> Vec<u8> {
    provenance::canonical(&provenance::import(&contract(), b"[{\"kind\":\"word\",\"moves\":[1,-1]}]"))
}

pub struct Ran {
    pub run: Run,
    pub shell: Vec<Shown>,
    pub captures: Vec<Capture>,
}

/// Runs `m.<entry>` of one checked module through a boundary as `author` in `profile`. The entry's state-read fact
/// comes from the checker, as the shell takes it (BND.41).
#[allow(clippy::too_many_arguments)]
pub fn run_as(
    source: &str,
    entry: &str,
    (author, profile): (Author, Profile),
    args: Vec<Value>,
    inputs: Vec<Vec<u8>>,
    engine: &mut Engine,
    store: &mut Store,
    options: &Options,
) -> Ran {
    let (lowered, loaded) = build(&[source]);
    let report = check(&lowered.program).unwrap();
    let reads_state = separation(report.summary("m", entry).expect("the entry"), &Policy::interim()).reads_state;
    let context = Context {
        run: "run".into(),
        base: "rev-7".into(),
        producer: producer(source, entry, author, profile),
        reads_state,
        inputs,
        pins: pins(),
    };
    let mut boundary = Boundary::new(engine, store, context);
    let run = loaded.run("m", entry, args, &mut boundary, options).unwrap_or_else(|r| panic!("{r:?}"));
    Ran { run, shell: boundary.shell, captures: boundary.captures }
}

pub fn ws(name: &str) -> Place {
    Place::Workspace(name.into())
}

pub fn file(name: &str) -> Place {
    Place::File(name.into())
}

/// The stored block of a place, read.
pub fn block_at(store: &Store, place: &Place) -> Json {
    provenance::read(&store.records[place].block).expect("the host's block reads")
}

/// The stored records as (place, value bytes, block bytes), to compare a store before and after a run.
pub fn records(store: &Store) -> Vec<(Place, Vec<u8>, Vec<u8>)> {
    store.records.iter().map(|(p, s)| (p.clone(), s.bytes.clone(), s.block.clone())).collect()
}

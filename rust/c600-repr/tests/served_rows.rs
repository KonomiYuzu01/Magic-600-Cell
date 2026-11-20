//! The representations tool's rows through the command and source routes (LP3 plan, C6: its rows meet C5's item 1).
//! A test host serves the six rows `rows::NEEDS` marks served with the entries it names (`CommandLayer::serve_row`),
//! installs C5's provenance reader, and runs the tool's own entry for each job the layer admits: on the job's
//! arguments, at the job's revision, in process on `c600-run` through the run-time boundary, in the solving context
//! that C5's `c600_tools::tool::Tool::context` gives the 600-cell-Full tool's runs. It reports what the run's sinks
//! gave (`CommandLayer::finish_program_job_with`).
//!
//! The command route is the shell's invocation of a row: the layer checks the arguments' forms and admits a program
//! job, the host runs the entry, and the layer completes the job as the row's kind says. The source route is the same
//! entry run through the runtime on the same arguments and state, with no command layer.
//!
//! Drafts of this host, which settle nothing for the layer, the runtime or the chapters:
//!
//! - The engine side answers the entries' two snapshot reads, `filter.preview` and `structure.compare`, with the state
//!   number of the revision the run reads, and grants nothing else. No kernel row counts the objects a predicate
//!   selects or compares two cells yet, and both entries read a literal query that takes no argument.
//! - A job's arguments map to the entry's parameters by name: a cell, a layer and each member to a natural, the
//!   choice words and the name to text, and a `FilterPreviewRef` to a record of its epoch and job. `filter_counts`
//!   takes neither of `filter.preview`'s arguments. A member outside 0 to 177,119 and a malformed `FilterPreviewRef`
//!   never reach the host: the layer refuses them at admission with its argument-form code.
//! - What a run published and wrote is read back from its canonical bytes as the layer's JSON, where an integer is
//!   `{"int": "n"}`, and a record's provenance is the text of its block.
//! - A job's run takes the blocks the job holds (`ProgramJob::inputs`) as its inputs. A shell invocation carries none:
//!   its arguments are what the solver entered, so the host gives the run a manual record under C5's draft contract
//!   (`named_word_block`). A running program's invocation carries one block, which its host builds from the run's
//!   context and passes with `invoke_with_inputs` (LP2's pull request 264): the block of what the run produces
//!   (`Boundary::block`), which covers every argument the run passes. A run that read a record it stored before it
//!   invoked would also have that record's block among its parts, which only the run's boundary knows, so these blocks
//!   stand for a run that has read none. A running program's job without blocks fails with
//!   `LG3:capability.not_granted`, and the host runs nothing.

mod common;

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};

use c600_command::{
    code, saved_set_record, ActionHint, Caller, CommandLayer, Completion, Direction, Error, Invocation, JobOutput,
    Kind, Overlay, ProgramJob, Receipt, RecordValue, RecordWrite, ResultRef, Revision, SavedSet, Status, Transaction,
    TransactionKind, UndoClass,
};
use c600_digest::sha256;
use c600_kernel::json::{self, Value as Json};
use c600_repr::rows::NEEDS;
use c600_repr::tool::{self, MODULE, SOURCE};
use c600_run::boundary::{Boundary, Context, Place, Store};
use c600_run::interp::{load, Host, Loaded, Options, Request, RowTariff, Sink};
use c600_run::provenance::{self, field, Author, Producer, Profile};
use c600_run::value::Value;
use c600_tools::bridge::base;
use c600_tools::tool::{checker, contract, lower_source, named_word_block, pins, LIMITS};
use c600_tools::workspace;

use common::*;

/// The queries the engine side answers.
const READS: [&str; 2] = ["filter.preview", "structure.compare"];

/// The engine side of one run: the entries' two snapshot reads, answered with the state number of the revision the run
/// reads, and the views the run publishes, in order. It grants nothing else.
struct Engine {
    state: u64,
    views: Vec<Vec<u8>>,
}

impl Host for Engine {
    fn tariff(&mut self, request: &Request) -> Result<RowTariff, &'static str> {
        match request {
            Request::Snapshot { query, .. } if READS.contains(query) => {
                Ok(RowTariff { kernel_cost: 1, result_bound: 64 })
            }
            _ => Err(code::NOT_GRANTED),
        }
    }

    fn perform(&mut self, request: &Request) -> Result<Value, &'static str> {
        match request {
            Request::Snapshot { query, .. } if READS.contains(query) => Ok(Value::int(self.state)),
            _ => Err(code::NOT_GRANTED),
        }
    }

    fn sink(&mut self, sink: &Sink, _: &Value, bytes: &[u8]) -> Result<(), &'static str> {
        match sink {
            Sink::View => {
                self.views.push(bytes.to_vec());
                Ok(())
            }
            _ => Err(code::NOT_GRANTED),
        }
    }
}

/// A host of the representations tool: the shipped source, lowered, admitted and loaded once.
struct ToolHost {
    loaded: Loaded,
    /// Whether each served row's entry holds a state read: its admission's capability separation (BND.22).
    reads_state: BTreeMap<&'static str, bool>,
    /// The inputs of each run this host made, in order: the blocks the context its boundary took holds.
    runs: RefCell<Vec<Vec<Vec<u8>>>>,
}

impl ToolHost {
    /// The shipped source, lowered and loaded, with each served row's entry admitted under the built-in column.
    fn new() -> ToolHost {
        let program = lower_source(SOURCE).expect("the shipped source lowers");
        let mut reads_state = BTreeMap::new();
        for entry in NEEDS.iter().filter(|need| need.served).map(|need| need.entry.expect("an entry")) {
            let admission = tool::admit(&program, MODULE, entry).expect("checked");
            assert!(admission.is_admitted(), "{entry}: {:?}", admission.codes());
            reads_state.insert(entry, admission.separation.reads_state);
        }
        ToolHost { loaded: load(&program).expect("the shipped source loads"), reads_state, runs: RefCell::default() }
    }

    /// The context of one run of `entry` at `revision` in the solving profile (BND.17, BND.22): the tool as a built-in
    /// producer, with `inputs` as the blocks of its arguments.
    fn context(&self, entry: &str, run: &str, revision: &Revision, inputs: Vec<Vec<u8>>) -> Context {
        let producer = Producer {
            contract: contract(),
            checker: Some(checker()),
            author: Author::BuiltIn,
            module: sha256(SOURCE.as_bytes()),
            entry: entry.to_owned(),
            profile: Profile::Solving,
        };
        Context {
            run: run.to_owned(),
            base: base(revision),
            producer,
            reads_state: self.reads_state[entry],
            inputs,
            pins: pins(),
        }
    }

    /// The provenance blocks a host gives a running program's invocation (BND.22), built from the context of the run
    /// `run` of `entry` at `revision`, which took the solver's word as its input: one block, that of what the run
    /// produces (`Boundary::block`, BND.24), for every argument the run passes.
    fn blocks(&self, entry: &str, run: &str, revision: &Revision) -> Vec<Vec<u8>> {
        let (mut engine, mut store) = (Engine { state: revision.state, views: Vec::new() }, Store::default());
        let context = self.context(entry, run, revision, vec![named_word_block()]);
        vec![Boundary::new(&mut engine, &mut store, context).block()]
    }

    /// Runs `entry` on `args` at `revision`, with `inputs` as the blocks of its arguments, through the run-time
    /// boundary onto the engine side and gives what the run's sinks gave, as the layer takes it. A refusal before the
    /// run and a stopped run are errors with their codes.
    fn run(
        &self,
        entry: &str,
        args: Vec<Value>,
        revision: &Revision,
        run: &str,
        inputs: Vec<Vec<u8>>,
    ) -> Result<JobOutput, Error> {
        let mut engine = Engine { state: revision.state, views: Vec::new() };
        let mut store = Store::default();
        let context = self.context(entry, run, revision, inputs);
        self.runs.borrow_mut().push(context.inputs.clone());
        let ran = {
            let mut boundary = Boundary::new(&mut engine, &mut store, context);
            self.loaded.run(MODULE, entry, args, &mut boundary, &Options::new(LIMITS))
        };
        let ran = ran.map_err(|refusal| Error::new(refusal.code, refusal.detail))?;
        let Some(output) = ran.output else {
            let stop = ran.summary.code.unwrap_or(code::NOT_GRANTED);
            return Err(Error::new(stop, format!("the run of `{entry}` stopped")));
        };
        let mut records = BTreeMap::new();
        for (place, stored) in store.records {
            let Place::Workspace(name) = place else {
                return Err(Error::new(code::NOT_GRANTED, "the tool writes no file"));
            };
            let provenance = String::from_utf8(stored.block).expect("a canonical block is text");
            records.insert(name, RecordValue { payload: read(&stored.bytes), provenance });
        }
        let views = engine.views.iter().map(|bytes| read(bytes)).collect();
        Ok(JobOutput { value: read(&output), views, records })
    }

    /// The host's side of the command route: takes the layer's next program job, runs the job's entry on its
    /// arguments at its revision, with the job's blocks as the run's inputs, and reports the output. A shell job
    /// without blocks runs on the manual record of a word the solver named; a running program's job without blocks
    /// fails without a run.
    fn serve_next(&self, layer: &mut CommandLayer) -> Option<(ProgramJob, Result<JobOutput, Error>, Receipt)> {
        let job = layer.take_program_job()?;
        let inputs = match (&job.caller, job.inputs.is_empty()) {
            (_, false) => Ok(job.inputs.clone()),
            (Caller::Shell, true) => Ok(vec![named_word_block()]),
            (Caller::Runtime { .. }, true) => {
                Err(Error::new(code::NOT_GRANTED, "a running program's job carries no block for its arguments"))
            }
        };
        let output = inputs.and_then(|inputs| {
            let args = entry_args(&job.row, &job.args)?;
            self.run(&job.overlay.entry, args, &job.revision, &format!("job/{}", job.job), inputs)
        });
        let receipt = layer.finish_program_job_with(job.job, output.clone()).expect("the job was running");
        Some((job, output, receipt))
    }
}

/// What a host does at every open: serves each row the list marks served with its entry, and installs C5's provenance
/// reader, which checks every record's block (PS1.16).
fn serve(layer: &mut CommandLayer) {
    for need in NEEDS.iter().filter(|need| need.served) {
        let overlay = Overlay { module: MODULE.to_owned(), entry: need.entry.expect("an entry").to_owned() };
        layer.serve_row(need.row, overlay).unwrap_or_else(|error| panic!("{}: {error:?}", need.row));
    }
    workspace::install(layer);
}

/// Canonical bytes (c600-json-1) read as the layer's JSON.
fn read(bytes: &[u8]) -> Json {
    json::parse(bytes).expect("canonical bytes read as JSON")
}

/// An integer as the canonical form writes it.
fn int(n: u64) -> Json {
    json::object([("int", Json::String(n.to_string()))])
}

fn text(s: &str) -> Json {
    Json::String(s.to_owned())
}

fn ints(items: &[i64]) -> Json {
    Json::Array(items.iter().map(|&n| Json::Int(n)).collect())
}

fn form(name: &str) -> Error {
    Error::new(code::ARGUMENT_FORM, format!("`{name}` is no argument the entry takes"))
}

/// The entry's arguments for a job's arguments, by name (a draft of this host).
fn entry_args(row: &str, args: &Json) -> Result<Vec<Value>, Error> {
    let arg = |name: &str| args.get(name).ok_or_else(|| form(name));
    let nat = |value: &Json, name: &str| value.as_i64().filter(|n| *n >= 0).map(Value::int).ok_or_else(|| form(name));
    let string = |name: &str| arg(name)?.as_str().map(Value::text).ok_or_else(|| form(name));
    Ok(match row {
        "filter.preview" => Vec::new(),
        "filter.apply" => {
            let preview = arg("preview")?;
            let field = |key: &str| preview.get(key).ok_or_else(|| form("preview")).and_then(|n| nat(n, "preview"));
            vec![Value::record(vec![("epoch", field("epoch")?), ("job", field("job")?)])]
        }
        "view.effect.scope" => vec![string("scope")?],
        "set.save" => {
            let members = arg("members")?.as_array().ok_or_else(|| form("members"))?;
            let members = members.iter().map(|member| nat(member, "members")).collect::<Result<_, _>>()?;
            vec![string("kind")?, Value::list(members), string("name")?]
        }
        "structure.compare" => match arg("cells")?.as_array() {
            Some([first, second]) => vec![nat(first, "cells")?, nat(second, "cells")?],
            _ => return Err(form("cells")),
        },
        "structure.layer" => vec![nat(arg("cell")?, "cell")?, nat(arg("layer")?, "layer")?],
        _ => return Err(Error::new(code::ROW_NOT_BUILT, format!("this host serves no `{row}`"))),
    })
}

/// The arguments of each invocation in [`command_route`] as the entry takes them, for the source route. `preview` is
/// the job and epoch of the `filter.preview` invocation ([`preview_parts`]), whose filter preview `filter.apply` names.
fn typed(row: &str, preview: (u64, u64)) -> Vec<Value> {
    match row {
        "structure.layer" => vec![Value::int(7), Value::int(2)],
        "filter.preview" => Vec::new(),
        "filter.apply" => {
            let (job, epoch) = preview;
            vec![Value::record(vec![("epoch", Value::int(epoch)), ("job", Value::int(job))])]
        }
        "view.effect.scope" => vec![Value::text("complete")],
        "set.save" => {
            vec![Value::text("identity"), Value::list(vec![Value::int(3), Value::int(5)]), Value::text("pair")]
        }
        "structure.compare" => vec![Value::int(1), Value::int(2)],
        _ => unreachable!("{row} is not served"),
    }
}

/// What the entry of each invocation in [`command_route`] publishes and writes at the state number `state`, `preview`
/// as in [`typed`].
fn expected(row: &str, state: u64, preview: (u64, u64)) -> (Vec<Json>, Option<Json>) {
    match row {
        "filter.preview" | "structure.compare" => (vec![int(state)], None),
        "filter.apply" => {
            let (job, epoch) = preview;
            (Vec::new(), Some(json::object([("epoch", int(epoch)), ("job", int(job))])))
        }
        "view.effect.scope" => (vec![text("complete")], None),
        "set.save" => {
            let members = Json::Array(vec![int(3), int(5)]);
            (Vec::new(), Some(json::object([("kind", text("identity")), ("members", members), ("name", text("pair"))])))
        }
        "structure.layer" => (Vec::new(), Some(json::object([("cell", int(7)), ("layer", int(2))]))),
        _ => unreachable!("{row} is not served"),
    }
}

/// What the layer writes for a row's output at the job's revision and the digest its result reference carries: the
/// output's records and digest, except that `set.save` writes the saved set's own record `set/<name>` and carries the
/// exact set's digest (`c600_command::sets`).
fn written(row: &str, output: &JobOutput, revision: &Revision) -> (String, Vec<RecordWrite>) {
    if row != "set.save" {
        let write =
            |(name, value): (&String, &RecordValue)| RecordWrite { name: name.clone(), value: Some(value.clone()) };
        return (output.digest(), output.records.iter().map(write).collect());
    }
    let entry = &output.records["set.save"];
    let set = SavedSet::from_entry(&entry.payload).expect("the entry's record");
    let model = &revision.identity;
    let value = RecordValue { payload: set.payload(model, revision), provenance: entry.provenance.clone() };
    (set.digest(model), vec![RecordWrite { name: saved_set_record(&set.name), value: Some(value) }])
}

/// One served row's invocation from the shell, and what the host and the layer made of it.
struct Served {
    row: &'static str,
    /// The revision, the durable log's length and the undo hint before the invocation and after its completion.
    before: (Revision, usize, Option<ActionHint>),
    after: (Revision, usize, Option<ActionHint>),
    /// The durable log's last action after the completion.
    last: Option<Transaction>,
    job: ProgramJob,
    output: JobOutput,
    receipt: Receipt,
}

/// The job and epoch of the `filter.preview` invocation among `served`: the parts of the `FilterPreviewRef` that names
/// the filter preview the layer keeps for it.
fn preview_parts(served: &[Served]) -> (u64, u64) {
    let one = served.iter().find(|one| one.row == "filter.preview").expect("invoked before `filter.apply`");
    (one.job.job, one.job.revision.epoch)
}

/// Invokes the six served rows from the shell, the host running each job as the layer admits it. `filter.preview`
/// takes the result reference of the `structure.layer` invocation before it as its predicate, which the layer resolves
/// at admission to the record that invocation wrote, and the current revision, which that invocation moved.
/// `filter.apply` names the filter preview the layer keeps for that `filter.preview` job, by the reference the layer
/// gives for it; the preview is still fresh, since no state moved and no record its filter read was written since.
fn command_route(host: &ToolHost, session: &mut Session) -> Vec<Served> {
    let mut served: Vec<Served> = Vec::new();
    let rows =
        ["structure.layer", "filter.preview", "filter.apply", "view.effect.scope", "set.save", "structure.compare"];
    for row in rows {
        let args = match row {
            "structure.layer" => json::object([("cell", Json::Int(7)), ("layer", Json::Int(2))]),
            "filter.preview" => {
                let prepared = served.iter().find(|s| s.row == "structure.layer").expect("invoked first");
                let job = i64::try_from(prepared.job.job).expect("a small job ID");
                let predicate = json::object([("digest", text(&prepared.output.digest())), ("job", Json::Int(job))]);
                json::object([("predicate", predicate), ("revision", session.layer.revision().to_json())])
            }
            "filter.apply" => {
                let (job, _) = preview_parts(&served);
                let kept = session.layer.filter_preview(job).expect("the layer keeps the filter preview");
                json::object([("preview", kept.reference())])
            }
            "view.effect.scope" => json::object([("scope", text("complete"))]),
            "set.save" => {
                json::object([("kind", text("identity")), ("members", ints(&[3, 5])), ("name", text("pair"))])
            }
            _ => json::object([("cells", ints(&[1, 2]))]),
        };
        let before = session.fixed();
        let started = session.shell.invoke(&mut session.layer, row, args).expect("prepared");
        let Status::Running(id) = started.status else { panic!("{row}: {:?}", started.status) };
        let (job, output, receipt) = host.serve_next(&mut session.layer).expect("the row's job");
        assert_eq!(job.job, id, "{row}");
        let output = output.unwrap_or_else(|error| panic!("{row}: {error:?}"));
        let (after, last) = (session.fixed(), session.layer.store().log.last().cloned());
        served.push(Served { row, before, after, last, job, output, receipt });
    }
    served
}

/// Each of the six served rows, invoked from the shell, runs the entry `rows::NEEDS` names on the job's arguments and
/// state and completes as its kind says. The two queries (`filter.preview`, `structure.compare`) and the view
/// (`view.effect.scope`) complete transiently at the job's revision with the output's digest as the result reference:
/// no revision moves, the durable log keeps its length and nothing is written; the two queries' views are the stand-in
/// engine's state number, not a count or a comparison, and the view's is the scope. The three workspace rows
/// (`filter.apply`, `set.save`, `structure.layer`) each complete as one durable workspace action at a revision whose
/// workspace number is one higher. `filter.apply`'s and `structure.layer`'s write the entry's record through C5's
/// reader, with the output's digest as the result reference; `set.save`'s writes the saved set's own record `set/pair`
/// with the exact set's digest. `filter.apply`'s record is the `FilterPreviewRef` it applied, and H-08 gives the row
/// the undo class `workspace`, so the next undo targets its action. H-08 gives the other two the undo class `none`, so
/// the next undo still targets the action before them. The next undo inverts the applied filter: its record goes and no
/// state moves. The one after it inverts the commit. Both leave the saved set and the prepared predicate as they were:
/// a saved set is removed by its own row, not by undo. Covers no conformance ID: it is C6's evidence for the command
/// route.
#[test]
fn each_served_row_runs_its_entry_and_completes_as_its_kind_says() {
    let host = ToolHost::new();
    let mut session = session();
    session.commit(&[1]);
    serve(&mut session.layer);
    let commit = session.layer.undo_hint(Direction::Undo).expect("the commit").target;
    let served = command_route(&host, &mut session);
    let rows: BTreeSet<&str> = served.iter().map(|one| one.row).collect();
    assert_eq!(rows, NEEDS.iter().filter(|need| need.served).map(|need| need.row).collect());

    let (preview, mut target) = (preview_parts(&served), commit);
    for one in &served {
        let (row, job, output) = (one.row, &one.job, &one.output);
        let need = NEEDS.iter().find(|need| need.row == row).expect("a listed row");
        assert_eq!((job.kind, &job.caller, &job.revision), (need.kind, &Caller::Shell, &one.before.0), "{row}");
        assert_eq!((job.overlay.module.as_str(), Some(job.overlay.entry.as_str())), (MODULE, need.entry), "{row}");
        let (views, record) = expected(row, job.revision.state, preview);
        assert_eq!((&output.value, &output.views), (&Json::Null, &views), "{row}");
        let records: Vec<(&str, &Json)> =
            output.records.iter().map(|(name, value)| (name.as_str(), &value.payload)).collect();
        assert_eq!(records, record.iter().map(|payload| (row, payload)).collect::<Vec<_>>(), "{row}");

        let mut revision = one.before.0.clone();
        if need.kind == Kind::Workspace {
            revision.workspace += 1;
        }
        let (digest, writes) = written(row, output, &job.revision);
        let result = Some(ResultRef { job: Some(job.job), digest: digest.clone() });
        let completion = Completion { kind: need.completion, revision: revision.clone(), result };
        assert_eq!(one.receipt.status, Status::Completed(completion), "{row}");
        if need.kind == Kind::Workspace {
            let last = one.last.as_ref().expect("a durable action");
            assert_eq!((last.kind, last.job, &last.records), (TransactionKind::Workspace, Some(job.job), &writes));
            assert_eq!(last.result, Some(digest), "{row}");
            assert_eq!((&one.after.0, one.after.1), (&revision, one.before.1 + 1), "{row}");
            if need.undo == UndoClass::Workspace {
                target = last.order;
            }
            assert_eq!(one.after.2.map(|hint| hint.target), Some(target), "{row}: the undo target");
        } else {
            assert_eq!(one.after, one.before, "{row}: nothing moves or is written");
        }
    }
    let writes: Vec<RecordWrite> =
        served.iter().flat_map(|one| written(one.row, &one.output, &one.job.revision).1).collect();
    for write in &writes {
        assert_eq!(session.layer.record(&write.name), write.value.as_ref(), "{}", write.name);
    }

    // The undo classes `workspace` and `none`: the next undo inverts the applied filter, the one after it the commit,
    // and the records the other two workspace rows wrote stay.
    let applied = served.iter().find(|one| one.row == "filter.apply").and_then(|one| one.last.as_ref());
    assert_eq!(applied.map(|action| action.order), Some(target), "the applied filter is on top of the undo stack");
    let kept: Vec<RecordWrite> = served
        .iter()
        .filter(|one| one.row != "filter.apply")
        .flat_map(|one| written(one.row, &one.output, &one.job.revision).1)
        .collect();
    let committed = session.layer.snapshot().digest().to_owned();
    for (undone, digest) in [(target, committed), (commit, kernel().solved().digest().to_owned())] {
        let hint = session.layer.undo_hint(Direction::Undo).expect("an undo target");
        assert_eq!(hint.target, undone);
        let undo = session.shell.invoke(&mut session.layer, "session.undo", json::object([("hint", hint.to_json())]));
        let undo = undo.expect("prepared");
        assert!(matches!(undo.status, Status::Committed(_)), "{:?}", undo.status);
        assert_eq!(session.layer.snapshot().digest(), digest);
        assert_eq!(session.layer.record("filter.apply"), None, "the applied filter is undone");
        for write in &kept {
            assert_eq!(session.layer.record(&write.name), write.value.as_ref(), "{} after the undo", write.name);
        }
    }
}

/// The same entry on the source route, this host's own run of it in process through `c600-run`'s boundary (not C5's
/// runner `c600-lp3-source` or the runtime's own process) on the same arguments at the job's revision with no command
/// layer, gives what the command route gave. For `filter.preview` the predicate does not reach the run: the entry
/// takes no argument, and only the layer checks it. For `filter.apply` the run takes the `FilterPreviewRef` as a record
/// of its epoch and job. The two routes give the same returned value, the same published views and the same records,
/// payload and provenance block, so the applied filter, the saved set and the prepared predicate the layer holds are
/// the ones the run stored (for `set.save`, the saved set its record names), and the completion's result reference is
/// the source run's digest (for `set.save`, the exact set's). What differs is by design: only the command route has a
/// receipt (command ID, request digest, caller and job), the layer's form and revision checks, `filter.apply`'s gate
/// `fresh_filter_preview` and, for a workspace row, the durable action, the workspace revision it moves and, for
/// `filter.apply`, its undo entry; on the source route the records stay in the run's store, and the layer's revision,
/// log and undo hint do not move. The run's name differs and appears in no output, since no entry proposes. Covers no
/// conformance ID: it is C6's evidence for C5's item 1.
#[test]
fn the_source_route_gives_the_command_routes_value_views_and_records() {
    let host = ToolHost::new();
    let mut session = session();
    session.commit(&[1]);
    serve(&mut session.layer);
    let served = command_route(&host, &mut session);
    let preview = preview_parts(&served);
    for one in &served {
        let (row, job) = (one.row, &one.job);
        let fixed = session.fixed();
        let (name, inputs) = (format!("source/{row}"), vec![named_word_block()]);
        let source = host.run(&job.overlay.entry, typed(row, preview), &job.revision, &name, inputs);
        let source = source.unwrap_or_else(|error| panic!("{row}: {error:?}"));
        assert_eq!(session.fixed(), fixed, "{row}: the source route moves nothing in the layer");
        assert_eq!(source, one.output, "{row}");
        let Status::Completed(Completion { result: Some(reference), .. }) = &one.receipt.status else {
            panic!("{row}: {:?}", one.receipt.status)
        };
        let (digest, writes) = written(row, &source, &job.revision);
        assert_eq!(reference.digest, digest, "{row}");
        for write in &writes {
            let held = session.layer.record(&write.name);
            assert_eq!(held, write.value.as_ref(), "{row}: the layer holds what the run stored");
        }
    }
}

/// The layer checks the forms of the rows' arguments before it admits a job, so no malformed or stale argument reaches
/// an entry: a scope other than `body` or `complete`, a pair of one cell, a cell outside 0 to 599, a negative layer, a
/// kind other than `identity` or `position`, a member outside 0 to 177,119 (`CanonicalObjectRefs`, the kernel's
/// `PIECES`), a name that is no text, and a `FilterPreviewRef` without its epoch, with a negative job or with a third
/// field are refused with `KN5:input.argument_form`, and a `revision` that is not the current one with
/// `KN5:stale.revision_changed`; none takes a job or moves anything. Covers no conformance ID.
#[test]
fn the_layer_checks_the_rows_argument_forms_before_any_job() {
    let mut session = session();
    serve(&mut session.layer);
    let mut stale = session.layer.revision().clone();
    stale.state += 1;
    // A well-formed `FilterRef` that names no predicate: the stale revision is refused before it is resolved.
    let unresolved = json::object([("digest", text("00")), ("job", Json::Int(999))]);
    let save = |kind: &str, members: &[i64], name: Json| {
        json::object([("kind", text(kind)), ("members", ints(members)), ("name", name)])
    };
    let refused = [
        ("view.effect.scope", json::object([("scope", text("partial"))]), code::ARGUMENT_FORM),
        ("structure.compare", json::object([("cells", ints(&[1]))]), code::ARGUMENT_FORM),
        ("structure.layer", json::object([("cell", Json::Int(600)), ("layer", Json::Int(2))]), code::ARGUMENT_FORM),
        ("structure.layer", json::object([("cell", Json::Int(7)), ("layer", Json::Int(-1))]), code::ARGUMENT_FORM),
        ("set.save", save("other", &[3], text("pair")), code::ARGUMENT_FORM),
        ("set.save", save("identity", &[3], Json::Int(5)), code::ARGUMENT_FORM),
        ("set.save", save("identity", &[3, -5], text("pair")), code::ARGUMENT_FORM),
        ("filter.apply", json::object([("preview", json::object([("job", Json::Int(1))]))]), code::ARGUMENT_FORM),
        (
            "filter.apply",
            json::object([("preview", json::object([("epoch", Json::Int(0)), ("job", Json::Int(-1))]))]),
            code::ARGUMENT_FORM,
        ),
        (
            "filter.apply",
            json::object([(
                "preview",
                json::object([("epoch", Json::Int(0)), ("job", Json::Int(1)), ("x", Json::Int(1))]),
            )]),
            code::ARGUMENT_FORM,
        ),
        (
            "filter.preview",
            json::object([("predicate", unresolved), ("revision", stale.to_json())]),
            code::REVISION_CHANGED,
        ),
    ];
    for (row, args, expected) in refused {
        let before = session.fixed();
        let receipt = session.shell.invoke(&mut session.layer, row, args).expect("prepared");
        assert_eq!(receipt.status.code(), Some(expected), "{row}: {:?}", receipt.status);
        assert!(session.layer.take_program_job().is_none(), "{row}");
        assert_eq!(session.fixed(), before, "{row}");
    }
    assert!(session.layer.records().is_empty());
}

/// `filter.apply` is served with the other five rows, and its gate `fresh_filter_preview` refuses a stale filter
/// preview; `each_served_row_runs_its_entry_and_completes_as_its_kind_says` runs its completion. The layer keeps a
/// filter preview for a completed `filter.preview` job, and the availability hint for its `FilterPreviewRef` is clear
/// while it is fresh. After a commit moves the state, the hint and the shell's invocation refuse it with
/// `KN5:stale.filter_preview`, as they refuse a reference to a job that made none; a running program may not invoke
/// the workspace row (`LG3:capability.not_granted`). No refusal takes a job or moves anything. Covers no conformance
/// ID: `c600-command`'s `tests/filter_apply.rs` runs the row's gate and completion.
#[test]
fn filter_apply_is_served_and_refuses_a_stale_filter_preview() {
    let host = ToolHost::new();
    let mut session = session();
    session.commit(&[1]);
    serve(&mut session.layer);
    let need = NEEDS.iter().find(|need| need.row == "filter.apply").expect("a listed row");
    assert!(need.served);
    let overlay = Overlay { module: MODULE.to_owned(), entry: need.entry.expect("an entry").to_owned() };
    let again = session.layer.serve_row(need.row, overlay).map_err(|error| error.code);
    assert_eq!(again, Err(code::OVERLAY_ROW_CONFLICT));

    let layer_args = json::object([("cell", Json::Int(7)), ("layer", Json::Int(2))]);
    session.shell.invoke(&mut session.layer, "structure.layer", layer_args).expect("prepared");
    let (prepared, output, _) = host.serve_next(&mut session.layer).expect("the predicate's job");
    let digest = output.expect("the predicate").digest();
    let predicate = json::object([("digest", text(&digest)), ("job", Json::Int(prepared.job as i64))]);
    let preview_args = json::object([("predicate", predicate), ("revision", session.layer.revision().to_json())]);
    session.shell.invoke(&mut session.layer, "filter.preview", preview_args).expect("prepared");
    let (counted, output, _) = host.serve_next(&mut session.layer).expect("the filter preview's job");
    output.expect("the counts");
    let kept = session.layer.filter_preview(counted.job).expect("the layer keeps the filter preview");
    let reference = kept.reference();
    let fresh = json::object([("preview", reference.clone())]);
    assert_eq!(session.layer.availability_for(need.row, &fresh), Ok(()), "a fresh filter preview");

    session.commit(&[2]);
    let before = session.fixed();
    let unknown = c600_command::filter::preview_ref(999, session.layer.revision().epoch);
    for preview in [reference, unknown] {
        let what = format!("{preview:?}");
        let args = json::object([("preview", preview)]);
        let hint = session.layer.availability_for(need.row, &args).map_err(|error| error.code);
        assert_eq!(hint, Err(code::STALE_FILTER_PREVIEW), "{what}");
        let receipt = session.shell.invoke(&mut session.layer, need.row, args).expect("prepared");
        assert_eq!(receipt.status.code(), Some(code::STALE_FILTER_PREVIEW), "{what} from the shell");
    }
    let receipt = session.layer.invoke(&program_channel(), session.runtime_call("rt/apply", need.row));
    assert_eq!(receipt.status.code(), Some(code::NOT_GRANTED), "from a program");
    assert!(session.layer.take_program_job().is_none());
    assert_eq!(session.fixed(), before);
    for need in NEEDS.iter().filter(|need| need.served) {
        assert_eq!(session.layer.availability(need.row), Ok(()), "{}", need.row);
    }
}

/// A running program may not invoke a served workspace row: a run reaches no workspace-kind row (finding B3 of the KN5
/// boundary attack; ONE.52, the owner's reading of 18 November 2026), so the layer refuses `set.save` on the program's
/// channel with `LG3:capability.not_granted`, as it refuses `workspace.set`, admits no job, and nothing is written or
/// moved. The host's own refusal of a running program's job (its arguments carry no blocks, which BND.22 needs) is no
/// longer reached for this row. Covers no conformance ID.
#[test]
fn a_running_program_cannot_invoke_a_served_workspace_row() {
    let mut session = session();
    serve(&mut session.layer);
    let before = session.fixed();
    let invocation = Invocation {
        command: "run-1/1".to_owned(),
        row: "set.save".to_owned(),
        args: json::object([("kind", text("identity")), ("members", ints(&[3, 5])), ("name", text("pair"))]),
        token: None,
        epoch: session.layer.revision().epoch,
    };
    let refused = session.layer.invoke(&program_channel(), invocation);
    assert_eq!(refused.status.code(), Some(code::NOT_GRANTED), "{:?}", refused.status);
    assert!(session.layer.take_program_job().is_none(), "the refused invocation admitted a job");
    assert_eq!(session.fixed(), before);
    assert!(session.layer.records().is_empty());
}

/// A running program's invocation of a served query or view row carries the provenance blocks of its arguments, which
/// its host builds from the run's context (BND.22; LP2's `invoke_with_inputs`). Here the calling run stands in as the
/// run `run-1` of the tool's `compare_cells` entry, and its one block reads as a program block of the tool, built-in,
/// in the solving profile, that may read the state, with the solver's word as its one part. The layer checks the block
/// with C5's reader and holds it on the job (`ProgramJob::inputs`), and the host runs the entry with it as the run's
/// input: `filter.preview`, `structure.compare` and `view.effect.scope` complete transiently at the job's revision with
/// the output the shell's invocation of the same row gives, and nothing moves. Without blocks the host fails such a job
/// with `LG3:capability.not_granted` and runs nothing; a block the reader refuses refuses the invocation with
/// `PS1:capability.provenance_incompatible` before any job. Covers no conformance ID.
#[test]
fn a_running_programs_query_and_view_jobs_run_on_blocks_built_from_its_context() {
    let host = ToolHost::new();
    let mut session = session();
    session.commit(&[1]);
    serve(&mut session.layer);
    let layer_args = json::object([("cell", Json::Int(7)), ("layer", Json::Int(2))]);
    session.shell.invoke(&mut session.layer, "structure.layer", layer_args).expect("prepared");
    let (prepared, output, _) = host.serve_next(&mut session.layer).expect("the predicate's job");
    let digest = output.expect("the predicate").digest();
    let predicate = json::object([("digest", text(&digest)), ("job", Json::Int(prepared.job as i64))]);

    let (caller, revision) = (program_channel(), session.layer.revision().clone());
    let Caller::Runtime { run } = &caller else { unreachable!("the program's channel") };
    let blocks = host.blocks("compare_cells", run, &revision);
    let block = provenance::read(&blocks[0]).expect("a canonical block");
    assert_eq!((field::author(&block), field::producer(&block)), (Author::BuiltIn.name(), "program"));
    assert_eq!(field::profile(&block), Profile::Solving.name());
    assert!(field::may_read_state(&block));
    assert_eq!(field::parts(&block), vec![sha256(&named_word_block())]);

    let calls = [
        ("filter.preview", json::object([("predicate", predicate), ("revision", revision.to_json())])),
        ("structure.compare", json::object([("cells", ints(&[1, 2]))])),
        ("view.effect.scope", json::object([("scope", text("complete"))])),
    ];
    for (n, (row, args)) in calls.into_iter().enumerate() {
        let call = |i: usize| Invocation {
            command: format!("{run}/{}", 3 * n + i),
            row: row.to_owned(),
            args: args.clone(),
            token: None,
            epoch: revision.epoch,
        };
        session.shell.invoke(&mut session.layer, row, args.clone()).expect("prepared");
        let (_, shell, _) = host.serve_next(&mut session.layer).expect("the shell's job");
        let shell = shell.unwrap_or_else(|error| panic!("{row} from the shell: {error:?}"));
        assert_eq!(host.runs.borrow().last(), Some(&vec![named_word_block()]), "{row}: the shell's run's input");
        let (before, runs) = (session.fixed(), host.runs.borrow().len());

        let bare = session.layer.invoke(&caller, call(1));
        assert!(matches!(bare.status, Status::Running(_)), "{row}: {:?}", bare.status);
        let (job, output, receipt) = host.serve_next(&mut session.layer).expect("the job without blocks");
        assert!(job.inputs.is_empty(), "{row}");
        assert_eq!(output.map_err(|error| error.code), Err(code::NOT_GRANTED), "{row}");
        assert_eq!(receipt.status.code(), Some(code::NOT_GRANTED), "{row}");
        assert_eq!(host.runs.borrow().len(), runs, "{row}: no run");

        let refused = session.layer.invoke_with_inputs(&caller, call(2), vec![b"no block".to_vec()]);
        assert_eq!(refused.status.code(), Some(code::PROVENANCE_INCOMPATIBLE), "{row}");
        assert!(session.layer.take_program_job().is_none(), "{row}");

        let started = session.layer.invoke_with_inputs(&caller, call(3), blocks.clone());
        let Status::Running(id) = started.status else { panic!("{row}: {:?}", started.status) };
        let (job, output, receipt) = host.serve_next(&mut session.layer).expect("the job with blocks");
        assert_eq!((job.job, &job.caller, &job.inputs, &job.revision), (id, &caller, &blocks, &revision), "{row}");
        let output = output.unwrap_or_else(|error| panic!("{row}: {error:?}"));
        assert_eq!(&host.runs.borrow()[runs..], std::slice::from_ref(&blocks), "{row}: one run, on the job's blocks");
        assert_eq!(output, shell, "{row}: the shell's output");
        let need = NEEDS.iter().find(|need| need.row == row).expect("a listed row");
        let result = Some(ResultRef { job: Some(id), digest: output.digest() });
        let completion = Completion { kind: need.completion, revision: revision.clone(), result };
        assert_eq!(receipt.status, Status::Completed(completion), "{row}");
        assert_eq!(session.fixed(), before, "{row}: nothing moves");
    }
}

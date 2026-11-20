//! The provenance blocks of a program job's arguments (BND.22; LP2 draft): `CommandLayer::invoke_with_inputs`. The host
//! builds the blocks, for a running program's invocation from the run's context, and passes them beside the envelope;
//! the layer checks each block's form with the host's reader and holds the blocks on the job (`ProgramJob::inputs`).
//! The frozen text says that the host fills a block and the engine checks its form only (BND.22); it is silent on how a
//! job's arguments carry blocks, so these tests follow the draft recorded in `src/layer.rs`.

use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use c600_command::filter::preview_ref;
use c600_command::{
    code, digest, draft_args, Caller, CommandLayer, Error, Invocation, MemoryStore, Overlay, Provenance, Secret,
    Status, Table,
};
use c600_kernel::json::{self, Value};
use c600_kernel::Kernel;

const SECRET: [u8; 32] = [7; 32];
const COMPARE: &str = "structure.compare";

fn kernel() -> Arc<Kernel> {
    static KERNEL: OnceLock<Arc<Kernel>> = OnceLock::new();
    KERNEL
        .get_or_init(|| {
            let repository: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..");
            match Kernel::load(&repository.join("assets")) {
                Ok(kernel) => Arc::new(kernel),
                Err(error) => panic!("the model in assets/ does not load: {error}"),
            }
        })
        .clone()
}

/// The host's provenance reader as the tests stand it in: it refuses the block `refused` and accepts every other.
struct Reader;

impl Provenance for Reader {
    fn check(&self, block: &[u8]) -> Result<(), Error> {
        match block {
            b"refused" => Err(Error::new(code::PROVENANCE_INCOMPATIBLE, "the test reader refuses the block")),
            _ => Ok(()),
        }
    }

    fn admit(&self, _block: &[u8]) -> Result<(), Error> {
        Ok(())
    }
}

/// A session at the solved state that protects nothing, on H-08's table with `structure.compare` served, and with the
/// test reader when `reader`.
fn layer(reader: bool) -> CommandLayer {
    let k = kernel();
    let (start, protected) = (k.solved(), k.protected_set(&[]).unwrap());
    let mut layer =
        CommandLayer::new(k, Table::h08(), Secret::fixed(SECRET), MemoryStore::default(), "s1", start, protected);
    let overlay = Overlay { module: "tools/representations".into(), entry: "compare_cells".into() };
    layer.serve_row(COMPARE, overlay).unwrap();
    if reader {
        layer.set_provenance(Box::new(Reader));
    }
    layer
}

fn text(s: &str) -> Value {
    Value::String(s.to_owned())
}

fn runtime() -> Caller {
    Caller::Runtime { run: "run-1".into() }
}

fn invocation(layer: &CommandLayer, command: &str, row: &str, args: Value) -> Invocation {
    let served = layer.table().row(row).is_some_and(|found| found.overlay.is_some());
    Invocation {
        command: command.into(),
        row: row.into(),
        args,
        token: None,
        epoch: layer.revision().epoch,
        budget: served.then_some(c600_command::Budget::DRAFT_CEILING),
    }
}

fn compare(layer: &CommandLayer, command: &str) -> Invocation {
    let cells = json::object([("cells", Value::Array(vec![Value::Int(3), Value::Int(4)]))]);
    invocation(layer, command, COMPARE, cells)
}

fn blocks() -> Vec<Vec<u8>> {
    vec![br#"{"producer":"manual"}"#.to_vec(), b"second".to_vec()]
}

/// A running program's invocation of a served query row, with the blocks its host built, admits a job that holds the
/// blocks as given and in their order; the shell's invocation without blocks admits one that holds none. A retry with
/// the same blocks finds the stored receipt, and one with other blocks or none is other content
/// (`KN5:input.command_id_reused`, KN5.21). Covers no conformance ID: BND.22 says the host fills the block; how a
/// job's arguments carry it is LP2's draft.
#[test]
fn a_job_holds_the_blocks_its_host_gave() {
    let mut layer = layer(true);
    let receipt = layer.invoke_with_inputs(&runtime(), compare(&layer, "run-1/1"), blocks());
    let Status::Running(job) = receipt.status else { panic!("{:?}", receipt.status) };
    let taken = layer.take_program_job().expect("the job");
    assert_eq!((taken.job, &taken.caller, &taken.inputs), (job, &runtime(), &blocks()));
    let done = layer.finish_program_job(job, Ok(json::object([("relation", text("adjacent"))]))).unwrap();
    assert!(matches!(done.status, Status::Completed(_)), "{:?}", done.status);

    assert_eq!(layer.invoke_with_inputs(&runtime(), compare(&layer, "run-1/1"), blocks()), done);
    let other = layer.invoke_with_inputs(&runtime(), compare(&layer, "run-1/1"), vec![b"other".to_vec()]);
    assert_eq!(other.status.code(), Some(code::COMMAND_ID_REUSED));
    let bare = layer.invoke(&runtime(), compare(&layer, "run-1/1"));
    assert_eq!(bare.status.code(), Some(code::COMMAND_ID_REUSED));
    assert!(layer.take_program_job().is_none());

    let mine = layer.invoke(&Caller::Shell, compare(&layer, "shell/1"));
    assert!(matches!(mine.status, Status::Running(_)), "{:?}", mine.status);
    assert!(layer.take_program_job().expect("the shell's job").inputs.is_empty());
}

/// The request digest of an invocation without blocks is the one it had before blocks existed, so stored receipts and
/// journals keep theirs; with blocks it also covers each block's SHA-256, in order. The run's invocation of a served
/// row declares its budget, which the digest covers either way (`tests/budget.rs`). Covers no conformance ID.
#[test]
fn only_an_invocation_with_blocks_has_them_in_its_request_digest() {
    let mut first = layer(true);
    let call = compare(&first, "run-1/1");
    let fields = |inputs: Value| {
        let mut request = json::object([
            ("session", text("s1")),
            ("epoch", Value::Int(call.epoch as i64)),
            ("caller", runtime().to_json()),
            ("command", text("run-1/1")),
            ("row", text(COMPARE)),
            ("args", call.args.clone()),
            ("token", Value::Null),
            ("budget", c600_command::Budget::DRAFT_CEILING.to_json()),
        ]);
        if let (Value::Object(fields), false) = (&mut request, inputs == Value::Null) {
            fields.insert("inputs".to_owned(), inputs);
        }
        request
    };
    let receipt = first.invoke_with_inputs(&runtime(), call.clone(), Vec::new());
    assert_eq!(receipt.request, digest(&fields(Value::Null)));
    // Pinned, so that a change to the digest's fields shows here as well as in the rebuilt fields above.
    assert_eq!(receipt.request, "a870e4db0fce072994a07913b528f2e9fa7505630412810a6f9ad54ee6e88468");

    let mut second = layer(true);
    let receipt = second.invoke_with_inputs(&runtime(), call.clone(), blocks());
    let hashed = blocks().iter().map(|block| text(&c600_digest::sha256_hex(block))).collect();
    assert_eq!(receipt.request, digest(&fields(Value::Array(hashed))));
}

/// The host's reader checks each block after the arguments' forms and before any job: a block it refuses refuses the
/// invocation with its error (`PS1:capability.provenance_incompatible`), from a running program and from the shell, and
/// with no reader an invocation with blocks is refused with `KN5:input.row_not_built`. A malformed argument is refused
/// as a form whatever the blocks. None of these takes a job or moves the revision or the durable log. Covers no
/// conformance ID.
#[test]
fn the_reader_checks_each_block_before_any_job() {
    let cases = [
        (true, vec![b"fine".to_vec(), b"refused".to_vec()], code::PROVENANCE_INCOMPATIBLE),
        (false, blocks(), code::ROW_NOT_BUILT),
    ];
    for (reader, inputs, expected) in cases {
        let mut layer = layer(reader);
        let before = layer.revision().clone();
        for (i, caller) in [runtime(), Caller::Shell].iter().enumerate() {
            let receipt = layer.invoke_with_inputs(caller, compare(&layer, &format!("c/{i}")), inputs.clone());
            assert_eq!(receipt.status.code(), Some(expected), "{caller:?}: {:?}", receipt.status);
            assert!(layer.take_program_job().is_none());
        }
        assert_eq!((layer.revision(), layer.store().log.len()), (&before, 0));
    }

    let mut layer = layer(true);
    let malformed = json::object([("cells", Value::Array(vec![Value::Int(3)]))]);
    let call = invocation(&layer, "c/9", COMPARE, malformed);
    let receipt = layer.invoke_with_inputs(&runtime(), call, vec![b"refused".to_vec()]);
    assert_eq!(receipt.status.code(), Some(code::ARGUMENT_FORM));
    assert!(layer.take_program_job().is_none());
}

/// The `FilterRef`s and the gate `fresh_filter_preview` come before the blocks: a `filter.preview` whose predicate
/// names no job, and a `filter.apply` that names no filter preview, are refused with their own codes
/// (`KN2:input.named_object_invalid`, `KN5:stale.filter_preview`) even when the reader would refuse a block. Covers no
/// conformance ID.
#[test]
fn the_filter_references_and_gate_come_before_the_blocks() {
    let mut layer = layer(true);
    layer
        .serve_row("filter.preview", Overlay { module: "tools/representations".into(), entry: "filter_counts".into() })
        .unwrap();
    layer
        .serve_row("filter.apply", Overlay { module: "tools/representations".into(), entry: "apply_filter".into() })
        .unwrap();
    let predicate = json::object([("digest", text("00")), ("job", Value::Int(999))]);
    let args = json::object([("predicate", predicate), ("revision", layer.revision().to_json())]);
    let epoch = layer.revision().epoch;
    let calls = [
        ("filter.preview", args, code::NAMED_OBJECT_INVALID),
        ("filter.apply", json::object([("preview", preview_ref(999, epoch))]), code::STALE_FILTER_PREVIEW),
    ];
    for (i, (row, args, expected)) in calls.into_iter().enumerate() {
        let call = invocation(&layer, &format!("shell/{i}"), row, args);
        let receipt = layer.invoke_with_inputs(&Caller::Shell, call, vec![b"refused".to_vec()]);
        assert_eq!(receipt.status.code(), Some(expected), "{row}: {:?}", receipt.status);
        assert!(layer.take_program_job().is_none(), "{row}");
    }
}

/// Blocks change no admission: a running program's invocation with blocks of a commit row is refused with
/// `KN5:capability.commit_row_refused` and of a served workspace row with `LG3:capability.not_granted`, as without
/// blocks. Such a refusal does not authenticate, so it is kept under the envelope's full digest, which covers the
/// blocks: the same envelope gets it again, one with other blocks is dispatched again, and neither attaches to the
/// command ID, which a later authentic invocation then uses as the first (ruling 4). Covers no conformance ID.
#[test]
fn blocks_change_no_admission() {
    let mut layer = layer(true);
    layer.serve_row("set.save", Overlay { module: "tools/representations".into(), entry: "save_set".into() }).unwrap();
    let before = layer.revision().clone();
    let execute = invocation(&layer, "run-1/1", "operation.execute", json::object([("preview", Value::Int(1))]));
    let receipt = layer.invoke_with_inputs(&runtime(), execute, blocks());
    assert_eq!(receipt.status.code(), Some(code::COMMIT_ROW_REFUSED), "{:?}", receipt.status);

    let save = json::object([
        ("kind", text("identity")),
        ("members", Value::Array(vec![Value::Int(3), Value::Int(5)])),
        ("name", text("pair")),
    ]);
    let call = invocation(&layer, "run-1/2", "set.save", save);
    let first = layer.invoke_with_inputs(&runtime(), call.clone(), blocks());
    assert_eq!(first.status.code(), Some(code::NOT_GRANTED), "{:?}", first.status);
    assert_eq!(layer.invoke_with_inputs(&runtime(), call.clone(), blocks()), first);
    let other = layer.invoke_with_inputs(&runtime(), call, vec![b"other".to_vec()]);
    assert_eq!(other.status.code(), Some(code::NOT_GRANTED), "{:?}", other.status);
    assert_ne!(other.request, first.request);
    assert!(layer.take_program_job().is_none());
    assert_eq!((layer.revision(), layer.store().log.len()), (&before, 0));

    let receipt = layer.invoke_with_inputs(&runtime(), compare(&layer, "run-1/2"), blocks());
    assert!(matches!(receipt.status, Status::Running(_)), "{:?}", receipt.status);
}

/// A row the layer builds runs no entry, so it takes no argument blocks: `operation.preview` with blocks is refused
/// with `KN5:input.argument_form` and admits no preview job, while the same draft without blocks is admitted. Covers no
/// conformance ID.
#[test]
fn a_row_that_runs_no_entry_takes_no_blocks() {
    let mut layer = layer(true);
    let recipe = json::parse(br#"[{"kind": "word", "moves": [1]}]"#).unwrap();
    let before = layer.revision().clone();
    let call = invocation(&layer, "shell/1", "operation.preview", draft_args(&recipe));
    let receipt = layer.invoke_with_inputs(&Caller::Shell, call, blocks());
    assert_eq!(receipt.status.code(), Some(code::ARGUMENT_FORM), "{:?}", receipt.status);
    assert_eq!(layer.run_jobs(), 0);
    assert_eq!((layer.revision(), layer.store().log.len()), (&before, 0));

    let call = invocation(&layer, "shell/2", "operation.preview", draft_args(&recipe));
    let receipt = layer.invoke_with_inputs(&Caller::Shell, call, Vec::new());
    assert!(matches!(receipt.status, Status::Running(_)), "{:?}", receipt.status);
}

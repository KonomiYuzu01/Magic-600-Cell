//! The budget a program row's invocation declares (KN7.7, BM3, DET.27, ERR.27; draft): CT-RTE-07's B13 and B14 at the
//! command layer, the ceiling's dimensions, the job's budget, the request digest and the journal. Where the frozen text
//! is silent, the tests follow the draft recorded in `src/lib.rs` under "Budgets".

use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use c600_command::{
    code, Budget, Caller, CommandLayer, Invocation, Kind, MemoryStore, Overlay, Profile, Receipt, Secret, Shell,
    Status, Table, Transaction,
};
use c600_kernel::json::{self, Value};
use c600_kernel::Kernel;

const SECRET: [u8; 32] = [7; 32];
/// A tool's query row, an overlay row on the program lane.
const TOOL: &str = "tool.count";
/// A tool's workspace row, whose completion is a durable action.
const STORE: &str = "tool.store";

fn kernel() -> Arc<Kernel> {
    static KERNEL: OnceLock<Arc<Kernel>> = OnceLock::new();
    let assets: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..").join("assets");
    KERNEL
        .get_or_init(|| match Kernel::load(&assets) {
            Ok(kernel) => Arc::new(kernel),
            Err(error) => panic!("the model in assets/ does not load: {error}"),
        })
        .clone()
}

fn rows(layer: &mut CommandLayer) {
    let overlay = |entry: &str| Overlay { module: "tools/count".into(), entry: entry.into() };
    layer.register_overlay(TOOL, Kind::Query, &[], overlay("run")).unwrap();
    layer.register_overlay(STORE, Kind::Workspace, &[], overlay("store")).unwrap();
}

/// A session at the solved state in `profile`, with the two tool rows and room for three program jobs.
fn session(profile: Profile) -> CommandLayer {
    let k = kernel();
    let (start, protected) = (k.solved(), k.protected_set(&[]).unwrap());
    let (table, secret, store) = (Table::h08().with_drafts(), Secret::fixed(SECRET), MemoryStore::default());
    let mut layer = CommandLayer::new_with_profile(k, table, secret, store, "s1", start, protected, profile);
    rows(&mut layer);
    layer.set_program_capacity(3);
    layer
}

/// One envelope of `row` with no arguments, no token and the current epoch.
fn envelope(layer: &CommandLayer, command: &str, row: &str, budget: Option<Budget>) -> Invocation {
    let (command, row, args) = (command.to_owned(), row.to_owned(), json::object([]));
    Invocation { command, row, args, token: None, epoch: layer.revision().epoch, budget }
}

/// The budget of CT-RTE-07's B14 (`limits` in `budgets.json` of LX-3's frozen corpus).
const B14: Budget = Budget { steps: 2, kernel_cost: 10_000, memory: 65_536, output: 65_536 };

fn refusal(receipt: &Receipt) -> (&'static str, &str) {
    match &receipt.status {
        Status::Refused(error) => (error.code, error.detail.as_str()),
        other => panic!("not refused: {other:?}"),
    }
}

/// What a refusal before admission must leave as it was: no job queued or issued, the revision and the journal.
fn unchanged(layer: &mut CommandLayer, revision: &Value, journal: usize) {
    assert!(layer.take_program_job().is_none(), "no job is queued");
    assert_eq!(&layer.revision().to_json(), revision, "the revision");
    assert_eq!(layer.store().log.len(), journal, "the journal");
    // The next admitted job takes the first job ID: the refusal issued none.
    let receipt = layer.invoke(&Caller::Shell, envelope(layer, "after", TOOL, Some(B14)));
    assert!(matches!(receipt.status, Status::Running(1)), "{:?}", receipt.status);
}

/// CT-RTE-07's B13 (KN7.7, ERR.27): a command in a tool-authoring session whose envelope declares no budget is refused
/// with `KN7:input.budget_not_declared` before admission, and uses nothing.
#[test]
fn b13_an_invocation_without_a_budget_is_refused_before_a_job_exists() {
    let mut layer = session(Profile::ToolAuthoring);
    let (revision, journal) = (layer.revision().to_json(), layer.store().log.len());
    let receipt = layer.invoke(&Caller::Shell, envelope(&layer, "c1", TOOL, None));
    assert_eq!(refusal(&receipt).0, code::BUDGET_NOT_DECLARED);
    assert_eq!(receipt.job, None);
    unchanged(&mut layer, &revision, journal);

    // A run's envelope is refused the same way, and so is the shell's when it declares none.
    let mut layer = session(Profile::ToolAuthoring);
    let receipt = layer.invoke(&Caller::Runtime { run: "r1".into() }, envelope(&layer, "c1", TOOL, None));
    assert_eq!(refusal(&receipt).0, code::BUDGET_NOT_DECLARED);
    let mut shell = Shell::new(Secret::fixed(SECRET), "shell");
    shell.budget = None;
    let receipt = shell.invoke(&mut layer, TOOL, json::object([])).unwrap();
    assert_eq!(refusal(&receipt).0, code::BUDGET_NOT_DECLARED);
}

/// CT-RTE-07's B14 (BM3, ERR.16, ERR.27): under a host ceiling of one step, B14's budget of two steps is refused with
/// `KN7:resource_limit.budget_ceiling` before admission, naming the dimension, the request and the ceiling, and uses
/// nothing.
#[test]
fn b14_a_budget_over_the_host_ceiling_is_refused_before_a_job_exists() {
    let mut layer = session(Profile::ToolAuthoring);
    layer.set_budget_ceiling(Budget { steps: 1, ..Budget::DRAFT_CEILING });
    let (revision, journal) = (layer.revision().to_json(), layer.store().log.len());
    let receipt = layer.invoke(&Caller::Shell, envelope(&layer, "c1", TOOL, Some(B14)));
    let (code, detail) = refusal(&receipt);
    assert_eq!(code, code::BUDGET_CEILING);
    assert!(detail.contains("`steps`") && detail.contains(" 2 ") && detail.ends_with(" 1"), "{detail}");
    assert_eq!(receipt.job, None);
    // The refusal used nothing; a budget at the ceiling is admitted.
    assert!(layer.take_program_job().is_none());
    assert_eq!(layer.revision().to_json(), revision);
    assert_eq!(layer.store().log.len(), journal);
    let one = Budget { steps: 1, ..B14 };
    let receipt = layer.invoke(&Caller::Shell, envelope(&layer, "c2", TOOL, Some(one)));
    assert!(matches!(receipt.status, Status::Running(1)), "{:?}", receipt.status);
}

/// BM3 (draft): every profile's ceiling is the draft ceiling, and a host ceiling lowers it in each dimension and raises
/// none.
#[test]
fn the_lower_ceiling_applies_in_each_dimension() {
    for profile in Profile::ALL {
        assert_eq!(profile.budget_ceiling(), Budget::DRAFT_CEILING, "{}", profile.name());
        let mut layer = session(profile);
        assert_eq!(layer.budget_ceiling(), Budget::DRAFT_CEILING);
        let high = Budget { steps: u128::MAX, kernel_cost: u128::MAX, memory: u128::MAX, output: u128::MAX };
        layer.set_budget_ceiling(high);
        assert_eq!(layer.budget_ceiling(), Budget::DRAFT_CEILING, "a host ceiling raises nothing");
        let mixed = Budget { steps: 10, memory: u128::MAX, ..Budget::DRAFT_CEILING };
        layer.set_budget_ceiling(mixed);
        assert_eq!(layer.budget_ceiling(), Budget { steps: 10, ..Budget::DRAFT_CEILING });
    }
    assert_eq!(
        Budget::DRAFT_CEILING,
        Budget { steps: 1 << 20, kernel_cost: 1 << 40, memory: 1 << 32, output: 1 << 26 },
        "c600-tools' tool::LIMITS"
    );
}

/// A budget at the ceiling is admitted; one over it in any dimension is refused, and the refusal names the first such
/// dimension in the meter's order (steps, kernel cost, memory, output).
#[test]
fn each_dimension_is_checked_in_the_meters_order() {
    let ceiling = Budget::DRAFT_CEILING;
    let mut layer = session(Profile::Solving);
    let receipt = layer.invoke(&Caller::Shell, envelope(&layer, "at", TOOL, Some(ceiling)));
    assert!(matches!(receipt.status, Status::Running(_)), "{:?}", receipt.status);
    let cases = [
        ("steps", Budget { steps: ceiling.steps + 1, ..ceiling }, ceiling.steps),
        ("kernel_cost", Budget { kernel_cost: ceiling.kernel_cost + 1, ..ceiling }, ceiling.kernel_cost),
        ("memory", Budget { memory: ceiling.memory + 1, ..ceiling }, ceiling.memory),
        ("output", Budget { output: ceiling.output + 1, ..ceiling }, ceiling.output),
        ("kernel_cost", Budget { kernel_cost: u128::MAX, output: u128::MAX, ..ceiling }, ceiling.kernel_cost),
    ];
    for (n, (name, budget, most)) in cases.into_iter().enumerate() {
        let receipt = layer.invoke(&Caller::Shell, envelope(&layer, &format!("over-{n}"), TOOL, Some(budget)));
        let (code, detail) = refusal(&receipt);
        assert_eq!(code, code::BUDGET_CEILING, "{name}");
        assert!(detail.contains(&format!("`{name}`")) && detail.ends_with(&format!(" {most}")), "{detail}");
    }
}

/// The job carries the admitted budget, and a row that runs no program reads none: an engine row admits an envelope
/// with or without one.
#[test]
fn the_job_carries_the_admitted_budget() {
    let mut layer = session(Profile::Solving);
    let receipt = layer.invoke(&Caller::Shell, envelope(&layer, "c1", TOOL, Some(B14)));
    assert!(matches!(receipt.status, Status::Running(_)), "{:?}", receipt.status);
    let job = layer.take_program_job().expect("the tool's job");
    assert_eq!(job.budget, B14);
    // The shell declares the draft ceiling by default.
    let mut shell = Shell::new(Secret::fixed(SECRET), "shell");
    let receipt = shell.invoke(&mut layer, TOOL, json::object([])).unwrap();
    assert!(matches!(receipt.status, Status::Running(_)), "{:?}", receipt.status);
    assert_eq!(layer.take_program_job().expect("the shell's job").budget, Budget::DRAFT_CEILING);
    assert_eq!(shell.prepare(&layer, "protection.orbit.set", json::object([])).unwrap().budget, None);
}

/// KN5.21: the request digest covers a declared budget, so the same envelope gets its receipt again and the command ID
/// with another budget is other content.
#[test]
fn the_request_digest_covers_a_declared_budget() {
    let mut layer = session(Profile::Solving);
    let first = layer.invoke(&Caller::Shell, envelope(&layer, "c1", TOOL, Some(B14)));
    assert!(matches!(first.status, Status::Running(_)), "{:?}", first.status);
    let again = layer.invoke(&Caller::Shell, envelope(&layer, "c1", TOOL, Some(B14)));
    assert_eq!(again.request, first.request);
    let other = layer.invoke(&Caller::Shell, envelope(&layer, "c1", TOOL, Some(Budget { steps: 3, ..B14 })));
    assert_eq!(refusal(&other).0, code::COMMAND_ID_REUSED);
    let undeclared = layer.invoke(&Caller::Shell, envelope(&layer, "c1", TOOL, None));
    assert_eq!(refusal(&undeclared).0, code::COMMAND_ID_REUSED);
}

/// A workspace row's completion keeps its job's budget in its journal entry, which reads back strictly, and a reopen
/// replays it; an entry journaled before budgets were declared reads with none.
#[test]
fn a_workspace_completion_journals_its_budget() {
    let mut layer = session(Profile::Solving);
    let receipt = layer.invoke(&Caller::Shell, envelope(&layer, "c1", STORE, Some(B14)));
    let Status::Running(job) = receipt.status else { panic!("{:?}", receipt.status) };
    layer.take_program_job().expect("the store's job");
    let done = layer.finish_program_job(job, Ok(Value::Null)).expect("a receipt");
    assert!(matches!(done.status, Status::Completed(_)), "{:?}", done.status);
    let stored = layer.store().log.last().expect("the completion").clone();
    assert_eq!(stored.budget, Some(B14));
    let entry = stored.to_json();
    assert_eq!(entry.get("budget"), Some(&B14.to_json()));
    assert_eq!(Transaction::from_json(&entry), Some(stored.clone()));
    assert_eq!(
        B14.to_json(),
        json::object([
            ("steps", Value::String("2".into())),
            ("kernel_cost", Value::String("10000".into())),
            ("memory", Value::String("65536".into())),
            ("output", Value::String("65536".into())),
        ])
    );

    // An entry without the field reads with none; one whose budget is not in canonical form does not read.
    let Value::Object(mut fields) = entry.clone() else { panic!("an entry is an object") };
    fields.remove("budget");
    let older = Transaction::from_json(&Value::Object(fields.clone())).expect("an entry without a budget");
    assert_eq!(older.budget, None);
    for bad in [
        json::object([("steps", Value::Int(2))]),
        json::object([("steps", Value::String("02".into())), ("kernel_cost", Value::String("1".into()))]),
    ] {
        fields.insert("budget".into(), bad);
        assert_eq!(Transaction::from_json(&Value::Object(fields.clone())), None);
    }
    let mut leading = B14.to_json();
    if let Value::Object(dimensions) = &mut leading {
        dimensions.insert("steps".into(), Value::String("02".into()));
    }
    fields.insert("budget".into(), leading);
    assert_eq!(Transaction::from_json(&Value::Object(fields)), None);

    // A reopen replays the completion with its budget.
    let mut reopened =
        CommandLayer::open(kernel(), Table::h08().with_drafts(), Secret::fixed(SECRET), layer.store().clone()).unwrap();
    rows(&mut reopened);
    assert_eq!(reopened.recovery(), None, "the replay agrees with the journal");
    assert_eq!(reopened.store().log.last().map(|t| t.budget), Some(Some(B14)));
}

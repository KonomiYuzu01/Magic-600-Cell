//! CT-RTE-07's B13 and B14 (KN7.7, BM3, DET.27, ERR.27; LX-3's frozen corpus `lx3-neutral-v2`, `budgets.json`) on the
//! product's route: the shell invokes the 600-cell-Full tool's row, [`TOOL_ROW`], and the command layer admits the
//! budget the invocation declares before a job exists (LP2's pull request 348).
//!
//! - B13: the shell declares no budget. The layer refuses the command with `KN7:input.budget_not_declared`.
//! - B14: the host's ceiling allows one step, and the shell declares the corpus's budget of two. The layer refuses the
//!   command with `KN7:resource_limit.budget_ceiling`.
//!
//! In both, the session is in the corpus's profile, tool authoring. The receipt names no job, the layer queues none,
//! and [`invoke_tool`] gets the refusal, so the tool does not run: no value, no use and no coverage, as the corpus
//! expects of a stop before admission. The session keeps the fields the corpus lists as unchanged where the command
//! layer holds them: the labels, the revision, the journal (the whole store), the protected slots and the pending
//! preview. It also stays out of read-only recovery. The preferences and the recovery checkpoint belong to the session
//! store (M6); the command layer holds neither and writes nothing durable but through its store. A command that then
//! declares a budget at the ceiling is admitted as job 1: the refusals issued no job ID.
//!
//! The corpus's run is an own tool's. On the product's route the built-in tool's entry serves the row, since no own
//! tool can be invoked yet; the layer admits a budget the same way for every row an entry serves. LP2's
//! `c600-command/tests/budget.rs` checks the same keys on a synthetic row. B10 and B11 are in `ct_rte_07_c5.rs`; the
//! rest of the 600-cell-Full subset is in `c600-module/tests/ct_rte_07_full.rs`.
//!
//! Evidence kind: source and synthetic.

mod process_support;

use c600_command::{Budget, CommandLayer, MemoryStore, Profile, Shell, Status, Table};
use c600_digest::sha256;
use c600_kernel::json::Value;
use c600_kernel::ProtectedSet;
use c600_run::provenance::{self, hex, Json};
use c600_tools::process::{invoke_tool, serve_tool, tool_args, TOOL_ROW};
use c600_tools::tool::Tool;

use process_support::{kernel, secret};

const CORPUS: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/../../work/experiments/language-lx/lx3-sandbox-flow/corpus/");

/// A host ceiling that limits no dimension.
const UNLIMITED: Budget = Budget { steps: u128::MAX, kernel_cost: u128::MAX, memory: u128::MAX, output: u128::MAX };

/// A budget key's case in the frozen `budgets.json`.
struct Case {
    /// The profile of the corpus's run.
    profile: Profile,
    /// The budget the run's command declares, if any.
    declared: Option<Budget>,
    /// The host's ceiling, if any, in the dimensions the case names.
    ceiling: Option<Budget>,
    /// The refusal's code.
    code: String,
    /// The session's fields that the refusal leaves as they were.
    unchanged: Vec<String>,
}

/// The amounts a budget object of the corpus names, by dimension.
fn amounts(object: &Json) -> Vec<(String, u128)> {
    let Json::Object(fields) = object else { panic!("a budget: {object:?}") };
    let amount = |(name, amount): (&String, &Json)| match amount {
        Json::Int(amount) => (name.clone(), u128::try_from(*amount).expect("a natural amount")),
        other => panic!("{name}: {other:?}"),
    };
    fields.iter().map(amount).collect()
}

/// `base` with each of `amounts` in its dimension.
fn with(base: Budget, amounts: &[(String, u128)]) -> Budget {
    let mut budget = base;
    for (name, amount) in amounts {
        let dimension = match name.as_str() {
            "steps" => &mut budget.steps,
            "kernel_cost" => &mut budget.kernel_cost,
            "memory" => &mut budget.memory,
            "output" => &mut budget.output,
            other => panic!("not a budget dimension: {other}"),
        };
        *dimension = *amount;
    }
    budget
}

fn case(id: &str) -> Case {
    let read = |name: &str| std::fs::read(format!("{CORPUS}{name}")).unwrap();
    let bytes = read("budgets.json");
    let freeze = provenance::parse(&read("FREEZE.json")).expect("FREEZE.json reads");
    let frozen = freeze.get("files").and_then(|f| f.get("budgets.json")).and_then(Json::as_text);
    assert_eq!(frozen, Some(hex(&sha256(&bytes)).as_str()), "budgets.json is the frozen one");
    let json = provenance::parse(&bytes).expect("budgets.json reads");
    let Some(Json::List(cases)) = json.get("cases") else { panic!("the cases") };
    let case = cases.iter().find(|c| c.get("id").and_then(Json::as_text) == Some(id)).expect("the case");
    let text = |o: &Json, k: &str| o.get(k).and_then(Json::as_text).unwrap_or_else(|| panic!("{k}")).to_owned();
    let Some(Json::List(runs)) = case.get("program").and_then(|p| p.get("runs")) else { panic!("its runs") };
    let [run] = runs.as_slice() else { panic!("one run: {runs:?}") };
    assert_eq!(text(run, "invocation"), "command", "the solver's command invokes the run");
    let profile = Profile::from_name(&text(run, "profile")).expect("a profile of the session");
    // A stop before admission: no value, no use and no coverage.
    let expected = case.get("expected").expect("its expectation");
    assert_eq!(text(expected, "outcome"), "refused");
    assert_eq!(text(expected, "stop"), "before admission");
    assert_eq!(expected.get("value"), Some(&Json::Null), "no value");
    let used = amounts(expected.get("used").expect("its use"));
    assert!(used.len() == 4 && used.iter().all(|(_, amount)| *amount == 0), "it uses nothing: {used:?}");
    let coverage = expected.get("coverage").expect("its coverage");
    assert_eq!(coverage.get("examined"), Some(&Json::List(Vec::new())), "it examines nothing");
    let declared = match case.get("limits").expect("its limits") {
        Json::Null => None,
        limits => {
            let named = amounts(limits);
            assert_eq!(named.len(), 4, "the limits name every dimension");
            Some(with(UNLIMITED, &named))
        }
    };
    let ceiling = case.get("host").expect("its host").get("ceiling").map(|c| with(UNLIMITED, &amounts(c)));
    let Some(Json::List(unchanged)) = case.get("unchanged") else { panic!("what it leaves unchanged") };
    let unchanged = unchanged.iter().map(|key| key.as_text().expect("a field").to_owned()).collect();
    Case { profile, declared, ceiling, code: text(expected, "code"), unchanged }
}

/// The session's fields that the corpus's `unchanged` keys name, where the command layer holds them.
struct Fields {
    labels: Vec<i32>,
    digest: String,
    revision: Value,
    store: String,
    protected: ProtectedSet,
    preview: bool,
}

fn fields(layer: &CommandLayer) -> Fields {
    Fields {
        labels: layer.snapshot().labels().to_vec(),
        digest: layer.snapshot().digest().to_owned(),
        revision: layer.revision().to_json(),
        store: format!("{:?}", layer.store()),
        protected: layer.protected().clone(),
        preview: layer.preview().is_some(),
    }
}

/// Asserts that `after` keeps each of `keys` as `before` had it.
fn kept(keys: &[String], before: &Fields, after: &Fields) {
    for key in keys {
        let same = match key.as_str() {
            "session.labels" => after.labels == before.labels && after.digest == before.digest,
            "session.revision" => after.revision == before.revision,
            "session.journal" => after.store == before.store,
            "session.protected_slots" => after.protected == before.protected,
            "pending_preview" => after.preview == before.preview,
            // The session store's (M6): the command layer holds neither and writes nothing durable but through its
            // store, which the journal's check compares whole.
            "session.preferences" | "session.recovery_checkpoint" => true,
            other => panic!("{other} is not a field of the session"),
        };
        assert!(same, "{key} changed");
    }
}

/// Runs the budget key `id` on the tool's row: the command is refused before admission with the corpus's code, the
/// tool does not run, the session keeps what the corpus lists, and a command that then declares a budget at the
/// ceiling is admitted as job 1.
fn refused_before_admission(id: &str) {
    let case = case(id);
    let tool = Tool::full_600_cell().expect("the 600-cell-Full tool");
    let kernel = kernel();
    let solved = kernel.solved();
    let (table, store, protected) = (Table::h08(), MemoryStore::default(), ProtectedSet::none());
    let mut layer =
        CommandLayer::new_with_profile(kernel, table, secret(), store, "budgets", solved, protected, case.profile);
    serve_tool(&mut layer, &tool).expect("the tool's row is served");
    if let Some(ceiling) = case.ceiling {
        layer.set_budget_ceiling(ceiling);
    }
    let mut shell = Shell::new(secret(), "shell");
    shell.budget = case.declared;
    let word = [1, -2];
    let before = fields(&layer);

    let receipt = shell.invoke(&mut layer, TOOL_ROW, tool_args(&word)).expect("the shell's invocation");
    let Status::Refused(error) = &receipt.status else { panic!("{id}: {receipt:?}") };
    assert_eq!(error.code, case.code, "{id}: {}", error.detail);
    assert_eq!(receipt.job, None, "{id}: the refusal names no job");
    assert!(layer.take_program_job().is_none(), "{id}: the layer queues no job");
    // The tool's command meets the same refusal, so the tool does not run.
    let refused = invoke_tool(&mut shell, &mut layer, &tool, &word).expect_err("the tool does not run");
    assert!(refused.contains(&case.code), "{id}: {refused}");
    assert!(layer.take_program_job().is_none(), "{id}: the layer queues no job");

    kept(&case.unchanged, &before, &fields(&layer));
    assert!(layer.recovery().is_none(), "{id}: the session is not in read-only recovery");

    // A budget at the ceiling is admitted, as job 1: the refusals issued no job ID.
    shell.budget = Some(layer.budget_ceiling());
    let command = invoke_tool(&mut shell, &mut layer, &tool, &word).expect("the tool's command");
    assert_eq!((command.job.job, command.job.budget), (1, layer.budget_ceiling()), "{id}");
}

/// B13 (KN7.7, DET.27, ERR.27): the shell declares no budget.
#[test]
fn ct_rte_07_full_b13_missing_declaration() {
    refused_before_admission("B13");
}

/// B14 (BM3, DET.27, ERR.27): under the host's ceiling of one step, the shell declares the corpus's budget of two.
#[test]
fn ct_rte_07_full_b14_host_ceiling() {
    refused_before_admission("B14");
}

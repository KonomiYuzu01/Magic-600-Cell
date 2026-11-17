//! The host bridge from `c600-run` onto `c600-command` (LP3, C5, acceptance item 3). The 600-cell-Full tool runs on
//! `c600-run` through `Boundary` in the solving profile; its proposal reaches the command layer as the runtime's
//! `operation.preview` invocation. Each test names the conformance parts it covers; none of those IDs runs whole here.

use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use c600_command::{
    code, draft_args, Caller, CommandLayer, HeadlessFrames, Invocation, Kind, PreviewState, Secret, Shell, Status,
    Table, Token,
};
use c600_digest::sha256_hex;
use c600_ir::{ExactRole, QTy, Ty};
use c600_kernel::json::{self, Value as Json};
use c600_kernel::{Kernel, ProtectedSet};
use c600_run::boundary::{Answer, Pins, Store};
use c600_run::interp::{Host, Request, Sink};
use c600_run::meter::code::NOT_GRANTED;
use c600_run::meter::Outcome;
use c600_run::provenance::INCOMPATIBLE;
use c600_run::value::{equal, Value};
use c600_tools::bridge::{base, word_args, Bridge};
use c600_tools::tool::{manifest, Manifest, Tool, ROW, SOURCE};

const SESSION: &str = "bridge";

fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..")
}

fn kernel() -> Arc<Kernel> {
    static KERNEL: OnceLock<Arc<Kernel>> = OnceLock::new();
    let kernel = KERNEL.get_or_init(|| match Kernel::load(&repository().join("assets")) {
        Ok(kernel) => Arc::new(kernel),
        Err(error) => panic!("the model in assets/ loads: {error}"),
    });
    kernel.clone()
}

/// A layer at the solved state with no protected orbit, and its shell, sharing one fixed secret.
fn session() -> (CommandLayer, Shell) {
    let kernel = kernel();
    let secret = Secret::fixed([5; 32]);
    let solved = kernel.solved();
    let none = ProtectedSet::none();
    let layer = CommandLayer::new(kernel, Table::h08(), secret.clone(), Default::default(), SESSION, solved, none);
    (layer, Shell::new(secret, "shell"))
}

/// The program's whole output: the fixed acknowledgement of its one proposal.
fn acknowledgement(base: &str, run: &str) -> Vec<u8> {
    format!("{{\"base\":\"{base}\",\"id\":\"{run}/1\"}}\n").into_bytes()
}

/// The head digest, the head event, the state revision and the number of durable transactions.
fn fingerprint(layer: &CommandLayer) -> (String, String, u64, usize) {
    let revision = layer.revision();
    (layer.snapshot().digest().to_owned(), revision.head.clone(), revision.state, layer.store().log.len())
}

const TO_EXECUTE: &[&str] = &["operation.execute"];
const TO_EXIT: &[&str] = &["app.exit"];

/// The tool with `rows[0]` as its proposal's target, loaded with a manifest that names that row.
fn targeting(rows: &'static [&'static str]) -> Tool {
    let source = SOURCE.replace("target(\"operation.preview\")", &format!("target(\"{}\")", rows[0]));
    assert_ne!(source, SOURCE);
    let manifest = Manifest { source_sha256: sha256_hex(source.as_bytes()), rows, ..manifest() };
    Tool::new(&source, manifest).unwrap_or_else(|error| panic!("the variant targeting {rows:?} loads: {error}"))
}

/// Runs `tool` on `word` as run `run` through a bridge onto `layer`; returns the run and the bridge's invocations.
fn run_on(
    tool: &Tool,
    layer: &mut CommandLayer,
    word: &[i32],
    run: &str,
    pins: Option<Pins>,
) -> (c600_tools::tool::Ran, Vec<c600_tools::bridge::Sent>) {
    let mut context = tool.context(run, &base(layer.revision()));
    if let Some(pins) = pins {
        context.pins = pins;
    }
    let mut bridge = Bridge::new(layer, run);
    let ran = tool.run_in(word, context, &mut bridge, &mut Store::default()).expect("the run starts");
    (ran, bridge.sent)
}

/// Acceptance item 3, the bridge. The tool's proposal becomes one invocation of `operation.preview` with
/// `Caller::Runtime` for the run, the fresh command ID `<run>/1`, the layer's current epoch, the literal word as the
/// absolute argument `{"draft": {"recipe": [{"kind": "word", "moves": [...]}]}}` and no token. The program receives only
/// the fixed acknowledgement whether the layer admits the preview, the layer refuses the word, or preview admission
/// refuses the proposal before the layer sees it; the charges are the same, and only the shell sees the layer's answer.
/// A preview changes no state, revision or journal.
///
/// CT-BND-11 (part, the 600-cell-Full tool on the command layer: in the solving profile its preview outcome is withheld)
/// and CT-BND-07 (part, the 600-cell-Full tool on the command layer: whatever the engine answers, the program gets the
/// fixed acknowledgement).
#[test]
fn each_proposal_is_one_runtime_preview_and_the_program_gets_the_fixed_acknowledgement() {
    let tool = Tool::full_600_cell().unwrap();
    let mut summaries = Vec::new();
    for (case, word, run) in [
        ("admitted", [1, -2], "run-a"),
        ("refused by the layer", [0, -2], "run-b"),
        ("refused at admission", [1, -2], "run-c"),
    ] {
        let (mut layer, _) = session();
        let read = base(layer.revision());
        assert_eq!(read, "bridge@1/e0");
        let before = fingerprint(&layer);
        let pins = (case == "refused at admission").then(Pins::default);
        let (ran, sent) = run_on(&tool, &mut layer, &word, run, pins);
        assert_eq!(ran.run.summary.outcome, Outcome::Completed, "{case}");
        assert_eq!(ran.run.output.as_deref(), Some(&acknowledgement(&read, run)[..]), "{case}");
        let [shown] = &ran.shell[..] else { panic!("{case}: one proposal") };
        assert_eq!((shown.id.clone(), &shown.word[..], shown.target.as_str()), (format!("{run}/1"), &word[..], ROW));
        if case == "refused at admission" {
            assert!(sent.is_empty(), "the layer never sees a proposal that preview admission refused");
            assert!(matches!(&shown.answer, Answer::Refused(r) if r.code == INCOMPATIBLE), "{:?}", shown.answer);
        } else {
            let [one] = &sent[..] else { panic!("{case}: one invocation") };
            let invocation = &one.invocation;
            assert_eq!((invocation.command.as_str(), invocation.row.as_str()), (&*format!("{run}/1"), ROW), "{case}");
            assert_eq!(one.caller, Caller::Runtime { run: run.into() }, "{case}");
            assert_eq!(invocation.epoch, layer.revision().epoch, "{case}");
            assert!(invocation.token.is_none(), "{case}");
            assert_eq!(invocation.args.canonical(), word_args(&word).canonical(), "{case}");
            let moves = r#"{"draft":{"recipe":[{"kind":"word","moves":["#;
            assert!(invocation.args.canonical().starts_with(moves), "{case}");
            match (case, &one.receipt.status, &shown.answer) {
                ("admitted", Status::Running(job), Answer::Previewed(Value::Receipt(answer))) => {
                    assert_eq!(layer.preview_state(*job), Some(PreviewState::Building));
                    assert!(equal(answer.get("status").unwrap(), &Value::tag("running")));
                    assert!(equal(answer.get("id").unwrap(), &Value::text(&format!("{run}/1"))));
                    assert!(equal(answer.get("job").unwrap(), &Value::int(*job)));
                }
                ("refused by the layer", Status::Refused(error), Answer::Refused(r)) => {
                    assert_eq!((error.code, r.code), (c600_kernel::codes::RECIPE_SHAPE, error.code));
                }
                (case, status, answer) => panic!("{case}: {status:?} {answer:?}"),
            }
        }
        assert_eq!(fingerprint(&layer), before, "{case}: a preview changes no state");
        summaries.push(ran.run.summary);
    }
    assert!(summaries.iter().all(|summary| summary == &summaries[0]), "the charges do not depend on the answer");
}

/// ADR-L02 and KN5.15: the runtime reaches no commit or app row. Every commit and app row of H-08, invoked by the
/// runtime through the bridge, is refused with `KN5:capability.commit_row_refused` before its arguments or token are
/// read. A variant of the tool whose proposal targets `operation.execute` or `app.exit` loads (the built-in column
/// does not judge the row) but the layer refuses its proposal: the program still gets the fixed acknowledgement, the
/// shell sees the refusal, and nothing changes.
#[test]
fn the_runtime_reaches_no_commit_or_app_row() {
    let (mut layer, _) = session();
    let before = fingerprint(&layer);
    let table = Table::h08();
    let rows: Vec<String> =
        table.rows().filter(|row| matches!(row.kind, Kind::Commit | Kind::App)).map(|row| row.id.clone()).collect();
    for row in ["operation.execute", "session.undo", "session.redo", "app.exit", "twist.apply"] {
        assert!(rows.iter().any(|id| id == row), "{row}");
    }
    let mut bridge = Bridge::new(&mut layer, "rows");
    for row in &rows {
        let receipt = bridge.invoke(row, json::object([("preview", Json::Int(1))]), None);
        assert_eq!(receipt.status.code(), Some(code::COMMIT_ROW_REFUSED), "{row}");
        assert_eq!(receipt.job, None, "{row}");
    }
    assert_eq!(bridge.sent.len(), rows.len());
    assert_eq!(fingerprint(&layer), before);

    for (rows, run) in [(TO_EXECUTE, "to-execute"), (TO_EXIT, "to-exit")] {
        let (tool, row) = (targeting(rows), rows[0]);
        assert_eq!(tool.manifest.rows, rows);
        let read = base(layer.revision());
        let (ran, sent) = run_on(&tool, &mut layer, &[1, -2], run, None);
        assert_eq!(ran.run.output.as_deref(), Some(&acknowledgement(&read, run)[..]), "{row}");
        let [one] = &sent[..] else { panic!("{row}: one invocation") };
        assert_eq!((one.invocation.row.as_str(), one.receipt.status.code()), (row, Some(code::COMMIT_ROW_REFUSED)));
        let [shown] = &ran.shell[..] else { panic!("{row}: one proposal") };
        assert!(matches!(&shown.answer, Answer::Refused(r) if r.code == code::COMMIT_ROW_REFUSED), "{row}");
        assert_eq!(fingerprint(&layer), before, "{row}");
    }
}

/// KN5.15: the runtime never holds a token. A runtime invocation carrying a genuine token the shell minted for the
/// presented preview is refused, `KN5:auth.token_invalid` on `operation.preview` and
/// `KN5:capability.commit_row_refused` on `operation.execute`, and neither attempt spends the token: the shell's own
/// invocation then commits.
#[test]
fn a_runtime_invocation_carrying_a_token_is_refused() {
    let (mut layer, mut shell) = session();
    let recipe = Json::Array(vec![json::object([
        ("kind", Json::String("word".into())),
        ("moves", Json::Array(vec![Json::Int(1), Json::Int(-2)])),
    ])]);
    let admitted = shell.invoke(&mut layer, "operation.preview", draft_args(&recipe)).unwrap();
    let Status::Running(id) = admitted.status else { panic!("{admitted:?}") };
    assert_eq!(layer.run_jobs(), 1);
    HeadlessFrames::after(&layer).present(&mut shell, &mut layer, id).unwrap();
    let before = fingerprint(&layer);
    let prepared =
        shell.prepare(&layer, "operation.execute", json::object([("preview", Json::Int(id as i64))])).unwrap();
    let token = prepared.token.clone().expect("the shell mints a token for a commit");

    let mut bridge = Bridge::new(&mut layer, "holder");
    let preview = bridge.invoke(ROW, word_args(&[1, -2]), Some(token.clone()));
    assert_eq!(preview.status.code(), Some(code::TOKEN_INVALID));
    let execute = bridge.invoke("operation.execute", prepared.args.clone(), Some(token));
    assert_eq!(execute.status.code(), Some(code::COMMIT_ROW_REFUSED));
    assert_eq!(fingerprint(&layer), before);
    assert_eq!(layer.preview_state(id), Some(PreviewState::Presented));

    let committed = shell.send(&mut layer, prepared);
    assert!(matches!(committed.status, Status::Committed(_)), "{committed:?}");
}

/// Acceptance item 3: only a token the shell mints commits the pending preview, which here the runtime proposed. The
/// shell mints no commit token before the built-in chain presents the preview. After it is presented, the runtime's
/// `operation.execute` is refused, and so are a shell invocation with no token, a token minted with another secret and
/// a genuine token whose binding was changed; none changes the state, the revision or the journal. The shell's commit
/// chord then commits exactly the preview: its pre- and post-state are the preview's.
///
/// CT-ONE-29 (part, headless: the shell binds a commit token only to a preview the built-in chain presented; no user
/// stage is drawn here).
#[test]
fn only_a_token_the_shell_mints_commits_the_pending_preview() {
    let tool = Tool::full_600_cell().unwrap();
    let (mut layer, mut shell) = session();
    let (ran, sent) = run_on(&tool, &mut layer, &[1, -2], "proposer", None);
    assert_eq!(ran.run.summary.outcome, Outcome::Completed);
    let Status::Running(id) = sent[0].receipt.status else { panic!("{:?}", sent[0].receipt) };
    assert_eq!(layer.run_jobs(), 1);
    let before = fingerprint(&layer);

    let unpresented = shell.press(&mut layer, "Ctrl+Shift+Enter", false);
    assert_eq!(unpresented.err().map(|error| error.code), Some(code::NO_FRESH_PREVIEW));
    HeadlessFrames::after(&layer).present(&mut shell, &mut layer, id).unwrap();
    let preview = layer.preview().cloned().expect("the pending preview");
    assert_eq!((preview.id, &preview.caller), (id, &Caller::Runtime { run: "proposer".into() }));

    let args = json::object([("preview", Json::Int(id as i64))]);
    let mut bridge = Bridge::new(&mut layer, "committer");
    assert_eq!(bridge.invoke("operation.execute", args.clone(), None).status.code(), Some(code::COMMIT_ROW_REFUSED));
    let epoch = layer.revision().epoch;
    let bare = Invocation {
        command: shell.command_id(),
        row: "operation.execute".into(),
        args: args.clone(),
        token: None,
        epoch,
    };
    assert_eq!(layer.invoke(&Caller::Shell, bare).status.code(), Some(code::TOKEN_INVALID));
    let genuine = shell.prepare(&layer, "operation.execute", args.clone()).unwrap();
    let mut other = genuine.clone();
    other.token = Some(Token::mint(&Secret::fixed([6; 32]), genuine.token.clone().unwrap().binding));
    assert_eq!(layer.invoke(&Caller::Shell, other).status.code(), Some(code::TOKEN_INVALID));
    let mut forged = shell.prepare(&layer, "operation.execute", args).unwrap();
    let mut token = forged.token.take().unwrap();
    token.binding.nonce += 1000;
    forged.token = Some(token);
    assert_eq!(layer.invoke(&Caller::Shell, forged).status.code(), Some(code::TOKEN_INVALID));
    assert_eq!(fingerprint(&layer), before);
    assert_eq!(layer.preview_state(id), Some(PreviewState::Presented));

    let receipt = shell.press(&mut layer, "Ctrl+Shift+Enter", false).unwrap().expect("not a repeat");
    let Status::Committed(record) = receipt.status else { panic!("{receipt:?}") };
    assert_eq!(record.pre, preview.base.digest());
    assert_eq!(Some(record.post.as_str()), preview.end.as_ref().map(|end| end.digest()));
    assert_eq!(layer.snapshot().digest(), record.post);
    assert_eq!(layer.preview_state(id), Some(PreviewState::Committed));
}

/// The bridge grants a run nothing but proposals: no state read, no declassified value, no workspace or file read or
/// write, no view, no exact sink and no clock.
#[test]
fn the_bridge_grants_nothing_but_a_proposal() {
    let (mut layer, _) = session();
    let before = fingerprint(&layer);
    let mut bridge = Bridge::new(&mut layer, "nothing");
    let (ty, qty) = (Ty::Word, QTy { ty: Ty::Word, display: false });
    let requests = [
        Request::Snapshot { query: "best_word", ty: &qty },
        Request::Declassified { id: "best_word", ty: &ty },
        Request::FileRead { path: "notes.txt", ty: &ty },
        Request::Clock,
    ];
    for request in requests {
        assert_eq!(bridge.tariff(&request).err(), Some(NOT_GRANTED), "{request:?}");
        assert_eq!(bridge.perform(&request).err(), Some(NOT_GRANTED), "{request:?}");
    }
    let sinks = [
        Sink::View,
        Sink::Workspace { record: "best_word", ty: &qty },
        Sink::File { path: "notes.txt" },
        Sink::Exact { role: ExactRole::Evidence },
    ];
    for sink in sinks {
        assert_eq!(bridge.sink(&sink, &Value::word(&[1, -2]), b"[1,-2]").err(), Some(NOT_GRANTED), "{sink:?}");
    }
    assert!(bridge.sent.is_empty());
    assert_eq!(fingerprint(&layer), before);
}

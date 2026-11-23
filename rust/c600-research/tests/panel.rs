//! LP5 piece R10: the research output panel's rows, driven headless through the command layer.
//!
//! The driver runs the panel's flow as a shell would: it reads the palette, invokes each row through `Shell::invoke`,
//! lets the research host run the row's program job, and reads the answer with its D2 state. Every run uses fresh
//! data: a new session in memory, the restoration corpus's members read in place (read-only) through the host's file
//! grant, and the written files in a fresh folder under the test's scratch directory.
//!
//! R10 has no indexed conformance test (LP5 plan, section 5), so no test here names a CT ID (PHS.62). What is not
//! run: the panel's look, layout, text and motion (ADR-D02, K2); a commit's cancellation of a queued row (a research
//! session has no commit route, since `operation.preview` is the solving profile's); `job.stop` and
//! `session.branch.select`, which the command layer does not build here; Lean, which no test runs (LP5-2).

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::OnceLock;

use c600_archive::bindings::Bindings;
use c600_checkers::engines::Loaded;
use c600_command::{
    code, CommandLayer, Kind, Lane, MemoryStore, PreviewRule, Profile, Receipt, Secret, Shell, Status, UndoClass,
};
use c600_research::panel::{self, invoke, Grant, Host, Read, D2, H08_ROWS, MODULE, ROWS};
use c600_session::persist::codec::Json;

const MANIFEST: &str = "work/experiments/language-lx/envelope-facts/MANIFEST.json";
const R01: &str = "research/jumbling/restoration/15-lowering-machine/results/certificates/R01.json";
const R01_CHECK: &str = "research/jumbling/restoration/15-lowering-machine/results/certificates/R01-check.json";
const STOP: &str = "research/jumbling/restoration/15-lowering-machine/results/runs/s7305-pstar.json";
const JAMS: &str = "research/jumbling/restoration/16-six-jams/jams.json";
/// R01's start table: stage 13's checkpoint table, outside the corpus, granted to the host.
const TABLE: &str =
    "research/jumbling/restoration/13-tutorial-runner/runs/dev/d2-s7302/component-checkpoint/tables/00021.json";
/// The start digest R01 states (`table_digest`).
const R01_TABLE_DIGEST: &str = "2430f4cc99ea121a6d3476ec21b66fc401e277d13809ad5e1c979e6e0abe1472";
const R01_GROUP: &str = "15/macro:research/jumbling/restoration/15-lowering-machine/results/certificates/R01.json#";
const Q11: &str = "19/q11:research/jumbling/restoration/19-height-depth-framework/records/\
                   type-lex-trimmed-independent-results.json#/q11_depth_certificate";
const GLOBAL_OPEN: &str =
    "19/global_open:research/jumbling/restoration/19-height-depth-framework/README.md#Remaining obligations (prose)";
/// The I_a menu record, written fresh into the scratch folder and granted under this name.
const MENU: &str = "fixtures/wj-I_a.menu.json";
/// The format packet's supported obligation body, written fresh and granted under this name.
const SUPPORTED: &str = "obligations/supported-statement.json";
const BUNDLE_OUT: &str = "out/bundle.zip";
const LEAN_OUT: &str = "out/obligation.lean";
const REFUSED_OUT: &str = "out/refused.lean";

fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read(relative: &str) -> Vec<u8> {
    std::fs::read(repository().join(relative)).unwrap_or_else(|e| panic!("{relative}: {e}"))
}

fn loaded() -> &'static Loaded {
    static LOADED: OnceLock<Loaded> = OnceLock::new();
    LOADED.get_or_init(|| Loaded::load(&repository().join("assets")).expect("the model in assets/ loads"))
}

/// A fresh, empty folder under the test's scratch directory.
fn scratch(tag: &str) -> PathBuf {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let folder = Path::new(env!("CARGO_TARGET_TMPDIR")).join("r10-panel").join(format!(
        "{tag}-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::SeqCst)
    ));
    let _ = std::fs::remove_dir_all(&folder);
    std::fs::create_dir_all(&folder).expect("a scratch folder");
    folder
}

// ------------------------------------------------------------------------------------------------------- JSON

fn t(text: &str) -> Json {
    Json::text(text)
}

fn obj(fields: &[(&str, Json)]) -> Json {
    Json::Object(fields.iter().map(|(name, value)| ((*name).to_owned(), value.clone())).collect())
}

fn texts(items: &[&str]) -> Json {
    Json::List(items.iter().map(|item| t(item)).collect())
}

fn at<'a>(value: &'a Json, path: &[&str]) -> &'a Json {
    path.iter().fold(value, |value, key| value.get(key).unwrap_or_else(|| panic!("no `{key}` in {value:?}")))
}

fn text_at<'a>(value: &'a Json, path: &[&str]) -> &'a str {
    at(value, path).as_text().unwrap_or_else(|| panic!("{path:?} is not text"))
}

fn items(value: &Json) -> &[Json] {
    match value {
        Json::List(items) => items,
        other => panic!("not a list: {other:?}"),
    }
}

// ---------------------------------------------------------------------------------------------------- the panel

/// A research session on the panel's table, with the tools' provenance reader, the shell and the research host.
struct Panel {
    layer: CommandLayer<MemoryStore>,
    shell: Shell,
    host: Host<'static>,
    folder: PathBuf,
}

fn layer(profile: Profile) -> CommandLayer<MemoryStore> {
    let kernel = loaded().kernel().clone();
    let solved = kernel.solved();
    let none = kernel.protected_set(&[]).expect("no protected orbit");
    let secret = Secret::fixed([7; 32]);
    CommandLayer::new_with_profile(kernel, panel::table(), secret, MemoryStore::default(), "r10", solved, none, profile)
}

/// The grant: the corpus manifest and four of its members, every stored input of R01's claim group, the I_a menu and
/// the supported obligation body for reading, and two files for writing.
fn grant(folder: &Path) -> Grant {
    let menu = {
        let fixture = read("research/jumbling/fixtures/wj-I_a.json");
        let doc = c600_checkers::claims::document(&fixture).ok().expect("the fixture is JSON");
        doc.get("menu").expect("a menu").canonical().into_bytes()
    };
    std::fs::write(folder.join("menu.json"), menu).expect("the menu is written");
    std::fs::write(folder.join("supported.json"), supported().0).expect("the body is written");
    let mut grant = Grant::new();
    let bindings = Bindings::edition_1();
    let row = bindings.find(R01_GROUP).expect("R01's group");
    let stored: Vec<String> =
        bindings.stored(row).into_iter().map(|(file, _)| bindings.file(file).path.clone()).collect();
    for name in [MANIFEST, R01, R01_CHECK, STOP, JAMS].into_iter().chain(stored.iter().map(String::as_str)) {
        grant = grant.read(name, &repository().join(name)).expect("a repository file");
    }
    grant
        .read(MENU, &folder.join("menu.json"))
        .and_then(|grant| grant.read(SUPPORTED, &folder.join("supported.json")))
        .expect("the scratch files")
        .write(BUNDLE_OUT, &folder.join("bundle.zip"))
        .write(LEAN_OUT, &folder.join("obligation.lean"))
}

/// The format packet's `supported-statement` vector: its body's canonical bytes, its StatementId and its expected
/// Lean source and digest.
fn supported() -> (Vec<u8>, String, String, String) {
    use c600_evidence::obligation::json;
    let vectors = json::parse(&read("docs/architecture/1.0/language/format-packet/vectors.json")).expect("JSON");
    let vector = vectors
        .get("obligations")
        .and_then(json::Json::as_array)
        .and_then(|all| all.iter().find(|v| v.get("id").and_then(json::Json::as_text) == Some("supported-statement")))
        .expect("the supported vector")
        .clone();
    let given = vector.get("given").expect("given");
    let body = json::canonical(given.get("body").expect("a body")).expect("canonical");
    let text = |value: &json::Json, path: &[&str]| -> String {
        let found = path.iter().fold(value, |value, key| value.get(key).expect("a field"));
        found.as_text().expect("text").to_owned()
    };
    let id = text(given, &["statement", "id", "content", "sha256"]);
    let expected = vector.get("expected").expect("expected");
    (body, id, text(expected, &["source"]), text(expected, &["sha256"]))
}

impl Panel {
    fn new() -> Panel {
        let folder = scratch("flow");
        let mut layer = layer(Profile::Research);
        c600_tools::workspace::install(&mut layer);
        let host = Host::new(loaded(), grant(&folder));
        host.serve(&mut layer).expect("a research session serves the panel's rows");
        Panel { layer, shell: Shell::new(Secret::fixed([7; 32]), "panel"), host, folder }
    }

    /// A control's press: the row through the shell and the command layer, the host's run of every queued job, and
    /// the answer as the panel reads it.
    fn press(&mut self, row: &str, args: Json) -> (Receipt, Read) {
        let receipt = invoke(&mut self.shell, &mut self.layer, row, &args).expect("the shell sends the invocation");
        assert!(self.host.run_pending(&mut self.layer).is_empty(), "every queued job is the panel's");
        let read = self.host.read(&self.layer, &receipt);
        (receipt, read)
    }

    /// A press whose answer must be current (or empty), returning the answer. The answer names the revision its job
    /// read: the head when the row was pressed.
    fn answer(&mut self, row: &str, args: Json) -> (Receipt, Json) {
        let pressed = c600_session::persist::codec::parse(self.layer.revision().to_json().canonical().as_bytes());
        let (receipt, read) = self.press(row, args);
        assert!(matches!(read.state, D2::Current | D2::Empty), "{row}: {read:?}");
        let answer = read.answer.expect("an answer");
        assert_eq!(text_at(&answer, &["row"]), row);
        assert_eq!(Some(at(&answer, &["revision"]).clone()), pressed, "{row}: computed at the revision it read");
        assert_eq!(text_at(&answer, &["research"]), self.host.research());
        (receipt, at(&answer, &["answer"]).clone())
    }

    fn failed(&mut self, row: &str, args: Json) -> &'static str {
        let (_, read) = self.press(row, args);
        match read.state {
            D2::Error { code } => code,
            other => panic!("{row}: {other:?}"),
        }
    }
}

fn completion_kind(receipt: &Receipt, layer: &CommandLayer<MemoryStore>) -> &'static str {
    match layer.outcome(&receipt.command, &receipt.request).expect("an outcome").status {
        Status::Completed(completion) => completion.kind.name(),
        other => panic!("not completed: {other:?}"),
    }
}

// ------------------------------------------------------------------------------------------------------ tests

/// The panel's rows are LP5's block of the command table as the host serves it: eleven rows with the section 6
/// shapes, none a commit or app row, none taking a token, preparing a preview or having a gate; H-08 holds them as its
/// fifth delta (owner, 23 November 2026, item 3.1), so the table is H-08's rows as H-08 has them, none added twice, and
/// the H-08 rows the panel invokes are in the table.
#[test]
fn panel_rows_are_lp5s_block_of_the_command_table() {
    let table = panel::table();
    let h08 = c600_command::Table::h08();
    for row in h08.rows() {
        assert_eq!(table.row(&row.id), Some(row), "{} is H-08's", row.id);
    }
    assert_eq!(table.rows().count(), h08.rows().count());
    let delta = c600_session::persist::codec::parse(panel::DELTA.as_bytes()).expect("the block reads");
    assert_eq!(text_at(&delta, &["format"]), "c600-command-rows-delta-1");
    assert!(items(at(&delta, &["amend"])).is_empty(), "the block amends no row");
    let ids: Vec<&str> = items(at(&delta, &["add"])).iter().map(|row| text_at(row, &["id"])).collect();
    assert_eq!(ids, ROWS.iter().map(|row| row.id).collect::<Vec<_>>());
    for row in &ROWS {
        let stored = table.row(row.id).expect("the row is in the table");
        assert_eq!(h08.row(row.id), Some(stored), "{} is H-08's own row", row.id);
        assert_eq!((stored.kind, stored.lane, stored.undo), (row.kind, row.lane, row.undo), "{}", row.id);
        let arguments: Vec<(&str, &str)> = stored.arguments.iter().map(|(n, t)| (n.as_str(), t.as_str())).collect();
        assert_eq!(arguments, row.arguments, "{}", row.id);
        assert!(!matches!(row.kind, Kind::Commit | Kind::App), "{}", row.id);
        assert!(!stored.takes_token() && stored.gates.is_empty() && stored.action_guard.is_none(), "{}", row.id);
        assert_eq!(stored.preview, PreviewRule::None, "{}", row.id);
        assert_eq!(stored.job, row.kind != Kind::Query, "{}", row.id);
        assert_eq!(row.lane == Lane::Program, row.kind != Kind::Query, "{}", row.id);
        assert_eq!(row.undo, UndoClass::None, "{}", row.id);
    }
    for id in H08_ROWS {
        assert_eq!(table.row(id), h08.row(id), "{id} is H-08's");
        assert!(table.row(id).is_some(), "{id}");
    }
}

/// A solving session gets none of the panel: the host refuses to serve it with `LG3:capability.grant_mismatch` and
/// serves nothing, so the rows stay H-08's unserved rows and no job reaches the host. The research library's searching
/// entries stay its four research tier candidates; no panel row runs a library entry.
#[test]
fn a_solving_session_is_served_no_panel_row() {
    let mut layer = layer(Profile::Solving);
    let host = Host::new(loaded(), Grant::new());
    let refused = host.serve(&mut layer).expect_err("refused");
    assert_eq!(refused.code, "LG3:capability.grant_mismatch");
    for row in &ROWS {
        assert!(layer.table().row(row.id).expect("in the table").overlay.is_none(), "{} is not served", row.id);
    }
    let mut shell = Shell::new(Secret::fixed([7; 32]), "solving");
    let receipt = invoke(&mut shell, &mut layer, panel::rows::CLAIMS, &obj(&[]));
    if let Ok(receipt) = receipt {
        assert!(!matches!(receipt.status, Status::Running(_) | Status::Completed(_)), "{receipt:?}");
    }
    assert!(layer.take_program_job().is_none(), "no job reaches a host");
    let tier: Vec<&str> = c600_library::research::research_tier_rows().iter().map(|row| row.entry).collect();
    assert_eq!(
        tier,
        [
            "Restricted word search",
            "Search reductions and gates",
            "Strategies, selectors and ranks",
            "Admitted-domain membership"
        ]
    );
    assert!(ROWS.iter().all(|row| !row.id.starts_with("library.")), "no panel row is a library row");
    assert_eq!(MODULE, "research.panel");
}

/// D2's pending states: a pressed row reads `loading` until the host has run its job; a job row pressed while the
/// program lane holds another reads `busy` with the lane's code, while a read on the `none` lane still queues; then
/// both queued rows complete as current.
#[test]
fn a_queued_row_reads_loading_and_a_full_lane_reads_busy() {
    let mut p = Panel::new();
    let import = obj(&[("manifest", t(MANIFEST)), ("members", texts(&[R01]))]);
    let queued = invoke(&mut p.shell, &mut p.layer, panel::rows::IMPORT, &import).expect("sent");
    assert_eq!(p.host.read(&p.layer, &queued).state, D2::Loading { job: queued.job });
    let bundle = obj(&[("records", texts(&[R01])), ("destination", t(BUNDLE_OUT))]);
    let full = invoke(&mut p.shell, &mut p.layer, panel::rows::BUNDLE, &bundle).expect("sent");
    assert_eq!(p.host.read(&p.layer, &full).state, D2::Busy { code: code::LANE_FULL });
    let read = invoke(&mut p.shell, &mut p.layer, panel::rows::OBLIGATIONS, &obj(&[])).expect("sent");
    assert!(matches!(p.host.read(&p.layer, &read).state, D2::Loading { .. }));
    assert!(p.host.run_pending(&mut p.layer).is_empty());
    assert_eq!(p.host.read(&p.layer, &queued).state, D2::Current);
    // The read ran after the import was published, from the same research data.
    assert_eq!(p.host.read(&p.layer, &read).state, D2::Current);
    assert!(!p.folder.join("bundle.zip").exists(), "the refused export wrote nothing");
}

/// The panel's flow end to end, with fresh data: the palette; the inventory before and after an import of four granted
/// members of the corpus; the claims with LB5.14's preflight; a KN6 check job of R01's claim group; its report, the
/// group's evidence word and the runs; a reproduction; the check bundle and the obligation exports; and the stale
/// answers after the revision or the research data moved. Every answer carries the revision it was computed at.
#[test]
fn panel_driver_end_to_end() {
    let mut p = Panel::new();

    // The palette: every panel row is available once served; it grants nothing.
    let palette = panel::palette(&p.layer);
    assert_eq!(&palette.revision, p.layer.revision());
    for row in &ROWS {
        assert_eq!(palette.answer(row.id, &c600_command::Binding::Computed), Some(Ok(())), "{}", row.id);
    }
    for id in H08_ROWS {
        let answer = palette.answer(id, &c600_command::Binding::Computed);
        assert_eq!(answer, Some(Err(code::ROW_NOT_BUILT)), "{id}: the command layer does not build it here");
    }

    // Before an import the inventory is empty.
    let (early, read) = p.press(panel::rows::INVENTORY, obj(&[]));
    assert_eq!(read.state, D2::Empty);
    assert_eq!(at(read.answer.as_ref().expect("an answer"), &["answer"]), &Json::Null);

    // The import publishes the granted members; it is a file row, so the revision stays and the research data moves.
    let head = p.layer.revision().clone();
    let (receipt, imported) = p.answer(
        panel::rows::IMPORT,
        obj(&[("manifest", t(MANIFEST)), ("members", texts(&[R01, R01_CHECK, STOP, JAMS]))]),
    );
    assert_eq!(completion_kind(&receipt, &p.layer), "file");
    assert_eq!(p.layer.revision(), &head);
    assert_eq!(text_at(&imported, &["manifest", "name"]), MANIFEST);
    let stale = p.host.read(&p.layer, &early);
    assert_eq!(stale.state, D2::Stale { at: Some(head.clone()), research_moved: true });

    // An ungranted member refuses the import, which publishes nothing.
    let research = p.host.research();
    let refused =
        p.failed(panel::rows::IMPORT, obj(&[("manifest", t(MANIFEST)), ("members", texts(&["not/granted.json"]))]));
    assert_eq!(refused, code::NOT_GRANTED);
    assert_eq!(p.host.research(), research);

    // Before any check the runs list only the archive's runs as recorded, which is something to show.
    let (_, read) = p.press(panel::rows::RUNS, obj(&[]));
    assert_eq!(read.state, D2::Current, "archived runs are listed");
    let answer = read.answer.expect("an answer");
    assert!(items(at(&answer, &["answer", "rows"])).is_empty());
    assert!(!items(at(&answer, &["answer", "archived"])).is_empty());

    // The inventory of stage 15: each record with its Git blob, SHA-256, format and its integrity, recognition and
    // interpretation as separate results.
    let (_, inventory) = p.answer(panel::rows::INVENTORY, obj(&[("stage", Json::Int(15))]));
    let rows = items(at(&inventory, &["rows"]));
    assert!(rows.iter().all(|row| at(row, &["stage"]) == &Json::Int(15)));
    let r01 = rows.iter().find(|row| text_at(row, &["name"]) == R01).expect("R01 is listed");
    assert_eq!(text_at(r01, &["git_blob"]), "ec847b1c900fae35b3b3d92743b553e3c99a3a49");
    assert_eq!(text_at(r01, &["sha256"]), "8d1fc7758ba20766ba927f5ac405a6a3ffcb5e6e31f03a408b4660c138ec45f5");
    assert_eq!(at(r01, &["present"]), &Json::Bool(true));
    assert_eq!(text_at(r01, &["integrity"]), "pass");
    assert_eq!(text_at(r01, &["recognition"]), "declared");
    assert!(rows.iter().any(|row| at(row, &["present"]) == &Json::Bool(false)), "ungranted members are listed");
    assert!(!rows.iter().any(|row| text_at(row, &["name"]) == JAMS), "stage 16's member is not stage 15's");

    // The claims: R01's group is ready, q11 is unavailable with its label word for word, and a preflight is no verdict.
    let (before, claims) = p.answer(panel::rows::CLAIMS, obj(&[("group", t(R01_GROUP))]));
    let group = &items(at(&claims, &["rows"]))[0];
    assert_eq!(text_at(group, &["preflight", "kind"]), "ready");
    let checkers: Vec<&str> =
        items(at(group, &["preflight", "checkers"])).iter().map(|c| text_at(c, &["id"])).collect();
    assert_eq!(checkers, ["s15.macro"]);
    assert_eq!(at(group, &["latest_check"]), &Json::Null);
    let (_, q11) = p.answer(panel::rows::CLAIMS, obj(&[("group", t(Q11))]));
    let q11 = &items(at(&q11, &["rows"]))[0];
    assert_eq!(text_at(q11, &["preflight", "kind"]), "unavailable");
    let reasons: Vec<&str> = items(at(q11, &["preflight", "reasons"])).iter().map(|r| r.as_text().unwrap()).collect();
    assert!(reasons.contains(&"dependency_missing"), "{reasons:?}");
    assert_eq!(text_at(q11, &["label"]), "PASS_SHORTENED_THREE_BLOCK_WITNESS_AND_Q11_DEPTH3");
    assert_eq!(at(q11, &["latest_check"]), &Json::Null);

    // A check of a group whose preflight is unavailable is refused before anything runs.
    let refused = p.failed(
        panel::rows::CHECK,
        obj(&[
            ("group", t(Q11)),
            ("checker", t("s15.macro")),
            ("certificate", t(R01)),
            ("inputs", obj(&[])),
            ("level", t("poses")),
            ("digest", t(R01_TABLE_DIGEST)),
            ("puzzle", t("jumbling")),
        ]),
    );
    assert_eq!(refused, code::NOT_GRANTED, "q11's missing input is a dependency the host does not hold");

    // The check job of R01's group: a workspace row whose run record is its workspace record.
    let check_args = obj(&[
        ("group", t(R01_GROUP)),
        ("checker", t("s15.macro")),
        ("certificate", t(R01)),
        ("inputs", obj(&[("table", t(TABLE)), ("menu", t(MENU)), ("jams", t(JAMS))])),
        ("level", t("poses")),
        ("digest", t(R01_TABLE_DIGEST)),
        ("puzzle", t("jumbling")),
    ]);
    let (receipt, run) = p.answer(panel::rows::CHECK, check_args);
    assert_eq!(completion_kind(&receipt, &p.layer), "workspace");
    assert_eq!(p.layer.revision().workspace, head.workspace + 1, "the run record is one workspace action");
    assert_eq!(text_at(&run, &["outcome"]), "completed");
    assert_eq!(text_at(&run, &["reading", "kind"]), "complete");
    assert_eq!(text_at(&run, &["branch", "digest"]), R01_TABLE_DIGEST);
    let check = &items(at(&run, &["checks"]))[0];
    assert_eq!(text_at(check, &["checker", "id"]), "s15.macro");
    assert_eq!(text_at(check, &["verdict", "kind"]), "pass");
    let sha = text_at(&run, &["run"]).to_owned();
    let record = p.layer.record(&format!("research.run/{sha}")).expect("the run record is a workspace record");
    let payload = c600_session::persist::codec::parse(record.payload.canonical().as_bytes()).expect("JSON");
    assert_eq!(c600_digest::sha256_hex(text_at(&payload, &["record"]).as_bytes()), sha);

    // The claims answer from before the check is stale: the revision moved and so did the research data.
    let stale = p.host.read(&p.layer, &before);
    assert!(matches!(stale.state, D2::Stale { at: Some(_), research_moved: true }), "{stale:?}");
    assert!(stale.answer.is_some(), "a stale answer is still shown, marked stale");
    let (_, claims) = p.answer(panel::rows::CLAIMS, obj(&[("group", t(R01_GROUP))]));
    let latest = at(&items(at(&claims, &["rows"]))[0], &["latest_check"]);
    assert_eq!(text_at(latest, &["run"]), sha);
    assert_eq!(text_at(&items(at(latest, &["verdicts"]))[0], &["kind"]), "pass");
    assert_eq!(text_at(&items(at(&claims, &["rows"]))[0], &["preflight", "kind"]), "ready", "preflight is unchanged");

    // The check's KN6 report, steppable without the producer, with the checker's source digest.
    let (_, report) = p.answer(panel::rows::REPORT, obj(&[("run", t(&sha)), ("check", Json::Int(0))]));
    let source = c600_checkers::archive::find("s15.macro").expect("registered").checker.sources.identity().source;
    assert_eq!(text_at(&report, &["checker", "source"]), source);
    assert_eq!(text_at(&report, &["report", "verdict", "kind"]), "pass");
    assert!(!items(at(&report, &["report", "trace"])).is_empty(), "the checker's own trace");
    assert_eq!(p.failed(panel::rows::REPORT, obj(&[("run", t(&sha)), ("check", Json::Int(1))])), code::ARGUMENT_FORM);

    // The group's evidence word: at the key the run covers, where the run's report is admitted through the job, a
    // finite instance on one instance; at the binding's key, scoped to the record's bytes, which no check covers, open.
    let (_, word) = p.answer(panel::rows::WORD, obj(&[("group", t(R01_GROUP))]));
    let words = items(at(&word, &["rows"]));
    assert_eq!(words.len(), 2);
    assert_eq!(at(&words[0], &["run"]), &Json::Null);
    assert_eq!(text_at(&words[0], &["scope", "input", "role"]), "bytes");
    assert_eq!(text_at(&words[0], &["word", "word"]), "open");
    assert_eq!(text_at(&words[1], &["run"]), sha);
    assert_eq!(at(&words[1], &["admitted", "leaves"]), &Json::Int(1));
    assert_eq!(text_at(&words[1], &["scope", "input", "role"]), "poses");
    assert_eq!(text_at(&words[1], &["scope", "input", "sha256"]), R01_TABLE_DIGEST);
    assert_eq!(text_at(&words[1], &["word", "word"]), "finite instance");
    assert_eq!(text_at(&words[1], &["word", "scope_phrase"]), "on one instance");
    assert_eq!(text_at(&words[0], &["scope", "identity"]), text_at(&words[1], &["scope", "identity"]));

    // The runs: the check job, then the archive's runs as recorded, which reproduce as not run.
    let (_, runs) = p.answer(panel::rows::RUNS, obj(&[]));
    assert_eq!(text_at(&items(at(&runs, &["rows"]))[0], &["run"]), sha);
    let archived = items(at(&runs, &["archived"]));
    assert!(archived.iter().any(|run| text_at(run, &["member"]) == STOP && at(run, &["stop"]) == &Json::Bool(true)));
    assert!(archived.iter().all(|run| text_at(run, &["reproduction", "kind"]) == "not_run"));

    // The reproduction: the same payload, byte for byte, written as a workspace record.
    let workspace = p.layer.revision().workspace;
    let (receipt, again) = p.answer(panel::rows::REPRODUCE, obj(&[("run", t(&sha))]));
    assert_eq!(completion_kind(&receipt, &p.layer), "workspace");
    assert_eq!(p.layer.revision().workspace, workspace + 1);
    assert_eq!(text_at(&again, &["answer", "kind"]), "same");
    assert!(p.layer.record(&format!("research.reproduction/{sha}")).is_some());
    let (_, runs) = p.answer(panel::rows::RUNS, obj(&[("group", t(R01_GROUP))]));
    assert_eq!(text_at(&items(at(&runs, &["rows"]))[0], &["reproduction", "kind"]), "same");
    let unknown = "0".repeat(64);
    assert_eq!(p.failed(panel::rows::REPRODUCE, obj(&[("run", t(&unknown))])), code::NAMED_OBJECT_INVALID);

    // The check bundle of R01 with its stored inputs, written to the granted file only.
    let (receipt, bundle) =
        p.answer(panel::rows::BUNDLE, obj(&[("records", texts(&[R01])), ("destination", t(BUNDLE_OUT))]));
    assert_eq!(completion_kind(&receipt, &p.layer), "file");
    let zip = std::fs::read(p.folder.join("bundle.zip")).expect("the bundle is written");
    assert!(zip.starts_with(b"PK"));
    assert_eq!(text_at(&bundle, &["sha256"]), c600_digest::sha256_hex(&zip));
    assert_eq!(
        p.failed(panel::rows::BUNDLE, obj(&[("records", texts(&[R01])), ("destination", t(REFUSED_OUT))])),
        code::NOT_GRANTED
    );

    // The obligation export: the format packet's supported statement, byte for byte; R01's claim statement and a
    // statement with no body are untranslatable and write nothing; an ungranted destination writes nothing.
    let (_, statement, source, source_sha) = supported();
    let (_, exported) =
        p.answer(panel::rows::OBLIGATION, obj(&[("statement", t(&statement)), ("destination", t(LEAN_OUT))]));
    let written = std::fs::read(p.folder.join("obligation.lean")).expect("the obligation is written");
    assert_eq!(written, source.as_bytes());
    assert_eq!(text_at(&exported, &["sha256"]), source_sha);
    std::fs::remove_file(p.folder.join("obligation.lean")).expect("removed");
    let r01_statement = Bindings::edition_1().find(R01_GROUP).expect("R01's group").statement.id.clone();
    for refused in [r01_statement.as_str(), "1".repeat(64).as_str()] {
        let code = p.failed(panel::rows::OBLIGATION, obj(&[("statement", t(refused)), ("destination", t(LEAN_OUT))]));
        assert_eq!(code, "LB5:input.obligation_untranslatable", "{refused}");
        assert!(!p.folder.join("obligation.lean").exists(), "a refusal writes nothing");
    }
    let code = p.failed(panel::rows::OBLIGATION, obj(&[("statement", t(&statement)), ("destination", t(REFUSED_OUT))]));
    assert_eq!(code, code::NOT_GRANTED);
    assert!(!p.folder.join("refused.lean").exists());

    // The open obligations: stage 19's remaining obligations among the record-only anchors, and every export.
    let (_, obligations) = p.answer(panel::rows::OBLIGATIONS, obj(&[]));
    let anchors = items(at(&obligations, &["rows"]));
    assert!(anchors.iter().any(|row| text_at(row, &["group"]) == GLOBAL_OPEN));
    let exports = items(at(&obligations, &["exports"]));
    let results: Vec<&str> = exports.iter().map(|e| text_at(e, &["result", "kind"])).collect();
    assert_eq!(results, ["written", "refused", "refused"]);
}

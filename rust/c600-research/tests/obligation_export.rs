//! LP5 piece R9's row, `research.obligation.export` (LB5.18; PS1.45, PS1.46; RTE.28; EVD.64), driven headless through
//! the command layer and the research host as `tests/panel.rs` drives the panel's rows.
//!
//! These are row halves of CT-LB5-11, CT-PS1-26 and CT-RTE-23, whose translator halves run in
//! `c600-evidence/tests/obligation.rs`. The format packet's `obligations` vectors are read in place from
//! `docs/architecture/1.0/language/format-packet/vectors.json`; expected text and codes come from those vectors and
//! from PS1.45's constants, never from the row's own output. Every run uses fresh data: a new session in memory,
//! repository files read in place (read-only) through the host's file grant, and the written file in a fresh folder
//! under the test's scratch directory. No Lean runs (owner, 23 November 2026, LP5-2). Evidence kind: source.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::OnceLock;

use c600_archive::bindings::Bindings;
use c600_checkers::engines::Loaded;
use c600_command::{
    code, CommandLayer, Kind, MemoryStore, PreviewRule, Profile, Receipt, Revision, Secret, Shell, Status,
    WorkspaceRecord,
};
use c600_evidence::obligation::{self, json};
use c600_research::panel::{self, invoke, rows, Grant, Host, D2};
use c600_session::persist::codec::Json;

const R01: &str = "research/jumbling/restoration/15-lowering-machine/results/certificates/R01.json";
const JAMS: &str = "research/jumbling/restoration/16-six-jams/jams.json";
/// R01's start table: stage 13's checkpoint table, outside the corpus, granted to the host.
const TABLE: &str =
    "research/jumbling/restoration/13-tutorial-runner/runs/dev/d2-s7302/component-checkpoint/tables/00021.json";
/// The start digest R01 states (`table_digest`).
const R01_TABLE_DIGEST: &str = "2430f4cc99ea121a6d3476ec21b66fc401e277d13809ad5e1c979e6e0abe1472";
const R01_GROUP: &str = "15/macro:research/jumbling/restoration/15-lowering-machine/results/certificates/R01.json#";
const MENU: &str = "fixtures/wj-I_a.menu.json";
const LEAN_OUT: &str = "out/obligation.lean";
/// The format packet's vectors whose bodies the grant holds, each under `obligations/<id>.json`.
const BODIES: [&str; 3] = ["supported-statement", "unknown-symbol", "wrong-arity"];
const UNTRANSLATABLE: &str = "LB5:input.obligation_untranslatable";

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
    let folder = Path::new(env!("CARGO_TARGET_TMPDIR")).join("r9-obligation").join(format!(
        "{tag}-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::SeqCst)
    ));
    let _ = std::fs::remove_dir_all(&folder);
    std::fs::create_dir_all(&folder).expect("a scratch folder");
    folder
}

/// The names of the files in `folder`, sorted.
fn listing(folder: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(folder)
        .expect("the scratch folder")
        .map(|entry| entry.expect("an entry").file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

fn t(text: &str) -> Json {
    Json::text(text)
}

fn obj(fields: &[(&str, Json)]) -> Json {
    Json::Object(fields.iter().map(|(name, value)| ((*name).to_owned(), value.clone())).collect())
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

/// The format packet's `obligations` vector `id`: its body's canonical bytes, the text the row is given for its
/// StatementId (a typed ID's content digest, or the canonical FP.19 form of a named ID) and its `expected` value.
fn vector(id: &str) -> (Vec<u8>, String, json::Json) {
    let vectors = json::parse(&read("docs/architecture/1.0/language/format-packet/vectors.json")).expect("JSON");
    let vector = vectors
        .get("obligations")
        .and_then(json::Json::as_array)
        .and_then(|all| all.iter().find(|v| v.get("id").and_then(json::Json::as_text) == Some(id)))
        .unwrap_or_else(|| panic!("the vector {id}"))
        .clone();
    let given = vector.get("given").expect("given");
    let body = json::canonical(given.get("body").expect("a body")).expect("canonical");
    let statement = given.get("statement").and_then(|s| s.get("id")).expect("a StatementId");
    let text = match statement.get("kind").and_then(json::Json::as_text) {
        Some("typed") => statement
            .get("content")
            .and_then(|c| c.get("sha256"))
            .and_then(json::Json::as_text)
            .expect("a content digest")
            .to_owned(),
        _ => String::from_utf8(json::canonical(statement).expect("canonical")).expect("UTF-8"),
    };
    (body, text, vector.get("expected").expect("expected").clone())
}

/// A research session serving the panel's rows, with the grant: R01, its claim group's stored inputs and stage 16's
/// jams, the I_a menu and the three vector bodies for reading, and one file for writing.
struct Panel {
    layer: CommandLayer<MemoryStore>,
    shell: Shell,
    host: Host<'static>,
    folder: PathBuf,
}

impl Panel {
    fn new() -> Panel {
        let folder = scratch("row");
        let kernel = loaded().kernel().clone();
        let (solved, none) = (kernel.solved(), kernel.protected_set(&[]).expect("no protected orbit"));
        let secret = Secret::fixed([7; 32]);
        let mut layer = CommandLayer::new_with_profile(
            kernel,
            panel::table(),
            secret,
            MemoryStore::default(),
            "r9",
            solved,
            none,
            Profile::Research,
        );
        c600_tools::workspace::install(&mut layer);
        let menu = {
            let fixture = read("research/jumbling/fixtures/wj-I_a.json");
            let doc = c600_checkers::claims::document(&fixture).ok().expect("the fixture is JSON");
            doc.get("menu").expect("a menu").canonical().into_bytes()
        };
        std::fs::write(folder.join("menu.json"), menu).expect("the menu is written");
        let mut grant = Grant::new().read(MENU, &folder.join("menu.json")).expect("the menu");
        for id in BODIES {
            std::fs::write(folder.join(format!("{id}.json")), vector(id).0).expect("the body is written");
            let name = format!("obligations/{id}.json");
            grant = grant.read(&name, &folder.join(format!("{id}.json"))).expect("the body");
        }
        let bindings = Bindings::edition_1();
        let row = bindings.find(R01_GROUP).expect("R01's group");
        let stored: Vec<String> =
            bindings.stored(row).into_iter().map(|(file, _)| bindings.file(file).path.clone()).collect();
        for name in [R01, JAMS].into_iter().chain(stored.iter().map(String::as_str)) {
            grant = grant.read(name, &repository().join(name)).expect("a repository file");
        }
        let grant = grant.write(LEAN_OUT, &folder.join("obligation.lean"));
        let host = Host::new(loaded(), grant);
        host.serve(&mut layer).expect("a research session serves the panel's rows");
        Panel { layer, shell: Shell::new(Secret::fixed([7; 32]), "obligation"), host, folder }
    }

    /// A control's press: the row through the shell and the command layer, then the host's run of the queued job.
    fn press(&mut self, row: &str, args: Json) -> (Receipt, c600_research::panel::Read) {
        let receipt = invoke(&mut self.shell, &mut self.layer, row, &args).expect("the shell sends the invocation");
        assert!(self.host.run_pending(&mut self.layer).is_empty(), "every queued job is the panel's");
        let read = self.host.read(&self.layer, &receipt);
        (receipt, read)
    }

    /// A press whose answer is current (or empty), computed at the head revision it read.
    fn answer(&mut self, row: &str, args: Json) -> (Receipt, Json) {
        let pressed = c600_session::persist::codec::parse(self.layer.revision().to_json().canonical().as_bytes());
        let (receipt, read) = self.press(row, args);
        assert!(matches!(read.state, D2::Current | D2::Empty), "{row}: {read:?}");
        let answer = read.answer.expect("an answer");
        assert_eq!(Some(at(&answer, &["revision"]).clone()), pressed, "{row}");
        (receipt, at(&answer, &["answer"]).clone())
    }

    fn failed(&mut self, row: &str, args: Json) -> &'static str {
        match self.press(row, args).1.state {
            D2::Error { code } => code,
            other => panic!("{row}: {other:?}"),
        }
    }

    fn export(&self, statement: &str) -> Json {
        obj(&[("statement", t(statement)), ("destination", t(LEAN_OUT))])
    }

    /// What an export must leave as it was: the session's revision, workspace records and preview, and R01's
    /// group as the evidence, claims, report and runs rows read it.
    fn seen(&mut self, run: &str) -> Seen {
        let group = obj(&[("group", t(R01_GROUP))]);
        Seen {
            revision: self.layer.revision().clone(),
            records: self.layer.records().clone(),
            preview: self.layer.preview().is_some(),
            word: self.answer(rows::WORD, group.clone()).1,
            claims: self.answer(rows::CLAIMS, group.clone()).1,
            report: self.answer(rows::REPORT, obj(&[("run", t(run)), ("check", Json::Int(0))])).1,
            runs: self.answer(rows::RUNS, group).1,
        }
    }
}

#[derive(Debug, PartialEq)]
struct Seen {
    revision: Revision,
    records: BTreeMap<String, WorkspaceRecord>,
    preview: bool,
    word: Json,
    claims: Json,
    report: Json,
    runs: Json,
}

fn completion_kind(receipt: &Receipt, layer: &CommandLayer<MemoryStore>) -> &'static str {
    match layer.outcome(&receipt.command, &receipt.request).expect("an outcome").status {
        Status::Completed(completion) => completion.kind.name(),
        other => panic!("not completed: {other:?}"),
    }
}

// ------------------------------------------------------------------------------------------------------ tests

/// CT-LB5-11 (source, part: at `research.obligation.export`, an export and two refusals change no evidence grade,
/// claim, revision, workspace record or commit gate, and a refusal after a written export leaves that file as it was;
/// not run here: the translator, the refusal vectors on a fresh host and the row's scan, which the other parts run).
/// CT-RTE-23 (source, part: at the row, an export in PS1.45's format through LB5.18's file grant promotes no claim or
/// commit authority; not run here: the translator and the scans, which the other parts run).
/// CT-PS1-26 (source, part: at the row, the supported statement exports its pinned Lean 4 source and vocabulary through
/// a granted file and promotes no evidence; not run here: the refusal vectors on a fresh host, which the next test
/// runs).
///
/// A KN6 check job of R01's claim group first gives the group a word on one admitted leaf, so there is evidence to
/// change. The row then writes the format packet's supported statement byte for byte with its format, vocabulary and
/// pins, and refuses the unknown-symbol and wrong-arity vectors. After the three, the group's evidence word, claims,
/// report and runs read as before, and so do the revision, the workspace records and the absence of a preview; only
/// the host's research data moved, since the exports are on record.
#[test]
fn ct_lb5_11_an_export_changes_no_evidence_claim_or_revision() {
    let mut p = Panel::new();
    let (_, run) = p.answer(
        rows::CHECK,
        obj(&[
            ("group", t(R01_GROUP)),
            ("checker", t("s15.macro")),
            ("certificate", t(R01)),
            ("inputs", obj(&[("table", t(TABLE)), ("menu", t(MENU)), ("jams", t(JAMS))])),
            ("level", t("poses")),
            ("digest", t(R01_TABLE_DIGEST)),
            ("puzzle", t("jumbling")),
        ]),
    );
    assert_eq!(text_at(&items(at(&run, &["checks"]))[0], &["verdict", "kind"]), "pass");
    let sha = text_at(&run, &["run"]).to_owned();
    let before = p.seen(&sha);
    let words = items(at(&before.word, &["rows"]));
    assert_eq!(at(&words[1], &["admitted", "leaves"]), &Json::Int(1), "the run's report is one admitted leaf");
    assert_eq!(text_at(&words[1], &["word", "word"]), "finite instance");
    assert!(!before.preview);
    let research = p.host.research();

    // The supported vector, written through the granted file in PS1.45's format, with its pins.
    let (_, statement, expected) = vector("supported-statement");
    let expected_text = |key: &str| expected.get(key).and_then(json::Json::as_text).expect("text").to_owned();
    let args = p.export(&statement);
    let (receipt, exported) = p.answer(rows::OBLIGATION, args);
    assert_eq!(completion_kind(&receipt, &p.layer), "file");
    let written = std::fs::read(p.folder.join("obligation.lean")).expect("the obligation is written");
    assert_eq!(written, expected_text("source").as_bytes());
    assert_eq!(text_at(&exported, &["sha256"]), expected_text("sha256"));
    assert_eq!(text_at(&exported, &["format"]), expected_text("format"));
    assert_eq!(text_at(&exported, &["format"]), obligation::FORMAT);
    assert_eq!(text_at(&exported, &["vocabulary"]), expected_text("vocabulary"));
    assert_eq!(text_at(&exported, &["pins", "statement"]), statement);
    assert_eq!(text_at(&exported, &["pins", "symbols"]), obligation::SYMBOLS_DRAFT_1_SHA256);
    assert_eq!(text_at(&exported, &["pins", "prelude"]), obligation::PRELUDE_DRAFT_1_SHA256);

    // Refusals after it leave the written file byte for byte and add no file.
    let files = listing(&p.folder);
    for id in ["unknown-symbol", "wrong-arity"] {
        let (_, statement, _) = vector(id);
        let args = p.export(&statement);
        assert_eq!(p.failed(rows::OBLIGATION, args), UNTRANSLATABLE, "{id}");
        assert_eq!(std::fs::read(p.folder.join("obligation.lean")).expect("still there"), written, "{id}");
        assert_eq!(listing(&p.folder), files, "{id}");
    }

    assert_eq!(p.seen(&sha), before, "an export changes no evidence, claim, revision, record or preview");
    assert_ne!(p.host.research(), research, "the exports are on record in the research data");
    let (_, obligations) = p.answer(rows::OBLIGATIONS, obj(&[]));
    let results: Vec<&str> =
        items(at(&obligations, &["exports"])).iter().map(|e| text_at(e, &["result", "kind"])).collect();
    assert_eq!(results, ["written", "refused", "refused"]);

    // The row's shape gives an export no commit gate: a `file` row that takes no token, has no gate or action guard
    // and prepares no preview.
    let row = p.layer.table().row(rows::OBLIGATION).expect("the row");
    assert_eq!(row.kind, Kind::File);
    assert!(!row.takes_token() && row.gates.is_empty() && row.action_guard.is_none());
    assert_eq!(row.preview, PreviewRule::None);
}

/// CT-PS1-26 (source, part: the format packet's unknown-symbol, wrong-arity and named-statement vectors given to
/// `research.obligation.export`, the first two by their typed StatementIds with their bodies granted under those
/// digests and the third as its FP.19 named form, each refuse with `LB5:input.obligation_untranslatable`, write nothing
/// and stay on record; not run here: the supported vector's byte-for-byte export, the vocabulary pins and the absent
/// return format, check mode and process or network grant, which `c600-evidence/tests/obligation.rs`, `panel.rs` and
/// this file's other tests run).
/// CT-ERR-19 (source, part: an untranslatable obligation refused through the row writes no file; not run here: the
/// codes' owning chapters, refusal form and retries, which `c600-evidence/tests/obligation.rs` runs).
///
/// The named-statement vector's StatementId is given as the text of its FP.19 form: the row passes it to the
/// translator, which refuses it because a named ID has no body (PS1.46). Text that is neither a typed ID's 64
/// lowercase hex digits nor a named ID's form (exactly `kind` `named`, `stage`, `commit` and `name`) is an
/// argument-form refusal, which writes nothing and reaches no translator.
#[test]
fn ct_ps1_26_refusal_vectors_refuse_through_the_row_and_write_nothing() {
    let mut p = Panel::new();
    let files = listing(&p.folder);
    for id in ["unknown-symbol", "wrong-arity"] {
        let (body, statement, expected) = vector(id);
        assert_eq!(c600_digest::sha256_hex(&body), statement, "{id}: the grant holds the body the ID names");
        assert_eq!(expected.as_text(), Some(UNTRANSLATABLE), "{id}");
        let args = p.export(&statement);
        assert_eq!(p.failed(rows::OBLIGATION, args), UNTRANSLATABLE, "{id}");
        assert!(!p.folder.join("obligation.lean").exists(), "{id}: a refusal writes nothing");
        assert_eq!(listing(&p.folder), files, "{id}");
    }

    let (_, named, expected) = vector("named-statement");
    assert_eq!(expected.as_text(), Some(UNTRANSLATABLE));
    assert!(named.starts_with('{'), "a named StatementId has no digest: {named}");
    let args = p.export(&named);
    assert_eq!(p.failed(rows::OBLIGATION, args), UNTRANSLATABLE, "a named statement has no body");
    assert!(!p.folder.join("obligation.lean").exists());
    assert_eq!(listing(&p.folder), files);

    let upper = named.replace("named", "NAMED");
    let fifth = named.replacen('{', "{\"extra\":\"x\",", 1);
    let typed_upper = "A".repeat(64);
    for text in [upper.as_str(), fifth.as_str(), typed_upper.as_str(), "not a statement", "{}"] {
        let args = p.export(text);
        assert_eq!(p.failed(rows::OBLIGATION, args), code::ARGUMENT_FORM, "{text}");
        assert!(!p.folder.join("obligation.lean").exists(), "{text}");
        assert_eq!(listing(&p.folder), files, "{text}");
    }

    let (_, obligations) = p.answer(rows::OBLIGATIONS, obj(&[]));
    let exports = items(at(&obligations, &["exports"]));
    let codes: Vec<&str> = exports.iter().map(|e| text_at(e, &["result", "code"])).collect();
    assert_eq!(codes, [UNTRANSLATABLE; 3], "the three refusals are on record");
}

/// The research host's sources and the translator the row calls, for the scan.
const SOURCES: [(&str, &str); 13] = [
    ("src/lib.rs", include_str!("../src/lib.rs")),
    ("src/archived.rs", include_str!("../src/archived.rs")),
    ("src/evidence.rs", include_str!("../src/evidence.rs")),
    ("src/job.rs", include_str!("../src/job.rs")),
    ("src/record.rs", include_str!("../src/record.rs")),
    ("src/panel/mod.rs", include_str!("../src/panel/mod.rs")),
    ("src/panel/answers.rs", include_str!("../src/panel/answers.rs")),
    ("src/panel/files.rs", include_str!("../src/panel/files.rs")),
    ("src/panel/host.rs", include_str!("../src/panel/host.rs")),
    ("src/panel/rows.rs", include_str!("../src/panel/rows.rs")),
    ("src/panel/wire.rs", include_str!("../src/panel/wire.rs")),
    ("../c600-evidence/src/obligation/mod.rs", include_str!("../../c600-evidence/src/obligation/mod.rs")),
    ("../c600-evidence/src/obligation/json.rs", include_str!("../../c600-evidence/src/obligation/json.rs")),
];

/// CT-LB5-11 (source, part: a scan of the row's code, every source of `c600-research` with the translator it calls,
/// finds no process, network or Lean, and file access only in the host's grant; no row of the command table and no
/// fact of the import table offers a proof-record return path; not run here: the row's export, which the other parts
/// run).
/// CT-RTE-23 (source, part: the same scan, with no Lean package in the workspace's lock file, no row that reads a proof
/// record back and no import fact asserting check `formal proof` or a check mode outside EVD.47's; not run here: the
/// export and the translator's own checks, which the other parts run).
#[test]
fn ct_lb5_11_the_rows_code_reaches_no_process_network_or_lean() {
    let crate_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut found = Vec::new();
    for folder in ["src", "src/panel"] {
        for entry in std::fs::read_dir(crate_dir.join(folder)).expect("a source folder") {
            let path = entry.expect("an entry").path();
            if path.is_file() {
                found.push(format!("{folder}/{}", path.file_name().expect("a name").to_string_lossy()));
            }
        }
    }
    found.sort();
    let mut scanned: Vec<String> =
        SOURCES.iter().filter(|(n, _)| n.starts_with("src/")).map(|(n, _)| n.to_string()).collect();
    scanned.sort();
    assert_eq!(found, scanned, "the scan lists every source of the crate");
    for (name, source) in SOURCES {
        for banned in [
            "std::process",
            "process::",
            "Command::new",
            "std::net",
            "TcpStream",
            "TcpListener",
            "UdpSocket",
            "c600_tools::process",
            ".spawn(",
        ] {
            assert!(!source.contains(banned), "{name} names {banned:?}");
        }
        let files = source.contains("std::fs") || source.contains("File::");
        assert_eq!(files, name == "src/panel/files.rs", "{name}: only the host's grant opens files");
    }

    // Lean is no dependency: the crate depends on eight workspace crates and has no build script, and no package of
    // the workspace's lock file is Lean.
    let manifest =
        String::from_utf8(std::fs::read(crate_dir.join("Cargo.toml")).expect("the manifest")).expect("UTF-8");
    let dependencies: Vec<&str> = manifest
        .split("[dependencies]")
        .nth(1)
        .unwrap_or("")
        .lines()
        .take_while(|l| !l.trim_start().starts_with('['))
        .filter_map(|l| l.split('=').next().map(str::trim).filter(|n| !n.is_empty() && !n.starts_with('#')))
        .collect();
    assert_eq!(
        dependencies,
        [
            "c600-archive",
            "c600-checkers",
            "c600-command",
            "c600-digest",
            "c600-evidence",
            "c600-library",
            "c600-session",
            "c600-tools"
        ]
    );
    assert!(!manifest.to_lowercase().contains("lean"), "no Lean dependency");
    assert!(!crate_dir.join("build.rs").exists(), "no build script");
    let lock = String::from_utf8(read("rust/Cargo.lock")).expect("UTF-8");
    let packages: Vec<&str> = lock.lines().filter_map(|l| l.strip_prefix("name = \"")).collect();
    assert!(packages.len() > 8, "the lock file lists the workspace's packages");
    assert!(packages.iter().all(|p| !p.to_lowercase().contains("lean")), "no Lean package");

    // No proof-record return path: in the table a research host serves, no row names Lean or a formal check, the one
    // row that names a proof is `file.proof.export`, which writes a journal range to a file (DT-M1 C1's verified proof
    // session) and reads no proof record back, the rows that name obligations are the export and its read, and no rule
    // of the import table sets check `formal proof` (EVD.47).
    let table = panel::table();
    let ids: Vec<&str> = table.rows().map(|row| row.id.as_str()).collect();
    let words = |id: &str| id.split(['.', '_']).map(str::to_lowercase).collect::<Vec<_>>();
    for id in &ids {
        assert!(words(id).iter().all(|w| w != "lean" && w != "formal"), "{id}");
    }
    let proofs: Vec<&str> = ids.iter().copied().filter(|id| words(id).iter().any(|w| w == "proof")).collect();
    assert_eq!(proofs, ["file.proof.export"]);
    let export = table.row("file.proof.export").expect("the row");
    assert_eq!(export.kind, Kind::File);
    let mut arguments: Vec<(&str, &str)> = export.arguments.iter().map(|(n, t)| (n.as_str(), t.as_str())).collect();
    arguments.sort();
    assert_eq!(arguments, [("destination", "FileRef"), ("history", "JournalRangeRef")]);
    let mut obligations: Vec<&str> = ids.iter().copied().filter(|id| id.contains("obligation")).collect();
    obligations.sort();
    assert_eq!(obligations, [rows::OBLIGATION, rows::OBLIGATIONS]);
    let imports = c600_archive::ImportTable::edition_1();
    assert!(!imports.rules().is_empty());
    for rule in imports.rules() {
        if let c600_archive::Outcome::Facts(facts) = &rule.outcome {
            assert_ne!(facts.check(), Some(c600_evidence::Check::FormalProof), "{}", rule.reason);
            let mode = facts.check_mode();
            assert!(!matches!(mode, Some(c600_evidence::CheckMode::Other(_))), "a new check mode: {}", rule.reason);
        }
    }
}

//! R-05 against the command layer (CT-KN5-10): every entry point of the H-08 table, from the shell and from a run,
//! against the restriction table, which is the table's `protection` gate and its confirmation of a release. The layer
//! is built as far as today's rows go: the I_a twist menu is pinned, the host's provenance reader is set and the drafts
//! are in the table. Where the frozen text is silent, the test follows the draft rulings recorded in `src/lib.rs`.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use c600_command::{
    code, draft_args, Caller, CommandLayer, Direction, Error, HeadlessFrames, Invocation, Kind, MemoryStore, Overlay,
    Provenance, Receipt, Row, Secret, Shell, Status, Table, TWIST_ROW,
};
use c600_jumble::twist::Menu;
use c600_jumble::Context;
use c600_kernel::json::{self, Value};
use c600_kernel::request::{self as wire, Binding, MenuPin, Operation};
use c600_kernel::{Kernel, ORBITS};

const SECRET: [u8; 32] = [7; 32];

/// The pinned I_a menu's set and order digests (`research/jumbling/fixtures/wj-I_a.json`), as KN3's tests pin them.
const SET: &str = "fddc8532b9184bd1b413c1032b5a7a305dd9fd1f10385d87ba6ec4fd06eb958c";
const ORDER: &str = "5a0429d2946fbf58b495d8f300e24a3fa164b1d9518ce4a5c3f4ef0a40625c3c";

fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..")
}

fn kernel() -> Arc<Kernel> {
    static KERNEL: OnceLock<Arc<Kernel>> = OnceLock::new();
    KERNEL
        .get_or_init(|| match Kernel::load(&repository().join("assets")) {
            Ok(kernel) => Arc::new(kernel),
            Err(error) => panic!("the model in assets/ does not load: {error}"),
        })
        .clone()
}

fn ctx() -> Arc<Context> {
    static CTX: OnceLock<Arc<Context>> = OnceLock::new();
    CTX.get_or_init(|| Arc::new(Context::load(&repository().join("assets")).expect("the exact model loads"))).clone()
}

fn pinned() -> Menu {
    let fixture = std::fs::read(repository().join("research/jumbling/fixtures/wj-I_a.json")).expect("the I_a fixture");
    let record = json::parse(&fixture).expect("the I_a fixture is JSON").get("menu").expect("a menu").clone();
    Menu::from_record(&ctx(), &record).expect("the pinned I_a menu")
}

/// `twist.apply`'s arguments for menu item `index` at cap 0, pressed directly: its cap, and the rest of the binding as
/// `rotation`. Item 1 is a retained item whose word moves every orbit.
fn twist_args(index: u64) -> Value {
    let binding = Binding {
        request: wire::menu_request(SET, ORDER, index),
        cap: 0,
        operation: Operation::MenuDirect,
        menu: Some(MenuPin { set: SET.to_owned(), order: ORDER.to_owned() }),
    };
    let Value::Object(mut fields) = binding.to_json() else { panic!("a binding is an object") };
    let cap = fields.remove("cap").expect("a binding has a cap");
    json::object([("cap", cap), ("rotation", Value::Object(fields))])
}

fn word(letters: &[i64]) -> Value {
    json::parse(format!(r#"[{{"kind": "word", "moves": {letters:?}}}]"#).as_bytes()).unwrap()
}

fn orbits(orbits: &[i64]) -> Value {
    Value::Array(orbits.iter().map(|&orbit| Value::Int(orbit)).collect())
}

fn text(s: &str) -> Value {
    Value::String(s.to_owned())
}

/// The host's provenance reader as the tests stand it in: it accepts every block.
struct Reader;

impl Provenance for Reader {
    fn check(&self, _block: &[u8]) -> Result<(), Error> {
        Ok(())
    }

    fn admit(&self, _block: &[u8]) -> Result<(), Error> {
        Ok(())
    }
}

/// Arguments of `row` with every argument other than the token given: a value of its form where the layer knows the
/// type (`src/forms.rs`), and `1` where it does not, so each row reaches the checks past its argument forms.
fn formed(row: &Row, revision: &Value) -> Value {
    let mut args = Vec::new();
    for (name, ty) in &row.arguments {
        let ty = ty.trim_end_matches('?');
        let value = match ty {
            "UserActionToken" => continue,
            "Text" | "ViewId" => text("r05"),
            "CellIdPair" => orbits(&[1, 2]),
            "CanonicalObjectRefs" => orbits(&[1]),
            "RevisionRef" => revision.clone(),
            "FilterRef" => json::object([("digest", text("r05")), ("job", Value::Int(1))]),
            "FilterPreviewRef" => json::object([("epoch", Value::Int(0)), ("job", Value::Int(1))]),
            "ViewSourceRef" => json::object([("built_in", text("r05"))]),
            _ if ty.split('|').all(|w| w.trim().starts_with(|c: char| c.is_ascii_lowercase())) => {
                text(ty.split('|').next().expect("a word").trim())
            }
            _ => Value::Int(1),
        };
        args.push((name.clone(), value));
    }
    Value::Object(args.into_iter().collect())
}

/// The outcome of a receipt for the walk's tally: its status, and its code when it has one.
fn outcome(receipt: &Receipt) -> String {
    match &receipt.status {
        Status::Refused(error) => format!("refused {}", error.code),
        Status::Failed(error) => format!("failed {}", error.code),
        other => format!("{other:?}").split(['(', ' ', '{']).next().expect("a status name").to_owned(),
    }
}

struct Fixture {
    layer: CommandLayer,
    shell: Shell,
    frames: HeadlessFrames,
}

impl Fixture {
    /// A session at the solved state that protects `protected`, with the I_a menu pinned and the test reader set.
    fn new(protected: &[i64]) -> Fixture {
        let k = kernel();
        let (start, protected) = (k.solved(), k.protected_set(protected).unwrap());
        let table = Table::h08().with_drafts();
        let mut layer =
            CommandLayer::new(k, table, Secret::fixed(SECRET), MemoryStore::default(), "s1", start, protected);
        layer.pin_menu(ctx(), pinned()).unwrap();
        layer.set_provenance(Box::new(Reader));
        Fixture { layer, shell: Shell::new(Secret::fixed(SECRET), "shell"), frames: HeadlessFrames::new() }
    }

    /// The labels' digest and the protected set: what no row may change while every orbit is protected.
    fn held(&self) -> (String, Vec<usize>) {
        (self.layer.revision().digest.clone(), self.layer.protected().orbits().to_vec())
    }

    fn protect(&mut self, set: &[i64], confirm: bool) -> Receipt {
        let mut args = vec![("orbits".to_owned(), orbits(set))];
        if confirm {
            args.push(("confirm".to_owned(), Value::Bool(true)));
        }
        self.shell.invoke(&mut self.layer, "protection.orbit.set", Value::Object(args.into_iter().collect())).unwrap()
    }

    /// Delivers `invocation` on `caller`'s channel, which must be refused with `expected` and change nothing.
    fn refused(&mut self, caller: &Caller, invocation: Invocation, expected: &str, what: &str) {
        let (before, log) = (self.held(), self.layer.store().log.len());
        let receipt = self.layer.invoke(caller, invocation);
        assert_eq!(receipt.status.code(), Some(expected), "{what}: {:?}", receipt.status);
        assert!(matches!(receipt.status, Status::Refused(_)), "{what}: {:?}", receipt.status);
        assert_eq!((self.held(), self.layer.store().log.len()), (before, log), "{what}");
    }
}

/// CT-KN5-10 (whole): R-05 against the command layer, beside the session's rows in
/// `c600-session/tests/conformance_rows.rs` and the layer's entry-point allowlist in `tests/kn5.rs`. With every orbit
/// protected, each of the H-08 table's 204 rows is delivered from the shell, with its token where it takes one, and on
/// a run's channel, with arguments of the forms the layer knows. No delivery changes the labels' digest or the
/// protected set. The table's `protection` gate names the six rows the restriction blocks: `operation.execute`,
/// `operation.preview.commit` (the solver's own macro kept in a record) and `twist.apply` are refused with the
/// protection code for a word that would move an orbit, `session.undo.partial` has
/// no entry to step back inside at the start (its protection refusal is in `tests/partial_undo.rs`), and
/// `session.practice` and `jumble.twist.menu` change nothing. Every path that releases protection asks for confirmation: a release by
/// `protection.orbit.set`, a weaker policy by `protection.policy.set`, and the undo of the change that protected the
/// last orbit; a run reaches no commit row and no protection row. `jumble.on` crosses with the set kept, and the way
/// back restores the labels. A row the layer does not build answers `row_not_built` and changes nothing, so a row built
/// later falls under the same walk; a stored row an entry serves is a query or workspace row (CT-LG3-02 in
/// `tests/served_rows.rs`), so serving one adds no path to a commit.
#[test]
fn every_table_row_from_the_shell_and_a_run_keeps_every_protected_orbit() {
    let every: Vec<i64> = (0..ORBITS as i64).collect();
    let mut f = Fixture::new(&every[..every.len() - 1]);
    assert!(matches!(f.protect(&every, false).status, Status::Completed(_)));
    let held = f.held();
    assert_eq!(held.1.len(), ORBITS);

    // The restriction table: the rows the `protection` gate names.
    let gated: Vec<&str> =
        f.layer.table().rows().filter(|row| row.has_gate("protection")).map(|row| row.id.as_str()).collect();
    assert_eq!(
        gated,
        [
            "jumble.twist.menu",
            "operation.execute",
            "operation.preview.commit",
            "session.practice",
            "session.undo.partial",
            "twist.apply"
        ]
    );

    // operation.execute: a preview of a word that would move an orbit is shown, and its press is refused.
    let p = {
        let receipt = f.shell.invoke(&mut f.layer, "operation.preview", draft_args(&word(&[1]))).unwrap();
        let Status::Running(p) = receipt.status else { panic!("preview not admitted: {:?}", receipt.status) };
        assert_eq!(f.layer.run_jobs(), 1);
        p
    };
    f.frames.present(&mut f.shell, &mut f.layer, p).unwrap();
    let press = f.shell.prepare(&f.layer, "operation.execute", json::object([("preview", Value::Int(p as i64))]));
    f.refused(&Caller::Shell, press.unwrap(), code::ORBIT_WOULD_MOVE, "operation.execute");
    // twist.apply: menu item 1's retained word moves every orbit.
    let press = f.shell.prepare(&f.layer, TWIST_ROW, twist_args(1)).unwrap();
    f.refused(&Caller::Shell, press, code::ORBIT_WOULD_MOVE, "twist.apply");
    // session.undo.partial: the start is no entry to step back inside.
    let start = json::object([("entry", Value::Int(0)), ("move", Value::Int(0))]);
    let press = f.shell.prepare(&f.layer, "session.undo.partial", start).unwrap();
    f.refused(&Caller::Shell, press, code::UNDO_SUPERSEDED, "session.undo.partial");

    // The release paths: each needs its confirmation, and a run may not take them.
    let release = f.shell.prepare(&f.layer, "protection.orbit.set", json::object([("orbits", orbits(&[]))])).unwrap();
    f.refused(&Caller::Shell, release, code::CONFIRMATION_REQUIRED, "a release");
    let weaker = f.shell.prepare(&f.layer, "protection.policy.set", json::object([("policy", text("net"))])).unwrap();
    f.refused(&Caller::Shell, weaker, code::CONFIRMATION_REQUIRED, "a weaker policy");
    let hint = f.layer.undo_hint(Direction::Undo).expect("the protection change is on top");
    let undo = f.shell.prepare(&f.layer, "session.undo", json::object([("hint", hint.to_json())])).unwrap();
    f.refused(&Caller::Shell, undo, code::CONFIRMATION_REQUIRED, "the undo of the protection");
    // operation.preview.commit (after the undo above, whose target the record write would become): the solver's
    // macro kept in a record moves an orbit, and its press is refused.
    let kept = json::object([
        ("payload", Value::Array(vec![Value::Int(1)])),
        ("provenance", text("block")),
        ("record", text("own")),
    ]);
    let written = f.shell.invoke(&mut f.layer, "workspace.set", kept).unwrap();
    assert!(matches!(written.status, Status::Completed(_)), "the record: {:?}", written.status);
    let own = json::object([("draft", json::object([("record", text("own"))]))]);
    let press = f.shell.prepare(&f.layer, "operation.preview.commit", own).unwrap();
    f.refused(&Caller::Shell, press, code::ORBIT_WOULD_MOVE, "operation.preview.commit");

    // The walk: every row, from the shell and on a run's channel.
    let tool = Overlay { module: "tools/full".into(), entry: "run".into() };
    f.layer.register_overlay("tool.full.run", Kind::Query, &[], tool).unwrap();
    let started = f.shell.invoke(&mut f.layer, "tool.full.run", json::object([])).unwrap();
    assert!(matches!(started.status, Status::Running(_)), "the tool's row: {:?}", started.status);
    let runtime = Caller::Runtime { run: f.layer.take_program_job().expect("the tool's job").run() };
    let rows: Vec<Row> = f.layer.table().rows().filter(|row| row.overlay.is_none()).cloned().collect();
    // 204 with `command-rows-lp4.json`'s six document rows (ADR-D01.84).
    assert_eq!(rows.len(), 204);
    let mut tally: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (n, row) in rows.iter().enumerate() {
        let args = formed(row, &f.layer.revision().to_json());
        let prepared = f.shell.prepare(&f.layer, &row.id, args.clone());
        let envelope = Invocation {
            command: format!("run-{n}"),
            row: row.id.clone(),
            args,
            token: None,
            epoch: f.layer.revision().epoch,
            budget: None,
        };
        let mut deliveries = vec![("run", runtime.clone(), envelope)];
        match prepared {
            Ok(invocation) => deliveries.push(("shell", Caller::Shell, invocation)),
            Err(error) => tally.entry(format!("shell prepare {}", error.code)).or_default().push(row.id.clone()),
        }
        for (name, caller, invocation) in deliveries {
            let receipt = f.layer.invoke(&caller, invocation);
            f.layer.run_jobs();
            if row.id == "jumble.on" && matches!(receipt.status, Status::Committed(_)) {
                // The crossing keeps the protected set, and the way back restores the labels it crossed from.
                assert_eq!(f.layer.protected().orbits(), held.1.as_slice());
                let back = json::object([("revision", f.layer.revision().to_json())]);
                let off = f.shell.invoke(&mut f.layer, "jumble.off", back).unwrap();
                assert!(matches!(off.status, Status::Committed(_)), "jumble.off: {:?}", off.status);
            }
            assert_eq!(f.held(), held, "{name} {}: {:?}", row.id, receipt.status);
            tally.entry(format!("{name} {}", outcome(&receipt))).or_default().push(row.id.clone());
        }
    }
    let rows_of = |key: &str| tally.get(key).cloned().unwrap_or_default();

    // A run reaches no commit row and no protection row.
    let commit_rows: Vec<String> =
        rows.iter().filter(|row| row.kind == Kind::Commit || row.kind == Kind::App).map(|r| r.id.clone()).collect();
    let mut refused_to_run = rows_of("run refused KN5:capability.commit_row_refused");
    refused_to_run.retain(|id| commit_rows.contains(id));
    assert_eq!(refused_to_run, commit_rows);
    for row in ["protection.orbit.set", "protection.policy.set"] {
        assert!(rows_of("run refused LG3:capability.not_granted").iter().any(|id| id == row), "{row}: {tally:?}");
    }
    // The gated rows the walk could not commit: `session.practice` is not built, and `jumble.twist.menu` asks for a
    // word the walk does not give.
    for row in ["session.practice", "jumble.twist.menu"] {
        assert!(!tally.iter().any(|(key, ids)| key.starts_with("shell Committed") && ids.iter().any(|id| id == row)));
    }
    // Most rows are not built in the layer yet; each answers that and changes nothing.
    assert!(rows_of("shell refused KN5:input.row_not_built").len() > 100, "{tally:?}");
}

//! Argument forms (draft; LP3's C6, its third common need). For a row it admits as a program job, a served stored row
//! or an overlay row, the layer checks the forms of the H-08 types it knows, and an optional argument may be left out of
//! any row. The frozen text is silent on these forms, so these tests follow the draft rulings recorded in `src/lib.rs`.

use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use c600_command::{
    code, digest, draft_args, CommandLayer, HeadlessFrames, Kind, MemoryStore, Overlay, Receipt, Secret, Shell, Status,
    Table, POLICY_ROW, RECORD_ROW, WORKSHEET_COMPARE_ROW, WORKSHEET_LOAD_ROW, WORKSHEET_SAVE_ROW,
};
use c600_kernel::json::{self, Value};
use c600_kernel::Kernel;

const SECRET: [u8; 32] = [7; 32];

const COMPARE: &str = "structure.compare";
const LAYER: &str = "structure.layer";
const SAVE: &str = "set.save";
const SCOPE: &str = "view.effect.scope";
const PLAY: &str = "recording.play";
const FILTER: &str = "filter.preview";
/// ADR-D01's row that resets a theme token's override; the headless layer does not build it.
const RESET: &str = "token.override.reset";

/// The stored rows the command layer builds itself.
const LAYER_ROWS: [&str; 11] = [
    "operation.preview",
    "operation.execute",
    "operation.preview.cancel",
    "session.undo",
    "session.redo",
    "protection.orbit.set",
    POLICY_ROW,
    RECORD_ROW,
    WORKSHEET_SAVE_ROW,
    WORKSHEET_LOAD_ROW,
    WORKSHEET_COMPARE_ROW,
];

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

fn word(letters: &[i64]) -> Value {
    json::parse(format!(r#"[{{"kind": "word", "moves": {letters:?}}}]"#).as_bytes()).unwrap()
}

fn parse(text: &str) -> Value {
    json::parse(text.as_bytes()).unwrap()
}

fn text(s: &str) -> Value {
    Value::String(s.to_owned())
}

fn overlay(entry: &str) -> Overlay {
    Overlay { module: "tools/representations".into(), entry: entry.into() }
}

fn preview_args(id: u64) -> Value {
    json::object([("preview", Value::Int(id as i64))])
}

fn running(receipt: &Receipt) -> bool {
    matches!(receipt.status, Status::Running(_))
}

/// The refusal's code, after checking that the refusal took no job.
fn refusal(receipt: &Receipt) -> &'static str {
    assert_eq!(receipt.job, None, "a refusal takes no job: {:?}", receipt.status);
    receipt.status.code().unwrap_or_else(|| panic!("not refused: {:?}", receipt.status))
}

struct Fixture {
    layer: CommandLayer,
    shell: Shell,
    frames: HeadlessFrames,
}

/// A session at the solved state that protects nothing, on H-08's table, with `served` served.
fn fixture(served: &[&str]) -> Fixture {
    let k = kernel();
    let (start, protected) = (k.solved(), k.protected_set(&[]).unwrap());
    let mut layer =
        CommandLayer::new(k, Table::h08(), Secret::fixed(SECRET), MemoryStore::default(), "s1", start, protected);
    for id in served {
        layer.serve_row(id, overlay(id)).unwrap();
    }
    Fixture { layer, shell: Shell::new(Secret::fixed(SECRET), "shell"), frames: HeadlessFrames::new() }
}

impl Fixture {
    /// Previews, presents and commits `recipe` through the command route.
    fn commit(&mut self, recipe: &Value) {
        let receipt = self.shell.invoke(&mut self.layer, "operation.preview", draft_args(recipe)).unwrap();
        let Status::Running(id) = receipt.status else { panic!("preview not admitted: {:?}", receipt.status) };
        assert_eq!(self.layer.run_jobs(), 1);
        self.frames.present(&mut self.shell, &mut self.layer, id).unwrap();
        let press = self.shell.prepare(&self.layer, "operation.execute", preview_args(id)).unwrap();
        let done = self.shell.send(&mut self.layer, press);
        assert!(matches!(done.status, Status::Committed(_)), "{:?}", done.status);
    }

    fn invoke(&mut self, row: &str, args: Value) -> Receipt {
        self.shell.invoke(&mut self.layer, row, args).unwrap()
    }

    /// A predicate a `structure.layer` job prepared that wrote no record, as the `FilterRef` that names it.
    fn predicate(&mut self) -> Value {
        let receipt = self.invoke(LAYER, json::object([("cell", Value::Int(1)), ("layer", Value::Int(2))]));
        let Status::Running(job) = receipt.status else { panic!("{:?}", receipt.status) };
        assert_eq!(self.layer.take_program_job().map(|taken| taken.job), Some(job));
        let done = self.layer.finish_program_job(job, Ok(Value::Null)).unwrap();
        assert!(matches!(done.status, Status::Completed(_)), "{:?}", done.status);
        json::object([("digest", Value::String(digest(&Value::Null))), ("job", Value::Int(job as i64))])
    }

    /// The rows of the program jobs waiting, in order, taken.
    fn take_all(&mut self) -> Vec<(String, Value)> {
        std::iter::from_fn(|| self.layer.take_program_job()).map(|job| (job.row, job.args)).collect()
    }
}

/// A served row's arguments have their types' forms: `CellIdPair` is two cells from 0 to 599, `CellId` one, `Nat` an
/// integer of at least 0, `Text` a string, and a choice of words one of its words. A malformed argument is refused with
/// `KN5:input.argument_form` and takes no job. `CanonicalObjectRefs` is a list of pieces or positions, each 0 to
/// 177,119 (the kernel's `PIECES`), empty and repeated members included (draft).
#[test]
fn a_served_rows_arguments_have_their_types_forms() {
    let mut f = fixture(&[COMPARE, LAYER, SAVE, SCOPE]);
    let cells = |pair: &str| json::object([("cells", parse(pair))]);
    for good in ["[0, 599]", "[7, 7]"] {
        assert!(running(&f.invoke(COMPARE, cells(good))), "{good}");
    }
    for bad in ["[0, 600]", "[-1, 2]", "[1]", "[1, 2, 3]", "[1, 2.5]", "[\"1\", \"2\"]", "\"1, 2\"", "null"] {
        assert_eq!(refusal(&f.invoke(COMPARE, cells(bad))), code::ARGUMENT_FORM, "{bad}");
    }

    let layer = |cell: &str, layer: &str| json::object([("cell", parse(cell)), ("layer", parse(layer))]);
    assert!(running(&f.invoke(LAYER, layer("599", "0"))));
    for (cell, depth) in [("600", "0"), ("-1", "0"), ("[3]", "0"), ("0", "-1"), ("0", "\"2\""), ("0", "1.0")] {
        assert_eq!(refusal(&f.invoke(LAYER, layer(cell, depth))), code::ARGUMENT_FORM, "{cell} {depth}");
    }

    let save =
        |kind: Value, members: Value, name: Value| json::object([("kind", kind), ("members", members), ("name", name)]);
    let last = (c600_kernel::PIECES - 1).to_string();
    let members_ok = ["[3, 5]".to_string(), "[]".to_string(), "[7, 7]".to_string(), format!("[0, {last}]")];
    for (kind, members) in [("identity", &members_ok[0]), ("position", &members_ok[1])] {
        assert!(running(&f.invoke(SAVE, save(text(kind), parse(members), text("pair")))), "{kind}");
    }
    for members in &members_ok[2..] {
        assert!(running(&f.invoke(SAVE, save(text("identity"), parse(members), text("pair")))), "{members}");
    }
    let past = c600_kernel::PIECES.to_string();
    for bad in ["[-1]", &format!("[3, {past}]"), "[1.5]", "[\"3\"]", "[[3]]", "[null]", "3", "\"[3]\"", "{}", "null"] {
        let receipt = f.invoke(SAVE, save(text("position"), parse(bad), text("pair")));
        assert_eq!(refusal(&receipt), code::ARGUMENT_FORM, "{bad}");
    }
    for (kind, name) in [(text("both"), text("pair")), (Value::Int(1), text("pair")), (text("identity"), Value::Int(7))]
    {
        let receipt = f.invoke(SAVE, save(kind.clone(), parse("[3, 5]"), name.clone()));
        assert_eq!(refusal(&receipt), code::ARGUMENT_FORM, "{kind:?} {name:?}");
    }

    for scope in [text("Body"), text("body|complete"), text(""), Value::Null] {
        let receipt = f.invoke(SCOPE, json::object([("scope", scope.clone())]));
        assert_eq!(refusal(&receipt), code::ARGUMENT_FORM, "{scope:?}");
    }
    assert!(running(&f.invoke(SCOPE, json::object([("scope", text("complete"))]))));

    let rows: Vec<String> = f.take_all().into_iter().map(|(row, _)| row).collect();
    assert_eq!(rows, [COMPARE, COMPARE, LAYER, SAVE, SAVE, SAVE, SAVE, SCOPE]);
}

/// An argument of an optional type may be left out: `recording.play`'s `from` is `Nat?`. Given, it has `Nat`'s form;
/// left out, the job's arguments do not hold it. Every other argument must be given, and no other name may be. The
/// names come before the row's arm, so a row the layer does not build is refused as not built once its names fit, with
/// its optional argument left out or given.
#[test]
fn an_optional_argument_may_be_left_out() {
    let mut f = fixture(&[PLAY]);
    f.layer.set_program_capacity(8);
    let fields =
        |pairs: &[(&str, Value)]| Value::Object(pairs.iter().map(|(k, v)| ((*k).to_owned(), v.clone())).collect());
    let given = [
        fields(&[("recording", text("r1"))]),
        fields(&[("from", Value::Int(0)), ("recording", text("r1"))]),
        fields(&[("from", Value::Int(12)), ("recording", text("r2"))]),
    ];
    for args in &given {
        assert!(running(&f.invoke(PLAY, args.clone())), "{args:?}");
    }
    let refused = [
        fields(&[("from", Value::Int(-1)), ("recording", text("r1"))]),
        fields(&[("from", text("2")), ("recording", text("r1"))]),
        fields(&[("from", Value::Null), ("recording", text("r1"))]),
        fields(&[("from", Value::Int(0))]),
        fields(&[]),
        fields(&[("recording", text("r1")), ("speed", Value::Int(2))]),
    ];
    for args in &refused {
        assert_eq!(refusal(&f.invoke(PLAY, args.clone())), code::ARGUMENT_FORM, "{args:?}");
    }
    let taken: Vec<Value> = f.take_all().into_iter().map(|(_, args)| args).collect();
    assert_eq!(taken, given);

    // `token.override.reset`, which nothing serves here, takes `name: TokenName` and `theme: ThemeFamilyId?`.
    // (`command.find`, which this test used before LP4-4 built it, is in `tests/palette.rs`.)
    for args in [fields(&[("name", text("accent"))]), fields(&[("name", text("accent")), ("theme", text("night"))])] {
        assert_eq!(refusal(&f.invoke(RESET, args.clone())), code::ROW_NOT_BUILT, "{args:?}");
    }
    for args in [fields(&[]), fields(&[("theme", text("night"))])] {
        assert_eq!(refusal(&f.invoke(RESET, args.clone())), code::ARGUMENT_FORM, "{args:?}");
    }
}

/// A `RevisionRef` names a revision: KN0.2's nine-key object, its text fields text and its counters naturals. A
/// malformed one is refused with `KN5:input.argument_form`; a well-formed one that is not the current revision in every
/// field, after a commit, after a workspace action or in another mode, is refused with `KN5:stale.revision_changed`
/// (EV4.14). Neither takes a job. The predicate is one a `structure.layer` job prepared, which stays valid throughout.
#[test]
fn a_revision_ref_names_the_current_revision() {
    let mut f = fixture(&[FILTER, LAYER]);
    let predicate = f.predicate();
    let filter = |revision: Value| json::object([("predicate", predicate.clone()), ("revision", revision)]);
    let first = f.layer.revision().to_json();
    assert!(running(&f.invoke(FILTER, filter(first.clone()))));

    let set = |key: &str, value: Value| {
        let Value::Object(mut fields) = first.clone() else { unreachable!("a revision is an object") };
        fields.insert(key.to_owned(), value);
        Value::Object(fields)
    };
    let without = |key: &str| {
        let Value::Object(mut fields) = first.clone() else { unreachable!("a revision is an object") };
        fields.remove(key);
        Value::Object(fields)
    };
    let malformed = [
        without("digest"),
        without("workspace"),
        set("extra", Value::Int(0)),
        set("state", text("0")),
        set("workspace", Value::Int(-1)),
        set("epoch", parse("1.5")),
        set("session", Value::Int(1)),
        set("mode", Value::Null),
        Value::Array(vec![first.clone()]),
        text("current"),
    ];
    for revision in &malformed {
        assert_eq!(refusal(&f.invoke(FILTER, filter(revision.clone()))), code::ARGUMENT_FORM, "{revision:?}");
    }
    let other_mode = set("mode", text("jumble_on"));
    assert_eq!(refusal(&f.invoke(FILTER, filter(other_mode))), code::REVISION_CHANGED);
    let later = set("workspace", Value::Int(f.layer.revision().workspace as i64 + 1));
    assert_eq!(refusal(&f.invoke(FILTER, filter(later))), code::REVISION_CHANGED);

    let policy = f.invoke(POLICY_ROW, json::object([("confirm", Value::Bool(true)), ("policy", text("strict"))]));
    assert!(matches!(policy.status, Status::Completed(_)), "{:?}", policy.status);
    let second = f.layer.revision().to_json();
    assert_ne!(second, first);
    assert_eq!(refusal(&f.invoke(FILTER, filter(first.clone()))), code::REVISION_CHANGED);
    assert!(running(&f.invoke(FILTER, filter(second.clone()))));

    f.commit(&word(&[1]));
    let third = f.layer.revision().to_json();
    for stale in [first, second] {
        assert_eq!(refusal(&f.invoke(FILTER, filter(stale))), code::REVISION_CHANGED);
    }
    assert!(running(&f.invoke(FILTER, filter(third.clone()))));
    assert_eq!(f.take_all(), [(FILTER.to_owned(), filter(third))]);
}

/// An overlay row's arguments have their declared types' forms too. Every form check comes before the revision check,
/// so a malformed request is never answered as stale, and both come before the lane's capacity. An optional type may
/// be left out, a choice of words may have spaces around its words, and a union of types, whose form is not written
/// down yet, passes unchecked.
#[test]
fn an_overlay_rows_forms_come_before_a_stale_revision() {
    let mut f = fixture(&[]);
    let arguments = [
        ("cell", "CellId"),
        ("draft", "ResolvedDraftRef|WordRef"),
        ("from", "Nat?"),
        ("revision", "RevisionRef"),
        ("target", "draft | source"),
    ];
    f.layer.register_overlay("tool.forms", Kind::Query, &arguments, overlay("forms")).unwrap();
    f.layer.set_program_capacity(1);
    let current = f.layer.revision().to_json();
    let mut stale = current.clone();
    if let Value::Object(fields) = &mut stale {
        fields.insert("state".to_owned(), Value::Int(9));
    }
    let args = |cell: i64, revision: &Value, target: &str| {
        json::object([
            ("cell", Value::Int(cell)),
            ("draft", Value::Int(5)),
            ("revision", revision.clone()),
            ("target", text(target)),
        ])
    };
    // The program lane's one place is taken, so a request that passes every check is refused as the lane is full.
    assert!(running(&f.invoke("tool.forms", args(5, &current, "source"))));
    assert_eq!(refusal(&f.invoke("tool.forms", args(600, &stale, "source"))), code::ARGUMENT_FORM);
    assert_eq!(refusal(&f.invoke("tool.forms", args(5, &stale, "both"))), code::ARGUMENT_FORM);
    assert_eq!(refusal(&f.invoke("tool.forms", args(5, &stale, "source"))), code::REVISION_CHANGED);
    assert_eq!(refusal(&f.invoke("tool.forms", args(599, &current, "draft"))), code::LANE_FULL);

    let job = f.layer.take_program_job().expect("the first request's job");
    assert_eq!(job.args, args(5, &current, "source"));
    assert!(f.layer.finish_program_job(job.job, Ok(Value::Null)).is_some());
    assert!(running(&f.invoke("tool.forms", args(599, &current, "draft"))));
    assert_eq!(f.take_all(), [("tool.forms".to_owned(), args(599, &current, "draft"))]);
}

/// None of the layer's own rows takes an optional argument, so every argument their arms read was given.
#[test]
fn the_layers_own_rows_take_no_optional_argument() {
    let table = Table::h08().with_drafts();
    for id in LAYER_ROWS {
        let row = table.row(id).unwrap_or_else(|| panic!("{id}"));
        assert!(row.arguments.values().all(|ty| !ty.ends_with('?')), "{id}: {:?}", row.arguments);
    }
}

//! Enumerations of the runtime's capability table (LP2, M2 and M3): the operations the IR offers, the requests and
//! sinks through which the interpreter reaches its host, and the values that come back.
//!
//! No crate keeps a listing of its operations, so the enumerations below are exhaustive matches over the crates' own
//! types: every variant and every field is named, with no wildcard and no `..`, so a new operation, request, sink,
//! type or value does not compile until it is classified here. Each classification is then checked against the
//! interpreter and the boundary adapter by running every operation. The session's tables (M6) and the command layer's
//! and the overlay's rows (LP3) are not built, so every enumeration here covers the runtime's half of its row only.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use c600_check::{admit, check, separation, Grant, Policy, TYPE_MISMATCH};
use c600_digest::sha256;
use c600_ir::build::{exact, fn_ref, helper, module, program, propose, publish_view, target, unit, write_workspace};
use c600_ir::{Capability, Effect, ExactRole, Expr, Lit, Program, QTy, ReceiptField, Ty};
use c600_run::boundary::{admit_preview, Boundary, Context, Pins, Place, Store, Stored};
use c600_run::interp::code::{IR_REJECTED, NONCANONICAL_VALUE};
use c600_run::interp::{load, Host, Loaded, Options, Request, RowTariff, Sink};
use c600_run::meter::code::{NOT_GRANTED, OUTPUT};
use c600_run::meter::{Amounts, Outcome};
use c600_run::provenance::{self, Author, Json, Pin, Producer, Profile};
use c600_run::value::Value;
use c600_syntax::{lower, parse};

const WIDE: Amounts = Amounts { steps: 1 << 40, kernel_cost: 1 << 40, memory: 1 << 40, output: 1 << 40 };
const RUST: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/..");
const ACK: &[u8] = b"{\"base\":\"rev-7\",\"id\":\"run/1\"}\n";

// ---------------------------------------------------------------------------------------------------------------------
// The table, as the crates define it.

/// What a row does. There is no commit kind and no adoption kind: no operation below classifies as either.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Kind {
    /// Evaluated inside the interpreter; the host is not asked.
    Pure,
    /// Display accounting, a draw from the seeded generator and a receipt field: interpreter rows without a host call.
    Accounting,
    Draw,
    ReceiptRead,
    /// Host rows: reads, then outputs, then the preview row.
    StateRead,
    FileRead,
    ClockRead,
    View,
    Workspace,
    FileWrite,
    ExactSink,
    Preview,
}

impl Kind {
    fn reaches_the_host(self) -> bool {
        !matches!(self, Kind::Pure | Kind::Accounting | Kind::Draw | Kind::ReceiptRead)
    }
}

/// Every IR operation, by variant: its name, its row kind and its operands. A literal, a name, a type, a field name, a
/// role or a receipt field is fixed in the program text; an operand is a child expression the checker types.
fn row(e: &Expr) -> (&'static str, Kind, &'static [&'static str]) {
    match e {
        Expr::Lit(_) => ("literal", Kind::Pure, &[]),
        Expr::Var(_) => ("variable", Kind::Pure, &[]),
        Expr::FnRef { module: _, name: _ } => ("function reference", Kind::Pure, &[]),
        Expr::Let { name: _, value: _, body: _ } => ("let", Kind::Pure, &["value", "body"]),
        Expr::If { cond: _, then: _, els: _ } => ("if", Kind::Pure, &["cond", "then", "els"]),
        Expr::Match { scrutinee: _, arms: _, default: _ } => ("match", Kind::Pure, &["scrutinee", "arms", "default"]),
        Expr::DisplayIf { cond: _, then: _, els: _ } => ("display_if", Kind::Pure, &["cond", "then", "els"]),
        Expr::Call { callee: _, args: _ } => ("call", Kind::Pure, &["callee", "args"]),
        Expr::Closure { params: _, result: _, body: _ } => ("closure", Kind::Pure, &["body"]),
        Expr::Prim { op: _, args: _ } => ("primitive", Kind::Pure, &["args"]),
        Expr::Record(_) => ("record", Kind::Pure, &["fields"]),
        Expr::Field { value: _, field: _ } => ("field", Kind::Pure, &["value"]),
        Expr::ReadSnapshot { query: _, ty: _ } => ("read_snapshot", Kind::StateRead, &[]),
        Expr::PublishView { value: _ } => ("publish_view", Kind::View, &["value"]),
        Expr::WriteWorkspace { record: _, ty: _, value: _ } => ("write_workspace", Kind::Workspace, &["value"]),
        Expr::Propose { word: _, target: _ } => ("propose", Kind::Preview, &["word", "target"]),
        Expr::ReadReceipt { receipt: _, field: _ } => ("read_receipt", Kind::ReceiptRead, &["receipt"]),
        Expr::UseExact { role: _, value: _ } => ("use_exact", Kind::ExactSink, &["value"]),
        Expr::Accounting(_) => ("accounting", Kind::Accounting, &[]),
        Expr::Declassified { id: _, ty: _ } => ("declassified", Kind::StateRead, &[]),
        Expr::FileRead { path: _, ty: _ } => ("file_read", Kind::FileRead, &[]),
        Expr::FileWrite { path: _, value: _ } => ("file_write", Kind::FileWrite, &["value"]),
        Expr::RandomDraw => ("random_draw", Kind::Draw, &[]),
        Expr::ReadClock => ("read_clock", Kind::ClockRead, &[]),
    }
}

/// What a field of a request or a sink carries. The bindings below are typed, so a field of another type, a handle or a
/// callable among them, does not compile.
enum Carries<'a> {
    Name(&'a str),
    Type(&'a Ty),
    Letters(&'a [i32]),
    Bytes(&'a [u8]),
    Role(ExactRole),
}

impl Carries<'_> {
    fn describe(&self) -> String {
        match self {
            Carries::Name(name) => format!("a name, {name}"),
            Carries::Type(ty) => format!("a type, whose values hold {}", holds(ty)),
            Carries::Letters(word) => format!("letters, {word:?}"),
            Carries::Bytes(bytes) => format!("canonical bytes, {} of them", bytes.len()),
            Carries::Role(role) => format!("an exact role, {role:?}"),
        }
    }
}

/// Every request the interpreter makes of its host, with every field.
fn request(r: &Request) -> (Kind, Vec<String>) {
    let (kind, fields) = match *r {
        Request::Snapshot { query, ty } => (Kind::StateRead, vec![Carries::Name(query), Carries::Type(&ty.ty)]),
        Request::Declassified { id, ty } => (Kind::StateRead, vec![Carries::Name(id), Carries::Type(ty)]),
        Request::FileRead { path, ty } => (Kind::FileRead, vec![Carries::Name(path), Carries::Type(ty)]),
        Request::Clock => (Kind::ClockRead, vec![]),
        Request::Propose { word, target, request } => {
            (Kind::Preview, vec![Carries::Letters(word), Carries::Name(target), Carries::Bytes(request)])
        }
    };
    (kind, fields.iter().map(Carries::describe).collect())
}

/// Every sink, with every field. The value travels beside it: for every sink but the exact one, as canonical bytes.
fn sink(s: &Sink) -> (Kind, Vec<String>) {
    let (kind, fields) = match *s {
        Sink::View => (Kind::View, vec![]),
        Sink::Workspace { record, ty } => (Kind::Workspace, vec![Carries::Name(record), Carries::Type(&ty.ty)]),
        Sink::File { path } => (Kind::FileWrite, vec![Carries::Name(path)]),
        Sink::Exact { role } => (Kind::ExactSink, vec![Carries::Role(role)]),
    };
    (kind, fields.iter().map(Carries::describe).collect())
}

/// Every IR type, by what a value of it holds. None holds a state, a revision, a digest, a token or a handle.
fn holds(ty: &Ty) -> &'static str {
    match ty {
        Ty::Unit | Ty::Bool | Ty::ApproxBool => "a truth value or nothing",
        Ty::Nat | Ty::Int | Ty::Rat | Ty::Q5 | Ty::Budget | Ty::Approx(_) => "a number",
        Ty::Text | Ty::Status | Ty::Colour => "a text",
        Ty::Word | Ty::Target => "letters or a target's name",
        Ty::Record(_) => "named values of these types",
        Ty::Fn(_) => "a function of the program, which runs only inside the interpreter",
        Ty::Receipt => "a preview receipt, read only through its fields",
        Ty::Var(_) => "a type parameter",
    }
}

/// Every run-time value, as [`holds`] lists types.
fn value_holds(v: &Value) -> &'static str {
    match v {
        Value::Unit | Value::Bool(_) | Value::ApproxBool(_) => "a truth value or nothing",
        Value::Int(_) | Value::Rat(_) | Value::Q5(_) | Value::Approx(_) => "a number",
        Value::Text(_) | Value::Tag(_) | Value::Colour(_) => "a text",
        Value::Word(_) | Value::Target(_) => "letters or a target's name",
        Value::Record(_) => "named values",
        Value::Closure(_) | Value::Fn(_) => "a function of the program, which runs only inside the interpreter",
        Value::Receipt(_) => "a preview receipt, read only through its fields",
    }
}

/// Every receipt field a program can name (BND.42). None is a token.
fn receipt_field(f: ReceiptField) -> &'static str {
    match f {
        ReceiptField::Id => "id",
        ReceiptField::Base => "base",
        ReceiptField::Effect => "effect",
        ReceiptField::Protection => "protection",
        ReceiptField::EndState => "end_state",
        ReceiptField::N => "n",
        ReceiptField::Error => "error",
    }
}

/// LG3.1's effect families, and LG3.8's capabilities. There is no commit effect (LG3.4).
fn effect(e: Effect) -> &'static str {
    match e {
        Effect::ReadSnapshot => "snapshot",
        Effect::ChangeView => "view",
        Effect::ChangeWorkspace => "workspace",
        Effect::PreparePreview => "preview",
        Effect::GrantedFile => "file",
        Effect::SeededRandomness => "random",
        Effect::Budgets => "budget",
    }
}

fn capability(c: Capability) -> &'static str {
    match c {
        Capability::File => "file",
        Capability::Clock => "clock",
        Capability::Randomness => "random",
        Capability::Network => "network",
        Capability::Process => "process",
        Capability::SessionFile => "session_file",
    }
}

// ---------------------------------------------------------------------------------------------------------------------
// Hosts.

/// The engine side: it answers every read with a number and every proposal with a receipt that also carries the
/// fields a program must never see.
#[derive(Default)]
struct Engine {
    log: Vec<String>,
    requests: Vec<Vec<u8>>,
}

impl Host for Engine {
    fn tariff(&mut self, _: &Request) -> Result<RowTariff, &'static str> {
        Ok(RowTariff { kernel_cost: 9, result_bound: 4096 })
    }

    fn perform(&mut self, r: &Request) -> Result<Value, &'static str> {
        self.log.push(format!("{:?}", request(r).0));
        match *r {
            Request::Propose { request, .. } => {
                self.requests.push(request.to_vec());
                Ok(Value::receipt(vec![
                    ("base", Value::text("rev-7")),
                    ("commit", Value::text("commit-1")),
                    ("id", Value::text("preview-1")),
                    ("token", Value::text("token-1")),
                ]))
            }
            _ => Ok(Value::int(3)),
        }
    }

    fn sink(&mut self, s: &Sink, _: &Value, _: &[u8]) -> Result<(), &'static str> {
        self.log.push(format!("{:?}", sink(s).0));
        Ok(())
    }
}

/// Sits between the interpreter and its host and records every row the interpreter hands over, and every receipt that
/// comes back to the program.
struct Tap<'a> {
    inner: &'a mut dyn Host,
    rows: Vec<Kind>,
    receipts: Vec<Value>,
    sinks: Vec<Value>,
}

impl<'a> Tap<'a> {
    fn new(inner: &'a mut dyn Host) -> Self {
        Tap { inner, rows: Vec::new(), receipts: Vec::new(), sinks: Vec::new() }
    }
}

impl Host for Tap<'_> {
    fn tariff(&mut self, r: &Request) -> Result<RowTariff, &'static str> {
        self.inner.tariff(r)
    }

    fn perform(&mut self, r: &Request) -> Result<Value, &'static str> {
        let (kind, _) = request(r);
        self.rows.push(kind);
        let got = self.inner.perform(r);
        if let (Kind::Preview, Ok(v)) = (kind, &got) {
            self.receipts.push(v.clone());
        }
        got
    }

    fn sink(&mut self, s: &Sink, v: &Value, bytes: &[u8]) -> Result<(), &'static str> {
        self.rows.push(sink(s).0);
        self.sinks.push(v.clone());
        self.inner.sink(s, v, bytes)
    }
}

fn contract() -> Pin {
    Pin::of("fixture-1", b"p3 fixture capability contract 1\n")
}

fn pins() -> Pins {
    Pins { contracts: vec![contract()], checkers: vec![] }
}

fn context(source: &str, entry: &str, author: Author, profile: Profile, reads_state: bool) -> Context {
    let producer = Producer {
        contract: contract(),
        checker: None,
        author,
        module: sha256(source.as_bytes()),
        entry: entry.into(),
        profile,
    };
    let manual = provenance::canonical(&provenance::manual(&contract()));
    Context { run: "run".into(), base: "rev-7".into(), producer, reads_state, inputs: vec![manual], pins: pins() }
}

fn lowered(source: &str) -> Program {
    let file = parse(source.as_bytes());
    assert!(file.diagnostics.is_empty(), "{:#?}", file.diagnostics);
    lower(&[file]).unwrap_or_else(|e| panic!("{e}")).program
}

fn build(source: &str, entry: &str) -> (Program, Loaded, bool) {
    let program = lowered(source);
    let report = check(&program).unwrap_or_else(|e| panic!("{e}"));
    assert!(report.is_admitted(), "{:#?}", report.diagnostics);
    let reads = separation(report.summary("m", entry).expect("the entry"), &Policy::interim()).reads_state;
    let loaded = load(&program).unwrap_or_else(|r| panic!("{r:?}"));
    (program, loaded, reads)
}

fn body<'p>(program: &'p Program, entry: &str) -> &'p Expr {
    let m = program.modules.iter().find(|m| m.name == "m").expect("module m");
    &m.functions.iter().find(|f| f.name == entry).expect("the entry").body
}

/// The kinds of every node of an expression, in path order.
fn kinds(e: &Expr, out: &mut Vec<Kind>) {
    out.push(row(e).1);
    for child in e.children() {
        kinds(child, out);
    }
}

// ---------------------------------------------------------------------------------------------------------------------
// Every operation, run.

/// Every effectful operation of the draft, once each, in a straight line.
const EVERY: &str = "edition \"6cs-draft-1\";\nmodule m requires [file, clock, random];\n\n\
    export fn every(w: Word) -> record { base: Text, id: Text } \
    effects [snapshot, view, workspace, preview, budget, file, random, clock] {\n    \
    let n = read_snapshot<Nat>(\"n_c0\");\n    let d = declassified<Nat>(\"example\");\n    \
    let f = file_read<Nat>(\"count\");\n    let c = read_clock();\n    let r = random_draw();\n    \
    let v = publish_view(record { used = used_steps(), left = unused_steps(), status = budget_status(), \
    now = c });\n    \
    let k = write_workspace<Word>(\"kept\", w);\n    let b = use_budget(n);\n    let g = use_goal(target(\"t\"));\n    \
    let e = use_evidence(record { n = n, d = d, f = f, r = r });\n    \
    let o = file_write(\"out.6cw\", w);\n    let p = propose(w, target(\"t\"));\n    \
    return record { base = receipt_base(p), id = receipt_id(p) };\n}\n";

struct Every {
    rows: Vec<Kind>,
    receipts: Vec<Value>,
    engine: Engine,
    store: Store,
    output: Option<Vec<u8>>,
    ops: Vec<String>,
}

/// Runs [`EVERY`] through the boundary adapter as the given author and profile.
fn every(author: Author, profile: Profile) -> Every {
    let (_, loaded, reads) = build(EVERY, "every");
    let (mut engine, mut store) = (Engine::default(), Store::default());
    let seed = [7u8; 32];
    let path = ["tables", "every"];
    let options = Options { seed: Some((&seed, &path)), ..Options::new(WIDE) };
    let (run, rows, receipts) = {
        let mut boundary = Boundary::new(&mut engine, &mut store, context(EVERY, "every", author, profile, reads));
        let mut tap = Tap::new(&mut boundary);
        let run = loaded.run("m", "every", vec![Value::word(&[1, -2])], &mut tap, &options).expect("a run");
        (run, tap.rows, tap.receipts)
    };
    assert_eq!(run.summary.outcome, Outcome::Completed, "{:?}", run.summary.stop);
    let ops = run.summary.rows.iter().map(|r| r.at.rsplit('#').next().unwrap_or_default().to_string()).collect();
    Every { rows, receipts, engine, store, output: run.output, ops }
}

const CLASSES: [(Author, Profile); 4] = [
    (Author::BuiltIn, Profile::Solving),
    (Author::Own, Profile::Solving),
    (Author::Own, Profile::ToolAuthoring),
    (Author::BuiltIn, Profile::Research),
];

/// CT-KN5-02 (part: the runtime's half of the table, the IR's operations and the interpreter's host rows; not run: the
/// session's table rows, which the session store builds (M6), and the command layer's and the overlay's rows (LP3)) and
/// CT-ONE-02 (part: the runtime's half; the session's rows run in `c600-session/tests/conformance_rows.rs`; not run:
/// the command layer's and the overlay's rows (LP3)): no operation in the runtime's table returns a token or calls
/// `commit`, and no row is of kind `commit`, in any profile. The IR's operations, the interpreter's requests and sinks,
/// LG3.1's effects and the receipt's fields are enumerated from the crates' own types, and none is a commit; the `Host`
/// trait has three rows, a tariff, a request and a sink, and a fourth required row would not compile here. Every
/// effectful operation, run through the boundary adapter as a built-in tool and a solver's own tool in a solving
/// session, in tool authoring and in research, reaches the host exactly as the table classifies it; workspace records
/// and files stay in the adapter's store, the engine is asked for no row of another kind, the meter records no `commit`
/// operation, and every receipt the program receives holds the ID and the base only, although the engine's receipt also
/// carries `token` and `commit` fields.
#[test]
fn ct_kn5_02_no_operation_returns_a_token_or_commits_in_any_profile() {
    let effects = [
        Effect::ReadSnapshot,
        Effect::ChangeView,
        Effect::ChangeWorkspace,
        Effect::PreparePreview,
        Effect::GrantedFile,
        Effect::SeededRandomness,
        Effect::Budgets,
    ];
    assert!(effects.iter().all(|&e| !effect(e).contains("commit")));
    let fields = [
        ReceiptField::Id,
        ReceiptField::Base,
        ReceiptField::Effect,
        ReceiptField::Protection,
        ReceiptField::EndState,
        ReceiptField::N,
        ReceiptField::Error,
    ];
    assert!(fields.iter().all(|&f| !["token", "commit"].contains(&receipt_field(f))));

    let (program, _, _) = build(EVERY, "every");
    let mut table = Vec::new();
    kinds(body(&program, "every"), &mut table);
    let mut expected: Vec<Kind> = table.into_iter().filter(|k| k.reaches_the_host()).collect();
    expected.sort();
    let all: BTreeSet<Kind> = expected.iter().copied().collect();
    assert_eq!(all.len(), 8, "every host row kind is exercised: {all:?}");

    for (author, profile) in CLASSES {
        let ran = every(author, profile);
        let mut rows = ran.rows.clone();
        rows.sort();
        assert_eq!(rows, expected, "{author:?} in {profile:?}");
        assert_eq!(ran.output.as_deref(), Some(ACK), "{author:?} in {profile:?}");
        assert!(ran.ops.iter().all(|op| !op.contains("commit")), "{:?}", ran.ops);
        assert_eq!(ran.ops.iter().filter(|op| *op == "invoke").count(), 1, "one preview row");
        assert!(ran.engine.log.iter().all(|l| !l.to_lowercase().contains("commit")), "{:?}", ran.engine.log);
        for receipt in &ran.receipts {
            let Value::Receipt(r) = receipt else { panic!("a receipt") };
            let names: Vec<&str> = r.fields().iter().map(|(k, _)| k.as_str()).collect();
            assert_eq!(names, ["base", "id"], "{author:?} in {profile:?}");
        }
        let previewed = profile == Profile::Solving && author == Author::BuiltIn;
        assert_eq!(ran.engine.requests.len(), usize::from(previewed), "{author:?} in {profile:?}");
        let places: Vec<_> = ran.store.records.keys().cloned().collect();
        assert_eq!(places, [Place::Workspace("kept".into()), Place::File("out.6cw".into())], "the adapter's store");
        assert!(ran.engine.log.iter().all(|l| l != "Workspace" && l != "FileWrite"), "{:?}", ran.engine.log);
    }
}

/// CT-ONE-01 (part: the runtime's half of the table; the session's rows run in
/// `c600-session/tests/conformance_rows.rs`; not run: the command layer's and the overlay's rows (LP3)): no operation
/// takes a state or a digest to adopt. No IR type and no run-time value holds a state, a revision, a digest or a token,
/// so no operand can carry one. The one row that reaches the engine's preview, `propose`, takes a word and a target and
/// nothing else: its request carries the letters, the target's name and the canonical bytes of exactly `{target,
/// word}`; a program that passes the receipt's base in place of the word is refused by the checker, and the same IR
/// built by hand is refused by the interpreter before the host is asked. The base a proposal reports comes from the
/// run's context, never from the program.
#[test]
fn ct_one_01_no_operation_takes_a_state_or_a_digest_to_adopt() {
    let types = [Ty::Unit, Ty::Nat, Ty::Text, Ty::Word, Ty::Target, Ty::Receipt, Ty::Record(vec![])];
    assert!(types.iter().all(|t| !["state", "digest", "token", "revision"].iter().any(|w| holds(t).contains(w))));
    let values = [Value::Unit, Value::int(1), Value::text("rev-7"), Value::word(&[1]), Value::receipt(vec![])];
    assert!(values.iter().all(|v| !["state", "digest", "token"].iter().any(|w| value_holds(v).contains(w))));
    assert_eq!(row(&propose(Expr::Lit(Lit::Word(vec![1])), target("t"))).2, ["word", "target"]);

    for (author, profile) in CLASSES {
        let ran = every(author, profile);
        for bytes in &ran.engine.requests {
            let json = provenance::parse(bytes).expect("canonical request bytes");
            let Json::Object(map) = json else { panic!("an object") };
            assert_eq!(map.keys().collect::<Vec<_>>(), ["target", "word"]);
            assert_eq!(map["word"], Json::List(vec![Json::Int(1), Json::Int(-2)]));
        }
        let base: Vec<_> = ran.receipts.iter().map(|r| format!("{r:?}")).collect();
        assert!(base.iter().all(|r| r.contains("rev-7")), "{base:?}");
    }

    let adopt = "edition \"6cs-draft-1\";\nmodule m requires [];\n\n\
        export fn adopt(w: Word) -> Text effects [preview] {\n    \
        let p = propose(w, target(\"t\"));\n    return receipt_id(propose(receipt_base(p), target(\"t\")));\n}\n";
    let report = check(&lowered(adopt)).unwrap();
    assert!(!report.is_admitted() && report.codes().contains(&TYPE_MISMATCH), "{:#?}", report.diagnostics);

    let hand = program(vec![module(
        "m",
        vec![helper("adopt", vec![], exact(Ty::Receipt), propose(Expr::Lit(Lit::Text("rev-7".into())), target("t")))
            .exported()],
    )]);
    let mut engine = Engine::default();
    let run = load(&hand).unwrap().run("m", "adopt", vec![], &mut engine, &Options::new(WIDE)).unwrap();
    assert_eq!(run.summary.code, Some(IR_REJECTED));
    assert!(engine.log.is_empty() && engine.requests.is_empty());
}

// ---------------------------------------------------------------------------------------------------------------------
// The renderer boundary.

/// The workspace's members, from its manifest.
fn members() -> Vec<String> {
    let manifest = std::fs::read_to_string(format!("{RUST}/Cargo.toml")).unwrap();
    let start = manifest.find("\nmembers").expect("a members list");
    let open = start + manifest[start..].find('[').unwrap() + 1;
    let list = &manifest[open..open + manifest[open..].find(']').unwrap()];
    list.split(',').map(|m| m.trim().trim_matches('"').to_string()).filter(|m| !m.is_empty()).collect()
}

/// Every Rust source file under a member's `src`.
fn sources(dir: &Path, out: &mut Vec<PathBuf>) {
    let mut entries: Vec<_> = std::fs::read_dir(dir).unwrap().map(|e| e.unwrap().path()).collect();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            sources(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

/// The lines of the product's sources that are not whole-line comments, by file.
fn code_lines() -> Vec<(PathBuf, usize, String)> {
    let mut files = Vec::new();
    for member in members() {
        sources(&Path::new(RUST).join(&member).join("src"), &mut files);
    }
    let mut out = Vec::new();
    for file in files {
        let text = std::fs::read_to_string(&file).unwrap();
        for (i, line) in text.lines().enumerate() {
            if !line.trim_start().starts_with("//") {
                out.push((file.clone(), i + 1, line.to_string()));
            }
        }
    }
    out
}

/// The packages of `Cargo.lock`: name, version and whether a registry supplies it.
fn locked() -> Vec<(String, String, bool)> {
    let lock = std::fs::read_to_string(format!("{RUST}/Cargo.lock")).unwrap();
    lock.split("[[package]]")
        .skip(1)
        .map(|block| {
            let value = |key: &str| {
                block
                    .lines()
                    .find_map(|l| l.strip_prefix(&format!("{key} = \"")))
                    .map(|v| v.trim_end_matches('"').to_string())
            };
            (value("name").unwrap(), value("version").unwrap(), value("source").is_some())
        })
        .collect()
}

/// A program that hands a function of its own to every output.
fn handing(output: Expr) -> Program {
    let f = helper("f", vec![], exact(Ty::Unit), unit());
    program(vec![module("m", vec![f, helper("hand", vec![], exact(Ty::Unit), output).exported()])])
}

/// CT-RPI-01 (part: the boundary enumeration; not run: switching a mock backend under a display plan, since display
/// plans, the adapter and the backends are built in LP3): the language has no renderer call, backend handle or callback
/// (RPI.1). The interpreter's requests and sinks carry names, types, letters, canonical bytes and an exact role only,
/// as their typed enumeration shows; no IR type or run-time value is a handle; a function of the program never reaches
/// a view, workspace or file sink, since the canonical encoding refuses it before the host is asked. Every package in
/// `Cargo.lock` is a workspace member or one of the four approved numeric crates, so no renderer, graphics or windowing
/// crate is linked; every member forbids unsafe code and no product source holds an `unsafe` block or an `extern`
/// declaration, so nothing calls a foreign renderer API.
#[test]
fn ct_rpi_01_no_renderer_call_handle_or_callback_crosses_the_boundary() {
    let approved = [("autocfg", "1.5.1"), ("num-bigint", "0.5.1"), ("num-integer", "0.1.47"), ("num-traits", "0.2.19")];
    let members = members();
    for (name, version, registry) in locked() {
        if registry {
            assert!(approved.contains(&(name.as_str(), version.as_str())), "{name} {version} is not approved");
        } else {
            assert!(members.contains(&name), "{name} is a path package outside the workspace");
        }
    }
    for member in &members {
        let root = std::fs::read_to_string(Path::new(RUST).join(member).join("src/lib.rs")).unwrap();
        assert!(root.contains("#![forbid(unsafe_code)]"), "{member}");
    }
    for (file, line, text) in code_lines() {
        let code = text.replace("#![forbid(unsafe_code)]", "");
        assert!(!code.contains("unsafe") && !code.contains("extern \""), "{}:{line}: {text}", file.display());
    }

    let qty = QTy { ty: Ty::Word, display: false };
    let requests = [
        Request::Snapshot { query: "n_c0", ty: &qty },
        Request::Declassified { id: "example", ty: &Ty::Nat },
        Request::FileRead { path: "count", ty: &Ty::Nat },
        Request::Clock,
        Request::Propose { word: &[1], target: "t", request: b"{}" },
    ];
    let sinks = [
        Sink::View,
        Sink::Workspace { record: "kept", ty: &qty },
        Sink::File { path: "out" },
        Sink::Exact { role: ExactRole::Goal },
    ];
    let carried: Vec<String> =
        requests.iter().flat_map(|r| request(r).1).chain(sinks.iter().flat_map(|s| sink(s).1)).collect();
    let kinds = ["a name", "a type", "letters", "canonical bytes", "an exact role"];
    assert!(carried.iter().all(|c| kinds.iter().any(|k| c.starts_with(k))), "{carried:?}");
    let types =
        [Ty::Fn(Box::new(c600_ir::FnTy { params: vec![], result: exact(Ty::Unit), effects: Default::default() }))];
    assert!(types.iter().all(|t| holds(t).contains("only inside the interpreter")));

    let closure = c600_ir::build::closure(vec![], exact(Ty::Unit), unit());
    let outputs = [
        publish_view(fn_ref("f")),
        write_workspace("kept", exact(Ty::Unit), closure.clone()),
        Expr::FileWrite { path: "out".into(), value: Box::new(fn_ref("f")) },
        publish_view(closure),
    ];
    for output in outputs {
        let mut engine = Engine::default();
        let run = load(&handing(output)).unwrap().run("m", "hand", vec![], &mut engine, &Options::new(WIDE)).unwrap();
        assert_eq!(run.summary.code, Some(NONCANONICAL_VALUE));
        assert!(engine.log.is_empty(), "{:?}", engine.log);
    }
}

// ---------------------------------------------------------------------------------------------------------------------
// Words and claims.

/// Keeps a word and hands a claim to the evidence sink; the policy and the admission never see either.
const KEEPER: &str = "edition \"6cs-draft-1\";\nmodule m requires [];\n\n\
    export fn keep(w: Word, claim: Nat) -> Unit effects [workspace] {\n    \
    let e = use_evidence(claim);\n    return write_workspace<Word>(\"kept\", w);\n}\n";

/// CT-EVD-21 (part: the policy, the grants, the pins, preview admission and the run's records; not run: the commit
/// path, which the command layer and the session build (LP3, M6)): no policy row, capability or admission reads a word
/// or a claim (EVD.56). The policy holds declassifier IDs, a grant holds capabilities, the pins hold contract and
/// checker pins, and a run's context holds its producer, its state-read fact and its inputs' blocks, as their typed
/// destructuring shows. Preview admission reads blocks only: stores with the same blocks and different records give
/// the same answers. A run that keeps different words and hands different claims to the evidence sink stores the same
/// block, and the checker admits two programs that differ only in a word and a claim alike.
#[test]
fn ct_evd_21_no_policy_row_capability_or_admission_reads_a_word_or_a_claim() {
    let Policy { enabled_declassifiers } = Policy::enabling(&["example"]);
    let _: &BTreeSet<String> = &enabled_declassifiers;
    let Grant { capabilities } = Grant::of(&[Capability::File]);
    let _: &BTreeSet<Capability> = &capabilities;
    assert!(capabilities.iter().all(|&c| !["word", "claim"].contains(&capability(c))));
    let Pins { contracts, checkers } = pins();
    let _: (&Vec<Pin>, &Vec<Pin>) = (&contracts, &checkers);
    let Context { run, base, producer, reads_state, inputs, pins: _ } =
        context(KEEPER, "keep", Author::BuiltIn, Profile::Solving, false);
    let _: (&String, &String, &bool, &Vec<Vec<u8>>) = (&run, &base, &reads_state, &inputs);
    let Producer { contract: _, checker: _, author: _, module, entry: _, profile: _ } = producer;
    let _: [u8; 32] = module;

    let manual = provenance::canonical(&provenance::manual(&contract()));
    let own = provenance::canonical(&provenance::program(
        &Producer {
            contract: contract(),
            checker: None,
            author: Author::Own,
            module: [1; 32],
            entry: "e".into(),
            profile: Profile::Solving,
        },
        false,
        &[],
    ));
    let answers = |records: &[(&str, Value)]| {
        let mut store = Store::default();
        for (name, value) in records {
            store.records.insert(
                Place::Workspace(name.to_string()),
                Stored { value: value.clone(), bytes: b"[1]\n".to_vec(), block: own.clone() },
            );
        }
        store.keep(&own);
        [&manual, &own].map(|b| admit_preview(b, &store, &pins()).map_err(|r| r.code))
    };
    assert_eq!(answers(&[]), answers(&[("kept", Value::word(&[5, -6])), ("claim", Value::int(9))]));
    assert_eq!(answers(&[]), [Ok(()), Err(NOT_GRANTED)]);

    let (_, loaded, reads) = build(KEEPER, "keep");
    let mut blocks = Vec::new();
    for (letters, claim) in [(&[1, -2][..], 1), (&[7][..], 1_000_000)] {
        let (mut engine, mut store) = (Engine::default(), Store::default());
        let mut boundary =
            Boundary::new(&mut engine, &mut store, context(KEEPER, "keep", Author::BuiltIn, Profile::Solving, reads));
        let args = vec![Value::word(letters), Value::int(claim)];
        loaded.run("m", "keep", args, &mut boundary, &Options::new(WIDE)).unwrap();
        drop(boundary);
        blocks.push(store.records[&Place::Workspace("kept".into())].block.clone());
        assert_eq!(engine.log, ["ExactSink"]);
    }
    assert_eq!(blocks[0], blocks[1]);

    let variant =
        KEEPER.replace("use_evidence(claim)", "use_evidence(42)").replace("(\"kept\", w)", "(\"kept\", word[+3])");
    let admitted: Vec<_> = [KEEPER, variant.as_str()]
        .iter()
        .map(|source| {
            let a =
                admit(&lowered(source), "m", "keep", c600_check::Profile::Solving, &Grant::none(), &Policy::interim())
                    .unwrap();
            (a.codes(), a.separation, a.open_cells)
        })
        .collect();
    assert_eq!(admitted[0], admitted[1]);
}

// ---------------------------------------------------------------------------------------------------------------------
// Reserved and retired codes.

/// CT-BND-24 (whole, with `c600-check`'s `ct_bnd_24_flow_refused_is_never_emitted_before_lq2`) and CT-ERR-15 (part: the
/// retired and unset flow codes, the recursion reservation and the plan reservation in LP3's headless display
/// contract; not run: the renderer behind its mock backend, which LP3 builds later): no product source of any
/// workspace member names `flow_refused` or `recursion_exhausted` outside a comment, so neither `LG3:effect.flow_refused`
/// (retired, ERR.24) nor `LG3:capability.flow_refused` (unset until LQ2) nor ERR.26's reservation can be emitted.
/// `display_plan_limit` appears once, as `c600-display`'s constant for ERR.25's reservation, and the sources only
/// compare a code with that constant, so none builds `KN7:budget.display_plan_limit` (RM.13). Deep recursion ends at a
/// budget of the settled meter, and an output over its budget stops with `KN7:budget.output`.
#[test]
fn ct_bnd_24_retired_and_reserved_codes_are_never_emitted() {
    let lines = code_lines();
    assert!(lines.len() > 1000, "the scan reads the product's sources");
    let mut named = Vec::new();
    for (file, line, text) in &lines {
        for token in ["flow_refused", "recursion_exhausted"] {
            assert!(!text.contains(token), "{}:{line}: {text}", file.display());
        }
        if text.contains("display_plan_limit") {
            named.push(format!("{}:{line}", file.display()));
        }
        if text.contains("PLAN_LIMIT") {
            let allowed = text.trim() == "pub const PLAN_LIMIT: &str = \"KN7:budget.display_plan_limit\";"
                || text.contains("== codes::PLAN_LIMIT");
            assert!(allowed, "{}:{line} uses the reserved plan budget: {text}", file.display());
        }
    }
    assert_eq!(named.len(), 1, "one reservation constant: {named:?}");
    assert!(named[0].replace('\\', "/").contains("c600-display/src/error.rs"), "{named:?}");

    let deep = "edition \"6cs-draft-1\";\nmodule m requires [];\n\n\
        export fn down(n: Nat) -> Nat effects [] {\n    return down(exact_add(n, 1));\n}\n";
    let (_, loaded, _) = build(deep, "down");
    let small = Amounts { steps: 20_000, kernel_cost: 1 << 20, memory: 1 << 16, output: 1 << 16 };
    let run = loaded.run("m", "down", vec![Value::int(0)], &mut Engine::default(), &Options::new(small)).unwrap();
    let code = run.summary.code.expect("a stop");
    assert!(code.starts_with("KN7:budget."), "{code}");

    let wide = "edition \"6cs-draft-1\";\nmodule m requires [];\n\n\
        export fn wide() -> Unit effects [view] {\n    \
        return publish_view(\"a text longer than the output budget\");\n}\n";
    let (_, loaded, _) = build(wide, "wide");
    let tight = Amounts { output: 8, ..WIDE };
    let mut engine = Engine::default();
    let run = loaded.run("m", "wide", vec![], &mut engine, &Options::new(tight)).unwrap();
    assert_eq!(run.summary.code, Some(OUTPUT));
    assert!(engine.log.is_empty());
}

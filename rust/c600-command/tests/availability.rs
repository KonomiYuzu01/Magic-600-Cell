//! A language-defined command's availability, computed with no language code running (KN5.5, KN5.7, LI.18 to LI.21).
//! The layer answers an overlay row from its stored metadata ([`Metadata`]) and its own state. The overlay rows here
//! name the entries of a module that does not exist, and the layer's crate reaches no language crate, so no answer can
//! have come from running language code. Where the frozen text leaves a choice open, these tests follow the draft
//! rulings recorded in `src/lib.rs` ("Overlay availability").
//!
//! LI.20's two other examples are covered by its general rule, since the headless layer has neither row. No row the
//! runtime may invoke answers `KN5:input.menu_word_needs_jumble_on` from its argument-independent gates, so the tests
//! show a required row's own code passing through unchanged with `KN5:input.lane_full`, `KN5:input.argument_form` and
//! `KN5:stale.revision_changed`; and the busy optional row is `operation.preview` rather than a repair-list row.
//!
//! CT-KN5-08 (source, whole): every outcome of LI.20's conjunction and its order, answered from data alone; LI.20's
//! static and computed vector; optional rows; no program, job or lane taken by a hint; the generation (LI.21); and the
//! crate's dependency closure.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use c600_command::{
    code, digest, draft_args, Binding, Caller, CommandLayer, Dependency, Error, Fate, HeadlessFrames, Invocation, Kind,
    MemoryStore, Metadata, Overlay, Receipt, Secret, Shell, Status, Table,
};
use c600_kernel::json::{self, Value};
use c600_kernel::Kernel;

const SECRET: [u8; 32] = [7; 32];

const COMPARE: &str = "structure.compare";
const FILTER: &str = "filter.preview";
const LAYER: &str = "structure.layer";
const PREVIEW: &str = "operation.preview";
const EXECUTE: &str = "operation.execute";
const ORBITS: &str = "protection.orbit.set";
const MISSING: &str = "structure.missing";

const READ: &str = "user.read";
const WRITE: &str = "user.write";
const VIEW: &str = "user.view";

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

fn text(s: &str) -> Value {
    Value::String(s.to_owned())
}

fn cells(a: i64, b: i64) -> Value {
    json::object([("cells", Value::Array(vec![Value::Int(a), Value::Int(b)]))])
}

fn preview_args(id: u64) -> Value {
    json::object([("preview", Value::Int(id as i64))])
}

/// An entry of a module that does not exist. The layer never looks a module up.
fn absent(entry: &str) -> Overlay {
    Overlay { module: "absent/module".into(), entry: entry.into() }
}

fn required(row: &str, binding: Binding) -> Dependency {
    Dependency { row: row.into(), binding, optional: false }
}

fn optional(row: &str, binding: Binding) -> Dependency {
    Dependency { row: row.into(), binding, optional: true }
}

fn requiring(requires: Vec<Dependency>) -> Metadata {
    Metadata { diagnostic: None, requires }
}

fn code_of(answer: Result<(), Error>) -> &'static str {
    answer.expect_err("an unavailable row").code
}

struct Fixture {
    layer: CommandLayer,
    shell: Shell,
    frames: HeadlessFrames,
}

/// A session at the solved state on H-08's table. A built-in tool's entries serve `structure.compare` and
/// `filter.preview`, so the layer builds them, and three overlay rows name entries of a module that does not exist: a
/// query and a workspace row on the program lane, and a view row on the view lane.
fn fixture() -> Fixture {
    let k = kernel();
    let (start, protected) = (k.solved(), k.protected_set(&[]).unwrap());
    let mut layer =
        CommandLayer::new(k, Table::h08(), Secret::fixed(SECRET), MemoryStore::default(), "s1", start, protected);
    let tool = |entry: &str| Overlay { module: "tools/full".into(), entry: entry.into() };
    layer.serve_row(COMPARE, tool("compare_cells")).unwrap();
    layer.serve_row(FILTER, tool("filter")).unwrap();
    layer.register_overlay(READ, Kind::Query, &[], absent("read")).unwrap();
    layer.register_overlay(WRITE, Kind::Workspace, &[], absent("write")).unwrap();
    layer.register_overlay(VIEW, Kind::View, &[], absent("view")).unwrap();
    Fixture { layer, shell: Shell::new(Secret::fixed(SECRET), "shell"), frames: HeadlessFrames::new() }
}

impl Fixture {
    fn declare(&mut self, row: &str, metadata: Metadata) {
        self.layer.declare_overlay(row, metadata).unwrap();
    }

    /// Serves `structure.layer` and returns the `FilterRef` of a predicate its job prepared, which wrote no record.
    fn predicate(&mut self) -> Value {
        let tool = Overlay { module: "tools/full".into(), entry: "layer_predicate".into() };
        self.layer.serve_row(LAYER, tool).unwrap();
        let args = json::object([("cell", Value::Int(1)), ("layer", Value::Int(2))]);
        let receipt = self.shell.invoke(&mut self.layer, LAYER, args).unwrap();
        let Status::Running(job) = receipt.status else { panic!("{:?}", receipt.status) };
        assert_eq!(self.layer.take_program_job().map(|taken| taken.job), Some(job));
        let done = self.layer.finish_program_job(job, Ok(Value::Null)).unwrap();
        assert!(matches!(done.status, Status::Completed(_)), "{:?}", done.status);
        json::object([("digest", text(&digest(&Value::Null))), ("job", Value::Int(job as i64))])
    }

    /// Starts a preview of letter 1 and leaves its job on the exclusive lane; returns the preview's ID.
    fn building(&mut self) -> u64 {
        let receipt = self.shell.invoke(&mut self.layer, PREVIEW, draft_args(&word(&[1]))).unwrap();
        let Status::Running(id) = receipt.status else { panic!("preview not admitted: {:?}", receipt.status) };
        id
    }

    /// Builds and presents a preview of letter 1; returns its ID.
    fn presented(&mut self) -> u64 {
        let id = self.building();
        assert_eq!(self.layer.run_jobs(), 1);
        self.frames.present(&mut self.shell, &mut self.layer, id).unwrap();
        id
    }

    /// The commit chord through the command route, bound to presented preview `id`.
    fn execute(&mut self, id: u64) -> Receipt {
        let press = self.shell.prepare(&self.layer, EXECUTE, preview_args(id)).unwrap();
        self.shell.send(&mut self.layer, press)
    }

    /// The solver invokes `user.read`, whose program job then waits on the program lane; returns the job.
    fn invoke_read(&mut self) -> u64 {
        let receipt = self.shell.invoke(&mut self.layer, READ, json::object([])).unwrap();
        let Status::Running(job) = receipt.status else { panic!("not admitted: {:?}", receipt.status) };
        job
    }

    /// What availability may not change: the revision, the generation, the journal and the jobs.
    fn quiet(&self) -> (Value, u64, usize, Option<u64>) {
        let layer = &self.layer;
        (layer.revision().to_json(), layer.generation(), layer.store().log.len(), layer.preview().map(|p| p.id))
    }
}

/// An overlay row answers from its stored metadata. Before any declaration the metadata is the default, a passing
/// static verdict and no kernel row; a declaration replaces it. Asking changes nothing: no program starts, no job is
/// admitted and no lane is taken (LI.20), and the generation stays (LI.21).
#[test]
fn an_overlay_row_answers_from_its_stored_metadata_alone() {
    let mut f = fixture();
    assert_eq!(f.layer.table().metadata(READ), Some(&Metadata::default()));
    assert!(f.layer.table().language_defined(READ));
    for stored in [COMPARE, EXECUTE] {
        assert!(!f.layer.table().language_defined(stored), "{stored}");
        assert_eq!(f.layer.table().metadata(stored), None, "{stored}");
    }
    for row in [READ, WRITE, VIEW] {
        assert_eq!(f.layer.availability(row), Ok(()), "{row}");
    }

    let metadata = requiring(vec![
        required(COMPARE, Binding::Static(cells(1, 2))),
        required(PREVIEW, Binding::Computed),
        optional(EXECUTE, Binding::Computed),
    ]);
    f.declare(READ, metadata.clone());
    assert_eq!(f.layer.table().metadata(READ), Some(&metadata));
    let before = f.quiet();
    for _ in 0..3 {
        assert_eq!(f.layer.availability(READ), Ok(()));
        assert_eq!(f.layer.availability_for(READ, &json::object([])), Ok(()));
    }
    assert_eq!(f.quiet(), before);
    assert_eq!(f.layer.run_jobs(), 0);
    assert!(f.layer.take_program_job().is_none());

    // No hint held a lane: the program lane, at its capacity of one, admits `user.read`'s job, which then fills it.
    f.invoke_read();
    assert_eq!(code_of(f.layer.availability(WRITE)), code::LANE_FULL);
}

/// LI.20's order: the static check's first failed diagnostic, then a missing grant, then the row's own gates, then
/// the first failing required row in sorted row ID order, its ties in stored order.
#[test]
fn the_first_failure_is_reported_in_li20_order() {
    let mut f = fixture();
    let diagnostic = Error::new("LG2:type.mismatch", "the entry returns Text where its row declares Nat");
    let metadata = |diagnostic: Option<Error>, ungranted: bool| Metadata {
        diagnostic,
        requires: [ungranted.then(|| required(EXECUTE, Binding::Computed)), Some(required(MISSING, Binding::Computed))]
            .into_iter()
            .flatten()
            .collect(),
    };

    // The program lane holds `user.read`'s own job, so the row's own lane gate fails; the view overlay's lane is free.
    let job = f.invoke_read();
    assert_eq!(code_of(f.layer.availability(READ)), code::LANE_FULL);
    assert_eq!(f.layer.availability(VIEW), Ok(()));

    f.declare(READ, metadata(Some(diagnostic.clone()), true));
    assert_eq!(f.layer.availability(READ), Err(diagnostic));
    f.declare(READ, metadata(None, true));
    assert_eq!(code_of(f.layer.availability(READ)), code::NOT_GRANTED);
    f.declare(READ, metadata(None, false));
    assert_eq!(code_of(f.layer.availability(READ)), code::LANE_FULL);
    let taken = f.layer.take_program_job().unwrap();
    assert_eq!(taken.job, job);
    f.layer.finish_program_job(job, Ok(Value::Null)).unwrap();
    assert_eq!(code_of(f.layer.availability(READ)), code::OVERLAY_DEPENDENCY_MISSING);
    f.declare(READ, Metadata::default());
    assert_eq!(f.layer.availability(READ), Ok(()));

    // A missing grant comes before a missing row that sorts first.
    f.declare(READ, requiring(vec![required("a.missing", Binding::Computed), required(EXECUTE, Binding::Computed)]));
    assert_eq!(code_of(f.layer.availability(READ)), code::NOT_GRANTED);

    // Required rows in sorted order: `operation.preview` before `structure.missing`, whatever the stored order.
    f.declare(READ, requiring(vec![required(MISSING, Binding::Computed), required(PREVIEW, Binding::Computed)]));
    let preview = f.building();
    assert_eq!(code_of(f.layer.availability(READ)), code::LANE_FULL);
    assert_eq!(f.layer.run_jobs(), 1);
    assert!(f.layer.preview_state(preview).is_some());
    assert_eq!(code_of(f.layer.availability(READ)), code::OVERLAY_DEPENDENCY_MISSING);

    // Ties in stored order: two static bindings of one row, each refused with its own detail.
    let (malformed, misnamed) = (cells(1, 600), json::object([("cell", Value::Int(1))]));
    let first = f.layer.availability_for(COMPARE, &malformed).unwrap_err();
    let second = f.layer.availability_for(COMPARE, &misnamed).unwrap_err();
    assert_eq!((first.code, second.code), (code::ARGUMENT_FORM, code::ARGUMENT_FORM));
    assert_ne!(first.detail, second.detail);
    for (order, expected) in [([&malformed, &misnamed], &first), ([&misnamed, &malformed], &second)] {
        let bound = order.iter().map(|args| required(COMPARE, Binding::Static((*args).clone()))).collect();
        f.declare(READ, requiring(bound));
        assert_eq!(f.layer.availability(READ).as_ref(), Err(expected));
    }
}

/// LI.20's vector: with every other gate passing, `(K, A)` available and `(K, B)` refused give an available overlay
/// for static A and that refusal for static B; a computed binding of K is available on K's argument-independent gates,
/// and computing B later still gets the refusal at invocation. A static binding that names a revision is stale once the
/// revision moves, and one whose `FilterRef` names no prepared predicate answers `KN2:input.named_object_invalid`, as
/// its invocation would.
#[test]
fn a_static_binding_answers_as_its_arguments_do() {
    let mut f = fixture();
    let (a, b) = (cells(1, 2), cells(1, 600));
    assert_eq!(f.layer.availability_for(COMPARE, &a), Ok(()));
    let refusal = f.layer.availability_for(COMPARE, &b).unwrap_err();
    assert_eq!(refusal.code, code::ARGUMENT_FORM);

    f.declare(READ, requiring(vec![required(COMPARE, Binding::Static(a))]));
    assert_eq!(f.layer.availability(READ), Ok(()));
    f.declare(READ, requiring(vec![required(COMPARE, Binding::Static(b.clone()))]));
    assert_eq!(f.layer.availability(READ), Err(refusal));
    f.declare(READ, requiring(vec![required(COMPARE, Binding::Computed)]));
    assert_eq!(f.layer.availability(READ), Ok(()));
    let runtime = Caller::Runtime { run: "run-1".into() };
    let epoch = f.layer.revision().epoch;
    let invocation = Invocation {
        command: "run-1/1".into(),
        row: COMPARE.into(),
        args: b,
        token: None,
        epoch,
        budget: Some(c600_command::Budget::DRAFT_CEILING),
    };
    assert_eq!(f.layer.invoke(&runtime, invocation).status.code(), Some(code::ARGUMENT_FORM));
    f.declare(READ, requiring(vec![required(COMPARE, Binding::Static(Value::Int(3)))]));
    assert_eq!(code_of(f.layer.availability(READ)), code::ARGUMENT_FORM);

    let predicate = f.predicate();
    let unknown = json::object([("digest", text("00")), ("job", Value::Int(999))]);
    let unknown = json::object([("predicate", unknown), ("revision", f.layer.revision().to_json())]);
    assert_eq!(code_of(f.layer.availability_for(FILTER, &unknown)), code::NAMED_OBJECT_INVALID);
    let bound = json::object([("predicate", predicate), ("revision", f.layer.revision().to_json())]);
    let both = vec![required(FILTER, Binding::Static(bound.clone())), required(FILTER, Binding::Computed)];
    f.declare(READ, requiring(both));
    assert_eq!(f.layer.availability(READ), Ok(()));
    let id = f.presented();
    assert!(matches!(f.execute(id).status, Status::Committed(_)));
    assert_eq!(code_of(f.layer.availability_for(FILTER, &bound)), code::REVISION_CHANGED);
    assert_eq!(code_of(f.layer.availability(READ)), code::REVISION_CHANGED);
    f.declare(READ, requiring(vec![required(FILTER, Binding::Computed)]));
    assert_eq!(f.layer.availability(READ), Ok(()));
}

/// Optional rows take no part: an ungranted, missing, busy or refused optional row leaves the overlay available.
#[test]
fn optional_rows_take_no_part() {
    let mut f = fixture();
    f.building();
    assert_eq!(code_of(f.layer.availability(PREVIEW)), code::LANE_FULL);
    let optional_rows = vec![
        optional(EXECUTE, Binding::Computed),
        optional(MISSING, Binding::Computed),
        optional(PREVIEW, Binding::Computed),
        optional(COMPARE, Binding::Static(cells(1, 600))),
        optional(VIEW, Binding::Computed),
    ];
    f.declare(READ, requiring(optional_rows));
    assert_eq!(f.layer.availability(READ), Ok(()));
    f.declare(READ, requiring(vec![required(PREVIEW, Binding::Computed)]));
    assert_eq!(code_of(f.layer.availability(READ)), code::LANE_FULL);
}

/// A required row that the runtime may not invoke is a missing grant; a required row that is not a kernel row, absent
/// or language-defined, is missing. A commit or app overlay is never registered, so no declaration or availability can
/// admit it later; metadata belongs to language-defined rows only. Refusals change nothing.
#[test]
fn grants_and_missing_rows() {
    let mut f = fixture();
    for ungranted in [EXECUTE, ORBITS, "session.undo", "jumble.on"] {
        f.declare(READ, requiring(vec![required(ungranted, Binding::Computed)]));
        let refusal = f.layer.availability(READ).unwrap_err();
        assert_eq!(refusal.code, code::NOT_GRANTED, "{ungranted}");
        assert!(refusal.detail.contains(ungranted), "{ungranted}: {}", refusal.detail);
    }
    for missing in [MISSING, VIEW, WRITE] {
        f.declare(READ, requiring(vec![required(missing, Binding::Computed)]));
        let refusal = f.layer.availability(READ).unwrap_err();
        assert_eq!(refusal.code, code::OVERLAY_DEPENDENCY_MISSING, "{missing}");
        assert!(refusal.detail.contains(missing), "{missing}: {}", refusal.detail);
    }

    let table = f.layer.table().rows().cloned().collect::<Vec<_>>();
    let generation = f.layer.generation();
    for kind in [Kind::Commit, Kind::App] {
        let refused = f.layer.register_overlay("user.commit", kind, &[], absent("commit")).unwrap_err();
        assert_eq!(refused.code, code::OVERLAY_ROW_KIND, "{kind:?}");
    }
    assert_eq!(code_of(f.layer.availability("user.commit")), code::UNKNOWN_ROW);
    let declared = f.layer.declare_overlay("user.commit", Metadata::default()).unwrap_err();
    assert_eq!(declared.code, code::UNKNOWN_ROW);
    for stored in [COMPARE, EXECUTE] {
        let refused = f.layer.declare_overlay(stored, Metadata::default()).unwrap_err();
        assert_eq!(refused.code, code::OVERLAY_ROW_CONFLICT, "{stored}");
        assert_eq!(f.layer.table().metadata(stored), None, "{stored}");
    }
    assert_eq!(f.layer.table().rows().cloned().collect::<Vec<_>>(), table);
    assert_eq!(f.layer.generation(), generation);
}

/// Read-only recovery is an own-row gate: it holds back an overlay that changes the session, and a required row that
/// does, after the diagnostic and the grants; a query or view overlay stays available.
#[test]
fn read_only_recovery_holds_back_an_overlay_that_changes_the_session() {
    let mut f = fixture();
    let id = f.presented();
    f.layer.store_mut().inject.push_back(Fate::Unknown);
    assert_eq!(f.execute(id).status, Status::UnknownOutcome);
    assert_eq!(f.layer.recovery().unwrap().code, code::COMMIT_FATE);

    assert_eq!(code_of(f.layer.availability(WRITE)), code::READ_ONLY_RECOVERY);
    assert_eq!((f.layer.availability(READ), f.layer.availability(VIEW)), (Ok(()), Ok(())));
    let diagnostic = Error::new("LG2:type.mismatch", "a declared type does not hold");
    f.declare(WRITE, Metadata { diagnostic: Some(diagnostic.clone()), requires: Vec::new() });
    assert_eq!(f.layer.availability(WRITE), Err(diagnostic));
    f.declare(WRITE, requiring(vec![required(ORBITS, Binding::Computed)]));
    assert_eq!(code_of(f.layer.availability(WRITE)), code::NOT_GRANTED);

    f.declare(READ, requiring(vec![required(PREVIEW, Binding::Computed)]));
    assert_eq!(code_of(f.layer.availability(READ)), code::READ_ONLY_RECOVERY);
    f.declare(READ, requiring(vec![optional(PREVIEW, Binding::Computed)]));
    assert_eq!(f.layer.availability(READ), Ok(()));
}

/// The generation advances when an input of availability changes: a registration, a served row, a declaration
/// (LI.21). Asking for availability, and a refused change, leave it.
#[test]
fn the_generation_advances_when_the_overlay_table_changes() {
    let mut f = fixture();
    let start = f.layer.generation();
    let _ = (f.layer.availability(READ), f.layer.availability_for(COMPARE, &cells(1, 2)));
    assert_eq!(f.layer.generation(), start);
    f.declare(READ, requiring(vec![required(COMPARE, Binding::Computed)]));
    assert_eq!(f.layer.generation(), start + 1);
    assert!(f.layer.declare_overlay(COMPARE, Metadata::default()).is_err());
    f.layer.register_overlay("user.more", Kind::Query, &[], absent("more")).unwrap();
    assert_eq!(f.layer.generation(), start + 2);
    assert!(f.layer.register_overlay("user.more", Kind::Query, &[], absent("more")).is_err());
    f.layer.serve_row("structure.adjacency", Overlay { module: "tools/full".into(), entry: "near".into() }).unwrap();
    assert_eq!(f.layer.generation(), start + 3);
    assert!(f.layer.serve_row(COMPARE, absent("again")).is_err());
    assert_eq!(f.layer.generation(), start + 3);
}

/// The entries of a manifest's dependency and build-dependency tables, its target tables among them: each name, and
/// the directory of a crate of this workspace that it names by its relative path, or `None` for a crate from the
/// registry, which must pin a version. A dependency's own table section, such as `[dependencies.name]`, fails the
/// test, since this reader does not follow one.
fn dependencies(manifest: &str) -> Vec<(String, Option<String>)> {
    let mut entries = Vec::new();
    let mut inside = false;
    for line in manifest.lines().map(str::trim) {
        if line.starts_with('[') {
            assert!(!line.contains("dependencies."), "a dependency's own table section, not followed here: {line}");
            let target = line.starts_with("[target.")
                && (line.ends_with(".dependencies]") || line.ends_with(".build-dependencies]"));
            inside = line == "[dependencies]" || line == "[build-dependencies]" || target;
            continue;
        }
        if !inside || line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (name, value) = line.split_once('=').expect("a dependency line");
        let directory = value.split_once("path = \"../").and_then(|(_, rest)| rest.split_once('"'));
        let directory = directory.map(|(directory, _)| directory.to_owned());
        assert!(directory.is_some() || value.contains("version = \""), "neither a path nor a version: {line}");
        entries.push((name.trim().to_owned(), directory));
    }
    entries
}

/// The command layer's crate depends on the kernel's crates only, read from the manifests: no language crate (the
/// checker, the IR, the modules, the runtime, the tools, the library) is in its closure, so the layer cannot run
/// language code to answer availability. The other crates in the closure come from the registry.
#[test]
fn the_command_layer_depends_on_no_language_crate() {
    let rust = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut closure = BTreeSet::new();
    let mut pending = vec!["c600-command".to_owned()];
    while let Some(name) = pending.pop() {
        let manifest = std::fs::read_to_string(rust.join(&name).join("Cargo.toml")).unwrap();
        for (dependency, directory) in dependencies(&manifest) {
            let Some(directory) = directory else {
                assert!(!dependency.starts_with("c600"), "{name} takes {dependency} from the registry");
                continue;
            };
            assert_eq!(directory, dependency, "{name}'s manifest");
            if closure.insert(dependency.clone()) {
                pending.push(dependency);
            }
        }
    }
    let kernel: BTreeSet<String> = ["c600-digest", "c600-jumble", "c600-kernel"].map(String::from).into();
    let why = concat!(
        "the command layer's path dependencies changed: ",
        "a new kernel crate belongs in this list, a language crate does not",
    );
    assert_eq!(closure, kernel, "{why}");
}

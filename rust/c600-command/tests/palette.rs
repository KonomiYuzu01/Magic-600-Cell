//! CT-EV4-01 (source, whole; EV4.1, EV4.2; LP4-4): overlay commands appear in the palette, forbidden kinds are
//! refused, and availability uses stored tables without running programs.
//!
//! The parts, one test each:
//! - a checked user command, registered as a view, workspace or query row or as a file row with a granted file
//!   capability (KN5.3), appears in the palette beside every stored row, with its label, kind and key sequences, and a
//!   search finds it by label, row ID or key name;
//! - a commit or app row, a file row without the file grant, a row that takes a token, and an ID a stored row, a
//!   service or another overlay holds are refused with their typed codes, change nothing and never appear in the
//!   palette;
//! - the palette's answers are KN5.6's availability table, read once per revision and generation from stored rows and
//!   metadata: no program starts, no job is queued, and a search reads the kept palette alone; an unavailable entry
//!   keeps its place with its typed reason;
//! - the checked command is invoked by key and from the palette through the same route and gates, and a command whose
//!   stored static verdict failed is refused by both with the reason the palette shows;
//! - `command.find` opens the palette: it is admitted and changes nothing;
//! - the shell's binding of a key refuses a sequence bound in an overlapping context.
//!
//! The overlay rows name entries of a module that does not exist: the layer never looks a module up, and its crate
//! reaches no language crate (the last test), so no answer here can have come from running language code (ONE.48).
//! Evidence kind: source; nothing here is Windows input or timing.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use c600_command::keys::{Arg, RoutingClass};
use c600_command::Dependency;
use c600_command::{
    code, draft_args, Binding, BindingSpec, Caller, CommandLayer, Error, Grant, Invocation, Kind, Lane, MemoryStore,
    Metadata, Overlay, Palette, Registration, Secret, Shell, Status, Table, FIND_ROW,
};
use c600_kernel::json::{self, Value};
use c600_kernel::Kernel;

const SECRET: [u8; 32] = [7; 32];

const INSPECT: &str = "user.inspect";
const NOTE: &str = "user.note";
const VIEW: &str = "user.view";
const EXPORT: &str = "user.export";
/// The user rows in the table's order, by row ID.
const USER_ROWS: [&str; 4] = [EXPORT, INSPECT, NOTE, VIEW];

/// A static check's diagnostic: a registered code of chapter 13.
const TYPE_MISMATCH: &str = "LG2:type.mismatch";

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

/// An entry of a module that does not exist. The layer never looks a module up.
fn absent(entry: &str) -> Overlay {
    Overlay { module: "absent/module".into(), entry: entry.into() }
}

fn empty() -> Value {
    Value::Object(Default::default())
}

fn text(s: &str) -> Value {
    Value::String(s.to_owned())
}

fn rows(layer: &CommandLayer) -> Vec<c600_command::Row> {
    layer.table().rows().cloned().collect()
}

struct Fixture {
    layer: CommandLayer,
    shell: Shell,
}

/// A session at the solved state on H-08's table, with four checked user commands: a query, a workspace and a view row,
/// and a file row whose module holds the granted file capability. Each declares a passing static verdict and no kernel
/// row, as a module that checked clean.
fn fixture() -> Fixture {
    let k = kernel();
    let (start, protected) = (k.solved(), k.protected_set(&[]).unwrap());
    let mut layer =
        CommandLayer::new(k, Table::h08(), Secret::fixed(SECRET), MemoryStore::default(), "s1", start, protected);
    let user = [
        (INSPECT, Kind::Query, Registration::labelled("Pairing check")),
        (NOTE, Kind::Workspace, Registration::labelled("Pairing note")),
        (VIEW, Kind::View, Registration::labelled("Pairing view")),
        (EXPORT, Kind::File, Registration::labelled("Pairing export").granting(Grant::File)),
    ];
    for (id, kind, registration) in user {
        layer.register_overlay_with(id, kind, &[], absent(id), registration).unwrap();
        layer.declare_overlay(id, Metadata::default()).unwrap();
    }
    Fixture { layer, shell: Shell::new(Secret::fixed(SECRET), "shell") }
}

/// A global-class binding of `sequence` to `row` with no arguments.
fn global(sequence: &[&str], row: &str) -> BindingSpec {
    BindingSpec::new(sequence, RoutingClass::Global, row, vec![])
}

fn ids<'a>(entries: impl IntoIterator<Item = &'a c600_command::PaletteEntry>) -> Vec<&'a str> {
    entries.into_iter().map(|entry| entry.row.as_str()).collect()
}

/// CT-EV4-01, part 1 (EV4.1, EV4.2; KN5.3): the palette includes every command, H-08's stored rows and the checked user
/// commands, in the table's order. A user command shows its label, its kind and that a language module defined it, and
/// is available; a stored row shows H-08's label. A search finds a user command by a word of its label, by its row ID
/// and by the key sequence bound to it, ignoring case, and an empty query matches every entry.
#[test]
fn overlay_commands_appear_in_the_palette() {
    let mut f = fixture();
    let table: Vec<String> = f.layer.table().rows().map(|row| row.id.clone()).collect();
    let palette = f.shell.read_palette(&f.layer).clone();
    assert_eq!(ids(palette.entries()), table);
    // H-08's 209 rows: 198 at the LP3 freeze and LP5's eleven research panel rows, its fifth delta
    // (`docs/progress/1.0/command-rows-research.json`; CM.19).
    assert_eq!(palette.entries().len(), 209 + USER_ROWS.len(), "H-08's 209 rows and the four user commands");

    for (id, kind, label) in [
        (INSPECT, Kind::Query, "Pairing check"),
        (NOTE, Kind::Workspace, "Pairing note"),
        (VIEW, Kind::View, "Pairing view"),
        (EXPORT, Kind::File, "Pairing export"),
    ] {
        let entry = palette.entry(id).unwrap();
        assert_eq!((entry.label.as_str(), entry.kind, entry.language_defined), (label, kind, true), "{id}");
        assert_eq!(entry.availability, Ok(()), "{id}");
        assert!(entry.keys.is_empty(), "{id}");
    }
    let stored = palette.entry(FIND_ROW).unwrap();
    assert_eq!((stored.label.as_str(), stored.kind, stored.language_defined), ("Find command", Kind::View, false));
    let undo = palette.entry("session.undo").unwrap();
    assert_eq!((undo.label.as_str(), undo.keys.as_slice()), ("Undo", ["Ctrl+KeyZ".to_owned()].as_slice()));
    assert!(palette
        .entries()
        .iter()
        .filter(|entry| entry.language_defined)
        .map(|entry| entry.row.as_str())
        .eq(USER_ROWS));

    assert_eq!(ids(palette.find("pairing")), USER_ROWS);
    assert_eq!(ids(palette.find("PAIRING View")), [VIEW]);
    assert_eq!(ids(palette.find("user.export")), [EXPORT]);
    assert_eq!(ids(palette.find("ctrl+keyz")), ["session.undo"]);
    assert_eq!(palette.find("").len(), palette.entries().len());
    assert!(palette.find("pairing nowhere").is_empty());

    // A key bound to the user command shows in its entry and finds it.
    f.shell.bind(global(&["Ctrl+Alt+KeyP"], INSPECT)).unwrap();
    let palette = f.shell.read_palette(&f.layer);
    assert_eq!(palette.entry(INSPECT).unwrap().keys, ["Ctrl+Alt+KeyP"]);
    assert_eq!(ids(palette.find("ctrl+alt+keyp")), [INSPECT]);
}

/// CT-EV4-01, part 2 (EV4.1; KN5.3, KN5.15): a language-defined command registers only as a view, workspace or query
/// row, or as a file row whose module holds the granted file capability. A commit or app row, a file row without the
/// file grant (none, or other grants only) and a row that takes a token are refused with `KN5:input.overlay_row_kind`;
/// an ID that a stored puzzle row, the palette's own row, a service or another overlay holds is refused with
/// `KN5:input.overlay_row_conflict`. A refusal changes neither the table nor the generation, so the palette stays
/// current, and a refused command never appears in it. The registered rows have their kinds, the view row the view
/// lane and the others the program lane, and none takes a token.
#[test]
fn forbidden_kinds_are_refused_and_never_listed() {
    let mut f = fixture();
    let palette = f.shell.read_palette(&f.layer).clone();
    let before = rows(&f.layer);
    let generation = f.layer.generation();
    let other_grants = Registration::default().granting(Grant::Clock).granting(Grant::Randomness);
    let token: &[(&str, &str)] = &[("token", "UserActionToken")];
    let cases: [(&str, Kind, &[(&str, &str)], Registration, &str); 10] = [
        ("user.commit", Kind::Commit, &[], Registration::default(), code::OVERLAY_ROW_KIND),
        ("user.app", Kind::App, &[], Registration::default(), code::OVERLAY_ROW_KIND),
        ("user.file", Kind::File, &[], Registration::default(), code::OVERLAY_ROW_KIND),
        ("user.file", Kind::File, &[], other_grants, code::OVERLAY_ROW_KIND),
        ("user.press", Kind::Query, token, Registration::default(), code::OVERLAY_ROW_KIND),
        ("operation.execute", Kind::Query, &[], Registration::default(), code::OVERLAY_ROW_CONFLICT),
        ("twist.apply", Kind::View, &[], Registration::default(), code::OVERLAY_ROW_CONFLICT),
        (FIND_ROW, Kind::View, &[], Registration::default(), code::OVERLAY_ROW_CONFLICT),
        ("kernel.outcome", Kind::Query, &[], Registration::default(), code::OVERLAY_ROW_CONFLICT),
        (INSPECT, Kind::Query, &[], Registration::labelled("Again"), code::OVERLAY_ROW_CONFLICT),
    ];
    for (id, kind, arguments, registration, expected) in cases {
        let refused = f.layer.register_overlay_with(id, kind, arguments, absent(id), registration).unwrap_err();
        assert_eq!(refused.code, expected, "{id} {kind:?}");
        assert_eq!(rows(&f.layer), before, "{id}");
        assert_eq!(f.layer.generation(), generation, "{id}");
    }
    // The plain registration grants nothing, so a file row is refused there too.
    let refused = f.layer.register_overlay("user.file", Kind::File, &[], absent("file")).unwrap_err();
    assert_eq!(refused.code, code::OVERLAY_ROW_KIND);
    assert!(palette.current(&f.layer));
    assert_eq!(f.shell.read_palette(&f.layer), &palette);
    for id in ["user.commit", "user.app", "user.file", "user.press"] {
        assert!(palette.entry(id).is_none() && palette.find(id).is_empty(), "{id}");
    }
    assert_eq!(palette.entry("operation.execute").unwrap().kind, Kind::Commit);

    for (id, kind, lane) in
        [(INSPECT, Kind::Query, Lane::Program), (NOTE, Kind::Workspace, Lane::Program), (VIEW, Kind::View, Lane::View)]
    {
        let row = f.layer.table().row(id).unwrap();
        assert_eq!((row.kind, row.lane, row.takes_token(), row.grants.is_empty()), (kind, lane, false, true), "{id}");
    }
    let export = f.layer.table().row(EXPORT).unwrap();
    assert_eq!((export.kind, export.lane), (Kind::File, Lane::Program));
    assert_eq!(export.grants, BTreeSet::from([Grant::File]));
}

/// CT-EV4-01, part 3 (EV4.2; KN5.5 to KN5.7, KN5.25): the palette's answers are KN5.6's availability table for every
/// row, computed from the stored rows, the overlays' stored metadata and the layer's state. Reading it starts no
/// program, queues no job and changes neither the revision nor the generation, and the shell reads again only at a new
/// generation. A search reads the kept palette alone: a kept palette answers as of its own generation after the layer
/// moves on. An unavailable entry keeps its place, in the entries and in the matches, with its typed reason: a failed
/// static verdict, a required row the runtime may not invoke, a required row whose lane is full, and a stored row this
/// build does not serve.
#[test]
fn availability_comes_from_the_stored_table_and_runs_no_program() {
    let mut f = fixture();
    let (revision, generation) = (f.layer.revision().clone(), f.layer.generation());
    let first = f.shell.read_palette(&f.layer).clone();
    assert_eq!((&first.revision, first.generation), (&revision, generation));
    assert_eq!((f.layer.revision(), f.layer.generation()), (&revision, generation));
    assert!(f.layer.take_program_job().is_none());
    assert_eq!(f.layer.run_jobs(), 0);

    // The answers are the table's, row by row.
    let computed = Binding::Computed;
    let table = f.layer.availability_table(f.layer.table().rows().map(|row| (row.id.as_str(), &computed)));
    assert_eq!(
        (&table.revision, table.generation, &table.context),
        (&first.revision, first.generation, &first.context)
    );
    for entry in first.entries() {
        assert_eq!(table.answer(&entry.row, &computed), Some(entry.availability), "{}", entry.row);
    }
    assert_eq!(first.entry("palette.pin").unwrap().availability, Err(code::ROW_NOT_BUILT));
    assert_eq!(first.entry(FIND_ROW).unwrap().availability, Ok(()));

    // No new generation, no new read.
    assert_eq!(f.shell.read_palette(&f.layer), &first);

    // The overlays' stored metadata changes: a failed static verdict, a required row the runtime may not invoke, and a
    // required row on the exclusive lane, which a preview then fills.
    let failed =
        Metadata { diagnostic: Some(Error::new(TYPE_MISMATCH, "a declared type does not hold")), ..Default::default() };
    f.layer.declare_overlay(NOTE, failed).unwrap();
    let execute = Dependency { row: "operation.execute".into(), binding: Binding::Computed, optional: false };
    f.layer.declare_overlay(INSPECT, Metadata { requires: vec![execute], ..Default::default() }).unwrap();
    let preview = Dependency { row: "operation.preview".into(), binding: Binding::Computed, optional: false };
    f.layer.declare_overlay(VIEW, Metadata { requires: vec![preview], ..Default::default() }).unwrap();
    let receipt = f
        .shell
        .invoke(
            &mut f.layer,
            "operation.preview",
            draft_args(&json::parse(br#"[{"kind": "word", "moves": [1]}]"#).unwrap()),
        )
        .unwrap();
    assert!(matches!(receipt.status, Status::Running(_)), "{:?}", receipt.status);

    // The kept palette answers as of its generation, with no engine call, and is no longer current.
    let kept = f.shell.palette().unwrap().clone();
    assert_eq!(kept, first);
    assert!(!kept.current(&f.layer));
    for query in ["p", "pa", "pai", "pair", "pairing"] {
        assert!(kept.find(query).iter().all(|entry| entry.availability.is_ok() || !entry.language_defined), "{query}");
    }

    let second = f.shell.read_palette(&f.layer).clone();
    assert!(second.generation > first.generation);
    assert_eq!(ids(second.entries()), ids(first.entries()), "every entry keeps its place");
    assert_eq!(ids(second.find("pairing")), USER_ROWS, "unavailable matches keep their places");
    assert_eq!(second.entry(NOTE).unwrap().availability, Err(TYPE_MISMATCH));
    assert_eq!(second.entry(INSPECT).unwrap().availability, Err(code::NOT_GRANTED));
    assert_eq!(second.entry(VIEW).unwrap().availability, Err(code::LANE_FULL));
    assert_eq!(second.entry(EXPORT).unwrap().availability, Ok(()));
    let table = f.layer.availability_table(f.layer.table().rows().map(|row| (row.id.as_str(), &computed)));
    for entry in second.entries() {
        assert_eq!(table.answer(&entry.row, &computed), Some(entry.availability), "{}", entry.row);
    }
    // No program ran for any answer: the only job is the preview's, on the exclusive lane.
    assert!(f.layer.take_program_job().is_none());
}

/// CT-EV4-01, part 4 (EV4.2; KN5.1, KN5.4): a checked user command is invoked by key and from the palette through the
/// same route, so the same gates apply: each press admits a program job of the command, on its row's lane, for the
/// shell. A view, workspace and file command are invoked from the palette alike (ADR-D01.2: an invoked entry point may
/// be a view or workspace row). Once the command's stored static verdict fails, the palette shows it unavailable with
/// that diagnostic, and a press by key or from the palette is refused with the same code and starts nothing.
#[test]
fn a_checked_command_is_invoked_by_key_or_by_palette() {
    let mut f = fixture();
    f.layer.set_program_capacity(8);
    f.shell.bind(global(&["Ctrl+Alt+KeyP"], INSPECT)).unwrap();

    let by_key = f.shell.press(&mut f.layer, "Ctrl+Alt+KeyP", false).unwrap().unwrap();
    let Status::Running(key_job) = by_key.status else { panic!("{:?}", by_key.status) };
    let found = f.shell.read_palette(&f.layer).find("pairing check");
    let chosen = ids(found)[0].to_owned();
    assert_eq!(chosen, INSPECT);
    let by_palette = f.shell.invoke(&mut f.layer, &chosen, empty()).unwrap();
    let Status::Running(palette_job) = by_palette.status else { panic!("{:?}", by_palette.status) };
    assert_ne!(by_key.command, by_palette.command);
    for expected in [key_job, palette_job] {
        let job = f.layer.take_program_job().unwrap();
        assert_eq!((job.job, job.row.as_str(), &job.caller), (expected, INSPECT, &Caller::Shell));
        assert_eq!((job.overlay.module.as_str(), job.overlay.entry.as_str()), ("absent/module", INSPECT));
    }

    for (id, kind) in [(VIEW, Kind::View), (NOTE, Kind::Workspace), (EXPORT, Kind::File)] {
        let receipt = f.shell.invoke(&mut f.layer, id, empty()).unwrap();
        assert!(matches!(receipt.status, Status::Running(_)), "{id}: {:?}", receipt.status);
        let job = f.layer.take_program_job().unwrap();
        assert_eq!((job.row.as_str(), job.kind), (id, kind));
    }

    let failed =
        Metadata { diagnostic: Some(Error::new(TYPE_MISMATCH, "a declared type does not hold")), ..Default::default() };
    f.layer.declare_overlay(INSPECT, failed).unwrap();
    assert_eq!(f.shell.read_palette(&f.layer).entry(INSPECT).unwrap().availability, Err(TYPE_MISMATCH));
    let by_key = f.shell.press(&mut f.layer, "Ctrl+Alt+KeyP", false).unwrap().unwrap();
    let by_palette = f.shell.invoke(&mut f.layer, INSPECT, empty()).unwrap();
    for receipt in [by_key, by_palette] {
        assert_eq!((receipt.job, receipt.status.code()), (None, Some(TYPE_MISMATCH)), "{:?}", receipt.status);
    }
    assert!(f.layer.take_program_job().is_none());
}

/// CT-EV4-01, part 5 (EV4.2; DT-M1 C1): `command.find` opens the palette. Bound to a key or invoked from the palette,
/// it is `accepted` with no job and changes neither the revision nor the generation; the palette then answers its query
/// from the kept read. Its `query` must be text and is required. A run invokes it only inside a command the solver
/// invoked (RPI.83). The layer builds the row itself, so no entry can serve it.
#[test]
fn command_find_opens_the_palette_and_changes_nothing() {
    let mut f = fixture();
    let query = || json::object([("query", text("pairing"))]);
    let find = BindingSpec::new(&["F1"], RoutingClass::Global, FIND_ROW, vec![("query", Arg::Fixed(text("pairing")))]);
    f.shell.bind(find).unwrap();
    let palette = f.shell.read_palette(&f.layer).clone();
    let entry = palette.entry(FIND_ROW).unwrap();
    assert_eq!((entry.availability, entry.keys.as_slice()), (Ok(()), ["F1".to_owned()].as_slice()));

    let (revision, generation) = (f.layer.revision().clone(), f.layer.generation());
    let by_key = f.shell.press(&mut f.layer, "F1", false).unwrap().unwrap();
    let by_palette = f.shell.invoke(&mut f.layer, FIND_ROW, query()).unwrap();
    for receipt in [by_key, by_palette] {
        assert_eq!((receipt.job, &receipt.status), (None, &Status::Accepted));
    }
    assert_eq!((f.layer.revision(), f.layer.generation()), (&revision, generation));
    assert!(f.layer.take_program_job().is_none());
    assert_eq!(ids(f.shell.read_palette(&f.layer).find("pairing")), USER_ROWS);

    for args in [json::object([("query", Value::Int(3))]), empty(), json::object([("object", Value::Int(3))])] {
        let receipt = f.shell.invoke(&mut f.layer, FIND_ROW, args.clone()).unwrap();
        assert_eq!((receipt.job, receipt.status.code()), (None, Some(code::ARGUMENT_FORM)), "{args:?}");
    }

    let epoch = f.layer.revision().epoch;
    let envelope = |command: &str| Invocation {
        command: command.to_owned(),
        row: FIND_ROW.to_owned(),
        args: query(),
        token: None,
        epoch,
        budget: None,
    };
    let outside = f.layer.invoke(&Caller::Runtime { run: "job-99".into() }, envelope("r1"));
    assert_eq!(outside.status.code(), Some(code::NOT_GRANTED));
    let receipt = f.shell.invoke(&mut f.layer, INSPECT, empty()).unwrap();
    assert!(matches!(receipt.status, Status::Running(_)), "{:?}", receipt.status);
    let run = f.layer.take_program_job().unwrap().run();
    let inside = f.layer.invoke(&Caller::Runtime { run }, envelope("r2"));
    assert_eq!(inside.status, Status::Accepted);

    let refused = f.layer.serve_row(FIND_ROW, absent("find")).unwrap_err();
    assert_eq!(refused.code, code::OVERLAY_ROW_CONFLICT);
}

/// CT-EV4-01, part 6 (EV4.3, EV4.7; LP4-8): the shell binds a key to a row only after EV4.7's checks of its whole key
/// map. A sequence the map already binds in the same routing class is refused with `EV4:input.binding_conflict` and
/// leaves the map and the palette's keys as they were; the same sequence in another routing class binds.
/// `tests/keymap.rs` runs CT-EV4-06's checks on the shell's map whole.
#[test]
fn a_binding_the_key_map_refuses_leaves_the_palette_as_it_was() {
    let mut f = fixture();
    let keymap = f.shell.keymap().clone();
    let refused = f.shell.bind(BindingSpec::new(&["Ctrl+'z'"], RoutingClass::Global, INSPECT, vec![])).unwrap_err();
    assert_eq!(refused.code, code::BINDING_CONFLICT);
    assert_eq!(f.shell.keymap(), &keymap);
    assert!(f.shell.read_palette(&f.layer).entry(INSPECT).unwrap().keys.is_empty());

    f.shell.bind(global(&["Ctrl+Alt+KeyP"], INSPECT)).unwrap();
    f.shell.bind(BindingSpec::new(&["Ctrl+Alt+KeyP"], RoutingClass::List, "list.move", vec![])).unwrap();
    let refused = f.shell.bind(global(&["Ctrl+Alt+KeyP"], NOTE)).unwrap_err();
    assert_eq!(refused.code, code::BINDING_CONFLICT);
    assert_eq!(f.shell.keymap().bindings().len(), keymap.bindings().len() + 2);
    let palette = f.shell.read_palette(&f.layer);
    assert_eq!((palette.entry(INSPECT).unwrap().keys.len(), palette.entry(NOTE).unwrap().keys.len()), (1, 0));
}

/// CT-EV4-01, part 7 (KN5.5, ONE.48): the command layer's crate names only the kernel's crates as its dependencies, so
/// neither availability nor the palette can run language code. `tests/availability.rs` follows the whole closure.
#[test]
fn the_palette_reaches_no_language_crate() {
    let manifest = std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml")).unwrap();
    let mut inside = false;
    let mut names = BTreeSet::new();
    for line in manifest.lines().map(str::trim) {
        if line.starts_with('[') {
            inside = line == "[dependencies]";
            continue;
        }
        if inside && !line.is_empty() && !line.starts_with('#') {
            names.insert(line.split_once('=').expect("a dependency line").0.trim().to_owned());
        }
    }
    assert_eq!(names, BTreeSet::from(["c600-digest", "c600-jumble", "c600-kernel"].map(String::from)));
    // A palette is plain data: it holds no reference to the layer.
    fn owned<T: 'static + Send + Sync>() {}
    owned::<Palette>();
}

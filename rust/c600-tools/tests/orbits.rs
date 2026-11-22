//! LB2's rows "Orbit catalogue and collateral stage order" and "Residuals and exact completion" on 600-cell-Full, as
//! library code: `c600-library`'s module `orbits` runs on `c600-run` through `Boundary` and the tool's host bridge onto
//! a command layer, in the solving profile, author `built_in`, within the budget ceiling
//! (`Budget::CEILING`). `orbit_catalogue` reads the `orbits` row at the orbits the caller names. A client
//! entry reads `positions` at the pieces the caller names and hands the answer to the pure `orbit_progress` or
//! `orbit_residuals`, so the row supplies membership only and the positions are a parameter (chapter 03). The
//! references are the kernel's own: `positions` at the solved state for the orbit table, its progress rows,
//! `positions` at the state the entry reads, and the buffers of the certified setup table,
//! `assets/execution_trees.json.gz`. A pass is source and fixture evidence.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use c600_check::{admit_built_in, check, Grant, Policy, Row, NOT_GRANTED};
use c600_command::{Budget, CommandLayer, Secret, Table};
use c600_digest::sha256;
use c600_kernel::{Derivation, Kernel, PiecePosition, ProtectedSet, Snapshot, ORBITS, PIECES};
use c600_library::{lower_sources, orbits_sha256, ORBITS_EXPORTS, ORBITS_MODULE, ORBITS_ROWS, ORBITS_SOURCE};
use c600_run::boundary::{Boundary, Context, Store};
use c600_run::interp::{load, Loaded, Options, Run};
use c600_run::meter::{Amounts, Outcome};
use c600_run::provenance::{Author, Producer, Profile};
use c600_run::value::Value;
use c600_tools::bridge::{base, Bridge};
use c600_tools::kernel_rows::{SetupTable, MAX_ORBIT_PIECES, ORBITS_ROW, POSITIONS};
use c600_tools::tool::{checker, contract, named_word_block, pins};

const CLIENT_MODULE: &str = "orbit_client";

/// A client of the module, as a host's entries would call it. `progress_at` reads `positions` at the pieces the caller
/// names; `residuals_at` reads the catalogue at the orbit the caller names and `positions` at the pieces the caller
/// names, so `complete` compares the named pieces with the catalogue's. Both read only at their parameters as given
/// (LB3.5). `chained` reads `positions` at the catalogue's answer, a prefetch that the built-in column refuses.
const CLIENT: &str = r#"edition "6cs-draft-1";
module orbit_client requires [];
import orbits "ORBITS_SHA";

export fn progress_at(orbit: Int, pieces: List<Nat>) -> record { orbit: Int, orientation_wrong: Nat, others: Nat, pieces: Nat, position_wrong: Nat, solved: Nat } effects [snapshot] {
    let positions = read_snapshot<List<record { home: List<Nat>, orbit: Int, piece: Nat, position: Int, slots: List<Nat> }>>("positions", pieces);
    return orbits.orbit_progress(orbit, positions);
}

export fn residuals_at(named: List<Int>, pieces: List<Nat>, buffer_a: Nat, buffer_b: Nat) -> record { buffer_a: record { occupant: Int, position: Nat, solved: Bool }, buffer_b: record { occupant: Int, position: Nat, solved: Bool }, complete: Bool, misoriented: List<Nat>, misplaced: List<Nat>, orbit: Int, solved: Bool, unknown: List<Nat> } effects [snapshot] {
    let members = read_snapshot<List<record { orbit: Int, pieces: List<Nat> }>>("orbits", named);
    let positions = read_snapshot<List<record { home: List<Nat>, orbit: Int, piece: Nat, position: Int, slots: List<Nat> }>>("positions", pieces);
    return orbits.orbit_residuals(index(members, 0), buffer_a, buffer_b, positions);
}

export fn chained(named: List<Int>) -> List<record { home: List<Nat>, orbit: Int, piece: Nat, position: Int, slots: List<Nat> }> effects [snapshot] {
    let members = read_snapshot<List<record { orbit: Int, pieces: List<Nat> }>>("orbits", named);
    let first = index(members, 0);
    return read_snapshot<List<record { home: List<Nat>, orbit: Int, piece: Nat, position: Int, slots: List<Nat> }>>("positions", first.pieces);
}
"#;

/// The budget ceiling of every profile (`Budget::CEILING`), as the run's limits.
const CEILING: Amounts = Amounts {
    steps: Budget::CEILING.steps,
    kernel_cost: Budget::CEILING.kernel_cost,
    memory: Budget::CEILING.memory,
    output: Budget::CEILING.output,
};

/// A fixed scramble from solved.
const SCRAMBLE: [i32; 20] =
    [17, -305, 911, 4, -1200, 600, 33, -88, 1021, -7, 450, 2, -999, 316, 75, -640, 1199, -12, 808, 23];

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

/// The client's source, importing the module by its digest.
fn client_source() -> &'static str {
    static SOURCE: OnceLock<String> = OnceLock::new();
    SOURCE.get_or_init(|| CLIENT.replace("ORBITS_SHA", &orbits_sha256()))
}

/// The module and the client, lowered and loaded together; the checker reports no diagnostic.
fn loaded() -> &'static Loaded {
    static LOADED: OnceLock<Loaded> = OnceLock::new();
    LOADED.get_or_init(|| {
        let program = lower_sources(&[ORBITS_SOURCE, client_source()]).expect("the module and the client lower");
        let report = check(&program).expect("well-formed IR");
        assert!(report.diagnostics.is_empty(), "{:#?}", report.diagnostics);
        load(&program).expect("the program loads")
    })
}

/// The certified guarded setup tables, for each moving orbit's buffers.
fn setups() -> &'static SetupTable {
    static SETUPS: OnceLock<SetupTable> = OnceLock::new();
    SETUPS.get_or_init(|| SetupTable::load(&repository().join("assets")).expect("the certified setup table loads"))
}

/// A moving orbit's buffers A and B, as the setup table gives them.
fn buffers(orbit: i64) -> [i64; 2] {
    let orbit = usize::try_from(orbit).expect("a moving orbit");
    setups().buffers(orbit).expect("the orbit's buffers").map(|buffer| buffer as i64)
}

/// The kernel's orbit table: each orbit, -1 to 34, with its pieces in increasing order, from the kernel's `positions`
/// at the solved state.
fn table() -> &'static BTreeMap<i64, Vec<i64>> {
    static TABLE: OnceLock<BTreeMap<i64, Vec<i64>>> = OnceLock::new();
    TABLE.get_or_init(|| {
        let kernel = kernel();
        let every: Vec<i64> = (0..PIECES as i64).collect();
        let mut table: BTreeMap<i64, Vec<i64>> = BTreeMap::new();
        for at in kernel.positions(&kernel.solved(), &every).expect("every piece") {
            table.entry(i64::from(at.orbit)).or_default().push(at.piece as i64);
        }
        table
    })
}

/// An orbit's pieces in the kernel's orbit table.
fn members(orbit: i64) -> &'static [i64] {
    table().get(&orbit).unwrap_or_else(|| panic!("orbit {orbit} in the table"))
}

/// The state [`SCRAMBLE`] derives from solved.
fn scrambled() -> Snapshot {
    let kernel = kernel();
    let letters: Vec<i64> = SCRAMBLE.iter().map(|&m| i64::from(m)).collect();
    let recipe = kernel.word(&letters).expect("a well-formed word");
    match kernel.derive(&kernel.solved(), &recipe, &ProtectedSet::none()).expect("no star certificate fails") {
        Derivation::Derived(derived) => derived.state,
        Derivation::Refused(refused) => panic!("refused with no protected orbit: {}", refused.refusal),
    }
}

/// A state in which each of `pieces`, an orbit's, holds the next piece's position with its stickers in that position's
/// home order, and the last holds the first's: every piece of the orbit away from Home. No twist need reach it; the
/// kernel reads it back from its labels.
fn cycled(pieces: &[i64]) -> Snapshot {
    let kernel = kernel();
    let solved = kernel.solved();
    let homes = kernel.positions(&solved, pieces).expect("the orbit's pieces");
    let mut labels = solved.labels().to_vec();
    for (i, from) in homes.iter().enumerate() {
        let to = &homes[(i + 1) % homes.len()];
        for (&sticker, &slot) in from.home.iter().zip(&to.home) {
            labels[slot as usize] = sticker;
        }
    }
    kernel.state_from_labels(labels).expect("a permutation of the slots")
}

/// A layer whose session starts at `start`, with no protected orbit.
fn layer_at(start: Snapshot) -> CommandLayer {
    let none = ProtectedSet::none();
    CommandLayer::new(kernel(), Table::h08(), Secret::fixed([5; 32]), Default::default(), "orbits", start, none)
}

/// The context of one run of `entry` of the module with source `source` in the solving profile, built in, with a
/// state read (LG3.13).
fn context(layer: &CommandLayer, source: &str, entry: &str, inputs: usize) -> Context {
    let producer = Producer {
        contract: contract(),
        checker: Some(checker()),
        author: Author::BuiltIn,
        module: sha256(source.as_bytes()),
        entry: entry.into(),
        profile: Profile::Solving,
    };
    let base = base(layer.revision());
    let inputs = vec![named_word_block(); inputs];
    Context { run: "orbits".into(), base, producer, reads_state: true, inputs, pins: pins() }
}

/// Runs `module.entry` on `args` within the ceiling through a boundary and a bridge onto `layer`; a read sends
/// no invocation and changes nothing.
fn run(layer: &mut CommandLayer, module: &str, entry: &str, args: Vec<Value>) -> Run {
    let source = if module == ORBITS_MODULE { ORBITS_SOURCE } else { client_source() };
    let before = (layer.revision().clone(), layer.store().log.len(), layer.snapshot().digest().to_owned());
    let context = context(layer, source, entry, args.len());
    let mut bridge = Bridge::new(layer, "orbits");
    let mut store = Store::default();
    let mut boundary = Boundary::new(&mut bridge, &mut store, context);
    let run = loaded().run(module, entry, args, &mut boundary, &Options::new(CEILING)).expect("the run starts");
    drop(boundary);
    assert!(bridge.sent.is_empty(), "a read sends no invocation");
    let after = (layer.revision().clone(), layer.store().log.len(), layer.snapshot().digest().to_owned());
    assert_eq!(after, before, "a read changes nothing");
    run
}

fn completed(run: Run) -> Value {
    assert_eq!((run.summary.outcome, run.summary.code), (Outcome::Completed, None), "{:?}", run.summary.stop);
    run.value.expect("an answer")
}

fn list(items: &[i64]) -> Value {
    Value::list(items.iter().map(|&item| Value::int(item)).collect())
}

fn field<'v>(value: &'v Value, name: &str) -> &'v Value {
    let Value::Record(record) = value else { panic!("a record: {value:?}") };
    record.get(name).unwrap_or_else(|| panic!("a field {name}: {value:?}"))
}

fn names(value: &Value) -> Vec<&str> {
    let Value::Record(record) = value else { panic!("a record: {value:?}") };
    record.fields().iter().map(|(name, _)| name.as_str()).collect()
}

fn int(value: &Value) -> i64 {
    let Value::Int(n) = value else { panic!("an integer: {value:?}") };
    n.to_string().parse().expect("an i64")
}

fn boolean(value: &Value) -> bool {
    let Value::Bool(b) = value else { panic!("a Bool: {value:?}") };
    *b
}

fn items(value: &Value) -> &[Value] {
    let Value::List(items) = value else { panic!("a list: {value:?}") };
    items.items()
}

fn ints(value: &Value) -> Vec<i64> {
    items(value).iter().map(int).collect()
}

/// `orbit_catalogue` at `named` on `layer`: each named orbit with its pieces.
fn catalogue(layer: &mut CommandLayer, named: &[i64]) -> Vec<(i64, Vec<i64>)> {
    let answer = completed(run(layer, ORBITS_MODULE, "orbit_catalogue", vec![list(named)]));
    let record = |record: &Value| {
        assert_eq!(names(record), ["orbit", "pieces"]);
        (int(field(record, "orbit")), ints(field(record, "pieces")))
    };
    items(&answer).iter().map(record).collect()
}

/// A progress answer: (pieces, solved, position wrong, orientation wrong, others).
fn progress_of(answer: &Value, orbit: i64) -> [i64; 5] {
    assert_eq!(names(answer), ["orbit", "orientation_wrong", "others", "pieces", "position_wrong", "solved"]);
    assert_eq!(int(field(answer, "orbit")), orbit);
    ["pieces", "solved", "position_wrong", "orientation_wrong", "others"].map(|name| int(field(answer, name)))
}

/// The client's progress of `orbit` over the records `positions` answers for `pieces`.
fn progress(layer: &mut CommandLayer, orbit: i64, pieces: &[i64]) -> [i64; 5] {
    let answer = completed(run(layer, CLIENT_MODULE, "progress_at", vec![Value::int(orbit), list(pieces)]));
    progress_of(&answer, orbit)
}

/// A residuals answer.
#[derive(Debug, PartialEq, Eq)]
struct Residuals {
    complete: bool,
    solved: bool,
    misplaced: Vec<i64>,
    misoriented: Vec<i64>,
    unknown: Vec<i64>,
    /// Buffer A's and buffer B's occupant, and whether it is solved.
    buffers: [(i64, bool); 2],
}

fn residuals_of(answer: &Value, orbit: i64, buffers: [i64; 2]) -> Residuals {
    let fields = ["buffer_a", "buffer_b", "complete", "misoriented", "misplaced", "orbit", "solved", "unknown"];
    assert_eq!(names(answer), fields);
    assert_eq!(int(field(answer, "orbit")), orbit);
    let buffer = |name: &str, position: i64| {
        let buffer = field(answer, name);
        assert_eq!(names(buffer), ["occupant", "position", "solved"]);
        assert_eq!(int(field(buffer, "position")), position, "{name} as named");
        (int(field(buffer, "occupant")), boolean(field(buffer, "solved")))
    };
    Residuals {
        complete: boolean(field(answer, "complete")),
        solved: boolean(field(answer, "solved")),
        misplaced: ints(field(answer, "misplaced")),
        misoriented: ints(field(answer, "misoriented")),
        unknown: ints(field(answer, "unknown")),
        buffers: [buffer("buffer_a", buffers[0]), buffer("buffer_b", buffers[1])],
    }
}

/// The client's residuals of `orbit`, whose catalogue row it reads, over the records `positions` answers for `pieces`.
fn residuals(layer: &mut CommandLayer, orbit: i64, pieces: &[i64], buffers: [i64; 2]) -> Residuals {
    let args = vec![list(&[orbit]), list(pieces), Value::int(buffers[0]), Value::int(buffers[1])];
    let answer = completed(run(layer, CLIENT_MODULE, "residuals_at", args));
    residuals_of(&answer, orbit, buffers)
}

/// The residuals by their definitions (solving workflow section 3.4; `c600_library`'s Drafts), from the kernel's
/// `positions` of an orbit's members in the catalogue's order.
fn expected(rows: &[PiecePosition], buffers: [i64; 2]) -> Residuals {
    let mut answer = Residuals {
        complete: !rows.is_empty(),
        solved: false,
        misplaced: Vec::new(),
        misoriented: Vec::new(),
        unknown: Vec::new(),
        buffers: [(-1, false); 2],
    };
    for row in rows {
        let piece = row.piece as i64;
        let position = row.position.map_or(-1, |position| position as i64);
        let home = row.slots == row.home;
        let own = buffers.iter().position(|&buffer| buffer == piece);
        if position == piece {
            match own {
                Some(i) => answer.buffers[i] = (piece, home),
                None if !home => answer.misoriented.push(piece),
                None => {}
            }
        } else {
            if own.is_none() {
                answer.misplaced.push(piece);
            }
            if position == -1 {
                answer.unknown.push(piece);
            }
            if let Some(i) = buffers.iter().position(|&buffer| buffer == position) {
                answer.buffers[i] = (piece, false);
            }
        }
    }
    answer.solved = answer.complete && rows.iter().all(|row| row.slots == row.home);
    answer
}

/// The built-in column admits `orbit_catalogue` only when its class lists the `orbits` row, and the entry reads state
/// and prepares no preview (LG3.13); `orbit_progress` and `orbit_residuals` read nothing and are admitted with no row.
/// The client's entries are admitted with the rows they read at their parameters as given, and refused without them;
/// a read of `positions` at the catalogue's answer is a prefetch, refused with `LG3:capability.not_granted` (LB3.5),
/// which is why the positions are a parameter.
#[test]
fn the_entries_are_admitted_with_their_rows() {
    assert_eq!(ORBITS_ROWS, [ORBITS_ROW]);
    assert_eq!(ORBITS_EXPORTS, ["orbit_catalogue", "orbit_progress", "orbit_residuals"]);
    let program = lower_sources(&[ORBITS_SOURCE, client_source()]).unwrap();
    let admit = |module: &str, entry: &str, rows: &[&str]| {
        let class: BTreeSet<Row> = rows.iter().map(|row| Row::Read(row.to_string())).collect();
        admit_built_in(&program, module, entry, &class, &Grant::none(), &Policy::interim()).unwrap()
    };
    let catalogue = admit(ORBITS_MODULE, "orbit_catalogue", &ORBITS_ROWS);
    assert!(catalogue.is_admitted(), "{:?}", catalogue.diagnostics);
    assert!(catalogue.separation.reads_state && !catalogue.separation.prepares_previews);
    assert!(!admit(ORBITS_MODULE, "orbit_catalogue", &[POSITIONS]).is_admitted(), "the catalogue without its row");
    for entry in ["orbit_progress", "orbit_residuals"] {
        let pure = admit(ORBITS_MODULE, entry, &[]);
        assert!(pure.is_admitted(), "{entry}: {:?}", pure.diagnostics);
        assert!(!pure.separation.reads_state && !pure.separation.prepares_previews, "{entry}");
    }
    for (entry, rows) in [("progress_at", &[POSITIONS][..]), ("residuals_at", &[ORBITS_ROW, POSITIONS][..])] {
        let reading = admit(CLIENT_MODULE, entry, rows);
        assert!(reading.is_admitted(), "{entry}: {:?}", reading.diagnostics);
        assert!(reading.separation.reads_state && !reading.separation.prepares_previews, "{entry}");
        for fewer in 0..rows.len() {
            assert!(!admit(CLIENT_MODULE, entry, &rows[..fewer]).is_admitted(), "{entry} with {:?}", &rows[..fewer]);
        }
    }
    let chained = admit(CLIENT_MODULE, "chained", &[ORBITS_ROW, POSITIONS]);
    let codes: Vec<&str> = chained.diagnostics.iter().map(|d| d.code).collect();
    assert!(!chained.is_admitted() && !codes.is_empty(), "a prefetch is refused");
    assert!(codes.iter().all(|&code| code == NOT_GRANTED), "{:?}", chained.diagnostics);
}

/// The catalogue answers the kernel's orbit table: for every orbit, -1 to 34, its pieces in increasing order, the same
/// at solved and at a scrambled head. The 36 answers split the 177,120 pieces between them, the largest has
/// [`MAX_ORBIT_PIECES`], and named orbits come back in the argument's order.
#[test]
fn the_catalogue_answers_the_kernel_orbit_table() {
    let every: Vec<i64> = (-1..ORBITS as i64).collect();
    let at_solved = catalogue(&mut layer_at(kernel().solved()), &every);
    let mut layer = layer_at(scrambled());
    let answer = catalogue(&mut layer, &every);
    assert_eq!(answer, at_solved, "model data, the same at every head");
    let reference: Vec<(i64, Vec<i64>)> = table().iter().map(|(&orbit, pieces)| (orbit, pieces.clone())).collect();
    assert_eq!(answer, reference, "the kernel's orbit table");
    assert_eq!(answer.iter().map(|(_, pieces)| pieces.len()).sum::<usize>(), PIECES);
    assert_eq!(answer.iter().map(|(_, pieces)| pieces.len()).max(), Some(MAX_ORBIT_PIECES));
    let named = [34, -1, 7, 34];
    let some = catalogue(&mut layer, &named);
    let reference: Vec<(i64, Vec<i64>)> = named.iter().map(|&orbit| (orbit, members(orbit).to_vec())).collect();
    assert_eq!(some, reference, "in the argument's order, repeats included");
}

/// At a scrambled head, the progress of each moving orbit over its catalogue pieces is the kernel's progress row for
/// the orbit: pieces, solved, position wrong and orientation wrong, with no record of another orbit. The fixed cell
/// centres, orbit -1, are all solved, and records of another orbit are counted in `others`, not in the orbit's counts.
#[test]
fn the_progress_per_orbit_is_the_kernels_progress_row() {
    let state = scrambled();
    let rows = kernel().progress(&state).expect("the progress rows").rows;
    assert_eq!(rows.len(), ORBITS);
    let mut layer = layer_at(state);
    let mut wrong = 0;
    for row in &rows {
        let orbit = row.orbit as i64;
        let counts = [row.pieces, row.solved, row.position_wrong, row.orientation_wrong, 0].map(|n| n as i64);
        assert_eq!(progress(&mut layer, orbit, members(orbit)), counts, "orbit {orbit}");
        wrong += row.position_wrong + row.orientation_wrong;
    }
    assert!(wrong > 0, "the scramble moves pieces");
    assert_eq!(progress(&mut layer, -1, members(-1)), [600, 600, 0, 0, 0], "the fixed cell centres");

    let mut mixed = members(0).to_vec();
    mixed.extend(members(1));
    let mut counts = progress(&mut layer, 0, members(0));
    counts[4] = members(1).len() as i64;
    assert_eq!(progress(&mut layer, 0, &mixed), counts, "orbit 1's records counted apart");
}

/// At a scrambled head, the residuals of each moving orbit, with the buffers of the certified setup table and its
/// catalogue pieces named, are what the definitions give on the kernel's `positions`, in the catalogue's order:
/// complete, with no frame unknown, since no twist splits a piece. Against the kernel's progress row: the misplaced
/// pieces and the buffers' own pieces away are the pieces in a wrong position, the misoriented pieces and the buffers'
/// own pieces at Home with a sticker elsewhere are the pieces with a wrong orientation, and the orbit is solved exactly
/// when every piece is.
#[test]
fn the_residuals_per_orbit_list_what_the_positions_show() {
    let kernel = kernel();
    let state = scrambled();
    let rows = kernel.progress(&state).expect("the progress rows").rows;
    let mut layer = layer_at(state.clone());
    let mut listed = 0;
    for row in &rows {
        let orbit = row.orbit as i64;
        let (pieces, buffers) = (members(orbit), buffers(orbit));
        let answer = residuals(&mut layer, orbit, pieces, buffers);
        assert_eq!(answer, expected(&kernel.positions(&state, pieces).unwrap(), buffers), "orbit {orbit}");
        assert!(answer.complete && answer.unknown.is_empty(), "orbit {orbit}");
        let own = |i: usize| answer.buffers[i].0 == buffers[i];
        let away = (0..2).filter(|&i| !own(i)).count();
        let turned = (0..2).filter(|&i| own(i) && !answer.buffers[i].1).count();
        assert_eq!(answer.misplaced.len() + away, row.position_wrong, "orbit {orbit}: position wrong");
        assert_eq!(answer.misoriented.len() + turned, row.orientation_wrong, "orbit {orbit}: orientation wrong");
        assert_eq!(answer.solved, row.solved == row.pieces, "orbit {orbit}: solved");
        listed += answer.misplaced.len() + answer.misoriented.len();
    }
    assert!(listed > 0, "the scramble leaves residuals");
}

/// At solved, every moving orbit with its catalogue pieces named is complete and solved, its lists empty and each
/// buffer held by its own piece, solved. Naming fewer pieces than the catalogue lists, or the same pieces in another
/// order, is never complete and never solved, though every named record is at Home.
#[test]
fn at_solved_an_orbit_is_solved_only_when_every_member_is_named() {
    let mut layer = layer_at(kernel().solved());
    for orbit in 0..ORBITS as i64 {
        let (pieces, buffers) = (members(orbit), buffers(orbit));
        let answer = residuals(&mut layer, orbit, pieces, buffers);
        let solved = Residuals {
            complete: true,
            solved: true,
            misplaced: Vec::new(),
            misoriented: Vec::new(),
            unknown: Vec::new(),
            buffers: [(buffers[0], true), (buffers[1], true)],
        };
        assert_eq!(answer, solved, "orbit {orbit}");
        if orbit % 17 == 0 {
            let partial = residuals(&mut layer, orbit, &pieces[1..], buffers);
            assert_eq!((partial.complete, partial.solved), (false, false), "orbit {orbit}: a partial list");
            let mut reversed = pieces.to_vec();
            reversed.reverse();
            let reversed = residuals(&mut layer, orbit, &reversed, buffers);
            assert_eq!((reversed.complete, reversed.solved), (false, false), "orbit {orbit}: another order");
        }
    }
}

/// The largest orbit, [`MAX_ORBIT_PIECES`] pieces, with every piece away from Home, each in the next piece's position:
/// the client's progress and residuals each run whole, their reads first, within the budget ceiling, and answer
/// what the definitions and the kernel's progress row give. The amounts used are printed.
#[test]
fn the_largest_orbit_runs_within_the_draft_ceiling() {
    let (&orbit, pieces) = table().iter().find(|(_, pieces)| pieces.len() == MAX_ORBIT_PIECES).expect("an orbit");
    let state = cycled(pieces);
    let kernel = kernel();
    let rows = kernel.progress(&state).expect("the progress rows").rows;
    let row = &rows[orbit as usize];
    assert_eq!((row.pieces, row.solved, row.position_wrong), (MAX_ORBIT_PIECES, 0, MAX_ORBIT_PIECES));
    let positions = kernel.positions(&state, pieces).unwrap();
    assert!(positions.iter().all(|at| at.position.is_some_and(|position| position as i64 != at.piece as i64)));
    let buffers = buffers(orbit);
    let mut layer = layer_at(state);
    let n = MAX_ORBIT_PIECES as i64;

    let read = run(&mut layer, CLIENT_MODULE, "progress_at", vec![Value::int(orbit), list(pieces)]);
    let used = read.summary.used;
    eprintln!(
        "progress_at, orbit {orbit}: {} steps, {} kernel cost, {} memory",
        used.steps, used.kernel_cost, used.memory
    );
    assert_eq!(progress_of(&completed(read), orbit), [n, 0, n, 0, 0]);

    let args = vec![list(&[orbit]), list(pieces), Value::int(buffers[0]), Value::int(buffers[1])];
    let read = run(&mut layer, CLIENT_MODULE, "residuals_at", args);
    let used = read.summary.used;
    eprintln!(
        "residuals_at, orbit {orbit}: {} steps, {} kernel cost, {} memory",
        used.steps, used.kernel_cost, used.memory
    );
    let answer = residuals_of(&completed(read), orbit, buffers);
    assert_eq!(answer, expected(&positions, buffers));
    assert!(answer.complete && !answer.solved);
    assert_eq!(answer.misplaced.len(), MAX_ORBIT_PIECES - 2, "every piece but the buffers' own");
}

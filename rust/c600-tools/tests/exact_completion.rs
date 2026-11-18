//! CT-LB2-04 (whole: exact completion reads identities, slots and orientations, and empty candidates, a permitted
//! residual or unchanged colours cannot pass it) on 600-cell-Full. The exact-completion entry is library code,
//! `c600-library`'s `completion.exact_completion`, admitted by the built-in column with the read rows `positions` and
//! `labels` and run on `c600-run` through `Boundary` and the tool's host bridge onto a command layer, in the solving
//! profile, author `built_in`; the bridge serves both rows at the layer's head (`c600_tools::kernel_rows`). Every case
//! judges completion by identity facts the kernel gives: each piece's position, its slots against its home slots, and
//! the labels of its home slots. Colours appear only where a case shows they are unchanged.
//!
//! The solved state and the state a fixed scramble derives are reached by twists. The states of the colour-preserving
//! exchange and the permitted residual are label arrays built from solved with `Kernel::state_from_labels`, which
//! accepts any permutation of the slots and does not decide whether twists reach it. A pass is source and synthetic
//! evidence.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use c600_check::{admit_built_in, Grant, Policy, Row, NOT_GRANTED};
use c600_command::{code, CommandLayer, Secret, Table};
use c600_digest::sha256;
use c600_kernel::{Derivation, Kernel, PiecePosition, ProtectedSet, Snapshot, PIECES, SLOTS};
use c600_library::{COMPLETION_EXPORTS, COMPLETION_MODULE, COMPLETION_ROWS, COMPLETION_SOURCE};
use c600_run::boundary::{Boundary, Context, Store};
use c600_run::interp::{load, Loaded, Options, Run};
use c600_run::meter::{Amounts, Outcome};
use c600_run::provenance::{Author, Producer, Profile};
use c600_run::value::Value;
use c600_tools::bridge::{base, Bridge};
use c600_tools::kernel_rows::{LABELS, POSITIONS};
use c600_tools::tool::{checker, contract, lower_source, named_word_block, pins};

const ENTRY: &str = COMPLETION_EXPORTS[0];

/// The slots of one cell: 600 cells of 433 slots each (0.4's `core.py`, `c600_jumble::model::CELL_SLOTS`), so a
/// sticker's colour is the cell of its home slot, `label / 433`, and a slot's cell is `slot / 433`. Only the
/// colour-preserving case reads it, to pick its pieces and to show that no slot changes colour; the entry reads no
/// colour.
const CELL_SLOTS: usize = 433;

const LIMITS: Amounts = Amounts { steps: 1 << 20, kernel_cost: 1 << 40, memory: 1 << 32, output: 1 << 26 };

/// The entry as it would be if it read `labels` at the home slots the `positions` answer gives, not at slots the solver
/// named.
const COMPUTED: &str = "edition \"6cs-draft-1\";\nmodule completion requires [];\n\n\
    export fn exact_completion(pieces: List<Nat>) -> Bool effects [snapshot] {\n    \
    let rows = read_snapshot<List<record { piece: Nat, orbit: Int, home: List<Nat>, slots: List<Nat>, \
    position: Int }>>(\"positions\", pieces);\n    \
    let home = index(rows, 0).home;\n    \
    return exact_eq(read_snapshot<List<Nat>>(\"labels\", home), home);\n}\n";

/// A fixed scramble from solved.
const SCRAMBLE: [i64; 20] =
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

fn loaded() -> Loaded {
    load(&lower_source(COMPLETION_SOURCE).expect("the library entry lowers")).expect("the library entry loads")
}

/// The state [`SCRAMBLE`] derives from solved.
fn scrambled() -> Snapshot {
    let kernel = kernel();
    let recipe = kernel.word(&SCRAMBLE).expect("a well-formed word");
    match kernel.derive(&kernel.solved(), &recipe, &ProtectedSet::none()).expect("no star certificate fails") {
        Derivation::Derived(derived) => derived.state,
        Derivation::Refused(refused) => panic!("refused with no protected orbit: {}", refused.refusal),
    }
}

/// Every piece's row in `state`, indexed by piece.
fn all_rows(state: &Snapshot) -> Vec<PiecePosition> {
    let all: Vec<i64> = (0..PIECES as i64).collect();
    kernel().positions(state, &all).expect("every piece is named")
}

/// The rows of `pieces` in `state`, in order.
fn rows_of(state: &Snapshot, pieces: &[usize]) -> Vec<PiecePosition> {
    let named: Vec<i64> = pieces.iter().map(|&piece| piece as i64).collect();
    kernel().positions(state, &named).expect("the pieces are named")
}

/// The state in which pieces `a` and `b` of solved exchange places, sticker for sticker: the sticker whose home slot
/// is `a.home[i]` goes to `b.home[i]`, and the reverse.
fn exchanged(a: &PiecePosition, b: &PiecePosition) -> Snapshot {
    let kernel = kernel();
    let mut labels = kernel.solved().labels().to_vec();
    for (&x, &y) in a.home.iter().zip(&b.home) {
        labels.swap(x as usize, y as usize);
    }
    kernel.state_from_labels(labels).expect("a permutation of the slots")
}

/// The labels of `slots` in `state`.
fn labels_at(state: &Snapshot, slots: &[i32]) -> Vec<i32> {
    slots.iter().map(|&slot| state.labels()[slot as usize]).collect()
}

/// The first complete piece of each orbit that has one in `rows`, in orbit order, fixed cell centres (orbit -1)
/// included, leaving out `except`.
fn complete_per_orbit(rows: &[PiecePosition], except: &[usize]) -> Vec<usize> {
    let mut first: BTreeMap<i32, usize> = BTreeMap::new();
    for row in rows.iter().filter(|row| row.solved() && !except.contains(&row.piece)) {
        first.entry(row.orbit).or_insert(row.piece);
    }
    first.into_values().collect()
}

/// A layer whose session starts at `start`, with no protected orbit.
fn layer_at(start: Snapshot) -> CommandLayer {
    let none = ProtectedSet::none();
    CommandLayer::new(kernel(), Table::h08(), Secret::fixed([5; 32]), Default::default(), "completion", start, none)
}

/// The context of one run of the entry in the solving profile, built in, with a state read (LG3.13).
fn context(layer: &CommandLayer) -> Context {
    let producer = Producer {
        contract: contract(),
        checker: Some(checker()),
        author: Author::BuiltIn,
        module: sha256(COMPLETION_SOURCE.as_bytes()),
        entry: ENTRY.into(),
        profile: Profile::Solving,
    };
    let base = base(layer.revision());
    let inputs = vec![named_word_block(); 2];
    Context { run: "complete".into(), base, producer, reads_state: true, inputs, pins: pins() }
}

/// The home slots of `pieces`, in order: model data, the same in every state, which the host names to the entry with
/// the pieces.
fn home_slots(pieces: &[usize]) -> Vec<usize> {
    let rows = rows_of(&kernel().solved(), pieces);
    rows.iter().flat_map(|row| row.home.iter().map(|&slot| slot as usize)).collect()
}

fn list(items: &[usize]) -> Value {
    Value::list(items.iter().map(|&item| Value::int(item as i64)).collect())
}

/// Runs the entry on `pieces` and `slots` through a boundary and a bridge onto `layer`.
fn run(layer: &mut CommandLayer, pieces: &[usize], slots: &[usize]) -> Run {
    let context = context(layer);
    let mut bridge = Bridge::new(layer, "complete");
    let mut store = Store::default();
    let mut boundary = Boundary::new(&mut bridge, &mut store, context);
    let args = vec![list(pieces), list(slots)];
    let run = loaded().run(COMPLETION_MODULE, ENTRY, args, &mut boundary, &Options::new(LIMITS));
    drop(boundary);
    assert!(bridge.sent.is_empty(), "a read sends no invocation");
    run.expect("the run starts")
}

/// The entry's answer for `pieces` named with `slots` at `layer`'s head: the run completes with a `Bool` and changes
/// nothing.
fn answer(layer: &mut CommandLayer, pieces: &[usize], slots: &[usize]) -> bool {
    let before = (layer.revision().clone(), layer.store().log.len());
    let run = run(layer, pieces, slots);
    assert_eq!((run.summary.outcome, run.summary.code), (Outcome::Completed, None), "{:?}", run.summary.stop);
    assert_eq!((layer.revision().clone(), layer.store().log.len()), before, "a read changes nothing");
    match run.value {
        Some(Value::Bool(answer)) => answer,
        other => panic!("a Bool: {other:?}"),
    }
}

/// The entry's answer for `candidates` named with their own home slots.
fn completes(layer: &mut CommandLayer, candidates: &[usize]) -> bool {
    answer(layer, candidates, &home_slots(candidates))
}

/// The built-in column admits the entry only when its class lists both read rows, `positions` and `labels`; the entry
/// reads state and prepares no preview (LG3.13).
#[test]
fn ct_lb2_04_the_entry_is_admitted_with_both_read_rows() {
    assert_eq!(COMPLETION_ROWS, [LABELS, POSITIONS]);
    let program = lower_source(COMPLETION_SOURCE).unwrap();
    let admit = |rows: &[&str]| {
        let class: BTreeSet<Row> = rows.iter().map(|row| Row::Read(row.to_string())).collect();
        admit_built_in(&program, COMPLETION_MODULE, ENTRY, &class, &Grant::none(), &Policy::interim()).unwrap()
    };
    let admitted = admit(&COMPLETION_ROWS);
    assert!(admitted.is_admitted(), "{:?}", admitted.diagnostics);
    assert!(admitted.separation.reads_state && !admitted.separation.prepares_previews);
    assert!(!admit(&[POSITIONS]).is_admitted(), "the entry reads `labels`");
    assert!(!admit(&[LABELS]).is_admitted(), "the entry reads `positions`");

    // The built-in column reads only for what the solver named (LB3.5), so an entry that reads `labels` at home slots
    // it took from the `positions` answer is refused with both rows in its class. That is why the entry takes the
    // candidates' home slots as named.
    let program = lower_source(COMPUTED).expect("the variant lowers");
    let class: BTreeSet<Row> = COMPLETION_ROWS.iter().map(|row| Row::Read(row.to_string())).collect();
    let refused = admit_built_in(&program, COMPLETION_MODULE, ENTRY, &class, &Grant::none(), &Policy::interim());
    let refused = refused.unwrap();
    assert!(!refused.is_admitted());
    assert!(refused.diagnostics.iter().all(|d| d.code == NOT_GRANTED), "{:?}", refused.diagnostics);
}

/// The entry reads `labels` at the home slots the host names with the candidates, and it answers `true` only when those
/// are the candidates' own home slots, in order. On the solved state, where every piece is complete, it answers `false`
/// for another piece's home slots, for none, for the candidates' home slots in another order or in part, and for more
/// than theirs, so a host that names the wrong slots never gets a `true` it should not.
#[test]
fn ct_lb2_04_the_named_home_slots_are_the_candidates_own() {
    let solved = kernel().solved();
    let rows = all_rows(&solved);
    let candidates = complete_per_orbit(&rows, &[]);
    let p = *candidates.iter().find(|&&piece| rows[piece].home.len() > 1).expect("a piece with two stickers or more");
    let q = *candidates.iter().find(|&&piece| piece != p).expect("another piece");

    let mut layer = layer_at(solved);
    assert!(answer(&mut layer, &[p], &home_slots(&[p])));
    assert!(answer(&mut layer, &[p, q], &home_slots(&[p, q])));
    assert!(!answer(&mut layer, &[p], &home_slots(&[q])), "another piece's home slots");
    assert!(!answer(&mut layer, &[p], &[]), "no home slots");
    let mut reversed = home_slots(&[p]);
    reversed.reverse();
    assert!(!answer(&mut layer, &[p], &reversed), "its home slots in another order");
    assert!(!answer(&mut layer, &[p, q], &home_slots(&[q, p])), "the pieces' home slots in another order");
    assert!(!answer(&mut layer, &[p, q], &home_slots(&[p])), "part of the home slots");
    assert!(!answer(&mut layer, &[p], &home_slots(&[p, q])), "more than its home slots");
}

/// CT-LB2-04, solved: every piece of the solved state is complete, so the entry answers `true` for one piece of each
/// orbit, fixed cell centres included, and for each of them alone. The identity facts it reads agree: each piece holds
/// its own position, its slots are its home slots, and the label of each home slot is that slot.
#[test]
fn ct_lb2_04_solved_passes() {
    let solved = kernel().solved();
    let rows = all_rows(&solved);
    let candidates = complete_per_orbit(&rows, &[]);
    assert_eq!(candidates.len(), 36, "35 orbits and the fixed cell centres");
    for &piece in &candidates {
        let row = &rows[piece];
        assert!(row.position_correct() && row.solved(), "{piece}");
        assert_eq!(labels_at(&solved, &row.home), row.home, "{piece}");
    }

    let mut layer = layer_at(solved);
    assert!(completes(&mut layer, &candidates));
    for &piece in &candidates {
        assert!(completes(&mut layer, &[piece]), "{piece}");
    }
}

/// CT-LB2-04, empty candidates: an empty list establishes nothing, so the entry answers `false` for it, even on the
/// solved state, and does not stop the run. A piece outside 0 to 177,119 does stop it, with the `positions` row's
/// refusal.
#[test]
fn ct_lb2_04_an_empty_list_does_not_pass() {
    let mut layer = layer_at(kernel().solved());
    assert!(!completes(&mut layer, &[]));
    assert!(completes(&mut layer, &[0]));
    let outside = run(&mut layer, &[0, PIECES], &home_slots(&[0]));
    let summary = &outside.summary;
    let stop = (summary.outcome, summary.code);
    assert_eq!(stop, (Outcome::Error, Some(code::NAMED_OBJECT_INVALID)), "{:?}", summary.stop);
}

/// CT-LB2-04, orientation: in the state a fixed scramble derives from solved, a piece in its home position but turned
/// holds its own stickers in its own slots, in another order. The entry answers `false` for it alone and among pieces
/// the scramble left complete, and `true` for those pieces without it. The identity facts: the piece holds its own
/// position, its slots are its home slots in another order, and the labels of its home slots are its home slots in
/// another order.
#[test]
fn ct_lb2_04_a_piece_twisted_in_place_does_not_pass() {
    let state = scrambled();
    let rows = all_rows(&state);
    let turned = rows.iter().find(|row| row.position_correct() && !row.solved()).expect("a turned piece");
    let mut slots = turned.slots.clone();
    slots.sort_unstable();
    assert_eq!(slots, turned.home, "its stickers are in its own slots");
    let labels = labels_at(&state, &turned.home);
    assert_ne!(labels, turned.home, "its home slots hold its stickers in another order");
    let mut sorted = labels.clone();
    sorted.sort_unstable();
    assert_eq!(sorted, turned.home);
    let complete = complete_per_orbit(&rows, &[]);
    assert!(complete.len() > 1 && !complete.contains(&turned.piece));

    let mut layer = layer_at(state);
    assert!(!completes(&mut layer, &[turned.piece]));
    assert!(completes(&mut layer, &complete));
    let mut with = complete.clone();
    with.insert(complete.len() / 2, turned.piece);
    assert!(!completes(&mut layer, &with));
}

/// CT-LB2-04, unchanged colours: two pieces of one orbit whose home slots lie in the same cells, slot for slot (the
/// pair with the most stickers), exchange places from solved. Every slot then holds a sticker of its own colour, so a
/// reading by colour sees the solved puzzle; but the entry answers `false` for either piece, for both, and for both
/// among complete pieces, and `true` for the complete pieces alone. The identity facts: each piece holds the other's
/// position, its slots are the other's home slots, and the labels of its home slots are the other's home slots.
#[test]
fn ct_lb2_04_a_colour_preserving_exchange_does_not_pass() {
    let solved = kernel().solved();
    let rows = all_rows(&solved);
    assert_eq!(SLOTS, 600 * CELL_SLOTS);
    let centres: BTreeSet<usize> =
        rows.iter().filter(|row| row.orbit == -1).map(|row| row.home[0] as usize / CELL_SLOTS).collect();
    assert_eq!(centres.len(), 600, "one fixed cell centre in each block of 433 slots");

    let cells = |row: &PiecePosition| row.home.iter().map(|&slot| slot as usize / CELL_SLOTS).collect::<Vec<usize>>();
    let mut groups: BTreeMap<(i32, Vec<usize>), Vec<usize>> = BTreeMap::new();
    for row in rows.iter().filter(|row| row.orbit >= 0) {
        groups.entry((row.orbit, cells(row))).or_default().push(row.piece);
    }
    let pair = groups.values().filter(|pieces| pieces.len() > 1).max_by_key(|pieces| rows[pieces[0]].home.len());
    let pair = pair.expect("two pieces of one orbit in the same cells");
    let (a, b) = (pair[0], pair[1]);
    let state = exchanged(&rows[a], &rows[b]);
    assert_ne!(state.digest(), solved.digest());
    let colour = |slot: usize| slot / CELL_SLOTS;
    assert!(state.labels().iter().enumerate().all(|(slot, &label)| colour(label as usize) == colour(slot)));

    let now = rows_of(&state, &[a, b]);
    assert_eq!((&now[0].slots, now[0].position), (&rows[b].home, Some(b)));
    assert_eq!((&now[1].slots, now[1].position), (&rows[a].home, Some(a)));
    assert_eq!(labels_at(&state, &rows[a].home), rows[b].home);
    assert_eq!(labels_at(&state, &rows[b].home), rows[a].home);

    let complete = complete_per_orbit(&rows, &[a, b]);
    let mut layer = layer_at(state);
    assert!(!completes(&mut layer, &[a]));
    assert!(!completes(&mut layer, &[b]));
    assert!(!completes(&mut layer, &[a, b]));
    assert!(completes(&mut layer, &complete));
    assert!(!completes(&mut layer, &[complete.clone(), vec![b, a]].concat()));
}

/// CT-LB2-04, a permitted residual: a buffer position holds another piece of its orbit, a residual that a check of
/// non-buffer positions and buffer occupants would permit. From solved, the buffer (the first piece of the orbit with
/// the most stickers) and another piece of its orbit exchange places, sticker for sticker, and every other piece stays
/// complete. The entry answers `false` for any candidate list that names the buffer, alone or among complete pieces,
/// and `true` for the complete pieces, which name neither piece. The identity facts: the buffer position holds the
/// other piece, and the labels of the buffer's home slots are the other piece's home slots. The entry judges only the
/// pieces the solver names: a list that leaves out both exchanged pieces says nothing about them, so it passes, and
/// the residual fails every list that names the buffer.
#[test]
fn ct_lb2_04_a_permitted_residual_does_not_pass() {
    let solved = kernel().solved();
    let rows = all_rows(&solved);
    let largest = rows.iter().map(|row| row.home.len()).max().expect("pieces");
    let buffer = rows.iter().find(|row| row.home.len() == largest).expect("a piece with the most stickers");
    let occupant = rows
        .iter()
        .find(|row| row.orbit == buffer.orbit && row.piece != buffer.piece && row.home.len() == largest)
        .expect("another piece of the buffer's orbit");
    let state = exchanged(buffer, occupant);

    let now = rows_of(&state, &[buffer.piece, occupant.piece]);
    assert_eq!(now[1].position, Some(buffer.piece), "the buffer position holds the other piece");
    assert_eq!(now[0].position, Some(occupant.piece));
    assert_eq!(labels_at(&state, &buffer.home), occupant.home);
    let incomplete: Vec<usize> = all_rows(&state).iter().filter(|row| !row.solved()).map(|row| row.piece).collect();
    assert_eq!(incomplete, [buffer.piece.min(occupant.piece), buffer.piece.max(occupant.piece)]);

    let complete = complete_per_orbit(&rows, &[buffer.piece, occupant.piece]);
    let mut layer = layer_at(state);
    assert!(!completes(&mut layer, &[buffer.piece]));
    let mut with = complete.clone();
    with.insert(0, buffer.piece);
    assert!(!completes(&mut layer, &with));
    assert!(completes(&mut layer, &complete));
}

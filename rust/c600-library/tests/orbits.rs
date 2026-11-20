//! LB2's orbit catalogue, progress and residuals as library code (chapter 03's rows "Orbit catalogue and collateral
//! stage order" and "Residuals and exact completion"): the source of the module `orbits`, its two catalogue rows, and
//! the pure progress and residuals on records the test writes, without a host. `orbit_catalogue` and both pure entries
//! run on the model's 36 orbits through the command layer in `c600-tools/tests/orbits.rs`. A pass is source evidence.

mod common;

use std::collections::BTreeSet;

use c600_check::{check, separation, Policy, Row, Separation, StateRead};
use c600_digest::sha256_hex;
use c600_ir::Effect;
use c600_library::{
    collateral_stage_order, lower_source, orbit_catalogue_and_progress, orbits_sha256, residuals_and_exact_completion,
    residuals_per_orbit, ORBITS_CATALOGUE, ORBITS_EXPORTS, ORBITS_MODULE, ORBITS_RESIDUALS, ORBITS_ROWS, ORBITS_SOURCE,
};
use c600_run::interp::{load, NoHost, Options};
use c600_run::meter::Outcome;
use c600_run::value::Value;
use c600_syntax::{format, parse};
use common::WIDE;

/// The source parses with no diagnostic, is in the formatter's canonical layout with LF line endings, and lowers to one
/// module, `orbits`, that requests no capability and exports the three entries. The checker reports no diagnostic.
/// `orbit_catalogue` has the effect `snapshot` and reads the draft `orbits` row and no other, so under capability
/// separation it reads state and prepares no preview; `orbit_progress` and `orbit_residuals` are pure: no effect,
/// capability, state read or row. The two catalogue rows split the exports between them, their Effect field is the
/// checker's effects of their functions, and their digest is the source's; each is a second part of a row whose other
/// part another module backs.
#[test]
fn the_orbits_source_checks_and_reads_the_orbits_row() {
    let file = parse(ORBITS_SOURCE.as_bytes());
    assert!(file.diagnostics.is_empty() && file.holes.is_empty(), "{:#?}", file.diagnostics);
    assert_eq!(format(&file).as_deref(), Some(ORBITS_SOURCE), "the source is in the canonical layout");
    assert!(!ORBITS_SOURCE.contains('\r') && ORBITS_SOURCE.ends_with('\n'), "LF line endings");

    let program = lower_source(ORBITS_SOURCE).expect("the source lowers");
    let [module] = &program.modules[..] else { panic!("one module, not {}", program.modules.len()) };
    assert_eq!(module.name, ORBITS_MODULE);
    assert!(module.capabilities.is_empty(), "the module requests no capability");
    let exports: Vec<&str> = module.functions.iter().filter(|f| f.exported).map(|f| f.name.as_str()).collect();
    assert_eq!(exports, ORBITS_EXPORTS);

    let report = check(&program).expect("well-formed IR");
    assert!(report.diagnostics.is_empty(), "{:#?}", report.diagnostics);
    let rows: BTreeSet<Row> = ORBITS_ROWS.iter().map(|row| Row::Read(row.to_string())).collect();
    for entry in ORBITS_EXPORTS {
        let summary = report.summary(ORBITS_MODULE, entry).expect("a summary");
        assert!(summary.capabilities.is_empty() && summary.vars.is_empty() && summary.unresolved.is_empty());
        let separated = separation(summary, &Policy::interim());
        if entry == "orbit_catalogue" {
            assert_eq!(summary.effects, BTreeSet::from([Effect::ReadSnapshot]), "{entry}");
            assert_eq!(summary.state_reads, BTreeSet::from([StateRead::Snapshot]), "{entry}");
            assert_eq!(summary.rows, rows, "{entry}");
            assert_eq!(separated, Separation { reads_state: true, prepares_previews: false });
        } else {
            assert!(summary.effects.is_empty() && summary.state_reads.is_empty() && summary.rows.is_empty(), "{entry}");
            assert_eq!(separated, Separation { reads_state: false, prepares_previews: false });
        }
    }

    let together: BTreeSet<&str> = ORBITS_CATALOGUE.iter().chain(&ORBITS_RESIDUALS).copied().collect();
    assert_eq!(together, ORBITS_EXPORTS.into_iter().collect(), "the two rows split the exports");
    assert_eq!(ORBITS_CATALOGUE.len() + ORBITS_RESIDUALS.len(), ORBITS_EXPORTS.len());
    for (row, other, entry, functions) in [
        (
            orbit_catalogue_and_progress(),
            collateral_stage_order(),
            "Orbit catalogue and collateral stage order",
            &ORBITS_CATALOGUE[..],
        ),
        (
            residuals_per_orbit(),
            residuals_and_exact_completion(),
            "Residuals and exact completion",
            &ORBITS_RESIDUALS[..],
        ),
    ] {
        assert_eq!((row.entry, other.entry), (entry, entry));
        assert_ne!(row.module, other.module, "{entry}: the other part is another module's");
        assert_eq!((row.module, row.functions), (ORBITS_MODULE, functions));
        let effects: BTreeSet<Effect> = functions
            .iter()
            .flat_map(|f| report.summary(ORBITS_MODULE, f).expect("a summary").effects.iter().copied())
            .collect();
        assert_eq!(row.effects.iter().copied().collect::<BTreeSet<_>>(), effects, "{entry}: Effect");
        assert!(functions.iter().all(|f| row.backing.contains(&format!("`orbits.{f}`"))), "{entry}: Backing");
        assert_eq!(row.source_sha256, sha256_hex(ORBITS_SOURCE.as_bytes()));
        assert_eq!(row.source_sha256, orbits_sha256());
    }
    assert!(orbit_catalogue_and_progress().notes.contains("never dropped"));
    assert!(residuals_per_orbit().notes.contains("never establishes"));
}

/// One `positions` record as the row answers it, with its fields in name order.
fn record(piece: i64, orbit: i64, position: i64, home: &[i64], slots: &[i64]) -> Value {
    let ints = |items: &[i64]| Value::list(items.iter().map(|&item| Value::int(item)).collect());
    Value::record(vec![
        ("home", ints(home)),
        ("orbit", Value::int(orbit)),
        ("piece", Value::int(piece)),
        ("position", Value::int(position)),
        ("slots", ints(slots)),
    ])
}

/// Orbit 7 of the test, pieces 10 to 15 with buffers A = 13 and B = 14, in the caller's order: 10 solved; 11 at Home
/// with its two stickers swapped; 12 in buffer A; buffer A's own piece 13 at position 12; buffer B's own piece 14
/// solved; and 15 with its stickers split over positions.
fn orbit_seven() -> Vec<Value> {
    vec![
        record(10, 7, 10, &[100, 101], &[100, 101]),
        record(11, 7, 11, &[110, 111], &[111, 110]),
        record(12, 7, 13, &[120, 121], &[130, 131]),
        record(13, 7, 12, &[130, 131], &[120, 121]),
        record(14, 7, 14, &[140, 141], &[140, 141]),
        record(15, 7, -1, &[150, 151], &[150, 160]),
    ]
}

/// Orbit 7 of the test with every piece solved.
fn orbit_seven_solved() -> Vec<Value> {
    (10..16)
        .map(|piece| record(piece, 7, piece, &[piece * 10, piece * 10 + 1], &[piece * 10, piece * 10 + 1]))
        .collect()
}

fn run(entry: &str, args: Vec<Value>) -> Value {
    let loaded = load(&lower_source(ORBITS_SOURCE).unwrap()).unwrap();
    let run = loaded.run(ORBITS_MODULE, entry, args, &mut NoHost, &Options::new(WIDE)).unwrap();
    assert_eq!(run.summary.outcome, Outcome::Completed, "{entry}: {:?}", run.summary.code);
    run.value.expect("an answer")
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

fn ints(value: &Value) -> Vec<i64> {
    let Value::List(items) = value else { panic!("a list: {value:?}") };
    items.items().iter().map(int).collect()
}

fn boolean(value: &Value) -> bool {
    let Value::Bool(b) = value else { panic!("a Bool: {value:?}") };
    *b
}

/// `orbit_progress` on the caller's records: (pieces, solved, orientation wrong, position wrong, others).
fn progress(orbit: i64, records: Vec<Value>) -> [i64; 5] {
    let answer = run("orbit_progress", vec![Value::int(orbit), Value::list(records)]);
    assert_eq!(names(&answer), ["orbit", "orientation_wrong", "others", "pieces", "position_wrong", "solved"]);
    assert_eq!(int(field(&answer, "orbit")), orbit);
    ["pieces", "solved", "orientation_wrong", "position_wrong", "others"].map(|name| int(field(&answer, name)))
}

/// The progress counts the named records of the orbit: a piece with every sticker in its home slot is solved, a piece
/// at Home with a sticker elsewhere is orientation wrong, and any other, a piece in a buffer or with its stickers split
/// over positions among them, is position wrong; `pieces` is their sum. Records of other orbits are counted apart in
/// `others`, never in the orbit's counts, and no records give zeros.
#[test]
fn the_progress_counts_the_named_records_of_the_orbit() {
    assert_eq!(progress(7, orbit_seven()), [6, 2, 1, 3, 0]);
    assert_eq!(progress(7, orbit_seven_solved()), [6, 6, 0, 0, 0]);
    let mut mixed = orbit_seven();
    mixed.insert(2, record(99, 3, 98, &[990], &[980]));
    mixed.push(record(98, 3, 98, &[980], &[980]));
    assert_eq!(progress(7, mixed.clone()), [6, 2, 1, 3, 2], "two records of orbit 3 apart");
    assert_eq!(progress(3, mixed), [2, 1, 0, 1, 6]);
    assert_eq!(progress(7, orbit_seven()[..2].to_vec()), [2, 1, 1, 0, 0], "what the named records show");
    assert_eq!(progress(7, vec![]), [0, 0, 0, 0, 0]);
}

/// The residuals of the caller's records: (complete, solved, misplaced, misoriented, unknown, buffer A's occupant and
/// whether it is solved, buffer B's occupant and whether it is solved).
type Residuals = (bool, bool, Vec<i64>, Vec<i64>, Vec<i64>, (i64, bool), (i64, bool));

fn residuals(orbit: i64, pieces: &[i64], buffers: [i64; 2], records: Vec<Value>) -> Residuals {
    let members = Value::record(vec![
        ("orbit", Value::int(orbit)),
        ("pieces", Value::list(pieces.iter().map(|&piece| Value::int(piece)).collect())),
    ]);
    let args = vec![members, Value::int(buffers[0]), Value::int(buffers[1]), Value::list(records)];
    let answer = run("orbit_residuals", args);
    assert_eq!(
        names(&answer),
        ["buffer_a", "buffer_b", "complete", "misoriented", "misplaced", "orbit", "solved", "unknown"]
    );
    assert_eq!(int(field(&answer, "orbit")), orbit);
    let buffer = |name: &str, position: i64| {
        let buffer = field(&answer, name);
        assert_eq!(names(buffer), ["occupant", "position", "solved"]);
        assert_eq!(int(field(buffer, "position")), position, "{name} as named");
        (int(field(buffer, "occupant")), boolean(field(buffer, "solved")))
    };
    (
        boolean(field(&answer, "complete")),
        boolean(field(&answer, "solved")),
        ints(field(&answer, "misplaced")),
        ints(field(&answer, "misoriented")),
        ints(field(&answer, "unknown")),
        buffer("buffer_a", buffers[0]),
        buffer("buffer_b", buffers[1]),
    )
}

/// The residuals list what the named records show, in the caller's order. The non-buffer pieces not at Home are
/// misplaced, a non-buffer piece in a buffer and one with its stickers split among them; buffer A's own piece away
/// from Home is not, and shows as A's occupant, the piece that is there. A non-buffer piece at Home with a sticker
/// elsewhere is misoriented; a buffer's own piece at Home with a sticker elsewhere is not, and leaves its buffer
/// unsolved. A piece with its stickers split is unknown, buffer pieces too, and occupies no buffer: a buffer no named
/// record is at has occupant -1.
#[test]
fn the_residuals_list_what_the_named_records_show() {
    let pieces = [10, 11, 12, 13, 14, 15];
    let answer = residuals(7, &pieces, [13, 14], orbit_seven());
    assert_eq!(answer, (true, false, vec![12, 15], vec![11], vec![15], (12, false), (14, true)));

    let mut records = orbit_seven_solved();
    records[4] = record(14, 7, 14, &[140, 141], &[141, 140]);
    records[3] = record(13, 7, -1, &[130, 131], &[130, 200]);
    let answer = residuals(7, &pieces, [13, 14], records);
    assert_eq!(answer, (true, false, vec![], vec![], vec![13], (-1, false), (14, false)));

    let answer = residuals(7, &pieces, [10, 15], orbit_seven());
    assert_eq!(answer.2, vec![12, 13], "with buffers 10 and 15: 15 is a buffer piece, 13 is not");
    assert_eq!((answer.5, answer.6), ((10, true), (-1, false)));
}

/// Solved is the exact status of the whole orbit: every member named, once, in the catalogue's order, all of the
/// orbit, with every sticker in its home slot. A partial, empty, reordered or repeated list, or a record of another
/// orbit among them, is not complete and never solved, whatever its records show.
#[test]
fn solved_needs_every_member_with_every_sticker_home() {
    let pieces = [10, 11, 12, 13, 14, 15];
    let solved = orbit_seven_solved();
    assert_eq!(
        residuals(7, &pieces, [13, 14], solved.clone()),
        (true, true, vec![], vec![], vec![], (13, true), (14, true))
    );

    let not_solved = |answer: Residuals| (answer.0, answer.1);
    assert_eq!(not_solved(residuals(7, &pieces, [13, 14], solved[..5].to_vec())), (false, false), "partial");
    assert_eq!(not_solved(residuals(7, &pieces[..5], [13, 14], solved.clone())), (false, false), "longer");
    assert_eq!(not_solved(residuals(7, &[], [13, 14], vec![])), (false, false), "empty");
    let mut reordered = solved.clone();
    reordered.swap(0, 1);
    assert_eq!(not_solved(residuals(7, &pieces, [13, 14], reordered)), (false, false), "reordered");
    let mut repeated = solved.clone();
    repeated[1] = repeated[0].clone();
    assert_eq!(not_solved(residuals(7, &pieces, [13, 14], repeated)), (false, false), "repeated");
    let mut other = solved.clone();
    other[5] = record(15, 3, 15, &[150, 151], &[150, 151]);
    let answer = residuals(7, &pieces, [13, 14], other);
    assert_eq!(answer, (false, false, vec![], vec![], vec![], (13, true), (14, true)), "a record of orbit 3");
    assert_eq!(not_solved(residuals(7, &pieces, [13, 14], orbit_seven())), (true, false), "complete, not solved");
}

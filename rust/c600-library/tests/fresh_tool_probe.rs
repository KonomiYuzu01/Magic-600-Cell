//! A tool the plan does not list, written from the language docs and the base library only, as a check of the
//! language's reach (owner request, 21 November 2026); not a CT test.
//!
//! The tool is `cell_progress` (`fixtures/fresh_cell_progress.6cs`): a read-only report of which cells are closest to
//! solved. For each cell it reads the labels of the cell's slots (a slot lies in cell `slot / 433`) and counts the
//! slots that hold their own sticker; it returns the cells with the most such stickers, highest count first and the
//! lower cell first among equals, and the number of cells that are wholly home. The test serves the `labels` row from
//! the kernel's state, as the bridge does, and computes the same report in Rust straight from the state's labels.

mod common;

use std::collections::BTreeSet;

use c600_check::{check, separation, Policy, Row, Separation, StateRead};
use c600_ir::Effect;
use c600_kernel::Snapshot;
use c600_library::lower_source;
use c600_run::interp::{load, Host, Options, Request, RowTariff, Sink};
use c600_run::meter::Outcome;
use c600_run::provenance::{parse, Json};
use c600_run::value::Value;
use c600_syntax::{format, parse as parse_source};
use common::{kernel, scrambled, WIDE};

const SOURCE: &str = include_str!("fixtures/fresh_cell_progress.6cs");
const MODULE: &str = "cell_progress";
const CELLS: usize = 600;
const PER_CELL: usize = 433;
const NOT_GRANTED: &str = "LG3:capability.not_granted";

/// A host that serves only the `labels` row, from a state, and counts the reads it answers.
struct LabelsHost<'s> {
    state: &'s Snapshot,
    reads: usize,
}

fn named(arg: &[u8]) -> Vec<usize> {
    let Some(Json::List(items)) = parse(arg) else { panic!("a list argument") };
    items
        .iter()
        .map(|item| {
            let Json::Object(fields) = item else { panic!("an integer object") };
            fields.get("int").and_then(Json::as_text).expect("an int").parse().expect("a slot")
        })
        .collect()
}

impl Host for LabelsHost<'_> {
    fn tariff(&mut self, request: &Request) -> Result<RowTariff, &'static str> {
        match request {
            Request::SnapshotWith { query: "labels", arg, .. } => {
                let n = named(arg).len() as u128;
                Ok(RowTariff { kernel_cost: n, result_bound: 1 << 20 })
            }
            _ => Err(NOT_GRANTED),
        }
    }

    fn perform(&mut self, request: &Request) -> Result<Value, &'static str> {
        let Request::SnapshotWith { query: "labels", arg, .. } = request else { return Err(NOT_GRANTED) };
        self.reads += 1;
        let labels = self.state.labels();
        Ok(Value::list(named(arg).into_iter().map(|slot| Value::int(i64::from(labels[slot]))).collect()))
    }

    fn sink(&mut self, _: &Sink, _: &Value, _: &[u8]) -> Result<(), &'static str> {
        Err(NOT_GRANTED)
    }
}

fn field<'v>(value: &'v Value, name: &str) -> &'v Value {
    let Value::Record(record) = value else { panic!("a record: {value:?}") };
    record.get(name).unwrap_or_else(|| panic!("a field {name}: {value:?}"))
}

fn int(value: &Value) -> usize {
    let Value::Int(n) = value else { panic!("an integer: {value:?}") };
    n.to_string().parse().expect("a usize")
}

/// The report as the program answers it: (cells, per cell, solved, closest as (cell, home) pairs).
type Report = (usize, usize, usize, Vec<(usize, usize)>);

/// Runs `closest_cells(cells, per_cell, keep)` on `state`; returns the report and the reads the host answered.
fn run_tool(state: &Snapshot, cells: usize, per_cell: usize, keep: usize) -> (Report, usize) {
    let program = lower_source(SOURCE).expect("the source lowers");
    let loaded = load(&program).expect("the program loads");
    let mut host = LabelsHost { state, reads: 0 };
    let args = vec![Value::int(cells as i64), Value::int(per_cell as i64), Value::int(keep as i64)];
    // A quiet run: the default keeps one row per operation, which grows past 16 GB for the larger run below.
    let options = Options { record: false, ..Options::new(WIDE) };
    let run = loaded.run(MODULE, "closest_cells", args, &mut host, &options).expect("the run starts");
    assert_eq!(run.summary.outcome, Outcome::Completed, "{:?}", run.summary.code);
    let answer = run.value.expect("an answer");
    let Value::Record(record) = &answer else { panic!("a record") };
    let names: Vec<&str> = record.fields().iter().map(|(name, _)| name.as_str()).collect();
    assert_eq!(names, ["cells", "closest", "per_cell", "solved"]);
    let Value::List(items) = field(&answer, "closest") else { panic!("a list") };
    let closest = items.items().iter().map(|row| (int(field(row, "cell")), int(field(row, "home")))).collect();
    let report =
        (int(field(&answer, "cells")), int(field(&answer, "per_cell")), int(field(&answer, "solved")), closest);
    (report, host.reads)
}

/// The same report in Rust, straight from the state's labels: a slot holds its own sticker when its label is the slot.
fn oracle(state: &Snapshot, cells: usize, per_cell: usize, keep: usize) -> Report {
    let labels = state.labels();
    let counts: Vec<usize> = (0..cells)
        .map(|cell| (cell * per_cell..(cell + 1) * per_cell).filter(|&slot| labels[slot] as usize == slot).count())
        .collect();
    let mut order: Vec<usize> = (0..cells).collect();
    order.sort_by_key(|&cell| (std::cmp::Reverse(counts[cell]), cell));
    let closest = order.into_iter().take(keep).map(|cell| (cell, counts[cell])).collect();
    (cells, per_cell, counts.iter().filter(|&&count| count == per_cell).count(), closest)
}

/// The source is in the canonical layout, checks with no diagnostic, requests no capability, and has the effect
/// `snapshot` and reads the `labels` row and no other: under capability separation it reads state and prepares no
/// preview.
#[test]
fn the_tool_checks_as_a_read_only_report() {
    let file = parse_source(SOURCE.as_bytes());
    assert!(file.diagnostics.is_empty() && file.holes.is_empty(), "{:#?}", file.diagnostics);
    assert_eq!(format(&file).as_deref(), Some(SOURCE), "the source is in the formatter's layout");

    let program = lower_source(SOURCE).expect("the source lowers");
    let report = check(&program).expect("well-formed IR");
    assert!(report.diagnostics.is_empty(), "{:#?}", report.diagnostics);
    let [module] = &program.modules[..] else { panic!("one module") };
    assert!(module.capabilities.is_empty());
    let rows = BTreeSet::from([Row::Read("labels".to_string())]);
    for entry in ["closest_cells", "cell_home_count"] {
        let summary = report.summary(MODULE, entry).expect("a summary");
        assert_eq!(summary.effects, BTreeSet::from([Effect::ReadSnapshot]), "{entry}");
        assert_eq!(summary.state_reads, BTreeSet::from([StateRead::Snapshot]), "{entry}");
        assert_eq!(summary.rows, rows, "{entry}");
        assert!(summary.capabilities.is_empty() && summary.vars.is_empty() && summary.unresolved.is_empty());
        assert_eq!(separation(summary, &Policy::interim()), Separation { reads_state: true, prepares_previews: false });
    }
}

/// On the solved state every cell is wholly home, so the closest five are cells 0 to 4 by the tie rule; on a scramble
/// the program and the Rust oracle give the same cells and counts, and the program reads once per cell.
#[test]
fn the_report_agrees_with_the_rust_oracle() {
    let kernel = kernel();
    let solved = kernel.solved();
    let (report, reads) = run_tool(&solved, CELLS, PER_CELL, 5);
    assert_eq!(report, oracle(&solved, CELLS, PER_CELL, 5));
    assert_eq!(report, (CELLS, PER_CELL, CELLS, (0..5).map(|cell| (cell, PER_CELL)).collect()));
    assert_eq!(reads, CELLS);

    // The scramble leaves 84 cells wholly home, so asking for 90 reaches six cells that are only partly home: the
    // counts and the tie rule are then tested on unequal counts too.
    let state = scrambled();
    let (report, reads) = run_tool(&state, CELLS, PER_CELL, 90);
    assert_eq!(report, oracle(&state, CELLS, PER_CELL, 90));
    assert_eq!(reads, CELLS);
    assert_eq!(report.2, 84, "wholly home cells");
    assert_eq!(report.3.len(), 90);
    assert!(report.3[83].1 == PER_CELL && report.3[84].1 < PER_CELL && report.3[89].1 <= report.3[84].1);
    assert!(report.3.windows(2).all(|pair| pair[0].1 > pair[1].1 || (pair[0].1 == pair[1].1 && pair[0].0 < pair[1].0)));
    assert_eq!(report.3[..5], oracle(&state, CELLS, PER_CELL, 5).3[..], "the default five");
}

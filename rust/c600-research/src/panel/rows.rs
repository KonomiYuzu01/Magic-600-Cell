//! The eleven rows of the LP5 plan's section 6, LP5's own block of the command table, and the table a research host
//! serves them from.

use c600_command::{Availability, Binding, CommandLayer, Kind, Lane, Store, Table, UndoClass};

/// The overlay module that serves the rows; each row's entry is its own ID.
pub const MODULE: &str = "research.panel";

/// LP5's block of the command table, in the H2 delta format (`c600-command-rows-delta-1`): H-08's fifth delta, which
/// `docs/progress/1.0/build_command_table.py` applies after the commit delta.
pub const DELTA: &str = include_str!("../../../../docs/progress/1.0/command-rows-research.json");

pub const INVENTORY: &str = "research.archive.inventory";
pub const CLAIMS: &str = "research.claims";
pub const REPORT: &str = "research.check.report";
pub const WORD: &str = "evidence.word";
pub const RUNS: &str = "research.runs";
pub const OBLIGATIONS: &str = "research.obligations";
pub const IMPORT: &str = "research.archive.import";
pub const CHECK: &str = "research.check.run";
pub const REPRODUCE: &str = "research.run.reproduce";
pub const BUNDLE: &str = "research.bundle.export";
pub const OBLIGATION: &str = "research.obligation.export";

/// The H-08 rows the panel invokes and the research host does not serve: the branch the panel shows, and a running
/// job's stop.
pub const H08_ROWS: [&str; 2] = ["session.branch.select", "job.stop"];

/// One row of the block as the host serves it: its kind, lane and undo class, and its arguments with their H-08
/// types. Every row takes no token, prepares no preview and has no gate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PanelRow {
    pub id: &'static str,
    pub kind: Kind,
    pub lane: Lane,
    pub undo: UndoClass,
    pub arguments: &'static [(&'static str, &'static str)],
}

const fn query(id: &'static str, arguments: &'static [(&'static str, &'static str)]) -> PanelRow {
    PanelRow { id, kind: Kind::Query, lane: Lane::None, undo: UndoClass::None, arguments }
}

const fn job(id: &'static str, kind: Kind, arguments: &'static [(&'static str, &'static str)]) -> PanelRow {
    PanelRow { id, kind, lane: Lane::Program, undo: UndoClass::None, arguments }
}

/// The block's rows: the six reads, then the five rows that import, check, reproduce and export.
pub const ROWS: [PanelRow; 11] = [
    query(INVENTORY, &[("manifest", "Text?"), ("stage", "Nat?")]),
    query(CLAIMS, &[("group", "Text?"), ("stage", "Nat?")]),
    query(REPORT, &[("check", "Nat"), ("run", "Text")]),
    query(WORD, &[("group", "Text")]),
    query(RUNS, &[("group", "Text?")]),
    query(OBLIGATIONS, &[("group", "Text?")]),
    job(IMPORT, Kind::File, &[("manifest", "Text"), ("members", "List<Text>")]),
    job(
        CHECK,
        Kind::Workspace,
        &[
            ("certificate", "Text"),
            ("checker", "Text"),
            ("digest", "Text"),
            ("group", "Text?"),
            ("inputs", "Map<Text,Text>"),
            ("level", "labels|poses"),
            ("puzzle", "full|jumbling"),
        ],
    ),
    job(REPRODUCE, Kind::Workspace, &[("run", "Text")]),
    job(BUNDLE, Kind::File, &[("destination", "Text"), ("records", "List<Text>")]),
    job(OBLIGATION, Kind::File, &[("destination", "Text"), ("statement", "Text")]),
];

/// The command table a research host serves the rows from: H-08 with the command layer's draft rows
/// ([`Table::with_drafts`]). H-08 holds [`DELTA`] as its fifth delta (owner, 23 November 2026, item 3.1), so every row
/// of the block is read from H-08 as it stores it and none is added here a second time; a stored table without one
/// of them is a build error, and this panics.
pub fn table() -> Table {
    let table = Table::h08().with_drafts();
    if let Some(row) = ROWS.iter().find(|row| table.row(row.id).is_none()) {
        panic!("the stored H-08 table does not hold LP5's row `{}`", row.id);
    }
    table
}

/// KN5.6's availability table for the panel's controls: every row of the block and the H-08 rows it invokes, bound as
/// computed arguments, since a control's arguments are the panel's selection when it is pressed. A palette reads it
/// once per generation and answers each keystroke from it with no engine call; it grants nothing.
pub fn palette<S: Store>(layer: &CommandLayer<S>) -> Availability {
    let computed = Binding::Computed;
    let ids = ROWS.iter().map(|row| row.id).chain(H08_ROWS);
    layer.availability_table(ids.map(|id| (id, &computed)).collect::<Vec<_>>())
}

//! What each of the tool's six rows needs from the command layer, what the layer meets of it and what remains (LP3
//! plan, C6 and C5's item 1). KN5 has one implementation, and LP3 adds no token, receipt, undo or outcome logic of its
//! own. LP2 built the four common needs in the layer (pull requests 200, 202 and 203) and `filter.apply`'s gate,
//! preview rule and workspace inverse (pull request 256), so a host serves all six rows with the entries named here
//! ([`RowNeed::served`]). The table below is a requirement list, checked against the stored H-08 table, the layer's
//! `serve_row` and the shipped entries by this crate's tests, which also run the six served rows through the command
//! and source routes (`tests/served_rows.rs`); it runs nothing itself.

use c600_command::{CompletionKind, Kind, Lane, PreviewRule, UndoClass};
use c600_ir::Effect;

/// One row's requirements, what the command layer meets of them and what remains.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RowNeed {
    pub row: &'static str,
    /// The tool's entry for the row, where the IR draft can express one.
    pub entry: Option<&'static str>,
    /// The row as H-08 stores it: its kind, lane, undo class, preview rule, gates and arguments with their types.
    pub kind: Kind,
    pub lane: Lane,
    pub undo: UndoClass,
    pub preview: PreviewRule,
    pub gates: &'static [&'static str],
    pub arguments: &'static [(&'static str, &'static str)],
    /// The effect class: LG3.1's effect families the row's entry has, in their order there.
    pub effects: &'static [Effect],
    /// The receipt a finished invocation ends in: `completed` with this kind.
    pub completion: CompletionKind,
    /// What the receipt must carry and when the invocation is refused instead.
    pub receipt: &'static str,
    /// The undo entry the row leaves.
    pub undo_entry: &'static str,
    /// Whether the layer serves the row with its entry (`CommandLayer::serve_row`).
    pub served: bool,
    /// What the layer meets of the row's requirements today.
    pub met: &'static str,
    /// What the row still needs, in the layer or elsewhere, before it meets every requirement above.
    pub remains: &'static str,
}

/// A need every one of the six rows has, and how the command layer meets it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Common {
    pub need: &'static str,
    pub met: &'static str,
}

/// What every one of the six rows needs first, before its own entry in [`NEEDS`], and what LP2 built for it.
pub const COMMON: [Common; 4] = [
    Common {
        need: "A dispatch for a stored row that a built-in tool's entry serves, so that an invocation of the row \
               reaches the entry.",
        met: "`CommandLayer::serve_row` (pull request 200): an invocation of a served row admits a program job for \
              the entry, and the host ends the job with `finish_program_job_with`. The layer serves only a view, \
              workspace, query or file row on the `none`, `view` or `program` lane with no gate, preview rule, action \
              guard, token or undo entry, which is five of the six rows; since pull request 256 it serves \
              `filter.apply` too, with its gate, preview rule and workspace undo entry. Serving is not journaled: a \
              host serves its rows again at every open.",
    },
    Common {
        need: "The source route's other end: the runtime's view and workspace sinks delivered to the layer as the \
               program job's result, so that an entry's `publish_view` and `write_workspace` become the row's receipt.",
        met: "`JobOutput` and `finish_program_job_with` (pull request 203): the value, views and records a run gave \
              complete the job, with their digest as the result reference. Only a workspace row's job writes records, \
              each through the host's provenance reader, in the completion's one durable action.",
    },
    Common {
        need: "Argument forms for the H-08 types these rows take (FilterRef, RevisionRef, FilterPreviewRef, \
               CanonicalObjectRefs, CellIdPair, CellId), checked before a job is admitted.",
        met: "The layer's argument forms (pull requests 202, 256 and 269) check `Nat`, `Text`, the choice words, \
              `CellId`, `CellIdPair`, `RevisionRef`, `FilterRef`, `FilterPreviewRef` and `CanonicalObjectRefs` before \
              it admits a job, and refuse a revision that is not the current one as stale. The IR draft has no \
              identity or reference type, so the entries still take naturals, a list of naturals, text and, for a \
              `FilterPreviewRef`, a record of two naturals in their place.",
    },
    Common {
        need: "A workspace revision that moves.",
        met: "A workspace completion advances the workspace number by one and leaves the state number, head and state \
              digest as they were (pull request 200).",
    },
];

/// The six rows.
pub const NEEDS: [RowNeed; 6] = [
    RowNeed {
        row: "filter.preview",
        entry: Some("filter_counts"),
        kind: Kind::Query,
        lane: Lane::None,
        undo: UndoClass::None,
        preview: PreviewRule::None,
        gates: &[],
        arguments: &[("predicate", "FilterRef"), ("revision", "RevisionRef")],
        effects: &[Effect::ReadSnapshot, Effect::ChangeView],
        completion: CompletionKind::Query,
        receipt: "completed(query) at the revision the `revision` argument names, with a result reference to the \
                  counts; refused as stale when that revision is not the current one. The result is the filter \
                  preview that `filter.apply` names, so the layer keeps it with its dependencies: epoch, identity, \
                  mode, state, protection and the predicate's digest.",
        undo_entry: "none: a query changes nothing.",
        served: true,
        met: "Served by `filter_counts`, which passes the layer predicate to its snapshot read `filter.preview`. C5's \
              host bridge answers the read from LP2's `CommandLayer::layer_counts` (pull request 262: the positions \
              and slots of one layer around a cell) with `c600_tools::kernel_rows::filter_preview`, and the tests' \
              host does the same. The layer checks the form of `revision` and refuses one that is not the current \
              revision as stale before any job; the job completes as `completed(query)` at its revision with the \
              output's digest as the result reference, and nothing is written.",
        remains: "The read and its tariff are drafts until the command table freezes at LP3. The entry takes the \
                  predicate as a record of its cell and layer, which the host reads from the record the predicate's \
                  job wrote, since the IR draft has no reference type. The `FilterRef` form and the store of filter \
                  previews are the layer's since pull request 256.",
    },
    RowNeed {
        row: "filter.apply",
        entry: Some("apply_filter"),
        kind: Kind::Workspace,
        lane: Lane::None,
        undo: UndoClass::Workspace,
        preview: PreviewRule::Filter,
        gates: &["fresh_filter_preview"],
        arguments: &[("preview", "FilterPreviewRef")],
        effects: &[Effect::ChangeWorkspace],
        completion: CompletionKind::Workspace,
        receipt: "completed(workspace) at a revision whose workspace number is one higher and whose state number, \
                  head and state digest are unchanged; refused under the gate `fresh_filter_preview` when the named \
                  filter preview is missing or its dependencies differ from the current ones.",
        undo_entry: "one workspace action in the durable action order, holding the filter before and after, which \
                     `session.undo` dispatches as a workspace action and which leaves the journal's state actions and \
                     the slot map as they were.",
        served: true,
        met: "The layer serves the row (LP2's filter previews, `c600-command`'s `filter` module). It checks the \
              `FilterPreviewRef` form, a job and the epoch it ran in, and keeps a filter preview for each completed \
              `filter.preview` job with KN0.9's dependencies: the epoch, identity, mode and state, and the digest of \
              the resolved filter, which holds each record the predicate's job wrote with its content digest and \
              version. The gate \
              `fresh_filter_preview` refuses a missing or stale filter preview with `KN5:stale.filter_preview` before \
              any job, reading neither the word-preview slot nor a presentation, and fails the job with that code, \
              writing nothing, when the filter preview went stale while the job ran. The completion is one workspace \
              action that `session.undo` and `session.redo` invert and repeat as workspace actions. Served by \
              `apply_filter`, which takes the `FilterPreviewRef` as a record of its epoch and job and writes it as the \
              record `filter.apply`; in the tests the row runs through the command and source routes, and the next \
              undo removes the record.",
        remains: "Nothing in the layer for the row's admission, gate, completion or undo. No display applies the \
                  applied filter yet.",
    },
    RowNeed {
        row: "view.effect.scope",
        entry: Some("effect_scope"),
        kind: Kind::View,
        lane: Lane::View,
        undo: UndoClass::None,
        preview: PreviewRule::None,
        gates: &[],
        arguments: &[("scope", "body|complete")],
        effects: &[Effect::ChangeView],
        completion: CompletionKind::ViewPublished,
        receipt: "completed(view_published) with the revision unchanged; a newer invocation in the row's view lane \
                  supersedes a waiting one; a `scope` other than `body` or `complete` is refused as an argument form.",
        undo_entry: "none: the row changes what is shown and no workspace or state revision.",
        served: true,
        met: "Served by `effect_scope`. The layer checks the `body|complete` form and keeps the row's `view` lane, \
              where a newer invocation supersedes a waiting one; the job completes as `completed(view_published)` \
              with the revision unchanged. The published scope is the job's view, which the layer takes only into the \
              result reference's digest.",
        remains: "Nothing in the layer. No display applies the published scope yet.",
    },
    RowNeed {
        row: "set.save",
        entry: Some("save_set"),
        kind: Kind::Workspace,
        lane: Lane::None,
        undo: UndoClass::None,
        preview: PreviewRule::None,
        gates: &[],
        arguments: &[("kind", "identity|position"), ("members", "CanonicalObjectRefs"), ("name", "Text")],
        effects: &[Effect::ChangeWorkspace],
        completion: CompletionKind::Workspace,
        receipt: "completed(workspace) at a revision whose workspace number is one higher, with a result reference to \
                  the saved set's exact-set digest; refused as an argument form when a member is no canonical object \
                  of the model or `kind` is neither value, and with the draft code \
                  `c600_command::code::SAVED_SET_EXISTS` when the name holds a saved set and the invocation does not \
                  carry `replace: true` (U1-057).",
        undo_entry: "none: H-08 gives the row the undo class `none`, so a saved set is removed by its own row, not by \
                     undo.",
        served: true,
        met: "Served by `save_set`. The layer checks `kind`, `members` (pull request 269: pieces or positions, each \
              below the kernel's piece count) and `name`; the job completes as one durable workspace action at a \
              revision whose workspace number is one higher, with the exact-set digest as the result reference and \
              no undo entry. In place of the entry's record `set.save` the action writes the saved set's own record \
              `set/<name>` through the host's provenance reader: its kind, its members ascending and distinct, the \
              model identity, the name and the capture revision (pull request 337, `c600_command::sets`). A taken \
              name is refused unless the invocation carries `replace: true`.",
        remains: "Nothing in the layer. The argument `replace` and the code `SAVED_SET_EXISTS` are drafts until the \
                  command table freezes at LP3.",
    },
    RowNeed {
        row: "structure.compare",
        entry: Some("compare_cells"),
        kind: Kind::Query,
        lane: Lane::None,
        undo: UndoClass::None,
        preview: PreviewRule::None,
        gates: &[],
        arguments: &[("cells", "CellIdPair")],
        effects: &[Effect::ReadSnapshot, Effect::ChangeView],
        completion: CompletionKind::Query,
        receipt: "completed(query) at the current revision with a result reference to the comparison; refused as an \
                  argument form when either cell is not a cell of the model.",
        undo_entry: "none: a query changes nothing.",
        served: true,
        met: "Served by `compare_cells`, which passes the two cells to its snapshot read `structure.compare`. C5's \
              host bridge answers the read from LP2's `CommandLayer::compare_cells` (pull request 262) with \
              `c600_tools::kernel_rows::structure_compare`, and the tests' host does the same. The layer checks the \
              `CellIdPair` form; the job completes as `completed(query)` at its revision with the output's digest as \
              the result reference, and nothing is written. The entry's answer is the job's view.",
        remains: "Nothing in the layer. The read and its tariff are drafts until the command table freezes at LP3.",
    },
    RowNeed {
        row: "structure.layer",
        entry: Some("layer_predicate"),
        kind: Kind::Workspace,
        lane: Lane::None,
        undo: UndoClass::None,
        preview: PreviewRule::None,
        gates: &[],
        arguments: &[("cell", "CellId"), ("layer", "Nat")],
        effects: &[Effect::ChangeWorkspace],
        completion: CompletionKind::Workspace,
        receipt: "completed(workspace) at a revision whose workspace number is one higher, with a result reference to \
                  the prepared predicate, which `filter.preview` then takes as its `predicate`; refused as an \
                  argument form when the cell is not a cell of the model.",
        undo_entry: "none: H-08 gives the row the undo class `none`; the prepared predicate is a draft in the \
                     workspace that the next one replaces.",
        served: true,
        met: "Served by `layer_predicate`. The layer checks the `CellId` and `Nat` forms; the job completes as one \
              durable workspace action at a revision whose workspace number is one higher, writing the record \
              `structure.layer` through the host's provenance reader, with the output's digest as the result reference \
              and no undo entry.",
        remains: "Nothing in the layer since pull requests 256 and 262: `filter.preview` takes this row's result \
                  reference as a `FilterRef`, and the layer counts the named layer with `CommandLayer::layer_counts`, \
                  which C5's host bridge serves to `filter.preview`'s entry (see that row).",
    },
];

//! What each of the tool's six rows needs from the command layer before it can run there (LP3 plan, C6 and C5's
//! item 1). The layer refuses all six as not built, and this crate builds none of them: KN5 has one implementation,
//! and LP3 adds no token, receipt, undo or outcome logic of its own. The table below is a requirement list, checked
//! against the stored H-08 table and the shipped entries by this crate's tests; it runs nothing.

use c600_command::{CompletionKind, Kind, Lane, PreviewRule, UndoClass};
use c600_ir::Effect;

/// One row's requirements.
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
    /// What the layer lacks for the row today.
    pub layer: &'static str,
}

/// What every one of the six rows needs first, before its own entry in [`NEEDS`].
pub const COMMON: [&str; 4] = [
    "A dispatch for a stored row that a built-in tool's entry serves. Today only a registered overlay row reaches a \
     program job, and registering an overlay under a stored row's ID is refused as a conflict, so no route leads from \
     these rows to the tool's entries.",
    "The source route's other end: the runtime's view and workspace sinks delivered to the layer as the program job's \
     result, so that an entry's `publish_view` and `write_workspace` become the row's receipt.",
    "Argument forms for the H-08 types these rows take (FilterRef, RevisionRef, FilterPreviewRef, CanonicalObjectRefs, \
     CellIdPair, CellId): the layer checks argument names only, and the IR draft has no identity, reference or list \
     type, so the entries take naturals and text in their place.",
    "A workspace revision that moves. The layer's revision keeps `workspace` at 0, and a finished workspace job \
     completes at the revision it was admitted at.",
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
        layer: "a store of filter previews by result reference with their dependencies, and the snapshot read of the \
                counts for a predicate",
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
        layer: "the gate's freshness check, a workspace action in the durable action order and its inverse; the \
                layer's undo reads journal actions only",
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
        layer: "the two-value argument check and the view lane keyed by the row's own control",
    },
    RowNeed {
        row: "set.save",
        entry: None,
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
                  of the model or `kind` is neither value.",
        undo_entry: "none: H-08 gives the row the undo class `none`, so a saved set is removed by its own row, not by \
                     undo.",
        layer: "a list argument form for the members and a workspace record of frozen sets; the tool has no entry \
                until the IR has a list type",
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
        layer: "the pair argument form and the snapshot read of the two cells' comparison",
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
        layer: "a workspace record of the prepared predicate and the reference it returns",
    },
];

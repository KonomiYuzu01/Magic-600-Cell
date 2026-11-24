//! Soft disclosure (EV5.8; LQ14, owner, 6 November 2026; DT-M1 Outcome K3; LP4-11; draft): which keys each learning
//! step's goal flow opens, and the concepts each key needs, read against the language learning path
//! (`docs/progress/1.0/glossary/learning-path.json`) and the glossary (`terms.json`).
//!
//! - A key opens at the first learning step whose goal flow needs it: the first step, in the path's order, whose
//!   command routes cite its H-08 row ([`KEYS`]). Its concepts are glossary terms that step, or a step before it on
//!   the path, teaches, or that the path takes from the manual, so no goal meets a closed key or an undisclosed
//!   concept (charter bar M6; CT-EV5-02's flow goals, in `tests/disclosure.rs`).
//! - Disclosure guides and never locks (EV5.8). [`Disclosure`] is the shell's display state: which steps' keys are
//!   open, the solver's saved and reversible "open all" choice, and which newly opened keys are not yet used. The
//!   command layer never reads it: the palette lists every command, every row is available and invocable in every
//!   disclosure state, and "show all keys" ([`Disclosure::shows`]) is a read that changes no disclosure state
//!   (CT-EV5-04, in `tests/disclosure.rs`).
//! - EV5.9's BS3 branch, a competence check on a principle record that gates the capability to prepare solving
//!   previews for that principle, waits for LQ2's option set and its joint setting: nothing here is built for it, and
//!   no learning result grants or withholds a capability.
//!
//! The mathematician reviewed the keys' concepts with the path's steps and the glossary's rows on 24 November 2026
//! (EV5.14); their findings are fixed in the keys below, the path and the glossary.

use std::collections::BTreeSet;

/// The learning path's steps in its order (EV5.6; `learning-path.json`).
pub const STEPS: [&str; 7] = ["S1", "S2", "S3", "S4", "S5", "S6", "S7"];

/// One key of a goal flow: its H-08 row, the step that opens it and the glossary terms it needs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Key {
    pub row: &'static str,
    pub step: &'static str,
    pub concepts: &'static [&'static str],
}

const fn key(row: &'static str, step: &'static str, concepts: &'static [&'static str]) -> Key {
    Key { row, step, concepts }
}

/// Every H-08 row the learning path's goal flows cite, at the first step that cites it, with its concepts. Steps 2 and
/// 5 to 7 cite no H-08 row: their routes are the editor's services, the debugger's and the language's own (EV5.6).
pub const KEYS: [Key; 27] = [
    // Step 1, use tools and read results.
    key("command.find", "S1", &["Command", "Availability"]),
    key("object.inspect", "S1", &["Claim", "Scope", "Evidence words", "Canonical ID", "Position"]),
    key("view.findings", "S1", &["View", "Claim", "Scope", "Standing", "Evidence words"]),
    // An exact per-orbit query that runs no job: no search outcome applies to it.
    key("view.residuals", "S1", &["View", "Orbit", "Residual", "Invariant"]),
    // Check full effect: a job returning the effect with chronological prefix coverage.
    key("operation.check", "S1", &["Complete effect", "Literal word", "Job", "Verdict", "Typed partial result"]),
    // Budget stays to contrast a cancelled job, which reads incomplete, with a budget stop.
    key("job.stop", "S1", &["Job", "Job result", "Incomplete", "Budget"]),
    // Step 3, save layouts and bindings.
    key("keymap.edit", "S3", &["Binding", "Routing class", "Keyboard key", "Workspace undo", "Layout"]),
    key("file.keymap.export", "S3", &["Binding", "Preference"]),
    key("file.keymap.check", "S3", &["Binding", "Routing class", "Digest", "Model identity"]),
    key("file.keymap.apply", "S3", &["Binding", "Workspace undo"]),
    // Step 4, compose recipes and macros (it comes after step 1 only, so its keys need nothing step 3 teaches).
    key("macro.base.open", "S4", &["Macro base", "Macro"]),
    key("macro.select", "S4", &["Macro"]),
    key("macro.add", "S4", &["Macro", "Word", "Recipe", "Work sheet", "Assistance provenance"]),
    key("macro.save", "S4", &["Macro", "Assistance provenance"]),
    key("macro.variant.create", "S4", &["Macro", "Conjugate", "Setup"]),
    key("macro.compare", "S4", &["Macro", "Literal word", "Equality key"]),
    key("operation.new", "S4", &["Recipe", "Setup"]),
    key("operation.phase.edit", "S4", &["Recipe", "Setup", "Star"]),
    key("operation.reuse", "S4", &["Recipe", "Preview", "Revision reference"]),
    key("operation.preview", "S4", &["Preview", "Proposal", "Complete effect"]),
    key("operation.execute", "S4", &["Preview", "User-action token", "Protection", "Net and Strict checks"]),
    key("worksheet.save", "S4", &["Work sheet", "Recipe", "Revision"]),
    key("worksheet.load", "S4", &["Work sheet", "Recipe", "Preview"]),
    key("recording.start", "S4", &["Recording"]),
    key("recording.stop", "S4", &["Recording"]),
    key("recording.play", "S4", &["Recording", "Preview", "User-action token"]),
    // The draft target holds the literal word (EV4.23); the source target's Module comes at step 6.
    key("recording.convert", "S4", &["Recording", "Recipe", "Literal word"]),
];

/// The goal-flow key of `row`, if a learning step cites it. `operation.check`, which steps 1 and 4 cite, opens at
/// step 1.
pub fn key_of(row: &str) -> Option<&'static Key> {
    KEYS.iter().find(|key| key.row == row)
}

/// The shell's disclosure state (EV5.8, K3; draft). It guides what the keyboard and the help show; nothing in the
/// command layer reads it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Disclosure {
    opened: BTreeSet<String>,
    open_all: bool,
    /// Keys opened by a step and used since, which no longer carry the "new" status.
    used: BTreeSet<String>,
}

impl Disclosure {
    /// Nothing opened: only the keys no goal flow cites are open.
    pub fn new() -> Disclosure {
        Disclosure::default()
    }

    /// Opens a step's keys in place, at the first use of its goal flow (K3). `false` for a step the path does not
    /// have, or one already open.
    pub fn open_step(&mut self, step: &str) -> bool {
        STEPS.contains(&step) && self.opened.insert(step.to_owned())
    }

    /// The solver's "open all" choice: saved and reversible (K3).
    pub fn set_open_all(&mut self, open: bool) {
        self.open_all = open;
    }

    pub fn open_all(&self) -> bool {
        self.open_all
    }

    /// The steps opened, in the path's order.
    pub fn opened(&self) -> Vec<&'static str> {
        STEPS.into_iter().filter(|step| self.opened.contains(*step)).collect()
    }

    /// Whether `row`'s key is open: with "open all", when its step is open, or when no goal flow cites it.
    pub fn is_open(&self, row: &str) -> bool {
        self.open_all || key_of(row).is_none_or(|key| self.opened.contains(key.step))
    }

    /// Whether the keyboard shows `row`'s key. "Show all keys" shows every key and changes no disclosure state: it
    /// takes the state by reference (K3).
    pub fn shows(&self, row: &str, show_all: bool) -> bool {
        show_all || self.is_open(row)
    }

    /// Whether `row`'s key carries the "new" status: opened by its step and not used since (K3).
    pub fn is_new(&self, row: &str) -> bool {
        !self.open_all && key_of(row).is_some_and(|key| self.opened.contains(key.step)) && !self.used.contains(row)
    }

    /// Records the first use of an open key, which ends its "new" status.
    pub fn use_key(&mut self, row: &str) {
        if self.is_new(row) {
            self.used.insert(row.to_owned());
        }
    }
}

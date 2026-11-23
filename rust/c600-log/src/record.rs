//! Writing a log action by action, with the kernel computing every state: the recorder a session or a fixture uses to
//! produce a log that compile can check. It holds one state per node it may return to, so it suits fixtures and
//! short sessions; the session store keeps its own states and converts its histories into a [`Log`] (LP4-14).

use std::collections::HashMap;

use c600_kernel::json::Value;
use c600_kernel::{Derivation, Kernel, ProtectedSet, Snapshot, Step};

use crate::log::{
    Commit, Cursor, CursorAction, Interchange, LayoutAction, LayoutEntry, LayoutStep, Log, MoveAction, MoveEntry,
    PartialUndo, Saved, WorkspaceStep,
};
use crate::problem::{Gate, Problem};

/// A log being written.
pub struct Recorder<'k> {
    kernel: &'k Kernel,
    log: Log,
    states: HashMap<String, Snapshot>,
    start: Snapshot,
    head: Option<String>,
    layout_head: Option<String>,
    next: u64,
}

impl<'k> Recorder<'k> {
    /// A log of the loaded model from its solved state.
    pub fn new(kernel: &'k Kernel) -> Recorder<'k> {
        let start = kernel.solved();
        Recorder {
            kernel,
            log: Log::new(kernel.identity(), start.digest()),
            states: HashMap::new(),
            start,
            head: None,
            layout_head: None,
            next: 0,
        }
    }

    /// The state at the move cursor.
    pub fn state(&self) -> &Snapshot {
        self.state_of(self.head.as_deref())
    }

    fn state_of(&self, node: Option<&str>) -> &Snapshot {
        node.and_then(|id| self.states.get(id)).unwrap_or(&self.start)
    }

    /// The node at the move cursor; `None` is the start.
    pub fn head(&self) -> Option<&str> {
        self.head.as_deref()
    }

    /// The log's other parts: view, layout, workspace, documents, macros, Library entries, scripts, preferences.
    pub fn log_mut(&mut self) -> &mut Log {
        &mut self.log
    }

    fn id(&mut self) -> String {
        self.next += 1;
        self.next.to_string()
    }

    fn push(&mut self, time: Option<&str>, action: MoveAction) -> String {
        let id = self.id();
        let node = action.is_node();
        self.log.moves.push(MoveEntry { id: id.clone(), time: time.map(str::to_owned), action });
        if node {
            self.head = Some(id.clone());
        }
        id
    }

    /// Commits a recipe in 0.4's step form at the cursor. A recipe the kernel refuses records nothing.
    pub fn commit(&mut self, recipe: &Value, origin: &str, name: &str, time: Option<&str>) -> Result<String, Problem> {
        let refused = |code: &'static str, message: String| Problem::new(code, Gate::Moves, "recipe", message);
        let normalized = self.kernel.recipe(recipe).map_err(|r| refused(r.code, r.message))?;
        let base = self.state().clone();
        let derivation = self
            .kernel
            .derive(&base, &normalized, &ProtectedSet::none())
            .map_err(|e| refused(crate::problem::code::CHECKPOINT_REPLAY, e.to_string()))?;
        let state = match derivation {
            Derivation::Derived(derived) => derived.state,
            Derivation::Refused(refused_) => return Err(refused(refused_.refusal.code, refused_.refusal.message)),
        };
        let stars = normalized.steps().iter().filter(|step| matches!(step, Step::Star(_))).count() as u64;
        let commit = Commit {
            parent: self.head.clone(),
            origin: origin.to_owned(),
            name: name.to_owned(),
            note: String::new(),
            assistance: "manual".to_owned(),
            recipe: recipe_value(&normalized),
            primitive_count: normalized.primitive_count() as u64,
            stars,
            pre: base.digest().to_owned(),
            post: state.digest().to_owned(),
            interchange: Interchange::None,
        };
        let id = self.push(time, MoveAction::Commit(commit));
        self.states.insert(id.clone(), state);
        Ok(id)
    }

    /// Steps back inside the commit at the cursor so that `keep` of its letters stay (ADR-D01.15). `None`, recording
    /// nothing, when the cursor is not on a commit or a partial undo of one, or `keep` is not below its length.
    pub fn partial_undo(&mut self, keep: u64, time: Option<&str>) -> Option<String> {
        let head = self.head.clone()?;
        let entry = match &self.log.node(&head)?.action {
            MoveAction::Commit(_) => head,
            MoveAction::PartialUndo(undo) => undo.entry.clone(),
            _ => return None,
        };
        let MoveAction::Commit(commit) = &self.log.node(&entry)?.action else { return None };
        if keep >= commit.primitive_count {
            return None;
        }
        let parent = commit.parent.clone();
        let recipe = self.kernel.recipe(&commit.recipe).ok()?;
        let letters = self.kernel.letters(&recipe).ok()?;
        let mut replay = self.kernel.replay(self.state_of(parent.as_deref()), &ProtectedSet::none()).ok()?;
        let kept: Vec<i64> = letters[..keep as usize].iter().map(|&m| i64::from(m)).collect();
        replay.push(&kept).ok()?;
        let Derivation::Derived(derived) = replay.finish() else { return None };
        let undo = PartialUndo { parent, entry, keep, post: derived.state.digest().to_owned() };
        let id = self.push(time, MoveAction::PartialUndo(undo));
        self.states.insert(id.clone(), derived.state);
        Some(id)
    }

    /// Records a puzzle-related workspace action; the state stays.
    pub fn workspace(&mut self, what: &str, value: Value, time: Option<&str>) -> String {
        let step = WorkspaceStep { parent: self.head.clone(), what: what.to_owned(), value };
        let state = self.state().clone();
        let id = self.push(time, MoveAction::Workspace(step));
        self.states.insert(id.clone(), state);
        id
    }

    fn parent_of(&self, node: &str) -> Option<String> {
        self.log.node(node).and_then(|e| e.action.parent()).flatten().map(str::to_owned)
    }

    fn cursor(&mut self, action: CursorAction, to: Option<String>, time: Option<&str>) {
        self.push(time, MoveAction::Cursor(Cursor { action, to: to.clone() }));
        self.head = to;
    }

    /// Moves the cursor one node back. `false`, recording nothing, at the start.
    pub fn undo(&mut self, time: Option<&str>) -> bool {
        let Some(head) = self.head.clone() else { return false };
        let to = self.parent_of(&head);
        self.cursor(CursorAction::Undo, to, time);
        true
    }

    /// Moves the cursor to the node added last after it. `false`, recording nothing, when there is none.
    pub fn redo(&mut self, time: Option<&str>) -> bool {
        let head = self.head.as_deref();
        let child = self.log.moves.iter().rev().find(|e| e.action.parent() == Some(head)).map(|e| e.id.clone());
        let Some(child) = child else { return false };
        self.cursor(CursorAction::Redo, Some(child), time);
        true
    }

    /// `session.branch.select`: moves the cursor to any node, or to the start. `false` for an unknown node or the
    /// cursor's own.
    pub fn select(&mut self, to: Option<&str>, time: Option<&str>) -> bool {
        if to == self.head.as_deref() || to.is_some_and(|id| self.log.node(id).is_none()) {
            return false;
        }
        self.cursor(CursorAction::Select, to.map(str::to_owned), time);
        true
    }

    /// Records a layout step.
    pub fn layout_step(&mut self, what: &str, origin: &str, value: Value, time: Option<&str>) -> String {
        let id = self.id();
        let step =
            LayoutStep { parent: self.layout_head.clone(), what: what.to_owned(), origin: origin.to_owned(), value };
        self.log.layout_history.push(LayoutEntry {
            id: id.clone(),
            time: time.map(str::to_owned),
            action: LayoutAction::Step(step),
        });
        self.layout_head = Some(id.clone());
        id
    }

    /// Moves the layout cursor one step back. `false` at the start.
    pub fn layout_undo(&mut self, time: Option<&str>) -> bool {
        let Some(head) = self.layout_head.clone() else { return false };
        let to = self.log.layout_history.iter().find_map(|e| match &e.action {
            LayoutAction::Step(step) if e.id == head => Some(step.parent.clone()),
            _ => None,
        });
        let Some(to) = to else { return false };
        let id = self.id();
        self.log.layout_history.push(LayoutEntry {
            id,
            time: time.map(str::to_owned),
            action: LayoutAction::Cursor(Cursor { action: CursorAction::Undo, to: to.clone() }),
        });
        self.layout_head = to;
        true
    }

    /// The finished log, saved at both cursors.
    pub fn finish(mut self) -> Log {
        let state = self.state().digest().to_owned();
        self.log.saved = Saved { moves: self.head.clone(), layout: self.layout_head.clone(), state };
        self.log
    }
}

/// A normalized recipe in 0.4's step form: words with their letters, stars with an explicit sign.
pub fn recipe_value(recipe: &c600_kernel::Recipe) -> Value {
    Value::Array(
        recipe
            .steps()
            .iter()
            .map(|step| match step {
                Step::Word(letters) => c600_kernel::json::object([
                    ("kind", Value::String("word".to_owned())),
                    ("moves", Value::Array(letters.iter().map(|&m| Value::Int(i64::from(m))).collect())),
                ]),
                Step::Star(star) => c600_kernel::json::object([
                    ("kind", Value::String("star".to_owned())),
                    ("node", Value::Int(star.node() as i64)),
                    ("orbit", Value::Int(star.orbit() as i64)),
                    ("sign", Value::Int(i64::from(star.sign()))),
                ]),
            })
            .collect(),
    )
}

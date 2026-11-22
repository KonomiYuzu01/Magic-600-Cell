//! ADR-D01.15's partial undo (owner, 22 November 2026, item 4; draft): `session.undo.partial {entry, move, token}`
//! steps back inside the head's journal entry, so that the entry's first `move` literal moves stay applied. It is a
//! live commit with `twist.apply`'s checks, recorded as its own retained move that names the entry and the move, so
//! nothing earlier is rewritten; `session.undo` undoes it as any commit, and a reopen rebuilds its place.

use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use c600_command::layer::{EXPLORE_ROW, PARTIAL_UNDO_ROW};
use c600_command::{
    code, draft_args, Caller, CommandLayer, Direction, HeadlessFrames, MemoryStore, Receipt, Revision, Secret, Shell,
    Status, Table, TransactionKind,
};
use c600_kernel::json::{self, Value};
use c600_kernel::{Kernel, Snapshot};

const SECRET: [u8; 32] = [7; 32];

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

fn word(letters: &[i64]) -> Value {
    json::parse(format!(r#"[{{"kind": "word", "moves": {letters:?}}}]"#).as_bytes()).unwrap()
}

/// A word, then a star, then a word: an entry whose expansion is longer than its steps.
fn macro_entry() -> Value {
    json::parse(
        br#"[{"kind": "word", "moves": [1, 2]}, {"kind": "star", "orbit": 3, "node": 5, "sign": 1},
             {"kind": "word", "moves": [3]}]"#,
    )
    .unwrap()
}

/// The literal letters of `recipe`'s expansion.
fn letters(recipe: &Value) -> Vec<i64> {
    let k = kernel();
    k.letters(&k.recipe(recipe).unwrap()).unwrap().into_iter().map(i64::from).collect()
}

fn code_of(receipt: &Receipt) -> &'static str {
    receipt.status.code().unwrap_or_else(|| panic!("not refused or failed: {:?}", receipt.status))
}

fn committed(receipt: &Receipt) -> bool {
    matches!(receipt.status, Status::Committed(_))
}

struct Fixture {
    layer: CommandLayer,
    shell: Shell,
    frames: HeadlessFrames,
}

/// A session at the solved state that protects `orbits`, on H-08's table.
fn fixture(orbits: &[i64]) -> Fixture {
    let k = kernel();
    let (start, protected) = (k.solved(), k.protected_set(orbits).unwrap());
    let layer =
        CommandLayer::new(k, Table::h08(), Secret::fixed(SECRET), MemoryStore::default(), "s1", start, protected);
    Fixture { layer, shell: Shell::new(Secret::fixed(SECRET), "shell"), frames: HeadlessFrames::new() }
}

/// A revision without its epoch, which a reopen draws afresh (L10).
fn epochless(revision: &Revision) -> Revision {
    Revision { epoch: 0, ..revision.clone() }
}

fn reopen(layer: &CommandLayer) -> Result<CommandLayer, c600_command::Error> {
    CommandLayer::open(kernel(), Table::h08(), Secret::fixed(SECRET), layer.store().clone())
}

impl Fixture {
    /// Previews, presents and commits `recipe` through the command route; returns its action order.
    fn commit(&mut self, recipe: &Value) -> u64 {
        let receipt = self.shell.invoke(&mut self.layer, "operation.preview", draft_args(recipe)).unwrap();
        let Status::Running(id) = receipt.status else { panic!("preview not admitted: {:?}", receipt.status) };
        assert_eq!(self.layer.run_jobs(), 1);
        self.frames.present(&mut self.shell, &mut self.layer, id).unwrap();
        let args = json::object([("preview", Value::Int(id as i64))]);
        let receipt = self.shell.invoke(&mut self.layer, "operation.execute", args).unwrap();
        assert!(committed(&receipt), "{:?}", receipt.status);
        self.last_order()
    }

    fn try_commit(&mut self, recipe: &Value) -> Receipt {
        let receipt = self.shell.invoke(&mut self.layer, "operation.preview", draft_args(recipe)).unwrap();
        let Status::Running(id) = receipt.status else { panic!("preview not admitted: {:?}", receipt.status) };
        assert_eq!(self.layer.run_jobs(), 1);
        self.frames.present(&mut self.shell, &mut self.layer, id).unwrap();
        let args = json::object([("preview", Value::Int(id as i64))]);
        self.shell.invoke(&mut self.layer, "operation.execute", args).unwrap()
    }

    fn partial_args(entry: u64, keep: i64) -> Value {
        json::object([("entry", Value::Int(entry as i64)), ("move", Value::Int(keep))])
    }

    fn partial(&mut self, entry: u64, keep: i64) -> Receipt {
        self.shell.invoke(&mut self.layer, PARTIAL_UNDO_ROW, Self::partial_args(entry, keep)).unwrap()
    }

    fn step(&mut self, direction: Direction) -> Receipt {
        let hint = self.layer.undo_hint(direction).expect("an eligible target");
        let row = if direction == Direction::Undo { "session.undo" } else { "session.redo" };
        self.shell.invoke(&mut self.layer, row, json::object([("hint", hint.to_json())])).unwrap()
    }

    fn protect(&mut self, orbits: &[i64]) -> Receipt {
        let orbits = Value::Array(orbits.iter().map(|&orbit| Value::Int(orbit)).collect());
        self.shell.invoke(&mut self.layer, "protection.orbit.set", json::object([("orbits", orbits)])).unwrap()
    }

    /// Sets the Strict protection policy, with the solver's confirmation.
    fn strict(&mut self) {
        let args = json::object([("confirm", Value::Bool(true)), ("policy", Value::String("strict".into()))]);
        let receipt = self.shell.invoke(&mut self.layer, "protection.policy.set", args).unwrap();
        assert!(receipt.status.code().is_none(), "{:?}", receipt.status);
    }

    fn last_order(&self) -> u64 {
        self.layer.store().log.last().map(|t| t.order).unwrap_or(0)
    }

    fn digest(&self) -> String {
        self.layer.revision().digest.clone()
    }

    /// Nothing changed since `order` and `digest` were read: no action was journaled and the state stayed.
    fn unchanged(&self, order: u64, digest: &str) {
        assert_eq!(self.last_order(), order, "a refusal journaled an action");
        assert_eq!(self.digest(), digest, "a refusal moved the state");
    }
}

/// The state after `moves` from the solved state.
fn state_after(moves: &[i64]) -> Snapshot {
    let k = kernel();
    let recipe = k.recipe(&word(moves)).unwrap();
    match k.derive(&k.solved(), &recipe, &k.protected_set(&[]).unwrap()).unwrap() {
        c600_kernel::Derivation::Derived(d) => d.state,
        c600_kernel::Derivation::Refused(_) => panic!("the word derives"),
    }
}

/// The state digest after committing `recipe` from the solved state, the reference a partial undo must restore.
fn reached(recipe: &Value) -> String {
    let mut g = fixture(&[]);
    g.commit(recipe);
    g.digest()
}

/// ADR-D01.15: the solver steps back inside the head's entry move by move. Each step is its own retained move whose
/// row is `session.undo.partial`, restores exactly the state of the entry's first moves, and leaves every earlier
/// transaction as it was (append-only). A partial undo of every move reaches the entry's start.
#[test]
fn steps_back_inside_the_head_entry_move_by_move() {
    let mut f = fixture(&[]);
    let solved = f.digest();
    let entry = f.commit(&word(&[1, 2, 3, 4]));
    assert_eq!(f.layer.head_entry(), Some((entry, 4)));
    assert_eq!(f.layer.availability(PARTIAL_UNDO_ROW), Ok(()));
    let before: Vec<_> = f.layer.store().log.iter().map(|t| t.to_json()).collect();

    for (keep, prefix) in [(3, &[1, 2, 3][..]), (1, &[1][..])] {
        let receipt = f.partial(entry, keep);
        assert!(committed(&receipt), "move {keep}: {:?}", receipt.status);
        assert_eq!(f.layer.head_entry(), Some((entry, keep as usize)));
        assert_eq!(f.digest(), reached(&word(prefix)), "move {keep} restores the entry's first moves exactly");
    }
    assert!(committed(&f.partial(entry, 0)));
    assert_eq!(f.layer.head_entry(), Some((entry, 0)));
    assert_eq!(f.digest(), solved, "a partial undo of every move reaches the entry's start");
    assert_eq!(f.layer.availability(PARTIAL_UNDO_ROW).unwrap_err().code, code::UNDO_SUPERSEDED);

    let log = &f.layer.store().log;
    let after: Vec<_> = log.iter().take(before.len()).map(|t| t.to_json()).collect();
    assert_eq!(after, before, "nothing earlier is rewritten");
    let rows: Vec<(TransactionKind, &str)> = log.iter().map(|t| (t.kind, t.row.as_str())).collect();
    assert_eq!(
        rows,
        [
            (TransactionKind::Commit, "operation.execute"),
            (TransactionKind::Commit, PARTIAL_UNDO_ROW),
            (TransactionKind::Commit, PARTIAL_UNDO_ROW),
            (TransactionKind::Commit, PARTIAL_UNDO_ROW),
        ]
    );
    // Each partial undo's recipe is the inverses of the moves it undoes, last first.
    assert_eq!(log[1].recipe, word(&[-4]));
    assert_eq!(log[2].recipe, word(&[-3, -2]));
    assert_eq!(log[3].recipe, word(&[-1]));
    let distinct: std::collections::BTreeSet<_> = log[1..].iter().map(|t| t.request.clone()).collect();
    assert_eq!(distinct.len(), 3, "each request names its own move");
}

/// A move is a literal move of the entry's expansion, so a partial undo can stop inside a macro step.
#[test]
fn a_partial_undo_stops_inside_a_macro_step() {
    let entry_recipe = macro_entry();
    let letters = letters(&entry_recipe);
    assert!(letters.len() > 4, "the star expands to several moves");
    let mut f = fixture(&[]);
    let entry = f.commit(&entry_recipe);
    assert_eq!(f.layer.head_entry(), Some((entry, letters.len())));
    let keep = letters.len() - 2;
    assert!(committed(&f.partial(entry, keep as i64)));
    assert_eq!(f.digest(), reached(&word(&letters[..keep])));
    assert!(committed(&f.partial(entry, 3)));
    assert_eq!(f.digest(), reached(&word(&letters[..3])), "inside the star's expansion");
    let g = reopen(&f.layer).unwrap();
    assert_eq!(g.recovery(), None);
    assert_eq!(g.head_entry(), Some((entry, 3)));
    assert_eq!(epochless(g.revision()), epochless(f.layer.revision()));
}

/// `session.undo` undoes a partial undo as any commit and `session.redo` repeats it; a new commit drops the redo and
/// becomes the head's entry.
#[test]
fn session_undo_and_redo_step_a_partial_undo_as_any_commit() {
    let mut f = fixture(&[]);
    let entry = f.commit(&word(&[1, 2, 3]));
    let whole = f.digest();
    assert!(committed(&f.partial(entry, 1)));
    let partial = f.last_order();
    let stepped = f.digest();
    assert_eq!(f.layer.undo_hint(Direction::Undo).map(|h| h.target), Some(partial));

    assert!(committed(&f.step(Direction::Undo)));
    assert_eq!((f.layer.head_entry(), f.digest()), (Some((entry, 3)), whole.clone()));
    assert!(committed(&f.step(Direction::Redo)));
    assert_eq!((f.layer.head_entry(), f.digest()), (Some((entry, 1)), stepped));
    assert!(committed(&f.step(Direction::Undo)));

    let next = f.commit(&word(&[5]));
    assert_eq!(f.layer.undo_hint(Direction::Redo), None, "a new commit drops the redo of the partial undo");
    assert_eq!(f.layer.head_entry(), Some((next, 1)));
    assert_eq!(code_of(&f.partial(entry, 1)), code::UNDO_SUPERSEDED, "the earlier entry is no longer the head's");
}

/// Only the head's entry and its applied moves are targets; every other press is `undo_superseded` and changes
/// nothing. A partial undo is no entry of its own, so its order is no target, and a malformed argument is refused
/// before the token.
#[test]
fn only_the_head_entry_and_its_applied_moves_are_targets() {
    let mut f = fixture(&[]);
    assert_eq!(f.layer.head_entry(), None);
    assert_eq!(f.layer.availability(PARTIAL_UNDO_ROW).unwrap_err().code, code::UNDO_SUPERSEDED);
    assert_eq!(code_of(&f.partial(0, 0)), code::UNDO_SUPERSEDED, "the start is no entry");

    let first = f.commit(&word(&[1]));
    let second = f.commit(&word(&[2, 3]));
    let (order, digest) = (f.last_order(), f.digest());
    for (entry, keep) in [(first, 0), (second, 2), (second, 7), (second + 1, 0)] {
        assert_eq!(code_of(&f.partial(entry, keep)), code::UNDO_SUPERSEDED, "entry {entry}, move {keep}");
        f.unchanged(order, &digest);
    }
    for args in [
        json::object([("entry", Value::Int(second as i64)), ("move", Value::Int(-1))]),
        json::object([("entry", Value::String("e2".into())), ("move", Value::Int(0))]),
    ] {
        let receipt = f.shell.invoke(&mut f.layer, PARTIAL_UNDO_ROW, args).unwrap();
        assert_eq!(code_of(&receipt), code::ARGUMENT_FORM);
        f.unchanged(order, &digest);
    }

    assert!(committed(&f.partial(second, 1)));
    let partial = f.last_order();
    let (order, digest) = (f.last_order(), f.digest());
    assert_eq!(code_of(&f.partial(partial, 0)), code::UNDO_SUPERSEDED, "a partial undo is no entry of its own");
    assert_eq!(code_of(&f.partial(second, 1)), code::UNDO_SUPERSEDED, "move 1 is no longer applied");
    f.unchanged(order, &digest);
    assert!(committed(&f.partial(second, 0)));
    assert_eq!(f.digest(), reached(&word(&[1])));
}

/// Every commit rechecks the protected orbits: a partial undo whose inverse moves a protected orbit is refused under
/// Net and Strict, though the entry moved that orbit before it was protected, and the refusal changes nothing.
#[test]
fn a_partial_undo_rechecks_the_protected_orbits() {
    let mut probe = fixture(&[0]);
    let letter = (1..40)
        .find(|&l| probe.try_commit(&word(&[l])).status.code() == Some(code::ORBIT_WOULD_MOVE))
        .expect("a letter that moves orbit 0");
    let other = letter % 39 + 1;
    for policy in ["net", "strict"] {
        let mut f = fixture(&[]);
        if policy == "strict" {
            f.strict();
        }
        let entry = f.commit(&word(&[other, letter]));
        assert!(f.protect(&[0]).status.code().is_none(), "{policy}");
        let (order, digest) = (f.last_order(), f.digest());
        assert_eq!(f.layer.head_entry(), Some((entry, 2)), "a protection change keeps the head's entry");
        assert_eq!(code_of(&f.partial(entry, 1)), code::ORBIT_WOULD_MOVE, "{policy}");
        f.unchanged(order, &digest);
    }
}

/// The token binds the invocation and the state and protection revisions (LI.6), and the target is judged before the
/// revisions, as an undo's hint is: a press raced by a commit is `undo_superseded`, one raced by another partial
/// undo or a protection-policy change is `revision_changed`, and an edited argument is `token_mismatch`.
#[test]
fn the_token_binds_the_press_and_the_revisions() {
    let mut f = fixture(&[]);
    let entry = f.commit(&word(&[1, 2, 3]));

    let raced = f.shell.prepare(&f.layer, PARTIAL_UNDO_ROW, Fixture::partial_args(entry, 1)).unwrap();
    assert!(committed(&f.partial(entry, 2)));
    let (order, digest) = (f.last_order(), f.digest());
    assert_eq!(code_of(&f.shell.send(&mut f.layer, raced)), code::REVISION_CHANGED);
    f.unchanged(order, &digest);

    let raced = f.shell.prepare(&f.layer, PARTIAL_UNDO_ROW, Fixture::partial_args(entry, 1)).unwrap();
    f.strict();
    let (order, digest) = (f.last_order(), f.digest());
    assert_eq!(code_of(&f.shell.send(&mut f.layer, raced)), code::REVISION_CHANGED);
    f.unchanged(order, &digest);

    let mut edited = f.shell.prepare(&f.layer, PARTIAL_UNDO_ROW, Fixture::partial_args(entry, 1)).unwrap();
    edited.args = Fixture::partial_args(entry, 0);
    assert_eq!(code_of(&f.shell.send(&mut f.layer, edited)), code::TOKEN_MISMATCH);
    f.unchanged(order, &digest);

    let raced = f.shell.prepare(&f.layer, PARTIAL_UNDO_ROW, Fixture::partial_args(entry, 1)).unwrap();
    f.commit(&word(&[4]));
    let (order, digest) = (f.last_order(), f.digest());
    assert_eq!(code_of(&f.shell.send(&mut f.layer, raced)), code::UNDO_SUPERSEDED);
    f.unchanged(order, &digest);
}

/// The explore state pauses the row like every commit row (ADR-D01.39), and the runtime may not invoke it (KN5.15).
#[test]
fn the_explore_state_pauses_it_and_a_run_never_reaches_it() {
    let mut f = fixture(&[]);
    let entry = f.commit(&word(&[1, 2]));
    let (order, digest) = (f.last_order(), f.digest());
    let on = f.shell.invoke(&mut f.layer, EXPLORE_ROW, json::object([("on", Value::Bool(true))])).unwrap();
    assert!(on.status.code().is_none(), "{:?}", on.status);
    assert_eq!(code_of(&f.partial(entry, 1)), code::COMMITS_PAUSED);
    f.unchanged(order, &digest);
    let off = f.shell.invoke(&mut f.layer, EXPLORE_ROW, json::object([("on", Value::Bool(false))])).unwrap();
    assert!(off.status.code().is_none());

    let run = Caller::Runtime { run: "r1".into() };
    let invocation = c600_command::Invocation {
        command: "c1".into(),
        row: PARTIAL_UNDO_ROW.into(),
        args: Fixture::partial_args(entry, 1),
        token: None,
        epoch: f.layer.revision().epoch,
        budget: None,
    };
    assert_eq!(code_of(&f.layer.invoke(&run, invocation)), code::COMMIT_ROW_REFUSED);
    f.unchanged(order, &digest);
    assert!(committed(&f.partial(entry, 1)));
}

/// A reopen rebuilds each node's place from the journal, so the head's entry, the revision and the next partial undo
/// agree with the live layer's; a journal whose partial undo does not step back inside the head's entry does not
/// replay.
#[test]
fn a_reopen_rebuilds_the_places_and_refuses_a_forged_partial_undo() {
    let mut f = fixture(&[]);
    let entry = f.commit(&word(&[1, 2, 3]));
    assert!(committed(&f.partial(entry, 2)));
    assert!(committed(&f.partial(entry, 1)));
    assert!(committed(&f.step(Direction::Undo)));

    let mut g = reopen(&f.layer).unwrap();
    assert_eq!(g.recovery(), None);
    assert_eq!(g.head_entry(), f.layer.head_entry());
    assert_eq!(g.head_entry(), Some((entry, 2)));
    assert_eq!(epochless(g.revision()), epochless(f.layer.revision()));
    for direction in [Direction::Undo, Direction::Redo] {
        assert_eq!(g.undo_hint(direction), f.layer.undo_hint(direction), "{direction:?}");
    }
    let mut shell = Shell::new(Secret::fixed(SECRET), "reopened");
    let receipt = shell.invoke(&mut g, PARTIAL_UNDO_ROW, Fixture::partial_args(entry, 0)).unwrap();
    assert!(committed(&receipt), "{:?}", receipt.status);
    assert!(committed(&f.partial(entry, 0)));
    assert_eq!(g.revision().digest, f.layer.revision().digest);

    // The first partial undo's recipe undoes the entry's first move instead of its last.
    let mut forged = f.layer.store().clone();
    forged.log[1].recipe = word(&[-1]);
    let reopened = CommandLayer::open(kernel(), Table::h08(), Secret::fixed(SECRET), forged).unwrap();
    assert_eq!(reopened.recovery().map(|e| e.code), Some(code::UNADMITTED_JOURNAL_ACTION), "{:?}", reopened.recovery());
    assert_eq!(reopened.head_entry(), Some((entry, 3)), "the replay stops before the forged action");
    // A partial undo whose head is no entry: the start.
    let mut h = fixture(&[]);
    let entry = h.commit(&word(&[1]));
    assert!(committed(&h.partial(entry, 0)));
    let mut forged = h.layer.store().clone();
    forged.log.remove(0);
    forged.log[0].order = 1;
    forged.log[0].event = "e1".into();
    let reopened = CommandLayer::open(kernel(), Table::h08(), Secret::fixed(SECRET), forged).unwrap();
    assert_eq!(reopened.recovery().map(|e| e.code), Some(code::UNADMITTED_JOURNAL_ACTION), "{:?}", reopened.recovery());
    assert_eq!(reopen(&f.layer).unwrap().recovery(), None, "the journal as written replays");
}

/// Guard (attack on row 3): partial undos on two branches, with undo and redo between them, reach the same place and
/// state live and after a reopen, and a commit on top and its undo return to that place.
#[test]
fn partial_undos_on_two_branches_replay() {
    let mut f = fixture(&[]);
    let entry = f.commit(&word(&[1, 2, 3, 4]));
    assert!(committed(&f.partial(entry, 2)));
    assert!(committed(&f.step(Direction::Undo)));
    assert!(committed(&f.partial(entry, 3)));
    assert!(f.layer.undo_hint(Direction::Redo).is_none(), "the new branch drops the redo");
    assert!(committed(&f.partial(entry, 1)));
    assert!(committed(&f.step(Direction::Undo)));
    assert!(committed(&f.step(Direction::Undo)));
    assert!(committed(&f.step(Direction::Redo)));
    assert_eq!(f.layer.head_entry(), Some((entry, 3)));
    assert_eq!(f.digest(), reached(&word(&[1, 2, 3])));
    let g = reopen(&f.layer).unwrap();
    assert_eq!(g.recovery(), None);
    assert_eq!(g.head_entry(), Some((entry, 3)));
    assert_eq!(g.revision().digest, f.digest());
    f.commit(&word(&[7]));
    assert!(committed(&f.step(Direction::Undo)));
    assert_eq!(f.layer.head_entry(), Some((entry, 3)));
    assert!(committed(&f.partial(entry, 0)));
    assert_eq!(f.digest(), kernel().solved().digest().to_owned());
    assert_eq!(reopen(&f.layer).unwrap().recovery(), None);
}

/// Guard (attack on row 3): a partial undo rechecks the head's state against its recorded digest, so a corrupted head
/// refuses it with `state_hash_mismatch` and journals nothing; with the head restored, it commits.
#[test]
fn a_corrupted_head_refuses_a_partial_undo() {
    let mut f = fixture(&[]);
    let entry = f.commit(&word(&[1, 2, 3]));
    f.layer.fault_replace_head(state_after(&[9]));
    let (order, digest) = (f.last_order(), f.digest());
    assert_eq!(code_of(&f.partial(entry, 1)), code::STATE_HASH_MISMATCH);
    f.unchanged(order, &digest);
    f.layer.fault_replace_head(state_after(&[1, 2, 3]));
    assert!(committed(&f.partial(entry, 1)));
    assert_eq!(f.digest(), reached(&word(&[1])));
}

/// Guard (attack on row 3): a partial undo of an entry longer than one word step (two steps of 60,000 moves) commits,
/// restores the prefix, is journaled as word steps and replays.
#[test]
fn a_partial_undo_longer_than_one_word_step() {
    let step: Vec<i64> = (0..60_000).map(|i| (i % 30) as i64 + 1).collect();
    let both = format!(r#"[{{"kind": "word", "moves": {step:?}}}, {{"kind": "word", "moves": {step:?}}}]"#);
    let mut f = fixture(&[]);
    let entry = f.commit(&json::parse(both.as_bytes()).unwrap());
    assert_eq!(f.layer.head_entry(), Some((entry, 120_000)));
    assert!(committed(&f.partial(entry, 5)));
    let stored = &f.layer.store().log.last().unwrap().recipe;
    assert_eq!(stored.as_array().map(|steps| steps.len()), Some(2), "two word steps");
    assert_eq!(f.digest(), reached(&word(&step[..5])));
    let g = reopen(&f.layer).unwrap();
    assert_eq!(g.recovery(), None);
    assert_eq!(g.head_entry(), Some((entry, 5)));
}

/// Guard (attack on row 3): a journal that renames an ordinary commit into a partial undo replays only when the commit
/// is exactly the step back that the live press would have made, and then it is that partial undo; otherwise the
/// replay stops in read-only recovery.
#[test]
fn a_renamed_commit_replays_only_as_the_step_back() {
    let mut f = fixture(&[]);
    let entry = f.commit(&word(&[1, 2]));
    f.commit(&word(&[-2]));
    let mut renamed = f.layer.store().clone();
    renamed.log[1].row = PARTIAL_UNDO_ROW.to_owned();
    let g = CommandLayer::open(kernel(), Table::h08(), Secret::fixed(SECRET), renamed).unwrap();
    assert_eq!(g.recovery(), None);
    assert_eq!(g.head_entry(), Some((entry, 1)), "the partial undo to the entry's first move");

    let mut h = fixture(&[]);
    h.commit(&word(&[1, 2]));
    h.commit(&word(&[-1]));
    let mut renamed = h.layer.store().clone();
    renamed.log[1].row = PARTIAL_UNDO_ROW.to_owned();
    let g = CommandLayer::open(kernel(), Table::h08(), Secret::fixed(SECRET), renamed).unwrap();
    assert_eq!(g.recovery().map(|e| e.code), Some(code::UNADMITTED_JOURNAL_ACTION));
}

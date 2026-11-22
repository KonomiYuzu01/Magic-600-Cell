//! ADR-D01.12's one-key preview-and-commit (owner, 22 November 2026, item 4; draft): `operation.preview.commit {draft,
//! token}` prepares the solver's own macro, a record draft, rechecks its preview as `operation.execute` rechecks a
//! presented one, and commits it, in one press. The preview replaces the pending one (ADR-D01.9); a failed recheck
//! leaves it pending and uncommitted. Only the solver's press reaches the row, the explore state pauses it, and it has
//! no default key.

use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use c600_command::layer::EXPLORE_ROW;
use c600_command::shell::default_keymap;
use c600_command::{
    code, draft_args, Caller, CommandLayer, Direction, Error, HeadlessFrames, Invocation, MemoryStore, PreviewState,
    Profile, Provenance, Receipt, Revision, Secret, Shell, Status, Table, TransactionKind, ONE_KEY_ROW, RECORD_ROW,
};
use c600_kernel::json::{self, Value};
use c600_kernel::Kernel;

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

fn letters(moves: &[i64]) -> Value {
    Value::Array(moves.iter().map(|&letter| Value::Int(letter)).collect())
}

fn text(s: &str) -> Value {
    Value::String(s.to_owned())
}

/// The one-key row's arguments for record `name`.
fn own(name: &str) -> Value {
    json::object([("draft", json::object([("record", text(name))]))])
}

fn code_of(receipt: &Receipt) -> &'static str {
    receipt.status.code().unwrap_or_else(|| panic!("not refused or failed: {:?}", receipt.status))
}

fn committed(receipt: &Receipt) -> bool {
    matches!(receipt.status, Status::Committed(_))
}

/// A revision without its epoch, which a reopen draws afresh (L10).
fn epochless(revision: &Revision) -> Revision {
    Revision { epoch: 0, ..revision.clone() }
}

/// The host's provenance reader as the tests stand it in: at a preview's admission it refuses a record's word whose
/// block is `foreign` (KN5.26), a word the solver did not keep; it accepts every other block.
struct Reader;

impl Provenance for Reader {
    fn check(&self, _block: &[u8]) -> Result<(), Error> {
        Ok(())
    }

    fn admit(&self, block: &[u8]) -> Result<(), Error> {
        match block {
            b"foreign" => Err(Error::new(code::NOT_GRANTED, "the test reader refuses the word in this session")),
            _ => Ok(()),
        }
    }
}

struct Fixture {
    layer: CommandLayer,
    shell: Shell,
    frames: HeadlessFrames,
}

/// A solving session at the solved state that protects `orbits`, with the test reader.
fn fixture(orbits: &[i64]) -> Fixture {
    let mut f = bare(orbits, Profile::Solving);
    f.layer.set_provenance(Box::new(Reader));
    f
}

/// A session in `profile` that protects `orbits`, with no provenance reader.
fn bare(orbits: &[i64], profile: Profile) -> Fixture {
    let k = kernel();
    let (start, protected) = (k.solved(), k.protected_set(orbits).unwrap());
    let table = Table::h08().with_drafts();
    let store = MemoryStore::default();
    let layer = CommandLayer::new_with_profile(k, table, Secret::fixed(SECRET), store, "s1", start, protected, profile);
    Fixture { layer, shell: Shell::new(Secret::fixed(SECRET), "shell"), frames: HeadlessFrames::new() }
}

fn reopen(layer: &CommandLayer) -> CommandLayer {
    CommandLayer::open(kernel(), Table::h08().with_drafts(), Secret::fixed(SECRET), layer.store().clone()).unwrap()
}

/// The state digest after committing `moves` from the solved state.
fn reached(moves: &[i64]) -> String {
    let mut g = fixture(&[]);
    g.keep("w", moves);
    assert!(committed(&g.one_key("w")));
    g.digest()
}

impl Fixture {
    /// Keeps `moves` in record `name` with an admitted block, as the solver's own macro.
    fn keep(&mut self, name: &str, moves: &[i64]) {
        self.write(name, letters(moves), "block");
    }

    fn write(&mut self, name: &str, payload: Value, block: &str) {
        let args = json::object([("payload", payload), ("provenance", text(block)), ("record", text(name))]);
        let receipt = self.shell.invoke(&mut self.layer, RECORD_ROW, args).unwrap();
        assert!(matches!(receipt.status, Status::Completed(_)), "{name}: {:?}", receipt.status);
    }

    fn one_key(&mut self, name: &str) -> Receipt {
        self.shell.invoke(&mut self.layer, ONE_KEY_ROW, own(name)).unwrap()
    }

    /// Previews and presents `moves` through `operation.preview`; returns the preview's ID.
    fn presented(&mut self, moves: &[i64]) -> u64 {
        let receipt = self.shell.invoke(&mut self.layer, "operation.preview", draft_args(&word(moves))).unwrap();
        let Status::Running(id) = receipt.status else { panic!("preview not admitted: {:?}", receipt.status) };
        assert_eq!(self.layer.run_jobs(), 1);
        self.frames.present(&mut self.shell, &mut self.layer, id).unwrap();
        id
    }

    fn execute(&mut self, id: u64) -> Receipt {
        let args = json::object([("preview", Value::Int(id as i64))]);
        self.shell.invoke(&mut self.layer, "operation.execute", args).unwrap()
    }

    fn explore(&mut self, on: bool) {
        let receipt = self.shell.invoke(&mut self.layer, EXPLORE_ROW, json::object([("on", Value::Bool(on))])).unwrap();
        assert!(receipt.status.code().is_none(), "{:?}", receipt.status);
    }

    /// Sets the Strict protection policy, with the solver's confirmation.
    fn strict(&mut self) {
        let args = json::object([("confirm", Value::Bool(true)), ("policy", text("strict"))]);
        let receipt = self.shell.invoke(&mut self.layer, "protection.policy.set", args).unwrap();
        assert!(receipt.status.code().is_none(), "{:?}", receipt.status);
    }

    fn step(&mut self, direction: Direction) -> Receipt {
        let hint = self.layer.undo_hint(direction).expect("an eligible target");
        let row = if direction == Direction::Undo { "session.undo" } else { "session.redo" };
        self.shell.invoke(&mut self.layer, row, json::object([("hint", hint.to_json())])).unwrap()
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

/// A letter whose move would move orbit 0.
fn letter_moving_orbit_0() -> i64 {
    (1..40)
        .find(|&letter| {
            let mut probe = fixture(&[0]);
            probe.keep("w", &[letter]);
            probe.one_key("w").status.code() == Some(code::ORBIT_WOULD_MOVE)
        })
        .expect("a letter that moves orbit 0")
}

/// ADR-D01.12: one press commits the solver's own macro kept in a record. The commit is the macro's retained move,
/// journaled with the one-key row; it leaves no preview pending, a reopen replays it, and `session.undo` and
/// `session.redo` step it as any commit.
#[test]
fn one_press_commits_the_solvers_own_macro() {
    let mut f = fixture(&[]);
    let solved = f.digest();
    f.keep("own", &[1, 2, 3]);
    assert_eq!(f.layer.availability(ONE_KEY_ROW), Ok(()));
    let receipt = f.one_key("own");
    assert!(committed(&receipt), "{:?}", receipt.status);
    assert_eq!(f.digest(), reached(&[1, 2, 3]));
    assert!(f.layer.preview().is_none(), "the committed preview is not pending");
    let last = f.layer.store().log.last().unwrap();
    assert_eq!((last.kind, last.row.as_str(), &last.caller), (TransactionKind::Commit, ONE_KEY_ROW, &Caller::Shell));
    assert_eq!(last.recipe, word(&[1, 2, 3]));

    let g = reopen(&f.layer);
    assert_eq!(g.recovery(), None);
    assert_eq!(epochless(g.revision()), epochless(f.layer.revision()));
    let committed_digest = f.digest();
    assert!(committed(&f.step(Direction::Undo)));
    assert_eq!(f.digest(), solved);
    assert!(committed(&f.step(Direction::Redo)));
    assert_eq!(f.digest(), committed_digest);
}

/// The press's preview replaces the pending one (ADR-D01.9), so a preview the solver was shown before the press is
/// replaced and no longer commits.
#[test]
fn the_press_replaces_the_pending_preview() {
    let mut f = fixture(&[]);
    f.keep("own", &[1]);
    let shown = f.presented(&[5]);
    assert_eq!(f.layer.preview_state(shown), Some(PreviewState::Presented));
    assert!(committed(&f.one_key("own")));
    assert_eq!(f.layer.preview_state(shown), Some(PreviewState::Replaced));
    let (order, digest) = (f.last_order(), f.digest());
    assert!(f
        .shell
        .prepare(&f.layer, "operation.execute", json::object([("preview", Value::Int(shown as i64))]))
        .map(|press| f.shell.send(&mut f.layer, press).status.code().is_some())
        .unwrap_or(true));
    f.unchanged(order, &digest);
    assert_eq!(f.digest(), reached(&[1]));
}

/// Every commit rechecks the protected orbits: a macro that would move a protected orbit is refused under Net and
/// Strict, and its preview stays pending and uncommitted for the solver to see.
#[test]
fn a_failed_recheck_leaves_the_preview_pending() {
    let letter = letter_moving_orbit_0();
    for policy in ["net", "strict"] {
        let mut f = fixture(&[0]);
        if policy == "strict" {
            f.strict();
        }
        f.keep("own", &[letter]);
        let (order, digest) = (f.last_order(), f.digest());
        assert_eq!(code_of(&f.one_key("own")), code::ORBIT_WOULD_MOVE, "{policy}");
        f.unchanged(order, &digest);
        let pending = f.layer.preview().expect("the refused press's preview");
        assert_eq!(pending.state, PreviewState::Ready, "{policy}");
        assert!(pending.end.is_none(), "{policy}: the macro's end is refused");
    }
}

/// The token binds the press and the state and protection revisions (LI.6): a press raced by a commit or by a
/// protection-policy change is `revision_changed`, and its preview, built at the head, stays pending and commits
/// through `operation.execute` once the solver is shown it; an edited argument is `token_mismatch` and builds no
/// preview, and a press sent again answers with its first receipt and commits nothing more.
#[test]
fn the_token_binds_the_press_and_the_revisions() {
    let mut f = fixture(&[]);
    f.keep("own", &[1, 2]);

    let raced = f.shell.prepare(&f.layer, ONE_KEY_ROW, own("own")).unwrap();
    let shown = f.presented(&[4]);
    assert!(committed(&f.execute(shown)));
    let (order, digest) = (f.last_order(), f.digest());
    assert_eq!(code_of(&f.shell.send(&mut f.layer, raced)), code::REVISION_CHANGED);
    f.unchanged(order, &digest);
    let pending = f.layer.preview().expect("the raced press's preview").id;
    f.frames.present(&mut f.shell, &mut f.layer, pending).unwrap();
    assert!(committed(&f.execute(pending)), "the pending preview commits once shown");
    let mut g = fixture(&[]);
    g.keep("w", &[4, 1, 2]);
    assert!(committed(&g.one_key("w")));
    assert_eq!(f.digest(), g.digest());

    let raced = f.shell.prepare(&f.layer, ONE_KEY_ROW, own("own")).unwrap();
    f.strict();
    let (order, digest) = (f.last_order(), f.digest());
    assert_eq!(code_of(&f.shell.send(&mut f.layer, raced)), code::REVISION_CHANGED);
    f.unchanged(order, &digest);

    f.keep("other", &[3]);
    let held = f.layer.preview().map(|p| p.id);
    let (order, digest) = (f.last_order(), f.digest());
    let mut edited = f.shell.prepare(&f.layer, ONE_KEY_ROW, own("own")).unwrap();
    edited.args = own("other");
    assert_eq!(code_of(&f.shell.send(&mut f.layer, edited)), code::TOKEN_MISMATCH);
    assert_eq!(f.layer.preview().map(|p| p.id), held, "a mismatched press builds no preview");
    f.unchanged(order, &digest);

    let press = f.shell.prepare(&f.layer, ONE_KEY_ROW, own("own")).unwrap();
    let first = f.shell.send(&mut f.layer, press.clone());
    assert!(committed(&first));
    let (order, digest) = (f.last_order(), f.digest());
    assert_eq!(f.shell.send(&mut f.layer, press).status, first.status, "the press's first receipt");
    f.unchanged(order, &digest);
}

/// The draft is the solver's own macro kept in a record, resolved as `operation.preview` resolves one: an inline
/// recipe or any other form is `argument_form` before the token is spent, a record that does not hold letters is
/// `named_object_invalid`, and in a solving session a word whose block the host's reader refuses, or any word without
/// a reader (a reopen replays the records without one), is `not_granted` (KN5.26). None builds a preview or changes
/// anything. A research session does not ask the reader.
#[test]
fn the_draft_is_a_kept_record_only() {
    let mut f = fixture(&[]);
    f.keep("own", &[1]);
    f.write("note", text("not a word"), "block");
    f.write("theirs", letters(&[2]), "foreign");
    let (order, digest) = (f.last_order(), f.digest());

    let inline = json::object([("recipe", word(&[1]))]);
    for draft in [
        inline,
        json::object([("record", text("own")), ("extra", Value::Int(1))]),
        json::object([("record", Value::Int(1))]),
        text("own"),
    ] {
        let press = f.shell.prepare(&f.layer, ONE_KEY_ROW, json::object([("draft", draft.clone())])).unwrap();
        assert_eq!(code_of(&f.shell.send(&mut f.layer, press)), code::ARGUMENT_FORM, "{draft:?}");
    }
    for (name, expected) in [("missing", code::NAMED_OBJECT_INVALID), ("note", code::NAMED_OBJECT_INVALID)] {
        assert_eq!(code_of(&f.one_key(name)), expected, "{name}");
    }
    assert_eq!(code_of(&f.one_key("theirs")), code::NOT_GRANTED, "a word the solver did not keep");
    assert!(f.layer.preview().is_none(), "no refused draft builds a preview");
    f.unchanged(order, &digest);

    let mut unread = reopen(&f.layer);
    let mut shell = Shell::new(Secret::fixed(SECRET), "reopened");
    let receipt = shell.invoke(&mut unread, ONE_KEY_ROW, own("own")).unwrap();
    assert_eq!(code_of(&receipt), code::NOT_GRANTED, "a solving session without the reader");
    assert!(unread.preview().is_none());

    let mut research = bare(&[], Profile::Research);
    research.layer.set_provenance(Box::new(Reader));
    research.write("theirs", letters(&[1]), "foreign");
    assert!(committed(&research.one_key("theirs")));
    assert_eq!(research.digest(), reached(&[1]));
}

/// No run reaches the row, a commit row (KN5.15), and the explore state pauses it before its token is spent
/// (ADR-D01.39), building no preview; the shell's next press commits once the state is off.
#[test]
fn the_explore_state_pauses_it_and_a_run_never_reaches_it() {
    let mut f = fixture(&[]);
    f.keep("own", &[1]);
    let (order, digest) = (f.last_order(), f.digest());
    f.explore(true);
    let press = f.shell.prepare(&f.layer, ONE_KEY_ROW, own("own")).unwrap();
    assert_eq!(code_of(&f.shell.send(&mut f.layer, press)), code::COMMITS_PAUSED);
    assert!(f.layer.preview().is_none());
    f.unchanged(order, &digest);
    f.explore(false);

    let run = Caller::Runtime { run: "r1".into() };
    let invocation = Invocation {
        command: "c1".into(),
        row: ONE_KEY_ROW.into(),
        args: own("own"),
        token: None,
        epoch: f.layer.revision().epoch,
        budget: None,
    };
    assert_eq!(code_of(&f.layer.invoke(&run, invocation)), code::COMMIT_ROW_REFUSED);
    assert!(f.layer.preview().is_none());
    f.unchanged(order, &digest);
    assert!(committed(&f.one_key("own")), "the next press commits once the state is off");
}

/// No default key: the solver binds their own macros.
#[test]
fn it_has_no_default_key() {
    assert!(default_keymap().iter().all(|binding| binding.row != ONE_KEY_ROW));
    assert_eq!(Table::h08().row(ONE_KEY_ROW).and_then(|row| row.key_0_4.clone()), None);
}

/// A journal that gives a one-key commit to a run does not reopen intact (B7), as any commit whose caller is not the
/// shell.
#[test]
fn a_journal_that_gives_the_commit_to_a_run_does_not_reopen_intact() {
    let mut f = fixture(&[]);
    f.keep("own", &[1]);
    assert!(committed(&f.one_key("own")));
    let mut forged = f.layer.store().clone();
    let index = forged.log.len() - 1;
    assert_eq!(forged.log[index].row, ONE_KEY_ROW);
    forged.log[index].caller = Caller::Runtime { run: "probe/2".into() };
    let reopened = CommandLayer::open(kernel(), Table::h08().with_drafts(), Secret::fixed(SECRET), forged).unwrap();
    assert_eq!(reopened.recovery().map(|e| e.code), Some(code::UNADMITTED_JOURNAL_ACTION));
}

//! KN4.3's journal query in the command layer (draft; LP3's C8): `CommandLayer::journal`, a page of the move tree's
//! events on one line, newest first, from a named event or the head toward the start. Each entry's kind is read from
//! the move tree's node alone and must agree with the transaction the journal keeps for its event; an exit's mark is
//! the digest of the exit record that transaction keeps, which the next entry's `exit_root` names. Where the frozen
//! text is silent, the test follows the draft rulings recorded in `src/lib.rs`. Covers no conformance ID.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use c600_command::{
    code, digest, draft_args, CommandLayer, Direction, EntryKind, HeadlessFrames, JournalEntry, JournalPage,
    MemoryStore, Receipt, Secret, Shell, Status, Table, Transaction, TransactionKind, JOURNAL_PAGE_LIMIT,
    JUMBLE_OFF_ROW, JUMBLE_ON_ROW, JUMBLE_TWIST_ROW,
};
use c600_jumble::twist::Menu;
use c600_jumble::Context;
use c600_kernel::json::{self, Value};
use c600_kernel::request::{self as wire, Binding, MenuPin, Operation};
use c600_kernel::{Derivation, Kernel, ProtectedSet, Snapshot};

const SECRET: [u8; 32] = [7; 32];

/// The pinned I_a menu's set and order digests.
const SET: &str = "fddc8532b9184bd1b413c1032b5a7a305dd9fd1f10385d87ba6ec4fd06eb958c";
const ORDER: &str = "5a0429d2946fbf58b495d8f300e24a3fa164b1d9518ce4a5c3f4ef0a40625c3c";

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

fn ctx() -> Arc<Context> {
    static CTX: OnceLock<Arc<Context>> = OnceLock::new();
    CTX.get_or_init(|| Arc::new(Context::load(&repository().join("assets")).expect("the exact model loads"))).clone()
}

/// The I_a menu of `research/jumbling/fixtures/wj-I_a.json`, the menu KN3's tests pin.
fn pinned() -> Menu {
    static FIXTURE: OnceLock<Value> = OnceLock::new();
    let fixture = FIXTURE.get_or_init(|| {
        let path = repository().join("research/jumbling/fixtures/wj-I_a.json");
        json::parse(&std::fs::read(path).expect("the I_a fixture")).expect("the I_a fixture is JSON")
    });
    Menu::from_record(&ctx(), fixture.get("menu").expect("a menu")).expect("the pinned I_a menu")
}

/// Menu item `index` at cap 0, pressed directly: item 6 is retained, item 12 a jumble item.
fn item(index: u64) -> Binding {
    Binding {
        request: wire::menu_request(SET, ORDER, index),
        cap: 0,
        operation: Operation::MenuDirect,
        menu: Some(MenuPin { set: SET.to_owned(), order: ORDER.to_owned() }),
    }
}

fn word(letters: &[i64]) -> Value {
    json::parse(format!(r#"[{{"kind": "word", "moves": {letters:?}}}]"#).as_bytes()).unwrap()
}

/// A commit's retained letters: the primitive count of its recipe.
fn letters(recipe: &Value) -> u64 {
    kernel().recipe(recipe).unwrap().primitive_count() as u64
}

fn committed(receipt: &Receipt) -> bool {
    matches!(receipt.status, Status::Committed(_))
}

struct Fixture {
    layer: CommandLayer,
    shell: Shell,
    frames: HeadlessFrames,
}

/// A session `session` at `start` with the I_a menu pinned and no protected orbit.
fn fixture_at(session: &str, start: Snapshot) -> Fixture {
    let table = Table::h08().with_drafts();
    let none = ProtectedSet::none();
    let mut layer =
        CommandLayer::new(kernel(), table, Secret::fixed(SECRET), MemoryStore::default(), session, start, none);
    layer.pin_menu(ctx(), pinned()).unwrap();
    Fixture { layer, shell: Shell::new(Secret::fixed(SECRET), "shell"), frames: HeadlessFrames::new() }
}

fn fixture() -> Fixture {
    fixture_at("s1", kernel().solved())
}

/// A reopen of `layer`'s store with the I_a menu pinned before the replay.
fn reopen(layer: &CommandLayer) -> Fixture {
    let (table, secret) = (Table::h08().with_drafts(), Secret::fixed(SECRET));
    let layer = CommandLayer::open_with_menu(kernel(), table, secret, layer.store().clone(), ctx(), pinned());
    Fixture { layer: layer.unwrap(), shell: Shell::new(Secret::fixed(SECRET), "shell"), frames: HeadlessFrames::new() }
}

impl Fixture {
    fn press(&mut self, row: &str) -> Receipt {
        let revision = self.layer.revision().to_json();
        let press = self.shell.prepare(&self.layer, row, json::object([("revision", revision)])).unwrap();
        self.shell.send(&mut self.layer, press)
    }

    fn enter(&mut self) -> Receipt {
        self.press(JUMBLE_ON_ROW)
    }

    fn exit(&mut self) -> Receipt {
        self.press(JUMBLE_OFF_ROW)
    }

    /// `jumble.twist.menu` for `binding`: with Jumble on, a pose move.
    fn menu_twist(&mut self, binding: &Binding) -> Receipt {
        let Value::Object(mut fields) = binding.to_json() else { panic!("a binding is an object") };
        let cap = fields.remove("cap").expect("a binding has a cap");
        let revision = self.layer.revision().to_json();
        let args = json::object([("cap", cap), ("letter", Value::Object(fields)), ("revision", revision)]);
        let press = self.shell.prepare(&self.layer, JUMBLE_TWIST_ROW, args).unwrap();
        self.shell.send(&mut self.layer, press)
    }

    fn commit(&mut self, recipe: &Value) -> Receipt {
        let receipt = self.shell.invoke(&mut self.layer, "operation.preview", draft_args(recipe)).unwrap();
        let Status::Running(id) = receipt.status else { panic!("preview not admitted: {:?}", receipt.status) };
        assert_eq!(self.layer.run_jobs(), 1);
        self.frames.present(&mut self.shell, &mut self.layer, id).unwrap();
        let press =
            self.shell.prepare(&self.layer, "operation.execute", json::object([("preview", Value::Int(id as i64))]));
        self.shell.send(&mut self.layer, press.unwrap())
    }

    fn step(&mut self, direction: Direction) -> Receipt {
        let hint = self.layer.undo_hint(direction).expect("an eligible target");
        let row = if direction == Direction::Undo { "session.undo" } else { "session.redo" };
        let args = Value::Object(BTreeMap::from([("hint".to_owned(), hint.to_json())]));
        self.shell.invoke(&mut self.layer, row, args).unwrap()
    }

    /// Every page of the walk from `from` at `limit` events a page, joined; asserts that each page but the last is
    /// full.
    fn walk(&self, from: Option<u64>, limit: u64) -> Vec<JournalEntry> {
        let (mut entries, mut cursor) = (Vec::new(), None::<String>);
        loop {
            let page = self.layer.journal(from, limit, cursor.as_deref()).unwrap();
            let full = page.entries.len() as u64 == limit;
            entries.extend(page.entries);
            match page.next {
                Some(next) => {
                    assert!(full, "only a full page has a cursor");
                    cursor = Some(next);
                }
                None => return entries,
            }
        }
    }

    /// The transaction the journal keeps for the event of action order `order`.
    fn transaction(&self, order: u64) -> &Transaction {
        self.layer.store().log.iter().find(|transaction| transaction.order == order).expect("a transaction")
    }

    /// What no read may change: the revision, the durable log and the head's undo and redo hints.
    fn fixed(&self) -> (String, usize, bool, bool) {
        let r = format!("{:?}", self.layer.revision());
        let hints = (self.layer.undo_hint(Direction::Undo).is_some(), self.layer.undo_hint(Direction::Redo).is_some());
        (r, self.layer.store().log.len(), hints.0, hints.1)
    }
}

fn orders(entries: &[JournalEntry]) -> Vec<u64> {
    entries.iter().map(|entry| entry.order).collect()
}

fn kinds(entries: &[JournalEntry]) -> Vec<&'static str> {
    entries.iter().map(|entry| entry.kind.name()).collect()
}

fn refused(page: Result<JournalPage, c600_command::Error>) -> &'static str {
    page.expect_err("a refusal").code
}

/// A session's move tree read back as entries: a commit (its letters are its recipe's primitive count), `jumble.on`'s
/// crossing, a pose move, `jumble.off`'s exit (its mark is the SHA-256 of the exit record its transaction keeps, which
/// the next entry's `exit_root` names) and the start, solved here. Each entry's kind agrees with its transaction's, and
/// the events are the transactions' and the revision's names. Paging gives the same walk at every limit, and the start
/// is answered once. The read answers with Jumble on and off, and changes nothing.
#[test]
fn a_page_reads_the_move_tree_newest_first() {
    let mut f = fixture();
    let start = f.layer.journal(None, 10, None).unwrap();
    let expected = JournalEntry { order: 0, kind: EntryKind::Start, letters: 0, solved: true, exit: None };
    assert_eq!(start, JournalPage { entries: vec![expected], next: None });
    assert_eq!(f.layer.revision().head, start.entries[0].event());

    let recipe = word(&[2, -1, 7]);
    assert!(committed(&f.commit(&recipe)));
    assert!(committed(&f.enter()));
    assert!(committed(&f.menu_twist(&item(6))));
    let before = f.fixed();
    let on = f.layer.journal(None, 10, None).unwrap();
    assert_eq!(f.fixed(), before, "a read with Jumble on changes nothing");
    assert_eq!(
        (orders(&on.entries), kinds(&on.entries)),
        (vec![3, 2, 1, 0], vec!["pose", "crossing", "commit", "start"])
    );
    assert!(committed(&f.exit()));
    assert!(committed(&f.enter()));

    let before = f.fixed();
    let page = f.layer.journal(None, JOURNAL_PAGE_LIMIT, None).unwrap();
    assert_eq!(f.fixed(), before, "a read changes nothing");
    assert_eq!(page.next, None);
    let entries = &page.entries;
    assert_eq!(orders(entries), [5, 4, 3, 2, 1, 0]);
    assert_eq!(kinds(entries), ["crossing", "exit", "pose", "crossing", "commit", "start"]);
    assert_eq!(f.layer.revision().head, entries[0].event());
    for entry in &entries[..5] {
        let transaction = f.transaction(entry.order);
        assert_eq!(transaction.event, entry.event());
        let kind = match transaction.kind {
            TransactionKind::Commit => "commit",
            TransactionKind::Crossing => "crossing",
            TransactionKind::JumbleMove => "pose",
            TransactionKind::Exit => "exit",
            other => panic!("not a move: {other:?}"),
        };
        assert_eq!(entry.kind.name(), kind, "{}", entry.event());
        let expected = if entry.kind == EntryKind::Commit { letters(&recipe) } else { 0 };
        assert_eq!((entry.letters, entry.solved), (expected, false), "{}", entry.event());
    }
    assert_eq!(entries[4].letters, 3);
    let exit = f.transaction(4).crossing.clone().expect("the exit's record");
    assert_eq!(entries[1].exit, Some(digest(&exit)));
    assert!(entries.iter().filter(|entry| entry.kind != EntryKind::Exit).all(|entry| entry.exit.is_none()));
    let root = f.transaction(5).crossing.clone().expect("the entry's exit_root record");
    assert_eq!(root.get("entrance"), Some(&Value::String("exit_root".to_owned())));
    assert_eq!(root.get("exit"), entries[1].exit.as_ref().map(|mark| Value::String(mark.clone())).as_ref());

    for limit in 1..=7 {
        assert_eq!(&f.walk(None, limit), entries, "limit {limit}");
    }
    let first = f.layer.journal(None, 2, None).unwrap();
    assert_eq!(orders(&first.entries), [5, 4]);
    let second = f.layer.journal(None, 2, first.next.as_deref()).unwrap();
    assert_eq!(orders(&second.entries), [3, 2]);
    let last = f.layer.journal(None, 2, second.next.as_deref()).unwrap();
    assert_eq!((orders(&last.entries), last.next), (vec![1, 0], None));
    assert_eq!(f.layer.journal(Some(3), 10, None).unwrap().entries, entries[2..]);
    assert_eq!(f.layer.journal(Some(0), 1, None).unwrap().entries, entries[5..]);
}

/// The walk reads one line: after an undo and a new commit, the head's line skips the other branch, while a `from` on
/// that branch reads it. An undone exit keeps its kind and mark when a new move drops the redo stack, since its node
/// keeps them. A start that is not solved says so.
#[test]
fn the_walk_follows_one_line_and_keeps_every_node() {
    let mut f = fixture();
    assert!(committed(&f.commit(&word(&[2]))));
    assert!(committed(&f.commit(&word(&[5, -3]))));
    assert!(committed(&f.step(Direction::Undo)));
    assert!(committed(&f.commit(&word(&[9]))));
    assert_eq!(f.layer.revision().head, "e4");
    assert_eq!(orders(&f.walk(None, 2)), [4, 1, 0]);
    assert_eq!(orders(&f.walk(Some(2), 2)), [2, 1, 0]);
    assert_eq!(f.layer.journal(Some(2), 1, None).unwrap().entries[0].letters, 2);
    assert_eq!(refused(f.layer.journal(Some(3), 1, None)), code::REVISION_GONE, "an undo is no node");

    assert!(committed(&f.enter()));
    assert!(committed(&f.menu_twist(&item(6))));
    assert!(committed(&f.exit()));
    let exit = f.layer.journal(None, 1, None).unwrap().entries[0].clone();
    assert_eq!(exit.kind, EntryKind::Exit);
    assert!(committed(&f.step(Direction::Undo)));
    assert!(committed(&f.menu_twist(&item(12))));
    assert_eq!(f.layer.journal(Some(exit.order), 1, None).unwrap().entries[0], exit, "a dropped redo keeps the node");
    assert_eq!(kinds(&f.walk(None, 3)), ["pose", "pose", "crossing", "commit", "commit", "start"]);

    let scrambled =
        match kernel().derive(&kernel().solved(), &kernel().word(&[17, -305, 911]).unwrap(), &ProtectedSet::none()) {
            Ok(Derivation::Derived(derived)) => derived.state,
            _ => panic!("a scramble derives"),
        };
    let g = fixture_at("s1", scrambled);
    let start = g.layer.journal(None, 1, None).unwrap().entries[0].clone();
    assert_eq!((start.kind, start.solved), (EntryKind::Start, false));
}

/// Refusals, in the draft order: a `limit` of 0 or above the page limit (`KN5:input.argument_form`), then a `from` the
/// move tree does not hold (`KN2:stale.revision_gone`), then a cursor of another walk, session or launch, a forged one
/// (an unkeyed digest of its binding among them) or text that is no cursor (`KN5:input.argument_form`). A refused read
/// changes nothing. The cursor grants nothing: a walk from the event it names reads the same entries.
#[test]
fn refusals_follow_the_draft_order_and_change_nothing() {
    let mut f = fixture();
    for recipe in [word(&[2]), word(&[3]), word(&[4])] {
        assert!(committed(&f.commit(&recipe)));
    }
    let before = f.fixed();
    for limit in [0, JOURNAL_PAGE_LIMIT + 1, u64::MAX] {
        assert_eq!(refused(f.layer.journal(None, limit, None)), code::ARGUMENT_FORM, "limit {limit}");
        assert_eq!(refused(f.layer.journal(Some(99), limit, Some("x"))), code::ARGUMENT_FORM, "limit first");
    }
    assert_eq!(f.layer.journal(None, JOURNAL_PAGE_LIMIT, None).unwrap().entries.len(), 4);
    for from in [4, 99, u64::MAX] {
        assert_eq!(refused(f.layer.journal(Some(from), 1, None)), code::REVISION_GONE, "e{from}");
        assert_eq!(refused(f.layer.journal(Some(from), 1, Some("x"))), code::REVISION_GONE, "from before cursor");
    }

    let first = f.layer.journal(None, 2, None).unwrap();
    let cursor = first.next.clone().expect("a cursor");
    assert!(cursor.starts_with("e1:"), "{cursor}");
    let (event, mark) = cursor.split_once(':').unwrap();
    let forged = [
        "x".to_owned(),
        String::new(),
        event.to_owned(),
        format!("e01:{mark}"),
        format!("e2:{mark}"),
        format!("{event}:{}", "0".repeat(64)),
        format!("{event}:{}", mark.to_uppercase()),
    ];
    for cursor in &forged {
        assert_eq!(refused(f.layer.journal(None, 2, Some(cursor))), code::ARGUMENT_FORM, "{cursor:?}");
    }
    assert_eq!(refused(f.layer.journal(Some(3), 2, Some(&cursor))), code::ARGUMENT_FORM, "another walk's cursor");
    let other = fixture_at("s2", kernel().solved());
    assert_eq!(refused(other.layer.journal(None, 2, Some(&cursor))), code::ARGUMENT_FORM, "another session's cursor");
    assert_eq!(f.fixed(), before, "a refused read changes nothing");
    let next = f.layer.journal(None, 2, Some(&cursor)).unwrap();
    assert_eq!(next.entries, f.layer.journal(Some(1), 2, None).unwrap().entries, "a cursor grants nothing");

    // A walk from the head binds no head: its cursor continues after a new commit moves the head.
    assert!(committed(&f.commit(&word(&[5]))));
    assert_eq!(f.layer.journal(None, 2, Some(&cursor)).unwrap(), next);

    // The mark is keyed by the launch's secret: the plain digest of the cursor's binding is refused, and a restart's
    // new secret refuses the old launch's cursors while a walk from the event still reads.
    let session = f.layer.revision().session.clone();
    let fields = [
        ("from", Value::String(String::new())),
        ("next", Value::String("e1".to_owned())),
        ("query", Value::String("journal".to_owned())),
        ("session", Value::String(session)),
    ];
    let unkeyed = format!("e1:{}", digest(&json::object(fields)));
    assert_eq!(refused(f.layer.journal(None, 2, Some(&unkeyed))), code::ARGUMENT_FORM, "an unkeyed cursor");
    f.layer.restart(Secret::fixed([9; 32]));
    assert_eq!(refused(f.layer.journal(None, 2, Some(&cursor))), code::ARGUMENT_FORM, "another launch's cursor");
    let renewed = f.layer.journal(None, 3, None).unwrap().next.expect("a cursor");
    assert!(renewed.starts_with("e1:") && renewed != cursor, "{renewed}");
    assert_eq!(f.layer.journal(None, 2, Some(&renewed)).unwrap().entries, next.entries);
}

/// The move tree is durable: a reopen, which draws a new epoch and rebuilds the tree from the journal, answers every
/// event as before, the exit's mark and the poses' kinds included, and a cursor stays valid under the same secret.
#[test]
fn a_reopen_answers_the_same_journal() {
    let mut f = fixture();
    assert!(committed(&f.commit(&word(&[2, -1]))));
    assert!(committed(&f.enter()));
    assert!(committed(&f.menu_twist(&item(6))));
    assert!(committed(&f.exit()));
    assert!(committed(&f.commit(&word(&[4]))));
    let page = f.layer.journal(None, JOURNAL_PAGE_LIMIT, None).unwrap();
    let first = f.layer.journal(None, 3, None).unwrap();
    let g = reopen(&f.layer);
    assert_ne!(g.layer.revision().epoch, f.layer.revision().epoch);
    assert_eq!(g.layer.journal(None, JOURNAL_PAGE_LIMIT, None).unwrap(), page);
    assert_eq!(kinds(&page.entries), ["commit", "exit", "pose", "crossing", "commit", "start"]);
    assert_eq!(g.layer.journal(None, 3, None).unwrap(), first);
    assert_eq!(g.layer.journal(None, 3, first.next.as_deref()).unwrap().entries, page.entries[3..]);
}

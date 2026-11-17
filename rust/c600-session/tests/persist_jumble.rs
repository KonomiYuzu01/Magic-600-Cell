//! The session store's jumbling records: stored menu words ([`word::admit_in`], PS1.18 and PS1.21) and the episodes
//! KN5's journal keeps ([`JournalStore`], PS1.40). CT-PS1-11 and CT-PS1-22, headless, with PHS.62 markers. The twist
//! menu is the I_a fixture's (`research/jumbling/fixtures/wj-I_a.json`), as in c600-command's jumbling tests; Windows,
//! display and performance parts are not run here.

mod common;
mod persist_support;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use c600_command::layer::{JUMBLE_ON_MODE, MODE};
use c600_command::{
    code, draft_args, CommandLayer, Completion, CompletionKind, Direction, Error, HeadlessFrames, Receipt, Secret,
    Shell, Status, Table, JUMBLE_OFF_ROW, JUMBLE_ON_ROW, JUMBLE_TWIST_ROW, POLICY_ROW, TWIST_ROW,
};
use c600_jumble::codes::MENU_WORD_NEEDS_JUMBLE_ON;
use c600_jumble::q5::transpose;
use c600_jumble::rotation;
use c600_jumble::twist::Menu;
use c600_jumble::Context;
use c600_kernel::codes::IDENTITY_MISMATCH;
use c600_kernel::json::{self, Value};
use c600_kernel::request::{self as wire, Binding, MenuPin, Operation};
use c600_kernel::ProtectedSet;
use c600_session::disk::{Fault, MemDisk};
use c600_session::journal::{self, Tail};
use c600_session::persist::codec::Json;
use c600_session::persist::codes::{IDENTITY, NEEDS_JUMBLE_ON};
use c600_session::persist::record::identity_json;
use c600_session::persist::word::{self, Current, Letter, Store};
use c600_session::JournalStore;
use persist_support::{ji23, outcome, set};

const SECRET: [u8; 32] = [7; 32];

fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..")
}

fn ctx() -> Arc<Context> {
    static CTX: OnceLock<Arc<Context>> = OnceLock::new();
    CTX.get_or_init(|| Arc::new(Context::load(&repository().join("assets")).expect("the exact model loads"))).clone()
}

/// `research/jumbling/fixtures/wj-I_a.json`'s menu record.
fn menu_record() -> &'static Value {
    static RECORD: OnceLock<Value> = OnceLock::new();
    RECORD.get_or_init(|| {
        let path = repository().join("research/jumbling/fixtures/wj-I_a.json");
        let fixture = json::parse(&std::fs::read(path).expect("the I_a fixture")).expect("the I_a fixture is JSON");
        fixture.get("menu").expect("a menu").clone()
    })
}

/// The I_a menu, the menu KN3's tests pin.
fn pinned() -> Menu {
    Menu::from_record(&ctx(), menu_record()).expect("the pinned I_a menu")
}

/// The I_a menu with its base reversed: the same set in another order, so another jumbling identity block.
fn reversed() -> Menu {
    let mut record = menu_record().clone();
    if let Value::Object(fields) = &mut record {
        if let Some(Value::Array(base)) = fields.get_mut("base") {
            base.reverse();
        }
    }
    Menu::from_record(&ctx(), &record).expect("the reversed I_a menu")
}

/// The direct press of `menu`'s item at a letter's grip.
fn press(menu: &Menu, (grip, item): Letter) -> Binding {
    Binding {
        request: wire::menu_request(&menu.identity, &menu.order, u64::from(item)),
        cap: u32::from(grip),
        operation: Operation::MenuDirect,
        menu: Some(MenuPin { set: menu.identity.clone(), order: menu.order.clone() }),
    }
}

/// Item `index` of the I_a menu at cap 0. At cap 0 item 0 is the identity, items 1 to 11 are retained and 12 to 59
/// are jumble items.
fn item(index: u16) -> Binding {
    press(&pinned(), (0, index))
}

fn recipe(letters: &[i64]) -> Value {
    json::parse(format!(r#"[{{"kind": "word", "moves": {letters:?}}}]"#).as_bytes()).unwrap()
}

fn text(s: &str) -> Value {
    Value::String(s.to_owned())
}

// ------------------------------------------------------------------------------------------------- menu words

/// Whether `letter` is a retained item of `menu` (JI.35), which a session computes from its pinned menu: the press
/// resolves and has a retained word. The identity item is retained, with the empty word.
fn retained(ctx: &Context, menu: &Menu, letter: Letter) -> bool {
    rotation::resolve(ctx, Some(menu), &press(menu, letter))
        .is_ok_and(|resolution| rotation::retained_word(ctx, &resolution).is_ok())
}

/// The inverse of each item of `menu`, by index.
fn inverses(ctx: &Context, menu: &Menu) -> BTreeMap<u16, u16> {
    let items = menu.for_cap(ctx, 0).expect("the menu at cap 0");
    let index = |i: usize| u16::try_from(i).expect("an item");
    let inverse = |item: usize| {
        let back = transpose(&items[item].matrix);
        items.iter().position(|other| *other.matrix == back).expect("the menu holds each inverse")
    };
    (0..items.len()).map(|i| (index(i), index(inverse(i)))).collect()
}

/// A menu word of `letters` recorded under `identity`, with the JI23 fixture's start and provenance, and its chunks
/// by stored digest.
fn menu_word(identity: &Json, letters: &[Letter]) -> (Json, BTreeMap<String, Vec<u8>>) {
    let f = ji23();
    let start = f.record.get("payload").and_then(|payload| payload.get("start")).cloned().expect("a start");
    let provenance = f.record.get("provenance").cloned().expect("a provenance");
    word::write(identity, &start, letters, &[], &[], &provenance).expect("a menu word")
}

/// `identity` with the field `key` replaced.
fn identity_with(identity: &Json, key: &str, value: Json) -> Json {
    let mut identity = identity.clone();
    set(&mut identity, key, value);
    identity
}

/// CT-PS1-11 (part: stored menu words, whole for PS1.18's identity block and PS1.21's Jumble-off rule; the journal
/// part is below): a word recorded under the pinned I_a menu replays its literal letters. One recorded under the
/// reversed I_a order, another contract revision or another model is refused as `KN1:input.identity_mismatch` before
/// a letter is read, so its letters are never read under the current menu instead. With Jumble off a word holding a
/// jumble letter is refused with the command layer's `KN5:input.menu_word_needs_jumble_on`, and a retained word
/// replays.
#[test]
fn ct_ps1_11_a_menu_word_is_admitted_only_under_its_identity_block() {
    let ctx = ctx();
    let menu = pinned();
    let identity = identity_json(&ctx.identity(Some(&menu)));
    let inverse = inverses(&ctx, &menu);
    let pin = word::Menu { id: menu.identity.clone(), order: menu.order.clone() };
    let is_retained = |letter: Letter| retained(&ctx, &menu, letter);
    assert!((0..12).all(|item| is_retained((0, item))), "items 0 to 11 are retained at cap 0");
    assert!(!is_retained((0, 12)), "item 12 is a jumble item");
    assert!(is_retained((5, 7)), "an item index keeps its kind at every cap");
    assert_eq!(NEEDS_JUMBLE_ON, MENU_WORD_NEEDS_JUMBLE_ON, "the store refuses with the command layer's code");

    let none = BTreeMap::new();
    let on = Current { identity: &identity, jumble_on: true, retained: &is_retained };
    let off = Current { jumble_on: false, ..on };
    let mixed: [Letter; 3] = [(0, 3), (0, 12), (5, 7)];
    let (written, blobs) = menu_word(&identity, &mixed);
    let store = Store { blobs: &blobs, records: &none, inverse: &inverse, menu: &pin };
    assert_eq!(word::admit_in(&written, &store, &on), Ok(mixed.to_vec()));
    assert_eq!(outcome(word::admit_in(&written, &store, &off)), NEEDS_JUMBLE_ON);
    let kept: [Letter; 3] = [(0, 3), (5, 7), (0, 0)];
    let (written, blobs) = menu_word(&identity, &kept);
    let store = Store { blobs: &blobs, records: &none, inverse: &inverse, menu: &pin };
    assert_eq!(word::admit_in(&written, &store, &off), Ok(kept.to_vec()));

    let others = [
        (identity_json(&ctx.identity(Some(&reversed()))), "the reversed I_a order"),
        (identity_with(&identity, "contract_rev", Json::text("state-contract 2026-11-14 A1-A5")), "another contract"),
        (identity_with(&identity, "model_id", Json::text(&"0".repeat(64))), "another model"),
    ];
    let empty = BTreeMap::new();
    for (other, what) in &others {
        assert_ne!(other, &identity, "{what}");
        let (written, blobs) = menu_word(other, &mixed);
        let store = Store { blobs: &blobs, records: &none, inverse: &inverse, menu: &pin };
        assert_eq!(outcome(word::admit_in(&written, &store, &on)), IDENTITY, "{what}");
        let bare = Store { blobs: &empty, records: &none, inverse: &inverse, menu: &pin };
        assert_eq!(outcome(word::admit_in(&written, &bare, &on)), IDENTITY, "{what}: before a letter is read");
    }
}

// ------------------------------------------------------------------------------------------------- the journal

struct Fixture {
    layer: CommandLayer<JournalStore>,
    shell: Shell,
    frames: HeadlessFrames,
}

/// A new session at the solved state on an empty memory disk, with `menu` pinned or none.
fn create(disk: &MemDisk, menu: Option<Menu>) -> Fixture {
    let k = common::kernel();
    let store = JournalStore::create(Box::new(disk.clone())).expect("an empty disk takes a session");
    let (table, secret) = (Table::h08().with_drafts(), Secret::fixed(SECRET));
    let mut layer = CommandLayer::new(k.clone(), table, secret, store, "s1", k.solved(), ProtectedSet::none());
    if let Some(menu) = menu {
        layer.pin_menu(ctx(), menu).expect("the I_a menu pins");
    }
    Fixture { layer, shell: Shell::new(Secret::fixed(SECRET), "shell"), frames: HeadlessFrames::new() }
}

/// The session `disk` holds, reopened: with `pin`'s menu, read in its context, pinned before the replay
/// (`CommandLayer::open_with_menu`), or with no menu (`CommandLayer::open`).
fn reopen(disk: &MemDisk, pin: Option<(Arc<Context>, Menu)>) -> Result<Fixture, Error> {
    let store = JournalStore::open(Box::new(disk.clone())).expect("the journal opens");
    let (k, table, secret) = (common::kernel(), Table::h08().with_drafts(), Secret::fixed(SECRET));
    let layer = match pin {
        Some((context, menu)) => CommandLayer::open_with_menu(k, table, secret, store, context, menu),
        None => CommandLayer::open(k, table, secret, store),
    }?;
    Ok(Fixture { layer, shell: Shell::new(Secret::fixed(SECRET), "shell"), frames: HeadlessFrames::new() })
}

/// `reopen` under the I_a menu.
fn reopen_pinned(disk: &MemDisk) -> Fixture {
    reopen(disk, Some((ctx(), pinned()))).expect("the session reopens")
}

impl Fixture {
    /// A row's press naming the current revision, prepared with its token and sent.
    fn crossing(&mut self, row: &str, mut args: BTreeMap<String, Value>) -> Receipt {
        args.insert("revision".to_owned(), self.layer.revision().to_json());
        let press = self.shell.prepare(&self.layer, row, Value::Object(args)).unwrap();
        self.shell.send(&mut self.layer, press)
    }

    fn enter(&mut self) -> Receipt {
        self.crossing(JUMBLE_ON_ROW, BTreeMap::new())
    }

    fn exit(&mut self) -> Receipt {
        self.crossing(JUMBLE_OFF_ROW, BTreeMap::new())
    }

    /// `jumble.twist.menu` for `binding`: with Jumble on, a pose move.
    fn menu_twist(&mut self, binding: &Binding) -> Receipt {
        let Value::Object(mut letter) = binding.to_json() else { panic!("a binding is an object") };
        let cap = letter.remove("cap").expect("a binding has a cap");
        let args = BTreeMap::from([("cap".to_owned(), cap), ("letter".to_owned(), Value::Object(letter))]);
        self.crossing(JUMBLE_TWIST_ROW, args)
    }

    /// `twist.apply` for `binding`: with Jumble off, a retained commit.
    fn twist(&mut self, binding: &Binding) -> Receipt {
        let Value::Object(mut rotation) = binding.to_json() else { panic!("a binding is an object") };
        let cap = rotation.remove("cap").expect("a binding has a cap");
        let args = json::object([("cap", cap), ("rotation", Value::Object(rotation))]);
        let press = self.shell.prepare(&self.layer, TWIST_ROW, args).unwrap();
        self.shell.send(&mut self.layer, press)
    }

    /// Previews `recipe`, presents the preview and commits it.
    fn commit(&mut self, recipe: &Value) -> Receipt {
        let receipt = self.shell.invoke(&mut self.layer, "operation.preview", draft_args(recipe)).unwrap();
        let Status::Running(id) = receipt.status else { panic!("preview not admitted: {:?}", receipt.status) };
        assert_eq!(self.layer.run_jobs(), 1);
        self.frames.present(&mut self.shell, &mut self.layer, id).unwrap();
        let args = json::object([("preview", Value::Int(id as i64))]);
        let press = self.shell.prepare(&self.layer, "operation.execute", args).unwrap();
        self.shell.send(&mut self.layer, press)
    }

    /// The undo or redo of the action on top of its stack.
    fn step(&mut self, direction: Direction) -> Receipt {
        let hint = self.layer.undo_hint(direction).expect("an eligible target");
        let row = if direction == Direction::Undo { "session.undo" } else { "session.redo" };
        self.shell.invoke(&mut self.layer, row, json::object([("hint", hint.to_json())])).unwrap()
    }

    fn set_policy(&mut self, policy: &str) -> Receipt {
        self.shell.invoke(&mut self.layer, POLICY_ROW, json::object([("policy", text(policy))])).unwrap()
    }
}

fn committed(receipt: &Receipt) -> bool {
    matches!(receipt.status, Status::Committed(_))
}

fn completed(receipt: &Receipt) -> bool {
    matches!(receipt.status, Status::Completed(Completion { kind: CompletionKind::Workspace, .. }))
}

/// What a reopen must restore, the epoch aside: the revision, the retained state, the protected set, the undo and
/// redo targets and, with Jumble on, the poses.
fn kept(layer: &CommandLayer<JournalStore>) -> String {
    let mut revision = layer.revision().clone();
    revision.epoch = 0;
    let poses = layer.jumbled().map(|derived| derived.digest(&ctx()));
    format!(
        "{revision:?} {} {:?} {:?} {:?} {poses:?}",
        layer.snapshot().digest(),
        layer.protected().orbits(),
        layer.undo_hint(Direction::Undo),
        layer.undo_hint(Direction::Redo)
    )
}

/// Where a layer stands: its mode, head, state revision and state digest, and whether it holds an episode.
fn position(layer: &CommandLayer<JournalStore>) -> (&'static str, String, u64, String, bool) {
    let r = layer.revision();
    (r.mode, r.head.clone(), r.state, r.digest.clone(), layer.jumbled().is_some())
}

/// The records of a journal whose frames are all whole.
fn records(bytes: &[u8]) -> Vec<Value> {
    let scan = journal::scan(bytes);
    assert_eq!(scan.tail, Tail::Clean, "a journal of whole frames");
    scan.payloads.iter().map(|payload| json::parse(payload).expect("a record is JSON")).collect()
}

/// `records` framed again, each as its canonical bytes.
fn framed(records: &[Value]) -> Vec<u8> {
    records.iter().flat_map(|record| journal::frame(record.canonical().as_bytes())).collect()
}

/// The kind of an action record's transaction; `None` for the session and epoch records.
fn kind(record: &Value) -> Option<&str> {
    record.get("transaction")?.get("kind")?.as_str()
}

/// The field at `path` in `value`.
fn field<'a>(value: &'a mut Value, path: &[&str]) -> &'a mut Value {
    path.iter().fold(value, |value, key| match value {
        Value::Object(fields) => fields.get_mut(*key).unwrap_or_else(|| panic!("no field {key}")),
        other => panic!("{key} of {other:?}"),
    })
}

/// CT-PS1-11 (part: the session journal, whole for the replay of a journaled episode): a crossing journaled under the
/// I_a menu replays only under that menu. Reopened with no menu, or with the I_a items in reversed order (the same
/// set, another identity block), the layer stops in read-only recovery at the revision before the crossing with
/// Jumble off, and the journal keeps the episode, which a later reopen under its own menu restores. A context of
/// another model does not open the session (`KN1:input.identity_mismatch`) and writes nothing; a crossing record
/// edited to another contract revision or another model is not replayed. With Jumble off a jumble item is refused
/// (`KN5:input.menu_word_needs_jumble_on`) and writes nothing.
#[test]
fn ct_ps1_11_a_journaled_episode_replays_only_under_its_menu_model_and_contract() {
    let disk = MemDisk::new();
    let mut f = create(&disk, Some(pinned()));
    let writes = disk.writes();
    let receipt = f.twist(&item(12));
    assert_eq!(receipt.status.code(), Some(MENU_WORD_NEEDS_JUMBLE_ON), "{:?}", receipt.status);
    assert_eq!(disk.writes(), writes, "a refused jumble item writes nothing");
    assert!(committed(&f.commit(&recipe(&[2]))));
    let off = position(&f.layer);
    assert_eq!((off.0, off.4), (MODE, false));
    assert!(committed(&f.enter()));
    assert!(committed(&f.menu_twist(&item(12))));
    let on = kept(&f.layer);
    assert!(f.layer.jumbled().is_some());
    drop(f);
    let journal = disk.durable("journal").expect("a journal");
    let kinds: Vec<_> = records(&journal).iter().filter_map(kind).map(str::to_owned).collect();
    assert_eq!(kinds, ["commit", "crossing", "jumble_move"]);

    let g = reopen_pinned(&disk);
    assert!(g.layer.recovery().is_none(), "{:?}", g.layer.recovery());
    assert_eq!(kept(&g.layer), on);
    drop(g);
    for (menu, what) in [(None, "no menu"), (Some(reversed()), "the reversed I_a order")] {
        let before = disk.durable("journal").expect("a journal");
        let h = reopen(&disk, menu.map(|menu| (ctx(), menu))).expect("the session reopens");
        assert_eq!(h.layer.recovery().map(|error| error.code), Some(code::READ_ONLY_RECOVERY), "{what}");
        assert_eq!(position(&h.layer), off, "{what}");
        drop(h);
        let after = disk.durable("journal").expect("a journal");
        assert!(after.starts_with(&before), "{what}: the journal is only appended to");
    }
    let g = reopen_pinned(&disk);
    assert!(g.layer.recovery().is_none(), "{:?}", g.layer.recovery());
    assert_eq!(kept(&g.layer), on, "the episode survives the reopens that could not replay it");
    drop(g);

    let mut other = Context::load(&repository().join("assets")).expect("the exact model loads");
    other.model.model_id = "0".repeat(64);
    let before = disk.durable("journal");
    match reopen(&disk, Some((Arc::new(other), pinned()))) {
        Err(error) => assert_eq!(error.code, IDENTITY_MISMATCH),
        Ok(_) => panic!("a context of another model opened the session"),
    }
    assert_eq!(disk.durable("journal"), before, "a refused open writes nothing");

    let held = records(&journal);
    assert_eq!(framed(&held), journal, "the records frame again to the journal's bytes");
    let crossing = held.iter().position(|record| kind(record) == Some("crossing")).expect("a crossing");
    for (key, value, what) in [
        ("contract_rev", text("state-contract 2026-11-14 A1-A5"), "another contract revision"),
        ("model_id", text(&"0".repeat(64)), "another model"),
    ] {
        let mut edited = held.clone();
        let entry = field(&mut edited[crossing], &["transaction", "crossing", "identity", key]);
        assert_ne!(*entry, value, "{what}");
        *entry = value;
        disk.overwrite("journal", &framed(&edited));
        let h = reopen_pinned(&disk);
        assert_eq!(h.layer.recovery().map(|error| error.code), Some(code::READ_ONLY_RECOVERY), "{what}");
        assert_eq!(position(&h.layer), off, "{what}");
    }
}

/// CT-PS1-22 (part: the never-on session, whole for its journal): a session that never turns Jumble on keeps no
/// jumbling section. The same retained actions journal the same bytes with the I_a menu pinned as with none; no
/// record is a crossing, a pose move or an exit, and none carries a crossing record; a retained menu press journals a
/// retained commit; and the journal reopens with no menu, so nothing in it needs jumbling support (KN4.5).
#[test]
fn ct_ps1_22_a_never_on_session_journals_no_jumbling_section() {
    let (with_menu, without) = (MemDisk::new(), MemDisk::new());
    let mut a = create(&with_menu, Some(pinned()));
    let mut b = create(&without, None);
    for f in [&mut a, &mut b] {
        assert!(committed(&f.commit(&recipe(&[2]))));
        assert!(committed(&f.commit(&recipe(&[1, -2]))));
        assert!(committed(&f.step(Direction::Undo)));
        assert!(committed(&f.step(Direction::Redo)));
        assert!(completed(&f.set_policy("strict")));
    }
    let journal = with_menu.durable("journal").expect("a journal");
    assert_eq!(Some(&journal), without.durable("journal").as_ref(), "a pinned menu adds no byte");

    assert!(committed(&a.twist(&item(6))));
    let held = records(&with_menu.durable("journal").expect("a journal"));
    for record in &held {
        assert!(!matches!(kind(record), Some("crossing" | "jumble_move" | "exit")), "{}", record.canonical());
        assert!(record.get("transaction").and_then(|t| t.get("crossing")).is_none(), "{}", record.canonical());
    }
    assert_eq!(held.last().and_then(kind), Some("commit"), "a retained menu press is a retained commit");
    let before = kept(&a.layer);
    drop(a);
    let g = reopen(&with_menu, None).expect("the session reopens");
    assert!(g.layer.recovery().is_none(), "{:?}", g.layer.recovery());
    assert_eq!(kept(&g.layer), before);
}

/// How an append fails, and what the power loss after it keeps.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Cut {
    /// The append fails and is cut back: rolled back.
    Error,
    /// The append writes `n` bytes, fails and is cut back: rolled back.
    Partial(usize),
    /// Nothing is written and the cut back fails: unknown, and absent after the power loss.
    Absent,
    /// The append and its sync take effect, the sync reports an error and the cut back fails: unknown, and present.
    Written,
    /// The append takes effect, its sync and the cut back fail, and the power loss keeps `n` bytes of the frame:
    /// unknown, and present only if the whole frame reached the disk.
    Torn(usize),
}

impl Cut {
    fn inject(self, disk: &MemDisk) {
        let next = disk.writes();
        match self {
            Cut::Error => disk.fail_write(next, Fault::Error),
            Cut::Partial(n) => disk.fail_write(next, Fault::Partial(n)),
            Cut::Absent => disk.fail_from(next),
            Cut::Written => {
                disk.fail_write(next + 1, Fault::Persisted);
                disk.fail_write(next + 2, Fault::Error);
            }
            Cut::Torn(_) => disk.fail_from(next + 1),
        }
    }

    fn crash(self, disk: &MemDisk) {
        match self {
            Cut::Torn(n) => disk.crash_keeping("journal", n),
            _ => disk.crash(),
        }
    }
}

/// A session with the I_a menu pinned after a retained commit and the first `n` of: the entry, and a retained item
/// as a pose move.
fn episode(disk: &MemDisk, n: usize) -> Fixture {
    let mut f = create(disk, Some(pinned()));
    assert!(committed(&f.commit(&recipe(&[2]))));
    if n > 0 {
        assert!(committed(&f.enter()));
    }
    if n > 1 {
        assert!(committed(&f.menu_twist(&item(6))));
    }
    f
}

/// CT-PS1-22 (part: an interrupted crossing, whole for the journal's appends and a power loss after each), KN5.23: a
/// failure at the entry, at a pose move or at the exit recovers with the step done or with the session at its last
/// committed revision before it, never half crossed: the reopened mode is Jumble on exactly when the layer holds an
/// episode. A rolled-back append fails the step and is cut back, so the same step sent again commits and survives the
/// power loss; an unknown fate puts the layer in read-only recovery without adopting the step, and the reopen settles
/// it from the journal: a frame that reached the disk whole is the step, and any shorter tail is cut. The disk is
/// `MemDisk`, so a pass is no evidence of a real disk's behaviour in a crash.
#[test]
fn ct_ps1_22_an_interrupted_crossing_recovers_on_or_at_the_last_committed_revision() {
    type Step = fn(&mut Fixture) -> Receipt;
    let steps: [(&str, usize, Step); 3] = [
        ("the entry", 0, |f| f.enter()),
        ("a jumble item as a pose move", 1, |f| f.menu_twist(&item(12))),
        ("the exit", 2, |f| f.exit()),
    ];
    for (what, prefix, step) in steps {
        let clean = MemDisk::new();
        let mut f = episode(&clean, prefix);
        let before = kept(&f.layer);
        let (writes, length) = (clean.writes(), clean.durable("journal").expect("a journal").len());
        assert!(committed(&step(&mut f)), "{what}");
        let after = kept(&f.layer);
        assert_ne!(after, before, "{what}");
        assert_eq!(clean.writes() - writes, 2, "{what}: one append and its sync");
        let frame = clean.durable("journal").expect("a journal").len() - length;

        let mut cuts = vec![Cut::Error, Cut::Partial(1), Cut::Partial(frame / 2), Cut::Absent, Cut::Written];
        cuts.extend([1, 8, frame / 2, frame - 1, frame].map(Cut::Torn));
        for cut in cuts {
            let disk = MemDisk::new();
            let mut f = episode(&disk, prefix);
            assert_eq!(kept(&f.layer), before, "{what}, {cut:?}");
            cut.inject(&disk);
            let receipt = step(&mut f);
            assert_eq!(kept(&f.layer), before, "{what}, {cut:?}: the layer adopts nothing unsettled");
            if matches!(cut, Cut::Error | Cut::Partial(_)) {
                assert!(matches!(receipt.status, Status::Failed(_)), "{what}, {cut:?}: {:?}", receipt.status);
                assert!(f.layer.recovery().is_none(), "{what}, {cut:?}: {:?}", f.layer.recovery());
            } else {
                assert_eq!(receipt.status, Status::UnknownOutcome, "{what}, {cut:?}");
                assert_eq!(f.layer.recovery().map(|error| error.code), Some(code::COMMIT_FATE), "{what}, {cut:?}");
            }
            drop(f);
            cut.crash(&disk);
            let g = reopen_pinned(&disk);
            assert!(g.layer.recovery().is_none(), "{what}, {cut:?}: {:?}", g.layer.recovery());
            let done = cut == Cut::Written || cut == Cut::Torn(frame);
            assert_eq!(kept(&g.layer), if done { after.clone() } else { before.clone() }, "{what}, {cut:?}");
            let on = g.layer.revision().mode == JUMBLE_ON_MODE;
            assert_eq!(on, g.layer.jumbled().is_some(), "{what}, {cut:?}: no half crossing");
        }

        // A rolled-back append is cut back, so the step sent again commits and survives the power loss.
        for cut in [Cut::Error, Cut::Partial(1), Cut::Partial(frame / 2)] {
            let disk = MemDisk::new();
            let mut f = episode(&disk, prefix);
            cut.inject(&disk);
            assert!(matches!(step(&mut f).status, Status::Failed(_)), "{what}, {cut:?}");
            assert!(committed(&step(&mut f)), "{what}, {cut:?}: the step again");
            assert_eq!(kept(&f.layer), after, "{what}, {cut:?}: the step again");
            drop(f);
            cut.crash(&disk);
            let g = reopen_pinned(&disk);
            assert!(g.layer.recovery().is_none(), "{what}, {cut:?}: {:?}", g.layer.recovery());
            assert_eq!(kept(&g.layer), after, "{what}, {cut:?}: the step again, reopened");
        }
    }
}

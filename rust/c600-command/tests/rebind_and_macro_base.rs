//! CT-EV4-07 (source; EV4.8, EV4.9; LP4-8): the solver's rebind command alone changes the applied grip-slot map, and
//! both puzzles use one macro base whose macros keep their identity, alphabet, literal order and exact expansion
//! length, through an undo, a redo and a reopen. Where the frozen text is silent, the tests follow the drafts recorded
//! in `src/grip.rs` and `src/macros.rs` and say so. Key computation is the interpreter's, so only the key contracts a
//! macro pins are checked here (`check_keys`).
//!
//! CT-EV4-07 (whole: the rebind leg on the layer and on the router's slot keys, `c600-input/tests/routing.rs`; the
//! macro base for both puzzles; headless, no Windows evidence).

use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use c600_command::macros::{check_keys, literal_digest, Alphabet, KeyContracts, FULL_DISJOINT_SUPPORT, TRACE_KEY};
use c600_command::{
    code, digest, draft_args, identity_json, Caller, CommandLayer, HeadlessFrames, Invocation, JobOutput, Macro,
    MemoryStore, Overlay, Provenance, Receipt, RecordValue, Secret, Shell, SlotMap, Status, Table, GRIP_REBIND_ROW,
    JUMBLE_ON_ROW, MACRO_SAVE_ROW, SLOT_MAP_RECORD,
};
use c600_jumble::twist::Menu;
use c600_jumble::Context;
use c600_kernel::json::{self, Value};
use c600_kernel::{Kernel, Puzzle};

const SECRET: [u8; 32] = [7; 32];
/// A workspace row of the layout history that an entry serves (ADR-D01.14).
const THEME: &str = "theme.select";
/// The I_a menu's set and order digests.
const SET: &str = "fddc8532b9184bd1b413c1032b5a7a305dd9fd1f10385d87ba6ec4fd06eb958c";
const ORDER: &str = "5a0429d2946fbf58b495d8f300e24a3fa164b1d9518ce4a5c3f4ef0a40625c3c";
/// JI.23's flat payload `[(0,12),(0,12),(0,45)]`: its `ia16-expanded-le/1` digest.
const JI23: &str = "c2ae6cf7df4e002e34b55017aa22a79d7b385830a0cf513f170586410380d309";

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

/// The I_a menu of `research/jumbling/fixtures/wj-I_a.json`, with its base reversed when `reversed`: the same set in
/// another order, so another jumbling identity block.
fn menu(reversed: bool) -> Menu {
    let path = repository().join("research/jumbling/fixtures/wj-I_a.json");
    let fixture = json::parse(&std::fs::read(path).expect("the I_a fixture")).expect("the I_a fixture is JSON");
    let mut record = fixture.get("menu").expect("a menu").clone();
    if let (true, Value::Object(fields)) = (reversed, &mut record) {
        if let Some(Value::Array(base)) = fields.get_mut("base") {
            base.reverse();
        }
    }
    Menu::from_record(&ctx(), &record).expect("the I_a menu")
}

fn text(s: &str) -> Value {
    Value::String(s.to_owned())
}

fn word(letters: &[i64]) -> Value {
    json::parse(format!(r#"[{{"kind": "word", "moves": {letters:?}}}]"#).as_bytes()).unwrap()
}

fn completed(receipt: &Receipt) -> bool {
    matches!(receipt.status, Status::Completed(_))
}

/// An undo or redo step lands as a commit-row receipt.
fn committed(receipt: &Receipt) -> bool {
    matches!(receipt.status, Status::Committed(_))
}

fn refusal(receipt: &Receipt) -> Option<&'static str> {
    receipt.status.code()
}

/// A provenance reader that admits every block.
struct Reader;

impl Provenance for Reader {
    fn check(&self, _block: &[u8]) -> Result<(), c600_command::Error> {
        Ok(())
    }

    fn admit(&self, _block: &[u8]) -> Result<(), c600_command::Error> {
        Ok(())
    }
}

struct Fixture {
    layer: CommandLayer,
    shell: Shell,
    frames: HeadlessFrames,
    sent: u64,
}

fn fixture() -> Fixture {
    let k = kernel();
    let (start, protected) = (k.solved(), k.protected_set(&[]).unwrap());
    let table = Table::h08().with_drafts();
    let mut layer = CommandLayer::new(k, table, Secret::fixed(SECRET), MemoryStore::default(), "s1", start, protected);
    layer.set_provenance(Box::new(Reader));
    Fixture { layer, shell: Shell::new(Secret::fixed(SECRET), "shell"), frames: HeadlessFrames::new(), sent: 0 }
}

/// A reopen of `store`, with the I_a menu pinned before the replay.
fn reopen(store: MemoryStore) -> Fixture {
    let (table, secret) = (Table::h08().with_drafts(), Secret::fixed(SECRET));
    let mut layer = CommandLayer::open_with_menu(kernel(), table, secret, store, ctx(), menu(false)).unwrap();
    layer.set_provenance(Box::new(Reader));
    Fixture { layer, shell: Shell::new(Secret::fixed(SECRET), "shell"), frames: HeadlessFrames::new(), sent: 0 }
}

/// The recovery error a reopen of `store` meets, if any.
fn recovery(store: MemoryStore) -> Option<&'static str> {
    let (table, secret) = (Table::h08().with_drafts(), Secret::fixed(SECRET));
    let layer = CommandLayer::open_with_menu(kernel(), table, secret, store, ctx(), menu(false)).unwrap();
    layer.recovery().map(|error| error.code)
}

/// A transported map of `caps`, slot `n` in frame `f{n}`.
fn map(caps: &[i64]) -> Value {
    Value::Array(
        caps.iter()
            .enumerate()
            .map(|(n, &cap)| json::object([("cap", Value::Int(cap)), ("frame", text(&format!("f{n}")))]))
            .collect(),
    )
}

fn rebind_args(anchor: i64, map: Value, expected: &str) -> Value {
    json::object([("anchor", Value::Int(anchor)), ("expected_map", text(expected)), ("map", map)])
}

fn save_args(name: &str, word: Value, confirm: Option<bool>) -> Value {
    let mut fields = vec![("name".to_owned(), text(name)), ("word".to_owned(), word)];
    if let Some(confirm) = confirm {
        fields.push(("confirm".to_owned(), Value::Bool(confirm)));
    }
    Value::Object(fields.into_iter().collect())
}

fn recipe(letters: &[i64]) -> Value {
    json::object([("recipe", word(letters))])
}

fn menu_word(letters: &[(i64, i64)]) -> Value {
    let letters = letters.iter().map(|&(grip, item)| Value::Array(vec![Value::Int(grip), Value::Int(item)]));
    json::object([("menu", Value::Array(letters.collect()))])
}

impl Fixture {
    fn invoke(&mut self, row: &str, args: Value) -> Receipt {
        self.shell.invoke(&mut self.layer, row, args).unwrap()
    }

    fn rebind(&mut self, caps: &[i64]) -> Receipt {
        let expected = self.layer.slot_map_digest();
        self.invoke(GRIP_REBIND_ROW, rebind_args(17, map(caps), &expected))
    }

    fn save(&mut self, name: &str, word: Value) -> Receipt {
        self.invoke(MACRO_SAVE_ROW, save_args(name, word, Some(true)))
    }

    fn press(&mut self, keys: &str) -> Receipt {
        self.shell.press(&mut self.layer, keys, false).unwrap().unwrap()
    }

    fn commit(&mut self, letters: &[i64]) {
        let receipt = self.invoke("operation.preview", draft_args(&word(letters)));
        let Status::Running(id) = receipt.status else { panic!("preview not admitted: {:?}", receipt.status) };
        assert_eq!(self.layer.run_jobs(), 1);
        self.frames.present(&mut self.shell, &mut self.layer, id).unwrap();
        let receipt = self.invoke("operation.execute", json::object([("preview", Value::Int(id as i64))]));
        assert!(matches!(receipt.status, Status::Committed(_)), "{:?}", receipt.status);
    }

    /// `row` invoked by a run.
    fn runtime(&mut self, row: &str, args: Value) -> Receipt {
        self.sent += 1;
        let invocation = Invocation {
            command: format!("run/{}", self.sent),
            row: row.to_owned(),
            args,
            token: None,
            epoch: self.layer.revision().epoch,
            budget: None,
        };
        self.layer.invoke(&Caller::Runtime { run: "run".to_owned() }, invocation)
    }

    /// `jumble.on`, naming the current revision.
    fn enter(&mut self) {
        let revision = self.layer.revision().to_json();
        let press = self.shell.prepare(&self.layer, JUMBLE_ON_ROW, json::object([("revision", revision)])).unwrap();
        let receipt = self.shell.send(&mut self.layer, press);
        assert!(matches!(receipt.status, Status::Committed(_)), "{:?}", receipt.status);
    }

    fn caps(&self) -> Option<Vec<u32>> {
        self.layer.slot_map().map(|map| map.slots.iter().map(|slot| slot.cap).collect())
    }
}

/// CT-EV4-07, the rebind leg (EV4.8; PS1.26; DT-M1 K2, Q3 (a)): before any rebind no map is applied, so the host's
/// default applies. A commit, a camera row, a preview and its cancel, and a restart derive no map. `grip.rebind`
/// applies the transported map with its anchor, caps, frames and model, as one workspace action that undo and redo
/// step and a reopen replays; only a later rebind replaces it.
#[test]
fn only_the_rebind_command_changes_the_applied_map() {
    let mut f = fixture();
    assert_eq!(f.layer.slot_map(), None);
    assert_eq!(f.layer.slot_map_digest(), digest(&Value::Null));

    f.commit(&[1, 2]);
    let _ = f.shell.invoke(&mut f.layer, "camera.rotate", json::object([("view", text("global"))]));
    let preview = f.invoke("operation.preview", draft_args(&word(&[3])));
    let Status::Running(id) = preview.status else { panic!("{:?}", preview.status) };
    f.layer.run_jobs();
    f.frames.present(&mut f.shell, &mut f.layer, id).unwrap();
    assert!(completed(&f.invoke("operation.preview.cancel", json::object([("preview", Value::Int(id as i64))]))));
    assert_eq!(f.layer.slot_map(), None, "no commit, camera or preview derives a map");
    let mut f = reopen(f.layer.store().clone());
    assert_eq!(f.layer.slot_map(), None, "a restart derives none");

    let rebound = f.rebind(&[0, 7, 123, 599]);
    assert!(completed(&rebound), "{:?}", rebound.status);
    let applied = f.layer.slot_map().expect("the rebind applied a map");
    assert_eq!((applied.anchor, applied.model.as_str()), (17, kernel().identity().model_id.as_str()));
    assert_eq!(f.caps(), Some(vec![0, 7, 123, 599]));
    assert_eq!(applied.slots[2].frame, text("f2"), "the transported frames are kept");
    assert_eq!(f.layer.slot_map_digest(), digest(&applied.to_payload()));

    f.commit(&[4]);
    assert_eq!(f.layer.slot_map(), Some(applied.clone()), "a commit derives no new map");
    let f2 = reopen(f.layer.store().clone());
    assert_eq!(f2.layer.slot_map(), Some(applied.clone()), "a reopen replays the rebind");

    assert!(committed(&f.press("Ctrl+KeyZ")), "undo of the commit");
    assert!(committed(&f.press("Ctrl+KeyZ")), "undo of the rebind, a workspace inverse in the move history");
    assert_eq!(f.layer.slot_map(), None, "undo restores the default");
    assert!(committed(&f.press("Ctrl+KeyY")));
    assert_eq!(f.layer.slot_map(), Some(applied));

    assert!(completed(&f.rebind(&[5])));
    assert_eq!(f.caps(), Some(vec![5]), "a later rebind replaces the map");
    assert_eq!(reopen(f.layer.store().clone()).caps(), Some(vec![5]));
}

/// A rebind is checked before it writes (EV4.8; draft forms): an `expected_map` that is not the applied map's digest
/// is `KN5:stale.revision_changed`, and an anchor or a map of another form is `KN5:input.argument_form`; nothing is
/// written.
#[test]
fn a_raced_or_malformed_rebind_writes_nothing() {
    let mut f = fixture();
    assert!(completed(&f.rebind(&[1, 2])));
    let before = (f.layer.slot_map(), f.layer.revision().clone());
    let stale = digest(&Value::Null);
    assert_eq!(refusal(&f.invoke(GRIP_REBIND_ROW, rebind_args(17, map(&[3]), &stale))), Some(code::REVISION_CHANGED));
    let current = f.layer.slot_map_digest();
    let slot = |fields: Vec<(&str, Value)>| {
        Value::Array(vec![Value::Object(fields.into_iter().map(|(k, v)| (k.to_owned(), v)).collect())])
    };
    for (anchor, bad) in [
        (17, map(&[600])),
        (17, map(&[-1])),
        (17, Value::Array(Vec::new())),
        (17, map(&vec![0; 601])),
        (17, slot(vec![("cap", Value::Int(1))])),
        (17, slot(vec![("cap", Value::Int(1)), ("frame", Value::Null), ("x", Value::Null)])),
        (17, slot(vec![("cap", text("1")), ("frame", Value::Null)])),
        (c600_kernel::PIECES as i64, map(&[1])),
        (-1, map(&[1])),
    ] {
        let refused = f.invoke(GRIP_REBIND_ROW, rebind_args(anchor, bad.clone(), &current));
        assert_eq!(refusal(&refused), Some(code::ARGUMENT_FORM), "{anchor} {bad:?}");
    }
    assert_eq!((f.layer.slot_map(), f.layer.revision().clone()), before);
}

/// Only `grip.rebind` writes the applied map and only `macro.save` writes the macro base (EV4.8, EV4.9; B7):
/// `workspace.set` refuses their names, a run reaches neither row, a served row's job that writes one of their records
/// fails, and a reopen does not admit a journal whose write of one comes from anything but its own row from the shell
/// with a record that reads.
#[test]
fn no_other_path_writes_the_grip_map_or_the_macro_base() {
    let mut f = fixture();
    for name in [SLOT_MAP_RECORD, "grip/other", "macro/m"] {
        let args = json::object([("payload", text("x")), ("provenance", text("block")), ("record", text(name))]);
        assert_eq!(refusal(&f.invoke("workspace.set", args)), Some(code::ARGUMENT_FORM), "{name}");
    }
    let rebind = rebind_args(17, map(&[1]), &f.layer.slot_map_digest());
    assert_eq!(refusal(&f.runtime(GRIP_REBIND_ROW, rebind)), Some(code::NOT_GRANTED));
    assert_eq!(refusal(&f.runtime(MACRO_SAVE_ROW, save_args("m", recipe(&[1]), Some(true)))), Some(code::NOT_GRANTED));
    assert_eq!(f.layer.records().len(), 0);

    // A served workspace row's job writes no record of either.
    f.layer.serve_row(THEME, Overlay { module: "tools/design".into(), entry: "theme".into() }).unwrap();
    for name in [SLOT_MAP_RECORD, "macro/m"] {
        let started = f.invoke(THEME, json::object([("family", text("dawn"))]));
        let Status::Running(job) = started.status else { panic!("{THEME}: {:?}", started.status) };
        assert_eq!(f.layer.take_program_job().map(|taken| taken.job), Some(job));
        let value = RecordValue { payload: Value::Null, provenance: "block".into() };
        let output = JobOutput { value: Value::Null, views: Vec::new(), records: [(name.into(), value)].into() };
        let done = f.layer.finish_program_job_with(job, Ok(output)).unwrap();
        assert_eq!(refusal(&done), Some(code::NAMED_OBJECT_INVALID), "{name}");
    }
    assert_eq!(
        f.layer.records().keys().filter(|name| name.starts_with("grip/") || name.starts_with("macro/")).count(),
        0
    );

    // Forged journals: a job's completion that writes either or that names either row (with a record of its own name),
    // a rebind relabelled as another row or caller, a write of another record under the row, and a save without its
    // stored confirmation or with a record that does not read.
    let started = f.invoke(THEME, json::object([("family", text("dawn"))]));
    let Status::Running(job) = started.status else { panic!("{THEME}: {:?}", started.status) };
    assert_eq!(f.layer.take_program_job().map(|taken| taken.job), Some(job));
    let value = RecordValue { payload: text("dawn"), provenance: "block".into() };
    let output = JobOutput { value: Value::Null, views: Vec::new(), records: [("theme/x".into(), value)].into() };
    assert!(completed(&f.layer.finish_program_job_with(job, Ok(output)).unwrap()));
    assert!(completed(&f.rebind(&[1])));
    assert!(completed(&f.save("m", recipe(&[1, 2]))));
    let store = f.layer.store().clone();
    assert_eq!(recovery(store.clone()), None);
    let (theme_at, rebind_at, save_at) = (store.log.len() - 3, store.log.len() - 2, store.log.len() - 1);
    let forgeries: Vec<Box<dyn Fn(&mut MemoryStore)>> = vec![
        Box::new(move |s| s.log[theme_at].records[0].name = SLOT_MAP_RECORD.into()),
        Box::new(move |s| s.log[theme_at].records[0].name = "macro/m".into()),
        Box::new(move |s| s.log[theme_at].row = GRIP_REBIND_ROW.into()),
        Box::new(move |s| s.log[theme_at].row = MACRO_SAVE_ROW.into()),
        Box::new(move |s| s.log[rebind_at].row = "workspace.set".into()),
        Box::new(move |s| s.log[rebind_at].caller = Caller::Runtime { run: "r".into() }),
        Box::new(move |s| s.log[rebind_at].record.as_mut().unwrap().name = "grip/other".into()),
        Box::new(move |s| s.log[rebind_at].record.as_mut().unwrap().value.as_mut().unwrap().payload = Value::Null),
        Box::new(move |s| s.log[save_at].confirm = false),
        Box::new(move |s| s.log[save_at].row = "workspace.set".into()),
        Box::new(move |s| s.log[save_at].record.as_mut().unwrap().name = "macro/other".into()),
        Box::new(move |s| s.log[save_at].record.as_mut().unwrap().value.as_mut().unwrap().provenance = "x".into()),
    ];
    for (n, forge) in forgeries.iter().enumerate() {
        let mut forged = store.clone();
        forge(&mut forged);
        assert_eq!(recovery(forged), Some(code::UNADMITTED_JOURNAL_ACTION), "forgery {n}");
    }
}

/// CT-EV4-07, the macro base (EV4.9; JI.18 to JI.20; DT-J1 E1, DT-M1 K6): one base holds a Full word and a word of the
/// pinned menu's letters, saved with Jumble off and with Jumble on. Each macro keeps its alphabet, its own identity
/// block, its literal letters in chronological order, its exact expanded length, its literal digest and its start,
/// and the base is the same after a reopen. A recipe's named parts (a star) stay in the source while the letters are
/// its flat expansion. Saving executes nothing.
#[test]
fn both_puzzles_use_one_macro_base() {
    let mut f = fixture();
    f.layer.pin_menu(ctx(), menu(false)).unwrap();
    let star = json::parse(br#"[{"kind": "word", "moves": [5]}, {"kind": "star", "orbit": 0, "node": 1, "sign": 1}]"#)
        .unwrap();
    let parts = kernel().recipe(&star).unwrap();
    let expansion: Vec<i64> = kernel().letters(&parts).unwrap().into_iter().map(i64::from).collect();
    let start = f.layer.revision().digest.clone();

    assert!(completed(&f.save("full", recipe(&[3, -7, 12]))));
    assert!(completed(&f.save("stars", json::object([("recipe", star.clone())]))));
    assert!(completed(&f.save("ji23", menu_word(&[(0, 12), (0, 12), (0, 45)]))));
    assert_eq!(f.layer.revision().digest, start, "saving executes nothing");
    f.enter();
    let poses = f.layer.revision().digest.clone();
    assert!(completed(&f.save("menu-on", menu_word(&[(599, 1), (7, 59)]))));
    assert!(completed(&f.save("full-on", recipe(&[-1]))));

    let full = kernel().identity();
    let jumbling = ctx().identity(Some(&menu(false)));
    let base = f.layer.macro_base();
    let summary: Vec<_> = base
        .iter()
        .map(|m| (m.name.as_str(), m.alphabet, m.identity.puzzle, m.letters.clone(), m.length, m.start.clone()))
        .collect();
    assert_eq!(
        summary,
        [
            ("full", Alphabet::Full, Puzzle::Full, vec![3, -7, 12], 3, start.clone()),
            ("full-on", Alphabet::Full, Puzzle::Full, vec![-1], 1, poses.clone()),
            ("ji23", Alphabet::Menu, Puzzle::Jumbling, vec![12, 12, 45], 3, start.clone()),
            ("menu-on", Alphabet::Menu, Puzzle::Jumbling, vec![599 * 60 + 1, 7 * 60 + 59], 2, poses),
            ("stars", Alphabet::Full, Puzzle::Full, expansion.clone(), expansion.len() as u64, start),
        ]
    );
    let by = |name: &str| base.iter().find(|m| m.name == name).unwrap().clone();
    assert_eq!(by("full").identity, full);
    assert_eq!(by("ji23").identity, jumbling);
    assert_eq!(
        (by("ji23").identity.menu_id.as_deref(), by("ji23").identity.menu_order.as_deref()),
        (Some(SET), Some(ORDER))
    );
    assert_eq!(by("ji23").literal, JI23, "JI.23's flat payload and digest");
    assert_eq!(by("full").literal, literal_digest(Alphabet::Full, &[3, -7, 12]));
    assert_eq!(by("stars").length, parts.primitive_count() as u64, "the exact length of the named parts' expansion");
    assert_eq!(by("stars").source, json::object([("recipe", star)]), "the source keeps the named parts");
    assert_eq!(by("full").keys.relation, FULL_DISJOINT_SUPPORT);
    assert_eq!(by("ji23").keys.relation, c600_jumble::locality::JUMBLE_LOCALITY);
    let record = f.layer.record("macro/full").unwrap();
    assert_eq!(record.payload, by("full").to_payload());
    assert_eq!(record.payload.get("identity"), Some(&identity_json(&full)));

    let reopened = reopen(f.layer.store().clone());
    assert_eq!(reopened.layer.macro_base(), base, "a reopen keeps the base");
}

/// A save is refused, writing nothing (EV4.9; H-08's `confirm` gate; JI.19; drafts in `src/macros.rs`): without
/// `confirm: true`, a menu word without a pinned menu, a letter outside the menu or JI.19's carrier, a word of another
/// form, an empty name, and a recipe the kernel refuses. A save under a used name writes the macro's next version, and
/// undo and redo step a save as a workspace inverse.
#[test]
fn a_save_is_checked_and_undone_as_one_workspace_action() {
    let mut f = fixture();
    let refused = |f: &mut Fixture, args: Value| refusal(&f.invoke(MACRO_SAVE_ROW, args));
    assert_eq!(refused(&mut f, save_args("m", recipe(&[1]), None)), Some(code::CONFIRMATION_REQUIRED));
    assert_eq!(refused(&mut f, save_args("m", recipe(&[1]), Some(false))), Some(code::CONFIRMATION_REQUIRED));
    assert_eq!(refused(&mut f, save_args("m", menu_word(&[(0, 1)]), Some(true))), Some(code::ROW_NOT_BUILT));
    f.layer.pin_menu(ctx(), menu(false)).unwrap();
    for word in [
        menu_word(&[(600, 1)]),
        menu_word(&[(0, 60)]),
        menu_word(&[(-1, 0)]),
        json::object([("menu", Value::Array(vec![Value::Int(12)]))]),
        json::object([("recipe", word(&[1])), ("menu", Value::Array(Vec::new()))]),
        text("1 2"),
    ] {
        assert_eq!(refused(&mut f, save_args("m", word.clone(), Some(true))), Some(code::ARGUMENT_FORM), "{word:?}");
    }
    assert_eq!(refused(&mut f, save_args("", recipe(&[1]), Some(true))), Some(code::ARGUMENT_FORM));
    // A stored word's form reads (BND.32; `a_macro_of_a_stored_word_keeps_the_words_block`): a missing record is no word.
    let missing = save_args("m", json::object([("record", text("r"))]), Some(true));
    assert_eq!(refused(&mut f, missing), Some(code::NAMED_OBJECT_INVALID));
    for word in [json::object([("record", Value::Int(5))]), json::object([("record", text("r")), ("x", Value::Int(1))])]
    {
        assert_eq!(refused(&mut f, save_args("m", word.clone(), Some(true))), Some(code::ARGUMENT_FORM), "{word:?}");
    }
    assert!(refused(&mut f, save_args("m", recipe(&[0]), Some(true))).is_some(), "the kernel refuses letter 0");
    assert_eq!(f.layer.macro_base(), Vec::<Macro>::new());

    assert!(completed(&f.save("m", recipe(&[1]))));
    assert!(completed(&f.save("m", recipe(&[2, 3]))));
    assert_eq!(f.layer.records()["macro/m"].version, 2, "a save under a used name writes the next version");
    assert_eq!(f.layer.macro_base()[0].letters, [2, 3]);
    assert!(committed(&f.press("Ctrl+KeyZ")));
    assert_eq!(f.layer.macro_base()[0].letters, [1]);
    assert!(committed(&f.press("Ctrl+KeyZ")));
    assert_eq!(f.layer.macro_base(), Vec::<Macro>::new());
    assert!(committed(&f.press("Ctrl+KeyY")));
    assert_eq!(f.layer.macro_base()[0].letters, [1]);
}

/// The key contracts (EV4.9, LG2.23; LB1.10): two macros' keys are compared only under the same alphabet, identity
/// block (menu order included), key kinds and versions, and independence contract; otherwise
/// `LB1:input.key_contract_mismatch`, with no equality or ordering result. Equal contracts compare nothing themselves.
#[test]
fn keys_compare_only_under_one_contract() {
    let mut f = fixture();
    f.layer.pin_menu(ctx(), menu(false)).unwrap();
    assert!(completed(&f.save("a", recipe(&[1, 2]))));
    assert!(completed(&f.save("b", recipe(&[2, 1]))));
    assert!(completed(&f.save("m", menu_word(&[(0, 12)]))));
    let base = f.layer.macro_base();
    let (a, b, m) = (&base[0], &base[1], &base[2]);
    assert_eq!(check_keys(a, b), Ok(()));
    assert_eq!(check_keys(a, m).unwrap_err().code, code::KEY_CONTRACT_MISMATCH);

    let mut reordered = m.clone();
    reordered.identity = ctx().identity(Some(&menu(true)));
    assert_eq!(check_keys(m, &reordered).unwrap_err().code, code::KEY_CONTRACT_MISMATCH, "another menu order");
    for keys in [
        KeyContracts { word: "word-key/2".into(), ..a.keys.clone() },
        KeyContracts { trace: "trace-key/2".into(), ..a.keys.clone() },
        KeyContracts { relation: "jumble-locality/1".into(), ..a.keys.clone() },
    ] {
        let other = Macro { keys, ..b.clone() };
        assert_eq!(check_keys(a, &other).unwrap_err().code, code::KEY_CONTRACT_MISMATCH);
    }
    assert_eq!(a.keys.trace, TRACE_KEY);
    assert!(Macro::from_payload(&Macro { keys: m.keys.clone(), ..a.clone() }.to_payload()).is_none());
    assert!(SlotMap::from_payload(&Value::Null).is_none());
}

fn set_args(record: &str, payload: Value, block: &str) -> Value {
    json::object([("payload", payload), ("provenance", text(block)), ("record", text(record))])
}

/// A record payload of plain letters, as M6 stores a word.
fn plain(letters: &[i64]) -> Value {
    Value::Array(letters.iter().map(|&letter| Value::Int(letter)).collect())
}

fn stored(record: &str) -> Value {
    json::object([("record", text(record))])
}

/// CT-BND-03 (part: the macro route; the workspace record and file routes are in `c600-run/tests/boundary.rs`, the
/// recording and both conversions in `c600-durable/tests/ct_bnd_03_recording_blocks.rs`). BND.32 and PS1.16; draft:
/// `macro.save` of a stored word, `{"record": r}`, saves the record's letters as a Full macro whose record keeps the
/// record's provenance block, and whose source pins the record and its content digest. A reopen admits that write only
/// against the record as the journal holds it at that point: a macro with another block, with the row's own block, or
/// saved from a record whose earlier write the journal changed, is an unadmitted action. Without the host's reader,
/// from a missing record, from a record that holds no word, and with letters the kernel refuses, the save is refused
/// and writes nothing.
#[test]
fn a_macro_of_a_stored_word_keeps_the_words_block() {
    let mut f = fixture();
    assert!(completed(&f.invoke("workspace.set", set_args("word", plain(&[1, 2]), "block-w"))));
    let held = f.layer.record("word").unwrap().clone();
    let receipt = f.save("m", stored("word"));
    assert!(completed(&receipt), "{:?}", receipt.status);
    let value = f.layer.record("macro/m").unwrap().clone();
    assert_eq!(value.provenance, "block-w", "the macro keeps the stored word's block");
    let saved = Macro::from_payload(&value.payload).unwrap();
    assert_eq!((saved.alphabet, saved.letters.clone(), saved.length), (Alphabet::Full, vec![1, 2], 2));
    assert_eq!(saved.source, c600_command::macros::stored_source("word", &held.digest()));
    assert_eq!(saved.stored(), Some(("word", held.digest().as_str())));
    assert!(completed(&f.save("inline", recipe(&[1, 2]))), "an inline word keeps the row's own block");
    assert_eq!(f.layer.record("macro/inline").unwrap().provenance, "macro.save");

    let store = f.layer.store().clone();
    assert_eq!(recovery(store.clone()), None);
    let reopened = reopen(store.clone());
    assert_eq!(reopened.layer.record("macro/m"), Some(&value), "a reopen keeps the macro and its block");
    let (set_at, save_at) = (store.log.len() - 3, store.log.len() - 2);
    let forgeries: Vec<Box<dyn Fn(&mut MemoryStore)>> = vec![
        Box::new(move |s| s.log[save_at].record.as_mut().unwrap().value.as_mut().unwrap().provenance = "x".into()),
        Box::new(move |s| {
            s.log[save_at].record.as_mut().unwrap().value.as_mut().unwrap().provenance = "macro.save".into()
        }),
        Box::new(move |s| s.log[set_at].record.as_mut().unwrap().value.as_mut().unwrap().provenance = "other".into()),
        // Other letters under the record's real block: a macro that reads, but not of the record's word.
        Box::new(move |s| {
            let value = s.log[save_at].record.as_mut().unwrap().value.as_mut().unwrap();
            let mut read = Macro::from_payload(&value.payload).unwrap();
            read.letters = vec![3];
            read.length = 1;
            read.literal = literal_digest(Alphabet::Full, &read.letters);
            value.payload = read.to_payload();
        }),
    ];
    for (n, forge) in forgeries.iter().enumerate() {
        let mut forged = store.clone();
        forge(&mut forged);
        assert_eq!(recovery(forged), Some(code::UNADMITTED_JOURNAL_ACTION), "forgery {n}");
    }

    // Refusals write nothing.
    assert!(completed(&f.invoke("workspace.set", set_args("text", text("not a word"), "block-t"))));
    assert!(completed(&f.invoke("workspace.set", set_args("zero", plain(&[0]), "block-z"))));
    let records = f.layer.records().clone();
    assert_eq!(refusal(&f.save("a", stored("missing"))), Some(code::NAMED_OBJECT_INVALID));
    assert_eq!(refusal(&f.save("b", stored("text"))), Some(code::NAMED_OBJECT_INVALID));
    assert!(refusal(&f.save("c", stored("zero"))).is_some(), "a letter the kernel refuses");
    assert_eq!(f.layer.records(), &records);
    let k = kernel();
    let (start, protected) = (k.solved(), k.protected_set(&[]).unwrap());
    let table = Table::h08().with_drafts();
    let layer = CommandLayer::new(k, table, Secret::fixed(SECRET), MemoryStore::default(), "s2", start, protected);
    let mut bare =
        Fixture { layer, shell: Shell::new(Secret::fixed(SECRET), "shell"), frames: HeadlessFrames::new(), sent: 0 };
    assert_eq!(refusal(&bare.save("m", stored("word"))), Some(code::ROW_NOT_BUILT), "no reader");
    assert!(bare.layer.record("macro/m").is_none());
}

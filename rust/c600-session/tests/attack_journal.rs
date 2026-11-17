//! Attack tests on the reopen of a journal that was edited outside the engine (owner-requested attack of the command
//! layer, class B: the replay of stored data; attacker 2, stored data edited or written by another build). The
//! journal's frames carry a SHA-256 of their payload and no key (`src/journal.rs`), and a record reads when its bytes
//! are canonical and its fields are the known ones (`src/command_store.rs`), so anything that can write the file can
//! write a record that reads. The command layer's replay then decides what a reopened session is.
//!
//! The frozen text is silent on what a replay must check again. These tests take the reading under which the boundary
//! holds against stored data: a journal that records an action no solver action could have produced does not reopen as
//! an intact, writable session. The other reading (KN4.1: the journal is the engine's own file and is trusted as
//! written) makes B7 the intended behaviour. The findings file gives both and decides none.
//!
//! Every test uses fresh memory disks and no other session. Run the ignored one with `-- --include-ignored`.

mod common;

use c600_command::{draft_args, CommandLayer, HeadlessFrames, Receipt, Secret, Shell, Status, Table};
use c600_kernel::json::{self, Value};
use c600_kernel::ProtectedSet;
use c600_session::disk::MemDisk;
use c600_session::journal::{self, Tail};
use c600_session::JournalStore;

const JOURNAL: &str = "journal";

fn parse(text: &str) -> Value {
    json::parse(text.as_bytes()).unwrap()
}

fn word(letters: &[i64]) -> Value {
    parse(&format!(r#"[{{"kind": "word", "moves": {letters:?}}}]"#))
}

/// One opener of a session on a memory disk: the command layer over the journal store, with its shell and frames.
struct Opener {
    layer: CommandLayer<JournalStore>,
    shell: Shell,
    frames: HeadlessFrames,
}

/// A new session at the solved state on an empty memory disk.
fn create(disk: &MemDisk) -> Opener {
    let k = common::kernel();
    let store = JournalStore::create(Box::new(disk.clone())).expect("an empty disk takes a session");
    let secret = Secret::fixed([7; 32]);
    let layer = CommandLayer::new(k.clone(), Table::h08(), secret, store, "s1", k.solved(), ProtectedSet::none());
    assert!(layer.recovery().is_none(), "{:?}", layer.recovery());
    Opener { layer, shell: Shell::new(Secret::fixed([7; 32]), "shell"), frames: HeadlessFrames::new() }
}

/// Opens the session the disk holds, or gives the code the open is refused with.
fn open(disk: &MemDisk) -> Result<Opener, &'static str> {
    let store = JournalStore::open(Box::new(disk.clone())).map_err(|e| e.code)?;
    let secret = Secret::fixed([9; 32]);
    let layer = CommandLayer::open(common::kernel(), Table::h08(), secret, store).map_err(|e| e.code)?;
    Ok(Opener { layer, shell: Shell::new(Secret::fixed([9; 32]), "shell"), frames: HeadlessFrames::new() })
}

impl Opener {
    /// The solver previews `recipe`, sees it and presses the commit chord; the press's receipt.
    fn commit(&mut self, recipe: &Value) -> Receipt {
        let receipt = self.shell.invoke(&mut self.layer, "operation.preview", draft_args(recipe)).unwrap();
        let Status::Running(id) = receipt.status else { return receipt };
        self.layer.run_jobs();
        if let Err(error) = self.frames.present(&mut self.shell, &mut self.layer, id) {
            panic!("the preview is not presented: {error:?}");
        }
        let args = json::object([("preview", Value::Int(id as i64))]);
        let press = self.shell.prepare(&self.layer, "operation.execute", args).unwrap();
        self.shell.send(&mut self.layer, press)
    }

    /// The solver sets the protected orbits, confirming a release when `confirm` is set.
    fn protect(&mut self, orbits: &str, confirm: bool) -> Receipt {
        let args = match confirm {
            true => json::object([("orbits", parse(orbits)), ("confirm", Value::Bool(true))]),
            false => json::object([("orbits", parse(orbits))]),
        };
        self.shell.invoke(&mut self.layer, "protection.orbit.set", args).unwrap()
    }

    fn head(&self) -> String {
        self.layer.snapshot().digest().to_owned()
    }
}

/// The journal's records, in order.
fn records(disk: &MemDisk) -> Vec<Value> {
    let bytes = disk.durable(JOURNAL).expect("the disk holds a journal");
    let scan = journal::scan(&bytes);
    assert_eq!(scan.tail, Tail::Clean);
    scan.payloads.iter().map(|payload| json::parse(payload).expect("a record")).collect()
}

/// Replaces the journal with these records, each framed as the store frames it.
fn write(disk: &MemDisk, records: &[Value]) {
    let mut bytes = Vec::new();
    for record in records {
        bytes.extend(journal::frame(record.canonical().as_bytes()));
    }
    disk.overwrite(JOURNAL, &bytes);
}

fn field<'a>(value: &'a mut Value, name: &str) -> &'a mut Value {
    match value {
        Value::Object(fields) => fields.get_mut(name).unwrap_or_else(|| panic!("no field `{name}`")),
        other => panic!("not an object: {other:?}"),
    }
}

/// The transaction of an action record.
fn transaction(record: &mut Value) -> &mut Value {
    field(record, "transaction")
}

/// What a reopen of the edited journal is: `None` when it is refused or read-only, or the intact session's head, its
/// protected orbits and the callers its journal names.
fn reopened(disk: &MemDisk) -> Option<String> {
    let opener = open(disk).ok()?;
    if opener.layer.recovery().is_some() {
        return None;
    }
    Some(format!("head {}, protected {:?}", &opener.head()[..8], opener.layer.protected().orbits()))
}

fn committed(receipt: &Receipt) -> bool {
    matches!(receipt.status, Status::Committed(_))
}

/// Finding B7 (class B, the replay of stored data; attacker 2). Rule, under the reading that the boundary holds
/// against stored data: only the solver's own action commits (KN5.11 [B]), a runtime caller reaches no commit row
/// (KN5.17 [B]), the active protection gates every commit (ONE.78 [B]) and a release needs the solver's confirmation
/// (ONE.47 [B], LI.26); so a journal that records the contrary is not the record of a session and does not reopen
/// intact. The replay derives a stored commit under no protection and compares none of the stored caller, row, command
/// or confirmation (`c600-command/src/layer.rs`, `replay` and `agrees`).
///
/// Three edits of an honest journal, each framed as the store frames it:
/// (a) the commit's caller is a run;
/// (b) after the journal's protection of orbit 0, a commit of a word that moves orbit 0 (the live layer refuses this
///     commit with `KN2:protection.orbit_would_move`, which the test checks first);
/// (c) a release of orbit 0 whose confirmation is false (the live layer refuses it with
///     `KN5:input.confirmation_required`, which the test checks first).
///
/// Observed on 4c20ce30: all three reopen as an intact, writable session; (b) reopens with orbit 0 protected and its
/// pieces moved. The exact answers are in the assertion's message.
#[test]
#[ignore = "finding B7: fails while an edited journal that no solver action could have written reopens intact"]
fn b7_a_journal_that_records_what_no_solver_action_could_do_does_not_reopen_intact() {
    let mut accepted = Vec::new();

    // An honest session: one commit of the word [1].
    let disk = MemDisk::new();
    let mut honest = create(&disk);
    assert!(committed(&honest.commit(&word(&[1]))));
    drop(honest);
    let honest = records(&disk);
    assert_eq!(honest.len(), 2);

    // (a) The commit's caller is a run.
    let mut edited = honest.clone();
    *field(transaction(&mut edited[1]), "caller") = parse(r#"{"runtime": "run-1"}"#);
    *field(transaction(&mut edited[1]), "command") = Value::String("run-1/1".into());
    write(&disk, &edited);
    if let Some(session) = reopened(&disk) {
        accepted.push(format!("(a) a commit the journal gives to a run reopens intact: {session}"));
    }

    // (b) A commit that moves a protected orbit. The live layer refuses it.
    let disk = MemDisk::new();
    let mut live = create(&disk);
    assert!(matches!(live.protect("[0]", false).status, Status::Completed(_)));
    assert_eq!(live.commit(&word(&[1])).status.code(), Some("KN2:protection.orbit_would_move"));
    drop(live);
    let mut edited = records(&disk);
    assert_eq!(edited.len(), 2);
    let mut commit = honest[1].clone();
    let t = transaction(&mut commit);
    *field(t, "event") = Value::String("e2".into());
    *field(t, "order") = Value::Int(2);
    *field(t, "orbits") = parse("[0]");
    let revision = field(t, "revision");
    *field(revision, "head") = Value::String("e2".into());
    *field(revision, "workspace") = Value::Int(1);
    *field(revision, "protection") = Value::Int(1);
    edited.push(commit);
    write(&disk, &edited);
    if let Some(session) = reopened(&disk) {
        accepted.push(format!("(b) a commit that moves protected orbit 0 reopens intact: {session}"));
    }

    // (c) A release without the solver's confirmation. The live layer refuses it.
    let disk = MemDisk::new();
    let mut live = create(&disk);
    assert!(matches!(live.protect("[0]", false).status, Status::Completed(_)));
    assert_eq!(live.protect("[]", false).status.code(), Some("KN5:input.confirmation_required"));
    assert!(matches!(live.protect("[]", true).status, Status::Completed(_)));
    drop(live);
    let mut edited = records(&disk);
    assert_eq!(edited.len(), 3);
    assert_eq!(*field(transaction(&mut edited[2]), "confirm"), Value::Bool(true));
    *field(transaction(&mut edited[2]), "confirm") = Value::Bool(false);
    write(&disk, &edited);
    if let Some(session) = reopened(&disk) {
        accepted.push(format!("(c) a release with no confirmation reopens intact: {session}"));
    }

    assert!(accepted.is_empty(), "{accepted:#?}");
}

/// Guard (class B, refuted): the replay catches an edit that changes what an action does. Another word under the
/// stored digests, another end state, and a skipped action order each reopen read-only or not at all, and the honest
/// journal reopens intact.
#[test]
fn an_edit_that_changes_what_an_action_does_is_caught_at_the_reopen() {
    let disk = MemDisk::new();
    let mut honest = create(&disk);
    assert!(committed(&honest.commit(&word(&[1]))));
    drop(honest);
    let honest = records(&disk);
    assert!(reopened(&disk).is_some(), "the honest journal reopens intact");

    let another_word: fn(&mut Value) = |t| *field(t, "recipe") = word(&[2]);
    let another_end: fn(&mut Value) = |t| {
        let start = field(t, "pre").clone();
        *field(t, "post") = start.clone();
        *field(field(t, "revision"), "digest") = start;
    };
    let skipped_order: fn(&mut Value) = |t| {
        *field(t, "event") = Value::String("e2".into());
        *field(t, "order") = Value::Int(2);
        *field(field(t, "revision"), "head") = Value::String("e2".into());
    };
    for (name, edit) in [("another word", another_word), ("another end state", another_end), ("a skipped order", skipped_order)]
    {
        let mut edited = honest.clone();
        edit(transaction(&mut edited[1]));
        write(&disk, &edited);
        assert_eq!(reopened(&disk), None, "{name}");
    }
    write(&disk, &honest);
    assert!(reopened(&disk).is_some(), "the honest journal reopens intact again");
}

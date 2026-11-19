//! The session's profile against the command layer (BND.14, BND.15, BND.17, KN5.26; draft): the host sets it when it
//! creates the session, a reopen reads it back, KN5.26's admission of a record's word runs only in a solving session,
//! and BND.16's table of the rows a run may invoke by profile follows chapter 07 section 8. Where the frozen text is
//! silent, the tests follow the draft recorded in `src/lib.rs` under "Drafts".

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, OnceLock};

use c600_command::{
    code, draft_args, Binding, Caller, CommandLayer, Dependency, Error, Invocation, Kind, MemoryStore, Metadata,
    Overlay, Profile, Provenance, Receipt, Reopen, Secret, Shell, Status, Table, WorksheetRef, RECORD_ROW,
    WORKSHEET_COMPARE_ROW, WORKSHEET_SAVE_ROW,
};
use c600_kernel::json::{self, Value};
use c600_kernel::Kernel;

const SECRET: [u8; 32] = [7; 32];
const PROFILES: [Profile; 3] = Profile::ALL;
/// A tool's query row whose program may invoke `operation.preview`.
const TOOL: &str = "tool.propose";

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

fn text(s: &str) -> Value {
    Value::String(s.to_owned())
}

/// The host's reader as these tests stand it in: it refuses the block `refused` at a write (PS1.16), refuses every
/// record's word at admission (KN5.26), and counts the admissions it was asked for.
struct Reader {
    admissions: Arc<AtomicUsize>,
}

impl Provenance for Reader {
    fn check(&self, block: &[u8]) -> Result<(), Error> {
        match block {
            b"refused" => Err(Error::new(code::PROVENANCE_INCOMPATIBLE, "the test reader refuses the block")),
            _ => Ok(()),
        }
    }

    fn admit(&self, _block: &[u8]) -> Result<(), Error> {
        self.admissions.fetch_add(1, Ordering::SeqCst);
        Err(Error::new(code::NOT_GRANTED, "the test reader admits no record's word"))
    }
}

struct Fixture {
    layer: CommandLayer,
    shell: Shell,
    admissions: Arc<AtomicUsize>,
    sent: u64,
}

/// A session at the solved state in `profile`, with the draft rows, the test reader and a tool row whose program may
/// invoke `operation.preview`.
fn session(profile: Profile) -> Fixture {
    let k = kernel();
    let (start, protected) = (k.solved(), k.protected_set(&[]).unwrap());
    let (table, secret, store) = (Table::h08().with_drafts(), Secret::fixed(SECRET), MemoryStore::default());
    let mut layer = CommandLayer::new_with_profile(k, table, secret, store, "s1", start, protected, profile);
    let admissions = Arc::new(AtomicUsize::new(0));
    layer.set_provenance(Box::new(Reader { admissions: admissions.clone() }));
    let overlay = Overlay { module: "tools/propose".into(), entry: "run".into() };
    layer.register_overlay(TOOL, Kind::Query, &[], overlay).unwrap();
    let preview = Dependency { row: "operation.preview".into(), binding: Binding::Computed, optional: false };
    layer.declare_overlay(TOOL, Metadata { diagnostic: None, requires: vec![preview] }).unwrap();
    Fixture { layer, shell: Shell::new(Secret::fixed(SECRET), "shell"), admissions, sent: 0 }
}

impl Fixture {
    /// `workspace.set` through the command route.
    fn write(&mut self, name: &str, payload: Value, provenance: &str) -> Receipt {
        let args = json::object([("payload", payload), ("provenance", text(provenance)), ("record", text(name))]);
        self.shell.invoke(&mut self.layer, RECORD_ROW, args).unwrap()
    }

    /// One envelope on the channel of run `run`: no token, a fresh command ID, the current epoch.
    fn runtime(&mut self, run: &str, row: &str, args: Value) -> Receipt {
        self.sent += 1;
        let invocation = Invocation {
            command: format!("{run}/{}", self.sent),
            row: row.to_owned(),
            args,
            token: None,
            epoch: self.layer.revision().epoch,
        };
        self.layer.invoke(&Caller::Runtime { run: run.to_owned() }, invocation)
    }
}

fn refused_with(receipt: &Receipt) -> Option<&Error> {
    match &receipt.status {
        Status::Refused(error) => Some(error),
        _ => None,
    }
}

fn running(receipt: &Receipt) -> bool {
    matches!(receipt.status, Status::Running(_))
}

/// BND.17 (the host sets the profile when it creates the session, and nothing changes it; draft), KN5.23 (a reopen
/// reads it back). `CommandLayer::new` is a solving session (BND.14, BND.15).
#[test]
fn a_session_keeps_the_profile_the_host_created_it_in() {
    let k = kernel();
    let plain = CommandLayer::new(
        k.clone(),
        Table::h08(),
        Secret::fixed(SECRET),
        MemoryStore::default(),
        "s1",
        k.solved(),
        k.protected_set(&[]).unwrap(),
    );
    assert_eq!(plain.profile(), Profile::Solving);
    assert_eq!(plain.store().clone().history().unwrap().genesis.profile, Profile::Solving);

    for profile in PROFILES {
        let mut f = session(profile);
        assert_eq!(f.layer.profile(), profile);
        assert_eq!(f.layer.store().clone().history().unwrap().genesis.profile, profile, "the start records it");
        assert!(matches!(f.write("w", Value::Array(vec![Value::Int(1)]), "block").status, Status::Completed(_)));
        let store = f.layer.store().clone();
        let reopened = CommandLayer::open(kernel(), Table::h08().with_drafts(), Secret::fixed(SECRET), store).unwrap();
        assert_eq!(reopened.profile(), profile, "a reopen reads it back");
        assert!(reopened.recovery().is_none());
        assert_eq!(Profile::from_name(profile.name()), Some(profile));
    }
    assert_eq!(Profile::from_name("practice"), None);
}

/// KN5.26 applies only in a solving session (draft): there the host's reader is asked to admit a record's word at
/// `operation.preview`, and its refusal stands; in `tool_authoring` and `research` the reader is not asked. The reader's
/// check at `workspace.set` (PS1.16) runs in every profile.
#[test]
fn kn5_26_admits_a_record_word_only_in_a_solving_session() {
    for profile in PROFILES {
        let mut f = session(profile);
        let refused = f.write("bad", Value::Array(vec![Value::Int(1)]), "refused");
        assert_eq!(refused_with(&refused).map(|e| e.code), Some(code::PROVENANCE_INCOMPATIBLE), "{profile:?}");
        assert!(f.layer.record("bad").is_none());

        assert!(matches!(f.write("w", Value::Array(vec![Value::Int(1)]), "block").status, Status::Completed(_)));
        let draft = json::object([("draft", json::object([("record", text("w"))]))]);
        let receipt = f.shell.invoke(&mut f.layer, "operation.preview", draft).unwrap();
        match profile {
            Profile::Solving => {
                assert_eq!(refused_with(&receipt).map(|e| e.code), Some(code::NOT_GRANTED));
                assert_eq!(f.admissions.load(Ordering::SeqCst), 1);
            }
            _ => {
                assert!(running(&receipt), "{profile:?}: {:?}", receipt.status);
                assert_eq!(f.admissions.load(Ordering::SeqCst), 0, "{profile:?}: the reader is not asked");
                assert_eq!(f.layer.run_jobs(), 1);
            }
        }
    }
}

/// `worksheet.compare` follows `operation.preview` (KN5.26, draft): its analysis shows what a preview of the saved
/// sheet's word would, so the reader is asked to admit the sheet's block only in a solving session, and its refusal
/// stands there. In `tool_authoring` and `research` the comparison is admitted without asking.
#[test]
fn a_comparison_admits_the_sheets_word_only_in_a_solving_session() {
    for profile in PROFILES {
        let mut f = session(profile);
        assert!(matches!(f.write("w", Value::Array(vec![Value::Int(1)]), "block").status, Status::Completed(_)));
        let save = json::object([("draft", json::object([("record", text("w"))])), ("name", text("sheet"))]);
        let saved = f.shell.invoke(&mut f.layer, WORKSHEET_SAVE_ROW, save).unwrap();
        assert!(matches!(saved.status, Status::Completed(_)), "{profile:?}: {:?}", saved.status);
        let sheet = WorksheetRef { record: "sheet".into(), version: f.layer.records()["sheet"].version };
        let args = json::object([("worksheet", sheet.to_json()), ("draft", json::object([("record", text("w"))]))]);
        let receipt = f.shell.invoke(&mut f.layer, WORKSHEET_COMPARE_ROW, args).unwrap();
        match profile {
            Profile::Solving => {
                assert_eq!(refused_with(&receipt).map(|e| e.code), Some(code::NOT_GRANTED));
                assert_eq!(f.admissions.load(Ordering::SeqCst), 1);
                assert_eq!(f.layer.run_jobs(), 0, "no job was queued");
            }
            _ => {
                assert!(running(&receipt), "{profile:?}: {:?}", receipt.status);
                assert_eq!(f.admissions.load(Ordering::SeqCst), 0, "{profile:?}: the reader is not asked");
                assert_eq!(f.layer.run_jobs(), 1);
            }
        }
    }
}

/// BND.16's table by profile, chapter 07 section 8 (draft): a run in a `tool_authoring` or `research` session may not
/// invoke `operation.preview` (BND.60: its `propose` goes to a capture), so an entry that requires the row is
/// unavailable there, while the solver's own preview is admitted in every profile. In a solving session the run's
/// preview is admitted as before.
#[test]
fn a_run_prepares_no_engine_preview_outside_a_solving_session() {
    for profile in PROFILES {
        let mut f = session(profile);
        let availability = f.layer.availability(TOOL);
        let receipt = f.runtime("r1", "operation.preview", draft_args(&word(&[1])));
        match profile {
            Profile::Solving => {
                assert!(running(&receipt), "{:?}", receipt.status);
                assert_eq!(availability, Ok(()));
                assert_eq!(f.layer.run_jobs(), 1);
            }
            _ => {
                let error = refused_with(&receipt).unwrap_or_else(|| panic!("{profile:?}: {:?}", receipt.status));
                assert_eq!(error.code, code::NOT_GRANTED, "{profile:?}");
                assert!(error.detail.contains("BND.60"), "{}", error.detail);
                assert_eq!(availability.map_err(|e| e.code), Err(code::NOT_GRANTED), "{profile:?}");
                assert_eq!(f.layer.run_jobs(), 0, "no job was queued");
                let mine = f.shell.invoke(&mut f.layer, "operation.preview", draft_args(&word(&[1]))).unwrap();
                assert!(running(&mine), "{profile:?}: the solver's preview: {:?}", mine.status);
            }
        }
    }
}

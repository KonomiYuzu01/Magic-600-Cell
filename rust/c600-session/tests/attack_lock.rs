//! Attack tests on the session lock (owner-requested attack of the command layer, class E: the session lock is
//! bypassed). The rule is KN4.1 [B], "Only the engine opens the session files", as the store keeps it: one opener at a
//! time, and a second open of a held session is refused with `KN4:busy.session_locked` and changes no file
//! (`tests/session_lock.rs`). Each test asserts the rule; a test that fails while a bypass exists is ignored and its
//! comment gives the finding and what was observed on commit 4c20ce30. Every test uses a fresh directory in the
//! target's temporary folder and no other session.
//!
//! Run the ignored ones with `-- --include-ignored`.

mod common;

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use c600_command::{draft_args, CommandLayer, HeadlessFrames, Receipt, Secret, Shell, Status, Table};
use c600_kernel::json::{self, Value};
use c600_kernel::ProtectedSet;
use c600_session::disk::{FsDisk, MemDisk, LOCK};
use c600_session::{code, JournalStore, Options, Session};

/// A fresh directory for one test.
fn directory(name: &str) -> PathBuf {
    let root = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("c600-attack-lock-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    root
}

fn disk(root: &Path) -> Box<FsDisk> {
    Box::new(FsDisk::new(root).expect("a store directory"))
}

fn word(letters: &[i64]) -> Value {
    json::parse(format!(r#"[{{"kind": "word", "moves": {letters:?}}}]"#).as_bytes()).unwrap()
}

/// One opener of a session directory: the command layer over the journal store, with its shell and frames.
struct Opener {
    layer: CommandLayer<JournalStore>,
    shell: Shell,
    frames: HeadlessFrames,
}

/// A new session at the solved state in `root`.
fn create(root: &Path) -> Opener {
    let k = common::kernel();
    let store = JournalStore::create(disk(root)).expect("an empty directory takes a session");
    let secret = Secret::fixed([7; 32]);
    let layer = CommandLayer::new(k.clone(), Table::h08(), secret, store, "s1", k.solved(), ProtectedSet::none());
    assert!(layer.recovery().is_none(), "{:?}", layer.recovery());
    Opener { layer, shell: Shell::new(Secret::fixed([7; 32]), "shell"), frames: HeadlessFrames::new() }
}

/// Opens the session `root` holds under `secret`, or gives the code the open is refused with.
fn open(root: &Path, secret: [u8; 32]) -> Result<Opener, &'static str> {
    let store = JournalStore::open(disk(root)).map_err(|e| e.code)?;
    let layer = CommandLayer::open(common::kernel(), Table::h08(), Secret::fixed(secret), store).map_err(|e| e.code)?;
    Ok(Opener { layer, shell: Shell::new(Secret::fixed(secret), "shell"), frames: HeadlessFrames::new() })
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

    fn head(&self) -> String {
        self.layer.snapshot().digest().to_owned()
    }
}

fn status(receipt: &Receipt) -> String {
    match &receipt.status {
        Status::Committed(_) => "committed".to_owned(),
        other => format!("{other:?}"),
    }
}

/// What each kind of opener gets on `root`: the code it is refused with, or `None` when it opens.
fn openers(root: &Path) -> Vec<(&'static str, Option<&'static str>)> {
    let k = common::kernel;
    vec![
        ("Session::open", Session::open(k(), disk(root), Options::default()).err().map(|e| e.code)),
        ("Session::create", Session::create(k(), disk(root), "s2", Options::default()).err().map(|e| e.code)),
        ("JournalStore::open", JournalStore::open(disk(root)).err().map(|e| e.code)),
        ("JournalStore::create", JournalStore::create(disk(root)).err().map(|e| e.code)),
    ]
}

/// Finding E1 (class E, the session lock is bypassed). Rule: KN4.1 [B]; a second open of a held session is refused
/// with `KN4:busy.session_locked` and changes no file. The lock is an exclusive lock on the file `lock` in the session
/// directory, and the store holds no other claim on the directory or the journal. The file is opened with the
/// standard library's default sharing, so while the holder has it another process may delete or rename it. The next
/// opener then creates a new `lock` file, locks that one, and opens the journal the holder is still writing.
///
/// The test holds a session, removes the lock file's name the two ways (delete, rename), and asks that a second opener
/// is still refused. Where the operating system refuses to remove the held file, the rule is kept there and the test
/// says so.
///
/// Observed on 4c20ce30 (Windows 11, NTFS), both ways: the held file's name was removed, the second opener opened the
/// session, both openers' commit presses were answered `committed` from the same base revision, and after both were
/// dropped the session reopened in read-only recovery (`KN5:unknown_outcome.read_only_recovery`) at the second
/// opener's head: the holder's acknowledged commit is not in it. The exact answers are in the assertion's message.
///
/// Fixed: `FsDisk` creates the lock file only in a directory that holds no journal, so the second opener finds the
/// session's journal without a lock file and is refused with `KN4:busy.session_locked`, both ways and on every
/// platform, and creates no file.
#[test]
fn e1_a_second_opener_is_refused_after_the_held_lock_file_is_deleted_or_renamed() {
    let mut bypassed = Vec::new();
    for way in ["deleted", "renamed"] {
        let root = directory(way);
        let mut holder = create(&root);
        assert!(matches!(holder.commit(&word(&[1])).status, Status::Committed(_)));
        let base = holder.head();
        let lock = root.join(LOCK);
        let removed = match way {
            "deleted" => fs::remove_file(&lock),
            _ => fs::rename(&lock, root.join("lock.old")),
        };
        if let Err(error) = removed {
            // The operating system keeps a held lock file where it is: the rule holds on this platform.
            println!("{way}: the held lock file stays ({error})");
            for (opener, refused) in openers(&root) {
                assert_eq!(refused, Some(code::SESSION_LOCKED), "{way}: {opener}");
            }
            continue;
        }
        let mut second = match open(&root, [9; 32]) {
            Err(refused) => {
                assert_eq!(refused, code::SESSION_LOCKED, "{way}");
                continue;
            }
            Ok(second) => second,
        };
        // Both openers go on from the same base. Each is answered by its own layer.
        assert_eq!(second.head(), base, "{way}");
        let by_second = second.commit(&word(&[3]));
        let second_head = second.head();
        let by_holder = holder.commit(&word(&[2]));
        let holder_head = holder.head();
        drop(holder);
        drop(second);
        let after = match open(&root, [11; 32]) {
            Ok(after) => format!(
                "recovery {:?}, head is the holder's: {}, head is the second opener's: {}, head is the base: {}",
                after.layer.recovery().map(|e| e.code),
                after.head() == holder_head,
                after.head() == second_head,
                after.head() == base
            ),
            Err(refused) => format!("refused with {refused}"),
        };
        bypassed.push(format!(
            "{way}: a second opener opened the held session; its commit was answered {}, the holder's {}; the session \
             then reopened: {after}",
            status(&by_second),
            status(&by_holder)
        ));
        let _ = fs::remove_dir_all(&root);
    }
    assert!(bypassed.is_empty(), "{bypassed:#?}");
}

/// Finding E2 (class E, an API hazard: attacker 4). Rule: KN4.1 [B] as above. `Disk::lock` has a default body that
/// takes no lock, and the memory disk keeps that default, so two stores open one memory disk at once. A product host
/// that hands the store any disk without its own `lock` has no session lock, and nothing says so.
///
/// Observed on 4c20ce30: the second `JournalStore::open` on a memory disk that a first store still holds opens.
#[test]
#[ignore = "finding E2: fails while a disk without its own lock opens twice"]
fn e2_a_disk_that_takes_no_lock_does_not_open_twice() {
    let disk = MemDisk::new();
    let k = common::kernel();
    let store = JournalStore::create(Box::new(disk.clone())).expect("an empty disk takes a session");
    let secret = Secret::fixed([7; 32]);
    let holder = CommandLayer::new(k.clone(), Table::h08(), secret, store, "s1", k.solved(), ProtectedSet::none());
    let second = JournalStore::open(Box::new(disk.clone())).err().map(|e| e.code);
    assert_eq!(second, Some(code::SESSION_LOCKED), "a second store opened the memory disk the first still holds");
    drop(holder);
}

/// Guard (class E, refuted): another spelling of the session directory's path is the same lock. A second opener that
/// names the directory through `.` or through a subdirectory and `..` is refused.
#[test]
fn another_path_to_the_held_directory_is_the_same_lock() {
    let root = directory("paths");
    let mut holder = create(&root);
    assert!(matches!(holder.commit(&word(&[1])).status, Status::Committed(_)));
    for other in [root.join("."), root.join("objects").join("..")] {
        for (opener, refused) in openers(&other) {
            assert_eq!(refused, Some(code::SESSION_LOCKED), "{opener} through {other:?}");
        }
    }
    assert!(matches!(holder.commit(&word(&[2])).status, Status::Committed(_)));
    drop(holder);
    fs::remove_dir_all(&root).expect("the directory is removed");
}

/// Guard (class E, refuted): the read-only recovery path takes the lock too. A session whose journal is damaged opens
/// read-only, and while that opener holds it every second opener is refused; the lock is taken before the journal is
/// read, so a second opener neither reads nor settles what the first holds. Once the first is dropped the session
/// opens again, still read-only.
#[test]
fn a_session_held_in_read_only_recovery_refuses_a_second_opener() {
    let root = directory("recovery");
    let mut first = create(&root);
    assert!(matches!(first.commit(&word(&[1])).status, Status::Committed(_)));
    drop(first);
    // Damage that is not a torn append: a frame header whose length is over the limit.
    let journal = root.join("journal");
    let mut file = fs::OpenOptions::new().append(true).open(&journal).expect("the journal exists");
    file.write_all(&[0xFF; 16]).expect("the damage is written");
    drop(file);
    let before = fs::read(&journal).expect("the journal is read");

    let holder = open(&root, [9; 32]).unwrap_or_else(|refused| panic!("the damaged session does not open: {refused}"));
    assert!(holder.layer.recovery().is_some(), "a damaged journal opens read-only");
    for (opener, refused) in openers(&root) {
        assert_eq!(refused, Some(code::SESSION_LOCKED), "{opener}");
    }
    assert_eq!(fs::read(&journal).expect("the journal is read"), before, "a refused opener changed the journal");
    drop(holder);
    let again = open(&root, [11; 32]).unwrap_or_else(|refused| panic!("the session does not open again: {refused}"));
    assert!(again.layer.recovery().is_some());
    drop(again);
    fs::remove_dir_all(&root).expect("the directory is removed");
}

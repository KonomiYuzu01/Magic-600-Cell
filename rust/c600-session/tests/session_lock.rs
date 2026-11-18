//! The session lock (KN4.1: only the engine opens the session files; Fable lead L9). A store takes it before it reads
//! the journal, so one process opens a session at a time. A second open of a held session, in this process or another,
//! is refused with `KN4:busy.session_locked` (a draft code) and changes no file. The session opens again once its
//! holder lets go: when the holder is dropped, or when its process ends, a hard exit included. Each test uses a fresh
//! directory in the target's temporary folder.

mod common;

use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};
use std::{env, fs, thread};

use c600_session::disk::{FsDisk, LOCK};
use c600_session::{code, JournalStore, Options, Session};
use common::*;

/// The variables that make [`holder_process`] hold a session: the session's directory, and the marker file it writes
/// once it holds the session.
const HOLDER: &str = "C600_SESSION_LOCK_HOLDER";
const MARKER: &str = "C600_SESSION_LOCK_MARKER";

/// A fresh directory for one test.
fn directory(name: &str) -> PathBuf {
    let root = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("c600-lock-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    root
}

fn disk(root: &Path) -> Box<FsDisk> {
    Box::new(FsDisk::new(root).expect("a store directory"))
}

/// Every file below `root` by its path under `root`, with its bytes. The lock file is listed without its bytes,
/// because on Windows its lock covers reads through other handles.
fn files(root: &Path) -> Vec<(PathBuf, Option<Vec<u8>>)> {
    let mut found = Vec::new();
    let mut dirs = vec![root.to_path_buf()];
    while let Some(dir) = dirs.pop() {
        for entry in fs::read_dir(&dir).expect("a readable directory") {
            let path = entry.expect("a directory entry").path();
            if path.is_dir() {
                dirs.push(path);
                continue;
            }
            let name = path.strip_prefix(root).expect("a path below the root").to_path_buf();
            let bytes = (name != Path::new(LOCK)).then(|| fs::read(&path).expect("a readable file"));
            found.push((name, bytes));
        }
    }
    found.sort();
    found
}

/// What each kind of opener gets on `root`: the code it is refused with, or `None` when it opens.
fn openers(root: &Path) -> Vec<(&'static str, Option<&'static str>)> {
    vec![
        ("Session::open", Session::open(kernel(), disk(root), Options::default()).err().map(|e| e.code)),
        ("Session::create", Session::create(kernel(), disk(root), "s2", Options::default()).err().map(|e| e.code)),
        ("JournalStore::open", JournalStore::open(disk(root)).err().map(|e| e.code)),
        ("JournalStore::create", JournalStore::create(disk(root)).err().map(|e| e.code)),
    ]
}

/// A second opener of a held session in the same process is refused with `KN4:busy.session_locked`, whether it opens
/// or creates, through M6's `Session` or the command layer's `JournalStore`. It changes no file, and the holder goes on
/// committing. Once the holder is dropped, the session opens with what the holder wrote.
#[test]
fn a_held_session_refuses_a_second_opener_in_the_same_process() {
    let root = directory("same");
    let holder = Session::create(kernel(), disk(&root), "s1", Options::default()).expect("the session is created");
    committed(&commit(&holder, "c1", &[1, 2]));
    let before = files(&root);
    assert!(before.iter().any(|(name, bytes)| name == Path::new(LOCK) && bytes.is_none()), "{before:?}");

    for (opener, refused) in openers(&root) {
        assert_eq!(refused, Some(code::SESSION_LOCKED), "{opener}");
    }
    assert_eq!(files(&root), before);
    committed(&commit(&holder, "c2", &[3]));
    let view = durable(&holder);
    drop(holder);

    let reopened = Session::open(kernel(), disk(&root), Options::default()).expect("the session opens");
    assert_eq!(durable(&reopened), view);
    drop(reopened);
    fs::remove_dir_all(&root).expect("the directory is removed");
}

/// Not a test of its own: the process that `a_session_held_by_another_process_opens_once_that_process_is_killed`
/// starts. With [`HOLDER`] set, it opens the session in that directory, writes the marker file that [`MARKER`] names,
/// and holds the session until its standard input closes or it is killed. Without it, it does nothing.
#[test]
fn holder_process() {
    let (Ok(root), Ok(marker)) = (env::var(HOLDER), env::var(MARKER)) else { return };
    let session = Session::open(kernel(), disk(Path::new(&root)), Options::default()).expect("the holder opens");
    fs::write(marker, b"held").expect("the marker is written");
    let _ = std::io::stdin().read_to_end(&mut Vec::new());
    drop(session);
}

/// A session that another process holds refuses this process's openers with `KN4:busy.session_locked` and changes no
/// file. Once that process is killed, the operating system releases its lock and the session opens.
#[test]
fn a_session_held_by_another_process_opens_once_that_process_is_killed() {
    let root = directory("other");
    let marker = root.with_extension("held");
    let _ = fs::remove_file(&marker);
    let session = Session::create(kernel(), disk(&root), "s1", Options::default()).expect("the session is created");
    committed(&commit(&session, "c1", &[1, 2]));
    let view = durable(&session);
    drop(session);

    let mut holder = Command::new(env::current_exe().expect("the test binary"))
        .args(["holder_process", "--exact"])
        .env(HOLDER, &root)
        .env(MARKER, &marker)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("the holder process starts");
    let start = Instant::now();
    while !marker.exists() {
        if let Some(status) = holder.try_wait().expect("the holder's status") {
            panic!("the holder ended before it held the session: {status}");
        }
        assert!(start.elapsed() < Duration::from_secs(120), "the holder never held the session");
        thread::sleep(Duration::from_millis(20));
    }
    let before = files(&root);
    for (opener, refused) in openers(&root) {
        assert_eq!(refused, Some(code::SESSION_LOCKED), "{opener}");
    }
    assert_eq!(files(&root), before);

    holder.kill().expect("the holder is killed");
    holder.wait().expect("the holder ends");
    // Windows releases a dead process's locks soon but at no set time, so the open is tried again for a while.
    let start = Instant::now();
    let reopened = loop {
        match Session::open(kernel(), disk(&root), Options::default()) {
            Ok(session) => break session,
            Err(e) if e.code == code::SESSION_LOCKED && start.elapsed() < Duration::from_secs(30) => {
                thread::sleep(Duration::from_millis(20))
            }
            Err(e) => panic!("the session does not open after its holder was killed: {e:?}"),
        }
    };
    assert_eq!(durable(&reopened), view);
    drop(reopened);
    fs::remove_file(&marker).expect("the marker is removed");
    fs::remove_dir_all(&root).expect("the directory is removed");
}

/// The engine's explicit "open anyway" (`FsDisk::open_anyway`; the owner's reading of finding E1, 18 November 2026).
/// A session whose `lock` file is gone, as after a hard exit with the file removed, refuses every plain opener and
/// changes no file. The open anyway puts back the lock file and opens it with what was written. While the session is
/// held, the open anyway is refused as any opener is, and on Windows the held lock file can be neither deleted nor
/// renamed (the platform layer). Once the holder is dropped, a plain open succeeds again.
#[test]
fn the_open_anyway_puts_back_a_lost_lock_file_and_never_opens_a_held_session() {
    let root = directory("anyway");
    let session = Session::create(kernel(), disk(&root), "s1", Options::default()).expect("the session is created");
    committed(&commit(&session, "c1", &[1, 2]));
    let view = durable(&session);
    drop(session);
    fs::remove_file(root.join(LOCK)).expect("the lock file is removed");

    let before = files(&root);
    for (opener, refused) in openers(&root) {
        assert_eq!(refused, Some(code::SESSION_LOCKED), "{opener}");
    }
    assert_eq!(files(&root), before, "a refused opener changed a file");

    let anyway = || Box::new(FsDisk::open_anyway(&root).expect("a store directory"));
    let holder = Session::open(kernel(), anyway(), Options::default()).expect("the open anyway opens");
    assert!(root.join(LOCK).is_file(), "the open anyway did not put back the lock file");
    assert_eq!(durable(&holder), view);
    assert_eq!(Session::open(kernel(), anyway(), Options::default()).err().map(|e| e.code), Some(code::SESSION_LOCKED));
    assert_eq!(JournalStore::open(anyway()).err().map(|e| e.code), Some(code::SESSION_LOCKED));
    for (opener, refused) in openers(&root) {
        assert_eq!(refused, Some(code::SESSION_LOCKED), "{opener}");
    }
    if cfg!(windows) {
        assert!(fs::remove_file(root.join(LOCK)).is_err(), "a held lock file was deleted");
        assert!(fs::rename(root.join(LOCK), root.join("lock.old")).is_err(), "a held lock file was renamed");
    }
    committed(&commit(&holder, "c2", &[3]));
    let view = durable(&holder);
    drop(holder);

    let reopened = Session::open(kernel(), disk(&root), Options::default()).expect("the session opens");
    assert_eq!(durable(&reopened), view);
    drop(reopened);
    fs::remove_dir_all(&root).expect("the directory is removed");
}

//! The platform layer of RPI.52: windowing, file locking, process ownership, platform paths and Direct3D 12 sit
//! behind narrow platform interfaces, and this member is where Rust code may call an operating system's own API
//! (`tests/test_language_scans.py`, `PLATFORM_LAYER`). The engine, the session store, the command layer and the view
//! model call it through portable functions and contain no Windows-only code (RPI.51).
//!
//! Its first interface is the session lock file's open (the owner's reading of finding E1 of the KN5 boundary
//! attack, 18 November 2026, `docs/wiki/decisions/owner-decisions-2026-11-18.md`; the layer-boundary change approved
//! under PHS.52). Its second, under process ownership, keeps a console program that a caller starts from opening a
//! console window of its own ([`no_console_window`]), for the language runtime's child. It uses std only.

#![forbid(unsafe_code)]

use std::fs::{File, OpenOptions};
use std::io;
use std::path::Path;
use std::process::Command;

/// Opens the session lock file at `path` for reading and writing, creating it empty when `create` is set and it is
/// missing, for the caller to lock with `File::try_lock`.
///
/// On Windows the handle shares reading and writing and denies delete sharing, so while it is open nothing can delete
/// or rename the file: the lock keeps its name while it is held. A second open of the file for the lock still
/// succeeds and its `try_lock` is refused. Elsewhere the file opens as std opens any file, and the file can be removed
/// while it is held.
pub fn open_lock_file(path: &Path, create: bool) -> io::Result<File> {
    let mut options = OpenOptions::new();
    options.read(true).write(true);
    if create {
        options.create(true).truncate(false);
    }
    keep_name_while_open(&mut options);
    options.open(path)
}

#[cfg(windows)]
fn keep_name_while_open(options: &mut OpenOptions) {
    use std::os::windows::fs::OpenOptionsExt;
    // `FILE_SHARE_READ | FILE_SHARE_WRITE`, without `FILE_SHARE_DELETE`; std's default shares all three.
    const SHARE_READ_WRITE: u32 = 0x1 | 0x2;
    options.share_mode(SHARE_READ_WRITE);
}

#[cfg(not(windows))]
fn keep_name_while_open(_: &mut OpenOptions) {}

/// Keeps the console program that `command` starts from opening a console window of its own, and returns `command`.
///
/// On Windows a console program started by a program that has no console, such as the shell, gets a new console
/// window each time it starts. The language runtime's parent pipes its child's three streams and starts a new child
/// after every fault, so such a window would only open and close over the shell. Here the child starts with
/// `CREATE_NO_WINDOW`. Elsewhere `command` is unchanged.
pub fn no_console_window(command: &mut Command) -> &mut Command {
    hide_console_window(command);
    command
}

#[cfg(windows)]
fn hide_console_window(command: &mut Command) {
    use std::os::windows::process::CommandExt;
    // `CREATE_NO_WINDOW`: a console program runs without a console window.
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    command.creation_flags(CREATE_NO_WINDOW);
}

#[cfg(not(windows))]
fn hide_console_window(_: &mut Command) {}

#[cfg(test)]
mod tests {
    use super::{no_console_window, open_lock_file};
    use std::fs;
    use std::path::PathBuf;
    use std::process::{Command, Stdio};

    /// A program started without a console window still runs with its streams piped: the test binary lists its own
    /// tests to its parent. Whether a window opens is not visible here; a device run shows that.
    #[test]
    fn a_child_without_a_console_window_still_answers_through_its_pipes() {
        let mut command = Command::new(std::env::current_exe().expect("the test binary"));
        command.arg("--list").stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped());
        let output = no_console_window(&mut command).output().expect("the child starts");
        assert!(output.status.success(), "{output:?}");
        let listed = String::from_utf8_lossy(&output.stdout);
        assert!(listed.contains("a_child_without_a_console_window_still_answers_through_its_pipes"), "{listed}");
    }

    fn directory(name: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!("c600-platform-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("a test directory");
        root
    }

    #[test]
    fn a_missing_lock_file_is_created_only_when_asked() {
        let root = directory("create");
        let path = root.join("lock");
        assert_eq!(open_lock_file(&path, false).err().map(|e| e.kind()), Some(std::io::ErrorKind::NotFound));
        assert!(!path.exists());
        drop(open_lock_file(&path, true).expect("the lock file is created"));
        assert_eq!(fs::read(&path).expect("the lock file is read"), b"");
        drop(open_lock_file(&path, false).expect("the lock file opens"));
        fs::remove_dir_all(&root).expect("the directory is removed");
    }

    /// While a handle is open, a second open succeeds and its lock is refused; on Windows the file can be neither
    /// deleted nor renamed, and once the handle is closed it can.
    #[test]
    fn a_held_lock_file_keeps_its_name_on_windows() {
        let root = directory("held");
        let path = root.join("lock");
        let held = open_lock_file(&path, true).expect("the lock file is created");
        held.try_lock().expect("the lock is taken");
        let second = open_lock_file(&path, false).expect("a second handle opens");
        assert!(second.try_lock().is_err(), "a second handle took a held lock");
        drop(second);
        if cfg!(windows) {
            assert!(fs::remove_file(&path).is_err(), "a held lock file was deleted");
            assert!(fs::rename(&path, root.join("lock.old")).is_err(), "a held lock file was renamed");
            assert!(path.exists());
        }
        drop(held);
        fs::rename(&path, root.join("lock.old")).expect("a closed lock file is renamed");
        fs::remove_file(root.join("lock.old")).expect("a closed lock file is deleted");
        fs::remove_dir_all(&root).expect("the directory is removed");
    }
}

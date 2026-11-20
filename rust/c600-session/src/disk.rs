//! The files below the store: a directory on a real file system, or memory that fails, tears and crashes on request.
//!
//! A store keeps two kinds of file, named with `/` separators: the journal `journal` and the content-addressed
//! objects `objects/<sha256>` (state objects, and the command store's record objects). Nothing written is durable until
//! [`Disk::sync`] returns for that file.
//!
//! A store takes the session lock ([`Disk::lock`]) before it reads the journal, so one process opens a session at
//! a time (KN4.1). Every disk takes it: [`FsDisk`] locks the file [`LOCK`] in its directory, and `MemDisk`, the memory
//! disk that only tests use (the `mem-disk` feature), holds a lock among its clones (finding E2 of the KN5 boundary
//! attack; the owner's reading of 18 November 2026, `docs/wiki/decisions/owner-decisions-2026-11-18.md`).

#[cfg(feature = "mem-disk")]
use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
#[cfg(feature = "mem-disk")]
use std::sync::{Arc, Condvar, Mutex, MutexGuard};

use crate::store::JOURNAL;

/// The operations a store needs. Every call but [`Disk::read`] and [`Disk::lock`] is a write and counts toward the
/// write index of a `MemDisk`.
pub trait Disk: Send {
    /// The whole file, or `None` if it does not exist.
    fn read(&mut self, name: &str) -> io::Result<Option<Vec<u8>>>;
    /// Appends bytes at the end of the file, creating it if needed.
    fn append(&mut self, name: &str, bytes: &[u8]) -> io::Result<()>;
    /// Writes the file whole, replacing any earlier contents.
    fn create(&mut self, name: &str, bytes: &[u8]) -> io::Result<()>;
    /// Cuts the file to `len` bytes.
    fn truncate(&mut self, name: &str, len: u64) -> io::Result<()>;
    /// Makes the file's current contents and its name durable.
    fn sync(&mut self, name: &str) -> io::Result<()>;
    /// Takes the session lock, which the disk holds until it is dropped: one process opens a session at a time
    /// (KN4.1). A lock that another opener holds is refused with `io::ErrorKind::WouldBlock`. Every disk implements
    /// it; there is no default that takes no lock (finding E2).
    fn lock(&mut self) -> io::Result<()>;
}

/// The name of a directory's lock file, which [`FsDisk`] creates empty and never removes.
pub const LOCK: &str = "lock";

/// A directory on the file system. On Windows the directory entry of a new file is not synced separately; NTFS
/// journals it (an M6 gap, recorded in `rust/README.md`).
///
/// Its session lock is an exclusive lock on the file [`LOCK`], taken with std's `File::try_lock` (`LockFileEx` on
/// Windows, `flock` on Unix). Its open handle holds the lock until the disk is dropped, and the operating system
/// releases it when the process ends, a hard exit included. A second handle conflicts even in the same process,
/// so a second `FsDisk` on the same directory is refused too. Before the lock is decided, [`FsDisk::new`] creates
/// the directory and its `objects` folder, only if it is missing, and [`Disk::lock`] creates the empty lock file,
/// only if it is missing and the directory holds no journal (below); a refused disk reads and writes no journal and
/// no state object.
///
/// A lock on a file holds only while the file keeps its name (finding E1 of the KN5 boundary attack). On Windows the
/// lock file is opened through the platform layer (`c600_platform::open_lock_file`, RPI.52) with a handle that
/// denies delete sharing, so while the session is held nothing can delete or rename the file, and a removal of the
/// whole session directory fails too (elsewhere it succeeds). Outside Windows the file can be removed while it is
/// held, and when nobody holds the session it can be removed everywhere. Two rules cover a removal on every platform:
/// [`Disk::lock`] creates the lock file only in a directory that holds no journal, since a store takes the lock before
/// it writes the journal and `FsDisk` never removes the lock file, so a journal without a lock file means that
/// something outside the store removed it, and the opener is refused as a held session is
/// (`io::ErrorKind::WouldBlock`), changing no file; and a disk that holds the lock puts back an empty lock file, if one
/// is missing, when it is dropped, so the session opens again once its holder is gone. Outside Windows a lock file
/// removed and then created again by something outside the store while the session is held lets a second opener in.
///
/// A hard exit alone leaves the lock file in place: the store never removes it. When something outside the store (a
/// person, a sync service, an antivirus) removed it while no opener held the session, or, outside Windows, while a
/// holder held it that then ended without dropping its disk, the session is refused until an empty `lock` file is
/// restored, by hand or through [`FsDisk::open_anyway`], the engine's explicit "open anyway" (the owner's reading, 18
/// November 2026). It sees only this machine's lock: a session in a folder that another machine or a sync service
/// shares can be held elsewhere, and the open anyway does not see it.
pub struct FsDisk {
    root: PathBuf,
    lock: Option<fs::File>,
    /// Whether [`Disk::lock`] puts back a missing lock file next to a journal instead of refusing.
    anyway: bool,
}

impl FsDisk {
    /// A store in `root`, which is created if missing.
    pub fn new(root: &Path) -> io::Result<FsDisk> {
        FsDisk::with(root, false)
    }

    /// The engine's explicit "open anyway" for a session whose lock file is missing next to its journal: an
    /// [`FsDisk::new`] whose [`Disk::lock`] puts back the empty lock file and takes it, where a plain disk refuses.
    /// While this machine's lock is held it refuses as any opener is (`io::ErrorKind::WouldBlock`): the lock file is
    /// there, and on Windows it cannot be removed while it is held. It sees no lock that another machine or a sync
    /// service holds on a shared folder, and outside Windows no holder of a lock file that was removed. It has no UI
    /// (the owner's reading of finding E1, 18 November 2026).
    pub fn open_anyway(root: &Path) -> io::Result<FsDisk> {
        FsDisk::with(root, true)
    }

    fn with(root: &Path, anyway: bool) -> io::Result<FsDisk> {
        fs::create_dir_all(root.join("objects"))?;
        Ok(FsDisk { root: root.to_path_buf(), lock: None, anyway })
    }

    fn path(&self, name: &str) -> PathBuf {
        name.split('/').fold(self.root.clone(), |path, part| path.join(part))
    }
}

impl Disk for FsDisk {
    fn read(&mut self, name: &str) -> io::Result<Option<Vec<u8>>> {
        match fs::read(self.path(name)) {
            Ok(bytes) => Ok(Some(bytes)),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e),
        }
    }

    fn append(&mut self, name: &str, bytes: &[u8]) -> io::Result<()> {
        OpenOptions::new().create(true).append(true).open(self.path(name))?.write_all(bytes)
    }

    fn create(&mut self, name: &str, bytes: &[u8]) -> io::Result<()> {
        fs::write(self.path(name), bytes)
    }

    fn truncate(&mut self, name: &str, len: u64) -> io::Result<()> {
        OpenOptions::new().write(true).open(self.path(name))?.set_len(len)
    }

    fn sync(&mut self, name: &str) -> io::Result<()> {
        let path = self.path(name);
        OpenOptions::new().write(true).open(&path)?.sync_all()?;
        #[cfg(unix)]
        if let Some(parent) = path.parent() {
            fs::File::open(parent)?.sync_all()?;
        }
        Ok(())
    }

    fn lock(&mut self) -> io::Result<()> {
        if self.lock.is_some() {
            return Ok(());
        }
        let path = self.path(LOCK);
        let file = match c600_platform::open_lock_file(&path, false) {
            Ok(file) => file,
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                if !self.anyway && self.path(JOURNAL).try_exists()? {
                    let detail = concat!(
                        "the session has a journal and no lock file: something removed the lock file, and another ",
                        "opener may hold the session; if none does, the open anyway puts the lock file back",
                    );
                    return Err(io::Error::new(io::ErrorKind::WouldBlock, detail));
                }
                c600_platform::open_lock_file(&path, true)?
            }
            Err(e) => return Err(e),
        };
        match file.try_lock() {
            Ok(()) => {
                self.lock = Some(file);
                Ok(())
            }
            Err(fs::TryLockError::WouldBlock) => {
                Err(io::Error::new(io::ErrorKind::WouldBlock, "another opener holds the session lock"))
            }
            Err(fs::TryLockError::Error(e)) => Err(e),
        }
    }
}

impl Drop for FsDisk {
    /// Puts back an empty lock file that something outside the store removed while this disk held the session, so
    /// that the session opens again once the holder is gone. A disk that holds no lock leaves the directory as it is.
    fn drop(&mut self) {
        // The held handle is closed first, so that a name it still occupies is free.
        if self.lock.take().is_some() {
            let _ = OpenOptions::new().write(true).create_new(true).open(self.path(LOCK));
        }
    }
}

/// How an injected failure behaves.
#[cfg(feature = "mem-disk")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fault {
    /// The write does nothing and reports an error.
    Error,
    /// An append writes only its first `n` bytes, then reports an error.
    Partial(usize),
    /// The write takes effect, a sync even durably, and still reports an error.
    Persisted,
}

#[cfg(feature = "mem-disk")]
#[derive(Clone, Default)]
struct MemFile {
    current: Option<Vec<u8>>,
    durable: Option<Vec<u8>>,
}

#[cfg(feature = "mem-disk")]
#[derive(Default)]
struct MemState {
    files: BTreeMap<String, MemFile>,
    writes: u64,
    reads: u64,
    faults: BTreeMap<u64, Fault>,
    fail_from: Option<u64>,
    hold_at: Option<u64>,
    held: bool,
    /// The ticket of the clone that holds the session lock, if one does.
    locked_by: Option<u64>,
    /// The last ticket issued.
    tickets: u64,
}

/// A disk in memory, shared by its clones, for tests only (the `mem-disk` feature): a test keeps one clone to inject
/// failures, crash and inspect, and hands another to the store. Writes are numbered from 0 in the order they are
/// attempted.
///
/// Its session lock is held by one clone among all the clones of a disk, from its [`Disk::lock`] until that clone is
/// dropped or the disk crashes ([`MemDisk::crash`]: the process that held it is gone), so a second store on a disk
/// that a first store holds is refused (`io::ErrorKind::WouldBlock`), as on a file system (finding E2). A new clone
/// holds no lock.
#[cfg(feature = "mem-disk")]
#[derive(Default)]
pub struct MemDisk {
    shared: Arc<(Mutex<MemState>, Condvar)>,
    /// The ticket this clone took the session lock with.
    ticket: Option<u64>,
}

#[cfg(feature = "mem-disk")]
impl Clone for MemDisk {
    fn clone(&self) -> MemDisk {
        MemDisk { shared: Arc::clone(&self.shared), ticket: None }
    }
}

#[cfg(feature = "mem-disk")]
impl Drop for MemDisk {
    fn drop(&mut self) {
        if let Some(ticket) = self.ticket.take() {
            let mut s = self.state();
            if s.locked_by == Some(ticket) {
                s.locked_by = None;
            }
        }
    }
}

#[cfg(feature = "mem-disk")]
impl MemDisk {
    pub fn new() -> MemDisk {
        MemDisk::default()
    }

    fn state(&self) -> MutexGuard<'_, MemState> {
        self.shared.0.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// The number of writes attempted so far, which is the index of the next one.
    pub fn writes(&self) -> u64 {
        self.state().writes
    }

    /// The number of reads so far.
    pub fn reads(&self) -> u64 {
        self.state().reads
    }

    /// Makes the write with this index fail.
    pub fn fail_write(&self, index: u64, fault: Fault) {
        self.state().faults.insert(index, fault);
    }

    /// Makes every write from this index on fail with [`Fault::Error`].
    pub fn fail_from(&self, index: u64) {
        self.state().fail_from = Some(index);
    }

    /// Removes every scheduled failure.
    pub fn heal(&self) {
        let mut s = self.state();
        s.faults.clear();
        s.fail_from = None;
    }

    /// Power loss: every file keeps only what was synced.
    pub fn crash(&self) {
        self.crash_keeping("", 0);
    }

    /// Power loss in which the unsynced appended tail of `name` partly reached the disk: it keeps up to `extra` bytes
    /// past its durable length, a torn append.
    pub fn crash_keeping(&self, name: &str, extra: usize) {
        let mut s = self.state();
        for (file_name, file) in s.files.iter_mut() {
            let mut kept = file.durable.clone();
            if file_name == name {
                if let (Some(durable), Some(current)) = (&file.durable, &file.current) {
                    if current.len() > durable.len() && current.starts_with(durable) {
                        kept = Some(current[..durable.len() + extra.min(current.len() - durable.len())].to_vec());
                    }
                }
            }
            file.current = kept.clone();
            file.durable = kept;
        }
        s.faults.clear();
        s.fail_from = None;
        s.hold_at = None;
        // The process that held the session lock is gone, and the lock with it.
        s.locked_by = None;
    }

    /// The process that holds the session lock ends without a crash: the lock is released, and every file keeps what
    /// was written, synced or not. A test calls it before it opens again a session whose first store it still owns,
    /// as a restart; the first store must not be used after it.
    pub fn end_holder(&self) {
        self.state().locked_by = None;
    }

    /// The durable bytes of a file.
    pub fn durable(&self, name: &str) -> Option<Vec<u8>> {
        self.state().files.get(name).and_then(|f| f.durable.clone())
    }

    /// Replaces the durable and current bytes of a file, as damage from outside the store would.
    pub fn overwrite(&self, name: &str, bytes: &[u8]) {
        let mut s = self.state();
        let file = s.files.entry(name.to_string()).or_default();
        file.current = Some(bytes.to_vec());
        file.durable = Some(bytes.to_vec());
    }

    /// Makes the write with this index wait, once it is reached, until [`MemDisk::release`].
    pub fn hold_write(&self, index: u64) {
        self.state().hold_at = Some(index);
    }

    /// Waits until a held write is waiting.
    pub fn wait_held(&self) {
        let (lock, cvar) = &*self.shared;
        let mut s = lock.lock().unwrap_or_else(|e| e.into_inner());
        while !s.held {
            s = cvar.wait(s).unwrap_or_else(|e| e.into_inner());
        }
    }

    /// Lets a held write go on.
    pub fn release(&self) {
        let (lock, cvar) = &*self.shared;
        let mut s = lock.lock().unwrap_or_else(|e| e.into_inner());
        s.hold_at = None;
        s.held = false;
        cvar.notify_all();
    }

    /// Numbers a write, honours a hold, and returns the fault scheduled for it.
    fn begin_write(&self) -> (MutexGuard<'_, MemState>, Option<Fault>) {
        let (lock, cvar) = &*self.shared;
        let mut s = lock.lock().unwrap_or_else(|e| e.into_inner());
        let index = s.writes;
        s.writes += 1;
        if s.hold_at == Some(index) {
            s.held = true;
            cvar.notify_all();
            while s.hold_at == Some(index) {
                s = cvar.wait(s).unwrap_or_else(|e| e.into_inner());
            }
        }
        let fault = s.faults.remove(&index).or(match s.fail_from {
            Some(from) if index >= from => Some(Fault::Error),
            _ => None,
        });
        (s, fault)
    }
}

#[cfg(feature = "mem-disk")]
fn injected() -> io::Error {
    io::Error::other("injected write failure")
}

#[cfg(feature = "mem-disk")]
impl Disk for MemDisk {
    fn read(&mut self, name: &str) -> io::Result<Option<Vec<u8>>> {
        let mut s = self.state();
        s.reads += 1;
        Ok(s.files.get(name).and_then(|f| f.current.clone()))
    }

    fn append(&mut self, name: &str, bytes: &[u8]) -> io::Result<()> {
        let (mut s, fault) = self.begin_write();
        let file = s.files.entry(name.to_string()).or_default();
        let current = file.current.get_or_insert_with(Vec::new);
        match fault {
            None => {
                current.extend_from_slice(bytes);
                Ok(())
            }
            Some(Fault::Error) => Err(injected()),
            Some(Fault::Partial(n)) => {
                current.extend_from_slice(&bytes[..n.min(bytes.len())]);
                Err(injected())
            }
            Some(Fault::Persisted) => {
                current.extend_from_slice(bytes);
                Err(injected())
            }
        }
    }

    fn create(&mut self, name: &str, bytes: &[u8]) -> io::Result<()> {
        let (mut s, fault) = self.begin_write();
        let file = s.files.entry(name.to_string()).or_default();
        match fault {
            None => {
                file.current = Some(bytes.to_vec());
                Ok(())
            }
            Some(Fault::Error) => Err(injected()),
            Some(Fault::Partial(n)) => {
                file.current = Some(bytes[..n.min(bytes.len())].to_vec());
                Err(injected())
            }
            Some(Fault::Persisted) => {
                file.current = Some(bytes.to_vec());
                Err(injected())
            }
        }
    }

    fn truncate(&mut self, name: &str, len: u64) -> io::Result<()> {
        let (mut s, fault) = self.begin_write();
        if fault == Some(Fault::Error) || matches!(fault, Some(Fault::Partial(_))) {
            return Err(injected());
        }
        let file = s.files.get_mut(name).ok_or_else(|| io::Error::from(io::ErrorKind::NotFound))?;
        if let Some(current) = &mut file.current {
            current.truncate(len as usize);
        }
        if fault.is_some() {
            return Err(injected());
        }
        Ok(())
    }

    fn sync(&mut self, name: &str) -> io::Result<()> {
        let (mut s, fault) = self.begin_write();
        if fault == Some(Fault::Error) || matches!(fault, Some(Fault::Partial(_))) {
            return Err(injected());
        }
        let file = s.files.get_mut(name).ok_or_else(|| io::Error::from(io::ErrorKind::NotFound))?;
        file.durable = file.current.clone();
        if fault.is_some() {
            return Err(injected());
        }
        Ok(())
    }

    fn lock(&mut self) -> io::Result<()> {
        let mut s = self.state();
        if self.ticket.is_some() && s.locked_by == self.ticket {
            return Ok(());
        }
        if s.locked_by.is_some() {
            return Err(io::Error::new(
                io::ErrorKind::WouldBlock,
                "another store holds the memory disk's session lock",
            ));
        }
        s.tickets += 1;
        let ticket = s.tickets;
        s.locked_by = Some(ticket);
        drop(s);
        self.ticket = Some(ticket);
        Ok(())
    }
}

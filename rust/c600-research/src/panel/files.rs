//! The research host's file grant (PS3.17, LB5.18): the files the session's user granted to the host, by name. A row
//! reads a granted file's bytes as the grant captured them and writes only a file granted for writing; a name the
//! grant does not hold is refused with `LG3:capability.not_granted`. The grant is the host's own: nothing here opens a
//! file dialog or reaches a path the grant does not name.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use c600_command::{code, Error};
use c600_session::persist::codec::Json;

/// A file granted for reading: its bytes as the grant captured them, with their SHA-256.
#[derive(Clone, Debug)]
struct Held {
    sha256: String,
    bytes: Vec<u8>,
}

/// The files granted to the research host. A name is the file's name in the archive it belongs to, as the corpus
/// manifest and the claim bindings name it (a repository-relative path for the restoration corpus); the grant maps it
/// to a file on disk.
#[derive(Clone, Debug, Default)]
pub struct Grant {
    reads: BTreeMap<String, Held>,
    writes: BTreeMap<String, PathBuf>,
}

impl Grant {
    pub fn new() -> Grant {
        Grant::default()
    }

    /// Grants the file at `path` for reading under `name`. Its bytes are read once, here: a later change on disk is
    /// not seen until the file is granted again.
    pub fn read(mut self, name: &str, path: &Path) -> std::io::Result<Grant> {
        let bytes = std::fs::read(path)?;
        self.reads.insert(name.to_owned(), Held { sha256: c600_digest::sha256_hex(&bytes), bytes });
        Ok(self)
    }

    /// Grants the file at `path` for writing under `name`. Nothing is written until a row writes it.
    pub fn write(mut self, name: &str, path: &Path) -> Grant {
        self.writes.insert(name.to_owned(), path.to_owned());
        self
    }

    /// The bytes of the file granted for reading under `name`.
    pub fn bytes(&self, name: &str) -> Result<&[u8], Error> {
        self.reads.get(name).map(|held| held.bytes.as_slice()).ok_or_else(|| {
            Error::new(code::NOT_GRANTED, format!("`{name}` is not a file granted to the research host for reading"))
        })
    }

    /// The SHA-256 of the file granted for reading under `name`.
    pub fn digest(&self, name: &str) -> Option<&str> {
        self.reads.get(name).map(|held| held.sha256.as_str())
    }

    /// The bytes of a granted file with this SHA-256, if the grant holds one.
    pub fn by_digest(&self, sha256: &str) -> Option<&[u8]> {
        self.reads.values().find(|held| held.sha256 == sha256).map(|held| held.bytes.as_slice())
    }

    /// Whether `name` is granted for writing.
    pub fn writable(&self, name: &str) -> Result<(), Error> {
        if self.writes.contains_key(name) {
            Ok(())
        } else {
            Err(Error::new(
                code::NOT_GRANTED,
                format!("`{name}` is not a file granted to the research host for writing"),
            ))
        }
    }

    /// Writes `bytes` to the file granted for writing under `name`, whole. The caller builds the bytes in full first,
    /// so a refusal before this point writes nothing.
    pub(crate) fn put(&self, name: &str, bytes: &[u8]) -> Result<(), Error> {
        self.writable(name)?;
        std::fs::write(&self.writes[name], bytes).map_err(|error| {
            Error::new(code::NOT_GRANTED, format!("the granted file `{name}` was not written: {error}"))
        })
    }

    /// The grant as research data: each read's name and digest, and each write's name (never a path).
    pub(crate) fn json(&self) -> Json {
        let reads = self.reads.iter().map(|(name, held)| (name.clone(), Json::text(&held.sha256))).collect();
        let writes = self.writes.keys().map(|name| Json::text(name)).collect();
        Json::object([("reads", Json::Object(reads)), ("writes", Json::List(writes))])
    }
}

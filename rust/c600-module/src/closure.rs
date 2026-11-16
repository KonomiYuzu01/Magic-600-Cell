//! Resolution and verification of a dependency closure (LG5.1, LG5.3, LG5.5, LG5.7, LG5.9).
//!
//! Bytes enter only through [`Files`], the local files the user chose, and are found by content digest; a path or a
//! module name never decides identity. [`verify`] checks, in order: the manifest and the lock (FP.10, FP.5), the lock's
//! root against the manifest bytes, the session identity against `supports`, the resolver and IR contracts, every
//! locked module's bytes against its digest, its parse and edition, the root header against the manifest, the closure
//! against the lock, and the lowering of the whole closure.

use std::collections::{BTreeMap, BTreeSet};

use c600_digest::sha256;
use c600_ir::{Program, EXEC_CONTRACT, IR_VERSION};
use c600_run::provenance::hex;
use c600_syntax::ast::{Item, ParsedFile};

use crate::code::{CONTRACT_MISMATCH, DEPENDENCY_MISSING, DIGEST_MISMATCH, IR_REJECTED, MODEL_NOT_SUPPORTED};
use crate::form::{unhex, Identity, Location, Lock, LockedModule, Manifest};
use crate::{refuse, Refusal, RESOLVER};

/// The local files a resolution or a load may read, by location (LG5.9). A location only finds bytes.
#[derive(Clone, Debug, Default)]
pub struct Files {
    pub entries: BTreeMap<String, Vec<u8>>,
}

impl Files {
    pub fn new() -> Files {
        Files::default()
    }

    pub fn with(mut self, path: &str, bytes: &[u8]) -> Files {
        self.entries.insert(path.to_string(), bytes.to_vec());
        self
    }

    /// The first file, in path order, whose bytes have this digest.
    fn find(&self, digest: &[u8; 32]) -> Option<(&str, &[u8])> {
        self.entries.iter().find(|(_, b)| sha256(b) == *digest).map(|(p, b)| (p.as_str(), b.as_slice()))
    }
}

/// A verified closure: the bytes that were checked are the bytes that run (LG5.1, LG5.6).
#[derive(Clone, Debug)]
pub struct Closure {
    pub manifest: Manifest,
    pub lock: Lock,
    pub manifest_digest: [u8; 32],
    pub lock_digest: [u8; 32],
    /// The closure digest ([`Lock::closure_digest`]).
    pub digest: [u8; 32],
    /// The session identity the closure was verified for (LG5.5).
    pub identity: Identity,
    /// The checked source bytes, by digest.
    pub sources: BTreeMap<[u8; 32], Vec<u8>>,
    /// Each module's name, by source digest.
    pub names: BTreeMap<[u8; 32], String>,
    /// The lowered closure, in the lock's module order.
    pub program: Program,
}

impl Closure {
    /// The source digest of the module with this name.
    pub fn module_digest(&self, name: &str) -> Option<[u8; 32]> {
        self.names.iter().find(|(_, n)| n.as_str() == name).map(|(d, _)| *d)
    }
}

/// Resolves a manifest's dependency closure among local files and writes its lock (LG5.3). The root and every import
/// are found by digest; a missing one is refused.
pub fn resolve(manifest: &[u8], files: &Files) -> Result<Lock, Refusal> {
    let m = Manifest::read(manifest)?;
    let (mut modules, mut locations) = (Vec::new(), Vec::new());
    let mut pending = vec![m.source];
    let mut seen = BTreeSet::new();
    while let Some(digest) = pending.pop() {
        if !seen.insert(digest) {
            continue;
        }
        let Some((path, bytes)) = files.find(&digest) else {
            let detail = format!("no local file has the digest {} (LG5.3, LG5.9)", hex(&digest));
            return Err(refuse(DEPENDENCY_MISSING, detail));
        };
        let parsed = parse(bytes)?;
        let name = header_name(&parsed)?;
        let edition = parsed.edition.clone().unwrap_or_default();
        let ir_version = IR_VERSION.to_string();
        modules.push(LockedModule {
            module: name,
            source: digest,
            edition,
            ir_version,
            exec_contract: EXEC_CONTRACT.into(),
        });
        locations.push(Location { path: path.to_string(), source: digest });
        pending.extend(imports(&parsed)?.into_iter().map(|(_, d)| d));
    }
    Ok(Lock { root: sha256(manifest), modules, locations, resolver: RESOLVER.into() })
}

/// Verifies a manifest, its lock and the local bytes for a session identity, and lowers the closure.
pub fn verify(manifest_bytes: &[u8], lock_bytes: &[u8], files: &Files, session: &Identity) -> Result<Closure, Refusal> {
    let manifest = Manifest::read(manifest_bytes)?;
    let lock = Lock::read(lock_bytes)?;
    let manifest_digest = sha256(manifest_bytes);
    if lock.root != manifest_digest {
        let detail = format!(
            "the lock resolves the manifest {}, not these manifest bytes {} (LG5.3)",
            hex(&lock.root),
            hex(&manifest_digest)
        );
        return Err(refuse(DIGEST_MISMATCH, detail));
    }
    if !manifest.supports.contains(session) {
        return Err(refuse(MODEL_NOT_SUPPORTED, "the manifest does not list the session's model identity (LG5.5)"));
    }
    if lock.resolver != RESOLVER {
        let detail = format!("the lock names the resolver `{}`; this reader knows `{RESOLVER}` (LG5.3)", lock.resolver);
        return Err(refuse(CONTRACT_MISMATCH, detail));
    }
    let mut sources = BTreeMap::new();
    let mut parsed = BTreeMap::new();
    let mut names = BTreeMap::new();
    for m in &lock.modules {
        let what = format!("module `{}` ({})", m.module, hex(&m.source));
        if m.ir_version != IR_VERSION || m.exec_contract != EXEC_CONTRACT {
            let detail = format!(
                "{what} is locked for {} with {}; this runtime runs {IR_VERSION} with {EXEC_CONTRACT} (LG5.3)",
                m.ir_version, m.exec_contract
            );
            return Err(refuse(CONTRACT_MISMATCH, detail));
        }
        let Some(location) = lock.locations.iter().find(|l| l.source == m.source) else {
            return Err(refuse(DEPENDENCY_MISSING, format!("{what} has no local resolution entry (LG5.3)")));
        };
        let Some(bytes) = files.entries.get(&location.path) else {
            let detail = format!("{what} is not at its location `{}` (LG5.3, LG5.9)", location.path);
            return Err(refuse(DEPENDENCY_MISSING, detail));
        };
        let found = sha256(bytes);
        if found != m.source {
            let detail = format!(
                "the bytes at `{}` have the digest {}; the lock fixes {} and different bytes never replace it (LG5.1)",
                location.path,
                hex(&found),
                hex(&m.source)
            );
            return Err(refuse(DIGEST_MISMATCH, detail));
        }
        let p = parse(bytes)?;
        if p.edition.as_deref() != Some(m.edition.as_str()) {
            let detail =
                format!("{what} is locked for edition `{}`; its header names another (LG5.2, LG5.3)", m.edition);
            return Err(refuse(CONTRACT_MISMATCH, detail));
        }
        let name = header_name(&p)?;
        if name != m.module {
            return Err(refuse(CONTRACT_MISMATCH, format!("{what} declares module `{name}` (LG5.3)")));
        }
        if names.values().any(|n| *n == name) {
            return Err(refuse(CONTRACT_MISMATCH, format!("two locked modules are named `{name}` (LG5.3)")));
        }
        names.insert(m.source, name);
        sources.insert(m.source, bytes.clone());
        parsed.insert(m.source, p);
    }
    let Some(root) = parsed.get(&manifest.source) else {
        let detail = format!("the lock does not fix the root source {} (LG5.3)", hex(&manifest.source));
        return Err(refuse(DEPENDENCY_MISSING, detail));
    };
    check_root(&manifest, root)?;
    // The closure the root reaches is exactly the locked set (LG5.3, LG5.7).
    let mut reached = BTreeSet::new();
    let mut pending = vec![manifest.source];
    while let Some(digest) = pending.pop() {
        if !reached.insert(digest) {
            continue;
        }
        for (alias, d) in imports(&parsed[&digest])? {
            if !parsed.contains_key(&d) {
                let detail = format!(
                    "module `{}` imports `{alias}` ({}), which the lock does not fix (LG5.3)",
                    names[&digest],
                    hex(&d)
                );
                return Err(refuse(DEPENDENCY_MISSING, detail));
            }
            pending.push(d);
        }
    }
    if let Some(extra) = parsed.keys().find(|d| !reached.contains(*d)) {
        let detail = format!("the lock fixes module `{}`, which the root does not reach (LG5.3)", names[extra]);
        return Err(refuse(CONTRACT_MISMATCH, detail));
    }
    let files: Vec<ParsedFile> = lock.modules.iter().map(|m| parsed[&m.source].clone()).collect();
    let lowered = c600_syntax::lower(&files).map_err(|e| refuse(IR_REJECTED, e.message))?;
    Ok(Closure {
        digest: lock.closure_digest(),
        lock_digest: sha256(lock_bytes),
        manifest,
        lock,
        manifest_digest,
        identity: session.clone(),
        sources,
        names,
        program: lowered.program,
    })
}

/// The root header agrees with the manifest: edition, name, capabilities and direct imports (LG5.2).
fn check_root(manifest: &Manifest, root: &ParsedFile) -> Result<(), Refusal> {
    if root.edition.as_deref() != Some(manifest.edition.as_str()) {
        let detail =
            format!("the manifest names edition `{}`; the root header names another (LG5.2)", manifest.edition);
        return Err(refuse(CONTRACT_MISMATCH, detail));
    }
    let name = header_name(root)?;
    if name != manifest.module {
        let detail = format!("the manifest names module `{}`; the root declares `{name}` (LG5.2)", manifest.module);
        return Err(refuse(CONTRACT_MISMATCH, detail));
    }
    let header = root.module.as_ref().expect("a named header");
    let requires: BTreeSet<&str> = header.requires.iter().map(|r| r.text.as_str()).collect();
    let declared: BTreeSet<&str> = manifest.capabilities.iter().map(String::as_str).collect();
    if requires != declared {
        let detail = "the manifest's capabilities differ from the root header's `requires` (LG5.2, LG3.9)";
        return Err(refuse(CONTRACT_MISMATCH, detail));
    }
    let imported: BTreeSet<(String, [u8; 32])> = imports(root)?.into_iter().collect();
    let dependencies: BTreeSet<(String, [u8; 32])> =
        manifest.dependencies.iter().map(|d| (d.alias.clone(), d.source)).collect();
    if imported != dependencies {
        let detail = "the manifest's dependencies differ from the root's imports (LG5.2)";
        return Err(refuse(CONTRACT_MISMATCH, detail));
    }
    Ok(())
}

/// Parses source bytes. The first syntax diagnostic refuses with its own code (LG1).
fn parse(bytes: &[u8]) -> Result<ParsedFile, Refusal> {
    let parsed = c600_syntax::parse(bytes);
    if let Some(d) = parsed.diagnostics.first() {
        return Err(refuse(d.code, d.message.clone()));
    }
    Ok(parsed)
}

fn header_name(parsed: &ParsedFile) -> Result<String, Refusal> {
    let header = parsed.module.as_ref().ok_or_else(|| refuse(CONTRACT_MISMATCH, "a source without a module header"))?;
    Ok(header.name.text.clone())
}

/// A file's imports: alias and digest.
fn imports(parsed: &ParsedFile) -> Result<Vec<(String, [u8; 32])>, Refusal> {
    let mut out = Vec::new();
    for item in &parsed.items {
        if let Item::Import(i) = item {
            let digest = unhex(&i.digest).ok_or_else(|| {
                refuse(DEPENDENCY_MISSING, format!("the import `{}` names no SHA-256 digest", i.alias.text))
            })?;
            out.push((i.alias.text.clone(), digest));
        }
    }
    Ok(out)
}

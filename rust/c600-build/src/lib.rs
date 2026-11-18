//! A language evidence builder and its build cache (DET.44 to DET.46; LP2 draft until the IR freeze). It is the host
//! envelope around the interpreter: the builder reads captured inputs and writes the cache and the published files,
//! so it lives outside `c600-run`, which makes no file-system call of its own (KN4.1, CT-KN4-01).
//!
//! - [`Grant`] is the scoped host output grant (DET.45): an output directory and a scratch directory. Each is refused
//!   with `LG3:capability.grant_mismatch` when it is the source tree, lies inside it or holds it. The locations enter
//!   no key and select no branch of computation.
//! - [`build`] runs one exported entry of a sealed module closure (DET.44). The program's modules must be exactly the
//!   sealed ones, so no module outside the closure is loaded, and the interpreter has no module search path. The seed
//!   is the spec's, which the key holds. The run takes the caller's thread and starts none. The builder's host grants
//!   no clock, snapshot, declassifier, preview, view or workspace: a request for one stops the run with
//!   `LG3:capability.not_granted`. It reads only the captured inputs, each by its logical name and type, and it writes
//!   only files at relative paths under the output grant, bounded by the run's output limit.
//! - The build cache (DET.46) lives under the scratch grant. Its key is the SHA-256 of every input needed to validate an
//!   entry: the IR and execution contract, the sealed modules, the entry and its arguments, the seed, the captured
//!   inputs' types and digests, the limits and the tariffs. Before reuse the host checks the entry's key, its recorded
//!   inputs against the inputs captured now, its recorded charges and digests against its result digest, and every
//!   stored value and file against its digest. After a construction it captures the inputs again and stores and
//!   publishes only when they are unchanged. A failed check gives the host-envelope diagnostic
//!   `LG4:input.cache_entry_mismatch`: a rejected entry is a miss, rebuilt with the charges the original run had
//!   (DET.16), and an input that changed during construction leaves nothing stored or published. The checks catch a
//!   changed or damaged entry; they authenticate nothing, so an entry rewritten whole with consistent digests passes,
//!   and the scratch grant is the host's alone.
//!
//! Only a completed run is stored and published. DET.47's atomic publication is chapter 05's: here each file is
//! written beside its place and renamed into it, and CT-DET-23's interruption at every boundary is not run.

#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use c600_digest::sha256_hex;
use c600_ir::{Program, Ty};

use c600_run::encode::encode;
use c600_run::interp::code::{HOST_EXHAUSTED, NONCANONICAL_VALUE};
use c600_run::interp::{load, Host, Options, Refusal, Request, RowTariff, Sink};
use c600_run::meter::code::NOT_GRANTED;
use c600_run::meter::{Amounts, DraftTariffs, Outcome, Tariffs};
use c600_run::provenance::{canonical, parse, Json};
use c600_run::value::Value;

/// The builder's codes.
pub mod code {
    /// A cached entry fails its recorded input or content checks, or an input changed during construction (DET.46).
    pub const CACHE_ENTRY_MISMATCH: &str = "LG4:input.cache_entry_mismatch";
    /// A grant location in the source tree, or a module outside the sealed closure (DET.44, DET.45).
    pub const GRANT_MISMATCH: &str = "LG3:capability.grant_mismatch";
    /// A named input the host cannot capture (DET.41).
    pub const INPUT_MISSING: &str = "LG4:input.repro_input_missing";
}

/// The version of this draft's build key and cache entry.
pub const KEY_VERSION: &str = "c600-build-key-draft-1";

/// The kernel cost of reading one captured input (a draft host tariff; LG4.8 owns the production tariffs).
pub const READ_COST: u128 = 1;

const CACHE: &str = "build-cache";
const BLOBS: &str = "blobs";

fn refuse(code: &'static str, detail: impl Into<String>) -> Refusal {
    Refusal { code, detail: detail.into() }
}

/// The scoped host output grant (DET.45).
#[derive(Clone, Debug)]
pub struct Grant {
    output: PathBuf,
    scratch: PathBuf,
}

impl Grant {
    /// Grants `output` and `scratch`, two existing directories, to builds from the source tree at `source`. Each is
    /// resolved through links first, then refused when it is the source tree, lies inside it or holds it.
    pub fn new(source: &Path, output: &Path, scratch: &Path) -> Result<Grant, Refusal> {
        let resolve = |what: &str, path: &Path| {
            fs::canonicalize(path)
                .ok()
                .filter(|p| p.is_dir())
                .ok_or_else(|| refuse(code::GRANT_MISMATCH, format!("the {what} directory does not resolve (DET.45)")))
        };
        let source = resolve("source", source)?;
        let mut granted = Vec::new();
        for (what, path) in [("output", output), ("scratch", scratch)] {
            let path = resolve(what, path)?;
            if path.starts_with(&source) || source.starts_with(&path) {
                return Err(refuse(
                    code::GRANT_MISMATCH,
                    format!("the {what} grant is the source tree, lies inside it or holds it (DET.45)"),
                ));
            }
            granted.push(path);
        }
        let scratch = granted.pop().expect("a scratch directory");
        let output = granted.pop().expect("an output directory");
        Ok(Grant { output, scratch })
    }

    pub fn output(&self) -> &Path {
        &self.output
    }

    pub fn scratch(&self) -> &Path {
        &self.scratch
    }
}

/// The host's captured inputs (DET.40, DET.41), by logical name: a granted resource scope, never a physical path. A
/// read gives the input's type and its value as it is now.
pub trait Inputs {
    fn read(&mut self, name: &str) -> Option<(Ty, Value)>;
}

/// What to build. Everything here enters the key; the grant's locations do not.
#[derive(Clone, Debug)]
pub struct Spec {
    /// The sealed module closure: each module's name and its source's SHA-256 (DET.44's dependency set).
    pub modules: Vec<(String, String)>,
    pub module: String,
    pub function: String,
    pub args: Vec<Value>,
    /// The sealed 32-byte seed and its stream path (DET.11, DET.61). Without one, a draw is not granted.
    pub seed: Option<([u8; 32], Vec<String>)>,
    /// The logical names of the inputs the build may read, captured before the key is sealed.
    pub inputs: Vec<String>,
    pub limits: Amounts,
    pub tariffs: Tariffs,
    pub draft: DraftTariffs,
}

/// Whether [`build`] ran the entry or reused a verified cache entry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    Built,
    Reused,
}

/// A completed build, stored and published.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Built {
    /// The build key: lowercase SHA-256 hex.
    pub key: String,
    pub source: Source,
    /// The run's charges: the original run's, also for a reused entry (DET.16).
    pub used: Amounts,
    /// The returned value's canonical bytes (DET.59).
    pub value: Vec<u8>,
    /// The published files: each relative path and the SHA-256 of its bytes, in path order.
    pub files: Vec<(String, String)>,
    /// Host-envelope diagnostics: `LG4:input.cache_entry_mismatch` for a rejected entry (DET.46).
    pub diagnostics: Vec<&'static str>,
}

/// A captured input: its type, its value and the SHA-256 of the value's canonical bytes.
struct Captured {
    ty: Ty,
    value: Value,
    sha256: String,
}

/// A cache entry: what the run returned and wrote, and what it was built from.
struct Entry {
    key: String,
    inputs: Json,
    used: Amounts,
    value: Vec<u8>,
    files: BTreeMap<String, Vec<u8>>,
}

/// Builds `spec`'s entry of `program`, or reuses its verified cache entry, and publishes the files it writes under
/// the grant's output directory.
pub fn build(program: &Program, spec: &Spec, inputs: &mut dyn Inputs, grant: &Grant) -> Result<Built, Refusal> {
    sealed(program, spec)?;
    let captured = capture(spec, inputs)?;
    let recorded = recorded(&captured);
    let key = sha256_hex(&canonical(&key(program, spec, &recorded)?));
    let cache = grant.scratch.join(CACHE);
    let mut diagnostics = Vec::new();
    match reuse(&cache, &key, &recorded) {
        Ok(Some(entry)) => {
            publish(&grant.output, &entry.files)?;
            return Ok(built(entry, Source::Reused, diagnostics));
        }
        Ok(None) => {}
        Err(()) => diagnostics.push(code::CACHE_ENTRY_MISMATCH),
    }
    let loaded = load(program)?;
    let path: Vec<&str> = spec.seed.iter().flat_map(|(_, p)| p.iter().map(String::as_str)).collect();
    let options = Options {
        limits: spec.limits,
        tariffs: spec.tariffs,
        draft: spec.draft,
        seed: spec.seed.as_ref().map(|(seed, _)| (&seed[..], &path[..])),
        record: false,
        cancel: None,
        sources: None,
        menu: None,
        relation: None,
    };
    let mut host = Builder { captured: &captured, written: BTreeMap::new() };
    let run = loaded.run(&spec.module, &spec.function, spec.args.clone(), &mut host, &options)?;
    let (Outcome::Completed, Some(value)) = (run.summary.outcome, run.output) else {
        let code = run.summary.code.unwrap_or(HOST_EXHAUSTED);
        return Err(refuse(code, "the build did not complete; nothing is stored or published"));
    };
    // DET.46: the inputs are captured again after construction; a changed one leaves nothing stored or published.
    if recorded != self::recorded(&capture(spec, inputs)?) {
        return Err(refuse(
            code::CACHE_ENTRY_MISMATCH,
            "a captured input changed during construction; nothing is stored or published (DET.46)",
        ));
    }
    let entry = Entry { key, inputs: recorded, used: run.summary.used, value, files: host.written };
    store(&cache, &entry)?;
    publish(&grant.output, &entry.files)?;
    Ok(built(entry, Source::Built, diagnostics))
}

fn built(entry: Entry, source: Source, diagnostics: Vec<&'static str>) -> Built {
    let files = entry.files.iter().map(|(path, bytes)| (path.clone(), sha256_hex(bytes))).collect();
    Built { key: entry.key, source, used: entry.used, value: entry.value, files, diagnostics }
}

/// DET.44: the program's modules are exactly the sealed closure, each once.
fn sealed(program: &Program, spec: &Spec) -> Result<(), Refusal> {
    let loaded: Vec<&str> = program.modules.iter().map(|m| m.name.as_str()).collect();
    let sealed: BTreeSet<&str> = spec.modules.iter().map(|(name, _)| name.as_str()).collect();
    if sealed.len() != spec.modules.len() || loaded.len() != sealed.len() || !loaded.iter().all(|m| sealed.contains(m))
    {
        return Err(refuse(code::GRANT_MISMATCH, "the program's modules are not the sealed closure (DET.44)"));
    }
    Ok(())
}

fn capture(spec: &Spec, inputs: &mut dyn Inputs) -> Result<BTreeMap<String, Captured>, Refusal> {
    let mut captured = BTreeMap::new();
    for name in &spec.inputs {
        let (ty, value) =
            inputs.read(name).ok_or_else(|| refuse(code::INPUT_MISSING, format!("input {name} is not there")))?;
        let sha256 = sha256_hex(&bytes(&value)?);
        captured.insert(name.clone(), Captured { ty, value, sha256 });
    }
    Ok(captured)
}

fn bytes(value: &Value) -> Result<Vec<u8>, Refusal> {
    let mut out = Vec::new();
    encode(value, &mut out).map_err(|_| refuse(NONCANONICAL_VALUE, "a build input has no canonical form"))?;
    Ok(out)
}

/// The captured inputs as a key and an entry record them: each name with its type and its value's digest.
fn recorded(captured: &BTreeMap<String, Captured>) -> Json {
    Json::Object(
        captured
            .iter()
            .map(|(name, c)| {
                let input =
                    Json::object([("type", Json::Text(format!("{:?}", c.ty))), ("sha256", Json::text(&c.sha256))]);
                (name.clone(), input)
            })
            .collect(),
    )
}

/// The key's fields. The tariff tables enter as their field-by-field text, so a changed tariff changes the key.
fn key(program: &Program, spec: &Spec, recorded: &Json) -> Result<Json, Refusal> {
    let modules = spec.modules.iter().map(|(name, sha)| (name.clone(), Json::text(sha))).collect();
    let args = spec.args.iter().map(|a| bytes(a).map(|b| Json::Text(sha256_hex(&b)))).collect::<Result<_, _>>()?;
    let seed = spec.seed.as_ref().map_or(Json::Null, |(seed, path)| {
        Json::object([
            ("sha256", Json::Text(sha256_hex(seed))),
            ("path", Json::List(path.iter().map(|s| Json::text(s)).collect())),
        ])
    });
    Ok(Json::object([
        ("version", Json::text(KEY_VERSION)),
        ("ir_version", Json::text(&program.ir_version)),
        ("exec_contract", Json::text(&program.exec_contract)),
        ("modules", Json::Object(modules)),
        ("entry", Json::List(vec![Json::text(&spec.module), Json::text(&spec.function)])),
        ("args", Json::List(args)),
        ("seed", seed),
        ("inputs", recorded.clone()),
        ("limits", amounts(&spec.limits)),
        ("tariffs", Json::Text(format!("{:?}", spec.tariffs))),
        ("draft_tariffs", Json::Text(format!("{:?}", spec.draft))),
    ]))
}

fn amounts(a: &Amounts) -> Json {
    Json::object([
        ("steps", Json::Text(a.steps.to_string())),
        ("kernel_cost", Json::Text(a.kernel_cost.to_string())),
        ("memory", Json::Text(a.memory.to_string())),
        ("output", Json::Text(a.output.to_string())),
    ])
}

fn read_amounts(json: &Json) -> Option<Amounts> {
    let n = |k: &str| json.get(k)?.as_text()?.parse::<u128>().ok();
    Some(Amounts { steps: n("steps")?, kernel_cost: n("kernel_cost")?, memory: n("memory")?, output: n("output")? })
}

/// The verified entry under `key`, none when there is none, or `Err` when one is there and fails a check.
fn reuse(cache: &Path, key: &str, recorded: &Json) -> Result<Option<Entry>, ()> {
    let Ok(stored) = fs::read(cache.join(format!("{key}.json"))) else {
        return Ok(None);
    };
    let json = parse(&stored).filter(|j| canonical(j) == stored).ok_or(())?;
    let text = |k: &str| json.get(k).and_then(Json::as_text).ok_or(());
    if text("version")? != KEY_VERSION || text("key")? != key || json.get("inputs") != Some(recorded) {
        return Err(());
    }
    let blob = |sha: &str| match fs::read(cache.join(BLOBS).join(sha)) {
        Ok(bytes) if sha256_hex(&bytes) == sha => Ok(bytes),
        _ => Err(()),
    };
    let value = blob(text("value")?)?;
    let Some(Json::Object(listed)) = json.get("files") else {
        return Err(());
    };
    let mut files = BTreeMap::new();
    for (path, sha) in listed {
        files.insert(path.clone(), blob(sha.as_text().ok_or(())?)?);
    }
    let used = json.get("used").and_then(read_amounts).ok_or(())?;
    let entry = Entry { key: key.into(), inputs: recorded.clone(), used, value, files };
    if text("result")? != sha256_hex(&canonical(&result(&entry))) {
        return Err(());
    }
    Ok(Some(entry))
}

/// What a run gave: its charges, its value's digest and its files' digests. The entry holds their digest, so a changed
/// field fails the entry's check.
fn result(entry: &Entry) -> Json {
    let files = entry.files.iter().map(|(path, bytes)| (path.clone(), Json::Text(sha256_hex(bytes)))).collect();
    Json::object([
        ("used", amounts(&entry.used)),
        ("value", Json::Text(sha256_hex(&entry.value))),
        ("files", Json::Object(files)),
    ])
}

fn store(cache: &Path, entry: &Entry) -> Result<(), Refusal> {
    let blobs = cache.join(BLOBS);
    fs::create_dir_all(&blobs).map_err(|_| refuse(HOST_EXHAUSTED, "the build cache cannot be written"))?;
    for bytes in std::iter::once(&entry.value).chain(entry.files.values()) {
        write(&blobs.join(sha256_hex(bytes)), bytes)?;
    }
    let result = result(entry);
    let field = |k: &str| result.get(k).cloned().unwrap_or(Json::Null);
    let json = Json::object([
        ("version", Json::text(KEY_VERSION)),
        ("key", Json::text(&entry.key)),
        ("inputs", entry.inputs.clone()),
        ("result", Json::Text(sha256_hex(&canonical(&result)))),
        ("used", field("used")),
        ("value", field("value")),
        ("files", field("files")),
    ]);
    write(&cache.join(format!("{}.json", entry.key)), &canonical(&json))
}

fn publish(output: &Path, files: &BTreeMap<String, Vec<u8>>) -> Result<(), Refusal> {
    for (path, bytes) in files {
        let target = path.split('/').fold(output.to_path_buf(), |p, segment| p.join(segment));
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).map_err(|_| refuse(HOST_EXHAUSTED, "an output cannot be published"))?;
        }
        write(&target, bytes)?;
    }
    Ok(())
}

/// Writes beside `target`, then renames into it.
fn write(target: &Path, bytes: &[u8]) -> Result<(), Refusal> {
    let mut partial = target.as_os_str().to_owned();
    partial.push(".partial");
    fs::write(&partial, bytes)
        .and_then(|()| fs::rename(&partial, target))
        .map_err(|_| refuse(HOST_EXHAUSTED, "the host cannot write a build file"))
}

/// Whether `path` is a relative path of plain segments: ASCII letters, digits, `.`, `_` and `-`, and no `.` or `..`.
fn plain(path: &str) -> bool {
    path.split('/').all(|segment| {
        !segment.is_empty()
            && segment != "."
            && segment != ".."
            && !segment.ends_with(".partial")
            && segment.bytes().all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
    })
}

/// The builder's host: the captured inputs and the files written so far.
struct Builder<'a> {
    captured: &'a BTreeMap<String, Captured>,
    written: BTreeMap<String, Vec<u8>>,
}

impl Builder<'_> {
    fn input(&self, request: &Request) -> Result<&Captured, &'static str> {
        match *request {
            Request::FileRead { path, ty } => self.captured.get(path).filter(|c| c.ty == *ty).ok_or(NOT_GRANTED),
            _ => Err(NOT_GRANTED),
        }
    }
}

impl Host for Builder<'_> {
    fn tariff(&mut self, request: &Request) -> Result<RowTariff, &'static str> {
        let input = self.input(request)?;
        Ok(RowTariff { kernel_cost: READ_COST, result_bound: input.value.size() })
    }

    fn perform(&mut self, request: &Request) -> Result<Value, &'static str> {
        Ok(self.input(request)?.value.clone())
    }

    fn sink(&mut self, sink: &Sink, _: &Value, bytes: &[u8]) -> Result<(), &'static str> {
        match *sink {
            Sink::File { path } if plain(path) => {
                self.written.insert(path.to_string(), bytes.to_vec());
                Ok(())
            }
            Sink::File { .. } => Err(code::GRANT_MISMATCH),
            _ => Err(NOT_GRANTED),
        }
    }
}

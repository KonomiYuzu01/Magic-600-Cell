//! The 0.4 proof log, `C600-STUDIO-SESSION-v1` (chapter 05's retained session exchange), as interchange: check,
//! import and export with 0.4's `log_io.py` limits, order of checks and texts.
//!
//! `check` is `decode_log` and `validate_record` over the file's bytes: at most 16 MiB stored and 24 MiB expanded,
//! UTF-8 JSON with an optional BOM and no repeated field, the 600-cell-Full model, a solved root, at most 10,000
//! transactions and 2,000,000 primitives, every recipe normalized under 0.4's checks with its counts, and a replay that
//! checks every pre and post state and the final state. Preferences are read and never applied.
//!
//! `export` writes what 0.4's `Session.export` writes for the line from the start to the saved cursor, as canonical
//! JSON (0.4's `canonical`), raw or in a stored gzip member with no time; 0.4's `decode_log` reads both.

use c600_kernel::archive::gunzip;
use c600_kernel::json::{self, Value};
use c600_kernel::{Derivation, Kernel, ProtectedSet, Recipe, Snapshot, Step};

use crate::compile::Compiled;
use crate::log::{Commit, Interchange, Log, MoveAction, MoveEntry, Origin, Saved, Start};
use crate::problem::{code, Gate, Problem};
use crate::record::recipe_value;
use crate::wire;

/// The 0.4 format identifier.
pub const FORMAT: &str = crate::log::C600_ORIGIN;
/// The file extension 0.4 writes, without its leading dot.
pub const EXTENSION: &str = "c600.json.gz";
/// `MAX_LOG_BYTES`: the stored file.
pub const MAX_LOG_BYTES: usize = 16 * 1024 * 1024;
/// `MAX_JSON_BYTES`: the expanded JSON.
pub const MAX_JSON_BYTES: usize = 24 * 1024 * 1024;
/// `MAX_TRANSACTIONS`.
pub const MAX_TRANSACTIONS: usize = 10_000;
/// `MAX_PRIMITIVES`, over all transactions.
pub const MAX_PRIMITIVES: u64 = 2_000_000;

const TRANSACTION_CAP: &str = "Interactive import is capped at 10,000 transactions";
const PRIMITIVE_CAP: &str = "Interactive import exceeds 2,000,000 primitive moves";

/// One checked transaction.
#[derive(Clone, Debug)]
pub struct C600Event {
    /// The transaction's `id` field as canonical JSON text, when it had one; 0.4 does not check it.
    pub id: Option<String>,
    pub recipe: Recipe,
    pub length: u64,
    pub stars: u64,
    pub pre: String,
    pub post: String,
    pub note: String,
    pub assistance: String,
}

/// What `validate_record` returns: the replayed transactions and the final state.
#[derive(Clone, Debug)]
pub struct C600Plan {
    pub events: Vec<C600Event>,
    pub state: Snapshot,
    pub primitive_count: u64,
    /// The file's preferences, recorded and never applied.
    pub prefs: Option<Value>,
}

/// How export stores the JSON.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Compression {
    /// The canonical JSON bytes.
    None,
    /// One gzip member of stored blocks, with no time.
    Gzip,
}

fn problem(code: &'static str, at: impl Into<String>, message: impl Into<String>) -> Problem {
    Problem::new(code, Gate::Interchange, at, message)
}

fn format_problem(at: impl Into<String>, message: impl Into<String>) -> Problem {
    problem(code::PERSISTENCE_FORMAT, at, message)
}

/// Checks a 0.4 proof log's bytes and replays it, changing nothing.
pub fn check(kernel: &Kernel, bytes: &[u8]) -> Result<C600Plan, Problem> {
    let record = decode(bytes)?;
    validate(kernel, &record)
}

/// `decode_log` without its base64 layer: the stored bytes to a JSON value.
pub fn decode(bytes: &[u8]) -> Result<Value, Problem> {
    if bytes.is_empty() || bytes.len() > MAX_LOG_BYTES {
        return Err(problem(code::PERSISTENCE_SIZE, "bytes", "Log file must contain 1 byte..16 MiB"));
    }
    let raw = if bytes.starts_with(&[0x1f, 0x8b]) {
        match gunzip(bytes, MAX_JSON_BYTES) {
            Ok(raw) => raw,
            Err(error) if error.0.contains("exceeds its limit") => {
                return Err(problem(code::PERSISTENCE_SIZE, "bytes", "Expanded log exceeds 24 MiB"))
            }
            Err(_) => return Err(format_problem("bytes", "Corrupt gzip log")),
        }
    } else {
        bytes.to_vec()
    };
    if raw.len() > MAX_JSON_BYTES {
        return Err(problem(code::PERSISTENCE_SIZE, "bytes", "Expanded log exceeds 24 MiB"));
    }
    let text = raw.strip_prefix(b"\xef\xbb\xbf".as_slice()).unwrap_or(&raw);
    json::parse(text).map_err(|_| format_problem("record", "Log must contain valid UTF-8 JSON"))
}

fn hash<'a>(value: Option<&'a Value>, at: &str, name: &str) -> Result<&'a str, Problem> {
    value
        .and_then(Value::as_str)
        .filter(|text| wire::is_sha256(text))
        .ok_or_else(|| format_problem(at, format!("Invalid {name} checksum")))
}

/// 0.4's `_count`: a nonnegative integer, or a decimal string of at most 16 digits. An integer too large for u64 is
/// still a count; it differs from every witness.
fn count(value: Option<&Value>, at: &str, name: &str) -> Result<u64, Problem> {
    let invalid = || format_problem(at, format!("Invalid {name}"));
    match value {
        Some(Value::Int(n)) => u64::try_from(*n).map_err(|_| invalid()),
        Some(Value::Number(lexeme)) if lexeme.bytes().all(|b| b.is_ascii_digit()) => Ok(u64::MAX),
        Some(Value::String(text)) => {
            let canonical = text == "0"
                || (text.len() <= 16
                    && text.starts_with(|c: char| ('1'..='9').contains(&c))
                    && text.bytes().all(|b| b.is_ascii_digit()));
            if canonical {
                text.parse().map_err(|_| invalid())
            } else {
                Err(invalid())
            }
        }
        _ => Err(invalid()),
    }
}

/// `validate_record` over a decoded value.
pub fn validate(kernel: &Kernel, record: &Value) -> Result<C600Plan, Problem> {
    let Value::Object(fields) = record else {
        return Err(format_problem("record", "Session format/model mismatch"));
    };
    match fields.get("format") {
        Some(Value::String(format)) if format == FORMAT => {}
        Some(Value::String(_)) => {
            return Err(problem(code::PERSISTENCE_VERSION, "format", "Session format/model mismatch"));
        }
        _ => return Err(format_problem("format", "Session format/model mismatch")),
    }
    if fields.get("model_id").and_then(Value::as_str) != Some(kernel.identity().model_id.as_str()) {
        return Err(problem(code::IDENTITY_MISMATCH, "model_id", "Session format/model mismatch"));
    }
    if fields.get("root").and_then(Value::as_str) != Some("labelled_identity") {
        return Err(format_problem("root", "Proof log root must be labelled_identity"));
    }
    if fields.get("native_windows_equivalence") != Some(&Value::Bool(false)) {
        return Err(format_problem("native_windows_equivalence", "Unsupported native equivalence claim"));
    }
    let prefs = fields.get("prefs");
    if prefs.is_some_and(|prefs| prefs.as_object().is_none()) {
        return Err(format_problem("prefs", "Log preferences must be an object (they are not imported)"));
    }
    match python_size(record) {
        None => return Err(format_problem("record", "Log contains invalid JSON values")),
        Some(size) if size > MAX_JSON_BYTES => {
            return Err(problem(code::PERSISTENCE_SIZE, "record", "Expanded log exceeds 24 MiB"));
        }
        Some(_) => {}
    }
    let events = match fields.get("events") {
        Some(Value::Array(events)) if events.len() <= MAX_TRANSACTIONS => events,
        Some(Value::Array(_)) => return Err(problem(code::PERSISTENCE_SIZE, "events", TRANSACTION_CAP)),
        _ => return Err(format_problem("events", TRANSACTION_CAP)),
    };
    let final_state = hash(fields.get("final_state"), "final_state", "final-state")?;

    let mut prepared = Vec::with_capacity(events.len());
    let mut total = 0u64;
    for (index, event) in events.iter().enumerate() {
        let at = format!("events[{index}]");
        let Value::Object(event) = event else {
            return Err(format_problem(at, "Log transaction must be an object"));
        };
        let pre = hash(event.get("pre"), &format!("{at}.pre"), "pre-state")?;
        let post = hash(event.get("post"), &format!("{at}.post"), "post-state")?;
        let recipe = kernel
            .recipe(event.get("recipe").unwrap_or(&Value::Null))
            .map_err(|refusal| problem(refusal.code, format!("{at}.recipe"), refusal.message))?;
        let length = recipe.primitive_count() as u64;
        total += length;
        if total > MAX_PRIMITIVES {
            return Err(problem(code::PERSISTENCE_SIZE, format!("{at}.recipe"), PRIMITIVE_CAP));
        }
        let stars = recipe.steps().iter().filter(|step| matches!(step, Step::Star(_))).count() as u64;
        let counted = count(event.get("primitive_count"), &format!("{at}.primitive_count"), "primitive count")?;
        let counted_stars = count(event.get("stars"), &format!("{at}.stars"), "star count")?;
        if counted != length || counted_stars != stars {
            return Err(problem(
                code::CHECKPOINT_REPLAY,
                format!("{at}.primitive_count"),
                "Log transaction count differs from its legal witness",
            ));
        }
        let note = match event.get("note") {
            None => Some("Imported"),
            Some(value) => value.as_str(),
        };
        let assistance = match event.get("assistance") {
            None => Some("unknown"),
            Some(value) => value.as_str(),
        };
        let (Some(note), Some(assistance)) = (note, assistance) else {
            return Err(format_problem(at, "Log note/assistance text is invalid or too long"));
        };
        if note.chars().count() > 500 || assistance.chars().count() > 80 {
            return Err(format_problem(at, "Log note/assistance text is invalid or too long"));
        }
        prepared.push(C600Event {
            id: event.get("id").map(Value::canonical),
            recipe,
            length,
            stars,
            pre: pre.to_owned(),
            post: post.to_owned(),
            note: note.to_owned(),
            assistance: assistance.to_owned(),
        });
    }

    let none = ProtectedSet::none();
    let mut state = kernel.solved();
    for (index, event) in prepared.iter().enumerate() {
        if state.digest() != event.pre {
            return Err(problem(
                code::CHECKPOINT_REPLAY,
                format!("events[{index}].pre"),
                format!("Import pre-state hash mismatch at transaction {index}"),
            ));
        }
        let derivation = kernel
            .derive(&state, &event.recipe, &none)
            .map_err(|error| problem(code::CHECKPOINT_REPLAY, format!("events[{index}]"), error.to_string()))?;
        state = match derivation {
            Derivation::Derived(derived) => derived.state,
            Derivation::Refused(refused) => {
                return Err(problem(refused.refusal.code, format!("events[{index}]"), refused.refusal.message))
            }
        };
        if state.digest() != event.post {
            return Err(problem(
                code::CHECKPOINT_REPLAY,
                format!("events[{index}].post"),
                format!("Import replay mismatch at transaction {index}"),
            ));
        }
    }
    if state.digest() != final_state {
        return Err(problem(code::CHECKPOINT_REPLAY, "final_state", "Import final-state mismatch"));
    }
    Ok(C600Plan { events: prepared, state, primitive_count: total, prefs: prefs.cloned() })
}

/// The length of Python's `json.dumps(value, ensure_ascii=False, allow_nan=False, separators=(",", ":"))` in UTF-8
/// bytes; `None` for a number Python reads as infinite.
fn python_size(value: &Value) -> Option<usize> {
    Some(match value {
        Value::Null | Value::Bool(true) => 4,
        Value::Bool(false) => 5,
        Value::Int(n) => n.to_string().len(),
        Value::Number(lexeme) if lexeme.bytes().all(|b| b.is_ascii_digit() || b == b'-') => lexeme.len(),
        Value::Number(lexeme) => float_repr_len(lexeme.parse::<f64>().ok().filter(|f| f.is_finite())?),
        Value::String(text) => string_size(text),
        Value::Array(items) => {
            2 + items.len().saturating_sub(1) + items.iter().map(python_size).sum::<Option<usize>>()?
        }
        Value::Object(fields) => {
            2 + fields.len().saturating_sub(1)
                + fields.iter().map(|(k, v)| Some(string_size(k) + 1 + python_size(v)?)).sum::<Option<usize>>()?
        }
    })
}

fn string_size(text: &str) -> usize {
    2 + text
        .chars()
        .map(|c| match c {
            '"' | '\\' | '\n' | '\r' | '\t' | '\u{8}' | '\u{c}' => 2,
            c if (c as u32) < 0x20 => 6,
            c => c.len_utf8(),
        })
        .sum::<usize>()
}

/// The length of Python's `repr` of a finite float: the shortest round-trip digits, fixed notation for decimal
/// exponents from -4 to 15 (with `.0` when integral), scientific notation with at least two exponent digits otherwise.
fn float_repr_len(value: f64) -> usize {
    let text = format!("{value:e}");
    let (mantissa, exponent) = text.split_once('e').unwrap_or((&text, "0"));
    let negative = mantissa.starts_with('-');
    let digits = mantissa.bytes().filter(u8::is_ascii_digit).count();
    let exponent: i64 = exponent.parse().unwrap_or(0);
    let sign = usize::from(negative);
    if (-4..16).contains(&exponent) {
        if exponent >= 0 {
            let integer = exponent as usize + 1;
            sign + integer + if digits > integer { 1 + digits - integer } else { 2 }
        } else {
            sign + 2 + (-exponent - 1) as usize + digits
        }
    } else {
        let exponent_digits = exponent.unsigned_abs().to_string().len().max(2);
        sign + digits + usize::from(digits > 1) + 2 + exponent_digits
    }
}

/// Imports a 0.4 proof log as a 1.0 log: one commit per transaction, the saved cursor on the last, the preferences
/// recorded as text. Nothing live changes; making the log current is the explicit import commit.
pub fn import(kernel: &Kernel, bytes: &[u8]) -> Result<Log, Problem> {
    let plan = check(kernel, bytes)?;
    let mut log = Log::new(kernel.identity(), plan.state.digest());
    let mut parent: Option<String> = None;
    for (index, event) in plan.events.iter().enumerate() {
        let id = (index + 1).to_string();
        log.moves.push(MoveEntry {
            id: id.clone(),
            time: None,
            action: MoveAction::Commit(Commit {
                parent: parent.clone(),
                origin: "import".to_owned(),
                name: String::new(),
                note: event.note.clone(),
                assistance: event.assistance.clone(),
                recipe: recipe_value(&event.recipe),
                primitive_count: event.length,
                stars: event.stars,
                pre: event.pre.clone(),
                post: event.post.clone(),
                interchange: Interchange::C600 { id: event.id.clone() },
            }),
        });
        parent = Some(id);
    }
    log.saved = Saved { moves: parent, layout: None, state: plan.state.digest().to_owned() };
    log.preferences = plan.prefs.as_ref().map(Value::canonical);
    log.origin = Origin::C600 { source: c600_digest::sha256_hex(bytes) };
    Ok(log)
}

/// The first problem of a compile that did not bind its state, or the bound log and state.
pub(crate) fn bound(compiled: &Compiled) -> Result<(&Log, &Snapshot), Problem> {
    match (compiled.log(), compiled.state()) {
        (Some(log), Some(state)) => Ok((log, state)),
        _ => Err(compiled.problems().first().cloned().unwrap_or_else(|| {
            Problem::new(code::CHECKPOINT_REPLAY, Gate::Interchange, "log", "the log did not compile")
        })),
    }
}

/// Exports the line from the start to the saved cursor of a compiled log as a 0.4 proof log. A partial undo becomes
/// a transaction of the kept letters from its commit's pre state; workspace actions write nothing.
pub fn export(kernel: &Kernel, compiled: &Compiled, compression: Compression) -> Result<Vec<u8>, Problem> {
    let (log, state) = bound(compiled)?;
    if log.start != Start::Solved {
        return Err(format_problem("payload.log.start", "a 0.4 proof log starts at the solved state"));
    }
    let mut events = Vec::new();
    let mut total = 0u64;
    for entry in log.line(log.saved.moves.as_deref()) {
        let position = Value::Int(events.len() as i64 + 1);
        let event = match &entry.action {
            MoveAction::Commit(commit) => {
                let id = match &commit.interchange {
                    Interchange::C600 { id: Some(id) } => json::parse(id.as_bytes()).unwrap_or(position),
                    _ => position,
                };
                total += commit.primitive_count;
                event(
                    id,
                    commit.recipe.clone(),
                    &commit.pre,
                    &commit.post,
                    commit.primitive_count,
                    commit.stars,
                    &commit.note,
                    &commit.assistance,
                )
            }
            MoveAction::PartialUndo(undo) if undo.keep > 0 => {
                let Some(MoveEntry { action: MoveAction::Commit(commit), .. }) = log.node(&undo.entry) else {
                    return Err(format_problem("payload.log.history.moves", "a partial undo names no commit"));
                };
                let recipe = kernel.recipe(&commit.recipe).map_err(|r| problem(r.code, "recipe", r.message))?;
                let letters = kernel.letters(&recipe).map_err(|r| problem(r.code, "recipe", r.message))?;
                let steps = letters[..undo.keep as usize]
                    .chunks(100_000)
                    .map(|word| {
                        json::object([
                            ("kind", Value::String("word".to_owned())),
                            ("moves", Value::Array(word.iter().map(|&m| Value::Int(i64::from(m))).collect())),
                        ])
                    })
                    .collect();
                total += undo.keep;
                let note = format!("Partial undo of entry {}", undo.entry);
                event(position, Value::Array(steps), &commit.pre, &undo.post, undo.keep, 0, &note, &commit.assistance)
            }
            _ => continue,
        };
        events.push(event);
        if events.len() > MAX_TRANSACTIONS {
            return Err(cap(TRANSACTION_CAP));
        }
        if total > MAX_PRIMITIVES {
            return Err(cap(PRIMITIVE_CAP));
        }
    }
    let prefs = match (&log.origin, &log.preferences) {
        (Origin::C600 { .. }, Some(text)) => json::parse(text.as_bytes()).ok().filter(|v| v.as_object().is_some()),
        _ => None,
    };
    let record = json::object([
        ("events", Value::Array(events)),
        ("final_state", Value::String(state.digest().to_owned())),
        ("format", Value::String(FORMAT.to_owned())),
        ("model_id", Value::String(log.identity.model_id.clone())),
        ("native_windows_equivalence", Value::Bool(false)),
        ("prefs", prefs.unwrap_or_else(|| Value::Object(Default::default()))),
        ("root", Value::String("labelled_identity".to_owned())),
    ]);
    let raw = record.canonical().into_bytes();
    if raw.len() > MAX_JSON_BYTES {
        return Err(problem(code::PERSISTENCE_SIZE, "record", "Proof log exceeds the 24 MiB interactive size limit"));
    }
    let out = match compression {
        Compression::None => raw,
        Compression::Gzip => wire::gzip_stored(&raw),
    };
    if out.len() > MAX_LOG_BYTES {
        return Err(problem(code::PERSISTENCE_SIZE, "record", "Compressed proof log exceeds 16 MiB"));
    }
    Ok(out)
}

fn cap(text: &str) -> Problem {
    problem(
        code::PERSISTENCE_SIZE,
        "events",
        format!("{text}; this history cannot be written as an importable C600 log. Make a session backup instead."),
    )
}

#[allow(clippy::too_many_arguments)]
fn event(
    id: Value,
    recipe: Value,
    pre: &str,
    post: &str,
    length: u64,
    stars: u64,
    note: &str,
    assistance: &str,
) -> Value {
    json::object([
        ("assistance", Value::String(assistance.to_owned())),
        ("id", id),
        ("note", Value::String(note.to_owned())),
        ("post", Value::String(post.to_owned())),
        ("pre", Value::String(pre.to_owned())),
        ("primitive_count", Value::String(length.to_string())),
        ("recipe", recipe),
        ("stars", Value::Int(stars as i64)),
    ])
}

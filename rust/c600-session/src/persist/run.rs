//! The run record (PS1.31) and chapter 09's `ReproKey` (DET.1): M6's draft of the `c600-run-draft` payload that FP.4
//! reserves, and the store's half of reproduction (DET.7).
//!
//! - [`Key`] holds every DET.1 field. Its `parameters` hold what DET.4 adds: the entry and its arguments, the consulted
//!   inputs captured by digest before the key is sealed ([`capture`], DET.41), the effective grants (DET.40), the
//!   profile and the policy identity (BND.17).
//! - [`Record`] keeps the key with its version and digest, the exact payload ([`exact`], DET.6), a safety stop's
//!   wall-clock observation apart from both, the cut-off run it continues (DET.29) or the run whose output it checks.
//! - [`reading`] gives DET.30's distinct results. Only a completed run covers its declared set; a deterministic cut-off
//!   keeps its covered set and leaves the rest open; any other stop reads open. A published output counts as a witness
//!   once a check record that reads complete confirms it (KN7.4).
//! - [`reproduce`] admits a record before execution. A stop that cannot repeat, or bytes that are not there, give `not
//!   run`; another implementation identity, other supplied bytes, or another grant, profile or policy give a refusal;
//!   the re-run's exact payload must equal the recorded one byte for byte. Execution is the caller's: LP3's runtime in
//!   a session, the interpreter in this crate's tests.
//!
//! `read_record` still keeps `c600-run-draft` bytes recorded: FP.4 leaves the reserved descriptors to their own packet,
//! and only [`Record::read`] reads this draft.

use std::collections::{BTreeMap, BTreeSet};

use c600_digest::sha256_hex;
use c600_kernel::{IdentityBlock, Puzzle};
use c600_run::encode::encode;
use c600_run::meter::{Amounts, ChildRecord, Position, Summary};
use c600_run::provenance::{Pin, Profile};
use c600_run::value::Value;

use super::codec::{canonical, charge, content_ref, digest_ref, nat, nat_text, read_canonical, sha_of, Json};
use super::codes::{DIGEST, FORMAT, IDENTITY, PROVENANCE, VERSION};
use super::record::{self, has_exactly, identity_json, reference};
use super::{require, Refusal};

/// FP.4's reserved descriptor of a run record.
pub const FORMAT_ID: &str = "c600-run-draft";
/// The version of this draft's `ReproKey` encoding.
pub const KEY_VERSION: &str = "c600-repro-key-draft-1";
/// The version of a draft policy identity ([`policy_identity`]).
pub const POLICY_VERSION: &str = "c600-policy-draft-1";
/// DET.4's consulted inputs, which a key names by captured digest.
pub const CAPTURED_KINDS: [&str; 5] = ["file", "workspace", "protection", "pick", "provenance"];
/// The capabilities a grant can name in 1.0: network, process and session-file access are never granted (DET.42).
pub const GRANT_KINDS: [&str; 3] = ["file", "clock", "randomness"];

/// The codes of DET.7 and KN7.10.
pub mod codes {
    pub const INPUT_MISSING: &str = "LG4:input.repro_input_missing";
    pub const KEY_MISMATCH: &str = "LG4:input.repro_key_mismatch";
    pub const GRANT_MISMATCH: &str = "LG3:capability.grant_mismatch";
    pub const REPRODUCIBILITY: &str = "LG4:evidence.reproducibility_mismatch";
    pub const SAFETY_STOP: &str = "KN7:resource_limit.safety_stop";
}

const PAYLOAD: [&str; 7] = ["key", "key_version", "key_digest", "result", "safety_stop", "continues", "checks"];
const RESULT: [&str; 7] = ["outcome", "code", "stop", "used", "children", "value", "outputs"];
const OUTCOMES: [&str; 4] = ["completed", "cut_off", "error", "cancelled"];
const BUDGET_PREFIX: &str = "KN7:budget.";

/// A consulted input captured before the key is sealed (DET.4, DET.41): its kind, its logical name (a granted resource
/// scope or a record name, never a physical path, DET.40) and the SHA-256 and size of its bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Captured {
    pub kind: String,
    pub name: String,
    pub sha256: String,
    pub size: u128,
}

/// An effective grant (DET.40): the capability, its rights, its logical resource scope and the SHA-256 of the grantee
/// module's source. The run's profile and policy identity apply to every grant: a run has one of each (BND.17).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Grant {
    pub kind: String,
    pub rights: Vec<String>,
    pub scope: String,
    pub grantee: String,
}

/// A sealed generator input (DET.11, DET.61): the SHA-256 of the 32-byte seed, the stream path and the algorithm.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Seed {
    pub sha256: String,
    pub path: Vec<String>,
    pub algorithm: String,
}

/// DET.1's `parameters` with what DET.4 and DET.40 fix there.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Parameters {
    /// The module and exported function the run called.
    pub module: String,
    pub function: String,
    /// The arguments, each as the FP.5 value of its canonical bytes (DET.59).
    pub args: Vec<Json>,
    pub captured: Vec<Captured>,
    pub grants: Vec<Grant>,
    pub profile: Profile,
    pub policy: Pin,
}

/// DET.1's `ReproKey`. Digests are lowercase SHA-256 hex.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Key {
    /// The checked source files.
    pub source: Vec<String>,
    /// The executed module closure: each module's name and its source's digest.
    pub modules: Vec<(String, String)>,
    pub edition: String,
    pub ir_version: String,
    pub identity: IdentityBlock,
    /// `Digest<level>`: the level (FP.8's `labels` or `poses`) and the digest.
    pub level: String,
    pub input: String,
    pub parameters: Parameters,
    pub seeds: Vec<Seed>,
    /// The deterministic limits and the unit and metering revision they are counted in (DET.29).
    pub limits: Amounts,
    pub metering: Pin,
    pub kernel_cost_table: Pin,
    pub kernel_api: Pin,
    pub checkers: Vec<Pin>,
}

/// A run record (PS1.31).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Record {
    pub key: Key,
    /// The exact payload ([`exact`]).
    pub result: Json,
    /// A safety stop's elapsed wall-clock milliseconds: host metadata, outside the key and the exact payload.
    pub safety_stop: Option<u128>,
    /// The cut-off run this run continues, by FP.11 Ref (DET.29).
    pub continues: Option<Json>,
    /// The run, by Ref, and the index of its published output that this run checks: a check is a record of its own.
    pub checks: Option<(Json, u128)>,
    /// FP.17's block of the run's program. Its profile, module and entry are the key's.
    pub provenance: Json,
}

/// How a run ended: the interpreter's terminal record, or a wall-clock safety stop that left none (KN7.10).
#[derive(Clone, Copy, Debug)]
pub enum Ending<'a> {
    Run { summary: &'a Summary, output: Option<&'a [u8]> },
    SafetyStop,
}

/// DET.30's coverage of a run record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Coverage {
    /// The run completed: its answer covers its declared set, and an empty answer reads `Empty`.
    Complete,
    /// A deterministic budget ended the run: its covered set, the outputs published before the stop boundary, is kept
    /// and the unexamined part is open. Without a checked witness it reads `not found within budget`.
    WithinBudget,
    /// Cancelled, stopped by the wall clock or by an error, or holding a clock grant: open, establishing no absence.
    /// The reason is the recorded stop code, or `LG4:input.repro_input_missing` for a clock grant (DET.41).
    Open(String),
}

/// What a run record supports (DET.30, KN7.4).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Reading {
    pub coverage: Coverage,
    /// The indices of published outputs that a completed check record confirmed, each once.
    pub witnesses: Vec<u128>,
}

impl Reading {
    /// Whether the record can support a negative claim: only a completed run without a clock grant can (DET.30).
    pub fn may_deny(&self) -> bool {
        self.coverage == Coverage::Complete
    }
}

/// What a host would run with now: the implementation identities and the effective admission (DET.7).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Admission {
    pub edition: String,
    pub ir_version: String,
    pub identity: IdentityBlock,
    pub metering: Pin,
    pub kernel_cost_table: Pin,
    pub kernel_api: Pin,
    pub checkers: Vec<Pin>,
    pub profile: Profile,
    pub policy: Pin,
    pub grants: Vec<Grant>,
}

impl Admission {
    /// The admission a key was sealed under.
    pub fn of(key: &Key) -> Admission {
        Admission {
            edition: key.edition.clone(),
            ir_version: key.ir_version.clone(),
            identity: key.identity.clone(),
            metering: key.metering.clone(),
            kernel_cost_table: key.kernel_cost_table.clone(),
            kernel_api: key.kernel_api.clone(),
            checkers: key.checkers.clone(),
            profile: key.parameters.profile,
            policy: key.parameters.policy.clone(),
            grants: key.parameters.grants.clone(),
        }
    }
}

/// The bytes a reproduction runs on, each checked against the key's digest.
#[derive(Debug, Default)]
pub struct Inputs<'a> {
    /// The key's source files, in key order.
    pub sources: Vec<&'a [u8]>,
    /// Each module's source, by module name.
    pub modules: BTreeMap<&'a str, &'a [u8]>,
    /// The seeds, in key order.
    pub seeds: Vec<&'a [u8]>,
    /// The captured inputs, by kind and name.
    pub captured: BTreeMap<(&'a str, &'a str), &'a [u8]>,
}

/// DET.7's answer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Reproduction {
    /// The re-run's exact payload, byte-equal to the record's (DET.5).
    Same(Json),
    /// Not run, with its reason.
    NotRun(String),
    Refused(String),
}

/// DET.41: captures a mutable input before the key is sealed. `read` is called twice; bytes that change between the
/// reads invalidate admission with `PS1:input.digest_mismatch` (R-08), and nothing is captured.
pub fn capture(kind: &str, name: &str, mut read: impl FnMut() -> Vec<u8>) -> Result<(Captured, Vec<u8>), Refusal> {
    require(CAPTURED_KINDS.contains(&kind), FORMAT)?;
    let bytes = read();
    require(read() == bytes, DIGEST)?;
    let captured =
        Captured { kind: kind.into(), name: name.into(), sha256: sha256_hex(&bytes), size: bytes.len() as u128 };
    Ok((captured, bytes))
}

/// The draft policy identity (BND.17, DET.40): a pin over the FP.5 bytes of the enabled declassifiers, sorted and each
/// once. Whatever LQ2's option set adds to a policy enters these bytes and changes the identity.
pub fn policy_identity(declassifiers: &[&str]) -> Pin {
    let mut ids = declassifiers.to_vec();
    ids.sort_unstable();
    ids.dedup();
    let value = Json::object([("declassifiers", Json::List(ids.into_iter().map(Json::text).collect()))]);
    Pin::of(POLICY_VERSION, &canonical(&value))
}

/// DET.6's exact payload: the outcome, its stop code and deterministic stop boundary, the used counters, the child
/// ledger, the returned value's bytes and the values published before the stop, by digest. Meter rows and live
/// counters are host diagnostics and stay out. A safety stop has no boundary and no counters.
pub fn exact(ending: &Ending, published: &[Vec<u8>]) -> Json {
    let outputs = Json::List(published.iter().map(|bytes| digest_ref("bytes", bytes)).collect());
    match ending {
        Ending::SafetyStop => Json::object([
            ("outcome", Json::text("cut_off")),
            ("code", Json::text(codes::SAFETY_STOP)),
            ("stop", Json::Null),
            ("used", Json::Null),
            ("children", Json::List(Vec::new())),
            ("value", Json::Null),
            ("outputs", outputs),
        ]),
        Ending::Run { summary, output } => Json::object([
            ("outcome", Json::text(summary.outcome.name())),
            ("code", summary.code.map_or(Json::Null, Json::text)),
            ("stop", summary.stop.as_ref().map_or(Json::Null, position)),
            ("used", amounts(&summary.used)),
            ("children", Json::List(summary.children.iter().map(child).collect())),
            ("value", output.map_or(Json::Null, |bytes| digest_ref("bytes", bytes))),
            ("outputs", outputs),
        ]),
    }
}

/// DET.30 and KN7.4: the record's coverage, and the outputs that `checks` confirm. A check confirms an output only when
/// it reads complete itself: a check under a clock grant, like any open run, confirms nothing.
pub fn reading(run: &Record, checks: &[Record]) -> Reading {
    let coverage = match (run.outcome(), run.code()) {
        _ if run.key.parameters.grants.iter().any(|grant| grant.kind == "clock") => {
            Coverage::Open(codes::INPUT_MISSING.into())
        }
        ("completed", _) => Coverage::Complete,
        ("cut_off", Some(code)) if code.starts_with(BUDGET_PREFIX) => Coverage::WithinBudget,
        (outcome, code) => Coverage::Open(code.unwrap_or(outcome).into()),
    };
    let target = reference(&run.json());
    let confirmed = sha256_hex(&encoded(&Value::Bool(true)));
    let outputs = run.outputs();
    let mut witnesses: Vec<u128> = checks
        .iter()
        .filter_map(|check| {
            let (checked, index) = check.checks.as_ref()?;
            let witness = outputs.get(usize::try_from(*index).ok()?)?;
            let holds = *checked == target
                && reading(check, &[]).coverage == Coverage::Complete
                && check.value() == Some(confirmed.as_str())
                && check.key.consumes(witness);
            holds.then_some(*index)
        })
        .collect();
    witnesses.sort_unstable();
    witnesses.dedup();
    Reading { coverage, witnesses }
}

/// DET.7's `reproduce`: reads the record, admits it against `now` and the bytes in `available` (by SHA-256 hex), and
/// runs it through `run`, which receives the checked bytes and answers the re-run's exact payload or its own `not
/// run` or refusal. Checkers and grants compare as sets. Execution never writes: whatever the re-run publishes is the
/// caller's to discard. A record that cannot repeat (a safety stop, a cancellation, an error or a clock grant) answers
/// `not run` first, whatever `now` is; for any other record, admission precedes execution.
pub fn reproduce(
    bytes: &[u8],
    available: &BTreeMap<String, Vec<u8>>,
    now: &Admission,
    run: impl FnOnce(&Key, &Inputs) -> Result<Json, Reproduction>,
) -> Reproduction {
    let record = match Record::read(bytes) {
        Ok(record) => record,
        Err(refusal) => return Reproduction::Refused(refusal.code.into()),
    };
    if let Coverage::Open(reason) = reading(&record, &[]).coverage {
        return Reproduction::NotRun(reason);
    }
    let key = &record.key;
    let implementation = now.edition == key.edition
        && now.ir_version == key.ir_version
        && now.identity == key.identity
        && now.metering == key.metering
        && now.kernel_cost_table == key.kernel_cost_table
        && now.kernel_api == key.kernel_api
        && same_items(&now.checkers, &key.checkers, Pin::json);
    if !implementation {
        return Reproduction::Refused(codes::KEY_MISMATCH.into());
    }
    let parameters = &key.parameters;
    let granted = same_items(&now.grants, &parameters.grants, Grant::json);
    if now.profile != parameters.profile || now.policy != parameters.policy || !granted {
        return Reproduction::Refused(codes::GRANT_MISMATCH.into());
    }
    let fetch = |sha256: &str| -> Result<&[u8], Reproduction> {
        match available.get(sha256) {
            None => Err(Reproduction::NotRun(codes::INPUT_MISSING.into())),
            Some(bytes) if sha256_hex(bytes) != sha256 => Err(Reproduction::Refused(codes::KEY_MISMATCH.into())),
            Some(bytes) => Ok(bytes),
        }
    };
    let inputs = (|| {
        let mut inputs = Inputs::default();
        for sha256 in &key.source {
            inputs.sources.push(fetch(sha256)?);
        }
        for (name, sha256) in &key.modules {
            inputs.modules.insert(name.as_str(), fetch(sha256)?);
        }
        for seed in &key.seeds {
            inputs.seeds.push(fetch(&seed.sha256)?);
        }
        for captured in &parameters.captured {
            inputs.captured.insert((captured.kind.as_str(), captured.name.as_str()), fetch(&captured.sha256)?);
        }
        Ok(inputs)
    })();
    let inputs = match inputs {
        Ok(inputs) => inputs,
        Err(answer) => return answer,
    };
    match run(key, &inputs) {
        Ok(result) if canonical(&result) == canonical(&record.result) => Reproduction::Same(result),
        Ok(_) => Reproduction::Refused(codes::REPRODUCIBILITY.into()),
        Err(answer) => answer,
    }
}

impl Key {
    /// FP.5's value of the key. Captured inputs, grants and checkers are sets: they are sorted by their bytes, so the
    /// key does not depend on the order the host gathered them in, and a key read back holds them in that order.
    pub fn json(&self) -> Json {
        let p = &self.parameters;
        let parameters = Json::object([
            ("entry", Json::object([("module", Json::text(&p.module)), ("function", Json::text(&p.function))])),
            ("args", Json::List(p.args.clone())),
            ("captured", sorted(p.captured.iter().map(Captured::json))),
            ("grants", sorted(p.grants.iter().map(Grant::json))),
            ("profile", Json::text(p.profile.name())),
            ("policy", p.policy.json()),
        ]);
        Json::object([
            ("source", Json::List(self.source.iter().map(|sha| sha_ref("source", sha)).collect())),
            (
                "modules",
                Json::List(
                    self.modules
                        .iter()
                        .map(|(name, sha)| {
                            Json::object([("name", Json::text(name)), ("source", sha_ref("source", sha))])
                        })
                        .collect(),
                ),
            ),
            ("edition", Json::text(&self.edition)),
            ("ir_version", Json::text(&self.ir_version)),
            ("identity", identity_json(&self.identity)),
            ("input", sha_ref(&self.level, &self.input)),
            ("parameters", parameters),
            ("seeds", Json::List(self.seeds.iter().map(Seed::json).collect())),
            ("budgets", Json::object([("limits", amounts(&self.limits)), ("metering", self.metering.json())])),
            ("kernel_cost_table", self.kernel_cost_table.json()),
            ("kernel_api", self.kernel_api.json()),
            ("checkers", sorted(self.checkers.iter().map(Pin::json))),
        ])
    }

    /// The SHA-256 hex of the key's FP.5 bytes.
    pub fn digest(&self) -> String {
        sha256_hex(&canonical(&self.json()))
    }

    /// Reads a key; its value must be the canonical form [`Key::json`] writes, with each grant and checker once, one
    /// capture per kind and name, and each module name once.
    pub fn read(value: &Json) -> Result<Key, Refusal> {
        let fields = [
            "source",
            "modules",
            "edition",
            "ir_version",
            "identity",
            "input",
            "parameters",
            "seeds",
            "budgets",
            "kernel_cost_table",
            "kernel_api",
            "checkers",
        ];
        require(has_exactly(value, &fields), FORMAT)?;
        let p = value.get("parameters").expect("a checked field");
        let fields = ["entry", "args", "captured", "grants", "profile", "policy"];
        require(has_exactly(p, &fields), FORMAT)?;
        let entry = p.get("entry").expect("a checked field");
        require(has_exactly(entry, &["module", "function"]), FORMAT)?;
        let profile = match p.get("profile").and_then(Json::as_text) {
            Some("solving") => Profile::Solving,
            Some("tool_authoring") => Profile::ToolAuthoring,
            Some("research") => Profile::Research,
            _ => return Err(Refusal::new(FORMAT)),
        };
        let parameters = Parameters {
            module: text(entry, "module")?,
            function: text(entry, "function")?,
            args: list(p, "args")?.to_vec(),
            captured: list(p, "captured")?.iter().map(Captured::read).collect::<Result<_, _>>()?,
            grants: list(p, "grants")?.iter().map(Grant::read).collect::<Result<_, _>>()?,
            profile,
            policy: read_pin(p.get("policy"))?,
        };
        let budgets = value.get("budgets").expect("a checked field");
        require(has_exactly(budgets, &["limits", "metering"]), FORMAT)?;
        let input = value.get("input").expect("a checked field");
        let level = text(input, "role")?;
        require(level == "labels" || level == "poses", FORMAT)?;
        let module = |m: &Json| -> Result<(String, String), Refusal> {
            require(has_exactly(m, &["name", "source"]), FORMAT)?;
            Ok((text(m, "name")?, sha(m.get("source"), "source")?))
        };
        let key = Key {
            source: list(value, "source")?.iter().map(|s| sha(Some(s), "source")).collect::<Result<_, _>>()?,
            modules: list(value, "modules")?.iter().map(module).collect::<Result<_, _>>()?,
            edition: text(value, "edition")?,
            ir_version: text(value, "ir_version")?,
            identity: read_identity(value.get("identity"))?,
            input: sha(Some(input), &level)?,
            level,
            parameters,
            seeds: list(value, "seeds")?.iter().map(Seed::read).collect::<Result<_, _>>()?,
            limits: read_amounts(budgets.get("limits"))?,
            metering: read_pin(budgets.get("metering"))?,
            kernel_cost_table: read_pin(value.get("kernel_cost_table"))?,
            kernel_api: read_pin(value.get("kernel_api"))?,
            checkers: list(value, "checkers")?.iter().map(|c| read_pin(Some(c))).collect::<Result<_, _>>()?,
        };
        require(key.json() == *value, FORMAT)?;
        let once = |items: &[Json]| items.windows(2).all(|pair| pair[0] != pair[1]);
        let mut names: Vec<(&str, &str)> = key.parameters.captured.iter().map(|c| (&*c.kind, &*c.name)).collect();
        names.sort_unstable();
        let named_once = names.windows(2).all(|pair| pair[0] != pair[1]);
        let mut modules: Vec<&str> = key.modules.iter().map(|(name, _)| name.as_str()).collect();
        modules.sort_unstable();
        let modules_once = modules.windows(2).all(|pair| pair[0] != pair[1]);
        let sets = once(list(p, "grants")?) && once(list(value, "checkers")?);
        require(named_once && modules_once && sets, FORMAT)?;
        Ok(key)
    }

    /// Whether the run took in the bytes with this digest: as an argument's canonical bytes or as a captured input.
    fn consumes(&self, sha256: &str) -> bool {
        let p = &self.parameters;
        p.args.iter().any(|arg| sha256_hex(&canonical(arg)) == sha256) || p.captured.iter().any(|c| c.sha256 == sha256)
    }
}

impl Record {
    /// FP.11's record. Its identity is the key's.
    pub fn json(&self) -> Json {
        let key = self.key.json();
        let payload = Json::object([
            ("key_digest", content_ref(&key)),
            ("key", key),
            ("key_version", Json::text(KEY_VERSION)),
            ("result", self.result.clone()),
            ("safety_stop", self.safety_stop.map_or(Json::Null, |ms| Json::object([("elapsed_ms", nat_text(ms))]))),
            ("continues", self.continues.clone().unwrap_or(Json::Null)),
            (
                "checks",
                self.checks.as_ref().map_or(Json::Null, |(run, index)| {
                    Json::object([("run", run.clone()), ("output", nat_text(*index))])
                }),
            ),
        ]);
        Json::object([
            ("format", Json::text(FORMAT_ID)),
            ("version", Json::Int(1)),
            ("identity", identity_json(&self.key.identity)),
            ("payload", payload),
            ("provenance", self.provenance.clone()),
        ])
    }

    /// The record's FP.5 bytes, after the checks [`Record::read`] makes.
    pub fn write(&self) -> Result<Vec<u8>, Refusal> {
        let value = self.json();
        record::check_value(&value, 0)?;
        let bytes = canonical(&value);
        charge("record_decoded", bytes.len() as u128)?;
        Record::read(&bytes)?;
        Ok(bytes)
    }

    /// Reads a run record: canonical bytes, this draft's format and version, the header, the provenance block, the key
    /// and its digest, the identity, the exact payload's shape and the references.
    pub fn read(bytes: &[u8]) -> Result<Record, Refusal> {
        let value = read_canonical(bytes)?;
        require(value.get("format").is_some() && value.get("version").is_some(), FORMAT)?;
        let format = value.get("format").and_then(Json::as_text);
        require(format == Some(FORMAT_ID) && value.get("version") == Some(&Json::Int(1)), VERSION)?;
        record::header_fields(&value)?;
        require(value.get("provenance") != Some(&Json::Null), FORMAT)?;
        record::check_provenance(&value, "run")?;
        let payload = value.get("payload").expect("a checked header");
        require(has_exactly(payload, &PAYLOAD), FORMAT)?;
        require(payload.get("key_version").and_then(Json::as_text) == Some(KEY_VERSION), VERSION)?;
        let key_value = payload.get("key").expect("a checked payload");
        let key = Key::read(key_value)?;
        require(payload.get("key_digest") == Some(&content_ref(key_value)), DIGEST)?;
        require(value.get("identity") == Some(&identity_json(&key.identity)), IDENTITY)?;
        let result = payload.get("result").expect("a checked payload").clone();
        check_result(&result)?;
        let safety_stop = match payload.get("safety_stop") {
            Some(Json::Null) => None,
            Some(observed) => {
                require(has_exactly(observed, &["elapsed_ms"]), FORMAT)?;
                Some(nat(observed.get("elapsed_ms").expect("a checked field"))?)
            }
            None => unreachable!("a checked payload"),
        };
        let stopped_by_clock = result.get("code").and_then(Json::as_text) == Some(codes::SAFETY_STOP);
        require(safety_stop.is_some() == stopped_by_clock, FORMAT)?;
        let continues = match payload.get("continues") {
            Some(Json::Null) => None,
            Some(run) => Some(run_ref(run)?),
            None => unreachable!("a checked payload"),
        };
        let checks = match payload.get("checks") {
            Some(Json::Null) => None,
            Some(checks) => {
                require(has_exactly(checks, &["run", "output"]), FORMAT)?;
                let index = nat(checks.get("output").expect("a checked field"))?;
                Some((run_ref(checks.get("run").expect("a checked field"))?, index))
            }
            None => unreachable!("a checked payload"),
        };
        let provenance = value.get("provenance").expect("a checked header").clone();
        let p = &key.parameters;
        let module = key.modules.iter().find(|(name, _)| *name == p.module).map(|(_, sha)| sha_ref("source", sha));
        let producer = provenance.get("producer");
        let produced = provenance.get("profile").and_then(Json::as_text) == Some(p.profile.name())
            && module.is_some()
            && producer.and_then(|b| b.get("module")) == module.as_ref()
            && producer.and_then(|b| b.get("entry")).and_then(Json::as_text) == Some(p.function.as_str());
        require(produced, PROVENANCE)?;
        Ok(Record { key, result, safety_stop, continues, checks, provenance })
    }

    /// The recorded outcome: `completed`, `cut_off`, `error` or `cancelled`.
    pub fn outcome(&self) -> &str {
        self.result.get("outcome").and_then(Json::as_text).unwrap_or("")
    }

    /// The recorded stop code, if any.
    pub fn code(&self) -> Option<&str> {
        self.result.get("code").and_then(Json::as_text)
    }

    /// The SHA-256 hex of the returned value's bytes, if the run returned one.
    pub fn value(&self) -> Option<&str> {
        self.result.get("value").and_then(sha_of)
    }

    /// The SHA-256 hex of each value published before the stop, in order.
    pub fn outputs(&self) -> Vec<&str> {
        match self.result.get("outputs") {
            Some(Json::List(items)) => items.iter().filter_map(sha_of).collect(),
            _ => Vec::new(),
        }
    }
}

impl Captured {
    fn json(&self) -> Json {
        Json::object([
            ("kind", Json::text(&self.kind)),
            ("name", Json::text(&self.name)),
            ("bytes", sha_ref("bytes", &self.sha256)),
            ("size", nat_text(self.size)),
        ])
    }

    fn read(value: &Json) -> Result<Captured, Refusal> {
        require(has_exactly(value, &["kind", "name", "bytes", "size"]), FORMAT)?;
        let kind = text(value, "kind")?;
        require(CAPTURED_KINDS.contains(&kind.as_str()), FORMAT)?;
        let sha256 = sha(value.get("bytes"), "bytes")?;
        let size = nat(value.get("size").expect("a checked field"))?;
        Ok(Captured { kind, name: text(value, "name")?, sha256, size })
    }
}

impl Grant {
    fn json(&self) -> Json {
        Json::object([
            ("kind", Json::text(&self.kind)),
            ("rights", Json::List(self.rights.iter().map(|right| Json::text(right)).collect())),
            ("scope", Json::text(&self.scope)),
            ("grantee", sha_ref("source", &self.grantee)),
        ])
    }

    fn read(value: &Json) -> Result<Grant, Refusal> {
        require(has_exactly(value, &["kind", "rights", "scope", "grantee"]), FORMAT)?;
        let kind = text(value, "kind")?;
        require(GRANT_KINDS.contains(&kind.as_str()), FORMAT)?;
        let rights = list(value, "rights")?;
        let rights = rights.iter().map(|r| r.as_text().map(str::to_owned).ok_or(Refusal::new(FORMAT)));
        let rights = rights.collect::<Result<_, _>>()?;
        Ok(Grant { kind, rights, scope: text(value, "scope")?, grantee: sha(value.get("grantee"), "source")? })
    }
}

impl Seed {
    fn json(&self) -> Json {
        Json::object([
            ("seed", sha_ref("bytes", &self.sha256)),
            ("path", Json::List(self.path.iter().map(|step| Json::text(step)).collect())),
            ("algorithm", Json::text(&self.algorithm)),
        ])
    }

    fn read(value: &Json) -> Result<Seed, Refusal> {
        require(has_exactly(value, &["seed", "path", "algorithm"]), FORMAT)?;
        let path = list(value, "path")?;
        let path = path.iter().map(|s| s.as_text().map(str::to_owned).ok_or(Refusal::new(FORMAT)));
        let path = path.collect::<Result<_, _>>()?;
        Ok(Seed { sha256: sha(value.get("seed"), "bytes")?, path, algorithm: text(value, "algorithm")? })
    }
}

/// The exact payload's shape: the fields of [`exact`], a known outcome with its code (none for a completed run, a text
/// code for any other stop, and the safety-stop code only on a cut-off), the returned value and the published outputs
/// as `bytes` digests. The stop boundary, the counters and the child ledger are compared whole.
fn check_result(result: &Json) -> Result<(), Refusal> {
    require(has_exactly(result, &RESULT), FORMAT)?;
    let outcome = result.get("outcome").and_then(Json::as_text);
    require(outcome.is_some_and(|o| OUTCOMES.contains(&o)), FORMAT)?;
    let code = match result.get("code") {
        Some(Json::Null) => None,
        Some(Json::Text(code)) => Some(code.as_str()),
        _ => return Err(Refusal::new(FORMAT)),
    };
    require((outcome == Some("completed")) == code.is_none(), FORMAT)?;
    require(code != Some(codes::SAFETY_STOP) || outcome == Some("cut_off"), FORMAT)?;
    match result.get("value") {
        Some(Json::Null) => {}
        value => drop(sha(value, "bytes")?),
    }
    list(result, "outputs")?.iter().try_for_each(|output| sha(Some(output), "bytes").map(drop))
}

/// An FP.11 Ref to another run record: dispatched as this draft, then shaped.
fn run_ref(value: &Json) -> Result<Json, Refusal> {
    require(value.get("format").is_some() && value.get("version").is_some(), FORMAT)?;
    let format = value.get("format").and_then(Json::as_text);
    require(format == Some(FORMAT_ID) && value.get("version") == Some(&Json::Int(1)), VERSION)?;
    require(has_exactly(value, &["format", "version", "content"]), FORMAT)?;
    sha(value.get("content"), "content")?;
    Ok(value.clone())
}

fn position(p: &Position) -> Json {
    let source = p.source.as_ref().map_or(Json::Null, |s| {
        Json::object([
            ("digest", Json::text(&s.digest)),
            ("start", nat_text(s.start as u128)),
            ("end", nat_text(s.end as u128)),
        ])
    });
    Json::object([
        ("at", Json::text(&p.at)),
        ("phase", Json::text(p.phase)),
        ("scope", Json::text(&p.scope)),
        ("source", source),
    ])
}

fn amounts(a: &Amounts) -> Json {
    Json::object([
        ("steps", nat_text(a.steps)),
        ("kernel_cost", nat_text(a.kernel_cost)),
        ("memory", nat_text(a.memory)),
        ("output", nat_text(a.output)),
    ])
}

fn read_amounts(value: Option<&Json>) -> Result<Amounts, Refusal> {
    let value = value.ok_or(Refusal::new(FORMAT))?;
    require(has_exactly(value, &["steps", "kernel_cost", "memory", "output"]), FORMAT)?;
    let field = |name: &str| nat(value.get(name).expect("a checked field"));
    Ok(Amounts {
        steps: field("steps")?,
        kernel_cost: field("kernel_cost")?,
        memory: field("memory")?,
        output: field("output")?,
    })
}

fn child(c: &ChildRecord) -> Json {
    Json::object([
        ("at", Json::text(&c.at)),
        ("requested", amounts(&c.requested)),
        ("effective", amounts(&c.effective)),
        ("outcome", Json::text(c.outcome.name())),
        ("code", c.code.map_or(Json::Null, Json::text)),
        ("stop", c.stop.as_ref().map_or(Json::Null, position)),
        ("used", amounts(&c.used)),
        ("returned_memory", nat_text(c.returned_memory)),
        ("live_after_return", nat_text(c.live_after_return)),
    ])
}

/// An identity block [`identity_json`] wrote: 600-cell-Full has no jumbling field; a jumbling state has its contract
/// revision, and its menu identity and order together or, in free mode, neither (KN0.1).
fn read_identity(value: Option<&Json>) -> Result<IdentityBlock, Refusal> {
    let value = value.ok_or(Refusal::new(FORMAT))?;
    let fields = ["puzzle", "model_id", "menu_id", "menu_order", "contract_rev"];
    require(has_exactly(value, &fields), FORMAT)?;
    let puzzle = match value.get("puzzle").and_then(Json::as_text) {
        Some("full") => Puzzle::Full,
        Some("jumbling") => Puzzle::Jumbling,
        _ => return Err(Refusal::new(FORMAT)),
    };
    let optional = |name: &str| match value.get(name) {
        Some(Json::Null) => Ok(None),
        Some(Json::Text(text)) => Ok(Some(text.clone())),
        _ => Err(Refusal::new(FORMAT)),
    };
    let block = IdentityBlock {
        puzzle,
        model_id: text(value, "model_id")?,
        menu_id: optional("menu_id")?,
        menu_order: optional("menu_order")?,
        contract_rev: optional("contract_rev")?,
    };
    let menu = [block.menu_id.is_some(), block.menu_order.is_some()];
    let complete = match puzzle {
        Puzzle::Full => menu == [false, false] && block.contract_rev.is_none(),
        Puzzle::Jumbling => menu[0] == menu[1] && block.contract_rev.is_some(),
    };
    require(complete, FORMAT)?;
    Ok(block)
}

fn read_pin(value: Option<&Json>) -> Result<Pin, Refusal> {
    let value = value.ok_or(Refusal::new(FORMAT))?;
    require(has_exactly(value, &["version", "source"]), FORMAT)?;
    let hex = sha(value.get("source"), "source")?;
    let mut source = [0u8; 32];
    for (i, byte) in source.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&hex[2 * i..2 * i + 2], 16).expect("checked hex");
    }
    Ok(Pin { version: text(value, "version")?, source })
}

/// FP.8's DigestRef of a SHA-256 hex.
fn sha_ref(role: &str, sha256: &str) -> Json {
    Json::object([("role", Json::text(role)), ("sha256", Json::text(sha256))])
}

/// The SHA-256 hex of a DigestRef with exactly this role.
fn sha(value: Option<&Json>, role: &str) -> Result<String, Refusal> {
    let value = value.ok_or(Refusal::new(FORMAT))?;
    require(has_exactly(value, &["role", "sha256"]), FORMAT)?;
    require(value.get("role").and_then(Json::as_text) == Some(role), FORMAT)?;
    let hex = sha_of(value).unwrap_or("");
    require(hex.len() == 64 && hex.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')), FORMAT)?;
    Ok(hex.to_owned())
}

fn text(value: &Json, name: &str) -> Result<String, Refusal> {
    value.get(name).and_then(Json::as_text).map(str::to_owned).ok_or(Refusal::new(FORMAT))
}

fn list<'a>(value: &'a Json, name: &str) -> Result<&'a [Json], Refusal> {
    match value.get(name) {
        Some(Json::List(items)) => Ok(items),
        _ => Err(Refusal::new(FORMAT)),
    }
}

/// Whether two lists hold the same items, by their FP.5 bytes, in any order.
fn same_items<T>(a: &[T], b: &[T], json: impl Fn(&T) -> Json) -> bool {
    let set = |items: &[T]| items.iter().map(|item| canonical(&json(item))).collect::<BTreeSet<_>>();
    set(a) == set(b)
}

/// A list sorted by the items' FP.5 bytes.
fn sorted(items: impl Iterator<Item = Json>) -> Json {
    let mut items: Vec<Json> = items.collect();
    items.sort_by_cached_key(canonical);
    Json::List(items)
}

fn encoded(value: &Value) -> Vec<u8> {
    let mut bytes = Vec::new();
    encode(value, &mut bytes).expect("a Bool has a canonical form");
    bytes
}

//! A check job's run record (PS1.31) with chapter 09's `ReproKey` (DET.1), its exact payload (DET.6) and its
//! reproduction (DET.7), on `c600-session`'s draft `c600-run-draft` record (`persist::run`).
//!
//! # The key
//!
//! The key names everything a check job's result depends on, and nothing of the machine it ran on (DET.1 to DET.4):
//!
//! - `source` and `modules`: the job adapter's source listing (module `research.check_job`, entry `check`), and each
//!   checker the job ran, by its ID and the SHA-256 of its source listing (KN6.3), so the executed closure is pinned;
//! - `edition` and `ir_version`: the session's language edition, and `none`, since a check job runs no IR;
//! - `identity` and `input`: the KN0.1 identity block and the `Digest<level>` of the input the job is pinned at;
//! - `parameters`: one argument per check (its kind, the digest its certificate is stated to have, its input roles and
//!   its requested caps), every certificate and input captured by digest under a logical name (DET.41), no grant, the
//!   research profile and the draft policy identity;
//! - `budgets`: the job's deterministic limits and the metering rule they are counted in ([`METERING_RULE`]);
//! - `kernel_cost_table`: the checkers' cost rule (`c600_checkers::claims::COST_RULE`); `kernel_api`: the kernel's
//!   build string; `checkers`: the identities of the checkers the result relies on (DET.3).
//!
//! # The exact payload
//!
//! It has the fields of `persist::run::exact`: the outcome, its code and deterministic stop position, the used counters,
//! the child ledger (one entry per check), the value's digest and the digests of the reports published before any stop.
//! A report is published as FP.5 bytes ([`REPORT_FORMAT`]) with its verdict, checker ID, kind, certificate digest,
//! bound inputs and the checker's own trace; the checker's source digest is the key's, not the payload's, so the payload
//! changes when a check's result does and the key when its implementation does. Wall-clock time, machine, user, paths
//! and locale appear in neither.

use std::collections::BTreeMap;

use c600_checkers::kinds::Checker;
use c600_checkers::{checker, CheckReport, CheckerKind, Engines, Refusal, Verdict};
use c600_digest::sha256_hex;
use c600_session::persist::codec::{canonical, nat, nat_text, Json};
use c600_session::persist::codes::FORMAT;
use c600_session::persist::record::identity_json;
use c600_session::persist::run::{self, policy_identity, Admission, Inputs, Key, Record, Reproduction};

use crate::job::{Budget, Cancel, Check, CheckJob, CheckRun, Granted, Pinned};

/// The job adapter's module name and entry in the key and the provenance block.
pub const ADAPTER_MODULE: &str = "research.check_job";
pub const ENTRY: &str = "check";
/// The session's language edition. A check job has no source of its own in the language.
pub const EDITION: &str = "6cs-draft-1";
/// A check job runs no typed IR.
pub const IR_VERSION: &str = "none";
/// The format of a published check report.
pub const REPORT_FORMAT: &str = "c600-check-report-draft-1";
/// The draft metering rule of a check job, pinned in its key (DET.29).
pub const METERING_VERSION: &str = "c600-research-check-metering-draft-1";
pub const METERING_RULE: &str =
    "c600-research-check-metering-draft-1: steps, one per check admitted; kernel_cost, the \
units its checker charges under kn6-check-cost-draft-1; memory, the bytes of the certificate and inputs a check holds \
while it runs, the job's peak; output, the bytes of each published report and of the job's value";
/// The version under which the kernel's build string is pinned as the key's `kernel_api`.
pub const KERNEL_API_VERSION: &str = "c600-kernel-build";
/// The version under which the checkers' cost rule is pinned as the key's `kernel_cost_table`.
pub const COST_TABLE_VERSION: &str = "kn6-check-cost-draft-1";

/// The job adapter's sources, hashed into its module digest: the files that run a job and write its record. The
/// evidence and archive modules only read runs and imports after the fact and write no payload, so they are not part
/// of the executed closure. The bytes are hashed as checked out, which the repository keeps byte for byte (no line-ending
/// conversion).
const ADAPTER: [(&str, &[u8]); 3] = [
    ("c600-research/Cargo.toml", include_bytes!("../Cargo.toml")),
    ("c600-research/src/job.rs", include_bytes!("job.rs")),
    ("c600-research/src/record.rs", include_bytes!("record.rs")),
];

/// A `sha256sum`-style listing (`<sha256>  <name>\n` per file, in the given order), as KN6.3 identifies a checker.
fn listing(files: &[(&str, &[u8])]) -> Vec<u8> {
    let mut out = String::new();
    for (name, bytes) in files {
        out.push_str(&format!("{}  {name}\n", sha256_hex(bytes)));
    }
    out.into_bytes()
}

/// The job adapter's source listing: the bytes its module digest names.
pub fn adapter_listing() -> Vec<u8> {
    listing(&ADAPTER)
}

/// The job adapter's module digest.
pub fn adapter_digest() -> String {
    sha256_hex(&adapter_listing())
}

/// A checker's source listing: the bytes its source digest names (KN6.3).
pub fn checker_listing(checker: &Checker) -> Vec<u8> {
    listing(checker.sources.files)
}

/// The sources a host holds for reproduction, by SHA-256: the adapter's listing and the listing of every checker it can
/// run (DET.7 fetches the key's sources and modules from these and never substitutes another).
pub fn sources() -> BTreeMap<String, Vec<u8>> {
    let mut out = BTreeMap::new();
    let mut add = |bytes: Vec<u8>| {
        out.insert(sha256_hex(&bytes), bytes);
    };
    add(adapter_listing());
    for kind in [CheckerKind::Admissibility, CheckerKind::Macro, CheckerKind::Expansion, CheckerKind::Checkpoints] {
        if let Some(checker) = checker(kind) {
            add(checker_listing(checker));
        }
    }
    for sub in c600_checkers::archive::registry() {
        add(checker_listing(&sub.checker));
    }
    out
}

/// A kind as the key's argument names it: a token and, for the research-archive kind, its sub-checker's ID.
pub fn kind_token(kind: CheckerKind) -> (&'static str, Option<&'static str>) {
    match kind {
        CheckerKind::Admissibility => ("admissibility", None),
        CheckerKind::Macro => ("macro", None),
        CheckerKind::Expansion => ("expansion", None),
        CheckerKind::Checkpoints => ("checkpoints", None),
        CheckerKind::Archive(id) => ("archive", Some(id)),
    }
}

/// The kind a key's argument names, if this build has it.
pub fn kind_from(token: &str, archive: Option<&str>) -> Option<CheckerKind> {
    match (token, archive) {
        ("admissibility", None) => Some(CheckerKind::Admissibility),
        ("macro", None) => Some(CheckerKind::Macro),
        ("expansion", None) => Some(CheckerKind::Expansion),
        ("checkpoints", None) => Some(CheckerKind::Checkpoints),
        ("archive", Some(id)) => c600_checkers::archive::find(id).map(|sub| CheckerKind::Archive(sub.id())),
        _ => None,
    }
}

fn sha_ref(role: &str, sha256: &str) -> Json {
    Json::object([("role", Json::text(role)), ("sha256", Json::text(sha256))])
}

fn pin(version: &str, sha256: &str) -> Json {
    Json::object([("version", Json::text(version)), ("source", sha_ref("source", sha256))])
}

/// FP.5's value of a budget's four counters.
pub(crate) fn amounts(b: &Budget) -> Json {
    Json::object([
        ("steps", nat_text(b.steps)),
        ("kernel_cost", nat_text(b.kernel_cost)),
        ("memory", nat_text(b.memory)),
        ("output", nat_text(b.output)),
    ])
}

fn read_amounts(value: &Json) -> Option<Budget> {
    let field = |name: &str| value.get(name).and_then(|v| nat(v).ok());
    Some(Budget {
        steps: field("steps")?,
        kernel_cost: field("kernel_cost")?,
        memory: field("memory")?,
        output: field("output")?,
    })
}

/// A list sorted by its items' FP.5 bytes: a set, as the key writes one.
fn sorted(mut items: Vec<Json>) -> Json {
    items.sort_by_cached_key(canonical);
    items.dedup();
    Json::List(items)
}

/// The capture name of a check's certificate and of one of its inputs (DET.40: a logical name, never a path).
pub(crate) fn certificate_name(index: usize) -> String {
    format!("check[{index}]/certificate")
}

pub(crate) fn input_name(index: usize, role: &str) -> String {
    format!("check[{index}]/input/{role}")
}

/// The pins of the checkers a job runs: the key's `checkers` (DET.3).
fn checker_pins(kinds: &[CheckerKind]) -> Vec<Json> {
    kinds
        .iter()
        .filter_map(|kind| checker(*kind))
        .map(|checker| {
            let id = checker.sources.identity();
            pin(id.id, &id.source)
        })
        .collect()
}

/// The pins of the cost rule, the metering rule and the kernel's build.
fn cost_pin() -> Json {
    pin(COST_TABLE_VERSION, &sha256_hex(c600_checkers::claims::COST_RULE.as_bytes()))
}

fn metering_pin() -> Json {
    pin(METERING_VERSION, &sha256_hex(METERING_RULE.as_bytes()))
}

fn kernel_api_pin() -> Json {
    pin(KERNEL_API_VERSION, &sha256_hex(c600_checkers::engines::kernel_build().as_bytes()))
}

fn refusal(code: &'static str, message: impl Into<String>) -> Refusal {
    Refusal { code, message: message.into() }
}

fn persist(error: c600_session::persist::Refusal) -> Refusal {
    refusal(error.code, error.to_string())
}

/// Seals a job's key before it runs: every certificate and input is captured by digest first (DET.41). A kind this build
/// has no checker for is refused with `KN6:evidence.no_checker`; a key that is not well formed (an input level other
/// than `labels` or `poses`, an identity block that is not one) with `PS1:input.persistence_format`.
pub(crate) fn seal(job: &CheckJob) -> Result<Key, Refusal> {
    let mut args = Vec::new();
    let mut captured = Vec::new();
    let mut kinds = Vec::new();
    let mut modules =
        vec![Json::object([("name", Json::text(ADAPTER_MODULE)), ("source", sha_ref("source", &adapter_digest()))])];
    let capture = |name: String, bytes: &[u8]| {
        Json::object([
            ("kind", Json::text("file")),
            ("name", Json::text(&name)),
            ("bytes", sha_ref("bytes", &sha256_hex(bytes))),
            ("size", nat_text(bytes.len() as u128)),
        ])
    };
    for (index, check) in job.checks.iter().enumerate() {
        let found = checker(check.kind).ok_or_else(|| {
            refusal(c600_checkers::codes::NO_CHECKER, format!("no checker for check[{index}] ({})", check.kind.name()))
        })?;
        let (token, archive) = kind_token(check.kind);
        args.push(Json::object([
            ("kind", Json::text(token)),
            ("archive", archive.map_or(Json::Null, Json::text)),
            ("certificate", Json::object([("stated", Json::text(&check.stated))])),
            ("inputs", Json::List(check.inputs.iter().map(|input| Json::text(&input.role)).collect())),
            ("cap", check.cap.as_ref().map_or(Json::Null, amounts)),
        ]));
        captured.push(capture(certificate_name(index), &check.certificate));
        for input in &check.inputs {
            captured.push(capture(input_name(index, &input.role), &input.bytes));
        }
        if !kinds.contains(&check.kind) {
            kinds.push(check.kind);
            let id = found.sources.identity();
            modules.push(Json::object([("name", Json::text(id.id)), ("source", sha_ref("source", &id.source))]));
        }
    }
    let parameters = Json::object([
        ("entry", Json::object([("module", Json::text(ADAPTER_MODULE)), ("function", Json::text(ENTRY))])),
        ("args", Json::List(args)),
        ("captured", sorted(captured)),
        ("grants", Json::List(Vec::new())),
        ("profile", Json::text("research")),
        ("policy", policy_identity(&[]).json()),
    ]);
    let value = Json::object([
        ("source", Json::List(vec![sha_ref("source", &adapter_digest())])),
        ("modules", Json::List(modules)),
        ("edition", Json::text(EDITION)),
        ("ir_version", Json::text(IR_VERSION)),
        ("identity", job.input.identity.clone()),
        ("input", sha_ref(&job.input.level, &job.input.digest)),
        ("parameters", parameters),
        ("seeds", Json::List(Vec::new())),
        ("budgets", Json::object([("limits", amounts(&job.budget)), ("metering", metering_pin())])),
        ("kernel_cost_table", cost_pin()),
        ("kernel_api", kernel_api_pin()),
        ("checkers", sorted(checker_pins(&kinds))),
    ]);
    Key::read(&value).map_err(persist)
}

/// FP.17's block of a check job's record: a built-in program, the job adapter, in the research profile, reading no
/// session state (the job reads its captured inputs only), under the tools' draft capability contract.
pub(crate) fn provenance() -> Json {
    let block = Json::object([
        ("version", Json::Int(1)),
        ("form", Json::text("capability_separation")),
        ("contract", c600_tools::tool::contract().json()),
        ("checker", Json::Null),
    ]);
    Json::object([
        ("block", block),
        ("author", Json::text("built_in")),
        (
            "producer",
            Json::object([
                ("kind", Json::text("program")),
                ("module", sha_ref("source", &adapter_digest())),
                ("entry", Json::text(ENTRY)),
            ]),
        ),
        ("profile", Json::text("research")),
        ("depends", Json::object([("read_state", Json::text("no"))])),
        ("parts", Json::List(Vec::new())),
        ("pick", Json::Null),
        ("declassified", Json::List(Vec::new())),
        ("legacy", Json::Null),
    ])
}

/// A published report's FP.5 value: the checker's ID (its source digest is the key's), the kind, the certificate's
/// digest, the verdict, the bound inputs and the checker's own trace, in its order.
pub fn report_json(report: &CheckReport) -> Json {
    let (token, _) = kind_token(report.kind);
    let verdict = match &report.verdict {
        Verdict::Pass => Json::object([("kind", Json::text("pass"))]),
        Verdict::Fail { claim, code, reason } => Json::object([
            ("kind", Json::text("fail")),
            ("claim", Json::text(claim)),
            ("code", Json::text(code)),
            ("reason", Json::text(reason)),
        ]),
        Verdict::NotApplicable { reason } => {
            Json::object([("kind", Json::text("not_applicable")), ("reason", Json::text(reason))])
        }
    };
    let inputs = report
        .inputs
        .iter()
        .map(|input| {
            let identity = input
                .identity
                .iter()
                .map(|(name, value)| Json::List(vec![Json::text(name), Json::text(value)]))
                .collect();
            Json::object([
                ("role", Json::text(&input.role)),
                ("sha256", Json::text(&input.sha256)),
                ("identity", Json::List(identity)),
            ])
        })
        .collect();
    let trace = report
        .trace
        .iter()
        .map(|step| {
            let span = step.span.as_ref().map_or(Json::Null, |span| {
                Json::object([
                    ("word", Json::text(&span.word)),
                    ("from", nat_text(u128::from(span.from))),
                    ("to", nat_text(u128::from(span.to))),
                ])
            });
            Json::object([
                ("claim", Json::text(&step.claim)),
                ("held", Json::Bool(step.held)),
                ("computed", Json::text(&step.computed)),
                ("span", span),
            ])
        })
        .collect();
    Json::object([
        ("format", Json::text(REPORT_FORMAT)),
        ("checker", Json::text(report.checker.id)),
        ("kind", Json::text(token)),
        ("certificate", Json::text(&report.input)),
        ("verdict", verdict),
        ("inputs", Json::List(inputs)),
        ("trace", Json::List(trace)),
    ])
}

/// A published report's bytes.
pub fn report_bytes(report: &CheckReport) -> Vec<u8> {
    canonical(&report_json(report))
}

/// The record of a sealed key and an exact payload, written and read back (`Record::write`).
pub(crate) fn write(key: &Key, payload: Json) -> Result<(Record, Vec<u8>), Refusal> {
    let record = Record {
        key: key.clone(),
        result: payload,
        safety_stop: None,
        continues: None,
        checks: None,
        provenance: provenance(),
    };
    let bytes = record.write().map_err(persist)?;
    Ok((record, bytes))
}

/// DET.7's answer for a stored check job record, with the re-run when its exact payload equals the record's byte for
/// byte: only then is the re-run's report the kernel checker's own again, and only through it can a stored check reach
/// evidence.
#[derive(Debug)]
pub struct Reproduced {
    pub answer: Reproduction,
    pub run: Option<CheckRun>,
}

/// The admission a host runs a recorded key under now (DET.7): the recorded parameters and input, with this build's
/// edition, IR version, model, metering, cost rule, kernel, checker identities, profile, policy and grants. A kind this
/// build lacks drops out of the checker set, so the set differs and reproduction refuses.
pub fn admission_now(recorded: &Key, engines: &Engines) -> Result<Admission, Refusal> {
    let mut value = recorded.json();
    let Json::Object(map) = &mut value else {
        return Err(refusal(FORMAT, "a key is an object"));
    };
    let mut identity = recorded.identity.clone();
    if let Some(kernel) = engines.kernel {
        identity.model_id = kernel.identity().model_id;
    }
    let kinds: Vec<CheckerKind> = recorded
        .parameters
        .args
        .iter()
        .filter_map(|arg| {
            let token = arg.get("kind").and_then(Json::as_text)?;
            kind_from(token, arg.get("archive").and_then(Json::as_text))
        })
        .collect();
    map.insert("edition".into(), Json::text(EDITION));
    map.insert("ir_version".into(), Json::text(IR_VERSION));
    map.insert("identity".into(), identity_json(&identity));
    map.insert("budgets".into(), Json::object([("limits", amounts(&limits(recorded))), ("metering", metering_pin())]));
    map.insert("kernel_cost_table".into(), cost_pin());
    map.insert("kernel_api".into(), kernel_api_pin());
    map.insert("checkers".into(), sorted(checker_pins(&kinds)));
    if let Some(Json::Object(parameters)) = map.get_mut("parameters") {
        parameters.insert("grants".into(), Json::List(Vec::new()));
        parameters.insert("profile".into(), Json::text("research"));
        parameters.insert("policy".into(), policy_identity(&[]).json());
    }
    Ok(Admission::of(&Key::read(&value).map_err(persist)?))
}

/// The job's limits as the key holds them.
fn limits(key: &Key) -> Budget {
    let l = &key.limits;
    Budget { steps: l.steps, kernel_cost: l.kernel_cost, memory: l.memory, output: l.output }
}

/// The job a key names, from its arguments and the captured bytes reproduction fetched by digest.
fn rebuild(key: &Key, inputs: &Inputs) -> Result<CheckJob, Reproduction> {
    let malformed = || Reproduction::Refused(FORMAT.into());
    let fetch = |name: &str| inputs.captured.get(&("file", name)).map(|bytes| bytes.to_vec()).ok_or_else(malformed);
    let mut checks = Vec::new();
    for (index, arg) in key.parameters.args.iter().enumerate() {
        let token = arg.get("kind").and_then(Json::as_text).ok_or_else(malformed)?;
        let archive = arg.get("archive").and_then(Json::as_text);
        let kind = kind_from(token, archive).ok_or_else(|| Reproduction::Refused(run::codes::KEY_MISMATCH.into()))?;
        let stated =
            arg.get("certificate").and_then(|c| c.get("stated")).and_then(Json::as_text).ok_or_else(malformed)?;
        let roles = match arg.get("inputs") {
            Some(Json::List(roles)) => roles.iter().map(|r| r.as_text().map(str::to_owned)).collect::<Option<Vec<_>>>(),
            _ => None,
        }
        .ok_or_else(malformed)?;
        let cap = match arg.get("cap") {
            Some(Json::Null) => None,
            Some(cap) => Some(read_amounts(cap).ok_or_else(malformed)?),
            None => return Err(malformed()),
        };
        let mut granted = Vec::new();
        for role in roles {
            let bytes = fetch(&input_name(index, &role))?;
            granted.push(Granted { role, bytes });
        }
        checks.push(Check {
            kind,
            certificate: fetch(&certificate_name(index))?,
            stated: stated.to_owned(),
            inputs: granted,
            cap,
        });
    }
    let input = Pinned {
        level: key.level.clone(),
        digest: key.input.clone(),
        identity: identity_json(&key.identity),
        revision: None,
    };
    Ok(CheckJob { checks, input, budget: limits(key) })
}

/// DET.7's `reproduce` of a stored check job record: the record is read and admitted against [`admission_now`] and
/// the bytes in `available` (by SHA-256: the sources of [`sources`] and the captured inputs) before anything runs; then
/// the job is run again on `engines` from the captured bytes alone and its exact payload compared with the record's,
/// byte for byte. A cancelled, failed or clock-stopped record answers `not run`; another implementation, model,
/// profile, policy or grant set a refusal; missing bytes `not run`; a key this build does not seal again from the
/// recorded job (another adapter source, module closure or argument) `LG4:input.repro_key_mismatch`; a re-run that
/// `cancel` stops `not run` with `KN7:cancelled.by_user`; a different payload `LG4:evidence.reproducibility_mismatch`. A
/// deterministic cut-off reproduces like a completed run (DET.5).
pub fn reproduce(
    engines: &Engines,
    bytes: &[u8],
    available: &BTreeMap<String, Vec<u8>>,
    cancel: &Cancel,
) -> Reproduced {
    let now = match Record::read(bytes) {
        Ok(record) => match admission_now(&record.key, engines) {
            Ok(now) => now,
            Err(error) => return Reproduced { answer: Reproduction::Refused(error.code.into()), run: None },
        },
        Err(error) => return Reproduced { answer: Reproduction::Refused(error.code.into()), run: None },
    };
    let mut rerun: Option<CheckRun> = None;
    let answer = run::reproduce(bytes, available, &now, |key, inputs| {
        let job = rebuild(key, inputs)?;
        let admitted = job.admit().map_err(|error| Reproduction::Refused(error.code.into()))?;
        // This build seals the recorded job to the recorded key, or it would run another implementation under that
        // key: the adapter's source and module digests are checked here, since the admission holds neither (DET.7).
        if admitted.key() != key {
            return Err(Reproduction::Refused(run::codes::KEY_MISMATCH.into()));
        }
        let ran = admitted.run(engines, cancel).map_err(|error| Reproduction::Refused(error.code.into()))?;
        // A record that reaches here was not cancelled, so a cancelled re-run is the caller's stop, not a mismatch.
        if ran.code() == Some(crate::job::codes::CANCELLED) {
            return Err(Reproduction::NotRun(crate::job::codes::CANCELLED.into()));
        }
        let payload = ran.payload().clone();
        rerun = Some(ran);
        Ok(payload)
    });
    let run = match answer {
        Reproduction::Same(_) => rerun,
        _ => None,
    };
    Reproduced { answer, run }
}

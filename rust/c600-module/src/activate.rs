//! Version separation and module activation (chapter 04 EV1.7, EV1.8 and EV1.11, chapter 02 LG5.6; LP4-2).
//!
//! [`Modules`] is a host's set of user modules for one session identity. It holds, for each module name, at most one
//! active version and the latest checked candidate, each a verified closure with its approvals.
//!
//! - [`Modules::check`] verifies a draft's closure and approves its entries. Success creates the module's candidate
//!   and returns its digest and check reference; it never changes the active version (EV1.11). A refused check, such as
//!   a broken draft, stores nothing, so the last valid version keeps running (EV1.7, LG5.6).
//! - [`Modules::activate_module`] is the only switch (EV1.11). Under one lock it checks that the reference covers
//!   exactly the candidate, that the host's activation grants still hold and that the active digest is the expected
//!   one, and only then installs the whole candidate closure. Every refusal leaves the active version as it was.
//! - [`Modules::bind`] binds an invocation wholly to the active version, under the same lock: the [`Binding`] holds that
//!   version's bytes, approvals and loaded program for the whole run, and nothing it holds changes when another version
//!   activates (EV1.11). [`Modules::bind_pinned`] is the route that names a digest.
//! - A [`Binding`] runs its entry in this process ([`Binding::run`]) or through another executor of the same closure
//!   ([`Binding::run_with`]), such as the runtime's own process. Its results, captures and source ranges name the
//!   version it bound (EV1.7). It keeps the version's immutable bytes while it lives, whatever the set does.
//!
//! # Drafts
//!
//! Each choice freezes with the module formats or the editor protocol (LG5.10, LQ10); none of them is a frozen format.
//!
//! - **Candidate digest** (EV1.11). The candidate's digest is its closure digest ([`crate::Closure::digest`]): the
//!   root manifest, every module's source digest with its edition and IR contract, and the resolver contract.
//! - **Check reference** (EV1.11). [`CheckReference`] is the SHA-256 of the check record `c600-check-reference-draft`
//!   version 1, which has no FP.4 row and is never stored: `{format, version, identity, payload, provenance}` with
//!   payload `{approvals, checker, closure, contract, ir, manifest, module, modules}`. `approvals` is the sorted list of
//!   the approval records' digests (`c600-approval-draft`, one per checked entry, each naming its author, profile, grant
//!   and policy); `checker` and `contract` are the pins the check ran under; `closure` and `manifest` are the closure
//!   and manifest digests; `ir` is the IR version and execution contract; `module` is the root module's name; and
//!   `modules` is the set of `{edition, module, source}`, sorted by source digest. So the reference covers exactly the
//!   candidate's source, manifest, lock closure, editions, execution contract and checker version, and a check under
//!   other entries, grants or pins is another reference.
//! - **Entries** (LG3.9, BND.17). A check approves one or more entries of the root module, each under its own author
//!   class, profile, grant and policy ([`crate::Entry`]); an entry of another module is refused with
//!   `LG5:input.module_contract_mismatch`. An invocation names the module and one of the checked entries.
//! - **Activation grants** (EV1.8, EV1.11; chapter 09's profile grants). The set holds the host's current grant
//!   ([`Modules::set_grant`]). Activation requires every checked entry's grant to lie within it, and each invocation
//!   requires its own entry's grant to lie within it at the bind; otherwise `LG3:capability.not_granted`. A revoked
//!   grant never touches a binding already made: stopping a run under KN7 is the run's host's (EV1.7).
//! - **One candidate per module.** A new successful check of a module replaces its candidate, so the set stays bounded
//!   while a draft is edited and checked again. Activating a replaced candidate is refused with
//!   `EV1:input.module_not_checked`: its check is no longer held. The digests of every successful check are kept (32
//!   bytes each) for [`Modules::bind_pinned`].
//! - **Invocations without a checked version.** A module with no active version, or an entry its active version's
//!   check did not cover, is refused with `EV1:input.module_not_checked`: no successful check covers what the
//!   invocation would run (EV1.7: an unchecked draft never runs).
//! - **A route that names a digest** (EV1.7: a view, a recording or a debugger). [`Modules::bind_pinned`] binds only
//!   when the digest is the active version's closure digest or its root source digest. A digest no successful check
//!   produced is `EV1:input.module_not_checked`; a checked digest that is not active is `EV1:stale.activation_base`,
//!   whose continuation `after_refresh` is the route's: it refreshes to the active version or asks for activation.
//!   Chapter 13 names `activate_module` as both codes' origin; this reading of them on a route is the integrator's to
//!   confirm.
//! - **Retention** (EV1.11). A [`Binding`] holds its version through an `Arc`, so the version's bytes, approvals and
//!   loaded program stay while any run can resume, also after another version activates or the set is dropped.
//! - **Source ranges** (EV1.7, LG4.12). [`Binding::run`] gives the interpreter the bound version's source map, whatever
//!   the request's options say, so a stop's source range names the bytes that ran.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use c600_check::Grant;
use c600_digest::sha256;
use c600_ir::{EXEC_CONTRACT, IR_VERSION};
use c600_run::boundary::{Pins, Store};
use c600_run::interp::{Host, Run, SourceRanges};
use c600_run::meter::SourceRange;
use c600_run::provenance::{self, digest_ref, hex, Json, Pin};
use c600_syntax::Lowered;

use crate::code::{CONTRACT_MISMATCH, IR_REJECTED, MODULE_NOT_CHECKED, NOT_GRANTED, STALE_ACTIVATION_BASE};
use crate::{
    approve, load, refuse, verify, Approval, Closure, Entry, Files, Halted, Identity, Outcome, Ready, Refusal,
    RunContext, RunRequest, FORM_VERSION,
};

/// The check record's form; FP.4 has no row for it (draft).
pub const CHECK_REFERENCE_FORMAT: &str = "c600-check-reference-draft";

/// A reference to one successful check: the SHA-256 of its check record (EV1.11; draft form
/// [`CHECK_REFERENCE_FORMAT`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CheckReference(pub [u8; 32]);

/// What a successful check created: the module's candidate, its digest and its check reference (EV1.11).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Candidate {
    /// The root module's name: the module the candidate may become the active version of.
    pub module: String,
    /// The candidate's closure digest.
    pub digest: [u8; 32],
    /// The root module's source digest.
    pub source: [u8; 32],
    pub reference: CheckReference,
    /// The check record's canonical bytes, whose SHA-256 is [`Candidate::reference`].
    pub record: Vec<u8>,
}

/// A successful activation: the previous active digest, none for an initial install, and the new one (EV1.11).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Activation {
    pub module: String,
    pub previous: Option<[u8; 32]>,
    pub new: [u8; 32],
}

/// One checked version: its verified closure, the source map of its lowering, one loaded and approved entry per checked
/// function, and its check reference.
struct Version {
    module: String,
    closure: Closure,
    lowered: Lowered,
    entries: BTreeMap<String, Ready>,
    reference: CheckReference,
    record: Vec<u8>,
}

impl Version {
    fn source(&self) -> [u8; 32] {
        self.closure.manifest.source
    }

    fn candidate(&self) -> Candidate {
        Candidate {
            module: self.module.clone(),
            digest: self.closure.digest,
            source: self.source(),
            reference: self.reference,
            record: self.record.clone(),
        }
    }

    /// Whether `digest` names this version: its closure digest or its root source digest.
    fn named_by(&self, digest: &[u8; 32]) -> bool {
        self.closure.digest == *digest || self.source() == *digest
    }
}

impl SourceRanges for Version {
    fn range(&self, module: &str, function: &str, path: &str) -> Option<SourceRange> {
        let map = self.lowered.source(module)?;
        let span = map.span(function, path)?;
        Some(SourceRange { digest: map.digest.clone(), start: span.start, end: span.end })
    }
}

#[derive(Default)]
struct State {
    grant: Grant,
    candidates: BTreeMap<String, Arc<Version>>,
    active: BTreeMap<String, Arc<Version>>,
    /// The closure and root source digests of every successful check.
    checked: BTreeSet<[u8; 32]>,
}

/// A host's user modules for one session identity: per module name, the active version and the latest candidate.
pub struct Modules {
    identity: Identity,
    contract: Pin,
    checker: Pin,
    state: Mutex<State>,
}

impl Modules {
    /// An empty set for the session identity `identity`, whose checks run under `contract` and `checker` and whose
    /// host grants `grant`.
    pub fn new(identity: Identity, contract: Pin, checker: Pin, grant: Grant) -> Modules {
        Modules { identity, contract, checker, state: Mutex::new(State { grant, ..State::default() }) }
    }

    fn state(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// The pins the policy in force holds for this set's runs: its contract and checker pins.
    pub fn pins(&self) -> Pins {
        Pins { contracts: vec![self.contract.clone()], checkers: vec![self.checker.clone()] }
    }

    /// The host's current grant.
    pub fn grant(&self) -> Grant {
        self.state().grant.clone()
    }

    /// Changes the host's grant. Later activations and binds check against it; a binding already made keeps running.
    pub fn set_grant(&self, grant: Grant) {
        self.state().grant = grant;
    }

    /// The active version's closure digest of `module`, if one is active.
    pub fn active(&self, module: &str) -> Option<[u8; 32]> {
        self.state().active.get(module).map(|v| v.closure.digest)
    }

    /// The candidate of `module`: the latest successful check, which may be the active version.
    pub fn candidate(&self, module: &str) -> Option<Candidate> {
        self.state().candidates.get(module).map(|v| v.candidate())
    }

    /// Checks a draft: verifies the manifest, its lock and the local bytes for the set's identity, approves each entry
    /// and creates the module's candidate (EV1.11). Nothing is active after a check that was not active before it; a
    /// refusal stores nothing (EV1.7). The entries must be functions of the root module.
    pub fn check(&self, manifest: &[u8], lock: &[u8], files: &Files, entries: &[Entry]) -> Result<Candidate, Refusal> {
        let closure = verify(manifest, lock, files, &self.identity)?;
        let module = closure.manifest.module.clone();
        if entries.is_empty() {
            return Err(refuse(CONTRACT_MISMATCH, format!("a check of `{module}` names no entry (EV1.11; draft)")));
        }
        let mut approved = BTreeMap::new();
        for entry in entries {
            if entry.module != module {
                let detail = format!(
                    "the entry `{}.{}` is not a function of the root module `{module}` (draft)",
                    entry.module, entry.function
                );
                return Err(refuse(CONTRACT_MISMATCH, detail));
            }
            let approval = approve(&closure, entry.clone(), self.contract.clone(), self.checker.clone())?;
            let ready = load(&closure, &approval)?;
            if approved.insert(entry.function.clone(), ready).is_some() {
                let detail = format!("the check names the entry `{module}.{}` twice (draft)", entry.function);
                return Err(refuse(CONTRACT_MISMATCH, detail));
            }
        }
        let lowered = relower(&closure)?;
        let record = record(&closure, &approved, &self.identity, &self.contract, &self.checker);
        let version = Arc::new(Version {
            module: module.clone(),
            reference: CheckReference(sha256(&record)),
            record,
            closure,
            lowered,
            entries: approved,
        });
        let candidate = version.candidate();
        let mut state = self.state();
        state.checked.insert(candidate.digest);
        state.checked.insert(candidate.source);
        state.candidates.insert(module, version);
        Ok(candidate)
    }

    /// The only switch (EV1.11): makes the candidate `candidate_digest` of `module` the active version when
    /// `check_reference` is the reference of its successful check, every checked entry's grant lies within the host's
    /// grant, and the active digest is `expected_active_digest` (none for an initial install). An unchecked or
    /// mismatched candidate is `EV1:input.module_not_checked`, a grant the host no longer holds
    /// `LG3:capability.not_granted`, and another active digest `EV1:stale.activation_base`. The checks and the install
    /// happen under one lock, so a refusal leaves the active version intact and an invocation binds wholly before or
    /// after the switch.
    pub fn activate_module(
        &self,
        module: &str,
        expected_active_digest: Option<[u8; 32]>,
        candidate_digest: [u8; 32],
        check_reference: CheckReference,
    ) -> Result<Activation, Refusal> {
        let mut state = self.state();
        let candidate = match state.candidates.get(module) {
            Some(v) if v.closure.digest == candidate_digest => Arc::clone(v),
            _ => {
                let detail = format!(
                    "no successful check of `{module}` covers the candidate {} (EV1.11)",
                    hex(&candidate_digest)
                );
                return Err(refuse(MODULE_NOT_CHECKED, detail));
            }
        };
        if candidate.reference != check_reference {
            let detail = format!(
                "the check reference {} does not cover the candidate {} of `{module}`, whose check is {} (EV1.11)",
                hex(&check_reference.0),
                hex(&candidate_digest),
                hex(&candidate.reference.0)
            );
            return Err(refuse(MODULE_NOT_CHECKED, detail));
        }
        for ready in candidate.entries.values() {
            ungranted(&ready.approval().entry, &state.grant)?;
        }
        let previous = state.active.get(module).map(|v| v.closure.digest);
        if previous != expected_active_digest {
            let name = |d: Option<[u8; 32]>| d.map_or_else(|| "none".to_owned(), |d| hex(&d));
            let detail = format!(
                "the active version of `{module}` is {}, not the expected {} (EV1.11)",
                name(previous),
                name(expected_active_digest)
            );
            return Err(refuse(STALE_ACTIVATION_BASE, detail));
        }
        state.active.insert(module.to_owned(), candidate);
        Ok(Activation { module: module.to_owned(), previous, new: candidate_digest })
    }

    /// Binds an invocation of `module.function` wholly to the active version (EV1.11). A module with no active version
    /// or an entry its check did not cover is `EV1:input.module_not_checked`; an entry whose grant the host no longer
    /// holds is `LG3:capability.not_granted`.
    pub fn bind(&self, module: &str, function: &str) -> Result<Binding, Refusal> {
        let state = self.state();
        let Some(version) = state.active.get(module) else {
            let detail = format!("no checked version of `{module}` is active (EV1.7, EV1.11)");
            return Err(refuse(MODULE_NOT_CHECKED, detail));
        };
        bind_entry(version, function, &state.grant)
    }

    /// Binds an invocation of `module.function` on a route that names the digest it expects, a closure digest or a
    /// root source digest: only when that digest names the active version. A digest no successful check produced is
    /// `EV1:input.module_not_checked`, and a checked digest that is not active `EV1:stale.activation_base` (see
    /// "A route that names a digest").
    pub fn bind_pinned(&self, module: &str, function: &str, digest: [u8; 32]) -> Result<Binding, Refusal> {
        let state = self.state();
        match state.active.get(module) {
            Some(version) if version.named_by(&digest) => bind_entry(version, function, &state.grant),
            active => {
                let active = active.map_or_else(|| "none".to_owned(), |v| hex(&v.closure.digest));
                if state.checked.contains(&digest) {
                    let detail = format!(
                        "the route names {}, a checked version of `{module}` that is not active; the active version is \
                         {active} (EV1.11)",
                        hex(&digest)
                    );
                    Err(refuse(STALE_ACTIVATION_BASE, detail))
                } else {
                    let detail = format!(
                        "the route names {}, which no successful check of `{module}` produced (EV1.7)",
                        hex(&digest)
                    );
                    Err(refuse(MODULE_NOT_CHECKED, detail))
                }
            }
        }
    }
}

fn bind_entry(version: &Arc<Version>, function: &str, grant: &Grant) -> Result<Binding, Refusal> {
    let Some(ready) = version.entries.get(function) else {
        let detail = format!(
            "the check of the active version {} of `{}` covers no entry `{function}` (EV1.7)",
            hex(&version.closure.digest),
            version.module
        );
        return Err(refuse(MODULE_NOT_CHECKED, detail));
    };
    ungranted(&ready.approval().entry, grant)?;
    Ok(Binding { version: Arc::clone(version), function: function.to_owned() })
}

/// Refuses an entry whose grant does not lie within the host's grant (EV1.8, EV1.11).
fn ungranted(entry: &Entry, grant: &Grant) -> Result<(), Refusal> {
    match entry.grant.capabilities.iter().find(|c| !grant.capabilities.contains(c)) {
        None => Ok(()),
        Some(cap) => {
            let detail = format!(
                "the entry `{}.{}` was checked with a {cap:?} grant the host no longer holds (EV1.8, EV1.11)",
                entry.module, entry.function
            );
            Err(refuse(NOT_GRANTED, detail))
        }
    }
}

/// Lowers a verified closure's bytes again, in the lock's module order, for the source map `verify` does not keep.
fn relower(closure: &Closure) -> Result<Lowered, Refusal> {
    let mut files = Vec::with_capacity(closure.lock.modules.len());
    for m in &closure.lock.modules {
        let bytes =
            closure.sources.get(&m.source).ok_or_else(|| refuse(IR_REJECTED, "a locked module has no bytes"))?;
        files.push(c600_syntax::parse(bytes));
    }
    c600_syntax::lower(&files).map_err(|e| refuse(IR_REJECTED, e.message))
}

/// The check record's canonical bytes (draft form [`CHECK_REFERENCE_FORMAT`]).
fn record(
    closure: &Closure,
    entries: &BTreeMap<String, Ready>,
    identity: &Identity,
    contract: &Pin,
    checker: &Pin,
) -> Vec<u8> {
    let mut approvals: Vec<[u8; 32]> = entries.values().map(|r| r.approval().digest()).collect();
    approvals.sort();
    let mut modules: Vec<_> = closure.lock.modules.iter().collect();
    modules.sort_by_key(|m| m.source);
    let modules = modules.into_iter().map(|m| {
        Json::object([
            ("edition", Json::text(&m.edition)),
            ("module", Json::text(&m.module)),
            ("source", digest_ref("source", &m.source)),
        ])
    });
    let payload = Json::object([
        ("approvals", Json::List(approvals.iter().map(|d| digest_ref("content", d)).collect())),
        ("checker", checker.json()),
        ("closure", digest_ref("content", &closure.digest)),
        ("contract", contract.json()),
        ("ir", Json::object([("contract", Json::text(EXEC_CONTRACT)), ("version", Json::text(IR_VERSION))])),
        ("manifest", digest_ref("content", &closure.manifest_digest)),
        ("module", Json::text(&closure.manifest.module)),
        ("modules", Json::List(modules.collect())),
    ]);
    provenance::canonical(&Json::object([
        ("format", Json::text(CHECK_REFERENCE_FORMAT)),
        ("identity", identity.json().clone()),
        ("payload", payload),
        ("provenance", Json::Null),
        ("version", Json::Int(FORM_VERSION)),
    ]))
}

/// An invocation bound wholly to one checked version (EV1.11): the version's closure, its approval of the invoked
/// entry and its loaded program, held for as long as the binding lives.
#[derive(Clone)]
pub struct Binding {
    version: Arc<Version>,
    function: String,
}

impl std::fmt::Debug for Binding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Binding")
            .field("module", &self.version.module)
            .field("function", &self.function)
            .field("closure", &hex(&self.version.closure.digest))
            .finish()
    }
}

/// The end of a bound run: the version it ran, named by its closure digest and root source digest, and the run's
/// outcome (EV1.7: each run names the version it executed).
#[derive(Debug)]
pub struct BoundRun {
    pub closure: [u8; 32],
    pub source: [u8; 32],
    pub outcome: Outcome,
}

impl Binding {
    fn ready(&self) -> &Ready {
        &self.version.entries[&self.function]
    }

    pub fn module(&self) -> &str {
        &self.version.module
    }

    pub fn function(&self) -> &str {
        &self.function
    }

    /// The bound version's closure digest.
    pub fn digest(&self) -> [u8; 32] {
        self.version.closure.digest
    }

    /// The bound version's root source digest.
    pub fn source(&self) -> [u8; 32] {
        self.version.source()
    }

    /// The reference of the check the bound version passed.
    pub fn reference(&self) -> CheckReference {
        self.version.reference
    }

    /// The approval of the bound entry.
    pub fn approval(&self) -> &Approval {
        self.ready().approval()
    }

    /// The bound version's verified closure.
    pub fn closure(&self) -> &Closure {
        &self.version.closure
    }

    /// The bound version's modules in the lock's order, which is the order its lowering read them: each module's name,
    /// source digest and bytes.
    pub fn sources(&self) -> Vec<(&str, [u8; 32], &[u8])> {
        let c = &self.version.closure;
        c.lock.modules.iter().map(|m| (m.module.as_str(), m.source, c.sources[&m.source].as_slice())).collect()
    }

    /// The source ranges of the bound version's lowering (LG4.12).
    pub fn ranges(&self) -> &dyn SourceRanges {
        &*self.version
    }

    /// The pins the bound approval names: its contract and checker pins.
    pub fn pins(&self) -> Pins {
        let a = self.approval();
        Pins { contracts: vec![a.contract.clone()], checkers: vec![a.checker.clone()] }
    }

    /// Runs the bound entry in this process. The interpreter gets the bound version's source map whatever
    /// `request.options` says, so a stop's source range names the bytes that ran.
    pub fn run(&self, request: RunRequest, engine: &mut dyn Host, store: &mut Store) -> Result<BoundRun, Refusal> {
        let options = c600_run::interp::Options { sources: Some(self.ranges()), ..request.options };
        let outcome = self.ready().run(RunRequest { options, ..request }, engine, store)?;
        Ok(BoundRun { closure: self.digest(), source: self.source(), outcome })
    }

    /// Runs the bound entry through `execute`, another executor of the bound closure, as [`Ready::run_with`] does.
    pub fn run_with<E>(
        &self,
        context: RunContext,
        engine: &mut dyn Host,
        store: &mut Store,
        execute: impl FnOnce(&mut dyn Host) -> Result<Run, E>,
    ) -> Result<BoundRun, Halted<E>> {
        let outcome = self.ready().run_with(context, engine, store, execute)?;
        Ok(BoundRun { closure: self.digest(), source: self.source(), outcome })
    }
}

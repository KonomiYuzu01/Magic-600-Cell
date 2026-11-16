//! Approval of a verified closure for one entry, bound to the closure's bytes (RTE.45's `source_bound` variant; LG5.6).
//!
//! [`admit_program`] is the check an approval rests on. It refuses dynamic code first, then gives `c600-check`'s
//! findings for the entry's author column: [`c600_check::admit`] for an own, forked or imported tool, and for a
//! built-in tool the checker's refusal of another IR version first, then its diagnostics with the same capability
//! rows (BND.37, BND.56). [`approve`] binds the result to the closure digest, so [`crate::load`] refuses any other
//! bytes.

use std::collections::BTreeSet;

use c600_check::{admit, check, separation, Grant, Policy, Separation, NEVER_GRANTED};
use c600_digest::sha256;
use c600_ir::{Program, EXEC_CONTRACT, IR_VERSION};
use c600_run::provenance::{self, digest_ref, Author, Json, Pin, Profile};

use crate::code::{IR_REJECTED, IR_VERSION_UNSUPPORTED, NOT_GRANTED};
use crate::{refuse, Closure, Identity, Refusal, APPROVAL_FORMAT, FORM_VERSION};

/// What an approval is asked for: one entry, under one author class, profile, grant and policy (LG3.9, BND.17).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub module: String,
    pub function: String,
    pub author: Author,
    pub profile: Profile,
    pub grant: Grant,
    pub policy: Policy,
}

/// What admission found for an admitted entry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Admitted {
    pub separation: Separation,
    /// The open cells of chapter 07's grant table the entry touches; they are not answers (BND.59).
    pub open_cells: Vec<&'static str>,
}

/// Checks a program for one entry. Dynamic code is refused before any other finding; then the first finding refuses.
pub fn admit_program(program: &Program, entry: &Entry) -> Result<Admitted, Refusal> {
    let report = check(program).map_err(|e| refuse(IR_REJECTED, e.0))?;
    let dynamic: BTreeSet<&str> =
        report.summaries.values().flat_map(|s| s.unresolved.iter().map(String::as_str)).collect();
    if !dynamic.is_empty() {
        let names: Vec<&str> = dynamic.into_iter().collect();
        let detail = format!(
            "dynamic code: the calls {} do not resolve to checked bytes, and the approved subset refuses them (RTE.45; \
             LX-3 code gap `dynamic_code`)",
            names.join(", ")
        );
        return Err(refuse(NOT_GRANTED, detail));
    }
    let (findings, separation, open_cells) = match entry.author {
        Author::Unknown => {
            return Err(refuse(NOT_GRANTED, "author `unknown` is never approved: no row admits it (BND.21)"));
        }
        Author::BuiltIn => {
            // A program for another IR version or execution contract has no summaries: it is refused with the
            // checker's code before the entry is looked up, as `c600_check::admit` and `admit_built_in` do (LG4.1).
            if let Some(d) = report.diagnostics.iter().find(|d| d.code == IR_VERSION_UNSUPPORTED) {
                return Err(finding(d.code, &d.module, &d.function, &d.path, &d.note));
            }
            let summary = report.summary(&entry.module, &entry.function).ok_or_else(|| no_entry(entry))?;
            let mut findings: Vec<Refusal> =
                report.diagnostics.iter().map(|d| finding(d.code, &d.module, &d.function, &d.path, &d.note)).collect();
            for cap in &summary.capabilities {
                let note = if NEVER_GRANTED.contains(cap) {
                    format!("{cap:?} access is never granted in 1.0 (LG3.8)")
                } else if !entry.grant.capabilities.contains(cap) {
                    format!("the run grants no {cap:?} capability; a declaration or an import supplies none (LG3.9)")
                } else {
                    continue;
                };
                findings.push(finding(NOT_GRANTED, &entry.module, &entry.function, "entry", &note));
            }
            (findings, separation(summary, &entry.policy), Vec::new())
        }
        Author::Own | Author::ForkedBuiltIn | Author::Imported => {
            let profile = match entry.profile {
                Profile::Solving => c600_check::Profile::Solving,
                Profile::ToolAuthoring => c600_check::Profile::ToolAuthoring,
                Profile::Research => c600_check::Profile::Research,
            };
            let a = admit(program, &entry.module, &entry.function, profile, &entry.grant, &entry.policy)
                .map_err(|e| refuse(IR_REJECTED, e.0))?;
            let findings = a.diagnostics.iter().map(|d| finding(d.code, &d.module, &d.function, &d.path, &d.note));
            (findings.collect(), a.separation, a.open_cells)
        }
    };
    match findings.into_iter().next() {
        Some(first) => Err(first),
        None => Ok(Admitted { separation, open_cells }),
    }
}

fn finding(code: &'static str, module: &str, function: &str, path: &str, note: &str) -> Refusal {
    refuse(code, format!("{module}.{function} at {path}: {note}"))
}

fn no_entry(entry: &Entry) -> Refusal {
    refuse(IR_REJECTED, format!("the closure has no function `{}.{}`", entry.module, entry.function))
}

/// An approval: one entry of one closure, bound to the closure's digest (RTE.45, LG5.6). It is never stored.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Approval {
    pub identity: Identity,
    pub closure: [u8; 32],
    pub lock: [u8; 32],
    pub manifest: [u8; 32],
    /// Every module's source digest, sorted.
    pub modules: Vec<[u8; 32]>,
    /// The entry module's source digest: the producing module of the run's blocks (BND.22).
    pub module: [u8; 32],
    pub entry: Entry,
    pub contract: Pin,
    pub checker: Pin,
    pub admitted: Admitted,
}

impl Approval {
    /// The approval record's canonical bytes (FP.5; draft form `c600-approval-draft`).
    pub fn bytes(&self) -> Vec<u8> {
        let e = &self.entry;
        let grant = e.grant.capabilities.iter().map(|c| Json::Text(format!("{c:?}"))).collect();
        let declassifiers = e.policy.enabled_declassifiers.iter().map(|d| Json::text(d)).collect();
        let s = self.admitted.separation;
        let payload = Json::object([
            ("author", Json::text(e.author.name())),
            ("checker", self.checker.json()),
            ("closure", digest_ref("content", &self.closure)),
            ("contract", self.contract.json()),
            ("entry", Json::object([("function", Json::text(&e.function)), ("module", Json::text(&e.module))])),
            ("grant", Json::List(grant)),
            ("ir", Json::object([("contract", Json::text(EXEC_CONTRACT)), ("version", Json::text(IR_VERSION))])),
            ("lock", digest_ref("content", &self.lock)),
            ("manifest", digest_ref("content", &self.manifest)),
            ("module", digest_ref("source", &self.module)),
            ("modules", Json::List(self.modules.iter().map(|d| digest_ref("source", d)).collect())),
            ("open_cells", Json::List(self.admitted.open_cells.iter().map(|c| Json::text(c)).collect())),
            ("policy", Json::object([("declassifiers", Json::List(declassifiers))])),
            ("profile", Json::text(e.profile.name())),
            (
                "separation",
                Json::object([
                    ("prepares_previews", Json::Bool(s.prepares_previews)),
                    ("reads_state", Json::Bool(s.reads_state)),
                ]),
            ),
        ]);
        provenance::canonical(&Json::object([
            ("format", Json::text(APPROVAL_FORMAT)),
            ("identity", self.identity.json().clone()),
            ("payload", payload),
            ("provenance", Json::Null),
            ("version", Json::Int(FORM_VERSION)),
        ]))
    }

    pub fn digest(&self) -> [u8; 32] {
        sha256(&self.bytes())
    }
}

/// Approves one entry of a verified closure under a contract and checker pin.
pub fn approve(closure: &Closure, entry: Entry, contract: Pin, checker: Pin) -> Result<Approval, Refusal> {
    let module = closure.module_digest(&entry.module).ok_or_else(|| no_entry(&entry))?;
    let admitted = admit_program(&closure.program, &entry)?;
    Ok(Approval {
        identity: closure.identity.clone(),
        closure: closure.digest,
        lock: closure.lock_digest,
        manifest: closure.manifest_digest,
        modules: closure.sources.keys().copied().collect(),
        module,
        entry,
        contract,
        checker,
        admitted,
    })
}

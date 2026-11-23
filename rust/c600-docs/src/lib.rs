//! EV5's reference docs and principle records for 600 Cell Script (LP4-10; chapter 04 EV5.2 to EV5.5 and EV5.13,
//! chapter 03 LB6).
//!
//! - [`Checked`] parses, lowers and checks a set of `.6cs` sources. A set that does not check is a broken draft,
//!   kept apart as [`reference::Status::Broken`] (EV1.7, EV5.2).
//! - [`reference`] generates the reference entries of a module from checked metadata: source and closure digests,
//!   edition, IR version and contract, supported identities, each exported function's signature, declared and
//!   inferred effects, capabilities and grant rows, and a link to its declaration. No entry keeps a second
//!   handwritten signature (EV5.2).
//! - [`decompose`] generates an entry's decomposition from the checked IR's call graph, with its library entries and
//!   kernel calls; higher-order calls are "may call" edges (LB6.2, EV5.4).
//! - [`record`] reads a `PrincipleRecord` from its data file and admits it against the generated decomposition and
//!   the glossary: every part present or a typed none, its source and evidence digests resolving, its expected claims
//!   taken from the written derivation, and its hand-written parts stale when the decomposition changed (LB6.1,
//!   LB6.2, CT-LB6-01).
//! - [`explain`] builds a tool's explanation: the record's parts, its arguments' words from chapter 06's word function
//!   with statement, scope, basis, check and job outcome kept apart, lazy provenance links, and a fork's explanation
//!   that inherits no evidence (EV5.3, EV5.4, EV5.5, EV5.13).
//!
//! Nothing here runs a program: the fixtures run in the tests, on the real geometry. Like the IR, every form here is a
//! draft until the language's interfaces freeze; the drafts are listed in `rust/README.md`.

#![forbid(unsafe_code)]

pub mod decompose;
pub mod explain;
pub mod json;
pub mod record;
pub mod reference;

use c600_check::{check, Report};
use c600_syntax::{lower, parse, Lowered};

/// The chapter 13 codes this crate raises (chapter 03, LB6; registered in chapter 13's LB6 table).
pub mod code {
    /// A required `PrincipleRecord` part, or a resolving evidence or fixture reference, is absent (LB6.1, LB6.2).
    pub const PRINCIPLE_RECORD_INCOMPLETE: &str = "LB6:evidence.principle_record_incomplete";
    /// A submitted graph omits a derived call or effect, or names other source or build inputs (LB6.6, RTE.80).
    pub const DECOMPOSITION_MISMATCH: &str = "LB6:evidence.decomposition_mismatch";
}

/// A refusal with its chapter 13 code.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DocsError {
    pub code: &'static str,
    pub detail: String,
}

impl DocsError {
    pub fn new(code: &'static str, detail: impl Into<String>) -> DocsError {
        DocsError { code, detail: detail.into() }
    }
}

impl std::fmt::Display for DocsError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.detail)
    }
}

impl std::error::Error for DocsError {}

/// A set of sources that parse, lower and check with no finding: the inputs of reference docs and decompositions.
#[derive(Clone, Debug)]
pub struct Checked {
    /// The sources in the order given.
    pub sources: Vec<Vec<u8>>,
    pub lowered: Lowered,
    pub report: Report,
}

/// Why a set of sources is a broken draft: each finding's code and text, in order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Broken {
    pub findings: Vec<(String, String)>,
}

impl Checked {
    /// Parses, lowers and checks `sources`, one module each. Imports resolve by digest among them.
    pub fn new(sources: &[&[u8]]) -> Result<Checked, Broken> {
        let mut files = Vec::new();
        let mut findings = Vec::new();
        for bytes in sources {
            let file = parse(bytes);
            for d in &file.diagnostics {
                findings.push((d.code.to_string(), d.message.clone()));
            }
            if !file.holes.is_empty() && file.diagnostics.is_empty() {
                findings.push((c600_syntax::code::RECOVERY_GAP.to_string(), "a recovery hole".into()));
            }
            files.push(file);
        }
        if !findings.is_empty() {
            return Err(Broken { findings });
        }
        let lowered = lower(&files)
            .map_err(|e| Broken { findings: vec![("LG4:input.ir_rejected".to_string(), e.to_string())] })?;
        let report = check(&lowered.program)
            .map_err(|e| Broken { findings: vec![("LG4:input.ir_rejected".to_string(), e.to_string())] })?;
        if !report.is_admitted() {
            let findings = report
                .diagnostics
                .iter()
                .map(|d| (d.code.to_string(), format!("{}.{} at {}: {}", d.module, d.function, d.path, d.note)))
                .collect();
            return Err(Broken { findings });
        }
        Ok(Checked { sources: sources.iter().map(|s| s.to_vec()).collect(), lowered, report })
    }

    /// The decomposition of `module.entry`.
    pub fn decompose(&self, module: &str, entry: &str) -> Result<decompose::Decomposition, DocsError> {
        decompose::decompose(&self.lowered, &self.report, module, entry)
    }

    /// The source bytes of `module`.
    pub fn source(&self, module: &str) -> Option<&[u8]> {
        let digest = &self.lowered.source(module)?.digest;
        self.sources.iter().find(|s| c600_digest::sha256_hex(s) == *digest).map(Vec::as_slice)
    }
}

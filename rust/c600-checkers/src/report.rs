//! KN6.1's interface: the certificate a check reads, the kind it is checked as, and the report it gives.
//!
//! The report is this crate's own type. `c600-evidence` admits a report as one leaf (EVD.46); the research host's job
//! adapter maps this report onto that admission (LP5 plan, piece R8), so nothing here depends on the evidence library.

use std::fmt;
use std::sync::OnceLock;

/// Chapter 13's KN6 codes, the only codes this crate gives.
pub mod codes {
    /// A certificate fails to support its claim for the stated inputs (KN6.1, KN6.6).
    pub const CERTIFICATE_REJECTED: &str = "KN6:evidence.certificate_rejected";
    /// No checker is available for this certificate kind (KN6.1).
    pub const NO_CHECKER: &str = "KN6:evidence.no_checker";
    /// The certificate or check request supplies the wrong start table (KN6.5).
    pub const START_TABLE_MISMATCH: &str = "KN6:evidence.start_table_mismatch";
}

/// One immutable input of a check, named by its role (a start table, a menu record, a stored word).
#[derive(Clone, Copy, Debug)]
pub struct Input<'a> {
    pub role: &'a str,
    pub bytes: &'a [u8],
}

/// KN6.1's `CertificateRef`: the certificate's bytes, the SHA-256 they are stated to have (from the archive's manifest
/// or the caller's record), and the immutable inputs the certificate names. A checker reads nothing else.
#[derive(Clone, Copy, Debug)]
pub struct CertificateRef<'a> {
    pub digest: &'a str,
    pub bytes: &'a [u8],
    pub inputs: &'a [Input<'a>],
}

impl<'a> CertificateRef<'a> {
    /// The input with this role, if supplied.
    pub fn input(&self, role: &str) -> Option<&'a Input<'a>> {
        self.inputs.iter().find(|input| input.role == role)
    }
}

/// KN6.1's certificate kinds. The research-archive kind is a family of sub-checkers, one per claim family, each with
/// its own identity and mutation gate (the LP5 plan's reading A); its member is named by the sub-checker's ID.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CheckerKind {
    /// Admissibility and straddlers.
    Admissibility,
    /// Macro certificates.
    Macro,
    /// Compressed-word expansion.
    Expansion,
    /// ν and label checkpoints.
    Checkpoints,
    /// Research archives: the sub-checker registered under this ID ([`crate::archive`]).
    Archive(&'static str),
}

impl CheckerKind {
    pub fn name(&self) -> &'static str {
        match self {
            CheckerKind::Admissibility => "admissibility and straddlers",
            CheckerKind::Macro => "macro certificates",
            CheckerKind::Expansion => "compressed-word expansion",
            CheckerKind::Checkpoints => "ν and label checkpoints",
            CheckerKind::Archive(_) => "research archives",
        }
    }
}

/// A checker's identity (KN6.3): its name and the SHA-256 of a `sha256sum`-style listing of its sources.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CheckerId {
    pub id: &'static str,
    pub source: String,
}

/// The sources one checker is made of, hashed into its [`CheckerId`]. The listing names each file relative to `rust/`.
pub struct Sources {
    pub id: &'static str,
    pub files: &'static [(&'static str, &'static [u8])],
    digest: OnceLock<String>,
}

impl Sources {
    pub const fn new(id: &'static str, files: &'static [(&'static str, &'static [u8])]) -> Sources {
        Sources { id, files, digest: OnceLock::new() }
    }

    /// The checker's identity: its ID and source digest.
    pub fn identity(&self) -> CheckerId {
        let source = self.digest.get_or_init(|| source_digest(self.files)).clone();
        CheckerId { id: self.id, source }
    }
}

/// The SHA-256 of a `sha256sum`-style listing (`<sha256>  <name>\n` per file, in the given order), as
/// `c600_kernel::build` hashes the kernel's sources.
pub fn source_digest(files: &[(&str, &[u8])]) -> String {
    let mut listing = String::new();
    for (name, bytes) in files {
        listing.push_str(&format!("{}  {name}\n", c600_digest::sha256_hex(bytes)));
    }
    c600_digest::sha256_hex(listing.as_bytes())
}

/// A claim's name: a path into the certificate, such as `macros[0].claims.digests[2]`.
pub type ClaimId = String;

/// KN6.1's verdict. A failure names the first claim that does not hold, with its KN6 code and the checker's own
/// account of the difference.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Verdict {
    Pass,
    Fail { claim: ClaimId, code: &'static str, reason: String },
    NotApplicable { reason: String },
}

impl Verdict {
    pub fn is_pass(&self) -> bool {
        matches!(self, Verdict::Pass)
    }

    pub fn is_fail(&self) -> bool {
        matches!(self, Verdict::Fail { .. })
    }

    /// The code of a failure.
    pub fn code(&self) -> Option<&'static str> {
        match self {
            Verdict::Fail { code, .. } => Some(code),
            _ => None,
        }
    }

    /// The claim a failure names.
    pub fn claim(&self) -> Option<&str> {
        match self {
            Verdict::Fail { claim, .. } => Some(claim),
            _ => None,
        }
    }
}

/// A stated input of the report: its role, the SHA-256 of its bytes, and the identity the checker read from it (a state
/// digest, a menu identity), so the report says for which inputs the claims hold (KN6.6, EVD.59).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BoundInput {
    pub role: String,
    pub sha256: String,
    pub identity: Vec<(String, String)>,
}

/// A span of a long word that a trace step cites instead of its letters (KN6.4): the word's digest and the positions
/// `[from, to)`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Span {
    pub word: String,
    pub from: u64,
    pub to: u64,
}

/// One step of the checker's own per-claim trace: the claim, whether it held, the value the checker computed, and the
/// span of the word it covers when there is one. Stepping through it needs no producer (KN6.4).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TraceStep {
    pub claim: ClaimId,
    pub held: bool,
    pub computed: String,
    pub span: Option<Span>,
}

/// KN6.1's `CheckReport`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CheckReport {
    pub verdict: Verdict,
    /// The checker that ran and its source digest (KN6.3).
    pub checker: CheckerId,
    pub kind: CheckerKind,
    /// The SHA-256 of the certificate bytes, computed by the checker.
    pub input: String,
    /// The stated inputs the claims were checked for.
    pub inputs: Vec<BoundInput>,
    /// The checker's per-claim trace, in the order the claims were checked.
    pub trace: Vec<TraceStep>,
}

impl CheckReport {
    /// Whether every claim the trace names held and the verdict is a pass.
    pub fn complete(&self) -> bool {
        self.verdict.is_pass() && self.trace.iter().all(|step| step.held)
    }
}

/// A check that could not run: a kind with no checker, or an engine the kind needs that the caller did not supply.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Refusal {
    pub code: &'static str,
    pub message: String,
}

impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for Refusal {}

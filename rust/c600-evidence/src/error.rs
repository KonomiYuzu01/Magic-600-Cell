//! Chapter 06 section 11 and chapter 13: the codes this library raises. Tests read `code`, never `detail` (ERR.6).

/// EVD.17, EVD.58.
pub const PREMISE_UNMET: &str = "EVD:evidence.premise_unmet";
/// EVD.18, EVD.58.
pub const SCOPE_MISMATCH: &str = "EVD:evidence.scope_mismatch";
/// EVD.40: I1 to I4 and I6 to I8.
pub const INCONSISTENT_FIELDS: &str = "EVD:evidence.inconsistent_fields";
/// EVD.40: I5, and EVD.43's refusal.
pub const NEGATIVE_WITHOUT_COMPLETION: &str = "EVD:evidence.negative_without_completion";
/// EVD.65.
pub const FIELD_NOT_WRITABLE: &str = "EVD:capability.field_not_writable";

/// The inconsistency cases of EVD.40.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Inconsistency {
    I1,
    I2,
    I3,
    I4,
    I5,
    I6,
    I7,
    I8,
}

/// A refusal with its chapter 13 code. `cases` lists the EVD.40 cases a row-0 refusal found.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvidenceError {
    pub code: &'static str,
    pub cases: Vec<Inconsistency>,
    pub detail: String,
}

impl EvidenceError {
    pub fn premise_unmet(detail: &str) -> EvidenceError {
        EvidenceError { code: PREMISE_UNMET, cases: Vec::new(), detail: detail.to_string() }
    }

    pub fn scope_mismatch(detail: &str) -> EvidenceError {
        EvidenceError { code: SCOPE_MISMATCH, cases: Vec::new(), detail: detail.to_string() }
    }

    pub fn field_not_writable(detail: &str) -> EvidenceError {
        EvidenceError { code: FIELD_NOT_WRITABLE, cases: Vec::new(), detail: detail.to_string() }
    }

    /// Row 0. I5 runs first (EVD.30, EVD.42), so a finding set that holds I5 keeps I5's code.
    pub fn inconsistent(mut cases: Vec<Inconsistency>, detail: &str) -> EvidenceError {
        cases.sort();
        cases.dedup();
        let code = if cases.contains(&Inconsistency::I5) { NEGATIVE_WITHOUT_COMPLETION } else { INCONSISTENT_FIELDS };
        EvidenceError { code, cases, detail: detail.to_string() }
    }

    pub fn has(&self, case: Inconsistency) -> bool {
        self.cases.contains(&case)
    }
}

impl std::fmt::Display for EvidenceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.detail)
    }
}

impl std::error::Error for EvidenceError {}

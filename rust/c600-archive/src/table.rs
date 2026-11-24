//! PS3.10's versioned import table keyed by (format, field, value), in FP.21's rule shape.
//!
//! Edition 1 (`data/import-table-1.json`) is the conversion of the pinned label table
//! (`evidence-import/corpus-labels.json`, EVD.49) that `tests/test_lp5_archive.py` makes and checks byte for byte. Its
//! declared-format rows are rules with a `declared` selector. The label table's synthetic-format rows are kept as
//! pinned rows that select nothing: a synthetic path format cannot identify a missing format (FP.22), so an
//! absent-format occurrence stays recorded (EVD.53). The rows from outside the corpus are kept beside them. Facts are
//! fragments of the six fields of partition section 8.1 and EVD.47's import values, never a whole claim inferred from
//! a label (EVD.50).

use std::collections::BTreeMap;

use c600_evidence::{Basis, Check, CheckMode, JobOutcome};
use c600_kernel::json::{parse, Value};

use crate::enumerate::Class;
use crate::ArchiveError;

/// Edition 1's bytes, embedded so that the edition's content digest is fixed by the build.
pub const EDITION_1: &[u8] = include_bytes!("../data/import-table-1.json");

/// The six fields a row may set (EVD.50), with their values as the table writes them.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Facts {
    pub claim: Option<String>,
    pub scope: Option<String>,
    pub basis: Option<String>,
    pub check: Option<String>,
    pub check_mode: Option<String>,
    pub job_outcome: Option<String>,
}

impl Facts {
    /// The basis as chapter 06's type; a value outside edition 1 stays `Other` (I1).
    pub fn basis(&self) -> Option<Basis> {
        self.basis.as_deref().map(|b| match b {
            "theorem" => Basis::Theorem,
            "certificate" => Basis::Certificate,
            "exhaustive computation" => Basis::ExhaustiveComputation,
            "exact computation" => Basis::ExactComputation,
            "sample" => Basis::Sample,
            other => Basis::Other(other.to_string()),
        })
    }

    pub fn check(&self) -> Option<Check> {
        self.check.as_deref().map(|c| match c {
            "none" => Check::None,
            "producer" => Check::Producer,
            "independent checker" => Check::IndependentChecker,
            "model review" => Check::ModelReview,
            other => Check::Other(other.to_string()),
        })
    }

    pub fn check_mode(&self) -> Option<CheckMode> {
        self.check_mode.as_deref().map(|m| match m {
            "reused through exact transport" => CheckMode::ReusedExactTransport,
            "fresh exact audit" => CheckMode::FreshExactAudit,
            "actual replay" => CheckMode::ActualReplay,
            "stated: not replayed" => CheckMode::StatedNotReplayed,
            other => CheckMode::Other(other.to_string()),
        })
    }

    pub fn job_outcome(&self) -> Option<JobOutcome> {
        self.job_outcome.as_deref().map(|j| match j {
            "completed" => JobOutcome::Completed,
            "cancelled" => JobOutcome::Cancelled,
            "cut off" => JobOutcome::CutOff,
            "error" => JobOutcome::Error,
            "not stated" => JobOutcome::NotStated,
            other => JobOutcome::Other(other.to_string()),
        })
    }

    fn from_value(v: &Value) -> Result<Facts, ArchiveError> {
        let fields = v.as_object().ok_or(ArchiveError::format("facts is not an object"))?;
        let mut f = Facts::default();
        for (k, x) in fields {
            let text = x.as_str().ok_or(ArchiveError::format("a fact is not text"))?.to_string();
            let slot = match k.as_str() {
                "claim" => &mut f.claim,
                "scope" => &mut f.scope,
                "basis" => &mut f.basis,
                "check" => &mut f.check,
                "check_mode" => &mut f.check_mode,
                "job_outcome" => &mut f.job_outcome,
                _ => return Err(ArchiveError::format("a fact names an unknown field")),
            };
            *slot = Some(text);
        }
        if f == Facts::default() {
            return Err(ArchiveError::format("a facts outcome sets no field"));
        }
        Ok(f)
    }
}

/// A rule's outcome (FP.21): facts, or recorded with its reason.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    Facts(Facts),
    Recorded,
}

/// One rule with a `declared` selector.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rule {
    pub format: String,
    pub field: String,
    pub value: Value,
    pub class: Class,
    pub outcome: Outcome,
    pub prerequisites: Vec<String>,
    /// The label table's rule identifier and note, kept word for word.
    pub reason: String,
}

/// A versioned import table.
#[derive(Clone, Debug)]
pub struct ImportTable {
    pub edition: String,
    /// SHA-256 of the edition's bytes: the content digest every converted envelope cites.
    pub sha256: String,
    /// The distinct-value limit of EVD.48's class S per (format, field).
    pub distinct_value_limit: usize,
    rules: Vec<Rule>,
    index: BTreeMap<(String, String, String), usize>,
    /// The pinned synthetic-format rows, which select nothing.
    pub absent_format_rows: usize,
    /// The rows from outside the corpus.
    pub external_rows: usize,
}

/// The mapping edition an import uses (PS3.17's `ImportTableRef`): the import table and its absent-format dispatch.
#[derive(Clone, Copy, Debug)]
pub struct ImportTableRef<'a> {
    pub table: &'a ImportTable,
    pub discriminator: &'a crate::Discriminator,
}

impl ImportTable {
    /// Edition 1, embedded in this crate.
    pub fn edition_1() -> ImportTable {
        ImportTable::parse(EDITION_1).expect("the embedded import table edition 1 is valid")
    }

    /// Reads an edition. Equal keys refuse the table: there is no first-match guess (FP.21).
    pub fn parse(bytes: &[u8]) -> Result<ImportTable, ArchiveError> {
        let v = parse(bytes).map_err(|_| ArchiveError::format("the import table is not strict JSON"))?;
        if v.get("format").and_then(Value::as_str) != Some("c600-import-table")
            || v.get("version") != Some(&Value::Int(1))
        {
            return Err(ArchiveError::format("not an import table of version 1"));
        }
        let edition = text(&v, "edition")?;
        let limit = v
            .get("enumeration")
            .and_then(|e| e.get("distinct_value_limit"))
            .and_then(Value::as_i64)
            .filter(|n| *n > 0)
            .ok_or(ArchiveError::format("the enumeration has no distinct-value limit"))?;
        let mut rules = Vec::new();
        let mut index = BTreeMap::new();
        for r in v.get("rules").and_then(Value::as_array).ok_or(ArchiveError::format("no rules"))? {
            let selector = r.get("selector").ok_or(ArchiveError::format("a rule has no selector"))?;
            if selector.get("kind").and_then(Value::as_str) != Some("declared") {
                return Err(ArchiveError::format("edition 1 rules have declared selectors only"));
            }
            let format = text(selector, "format")?;
            let field = text(r, "field")?;
            let value = r.get("value").cloned().ok_or(ArchiveError::format("a rule has no value"))?;
            let class = match r.get("class").and_then(Value::as_str) {
                Some("S") => Class::S,
                Some("B") => Class::B,
                _ => return Err(ArchiveError::format("a rule's class is neither S nor B")),
            };
            let outcome = match r.get("outcome").and_then(Value::as_str) {
                Some("facts") => Outcome::Facts(Facts::from_value(r.get("facts").unwrap_or(&Value::Null))?),
                Some("recorded") if r.get("facts") == Some(&Value::Null) => Outcome::Recorded,
                _ => return Err(ArchiveError::format("a rule's outcome is neither facts nor recorded")),
            };
            let prerequisites = r
                .get("prerequisites")
                .and_then(Value::as_array)
                .ok_or(ArchiveError::format("a rule has no prerequisite list"))?
                .iter()
                .map(|p| p.as_str().map(str::to_string).ok_or(ArchiveError::format("a prerequisite is not text")))
                .collect::<Result<Vec<_>, _>>()?;
            let reason = text(r, "reason")?;
            let key = (format.clone(), field.clone(), value.canonical());
            if index.insert(key, rules.len()).is_some() {
                return Err(ArchiveError::format("two rules have the same (format, field, value)"));
            }
            rules.push(Rule { format, field, value, class, outcome, prerequisites, reason });
        }
        let count = |name: &str| v.get(name).and_then(Value::as_array).map_or(0, <[Value]>::len);
        Ok(ImportTable {
            edition,
            sha256: c600_digest::sha256_hex(bytes),
            distinct_value_limit: limit as usize,
            rules,
            index,
            absent_format_rows: count("absent_format_rows"),
            external_rows: count("external_rows"),
        })
    }

    /// The rule for an exact (format, field, value), with its index.
    pub fn lookup(&self, format: &str, field: &str, value: &Value) -> Option<(usize, &Rule)> {
        self.index.get(&(format.to_string(), field.to_string(), value.canonical())).map(|&i| (i, &self.rules[i]))
    }

    pub fn rules(&self) -> &[Rule] {
        &self.rules
    }
}

fn text(v: &Value, key: &str) -> Result<String, ArchiveError> {
    v.get(key)
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or(ArchiveError::format("a required text field is missing"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table(rules: &str) -> Result<ImportTable, ArchiveError> {
        let doc = format!(
            r#"{{"format":"c600-import-table","version":1,"edition":"t","enumeration":{{"distinct_value_limit":32}},"rules":[{rules}]}}"#
        );
        ImportTable::parse(doc.as_bytes())
    }

    const RULE: &str = r#"{"selector":{"kind":"declared","format":"f/1"},"field":"status","value":"done","class":"S","outcome":"facts","facts":{"job_outcome":"completed"},"prerequisites":[],"reason":"J1"}"#;

    #[test]
    fn edition_1_loads_with_its_digest() {
        let t = ImportTable::edition_1();
        assert_eq!(t.edition, "c600-import-table-1");
        assert_eq!(t.sha256, c600_digest::sha256_hex(EDITION_1));
        assert!(!t.rules().is_empty());
        // FP.21: rules sorted by the canonical bytes of selector, field and value.
        let key = |r: &Rule| {
            let selector = c600_kernel::json::object([
                ("format", Value::String(r.format.clone())),
                ("kind", Value::String("declared".into())),
            ]);
            (selector.canonical(), Value::String(r.field.clone()).canonical(), r.value.canonical())
        };
        assert!(t.rules().windows(2).all(|w| key(&w[0]) < key(&w[1])));
    }

    #[test]
    fn lookup_is_exact_and_equal_keys_refuse_the_table() {
        let t = table(RULE).unwrap();
        let (_, r) = t.lookup("f/1", "status", &Value::String("done".into())).unwrap();
        assert_eq!(r.outcome, Outcome::Facts(Facts { job_outcome: Some("completed".into()), ..Facts::default() }));
        assert!(t.lookup("f/1", "status", &Value::String("Done".into())).is_none());
        assert!(t.lookup("f/2", "status", &Value::String("done".into())).is_none());
        assert!(t.lookup("f/1", "x.status", &Value::String("done".into())).is_none());
        let recorded = RULE.replace(
            r#""outcome":"facts","facts":{"job_outcome":"completed"}"#,
            r#""outcome":"recorded","facts":null"#,
        );
        assert_eq!(table(&format!("{RULE},{recorded}")).unwrap_err().code, crate::FORMAT);
        assert_eq!(table(&format!("{RULE},{RULE}")).unwrap_err().code, crate::FORMAT);
        // A selector other than declared, and a facts outcome with no field, refuse the table.
        assert!(table(&RULE.replace("declared", "absent")).is_err());
        assert!(table(&RULE.replace(r#"{"job_outcome":"completed"}"#, "{}")).is_err());
    }

    #[test]
    fn facts_map_to_chapter_06_types() {
        let f = Facts {
            basis: Some("certificate".into()),
            check: Some("producer".into()),
            check_mode: Some("actual replay".into()),
            job_outcome: Some("cut off".into()),
            ..Facts::default()
        };
        assert_eq!(f.basis(), Some(Basis::Certificate));
        assert_eq!(f.check(), Some(Check::Producer));
        assert_eq!(f.check_mode(), Some(CheckMode::ActualReplay));
        assert_eq!(f.job_outcome(), Some(JobOutcome::CutOff));
    }
}

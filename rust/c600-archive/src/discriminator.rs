//! PS3.12 and FP.22: the discriminator for records with no top-level `format` key, as versioned LP5 data.
//!
//! A rule's selector is `{kind:absent, manifest, source, pointer}`: the stored-byte digest of the corpus manifest the
//! record was granted under, the record's own stored-byte digest and an exact JSON pointer into it. An interpretation
//! needs an exact manifest/source/pointer match and every prerequisite. Zero matches leave the record recorded
//! (EVD.53), two matches with different interpretations leave it recorded as ambiguous, and no filename, synthetic path
//! label or status word is ever consulted. The original absence stays absent; an interpretation is kept beside it as a
//! separate recorded interpretation, and it produces no facts on its own.
//!
//! Edition 1 (`data/discriminator-1.json`) holds one rule per record whose own bytes declare the format of a member
//! object at a pointer (nine stage 15 run wrappers at `/run`, one stage 18 lattice report at `/player`). Its rules
//! await the integrator's review, which the edition states; the other 146 absent-format records have no rule.

use c600_kernel::json::{parse, Value};

use crate::ArchiveError;

/// Edition 1's bytes.
pub const EDITION_1: &[u8] = include_bytes!("../data/discriminator-1.json");

/// One rule of the discriminator.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiscriminatorRule {
    /// SHA-256 of the corpus manifest's stored bytes.
    pub manifest: String,
    /// SHA-256 of the record's stored bytes.
    pub source: String,
    /// The exact JSON pointer of the interpreted member.
    pub pointer: String,
    /// The format the member at the pointer declares.
    pub inner_format: String,
    pub reason: String,
}

/// What the discriminator says about one absent-format record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Discrimination {
    /// No rule matches: the record stays recorded (EVD.53).
    Undiscriminated,
    /// One interpretation, kept separately from the record's absence.
    Discriminated { rule: usize, pointer: String, inner_format: String },
    /// A matching rule whose prerequisite fails: recorded with the reason.
    PrerequisiteUnmet { rule: usize, reason: &'static str },
    /// Matching rules disagree: recorded as ambiguous.
    Ambiguous { rules: Vec<usize> },
}

/// A discriminator edition.
#[derive(Clone, Debug)]
pub struct Discriminator {
    pub edition: String,
    pub sha256: String,
    /// The edition's review state, kept word for word.
    pub review: String,
    rules: Vec<DiscriminatorRule>,
}

impl Discriminator {
    pub fn edition_1() -> Discriminator {
        Discriminator::parse(EDITION_1).expect("the embedded discriminator edition 1 is valid")
    }

    pub fn parse(bytes: &[u8]) -> Result<Discriminator, ArchiveError> {
        let v = parse(bytes).map_err(|_| ArchiveError::format("the discriminator is not strict JSON"))?;
        if v.get("format").and_then(Value::as_str) != Some("c600-absent-format-discriminator")
            || v.get("version") != Some(&Value::Int(1))
        {
            return Err(ArchiveError::format("not a discriminator of version 1"));
        }
        let text = |x: &Value, k: &str| {
            x.get(k).and_then(Value::as_str).map(str::to_string).ok_or(ArchiveError::format("a discriminator field"))
        };
        let digest = |x: &Value, k: &str| -> Result<String, ArchiveError> {
            let d = x.get(k).ok_or(ArchiveError::format("a selector digest is missing"))?;
            if d.get("role").and_then(Value::as_str) != Some("bytes") {
                return Err(ArchiveError::format("a selector digest is not a bytes digest"));
            }
            let hex = text(d, "sha256")?;
            if !crate::is_sha256(&hex) {
                return Err(ArchiveError::format("a selector digest is not 64 lowercase hex digits"));
            }
            Ok(hex)
        };
        let mut rules = Vec::new();
        for r in v.get("rules").and_then(Value::as_array).ok_or(ArchiveError::format("no rules"))? {
            let s = r.get("selector").ok_or(ArchiveError::format("a rule has no selector"))?;
            if s.get("kind").and_then(Value::as_str) != Some("absent") {
                return Err(ArchiveError::format("a discriminator selector is not absent"));
            }
            let pointer = text(s, "pointer")?;
            if !(pointer.is_empty() || pointer.starts_with('/')) {
                return Err(ArchiveError::format("a selector pointer is not a JSON pointer"));
            }
            let interpretation = r.get("interpretation").ok_or(ArchiveError::format("a rule has no interpretation"))?;
            rules.push(DiscriminatorRule {
                manifest: digest(s, "manifest")?,
                source: digest(s, "source")?,
                pointer,
                inner_format: text(interpretation, "inner_format")?,
                reason: text(r, "reason")?,
            });
        }
        Ok(Discriminator {
            edition: text(&v, "edition")?,
            sha256: c600_digest::sha256_hex(bytes),
            review: text(&v, "review")?,
            rules,
        })
    }

    pub fn rules(&self) -> &[DiscriminatorRule] {
        &self.rules
    }

    /// Discriminates one absent-format record from its digests and its own decoded value only.
    pub fn discriminate(&self, manifest: &str, source: &str, record: &Value) -> Discrimination {
        let matches: Vec<usize> = (0..self.rules.len())
            .filter(|&i| self.rules[i].manifest == manifest && self.rules[i].source == source)
            .collect();
        match matches.as_slice() {
            [] => Discrimination::Undiscriminated,
            [first, rest @ ..] => {
                let r = &self.rules[*first];
                if rest
                    .iter()
                    .any(|&i| (&self.rules[i].pointer, &self.rules[i].inner_format) != (&r.pointer, &r.inner_format))
                {
                    return Discrimination::Ambiguous { rules: matches };
                }
                match pointer_get(record, &r.pointer) {
                    None => {
                        Discrimination::PrerequisiteUnmet { rule: *first, reason: "the pointer resolves to nothing" }
                    }
                    Some(member) if member.get("format").and_then(Value::as_str) != Some(r.inner_format.as_str()) => {
                        Discrimination::PrerequisiteUnmet {
                            rule: *first,
                            reason: "the member at the pointer does not declare the rule's format",
                        }
                    }
                    Some(_) => Discrimination::Discriminated {
                        rule: *first,
                        pointer: r.pointer.clone(),
                        inner_format: r.inner_format.clone(),
                    },
                }
            }
        }
    }
}

/// RFC 6901: the value at `pointer`, or `None`.
pub fn pointer_get<'a>(v: &'a Value, pointer: &str) -> Option<&'a Value> {
    if pointer.is_empty() {
        return Some(v);
    }
    let mut at = v;
    for token in pointer.strip_prefix('/')?.split('/') {
        let token = token.replace("~1", "/").replace("~0", "~");
        at = match at {
            Value::Object(fields) => fields.get(&token)?,
            Value::Array(items) => {
                if token.len() > 1 && token.starts_with('0') {
                    return None;
                }
                items.get(token.parse::<usize>().ok()?)?
            }
            _ => return None,
        };
    }
    Some(at)
}

#[cfg(test)]
mod tests {
    use super::*;

    const M: &str = "1111111111111111111111111111111111111111111111111111111111111111";
    const S: &str = "2222222222222222222222222222222222222222222222222222222222222222";

    fn edition(rules: &[(&str, &str, &str, &str)]) -> Discriminator {
        let rows: Vec<String> = rules
            .iter()
            .map(|(m, s, p, f)| {
                format!(
                    r#"{{"selector":{{"kind":"absent","manifest":{{"role":"bytes","sha256":"{m}"}},"source":{{"role":"bytes","sha256":"{s}"}},"pointer":"{p}"}},"interpretation":{{"inner_format":"{f}"}},"prerequisites":[],"reason":"r"}}"#
                )
            })
            .collect();
        let doc = format!(
            r#"{{"format":"c600-absent-format-discriminator","version":1,"edition":"t","review":"none","rules":[{}]}}"#,
            rows.join(",")
        );
        Discriminator::parse(doc.as_bytes()).unwrap()
    }

    #[test]
    fn edition_1_loads() {
        let d = Discriminator::edition_1();
        assert_eq!(d.edition, "c600-absent-format-discriminator-1");
        assert_eq!(d.rules().len(), 10);
        assert!(d.rules().iter().all(|r| r.pointer == "/run" || r.pointer == "/player"));
    }

    #[test]
    fn an_exact_match_with_its_prerequisite_discriminates() {
        let d = edition(&[(M, S, "/run", "pstar-run/1")]);
        let rec = parse(br#"{"kind":"pstar","run":{"format":"pstar-run/1"}}"#).unwrap();
        assert_eq!(
            d.discriminate(M, S, &rec),
            Discrimination::Discriminated { rule: 0, pointer: "/run".into(), inner_format: "pstar-run/1".into() }
        );
        // Another manifest or another source digest matches nothing, whatever the record says.
        assert_eq!(d.discriminate(S, S, &rec), Discrimination::Undiscriminated);
        assert_eq!(d.discriminate(M, M, &rec), Discrimination::Undiscriminated);
        // The prerequisite reads the record's own bytes.
        let other = parse(br#"{"kind":"pstar","run":{"format":"pstar-run/2"}}"#).unwrap();
        assert!(matches!(d.discriminate(M, S, &other), Discrimination::PrerequisiteUnmet { rule: 0, .. }));
        let none = parse(br#"{"kind":"pstar"}"#).unwrap();
        assert!(matches!(d.discriminate(M, S, &none), Discrimination::PrerequisiteUnmet { rule: 0, .. }));
    }

    #[test]
    fn conflicting_matches_stay_ambiguous() {
        let d = edition(&[(M, S, "/run", "pstar-run/1"), (M, S, "/player", "literal-lattice/1")]);
        let rec = parse(br#"{"run":{"format":"pstar-run/1"},"player":{"format":"literal-lattice/1"}}"#).unwrap();
        assert_eq!(d.discriminate(M, S, &rec), Discrimination::Ambiguous { rules: vec![0, 1] });
    }

    #[test]
    fn pointers_follow_rfc_6901() {
        let v = parse(br#"{"a/b":{"~x":[10,20]}}"#).unwrap();
        assert_eq!(pointer_get(&v, "/a~1b/~0x/1"), Some(&Value::Int(20)));
        assert_eq!(pointer_get(&v, "/a~1b/~0x/01"), None);
        assert_eq!(pointer_get(&v, ""), Some(&v));
        assert_eq!(pointer_get(&v, "a"), None);
    }
}

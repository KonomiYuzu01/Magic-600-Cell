//! The state of one check while it runs: the certificate, the stated inputs it bound, its per-claim trace, and the
//! early stop of a failed or inapplicable claim. Every checker of this crate, the archive sub-checkers included, reads
//! the certificate through these helpers, so a malformed field fails as a claim and never panics.

use std::collections::BTreeMap;
use std::fmt::Display;

use c600_kernel::json::{self, Value};

use crate::report::{codes, BoundInput, CertificateRef, ClaimId, Span, TraceStep};

/// Why a check stopped before its last claim.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Stop {
    Fail { claim: ClaimId, code: &'static str, reason: String },
    NotApplicable { reason: String },
}

/// A step of a check: its value, or the stop.
pub type Step<T> = Result<T, Stop>;

/// A claim that does not hold for the stated inputs.
pub fn reject(claim: impl Into<String>, reason: impl Into<String>) -> Stop {
    Stop::Fail { claim: claim.into(), code: codes::CERTIFICATE_REJECTED, reason: reason.into() }
}

/// A start table that is not the one the certificate names.
pub fn start_mismatch(claim: impl Into<String>, reason: impl Into<String>) -> Stop {
    Stop::Fail { claim: claim.into(), code: codes::START_TABLE_MISMATCH, reason: reason.into() }
}

/// A certificate outside the checker's kind (KN6.6: no judgement beyond the kind).
pub fn not_applicable(reason: impl Into<String>) -> Stop {
    Stop::NotApplicable { reason: reason.into() }
}

/// One running check.
pub struct Run<'a> {
    pub cert: &'a CertificateRef<'a>,
    pub trace: Vec<TraceStep>,
    pub inputs: Vec<BoundInput>,
}

impl<'a> Run<'a> {
    pub fn new(cert: &'a CertificateRef<'a>) -> Run<'a> {
        Run { cert, trace: Vec::new(), inputs: Vec::new() }
    }

    /// Records a claim that held, with the value the checker computed.
    pub fn held(&mut self, claim: impl Into<String>, computed: impl Display) {
        self.trace.push(TraceStep { claim: claim.into(), held: true, computed: computed.to_string(), span: None });
    }

    /// Records a claim over a span of a long word that held.
    pub fn held_span(&mut self, claim: impl Into<String>, computed: impl Display, span: Span) {
        self.trace.push(TraceStep {
            claim: claim.into(),
            held: true,
            computed: computed.to_string(),
            span: Some(span),
        });
    }

    /// Records a claim that did not hold and returns the stop that names it.
    pub fn failed(&mut self, stop: Stop, computed: impl Display) -> Stop {
        if let Stop::Fail { claim, .. } = &stop {
            self.trace.push(TraceStep {
                claim: claim.clone(),
                held: false,
                computed: computed.to_string(),
                span: None,
            });
        }
        stop
    }

    /// Compares a stated value with the computed one under `code`, and records the step.
    pub fn compare<T: PartialEq + Display>(
        &mut self,
        claim: &str,
        stated: &T,
        computed: &T,
        code: &'static str,
    ) -> Step<()> {
        if stated == computed {
            self.held(claim, computed);
            Ok(())
        } else {
            let stop =
                Stop::Fail { claim: claim.to_owned(), code, reason: format!("stated {stated}, computed {computed}") };
            Err(self.failed(stop, computed))
        }
    }

    /// [`Run::compare`] under `KN6:evidence.certificate_rejected`.
    pub fn claim<T: PartialEq + Display>(&mut self, claim: &str, stated: &T, computed: &T) -> Step<()> {
        self.compare(claim, stated, computed, codes::CERTIFICATE_REJECTED)
    }

    /// Records a stated input: its role, the SHA-256 of its bytes and the identity read from it.
    pub fn bind(&mut self, role: &str, bytes: &[u8], identity: Vec<(&str, String)>) {
        self.inputs.push(BoundInput {
            role: role.to_owned(),
            sha256: c600_digest::sha256_hex(bytes),
            identity: identity.into_iter().map(|(k, v)| (k.to_owned(), v)).collect(),
        });
    }

    /// The bytes of a required input.
    pub fn input(&self, role: &str) -> Step<&'a [u8]> {
        self.cert
            .input(role)
            .map(|input| input.bytes)
            .ok_or_else(|| reject(role, format!("the input {role} is absent")))
    }

    /// The bytes of an optional input.
    pub fn optional_input(&self, role: &str) -> Option<&'a [u8]> {
        self.cert.input(role).map(|input| input.bytes)
    }

    /// Checks the certificate's own `inputs` binding: an object from role to the SHA-256 of that input's bytes. Every
    /// named role must be supplied with those bytes and no other role may be supplied. A wrong start input fails under
    /// `KN6:evidence.start_table_mismatch`.
    pub fn check_bindings(&mut self, doc: &Value, start_roles: &[&str]) -> Step<()> {
        let named = object(field(doc, "inputs", "inputs")?, "inputs")?;
        for supplied in self.cert.inputs {
            if !named.contains_key(supplied.role) {
                return Err(reject("inputs", format!("the input {} is not named by the certificate", supplied.role)));
            }
        }
        for (role, stated) in named {
            let claim = format!("inputs.{role}");
            let stated = hex64(stated, &claim)?;
            let bytes = self.input(role).map_err(|_| reject(&claim, format!("the input {role} is absent")))?;
            let code = if start_roles.contains(&role.as_str()) {
                codes::START_TABLE_MISMATCH
            } else {
                codes::CERTIFICATE_REJECTED
            };
            self.compare(&claim, &stated.to_owned(), &c600_digest::sha256_hex(bytes), code)?;
        }
        Ok(())
    }
}

/// The certificate as a JSON document.
pub fn document(bytes: &[u8]) -> Step<Value> {
    json::parse(bytes).map_err(|error| reject("certificate", format!("not a JSON document: {error}")))
}

/// The certificate's `format`; a document of another format is outside the kind.
pub fn require_format(doc: &Value, format: &str) -> Step<()> {
    match doc.get("format").and_then(Value::as_str) {
        Some(found) if found == format => Ok(()),
        Some(found) => Err(not_applicable(format!("the certificate's format is {found}, not {format}"))),
        None => Err(not_applicable(format!("the certificate has no format; this kind reads {format}"))),
    }
}

/// An object with exactly the required keys and some of the optional ones.
pub fn keys(value: &Value, claim: &str, required: &[&str], optional: &[&str]) -> Step<()> {
    let map = object(value, claim)?;
    for key in required {
        if !map.contains_key(*key) {
            return Err(reject(claim, format!("the field {key} is missing")));
        }
    }
    for key in map.keys() {
        if !required.contains(&key.as_str()) && !optional.contains(&key.as_str()) {
            return Err(reject(claim, format!("the field {key} is unknown")));
        }
    }
    Ok(())
}

pub fn object<'v>(value: &'v Value, claim: &str) -> Step<&'v BTreeMap<String, Value>> {
    value.as_object().ok_or_else(|| reject(claim, "expected an object"))
}

pub fn field<'v>(value: &'v Value, key: &str, claim: &str) -> Step<&'v Value> {
    object(value, claim)?.get(key).ok_or_else(|| reject(claim, format!("the field {key} is missing")))
}

pub fn array<'v>(value: &'v Value, claim: &str) -> Step<&'v [Value]> {
    value.as_array().ok_or_else(|| reject(claim, "expected a list"))
}

pub fn int(value: &Value, claim: &str) -> Step<i64> {
    value.as_i64().ok_or_else(|| reject(claim, "expected an integer"))
}

/// A nonnegative integer.
pub fn count(value: &Value, claim: &str) -> Step<u64> {
    u64::try_from(int(value, claim)?).map_err(|_| reject(claim, "expected a nonnegative integer"))
}

pub fn string<'v>(value: &'v Value, claim: &str) -> Step<&'v str> {
    value.as_str().ok_or_else(|| reject(claim, "expected a string"))
}

/// A lowercase SHA-256 hex digest.
pub fn hex64<'v>(value: &'v Value, claim: &str) -> Step<&'v str> {
    let text = string(value, claim)?;
    if text.len() == 64 && text.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)) {
        Ok(text)
    } else {
        Err(reject(claim, "expected a lowercase SHA-256 digest"))
    }
}

/// The path of an element of a list claim.
pub fn at(claim: &str, index: usize) -> String {
    format!("{claim}[{index}]")
}

/// The path of a member of an object claim.
pub fn dot(claim: &str, key: &str) -> String {
    if claim.is_empty() {
        key.to_owned()
    } else {
        format!("{claim}.{key}")
    }
}

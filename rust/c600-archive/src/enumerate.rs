//! EVD.48's enumeration rule over one decoded record: every (format, field, value) occurrence, the prose fields and the
//! payload booleans, as the pinned label table's `rule.txt` and `extract_labels.py` define them.
//!
//! - *Format*: the innermost enclosing object's string member `format` when it is not empty; a leaf with none has an
//!   absent format. The label table gave such a leaf a synthetic format made from its path; this module never does,
//!   because no filename supplies a format (PS3.11, FP.22). Absence stays absence.
//! - *Field*: the key path from the record root, `[]` for list items and `<k>` for keys that are numbers, digests or
//!   state names.
//! - *Value*: class S (an enumerated string, or a null under a lexicon key) or class B (a verdict boolean). Prose fields
//!   (class T) are kept word for word and counted; payload booleans (class P) are counted by last key; numbers are not
//!   occurrences.
//!
//! The classes follow the script's regular expressions exactly, written out here because no external crate may be
//! added. Each occurrence keeps its exact JSON pointer (RFC 6901), so the inventory keeps array indices (FP.21).

use c600_kernel::json::Value;

/// EVD.48's classes of a leaf.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Class {
    /// An enumerated string, or a null under a lexicon key.
    S,
    /// A verdict boolean.
    B,
    /// A prose field, kept word for word.
    T,
    /// A payload boolean, counted by last key.
    P,
}

impl Class {
    pub fn text(self) -> &'static str {
        match self {
            Class::S => "S",
            Class::B => "B",
            Class::T => "T",
            Class::P => "P",
        }
    }
}

/// One leaf of a record that EVD.48 enumerates.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Leaf {
    /// The innermost enclosing non-empty string `format`; `None` is an absent format.
    pub format: Option<String>,
    /// The enumeration path.
    pub field: String,
    /// The exact JSON pointer of the leaf.
    pub pointer: String,
    pub value: Value,
    pub class: Class,
}

/// The leaves of a record in document order (object members in key order, as the reader keeps them).
pub fn leaves(record: &Value) -> Vec<Leaf> {
    let mut out = Vec::new();
    let format = match record {
        Value::Object(_) => own_format(record),
        _ => None,
    };
    walk(record, format, String::new(), String::new(), &mut out);
    out
}

/// The enumeration path's last key: the text after the last `.`, with every `[]` removed.
pub fn last_key(field: &str) -> String {
    field.rsplit('.').next().unwrap_or("").replace("[]", "")
}

fn own_format(v: &Value) -> Option<String> {
    match v.get("format") {
        Some(Value::String(s)) if !s.is_empty() => Some(s.clone()),
        _ => None,
    }
}

fn walk(v: &Value, format: Option<String>, path: String, pointer: String, out: &mut Vec<Leaf>) {
    match v {
        Value::Object(fields) => {
            let format = own_format(v).or(format);
            for (k, x) in fields.iter().filter(|(_, x)| !is_number(x)) {
                let part = if is_state_key(k) { "<k>" } else { k.as_str() };
                walk(x, format.clone(), format!("{path}.{part}"), format!("{pointer}/{}", escape(k)), out);
            }
        }
        Value::Array(items) => {
            let child = format!("{path}[]");
            for (n, x) in items.iter().enumerate().filter(|(_, x)| !is_number(x)) {
                walk(x, format.clone(), child.clone(), format!("{pointer}/{n}"), out);
            }
        }
        leaf => {
            let field = path.trim_start_matches('.').to_string();
            if let Some(class) = classify(&field, leaf) {
                out.push(Leaf { format, field, pointer, value: leaf.clone(), class });
            }
        }
    }
}

/// Numbers are never occurrences (EVD.48), so the walk skips them before building any path.
fn is_number(v: &Value) -> bool {
    matches!(v, Value::Int(_) | Value::Number(_))
}

/// RFC 6901 escaping of one reference token.
pub fn escape(token: &str) -> String {
    token.replace('~', "~0").replace('/', "~1")
}

/// The class of a leaf at `field`, or `None` when EVD.48 does not count it.
pub fn classify(field: &str, value: &Value) -> Option<Class> {
    let k = last_key(field);
    match value {
        Value::Bool(_) => Some(if is_verdict_key(&k) { Class::B } else { Class::P }),
        Value::Null => is_lexicon_key(&k).then_some(Class::S),
        Value::String(v) => {
            if is_prose_key(&k) {
                Some(Class::T)
            } else if starts_with_verdict_word(v) || v.contains("not_claimed") || is_lexicon_key(&k) {
                (!is_excluded_value(v) && v.chars().count() <= 80).then_some(Class::S)
            } else {
                None
            }
        }
        _ => None,
    }
}

// --- the script's regular expressions, written out ----------------------------------------------------------------

/// Python's `$` without MULTILINE also matches before one final newline.
fn full(s: &str, pred: impl Fn(&str) -> bool) -> bool {
    pred(s) || s.strip_suffix('\n').is_some_and(&pred)
}

/// Python's `\w` on text: a letter, digit or underscore in Unicode.
fn word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

fn digit(c: char) -> bool {
    c.is_ascii_digit()
}

fn all_digits(s: &str) -> bool {
    !s.is_empty() && s.chars().all(digit)
}

/// `KEY = ^(-?\d+|[0-9a-f]{8,}|[A-Za-z]?\d+(_\d+)*|[ds]\d+(-\w+)+)$`
pub fn is_state_key(k: &str) -> bool {
    full(k, |s| {
        let signed = all_digits(s.strip_prefix('-').unwrap_or(s));
        let hex = s.chars().count() >= 8 && s.chars().all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c));
        let lettered = {
            let rest = match s.chars().next() {
                Some(c) if c.is_ascii_alphabetic() => &s[1..],
                _ => s,
            };
            let mut parts = rest.split('_');
            parts.next().is_some_and(all_digits) && parts.all(all_digits)
        };
        let named = match s.chars().next() {
            Some('d' | 's') => {
                let rest = &s[1..];
                let mut parts = rest.split('-');
                parts.next().is_some_and(all_digits) && {
                    let tail: Vec<&str> = parts.collect();
                    !tail.is_empty() && tail.iter().all(|p| !p.is_empty() && p.chars().all(word_char))
                }
            }
            _ => false,
        };
        signed || hex || lettered || named
    })
}

const LEXICON: [&str; 19] = [
    "verdict",
    "result",
    "outcome",
    "reason",
    "stop",
    "stop_reason",
    "label",
    "kind",
    "classification",
    "severity",
    "all_predicates",
    "profile_join",
    "return",
    "standing",
    "gate",
    "contract",
    "evidence_kind",
    "search",
    "mode",
];

/// `LEX = ^(\w*status|verdict|...|\w*reachability|...|evidence_kind|search|mode)$`
pub fn is_lexicon_key(k: &str) -> bool {
    full(k, |s| {
        LEXICON.contains(&s)
            || ["status", "reachability"]
                .iter()
                .any(|end| s.strip_suffix(end).is_some_and(|head| head.chars().all(word_char)))
    })
}

/// `VAL = ^(PASS|FAIL|pass|fail|UNKNOWN|INVALID)\b|not_claimed`, the first half.
fn starts_with_verdict_word(v: &str) -> bool {
    ["PASS", "FAIL", "pass", "fail", "UNKNOWN", "INVALID"]
        .iter()
        .any(|w| v.strip_prefix(w).is_some_and(|rest| rest.chars().next().is_none_or(|c| !word_char(c))))
}

/// `BAD = ^[0-9a-f]{16,}$|^-?\d+$|^\d+:\d+|[\/]|\.(json|npz|md|gz)$`
pub fn is_excluded_value(v: &str) -> bool {
    let hex = full(v, |s| s.chars().count() >= 16 && s.chars().all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c)));
    let integer = full(v, |s| all_digits(s.strip_prefix('-').unwrap_or(s)));
    let pair = {
        let head = v.chars().take_while(|c| digit(*c)).count();
        head > 0 && v[head..].strip_prefix(':').is_some_and(|rest| rest.chars().next().is_some_and(digit))
    };
    let slash = v.contains('/') || v.contains('\\');
    let file = full(v, |s| [".json", ".npz", ".md", ".gz"].iter().any(|e| s.ends_with(e)));
    hex || integer || pair || slash || file
}

const PROSE: [&str; 10] = [
    "scope",
    "limits",
    "not_claimed",
    "evidence_class",
    "explicitly_not_newly_verified",
    "metric_scope",
    "source_review_scope",
    "full_state_replay_evidence",
    "claim",
    "proof_checks",
];

/// `PRO = ^(scope|limits|...|\w*note|\w*basis|...|proof_checks)$`
pub fn is_prose_key(k: &str) -> bool {
    full(k, |s| {
        PROSE.contains(&s)
            || ["note", "basis"].iter().any(|end| s.strip_suffix(end).is_some_and(|head| head.chars().all(word_char)))
    })
}

const VERDICT_PARTS: [&str; 36] = [
    "replay",
    "fresh",
    "recomputed",
    "rechecked",
    "checked",
    "verified",
    "audit",
    "review",
    "reexecuted",
    "recertified",
    "reused",
    "exhausted",
    "complete",
    "truncated",
    "uncertain",
    "applied",
    "admissible",
    "accepted",
    "legal",
    "claim",
    "established",
    "impossible",
    "hypothes",
    "restored",
    "not_a_",
    "imported",
    "new_search",
    "count_only",
    "branching_used",
    "cached",
    "dirty",
    "implementation",
    "sound",
    "contradiction",
    "prior_proofs",
    "full_tree_info",
];

/// `BL`, searched anywhere in the last key: the parts above, `_pass$` and `^passed$`.
pub fn is_verdict_key(k: &str) -> bool {
    VERDICT_PARTS.iter().any(|p| k.contains(p)) || full(k, |s| s.ends_with("_pass")) || full(k, |s| s == "passed")
}

#[cfg(test)]
mod tests {
    use super::*;
    use c600_kernel::json::parse;

    #[test]
    fn state_keys_follow_the_key_pattern() {
        for k in
            ["7302", "-1", "deadbeef", "0123456789abcdef", "R01", "q11", "s12", "12_3_4", "d1-acc-s7101", "s7302-pstar"]
        {
            assert!(is_state_key(k), "{k}");
        }
        for k in ["status", "R", "deadbee", "abc12", "d-1", "rows", "x1y", "DEADBEEF"] {
            assert!(!is_state_key(k), "{k}");
        }
    }

    #[test]
    fn classes_follow_the_rule() {
        let s = |text: &str| Value::String(text.to_string());
        assert_eq!(classify("rows[].status", &s("budget")), Some(Class::S));
        assert_eq!(classify("verdict", &s("x")), Some(Class::S));
        assert_eq!(classify("note", &s("PASS")), Some(Class::T));
        assert_eq!(classify("notes.claim", &s("anything")), Some(Class::T));
        // `\b` after the verdict word: an underscore or a letter continues the word.
        assert_eq!(classify("anything", &s("PASS; scoped")), Some(Class::S));
        assert_eq!(classify("anything", &s("PASS_SCOPED")), None);
        assert_eq!(classify("status", &s("PASS_SCOPED")), Some(Class::S));
        assert_eq!(classify("anything", &s("PASSING")), None);
        assert_eq!(classify("anything", &s("FAIL: reason")), Some(Class::S));
        assert_eq!(classify("anything", &s("x not_claimed y")), Some(Class::S));
        assert_eq!(classify("status", &s("a/b")), None);
        assert_eq!(classify("status", &s("12:3")), None);
        assert_eq!(classify("status", &s("0123456789abcdef")), None);
        assert_eq!(classify("status", &s("-12")), None);
        assert_eq!(classify("status", &s("x.json")), None);
        assert_eq!(classify("status", &s(&"y".repeat(81))), None);
        assert_eq!(classify("status", &s(&"y".repeat(80))), Some(Class::S));
        assert_eq!(classify("status", &Value::Null), Some(Class::S));
        assert_eq!(classify("value", &Value::Null), None);
        assert_eq!(classify("replay.passed", &Value::Bool(true)), Some(Class::B));
        assert_eq!(classify("uniform_x_guard_pass", &Value::Bool(false)), Some(Class::B));
        assert_eq!(classify("big", &Value::Bool(false)), Some(Class::P));
        assert_eq!(classify("status", &Value::Int(3)), None);
    }

    #[test]
    fn leaves_keep_the_innermost_format_and_exact_pointers() {
        let v = parse(br#"{"kind":"pstar","run":{"format":"pstar-run/1","status":"stopped","rows":[{"status":"budget"}],"7302":{"status":"x"}},"a/b":{"status":"y"}}"#).unwrap();
        let got: Vec<(Option<String>, String, String)> =
            leaves(&v).into_iter().map(|l| (l.format, l.field, l.pointer)).collect();
        assert_eq!(
            got,
            vec![
                (None, "a/b.status".to_string(), "/a~1b/status".to_string()),
                (None, "kind".to_string(), "/kind".to_string()),
                (Some("pstar-run/1".into()), "run.<k>.status".to_string(), "/run/7302/status".to_string()),
                (Some("pstar-run/1".into()), "run.rows[].status".to_string(), "/run/rows/0/status".to_string()),
                (Some("pstar-run/1".into()), "run.status".to_string(), "/run/status".to_string()),
            ]
        );
        // A top-level list: fields start with [].
        let v = parse(br#"[{"status":"ok"}]"#).unwrap();
        assert_eq!(leaves(&v)[0].field, "[].status");
        // An empty format is no format.
        let v = parse(br#"{"format":"","status":"ok"}"#).unwrap();
        assert_eq!(leaves(&v)[0].format, None);
    }
}

//! `PrincipleRecord` (chapter 03 LB6.2) as data, its admission and its staleness.
//!
//! A record is a data file, `docs/progress/1.0/principle-records/<module>.json`, in the draft form
//! `c600-principle-record-draft` version 1. Its parts follow LB6.2's conceptual content: the specification, the
//! justification, the instance with its fixture, the binding to the module and the decomposition it was written
//! against, and the parts a person writes by hand. Every part is either given or a typed none, `{"none": "because ..."}`
//! (LB6.2: a check has no empty answer, a definition has no hypothesis). A part that is absent, or a none without its
//! reason, makes the record incomplete (`LB6:evidence.principle_record_incomplete`).
//!
//! [`admit`] checks a record against the generated decomposition, the module's source and closure digests, the
//! glossary and the repository's bytes:
//!
//! - binding: the source and closure digests are the module's, or the record is bound to other source
//!   (`LB6:evidence.decomposition_mismatch`);
//! - staleness: when the generated decomposition's digest differs from the one the record was written against, every
//!   hand-written part is stale (LB6.2), and the record is not current;
//! - evidence: each evidence reference names a repository file whose SHA-256 is the recorded one;
//! - the fixture: the named test function exists in the named file;
//! - expected claims: each comes from a step of the written derivation, whose result it repeats; none comes from the
//!   module's output;
//! - the step map reports the unmapped nodes and steps on both sides, as the decomposition and the derivation have them;
//! - each opaque kernel leaf the decomposition reaches has a leaf specification naming its covering test;
//! - prerequisites: glossary terms that exist, are learner-facing and sit at or below the record's level, and earlier
//!   records that exist; [`prerequisite_cycle`] checks that the records form a DAG;
//! - the reason's terms sit at or below the record's level (glossary terms, bar M6).

use std::collections::{BTreeMap, BTreeSet};

use crate::code::{DECOMPOSITION_MISMATCH, PRINCIPLE_RECORD_INCOMPLETE};
use crate::decompose::Decomposition;
use crate::json::{self, Json};

/// The record form and its version (draft; no FP.4 row: a record is repository data, not a stored user record).
pub const FORMAT: &str = "c600-principle-record-draft";
pub const VERSION: i64 = 1;

/// A part that is given, or a typed none with its reason.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Part<T> {
    Given(T),
    None(String),
}

impl<T> Part<T> {
    pub fn given(&self) -> Option<&T> {
        match self {
            Part::Given(t) => Some(t),
            Part::None(_) => None,
        }
    }

    pub fn is_none(&self) -> bool {
        matches!(self, Part::None(_))
    }
}

/// One entry's specification (LB6.2's `specification`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Specification {
    pub entry: String,
    pub function: String,
    pub inputs: String,
    pub parameters: Part<String>,
    pub budget: Part<String>,
    pub output: String,
    pub puzzle: String,
    pub menu: Part<String>,
    pub level: String,
    pub domain: String,
    pub tie_rule: Part<String>,
    pub empty_answer: Part<String>,
    pub negative_scope: Part<String>,
    /// The part of the domain outside the cited statement's scope and what the answer means there, or none.
    pub outside_scope: Part<String>,
}

/// The kind of one argument of the justification.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArgumentKind {
    Lemma,
    Certificate,
    Replay,
    ByDefinition,
}

impl ArgumentKind {
    pub fn name(self) -> &'static str {
        match self {
            ArgumentKind::Lemma => "lemma",
            ArgumentKind::Certificate => "certificate",
            ArgumentKind::Replay => "replay",
            ArgumentKind::ByDefinition => "by definition",
        }
    }
}

/// An evidence reference: a repository file with its digest and the grade its record states (chapter 06).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Evidence {
    pub path: String,
    pub sha256: String,
    pub basis: String,
    pub check: String,
    /// The source label the file's record carries (EVD.32 row 7).
    pub label: String,
    /// For an imported result: the producer is absent, and the trace says so (EV5.4).
    pub imported: bool,
}

/// One argument of the justification.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Argument {
    pub kind: ArgumentKind,
    /// A stage-qualified, commit-pinned statement name, or none with the reason.
    pub statement: Part<String>,
    pub text: String,
    pub evidence: Part<Evidence>,
}

/// An expected claim of the instance and the derivation step it comes from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Expected {
    pub claim: String,
    pub value: Json,
    /// The 1-based index of the derivation step whose result it is.
    pub from_step: usize,
}

/// The instance on the real geometry and its headless fixture.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Instance {
    pub model_identity: String,
    pub level: String,
    /// The fixture's input, read by the test that runs it.
    pub input: Json,
    pub expected: Vec<Expected>,
    /// `path::function` of the test that runs the fixture.
    pub fixture: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StepMap {
    /// (decomposition node or leaf label, statement ID, step label).
    pub map: Vec<(String, String, String)>,
    pub unmapped_nodes: Vec<String>,
    pub unmapped_steps: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Reason {
    pub statement: Part<String>,
    pub idea: String,
    /// The glossary terms the idea uses.
    pub terms: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Failure {
    pub hypothesis: String,
    /// A concrete counterexample, a weaker conclusion, or "necessity open".
    pub outcome: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Given {
    pub name: String,
    pub value: Json,
    /// The named leaf call or certificate the given is read from.
    pub leaf: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Step {
    pub check: String,
    pub result: Json,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Prerequisites {
    pub terms: Vec<String>,
    pub records: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LeafSpec {
    pub leaf: String,
    pub spec: String,
    /// `path::function` of the oracle test that covers the leaf.
    pub test: String,
}

/// The parts a person writes by hand (LB6.2's `by hand`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ByHand {
    pub step_map: Part<StepMap>,
    pub reason: Part<Reason>,
    pub failure: Part<Vec<Failure>>,
    pub not_meant: Part<String>,
    pub givens: Part<Vec<Given>>,
    pub steps: Part<Vec<Step>>,
    pub prerequisites: Part<Prerequisites>,
    pub leaf_specs: Part<Vec<LeafSpec>>,
}

/// The hand-written parts by name, in LB6.2's order.
pub const HAND_PARTS: [&str; 8] =
    ["step map", "reason", "failure", "not meant", "givens", "steps", "prerequisites", "leaf specs"];

/// What a record is bound to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Binding {
    pub source_sha256: String,
    pub closure_sha256: String,
    pub model_identity: String,
    /// The combined digest of the entries' decompositions the hand-written parts were written against.
    pub decomposition_sha256: String,
    pub statements: Vec<String>,
}

/// A principle record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PrincipleRecord {
    pub tool: String,
    pub module: String,
    pub entries: Vec<String>,
    /// The record's learning level, `L1` to `L6`.
    pub level: String,
    pub binding: Binding,
    pub specification: Part<Vec<Specification>>,
    pub justification: Part<Vec<Argument>>,
    pub instance: Part<Instance>,
    pub by_hand: ByHand,
    /// The review the record's mathematical content has had or waits for (EV5.1).
    pub review: String,
}

/// One finding of a record's reading or admission, with its code and the part it concerns.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Finding {
    pub code: &'static str,
    pub part: String,
    pub detail: String,
}

fn incomplete(part: &str, detail: impl Into<String>) -> Finding {
    Finding { code: PRINCIPLE_RECORD_INCOMPLETE, part: part.to_string(), detail: detail.into() }
}

// ------------------------------------------------------------------------------------------------- reading

struct Reader {
    findings: Vec<Finding>,
}

impl Reader {
    fn field<'a>(&mut self, obj: &'a Json, key: &str, part: &str) -> Option<&'a Json> {
        let v = obj.get(key);
        if v.is_none() {
            self.findings.push(incomplete(part, format!("the part `{key}` is absent")));
        }
        v
    }

    fn text(&mut self, obj: &Json, key: &str, part: &str) -> String {
        match self.field(obj, key, part) {
            Some(Json::Text(s)) if !s.trim().is_empty() => s.clone(),
            Some(_) => {
                self.findings.push(incomplete(part, format!("`{key}` is not a nonempty text")));
                String::new()
            }
            None => String::new(),
        }
    }

    fn texts(&mut self, obj: &Json, key: &str, part: &str) -> Vec<String> {
        match self.field(obj, key, part) {
            Some(Json::List(items)) => items
                .iter()
                .filter_map(|i| {
                    let s = i.as_str().map(str::to_string);
                    if s.is_none() {
                        self.findings.push(incomplete(part, format!("`{key}` holds a value that is not text")));
                    }
                    s
                })
                .collect(),
            Some(_) => {
                self.findings.push(incomplete(part, format!("`{key}` is not a list")));
                Vec::new()
            }
            None => Vec::new(),
        }
    }

    /// A part that is given or a typed none.
    fn part<T>(&mut self, obj: &Json, key: &str, part: &str, read: impl FnOnce(&mut Reader, &Json) -> T) -> Part<T> {
        let Some(v) = self.field(obj, key, part) else { return Part::None(String::new()) };
        if let Some(Json::Text(reason)) = v.get("none") {
            if v.as_object().map(<[(String, Json)]>::len) == Some(1) {
                if reason.trim().is_empty() {
                    self.findings.push(incomplete(part, format!("`{key}` is a none without its reason")));
                }
                return Part::None(reason.clone());
            }
        }
        if v.get("none").is_some() {
            self.findings.push(incomplete(part, format!("`{key}`: a none is `{{\"none\": \"because ...\"}}`")));
            return Part::None(String::new());
        }
        Part::Given(read(self, v))
    }

    fn text_part(&mut self, obj: &Json, key: &str, part: &str) -> Part<String> {
        self.part(obj, key, part, |r, v| match v {
            Json::Text(s) if !s.trim().is_empty() => s.clone(),
            _ => {
                r.findings.push(incomplete(part, format!("`{key}` is not a nonempty text or a none")));
                String::new()
            }
        })
    }

    fn list<T>(&mut self, v: &Json, part: &str, mut read: impl FnMut(&mut Reader, &Json) -> T) -> Vec<T> {
        match v.as_list() {
            Some(items) if !items.is_empty() => items.iter().map(|i| read(self, i)).collect(),
            Some(_) => {
                self.findings.push(incomplete(part, "an empty list: a part with nothing in it is a typed none"));
                Vec::new()
            }
            None => {
                self.findings.push(incomplete(part, "not a list"));
                Vec::new()
            }
        }
    }
}

fn kind_of(s: &str) -> Option<ArgumentKind> {
    Some(match s {
        "lemma" => ArgumentKind::Lemma,
        "certificate" => ArgumentKind::Certificate,
        "replay" => ArgumentKind::Replay,
        "by_definition" => ArgumentKind::ByDefinition,
        _ => return None,
    })
}

/// Reads a record from its data file's bytes. The findings list every part that is absent or malformed; an empty list
/// means the record is complete as data, which [`admit`] then checks against the module.
pub fn read(bytes: &[u8]) -> (Option<PrincipleRecord>, Vec<Finding>) {
    let doc = match json::parse(bytes) {
        Ok(doc) => doc,
        Err(e) => return (None, vec![incomplete("record", format!("not JSON: {e}"))]),
    };
    let mut r = Reader { findings: Vec::new() };
    if doc.get("format").and_then(Json::as_str) != Some(FORMAT)
        || doc.get("version").and_then(Json::as_i64) != Some(VERSION)
    {
        r.findings.push(incomplete("record", format!("the form is not `{FORMAT}` version {VERSION}")));
        return (None, r.findings);
    }
    let tool = r.text(&doc, "tool", "record");
    let module = r.text(&doc, "module", "record");
    let entries = r.texts(&doc, "entries", "record");
    let level = r.text(&doc, "level", "record");
    let review = r.text(&doc, "review", "record");
    let empty = Json::Object(Vec::new());
    let b = r.field(&doc, "binding", "binding").unwrap_or(&empty).clone();
    let binding = Binding {
        source_sha256: r.text(&b, "source_sha256", "binding"),
        closure_sha256: r.text(&b, "closure_sha256", "binding"),
        model_identity: r.text(&b, "model_identity", "binding"),
        decomposition_sha256: r.text(&b, "decomposition_sha256", "binding"),
        statements: r.texts(&b, "statements", "binding"),
    };
    let specification = r.part(&doc, "specification", "specification", |r, v| {
        r.list(v, "specification", |r, s| Specification {
            entry: r.text(s, "entry", "specification"),
            function: r.text(s, "function", "specification"),
            inputs: r.text(s, "inputs", "specification"),
            parameters: r.text_part(s, "parameters", "specification"),
            budget: r.text_part(s, "budget", "specification"),
            output: r.text(s, "output", "specification"),
            puzzle: r.text(s, "puzzle", "specification"),
            menu: r.text_part(s, "menu", "specification"),
            level: r.text(s, "level", "specification"),
            domain: r.text(s, "domain", "specification"),
            tie_rule: r.text_part(s, "tie_rule", "specification"),
            empty_answer: r.text_part(s, "empty_answer", "specification"),
            negative_scope: r.text_part(s, "negative_scope", "specification"),
            outside_scope: r.text_part(s, "outside_scope", "specification"),
        })
    });
    let justification = r.part(&doc, "justification", "justification", |r, v| {
        r.list(v, "justification", |r, a| {
            let kind_text = r.text(a, "kind", "justification");
            let kind = kind_of(&kind_text).unwrap_or_else(|| {
                r.findings.push(incomplete("justification", format!("unknown argument kind `{kind_text}`")));
                ArgumentKind::ByDefinition
            });
            Argument {
                kind,
                statement: r.text_part(a, "statement", "justification"),
                text: r.text(a, "text", "justification"),
                evidence: r.part(a, "evidence", "justification", |r, e| Evidence {
                    path: r.text(e, "path", "justification"),
                    sha256: r.text(e, "sha256", "justification"),
                    basis: r.text(e, "basis", "justification"),
                    check: r.text(e, "check", "justification"),
                    label: r.text(e, "label", "justification"),
                    imported: e.get("imported") == Some(&Json::Bool(true)),
                }),
            }
        })
    });
    let instance = r.part(&doc, "instance", "instance", |r, i| Instance {
        model_identity: r.text(i, "model_identity", "instance"),
        level: r.text(i, "level", "instance"),
        input: r.field(i, "input", "instance").cloned().unwrap_or(Json::Null),
        expected: match r.field(i, "expected", "instance") {
            Some(v) => r.list(v, "instance", |r, e| Expected {
                claim: r.text(e, "claim", "instance"),
                value: r.field(e, "value", "instance").cloned().unwrap_or(Json::Null),
                from_step: e.get("from_step").and_then(Json::as_i64).unwrap_or(0) as usize,
            }),
            None => Vec::new(),
        },
        fixture: r.text(i, "fixture", "instance"),
    });
    let h = r.field(&doc, "by_hand", "by hand").unwrap_or(&empty).clone();
    let by_hand = ByHand {
        step_map: r.part(&h, "step_map", "step map", |r, v| StepMap {
            map: match r.field(v, "map", "step map") {
                Some(m) => r.list(m, "step map", |r, e| {
                    (r.text(e, "node", "step map"), r.text(e, "statement", "step map"), r.text(e, "step", "step map"))
                }),
                None => Vec::new(),
            },
            unmapped_nodes: v.get("unmapped_nodes").map(|_| r.texts(v, "unmapped_nodes", "step map")).unwrap_or_else(
                || {
                    r.findings.push(incomplete("step map", "`unmapped_nodes` is absent"));
                    Vec::new()
                },
            ),
            unmapped_steps: v.get("unmapped_steps").map(|_| r.texts(v, "unmapped_steps", "step map")).unwrap_or_else(
                || {
                    r.findings.push(incomplete("step map", "`unmapped_steps` is absent"));
                    Vec::new()
                },
            ),
        }),
        reason: r.part(&h, "reason", "reason", |r, v| Reason {
            statement: r.text_part(v, "statement", "reason"),
            idea: r.text(v, "idea", "reason"),
            terms: r.texts(v, "terms", "reason"),
        }),
        failure: r.part(&h, "failure", "failure", |r, v| {
            r.list(v, "failure", |r, f| Failure {
                hypothesis: r.text(f, "hypothesis", "failure"),
                outcome: r.text(f, "outcome", "failure"),
            })
        }),
        not_meant: r.text_part(&h, "not_meant", "not meant"),
        givens: r.part(&h, "givens", "givens", |r, v| {
            r.list(v, "givens", |r, g| Given {
                name: r.text(g, "name", "givens"),
                value: r.field(g, "value", "givens").cloned().unwrap_or(Json::Null),
                leaf: r.text(g, "leaf", "givens"),
            })
        }),
        steps: r.part(&h, "steps", "steps", |r, v| {
            r.list(v, "steps", |r, s| Step {
                check: r.text(s, "check", "steps"),
                result: r.field(s, "result", "steps").cloned().unwrap_or(Json::Null),
            })
        }),
        prerequisites: r.part(&h, "prerequisites", "prerequisites", |r, v| Prerequisites {
            terms: r.texts(v, "terms", "prerequisites"),
            records: r.texts(v, "records", "prerequisites"),
        }),
        leaf_specs: r.part(&h, "leaf_specs", "leaf specs", |r, v| {
            r.list(v, "leaf specs", |r, l| LeafSpec {
                leaf: r.text(l, "leaf", "leaf specs"),
                spec: r.text(l, "spec", "leaf specs"),
                test: r.text(l, "test", "leaf specs"),
            })
        }),
    };
    let record = PrincipleRecord {
        tool,
        module,
        entries,
        level,
        binding,
        specification,
        justification,
        instance,
        by_hand,
        review,
    };
    (Some(record), r.findings)
}

// ------------------------------------------------------------------------------------------------- the glossary

/// The glossary's terms by ID: the level and whether a learner meets the term (EV5.10's flag).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Glossary {
    pub terms: BTreeMap<String, (u8, bool)>,
}

/// A level `L1` to `L6` as its number.
pub fn level_number(level: &str) -> Option<u8> {
    let n: u8 = level.strip_prefix('L')?.parse().ok()?;
    (1..=6).contains(&n).then_some(n)
}

impl Glossary {
    /// Reads `terms.json`: each term's `id`, `level` and `learner_facing` (true when absent).
    pub fn read(bytes: &[u8]) -> Result<Glossary, String> {
        let doc = json::parse(bytes).map_err(|e| e.to_string())?;
        let mut terms = BTreeMap::new();
        for t in doc.get("terms").and_then(Json::as_list).ok_or("no `terms` list")? {
            let id = t.get("id").and_then(Json::as_str).ok_or("a term without an id")?;
            let level = t.get("level").and_then(Json::as_str).and_then(level_number).ok_or("a term without a level")?;
            let facing = t.get("learner_facing") != Some(&Json::Bool(false));
            terms.insert(id.to_string(), (level, facing));
        }
        Ok(Glossary { terms })
    }
}

// ------------------------------------------------------------------------------------------------- admission

/// The module facts a record is admitted against.
#[derive(Clone, Debug)]
pub struct Generated<'a> {
    pub source_sha256: String,
    pub closure_sha256: String,
    /// The decompositions of the record's entries, in the record's order.
    pub decompositions: Vec<&'a Decomposition>,
}

/// The combined digest of a record's decompositions, in order.
pub fn combined_digest(decompositions: &[&Decomposition]) -> String {
    let text: Vec<String> = decompositions.iter().map(|d| d.canonical()).collect();
    c600_digest::sha256_hex(text.join("\n").as_bytes())
}

/// A record's state after admission.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum State {
    /// Complete, bound to the module, and written against the current decomposition.
    Current,
    /// Complete and bound, but the decomposition changed: every hand-written part is stale (LB6.2).
    Stale { parts: Vec<&'static str>, was: String, now: String },
    /// Incomplete or bound to other source: refused, with its findings.
    Refused,
}

/// The result of [`admit`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Admission {
    pub state: State,
    pub findings: Vec<Finding>,
    /// The decomposition's nodes and leaves no step map entry names, and the derivation's steps none names.
    pub unmapped_nodes: Vec<String>,
    pub unmapped_steps: Vec<String>,
}

impl Admission {
    pub fn codes(&self) -> BTreeSet<&'static str> {
        self.findings.iter().map(|f| f.code).collect()
    }
}

/// The node and leaf labels of a set of decompositions: what a step map may name.
pub fn labels(decompositions: &[&Decomposition]) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for d in decompositions {
        out.extend(d.node_labels());
        out.extend(d.kernel_leaves());
        out.extend(d.base_parts());
    }
    out
}

/// Whether `test` (`path::function`) names a function that `bytes` defines.
fn defines(bytes: &[u8], function: &str) -> bool {
    let text = String::from_utf8_lossy(bytes);
    text.contains(&format!("fn {function}("))
}

fn check_test_ref(part: &str, test: &str, resolve: &dyn Fn(&str) -> Option<Vec<u8>>, findings: &mut Vec<Finding>) {
    let Some((path, function)) = test.split_once("::") else {
        findings.push(incomplete(part, format!("`{test}` is not `path::function`")));
        return;
    };
    match resolve(path) {
        Some(bytes) if defines(&bytes, function) => {}
        Some(_) => findings.push(incomplete(part, format!("`{path}` defines no `{function}`"))),
        None => findings.push(incomplete(part, format!("`{path}` does not exist"))),
    }
}

/// Admits `record` against the module's generated facts, the glossary and the repository's bytes (`resolve` reads a
/// repository-relative path). Findings carry their chapter 13 codes; the state says whether the record is current.
pub fn admit(
    record: &PrincipleRecord,
    generated: &Generated,
    glossary: &Glossary,
    known_records: &BTreeSet<String>,
    resolve: &dyn Fn(&str) -> Option<Vec<u8>>,
) -> Admission {
    let mut findings = Vec::new();
    let b = &record.binding;
    if b.source_sha256 != generated.source_sha256 {
        findings.push(Finding {
            code: DECOMPOSITION_MISMATCH,
            part: "binding".into(),
            detail: format!(
                "the record is bound to source {}, the module is {}",
                b.source_sha256, generated.source_sha256
            ),
        });
    }
    if b.closure_sha256 != generated.closure_sha256 {
        findings.push(Finding {
            code: DECOMPOSITION_MISMATCH,
            part: "binding".into(),
            detail: format!(
                "the record is bound to closure {}, the module's is {}",
                b.closure_sha256, generated.closure_sha256
            ),
        });
    }
    if b.statements.is_empty() {
        findings.push(incomplete("binding", "the record names no statement"));
    }
    let record_level = level_number(&record.level);
    if record_level.is_none() {
        findings.push(incomplete("record", format!("`{}` is no learning level L1 to L6", record.level)));
    }

    // The specification covers every entry, and each names what it does outside its cited scope.
    match &record.specification {
        Part::Given(specs) => {
            let named: BTreeSet<&str> = specs.iter().map(|s| s.entry.as_str()).collect();
            for e in &record.entries {
                if !named.contains(e.as_str()) {
                    findings.push(incomplete("specification", format!("no specification of the entry `{e}`")));
                }
            }
        }
        Part::None(_) => findings.push(incomplete("specification", "a tool has a specification; it cannot be none")),
    }

    // Evidence references resolve to their recorded bytes.
    if let Part::Given(args) = &record.justification {
        for a in args {
            if let Part::Given(e) = &a.evidence {
                match resolve(&e.path) {
                    Some(bytes) if c600_digest::sha256_hex(&bytes) == e.sha256 => {}
                    Some(_) => {
                        findings.push(incomplete("justification", format!("`{}` is not the recorded bytes", e.path)))
                    }
                    None => findings.push(incomplete("justification", format!("`{}` does not exist", e.path))),
                }
                if e.check == "formal_proof" {
                    findings.push(incomplete("justification", "check formal proof has no writer in 1.0 (EVD.64)"));
                }
            }
        }
    }

    // The steps, and the expected claims that come from them.
    let steps: Vec<Step> = record.by_hand.steps.given().cloned().unwrap_or_default();
    match &record.instance {
        Part::Given(i) => {
            check_test_ref("instance", &i.fixture, resolve, &mut findings);
            if i.model_identity.is_empty() || i.level.is_empty() {
                findings.push(incomplete("instance", "the fixture names its model identity and level"));
            }
            for e in &i.expected {
                match steps.get(e.from_step.wrapping_sub(1)) {
                    Some(step) if step.result == e.value => {}
                    Some(step) => findings.push(incomplete(
                        "instance",
                        format!(
                            "the claim `{}` is {} but step {} of the derivation gives {}",
                            e.claim,
                            e.value.compact(),
                            e.from_step,
                            step.result.compact()
                        ),
                    )),
                    None => findings.push(incomplete(
                        "instance",
                        format!("the claim `{}` names no step of the written derivation", e.claim),
                    )),
                }
            }
        }
        Part::None(_) => {
            findings.push(incomplete("instance", "every built-in tool has a real-geometry instance (LB6.1)"))
        }
    }

    // The hand-written parts that read the decomposition are checked against it only when they were written against
    // it. When it changed, they are stale (LB6.2), not wrong: the step map and the leaf specifications wait for the
    // person who refreshes them, and the state says so.
    let decompositions = &generated.decompositions;
    let now = combined_digest(decompositions);
    let stale = b.decomposition_sha256 != now;

    // Leaf specifications for every opaque kernel leaf.
    let kernel_leaves: BTreeSet<String> = decompositions.iter().flat_map(|d| d.kernel_leaves()).collect();
    match &record.by_hand.leaf_specs {
        _ if stale => {}
        Part::Given(specs) => {
            let specified: BTreeSet<&str> = specs.iter().map(|s| s.leaf.as_str()).collect();
            for leaf in &kernel_leaves {
                if !specified.contains(leaf.as_str()) {
                    findings.push(incomplete("leaf specs", format!("the kernel leaf `{leaf}` has no specification")));
                }
            }
            for s in specs {
                check_test_ref("leaf specs", &s.test, resolve, &mut findings);
            }
        }
        Part::None(_) if !kernel_leaves.is_empty() => findings.push(incomplete(
            "leaf specs",
            format!("the decomposition reaches kernel leaves {kernel_leaves:?}; each needs a specification"),
        )),
        Part::None(_) => {}
    }

    // The step map's unmapped items on both sides.
    let known_labels = labels(decompositions);
    let step_labels: BTreeSet<String> = (1..=steps.len()).map(|n| format!("step {n}")).collect();
    let (mut unmapped_nodes, mut unmapped_steps) = (Vec::new(), Vec::new());
    if let (Part::Given(map), false) = (&record.by_hand.step_map, stale) {
        let mapped_nodes: BTreeSet<&str> = map.map.iter().map(|(n, _, _)| n.as_str()).collect();
        let mapped_steps: BTreeSet<&str> = map.map.iter().map(|(_, _, s)| s.as_str()).collect();
        for (node, _, step) in &map.map {
            if !known_labels.contains(node) {
                findings.push(incomplete("step map", format!("`{node}` is no node or leaf of the decomposition")));
            }
            if !step_labels.contains(step) && !step.starts_with("lemma ") {
                findings.push(incomplete("step map", format!("`{step}` is no step of the derivation")));
            }
        }
        unmapped_nodes = known_labels.iter().filter(|l| !mapped_nodes.contains(l.as_str())).cloned().collect();
        unmapped_steps = step_labels.iter().filter(|s| !mapped_steps.contains(s.as_str())).cloned().collect();
        if map.unmapped_nodes != unmapped_nodes || map.unmapped_steps != unmapped_steps {
            findings.push(incomplete(
                "step map",
                format!(
                    "the record reports unmapped nodes {:?} and steps {:?}; the decomposition and the derivation leave \
                     {:?} and {:?}",
                    map.unmapped_nodes, map.unmapped_steps, unmapped_nodes, unmapped_steps
                ),
            ));
        }
    }

    // Prerequisites and the reason's terms against the glossary and the levels.
    let at_or_below = |part: &str, term: &str, findings: &mut Vec<Finding>| match glossary.terms.get(term) {
        None => findings.push(incomplete(part, format!("`{term}` is no glossary term"))),
        Some((_, false)) => findings.push(incomplete(part, format!("`{term}` is not learner-facing"))),
        Some((level, _)) if record_level.is_some_and(|r| *level > r) => findings
            .push(incomplete(part, format!("`{term}` sits at L{level}, above the record's level {}", record.level))),
        Some(_) => {}
    };
    if let Part::Given(p) = &record.by_hand.prerequisites {
        for t in &p.terms {
            at_or_below("prerequisites", t, &mut findings);
        }
        for r in &p.records {
            if !known_records.contains(r) {
                findings.push(incomplete("prerequisites", format!("no principle record of `{r}`")));
            }
        }
    }
    if let Part::Given(reason) = &record.by_hand.reason {
        for t in &reason.terms {
            at_or_below("reason", t, &mut findings);
        }
    }

    let state = if !findings.is_empty() {
        State::Refused
    } else if stale {
        State::Stale { parts: HAND_PARTS.to_vec(), was: b.decomposition_sha256.clone(), now }
    } else {
        State::Current
    };
    Admission { state, findings, unmapped_nodes, unmapped_steps }
}

/// The first cycle among the records' prerequisite records, if any: records form a DAG (LB6.2).
pub fn prerequisite_cycle(records: &[&PrincipleRecord]) -> Option<Vec<String>> {
    let edges: BTreeMap<&str, Vec<&str>> = records
        .iter()
        .map(|r| {
            let deps = r.by_hand.prerequisites.given().map(|p| p.records.iter().map(String::as_str).collect());
            (r.module.as_str(), deps.unwrap_or_default())
        })
        .collect();
    fn visit<'a>(
        n: &'a str,
        edges: &BTreeMap<&'a str, Vec<&'a str>>,
        path: &mut Vec<&'a str>,
        done: &mut BTreeSet<&'a str>,
    ) -> Option<Vec<String>> {
        if let Some(at) = path.iter().position(|p| *p == n) {
            return Some(path[at..].iter().map(|s| s.to_string()).chain([n.to_string()]).collect());
        }
        if done.contains(n) {
            return None;
        }
        path.push(n);
        for m in edges.get(n).into_iter().flatten() {
            if let Some(cycle) = visit(m, edges, path, done) {
                return Some(cycle);
            }
        }
        path.pop();
        done.insert(n);
        None
    }
    let mut done = BTreeSet::new();
    for n in edges.keys() {
        if let Some(cycle) = visit(n, &edges, &mut Vec::new(), &mut done) {
            return Some(cycle);
        }
    }
    None
}

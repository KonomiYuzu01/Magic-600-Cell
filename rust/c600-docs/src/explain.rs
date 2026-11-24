//! A tool's explanation (EV5.3 to EV5.5, EV5.13).
//!
//! An explanation consumes a [`PrincipleRecord`]: the specification, each argument of the justification with its
//! claim's word, the instance with its fixture's outcome, the decomposition as lazy links, and the parts a person
//! writes. The words come from chapter 06's word function (`c600_evidence::word`) over a graph of the stored facts, at
//! a pinned edition (EV5.13). Each claim keeps its statement, scope, bases, checks and job outcome apart, its source
//! labels and check references stay visible, and an explanation says so where no human review or formal proof is
//! recorded. A record's evidence enters the graph through chapter 06's `record` sink with the host as writer, so
//! check `formal proof`, which has no writer in 1.0, is refused (EVD.64).
//!
//! The word of a cited statement is that statement's word only: no field here carries a word for the shown answer.
//! Author class, evidence and permission stay apart (INV-5, BM2): the author class is shown as data, no word reads it,
//! and an explanation grants nothing. A fork inherits no evidence (EV5.5, default 18): its explanation keeps the
//! parent's statements as references and records no leaf of the parent's. An explanation supplies the means of
//! understanding; it cannot certify understanding (DT-L1 R3-M4, BND.72).

use c600_evidence::{
    record as record_leaf, word, Basis, Check, Citation, Covered, Covers, DigestRef, DigestRole, Edition, Form, Graph,
    JobOutcome, Key, Leaf, ProducingJob, Scope, Statement, StatementId, Word, Writer,
};

use crate::decompose::{Decomposition, Edge};
use crate::json::Json;
use crate::record::{Argument, ArgumentKind, ByHand, Evidence, Part, PrincipleRecord, Specification, State};

/// The bounds of a computed argument's claim: the inputs its evidence file records, and nothing outside them.
pub const RECORDED: &str = "the recorded inputs";

/// The scope contract the explanation's keys name (draft).
pub const CONTRACT: &str = "c600-docs-explanation-draft-1";

/// What EV5.3 says an explanation is, shown with every explanation.
pub const MEANS_NOT_CERTIFICATE: &str =
    "This explanation supplies the means of understanding the tool. It cannot certify that anyone understands it.";

/// The note shown beside a word when no recorded check is a human review or a formal proof (EV5.13).
pub const NO_REVIEW: &str = "No human review or formal proof is recorded for this claim.";

/// One claim as chapter 06 reads it, its fields kept apart (EV5.13).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClaimView {
    /// The statement's ID as text.
    pub statement: String,
    /// The scope phrase (EVD.36).
    pub scope: String,
    /// The bases of the claim's live leaves, by their wire names.
    pub bases: Vec<String>,
    /// The checks of the claim's live leaves, shown beside the word.
    pub checks: Vec<String>,
    /// The producing job's outcome, when a leaf names one.
    pub job_outcome: Option<String>,
    /// Chapter 06's word for the statement and its scope.
    pub word: Word,
    pub edition: String,
    /// The source labels of the claim's records.
    pub source_labels: Vec<String>,
    /// The check references: the digests of the claim's supporting records.
    pub check_references: Vec<String>,
    /// [`NO_REVIEW`] when no live leaf's check is a human review or a formal proof.
    pub review_note: Option<&'static str>,
    /// A write the record sink refused, with its code: such evidence is not in the graph.
    pub refused: Option<String>,
}

/// One argument of the justification with its claim.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArgumentView {
    pub kind: ArgumentKind,
    pub text: String,
    pub evidence: Part<Evidence>,
    pub claim: ClaimView,
    /// The argument's provenance link: a lemma, an imported result or nothing further.
    pub link: Option<Link>,
}

/// What the fixture did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FixtureOutcome {
    /// The fixture ran to completion and reproduced the expected claims.
    Passed,
    /// The fixture ran and did not reproduce them.
    Failed,
    /// The fixture did not run.
    NotRun,
}

/// A tool's explanation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Explanation {
    pub tool: String,
    pub module: String,
    /// The author class, shown as data; no word reads it.
    pub author_class: String,
    /// Whether this is a fork's explanation, which inherits no evidence.
    pub fork_of: Option<String>,
    pub specification: Part<Vec<Specification>>,
    pub arguments: Vec<ArgumentView>,
    /// The instance's claim: what the fixture shows at that input, nothing wider (EV5.4).
    pub instance: Option<ClaimView>,
    /// The decomposition's roots, collapsed until expanded.
    pub decomposition: Vec<Link>,
    pub by_hand: ByHand,
    /// The hand-written parts that are stale, when the decomposition changed.
    pub stale: Vec<&'static str>,
    /// What the explanation is.
    pub note: &'static str,
    /// Permission is decided at invocation, never by an explanation.
    pub permission: &'static str,
}

/// A provenance link, expanded on request (EV5.4).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Link {
    /// A function of the decomposition, collapsed: its label, source digest and the number of its edges.
    Function { label: String, source_sha256: String, edges: usize },
    /// An opaque kernel leaf.
    Kernel { label: String },
    /// A base part.
    Base { label: String },
    /// A stage-qualified, source-commit-pinned lemma.
    Lemma { statement: String },
    /// An imported result: its stored bytes and source label; its producer is absent.
    Imported { path: String, bytes_sha256: String, source_label: String, producer: &'static str },
    /// A word, as its literal digest and length until expanded.
    Word { digest: String, length: usize },
    /// A call the checker could not pin: "may call".
    MayCall { label: String },
}

/// The literal digest of a word: the SHA-256 of its letters as little-endian 32-bit integers (KI.13).
pub fn word_digest(letters: &[i32]) -> String {
    let bytes: Vec<u8> = letters.iter().flat_map(|l| l.to_le_bytes()).collect();
    c600_digest::sha256_hex(&bytes)
}

/// A word's link: digest and length, no letters.
pub fn word_link(letters: &[i32]) -> Link {
    Link::Word { digest: word_digest(letters), length: letters.len() }
}

/// The letters of a word link at positions `start..start + count`, read from the word's stored letters, which must
/// have the link's digest: expansion never reads other bytes.
pub fn expand_word(link: &Link, letters: &[i32], start: usize, count: usize) -> Option<Vec<i32>> {
    let Link::Word { digest, length } = link else { return None };
    if *digest != word_digest(letters) || *length != letters.len() || start.checked_add(count)? > letters.len() {
        return None;
    }
    Some(letters[start..start + count].to_vec())
}

/// The roots of the decompositions: one collapsed link per entry.
pub fn roots(decompositions: &[&Decomposition]) -> Vec<Link> {
    decompositions
        .iter()
        .filter_map(|d| d.node(&format!("{}.{}", d.module, d.entry)))
        .map(|n| Link::Function { label: n.label(), source_sha256: n.source_sha256.clone(), edges: n.edges.len() })
        .collect()
}

/// Expands one function link: its calls (collapsed), "may call" edges, kernel leaves and base parts. `None` when the
/// label names no node.
pub fn expand(decomposition: &Decomposition, label: &str) -> Option<Vec<Link>> {
    let node = decomposition.node(label)?;
    Some(
        node.edges
            .iter()
            .map(|e| match e {
                Edge::Call { module, function } => {
                    let label = format!("{module}.{function}");
                    match decomposition.node(&label) {
                        Some(n) => {
                            Link::Function { label, source_sha256: n.source_sha256.clone(), edges: n.edges.len() }
                        }
                        None => Link::MayCall { label },
                    }
                }
                Edge::MayCall { module, function } => Link::MayCall { label: format!("{module}.{function}") },
                Edge::Unresolved { name } => Link::MayCall { label: format!("unresolved {name}") },
                Edge::Kernel { .. } => Link::Kernel { label: e.leaf_label().unwrap_or_default() },
                Edge::Base { .. } => Link::Base { label: e.leaf_label().unwrap_or_default() },
            })
            .collect(),
    )
}

/// Reads `stage@commit:name` as a stage-qualified, commit-pinned statement; any other text is a typed statement.
pub fn statement_id(text: &str) -> StatementId {
    if let Some((stage, rest)) = text.split_once('@') {
        if let Some((commit, name)) = rest.split_once(':') {
            let hex = |s: &str| (s.len() == 40 || s.len() == 64) && s.bytes().all(|b| b.is_ascii_hexdigit());
            if hex(commit) && !stage.is_empty() && !name.is_empty() {
                return StatementId::named(stage, commit, name);
            }
        }
    }
    StatementId::typed(text)
}

fn statement_text(id: &StatementId) -> String {
    match id {
        StatementId::Named { stage, commit, name } => format!("{stage}@{commit}:{name}"),
        StatementId::Typed { content } => content.clone(),
    }
}

fn view(g: &Graph, key: &Key, edition: &Edition, refused: Option<String>) -> ClaimView {
    let word = match word(g, key, edition) {
        Ok(w) => w,
        Err(e) => panic!("a record's graph is consistent by construction: {e:?}"),
    };
    let leaves: Vec<&Leaf> = g
        .root_of(key)
        .map(|root| g.arguments(root).into_iter().flatten().filter_map(|n| g.leaf(n)).filter(|l| !l.failed).collect())
        .unwrap_or_default();
    let mut bases: Vec<String> = leaves.iter().map(|l| l.grade.basis.wire().to_string()).collect();
    let mut checks: Vec<String> = leaves.iter().map(|l| l.grade.check.wire().to_string()).collect();
    bases.sort();
    bases.dedup();
    checks.sort();
    checks.dedup();
    let reviewed = leaves.iter().any(|l| matches!(l.grade.check, Check::HumanReview | Check::FormalProof));
    let job_outcome = leaves.iter().find_map(|l| l.producing_job.as_ref()).map(|j| format!("{:?}", j.outcome));
    let mut labels: Vec<String> =
        g.statement(&key.statement).and_then(|s| s.source_label.clone()).into_iter().collect();
    labels.extend(word.source_label.clone());
    labels.sort();
    labels.dedup();
    let mut references: Vec<String> = leaves.iter().flat_map(|l| l.support.iter().cloned()).collect();
    references.sort();
    references.dedup();
    ClaimView {
        statement: statement_text(&key.statement),
        scope: word.scope_phrase.clone(),
        bases,
        checks,
        job_outcome,
        word,
        edition: edition.name().to_string(),
        source_labels: labels,
        check_references: references,
        review_note: (!reviewed).then_some(NO_REVIEW),
        refused,
    }
}

fn domain_of(record: &PrincipleRecord) -> String {
    match &record.specification {
        Part::Given(specs) => specs.iter().map(|s| s.domain.clone()).collect::<Vec<_>>().join("; "),
        Part::None(r) => r.clone(),
    }
}

/// Whether evidence of this basis is a computation or a certificate over finite recorded inputs (chapter 06's computed
/// bases): its claim then covers the recorded set only, never the specification's whole domain.
pub fn computed_basis(basis: &str) -> bool {
    matches!(basis, "certificate" | "exact_computation" | "exhaustive_computation")
}

fn computed(a: &Argument) -> bool {
    a.evidence.given().is_some_and(|e| computed_basis(&e.basis))
}

/// The key of an argument's claim. A lemma or a definition is asserted over the specification's domain; a computation
/// covers the recorded set its evidence names, and nothing outside it (EVD.33, I7).
fn argument_key(record: &PrincipleRecord, a: &Argument, index: usize) -> Key {
    let id = match &a.statement {
        Part::Given(s) => statement_id(s),
        Part::None(_) => StatementId::typed(&format!("{} argument {}: {}", record.module, index + 1, a.text)),
    };
    let covers = match a.evidence.given() {
        Some(e) if computed(a) => Covers::search_set(&e.path, "", RECORDED),
        _ => Covers::admitted_domain(&domain_of(record), ""),
    };
    let scope = Scope::new(covers, &record.binding.model_identity, CONTRACT, None);
    Key::new(id, scope)
}

fn add_statement(g: &mut Graph, key: &Key, form: Form, label: Option<&str>, references: &[Key]) {
    let mut s = Statement::new(key.statement.clone()).form(form);
    if let Some(label) = label {
        s = s.label(label);
    }
    for r in references {
        s = s.reference(r);
    }
    let _ = g.add_statement(s);
}

/// Enters an argument's evidence through the record sink: the host writes, as for an import of records on file.
fn enter(g: &mut Graph, key: &Key, e: &Evidence) -> Option<String> {
    let grade = c600_evidence::Grade::new(Basis::from_wire(&e.basis), Check::from_wire(&e.check));
    let mut leaf = Leaf::new(key, grade.clone()).support(&e.sha256);
    if computed_basis(&e.basis) {
        leaf = leaf.covered(Covered::of_scope(&key.scope));
    }
    let commit = match &key.statement {
        StatementId::Named { commit, .. } => commit.clone(),
        StatementId::Typed { .. } => String::new(),
    };
    let citation = Citation::theorem(&e.path, &commit).with_review(&e.sha256);
    let writer = if e.imported { Writer::Import } else { Writer::Host };
    record_leaf(g, writer, key, leaf, &citation).err().map(|err| format!("{err:?}"))
}

/// The key of the instance's claim: one finite instance, named by the digest of its input.
pub fn instance_key(record: &PrincipleRecord, input: &Json) -> Key {
    let digest = c600_digest::sha256_hex(input.compact().as_bytes());
    let id = StatementId::typed(&format!("{} reproduces its expected claims at input {digest}", record.module));
    let scope = Scope::new(
        Covers::OneInstance,
        &record.binding.model_identity,
        CONTRACT,
        Some(DigestRef::new(DigestRole::Content, &digest)),
    );
    Key::new(id, scope)
}

/// Builds the explanation of `record` with the fixture's outcome, at a pinned evidence edition. `fork_of` names the
/// parent module when this is a fork's explanation: the parent's statements become references and none of its
/// evidence is recorded, and the fork's own fixture outcome alone stands for its instance.
pub fn explain(
    record: &PrincipleRecord,
    state: &State,
    decompositions: &[&Decomposition],
    fixture: FixtureOutcome,
    author_class: &str,
    fork_of: Option<&str>,
    edition: &Edition,
) -> Explanation {
    let mut g = Graph::new();
    let mut arguments = Vec::new();
    if let Part::Given(args) = &record.justification {
        for (i, a) in args.iter().enumerate() {
            let key = argument_key(record, a, i);
            let label = a.evidence.given().map(|e| e.label.as_str());
            // A finite statement covers one instance (EVD I7): an argument's claim is universal, over the recorded
            // inputs when its evidence is computed and over the specification's domain otherwise.
            let form = Form::Universal;
            let mut refused = None;
            if let Some(parent) = fork_of {
                // The fork's statement keeps the parent's as a reference, never as an edge (LB6.3).
                let parent_key =
                    Key::new(key.statement.clone(), Scope::new(key.scope.covers.clone(), parent, CONTRACT, None));
                add_statement(&mut g, &key, form, label, &[parent_key]);
            } else {
                add_statement(&mut g, &key, form, label, &[]);
                if let Part::Given(e) = &a.evidence {
                    refused = enter(&mut g, &key, e);
                }
            }
            let link = match (&a.evidence, &key.statement) {
                (Part::Given(e), _) if e.imported => Some(Link::Imported {
                    path: e.path.clone(),
                    bytes_sha256: e.sha256.clone(),
                    source_label: e.label.clone(),
                    producer: "absent",
                }),
                (_, StatementId::Named { .. }) => Some(Link::Lemma { statement: statement_text(&key.statement) }),
                _ => None,
            };
            let claim = view(&g, &key, edition, refused);
            arguments.push(ArgumentView {
                kind: a.kind,
                text: a.text.clone(),
                evidence: a.evidence.clone(),
                claim,
                link,
            });
        }
    }
    let instance = record.instance.given().map(|i| {
        let key = instance_key(record, &i.input);
        add_statement(&mut g, &key, Form::Finite, None, &[]);
        if fixture != FixtureOutcome::NotRun {
            let covered = Covered::new(Covers::OneInstance, key.scope.input.clone());
            let mut leaf = Leaf::new(&key, c600_evidence::Grade::new(Basis::ExactComputation, Check::Producer))
                .covered(covered)
                .support(&i.fixture)
                .job(ProducingJob::new(&i.fixture, JobOutcome::Completed));
            leaf.failed = fixture == FixtureOutcome::Failed;
            if let Err(e) = c600_evidence::offer(&mut g, &key, leaf) {
                panic!("an instance leaf is offered in its own scope: {e:?}");
            }
        }
        view(&g, &key, edition, None)
    });
    let stale = match state {
        State::Stale { parts, .. } => parts.clone(),
        _ => Vec::new(),
    };
    Explanation {
        tool: record.tool.clone(),
        module: record.module.clone(),
        author_class: author_class.to_string(),
        fork_of: fork_of.map(str::to_string),
        specification: record.specification.clone(),
        arguments,
        instance,
        decomposition: roots(decompositions),
        by_hand: record.by_hand.clone(),
        stale,
        note: MEANS_NOT_CERTIFICATE,
        permission: "decided at invocation; an explanation grants nothing",
    }
}

/// The explanation as text: every claim with its word, its checks beside it, and its note.
pub fn render(e: &Explanation) -> String {
    let mut out = format!("# {} (`{}`)\n\n{}\n\n- Author class: {}\n", e.tool, e.module, e.note, e.author_class);
    if let Some(parent) = &e.fork_of {
        out.push_str(&format!(
            "- Fork of `{parent}`: no evidence is inherited, and the parent's checks certify nothing here\n"
        ));
    }
    out.push_str(&format!("- Permission: {}\n", e.permission));
    if !e.stale.is_empty() {
        out.push_str(&format!("- Stale: {} (the decomposition changed)\n", e.stale.join(", ")));
    }
    out.push_str("\n## Justification\n");
    for a in &e.arguments {
        out.push_str(&format!("\n- {}: {}\n", a.kind.name(), a.text));
        out.push_str(&claim_lines(&a.claim));
    }
    if let Some(i) = &e.instance {
        out.push_str("\n## Instance\n\n");
        out.push_str(&claim_lines(i));
    }
    out
}

fn claim_lines(c: &ClaimView) -> String {
    let mut out = format!("  - Statement: `{}`\n", c.statement);
    out.push_str(&format!("  - Word ({}): {}; scope: {}\n", c.edition, c.word.display(), c.scope));
    out.push_str(&format!("  - Basis: [{}]; check: [{}]\n", c.bases.join(", "), c.checks.join(", ")));
    if let Some(job) = &c.job_outcome {
        out.push_str(&format!("  - Job outcome: {job}\n"));
    }
    if !c.source_labels.is_empty() {
        out.push_str(&format!("  - Source labels: {}\n", c.source_labels.join(", ")));
    }
    if let Some(note) = c.review_note {
        out.push_str(&format!("  - {note}\n"));
    }
    if let Some(r) = &c.refused {
        out.push_str(&format!("  - Refused evidence: {r}\n"));
    }
    out
}

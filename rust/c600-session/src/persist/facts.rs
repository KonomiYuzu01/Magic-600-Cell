//! FP.19 and FP.20's facts envelope (PS1.35 to PS1.37, PS2.6): the archival admission of an evidence payload, its
//! reading into `c600-evidence`'s graph, and the display word per recorded function edition.
//!
//! Admission checks structure, node and scope closure, the producing job's captured source and immutable scope. It
//! never checks a claim and keeps no second evidence order (FP.20): mutual negation, a leaf's identity against its
//! scope and coverage are chapter 06's I2 and I7, which [`derive`] reads through `c600-evidence`'s word function. An
//! enumeration value outside edition 1's list is kept as read and reads I1 there (EVD.40); a structural fault is
//! `PS1:input.persistence_format`. A `recorded` payload supplies no admitted support: its roots read without children.

use std::collections::{BTreeMap, BTreeSet};

use c600_evidence::model::{
    Basis, Check, CheckFact, CheckMode, Complete, Covered, Covers, Declared, DigestRef, DigestRole, Form, Grade, Guard,
    JobOutcome, Key, Leaf, Level, Membership, PremiseClass, ProducingJob, Quantifier, Scope, SearchRecord, Statement,
    StatementId, Stop,
};
use c600_evidence::{word, Edition, EvidenceError, Graph, Word};

use super::codec::{canonical, Json};
use super::codes::{FORMAT, SCOPE};
use super::record::has_exactly;
use super::{require, Refusal};

/// FP.19's payload fields.
pub const PAYLOAD: [&str; 10] =
    ["form", "status", "statements", "scopes", "nodes", "roots", "sources", "editions", "caches", "missing"];
/// FP.19's function editions.
pub const EDITIONS: [&str; 6] = ["order", "summary", "display", "invalid_pairs", "threshold", "import"];
const SCOPE_FIELDS: [&str; 6] = ["id", "covers", "identity", "contract", "input", "sources"];
/// Revision-bound context that never enters a stored scope (PS1.35, EVD.10). Draft list.
pub const TRANSIENT: [&str; 6] = ["revision", "epoch", "role", "goal", "protection", "orbit"];
const IDENTITY_FIELDS: [&str; 5] = ["puzzle", "model_id", "menu_id", "menu_order", "contract_rev"];
const LEAF: [&str; 12] = [
    "id",
    "kind",
    "key",
    "grade",
    "identity",
    "covered",
    "record",
    "checks",
    "membership",
    "search",
    "producing_job",
    "reachability",
];
const ENGINEERING: [&str; 5] = ["source", "synthetic", "windows", "performance", "human"];

fn bad() -> Refusal {
    Refusal::new(FORMAT)
}

fn field<'a>(value: &'a Json, name: &str) -> Result<&'a Json, Refusal> {
    value.get(name).ok_or_else(bad)
}

fn text<'a>(value: &'a Json, name: &str) -> Result<&'a str, Refusal> {
    field(value, name)?.as_text().ok_or_else(bad)
}

fn opt_text<'a>(value: &'a Json, name: &str) -> Result<Option<&'a str>, Refusal> {
    match field(value, name)? {
        Json::Null => Ok(None),
        Json::Text(s) => Ok(Some(s)),
        _ => Err(bad()),
    }
}

fn items<'a>(value: &'a Json, name: &str) -> Result<&'a [Json], Refusal> {
    match field(value, name)? {
        Json::List(items) => Ok(items),
        _ => Err(bad()),
    }
}

fn texts(value: &Json, name: &str) -> Result<Vec<String>, Refusal> {
    items(value, name)?.iter().map(|t| t.as_text().map(str::to_string).ok_or_else(bad)).collect()
}

/// The compact FP.5 text of a value, which the evidence model compares by equality.
fn value_text(value: &Json) -> String {
    let mut bytes = canonical(value);
    bytes.pop();
    String::from_utf8(bytes).expect("FP.5 bytes are UTF-8")
}

fn is_hex64(s: &str) -> bool {
    s.len() == 64 && s.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

fn digest(value: &Json) -> Result<DigestRef, Refusal> {
    require(has_exactly(value, &["role", "sha256"]), FORMAT)?;
    let sha = text(value, "sha256")?;
    require(is_hex64(sha), FORMAT)?;
    let role = match text(value, "role")? {
        "poses" => DigestRole::Poses,
        "labels" => DigestRole::Labels,
        "sticker_class" => DigestRole::StickerClass,
        "bytes" => DigestRole::Bytes,
        "content" => DigestRole::Content,
        "source" => DigestRole::Source,
        other => DigestRole::Other(other.to_string()),
    };
    Ok(DigestRef::new(role, sha))
}

fn digest_of_role(value: &Json, role: &str) -> Result<String, Refusal> {
    require(value.get("role").and_then(Json::as_text) == Some(role), FORMAT)?;
    Ok(digest(value)?.sha256)
}

/// The wire name of a digest role.
pub fn role_name(role: &DigestRole) -> &str {
    match role {
        DigestRole::Poses => "poses",
        DigestRole::Labels => "labels",
        DigestRole::StickerClass => "sticker_class",
        DigestRole::Bytes => "bytes",
        DigestRole::Content => "content",
        DigestRole::Source => "source",
        DigestRole::Other(s) => s,
    }
}

fn blob_ref(value: &Json) -> Result<String, Refusal> {
    require(has_exactly(value, &["stored", "decoded", "stored_bytes", "decoded_bytes", "encoding"]), FORMAT)?;
    digest_of_role(field(value, "stored")?, "bytes")?;
    super::codec::nat(field(value, "stored_bytes")?)?;
    super::codec::nat(field(value, "decoded_bytes")?)?;
    text(value, "encoding")?;
    digest_of_role(field(value, "decoded")?, "bytes")
}

fn pin(value: &Json) -> Result<(), Refusal> {
    require(has_exactly(value, &["version", "source"]), FORMAT)?;
    text(value, "version")?;
    digest_of_role(field(value, "source")?, "source").map(drop)
}

fn reference(value: &Json) -> Result<String, Refusal> {
    require(has_exactly(value, &["format", "version", "content"]), FORMAT)?;
    text(value, "format")?;
    require(matches!(field(value, "version")?, Json::Int(_)), FORMAT)?;
    digest_of_role(field(value, "content")?, "content")
}

fn identity(value: &Json) -> Result<String, Refusal> {
    require(has_exactly(value, &IDENTITY_FIELDS), FORMAT)?;
    Ok(value_text(value))
}

fn statement_id(value: &Json) -> Result<StatementId, Refusal> {
    match text(value, "kind")? {
        "named" if has_exactly(value, &["kind", "stage", "commit", "name"]) => {
            Ok(StatementId::named(text(value, "stage")?, text(value, "commit")?, text(value, "name")?))
        }
        "typed" if has_exactly(value, &["kind", "content"]) => {
            Ok(StatementId::typed(&digest_of_role(field(value, "content")?, "content")?))
        }
        _ => Err(bad()),
    }
}

fn form(value: &Json) -> Result<Option<Form>, Refusal> {
    if value == &Json::Null {
        return Ok(None);
    }
    let kind = text(value, "kind")?;
    let form = match kind {
        "mixed" => {
            require(has_exactly(value, &["kind", "outer"]), FORMAT)?;
            match text(value, "outer")? {
                "forall" => Form::Mixed(Quantifier::Forall),
                "exists" => Form::Mixed(Quantifier::Exists),
                _ => Form::Other(value_text(value)),
            }
        }
        _ => {
            require(has_exactly(value, &["kind"]), FORMAT)?;
            match kind {
                "finite" => Form::Finite,
                "universal" => Form::Universal,
                "existence" => Form::Existence,
                other => Form::Other(other.to_string()),
            }
        }
    };
    Ok(Some(form))
}

fn level(name: &str) -> Level {
    match name {
        "pose" => Level::Pose,
        "sticker_class" => Level::StickerClass,
        "labels" => Level::Labels,
        "region_class" => Level::RegionClass,
        "position" => Level::Position,
        "coset" => Level::Coset,
        "tree_data" => Level::TreeData,
        "shape" => Level::Shape,
        other => Level::Other(other.to_string()),
    }
}

fn premise_class(name: &str) -> PremiseClass {
    match name {
        "reachability_promise" => PremiseClass::ReachabilityPromise,
        "open_hypothesis" => PremiseClass::OpenHypothesis,
        "model_assumption" => PremiseClass::ModelAssumption,
        other => PremiseClass::Other(other.to_string()),
    }
}

fn job_outcome(name: &str) -> JobOutcome {
    match name {
        "completed" => JobOutcome::Completed,
        "cancelled" => JobOutcome::Cancelled,
        "cut_off" => JobOutcome::CutOff,
        "error" => JobOutcome::Error,
        "not_run" => JobOutcome::NotRun,
        "not_stated" => JobOutcome::NotStated,
        other => JobOutcome::Other(other.to_string()),
    }
}

fn statement(value: &Json) -> Result<Statement, Refusal> {
    require(has_exactly(value, &["id", "form", "negation", "subject", "premises", "guards"]), FORMAT)?;
    let mut s = Statement::new(statement_id(field(value, "id")?)?);
    s.form = form(field(value, "form")?)?;
    s.negation = match field(value, "negation")? {
        Json::Null => None,
        id => Some(statement_id(id)?),
    };
    s.subject = opt_text(value, "subject")?.map(level);
    s.premises = texts(value, "premises")?;
    for guard in items(value, "guards")? {
        require(has_exactly(guard, &["class", "name"]), FORMAT)?;
        s.guards.push(Guard { class: premise_class(text(guard, "class")?), name: text(guard, "name")?.to_string() });
    }
    Ok(s)
}

fn covers(value: &Json) -> Result<Covers, Refusal> {
    let kind = text(value, "kind")?;
    let shape: &[&str] = match kind {
        "one_instance" => &["kind"],
        "search_set" => &["kind", "family", "alphabet", "bounds"],
        "admitted_domain" => &["kind", "name", "parameters"],
        "every_reachable" => &["kind", "puzzle"],
        _ => return Ok(Covers::Other(value_text(value))),
    };
    require(has_exactly(value, shape), FORMAT)?;
    Ok(match kind {
        "one_instance" => Covers::OneInstance,
        "search_set" => Covers::search_set(
            text(value, "family")?,
            &value_text(field(value, "alphabet")?),
            &value_text(field(value, "bounds")?),
        ),
        "admitted_domain" => Covers::admitted_domain(text(value, "name")?, &value_text(field(value, "parameters")?)),
        _ => Covers::every_reachable(text(value, "puzzle")?),
    })
}

fn optional_digest(value: &Json) -> Result<Option<DigestRef>, Refusal> {
    match value {
        Json::Null => Ok(None),
        d => digest(d).map(Some),
    }
}

/// PS1.35 and EVD.10: a stored scope holds no revision-bound context. A scope field named in [`TRANSIENT`], or an
/// `input` that is neither null nor a digest reference, is `PS1:evidence.scope_not_immutable`. Applied at write and
/// at read.
pub fn check_scopes(payload: &Json) -> Result<(), Refusal> {
    let Some(Json::List(scopes)) = payload.get("scopes") else { return Ok(()) };
    for scope in scopes {
        let Json::Object(fields) = scope else { continue };
        require(!fields.keys().any(|k| TRANSIENT.contains(&k.as_str())), SCOPE)?;
        if let Some(input) = fields.get("input") {
            let immutable = input == &Json::Null || has_exactly(input, &["role", "sha256"]);
            require(immutable, SCOPE)?;
        }
    }
    Ok(())
}

fn scope(value: &Json) -> Result<(String, Scope), Refusal> {
    require(has_exactly(value, &SCOPE_FIELDS), FORMAT)?;
    let sources = items(value, "sources")?.iter().map(digest).collect::<Result<Vec<_>, _>>()?;
    let s = Scope::new(
        covers(field(value, "covers")?)?,
        &identity(field(value, "identity")?)?,
        text(value, "contract")?,
        optional_digest(field(value, "input")?)?,
    )
    .with_sources(sources);
    Ok((text(value, "id")?.to_string(), s))
}

fn grade(value: &Json) -> Result<Grade, Refusal> {
    require(has_exactly(value, &["basis", "check"]), FORMAT)?;
    Ok(Grade::new(Basis::from_wire(text(value, "basis")?), Check::from_wire(text(value, "check")?)))
}

/// A captured source with its original labels, word for word (PS1.35).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Source {
    /// The decoded digest of the captured bytes.
    pub bytes: String,
    pub format: Option<String>,
    pub pointer: String,
    /// `(pointer, value)` of each original label, in stored order.
    pub labels: Vec<(String, Json)>,
    pub stage: String,
    pub source_version: Option<String>,
    pub engineering: Option<String>,
}

fn source(value: &Json) -> Result<Source, Refusal> {
    let fields = ["bytes", "format", "pointer", "labels", "stage", "source_version", "attribution", "engineering"];
    require(has_exactly(value, &fields), FORMAT)?;
    let mut labels = Vec::new();
    for label in items(value, "labels")? {
        require(has_exactly(label, &["pointer", "value"]), FORMAT)?;
        labels.push((text(label, "pointer")?.to_string(), field(label, "value")?.clone()));
    }
    let engineering = opt_text(value, "engineering")?;
    require(engineering.is_none_or(|e| ENGINEERING.contains(&e)), FORMAT)?;
    Ok(Source {
        bytes: blob_ref(field(value, "bytes")?)?,
        format: opt_text(value, "format")?.map(str::to_string),
        pointer: text(value, "pointer")?.to_string(),
        labels,
        stage: text(value, "stage")?.to_string(),
        source_version: opt_text(value, "source_version")?.map(str::to_string),
        engineering: engineering.map(str::to_string),
    })
}

/// FP.20's minimal closure rules, as the packet's `graph_admission` applies them: unique IDs, no empty AND, every
/// child present.
pub fn graph_admission(nodes: &[Json]) -> Result<(), Refusal> {
    let ids: Vec<&str> = nodes.iter().map(|n| text(n, "id")).collect::<Result<_, _>>()?;
    let unique: BTreeSet<&str> = ids.iter().copied().collect();
    require(unique.len() == ids.len(), FORMAT)?;
    for node in nodes {
        let children = match node.get("children") {
            None => Vec::new(),
            Some(_) => texts(node, "children")?,
        };
        require(text(node, "kind")? != "and" || !children.is_empty(), FORMAT)?;
        require(children.iter().all(|c| unique.contains(c.as_str())), FORMAT)?;
    }
    Ok(())
}

/// An admitted envelope payload, read into the evidence graph.
#[derive(Clone, Debug)]
pub struct Facts {
    pub graph: Graph,
    /// Each root's node ID and key, in stored order.
    pub roots: Vec<(String, Key)>,
    pub sources: Vec<Source>,
    /// The recorded function editions (FP.19), as stored.
    pub editions: Json,
    /// `true` for `status: complete`.
    pub complete: bool,
}

/// FP.19's editions block: exactly the six pins, `import` required.
pub fn check_editions(editions: &Json) -> Result<(), Refusal> {
    require(has_exactly(editions, &EDITIONS), FORMAT)?;
    for name in EDITIONS {
        match field(editions, name)? {
            Json::Null => require(name != "import", FORMAT)?,
            p => pin(p)?,
        }
    }
    Ok(())
}

fn key(value: &Json, scopes: &BTreeMap<String, Scope>) -> Result<Key, Refusal> {
    require(has_exactly(value, &["statement", "scope"]), FORMAT)?;
    let scope = scopes.get(text(value, "scope")?).ok_or_else(bad)?;
    Ok(Key::new(statement_id(field(value, "statement")?)?, scope.clone()))
}

fn search(value: &Json) -> Result<SearchRecord, Refusal> {
    require(has_exactly(value, &["target", "declared", "complete", "stop", "job"]), FORMAT)?;
    let declared = field(value, "declared")?;
    require(has_exactly(declared, &["family", "alphabet", "bounds"]), FORMAT)?;
    let stop = field(value, "stop")?;
    let stop = match text(stop, "kind")? {
        "other" if has_exactly(stop, &["kind", "reason"]) => Stop::Other(text(stop, "reason")?.to_string()),
        "closed" | "deterministic_budget" | "declared_limit" if has_exactly(stop, &["kind"]) => {
            match text(stop, "kind")? {
                "closed" => Stop::Closed,
                "deterministic_budget" => Stop::DeterministicBudget,
                _ => Stop::DeclaredLimit,
            }
        }
        _ => return Err(bad()),
    };
    Ok(SearchRecord {
        target: statement_id(field(value, "target")?)?,
        declared: Declared {
            family: text(declared, "family")?.to_string(),
            alphabet: value_text(field(declared, "alphabet")?),
            bounds: value_text(field(declared, "bounds")?),
        },
        complete: match text(value, "complete")? {
            "yes" => Complete::Yes,
            "no" => Complete::No,
            other => Complete::Other(other.to_string()),
        },
        stop,
        job: job_outcome(text(value, "job")?),
    })
}

fn check_fact(value: &Json) -> Result<CheckFact, Refusal> {
    let fields = ["tool", "common_base", "report", "kind", "result", "mode", "transport"];
    require(has_exactly(value, &fields), FORMAT)?;
    blob_ref(field(value, "report")?)?;
    if field(value, "transport")? != &Json::Null {
        reference(field(value, "transport")?)?;
    }
    let common_base: Result<Vec<String>, Refusal> =
        items(value, "common_base")?.iter().map(|d| digest_of_role(d, "source")).collect();
    Ok(CheckFact {
        tool: digest_of_role(field(value, "tool")?, "source")?,
        common_base: common_base?,
        kind: text(value, "kind")?.to_string(),
        mode: match text(value, "mode")? {
            "reused_exact_transport" => CheckMode::ReusedExactTransport,
            "fresh_exact_audit" => CheckMode::FreshExactAudit,
            "actual_replay" => CheckMode::ActualReplay,
            other => CheckMode::Other(other.to_string()),
        },
        transport: None,
    })
}

/// The wire name of a check mode.
pub fn mode_name(mode: &CheckMode) -> &str {
    match mode {
        CheckMode::ReusedExactTransport => "reused_exact_transport",
        CheckMode::FreshExactAudit => "fresh_exact_audit",
        CheckMode::ActualReplay => "actual_replay",
        CheckMode::StatedNotReplayed => "stated_not_replayed",
        CheckMode::Other(s) => s,
    }
}

fn leaf(value: &Json, scopes: &BTreeMap<String, Scope>, sources: &[Source]) -> Result<Leaf, Refusal> {
    require(has_exactly(value, &LEAF), FORMAT)?;
    let k = key(field(value, "key")?, scopes)?;
    let mut leaf = Leaf::new(&k, grade(field(value, "grade")?)?)
        .identity(&identity(field(value, "identity")?)?)
        .support(&blob_ref(field(value, "record")?)?);
    if let covered @ Json::Object(_) = field(value, "covered")? {
        require(has_exactly(covered, &["covers", "input"]), FORMAT)?;
        leaf =
            leaf.covered(Covered::new(covers(field(covered, "covers")?)?, optional_digest(field(covered, "input")?)?));
    } else {
        require(field(value, "covered")? == &Json::Null, FORMAT)?;
    }
    for check in items(value, "checks")? {
        leaf = leaf.check_fact(check_fact(check)?);
    }
    for member in items(value, "membership")? {
        require(has_exactly(member, &["key", "member", "check"]), FORMAT)?;
        leaf.membership.push(Membership {
            key: key(field(member, "key")?, scopes)?,
            member: reference(field(member, "member")?)?,
            check: grade(field(member, "check")?)?,
        });
    }
    if field(value, "search")? != &Json::Null {
        leaf = leaf.search(search(field(value, "search")?)?);
    }
    if let job @ Json::Object(_) = field(value, "producing_job")? {
        require(has_exactly(job, &["source", "pointer", "id", "outcome"]), FORMAT)?;
        let captured = digest_of_role(field(job, "source")?, "bytes")?;
        require(sources.iter().any(|s| s.bytes == captured), FORMAT)?;
        leaf = leaf.job(ProducingJob {
            source: captured,
            pointer: text(job, "pointer")?.to_string(),
            id: text(job, "id")?.to_string(),
            outcome: job_outcome(text(job, "outcome")?),
        });
    } else {
        require(field(value, "producing_job")? == &Json::Null, FORMAT)?;
    }
    if field(value, "reachability")? != &Json::Null {
        reference(field(value, "reachability")?)?;
    }
    Ok(leaf)
}

fn sorted_unique(ids: &[String]) -> bool {
    ids.windows(2).all(|w| w[0] < w[1])
}

/// FP.19 and FP.20's admission of a facts payload, read into the evidence graph.
pub fn admit(payload: &Json) -> Result<Facts, Refusal> {
    check_scopes(payload)?;
    require(has_exactly(payload, &PAYLOAD), FORMAT)?;
    require(text(payload, "form")? == "facts", FORMAT)?;
    let complete = match text(payload, "status")? {
        "complete" => true,
        "recorded" => false,
        _ => return Err(bad()),
    };
    check_editions(field(payload, "editions")?)?;
    for cache in items(payload, "caches")? {
        require(has_exactly(cache, &["inputs", "functions", "output"]), FORMAT)?;
        items(cache, "inputs")?.iter().try_for_each(|d| digest(d).map(drop))?;
        items(cache, "functions")?.iter().try_for_each(pin)?;
        blob_ref(field(cache, "output")?)?;
    }
    let missing = items(payload, "missing")?;
    for entry in missing {
        require(has_exactly(entry, &["field", "reason", "source"]), FORMAT)?;
        text(entry, "field")?;
        text(entry, "reason")?;
        digest_of_role(field(entry, "source")?, "bytes")?;
    }
    require(!complete || missing.is_empty(), FORMAT)?;
    let sources = items(payload, "sources")?.iter().map(source).collect::<Result<Vec<_>, _>>()?;

    let mut graph = Graph::new();
    let statements = items(payload, "statements")?.iter().map(statement).collect::<Result<Vec<_>, _>>()?;
    let by_id: BTreeMap<&StatementId, &Statement> = statements.iter().map(|s| (&s.id, s)).collect();
    require(by_id.len() == statements.len(), FORMAT)?;
    for s in &statements {
        graph.add_statement(s.clone()).map_err(|_| bad())?;
    }
    let mut scopes = BTreeMap::new();
    for s in items(payload, "scopes")? {
        let (id, scope) = scope(s)?;
        require(scopes.insert(id, scope).is_none(), FORMAT)?;
    }

    let nodes = items(payload, "nodes")?;
    graph_admission(nodes)?;
    let ids: Vec<String> = nodes.iter().map(|n| text(n, "id").map(str::to_string)).collect::<Result<_, _>>()?;
    require(sorted_unique(&ids), FORMAT)?;
    let by_node: BTreeMap<&str, &Json> = ids.iter().map(String::as_str).zip(nodes.iter()).collect();
    let mut built: BTreeMap<String, usize> = BTreeMap::new();
    let mut keys: BTreeMap<String, Key> = BTreeMap::new();
    // OR roots first, so an AND may name one; then leaves and premises; then the ANDs; then the OR edges.
    for (id, node) in &by_node {
        let kind = text(node, "kind")?;
        match kind {
            "or" => {
                require(has_exactly(node, &["id", "kind", "key", "children"]), FORMAT)?;
                let k = key(field(node, "key")?, &scopes)?;
                require(graph.root_of(&k).is_none(), FORMAT)?;
                built.insert(id.to_string(), graph.root(&k));
                keys.insert(id.to_string(), k);
            }
            "leaf" if complete => {
                built.insert(id.to_string(), graph.add_leaf(leaf(node, &scopes, &sources)?));
            }
            "leaf" => {
                leaf(node, &scopes, &sources)?;
            }
            "premise" => {
                require(has_exactly(node, &["id", "kind", "key", "class"]), FORMAT)?;
                let k = key(field(node, "key")?, &scopes)?;
                built.insert(id.to_string(), graph.add_premise(&k, premise_class(text(node, "class")?)));
            }
            "and" => require(has_exactly(node, &["id", "kind", "children"]), FORMAT)?,
            _ => return Err(bad()),
        }
        let children = match node.get("children") {
            Some(_) => texts(node, "children")?,
            None => Vec::new(),
        };
        require(sorted_unique(&children), FORMAT)?;
    }
    if complete {
        let and_ids: Vec<&str> =
            by_node.iter().filter(|(_, n)| text(n, "kind") == Ok("and")).map(|(id, _)| *id).collect();
        for id in and_ids {
            build_and(id, &by_node, &mut graph, &mut built, &mut Vec::new())?;
        }
        for (id, k) in &keys {
            for child in texts(by_node[id.as_str()], "children")? {
                graph.add_or_child(k, built[&child]).map_err(|_| bad())?;
            }
        }
    }
    let root_ids = texts(payload, "roots")?;
    require(sorted_unique(&root_ids), FORMAT)?;
    let mut roots = Vec::new();
    for id in root_ids {
        let k = keys.get(&id).ok_or_else(bad)?.clone();
        roots.push((id, k));
    }
    Ok(Facts { graph, roots, sources, editions: field(payload, "editions")?.clone(), complete })
}

/// Builds an AND node after its children. An AND that reaches itself through ANDs alone cannot be built and is a
/// format fault (draft); a cycle through an OR node is EVD.26's child cycle and is kept.
fn build_and(
    id: &str,
    nodes: &BTreeMap<&str, &Json>,
    graph: &mut Graph,
    built: &mut BTreeMap<String, usize>,
    path: &mut Vec<String>,
) -> Result<usize, Refusal> {
    if let Some(n) = built.get(id) {
        return Ok(*n);
    }
    require(!path.iter().any(|p| p == id), FORMAT)?;
    path.push(id.to_string());
    let mut children = Vec::new();
    for child in texts(nodes[id], "children")? {
        let n = match built.get(&child) {
            Some(n) => *n,
            None => build_and(&child, nodes, graph, built, path)?,
        };
        children.push(n);
    }
    path.pop();
    let n = graph.add_and(&children).map_err(|_| bad())?;
    built.insert(id.to_string(), n);
    Ok(n)
}

/// The function editions this build can run: each recorded editions block it can execute, with the evidence tables it
/// pins.
#[derive(Clone, Debug, Default)]
pub struct Functions {
    known: Vec<(Json, Edition)>,
}

impl Functions {
    pub fn new() -> Functions {
        Functions::default()
    }

    pub fn with(mut self, editions: Json, edition: Edition) -> Functions {
        self.known.push((editions, edition));
        self
    }

    pub fn edition(&self, editions: &Json) -> Option<&Edition> {
        self.known.iter().find(|(pins, _)| pins == editions).map(|(_, e)| e)
    }
}

/// A derived judgement of one root under one recorded edition.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Derived {
    /// EVD.32's display word, rows 1 to 7.
    Word(Word),
    /// Row 0: the typed inconsistency.
    Inconsistent(EvidenceError),
    /// The recorded edition's functions are unavailable: the facts stay stored without a derived verdict (FP.19).
    NoVerdict,
}

/// The display word of each root under `editions`, recomputed from the facts with no cache.
pub fn derive(facts: &Facts, editions: &Json, functions: &Functions) -> Vec<(String, Derived)> {
    let edition = functions.edition(editions);
    facts
        .roots
        .iter()
        .map(|(id, k)| {
            let derived = match edition {
                None => Derived::NoVerdict,
                Some(e) => match word(&facts.graph, k, e) {
                    Ok(w) => Derived::Word(w),
                    Err(error) => Derived::Inconsistent(error),
                },
            };
            (id.clone(), derived)
        })
        .collect()
}

/// PS2.6 and FP.19's edition note, as the packet's `editions_result` computes it: the changed function fields, the
/// unchanged fact-input digest and the digests of the old and new editions blocks.
pub fn edition_note(old: &Json, new: &Json, facts: &Json) -> Result<Json, Refusal> {
    require(has_exactly(old, &EDITIONS) && has_exactly(new, &EDITIONS), FORMAT)?;
    require(old.get("import") != Some(&Json::Null) && new.get("import") != Some(&Json::Null), FORMAT)?;
    let changed = EDITIONS.iter().filter(|f| old.get(f) != new.get(f)).map(|f| Json::text(f)).collect::<Vec<_>>();
    let mut changed = changed;
    changed.sort_by(|a, b| a.as_text().cmp(&b.as_text()));
    let sha = |v: &Json| Json::Text(c600_digest::sha256_hex(&canonical(v)));
    Ok(Json::object([("changed", Json::List(changed)), ("facts", sha(facts)), ("old", sha(old)), ("new", sha(new))]))
}

/// The packet's `editions_result`: [`edition_note`] for a payload that holds no cache.
pub fn editions_result(given: &Json) -> Result<Json, Refusal> {
    require(given.get("caches") == Some(&Json::List(Vec::new())), FORMAT)?;
    edition_note(field(given, "old")?, field(given, "new")?, field(given, "facts")?)
}

/// The fact input an edition note digests: the payload without its function editions and caches, which an edition
/// change leaves unchanged (PS2.6). Draft.
pub fn fact_input(payload: &Json) -> Json {
    match payload {
        Json::Object(fields) => Json::Object(
            fields
                .iter()
                .filter(|(k, _)| *k != "editions" && *k != "caches")
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
        ),
        other => other.clone(),
    }
}

/// The caches a new editions block makes stale: a cache is reused only on equality of every input and function
/// edition (FP.19, PS2.7), so a cache whose functions are not exactly the new block's pins no longer applies.
pub fn stale_caches(caches: &[Json], editions: &Json) -> Vec<Json> {
    let pins: Vec<&Json> = EDITIONS.iter().filter_map(|f| editions.get(f)).filter(|p| **p != Json::Null).collect();
    caches
        .iter()
        .filter(|cache| match cache.get("functions") {
            Some(Json::List(functions)) => !functions.iter().all(|f| pins.contains(&f)),
            _ => true,
        })
        .cloned()
        .collect()
}

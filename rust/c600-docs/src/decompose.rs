//! The generated decomposition (LB6.2, EV5.4): the checked call graph of one entry before executor lowering, with
//! every library entry and kernel call it reaches. It gives structure, not reasons (DT-L1 R3-M4).
//!
//! A node is a function the entry reaches, in any module of the program, with the checker's inferred effects,
//! capabilities and grant-table rows. Its edges are its direct calls, its "may call" edges (a call of a function value
//! or a function reference: the checker's `reaches` over-approximates them, LB6.2), the kernel and host operations its
//! body performs, and its base parts, the IR's own operations at which the decomposition stops. The decomposition's
//! digest is the SHA-256 of its canonical text, which names the IR version and execution contract and every module's
//! source digest, so a changed source, call, effect or kernel call changes it.

use std::collections::{BTreeMap, BTreeSet};

use c600_check::Report;
use c600_ir::{Callee, Expr, FnKind, Program, EXEC_CONTRACT, IR_VERSION};
use c600_syntax::Lowered;

use crate::DocsError;

/// One edge of a node.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Edge {
    /// A direct call of a named function.
    Call { module: String, function: String },
    /// A call the checker cannot pin to one callee: a call of a function value, or a function reference passed on.
    MayCall { module: String, function: String },
    /// A call the lowering could not resolve: dynamic code, never admitted.
    Unresolved { name: String },
    /// A kernel or host operation, an opaque leaf: `op` is the operation, `name` its row, record, target or field.
    Kernel { op: String, name: String },
    /// One of the IR's own operations, a base part.
    Base { op: String },
}

impl Edge {
    /// The edge's canonical text.
    pub fn text(&self) -> String {
        match self {
            Edge::Call { module, function } => format!("call {module}.{function}"),
            Edge::MayCall { module, function } => format!("may call {module}.{function}"),
            Edge::Unresolved { name } => format!("unresolved {name}"),
            Edge::Kernel { op, name } if name.is_empty() => format!("kernel {op}"),
            Edge::Kernel { op, name } => format!("kernel {op}({name})"),
            Edge::Base { op } => format!("base {op}"),
        }
    }

    /// The label a step map names a leaf by: `kernel:op(name)` or `base:op`. Calls have none.
    pub fn leaf_label(&self) -> Option<String> {
        match self {
            Edge::Kernel { op, name } if name.is_empty() => Some(format!("kernel:{op}")),
            Edge::Kernel { op, name } => Some(format!("kernel:{op}({name})")),
            Edge::Base { op } => Some(format!("base:{op}")),
            _ => None,
        }
    }
}

/// One function the entry reaches.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Node {
    pub module: String,
    pub function: String,
    /// The SHA-256 of the module's source.
    pub source_sha256: String,
    pub kind: &'static str,
    pub exported: bool,
    /// The checker's inferred effects, capabilities and grant-table rows, by their IR names.
    pub effects: Vec<String>,
    pub capabilities: Vec<String>,
    pub rows: Vec<String>,
    pub edges: BTreeSet<Edge>,
}

impl Node {
    /// `module.function`: the label a step map names a node by.
    pub fn label(&self) -> String {
        format!("{}.{}", self.module, self.function)
    }
}

/// The generated decomposition of one entry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Decomposition {
    pub module: String,
    pub entry: String,
    pub ir_version: String,
    pub exec_contract: String,
    /// In the order (module, function).
    pub nodes: Vec<Node>,
}

impl Decomposition {
    /// The canonical text: one line per fact, in a fixed order.
    pub fn canonical(&self) -> String {
        let mut out = format!(
            "c600-decomposition-draft-1\nir {} {}\nentry {}.{}\n",
            self.ir_version, self.exec_contract, self.module, self.entry
        );
        for n in &self.nodes {
            out.push_str(&format!(
                "node {} source {} kind {} exported {} effects [{}] capabilities [{}] rows [{}]\n",
                n.label(),
                n.source_sha256,
                n.kind,
                n.exported,
                n.effects.join(", "),
                n.capabilities.join(", "),
                n.rows.join(", ")
            ));
            for e in &n.edges {
                out.push_str(&format!("  {}\n", e.text()));
            }
        }
        out
    }

    /// The SHA-256 of [`Decomposition::canonical`].
    pub fn digest(&self) -> String {
        c600_digest::sha256_hex(self.canonical().as_bytes())
    }

    /// Every node's label.
    pub fn node_labels(&self) -> BTreeSet<String> {
        self.nodes.iter().map(Node::label).collect()
    }

    /// The opaque kernel leaves the entry reaches transitively, by leaf label.
    pub fn kernel_leaves(&self) -> BTreeSet<String> {
        self.edges().filter(|e| matches!(e, Edge::Kernel { .. })).filter_map(Edge::leaf_label).collect()
    }

    /// The base parts the entry reaches transitively, by leaf label.
    pub fn base_parts(&self) -> BTreeSet<String> {
        self.edges().filter(|e| matches!(e, Edge::Base { .. })).filter_map(Edge::leaf_label).collect()
    }

    /// The modules the entry reaches, other than its own.
    pub fn other_modules(&self) -> BTreeSet<String> {
        self.nodes.iter().map(|n| n.module.clone()).filter(|m| *m != self.module).collect()
    }

    fn edges(&self) -> impl Iterator<Item = &Edge> {
        self.nodes.iter().flat_map(|n| n.edges.iter())
    }

    /// The node `module.function`, if the entry reaches it.
    pub fn node(&self, label: &str) -> Option<&Node> {
        self.nodes.iter().find(|n| n.label() == label)
    }
}

fn kind_name(kind: FnKind) -> &'static str {
    match kind {
        FnKind::Helper => "helper",
        FnKind::View => "view",
        FnKind::Command => "command",
        FnKind::Contract => "contract",
    }
}

/// Generates the decomposition of `module.entry` from a lowered program and the checker's report for it. The report
/// must admit the program: an unchecked graph is no decomposition (LB6.2).
pub fn decompose(lowered: &Lowered, report: &Report, module: &str, entry: &str) -> Result<Decomposition, DocsError> {
    if !report.is_admitted() {
        let codes = report.codes().join(", ");
        return Err(DocsError::new(
            crate::code::DECOMPOSITION_MISMATCH,
            format!("the checker refuses the program: {codes}"),
        ));
    }
    let program = &lowered.program;
    let digests: BTreeMap<&str, &str> =
        lowered.sources.iter().map(|s| (s.module.as_str(), s.digest.as_str())).collect();
    let mut nodes: BTreeMap<(String, String), Node> = BTreeMap::new();
    let mut pending = vec![(module.to_string(), entry.to_string())];
    while let Some((m, f)) = pending.pop() {
        if nodes.contains_key(&(m.clone(), f.clone())) {
            continue;
        }
        let function = find(program, &m, &f).ok_or_else(|| {
            DocsError::new(crate::code::DECOMPOSITION_MISMATCH, format!("the program has no function `{m}.{f}`"))
        })?;
        let mut edges = BTreeSet::new();
        walk(&function.body, &m, &mut edges);
        let summary = report.summary(&m, &f);
        // A function value or reference may call whatever the checker says the body reaches beyond its direct calls.
        if let Some(summary) = summary {
            let direct: BTreeSet<(String, String)> = edges
                .iter()
                .filter_map(|e| match e {
                    Edge::Call { module, function } => Some((module.clone(), function.clone())),
                    _ => None,
                })
                .collect();
            for (rm, rf) in &summary.reaches {
                if !direct.contains(&(rm.clone(), rf.clone())) {
                    edges.insert(Edge::MayCall { module: rm.clone(), function: rf.clone() });
                }
            }
        }
        for e in &edges {
            if let Edge::Call { module, function } | Edge::MayCall { module, function } = e {
                pending.push((module.clone(), function.clone()));
            }
        }
        let names = |items: Vec<String>| {
            let mut items = items;
            items.sort();
            items.dedup();
            items
        };
        let node = Node {
            module: m.clone(),
            function: f.clone(),
            source_sha256: digests.get(m.as_str()).map(|d| d.to_string()).unwrap_or_default(),
            kind: kind_name(function.kind),
            exported: function.exported,
            effects: names(summary.map(|s| s.effects.iter().map(|e| format!("{e:?}")).collect()).unwrap_or_default()),
            capabilities: names(
                summary.map(|s| s.capabilities.iter().map(|c| format!("{c:?}")).collect()).unwrap_or_default(),
            ),
            rows: names(summary.map(|s| s.rows.iter().map(|r| format!("{r:?}")).collect()).unwrap_or_default()),
            edges,
        };
        nodes.insert((m, f), node);
    }
    Ok(Decomposition {
        module: module.to_string(),
        entry: entry.to_string(),
        ir_version: program.ir_version.clone(),
        exec_contract: program.exec_contract.clone(),
        nodes: nodes.into_values().collect(),
    })
}

fn find<'p>(program: &'p Program, module: &str, function: &str) -> Option<&'p c600_ir::Function> {
    program.modules.iter().filter(|m| m.name == module).flat_map(|m| &m.functions).find(|f| f.name == function)
}

fn kernel(op: &str, name: &str) -> Edge {
    Edge::Kernel { op: op.to_string(), name: name.to_string() }
}

fn literal_text(e: &Expr) -> String {
    match e {
        Expr::Lit(c600_ir::Lit::Target(t)) => t.clone(),
        _ => "not a literal".to_string(),
    }
}

/// Collects the edges of one body, closures included.
fn walk(e: &Expr, module: &str, out: &mut BTreeSet<Edge>) {
    match e {
        Expr::Call { callee: Callee::Fn { module: m, name }, .. } => {
            let m = m.clone().unwrap_or_else(|| module.to_string());
            out.insert(Edge::Call { module: m, function: name.clone() });
        }
        Expr::Call { callee: Callee::Unresolved { name, .. }, .. } => {
            out.insert(Edge::Unresolved { name: name.clone() });
        }
        Expr::FnRef { module: m, name } => {
            let m = m.clone().unwrap_or_else(|| module.to_string());
            out.insert(Edge::MayCall { module: m, function: name.clone() });
        }
        Expr::Prim { op, .. } => {
            out.insert(Edge::Base { op: format!("{op:?}") });
        }
        Expr::Repeat { .. } => {
            out.insert(Edge::Base { op: "Repeat".into() });
        }
        Expr::Child { .. } => {
            out.insert(Edge::Base { op: "Child".into() });
        }
        Expr::UseExact { role, .. } => {
            out.insert(Edge::Base { op: format!("UseExact({role:?})") });
        }
        Expr::TextDecodeHex { .. } => {
            out.insert(Edge::Base { op: "TextDecodeHex".into() });
        }
        Expr::ReadSnapshot { query, .. } | Expr::ReadSnapshotWith { query, .. } => {
            out.insert(kernel("read_snapshot", query));
        }
        Expr::PublishView { .. } => {
            out.insert(kernel("publish_view", ""));
        }
        Expr::WriteWorkspace { record, .. } => {
            out.insert(kernel("write_workspace", record));
        }
        Expr::Propose { target, .. } => {
            out.insert(kernel("propose", &literal_text(target)));
        }
        Expr::ReadReceipt { field, .. } => {
            out.insert(kernel("receipt", &format!("{field:?}")));
        }
        Expr::Declassified { id, .. } => {
            out.insert(kernel("declassified", id));
        }
        Expr::FileRead { path, .. } => {
            out.insert(kernel("file_read", path));
        }
        Expr::FileWrite { path, .. } => {
            out.insert(kernel("file_write", path));
        }
        Expr::RandomDraw => {
            out.insert(kernel("random_draw", ""));
        }
        Expr::ReadClock => {
            out.insert(kernel("read_clock", ""));
        }
        Expr::RngInit { .. } => {
            out.insert(kernel("rng_init", ""));
        }
        Expr::Accounting(a) => {
            out.insert(kernel("accounting", &format!("{a:?}")));
        }
        _ => {}
    }
    // A call of a function value: the callee is among the children, and what it may call is the checker's `reaches`.
    for child in e.children() {
        walk(child, module, out);
    }
}

/// A submitted decomposition against the generated one (LB6.6, RTE.80): a submitted graph that omits a derived node,
/// call, effect or kernel call, or that names other source or build inputs, is `LB6:evidence.decomposition_mismatch`.
pub fn check_submitted(submitted: &Decomposition, generated: &Decomposition) -> Result<(), DocsError> {
    let refuse = |detail: String| Err(DocsError::new(crate::code::DECOMPOSITION_MISMATCH, detail));
    if (submitted.ir_version.as_str(), submitted.exec_contract.as_str()) != (IR_VERSION, EXEC_CONTRACT)
        || (submitted.ir_version.as_str(), submitted.exec_contract.as_str())
            != (generated.ir_version.as_str(), generated.exec_contract.as_str())
    {
        return refuse("the submitted graph names another IR version or execution contract".into());
    }
    if (submitted.module.as_str(), submitted.entry.as_str()) != (generated.module.as_str(), generated.entry.as_str()) {
        return refuse("the submitted graph is of another entry".into());
    }
    for g in &generated.nodes {
        let Some(s) = submitted.nodes.iter().find(|s| s.label() == g.label()) else {
            return refuse(format!("the submitted graph omits the derived node {}", g.label()));
        };
        if s.source_sha256 != g.source_sha256 {
            return refuse(format!("{} is bound to other source bytes", g.label()));
        }
        for e in &g.effects {
            if !s.effects.contains(e) {
                return refuse(format!("{} omits the derived effect {e}", g.label()));
            }
        }
        for e in &g.edges {
            if !s.edges.contains(e) {
                return refuse(format!("{} omits the derived edge `{}`", g.label(), e.text()));
            }
        }
    }
    Ok(())
}

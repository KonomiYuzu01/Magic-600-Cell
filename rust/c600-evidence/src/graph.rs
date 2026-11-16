//! Section 3: the AND/OR evidence graph over keys. Arguments are immutable: a new record enters as a new node or a
//! new OR child, and nothing stored changes (EVD.16). The building calls here are the stored-record path; `sink`
//! holds the entry checks of EVD.18 and the writer checks of EVD.64.

use std::collections::{BTreeMap, BTreeSet};

use crate::error::EvidenceError;
use crate::model::{Key, Leaf, Premise, PremiseClass, ProducingJob, SearchRecord, Statement, StatementId};

/// Index of a node in its graph.
pub type NodeId = usize;

/// The OR node of one key (EVD.15), with the search records and jobs recorded for the key itself.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OrNode {
    pub key: Key,
    pub children: Vec<NodeId>,
    /// Search records that rest on no leaf, such as a truncated beam that found nothing (section 9, E3). Draft:
    /// FP.20 carries a search record inside a leaf only.
    pub searches: Vec<SearchRecord>,
    /// Jobs recorded for the key with no leaf, such as a cancelled run (section 9, E3).
    pub jobs: Vec<ProducingJob>,
}

/// A node of the graph.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Node {
    Or(OrNode),
    And(Vec<NodeId>),
    Leaf(Leaf),
    Premise(Premise),
}

/// How the negation of a key resolves (EVD.9).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NegResolution {
    Absent,
    One(NodeId),
    /// Several roots carry the negated statement and none has the identical scope. Read as I2 (draft).
    Ambiguous,
}

/// The evidence graph: statement facts, one OR node per key, and the nodes.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Graph {
    statements: BTreeMap<StatementId, Statement>,
    roots: BTreeMap<Key, NodeId>,
    nodes: Vec<Node>,
}

impl Graph {
    pub fn new() -> Graph {
        Graph::default()
    }

    /// Records a statement's facts. Statements are immutable: recording the same facts again is a no-op, and
    /// different facts under a stored ID are refused (EVD.8, EVD.16).
    pub fn add_statement(&mut self, statement: Statement) -> Result<(), EvidenceError> {
        match self.statements.get(&statement.id) {
            Some(old) if *old == statement => Ok(()),
            Some(_) => Err(EvidenceError::inconsistent(
                vec![crate::error::Inconsistency::I2],
                "a stored statement's facts cannot change",
            )),
            None => {
                self.statements.insert(statement.id.clone(), statement);
                Ok(())
            }
        }
    }

    pub fn statement(&self, id: &StatementId) -> Option<&Statement> {
        self.statements.get(id)
    }

    pub fn statements(&self) -> impl Iterator<Item = &Statement> {
        self.statements.values()
    }

    /// The OR node of `key`, created empty when absent. An empty OR is unsupported (FP.20).
    pub fn root(&mut self, key: &Key) -> NodeId {
        if let Some(id) = self.roots.get(key) {
            return *id;
        }
        let id = self.nodes.len();
        self.nodes.push(Node::Or(OrNode {
            key: key.clone(),
            children: Vec::new(),
            searches: Vec::new(),
            jobs: Vec::new(),
        }));
        self.roots.insert(key.clone(), id);
        id
    }

    pub fn root_of(&self, key: &Key) -> Option<NodeId> {
        self.roots.get(key).copied()
    }

    pub fn roots(&self) -> impl Iterator<Item = (&Key, &NodeId)> {
        self.roots.iter()
    }

    pub fn node(&self, id: NodeId) -> Option<&Node> {
        self.nodes.get(id)
    }

    pub fn nodes(&self) -> &[Node] {
        &self.nodes
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// A leaf node. It supports nothing until an OR or AND node names it.
    pub fn add_leaf(&mut self, leaf: Leaf) -> NodeId {
        self.nodes.push(Node::Leaf(leaf));
        self.nodes.len() - 1
    }

    /// An AND node over existing nodes. An AND with no children is stored so that it reads I8 (EVD.15).
    pub fn add_and(&mut self, children: &[NodeId]) -> Result<NodeId, EvidenceError> {
        self.check_ids(children)?;
        self.nodes.push(Node::And(children.to_vec()));
        Ok(self.nodes.len() - 1)
    }

    /// A premise node for `key`. It holds when assumed or when the claim with the identical key holds.
    pub fn add_premise(&mut self, key: &Key, class: PremiseClass) -> NodeId {
        self.nodes.push(Node::Premise(Premise { key: key.clone(), class, discharged_by: None }));
        self.nodes.len() - 1
    }

    /// A premise node with a recorded discharge edge. An edge to a different key reads I2.
    pub fn add_premise_discharged_by(&mut self, key: &Key, class: PremiseClass, by: &Key) -> NodeId {
        self.nodes.push(Node::Premise(Premise { key: key.clone(), class, discharged_by: Some(by.clone()) }));
        self.nodes.len() - 1
    }

    /// A new OR child of `key`: one more independent argument (EVD.15, EVD.16).
    pub fn add_or_child(&mut self, key: &Key, child: NodeId) -> Result<(), EvidenceError> {
        self.check_ids(&[child])?;
        let root = self.root(key);
        if let Node::Or(or) = &mut self.nodes[root] {
            if !or.children.contains(&child) {
                or.children.push(child);
            }
        }
        Ok(())
    }

    /// Stored-record path: a leaf as a new argument of `key`, with no entry check (I7 is what such a record reads,
    /// EVD.18). `sink::offer` is the checked path.
    pub fn support(&mut self, key: &Key, leaf: Leaf) -> NodeId {
        let id = self.add_leaf(leaf);
        let root = self.root(key);
        if let Node::Or(or) = &mut self.nodes[root] {
            or.children.push(id);
        }
        id
    }

    /// A search record for `key` that rests on no leaf.
    pub fn attach_search(&mut self, key: &Key, record: SearchRecord) {
        let root = self.root(key);
        if let Node::Or(or) = &mut self.nodes[root] {
            or.searches.push(record);
        }
    }

    /// A job recorded for `key` with no leaf.
    pub fn attach_job(&mut self, key: &Key, job: ProducingJob) {
        let root = self.root(key);
        if let Node::Or(or) = &mut self.nodes[root] {
            or.jobs.push(job);
        }
    }

    fn check_ids(&self, ids: &[NodeId]) -> Result<(), EvidenceError> {
        match ids.iter().find(|i| **i >= self.nodes.len()) {
            Some(i) => Err(EvidenceError::inconsistent(
                vec![crate::error::Inconsistency::I2],
                &format!("node {i} does not exist"),
            )),
            None => Ok(()),
        }
    }

    /// EVD.9: the root of neg K. Draft: the root with the negated statement and the identical scope; otherwise the
    /// only root with the negated statement; several such roots read I2.
    pub fn neg_root(&self, key: &Key) -> NegResolution {
        let Some(neg) = self.statements.get(&key.statement).and_then(|s| s.negation.clone()) else {
            return NegResolution::Absent;
        };
        let same_scope = Key { statement: neg.clone(), scope: key.scope.clone() };
        if let Some(id) = self.roots.get(&same_scope) {
            return NegResolution::One(*id);
        }
        let mut found: Vec<NodeId> = self.roots.iter().filter(|(k, _)| k.statement == neg).map(|(_, id)| *id).collect();
        match found.len() {
            0 => NegResolution::Absent,
            1 => NegResolution::One(found.pop().unwrap_or_default()),
            _ => NegResolution::Ambiguous,
        }
    }

    /// The key of an OR node.
    pub fn key_of(&self, root: NodeId) -> Option<&Key> {
        match self.nodes.get(root) {
            Some(Node::Or(or)) => Some(&or.key),
            _ => None,
        }
    }

    /// EVD.38: the smallest set of nodes containing S and neg S, closed under children, discharge edges and negation
    /// pairs of premises. Recorded discharge edges to other keys are followed too, so that I2 can read them.
    pub fn reach(&self, key: &Key) -> BTreeSet<NodeId> {
        let mut start = Vec::new();
        if let Some(r) = self.root_of(key) {
            start.push(r);
        }
        if let NegResolution::One(n) = self.neg_root(key) {
            start.push(n);
        }
        self.closure(start)
    }

    /// Closure under children, discharge edges and negation pairs of premises.
    pub fn closure(&self, start: Vec<NodeId>) -> BTreeSet<NodeId> {
        let mut seen: BTreeSet<NodeId> = BTreeSet::new();
        let mut stack = start;
        while let Some(id) = stack.pop() {
            if !seen.insert(id) {
                continue;
            }
            match &self.nodes[id] {
                Node::Or(or) => stack.extend(or.children.iter().copied()),
                Node::And(children) => stack.extend(children.iter().copied()),
                Node::Leaf(_) => {}
                Node::Premise(p) => {
                    if let Some(r) = self.root_of(&p.key) {
                        stack.push(r);
                    }
                    if let Some(by) = &p.discharged_by {
                        if let Some(r) = self.root_of(by) {
                            stack.push(r);
                        }
                    }
                    if let NegResolution::One(n) = self.neg_root(&p.key) {
                        stack.push(n);
                    }
                }
            }
        }
        seen
    }

    /// The arguments of an OR node: for each OR child, the leaves in it. A leaf is in an argument of S when it is the
    /// direct child of S's OR node, or a child of an AND node under it; a child reached through a premise or another
    /// key belongs to that key's statement (EVD.40).
    pub fn arguments(&self, root: NodeId) -> Vec<Vec<NodeId>> {
        let Some(Node::Or(or)) = self.nodes.get(root) else {
            return Vec::new();
        };
        or.children.iter().map(|c| self.leaves_under(*c)).collect()
    }

    fn leaves_under(&self, start: NodeId) -> Vec<NodeId> {
        let mut out = Vec::new();
        let mut seen = BTreeSet::new();
        let mut stack = vec![start];
        while let Some(id) = stack.pop() {
            if !seen.insert(id) {
                continue;
            }
            match &self.nodes[id] {
                Node::Leaf(_) => out.push(id),
                Node::And(children) => stack.extend(children.iter().copied()),
                Node::Or(_) | Node::Premise(_) => {}
            }
        }
        out.sort();
        out
    }

    pub fn leaf(&self, id: NodeId) -> Option<&Leaf> {
        match self.nodes.get(id) {
            Some(Node::Leaf(l)) => Some(l),
            _ => None,
        }
    }
}

//! Shared helpers for the chapter 06 conformance tests: builders for statements and keys, a deterministic PRNG,
//! a random graph generator and the test's own transcriptions of EVD.19's order, EVD.22's least solution and
//! EVD.38's Reach. Expected values never come from the library's output.

#![allow(dead_code)]

use std::collections::BTreeSet;

use c600_evidence::*;

pub const IDENTITY: &str = "identity:600-cell-Full:test";
pub const CONTRACT: &str = "contract:test-r1";
pub const COMMIT: &str = "0123456789abcdef0123456789abcdef01234567";

pub fn ed() -> Edition {
    Edition::draft_1()
}

pub fn gr(basis: Basis, check: Check) -> Grade {
    Grade::new(basis, check)
}

/// (theorem, model review): inside the trial threshold U0.
pub fn th() -> Grade {
    gr(Basis::Theorem, Check::ModelReview)
}

/// (exact computation, producer): inside U0.
pub fn exact() -> Grade {
    gr(Basis::ExactComputation, Check::Producer)
}

/// (exhaustive computation, producer): inside U0.
pub fn exhaustive() -> Grade {
    gr(Basis::ExhaustiveComputation, Check::Producer)
}

pub fn sid(name: &str) -> StatementId {
    StatementId::named("test", COMMIT, name)
}

pub fn scope(covers: Covers) -> Scope {
    Scope::new(covers, IDENTITY, CONTRACT, None)
}

pub fn scope_in(covers: Covers, input: DigestRef) -> Scope {
    Scope::new(covers, IDENTITY, CONTRACT, Some(input))
}

pub fn key(name: &str, covers: Covers) -> Key {
    Key::new(sid(name), scope(covers))
}

/// A synthetic poses digest: SHA-256 of a tag, so distinct tags give distinct digests.
pub fn poses(tag: &str) -> DigestRef {
    DigestRef::poses(&c600_digest::sha256_hex(tag.as_bytes()))
}

pub fn labels(tag: &str) -> DigestRef {
    DigestRef::labels(&c600_digest::sha256_hex(tag.as_bytes()))
}

/// Registers a mutual negation pair (EVD.9).
pub fn pair(g: &mut Graph, s: Statement, n: Statement) {
    let sid_s = s.id.clone();
    let sid_n = n.id.clone();
    g.add_statement(s.negation(&sid_n)).expect("statement");
    g.add_statement(n.negation(&sid_s)).expect("statement");
}

pub fn finite(name: &str) -> Statement {
    Statement::new(sid(name)).form(Form::Finite)
}

pub fn universal(name: &str) -> Statement {
    Statement::new(sid(name)).form(Form::Universal)
}

pub fn existence(name: &str) -> Statement {
    Statement::new(sid(name)).form(Form::Existence)
}

pub fn code(r: Result<Word, EvidenceError>) -> String {
    match r {
        Ok(w) => format!("row {}", w.row),
        Err(e) => e.code.to_string(),
    }
}

pub fn row(g: &Graph, k: &Key, e: &Edition) -> u8 {
    match word(g, k, e) {
        Ok(w) => w.row,
        Err(err) => panic!("expected a word for {k:?}, got {err}"),
    }
}

// --- EVD.19 and EVD.29, transcribed by the test ---------------------------------------------------------------

/// EVD.19: sample lies below the four other bases, which are pairwise incomparable.
pub fn t_basis_le(a: &Basis, b: &Basis) -> bool {
    a == b || *a == Basis::Sample
}

/// EVD.19: none lies below producer, and producer below the four later checks, which are pairwise incomparable.
pub fn t_check_le(a: &Check, b: &Check) -> bool {
    a == b || *a == Check::None || (*a == Check::Producer && *b != Check::None)
}

pub fn t_le(a: &Grade, b: &Grade) -> bool {
    t_basis_le(&a.basis, &b.basis) && t_check_le(&a.check, &b.check)
}

pub fn all_grades() -> Vec<Grade> {
    let mut out = Vec::new();
    for b in [Basis::Theorem, Basis::Certificate, Basis::ExhaustiveComputation, Basis::ExactComputation, Basis::Sample] {
        for k in
            [Check::None, Check::Producer, Check::IndependentChecker, Check::ModelReview, Check::HumanReview, Check::FormalProof]
        {
            out.push(gr(b.clone(), k));
        }
    }
    out
}

/// Calls `f` with every up-set U of P inside U0 = {(b, k) : b != sample, k != none} (EVD.29). Inside U0 a grade
/// (b, producer) lies below the four later checks of the same basis, and nothing else is comparable, so for each of
/// the four bases U holds either a subset of the four later checks or all five checks: 17 choices, 17^4 = 83,521 sets.
pub fn for_each_threshold(mut f: impl FnMut(&BTreeSet<Grade>)) {
    let bases = [Basis::Theorem, Basis::Certificate, Basis::ExhaustiveComputation, Basis::ExactComputation];
    let tops = [Check::IndependentChecker, Check::ModelReview, Check::HumanReview, Check::FormalProof];
    let choice = |b: &Basis, c: usize| -> Vec<Grade> {
        if c == 16 {
            let mut v = vec![gr(b.clone(), Check::Producer)];
            v.extend(tops.iter().map(|k| gr(b.clone(), k.clone())));
            v
        } else {
            (0..4).filter(|i| c & (1 << i) != 0).map(|i| gr(b.clone(), tops[i].clone())).collect()
        }
    };
    for c0 in 0..17 {
        for c1 in 0..17 {
            for c2 in 0..17 {
                for c3 in 0..17 {
                    let mut u = BTreeSet::new();
                    for (b, c) in bases.iter().zip([c0, c1, c2, c3]) {
                        u.extend(choice(b, c));
                    }
                    f(&u);
                }
            }
        }
    }
}

// --- A deterministic PRNG (SplitMix64) ------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed)
    }

    pub fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    pub fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }

    pub fn chance(&mut self, percent: u64) -> bool {
        self.next() % 100 < percent
    }

    pub fn shuffle<T>(&mut self, v: &mut [T]) {
        for i in (1..v.len()).rev() {
            let j = self.below(i + 1);
            v.swap(i, j);
        }
    }
}

// --- Random graphs --------------------------------------------------------------------------------------------

/// A graph description the tests can change and rebuild. Keys 0..base are statements `s{i}`; when `paired`, key
/// base + i is `not-s{i}`, the negation of key i. Every statement is finite over one instance, so no admission rule
/// other than I4 can fire.
#[derive(Clone, Debug)]
pub struct Spec {
    pub base: usize,
    pub paired: bool,
    pub nodes: Vec<SpecNode>,
    pub or_children: Vec<Vec<usize>>,
}

#[derive(Clone, Debug)]
pub enum SpecNode {
    Leaf(Grade),
    Premise(usize),
    And(Vec<usize>),
    Root(usize),
}

impl Spec {
    pub fn key_count(&self) -> usize {
        if self.paired {
            2 * self.base
        } else {
            self.base
        }
    }

    pub fn key(&self, k: usize) -> Key {
        if k < self.base {
            key(&format!("s{k}"), Covers::OneInstance)
        } else {
            key(&format!("not-s{}", k - self.base), Covers::OneInstance)
        }
    }

    pub fn keys(&self) -> Vec<Key> {
        (0..self.key_count()).map(|k| self.key(k)).collect()
    }

    pub fn statements(&self) -> Vec<Statement> {
        let mut out = Vec::new();
        for i in 0..self.base {
            let s = finite(&format!("s{i}"));
            if self.paired {
                out.push(s.negation(&sid(&format!("not-s{i}"))));
                out.push(finite(&format!("not-s{i}")).negation(&sid(&format!("s{i}"))));
            } else {
                out.push(s);
            }
        }
        out
    }

    /// Builds the graph. Returns it with the node id of each spec node.
    pub fn build(&self) -> (Graph, Vec<NodeId>) {
        let mut g = Graph::new();
        for s in self.statements() {
            g.add_statement(s).expect("statement");
        }
        for k in 0..self.key_count() {
            g.root(&self.key(k));
        }
        let mut ids = Vec::new();
        for n in &self.nodes {
            let id = match n {
                SpecNode::Leaf(grade) => g.add_leaf(Leaf::new(&self.key(0), grade.clone())),
                SpecNode::Premise(k) => g.add_premise(&self.key(*k), PremiseClass::OpenHypothesis),
                SpecNode::And(children) => {
                    let c: Vec<NodeId> = children.iter().map(|i| ids[*i]).collect();
                    g.add_and(&c).expect("and")
                }
                SpecNode::Root(k) => g.root_of(&self.key(*k)).expect("root"),
            };
            ids.push(id);
        }
        for (k, children) in self.or_children.iter().enumerate() {
            for c in children {
                g.add_or_child(&self.key(k), ids[*c]).expect("or child");
            }
        }
        (g, ids)
    }

    /// The same graph with every shared node copied, so each reference gets its own node. OR roots stay shared.
    pub fn unshared(&self) -> Spec {
        let mut out = Spec { base: self.base, paired: self.paired, nodes: Vec::new(), or_children: Vec::new() };
        fn copy(spec: &Spec, out: &mut Spec, i: usize) -> usize {
            let node = match &spec.nodes[i] {
                SpecNode::And(children) => {
                    let c: Vec<usize> = children.iter().map(|j| copy(spec, out, *j)).collect();
                    SpecNode::And(c)
                }
                other => other.clone(),
            };
            out.nodes.push(node);
            out.nodes.len() - 1
        }
        for children in &self.or_children {
            let c: Vec<usize> = children.iter().map(|i| copy(self, &mut out, *i)).collect();
            out.or_children.push(c);
        }
        out
    }

    pub fn grades(&self) -> BTreeSet<Grade> {
        self.nodes
            .iter()
            .filter_map(|n| match n {
                SpecNode::Leaf(g) => Some(g.clone()),
                _ => None,
            })
            .collect()
    }
}

/// A random spec: `base` keys, a pool of `pool` grades, up to three OR children per key, AND nodes of one to three
/// parts, premises, shared nodes and child edges to OR roots (which can close a pure child cycle). Negation keys get
/// leaves only, so a negation root is reached only through EVD.38's negation pairs.
pub fn random_spec(rng: &mut Rng, base: usize, paired: bool, pool: usize) -> Spec {
    let all = all_grades();
    let mut pool_grades: Vec<Grade> = Vec::new();
    while pool_grades.len() < pool {
        let g = all[rng.below(all.len())].clone();
        if !pool_grades.contains(&g) {
            pool_grades.push(g);
        }
    }
    let mut spec = Spec { base, paired, nodes: Vec::new(), or_children: Vec::new() };
    fn gen(rng: &mut Rng, spec: &mut Spec, pool: &[Grade], depth: usize) -> usize {
        let roll = rng.below(100);
        let node = if roll < 40 || depth >= 2 {
            SpecNode::Leaf(pool[rng.below(pool.len())].clone())
        } else if roll < 60 {
            SpecNode::Premise(rng.below(spec.base))
        } else if roll < 85 {
            let n = 1 + rng.below(3);
            let c: Vec<usize> = (0..n).map(|_| gen(rng, spec, pool, depth + 1)).collect();
            SpecNode::And(c)
        } else if roll < 95 {
            let shareable: Vec<usize> =
                (0..spec.nodes.len()).filter(|i| !matches!(spec.nodes[*i], SpecNode::Root(_))).collect();
            if shareable.is_empty() {
                SpecNode::Leaf(pool[rng.below(pool.len())].clone())
            } else {
                return shareable[rng.below(shareable.len())];
            }
        } else {
            SpecNode::Root(rng.below(spec.base))
        };
        spec.nodes.push(node);
        spec.nodes.len() - 1
    }
    for k in 0..spec.key_count() {
        let mut children = Vec::new();
        let n = rng.below(4);
        for _ in 0..n {
            let c = if k < base {
                gen(rng, &mut spec, &pool_grades, 0)
            } else {
                spec.nodes.push(SpecNode::Leaf(pool_grades[rng.below(pool_grades.len())].clone()));
                spec.nodes.len() - 1
            };
            children.push(c);
        }
        spec.or_children.push(children);
    }
    spec
}

// --- EVD.22 and EVD.38, transcribed by the test ---------------------------------------------------------------

/// EVD.22's least solution at one scenario, by iteration from the empty set (Kleene). Every leaf of a random spec is
/// live (finite statements, no jobs, no failed checks).
pub fn brute_val(g: &Graph, distrusted: &dyn Fn(&Grade) -> bool, assumed: &BTreeSet<Key>) -> Vec<bool> {
    let nodes = g.nodes();
    let mut v = vec![false; nodes.len()];
    loop {
        let mut changed = false;
        for (i, n) in nodes.iter().enumerate() {
            if v[i] {
                continue;
            }
            let nv = match n {
                Node::Leaf(q) => !q.failed && !distrusted(&q.grade),
                Node::Premise(p) => assumed.contains(&p.key) || g.root_of(&p.key).is_some_and(|r| v[r]),
                Node::And(c) => !c.is_empty() && c.iter().all(|x| v[*x]),
                Node::Or(or) => or.children.iter().any(|x| v[*x]),
            };
            if nv {
                v[i] = true;
                changed = true;
            }
        }
        if !changed {
            return v;
        }
    }
}

/// EVD.38: S and neg S, closed under children, discharge edges and negation pairs of premises.
pub fn brute_reach(g: &Graph, spec: &Spec, k: usize) -> BTreeSet<NodeId> {
    let neg = |k: usize| -> Option<usize> {
        if !spec.paired {
            None
        } else if k < spec.base {
            Some(k + spec.base)
        } else {
            Some(k - spec.base)
        }
    };
    let key_index = |key: &Key| spec.keys().iter().position(|x| x == key);
    let mut seen = BTreeSet::new();
    let mut stack = vec![g.root_of(&spec.key(k)).expect("root")];
    if let Some(n) = neg(k) {
        stack.push(g.root_of(&spec.key(n)).expect("root"));
    }
    while let Some(id) = stack.pop() {
        if !seen.insert(id) {
            continue;
        }
        match &g.nodes()[id] {
            Node::Or(or) => stack.extend(or.children.iter().copied()),
            Node::And(c) => stack.extend(c.iter().copied()),
            Node::Leaf(_) => {}
            Node::Premise(p) => {
                let pk = key_index(&p.key).expect("premise key");
                stack.push(g.root_of(&p.key).expect("root"));
                if let Some(n) = neg(pk) {
                    stack.push(g.root_of(&spec.key(n)).expect("root"));
                }
            }
        }
    }
    seen
}

/// Sat(a) membership (EVD.23): L and D are disjoint and Pi lies inside T.
pub fn in_sat(a: &Alternative, distrusted: &dyn Fn(&Grade) -> bool, assumed: &BTreeSet<Key>) -> bool {
    a.grades.iter().all(|g| !distrusted(g)) && a.premises.iter().all(|p| assumed.contains(p))
}

impl Spec {
    pub fn premise_keys(&self) -> BTreeSet<Key> {
        self.nodes
            .iter()
            .filter_map(|n| match n {
                SpecNode::Premise(k) => Some(self.key(*k)),
                _ => None,
            })
            .collect()
    }
}

/// The scenarios that separate every val over a spec: D = the down-closure of Y for each subset Y of the grades used,
/// and T each subset of the premise keys used. Two vals that agree on these agree on every scenario, because a val
/// reads D only through D's intersection with the grades used, and that intersection is always such a Y.
pub fn scenarios(spec: &Spec) -> Vec<(BTreeSet<Grade>, BTreeSet<Key>)> {
    scenarios_over(&spec.grades(), &spec.premise_keys())
}

pub fn scenarios_over(grades: &BTreeSet<Grade>, premises: &BTreeSet<Key>) -> Vec<(BTreeSet<Grade>, BTreeSet<Key>)> {
    let used: Vec<Grade> = grades.iter().cloned().collect();
    let prem: Vec<Key> = premises.iter().cloned().collect();
    let all = all_grades();
    let mut out = Vec::new();
    for ymask in 0..(1u32 << used.len()) {
        let y: Vec<&Grade> = (0..used.len()).filter(|i| ymask & (1 << i) != 0).map(|i| &used[i]).collect();
        let d: BTreeSet<Grade> = all.iter().filter(|g| y.iter().any(|yy| t_le(g, yy))).cloned().collect();
        for tmask in 0..(1u32 << prem.len()) {
            let t: BTreeSet<Key> =
                (0..prem.len()).filter(|i| tmask & (1 << i) != 0).map(|i| prem[i].clone()).collect();
            out.push((d.clone(), t));
        }
    }
    out
}

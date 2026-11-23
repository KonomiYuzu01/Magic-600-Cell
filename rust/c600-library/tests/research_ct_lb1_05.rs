//! CT-LB1-05 (chapter 03's test table; LB1.8's rows "Finite fixed points" and "Exact matching and integral flows"):
//! fixed points satisfy their finite operator, matching and integral-flow witnesses satisfy every edge and demand
//! constraint, and small brute-force instances agree on infeasibility.
//!
//! The entries are language code, the research library's modules `research_fixpoints` and `research_flows`, run on
//! `c600-run` with no host, on seeded synthetic graphs and networks small enough for the test to enumerate every
//! subset, matching and integral flow in Rust. Regional-family recognition, which matches through `research_flows`,
//! runs in `research_entries.rs`. A pass is source and fixture evidence.
//!
//! CT-LB1-05 (source, part: the library half, fixed points, matching and integral flows on synthetic instances; not
//! run here: the matching half over stage 19's stored Γ(Y) certificate, which is R6's).

mod research_common;

use c600_library::research::{FIXPOINTS_MODULE, FIXPOINTS_SOURCE, FLOWS_MODULE, FLOWS_SOURCE};
use c600_run::interp::Loaded;
use c600_run::value::Value;
use research_common::*;

#[derive(Clone, Debug)]
struct Node {
    and: bool,
    children: Vec<usize>,
}

fn nodes_value(nodes: &[Node]) -> Value {
    list(
        nodes
            .iter()
            .map(|n| {
                let children = n.children.iter().map(|&c| c as u64).collect::<Vec<_>>();
                record(vec![("children", nats(&children)), ("kind", text(if n.and { "and" } else { "or" }))])
            })
            .collect(),
    )
}

/// The operator on a subset given as a bit mask.
fn operator(nodes: &[Node], x: u32) -> u32 {
    let inside = |c: &usize| x & (1 << c) != 0;
    nodes.iter().enumerate().fold(0, |out, (v, n)| {
        let holds = if n.and { n.children.iter().all(inside) } else { n.children.iter().any(inside) };
        if holds {
            out | (1 << v)
        } else {
            out
        }
    })
}

fn members(x: u32, n: usize) -> Vec<u64> {
    (0..n as u64).filter(|&v| x & (1 << v) != 0).collect()
}

/// Fixed points satisfy their finite operator: on 80 seeded AND/OR graphs of up to 9 nodes, the least and greatest
/// fixed points equal the test's Knaster-Tarski answers over all 2^n subsets (the intersection of every set the
/// operator maps into itself, and the union of every set inside its image), the operator maps each to itself in Rust
/// and in the language, and `holds` says so. A set that is not fixed is told apart.
#[test]
fn ct_lb1_05_fixed_points_satisfy_their_operator() {
    let loaded = loaded(&[FIXPOINTS_SOURCE]);
    let mut g = Fixtures::new(0x5170_0105);
    let mut empty_least = 0;
    for _ in 0..80 {
        let n = 1 + g.below(9) as usize;
        let nodes: Vec<Node> = (0..n)
            .map(|_| Node {
                and: g.below(2) == 0,
                children: (0..g.below(4)).map(|_| g.below(n as u64) as usize).collect(),
            })
            .collect();
        let full = (1u32 << n) - 1;
        let least = (0..=full).filter(|&x| operator(&nodes, x) & !x == 0).fold(full, |a, x| a & x);
        let greatest = (0..=full).filter(|&x| x & !operator(&nodes, x) == 0).fold(0, |a, x| a | x);
        assert_eq!(operator(&nodes, least), least);
        assert_eq!(operator(&nodes, greatest), greatest);
        empty_least += usize::from(least == 0);
        let value = nodes_value(&nodes);
        for (function, expected) in [("least_fixed_point", least), ("greatest_fixed_point", greatest)] {
            let answer = call(&loaded, FIXPOINTS_MODULE, function, vec![value.clone()]);
            assert_eq!(u64s(field(&answer, "members")), members(expected, n), "{function} of {nodes:?}");
            assert!(as_bool(field(&answer, "holds")));
            assert!(as_u64(field(&answer, "rounds")) <= n as u64);
            let fixed =
                call(&loaded, FIXPOINTS_MODULE, "is_fixed_point", vec![value.clone(), nats(&members(expected, n))]);
            assert!(as_bool(&fixed));
            let image =
                call(&loaded, FIXPOINTS_MODULE, "apply_operator", vec![value.clone(), nats(&members(expected, n))]);
            assert_eq!(u64s(&image), members(expected, n));
        }
        if let Some(x) = (0..=full).find(|&x| operator(&nodes, x) != x) {
            let fixed = call(&loaded, FIXPOINTS_MODULE, "is_fixed_point", vec![value.clone(), nats(&members(x, n))]);
            assert!(!as_bool(&fixed), "{x:b} is not fixed");
            let image = call(&loaded, FIXPOINTS_MODULE, "apply_operator", vec![value, nats(&members(x, n))]);
            assert_eq!(u64s(&image), members(operator(&nodes, x), n));
        }
    }
    assert!(empty_least > 0 && empty_least < 80, "the fixtures include empty and non-empty least fixed points");
}

fn edges_value(edges: &[(u64, u64)]) -> Value {
    list(edges.iter().map(|&(l, r)| record(vec![("left", nat(l)), ("right", nat(r))])).collect())
}

/// The largest matching by enumerating every subset of the edges.
fn brute_matching(edges: &[(u64, u64)]) -> usize {
    (0u32..1 << edges.len())
        .filter(|mask| {
            let chosen: Vec<_> = (0..edges.len()).filter(|i| mask & (1 << i) != 0).map(|i| edges[i]).collect();
            let lefts: std::collections::BTreeSet<_> = chosen.iter().map(|e| e.0).collect();
            let rights: std::collections::BTreeSet<_> = chosen.iter().map(|e| e.1).collect();
            lefts.len() == chosen.len() && rights.len() == chosen.len()
        })
        .map(|mask| mask.count_ones() as usize)
        .max()
        .unwrap_or(0)
}

fn pairs(value: &Value) -> Vec<(u64, u64)> {
    items(value).iter().map(|p| (as_u64(field(p, "left")), as_u64(field(p, "right")))).collect()
}

/// Matching witnesses satisfy every edge, and brute force agrees on infeasibility: on 120 seeded bipartite graphs of up
/// to 4 by 4 vertices and 12 edges, `max_matching` answers a matching of the brute-force maximum size whose pairs are
/// edges with distinct ends (checked in Rust and by `check_matching`); `perfect` agrees with the brute force, so a
/// graph with no perfect matching is answered as such. A pair that is no edge, a repeated vertex and a vertex out of
/// range each fail `check_matching`.
#[test]
fn ct_lb1_05_matching_witnesses_satisfy_every_edge() {
    let loaded = loaded(&[FLOWS_SOURCE]);
    let mut g = Fixtures::new(0x5170_1105);
    let (mut perfect, mut imperfect) = (0, 0);
    for _ in 0..120 {
        let (left, right) = (1 + g.below(4), 1 + g.below(4));
        let mut edges: Vec<(u64, u64)> = (0..g.below(13)).map(|_| (g.below(left), g.below(right))).collect();
        edges.sort();
        edges.dedup();
        let answer = call(&loaded, FLOWS_MODULE, "max_matching", vec![nat(left), nat(right), edges_value(&edges)]);
        let matching = pairs(field(&answer, "matching"));
        let best = brute_matching(&edges);
        assert_eq!(matching.len(), best, "{left}x{right} {edges:?}");
        assert_eq!(as_u64(field(&answer, "size")), best as u64);
        assert!(matching.iter().all(|e| edges.contains(e)));
        let lefts: std::collections::BTreeSet<_> = matching.iter().map(|e| e.0).collect();
        let rights: std::collections::BTreeSet<_> = matching.iter().map(|e| e.1).collect();
        assert!(lefts.len() == matching.len() && rights.len() == matching.len());
        let is_perfect = left == right && best as u64 == left;
        assert_eq!(as_bool(field(&answer, "perfect")), is_perfect);
        if is_perfect {
            perfect += 1
        } else {
            imperfect += 1
        }
        let witness = field(&answer, "matching").clone();
        let check = |m: Value| {
            as_bool(&call(&loaded, FLOWS_MODULE, "check_matching", vec![nat(left), nat(right), edges_value(&edges), m]))
        };
        assert!(check(witness));
        if let Some(&(l, r)) = matching.first() {
            if let Some(other) = (0..right).find(|&o| !edges.contains(&(l, o))) {
                assert!(!check(edges_value(&[(l, other)])), "a pair that is no edge");
            }
            assert!(!check(edges_value(&[(l, r), (l, r)])), "a repeated vertex");
            assert!(!check(edges_value(&[(left, r)])), "a vertex out of range");
        }
    }
    assert!(perfect > 0 && imperfect > 0, "{perfect} perfect and {imperfect} imperfect instances");
}

#[derive(Clone, Debug)]
struct Network {
    nodes: u64,
    edges: Vec<(u64, u64, u64)>,
    supply: Vec<u64>,
    demand: Vec<u64>,
}

impl Network {
    fn args(&self) -> Vec<Value> {
        let edges = self
            .edges
            .iter()
            .map(|&(from, to, capacity)| {
                record(vec![("capacity", nat(capacity)), ("from", nat(from)), ("to", nat(to))])
            })
            .collect();
        vec![nat(self.nodes), list(edges), nats(&self.supply), nats(&self.demand)]
    }

    /// Whether integral `flows` satisfy every capacity and every node's supply and demand.
    fn satisfied_by(&self, flows: &[i64]) -> bool {
        let mut balance = vec![0i64; self.nodes as usize];
        for (&(from, to, capacity), &f) in self.edges.iter().zip(flows) {
            if f < 0 || f > capacity as i64 {
                return false;
            }
            balance[from as usize] += f;
            balance[to as usize] -= f;
        }
        flows.len() == self.edges.len()
            && (0..self.nodes as usize).all(|v| balance[v] == self.supply[v] as i64 - self.demand[v] as i64)
    }

    /// Brute force: whether any integral flow satisfies every constraint.
    fn feasible(&self) -> bool {
        let mut flows = vec![0i64; self.edges.len()];
        loop {
            if self.satisfied_by(&flows) {
                return true;
            }
            let mut k = 0;
            loop {
                if k == flows.len() {
                    return false;
                }
                if flows[k] < self.edges[k].2 as i64 {
                    flows[k] += 1;
                    break;
                }
                flows[k] = 0;
                k += 1;
            }
        }
    }

    /// The capacity of the cut whose source side is the super source and `side`, in the network with a super source
    /// feeding each supply and a super sink taking each demand.
    fn cut_capacity(&self, side: &[u64]) -> u64 {
        let inside = |v: u64| side.contains(&v);
        let edges: u64 = self.edges.iter().filter(|e| inside(e.0) && !inside(e.1)).map(|e| e.2).sum();
        let sources: u64 = (0..self.nodes).filter(|&v| !inside(v)).map(|v| self.supply[v as usize]).sum();
        let sinks: u64 = (0..self.nodes).filter(|&v| inside(v)).map(|v| self.demand[v as usize]).sum();
        edges + sources + sinks
    }
}

/// Integral-flow witnesses satisfy every edge and demand, and brute force agrees on infeasibility: on 150 seeded
/// networks of up to 4 nodes and 5 edges with capacities, supplies and demands up to 2, `feasible_flow` answers
/// feasibility as the brute force over every integral flow does. A feasible answer's flows satisfy every capacity and
/// every node's balance in Rust and by `check_flow`; an infeasible answer's flow value equals the capacity of its
/// residual cut (so no flow is larger), which is below the total supply or the supplies and demands differ. A flow
/// over a capacity or off balance fails `check_flow`.
#[test]
fn ct_lb1_05_integral_flow_witnesses_satisfy_every_demand() {
    let loaded = loaded(&[FLOWS_SOURCE]);
    let mut g = Fixtures::new(0x5170_2105);
    let (mut feasible, mut infeasible) = (0, 0);
    for _ in 0..150 {
        let nodes = 1 + g.below(4);
        let edges = (0..g.below(6)).map(|_| (g.below(nodes), g.below(nodes), g.below(3))).collect();
        let supply = (0..nodes).map(|_| g.below(3)).collect();
        let demand = (0..nodes).map(|_| g.below(3)).collect();
        let network = Network { nodes, edges, supply, demand };
        let answer = call(&loaded, FLOWS_MODULE, "feasible_flow", network.args());
        let expected = network.feasible();
        assert_eq!(as_bool(field(&answer, "feasible")), expected, "{network:?}");
        let flows = i64s(field(&answer, "flows"));
        let value = as_u64(field(&answer, "value"));
        let required: u64 = network.supply.iter().sum();
        assert_eq!(as_u64(field(&answer, "required")), required);
        let cut = u64s(field(&answer, "cut"));
        assert_eq!(network.cut_capacity(&cut), value, "max flow equals the residual cut: {network:?}");
        let mut args = network.args();
        args.push(field(&answer, "flows").clone());
        let checked = as_bool(&call(&loaded, FLOWS_MODULE, "check_flow", args));
        if expected {
            feasible += 1;
            assert!(network.satisfied_by(&flows) && checked, "{network:?} {flows:?}");
            assert_eq!(value, required);
        } else {
            infeasible += 1;
            assert!(!checked);
            assert!(value < required || required != network.demand.iter().sum::<u64>());
        }
        if let Some(k) = (0..network.edges.len()).find(|&k| network.edges[k].2 < 2) {
            let mut over = flows.clone();
            over[k] = network.edges[k].2 as i64 + 1;
            let mut args = network.args();
            args.push(list(over.iter().map(|&f| int(f)).collect()));
            assert!(!as_bool(&call(&loaded, FLOWS_MODULE, "check_flow", args)), "a flow over its capacity");
        }
    }
    assert!(feasible > 10 && infeasible > 10, "{feasible} feasible and {infeasible} infeasible networks");
}

/// The matching entries refuse a malformed graph rather than answer about another one: an edge out of range stops
/// `max_matching` at its pointwise assertion (`LG4:input.pointwise_failed`) with no value.
#[test]
fn ct_lb1_05_out_of_range_input_stops_with_no_answer() {
    let loaded: Loaded = loaded(&[FLOWS_SOURCE]);
    let run = run_with(
        &loaded,
        FLOWS_MODULE,
        "max_matching",
        vec![nat(2), nat(2), edges_value(&[(0, 0), (2, 1)])],
        &c600_run::interp::Options::new(WIDE),
    );
    assert_eq!(run.summary.code, Some("LG4:input.pointwise_failed"));
    assert!(run.value.is_none());
}

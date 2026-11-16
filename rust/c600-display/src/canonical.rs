//! The canonical form of a publication and of a request. It gives the ResultRef digest that identifies the
//! canonical publication (RM.20) and the canonical uncompressed byte count charged under RM.13 and LI.16: the
//! canonical JSON of the metadata, with each array by its RM.6 descriptor, plus every array's bytes and every pinned
//! source's bytes. Chapter 05 and LI.16 own the stored and output byte forms; this is a draft stand-in for them.

use c600_digest::sha256_hex;
use c600_kernel::json::{object, Value};
use c600_kernel::{IdentityBlock, Puzzle};

use crate::array::Array;
use crate::plan::*;

fn n(value: impl ToString) -> Value {
    Value::Number(value.to_string())
}

fn s(text: &str) -> Value {
    Value::String(text.to_owned())
}

fn list<T>(items: &[T], f: impl Fn(&T) -> Value) -> Value {
    Value::Array(items.iter().map(f).collect())
}

fn opt<T>(item: &Option<T>, f: impl Fn(&T) -> Value) -> Value {
    item.as_ref().map_or(Value::Null, f)
}

fn identity(block: &IdentityBlock) -> Value {
    object([
        ("puzzle", s(if block.puzzle == Puzzle::Full { "full" } else { "jumbling" })),
        ("model_id", s(&block.model_id)),
        ("menu_id", opt(&block.menu_id, |v| s(v))),
        ("menu_order", opt(&block.menu_order, |v| s(v))),
        ("contract_rev", opt(&block.contract_rev, |v| s(v))),
    ])
}

pub(crate) fn revision(rev: &RevisionRef) -> Value {
    object([
        ("session", s(&rev.session)),
        ("epoch", n(rev.epoch)),
        ("identity", identity(&rev.identity)),
        ("mode", s(if rev.mode == Mode::JumbleOff { "jumble_off" } else { "jumble_on" })),
        ("head", n(rev.head)),
        ("state", n(rev.state)),
        ("workspace", n(rev.workspace)),
        ("protection", n(rev.protection)),
        ("digest", s(&rev.digest)),
    ])
}

fn state(state: &StateRef) -> Value {
    match state {
        StateRef::Snapshot(rev) => object([("snapshot", revision(rev))]),
        StateRef::Derived(derived) => object([(
            "derived",
            object([
                ("digest", s(&derived.digest)),
                ("level", s(if derived.level == Level::Labels { "labels" } else { "poses" })),
                ("identity", identity(&derived.identity)),
            ]),
        )]),
    }
}

fn lineage(lineage: &Lineage) -> Value {
    match lineage {
        Lineage::Derivation { base, word } => {
            object([("derivation", object([("base", opt(base, state)), ("word", s(word))]))])
        }
        Lineage::Imported { import, comparison_base } => {
            object([("imported", object([("import", s(import)), ("comparison_base", opt(comparison_base, revision))]))])
        }
    }
}

fn array(array: &Array) -> Value {
    object([
        ("elem", s(array.desc.elem.name())),
        ("shape", list(&array.desc.shape, |d| n(d))),
        ("byte_len", n(array.desc.byte_len)),
        ("digest", s(&array.desc.digest)),
    ])
}

fn strings(items: &[String]) -> Value {
    list(items, |v| s(v))
}

fn object_id(object_id: &ObjectId) -> Value {
    s(&object_text(object_id))
}

fn link(link: &SourceLink) -> Value {
    match link {
        SourceLink::Range { digest, start, end } => {
            object([("digest", s(digest)), ("start", n(start)), ("end", n(end))])
        }
        SourceLink::Missing { digest, reason } => object([("digest", s(digest)), ("source_missing", s(reason))]),
    }
}

fn layout(layout: &LayoutInput) -> Value {
    let kind = match &layout.kind {
        LayoutKind::Sheet => s("sheet"),
        LayoutKind::Atlas { membership } => object([("atlas", list(membership, |c| n(c.0)))]),
        LayoutKind::Tree { roots, parent_child } => {
            object([("tree", object([("roots", list(roots, |r| n(r))), ("parent_child", array(parent_child))]))])
        }
        LayoutKind::Complex(incidence) => object([(
            "complex",
            object([
                ("dims", array(&incidence.dims)),
                ("offsets", array(&incidence.offsets)),
                ("boundary", array(&incidence.boundary)),
                ("orientation", array(&incidence.orientation)),
            ]),
        )]),
        LayoutKind::Graph(graph) => object([(
            "graph",
            object([
                ("subjects", list(&graph.subjects, state)),
                ("edges", array(&graph.edges)),
                ("transitions", strings(&graph.transitions)),
            ]),
        )]),
    };
    object([
        ("instances", list(&layout.instances, |i| n(i.0))),
        ("positions", array(&layout.positions)),
        ("edges", opt(&layout.edges, array)),
        ("triangles", opt(&layout.triangles, array)),
        ("kind", kind),
    ])
}

fn point(point: &CertPoint) -> Value {
    object([("exact", s(&point.exact)), ("vector", list(&point.vector, |x| n(format!("{x:?}"))))])
}

fn primitive(item: &Primitive) -> Value {
    let body = match item {
        Primitive::Retained(r) => object([
            ("labels", array(&r.labels)),
            ("visibility", array(&r.visibility)),
            ("interaction", array(&r.interaction)),
            ("tokens", strings(&r.tokens)),
        ]),
        Primitive::Posed(p) => object([
            ("pose_index", array(&p.pose_index)),
            ("poses", array(&p.poses)),
            ("pose_lattice", array(&p.pose_lattice)),
        ]),
        Primitive::Transform(t) => object([
            ("grip", n(t.grip)),
            ("plane", opt(&t.plane, array)),
            ("angle", n(format!("{:?}", t.angle))),
            ("moving", array(&t.moving)),
            ("moving_digest", s(&t.moving_digest)),
            ("from", n(t.from)),
            ("to", n(t.to)),
        ]),
        Primitive::ChartStage(stage) => object([("chart", n(stage.chart.0))]),
        Primitive::UserStage(stage) => object([
            ("kind", s(if stage.kind == StageKind::Vertex { "vertex" } else { "fragment" })),
            ("digest", s(&stage.digest)),
            ("compile_record", s(&stage.compile_record)),
        ]),
        Primitive::Layout(l) => layout(l),
        Primitive::CertOverlay(c) => object([
            ("grip", n(c.grip)),
            ("cap", n(c.cap)),
            ("kind", s(if c.kind == CertKind::FullNamedCapSet { "full_named_cap_set" } else { "cap_map_witness" })),
            ("pieces", list(&c.pieces, |p| n(p))),
            ("set_digest", s(&c.set_digest)),
            ("pairs", list(&c.pairs, |pair| object([("piece", n(pair.piece)), ("points", list(&pair.points, point))]))),
            ("survey", list(&c.survey, |g| s(&format!("{g:?}")))),
            (
                "cut",
                opt(&c.cut, |cut| {
                    object([
                        ("pole", n(cut.pole)),
                        ("plane", s(&cut.plane)),
                        ("alpha", s(&format!("{}/{}", cut.alpha.0, cut.alpha.1))),
                    ])
                }),
            ),
        ]),
        Primitive::SetOverlay(o) => object([
            (
                "kind",
                match &o.kind {
                    OverlayKind::Set => s("set"),
                    OverlayKind::Relation { claim } => object([("relation", opt(claim, |k| s(&k.0)))]),
                },
            ),
            ("objects", list(&o.objects, object_id)),
            ("set_digest", s(&o.set_digest)),
            (
                "coverage",
                match &o.coverage {
                    Coverage::Complete => s("complete"),
                    Coverage::Partial { missing } => object([("partial", strings(missing))]),
                    Coverage::Undefined(reason) => object([("undefined", s(reason))]),
                },
            ),
            (
                "predicate",
                opt(&o.predicate, |p| match p {
                    Predicate::Section(d) => object([("section", s(d))]),
                    Predicate::Slab(d) => object([("slab", s(d))]),
                    Predicate::Clip(d) => object([("clip", s(d))]),
                }),
            ),
            ("positions", opt(&o.positions, array)),
            ("lines", opt(&o.lines, array)),
            ("triangles", opt(&o.triangles, array)),
            ("tokens", strings(&o.tokens)),
        ]),
        Primitive::Tokens(t) => object([
            ("table", s(&t.table)),
            ("names", list(&t.names, |(class, name)| Value::Array(vec![s(class), s(name)]))),
        ]),
        Primitive::Derived(d) => object([
            ("subject", state(&StateRef::Derived(d.subject.clone()))),
            ("lineage", opt(&d.lineage, lineage)),
            ("reachability", s(&d.reachability)),
            ("primitives", list(&d.primitives, primitive)),
        ]),
        Primitive::Preview(p) => object([
            ("preview", n(p.preview.0)),
            ("base", revision(&p.base)),
            ("literal", s(&p.literal)),
            ("effect", s(&p.effect)),
            ("primitives", list(&p.primitives, primitive)),
        ]),
    };
    object([("row", s(item.feature())), ("payload", body)])
}

fn chart(chart: &Chart) -> Value {
    object([
        ("chart", n(chart.chart.0)),
        (
            "stages",
            list(&chart.stages, |stage| {
                object([
                    ("name", s(&stage.name)),
                    ("version", n(stage.version)),
                    ("definition", s(&stage.definition)),
                    ("params", list(&stage.params, |p| n(format!("{p:?}")))),
                ])
            }),
        ),
        ("dimension", n(chart.dimension)),
        ("precision", s(&chart.precision)),
        ("properties", strings(&chart.properties)),
    ])
}

fn plan(plan: &DisplayPlan) -> Value {
    object([
        ("plan", n(plan.plan.0)),
        ("base", revision(&plan.base)),
        ("subject", state(&plan.subject)),
        ("derivation", opt(&plan.derivation, lineage)),
        ("reachability", opt(&plan.reachability, |r| s(r))),
        ("dimension", n(plan.dimension)),
        (
            "geometry",
            object([
                ("identity", identity(&plan.geometry.identity)),
                ("digests", Value::Object(plan.geometry.digests.iter().map(|(k, v)| (k.clone(), s(v))).collect())),
            ]),
        ),
        ("primitives", list(&plan.primitives, primitive)),
        ("charts", list(&plan.charts, chart)),
        (
            "instances",
            list(&plan.instances, |i| {
                object([
                    ("instance", n(i.instance.0)),
                    ("plan", n(i.plan.0)),
                    ("object", object_id(&i.object)),
                    ("primitive", n(i.primitive)),
                    ("piece", opt(&i.piece, |p| n(p))),
                    ("interaction", Value::Bool(i.interaction)),
                ])
            }),
        ),
        ("source_map", n(plan.source_map.0)),
        ("precision", s(&plan.precision)),
        ("dependencies", Value::Object(plan.dependencies.iter().map(|(k, v)| (k.clone(), s(v))).collect())),
    ])
}

fn node(node: &ViewNode) -> Value {
    object([
        ("node", n(node.node.0)),
        ("base", revision(&node.base)),
        ("kind", s(&format!("{:?}", node.kind).to_lowercase())),
        (
            "content",
            list(&node.content, |c| match c {
                Content::State { row, digest } => object([("state", object([("row", s(row)), ("digest", s(digest))]))]),
                Content::Word(key) => object([("word", s(&key.0))]),
                Content::Text(text) => object([("text", s(text))]),
            }),
        ),
        ("plans", list(&node.plans, |p| n(p.0))),
        ("children", list(&node.children, |c| n(c.0))),
        ("source", opt(&node.source, link)),
        ("commands", strings(&node.commands)),
        ("claims", list(&node.claims, |k| s(&k.0))),
    ])
}

fn map(map: &SourceMap) -> Value {
    object([
        ("map", n(map.map.0)),
        ("plan", n(map.plan.0)),
        ("base", revision(&map.base)),
        ("sources", strings(&map.sources)),
        (
            "entries",
            Value::Object(
                map.entries
                    .iter()
                    .map(|(id, entry)| {
                        (
                            id.0.to_string(),
                            object([("object", object_id(&entry.object)), ("source", link(&entry.source))]),
                        )
                    })
                    .collect(),
            ),
        ),
    ])
}

/// The canonical metadata of a publication.
pub fn publication(publication: &Publication) -> Value {
    object([
        ("target", n(publication.target.0)),
        ("base", revision(&publication.base)),
        ("nodes", list(&publication.nodes, node)),
        ("plans", list(&publication.plans, plan)),
        ("maps", list(&publication.maps, map)),
        (
            "sources",
            Value::Object(publication.sources.iter().map(|(digest, bytes)| (digest.clone(), n(bytes.len()))).collect()),
        ),
    ])
}

fn payload_bytes(primitive: &Primitive) -> u64 {
    fn arrays(items: &[&Array]) -> u64 {
        items.iter().map(|a| a.desc.byte_len).sum()
    }
    match primitive {
        Primitive::Retained(r) => arrays(&[&r.labels, &r.visibility, &r.interaction]),
        Primitive::Posed(p) => arrays(&[&p.pose_index, &p.poses, &p.pose_lattice]),
        Primitive::Transform(t) => arrays(&[&t.moving]) + t.plane.as_ref().map_or(0, |p| p.desc.byte_len),
        Primitive::Layout(l) => {
            let mut total = l.positions.desc.byte_len;
            total += l.edges.as_ref().map_or(0, |a| a.desc.byte_len);
            total += l.triangles.as_ref().map_or(0, |a| a.desc.byte_len);
            total
                + match &l.kind {
                    LayoutKind::Tree { parent_child, .. } => parent_child.desc.byte_len,
                    LayoutKind::Complex(i) => arrays(&[&i.dims, &i.offsets, &i.boundary, &i.orientation]),
                    LayoutKind::Graph(g) => g.edges.desc.byte_len,
                    _ => 0,
                }
        }
        Primitive::SetOverlay(o) => {
            [&o.positions, &o.lines, &o.triangles].iter().map(|a| a.as_ref().map_or(0, |a| a.desc.byte_len)).sum()
        }
        Primitive::Derived(d) => d.primitives.iter().map(payload_bytes).sum(),
        Primitive::Preview(p) => p.primitives.iter().map(payload_bytes).sum(),
        _ => 0,
    }
}

/// The input bytes of RM.11: the sum of the descriptor byte lengths.
pub fn input_bytes(publication: &Publication) -> u64 {
    publication.plans.iter().flat_map(|p| p.primitives.iter()).map(payload_bytes).sum()
}

/// The canonical uncompressed byte count and the digest of a publication.
pub fn measure(publication: &Publication) -> (u64, String) {
    let text = self::publication(publication).canonical();
    let sources: u64 = publication.sources.values().map(|b| b.len() as u64).sum();
    (text.len() as u64 + input_bytes(publication) + sources, sha256_hex(text.as_bytes()))
}

/// The request digest of a standalone publication (KN5.21): its canonical metadata.
pub fn request_digest(publication: &Publication) -> String {
    sha256_hex(self::publication(publication).canonical().as_bytes())
}

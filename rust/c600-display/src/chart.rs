//! Exact chart-property checks at exact camera poses (RPI.28 to RPI.30, RPI.34, LB4.1, LB4.2) and the admission of a
//! chart against its check results (RPI.48).
//!
//! The repertoire is a restricted family with known checkers, RPI.34's first option, as a draft: the stereographic
//! chart, declared exactly by its pole ([`Stereographic`]), and the projective charts x ↦ (M·x + t)/(c·x + d), affine
//! maps and central projections among them ([`Projective`]). A region is a union of cells, each the convex hull of
//! exact vertices in Q(√5) ([`Region`]). An exact pose is the identity or a verified element of K⁺; a float camera
//! matrix is refused, never rounded into a pose (RPI.30).
//!
//! Each property is decided by its own check over the whole region: never inferred from another property, and never
//! established from the vertices alone ([`vertex_test`] can only refute). The region checks are KN3.3's exact polytope
//! operations in `c600_jumble::polytope`:
//!
//! - domain: for the stereographic chart, the region test `Polytope::ray_meet` from the origin along the pole; for a
//!   projective chart, the sign of its denominator at the posed vertices, to which convexity reduces it;
//! - injectivity on each cell, and embedding, which is injectivity on the union of the cells: `polytope::injective` and
//!   `polytope::embedding`, for the stereographic chart through the reduction to a central projection
//!   ([`CENTRAL_PROJECTION`]);
//! - isometry on each cell, and homomorphism, each cell onto a cell of the codomain: `polytope::isometry` and
//!   `polytope::homomorphism`, for a projective chart. A stereographic chart is not a projective map, so these two have
//!   no exact checker for it and stay `not_established`, never false;
//! - section membership: the posed vertices, to which convexity reduces it.
//!
//! The reduction needs a functional h positive on the region, one h for all cells of an embedding; a region through
//! the origin has none, and its injectivity stays `not_established`. Where h·x > 0 and off the singular ray, the chart
//! and x ↦ x/(h·x) identify exactly the points of one open ray. So an injective central projection establishes the
//! chart's injectivity on the part of the region in the domain, and the central projection's pair of points, which lie
//! on one open ray, refutes it unless that ray is the singular ray, whose points are outside the domain and witness
//! nothing. On a region that misses the closed singular ray, which the domain check decides, the two are equivalent.
//! Every verdict records the property, region, vertex count and pose it covers.

use std::collections::BTreeSet;

use c600_digest::sha256_hex;
use c600_jumble::polytope::{self, Check, Metric, Point, Polytope, PolytopeError, ProjectiveMap, SameImage};
use c600_jumble::q5::Q5 as Big;
use c600_kernel::json::Value;

use crate::error::{codes, Error};
use crate::exact::{self, M4, Q5, V4};
use crate::plan::{Chart, ChartId};

/// An exact camera pose (RPI.30): the identity or an element of K⁺, verified when it is made.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pose {
    matrix: Option<M4>,
}

impl Pose {
    pub fn identity() -> Pose {
        Pose { matrix: None }
    }

    /// The pose of an exact matrix, accepted only for an element of K⁺. Any other matrix is not an exact pose, and a
    /// check offered one refuses it as approximate input (a draft).
    pub fn rotation(matrix: M4) -> Result<Pose, Error> {
        match exact::in_k_plus(&matrix) {
            Some(true) => Ok(Pose { matrix: Some(matrix) }),
            Some(false) => Err(approximate("the matrix is not an element of K⁺")),
            None => Err(approximate("the matrix overflows exact arithmetic")),
        }
    }

    pub fn matrix(&self) -> M4 {
        self.matrix.unwrap_or_else(exact::identity)
    }

    /// The pose's identity: `identity`, or the digest of the exact matrix text.
    pub fn id(&self) -> String {
        match &self.matrix {
            None => "identity".to_owned(),
            Some(m) => {
                let rows: Vec<String> = m.iter().map(exact::text).collect();
                format!("k+:{}", sha256_hex(rows.join(";").as_bytes()))
            }
        }
    }

    fn apply(&self, v: &V4) -> Option<V4> {
        match &self.matrix {
            None => Some(*v),
            Some(m) => exact::apply(m, v),
        }
    }
}

/// A camera pose as the host holds it: an exact pose, or a float matrix between exact poses with the declared text of
/// how it was made.
#[derive(Clone, Debug, PartialEq)]
pub enum CameraPose {
    Exact(Pose),
    Approximate { label: String, matrix: [[f64; 4]; 4] },
}

fn approximate(summary: &str) -> Error {
    Error::new(codes::APPROXIMATE_PROPERTY_INPUT, summary)
}

/// A region given exactly by its vertices: the union of its cells, each the convex hull of the vertices it lists. A
/// region that lists no cells is one cell, the convex hull of all its vertices.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Region {
    pub name: String,
    pub vertices: Vec<V4>,
    /// Each cell's vertices, as indices into `vertices`. Every vertex must be in a cell, so that a vertex test reads
    /// points of the region only.
    pub cells: Vec<Vec<usize>>,
}

impl Region {
    /// The convex hull of `vertices`, one cell.
    pub fn hull(name: &str, vertices: Vec<V4>) -> Region {
        Region { name: name.to_owned(), vertices, cells: Vec::new() }
    }

    /// The union of `cells`, each the convex hull of the vertices it lists by index.
    pub fn complex(name: &str, vertices: Vec<V4>, cells: Vec<Vec<usize>>) -> Region {
        Region { name: name.to_owned(), vertices, cells }
    }
}

/// The exact declaration of the stereographic chart that the checks read (RPI.28). Map: x ↦ the stereographic image of
/// x/|x| from the unit pole. Domain: R⁴ without the closed ray {t·pole : t ≥ 0}, the origin included. Inverse: on the
/// image, up to the radius. Loss: the radius |x|, and the singular ray.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Stereographic {
    /// The pole's direction; it need not be a unit vector.
    pub pole: V4,
}

impl Stereographic {
    /// The declaration's identity.
    pub fn id(&self) -> String {
        format!("stereographic/1 pole {}", exact::text(&self.pole))
    }
}

/// The exact declaration of a projective chart (a draft): x ↦ (M·x + t)/(c·x + d) from R⁴ to Rᵐ, defined where
/// c·x + d > 0. An affine map has c = 0 and d = 1; the central projection onto h·x = 1 has M = I, t = 0, c = h and
/// d = 0. A constant negative denominator is read as the same map with every entry negated.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Projective {
    /// The m rows of M.
    pub matrix: Vec<V4>,
    /// t, with m entries.
    pub offset: Vec<Q5>,
    /// c.
    pub denominator: V4,
    /// d.
    pub denominator_offset: Q5,
}

impl Projective {
    /// The affine map x ↦ M·x + t.
    pub fn affine(matrix: Vec<V4>, offset: Vec<Q5>) -> Projective {
        Projective { matrix, offset, denominator: [Q5::ZERO; 4], denominator_offset: Q5::ONE }
    }

    /// The central projection x ↦ x/(h·x) onto the hyperplane h·x = 1, defined where h·x > 0.
    pub fn central(h: V4) -> Projective {
        let matrix = exact::identity().to_vec();
        Projective { matrix, offset: vec![Q5::ZERO; 4], denominator: h, denominator_offset: Q5::ZERO }
    }

    fn map(&self) -> Result<ProjectiveMap, Open> {
        let matrix = big_rows(&self.matrix)?;
        let (offset, denominator) = (big_point(&self.offset)?, big_point(&self.denominator)?);
        Ok(ProjectiveMap::new(matrix, offset, denominator, big(self.denominator_offset)?)?)
    }
}

/// A declared chart property, in camera coordinates after the pose.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Property {
    /// The posed region lies in the chart's domain.
    Domain,
    /// The chart is injective on each cell of the posed region, within its domain.
    Injective,
    /// The chart is injective on the union of the posed region's cells, within its domain: on this compact union, an
    /// embedding.
    Embedding,
    /// The chart keeps distances on each cell of the posed region, from the `source` metric on R⁴ to the `target`
    /// metric on its image space, each given by the rows of a symmetric positive definite Gram matrix.
    Isometry { source: Vec<Vec<Q5>>, target: Vec<Vec<Q5>> },
    /// The chart maps each cell of the posed region onto a cell of the codomain, compared by vertex set. Each codomain
    /// cell is the convex hull of the points it lists, in the chart's image space.
    Homomorphism { codomain: Vec<Vec<Vec<Q5>>> },
    /// The posed region lies in the slab low ≤ normal · x ≤ high.
    Section { normal: V4, low: Q5, high: Q5 },
}

impl Property {
    pub fn name(&self) -> &'static str {
        match self {
            Property::Domain => "domain",
            Property::Injective => "injective",
            Property::Embedding => "embedding",
            Property::Isometry { .. } => "isometry",
            Property::Homomorphism { .. } => "homomorphism",
            Property::Section { .. } => "section",
        }
    }
}

/// The reduction that decides a stereographic chart's injectivity and embedding (`c600_jumble::polytope`'s module
/// doc), cited by the verdicts it reaches.
pub const CENTRAL_PROJECTION: &str =
    "where h·x > 0 and off the singular ray, the chart and x ↦ x/(h·x) identify exactly the points of one open ray";

/// How a verdict was reached.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Method {
    /// The exact region test.
    RegionTest,
    /// The cited reduction to a central projection, whose own `c600_jumble::polytope` check decided the property.
    Reduction(&'static str),
    /// The property's own `c600_jumble::polytope` check, over the whole region.
    PolytopeCheck,
    /// The posed vertices, to which convexity reduces the property.
    PosedVertices,
    /// A per-vertex test, which can refute and cannot establish.
    VertexTest,
}

/// An exact counterexample. Points are posed points, in camera coordinates.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Witness {
    /// A point of the posed region on the singular ray.
    PointOnSingularRay(V4),
    /// A posed vertex on the singular ray.
    VertexOnSingularRay(usize),
    /// Two posed vertices on one ray from the origin, off the singular ray.
    SharedRay(usize, usize),
    /// Two distinct points of the posed region on one open ray from the origin, off the singular ray: both are in the
    /// domain, and the stereographic chart maps them to one point.
    PointsOnOneRay(V4, V4),
    /// A posed vertex outside the section.
    VertexOutside(usize),
    /// A posed vertex where a projective chart's denominator is not positive.
    VertexOutsideDomain(usize),
    /// Two distinct points of the posed region with one image under a projective chart.
    SameImage { x: V4, y: V4, image: Vec<Q5> },
    /// Two points of a cell whose squared distance the chart changes: `before` in the source metric, `after` between
    /// their images in the target metric.
    DistanceChange { x: V4, y: V4, before: Q5, after: Q5 },
    /// A cell whose image is no cell of the codomain, with the image's vertices.
    UnmatchedCell { cell: usize, image: Vec<Vec<Q5>> },
}

/// What a check covers (RPI.30, INV-5): the property it decided, its regions, their vertex count and its poses.
/// Nothing outside it is claimed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Scope {
    /// The name of the property whose own check made the verdict ([`Property::name`]).
    pub property: String,
    pub regions: Vec<String>,
    pub vertices: u64,
    pub poses: Vec<String>,
}

/// The verdict of one check.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Verdict {
    Established { method: Method, scope: Scope },
    Refuted { method: Method, witness: Witness, scope: Scope },
    NotEstablished { reason: String, scope: Scope },
}

impl Verdict {
    pub fn scope(&self) -> &Scope {
        match self {
            Verdict::Established { scope, .. }
            | Verdict::Refuted { scope, .. }
            | Verdict::NotEstablished { scope, .. } => scope,
        }
    }
}

fn not_established(reason: &str, scope: Scope) -> Verdict {
    Verdict::NotEstablished { reason: reason.to_owned(), scope }
}

const OVERFLOW: &str = "exact arithmetic overflowed; the check decides nothing";
const VERTICES_CANNOT_ESTABLISH: &str = "a vertex test can refute a region property and cannot establish one";
const ZERO_POLE: &str = "the declared pole is zero, so the chart is not defined";
const NOT_PROJECTIVE: &str =
    "a stereographic chart is not a projective map, and no exact checker decides this property for it";
const THROUGH_ORIGIN: &str =
    "a cell contains the origin, so no positive functional reduces the chart to a central projection there";
const NO_COMMON_FUNCTIONAL: &str =
    "the origin is in the convex hull of the region, so no one functional is positive on every cell";
const PAIR_ON_SINGULAR_RAY: &str =
    "the central projection's pair lies on the singular ray, outside the domain, and refutes nothing";
const CONVERSION: &str = "an exact value did not convert between the two exact arithmetics";

/// Why a check decided nothing.
enum Open {
    /// An i128 part overflowed, in this crate's arithmetic or in a value read back from `c600_jumble`.
    Overflow,
    /// A polytope operation refused its input.
    Polytope(PolytopeError),
    Reason(&'static str),
}

impl From<PolytopeError> for Open {
    fn from(error: PolytopeError) -> Open {
        Open::Polytope(error)
    }
}

impl Open {
    fn verdict(self, scope: Scope) -> Verdict {
        match self {
            Open::Overflow => not_established(OVERFLOW, scope),
            Open::Polytope(error) => not_established(&format!("the exact polytope check did not run: {error}"), scope),
            Open::Reason(reason) => not_established(reason, scope),
        }
    }
}

/// A decided check: the method, and the witness when the property fails.
struct Decided {
    method: Method,
    witness: Option<Witness>,
}

fn holds(method: Method) -> Decided {
    Decided { method, witness: None }
}

fn fails(method: Method, witness: Witness) -> Decided {
    Decided { method, witness: Some(witness) }
}

fn conclude(decided: Result<Decided, Open>, scope: Scope) -> Verdict {
    match decided {
        Ok(Decided { method, witness: None }) => Verdict::Established { method, scope },
        Ok(Decided { method, witness: Some(witness) }) => Verdict::Refuted { method, witness, scope },
        Err(open) => open.verdict(scope),
    }
}

/// An exact value in `c600_jumble`'s unbounded Q(√5), read from its integer triple.
fn big(x: Q5) -> Result<Big, Open> {
    let (a, b, d) = x.parts();
    let part = |n: i128| Value::Number(n.to_string());
    Big::from_json_any(&Value::Array(vec![part(a), part(b), part(d)])).map_err(|_| Open::Reason(CONVERSION))
}

/// A value of `c600_jumble`'s Q(√5) in checked i128 parts, or an overflow.
fn small(x: &Big) -> Result<Q5, Open> {
    let part = |n: String| n.parse::<i128>().map_err(|_| Open::Overflow);
    Q5::new(part(x.a().to_string())?, part(x.b().to_string())?, part(x.d().to_string())?).ok_or(Open::Overflow)
}

fn big_point(v: &[Q5]) -> Result<Point, Open> {
    v.iter().map(|x| big(*x)).collect()
}

fn big_rows<R: AsRef<[Q5]>>(rows: &[R]) -> Result<Vec<Point>, Open> {
    rows.iter().map(|row| big_point(row.as_ref())).collect()
}

fn small_point(p: &[Big]) -> Result<Vec<Q5>, Open> {
    p.iter().map(small).collect()
}

fn small_v4(p: &[Big]) -> Result<V4, Open> {
    V4::try_from(small_point(p)?).map_err(|_| Open::Reason(CONVERSION))
}

/// The convex hull of one cell's posed vertices.
fn hull(posed: &[V4], cell: &[usize]) -> Result<Polytope, Open> {
    let points = cell.iter().map(|&i| big_point(&posed[i])).collect::<Result<Vec<_>, _>>()?;
    Ok(Polytope::hull(&points)?)
}

fn hulls(posed: &[V4], cells: &[Vec<usize>]) -> Result<Vec<Polytope>, Open> {
    cells.iter().map(|cell| hull(posed, cell)).collect()
}

/// The region's cells as index lists, or why they describe no region: a region has vertices, each cell lists at least
/// one of them, and every vertex is in a cell.
fn cells(region: &Region) -> Result<Vec<Vec<usize>>, Open> {
    let n = region.vertices.len();
    if n == 0 {
        return Err(Open::Reason("the region has no vertices"));
    }
    if region.cells.is_empty() {
        return Ok(vec![(0..n).collect()]);
    }
    let mut used = vec![false; n];
    for cell in &region.cells {
        if cell.is_empty() {
            return Err(Open::Reason("a cell of the region lists no vertices"));
        }
        for &i in cell {
            *used.get_mut(i).ok_or(Open::Reason("a cell names a vertex the region does not have"))? = true;
        }
    }
    if used.contains(&false) {
        return Err(Open::Reason("a vertex of the region is in no cell"));
    }
    Ok(region.cells.clone())
}

fn exact_pose(pose: &CameraPose) -> Result<&Pose, Error> {
    match pose {
        CameraPose::Exact(pose) => Ok(pose),
        CameraPose::Approximate { .. } => {
            Err(approximate("a float camera matrix is not an exact pose; it is never rounded into one"))
        }
    }
}

fn setup(property: &Property, region: &Region, pose: &CameraPose) -> Result<(Option<Vec<V4>>, Scope), Error> {
    let pose = exact_pose(pose)?;
    let scope = Scope {
        property: property.name().to_owned(),
        regions: vec![region.name.clone()],
        vertices: region.vertices.len() as u64,
        poses: vec![pose.id()],
    };
    let posed: Option<Vec<V4>> = region.vertices.iter().map(|v| pose.apply(v)).collect();
    Ok((posed, scope))
}

/// The posed vertices and the cells, or why there is nothing to check.
fn prepare(posed: Option<Vec<V4>>, region: &Region) -> Result<(Vec<V4>, Vec<Vec<usize>>), Open> {
    let cells = cells(region)?;
    Ok((posed.ok_or(Open::Overflow)?, cells))
}

/// The exact check of a declared property of the stereographic chart at an exact pose. A float pose is
/// `LB4:type.approximate_property_input`.
pub fn check(chart: &Stereographic, property: &Property, region: &Region, pose: &CameraPose) -> Result<Verdict, Error> {
    let (posed, scope) = setup(property, region, pose)?;
    if exact::is_zero(&chart.pole) {
        return Ok(not_established(ZERO_POLE, scope));
    }
    let pole = &chart.pole;
    let decided = prepare(posed, region).and_then(|(posed, cells)| match property {
        Property::Domain => domain(pole, &posed, &cells),
        Property::Injective => injective(pole, &posed, &cells),
        Property::Embedding => embedding(pole, &posed, &cells),
        Property::Isometry { .. } | Property::Homomorphism { .. } => Err(Open::Reason(NOT_PROJECTIVE)),
        Property::Section { normal, low, high } => section(normal, *low, *high, &posed),
    });
    Ok(conclude(decided, scope))
}

/// The per-vertex test of the stereographic chart at an exact pose: it refutes with an exact witness or establishes
/// nothing. The domain is refuted by a vertex on the singular ray, injectivity by two vertices of one cell on one ray
/// off it, an embedding by any two vertices on one ray off it, and a section by a vertex outside the slab. Isometry
/// and homomorphism have no test for this chart.
pub fn vertex_test(
    chart: &Stereographic,
    property: &Property,
    region: &Region,
    pose: &CameraPose,
) -> Result<Verdict, Error> {
    let (posed, scope) = setup(property, region, pose)?;
    if exact::is_zero(&chart.pole) {
        return Ok(not_established(ZERO_POLE, scope));
    }
    let pole = &chart.pole;
    let decided = prepare(posed, region).and_then(|(posed, cells)| {
        let pair = |found: Option<(usize, usize)>| found.map(|(i, j)| Witness::SharedRay(i, j));
        let all: Vec<usize> = (0..posed.len()).collect();
        let witness = match property {
            Property::Domain => vertex_on_ray(pole, &posed).map(|found| found.map(Witness::VertexOnSingularRay)),
            Property::Injective => shared_in_cells(pole, &posed, &cells).map(pair),
            Property::Embedding => shared_ray(pole, &posed, &all).map(pair),
            Property::Isometry { .. } | Property::Homomorphism { .. } => return Err(Open::Reason(NOT_PROJECTIVE)),
            Property::Section { normal, low, high } => {
                vertex_outside(normal, *low, *high, &posed).map(|found| found.map(Witness::VertexOutside))
            }
        };
        match witness.ok_or(Open::Overflow)? {
            Some(witness) => Ok(fails(Method::VertexTest, witness)),
            None => Err(Open::Reason(VERTICES_CANNOT_ESTABLISH)),
        }
    });
    Ok(conclude(decided, scope))
}

/// The exact check of a declared property of a projective chart at an exact pose: injectivity, embedding, isometry
/// and homomorphism each by its own `c600_jumble::polytope` check, the domain and a section on the posed vertices. On
/// a region its denominator does not keep positive, the four decide nothing. A float pose is
/// `LB4:type.approximate_property_input`.
pub fn check_projective(
    chart: &Projective,
    property: &Property,
    region: &Region,
    pose: &CameraPose,
) -> Result<Verdict, Error> {
    let (posed, scope) = setup(property, region, pose)?;
    let decided = prepare(posed, region).and_then(|(posed, cells)| projective(chart, property, &posed, &cells));
    Ok(conclude(decided, scope))
}

fn on_ray(v: &V4, pole: &V4) -> Option<bool> {
    Some(exact::is_zero(v) || exact::same_ray(v, pole)?)
}

fn vertex_on_ray(pole: &V4, posed: &[V4]) -> Option<Option<usize>> {
    for (i, v) in posed.iter().enumerate() {
        if on_ray(v, pole)? {
            return Some(Some(i));
        }
    }
    Some(None)
}

/// Two distinct posed vertices among `indices` on one ray from the origin, off the singular ray: both are in the
/// domain, and the chart maps them to one point. Vertices on the singular ray are outside the domain and witness
/// nothing about injectivity.
fn shared_ray(pole: &V4, posed: &[V4], indices: &[usize]) -> Option<Option<(usize, usize)>> {
    for (k, &i) in indices.iter().enumerate() {
        if on_ray(&posed[i], pole)? {
            continue;
        }
        for &j in &indices[k + 1..] {
            if posed[i] != posed[j] && exact::same_ray(&posed[i], &posed[j])? {
                return Some(Some((i.min(j), i.max(j))));
            }
        }
    }
    Some(None)
}

/// Two vertices of one cell on one ray off the singular ray.
fn shared_in_cells(pole: &V4, posed: &[V4], cells: &[Vec<usize>]) -> Option<Option<(usize, usize)>> {
    for cell in cells {
        if let Some(found) = shared_ray(pole, posed, cell)? {
            return Some(Some(found));
        }
    }
    Some(None)
}

fn vertex_outside(normal: &V4, low: Q5, high: Q5, posed: &[V4]) -> Option<Option<usize>> {
    for (i, v) in posed.iter().enumerate() {
        let h = exact::dot(normal, v)?;
        if h.sub(low)?.sign()? < 0 || high.sub(h)?.sign()? < 0 {
            return Some(Some(i));
        }
    }
    Some(None)
}

/// The exact region test of the domain: the region misses the closed singular ray {t·pole : t ≥ 0} exactly when every
/// cell does, and `ray_meet` from the origin along the pole finds a cell's point on it with the least t.
fn domain(pole: &V4, posed: &[V4], cells: &[Vec<usize>]) -> Result<Decided, Open> {
    let origin = vec![Big::zero(); 4];
    let direction = big_point(pole)?;
    for cell in cells {
        if let Some(point) = hull(posed, cell)?.ray_meet(&origin, &direction)? {
            return Ok(fails(Method::RegionTest, Witness::PointOnSingularRay(small_v4(&point)?)));
        }
    }
    Ok(holds(Method::RegionTest))
}

/// A central projection's check read for the stereographic chart: `None` when it holds, so that no two points of the
/// region lie on one open ray; its pair when that pair is off the singular ray; otherwise nothing is decided.
fn off_the_singular_ray(pole: &V4, check: Check<SameImage>) -> Result<Option<(V4, V4)>, Open> {
    let Check::Fails(pair) = check else { return Ok(None) };
    let (x, y) = (small_v4(&pair.x)?, small_v4(&pair.y)?);
    if on_ray(&x, pole).ok_or(Open::Overflow)? {
        return Err(Open::Reason(PAIR_ON_SINGULAR_RAY));
    }
    Ok(Some((x, y)))
}

/// The reduction on one cell, with a functional positive on that cell.
fn reduce_cell(pole: &V4, posed: &[V4], cell: &[usize]) -> Result<Option<(V4, V4)>, Open> {
    let region = hull(posed, cell)?;
    let h = region.positive_functional().ok_or(Open::Reason(THROUGH_ORIGIN))?;
    off_the_singular_ray(pole, polytope::injective(&ProjectiveMap::central(h)?, &region)?)
}

fn on_one_ray(x: V4, y: V4) -> Decided {
    fails(Method::Reduction(CENTRAL_PROJECTION), Witness::PointsOnOneRay(x, y))
}

/// Injectivity on each cell, by the reduction. Where the reduction decides nothing for some cell, two vertices of one
/// cell, of any cell, on one ray may still refute it.
fn injective(pole: &V4, posed: &[V4], cells: &[Vec<usize>]) -> Result<Decided, Open> {
    let mut open = None;
    for cell in cells {
        match reduce_cell(pole, posed, cell) {
            Ok(None) => {}
            Ok(Some((x, y))) => return Ok(on_one_ray(x, y)),
            Err(why) => {
                open.get_or_insert(why);
            }
        }
    }
    let Some(why) = open else { return Ok(holds(Method::Reduction(CENTRAL_PROJECTION))) };
    match shared_in_cells(pole, posed, cells).ok_or(Open::Overflow)? {
        Some((i, j)) => Ok(fails(Method::VertexTest, Witness::SharedRay(i, j))),
        None => Err(why),
    }
}

/// An embedding: injectivity on the union of the cells, by the reduction with one functional positive on every cell.
/// Where the reduction decides nothing, two vertices on one ray may still refute it.
fn embedding(pole: &V4, posed: &[V4], cells: &[Vec<usize>]) -> Result<Decided, Open> {
    let all: Vec<usize> = (0..posed.len()).collect();
    let reduced = hull(posed, &all).and_then(|union| {
        let h = union.positive_functional().ok_or(Open::Reason(NO_COMMON_FUNCTIONAL))?;
        off_the_singular_ray(pole, polytope::embedding(&ProjectiveMap::central(h)?, &hulls(posed, cells)?)?)
    });
    match reduced {
        Ok(None) => Ok(holds(Method::Reduction(CENTRAL_PROJECTION))),
        Ok(Some((x, y))) => Ok(on_one_ray(x, y)),
        Err(why) => match shared_ray(pole, posed, &all).ok_or(Open::Overflow)? {
            Some((i, j)) => Ok(fails(Method::VertexTest, Witness::SharedRay(i, j))),
            None => Err(why),
        },
    }
}

/// Section membership on the posed vertices: the slab is convex, so the region lies in it exactly when they do.
fn section(normal: &V4, low: Q5, high: Q5, posed: &[V4]) -> Result<Decided, Open> {
    Ok(match vertex_outside(normal, low, high, posed).ok_or(Open::Overflow)? {
        Some(i) => fails(Method::PosedVertices, Witness::VertexOutside(i)),
        None => holds(Method::PosedVertices),
    })
}

fn same_image(pair: &SameImage) -> Result<Witness, Open> {
    Ok(Witness::SameImage { x: small_v4(&pair.x)?, y: small_v4(&pair.y)?, image: small_point(&pair.image)? })
}

/// A projective chart's property, each by its own check.
fn projective(chart: &Projective, property: &Property, posed: &[V4], cells: &[Vec<usize>]) -> Result<Decided, Open> {
    let map = chart.map()?;
    let own = Method::PolytopeCheck;
    match property {
        // The denominator is affine, so it is positive on the region exactly when it is at every posed vertex.
        Property::Domain => {
            for (i, v) in posed.iter().enumerate() {
                if map.scale_at(&big_point(v)?)?.sign() <= 0 {
                    return Ok(fails(Method::PosedVertices, Witness::VertexOutsideDomain(i)));
                }
            }
            Ok(holds(Method::PosedVertices))
        }
        Property::Injective => {
            for cell in cells {
                if let Check::Fails(pair) = polytope::injective(&map, &hull(posed, cell)?)? {
                    return Ok(fails(own, same_image(&pair)?));
                }
            }
            Ok(holds(own))
        }
        Property::Embedding => Ok(match polytope::embedding(&map, &hulls(posed, cells)?)? {
            Check::Holds => holds(own),
            Check::Fails(pair) => fails(own, same_image(&pair)?),
        }),
        Property::Isometry { source, target } => {
            let (source, target) = (Metric::gram(big_rows(source)?)?, Metric::gram(big_rows(target)?)?);
            for cell in cells {
                if let Check::Fails(change) = polytope::isometry(&map, &hull(posed, cell)?, &source, &target)? {
                    let (x, y) = (small_v4(&change.x)?, small_v4(&change.y)?);
                    let (before, after) = (small(&change.before)?, small(&change.after)?);
                    return Ok(fails(own, Witness::DistanceChange { x, y, before, after }));
                }
            }
            Ok(holds(own))
        }
        Property::Homomorphism { codomain } => {
            let mut there = Vec::with_capacity(codomain.len());
            for cell in codomain {
                there.push(Polytope::hull(&big_rows(cell)?)?);
            }
            Ok(match polytope::homomorphism(&map, &hulls(posed, cells)?, &there)? {
                Check::Holds => holds(own),
                Check::Fails(unmatched) => {
                    let mut image = Vec::with_capacity(unmatched.image.vertices().len());
                    for vertex in unmatched.image.vertices() {
                        image.push(small_point(vertex)?);
                    }
                    fails(own, Witness::UnmatchedCell { cell: unmatched.cell, image })
                }
            })
        }
        Property::Section { normal, low, high } => section(normal, *low, *high, posed),
    }
}

/// The content identity of a plan's chart (RPI.28): its stages with their definitions and exact parameter bits, its
/// dimension and its precision. Its claimed properties are not part of it.
pub fn identity(chart: &Chart) -> String {
    let mut text = format!("chart/1\ndimension {}\nprecision {}\n", chart.dimension, chart.precision);
    for stage in &chart.stages {
        text.push_str(&format!("{} {} {}", stage.name, stage.version, stage.definition));
        for p in &stage.params {
            text.push_str(&format!(" {:016x}", p.to_bits()));
        }
        text.push('\n');
    }
    sha256_hex(text.as_bytes())
}

/// A check result that admission consumes (RPI.48): the chart's content identity, the property name the plan claims
/// and the verdict with its scope.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChartCheck {
    pub chart: String,
    pub property: String,
    pub verdict: Verdict,
}

impl ChartCheck {
    /// The check's identity: its chart, the property name the plan claims and the scope, with the property it decided.
    pub fn id(&self) -> String {
        let scope = self.verdict.scope();
        let text = format!(
            "{}\n{}\n{}\n{}\n{}\n{}",
            self.chart,
            self.property,
            scope.property,
            scope.regions.join(","),
            scope.vertices,
            scope.poses.join(",")
        );
        sha256_hex(text.as_bytes())
    }
}

/// The status of an exact chart property in a validated publication: established by check results, with their
/// scopes, or not established. There is no `false`: a failed check withholds the status and certifies no negative.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PropertyStatus {
    NotEstablished,
    Established(Vec<Scope>),
}

/// RPI.48's `admit_chart` result.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Admission {
    Admitted,
    NotEstablished(String),
    Refused(Error),
}

/// The (region, pose) pairs a scope covers.
fn covered(scope: &Scope) -> impl Iterator<Item = (&str, &str)> + '_ {
    scope.regions.iter().flat_map(move |r| scope.poses.iter().map(move |p| (r.as_str(), p.as_str())))
}

/// The property a claimed name refers to: the part before its first `:`, after which a plan puts the property's
/// parameters when it must tell two claims of one property apart, such as two sections' slabs (a draft).
fn base_name(claimed: &str) -> &str {
    claimed.split_once(':').map_or(claimed, |(base, _)| base)
}

/// RPI.48's `admit_chart` for a built-in chart. A check result counts for a claimed property when it names this chart
/// and the claimed name, and its scope says it is the own check of the property that name refers to ([`base_name`]),
/// so no property is taken from another's check. A refuted one refuses the chart with
/// `EV3:evidence.chart_check_failed`, naming the chart, property and check. A property is established when its own
/// established results cover every region and exact pose at which any result for this chart was checked; anything else
/// stays `not_established`. Also returns each claimed property's status.
pub fn admit_chart(chart: &Chart, checks: &[ChartCheck]) -> (Admission, Vec<(String, PropertyStatus)>) {
    let id = identity(chart);
    let mine: Vec<&ChartCheck> = checks.iter().filter(|c| c.chart == id).collect();
    let surveyed: BTreeSet<(&str, &str)> = mine.iter().flat_map(|c| covered(c.verdict.scope())).collect();
    let mut statuses = Vec::new();
    let mut unknown = Vec::new();
    for property in &chart.properties {
        let own: Vec<&ChartCheck> = mine
            .iter()
            .copied()
            .filter(|c| c.property == *property && c.verdict.scope().property == base_name(property))
            .collect();
        if let Some(failed) = own.iter().find(|c| matches!(c.verdict, Verdict::Refuted { .. })) {
            let error = Error::new(
                codes::CHART_CHECK_FAILED,
                format!("chart {} failed its {property} check; its checked status is withheld", chart.chart.0),
            )
            .with("chart", chart.chart.0)
            .with("property", property)
            .with("check", failed.id());
            let withheld = chart.properties.iter().map(|p| (p.clone(), PropertyStatus::NotEstablished)).collect();
            return (Admission::Refused(error), withheld);
        }
        let scopes: Vec<&Scope> = own
            .iter()
            .filter_map(|c| match &c.verdict {
                Verdict::Established { scope, .. } => Some(scope),
                _ => None,
            })
            .collect();
        let reached: BTreeSet<(&str, &str)> = scopes.iter().copied().flat_map(covered).collect();
        if scopes.is_empty() || !surveyed.is_subset(&reached) {
            unknown.push(property.clone());
            statuses.push((property.clone(), PropertyStatus::NotEstablished));
        } else {
            statuses.push((property.clone(), PropertyStatus::Established(scopes.into_iter().cloned().collect())));
        }
    }
    let admission = if unknown.is_empty() {
        Admission::Admitted
    } else {
        Admission::NotEstablished(format!(
            "no established check of its own at every surveyed region and pose for {}",
            unknown.join(", ")
        ))
    };
    (admission, statuses)
}

/// One chart's admission in a validated publication, with the coverage of its output: a refused chart's output is
/// undefined with the error's code, and the rest of the publication is unaffected (RPI.48).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChartAdmission {
    pub plan: crate::plan::PlanId,
    pub chart: ChartId,
    pub admission: Admission,
    pub coverage: crate::plan::Coverage,
}

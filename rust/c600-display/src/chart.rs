//! Exact chart-property checks at exact camera poses (RPI.28 to RPI.30, RPI.34, LB4.2) and the admission of a chart
//! against its check results (RPI.48).
//!
//! The repertoire is a restricted family with known checkers, RPI.34's first option, as a draft: the stereographic
//! chart, declared exactly by its pole, over regions given as the convex hull of exact vertices in Q(√5). An exact pose
//! is the identity or a verified element of K⁺; a float camera matrix is refused, never rounded into a pose (RPI.30).
//! Domain is decided by an exact region test, injectivity by a theorem or a vertex refutation, and section membership
//! on the posed vertices; [`vertex_test`] can only refute. Every verdict records the region, vertices and pose it
//! covers.

use c600_digest::sha256_hex;

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

/// A region given exactly as the convex hull of its vertices.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Region {
    pub name: String,
    pub vertices: Vec<V4>,
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

/// A declared chart property, in camera coordinates after the pose.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Property {
    /// The posed region lies in the chart's domain.
    Domain,
    /// The chart is injective on the posed region, within its domain.
    Injective,
    /// The posed region lies in the slab low ≤ normal · x ≤ high.
    Section { normal: V4, low: Q5, high: Q5 },
}

impl Property {
    pub fn name(&self) -> &'static str {
        match self {
            Property::Domain => "domain",
            Property::Injective => "injective",
            Property::Section { .. } => "section",
        }
    }
}

/// The theorem that establishes injectivity on a flat region.
pub const RADIAL_THEOREM: &str =
    "a ray from the origin meets an affine hyperplane that misses the origin at most once, \
                                  and stereographic projection is injective off its pole";

/// How a verdict was reached.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Method {
    /// The exact region test.
    RegionTest,
    /// A theorem whose exact precondition held.
    Theorem(&'static str),
    /// The posed vertices, to which convexity reduces the property.
    PosedVertices,
    /// A per-vertex test, which can refute and cannot establish.
    VertexTest,
}

/// An exact counterexample.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Witness {
    /// A point of the posed region on the singular ray.
    PointOnSingularRay(V4),
    /// A posed vertex on the singular ray.
    VertexOnSingularRay(usize),
    /// Two posed vertices on one ray from the origin.
    SharedRay(usize, usize),
    /// A posed vertex outside the section.
    VertexOutside(usize),
}

/// What a check covers (RPI.30, INV-5): its regions, their vertex count and its poses. Nothing outside it is claimed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Scope {
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

fn exact_pose(pose: &CameraPose) -> Result<&Pose, Error> {
    match pose {
        CameraPose::Exact(pose) => Ok(pose),
        CameraPose::Approximate { .. } => {
            Err(approximate("a float camera matrix is not an exact pose; it is never rounded into one"))
        }
    }
}

fn setup(region: &Region, pose: &CameraPose) -> Result<(Option<Vec<V4>>, Scope), Error> {
    let pose = exact_pose(pose)?;
    let scope =
        Scope { regions: vec![region.name.clone()], vertices: region.vertices.len() as u64, poses: vec![pose.id()] };
    let posed: Option<Vec<V4>> = region.vertices.iter().map(|v| pose.apply(v)).collect();
    Ok((posed, scope))
}

/// The exact check of a declared property at an exact pose. A float pose is `LB4:type.approximate_property_input`.
pub fn check(chart: &Stereographic, property: &Property, region: &Region, pose: &CameraPose) -> Result<Verdict, Error> {
    let (posed, scope) = setup(region, pose)?;
    if exact::is_zero(&chart.pole) {
        return Ok(not_established(ZERO_POLE, scope));
    }
    let verdict = posed.and_then(|posed| match property {
        Property::Domain => domain(&chart.pole, &posed, scope.clone()),
        Property::Injective => injective(&chart.pole, &posed, scope.clone()),
        Property::Section { normal, low, high } => section(normal, *low, *high, &posed, scope.clone()),
    });
    Ok(verdict.unwrap_or_else(|| not_established(OVERFLOW, scope)))
}

/// The per-vertex test at an exact pose: it refutes with an exact witness or establishes nothing.
pub fn vertex_test(
    chart: &Stereographic,
    property: &Property,
    region: &Region,
    pose: &CameraPose,
) -> Result<Verdict, Error> {
    let (posed, scope) = setup(region, pose)?;
    if exact::is_zero(&chart.pole) {
        return Ok(not_established(ZERO_POLE, scope));
    }
    let verdict = posed.and_then(|posed| {
        let witness = match property {
            Property::Domain => vertex_on_ray(&chart.pole, &posed)?.map(Witness::VertexOnSingularRay),
            Property::Injective => shared_ray(&chart.pole, &posed)?.map(|(i, j)| Witness::SharedRay(i, j)),
            Property::Section { normal, low, high } => {
                vertex_outside(normal, *low, *high, &posed)?.map(Witness::VertexOutside)
            }
        };
        Some(match witness {
            Some(witness) => Verdict::Refuted { method: Method::VertexTest, witness, scope: scope.clone() },
            None => not_established(VERTICES_CANNOT_ESTABLISH, scope.clone()),
        })
    });
    Ok(verdict.unwrap_or_else(|| not_established(OVERFLOW, scope)))
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

/// Two distinct posed vertices on one ray from the origin, off the singular ray: both are in the domain, and the chart
/// maps them to one point. Vertices on the singular ray are outside the domain and witness nothing about injectivity.
fn shared_ray(pole: &V4, posed: &[V4]) -> Option<Option<(usize, usize)>> {
    for i in 0..posed.len() {
        if on_ray(&posed[i], pole)? {
            continue;
        }
        for j in i + 1..posed.len() {
            if posed[i] != posed[j] && exact::same_ray(&posed[i], &posed[j])? {
                return Some(Some((i, j)));
            }
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

/// The four-element index subsets of 0..n, in lexicographic order.
fn bases(n: usize) -> Vec<[usize; 4]> {
    let mut out = Vec::new();
    for a in 0..n {
        for b in a + 1..n {
            for c in b + 1..n {
                for d in c + 1..n {
                    out.push([a, b, c, d]);
                }
            }
        }
    }
    out
}

/// The exact region test of the domain. The region misses the closed singular ray exactly when the pole is outside the
/// cone of the posed vertices and the origin is outside their hull. With four independent vertices, the pole is in the
/// cone exactly when some basis of vertices gives it non-negative Cramer coefficients (Carathéodory); the origin is
/// certified outside when every vertex has a positive product with the vertex sum.
fn domain(pole: &V4, posed: &[V4], scope: Scope) -> Option<Verdict> {
    let mut full_rank = false;
    for basis in bases(posed.len()) {
        let rows: M4 = basis.map(|i| posed[i]);
        let det = exact::det(&rows)?;
        let sign = det.sign()?;
        if sign == 0 {
            continue;
        }
        full_rank = true;
        let mut coefficients = [Q5::ZERO; 4];
        let mut inside = true;
        for i in 0..4 {
            let mut replaced = rows;
            replaced[i] = *pole;
            let d = exact::det(&replaced)?;
            if d.sign()? * sign < 0 {
                inside = false;
                break;
            }
            coefficients[i] = d.div(det)?;
        }
        if inside {
            // pole = Σ μ_i b_i with μ ≥ 0, so Σ μ_i b_i / Σ μ_i is a point of the region on the pole's ray.
            let total = coefficients.iter().try_fold(Q5::ZERO, |sum, c| sum.add(*c))?;
            let mut point = [Q5::ZERO; 4];
            for (i, c) in coefficients.iter().enumerate() {
                point = exact::add(&point, &exact::scale(c.div(total)?, &rows[i])?)?;
            }
            return Some(Verdict::Refuted {
                method: Method::RegionTest,
                witness: Witness::PointOnSingularRay(point),
                scope,
            });
        }
    }
    if !full_rank {
        return Some(not_established("the region test needs four independent vertices", scope));
    }
    let sum = posed.iter().try_fold([Q5::ZERO; 4], |sum, v| exact::add(&sum, v))?;
    for v in posed {
        if exact::dot(v, &sum)?.sign()? <= 0 {
            return Some(not_established("no exact certificate that the region misses the origin", scope));
        }
    }
    Some(Verdict::Established { method: Method::RegionTest, scope })
}

/// Injectivity: refuted by two vertices in the domain on one ray; established by [`RADIAL_THEOREM`] when four vertices
/// are independent and every vertex lies in their affine hull, a hyperplane that misses the origin.
fn injective(pole: &V4, posed: &[V4], scope: Scope) -> Option<Verdict> {
    if let Some((i, j)) = shared_ray(pole, posed)? {
        return Some(Verdict::Refuted { method: Method::VertexTest, witness: Witness::SharedRay(i, j), scope });
    }
    for basis in bases(posed.len()) {
        let rows: M4 = basis.map(|i| posed[i]);
        if exact::det(&rows)?.is_zero() {
            continue;
        }
        let edges = [exact::sub(&rows[1], &rows[0])?, exact::sub(&rows[2], &rows[0])?, exact::sub(&rows[3], &rows[0])?];
        let mut flat = true;
        for v in posed {
            let offset = exact::sub(v, &rows[0])?;
            if !exact::det(&[edges[0], edges[1], edges[2], offset])?.is_zero() {
                flat = false;
                break;
            }
        }
        return Some(if flat {
            Verdict::Established { method: Method::Theorem(RADIAL_THEOREM), scope }
        } else {
            not_established("no exact checker for the injectivity of a full-dimensional region", scope)
        });
    }
    Some(not_established("the theorem needs four independent vertices", scope))
}

/// Section membership on the posed vertices: the slab is convex, so the region lies in it exactly when they do.
fn section(normal: &V4, low: Q5, high: Q5, posed: &[V4], scope: Scope) -> Option<Verdict> {
    Some(match vertex_outside(normal, low, high, posed)? {
        Some(i) => Verdict::Refuted { method: Method::PosedVertices, witness: Witness::VertexOutside(i), scope },
        None => Verdict::Established { method: Method::PosedVertices, scope },
    })
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
    /// The check's identity: its chart, property and scope.
    pub fn id(&self) -> String {
        let scope = self.verdict.scope();
        let text = format!(
            "{}\n{}\n{}\n{}\n{}",
            self.chart,
            self.property,
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

/// RPI.48's `admit_chart` for a built-in chart: every property the chart claims needs an established check result for
/// that chart; a refuted one refuses the chart with `EV3:evidence.chart_check_failed`, naming the chart, property and
/// check; anything else stays `not_established`. Also returns each claimed property's status.
pub fn admit_chart(chart: &Chart, checks: &[ChartCheck]) -> (Admission, Vec<(String, PropertyStatus)>) {
    let id = identity(chart);
    let mut statuses = Vec::new();
    let mut unknown = Vec::new();
    for property in &chart.properties {
        let results: Vec<&ChartCheck> = checks.iter().filter(|c| c.chart == id && &c.property == property).collect();
        if let Some(failed) = results.iter().find(|c| matches!(c.verdict, Verdict::Refuted { .. })) {
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
        let scopes: Vec<Scope> = results
            .iter()
            .filter_map(|c| match &c.verdict {
                Verdict::Established { scope, .. } => Some(scope.clone()),
                _ => None,
            })
            .collect();
        if scopes.is_empty() {
            unknown.push(property.clone());
            statuses.push((property.clone(), PropertyStatus::NotEstablished));
        } else {
            statuses.push((property.clone(), PropertyStatus::Established(scopes)));
        }
    }
    let admission = if unknown.is_empty() {
        Admission::Admitted
    } else {
        Admission::NotEstablished(format!("no established check for {}", unknown.join(", ")))
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

//! The CPU reference evaluation of the stereographic chart at a camera pose, in binary32 (RPI.37, RPI.56, RPI.69,
//! RPI.74), for the coverage and reproducibility fixtures. A posed vertex at the origin or, at an exact pose, on the
//! singular ray is decided exactly; near the ray a declared exclusion neighbourhood decides for display only. A point
//! without finite coordinates carries missing coverage and a reason and enters neither the draw list nor the pick
//! list; no finite point is fabricated, and nothing here changes a plan, a subject or a claim.

use c600_digest::sha256_hex;

use crate::chart::{CameraPose, Stereographic};
use crate::exact::{self, Q5, V4};
use crate::plan::{Chart, Coverage, DisplayPlan, PlanId};

/// A declared precision rule. RPI.36 leaves the rule open, so both values are fixture declarations: an absolute bound
/// on every output coordinate against the exact image, outside the exclusion neighbourhood 1 − u·p ≤ `exclusion` of
/// the pole, where u and p are the normalised point and pole.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PrecisionRule {
    pub absolute: f32,
    pub exclusion: f32,
}

impl PrecisionRule {
    /// The rule's exact text: the bit patterns of its two binary32 values.
    pub fn text(&self) -> String {
        format!("absolute {:08x} exclusion {:08x}", self.absolute.to_bits(), self.exclusion.to_bits())
    }
}

/// Why a point has no coordinates (RPI.37's reason).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Missing {
    /// The posed vertex is the origin, where the map is undefined; decided exactly.
    Origin,
    /// The posed vertex is on the singular ray, decided exactly at an exact pose.
    Singular,
    /// Inside the declared exclusion neighbourhood of the pole; a display decision that decides nothing.
    Excluded,
    /// Binary32 evaluation gave no finite coordinates.
    NonFinite,
}

impl Missing {
    pub fn reason(self) -> &'static str {
        match self {
            Missing::Origin => "origin",
            Missing::Singular => "singular",
            Missing::Excluded => "excluded",
            Missing::NonFinite => "non_finite",
        }
    }
}

/// One vertex's result: binary32 coordinates of the image in the pole's orthogonal complement, written in R⁴, or
/// missing coverage with its reason.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Point {
    Drawn([f32; 4]),
    Missing(Missing),
}

/// The exact fields of an evaluation (RPI.56): the same key gives the same fields wherever it runs. Float camera
/// matrices are not among them; an approximate pose enters by its declared text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExactFields {
    pub plan: PlanId,
    pub subject: String,
    pub base: u64,
    pub chart: String,
    pub declaration: String,
    pub pose: String,
    pub vertices: String,
    pub precision: String,
    /// The vertices whose coverage is decided exactly, with the reason.
    pub exact_missing: Vec<(u32, Missing)>,
}

impl ExactFields {
    fn text(&self) -> String {
        let missing: Vec<String> = self.exact_missing.iter().map(|(i, m)| format!("{i}:{}", m.reason())).collect();
        format!(
            "reference/1\nplan {}\nsubject {}\nbase {}\nchart {}\ndeclaration {}\npose {}\nvertices {}\nprecision {}\n\
             missing {}\n",
            self.plan.0,
            self.subject,
            self.base,
            self.chart,
            self.declaration,
            self.pose,
            self.vertices,
            self.precision,
            missing.join(",")
        )
    }

    /// The reproducibility key: the SHA-256 of the exact fields' canonical text.
    pub fn key(&self) -> String {
        sha256_hex(self.text().as_bytes())
    }
}

/// A reference evaluation: its exact fields and one point per vertex.
#[derive(Clone, Debug, PartialEq)]
pub struct Evaluation {
    pub exact: ExactFields,
    pub points: Vec<Point>,
}

impl Evaluation {
    pub fn key(&self) -> String {
        self.exact.key()
    }

    fn finite(&self) -> Vec<(u32, [f32; 4])> {
        self.points
            .iter()
            .enumerate()
            .filter_map(|(i, p)| match p {
                Point::Drawn(y) if y.iter().all(|c| c.is_finite()) => Some((i as u32, *y)),
                _ => None,
            })
            .collect()
    }

    /// What the draw consumes: the finite points with their vertex indices.
    pub fn draw(&self) -> Vec<(u32, [f32; 4])> {
        self.finite()
    }

    /// What hit testing reads: the same points as the draw; undefined coordinates enter neither (RPI.37, RPI.75).
    pub fn pick(&self) -> Vec<(u32, [f32; 4])> {
        self.finite()
    }

    /// The projection's coverage: complete, or partial with every missing vertex and its reason (KN0.4).
    pub fn coverage(&self) -> Coverage {
        let missing: Vec<String> = self
            .points
            .iter()
            .enumerate()
            .filter_map(|(i, p)| match p {
                Point::Missing(m) => Some(format!("vertex {i}: {}", m.reason())),
                Point::Drawn(_) => None,
            })
            .collect();
        if missing.is_empty() {
            Coverage::Complete
        } else {
            Coverage::Partial { missing }
        }
    }
}

/// The inputs of a reference evaluation: the plan and its chart, the chart's exact declaration, the exact vertices and
/// the declared precision rule.
#[derive(Clone, Copy, Debug)]
pub struct Fixture<'a> {
    pub plan: &'a DisplayPlan,
    pub chart: &'a Chart,
    pub declaration: &'a Stereographic,
    pub vertices: &'a [V4],
    pub rule: PrecisionRule,
}

fn dot32(u: &[f32; 4], v: &[f32; 4]) -> f32 {
    u[0] * v[0] + u[1] * v[1] + u[2] * v[2] + u[3] * v[3]
}

fn exact32(v: &V4) -> [f32; 4] {
    v.map(|q| q.approx() as f32)
}

/// The exact decision for one vertex, where one exists: the origin always, the singular ray at an exact pose.
fn exact_decision(v: &V4, pose: &CameraPose, pole: &V4) -> Option<Missing> {
    if exact::is_zero(v) {
        return Some(Missing::Origin);
    }
    let CameraPose::Exact(pose) = pose else { return None };
    let posed = exact::apply(&pose.matrix(), v)?;
    if exact::same_ray(&posed, pole)? {
        return Some(Missing::Singular);
    }
    None
}

/// Evaluates the chart at `pose` in binary32.
pub fn evaluate(fixture: &Fixture<'_>, pose: &CameraPose) -> Evaluation {
    let matrix: [[f32; 4]; 4] = match pose {
        CameraPose::Exact(p) => p.matrix().map(|row| row.map(|q| q.approx() as f32)),
        CameraPose::Approximate { matrix, .. } => matrix.map(|row| row.map(|x| x as f32)),
    };
    let pole = exact32(&fixture.declaration.pole);
    let length = dot32(&pole, &pole).sqrt();
    let pole = pole.map(|c| c / length);
    let mut points = Vec::with_capacity(fixture.vertices.len());
    let mut exact_missing = Vec::new();
    for (i, v) in fixture.vertices.iter().enumerate() {
        if let Some(missing) = exact_decision(v, pose, &fixture.declaration.pole) {
            exact_missing.push((i as u32, missing));
            points.push(Point::Missing(missing));
            continue;
        }
        let v = exact32(v);
        let x: [f32; 4] = std::array::from_fn(|r| dot32(&matrix[r], &v));
        let radius = dot32(&x, &x).sqrt();
        if !radius.is_finite() {
            points.push(Point::Missing(Missing::NonFinite));
            continue;
        }
        if radius == 0.0 {
            points.push(Point::Missing(Missing::Origin));
            continue;
        }
        let u = x.map(|c| c / radius);
        let s = dot32(&u, &pole);
        let denominator = 1.0 - s;
        if denominator.is_nan() {
            points.push(Point::Missing(Missing::NonFinite));
            continue;
        }
        if denominator <= fixture.rule.exclusion {
            points.push(Point::Missing(Missing::Excluded));
            continue;
        }
        let y: [f32; 4] = std::array::from_fn(|k| (u[k] - s * pole[k]) / denominator);
        points.push(if y.iter().all(|c| c.is_finite()) { Point::Drawn(y) } else { Point::Missing(Missing::NonFinite) });
    }
    let pose_text = match pose {
        CameraPose::Exact(p) => p.id(),
        CameraPose::Approximate { label, .. } => format!("approximate {label}"),
    };
    let vertex_text: Vec<String> = fixture.vertices.iter().map(exact::text).collect();
    let exact = ExactFields {
        plan: fixture.plan.plan,
        subject: fixture.plan.subject.digest().to_owned(),
        base: fixture.plan.base.state,
        chart: crate::chart::identity(fixture.chart),
        declaration: fixture.declaration.id(),
        pose: pose_text,
        vertices: sha256_hex(vertex_text.join("\n").as_bytes()),
        precision: fixture.rule.text(),
        exact_missing,
    };
    Evaluation { exact, points }
}

/// The exact stereographic image of a unit point x from a unit pole p, (x − (x·p) p) / (1 − x·p), the reference for
/// approximate values. `None` when either is not a unit vector, when x is the pole, or on overflow.
pub fn exact_image(pole: &V4, x: &V4) -> Option<V4> {
    if exact::dot(pole, pole)? != Q5::ONE || exact::dot(x, x)? != Q5::ONE {
        return None;
    }
    let s = exact::dot(x, pole)?;
    let denominator = Q5::ONE.sub(s)?;
    if denominator.is_zero() {
        return None;
    }
    exact::scale(denominator.inv()?, &exact::sub(x, &exact::scale(s, pole)?)?)
}

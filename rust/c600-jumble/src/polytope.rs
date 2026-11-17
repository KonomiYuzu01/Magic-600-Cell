//! KN3.3's exact polytope operations over Q(√5), beside [`crate::dd`]'s exact vertices, and the exact core of LB4.1's
//! map property checks (LP2, for LP3's C6 and CT-LB4-01).
//!
//! A [`Polytope`] is the convex hull of finitely many exact points, of any dimension in any ambient space. It keeps
//! its vertices, the equations of its affine hull, its facets within that hull with the vertices on each, and a basis
//! of the hull's direction space. The facets come from [`dd`](crate::dd) on the polar about the points' centroid.
//! Every question about the region is then decided exactly over the whole region:
//!
//! - [`Polytope::locate`]: a point in the relative interior, on facets, or outside with the first constraint it breaks;
//! - [`Polytope::line_meet`] and [`Polytope::ray_meet`]: a line's or a closed ray's meeting with the region;
//! - [`Polytope::intersect`]: the intersection of two regions;
//! - [`Polytope::positive_functional`]: a functional positive on the region, which exists when the origin is outside.
//!
//! A [`ProjectiveMap`] is x ↦ (M·x + t)/(c·x + d), defined where c·x + d > 0: affine maps and central projections. Its
//! image of a region where it is defined is the hull of the images of the region's vertices. The four checks each
//! decide their own property over the whole region, never over the vertices alone (LB4.2: a per-vertex test can refute
//! a region property and cannot establish one), and each failure carries exact points that verify it:
//!
//! - [`injective`] on a region;
//! - [`embedding`] on a union of cells: injective on each, and f(σ) ∩ f(τ) = f(σ ∩ τ) for every pair;
//! - [`isometry`] on a region, between two [`Metric`]s;
//! - [`homomorphism`] of cell complexes: each cell's image is a cell of the codomain.
//!
//! None of them reports another's property (LB4.1): a caller that claims one property from another, such as
//! injectivity from an embedding, cites the theorem.
//!
//! For injectivity and embedding, radial and stereographic charts reduce to a central projection. Where h·x > 0, the
//! map x ↦ x/|x| has the fibres of x ↦ x/(h·x) ([`ProjectiveMap::central`]): both identify exactly the points of one
//! open ray from the origin. On a region that misses the closed ray {t·pole : t ≥ 0}, the stereographic image of x/|x|
//! from the pole is injective exactly where x ↦ x/|x| is, since stereographic projection is injective off its pole.
//! [`Polytope::ray_meet`] from the origin along the pole decides whether a region meets that ray, and
//! [`Polytope::positive_functional`] gives an h for a region that misses the origin; for a union of cells, one h must be
//! positive on every cell. A caller that uses the reduction cites it.
//!
//! These functions take no budget; a language entry that wraps them keeps its own (LB4.1). Nothing here has a float
//! type.

use std::fmt;

use crate::dd::{double_description, Constraint};
use crate::q5::{dot, rank, Q5};

/// An exact point or vector.
pub type Point = Vec<Q5>;

/// Why an operation could not run. A check that runs gives a [`Check`].
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum PolytopeError {
    /// No points, or no cells.
    Empty,
    /// Points, vectors, maps, metrics or regions of different dimensions, or of dimension 0.
    DimensionMismatch,
    /// A zero direction for a line or a ray, or a zero functional for a central projection.
    ZeroDirection,
    /// A projective map whose denominator is zero everywhere.
    ZeroDenominator,
    /// A Gram matrix that is not symmetric and positive definite.
    NotAMetric,
    /// A map whose denominator is not positive on all of a region.
    OutsideMapDomain,
    /// Constraints that do not describe a bounded full-dimensional region; the reason is [`dd`](crate::dd)'s.
    Description(&'static str),
    /// An internal step failed. This is a defect, never an answer.
    Internal(&'static str),
}

impl fmt::Display for PolytopeError {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PolytopeError::Empty => write!(out, "no points or no cells"),
            PolytopeError::DimensionMismatch => write!(out, "dimensions do not match"),
            PolytopeError::ZeroDirection => write!(out, "a zero direction or functional"),
            PolytopeError::ZeroDenominator => write!(out, "the map's denominator is zero everywhere"),
            PolytopeError::NotAMetric => write!(out, "the Gram matrix is not symmetric and positive definite"),
            PolytopeError::OutsideMapDomain => write!(out, "the map is not defined on all of the region"),
            PolytopeError::Description(reason) => write!(out, "the constraints describe no polytope: {reason}"),
            PolytopeError::Internal(reason) => write!(out, "internal error: {reason}"),
        }
    }
}

impl std::error::Error for PolytopeError {}

/// A facet within the polytope's affine hull: normal·x ≤ offset on the polytope, with equality exactly on the facet.
/// The normal is zero outside the hull's chart columns, so it is unique up to a positive factor, and it is scaled so
/// that its first nonzero entry is 1 or −1.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Facet {
    pub normal: Point,
    pub offset: Q5,
    /// The indices of the polytope's vertices on the facet, in increasing order.
    pub vertices: Vec<usize>,
}

/// An equation normal·x = offset of the polytope's affine hull.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Equation {
    pub normal: Point,
    pub offset: Q5,
}

/// The first constraint a point breaks, by index.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Violation {
    Equation(usize),
    Facet(usize),
}

/// Where a point lies.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Location {
    /// Outside, with the first equation or facet the point breaks.
    Outside(Violation),
    /// On the relative boundary, on these facets.
    Boundary(Vec<usize>),
    /// In the relative interior. A one-point polytope is its own relative interior.
    RelativeInterior,
}

/// The convex hull of finitely many exact points, with its facets, its affine hull's equations and its directions.
#[derive(Clone, Debug)]
pub struct Polytope {
    vertices: Vec<Point>,
    equations: Vec<Equation>,
    facets: Vec<Facet>,
    directions: Vec<Point>,
}

/// Two polytopes are equal when they have the same ambient dimension and the same vertices, in any order.
impl PartialEq for Polytope {
    fn eq(&self, other: &Polytope) -> bool {
        if self.ambient() != other.ambient() || self.vertices.len() != other.vertices.len() {
            return false;
        }
        let mut mine = self.vertices.clone();
        let mut theirs = other.vertices.clone();
        mine.sort();
        theirs.sort();
        mine == theirs
    }
}

impl Eq for Polytope {}

impl Polytope {
    /// The convex hull of `points`, all of one dimension n ≥ 1. A repeated point counts once, and the vertices are the
    /// extreme points in the order they first appear.
    pub fn hull(points: &[Point]) -> Result<Polytope, PolytopeError> {
        let first = points.first().ok_or(PolytopeError::Empty)?;
        let n = first.len();
        if n == 0 || points.iter().any(|q| q.len() != n) {
            return Err(PolytopeError::DimensionMismatch);
        }
        let mut distinct: Vec<Point> = Vec::new();
        for q in points {
            if !distinct.contains(q) {
                distinct.push(q.clone());
            }
        }
        let base = distinct[0].clone();
        let differences: Vec<Point> = distinct[1..].iter().map(|q| sub(q, &base)).collect();
        let (directions, chart) = rref(&differences, n);
        // One equation per column without a pivot: that entry of x − base follows from the pivot entries.
        let equations = (0..n)
            .filter(|i| !chart.contains(i))
            .map(|i| {
                let mut normal = vec![Q5::zero(); n];
                normal[i] = Q5::one();
                for (row, &c) in directions.iter().zip(&chart) {
                    normal[c] = row[i].neg();
                }
                let offset = dot(&normal, &base);
                Equation { normal, offset }
            })
            .collect();
        let k = chart.len();
        if k == 0 {
            return Ok(Polytope { vertices: vec![base], equations, facets: Vec::new(), directions });
        }
        // Chart coordinates are the pivot entries: an affine bijection of the hull onto k-space.
        let local: Vec<Point> = distinct.iter().map(|q| chart.iter().map(|&c| q[c].clone()).collect()).collect();
        let center = centroid(&local);
        let offsets: Vec<Point> = local.iter().map(|q| sub(q, &center)).collect();
        // The centroid is relatively interior, so the polar {y : y·(q − center) ≤ 1} is bounded and full-dimensional,
        // and its vertices are exactly the facets y·(x − center) ≤ 1.
        let constraints: Vec<Constraint> =
            offsets.iter().filter(|a| !is_zero(a)).map(|a| Constraint { a: a.clone(), b: Q5::one() }).collect();
        let normals = double_description(&constraints, k).map_err(|e| PolytopeError::Internal(e.0))?;
        let on: Vec<Vec<bool>> =
            normals.iter().map(|y| offsets.iter().map(|a| dot(y, a) == Q5::one()).collect()).collect();
        // A point is a vertex exactly when the facets through it have rank k.
        let mut index: Vec<Option<usize>> = vec![None; distinct.len()];
        let mut vertices = Vec::new();
        for (q, point) in distinct.iter().enumerate() {
            let through: Vec<Point> =
                normals.iter().zip(&on).filter(|(_, tight)| tight[q]).map(|(y, _)| y.clone()).collect();
            if rank(&through) == k {
                index[q] = Some(vertices.len());
                vertices.push(point.clone());
            }
        }
        let facets = normals
            .iter()
            .zip(&on)
            .map(|(y, tight)| {
                let mut normal = vec![Q5::zero(); n];
                for (value, &c) in y.iter().zip(&chart) {
                    normal[c] = value.clone();
                }
                let (normal, offset) = normalized(normal, Q5::one().add(&dot(y, &center)));
                let vertices = (0..distinct.len()).filter(|&q| tight[q]).filter_map(|q| index[q]).collect();
                Facet { normal, offset, vertices }
            })
            .collect();
        Ok(Polytope { vertices, equations, facets, directions })
    }

    /// The region {x : a·x ≤ b for every constraint} in dimension `dim`, which must be bounded and full-dimensional
    /// ([`dd`](crate::dd)'s condition).
    pub fn from_constraints(constraints: &[Constraint], dim: usize) -> Result<Polytope, PolytopeError> {
        if dim == 0 || constraints.iter().any(|c| c.a.len() != dim) {
            return Err(PolytopeError::DimensionMismatch);
        }
        let vertices = double_description(constraints, dim).map_err(|e| PolytopeError::Description(e.0))?;
        Polytope::hull(&vertices)
    }

    /// The dimension n of the ambient space.
    pub fn ambient(&self) -> usize {
        self.vertices[0].len()
    }

    /// The dimension of the affine hull.
    pub fn dimension(&self) -> usize {
        self.directions.len()
    }

    pub fn vertices(&self) -> &[Point] {
        &self.vertices
    }

    pub fn facets(&self) -> &[Facet] {
        &self.facets
    }

    pub fn equations(&self) -> &[Equation] {
        &self.equations
    }

    /// A basis of the affine hull's direction space, in reduced row echelon form.
    pub fn directions(&self) -> &[Point] {
        &self.directions
    }

    fn same_dimension(&self, x: &[Q5]) -> Result<(), PolytopeError> {
        if x.len() == self.ambient() {
            Ok(())
        } else {
            Err(PolytopeError::DimensionMismatch)
        }
    }

    /// Where `x` lies: outside with the first equation, then facet, that it breaks; on the facets it meets; or in the
    /// relative interior.
    pub fn locate(&self, x: &[Q5]) -> Result<Location, PolytopeError> {
        self.same_dimension(x)?;
        for (i, e) in self.equations.iter().enumerate() {
            if dot(&e.normal, x) != e.offset {
                return Ok(Location::Outside(Violation::Equation(i)));
            }
        }
        let mut on = Vec::new();
        for (i, f) in self.facets.iter().enumerate() {
            match dot(&f.normal, x).sub(&f.offset).sign() {
                1 => return Ok(Location::Outside(Violation::Facet(i))),
                0 => on.push(i),
                _ => {}
            }
        }
        Ok(if on.is_empty() { Location::RelativeInterior } else { Location::Boundary(on) })
    }

    pub fn contains(&self, x: &[Q5]) -> Result<bool, PolytopeError> {
        Ok(!matches!(self.locate(x)?, Location::Outside(_)))
    }

    /// The parameters {t : p + t·d in the polytope} as a closed interval (lo, hi), or `None` when the line misses the
    /// polytope. The interval is bounded, since the polytope is.
    pub fn line_meet(&self, p: &[Q5], d: &[Q5]) -> Result<Option<(Q5, Q5)>, PolytopeError> {
        self.same_dimension(p)?;
        self.same_dimension(d)?;
        if is_zero(d) {
            return Err(PolytopeError::ZeroDirection);
        }
        let mut fixed: Option<Q5> = None;
        for e in &self.equations {
            let slope = dot(&e.normal, d);
            let gap = e.offset.sub(&dot(&e.normal, p));
            if slope.is_zero() {
                if !gap.is_zero() {
                    return Ok(None);
                }
            } else {
                let t = quotient(&gap, &slope);
                if fixed.as_ref().is_some_and(|f| *f != t) {
                    return Ok(None);
                }
                fixed = Some(t);
            }
        }
        let mut lo = fixed.clone();
        let mut hi = fixed;
        for f in &self.facets {
            let slope = dot(&f.normal, d);
            let gap = f.offset.sub(&dot(&f.normal, p));
            match slope.sign() {
                0 if gap.sign() < 0 => return Ok(None),
                0 => {}
                1 => {
                    let t = quotient(&gap, &slope);
                    if hi.as_ref().is_none_or(|h| t < *h) {
                        hi = Some(t);
                    }
                }
                _ => {
                    let t = quotient(&gap, &slope);
                    if lo.as_ref().is_none_or(|l| t > *l) {
                        lo = Some(t);
                    }
                }
            }
        }
        match (lo, hi) {
            (Some(lo), Some(hi)) => Ok((lo <= hi).then_some((lo, hi))),
            _ => Err(PolytopeError::Internal("a line meets a bounded polytope in an unbounded set")),
        }
    }

    /// The point of the closed ray {e + t·d : t ≥ 0} in the polytope with the least t, or `None` when the ray misses
    /// the polytope.
    pub fn ray_meet(&self, e: &[Q5], d: &[Q5]) -> Result<Option<Point>, PolytopeError> {
        let Some((lo, hi)) = self.line_meet(e, d)? else { return Ok(None) };
        if hi.sign() < 0 {
            return Ok(None);
        }
        let t = if lo.sign() > 0 { lo } else { Q5::zero() };
        Ok(Some(along(e, &t, d)))
    }

    /// The centroid of the vertices, a point of the relative interior.
    pub fn relative_interior_point(&self) -> Point {
        centroid(&self.vertices)
    }

    /// A functional h with h·x > 0 on the whole polytope, or `None` when the origin is in it. The origin breaks an
    /// equation of the hull, and then h·x = 1 on the hull, or a facet, and then h is the facet's normal negated.
    pub fn positive_functional(&self) -> Option<Point> {
        let origin = vec![Q5::zero(); self.ambient()];
        match self.locate(&origin) {
            Ok(Location::Outside(Violation::Equation(i))) => {
                let e = &self.equations[i];
                let inverse = e.offset.inv()?;
                Some(e.normal.iter().map(|x| x.mul(&inverse)).collect())
            }
            Ok(Location::Outside(Violation::Facet(i))) => Some(negated(&self.facets[i].normal)),
            _ => None,
        }
    }

    /// The edges, as pairs of vertex indices i < j: two vertices whose smallest common face has no other vertex. That
    /// face is the intersection of the facets through both, or the polytope itself when no facet holds both.
    pub fn edges(&self) -> Vec<(usize, usize)> {
        let count = self.vertices.len();
        let mut on = vec![vec![false; self.facets.len()]; count];
        for (f, facet) in self.facets.iter().enumerate() {
            for &v in &facet.vertices {
                on[v][f] = true;
            }
        }
        let mut edges = Vec::new();
        for i in 0..count {
            for j in i + 1..count {
                let common: Vec<usize> = (0..self.facets.len()).filter(|&f| on[i][f] && on[j][f]).collect();
                if (0..count).all(|w| w == i || w == j || common.iter().any(|&f| !on[w][f])) {
                    edges.push((i, j));
                }
            }
        }
        edges
    }

    /// The intersection with `other`, or `None` when it is empty: this polytope cut by `other`'s equations and then by
    /// its facets.
    pub fn intersect(&self, other: &Polytope) -> Result<Option<Polytope>, PolytopeError> {
        if self.ambient() != other.ambient() {
            return Err(PolytopeError::DimensionMismatch);
        }
        if separated(self, other) || separated(other, self) {
            return Ok(None);
        }
        let mut current = self.clone();
        let cuts = other.equations.iter().map(|e| (&e.normal, &e.offset, true));
        for (normal, offset, equality) in cuts.chain(other.facets.iter().map(|f| (&f.normal, &f.offset, false))) {
            match current.cut(normal, offset, equality)? {
                Some(next) => current = next,
                None => return Ok(None),
            }
        }
        Ok(Some(current))
    }

    /// The part where a·x ≤ b, or a·x = b with `equality`, or `None` when it is empty. Its vertices are the vertices
    /// on the kept side and the points where edges cross a·x = b: a vertex of the cut that is no vertex here lies on
    /// the hyperplane and in a face that the hyperplane meets in one point, which is an edge.
    fn cut(&self, a: &[Q5], b: &Q5, equality: bool) -> Result<Option<Polytope>, PolytopeError> {
        let values: Vec<Q5> = self.vertices.iter().map(|v| dot(a, v).sub(b)).collect();
        let kept = |s: &Q5| if equality { s.is_zero() } else { s.sign() <= 0 };
        if values.iter().all(kept) {
            return Ok(Some(self.clone()));
        }
        let mut points: Vec<Point> =
            self.vertices.iter().zip(&values).filter(|(_, s)| kept(s)).map(|(v, _)| v.clone()).collect();
        for (i, j) in self.edges() {
            let (si, sj) = (&values[i], &values[j]);
            if si.sign() * sj.sign() < 0 {
                let t = quotient(si, &si.sub(sj));
                points.push(along(&self.vertices[i], &t, &sub(&self.vertices[j], &self.vertices[i])));
            }
        }
        if points.is_empty() {
            return Ok(None);
        }
        Polytope::hull(&points).map(Some)
    }
}

/// Whether `b` lies strictly beyond one of `a`'s equations or facets, so that the two are disjoint.
fn separated(a: &Polytope, b: &Polytope) -> bool {
    let beyond =
        |normal: &[Q5], offset: &Q5, side: i32| b.vertices.iter().all(|v| dot(normal, v).sub(offset).sign() == side);
    a.equations.iter().any(|e| beyond(&e.normal, &e.offset, 1) || beyond(&e.normal, &e.offset, -1))
        || a.facets.iter().any(|f| beyond(&f.normal, &f.offset, 1))
}

/// The projective map x ↦ (M·x + t)/(c·x + d) from n-space to m-space, defined where c·x + d > 0. A map whose
/// denominator is negative on a convex region is the same map with every entry negated, so the sign is no restriction.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectiveMap {
    matrix: Vec<Point>,
    offset: Point,
    denominator: Point,
    denominator_offset: Q5,
}

impl ProjectiveMap {
    /// The map with the m×n `matrix` M, `offset` t, `denominator` c and `denominator_offset` d, with n, m ≥ 1. A
    /// constant negative denominator is negated with the numerator; a zero one is refused.
    pub fn new(
        matrix: Vec<Point>,
        offset: Point,
        denominator: Point,
        denominator_offset: Q5,
    ) -> Result<ProjectiveMap, PolytopeError> {
        let n = denominator.len();
        if n == 0 || matrix.is_empty() || offset.len() != matrix.len() || matrix.iter().any(|row| row.len() != n) {
            return Err(PolytopeError::DimensionMismatch);
        }
        if is_zero(&denominator) {
            match denominator_offset.sign() {
                0 => return Err(PolytopeError::ZeroDenominator),
                -1 => {
                    return Ok(ProjectiveMap {
                        matrix: matrix.iter().map(|row| negated(row)).collect(),
                        offset: negated(&offset),
                        denominator,
                        denominator_offset: denominator_offset.neg(),
                    });
                }
                _ => {}
            }
        }
        Ok(ProjectiveMap { matrix, offset, denominator, denominator_offset })
    }

    /// The affine map x ↦ M·x + t.
    pub fn affine(matrix: Vec<Point>, offset: Point) -> Result<ProjectiveMap, PolytopeError> {
        let n = matrix.first().map_or(0, Vec::len);
        ProjectiveMap::new(matrix, offset, vec![Q5::zero(); n], Q5::one())
    }

    /// The central projection x ↦ x/(h·x) onto the hyperplane h·x = 1, defined where h·x > 0.
    pub fn central(h: Point) -> Result<ProjectiveMap, PolytopeError> {
        let n = h.len();
        if n == 0 {
            return Err(PolytopeError::DimensionMismatch);
        }
        if is_zero(&h) {
            return Err(PolytopeError::ZeroDirection);
        }
        let matrix = (0..n).map(|i| unit(i, n)).collect();
        ProjectiveMap::new(matrix, vec![Q5::zero(); n], h, Q5::zero())
    }

    pub fn matrix(&self) -> &[Point] {
        &self.matrix
    }

    pub fn offset(&self) -> &[Q5] {
        &self.offset
    }

    pub fn denominator(&self) -> &[Q5] {
        &self.denominator
    }

    pub fn denominator_offset(&self) -> &Q5 {
        &self.denominator_offset
    }

    pub fn source_dimension(&self) -> usize {
        self.denominator.len()
    }

    pub fn target_dimension(&self) -> usize {
        self.matrix.len()
    }

    /// c·x + d.
    pub fn scale_at(&self, x: &[Q5]) -> Result<Q5, PolytopeError> {
        if x.len() != self.source_dimension() {
            return Err(PolytopeError::DimensionMismatch);
        }
        Ok(dot(&self.denominator, x).add(&self.denominator_offset))
    }

    /// The image of `x`, or `None` where c·x + d ≤ 0.
    pub fn apply(&self, x: &[Q5]) -> Result<Option<Point>, PolytopeError> {
        let scale = self.scale_at(x)?;
        let Some(inverse) = (scale.sign() > 0).then(|| scale.inv()).flatten() else { return Ok(None) };
        Ok(Some(self.matrix.iter().zip(&self.offset).map(|(row, t)| dot(row, x).add(t).mul(&inverse)).collect()))
    }

    /// Whether the map is defined on all of `region`. The denominator is affine, so its least value is at a vertex.
    pub fn defined_on(&self, region: &Polytope) -> Result<bool, PolytopeError> {
        if region.ambient() != self.source_dimension() {
            return Err(PolytopeError::DimensionMismatch);
        }
        Ok(region.vertices.iter().all(|v| dot(&self.denominator, v).add(&self.denominator_offset).sign() > 0))
    }

    fn require_on(&self, region: &Polytope) -> Result<(), PolytopeError> {
        if self.defined_on(region)? {
            Ok(())
        } else {
            Err(PolytopeError::OutsideMapDomain)
        }
    }

    /// The image of a region on which the map is defined: the hull of its vertices' images, since a projective map
    /// with a positive denominator on a convex set maps each segment of it onto the segment between the images.
    pub fn image(&self, region: &Polytope) -> Result<Polytope, PolytopeError> {
        self.require_on(region)?;
        let mut images = Vec::with_capacity(region.vertices.len());
        for v in &region.vertices {
            images.push(self.apply(v)?.ok_or(PolytopeError::Internal("a vertex left the map's domain"))?);
        }
        Polytope::hull(&images)
    }

    /// F(u; s) = (M·u + t·s; c·u + d·s): the linear map on (n+1)-space under which the map sends the ray of (x; 1)
    /// to the ray of (f(x); 1).
    fn homogeneous(&self, u: &[Q5], s: &Q5) -> Point {
        let mut out: Point = self.matrix.iter().zip(&self.offset).map(|(row, t)| dot(row, u).add(&t.mul(s))).collect();
        out.push(dot(&self.denominator, u).add(&self.denominator_offset.mul(s)));
        out
    }

    /// The point of `cell` that the map sends to `y`, for a cell on which the map is injective and a `y` in the
    /// cell's image. f(x) = y exactly when (M − y·cᵀ)·x = d·y − t, which is linear in the chart coordinates z of
    /// x = p + Σ z_j·b_j, with a unique solution because the map is injective on the cell.
    fn preimage(&self, cell: &Polytope, y: &[Q5]) -> Result<Point, PolytopeError> {
        let p = cell.relative_interior_point();
        let reduced: Vec<Point> = self
            .matrix
            .iter()
            .zip(y)
            .map(|(row, yi)| row.iter().zip(&self.denominator).map(|(m, c)| m.sub(&yi.mul(c))).collect())
            .collect();
        let lhs: Vec<Point> = reduced.iter().map(|row| cell.directions.iter().map(|b| dot(row, b)).collect()).collect();
        let rhs: Point = reduced
            .iter()
            .zip(y)
            .zip(&self.offset)
            .map(|((row, yi), t)| yi.mul(&self.denominator_offset).sub(t).sub(&dot(row, &p)))
            .collect();
        let z = solve_unique(&lhs, &rhs, cell.dimension())
            .ok_or(PolytopeError::Internal("no unique preimage in a cell where the map is injective"))?;
        let x = combine(&p, &z, &cell.directions);
        if !cell.contains(&x)? || self.apply(&x)?.as_deref() != Some(y) {
            return Err(PolytopeError::Internal("a preimage did not verify"));
        }
        Ok(x)
    }
}

/// A metric (x − y)ᵀ·G·(x − y) given by a symmetric positive definite Gram matrix G. Distances are compared squared,
/// so they stay exact.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Metric {
    gram: Vec<Point>,
}

impl Metric {
    /// The Euclidean metric of n-space.
    pub fn euclidean(n: usize) -> Metric {
        Metric { gram: (0..n).map(|i| unit(i, n)).collect() }
    }

    /// The metric with Gram matrix `gram`, which must be square of size at least 1, symmetric and positive definite:
    /// elimination without row exchanges then keeps every pivot positive (Sylvester's criterion).
    pub fn gram(gram: Vec<Point>) -> Result<Metric, PolytopeError> {
        let n = gram.len();
        if n == 0 || gram.iter().any(|row| row.len() != n) {
            return Err(PolytopeError::DimensionMismatch);
        }
        if (0..n).any(|i| (0..i).any(|j| gram[i][j] != gram[j][i])) {
            return Err(PolytopeError::NotAMetric);
        }
        let mut a = gram.clone();
        for i in 0..n {
            if a[i][i].sign() <= 0 {
                return Err(PolytopeError::NotAMetric);
            }
            let inverse = a[i][i].inv().ok_or(PolytopeError::Internal("a positive pivot has no inverse"))?;
            for r in i + 1..n {
                let factor = a[r][i].mul(&inverse);
                if !factor.is_zero() {
                    let pivot_row = a[i].clone();
                    a[r] = a[r].iter().zip(&pivot_row).map(|(x, y)| x.sub(&factor.mul(y))).collect();
                }
            }
        }
        Ok(Metric { gram })
    }

    pub fn dimension(&self) -> usize {
        self.gram.len()
    }

    pub fn matrix(&self) -> &[Point] {
        &self.gram
    }

    /// The squared distance of `x` and `y`.
    pub fn squared_distance(&self, x: &[Q5], y: &[Q5]) -> Result<Q5, PolytopeError> {
        if x.len() != self.dimension() || y.len() != self.dimension() {
            return Err(PolytopeError::DimensionMismatch);
        }
        let d = sub(x, y);
        let g: Point = self.gram.iter().map(|row| dot(row, &d)).collect();
        Ok(dot(&d, &g))
    }
}

/// The result of a property check: the property holds on everything the check was given, or it fails with an exact
/// witness.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Check<W> {
    Holds,
    Fails(W),
}

/// Two distinct points of the domain with one image.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SameImage {
    pub x: Point,
    pub y: Point,
    pub image: Point,
}

/// Two points of the region whose distance the map changes, with both squared distances.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DistanceChange {
    pub x: Point,
    pub y: Point,
    /// The squared distance of x and y in the source metric.
    pub before: Q5,
    /// The squared distance of their images in the target metric.
    pub after: Q5,
}

/// A cell whose image is no cell of the codomain.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnmatchedCell {
    /// The cell's index.
    pub cell: usize,
    pub image: Polytope,
}

/// Whether the map is injective on `region`, decided over the whole region. Let F be the homogeneous map, p the
/// region's centroid and W the span of (p; 1) and the (b_j; 0) for the region's directions b_j. Two points x ≠ x' of
/// the region with one image give the nonzero kernel vector s(x')·(x; 1) − s(x)·(x'; 1) of F in W, where s is the
/// denominator. Conversely, a kernel vector z_0·(p; 1) + Σ z_j·(b_j; 0) of F in W gives v = Σ z_j·b_j ≠ 0 with
/// F(p + τ·v; 1) = (1 − τ·z_0)·F(p; 1), so every point of the region on the line p + τ·v has the image of p. The
/// witness is that chord's two ends.
pub fn injective(map: &ProjectiveMap, region: &Polytope) -> Result<Check<SameImage>, PolytopeError> {
    map.require_on(region)?;
    let p = region.relative_interior_point();
    let mut columns = vec![map.homogeneous(&p, &Q5::one())];
    columns.extend(region.directions.iter().map(|b| map.homogeneous(b, &Q5::zero())));
    let Some(z) = null_vector(&columns) else { return Ok(Check::Holds) };
    let v = combine(&vec![Q5::zero(); region.ambient()], &z[1..], &region.directions);
    if is_zero(&v) {
        return Err(PolytopeError::Internal("a kernel vector of the homogeneous map has no direction"));
    }
    let (lo, hi) = chord(region, &p, &v)?;
    same_image(map, along(&p, &lo, &v), along(&p, &hi, &v))
}

/// Whether the map is injective on the union of `cells`, which for a compact union is what an embedding is. It must
/// be injective on each cell (else that cell's witness), and f(σ) ∩ f(τ) = f(σ ∩ τ) for every pair. The right side
/// lies in the left and both are polytopes, so a vertex y of the left outside the right refutes it, and the witness is
/// y's preimages in σ and in τ: they differ, since y is no image of a point of σ ∩ τ.
pub fn embedding(map: &ProjectiveMap, cells: &[Polytope]) -> Result<Check<SameImage>, PolytopeError> {
    if cells.is_empty() {
        return Err(PolytopeError::Empty);
    }
    let mut images = Vec::with_capacity(cells.len());
    for cell in cells {
        if let Check::Fails(witness) = injective(map, cell)? {
            return Ok(Check::Fails(witness));
        }
        images.push(map.image(cell)?);
    }
    for i in 0..cells.len() {
        for j in i + 1..cells.len() {
            let Some(meet) = images[i].intersect(&images[j])? else { continue };
            let expected = match cells[i].intersect(&cells[j])? {
                Some(common) => Some(map.image(&common)?),
                None => None,
            };
            for y in meet.vertices() {
                let covered = match &expected {
                    Some(image) => image.contains(y)?,
                    None => false,
                };
                if !covered {
                    return same_image(map, map.preimage(&cells[i], y)?, map.preimage(&cells[j], y)?);
                }
            }
        }
    }
    Ok(Check::Holds)
}

/// Whether the map keeps distances on `region` from the `source` metric to the `target` metric, decided over the whole
/// region. If the denominator changes along a direction b of the region, the map is not affine on the chord p + τ·b
/// through the centroid: the images lie on one line at the parameter g(τ) = τ/(s + τ·c·b), increasing in τ, and g at
/// the chord's midpoint is the mean of g at its ends only when the denominator's arithmetic and harmonic means at the
/// ends agree, which needs c·b = 0. So one half of the chord changes its length, and that half is the witness.
/// Otherwise the map is affine on the region's affine hull, and it keeps distances exactly when it keeps the quadratic
/// form on the directions, which polarization reduces to the chords along each b_i and each b_i + b_j.
pub fn isometry(
    map: &ProjectiveMap,
    region: &Polytope,
    source: &Metric,
    target: &Metric,
) -> Result<Check<DistanceChange>, PolytopeError> {
    if source.dimension() != map.source_dimension() || target.dimension() != map.target_dimension() {
        return Err(PolytopeError::DimensionMismatch);
    }
    map.require_on(region)?;
    let p = region.relative_interior_point();
    let b = &region.directions;
    if let Some(v) = b.iter().find(|v| !dot(&map.denominator, v).is_zero()) {
        let (lo, hi) = chord(region, &p, v)?;
        let middle = lo.add(&hi).mul(&Q5::of(1, 0, 2));
        let (start, mid, end) = (along(&p, &lo, v), along(&p, &middle, v), along(&p, &hi, v));
        for (x, y) in [(start, mid.clone()), (mid, end)] {
            if let Some(change) = distance_change(map, source, target, x, y)? {
                return Ok(Check::Fails(change));
            }
        }
        return Err(PolytopeError::Internal("a map that is not affine on a chord kept both of its halves"));
    }
    for i in 0..b.len() {
        let mut chords = vec![b[i].clone()];
        chords.extend(b[i + 1..].iter().map(|bj| add(&b[i], bj)));
        for v in chords {
            let (lo, hi) = chord(region, &p, &v)?;
            if let Some(change) = distance_change(map, source, target, along(&p, &lo, &v), along(&p, &hi, &v))? {
                return Ok(Check::Fails(change));
            }
        }
    }
    Ok(Check::Holds)
}

/// Whether the map is a homomorphism of cell complexes from `cells` to `codomain`: it is defined on each cell and maps
/// it onto a cell of the codomain, compared by vertex set. It checks the cells it is given and no face relation of its
/// own. When both lists are complexes with every face listed as a cell, the face relations follow: a face's image is a
/// codomain cell inside its cell's image, and in a complex that makes it a face of that image. The witness is the first
/// cell whose image is no codomain cell, with that image.
pub fn homomorphism(
    map: &ProjectiveMap,
    cells: &[Polytope],
    codomain: &[Polytope],
) -> Result<Check<UnmatchedCell>, PolytopeError> {
    if cells.is_empty() {
        return Err(PolytopeError::Empty);
    }
    if codomain.iter().any(|c| c.ambient() != map.target_dimension()) {
        return Err(PolytopeError::DimensionMismatch);
    }
    for (i, cell) in cells.iter().enumerate() {
        let image = map.image(cell)?;
        if !codomain.contains(&image) {
            return Ok(Check::Fails(UnmatchedCell { cell: i, image }));
        }
    }
    Ok(Check::Holds)
}

/// The two points as a verified witness of a failed injectivity: distinct, with one image.
fn same_image(map: &ProjectiveMap, x: Point, y: Point) -> Result<Check<SameImage>, PolytopeError> {
    let fx = map.apply(&x)?;
    let fy = map.apply(&y)?;
    match (fx, fy) {
        (Some(fx), Some(fy)) if x != y && fx == fy => Ok(Check::Fails(SameImage { x, y, image: fx })),
        _ => Err(PolytopeError::Internal("an injectivity witness did not verify")),
    }
}

/// The change of the squared distance of `x` and `y` under the map, if there is one.
fn distance_change(
    map: &ProjectiveMap,
    source: &Metric,
    target: &Metric,
    x: Point,
    y: Point,
) -> Result<Option<DistanceChange>, PolytopeError> {
    let before = source.squared_distance(&x, &y)?;
    let (Some(fx), Some(fy)) = (map.apply(&x)?, map.apply(&y)?) else {
        return Err(PolytopeError::Internal("a point of the region left the map's domain"));
    };
    let after = target.squared_distance(&fx, &fy)?;
    Ok((before != after).then_some(DistanceChange { x, y, before, after }))
}

/// The ends lo < hi of the chord of `region` through its relative interior point `p` along a nonzero direction `v` of
/// its affine hull.
fn chord(region: &Polytope, p: &[Q5], v: &[Q5]) -> Result<(Q5, Q5), PolytopeError> {
    match region.line_meet(p, v)? {
        Some((lo, hi)) if lo < hi => Ok((lo, hi)),
        _ => Err(PolytopeError::Internal("a chord through the centroid along a direction of the hull is degenerate")),
    }
}

fn sub(x: &[Q5], y: &[Q5]) -> Point {
    x.iter().zip(y).map(|(a, b)| a.sub(b)).collect()
}

fn add(x: &[Q5], y: &[Q5]) -> Point {
    x.iter().zip(y).map(|(a, b)| a.add(b)).collect()
}

fn negated(x: &[Q5]) -> Point {
    x.iter().map(Q5::neg).collect()
}

/// p + t·d.
fn along(p: &[Q5], t: &Q5, d: &[Q5]) -> Point {
    p.iter().zip(d).map(|(a, b)| a.add(&t.mul(b))).collect()
}

/// p + Σ z_j·directions_j.
fn combine(p: &[Q5], z: &[Q5], directions: &[Point]) -> Point {
    let mut x = p.to_vec();
    for (zj, b) in z.iter().zip(directions) {
        x = along(&x, zj, b);
    }
    x
}

fn is_zero(x: &[Q5]) -> bool {
    x.iter().all(Q5::is_zero)
}

fn unit(i: usize, n: usize) -> Point {
    (0..n).map(|j| if i == j { Q5::one() } else { Q5::zero() }).collect()
}

/// The centroid of a nonempty list of points.
fn centroid(points: &[Point]) -> Point {
    let count = Q5::int(i64::try_from(points.len()).expect("a point count fits in i64"));
    let inverse = count.inv().expect("a nonempty list");
    let mut sum = vec![Q5::zero(); points[0].len()];
    for q in points {
        sum = add(&sum, q);
    }
    sum.iter().map(|x| x.mul(&inverse)).collect()
}

/// a/b for a nonzero b.
fn quotient(a: &Q5, b: &Q5) -> Q5 {
    a.div(b).expect("the divisor is nonzero")
}

/// (normal, offset) scaled by a positive factor so that the normal's first nonzero entry is 1 or −1.
fn normalized(normal: Point, offset: Q5) -> (Point, Q5) {
    let Some(first) = normal.iter().find(|x| !x.is_zero()) else { return (normal, offset) };
    let size = if first.sign() > 0 { first.clone() } else { first.neg() };
    let inverse = size.inv().expect("nonzero");
    (normal.iter().map(|x| x.mul(&inverse)).collect(), offset.mul(&inverse))
}

/// The reduced row echelon form of `rows`, each of length `cols`, without its zero rows, and its pivot columns.
fn rref(rows: &[Point], cols: usize) -> (Vec<Point>, Vec<usize>) {
    let mut a: Vec<Point> = rows.to_vec();
    let mut pivots = Vec::new();
    for col in 0..cols {
        let r = pivots.len();
        if r == a.len() {
            break;
        }
        let Some(pivot) = (r..a.len()).find(|&i| !a[i][col].is_zero()) else { continue };
        a.swap(r, pivot);
        let inverse = a[r][col].inv().expect("the pivot is nonzero");
        a[r] = a[r].iter().map(|x| x.mul(&inverse)).collect();
        for i in 0..a.len() {
            if i != r && !a[i][col].is_zero() {
                let factor = a[i][col].clone();
                let pivot_row = a[r].clone();
                a[i] = a[i].iter().zip(&pivot_row).map(|(x, y)| x.sub(&factor.mul(y))).collect();
            }
        }
        pivots.push(col);
    }
    a.truncate(pivots.len());
    (a, pivots)
}

/// A nonzero z with Σ z_j·columns_j = 0, or `None` when the columns are independent.
fn null_vector(columns: &[Point]) -> Option<Point> {
    let n = columns.len();
    let height = columns.first().map_or(0, Vec::len);
    let rows: Vec<Point> = (0..height).map(|r| columns.iter().map(|c| c[r].clone()).collect()).collect();
    let (reduced, pivots) = rref(&rows, n);
    let free = (0..n).find(|j| !pivots.contains(j))?;
    let mut z = vec![Q5::zero(); n];
    z[free] = Q5::one();
    for (row, &p) in reduced.iter().zip(&pivots) {
        z[p] = row[free].neg();
    }
    Some(z)
}

/// The unique z with a·z = b, where each row of `a` has length k, or `None` when there is none or more than one.
fn solve_unique(a: &[Point], b: &[Q5], k: usize) -> Option<Point> {
    let rows: Vec<Point> =
        a.iter().zip(b).map(|(row, x)| row.iter().chain(std::iter::once(x)).cloned().collect()).collect();
    let (reduced, pivots) = rref(&rows, k + 1);
    (pivots == (0..k).collect::<Vec<_>>()).then(|| reduced.iter().map(|row| row[k].clone()).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(xs: &[i64]) -> Point {
        xs.iter().map(|&x| Q5::int(x)).collect()
    }

    #[test]
    fn row_reduction_kernels_and_unique_solutions() {
        let (rows, pivots) = rref(&[v(&[0, 2, 4]), v(&[0, 1, 2]), v(&[1, 1, 1])], 3);
        assert_eq!(pivots, vec![0, 1]);
        assert_eq!(rows, vec![v(&[1, 0, -1]), v(&[0, 1, 2])]);
        assert_eq!(rref(&[], 3), (vec![], vec![]));
        assert_eq!(null_vector(&[v(&[1, 0]), v(&[0, 1]), v(&[1, 1])]), Some(v(&[-1, -1, 1])));
        assert_eq!(null_vector(&[v(&[1, 0]), v(&[0, 1])]), None);
        assert_eq!(null_vector(&[v(&[1, 2]), v(&[0, 0])]), Some(v(&[0, 1])));
        let a = [v(&[1, 1]), v(&[1, -1]), v(&[2, 0])];
        assert_eq!(solve_unique(&a, &v(&[3, 1, 4]), 2), Some(v(&[2, 1])));
        assert_eq!(solve_unique(&a, &v(&[3, 1, 5]), 2), None, "inconsistent");
        assert_eq!(solve_unique(&[v(&[1, 1]), v(&[2, 2])], &v(&[1, 2]), 2), None, "not unique");
        assert_eq!(solve_unique(&[vec![], vec![]], &v(&[0, 0]), 0), Some(vec![]));
        assert_eq!(solve_unique(&[vec![]], &v(&[1]), 0), None);
    }

    #[test]
    fn facet_normals_are_scaled_by_a_positive_factor() {
        assert_eq!(normalized(v(&[0, -2, 4]), Q5::int(6)), (v(&[0, -1, 2]), Q5::int(3)));
        assert_eq!(normalized(v(&[0, 3, 1]), Q5::int(-6)), (vec![Q5::zero(), Q5::one(), Q5::of(1, 0, 3)], Q5::int(-2)));
        let phi = Q5::of(1, 1, 2);
        let (normal, offset) = normalized(vec![phi.clone(), Q5::one()], phi.clone());
        assert_eq!((normal[0].clone(), normal[1].clone(), offset), (Q5::one(), phi.inv().unwrap(), Q5::one()));
        assert_eq!(normalized(v(&[0, 0]), Q5::int(5)), (v(&[0, 0]), Q5::int(5)));
    }
}

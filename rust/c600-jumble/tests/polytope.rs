//! KN3.3's exact polytope operations and LB4.1's map property checks (`c600_jumble::polytope`), on regions whose faces
//! and answers are known by hand: the 4-simplex, the 16-cell, the tesseract, the 24-cell, a cell of the 600-cell and
//! small flat pieces. Every failed check's witness is checked again here from its exact points.

use c600_jumble::dd::Constraint;
use c600_jumble::polytope::{
    embedding, homomorphism, injective, isometry, Check, DistanceChange, Location, Metric, Point, Polytope,
    PolytopeError, ProjectiveMap, SameImage, UnmatchedCell, Violation,
};
use c600_jumble::q5::{dot, Q5};

fn q(n: i64) -> Q5 {
    Q5::int(n)
}

fn p(xs: &[i64]) -> Point {
    xs.iter().map(|&x| Q5::int(x)).collect()
}

fn unit(i: usize, n: usize) -> Point {
    (0..n).map(|j| if i == j { Q5::one() } else { Q5::zero() }).collect()
}

fn origin() -> Point {
    p(&[0, 0, 0, 0])
}

fn times(t: &Q5, x: &[Q5]) -> Point {
    x.iter().map(|a| a.mul(t)).collect()
}

fn plus(x: &[Q5], y: &[Q5]) -> Point {
    x.iter().zip(y).map(|(a, b)| a.add(b)).collect()
}

fn minus(x: &[Q5], y: &[Q5]) -> Point {
    x.iter().zip(y).map(|(a, b)| a.sub(b)).collect()
}

fn negated(x: &[Q5]) -> Point {
    x.iter().map(Q5::neg).collect()
}

fn hull(points: &[Point]) -> Polytope {
    Polytope::hull(points).expect("a hull")
}

/// The 4-simplex {0, e1, e2, e3, e4}.
fn simplex() -> Polytope {
    let mut points = vec![origin()];
    points.extend((0..4).map(|i| unit(i, 4)));
    hull(&points)
}

/// The 16-cell ±t·e_i.
fn cross(t: i64) -> Polytope {
    let points: Vec<Point> = (0..4).flat_map(|i| [times(&q(t), &unit(i, 4)), times(&q(-t), &unit(i, 4))]).collect();
    hull(&points)
}

/// The tesseract (±1, ±1, ±1, ±1).
fn cube() -> Polytope {
    let points: Vec<Point> =
        (0..16i64).map(|bits| (0..4).map(|i| q(if (bits >> i) & 1 == 1 { 1 } else { -1 })).collect()).collect();
    hull(&points)
}

/// The 24-cell: the permutations of (±1, ±1, 0, 0).
fn cell24() -> Polytope {
    let mut points = Vec::new();
    for i in 0..4 {
        for j in i + 1..4 {
            for (a, b) in [(1, 1), (1, -1), (-1, 1), (-1, -1)] {
                let mut x = origin();
                x[i] = q(a);
                x[j] = q(b);
                points.push(x);
            }
        }
    }
    hull(&points)
}

/// The rectangle [x0, x1] × [y0, y1].
fn rectangle(x0: i64, y0: i64, x1: i64, y1: i64) -> Polytope {
    hull(&[p(&[x0, y0]), p(&[x1, y0]), p(&[x1, y1]), p(&[x0, y1])])
}

/// A cell of the 600-cell of circumradius 1: a regular tetrahedron with edge 1/φ.
fn cell600() -> Polytope {
    let half_phi = Q5::of(1, 1, 4);
    let half = Q5::of(1, 0, 2);
    let half_inverse_phi = Q5::of(-1, 1, 4);
    hull(&[
        p(&[1, 0, 0, 0]),
        vec![half_phi.clone(), half.clone(), half_inverse_phi.clone(), Q5::zero()],
        vec![half_phi.clone(), half.clone(), half_inverse_phi.neg(), Q5::zero()],
        vec![half_phi, half_inverse_phi, Q5::zero(), half],
    ])
}

/// conv{e4, e1 + e4, e2 + e4, e3 + e4}, a tetrahedron in the hyperplane x4 = 1.
fn lower() -> Polytope {
    let e4 = unit(3, 4);
    hull(&[e4.clone(), plus(&unit(0, 4), &e4), plus(&unit(1, 4), &e4), plus(&unit(2, 4), &e4)])
}

/// conv{e1 + e4, e2 + e4, e3 + e4, (1, 1, 1, 1)}, which meets `lower` in a common triangle.
fn upper() -> Polytope {
    let e4 = unit(3, 4);
    hull(&[plus(&unit(0, 4), &e4), plus(&unit(1, 4), &e4), plus(&unit(2, 4), &e4), p(&[1, 1, 1, 1])])
}

/// conv{0, e1, e2, e3}, a tetrahedron in the hyperplane x4 = 0.
fn flat() -> Polytope {
    hull(&[origin(), unit(0, 4), unit(1, 4), unit(2, 4)])
}

/// The map from n-space to (n−1)-space that drops the last coordinate.
fn drop_last(n: usize) -> ProjectiveMap {
    ProjectiveMap::affine((0..n - 1).map(|i| unit(i, n)).collect(), vec![Q5::zero(); n - 1]).expect("a map")
}

/// The linear map with these integer rows.
fn linear(rows: &[&[i64]]) -> ProjectiveMap {
    ProjectiveMap::affine(rows.iter().map(|row| p(row)).collect(), vec![Q5::zero(); rows.len()]).expect("a map")
}

/// Whether x and y are nonzero and y = λ·x for some λ > 0.
fn on_one_ray(x: &[Q5], y: &[Q5]) -> bool {
    let parallel = (0..x.len()).all(|i| (0..x.len()).all(|j| x[i].mul(&y[j]) == x[j].mul(&y[i])));
    parallel && dot(x, y).sign() > 0
}

/// The faces agree with the vertices: each equation holds at every vertex; each facet holds exactly its listed
/// vertices, has the others strictly inside and a normal whose first nonzero entry is 1 or −1; the centroid is in the
/// relative interior; and the equations and the directions split the ambient dimension.
fn consistent(poly: &Polytope) {
    for e in poly.equations() {
        assert!(poly.vertices().iter().all(|v| dot(&e.normal, v) == e.offset), "an equation of the hull");
    }
    for f in poly.facets() {
        for (i, v) in poly.vertices().iter().enumerate() {
            let side = dot(&f.normal, v).sub(&f.offset).sign();
            assert!(side <= 0, "a vertex beyond a facet");
            assert_eq!(side == 0, f.vertices.contains(&i), "facet incidence");
        }
        let first = f.normal.iter().find(|x| !x.is_zero()).expect("a nonzero normal");
        assert_eq!(first.mul(first), Q5::one(), "a normal scaled to a first entry of 1 or -1");
    }
    assert_eq!(poly.locate(&poly.relative_interior_point()).expect("a point"), Location::RelativeInterior);
    assert_eq!(poly.equations().len() + poly.dimension(), poly.ambient());
}

/// A failed injectivity's witness, checked again: two distinct points of the regions with the stated image.
fn verified(map: &ProjectiveMap, regions: &[&Polytope], witness: &SameImage) {
    assert_ne!(witness.x, witness.y);
    for x in [&witness.x, &witness.y] {
        assert!(regions.iter().any(|r| r.contains(x).expect("a point")), "a witness point outside the regions");
        assert_eq!(map.apply(x).expect("a point").as_ref(), Some(&witness.image));
    }
}

/// A failed isometry's witness, checked again: two points of the region whose squared distance changes as stated.
fn verified_change(map: &ProjectiveMap, region: &Polytope, source: &Metric, target: &Metric, change: &DistanceChange) {
    assert!(region.contains(&change.x).expect("a point") && region.contains(&change.y).expect("a point"));
    assert_eq!(source.squared_distance(&change.x, &change.y).expect("a distance"), change.before);
    let fx = map.apply(&change.x).expect("a point").expect("in the domain");
    let fy = map.apply(&change.y).expect("a point").expect("in the domain");
    assert_eq!(target.squared_distance(&fx, &fy).expect("a distance"), change.after);
    assert_ne!(change.before, change.after);
}

fn not_injective(map: &ProjectiveMap, region: &Polytope) -> SameImage {
    match injective(map, region).expect("the check runs") {
        Check::Fails(witness) => {
            verified(map, &[region], &witness);
            witness
        }
        Check::Holds => panic!("injective on a region where it is not"),
    }
}

#[test]
fn regular_polytopes_have_their_faces() {
    // (polytope, vertices, facets, edges, vertices on each facet, squared edge length)
    let cases = [
        (simplex(), 5, 5, 10, 4, None),
        (cross(1), 8, 16, 24, 4, Some(q(2))),
        (cube(), 16, 8, 32, 8, Some(q(4))),
        (cell24(), 24, 24, 96, 6, Some(q(2))),
    ];
    for (poly, vertices, facets, edges, per_facet, edge) in cases {
        consistent(&poly);
        assert_eq!((poly.ambient(), poly.dimension(), poly.equations().len()), (4, 4, 0));
        assert_eq!((poly.vertices().len(), poly.facets().len(), poly.edges().len()), (vertices, facets, edges));
        assert!(poly.facets().iter().all(|f| f.vertices.len() == per_facet));
        for v in poly.vertices() {
            assert!(matches!(poly.locate(v).expect("a point"), Location::Boundary(on) if on.len() >= 4));
        }
        if let Some(edge) = edge {
            for (i, j) in poly.edges() {
                let d = minus(&poly.vertices()[i], &poly.vertices()[j]);
                assert_eq!(dot(&d, &d), edge);
            }
        }
    }
}

#[test]
fn repeated_and_inner_points_are_no_vertices() {
    let base: Vec<Point> = [origin()].into_iter().chain((0..4).map(|i| times(&q(5), &unit(i, 4)))).collect();
    let points = [
        origin(),
        p(&[1, 1, 1, 1]),
        base[1].clone(),
        p(&[1, 1, 0, 0]),
        base[2].clone(),
        p(&[2, 0, 0, 0]),
        base[3].clone(),
        base[2].clone(),
        base[4].clone(),
    ];
    let poly = hull(&points);
    assert_eq!(poly.vertices(), &base[..], "the extreme points in the order they first appear");
    consistent(&poly);
    assert_eq!(poly.locate(&p(&[1, 1, 1, 1])).expect("a point"), Location::RelativeInterior);
    assert!(matches!(poly.locate(&p(&[1, 1, 0, 0])).expect("a point"), Location::Boundary(on) if on.len() == 2));
    assert!(matches!(poly.locate(&p(&[2, 0, 0, 0])).expect("a point"), Location::Boundary(on) if on.len() == 3));
    assert!(matches!(poly.locate(&p(&[6, 0, 0, 0])).expect("a point"), Location::Outside(Violation::Facet(_))));
}

#[test]
fn lower_dimensional_polytopes_keep_their_hull_equations() {
    let triangle = hull(&[unit(0, 4), unit(1, 4), unit(2, 4)]);
    consistent(&triangle);
    assert_eq!((triangle.dimension(), triangle.equations().len(), triangle.facets().len()), (2, 2, 3));
    assert_eq!(triangle.edges(), vec![(0, 1), (0, 2), (1, 2)]);
    assert_eq!(triangle.locate(&p(&[1, 1, 1, 1])).expect("a point"), Location::Outside(Violation::Equation(0)));
    assert!(matches!(triangle.locate(&p(&[2, -1, 0, 0])).expect("a point"), Location::Outside(Violation::Facet(_))));
    assert!(matches!(triangle.locate(&unit(0, 4)).expect("a point"), Location::Boundary(on) if on.len() == 2));

    let segment = hull(&[origin(), p(&[1, 1, 1, 1]), p(&[2, 2, 2, 2])]);
    consistent(&segment);
    assert_eq!(segment.vertices(), &[origin(), p(&[2, 2, 2, 2])][..]);
    assert_eq!((segment.dimension(), segment.equations().len(), segment.facets().len()), (1, 3, 2));
    assert_eq!(segment.edges(), vec![(0, 1)]);

    let point = hull(&[p(&[1, 2, 3, 4]), p(&[1, 2, 3, 4])]);
    consistent(&point);
    assert_eq!((point.dimension(), point.equations().len(), point.facets().len()), (0, 4, 0));
    assert!(point.edges().is_empty());
    assert_eq!(point.locate(&p(&[1, 2, 3, 4])).expect("a point"), Location::RelativeInterior);
    assert_eq!(point.locate(&p(&[1, 2, 3, 5])).expect("a point"), Location::Outside(Violation::Equation(3)));
    assert_eq!(point.line_meet(&origin(), &p(&[1, 2, 3, 4])).expect("a line"), Some((q(1), q(1))));
    assert_eq!(point.line_meet(&origin(), &p(&[1, 2, 3, 5])).expect("a line"), None);
}

#[test]
fn lines_and_rays_meet_the_unit_square_exactly() {
    let constraints = [
        Constraint { a: p(&[-1, 0]), b: q(0) },
        Constraint { a: p(&[0, -1]), b: q(0) },
        Constraint { a: p(&[1, 0]), b: q(1) },
        Constraint { a: p(&[0, 1]), b: q(1) },
    ];
    let square = Polytope::from_constraints(&constraints, 2).expect("a square");
    assert_eq!(square, rectangle(0, 0, 1, 1));
    consistent(&square);
    assert!(matches!(square.locate(&p(&[0, 0])).expect("a point"), Location::Boundary(on) if on.len() == 2));
    let half = Q5::of(1, 0, 2);
    let line = |from: &[Q5], along: &[i64]| square.line_meet(from, &p(along)).expect("a line");
    assert_eq!(line(&[Q5::zero(), half.clone()], &[1, 0]), Some((q(0), q(1))));
    assert_eq!(line(&p(&[2, 2]), &[1, 0]), None);
    assert_eq!(line(&p(&[0, 0]), &[1, 1]), Some((q(0), q(1))));
    assert_eq!(line(&p(&[2, 0]), &[-1, 1]), Some((q(1), q(1))), "the line touches one corner");
    assert_eq!(square.ray_meet(&p(&[3, 0]), &p(&[1, 0])).expect("a ray"), None);
    assert_eq!(square.ray_meet(&p(&[3, 0]), &p(&[-1, 0])).expect("a ray"), Some(p(&[1, 0])));
    let inside = vec![half.clone(), half];
    assert_eq!(square.ray_meet(&inside, &p(&[1, 0])).expect("a ray"), Some(inside.clone()));
    assert_eq!(square.positive_functional(), None);
    assert_eq!(rectangle(2, -1, 3, 1).positive_functional(), Some(p(&[1, 0])));
}

#[test]
fn a_cell_of_the_600_cell() {
    let cell = cell600();
    consistent(&cell);
    assert_eq!((cell.dimension(), cell.equations().len(), cell.facets().len(), cell.edges().len()), (3, 1, 4, 6));
    assert!(!cell.equations()[0].offset.is_zero());
    for v in cell.vertices() {
        assert_eq!(dot(v, v), Q5::one(), "on the unit sphere");
    }
    for (i, j) in cell.edges() {
        let d = minus(&cell.vertices()[i], &cell.vertices()[j]);
        assert_eq!(dot(&d, &d), Q5::of(3, -1, 2), "edges of length 1/φ");
    }
    let h = cell.positive_functional().expect("the origin is outside the cell");
    assert!(cell.vertices().iter().all(|v| dot(&h, v) == Q5::one()));

    // LB4.2 and CT-LB4-01: a stereographic pole on the cell's axis. No vertex lies on the closed ray of the pole, yet
    // the region test finds the cell's centroid on it, so a chart with this pole has no domain on the cell.
    let pole = cell.relative_interior_point();
    assert!(cell.vertices().iter().all(|v| !on_one_ray(&pole, v)));
    assert_eq!(cell.ray_meet(&origin(), &pole).expect("a ray"), Some(pole.clone()));
    assert_eq!(cell.locate(&pole).expect("a point"), Location::RelativeInterior);
    assert_eq!(cell.ray_meet(&origin(), &negated(&pole)).expect("a ray"), None);
    assert_eq!(cell.ray_meet(&origin(), &p(&[-1, 0, 0, 0])).expect("a ray"), None);
    assert_eq!(cell.ray_meet(&origin(), &p(&[1, 0, 0, 0])).expect("a ray"), Some(p(&[1, 0, 0, 0])), "a vertex");
}

#[test]
fn intersections_of_polytopes() {
    assert_eq!(cross(2).intersect(&cube()).expect("same dimension"), Some(cell24()));
    assert_eq!(cube().intersect(&cross(2)).expect("same dimension"), Some(cell24()));
    let meet = |a: &Polytope, b: &Polytope| a.intersect(b).expect("same dimension");
    assert_eq!(meet(&rectangle(0, 0, 2, 2), &rectangle(1, 1, 3, 3)), Some(rectangle(1, 1, 2, 2)));
    let edge = meet(&rectangle(0, 0, 1, 1), &rectangle(1, 0, 2, 1)).expect("a common edge");
    assert_eq!((edge.dimension(), edge), (1, hull(&[p(&[1, 0]), p(&[1, 1])])));
    let corner = meet(&rectangle(0, 0, 1, 1), &rectangle(1, 1, 2, 2)).expect("a common corner");
    assert_eq!((corner.dimension(), corner.vertices()), (0, &[p(&[1, 1])][..]));
    assert_eq!(meet(&rectangle(0, 0, 1, 1), &rectangle(2, 2, 3, 3)), None);
    let diagonal = hull(&[p(&[-1, -1]), p(&[3, 3])]);
    let expected = Some(hull(&[p(&[0, 0]), p(&[2, 2])]));
    assert_eq!(meet(&diagonal, &rectangle(0, 0, 2, 2)), expected);
    assert_eq!(meet(&rectangle(0, 0, 2, 2), &diagonal), expected);

    // Two tetrahedra with crossed edges: no face of either separates them, only the plane between the crossed edges.
    let below = hull(&[p(&[-1, 0, 0]), p(&[1, 0, 0]), p(&[0, -1, -1]), p(&[0, 1, -1])]);
    let lifted = |dz: i64| hull(&below.vertices().iter().map(|v| plus(v, &p(&[0, 0, dz]))).collect::<Vec<_>>());
    assert_eq!(meet(&below, &lifted(2)), None);
    assert_eq!(meet(&lifted(2), &below), None);
    assert_eq!(meet(&below, &lifted(1)), Some(hull(&[p(&[0, 0, 0])])), "the crossed edges touch at one point");
    assert_eq!(meet(&lifted(1), &below), Some(hull(&[p(&[0, 0, 0])])));
}

#[test]
fn injectivity_is_decided_over_the_region() {
    let drop = drop_last(4);
    let witness = not_injective(&drop, &simplex());
    assert_eq!(witness.x[..3], witness.y[..3]);
    assert_eq!(injective(&drop, &flat()).expect("the check runs"), Check::Holds);
    not_injective(&drop, &hull(&[p(&[1, 0, 0, 0]), p(&[1, 0, 0, 1]), p(&[1, 1, 0, 0])]));
    assert_eq!(injective(&drop, &hull(&[p(&[1, 2, 3, 4])])).expect("the check runs"), Check::Holds);

    // Central projections are injective on a cell, which lies in a hyperplane that misses the origin.
    let cell = cell600();
    let fixing = ProjectiveMap::central(cell.positive_functional().expect("a functional")).expect("a map");
    assert_eq!(injective(&fixing, &cell).expect("the check runs"), Check::Holds);
    let tilting = ProjectiveMap::central(unit(0, 4)).expect("a map");
    assert_eq!(injective(&tilting, &cell).expect("the check runs"), Check::Holds);

    // LB4.2: on a region of full dimension a central projection is not injective, though no two vertices share a ray
    // and their images all differ. A per-vertex test cannot establish the property.
    let shifted = hull(&simplex().vertices().iter().map(|v| plus(v, &p(&[2, 1, 1, 1]))).collect::<Vec<_>>());
    let images: Vec<Point> =
        shifted.vertices().iter().map(|v| tilting.apply(v).expect("a point").expect("in the domain")).collect();
    assert!((0..5).all(|i| (i + 1..5).all(|j| images[i] != images[j])));
    let witness = not_injective(&tilting, &shifted);
    assert!(on_one_ray(&witness.x, &witness.y));

    assert_eq!(injective(&tilting, &simplex()), Err(PolytopeError::OutsideMapDomain));
    assert_eq!(injective(&drop_last(3), &simplex()), Err(PolytopeError::DimensionMismatch));
}

#[test]
fn embeddings_of_cell_unions() {
    let central = ProjectiveMap::central(unit(3, 4)).expect("a map");
    assert_eq!(embedding(&central, &[lower(), upper()]).expect("the check runs"), Check::Holds);

    // 2·lower has lower's image, and the two cells are disjoint: the union is no embedding.
    let doubled = hull(&lower().vertices().iter().map(|v| times(&q(2), v)).collect::<Vec<_>>());
    assert_eq!(central.image(&doubled).expect("an image"), lower());
    match embedding(&central, &[lower(), doubled.clone()]).expect("the check runs") {
        Check::Fails(witness) => {
            verified(&central, &[&lower(), &doubled], &witness);
            assert!(on_one_ray(&witness.x, &witness.y));
        }
        Check::Holds => panic!("two cells with one image embedded"),
    }

    // Stacked triangles: each is embedded, the pair is not.
    let floor = hull(&[p(&[0, 0, 0]), p(&[1, 0, 0]), p(&[0, 1, 0])]);
    let roof = hull(&[p(&[0, 0, 1]), p(&[1, 0, 1]), p(&[0, 1, 1])]);
    let drop = drop_last(3);
    assert_eq!(embedding(&drop, &[floor.clone()]).expect("the check runs"), Check::Holds);
    assert_eq!(embedding(&drop, &[roof.clone()]).expect("the check runs"), Check::Holds);
    match embedding(&drop, &[floor.clone(), roof.clone()]).expect("the check runs") {
        Check::Fails(witness) => verified(&drop, &[&floor, &roof], &witness),
        Check::Holds => panic!("stacked triangles embedded"),
    }

    // Two triangles that meet in a vertex, with images that meet in a whole edge.
    let left = hull(&[p(&[0, 0, 0]), p(&[1, 0, 0]), p(&[0, 1, 0])]);
    let right = hull(&[p(&[0, 0, 0]), p(&[1, 0, 1]), p(&[0, -1, 0])]);
    let squash = linear(&[&[1, 0, 0], &[0, 1, 0]]);
    match embedding(&squash, &[left.clone(), right.clone()]).expect("the check runs") {
        Check::Fails(witness) => verified(&squash, &[&left, &right], &witness),
        Check::Holds => panic!("triangles folded onto a common edge embedded"),
    }

    // LB4.2: two disjoint segments whose four vertex images all differ, while the images cross at an inner point.
    let below = hull(&[p(&[-1, 0, 0]), p(&[1, 0, 0])]);
    let above = hull(&[p(&[0, -1, 1]), p(&[0, 1, 1])]);
    let ends: Vec<Point> = below
        .vertices()
        .iter()
        .chain(above.vertices())
        .map(|v| drop.apply(v).expect("a point").expect("in the domain"))
        .collect();
    assert!((0..4).all(|i| (i + 1..4).all(|j| ends[i] != ends[j])));
    match embedding(&drop, &[below.clone(), above.clone()]).expect("the check runs") {
        Check::Fails(witness) => {
            verified(&drop, &[&below, &above], &witness);
            assert_eq!(witness.image, p(&[0, 0]));
        }
        Check::Holds => panic!("segments whose images cross embedded"),
    }

    // A cell on which the map is not injective.
    match embedding(&drop_last(4), &[flat(), simplex()]).expect("the check runs") {
        Check::Fails(witness) => verified(&drop_last(4), &[&simplex()], &witness),
        Check::Holds => panic!("a 4-simplex embedded in 3-space"),
    }
    assert_eq!(embedding(&drop, &[]), Err(PolytopeError::Empty));
}

#[test]
fn isometries_are_decided_over_the_region() {
    let euclid4 = Metric::euclidean(4);
    let euclid3 = Metric::euclidean(3);

    // Left multiplication by a unit quaternion of the 600-cell, then a shift.
    let (a, b, c, d) = (Q5::of(1, 1, 4), Q5::of(1, 0, 2), Q5::of(-1, 1, 4), Q5::zero());
    let rotation = vec![
        vec![a.clone(), b.neg(), c.neg(), d.neg()],
        vec![b.clone(), a.clone(), d.neg(), c.clone()],
        vec![c.clone(), d.clone(), a.clone(), b.neg()],
        vec![d, c.neg(), b, a],
    ];
    let turn = ProjectiveMap::affine(rotation, p(&[1, 2, 3, 4])).expect("a map");
    assert_eq!(isometry(&turn, &simplex(), &euclid4, &euclid4).expect("the check runs"), Check::Holds);
    let double = linear(&[&[2, 0, 0, 0], &[0, 2, 0, 0], &[0, 0, 2, 0], &[0, 0, 0, 2]]);
    match isometry(&double, &simplex(), &euclid4, &euclid4).expect("the check runs") {
        Check::Fails(change) => {
            verified_change(&double, &simplex(), &euclid4, &euclid4, &change);
            assert_eq!(change.after, change.before.mul(&q(4)));
        }
        Check::Holds => panic!("doubling kept distances"),
    }
    assert_eq!(
        isometry(&double, &hull(&[p(&[1, 2, 3, 4])]), &euclid4, &euclid4).expect("the check runs"),
        Check::Holds
    );

    // Dropping a coordinate keeps the distances of a region where it is constant, and no others.
    let drop = drop_last(4);
    assert_eq!(isometry(&drop, &flat(), &euclid4, &euclid3).expect("the check runs"), Check::Holds);
    let slanted = hull(&[origin(), p(&[1, 0, 0, 1]), p(&[0, 1, 0, 0])]);
    match isometry(&drop, &slanted, &euclid4, &euclid3).expect("the check runs") {
        Check::Fails(change) => verified_change(&drop, &slanted, &euclid4, &euclid3, &change),
        Check::Holds => panic!("a slanted triangle kept its distances"),
    }

    // A central projection is an isometry on the hyperplane it fixes, and not on a cell it tilts.
    let cell = cell600();
    let fixing = ProjectiveMap::central(cell.positive_functional().expect("a functional")).expect("a map");
    assert_eq!(isometry(&fixing, &cell, &euclid4, &euclid4).expect("the check runs"), Check::Holds);
    let tilting = ProjectiveMap::central(unit(0, 4)).expect("a map");
    match isometry(&tilting, &cell, &euclid4, &euclid4).expect("the check runs") {
        Check::Fails(change) => verified_change(&tilting, &cell, &euclid4, &euclid4, &change),
        Check::Holds => panic!("a tilting central projection kept distances"),
    }
    let central = ProjectiveMap::central(unit(3, 4)).expect("a map");
    assert_eq!(isometry(&central, &lower(), &euclid4, &euclid4).expect("the check runs"), Check::Holds);

    // LB4.2: x ↦ 2x/(1 + x) on [0, 1] keeps the distance of the two vertices and changes the halves.
    let segment = hull(&[p(&[0]), p(&[1])]);
    let bend = ProjectiveMap::new(vec![p(&[2])], p(&[0]), p(&[1]), q(1)).expect("a map");
    let euclid1 = Metric::euclidean(1);
    let ends: Vec<Point> =
        segment.vertices().iter().map(|v| bend.apply(v).expect("a point").expect("in the domain")).collect();
    let before = euclid1.squared_distance(&segment.vertices()[0], &segment.vertices()[1]).expect("a distance");
    assert_eq!(euclid1.squared_distance(&ends[0], &ends[1]).expect("a distance"), before);
    match isometry(&bend, &segment, &euclid1, &euclid1).expect("the check runs") {
        Check::Fails(change) => verified_change(&bend, &segment, &euclid1, &euclid1, &change),
        Check::Holds => panic!("a map that bends a segment kept its distances"),
    }

    // Another source metric.
    let stretched =
        Metric::gram(vec![p(&[1, 0, 0, 0]), p(&[0, 1, 0, 0]), p(&[0, 0, 1, 0]), p(&[0, 0, 0, 4])]).expect("a metric");
    let widen = linear(&[&[1, 0, 0, 0], &[0, 1, 0, 0], &[0, 0, 1, 0], &[0, 0, 0, 2]]);
    assert_eq!(isometry(&widen, &simplex(), &stretched, &euclid4).expect("the check runs"), Check::Holds);
    match isometry(&widen, &simplex(), &euclid4, &euclid4).expect("the check runs") {
        Check::Fails(change) => verified_change(&widen, &simplex(), &euclid4, &euclid4, &change),
        Check::Holds => panic!("stretching kept Euclidean distances"),
    }

    assert_eq!(isometry(&drop, &simplex(), &euclid4, &euclid4), Err(PolytopeError::DimensionMismatch));
    assert_eq!(isometry(&tilting, &simplex(), &euclid4, &euclid4), Err(PolytopeError::OutsideMapDomain));
}

#[test]
fn homomorphisms_of_cell_complexes() {
    let identity = linear(&[&[1, 0, 0, 0], &[0, 1, 0, 0], &[0, 0, 1, 0], &[0, 0, 0, 1]]);
    let complex = [lower(), upper()];
    assert_eq!(homomorphism(&identity, &complex, &complex).expect("the check runs"), Check::Holds);
    assert_eq!(
        homomorphism(&identity, &complex, &[upper()]).expect("the check runs"),
        Check::Fails(UnmatchedCell { cell: 0, image: lower() })
    );

    let drop = drop_last(4);
    let solid = hull(&[p(&[0, 0, 0]), p(&[1, 0, 0]), p(&[0, 1, 0]), p(&[0, 0, 1])]);
    let triangle = hull(&[p(&[0, 0, 0]), p(&[1, 0, 0]), p(&[0, 1, 0])]);
    assert_eq!(
        homomorphism(&drop, &[flat()], &[triangle.clone(), solid.clone()]).expect("the check runs"),
        Check::Holds
    );
    assert_eq!(
        homomorphism(&drop, &[flat()], &[triangle.clone()]).expect("the check runs"),
        Check::Fails(UnmatchedCell { cell: 0, image: solid.clone() })
    );
    // A cell may collapse onto a lower cell of the codomain.
    let collapsing = hull(&[origin(), unit(0, 4), unit(1, 4), unit(3, 4)]);
    assert_eq!(homomorphism(&drop, &[collapsing], &[solid.clone(), triangle]).expect("the check runs"), Check::Holds);

    assert_eq!(homomorphism(&drop, &[flat()], &[flat()]), Err(PolytopeError::DimensionMismatch));
    let central = ProjectiveMap::central(unit(3, 4)).expect("a map");
    assert_eq!(homomorphism(&central, &[simplex()], &[]), Err(PolytopeError::OutsideMapDomain));
    assert_eq!(homomorphism(&drop, &[], &[solid]), Err(PolytopeError::Empty));
}

#[test]
fn maps_and_metrics() {
    let central = ProjectiveMap::central(unit(0, 4)).expect("a map");
    assert_eq!(central.apply(&p(&[2, 4, 0, 0])).expect("a point"), Some(p(&[1, 2, 0, 0])));
    assert_eq!(central.apply(&p(&[0, 1, 0, 0])).expect("a point"), None);
    assert_eq!(central.apply(&p(&[-1, 0, 0, 0])).expect("a point"), None);
    assert!(!central.defined_on(&simplex()).expect("same dimension"));
    assert!(central.defined_on(&cell600()).expect("same dimension"));
    assert_eq!(central.image(&simplex()), Err(PolytopeError::OutsideMapDomain));
    assert_eq!(central.apply(&p(&[1, 2])), Err(PolytopeError::DimensionMismatch));
    assert_eq!(central.defined_on(&rectangle(0, 0, 1, 1)), Err(PolytopeError::DimensionMismatch));
    assert_eq!((central.source_dimension(), central.target_dimension()), (4, 4));

    // A constant negative denominator is negated with the numerator: the same map.
    let negative = ProjectiveMap::new(vec![p(&[1, 0])], p(&[0]), p(&[0, 0]), q(-2)).expect("a map");
    assert_eq!(negative.denominator_offset(), &q(2));
    assert_eq!(negative.apply(&p(&[4, 0])).expect("a point"), Some(p(&[-2])));

    assert_eq!(ProjectiveMap::new(vec![p(&[1, 0])], p(&[0]), p(&[0, 0]), q(0)), Err(PolytopeError::ZeroDenominator));
    assert_eq!(ProjectiveMap::central(p(&[0, 0])), Err(PolytopeError::ZeroDirection));
    assert_eq!(ProjectiveMap::central(vec![]), Err(PolytopeError::DimensionMismatch));
    assert_eq!(ProjectiveMap::affine(vec![p(&[1, 0]), p(&[1])], p(&[0, 0])), Err(PolytopeError::DimensionMismatch));
    assert_eq!(ProjectiveMap::affine(vec![p(&[1, 0])], p(&[0, 0])), Err(PolytopeError::DimensionMismatch));
    assert_eq!(ProjectiveMap::affine(vec![], vec![]), Err(PolytopeError::DimensionMismatch));

    let metric = Metric::gram(vec![p(&[2, 1]), p(&[1, 2])]).expect("a metric");
    assert_eq!(metric.squared_distance(&p(&[1, 1]), &p(&[0, 0])).expect("a distance"), q(6));
    assert_eq!(metric.squared_distance(&p(&[1]), &p(&[0, 0])), Err(PolytopeError::DimensionMismatch));
    assert_eq!(Metric::gram(vec![p(&[1, 2]), p(&[2, 1])]), Err(PolytopeError::NotAMetric), "indefinite");
    assert_eq!(Metric::gram(vec![p(&[1, 1]), p(&[0, 1])]), Err(PolytopeError::NotAMetric), "not symmetric");
    assert_eq!(Metric::gram(vec![p(&[0])]), Err(PolytopeError::NotAMetric), "semidefinite");
    assert_eq!(Metric::gram(vec![p(&[1, 0])]), Err(PolytopeError::DimensionMismatch));
    assert_eq!(Metric::gram(vec![]), Err(PolytopeError::DimensionMismatch));
}

#[test]
fn malformed_regions_are_errors() {
    assert_eq!(Polytope::hull(&[]), Err(PolytopeError::Empty));
    assert_eq!(Polytope::hull(&[p(&[1, 2]), p(&[1, 2, 3])]), Err(PolytopeError::DimensionMismatch));
    assert_eq!(Polytope::hull(&[vec![]]), Err(PolytopeError::DimensionMismatch));
    let s = simplex();
    assert_eq!(s.locate(&p(&[0, 0, 0])), Err(PolytopeError::DimensionMismatch));
    assert_eq!(s.line_meet(&origin(), &p(&[0, 0, 0])), Err(PolytopeError::DimensionMismatch));
    assert_eq!(s.line_meet(&origin(), &origin()), Err(PolytopeError::ZeroDirection));
    assert_eq!(s.ray_meet(&origin(), &origin()), Err(PolytopeError::ZeroDirection));
    assert_eq!(s.intersect(&rectangle(0, 0, 1, 1)), Err(PolytopeError::DimensionMismatch));

    let unbounded = [
        Constraint { a: p(&[-1, 0]), b: q(0) },
        Constraint { a: p(&[0, -1]), b: q(0) },
        Constraint { a: p(&[1, -1]), b: q(1) },
    ];
    assert!(matches!(Polytope::from_constraints(&unbounded, 2), Err(PolytopeError::Description(_))));
    assert_eq!(Polytope::from_constraints(&unbounded, 3), Err(PolytopeError::DimensionMismatch));
    assert_eq!(Polytope::from_constraints(&unbounded, 0), Err(PolytopeError::DimensionMismatch));
    assert_eq!(PolytopeError::OutsideMapDomain.to_string(), "the map is not defined on all of the region");
}

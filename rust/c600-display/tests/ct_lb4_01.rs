//! CT-LB4-01 (source, whole definition over this crate's drafted chart repertoire, RPI.34 [O]). At exact camera poses
//! the declared domain is decided by an exact region test and section membership on the posed vertices, boundary
//! cases included; a chart whose vertices all pass and whose domain fails at an interior point is refused; and
//! injectivity, embedding, isometry and homomorphism each require their own check. The repertoire is a draft: the
//! stereographic chart, whose injectivity and embedding reduce to a central projection and whose isometry and
//! homomorphism stay `not_established`, and the projective charts, for which each of the four has an exact check of
//! its own. Witnesses are read back against the region with `c600_jumble`'s exact polytope operations. Source and
//! synthetic fixture outcomes only: no device ran, and no agreement of a GPU path is claimed.

mod common;

use std::collections::HashSet;

use c600_display::chart::{
    self, Admission, CameraPose, ChartCheck, Method, Pose, Projective, Property, PropertyStatus, Region, Stereographic,
    Verdict, Witness, CENTRAL_PROJECTION,
};
use c600_display::codes;
use c600_display::exact::{self, Q5, V4};
use c600_display::plan::Chart;
use c600_jumble::polytope::{Location, Polytope};
use c600_jumble::q5::Q5 as Big;
use common::*;

/// A vector of small integers.
fn v(x: [i128; 4]) -> V4 {
    x.map(Q5::int)
}

/// The rational a/d.
fn q(a: i128, d: i128) -> Q5 {
    Q5::new(a, 0, d).expect("a rational")
}

/// A vector of rationals with one denominator.
fn vq(x: [i128; 4], d: i128) -> V4 {
    x.map(|a| q(a, d))
}

/// Points of small integers in a chart's image space.
fn points(rows: &[&[i128]]) -> Vec<Vec<Q5>> {
    rows.iter().map(|row| row.iter().map(|&a| Q5::int(a)).collect()).collect()
}

/// The rows of the Gram matrix s·I of size n.
fn gram(n: usize, s: Q5) -> Vec<Vec<Q5>> {
    (0..n).map(|i| (0..n).map(|j| if i == j { s } else { Q5::ZERO }).collect()).collect()
}

fn as_set(points: &[Vec<Q5>]) -> HashSet<Vec<Q5>> {
    points.iter().cloned().collect()
}

/// A value in `c600_jumble`'s exact arithmetic.
fn big(x: &Q5) -> Big {
    let (a, b, d) = x.parts();
    let small = |n: i128| i64::try_from(n).expect("a small fixture value");
    Big::of(small(a), small(b), small(d))
}

/// Where `x` lies in the convex hull of `points`, by `c600_jumble`'s exact polytope operations.
fn locate(points: &[V4], x: &V4) -> Location {
    let points: Vec<Vec<Big>> = points.iter().map(|p| p.iter().map(big).collect()).collect();
    let x: Vec<Big> = x.iter().map(big).collect();
    Polytope::hull(&points).expect("a hull").locate(&x).expect("one dimension")
}

fn inside(points: &[V4], x: &V4) -> bool {
    !matches!(locate(points, x), Location::Outside(_))
}

/// Whether `x` is on the closed singular ray {t·w : t ≥ 0} of the chart whose pole is +w.
fn on_singular_ray(x: &V4) -> bool {
    x[..3].iter().all(|c| c.is_zero()) && x[3].sign() != Some(-1)
}

/// Whether `x` and `y` are distinct points of one open ray from the origin, which the stereographic chart maps to one
/// point.
fn one_open_ray(x: &V4, y: &V4) -> bool {
    x != y && exact::same_ray(x, y) == Some(true)
}

/// Whether the unordered pair (x, y) is {a, b}.
fn same_pair((x, y): (&V4, &V4), a: &V4, b: &V4) -> bool {
    (x == a && y == b) || (x == b && y == a)
}

fn at_identity() -> CameraPose {
    CameraPose::Exact(Pose::identity())
}

/// The exact pose −I, an element of K⁺.
fn antipodal() -> CameraPose {
    let mut minus = [[Q5::ZERO; 4]; 4];
    for (i, row) in minus.iter_mut().enumerate() {
        row[i] = Q5::int(-1);
    }
    CameraPose::Exact(Pose::rotation(minus).expect("−I is in K⁺"))
}

/// A float camera matrix between exact poses.
fn between() -> CameraPose {
    let mut matrix = [[0.0; 4]; 4];
    for (i, row) in matrix.iter_mut().enumerate() {
        row[i] = 1.0;
    }
    CameraPose::Approximate { label: "between two exact poses".to_owned(), matrix }
}

/// The built-in stereographic chart's declaration: its pole at +w.
fn pole_w() -> Stereographic {
    Stereographic { pole: W }
}

/// The stereographic chart's own check of `property`.
fn stereo(property: &Property, region: &Region, pose: &CameraPose) -> Verdict {
    chart::check(&pole_w(), property, region, pose).expect("an exact pose")
}

/// The stereographic chart's vertex test of `property`.
fn vertices_only(property: &Property, region: &Region, pose: &CameraPose) -> Verdict {
    chart::vertex_test(&pole_w(), property, region, pose).expect("an exact pose")
}

/// A projective chart's own check of `property`.
fn projective(map: &Projective, property: &Property, region: &Region, pose: &CameraPose) -> Verdict {
    chart::check_projective(map, property, region, pose).expect("an exact pose")
}

fn established(verdict: &Verdict) -> &Method {
    match verdict {
        Verdict::Established { method, .. } => method,
        other => panic!("not established: {other:?}"),
    }
}

fn refuted(verdict: &Verdict) -> (&Method, &Witness) {
    match verdict {
        Verdict::Refuted { method, witness, .. } => (method, witness),
        other => panic!("not refuted: {other:?}"),
    }
}

fn open(verdict: &Verdict) -> &str {
    match verdict {
        Verdict::NotEstablished { reason, .. } => reason,
        other => panic!("not open: {other:?}"),
    }
}

/// The pair of a refutation by the reduction to a central projection.
fn ray_pair(verdict: &Verdict) -> (&V4, &V4) {
    match refuted(verdict) {
        (Method::Reduction(reduction), Witness::PointsOnOneRay(x, y)) if *reduction == CENTRAL_PROJECTION => (x, y),
        other => panic!("not refuted by the reduction: {other:?}"),
    }
}

/// The built-in chain claiming `properties`.
fn claiming(properties: &[&str]) -> Chart {
    let mut claimed = built_in(1);
    claimed.properties = properties.iter().map(|p| (*p).to_owned()).collect();
    claimed
}

/// A check result as admission consumes it, under the property name the plan claims.
fn record(claimed: &Chart, property: &Property, verdict: Verdict) -> ChartCheck {
    ChartCheck { chart: chart::identity(claimed), property: property.name().to_owned(), verdict }
}

/// A full-dimensional simplex in the domain, no two of whose vertices lie on one ray from the origin: (2, 0, 0, 0), the
/// three points one unit from it along the other axes, and (3, 1, 1, 1).
fn solid() -> Vec<V4> {
    vec![v([2, 0, 0, 0]), v([2, 1, 0, 0]), v([2, 0, 1, 0]), v([2, 0, 0, 1]), v([3, 1, 1, 1])]
}

/// A full-dimensional simplex that the singular ray touches at one boundary point, (0, 0, 0, 2), the midpoint of the
/// edge between its first two vertices.
fn touching() -> Vec<V4> {
    vec![v([1, 0, 0, 2]), v([-1, 0, 0, 2]), v([0, 0, 1, 2]), v([0, 1, 0, 2]), v([0, 1, 0, 3])]
}

/// A full-dimensional simplex whose interior the singular ray crosses while every vertex is off it: its centroid is
/// (0, 0, 0, 2).
fn piercing() -> Vec<V4> {
    vec![v([1, 0, 0, 3]), v([0, 1, 0, 3]), v([0, 0, 1, 3]), v([1, 1, 1, 1]), v([-2, -2, -2, 0])]
}

/// The affine chart that drops w.
fn drop_w() -> Projective {
    Projective::affine(vec![v([1, 0, 0, 0]), v([0, 1, 0, 0]), v([0, 0, 1, 0])], vec![Q5::ZERO; 3])
}

/// The affine chart x ↦ 2x.
fn doubling() -> Projective {
    let rows = gram(4, Q5::int(2)).into_iter().map(|row| V4::try_from(row).expect("four entries")).collect();
    Projective::affine(rows, vec![Q5::ZERO; 4])
}

/// The domain by the exact region test and section membership on the posed vertices: the singular ray through a
/// region's interior with every vertex off it, the ray touching a region at a boundary point, at a vertex and at a
/// single point, a region through the origin, lower-dimensional regions, and a float pose.
#[test]
fn ct_lb4_01_domain_by_the_exact_region_test_with_boundary_cases() {
    let identity = at_identity();
    let cannot = "cannot establish";

    // Every vertex passes and the domain fails at an interior point: the region test refutes with the ray's first
    // point in the region, and the chart that claims the domain is refused. At the antipodal pose the region misses
    // the ray.
    let piercing = piercing();
    assert_eq!(locate(&piercing, &v([0, 0, 0, 2])), Location::RelativeInterior);
    let region = Region::hull("piercing", piercing.clone());
    assert!(open(&vertices_only(&Property::Domain, &region, &identity)).contains(cannot));
    let domain = stereo(&Property::Domain, &region, &identity);
    let first = vq([0, 0, 0, 2], 3);
    assert_eq!(refuted(&domain), (&Method::RegionTest, &Witness::PointOnSingularRay(first)));
    assert!(on_singular_ray(&first) && inside(&piercing, &first));
    let claimed = claiming(&["domain"]);
    let (admission, statuses) = chart::admit_chart(&claimed, &[record(&claimed, &Property::Domain, domain)]);
    let Admission::Refused(error) = admission else { panic!("{admission:?}") };
    assert_eq!((error.code, error.context["property"].as_str()), (codes::CHART_CHECK_FAILED, "domain"));
    assert_eq!(statuses, vec![("domain".to_owned(), PropertyStatus::NotEstablished)]);
    assert_eq!(established(&stereo(&Property::Domain, &region, &antipodal())), &Method::RegionTest);

    // The ray touches a full-dimensional region at a boundary point between two vertices, none of them on the ray.
    let touching = touching();
    assert!(matches!(locate(&touching, &v([0, 0, 0, 2])), Location::Boundary(_)));
    let region = Region::hull("touching", touching.clone());
    assert!(open(&vertices_only(&Property::Domain, &region, &identity)).contains(cannot));
    let witness = Witness::PointOnSingularRay(v([0, 0, 0, 2]));
    assert_eq!(refuted(&stereo(&Property::Domain, &region, &identity)), (&Method::RegionTest, &witness));
    assert_eq!(established(&stereo(&Property::Domain, &region, &antipodal())), &Method::RegionTest);

    // Lower-dimensional regions: a face touching the ray at the same boundary point, a triangle with a vertex on it, a
    // segment whose relative interior holds the origin, the ray's own end, and single points on the ray, off it and on
    // the opposite ray.
    let face = Region::hull("face", touching[..3].to_vec());
    assert_eq!(refuted(&stereo(&Property::Domain, &face, &identity)), (&Method::RegionTest, &witness));
    let cornered = Region::hull("cornered", vec![v([0, 0, 0, 1]), v([1, 0, 0, 1]), v([0, 1, 0, 1])]);
    let at_vertex = Witness::PointOnSingularRay(v([0, 0, 0, 1]));
    assert_eq!(refuted(&stereo(&Property::Domain, &cornered, &identity)), (&Method::RegionTest, &at_vertex));
    assert_eq!(
        refuted(&vertices_only(&Property::Domain, &cornered, &identity)),
        (&Method::VertexTest, &Witness::VertexOnSingularRay(0))
    );
    let origin = [Q5::ZERO; 4];
    let through = vec![v([-1, 0, 0, 0]), v([1, 0, 0, 0])];
    assert_eq!(locate(&through, &origin), Location::RelativeInterior);
    let through = Region::hull("through the origin", through);
    assert!(open(&vertices_only(&Property::Domain, &through, &identity)).contains(cannot));
    let at_origin = Witness::PointOnSingularRay(origin);
    assert_eq!(refuted(&stereo(&Property::Domain, &through, &identity)), (&Method::RegionTest, &at_origin));
    for (point, on) in [([0, 0, 0, 5], true), ([1, 0, 0, 5], false), ([0, 0, 0, -5], false)] {
        let verdict = stereo(&Property::Domain, &Region::hull("point", vec![v(point)]), &identity);
        if on {
            assert_eq!(refuted(&verdict), (&Method::RegionTest, &Witness::PointOnSingularRay(v(point))));
        } else {
            assert_eq!(established(&verdict), &Method::RegionTest, "{point:?}");
        }
    }

    // Section membership on the posed vertices, the slab's boundary included. The vertex test refutes with the same
    // vertex and establishes nothing; at the antipodal pose the posed vertices decide.
    let region = Region::hull("touching", touching);
    let slab = |low: Q5, high: Q5| Property::Section { normal: W, low, high };
    let wide = slab(Q5::int(2), Q5::int(3));
    assert_eq!(established(&stereo(&wide, &region, &identity)), &Method::PosedVertices);
    assert!(open(&vertices_only(&wide, &region, &identity)).contains(cannot));
    let thin = slab(Q5::int(2), q(5, 2));
    assert_eq!(refuted(&stereo(&thin, &region, &identity)), (&Method::PosedVertices, &Witness::VertexOutside(4)));
    assert_eq!(refuted(&vertices_only(&thin, &region, &identity)), (&Method::VertexTest, &Witness::VertexOutside(4)));
    let below = slab(Q5::int(-3), Q5::int(-2));
    assert_eq!(established(&stereo(&below, &region, &antipodal())), &Method::PosedVertices);
    assert_eq!(refuted(&stereo(&wide, &region, &antipodal())), (&Method::PosedVertices, &Witness::VertexOutside(0)));

    // A float camera matrix is no exact pose: every check refuses it, and none rounds it into one.
    let float = between();
    let refusals = [
        chart::check(&pole_w(), &Property::Domain, &region, &float),
        chart::vertex_test(&pole_w(), &Property::Domain, &region, &float),
        chart::check_projective(&Projective::central(E1), &Property::Domain, &region, &float),
    ];
    for refusal in refusals {
        assert_eq!(refusal.expect_err("a float pose").code, codes::APPROXIMATE_PROPERTY_INPUT);
    }
}

/// Injectivity by its own check: refuted on a full-dimensional region whose vertices all pass, with two distinct
/// points of one image, established where the reduction applies, and decided or left open at the boundary cases.
#[test]
fn ct_lb4_01_injectivity_by_its_own_check() {
    let identity = at_identity();

    // A full-dimensional region in the domain: every vertex passes the vertex test, which establishes nothing, and the
    // reduction's check finds the radial chord through the centroid, an interior point, whose ends have its image.
    // The domain is established and injectivity refuted, each by its own check, and the chart that claims both is
    // refused.
    let solid = solid();
    let region = Region::hull("solid", solid.clone());
    let domain = stereo(&Property::Domain, &region, &identity);
    assert_eq!(established(&domain), &Method::RegionTest);
    assert!(open(&vertices_only(&Property::Injective, &region, &identity)).contains("cannot establish"));
    let injective = stereo(&Property::Injective, &region, &identity);
    let (x, y) = ray_pair(&injective);
    assert!(same_pair((x, y), &vq([33, 6, 6, 6], 16), &vq([22, 4, 4, 4], 9)), "{injective:?}");
    assert!(one_open_ray(x, y) && !on_singular_ray(x));
    assert!(inside(&solid, x) && inside(&solid, y));
    let centroid = vq([11, 2, 2, 2], 5);
    assert_eq!(locate(&solid, &centroid), Location::RelativeInterior);
    assert!(one_open_ray(x, &centroid) && one_open_ray(&centroid, y));
    let claimed = claiming(&["domain", "injective"]);
    let checks =
        [record(&claimed, &Property::Domain, domain), record(&claimed, &Property::Injective, injective.clone())];
    let (admission, statuses) = chart::admit_chart(&claimed, &checks);
    let Admission::Refused(error) = admission else { panic!("{admission:?}") };
    assert_eq!(error.context["property"], "injective");
    assert!(statuses.iter().all(|(_, status)| *status == PropertyStatus::NotEstablished), "{statuses:?}");

    // Regions in the hyperplane x = 2, which misses the origin: the reduction establishes injectivity, down to a point.
    for count in 1..=4 {
        let verdict = stereo(&Property::Injective, &Region::hull("flat", solid[..count].to_vec()), &identity);
        assert_eq!(established(&verdict), &Method::Reduction(CENTRAL_PROJECTION), "{count} vertices");
    }

    // A radial segment: the reduction and the vertex test both refute, with its two vertices.
    let radial = Region::hull("radial", vec![v([1, 1, 0, 0]), v([2, 2, 0, 0])]);
    let verdict = stereo(&Property::Injective, &radial, &identity);
    assert!(same_pair(ray_pair(&verdict), &v([1, 1, 0, 0]), &v([2, 2, 0, 0])), "{verdict:?}");
    assert_eq!(
        refuted(&vertices_only(&Property::Injective, &radial, &identity)),
        (&Method::VertexTest, &Witness::SharedRay(0, 1))
    );

    // The ray touching a full-dimensional region at a boundary point: the domain fails there, and injectivity fails at
    // a pair in the domain. On the face through that point the chart is injective on the part in its domain.
    let touching = touching();
    let verdict = stereo(&Property::Injective, &Region::hull("touching", touching.clone()), &identity);
    let (x, y) = ray_pair(&verdict);
    assert!(same_pair((x, y), &vq([0, 4, 2, 22], 11), &vq([0, 4, 2, 22], 9)), "{verdict:?}");
    assert!(inside(&touching, x) && inside(&touching, y) && !on_singular_ray(x));
    let face = Region::hull("face", touching[..3].to_vec());
    assert_eq!(established(&stereo(&Property::Injective, &face, &identity)), &Method::Reduction(CENTRAL_PROJECTION));

    // A segment through the origin has no positive functional, and a segment on the singular ray gives a pair outside
    // the domain: neither decides injectivity, and neither is read as false.
    let through = Region::hull("through the origin", vec![v([-1, 0, 0, 0]), v([1, 0, 0, 0])]);
    assert!(open(&stereo(&Property::Injective, &through, &identity)).contains("origin"));
    let on_ray = Region::hull("on the ray", vec![v([0, 0, 0, 1]), v([0, 0, 0, 3])]);
    assert!(open(&stereo(&Property::Injective, &on_ray, &identity)).contains("singular ray"));
}

/// Embedding by its own check, injectivity on the union of the cells: refuted where every cell is injective, with one
/// point of each cell on one ray, and established for disjoint images and for cells meeting in a common face.
/// Isometry and homomorphism have no exact checker for the stereographic chart and stay open.
#[test]
fn ct_lb4_01_stereographic_embedding_by_its_own_check() {
    let identity = at_identity();
    let triangle = vec![v([2, 0, 0, 0]), v([2, 2, 0, 0]), v([2, 0, 2, 0])];
    let segment = vec![v([4, 2, 0, 0]), v([4, 0, 2, 0])];
    let cells = vec![vec![0, 1, 2], vec![3, 4]];
    let complex = Region::complex("overlapping", [triangle.clone(), segment.clone()].concat(), cells.clone());
    assert_eq!(established(&stereo(&Property::Domain, &complex, &identity)), &Method::RegionTest);
    let injective = stereo(&Property::Injective, &complex, &identity);
    assert_eq!(established(&injective), &Method::Reduction(CENTRAL_PROJECTION));
    assert!(open(&vertices_only(&Property::Embedding, &complex, &identity)).contains("cannot establish"));
    let embedding = stereo(&Property::Embedding, &complex, &identity);
    let (x, y) = ray_pair(&embedding);
    let ends = [(v([2, 1, 0, 0]), v([4, 2, 0, 0])), (v([2, 0, 1, 0]), v([4, 0, 2, 0]))];
    assert!(ends.iter().any(|(a, b)| same_pair((x, y), a, b)), "{embedding:?}");
    assert!(one_open_ray(x, y));
    assert!((inside(&triangle, x) && inside(&segment, y)) || (inside(&triangle, y) && inside(&segment, x)));

    let isometry = Property::Isometry { source: gram(4, Q5::ONE), target: gram(4, Q5::ONE) };
    let homomorphism = Property::Homomorphism { codomain: vec![points(&[&[0, 0, 0, 0]])] };
    for property in [&isometry, &homomorphism] {
        assert!(open(&stereo(property, &complex, &identity)).contains("not a projective map"));
        assert!(open(&vertices_only(property, &complex, &identity)).contains("not a projective map"));
    }

    // The chart that claims both is refused on the embedding's own failure; claiming injectivity alone, it is admitted.
    let both = claiming(&["injective", "embedding"]);
    let checks =
        [record(&both, &Property::Injective, injective), record(&both, &Property::Embedding, embedding.clone())];
    let (admission, statuses) = chart::admit_chart(&both, &checks);
    let Admission::Refused(error) = admission else { panic!("{admission:?}") };
    assert_eq!(error.context["property"], "embedding");
    let withheld = ["injective", "embedding"].map(|p| (p.to_owned(), PropertyStatus::NotEstablished));
    assert_eq!(statuses, withheld.to_vec());
    assert_eq!(chart::admit_chart(&claiming(&["injective"]), &checks).0, Admission::Admitted);

    // Cells with disjoint images, and two triangles meeting in their common edge: embeddings.
    let disjoint = [triangle, vec![v([4, 0, 0, 1]), v([4, 0, 0, 2])]].concat();
    let verdict = stereo(&Property::Embedding, &Region::complex("disjoint", disjoint, cells), &identity);
    assert_eq!(established(&verdict), &Method::Reduction(CENTRAL_PROJECTION));
    let square = vec![v([2, 0, 0, 0]), v([2, 1, 0, 0]), v([2, 0, 1, 0]), v([2, 1, 1, 0])];
    let glued = Region::complex("glued", square, vec![vec![0, 1, 2], vec![1, 2, 3]]);
    assert_eq!(established(&stereo(&Property::Embedding, &glued, &identity)), &Method::Reduction(CENTRAL_PROJECTION));

    // Two points whose hull contains the origin: each cell is injective, and no one functional is positive on both, so
    // the embedding stays open rather than false.
    let apart = Region::complex("apart", vec![v([1, 0, 0, 0]), v([-1, 0, 0, 0])], vec![vec![0], vec![1]]);
    assert_eq!(established(&stereo(&Property::Injective, &apart, &identity)), &Method::Reduction(CENTRAL_PROJECTION));
    assert!(open(&stereo(&Property::Embedding, &apart, &identity)).contains("origin"));
}

/// The projective charts: injectivity, embedding, isometry and homomorphism each decided by its own check over the
/// whole region, each refuted on a region where another of them is established, with the domain and a section on the
/// posed vertices.
#[test]
fn ct_lb4_01_projective_charts_each_property_by_its_own_check() {
    let identity = at_identity();

    // Dropping w from a triangle at w = 0 and a segment at w = 1: injective on each cell and an isometry of each, while
    // the images of the two cells meet and the embedding is refuted. The vertices' images are pairwise distinct, so no
    // vertex test could find that.
    let p = drop_w();
    let vertices = vec![v([0, 0, 0, 0]), v([2, 0, 0, 0]), v([0, 2, 0, 0]), v([1, -1, 0, 1]), v([1, 3, 0, 1])];
    let images: HashSet<Vec<Q5>> = vertices.iter().map(|x| x[..3].to_vec()).collect();
    assert_eq!(images.len(), vertices.len());
    let (sigma, tau) = (vertices[..3].to_vec(), vertices[3..].to_vec());
    let region = Region::complex("two cells", vertices, vec![vec![0, 1, 2], vec![3, 4]]);
    let check = |property: &Property| projective(&p, property, &region, &identity);
    assert_eq!(established(&check(&Property::Domain)), &Method::PosedVertices);
    assert_eq!(established(&check(&Property::Injective)), &Method::PolytopeCheck);
    let euclidean = Property::Isometry { source: gram(4, Q5::ONE), target: gram(3, Q5::ONE) };
    assert_eq!(established(&check(&euclidean)), &Method::PolytopeCheck);
    let embedding = check(&Property::Embedding);
    let (method, Witness::SameImage { x, y, image }) = refuted(&embedding) else { panic!("{embedding:?}") };
    assert_eq!(method, &Method::PolytopeCheck);
    assert!(x != y && image[..] == x[..3] && image[..] == y[..3], "{embedding:?}");
    assert!([points(&[&[1, 0, 0]]), points(&[&[1, 1, 0]])].iter().any(|one| one[0] == *image), "{embedding:?}");
    assert!((inside(&sigma, x) && inside(&tau, y)) || (inside(&sigma, y) && inside(&tau, x)));
    let slab = Property::Section { normal: W, low: Q5::ZERO, high: Q5::ONE };
    assert_eq!(established(&check(&slab)), &Method::PosedVertices);

    // Homomorphism: each cell onto a cell of the codomain, compared by vertex set.
    let sigma_image = points(&[&[0, 0, 0], &[2, 0, 0], &[0, 2, 0]]);
    let tau_image = points(&[&[1, -1, 0], &[1, 3, 0]]);
    let onto = Property::Homomorphism { codomain: vec![tau_image.clone(), sigma_image.clone()] };
    assert_eq!(established(&check(&onto)), &Method::PolytopeCheck);
    let short = check(&Property::Homomorphism { codomain: vec![sigma_image] });
    let (method, Witness::UnmatchedCell { cell, image }) = refuted(&short) else { panic!("{short:?}") };
    assert_eq!((method, *cell), (&Method::PolytopeCheck, 1));
    assert_eq!(as_set(image), as_set(&tau_image));

    // A full-dimensional region: dropping w maps it onto a cell of the codomain, a homomorphism, while its injectivity
    // is refuted with the chord along w through the centroid. Doubling is injective there.
    let solid = Region::hull("solid", solid());
    let onto = Property::Homomorphism { codomain: vec![points(&[&[2, 0, 0], &[2, 1, 0], &[2, 0, 1], &[3, 1, 1]])] };
    assert_eq!(established(&projective(&p, &onto, &solid, &identity)), &Method::PolytopeCheck);
    let verdict = projective(&p, &Property::Injective, &solid, &identity);
    let (method, Witness::SameImage { x, y, image }) = refuted(&verdict) else { panic!("{verdict:?}") };
    assert_eq!(method, &Method::PolytopeCheck);
    assert!(same_pair((x, y), &vq([11, 2, 2, 1], 5), &vq([11, 2, 2, 3], 5)), "{verdict:?}");
    assert_eq!(image, &vec![q(11, 5), q(2, 5), q(2, 5)]);
    let d = doubling();
    assert_eq!(established(&projective(&d, &Property::Injective, &solid, &identity)), &Method::PolytopeCheck);

    // Doubling a triangle and its edge: an embedding, while the Euclidean isometry is refuted, a quarter of the target
    // metric is established, and the homomorphism holds onto the doubled cells only, at each pose onto its own.
    let corner = Region::complex(
        "corner",
        vec![v([0, 0, 0, 0]), v([1, 0, 0, 0]), v([0, 1, 0, 0])],
        vec![vec![0, 1, 2], vec![1, 2]],
    );
    let check = |property: &Property, pose: &CameraPose| projective(&d, property, &corner, pose);
    assert_eq!(established(&check(&Property::Injective, &identity)), &Method::PolytopeCheck);
    assert_eq!(established(&check(&Property::Embedding, &identity)), &Method::PolytopeCheck);
    let euclidean = Property::Isometry { source: gram(4, Q5::ONE), target: gram(4, Q5::ONE) };
    let verdict = check(&euclidean, &identity);
    let (method, Witness::DistanceChange { before, after, .. }) = refuted(&verdict) else { panic!("{verdict:?}") };
    assert_eq!(method, &Method::PolytopeCheck);
    assert!(!before.is_zero() && Q5::int(4).mul(*before) == Some(*after), "{verdict:?}");
    let rescaled = Property::Isometry { source: gram(4, Q5::ONE), target: gram(4, q(1, 4)) };
    assert_eq!(established(&check(&rescaled, &identity)), &Method::PolytopeCheck);
    let cells =
        |s: i128| vec![points(&[&[0, 0, 0, 0], &[s, 0, 0, 0], &[0, s, 0, 0]]), points(&[&[s, 0, 0, 0], &[0, s, 0, 0]])];
    let onto = |s: i128| Property::Homomorphism { codomain: cells(s) };
    let verdict = check(&onto(1), &identity);
    let (method, Witness::UnmatchedCell { cell, image }) = refuted(&verdict) else { panic!("{verdict:?}") };
    assert_eq!((method, *cell), (&Method::PolytopeCheck, 0));
    assert_eq!(as_set(image), as_set(&cells(2)[0]));
    assert_eq!(established(&check(&onto(2), &identity)), &Method::PolytopeCheck);
    assert!(matches!(check(&onto(2), &antipodal()), Verdict::Refuted { .. }));
    assert_eq!(established(&check(&onto(-2), &antipodal())), &Method::PolytopeCheck);

    // A central projection: not affine on a segment along which its denominator changes, so no isometry, though
    // injective there; on a segment where the denominator turns negative, the domain fails at the posed vertex and
    // the other checks decide nothing.
    let central = Projective::central(E1);
    let slanted = Region::hull("slanted", vec![v([1, 0, 0, 0]), v([2, 0, 0, 1])]);
    assert_eq!(established(&projective(&central, &Property::Domain, &slanted, &identity)), &Method::PosedVertices);
    assert_eq!(established(&projective(&central, &Property::Injective, &slanted, &identity)), &Method::PolytopeCheck);
    let verdict = projective(&central, &euclidean, &slanted, &identity);
    assert!(matches!(refuted(&verdict), (Method::PolytopeCheck, Witness::DistanceChange { .. })), "{verdict:?}");
    let crossing = Region::hull("crossing", vec![v([1, 0, 0, 0]), v([-1, 0, 0, 1])]);
    assert_eq!(
        refuted(&projective(&central, &Property::Domain, &crossing, &identity)),
        (&Method::PosedVertices, &Witness::VertexOutsideDomain(1))
    );
    assert!(open(&projective(&central, &Property::Injective, &crossing, &identity)).contains("did not run"));

    // A malformed declaration decides nothing.
    let mut short = drop_w();
    short.offset.pop();
    assert!(open(&projective(&short, &Property::Injective, &region, &identity)).contains("did not run"));
    let undefined = Projective { denominator_offset: Q5::ZERO, ..drop_w() };
    assert!(open(&projective(&undefined, &Property::Domain, &region, &identity)).contains("did not run"));
}

/// Admission (RPI.48): a claimed property is admitted only when its own established checks cover every region and
/// exact pose at which the chart was checked. Another property's established check never stands in for it, also under
/// a claimed name that carries the property's parameters after a colon (a draft).
#[test]
fn ct_lb4_01_admission_needs_each_property_own_check_at_every_surveyed_pose() {
    let solid = solid();
    let flat = Region::hull("flat", solid[..4].to_vec());
    let edge = Region::hull("edge", solid[..2].to_vec());
    let (identity, antipodal) = (at_identity(), antipodal());
    let claimed = claiming(&["injective"]);
    let check = |property: &Property, region: &Region, pose: &CameraPose| {
        record(&claimed, property, stereo(property, region, pose))
    };

    // The domain alone, though established, does not establish injectivity.
    let mut checks = vec![check(&Property::Domain, &flat, &identity), check(&Property::Domain, &flat, &antipodal)];
    let (admission, statuses) = chart::admit_chart(&claimed, &checks);
    assert!(matches!(admission, Admission::NotEstablished(_)), "{admission:?}");
    assert_eq!(statuses, vec![("injective".to_owned(), PropertyStatus::NotEstablished)]);

    // Injectivity at the identity only, while the chart was checked at the antipodal pose too.
    checks.push(check(&Property::Injective, &flat, &identity));
    assert!(matches!(chart::admit_chart(&claimed, &checks).0, Admission::NotEstablished(_)));
    checks.push(check(&Property::Injective, &flat, &antipodal));
    let (admission, statuses) = chart::admit_chart(&claimed, &checks);
    assert_eq!(admission, Admission::Admitted);
    let PropertyStatus::Established(scopes) = &statuses[0].1 else { panic!("{statuses:?}") };
    assert_eq!(scopes.len(), 2);
    assert!(scopes.iter().all(|scope| scope.property == "injective" && scope.regions == ["flat"]));

    // Another region checked: not admitted until injectivity is checked there too.
    checks.push(check(&Property::Domain, &edge, &identity));
    assert!(matches!(chart::admit_chart(&claimed, &checks).0, Admission::NotEstablished(_)));
    checks.push(check(&Property::Injective, &edge, &identity));
    assert_eq!(chart::admit_chart(&claimed, &checks).0, Admission::Admitted);

    // A record that names the embedding but carries injectivity's verdict is not the embedding's own check.
    let embedding = claiming(&["embedding"]);
    let borrowed = ChartCheck {
        chart: chart::identity(&embedding),
        property: "embedding".to_owned(),
        verdict: stereo(&Property::Injective, &flat, &identity),
    };
    let (admission, statuses) = chart::admit_chart(&embedding, &[borrowed]);
    assert!(matches!(admission, Admission::NotEstablished(_)), "{admission:?}");
    assert_eq!(statuses, vec![("embedding".to_owned(), PropertyStatus::NotEstablished)]);

    // Two claimed sections, told apart by their slabs after the name and a colon: each is admitted on section checks
    // recorded under its own claimed name, and a record under that name with the domain's verdict is not its own.
    let sections = claiming(&["section:wide", "section:wider"]);
    let named = |name: &str, property: &Property| ChartCheck {
        chart: chart::identity(&sections),
        property: name.to_owned(),
        verdict: stereo(property, &flat, &identity),
    };
    let wide = Property::Section { normal: W, low: Q5::int(-50), high: Q5::int(50) };
    let wider = Property::Section { normal: W, low: Q5::int(-100), high: Q5::int(100) };
    let mut records = vec![named("section:wide", &wide), named("section:wider", &Property::Domain)];
    let (admission, statuses) = chart::admit_chart(&sections, &records);
    assert!(matches!(admission, Admission::NotEstablished(_)), "{admission:?}");
    assert!(matches!(&statuses[0], (name, PropertyStatus::Established(_)) if name == "section:wide"), "{statuses:?}");
    assert_eq!(statuses[1], ("section:wider".to_owned(), PropertyStatus::NotEstablished));
    records.push(named("section:wider", &wider));
    assert_eq!(chart::admit_chart(&sections, &records).0, Admission::Admitted);
}

//! RPI.72's raster bound U, as the formula over evaluated screen positions that every backend must agree with.
//!
//! In a frame with a user stage in its chain, the adapter evaluates the vertex stage into a charged buffer, expands
//! each line and point into counted triangles, and computes U on the device before any fragment runs (RPI.72 steps 1
//! to 3); that is a backend's work (C9, CT-RM-10). This module is the formula of step 3 on the CPU, over triangles the
//! adapter has already placed on the surface and classified against the clip volume, so that RM.40's vectors run
//! headless and a backend's U can be compared with it on the same positions.
//!
//! U is the sum over triangles of the triangle's bounding rectangle, rounded outward to whole pixels, clipped to the
//! surface and widened to whole 2×2 quads (helper lanes), times the shading samples per pixel, summed over every pass
//! including the ID pass. A triangle wholly outside the clip volume counts zero; one that crosses the near plane
//! counts the whole surface; one with an undefined vertex enters neither the sum nor the draw (RPI.37). A frame stops
//! when U exceeds the surface pixels times `overdraw`; at equality it runs (RPI.72 step 4,
//! [`crate::stage::Quarantine`]).
//!
//! Drafts: surface coordinates are pixels from one corner, with the surface covering `[0, width] × [0, height]`. A
//! quad is widened after clipping and not clipped again, so on a surface of odd size it may count the helper lanes
//! past the last row or column. A rectangle with no width or height counts zero, since it bounds a triangle of no
//! area, which covers no pixel. A non-finite coordinate is an undefined vertex. A line's quad runs along the segment
//! with no caps. Not here: points, and lines of zero length, whose expansion RPI.72 does not pin; the projection to
//! the surface and the clip-volume classification, whose conventions are the adapter's (C9).

/// One view's surface, in pixels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Surface {
    pub width: u32,
    pub height: u32,
}

impl Surface {
    pub fn pixels(self) -> u64 {
        u64::from(self.width) * u64::from(self.height)
    }
}

/// A raster triangle of a frame, as the adapter classifies it from the evaluated positions.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Triangle {
    /// Not wholly outside the clip volume and not crossing the near plane: its three vertices in surface pixels.
    Placed([[f32; 2]; 3]),
    /// Wholly outside the clip volume: it counts zero and draws nothing.
    Outside,
    /// Crosses the near plane: it counts the whole surface.
    CrossesNear,
    /// A vertex is undefined (a coverage code): it enters neither the sum nor the draw (RPI.37).
    Undefined,
}

/// The extent one axis of a bounding rectangle counts: rounded outward, clipped to `0..=size`, then widened to whole
/// quads. An empty extent counts zero.
fn extent(low: f32, high: f32, size: u32) -> u64 {
    let size = i64::from(size);
    let low = (low.floor() as i64).clamp(0, size);
    let high = (high.ceil() as i64).clamp(0, size);
    if low >= high {
        return 0;
    }
    let (low, high) = (low & !1, (high + 1) & !1);
    (high - low) as u64
}

/// The pixels one triangle counts toward U on `surface`, for one shading sample of one pass.
pub fn count(surface: Surface, triangle: &Triangle) -> u64 {
    let corners = match triangle {
        Triangle::Placed(corners) => corners,
        Triangle::CrossesNear => return surface.pixels(),
        Triangle::Outside | Triangle::Undefined => return 0,
    };
    if corners.iter().flatten().any(|c| !c.is_finite()) {
        return 0;
    }
    let axis = |i: usize, size: u32| {
        let low = corners.iter().map(|c| c[i]).fold(f32::INFINITY, f32::min);
        let high = corners.iter().map(|c| c[i]).fold(f32::NEG_INFINITY, f32::max);
        extent(low, high, size)
    };
    axis(0, surface.width) * axis(1, surface.height)
}

/// RPI.72 step 2: the line from `ends[0]` to `ends[1]` drawn `width` pixels wide, as the two counted triangles of its
/// screen-space quad. An undefined end makes both triangles undefined. A line of zero length or a width that is not
/// positive and finite has no expansion here (`None`): RPI.72 does not pin one.
pub fn line(ends: [[f32; 2]; 2], width: f32) -> Option<[Triangle; 2]> {
    if ends.iter().flatten().any(|c| !c.is_finite()) {
        return Some([Triangle::Undefined; 2]);
    }
    let [a, b] = ends.map(|[x, y]| [f64::from(x), f64::from(y)]);
    let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
    let length = dx.hypot(dy);
    if length == 0.0 || !(width > 0.0 && width.is_finite()) {
        return None;
    }
    let half = f64::from(width) / 2.0;
    let (ox, oy) = (-dy / length * half, dx / length * half);
    let corner = |p: [f64; 2], sign: f64| [(p[0] + sign * ox) as f32, (p[1] + sign * oy) as f32];
    let (p0, p1, p2, p3) = (corner(a, 1.0), corner(b, 1.0), corner(b, -1.0), corner(a, -1.0));
    Some([Triangle::Placed([p0, p1, p2]), Triangle::Placed([p0, p2, p3])])
}

/// One pass of a frame, the ID pass included: its triangles and its shading samples per pixel.
#[derive(Clone, Copy, Debug)]
pub struct Pass<'a> {
    pub triangles: &'a [Triangle],
    pub samples: u32,
}

/// U for one frame: over every pass, the pass's samples times the pixels its triangles count.
pub fn bound(surface: Surface, passes: &[Pass]) -> u128 {
    passes
        .iter()
        .map(|pass| {
            let pixels: u128 = pass.triangles.iter().map(|t| u128::from(count(surface, t))).sum();
            pixels * u128::from(pass.samples)
        })
        .sum()
}

/// A frame's allowance: surface pixels times the chain's `overdraw`.
pub fn allowance(surface: Surface, overdraw: u64) -> u128 {
    u128::from(surface.pixels()) * u128::from(overdraw)
}

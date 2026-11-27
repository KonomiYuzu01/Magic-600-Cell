//! ADR-D02 B22 item 1 (ADR-D02.9): the chart assignment of the Global chart presets. Per revision, on the CPU in the
//! host, each posed sticker's chart is fixed once: the cell nearest the posed centre of the sticker's own cell, ties
//! to the lowest cell id. The host hands the core one cell id per slot (`sa2_charts_set`) at each adoption; a view
//! with a Hopf layout (the ring sheet or the Hopf rings, only ever Global) draws each sticker in that cell's chart.
//! Local never reads it.

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use c600_display::produce::GeometryInput;
use c600_kernel::json::{self, Value};

/// Stickers per cell: slot = cell * 433 + local.
const PER_CELL: usize = 433;
/// Two cells whose cosines with the posed centre differ by at most this much are a tie (binary32 poses round each
/// entry by about 6e-8, so an exact tie can differ by about 1e-7); the lower cell id wins.
pub const TIE: f64 = 1e-6;

/// The model's 600 unit cell centres and the piece of each slot, read once per host.
#[derive(Debug)]
pub struct ChartModel {
    pub centres: Vec<[f64; 4]>,
    pub slot_piece: Vec<u32>,
}

impl ChartModel {
    /// The centres as the core places them (`Scene::cell_centre`): each cell's frame times the manifest's
    /// `mesh.normal`, normalized; the slot pieces from the checked geometry.
    pub fn load(geometry: &GeometryInput, assets: &Path) -> Result<ChartModel, String> {
        let bytes = fs::read(assets.join("manifest.json")).map_err(|e| format!("manifest.json: {e}"))?;
        let manifest = json::parse(&bytes).map_err(|_| "manifest.json is not JSON".to_owned())?;
        let normal: Vec<f64> = manifest
            .get("mesh")
            .and_then(|mesh| mesh.get("normal"))
            .and_then(Value::as_array)
            .ok_or_else(|| "manifest.json names no mesh.normal".to_owned())?
            .iter()
            .map(|v| match v {
                Value::Int(i) => Some(*i as f64),
                Value::Number(text) => text.parse::<f64>().ok(),
                _ => None,
            })
            .collect::<Option<Vec<f64>>>()
            .filter(|n| n.len() == 4)
            .ok_or_else(|| "mesh.normal is not four numbers".to_owned())?;
        let frames = geometry.frames.as_f32();
        if frames.len() != 600 * 16 {
            return Err("the cell frames are not 600 4 x 4 matrices".to_owned());
        }
        let mut centres = Vec::with_capacity(600);
        for cell in 0..600 {
            let mut c = [0.0f64; 4];
            for (k, n) in normal.iter().enumerate() {
                for (i, ci) in c.iter_mut().enumerate() {
                    *ci += f64::from(frames[(cell * 4 + k) * 4 + i]) * n; // column-major: column k, row i
                }
            }
            let length = c.iter().map(|v| v * v).sum::<f64>().sqrt();
            if length <= 0.0 {
                return Err(format!("cell {cell}'s centre has no length"));
            }
            centres.push(c.map(|v| v / length));
        }
        let slot_piece = geometry.slot_piece.as_u32();
        if slot_piece.len() != 600 * PER_CELL {
            return Err("the slot pieces are not one per slot".to_owned());
        }
        Ok(ChartModel { centres, slot_piece })
    }

    /// Each slot's chart at a revision: with `posed` (J02's pose index per piece and row-major 4 x 4 poses acting on
    /// column vectors) the nearest cell to the slot's own cell centre under its piece's pose, else the slot's own
    /// cell. One search per distinct cell and pose.
    pub fn assign(&self, posed: Option<(&[i32], &[f32])>) -> Vec<u32> {
        let mut out = vec![0u32; self.slot_piece.len()];
        let Some((index, poses)) = posed else {
            for (slot, chart) in out.iter_mut().enumerate() {
                *chart = (slot / PER_CELL) as u32;
            }
            return out;
        };
        let mut found: HashMap<(u32, i32), u32> = HashMap::new();
        for (slot, chart) in out.iter_mut().enumerate() {
            let cell = slot / PER_CELL;
            let pose = index.get(self.slot_piece[slot] as usize).copied().unwrap_or(-1);
            let start = pose.max(0) as usize * 16;
            *chart = match (pose >= 0, poses.get(start..start + 16)) {
                (true, Some(m)) => *found.entry((cell as u32, pose)).or_insert_with(|| {
                    let c = &self.centres[cell];
                    let p: [f64; 4] = std::array::from_fn(|r| (0..4).map(|i| f64::from(m[r * 4 + i]) * c[i]).sum());
                    nearest(&self.centres, p)
                }),
                _ => cell as u32,
            };
        }
        out
    }
}

/// The cell whose centre is nearest `p` on the sphere (the largest cosine), ties within [`TIE`] to the lowest id.
pub fn nearest(centres: &[[f64; 4]], p: [f64; 4]) -> u32 {
    let length = p.iter().map(|v| v * v).sum::<f64>().sqrt().max(f64::MIN_POSITIVE);
    let cosine = |c: &[f64; 4]| (0..4).map(|i| c[i] * p[i]).sum::<f64>() / length;
    let best = centres.iter().map(cosine).fold(f64::NEG_INFINITY, f64::max);
    centres.iter().position(|c| cosine(c) >= best - TIE).unwrap_or(0) as u32
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::time::Instant;

    fn assets() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../assets")
    }

    fn unit(p: [f64; 4]) -> [f64; 4] {
        let n = p.iter().map(|v| v * v).sum::<f64>().sqrt();
        p.map(|v| v / n)
    }

    /// The rotation in the plane of unit `a` and unit `b` (not +-a) that takes `a` to `b`, row-major, in binary32.
    fn rotation(a: [f64; 4], b: [f64; 4]) -> Vec<f32> {
        let c: f64 = (0..4).map(|i| a[i] * b[i]).sum();
        let v = unit(std::array::from_fn(|i| b[i] - c * a[i]));
        let s = (1.0 - c * c).max(0.0).sqrt();
        let mut m = vec![0.0f32; 16];
        for r in 0..4 {
            for i in 0..4 {
                let identity = if r == i { 1.0 } else { 0.0 };
                m[r * 4 + i] = (identity + (c - 1.0) * (a[r] * a[i] + v[r] * v[i]) + s * (v[r] * a[i] - a[r] * v[i])) as f32;
            }
        }
        m
    }

    fn reference(model: &ChartModel, index: &[i32], poses: &[f32]) -> Vec<u32> {
        (0..model.slot_piece.len())
            .map(|slot| {
                let c = model.centres[slot / PER_CELL];
                let m = &poses[index[model.slot_piece[slot] as usize] as usize * 16..][..16];
                let p: [f64; 4] = std::array::from_fn(|r| (0..4).map(|i| f64::from(m[r * 4 + i]) * c[i]).sum());
                nearest(&model.centres, p)
            })
            .collect()
    }

    #[test]
    fn the_chart_assignment_is_nearest_cell_lowest_id() {
        let geometry = GeometryInput::load(&assets()).expect("the shipped geometry loads");
        let model = ChartModel::load(&geometry, &assets()).expect("the chart model loads");
        assert_eq!(model.centres.len(), 600);
        // Unposed: every slot's chart is its own cell, and each centre is its own nearest cell.
        let own = model.assign(None);
        assert!(own.iter().enumerate().all(|(slot, &chart)| chart as usize == slot / PER_CELL));
        assert!((0..600).all(|k| nearest(&model.centres, model.centres[k]) == k as u32));
        // A neighbour a of cell 0, a neighbour b of a, and points equidistant from two cells: the lower id wins, either way round.
        let cos = |a: usize, b: usize| (0..4).map(|i| model.centres[a][i] * model.centres[b][i]).sum::<f64>();
        let closest = (1..600).map(|k| cos(0, k)).fold(f64::NEG_INFINITY, f64::max);
        let a = (1..600).find(|&k| cos(0, k) > closest - 1e-5).expect("cell 0 has a neighbour");
        let next = (0..600).filter(|&k| k != a && k != 0).map(|k| cos(a, k)).fold(f64::NEG_INFINITY, f64::max);
        let b = (0..600).find(|&k| k != a && k != 0 && cos(a, k) > next - 1e-5).expect("a has a neighbour besides cell 0");
        let mid = |x: usize, y: usize| unit(std::array::from_fn(|i| model.centres[x][i] + model.centres[y][i]));
        assert_eq!(nearest(&model.centres, mid(a, b)), a.min(b) as u32);
        assert_eq!(nearest(&model.centres, mid(b, a)), a.min(b) as u32);
        assert_eq!(nearest(&model.centres, mid(a, 0)), 0, "a tie between cell 0 and a neighbour goes to cell 0");
        // Posed, binary32 poses: pose 0 the identity, pose 1 takes cell 0's centre exactly to neighbour a, pose 2 to
        // the tie point of a and b (the lower wins); pieces cycle through the three.
        let mut poses: Vec<f32> = (0..16).map(|e| if e % 5 == 0 { 1.0 } else { 0.0 }).collect();
        poses.extend(rotation(model.centres[0], model.centres[a]));
        poses.extend(rotation(model.centres[0], mid(a, b)));
        let index: Vec<i32> = (0..177_120).map(|piece| piece % 3).collect();
        let charts = model.assign(Some((&index, &poses)));
        assert_eq!(charts, reference(&model, &index, &poses), "the search per cell and pose is the per-slot search");
        for slot in 0..PER_CELL {
            let expected = match index[model.slot_piece[slot] as usize] {
                0 => 0,
                1 => a as u32,
                _ => a.min(b) as u32,
            };
            assert_eq!(charts[slot], expected, "cell 0's slot {slot}");
        }
        // The assignment's time per revision (B22's Machine line), on this machine: unposed, three poses, and one
        // generic pose per piece (each slot its own search, the worst case).
        let started = Instant::now();
        let _ = model.assign(None);
        let unposed = started.elapsed();
        let started = Instant::now();
        let _ = model.assign(Some((&index, &poses)));
        let three = started.elapsed();
        let mut many = Vec::with_capacity(177_120 * 16);
        for piece in 0..177_120usize {
            let x = piece as f64;
            let target = unit([x.sin(), (1.3 * x).cos(), (0.7 * x).sin(), (2.1 * x).cos() + 0.5]);
            many.extend(rotation(model.centres[piece % 600], target));
        }
        let each: Vec<i32> = (0..177_120).collect();
        let started = Instant::now();
        let worst = model.assign(Some((&each, &many)));
        let worst_time = started.elapsed();
        assert!(worst.iter().all(|&chart| chart < 600));
        println!("chart assignment per revision: unposed {unposed:?}, three poses {three:?}, one pose per piece {worst_time:?}");
    }
}

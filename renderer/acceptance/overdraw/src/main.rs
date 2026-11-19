//! `c9-overdraw --root <repository> --out <file.jsonl> [--first <k>] [--last <k>] [--theta-steps <n>] [--threads <n>]`
//! `c9-overdraw --out <file.jsonl> --merge <part.jsonl> [--merge <part.jsonl> ...]`
//!
//! Sizes the renderer gate stages' declared overdraw (C9 U5c) from RPI.72's raster bound U of the gate scene W3 with
//! the gate stages in its chain, headless: no device runs. U is computed at 1920x1080, 2560x1440 and 2560x1600, since
//! the owner's native resolution is not on record and U per pixel depends on the resolution (each small triangle
//! counts at least one 2x2 quad).
//!
//! The scene, as the core draws W3 with a user vertex stage (renderer/core/shaders/stages.hlsl, `m600_eval_stage`,
//! over geometry.hlsl): each of the 600 cells instances the 30,480 base vertices (10,160 triangles of 433 stickers),
//! so a frame evaluates the stage 18,288,000 times and counts 6,096,000 triangles over all 259,800 slots at full
//! detail. Camera frame k's view rotation Q_k is k rotations by 0.002 rad in the (0, 3) plane from the identity, each
//! in double and rounded to binary32, as `sb::rotate` does (renderer/core/src/sb/probe.cpp); 3,142 frames turn it once.
//! A traced frame turns the 4,605 moving slots of work/experiments/renderer-sb/workload/turn.json by the turn clock's
//! theta, which takes every value in [-angle, angle] at some wall-clock time; the tool samples theta at
//! `angle * j / n` for j in -n..=n (`--theta-steps n`, default 16) and reports the largest U of the samples. A
//! frame's U is the sum of the fixed triangles' U and the moving triangles' U at that theta, which is RPI.72's sum.
//!
//! Per vertex: `homePoint`, the turn (theta not 0), `mul(Q, world)`, then the gate vertex stage through c600-render's
//! binary32 CPU reference (`c600_render::stage::evaluate`), then `m600_clip` with the surface's aspect. Per triangle:
//! the adapter's classification (`m600_count`): a coverage code or a nonfinite clip component makes it undefined;
//! three vertices beyond one clip plane make it outside; a vertex with z below 0 or w not above 0 makes it cross the
//! near plane; any other is placed with D3D12's viewport transform. Then `c600_display::raster::bound` over one pass
//! of one shading sample, which is what the core counts for W3 (no ID pass and no multisampling in the stage chain).
//! The arithmetic follows the shader text's order in binary32; a device may contract or reorder it, so a device's U
//! may differ where a bounding box edge falls on a pixel or quad boundary. Agreement is recorded, never a proof
//! (RPI.35).
//!
//! The assets are read and checked as the core's `Assets` does: the manifest's profile and SHA-256 digests, the mesh
//! counts and ranges, the shrink anchors against their pinned SHA-256, the turn's format and model identity.
//!
//! Output, JSON lines (`magic600-stage-overdraw-cpu/1`): a header, one row per camera frame and a summary. The summary
//! gives, per surface size, the largest U and U per pixel over the frames, the overdraw that size needs with a margin
//! of 5/4 (`ceil(5 U / (4 pixels))`), and the overdraw the tool proposes: the largest of the three. It charges
//! RM.38's `frame_work` from the declared values (vertex evaluations times the vertex stage's `ops`, plus surface
//! pixels times the proposed overdraw times the fragment stage's `ops`) against the shell host's limit profile,
//! `shell-draft/1` with every ceiling 2^40 (renderer/bridge/host/src/lib.rs).
//!
//! The camera frames are independent, so a sweep may run as several processes over disjoint `--first`..`--last`
//! ranges (each process starts the camera from the identity). `--merge` reads such parts, checks that their headers
//! agree apart from the range and the thread count and that their frames are contiguous with no repeat, and writes
//! one file of the same form: the header, every row in frame order and the summary of all rows. Exit codes: 0 done;
//! 2 a usage, asset, stage or part error.
//!
//! Outside CI: the crate is a workspace of its own (its own `Cargo.lock`, and `.cargo/config.toml` pointing at
//! `rust/vendor`) with path dependencies on c600-render, c600-display and their path crates. Neither the rust
//! workspace's tests, CI nor the landing rules' reruns build it, so a change to those crates, `raster::bound` and the
//! stage reference included, is not caught here. Its result, overdraw 108, is pinned in the gate stages
//! (`renderer/acceptance/stages/vertex.6cs`, `fragment.6cs` and `limits.json`) and in
//! `rust/c600-render/tests/stage_hlsl.rs`. To recheck it after such a change, run `cargo test --locked --offline`
//! here (the crate has no unit tests: the run builds it against the current crates), then the sweep that
//! `renderer/acceptance/README.md` gives, and compare the summary's proposed overdraw with 108.

use std::fmt::Write as _;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

use c600_check::CheckedStage;
use c600_display::raster::{bound, Pass, Surface, Triangle};
use c600_display::LimitProfile;
use c600_ir::{ModuleKind, StageKind};
use c600_kernel::json::{self, Value};
use c600_render::stage::{bind, evaluate, parse_limits, Binding, Coverage, Inputs};

const VERTICES: usize = 30_480;
const TRIANGLES: usize = VERTICES / 3;
const STICKERS: usize = 433;
const CELLS: usize = 600;
const SLOTS: usize = CELLS * STICKERS;
const MOVING: usize = 4_605;
/// The vertex evaluations of one W3 frame: one per (cell, base vertex).
const EVALUATIONS: u64 = (CELLS * VERTICES) as u64;
/// SPEC section 3's pinned shrink anchors (renderer/core/src/sb/probe.cpp, `AnchorSha256`).
const ANCHOR_SHA256: &str = "0b6ead284d3f62719e3e6ec893d6c199d49698d246a59164bc362446e39faca6";
const CAMERA_STEP: f64 = 0.002;
/// renderer/core/src/scene.cpp, `constants()`: cellShrink, stickerShrink, zoom.
const CELL_SHRINK: f32 = 0.76;
const STICKER_SHRINK: f32 = 0.82;
const ZOOM: f32 = 1.15;
const SIZES: [(u32, u32); 3] = [(1920, 1080), (2560, 1440), (2560, 1600)];
/// The margin over the largest U per pixel, as a fraction.
const MARGIN: (u128, u128) = (5, 4);
/// The shell host's limit profile (renderer/bridge/host/src/lib.rs, `LIMITS` and `LimitProfile::uniform`).
const PROFILE: &str = "shell-draft/1";
const PROFILE_VALUE: u64 = 1 << 40;
const FORMAT: &str = "magic600-stage-overdraw-cpu/1";
const USAGE: &str = "usage: c9-overdraw --root <repository> --out <file.jsonl> [--first <k>] [--last <k>] \
                     [--theta-steps <n>] [--threads <n>]\n       c9-overdraw --out <file.jsonl> --merge <part.jsonl> \
                     [--merge <part.jsonl> ...]";

struct Args {
    root: PathBuf,
    out: PathBuf,
    first: u64,
    last: u64,
    theta_steps: u32,
    threads: usize,
    merge: Vec<PathBuf>,
}

fn args() -> Result<Args, String> {
    let mut root = None;
    let mut out = None;
    let mut first = 1u64;
    let mut last = 3_142u64;
    let mut theta_steps = 16u32;
    let mut threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1);
    let mut merge = Vec::new();
    let mut sweep = false;
    let mut it = std::env::args_os().skip(1);
    while let Some(flag) = it.next() {
        let flag = flag.into_string().map_err(|_| "an argument is no UTF-8 text".to_string())?;
        let value = it.next().ok_or_else(|| format!("`{flag}` needs a value"))?;
        let number = |v: &std::ffi::OsStr| -> Result<u64, String> {
            v.to_str().and_then(|s| s.parse().ok()).ok_or_else(|| format!("`{flag}` needs a whole number"))
        };
        sweep |= !matches!(flag.as_str(), "--out" | "--merge");
        match flag.as_str() {
            "--root" => root = Some(PathBuf::from(value)),
            "--out" => out = Some(PathBuf::from(value)),
            "--merge" => merge.push(PathBuf::from(value)),
            "--first" => first = number(&value)?,
            "--last" => last = number(&value)?,
            "--theta-steps" => {
                theta_steps = u32::try_from(number(&value)?).map_err(|_| "`--theta-steps` is too large".to_string())?
            }
            "--threads" => {
                threads = usize::try_from(number(&value)?).map_err(|_| "`--threads` is too large".to_string())?
            }
            other => return Err(format!("unknown argument `{other}`")),
        }
    }
    let out = out.ok_or("`--out` is missing")?;
    if !merge.is_empty() {
        if sweep {
            return Err("`--merge` takes only `--out`".into());
        }
        return Ok(Args { root: PathBuf::new(), out, first, last, theta_steps, threads, merge });
    }
    if first == 0 || first > last {
        return Err("the frames run from `--first` 1 or later to `--last`, not below `--first`".into());
    }
    if theta_steps == 0 || threads == 0 {
        return Err("`--theta-steps` and `--threads` must be positive".into());
    }
    Ok(Args { root: root.ok_or("`--root` is missing")?, out, first, last, theta_steps, threads, merge })
}

fn read(path: &Path) -> Result<Vec<u8>, String> {
    std::fs::read(path).map_err(|e| format!("cannot read {}: {e}", path.display()))
}

fn read_json(path: &Path) -> Result<Value, String> {
    json::parse(&read(path)?).map_err(|e| format!("{}: {e}", path.display()))
}

fn number(value: Option<&Value>) -> Option<f64> {
    match value? {
        Value::Int(i) => Some(*i as f64),
        Value::Number(lexeme) => lexeme.parse().ok(),
        _ => None,
    }
}

fn vector4(value: Option<&Value>, what: &str) -> Result<[f32; 4], String> {
    let items = value.and_then(Value::as_array).filter(|a| a.len() == 4).ok_or(format!("{what} is no 4-vector"))?;
    let mut v = [0f32; 4];
    for (slot, item) in v.iter_mut().zip(items) {
        *slot = number(Some(item)).ok_or(format!("{what} holds a value that is no number"))? as f32;
    }
    Ok(v)
}

fn words(bytes: &[u8], count: usize, what: &str) -> Result<Vec<[u8; 4]>, String> {
    if bytes.len() != count * 4 {
        return Err(format!("{what} holds {} bytes, and {} are needed", bytes.len(), count * 4));
    }
    Ok(bytes.chunks_exact(4).map(|c| [c[0], c[1], c[2], c[3]]).collect())
}

fn quads(values: Vec<[u8; 4]>) -> Vec<[f32; 4]> {
    values
        .chunks_exact(4)
        .map(|c| {
            [f32::from_le_bytes(c[0]), f32::from_le_bytes(c[1]), f32::from_le_bytes(c[2]), f32::from_le_bytes(c[3])]
        })
        .collect()
}

struct Assets {
    vertices: Vec<[f32; 4]>,
    local: Vec<u32>,
    /// Four columns per cell.
    frames: Vec<[f32; 4]>,
    centers: Vec<[f32; 4]>,
    normal: [f32; 4],
    radius: f32,
    plane_u: [f32; 4],
    plane_v: [f32; 4],
    angle: f64,
    moving: Vec<bool>,
    model_id: String,
    /// One base vertex for each distinct pair of sticker and vertex bits, and each base vertex's index in it.
    distinct: Vec<usize>,
    canon: Vec<u32>,
}

/// SPEC section 3: area-weighted triangle centroids in double, one rounding to binary32, pinned bytes.
fn shrink_anchors(offsets: &[usize], vertices: &[[f32; 4]]) -> Result<Vec<[f32; 4]>, String> {
    let mut centers = Vec::with_capacity(STICKERS);
    for l in 0..STICKERS {
        let mut total = 0f64;
        let mut weighted = [0f64; 4];
        let mut t = offsets[l];
        while t < offsets[l + 1] {
            let [a, b, c] = [vertices[t], vertices[t + 1], vertices[t + 2]].map(|v| v.map(f64::from));
            let e1: [f64; 4] = std::array::from_fn(|i| b[i] - a[i]);
            let e2: [f64; 4] = std::array::from_fn(|i| c[i] - a[i]);
            let d11 = e1[0] * e1[0] + e1[1] * e1[1] + e1[2] * e1[2] + e1[3] * e1[3];
            let d22 = e2[0] * e2[0] + e2[1] * e2[1] + e2[2] * e2[2] + e2[3] * e2[3];
            let d12 = e1[0] * e2[0] + e1[1] * e2[1] + e1[2] * e2[2] + e1[3] * e2[3];
            let area = 0.5 * (d11 * d22 - d12 * d12).max(0.0).sqrt();
            total += area;
            for i in 0..4 {
                weighted[i] += area * ((a[i] + b[i] + c[i]) / 3.0);
            }
            t += 3;
        }
        if !(total.is_finite() && total > 0.0) {
            return Err(format!("sticker {l}: no finite positive triangle area"));
        }
        centers.push(std::array::from_fn(|i| (weighted[i] / total) as f32));
    }
    let bytes: Vec<u8> = centers.iter().flatten().flat_map(|f: &f32| f.to_le_bytes()).collect();
    if c600_digest::sha256_hex(&bytes) != ANCHOR_SHA256 {
        return Err("shrink anchors SHA-256 mismatch with SPEC section 3".into());
    }
    Ok(centers)
}

fn load(root: &Path) -> Result<Assets, String> {
    let manifest = read_json(&root.join("assets/manifest.json"))?;
    if manifest.get("puzzle").and_then(Value::as_str) != Some("600-cell-Full") {
        return Err("assets/manifest.json: not the 600-cell-Full profile".into());
    }
    let files = manifest.get("files").and_then(Value::as_object).ok_or("assets/manifest.json: no files")?;
    for (name, digest) in files {
        let digest = digest.as_str().ok_or(format!("assets/manifest.json: the digest of {name} is no text"))?;
        if c600_digest::sha256_hex(&read(&root.join(name))?) != digest {
            return Err(format!("asset SHA-256 mismatch: {name}"));
        }
    }
    let model_id = manifest.get("model_id").and_then(Value::as_str).ok_or("assets/manifest.json: no model_id")?;
    let mesh = read_json(&root.join("assets/mesh.json"))?;
    for (key, want) in [("base_vertices", VERTICES), ("base_stickers", STICKERS), ("cells", CELLS), ("slots", SLOTS)] {
        if mesh.get(key).and_then(Value::as_i64) != i64::try_from(want).ok() {
            return Err("assets/mesh.json: not the full-detail mesh".into());
        }
    }
    let offsets: Vec<usize> = mesh
        .get("offsets")
        .and_then(Value::as_array)
        .and_then(|a| a.iter().map(|v| v.as_i64().and_then(|i| usize::try_from(i).ok())).collect())
        .ok_or("assets/mesh.json: offsets are no list of counts")?;
    if offsets.len() != STICKERS + 1 || offsets[0] != 0 || offsets[STICKERS] != VERTICES {
        return Err("assets/mesh.json: bad mesh ranges".into());
    }
    let normal = vector4(mesh.get("normal"), "assets/mesh.json: normal")?;
    let radius = number(mesh.get("normal_length")).ok_or("assets/mesh.json: no normal_length")? as f32;
    if !(radius > 0.0) {
        return Err("assets/mesh.json: bad mesh ranges".into());
    }
    let vertices = quads(words(&read(&root.join("assets/mesh_vertices.f32"))?, VERTICES * 4, "mesh_vertices.f32")?);
    let local: Vec<u32> = words(&read(&root.join("assets/mesh_sticker.u32"))?, VERTICES, "mesh_sticker.u32")?
        .into_iter()
        .map(u32::from_le_bytes)
        .collect();
    let frames = quads(words(&read(&root.join("assets/cell_frames.f32"))?, CELLS * 16, "cell_frames.f32")?);
    for l in 0..STICKERS {
        let (start, end) = (offsets[l], offsets[l + 1]);
        if start >= end || (end - start) % 3 != 0 {
            return Err("assets/mesh.json: bad triangle range".into());
        }
        if local[start..end].iter().any(|&s| s as usize != l) {
            return Err("assets: sticker/range mismatch".into());
        }
    }
    if vertices.iter().chain(&frames).flatten().any(|f| !f.is_finite()) {
        return Err("assets: nonfinite geometry asset".into());
    }
    let centers = shrink_anchors(&offsets, &vertices)?;
    let turn = read_json(&root.join("work/experiments/renderer-sb/workload/turn.json"))?;
    if turn.get("format").and_then(Value::as_str) != Some("magic600-sb-turn-v1")
        || turn.get("model_id").and_then(Value::as_str) != Some(model_id)
    {
        return Err("turn.json: wrong turn identity".into());
    }
    let plane_u = vector4(turn.get("plane_u"), "turn.json: plane_u")?;
    let plane_v = vector4(turn.get("plane_v"), "turn.json: plane_v")?;
    let angle = number(turn.get("angle")).filter(|a| a.is_finite()).ok_or("turn.json: no angle")?;
    let slots: Vec<usize> = turn
        .get("moving_slots")
        .and_then(Value::as_array)
        .and_then(|a| a.iter().map(|v| v.as_i64().and_then(|i| usize::try_from(i).ok())).collect())
        .ok_or("turn.json: moving_slots are no list of slots")?;
    let mut moving = vec![false; SLOTS];
    for &slot in &slots {
        if slot >= SLOTS || std::mem::replace(&mut moving[slot], true) {
            return Err("turn.json: a moving slot is out of range or repeated".into());
        }
    }
    if slots.len() != MOVING {
        return Err("turn.json: unexpected turn size".into());
    }
    // A triangle corner is a base vertex of one sticker; corners shared between triangles repeat the same bits.
    let mut index = std::collections::HashMap::new();
    let mut distinct = Vec::new();
    let mut canon = Vec::with_capacity(VERTICES);
    for vi in 0..VERTICES {
        let next = u32::try_from(distinct.len()).expect("fewer than 2^32 vertices");
        let d = *index.entry((local[vi], vertices[vi].map(f32::to_bits))).or_insert(next);
        if d == next {
            distinct.push(vi);
        }
        canon.push(d);
    }
    Ok(Assets {
        vertices,
        local,
        frames,
        centers,
        normal,
        radius,
        plane_u,
        plane_v,
        angle,
        moving,
        model_id: model_id.to_string(),
        distinct,
        canon,
    })
}

struct Stage {
    checked: CheckedStage,
    binding: Binding,
    digest: String,
}

fn admit(path: &Path, limits: &c600_check::StageLimits, kind: StageKind) -> Result<Stage, String> {
    let file = c600_syntax::parse(&read(path)?);
    if let Some(d) = file.diagnostics.first() {
        return Err(format!("{}: {}: {}", path.display(), d.code, d.message));
    }
    let lowered = c600_syntax::lower(std::slice::from_ref(&file)).map_err(|e| format!("{}: {e}", path.display()))?;
    let stages: Vec<&str> = lowered
        .program
        .modules
        .iter()
        .filter(|m| matches!(m.kind, ModuleKind::Stage(_)))
        .map(|m| m.name.as_str())
        .collect();
    let [module] = stages.as_slice() else {
        return Err(format!("{}: {} stage modules, and one is needed", path.display(), stages.len()));
    };
    let admission = c600_check::admit_stage(&lowered.program, module, "main", limits)
        .map_err(|e| format!("{}: {e}", path.display()))?;
    c600_display::stage::take_up(&admission).map_err(|errors| {
        let texts: Vec<String> = errors.iter().map(|e| e.to_string()).collect();
        format!("{}: {}", path.display(), texts.join("; "))
    })?;
    let checked = admission.stage.ok_or(format!("{}: the admission holds no stage", path.display()))?;
    if checked.kind() != kind {
        return Err(format!("{}: the stage is not of the expected kind", path.display()));
    }
    let binding = bind(&checked).map_err(|errors| {
        let texts: Vec<String> = errors.iter().map(|e| e.to_string()).collect();
        format!("{}: {}", path.display(), texts.join("; "))
    })?;
    let digest = c600_display::stage::describe(&checked).digest;
    Ok(Stage { checked, binding, digest })
}

/// The vertex stage's buffer from arrays.json: its declared arrays in order, each value a binary32 bit pattern.
fn stage_arrays(path: &Path, binding: &Binding) -> Result<(Vec<f32>, Vec<String>), String> {
    let doc = read_json(path)?;
    if doc.get("format").and_then(Value::as_str) != Some("magic600-stage-arrays/1") {
        return Err(format!("{}: not magic600-stage-arrays/1", path.display()));
    }
    let table = doc.get("vertex").and_then(Value::as_object).ok_or(format!("{}: no vertex arrays", path.display()))?;
    if table.len() != binding.arrays.len() {
        return Err(format!("{}: the vertex arrays are not the stage's", path.display()));
    }
    let mut values = Vec::new();
    let mut bits = Vec::new();
    for array in &binding.arrays {
        let list =
            table.get(&array.name).and_then(Value::as_array).ok_or(format!("{}: no {}", path.display(), array.name))?;
        if u64::try_from(list.len()).ok() != Some(array.length) {
            return Err(format!("{}: {} has the wrong length", path.display(), array.name));
        }
        for item in list {
            let text = item.as_str().and_then(|s| s.strip_prefix("0x")).filter(|s| s.len() == 8);
            let word = text.and_then(|s| u32::from_str_radix(s, 16).ok());
            let word =
                word.ok_or(format!("{}: {} holds a value that is no bit pattern", path.display(), array.name))?;
            values.push(f32::from_bits(word));
            bits.push(format!("0x{word:08x}"));
        }
    }
    Ok((values, bits))
}

fn identity() -> [f32; 16] {
    std::array::from_fn(|i| if i % 5 == 0 { 1.0 } else { 0.0 })
}

/// `sb::rotate`: columns `a` and `b` of the column-major matrix, in double, each value rounded to binary32.
fn rotate(q: &mut [f32; 16], a: usize, b: usize, theta: f64) {
    let (c, s) = (theta.cos(), theta.sin());
    for j in 0..4 {
        let (x, y) = (f64::from(q[a * 4 + j]), f64::from(q[b * 4 + j]));
        q[a * 4 + j] = (c * x - s * y) as f32;
        q[b * 4 + j] = (s * x + c * y) as f32;
    }
}

#[derive(Clone, Copy)]
struct View {
    surface: Surface,
    aspect: f32,
}

struct Scene<'a> {
    assets: &'a Assets,
    stage: &'a Stage,
    arrays: &'a [f32],
    thetas: &'a [f32],
    views: [View; 3],
    near_scale: f32,
}

#[derive(Clone, Default)]
struct Tally {
    fixed: [u128; 3],
    /// Per theta sample: the moving triangles' U.
    moving: Vec<[u128; 3]>,
    /// At theta 0, over every triangle.
    outside: [u64; 3],
    near: [u64; 3],
    undefined: [u64; 3],
    evaluations: u64,
}

impl Scene<'_> {
    /// geometry.hlsl's `homePoint`.
    fn home(&self, cell: usize, vi: usize) -> [f32; 4] {
        let a = self.assets;
        let center = a.centers[a.local[vi] as usize];
        let vertex = a.vertices[vi];
        let shrink = CELL_SHRINK * STICKER_SHRINK;
        let v: [f32; 4] = std::array::from_fn(|i| {
            (a.normal[i] + CELL_SHRINK * (center[i] - a.normal[i]) + shrink * (vertex[i] - center[i])) / a.radius
        });
        let f = &a.frames[cell * 4..cell * 4 + 4];
        std::array::from_fn(|i| f[0][i] * v[0] + f[1][i] * v[1] + f[2][i] * v[2] + f[3][i] * v[3])
    }

    /// projectVertex's turn of a moving slot.
    fn turn(&self, world: &mut [f32; 4], theta: f32) {
        let (u, v) = (self.assets.plane_u, self.assets.plane_v);
        let x = world[0] * u[0] + world[1] * u[1] + world[2] * u[2] + world[3] * u[3];
        let y = world[0] * v[0] + world[1] * v[1] + world[2] * v[2] + world[3] * v[3];
        let (c, s) = (theta.cos(), theta.sin());
        let (p, r) = ((c - 1.0) * x - s * y, s * x + (c - 1.0) * y);
        for i in 0..4 {
            world[i] += p * u[i] + r * v[i];
        }
    }

    /// The gate vertex stage through the CPU reference: the position, or `None` for a coverage code.
    fn position(&self, world: [f32; 4]) -> Option<[f32; 3]> {
        let inputs = Inputs { x: world[0], y: world[1], z: world[2], w: world[3], ..Inputs::default() };
        let evaluation = evaluate(&self.stage.checked, &self.stage.binding, &inputs, self.arrays)
            .unwrap_or_else(|e| panic!("the CPU reference refused an evaluation: {e}"));
        (evaluation.coverage == Coverage::Point).then_some(evaluation.value)
    }

    /// `m600_clip` and `m600_store`: a nonfinite clip position is undefined.
    fn clip(&self, p: [f32; 3], aspect: f32) -> Option<[f32; 4]> {
        let z = 5.0 - p[2];
        let clip = [p[0] * ZOOM / aspect, p[1] * ZOOM, self.near_scale * (z - 0.05), z];
        clip.iter().all(|v| v.is_finite()).then_some(clip)
    }

    /// The cells that `next` hands out, each at every theta sample. A base vertex's chain depends only on the cell,
    /// its sticker, its bits and theta, so each distinct pair of sticker and bits is evaluated once per cell and theta,
    /// and every corner that repeats it takes the same clip position: the same binary32 inputs give the same outputs.
    fn cells(&self, q: &[f32; 16], next: &AtomicUsize) -> Tally {
        let a = self.assets;
        let mut tally = Tally { moving: vec![[0; 3]; self.thetas.len()], ..Tally::default() };
        let mut clips: Vec<[Option<[f32; 4]>; 3]> = vec![[None; 3]; a.distinct.len()];
        let mut fixed: [Vec<Triangle>; 3] = Default::default();
        let mut moving: [Vec<Triangle>; 3] = Default::default();
        loop {
            let cell = next.fetch_add(1, Ordering::Relaxed);
            if cell >= CELLS {
                return tally;
            }
            for (j, &theta) in self.thetas.iter().enumerate() {
                for (d, &vi) in a.distinct.iter().enumerate() {
                    let turning = a.moving[cell * STICKERS + a.local[vi] as usize];
                    if j > 0 && !turning {
                        continue;
                    }
                    let mut world = self.home(cell, vi);
                    if theta != 0.0 && turning {
                        self.turn(&mut world, theta);
                    }
                    let world: [f32; 4] = std::array::from_fn(|r| {
                        q[r] * world[0] + q[4 + r] * world[1] + q[8 + r] * world[2] + q[12 + r] * world[3]
                    });
                    tally.evaluations += 1;
                    let p = self.position(world);
                    for (s, view) in self.views.iter().enumerate() {
                        clips[d][s] = p.and_then(|p| self.clip(p, view.aspect));
                    }
                }
                fixed.iter_mut().chain(moving.iter_mut()).for_each(Vec::clear);
                for tri in 0..TRIANGLES {
                    let turning = a.moving[cell * STICKERS + a.local[3 * tri] as usize];
                    if j > 0 && !turning {
                        continue;
                    }
                    let corners = [0, 1, 2].map(|v| a.canon[3 * tri + v] as usize);
                    for (s, view) in self.views.iter().enumerate() {
                        let triangle = classify(corners.map(|d| clips[d][s]), view.surface);
                        if j == 0 {
                            match triangle {
                                Triangle::Outside => tally.outside[s] += 1,
                                Triangle::CrossesNear => tally.near[s] += 1,
                                Triangle::Undefined => tally.undefined[s] += 1,
                                Triangle::Placed(_) => {}
                            }
                        }
                        (if turning { &mut moving[s] } else { &mut fixed[s] }).push(triangle);
                    }
                }
                for (s, view) in self.views.iter().enumerate() {
                    if j == 0 {
                        tally.fixed[s] += bound(view.surface, &[Pass { triangles: &fixed[s], samples: 1 }]);
                    }
                    tally.moving[j][s] += bound(view.surface, &[Pass { triangles: &moving[s], samples: 1 }]);
                }
            }
        }
    }
}

/// `m600_count`'s classification and D3D12's viewport transform (`m600_pixel`).
fn classify(clips: [Option<[f32; 4]>; 3], surface: Surface) -> Triangle {
    let [Some(a), Some(b), Some(c)] = clips else { return Triangle::Undefined };
    let all = |f: fn(&[f32; 4]) -> bool| f(&a) && f(&b) && f(&c);
    if all(|p| p[0] > p[3])
        || all(|p| p[0] < -p[3])
        || all(|p| p[1] > p[3])
        || all(|p| p[1] < -p[3])
        || all(|p| p[2] < 0.0)
        || all(|p| p[2] > p[3])
    {
        return Triangle::Outside;
    }
    if [a, b, c].iter().any(|p| p[2] < 0.0 || !(p[3] > 0.0)) {
        return Triangle::CrossesNear;
    }
    let (width, height) = (surface.width as f32, surface.height as f32);
    let pixel = |p: [f32; 4]| {
        let (nx, ny) = (p[0] / p[3], p[1] / p[3]);
        [(nx * 0.5 + 0.5) * width, (0.5 - ny * 0.5) * height]
    };
    let corners = [pixel(a), pixel(b), pixel(c)];
    if corners.iter().flatten().any(|v| !v.is_finite()) {
        return Triangle::Undefined;
    }
    Triangle::Placed(corners)
}

fn quote(text: &str) -> String {
    let mut out = String::from("\"");
    for ch in text.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if u32::from(c) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", u32::from(c));
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// `u / pixels` with four decimals, truncated.
fn per_pixel(u: u128, pixels: u128) -> String {
    format!("{}.{:04}", u / pixels, (u % pixels) * 10_000 / pixels)
}

/// One surface size's figures in one camera frame: U at theta 0, and the largest and smallest U of the samples.
#[derive(Clone, Copy, Default)]
struct SizeRow {
    u0: u128,
    max: u128,
    theta_max: f32,
    min: u128,
    theta_min: f32,
}

/// One camera frame's row.
struct Frame {
    frame: u64,
    sizes: [SizeRow; 3],
    seconds: f64,
}

/// The declared values that the summary charges.
struct Declared {
    stage_ops: u64,
    fragment_ops: u64,
    overdraw: u64,
}

/// The summary over every frame's row: per size the largest U (with its frame and theta), the largest at theta 0
/// and the smallest, the overdraw the size needs with the margin, RM.38's `frame_work` at the proposal, and the
/// frames that would stop at the proposal, at the declared overdraw and at U5a's draft of 8.
fn summary(frames: &[Frame], declared: &Declared) -> Result<String, String> {
    let (first, last) = match (frames.first(), frames.last()) {
        (Some(first), Some(last)) => (first.frame, last.frame),
        _ => return Err("there are no frames to summarize".into()),
    };
    let (num, den) = MARGIN;
    let pixels = |s: usize| u128::from(SIZES[s].0) * u128::from(SIZES[s].1);
    let largest = |s: usize, key: fn(&SizeRow) -> u128| {
        frames.iter().max_by_key(|f| (key(&f.sizes[s]), std::cmp::Reverse(f.frame))).expect("frames")
    };
    let needed: Vec<u128> =
        (0..3).map(|s| (num * largest(s, |r| r.max).sizes[s].max + den * pixels(s) - 1) / (den * pixels(s))).collect();
    let proposal = *needed.iter().max().expect("three sizes");
    let proposal = u64::try_from(proposal).map_err(|_| "the proposed overdraw does not fit 64 bits".to_string())?;
    let profile = LimitProfile::uniform(PROFILE, PROFILE_VALUE);
    let ceiling = |field: &str| profile.get(field).expect("an RM.11 field");
    let mut fits = EVALUATIONS <= ceiling("view_vertex_evals")
        && declared.stage_ops <= ceiling("stage_ops")
        && declared.fragment_ops <= ceiling("fragment_ops")
        && proposal <= ceiling("overdraw");
    let mut sizes = Vec::new();
    for (s, &(width, height)) in SIZES.iter().enumerate() {
        let p = pixels(s);
        let top = largest(s, |r| r.max);
        let still = largest(s, |r| r.u0);
        let low = frames.iter().min_by_key(|f| (f.sizes[s].min, f.frame)).expect("frames");
        let work = u128::from(EVALUATIONS) * u128::from(declared.stage_ops)
            + p * u128::from(proposal) * u128::from(declared.fragment_ops);
        fits &= work <= u128::from(ceiling("frame_work"));
        let stops = |overdraw: u64| frames.iter().filter(|f| f.sizes[s].max > p * u128::from(overdraw)).count();
        sizes.push(format!(
            "{{\"width\":{width},\"height\":{height},\"pixels\":{p},\"largest_u\":{},\"largest_frame\":{},\
             \"largest_theta\":{:?},\"largest_per_pixel\":\"{}\",\"largest_u_theta0\":{},\"largest_theta0_frame\":{},\
             \"largest_theta0_per_pixel\":\"{}\",\"smallest_u\":{},\"smallest_frame\":{},\"smallest_theta\":{:?},\
             \"smallest_per_pixel\":\"{}\",\"overdraw_needed\":{},\"frame_work\":{work},\
             \"frames_stopping_at_proposal\":{},\"frames_stopping_at_declared\":{},\"frames_stopping_at_8\":{}}}",
            top.sizes[s].max,
            top.frame,
            top.sizes[s].theta_max,
            per_pixel(top.sizes[s].max, p),
            still.sizes[s].u0,
            still.frame,
            per_pixel(still.sizes[s].u0, p),
            low.sizes[s].min,
            low.frame,
            low.sizes[s].theta_min,
            per_pixel(low.sizes[s].min, p),
            needed[s],
            stops(proposal),
            stops(declared.overdraw),
            stops(8)
        ));
    }
    Ok(format!(
        "{{\"summary\":true,\"frames\":{},\"first\":{first},\"last\":{last},\"margin\":\"{num}/{den}\",\
         \"overdraw\":{proposal},\"declared\":{},\"vertex_evaluations\":{EVALUATIONS},\"stage_ops\":{},\
         \"fragment_ops\":{},\"profile\":{},\"frame_work_ceiling\":{},\"view_vertex_evals_ceiling\":{},\
         \"overdraw_ceiling\":{},\"fits\":{fits},\"frame_seconds\":{:.1},\"sizes\":[{}]}}",
        frames.len(),
        declared.overdraw,
        declared.stage_ops,
        declared.fragment_ops,
        quote(PROFILE),
        ceiling("frame_work"),
        ceiling("view_vertex_evals"),
        ceiling("overdraw"),
        frames.iter().map(|f| f.seconds).sum::<f64>(),
        sizes.join(",")
    ))
}

/// A frame's row as the sweep writes it.
fn parse_row(row: &Value) -> Result<Frame, String> {
    let frame =
        row.get("frame").and_then(Value::as_i64).and_then(|k| u64::try_from(k).ok()).ok_or("a row has no frame")?;
    let seconds = number(row.get("seconds")).ok_or(format!("frame {frame}: no seconds"))?;
    let items = row.get("sizes").and_then(Value::as_array).filter(|a| a.len() == 3);
    let items = items.ok_or(format!("frame {frame}: not three sizes"))?;
    let mut sizes = [SizeRow::default(); 3];
    for (s, item) in items.iter().enumerate() {
        let (width, height) = SIZES[s];
        if item.get("width").and_then(Value::as_i64) != Some(i64::from(width))
            || item.get("height").and_then(Value::as_i64) != Some(i64::from(height))
        {
            return Err(format!("frame {frame}: size {s} is not {width}x{height}"));
        }
        let whole = |key: &str| -> Result<u128, String> {
            let value = item.get(key).and_then(Value::as_i64).and_then(|i| u128::try_from(i).ok());
            value.ok_or(format!("frame {frame}: no {key}"))
        };
        let single = |key: &str| -> Result<f32, String> {
            let value = match item.get(key) {
                Some(Value::Number(lexeme)) => lexeme.parse::<f32>().ok(),
                Some(Value::Int(i)) => Some(*i as f32),
                _ => None,
            };
            value.ok_or(format!("frame {frame}: no {key}"))
        };
        sizes[s] = SizeRow {
            u0: whole("u_theta0")?,
            max: whole("u_max")?,
            theta_max: single("theta_max")?,
            min: whole("u_min")?,
            theta_min: single("theta_min")?,
        };
    }
    Ok(Frame { frame, sizes, seconds })
}

/// `--merge`: the parts of one sweep as one file.
fn merge(out: &Path, parts: &[PathBuf]) -> Result<String, String> {
    let mut known: Option<Value> = None;
    let mut header: Option<Value> = None;
    let mut rows: Vec<(Frame, String)> = Vec::new();
    for part in parts {
        let text = String::from_utf8(read(part)?).map_err(|_| format!("{} is no UTF-8 text", part.display()))?;
        let mut lines = text.lines();
        let head =
            json::parse(lines.next().unwrap_or("").as_bytes()).map_err(|e| format!("{}: {e}", part.display()))?;
        if head.get("format").and_then(Value::as_str) != Some(FORMAT) {
            return Err(format!("{} is not a {FORMAT} part", part.display()));
        }
        let mut key = head.clone();
        if let Value::Object(fields) = &mut key {
            for field in ["first", "last", "threads"] {
                fields.remove(field);
            }
        }
        match &known {
            None => known = Some(key),
            Some(k) if *k == key => {}
            Some(_) => return Err(format!("{}: its header is not the other parts'", part.display())),
        }
        header.get_or_insert(head);
        for line in lines.filter(|l| !l.trim().is_empty()) {
            let row = json::parse(line.as_bytes()).map_err(|e| format!("{}: {e}", part.display()))?;
            if row.get("summary").is_none() {
                rows.push((parse_row(&row).map_err(|e| format!("{}: {e}", part.display()))?, line.to_string()));
            }
        }
    }
    rows.sort_by_key(|(f, _)| f.frame);
    if let Some(pair) = rows.windows(2).find(|pair| pair[1].0.frame != pair[0].0.frame + 1) {
        return Err(format!("the parts' frames are not contiguous after frame {}", pair[0].0.frame));
    }
    let mut header = header.ok_or("no parts")?;
    let field = |object: &str, key: &str| -> Result<u64, String> {
        let value = header.get(object).and_then(|o| o.get(key)).and_then(Value::as_i64);
        value.and_then(|v| u64::try_from(v).ok()).ok_or(format!("the header has no {object} {key}"))
    };
    let declared = Declared {
        stage_ops: field("vertex", "ops")?,
        fragment_ops: field("fragment", "ops")?,
        overdraw: field("vertex", "overdraw")?,
    };
    let (texts, frames): (Vec<String>, Vec<Frame>) = rows.into_iter().map(|(f, t)| (t, f)).unzip();
    let summary = summary(&frames, &declared)?;
    if let Value::Object(fields) = &mut header {
        let count = |n: u64| Value::Int(i64::try_from(n).expect("a frame count fits i64"));
        fields.insert("first".into(), count(frames[0].frame));
        fields.insert("last".into(), count(frames[frames.len() - 1].frame));
        fields.remove("threads");
        fields.insert("parts".into(), count(parts.len() as u64));
    }
    let mut text = header.canonical();
    text.push('\n');
    for row in &texts {
        text.push_str(row);
        text.push('\n');
    }
    text.push_str(&summary);
    text.push('\n');
    std::fs::write(out, text).map_err(|e| format!("cannot write {}: {e}", out.display()))?;
    Ok(summary)
}

fn run() -> Result<ExitCode, String> {
    let args = args().map_err(|e| format!("{e}\n{USAGE}"))?;
    let summary = if args.merge.is_empty() { sweep(&args)? } else { merge(&args.out, &args.merge)? };
    println!("{summary}");
    Ok(ExitCode::SUCCESS)
}

/// The sweep over `--first`..=`--last`, written to `--out` as it goes; its summary.
fn sweep(args: &Args) -> Result<String, String> {
    let root = &args.root;
    let stages = root.join("renderer/acceptance/stages");
    let limits = parse_limits(&read(&stages.join("limits.json"))?)?;
    let vertex = admit(&stages.join("vertex.6cs"), &limits, StageKind::Vertex)?;
    let fragment = admit(&stages.join("fragment.6cs"), &limits, StageKind::Fragment)?;
    let (arrays, array_bits) = stage_arrays(&stages.join("arrays.json"), &vertex.binding)?;
    let (stage_ops, declared) = (vertex.checked.declaration().ops, vertex.checked.declaration().overdraw);
    let fragment_ops = fragment.checked.declaration().ops;
    if fragment.checked.declaration().overdraw != declared {
        return Err("the gate stages declare different overdraws".into());
    }
    let assets = load(root)?;
    let n = i64::from(args.theta_steps);
    let mut thetas = vec![0f32];
    thetas.extend((-n..=n).filter(|&j| j != 0).map(|j| (assets.angle * j as f64 / n as f64) as f32));
    let views =
        SIZES.map(|(width, height)| View { surface: Surface { width, height }, aspect: width as f32 / height as f32 });
    let scene = Scene {
        assets: &assets,
        stage: &vertex,
        arrays: &arrays,
        thetas: &thetas,
        views,
        near_scale: (100.0f64 / (100.0 - 0.05)) as f32,
    };
    let file = std::fs::File::create(&args.out).map_err(|e| format!("cannot create {}: {e}", args.out.display()))?;
    let mut out = std::io::BufWriter::new(file);
    let line = |text: String, out: &mut std::io::BufWriter<std::fs::File>| -> Result<(), String> {
        writeln!(out, "{text}")
            .and_then(|_| out.flush())
            .map_err(|e| format!("cannot write {}: {e}", args.out.display()))
    };
    let sizes_json: Vec<String> = SIZES.iter().map(|(w, h)| format!("{{\"width\":{w},\"height\":{h}}}")).collect();
    let thetas_json: Vec<String> = thetas.iter().map(|t| format!("{t:?}")).collect();
    let bits_json: Vec<String> = array_bits.iter().map(|b| quote(b)).collect();
    line(
        format!(
            "{{\"format\":{},\"scene\":\"w3\",\"model_id\":{},\"anchors\":{},\"vertex\":{{\"digest\":{},\"ops\":{stage_ops},\
             \"overdraw\":{declared},\"d4\":[{}]}},\"fragment\":{{\"digest\":{},\"ops\":{fragment_ops}}},\"sizes\":[{}],\
             \"camera_step_rad\":{CAMERA_STEP},\"angle\":{},\"theta_steps\":{},\"thetas\":[{}],\"first\":{},\"last\":{},\
             \"evaluations_per_frame\":{EVALUATIONS},\"distinct_vertices\":{},\"passes\":1,\"samples\":1,\"threads\":{}}}",
            quote(FORMAT),
            quote(&assets.model_id),
            quote(ANCHOR_SHA256),
            quote(&vertex.digest),
            bits_json.join(","),
            quote(&fragment.digest),
            sizes_json.join(","),
            assets.angle,
            args.theta_steps,
            thetas_json.join(","),
            args.first,
            args.last,
            assets.distinct.len(),
            args.threads
        ),
        &mut out,
    )?;
    eprintln!(
        "c9-overdraw: W3, frames {}..={}, {} theta samples, {} threads; vertex {} fragment {}",
        args.first,
        args.last,
        thetas.len(),
        args.threads,
        &vertex.digest[..12],
        &fragment.digest[..12]
    );
    let declared = Declared { stage_ops, fragment_ops, overdraw: declared };
    let mut frames: Vec<Frame> = Vec::new();
    let mut q = identity();
    let started = Instant::now();
    for k in 1..=args.last {
        rotate(&mut q, 0, 3, CAMERA_STEP);
        if k < args.first {
            continue;
        }
        let frame_start = Instant::now();
        let next = AtomicUsize::new(0);
        let tallies: Vec<Tally> = std::thread::scope(|s| {
            let workers: Vec<_> = (0..args.threads).map(|_| s.spawn(|| scene.cells(&q, &next))).collect();
            workers.into_iter().map(|w| w.join().expect("a worker failed")).collect()
        });
        let mut total = Tally { moving: vec![[0; 3]; thetas.len()], ..Tally::default() };
        for t in &tallies {
            total.evaluations += t.evaluations;
            for s in 0..3 {
                total.fixed[s] += t.fixed[s];
                total.outside[s] += t.outside[s];
                total.near[s] += t.near[s];
                total.undefined[s] += t.undefined[s];
                for (j, m) in t.moving.iter().enumerate() {
                    total.moving[j][s] += m[s];
                }
            }
        }
        let mut rows = Vec::new();
        let mut sizes = [SizeRow::default(); 3];
        for (s, view) in views.iter().enumerate() {
            let u: Vec<u128> = total.moving.iter().map(|m| total.fixed[s] + m[s]).collect();
            let top = (0..u.len()).fold(0, |t, j| if u[j] > u[t] { j } else { t });
            let low = (0..u.len()).fold(0, |l, j| if u[j] < u[l] { j } else { l });
            sizes[s] = SizeRow { u0: u[0], max: u[top], theta_max: thetas[top], min: u[low], theta_min: thetas[low] };
            rows.push(format!(
                "{{\"width\":{},\"height\":{},\"u_theta0\":{},\"u_max\":{},\"theta_max\":{:?},\"u_min\":{},\"theta_min\":{:?},\
                 \"outside\":{},\"near\":{},\"undefined\":{}}}",
                view.surface.width,
                view.surface.height,
                u[0],
                u[top],
                thetas[top],
                u[low],
                thetas[low],
                total.outside[s],
                total.near[s],
                total.undefined[s]
            ));
        }
        // Rounded as written, so that a merged file summarizes exactly what a single run would.
        let seconds = (frame_start.elapsed().as_secs_f64() * 1000.0).round() / 1000.0;
        line(
            format!(
                "{{\"frame\":{k},\"stage_evaluations\":{},\"seconds\":{seconds:.3},\"sizes\":[{}]}}",
                total.evaluations,
                rows.join(",")
            ),
            &mut out,
        )?;
        if k == args.first || k % 50 == 0 || k == args.last {
            eprintln!(
                "frame {k}: U/px {} {} {} ({seconds:.2} s; {:.0} s so far)",
                per_pixel(sizes[0].max, u128::from(views[0].surface.pixels())),
                per_pixel(sizes[1].max, u128::from(views[1].surface.pixels())),
                per_pixel(sizes[2].max, u128::from(views[2].surface.pixels())),
                started.elapsed().as_secs_f64()
            );
        }
        frames.push(Frame { frame: k, sizes, seconds });
    }
    let summary = summary(&frames, &declared)?;
    line(summary.clone(), &mut out)?;
    Ok(summary)
}

fn main() -> ExitCode {
    run().unwrap_or_else(|e| {
        eprintln!("c9-overdraw: {e}");
        ExitCode::from(2)
    })
}

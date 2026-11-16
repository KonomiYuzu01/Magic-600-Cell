//! Reading 600-cell-Full from `assets/`: the manifest and its file digests, the arrays of `model.npz` and
//! `primitives.npz`, the execution trees, the census and the seed atlas. Only what the oracle rows and the star
//! certificates read is kept. Every check here fails closed with a [`LoadError`]; the 0.4 runtime trusts some of these
//! facts, and the kernel checks them instead because its loops rely on them (bounded indices, a tree whose walk
//! ends, a move that is a permutation of its support).

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use crate::archive::{self, Zip};
use crate::json::{self, Value};
use crate::LoadError;

pub(crate) const PUZZLE: &str = "600-cell-Full";
pub(crate) const SLOTS: usize = 259_800;
pub(crate) const PIECES: usize = 177_120;
pub(crate) const ORBITS: usize = 35;
pub(crate) const GENERATORS: usize = 1_200;
pub(crate) const CELLS: usize = 600;
pub(crate) const CELL_STICKERS: usize = 433;

/// The largest decompressed member or file the loader accepts.
const LIMIT: usize = 1 << 30;
/// The files the kernel reads besides the manifest. Each must be listed in the manifest, so its digest is checked.
const NEEDED: [&str; 5] = ["model.npz", "primitives.npz", "execution_trees.json.gz", "census.json", "seed_atlas.json"];

pub(crate) struct Tree {
    pub seed: Vec<i32>,
    pub relocation: Vec<i32>,
    pub parent: Vec<i32>,
    pub moves: Vec<i32>,
    pub buffers: [usize; 2],
    /// Per frame node, the slots that identify the transported star's third piece and its orientation.
    pub frames: Vec<Vec<i32>>,
}

pub(crate) struct Atlas {
    pub word: Vec<i32>,
    pub collateral: Vec<i64>,
    /// The slots of the seed's three target pieces, in cycle order.
    pub frames: [Vec<i32>; 3],
}

pub(crate) struct Census {
    pub rank: i64,
    pub colors: i64,
    pub orientation_group: String,
}

pub(crate) struct Model {
    pub model_id: String,
    pub manifest_sha256: String,
    pub verified_files: usize,
    pub slot_piece: Vec<i32>,
    pub piece_orbit: Vec<i32>,
    pub slot_orbit: Vec<i32>,
    pub first_slot: Vec<i32>,
    pub mask_offsets: Vec<i32>,
    pub mask_values: Vec<i32>,
    pub move_offsets: Vec<i32>,
    pub move_source: Vec<i32>,
    pub move_destination: Vec<i32>,
    pub trees: Vec<Tree>,
    pub atlas: Vec<Atlas>,
    pub census: Vec<Census>,
}

fn fail(message: impl Into<String>) -> LoadError {
    LoadError(message.into())
}

fn is_digest(text: &str) -> bool {
    text.len() == 64 && text.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

pub(crate) fn valid_primitive(m: i64) -> bool {
    m != 0 && m.unsigned_abs() <= GENERATORS as u64
}

fn read(assets: &Path, name: &str) -> Result<Vec<u8>, LoadError> {
    fs::read(assets.join(name)).map_err(|_| fail(format!("Cannot read assets/{name}")))
}

fn array(zip: &Zip<'_>, member: &str, descr: &str, length: Option<usize>) -> Result<Vec<i32>, LoadError> {
    let bytes = zip.read(member, LIMIT).map_err(|e| fail(format!("{member}: {e}")))?;
    let npy = archive::read_npy(&bytes).map_err(|e| fail(format!("{member}: {e}")))?;
    if npy.descr != descr || npy.shape.len() != 1 || length.is_some_and(|n| npy.shape[0] != n) {
        return Err(fail(format!("{member} has an unexpected dtype or shape")));
    }
    Ok(npy.values)
}

fn ints(value: Option<&Value>, what: &str) -> Result<Vec<i64>, LoadError> {
    let items = value.and_then(Value::as_array).ok_or_else(|| fail(format!("{what} is not a list")))?;
    items.iter().map(|item| item.as_i64().ok_or_else(|| fail(format!("{what} holds a non-integer")))).collect()
}

fn primitives(value: Option<&Value>, what: &str) -> Result<Vec<i32>, LoadError> {
    let values = ints(value, what)?;
    if !values.iter().all(|&m| valid_primitive(m)) {
        return Err(fail(format!("{what} holds an invalid primitive")));
    }
    Ok(values.into_iter().map(|m| m as i32).collect())
}

fn slots(value: &Value, what: &str) -> Result<Vec<i32>, LoadError> {
    let slots = ints(Some(value), what)?;
    if !slots.iter().all(|&s| (0..SLOTS as i64).contains(&s)) {
        return Err(fail(format!("{what} names a slot out of range")));
    }
    Ok(slots.into_iter().map(|s| s as i32).collect())
}

fn offsets_ok(offsets: &[i32], total: usize) -> bool {
    offsets.first() == Some(&0)
        && offsets.windows(2).all(|w| w[0] <= w[1])
        && offsets.last().is_some_and(|&last| last as usize == total)
}

impl Model {
    pub(crate) fn load(assets: &Path) -> Result<Model, LoadError> {
        let manifest_bytes = read(assets, "manifest.json")?;
        let manifest_sha256 = c600_digest::sha256_hex(&manifest_bytes);
        let manifest = json::parse(&manifest_bytes).map_err(|e| fail(format!("manifest.json: {e}")))?;
        if manifest.get("puzzle").and_then(Value::as_str) != Some(PUZZLE) {
            return Err(fail("manifest.json does not name 600-cell-Full"));
        }
        let model_id = match manifest.get("model_id").and_then(Value::as_str) {
            Some(id) if is_digest(id) => id.to_owned(),
            _ => return Err(fail("manifest.json has no well-formed model_id")),
        };
        let files =
            manifest.get("files").and_then(Value::as_object).ok_or_else(|| fail("manifest.json lists no files"))?;
        // Verify every listed file, as the 0.4 Model does, and keep the verified bytes of the files read below.
        let mut kept: BTreeMap<&str, Vec<u8>> = BTreeMap::new();
        for (name, digest) in files {
            let base = match name.strip_prefix("assets/") {
                Some(base) if !base.is_empty() && !base.contains(['/', '\\']) && base != "." && base != ".." => base,
                _ => return Err(fail("manifest.json lists a file outside assets/")),
            };
            let digest =
                digest.as_str().filter(|d| is_digest(d)).ok_or_else(|| fail("Malformed digest in manifest"))?;
            let bytes = read(assets, base)?;
            if c600_digest::sha256_hex(&bytes) != digest {
                return Err(fail(format!("Asset hash mismatch: {name}")));
            }
            if let Some(needed) = NEEDED.iter().find(|&&needed| needed == base) {
                kept.insert(*needed, bytes);
            }
        }
        let mut take =
            |name: &str| kept.remove(name).ok_or_else(|| fail(format!("manifest.json does not list {name}")));
        let (model_npz, primitives_npz) = (take("model.npz")?, take("primitives.npz")?);
        let (trees_gz, census_bytes, atlas_bytes) =
            (take("execution_trees.json.gz")?, take("census.json")?, take("seed_atlas.json")?);

        let zip = Zip::parse(&model_npz).map_err(|e| fail(format!("model.npz: {e}")))?;
        let slot_piece = array(&zip, "slot_piece.npy", "<i4", Some(SLOTS))?;
        let piece_orbit = array(&zip, "orbit_id.npy", "<i2", Some(PIECES))?;
        let face_offsets = array(&zip, "face_offsets.npy", "<i4", Some(PIECES + 1))?;
        let mask_offsets = array(&zip, "mask_offsets.npy", "<i4", Some(PIECES + 1))?;
        let mask_values = array(&zip, "mask_values.npy", "<i2", None)?;
        let zip = Zip::parse(&primitives_npz).map_err(|e| fail(format!("primitives.npz: {e}")))?;
        let move_offsets = array(&zip, "offsets.npy", "<i4", Some(GENERATORS + 1))?;
        let move_source = array(&zip, "src.npy", "<i4", None)?;
        let move_destination = array(&zip, "dst.npy", "<i4", None)?;

        if !slot_piece.iter().all(|&p| (0..PIECES as i32).contains(&p)) {
            return Err(fail("slot_piece holds a piece out of range"));
        }
        if !piece_orbit.iter().all(|&o| (-1..ORBITS as i32).contains(&o)) {
            return Err(fail("orbit_id holds an orbit out of range"));
        }
        // 0.4 takes a piece's first slot as the stable sort of slot_piece cut at face_offsets; that is the piece's
        // smallest slot exactly when face_offsets counts each piece's slots.
        let mut counts = vec![0i32; PIECES];
        let mut first_slot = vec![-1i32; PIECES];
        for (slot, &piece) in slot_piece.iter().enumerate() {
            counts[piece as usize] += 1;
            if first_slot[piece as usize] < 0 {
                first_slot[piece as usize] = slot as i32;
            }
        }
        if !offsets_ok(&face_offsets, SLOTS) || (0..PIECES).any(|p| face_offsets[p + 1] - face_offsets[p] != counts[p])
        {
            return Err(fail("face_offsets disagree with slot_piece"));
        }
        if counts.contains(&0) {
            return Err(fail("A piece has no slot"));
        }
        if !offsets_ok(&mask_offsets, mask_values.len()) || !mask_values.iter().all(|&c| (0..CELLS as i32).contains(&c))
        {
            return Err(fail("mask_offsets or mask_values are malformed"));
        }
        if !offsets_ok(&move_offsets, move_source.len()) || move_source.len() != move_destination.len() {
            return Err(fail("Primitive offsets disagree with the arrays"));
        }
        if !move_source.iter().chain(&move_destination).all(|&s| (0..SLOTS as i32).contains(&s)) {
            return Err(fail("A primitive names a slot out of range"));
        }
        // Each move lists its sources in increasing order, and its destinations are the same set: a permutation of
        // its support. Inverses swap the two lists, and conjugation looks sources up by value.
        let mut mark = vec![0u32; SLOTS];
        for i in 0..GENERATORS {
            let range = move_offsets[i] as usize..move_offsets[i + 1] as usize;
            let (source, destination) = (&move_source[range.clone()], &move_destination[range]);
            if !source.windows(2).all(|w| w[0] < w[1]) {
                return Err(fail("Primitive sources are not strictly increasing"));
            }
            let tag = i as u32 + 1;
            for &s in source {
                mark[s as usize] = tag;
            }
            for &d in destination {
                if mark[d as usize] != tag {
                    return Err(fail("A primitive is not a permutation of its support"));
                }
                mark[d as usize] = 0;
            }
        }
        let slot_orbit = slot_piece.iter().map(|&p| piece_orbit[p as usize]).collect();

        let census = json::parse(&census_bytes).map_err(|e| fail(format!("census.json: {e}")))?;
        let rows = census.get("orbits").and_then(Value::as_array).filter(|rows| rows.len() == ORBITS);
        let rows = rows.ok_or_else(|| fail("census.json does not hold 35 orbits"))?;
        let mut census = Vec::with_capacity(ORBITS);
        for (i, row) in rows.iter().enumerate() {
            let field = |key: &str| row.get(key).and_then(Value::as_i64);
            match (field("id"), field("rank"), field("colors"), row.get("orientation_group").and_then(Value::as_str)) {
                (Some(id), Some(rank), Some(colors), Some(group)) if id == i as i64 && colors > 0 => {
                    census.push(Census { rank, colors, orientation_group: group.to_owned() })
                }
                _ => return Err(fail("census.json holds a malformed orbit row")),
            }
        }

        let atlas_json = json::parse(&atlas_bytes).map_err(|e| fail(format!("seed_atlas.json: {e}")))?;
        let rows = atlas_json.as_array().filter(|rows| rows.len() == ORBITS);
        let rows = rows.ok_or_else(|| fail("seed_atlas.json does not hold 35 orbits"))?;
        let mut atlas = Vec::with_capacity(ORBITS);
        for (i, row) in rows.iter().enumerate() {
            if row.get("orbit").and_then(Value::as_i64) != Some(i as i64) {
                return Err(fail("seed_atlas.json rows are out of order"));
            }
            let frames = row.get("frames").and_then(Value::as_array).filter(|f| f.len() == 3);
            let frames = frames.ok_or_else(|| fail("seed_atlas.json frames are malformed"))?;
            atlas.push(Atlas {
                word: primitives(row.get("word"), "seed_atlas.json word")?,
                collateral: ints(row.get("collateral"), "seed_atlas.json collateral")?,
                frames: [
                    slots(&frames[0], "seed_atlas.json frame")?,
                    slots(&frames[1], "seed_atlas.json frame")?,
                    slots(&frames[2], "seed_atlas.json frame")?,
                ],
            });
        }

        let trees_bytes =
            archive::gunzip(&trees_gz, LIMIT).map_err(|e| fail(format!("execution_trees.json.gz: {e}")))?;
        let trees_json = json::parse(&trees_bytes).map_err(|e| fail(format!("execution_trees.json.gz: {e}")))?;
        drop(trees_bytes);
        let rows = trees_json.as_array().filter(|rows| rows.len() == ORBITS);
        let rows = rows.ok_or_else(|| fail("execution_trees.json.gz does not hold 35 trees"))?;
        let mut trees = Vec::with_capacity(ORBITS);
        for (i, row) in rows.iter().enumerate() {
            if row.get("orbit").and_then(Value::as_i64) != Some(i as i64) {
                return Err(fail("Execution trees are out of order"));
            }
            let parent = ints(row.get("parent"), "Execution tree parents")?;
            let moves = ints(row.get("move"), "Execution tree moves")?;
            let buffers = ints(row.get("buffers"), "Execution tree buffers")?;
            let frames = row.get("frames").and_then(Value::as_array);
            let frames = frames.ok_or_else(|| fail("Execution tree frames are not a list"))?;
            // Every node but the root has an earlier parent, so the walk to the root ends.
            let shaped = !parent.is_empty()
                && parent[0] == -1
                && parent.iter().enumerate().skip(1).all(|(n, &p)| p >= 0 && (p as usize) < n)
                && moves.len() == parent.len()
                && frames.len() == parent.len()
                && moves.iter().skip(1).all(|&m| valid_primitive(m))
                && buffers.len() == 2
                && buffers.iter().all(|&b| (0..PIECES as i64).contains(&b));
            if !shaped {
                return Err(fail("An execution tree is malformed"));
            }
            trees.push(Tree {
                seed: primitives(row.get("seed"), "Execution tree seed")?,
                relocation: primitives(row.get("relocation"), "Execution tree relocation")?,
                parent: parent.iter().map(|&p| p as i32).collect(),
                moves: moves.iter().map(|&m| m as i32).collect(),
                buffers: [buffers[0] as usize, buffers[1] as usize],
                frames: frames.iter().map(|f| slots(f, "Execution tree frame")).collect::<Result<_, _>>()?,
            });
        }

        Ok(Model {
            model_id,
            manifest_sha256,
            verified_files: files.len(),
            slot_piece,
            piece_orbit,
            slot_orbit,
            first_slot,
            mask_offsets,
            mask_values,
            move_offsets,
            move_source,
            move_destination,
            trees,
            atlas,
            census,
        })
    }

    /// The source and destination lists of primitive `m`; an inverse swaps them.
    pub(crate) fn pairs(&self, m: i32) -> (&[i32], &[i32]) {
        let i = (m.unsigned_abs() - 1) as usize;
        let range = self.move_offsets[i] as usize..self.move_offsets[i + 1] as usize;
        let (source, destination) = (&self.move_source[range.clone()], &self.move_destination[range]);
        if m > 0 {
            (source, destination)
        } else {
            (destination, source)
        }
    }

    /// The cells of piece `p`'s cap mask (0.4 `Model.caps`).
    pub(crate) fn caps(&self, p: usize) -> &[i32] {
        &self.mask_values[self.mask_offsets[p] as usize..self.mask_offsets[p + 1] as usize]
    }
}

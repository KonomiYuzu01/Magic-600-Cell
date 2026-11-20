//! Reading 600-cell-Full from `assets/`: the manifest and its file digests, the arrays of `model.npz` and
//! `primitives.npz`, the execution trees, the census and the seed atlas. Only what the oracle rows and the star
//! certificates read is kept. Every check here fails closed with a [`LoadError`]; the 0.4 runtime trusts some of these
//! facts, and the kernel checks them instead because its loops rely on them (bounded indices, a tree whose walk
//! ends, a move that is a permutation of its support).
//!
//! Charter E3's startup (a draft): the files are hashed, the arrays unpacked and the execution trees read side by side
//! ([`crate::parallel`]). Each is taken where it was read one after another, so a load gives the same model and the
//! same first error.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use crate::archive::{self, Zip};
use crate::json::{self, Value};
use crate::parallel::{self, Background};
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
    /// Per generator 1 to 1,200 (index 0 to 1,199), bit `o` is set when the primitive moves a sticker of orbit `o`
    /// (0 to 34). Strict protection reads it once per literal letter.
    pub move_orbits: Vec<u64>,
    /// The home slots of each piece in increasing order: piece `p`'s are
    /// `piece_slots[piece_offsets[p]..piece_offsets[p + 1]]`.
    pub piece_offsets: Vec<i32>,
    pub piece_slots: Vec<i32>,
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

/// The file name in `assets/` of a manifest entry: `assets/` and a plain ASCII name of letters, digits, `.`, `_` and
/// `-` that is not `.` or `..`, so it cannot leave the folder on any platform (no separator, drive or stream colon).
fn plain_name(name: &str) -> Option<&str> {
    let base = name.strip_prefix("assets/")?;
    let plain = base.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
        && base.bytes().any(|b| b != b'.');
    plain.then_some(base)
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

/// The files of `assets/` after the asset checks of [`verify_assets`].
pub struct VerifiedAssets {
    /// The manifest's model ID, the SHA-256 of its file list.
    pub model_id: String,
    /// The SHA-256 of `manifest.json`.
    pub manifest_sha256: String,
    /// Every listed file (`assets/<name>`) with its digest, each checked against the file's bytes.
    pub digests: BTreeMap<String, String>,
    /// The bytes of the files asked for, by plain file name.
    pub files: BTreeMap<String, Vec<u8>>,
}

/// Reads and checks `assets/` (KN0.1): the manifest names 600-cell-Full, its model ID is the SHA-256 of its file
/// list, and every listed file has its listed digest. Returns the bytes of the `needed` files, each of which must be
/// listed. Every check fails closed with a [`LoadError`]. The kernel and the jumbling half both load through this, so
/// an identity block's model ID always names the files that were read.
pub fn verify_assets(assets: &Path, needed: &[&str]) -> Result<VerifiedAssets, LoadError> {
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
    let files = manifest.get("files").and_then(Value::as_object).ok_or_else(|| fail("manifest.json lists no files"))?;
    // The model ID names exactly the listed files and digests: build_assets.py writes it as the SHA-256 of
    // `json.dumps(files, sort_keys=True, separators=(',', ':'))`. Names are plain ASCII file names, so that text
    // needs no escapes, and the map's byte order is Python's key order. 0.4 does not check this; the kernel does,
    // because its identity block rests on the model ID (KN0.1).
    let mut listed = Vec::with_capacity(files.len());
    let mut file_list = String::from("{");
    for (name, digest) in files {
        let base = plain_name(name)
            .ok_or_else(|| fail("manifest.json lists a file that is not a plain file name in assets/"))?;
        let digest = digest.as_str().filter(|d| is_digest(d)).ok_or_else(|| fail("Malformed digest in manifest"))?;
        if !listed.is_empty() {
            file_list.push(',');
        }
        file_list.push_str(&format!("\"{name}\":\"{digest}\""));
        listed.push((name, base, digest));
    }
    file_list.push('}');
    if c600_digest::sha256_hex(file_list.as_bytes()) != model_id {
        return Err(fail("manifest.json's model_id does not match its file list"));
    }
    // Verify every listed file, as the 0.4 Model does, and keep the verified bytes of the needed files. The files are
    // read and hashed side by side and taken in the list's order, so the first failing file is the answer.
    let checked = parallel::map(listed.len(), |i| {
        let (name, base, digest) = listed[i];
        let bytes = read(assets, base)?;
        if c600_digest::sha256_hex(&bytes) != digest {
            return Err(fail(format!("Asset hash mismatch: {name}")));
        }
        Ok(needed.contains(&base).then_some(bytes))
    });
    let mut digests = BTreeMap::new();
    let mut kept = BTreeMap::new();
    for ((name, base, digest), checked) in listed.into_iter().zip(checked) {
        if let Some(bytes) = parallel::taken(checked)? {
            kept.insert(base.to_owned(), bytes);
        }
        digests.insert(name.clone(), digest.to_owned());
    }
    if let Some(missing) = needed.iter().find(|&&name| !kept.contains_key(name)) {
        return Err(fail(format!("manifest.json does not list {missing}")));
    }
    Ok(VerifiedAssets { model_id, manifest_sha256, digests, files: kept })
}

/// The execution trees of `execution_trees.json.gz`, checked: 35 trees in orbit order, each node's parent before it,
/// valid primitives, two buffer pieces and a frame per node.
fn read_trees(trees_gz: &[u8]) -> Result<Vec<Tree>, LoadError> {
    let trees_bytes = archive::gunzip(trees_gz, LIMIT).map_err(|e| fail(format!("execution_trees.json.gz: {e}")))?;
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
    Ok(trees)
}

impl Model {
    pub(crate) fn load(assets: &Path) -> Result<Model, LoadError> {
        let VerifiedAssets { model_id, manifest_sha256, digests, files: mut kept } = verify_assets(assets, &NEEDED)?;
        let mut take =
            |name: &str| kept.remove(name).ok_or_else(|| fail(format!("manifest.json does not list {name}")));
        let (model_npz, primitives_npz) = (take("model.npz")?, take("primitives.npz")?);
        let (trees_gz, census_bytes, atlas_bytes) =
            (take("execution_trees.json.gz")?, take("census.json")?, take("seed_atlas.json")?);
        // The execution trees read only their own file: they are read on a thread of their own while the arrays are
        // unpacked, and taken where they were read before, after the census and the atlas.
        let trees = Background::start("c600-kernel-load-trees", move || read_trees(&trees_gz));

        // The arrays are unpacked side by side and taken in this order. The primitives' arrays are unpacked only from
        // an archive that parses, and its failure is answered after the model's arrays, where it was read before.
        let model_zip = Zip::parse(&model_npz).map_err(|e| fail(format!("model.npz: {e}")))?;
        let primitives_zip = Zip::parse(&primitives_npz).map_err(|e| fail(format!("primitives.npz: {e}")));
        let mut members = vec![
            (&model_zip, "slot_piece.npy", "<i4", Some(SLOTS)),
            (&model_zip, "orbit_id.npy", "<i2", Some(PIECES)),
            (&model_zip, "face_offsets.npy", "<i4", Some(PIECES + 1)),
            (&model_zip, "mask_offsets.npy", "<i4", Some(PIECES + 1)),
            (&model_zip, "mask_values.npy", "<i2", None),
        ];
        if let Ok(zip) = &primitives_zip {
            members.extend([
                (zip, "offsets.npy", "<i4", Some(GENERATORS + 1)),
                (zip, "src.npy", "<i4", None),
                (zip, "dst.npy", "<i4", None),
            ]);
        }
        let unpacked = parallel::map(members.len(), |i| {
            let (zip, member, descr, length) = members[i];
            array(zip, member, descr, length)
        });
        let mut unpacked = unpacked.into_iter().map(parallel::taken);
        let mut next = || unpacked.next().expect("an array for each member");
        let slot_piece = next()?;
        let piece_orbit = next()?;
        let face_offsets = next()?;
        let mask_offsets = next()?;
        let mask_values = next()?;
        primitives_zip?;
        let move_offsets = next()?;
        let move_source = next()?;
        let move_destination = next()?;

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
        let slot_orbit: Vec<i32> = slot_piece.iter().map(|&p| piece_orbit[p as usize]).collect();
        // A listed pair whose source is its destination moves nothing, so it marks no orbit.
        let move_orbits = (0..GENERATORS)
            .map(|i| {
                let range = move_offsets[i] as usize..move_offsets[i + 1] as usize;
                let pairs = move_source[range.clone()].iter().zip(&move_destination[range]);
                pairs.filter(|(s, d)| s != d).fold(0u64, |mask, (&s, _)| match slot_orbit[s as usize] {
                    orbit if orbit >= 0 => mask | 1 << orbit,
                    _ => mask,
                })
            })
            .collect();
        // A counting sort of the slots by piece; face_offsets counts each piece's slots (checked above).
        let mut next = face_offsets[..PIECES].to_vec();
        let mut piece_slots = vec![0i32; SLOTS];
        for (slot, &piece) in slot_piece.iter().enumerate() {
            let at = &mut next[piece as usize];
            piece_slots[*at as usize] = slot as i32;
            *at += 1;
        }

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

        let trees = trees.join()?;

        Ok(Model {
            model_id,
            manifest_sha256,
            verified_files: digests.len(),
            slot_piece,
            piece_orbit,
            slot_orbit,
            first_slot,
            mask_offsets,
            mask_values,
            move_offsets,
            move_source,
            move_destination,
            move_orbits,
            piece_offsets: face_offsets,
            piece_slots,
            trees,
            atlas,
            census,
        })
    }

    /// The home slots of piece `p`, in increasing order.
    pub(crate) fn home_slots(&self, p: usize) -> &[i32] {
        &self.piece_slots[self.piece_offsets[p] as usize..self.piece_offsets[p + 1] as usize]
    }

    /// The orbits primitive `m` moves, as a bit set over orbits 0 to 34; an inverse moves the same stickers.
    pub(crate) fn letter_orbits(&self, m: i32) -> u64 {
        self.move_orbits[(m.unsigned_abs() - 1) as usize]
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

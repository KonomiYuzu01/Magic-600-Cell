//! The exact mechanics of 600-cell-Full, ported from `core.py` of 0.4: recipe normalizing with the 0.4 limits and
//! texts, expansion into literal letters, simultaneous application of source-to-destination permutations, net maps,
//! support per orbit, the star certificates and their cache, and the progress rows of `PuzzleState`.

use std::sync::Arc;

use crate::json::Value;
use crate::model::{self, Model, CELLS, CELL_STICKERS, ORBITS, PIECES, SLOTS};
use crate::{codes, CertificateError, ProgressRow, Refusal, Star, Step, SupportRow};

pub(crate) const MAX_STEPS: usize = 256;
pub(crate) const MAX_WORD: usize = 100_000;
pub(crate) const MAX_PRIMITIVES: usize = 3_000_000;
/// The number of star nets the cache keeps, as `Model.star_net` does.
const STAR_CACHE: usize = 96;

const STEP_COUNT: &str = "Recipe must contain 1..256 steps";
const BAD_STEP: &str = "Bad recipe step";
const WORD_LENGTH: &str = "Word length must be 1..100000";
pub(crate) const BAD_PRIMITIVE: &str = "Primitive ID must be an integer in ±1..±1200";
const BAD_STAR: &str = "Bad star fields";
const BAD_NODE: &str = "Frame node out of range";
const BAD_KIND: &str = "Only word and star steps are accepted";
pub(crate) const CEILING: &str = "Preview expansion exceeds 3,000,000 primitives; split the batch";

/// A net map: sources in increasing order and their destinations.
pub(crate) type Net = (Vec<i32>, Vec<i32>);

fn shape(message: &str) -> Refusal {
    Refusal { code: codes::RECIPE_SHAPE, message: message.to_owned(), fact: None }
}

fn certificate(message: &str) -> CertificateError {
    CertificateError(message.to_owned())
}

/// The letters of a word step; `None` stands for a letter that is not an integer.
pub(crate) fn word(letters: impl Iterator<Item = Option<i64>>, count: usize) -> Result<Vec<i32>, Refusal> {
    if !(1..=MAX_WORD).contains(&count) {
        return Err(shape(WORD_LENGTH));
    }
    letters
        .map(|m| m.filter(|&m| model::valid_primitive(m)).map(|m| m as i32).ok_or_else(|| shape(BAD_PRIMITIVE)))
        .collect()
}

/// `Model.normalize`: the whole recipe is checked before anything is computed, in 0.4's order and with its texts.
/// Integers must be JSON integers; a boolean, a float or a string never counts as one.
pub(crate) fn normalize(model: &Model, recipe: &Value) -> Result<(Vec<Step>, usize), Refusal> {
    let steps = match recipe.as_array() {
        Some(steps) if (1..=MAX_STEPS).contains(&steps.len()) => steps,
        _ => return Err(shape(STEP_COUNT)),
    };
    let mut out = Vec::with_capacity(steps.len());
    let mut length = 0usize;
    for step in steps {
        if step.as_object().is_none() {
            return Err(shape(BAD_STEP));
        }
        match step.get("kind").and_then(Value::as_str) {
            Some("word") => {
                let letters = step.get("moves").and_then(Value::as_array).ok_or_else(|| shape(WORD_LENGTH))?;
                let letters = word(letters.iter().map(Value::as_i64), letters.len())?;
                length += letters.len();
                out.push(Step::Word(letters));
            }
            Some("star") => {
                let orbit = step.get("orbit").and_then(Value::as_i64).filter(|o| (0..ORBITS as i64).contains(o));
                // A missing sign defaults to 1; a present null is not an integer.
                let sign = match step.get("sign") {
                    None => Some(1),
                    Some(sign) => sign.as_i64().filter(|&s| s == 1 || s == -1),
                };
                let (Some(orbit), Some(sign)) = (orbit, sign) else {
                    return Err(shape(BAD_STAR));
                };
                let tree = &model.trees[orbit as usize];
                let node =
                    step.get("node").and_then(Value::as_i64).filter(|n| (0..tree.parent.len() as i64).contains(n));
                let Some(node) = node else {
                    return Err(shape(BAD_NODE));
                };
                let path = path(model, orbit as usize, node as usize);
                length += tree.seed.len() + 2 * tree.relocation.len() + 2 * path.len();
                out.push(Step::Star(Star { orbit: orbit as usize, node: node as usize, sign: sign as i32, path }));
            }
            _ => return Err(shape(BAD_KIND)),
        }
    }
    if length > MAX_PRIMITIVES {
        return Err(Refusal { code: codes::PRIMITIVE_CEILING, message: CEILING.to_owned(), fact: None });
    }
    Ok((out, length))
}

/// `Model.path`: the letters on the way from the tree's root to `node`. The loader checked that every parent comes
/// before its child, so the walk ends.
pub(crate) fn path(model: &Model, orbit: usize, node: usize) -> Vec<i32> {
    let tree = &model.trees[orbit];
    let mut letters = Vec::new();
    let mut n = node;
    while tree.parent[n] >= 0 {
        letters.push(tree.moves[n]);
        n = tree.parent[n] as usize;
    }
    letters.reverse();
    letters
}

/// `Model.expand`: a star is `invword(path) + invword(relocation) + seed + relocation + path`, inverted whole for
/// sign -1.
pub(crate) fn expand(model: &Model, steps: &[Step], out: &mut Vec<i32>) {
    for step in steps {
        match step {
            Step::Word(word) => out.extend_from_slice(word),
            Step::Star(star) => {
                let tree = &model.trees[star.orbit];
                let start = out.len();
                out.extend(star.path.iter().rev().map(|m| -m));
                out.extend(tree.relocation.iter().rev().map(|m| -m));
                out.extend_from_slice(&tree.seed);
                out.extend_from_slice(&tree.relocation);
                out.extend_from_slice(&star.path);
                if star.sign < 0 {
                    out[start..].reverse();
                    for m in &mut out[start..] {
                        *m = -*m;
                    }
                }
            }
        }
    }
}

/// `target[d] = target[s]` for all pairs at once.
pub(crate) fn apply(target: &mut [i32], source: &[i32], destination: &[i32], scratch: &mut Vec<i32>) {
    scratch.clear();
    scratch.extend(source.iter().map(|&s| target[s as usize]));
    for (&d, &value) in destination.iter().zip(scratch.iter()) {
        target[d as usize] = value;
    }
}

/// The net map of an applied identity map (`map[d]` is the slot the sticker in `d` came from), sorted by source.
pub(crate) fn net_from_map(map: &[i32]) -> Net {
    let mut by_source = vec![-1i32; map.len()];
    for (d, &s) in map.iter().enumerate() {
        if s as usize != d {
            by_source[s as usize] = d as i32;
        }
    }
    let mut source = Vec::new();
    let mut destination = Vec::new();
    for (s, &d) in by_source.iter().enumerate() {
        if d >= 0 {
            source.push(s as i32);
            destination.push(d);
        }
    }
    (source, destination)
}

/// The SHA-256 of integers written as little-endian 32-bit values (`numpy` `<i4`).
pub(crate) fn digest_ints(values: &[i32]) -> String {
    let mut bytes = Vec::with_capacity(4 * values.len());
    for value in values {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    c600_digest::sha256_hex(&bytes)
}

/// `Model.support`: per orbit, the moved stickers and the distinct pieces they belong to.
pub(crate) fn support(model: &Model, source: &[i32]) -> Vec<SupportRow> {
    let mut stickers = [0usize; ORBITS + 1];
    let mut pieces = [0usize; ORBITS + 1];
    let mut seen = vec![false; PIECES];
    for &s in source {
        let piece = model.slot_piece[s as usize] as usize;
        let row = (model.piece_orbit[piece] + 1) as usize;
        stickers[row] += 1;
        if !seen[piece] {
            seen[piece] = true;
            pieces[row] += 1;
        }
    }
    (0..=ORBITS)
        .filter(|&row| stickers[row] > 0)
        .map(|row| SupportRow { orbit: row as i32 - 1, stickers: stickers[row], pieces: pieces[row] })
        .collect()
}

/// Maps both endpoints of a net through primitive `m` (`Model.mapped`). `image` is the identity on entry and on exit.
fn conjugate(model: &Model, m: i32, source: &mut [i32], destination: &mut [i32], image: &mut [i32]) {
    let (from, to) = model.pairs(m);
    for (&f, &t) in from.iter().zip(to) {
        image[f as usize] = t;
    }
    for value in source.iter_mut().chain(destination.iter_mut()) {
        *value = image[*value as usize];
    }
    for &f in from {
        image[f as usize] = f;
    }
}

fn sorted(source: Vec<i32>, destination: Vec<i32>) -> Net {
    let mut pairs: Vec<(i32, i32)> = source.into_iter().zip(destination).collect();
    pairs.sort_unstable();
    pairs.into_iter().unzip()
}

/// Whether the net moves each slot of `from` to the slot at the same place in `to` (0.4's `searchsorted` frame test;
/// frames of different lengths never match, as with `np.array_equal`).
fn maps(net: &Net, from: &[i32], to: &[i32]) -> bool {
    from.len() == to.len() && from.iter().zip(to).all(|(f, t)| net.0.binary_search(f).is_ok_and(|q| net.1[q] == *t))
}

/// `Model.certify_seed`: the seed's net map, checked as a three-piece cycle with the atlas's collateral, relocated,
/// and checked against the atlas frames.
fn certify(model: &Model, orbit: usize, image: &mut [i32]) -> Result<Net, CertificateError> {
    let tree = &model.trees[orbit];
    let atlas = &model.atlas[orbit];
    let mut map: Vec<i32> = (0..SLOTS as i32).collect();
    let mut scratch = Vec::new();
    for &m in &tree.seed {
        let (source, destination) = model.pairs(m);
        apply(&mut map, source, destination, &mut scratch);
    }
    let (mut source, mut destination) = net_from_map(&map);
    if tree.seed != atlas.word {
        return Err(certificate("Seed definition mismatch"));
    }
    let target: Vec<usize> =
        (0..source.len()).filter(|&i| model.slot_orbit[source[i] as usize] == orbit as i32).collect();
    let from: Vec<i32> = target.iter().map(|&i| model.slot_piece[source[i] as usize]).collect();
    let to: Vec<i32> = target.iter().map(|&i| model.slot_piece[destination[i] as usize]).collect();
    let mut distinct = from.clone();
    distinct.sort_unstable();
    distinct.dedup();
    if target.len() as i64 != 3 * model.census[orbit].colors || distinct.len() != 3 {
        return Err(certificate("Target seed is not a three-piece cycle"));
    }
    // A dictionary built from the pairs in order: a later pair for the same piece wins.
    let mut transport: Vec<(i32, i32)> = Vec::new();
    for (&a, &b) in from.iter().zip(&to) {
        match transport.iter_mut().find(|entry| entry.0 == a) {
            Some(entry) => entry.1 = b,
            None => transport.push((a, b)),
        }
    }
    let image_of = |x: i32| transport.iter().find(|entry| entry.0 == x).map(|entry| entry.1);
    if !transport.iter().all(|&(x, y)| y != x && image_of(y).and_then(image_of) == Some(x)) {
        return Err(certificate("Bad target piece transport"));
    }
    let mut collateral: Vec<i64> =
        source.iter().map(|&s| model.slot_orbit[s as usize] as i64).filter(|&o| o != orbit as i64).collect();
    collateral.sort_unstable();
    collateral.dedup();
    let mut expected = atlas.collateral.clone();
    expected.sort_unstable();
    if collateral != expected {
        return Err(certificate("Collateral orbit mismatch"));
    }
    for &m in &tree.relocation {
        conjugate(model, m, &mut source, &mut destination, image);
    }
    let net = sorted(source, destination);
    let [fa, fb, fc] = &atlas.frames;
    if ![(fa, fb), (fb, fc), (fc, fa)].iter().all(|&(x, y)| maps(&net, x, y)) {
        return Err(certificate("Relocated frame mismatch"));
    }
    Ok(net)
}

/// The certified seed per orbit and the most recent star nets.
pub(crate) struct Stars {
    base: Vec<Option<Arc<Net>>>,
    /// Least recently used first.
    recent: Vec<((usize, usize), Arc<Net>)>,
    image: Vec<i32>,
}

impl Stars {
    pub(crate) fn new() -> Stars {
        Stars { base: vec![None; ORBITS], recent: Vec::new(), image: (0..SLOTS as i32).collect() }
    }

    /// `Model.star_net` for sign 1: the relocated seed conjugated by the star's setup path, with the buffer guard
    /// and the frame checks. The caller swaps the two lists for sign -1.
    pub(crate) fn star(&mut self, model: &Model, star: &Star) -> Result<Arc<Net>, CertificateError> {
        let key = (star.orbit, star.node);
        if let Some(i) = self.recent.iter().position(|(k, _)| *k == key) {
            let entry = self.recent.remove(i);
            let net = Arc::clone(&entry.1);
            self.recent.push(entry);
            return Ok(net);
        }
        let base = match self.base[star.orbit].clone() {
            Some(base) => base,
            None => {
                let base = Arc::new(certify(model, star.orbit, &mut self.image)?);
                self.base[star.orbit] = Some(Arc::clone(&base));
                base
            }
        };
        let tree = &model.trees[star.orbit];
        let mut guard = [false; CELLS];
        for &buffer in &tree.buffers {
            for &cell in model.caps(buffer) {
                guard[cell as usize] = true;
            }
        }
        let (mut source, mut destination) = (base.0.clone(), base.1.clone());
        for &m in &star.path {
            if guard[((m.unsigned_abs() - 1) / 2) as usize] {
                return Err(certificate("Setup moves a guarded buffer position"));
            }
            conjugate(model, m, &mut source, &mut destination, &mut self.image);
        }
        let net = sorted(source, destination);
        let atlas = &model.atlas[star.orbit];
        let (fa, fb, fx) = (&atlas.frames[0], &atlas.frames[1], &tree.frames[star.node]);
        if ![(fa, fb), (fb, fx), (fx, fa)].iter().all(|&(x, y)| maps(&net, x, y)) {
            return Err(certificate("Transported star frame mismatch"));
        }
        let net = Arc::new(net);
        self.recent.push((key, Arc::clone(&net)));
        while self.recent.len() > STAR_CACHE {
            self.recent.remove(0);
        }
        Ok(net)
    }
}

/// `PuzzleState.progress`: per moving orbit, the pieces, the solved pieces, those off their position, those on
/// their position but turned, those with every sticker in its home cell, and the census fields.
pub(crate) fn progress(model: &Model, labels: &[i32]) -> Vec<ProgressRow> {
    let mut correct = vec![true; PIECES];
    let mut color = vec![true; PIECES];
    for (slot, &label) in labels.iter().enumerate() {
        let piece = model.slot_piece[slot] as usize;
        if label as usize != slot {
            correct[piece] = false;
        }
        if label as usize / CELL_STICKERS != slot / CELL_STICKERS {
            color[piece] = false;
        }
    }
    let mut counts = [[0usize; 5]; ORBITS];
    for piece in 0..PIECES {
        let orbit = model.piece_orbit[piece];
        if orbit < 0 {
            continue;
        }
        let at = model.slot_piece[labels[model.first_slot[piece] as usize] as usize] as usize;
        let position = at == piece;
        let row = &mut counts[orbit as usize];
        row[0] += 1;
        row[1] += usize::from(correct[piece]);
        row[2] += usize::from(!position);
        row[3] += usize::from(position && !correct[piece]);
        row[4] += usize::from(color[piece]);
    }
    counts
        .iter()
        .enumerate()
        .map(|(orbit, c)| {
            let census = &model.census[orbit];
            ProgressRow {
                orbit,
                pieces: c[0],
                solved: c[1],
                position_wrong: c[2],
                orientation_wrong: c[3],
                color_solved: c[4],
                rank: census.rank,
                stickers: census.colors,
                orientation: census.orientation_group.clone(),
            }
        })
        .collect()
}

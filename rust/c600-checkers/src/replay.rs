//! Letter-by-letter replay on the exact engines (KN6.2): `c600-jumble` for the jumbling puzzle, `c600-kernel` for
//! 600-cell-Full. The checker keeps no cache of its own: every letter is applied by the engine, every state digest and
//! every height is recomputed from the state after the letter, and nothing is carried between checks. The engines'
//! internal memos (the pose-facts table of `c600_jumble::observe`) never change a result; their oracles test them
//! separately. No search, chooser or construction code is reached: a jumbling letter is the menu item transported to
//! its cap by the retained frame, as the engine's menu defines it, and a 600-cell-Full letter is a primitive.

use std::sync::Arc;

use c600_jumble::model::{NP, POLES};
use c600_jumble::observe::{self, Tables};
use c600_jumble::q5::{is_rotation, matmul, matrix_from_json, transpose, Vec4, Q5};
use c600_jumble::state::{Outcome, Pose, State};
use c600_jumble::twist::{Menu, Twist};
use c600_jumble::{levels, Context};
use c600_kernel::json::{self, Value};
use c600_kernel::{Derivation, Kernel, ProtectedSet, Snapshot};

use crate::claims::{array, at, count, dot, field, hex64, int, keys, reject, start_mismatch, string, Run, Step};

// ------------------------------------------------------------------------------------------------ the jumbling puzzle

/// A jumbling state under a menu, replayed one letter at a time.
pub struct Jumbling<'e> {
    pub ctx: &'e Context,
    pub menu: Arc<Menu>,
    pub state: State,
}

impl<'e> Jumbling<'e> {
    pub fn new(ctx: &'e Context, menu: Arc<Menu>, state: State) -> Jumbling<'e> {
        Jumbling { ctx, menu, state }
    }

    /// The twist of letter `[cap, item]`: the menu's item transported to the cap by the retained frame, Λ_c = F_c Λ₀
    /// F_c⁻¹ (the engine's `Menu::for_cap`, computed for the one item).
    pub fn twist(&self, cap: usize, item: usize, claim: &str) -> Step<Twist> {
        let ctx = self.ctx;
        if cap >= POLES || item >= self.menu.items.len() {
            return Err(reject(claim, format!("the letter [{cap}, {item}] is outside the menu")));
        }
        let frame = ctx.kplus.matrix(ctx.kplus.frame_idx[cap], &ctx.model);
        let matrix = matmul(&matmul(frame, &self.menu.items[item].1), &transpose(frame));
        Twist::new(ctx, cap, matrix).map_err(|error| reject(claim, format!("the letter's rotation is {error}")))
    }

    /// Applies a twist exactly (no budget, no protection). Only an admissible outcome changes the state. The letter is
    /// charged to the open stop scopes first ([`crate::claims::poll`]).
    pub fn apply(&mut self, twist: &Twist, claim: &str) -> Step<Outcome> {
        crate::claims::poll(1)?;
        self.state
            .apply(self.ctx, twist, None, &[])
            .map_err(|error| reject(claim, format!("the engine stopped: {}", error.0)))
    }

    /// The state's canonical digest.
    pub fn digest(&self) -> String {
        self.state.digest(self.ctx)
    }
}

/// A `[cap, item]` letter of a menu with `items` items.
pub fn letter(value: &Value, claim: &str, items: usize) -> Step<(usize, usize)> {
    let pair = array(value, claim)?;
    if pair.len() != 2 {
        return Err(reject(claim, "a letter is [cap, item]"));
    }
    let cap = usize::try_from(int(&pair[0], claim)?).ok().filter(|&c| c < POLES);
    let item = usize::try_from(int(&pair[1], claim)?).ok().filter(|&i| i < items);
    match (cap, item) {
        (Some(cap), Some(item)) => Ok((cap, item)),
        _ => Err(reject(claim, format!("a letter's cap is 0 to {} and its item below {items}", POLES - 1))),
    }
}

/// The menu record of input `role`, bound with its name, identity and order digest.
pub fn read_menu(ctx: &Context, run: &mut Run, role: &str) -> Step<Arc<Menu>> {
    let bytes = run.input(role)?;
    let doc = json::parse(bytes).map_err(|error| reject(role, format!("the menu record is not JSON: {error}")))?;
    let menu = Menu::from_record(ctx, &doc).map_err(|error| reject(role, format!("the menu record: {error}")))?;
    run.bind(
        role,
        bytes,
        vec![("name", menu.name.clone()), ("identity", menu.identity.clone()), ("order", menu.order.clone())],
    );
    Ok(Arc::new(menu))
}

/// The start state of input `role`: a state record (`pose_table`, `pose_ids`, `digest`) or a pose table as the
/// research archive stores it (`poses`, `pose_id`, `digest`). Without the input the start is the solved state. A table
/// that does not reproduce its own stated digest is the wrong start table.
pub fn read_state(ctx: &Context, run: &mut Run, menu: &Arc<Menu>, role: &str) -> Step<State> {
    let Some(bytes) = run.optional_input(role) else {
        let state = State::new(Some(menu.clone()));
        let digest = state.digest(ctx);
        run.inputs.push(crate::report::BoundInput {
            role: role.to_owned(),
            sha256: String::new(),
            identity: vec![("solved".to_owned(), digest)],
        });
        return Ok(state);
    };
    let doc = json::parse(bytes).map_err(|error| start_mismatch(role, format!("the table is not JSON: {error}")))?;
    let state = if doc.get("pose_table").is_some() {
        State::from_record(ctx, Some(menu.clone()), &doc).map_err(|error| start_mismatch(role, error))?
    } else {
        let state = table_state(ctx, menu, &doc, role)?;
        if let Some(stated) = doc.get("digest") {
            if stated.as_str() != Some(state.digest(ctx).as_str()) {
                return Err(start_mismatch(role, "the table does not reproduce its own digest"));
            }
        }
        state
    };
    run.bind(role, bytes, vec![("digest", state.digest(ctx))]);
    Ok(state)
}

/// A research pose table: the exact pose matrices and one pose index per piece.
fn table_state(ctx: &Context, menu: &Arc<Menu>, doc: &Value, role: &str) -> Step<State> {
    let wrong = |reason: String| start_mismatch(role, reason);
    let poses =
        field(doc, "poses", role)?.as_array().ok_or_else(|| wrong("the table's poses are not a list".into()))?;
    let ids =
        field(doc, "pose_id", role)?.as_array().ok_or_else(|| wrong("the table's pose_id is not a list".into()))?;
    if ids.len() != NP {
        return Err(wrong(format!("the table names {} pieces, not {NP}", ids.len())));
    }
    let mut table = Vec::with_capacity(poses.len());
    for (index, entry) in poses.iter().enumerate() {
        let m = matrix_from_json(entry).map_err(|error| wrong(format!("pose {index}: {error}")))?;
        if !is_rotation(&m) {
            return Err(wrong(format!("pose {index} is not an exact rotation")));
        }
        table.push(match ctx.kplus.index_of_matrix(&m, &ctx.model) {
            Some(k) => Pose::K(k),
            None => Pose::M(Arc::new(m)),
        });
    }
    let mut per_piece = Vec::with_capacity(NP);
    for id in ids {
        let pose = id.as_i64().and_then(|id| usize::try_from(id).ok()).and_then(|id| table.get(id));
        per_piece.push(pose.ok_or_else(|| wrong("a piece names no pose of the table".into()))?.clone());
    }
    levels::state_of(ctx, Some(menu.clone()), &per_piece).map_err(wrong)
}

/// The observation tables of the jumbling engine (tree heights and branches).
pub fn tree_tables<'c>(ctx: &'c Context, claim: &str) -> Step<&'c Tables> {
    observe::tables(ctx).map_err(|error| reject(claim, format!("the engine's tree tables: {error}")))
}

/// The left height and left branch of a pose. The calculus of the archive's height claims holds where every right
/// height is 0, as stage 14's `tree_info` requires; a pose with another right height fails the claim.
pub fn left_facts(ctx: &Context, tables: &Tables, pose: &Pose, claim: &str) -> Step<(i64, i64)> {
    let facts = tables.facts(ctx, pose).map_err(|error| reject(claim, format!("the engine's tree facts: {error}")))?;
    if facts.dr != 0 {
        return Err(reject(claim, format!("a pose has right height {}, outside the height calculus", facts.dr)));
    }
    Ok((facts.dl, facts.bl))
}

/// Φ = Σ max(0, h − 2) over the pieces, from the left heights of the state's poses, recomputed in full.
pub fn phi(ctx: &Context, tables: &Tables, state: &State, claim: &str) -> Step<i64> {
    let mut counts = vec![0i64; state.poses().len()];
    for &id in state.pose_ids() {
        counts[id as usize] += 1;
    }
    let mut total = 0i64;
    for (pose, &n) in state.poses().iter().zip(&counts) {
        if n > 0 {
            let (h, _) = left_facts(ctx, tables, pose, claim)?;
            total += n * (h - 2).max(0);
        }
    }
    Ok(total)
}

/// A support claim's value: the number of pieces and the SHA-256 of their sorted IDs as little-endian int64.
pub fn support(pieces: &[u32]) -> (u64, String) {
    let mut sorted = pieces.to_vec();
    sorted.sort_unstable();
    let bytes: Vec<u8> = sorted.iter().flat_map(|&p| (p as i64).to_le_bytes()).collect();
    (sorted.len() as u64, c600_digest::sha256_hex(&bytes))
}

/// An exact point `[[a, b, d] × 4]`, each coordinate (a + b√5)/d in lowest terms.
pub fn point(value: &Value, claim: &str) -> Step<Vec4> {
    let entries = array(value, claim)?;
    if entries.len() != 4 {
        return Err(reject(claim, "a point has four coordinates"));
    }
    let parsed: Result<Vec<Q5>, String> = entries.iter().map(Q5::from_json_reduced).collect();
    let parsed = parsed.map_err(|error| reject(claim, error))?;
    Ok([parsed[0].clone(), parsed[1].clone(), parsed[2].clone(), parsed[3].clone()])
}

/// The menu item whose exact rotation is the inverse of item `item`'s.
pub fn inverse_item(menu: &Menu, item: usize) -> Option<usize> {
    let inverse = transpose(&menu.items.get(item)?.1);
    menu.items.iter().position(|(_, m)| *m == inverse)
}

// ---------------------------------------------------------------------------------------------- 600-cell-Full

/// A 600-cell-Full state, replayed by primitive letters through the kernel's whole-word replay, which applies each
/// primitive's permutation and uses no star cache (KN2.4 `replay`, KN2.10).
pub struct Full<'k> {
    pub kernel: &'k Kernel,
    pub state: Snapshot,
}

impl<'k> Full<'k> {
    /// Replays `letters` from the current state; the derived state replaces it.
    pub fn advance(&mut self, letters: &[i64], claim: &str) -> Step<()> {
        let mut replay = self
            .kernel
            .replay(&self.state, &ProtectedSet::none())
            .map_err(|refusal| reject(claim, format!("the kernel refused the replay: {refusal}")))?;
        for chunk in letters.chunks(4096) {
            crate::claims::poll(chunk.len() as u64)?;
            replay.push(chunk).map_err(|refusal| reject(claim, format!("the kernel refused a letter: {refusal}")))?;
        }
        match replay.finish() {
            Derivation::Derived(derived) => {
                self.state = derived.state;
                Ok(())
            }
            Derivation::Refused(refused) => Err(reject(claim, format!("the replay was refused: {}", refused.refusal))),
        }
    }
}

/// The 600-cell-Full start state of input `role`: a JSON object `{"labels": [...]}` with one label per slot. Without
/// the input the start is the solved state.
pub fn read_full_state(kernel: &Kernel, run: &mut Run, role: &str) -> Step<Snapshot> {
    let Some(bytes) = run.optional_input(role) else {
        let state = kernel.solved();
        run.inputs.push(crate::report::BoundInput {
            role: role.to_owned(),
            sha256: String::new(),
            identity: vec![("solved".to_owned(), state.digest().to_owned())],
        });
        return Ok(state);
    };
    let doc = json::parse(bytes).map_err(|error| start_mismatch(role, format!("the labels are not JSON: {error}")))?;
    let labels = field(&doc, "labels", role)
        .map_err(|_| start_mismatch(role, "the start needs labels"))?
        .as_array()
        .ok_or_else(|| start_mismatch(role, "the labels are not a list"))?;
    let labels: Option<Vec<i32>> = labels.iter().map(|v| v.as_i64().and_then(|n| i32::try_from(n).ok())).collect();
    let state = labels
        .and_then(|labels| kernel.state_from_labels(labels))
        .ok_or_else(|| start_mismatch(role, "the labels are not a state of 600-cell-Full"))?;
    run.bind(role, bytes, vec![("digest", state.digest().to_owned())]);
    Ok(state)
}

/// A primitive letter of 600-cell-Full: ±1 to ±1,200.
pub fn primitive(value: &Value, claim: &str) -> Step<i64> {
    let m = int(value, claim)?;
    if m != 0 && m.unsigned_abs() <= c600_kernel::GENERATORS as u64 {
        Ok(m)
    } else {
        Err(reject(claim, "a primitive letter is ±1 to ±1,200"))
    }
}

// ------------------------------------------------------------------------------------------------------- words

/// A literal word of either puzzle. A jumbling letter is kept as its code `items · cap + item` (for I_a, stage 13's
/// `ia16` code `60 · cap + item`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Word {
    Full(Vec<i64>),
    Jumbling { codes: Vec<u32>, items: usize },
}

impl Word {
    pub fn len(&self) -> usize {
        match self {
            Word::Full(letters) => letters.len(),
            Word::Jumbling { codes, .. } => codes.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The word's digest: for 600-cell-Full the kernel's `Digest<literal word>` (letters as little-endian int32), for
    /// the jumbling puzzle the SHA-256 of its codes as little-endian uint16, as an `ia16` word file holds them.
    pub fn digest(&self) -> String {
        let mut hash = c600_digest::Sha256::new();
        match self {
            Word::Full(letters) => letters.iter().for_each(|&m| hash.update(&(m as i32).to_le_bytes())),
            Word::Jumbling { codes, .. } => codes.iter().for_each(|&c| hash.update(&(c as u16).to_le_bytes())),
        }
        c600_digest::hex(&hash.finish())
    }
}

/// The largest expansion of DEFLATE: 258 bytes from a one-bit literal-length code and a one-bit distance code.
const DEFLATE_RATIO: usize = 1032;

/// Whether a menu's letter codes fit 16 bits: 600 caps of at most 109 items.
pub fn coded(items: usize, claim: &str) -> Step<()> {
    if items * POLES <= 1 << 16 {
        Ok(())
    } else {
        Err(reject(claim, format!("a menu of {items} items has no 16-bit letter codes")))
    }
}

/// A word: inline (`[m, ...]` for 600-cell-Full, `[[cap, item], ...]` for the jumbling puzzle), or from an input as
/// `{"input": role, "encoding": "i32le" | "ia16" | "ia16-gzip", "letters": n, "sha256": ...}`. An input word's length
/// is checked against its declared letters before anything is allocated, a gzip member may not inflate past it, and
/// the SHA-256 of the decoded bytes must be the stated one. `items` is `None` for 600-cell-Full.
pub fn read_word(run: &mut Run, value: &Value, claim: &str, items: Option<usize>) -> Step<Word> {
    if let Some(list) = value.as_array() {
        return match items {
            None => Ok(Word::Full(
                list.iter().enumerate().map(|(k, v)| primitive(v, &at(claim, k))).collect::<Step<Vec<_>>>()?,
            )),
            Some(items) => {
                coded(items, claim)?;
                let codes = list
                    .iter()
                    .enumerate()
                    .map(|(k, v)| letter(v, &at(claim, k), items).map(|(c, i)| (c * items + i) as u32))
                    .collect::<Step<Vec<_>>>()?;
                Ok(Word::Jumbling { codes, items })
            }
        };
    }
    keys(value, claim, &["input", "encoding", "letters", "sha256"], &[])?;
    let role = string(field(value, "input", claim)?, &dot(claim, "input"))?;
    let encoding = string(field(value, "encoding", claim)?, &dot(claim, "encoding"))?;
    let letters = usize::try_from(count(field(value, "letters", claim)?, &dot(claim, "letters"))?)
        .map_err(|_| reject(dot(claim, "letters"), "too many letters"))?;
    let stated = hex64(field(value, "sha256", claim)?, &dot(claim, "sha256"))?.to_owned();
    let bytes = run.input(role)?;
    let width = match (encoding, items) {
        ("i32le", None) => 4,
        ("ia16" | "ia16-gzip", Some(items)) => {
            coded(items, claim)?;
            2
        }
        _ => {
            return Err(reject(dot(claim, "encoding"), format!("the encoding {encoding} is not one of this puzzle's")))
        }
    };
    let size = letters.checked_mul(width).ok_or_else(|| reject(dot(claim, "letters"), "too many letters"))?;
    let inflated;
    let raw: &[u8] = if encoding == "ia16-gzip" {
        // DEFLATE expands at most 1,032 times (a 258-byte match in two bits per symbol, RFC 1951), so a file that
        // cannot hold the declared letters fails before anything is inflated.
        if size > bytes.len().saturating_mul(DEFLATE_RATIO) {
            return Err(reject(
                dot(claim, "letters"),
                format!("a {}-byte gzip file cannot hold {size} bytes", bytes.len()),
            ));
        }
        inflated = c600_kernel::archive::gunzip(bytes, size)
            .map_err(|error| reject(dot(claim, "letters"), format!("the word file: {}", error.0)))?;
        &inflated
    } else {
        bytes
    };
    if raw.len() != size {
        return Err(reject(dot(claim, "letters"), format!("the word holds {} bytes, not {size}", raw.len())));
    }
    let digest = c600_digest::sha256_hex(raw);
    run.bind(role, bytes, vec![("encoding", encoding.to_owned()), ("decoded_sha256", digest.clone())]);
    run.claim(&dot(claim, "sha256"), &stated, &digest)?;
    match items {
        None => {
            let letters: Vec<i64> =
                raw.chunks_exact(4).map(|b| i32::from_le_bytes([b[0], b[1], b[2], b[3]]) as i64).collect();
            if let Some(k) = letters.iter().position(|&m| m == 0 || m.unsigned_abs() > c600_kernel::GENERATORS as u64) {
                return Err(reject(at(claim, k), "a primitive letter is ±1 to ±1,200"));
            }
            Ok(Word::Full(letters))
        }
        Some(items) => {
            let codes: Vec<u32> = raw.chunks_exact(2).map(|b| u16::from_le_bytes([b[0], b[1]]) as u32).collect();
            if let Some(k) = codes.iter().position(|&c| c as usize >= items * POLES) {
                return Err(reject(at(claim, k), "a letter code names no cap and item"));
            }
            Ok(Word::Jumbling { codes, items })
        }
    }
}

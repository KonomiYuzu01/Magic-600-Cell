//! The identity tower's kernel rows on 600-cell-Full (T2 draft until the IR freeze, PHS.49; plan section 2's
//! operations), served by the bridge over `c600-kernel` and the command layer's value store ([`c600_command::values`]).
//! Each row is a [`Request::Kernel`] of its own, so a host can grant `derive` apart from the reads (BND.57); the bridge
//! grants them all. 600-cell-Full holds the labels and the position only (KN1.7): the interpreter refuses every other
//! level with `KN1:input.level_not_held`, and every goal but the sticker test with `KN1:input.identity_mismatch`,
//! before the bridge is asked.
//!
//! # Rows
//!
//! - `snapshot<Full>()`: the layer's head as a state handle, kept in the store as a root under its revision (session,
//!   head event and digest). With Jumble on it is refused with `KN5:input.retained_tool_needs_jumble_off` (LG2.8).
//! - `project(s, L)`: `total` of the state's level value as a handle; a 600-cell-Full state is total at both levels.
//! - `digest(v)`: the level value's `Digest<Full, L>`: `Digest<labels>` (the kernel's state hash), or for the position
//!   the SHA-256 of each piece's position as little-endian u32 in piece order.
//! - `compare(a, b, L)`: `total` of `yes` when both states' digests at the level are equal and `no` otherwise, with the
//!   evidence `{ level, a, b }` (ONE.25).
//! - `derive(s, w)`: the word's literal letters derived from the state by the kernel under the session's protection
//!   set and policy, as the command layer derives a commit (Net or Strict). `derived(State<Full>)` is kept in the store
//!   under its derivation ([`ValueKey::Derived`]); a refusal is `refused({ prefix, outcome, code })`, with the first
//!   refused prefix under Strict and the whole word's length under Net, which judges only the whole word (draft), and
//!   no value is made.
//! - `import(t, r)`: not granted (`LG3:capability.not_granted`): a raw pose table is the jumbling puzzle's (KN2.14).
//! - `reachability(s)`: `witnessed`, since every 600-cell-Full word is legal and the root is a revision the journal
//!   reached: the evidence names the revision, or the base's digest and the word's digest.
//! - `check(sticker_test, s)`: `yes(Holds)` when every slot has its home label, whose subject is `Digest<labels>`;
//!   otherwise `no` with the count of slots that differ.
//!
//! # Tariffs (draft; kernel cost in slots, as the bridge's other rows count it)
//!
//! `snapshot` and `reachability` 0; `project`, `digest` and `check` one pass over the 259,800 slots; `compare` two;
//! `derive` the `replay` row's tariff for its letters (two passes over the slots and two per letter, for the net map
//! and the prefixes). The result bound of every row is [`RESULT_BOUND`]. A hit in the store, a miss and a
//! recomputation after an eviction are charged alike, and each handle declares its value's logical size (KN7.9,
//! DET.24): a state and its labels 1,039,200 bytes, its position 708,480, evidence its canonical JSON's length.
//!
//! A handle of an ended epoch is refused at the tariff with `KN2:stale.epoch_changed`, at the cost of the row's step.

use std::sync::Arc;

use c600_command::values::{
    protection_key, BaseKey, Letters, Level as LevelKey, Lookup, Named, Recipe, Source, Stored, ValueKey,
};
use c600_command::{code, Store};
use c600_ir::{Goal, KernelOp, Level, Puzzle};
use c600_kernel::json::{self, Value as Json};
use c600_kernel::{Derivation, ProtectedSet, ProtectionMode, Snapshot, PIECES, SLOTS};
use c600_run::interp::RowTariff;
use c600_run::meter::code::NOT_GRANTED;
use c600_run::tower::{Digest, Handle, HandleTy, Holds, SumTy};
use c600_run::value::Value;

use super::Bridge;
use crate::kernel_rows::replay_tariff;

/// The result bound of every kernel row of the tower on 600-cell-Full (a draft): 2 MiB of logical size, above a state's
/// 1,039,200 bytes.
pub const RESULT_BOUND: u128 = 1 << 21;

/// The logical size of a 600-cell-Full state and of its labels (KN7.9): 4 bytes for each of the 259,800 labels.
pub const STATE_SIZE: u128 = 4 * SLOTS as u128;

/// The logical size of a 600-cell-Full state's position: 4 bytes for each of the 177,120 pieces.
pub const POSITION_SIZE: u128 = 4 * PIECES as u128;

/// The level key the store keeps of an IR level.
fn level_key(level: Level) -> LevelKey {
    LevelKey::parse(level.name()).expect("the IR names the jumbling engine's levels")
}

/// `Digest<literal word>` of 600-cell-Full's letters, the kernel's convention: the SHA-256 of the letters as
/// little-endian int32.
pub fn word_digest(letters: &[i32]) -> String {
    let bytes: Vec<u8> = letters.iter().flat_map(|letter| letter.to_le_bytes()).collect();
    c600_digest::sha256_hex(&bytes)
}

/// The tariff of a kernel row, or the refusal that costs the row's step only.
pub(super) fn tariff<S: Store>(
    bridge: &mut Bridge<'_, S>,
    op: KernelOp,
    args: &[Value],
) -> Result<RowTariff, &'static str> {
    for arg in args {
        if let Value::Handle(handle) = arg {
            bridge.layer.values().named(handle.id())?;
        }
    }
    let slots = SLOTS as u128;
    let kernel_cost = match op {
        KernelOp::Snapshot(Puzzle::Full) => {
            if bridge.layer.jumbled().is_some() {
                return Err(code::RETAINED_TOOL_NEEDS_JUMBLE_OFF);
            }
            0
        }
        KernelOp::Snapshot(Puzzle::Jumbling) | KernelOp::Import => return Err(NOT_GRANTED),
        KernelOp::Project(_) | KernelOp::Digest | KernelOp::Check(_) => slots,
        KernelOp::Compare(_) => 2 * slots,
        KernelOp::Derive => match args {
            [_, Value::Word(word)] => replay_tariff(word.len()).kernel_cost,
            _ => return Err(code::ARGUMENT_FORM),
        },
        KernelOp::Reachability => 0,
    };
    Ok(RowTariff { kernel_cost, result_bound: RESULT_BOUND })
}

/// Performs an admitted kernel row.
pub(super) fn perform<S: Store>(
    bridge: &mut Bridge<'_, S>,
    op: KernelOp,
    args: &[Value],
) -> Result<Value, &'static str> {
    match (op, args) {
        (KernelOp::Snapshot(Puzzle::Full), []) => {
            let revision = bridge.layer.revision();
            let key = BaseKey::Revision {
                session: revision.session.clone(),
                head: revision.head.clone(),
                digest: revision.digest.clone(),
            };
            let root = Arc::new(Stored::Full(bridge.layer.snapshot().clone()));
            let values = bridge.layer.values_mut();
            values.insert_root(key.clone(), root);
            let id = values.handle(Named::Root(key))?;
            Ok(Value::handle(Handle::new(HandleTy::State(Puzzle::Full), id, STATE_SIZE)))
        }
        (KernelOp::Project(level), [s]) => {
            let of = named(bridge, s)?;
            resolve(bridge, &of)?;
            let size = if level == Level::Position { POSITION_SIZE } else { STATE_SIZE };
            let named = Named::Level { of: Box::new(of), level: level_key(level), size };
            let id = bridge.layer.values_mut().handle(named)?;
            let value = Value::handle(Handle::new(HandleTy::LevelValue(Puzzle::Full, level), id, size));
            Ok(Value::sum(SumTy::Partial, "total", vec![value]))
        }
        (KernelOp::Digest, [v]) => {
            let Named::Level { of, level, .. } = named(bridge, v)? else { return Err(code::ARGUMENT_FORM) };
            let level = Level::of_name(level.name()).ok_or(code::ARGUMENT_FORM)?;
            let state = full(&resolve(bridge, &of)?)?;
            let hex = level_digest(bridge, &state, level)?;
            Value::digest(Puzzle::Full, level, &hex).ok_or(code::STATE_HASH_MISMATCH)
        }
        (KernelOp::Compare(level), [a, b]) => {
            let (a, b) = (named(bridge, a)?, named(bridge, b)?);
            let (a, b) = (full(&resolve(bridge, &a)?)?, full(&resolve(bridge, &b)?)?);
            let (da, db) = (level_digest(bridge, &a, level)?, level_digest(bridge, &b, level)?);
            let evidence = json::object([
                ("level", Json::String(level.name().into())),
                ("a", Json::String(da.clone())),
                ("b", Json::String(db.clone())),
            ]);
            let evidence = evidence_handle(bridge, evidence)?;
            let ctor = if da == db { "yes" } else { "no" };
            let verdict = Value::sum(SumTy::Verdict(Puzzle::Full, level, None), ctor, vec![evidence]);
            Ok(Value::sum(SumTy::Partial, "total", vec![verdict]))
        }
        (KernelOp::Derive, [s, Value::Word(word)]) => derive(bridge, s, word),
        (KernelOp::Reachability, [s]) => {
            let evidence = match named(bridge, s)? {
                Named::Root(BaseKey::Revision { session, head, digest }) => json::object([
                    ("from", Json::String("revision".into())),
                    ("session", Json::String(session)),
                    ("head", Json::String(head)),
                    ("digest", Json::String(digest)),
                ]),
                Named::Value(key) => {
                    let Some(Recipe::Derive { letters: Letters::Full(letters), .. }) =
                        bridge.layer.values().recipe(&key).cloned()
                    else {
                        return Err(code::ARGUMENT_FORM);
                    };
                    let base = match &key {
                        ValueKey::Derived { base: BaseKey::Revision { digest, .. }, .. }
                        | ValueKey::Derived { base: BaseKey::Digest(digest), .. } => digest.clone(),
                        ValueKey::Imported { .. } => return Err(code::ARGUMENT_FORM),
                    };
                    json::object([
                        ("base", Json::String(base)),
                        ("word", Json::String(word_digest(&letters))),
                        ("letters", Json::Int(letters.len() as i64)),
                    ])
                }
                _ => return Err(code::ARGUMENT_FORM),
            };
            let evidence = evidence_handle(bridge, evidence)?;
            Ok(Value::sum(SumTy::Reachability(Puzzle::Full), "witnessed", vec![evidence]))
        }
        (KernelOp::Check(Goal::StickerTest), [s]) => {
            let state = full(&resolve(bridge, &named(bridge, s)?)?)?;
            let displaced = state.labels().iter().enumerate().filter(|(slot, &label)| *slot as i64 != label as i64);
            let displaced = displaced.count();
            let ty = SumTy::Verdict(Puzzle::Full, Level::Labels, Some(Goal::StickerTest));
            if displaced == 0 {
                let subject =
                    Digest::new(Puzzle::Full, Level::Labels, state.digest()).ok_or(code::STATE_HASH_MISMATCH)?;
                let holds = Holds::new(Goal::StickerTest, subject, None).ok_or(code::STATE_HASH_MISMATCH)?;
                return Ok(Value::sum(ty, "yes", vec![Value::holds(holds)]));
            }
            let fact =
                json::object([("fact", Json::String("labels_differ".into())), ("slots", Json::Int(displaced as i64))]);
            Ok(Value::sum(ty, "no", vec![evidence_handle(bridge, fact)?]))
        }
        _ => Err(code::ARGUMENT_FORM),
    }
}

/// What a handle argument names in this epoch's store.
fn named<S: Store>(bridge: &Bridge<'_, S>, arg: &Value) -> Result<Named, &'static str> {
    match arg {
        Value::Handle(handle) => bridge.layer.values().named(handle.id()).cloned(),
        _ => Err(code::ARGUMENT_FORM),
    }
}

/// A 600-cell-Full state the store holds.
fn full(stored: &Arc<Stored>) -> Result<Snapshot, &'static str> {
    match &**stored {
        Stored::Full(state) => Ok(state.clone()),
        _ => Err(code::ARGUMENT_FORM),
    }
}

/// A handle of evidence, whose logical size is its canonical JSON's length (draft).
fn evidence_handle<S: Store>(bridge: &mut Bridge<'_, S>, evidence: Json) -> Result<Value, &'static str> {
    let size = evidence.canonical().len() as u128;
    let id = bridge.layer.values_mut().handle(Named::Evidence(Arc::new(evidence)))?;
    Ok(Value::handle(Handle::new(HandleTy::Evidence, id, size)))
}

/// The digest of a state at a level 600-cell-Full holds.
fn level_digest<S: Store>(bridge: &Bridge<'_, S>, state: &Snapshot, level: Level) -> Result<String, &'static str> {
    match level {
        Level::Labels => Ok(state.digest().to_owned()),
        Level::Position => {
            let pieces: Vec<i64> = (0..PIECES as i64).collect();
            let rows = bridge.layer.kernel().positions(state, &pieces).map_err(|refusal| refusal.code)?;
            let mut bytes = Vec::with_capacity(4 * PIECES);
            for row in rows {
                let place = row.position.ok_or(code::STATE_HASH_MISMATCH)?;
                bytes.extend_from_slice(&(place as u32).to_le_bytes());
            }
            Ok(c600_digest::sha256_hex(&bytes))
        }
        _ => Err(c600_run::interp::code::LEVEL_NOT_HELD),
    }
}

/// The state a handle target names: a root, or a stored value, recomputed from its recipe when it was evicted.
fn resolve<S: Store>(bridge: &mut Bridge<'_, S>, named: &Named) -> Result<Arc<Stored>, &'static str> {
    match named {
        Named::Root(key) => bridge.layer.values().root(key).ok_or(code::ARGUMENT_FORM),
        Named::Value(key) => match bridge.layer.values_mut().lookup(key) {
            Lookup::Hit(value) => Ok(value),
            Lookup::Evicted(recipe) => {
                let (value, digest) = recompute(bridge, &recipe)?;
                bridge.layer.values_mut().insert(key.clone(), recipe, value, digest)
            }
            Lookup::Miss => Err(code::ARGUMENT_FORM),
        },
        _ => Err(code::ARGUMENT_FORM),
    }
}

/// A stored derivation's base: a root of this epoch, or a revision the layer's head still is (ONE.17: a different
/// epoch does not invalidate it), or another entry. A base it cannot resolve is `KN2:stale.value_unavailable`.
fn base<S: Store>(bridge: &mut Bridge<'_, S>, source: &Source) -> Result<Arc<Stored>, &'static str> {
    match source {
        Source::Root(key) => {
            if let Some(root) = bridge.layer.values().root(key) {
                return Ok(root);
            }
            let revision = bridge.layer.revision();
            let current = matches!(key, BaseKey::Revision { session, head, digest }
                if *session == revision.session && *head == revision.head && *digest == revision.digest);
            if !current || bridge.layer.jumbled().is_some() {
                return Err(VALUE_UNAVAILABLE);
            }
            let root = Arc::new(Stored::Full(bridge.layer.snapshot().clone()));
            Ok(bridge.layer.values_mut().insert_root(key.clone(), root))
        }
        Source::Value { key, recipe } => stored(bridge, key, recipe),
    }
}

/// The stored value of `key`: a hit, or `recipe` recomputed after an eviction or in a store that lacks the entry, such
/// as a later epoch's.
fn stored<S: Store>(bridge: &mut Bridge<'_, S>, key: &ValueKey, recipe: &Recipe) -> Result<Arc<Stored>, &'static str> {
    if let Lookup::Hit(value) = bridge.layer.values_mut().lookup(key) {
        return Ok(value);
    }
    let (value, digest) = recompute(bridge, recipe)?;
    bridge.layer.values_mut().insert(key.clone(), recipe.clone(), value, digest)
}

/// `KN2:stale.value_unavailable`: a stored derivation whose base no longer resolves.
pub const VALUE_UNAVAILABLE: &str = "KN2:stale.value_unavailable";

/// The value a stored derivation makes, with its digest: what a miss computes and an eviction recomputes.
pub fn recompute<S: Store>(bridge: &mut Bridge<'_, S>, recipe: &Recipe) -> Result<(Arc<Stored>, String), &'static str> {
    let Recipe::Derive { base: source, letters: Letters::Full(letters), policy, protect } = recipe else {
        return Err(code::ARGUMENT_FORM);
    };
    let start = full(&base(bridge, source)?)?;
    match derived(bridge, &start, letters, *policy, protect)? {
        Ok(state) => {
            let digest = state.digest().to_owned();
            Ok((Arc::new(Stored::Full(state)), digest))
        }
        Err(_) => Err(code::STATE_HASH_MISMATCH),
    }
}

/// The kernel's derivation of `letters` from `start` under `policy` and the orbits `protect`: the derived state, or
/// the refusal's first refused prefix (the word's length under Net) and code.
fn derived<S: Store>(
    bridge: &Bridge<'_, S>,
    start: &Snapshot,
    letters: &[i32],
    policy: ProtectionMode,
    protect: &[usize],
) -> Result<Result<Snapshot, (u64, &'static str)>, &'static str> {
    let kernel = bridge.layer.kernel();
    let word: Vec<i64> = letters.iter().map(|&l| i64::from(l)).collect();
    let recipe = kernel.word(&word).map_err(|refusal| refusal.code)?;
    let orbits: Vec<i64> = protect.iter().map(|&o| o as i64).collect();
    let set: ProtectedSet = kernel.protected_set(&orbits).map_err(|refusal| refusal.code)?;
    let derivation = match policy {
        ProtectionMode::Net => kernel.derive(start, &recipe, &set),
        ProtectionMode::Strict => kernel.derive_strict(start, &recipe, &set),
    };
    let error = |error: c600_kernel::Error| match error {
        c600_kernel::Error::Refusal(refusal) => refusal.code,
        c600_kernel::Error::Certificate(_) => code::STATE_HASH_MISMATCH,
    };
    Ok(match derivation.map_err(error)? {
        Derivation::Derived(d) => Ok(d.state),
        Derivation::Refused(r) => Err((r.first_refused_prefix.unwrap_or(letters.len() as u64), r.refusal.code)),
    })
}

/// `derive(s, w)`: a hit, a miss or a recomputation of the stored value, or a refusal as a result.
fn derive<S: Store>(bridge: &mut Bridge<'_, S>, s: &Value, word: &Arc<[i32]>) -> Result<Value, &'static str> {
    let of = named(bridge, s)?;
    let (source, base_key) = match &of {
        Named::Root(key) => (Source::Root(key.clone()), key.clone()),
        Named::Value(key) => {
            resolve(bridge, &of)?;
            let digest = bridge.layer.values().digest(key).ok_or(code::ARGUMENT_FORM)?.to_owned();
            let recipe = Arc::new(bridge.layer.values().recipe(key).ok_or(code::ARGUMENT_FORM)?.clone());
            (Source::Value { key: key.clone(), recipe }, BaseKey::Digest(digest))
        }
        _ => return Err(code::ARGUMENT_FORM),
    };
    let policy = bridge.layer.policy();
    let protect: Vec<usize> = bridge.layer.protected().orbits().to_vec();
    let key =
        ValueKey::Derived { base: base_key, word: word_digest(word), protection: protection_key(policy, &protect) };
    let recipe = Recipe::Derive { base: source.clone(), letters: Letters::Full(word.clone()), policy, protect };
    let ty = SumTy::Derivation(Puzzle::Full);
    let held = match bridge.layer.values_mut().lookup(&key) {
        Lookup::Hit(value) => Some(value),
        Lookup::Evicted(_) => Some(resolve(bridge, &Named::Value(key.clone()))?),
        Lookup::Miss => None,
    };
    if held.is_none() {
        let start = full(&base(bridge, &source)?)?;
        let Recipe::Derive { letters: Letters::Full(letters), policy, protect, .. } = &recipe else { unreachable!() };
        match derived(bridge, &start, letters, *policy, protect)? {
            Ok(state) => {
                let digest = state.digest().to_owned();
                bridge.layer.values_mut().insert(key.clone(), recipe, Arc::new(Stored::Full(state)), digest)?;
            }
            Err((prefix, code)) => {
                let refused = Value::record(vec![
                    ("prefix", Value::int(prefix)),
                    ("outcome", Value::text("protected")),
                    ("code", Value::text(code)),
                ]);
                return Ok(Value::sum(ty, "refused", vec![refused]));
            }
        }
    }
    let id = bridge.layer.values_mut().handle(Named::Value(key))?;
    let state = Value::handle(Handle::new(HandleTy::State(Puzzle::Full), id, STATE_SIZE));
    Ok(Value::sum(ty, "derived", vec![state]))
}

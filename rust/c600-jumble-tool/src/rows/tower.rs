//! The identity tower's kernel rows on the jumbling puzzle (T2 draft until the IR freeze, PHS.49; plan section 2's
//! operations), served by the rows' bridge over M8's `levels` module and a value store ([`c600_command::values`]): the
//! command layer's at its head ([`Rows::tower_at_head`]), or the bridge's own for a base apart from a layer. Each row
//! is a [`Request::Kernel`] of its own, so a host can grant `derive` apart from the reads (BND.57); the bridge grants
//! them all. The jumbling puzzle holds every level (KN1.7), but this edition keys no sticker class (KN1.8): a
//! projection or a comparison at it is refused at the tariff with `KN1:input.level_not_held`.
//!
//! # Rows
//!
//! - `snapshot<Jumbling>()`: the rows' base as a state handle, kept in the store as a root under the layer's revision
//!   (session, head event and digest) at a layer's head, or under its pose digest apart from one. Jumble off refuses it
//!   as every row is refused there; a state with no witnessed claim ([`Rows::at_state`]) has no snapshot to give.
//! - `project(s, L)`: KN1.9's projection (`levels::project`): `total` of the level value as a handle, `partial` of the
//!   covered part with the off count, the uncovered slots or chambers and the collisions, or `undefined` with the
//!   reason; never `unknown` (LG2.3, LG2.9).
//! - `digest(v)`: the level value's digest (`LevelValue::digest`); of a covered part, that part's digest.
//! - `compare(a, b, L)`: ONE.25's comparison (`levels::compare`): `total` of the verdict, `partial` of the verdict on
//!   what both cover with the missing counts of both sides added (draft), or `undefined`. A refusal of the identity
//!   blocks is its code.
//! - `derive(s, w)`: a menu word's literal letters derived from the state (`levels::derive`) under the state's own
//!   identity block, the protection policy and the pose-protected pieces in force (at a layer's head, the pieces that
//!   carry the protected orbits, as a pose move judges them); a word of another alphabet is
//!   `LB1:input.incompatible_alphabet`. `derived(State<Jumbling>)` is kept in the store under its derivation; a
//!   refusal is `refused({ prefix, outcome, code })`, with the first refused prefix, the whole word's length for a Net
//!   protection refusal (draft), and no value is made.
//! - `import(t, r)`: KN2.14 (`levels::import`): the raw table read under the base's menu with the claim `r`, which must
//!   be open or promised (`KN5:input.argument_form` at the tariff otherwise); the value is kept under the digests of
//!   the table and the claim.
//! - `reachability(s)`: the state's claim (KN2.13), its kind with its evidence.
//! - `check(g, s)`: the goal at its own level (LG2.10), `yes` with the goal's certificate, whose subject is the
//!   state's digest at that level: the exact pose (every piece at its solved pose); the sticker test
//!   (`levels::sticker_test`); the lattice placement (no piece off the lattice); the low height (the state's
//!   `max_left_height` at most the bound); the retained checkpoint (on the lattice, every centre in its own chamber,
//!   and the state's own claim witnessed by a retained word: the bridge reads the claim and builds no witness, so
//!   without one the verdict is unknown); the label checkpoint (as the `label_checkpoint` row answers it, ν from the
//!   ν row when needed).
//! - A snapshot read declared `PoseTable` of a table the host gave the bridge ([`Rows::with_pose_table`]) gives that
//!   raw table as a handle (draft): data only, never a state (LG2.7).
//!
//! # Tariffs (draft; one unit per piece for each pass over a state, as the bridge's other rows)
//!
//! `snapshot`, `reachability` and a pose table's read 0; `project`, `digest`, `import` and `check` one pass (the label
//! checkpoint two, the labels and ν, as its row); `compare` two; `derive` one for each letter. The result bound of
//! every row is [`RESULT_BOUND`]. A hit in the store, a miss and a recomputation after an eviction are charged alike,
//! and each handle declares its value's logical size (KN7.9, DET.24, [`state_size`], [`level_size`]); evidence and a
//! raw table declare their canonical JSON's length.
//!
//! A handle of an ended epoch is refused at the tariff with `KN2:stale.epoch_changed`, at the cost of the row's step.

use std::collections::BTreeSet;
use std::sync::Arc;

use c600_command::values::{
    protection_key, BaseKey, Letters, Level as LevelKey, Lookup, Named, Recipe, Source, Stored, ValueKey,
};
use c600_command::{code, CommandLayer, Store, TowerAtHead, ValueStore};
use c600_ir::{Goal, KernelOp, Level, Puzzle, Ty};
use c600_jumble::codes::{IDENTITY_MISMATCH, MENU_WORD_NEEDS_JUMBLE_ON};
use c600_jumble::levels::{
    self, Base, Comparison, Derivation, LevelValue, Missing, Partial, Projection, ReachKind, Reachability, Verdict,
};
use c600_jumble::model::NP;
use c600_jumble::observe::heights;
use c600_jumble::state::{Pose, State};
use c600_jumble::{nu, Context};
use c600_kernel::json::Value as Json;
use c600_run::interp::{code::LEVEL_NOT_HELD, Request, RowTariff};
use c600_run::meter::code::NOT_GRANTED;
use c600_run::tower::{Digest, Handle, HandleTy, Holds, SumTy};
use c600_run::value::{MenuWord, Value};

use super::{engine, At, Rows, RESULT_BOUND};
use crate::word::{self, menu_identity, INCOMPATIBLE_ALPHABET};

/// `KN2:stale.value_unavailable`: a stored derivation whose base no longer resolves.
pub const VALUE_UNAVAILABLE: &str = "KN2:stale.value_unavailable";

/// The tower's part of the rows' bridge: its value store, the base's root key, the protection in force and the raw
/// tables the host gave.
pub(super) struct Tower<'a> {
    own: ValueStore,
    layer: Option<&'a mut ValueStore>,
    root: Option<BaseKey>,
    policy: c600_kernel::ProtectionMode,
    protect: Vec<u32>,
    tables: Vec<(String, Arc<Json>)>,
}

impl Default for Tower<'_> {
    fn default() -> Self {
        Tower {
            own: ValueStore::default(),
            layer: None,
            root: None,
            policy: c600_kernel::ProtectionMode::Net,
            protect: Vec::new(),
            tables: Vec::new(),
        }
    }
}

impl Tower<'_> {
    fn values(&mut self) -> &mut ValueStore {
        match &mut self.layer {
            Some(values) => values,
            None => &mut self.own,
        }
    }

    fn values_ref(&self) -> &ValueStore {
        match &self.layer {
            Some(values) => values,
            None => &self.own,
        }
    }
}

impl<'a> Rows<'a> {
    /// The rows at `layer`'s head with Jumble on, as [`Rows::at_head`] reads them, with the tower's kernel rows on the
    /// layer's value store, the head's revision as the root's key and the layer's protection policy and pose-protected
    /// pieces (T2 draft).
    pub fn tower_at_head<S: Store>(ctx: &'a Context, layer: &'a mut CommandLayer<S>) -> Rows<'a> {
        let TowerAtHead { head, revision, policy, protect, values } = layer.tower_at_head();
        let at = match head {
            None => At::Refused(MENU_WORD_NEEDS_JUMBLE_ON),
            Some(head) if ctx.identity(head.state().menu()) != *head.identity() => At::Refused(IDENTITY_MISMATCH),
            Some(head) => At::Base(Base::Derived(head)),
        };
        let root = BaseKey::Revision {
            session: revision.session.clone(),
            head: revision.head.clone(),
            digest: revision.digest.clone(),
        };
        let mut rows = Rows::new(ctx, at);
        rows.tower.layer = Some(values);
        rows.tower.root = Some(root);
        rows.tower.policy = policy;
        rows.tower.protect = protect;
        rows
    }

    /// These rows with the raw pose table `record` under the name `name`, which a snapshot read declared `PoseTable`
    /// reads (T2 draft).
    pub fn with_pose_table(mut self, name: &str, record: Json) -> Rows<'a> {
        self.tower.tables.push((name.to_owned(), Arc::new(record)));
        self
    }

    /// These rows with the protection policy `policy` and the pose-protected pieces `pieces` for `derive` (T2 draft);
    /// apart from a layer the rows derive under Net with no protected piece.
    pub fn with_protection(mut self, policy: c600_kernel::ProtectionMode, pieces: Vec<u32>) -> Rows<'a> {
        self.tower.policy = policy;
        self.tower.protect = pieces;
        self
    }

    /// The value store the tower's rows use (T2 draft): the layer's at its head, or the bridge's own.
    pub fn values(&mut self) -> &mut ValueStore {
        self.tower.values()
    }

    /// The digest of the value that a stored derivation `recipe` makes, recomputed in this store (T2 draft): its base
    /// resolves while it is the rows' base, whatever epoch the derivation was stored in (ONE.17, ONE.21).
    pub fn recompute(&mut self, recipe: &Recipe) -> Result<String, &'static str> {
        recompute(self, recipe).map(|(_, digest)| digest)
    }
}

/// The tariff of a tower row or a pose table's read; `None` for another request, which the rows' bridge answers.
pub(super) fn tariff(rows: &mut Rows<'_>, request: &Request) -> Option<Result<RowTariff, &'static str>> {
    match *request {
        Request::Kernel { op, args, .. } => Some(kernel_tariff(rows, op, args)),
        Request::Snapshot { query, ty } if ty.ty == Ty::PoseTable => Some(match table(rows, query) {
            Some(_) => Ok(RowTariff { kernel_cost: 0, result_bound: RESULT_BOUND }),
            None => Err(NOT_GRANTED),
        }),
        _ => None,
    }
}

/// Performs an admitted tower row or pose table's read; `None` for another request.
pub(super) fn perform(rows: &mut Rows<'_>, request: &Request) -> Option<Result<Value, &'static str>> {
    match *request {
        Request::Kernel { op, args, .. } => Some(kernel(rows, op, args)),
        Request::Snapshot { query, ty } if ty.ty == Ty::PoseTable => Some((|| {
            let record = table(rows, query).ok_or(NOT_GRANTED)?;
            let size = record.canonical().len() as u128;
            let id = rows.tower.values().handle(Named::PoseTable(record))?;
            Ok(Value::handle(Handle::new(HandleTy::PoseTable, id, size)))
        })()),
        _ => None,
    }
}

fn table(rows: &Rows<'_>, name: &str) -> Option<Arc<Json>> {
    rows.tower.tables.iter().find(|(n, _)| n == name).map(|(_, record)| record.clone())
}

/// Every handle among `args`, a claim's evidence included.
fn handles(args: &[Value]) -> Vec<u64> {
    let mut ids = Vec::new();
    for arg in args {
        match arg {
            Value::Handle(handle) => ids.push(handle.id()),
            Value::Sum(sum) => ids.extend(handles(sum.payload())),
            _ => {}
        }
    }
    ids
}

fn kernel_tariff(rows: &mut Rows<'_>, op: KernelOp, args: &[Value]) -> Result<RowTariff, &'static str> {
    if let At::Refused(code) = rows.at {
        return Err(code);
    }
    for id in handles(args) {
        rows.tower.values_ref().named(id)?;
    }
    let passes: u128 = match op {
        KernelOp::Snapshot(Puzzle::Jumbling) => match rows.at {
            At::Base(_) => 0,
            _ => return Err(NOT_GRANTED),
        },
        KernelOp::Snapshot(Puzzle::Full) => return Err(NOT_GRANTED),
        KernelOp::Project(Level::StickerClass) | KernelOp::Compare(Level::StickerClass) => return Err(LEVEL_NOT_HELD),
        KernelOp::Project(_) | KernelOp::Digest => 1,
        KernelOp::Compare(_) => 2,
        KernelOp::Derive => match args {
            [_, Value::MenuWord(word)] => word.letters().len() as u128,
            _ => return Err(code::ARGUMENT_FORM),
        },
        KernelOp::Import => match args {
            [_, Value::Sum(claim)] if matches!(claim.ctor(), "open" | "promised") => 1,
            _ => return Err(code::ARGUMENT_FORM),
        },
        KernelOp::Reachability => 0,
        KernelOp::Check(Goal::LabelCheckpoint) => 2,
        KernelOp::Check(_) => 1,
    };
    Ok(RowTariff { kernel_cost: (NP as u128).saturating_mul(passes), result_bound: RESULT_BOUND })
}

/// The level key the store keeps of an IR level.
fn level_key(level: Level) -> LevelKey {
    LevelKey::parse(level.name()).expect("the IR names the jumbling engine's levels")
}

/// The IR level of the engine's.
fn ir_level(level: LevelKey) -> Level {
    Level::of_name(level.name()).expect("the engine names the IR's levels")
}

/// KN7.9's logical size of a jumbling state (draft): 4 bytes for each piece's pose ID, plus each distinct pose in use,
/// 2 bytes for a lattice pose (its K⁺ index) and the length of its compact exact matrix text for another.
pub fn state_size(state: &State) -> u128 {
    let used: BTreeSet<u32> = state.pose_ids().iter().copied().collect();
    let entries: u128 = used
        .iter()
        .map(|&id| match &state.poses()[id as usize] {
            Pose::K(_) => 2,
            Pose::M(m) => c600_jumble::q5::matrix_compact(m).len() as u128,
        })
        .sum();
    4 * NP as u128 + entries
}

/// KN7.9's logical size of a level value (draft), the byte length of its arrays in its digest's convention: the pose
/// digest's 32 bytes; 4 bytes per label or chamber of a total value and 8 per covered slot or piece of a partial one;
/// the key lines' text of the region class and the coset; 32 bytes per piece of tree data; the shape's lines.
pub fn level_size(value: &LevelValue) -> u128 {
    let lines = |texts: Vec<String>| texts.iter().map(|t| t.len() as u128 + 1).sum::<u128>();
    match value {
        LevelValue::Poses { .. } => 32,
        LevelValue::Labels(labels) if labels.slots.len() == c600_jumble::model::NS => 4 * labels.labels.len() as u128,
        LevelValue::Labels(labels) => 8 * labels.labels.len() as u128,
        LevelValue::Position(positions) if positions.pieces.len() == NP => 4 * positions.chambers.len() as u128,
        LevelValue::Position(positions) => 8 * positions.chambers.len() as u128,
        LevelValue::RegionClass(keys) => lines(keys.iter().map(|k| k.text()).collect()),
        LevelValue::Coset(domains) => lines(domains.iter().map(|d| d.to_string()).collect()),
        LevelValue::TreeData(rows) => 32 * rows.len() as u128,
        LevelValue::Shape(shape) => lines(shape.regions.iter().map(|(k, n)| format!("{}\t{n}", k.text())).collect()),
    }
}

fn named(rows: &Rows<'_>, arg: &Value) -> Result<Named, &'static str> {
    match arg {
        Value::Handle(handle) => rows.tower.values_ref().named(handle.id()).cloned(),
        _ => Err(code::ARGUMENT_FORM),
    }
}

fn evidence(rows: &mut Rows<'_>, evidence: Json) -> Result<Value, &'static str> {
    let size = evidence.canonical().len() as u128;
    let id = rows.tower.values().handle(Named::Evidence(Arc::new(evidence)))?;
    Ok(Value::handle(Handle::new(HandleTy::Evidence, id, size)))
}

fn fact(name: &str, more: Vec<(&str, Json)>) -> Json {
    let mut fields = vec![("fact".to_owned(), Json::String(name.into()))];
    fields.extend(more.into_iter().map(|(k, v)| (k.to_owned(), v)));
    Json::Object(fields.into_iter().collect())
}

/// The state and claim a stored jumbling value holds.
fn parts(stored: &Stored) -> Result<(&State, &Reachability), &'static str> {
    match stored {
        Stored::JumblingSnapshot(snapshot) => Ok((snapshot.state(), snapshot.reachability())),
        Stored::Jumbling(derived) => Ok((derived.state(), derived.reachability())),
        Stored::Full(_) => Err(code::ARGUMENT_FORM),
    }
}

/// The rows' base as a stored root.
fn base_root(rows: &Rows<'_>) -> Result<Stored, &'static str> {
    match rows.at {
        At::Base(Base::Snapshot(snapshot)) => Ok(Stored::JumblingSnapshot(snapshot.clone())),
        At::Base(Base::Derived(derived)) => Ok(Stored::Jumbling(derived.clone())),
        At::State(_) => Err(NOT_GRANTED),
        At::Refused(code) => Err(code),
    }
}

/// The root key of the rows' base: the layer's revision, or the base's pose digest.
fn root_key(rows: &Rows<'_>) -> Result<BaseKey, &'static str> {
    if let Some(key) = &rows.tower.root {
        return Ok(key.clone());
    }
    Ok(BaseKey::Digest(rows.state()?.digest(rows.ctx)))
}

/// The state a handle target names: a root, or a stored value, recomputed from its recipe when it was evicted.
fn resolve(rows: &mut Rows<'_>, target: &Named) -> Result<Arc<Stored>, &'static str> {
    match target {
        Named::Root(key) => rows.tower.values_ref().root(key).ok_or(code::ARGUMENT_FORM),
        Named::Value(key) => match rows.tower.values().lookup(key) {
            Lookup::Hit(value) => Ok(value),
            Lookup::Evicted(recipe) => {
                let (value, digest) = recompute(rows, &recipe)?;
                rows.tower.values().insert(key.clone(), recipe, value, digest)
            }
            Lookup::Miss => Err(code::ARGUMENT_FORM),
        },
        _ => Err(code::ARGUMENT_FORM),
    }
}

/// A stored derivation's base: a root of this store, or the rows' own base under its key (ONE.17: a different epoch
/// does not invalidate it), or another entry. A base that no longer resolves is `KN2:stale.value_unavailable`.
fn source(rows: &mut Rows<'_>, source: &Source) -> Result<Arc<Stored>, &'static str> {
    match source {
        Source::Root(key) => {
            if let Some(root) = rows.tower.values_ref().root(key) {
                return Ok(root);
            }
            if root_key(rows).ok().as_ref() != Some(key) {
                return Err(VALUE_UNAVAILABLE);
            }
            let root = Arc::new(base_root(rows)?);
            Ok(rows.tower.values().insert_root(key.clone(), root))
        }
        Source::Value { key, recipe } => stored(rows, key, recipe),
    }
}

/// The stored value of `key`: a hit, or `recipe` recomputed after an eviction or in a store that lacks the entry, such
/// as a later epoch's.
fn stored(rows: &mut Rows<'_>, key: &ValueKey, recipe: &Recipe) -> Result<Arc<Stored>, &'static str> {
    if let Lookup::Hit(value) = rows.tower.values().lookup(key) {
        return Ok(value);
    }
    let (value, digest) = recompute(rows, recipe)?;
    rows.tower.values().insert(key.clone(), recipe.clone(), value, digest)
}

/// The value a stored derivation makes, with its digest: what a miss computes and an eviction recomputes.
fn recompute(rows: &mut Rows<'_>, recipe: &Recipe) -> Result<(Arc<Stored>, String), &'static str> {
    match recipe {
        Recipe::Derive { base, letters: Letters::Jumbling(letters), policy, protect } => {
            let start = source(rows, base)?;
            let protect: Vec<u32> = protect.iter().map(|&p| p as u32).collect();
            match derived(rows, &start, letters, *policy, &protect)? {
                Ok(value) => {
                    let digest = value.digest(rows.ctx);
                    Ok((Arc::new(Stored::Jumbling(value)), digest))
                }
                Err(_) => Err(code::STATE_HASH_MISMATCH),
            }
        }
        Recipe::Import { table, kind, evidence } => {
            let value = imported(rows, table, kind, evidence)?;
            let digest = value.digest(rows.ctx);
            Ok((Arc::new(Stored::Jumbling(value)), digest))
        }
        _ => Err(code::ARGUMENT_FORM),
    }
}

/// `levels::derive` of the letters `[cap, item]` from `start` under its own identity block: the derived value, or a
/// refusal's first refused prefix (the word's length under Net), outcome and code.
#[allow(clippy::type_complexity)]
fn derived(
    rows: &Rows<'_>,
    start: &Stored,
    letters: &[[u32; 2]],
    policy: c600_kernel::ProtectionMode,
    protect: &[u32],
) -> Result<Result<levels::Derived, (u64, &'static str, &'static str)>, &'static str> {
    let ctx = rows.ctx;
    let base = start.base().ok_or(code::ARGUMENT_FORM)?;
    let (state, _) = parts(start)?;
    let menu = state.menu().ok_or(INCOMPATIBLE_ALPHABET)?;
    let flat: Vec<i32> = letters.iter().flatten().map(|&i| i as i32).collect();
    let twists: Vec<_> =
        word::letters(ctx, menu, &flat).map_err(|_| code::ARGUMENT_FORM)?.into_iter().map(|l| l.1).collect();
    let identity = ctx.identity(state.menu());
    match levels::derive(ctx, base, &identity, &twists, policy, protect).map_err(engine)? {
        Err(refusal) => Err(refusal.code),
        Ok(Derivation::Derived(value)) => Ok(Ok(value)),
        Ok(Derivation::Refused(refused)) => {
            let prefix = refused.first_refused_prefix.unwrap_or(letters.len() as u64);
            Ok(Err((prefix, refused.outcome.as_str(), refused.refusal.code)))
        }
    }
}

/// `levels::import` of a raw table under the base's menu with a stated claim.
fn imported(rows: &Rows<'_>, table: &Json, kind: &str, evidence: &Json) -> Result<levels::Derived, &'static str> {
    let ctx = rows.ctx;
    let menu = rows.state()?.menu_arc();
    let kind = ReachKind::parse(kind).ok_or(code::ARGUMENT_FORM)?;
    let claim = Reachability::stated(kind, evidence.clone()).ok_or(code::ARGUMENT_FORM)?;
    let identity = ctx.identity(menu.as_deref());
    match levels::import(ctx, menu, table, &identity, claim) {
        Ok(Ok(value)) => Ok(value),
        Ok(Err(refusal)) => Err(refusal.code),
        Err(_) => Err(code::ARGUMENT_FORM),
    }
}

/// A state handle of a stored value.
fn state_handle(rows: &mut Rows<'_>, target: Named, stored: &Stored) -> Result<Value, &'static str> {
    let size = state_size(parts(stored)?.0);
    let id = rows.tower.values().handle(target)?;
    Ok(Value::handle(Handle::new(HandleTy::State(Puzzle::Jumbling), id, size)))
}

/// A verdict of KN0.5 as a sum value of `ty`, with `yes` carrying `holds` when the verdict is a goal's.
fn verdict(rows: &mut Rows<'_>, ty: SumTy, found: &Verdict, holds: Option<Holds>) -> Result<Value, &'static str> {
    Ok(match (found, holds) {
        (Verdict::Yes(_), Some(holds)) => Value::sum(ty, "yes", vec![Value::holds(holds)]),
        (Verdict::Yes(ev), None) => Value::sum(ty, "yes", vec![evidence(rows, ev.clone())?]),
        (Verdict::No(ev), _) => Value::sum(ty, "no", vec![evidence(rows, ev.clone())?]),
        (Verdict::Unknown(reason), _) => Value::sum(ty, "unknown", vec![Value::text(reason)]),
    })
}

fn missing(m: &Missing) -> Value {
    Value::record(vec![
        ("off", Value::int(m.off as u64)),
        ("uncovered", Value::int(m.uncovered as u64)),
        ("collisions", Value::int(m.collisions as u64)),
    ])
}

fn kernel(rows: &mut Rows<'_>, op: KernelOp, args: &[Value]) -> Result<Value, &'static str> {
    let ctx = rows.ctx;
    match (op, args) {
        (KernelOp::Snapshot(Puzzle::Jumbling), []) => {
            let key = root_key(rows)?;
            let root = match rows.tower.values_ref().root(&key) {
                Some(root) => root,
                None => {
                    let root = Arc::new(base_root(rows)?);
                    rows.tower.values().insert_root(key.clone(), root)
                }
            };
            state_handle(rows, Named::Root(key), &root)
        }
        (KernelOp::Project(level), [s]) => {
            let of = named(rows, s)?;
            let stored = resolve(rows, &of)?;
            let (state, _) = parts(&stored)?;
            let partial = match levels::project(ctx, state, level_key(level)).map_err(engine)? {
                Projection::Refused(refusal) => return Err(refusal.code),
                Projection::Value(partial) => partial,
            };
            let mut handle = |value: &LevelValue| -> Result<Value, &'static str> {
                let size = level_size(value);
                let target = Named::Level { of: Box::new(of.clone()), level: level_key(level), size };
                let id = rows.tower.values().handle(target)?;
                Ok(Value::handle(Handle::new(HandleTy::LevelValue(Puzzle::Jumbling, level), id, size)))
            };
            Ok(match partial {
                Partial::Total(value) => Value::sum(SumTy::Partial, "total", vec![handle(&value)?]),
                Partial::Partial { covered, missing: m } => {
                    Value::sum(SumTy::Partial, "partial", vec![handle(&covered)?, missing(&m)])
                }
                Partial::Undefined(reason) => Value::sum(SumTy::Partial, "undefined", vec![Value::text(&reason)]),
            })
        }
        (KernelOp::Digest, [v]) => {
            let Named::Level { of, level, .. } = named(rows, v)? else { return Err(code::ARGUMENT_FORM) };
            let stored = resolve(rows, &of)?;
            let (state, _) = parts(&stored)?;
            let hex = match levels::project(ctx, state, level).map_err(engine)? {
                Projection::Value(Partial::Total(value))
                | Projection::Value(Partial::Partial { covered: value, .. }) => value.digest(),
                Projection::Value(Partial::Undefined(_)) => return Err(code::STATE_HASH_MISMATCH),
                Projection::Refused(refusal) => return Err(refusal.code),
            };
            Value::digest(Puzzle::Jumbling, ir_level(level), &hex).ok_or(code::STATE_HASH_MISMATCH)
        }
        (KernelOp::Compare(level), [a, b]) => {
            let (a, b) = (named(rows, a)?, named(rows, b)?);
            let (a, b) = (resolve(rows, &a)?, resolve(rows, &b)?);
            let comparison = levels::compare(ctx, parts(&a)?.0, parts(&b)?.0, level_key(level)).map_err(engine)?;
            let ty = SumTy::Verdict(Puzzle::Jumbling, level, None);
            Ok(match comparison {
                Comparison::Total(found) => Value::sum(SumTy::Partial, "total", vec![verdict(rows, ty, &found, None)?]),
                Comparison::Partial { covered, a, b } => {
                    let both = Missing {
                        off: a.off + b.off,
                        uncovered: a.uncovered + b.uncovered,
                        collisions: a.collisions + b.collisions,
                    };
                    let found = verdict(rows, ty, &covered, None)?;
                    Value::sum(SumTy::Partial, "partial", vec![found, missing(&both)])
                }
                Comparison::Undefined(reason) => Value::sum(SumTy::Partial, "undefined", vec![Value::text(&reason)]),
                Comparison::Refused(refusal) => return Err(refusal.code),
            })
        }
        (KernelOp::Derive, [s, Value::MenuWord(word)]) => derive(rows, s, word),
        (KernelOp::Import, [t, Value::Sum(claim)]) => {
            let Named::PoseTable(table) = named(rows, t)? else { return Err(code::ARGUMENT_FORM) };
            let [Value::Handle(handle)] = claim.payload() else { return Err(code::ARGUMENT_FORM) };
            let Named::Evidence(ev) = rows.tower.values_ref().named(handle.id())?.clone() else {
                return Err(code::ARGUMENT_FORM);
            };
            let kind = claim.ctor().to_owned();
            let claim_text = format!("{kind}\n{}", ev.canonical());
            let key = ValueKey::Imported {
                table: c600_digest::sha256_hex(table.canonical().as_bytes()),
                claim: c600_digest::sha256_hex(claim_text.as_bytes()),
            };
            let recipe = Recipe::Import { table, kind, evidence: ev };
            let stored = match rows.tower.values().lookup(&key) {
                Lookup::Hit(value) => value,
                Lookup::Evicted(_) => resolve(rows, &Named::Value(key.clone()))?,
                Lookup::Miss => {
                    let (value, digest) = recompute(rows, &recipe)?;
                    rows.tower.values().insert(key.clone(), recipe, value, digest)?
                }
            };
            state_handle(rows, Named::Value(key), &stored)
        }
        (KernelOp::Reachability, [s]) => {
            let target = named(rows, s)?;
            let stored = resolve(rows, &target)?;
            let claim = parts(&stored)?.1.clone();
            let ev = evidence(rows, claim.to_json())?;
            Ok(Value::sum(SumTy::Reachability(Puzzle::Jumbling), claim.kind().name(), vec![ev]))
        }
        (KernelOp::Check(goal), [s, rest @ ..]) => {
            let target = named(rows, s)?;
            let stored = resolve(rows, &target)?;
            check(rows, goal, &stored, rest)
        }
        _ => Err(code::ARGUMENT_FORM),
    }
}

/// `derive(s, w)`: a hit, a miss or a recomputation of the stored value, or a refusal as a result.
fn derive(rows: &mut Rows<'_>, s: &Value, word: &MenuWord) -> Result<Value, &'static str> {
    let ctx = rows.ctx;
    let of = named(rows, s)?;
    let start = resolve(rows, &of)?;
    let (src, base_key) = match &of {
        Named::Root(key) => (Source::Root(key.clone()), key.clone()),
        Named::Value(key) => {
            let digest = rows.tower.values_ref().digest(key).ok_or(code::ARGUMENT_FORM)?.to_owned();
            let recipe = Arc::new(rows.tower.values_ref().recipe(key).ok_or(code::ARGUMENT_FORM)?.clone());
            (Source::Value { key: key.clone(), recipe }, BaseKey::Digest(digest))
        }
        _ => return Err(code::ARGUMENT_FORM),
    };
    let (state, _) = parts(&start)?;
    // The word's alphabet must be the state's (LB1.3): another model, menu or menu order, or free mode, refuses it.
    if word.identity().differing(&menu_identity(&ctx.identity(state.menu()))).is_some() {
        return Err(INCOMPATIBLE_ALPHABET);
    }
    let menu = state.menu().ok_or(INCOMPATIBLE_ALPHABET)?;
    let flat: Vec<i32> = word.letters().iter().flatten().map(|&i| i as i32).collect();
    let twists: Vec<_> =
        word::letters(ctx, menu, &flat).map_err(|_| code::ARGUMENT_FORM)?.into_iter().map(|l| l.1).collect();
    let (policy, protect) = (rows.tower.policy, rows.tower.protect.clone());
    let protect_ids: Vec<usize> = protect.iter().map(|&p| p as usize).collect();
    let key = ValueKey::Derived {
        base: base_key,
        word: levels::word_digest(&twists),
        protection: protection_key(policy, &protect_ids),
    };
    let letters = Letters::Jumbling(Arc::from(word.letters()));
    let recipe = Recipe::Derive { base: src, letters, policy, protect: protect_ids };
    let ty = SumTy::Derivation(Puzzle::Jumbling);
    let stored = match rows.tower.values().lookup(&key) {
        Lookup::Hit(value) => value,
        Lookup::Evicted(_) => resolve(rows, &Named::Value(key.clone()))?,
        Lookup::Miss => match derived(rows, &start, word.letters(), policy, &protect)? {
            Ok(value) => {
                let digest = value.digest(ctx);
                rows.tower.values().insert(key.clone(), recipe, Arc::new(Stored::Jumbling(value)), digest)?
            }
            Err((prefix, outcome, code)) => {
                let refused = Value::record(vec![
                    ("prefix", Value::int(prefix)),
                    ("outcome", Value::text(outcome)),
                    ("code", Value::text(code)),
                ]);
                return Ok(Value::sum(ty, "refused", vec![refused]));
            }
        },
    };
    let state = state_handle(rows, Named::Value(key), &stored)?;
    Ok(Value::sum(ty, "derived", vec![state]))
}

/// `check(goal, s)` at the goal's own level (LG2.10): `yes` with the goal's certificate, `no` with evidence, `unknown`
/// with a reason.
fn check(rows: &mut Rows<'_>, goal: Goal, stored: &Stored, rest: &[Value]) -> Result<Value, &'static str> {
    let ctx = rows.ctx;
    let (state, reach) = parts(stored)?;
    let ty = SumTy::Verdict(Puzzle::Jumbling, goal.level(), Some(goal));
    let pose_digest = state.digest(ctx);
    let subject = |level: Level, hex: &str| Digest::new(Puzzle::Jumbling, level, hex).ok_or(code::STATE_HASH_MISMATCH);
    let certificate = |hex: &str| -> Result<Holds, &'static str> {
        Holds::new(goal, subject(goal.level(), hex)?, None).ok_or(code::STATE_HASH_MISMATCH)
    };
    let off = state.off_lattice_count();
    let found = match goal {
        Goal::ExactPose => {
            let differ = state.pose_ids().iter().filter(|&&id| state.poses()[id as usize] != Pose::K(0)).count();
            if differ == 0 {
                return verdict(rows, ty, &Verdict::Yes(Json::Null), Some(certificate(&pose_digest)?));
            }
            Verdict::No(fact("pose_differs", vec![("pieces", Json::Int(differ as i64))]))
        }
        Goal::StickerTest => {
            let found = levels::sticker_test(ctx, state).map_err(engine)?;
            if let Verdict::Yes(ev) = &found {
                let hex = ev.get("labels_digest").and_then(Json::as_str).ok_or(code::STATE_HASH_MISMATCH)?;
                return verdict(rows, ty, &found, Some(certificate(hex)?));
            }
            found
        }
        Goal::LatticePlacement => {
            if off == 0 {
                return verdict(rows, ty, &Verdict::Yes(Json::Null), Some(certificate(&pose_digest)?));
            }
            Verdict::No(fact("off_lattice", vec![("off", Json::Int(off as i64))]))
        }
        Goal::LowHeight => {
            let [Value::Int(bound)] = rest else { return Err(code::ARGUMENT_FORM) };
            let all = heights(ctx, state).map_err(engine)?;
            let max = all.get("max_left_height").and_then(Json::as_i64).ok_or(code::STATE_HASH_MISMATCH)?;
            // A bound beyond `i64` is above every height; the interpreter refuses a negative one before the row.
            if i64::try_from(&**bound).map_or(true, |bound| max <= bound) {
                let subject = subject(goal.level(), &pose_digest)?;
                let holds = Holds::new(goal, subject, Some((**bound).clone())).ok_or(code::STATE_HASH_MISMATCH)?;
                return verdict(rows, ty, &Verdict::Yes(Json::Null), Some(holds));
            }
            let bound = Json::String(bound.to_string());
            Verdict::No(fact("height_above_bound", vec![("max_left_height", Json::Int(max)), ("bound", bound)]))
        }
        Goal::RetainedCheckpoint => {
            let centres = if off == 0 { levels::displaced_centres(ctx, state).map_err(engine)? } else { Vec::new() };
            if off > 0 {
                Verdict::No(fact("off_lattice", vec![("off", Json::Int(off as i64))]))
            } else if !centres.is_empty() {
                let ids = Json::Array(centres.iter().map(|&p| Json::Int(i64::from(p))).collect());
                Verdict::No(fact("centre_displaced", vec![("centres", ids)]))
            } else if reach.retained() && reach.at() == Some(pose_digest.as_str()) {
                return verdict(rows, ty, &Verdict::Yes(Json::Null), Some(certificate(&pose_digest)?));
            } else {
                Verdict::Unknown("no retained witness word of this state is held".into())
            }
        }
        Goal::LabelCheckpoint => {
            let base = stored.base().ok_or(code::ARGUMENT_FORM)?;
            let checkpoint = |nu: Option<&[i64]>| levels::label_checkpoint(ctx, state, reach, nu).map_err(engine);
            let mut found = checkpoint(None)?;
            if matches!(found, Verdict::Unknown(_)) {
                if let Partial::Total(row) = nu::nu(ctx, base).map_err(engine)?.map_err(|r| r.code)? {
                    found = checkpoint(Some(&row.values()[..]))?;
                }
            }
            if let Verdict::Yes(ev) = &found {
                let hex = ev.get("labels_digest").and_then(Json::as_str).ok_or(code::STATE_HASH_MISMATCH)?;
                return verdict(rows, ty, &found, Some(certificate(hex)?));
            }
            found
        }
    };
    verdict(rows, ty, &found, None)
}

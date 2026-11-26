//! The value store (T2 draft until the IR freeze, PHS.49; KN2.12, KN2.15, ONE.21, ONE.29, ONE.30, ONE.70, DET.16,
//! DET.24): the derived and imported states that a run's kernel rows make, held engine-side by the command layer, one
//! store per engine epoch ([`crate::layer::CommandLayer::values`]), and the host handles that name them.
//!
//! - **Entries** are keyed by their derivation ([`ValueKey`], ONE.21): a derived value by its base, named by its
//!   revision (KN0.2's `RevisionRef`: session, head event and digest) or by the base value's digest and never by a
//!   handle, by the `Digest<literal word>` of its word, and by the protection it was derived under; an imported value
//!   by the digests of its raw table and of its claim. Each entry keeps its stored derivation ([`Recipe`]): the base,
//!   the literal letters and the protection, or the raw table and the claim. The value itself may be evicted
//!   ([`ValueStore::evict`]); the entry, its recipe and its value's digest stay, so a host recomputes the same value
//!   from them (KN2.15).
//! - **Roots** are the snapshots the runs read, by revision or by digest. They are no derived values (KN2.12), so the
//!   store never evicts them.
//! - **Handles** ([`Named`]) name a root, an entry, a level value of one, a piece of evidence or a raw pose table. A
//!   handle's id names the epoch and the handle: its high 32 bits are the store's ordinal in its layer (1 for the first
//!   epoch, one more at each restart) and its low 32 bits the handle's index. A handle of an ended epoch is refused
//!   with `KN2:stale.epoch_changed` ([`ValueStore::named`]); the stored derivation it named still recomputes the same
//!   value in the new epoch's store. A handle the store never issued is `KN5:input.argument_form`.
//! - The store is not durable (ONE.30, ONE.70): an engine restart replaces it, and every value and handle ends with it.
//!
//! The store computes nothing. A host computes a value and puts it in ([`ValueStore::insert`]); on a later lookup it
//! gets a hit, or the recipe of an evicted value to recompute, or a miss. A hit, a miss and a recomputation give the
//! same value, and the host charges each the same declared tariff and the value's logical size for each live handle
//! (DET.16, DET.23, DET.24): nothing here is visible to a run. The counts ([`ValueStore::counts`]) are for tests.
//!
//! Every name here is a draft until the IR freeze.

use std::collections::BTreeMap;
use std::sync::Arc;

use c600_jumble::levels::{self, Base};
use c600_kernel::json::Value;
use c600_kernel::{ProtectionMode, Snapshot};

use crate::receipt::code;

/// KN1.7's levels, as the jumbling engine names them: the level of a level value's handle.
pub use c600_jumble::levels::Level;

/// A state the store holds: a 600-cell-Full label state, or a jumbling snapshot or derived value.
#[derive(Clone, Debug)]
pub enum Stored {
    Full(Snapshot),
    JumblingSnapshot(levels::Snapshot),
    Jumbling(levels::Derived),
}

impl Stored {
    /// The jumbling base of a derivation from this state; `None` for a 600-cell-Full state.
    pub fn base(&self) -> Option<Base<'_>> {
        match self {
            Stored::Full(_) => None,
            Stored::JumblingSnapshot(snapshot) => Some(Base::Snapshot(snapshot)),
            Stored::Jumbling(derived) => Some(Base::Derived(derived)),
        }
    }
}

/// The base of a derivation as its key names it (ONE.21): a revision (`RevisionRef`'s session, head event and digest;
/// a different epoch does not change it, ONE.17) or a value's digest. Never a handle.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BaseKey {
    Revision { session: String, head: String, digest: String },
    Digest(String),
}

/// The key of a stored value (ONE.21; draft).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ValueKey {
    /// A derived value: its base, the `Digest<literal word>` of its word and its protection ([`protection_key`]).
    Derived { base: BaseKey, word: String, protection: String },
    /// An imported value (KN2.14): the SHA-256 of its raw table's canonical JSON and of its claim's.
    Imported { table: String, claim: String },
}

/// The literal letters of a stored derivation: 600-cell-Full's signed generators, or the jumbling puzzle's menu letters
/// `[cap, item]` under the base's menu.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Letters {
    Full(Arc<[i32]>),
    Jumbling(Arc<[[u32; 2]]>),
}

impl Letters {
    /// The word's exact length.
    pub fn len(&self) -> usize {
        match self {
            Letters::Full(letters) => letters.len(),
            Letters::Jumbling(letters) => letters.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Where a stored derivation's base comes from: a root by its key, or another stored value by its key with the
/// stored derivation that recomputes it, so the chain leads back to a root without a handle or another entry (ONE.21).
#[derive(Clone, Debug, PartialEq)]
pub enum Source {
    Root(BaseKey),
    Value { key: ValueKey, recipe: Arc<Recipe> },
}

/// A stored derivation (ONE.21, ONE.30; draft): what recomputes a value without a handle.
#[derive(Clone, Debug, PartialEq)]
pub enum Recipe {
    /// KN2.11's derivation: the base, the literal letters, and the protection policy and protected set in force (the
    /// orbits for 600-cell-Full, the pieces for the jumbling puzzle).
    Derive { base: Source, letters: Letters, policy: ProtectionMode, protect: Vec<usize> },
    /// KN2.14's import under the base's menu: the raw table and the claim's kind and evidence.
    Import { table: Arc<Value>, kind: String, evidence: Arc<Value> },
}

/// The protection part of a derivation's key (draft): the policy's name and the SHA-256 of the protected set's ids
/// written in increasing order, each followed by a line feed.
pub fn protection_key(policy: ProtectionMode, protect: &[usize]) -> String {
    let mut ids = protect.to_vec();
    ids.sort_unstable();
    ids.dedup();
    let text: String = ids.iter().map(|id| format!("{id}\n")).collect();
    let policy = match policy {
        ProtectionMode::Net => "net",
        ProtectionMode::Strict => "strict",
    };
    format!("{policy}:{}", c600_digest::sha256_hex(text.as_bytes()))
}

/// What a handle names (draft).
#[derive(Clone, Debug, PartialEq)]
pub enum Named {
    /// A snapshot a run read.
    Root(BaseKey),
    /// A derived or imported value.
    Value(ValueKey),
    /// A state's value at a level (KN1.9): the state's handle target, the level and the logical size the host
    /// declared. It is recomputed from the state when it is read.
    Level { of: Box<Named>, level: Level, size: u128 },
    /// An evidence envelope a row answered with (KN0.5): its JSON.
    Evidence(Arc<Value>),
    /// Raw pose data a host gave a run (KN2.14): its JSON record.
    PoseTable(Arc<Value>),
}

/// A stored value or the recipe that recomputes it.
#[derive(Clone, Debug)]
pub enum Lookup {
    Hit(Arc<Stored>),
    Evicted(Recipe),
    Miss,
}

/// How often the store answered (for tests: none of it reaches a run).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Counts {
    pub hits: u64,
    pub misses: u64,
    pub recomputed: u64,
    pub evicted: u64,
}

/// One entry: its recipe, its value while held, and its value's digest, which eviction keeps.
#[derive(Clone, Debug)]
struct Entry {
    recipe: Recipe,
    value: Option<Arc<Stored>>,
    digest: String,
}

/// The value store of one engine epoch (draft; see the module).
#[derive(Debug)]
pub struct ValueStore {
    ordinal: u32,
    roots: BTreeMap<BaseKey, Arc<Stored>>,
    entries: BTreeMap<ValueKey, Entry>,
    handles: Vec<Named>,
    counts: Counts,
}

impl Default for ValueStore {
    fn default() -> ValueStore {
        ValueStore::new(1)
    }
}

impl ValueStore {
    /// An empty store for the epoch of ordinal `ordinal` (1 for a layer's first epoch).
    pub fn new(ordinal: u32) -> ValueStore {
        ValueStore {
            ordinal,
            roots: BTreeMap::new(),
            entries: BTreeMap::new(),
            handles: Vec::new(),
            counts: Counts::default(),
        }
    }

    /// The store of the next epoch: empty, with the next ordinal (a restart, ONE.70).
    pub fn next(&self) -> ValueStore {
        ValueStore::new(self.ordinal.saturating_add(1))
    }

    /// The epoch's ordinal in its layer.
    pub fn ordinal(&self) -> u32 {
        self.ordinal
    }

    /// A new handle of `named`: its id names this epoch and the handle. `KN7:resource_limit.host_exhausted` after
    /// 2^32 handles in one epoch.
    pub fn handle(&mut self, named: Named) -> Result<u64, &'static str> {
        let index = u32::try_from(self.handles.len()).map_err(|_| HOST_EXHAUSTED)?;
        self.handles.push(named);
        Ok(u64::from(self.ordinal) << 32 | u64::from(index))
    }

    /// What handle `id` names. A handle of an earlier epoch is `KN2:stale.epoch_changed`; one this store never issued
    /// is `KN5:input.argument_form`.
    pub fn named(&self, id: u64) -> Result<&Named, &'static str> {
        let ordinal = (id >> 32) as u32;
        if ordinal < self.ordinal {
            return Err(code::EPOCH_CHANGED);
        }
        if ordinal != self.ordinal {
            return Err(code::ARGUMENT_FORM);
        }
        self.handles.get((id & u64::from(u32::MAX)) as usize).ok_or(code::ARGUMENT_FORM)
    }

    /// The root snapshot `key`, if a run read it in this epoch.
    pub fn root(&self, key: &BaseKey) -> Option<Arc<Stored>> {
        self.roots.get(key).cloned()
    }

    /// Keeps the root snapshot `key`; a root already kept stays as it is.
    pub fn insert_root(&mut self, key: BaseKey, value: Arc<Stored>) -> Arc<Stored> {
        self.roots.entry(key).or_insert(value).clone()
    }

    /// The value of `key`: a hit, the recipe of an evicted value, or a miss.
    pub fn lookup(&mut self, key: &ValueKey) -> Lookup {
        match self.entries.get(key) {
            Some(Entry { value: Some(value), .. }) => {
                self.counts.hits += 1;
                Lookup::Hit(value.clone())
            }
            Some(entry) => Lookup::Evicted(entry.recipe.clone()),
            None => Lookup::Miss,
        }
    }

    /// The recipe of `key`, if the store has its entry.
    pub fn recipe(&self, key: &ValueKey) -> Option<&Recipe> {
        self.entries.get(key).map(|entry| &entry.recipe)
    }

    /// The digest of `key`'s value, if the store has its entry: kept after an eviction.
    pub fn digest(&self, key: &ValueKey) -> Option<&str> {
        self.entries.get(key).map(|entry| entry.digest.as_str())
    }

    /// Keeps `value`, whose digest is `digest`, under `key` with its recipe: a miss, or the recomputation of an evicted
    /// value. A recomputation must reach the digest the entry kept; `KN5:consistency.state_hash_mismatch` otherwise,
    /// and the entry stays evicted.
    pub fn insert(
        &mut self,
        key: ValueKey,
        recipe: Recipe,
        value: Arc<Stored>,
        digest: String,
    ) -> Result<Arc<Stored>, &'static str> {
        match self.entries.get_mut(&key) {
            Some(entry) if entry.value.is_some() => Ok(entry.value.clone().expect("a held value")),
            Some(entry) => {
                if entry.digest != digest || entry.recipe != recipe {
                    return Err(code::STATE_HASH_MISMATCH);
                }
                self.counts.recomputed += 1;
                entry.value = Some(value.clone());
                Ok(value)
            }
            None => {
                self.counts.misses += 1;
                self.entries.insert(key, Entry { recipe, value: Some(value.clone()), digest });
                Ok(value)
            }
        }
    }

    /// Evicts the value of `key`, or every value with `None`; the entries, their recipes and digests stay. Roots are
    /// not values and stay. The number of values evicted.
    pub fn evict(&mut self, key: Option<&ValueKey>) -> usize {
        let mut evicted = 0;
        for (k, entry) in self.entries.iter_mut() {
            let named = match key {
                Some(key) => key == k,
                None => true,
            };
            if named && entry.value.take().is_some() {
                evicted += 1;
            }
        }
        self.counts.evicted += evicted as u64;
        evicted
    }

    /// The keys of the store's entries, in key order.
    pub fn keys(&self) -> impl Iterator<Item = &ValueKey> {
        self.entries.keys()
    }

    /// Whether the store holds `key`'s value now.
    pub fn holds(&self, key: &ValueKey) -> bool {
        self.entries.get(key).is_some_and(|entry| entry.value.is_some())
    }

    /// The number of entries, held or evicted.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the store has no entry, no root and no handle: a new epoch's store.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty() && self.roots.is_empty() && self.handles.is_empty()
    }

    pub fn counts(&self) -> Counts {
        self.counts
    }
}

/// The interpreter's code for an answer that physical limits end (`KN7:resource_limit.host_exhausted`).
const HOST_EXHAUSTED: &str = "KN7:resource_limit.host_exhausted";

#[cfg(test)]
mod tests {
    use super::*;

    fn key(word: &str) -> ValueKey {
        ValueKey::Derived { base: BaseKey::Digest("b".into()), word: word.into(), protection: "net:x".into() }
    }

    fn recipe() -> Recipe {
        Recipe::Derive {
            base: Source::Root(BaseKey::Digest("b".into())),
            letters: Letters::Full(Arc::from(&[1, -2][..])),
            policy: ProtectionMode::Net,
            protect: Vec::new(),
        }
    }

    fn value() -> Arc<Stored> {
        let snapshot = levels::Snapshot::solved(&test_context(), None);
        Arc::new(Stored::JumblingSnapshot(snapshot))
    }

    fn test_context() -> c600_jumble::Context {
        let assets = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets");
        c600_jumble::Context::load(&assets).expect("the jumbling model loads")
    }

    #[test]
    fn handles_name_their_epoch_and_end_with_it() {
        let mut store = ValueStore::new(1);
        let id = store.handle(Named::Value(key("w"))).unwrap();
        assert_eq!(store.named(id), Ok(&Named::Value(key("w"))));
        let next = store.next();
        assert_eq!(next.ordinal(), 2);
        assert!(next.is_empty());
        assert_eq!(next.named(id), Err(code::EPOCH_CHANGED));
        assert_eq!(store.named(id + 1), Err(code::ARGUMENT_FORM), "never issued");
        assert_eq!(store.named(id | 7 << 32), Err(code::ARGUMENT_FORM), "a later epoch's");
    }

    #[test]
    fn eviction_keeps_the_recipe_and_the_digest() {
        let mut store = ValueStore::new(1);
        assert!(matches!(store.lookup(&key("w")), Lookup::Miss));
        store.insert(key("w"), recipe(), value(), "d".into()).unwrap();
        assert!(matches!(store.lookup(&key("w")), Lookup::Hit(_)));
        assert_eq!(store.evict(None), 1);
        assert!(!store.holds(&key("w")));
        assert!(matches!(store.lookup(&key("w")), Lookup::Evicted(r) if r == recipe()));
        assert_eq!(store.digest(&key("w")), Some("d"));
        assert_eq!(store.insert(key("w"), recipe(), value(), "e".into()).err(), Some(code::STATE_HASH_MISMATCH));
        store.insert(key("w"), recipe(), value(), "d".into()).unwrap();
        assert!(store.holds(&key("w")));
        assert_eq!(store.counts(), Counts { hits: 1, misses: 1, recomputed: 1, evicted: 1 });
        assert_eq!(store.evict(Some(&key("other"))), 0);
    }
}

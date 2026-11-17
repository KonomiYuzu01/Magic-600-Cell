//! LB1.8's literal words of one pinned menu (LP2 draft): the menu alphabet a run pins, and the identity block that
//! names it.
//!
//! A menu word ([`crate::value::MenuWord`]) is a sequence of jumbling menu letters `[cap, item]`: a pole index of the
//! model and an index in the pinned order of one menu, each counted from 0. It carries the identity block of its
//! alphabet (KN0.1), so a stored or crossing word names its menu. A run pins at most one alphabet
//! ([`crate::interp::Options::menu`]), and every menu word in the run is a word of it:
//!
//! - a menu-word literal takes the pinned alphabet when it is read, and a letter outside it stops the run;
//! - an argument, or a host's answer, that holds a menu word of another alphabet, or one with a letter outside it, is
//!   refused or stops the run;
//! - an inverse reads the pinned menu's inverse items, which no word carries.
//!
//! A run without a pinned alphabet holds no menu word. Each of these stops and refusals has the code
//! [`INCOMPATIBLE_ALPHABET`]. A host previews a menu word through [`crate::interp::Request::ProposeMenu`], which names
//! the word's alphabet, so a word played at a state of another alphabet is refused there, by the first field in which
//! the identity blocks differ, in KN1.4's order ([`MenuIdentity::differing`]).
//!
//! `c600-run` does not depend on the kernel crates, so the identity block's fields are text here: a host converts
//! `c600_kernel::IdentityBlock` and the inverse items of `c600_jumble`'s menu into a [`MenuAlphabet`].

use std::sync::Arc;

use crate::value::Value;

/// The chapter 13 code of a word operation, literal, argument, host answer or proposal whose letters are not of the
/// run's alphabet (LB1.3, LB1.8, LB1.9).
pub const INCOMPATIBLE_ALPHABET: &str = "LB1:input.incompatible_alphabet";

pub use c600_ir::MENU_INDEX_LIMIT;

/// The jumbling identity block that names a menu alphabet (KN0.1), as text, its fields in KN1.4's order. A block with
/// an empty field names no alphabet ([`MenuIdentity::names_alphabet`]): a menu-word literal has the empty block until
/// a run reads the literal with its pinned alphabet, and such a word has no canonical form.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MenuIdentity {
    pub puzzle: String,
    /// The `model_id` of the model's manifest.
    pub model_id: String,
    /// The order-free menu identity.
    pub menu_id: String,
    /// The menu-order digest.
    pub menu_order: String,
    /// The state contract revision.
    pub contract_rev: String,
}

impl MenuIdentity {
    /// The fields with their names, in KN1.4's order.
    pub fn fields(&self) -> [(&'static str, &str); 5] {
        [
            ("puzzle", &self.puzzle),
            ("model_id", &self.model_id),
            ("menu_id", &self.menu_id),
            ("menu_order", &self.menu_order),
            ("contract_rev", &self.contract_rev),
        ]
    }

    /// Whether the block names an alphabet: no field is empty.
    pub fn names_alphabet(&self) -> bool {
        self.fields().iter().all(|(_, text)| !text.is_empty())
    }

    /// The first field in which two identity blocks differ, in KN1.4's order: `puzzle`, `model_id`, `menu_id`,
    /// `menu_order`, then `contract_rev`. `None` when they name one alphabet.
    pub fn differing(&self, other: &MenuIdentity) -> Option<&'static str> {
        self.fields().iter().zip(other.fields()).find(|(a, b)| a.1 != b.1).map(|(a, _)| a.0)
    }
}

/// Why a menu alphabet is malformed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MenuError {
    /// A field of the identity block is empty, so the block names no alphabet.
    Identity(&'static str),
    /// More caps than [`MENU_INDEX_LIMIT`].
    Caps(u32),
    /// More items than [`MENU_INDEX_LIMIT`].
    Items(usize),
    /// The inverse items are no involution of the items: `inverse[item]` is not an item whose inverse is `item`.
    Inverse { item: usize },
}

impl std::fmt::Display for MenuError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MenuError::Identity(field) => write!(f, "the identity block's {field} is empty: it names no alphabet"),
            MenuError::Caps(caps) => write!(f, "a menu alphabet has at most {MENU_INDEX_LIMIT} caps, not {caps}"),
            MenuError::Items(items) => write!(f, "a menu has at most {MENU_INDEX_LIMIT} items, not {items}"),
            MenuError::Inverse { item } => {
                write!(f, "the inverse of item {item} is not an item whose inverse is item {item}")
            }
        }
    }
}

impl std::error::Error for MenuError {}

/// A pinned menu alphabet (LB1.8, LB1.9; LP2 draft): the letters `[cap, item]` with a cap below [`Self::caps`] and an
/// item of the menu, named by the menu's identity block, with the item of each item's inverse rotation. Two alphabets
/// with one identity block are one alphabet.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MenuAlphabet {
    identity: Arc<MenuIdentity>,
    caps: u32,
    inverse: Arc<[u32]>,
}

impl MenuAlphabet {
    /// The alphabet of `caps` caps and the menu whose item `x` has the inverse rotation `inverse[x]`. Refused when a
    /// field of the identity block is empty, when the caps or the items exceed [`MENU_INDEX_LIMIT`], or when the
    /// inverse items are no involution of the items, so the inverse of an inverse word is the word (LB1.9).
    pub fn new(identity: MenuIdentity, caps: u32, inverse: Vec<u32>) -> Result<MenuAlphabet, MenuError> {
        if let Some(&(field, _)) = identity.fields().iter().find(|(_, text)| text.is_empty()) {
            return Err(MenuError::Identity(field));
        }
        if caps > MENU_INDEX_LIMIT {
            return Err(MenuError::Caps(caps));
        }
        if inverse.len() > MENU_INDEX_LIMIT as usize {
            return Err(MenuError::Items(inverse.len()));
        }
        for (item, &y) in inverse.iter().enumerate() {
            if inverse.get(y as usize).map_or(true, |&x| x as usize != item) {
                return Err(MenuError::Inverse { item });
            }
        }
        Ok(MenuAlphabet { identity: Arc::new(identity), caps, inverse: inverse.into() })
    }

    /// The identity block that names the alphabet.
    pub fn identity(&self) -> &MenuIdentity {
        &self.identity
    }

    /// The identity block as the words of the alphabet share it.
    pub fn shared_identity(&self) -> &Arc<MenuIdentity> {
        &self.identity
    }

    /// The number of caps: a letter's cap is below it.
    pub fn caps(&self) -> u32 {
        self.caps
    }

    /// The number of items of the menu: a letter's item is below it.
    pub fn items(&self) -> usize {
        self.inverse.len()
    }

    /// The item of each item's inverse rotation, by item.
    pub fn inverse_items(&self) -> &[u32] {
        &self.inverse
    }

    /// Whether `letter` is a letter of the alphabet: a cap below [`Self::caps`] and an item of the menu.
    pub fn holds(&self, [cap, item]: [u32; 2]) -> bool {
        cap < self.caps && (item as usize) < self.inverse.len()
    }

    /// The inverse of a word of the alphabet (LB1.8, LB1.9): its letters in reverse order, each at its own cap with
    /// the item of the inverse rotation. `None` when a letter is not of the alphabet.
    pub fn inverse_word(&self, letters: &[[u32; 2]]) -> Option<Vec<[u32; 2]>> {
        letters
            .iter()
            .rev()
            .map(|&[cap, item]| self.holds([cap, item]).then(|| [cap, self.inverse[item as usize]]))
            .collect()
    }
}

/// Checks that every menu word in `value`, in its records, receipts, lists, sets and maps, is a word of `alphabet`: its
/// identity block is the alphabet's and each letter is in it. The error says why not: the first field in which the
/// blocks differ (KN1.4's order), a letter outside the alphabet, or no pinned alphabet at all. Its code is
/// [`INCOMPATIBLE_ALPHABET`]. The walk uses an explicit stack, so a deep value cannot overflow the host stack.
pub fn check_words(value: &Value, alphabet: Option<&MenuAlphabet>) -> Result<(), String> {
    let mut stack = vec![value];
    while let Some(v) = stack.pop() {
        match v {
            Value::MenuWord(w) => {
                let Some(alphabet) = alphabet else {
                    return Err("a menu word where the run pins no menu alphabet".into());
                };
                if let Some(field) = w.identity().differing(alphabet.identity()) {
                    return Err(format!(
                        "a menu word of another alphabet: the identity blocks differ in their {field}"
                    ));
                }
                if let Some(at) = w.letters().iter().position(|&l| !alphabet.holds(l)) {
                    let [cap, item] = w.letters()[at];
                    return Err(format!("letter {at}, [{cap}, {item}], is not a letter of the pinned menu"));
                }
            }
            Value::Record(r) | Value::Receipt(r) => stack.extend(r.fields().iter().map(|(_, v)| v)),
            Value::List(l) | Value::Set(l) | Value::Map(l) => stack.extend(l.items()),
            _ => {}
        }
    }
    Ok(())
}

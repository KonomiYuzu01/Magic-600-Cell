//! The letters of a literal jumbling word: the draft reading of the `Word` a solver names for the jumbling tool, and
//! LB1's literal-word constructors over the letters of one pinned menu.
//!
//! A `Word` of the IR draft is a sequence of integers. The draft reading takes them in pairs, `cap` then `item`: the
//! cap is a pole index and the item an index in the pinned order of the state's menu, as in the reference's letters
//! `[grip, item]`. The reading checks the shape only. Whether a letter is admissible is the kernel's answer when the
//! letter acts, and nothing here reduces, reorders or repairs a word: the letters are the literal ones (LB1.9, LB3.8).
//!
//! A [`LiteralWord`] is a word of one [`Alphabet`], the letters of one pinned menu named by its identity block (KN0.1).
//! Concatenation, inverse, conjugate, commutator and powers construct words from the letters alone (LB1.8): they take
//! no state and no model, so construction checks no state, and a constructed word is admitted or refused only when its
//! letters act, prefix by prefix. A word operation that mixes alphabets, or a word played at a state of another
//! alphabet, is refused with [`INCOMPATIBLE_ALPHABET`].
//!
//! [`Alphabet::pinned`] gives the same alphabet as the interpreter's menu alphabet (`c600_run::menu`, LP2's pull
//! request 210), which a run pins for the IR draft's menu words, so the library entries written as language code
//! (`crate::library`) build the words that [`LiteralWord`] builds here.

use std::collections::HashMap;
use std::sync::Arc;

use c600_jumble::model::POLES;
use c600_jumble::q5::transpose;
use c600_jumble::state::State;
use c600_jumble::twist::{Menu, Twist};
use c600_jumble::Context;
use c600_kernel::{IdentityBlock, Puzzle};
use c600_run::menu::{MenuAlphabet, MenuError, MenuIdentity};

/// The chapter 13 code of a word composition or application that mixes puzzle alphabets or pinned letter meanings
/// (LB1.3, LB1.9).
pub const INCOMPATIBLE_ALPHABET: &str = "LB1:input.incompatible_alphabet";

/// A letter as `c600_jumble::observe` takes it: the pair `[cap, item]` and its exact twist.
pub type Letter = ([i64; 2], Twist);

/// Why a `Word` is not a sequence of letters of the menu. The index is the letter's position in the word.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WordError {
    /// The word has an odd number of integers, so its last letter has no item.
    Unpaired { integers: usize },
    /// The cap is not a pole index.
    Cap { index: usize, cap: i32 },
    /// The item is not an index of the menu.
    Item { index: usize, item: i32, items: usize },
    /// A named twist is a word of exactly one letter.
    NotOneLetter { letters: usize },
    /// The menu has no exact twist at the cap: a broken menu, never a property of the word.
    Menu { index: usize, detail: String },
    /// The menu has no item whose rotation is the inverse of this item's: a broken menu, never a property of a word.
    NoInverse { item: usize },
    /// Two words, or a word and a state, have different alphabets: `field` is the first field in which their identity
    /// blocks differ (puzzle, model, menu identity, menu order or contract revision).
    Alphabet { field: &'static str },
}

impl WordError {
    /// The chapter 13 code of the error, where the chapter registers one: [`INCOMPATIBLE_ALPHABET`] for a mix of
    /// alphabets. A word that is no sequence of letters has none here; its code belongs to the command layer's row.
    pub fn code(&self) -> Option<&'static str> {
        match self {
            WordError::Alphabet { .. } => Some(INCOMPATIBLE_ALPHABET),
            _ => None,
        }
    }
}

impl std::fmt::Display for WordError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WordError::Unpaired { integers } => {
                write!(f, "a jumbling word is pairs of cap and item, and this word has {integers} integers")
            }
            WordError::Cap { index, cap } => write!(f, "letter {index}: cap {cap} is not a pole index"),
            WordError::Item { index, item, items } => {
                write!(f, "letter {index}: item {item} is not one of the menu's {items} items")
            }
            WordError::NotOneLetter { letters } => {
                write!(f, "a named twist is one letter, and this word has {letters}")
            }
            WordError::Menu { index, detail } => write!(f, "letter {index}: {detail}"),
            WordError::NoInverse { item } => write!(f, "the menu has no inverse of item {item}"),
            WordError::Alphabet { field } => {
                write!(f, "{INCOMPATIBLE_ALPHABET}: the alphabets differ in their {field}")
            }
        }
    }
}

impl std::error::Error for WordError {}

/// The literal letters of `word` under `menu`, in order. The empty word has no letters.
pub fn letters(ctx: &Context, menu: &Menu, word: &[i32]) -> Result<Vec<Letter>, WordError> {
    if word.len() % 2 != 0 {
        return Err(WordError::Unpaired { integers: word.len() });
    }
    let mut at_cap: HashMap<usize, Vec<Twist>> = HashMap::new();
    let mut out = Vec::with_capacity(word.len() / 2);
    for (index, pair) in word.chunks_exact(2).enumerate() {
        let (cap, item) = (pair[0], pair[1]);
        let c = usize::try_from(cap).ok().filter(|c| *c < POLES).ok_or(WordError::Cap { index, cap })?;
        let x = usize::try_from(item).ok().filter(|x| *x < menu.items.len()).ok_or(WordError::Item {
            index,
            item,
            items: menu.items.len(),
        })?;
        if !at_cap.contains_key(&c) {
            let twists = menu.for_cap(ctx, c).map_err(|error| WordError::Menu { index, detail: error.to_string() })?;
            at_cap.insert(c, twists);
        }
        out.push(([i64::from(cap), i64::from(item)], at_cap[&c][x].clone()));
    }
    Ok(out)
}

/// The one letter of a named twist: a word of exactly one letter.
pub fn twist(ctx: &Context, menu: &Menu, word: &[i32]) -> Result<Letter, WordError> {
    let mut letters = letters(ctx, menu, word)?;
    match letters.len() {
        1 => Ok(letters.remove(0)),
        letters => Err(WordError::NotOneLetter { letters }),
    }
}

/// The `Word` of a sequence of `[cap, item]` pairs: the inverse of [`letters`].
pub fn word_of(pairs: &[[i32; 2]]) -> Vec<i32> {
    pairs.iter().flatten().copied().collect()
}

/// The identity block as the interpreter's menu alphabet names it (`c600_run::menu`, which does not depend on the
/// kernel crates): its five fields as text in KN1.4's order, the puzzle written `jumbling` or `full` as the session
/// store writes it. A field the block does not have, as in free mode or for 600-cell-Full, is empty, and the text then
/// names no alphabet ([`MenuIdentity::names_alphabet`]).
pub fn menu_identity(block: &IdentityBlock) -> MenuIdentity {
    let text = |field: &Option<String>| field.clone().unwrap_or_default();
    let puzzle = match block.puzzle {
        Puzzle::Full => "full",
        Puzzle::Jumbling => "jumbling",
    };
    MenuIdentity {
        puzzle: puzzle.to_owned(),
        model_id: block.model_id.clone(),
        menu_id: text(&block.menu_id),
        menu_order: text(&block.menu_order),
        contract_rev: text(&block.contract_rev),
    }
}

/// The first field in which two identity blocks differ, in KN1.4's order.
fn differing(a: &IdentityBlock, b: &IdentityBlock) -> Option<&'static str> {
    if a.puzzle != b.puzzle {
        Some("puzzle")
    } else if a.model_id != b.model_id {
        Some("model_id")
    } else if a.menu_id != b.menu_id {
        Some("menu_id")
    } else if a.menu_order != b.menu_order {
        Some("menu_order")
    } else if a.contract_rev != b.contract_rev {
        Some("contract_rev")
    } else {
        None
    }
}

#[derive(Debug, PartialEq, Eq)]
struct Pinned {
    identity: IdentityBlock,
    /// `inverse[x]` is the item whose rotation is the inverse of item x's.
    inverse: Vec<i32>,
}

/// The alphabet of a literal jumbling word (LB1.9, LB1.10): the letters `[cap, item]` of one pinned menu, named by the
/// jumbling identity block of that menu (puzzle, model, order-free menu identity, menu-order digest and contract
/// revision), with the item of each item's inverse rotation. Two alphabets are one alphabet when their identity blocks
/// are equal, so an item index means the same rotation in both.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Alphabet(Arc<Pinned>);

impl Alphabet {
    /// The alphabet of `menu` in the model of `ctx`. The inverse of an item is the item whose matrix is its transpose,
    /// which an inverse-closed menu has.
    pub fn of(ctx: &Context, menu: &Menu) -> Result<Alphabet, WordError> {
        let inverse = (0..menu.items.len())
            .map(|item| {
                let inverse = transpose(&menu.items[item].1);
                let found = menu.items.iter().position(|(_, m)| *m == inverse);
                found.map(|x| x as i32).ok_or(WordError::NoInverse { item })
            })
            .collect::<Result<Vec<i32>, WordError>>()?;
        Ok(Alphabet(Arc::new(Pinned { identity: ctx.identity(Some(menu)), inverse })))
    }

    /// The identity block that names the alphabet.
    pub fn identity(&self) -> &IdentityBlock {
        &self.0.identity
    }

    /// The number of items of the menu.
    pub fn items(&self) -> usize {
        self.0.inverse.len()
    }

    /// The item of the inverse rotation of `item`, or `None` for an index outside the menu.
    pub fn inverse_item(&self, item: i32) -> Option<i32> {
        usize::try_from(item).ok().and_then(|x| self.0.inverse.get(x)).copied()
    }

    /// This alphabet as the interpreter's menu alphabet, which a run pins for the IR draft's menu words (LB1.8; LP2
    /// draft): the identity block as text ([`menu_identity`]), every pole of the model as a cap, and the item of each
    /// item's inverse rotation. A menu-word literal of the run then takes this alphabet, its `inverse` reads these
    /// inverse items, and an argument or a host's answer of another alphabet is refused with
    /// `LB1:input.incompatible_alphabet` (`c600_run::menu`).
    pub fn pinned(&self) -> Result<MenuAlphabet, MenuError> {
        let inverse = self.0.inverse.iter().map(|&item| item as u32).collect();
        MenuAlphabet::new(menu_identity(self.identity()), POLES as u32, inverse)
    }

    /// Refuses another alphabet with [`INCOMPATIBLE_ALPHABET`].
    pub fn check(&self, other: &Alphabet) -> Result<(), WordError> {
        match differing(self.identity(), other.identity()) {
            Some(field) => Err(WordError::Alphabet { field }),
            None => Ok(()),
        }
    }

    /// The literal word of `pairs` in this alphabet. It checks the shape of each letter, a cap that is a pole index and
    /// an item of the menu, and nothing else: no state is read.
    pub fn word(&self, pairs: &[[i32; 2]]) -> Result<LiteralWord, WordError> {
        for (index, &[cap, item]) in pairs.iter().enumerate() {
            if usize::try_from(cap).ok().filter(|&c| c < POLES).is_none() {
                return Err(WordError::Cap { index, cap });
            }
            if self.inverse_item(item).is_none() {
                return Err(WordError::Item { index, item, items: self.items() });
            }
        }
        Ok(LiteralWord { alphabet: self.clone(), pairs: pairs.to_vec() })
    }
}

/// A literal jumbling word: letters `[cap, item]` of one alphabet, in order (LB1.8, LB1.9). The empty word is a word.
/// Every constructor below reads the letters and nothing else; whether a word is admitted is the kernel's answer at a
/// state, prefix by prefix, when its letters act, and no constructor promises it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LiteralWord {
    alphabet: Alphabet,
    pairs: Vec<[i32; 2]>,
}

impl LiteralWord {
    pub fn alphabet(&self) -> &Alphabet {
        &self.alphabet
    }

    /// The letters `[cap, item]`, in order.
    pub fn pairs(&self) -> &[[i32; 2]] {
        &self.pairs
    }

    pub fn len(&self) -> usize {
        self.pairs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.pairs.is_empty()
    }

    /// The `Word` of the IR draft: the letters' pairs, flattened.
    pub fn word(&self) -> Vec<i32> {
        word_of(&self.pairs)
    }

    fn with(&self, pairs: Vec<[i32; 2]>) -> LiteralWord {
        LiteralWord { alphabet: self.alphabet.clone(), pairs }
    }

    /// This word, then `other` (LB1.8).
    pub fn concat(&self, other: &LiteralWord) -> Result<LiteralWord, WordError> {
        self.alphabet.check(&other.alphabet)?;
        Ok(self.with(self.pairs.iter().chain(&other.pairs).copied().collect()))
    }

    /// The inverse: the letters in reverse order, each at its own cap with the item of the inverse rotation. From the
    /// end of an admitted path it retraces that path to its start (LB1.9); the guarantee is the kernel's, at the state.
    pub fn inverse(&self) -> LiteralWord {
        self.with(self.pairs.iter().rev().map(|&[cap, item]| [cap, self.alphabet.0.inverse[item as usize]]).collect())
    }

    /// The conjugate by `setup`: `setup`, this word, then the inverse of `setup`. It carries no admissibility
    /// guarantee, whatever this word's (LB1.9).
    pub fn conjugate(&self, setup: &LiteralWord) -> Result<LiteralWord, WordError> {
        setup.concat(self)?.concat(&setup.inverse())
    }

    /// The commutator with `other`: this word, `other`, this word's inverse, then `other`'s inverse.
    pub fn commutator(&self, other: &LiteralWord) -> Result<LiteralWord, WordError> {
        self.concat(other)?.concat(&self.inverse())?.concat(&other.inverse())
    }

    /// The power `n`: this word `n` times; the empty word for 0; the inverse `-n` times for a negative `n`. The power
    /// is expanded letter by letter.
    pub fn power(&self, n: i32) -> LiteralWord {
        let base = if n < 0 { self.inverse() } else { self.clone() };
        self.with(std::iter::repeat_n(&base.pairs, n.unsigned_abs() as usize).flatten().copied().collect())
    }

    /// The literal letters this word plays at `state` (LB1.3, LB1.9), refused with [`INCOMPATIBLE_ALPHABET`] when the
    /// state's identity block is not the word's alphabet: another model, another menu or menu order, or free mode,
    /// which has no menu. Whether each prefix is admitted is the kernel's answer when the letters act.
    pub fn letters(&self, ctx: &Context, state: &State) -> Result<Vec<Letter>, WordError> {
        if let Some(field) = differing(self.alphabet.identity(), &ctx.identity(state.menu())) {
            return Err(WordError::Alphabet { field });
        }
        let Some(menu) = state.menu() else { return Err(WordError::Alphabet { field: "menu_id" }) };
        letters(ctx, menu, &self.word())
    }
}

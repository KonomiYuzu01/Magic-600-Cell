//! The letters of a literal jumbling word: the draft reading of the `Word` a solver names for the jumbling tool.
//!
//! A `Word` of the IR draft is a sequence of integers. The draft reading takes them in pairs, `cap` then `item`: the
//! cap is a pole index and the item an index in the pinned order of the state's menu, as in the reference's letters
//! `[grip, item]`. The reading checks the shape only. Whether a letter is admissible is the kernel's answer when the
//! letter acts, and nothing here reduces, reorders or repairs a word: the letters are the literal ones (LB1.9, LB3.8).

use std::collections::HashMap;

use c600_jumble::model::POLES;
use c600_jumble::twist::{Menu, Twist};
use c600_jumble::Context;

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

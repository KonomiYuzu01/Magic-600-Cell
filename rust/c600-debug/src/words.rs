//! Word references (EV2.5, EV2.11; LP4-6): letter positions of a literal word, resolved lazily.
//!
//! A [`WordRef`] cites letters of one expanded literal word by H7's zero-based half-open positions, with the word's
//! literal digest, identity block, alphabet and exact expanded length, `0 <= start <= end <= length`. A letter at `i` is
//! `[i, i+1)`, and `[length, length)` is the completed cursor. Positions count letters, never source bytes, chunks or
//! equality-key tokens. A compressed reference also carries an [`Occurrence`]: a path to one expansion occurrence and
//! an offset in it. The path tells child occurrences, repeat iterations and inverse traversals apart, so one part used
//! twice has two paths; an offset counts the occurrence's emitted letters in chronological order, an inverse part's in
//! its emitted order. So in `repeat([a, b], 3)`, iteration 1 at offset 1 cites the `b` at `[3, 4)`, not the one at
//! `[1, 2)`.
//!
//! [`Carrier::resolve`] checks a reference against the word it holds and gives the cited letters with their alphabet.
//! It walks the word's structure to the cited positions only: a compressed word of `3·2^70` letters gives any short
//! part at once. A reference to another literal digest, or one that reaches a part whose expansion the carrier does not
//! hold, is `EV2:input.replay_dependency_missing`; another identity, alphabet or length, an endpoint out of order or out
//! of range, and a path or offset that disagrees with the absolute position are `EV2:input.word_range_invalid`, and
//! nothing else is resolved in their place. Flat H10 storage is a single run of letters, whose absolute range is the
//! reference itself.
//!
//! # Drafts
//!
//! Chapter 05 owns the stored carrier and its bytes; [`Node`] is the debugger's reading of it. The literal digest is
//! KI.13's for retained generators, the SHA-256 of the letters as little-endian 32-bit integers; for menu letters it is
//! the SHA-256 of each letter's cap and item as two little-endian 32-bit integers, and for UI actions of each action as
//! one, both drafts. Positions are `u128`; a word longer than that has no carrier here.

use std::sync::Arc;

use c600_digest::{hex, Sha256};
use c600_run::value::{Compressed, Part as RunPart};

use crate::code::{REPLAY_DEPENDENCY_MISSING, WORD_RANGE_INVALID};
use crate::profile::Alphabet;
use crate::{DebugError, Identity};

/// One letter of a word, in its alphabet.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Letter {
    /// A signed retained generator of 600-cell-Full, magnitude 1 to 1200.
    Generator(i32),
    /// A jumbling menu letter `[cap, item]`.
    Menu { cap: u32, item: u32 },
    /// A UI action.
    Action(u32),
}

impl Letter {
    fn alphabet(self) -> Alphabet {
        match self {
            Letter::Generator(_) => Alphabet::RetainedGenerators,
            Letter::Menu { .. } => Alphabet::MenuLetters,
            Letter::Action(_) => Alphabet::UiActions,
        }
    }

    fn digest_bytes(self, out: &mut Sha256) {
        match self {
            Letter::Generator(g) => out.update(&g.to_le_bytes()),
            Letter::Menu { cap, item } => {
                out.update(&cap.to_le_bytes());
                out.update(&item.to_le_bytes());
            }
            Letter::Action(a) => out.update(&a.to_le_bytes()),
        }
    }
}

/// What a node of a word's structure is.
#[derive(Debug)]
pub enum Kind {
    /// Literal letters.
    Letters(Arc<[Letter]>),
    /// Child occurrences in order.
    Concat(Vec<Arc<Node>>),
    /// A child repeated a number of times.
    Repeat(Arc<Node>, u128),
    /// A child traversed inversely: its letters in reverse order, each inverted.
    Inverse(Arc<Node>),
    /// A part whose expansion the carrier does not hold: only its length and the digest that names it.
    Missing(String),
}

/// A node of a word's structure with its exact expanded length. Nodes are shared, so a word can be far longer than
/// its structure.
#[derive(Debug)]
pub struct Node {
    pub kind: Kind,
    pub length: u128,
}

fn invalid(detail: impl Into<String>) -> DebugError {
    DebugError::new(WORD_RANGE_INVALID, detail)
}

fn missing(detail: impl Into<String>) -> DebugError {
    DebugError::new(REPLAY_DEPENDENCY_MISSING, detail)
}

fn too_long() -> String {
    "the word is longer than this draft's positions hold".to_owned()
}

impl Node {
    pub fn letters(letters: &[Letter]) -> Arc<Node> {
        Arc::new(Node { length: letters.len() as u128, kind: Kind::Letters(letters.into()) })
    }

    pub fn concat(children: Vec<Arc<Node>>) -> Result<Arc<Node>, String> {
        let length = children.iter().try_fold(0u128, |s, c| s.checked_add(c.length)).ok_or_else(too_long)?;
        Ok(Arc::new(Node { kind: Kind::Concat(children), length }))
    }

    pub fn repeat(child: Arc<Node>, count: u128) -> Result<Arc<Node>, String> {
        let length = child.length.checked_mul(count).ok_or_else(too_long)?;
        Ok(Arc::new(Node { kind: Kind::Repeat(child, count), length }))
    }

    pub fn inverse(child: Arc<Node>) -> Arc<Node> {
        Arc::new(Node { length: child.length, kind: Kind::Inverse(child) })
    }

    pub fn missing(digest: &str, length: u128) -> Arc<Node> {
        Arc::new(Node { kind: Kind::Missing(digest.to_owned()), length })
    }
}

/// One step of an occurrence path.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    /// The child occurrence at this index of a concatenation.
    Child(usize),
    /// This iteration of a repeat, counted from 0.
    Iteration(u128),
    /// The child of an inverse part, traversed inversely.
    Inverse,
}

/// A compressed reference's occurrence: the path from the word's root and the offset in the occurrence's emitted
/// letters.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Occurrence {
    pub path: Vec<Step>,
    pub offset: u128,
}

/// A reference to letters of an expanded literal word (EV2.11).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WordRef {
    /// The word's literal digest (lowercase hex).
    pub digest: String,
    pub identity: Identity,
    pub alphabet: Alphabet,
    pub length: u128,
    pub start: u128,
    pub end: u128,
    pub occurrence: Option<Occurrence>,
}

/// Resolved letters: the cited interval, its letters in order and their alphabet and identity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Resolved {
    pub start: u128,
    pub end: u128,
    pub letters: Vec<Letter>,
    pub alphabet: Alphabet,
    pub identity: Identity,
}

/// A word as its carrier holds it: its structure, alphabet, identity block, exact length and literal digest.
#[derive(Clone, Debug)]
pub struct Carrier {
    root: Arc<Node>,
    alphabet: Alphabet,
    identity: Identity,
    digest: String,
    /// For menu letters, each item's inverse item, by item index.
    inverse_items: Option<Arc<[u32]>>,
}

impl Carrier {
    /// A carrier of `root`, whose literal digest the stored carrier names (`digest`). Every letter must be of
    /// `alphabet`; an inverse part needs invertible letters: generators, or menu letters with `inverse_items`.
    pub fn new(
        root: Arc<Node>,
        alphabet: Alphabet,
        identity: Identity,
        digest: &str,
        inverse_items: Option<Arc<[u32]>>,
    ) -> Result<Carrier, String> {
        let carrier = Carrier { root, alphabet, identity, digest: digest.to_owned(), inverse_items };
        carrier.validate()?;
        Ok(carrier)
    }

    /// The flat carrier of `letters` (H10's flat storage), with the literal digest of the letters.
    pub fn literal(
        letters: &[Letter],
        alphabet: Alphabet,
        identity: Identity,
        inverse_items: Option<Arc<[u32]>>,
    ) -> Result<Carrier, String> {
        let digest = literal_digest(letters);
        Carrier::new(Node::letters(letters), alphabet, identity, &digest, inverse_items)
    }

    /// The carrier of an interpreter's compressed word of retained generators (LG2.16), whose literal digest the
    /// stored carrier names.
    pub fn of_compressed(word: &Compressed, identity: Identity, digest: &str) -> Result<Carrier, String> {
        let mut nodes: Vec<Arc<Node>> = Vec::with_capacity(word.parts());
        for part in word.part_list() {
            let node = match part {
                RunPart::Leaf(letters) => {
                    Node::letters(&letters.iter().map(|&g| Letter::Generator(g)).collect::<Vec<_>>())
                }
                RunPart::Concat(a, b) => {
                    let (a, b) = (nodes.get(*a).cloned(), nodes.get(*b).cloned());
                    Node::concat(vec![a.ok_or("a part names a later part")?, b.ok_or("a part names a later part")?])?
                }
            };
            nodes.push(node);
        }
        let root = nodes.pop().ok_or("a compressed word without parts")?;
        Carrier::new(root, Alphabet::RetainedGenerators, identity, digest, None)
    }

    fn validate(&self) -> Result<(), String> {
        let mut seen = std::collections::HashSet::new();
        let mut stack = vec![(Arc::clone(&self.root), false)];
        while let Some((node, inverted)) = stack.pop() {
            if !seen.insert((Arc::as_ptr(&node) as usize, inverted)) {
                continue;
            }
            match &node.kind {
                Kind::Letters(letters) => {
                    if let Some(l) = letters.iter().find(|l| l.alphabet() != self.alphabet) {
                        return Err(format!("{l:?} is not in the word's alphabet {}", self.alphabet.name()));
                    }
                    if inverted {
                        for l in letters.iter() {
                            self.invert(*l)?;
                        }
                    }
                }
                Kind::Concat(children) => stack.extend(children.iter().map(|c| (Arc::clone(c), inverted))),
                Kind::Repeat(child, _) => stack.push((Arc::clone(child), inverted)),
                Kind::Inverse(child) => stack.push((Arc::clone(child), !inverted)),
                Kind::Missing(_) => {}
            }
        }
        Ok(())
    }

    fn invert(&self, letter: Letter) -> Result<Letter, String> {
        match letter {
            Letter::Generator(g) => Ok(Letter::Generator(-g)),
            Letter::Menu { cap, item } => self
                .inverse_items
                .as_ref()
                .and_then(|inverse| inverse.get(item as usize))
                .map(|&item| Letter::Menu { cap, item })
                .ok_or_else(|| format!("item {item} has no inverse item in the carrier")),
            Letter::Action(a) => Err(format!("the UI action {a} has no inverse")),
        }
    }

    pub fn length(&self) -> u128 {
        self.root.length
    }

    pub fn digest(&self) -> &str {
        &self.digest
    }

    pub fn alphabet(&self) -> Alphabet {
        self.alphabet
    }

    pub fn identity(&self) -> &Identity {
        &self.identity
    }

    /// A flat reference to `[start, end)` of this word.
    pub fn reference(&self, start: u128, end: u128) -> WordRef {
        WordRef {
            digest: self.digest.clone(),
            identity: self.identity.clone(),
            alphabet: self.alphabet,
            length: self.length(),
            start,
            end,
            occurrence: None,
        }
    }

    /// The emitted range `[base, base + length)` of the occurrence `path` names, and whether it is traversed
    /// inversely.
    pub fn occurrence(&self, path: &[Step]) -> Result<(u128, u128, bool), DebugError> {
        let (mut node, mut base, mut inverted) = (Arc::clone(&self.root), 0u128, false);
        for (depth, step) in path.iter().enumerate() {
            let (child, within) = match (&node.kind, *step) {
                (Kind::Concat(children), Step::Child(i)) if i < children.len() => {
                    let before: u128 = children[..i].iter().map(|c| c.length).sum();
                    (Arc::clone(&children[i]), before)
                }
                (Kind::Repeat(child, count), Step::Iteration(k)) if k < *count => (Arc::clone(child), k * child.length),
                (Kind::Inverse(child), Step::Inverse) => {
                    inverted = !inverted;
                    node = Arc::clone(child);
                    continue;
                }
                (Kind::Missing(digest), _) => {
                    return Err(missing(format!("the path reaches part {digest}, whose expansion is not held")));
                }
                _ => return Err(invalid(format!("step {depth} of the path, {step:?}, names no occurrence"))),
            };
            base += if inverted { node.length - within - child.length } else { within };
            node = child;
        }
        Ok((base, node.length, inverted))
    }

    /// Checks `reference` against this word and gives the letters it cites (see the module's description).
    pub fn resolve(&self, reference: &WordRef) -> Result<Resolved, DebugError> {
        if reference.digest != self.digest {
            return Err(missing(format!("the reference names the word {}, not this one", reference.digest)));
        }
        if reference.identity != self.identity || reference.alphabet != self.alphabet {
            return Err(invalid("the reference's identity block or alphabet is not the word's"));
        }
        if reference.length != self.length() {
            return Err(invalid(format!(
                "the reference's length {} is not the word's {}",
                reference.length,
                self.length()
            )));
        }
        if reference.start > reference.end || reference.end > self.length() {
            return Err(invalid(format!(
                "[{}, {}) is no interval of a word of length {}",
                reference.start,
                reference.end,
                self.length()
            )));
        }
        if let Some(occurrence) = &reference.occurrence {
            let (base, length, _) = self.occurrence(&occurrence.path)?;
            let at = base.checked_add(occurrence.offset).filter(|_| occurrence.offset <= length);
            if at != Some(reference.start) || reference.end > base + length {
                return Err(invalid(format!(
                    "the occurrence at {base} of length {length}, offset {}, does not give [{}, {})",
                    occurrence.offset, reference.start, reference.end
                )));
            }
        }
        let mut letters = Vec::with_capacity((reference.end - reference.start).min(1 << 16) as usize);
        self.emit(&self.root, reference.start, reference.end, false, &mut letters)?;
        Ok(Resolved {
            start: reference.start,
            end: reference.end,
            letters,
            alphabet: self.alphabet,
            identity: self.identity.clone(),
        })
    }

    /// Appends the letters of `[start, end)` of `node`'s emitted sequence, traversed inversely when `inverted`.
    fn emit(
        &self,
        node: &Node,
        start: u128,
        end: u128,
        inverted: bool,
        out: &mut Vec<Letter>,
    ) -> Result<(), DebugError> {
        if start >= end {
            return Ok(());
        }
        // In an inverse traversal, emitted position i is position length - 1 - i of the node's own order.
        let (lo, hi) = if inverted { (node.length - end, node.length - start) } else { (start, end) };
        match &node.kind {
            Kind::Letters(letters) => {
                let slice = &letters[lo as usize..hi as usize];
                if inverted {
                    for l in slice.iter().rev() {
                        out.push(self.invert(*l).map_err(invalid)?);
                    }
                } else {
                    out.extend_from_slice(slice);
                }
                Ok(())
            }
            Kind::Missing(digest) => {
                Err(missing(format!("[{start}, {end}) lies in part {digest}, whose expansion is not held")))
            }
            Kind::Inverse(child) => self.emit(child, start, end, !inverted, out),
            Kind::Concat(children) => {
                let order: Vec<&Arc<Node>> =
                    if inverted { children.iter().rev().collect() } else { children.iter().collect() };
                let mut base = 0u128;
                for child in order {
                    let (s, e) = (base, base + child.length);
                    if e > start && s < end {
                        self.emit(child, start.max(s) - s, end.min(e) - s, inverted, out)?;
                    }
                    if e >= end {
                        break;
                    }
                    base = e;
                }
                Ok(())
            }
            Kind::Repeat(child, _) => {
                let c = child.length;
                if c == 0 {
                    return Ok(());
                }
                // Every iteration emits the same letters, so an inverse traversal of a repeat is the repeat of the
                // child's inverse traversal.
                let mut at = start;
                while at < end {
                    let k = at / c;
                    let (s, e) = (k * c, (k + 1) * c);
                    self.emit(child, at - s, end.min(e) - s, inverted, out)?;
                    at = e;
                }
                Ok(())
            }
        }
    }
}

/// The literal digest of `letters` (see "Drafts"), in lowercase hex.
pub fn literal_digest(letters: &[Letter]) -> String {
    let mut sha = Sha256::new();
    for l in letters {
        l.digest_bytes(&mut sha);
    }
    hex(&sha.finish())
}

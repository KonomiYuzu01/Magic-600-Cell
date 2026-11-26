//! LG2.18's word key `word-key/1` of jumbling menu words (LP2 draft), under the item matrices of the run's pinned menu.
//!
//! The key replaces each maximal run of adjacent letters at one grip c by one token: c and the exact product of the
//! run's items in chronological action order. A twist acts on a pose from the left, so the letters
//! `[c, x_1] … [c, x_m]` give the token `(c, M_{x_m} ⋯ M_{x_1})`. The product is taken in the menu's base frame, in
//! which the menu record states its items. The twist at c is that product conjugated by c's retained frame, which is a
//! bijection, so two tokens at one grip are equal exactly when their twists are. A token is kept even when its product
//! is the identity, and a product outside the menu stays an exact group element. No commuting swap, identity deletion,
//! minimality or dynamic-target reduction is part of the key.
//!
//! A key carries its header (LG2.23): the alphabet's identity block, which pins the menu mapping and order, the grip
//! count and the item count, under the kind [`WORD_KEY`]. Keys order by header, then token by token: the grip, then
//! the sixteen entries row-major, each as its canonical triple (a, b, d) compared lexicographically as integers, with a
//! proper prefix first.
//!
//! A run keys menu words only when its pinned alphabet carries the item matrices ([`MenuAlphabet::with_matrices`]).
//! Such a run keys no Full word, and a run without them keys no menu word. So the keys a run compares share one header.
//! A host's answer that holds a key under another header, or a key out of its canonical form, stops the run
//! ([`check_keys`]). Each of these stops has the code [`KEY_CONTRACT_MISMATCH`]. A key has no canonical output form, so
//! no argument or result holds one (LG2.13).
//!
//! Menu items are generally not symmetries of the model, so the products of a run can grow. With one denominator per
//! matrix, a product's numerators have at most the factors' bits plus 6 and its denominator at most the factors' bits.
//! [`price`] reserves that bound and charges each product by its operands' sizes (a draft DET.57 price).

use std::cmp::Ordering;
use std::sync::Arc;

use num_bigint::BigInt;
use num_integer::Integer;
use num_traits::{One, Signed, Zero};

pub use crate::trace::KEY_CONTRACT_MISMATCH;

use crate::menu::{ItemMatrix, MenuAlphabet, MenuIdentity};
use crate::meter::integer_size;
use crate::value::{Quad, Value};

/// LG2.18's key kind and its rewrite version.
pub const WORD_KEY: &str = "word-key/1";

/// One token of a key: a grip and the exact product of a maximal run of adjacent letters at it, in the menu's base
/// frame, each entry in lowest terms with a positive denominator.
#[derive(Debug, PartialEq, Eq)]
pub struct Token {
    pub grip: u32,
    pub product: ItemMatrix,
}

impl Token {
    /// The logical size (DET.57): a 2-byte grip and the sixteen entries, each its three integers.
    fn size(&self) -> u128 {
        self.product.iter().flatten().fold(2, |s, q| s.saturating_add(quad_size(q)))
    }
}

/// LG2.18's key of a menu word (LP2 draft): its tokens under its header.
///
/// The header is the run's pinned metadata, shared by the run's keys, and not payload: a key's logical size is that of
/// its tokens (DET.57).
#[derive(Debug, PartialEq, Eq)]
pub struct MenuWordKey {
    identity: Arc<MenuIdentity>,
    grips: u32,
    items: u32,
    tokens: Vec<Token>,
}

impl MenuWordKey {
    /// A key as a host or the wire hands it over: `tokens` under the header of the alphabet named by `identity`, with
    /// `grips` grips and `items` items, unchecked. A run checks it against its pinned alphabet when it reads it
    /// ([`check_keys`]).
    pub fn from_parts(identity: Arc<MenuIdentity>, grips: u32, items: u32, tokens: Vec<Token>) -> MenuWordKey {
        MenuWordKey { identity, grips, items, tokens }
    }

    /// The identity block of the key's alphabet.
    pub fn identity(&self) -> &MenuIdentity {
        &self.identity
    }

    /// The alphabet's grip count: a token's grip is below it.
    pub fn grips(&self) -> u32 {
        self.grips
    }

    /// The alphabet's item count.
    pub fn items(&self) -> u32 {
        self.items
    }

    /// The tokens in order.
    pub fn tokens(&self) -> &[Token] {
        &self.tokens
    }

    /// The logical size of the tokens (DET.57).
    pub fn size(&self) -> u128 {
        self.tokens.iter().fold(0, |s, t| s.saturating_add(t.size()))
    }

    /// The first field in which the headers of two keys differ: the identity block's fields in KN1.4's order, then
    /// `grips` and `items`. `None` when they are one header.
    pub fn differing(&self, other: &MenuWordKey) -> Option<&'static str> {
        self.differs_from(&other.identity, other.grips, other.items)
    }

    fn differs_from(&self, identity: &MenuIdentity, grips: u32, items: u32) -> Option<&'static str> {
        self.identity.differing(identity).or_else(|| {
            if self.grips != grips {
                Some("grips")
            } else if self.items != items {
                Some("items")
            } else {
                None
            }
        })
    }

    /// Whether the tokens are in their canonical form: each grip below the grip count, adjacent tokens at distinct
    /// grips (each token covers a maximal run), and each entry a triple in lowest terms with a positive denominator.
    pub fn canonical(&self) -> bool {
        let grips = self.tokens.iter().all(|t| t.grip < self.grips);
        let maximal = self.tokens.windows(2).all(|w| w[0].grip != w[1].grip);
        grips && maximal && self.tokens.iter().all(|t| t.product.iter().flatten().all(canonical_quad))
    }
}

impl PartialOrd for MenuWordKey {
    fn partial_cmp(&self, other: &MenuWordKey) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for MenuWordKey {
    /// Headers first, then LG2.23's token order.
    fn cmp(&self, other: &MenuWordKey) -> Ordering {
        (&self.identity, self.grips, self.items)
            .cmp(&(&other.identity, other.grips, other.items))
            .then_with(|| tokens_order(&self.tokens, &other.tokens))
    }
}

/// LG2.23's order of two token sequences: lexicographic, with a proper prefix first. Tokens order by grip, then by
/// their entries row-major, each entry's triple (a, b, d) lexicographically as integers.
pub fn tokens_order(a: &[Token], b: &[Token]) -> Ordering {
    for (x, y) in a.iter().zip(b) {
        let order = x.grip.cmp(&y.grip).then_with(|| {
            x.product
                .iter()
                .flatten()
                .zip(y.product.iter().flatten())
                .fold(Ordering::Equal, |o, (p, q)| o.then_with(|| (&p.a, &p.b, &p.d).cmp(&(&q.a, &q.b, &q.d))))
        });
        if order != Ordering::Equal {
            return order;
        }
    }
    a.len().cmp(&b.len())
}

fn quad_size(q: &Quad) -> u128 {
    let size = |n: &BigInt| integer_size(n.bits() as u128);
    size(&q.a) + size(&q.b) + size(&q.d)
}

fn canonical_quad(q: &Quad) -> bool {
    q.d.is_positive() && q.a.gcd(&q.b).gcd(&q.d).is_one()
}

/// A 4×4 matrix over Z[√5] with one positive denominator: entry (i, j) is `(a[i][j] + b[i][j]·√5) / d`, and the gcd
/// of all its integers is 1.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Scaled {
    a: [[BigInt; 4]; 4],
    b: [[BigInt; 4]; 4],
    d: BigInt,
}

impl Scaled {
    /// A matrix of canonical entries over their least common denominator.
    pub(crate) fn of(m: &ItemMatrix) -> Scaled {
        let d = m.iter().flatten().fold(BigInt::one(), |l, q| l.lcm(&q.d));
        let scale = |i: usize, j: usize| &d / &m[i][j].d;
        let a = std::array::from_fn(|i| std::array::from_fn(|j| &m[i][j].a * scale(i, j)));
        let b = std::array::from_fn(|i| std::array::from_fn(|j| &m[i][j].b * scale(i, j)));
        Scaled { a, b, d }.reduced()
    }

    /// The product `self · other`.
    fn mul(&self, o: &Scaled) -> Scaled {
        let five = BigInt::from(5u8);
        let a = std::array::from_fn(|i| {
            std::array::from_fn(|j| {
                (0..4).fold(BigInt::zero(), |s, k| s + &self.a[i][k] * &o.a[k][j] + &five * &self.b[i][k] * &o.b[k][j])
            })
        });
        let b = std::array::from_fn(|i| {
            std::array::from_fn(|j| {
                (0..4).fold(BigInt::zero(), |s, k| s + &self.a[i][k] * &o.b[k][j] + &self.b[i][k] * &o.a[k][j])
            })
        });
        Scaled { a, b, d: &self.d * &o.d }.reduced()
    }

    /// The same matrix with the gcd of its integers divided out.
    fn reduced(self) -> Scaled {
        let g = self.a.iter().chain(&self.b).flatten().fold(self.d.clone(), |g, n| g.gcd(n));
        if g.is_one() {
            return self;
        }
        let div = |m: &[[BigInt; 4]; 4]| std::array::from_fn(|i| std::array::from_fn(|j| &m[i][j] / &g));
        Scaled { a: div(&self.a), b: div(&self.b), d: &self.d / &g }
    }

    /// The entries in lowest terms.
    fn quads(&self) -> ItemMatrix {
        std::array::from_fn(|i| {
            std::array::from_fn(|j| Quad::new(self.a[i][j].clone(), self.b[i][j].clone(), self.d.clone()))
        })
    }

    /// The bits of the largest numerator and of the denominator.
    pub(crate) fn bits(&self) -> (u128, u128) {
        let numerator = self.a.iter().chain(&self.b).flatten().map(|n| n.bits() as u128).max().unwrap_or(0);
        (numerator, self.d.bits() as u128)
    }
}

/// The maximal runs of adjacent letters at one grip: each run's grip and items, in order.
fn runs(letters: &[[u32; 2]]) -> impl Iterator<Item = (u32, &[[u32; 2]])> {
    letters.chunk_by(|x, y| x[0] == y[0]).map(|run| (run[0][0], run))
}

/// LG2.18's key of a word of `alphabet`. `None` when the alphabet carries no item matrices or a letter is not in it.
pub fn key(alphabet: &MenuAlphabet, letters: &[[u32; 2]]) -> Option<MenuWordKey> {
    let scaled = alphabet.scaled()?;
    if !letters.iter().all(|&l| alphabet.holds(l)) {
        return None;
    }
    let tokens = runs(letters)
        .map(|(grip, run)| {
            let mut product = scaled[run[0][1] as usize].clone();
            for &[_, item] in &run[1..] {
                product = scaled[item as usize].mul(&product);
            }
            Token { grip, product: product.quads() }
        })
        .collect();
    let items = alphabet.items() as u32;
    Some(MenuWordKey { identity: alphabet.shared_identity().clone(), grips: alphabet.caps(), items, tokens })
}

/// The draft price (DET.57) of the key of a word of `alphabet`: its work at `per_pair` units per pair of operand bytes
/// of each product, and the reservation of its tokens. `None` when [`key`] gives none.
///
/// Each product of a run multiplies the running product by the next item: 64 products of entries, each of the two
/// factors' entry sizes, then a reduction of the result's 33 integers. The last product's sixteen entries are put in
/// lowest terms. The reservation bounds each token's entries by the bits of its run's factors.
pub fn price(alphabet: &MenuAlphabet, letters: &[[u32; 2]], per_pair: u128) -> Option<(u128, u128)> {
    let scaled = alphabet.scaled()?;
    if !letters.iter().all(|&l| alphabet.holds(l)) {
        return None;
    }
    let entry = |(numerator, denominator): (u128, u128)| {
        integer_size(numerator).saturating_mul(2).saturating_add(integer_size(denominator))
    };
    let (mut work, mut reservation) = (0u128, 0u128);
    for (_, run) in runs(letters) {
        let mut bits = scaled[run[0][1] as usize].bits();
        for &[_, item] in &run[1..] {
            let factor = scaled[item as usize].bits();
            let operands = entry(bits).saturating_mul(entry(factor));
            bits = (bits.0.saturating_add(factor.0).saturating_add(6), bits.1.saturating_add(factor.1));
            let reduce = entry(bits).saturating_mul(entry(bits)).saturating_mul(33);
            work = work.saturating_add(per_pair.saturating_mul(operands.saturating_mul(64).saturating_add(reduce)));
        }
        let size = entry(bits);
        work = work.saturating_add(per_pair.saturating_mul(size.saturating_mul(size).saturating_mul(16)));
        reservation = reservation.saturating_add(size.saturating_mul(16).saturating_add(2));
    }
    Some((work, reservation))
}

/// Checks the word keys in `value`, in its records, receipts, lists, sets and maps, against the run's pinned
/// `alphabet`. When the alphabet carries item matrices, every key is a menu word key under its header, in canonical
/// form, and no key is a Full word key; otherwise no key is a menu word key. The error says why not. Its code is
/// [`KEY_CONTRACT_MISMATCH`]. The walk uses an explicit stack, so a deep value cannot overflow the host stack.
pub fn check_keys(value: &Value, alphabet: Option<&MenuAlphabet>) -> Result<(), String> {
    let keyed = alphabet.filter(|m| m.scaled().is_some());
    let mut stack = vec![value];
    while let Some(v) = stack.pop() {
        match v {
            Value::MenuWordKey(k) => {
                let Some(alphabet) = keyed else {
                    return Err("a menu word key where the run pins no menu matrices".into());
                };
                let items = alphabet.items() as u32;
                if let Some(field) = k.differs_from(alphabet.identity(), alphabet.caps(), items) {
                    return Err(format!("a menu word key under another header: the headers differ in their {field}"));
                }
                if !k.canonical() {
                    return Err("a menu word key whose tokens are not in their canonical form".into());
                }
            }
            Value::WordEqualityKey(_) if keyed.is_some() => {
                return Err("a Full word key where the run keys menu words".into());
            }
            Value::Record(r) | Value::Receipt(r) => stack.extend(r.fields().iter().map(|(_, v)| v)),
            Value::List(l) | Value::Set(l) | Value::Map(l) => stack.extend(l.items()),
            Value::Sum(s) => stack.extend(s.payload()),
            _ => {}
        }
    }
    Ok(())
}

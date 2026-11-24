//! CT-LB1-04 (chapter 03's test table; LB1.8's rows "Straight-line programs" and "Big integers and symbolic bounds",
//! LB1.11, LB5.5): a straight-line program's length equals its streamed expansion length on small fixtures, huge
//! symbolic bounds compute without expansion, and admission refuses an oversized expansion before allocation and
//! creates no new operation boundary.
//!
//! The entries are language code, the research library's modules `research_slp` and `research_bigint`, run on
//! `c600-run` with no host. The straight-line programs are small synthetic fixtures from a seeded generator; their
//! expansions are checked against the test's own expansion in Rust, and the streamed literal digest against the
//! SHA-256 of the letters as little-endian 32-bit integers. The huge bound is stage 19's B11 (its README, section
//! "Upper bounds and complete branches", read here as pinned bytes), checked against the test's own decimal big
//! integers. A pass is source and fixture evidence.
//!
//! CT-LB1-04 (source, part: the library half, straight-line length and streaming, symbolic bounds and admission;
//! not run: the kernel's expansion half over the recorded stage 18 words, which `c600-checkers/tests/s18.rs` runs).

mod research_common;

use c600_digest::sha256_hex;
use c600_library::research::{BIGINT_MODULE, BIGINT_SOURCE, SLP_MODULE, SLP_SOURCE};
use c600_run::interp::{Loaded, Options};
use c600_run::meter::{Amounts, Outcome};
use c600_run::value::Value;
use research_common::*;

/// The SHA-256 of the stage 19 README whose B11 the test computes.
const STAGE19_README_SHA256: &str = "a9b1b3334fabe880626395fdcd94b58d0e5dee444e29a15025ed90ab2cef6f8d";

#[derive(Clone, Debug)]
enum Rule {
    Leaf(Vec<i32>),
    Concat(usize, usize),
}

fn rule_value(rule: &Rule) -> Value {
    match rule {
        Rule::Leaf(letters) => {
            record(vec![("kind", text("leaf")), ("left", nat(0)), ("letters", Value::word(letters)), ("right", nat(0))])
        }
        Rule::Concat(l, r) => record(vec![
            ("kind", text("concat")),
            ("left", nat(*l as u64)),
            ("letters", Value::word(&[])),
            ("right", nat(*r as u64)),
        ]),
    }
}

/// The test's own expansion of a program: the last rule's letters.
fn expansion(rules: &[Rule]) -> Vec<i32> {
    let mut words: Vec<Vec<i32>> = Vec::new();
    for rule in rules {
        let word = match rule {
            Rule::Leaf(letters) => letters.clone(),
            Rule::Concat(l, r) => [words[*l].clone(), words[*r].clone()].concat(),
        };
        words.push(word);
    }
    words.pop().unwrap_or_default()
}

fn literal_digest(letters: &[i32]) -> String {
    let bytes: Vec<u8> = letters.iter().flat_map(|l| l.to_le_bytes()).collect();
    sha256_hex(&bytes)
}

/// A seeded synthetic program: leaves of up to four letters and concatenations of earlier rules.
fn program(g: &mut Fixtures) -> Vec<Rule> {
    let count = 1 + g.below(9) as usize;
    let mut rules = Vec::with_capacity(count);
    for k in 0..count {
        if k == 0 || g.below(3) == 0 {
            let letters = (0..g.below(5))
                .map(|_| {
                    let magnitude = 1 + g.below(1200) as i32;
                    if g.below(2) == 0 {
                        magnitude
                    } else {
                        -magnitude
                    }
                })
                .collect();
            rules.push(Rule::Leaf(letters));
        } else {
            rules.push(Rule::Concat(g.below(k as u64) as usize, g.below(k as u64) as usize));
        }
    }
    rules
}

fn slp() -> Loaded {
    loaded(&[SLP_SOURCE])
}

/// A program's rules as the entries take them. A compressed word has no canonical form, so a run cannot return one to
/// the host and the entries build the word from its rules.
fn rules_value(rules: &[Rule]) -> Value {
    list(rules.iter().map(rule_value).collect())
}

/// Straight-line length equals streamed expansion length: for 60 seeded programs, `program_length` answers the length
/// of the test's own expansion without expanding, and `streamed_length` in chunks of 1, 2, 3, 5, the whole length and
/// more expands exactly that many letters in the expected number of chunks, with the literal digest of the test's
/// expansion. `admitted_expansion` within its bound gives the expansion's letters. `program_cost` is the length times
/// the tariff.
#[test]
fn ct_lb1_04_straight_line_length_equals_streamed_expansion() {
    let loaded = slp();
    let mut g = Fixtures::new(0x5170_0104);
    let mut longest = 0;
    for _ in 0..60 {
        let rules = program(&mut g);
        let letters = expansion(&rules);
        longest = longest.max(letters.len());
        let c = rules_value(&rules);
        let length = as_u64(&call(&loaded, SLP_MODULE, "program_length", vec![c.clone()]));
        assert_eq!(length, letters.len() as u64, "{rules:?}");
        let cost = as_u64(&call(&loaded, SLP_MODULE, "program_cost", vec![c.clone(), nat(7)]));
        assert_eq!(cost, 7 * length);
        for chunk in [1, 2, 3, 5, length.max(1), length + 3] {
            let streamed = call(&loaded, SLP_MODULE, "streamed_length", vec![c.clone(), nat(chunk)]);
            assert_eq!(as_u64(field(&streamed, "length")), length);
            assert_eq!(as_u64(field(&streamed, "streamed")), length, "chunk {chunk}: {rules:?}");
            assert_eq!(as_u64(field(&streamed, "chunks")), length.div_ceil(chunk), "chunk {chunk}");
            assert_eq!(as_text(field(&streamed, "digest")), literal_digest(&letters), "chunk {chunk}");
        }
        let admitted = call(&loaded, SLP_MODULE, "admitted_expansion", vec![c.clone(), nat(length)]);
        assert!(as_bool(field(&admitted, "admitted")));
        assert_eq!(field(&admitted, "letters").clone().into_letters(), letters);
    }
    assert!(longest >= 16, "the fixtures reach words of {longest} letters");
}

/// A doubling chain of 200 concatenations holds 2^200 · 3 letters. Its exact length, 1,606,938,044,258,990,275,541,
/// 962,092,341,162,602,522,202,993,782,792,835,301,376 times 3, comes from `program_length` in one run that expands
/// nothing: the run has no `expand` and no stream row, and its memory stays small.
#[test]
fn ct_lb1_04_huge_lengths_compute_without_expansion() {
    let loaded = slp();
    let mut rules = vec![Rule::Leaf(vec![5, -6, 7])];
    for k in 0..200 {
        rules.push(Rule::Concat(k, k));
    }
    let c = rules_value(&rules);
    let run = run_with(&loaded, SLP_MODULE, "program_length", vec![c], &Options::new(WIDE));
    assert_eq!(run.summary.outcome, Outcome::Completed);
    let two_to_200 = "1606938044258990275541962092341162602522202993782792835301376";
    assert_eq!(decimal(run.value.as_ref().expect("a length")), times(two_to_200, 3));
    assert!(run.summary.rows.iter().all(|r| !r.at.ends_with("#expand") && !r.at.contains("#chunk:")));
    assert!(run.summary.used.memory < 1 << 26, "no expansion was allocated: {}", run.summary.used.memory);
}

/// Admission refuses an oversized expansion before allocation and creates no new operation boundary: over its bound,
/// `admitted_expansion` answers `admitted: false` with the exact length and no letters, in a run whose memory limit
/// could not hold the word and which has no `expand` row. A word of 2^100 letters admitted by a bound that large
/// reaches the IR draft's `expand`, which charges the letters' cost and memory before it expands one (KN2.10,
/// DET.57): the run stops on its budget at that row, and no value, so no partial word, comes back. Streaming chunks
/// stay rows of the one run that asked for them: one outcome, one value.
#[test]
fn ct_lb1_04_admission_refuses_an_oversized_expansion_before_allocation() {
    let loaded = slp();
    let mut rules = vec![Rule::Leaf(vec![11])];
    for k in 0..100 {
        rules.push(Rule::Concat(k, k));
    }
    let c = rules_value(&rules);
    let tight = Amounts { memory: 1 << 22, ..WIDE };
    let refused =
        run_with(&loaded, SLP_MODULE, "admitted_expansion", vec![c.clone(), nat(10_000_000)], &Options::new(tight));
    assert_eq!(
        (refused.summary.outcome, refused.summary.code),
        (Outcome::Completed, None),
        "{:?}",
        refused.summary.stop
    );
    let answer = refused.value.expect("an answer");
    assert!(!as_bool(field(&answer, "admitted")));
    assert_eq!(decimal(field(&answer, "length")), "1267650600228229401496703205376");
    assert_eq!(as_u64(field(&answer, "limit")), 10_000_000);
    assert!(field(&answer, "letters").clone().into_letters().is_empty());
    assert!(refused.summary.rows.iter().all(|r| !r.at.ends_with("#expand")), "nothing was expanded");

    let unbounded = Value::int(u128::MAX);
    let stopped = run_with(&loaded, SLP_MODULE, "admitted_expansion", vec![c, unbounded], &Options::new(WIDE));
    assert_eq!(stopped.summary.outcome, Outcome::CutOff, "{:?}", stopped.summary.stop);
    let stop = stopped.summary.stop.as_ref().expect("a stop position");
    assert!(stop.at.ends_with("#expand") && matches!(stop.phase, "kernel_cost" | "memory"), "{stop:?}");
    assert!(stopped.value.is_none(), "no partial word comes back");
    assert!(stopped.summary.used.memory < WIDE.memory);

    let small = rules_value(&[Rule::Leaf(vec![1, 2, 3, 4, 5, 6, 7])]);
    let streamed = run_with(&loaded, SLP_MODULE, "streamed_length", vec![small, nat(2)], &Options::new(WIDE));
    assert_eq!(streamed.summary.outcome, Outcome::Completed);
    let chunks = streamed.summary.rows.iter().filter(|r| r.at.contains("#chunk:")).count();
    assert_eq!(chunks, 4, "the stream's chunks are rows of this one run");
    assert_eq!(as_u64(field(streamed.value.as_ref().expect("a value"), "chunks")), 4);
}

/// The test's own decimal big integers, little-endian in base 10^9: enough for B11's binomials.
#[derive(Clone, Debug, PartialEq)]
struct Big(Vec<u64>);

const BASE: u64 = 1_000_000_000;

impl Big {
    fn small(n: u64) -> Big {
        let mut b = Big(vec![n]);
        b.normalize();
        b
    }

    fn normalize(&mut self) {
        let mut carry = 0;
        for limb in &mut self.0 {
            let v = *limb + carry;
            *limb = v % BASE;
            carry = v / BASE;
        }
        while carry > 0 {
            self.0.push(carry % BASE);
            carry /= BASE;
        }
        while self.0.len() > 1 && self.0.last() == Some(&0) {
            self.0.pop();
        }
    }

    fn mul_small(&mut self, m: u64) {
        let mut carry = 0;
        for limb in &mut self.0 {
            let v = *limb * m + carry;
            *limb = v % BASE;
            carry = v / BASE;
        }
        self.0.push(carry);
        self.normalize();
    }

    /// Divides by `d` and returns the remainder.
    fn div_small(&mut self, d: u64) -> u64 {
        let mut rest = 0;
        for limb in self.0.iter_mut().rev() {
            let v = rest * BASE + *limb;
            *limb = v / d;
            rest = v % d;
        }
        self.normalize();
        rest
    }

    fn add(&mut self, other: &Big) {
        if self.0.len() < other.0.len() {
            self.0.resize(other.0.len(), 0);
        }
        for (limb, o) in self.0.iter_mut().zip(&other.0) {
            *limb += o;
        }
        self.normalize();
    }

    fn binomial(n: u64, k: u64) -> Big {
        let mut b = Big::small(1);
        for i in 0..k {
            b.mul_small(n - i);
            assert_eq!(b.div_small(i + 1), 0, "C(n, i + 1) is whole");
        }
        b
    }

    fn decimal(&self) -> String {
        let mut out = self.0.last().expect("a limb").to_string();
        for limb in self.0.iter().rev().skip(1) {
            out.push_str(&format!("{limb:09}"));
        }
        out
    }
}

fn times(decimal: &str, m: u64) -> String {
    let mut b = Big::small(0);
    for chunk in decimal.as_bytes().chunks(1) {
        b.mul_small(10);
        b.add(&Big::small(u64::from(chunk[0] - b'0')));
    }
    b.mul_small(m);
    b.decimal()
}

/// Stage 19's B10 and B11 (its README): with N = 177120, B10 = 2 C(N,198) + C(N,111) + C(N,86) + 740 + the sum of
/// C(N,n) for n in {673, 389, 241, 216, 185, 60} + 6 C(N,3097), and B11 = B10 + C(N,111) + 59, as (coefficient, k)
/// terms and a constant.
const N: u64 = 177_120;
const B10_TERMS: [(i64, u64); 10] =
    [(2, 198), (1, 111), (1, 86), (1, 673), (1, 389), (1, 241), (1, 216), (1, 185), (1, 60), (6, 3097)];
const B10_CONSTANT: i64 = 740;
const C: u64 = 20_892_696;

fn terms(extra: &[(i64, u64)]) -> Value {
    list(
        B10_TERMS
            .iter()
            .chain(extra)
            .map(|&(coefficient, k)| record(vec![("coefficient", int(coefficient)), ("k", nat(k))]))
            .collect(),
    )
}

fn oracle(extra: &[(i64, u64)], constant: u64) -> Big {
    let mut total = Big::small(constant);
    for &(coefficient, k) in B10_TERMS.iter().chain(extra) {
        let mut term = Big::binomial(N, k);
        term.mul_small(coefficient as u64);
        total.add(&term);
    }
    total
}

/// Huge symbolic bounds compute without expansion: stage 19's B11, in `research_bigint`, is the test's own big
/// integer to the last digit and has 6,775 decimal digits, as the README states; B11 - B10 = C(N,111) + 59 and
/// T = B11 + C. `symbolic_bound` keeps the name and premises and answers `expanded: false` and `replayed: false`. The
/// runs read no word: no `expand`, `compress` or stream row. The README is checked as pinned bytes and for the formula
/// the test transcribes.
#[test]
fn ct_lb1_04_b11_has_6775_digits_without_expansion() {
    let readme_path = repository().join("research/jumbling/restoration/19-height-depth-framework/README.md");
    let readme = std::fs::read(&readme_path).expect("the stage 19 README");
    assert_eq!(sha256_hex(&readme), STAGE19_README_SHA256, "the pinned README");
    let readme = String::from_utf8(readme).expect("UTF-8");
    for line in [
        "B10 = 2 binom(N,198) + binom(N,111) + binom(N,86) + 740",
        "    + sum_(n in {673,389,241,216,185,60}) binom(N,n)",
        "    + 6 binom(N,3097)",
        "B11 = B10 + binom(N,111) + 59",
        "C   = 20892696",
        "T   = B11 + C.",
    ] {
        assert!(readme.contains(line), "the README states {line:?}");
    }
    assert!(readme.contains("With `N=177120`") && readme.contains("B11 is a theoretical 6,775-digit"));

    let loaded = loaded(&[BIGINT_SOURCE]);
    let b10 = call(&loaded, BIGINT_MODULE, "binomial_sum", vec![nat(N), terms(&[]), int(B10_CONSTANT)]);
    let run = run_with(
        &loaded,
        BIGINT_MODULE,
        "binomial_sum",
        vec![nat(N), terms(&[(1, 111)]), int(B10_CONSTANT + 59)],
        &Options::new(WIDE),
    );
    assert_eq!(run.summary.outcome, Outcome::Completed);
    let reads_words = |r: &c600_run::meter::Row| r.at.ends_with("#expand") || r.at.ends_with("#compress");
    assert!(run.summary.rows.iter().all(|r| !reads_words(r) && !r.at.contains("#chunk:")));
    let b11 = run.value.expect("B11");
    let expected = oracle(&[(1, 111)], 799);
    assert_eq!(decimal(&b11), expected.decimal());
    assert_eq!(decimal(&b10), oracle(&[], 740).decimal());
    assert_eq!(as_u64(&call(&loaded, BIGINT_MODULE, "digit_count", vec![b11.clone()])), 6775);
    assert_eq!(expected.decimal().len(), 6775);

    let mut difference = Big::binomial(N, 111);
    difference.add(&Big::small(59));
    let mut b10_plus = oracle(&[], 740);
    b10_plus.add(&difference);
    assert_eq!(decimal(&b11), b10_plus.decimal(), "B11 = B10 + C(N,111) + 59");
    let t = call(&loaded, BIGINT_MODULE, "product", vec![list(vec![b11.clone(), int(1)])]);
    let mut t_expected = expected.clone();
    t_expected.add(&Big::small(C));
    let t_sum = call(
        &loaded,
        BIGINT_MODULE,
        "binomial_sum",
        vec![nat(N), terms(&[(1, 111)]), int(B10_CONSTANT + 59 + C as i64)],
    );
    assert_eq!(decimal(&t_sum), t_expected.decimal(), "T = B11 + C");
    assert_eq!(decimal(&t), decimal(&b11));

    let premises = texts(&["availability", "invariant_domain", "selector"]);
    let bound = call(&loaded, BIGINT_MODULE, "symbolic_bound", vec![text("B11"), b11.clone(), premises.clone()]);
    assert_eq!(as_u64(field(&bound, "digits")), 6775);
    assert!(!as_bool(field(&bound, "expanded")) && !as_bool(field(&bound, "replayed")));
    assert_eq!(as_text(field(&bound, "name")), "B11");
    assert_eq!(strings(field(&bound, "premises")), ["availability", "invariant_domain", "selector"]);
}

/// The other big-integer entries against the test's own arithmetic: binomials at the edges (k = 0, k = n, k > n),
/// digit counts at the powers of ten and their neighbours, negatives and zero, products and the affine recurrence.
#[test]
fn ct_lb1_04_big_integer_entries_agree_with_the_test() {
    let loaded = loaded(&[BIGINT_SOURCE]);
    for (n, k) in [(0, 0), (5, 0), (5, 5), (5, 6), (10, 3), (52, 5), (177_120, 2), (300, 150)] {
        let value = call(&loaded, BIGINT_MODULE, "binomial", vec![nat(n), nat(k)]);
        let expected = if k > n { "0".to_string() } else { Big::binomial(n, k).decimal() };
        assert_eq!(decimal(&value), expected, "C({n}, {k})");
    }
    for x in [0i64, 1, 9, 10, 11, 99, 100, 101, 999_999_999, 1_000_000_000, -1, -10, -12345, i64::MAX, i64::MIN + 1] {
        let digits = as_u64(&call(&loaded, BIGINT_MODULE, "digit_count", vec![int(x)]));
        assert_eq!(digits, x.unsigned_abs().to_string().len() as u64, "digits of {x}");
    }
    let big = Big::binomial(1000, 500).decimal();
    let value = call(&loaded, BIGINT_MODULE, "binomial", vec![nat(1000), nat(500)]);
    assert_eq!(as_u64(&call(&loaded, BIGINT_MODULE, "digit_count", vec![value])), big.len() as u64);
    let product = call(&loaded, BIGINT_MODULE, "product", vec![list(vec![int(-3), int(7), int(11)])]);
    assert_eq!(as_i64(&product), -231);
    assert_eq!(as_i64(&call(&loaded, BIGINT_MODULE, "product", vec![list(vec![])])), 1);
    let recurrence = call(&loaded, BIGINT_MODULE, "affine_recurrence", vec![int(1), int(2), int(1), nat(10)]);
    assert_eq!(as_i64(&recurrence), 2047, "x -> 2x + 1 ten times from 1");
}

trait Letters {
    fn into_letters(self) -> Vec<i32>;
}

impl Letters for Value {
    fn into_letters(self) -> Vec<i32> {
        match self {
            Value::Word(letters) => letters.to_vec(),
            other => panic!("not a word: {other:?}"),
        }
    }
}

"""Write candidate.rs: LX-5's frozen Rust candidate with its own big integers replaced by num-bigint 0.5.1.

Everything from `fn rat` to the end of LX-5's candidate.rs is copied byte for byte. Only the header comment, the imports
and the `Int` type change. `Int` keeps the same fields' meaning (a sign in {-1, 0, 1} that is 0 exactly for zero, and a
magnitude) and every method's contract, so the rest of the program, its self-test and its output format are LX-5's.

From the repository root:

    python work/experiments/language-lx/lx5-crate/make_candidate.py           verify candidate.rs
    python work/experiments/language-lx/lx5-crate/make_candidate.py --write   write candidate.rs
"""
import argparse
import hashlib
import json
from pathlib import Path

HERE = Path(__file__).resolve().parent
LX5 = HERE.parent / "lx5-arithmetic"
SOURCE = LX5 / "candidate.rs"
TARGET = HERE / "candidate.rs"
SPLIT = b"fn rat(a: Int, d: Int) -> Vec<Int> {"

HEAD = """\
// LX-5 follow-up experiment, NOT production code. The big integers are num-bigint 0.5.1 with
// num-integer 0.1.47 and num-traits 0.2.19 (each MIT OR Apache-2.0). Everything from `fn rat`
// on is LX-5's frozen Rust candidate, copied byte for byte. No unsafe, FFI or subprocesses here.
use std::cmp::Ordering;
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{Read, Write, BufWriter};
use std::path::Path;
use std::time::Instant;

use num_bigint::BigUint;
use num_integer::Integer;
use num_traits::{ToPrimitive, Zero};

// Same contract as LX-5's own type: sign is -1, 0 or 1, and 0 exactly when the magnitude is zero.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Int { sign: i8, mag: BigUint }
impl Int {
    fn new(sign: i8, mag: BigUint) -> Self {
        Self { sign: if mag.is_zero() { 0 } else { sign }, mag }
    }
    fn zero() -> Self { Self::new(0, BigUint::zero()) }
    fn small(n: i64) -> Self { Self::new(n.signum() as i8, BigUint::from(n.unsigned_abs())) }
    fn abs(&self) -> Self { Self::new(1, self.mag.clone()) }
    fn neg(&self) -> Self { Self::new(-self.sign, self.mag.clone()) }
    fn cmp_abs(&self, b: &Self) -> Ordering { self.mag.cmp(&b.mag) }
    fn cmp(&self, b: &Self) -> i64 {
        let order = self.sign.cmp(&b.sign).then_with(|| {
            let order = self.cmp_abs(b);
            if self.sign < 0 { order.reverse() } else { order }
        });
        match order { Ordering::Less => -1, Ordering::Equal => 0, Ordering::Greater => 1 }
    }
    fn add_abs(&self, b: &Self) -> Self { Self::new(1, &self.mag + &b.mag) }
    fn sub_abs(&self, b: &Self) -> Self {
        assert!(self.cmp_abs(b) != Ordering::Less);
        Self::new(1, &self.mag - &b.mag)
    }
    fn add(&self, b: &Self) -> Self {
        if self.sign == 0 { return b.clone(); }
        if b.sign == 0 { return self.clone(); }
        if self.sign == b.sign {
            let mut result = self.add_abs(b); result.sign = self.sign; result
        } else if self.cmp_abs(b) == Ordering::Less {
            let mut result = b.sub_abs(self); if result.sign != 0 { result.sign = b.sign; } result
        } else {
            let mut result = self.sub_abs(b); if result.sign != 0 { result.sign = self.sign; } result
        }
    }
    fn sub(&self, b: &Self) -> Self { self.add(&b.neg()) }
    fn mul(&self, b: &Self) -> Self {
        if self.sign == 0 || b.sign == 0 { return Self::zero(); }
        Self::new(self.sign * b.sign, &self.mag * &b.mag)
    }
    fn mul_small(&self, b: u32) -> Self { Self::new(self.sign, &self.mag * b) }
    fn bits(&self) -> usize { self.mag.bits() as usize }
    fn shl(&self, bits: usize) -> Self {
        if self.sign == 0 { return Self::zero(); }
        Self::new(self.sign, &self.mag << bits)
    }
    fn shr_abs(&self, bits: usize) -> Self { Self::new(1, &self.mag >> bits) }
    fn shr(&self, bits: usize) -> Self {
        let mut result = self.shr_abs(bits);
        if self.sign < 0 {
            if result.shl(bits) != self.abs() { result = result.add(&Self::small(1)); }
            result = result.neg();
        }
        result
    }
    fn div_small(&self, divisor: u32) -> (Self, u32) {
        assert!(divisor != 0);
        let (q, r) = self.mag.div_rem(&BigUint::from(divisor));
        (Self::new(1, q), r.to_u32().unwrap())
    }
    fn div_rem_abs(&self, b: &Self) -> (Self, Self) {
        assert_ne!(b.sign, 0);
        let (q, r) = self.mag.div_rem(&b.mag);
        (Self::new(1, q), Self::new(1, r))
    }
    fn div_rem(&self, b: &Self) -> (Self, Self) {
        let (mut q, mut r) = self.div_rem_abs(b);
        if self.sign * b.sign < 0 {
            if r.sign != 0 { q = q.add(&Self::small(1)); r = b.abs().sub_abs(&r); }
            q = q.neg();
        }
        if r.sign != 0 { r.sign = b.sign; }
        (q, r)
    }
    fn div(&self, b: &Self) -> Self { self.div_rem(b).0 }
    fn gcd(&self, b: &Self) -> Self { Self::new(1, self.mag.gcd(&b.mag)) }
    fn parse(text: &str) -> Self {
        let (sign, digits) = if let Some(s) = text.strip_prefix('-') { (-1, s) } else { (1, text) };
        assert!(digits.bytes().all(|c| c.is_ascii_digit()), "decimal digits only");
        if digits.is_empty() { return Self::zero(); }
        Self::new(sign, BigUint::parse_bytes(digits.as_bytes(), 10).unwrap())
    }
    fn decimal(&self) -> String {
        if self.sign < 0 { format!("-{}", self.mag.to_str_radix(10)) } else { self.mag.to_str_radix(10) }
    }
    fn hex(&self) -> String {
        if self.sign < 0 { format!("-{}", self.mag.to_str_radix(16)) } else { self.mag.to_str_radix(16) }
    }
    fn parse_hex(text: &str) -> Self {
        let (sign, digits) = if let Some(s) = text.strip_prefix('-') { (-1, s) } else { (1, text) };
        assert!(digits.bytes().all(|c| c.is_ascii_hexdigit()), "hex digits only");
        if digits.is_empty() { return Self::zero(); }
        Self::new(sign, BigUint::parse_bytes(digits.as_bytes(), 16).unwrap())
    }
}

"""


def frozen_digest():
    return json.loads((LX5 / "SOURCE-MANIFEST.json").read_bytes())["candidate.rs"]


def build():
    """Return the candidate's bytes and the digest of the LX-5 source they were made from."""
    data = SOURCE.read_bytes()
    digest = hashlib.sha256(data).hexdigest()
    if digest != frozen_digest():
        raise RuntimeError("LX-5 candidate.rs differs from its frozen digest; refusing to derive from it")
    if data.count(SPLIT) != 1:
        raise RuntimeError("LX-5 candidate.rs has no single `fn rat` line to split at")
    newline = b"\r\n" if b"\r\n" in data else b"\n"
    head = HEAD.encode("ascii").replace(b"\n", newline)
    return head + data[data.index(SPLIT):], digest


def verify():
    """Raise unless candidate.rs is exactly what build() makes; return the LX-5 source digest."""
    content, digest = build()
    if not TARGET.exists() or TARGET.read_bytes() != content:
        raise RuntimeError("candidate.rs differs from make_candidate.py's output")
    return digest


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--write", action="store_true", help="write candidate.rs")
    args = parser.parse_args()
    try:
        if args.write:
            content, digest = build()
            TARGET.write_bytes(content)
            print("candidate.rs written from LX-5 candidate.rs", digest[:12])
        else:
            print("candidate.rs verified from LX-5 candidate.rs", verify()[:12])
    except RuntimeError as error:
        raise SystemExit(str(error))

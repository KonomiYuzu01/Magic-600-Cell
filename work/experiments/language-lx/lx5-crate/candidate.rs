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

fn rat(a: Int, d: Int) -> Vec<Int> {
    assert_ne!(d.sign, 0);
    let mut g = a.gcd(&d); g.sign = d.sign;
    vec![a.div(&g), d.div(&g)]
}
fn q5(a: Int, b: Int, d: Int) -> Vec<Int> {
    assert_ne!(d.sign, 0);
    let mut g = a.gcd(&b).gcd(&d); g.sign = d.sign;
    vec![a.div(&g), b.div(&g), d.div(&g)]
}
fn join(x: &[Int]) -> String { x.iter().map(Int::decimal).collect::<Vec<_>>().join(",") }
fn qsign(a: &Int, b: &Int) -> i64 {
    if a.sign == 0 { return b.sign as i64; }
    if b.sign == 0 || a.sign == b.sign { return a.sign as i64; }
    a.sign as i64 * a.mul(a).cmp(&b.mul(b).mul_small(5))
}

// Own SHA-256 because Rust std has no cryptographic hash. This adds audit burden.
struct Sha256 { state: [u32; 8], block: [u8; 64], used: usize, bytes: u64 }
impl Sha256 {
    fn new() -> Self { Self { state: [0x6a09e667,0xbb67ae85,0x3c6ef372,0xa54ff53a,0x510e527f,0x9b05688c,0x1f83d9ab,0x5be0cd19], block: [0;64], used: 0, bytes: 0 } }
    fn compress(&mut self, block: &[u8]) {
        const K: [u32;64] = [
            0x428a2f98,0x71374491,0xb5c0fbcf,0xe9b5dba5,0x3956c25b,0x59f111f1,0x923f82a4,0xab1c5ed5,
            0xd807aa98,0x12835b01,0x243185be,0x550c7dc3,0x72be5d74,0x80deb1fe,0x9bdc06a7,0xc19bf174,
            0xe49b69c1,0xefbe4786,0x0fc19dc6,0x240ca1cc,0x2de92c6f,0x4a7484aa,0x5cb0a9dc,0x76f988da,
            0x983e5152,0xa831c66d,0xb00327c8,0xbf597fc7,0xc6e00bf3,0xd5a79147,0x06ca6351,0x14292967,
            0x27b70a85,0x2e1b2138,0x4d2c6dfc,0x53380d13,0x650a7354,0x766a0abb,0x81c2c92e,0x92722c85,
            0xa2bfe8a1,0xa81a664b,0xc24b8b70,0xc76c51a3,0xd192e819,0xd6990624,0xf40e3585,0x106aa070,
            0x19a4c116,0x1e376c08,0x2748774c,0x34b0bcb5,0x391c0cb3,0x4ed8aa4a,0x5b9cca4f,0x682e6ff3,
            0x748f82ee,0x78a5636f,0x84c87814,0x8cc70208,0x90befffa,0xa4506ceb,0xbef9a3f7,0xc67178f2];
        let mut w = [0u32; 64];
        for i in 0..16 { w[i] = u32::from_be_bytes(block[4*i..4*i+4].try_into().unwrap()); }
        for i in 16..64 {
            let x = w[i-15]; let y = w[i-2];
            let s0 = x.rotate_right(7) ^ x.rotate_right(18) ^ (x >> 3);
            let s1 = y.rotate_right(17) ^ y.rotate_right(19) ^ (y >> 10);
            w[i] = w[i-16].wrapping_add(s0).wrapping_add(w[i-7]).wrapping_add(s1);
        }
        let [mut a,mut b,mut c,mut d,mut e,mut f,mut g,mut h] = self.state;
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let t1 = h.wrapping_add(s1).wrapping_add((e & f) ^ (!e & g)).wrapping_add(K[i]).wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let t2 = s0.wrapping_add((a & b) ^ (a & c) ^ (b & c));
            h=g; g=f; f=e; e=d.wrapping_add(t1); d=c; c=b; b=a; a=t1.wrapping_add(t2);
        }
        for (s, v) in self.state.iter_mut().zip([a,b,c,d,e,f,g,h]) { *s = s.wrapping_add(v); }
    }
    fn update(&mut self, mut data: &[u8]) {
        self.bytes = self.bytes.checked_add(data.len() as u64).unwrap();
        while !data.is_empty() {
            if self.used == 0 && data.len() >= 64 { self.compress(&data[..64]); data = &data[64..]; }
            else {
                let n = (64 - self.used).min(data.len());
                self.block[self.used..self.used+n].copy_from_slice(&data[..n]);
                self.used += n; data = &data[n..];
                if self.used == 64 { let block = self.block; self.compress(&block); self.used = 0; }
            }
        }
    }
    fn finish(mut self) -> String {
        let bits = self.bytes.checked_mul(8).unwrap();
        self.update(&[0x80]);
        while self.used != 56 { self.update(&[0]); }
        self.update(&bits.to_be_bytes());
        self.state.iter().map(|x| format!("{:08x}", x)).collect()
    }
}
fn word(path: &Path) -> String {
    let mut file = File::open(path).unwrap();
    let mut hash = Sha256::new(); let mut counts = vec![0u64; 65536];
    let mut block = [0u8; 65536]; let mut length = Int::zero(); let mut total = Int::zero();
    loop {
        let n = file.read(&mut block).unwrap(); if n == 0 { break; }
        assert_eq!(n % 2, 0, "odd ia16 bytes");
        hash.update(&block[..n]);
        let mut subtotal = 0u64;
        for bytes in block[..n].chunks_exact(2) {
            let letter = u16::from_le_bytes(bytes.try_into().unwrap()) as usize;
            counts[letter] = counts[letter].checked_add(1).unwrap(); subtotal += letter as u64;
        }
        length = length.add(&Int::small((n / 2) as i64)); total = total.add(&Int::small(subtotal as i64));
    }
    let hist = counts.iter().enumerate().filter(|(_, n)| **n != 0).map(|(i,n)| format!("{}:{}",i,n)).collect::<Vec<_>>().join(",");
    format!("{}|{}|{}|{}", length.decimal(), total.decimal(), hash.finish(), hist)
}

struct Case { id: String, size: String, op: String, measured: bool, x: Vec<Int>, file: String }
fn evaluate(c: &Case, word_dir: &Path) -> String {
    if c.op == "word" { return word(&word_dir.join(&c.file)); }
    let x = &c.x; let a = &x[0];
    let zero = Int::zero(); let b = x.get(1).unwrap_or(&zero);
    let result = match c.op.as_str() {
        "add" => a.add(b), "sub" => a.sub(b), "mul" => a.mul(b),
        "div" => a.div(b), "rem" => a.div_rem(b).1, "gcd" => a.gcd(b), "cmp" => Int::small(a.cmp(b)),
        "shl" => a.shl(b.decimal().parse().unwrap()), "shr" => a.shr(b.decimal().parse().unwrap()),
        "bits" => Int::small(a.bits() as i64), "dec" => Int::parse(&a.decimal()),
        "hex" => { let h = a.hex(); return format!("{}|{}", h, Int::parse_hex(&h).decimal()); },
        "sum" => x.iter().fold(Int::zero(), |s,v| s.add(v)),
        "rat_add" => return join(&rat(a.mul(&x[3]).add(&x[2].mul(b)), b.mul(&x[3]))),
        "rat_mul" => return join(&rat(a.mul(&x[2]), b.mul(&x[3]))),
        "rat_cmp" => {
            let r = rat(a.clone(), b.clone()); let s = rat(x[2].clone(), x[3].clone());
            Int::small(r[0].mul(&s[1]).cmp(&s[0].mul(&r[1])))
        },
        "q_add" | "q_mul" | "q_conj" | "q_norm" | "q_sign" => {
            let left = q5(a.clone(), b.clone(), x[2].clone());
            let (p,q,d) = (&left[0], &left[1], &left[2]);
            if c.op == "q_conj" { return join(&[p.clone(), q.neg(), d.clone()]); }
            if c.op == "q_norm" { return join(&rat(p.mul(p).sub(&q.mul(q).mul_small(5)), d.mul(d))); }
            if c.op == "q_sign" { return qsign(p,q).to_string(); }
            let right = q5(x[3].clone(), x[4].clone(), x[5].clone());
            let (u,v,e) = (&right[0], &right[1], &right[2]);
            return join(&if c.op == "q_add" {
                q5(p.mul(e).add(&u.mul(d)), q.mul(e).add(&v.mul(d)), d.mul(e))
            } else {
                q5(p.mul(u).add(&q.mul(v).mul_small(5)), p.mul(v).add(&q.mul(u)), d.mul(e))
            });
        },
        _ => panic!("unknown operation"),
    };
    result.decimal()
}

fn self_test() {
    // Independently check small random values against native i128, plus SHA vectors.
    let mut state = 0x6005u64;
    for _ in 0..512 {
        state ^= state << 13; state ^= state >> 7; state ^= state << 17;
        let a = (state as i64 >> 2) as i128;
        state ^= state << 13; state ^= state >> 7; state ^= state << 17;
        let b = ((state as i64 >> 2) | 1) as i128;
        let x = Int::parse(&a.to_string()); let y = Int::parse(&b.to_string());
        assert_eq!(x.add(&y).decimal(), (a+b).to_string());
        assert_eq!(x.sub(&y).decimal(), (a-b).to_string());
        assert_eq!(x.mul(&y).decimal(), (a*b).to_string());
        let q = a / b - i128::from(a % b != 0 && (a < 0) != (b < 0));
        let (quotient, remainder) = x.div_rem(&y);
        assert_eq!(quotient.decimal(), q.to_string());
        assert_eq!(remainder.decimal(), (a-q*b).to_string());
        assert_eq!(x.shr(33).decimal(), (a >> 33).to_string());
    }
    for (input, expected) in [
        ("", "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"),
        ("abc", "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"),
        ("abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq", "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1")
    ] {
        let mut h = Sha256::new(); for b in input.bytes() { h.update(&[b]); } assert_eq!(h.finish(), expected);
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut values = HashMap::new(); let mut cases = vec![];
    for line in fs::read_to_string(&args[0]).unwrap().lines() {
        let f: Vec<&str> = line.split('\t').collect();
        if f[0] == "value" { values.insert(f[1].to_string(), Int::parse(f[2])); }
        else { cases.push(Case { id: f[1].into(), size: f[2].into(), op: f[3].into(), measured: f[4] == "m", file: f[5].into(),
            x: if f[3] == "word" { vec![] } else { f[5..].iter().map(|k| values.get(*k).unwrap().clone()).collect() } }); }
    }
    let mode = args.get(3).map(String::as_str).unwrap_or("run");
    let selected: Vec<&Case> = cases.iter().filter(|c| match mode {
        "bench" => c.measured,
        "selftest" => c.size.starts_with("random-") || c.size == "limb-boundaries",
        _ => true,
    }).collect();
    let word_dir = Path::new(&args[1]);
    let mut answers = HashMap::new(); let mut timings = String::new();
    if mode == "bench" {
        let samples: usize = args[4].parse().unwrap(); let series_count: usize = args[5].parse().unwrap();
        for series in 0..series_count { for c in &selected {
            let warm = evaluate(c, word_dir);
            for sample in 0..samples {
                let start = Instant::now(); let result = evaluate(c, word_dir); let elapsed = start.elapsed().as_nanos();
                assert_eq!(result, warm, "nonrepeatable result: {}", c.id);
                answers.insert(c.id.clone(), result);
                timings.push_str(&format!("{}\t{}\t{}\t{}\n", c.id, series, sample, elapsed));
            }
        } }
        fs::write(format!("{}.timings.tsv", args[2]), timings).unwrap();
    } else {
        if mode == "selftest" { self_test(); }
        for c in &selected { answers.insert(c.id.clone(), evaluate(c, word_dir)); }
        if mode == "selftest" {
            let expected: HashMap<String,String> = fs::read_to_string(&args[4]).unwrap().lines().map(|line| {
                let (k,v) = line.split_once('\t').unwrap(); (k.into(),v.into())
            }).collect();
            for c in &selected { assert_eq!(answers[&c.id], expected[&c.id], "frozen random fixture: {}", c.id); }
            println!("Rust self-test: 512 native i128 cases, 3 SHA-256 vectors, {} frozen differential cases", selected.len());
        }
    }
    let mut out = BufWriter::new(File::create(&args[2]).unwrap());
    for c in selected { writeln!(out, "{}\t{}", c.id, answers[&c.id]).unwrap(); }
    out.flush().unwrap();
}

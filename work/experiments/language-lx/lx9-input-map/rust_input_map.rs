// LX-9 experiment only. Standard library; no crates or product changes.
use std::cmp::Ordering;
use std::env;
use std::fs::{read_to_string, File};
use std::io::{BufWriter, Write};

// Only the operations needed for squared integer dot products and cross-products.
// Limbs are little-endian base 2^32. This is not a fixed-width approximation.
#[derive(Clone, Debug, Eq, PartialEq)]
struct Nat(Vec<u32>);

impl Nat {
    fn zero() -> Self { Self(Vec::new()) }
    fn trim(mut self) -> Self {
        while self.0.last() == Some(&0) { self.0.pop(); }
        self
    }
    fn shifted(value: u64, shift: usize) -> Self {
        if value == 0 { return Self::zero(); }
        let mut out = vec![0; shift / 32];
        let wide = (value as u128) << (shift % 32);
        out.extend([wide as u32, (wide >> 32) as u32, (wide >> 64) as u32]);
        Self(out).trim()
    }
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.len().cmp(&other.0.len()).then_with(|| {
            self.0.iter().rev().cmp(other.0.iter().rev())
        })
    }
    fn add_assign(&mut self, other: &Self) {
        self.0.resize(self.0.len().max(other.0.len()), 0);
        let mut carry = 0u64;
        for i in 0..self.0.len() {
            let value = self.0[i] as u64 + other.0.get(i).copied().unwrap_or(0) as u64 + carry;
            self.0[i] = value as u32;
            carry = value >> 32;
        }
        if carry != 0 { self.0.push(carry as u32); }
    }
    fn sub(&self, other: &Self) -> Self {
        assert!(self.cmp(other) != Ordering::Less);
        let mut out = Vec::with_capacity(self.0.len());
        let mut borrow = 0i64;
        for i in 0..self.0.len() {
            let value = self.0[i] as i64 - other.0.get(i).copied().unwrap_or(0) as i64 - borrow;
            out.push(value as u32);
            borrow = if value < 0 { 1 } else { 0 };
        }
        assert_eq!(borrow, 0);
        Self(out).trim()
    }
    fn small(&self, factor: u32) -> Self {
        let mut out = Vec::with_capacity(self.0.len() + 1);
        let mut carry = 0u64;
        for &x in &self.0 {
            let value = x as u64 * factor as u64 + carry;
            out.push(value as u32);
            carry = value >> 32;
        }
        if carry != 0 { out.push(carry as u32); }
        Self(out).trim()
    }
    fn mul(&self, other: &Self) -> Self {
        let mut out = vec![0u32; self.0.len() + other.0.len()];
        for (i, &a) in self.0.iter().enumerate() {
            let mut carry = 0u64;
            for (j, &b) in other.0.iter().enumerate() {
                let value = a as u64 * b as u64 + out[i+j] as u64 + carry;
                out[i+j] = value as u32;
                carry = value >> 32;
            }
            out[i+other.0.len()] = carry as u32;
        }
        Self(out).trim()
    }
}

fn integer_target(target: [f64; 4]) -> [(bool, Nat); 4] {
    let parts: [(bool, u64, i32); 4] = target.map(|x| {
        let bits = x.to_bits();
        let exponent = ((bits >> 52) & 2047) as i32;
        let fraction = bits & ((1u64 << 52) - 1);
        assert!(exponent < 2047);
        (bits >> 63 != 0, if exponent == 0 { fraction } else { fraction | (1u64 << 52) },
         if exponent == 0 { -1074 } else { exponent - 1023 - 52 })
    });
    let common = parts.iter().filter(|p| p.1 != 0).map(|p| p.2).min().unwrap();
    parts.map(|(sign, mantissa, exponent)| {
        (sign, if mantissa == 0 { Nat::zero() } else { Nat::shifted(mantissa, (exponent-common) as usize) })
    })
}

fn squared_dot(t: &[(bool, Nat); 4], q: [i32; 4]) -> Nat {
    let mut pos = Nat::zero();
    let mut neg = Nat::zero();
    for (j, &coefficient) in q.iter().enumerate() {
        let term = t[j].1.small(coefficient.unsigned_abs());
        if t[j].0 ^ (coefficient < 0) { neg.add_assign(&term); } else { pos.add_assign(&term); }
    }
    let difference = if pos.cmp(&neg) == Ordering::Less { neg.sub(&pos) } else { pos.sub(&neg) };
    difference.mul(&difference)
}

// The fixtures are within one revolution. IEEE remainder reduces their angle.
fn remainder(degrees: f64) -> f64 {
    assert!(degrees.abs() <= 360.0);
    degrees - (degrees / 360.0).round_ties_even() * 360.0
}

fn norm(values: &[f64], fused: bool) -> f64 {
    let mut total = 0.0;
    for &x in values { total = if fused { x.mul_add(x, total) } else { total + x*x }; }
    total.sqrt()
}

fn target(axis: [f64; 3], degrees: f64, fused: bool) -> [f64; 4] {
    let maximum = axis.iter().map(|x| x.abs()).fold(0.0, f64::max);
    let mut a = axis.map(|x| x / maximum);
    let length = norm(&a, fused);
    a = a.map(|x| x / length);
    // Same binary64 radians multiplier as CPython math.radians.
    let theta = remainder(degrees) * 0.017453292519943295;
    let half = theta / 2.0;
    let sine = half.sin();
    let q = [half.cos(), sine*a[0], sine*a[1], sine*a[2]];
    let length = norm(&q, fused);
    q.map(|x| x / length)
}

fn choose(target: [f64; 4], degrees: f64, n: i32, dmax: i32) -> (i32, [i32; 3], &'static str) {
    assert!((0..=2).contains(&n) && (1..=16).contains(&dmax));
    let t = integer_target(target);
    let half = (remainder(degrees).abs() - 180.0).abs() <= 1.0;
    let mut best = None;
    let mut best_num = Nat::zero();
    let mut best_den = 1;
    // Exactly the reference's lexicographic (den, p, family) tie order.
    for d in 1..=dmax {
        for x in -n..=n { for y in -n..=n { for z in -n..=n {
            let p = [x, y, z];
            let families = if half && d == 1 && p != [0, 0, 0] { 2 } else { 1 };
            for family in 0..families {
                let q = [if family == 0 { d } else { 0 }, x, y, z];
                let den = q.iter().map(|v| (v*v) as u32).sum::<u32>();
                let num = squared_dot(&t, q);
                if best.is_none() || num.small(best_den).cmp(&best_num.small(den)) == Ordering::Greater {
                    best = Some((d, p, if family == 0 { "cayley" } else { "half_turn" }));
                    best_num = num;
                    best_den = den;
                }
            }
        } } }
    }
    best.unwrap()
}

fn main() {
    let args: Vec<String> = env::args().collect();
    assert_eq!(args.len(), 4, "input output plain|fma");
    assert!(args[3] == "plain" || args[3] == "fma");
    let fused = args[3] == "fma";
    let input = read_to_string(&args[1]).unwrap();
    let mut output = BufWriter::new(File::create(&args[2]).unwrap());
    writeln!(output, "id\tq0\tq1\tq2\tq3\tfamily\tden\tp0\tp1\tp2").unwrap();
    let mut lines = input.lines();
    assert_eq!(lines.next().unwrap(), "id\tax\tay\taz\tdegrees\tN\tD");
    for line in lines {
        let f: Vec<&str> = line.split('\t').collect();
        assert_eq!(f.len(), 7);
        let from_bits = |s| f64::from_bits(u64::from_str_radix(s, 16).unwrap());
        let axis = [from_bits(f[1]), from_bits(f[2]), from_bits(f[3])];
        let degrees = from_bits(f[4]);
        let q = target(axis, degrees, fused);
        let (d, p, family) = choose(q, degrees, f[5].parse().unwrap(), f[6].parse().unwrap());
        writeln!(output, "{}\t{:016x}\t{:016x}\t{:016x}\t{:016x}\t{}\t{}\t{}\t{}\t{}",
                 f[0], q[0].to_bits(), q[1].to_bits(), q[2].to_bits(), q[3].to_bits(),
                 family, d, p[0], p[1], p[2]).unwrap();
    }
    output.flush().unwrap();
}

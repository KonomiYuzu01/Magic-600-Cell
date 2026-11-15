// LX-4 experiment only. Int and SHA-256 extracted from lx5-arithmetic/candidate.rs.
// Unused arithmetic entry points removed; decimal parsing also accepts leading plus.
use std::cmp::Ordering;

#[derive(Clone, Debug, PartialEq, Eq)]
struct Int { sign: i8, limbs: Vec<u32> }
impl Int {
    fn new(sign: i8, mut limbs: Vec<u32>) -> Self {
        while limbs.last() == Some(&0) { limbs.pop(); }
        Self { sign: if limbs.is_empty() { 0 } else { sign }, limbs }
    }
    fn zero() -> Self { Self::new(0, vec![]) }
    fn small(n: i64) -> Self {
        let a = n.unsigned_abs();
        Self::new(n.signum() as i8, vec![a as u32, (a >> 32) as u32])
    }
    fn abs(&self) -> Self { Self::new(1, self.limbs.clone()) }
    fn neg(&self) -> Self { Self::new(-self.sign, self.limbs.clone()) }
    fn cmp_abs(&self, b: &Self) -> Ordering {
        self.limbs.len().cmp(&b.limbs.len()).then_with(|| self.limbs.iter().rev().cmp(b.limbs.iter().rev()))
    }
    fn add_abs(&self, b: &Self) -> Self {
        let mut result = Vec::new();
        let mut carry = 0u64;
        for i in 0..self.limbs.len().max(b.limbs.len()) {
            let v = *self.limbs.get(i).unwrap_or(&0) as u64 + *b.limbs.get(i).unwrap_or(&0) as u64 + carry;
            result.push(v as u32); carry = v >> 32;
        }
        result.push(carry as u32);
        Self::new(1, result)
    }
    fn sub_abs(&self, b: &Self) -> Self {
        assert!(self.cmp_abs(b) != Ordering::Less);
        let mut result = Vec::new();
        let mut borrow = 0u64;
        for i in 0..self.limbs.len() {
            let sub = *b.limbs.get(i).unwrap_or(&0) as u64 + borrow;
            let a = self.limbs[i] as u64;
            result.push(a.wrapping_sub(sub) as u32); borrow = u64::from(a < sub);
        }
        assert_eq!(borrow, 0);
        Self::new(1, result)
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
    fn mul(&self, b: &Self) -> Self {
        if self.sign == 0 || b.sign == 0 { return Self::zero(); }
        let mut result = vec![0u32; self.limbs.len() + b.limbs.len()];
        for (i, &a) in self.limbs.iter().enumerate() {
            let mut carry = 0u64;
            for (j, &v) in b.limbs.iter().enumerate() {
                let p = a as u64 * v as u64 + result[i + j] as u64 + carry;
                result[i + j] = p as u32; carry = p >> 32;
            }
            result[i + b.limbs.len()] = carry as u32;
        }
        Self::new(self.sign * b.sign, result)
    }
    fn mul_small(&self, b: u32) -> Self {
        let mut result = Vec::new();
        let mut carry = 0u64;
        for &a in &self.limbs {
            let p = a as u64 * b as u64 + carry;
            result.push(p as u32); carry = p >> 32;
        }
        result.push(carry as u32);
        Self::new(self.sign, result)
    }
    fn bits(&self) -> usize {
        self.limbs.last().map_or(0, |a| (self.limbs.len() - 1) * 32 + (32 - a.leading_zeros()) as usize)
    }
    fn shl(&self, bits: usize) -> Self {
        if self.sign == 0 { return Self::zero(); }
        let mut result = vec![0u32; bits / 32];
        let mut carry = 0u64;
        for &a in &self.limbs {
            let v = ((a as u64) << (bits % 32)) | carry;
            result.push(v as u32); carry = v >> 32;
        }
        result.push(carry as u32);
        Self::new(self.sign, result)
    }
    fn shr_abs(&self, bits: usize) -> Self {
        if bits >= self.bits() { return Self::zero(); }
        let skip = bits / 32; let shift = bits % 32;
        let mut result = Vec::new();
        for i in skip..self.limbs.len() {
            let high = *self.limbs.get(i + 1).unwrap_or(&0) as u64;
            result.push((((high << 32) | self.limbs[i] as u64) >> shift) as u32);
        }
        Self::new(1, result)
    }
    fn div_small(&self, divisor: u32) -> (Self, u32) {
        assert!(divisor != 0);
        let mut result = vec![0u32; self.limbs.len()];
        let mut remainder = 0u64;
        for i in (0..self.limbs.len()).rev() {
            let value = (remainder << 32) | self.limbs[i] as u64;
            result[i] = (value / divisor as u64) as u32;
            remainder = value % divisor as u64;
        }
        (Self::new(1, result), remainder as u32)
    }
    fn div_rem_abs(&self, b: &Self) -> (Self, Self) {
        assert_ne!(b.sign, 0);
        if self.cmp_abs(b) == Ordering::Less { return (Self::zero(), self.abs()); }
        if b.limbs.len() == 1 {
            let (q, r) = self.div_small(b.limbs[0]); return (q, Self::small(r as i64));
        }
        // Algorithm D: normalize the divisor, estimate one quotient limb,
        // correct against the next limb, subtract, and add back if needed.
        let shift = b.limbs.last().unwrap().leading_zeros() as usize;
        let v = b.abs().shl(shift).limbs;
        let n = self.limbs.len(); let m = v.len();
        let mut u = self.abs().shl(shift).limbs;
        u.resize(n + 1, 0);
        let mut quotient = vec![0u32; n - m + 1];
        let base = 1u64 << 32;
        for j in (0..=n - m).rev() {
            let numerator = ((u[j + m] as u64) << 32) | u[j + m - 1] as u64;
            let mut qhat = numerator / v[m - 1] as u64;
            let mut rhat = numerator % v[m - 1] as u64;
            while qhat >= base || qhat * v[m - 2] as u64 > base * rhat + u[j + m - 2] as u64 {
                qhat -= 1; rhat += v[m - 1] as u64;
                if rhat >= base { break; }
            }
            let mut borrow = 0u64;
            for i in 0..m {
                let product = qhat * v[i] as u64 + borrow;
                let (difference, underflow) = u[j + i].overflowing_sub(product as u32);
                u[j + i] = difference;
                borrow = (product >> 32) + u64::from(underflow);
            }
            let negative = (u[j + m] as u64) < borrow;
            u[j + m] = u[j + m].wrapping_sub(borrow as u32);
            if negative {
                qhat -= 1;
                let mut carry = 0u64;
                for i in 0..m {
                    let sum = u[j + i] as u64 + v[i] as u64 + carry;
                    u[j + i] = sum as u32; carry = sum >> 32;
                }
                u[j + m] = u[j + m].wrapping_add(carry as u32);
            }
            quotient[j] = qhat as u32;
        }
        (Self::new(1, quotient), Self::new(1, u[..m].to_vec()).shr_abs(shift))
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
    fn gcd(&self, b: &Self) -> Self {
        let mut a = self.abs(); let mut b = b.abs();
        while b.sign != 0 { let r = a.div_rem_abs(&b).1; a = b; b = r; }
        a
    }
    fn parse(text: &str) -> Self {
        let text = text.strip_prefix('+').unwrap_or(text);
        let (sign, digits) = if let Some(s) = text.strip_prefix('-') { (-1, s) } else { (1, text) };
        let mut value = Self::zero();
        // Decimal chunks avoid quadratic single-digit parsing overhead constants.
        let first = if digits.len() % 9 == 0 { 9 } else { digits.len() % 9 };
        let mut offset = 0;
        while offset < digits.len() {
            let len = if offset == 0 { first } else { 9 };
            let part: u32 = digits[offset..offset + len].parse().unwrap();
            value = value.mul_small(10u32.pow(len as u32)).add(&Self::small(part as i64));
            offset += len;
        }
        if value.sign != 0 { value.sign = sign; }
        value
    }
    fn decimal(&self) -> String {
        if self.sign == 0 { return "0".into(); }
        let mut value = self.abs(); let mut parts = vec![];
        while value.sign != 0 { let (q, r) = value.div_small(1_000_000_000); parts.push(r); value = q; }
        let mut result = if self.sign < 0 { "-".to_string() } else { String::new() };
        result.push_str(&parts.pop().unwrap().to_string());
        for part in parts.iter().rev() { result.push_str(&format!("{:09}", part)); }
        result
    }
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
fn sha256(data: &[u8]) -> String { let mut h = Sha256::new(); h.update(data); h.finish() }

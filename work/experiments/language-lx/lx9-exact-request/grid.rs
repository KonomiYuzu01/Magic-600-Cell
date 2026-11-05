// Full N=16, D=1000 enumeration. Standard library, checked u128 comparisons.
use std::{env, fs};

fn main() {
    let input = fs::read_to_string(env::args().nth(1).expect("input file")).unwrap();
    for line in input.lines() {
        let fields: Vec<_> = line.split_whitespace().collect();
        assert_eq!(fields.len(), 6);
        let t: Vec<i128> = fields[1..5].iter().map(|s| s.parse().unwrap()).collect();
        let half: bool = fields[5] == "1";
        let mut best_num = 0u128;
        let mut best_den = 1u128;
        let mut best = (0, 0, 0, 0, 0u8);
        let mut count = 0u64;
        let mut peak = 0u128;
        let mut visit = |d: i32, x: i32, y: i32, z: i32, h: bool| {
            let scalar = if h { 0 } else { d };
            let norm = (scalar * scalar + x * x + y * y + z * z) as u128;
            let dot = t[0] * scalar as i128 + t[1] * x as i128
                + t[2] * y as i128 + t[3] * z as i128;
            let num = dot.unsigned_abs().checked_mul(dot.unsigned_abs()).unwrap();
            let left = num.checked_mul(best_den).unwrap();
            let right = best_num.checked_mul(norm).unwrap();
            peak = peak.max(num).max(norm).max(left).max(right);
            let key = (d, x, y, z, if h { 1u8 } else { 0u8 });
            if count == 0 || left > right || (left == right && key < best) {
                best = key;
                best_num = num;
                best_den = norm;
            }
            count += 1;
        };
        for d in 1..=1000 {
            for x in -16..=16 {
                for y in -16..=16 {
                    for z in -16..=16 { visit(d, x, y, z, false); }
                }
            }
        }
        if half {
            for x in -16..=16 {
                for y in -16..=16 {
                    for z in -16..=16 {
                        if x != 0 || y != 0 || z != 0 { visit(1, x, y, z, true); }
                    }
                }
            }
        }
        println!("{}\t{},{},{},{},{}\t{}\t{}", fields[0], best.0, best.1, best.2, best.3,
                 if best.4 == 0 { "C" } else { "H" }, count, 128 - peak.leading_zeros());
    }
}

//! The big-integer sign test of `c600-jumble`'s `q5` at the state contract's smallest margins: Lucas and Fibonacci
//! numbers beyond the 128-bit limit of `c600-kernel`'s fixed-width test, where |L² − 5F²| = 4 with the sign of
//! (−1)^n. A pass is source and fixture evidence for the sign test only, not for the model's regions.

use c600_jumble::q5::sign5;
use num_bigint::BigInt;

/// The pairs (n, L_n, F_n) for n from 0 to 400.
fn lucas_fibonacci() -> Vec<(usize, BigInt, BigInt)> {
    let (mut l, mut f) = ((BigInt::from(2), BigInt::from(1)), (BigInt::from(0), BigInt::from(1)));
    let mut pairs = Vec::new();
    for n in 0..=400 {
        pairs.push((n, l.0.clone(), f.0.clone()));
        l = (l.1.clone(), &l.0 + &l.1);
        f = (f.1.clone(), &f.0 + &f.1);
    }
    pairs
}

/// CT-KN3-01 (part: the big-integer sign test at Lucas and Fibonacci margins beyond 128 bits, at both signs; not run:
/// the model's regions and the state contract's E0 controls).
#[test]
fn ct_kn3_01_big_integer_signs_at_lucas_fibonacci_margins() {
    let five = BigInt::from(5);
    for (n, l, f) in lucas_fibonacci().into_iter().filter(|(n, _, _)| *n >= 130) {
        let margin = &l * &l - &five * &f * &f;
        let expected = if n % 2 == 0 { 1 } else { -1 };
        assert_eq!(margin, BigInt::from(4 * expected), "L_{n}^2 - 5 F_{n}^2");
        assert_eq!(sign5(&l, &-&f), expected, "L_{n} - F_{n} sqrt 5");
        assert_eq!(sign5(&-&l, &f), -expected, "-L_{n} + F_{n} sqrt 5");
        assert_eq!(sign5(&l, &f), 1, "L_{n} + F_{n} sqrt 5");
        assert_eq!(sign5(&-&l, &-&f), -1, "-L_{n} - F_{n} sqrt 5");
    }
}

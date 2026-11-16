//! The counter-addressed generator `sha256-counter-draft-1` (DET.61). It has no host entropy and no scheduling
//! input: a stream is addressed by its seed and path, and draw `i` is a hash of the stream key and `i`.

use c600_digest::sha256;

/// The domain prefix of every stream key.
pub const DOMAIN: &[u8] = b"C600-LP1-RNG\0";

/// One past the last counter, 2^64. A cursor at this counter is exhausted.
pub const END: u128 = 1 << 64;

/// A generator cursor: its stream key and the counter of its next draw. Each owner holds its own cursor; an alias
/// copies it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cursor {
    pub key: [u8; 32],
    pub counter: u128,
}

/// The stream key of `seed` and `path`, and the length of the hashed input, which `rng_init`'s tariff prices. The
/// result is `None` for invalid input (`LG4:input.generator_input`): a seed is exactly 32 bytes, and the path's
/// segment count and each segment's UTF-8 length fit in 32 bits.
pub fn stream_key(seed: &[u8], path: &[&str]) -> Option<([u8; 32], usize)> {
    if seed.len() != 32 {
        return None;
    }
    let mut data = DOMAIN.to_vec();
    data.extend_from_slice(seed);
    data.extend_from_slice(&u32::try_from(path.len()).ok()?.to_be_bytes());
    for segment in path {
        data.extend_from_slice(&u32::try_from(segment.len()).ok()?.to_be_bytes());
        data.extend_from_slice(segment.as_bytes());
    }
    Some((sha256(&data), data.len()))
}

/// The input length `rng_init` prices for a seed of `seed_len` bytes and the given path, whether or not it is valid.
pub fn key_input_len(seed_len: usize, path: &[&str]) -> u128 {
    let segments: u128 = path.iter().map(|s| 4 + s.len() as u128).sum();
    DOMAIN.len() as u128 + seed_len as u128 + 4 + segments
}

impl Cursor {
    pub fn new(key: [u8; 32]) -> Cursor {
        Cursor { key, counter: 0 }
    }

    /// The next block as an unsigned big-endian integer's bytes, or `None` once the cursor is exhausted. Only a
    /// successful draw advances the counter; there is no wraparound.
    pub fn draw(&mut self) -> Option<[u8; 32]> {
        if self.counter >= END {
            return None;
        }
        let mut data = self.key.to_vec();
        data.extend_from_slice(&(self.counter as u64).to_be_bytes());
        self.counter += 1;
        Some(sha256(&data))
    }
}

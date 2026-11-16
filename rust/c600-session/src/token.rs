//! User-action tokens (KN5.14, KN5.15). The shell stub mints a token for one invocation. Its authenticator is
//! HMAC-SHA256 under a per-launch secret over the token's binding and a nonce that never repeats in the launch. The
//! secret stays in the engine process; no runtime operation returns a token. A token is single-use and never
//! persisted.

use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hasher};
use std::time::{SystemTime, UNIX_EPOCH};

use c600_digest::sha256;
use c600_kernel::IdentityBlock;
use c600_run::provenance::{canonical, Json};

use crate::wire::{identity, num};

/// HMAC-SHA256 (RFC 2104) over c600-digest's SHA-256.
pub fn hmac_sha256(key: &[u8], message: &[u8]) -> [u8; 32] {
    const BLOCK: usize = 64;
    let mut padded = [0u8; BLOCK];
    if key.len() > BLOCK {
        padded[..32].copy_from_slice(&sha256(key));
    } else {
        padded[..key.len()].copy_from_slice(key);
    }
    let mut inner = Vec::with_capacity(BLOCK + message.len());
    inner.extend(padded.iter().map(|b| b ^ 0x36));
    inner.extend_from_slice(message);
    let inner = sha256(&inner);
    let mut outer = Vec::with_capacity(BLOCK + 32);
    outer.extend(padded.iter().map(|b| b ^ 0x5c));
    outer.extend_from_slice(&inner);
    sha256(&outer)
}

/// The per-launch secret. M6 draws it from std's randomly keyed hasher, the time and the process ID; a product engine
/// takes it from the operating system's generator (a gap recorded in `rust/README.md`).
pub(crate) struct Secret([u8; 32]);

impl Secret {
    pub(crate) fn fresh() -> Secret {
        let nanos = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
        let mut seed = Vec::with_capacity(64);
        for i in 0..4u64 {
            let mut hasher = RandomState::new().build_hasher();
            hasher.write_u64(i);
            hasher.write_u128(nanos);
            hasher.write_u32(std::process::id());
            seed.extend_from_slice(&hasher.finish().to_le_bytes());
        }
        seed.extend_from_slice(&nanos.to_le_bytes());
        Secret(sha256(&seed))
    }

    /// A value derived from the secret for another purpose, such as the epoch number.
    pub(crate) fn derive(&self, label: &str) -> [u8; 32] {
        hmac_sha256(&self.0, label.as_bytes())
    }

    pub(crate) fn mac(&self, binding: &Binding, nonce: u64) -> [u8; 32] {
        hmac_sha256(&self.0, &message(binding, nonce))
    }

    /// Checks a token's authenticator in constant time over its 32 bytes.
    pub(crate) fn verify(&self, token: &Token) -> bool {
        let expected = self.mac(&token.binding, token.nonce);
        expected.iter().zip(token.mac.iter()).fold(0u8, |acc, (a, b)| acc | (a ^ b)) == 0
    }
}

/// The preview a commit token names: its ID and the digest of its literal letters (KN5.14).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreviewBinding {
    pub id: u64,
    pub literal: String,
}

/// What a token binds: the invocation, the session, the epoch, the row and its absolute arguments, the identity, the
/// revisions the action was resolved against and, for `operation.execute`, the preview on screen.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Binding {
    pub session: String,
    pub epoch: u64,
    pub command: String,
    pub row: &'static str,
    /// The digest of the row's canonical arguments.
    pub args: String,
    pub identity: IdentityBlock,
    /// The `state` and `protection` revisions the action was resolved against (LI.6). A word commit binds its
    /// preview's own dependencies.
    pub state: u64,
    pub protection: u64,
    pub preview: Option<PreviewBinding>,
}

impl Binding {
    pub(crate) fn json(&self) -> Json {
        let preview = match &self.preview {
            Some(p) => Json::object([("id", num(p.id)), ("literal", Json::text(&p.literal))]),
            None => Json::Null,
        };
        Json::object([
            ("session", Json::text(&self.session)),
            ("epoch", num(self.epoch)),
            ("command", Json::text(&self.command)),
            ("row", Json::text(self.row)),
            ("args", Json::text(&self.args)),
            ("identity", identity(&self.identity)),
            ("state", num(self.state)),
            ("protection", num(self.protection)),
            ("preview", preview),
        ])
    }
}

/// A user-action token.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Token {
    pub binding: Binding,
    pub nonce: u64,
    pub mac: [u8; 32],
}

impl Token {
    pub(crate) fn json(&self) -> Json {
        Json::object([
            ("binding", self.binding.json()),
            ("nonce", num(self.nonce)),
            ("mac", Json::Text(c600_run::provenance::hex(&self.mac))),
        ])
    }
}

fn message(binding: &Binding, nonce: u64) -> Vec<u8> {
    canonical(&Json::object([("binding", binding.json()), ("nonce", num(nonce))]))
}

#[cfg(test)]
mod tests {
    use super::hmac_sha256;

    fn hex(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{b:02x}")).collect()
    }

    /// RFC 4231 test cases 2 and 6 (a short key and a key longer than the block).
    #[test]
    fn hmac_matches_rfc_4231() {
        assert_eq!(
            hex(&hmac_sha256(b"Jefe", b"what do ya want for nothing?")),
            "5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843"
        );
        assert_eq!(
            hex(&hmac_sha256(&[0xaa; 131], b"Test Using Larger Than Block-Size Key - Hash Key First")),
            "60e431591ee0b67f0d8a26aacbf5b77f8e0bc6213728c5140546040f0ee37f54"
        );
    }
}

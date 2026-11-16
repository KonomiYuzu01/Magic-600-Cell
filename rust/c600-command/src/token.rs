//! User-action tokens (KN5.14 to KN5.16; LI.6).
//!
//! The shell mints a token for one press from a per-launch secret; the engine checks its authenticator against the
//! same secret. The runtime never receives a token or the secret (KN5.15): nothing in this crate hands either to a
//! [`crate::Caller::Runtime`]. A token binds the session, the epoch, the invocation, the row, the absolute arguments
//! and the model identity. A word commit's token also binds the presented preview's ID, its literal digest and its
//! dependencies; an undo or redo token binds the action hint (LI.6).

use std::collections::hash_map::RandomState;
use std::fmt;
use std::hash::{BuildHasher, Hasher};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use c600_digest::sha256;
use c600_kernel::json::{self, Value};

use crate::receipt::{digest, ActionHint, PreviewDeps};

/// The per-launch secret. Its bytes never leave this module; `Debug` does not show them.
#[derive(Clone)]
pub struct Secret([u8; 32]);

impl fmt::Debug for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Secret(..)")
    }
}

static LAUNCHES: AtomicU64 = AtomicU64::new(0);

impl Secret {
    /// A fresh secret for this launch, from the standard library's random hasher keys, the clock, the process ID and
    /// a counter, hashed with SHA-256. This is a headless draft; a product build takes an operating-system source.
    pub fn launch() -> Secret {
        let mut seed = Vec::with_capacity(64);
        for _ in 0..4 {
            let mut hasher = RandomState::new().build_hasher();
            hasher.write_u64(LAUNCHES.fetch_add(1, Ordering::Relaxed));
            seed.extend_from_slice(&hasher.finish().to_le_bytes());
        }
        let now = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_nanos());
        seed.extend_from_slice(&now.to_le_bytes());
        seed.extend_from_slice(&std::process::id().to_le_bytes());
        Secret(sha256(&seed))
    }

    /// A fixed secret, for tests.
    pub fn fixed(bytes: [u8; 32]) -> Secret {
        Secret(bytes)
    }

    /// HMAC-SHA-256 under this secret.
    fn mac(&self, message: &[u8]) -> [u8; 32] {
        let mut inner = Vec::with_capacity(64 + message.len());
        let mut outer = Vec::with_capacity(64 + 32);
        let mut key = [0u8; 64];
        key[..32].copy_from_slice(&self.0);
        inner.extend(key.iter().map(|b| b ^ 0x36));
        inner.extend_from_slice(message);
        outer.extend(key.iter().map(|b| b ^ 0x5c));
        outer.extend_from_slice(&sha256(&inner));
        sha256(&outer)
    }
}

/// The preview a word commit's token names.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreviewBinding {
    pub id: u64,
    /// `Digest<literal word>` of the preview.
    pub literal: String,
    pub deps: PreviewDeps,
}

/// What a token binds (LI.6).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TokenBinding {
    /// Unique per launch; a used nonce is `KN5:stale.token_used`.
    pub nonce: u64,
    pub session: String,
    pub epoch: u64,
    /// The invocation's command ID.
    pub command: String,
    pub row: String,
    /// The digest of the absolute arguments.
    pub args: String,
    /// The model ID.
    pub identity: String,
    pub preview: Option<PreviewBinding>,
    /// The state and protection revisions of a commit without a preview.
    pub revision: Option<(u64, u64)>,
    pub hint: Option<ActionHint>,
}

impl TokenBinding {
    /// The binding's canonical JSON. It enters the request digest; the authenticator does not (LI.5).
    pub fn to_json(&self) -> Value {
        let nat = |n: u64| Value::Int(n as i64);
        json::object([
            ("nonce", nat(self.nonce)),
            ("session", Value::String(self.session.clone())),
            ("epoch", nat(self.epoch)),
            ("command", Value::String(self.command.clone())),
            ("row", Value::String(self.row.clone())),
            ("args", Value::String(self.args.clone())),
            ("identity", Value::String(self.identity.clone())),
            (
                "preview",
                self.preview.as_ref().map_or(Value::Null, |p| {
                    json::object([
                        ("id", nat(p.id)),
                        ("literal", Value::String(p.literal.clone())),
                        ("deps", p.deps.to_json()),
                    ])
                }),
            ),
            (
                "revision",
                self.revision.map_or(Value::Null, |(state, protection)| {
                    json::object([("state", nat(state)), ("protection", nat(protection))])
                }),
            ),
            ("hint", self.hint.as_ref().map_or(Value::Null, ActionHint::to_json)),
        ])
    }
}

/// A minted token.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Token {
    pub binding: TokenBinding,
    mac: [u8; 32],
}

impl Token {
    pub fn mint(secret: &Secret, binding: TokenBinding) -> Token {
        let mac = secret.mac(binding.to_json().canonical().as_bytes());
        Token { binding, mac }
    }

    /// Whether the authenticator matches the binding under `secret`, compared in constant time.
    pub fn verify(&self, secret: &Secret) -> bool {
        let expected = secret.mac(self.binding.to_json().canonical().as_bytes());
        expected.iter().zip(&self.mac).fold(0u8, |acc, (a, b)| acc | (a ^ b)) == 0
    }
}

/// The digest of absolute arguments, as a token binds them.
pub fn args_digest(args: &Value) -> String {
    digest(args)
}

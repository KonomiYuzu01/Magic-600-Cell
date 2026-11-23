//! The module context a request names (EV1.1): the edition, the session identity and the draft manifest's claims
//! (LG5.2, LG5.5), the digest-pinned dependency bytes (LG5.1, LG5.3) and the obligations the host requires (EV1.4).
//!
//! The context is data the host supplies, in the editor's own plain types: this crate reads no manifest or lock file
//! and has no path to `c600-module`, whose forms are LP4-2's. A dependency is named by the SHA-256 of its bytes, which
//! the context computes, so different bytes are always a different dependency and never replace an addressed one.

use std::collections::BTreeMap;

use c600_syntax::{sha256_hex, EDITION};

/// A model identity in KN0.1's field names (FP.11): `puzzle` and `model_id`, and the three jumbling-only fields,
/// absent for 600-cell-Full.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ModelIdentity {
    pub puzzle: String,
    pub model_id: String,
    pub menu_id: Option<String>,
    pub menu_order: Option<String>,
    pub contract_rev: Option<String>,
}

/// One field of a model identity, in KN0.1's order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum IdentityField {
    Puzzle,
    ModelId,
    MenuId,
    MenuOrder,
    ContractRev,
}

impl IdentityField {
    pub const ALL: [IdentityField; 5] = [
        IdentityField::Puzzle,
        IdentityField::ModelId,
        IdentityField::MenuId,
        IdentityField::MenuOrder,
        IdentityField::ContractRev,
    ];

    /// KN0.1's field name.
    pub fn name(self) -> &'static str {
        match self {
            IdentityField::Puzzle => "puzzle",
            IdentityField::ModelId => "model_id",
            IdentityField::MenuId => "menu_id",
            IdentityField::MenuOrder => "menu_order",
            IdentityField::ContractRev => "contract_rev",
        }
    }
}

impl ModelIdentity {
    /// The value of one field.
    pub fn field(&self, field: IdentityField) -> Option<&str> {
        match field {
            IdentityField::Puzzle => Some(&self.puzzle),
            IdentityField::ModelId => Some(&self.model_id),
            IdentityField::MenuId => self.menu_id.as_deref(),
            IdentityField::MenuOrder => self.menu_order.as_deref(),
            IdentityField::ContractRev => self.contract_rev.as_deref(),
        }
    }

    /// The first field, in KN0.1's order, where the two identities differ. Matching a model digest alone does not
    /// waive a differing menu order or contract revision (LG5.5).
    pub fn first_difference(&self, other: &ModelIdentity) -> Option<IdentityField> {
        IdentityField::ALL.into_iter().find(|f| self.field(*f) != other.field(*f))
    }

    fn canonical(&self) -> String {
        IdentityField::ALL.iter().map(|f| field(self.field(*f))).collect::<Vec<_>>().join(",")
    }
}

/// A required claim or check (EV1.4).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ObligationRole {
    /// The declaration supplies evidence: an exact evidence sink, `use_evidence`, in its body or in a function it
    /// calls (LG2.20; the checker's summary). The IR draft has no claim types yet, so this is the draft's one
    /// obligation (`rust/README.md`).
    Evidence,
}

impl ObligationRole {
    pub fn name(self) -> &'static str {
        match self {
            ObligationRole::Evidence => "evidence",
        }
    }
}

/// An obligation the host requires of one declaration of the draft.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Obligation {
    pub declaration: String,
    pub role: ObligationRole,
}

/// A digest-pinned dependency: its bytes and, when it carries a manifest of its own, the identities it claims.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Dependency {
    pub digest: String,
    pub bytes: Vec<u8>,
    /// `None` when the dependency has no manifest of its own; its source header then declares what it needs.
    pub supports: Option<Vec<ModelIdentity>>,
}

/// What a request checks against besides the source bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModuleContext {
    /// The edition the manifest requires of the draft's files.
    pub edition: String,
    /// The session identity the draft is checked for; `None` for a model-independent request.
    pub session: Option<ModelIdentity>,
    /// The draft manifest's supported identities; `None` when the draft has no manifest yet.
    pub supports: Option<Vec<ModelIdentity>>,
    pub obligations: Vec<Obligation>,
    dependencies: BTreeMap<String, Dependency>,
}

impl Default for ModuleContext {
    fn default() -> Self {
        ModuleContext::new()
    }
}

impl ModuleContext {
    /// A context for this parser's edition with no identity, claims, obligations or dependencies.
    pub fn new() -> ModuleContext {
        ModuleContext {
            edition: EDITION.to_owned(),
            session: None,
            supports: None,
            obligations: Vec::new(),
            dependencies: BTreeMap::new(),
        }
    }

    /// Adds dependency bytes, named by their digest, and returns the digest.
    pub fn add_dependency(&mut self, bytes: impl Into<Vec<u8>>, supports: Option<Vec<ModelIdentity>>) -> String {
        let bytes = bytes.into();
        let digest = sha256_hex(&bytes);
        self.dependencies.insert(digest.clone(), Dependency { digest: digest.clone(), bytes, supports });
        digest
    }

    pub fn with_dependency(mut self, bytes: impl Into<Vec<u8>>, supports: Option<Vec<ModelIdentity>>) -> Self {
        self.add_dependency(bytes, supports);
        self
    }

    pub fn with_session(mut self, session: ModelIdentity, supports: Vec<ModelIdentity>) -> Self {
        self.session = Some(session);
        self.supports = Some(supports);
        self
    }

    pub fn with_obligation(mut self, declaration: &str, role: ObligationRole) -> Self {
        self.obligations.push(Obligation { declaration: declaration.to_owned(), role });
        self
    }

    pub fn dependency(&self, digest: &str) -> Option<&Dependency> {
        self.dependencies.get(digest)
    }

    /// The dependencies in digest order.
    pub fn dependencies(&self) -> impl Iterator<Item = &Dependency> {
        self.dependencies.values()
    }

    /// The SHA-256 of the context's canonical text: every field, each value length-prefixed, the dependencies by
    /// digest. Two contexts with the same digest hold the same data.
    pub fn digest(&self) -> String {
        let mut text = format!("c600-editor-context-draft-1\nedition {}\n", field(Some(&self.edition)));
        let session = self.session.as_ref().map(ModelIdentity::canonical);
        text.push_str(&format!("session {}\n", field(session.as_deref())));
        text.push_str(&format!("supports {}\n", claims(self.supports.as_deref())));
        for d in self.dependencies.values() {
            text.push_str(&format!("dependency {} {}\n", d.digest, claims(d.supports.as_deref())));
        }
        let mut obligations = self.obligations.clone();
        obligations.sort();
        for o in obligations {
            text.push_str(&format!("obligation {} {}\n", field(Some(&o.declaration)), o.role.name()));
        }
        sha256_hex(text.as_bytes())
    }
}

/// A value as `<length>:<text>`, or `-` when absent.
fn field(value: Option<&str>) -> String {
    value.map_or_else(|| "-".to_owned(), |v| format!("{}:{v}", v.len()))
}

fn claims(supports: Option<&[ModelIdentity]>) -> String {
    match supports {
        None => "-".to_owned(),
        Some(list) => {
            let mut list: Vec<String> = list.iter().map(ModelIdentity::canonical).collect();
            list.sort();
            format!("[{}]", list.join(";"))
        }
    }
}

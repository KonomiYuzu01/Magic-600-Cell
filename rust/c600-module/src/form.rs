//! The stored forms: the module manifest, the lockfile and the model identity they name (FP.4, FP.5, FP.10, FP.11,
//! PS1.15). Reading checks FP.10's format and version dispatch first, then FP.5's canonical bytes, then the fields.

use std::collections::BTreeMap;

use c600_digest::sha256;
use c600_run::menu::MenuIdentity;
use c600_run::provenance::{self, digest_ref, Json};

use crate::code::{CONTRACT_MISMATCH, NONCANONICAL_VALUE, PERSISTENCE_FORMAT, PERSISTENCE_VERSION};
use crate::{refuse, Refusal, FORM_VERSION, FULL_MODEL_ID, LOCK_FORMAT, MANIFEST_FORMAT};

const RECORD_FIELDS: [&str; 6] = ["extensions", "format", "identity", "payload", "provenance", "version"];
const IDENTITY_FIELDS: [&str; 5] = ["contract_rev", "menu_id", "menu_order", "model_id", "puzzle"];
const MANIFEST_FIELDS: [&str; 9] =
    ["boundary", "capabilities", "dependencies", "edition", "module", "parameters", "principle", "source", "supports"];
const LOCK_FIELDS: [&str; 4] = ["locations", "modules", "resolver", "root"];

/// A model identity in KN0.1's field names (FP.11): `puzzle` and `model_id`, and the three jumbling-only fields, which
/// are null for Full and non-null for a jumbling puzzle.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Identity(Json);

impl Identity {
    /// 600-cell-Full's identity block.
    pub fn full() -> Identity {
        Identity(Json::object([
            ("contract_rev", Json::Null),
            ("menu_id", Json::Null),
            ("menu_order", Json::Null),
            ("model_id", Json::text(FULL_MODEL_ID)),
            ("puzzle", Json::text("full")),
        ]))
    }

    /// An identity block from its JSON value, checked as FP.11 states.
    pub fn from_json(value: &Json) -> Result<Identity, Refusal> {
        let m = fields(value, &IDENTITY_FIELDS, "an identity block")?;
        let puzzle = text(&m["puzzle"], "an identity's `puzzle`")?;
        if unhex(text(&m["model_id"], "an identity's `model_id`")?).is_none() {
            return Err(refuse(PERSISTENCE_FORMAT, "an identity's `model_id` is a lowercase hex digest (FP.11)"));
        }
        let nulls = ["contract_rev", "menu_id", "menu_order"].iter().filter(|k| m[**k] == Json::Null).count();
        if (puzzle == "full" && nulls != 3) || (puzzle != "full" && nulls != 0) {
            let detail = "the three jumbling-only identity fields are null for Full and non-null for jumbling (FP.11)";
            return Err(refuse(PERSISTENCE_FORMAT, detail));
        }
        Ok(Identity(value.clone()))
    }

    pub fn json(&self) -> &Json {
        &self.0
    }

    /// The first field in which two identity blocks differ, in KN1.4's order: `puzzle`, `model_id`, `menu_id`,
    /// `menu_order`, then `contract_rev`. `None` when they are one identity (LG5.5, KN1.4; LP2 draft).
    pub fn differing(&self, other: &Identity) -> Option<&'static str> {
        KN14_ORDER.into_iter().find(|k| self.field(k) != other.field(k))
    }

    /// The first field, in KN1.4's order, in which this identity differs from a menu alphabet's identity block. A
    /// field that is null here (Full) never equals the menu's text, so a menu pinned under a Full identity differs
    /// in `menu_id` or earlier (LG5.5, KN1.4; LP2 draft).
    pub fn differing_menu(&self, menu: &MenuIdentity) -> Option<&'static str> {
        menu.fields().into_iter().find(|(k, v)| self.field(k) != Some(*v)).map(|(k, _)| k)
    }

    fn field(&self, key: &str) -> Option<&str> {
        match &self.0 {
            Json::Object(m) => m.get(key).and_then(Json::as_text),
            _ => None,
        }
    }
}

/// KN1.4's order of the identity block's fields.
const KN14_ORDER: [&str; 5] = ["puzzle", "model_id", "menu_id", "menu_order", "contract_rev"];

/// A direct import of the root module: its alias and its source digest (LG5.2).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Dependency {
    pub alias: String,
    pub source: [u8; 32],
}

/// A module manifest (LG5.2, PS1.15): a declaration of compatibility and requirements. Declaring a capability,
/// boundary class or principle record grants nothing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Manifest {
    /// The root module's name, as its source header declares it.
    pub module: String,
    pub edition: String,
    /// The SHA-256 of the root module's source bytes.
    pub source: [u8; 32],
    pub dependencies: Vec<Dependency>,
    /// The capability names the root header requires.
    pub capabilities: Vec<String>,
    /// A built-in tool's reviewed boundary class (BND.37); data for the policy, never a grant.
    pub boundary: Option<String>,
    /// The digest of a built-in tool's principle record (chapter 03), when it has one.
    pub principle: Option<[u8; 32]>,
    pub supports: Vec<Identity>,
}

impl Manifest {
    pub fn json(&self) -> Json {
        let dependency = |d: &Dependency| {
            Json::object([("alias", Json::text(&d.alias)), ("source", digest_ref("source", &d.source))])
        };
        let payload = Json::object([
            ("boundary", self.boundary.as_deref().map_or(Json::Null, Json::text)),
            ("capabilities", set_of(self.capabilities.iter().map(|c| Json::text(c)))),
            ("dependencies", set_of(self.dependencies.iter().map(dependency))),
            ("edition", Json::text(&self.edition)),
            ("module", Json::text(&self.module)),
            ("parameters", Json::List(Vec::new())),
            ("principle", self.principle.map_or(Json::Null, |d| digest_ref("content", &d))),
            ("source", digest_ref("source", &self.source)),
            ("supports", set_of(self.supports.iter().map(|i| i.0.clone()))),
        ]);
        envelope(MANIFEST_FORMAT, payload)
    }

    /// The manifest's canonical bytes (FP.5).
    pub fn bytes(&self) -> Vec<u8> {
        provenance::canonical(&self.json())
    }

    pub fn read(bytes: &[u8]) -> Result<Manifest, Refusal> {
        let json = parse(bytes)?;
        let payload = record(&json, bytes, MANIFEST_FORMAT)?;
        let m = fields(payload, &MANIFEST_FIELDS, "the manifest payload")?;
        let mut dependencies = Vec::new();
        for d in set(&m["dependencies"], "the manifest's `dependencies`")? {
            let d = fields(d, &["alias", "source"], "a dependency")?;
            let alias = text(&d["alias"], "a dependency's `alias`")?.to_string();
            dependencies.push(Dependency { alias, source: digest(&d["source"], "source", "a dependency's source")? });
        }
        let mut capabilities = Vec::new();
        for c in set(&m["capabilities"], "the manifest's `capabilities`")? {
            capabilities.push(text(c, "a capability")?.to_string());
        }
        let boundary = match &m["boundary"] {
            Json::Null => None,
            b => Some(text(b, "the manifest's `boundary`")?.to_string()),
        };
        let principle = match &m["principle"] {
            Json::Null => None,
            p => Some(digest(p, "content", "the manifest's `principle`")?),
        };
        if m["parameters"] != Json::List(Vec::new()) {
            let detail = "declared parameters are not drafted yet; this reader accepts an empty list only (PS1.15)";
            return Err(refuse(CONTRACT_MISMATCH, detail));
        }
        let mut supports = Vec::new();
        for i in set(&m["supports"], "the manifest's `supports`")? {
            supports.push(Identity::from_json(i)?);
        }
        Ok(Manifest {
            module: text(&m["module"], "the manifest's `module`")?.to_string(),
            edition: text(&m["edition"], "the manifest's `edition`")?.to_string(),
            source: digest(&m["source"], "source", "the manifest's `source`")?,
            dependencies,
            capabilities,
            boundary,
            principle,
            supports,
        })
    }
}

/// One module of a locked closure: its name, source digest, edition and IR contract (LG5.3).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LockedModule {
    pub module: String,
    pub source: [u8; 32],
    pub edition: String,
    pub ir_version: String,
    pub exec_contract: String,
}

/// A local resolution entry: where the bytes of a digest were found. It locates bytes only (LG5.1).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Location {
    pub path: String,
    pub source: [u8; 32],
}

/// A lockfile (LG5.3, PS1.15): the resolved closure of one manifest, fixed by digest.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Lock {
    /// The SHA-256 of the manifest bytes this lock resolves.
    pub root: [u8; 32],
    pub modules: Vec<LockedModule>,
    pub locations: Vec<Location>,
    pub resolver: String,
}

impl Lock {
    fn payload(&self, locations: bool) -> Json {
        let module = |m: &LockedModule| {
            Json::object([
                ("edition", Json::text(&m.edition)),
                (
                    "ir",
                    Json::object([("contract", Json::text(&m.exec_contract)), ("version", Json::text(&m.ir_version))]),
                ),
                ("module", Json::text(&m.module)),
                ("source", digest_ref("source", &m.source)),
            ])
        };
        let location =
            |l: &Location| Json::object([("path", Json::text(&l.path)), ("source", digest_ref("source", &l.source))]);
        let mut payload = BTreeMap::new();
        payload.insert("modules".to_string(), set_of(self.modules.iter().map(module)));
        payload.insert("resolver".to_string(), Json::object([("contract", Json::text(&self.resolver))]));
        payload.insert("root".to_string(), digest_ref("content", &self.root));
        if locations {
            payload.insert("locations".to_string(), set_of(self.locations.iter().map(location)));
        }
        Json::Object(payload)
    }

    pub fn json(&self) -> Json {
        envelope(LOCK_FORMAT, self.payload(true))
    }

    /// The lock's canonical bytes (FP.5).
    pub fn bytes(&self) -> Vec<u8> {
        provenance::canonical(&self.json())
    }

    /// The closure digest: the SHA-256 of the canonical payload without its locations (LG5.1, LG5.3, LG5.4).
    pub fn closure_digest(&self) -> [u8; 32] {
        sha256(&provenance::canonical(&self.payload(false)))
    }

    pub fn read(bytes: &[u8]) -> Result<Lock, Refusal> {
        let json = parse(bytes)?;
        let payload = record(&json, bytes, LOCK_FORMAT)?;
        let m = fields(payload, &LOCK_FIELDS, "the lock payload")?;
        let mut modules = Vec::new();
        for e in set(&m["modules"], "the lock's `modules`")? {
            let e = fields(e, &["edition", "ir", "module", "source"], "a locked module")?;
            let ir = fields(&e["ir"], &["contract", "version"], "a locked module's `ir`")?;
            modules.push(LockedModule {
                module: text(&e["module"], "a locked module's name")?.to_string(),
                source: digest(&e["source"], "source", "a locked module's source")?,
                edition: text(&e["edition"], "a locked module's edition")?.to_string(),
                ir_version: text(&ir["version"], "a locked module's IR version")?.to_string(),
                exec_contract: text(&ir["contract"], "a locked module's IR contract")?.to_string(),
            });
        }
        let mut locations = Vec::new();
        for l in set(&m["locations"], "the lock's `locations`")? {
            let l = fields(l, &["path", "source"], "a location")?;
            let path = text(&l["path"], "a location's `path`")?.to_string();
            locations.push(Location { path, source: digest(&l["source"], "source", "a location's source")? });
        }
        let resolver = fields(&m["resolver"], &["contract"], "the lock's `resolver`")?;
        Ok(Lock {
            root: digest(&m["root"], "content", "the lock's `root`")?,
            modules,
            locations,
            resolver: text(&resolver["contract"], "the resolver contract")?.to_string(),
        })
    }
}

fn envelope(format: &str, payload: Json) -> Json {
    Json::object([
        ("format", Json::text(format)),
        ("identity", Json::Null),
        ("payload", payload),
        ("provenance", Json::Null),
        ("version", Json::Int(FORM_VERSION)),
    ])
}

fn parse(bytes: &[u8]) -> Result<Json, Refusal> {
    provenance::parse(bytes)
        .ok_or_else(|| refuse(PERSISTENCE_FORMAT, "the bytes do not read as strict JSON text (FP.5)"))
}

/// FP.10's dispatch, FP.5's canonical check and FP.11's record shape. Gives the payload.
fn record<'a>(json: &'a Json, bytes: &[u8], format: &str) -> Result<&'a Json, Refusal> {
    let (Some(f), Some(v)) = (json.get("format"), json.get("version")) else {
        return Err(refuse(PERSISTENCE_FORMAT, "a record header without `format` or `version` (FP.10)"));
    };
    if f.as_text() != Some(format) || *v != Json::Int(FORM_VERSION) {
        let detail =
            format!("an unknown format/version pair; this reader knows `{format}` version {FORM_VERSION} (FP.10)");
        return Err(refuse(PERSISTENCE_VERSION, detail));
    }
    if provenance::canonical(json) != bytes {
        return Err(refuse(NONCANONICAL_VALUE, "the bytes are not FP.5's canonical form"));
    }
    let Json::Object(m) = json else { unreachable!("a record with a header is an object") };
    if let Some(key) = m.keys().find(|k| !RECORD_FIELDS.contains(&k.as_str())) {
        return Err(refuse(PERSISTENCE_FORMAT, format!("unknown record field `{key}` (FP.10, FP.11)")));
    }
    if let Some(key) = ["identity", "payload", "provenance"].iter().find(|k| !m.contains_key(**k)) {
        return Err(refuse(PERSISTENCE_FORMAT, format!("missing record field `{key}` (FP.11)")));
    }
    if m["identity"] != Json::Null {
        let detail = "the draft manifest and lock are not model-dependent: `identity` is null and `payload.supports` \
                      names the model identities (draft)";
        return Err(refuse(PERSISTENCE_FORMAT, detail));
    }
    if m["provenance"] != Json::Null {
        return Err(refuse(PERSISTENCE_FORMAT, "a manifest or lock is no word, macro or workspace record (FP.11)"));
    }
    if let Some(extensions) = m.get("extensions") {
        for e in set(extensions, "`extensions`")? {
            let e = fields(e, &["name", "required", "value"], "an extension")?;
            match e["required"] {
                Json::Bool(false) => {}
                Json::Bool(true) => {
                    let name = text(&e["name"], "an extension's `name`")?;
                    let detail = format!("the required extension `{name}` is unknown to this reader (FP.10)");
                    return Err(refuse(PERSISTENCE_VERSION, detail));
                }
                _ => return Err(refuse(PERSISTENCE_FORMAT, "an extension's `required` is a Bool (FP.10)")),
            }
        }
    }
    Ok(&m["payload"])
}

/// An object with exactly these fields.
fn fields<'a>(value: &'a Json, keys: &[&str], what: &str) -> Result<&'a BTreeMap<String, Json>, Refusal> {
    match value {
        Json::Object(m) if m.len() == keys.len() && keys.iter().all(|k| m.contains_key(*k)) => Ok(m),
        _ => Err(refuse(PERSISTENCE_FORMAT, format!("{what} has exactly the fields {} (FP.10)", keys.join(", ")))),
    }
}

fn text<'a>(value: &'a Json, what: &str) -> Result<&'a str, Refusal> {
    value.as_text().ok_or_else(|| refuse(PERSISTENCE_FORMAT, format!("{what} is Text (FP.11)")))
}

/// A `DigestRef(role)` (FP.11).
fn digest(value: &Json, role: &str, what: &str) -> Result<[u8; 32], Refusal> {
    let m = fields(value, &["role", "sha256"], what)?;
    if m["role"].as_text() != Some(role) {
        return Err(refuse(PERSISTENCE_FORMAT, format!("{what} is a DigestRef({role}) (FP.11)")));
    }
    unhex(text(&m["sha256"], what)?)
        .ok_or_else(|| refuse(PERSISTENCE_FORMAT, format!("{what} names a lowercase hex SHA-256 (FP.11)")))
}

/// A set: a list sorted by FP.5 element bytes, without duplicates (FP.11).
fn set<'a>(value: &'a Json, what: &str) -> Result<&'a [Json], Refusal> {
    let Json::List(items) = value else {
        return Err(refuse(PERSISTENCE_FORMAT, format!("{what} is a list (FP.11)")));
    };
    let keys: Vec<Vec<u8>> = items.iter().map(element).collect();
    if keys.windows(2).any(|w| w[0] >= w[1]) {
        return Err(refuse(
            NONCANONICAL_VALUE,
            format!("{what} is a set: sorted by element bytes, no duplicates (FP.11)"),
        ));
    }
    Ok(items)
}

fn set_of(items: impl Iterator<Item = Json>) -> Json {
    let mut keyed: Vec<(Vec<u8>, Json)> = items.map(|j| (element(&j), j)).collect();
    keyed.sort_by(|a, b| a.0.cmp(&b.0));
    keyed.dedup_by(|a, b| a.0 == b.0);
    Json::List(keyed.into_iter().map(|(_, j)| j).collect())
}

/// An element's FP.5 bytes without the terminal line feed.
fn element(value: &Json) -> Vec<u8> {
    let mut bytes = provenance::canonical(value);
    bytes.pop();
    bytes
}

/// A lowercase hex SHA-256.
pub(crate) fn unhex(s: &str) -> Option<[u8; 32]> {
    if s.len() != 64 || !s.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)) {
        return None;
    }
    let mut out = [0u8; 32];
    for (i, byte) in out.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&s[2 * i..2 * i + 2], 16).ok()?;
    }
    Some(out)
}

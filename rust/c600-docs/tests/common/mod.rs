//! Shared by the conformance tests of `c600-docs`: the shipped subjects (the three built-in tools of PHS.22 and the
//! library's modules), their principle records under `docs/progress/1.0/principle-records/`, the glossary, and the
//! facts a record is admitted against. Every value here is a source or fixture value.

#![allow(dead_code)]

pub mod run;

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use c600_docs::decompose::Decomposition;
use c600_docs::json::Json;
use c600_docs::record::{self, combined_digest, Admission, Generated, Glossary, PrincipleRecord};
use c600_docs::reference::closure;
use c600_docs::Checked;
use c600_jumble::twist::Menu;
use c600_jumble::Context as Engine;
use c600_kernel::Puzzle;
use c600_module::Identity;
use c600_run::provenance::Json as Wire;

/// The folder of the principle records, relative to the repository.
pub const RECORDS: &str = "docs/progress/1.0/principle-records";
/// The glossary's terms, relative to the repository.
pub const TERMS: &str = "docs/progress/1.0/glossary/terms.json";
/// The W-J fixture whose pinned menu the jumbling tool's instance runs under.
pub const MENU_FIXTURE: &str = "research/jumbling/fixtures/wj-I_a.json";

pub fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..")
}

/// The bytes of a repository-relative path, or `None` when there is no such file.
pub fn resolve(path: &str) -> Option<Vec<u8>> {
    if path.contains("..") || path.starts_with('/') || path.contains(':') {
        return None;
    }
    std::fs::read(repository().join(path)).ok()
}

/// The jumbling model of `assets/`, loaded once.
pub fn engine() -> &'static Engine {
    static ENGINE: OnceLock<Engine> = OnceLock::new();
    ENGINE.get_or_init(|| Engine::load(&repository().join("assets")).expect("the jumbling model loads"))
}

/// The 600-cell-Full kernel of `assets/`, loaded once.
pub fn kernel() -> Arc<c600_kernel::Kernel> {
    static KERNEL: OnceLock<Arc<c600_kernel::Kernel>> = OnceLock::new();
    KERNEL
        .get_or_init(|| Arc::new(c600_kernel::Kernel::load(&repository().join("assets")).expect("the model loads")))
        .clone()
}

/// The pinned menu of [`MENU_FIXTURE`].
pub fn menu() -> Arc<Menu> {
    static MENU: OnceLock<Arc<Menu>> = OnceLock::new();
    MENU.get_or_init(|| {
        let bytes = resolve(MENU_FIXTURE).expect("the W-J fixture");
        let doc = c600_kernel::json::parse(&bytes).expect("the fixture is JSON");
        Arc::new(Menu::from_record(engine(), doc.get("menu").expect("a menu")).expect("the fixture's menu"))
    })
    .clone()
}

fn optional(field: &Option<String>) -> Wire {
    field.as_deref().map_or(Wire::Null, Wire::text)
}

/// The identity block of the jumbled puzzle under [`MENU_FIXTURE`]'s menu, in FP.11's field names.
pub fn jumbling_identity() -> Identity {
    let alphabet = c600_jumble_tool::word::Alphabet::of(engine(), &menu()).expect("the menu's alphabet");
    let block = alphabet.identity();
    assert_eq!(block.puzzle, Puzzle::Jumbling);
    let json = Wire::object([
        ("contract_rev", optional(&block.contract_rev)),
        ("menu_id", optional(&block.menu_id)),
        ("menu_order", optional(&block.menu_order)),
        ("model_id", Wire::text(&block.model_id)),
        ("puzzle", Wire::text("jumbling")),
    ]);
    Identity::from_json(&json).expect("an FP.11 identity block")
}

/// A shipped subject: a built-in tool or a library module, with its source, its entries and the identity its record
/// is bound to.
pub struct Subject {
    pub tool: &'static str,
    pub module: &'static str,
    pub source: &'static str,
    /// The exported entries the record specifies.
    pub entries: Vec<String>,
    pub identity: Identity,
}

impl Subject {
    /// The record's data file, relative to the repository.
    pub fn record_path(&self) -> String {
        format!("{RECORDS}/{}.json", self.module)
    }

    /// The identity's canonical text: the record's `model_identity`.
    pub fn model_identity(&self) -> String {
        String::from_utf8(c600_run::provenance::canonical(self.identity.json()))
            .expect("canonical JSON is text")
            .trim_end()
            .to_string()
    }

    pub fn checked(&self) -> Checked {
        Checked::new(&[self.source.as_bytes()]).unwrap_or_else(|b| panic!("{} checks: {:?}", self.module, b.findings))
    }

    pub fn decompositions(&self, checked: &Checked) -> Vec<Decomposition> {
        self.entries.iter().map(|e| checked.decompose(self.module, e).expect("a decomposition")).collect()
    }

    /// The closure digest the record is bound to: the module's manifest with the subject's identity.
    pub fn closure_sha256(&self) -> String {
        closure(self.source.as_bytes(), std::slice::from_ref(&self.identity), None).expect("a closure").1
    }

    pub fn record(&self) -> PrincipleRecord {
        let bytes = resolve(&self.record_path()).unwrap_or_else(|| panic!("{} exists", self.record_path()));
        let (record, findings) = record::read(&bytes);
        assert!(findings.is_empty(), "{}: {findings:#?}", self.record_path());
        record.expect("a record")
    }

    /// Admits the subject's record against its generated facts.
    pub fn admit(&self, record: &PrincipleRecord) -> Admission {
        let checked = self.checked();
        let decompositions = self.decompositions(&checked);
        let generated = Generated {
            source_sha256: c600_digest::sha256_hex(self.source.as_bytes()),
            closure_sha256: self.closure_sha256(),
            decompositions: decompositions.iter().collect(),
        };
        record::admit(record, &generated, &glossary(), &record_names(), &|p| resolve(p))
    }

    /// The facts a record's generated fields hold, as JSON: printed when a record is not current, so that the person
    /// who refreshes it sees what the module now is.
    pub fn facts(&self) -> String {
        let checked = self.checked();
        let decompositions = self.decompositions(&checked);
        let refs: Vec<&Decomposition> = decompositions.iter().collect();
        let labels = record::labels(&refs);
        let kernel: BTreeSet<String> = refs.iter().flat_map(|d| d.kernel_leaves()).collect();
        let list = |items: &BTreeSet<String>| Json::List(items.iter().map(|s| Json::text(s)).collect());
        Json::object([
            ("module", Json::text(self.module)),
            ("source_sha256", Json::text(&c600_digest::sha256_hex(self.source.as_bytes()))),
            ("closure_sha256", Json::text(&self.closure_sha256())),
            ("model_identity", Json::text(&self.model_identity())),
            ("decomposition_sha256", Json::text(&combined_digest(&refs))),
            ("labels", list(&labels)),
            ("kernel_leaves", list(&kernel)),
        ])
        .compact()
    }
}

fn exports(source: &str, module: &str) -> Vec<String> {
    let program = c600_library::lower_source(source).expect("the module lowers");
    let m = program.modules.iter().find(|m| m.name == module).expect("the module");
    m.functions.iter().filter(|f| f.exported).map(|f| f.name.clone()).collect()
}

/// The shipped subjects: the three built-in tools (PHS.22) and the library's twelve modules, in that order.
pub fn subjects() -> Vec<Subject> {
    let full = Identity::full;
    let mut out = vec![
        Subject {
            tool: "600-cell-Full tool",
            module: c600_tools::tool::MODULE,
            source: c600_tools::tool::SOURCE,
            entries: vec![c600_tools::tool::ENTRY.to_string()],
            identity: full(),
        },
        Subject {
            tool: "jumbling tool",
            module: c600_jumble_tool::tool::MODULE,
            source: c600_jumble_tool::tool::SOURCE,
            entries: c600_jumble_tool::tool::ENTRIES.map(String::from).to_vec(),
            identity: jumbling_identity(),
        },
        Subject {
            tool: "representations tool",
            module: c600_repr::tool::MODULE,
            source: c600_repr::tool::SOURCE,
            entries: exports(c600_repr::tool::SOURCE, c600_repr::tool::MODULE),
            identity: full(),
        },
    ];
    use c600_library as l;
    for (module, source) in [
        (l::MODULE, l::SOURCE),
        (l::QUADRATIC_MODULE, l::QUADRATIC_SOURCE),
        (l::WORD_KEYS_MODULE, l::WORD_KEYS_SOURCE),
        (l::LINALG_MODULE, l::LINALG_SOURCE),
        (l::TREES_MODULE, l::TREES_SOURCE),
        (l::RECIPES_MODULE, l::RECIPES_SOURCE),
        (l::COMPLETION_MODULE, l::COMPLETION_SOURCE),
        (l::PROTECTION_MODULE, l::PROTECTION_SOURCE),
        (l::SETUPS_MODULE, l::SETUPS_SOURCE),
        (l::MACROS_MODULE, l::MACROS_SOURCE),
        (l::SEEDS_MODULE, l::SEEDS_SOURCE),
        (l::ORBITS_MODULE, l::ORBITS_SOURCE),
    ] {
        out.push(Subject { tool: "library", module, source, entries: exports(source, module), identity: full() });
    }
    out
}

/// The modules that have a principle record.
pub fn record_names() -> BTreeSet<String> {
    subjects().iter().map(|s| s.module.to_string()).collect()
}

pub fn glossary() -> Glossary {
    Glossary::read(&resolve(TERMS).expect("the glossary")).expect("the glossary reads")
}

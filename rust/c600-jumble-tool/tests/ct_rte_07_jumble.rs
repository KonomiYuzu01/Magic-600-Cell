//! CT-RTE-07, the jumbling keys' M5 leg (RTE.46; LX-3's frozen corpus `lx3-neutral-v2`): MR01, MR02, MR05, MR06,
//! A31 and A32, which `c600-module/tests/ct_rte_07_full.rs` leaves for M5, on the jumbled puzzle's own rows.
//!
//! Each key is re-expressed in `.6cs` and runs ten times through `c600-module`'s boundary, as the Full leg runs its
//! keys: a manifest that supports the jumbled puzzle's identity (I_a's model, menu, menu order and contract revision,
//! FP.11), verified, approved through `c600-check`, loaded and run. The host is the jumbling rows' bridge on a witnessed base
//! (`c600_jumble_tool::rows::Rows::on`), which reads `c600_jumble`. The ten runs give one SHA-256 over the canonical
//! results, the rows each run read, the proposals that reached the host and the store.
//!
//! The corpus's fake rows are M5's real ones, read at bases chosen so that they hold the corpus's values:
//!
//! - `nu2` and `nu5` (MR05, MR06, A31, A32) are coordinates of KN2.4's `nu` row at the I_a state reached from solved
//!   by corrections 0, 27 and 27, each played letter by letter from the shipped catalogue and witnessed by that
//!   journal. There ν has b8 = 1, b25 = 1 and C5_17 = 3 and is 0 elsewhere, so `nu2` = (b8, b34) = (1, 0) and `nu5` =
//!   (C5_28, C5_17) = (0, 3), the corpus's `fixture.json` values. The test checks these against `nu::nu` and the
//!   catalogue's rows.
//! - `counts` (MR01, MR02) are N_c at three caps, read through the bridge's `seam_facts` row at the state of the I_a
//!   walk `wj-I_a.json` after four records, where four admissible caps are misaligned and two of them share the
//!   largest N_c. The caps are literals of the program, chosen by a fixed rule of the test from the engine's rim
//!   summary so that N_c has the corpus's shape [x, y, y] with 0 < x < y, a tied maximum; the corpus names the caps
//!   `c0`, `c1` and `c2`, and so do the program's targets.
//!
//! The corpus's words are literal letters, as in the Full leg: `a, b, c, d` are `+1, +2, +3, +4`. Each key also runs
//! with menu words of the I_a alphabet (LB1.8), the menu-word half: the same program builds and proposes menu words,
//! `a, b, c, d` are I_a's letters `0:1`, `0:2`, `0:3` and `0:4`, and the run pins the alphabet with its item matrices,
//! as `word::Alphabet::pinned` pins it for LG2.18's word keys. Both halves give the corpus's answers.
//!
//! A31 and A32, in research, complete with the corpus's value. MR01, MR02, MR05 and MR06, in solving, propose the word
//! they computed and are refused with the corpus's code by the split at the preview point (LG3.13, BND.39): the
//! proposal's block may read state. The bridge grants no preview, so the leg's host adds a preview point that prices a
//! proposal and records each one that reaches it ([`Watched`]); none does, so no preview is installed, and the base
//! the rows read is never changed (the bridge reads a clone where a row classifies caps).
//!
//! Evidence kind: source and fixture, on M5's real model through the headless engine; nothing here measures time.

mod common;

use std::collections::BTreeMap;
use std::sync::{Arc, OnceLock};

use c600_check::{Grant, Policy};
use c600_digest::sha256;
use c600_jumble::corrections;
use c600_jumble::levels::{Base, Partial, Snapshot};
use c600_jumble::nu::{self, NuVector};
use c600_jumble::observe::cuts::rim_summary;
use c600_jumble::state::{State, Status};
use c600_jumble::twist::{Menu, Twist};
use c600_jumble_tool::rows::Rows;
use c600_jumble_tool::word::Alphabet;
use c600_kernel::json::Value as JsonValue;
use c600_module::code::NOT_GRANTED;
use c600_module::{approve, load, resolve, verify, Entry, Files, Identity, Manifest, Outcome, Refusal, RunRequest};
use c600_run::boundary::{Answer, Pins, Place, Shown, Store};
use c600_run::interp::{Host, Options, Request, RowTariff, Sink};
use c600_run::menu::MenuAlphabet;
use c600_run::meter::{Amounts, Outcome as Ended};
use c600_run::provenance::{self, hex, Author, Json, Pin, Profile};
use c600_run::value::Value;

use common::{engine, repository, Fixture};

const CORPUS: &str = "work/experiments/language-lx/lx3-sandbox-flow/corpus/";
const EDITION: &str = "6cs-draft-1";
const WIDE: Amounts = Amounts { steps: 1 << 40, kernel_cost: 1 << 40, memory: 1 << 40, output: 1 << 40 };

/// The keys realised here, one test each; `ct_rte_07_full.rs` lists them as left for M5 and points here.
const REALISED: [&str; 6] = ["MR01", "MR02", "MR05", "MR06", "A31", "A32"];

/// The fixture letters `a` and `d`.
const A: i32 = 1;
const D: i32 = 4;

/// The fixture letters `a, b, c, d` as letters `[grip, item]` of the I_a alphabet, in the menu-word half.
const MENU: [[u32; 2]; 4] = [[0, 1], [0, 2], [0, 3], [0, 4]];

/// ν's coordinates that the corpus's `nu2` and `nu5` name here: (b8, b34) and (C5_28, C5_17).
const NU2: [usize; 2] = [8, 34];
const NU5: [usize; 2] = [44, 43];

/// The corrections played from solved for the ν keys' base, in order.
const BASE_CORRECTIONS: [u32; 3] = [0, 27, 27];

/// The records of the I_a walk played for MR01's and MR02's state.
const WALK_RECORDS: usize = 4;

// ------------------------------------------------------------------------------------------------------- the bases

/// The pinned I_a menu of `wj-I_a.json`, the catalogue's.
fn i_a() -> Arc<Menu> {
    static I_A: OnceLock<Arc<Menu>> = OnceLock::new();
    I_A.get_or_init(|| Arc::new(Fixture::load("I_a").menu())).clone()
}

/// The I_a alphabet with its item matrices, as `word::Alphabet::pinned` pins it for a run (LG2.18).
fn alphabet() -> &'static MenuAlphabet {
    static PINNED: OnceLock<MenuAlphabet> = OnceLock::new();
    PINNED.get_or_init(|| {
        let alphabet = Alphabet::of(engine(), &i_a()).expect("an inverse-closed menu");
        alphabet.pinned().expect("I_a's alphabet pins with its item matrices")
    })
}

/// `journal` played from solved under `menu`, every letter admissible, with the claim its replay witnesses.
fn witnessed(menu: &Arc<Menu>, journal: &[Twist]) -> Snapshot {
    let mut state = State::new(Some(menu.clone()));
    for (index, twist) in journal.iter().enumerate() {
        let outcome = state.apply(engine(), twist, None, &[]).expect("no engine error");
        assert_eq!(outcome.status, Status::Admissible, "letter {index}");
    }
    Snapshot::witnessed(engine(), state, journal, None).expect("no engine error").expect("the journal witnesses it")
}

/// The ν keys' base: corrections 0, 27 and 27 played letter by letter from solved under I_a.
fn nu_base() -> &'static Snapshot {
    static BASE: OnceLock<Snapshot> = OnceLock::new();
    BASE.get_or_init(|| {
        let catalogue = corrections::catalogue().expect("the catalogue reads");
        let menu = i_a();
        let mut journal = Vec::new();
        for id in BASE_CORRECTIONS {
            journal.extend(catalogue.twists(engine(), &menu, id).expect("no engine error").expect("a shipped word"));
        }
        witnessed(&menu, &journal)
    })
}

/// MR01's and MR02's base: the I_a walk after four records.
fn walk() -> &'static Snapshot {
    static WALK: OnceLock<Snapshot> = OnceLock::new();
    WALK.get_or_init(|| {
        let fixture = Fixture::load("I_a");
        witnessed(&i_a(), &fixture.twists[..WALK_RECORDS])
    })
}

/// The jumbled puzzle's identity block under I_a, as FP.11 writes it in a manifest's `supports`.
fn jumbling() -> Identity {
    let block = engine().identity(Some(&i_a()));
    let field = |f: &Option<String>| Json::text(f.as_deref().expect("a jumbling identity field"));
    let json = Json::object([
        ("contract_rev", field(&block.contract_rev)),
        ("menu_id", field(&block.menu_id)),
        ("menu_order", field(&block.menu_order)),
        ("model_id", Json::text(&block.model_id)),
        ("puzzle", Json::text("jumbling")),
    ]);
    Identity::from_json(&json).expect("FP.11's jumbling identity")
}

// ---------------------------------------------------------------------------------------------------- the harness

fn contract() -> Pin {
    Pin::of("fixture-1", b"p3 fixture capability contract 1\n")
}

fn checker() -> Pin {
    Pin::of("m1-draft", b"c600-check fixture\n")
}

/// A hand-entered value's block.
fn manual() -> Vec<u8> {
    provenance::canonical(&provenance::manual(&contract()))
}

fn encoded(value: &Value) -> Vec<u8> {
    let mut bytes = Vec::new();
    c600_run::encode::encode(value, &mut bytes).expect("a canonical value");
    bytes
}

/// A root module `m` with no import and no required capability, and `body`.
fn root(body: &str) -> String {
    format!("edition \"{EDITION}\";\nmodule m requires [];\n\n{body}")
}

/// The words a key's program builds and proposes: literal letters, as the corpus writes them, or menu words of the
/// I_a alphabet, the menu-word half, whose run pins that alphabet.
#[derive(Clone, Copy)]
enum Words {
    Literal,
    Menu,
}

impl Words {
    /// The type of the words.
    fn ty(self) -> &'static str {
        match self {
            Words::Literal => "Word",
            Words::Menu => "MenuWord",
        }
    }

    /// The source literal of the word of fixture letters `letters` (1 to 4).
    fn literal(self, letters: &[i32]) -> String {
        let written: Vec<String> = match self {
            Words::Literal => letters.iter().map(i32::to_string).collect(),
            Words::Menu => letters.iter().map(|&l| MENU[l as usize - 1]).map(|[g, i]| format!("{g}:{i}")).collect(),
        };
        let kind = match self {
            Words::Literal => "word",
            Words::Menu => "menu",
        };
        format!("{kind}[{}]", written.join(", "))
    }

    /// The value of the word of fixture letters `letters`.
    fn value(self, letters: &[i32]) -> Value {
        match self {
            Words::Literal => Value::word(letters),
            Words::Menu => {
                let letters: Vec<[u32; 2]> = letters.iter().map(|&l| MENU[l as usize - 1]).collect();
                Value::menu_word(alphabet().shared_identity(), &letters).expect("letters below 600")
            }
        }
    }

    /// The alphabet the run pins.
    fn pinned(self) -> Option<&'static MenuAlphabet> {
        match self {
            Words::Literal => None,
            Words::Menu => Some(alphabet()),
        }
    }

    /// A shell row's proposed word in fixture letters: a literal word, or a menu word of the I_a alphabet.
    fn shown(self, shown: &Shown) -> Vec<i32> {
        match self {
            Words::Literal => {
                assert!(shown.menu.is_none(), "a literal proposal");
                shown.word.clone()
            }
            Words::Menu => {
                let menu = shown.menu.as_ref().expect("a menu-word proposal");
                assert!(shown.word.is_empty() && menu.identity() == alphabet().identity(), "of I_a's alphabet");
                let letter = |l: &[u32; 2]| MENU.iter().position(|m| m == l).expect("a fixture letter") as i32 + 1;
                menu.letters().iter().map(letter).collect()
            }
        }
    }
}

/// The leg's engine side: the rows' bridge, which grants no preview, and a preview point that prices a proposal as
/// the Full leg's fixture engine prices a request it prices nowhere (9, bounded by 4096) and records each proposal
/// that reaches it. The split refuses a proposal whose block may read state between the boundary and this host, so a
/// recorded proposal is one the engine would have previewed.
struct Watched<'a> {
    rows: Rows<'a>,
    proposals: usize,
}

impl Host for Watched<'_> {
    fn tariff(&mut self, request: &Request) -> Result<RowTariff, &'static str> {
        match request.is_proposal() {
            true => Ok(RowTariff { kernel_cost: 9, result_bound: 4096 }),
            false => self.rows.tariff(request),
        }
    }

    fn perform(&mut self, request: &Request) -> Result<Value, &'static str> {
        if request.is_proposal() {
            self.proposals += 1;
            return Err(NOT_GRANTED);
        }
        self.rows.perform(request)
    }

    fn sink(&mut self, sink: &Sink, value: &Value, bytes: &[u8]) -> Result<(), &'static str> {
        self.rows.sink(sink, value, bytes)
    }
}

/// One run of a key: `m.go(c0, a)`, both arguments entered by hand, `a` a word of `words`, on `base` under the jumbled
/// identity.
struct Trial {
    result: Result<Outcome, Refusal>,
    asked: Vec<String>,
    proposals: usize,
    store: Store,
}

impl Trial {
    fn run(source: &str, words: Words, profile: Profile, base: &Snapshot) -> Trial {
        let identity = jumbling();
        let files = Files::new().with("src/root.6cs", source.as_bytes());
        let manifest = Manifest {
            module: "m".into(),
            edition: EDITION.into(),
            source: sha256(source.as_bytes()),
            dependencies: Vec::new(),
            capabilities: Vec::new(),
            boundary: None,
            principle: None,
            supports: vec![identity.clone()],
        }
        .bytes();
        let lock = resolve(&manifest, &files).unwrap_or_else(|r| panic!("{r:?}")).bytes();
        let closure = verify(&manifest, &lock, &files, &identity).unwrap_or_else(|r| panic!("{r:?}"));
        let entry = Entry {
            module: "m".into(),
            function: "go".into(),
            author: Author::BuiltIn,
            profile,
            grant: Grant::none(),
            policy: Policy::interim(),
        };
        let mut host = Watched { rows: Rows::on(engine(), Base::Snapshot(base)), proposals: 0 };
        let mut store = Store::default();
        let result = approve(&closure, entry, contract(), checker()).and_then(|approval| {
            let ready = load(&closure, &approval)?;
            let request = RunRequest {
                run: "r1",
                base: "rev-7",
                args: vec![Value::target("c0"), words.value(&[A])],
                inputs: vec![manual(), manual()],
                pins: Pins { contracts: vec![contract()], checkers: vec![checker()] },
                options: Options { menu: words.pinned(), ..Options::new(WIDE) },
            };
            ready.run(request, &mut host, &mut store)
        });
        Trial { result, asked: host.rows.asked().to_vec(), proposals: host.proposals, store }
    }

    /// RTE.46's digest: the canonical result (a refusal by its code), the rows read, the proposals that reached the
    /// host, and the store.
    fn digest(&self) -> String {
        let run = match &self.result {
            Ok(o) => Json::Text(String::from_utf8_lossy(&o.canonical()).into_owned()),
            Err(e) => Json::object([("refused", Json::text(e.code))]),
        };
        let records = self.store.records.iter().map(|(place, s)| {
            let place = match place {
                Place::Workspace(n) => Json::object([("workspace", Json::text(n))]),
                Place::File(n) => Json::object([("file", Json::text(n))]),
            };
            Json::object([("block", Json::Text(hex(&sha256(&s.block)))), ("place", place)])
        });
        hex(&sha256(&provenance::canonical(&Json::object([
            ("asked", Json::List(self.asked.iter().map(|q| Json::text(q)).collect())),
            ("proposals", Json::Int(self.proposals as i64)),
            ("run", run),
            ("store", Json::List(records.collect())),
        ]))))
    }

    fn outcome(&self) -> &Outcome {
        self.result.as_ref().unwrap_or_else(|r| panic!("{r:?}"))
    }
}

/// RTE.46: ten runs of a key, each with a fresh host and store, give one SHA-256.
fn ten(key: impl Fn() -> Trial) -> Trial {
    let first = key();
    let digest = first.digest();
    for i in 2..=10 {
        assert_eq!(key().digest(), digest, "run {i} of 10");
    }
    first
}

/// An allowed key: completed with the expected output, no proposal, nothing written.
fn quiet(trial: &Trial, output: &[u8]) {
    let out = trial.outcome();
    assert_eq!((out.run.summary.outcome, out.run.summary.code), (Ended::Completed, None), "{:?}", out.run.summary);
    assert_eq!(out.run.output.as_deref(), Some(output));
    assert!(out.shell.is_empty() && out.captures.is_empty() && out.separated.is_empty());
    assert_eq!(trial.proposals, 0);
    assert!(trial.store.records.is_empty());
}

/// A refused key, refused at its first proposal by the split: the shell answer has `code`, the proposal's id is
/// separated, no proposal reached the host, and nothing was written. Returns the refused proposal's word, a word of
/// `words` in fixture letters, and its target.
fn refused_by_split(trial: &Trial, words: Words, code: &str) -> (Vec<i32>, String) {
    let out = trial.outcome();
    assert!(!out.shell.is_empty(), "{:?}", out.run.summary);
    let shown = &out.shell[0];
    assert!(matches!(&shown.answer, Answer::Refused(r) if r.code == code), "{:?}", shown.answer);
    assert!(out.separated.contains(&shown.id), "{:?}", out.separated);
    assert_eq!(trial.proposals, 0, "no proposal reaches the engine");
    assert!(trial.store.records.is_empty());
    (words.shown(shown), shown.target.clone())
}

/// The corpus's expected code for a refused case: the first `expected_code` after the case's `id`.
fn corpus_code(file: &str, id: &str) -> String {
    let text = std::fs::read_to_string(repository().join(CORPUS).join(file)).expect("the corpus reads");
    let at = text.find(&format!("\"id\": \"{id}\"")).expect("the case");
    let key = "\"expected_code\": \"";
    let start = at + text[at..].find(key).expect("its code") + key.len();
    text[start..start + text[start..].find('"').unwrap()].to_string()
}

// ------------------------------------------------------------------------------------------------------ the ν keys

/// MR05's, MR06's, A31's and A32's program body: read ν, take two coordinates in field `F` (`f2` or `f5`), solve the
/// corpus's literal identity matrix against them by Cramer's rule, and combine the corpus's two literal words
/// chronologically, the first `x0` times, then the second `x1` times. The solve and the combination are generic in
/// the matrix and the words, as the corpus's `linear_solve_mod` and `combine_corrections` are, and builds words of
/// `kind`.
fn nu_program(kind: Words, field: &str, coordinates: [usize; 2], words: [i32; 2]) -> String {
    let [i, j] = coordinates;
    let (ty, empty) = (kind.ty(), kind.literal(&[]));
    let [p, q] = words.map(|w| kind.literal(&[w]));
    format!(
        "    let nu = read_snapshot<record {{ reason: List<Text>, values: List<Int> }}>(\"nu\");
    let v0 = to_{field}(index(nu.values, {i}));
    let v1 = to_{field}(index(nu.values, {j}));
    let m00 = to_{field}(1);
    let m01 = to_{field}(0);
    let m10 = to_{field}(0);
    let m11 = to_{field}(1);
    let inv = field_inverse(field_sub(field_mul(m00, m11), field_mul(m01, m10)));
    let x0 = field_mul(inv, field_sub(field_mul(v0, m11), field_mul(m01, v1)));
    let x1 = field_mul(inv, field_sub(field_mul(m00, v1), field_mul(v0, m10)));
    let first = repeat(residue(x0), {empty}, fn(done: {ty}) -> {ty} {{
        return concat(done, {p});
    }});
    let second = repeat(residue(x1), {empty}, fn(done: {ty}) -> {ty} {{
        return concat(done, {q});
    }});
    let correction = concat(first, second);
"
    )
}

/// A research run that returns the computed word (A31, A32).
fn research(kind: Words, field: &str, coordinates: [usize; 2], words: [i32; 2]) -> String {
    let (ty, body) = (kind.ty(), nu_program(kind, field, coordinates, words));
    root(&format!("export fn go(t: Target, w: {ty}) -> {ty} effects [snapshot] {{\n{body}    return correction;\n}}\n"))
}

/// A solving run that proposes the computed word at the input target (MR05, MR06).
fn solving(kind: Words, field: &str, coordinates: [usize; 2], words: [i32; 2]) -> String {
    let (ty, body) = (kind.ty(), nu_program(kind, field, coordinates, words));
    root(&format!(
        "export fn go(t: Target, w: {ty}) -> Text effects [snapshot, preview] {{\n{body}    \
         let r = propose(correction, t);\n    return \"requested\";\n}}\n"
    ))
}

/// The base holds the corpus's values: ν is total, equals the catalogue's rows 0 + 2·27, and its `nu2` and `nu5`
/// coordinates are `fixture.json`'s.
fn base_holds_the_corpus_values() {
    let value = match nu::nu(engine(), Base::Snapshot(nu_base())).expect("no engine error").expect("no refusal") {
        Partial::Total(value) => value,
        other => panic!("ν is not total: {other:?}"),
    };
    let catalogue = corrections::catalogue().expect("the catalogue reads");
    let row = |id: u32| *catalogue.lookup(id).expect("a shipped id").row();
    assert_eq!(value, NuVector::ZERO.add_times(&row(0), 1).add_times(&row(27), 2));
    assert_eq!(value.nonzero(), vec![("b8", 1), ("b25", 1), ("C5_17", 3)]);
    let fixture = provenance::parse(&std::fs::read(repository().join(CORPUS).join("fixture.json")).unwrap()).unwrap();
    let corpus = |row: &str| -> Vec<i64> {
        let Some(Json::List(values)) = fixture.get(row) else { panic!("the fixture's {row}") };
        values.iter().map(|v| if let Json::Int(v) = v { *v } else { panic!("{row}: {v:?}") }).collect()
    };
    let values = value.values();
    assert_eq!(NU2.map(|i| values[i]).to_vec(), corpus("nu2"));
    assert_eq!(NU5.map(|i| values[i]).to_vec(), corpus("nu5"));
}

/// A31: research may solve F2 against the read ν and returns the correction `a`, a word of `kind`.
fn a31(kind: Words) {
    base_holds_the_corpus_values();
    let source = research(kind, "f2", NU2, [1, 2]);
    let trial = ten(|| Trial::run(&source, kind, Profile::Research, nu_base()));
    quiet(&trial, &encoded(&kind.value(&[A])));
    assert_eq!(trial.asked, ["nu"]);
}

/// A31 with literal words.
#[test]
fn ct_rte_07_jumble_a31_research_f2_correction() {
    a31(Words::Literal);
}

/// A31 with menu words of the I_a alphabet.
#[test]
fn ct_rte_07_jumble_a31_research_f2_correction_in_menu_words() {
    a31(Words::Menu);
}

/// A32: research may solve F5 against the read ν and returns the correction `d d d`, a word of `kind`.
fn a32(kind: Words) {
    base_holds_the_corpus_values();
    let source = research(kind, "f5", NU5, [3, 4]);
    let trial = ten(|| Trial::run(&source, kind, Profile::Research, nu_base()));
    quiet(&trial, &encoded(&kind.value(&[D, D, D])));
    assert_eq!(trial.asked, ["nu"]);
}

/// A32 with literal words.
#[test]
fn ct_rte_07_jumble_a32_research_f5_correction() {
    a32(Words::Literal);
}

/// A32 with menu words of the I_a alphabet.
#[test]
fn ct_rte_07_jumble_a32_research_f5_correction_in_menu_words() {
    a32(Words::Menu);
}

/// MR05: a solving run solves F2 against the read ν and proposes the correction `a`, a word of `kind`; the split
/// refuses it.
fn mr05(kind: Words) {
    base_holds_the_corpus_values();
    let source = solving(kind, "f2", NU2, [1, 2]);
    let trial = ten(|| Trial::run(&source, kind, Profile::Solving, nu_base()));
    let code = corpus_code("must-refuse.json", "MR05");
    assert_eq!(code, NOT_GRANTED);
    assert_eq!(refused_by_split(&trial, kind, &code), (vec![A], "c0".to_string()));
    assert_eq!(trial.asked, ["nu"]);
}

/// MR05 with literal words.
#[test]
fn ct_rte_07_jumble_mr05_f2_solve_is_refused() {
    mr05(Words::Literal);
}

/// MR05 with menu words of the I_a alphabet.
#[test]
fn ct_rte_07_jumble_mr05_f2_solve_is_refused_in_menu_words() {
    mr05(Words::Menu);
}

/// MR06: a solving run solves F5 against the read ν and proposes the correction `d d d`, a word of `kind`; the split
/// refuses it.
fn mr06(kind: Words) {
    base_holds_the_corpus_values();
    let source = solving(kind, "f5", NU5, [3, 4]);
    let trial = ten(|| Trial::run(&source, kind, Profile::Solving, nu_base()));
    let code = corpus_code("must-refuse.json", "MR06");
    assert_eq!(code, NOT_GRANTED);
    assert_eq!(refused_by_split(&trial, kind, &code), (vec![D, D, D], "c0".to_string()));
    assert_eq!(trial.asked, ["nu"]);
}

/// MR06 with literal words.
#[test]
fn ct_rte_07_jumble_mr06_f5_solve_is_refused() {
    mr06(Words::Literal);
}

/// MR06 with menu words of the I_a alphabet.
#[test]
fn ct_rte_07_jumble_mr06_f5_solve_is_refused_in_menu_words() {
    mr06(Words::Menu);
}

// ---------------------------------------------------------------------------------------------------- the N_c keys

/// The three literal caps of MR01 and MR02 at the walk's state, by a fixed rule of the test from the engine's rim
/// summary in cap-ID order: `c1` and `c2` are the first two admissible caps whose N_c is the largest value that two
/// admissible caps share, and `c0` the first misaligned admissible cap with a smaller N_c. With the engine's N_c at
/// them: caps 57, 86 and 315 with 1360, 1566 and 1566 at this state.
fn literal_caps() -> ([i64; 3], [i64; 3]) {
    let summary = rim_summary(engine(), &mut walk().state().clone(), None, None).expect("no engine error");
    let rows = summary.get("rows").and_then(JsonValue::as_array).expect("a row for each cap");
    let field = |row: &JsonValue, name: &str| row.get(name).and_then(JsonValue::as_i64);
    let admissible: Vec<(i64, i64)> =
        rows.iter().filter_map(|row| Some((field(row, "cap")?, field(row, "N_c")?))).collect();
    let mut by_value: BTreeMap<i64, Vec<i64>> = BTreeMap::new();
    for &(cap, n) in &admissible {
        by_value.entry(n).or_default().push(cap);
    }
    let (&y, tied) = by_value.iter().rev().find(|(_, caps)| caps.len() >= 2).expect("a shared N_c");
    let &(c0, x) = admissible.iter().find(|&&(_, n)| 0 < n && n < y).expect("a smaller N_c");
    ([c0, tied[0], tied[1]], [x, y, y])
}

/// MR01's and MR02's reads: N_c at the three literal caps through the `seam_facts` row.
fn counts(caps: [i64; 3]) -> String {
    let [c0, c1, c2] = caps;
    let seam = "record { totals: List<record { N_c: Int }> }";
    format!(
        "    let n0 = index(read_snapshot<{seam}>(\"seam_facts\", {c0}).totals, 0).N_c;
    let n1 = index(read_snapshot<{seam}>(\"seam_facts\", {c1}).totals, 0).N_c;
    let n2 = index(read_snapshot<{seam}>(\"seam_facts\", {c2}).totals, 0).N_c;
"
    )
}

/// The index of the first maximum of `n0, n1, n2`: ties go to the fixed cap order.
const ARGMAX: &str = "if(exact_less(n0, n1), if(exact_less(n1, n2), 2, 1), if(exact_less(n0, n2), 2, 0))";

/// The corpus's targets `c0`, `c1`, `c2` for the three caps, and a proposal of the input word at the one at `i`.
const PROPOSE: &str = "    let caps = push(push(push(list<Target>(), target(\"c0\")), target(\"c1\")), target(\"c2\"));
    let r = propose(w, index(caps, i));
    return \"requested\";
";

/// The checks of MR01 and MR02: the engine's N_c at the literal caps is [x, y, y] with x < y, the bridge serves it,
/// and the refused proposal is the input word, a word of `kind`, at `c1`, the first of the tie. `body` is the body of
/// `go` for the literal caps.
fn n_c_key(id: &str, kind: Words, body: impl Fn([i64; 3]) -> String) {
    let (caps, n) = literal_caps();
    assert!(n[0] < n[1] && n[1] == n[2], "{n:?}");
    let mut rows = Rows::on(engine(), Base::Snapshot(walk()));
    for (cap, n) in caps.iter().zip(n) {
        let arg = provenance::parse(format!("{{\"int\":\"{cap}\"}}").as_bytes()).expect("a cap");
        let served = rows.answer("seam_facts", Some(&arg)).expect("served");
        assert_eq!(served.get("totals").and_then(|t| t.get("N_c")).and_then(JsonValue::as_i64), Some(n), "cap {cap}");
    }
    let (ty, body) = (kind.ty(), body(caps));
    let source = root(&format!("export fn go(t: Target, w: {ty}) -> Text effects [snapshot, preview] {{\n{body}}}\n"));
    let trial = ten(|| Trial::run(&source, kind, Profile::Solving, walk()));
    let code = corpus_code("must-refuse.json", id);
    assert_eq!(code, NOT_GRANTED);
    assert_eq!(refused_by_split(&trial, kind, &code), (vec![A], "c1".to_string()));
    assert_eq!(trial.asked, ["seam_facts"; 3]);
}

/// MR01: N_c, the first maximum, and a proposal at its cap of the input word, a word of `kind`; the split refuses it.
fn mr01(kind: Words) {
    n_c_key("MR01", kind, |caps| format!("{}    let i = {ARGMAX};\n{PROPOSE}", counts(caps)));
}

/// MR01 with literal words.
#[test]
fn ct_rte_07_jumble_mr01_largest_n_c_is_refused() {
    mr01(Words::Literal);
}

/// MR01 with menu words of the I_a alphabet.
#[test]
fn ct_rte_07_jumble_mr01_largest_n_c_is_refused_in_menu_words() {
    mr01(Words::Menu);
}

/// MR02: N_c ranked in descending order, stable in the fixed cap order, unrolled over the three caps; the first of
/// the rank, and a proposal at its cap of the input word, a word of `kind`; the split refuses it.
fn mr02(kind: Words) {
    n_c_key("MR02", kind, |caps| {
        let counts = counts(caps);
        format!(
            "{counts}    \
             let first = {ARGMAX};\n    \
             let second = if(exact_eq(first, 0), if(exact_less(n1, n2), 2, 1), \
             if(exact_eq(first, 1), if(exact_less(n0, n2), 2, 0), if(exact_less(n0, n1), 1, 0)));\n    \
             let pair = exact_add(first, second);\n    \
             let third = if(exact_eq(pair, 1), 2, if(exact_eq(pair, 2), 1, 0));\n    \
             let rank = push(push(push(list<Nat>(), first), second), third);\n    \
             let i = index(rank, 0);\n{PROPOSE}"
        )
    });
}

/// MR02 with literal words.
#[test]
fn ct_rte_07_jumble_mr02_descending_rank_is_refused() {
    mr02(Words::Literal);
}

/// MR02 with menu words of the I_a alphabet.
#[test]
fn ct_rte_07_jumble_mr02_descending_rank_is_refused_in_menu_words() {
    mr02(Words::Menu);
}

/// The control of the refusals above: a solving run that reads no state and proposes the input word, a word of `kind`,
/// passes the split and reaches the leg's preview point, as A24's manual preview reaches the Full leg's engine.
fn manual_proposal(kind: Words) {
    let source = root(&format!(
        "export fn go(t: Target, w: {}) -> Text effects [preview] {{\n    let r = propose(w, t);\n    \
         return \"requested\";\n}}\n",
        kind.ty()
    ));
    let trial = ten(|| Trial::run(&source, kind, Profile::Solving, walk()));
    let out = trial.outcome();
    assert_eq!((out.run.summary.outcome, out.run.summary.code), (Ended::Completed, None), "{:?}", out.run.summary);
    assert_eq!((out.shell.len(), out.separated.len(), trial.proposals), (1, 0, 1));
    assert_eq!((kind.shown(&out.shell[0]), out.shell[0].target.as_str()), (vec![A], "c0"));
    assert!(trial.asked.is_empty());
}

/// The control with literal words.
#[test]
fn ct_rte_07_jumble_a_manual_proposal_reaches_the_preview_point() {
    manual_proposal(Words::Literal);
}

/// The control with menu words of the I_a alphabet.
#[test]
fn ct_rte_07_jumble_a_manual_menu_word_proposal_reaches_the_preview_point() {
    manual_proposal(Words::Menu);
}

/// Every key this leg realises is a key of the frozen corpus that the Full leg leaves for M5.
#[test]
fn ct_rte_07_jumble_realises_the_keys_the_full_leg_leaves_for_m5() {
    let keys = std::fs::read_to_string(repository().join(CORPUS).join("keys.json")).expect("the corpus reads");
    let full = std::fs::read_to_string(repository().join("rust/c600-module/tests/ct_rte_07_full.rs")).unwrap();
    for key in REALISED {
        assert!(keys.contains(&format!("\"case_id\": \"{key}\"")), "{key} is a corpus key");
        assert!(full.contains(&format!("(\"{key}\", M5)")), "the Full leg leaves {key} for M5");
    }
    assert_eq!(full.matches(", M5)").count(), REALISED.len(), "no other key waits for M5");
}

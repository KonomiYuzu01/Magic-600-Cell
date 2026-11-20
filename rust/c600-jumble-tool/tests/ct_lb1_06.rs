//! CT-LB1-06, the jumbling half (LB1.10; chapter 03), on the model in `assets/` and the W-J fixtures' menus: the
//! relation `jumble-locality/1` that `c600_jumble_tool::locality` pins beside a menu alphabet, the trace keys it gives
//! menu words at run time (LP2's pull request 251), and the library entry `menu_traces` in the language.
//!
//! What runs: the pinned relation agrees with M5's table (`c600_jumble::locality`) on every pair of grips and names the
//! menu it keys; with grips a < b < c, b independent of a and of c and a interacting with c, bca and cab key to bca,
//! which keeps c before a, and abc keys to itself; identity letters and an item beside its inverse stay in the key; on
//! M5's states (solved and two heads of the I_a journal), words with one key are admitted alike and end at one digest
//! (LB1.9), while an identity letter at a straddled grip is refused where the empty word is admitted, so a key that
//! dropped it would change admissibility; `menu_traces.same_trace` and `menu_traces.trace_commute` check as pure code
//! and answer as the keys do; a key under another menu, contract revision, key version or relation version
//! refuses comparison with `LB1:input.key_contract_mismatch`; and word keys remain separate: in one run that pins
//! I_a's alphabet with its item matrices (`Alphabet::pinned`) and the relation beside it, `word_key` (LG2.18's
//! `word-key/1`, LP2's pull request 338) and `trace_key` key the same menu words each by its own rule, and a source
//! that compares a word key with a trace key is a type mismatch.
//!
//! The 600-cell-Full half is LP2's (pull request 214), so with it CT-LB1-06 runs whole. A pass is source and fixture
//! evidence.

mod common;

use std::collections::BTreeSet;
use std::sync::{Arc, OnceLock};

use c600_check::{check, separation, Policy, Separation, TYPE_MISMATCH};
use c600_jumble::locality::Locality;
use c600_jumble::q5::{identity4, matmul};
use c600_jumble::state::{State, Status};
use c600_jumble::twist::Menu;
use c600_jumble_tool::library::{self, MENU_TRACES};
use c600_jumble_tool::locality::{relation, relation_with, table};
use c600_jumble_tool::word::{menu_identity, Alphabet, LiteralWord};
use c600_library::lower_source;
use c600_run::interp::code::INCOMPATIBLE_ALPHABET;
use c600_run::interp::{load, Loaded, NoHost, Options, Refusal, Run};
use c600_run::menu::MenuAlphabet;
use c600_run::menu_key;
use c600_run::meter::{Amounts, Outcome};
use c600_run::trace::{check_keys, Relation, TraceHeader, TraceKey, JUMBLE_LOCALITY, KEY_CONTRACT_MISMATCH, TRACE_KEY};
use c600_run::value::Value as RunValue;
use c600_syntax::{format, parse};
use common::{engine, Fixture};

/// Run limits wide enough for every run here.
const WIDE: Amounts = Amounts { steps: 1 << 40, kernel_cost: 1 << 40, memory: 1 << 40, output: 1 << 40 };

/// The heads of the I_a journal tested besides solved: the first return onto the lattice (CT-LB4-03's record 13) and
/// the head after 130 records.
const HEADS: [usize; 2] = [13, 130];

/// M5's table of the model, regenerated once.
fn locality() -> &'static Locality {
    static TABLE: OnceLock<Locality> = OnceLock::new();
    TABLE.get_or_init(|| table(engine()).expect("the locality table"))
}

/// One W-J fixture's menu, alphabet and the relation the tool pins for it.
struct Pinned {
    menu: Arc<Menu>,
    alphabet: Alphabet,
    pinned: MenuAlphabet,
    relation: Relation,
}

fn pinned(name: &'static str) -> Pinned {
    let menu = Fixture::load(name).menu();
    let alphabet = Alphabet::of(engine(), &menu).expect("an inverse-closed menu");
    let relation = relation_with(locality(), &alphabet).expect("the relation");
    let pinned = alphabet.pinned().expect("the alphabet pins");
    Pinned { menu: Arc::new(menu), alphabet, pinned, relation }
}

fn i_a() -> &'static Pinned {
    static I_A: OnceLock<Pinned> = OnceLock::new();
    I_A.get_or_init(|| pinned("I_a"))
}

/// The menu's identity item and the first item that is not its own inverse, with that item's inverse: A4 has
/// rotations of order 3.
fn items(p: &Pinned) -> (u32, u32, u32) {
    let identity = p.menu.items.iter().position(|(_, m)| *m == identity4()).expect("the menu holds A4") as u32;
    let inverse = |x: u32| p.alphabet.inverse_item(x as i32).expect("an inverse item") as u32;
    let x = (0..p.alphabet.items() as u32).find(|&x| inverse(x) != x).expect("an item of order above 2");
    (identity, x, inverse(x))
}

/// Grips a < b < c with a and c interacting and b independent of both, as M5's test finds them.
fn grips() -> [u32; 3] {
    let loc = locality();
    let a = 0usize;
    let (b, c) = (loc.partners(a).iter().map(|&c| c as usize))
        .find_map(|c| (a + 1..c).find(|&b| loc.independent(a, b) && loc.independent(b, c)).map(|b| (b, c)))
        .expect("a partner c of a with a grip b between them independent of both");
    assert!(!loc.independent(a, c));
    [a, b, c].map(|g| g as u32)
}

fn key(r: &Relation, w: &[[u32; 2]]) -> TraceKey {
    r.menu_key(w).expect("letters of the alphabet")
}

fn letters(r: &Relation, w: &[[u32; 2]]) -> Vec<[u32; 2]> {
    key(r, w).menu_letters().to_vec()
}

fn literal(p: &Pinned, w: &[[u32; 2]]) -> LiteralWord {
    let pairs: Vec<[i32; 2]> = w.iter().map(|&[g, x]| [g as i32, x as i32]).collect();
    p.alphabet.word(&pairs).expect("a word of the alphabet")
}

/// The end digest of `w` played letter by letter from `state`, or `None` when a prefix is refused.
fn play(p: &Pinned, state: &State, w: &[[u32; 2]]) -> Option<String> {
    let mut s = state.clone();
    for (_, twist) in literal(p, w).letters(engine(), &s).expect("letters of the state's alphabet") {
        let outcome = s.apply(engine(), &twist, None, &[]).expect("the engine answers");
        if outcome.status != Status::Admissible {
            return None;
        }
    }
    Some(s.digest(engine()))
}

/// Solved under I_a's menu, and the heads of the I_a journal after [`HEADS`] records.
fn states() -> &'static [State] {
    static STATES: OnceLock<Vec<State>> = OnceLock::new();
    STATES.get_or_init(|| {
        let fixture = Fixture::load("I_a");
        let mut state = State::new(Some(i_a().menu.clone()));
        let mut out = vec![state.clone()];
        for (index, twist) in fixture.twists.iter().take(HEADS[1]).enumerate() {
            let outcome = state.apply(engine(), twist, None, &[]).expect("the engine answers");
            assert_eq!(outcome.status, Status::Admissible, "journal record {index}");
            if HEADS.contains(&(index + 1)) {
                out.push(state.clone());
            }
        }
        out
    })
}

/// The pinned relation is M5's table of the model under the menu's identity block: every pair of distinct grips is
/// independent exactly as `Locality::independent` says, letters at one grip are dependent, and the header names the
/// key, the contract, the 600 grips and the menu with its item count. Relations of two menus of the model share the
/// table's digest and differ in the menu they name, and each keys only its own alphabet.
#[test]
fn the_tool_pins_the_models_relation_beside_the_menu_alphabet() {
    let loc = locality();
    let menus = [i_a(), &pinned("I_b"), &pinned("S4")];
    for p in menus {
        let r = &p.relation;
        let h = r.header();
        assert_eq!((h.key.as_str(), h.contract.as_str(), h.generators), (TRACE_KEY, JUMBLE_LOCALITY, 600));
        assert_eq!(h.menu_identity(), Some(menu_identity(p.alphabet.identity())));
        assert_eq!(h.menu.as_ref().map(|m| m.items), Some(p.alphabet.items() as u32));
        assert_eq!(r.bits(), loc.bits(), "the model's table, as regenerated");
        assert!(r.is_menu() && r.keys_menu(&p.pinned));
        for c in 0..loc.grips() {
            assert!(!r.menu_independent([c as u32, 0], [c as u32, 1]), "letters at grip {c} are dependent");
            for d in c + 1..loc.grips() {
                let (gc, gd) = (c as u32, d as u32);
                assert_eq!(r.menu_independent([gc, 1], [gd, 0]), loc.independent(c, d), "({c}, {d})");
            }
        }
    }
    let [a, b, s4] = menus.map(|p| &p.relation);
    assert_eq!((a.digest(), b.digest()), (s4.digest(), s4.digest()), "one model, one table");
    assert_eq!(a.header().differing(b.header()), Some("menu_id"));
    assert!(!a.keys_menu(&menus[1].pinned) && !s4.keys_menu(&menus[0].pinned));
    assert_eq!(relation(engine(), &i_a().alphabet).as_ref(), Ok(a), "the table built again gives the same relation");
}

/// CT-LB1-06 (part: LB1.10's jumbling half on the model, keys and their soundness on M5). bca and cab key to bca and
/// abc to itself. Identity letters and an item beside its inverse stay. On every tested state, bca and cab are admitted
/// alike and end at one digest, and so are words that adjacent swaps of letters at independent grips relate; at
/// solved, abc ends elsewhere. At a straddled grip g of a jumbled head, the identity letter and the word x·x⁻¹ at g are
/// refused where the empty word is admitted, while at solved x·x⁻¹ ends where the empty word does: the key keeps both
/// letters, so it never equates words that the domain tells apart.
#[test]
fn ct_lb1_06_bca_and_cab_key_to_bca_and_keys_keep_admissibility_and_end_digest() {
    let p = i_a();
    let r = &p.relation;
    let (id, x, xi) = items(p);
    let [ga, gb, gc] = grips();
    let (a, b, c) = ([ga, x], [gb, x], [gc, x]);
    assert_eq!(letters(r, &[b, c, a]), [b, c, a]);
    assert_eq!(letters(r, &[c, a, b]), [b, c, a]);
    assert_eq!(letters(r, &[a, b, c]), [a, b, c]);
    assert_ne!(key(r, &[a, b, c]), key(r, &[b, c, a]), "c before a is kept");
    assert_eq!(letters(r, &[[gb, id], [ga, id]]), [[ga, id], [gb, id]], "identity letters at independent grips swap");
    assert_eq!(letters(r, &[[gc, id], [ga, id]]), [[gc, id], [ga, id]], "and at interacting grips do not");
    assert_eq!(letters(r, &[[ga, x], [ga, xi]]), [[ga, x], [ga, xi]], "an item and its inverse do not cancel");
    assert_ne!(key(r, &[[ga, id]]), key(r, &[]), "an identity letter stays");

    let solved = &states()[0];
    for (i, state) in states().iter().enumerate() {
        let (bca, cab) = (play(p, state, &[b, c, a]), play(p, state, &[c, a, b]));
        assert_eq!(bca, cab, "state {i}: bca and cab");
    }
    let abc = play(p, solved, &[a, b, c]).expect("abc is admitted at solved");
    assert_ne!(Some(abc), play(p, solved, &[b, c, a]), "abc ends elsewhere at solved");

    // A straddled grip of a jumbled head: its identity letter is refused there, and x·x⁻¹ at it too.
    let head = states().last().expect("a jumbled head");
    let g = (0..600u32).find(|&g| play(p, head, &[[g, id]]).is_none()).expect("a straddled grip at the head");
    assert_eq!(play(p, head, &[]), Some(head.digest(engine())), "the empty word is admitted");
    assert_eq!(play(p, head, &[[g, x], [g, xi]]), None, "x·x⁻¹ at a straddled grip is refused");
    assert_eq!(play(p, solved, &[[g, x], [g, xi]]), Some(solved.digest(engine())), "and ends at solved where it began");
    assert_ne!(key(r, &[[g, x], [g, xi]]), key(r, &[]));

    // Words related by adjacent swaps of letters at independent grips: one key, and alike on every tested state.
    let pool = {
        let loc = locality();
        let mut pool: BTreeSet<u32> = [ga, gb, gc, g].into();
        pool.extend(loc.partners(g as usize).iter().take(2).map(|&d| d as u32));
        pool.extend((0..600u32).filter(|&d| d != g && loc.independent(g as usize, d as usize)).take(2));
        pool.into_iter().collect::<Vec<u32>>()
    };
    let mut seed = 0x2545_f491_4f6c_dd1du64;
    let mut next = |bound: usize| {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        (seed % bound as u64) as usize
    };
    let (mut swapped_pairs, mut admitted, mut refused) = (0, 0, 0);
    for _ in 0..24 {
        let length = 2 + next(4);
        let w: Vec<[u32; 2]> = (0..length).map(|_| [pool[next(pool.len())], [id, x, xi][next(3)]]).collect();
        let mut v = w.clone();
        for _ in 0..6 {
            let i = 1 + next(v.len() - 1);
            if r.menu_independent(v[i - 1], v[i]) {
                v.swap(i - 1, i);
            }
        }
        assert_eq!(key(r, &w), key(r, &v), "{w:?} and {v:?}");
        if w != v {
            swapped_pairs += 1;
        }
        for (i, state) in states().iter().enumerate() {
            let end = play(p, state, &w);
            assert_eq!(end, play(p, state, &v), "state {i}: {w:?} and {v:?}");
            if i > 0 {
                *if end.is_some() { &mut admitted } else { &mut refused } += 1;
            }
        }
    }
    assert!(swapped_pairs > 0, "some words were reordered");
    assert!(admitted > 0 && refused > 0, "the jumbled heads admit some words and refuse others: {admitted}, {refused}");
}

/// The library's module source parses with no diagnostic, is in the formatter's canonical layout and has LF endings.
fn canonical(source: &str) {
    let file = parse(source.as_bytes());
    assert!(file.diagnostics.is_empty() && file.holes.is_empty(), "{:#?}", file.diagnostics);
    assert_eq!(format(&file).as_deref(), Some(source), "the canonical layout");
    assert!(!source.contains('\r') && source.ends_with('\n'), "LF line endings");
}

fn loaded() -> &'static Loaded {
    static LOADED: OnceLock<Loaded> = OnceLock::new();
    LOADED.get_or_init(|| load(&lower_source(MENU_TRACES.source).expect("the module lowers")).expect("it loads"))
}

fn menu_word(p: &Pinned, w: &[[u32; 2]]) -> RunValue {
    RunValue::menu_word(p.pinned.shared_identity(), w).expect("letters below 600")
}

fn run(entry: &str, args: Vec<RunValue>, alphabet: &MenuAlphabet, r: &Relation) -> Result<Run, Refusal> {
    loaded().run(
        MENU_TRACES.name,
        entry,
        args,
        &mut NoHost,
        &Options { menu: Some(alphabet), relation: Some(r), ..Options::new(WIDE) },
    )
}

fn answer(entry: &str, p: &Pinned, x: &[[u32; 2]], y: &[[u32; 2]]) -> bool {
    let ran = run(entry, vec![menu_word(p, x), menu_word(p, y)], &p.pinned, &p.relation).expect("the run starts");
    assert_eq!((ran.summary.outcome, ran.summary.code), (Outcome::Completed, None), "{entry}({x:?}, {y:?})");
    match ran.value {
        Some(RunValue::Bool(answer)) => answer,
        other => panic!("{entry}: {other:?}"),
    }
}

/// CT-LB1-06 (part: the library entry, `menu_traces`, in the language on the model's relation). The module is in the
/// canonical layout, requests no capability and checks with no diagnostic; both entries are pure, and under capability
/// separation neither reads state nor prepares previews. Run with I_a's alphabet and the relation the tool pins,
/// `same_trace` answers as the keys compare, and `trace_commute` says a commutes with b and not with c. The catalogue
/// row names the entries, the effect and the source's digest.
#[test]
fn ct_lb1_06_menu_traces_compares_keys_in_the_language() {
    canonical(MENU_TRACES.source);
    let program = lower_source(MENU_TRACES.source).expect("the module lowers");
    let [module] = &program.modules[..] else { panic!("one module") };
    assert!(module.capabilities.is_empty());
    let exports: Vec<&str> = module.functions.iter().filter(|f| f.exported).map(|f| f.name.as_str()).collect();
    assert_eq!(exports, MENU_TRACES.exports);
    let report = check(&program).expect("well-formed IR");
    assert!(report.diagnostics.is_empty(), "{:#?}", report.diagnostics);
    for entry in MENU_TRACES.exports {
        let summary = report.summary(MENU_TRACES.name, entry).expect("a summary");
        assert!(summary.is_pure() && summary.state_reads.is_empty() && summary.rows.is_empty(), "{summary:?}");
        let none = Separation { reads_state: false, prepares_previews: false };
        assert_eq!(separation(summary, &Policy::interim()), none);
    }
    let row = library::trace_equality_keys();
    assert_eq!((row.entry, row.module, row.functions), ("Trace equality keys", MENU_TRACES.name, MENU_TRACES.exports));
    assert!(row.effects.is_empty() && row.notes.contains(KEY_CONTRACT_MISMATCH));
    assert_eq!(row.source_sha256, MENU_TRACES.sha256());

    let p = i_a();
    let (id, x, xi) = items(p);
    let [ga, gb, gc] = grips();
    let (a, b, c) = ([ga, x], [gb, x], [gc, x]);
    for (u, v, same) in [
        (&[b, c, a][..], &[c, a, b][..], true),
        (&[a, b, c], &[b, c, a], false),
        (&[[ga, id]], &[], false),
        (&[[ga, x], [ga, xi]], &[], false),
        (&[[gb, id], [ga, id]], &[[ga, id], [gb, id]], true),
    ] {
        assert_eq!(answer("same_trace", p, u, v), same, "{u:?} and {v:?}");
        assert_eq!(answer("same_trace", p, u, v), key(&p.relation, u) == key(&p.relation, v));
    }
    assert!(answer("trace_commute", p, &[a], &[b]) && answer("trace_commute", p, &[b, b], &[c, a]));
    assert!(!answer("trace_commute", p, &[a], &[c]) && !answer("trace_commute", p, &[[ga, x]], &[[ga, xi]]));
}

/// CT-LB1-06 (part: changed versions refuse comparison). A key of I_a's words under I_b's relation, under a relation of
/// another contract revision, or with another key or relation version in its header is refused with
/// `LB1:input.key_contract_mismatch`; a run that pins I_a's alphabet with I_b's relation stops with that code before it
/// keys a word, and an argument of another menu's alphabet is refused with `LB1:input.incompatible_alphabet`.
#[test]
fn ct_lb1_06_keys_of_another_menu_or_version_refuse_comparison() {
    let (p, other) = (i_a(), pinned("I_b"));
    let [ga, gb, gc] = grips();
    let (_, x, _) = items(p);
    let w = [[gb, x], [gc, x], [ga, x]];
    let held = |k: TraceKey| RunValue::TraceEqualityKey(Arc::new(k));
    assert_eq!(check_keys(&held(key(&p.relation, &w)), Some(&p.relation)), Ok(()));

    let edited = |edit: fn(&mut TraceHeader)| {
        let mut h = p.relation.header().clone();
        edit(&mut h);
        held(TraceKey::from_menu_parts(Arc::new(h), &letters(&p.relation, &w)))
    };
    let mut later = menu_identity(p.alphabet.identity());
    later.contract_rev.push_str("-next");
    let revised = Relation::jumble_locality(&later, 600, p.alphabet.items() as u32, locality().bits().to_vec())
        .expect("a relation of another contract revision");
    for (k, field) in [
        (held(key(&other.relation, &w)), "menu_id"),
        (held(key(&revised, &w)), "contract_rev"),
        (edited(|h| h.key = "trace-key/2".into()), "key"),
        (edited(|h| h.contract = "jumble-locality/2".into()), "contract"),
    ] {
        let why = check_keys(&k, Some(&p.relation)).expect_err(field);
        assert!(why.contains(field), "{field}: {why}");
    }

    let args = || vec![menu_word(p, &w), menu_word(p, &w)];
    let stop = run("same_trace", args(), &p.pinned, &other.relation).expect("the run starts");
    assert_eq!((stop.summary.outcome, stop.summary.code), (Outcome::Error, Some(KEY_CONTRACT_MISMATCH)));
    assert!(stop.value.is_none());
    let foreign = vec![menu_word(&other, &w), menu_word(p, &w)];
    let refused = run("same_trace", foreign, &p.pinned, &p.relation).expect_err("another alphabet");
    assert_eq!(refused.code, INCOMPATIBLE_ALPHABET);
}

/// Both keys of two menu words, in one module: `traces` compares their trace keys and `words` their word keys.
const BOTH: &str = "edition \"6cs-draft-1\";\nmodule both requires [];\n\n\
    export fn traces(x: MenuWord, y: MenuWord) -> Bool effects [] {\n    \
    return exact_eq(trace_key(x), trace_key(y));\n}\n\n\
    export fn words(x: MenuWord, y: MenuWord) -> Bool effects [] {\n    \
    return exact_eq(word_key(x), word_key(y));\n}\n";

/// A source that compares a menu word's word key with its trace key.
const MIXED: &str = "edition \"6cs-draft-1\";\nmodule mixed requires [];\n\n\
    export fn mixed(x: MenuWord) -> Bool effects [] {\n    return exact_eq(word_key(x), trace_key(x));\n}\n";

fn both() -> &'static Loaded {
    static LOADED: OnceLock<Loaded> = OnceLock::new();
    LOADED.get_or_init(|| load(&lower_source(BOTH).expect("the module lowers")).expect("it loads"))
}

/// `both.<entry>(x, y)` in a run that pins the alphabet with its item matrices and the relation beside it.
fn compared(entry: &str, p: &Pinned, x: &[[u32; 2]], y: &[[u32; 2]]) -> bool {
    let options = Options { menu: Some(&p.pinned), relation: Some(&p.relation), ..Options::new(WIDE) };
    let args = vec![menu_word(p, x), menu_word(p, y)];
    let ran = both().run("both", entry, args, &mut NoHost, &options).expect("the run starts");
    assert_eq!((ran.summary.outcome, ran.summary.code), (Outcome::Completed, None), "{entry}({x:?}, {y:?})");
    match ran.value {
        Some(RunValue::Bool(answer)) => answer,
        other => panic!("{entry}: {other:?}"),
    }
}

/// CT-LB1-06 (part: "Word keys remain separate" on the jumbling side, against LG2.18's word key `word-key/1` under
/// I_a's item matrices, which `Alphabet::pinned` pins beside the relation). In one run, `trace_key` and `word_key` key
/// the same menu words each by its own rule, neither composed with the other (LG2.18, LB1.10). bca and cab have one
/// trace key and two word keys, since a word key swaps no letters, and so do identity letters at two independent
/// grips. An item and its inverse at one grip, or an item twice, have the word key of the one letter at that grip whose
/// item is their product, and trace keys of their own, since a trace key keeps every letter. x·x⁻¹ keeps its grip's
/// token, so neither of its keys is the empty word's. Each answer is the comparison of the keys that `c600_run` builds,
/// a word key of I_b's alphabet is refused under I_a's header with `LB1:input.key_contract_mismatch`'s reason, and a
/// source that compares a word key with a trace key is a type mismatch.
#[test]
fn ct_lb1_06_word_keys_remain_separate_from_trace_keys() {
    let p = i_a();
    let r = &p.relation;
    assert!(p.pinned.item_matrices().is_some(), "the alphabet carries its item matrices");
    let (id, x, xi) = items(p);
    let [ga, gb, gc] = grips();
    let (a, b, c) = ([ga, x], [gb, x], [gc, x]);
    let mx = &p.menu.items[x as usize].1;
    let square = matmul(mx, mx);
    let square = p.menu.items.iter().position(|(_, m)| *m == square).expect("x² is an item of the menu") as u32;
    let word_key = |w: &[[u32; 2]]| menu_key::key(&p.pinned, w).expect("letters of the alphabet");

    for (u, v, traces, words) in [
        (&[b, c, a][..], &[c, a, b][..], true, false),
        (&[[gb, id], [ga, id]], &[[ga, id], [gb, id]], true, false),
        (&[[ga, x], [ga, xi]], &[[ga, id]], false, true),
        (&[[ga, x], [ga, x]], &[[ga, square]], false, true),
        (&[[ga, x], [ga, xi]], &[], false, false),
        (&[b, c, a], &[b, c, a], true, true),
    ] {
        assert_eq!(compared("traces", p, u, v), traces, "trace keys of {u:?} and {v:?}");
        assert_eq!(compared("words", p, u, v), words, "word keys of {u:?} and {v:?}");
        assert_eq!(key(r, u) == key(r, v), traces);
        assert_eq!(word_key(u) == word_key(v), words);
    }

    let other = pinned("I_b");
    let foreign = menu_key::key(&other.pinned, &[[ga, 0]]).expect("a letter of I_b's alphabet");
    let why = menu_key::check_keys(&RunValue::MenuWordKey(Arc::new(foreign)), Some(&p.pinned)).expect_err("I_b's key");
    assert!(why.contains("menu_id"), "{why}");

    let program = lower_source(MIXED).expect("the module lowers");
    let report = check(&program).expect("well-formed IR");
    assert!(report.diagnostics.iter().any(|d| d.code == TYPE_MISMATCH), "{:#?}", report.diagnostics);
}

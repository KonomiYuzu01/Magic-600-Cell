//! Kernel tests against the model in `assets/` and the frozen oracle cases. `tests/test_kernel_oracle.py` at the
//! repository root replays all seven cases, and the new cases N01 to N04, through the adapter in a release build and
//! runs the full KN2.9 agreement test there; these tests run the small cases O06 and O07, the first steps of O01 and a
//! short agreement test through the same API in the test profile, and the kernel-side leads L4, L6 and L9 of the M4
//! differential tests (work/reviews/leads/2026-11-16-m4-differential-tests.md).

use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use c600_kernel::json::{self, Value};
use c600_kernel::{
    archive, codes, Derivation, Effect, IdentityBlock, Kernel, Level, ProtectedSet, ProtectionMode, Puzzle, Recipe,
    Snapshot, Step, SupportRow, Verdict, PIECES, PROTECTED_MESSAGE, SLOTS,
};

/// `Digest<literal word>` of `[1, -1]`, which O06 records.
const ONE_MINUS_ONE: &str = "b15348c8f462384c01e83b6d499c6faf3f96808f5aa07c6bab4b65b36b4445d4";
/// 0.4's texts for a bad letter and for the compatibility ceiling (`core.py`, `Model.normalize`).
const BAD_PRIMITIVE: &str = "Primitive ID must be an integer in ±1..±1200";
const CEILING: &str = "Preview expansion exceeds 3,000,000 primitives; split the batch";

fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..")
}

fn kernel() -> &'static Kernel {
    static KERNEL: OnceLock<Kernel> = OnceLock::new();
    KERNEL.get_or_init(|| match Kernel::load(&repository().join("assets")) {
        Ok(kernel) => kernel,
        Err(error) => panic!("the model in assets/ does not load: {error}"),
    })
}

/// A frozen oracle case, read with the crate's own gzip and JSON readers.
fn case(name: &str) -> Value {
    let path = repository().join("work/experiments/language-lx/inputs/oracle").join(format!("{name}.json.gz"));
    let bytes = std::fs::read(path).expect("the frozen case file is readable");
    json::parse(&archive::gunzip(&bytes, 1 << 30).expect("the case file is gzip")).expect("the case file is JSON")
}

fn parse(text: &str) -> Value {
    json::parse(text.as_bytes()).unwrap()
}

fn integers(value: Option<&Value>) -> Vec<i64> {
    value.and_then(Value::as_array).unwrap().iter().map(|item| item.as_i64().unwrap()).collect()
}

fn derive(base: &Snapshot, recipe: &Recipe, protected: &ProtectedSet) -> Derivation {
    kernel().derive(base, recipe, protected).expect("no star certificate fails")
}

fn derived(base: &Snapshot, letters: &[i64]) -> Snapshot {
    match derive(base, &kernel().word(letters).unwrap(), &ProtectedSet::none()) {
        Derivation::Derived(derived) => derived.state,
        Derivation::Refused(refused) => panic!("unexpected refusal: {}", refused.refusal),
    }
}

fn star(orbit: usize, node: usize, sign: i32) -> Recipe {
    kernel()
        .recipe(&parse(&format!(r#"[{{"kind": "star", "orbit": {orbit}, "node": {node}, "sign": {sign}}}]"#)))
        .unwrap()
}

fn rows(rows: &[SupportRow]) -> Value {
    let int = |n: usize| Value::Int(n as i64);
    let row = |r: &SupportRow| {
        json::object([("orbit", Value::Int(r.orbit as i64)), ("pieces", int(r.pieces)), ("stickers", int(r.stickers))])
    };
    Value::Array(rows.iter().map(row).collect())
}

/// The oracle row of one step, built from the kernel's answers as the adapter builds it.
fn row(accepted: bool, error: Option<&str>, state: &Snapshot, effect: Option<(&Effect, &[SupportRow])>) -> Value {
    let int = |n: usize| Value::Int(n as i64);
    let text = |s: &str| Value::String(s.to_owned());
    let progress = kernel().progress(state).unwrap().sha256;
    let [net, stickers, pieces, expansion, count, support, conflicts] = match effect {
        Some((effect, conflicts)) => [
            text(&effect.net_sha256),
            int(effect.moved_stickers()),
            int(effect.moved_pieces),
            text(&effect.expansion_sha256),
            int(effect.primitive_count),
            rows(&effect.support),
            rows(conflicts),
        ],
        None => std::array::from_fn(|_| Value::Null),
    };
    json::object([
        ("accepted", Value::Bool(accepted)),
        ("error", error.map_or(Value::Null, text)),
        ("state_hash", text(state.digest())),
        ("net_sha256", net),
        ("net_moved_stickers", stickers),
        ("net_moved_pieces", pieces),
        ("expansion_sha256", expansion),
        ("primitive_count", count),
        ("support", support),
        ("conflicts", conflicts),
        ("progress_sha256", text(&progress)),
    ])
}

/// Replays the first `limit` steps of a frozen case through the kernel API, checks every field of every row, and
/// returns the number of steps replayed.
fn replay(name: &str, limit: usize) -> usize {
    let kernel = kernel();
    let case = case(name);
    let model = case.get("model").unwrap();
    assert_eq!(model.get("model_id").and_then(Value::as_str), Some(kernel.identity().model_id.as_str()));
    assert_eq!(model.get("manifest_sha256").and_then(Value::as_str), Some(kernel.manifest_sha256()));
    let initial = case.get("initial").unwrap();
    let moves = integers(initial.get("moves"));
    let start = if moves.is_empty() { kernel.solved() } else { derived(&kernel.solved(), &moves) };
    assert_eq!(initial.get("state_hash").and_then(Value::as_str), Some(start.digest()));
    let protected = kernel.protected_set(&integers(case.get("protected"))).unwrap();
    let chain = match case.get("mode").and_then(Value::as_str) {
        Some("chain") => true,
        Some("independent") => false,
        mode => panic!("unknown mode {mode:?}"),
    };
    let mut state = start.clone();
    let steps = case.get("steps").and_then(Value::as_array).unwrap();
    for step in steps.iter().take(limit) {
        let index = step.get("index").and_then(Value::as_i64).unwrap();
        let base = if chain { state.clone() } else { start.clone() };
        let got = match kernel.recipe(step.get("recipe").unwrap()) {
            Err(refusal) => {
                assert!([codes::RECIPE_SHAPE, codes::PRIMITIVE_CEILING].contains(&refusal.code), "{name} {index}");
                row(false, Some("invalid-input"), &base, None)
            }
            Ok(recipe) => match derive(&base, &recipe, &protected) {
                Derivation::Derived(derived) => {
                    assert_eq!(derived.base, base.digest());
                    assert_eq!(derived.word, derived.effect.expansion_sha256);
                    assert_eq!(derived.protection.verdict, Verdict::Clear);
                    let got = row(true, None, &derived.state, Some((&derived.effect, &derived.protection.conflicts)));
                    if chain {
                        state = derived.state;
                    }
                    got
                }
                Derivation::Refused(refused) => {
                    // CT-KN2-04: the refusal names the unchanged base and still reports the effect.
                    assert_eq!(refused.base, base.digest());
                    assert_eq!(refused.refusal.code, codes::ORBIT_WOULD_MOVE);
                    assert_eq!(refused.refusal.message, PROTECTED_MESSAGE);
                    assert_eq!(refused.first_refused_prefix, None);
                    assert_eq!(refused.protection.verdict, Verdict::Moves);
                    row(false, Some("protected"), &base, Some((&refused.effect, &refused.protection.conflicts)))
                }
            },
        };
        assert_eq!(&got, step.get("expect").unwrap(), "{name} step {index}");
    }
    steps.len().min(limit)
}

/// CT-KN1-01 (part: identity blocks and the registry here; snapshots and recipes of another model in
/// `tests/load.rs`, `ct_kn1_01_values_of_another_model_are_refused`. A jumbling identity is only refused, since the
/// jumbling half is M5's, and marking a value as recorded needs the session store, M6).
#[test]
fn ct_kn1_01_identity_block_and_registry() {
    let kernel = kernel();
    let manifest = std::fs::read(repository().join("assets").join("manifest.json")).unwrap();
    let parsed = json::parse(&manifest).unwrap();
    let identity = kernel.identity();
    assert_eq!(identity, IdentityBlock::full(parsed.get("model_id").and_then(Value::as_str).unwrap()));
    assert_eq!(kernel.manifest_sha256(), c600_digest::sha256_hex(&manifest));
    kernel.check_identity(&identity).unwrap();
    let info = kernel.model_info(&identity).unwrap();
    assert_eq!(info.identity, identity);
    assert_eq!(
        (info.puzzle_name, info.slots, info.pieces, info.orbits, info.generators),
        ("600-cell-Full", 259_800, 177_120, 35, 1_200)
    );
    assert_eq!(info.verified_files, parsed.get("files").and_then(Value::as_object).unwrap().len());

    let mismatch = |block: IdentityBlock, field: &str| {
        let refusal = kernel.check_identity(&block).unwrap_err();
        assert_eq!(refusal.code, codes::IDENTITY_MISMATCH);
        assert!(refusal.message.ends_with(field), "{refusal}");
        assert_eq!(kernel.model_info(&block).unwrap_err().code, codes::UNKNOWN_MODEL);
    };
    let other = "0".repeat(64);
    mismatch(IdentityBlock::full(&other), "model_id");
    mismatch(IdentityBlock { puzzle: Puzzle::Jumbling, ..identity.clone() }, "puzzle");
    mismatch(IdentityBlock { menu_id: Some(other.clone()), ..identity.clone() }, "menu_id");
    mismatch(IdentityBlock { menu_order: Some(other.clone()), ..identity.clone() }, "menu_order");
    mismatch(IdentityBlock { contract_rev: Some("r1".into()), ..identity }, "contract_rev");
}

/// CT-KN1-05 (part: the labels half. `Digest<labels>` is the oracle's state hash on the solved state here, and on
/// every row of O01 to O07 and N01 to N04 in `tests/test_kernel_oracle.py`; `Digest<poses>` against J1 is M5's).
#[test]
fn ct_kn1_05_labels_digest_is_the_oracle_state_hash() {
    let solved = kernel().solved();
    assert_eq!(solved.labels().len(), 259_800);
    assert!(solved.labels().iter().enumerate().all(|(slot, &label)| label as usize == slot));
    let bytes: Vec<u8> = (0..259_800i32).flat_map(i32::to_le_bytes).collect();
    assert_eq!(solved.digest(), c600_digest::sha256_hex(&bytes));
    let initial = case("O01").get("initial").cloned().unwrap();
    assert_eq!(initial.get("kind").and_then(Value::as_str), Some("solved"));
    assert_eq!(initial.get("state_hash").and_then(Value::as_str), Some(solved.digest()));
}

/// CT-KN2-01 (part: O06 whole through the kernel API in the test profile; `tests/test_kernel_oracle.py` replays all
/// of O01 to O07 and N01 to N04 through the adapter; the J1 harness half is M5's).
#[test]
fn ct_kn2_01_full_o06_protection_chain() {
    assert_eq!(replay("O06", usize::MAX), 6);
}

/// CT-KN2-01 (part: O07 whole here; the rest as for `ct_kn2_01_full_o06_protection_chain`).
#[test]
fn ct_kn2_01_full_o07_invalid_inputs() {
    assert_eq!(replay("O07", usize::MAX), 10);
}

/// CT-KN2-01 (part: the first eight steps of O01 here; the rest as for `ct_kn2_01_full_o06_protection_chain`).
#[test]
fn ct_kn2_01_full_o01_first_generators() {
    assert_eq!(replay("O01", 8), 8);
}

/// CT-KN2-04 (part: a blocked and an invalid recipe; `tests/test_kernel_oracle.py` adds the invalid recipe family of
/// leads L1 and L4, each an invalid-input step that keeps the state hash. An uncertain word needs the jumbling half,
/// M5).
#[test]
fn ct_kn2_04_blocked_and_invalid_recipes_change_no_digest() {
    let kernel = kernel();
    let base = derived(&kernel.solved(), &[5, -77, 300]);
    let digest = base.digest().to_owned();
    let labels = base.labels().to_vec();
    let one = kernel.word(&[1]).unwrap();
    let effect = kernel.effect(&one).unwrap();
    let moved: Vec<i64> = effect.support.iter().filter(|r| r.orbit >= 0).map(|r| r.orbit as i64).collect();
    assert!(!moved.is_empty());

    // Every orbit the twist moves is protected: refused, with the effect still reported.
    let Derivation::Refused(refused) = derive(&base, &one, &kernel.protected_set(&moved).unwrap()) else {
        panic!("a twist of protected orbits was accepted");
    };
    assert_eq!((refused.base.as_str(), refused.word.as_str()), (digest.as_str(), effect.expansion_sha256.as_str()));
    assert_eq!(refused.effect, effect);
    let conflicts: Vec<SupportRow> = effect.support.iter().filter(|r| r.orbit >= 0).cloned().collect();
    assert_eq!(refused.protection.conflicts, conflicts);
    assert_eq!((base.digest(), base.labels()), (digest.as_str(), labels.as_slice()));

    // An invalid recipe is refused whole, before any effect.
    let invalid = parse(r#"[{"kind": "word", "moves": [1]}, {"kind": "word", "moves": [0]}]"#);
    assert_eq!(kernel.recipe(&invalid).unwrap_err().code, codes::RECIPE_SHAPE);
    assert_eq!(base.digest(), digest);

    // Accepted, the recipe derives a new snapshot and the base keeps its digest.
    let Derivation::Derived(next) = derive(&base, &one, &ProtectedSet::none()) else {
        panic!("an unprotected twist was refused");
    };
    assert_ne!(next.state.digest(), digest);
    assert_eq!(next.base, digest);
    assert_eq!(base.digest(), digest);
    assert_eq!(derived(&next.state, &[-1]).digest(), digest);
}

#[test]
fn net_protection_accepts_a_word_whose_net_map_is_the_identity() {
    let kernel = kernel();
    let every: Vec<i64> = (0..35).collect();
    let base = kernel.solved();
    let Derivation::Derived(derived) =
        derive(&base, &kernel.word(&[1, -1]).unwrap(), &kernel.protected_set(&every).unwrap())
    else {
        panic!("word([1, -1]) was refused under Net protection");
    };
    assert_eq!(derived.state.digest(), base.digest());
    assert_eq!((derived.effect.moved_stickers(), derived.effect.moved_pieces), (0, 0));
    assert!(derived.effect.support.is_empty());
    assert_eq!(derived.effect.net_sha256, c600_digest::sha256_hex(b""));
    assert_eq!((derived.effect.expansion_sha256.as_str(), derived.effect.primitive_count), (ONE_MINUS_ONE, 2));
    assert_eq!(derived.protection.verdict, Verdict::Clear);
    assert_eq!(derived.protection.protected, (0..35).collect::<Vec<usize>>());
}

#[test]
fn recipes_are_checked_whole_with_the_texts_of_0_4() {
    let kernel = kernel();
    let check = |text: &str, message: &str| {
        let refusal = kernel.recipe(&parse(text)).unwrap_err();
        assert_eq!((refusal.code, refusal.message.as_str()), (codes::RECIPE_SHAPE, message), "{text}");
    };
    check("[]", "Recipe must contain 1..256 steps");
    check("{}", "Recipe must contain 1..256 steps");
    check("[1]", "Bad recipe step");
    check(r#"[{"kind": "word"}]"#, "Word length must be 1..100000");
    check(r#"[{"kind": "word", "moves": []}]"#, "Word length must be 1..100000");
    check(r#"[{"kind": "word", "moves": 1}]"#, "Word length must be 1..100000");
    for letter in ["0", "1201", "-1201", "true", "1.0", "\"1\"", "null", "99999999999999999999"] {
        check(
            &format!(r#"[{{"kind": "word", "moves": [1, {letter}]}}]"#),
            "Primitive ID must be an integer in ±1..±1200",
        );
    }
    for fields in [
        r#""orbit": 35, "node": 0"#,
        r#""orbit": -1, "node": 0"#,
        r#""orbit": true, "node": 0"#,
        r#""node": 0"#,
        r#""orbit": 0, "node": 0, "sign": 0"#,
        r#""orbit": 0, "node": 0, "sign": null"#,
        r#""orbit": 0, "node": 0, "sign": 1.0"#,
    ] {
        check(&format!(r#"[{{"kind": "star", {fields}}}]"#), "Bad star fields");
    }
    let nodes = kernel.star_nodes(0).unwrap();
    for node in
        [r#", "node": -1"#.to_owned(), format!(r#", "node": {nodes}"#), r#", "node": 0.0"#.to_owned(), String::new()]
    {
        check(&format!(r#"[{{"kind": "star", "orbit": 0{node}}}]"#), "Frame node out of range");
    }
    check(r#"[{"kind": "jump"}]"#, "Only word and star steps are accepted");
    check(r#"[{"moves": [1]}]"#, "Only word and star steps are accepted");
    // A bad last step refuses the whole recipe.
    check(
        r#"[{"kind": "word", "moves": [1]}, {"kind": "star", "orbit": 0, "node": 0}, {"kind": "jump"}]"#,
        "Only word and star steps are accepted",
    );
}

/// Lead L4 (M4), the positive twins of the invalid recipe family in `tools/oracle/invalid_recipes.py`: a star without
/// `sign` is the star with sign 1, and members that 0.4's `Model.normalize` does not read are ignored, whatever they
/// hold. `tests/test_oracle.py` checks the family against 0.4 itself.
#[test]
fn a_missing_sign_and_extra_keys_are_accepted() {
    let kernel = kernel();
    let recipe = |text: &str| kernel.recipe(&parse(text)).unwrap();
    assert_eq!(recipe(r#"[{"kind": "star", "orbit": 3, "node": 5}]"#), star(3, 5, 1));
    let extra = r#""moves": [0], "x": {"y": [null, 1.5]}, "": "𐀀""#;
    assert_eq!(recipe(&format!(r#"[{{"kind": "star", "orbit": 3, "node": 5, "sign": -1, {extra}}}]"#)), star(3, 5, -1));
    let word = recipe(r#"[{"kind": "word", "moves": [1, -2], "orbit": -1, "node": 1.5, "sign": 0, "Kind": "jump"}]"#);
    assert_eq!(word, kernel.word(&[1, -2]).unwrap());
}

/// CT-PS1-12 (part: the 3,000,000 total over 100,000-letter word steps; the rest in
/// `ct_ps1_12_normalisation_keeps_literal_prefixes_and_the_total`).
#[test]
fn recipe_limits_are_those_of_0_4() {
    let kernel = kernel();
    let step = parse(r#"{"kind": "word", "moves": [1]}"#);
    assert_eq!(kernel.recipe(&Value::Array(vec![step.clone(); 256])).unwrap().primitive_count(), 256);
    assert_eq!(kernel.recipe(&Value::Array(vec![step; 257])).unwrap_err().message, "Recipe must contain 1..256 steps");

    let word = |n: usize| {
        json::object([("kind", Value::String("word".into())), ("moves", Value::Array(vec![Value::Int(-7); n]))])
    };
    assert_eq!(kernel.recipe(&Value::Array(vec![word(100_000)])).unwrap().primitive_count(), 100_000);
    assert_eq!(kernel.recipe(&Value::Array(vec![word(100_001)])).unwrap_err().message, "Word length must be 1..100000");
    assert_eq!(kernel.word(&vec![7; 100_000]).unwrap().primitive_count(), 100_000);
    assert_eq!(kernel.word(&vec![7; 100_001]).unwrap_err().message, "Word length must be 1..100000");
    assert_eq!(kernel.word(&[]).unwrap_err().message, "Word length must be 1..100000");
    assert_eq!(kernel.word(&[1200, -1200]).unwrap().primitive_count(), 2);
    assert_eq!(kernel.word(&[i64::MIN]).unwrap_err().message, "Primitive ID must be an integer in ±1..±1200");

    // The compatibility ceiling: thirty words of 100,000 letters pass and thirty-one do not.
    let long = word(100_000);
    assert_eq!(kernel.recipe(&Value::Array(vec![long.clone(); 30])).unwrap().primitive_count(), 3_000_000);
    let over = kernel.recipe(&Value::Array(vec![long; 31])).unwrap_err();
    assert_eq!(over.code, codes::PRIMITIVE_CEILING);
    assert_eq!(over.message, "Preview expansion exceeds 3,000,000 primitives; split the batch");
}

/// Lead L9 (M4): the ceiling counts a star's letters, the seed and twice the relocation and the path. With L the
/// star's length, 29 words of 100,000 letters, one of 100,000 - L and the star expand to exactly 3,000,000 letters;
/// one more letter is refused. `tests/test_oracle.py` checks the same recipes against 0.4's `Model.normalize`.
#[test]
fn the_ceiling_counts_star_letters() {
    let kernel = kernel();
    let node = kernel.star_nodes(0).unwrap() - 1;
    let last = star(0, node, 1);
    let length = last.primitive_count();
    let word = |n: usize| {
        json::object([("kind", Value::String("word".into())), ("moves", Value::Array(vec![Value::Int(7); n]))])
    };
    let recipe = |extra: usize| {
        let mut steps = vec![word(100_000); 29];
        steps.push(word(100_000 - length + extra));
        steps.push(parse(&format!(r#"{{"kind": "star", "orbit": 0, "node": {node}}}"#)));
        kernel.recipe(&Value::Array(steps))
    };
    let full = recipe(0).unwrap();
    assert_eq!(full.primitive_count(), 3_000_000);
    let letters = kernel.letters(&full).unwrap();
    assert_eq!(letters.len(), 3_000_000);
    assert_eq!(letters[3_000_000 - length..], kernel.letters(&last).unwrap());
    let over = recipe(1).unwrap_err();
    assert_eq!(over.code, codes::PRIMITIVE_CEILING);
    assert_eq!(over.message, "Preview expansion exceeds 3,000,000 primitives; split the batch");
}

#[test]
fn a_word_recipe_and_a_word_agree() {
    let kernel = kernel();
    let from_json = kernel.recipe(&parse(r#"[{"kind": "word", "moves": [3, -1200, 77]}]"#)).unwrap();
    let direct = kernel.word(&[3, -1200, 77]).unwrap();
    assert_eq!(from_json, direct);
    assert_eq!(direct.steps(), [Step::Word(vec![3, -1200, 77])]);
    assert_eq!(direct.model_id(), kernel.identity().model_id);
    assert_eq!(kernel.letters(&direct).unwrap(), [3, -1200, 77]);
    assert_eq!(kernel.effect(&from_json).unwrap(), kernel.effect_by_replay(&direct).unwrap());
}

/// CT-LB2-02 (part: the kernel half on 600-cell-Full; not run: the captured lock, LB2.3's coverage report and
/// `LB2:protection.incomplete_coverage`, which are LP3's library rows). A word with an identity net map and a
/// moving prefix passes Net and fails Strict. The refusal has KN2.11's shape: the first refused prefix, the outcome and
/// certificates, which the one-letter effect confirms. On the shipped model every generator moves a sticker of every
/// orbit, so Strict refuses any word at its first prefix once one orbit is protected; `tests/load.rs` tests later
/// prefixes on a changed copy.
#[test]
fn ct_lb2_02_strict_refuses_a_word_that_net_accepts() {
    let kernel = kernel();
    let (base, every) = (kernel.solved(), kernel.protected_set(&(0..35).collect::<Vec<i64>>()).unwrap());
    let recipe = kernel.word(&[1, -1]).unwrap();
    let Derivation::Derived(net) = derive(&base, &recipe, &every) else { panic!("Net refused word([1, -1])") };
    assert_eq!(net.state.digest(), base.digest());
    let Derivation::Refused(refused) = kernel.derive_strict(&base, &recipe, &every).unwrap() else {
        panic!("Strict accepted word([1, -1])");
    };
    assert_eq!((refused.first_refused_prefix, refused.protection.first_refused_prefix), (Some(1), Some(1)));
    assert_eq!((refused.protection.mode, refused.protection.verdict), (ProtectionMode::Strict, Verdict::Moves));
    assert_eq!((refused.refusal.code, refused.refusal.message.as_str()), (codes::ORBIT_WOULD_MOVE, PROTECTED_MESSAGE));
    assert_eq!((refused.base.as_str(), refused.word.as_str()), (base.digest(), ONE_MINUS_ONE));
    assert_eq!(refused.effect, net.effect);

    // The first refused prefix is word([1]): its conflicts are that letter's rows, and Net's refusal of word([1])
    // gives the same certificates, one per orbit, each a move of the letter.
    let one = kernel.word(&[1]).unwrap();
    let Derivation::Refused(single) = derive(&base, &one, &every) else { panic!("Net accepted word([1])") };
    assert_eq!((single.first_refused_prefix, single.protection.first_refused_prefix), (None, None));
    assert_eq!(refused.protection.conflicts, single.protection.conflicts);
    assert_eq!(refused.certificates, single.certificates);
    assert_eq!(refused.certificates.iter().map(|c| c.orbit).collect::<Vec<_>>(), (0..35).collect::<Vec<_>>());
    for certificate in &refused.certificates {
        let at = single.effect.source().binary_search(&certificate.source).expect("the letter moves the source");
        assert_eq!(single.effect.destination()[at], certificate.destination);
    }
    assert_eq!(kernel.strict_protection(&recipe, &every).unwrap(), refused.protection);
    // Every generator, not only 1, moves a sticker of each of the 35 orbits.
    for m in 1..=1_200 {
        let verdict = kernel.strict_protection(&kernel.word(&[m]).unwrap(), &every).unwrap();
        assert_eq!((verdict.first_refused_prefix, verdict.conflicts.len()), (Some(1), 35), "generator {m}");
    }

    // Without a protected orbit Strict clears every prefix and derives what Net derives.
    let clear = kernel.strict_protection(&recipe, &ProtectedSet::none()).unwrap();
    assert_eq!((clear.verdict, clear.first_refused_prefix, clear.conflicts.len()), (Verdict::Clear, None, 0));
    let star = star(4, 9, -1);
    let none = ProtectedSet::none();
    let (Derivation::Derived(strict), Derivation::Derived(net)) =
        (kernel.derive_strict(&base, &star, &none).unwrap(), derive(&base, &star, &none))
    else {
        panic!("a star was refused without protection");
    };
    assert_eq!((strict.state.digest(), &strict.effect), (net.state.digest(), &net.effect));
    assert_eq!((strict.protection.mode, net.protection.mode), (ProtectionMode::Strict, ProtectionMode::Net));
}

/// CT-LB2-04 (part: the kernel's `positions` row on 600-cell-Full, which reads identities, slots and orientations;
/// not run: exact completion, empty candidates, permitted residuals and colours, which are LP3's library rows). On
/// O03's scrambled initial state every piece's row agrees with the label array and with the progress rows that the
/// oracle replays check: the pieces out of position and those in position but turned are counted per orbit as 0.4
/// counts them.
#[test]
fn ct_lb2_04_positions_read_identities_slots_and_orientations() {
    let kernel = kernel();
    let every: Vec<i64> = (0..PIECES as i64).collect();
    let solved = kernel.positions(&kernel.solved(), &every).unwrap();
    assert!(solved.iter().all(|row| row.solved() && row.position_correct() && row.slots == row.home));
    assert_eq!(solved.iter().map(|row| row.home.len()).sum::<usize>(), SLOTS);
    assert_eq!(solved.iter().filter(|row| row.orbit < 0).count(), 600);
    let mut piece_of = vec![usize::MAX; SLOTS];
    for row in &solved {
        assert!(row.home.windows(2).all(|pair| pair[0] < pair[1]));
        for &slot in &row.home {
            piece_of[slot as usize] = row.piece;
        }
    }

    let initial = case("O03").get("initial").cloned().unwrap();
    let state = derived(&kernel.solved(), &integers(initial.get("moves")));
    let rows = kernel.positions(&state, &every).unwrap();
    let (mut position_wrong, mut orientation_wrong) = ([0usize; 35], [0usize; 35]);
    for row in &rows {
        for (&home, &slot) in row.home.iter().zip(&row.slots) {
            assert_eq!(state.labels()[slot as usize], home);
        }
        let place = row.position.expect("a twisted state keeps every piece together");
        assert!(row.slots.iter().all(|&slot| piece_of[slot as usize] == place));
        assert_eq!(solved[place].orbit, row.orbit);
        if row.orbit >= 0 && !row.position_correct() {
            position_wrong[row.orbit as usize] += 1;
        } else if row.orbit >= 0 && !row.solved() {
            orientation_wrong[row.orbit as usize] += 1;
        }
    }
    let progress = kernel.progress(&state).unwrap().rows;
    assert!(progress.iter().any(|row| row.position_wrong > 0) && progress.iter().any(|row| row.orientation_wrong > 0));
    for row in progress {
        let counted = (position_wrong[row.orbit], orientation_wrong[row.orbit]);
        assert_eq!((row.position_wrong, row.orientation_wrong), counted);
    }

    // Repeats are answered in order; a label array with one piece's stickers split has no position for it.
    let pair = kernel.positions(&state, &[7, 7]).unwrap();
    assert_eq!((&pair[0], &pair[1]), (&rows[7], &rows[7]));
    let mut multi = solved.iter().filter(|row| row.home.len() > 1);
    let (x, y) = (multi.next().unwrap(), multi.next().unwrap());
    let (a, b) = (x.home[0], y.home[0]);
    let mut labels: Vec<i32> = (0..SLOTS as i32).collect();
    labels.swap(a as usize, b as usize);
    let split = kernel.state_from_labels(labels).unwrap();
    let named = [x.piece as i64, y.piece as i64];
    let [first, second] = &kernel.positions(&split, &named).unwrap()[..] else { panic!("two rows") };
    assert_eq!((first.position, second.position, first.slots[0], second.slots[0]), (None, None, b, a));
    for bad in [-1, PIECES as i64] {
        let refusal = kernel.positions(&state, &[3, bad]).unwrap_err();
        assert_eq!(
            (refusal.code, refusal.message.as_str()),
            (codes::NAMED_OBJECT_INVALID, "Position must be an integer in 0..177119")
        );
    }
    assert!(kernel.positions(&state, &[]).unwrap().is_empty());
}

/// A seeded word of `n` letters, from the generator of `ct_ps1_12_normalisation_keeps_literal_prefixes_and_the_total`.
fn seeded_word(n: usize) -> Vec<i64> {
    let mut seed = 600u64;
    let mut letter = || {
        seed = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
        let m = (seed >> 33) % 2_400;
        if m < 1_200 {
            m as i64 + 1
        } else {
            1_199 - m as i64
        }
    };
    (0..n).map(|_| letter()).collect()
}

/// A recipe of `word` cut into word steps of the given sizes.
fn word_steps(word: &[i64], sizes: &[usize]) -> Recipe {
    let mut at = 0;
    let steps = sizes.iter().map(|&size| {
        let moves = word[at..at + size].iter().map(|&m| Value::Int(m)).collect();
        at += size;
        json::object([("kind", Value::String("word".into())), ("moves", Value::Array(moves))])
    });
    kernel().recipe(&Value::Array(steps.collect())).unwrap()
}

/// CT-LG2-14 (part: the kernel's streaming of positions and letters on 600-cell-Full; not run: compressed words and
/// their exact big-integer lengths, which are library code). Each recipe of O04, star steps included, streams in
/// chunks of several sizes: the positions run on without a gap, every chunk but the last is full, and the letters
/// equal the whole expansion and the oracle's independently expanded digest and count (0.4's `Model.expand`).
#[test]
fn ct_lg2_14_streamed_positions_and_letters_equal_the_oracle_expansion() {
    let kernel = kernel();
    let mut checked = 0;
    for step in case("O04").get("steps").and_then(Value::as_array).unwrap() {
        let recipe = kernel.recipe(step.get("recipe").unwrap()).unwrap();
        let whole = kernel.letters(&recipe).unwrap();
        for size in [1, 7, 1_000, 100_000] {
            let mut letters: Vec<i32> = Vec::new();
            for chunk in kernel.chunks(&recipe, NonZeroUsize::new(size).unwrap()).unwrap() {
                assert_eq!(chunk.start, letters.len() as u64);
                letters.extend(&chunk.letters);
                assert!(chunk.letters.len() == size || (letters.len() == whole.len() && !chunk.letters.is_empty()));
            }
            assert_eq!(letters, whole, "size {size}");
        }
        let expect = step.get("expect").unwrap();
        let bytes: Vec<u8> = whole.iter().flat_map(|m| m.to_le_bytes()).collect();
        let digest = c600_digest::sha256_hex(&bytes);
        assert_eq!(expect.get("expansion_sha256").and_then(Value::as_str), Some(digest.as_str()));
        assert_eq!(expect.get("primitive_count").and_then(Value::as_i64), Some(whole.len() as i64));
        checked += 1;
    }
    assert_eq!(checked, 25);
}

/// CT-KN2-07 (part: a 600-cell-Full word longer than a recipe's 100,000-letter word step passes through the replay
/// interface in chunks and equals the derivation of the whole word; not run: the recorded words of 672,417, 871,582
/// and 2,352,984 letters and the 13,231,266-letter stream, which are jumbling words that the jumbling half replays, and
/// any Full stream beyond 3,000,000 letters in the test profile) and CT-LB2-03 (part: the kernel half; not run: named
/// work-sheet reuse and the preview, which are LP3's). Chunking keeps the whole operation. Under the ceiling the total
/// counts over all chunks, and a push that passes 3,000,000 letters is refused whole, with no silent split, as is a
/// chunk with a bad letter. A refusal is known as soon as its prefix is pushed, and there is no verdict before the end.
#[test]
fn ct_kn2_07_and_ct_lb2_03_a_word_replays_in_chunks() {
    let kernel = kernel();
    let (base, none) = (derived(&kernel.solved(), &[17, -400, 1013]), ProtectedSet::none());
    let word = seeded_word(100_001);
    let Derivation::Derived(whole) = derive(&base, &word_steps(&word, &[100_000, 1]), &none) else {
        panic!("refused without protection");
    };
    let mut replay = kernel.replay(&base, &none).unwrap();
    for chunk in [&word[..1], &word[1..60_000], &word[60_000..]] {
        replay.push(chunk).unwrap();
    }
    assert_eq!((replay.position(), replay.first_refused_prefix()), (100_001, None));
    let Derivation::Derived(streamed) = replay.finish() else { panic!("the replay refused") };
    assert_eq!((streamed.state.digest(), streamed.base.as_str()), (whole.state.digest(), base.digest()));
    assert_eq!((&streamed.word, &streamed.effect), (&whole.word, &whole.effect));
    assert_eq!((streamed.protection.mode, streamed.protection.verdict), (ProtectionMode::Strict, Verdict::Clear));

    // Under protection the replay refuses as derive_strict does, from the first chunk on.
    let every = kernel.protected_set(&(0..35).collect::<Vec<i64>>()).unwrap();
    let mut replay = kernel.replay(&base, &every).unwrap();
    replay.push(&[5]).unwrap();
    assert_eq!(replay.first_refused_prefix(), Some(1));
    replay.push(&[-5, 9]).unwrap();
    let (Derivation::Refused(streamed), Derivation::Refused(strict)) =
        (replay.finish(), kernel.derive_strict(&base, &kernel.word(&[5, -5, 9]).unwrap(), &every).unwrap())
    else {
        panic!("a protected word was derived");
    };
    assert_eq!((streamed.first_refused_prefix, &streamed.certificates), (Some(1), &strict.certificates));
    assert_eq!((&streamed.effect, &streamed.protection), (&strict.effect, &strict.protection));
    assert_eq!((&streamed.word, &streamed.refusal, &streamed.base), (&strict.word, &strict.refusal, &strict.base));

    // The ceiling counts over chunks, and a refused chunk changes nothing.
    let mut replay = kernel.replay_under_ceiling(&base, &none).unwrap();
    replay.push(&[1]).unwrap();
    let over = replay.push(&vec![2; 3_000_000]).unwrap_err();
    assert_eq!((over.code, over.message.as_str()), (codes::PRIMITIVE_CEILING, CEILING));
    for bad in [&[3, 0][..], &[1_201], &[-1_201, 4]] {
        let refusal = replay.push(bad).unwrap_err();
        assert_eq!((refusal.code, refusal.message.as_str()), (codes::RECIPE_SHAPE, BAD_PRIMITIVE));
    }
    assert_eq!(replay.position(), 1);
    let Derivation::Derived(one) = replay.finish() else { panic!("word([1]) was refused") };
    assert_eq!(one.state.digest(), derived(&base, &[1]).digest());

    // A word of no letters derives the base state, even with every orbit protected.
    let Derivation::Derived(empty) = kernel.replay(&base, &every).unwrap().finish() else { panic!("refused") };
    assert_eq!((empty.state.digest(), empty.effect.primitive_count), (base.digest(), 0));
    assert_eq!(empty.word, c600_digest::sha256_hex(b""));
}

#[test]
fn protected_sets_name_moving_orbits() {
    let kernel = kernel();
    assert_eq!(kernel.protected_set(&[3, 1, 3]).unwrap().orbits(), [1, 3]);
    assert!(kernel.protected_set(&[]).unwrap().orbits().is_empty());
    // A nonempty set names this model; the empty set names no orbit and so no model.
    assert_eq!(kernel.protected_set(&[3]).unwrap().model_id(), Some(kernel.identity().model_id.as_str()));
    assert_eq!(kernel.protected_set(&[]).unwrap(), ProtectedSet::none());
    assert_eq!(ProtectedSet::none().model_id(), None);
    for bad in [35, -1] {
        let refusal = kernel.protected_set(&[2, bad]).unwrap_err();
        assert_eq!((refusal.code, refusal.message.as_str()), (codes::NAMED_OBJECT_INVALID, "Bad protected orbit list"));
    }
}

#[test]
fn star_expansion_and_its_inverse() {
    let kernel = kernel();
    for orbit in [0, 17, 34] {
        let node = kernel.star_nodes(orbit).unwrap() - 1;
        let plus = star(orbit, node, 1);
        let Step::Star(step) = &plus.steps()[0] else { panic!("not a star step") };
        assert_eq!((step.orbit(), step.node(), step.sign()), (orbit, node, 1));
        let letters = kernel.letters(&plus).unwrap();
        assert_eq!(letters.len(), plus.primitive_count());
        // invword(path) + invword(relocation) + seed + relocation + path
        let inverse_path: Vec<i32> = step.path().iter().rev().map(|m| -m).collect();
        assert!(letters.starts_with(&inverse_path) && letters.ends_with(step.path()));
        let inverted: Vec<i32> = letters.iter().rev().map(|m| -m).collect();
        assert_eq!(kernel.letters(&star(orbit, node, -1)).unwrap(), inverted);
    }
    assert_eq!(kernel.star_nodes(35), None);
}

#[test]
fn kn2_9_star_cache_agrees_with_replay_on_the_shortest_orbit() {
    let kernel = kernel();
    let orbit = (0..35).min_by_key(|&orbit| star(orbit, 0, 1).primitive_count()).unwrap();
    let nodes = kernel.star_nodes(orbit).unwrap();
    for node in [0, nodes / 2, nodes - 1] {
        for sign in [1, -1] {
            let recipe = star(orbit, node, sign);
            let cached = kernel.effect(&recipe).unwrap();
            assert_eq!(cached, kernel.effect_by_replay(&recipe).unwrap(), "orbit {orbit} node {node} sign {sign}");
            assert_eq!(cached, kernel.effect(&recipe).unwrap());
        }
    }
}

/// Lead L6 (M4): a star evicted from the 96-entry star cache is computed again with the same net map. A fresh kernel,
/// whose cache no other test touches, asks for nodes 0 to 96 of the shortest orbit, so node 0 leaves the cache; node 0
/// with sign -1 then agrees with replay, and node 0 with sign 1 with its first answer.
#[test]
fn an_evicted_star_is_recomputed_identically() {
    let fresh = Kernel::load(&repository().join("assets")).unwrap();
    let orbit = (0..35).min_by_key(|&orbit| star(orbit, 0, 1).primitive_count()).unwrap();
    assert!(fresh.star_nodes(orbit).unwrap() > 96, "orbit {orbit} has too few frame nodes");
    let first = fresh.effect(&star(orbit, 0, 1)).unwrap();
    for node in 1..=96 {
        fresh.effect(&star(orbit, node, 1)).unwrap();
    }
    let inverse = star(orbit, 0, -1);
    assert_eq!(fresh.effect(&inverse).unwrap(), fresh.effect_by_replay(&inverse).unwrap());
    assert_eq!(fresh.effect(&star(orbit, 0, 1)).unwrap(), first);
}

#[test]
fn progress_of_the_solved_state_and_of_one_twist() {
    let kernel = kernel();
    let solved = kernel.progress(&kernel.solved()).unwrap();
    assert_eq!(solved.rows.len(), 35);
    for (orbit, row) in solved.rows.iter().enumerate() {
        assert_eq!(row.orbit, orbit);
        assert_eq!(
            (row.solved, row.color_solved, row.position_wrong, row.orientation_wrong),
            (row.pieces, row.pieces, 0, 0)
        );
    }
    // Every piece but the 600 fixed cell centres belongs to a moving orbit.
    assert_eq!(solved.rows.iter().map(|row| row.pieces).sum::<usize>(), 177_120 - 600);
    // O06 ends on the solved state, so its last row holds the solved state's progress digest.
    let o06 = case("O06");
    let last =
        o06.get("steps").and_then(Value::as_array).and_then(|steps| steps.last()).unwrap().get("expect").unwrap();
    assert_eq!(last.get("state_hash").and_then(Value::as_str), Some(kernel.solved().digest()));
    assert_eq!(last.get("progress_sha256").and_then(Value::as_str), Some(solved.sha256.as_str()));

    // After one twist, the unsolved pieces of each orbit are the pieces the twist moves.
    let effect = kernel.effect(&kernel.word(&[1]).unwrap()).unwrap();
    let twisted = kernel.progress(&derived(&kernel.solved(), &[1])).unwrap();
    for row in &twisted.rows {
        let moved = effect.support.iter().find(|s| s.orbit == row.orbit as i32).map_or(0, |s| s.pieces);
        assert_eq!(row.pieces - row.solved, moved, "orbit {}", row.orbit);
    }
}

#[test]
fn build_hashes_every_source_file() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut files = vec!["Cargo.toml".to_owned()];
    let mut pending = vec![root.join("src")];
    while let Some(folder) = pending.pop() {
        for entry in std::fs::read_dir(folder).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                pending.push(path);
            } else {
                files.push(path.strip_prefix(root).unwrap().to_string_lossy().replace('\\', "/"));
            }
        }
    }
    files.sort();
    let listed: Vec<&str> =
        c600_kernel::SOURCES.iter().filter_map(|(name, _)| name.strip_prefix("c600-kernel/")).collect();
    assert_eq!(listed, files);
    for (name, bytes) in c600_kernel::SOURCES {
        assert_eq!(std::fs::read(root.join("..").join(name)).unwrap(), bytes, "{name}");
    }
    let build = c600_kernel::build();
    let digest = build.strip_prefix(&format!("c600-kernel {} sources-sha256:", env!("CARGO_PKG_VERSION"))).unwrap();
    assert!(digest.len() == 64 && digest.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')));
}

/// CT-KN1-04 (part: the 600-cell-Full API of M4, whose one level is labels; the pose level and its projections come
/// with the jumbling half, M5). An enumeration of the kernel's public functions finds no lift from a coarser level to
/// a finer one: a state comes only from `solved`, from `derive` on a label snapshot, or from `state_from_labels`, which
/// reads a stored state back from labels at the same level (M6's store; it decides no reachability), a snapshot's
/// field and constructor are private, and no public function names a pose.
#[test]
fn ct_kn1_04_no_public_call_lifts_a_coarser_level() {
    // A new level makes this match fail to compile, so that the enumeration below is read again.
    match Level::Labels {
        Level::Labels => {}
    }
    let source = include_str!("../src/lib.rs");
    let signatures: Vec<&str> = source.lines().filter_map(|line| line.trim_start().strip_prefix("pub fn ")).collect();
    assert!(signatures.len() >= 30, "the enumeration found {} public functions", signatures.len());
    let states: Vec<&str> = signatures
        .iter()
        .filter(|signature| {
            signature.split_once(" -> ").is_some_and(|(_, out)| out.contains("Snapshot") || out.contains("Deriv"))
        })
        .copied()
        .collect();
    assert_eq!(
        states,
        [
            "solved(&self) -> Snapshot {",
            "state_from_labels(&self, labels: Vec<i32>) -> Option<Snapshot> {",
            "derive(&self, base: &Snapshot, recipe: &Recipe, protected: &ProtectedSet) -> Result<Derivation, Error> {",
            "derive_strict(&self, base: &Snapshot, recipe: &Recipe, set: &ProtectedSet) -> Result<Derivation, Error> {",
            "finish(self) -> Derivation {",
        ]
    );
    for signature in &signatures {
        assert!(!signature.to_lowercase().contains("pose"), "{signature}");
    }
    // A state is held only by a derived state's public field and the kernel's private solved state, and a snapshot is
    // built only inside the crate.
    let fields: Vec<&str> = source.lines().map(str::trim).filter(|line| line.contains(": Snapshot")).collect();
    assert_eq!(fields, ["pub state: Snapshot,", "solved: Snapshot,"]);
    assert!(source.contains("pub struct Snapshot(Arc<SnapshotData>);"));
    assert!(source.contains("\n    fn new(model_id: &str, labels: Vec<i32>) -> Snapshot {"));
}

/// CT-PS1-04 (part: the labels half. `Digest<poses>` against J1 and the store helper is M5's and M6's. Confusing
/// levels or hashing render floats cannot be expressed through M4's API, which holds the one level `Labels`, builds a
/// snapshot only from labels and reads no render float, so no such refusal is run). `Digest<labels>` is the SHA-256
/// of the labels as little-endian int32 in slot order (PS1.6), which is `core.state_hash`: on the solved state in
/// `ct_kn1_05_labels_digest_is_the_oracle_state_hash`, on O03's scrambled initial state here, and on every row the
/// replays compare. A protection verdict carries its level.
#[test]
fn ct_ps1_04_labels_digest_is_core_state_hash() {
    let kernel = kernel();
    let initial = case("O03").get("initial").cloned().unwrap();
    let state = derived(&kernel.solved(), &integers(initial.get("moves")));
    assert_eq!(initial.get("state_hash").and_then(Value::as_str), Some(state.digest()));
    assert_ne!(state.digest(), kernel.solved().digest());
    let little: Vec<u8> = state.labels().iter().flat_map(|label| label.to_le_bytes()).collect();
    let big: Vec<u8> = state.labels().iter().flat_map(|label| label.to_be_bytes()).collect();
    assert_eq!(state.digest(), c600_digest::sha256_hex(&little));
    assert_ne!(state.digest(), c600_digest::sha256_hex(&big));
    let Derivation::Derived(next) = derive(&state, &kernel.word(&[1]).unwrap(), &ProtectedSet::none()) else {
        panic!("an unprotected twist was refused");
    };
    assert_eq!((next.protection.level, next.protection.mode), (Level::Labels, ProtectionMode::Net));
}

/// CT-PS1-12 (whole, with `strict_protection_judges_each_literal_prefix` in `tests/load.rs`, which runs the Strict
/// clause at later prefixes on a changed copy of the model). Normalisation keeps the literal steps in order (PS1.23):
/// no letter is reduced away, every prefix of a recipe expands to a prefix of its letters, and a word split into
/// chunks of up to 100,000 letters expands, digests, acts and is judged by Strict protection as the same letters. The
/// total stays at most 3,000,000 over every chunk, here and in `recipe_limits_are_those_of_0_4`, and through a star in
/// `the_ceiling_counts_star_letters`.
#[test]
fn ct_ps1_12_normalisation_keeps_literal_prefixes_and_the_total() {
    let kernel = kernel();
    let digest = |letters: &[i32]| {
        let bytes: Vec<u8> = letters.iter().flat_map(|m| m.to_le_bytes()).collect();
        c600_digest::sha256_hex(&bytes)
    };
    let steps = [
        r#"{"kind": "word", "moves": [1, -1, 5]}"#,
        r#"{"kind": "star", "orbit": 4, "node": 9, "sign": -1}"#,
        r#"{"kind": "word", "moves": [-5, 5]}"#,
        r#"{"kind": "star", "orbit": 4, "node": 9}"#,
    ];
    let prefix = |count: usize| kernel.recipe(&parse(&format!("[{}]", steps[..count].join(", ")))).unwrap();
    let mut expected = vec![1, -1, 5];
    expected.extend(kernel.letters(&star(4, 9, -1)).unwrap());
    expected.extend([-5, 5]);
    expected.extend(kernel.letters(&star(4, 9, 1)).unwrap());
    assert_eq!(kernel.letters(&prefix(4)).unwrap(), expected);
    for count in 1..=4 {
        let recipe = prefix(count);
        let letters = kernel.letters(&recipe).unwrap();
        assert_eq!(letters.len(), recipe.primitive_count());
        assert!(expected.starts_with(&letters), "prefix of {count} steps");
        assert_eq!(kernel.effect(&recipe).unwrap().expansion_sha256, digest(&letters));
    }

    // A seeded literal word of 250,000 letters in chunks of different sizes.
    let mut seed = 600u64;
    let word: Vec<i64> = (0..250_000)
        .map(|_| {
            seed = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
            let m = (seed >> 33) % 2_400;
            if m < 1_200 {
                m as i64 + 1
            } else {
                1_199 - m as i64
            }
        })
        .collect();
    let chunked = |sizes: &[usize]| {
        let mut at = 0;
        let mut steps = Vec::new();
        for &size in sizes {
            let moves = word[at..at + size].iter().map(|&m| Value::Int(m)).collect();
            steps.push(json::object([("kind", Value::String("word".into())), ("moves", Value::Array(moves))]));
            at += size;
        }
        kernel.recipe(&Value::Array(steps)).unwrap()
    };
    let literal: Vec<i32> = word.iter().map(|&m| m as i32).collect();
    for sizes in [&[100_000, 100_000, 50_000][..], &[50_000, 100_000, 100_000], &[1, 99_999, 100_000, 49_999, 1]] {
        let recipe = chunked(sizes);
        assert_eq!(recipe.primitive_count(), 250_000);
        assert_eq!(kernel.letters(&recipe).unwrap(), literal, "{sizes:?}");
    }
    // Nor does Strict protection: on the shipped model every letter moves every orbit, so each chunking is refused at
    // its first prefix (later prefixes on a changed copy in `tests/load.rs`).
    let protected = kernel.protected_set(&[0]).unwrap();
    for sizes in [&[100_000, 100_000, 50_000][..], &[1, 99_999, 100_000, 49_999, 1]] {
        assert_eq!(kernel.strict_protection(&chunked(sizes), &protected).unwrap().first_refused_prefix, Some(1));
    }
    // The net effect of a shorter word does not depend on its chunks either.
    let effect = |sizes: &[usize]| kernel.effect(&chunked(sizes)).unwrap();
    let whole = effect(&[600]);
    assert_eq!(whole.expansion_sha256, digest(&literal[..600]));
    assert_eq!(effect(&[200, 400]), whole);
    assert_eq!(effect(&[1, 599]), whole);

    // Thirty full chunks reach the total; one more letter in a chunk of its own is refused.
    let full =
        json::object([("kind", Value::String("word".into())), ("moves", Value::Array(vec![Value::Int(1); 100_000]))]);
    let mut steps = vec![full; 30];
    assert_eq!(kernel.recipe(&Value::Array(steps.clone())).unwrap().primitive_count(), 3_000_000);
    steps.push(parse(r#"{"kind": "word", "moves": [1]}"#));
    assert_eq!(kernel.recipe(&Value::Array(steps)).unwrap_err().code, codes::PRIMITIVE_CEILING);
}

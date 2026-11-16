//! Kernel tests against the model in `assets/` and the frozen oracle cases. `tests/test_kernel_oracle.py` at the
//! repository root replays all seven cases through the adapter in a release build and runs the full KN2.9 agreement
//! test there; these tests run the small cases O06 and O07, the first steps of O01 and a short agreement test through
//! the same API in the test profile.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use c600_kernel::json::{self, Value};
use c600_kernel::{
    archive, codes, Derivation, Effect, IdentityBlock, Kernel, ProtectedSet, Puzzle, Recipe, Snapshot, Step,
    SupportRow, Verdict, PROTECTED_MESSAGE,
};

/// `Digest<literal word>` of `[1, -1]`, which O06 records.
const ONE_MINUS_ONE: &str = "b15348c8f462384c01e83b6d499c6faf3f96808f5aa07c6bab4b65b36b4445d4";

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

#[test]
fn ct_kn2_01_full_o06_protection_chain() {
    assert_eq!(replay("O06", usize::MAX), 6);
}

#[test]
fn ct_kn2_01_full_o07_invalid_inputs() {
    assert_eq!(replay("O07", usize::MAX), 10);
}

#[test]
fn ct_kn2_01_full_o01_first_generators() {
    assert_eq!(replay("O01", 8), 8);
}

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

#[test]
fn a_word_recipe_and_a_word_agree() {
    let kernel = kernel();
    let from_json = kernel.recipe(&parse(r#"[{"kind": "word", "moves": [3, -1200, 77]}]"#)).unwrap();
    let direct = kernel.word(&[3, -1200, 77]).unwrap();
    assert_eq!(from_json, direct);
    assert_eq!(direct.steps(), [Step::Word(vec![3, -1200, 77])]);
    assert_eq!(direct.model_id(), kernel.identity().model_id);
    assert_eq!(kernel.letters(&direct), [3, -1200, 77]);
    assert_eq!(kernel.effect(&from_json).unwrap(), kernel.effect_by_replay(&direct).unwrap());
}

#[test]
fn protected_sets_name_moving_orbits() {
    let kernel = kernel();
    assert_eq!(kernel.protected_set(&[3, 1, 3]).unwrap().orbits(), [1, 3]);
    assert!(kernel.protected_set(&[]).unwrap().orbits().is_empty());
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
        let letters = kernel.letters(&plus);
        assert_eq!(letters.len(), plus.primitive_count());
        // invword(path) + invword(relocation) + seed + relocation + path
        let inverse_path: Vec<i32> = step.path().iter().rev().map(|m| -m).collect();
        assert!(letters.starts_with(&inverse_path) && letters.ends_with(step.path()));
        let inverted: Vec<i32> = letters.iter().rev().map(|m| -m).collect();
        assert_eq!(kernel.letters(&star(orbit, node, -1)), inverted);
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

//! Compressed-word expansion (`kn6-expansion/1`): a straight-line program, its expanded length and digest, and
//! optionally the replay of the expanded word.
//!
//! A program is a list of rules. A rule is a list of items: a literal letter, or a reference to an earlier rule, which
//! may be inverted and repeated. References point only backwards, so every program is finite and acyclic. The check
//! is streamed:
//!
//! 1. every rule's length is computed with checked arithmetic, and the root's length must equal the claimed length
//!    before a single letter is produced; a length past 2⁶⁴ − 1 fails;
//! 2. the root is expanded depth first on an explicit stack bounded by the number of rules, and each letter goes
//!    straight to the hash and, with `replay`, to the engine; the expanded word is never held;
//! 3. the SHA-256 of the expanded word must equal the claimed digest; with `replay`, the start must be the stated one,
//!    every letter of a jumbling word admissible, and the end digest the stated one.
//!
//! The alphabet `full` has 600-cell-Full's primitive letters ±1 to ±1,200, hashed as little-endian int32 (the kernel's
//! `Digest<literal word>`), with inverse −m. The alphabet `ia16` has the I_a letters as stage 13's codes
//! `60 · cap + item`, hashed as little-endian uint16 (an `ia16` word file's bytes), with the inverse letter
//! `[cap, item']` where item' is the menu item of the inverse rotation, read from the exact menu.
//!
//! ```text
//! { "format": "kn6-expansion/1",
//!   "alphabet": "full" | "ia16",
//!   "inputs": {"menu": sha256, "start": sha256},          (menu for ia16; start optional, with replay)
//!   "menu": {"identity": ..., "order": ...},               (ia16)
//!   "rules": [[letter | {"rule": k, "inverse": bool, "times": n}, ...], ...],
//!   "root": k,
//!   "claims": {"length": n, "sha256": ...},
//!   "replay": {"start": {"digest": ...}, "end": {"digest": ...}} }
//! ```

use c600_jumble::nu::{I_A_MENU_IDENTITY, I_A_ORDER_DIGEST};
use c600_jumble::state::Status;
use c600_kernel::json::Value;
use c600_kernel::{Derivation, ProtectedSet};

use super::{Checker, Needs};
use crate::claims::{
    array, at, count, document, dot, field, hex64, int, keys, object, poll, reject, require_format, string, Run, Step,
    Stop,
};
use crate::replay::{inverse_item, primitive, read_full_state, read_menu, read_state, Jumbling};
use crate::report::{codes, Sources, Span};
use crate::Engines;

pub const FORMAT: &str = "kn6-expansion/1";

/// I_a's 60 items per cap: an `ia16` code is `60 · cap + item`.
const IA16_ITEMS: u32 = 60;

pub static SOURCES: Sources = Sources::new(
    "kn6.expansion/1",
    &[
        ("c600-checkers/Cargo.toml", include_bytes!("../../Cargo.toml")),
        ("c600-checkers/src/claims.rs", include_bytes!("../claims.rs")),
        ("c600-checkers/src/kinds/expansion.rs", include_bytes!("expansion.rs")),
        ("c600-checkers/src/kinds/mod.rs", include_bytes!("mod.rs")),
        ("c600-checkers/src/lib.rs", include_bytes!("../lib.rs")),
        ("c600-checkers/src/replay.rs", include_bytes!("../replay.rs")),
        ("c600-checkers/src/report.rs", include_bytes!("../report.rs")),
    ],
);

pub static CHECKER: Checker = Checker { sources: &SOURCES, needs: Needs::BOTH, run };

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Alphabet {
    Full,
    Ia16,
}

#[derive(Clone, Copy, Debug)]
enum Item {
    Letter(i64),
    Rule { rule: usize, inverse: bool, times: u64 },
}

/// One open rule on the expansion stack.
struct Frame {
    rule: usize,
    next: usize,
    left: u64,
    inverse: bool,
}

/// The letter sink of the expansion: the hash, and the engine when the word is replayed.
enum Engine<'e> {
    None,
    Full { replay: c600_kernel::Replay<'e>, buffer: Vec<i64> },
    Jumbling { j: Jumbling<'e> },
}

fn run(engines: &Engines, run: &mut Run) -> Step<()> {
    let doc = document(run.cert.bytes)?;
    require_format(&doc, FORMAT)?;
    let alphabet = match string(field(&doc, "alphabet", "alphabet")?, "alphabet")? {
        "full" => Alphabet::Full,
        "ia16" => Alphabet::Ia16,
        _ => return Err(reject("alphabet", "the alphabet is full or ia16")),
    };
    let required: &[&str] = match alphabet {
        Alphabet::Full => &["format", "alphabet", "inputs", "rules", "root", "claims"],
        Alphabet::Ia16 => &["format", "alphabet", "inputs", "menu", "rules", "root", "claims"],
    };
    keys(&doc, "certificate", required, &["replay"])?;
    run.check_bindings(&doc, &["start"])?;

    // The I_a menu gives ia16 letters their inverses and the jumbling replay its twists.
    let menu = match alphabet {
        Alphabet::Full => None,
        Alphabet::Ia16 => {
            let ctx = engines.jumble.ok_or_else(|| reject("alphabet", "the jumbling engine is absent"))?;
            let menu = read_menu(ctx, run, "menu")?;
            let stated = field(&doc, "menu", "menu")?;
            keys(stated, "menu", &["identity", "order"], &[])?;
            run.claim(
                "menu.identity",
                &hex64(field(stated, "identity", "menu")?, "menu.identity")?.to_owned(),
                &menu.identity,
            )?;
            run.claim("menu.order", &hex64(field(stated, "order", "menu")?, "menu.order")?.to_owned(), &menu.order)?;
            run.claim("menu.ia16", &I_A_MENU_IDENTITY.to_owned(), &menu.identity)?;
            run.claim("menu.ia16_order", &I_A_ORDER_DIGEST.to_owned(), &menu.order)?;
            Some(menu)
        }
    };
    let inverse: Vec<u32> = match &menu {
        None => Vec::new(),
        Some(menu) => (0..menu.items.len())
            .map(|i| inverse_item(menu, i).map(|j| j as u32))
            .collect::<Option<Vec<_>>>()
            .ok_or_else(|| reject("menu", "the menu is not inverse-closed"))?,
    };

    let rules = read_rules(&doc, alphabet)?;
    let root = usize::try_from(int(field(&doc, "root", "root")?, "root")?)
        .ok()
        .filter(|&r| r < rules.len())
        .ok_or_else(|| reject("root", "the root names a rule"))?;

    // 1. Lengths, before any letter.
    let mut lengths: Vec<u64> = Vec::with_capacity(rules.len());
    for (r, rule) in rules.iter().enumerate() {
        let mut total = 0u64;
        for item in rule {
            let add = match *item {
                Item::Letter(_) => Some(1),
                Item::Rule { rule, times, .. } => lengths[rule].checked_mul(times),
            };
            total = add
                .and_then(|add| total.checked_add(add))
                .ok_or_else(|| reject("claims.length", format!("rule {r} expands past 2^64 - 1 letters")))?;
        }
        lengths.push(total);
    }
    let claims = field(&doc, "claims", "claims")?;
    keys(claims, "claims", &["length", "sha256"], &[])?;
    let length = lengths[root];
    run.claim("claims.length", &count(field(claims, "length", "claims")?, "claims.length")?, &length)?;
    let stated_sha = hex64(field(claims, "sha256", "claims")?, "claims.sha256")?.to_owned();

    // The replay's start, before the expansion.
    let replay_doc = doc.get("replay");
    let mut engine = match replay_doc {
        None => Engine::None,
        Some(replay_doc) => {
            keys(replay_doc, "replay", &["start", "end"], &[])?;
            let start = field(replay_doc, "start", "replay")?;
            keys(start, "replay.start", &["digest"], &[])?;
            let stated = hex64(field(start, "digest", "replay.start")?, "replay.start.digest")?.to_owned();
            match alphabet {
                Alphabet::Full => {
                    let kernel =
                        engines.kernel.ok_or_else(|| reject("replay", "the 600-cell-Full kernel is absent"))?;
                    let state = read_full_state(kernel, run, "start")?;
                    run.compare(
                        "replay.start.digest",
                        &stated,
                        &state.digest().to_owned(),
                        codes::START_TABLE_MISMATCH,
                    )?;
                    let replay = kernel
                        .replay(&state, &ProtectedSet::none())
                        .map_err(|refusal| reject("replay", format!("the kernel refused the replay: {refusal}")))?;
                    Engine::Full { replay, buffer: Vec::with_capacity(4096) }
                }
                Alphabet::Ia16 => {
                    let ctx = engines.jumble.ok_or_else(|| reject("replay", "the jumbling engine is absent"))?;
                    let menu = menu.clone().expect("an ia16 menu");
                    let state = read_state(ctx, run, &menu, "start")?;
                    let j = Jumbling::new(ctx, menu, state);
                    run.compare("replay.start.digest", &stated, &j.digest(), codes::START_TABLE_MISMATCH)?;
                    Engine::Jumbling { j }
                }
            }
        }
    };

    // 2. The streamed expansion.
    let mut hash = c600_digest::Sha256::new();
    let mut position = 0u64;
    let mut emit = |letter: i64, engine: &mut Engine, position: &mut u64| -> Step<()> {
        // Each letter produced, and each 600-cell-Full chunk applied, is charged first (`claims::COST_RULE`): under a
        // job's budget a long expansion stops here as the job's cut-off, never as a verdict.
        poll(1)?;
        match alphabet {
            Alphabet::Full => hash.update(&(letter as i32).to_le_bytes()),
            Alphabet::Ia16 => hash.update(&(letter as u16).to_le_bytes()),
        }
        match engine {
            Engine::None => {}
            Engine::Full { replay, buffer } => {
                buffer.push(letter);
                if buffer.len() == buffer.capacity() {
                    poll(buffer.len() as u64)?;
                    replay.push(buffer).map_err(|r| reject("replay", format!("the kernel refused a letter: {r}")))?;
                    buffer.clear();
                }
            }
            Engine::Jumbling { j } => {
                let (cap, item) = ((letter as u32 / IA16_ITEMS) as usize, (letter as u32 % IA16_ITEMS) as usize);
                let claim = format!("replay.letters[{position}]");
                let twist = j.twist(cap, item, &claim)?;
                let outcome = j.apply(&twist, &claim)?;
                if outcome.status != Status::Admissible {
                    return Err(reject(claim, format!("the letter is {}", outcome.status.as_str())));
                }
            }
        }
        *position += 1;
        Ok(())
    };
    let invert = |letter: i64| -> i64 {
        match alphabet {
            Alphabet::Full => -letter,
            Alphabet::Ia16 => {
                let (cap, item) = (letter as u32 / IA16_ITEMS, letter as u32 % IA16_ITEMS);
                (cap * IA16_ITEMS + inverse[item as usize]) as i64
            }
        }
    };
    let mut stack = vec![Frame { rule: root, next: 0, left: 1, inverse: false }];
    while let Some(frame) = stack.last_mut() {
        let rule = &rules[frame.rule];
        if frame.next == rule.len() {
            frame.left -= 1;
            frame.next = 0;
            if frame.left == 0 || rule.is_empty() {
                stack.pop();
            }
            continue;
        }
        let index = if frame.inverse { rule.len() - 1 - frame.next } else { frame.next };
        frame.next += 1;
        let flipped = frame.inverse;
        match rule[index] {
            Item::Letter(letter) => {
                let letter = if flipped { invert(letter) } else { letter };
                if let Err(stop) = emit(letter, &mut engine, &mut position) {
                    return Err(fail_traced(run, stop));
                }
            }
            Item::Rule { rule, inverse, times } => {
                if times > 0 && lengths[rule] > 0 {
                    stack.push(Frame { rule, next: 0, left: times, inverse: inverse != flipped });
                }
            }
        }
    }
    let word = c600_digest::hex(&hash.finish());
    let span = Span { word: word.clone(), from: 0, to: position };
    if stated_sha != word {
        let stop = reject("claims.sha256", format!("stated {stated_sha}, computed {word}"));
        return Err(run.failed(stop, &word));
    }
    run.held_span("claims.sha256", &word, span.clone());

    // 3. The replay's end.
    if let Some(replay_doc) = replay_doc {
        let end = field(replay_doc, "end", "replay")?;
        keys(end, "replay.end", &["digest"], &[])?;
        let stated = hex64(field(end, "digest", "replay.end")?, "replay.end.digest")?.to_owned();
        let computed = match engine {
            Engine::None => unreachable!("a replay has an engine"),
            Engine::Full { mut replay, buffer } => {
                poll(buffer.len() as u64)?;
                replay.push(&buffer).map_err(|r| reject("replay", format!("the kernel refused a letter: {r}")))?;
                match replay.finish() {
                    Derivation::Derived(derived) => derived.state.digest().to_owned(),
                    Derivation::Refused(refused) => {
                        return Err(reject("replay", format!("the replay was refused: {}", refused.refusal)))
                    }
                }
            }
            Engine::Jumbling { j } => j.digest(),
        };
        if stated != computed {
            return Err(
                run.failed(reject("replay.end.digest", format!("stated {stated}, computed {computed}")), computed)
            );
        }
        run.held_span("replay.end.digest", computed, span);
    }
    Ok(())
}

fn fail_traced(run: &mut Run, stop: Stop) -> Stop {
    run.failed(stop, "refused")
}

fn read_rules(doc: &Value, alphabet: Alphabet) -> Step<Vec<Vec<Item>>> {
    let mut rules = Vec::new();
    for (r, rule) in array(field(doc, "rules", "rules")?, "rules")?.iter().enumerate() {
        let path = at("rules", r);
        let mut items = Vec::new();
        for (k, item) in array(rule, &path)?.iter().enumerate() {
            let claim = at(&path, k);
            items.push(match item {
                Value::Int(_) => Item::Letter(match alphabet {
                    Alphabet::Full => primitive(item, &claim)?,
                    Alphabet::Ia16 => {
                        let code = int(item, &claim)?;
                        if !(0..600 * IA16_ITEMS as i64).contains(&code) {
                            return Err(reject(&claim, "an ia16 code is 0 to 35,999"));
                        }
                        code
                    }
                }),
                Value::Object(_) => {
                    keys(item, &claim, &["rule"], &["inverse", "times"])?;
                    let map = object(item, &claim)?;
                    let rule = usize::try_from(int(field(item, "rule", &claim)?, &dot(&claim, "rule"))?)
                        .ok()
                        .filter(|&k| k < r)
                        .ok_or_else(|| reject(dot(&claim, "rule"), "a reference names an earlier rule"))?;
                    let inverse = match map.get("inverse") {
                        None => false,
                        Some(Value::Bool(b)) => *b,
                        Some(_) => return Err(reject(dot(&claim, "inverse"), "expected true or false")),
                    };
                    let times = match map.get("times") {
                        None => 1,
                        Some(value) => count(value, &dot(&claim, "times"))?,
                    };
                    Item::Rule { rule, inverse, times }
                }
                _ => return Err(reject(&claim, "an item is a letter or a reference")),
            });
        }
        rules.push(items);
    }
    if rules.is_empty() {
        return Err(reject("rules", "a program has at least one rule"));
    }
    Ok(rules)
}

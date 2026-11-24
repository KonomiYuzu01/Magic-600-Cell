//! `s17.rr`: stage 17's routing records (family RR, `rr.py`; the driver P***, `pxrr.py`; the transport stops at the
//! 7304 stop) and the two recorded gate inputs of its acceptance test (`scripts/test_rr.py`, H2 and H2b).
//!
//! # The tails of Lemma RR against brute force, at the recorded gate inputs (KN6.1 reading B)
//!
//! After a routing word that releases a blocked grip r, family RR searches only the tails of Lemma RR in the five forms
//! its reductions leave: (a) a lowering at once; (b) a retained letter at a clean grip y other than r, then a lowering
//! at a partner of y; (c) a letter at r, then a lowering at a partner of r; (d) a letter at r, a letter at a clean grip
//! y, then a lowering at a partner of y, with r a partner of y or of the target; (e) a letter at a clean partner of r, a
//! letter at r, then a lowering at a partner of r. The stage's acceptance test compares the best (terminal Φ, length)
//! per target of these tails with a brute force over the whole tail language (every word of at most two retained
//! letters at clean grips, at most one of them at a grip other than r, each admissible when it acts, then every
//! lowering at a grip clean after it, never at the last letter's grip) at two recorded inputs only, both at the 7306
//! jam: H2, after `561:1,598:4` (r = 536), and H2b, after `561:1,598:4,536:11,233:6,240:3,240:9` (r = 90, blocked
//! before `240:9`, with the lowering at 370 right after the release reaching Φ 1268 in one letter).
//!
//! A `kn6-rr-gate/1` certificate states one such input: `seed`, the routing `word` after the jam, the `released` grip,
//! the jam's `phi`, `lowers` (the tail language has a lowering), `stated` (null, or a `target` with its best `[terminal
//! Φ, length]` as the test states it) and `agree` (the tails' best equals the brute force's at every target). The
//! seed, word and released grip must be one of the two recorded inputs ([`RECORDED`]): the gate checks those and is
//! never a search. Inputs: `jams`, `table` (the seed's D2 end table) and `menu` (I_a). The jam is replayed from its
//! exact table
//! ([`crate::archive::s16::release::jam_state`]), then the word letter by letter: each letter a retained item (1 to 11)
//! and admissible, the released grip with a straddler before the last letter and none after the word. The full label
//! table of that state is classified exactly ([`crate::archive::s15::labels`]); both searches run on it in the label
//! calculus below, and their maps are compared target by target. A pass is a statement about the recorded input only,
//! not about Lemma RR at any other state, and nothing here chooses, ranks or outputs a word: best values are compared,
//! and no word is reported.
//!
//! ## The label calculus
//!
//! A retained letter at a clean grip g is a K⁺ element k that fixes g, and the engine replaces the pose P of every piece
//! inside cap g by k·P. The caps are permuted by K⁺, so a moved piece is inside, straddling or outside cap k·e exactly
//! when it was so for cap e (Lemma L): its label row is permuted by k's action on the poles, and every other row stays.
//! The calculus keeps, from the exact table, every piece off the lattice that straddles some cap or has left height at
//! least 2 (a lattice piece has left height 0 and straddles nothing, and so does its image). Poses are exact matrices
//! and k·P is formed exactly; heights and branches are the engine's tree facts of each pose, and a pose whose left
//! height differs from its parent's stops the check. S_x, F_x and max D_x are then read as
//! [`crate::archive::s15::grips`] defines them. D's lowering test of a pose P at grip x reads the left heights of the
//! jumble items Λ applied to F_x⁻¹P, where F_x is the retained frame of x: the engine's jumble at x is F_x Λ F_x⁻¹, and a
//! left factor in K⁺ keeps the left height (checked on F_x⁻¹P itself). R4's tests compare the calculus with the
//! engine's own classification and observables before and after letters.
//!
//! # Bounded records
//!
//! The release searches (`s16-rr/1`), the driver runs (`s16-pxrr/1`) and the transport stops (`s16-transport/1`) of the
//! stage are `not_applicable`, each with the bounds read from the record: a negative within them, never for all words,
//! and no KN6 checker of a search's coverage exists (LB5.15). Their chosen descents are the PX3 and RR-7304
//! certificates, which `s17.macro` checks. A record of none of these forms fails at the field that departs from them.

use std::collections::{BTreeMap, HashMap};
use std::ops::RangeInclusive;
use std::sync::Arc;

use c600_jumble::locality::Locality;
use c600_jumble::model::{NP, POLES};
use c600_jumble::observe::Tables;
use c600_jumble::q5::{matmul, transpose, Mat4};
use c600_jumble::state::{Pose, Status};
use c600_kernel::json::Value;

use crate::archive::s15::grips::straddlers;
use crate::archive::s15::jam::{boolean, letters, parse_word, show, Letter};
use crate::archive::s15::labels::LabelTable;
use crate::archive::s16::release::jam_state;
use crate::archive::SubChecker;
use crate::claims::{array, at, document, field, int, keys, not_applicable, reject, string, Run, Step};
use crate::kinds::{Checker, Needs};
use crate::replay::{left_facts, phi, tree_tables, Jumbling};
use crate::report::Sources;
use crate::Engines;

/// The checker-owned format of a recorded gate input.
pub const GATE: &str = "kn6-rr-gate/1";
const RR: &str = "s16-rr/1";
const PXRR: &str = "s16-pxrr/1";
const TRANSPORT: &str = "s16-transport/1";

/// The retained items of menu I_a at every grip.
pub const RETAINED: RangeInclusive<usize> = 1..=11;

pub static SOURCES: Sources = Sources::new(
    "s17.rr",
    &[
        ("c600-checkers/Cargo.toml", include_bytes!("../../../Cargo.toml")),
        ("c600-checkers/src/archive/s15/grips.rs", include_bytes!("../s15/grips.rs")),
        ("c600-checkers/src/archive/s15/jam.rs", include_bytes!("../s15/jam.rs")),
        ("c600-checkers/src/archive/s15/labels.rs", include_bytes!("../s15/labels.rs")),
        ("c600-checkers/src/archive/s16/release.rs", include_bytes!("../s16/release.rs")),
        ("c600-checkers/src/archive/s17/rr.rs", include_bytes!("rr.rs")),
        ("c600-checkers/src/claims.rs", include_bytes!("../../claims.rs")),
        ("c600-checkers/src/kinds/mod.rs", include_bytes!("../../kinds/mod.rs")),
        ("c600-checkers/src/lib.rs", include_bytes!("../../lib.rs")),
        ("c600-checkers/src/replay.rs", include_bytes!("../../replay.rs")),
        ("c600-checkers/src/report.rs", include_bytes!("../../report.rs")),
    ],
);

pub static SUB: SubChecker = SubChecker {
    checker: Checker { sources: &SOURCES, needs: Needs::JUMBLE, run },
    stage: 17,
    family: "S17 family RR: the five tails against brute force at the recorded gate inputs H2 and H2b (no lemma); \
             RR, P*** and transport stops keep their bounds",
};

fn run(engines: &Engines, run: &mut Run) -> Step<()> {
    let doc = document(run.cert.bytes)?;
    const COMMON: [&str; 7] = ["format", "seed", "table_digest", "word", "digest", "phi", "options"];
    let with = |more: &[&'static str]| COMMON.iter().chain(more).copied().collect::<Vec<&str>>();
    match doc.get("format").and_then(Value::as_str) {
        Some(GATE) => gate(engines, run, &doc),
        Some(RR) => {
            keys(&doc, "record", &with(&["counts", "pairs", "best_per_target", "complete"]), &["cpu_s"])?;
            Err(not_applicable(rr_scope(&doc)?))
        }
        Some(PXRR) => {
            let fields = with(&["start", "segments", "start_macros", "status", "reason", "complete"]);
            keys(&doc, "record", &fields, &["cpu_s"])?;
            Err(not_applicable(pxrr_scope(&doc)?))
        }
        Some(TRANSPORT) => {
            let fields = with(&["extra_prefix", "K", "minimal", "counts", "best_per_target", "complete"]);
            keys(&doc, "record", &fields, &["cpu_s"])?;
            Err(not_applicable(transport_scope(&doc)?))
        }
        Some(other) => {
            Err(reject("format", format!("the record's format is {other}, not {GATE}, {RR}, {PXRR} or {TRANSPORT}")))
        }
        None => Err(reject("format", "the record states no format")),
    }
}

// --------------------------------------------------------------------------------------------------- the scopes

/// A member of an object field, by its dotted claim path.
fn member<'v>(doc: &'v Value, outer: &str, key: &str) -> Step<&'v Value> {
    field(field(doc, outer, outer)?, key, &format!("{outer}.{key}"))
}

/// A number as written (an integer or a decimal).
fn number(value: &Value, claim: &str) -> Step<String> {
    match value {
        Value::Int(n) => Ok(n.to_string()),
        Value::Number(lexeme) => Ok(lexeme.clone()),
        _ => Err(reject(claim, "expected a number")),
    }
}

/// The number of letters of the record's prefix word.
fn prefix_letters(doc: &Value) -> Step<usize> {
    Ok(parse_word(string(field(doc, "word", "word")?, "word")?, "word")?.len())
}

fn rr_scope(doc: &Value) -> Step<String> {
    array(field(doc, "pairs", "pairs")?, "pairs")?;
    array(field(doc, "best_per_target", "best_per_target")?, "best_per_target")?;
    let count = |outer: &str, key: &str| int(member(doc, outer, key)?, &format!("{outer}.{key}"));
    Ok(format!(
        "a release search of family RR at {} after a prefix of {} letters (carrier actions {} to {}, at most {} helpers, \
         pairs {}; {} words, {} releases, {} hits; complete {}): a negative for these bounds only, not for all words; \
         its best words are leads that only a macro certificate certifies; no KN6 checker of the search's coverage \
         exists (LB5.15)",
        string(field(doc, "seed", "seed")?, "seed")?,
        prefix_letters(doc)?,
        count("options", "nmin")?,
        count("options", "nmax")?,
        count("options", "helpers")?,
        string(member(doc, "options", "pairs")?, "options.pairs")?,
        count("counts", "words")?,
        count("counts", "releases")?,
        count("counts", "hits")?,
        boolean(field(doc, "complete", "complete")?, "complete")?,
    ))
}

fn pxrr_scope(doc: &Value) -> Step<String> {
    let segments = array(field(doc, "segments", "segments")?, "segments")?.len();
    array(field(doc, "start_macros", "start_macros")?, "start_macros")?;
    Ok(format!(
        "a run of the driver P*** at {} after a prefix of {} letters ({segments} segments, status {}, reason {}, \
         complete {}, CPU limit {} s): its stop is a stop of the declared families within their bounds, not a negative \
         for all words; its chosen descents are the PX3 certificates, which s17.macro checks; no KN6 checker of the \
         search's coverage exists (LB5.15)",
        string(field(doc, "seed", "seed")?, "seed")?,
        prefix_letters(doc)?,
        string(field(doc, "status", "status")?, "status")?,
        string(field(doc, "reason", "reason")?, "reason")?,
        boolean(field(doc, "complete", "complete")?, "complete")?,
        number(member(doc, "options", "cpu_limit")?, "options.cpu_limit")?,
    ))
}

fn transport_scope(doc: &Value) -> Step<String> {
    array(field(doc, "best_per_target", "best_per_target")?, "best_per_target")?;
    let count = |key: &str| int(member(doc, "counts", key)?, &format!("counts.{key}"));
    Ok(format!(
        "a transport stop of family X at {} after a prefix of {} letters (K {}, minimal {}; {} letters, {} leaf tests, \
         {} hits; complete {}): a negative for K retained letters and one lowering only; its reductions rest on their \
         proofs and are not promoted to a lemma; no KN6 checker of the search's coverage exists (LB5.15)",
        string(field(doc, "seed", "seed")?, "seed")?,
        prefix_letters(doc)?,
        int(field(doc, "K", "K")?, "K")?,
        boolean(field(doc, "minimal", "minimal")?, "minimal")?,
        count("letters")?,
        count("leaf_tests")?,
        count("hits")?,
        boolean(field(doc, "complete", "complete")?, "complete")?,
    ))
}

// ----------------------------------------------------------------------------------------------------- the gate

/// A grip number.
fn grip(value: &Value, claim: &str) -> Step<usize> {
    usize::try_from(int(value, claim)?)
        .ok()
        .filter(|&x| x < POLES)
        .ok_or_else(|| reject(claim, format!("a grip is 0 to {}", POLES - 1)))
}

/// The recorded gate inputs of `test_rr.py`: name, seed, the word after the jam, and the released grip.
pub const RECORDED: [(&str, &str, &[Letter], usize); 2] = [
    ("H2", "7306", &[(561, 1), (598, 4)], 536),
    ("H2b", "7306", &[(561, 1), (598, 4), (536, 11), (233, 6), (240, 3), (240, 9)], 90),
];

/// The best (terminal Φ, length) per target.
pub type Best = BTreeMap<usize, (i64, usize)>;

fn show_best(best: &Best) -> String {
    best.iter().map(|(x, (p, l))| format!("{x}:{p}/{l}")).collect::<Vec<_>>().join(",")
}

fn gate(engines: &Engines, run: &mut Run, doc: &Value) -> Step<()> {
    keys(doc, "certificate", &["format", "seed", "word", "released", "phi", "lowers", "stated", "agree"], &[])?;
    let seed = string(field(doc, "seed", "seed")?, "seed")?.to_owned();
    let word: Vec<Letter> = letters(field(doc, "word", "word")?, "word")?;
    if word.is_empty() {
        return Err(reject("word", "the routing word is empty"));
    }
    let r = grip(field(doc, "released", "released")?, "released")?;
    let recorded = RECORDED.iter().find(|(_, s, w, _)| *s == seed && *w == word.as_slice()).ok_or_else(|| {
        reject(
            "word",
            "the seed and word are no recorded gate input of test_rr.py (H2, H2b); the gate is never a search",
        )
    })?;
    if recorded.3 != r {
        return Err(reject("released", format!("{} releases grip {}", recorded.0, recorded.3)));
    }
    run.held("word", format!("the recorded gate input {}", recorded.0));
    let stated = match field(doc, "stated", "stated")? {
        Value::Null => None,
        value => {
            keys(value, "stated", &["target", "best"], &[])?;
            let target = grip(field(value, "target", "stated")?, "stated.target")?;
            let pair = array(field(value, "best", "stated")?, "stated.best")?;
            let [p, l] = pair else { return Err(reject("stated.best", "a best value is [terminal Φ, length]")) };
            let length =
                usize::try_from(int(l, "stated.best")?).map_err(|_| reject("stated.best", "a length is >= 0"))?;
            Some((target, (int(p, "stated.best")?, length)))
        }
    };

    let mut j = jam_state(engines, run, &seed, "seed")?;
    let ctx = j.ctx;
    let tables = tree_tables(ctx, "phi")?;
    for (n, &(cap, item)) in word.iter().enumerate() {
        let claim = at("word", n);
        if !RETAINED.contains(&item) {
            return Err(reject(&claim, "a routing letter is a retained item, 1 to 11"));
        }
        if n + 1 == word.len() {
            let s = straddlers(ctx, &mut j.state, r, "released")?;
            if s == 0 {
                let stop = reject("released", format!("grip {r} has no straddler before the word's last letter"));
                return Err(run.failed(stop, "0 straddlers"));
            }
            run.held("released.before", format!("{s} straddlers at grip {r} before the last letter"));
        }
        let twist = j.twist(cap, item, &claim)?;
        if twist.kidx.is_none() {
            return Err(reject(&claim, "the letter is not a retained rotation"));
        }
        let outcome = j.apply(&twist, &claim)?;
        if outcome.status != Status::Admissible {
            return Err(run.failed(reject(&claim, format!("the letter is {}", outcome.status.as_str())), "refused"));
        }
    }
    run.held("word", format!("{} admissible retained letters after the jam", show(&word)));
    let phi0 = phi(ctx, tables, &j.state, "phi")?;
    run.claim("phi", &int(field(doc, "phi", "phi")?, "phi")?, &phi0)?;
    let labels = LabelTable::build(ctx, &j.state, "released")?;
    if !labels.clean(r) {
        let stop = reject("released", format!("grip {r} has a straddler after the word"));
        return Err(run.failed(stop, format!("{} straddlers", labels.straddle[r].len())));
    }
    run.held("released", format!("grip {r} has no straddler after the word"));

    let locality = Locality::build(ctx).map_err(|error| reject("agree", format!("the locality table: {error}")))?;
    let near: Vec<Vec<bool>> = (0..POLES)
        .map(|c| {
            let mut row = vec![false; POLES];
            row[c] = true;
            for &d in locality.partners(c) {
                row[d as usize] = true;
            }
            row
        })
        .collect();
    let mut calc = Calculus::new(&j, tables, &labels, "agree")?;
    let brute = calc.brute(r, phi0)?;
    let brute_letters = calc.letters;
    let tails = calc.tails(r, phi0, &near)?;
    run.held("brute_force", format!("{brute_letters} letters, best at {} targets: {}", brute.len(), show_best(&brute)));
    run.claim("lowers", &boolean(field(doc, "lowers", "lowers")?, "lowers")?, &!brute.is_empty())?;
    if let Some((target, best)) = stated {
        let show_pair = |b: Option<&(i64, usize)>| b.map_or("none".to_owned(), |(p, l)| format!("{target}:[{p},{l}]"));
        run.claim("stated", &show_pair(Some(&best)), &show_pair(tails.get(&target)))?;
    }
    run.held("tails", format!("best at {} targets: {}", tails.len(), show_best(&tails)));
    run.claim("agree", &boolean(field(doc, "agree", "agree")?, "agree")?, &(tails == brute))?;
    Ok(())
}

// ---------------------------------------------------------------------------------------------- the calculus

/// One kept piece: the grips whose caps hold it, the grips it straddles, and its pose (an index of the calculus).
#[derive(Clone, Debug)]
struct Piece {
    inside: Vec<u16>,
    straddle: Vec<u16>,
    pose: u32,
}

/// The label calculus of one jumbling state under retained letters (see the module documentation).
pub struct Calculus<'j, 'e> {
    j: &'j Jumbling<'e>,
    tables: &'e Tables,
    claim: String,
    cls: Vec<i64>,
    pieces: Vec<Piece>,
    words: usize,
    /// Per grip, the kept pieces inside its cap, as a bit set.
    inside: Vec<Vec<u64>>,
    /// The kept pieces of left height at least 4.
    high: Vec<u64>,
    ns: Vec<u32>,
    foreign: Vec<u32>,
    poses: Vec<Mat4>,
    pose_index: HashMap<Mat4, u32>,
    facts: Vec<(i64, i64)>,
    steps: HashMap<(u32, u16), u32>,
    retained: HashMap<(usize, usize), u16>,
    jumbles: Vec<Mat4>,
    frames: Vec<Option<Mat4>>,
    masks: HashMap<(u32, u16), u64>,
    relative: HashMap<Mat4, (u64, i64)>,
    undo: Vec<Vec<(usize, Piece)>>,
    /// Retained letters applied so far.
    pub letters: u64,
}

impl<'j, 'e> Calculus<'j, 'e> {
    /// The calculus at the state of `j`, from its exact label table.
    pub fn new(j: &'j Jumbling<'e>, tables: &'e Tables, labels: &LabelTable, claim: &str) -> Step<Calculus<'j, 'e>> {
        let ctx = j.ctx;
        let cls = (0..POLES)
            .map(|x| tables.cls(ctx, x).map_err(|error| reject(claim, format!("the class of grip {x}: {error}"))))
            .collect::<Step<Vec<i64>>>()?;
        let mut jumbles = Vec::new();
        for (item, (_, m)) in j.menu.items.iter().enumerate() {
            let lattice = ctx.kplus.index_of_matrix(m, &ctx.model).is_some();
            if RETAINED.contains(&item) && !lattice {
                return Err(reject(claim, format!("menu item {item} is not a retained rotation")));
            }
            if !lattice {
                jumbles.push(m.clone());
            }
        }
        if jumbles.is_empty() || jumbles.len() > 64 {
            return Err(reject(claim, format!("the menu has {} jumble items, not 1 to 64", jumbles.len())));
        }
        let mut calc = Calculus {
            j,
            tables,
            claim: claim.to_owned(),
            cls,
            pieces: Vec::new(),
            words: 0,
            inside: Vec::new(),
            high: Vec::new(),
            ns: vec![0; POLES],
            foreign: vec![0; POLES],
            poses: Vec::new(),
            pose_index: HashMap::new(),
            facts: Vec::new(),
            steps: HashMap::new(),
            retained: HashMap::new(),
            jumbles,
            frames: vec![None; POLES],
            masks: HashMap::new(),
            relative: HashMap::new(),
            undo: Vec::new(),
            letters: 0,
        };
        let state = &j.state;
        let mut straddles = vec![false; NP];
        for list in &labels.straddle {
            for &p in list {
                straddles[p as usize] = true;
            }
        }
        let mut of_state: HashMap<u32, u32> = HashMap::new();
        let mut index = vec![u32::MAX; NP];
        for (p, &pid) in state.pose_ids().iter().enumerate() {
            let Pose::M(m) = &state.poses()[pid as usize] else {
                if straddles[p] {
                    return Err(reject(claim, format!("lattice piece {p} straddles a cap")));
                }
                continue;
            };
            let pose = match of_state.get(&pid) {
                Some(&pose) => pose,
                None => {
                    let pose = calc.intern((**m).clone())?;
                    of_state.insert(pid, pose);
                    pose
                }
            };
            if calc.facts[pose as usize].0 >= 2 || straddles[p] {
                index[p] = calc.pieces.len() as u32;
                calc.pieces.push(Piece { inside: Vec::new(), straddle: Vec::new(), pose });
            }
        }
        for (e, list) in labels.inside.iter().enumerate() {
            for &p in list {
                if let Some(piece) = calc.pieces.get_mut(index[p as usize] as usize) {
                    piece.inside.push(e as u16);
                }
            }
        }
        for (e, list) in labels.straddle.iter().enumerate() {
            for &p in list {
                calc.pieces[index[p as usize] as usize].straddle.push(e as u16);
            }
        }
        calc.words = calc.pieces.len().div_ceil(64);
        calc.inside = vec![vec![0u64; calc.words]; POLES];
        calc.high = vec![0u64; calc.words];
        for q in 0..calc.pieces.len() {
            if calc.facts[calc.pieces[q].pose as usize].0 >= 4 {
                calc.high[q / 64] |= 1 << (q % 64);
            }
            calc.place(q, true);
        }
        Ok(calc)
    }

    /// The number of kept pieces.
    pub fn kept(&self) -> usize {
        self.pieces.len()
    }

    fn intern(&mut self, m: Mat4) -> Step<u32> {
        if let Some(&pose) = self.pose_index.get(&m) {
            return Ok(pose);
        }
        let facts = left_facts(self.j.ctx, self.tables, &Pose::M(Arc::new(m.clone())), &self.claim)?;
        let pose = self.poses.len() as u32;
        self.poses.push(m.clone());
        self.pose_index.insert(m, pose);
        self.facts.push(facts);
        Ok(pose)
    }

    /// The pose k·P, with P's left height.
    fn step(&mut self, pose: u32, k: u16) -> Step<u32> {
        if let Some(&next) = self.steps.get(&(pose, k)) {
            return Ok(next);
        }
        let ctx = self.j.ctx;
        let image = matmul(ctx.kplus.matrix(k, &ctx.model), &self.poses[pose as usize]);
        let next = self.intern(image)?;
        if self.facts[next as usize].0 != self.facts[pose as usize].0 {
            return Err(reject(&self.claim, "a retained letter changed a left height; the calculus does not apply"));
        }
        self.steps.insert((pose, k), next);
        Ok(next)
    }

    /// The K⁺ element of the letter [g, item], from the replay's own twist.
    fn retained(&mut self, g: usize, item: usize) -> Step<u16> {
        if let Some(&k) = self.retained.get(&(g, item)) {
            return Ok(k);
        }
        let twist = self.j.twist(g, item, &self.claim)?;
        let k = twist.kidx.ok_or_else(|| reject(&self.claim, format!("the letter {g}:{item} is not retained")))?;
        self.retained.insert((g, item), k);
        Ok(k)
    }

    /// Adds or removes piece q's labels.
    fn place(&mut self, q: usize, add: bool) {
        let Calculus { pieces, inside, foreign, ns, facts, cls, .. } = self;
        let piece = &pieces[q];
        let (h, b) = facts[piece.pose as usize];
        let bit = 1u64 << (q % 64);
        for &d in &piece.inside {
            let d = d as usize;
            if add {
                inside[d][q / 64] |= bit;
            } else {
                inside[d][q / 64] &= !bit;
            }
            if h >= 2 && b != cls[d] {
                if add {
                    foreign[d] += 1;
                } else {
                    foreign[d] -= 1;
                }
            }
        }
        for &d in &piece.straddle {
            if add {
                ns[d as usize] += 1;
            } else {
                ns[d as usize] -= 1;
            }
        }
    }

    fn members(&self, x: usize) -> Vec<usize> {
        let mut out = Vec::new();
        for (w, &word) in self.inside[x].iter().enumerate() {
            let mut bits = word;
            while bits != 0 {
                out.push(w * 64 + bits.trailing_zeros() as usize);
                bits &= bits - 1;
            }
        }
        out
    }

    /// Whether grip x has no straddler.
    pub fn clean(&self, x: usize) -> bool {
        self.ns[x] == 0
    }

    fn clean_grips(&self) -> Vec<usize> {
        (0..POLES).filter(|&x| self.ns[x] == 0).collect()
    }

    /// Applies the retained letter [g, item] at a clean grip.
    pub fn apply(&mut self, g: usize, item: usize) -> Step<()> {
        if self.ns[g] != 0 {
            return Err(reject(&self.claim, format!("a letter at grip {g}, which has a straddler")));
        }
        let k = self.retained(g, item)?;
        let ctx = self.j.ctx;
        let perm = &ctx.kplus.perms[k as usize];
        let mut log = Vec::new();
        for q in self.members(g) {
            let pose = self.step(self.pieces[q].pose, k)?;
            self.place(q, false);
            let old = self.pieces[q].clone();
            self.pieces[q] = Piece {
                inside: old.inside.iter().map(|&d| perm[d as usize]).collect(),
                straddle: old.straddle.iter().map(|&d| perm[d as usize]).collect(),
                pose,
            };
            self.place(q, true);
            log.push((q, old));
        }
        self.undo.push(log);
        self.letters += 1;
        Ok(())
    }

    /// Undoes the last letter.
    pub fn undo(&mut self) {
        if let Some(log) = self.undo.pop() {
            for (q, old) in log.into_iter().rev() {
                self.place(q, false);
                self.pieces[q] = old;
                self.place(q, true);
            }
        }
    }

    /// The jumble items that lower a pose of left height h ≥ 4 by 2 at grip x, as a bit mask in menu order.
    fn mask(&mut self, pose: u32, x: usize) -> Step<u64> {
        if let Some(&mask) = self.masks.get(&(pose, x as u16)) {
            return Ok(mask);
        }
        let ctx = self.j.ctx;
        let frame = self.frames[x]
            .get_or_insert_with(|| transpose(ctx.kplus.matrix(ctx.kplus.frame_idx[x], &ctx.model)))
            .clone();
        let relative = matmul(&frame, &self.poses[pose as usize]);
        let h = self.facts[pose as usize].0;
        let mask = match self.relative.get(&relative) {
            Some(&(mask, hr)) if hr == h => mask,
            Some(_) => {
                return Err(reject(&self.claim, "two poses of one relative pose have different left heights"));
            }
            None => {
                let (hr, _) = left_facts(ctx, self.tables, &Pose::M(Arc::new(relative.clone())), &self.claim)?;
                if hr != h {
                    return Err(reject(
                        &self.claim,
                        "a retained frame changed a left height; the calculus does not apply",
                    ));
                }
                let mut mask = 0u64;
                for (b, lambda) in self.jumbles.iter().enumerate() {
                    let image = matmul(lambda, &relative);
                    let facts = self
                        .tables
                        .facts(ctx, &Pose::M(Arc::new(image)))
                        .map_err(|error| reject(&self.claim, format!("the engine's tree facts: {error}")))?;
                    if facts.dl == h - 2 {
                        mask |= 1 << b;
                    }
                }
                self.relative.insert(relative, (mask, h));
                mask
            }
        };
        self.masks.insert((pose, x as u16), mask);
        Ok(mask)
    }

    /// S_x, F_x and max D_x.
    pub fn metric(&mut self, x: usize) -> Step<(u32, u32, u32)> {
        let mut counts = [0u32; 64];
        for w in 0..self.words {
            let mut bits = self.inside[x][w] & self.high[w];
            while bits != 0 {
                let q = w * 64 + bits.trailing_zeros() as usize;
                bits &= bits - 1;
                let pose = self.pieces[q].pose;
                if self.facts[pose as usize].1 != self.cls[x] {
                    continue;
                }
                let mut mask = self.mask(pose, x)?;
                while mask != 0 {
                    counts[mask.trailing_zeros() as usize] += 1;
                    mask &= mask - 1;
                }
            }
        }
        Ok((self.ns[x], self.foreign[x], counts.iter().copied().max().unwrap_or(0)))
    }

    /// Records a lowering at x after a word of `len` letters: (Φ₀ + 2(F − max D), len + 1) when F < max D.
    fn lower(&mut self, best: &mut Best, x: usize, len: usize, phi0: i64) -> Step<()> {
        let (_, f, d) = self.metric(x)?;
        if f < d {
            let key = (phi0 + 2 * (i64::from(f) - i64::from(d)), len + 1);
            let entry = best.entry(x).or_insert(key);
            *entry = (*entry).min(key);
        }
        Ok(())
    }

    /// Every lowering at a clean grip other than `last`, after a word of `len` letters.
    fn lower_all(&mut self, best: &mut Best, len: usize, last: Option<usize>, phi0: i64) -> Step<()> {
        for x in self.clean_grips() {
            if Some(x) != last {
                self.lower(best, x, len, phi0)?;
            }
        }
        Ok(())
    }

    /// The brute force over the whole tail language at a configuration where r is clean.
    pub fn brute(&mut self, r: usize, phi0: i64) -> Step<Best> {
        let mut best = Best::new();
        self.lower_all(&mut best, 0, None, phi0)?;
        for g1 in self.clean_grips() {
            for i1 in RETAINED {
                self.apply(g1, i1)?;
                self.lower_all(&mut best, 1, Some(g1), phi0)?;
                for g2 in self.clean_grips() {
                    if g2 == g1 || (g1 != r && g2 != r) {
                        continue;
                    }
                    for i2 in RETAINED {
                        self.apply(g2, i2)?;
                        self.lower_all(&mut best, 2, Some(g2), phi0)?;
                        self.undo();
                    }
                }
                self.undo();
            }
        }
        Ok(best)
    }

    /// Every retained letter at the clean grip g, then the lowerings at the clean targets in `allowed` other than g.
    fn last_letter(&mut self, best: &mut Best, g: usize, allowed: &[bool], before: usize, phi0: i64) -> Step<()> {
        for i in RETAINED {
            self.apply(g, i)?;
            for x in self.clean_grips() {
                if allowed[x] && x != g {
                    self.lower(best, x, before + 1, phi0)?;
                }
            }
            self.undo();
        }
        Ok(())
    }

    /// The tails (a) to (e) of Lemma RR at a configuration where r is clean, with `near[c]` the partners of c and c.
    pub fn tails(&mut self, r: usize, phi0: i64, near: &[Vec<bool>]) -> Step<Best> {
        if !self.clean(r) {
            return Err(reject(&self.claim, format!("grip {r} is not released")));
        }
        let mut found = Best::new();
        let clean = self.clean_grips();
        // (a) a lowering right after the routing word.
        for &x in &clean {
            self.lower(&mut found, x, 0, phi0)?;
        }
        // (b) a letter at a clean grip y other than r, then a lowering at a partner of y.
        for &y in &clean {
            if y != r {
                self.last_letter(&mut found, y, &near[y], 0, phi0)?;
            }
        }
        // (c) a letter at r, then a lowering at a partner of r.
        self.last_letter(&mut found, r, &near[r], 0, phi0)?;
        // (d) a letter at r, a letter at a clean y, then a lowering at a partner of y, r a partner of y or of it.
        for m in RETAINED {
            self.apply(r, m)?;
            for y in self.clean_grips() {
                if y == r {
                    continue;
                }
                let allowed: Vec<bool> = if near[y][r] {
                    near[y].clone()
                } else {
                    near[y].iter().zip(&near[r]).map(|(a, b)| *a && *b).collect()
                };
                self.last_letter(&mut found, y, &allowed, 1, phi0)?;
            }
            self.undo();
        }
        // (e) a letter at a clean partner y of r, a letter at r, then a lowering at a partner of r.
        for &y in &clean {
            if y == r || !near[y][r] {
                continue;
            }
            for i in RETAINED {
                self.apply(y, i)?;
                if self.clean(r) {
                    self.last_letter(&mut found, r, &near[r], 1, phi0)?;
                }
                self.undo();
            }
        }
        Ok(found)
    }
}

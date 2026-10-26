# Card 9: the tail completion, from the depth 40 tail stop

Status: **ready** (26 October 2026).

The next card of the jumbling theorem programme after the six jams ([card 8](card-8-six-jams.md)), which the owner
put first after stage 17 on 26 October 2026, with a freeze of the tutorial and the jumbling research when it ends
([`owner-decisions-2026-10-26-tail-then-freeze`](../../../../docs/wiki/decisions/owner-decisions-2026-10-26-tail-then-freeze.md)).
The tail is the fourth phase of the tutorial runner ([tutorial chapter 5](../../tutorial/05-tail.md)): it lands the
pieces still off the lattice one unit per step, each step one literal word that passes the strict′ gate. The depth 40
development run D1 (seed 7101, run 4, acceptance mode) entered the tail with 1,193 pieces off, took 17 strict′ steps
and stopped `stuck` with 158 off, all at height 2: no tail rule has a step for any of its five units, and no word of
length 1 lands the first of them
([`d1-acc-s7101/RESULT.md`](../../restoration/13-tutorial-runner/runs/dev/d1-acc-s7101/RESULT.md)). Its tail entry is
the state from which the stage 12 replay E2 restored every piece, and at its stop every unit passes the exact
whole-cap test of stage 12 with SAFE preparations, a route the runner does not have (facts T4 and T5 below); four
literal whole-cap steps, two of them prepared, put every piece of the stop on the lattice (fact T6). Answer
Questions 1 to 4. Each answer may be a proof, a reduction to finite certificates that a computer can check at the
state, or a precise counterexample or counterexample shape. Partial results are useful. Label every statement proved,
conditional (name the condition) or conjectured. Name every machine check as a certificate with its exact statement,
and do not claim that a certificate holds.

The target: from a tail state, a sequence of strict′ steps that ends with every piece on the lattice, each step a
**literal** word of at most 10⁷ letters played twist by twist. A step known only by a certificate counts as stuck
([owner decision, 20 October 2026](../../../../docs/wiki/decisions/owner-decisions-2026-10-20-tutorial-acceptance.md),
item 3; chapter 5, section 5.10).

## Model

As in [tutorial chapter 5](../../tutorial/05-tail.md), whose terms this card uses: pose table, lattice, off piece,
chamber, hole, cap, grip, item, letter, word, domain, unit and its key, the class cell model (class, hub, content,
label, cell, point, kind, free grip, owner, glued, vacant home point), strict′, the rules `zero_bad`, `pure`, `close`
(V·W·V⁻¹) and `t1prime` (W·V′·W⁻¹·V′⁻¹), Bad(W), the SAFE-ALL letters of a state Z (letters at grips that no piece
straddles and whose caps hold no off piece) and their group G(Z) acting on the 177,120 chambers, N1 and N2, the
literal limit, the fallback and the probe. The number of holes always equals the number of off pieces. From stage 12
([`PROOFS-B.md`](../../restoration/12-low-height/PROOFS-B.md)): the good-ghost map f of a core W, the landing chambers
D, the middle Z·W with its displaced lattice geometry, SAFE preparations X, global Jordan certificates, whole-cap cores
and their three-segment form (COV-12), the catalogue and Cover(Z) (COV-15), and 𝒞₂, the valid states with every off
piece at height 2.

New here:
- A **whole-cap instance** at Z: an unstraddled cap c whose off pieces U_c are of one frame domain, an item i with
  g·γ_p ∈ K⁺ for every p ∈ U_c (g the element of (c, i), γ_p the pose of p; COV-12's eligibility), and a prepared hole
  set J in the orbit G(Z)·H of the hole set H. Its step is X·(c, i)·V′·(c, i)⁻¹·V′⁻¹, with X a SAFE-ALL word that
  carries H to J and V′ a SAFE-ALL word of the middle Z·X·(c, i) that vacates cap c; it lands every off piece in the
  cap, whatever units they belong to (COV-32). It is **unprepared** when J = H.
- Its **demand** is the landing chambers D counted per orbit of the middle SAFE-ALL group; its **resource** is the
  holes of Z per orbit of G(Z) that a flow assigns to that demand (COV-10).

## Given (proved; use as facts)

From chapter 5 and stage 8:
- **Lemma U** (`restoration/08-units-and-clean-steps/PROOFS-U.md`). Every clean step lowers the off count by at least
  the size of its unit; so the tail takes at most as many steps as there were off pieces when it began.

From stage 12 (`PROOFS-B.md`, proved by model review; the labels there):
- **R22A-06** (closure). If V is SAFE-ALL at Z and Bad(W) ⊆ holes(Z·V), then V·W·V⁻¹ is a strict step of
  2|V| + |W| letters.
- **R22C-T1′** (T1′). For a class-P unit U that is the whole content of its cell, a word W of free class-P letters
  that lands U and keeps every other off piece at its pose, and a SAFE-ALL word V′ at Z·W that vacates Bad(W⁻¹):
  W·V′·W⁻¹·V′⁻¹ is strict for U, of 2|W| + 2|V′| letters. **R22C-T2′**: the same with one partner D.
- **Lemma G** (R22C-G). Free class-P letters rotate each cell as a whole; when W lands a content of a cell, every
  lattice piece of that cell goes off, so Bad(W) contains every occupied chamber of the cell; a vertex content lands in
  place only.
- **R22C-T** (locks, Lemma T) and **R22C-S** (shadows). An occupied chamber locked to a block keeps its content under
  every retained, SAFE-ALL or one-class word that returns the block, so no single-turn template lands a unit whose cell
  has one; a chamber shadowed by a block lies in no SAFE-ALL cap while the block keeps its pose, so N1 fails for every
  target that contains it occupied.
- **R22C-M** (partners do not repair N1) and **R22C-V** (type counts; item 4 the necessary T1′ counts).
- **R22C-FC** (Theorem FC). At a reachable state, every nonempty set S of off pieces has a cap that holds a piece of S
  and that no piece of S straddles.
- **Corollary A1** (`theory/buffer-transport.md`). If every orbit that meets a target T outside the fixed chambers
  has a Jordan certificate, a SAFE-ALL word that clears T exists exactly when N1 and N2 hold. **R22B-19**: local
  alternating actions and separating words give a product of alternating groups. **R22B-29**: the vacancy theorem, a
  finite equivalence without alternating hypotheses.
- **R22C3-01.** Strict′ steps keep the SAFE-ALL alphabet (G(Z) ≤ G(Z′), orbits only merge), not the hole
  inventories; the inventory identity h′_Δ = h_Δ + |L ∩ Δ| − |φ(L) ∩ Δ| − |D ∩ Δ|. **R22C3-02**: an order of steps
  exists and is strict′ when, at each stage, the step's local hypotheses and the resource contracts that later stages
  need hold. **R22C3-06**: a uniform bound b = 13,205! − 1 for native free-class cores.
- **R22A2-04 to R22A2-06.** Fresh closures land fourteen units in a checked order; a shared target needs a fresh
  closure; closure bounds and the two failure modes at a stop.
- **COV-06.** At a strict′ boundary the off poses and the hole set decide every later outcome; the successor formulas
  of T1 and T1′.
- **COV-08.** T1′ resources pull back through the good-ghost map: N2 at the middle is |H ∩ f⁻¹(Δ∖T)| ≥ |D ∩ Δ| for
  every middle orbit Δ; a strict′ predecessor changes these counts by the inventory identity.
- **COV-09.** A strict′ predecessor can turn N2 at a whole-cap middle from true to false (the b15 step before b06's
  whole cap in E2); resources are not monotone.
- **COV-10.** A SAFE preparation exists exactly when some J ∈ G(Z)·H gives a middle at which the vacancy test of
  R22B-29 accepts; a flow from the source orbits to the middle orbits, with Jordan certificates on the partially filled
  source orbits and on the middle demand orbits, is sufficient. **COV-11**: the greedy preparation of E2 (prep2) is a
  heuristic, not a test.
- **COV-12** (whole-cap eligibility, vacancy and the three-segment form) and **COV-32** (the exact joint test for a
  whole-cap step with several off pieces; lifting a certified word past separated contents). **COV-38**: separated
  contents restore one at a time while their hole resources survive.
- **COV-15.** Cover(Z), some catalogue instance passes at Z, is a finite predicate; if it holds at every state of 𝒞₂
  with off pieces, each such state is restored in at most n(Z) strict′ steps (conditional).
- **COV-13** (a reachable height-2 state with no eligible cap, restored by its free-class inverse history), **COV-14**
  (an isolated triangle with whole-cap landers and no prepared vacancy), **COV-33** (whole-cap counts at E2's 737
  state: five caps pass, cap 91 only after a preparation; ten eligible caps fail under every preparation).
- **COV-34 and COV-56** (what B′(2) still needs): SingletonResidual (112 deep cases), MultiCover (every reachable
  height-2 state with at least two off pieces has a passing catalogue instance), MixedCoreBound; the bounds.

## The stop (exact)

The D1 stop is `research/jumbling/restoration/13-tutorial-runner/runs/dev/d1-acc-s7101/out4/tables/00113.json`, digest
`966dc8a96f7bf40b4c0361d7a26d9a3efbf62e6c59c1a6677cf5de9f8aefe661`: 158 pieces off, all at height 2, 158 holes. Its
history from the scramble is the run's 114 steps (`out4/words/`; the tail's start is the bulk's end, digest
`7e128820…`, 1,193 off). The units at the stop (`mkblocks2.py` on the table; the runner's order is decreasing size,
then key):

| Unit (key) | Size | E2's name | Class | Cell (class caps) | Kind | Census | Caps that hold it | Caps it straddles |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 9013 | 61 | b05 | 3 | {12, 91} | edge | glued (lattice content of 86) | 12, 35, 56, 67, 91 | 6, 17, 60, 74 |
| 60085 | 33 | b08 | 4 | {439, 548, 560} | triangle | live; `small_bad` core, Bad 61, 96 letters | 439, 474, 501, 542, 548, 551, 560, 580 | 496, 582 |
| 81223 | 29 | b11 | 3 | {148, 217, 324, 353} | tetrahedron | live; `small_bad` core, Bad 80, 60 letters | 148, 203, 217, 224, 252, 286, 324, 353 | 242, 270, 371 |
| 112734 | 25 | b15 | 3 | {217, 324} | edge | glued (lattice content of 86) | 203, 217, 224, 242, 286, 324 | 252, 270, 371 |
| 16314 | 10 | b06 | 3 | {91} | vertex | glued (lattice content of 378) | 35, 56, 67, 91 | 17, 60, 74, 133 |

Each unit holds the same pieces as E2's unit file of its name
(`restoration/12-low-height/runs/round22/line-a/units/eb5-k40-k00-b0N.json` and `…-b1N.json`). Class 3 at the stop:
118 free grips (582 blocked), 1,445 own contents (the four class-3 units are its four off contents), 2,641 cells,
1,441 home cells, 6,962 generators, no vacant home point of any kind. Class 4: 116 free grips (60, 242 and 252
blocked), 1,442 own contents (one off content, 60085), no vacant home point.

## Computed facts (exact; leads, not proofs)

Programs: the runner's own children (`transport_census.py`, `cellpure5.py`, `tclose2.py`, `stepchk.py`,
`prep_t1prime.py`, `fallback.py`), which wrote the attempt logs of the stop (`out4/logs/attempts-00114-*`), and the
integrator's `wholecap.py` (stage 18, `scripts/`), COV-33's computation generalized to any table.

- **T1 (what the runner tried at the stop).** For every unit, in unit order:

  | Unit | `zero_bad` | `pure` | `close`, pure core | `close`, transport core | `t1prime` |
  | --- | --- | --- | --- | --- | --- |
  | 9013 | no word (glued) | lands the unit, 86 lattice pieces off (432 letters) | N1 and N2 false (below) | no core (glued edge cell) | N1 and N2 false at Z1: 31 orbits short by one |
  | 60085 | no word (no vacant home point) | lands it, 61 off (12 letters) | N1 true, N2 false: 5 orbits short by 20 | N1 true, N2 false: the same 5 orbits | N1 true, N2 false at Z1: 4 orbits short by 6 |
  | 81223 | no word | lands it, 80 off (24 letters) | N1 true, N2 false: 4 orbits short by 37 | N1 true, N2 false: the same | N1 true, N2 false at Z1: one orbit short by 2 |
  | 112734 | no word (glued) | lands it, 86 off (432 letters) | N1 and N2 false (below) | no core (glued edge cell) | N1 and N2 false at Z1: 24 orbits short by one |
  | 16314 | no word (glued) | no word (a vertex cell; no power lands it) | no word | no core | no word |

  For 9013 and 112734 the closure's target is the 86 home chambers of the unit's cell, all occupied, in 27 orbits of
  the SAFE-ALL group: 6 of them are one-chamber orbits (so N1 fails), and the seventh short orbit is the orbit of
  7,167 chambers that holds no hole and one target chamber (the integrator's check of the stop with the attempt logs'
  orbit numbering: orbits 134 to 137, 174 and 176 for {12, 91}; 331, 333, 336, 340, 415 and 417 for {217, 324}; orbit
  7 for both).

  The pure words: 9013, the cycle of the edge cells {12, 91}, {8, 12}, {8, 91} (432 letters); 112734, the cycle of
  {217, 324}, {148, 217}, {148, 324} (432 letters); 60085, [(548, 28), (549, 9)]³ (12 letters); 81223,
  [(148, 23), (201, 5)]⁶ (24 letters). The fallback probe for 9013 tried 34,762 words of length 1 and found none
  (1,515 s). No attempt timed out.
- **T2 (the SAFE-ALL group at the stop).** 569 SAFE-ALL grips, 6,259 letters, 935 orbits on the chambers, 900 of them
  of one chamber; the 158 holes lie in 28 orbits.
- **T3 (the runner's order).** From 1,193 off the tail took 17 steps (`run4-steps.md`; units by E2's names, which
  agree with the runner's keys and sizes): `zero_bad` on b02, b07, b16 and b13 (111, 111, 111 and 101 pieces;
  1,193 → 759), `pure` on b17 (86; → 673), `close` on b14 (78; → 595, a certificate V of 55,646 letters, 111,316 in all)
  and b01 (72; → 523), `t1prime` on b03 (63; → 460), `close` on b20 and b09 (60 and 58; → 342), `t1prime` on b04 and b12
  (45 and 41; → 256), `close` on b10 (34; → 222, a certificate V of 114,392 letters, 228,796 in all), `t1prime` on b00,
  b19, b21 and b18 (33, 16, 10 and 5; → 158): 397,280 twists. 9013 was tried and failed at every state from 460 off on.
  60085, 81223 and 112734, further down the size order, were first tried at 189 off and failed there with the outcomes
  of T1 (closure deficits 11 and 30, and N1); 16314 was first tried at 173.
- **T4 (E2 from the same entry).** The tail's entry, `7e128820…` with 1,193 off, is the state from which E2 restored
  every piece in 21 strict′ steps (stage 12 `README.md:882-938`; the literal form of its last 13 steps in chapter 5,
  section 5.17.5). The runner's 22 units, the 17 it landed and the 5 at the stop, are E2's 22 units by key and size.
  E2 landed b00, b01 and b02 first (1,193 → 977), then b08 (60085) at 977 → 944 and b11 (81223) at 944 → 915 by
  closures of 762 and 878 letters, b15 (112734) at 96 → 71 by a T1′ step once its COV-04 blockers b07, b12, b13 and b14
  had landed, and b05 (9013) with b06 (16314) last, at 71 → 0, by the prepared whole-cap step
  X·(91, 14)·V′·(91, 14)⁻¹·V′⁻¹ with X = (178, 7)(170, 1)(75, 1) and V′ of 336,206 letters for the 3,097 chambers of
  cap 91 (672,417 letters in all). From
  737 on E2 switched from closures to T1′ steps, because a closure needs a hole outside Bad(W) for each lattice piece
  that W takes off and a T1′ step one outside its target for each landing piece only (COV-06).
- **T5 (whole caps at the stop).** `wholecap.py` at the stop: of the 600 caps, 569 hold no off piece and no piece
  straddles them, 11 are straddled, 11 hold off pieces of one domain with no landing item (they are caps of other
  classes: 35, 56, 67, 203, 224, 286, 474, 501, 542, 551, 580), and 9 are eligible, the caps of the units' own cells:

  | Cap | Off pieces | Units | Unprepared, at each item | Result |
  | --- | --- | --- | --- | --- |
  | 12 | 61 | 9013 | 32 to 56 occupied landing chambers fixed | no preparation helps |
  | 91 | 71 | 9013, 16314 | none fixed; 19 holes short | passes prepared, item 14 |
  | 148 | 29 | 81223 | none fixed; 2 short | passes prepared, item 15 |
  | 217 | 54 | 81223, 112734 | none fixed; 2 short | passes prepared, item 15 |
  | 324 | 54 | 81223, 112734 | 16 to 19 fixed | no preparation helps |
  | 353 | 29 | 81223 | items 15 and 18: 2 fixed; item 22: none fixed, none short | passes unprepared, item 22 |
  | 439 | 33 | 60085 | 2 to 21 fixed | no preparation helps |
  | 548 | 33 | 60085 | none fixed; 7 short | passes prepared, item 12 |
  | 560 | 33 | 60085 | none fixed; 2 short | passes prepared, item 13 |

  "No preparation helps": the source orbits without a hole keep their chambers occupied under every preparation, and
  the middle SAFE-ALL group built from those regions alone, which contains every actual middle group, already fixes
  occupied landing chambers at every landing item (COV-33's obstruction over all preparations). "Passes prepared": with
  the inside mask empty (every hole moved out of the cap), an integer flow from the 28 source orbits to the middle
  demand orbits supplies every landing chamber (71, 29, 54, 33 and 33), and every partially filled source orbit and
  every middle demand orbit has a global Jordan certificate, each checked again; by COV-10, COV-12, COV-32 and
  Corollary A1 a SAFE-ALL preparation X and a vacating V′ exist together. "Passes unprepared": deficit 0 and global
  Jordan certificates on every middle orbit that meets the landing chambers (6 orbits). `wholecap.py` builds no X and
  no V′ as a word (T6 does). So the three disjoint instances at caps 91, 217 and 548 (or 560) together cover all 158
  off pieces, each passing at the stop by itself.

  The same program reproduces COV-33's record at E2's 737 state (`runs/round22/cov33_whole_cap_737.txt`) row for row:
  the same 15 eligible caps, the same items, the same counts and verdicts, cap 91 prepared with a flow of 179; and at
  E2's state with 71 off (`literal/run2b/z-04-b15.json.gz`, digest `31139216…`) it finds two eligible caps, 12 (no
  preparation helps) and 91 (20 short unprepared; passes prepared at item 14 with a flow of 71), the cap and item of
  E2's last step.
- **T6 (the tail completed from the stop, literal).** Four whole-cap steps put every piece of the stop on the lattice.
  Each was built as one literal word at its state and played twist by twist from the end table of the step before by
  `slp_apply.py --unit`, which passed the strict′ contract for each (the unit on the lattice, no collateral, every
  other off piece at its pose). The integrator chose the order by hand: at each state, a passing unprepared instance
  if `wholecap.py` found one, else the passing prepared instance that moves the fewest holes.

  | Step | Off | Cap, item | Lands | X (holes moved out of the cap) | V′ | Letters | End digest |
  | --- | --- | --- | --- | --- | --- | --- | --- |
  | 1 | 158 → 129 | 353, 22 | 81223 | none | 43,432 | 86,866 | `1007a601…` |
  | 2 | 129 → 104 | 217, 15 | 112734 | none | 224,538 | 449,078 | `5e9f262c…` |
  | 3 | 104 → 71 | 560, 13 | 60085 | 9,748 (23) | 46,740 | 103,230 | `b28448f3…` |
  | 4 | 71 → 0 | 91, 14 | 9013, 16314 | 21,592 (19) | 424,994 | 871,582 | `c75c8dde…` |

  1,510,756 letters in all. The builders are stage 18's `wc_prep.py` and `wc_t1.py`. `wc_prep.py` chooses the prepared
  hole set J by the flow of T5 (outside the cap, the same number of holes in every source orbit, current holes kept
  where the flow allows: 81 of 104 and 52 of 71), builds X as an R22B-10 certificate for J at Z (`r22b10_cert.py
  --seeds atlas`, checked by `check_r22b10.py`, expanded by `expand_slp.py`) and plays it; after X the holes are
  exactly J and every off piece keeps its pose. `wc_t1.py` is E2's builder `e2_b06_t1.py` with the cap and the unit as
  arguments: W = (c, i) lands the cap's off pieces at Z·X; at the middle Z1 = Z·X·W, Bad(W⁻¹) is the cap's 3,097
  chambers with N1 and N2 true; `tclose2.py`'s peeling and 60-second greedy searches for a literal V′ found none (the
  peeling stopped at 0 of 1,981 to 2,382 target chambers); so V′ is an R22B-10 certificate built and checked at Z1 and
  expanded. Each step took 13 to 38 minutes of one core, 7 to 14 of them in the literal searches that found nothing.
  `wholecap.py` before each step: at 129 off, cap 217 holds 112734 alone and passes unprepared at item 15 (at the stop
  it held 81223 too and needed a preparation); 548 and 560 need 15 and 7 holes moved (7 and 2 at the stop), and 91
  needs 19 (as at the stop); at 104 off, 548 and 560 need 8 and 3, 91 still 19; caps 12, 324 and 439 stay "no
  preparation helps" throughout. E2's last step, at the same cap and item from E2's 71 state, had X = (178, 7)(170, 1)
  (75, 1) and 672,417 letters (T4). The words and a replay that checks every step, digest, count and word hash from
  the stop are in stage 18 (`results/d1-tail/`, `scripts/replay_tail.py` with its gate `test_replay_tail.py`; both
  pass). The end state `c75c8dde…` has every piece on the lattice; from it the runner in development mode
  (`runner.py --table`, restorer `schedule11`, chapter 6) ended `solved` after 53 lattice steps and 13,231,266 twists,
  and `check_run.py` passes that record. So D1's scramble is restored to identity labels by the runner's 114 steps,
  T6's four steps and the lattice phase. This is not a runner run: T6's steps are not runner rules, and their order is
  the integrator's.

## Questions

### 1. Completing the tail from the stop

1. T6 builds X and V′ for four instances from certificates (three-cycles conjugated by table paths) and finds no
   literal V′ by tclose2's searches. Give a construction of X and V′ for a passing instance of T5 with a length bound
   computed from the certificate data (orbit sizes, the Jordan primes, the three-cycles and their conjugators, the
   preparation's moves), and say whether the whole step stays within the literal limit of 10⁷ letters; compare it with
   T6's lengths. COV-10's bound on a preparation, ∏ binom(|A_i|, h_i) − 1, is far too large; give one in the size of
   the orbits and the number of holes moved, or a counterexample shape where no short preparation exists.
2. The three disjoint instances at caps 91, 217 and 548 (or 560) each pass at the stop, and T6's order (353, 217,
   560, 91) completes it, where the first step turns 217 from a prepared instance for two units into an unprepared one
   for one. Give an order and a resource contract (COV-08, R22C3-02) under which each later instance still passes after
   the earlier steps, as a finite test at the stop that does not need the earlier words; or show that a first step can
   destroy a later instance's resources (as COV-09 does in E2; for example 91 or 560 first), and how a preparation or
   another first step repairs it. Is there a joint step that lands two of them at once (caps of different classes;
   COV-32)?
3. Which other catalogue instances (COV-15) pass at the stop: a T1′ or a closure with a preparation for the units' own
   pure words, a partner (R22C-T2′), or 112734's own T1′ after 81223 has landed? Say what each costs in holes.

### 2. Shared cells, locks and fixed chambers

At the stop three units are glued (Lemma G). For 9013 and 112734 the runner's closure fails N1 (six occupied
one-chamber orbits in the cell's 86 chambers, T1) and its T1′ middle fails N1 and N2 (31 and 24 orbits short), while
the whole caps 91 and 217 have no fixed landing chamber (T5).

1. Explain the fixed chambers: which caps make a chamber of the target lie in a one-chamber SAFE-ALL orbit (off
   pieces inside, straddlers; R22C-S shadows, R22C-T locks), and why the whole-cap middle avoids them where the
   cell-cycle middle does not. Give a finite criterion at a state, in terms of the cells, the off pieces in the caps and
   the holes, that decides whether a glued unit has a catalogue step, with a construction when it holds.
2. Relate this to the vacancy question of [card 4](card-4-class-puzzle-and-vacancies.md), Part II (how to empty a cell
   that an off content shares with lattice pieces): is every glued unit at a state of 𝒞₂ landed by turning a cap of its
   cell as a whole (T1′ with the cap as core, preparations allowed), and when does that fail (COV-14)?

### 3. An order that does not stick

From the same entry the runner's order stops at 158 and E2's does not (T3, T4): the runner lands the largest units
first and reaches 60085 and 81223, whose closures need many holes in particular orbits, only at 189 off, and 112734
while 81223, which shares and straddles its caps, is still off.

1. Give a state-only order rule for the tail, with a resource contract, under which the tail does not stick on a
   stated class of states that contains D1's entry `7e128820…`, with a proof; for example: land first the units whose
   cheapest passing instance needs the most holes, or whose landing frees the SAFE-ALL grips of others; keep a flow
   condition for all remaining units as an invariant (COV-10 and COV-38); or T6's rule (an unprepared whole-cap
   instance first, else the prepared one that moves the fewest holes), which completed the stop but has not been run
   from the entry. Or show a state of 𝒞₂ where every order of a stated catalogue sticks, with a finite negative
   certificate.
2. State the rule so that a program that reads only the current state can follow it, as chapter 5 states its rules:
   what it reads, its candidates in their exact order, the test of a candidate, the word and its tree, and the
   contract; include the whole-cap instance with its preparation as a rule.
3. Is Cover(Z) (COV-15) at every state of the tail of a run whose entry has Cover enough, or must the order keep
   Cover? Give the condition under which COV-15's conditional theorem applies along a run.

### 4. Literal lengths

1. Bounds for the literal length of each kind of step (closure, T1′, prepared T1′, whole cap) in terms of finite data
   at the state, and whether the steps of Questions 1 and 3 stay within 10⁷ letters at D1 and along E2's trajectory
   (E2's longest step was 672,417 letters; the runner's longest at D1 was 228,796; T6's longest is 871,582).
2. The certificate builder (`r22b10_cert.py`, three-cycles conjugated by table paths, `--seeds atlas`) depends on a
   pseudo-random seed and on clocks (chapter 5, section 5.18). Give a deterministic construction of V and V′ with a
   bound that does not depend on either.

## Return

A proof blueprint with numbered lemmas for each question attempted. State every certificate exactly (what is computed,
on which objects, and what it must show). Give every step you find as an exact word, or as an exact construction from
named certificates, from the stop (with its digest), with the off count and end digest you computed, and every
counterexample as a pose table or an exact word from solved. Keep proved, conditional and conjectured statements
apart. For each construction, name an adjudicating experiment at the D1 stop, at D1's entry or at a state of E2 (the
input table, the procedure or the word, and the outcome that confirms or refutes it), and say which units of the stop
it is expected to land.

## Attached evidence

- [`../../restoration/13-tutorial-runner/runs/dev/d1-acc-s7101/`](../../restoration/13-tutorial-runner/runs/dev/d1-acc-s7101/):
  `RESULT.md`, `run4-steps.md` and `out4/` (`run.json` with the stuck record and every attempt, `tables/` with the
  stop `00113.json` and the earlier tail states, `units/`, `words/`, `step-trees/` and `logs/`, the attempt logs of
  the stop among them), and `check.out`, whose checker passes every one of the 114 steps.
- [`../../restoration/12-low-height/`](../../restoration/12-low-height/): `PROOFS-B.md` (every result listed under
  Given, with proofs), `README.md` (section 2, E2; lines 905-949, the 21 steps), `scripts/round22/`
  (`cov33_whole_cap_737.py`, `e2_b06_prep1.py`, `e2_b06_prep2.py`, `transport_census.py`, `stepchk.py`,
  `prep_t1prime.py`, `jordan_target.py`) and `scripts/` (`cellpure5.py`, `r22b10_cert.py`, `check_r22b10.py`,
  `expand_slp.py`, `slp_apply.py`), and `runs/round22/line-a/` (E2's tables, unit files, words and logs).
- [`../../restoration/18-tail-completion/`](../../restoration/18-tail-completion/): `scripts/wholecap.py` (the
  whole-cap test of T5; writes nothing, one line per result), `scripts/wc_prep.py`, `scripts/wc_t1.py`,
  `scripts/replay_tail.py` and its gate `scripts/test_replay_tail.py` (T6), `results/stop-units/` (the five units of
  the stop), `results/wholecap/` (the whole-cap outputs of T5 and T6), `results/d1-tail/` (the four steps of T6: X, W,
  V′ and the unit of each, with `steps.json`) and `results/lattice-d1/` (the lattice phase from T6's end, with
  `check_run.py`'s output).
- [`../../tutorial/05-tail.md`](../../tutorial/05-tail.md): the tail's rules as the runner implements them, with
  `path:line` citations.
- The scripts run on the exact jumbling simulator of the research tree (`research/jumbling`) with
  `python3 -I -B <script> ...` from the repository root. They are exact computations in synthetic geometry, not
  native-host evidence.

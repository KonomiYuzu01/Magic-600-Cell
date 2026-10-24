# Problem cards for an external prover

Self-contained problem statements for a natural-language proof assistant (the owner runs it locally). Each card restates the model it needs, lists what is proved and what is only computed, and asks for a proof blueprint that names every machine check as a certificate. A returned blueprint is a lead until the integrator checks it and, where it names certificates, until exact code confirms them.

| Card | Open problem | Question | Status |
| --- | --- | --- | --- |
| [card-1-retained-group.md](card-1-retained-group.md) | L1 | Finite certificates that would make the retained group equal to the kernel of its Lemma 8 invariants. | ready |
| [card-2-jumbled-lattice-invariants.md](card-2-jumbled-lattice-invariants.md) | L2 | Do lattice states reached through jumbling keep the Lemma 8 invariants? | ready |
| [card-3-hidden-puzzles.md](card-3-hidden-puzzles.md) | R1 | Each of the five inscribed 600-cells of the grips carries an ordinary puzzle that never blocks itself; when can a stop be restored one class at a time? | ready |
| [card-4-class-puzzle-and-vacancies.md](card-4-class-puzzle-and-vacancies.md) | R1 | The structure E3 used to return two fragments of one class (stars, label orbits, counting, edge holonomy), and how to empty a cell that a fragment shares with lattice pieces (moving vacant chambers). | ready |
| [card-5-depth-and-lattice-group.md](card-5-depth-and-lattice-group.md) | R1, L2 | For the menu I_a: is the depth of poses on the left tree bounded, and which lattice correction group A_J do jumbled histories reach? Evidence in [card-5-evidence/](card-5-evidence/). | answered in part (below) |
| [card-6-blocking-structure.md](card-6-blocking-structure.md) | M1, R1 | Why the grips a height-lowering needs are blocked at deep states, and a tame unlocking: a construction from the state that frees such a grip without raising any left height. Evidence in [restoration stage 14](../../restoration/14-blocking-structure/README.md). | answered in part (below) |
| [card-7-lowering-machine.md](card-7-lowering-machine.md) | M1, R1 | Where the precedence peel stops (straddlers without a clean holder, F ≥ D, overlapping helpers): multi-level unlocking, the lowering machine (the X1 pump reversed, relays, the choice of lowering item), placement, and an order that does not jam, with bounded excursions allowed. Evidence in [restoration stage 14](../../restoration/14-blocking-structure/README.md). | ready |

The lattice-prefix rule ([open-problems](../open-problems.md), "Results of 11 October 2026") was checked separately and has no card.

## Answers so far (24 October 2026)

- **Card 1.** Accepted theorem: the certificates and F1 imply G = 𝒦. All 35 position and 11 frame certificates were then constructed and pass an independent checker, so the retained group equals the kernel of its invariants, without hypothesis C ([restoration stage 03](../../restoration/03-retained-group-and-lattice-restorer/README.md)).
- **Card 2.** Answered by a counterexample: the 9-twist journal C2 reaches a lattice state that breaks the invariants. Card 5 starts from it.
- **Card 3.** Theorems A, B and D1 were accepted; card 5 uses them as given.
- **Card 5.** Five of its seven sub-questions are answered, among them A_J = C2^34 × C5^2 exactly; the normal form from height 6 (1.3) stays open ([restoration stage 04](../../restoration/04-card5-lattice-invariants/README.md)). Finiteness (1.1) was answered on 17 October 2026: R(I_a) is infinite ([restoration stage 10](../../restoration/10-x1-unbounded/README.md)).
- **Card 6** (24 October 2026). Joint attack by Astra and Claude Opus, under the owner's decision of that day. Question 1.1 is answered by a proved construction with finite hypotheses at the state and a length bound (the precedence peel T3P, at most 81 letters per round with clean helpers), which gives a tame descent at the D2 height end of seed 7304; Question 1.2 by an exact safety table and its symmetry; the mechanism of Question 3.2 by a per-cap accounting (each lowering turns the pieces it lowers into the cap's new foreign load, and after a best lowering, one with the largest supply, the cap descends again only after letters at other grips lower that load or raise its lowering supply). Questions 1.3, 2 and 3.1 are answered in part, and 3.3 is open: chained greedily, the construction's rounds lower Φ by 6% to 56% at the six D2 height ends and then jam ([restoration stage 14, section 5](../../restoration/14-blocking-structure/README.md#5-joint-attack-24-october-2026)).

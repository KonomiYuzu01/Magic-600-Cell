# Problem cards for an external prover

Self-contained problem statements for a natural-language proof assistant (the owner runs it locally). Each card restates the model it needs, lists what is proved and what is only computed, and asks for a proof blueprint that names every machine check as a certificate. A returned blueprint is a lead until the integrator checks it and, where it names certificates, until exact code confirms them.

| Card | Open problem | Question | Status |
| --- | --- | --- | --- |
| [card-1-retained-group.md](card-1-retained-group.md) | L1 | Finite certificates that would make the retained group equal to the kernel of its Lemma 8 invariants. | ready |
| [card-2-jumbled-lattice-invariants.md](card-2-jumbled-lattice-invariants.md) | L2 | Do lattice states reached through jumbling keep the Lemma 8 invariants? | ready |
| [card-3-hidden-puzzles.md](card-3-hidden-puzzles.md) | R1 | Each of the five inscribed 600-cells of the grips carries an ordinary puzzle that never blocks itself; when can a stop be restored one class at a time? | ready |
| [card-4-class-puzzle-and-vacancies.md](card-4-class-puzzle-and-vacancies.md) | R1 | The structure E3 used to return two fragments of one class (stars, label orbits, counting, edge holonomy), and how to empty a cell that a fragment shares with lattice pieces (moving vacant chambers). | ready |
| [card-5-depth-and-lattice-group.md](card-5-depth-and-lattice-group.md) | R1, L2 | For the menu I_a: is the depth of poses on the left tree bounded, and which lattice correction group A_J do jumbled histories reach? Evidence in [card-5-evidence/](card-5-evidence/). | answered in part (below) |

The lattice-prefix rule ([open-problems](../open-problems.md), "Results of 11 October 2026") was checked separately and has no card.

## Answers so far (15 October 2026)

- **Card 1.** Accepted theorem: the certificates and F1 imply G = 𝒦. All 35 position and 11 frame certificates were then constructed and pass an independent checker, so the retained group equals the kernel of its invariants, without hypothesis C ([restoration stage 03](../../restoration/03-retained-group-and-lattice-restorer/README.md)).
- **Card 2.** Answered by a counterexample: the 9-twist journal C2 reaches a lattice state that breaks the invariants. Card 5 starts from it.
- **Card 3.** Theorems A, B and D1 were accepted; card 5 uses them as given.
- **Card 5.** Five of its seven sub-questions are answered, among them A_J = C2^34 × C5^2 exactly; the normal form from height 6 (1.3) stays open ([restoration stage 04](../../restoration/04-card5-lattice-invariants/README.md)). Finiteness (1.1) was answered on 17 October 2026: R(I_a) is infinite ([restoration stage 10](../../restoration/10-x1-unbounded/README.md)).

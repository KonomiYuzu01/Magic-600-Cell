# ADR-L05: the evidence model and certificates

Status: **Proposed.** Not accepted. The owner accepts or rejects it ([`AGENTS.md`](../../../../../AGENTS.md), "Ask the
owner"). Its Decision is itself a proposal under LQ3, so accepting this record and answering LQ3 are one step.

Rests on: partition INV-5 and section 8.1; [chapter 06](../architecture/06-evidence.md); [chapter
01](../architecture/01-kernel.md) KN6; [the label table](../evidence-import/README.md). Round table: DT-L1 M1, M2,
R2-E1, R3-E1, R3-E3, R3-M1, R4-M38, R4-M39, note N2; explainers P-M6, P-M9 and P-M10.

## Context

The owner named five grades for a result: proved, conditional, finite instance, not found within budget, incomplete. The
research records carry many more labels, and they do not line up with those words. Across the 497 records of stages 14
to 19 there are 387 (format, field, value) triples, and seven exact words have more than one mapping (EVD.49, EVD.50).
Stage 15's `stopped` is a negative for four families and not for a fifth (EVD.51). A beam search can complete and still
miss 228 descending words (EVD.5).

A tool will show these results to a solver and build on them. So the language needs one model that says what a result
states, how results combine, and what can never be inferred.

## Decision

Under LQ3's recommendation:

- **ADR-L05.1** A claim keeps four questions apart: claim, scope, basis and check. How the job ended is a separate fact,
  and it is not evidence (EVD.1 to EVD.3, EVD.5).
- **ADR-L05.2** A key is a statement ID with its scope. A statement ID is injective on meanings, every scope field is
  immutable, and the context of a check stays apart from scope (EVD.7, EVD.8, EVD.10, EVD.12). A stored record holds
  section 8.1's fields and the nine facts of EVD.13. The only other inputs of a word are four versioned tables
  (EVD.14).
- **ADR-L05.3** Evidence is an AND/OR graph over keys. Arguments are immutable, and a premise is discharged only by a
  claim with the identical key (EVD.15 to EVD.17).
- **ADR-L05.4** Grades are ordered by a product order under one stated principle: x ≤ y only when a trust policy that
  accepts x and rejects y would be incoherent (EVD.19, EVD.20). One argument is governed by its weakest leaves (Theorem
  A). A statement has one canonical summary (Theorem C). Independent arguments keep every undominated argument, and
  nothing adds up (EVD.27). Independence is a displayed fact (EVD.28).
- **ADR-L05.5** Standing is computed in two passes against a versioned threshold, so there is no circularity through
  negation (EVD.29, EVD.30). A failed check removes its leaf and refutes nothing (EVD.31).
- **ADR-L05.6** The owner's words are computed, never stored as truth. A decision list of eight rows makes the function
  total and single-valued (EVD.32). Theorem D states locality, non-upgrade and scoped negatives. "Empty", "not found
  within budget" and "not run" stay distinct (EVD.33). A word is shown with its scope (EVD.36).
- **ADR-L05.7** Refutation is asymmetric. A negative is proved only for its declared set, and sampling never upgrades a
  universal claim (EVD.42 to EVD.44). A finite computation does not stand for a wider statement: a leaf narrower than its
  statement reads as an inconsistency (I7 of EVD.40, EVD.59).
- **ADR-L05.8** A certificate is a stored object of a named kind with a digest. One small kernel checker exists per
  kind. It recomputes without caches, it shares no search or construction code with the producer, and it uses the
  kernel's exact engines and model files, which the oracles test separately. Its report enters the graph as one leaf
  (EVD.46, KN6.1 to KN6.6).
- **ADR-L05.9** Labels on record map by (format, field, value), never by word. A value that depends on a sibling field
  stays recorded until a whole-record rule maps it, and a recorded claim reads open (EVD.48 to EVD.52).
- **ADR-L05.10** Evidence never grants anything, and no policy row reads a word (INV-5, EVD.56, BND.81). The model is
  pure library code over data, so a bug in it can give a wrong word and cannot change a session (EVD.54).

## What this record does not decide

Open whatever the owner answers:

- The content of the invalid-pairs table, which starts empty (EVD.21).
- Records with no `format` key (EVD.53, PS3.12). Until a discriminator table exists they stay wholly recorded.

Conditional on an owner question:

- **LQ3:** the whole Decision above. The alternatives are one claim axis with the five words plus refuted and open, or
  one linear scale. LQ3 also decides whether to raise the threshold above 𝒰₀ (EVD.29).
- **LQ10:** whether the envelope stores facts and derives judgements, or stores judgements (EVD.55, PS1.36).
- **LQ15:** whether formal proofs enter 1.0. The check value exists either way (EVD.3, RTE.28).

## Why

- One axis cannot hold the cases on record. A certificate passed by a checker and a proof read by a model fail in
  different ways, and a scale has to rank them anyway. A wrong comparability hides one leaf behind another, while a
  missing one only shows a leaf more (EVD.20).
- The proofs of chapter 06 hold for any finite poset (EVD.14). So the owner can change the order or the threshold as an
  edition, and no proof breaks and no stored fact is rewritten.
- Computing the words means a later edition re-reads old facts. Stored judgements would have to be migrated, and each
  migration would be a place to overstate a claim.
- Keeping job outcome out of the claim blocks the commonest overstatement: reading "the job finished" as "the search was
  complete". An outcome can still withhold a recorded leaf, and it supplies none (Theorem D (b1)).

## Alternatives considered

| Alternative | Why it was turned down |
| --- | --- |
| One claim axis, five words plus refuted and open (LQ3) | Simple to show. It merges basis and check, so it cannot say why two "proved" results differ. It stays open as LQ3's alternative |
| One linear scale (LQ3) | Forces an order on incomparable grades. It stays open as LQ3's alternative |
| Combine arguments by the strongest | Undefined on incomparable arguments (EVD.27) |
| Count independent samples towards a proof | No number of samples entails a universal statement (EVD.27, EVD.44) |
| Map labels by their word | Seven words have more than one mapping (EVD.50) |

## Consequences

- There is more to store and more to learn. The word is the short form, and the graph is what a reader checks (EVD.39).
- The exact alternative form can be exponential, so it is computed lazily for display (EVD.30).
- An equivalent statement under another ID is refused until a bridge lemma exists (EVD.17). That is the safe direction,
  and it costs extra lemmas.
- Much of the record stays recorded for now: 89 of the 387 triples have a reason and no mapping (EVD.49). They read
  open.
- The envelope's size is unknown until the experiment of PS1.38 runs.

## Evidence

Classes: source, and proofs in the text. Theorems A to D are proved in chapter 06 from the mathematician's note N2. No
person has reviewed them. The label counts are source evidence counted from Git blobs, and `corpus-labels.json` and its
README give the production command. CT-EVD-01 to CT-EVD-23 are requirements only. No kernel checker in this design has
been built, though the research has its own checkers on record.

## What the owner accepts

Accepting this record, with LQ3's recommendation, accepts ADR-L05.1 to ADR-L05.10 and the clauses they cite.

The owner may overrule these integrator additions: the empty start of the invalid-pairs table (EVD.21), the default for
records with no `format` key (EVD.53), the import values "not run", "not stated" and "stated: not replayed" (EVD.47),
the covered-set rule I7 with EVD.59, and the guards of EVD.36.

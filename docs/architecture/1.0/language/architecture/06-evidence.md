# Chapter 06: evidence and certificates

Status: **draft for the owner. Not accepted.** Conventions are in [the overview](00-overview.md), section 3.

This chapter says what a result claims, on what grounds, about what, and which word the display may use for it. It holds
the partial order on evidence with its proofs, the function that computes every display word, the rules for premises and
refutation, and the mapping from the labels already on record. Evidence never gives permission and never gives a score
(INV-5).

The whole model is the recommended option of LQ3. Under LQ3's alternatives this chapter changes, and section 15 says how.
What the stored envelope holds, facts or judgements, is LQ10, and chapter 05 carries both forms.

Sources: [the partition](../ecosystem-partition.md), section 8.1 and defaults 5, 6 and 20;
[DT-L1](../../../../progress/1.0/design-table/topics/language-ecosystem.md) (M1, M2, R2-E1, R3-E1 to R3-E3, R3-M1, note
N2, R4-M38, R4-M39); [the charter](../../../../progress/1.0/charter-2.0-draft.md), bar M3;
[the label table](../evidence-import/README.md); chapters 01, 05 and 13.

Evidence: the order and the theorems below are the mathematician's note N2 in DT-L1 round 3, and I7 with Theorem D (b2)
are its round 4 addition. Theorem D (b1) replaces a part of the note's (b) that the Astra review refuted (DT-L1). By
this chapter's own grades that note is (theorem, producer) until a model review reads it, and no person has reviewed it.
The figures for the corpus are source evidence counted from Git blobs. The worked examples cite research records. No
conformance test exists yet.

## 1. What a result states

- **EVD.1 [P LQ3]** A claim keeps apart four questions about a result, and records how the job that produced it ended as
  a separate fact.

| Field | Question | Values |
| --- | --- | --- |
| Claim | How true | proved; conditional (named premises); refuted; open |
| Scope | About what | Section 2 |
| Basis | On what | theorem; certificate; exhaustive computation; exact computation; sample |
| Check | Who confirmed | none; producer; independent checker; model review; human review; formal proof |
| Job outcome (KN7, not evidence) | How the job ended | completed; cancelled; cut off; error |

- **EVD.2 [P LQ3]** The basis values mean:

| Basis | Meaning |
| --- | --- |
| Theorem | A cited proof, qualified by stage and pinned to a source commit |
| Certificate | A certificate of a named kind, by digest |
| Exhaustive computation | A completed search over a declared set |
| Exact computation | Exact computation on finite inputs; the research's "measured" |
| Sample | A finite sample that does not entail the statement |

- **EVD.3 [P LQ3]** The check values mean:

| Check | Meaning |
| --- | --- |
| None | Nothing confirmed it |
| Producer | The program or person that made it confirmed it |
| Independent checker | A KN6 checker: it recomputes every claim letter by letter without caches and shares no search or construction code with the producer (KN6.2) |
| Model review | A recorded model review |
| Human review | A recorded review by a person |
| Formal proof | A proof checked by a proof assistant (LQ15) |

- **EVD.4 [D]** Each check records its mode: reused through exact transport, fresh exact audit, or actual replay. EVD.47
  adds one import value for a stated absence. Modes are facts about what the check covered. They are not a strength
  order, and no word reads them. A reused leaf is written as AND(source leaf, transport lemma leaf), so reuse is visible
  in the graph. A fresh audit and an actual replay differ in what they cover, so they are different statements or a
  recorded fact. The grades therefore number at most 30, not 90.
- **EVD.5 [D]** Job outcome is not evidence (KN7.3). A job can complete and leave a search incomplete, because a beam can
  run to its depth and miss 228 descending words (stage 15). So completeness of a search is its own fact (EVD.13). An
  outcome can withhold a leaf (EVD.41), and it supplies none (Theorem D (b1)).
- **EVD.6 [D]** Provenance travels with the evidence as data: author, reproducer, scope setter and review class, as stage
  18 records them per theorem. Absence stays absent (PS1.35). The `AGENTS.md` evidence kinds for engineering claims
  (source or fixture, synthetic, actual Windows/DirectX, performance) are a separate field, which the display never folds
  into the words of this chapter.

## 2. Statements and scope

  ```text
  Statement = {
    id:        StatementId,
    form:      finite | universal | existence | mixed(outer: forall | exists),
    negation:  StatementId?,
    subject:   Level,
    premises:  List<PremiseRef>,
    guards:    List<{ class, name }>,
  }

  StatementId = stage-qualified, commit-pinned name | digest of a typed statement

  Scope = {
    covers:    one_instance | search_set(family, alphabet, bounds)
             | admitted_domain(name, parameters) | every_reachable(puzzle),
    identity:  IdentityBlock,
    contract:  contract revision,
    input:     Digest<level>?,
    sources:   source and module digests,
  }
  ```

- **EVD.7 [D]** A *key* is a statement ID with its scope. Everything that discharges, refutes or transports works on keys.
- **EVD.8 [D]** A statement ID must be injective on meanings. It binds every parameter, every guard, the puzzle and the
  level. An ID that omitted k would let a proof for D[7] discharge a premise about D[9]. Two statements that mean the
  same under different IDs stay different until a bridge lemma with its own ID links them. That is the safe direction.
- **EVD.9 [D]** Statement form decides what can refute a claim (section 6), and a statement names its negation when it
  has one. The pair is mutual. One record can settle both sides: the word `535:1,363:5,535:1,597:14` gives "finite
  instance" for "a descent exists here" and "refuted" for "no descent exists here" (stage 15). A statement whose
  `covers` is not one_instance is never of form finite. A mixed statement names its outer quantifier, and its negation
  has the other one.
- **EVD.10 [D]** Every scope field is immutable, and no scope field is revision-bound context. Scope does not require an
  input digest. A theorem and a bound on D[k] have none. This replaces the earlier sentence that a stored scope "is an
  immutable input digest", which would have excluded every theorem (R3-M1).
- **EVD.11 [D]** A digest in a scope names its level, and that level must be at least as fine as the statement's subject
  level. A coarser digest is a typed inconsistency (EVD.40, I3). The counterexample: "Z is reachable in J(A4₀)" with a
  label digest. Solved, and solved with centre 0 turned, share their labels, and the second is unreachable. Off the lattice
  pose is the only stored level until a sticker-class digest exists (KN1.8).
- **EVD.12 [D]** The context of a check is apart from scope. A revision, a protection set, a goal and a role serve claims
  in memory and on the display, and are never stored as evidence (PS1.35, PS3.7). A result in memory names the revision
  it read (KN0.6).

  ```text
  SearchRecord = {
    target:    the existence statement, shared by the pair {E, neg E},
    declared:  family, alphabet, bounds,
    complete:  yes | no,
    stop:      closed | deterministic_budget | declared_limit | other(reason),
    job:       JobOutcome | not_run | not_stated,
  }

  ScopeMembership = { key, member: the checked fact that an instance lies in the scope, check: Grade }
  Premise = { key, class: reachability_promise | open_hypothesis | model_assumption }
  Leaf    = { grade, identity, covered: Covered? }
  Covered = { covers: a value of Scope.covers, input: Digest<level>? }
  ```

- **EVD.13 [D]** The facts a stored record holds, in addition to section 8.1's fields, are nine: the seven R3-M1 named
  and two that round 4 added. Each of them was shown to be needed by a case the earlier fields got wrong.

| Fact | Why it is needed |
| --- | --- |
| Statement form | Refutation depends on it (section 6) |
| Negation pair | One record settles two statements (EVD.9) |
| Subject level | The scope rule (EVD.11) |
| Scope-membership evidence | A failure at a state of unknown reachability says nothing about reachable states (stage 19) |
| Completeness or closure of a search | A completed job can leave a search incomplete (EVD.5) |
| Premise class | A reachability promise is not an open hypothesis and not a model assumption |
| Tool digest of each check, and the declared common base | "Independent" reads them (EVD.28); `macro.py` holds both `cert` and `check --replay` |
| Covered set of a leaf | A finite computation can otherwise stand for a wider statement (I7) |
| Guards of a statement | "Proved on D[k]" would otherwise hide a reachability promise (EVD.36) |

- **EVD.59 [D]** `covered` is computed from the leaf's own record, never copied from the statement. Identical means
  equal field by field, and digests at different levels are never identical (KN1.8).

| Basis | `covered` |
| --- | --- |
| Exhaustive computation | `SearchRecord.declared` as search_set(family, alphabet, bounds), with the job's input digest |
| Exact computation | The job's input: one_instance with its digest, or search_set for a declared list |
| Certificate | The certificate's stated inputs (KN6.6, EVD.46); each checker kind states them |
| Theorem, sample | Not read |

  No row of the label table sets admitted domain, every reachable state or sample, so I7 (b) has no trigger on a
  labelled row today. I7 (a) cannot be tested on the table, which has no `form` and no statement's `covers`. A
  whole-record rule may add to this.

- **EVD.14 [D]** The only inputs of a word that are not facts are four versioned tables: the order on grades, the invalid
  pairs, the threshold and the import table. So a word is a function of the facts and an edition. The proofs of section 4
  hold for any finite poset, and changing a table is an edition that breaks no proof.

## 3. The evidence graph

- **EVD.15 [D]** Evidence is an AND/OR graph over keys. A *leaf* is one piece of support with its grade. An AND node
  needs all its children, which are the parts of one argument. An OR node has one child per independent argument. Each key
  has one OR node. A *premise* is a node that holds when it is assumed or when a claim with the identical key holds.
- **EVD.16 [D]** Arguments are immutable. A new record enters as a new node or a new OR child, and it changes nothing
  that was stored.
- **EVD.17 [D]** A premise is discharged only by a claim with the identical key. Identity is by statement ID. An
  equivalent statement under another ID is refused with `EVD:evidence.premise_unmet` until a bridge lemma exists. A
  universal reaches an instance only through an AND node with the instance's membership check. A finite computation that
  serves a proof of a universal statement has its own statement and key. It enters the universal's argument as a premise
  discharged by that key, beside a theorem leaf saying why it suffices. A bridge lemma is such a theorem.
- **EVD.18 [D]** A claim about a pinned input reaches another revision only by a re-run at that revision or by an exact
  transport that a lemma justifies. Offered elsewhere, it is refused with `EVD:evidence.scope_mismatch`. Exact transport
  along a gauge is the reused leaf of EVD.4, with the gauge theorem and the checked relation Z = X·γ (KN1.16). A result
  on a derived state keeps that state's reachability claim (KN2.13). A leaf that would meet I7 is refused at entry with
  `EVD:evidence.scope_mismatch`. I7 is what a stored or imported record reads, because arguments are immutable (EVD.16).

## 4. The order on evidence

### Grades

- **EVD.19 [P LQ3]** A *grade* is a pair (basis, check) from P ⊂ Basis × Check, at most 30 values. The order is the
  product: (b, k) ≤ (b′, k′) exactly when b ≤ b′ and k ≤ k′.
  - Basis: sample lies below theorem, certificate, exhaustive computation and exact computation, which are pairwise
    incomparable.
  - Check: none lies below producer, and producer below independent checker, model review, human review and formal proof,
    which are pairwise incomparable.
- **EVD.20 [D]** The principle: x ≤ y only when a trust policy that accepts x and rejects y would be incoherent. Everything
  else stays incomparable. A wrong comparability hides one leaf behind another, while a missing one only shows one leaf
  more. So:
  - a later check runs on a result the producer accepted, so what passes it has passed the producer, and "none" detects
    nothing;
  - a sample does not entail its statement even when everything recorded is right;
  - the product is right and an ordinal sum is not, because (sample, independent checker) against (theorem, none) can be
    coherently accepted and rejected either way.
- **EVD.21 [D]** Pairs that make no sense are listed in a versioned table of invalid pairs, and read as typed
  inconsistencies (EVD.40, I1). The table's content is an LQ3 matter. It starts empty.

### Scenarios

- **EVD.22 [D]** A *scenario* is ω = (D, T): D is a down-set of distrusted grades, and T a set of assumed premises. Scenarios
  are ordered by less distrust and more assumptions. val is the least solution of:
  - val(leaf q) = {ω : q ∉ D};
  - val(premise p) = {ω : p ∈ T} ∪ val(C), with C the claim with the identical key, if one exists;
  - AND is ∩, and the empty AND is every scenario;
  - OR is ∪, and the empty OR is ∅.

  The system is monotone on a finite lattice, so the least solution exists (Knaster-Tarski) and is reached by iteration from
  ∅ (Kleene). Every val is an up-set.

### Summaries

- **EVD.23 [D]** An *alternative* is a = (L, Π), with L a set of grades and Π a set of premises.
  Sat(a) = {(D, T) : L ∩ D = ∅, Π ⊆ T}. Every val is a finite union of such sets. a dominates b when Sat(b) ⊆ Sat(a).

**Theorem A (the weakest governs).** Sat(L, Π) = Sat(min L, Π).

*Proof.* If y ∈ L ∩ D, a minimal x ≤ y in L lies in D, because D is a down-set. ∎

**Theorem B.** Sat(L₂, Π₂) ⊆ Sat(L₁, Π₁) exactly when Π₁ ⊆ Π₂ and L₁ ⊆ ↑L₂.

*Proof.* (⇐) Let (D, T) ∈ Sat(L₂, Π₂). Then Π₁ ⊆ T. If x ∈ L₁ ∩ D, some y ∈ L₂ with y ≤ x lies in D, a contradiction.
(⇒) Take ω = (P∖↑L₂, Π₂), which lies in Sat(L₂, Π₂). ∎

**Theorem C (canonical summary).** Each val(v) has exactly one set 𝒮(v) of alternatives whose L are antichains and which
are pairwise non-dominating.

- AND: take pairwise (min(L₁ ∪ L₂), Π₁ ∪ Π₂), then reduce.
- OR: take the union, then reduce.

*Proof.* Existence: use Theorem A and delete dominated alternatives. Uniqueness: let 𝒜 and ℬ be two such sets. For a ∈ 𝒜,
ω_a = (P∖↑L_a, Π_a) lies in some Sat(b), which by Theorem B gives Sat(a) ⊆ Sat(b). Symmetrically Sat(b) ⊆ Sat(a′).
Non-domination forces a = a′, so Sat(a) = Sat(b), and antichains then give a = b. ∎

- **EVD.24 [D]** So the summary does not depend on evaluation order, because ∩ and ∪ are associative, commutative and
  idempotent. It is defined on a DAG, because sharing is harmless. It is monotone: raising a grade or adding an OR child
  enlarges val.
- **EVD.25 [D]** One argument is summarised by its weakest leaves (Theorem A). A statement's summary is a set of
  alternatives, because an OR node has one per surviving argument. The set of weakest leaves is right at an AND node only.

### Cycles and unresolved premises

- **EVD.26 [D]** The least solution is the union of well-founded derivations, so a cycle gives its members only what they
  have from outside. A pure child cycle has val ∅ and reads open with a diagnostic. A circular discharge stays conditional
  on its premises and is never proved. An unresolved premise is a free atom: an open claim C leaves S conditional on p, and
  does not drag S to open.

### Independent arguments

- **EVD.27 [D]** Independent arguments keep every undominated argument. Nothing adds up. If one argument dominates, it
  alone stays. When two are incomparable both stay: two samples stay below every threshold, and no number of them makes a
  proof. This replaces "combine by the strongest", which is undefined on incomparable arguments.
- **EVD.28 [D]** "Independent" is a displayed fact: two arguments share no leaf identity and no tool digest apart from the
  declared common base (kernels, model files). Soundness does not need it. A leaf present in every alternative stays a
  single point of failure, since val ⊆ val(leaf). The graph keeps leaf identities, grades are a projection of them, and
  common failure points are shown.

### Threshold and standing

- **EVD.29 [P LQ3]** The threshold is a versioned up-set 𝒰 ⊆ 𝒰₀ = {(b, k) : b ≠ sample, k ≠ none}. 𝒰₀ is the weakest
  threshold that keeps partition 8.1. Whether to raise it is the owner's question under LQ3, and this chapter uses 𝒰₀ only
  as the bound the proofs need.
- **EVD.30 [D]** Standing is two passes.
  - ω₀ = (P∖𝒰, ∅), and stands₀(v) means ω₀ ∈ val(v).
  - Ref = {p : stands₀(neg p)}.
  - ω₁ = (P∖𝒰, Prem∖Ref), and stands₁(v) means ω₁ ∈ val(v).

  stands₀ does not read Ref, so there is no circularity through negation. Both are one pass of unit propagation, and stands₀
  implies stands₁. The exact alternative form can be exponential and is computed lazily, for display.
- **EVD.31 [D]** A failed check removes its leaf from val and refutes nothing. The claim reads as it would without that
  support.

## 5. The word function

- **EVD.32 [P LQ3]** The owner's words are computed, never stored as truth. The function is a decision list: the first
  match wins, so it is total and single-valued.

| Row | Condition | Word | Claim field |
| --- | --- | --- | --- |
| 0 | Any inconsistency I1 to I7 in Reach(S) | typed inconsistency | none |
| 1 | stands₀(neg S) | refuted | refuted |
| 2 | stands₀(S), and the covered set is one finite instance or a declared search set | finite instance | proved |
| 3 | stands₀(S), and the covered set is an admitted domain or every reachable state | proved, with a mandatory scope phrase ("proved on D[k], for reachable inputs") | proved |
| 4 | stands₁(S) | conditional, premises named | conditional |
| 5 | A search record for S or neg S with complete = no and a stop that is a deterministic budget or a declared limit | not found within budget | open |
| 6 | A job for S or neg S ended cancelled, in error, by wall clock or resource stop, or cut off on a non-search job | incomplete | open |
| 7 | Otherwise | open (a recorded-only claim keeps its source label) | open |

- **EVD.33 [D]** Default 6's three results stay distinct. "Empty" is a completed exhaustive negative (row 2). "Not found"
  is row 5. "Not run" is no record, or the import value of EVD.47 (row 7).
- **EVD.34 [D]** Reachability values (KN2.13) map as follows: witnessed to finite instance; by theorem without a witness to
  finite instance with basis theorem shown; promised to conditional; refuted to refuted; open to open.
- **EVD.35 [D]** A ceiling of the product is not a declared limit, so it reads row 6, matching the boundary between the
  `budget` and `resource_limit` categories (ERR.16).
- **EVD.36 [D]** A display phrase carries the claim's scope. A mapped word without its scope text overstates the claim,
  and a scope that cannot be typed keeps the row recorded (R3-E3). Every row prints its scope in an explanation, row 2
  included: "finite instance" alone hides "three letters of family R only". Row 3's word cannot be shown without its
  phrase. The `claim` field is never a display word. The phrase names the covered set and every *guard* of the
  statement, and `scope_phrase` is computed from `covers` and `guards`. A guard is a hypothesis that meets three
  conditions: the statement's own text holds it, so it is no node of the graph; the membership test of `covers` does not
  decide it; and applying the statement to an instance must discharge it as a premise (EVD.17). A guard reads by its
  premise class: a reachability promise as "for reachable inputs", an open hypothesis as "given" and its name, a model
  assumption as "under" and its name. A hypothesis the membership test decides is not a guard, because the set's name
  already says it. A premise that is a node of the graph is not a guard, because row 4 names it. Where the covered set
  is an admitted domain or every reachable state, a statement whose guards cannot be typed keeps the row recorded.
- **EVD.37 [D]** A conditional refutation is a conditional neg S. S itself reads open, and no further Claim value is needed.

  ```text
  operation word(statement: Key, tables: EvidenceEdition) -> Word
    effect:  pure
    cost:    size of Reach(S)
    errors:  EVD:evidence.inconsistent_fields, EVD:evidence.negative_without_completion

  Word = { word, claim, scope_phrase?, premises?, source_label? }
  ```

- **EVD.38 [D]** *Reach(S)* is the smallest set of nodes containing S and neg S and closed under children, discharge edges
  and negation pairs of premises.
- **EVD.39 [D]** A word is shown with the evidence behind it, on request: the alternatives, their leaves and the common
  failure points. The word is a summary, and the graph is what a reader checks. EV5 owns how that is taught.

### Inconsistencies

- **EVD.40 [D]** Row 0 reads the following. I1 to I4, I6 and I7 read `EVD:evidence.inconsistent_fields`, and I5 reads
  `EVD:evidence.negative_without_completion`.

| Case | Inconsistency |
| --- | --- |
| I1 | A value outside its versioned enumeration, or a pair the table lists as invalid |
| I2 | A discharge edge between different keys, a negation pair that is not mutual, or a leaf whose identity fields differ from its statement's scope |
| I3 | A scope digest coarser than the statement's subject level |
| I4 | stands₀(S) and stands₀(neg S) |
| I5 | A negative or exhaustive leaf whose search record says complete = no, or says complete = yes with job outcome cancelled, cut off, error or not run |
| I6 | A refuting leaf on a universal statement with no scope-membership fact |
| I7 | (a) A leaf of basis certificate, exhaustive computation or exact computation in an argument of a universal statement, or of a mixed statement with outer quantifier universal, whose `covered` is not identical to the statement's `covers` and `input`. In a reused leaf (EVD.4) the source leaf is compared after the transport lemma's checked relation. (b) A leaf of basis exhaustive computation or exact computation whose `covered` is neither one_instance nor search_set. (c) A statement of form finite whose `covers` is not one_instance. |

  A leaf is in an argument of S when it is the direct child of S's OR node, or a child of an AND node under it. A child
  reached through a premise or another key belongs to that key's statement.

- **EVD.41 [D]** Job outcome "not stated" is no contradiction. Unless the record says complete = no, which is I5, it
  makes a negative or exhaustive leaf that rests on a search record inert, which blocks the negative claim, and the word
  stays row 7 with the source label. A witness from such a job counts once it is checked. Two further absences are no
  contradiction. A leaf of basis certificate, exhaustive computation or exact computation with no `covered` fact, in an
  argument I7 (a) reads, is inert. A statement with no `form` fact counts as universal for I7 (a), and a leaf that I7 (a)
  would flag there is inert instead. An inert leaf counts at no threshold, and the word reads as it would without it.

### Non-upgrade

**Theorem D.**

- (a) Locality. word(S) reads only facts on Reach(S) and the versioned tables, so a record that adds no edge into Reach(S)
  changes no word.
- (b) For any 𝒰 ⊆ 𝒰₀, a sample leaf never moves S from rows 5 to 7 into rows 1 to 4.
  *Proof.* A sample leaf's grade is outside 𝒰, so it is false at ω₀ and ω₁. As a new OR child it changes neither pass, nor
  Ref. Rows 1 to 4 read only the passes. ∎
- (b1) A job outcome supplies no support. Let 𝒰 ⊆ 𝒰₀, and let two records differ only in job outcomes, with the same
  leaves inert in both. If S reads a row of 5 to 7 under one, it reads no row of 1 to 4 under the other. *Proof.* A job
  outcome is not a leaf, so it adds no OR child. With the same inert leaves the passes and Ref agree, and rows 1 to 4
  read only the passes. ∎ The inert leaves change in one way only: "not stated" holds a leaf inert (EVD.41), and a
  stated outcome ends that. Completed then activates a leaf the graph already holds, with its own basis, check, covered
  set and complete = yes, so S can move from row 7 to row 2 (CT-EVD-05). The support is that leaf, and the outcome stops
  withholding it. Any other stated outcome on such a leaf is I5, and no outcome lifts complete = no, which is I5 too.
- (b2) Narrower support. Let S be universal, mixed with outer quantifier universal, or without a `form` fact, and let 𝒰
  ⊆ 𝒰₀. A leaf of basis certificate, exhaustive computation or exact computation in an argument of S, whose `covered` is
  absent or, as I7 compares it, not identical to S's `covers` and `input`, never moves S from rows 5 to 7 into rows 1 to
  4. The same holds for refuting an existence statement, whose negation is universal. *Proof.* If S has a form and
  `covered` is present, I7 holds, and row 0 comes first in the decision list. Otherwise the leaf is inert (EVD.41), so
  it is false at ω₀ and ω₁ as a sample leaf is, and it changes neither pass, nor Ref. Rows 1 to 4 read only the passes.
  ∎ (b2) concerns recorded fields: a false `covered` is a false record, which no word can detect, and the check field is
  the defence.
- (c1) Scoped negative. Its covered set is a declared search set, and scope is part of the key and immutable, so its
  word is at most "finite instance". The full-family statement has a different key. I7 keeps a leaf of the narrower
  search out of its arguments, so the only edge to it is a bridge lemma with its own ID (stage 16 `PROOFS.md`, LB5). An
  upgrade happens only by a proof.
- (c2) Reachability promise. T(Z) = AND(theorem on D[k], admission(Z), reach(Z)). val(reach(Z)) at T = ∅ is nonempty only
  through a claim with the identical key: a journal from solved, a checked witness or a theorem. So T(Z) stays conditional
  until that claim exists.
- (c3) `OUTSIDE_CERTIFIED_DOMAIN(k)` is a leaf for ¬Adm_k(Z), whose word is finite instance, with no edge to reachability or
  solvability, so their words do not change. T(Z) falls to open, not refuted. A different k is a different key.

## 6. Refutation and negatives

- **EVD.42 [P LQ3]** Refutation is asymmetric.
  - One exactly checked counterexample refutes a universal statement, however it was found. It needs checked scope
    membership and identical identity fields. A failure at a state whose membership is unknown refutes nothing (I6).
  - An existence statement is refuted only by a proof or a complete certificate.
  - A mixed statement with outer quantifier universal is refuted only by a witness x plus a complete negative at x. One
    with outer quantifier existential is refuted as an existence statement is, only by a proof or a complete
    certificate.
- **EVD.43 [P LQ3]** A negative is proved only for its declared set, by a completed exhaustive search, by a closure-set
  certificate or by a theorem. The floor lemma (`research/jumbling/theory/open-problems.md`) and the all-word exclusions
  on record are negatives proved by a theorem. The floor lemma needs a stage-qualified name before it serves as evidence
  (EVD.2).
  - A cut-off search reads open and keeps its search record.
  - A job that did not complete yields no negative claim (KN7.4). A witness it found counts once it is checked.
  - Offered as a negative anyway, it is refused with `EVD:evidence.negative_without_completion`.
- **EVD.44 [P LQ3]** Sampling never upgrades a universal claim. A checked witness found by sampling does prove an existence
  statement, because its leaf's basis is exact computation or certificate, not sample. How the witness was found does not
  enter the grade.
- **EVD.45 [D]** `OUTSIDE_CERTIFIED_DOMAIN(k)` refutes membership in D[k]. It says nothing about reachability or
  solvability (LB5.4; Theorem D (c3)).

## 7. Certificates

- **EVD.46 [D]** A certificate is a stored object of a named kind with a digest. KN6 owns the checkers. A `CheckReport`
  (KN6.1) enters the graph as one leaf: basis certificate, check independent checker, with the checker's source digest and
  the certificate's byte digest as its identity. The report says that a certificate's claims hold for its stated inputs
  (KN6.6). Which statement that supports is what this chapter's graph records. `verdict: fail` removes the leaf (EVD.31),
  and the kind of failure stays with the report under the codes of KN6. `not_applicable` adds no leaf.

## 8. Mapping the labels on record

### Definitions

- **EVD.47 [D]** Two import values extend job outcome: *not run* and *not stated*. One extends check mode: *stated: not
  replayed*. They are import values only, never KN7 outcomes or modes of checking. "Not run" reads row 7. "Not stated"
  acts as EVD.41 says. "Stated: not replayed" records that a part was not replayed: no check ran there, so it adds no
  check, and no word reads it (EVD.4). A record such as `17/results/runs/pxrr-7304.json`, whose status reads unknown and
  whose reason reads initialised, is "not stated". Only its stage's README says it was stopped (`17/README.md:151`). The
  import may keep that statement beside the record as a separate recorded interpretation with the README's digest
  (EVD.53). It changes no field of the record and no row of the table, and the word stays row 7.
- **EVD.48 [D]** The enumeration rule, from the label table (`rule.txt`):
  - *Corpus.* The Git blobs of a manifest of path, blob digest and size. No importer or test walks a checkout.
  - *Record.* A `.json`, `.json.gz`, `.jsonl` or `.jsonl.gz` blob.
  - *Format.* The innermost enclosing string key `format`. A leaf with none gets a synthetic format from its path.
  - *Field.* The key path from the record root, with `[]` for list items and `<k>` for keys that are numbers, digests or
    state names.
  - *Value.* An enumerated string or a verdict boolean, by the classes of `rule.txt`. A boolean row states its *sense*: which
    of true and false is the positive reading. This is not the negation pair of EVD.9.
  - *Scope text.* A prose field such as `scope`, `limits` or `not_claimed` is kept word for word beside each mapped row and
    counted.
  - *Payload.* Numbers and payload booleans are not rows. They are counted by last key.
- **EVD.49 [D]** The table on record covers the 497 records of stages 14 to 19: 387 triples, of which 298 are mapped, 89 are
  recorded with a reason and none lacks a rule, plus 24 rows from outside the corpus (source evidence, DT-L1 R3-E2).
  [`corpus-labels.json`](../evidence-import/corpus-labels.json) is the table and its README gives the production command.

### Rules

- **EVD.50 [D]** One row rarely states a whole claim. It sets one or more of six fields, and an envelope is built from
  several rows of one record. Mapping is by (format, field, value), never by word: seven exact words have more than one
  mapping in the corpus.
- **EVD.51 [D]** A value whose meaning depends on a sibling field stays recorded until a whole-record rule maps it. That is
  the safe direction, because a recorded claim reads open. Stage 15's `stopped` is a negative for families T, M, E and L and
  not for R, so no single row can say what it means. Row 5 reads a search record's stop reason, and never the word `budget`.
- **EVD.52 [D]** The four rules the label table left to this chapter stand, with these limits:

| Rule | Ruling | Limit |
| --- | --- | --- |
| V14 | Historical `*-independent*` reviewer records map to model review with a fresh audit, and "independent checker" stays for a 1.0 checker under KN6. KN6 is defined by properties those records do not show, and the two checks are incomparable, so the mapping loses no strength. | It applies to reviewer records, not to program checkers such as `replay_word.py` |
| P13 | A check mode may be set from a key's name, because no word reads check mode | A mode read from a key name never creates a transport edge |
| K1 | A row may leave the check to the enclosing record | Only when the enclosing record's check covers the row; otherwise the check reads none |
| H1 | Reachability rows map to open on one finite instance, as claims of their own | An open reach(Z) row adds no OR child, so it can discharge nothing |

- **EVD.53 [O]** Ambiguous records with no `format` key are PS3.12's question, shared with EN4: a source-digest discriminator
  for a table entry, or keeping the whole record as recorded until a format-specific checker identifies it. Either keeps the
  bytes, and an inferred format is a separate recorded interpretation. No complete discriminator table exists yet, and
  until one exists the second option applies by default, because a recorded claim reads open.

### Existing vocabularies

| Source | Labels | How they map |
| --- | --- | --- |
| Research | proved, certificate, computed, measured, open | Directly; "measured" is exact computation on finite inputs |
| Stages 14 to 18 | computed, lead, reported, conditional | Directly; "reported" (a program that is not committed) is a replayability fact and stays visible |
| Command layer | verified, recorded, unknown, unchecked | Directly |
| 0.4 macro certificates | States A, B | Recorded claims until the 1.0 engine re-verifies them (PS3.7) |
| 0.4 macro certificates | State C | A check context: never persisted, never evidence |
| 0.4 macro certificates | State D | The preview token: a permission, outside the evidence fields |

## 9. Worked examples

These come from the note N2. They show the rules on real records and establish nothing further.

**E1, a macro certificate (stage 15, R01).** S₁ is "the word `104:8,46:4,104:15` from `b47d8474…` ends at `1db0dd54…` with
Φ 1376, 1376, 1376, 1350 and excursion 0": finite instance, pose level. S₁ is the OR of three arguments:

| Argument | Leaves |
| --- | --- |
| a₁ | (certificate, producer) from `results/certificates/R01.json`, and (theorem, model review) from Theorem MC |
| a₂ | (exact computation, producer) from `macro.py check --replay` |
| a₃ | (exact computation, independent checker) from `replay_word.py` |

a₃ dominates a₂ by Theorem B. a₁ and a₃ are incomparable, so both stay, and "the strongest" would be undefined. Word:
finite instance. Under a raised threshold that requires a later check, a₁ falls and a₃ carries the same word. All three
share the exact geometry and model files, which is the declared common base. `macro.py` holds both `cert` and
`check --replay`, so a₂ counts as an independent checker only if the tool-digest fact says so.

**E2, a conditional bound (stage 19).**

| Statement | Word |
| --- | --- |
| The fixed-L implication | proved, given Sel: (theorem, model review) |
| The bound "cost ≤ L·R_L(Z₀)", which is AND(implication, premise Sel) | conditional, on Sel |
| U(h, k) on D[k] | "proved on D[k], for reachable inputs": the reachability promise is a guard of the statement |
| U(h, k) applied to a concrete Z | conditional on reach(Z) |
| ¬Adm_k(Z) for a rejected Z | finite instance; T(Z) reads open |

Sel is a mixed statement: every admitted nonterminal state has a legal selected macro. A macro found at one state discharges
nothing, and a failed budgeted search at one state does not refute Sel.

**E3, a cut-off search (the 7302 stop, Φ 1094).** E₃ is "some family-R word with at most three retained letters lowers Φ
here", and S₄ is neg E₃.

| Record | Word |
| --- | --- |
| Beam 64, truncated at three letters: job completed, search complete = no | Both read not found within budget |
| The two-letter subset, exhaustive | It has its own key, and reads finite instance |
| The unbeamed search: `535:1,363:5,535:1,597:14`, Φ 688, 228 descending words | E₃ reads finite instance, and S₄ reads refuted |
| The same search at the 7303 stop, 0 words | A scoped negative: finite instance, three letters of family R only |
| The runs at 7305 and 7306 stopped by the integrator | Cancelled jobs: incomplete |
| The two-level stop at Φ 498 | A negative for the five declared families only; "no word descends there" stays open |

## 10. Where the model lives

- **EVD.54 [D]** Statements, keys, leaves and the graph are data in chapter 05's envelope. The kernel stores the bytes and
  runs the checkers. The word function, the summaries and the passes are pure library code over those facts, with a pinned
  edition of the four tables. They have no session effect, so a bug in them can give a wrong word and cannot change a
  session (overview, section 7).
- **EVD.55 [P LQ10]** Under LQ10's recommendation the envelope stores facts and derives judgements. The nine facts of
  EVD.13 enter the field list of PS1.36. Which word encoding the data core uses stays open (R3-E4). Under the judgement
  alternative the envelope stores the claim fields of EVD.1, and freezes only after LQ3 and this chapter are settled.
- **EVD.56 [D]** Evidence never grants anything. No policy row reads a word, and a stronger claim admits no preview (INV-5,
  BND.81). A preview's token is a permission and never a claim.
- **EVD.57 [D]** Types keep claims away from booleans (LG2.9). A word and a verdict are not interchangeable: `unknown` is not
  false, and `LG2:type.verdict_as_boolean` holds the line in source.

## 11. Errors

- **EVD.58 [D]** This chapter confirms the four reservations of chapter 13 and adds no other code.

| Code | Raised when |
| --- | --- |
| `EVD:evidence.premise_unmet` | A premise is cited with no claim of the identical key (EVD.17) |
| `EVD:evidence.scope_mismatch` | A claim is offered outside its immutable scope (EVD.18) |
| `EVD:evidence.inconsistent_fields` | Row 0 reads I1 to I4, I6 or I7 (EVD.40) |
| `EVD:evidence.negative_without_completion` | Row 0 reads I5, or a negative is offered from a job that did not complete its declared coverage (EVD.40, EVD.43) |

  Other chapters' evidence rows stay theirs: `KN6:evidence.*`, `LB5:evidence.reduction_scope_mismatch`,
  `LB6:evidence.principle_record_incomplete`, `EV1:evidence.obligation_unmet`, `EV2:evidence.replay_mismatch`,
  `EV3:evidence.chart_check_failed`, `LG4:evidence.reproducibility_mismatch` and `PS1:evidence.scope_not_immutable`.
  That last code keeps its name, and it now means a stored claim with revision-bound context in its scope (EVD.10).

## 12. Conformance tests

No conformance test exists yet.

| Test | Checks | Class |
| --- | --- | --- |
| CT-EVD-01 | The 504 node-level cases: stands₀ and stands₁ (6), covered-set kind (4), search record (3), job outcome (7). Each gives exactly one row of EVD.32 or a typed inconsistency | source |
| CT-EVD-02 | Property test on small random graphs: summaries computed in any order are equal, and Theorem C's canonical form is unique | source |
| CT-EVD-03 | Property test: raising a grade or adding an OR child never shrinks val, and sharing a node changes nothing | source |
| CT-EVD-04 | Property test of the two passes: stands₀ implies stands₁, stands₀ reads neither stands₁ nor Ref, and a proof of neg p that puts p in Ref turns stands₁ off for a statement resting on p | source |
| CT-EVD-05 | A sample leaf never moves a statement from rows 5 to 7 into rows 1 to 4, for 𝒰 ⊆ 𝒰₀, and with the same inert leaves neither does a job outcome. Two records that differ only in job outcome, on an exhaustive leaf with matching `covered` and complete = yes, read row 7 under not stated and row 2 under completed; with complete = no both read row 0 (EVD.41, Theorem D (b), (b1)) | source |
| CT-EVD-06 | A record that adds no edge into Reach(S) changes no word (Theorem D (a)) | source |
| CT-EVD-07 | A pure child cycle reads open with a diagnostic, and a circular discharge stays conditional (EVD.26) | source |
| CT-EVD-08 | Discharge by an identical key succeeds; an equivalent statement under another ID is refused until a bridge lemma exists, and a proof for D[7] does not discharge D[9] | source |
| CT-EVD-09 | Each of I1 to I7 raises its code, and "not stated" does not (EVD.40, EVD.41) | source |
| CT-EVD-10 | A scope digest coarser than the subject level is refused with the solved and centre-0-turned pair (EVD.11) | synthetic |
| CT-EVD-11 | A universal is refuted by one checked counterexample with membership, and not without it; an existence statement is not refuted by a cut-off search; a mixed statement with outer quantifier universal needs a witness and a complete negative, and one with outer quantifier existential a proof or a complete certificate (EVD.42) | source |
| CT-EVD-12 | E1, E2 and E3 give the words of section 9 from their records | source |
| CT-EVD-13 | A checked witness found by sampling proves its existence statement, and two samples stay below every threshold (EVD.27, EVD.44) | source |
| CT-EVD-14 | The label table covers every (format, field, value) of a corpus read from Git blobs; each row is mapped or recorded with a reason, and a run from a checkout is refused (EVD.48) | source |
| CT-EVD-15 | A mapped row keeps its scope text, and a row with an untypable scope stays recorded. U(h, k) on D[k] reads "proved on D[k], for reachable inputs", and a statement whose guards cannot be typed keeps the row recorded (EVD.36) | source |
| CT-EVD-16 | A whole-record rule maps `stopped` by family, and a word alone maps nothing (EVD.51) | source |
| CT-EVD-17 | V14, P13, K1 and H1 hold with their limits: a mode from a key creates no transport edge, K1 without a covering check reads none, an open reach row discharges nothing | source |
| CT-EVD-18 | Reused evidence appears as AND(source leaf, transport leaf), and a transport along a gauge names its checked relation (EVD.4, EVD.18) | source |
| CT-EVD-19 | A `CheckReport` pass adds one leaf with its digests, a fail removes it and refutes nothing, `not_applicable` adds none (EVD.31, EVD.46) | source |
| CT-EVD-20 | The word function runs with no session effect, and changing a table edition changes words only through that edition (EVD.14, EVD.54) | source |
| CT-EVD-21 | No policy row, capability or commit path reads a word or a claim (EVD.56) | source |
| CT-EVD-22 | A leaf narrower than its statement reads row 0 or is refused at entry, an absent `covered` is inert, and a mixed(exists) refutation passes (EVD.18, EVD.40, EVD.41, EVD.59, Theorem D (b2)) | source |
| CT-EVD-23 | The blob of `pxrr-7304.json` imports with job outcome not stated and reads row 7, and a README statement kept beside it changes no field. Every mapped job outcome and check mode lies in KN7.3, EVD.4 or EVD.47 (EVD.47) | source |

## 13. Open items

| Item | Clauses | What decides it |
| --- | --- | --- |
| LQ3 | EVD.1 to EVD.3, EVD.19, EVD.21, EVD.29, EVD.32, EVD.42 to EVD.44 | The owner. Whether to raise the threshold above 𝒰₀, and what the invalid pairs are |
| LQ10 | EVD.55 | The owner. Facts or judgements in the envelope, and the word encoding (R3-E4). Under the judgement alternative I7 needs the same facts in whatever stores the justification |
| LQ15 | EVD.3 | The owner. Whether formal proof is a live value in 1.0 |
| PS3.12 | EVD.53 | This chapter and EN4. Dispatch of absent-format records |
| The label table | EVD.49 to EVD.52 | A draft. It freezes with the edition that first uses it |
| The `form` fact | EVD.9, EVD.40, EVD.42 | The label table has no producer for `form`, though EVD.42, I6 and I7 read it. CT-EVD-12 therefore needs the forms of E1 to E3 from a whole-record rule or from the test's own statements |

Under LQ3's alternatives this chapter changes as follows. A simpler model with fewer fields keeps sections 5 and 6 with fewer
rows, and needs its own mapping of the labels on record. Judgements stored directly remove EVD.14's freedom to change a table
as an edition.

Round 3 changed these rules against the partition's first text: partition section 19 lists them.

# Appendix A. Macros

A macro is a step whose word has named parts: a conjugate, a commutator, a pair, a cycle. A rule that plays one
letter is not a macro; its figures are in its chapter. Each entry gives the shape of the word, the rule and the
section that state it, its step tree as the record stores it, and the region it acts on: a picture of an actual
step before and after, with the pieces the step moves drawn opaque.

Notation as in [chapter 1](01-state.md): a letter is (cap, item); a word is read left to right; W⁻¹ is the inverse
word, its letters' inverses in reverse order. Named parts are the nodes of the step tree
([section 1.13](01-state.md#113-step-trees)); `inverse_of` and `same_as` mark a part that repeats an earlier one.

## Bulk

### A.1 conj: (c, x)·(d, y)·(c, z)

- **Rule**: the bulk's three-letter repair U5″ ([section 2.7](02-bulk.md#27-conj-u5-the-three-letter-repair)).
- **Shape**: three one-letter parts named x, y and z; caps c and d are neighbours; the first and last letters turn
  the same cap.

### A.2 lock: open, close, close

- **Rule**: U4 ([section 2.8](02-bulk.md#28-lock-u4-open-close-and-close)).
- **Shape**: an opening letter x at cap c, then two closing letters y and z at neighbouring caps; the record stores
  the word as one span named by the rule.

### A.3 shift: open, then improve

- **Rule**: U5′ ([section 2.10](02-bulk.md#210-shift-u5-open-then-improve)).
- **Shape**: an opening letter x at cap c, then one improving letter y at a cap near c; one span in the record.

## Height

### A.4 r22d88/U2: a pair

- **Rule**: the pair program of r22d81 ([section 3.4.4](03-height.md#344-r22d88u-and-r22d88u2-the-pair-program)).
- **Shape**: two letters, `first letter` and `second letter`, that lower u, the number of straddlers of the target
  τ, where neither letter alone would.

### A.5 h4/pair: the two-letter U step

- **Rule**: h4 ([section 3.5.5](03-height.md#355-h4pair-the-two-letter-u-step)).
- **Shape**: a first letter at a class-A cap that frees a blocked holder, then a second letter at that holder.

## High

### A.6 F2: (c, x)·(d, y)·(c, z)

- **Rule**: the kernel's second family ([section 4.8](04-high.md#48-f2-three-letters-c-xd-yc-z)).
- **Shape**: three named one-letter parts; the third letter turns the first cap again.

### A.7 F3: a·W·a⁻¹·W⁻¹

- **Rule**: the kernel's third family ([section 4.9](04-high.md#49-f3-awa¹w¹)).
- **Shape**: a commutator of the letter a with a word W; the parts a⁻¹ and W⁻¹ are `inverse_of` a and W.

### A.8 D19: a·V·C⁶·V⁻¹·a⁻¹

- **Rule**: the height-4 rule D19 ([section 4.10](04-high.md#410-d19-the-height-4-rule-d19)); worked example R22D-16
  ([section 4.14.2](04-high.md#4142-a-d19-word-r22d-16)), 28 letters.
- **Shape**: a lowering letter a, a setup V, six copies of a commutator C, then V⁻¹ and a⁻¹.

### A.9 S·T: a protected letter, then a landing letter

- **Rule**: [section 4.11](04-high.md#411-st-one-protected-letter-to-height-2-then-one-landing-letter).
- **Shape**: S lowers the unit to height 2 while every height-6 piece stays exact; T lands it.

## Tail

### A.10 zero_bad: a transport that takes nothing off

- **Rule**: [section 5.5](05-tail.md#55-rule-zero_bad-a-transport-that-takes-nothing-off).
- **Shape**: the word of the first path the transport search finds; one span.

### A.11 pure: the class-cell pure word

- **Rule**: [section 5.6](05-tail.md#56-rule-pure-the-class-cell-pure-word).
- **Shape**: a word in the class cell model that moves the unit's content home and takes no lattice piece off
  outside its cells.

### A.12 close: V·W·V⁻¹

- **Rule**: [section 5.8](05-tail.md#58-rule-close-the-closure-vwv¹).
- **Shape**: V vacates the chambers Bad(W) that the landing word W would disturb, W lands the unit, V⁻¹ puts the
  vacated pieces back; V⁻¹ is `inverse_of` V.

### A.13 t1prime: W·V′·W⁻¹·V′⁻¹

- **Rule**: [section 5.9](05-tail.md#59-rule-t1prime-wvw¹v¹).
- **Shape**: a commutator of the landing word W with a vacating word V′; W⁻¹ and V′⁻¹ are `inverse_of` W and V′.

## Lattice

### A.14 Corrections

- **Rule**: L1 ([section 6.5](06-lattice.md#65-rule-l1-choose-the-corrections)).
- **Shape**: a macro of the correction library, played from its first cap; each changes stated coordinates of ν.

### A.15 An orbit step under schedule11

- **Rule**: L3 ([section 6.7](06-lattice.md#67-rule-l3-one-orbit-step-under-schedule11-default-and-acceptance)).
- **Shape**: the factors that solve one orbit, each a twisted cycle g·r⁻¹·c·r·g⁻¹ with a setup g, a route r and a
  cycle c; factors of one orbit can share one seed (`same_as`).

### A.16 A three-cycle of the late restorer

- **Rule**: L3′, development mode only
  ([section 6.8](06-lattice.md#68-rule-l3-one-orbit-step-under-the-late-restorer-development-mode-only)).
- **Shape**: star(X)⁻¹·star(Y), each star a cycle through a tree path; the seed is the cycle A → B → C → A.

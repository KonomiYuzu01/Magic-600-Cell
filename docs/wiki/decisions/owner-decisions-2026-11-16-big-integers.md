---
id: owner-decisions-2026-11-16-big-integers
type: decision
status: verified
visibility: public
summary: Owner decision of 16 November 2026 - LP2's unbounded integers use num-bigint 0.5.1 with num-integer 0.1.47 and num-traits 0.2.19 (option A), not the experiment's own limbs. Each crate is MIT OR Apache-2.0 as read from the verified archives of LX-5's crate run. M2, the interpreter and its reference meter, no longer waits.
related: [owner-decisions-2026-11-14-language-route, owner-decisions-2026-11-16-time-saving, owner-decisions-2026-11-15-freeze]
supersedes: []
claims:
  - {id: big-integers-option-a, evidence_kind: decision, checked_at: 2026-11-16}
  - {id: lx5-crate-exact-and-licences, evidence_kind: source, path: work/experiments/language-lx/lx5-crate/RESULT.md, checked_at: 2026-11-16}
  - {id: lx5-decisive-timing, evidence_kind: performance, path: work/experiments/language-lx/quiet-window/2026-11-14T1818Z/RESULT.md, checked_at: 2026-11-16}
---

# Owner decision, 16 November 2026: big integers through num-bigint (option A)

The owner gave this decision in chat in Chinese on 16 November 2026 ("for the big-integer question, choose A"). This
page records it in English.

## Question

M2's one question card (LP2 plan, section 7, item 1; workbench card `20261115T224140Z-748e3d6b`) asked how LP2
implements unbounded integers (RTE.18). Either answer touches a product dependency, which needs the owner (RTE.70):

- **A**: `num-bigint` 0.5.1 with `num-integer` 0.1.47 and `num-traits` 0.2.19;
- **B**: the experiment's own limbs, with no dependency.

The card recommended A.

## Decision

Option A. LP2's big-integer implementation in the Rust workspace `rust/` uses these crates, pinned exactly, with
default features off and `std` on, as in LX-5's crate run:

| Crate | Version | Licence, read from the verified archive | Archive SHA-256, checked against `Cargo.lock` |
| --- | --- | --- | --- |
| `num-bigint` | 0.5.1 | MIT OR Apache-2.0 | `93e7820bc0a80a0238e650327316f929ba18d5be054b647490a3a6a339f3e7c0` |
| `num-integer` | 0.1.47 | MIT OR Apache-2.0 | `7ce2d95d4b3734dc35aa2f45e1aa22cd416814592a4f9d9205e11affd5b8e10b` |
| `num-traits` | 0.2.19 | MIT OR Apache-2.0 | `071dfc062690e90b734c0b2273ce72ad0ffa95f0c74596bc250dcfd960262841` |
| `autocfg`, a build dependency of `num-traits` | 1.5.1 | Apache-2.0 OR MIT | `f2032f911046de80f0a198e0901378627c33f59ea0ac00e363d481118bd70a53` |

M2's interpreter and its reference meter are the first users. Another version or another crate needs the owner's
approval again (RTE.70).

## What the choice rests on

- **Exactness.** LX-5's crate run gave 3,374 of 3,374 frozen cases byte-identical to LX-5's reference
  (`work/experiments/language-lx/lx5-crate/RESULT.md`). Source and synthetic evidence.
- **Speed.** In the owner's quiet window of 14 November 2026, which the owner accepted as decisive (RTE.82),
  `num-bigint` took 340.7 ms in total and the own limbs 1,145.8 ms, about 3.4 times slower
  (`work/experiments/language-lx/quiet-window/2026-11-14T1818Z/RESULT.md`). `num-bigint`'s slow rows are random
  22,500-bit inputs, with gcd and sign hot spots (RTE.18). This is performance evidence for these rows on the owner's
  machine, not a product performance claim.
- **Audit.** The own limbs are no audited library (RTE.18).

## What follows

- M2 no longer waits: the LP2 plan's section 2 and its M2 row say so.
- M2 adds the crates with the checksums above and keeps `cargo test --locked --offline` as the runner (LP2 plan,
  section 3), so the sources must be available to an offline build. How they get there is M2's choice within this
  approval; a choice that needs another source, a cost or a network service goes to the owner first.
- `rust/README.md` lists the approved crates with their licences (LP2 plan, section 3, item 2). The upstream licence
  notices are kept wherever the sources are copied (`AGENTS.md`).
- Rational literals are reduced to lowest terms in M2 (LP2 plan, section 8, item 3).
- No frozen text changes. No progress item is ticked for the answer: M2's item stays open until M2's pull request.
- The workbench card stays in the owner's inbox until the owner closes it; `ask.py` has no withdraw command.

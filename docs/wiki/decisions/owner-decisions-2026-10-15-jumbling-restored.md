---
id: owner-decisions-2026-10-15-jumbling-restored
type: decision
status: verified
visibility: public
summary: Owner decision of 15 October 2026 - for the jumbling research a configuration counts as restored when every sticker slot carries its home label; a macro may be very long as long as a person can replicate it with the program's help; most important is that the machine has a general method; the experiments are to be wrapped up quickly.
related: [owner-decisions-2026-10-13-jumbling-menu, owner-decisions-2026-10-10-jumbling-complete, owner-decisions-2026-10-10-jumbling-deferred]
supersedes: []
claims:
  - {id: sticker-definition-of-restored, evidence_kind: decision, checked_at: 2026-10-15}
  - {id: long-macros-with-program-help, evidence_kind: decision, checked_at: 2026-10-15}
  - {id: machine-general-method-first, evidence_kind: decision, checked_at: 2026-10-15}
---

# Owner decision, 15 October 2026: what counts as restored

The owner gave this decision in a chat message in Chinese; this page records it in English. It relaxes the restoration requirements of the [10 October](owner-decisions-2026-10-10-jumbling-complete.md) and [13 and 14 October](owner-decisions-2026-10-13-jumbling-menu.md) decisions for the jumbling research.

## Decisions

1. **Sticker definition.** A configuration counts as restored when all 259,800 sticker slots carry their home labels. A piece may still carry a rotation that maps the piece and each of its stickers onto themselves. Its exact pose then differs from the solved pose, but the configuration counts as restored.
2. **Length.** A macro may be very long, as long as a person can replicate it. The program may help the person do so.
3. **Priority.** Most important is that the machine has a general method: restoration from every reachable configuration.
4. **Pace.** The experiments are to be wrapped up quickly.

## Effect

- **The off-lattice phase is unchanged.** While a piece is off the lattice, the lattice has a hole: the number of holes equals the number of off pieces. Such a configuration is not restored under the sticker definition either.
- **What the decision removes.** It removes three requirements of the earlier programme:
  - the exact pose at the end;
  - fixed length limits;
  - reading the state by eye in the theory. The program reads and checks the state.
- **The research priority** is a general method that works from the current configuration alone, in [research/jumbling/restoration](../../../research/jumbling/restoration/README.md).
- **Unchanged.** Three things stay as they were:
  - 600-cell-Full, its model identity and its contract;
  - the I_a menu;
  - the product's human-solve boundary of 2 October 2026. This decision sets the research definition of restored and the research priority, not product behaviour.

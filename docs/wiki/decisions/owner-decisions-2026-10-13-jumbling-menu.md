---
id: owner-decisions-2026-10-13-jumbling-menu
type: decision
status: verified
visibility: public
summary: Owner decisions of 13 and 14 October 2026 - the jumbling puzzle's twist menu is I_a (the 12 retained rotations and 48 jumble twists of one icosahedral group at every grip), chosen ahead of the 2.5 freeze; a restoration method a person executes has no fixed size limit as long as its counts and lengths are not astronomical, and its macros are kept as short and as few as possible.
related: [owner-decisions-2026-10-09-jumbling, owner-decisions-2026-10-10-jumbling-deferred]
supersedes: []
claims:
  - {id: menu-is-i-a, evidence_kind: decision, checked_at: 2026-10-13}
  - {id: human-scale-not-astronomical, evidence_kind: decision, checked_at: 2026-10-14}
  - {id: i-a-exact-definition, evidence_kind: source, path: research/jumbling/theory/groups.py, sha256: e2ebd8cdb14e8ca19f515c1ae464b79bbcca51be797adbb8dfe1e5b481ca2d72, checked_at: 2026-10-14}
---

# Owner decisions, 13 and 14 October 2026: jumbling twist menu and human scale

The owner gave these decisions in chat messages in Chinese; this page records them in English.

## Context

- **The open menu.** The [9 October decisions](owner-decisions-2026-10-09-jumbling.md) left the twist menu, which defines the jumbling puzzle, to an owner sign-off at the 2.5 architecture freeze.
- **The candidates.** Besides the retained menu A4₀, the menus with finite same-cap behaviour are S4₀, I_a and I_b ([theory draft](../../../research/jumbling/theory/theory-draft.md), Proposition 3.1). I_b is the mirror image of I_a. A menu that combines two of the three lies in none of them, so it gives an infinite puzzle (Corollary 3.4).
- **The recommendation.** The integrator recommended I_a, for three reasons:
  - the deep-restoration research since 10 October has been done on I_a;
  - I_b would only mirror that work;
  - S4₀ would start it again.

  It suggested treating I_a as a working assumption until the freeze. The owner chose to decide now.

## Decisions

1. **The twist menu is I_a** (13 October 2026). The owner chose it now, ahead of the 2.5 freeze where the 9 October decisions placed the sign-off.
   - **Shape.** At the base grip, I_a is the rotation group of the regular icosahedron whose vertices are the cyclic permutations of (0, ±1, ±φ) in the cap frame of `research/jumbling/theory/groups.py`. The twofold axes of A4₀ are that frame's coordinate axes.
   - **Exact definition.** I_a is the closure of A4₀ and the 72° rotation about (0, 1, φ) (`groups.py`).
   - **Elements.** It has 60 rotations: the 12 retained rotations of A4₀ and 48 jumble twists. The jumble twists are:
     - 24 about the six fivefold axes;
     - 12 about the six threefold axes outside A4₀;
     - 12 about the twelve twofold axes outside A4₀.
   - **Other grips.** Every other grip's menu is the K⁺ transport of I_a ([state contract](../../../research/jumbling/state-contract.md), amendment A4).
   - **Identity.** The J1 menu identity is `fddc8532b9184bd1b413c1032b5a7a305dd9fd1f10385d87ba6ec4fd06eb958c`. It is the value in the [mid-stage table](../../progress/1.0/jumbling-midstage.md), recomputed on 14 October 2026 from `groups.py` with J1's `TwistMenu.identity`.
2. **Human scale has no fixed numeric limit** (14 October 2026). For a restoration method that a person executes, the integrator proposed limits:
   - at most 20 named macros, each of at most about 100 twists;
   - a whole restoration of at most about ten times a retained solve.

   The owner set no numbers. Counts and lengths are acceptable as long as they are not astronomical, and the macros are kept as short and as few as possible.

## Effect

- **Research.** The jumbling research concentrates on I_a. S4₀ and I_b are no longer studied separately; their existing data, fixtures and results stay as comparisons. Open problem X1 in [open-problems](../../../research/jumbling/theory/open-problems.md) now asks for the finiteness of R(I_a) only.
- **Restoration requirement.** It is unchanged: the method must be human-executable, read only the visible configuration and be complete ([deferred-problems decision](owner-decisions-2026-10-10-jumbling-deferred.md)). Decision 2 is the size criterion for "human-executable".
- **Puzzle definition.** The jumbling puzzle's model identity is built on this menu ([jumbling plan](../../progress/1.0/jumbling-plan.md), section 3b: one model identity per signed menu). Of the three W-J fixtures, `research/jumbling/fixtures/wj-I_a.json` is the one for the chosen menu.
- **Unchanged.** 600-cell-Full, its model identity and its contract are unchanged, and the human-solve boundary applies unchanged.
- **Records left as written.** The engineering packet ENG-0J and the plan-check packet were written before this decision and are left as handed over; they still name the menu as an open sign-off.

## Where it is applied

- [jumbling-plan](../../progress/1.0/jumbling-plan.md): section 2, step 3, and amendment A4 in section 4.
- [jumbling-midstage](../../progress/1.0/jumbling-midstage.md): section 4.
- [restoration](../../../research/jumbling/theory/restoration.md): section 5, "What 1.0 takes from this".
- [open-problems](../../../research/jumbling/theory/open-problems.md): X1.

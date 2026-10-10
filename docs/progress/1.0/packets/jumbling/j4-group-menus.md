# Packet: J4 explorer with the exact group menus and J1 menu identities (implementation)

Run as an implementation with the default reviewer model, effort `max` and speed tier `fast`, in the worktree the wrapper creates. Change only the allowed files.

## 1. Goal and acceptance
- Goal: mid-stage row J4 of `docs/progress/1.0/jumbling-plan.md` section 2 asks for orbit data for the candidate menus, each with its exact menu identity. Two gaps remain:
  - The finite same-cap candidates are the groups S4₀, I_a and I_b containing A4₀ (`research/jumbling/theory/theory-draft.md`, Proposition 3.1). J4 has no run for them. Its `class-00` menu (A4 with the conjugates of a quarter-turn, 18 elements) is not the group S4₀ (24 elements).
  - J4 menus carry no identity comparable with J1's `TwistMenu.identity`, so J4 results cannot be matched with J1, J2 or the W-J fixtures.
- **Group menus.** Add three exact menus to `all_menus`: `s4` (24 elements), `i_a` (60) and `i_b` (60), family `group`.
  - Their exact elements come from `research/jumbling/theory/groups.py` (`build()` gives 3 × 3 cap-frame matrices for J1 pole 0, `lift()` the exact 4 × 4 rotations).
  - J1 and J4 use the same coordinates of R⁴, but J4's base pole n₀ is a different pole from J1's pole 0. Map each element to J4's base pole by exact conjugation with a K⁺ element that takes J1's pole 0 to J4's n₀, found and checked exactly (pole directions compared exactly in Q(√5)).
  - Each mapped element must be checked exactly to be a rotation fixing J4's n₀. Each menu must be exactly closed under multiplication and inverses, and must contain J4's A4 exactly.
  - `Menu` takes generators and closes them under A4 conjugation and inverses; passing all group elements as generators must give exactly the group. Assert the sizes 24, 60 and 60.
- **J1 menu identities.** For every exact J4 menu (all except `plane-36-float`), compute `j1_menu_identity`: conjugate each element back to J1's pole 0 exactly, and take `sim.TwistMenu(ctx, name, items, close=False).identity` (`research/jumbling/sim/twists.py`). Record it in the menu's `describe()` output. Record also `j1_relation`: whether the menu equals or is contained in J1's S4 (`sim.TwistMenu.s4`) or in the I_a or I_b group from `groups.py`, decided by exact matrix sets.
- **Runs.** Add a way to run the preset for named menus only and merge the runs into `explorer-results.json` and `points/`, leaving the other runs byte-identical: for example `--preset --menus s4,i_a,i_b`. The preset parameters stay as they are. The integrator runs it outside the sandbox.
- **Viewer and README.** The menu picker lists the group menus with their family and size; the detail panel shows `j1_menu_identity` (shortened, full value on hover or in a details row) and `j1_relation`. The README describes the group menus, the frame map and the identity rule.
- **Self-test.** Extend `--selftest`:
  - lattice closure of `s4` to depth 2, reduced against the direct brute-force count, as for the existing menus;
  - `j1_menu_identity` of `a4` equals `sim.TwistMenu.a4(ctx).identity`, and of `s4` equals `sim.TwistMenu.s4(ctx).identity`;
  - `i_a` and `i_b` identities equal those of `sim.TwistMenu` built from `groups.py` without the frame map;
  - `class-00` is reported as contained in S4 and not equal to it.
- **Acceptance.** The acceptance check below exits 0 inside the sandbox and prints `ok` for every self-test line. `node --check research/jumbling/explorer/explorer.js` passes. The integrator then runs the merged preset for the three menus and the headless page check outside the sandbox.
- Out of scope:
  - `research/jumbling/sim/` and `research/jumbling/theory/` (read only);
  - existing menus and runs, except the added identity fields;
  - look, colour and motion;
  - performance.

## 2. Actual problem and reproduction
- `python research/jumbling/explorer/explore.py --list-menus` lists 38 menus: `a4`, `class-00` to `class-32`, `plane-10`, `plane-36`, `plane-72` and `plane-36-float`. None is S4₀, I_a or I_b.
- J1's pole 0 and J4's n₀ differ: J1 pole 0 is (0.9256, 0.2185, 0.2185, 0.2185) after normalising, and J4's n₀ is (−0.9256, −0.3536, 0, −0.1350). Every J4 pole direction equals some J1 pole direction in the same coordinates.

## 3. Environment and versions
- Branch `claude/jumbling-1-0`, committed head. Linux, Python 3.11 or later with NumPy, Node 22. The page loads three.js 0.160.0 from jsDelivr; Playwright with headless Chromium may not run inside the sandbox, so the integrator runs `check_page.mjs`.

## 4. Necessary source and evidence
- `research/jumbling/explorer/explore.py`: `Geometry` (poles, K⁺, frames, A4), `Menu`, `all_menus`, `selftest`, `preset`, `main`.
- `research/jumbling/explorer/explorer.js`, `index.html`, `check_page.mjs`, `README.md`.
- `research/jumbling/theory/groups.py`: `build`, `lift`, and the exact closure checks.
- `research/jumbling/sim/`: `get_context`, `ctx.data.N` (exact poles), `ctx.kplus.matrix`, `ctx.kplus.frame_idx`, `TwistMenu` (`identity`, `a4`, `s4`, `close=False`), and `README.md`.
- `research/jumbling/exact.py`: the exact Q(√5) type shared by J1, J4 and the theory scripts.

## 5. Attempts so far
| # | Step | Result |
|---|---|---|
| 1 | J4 explorer with 38 menus and the preset (`434c8fd`) | integrated; not reviewed yet |
| 2 | Exact groups S4₀, I_a, I_b in J1's frame (`research/jumbling/theory/groups.py`) | exactly closed, contain A4₀, every element a rotation fixing pole 0 |

## 6. Constraints and owned files
- Change only the allowed files below. Do not commit; the wrapper collects the patch. No float decision may replace an exact check: floats may propose a match, and exact arithmetic confirms it.
- Every identity is computed, never typed in by hand.

```implement-contract
{"allowed_files": ["research/jumbling/explorer/explore.py", "research/jumbling/explorer/explorer.js", "research/jumbling/explorer/index.html", "research/jumbling/explorer/check_page.mjs", "research/jumbling/explorer/README.md"], "acceptance_check": ["python", "research/jumbling/explorer/explore.py", "--selftest"], "stop_condition": "the three group menus are exact and checked, every exact menu has its J1 identity and relation, the preset can run named menus and merge, the viewer and README show them, and the self-test passes"}
```

## 7. Required return format
- Changes only in the assigned worktree. The final message lists:
  - the changed files;
  - the frame map used (the J1 pole matching J4's n₀ and the conjugating element);
  - the J1 identity and relation of every exact menu;
  - the self-test output;
  - open points.

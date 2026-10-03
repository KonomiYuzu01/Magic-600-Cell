# Stage 2.2 dispositions: method

Status: **signed by the owner on 3 October 2026** ([owner-decisions-2026-10-03](../../../wiki/decisions/owner-decisions-2026-10-03.md) sections 4 and 5). The exit of 2.2 is that every inventory item has keep, redesign, delete or automate with a reason, and the owner signs every `delete` and `automate` decision (charter sections 3 and 8). The result is in [dispositions.md](dispositions.md).

## Input

- The 2.1 result: the twelve sealed inventory files and the integrator's correspondences in [../inventory/](../inventory/README.md). The inventory files are never edited.
- [solving-workflow.md](../solving-workflow.md), [charter](../charter-2.0-draft.md) sections 2, 4 and 5, the owner's design decisions ([owner-decisions-2026-10-02-design](../../../wiki/decisions/owner-decisions-2026-10-02-design.md)) and scope ([owner-decisions-2026-10-02-scope](../../../wiki/decisions/owner-decisions-2026-10-02-scope.md)), and the [command table](../command-table.md).
- [flows.json](flows.json): the flows a unit can serve. **The flows are provisional.** They come from the solving workflow and charter section 5 until the owner's core-flow storyboards (protocol section 2.1) replace or confirm them. A change of flows reopens only the rows whose flows changed.

## Units

[build_units.py](build_units.py) writes [units.json](units.json). A unit is one 0.4 function: the inventory rows the correspondences join, closed transitively. A row with no correspondence is a unit of its own. Units never cross shards.
- 503 units come from 1,293 rows.
- `depends_on_units` and `used_by_units` link layers: a browser button, the adapter action it calls and the engine route behind it. The two inventories name one route or command in different notations; `normalize()` maps the known ones to one key. Links are listed, not merged, and a dependency is evidence for `same_purpose_as`, not proof of it.
- `unresolved_depends_on` keeps the dependencies that name a module function or free text rather than another row's entry point; the builder prints their count per shard.
- A correspondence group can join rows with different purposes (a coarse row of one inventory matched to several fine rows of the other). Such a unit gets one `split` row whose `parts` decide subsets of its inventory rows; every row is in at least one part, and each part is its own decision in the report and in the owner's signature table.
- `screening` lists the 0.4.1 findings attached to the unit's rows; `screening_rows` names the rows each one is attached to. `open_candidates` lists units the 2.1 matching left as unrecorded candidate pairs.

## Dispositions

Each unit is judged **by its purpose, not by its 0.4 presentation**. 0.4 is not a UX baseline (protocol section 1), so every 1.0 screen is new whatever the disposition.

| Disposition | Meaning | Owner signs |
|---|---|---|
| `keep` | The purpose and the behaviour carry into 1.0 as a contract: same inputs, results, guarantees and mathematics, only re-hosted in the six layers. Typical: engine mathematics, journal semantics, protection checks. | no |
| `redesign` | The purpose is kept; the behaviour, interaction, granularity or host changes. For example, it is merged with other units into one command, moved to another layer, or drawn instead of written. A 0.4 host control (a native or browser button) whose purpose survives is `redesign`, never `delete`. | no |
| `delete` | The purpose is not needed in 1.0: no flow uses it, it serves only 0.4 infrastructure that 1.0 does not inherit (the retained MPUlt runtime, Managed DirectX, the 0.4 packaging and release line), or it crosses the human-solve boundary. | yes |
| `automate` | The purpose is kept, but the solver no longer does it by hand; the program does it as bookkeeping. Automation never chooses, outputs or executes a solving macro (human-solve boundary). | yes |

Rules:
- A `keep` or `redesign` unit serves at least one flow. A unit no flow uses is a `delete` or `automate` candidate.
- The human-solve boundary decides by itself: anything that chooses or outputs a solving macro, executes without the solver's action, or solves automatically is `delete`. Setup search for a target the solver names, explained candidates and the analysis of the solver's own macros stay.
- `same_purpose_as` names the units in other layers or shards with the same purpose, so that a purpose gets consistent dispositions across layers. Layers may differ legitimately: an engine function `keep`, its 0.4 button `redesign`.
- `requirements` carries what 1.0 must do differently, starting with every open or needs-verification screening finding attached to a unit (or part) whose purpose survives, that is `keep`, `redesign` or `automate`, written as `<finding id>: <requirement>`. 0.4.1 screening findings are 1.0 requirements (briefing section 1).
- `reason` is one or two sentences that cite the flow, the boundary, the charter or the decision it rests on. `purpose` states what the user gets, independent of 0.4's controls.

## Sequence

1. This bootstrap (units, flows, checker, report, tests, packets) gets a plan check before the shards start.
2. Six Codex shards draft `dispositions-s1.json` to `-s6.json` concurrently through the wrapper (`--kind implement`, packets in [../packets/](../packets/)). Nothing is committed, tagged, fetched or pushed in the repository while they run.
3. The integrator reads every `delete` and `automate` decision and every purpose group with mixed dispositions (`delete` against any surviving purpose, or `automate` against `keep` or `redesign`; [build_report.py](build_report.py) lists them), checks a sample of `keep` and `redesign` rows against the source, and corrects rows with a recorded reason in `notes`.
4. `check_dispositions.py --all` passes (each shard file exactly once, under its own name, and every unit of `units.json` decided) and `build_report.py` writes [dispositions.md](dispositions.md).
5. One Codex Astra review of the finished candidate, then the owner signs the `delete` and `automate` rows. The signature is recorded in a wiki decision page, and the owner's changes are applied to the shard files.

## Integrator check (3 October 2026)

Six Codex Sol shards drafted the files from commit `0472333` (calls `20261003T170236Z-81491429`, `-170256Z-df5e8341`, `-170316Z-6fe9efa8`, `-170336Z-ce4710a3`, `-170356Z-6ee02d0b`, `-170416Z-da016664`); every patch passed its acceptance check and was applied unchanged.
- Draft result: 503 units, 556 decisions (keep 52, redesign 479, delete 19, automate 6). After the owner's ruling below: keep 52, redesign 486, delete 12, automate 6; see [dispositions.md](dispositions.md).
- All 25 drafted `delete` and `automate` decisions were read against the source and the rules. They fall into four groups:
  - 0.4 infrastructure that 1.0 does not inherit: the browser client, the frozen engine entry, Managed DirectX preparation and the MPUlt regression, inspection and patch modes;
  - the standalone Global or Local view dismissal, because the owner decided that both views are visible together;
  - program-built solving recipes: the insertion planner (`/api/suggest`, U1-063 and U4-038/2) and the endgame family constructors;
  - bookkeeping that becomes automatic: completion receipts, cycle-inspection refreshes, session-report refresh and the library effect check. Starting and pausing the session timer stays an explicit action (U4-050/1, U6-039).
- **Owner ruling (3 October 2026, [owner-decisions-2026-10-03](../../../wiki/decisions/owner-decisions-2026-10-03.md) section 4).** A construction that the solver selects by family and parameters (placement star, orientation transfer, buffer-A correction, final-buffer commutator) is the solver's own macro. The seven drafted deletions U2-013, U2-015, U2-017, U2-019, U3-093, U3-095 and U6-033/2 are therefore `redesign`, each with a note naming the ruling. Setup search for a named target (U1-053, U4-045), preview of a star the solver selects (U4-046) and composition of the solver's own macros (U1-058, U4-051) were already kept. The insertion planner stays `delete`: it chooses targets, stars, transfers and corrections from the orbit state itself.
- Mixed pairs in the report were checked. Every pair is one of four legitimate kinds:
  - a split unit whose other part keeps the purpose;
  - a 0.4 host control redesigned while the 0.4 serving infrastructure is deleted;
  - analysis kept while construction by the program is deleted;
  - retained analysis whose invocation or refresh becomes automatic (U2-005 and U3-104, U2-034 and U3-085).
- A sample of 14 `keep` and `redesign` decisions was read against their units: the flows, reasons and requirements were consistent. No row was changed.

## Evidence

Source only (Linux cloud session). No Windows, native-host or input-device behaviour is claimed beyond what the inventories say.

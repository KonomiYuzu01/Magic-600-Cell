# Stage 2.2 dispositions: method

Status: **method and draft for the owner.** The exit of 2.2 is that every inventory item has keep, redesign, delete or automate with a reason. The owner signs every `delete` and `automate` row (charter sections 3 and 8). Rows here are proposals until then, and the result is in [dispositions.md](dispositions.md).

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

## Evidence

Source only (Linux cloud session). No Windows, native-host or input-device behaviour is claimed beyond what the inventories say.

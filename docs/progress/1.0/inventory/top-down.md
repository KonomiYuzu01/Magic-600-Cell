# Stage 2.1 top-down inventory (from documents)

Status: **draft for the 2.1 merge on stage day 3**. Written by Claude on 1 October 2026 from user and design documents only. No source code was read, so this list is independent of the two source-derived inventories (the Codex shards `../packets/inventory-bottom-up-s1.md` to `-s6.md` and the Claude inventory; method in [README](README.md)), which read only the code. It is kept as a third reference; the source-derived inventories must not read it before they are sealed.

Data: [top-down.json](top-down.json), 72 functions and 7 unclear items. Each evidence anchor is resolved to a `path:line` by [build_top_down.py](build_top_down.py) (`python docs/progress/1.0/inventory/build_top_down.py`). The script fails if an anchor is missing.

## Sources

| Kind | Documents |
|---|---|
| Release-facing | `USAGE.md`, `README.md`, `docs/RELEASE_0_4.md`, `work/experiments/magic600-04/packaging/CHANGES.md` |
| 0.3 and shared reference | `docs/STRUCTURE_EXPLORER.md`, `docs/LIMITATIONS_AND_ROADMAP.md` (historical table) |
| 0.4 development guide and design contracts | the archived snapshot under `docs/progress/0.4/snapshot/`: the native review guide, the Functions, keymap-file, session and session-log evidence notes, the display-controls review, the product brief (`01`), the naming contract (`07`) and the integration and endgame contract (`08`) |

Some evidence notes name source files. Those names were not followed; only the described behaviour was used.

## Row format

The rows use the same fields as the bottom-up shards: `id`, `name`, `entry_points`, `reads`, `writes` and `engine_call`. `engine_call` is always `null` here, because the documents do not name engine calls. Three fields are added:
- `area`: launch, data, process, session, files, solve, macros, protection, input, view or structure;
- `purpose`: what the user gets from the function;
- `status`: how strong the documentary evidence is.

| Status | Meaning | Rows |
|---|---|---|
| `release-doc` | A release-facing document describes the function | 42 |
| `0.4-dev-doc` | Only the 0.4 development guide or a design contract describes it | 22 |
| `0.3-reference` | Only a 0.3 or shared reference describes it; presence in 0.4 is unconfirmed | 7 |
| `unclear-in-0.4` | Described historically; its form in 0.4 is not stated | 1 |

`reads` and `writes` are inferred from the descriptions. Treat them as hypotheses for the merge.

## Areas

| Area | Rows | Covers |
|---|---|---|
| solve | 23 | Solve window; Prepare, Macro and Cleanup; check, preview and execute; forecasts; cycle views; Local and Global views; Current and Next; buffers; blocks; intents; residuals; findings; cross-orbit work; recommendations; names |
| session | 9 | New, Resume, timer, scramble, reset, completion summary, undo and redo, checkpoints, reports |
| input | 9 | onscreen keyboard, Grip and Twist, live or draft destination, banks, Set grips, Functions bank, key editor, command index, physical keys |
| view | 7 | camera and mouse twists, inspection by click, piece filter, frame visibility, adaptive detail, display sliders, instant turn |
| structure | 6 | structure explorer, cell layers, auxiliary cell views, center viewport, saved sets, browser client |
| macros | 5 | Macro Base, bank fixed macros, worksheets and comparison, endgame families, reference variants |
| launch, data, process | 7 | G2 and G1 start, `--data`, profile separation, rollback, engine lifecycle, window management |
| files | 3 | C600 and MPUlt log export, log check and import, keymap files |
| protection | 3 | policies (net or strict), orbit checks and automatic orbit protection, position protection |

## Unclear items for the merge

These seven items should be settled from the code shards on day 3:
- whether the structure explorer, cell layers, auxiliary views and saved sets can be reached in 0.4 G2;
- G1 and G2 parity, including ordered-frame capture;
- which scramble kinds exist;
- whether Reset view and Reset workspace are separate actions;
- whether macros can be recorded;
- whether development fixtures can be reached from All commands in the release;
- whether 0.4 has a progress-report view.

The JSON lists the evidence for each item.

## Merge procedure (superseded)

Superseded on 2 October 2026: the 2.1 comparison is now between the two source-derived inventories (Codex and Claude), with recorded correspondences ([README](README.md)); `merged.json` in this directory is that comparison. This list may still be compared with it as a third reference, as a separate step that writes its own file. The original procedure is kept for the record:

1. Pair each top-down row with bottom-up rows by entry point (key, menu, button or command ID), then by name.
2. Put every row into one of three groups: **matched**, **documented only** (described in the documents but not found in the code), or **code only** (reachable in the code but undocumented).
3. Every disagreement is an input to 2.2. Documented-only rows include promised functions that were never built. Code-only rows include hidden or developer functions, which are candidates for DELETE or AUTO and need the owner's signature.
4. The merged list was to be `merged.json` in this directory, each row citing both its document evidence and its code evidence.

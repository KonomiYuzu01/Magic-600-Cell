# Stage 2.1 inventories: method

Status: **method and first result of the 2.1 inventories (owner instruction of 2 October 2026).** The comparison is in [merged.md](merged.md). No row here carries a keep, redesign, delete or automate judgement; dispositions are stage 2.2 and the owner's.

Two independent inventories of every user-visible 0.4 function are written from the source code alone, one by Codex and one by Claude, sealed before either side reads the other, then compared. The document-based [top-down list](top-down.md) is kept as a third reference only.

## Files

| File | What it is |
|---|---|
| [shards.json](shards.json) | The owner's source scope and the shard (S1 to S6) that owns each tracked file; excluded files carry a reason. |
| [check_inventory.py](check_inventory.py) | Format check of one inventory file; `--coverage` checks `shards.json` against the tracked files. Tests: `tests/test_inventory_check.py`. |
| `bottom-up-s1.json` to `-s6.json` | Codex inventory, one file per shard (packets: `../packets/inventory-bottom-up-s1.md` to `-s6.md`). |
| `claude-source-s1.json` to `-s6.json` | Claude inventory, same scope and format, written by Claude subagents without access to the Codex files. |
| `sealed.json` | SHA-256 digests of the twelve files, recorded before either side's file was read by the other side or by the merge. |
| `census.py`, `census.json` | A source-derived list of entry points of four kinds (command IDs, engine routes, adapter actions, CLI options), written after sealing, used to audit both inventories for omissions. The sealed files are never edited. |
| `screening_map.py`, `screening-map.json` | Each 0.4.1 screening finding with its recorded status and historical location (commit 9768913), mapped by enclosing symbol (Python `ast` line ranges, C# brace matching) to a current location. |
| `screening-attachments.json` | The integrator's reviewed mapping of each finding to the rows, on both sides, whose behaviour it affects, with a reason; drafted by a Claude subagent from the finding text and the code, spot-checked by the integrator. |
| `correspondence.json` | The integrator's recorded row correspondences between the two inventories, each with a reason; drafted per shard by Claude subagents, spot-checked by the integrator. |
| `merge_inventories.py`, `merged.json`, `merged.md` | The deterministic comparison. |

## Scope and format

- Source scope (owner): root `core.py`, `session.py`, `log_io.py`, `server.py`, `engine_process.py`, plus `native/` and `work/experiments/magic600-04/`; the root helpers the engine imports (`session_lock.py`, `enhanced.py`, `grips.py`, `mpult_log.py`) are added. Every tracked file in the scope belongs to exactly one shard or is excluded with a reason (`check_inventory.py --coverage`).
- A shard lists the functions whose entry points are defined in its own files. A layer that only forwards to another shard names that shard's entry point in `depends_on`.
- Row fields: `id`, `name`, `behaviour`, `entry_points` (qualified by context, for example `key Workspace: Q` and `key Filter: Q` are different entry points), `evidence` (`path:line`), `depends_on`, `evidence_kind` (`source`, `source+test` with the test named in `notes`, or `inferred` with the reason in `notes`), `reads`, `writes`, `engine_call`, optional `notes`.

## Sequence

1. The bootstrap candidate (this folder's checker, tests, `shards.json` and the six packets) is reviewed before it is committed; the Codex shards start from that commit.
2. Six Codex shards run concurrently (`--kind implement`). While they run, nothing in the repository is committed, tagged, fetched or pushed.
3. In parallel, six Claude subagents write `claude-source-s1.json` to `-s6.json` outside the repository, each told not to read any `bottom-up-*`, `top-down*`, census or screening file. Each file passes the format check and its digest goes into `sealed.json` before any Codex file is opened.
4. The Codex patches are applied unchanged (format only) and their digests are added to `sealed.json`.
5. After sealing, `census.py` lists the entry points it can find mechanically. For each inventory, the entry points it lacks are reported as possible omissions. This catches what both sides missed, which a comparison of the two alone cannot.
6. Matching. Shared evidence locations and shared entry-point strings only **propose** candidate pairs; they never decide a match, because adjacent lines can hold different actions (Undo and Redo), one key can mean different things in different banks, and one route can serve several actions. The integrator records every correspondence (one-to-one, one-to-many, many-to-many) with a reason in `correspondence.json`; an unrecorded candidate stays unmatched. Rows keep namespaced ids (`codex:S1-3`, `claude:S1-3`). For a matched pair, differing behaviour, data or engine-call fields are listed, not reconciled.
7. Screening findings. Coordinates in [screening-findings](../../0.4.1/screening-findings.md) describe commit 9768913, and several findings are fixed since. `screening-map.json` keeps each finding's status (fixed, open, needs verification, rejected) and historical location, and maps it by symbol to a current file and line range. Line proximity cannot tell which user-visible function a defect affects (one dispatcher line serves every command), so a finding attaches only to the rows recorded in `screening-attachments.json`; rows whose evidence falls in the mapped range are reported as `line_candidates`, a lead only.
8. `merge_inventories.py` writes `merged.json` and `merged.md`: matched rows with their field differences, rows only one side found, census omissions per side, and the screening attachments. Running it twice gives byte-identical output.
9. One Codex review of the finished candidate (including the correspondence), at most two scoped verification rounds, then merge under `AGENTS.md` "Merging".

## Result notes

- Counts, omissions, attachments and unmatched rows: [merged.md](merged.md).
- Candidate pairs and correspondences are only proposed and recorded **within a shard**. Rows of different shards describe different layers (a native button and the adapter action it sends), so a key they share is a dependency, not a match.
- The census covers command IDs, routes, adapter actions and CLI options only. Buttons, menus, key bindings and mouse gestures are compared row by row, not against the census.
- Screening attachments are reviewed by behaviour, not derived from line ranges. A finding without a source location is attached only where its text names the affected function. In `screening-map.json`, a symbol longer than 80 lines is narrowed to a window of 15 lines around the historical offset (`narrowed`); that range only produces `line_candidates`.
- Matched rows list the behaviour texts of both sides when they differ; independent authors almost always word them differently, so the reader compares them, the script does not.
- Codex shard s2 ran twice. The first run was invalid: an implementation call started from another checkout at the same time created a Codex branch that this checkout's wrapper did not own. The rerun started from the same commit, after the Claude files were sealed, and its patch was applied unchanged. Lesson: run concurrent implementation calls from one checkout only.
- Claude rows are coarser in shards 6 and 4 (one row may group several commands); Codex lists library methods as rows in shard 2. These are recorded as one-to-many or unmatched, never rewritten.

## Evidence

Source only (Linux cloud session). No Windows, native-host or input-device behaviour is claimed beyond what the source says; rows reached only through the retained MPUlt runtime are `inferred`.

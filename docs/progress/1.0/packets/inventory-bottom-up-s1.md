# Packet: stage 2.1 source inventory, shard S1 (Engine and HTTP API (root modules))

The six shards S1 to S6 run concurrently:
`python tools/agents/codex_review.py --kind implement --model gpt-6.1-sol --effort max --packet docs/progress/1.0/packets/inventory-bottom-up-s1.md`
Then the integrator reviews `changes.patch`, applies it unchanged (format only; content is not edited) and runs `--cleanup <call-id>`.

## 1. Goal and acceptance
- Goal: list every user-reachable function of the 0.4 program whose entry point is defined in this shard's files (HTTP routes, session operations (preview, commit, undo, redo, reset), import and export, logs, protection, checkpoints, engine start and stop), found from the source code alone, so that stage 2.2 can later judge each one. An independent inventory of the same source code, written separately by Claude, is compared with this one afterwards; their matches and differences are listed. A document-based list exists only as a third reference and is not part of this comparison.
- Acceptance: `docs/progress/1.0/inventory/bottom-up-s1.json` holds one row per user-reachable function. Every command ID, key binding, button, menu item, window and HTTP route defined in this shard's files appears in exactly one row's `entry_points` or in `internal_only`. The contract's acceptance check passes.
- Non-goals: judging value (no keep, redesign, delete or automate), proposing 1.0 designs, changing any source file.

## 2. Actual problem and reproduction
- 1.0 does not inherit the 0.4 runtime or its UI (owner decisions of 29 and 30 September 2026), so it needs a complete list of what 0.4 does. No complete source-derived list exists.

## 3. Environment and versions
- Base commit: the committed HEAD this worktree was created from. Evidence kind: source (and tests, where a test exercises the function).

## 4. Necessary source and evidence
- This shard owns exactly these files (`docs/progress/1.0/inventory/shards.json`; every file in the owner's source scope belongs to exactly one shard):
  - `core.py`
  - `engine_process.py`
  - `enhanced.py`
  - `grips.py`
  - `log_io.py`
  - `mpult_log.py`
  - `server.py`
  - `session.py`
  - `session_lock.py`
- A function whose entry point is in this shard but whose work is done in another shard's files is listed here, with that other entry point (a route, command ID or method) in `depends_on`. Do not list entry points defined in other shards' files.
- Tests under `tests/` and `work/experiments/magic600-04/tests/` may be read to confirm behaviour.

## 5. Attempts so far
- The 0.4.1 screening (`docs/progress/0.4.1/packets/`) read the same code for defects, not for an inventory.

## 6. Constraints and owned files
- Write only `docs/progress/1.0/inventory/bottom-up-s1.json`. Change no other file.
- Never open or reference a personal session or user data directory.
- Independence: do not read `docs/progress/1.0/inventory/top-down.json`, `top-down.md`, `build_top_down.py`, any `claude-*` or `bottom-up-*` file other than your own, the user documents they cite, or `docs/progress/0.4.1/screening-findings.md`. This inventory comes from the code alone.
- JSON format (checked by `docs/progress/1.0/inventory/check_inventory.py`): `{"shard": "s1", "functions": [...], "internal_only": [...], "unclear": [...]}` and no other top-level keys. Each function row has exactly these fields, plus an optional non-empty `notes` string:
  - `id`: `S1-<number>`, unique;
  - `name`: a short name;
  - `behaviour`: what the user sees or gets, in one or two sentences;
  - `entry_points`: non-empty list of strings, each qualified by its context: `key <bank or window>: <key>`, `button <window>: <label>`, `menu <window>: <path>`, `command <id>`, `route <METHOD> <path>` (with the action field when one route serves several actions), `window <name>`, `cli <program> <option>`;
  - `evidence`: non-empty list of repository-relative `path:line` (the line where the behaviour or the entry point is implemented);
  - `depends_on`: list of other rows' ids, other shards' entry points or module-level functions it needs (may be empty);
  - `evidence_kind`: `source` (read in code), `source+test` (also exercised by a test; `notes` names the test file path) or `inferred` (indirect; `notes` explains);
  - `reads`, `writes`: non-empty lists drawn from state, protection, journal, checkpoints, workspace, preferences, files, none;
  - `engine_call`: the engine method or route it reaches, or null.
  `internal_only` and `unclear` items have exactly `name`, a non-empty `evidence` list in the same `path:line` form, and an optional `reason`.
- Check your file with `python docs/progress/1.0/inventory/check_inventory.py docs/progress/1.0/inventory/bottom-up-s1.json` before you finish.

```implement-contract
{"allowed_files": ["docs/progress/1.0/inventory/bottom-up-s1.json"], "acceptance_check": ["python", "docs/progress/1.0/inventory/check_inventory.py", "docs/progress/1.0/inventory/bottom-up-s1.json"], "stop_condition": "docs/progress/1.0/inventory/bottom-up-s1.json lists every user-reachable function whose entry point is defined in shard s1's files and the acceptance check passes"}
```

## 7. Required return format
- The JSON file above; the final message lists the row count, the `internal_only` and `unclear` counts and open points.

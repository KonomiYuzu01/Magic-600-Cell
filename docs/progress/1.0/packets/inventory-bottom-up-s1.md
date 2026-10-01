# Packet: stage 2.1 bottom-up inventory, shard S1 (engine and HTTP)

Run from the local session; the five shards S1 to S5 run concurrently:
`python tools/agents/codex_review.py --kind implement --model gpt-6.1-sol --effort max --packet docs/progress/1.0/packets/inventory-bottom-up-s1.md`
Then review `changes.patch`, apply it, and run `--cleanup <call-id>`.

## 1. Goal and acceptance
- Goal: list every function of the 0.4 program in this shard (routes, session operations, import and export, protection, checkpoints), found from the code alone, so stage 2.2 can give each one keep, redesign, delete or automate. A second, top-down inventory is written independently from the user documents; the two are merged and their differences listed.
- Acceptance: `docs/progress/1.0/inventory/bottom-up-s1.json` holds one row per user-reachable function, and every command ID, key binding, button, menu item and HTTP route in this shard appears in exactly one row or in `internal_only`. The contract's acceptance check passes.
- Non-goals: judging value, proposing 1.0 designs, changing any source file.

## 2. Actual problem and reproduction
- 1.0 does not inherit the 0.4 runtime or its UI (owner decisions of 30 September 2026), so it needs a complete list of what 0.4 does, judged by purpose. No complete list exists.

## 3. Environment and versions
- Base commit: current `main`. Evidence kind: source.

## 4. Necessary source and evidence
- Shard sources: `server.py`, `session.py`, `core.py`, `enhanced.py`, `grips.py`, `log_io.py`, `mpult_log.py`.
- Other shards cover the rest; reference a function in another shard by its entry point instead of listing it again.

## 5. Attempts so far
- The 0.4.1 screening (`docs/progress/0.4.1/packets/`) read the same code for defects, not for an inventory.

## 6. Constraints and owned files
- Write only `docs/progress/1.0/inventory/bottom-up-s1.json`. Change no other file.
- Never open or reference a personal session or user data directory.
- Do not read `docs/progress/1.0/inventory/top-down.json`, `top-down.md` or `build_top_down.py`, or the user documents they cite: this inventory comes from the code alone, so the merge can compare two independent lists.
- JSON format: `{"shard": "s1", "functions": [...], "internal_only": [...], "unclear": [...]}`. Each function has `id` (`S1-<n>`), `name`, `entry_points` (list of command ID, key, button, menu or route strings), `reads`, `writes` (lists drawn from: state, protection, journal, checkpoints, workspace, preferences, files, none), `engine_call` (string or null) and `evidence` (list of `path:line`). `unclear` lists items that could not be classified, with evidence.

```implement-contract
{"allowed_files": ["docs/progress/1.0/inventory/bottom-up-s1.json"], "acceptance_check": ["python", "-c", "import json;d=json.load(open('docs/progress/1.0/inventory/bottom-up-s1.json',encoding='utf-8'));assert d['shard']=='s1' and isinstance(d['functions'],list) and d['functions'] and all(set(f)>={'id','name','entry_points','reads','writes','engine_call','evidence'} for f in d['functions'])"], "stop_condition": "docs/progress/1.0/inventory/bottom-up-s1.json lists every user-reachable function of shard s1 and the acceptance check passes"}
```

## 7. Required return format
- The JSON file above; the final message lists the row count, the `unclear` count and open points.

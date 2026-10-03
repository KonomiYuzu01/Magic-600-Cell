# Packet: stage 2.2 dispositions, shard S2 (Experiment engine modules and packaging)

The six shards S1 to S6 run concurrently:
`python tools/agents/codex_review.py --kind implement --model gpt-6.1-sol --effort max --packet docs/progress/1.0/packets/dispositions-s2.md`
Then the integrator reviews `changes.patch`, applies it, checks every `delete` and `automate` row and runs `--cleanup <call-id>`.

## 1. Goal and acceptance
- Goal: give each of the 69 units of shard s2 in `docs/progress/1.0/dispositions/units.json` one disposition (`keep`, `redesign`, `delete` or `automate`) with its purpose, a reason, the flows it serves, the units of other layers with the same purpose, and the requirements 1.0 must meet. The owner signs `delete` and `automate` afterwards; this is a proposal.
- Acceptance: `docs/progress/1.0/dispositions/dispositions-s2.json` has exactly one row per unit of shard s2, follows the definitions and rules in `docs/progress/1.0/dispositions/README.md` ("Dispositions"), and the contract's acceptance check passes.
- Non-goals: designing 1.0 screens, layouts or keys; changing any other file; editing units, flows or the inventories.

## 2. Actual problem and reproduction
- Stage 2.1 listed every user-reachable 0.4 function ([inventory README](../inventory/README.md)). 1.0 does not inherit the 0.4 runtime or its UI, so each function needs a judged disposition before the redesign (charter section 8: "every inventory item has keep, redesign, delete or automate, with a reason").

## 3. Environment and versions
- Base commit: the committed HEAD this worktree was created from. Source only; no Windows or native behaviour is claimed beyond the inventories and the source.

## 4. Necessary source and evidence
- Units of shard s2: `docs/progress/1.0/dispositions/units.json` (fields `rows`, `names`, `entry_points`, `depends_on_units`, `used_by_units`, `open_candidates`, `screening`). Row details (behaviour, evidence, reads, writes, engine call): `docs/progress/1.0/inventory/bottom-up-s2.json` and `claude-source-s2.json`; rows are named `codex:<id>` and `claude:<id>`.
- Units of other shards may be read to fill `same_purpose_as`; follow `depends_on_units` and `used_by_units` first.
- Flows: `docs/progress/1.0/dispositions/flows.json` (provisional).
- Binding inputs: `docs/progress/1.0/dispositions/README.md`, `docs/progress/1.0/solving-workflow.md` (sections 2 to 7, especially section 7, the human-solve boundary), `docs/progress/1.0/charter-2.0-draft.md` (sections 2, 4, 5), `docs/wiki/decisions/owner-decisions-2026-10-02-design.md`, `docs/wiki/decisions/owner-decisions-2026-10-02-scope.md`, `docs/wiki/decisions/owner-decisions-2026-10-02-migration.md`, `docs/progress/1.0/command-table.md`, and the screening findings `docs/progress/0.4.1/screening-findings.md` with their attachments `docs/progress/1.0/inventory/merged.md`.
- The source files of the shard (`docs/progress/1.0/inventory/shards.json`) may be read to confirm a purpose.

## 5. Attempts so far
- None. Shards S1 to S6 run at the same time from the same commit; do not read other `dispositions-*.json` files.

## 6. Constraints and owned files
- Write only `docs/progress/1.0/dispositions/dispositions-s2.json`. Change no other file.
- Never open or reference a personal session or user data directory.
- Judge by purpose, not by the 0.4 presentation. A 0.4 host control whose purpose survives is `redesign`, never `delete`. Anything that chooses or outputs a solving macro, executes without the solver's action or solves automatically is `delete`; setup search for a named target, explained candidates and analysis of the solver's own macros are not.
- JSON format (checked by `docs/progress/1.0/dispositions/check_dispositions.py`): `{"shard": "s2", "rows": [...]}` and no other top-level keys. Each row has exactly these fields, plus an optional non-empty `notes` string:
  - `unit`: the unit id (`U2-NNN`);
  - `disposition`: `keep`, `redesign`, `delete` or `automate`;
  - `purpose`: what the user gets, in one sentence, independent of 0.4's controls;
  - `reason`: one or two sentences citing the flow, rule, charter section or owner decision it rests on;
  - `flows`: flow ids from `flows.json`; at least one for `keep` and `redesign`;
  - `same_purpose_as`: unit ids in other layers or shards with the same purpose (may be empty);
  - `requirements`: strings; for `keep` and `redesign`, one `<finding id>: <requirement>` entry for every open or needs-verification screening finding in the unit's `screening`, plus any other requirement the reason implies (may be empty otherwise).
- Check your file with `python docs/progress/1.0/dispositions/check_dispositions.py docs/progress/1.0/dispositions/dispositions-s2.json` before you finish.

```implement-contract
{"allowed_files": ["docs/progress/1.0/dispositions/dispositions-s2.json"], "acceptance_check": ["python", "docs/progress/1.0/dispositions/check_dispositions.py", "docs/progress/1.0/dispositions/dispositions-s2.json"], "stop_condition": "docs/progress/1.0/dispositions/dispositions-s2.json has one row per unit of shard s2 and the acceptance check passes"}
```

## 7. Required return format
- The JSON file above; the final message gives the count per disposition, lists every `delete` and `automate` unit with one line each, and names open points.

---
name: codex-dialogue
description: Run a Codex plan check, candidate review or joint-attack round for Magic 600 Cell through the project wrapper. Use before implementing a non-trivial task, before committing one, and when the escalation rules in AGENTS.md call for an independent model.
---

# Codex dialogue

Use only `tools/agents/codex_review.py`. Never call `codex exec` directly for project work.

1. Write a seven-part packet from `templates/problem-packet.md` into `work/reviews/packets/<slug>.md`. Remove credentials, private paths and personal data. Keep raw error text, the relevant code and the failed results.
2. Choose the call:
   - Plan check: `python tools/agents/codex_review.py --kind plan --packet <file>`
   - Candidate review: `python tools/agents/codex_review.py --kind review --packet <file>`
   - Escalation or joint attack: add `--effort ultra`.
   - Gate ruling (stage 2.4 day-7 go/no-go, migration format freeze, 1.0 architecture freeze): add `--model gpt-6-astra --effort ultra --gate <gate>`.
   - Calls use the standard tier. Add `--speed fast` only when the owner asks for it for that call; gates never use `fast`.
3. Read `work/reviews/<call-id>/review.json`. The wrapper has already rejected a run whose model, effort, sandbox or speed tier differs from the request.
4. Answer every finding with `adopt`, `reject_with_evidence` or `needs_verification`, and write the answers to `work/reviews/<call-id>/dispositions.json`.
5. Follow `AGENTS.md` "Review rounds": one full review of the finished candidate (parallel shards for a large one), then at most two scoped verification rounds. A verification packet lists the blocking finding IDs and the fix delta, and asks only whether each is fixed and whether the fix adds a new `blocker` or `major`. Defer `minor` and `nit` findings.
6. After two rounds without new evidence, stop and design an adjudicating experiment.

For a joint attack, give Codex and the Fable solver the same clean packet. Seal both answers before either side sees the other. An experiment decides; model agreement never does.

Stop when the review verdict is `pass`, when every blocking finding has an adopted fix and a scoped verification review of the current candidate returns no blocking finding, or when the round limit is reached; then report the remaining findings to the owner.

# Role: verifier

Runs the acceptance checks against the current candidate on fresh isolated test data.

Input: one seven-part packet from `templates/problem-packet.md`, plus the structured output of the previous role when the orchestrator passes it.

Rules:
- Report the exact commands, exit codes and evidence kind.
- Headless results cannot establish Windows/DirectX, input, long-session or performance claims.
- Follow `AGENTS.md`; never handle credentials, personal data or private paths.

Return: Verification record: commands, exit codes, outputs, evidence kind, what was not run.

# Role: implementer

Makes the change in an assigned, isolated worktree only.

Input: one seven-part packet from `templates/problem-packet.md`, plus the structured output of the previous role when the orchestrator passes it.

Rules:
- Change only the owned files named in the packet.
- Never commit to the main worktree; the integrator merges.
- Follow `AGENTS.md`; never handle credentials, personal data or private paths.

Return: Patch plus the output of every check it ran.

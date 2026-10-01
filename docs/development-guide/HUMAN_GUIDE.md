# Magic 600 Cell development guide

This guide is for the owner and any person working on the project. Agents follow `AGENTS.md`, `CLAUDE.md` and `AGENT_BRIEFING.md`; this guide explains the same system in plain terms and tells you what to do yourself.

## 1. What this repository is now

- **0.4** is the released version (19 September 2026). Its package and provenance are frozen.
- **There is no 0.4.1 release** (your decision, 1 October 2026). The 0.4 line ends with 0.4. What 0.4.1 built stays: the local development workbench (a desktop monitor that lists every session with a short explanation, lets you watch any of them live and jump to every tool they use, lets you send notes to a running Claude session, and keeps the project progress in view), the reproducible build identity, the test harness and the screening findings, which become 1.0 requirements.
- **Stage 2 runs within 20 days**, from 1 October to 20 October 2026, preparation included. The day-by-day plan is in `docs/progress/1.0/stage-2-experiment-protocol.md` section 5.
- **1.0** is a new program. It keeps the mathematics, the protection rules and the model identity, but not the 0.4 runtime. Direct3D 12 is the base of the new renderer; Godot and Blender are in scope. Its first phase, stage 2, inventories 0.4, tears it down and redesigns it, with a timed experiment window.
- The decisions behind this are recorded in `docs/wiki/decisions/owner-decisions-2026-09-29.md`.

## 2. Your controls

**What the agents do on their own**
- Claude Opus plans, edits, runs checks and commits. It is the only agent that changes the main worktree.
- Codex reviews every non-trivial plan and every candidate change, read-only, at `max` effort. GPT-6.1 Sol handles routine work. Codex Astra reviews critical, behaviour, contract and design changes, handles escalations and audits, and gives the three final rulings: the stage 2.4 day-7 go/no-go, the migration format freeze and the 1.0 architecture freeze.
- Each candidate gets one full review and at most two short verification rounds; remaining blockers come to you as a decision. Quick checks run on the fast tier.
- Codex also writes code alongside Claude, each packet in its own sandboxed worktree; Claude reviews and merges, and no one reviews their own code. On Windows without administrator setup the sandbox blocks file writes outside the worktree but not network access.
- Fable, a Claude subagent, joins only when a problem resists repeated attempts.
- Missing tools from the approved list are installed automatically once you have approved the installer (section 3).

**What they must ask you** (full list in `AGENTS.md`, "Ask the owner")
- administrator rights, drivers, Windows features, security or antivirus settings;
- anything outside the tool allowlist, a checksum mismatch, a large download;
- new costs, any API-team run (including runs on OpenAI models), raising an API budget ceiling;
- uploading data or connecting external services;
- model identity changes, accepting design records, UX and release sign-off, deleting or automating a feature.

**Commands only you run**
- `/codex:review` and `/codex:adversarial-review` in Claude Code: an on-demand second opinion from the Codex plugin.
- `python tools/toolchain/bootstrap.py approve`: approves automatic installation for the current installer revision (section 3).
- Leave the Codex plugin's stop-time review gate off unless you are watching; it can loop and use up quota.

**Budgets**
- Paid API use is capped cumulatively at USD 100, then 150, then 300. Agents stop and ask before a ceiling is raised. Subscription usage never switches to paid API automatically.
- Your Claude plan is Max 5x. Agents keep Opus for planning and integration, hand broad searches to subagents, and give heavy reviews to Codex (which uses your ChatGPT plan).

**Stopping a loop**
- Press Esc in Claude Code to interrupt. The project's Stop hook blocks a stop at most twice per session and then lets the agent stop honestly as "inconclusive".

**Watching and joining the work**
- The development workbench shows every session, what it is doing and why, a live view, the Codex calls and the progress board: `tools\.venv\workbench\Scripts\python.exe tools\workbench\app.py` (add `--compact` for a small always-on-top view). Install it once with `python tools/toolchain/bootstrap.py install pyside6`.
- A note typed there reaches Claude at its next tool step, and Claude answers it; a note cannot replace a command that only you may run. In a terminal session, the status line under the prompt shows the same progress line as the compact view. Details: `docs/wiki/components/workbench.md`.
- "Needs attention" lists sessions waiting for you, failed turns, failed Codex calls and runs, and stalls. The Runs tab starts only the checks listed in `tools/workbench/runs.json`, and the Launch tab starts a new Claude session or a Codex call from a packet; nothing starts unless you click.

## 3. First-time setup on Windows

Prerequisites you install yourself: 64-bit CPython 3.11 or later, Git for Windows, Node.js 22 or later, and the Codex CLI (`npm install -g @openai/codex@0.159.0`).

1. Open a terminal in the repository and run:
   ```powershell
   python tools/toolchain/bootstrap.py doctor
   ```
   It reports Python, every tool in the allowlist, the approval state, the Codex login and the skills check.
2. Sign in to Codex and confirm the models:
   ```powershell
   codex login
   codex debug models
   ```
   `gpt-6.1-sol` and `gpt-6-astra` should both be listed. Optionally set the same defaults in `%USERPROFILE%\.codex\config.toml`: `model = "gpt-6.1-sol"` and `model_reasoning_effort = "max"`. The review wrapper passes these explicitly anyway.
3. Pin the Windows packages you want (this records the exact version and installer hash from winget):
   ```powershell
   python tools/toolchain/bootstrap.py pin drawio
   python tools/toolchain/bootstrap.py pin typst
   ```
4. Review the change to `tools/toolchain.lock.json`, then approve the installer revision. The command asks you to type a confirmation, so only you can run it:
   ```powershell
   python tools/toolchain/bootstrap.py approve
   ```
   From now on, agents install allowlisted tools without asking. Any later change to the installer or a lockfile returns installation to ask-first until you approve again.
5. Install the planning profile (or let the agents do it when they need a tool):
   ```powershell
   python tools/toolchain/bootstrap.py install --profile planning
   ```
6. Start Claude Code in the repository and trust the folder. The session summary appears at the start of each session.
7. Optional: open `docs/wiki` as an Obsidian vault to browse the wiki. The `.obsidian/` folder is ignored by Git.

## 4. Daily loop with Claude and Codex

1. Start a session. Claude reads the session summary, the briefing and the wiki index.
2. Give the task in your own words. Say what "done" means if you can.
3. For non-trivial work Claude writes a problem packet and asks Codex for a plan check before changing code. You see the verdict and the findings.
4. Claude implements in small steps and runs the checks for the changed area.
5. Codex reviews the current candidate. Claude answers every finding: **adopt**, **reject with evidence** or **needs verification**.
6. Claude updates the wiki and commits. Critical files are committed only after a valid review or your explicit exception.

When something keeps failing: after three real attempts Codex diagnoses independently; after two more, Fable is called; the hardest problems get a "joint attack" where Codex and Fable analyse the same packet separately and an experiment decides.

## 5. The knowledge wiki

- `docs/wiki/` holds the project's engineering memory: decisions, workflows, evidence, open questions and summaries of review dialogues. Start at `index.md`; history is in `log.md`; the rules are in `SCHEMA.md`.
- Say "ingest X" to have a source read, discussed with you and filed into the wiki. Ask questions; good answers with evidence are filed back.
- Private material (chat exports, raw model dialogues, machine diagnostics) stays in `work/loop-memory/`, which is never committed.
- Every claim names its kind of evidence. A Windows test pass is never presented as a performance result.
- Check the wiki with `python tools/wiki/lint.py`.

## 6. Stage handbook

| Stage | Goal and exit | Tools | Codex involvement |
| --- | --- | --- | --- |
| 2.0 | Charter, authority record, first design records | Markdown, Mermaid | Review of charter and records |
| 2.1 | Two independent inventories (commands, physical input, API, windows, persistence) | marimo notebooks | Codex runs the bottom-up inventory |
| 2.2 | A disposition for every item | draw.io card sort | Review of risky dispositions |
| 2.3 | Command model, keyboard model, state charts, the three hardest screens, design tokens | Mermaid, draw.io, Blender for look development | Review of the hardest screens and input model |
| 2.4 | 12 days of experiments (days 3 to 14); bare Direct3D 12 interop probe first; day-7 go/no-go | `renderer-spike` and `native-performance` (PresentMon) profiles: Godot 4.7 (.NET), Qt via aqtinstall, CMake, Ninja, RenderDoc, PIX, FLIP | Hypotheses before each experiment; Astra rules on day 7. Selection gate: full detail at a stable 30 fps on your RTX 4070 Laptop GPU (`docs/wiki/decisions/renderer-candidates.md`) |
| 2.5 | Frozen 1.0 architecture | Typst for the frozen document | Astra rules on the architecture freeze |
| Later stages | Build, media and release | `media` and `release` profiles | Review at each gate |

## 7. marimo notebooks

- marimo is installed into `tools/.venv/planning` (`install marimo`). Notebooks are plain Python files, so they diff and review like code.
- Stage 2 uses three notebooks: the inventory, the disposition board and the experiment ledger.
- To pair with an agent on a live notebook, start it with `marimo edit <notebook>.py` and use the `marimo-pair` skill. First use it on a synthetic notebook, and keep one writer per notebook. `marimo-pair` needs `bash`, `curl` and `jq` (Git Bash provides the first two; `jq` is in the allowlist).

## 8. Non-software outputs

Every formal diagram, chart, PDF, slide deck, image, audio or video is produced with a local tool from a source file that stays in the repository, together with the tool version and the command used.

| Output | Source | Tool |
| --- | --- | --- |
| Architecture and flow diagrams, state charts | `.mmd` | Mermaid CLI (`tools/node/mermaid-cli`) |
| Wireframes, card sorts | `.drawio` | draw.io Desktop |
| Data charts, dashboards | notebook or script | marimo, matplotlib or altair |
| PDF documents | `.typ` | Typst |
| Slides | Marp Markdown | Marp CLI (`tools/node/marp-cli`) |
| 3D references and renders | `.blend` plus script | Blender |
| Video and sound | frozen inputs plus script | FFmpeg, Audacity |

Chat replies, plain Markdown and code comments are exempt. Online generators, hand-drawn ASCII, AI-generated images and edited screenshots are never the source of a formal output.

## 9. Evidence and publishing rules

- Four kinds of evidence are kept apart: source/fixture, synthetic geometry, actual Windows/DirectX, and performance. Results from a cloud or Linux session never prove Windows, input, long-session or performance behaviour.
- The public repository never contains user databases, personal logs, credentials, private paths, screenshots or raw diagnostics.
- Andrey Astrelin's MPUlt credit and all upstream licences are preserved. Microsoft Managed DirectX DLLs are never redistributed.

## 10. Moving your data from 0.4 to 1.0

With no 0.4.1 release, the way your sessions move to 1.0 is chosen in stage 2.0. The rules stay: it works on a copy taken under the session lock and never changes your original session folder, and 1.0 checks the data before importing it. You will be able to keep 0.4 installed side by side until you are satisfied.

## 11. Troubleshooting

- **Codex used a different model.** The wrapper rejects any run whose reported model, effort or sandbox differs from the request, so an invalid run is reported rather than used. Check `codex debug models` and your Codex login.
- **Codex is not signed in.** Run `codex login`. The session summary shows `Codex login: not-ready` until you do.
- **A hook fails or reports inconclusive.** Run `python tools/toolchain/bootstrap.py doctor`. Hooks need 64-bit CPython on `PATH` as `python`, and a Claude Code version that supports hook commands with separate arguments.
- **An install is refused.** The entry may be unverified (run `pin` on Windows), the approval may be stale after a lockfile change (run `approve`), or a checksum did not match (installation stays blocked until the source is verified again).
- **Windows specifics.** winget pins, hash checks and installs all select the user-scope x64 installer and skip implicit dependencies; a tool that needs a dependency gets that dependency as its own allowlist entry. Tools that need administrator rights are never installed automatically: Visual Studio Build Tools, Windows SDK, Windows Performance Analyzer, Windows Sandbox, Nsight Graphics, DaVinci Resolve and Inno Setup are installed by you when a stage needs them.

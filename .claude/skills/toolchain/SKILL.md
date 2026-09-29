---
name: toolchain
description: Check, install and diagnose the pinned Magic 600 Cell development tools and third-party skills. Use when a required tool or skill is missing, when a probe fails, or before starting a stage that needs a new tool profile.
---

# Toolchain

The allowlist is `tools/toolchain.lock.json`. Only entries with `"status": "verified"` and `"installable": true` can be installed.

- Quick check: `python tools/toolchain/bootstrap.py check`
- Full report: `python tools/toolchain/bootstrap.py doctor`
- Install one tool: `python tools/toolchain/bootstrap.py install <id>`
- Install a profile: `python tools/toolchain/bootstrap.py install --profile <profile>`
- Install a pinned third-party skill: `python tools/toolchain/bootstrap.py install-skill <id>`
- Regenerate Claude skill copies: `python tools/skills/sync.py`; check them with `python tools/skills/sync.py --check`

When a tool is missing during development, install it with the commands above, then run its probe. Do not install from hooks or from normal application startup.

Stop and ask the owner instead of installing when the entry is unverified, the source is outside the allowlist, a checksum or signature does not match, or the step needs administrator rights, new costs, data uploads, a large download or a change to security settings. Report a missing tool; never substitute another version or fabricate its output.

Pinning a winget entry (`python tools/toolchain/bootstrap.py pin <id>`) changes the lockfile. That change returns installation to ask-first until the owner approves the new revision.

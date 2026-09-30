# Review of two unmerged branches (2026-09-30)

Both branches fork from `main` at 9 September 2026, before 0.4. Neither should be merged as is.

## `codex/runtime-dependencies` (1 commit, 9 new files under `runtime-prerequisites/` plus a CI workflow)

A separate "runtime prerequisites" companion ZIP. It contains:
- `Install-DotNet35.cmd`, which enables NetFx3 through DISM;
- `Install-DirectX.cmd`, which starts Microsoft's unmodified `directx_Jun2010_redist.exe`;
- a C# `Check-Runtime` that reads the installed assemblies against the hash allowlist;
- `build.ps1`, which downloads the redist, checks its SHA-256 and Authenticode signature, and writes a manifest;
- a GitHub Actions workflow.

Worth taking:
- **README text.** The exact compatibility boundary (x86 .NET 4.x host, legacy CLR v2 chain, assembly identities, that the June 2010 runtime does not replace the DirectX 11/12 core) and the failure guidance. This belongs in `DIRECTX.md` / release notes for step 7.
- **`Check-Runtime.cs`.** A read-only prerequisite checker with no DLL copying, no GAC changes and no device creation. It can be reused as a user-facing diagnostic.

Needs an owner decision before use:
- **Redistributing Microsoft's installer.** The companion ZIP bundles `directx_Jun2010_redist.exe`. The rule forbids redistributing the Managed DirectX DLLs. Shipping Microsoft's unmodified redistributable installer is a separate question, which `DIRECTX.md` currently answers "no": link to Microsoft instead. Recommendation: keep linking and do not ship the installer.
- **The CI workflow.** It downloads from Microsoft in CI, which is new network and CI scope.

Recommendation: in step 7, port the README boundary text and optionally `Check-Runtime`. Then delete the branch.

## `codex/code-debug-performance` (1 commit)

It adds:
- `preparation.py`, a read-only orbit-first preparation service built on 0.3;
- `tests/test_preparation.py`;
- `docs/NEXT_UPDATE.md`;
- edits to `AGENTS.md`, `README.md`, `SOURCE_MANIFEST.json` and `DEVELOPMENT_LOG.md`.

The branch is superseded by 0.4. 0.4 shipped explicit Prepare / Macro / Cleanup operations, buffer and orientation tools, and position requirements (`work/experiments/magic600-04/position_requirements.py`, `work_intents.py`). Its `AGENTS.md` and `SOURCE_MANIFEST.json` edits conflict with the current rules and the frozen 0.4 manifest.

Worth keeping:
- **The product statement in `docs/NEXT_UPDATE.md`.** Its "Product purpose and priorities" section says the main human bottleneck is buffer preparation and protecting completed orbits, not executing the macro. It also defines the preparation-cycle unit. Both are direct input to 1.0 stage 2.1 (inventory) and 2.3 (redesign). Recommendation: when stage 2 starts, file those two sections as a wiki source note, via an owner-requested ingest.

Recommendation: after that note is filed, delete the branch. The code has no 0.4.1 use.

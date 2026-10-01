---
id: build-identity-v2
type: concept
status: verified
visibility: public
summary: How a native build of the retained 0.4 host is identified from 0.4.1 on, what the identity covers and what it cannot promise.
related: [build-reproducibility-decisions, owner-decisions-2026-09-29]
supersedes: []
claims:
  - {id: identity-payload, evidence_kind: source, path: work/experiments/magic600-04/build_identity.py, sha256: e1aca296cadbf0027f79c3ac4d7b1a0afa83ebf4e87540ad4bcc2ebadda7aeee, checked_at: 2026-10-01}
  - {id: sealed-recipe, evidence_kind: source, path: native/bootstrap.py, sha256: 669a7b2bab659320878b18d0b1f7034812b151899bb1fb92b9c3380536f07c9e, checked_at: 2026-09-30}
  - {id: receipt-checks, evidence_kind: source, path: work/experiments/magic600-04/native_launch.py, sha256: 26ab3b85c388e8b34400e25cdaf018af8599b3f4fda77da076d0e730508c3343, checked_at: 2026-10-01}
  - {id: packaging-consumer, evidence_kind: source, path: work/experiments/magic600-04/packaging/assemble.py, sha256: 8502cf0eeb44f21a647f37ab35824b1c4890c66089fd6f8d69ab1b7acab6b44a, checked_at: 2026-09-30}
  - {id: invariance-tests, evidence_kind: fixture, path: tests/test_build_identity.py, sha256: ac34dffa7f766cb3ebf5e45685062092d76c00371bdbca096b25b824ecc2647b, checked_at: 2026-10-01}
  - {id: byte-exact-checkout, evidence_kind: source, path: .gitattributes, sha256: 018ad2ea40527c9f02c1e384103567d79b00ab3c6b41c54ac67f79b5e2205ed1, checked_at: 2026-09-30}
  - {id: engine-environment, evidence_kind: source, path: tools/toolchain.lock.json, sha256: 1a181fd36116af5906edbf54f0d19f0683f7b37840d2192bafcd74437e4e1c6c, checked_at: 2026-09-30}
  - {id: continuity-0-4, evidence_kind: source, path: docs/RELEASE_0_4_CONTINUITY.json, sha256: a40c915dc56ff73c96d6ffb164a3264693f0e6faceddfeb96eaa7f4758d53759, checked_at: 2026-09-30}
  - {id: windows-startup-regression, evidence_kind: actual_windows_directx, checked_at: 2026-09-30}
  - {id: windows-harness-endgame, evidence_kind: actual_windows_directx, checked_at: 2026-09-30}
  - {id: clean-checkout-audit, evidence_kind: source, path: docs/progress/0.4.1/clean-checkout-audit.md, sha256: 0079ee2196adb693d2d6bfeae8a55cb2a6ac726f7e5ccfb89e95a1b839e704fc, checked_at: 2026-09-30}
---

# Build identity v2

A native build of the retained 0.4 host is identified by a SHA-256 over what the build is made from. The same inputs must give the same identity whatever the checkout location, the Git line-ending setting, the Windows build or edits to test-only files. From 0.4.1 on, every build receipt (`build.json`, `evidence_version: 2`) carries this identity. The build recipe is in `docs/DEVELOPMENT.md`; the choices behind it are in [build-reproducibility-decisions](../decisions/build-reproducibility-decisions.md).

## Why v1 was replaced

The 0.4 identity hashed the whole evidence manifest, which had three problems:
- it included the relative interpreter, NumPy and compiler paths, the platform string and whole harness files, so moving the checkout, a Windows update or a test edit changed it;
- it left out the retained runtime that the program starts with;
- it left out the compiler's implicit inputs.

A clean Windows clone could not even build, for two reasons:
- Git converted line endings, which broke the raw asset hashes;
- the harness file that the build hashed was never published.

## What the identity covers

The identity is the digest of one `identity` payload with four parts:
- **product**: the C# and Python sources, the CLR configuration, the model manifest and assets, and the retained runtime. That runtime is `MPUlt.exe`, its puzzle definitions, its distributed default settings and the DirectX loader. Files are keyed by repository path.
- **compile_recipe**: the flags and reference names of the sealed `csc` invocation. The invocation passes `/noconfig /nostdlib+`, and all 39 system references, `mscorlib` included, by full path from the compiler's own directory. It also passes the default Win32 manifest, runs in an empty directory and refuses `LIB`. The recipe is data, so harness text in the same scripts does not affect it.
- **compiler**: the banner, plus the hashes of the compiler files and of every reference assembly, recorded by role.
- **interpreter**: the implementation, exact version and bits, and the base binaries and environment launcher by role. For NumPy, it binds a digest of the wheel payload, verified file by file against the installed `RECORD`. It excludes installer-written files, bytecode and console-script wrappers, which embed their install path. Capture is refused when the imported `numpy` is not the installed distribution's own `numpy/__init__.py`, so a shadow on the import path cannot inherit its identity.

The receipt also records, outside the identity:
- the harness files: the launcher, the identity helper, the bootstrap, runtime inspection and DirectX loader scripts, and for the native harness its checks, cases and runner. `check_evidence` compares this list with the one the code declares, so a receipt that drops or adds a harness file is refused;
- the platform string;
- the artifacts;
- launch evidence such as the copied runtime and the user-editable settings.

## How it is checked

- `check_evidence` refuses receipts whose `evidence_version` is missing or does not match their sections. For a v2 receipt, it:
  - rehashes the identity;
  - checks the identity against the inventory that the code declares, not the receipt. That inventory is every product source, the model manifest and each asset it lists, the retained runtime, the current recipe, every compiler and interpreter role, and the NumPy payload. A receipt that drops or adds a binding is refused even when it rehashes;
  - rehashes every product, harness, artifact and runtime file. A path bound with two different hashes in different sections is a conflict, never an override;
  - re-resolves the compiler and interpreter and compares them role by role, counting a missing role as missing;
  - rechecks, at every phase, local tools that the harness uses, such as the recorder, by hash. Their paths are never written.
- A build is reused only from a valid v2 receipt for exactly the same identity payload.
- Runtime assemblies from outside the repository, such as a system DirectX installation, are bound by hash under the repository path of their copy. The origin path is never stored.
- Test builds bind their extra references, the DirectX assemblies, by role and hash. They recheck them after compiling and before running, and keep them out of the product identity.
- `packaging/assemble.py` accepts a v2 receipt only if the identity rehashes, binds the complete inventory, and every product file matches current bytes. The stored `after_build` status alone is not enough.
- The engine environment is the allowlisted `engine-python` entry: the owner-installed CPython 3.14.7 with hash-pinned NumPy 2.3.5, never a downloaded or uv-managed interpreter.

## What it cannot promise

- The executable is not bit-reproducible. The legacy compiler has no deterministic mode, so a build is reused only while its recorded executable hash still matches.
- The standard library is covered by the exact interpreter version and base binaries, not file by file.
- The 0.4 build itself cannot be recreated on later machines. Its toolchain (CPython 3.12.14, csc 4.8.9232.0, Windows 10) differs. `tools/provenance/continuity_04.py` instead proves the following:
  - the stored 0.4 payload still rehashes to its identity;
  - 88 of its 96 inputs are byte-identical in this repository;
  - the other 8 are listed 0.4.1 adaptations (the step 1 build identity and the screening fixes) whose release bytes remain in Git history.

## Acceptance of 0.4.1 step 1

Step 1 is done when all of these hold on a fresh clone of the merged branch. The items come from the clean-checkout audit (`docs/progress/0.4.1/clean-checkout-audit.md`) and the approved step 1 plan.
- `python tests/test_core.py`, `python tests/test_reference_maps.py`, `python tests/test_crash.py` and `python tests/test_engine_lifecycle.py` pass.
- Every other script under `tests/` runs without a missing-directory error, or is a harness tool that `docs/DEVELOPMENT.md` lists with its arguments.
- `git status --porcelain` is empty after those runs.
- The native build finds every harness file that it hashes, `run_postapproval.py` included.
- The build identity does not depend on the checkout path or the Git line-ending setting (`tests/test_build_identity.py`).
- `python work/experiments/magic600-04/print_identity.py` prints the source identity, the build identity when the tools resolve, and the harness inventory, without compiling.

## Evidence on the owner's Windows 11 machine (30 September 2026)

- A checkout under `* -text` held the committed bytes for all tracked files, and all model assets matched.
- The product built with identity `56039c362c0fd361…`. The identity was unchanged after harness-only edits and line-ending repairs, and the build then reused the recorded executable.
- The native harness compiled with its inputs bound (`run_postapproval.py --compile-only`).
- `print_identity.py` printed, without compiling, the build identity of a later `--build-only` receipt of the same tree (`ab12e78d8223dfa8…`) and exactly the 27 harness files, with their hashes, of a `--compile-only` receipt. The closing review of 1 October added `build_identity.py` and `native/directx_runtime.py` to the harness (29 files); that comparison was not repeated on Windows after the change.
- The WinForms startup regression passed its 19 checks (`native/bootstrap.py --self-test-only`). This is actual Windows evidence of startup only.
- The native harness then ran on the desktop with `--focus endgame`, on a synthetic legal fixture and a synthetic key route, not physical typing. The second run passed all 96 checks, with 4 skips and exit code 0. Its summary step then failed to decode `run.log`, which Windows writes in the console code page; `run_postapproval.py` now falls back to that code page.
- The first of those runs stopped with an access violation inside Managed DirectX `Device.Reset` during a window resize. Step 1 changed no product C# source, so this is recorded as an observation of the retained 0.4 host, not as a step 1 regression.
- Long sessions and performance were not exercised, and the full harness run without a focus is still pending.
- On 1 October, on the same machine, the closing candidate passed the NumPy backend checks under the engine environment (`test_core.py`, `test_reference_maps.py`, `test_crash.py` and the step 1 identity and B4-12 tests), and `tests/test_codex_implement.py` passed under the approved CPython 3.14.7. These are headless checks, not native evidence.

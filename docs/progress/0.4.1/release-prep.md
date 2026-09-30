# 0.4.1 release preparation (steps 5 and 7)

Prepared on 2026-09-30 from source inspection. It lists what the freeze candidate (step 5) and the release (step 7) must satisfy, so those steps do not start from zero. It authorizes nothing: release sign-off stays with the owner (`AGENTS.md` "Ask the owner").

## 1. Freeze-candidate criteria (step 5)

A candidate is frozen only when all of these hold on one commit:

- [ ] Step 1 done: reproducible build identity; clean-checkout findings closed or recorded (`clean-checkout-audit.md`); `tests/run_postapproval.py` gap resolved.
- [ ] Step 2 baseline recorded for the matching 0.4 source.
- [ ] Step 3 done: minimal exporter, `.c600migrate` schema and validator, core fixtures. The exporter copies the database under the session lock and never modifies the original session directory; it never opens a user database with `immutable=1`.
- [ ] Step 4: every adopted screening finding fixed or explicitly deferred with a reason; no open `blocker` or `major`.
- [ ] B4-12 closeout decided: each target (instant turn p95 at most 100 ms, structure and navigation p95 below 50 ms, full-detail 30 fps) is either met with step-6 evidence or recorded as not met with the measured numbers. 0.4.1 is the closing release of the 0.4 line; B4-12 is not deferred to 1.0.
- [ ] Mechanics unchanged: `assets/manifest.json` and model identity byte-identical to 0.4; all 259,800 sticker slots and 1,200 generators kept.
- [ ] Checks pass on the candidate (Windows): `tests/test_core.py`, `tests/test_reference_maps.py`, `tests/test_crash.py`, `tests/test_engine_lifecycle.py`, `tests/native/NativeHostRegression.cs`, `work/experiments/magic600-04/packaging/test_package_contract.py`.
- [ ] Astra full review of the candidate (at least two shards) with no open `blocker` or `major`.
- [ ] Development workbench: out of the public package (it is a local development tool).

## 2. Release checklist against the 0.4 package contract (step 7)

The 0.4 package contract lives in `work/experiments/magic600-04/packaging/` (`package_contract.py`, `assemble.py`, `launcher.py`, `check_package.py`, `Magic600Cell.spec`). Version strings are hard-coded; each needs a deliberate 0.4.1 value.

| Item | 0.4 location | 0.4.1 action |
|---|---|---|
| `VERSION`, `BUNDLE_NAME` | `package_contract.py:7-8` | `0.4.1`, `Magic600Cell-0.4.1-Windows-x64` |
| PyInstaller bundle name | `Magic600Cell.spec:26` | match `BUNDLE_NAME` |
| Windows file version resource | `assemble.py:191` (`filevers=(0,4,0,0)`, `FileVersion '0.4'`) | `(0,4,1,0)`, `'0.4.1'` |
| Manifest `layout_version` | `package_contract.py:39`, `assemble.py:204` | keep `1` unless the layout changes |
| Test fixture version | `test_package_contract.py:34` | follow `VERSION` |
| Default data directory | `launcher.py:186` (`%LOCALAPPDATA%/Magic600Cell/0.4`) | keep `0.4` (owner decision 2026-09-30); see [data-directory-plan](data-directory-plan.md) |
| Native host cache | `launcher.py:35,93,200` (`native-host-0.4`) | keep `native-host-0.4`: it holds the user-owned `MPUlt_settings.txt` |
| Release record | `RELEASE.json` | new version, hashes, `microsoft_directx_payload_included: false` |
| Provenance | `docs/RELEASE_0_4_PROVENANCE.json` (keys: `version`, `source_files`, `native`, `model_id`, `launcher_sha256`, `package_manifest_sha256`, `zip_sha256`, `reused_evidence`, `raw_recordings`, `startup_defaults`) | new `docs/RELEASE_0_4_1_PROVENANCE.json` with the same keys; the 0.4 file stays frozen |
| Release page | `docs/RELEASE_0_4.md` | new `docs/RELEASE_0_4_1.md` (skeleton in section 3) |
| Package changes | `CHANGES.md` | new section "Changes since 0.4" |
| Payload check | `inspect_payload` in `package_contract.py` | unchanged: no DirectX DLLs, databases, logs, `.pdb`, `.pyc`, sessions, diagnostics, `.env` |
| Required licence files | `check_package.py:333-335` | unchanged list, verified in the built ZIP |
| Assets | GitHub release: portable ZIP and `SHA256SUMS.txt` | same; 0.4 assets and tag unchanged |

**Data directory (decided 2026-09-30).** 0.4.1 keeps the 0.4 data directory and native cache; change list and test points in [data-directory-plan](data-directory-plan.md). Never modify or move the original 0.4 session directory.

Release evidence rules: publish only verified results for the matching source and build; separate source/fixture, synthetic, actual Windows/DirectX and performance evidence; headless runs cannot support Windows/DirectX, input, long-session or performance claims.

## 3. Release-notes skeleton (`docs/RELEASE_0_4_1.md`)

```markdown
# Magic 600 Cell 0.4.1

Magic 600 Cell 0.4.1 is the closing release of the 0.4 line: fixes, optimization and performance work on the 0.4 native workspace. The puzzle model is unchanged.

Download the Windows x64 portable ZIP, extract it into a fresh folder and run Magic600Cell.exe. Microsoft Managed DirectX remains an external prerequisite; see [DIRECTX.md](../DIRECTX.md).

## What changed
- <fixes from step 4, one line each, user-visible wording>

## Performance
<B4-12 result per target: met with the measured value, or not met with the measured value. State the machine, sample count and method (3 x 100 formal measurement).>

## Your data
0.4.1 uses the same data folder as 0.4. Your sessions, macros, keybindings and MPUlt view settings carry over.

## Source and package binding
The frozen source-file hashes, native inputs and ZIP hash are in [RELEASE_0_4_1_PROVENANCE.json](RELEASE_0_4_1_PROVENANCE.json).

Original puzzle simulator and renderer: Magic Puzzle Ultimate by Andrey Astrelin. See the included credits and third-party notices.

[Usage](../USAGE.md) · [Documentation index](README.md)
```

## 4. Credit and licence check (current source, 2026-09-30)

| Check | Status in source | Release action |
|---|---|---|
| Andrey Astrelin named as primary MPUlt author | Present: `CREDITS.md` (first line), `THIRD_PARTY_NOTICES.md` (MPUlt row), `work/experiments/magic600-04/packaging/README.md` (first line), `docs/RELEASE_0_4.md` | Keep all four; add the credit line to `RELEASE_0_4_1.md` |
| `licenses/MPUlt-MIT.txt` with the unchanged Melinda Green copyright line | Present | Verify byte-identical to 0.4 |
| CPython, NumPy (full wheel licence including OpenBLAS/LAPACK terms), PyInstaller, Inno Setup notices | `licenses/` and `THIRD_PARTY_NOTICES.md` | If the build runtime changes versions, replace the notices from the exact new distribution and update the NumPy/OpenBLAS version line |
| No Microsoft Managed DirectX DLLs in the package | Enforced by `FORBIDDEN_DLLS` and `inspect_payload` | Run `check_package.py` on the final ZIP; `RELEASE.json` keeps `microsoft_directx_payload_included: false` |
| DirectX prerequisite points to Microsoft's official runtime | `DIRECTX.md`, `work/experiments/magic600-04/packaging/README.md` | Unchanged |
| No private material (databases, logs, screenshots, diagnostics, tokens) | `THIRD_PARTY_NOTICES.md` statement and payload check | Review the staged file list before the release commit |
| New dependencies added in 0.4.1 | None known | Any new runtime dependency needs a `THIRD_PARTY_NOTICES.md` row and its licence under `licenses/` |

The unmerged branch `codex/runtime-dependencies` contains earlier runtime-prerequisite packaging and notice work. Review it during step 7 before rewriting that material.

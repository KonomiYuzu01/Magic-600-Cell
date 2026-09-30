# 0.4.1 data directory: change list and test points

Owner decision, 2026-09-30: 0.4.1 keeps the 0.4 data directory. It is a bug-fix release of the same data format, so existing 0.4 users open their current session with no copy or migration. One-way migration to 1.0 remains the job of the `.c600migrate` exporter ([exporter-spec-draft](exporter-spec-draft.md)).

## Current behaviour (0.4 source)

- `launcher.py:186` sets the default data directory to `%LOCALAPPDATA%/Magic600Cell/0.4`. The engine opens the session directly in it: `engine.py:116` calls `Session(model, data)`, which gives `session.sqlite3` and `engine.lock`.
- `launcher.py:35`, `93` and `200` define the native cache `data/native-host-0.4/`, which holds the host executable, `launch.json` and `engine.log`.
- `prepare_native` (`launcher.py:34-53`) keeps the MPUlt runtime in `native-host-0.4/runtime/<first 16 hex of the MPUlt.exe sha256>/`. After the first launch, `MPUlt_settings.txt` in that directory is **user-owned** (size, field of view, lighting) and is never overwritten.
- Both directory names come from literals, not from `VERSION`. A version bump alone changes neither, but a search-and-replace of `0.4` would change both.

## Change list (for the step-5 or step-7 packaging packet)

Allowed files: `work/experiments/magic600-04/packaging/package_contract.py`, `launcher.py` and `test_package_contract.py`. Nothing else.

1. In `package_contract.py`, add `DATA_LINE = '0.4'` next to `VERSION`. Comment it as the data directory for the whole 0.4 line, deliberately separate from the release version.
2. In `launcher.py:186`, replace the literal `'0.4'` with `DATA_LINE`.
3. In `launcher.py:35`, `93` and `200`, replace the literal `'native-host-0.4'` with `'native-host-' + DATA_LINE`. **Do not** rename it to `native-host-0.4.1`. That would orphan the user's `MPUlt_settings.txt`, and the MPUlt size, field of view and lighting settings would silently reset.
4. `VERSION = '0.4.1'` and `BUNDLE_NAME` change as listed in [release-prep](release-prep.md) section 2. The package manifest `version` becomes `0.4.1`; `layout_version` stays `1`.
5. The error text in `launcher.py:52` ("Use a fresh 0.4 data folder") stays accurate and can stay unchanged.
6. `MPUlt.exe` and `MPUlt_puzzles.txt` must be byte-identical to 0.4, because the model identity is unchanged. If `MPUlt.exe` changes, the runtime subdirectory hash changes and the user's MPUlt settings are not carried over. Treat that as a release blocker unless an explicit settings-copy step is added and reviewed.

These are persistence-adjacent changes. Run `python tests/test_core.py`, `python tests/test_reference_maps.py`, `python tests/test_crash.py`, `python tests/test_engine_lifecycle.py` and `python work/experiments/magic600-04/packaging/test_package_contract.py`.

## Test points (fresh isolated directories only; never the owner's session)

| ID | Setup | Expected |
|---|---|---|
| D1 | A synthetic 0.4 data directory (created by the 0.4 package with a few commits, a checkpoint and changed MPUlt settings); start 0.4.1 with it as `--data` | Same head, state hash, checkpoints and workspace (macros, keybinds, filters); `MPUlt_settings.txt` unchanged byte for byte |
| D2 | Default path resolution with `LOCALAPPDATA` pointed at a temporary directory | 0.4.1 resolves `.../Magic600Cell/0.4`, not `0.4.1` |
| D3 | 0.4 holds the session open, then 0.4.1 starts on the same directory | 0.4.1 refuses cleanly through the session lock; the directory is unchanged |
| D4 | Downgrade: after D1, start 0.4 again on the same directory | 0.4 opens it with no error; this checks that 0.4.1 wrote no key or schema that 0.4 rejects. If 0.4.1 must add a new preference key, 0.4 must ignore it (`session.py:23` uses `setdefault`, so this is expected to hold; verify it). |
| D5 | Fresh machine case: no existing directory | 0.4.1 creates `.../0.4/` and a solved root, the same as 0.4 does |
| D6 | `--verify-package` and `--engine-test` with a fresh `--data` | Unchanged behaviour; they still refuse existing directories |
| D7 | Package payload | No session, database or log files in the ZIP (`inspect_payload` unchanged) |

D1, D3 and D4 need the Windows package build. D2, D5 and D6 can run headless on Windows with the engine environment.

## Release note line

"0.4.1 uses the same data folder as 0.4. Your sessions, macros, keybindings and MPUlt view settings carry over. Keep a backup of `%LOCALAPPDATA%\Magic600Cell\0.4` if you plan to switch back to 0.4."

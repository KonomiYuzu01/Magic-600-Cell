---
id: build-identity-v2
type: concept
status: verified
visibility: public
summary: How a native build of the retained 0.4 host is identified from 0.4.1 on, what the identity covers and what it cannot promise.
related: [build-reproducibility-decisions, owner-decisions-2026-09-29]
supersedes: []
claims:
  - {id: identity-payload, evidence_kind: source, path: work/experiments/magic600-04/build_identity.py, sha256: 3af33ded7d4f82a18b338ab91d66fdf256e8be744514a95107db9039cdfb1115, checked_at: 2026-09-30}
  - {id: sealed-recipe, evidence_kind: source, path: native/bootstrap.py, sha256: e72cb94e70de2f39a1df04392f6594289b0efffb7c756850dc83ba568bc73266, checked_at: 2026-09-30}
  - {id: receipt-checks, evidence_kind: source, path: work/experiments/magic600-04/native_launch.py, sha256: dab89208b63f8e37e1fd016ed3a88d61c39853a6b7df609d6b54fcfbbc182229, checked_at: 2026-09-30}
  - {id: packaging-consumer, evidence_kind: source, path: work/experiments/magic600-04/packaging/assemble.py, sha256: 3833efb59482c285fbf273d256df24277da42830241b3b0c3296a1c09a14efde, checked_at: 2026-09-30}
  - {id: invariance-tests, evidence_kind: fixture, path: tests/test_build_identity.py, sha256: 971463c4dbe8ca11bee655cfb485f715c54ef04c3d6fcdfa1ab9fd6c33c5c079, checked_at: 2026-09-30}
  - {id: byte-exact-checkout, evidence_kind: source, path: .gitattributes, sha256: 018ad2ea40527c9f02c1e384103567d79b00ab3c6b41c54ac67f79b5e2205ed1, checked_at: 2026-09-30}
  - {id: engine-environment, evidence_kind: source, path: tools/toolchain.lock.json, sha256: 1a181fd36116af5906edbf54f0d19f0683f7b37840d2192bafcd74437e4e1c6c, checked_at: 2026-09-30}
  - {id: continuity-0-4, evidence_kind: source, path: docs/RELEASE_0_4_CONTINUITY.json, sha256: f75c6411fc6cc7d1983836460f33f7324e6d89b4daec17b374a8e193b537b41b, checked_at: 2026-09-30}
  - {id: windows-startup-regression, evidence_kind: actual_windows_directx, checked_at: 2026-09-30}
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
- **interpreter**: the implementation, exact version and bits, and the base binaries and environment launcher by role. For NumPy, it binds a digest of the wheel payload, verified file by file against the installed `RECORD`. It excludes installer-written files, bytecode and console-script wrappers, which embed their install path.

The receipt also records, outside the identity:
- the harness files;
- the platform string;
- the artifacts;
- launch evidence such as the copied runtime and the user-editable settings.

## How it is checked

- `check_evidence` refuses receipts whose `evidence_version` is missing or does not match their sections. For a v2 receipt, it:
  - rehashes the identity;
  - rehashes every product, harness and artifact file;
  - re-resolves the compiler and interpreter and compares them role by role, counting a missing role as missing.
- `packaging/assemble.py` accepts a v2 receipt only if the identity rehashes and every product file matches current bytes. The stored `after_build` status alone is not enough.
- The engine environment is the allowlisted `engine-python` entry: the owner-installed CPython 3.14.7 with hash-pinned NumPy 2.3.5, never a downloaded or uv-managed interpreter.

## What it cannot promise

- The executable is not bit-reproducible. The legacy compiler has no deterministic mode, so a build is reused only while its recorded executable hash still matches.
- The standard library is covered by the exact interpreter version and base binaries, not file by file.
- The 0.4 build itself cannot be recreated on later machines. Its toolchain (CPython 3.12.14, csc 4.8.9232.0, Windows 10) differs. `tools/provenance/continuity_04.py` instead proves the following:
  - the stored 0.4 payload still rehashes to its identity;
  - 93 of its 96 inputs are byte-identical in this repository;
  - the other 3 are listed 0.4.1 adaptations whose release bytes remain in Git history.

## Evidence on the owner's Windows 11 machine (30 September 2026)

- A checkout under `* -text` held the committed bytes for all tracked files, and all model assets matched.
- The product built with identity `56039c362c0fd361…`. The identity was unchanged after harness-only edits and line-ending repairs, and the build then reused the recorded executable.
- The native harness compiled with its inputs bound (`run_postapproval.py --compile-only`).
- The WinForms startup regression passed its 19 checks (`native/bootstrap.py --self-test-only`). This is actual Windows evidence of startup only. The DirectX rendering path, input, long sessions and performance were not exercised, and the full harness run is still pending.

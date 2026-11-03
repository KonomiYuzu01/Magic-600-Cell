# Public edition and validation boundary

This is a documentation and measured-results ingestion. Under the repository's
documentation/result-record risk tier it does not require another model review.
The mathematical independent reviews are preserved as sanitized research
records; they are not reviews of new product code. No runtime or research
executable, architecture contract, agent rule or toolchain input is changed.

Production used locally installed CPython 3.14.7 and Python's standard-library
JSON, text, SHA-256 and gzip support. The original mathematical checks used
CPython 3.14.7 and NumPy 2.3.5. The local publication command was
`python <publication-tools>/package_results.py`; the original inputs and that
local preparation tool are retained outside the repository. Angle-bracket paths
in old records are location tokens, never directories to resolve on a reader's
machine. This is not a claim that the local preparation tool is shipped here.

The preparation selects a bounded list of completed proofs and receipts, removes
administrative metadata, replaces machine paths with location tokens, serializes
JSON with `ensure_ascii=False` and compact separators, and uses
`gzip.compress(data, compresslevel=9, mtime=0)` for JSON over 250,000 bytes.
`IMPORT-MANIFEST.json` records both captured-input and public-output digests.
The English summary is an editorial guide to those sources, not a new theorem.

Validation for the public edition:

- Verify every original digest against its captured input, every stored digest,
  every decompressed digest, and every parsed JSON payload against the permitted
  metadata-only transformation. All numeric mathematical values are preserved.
- Scan every added text file and every decompressed JSON key/value for local
  paths, account/usage data, session identifiers, personal email and credential
  patterns. No transcript, screenshot or raw machine log is included.
- Recompute the final 36-coordinate rank arithmetic, the 1,777-piece cleanup
  difference, and the regional input/endpoint matching equations on all
  177,120 pieces. These are ingestion integrity checks, not fresh trajectory
  replays or new frontier research.
- Run `python tools/wiki/lint.py`, `python tests/test_wiki_lint.py` and
  `python tests/test_agent_rules_sync.py` on the finished branch. The PR reports
  their actual outcomes and the GitHub-hosted headless workflow result.

The import does not rerun all archived simulator trajectories, full suffix
searches, DirectX/native regressions or performance measurements. Historical
PASS receipts keep their original finite scope and source version.

The base revision contains six pairs of tracked run filenames differing only
in case under stage 16. A Windows checkout cannot materialize both members of
each pair simultaneously. They are pre-existing, excluded from this change and
left untouched in the Git index; this publication does not repair those files.

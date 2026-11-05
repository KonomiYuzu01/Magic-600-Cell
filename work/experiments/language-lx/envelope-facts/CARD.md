# E-2.4-LE: PS1.38 facts envelopes

This is experiment code, never product code. Every decision is **open**, with the
owner as decider. It selects no route, engine language or syntax and answers no LQ.

| Field | Frozen card |
| --- | --- |
| LX/card | Supplement to LX-1 to LX-9: PS1.35 to PS1.38, E-2.4-LE |
| Allowance | docs/wiki/decisions/owner-decisions-2026-11-05.md, decision 2; packet PS1.38 |
| Time box | what can be done now; one wrapper call of at most two hours |
| Question measured | Facts-envelope nodes and bytes for the identical stage 14 to 19 corpus inclusion rule |
| Input | Git blobs at 1e7d0074df96cbeb05f55282cf8f6c03fbf4a4ff, pinned in MANIFEST.json |
| Baseline | 497 records: 417 JSON and 80 gzip JSON, 13,412,769 bytes as stored, no JSONL |
| Required outputs | Manifest, facts envelopes/digests/sample, leaf/AND/OR/edge counts, canonical and compressed bytes, shared dependencies, unmapped labels, every incomplete record, finite display enumeration and open collisions |
| Acceptance | `python work/experiments/language-lx/envelope-facts/check.py` exits 0 |
| Evidence class | Source for serialization and corpus accounting; synthetic for constructed display combinations |
| Kill criteria | Baseline mismatch, lost source or label, false binding, invented support, non-total enumeration, stale artifacts or a reached time limit |
| Cannot show | A product migration format, complete mathematical argument graphs, lossless judgements, new verification/replay, accepted display policy, performance or a route/engine-language/syntax choice |
| Decision / decider | open / owner |

The judgement alternative waits for LQ3 and chapter 06. No research record, schema,
architecture chapter or acceptance decision is changed. The packet's stop condition
is the passing acceptance command plus a report of every PS1.38 measurement and open
collision. There is no installation, network use, personal session or native/GPU run.

## PS1.38 clause-by-clause check

| Requirement | Implementation / retained evidence |
| --- | --- |
| Facts-option construction for stages 14 to 19 | corpus.py and envelopes.py; 497 partial envelopes |
| Baseline and same inclusion rule | MANIFEST.json and RECORDS.json; gzip members count once each |
| Git blobs via path/blob digest/size manifest | `ls-tree` and `cat-file --batch`; no corpus checkout reads |
| Leaf, AND, OR, edge counts | SUMMARY.md; incomplete leaf slots distinguished from admitted support |
| Canonical uncompressed versus stored compressed bytes | Per-record and package boundaries in SUMMARY.md |
| Shared dependencies | DEPENDENCIES.json and regenerated catalogue; source digest equality is not independence |
| Unmapped labels | All 89 rows in UNMAPPED.json, plus missing typed bindings per record |
| Every admitted field combination has one word or typed inconsistency | DISPLAY.json; finite domain declared in envelope_model.md |
| No judgement-option decision | decision open; no claim that chapters 05/06 are accepted |

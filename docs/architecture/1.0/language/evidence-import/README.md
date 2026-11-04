# Evidence import table: draft

Status: **draft for the owner. Not accepted.** This folder holds the first version of the table that
[default 20](../ecosystem-partition.md) asks for: every (format, field, value) of the stage 14 to 19 research records with
what it maps to, or the reason it stays recorded word for word. Chapter 06 and
[chapter 05](../architecture/05-persistence.md), section 11, use it. Nothing is frozen, and four rules below still need
the mathematician's and the main session's reading.

| File | What it is |
| --- | --- |
| `corpus-labels.json` | The full table, generated |
| `corpus-labels.md` | The human summary, generated |
| `extract_labels.py` | The script that makes both |
| `rules.txt`, `refs.txt`, `ext.txt`, `rule.txt` | Its inputs: mapping rules, the reason for each rule, rows from outside the corpus, the enumeration rule |

## How it is made

Run this from the repository root, with Python 3 and Git:

```bash
python docs/architecture/1.0/language/evidence-import/extract_labels.py
```

- The script reads the corpus from Git blobs at HEAD, never from a checkout. On Windows six pairs of names under
  `16-six-jams/results/runs/` differ only in case, so a walk of the checkout would miss six records (DT-L1 R3-E2).
- It writes only the two generated files of this folder. It changes no research file.
- The table on record was made at Git HEAD `7927cb30fb66398315426d8ca17a48ee3874b2d6`. The corpus manifest has SHA-256
  `e2b388befa1671be67219b23c4a565c003c8ae960bd0aa69e07ddb030a669702`.
- `corpus-labels.json` has SHA-256 `6fcd45325a8789736534d53cd336297a0718c3c4386bb9da5af21fbb0ac13a68`. A run at a later
  HEAD changes the `head` field, and it changes the case-pair and working-tree fields if the checkout differs. The manifest
  digest is what identifies the corpus.

## What it says

Source evidence, counted from Git blobs (DT-L1 round 3, engineer advisor):

| Measure | Value |
| --- | --- |
| Records | 497: 417 `.json` and 80 `.json.gz`, 13,412,769 bytes as stored |
| Formats | 160: 54 declared, 106 synthetic |
| Triples | 387: 190 strings, 197 booleans |
| Mapped | 298 |
| Recorded with a stated reason | 89 |
| Without a rule | 0 |
| Occurrences | 22,513 |

Rows set one or more of six fields: claim, scope, basis, check, check mode and job outcome. Their values are those of
partition section 8.1 and the import values of chapter 06, EVD.47. One row rarely states a whole claim, and an evidence
envelope is built from several rows of one record.

## What was inferred

The engineer advisor set these four rules from names and stage READMEs. Chapter 06 (EVD.52) lets them stand, each with a
limit:

- **V14** maps historical `*-independent*` reviewer records to model review with a fresh audit, and keeps "independent
  checker" for a 1.0 checker under KN6. It applies to reviewer records, not to program checkers.
- **P13** sets the check mode from a key's name alone. A mode read that way never creates a transport edge.
- **K1** leaves the check to the enclosing record, and only when that record's check covers the row.
- **H1** maps reachability rows to open on one finite instance, as a claim of their own. Such a row can discharge nothing.

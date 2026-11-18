---
id: owner-decisions-2026-11-17-c5-evidence
type: decision
status: verified
visibility: public
summary: Owner decisions of 17 November 2026, each the recommended option - C5's word keys (LG2.18; CT-LB1-06) use LB1.10's draft `trace-key/1`, for 600-cell-Full words only, freezing with the IR, with the jumbling letters later, so CT-LB1-06 proceeds; and a receipt does not count as evidence, so `use_evidence` and `use_goal` keep refusing it and a program cites committed state through exact data. No frozen text and no code changes.
related: [owner-decisions-2026-11-16-phs52, owner-decisions-2026-11-15-freeze]
supersedes: []
claims:
  - {id: c5-word-keys-trace-key-1, evidence_kind: decision, checked_at: 2026-11-17}
  - {id: receipt-not-evidence, evidence_kind: decision, checked_at: 2026-11-17}
  - {id: trace-key-1-draft, evidence_kind: source, path: rust/c600-run/src/trace.rs, checked_at: 2026-11-18}
---

# Owner decisions, 17 November 2026: word keys and receipts

The owner answered two open language questions by card on 17 November 2026, in Chinese. This page records the answers
in English. Both answers are the recommended options.

## 1. Word keys for C5 (LG2.18; CT-LB1-06)

The card: LP3 needs a word key to decide whether two words count as the same word. Option one uses `trace-key/1`: two
words that differ only by swapping adjacent twists whose order does not change the result count as the same word, and
the lexicographically least spelling is the key. It is a draft, used for 600-cell-Full words only, and it freezes with
the IR; the jumbling letters come later, and CT-LB1-06 proceeds. Option two leaves the key open, and CT-LB1-06 waits.

The owner chose option one, the recommended option: "use trace-key/1".

What follows:

- LB1.10's `trace-key/1`, under the independence contract `full-disjoint-support/1`, is the draft key of 600-cell-Full
  words. LP2 built it in `rust/c600-run/src/trace.rs` (pull request 214); `rust/c600-tools/src/supports.rs` builds the
  relation from the loaded model's 1,200 generator supports. It freezes with the IR (PHS.49).
- CT-LB1-06 proceeds on its 600-cell-Full half. Its jumbling half, `jumble-locality/1` over menu letters, waits for an
  exact Δ(c,d) entry in `c600-jumble` (LP2).
- A trace key stays its own kind: it is never a word or a word key, and it never executes.

## 2. A receipt is not evidence

The card: today a receipt cannot be given at `use_evidence` or `use_goal`; EVD.5 says a job outcome is not evidence;
EVD.64 lets a program write only the lowest grade. The options were:

- not evidence, keep refusing; a program cites committed state through exact data such as a state hash, treated as
  the program's own computation; no change to frozen text or code;
- admit the receipt's exact fields, which changes LG2 and EVD under PHS.52;
- admit only a host-verified receipt, which changes frozen text and adds a writer to EVD.64.

The owner chose the first, the recommended option: "it does not count; keep refusing".

What follows: no code and no frozen text change. The checker's refusal of a receipt at an evidence or goal sink and the
run-time code `PS1:input.noncanonical_value` stay as section 4 of the
[16 November decisions](owner-decisions-2026-11-16-phs52.md) left them. This answers the first point under that page's
"What stays open".

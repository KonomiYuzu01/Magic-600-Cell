# 01. Three clean restorations: X to E1 to E2 to E3 (12 October 2026)

**The question.** Can pieces that are off the lattice be put back on it, without disturbing anything else, by words
built only from the current state? This stage answers it on one reachable input.

**The input.** X is a reachable configuration with 107 pieces off the lattice, in three rigid blocks of 87, 17 and 3
pieces. Its blocking mechanism is the same as that of the owner's successful state 36563: the old window 36568,
replayed unchanged on X, is blocked at its twelfth twist. X is reached from the solved state by one replayed word;
see [stage 05](../05-seed-table/PROOFS-3E.md), section 5.

**The result.** Three words built from the current state restore the three blocks one after another. Each step:
- leaves zero collateral;
- keeps every other off piece at its exact pose;
- is replayed on the exact simulator with every twist admissible and depth one tested at every prefix.

| Segment | Directory | Word | Twists (five parts) | Off pieces | Peak off inside the segment | Restored |
| --- | --- | --- | ---: | --- | ---: | --- |
| X to E1 | [x-17-block](x-17-block/) | V·P·Q·P⁻¹·V⁻¹ | 1,112 (541+10+10+10+541) | 107 → 90 | 6,015 | the 17-piece block |
| E1 to E2 | [e1-87-block](e1-87-block/) | V2·P2·Q2·P2⁻¹·V2⁻¹ | 1,210 (590+10+10+10+590) | 90 → 3 | 5,802 | the 87-piece block |
| E2 to E3 | [e2-b-prime](e2-b-prime/) | V3·P4·F·P4⁻¹·V3⁻¹ | 79 (8+31+1+31+8) | 3 → 0 | 3,097 | B′: pieces 16245, 16249 and 32088 |

At E3 all 177,120 pieces are on the lattice. That is not a solved puzzle: the pieces sit at lattice poses, not
necessarily at home. The lattice phase that finishes it to the solved state is in
[stage 09](../09-general-method/README.md) (`earlier/x-e3-solved/`).

## How the words work

- **Conjugate closure.** A word W that would move an off block onto the lattice usually has to pass through
  chambers that are occupied. V is a SAFE-ALL word that first empties exactly those chambers, the set Bad(W). Then
  W runs, and then V⁻¹ undoes the preparation.
  - V comes from a peeling certificate (Theorem B of
    [buffer-transport](../../theory/buffer-transport.md)), checked before V is built.
  - The geometric premises (cut compatibility, block condition, triple lens, parking rule, ghost prediction) are exact
    tests on the word, the block and the chambers. They are kept apart from what only the replay can show:
    legality at every prefix, depth, end pose and collateral.
  - [x-17-block/report/A-B-closure.md](x-17-block/report/A-B-closure.md) states the premises and conclusions.
- **Parking.** In the first two segments W = P·Q·P⁻¹:
  - P = [[X, Y], Z] parks lattice pieces out of the way (198 pieces in segment 1, 592 in segment 2);
  - Q = [A, [B, X]] puts the block on the lattice;
  - P⁻¹ brings the parked pieces back.
- **Lemma H′** (segment 2). If U is admissible at Y, and S is the set of pieces that are off the lattice at Y and
  on it after U, then |Bad(U)| ≥ |S|. So a word that cleanly restores every off piece has Bad(U) equal to the set of
  holes of Y.
- **Iterated conjugation** (segment 3), a new template.
  - It sets P₀ = (237, 13) and P_{k+1} = P_k·x_k·P_k⁻¹, with x_k a K⁺ letter at each layer.
  - After four layers the bubble of bad chambers around B′ is gone: the parked counts are 3,097, 134, 80, 14 and
    10.
  - F = (162, 1) finishes.
  - The whole 79-twist word has Bad equal to the 3 holes of E2, as Lemma H′ requires.

The SAFE-ALL resources grew along the chain:

| State | Off pieces | SAFE-ALL grips | Letters |
| --- | ---: | ---: | ---: |
| X | 107 | 570 | 6,270 |
| E1 | 90 | 581 | 6,391 |
| E2 | 3 | 592 | 6,512 |
| E3 | 0 | 600 | 6,600 |

Each restoration freed the grips its block had blocked; no grip was lost.

## Reviews

| Segment | Exact replay | Independent audit | Codex review (read-only, fast tier) |
| --- | --- | --- | --- |
| X to E1 | 1,112 twists, all admissible, depth one at every prefix | Audited on the owner's machine: the 302-twist reachability word and the 1,112-twist restoring word, all 1,414 prefixes and full matrices. The owner's independent review judged it correct with 0 errors or gaps. The acceptance covers this native chain only, not the general A, B and G1 to G4 statements of the segment | none |
| E1 to E2 | 1,210 twists, same checks | 12 audit agents (details under Errata). Lemma H was withdrawn and replaced by Lemma H′ | Full review: 1 blocker and 2 majors, all adopted. Scoped verification 1: 1 new blocker, adopted. Scoped verification 2: pass, 0 findings |
| E2 to E3 | 79 twists, same checks | 7 audit agents: replay, tables, word, certificate, layers of the iterated conjugation, report against data, mathematics. No blocker or major; minors fixed | Full review: pass, 0 findings. It reviewed the text before the audit; the later added proof steps were not re-reviewed |

The Codex reviews checked mathematical arguments only and took the computed values as given. The review records are
under each segment's `audit/codex/`.

## Errata

These corrections were issued on 12 October, after delivery, and are not edited into the segment reports. This page
takes precedence over them.
- **E-1. Review status of segment 1.** The owner completed an audit and an independent review of segment 1 on their
  own machine (see the table above).
- **E-2. Coverage of `verify_chain.py`.** The script reads neither `replay_seconds` nor `restores`, nor the
  descriptive strings. "Claims pass" is taken from the replay report's own verdict. Off counts at the links are not
  cross-checked. Check the checksum list before running it.
- **E-3. Scope of the segment 3 Codex review.** It reviewed the report before the audit. The steps added afterwards
  were not re-reviewed:
  - a step of the step lemma;
  - the parking-count condition;
  - the S-type premise |GH| = |L|;
  - the state F acts on.
- **E-4. Independence of the segment 2 audit.** All 12 audit items wrote their own code. Only the replay, table and
  word audits avoided the construction code. Six items reused the ghost modules (`ghostlib`, `fastghost`), and
  skeptic-F4 re-ran `premx.py` for comparison.
- **E-5. Audit counts.** The counts quoted in the segment 2 summary differ from those in its output files:
  - the word audit records 73 static checks with 0 failures;
  - the table audit has 44 fields, of which 17 are boolean checks, all true;
  - the report-against-data audit ships its script only.
- **E-6. Off-lattice counts in segment 3.** The figures 2,112 and 952 describe the 3,091 lattice pieces that changed
  exact pose. Over the whole segment, 5,263 lattice pieces of E2 leave the lattice at some prefix, at most 3,097 at
  once.
- **E-7. Poses.** In segments 1 and 2 every lattice piece keeps its exact pose. In segment 3 the lattice pieces
  stay on the lattice, but 3,091 of them change exact pose and 3,086 of those change chamber.
- **E-8. "Before construction" in segment 3** means before any twist of the delivered word was applied. The S-type
  word P4·V3·F·P4⁻¹ (71 twists) had already been built and replayed on E2 before that.
- **E-9. Further corrections.** Three corrections are missing from the list:
  - the premises G1 to G4 of segment 2 were computed after the replay, without using its result;
  - segment 3 added a step to the step lemma;
  - segment 3 added the S-type premise.
- **E-10. Limits.**
  - The triple lens G3 and the parking rule G4 hold on the computed instances only and do not apply at E2.
  - N1 and N2 are necessary conditions only.
  - The searches were finite:
    - segment 1 used 21 of 60,588 G2 candidates and replayed 1 of 23 parking words;
    - segment 2 tried 23 Q and 3,718 parking words, took 3 into construction, and c5 failed finitely.
- **E-11. Lemma H′** holds in the simulator model under two premises:
  - piece regions have disjoint interiors;
  - every menu element fixes its grip's pole. This was checked on 36,000 of 36,000 cases and is not proved.
- **E-12.** The peeling sentence of segment 2 needs a missing step: the thresholds equal the target, and the holes
  meet every threshold.

## Limits

- **One instance.** This is a single instance, not a general closure family and not a general solution of
  question E.
- **Search.** Iterated conjugation was computed on this instance only. Whether it always reaches a layer without a
  bubble is not proved; the greedy rule is one choice among several.
- **Not run.** Windows, DirectX, the native host regression and any performance measurement.

## Replay

Run from a segment directory. `<theory>` is `research/jumbling/theory`. First decompress the tables, for example with
`gunzip -k input/*.gz`.

    # segment 1 (x-17-block)
    python -I <theory>/replay_check.py --table input/X-table.json \
        --words D17/parts/V.json D17/parts/P.json D17/parts/Q.json D17/parts/Pinv.json D17/parts/Vinv.json \
        --depth-every 1 --claims D17/claims.json --export end-table.json --out replay.json
    # segment 2 (e1-87-block)
    python -I <theory>/replay_check.py --table input/E1-table.json \
        --words D87/parts/V.json D87/parts/P.json D87/parts/Q.json D87/parts/Pinv.json D87/parts/Vinv.json \
        --depth-every 1 --claims D87/claims.json --export E2-table.json --out replay.json
    # segment 3 (e2-b-prime)
    python -I <theory>/replay_check.py --table input/E2-table.json \
        --words D3/parts/V.json D3/parts/P.json D3/parts/Q.json D3/parts/Pinv.json D3/parts/Vinv.json \
        --depth-every 1 --claims D3/claims.json --export E3-table.json --out replay.json

Each segment's `scripts/` rebuild the certificate and the word from the start table, for example
`scripts/closure2.py`. The reports give the commands and the run times.

## Files

- **Each segment directory holds:**
  - `report/`: the English reports. Segment 1 has `C-X.md` (the input X), `A-B-closure.md` (conjugate closure) and
    `D-X17.md` (the 17-block); segment 2 has `D2-E1-E2.md`; segment 3 has `D3-E2-E3.md`;
  - `input/`: the start table;
  - the construction records (`D17/`, `D87/` or `D3/`): the certificate, word parts, claims, replay, prefixes and
    end table;
  - `audit/`: the independent audit, with the Codex review records under `audit/codex/`;
  - `scripts/`.
- **`chain/`:**
  - `chain.json`: the chain index, with the packages, tables, digests, part lengths, off counts and links;
  - `verify_chain.py`: a standard-library checker of the three original delivery archives. The archives are not
    stored here, so it does not run on this directory.
- **State digests:**
  - X: `f13c168e…`
  - E1: `90399745…`
  - E2: `19e404d4…`
  - E3: `fb199d99…`
- **Not stored:** the original archives, the Chinese delivery notes and the checksum lists.
- **Stored gzipped:** data files of 256 KiB or more. `IMPORT-MANIFEST.json` in each segment lists them, with the
  SHA-256 of the original bytes.

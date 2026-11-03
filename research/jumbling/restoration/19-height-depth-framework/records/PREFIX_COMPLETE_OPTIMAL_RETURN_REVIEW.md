# Prefix-complete optimal return: independent review

Archived mathematical record from 2 November 2026. See README.md for its current scope and the independent review record. Original-source digests below identify the captured research version; IMPORT-MANIFEST.json binds this sanitized edition.

This review covers PREFIX_COMPLETE_OPTIMAL_RETURN.md SHA256 9f687b88bb67e4f8b88c43fde9fa69d60590ca62a830c9ef8f4bb99add9fb2ae. The computation initially read its earlier version c60a600c0bf7f8cc2df66bcf90de0432b6475f1dfec2d10c43696a4ff767161a; the only reviewed correction is the endpoint-background clarification discussed below. Source identity remains a2ebb4cccd6b60d7527c001ef3b2cbc56b179249.

Evidence:
- evidence/prefix-complete-optimal-decoder.json, SHA256 e0ee92e8cefd7870abb8d6411909eb922fd56e3e549d56590ff9bdf910fd6680.
- prefix-complete-optimal-decoder-command.ps1.txt, SHA256 9cae131d34f1280c5981989d69e025407acbfa3816a887889288245b52770d7a.
- The JSON pins the boundary-code certificate, exact sigma table, source CERT.json and REPORT.md, and independently reviewed 1824-letter history suffix.

## Proof audit

Let A be exactly the guarded boundary language already recognized by the boundary decoder. For each certificate word W_i and proper-prefix length 0 <= k < |W_i|, retain its chronological full target matrix P_(i,k) and its two cumulative debris products relative to sigma. The required prefix language is the finite union of V_(i,k)(A).

The source premise is sufficient for all seven prefix families, not just the next word selected by a particular covering box. REPORT.md:150 checks the Good products and the perturbation inequality at every state of every W_i. REPORT.md:163 states the uniform perturbation lemma for all partial products and all unit u in the guard ball. Combined with the isolation lemma at REPORT.md:107 and step lemma at REPORT.md:131, this makes each prefix legal from every guarded boundary, keeps all K pieces co-posed, and raises their common height by exactly 2k. The box covering rule is needed to ensure a selected complete word returns to the guard; it is not needed for these prefix conclusions.

For sound recognition, a matching full outside template and an accepted boundary candidate Y imply:
1. Y is an admitted boundary by the existing decoder theorem.
2. The source prefix result makes V_(i,k) legal from Y.
3. The target product gives exactly the input K pose; the complete template gives every remaining labelled pose.
Thus the reconstructed endpoint is exactly the input, so reversing the literal prefix is legal and reaches Y. The boundary inverse and fixed history suffix then reach the same exact tau240 state.

For completeness, any defining description of a state in the finite union supplies a matching template and an admitted predecessor. There is no need to recover its historical description uniquely. Each accepted description has return length
    k + (H - 2k - 240)/2 = (H - 240)/2.
The universal per-letter height-change bound for any fixed labelled K piece gives the matching lower bound for all competing legal words, including paths using other grips or splitting K. It follows that this is the distance to fixed tau240 and the minimum over the stated low-height target sets.

A prefix matrix need not be pure left quaternion multiplication. Only the complete W_i products have that certified form. The prototype retains full exact 4x4 prefix products and tests that P^-1 Q Qstar^T is pure unit left before calling the boundary decoder; no invalid partial-product simplification is used.

The finite catalogue and decreasing boundary decoder prove termination. The prototype checks the candidate height identity and floor before decoding; its iteration bound is therefore immediately at most floor((H-3888)/132) when H>=3888. Inputs below the floor have no eligible candidate. This is not a bit-complexity bound.

## Finite verification

The exact catalogue has:
- 1746 proper-prefix occurrences, including k=0 for each word;
- 1348 distinct full target-product/debris-label entries;
- 1348 distinct target matrices;
- 298 distinct sigma-relative debris-label pairs.

Every occurrence is preserved in the catalogue. Duplicate exact entries have the same prefix length. Labels are merely a candidate index. The implementation infers them from one representative in each nonempty debris set, then compares all 177113 outside-K canonical matrices with the predicted template. The sigma debris sets have sizes 2505 at grip11 and2702 at grip80.

Two non-boundary samples were generated using the exact deterministic first-containing-box rule:
- One letter into W_2 from sigma: height3890, branch3; digest f0a8c255aa9910fd937f96b74976e704483f35fe35c6d7d7fa86f60918dab9a4. The recognized inverse prefix returns exact sigma. The complete return length including the pinned suffix is1825.
- W_2 followed by seven letters of selected W_5: height4506, branch3; digest 0ddc9b15018f750c4d5d4d605beca46ba39760a605e6a31384bba2cc1687d38c. The recognized inverse prefix returns the exact W_2 boundary, and the boundary decoder recovers reverse block list[2]. Its full inverse replay reaches exact sigma. The complete return length including the pinned suffix is2133.

Both were recognized using equivalent catalogue descriptions from W_0. This is consistent with repeated prefix occurrences and demonstrates why occurrence identity need not be unique. Altogether620 literal letters were fully replayed forward and back. The1824-letter suffix was not redundantly replayed here; its independently pinned full-state verification remains a premise.

Both negative tests reject:
- A synthetic change to outside piece0 leaves the two representative labels unchanged. The full template comparison detects exactly that one mismatch.
- A synthetic common K pose L(u)Qstar, with
      u=(1001997/1002005,4004/1002005,0,0),
  has exact unit norm, satisfies the Euclidean guard, has height3888 and branch2, and has sigma's complete outside background, yet differs from sigma. Both recognition layers reject it at the exact floor. This is a deliberately synthetic rational pose; reachability by I_a is not asserted.

The bounded prototype used63.656 CPU seconds, below its600-second limit. These are headless exact-geometry results, not native rendering or performance evidence.

## Resolved scope finding

PC-01: endpoint-background ambiguity, raised by the parent during review and verified corrected.

The original draft's optional outside-restoration wording could be read as restoration of the prefix input's own background. That would exceed the proved result. For example, the first positive sample has sigma-relative debris labels(42,0), while tau240 has labels(0,0); nonempty debris and invertible exact poses make these backgrounds different.

The revised draft explicitly restricts the optional endpoint condition to equality with sigma's outside-K background at PREFIX_COMPLETE_OPTIMAL_RETURN.md:37, digest9f687b88bb67e4f8b88c43fde9fa69d60590ca62a830c9ef8f4bb99add9fb2ae. The JSON endpoint_contract states the same restriction. Verification status: resolved. No restoration to the prefix input's own outside background is claimed.

## Remaining boundaries

The infinite-language claim follows from the source lemmas and boundary theorem; two samples do not establish universal geometry on their own. This run does not repeat all published perturbation-margin checks. It neither recognizes arbitrary same-height or guard-only states nor supplies a general lattice landing algorithm. Exact equality is proved for the fixed tau240 endpoint and minima over the specified target sets, not for every individual low-height target.

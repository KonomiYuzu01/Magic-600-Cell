# E-2.4-L3 part B, step 0: shared synthetic bench

Decision: **open**. Decider: **owner**. The LX-3 card is chapter 11 RTE.44 to
RTE.46; allowance is the 5 November 2026 owner decision 2 and this scoped packet.

| Field | Frozen card |
| --- | --- |
| Question | Does each candidate enforce its sandbox and selected flow mechanism while reproducing exact results and reporting harmless-tool refusals? |
| Decision it feeds | LQ9 route admission and LQ21 form of flow control; this step selects neither |
| Hypothesis | None established; deterministic inputs and conservative flow rules are requirements to test |
| Method | Freeze keys, exact payload profile, display precision, policy, allowed-tool list and must-refuse corpus by digest; test repeated Windows/Linux runs, each budget, interruption, cache changes and planted clock, file, network and process access without grants under DET.49; exercise the forms and variants in RTE.45 |
| Time box | what can be done now; one wrapper call of at most two hours |
| Kill criteria | A reproducibility mismatch, successful planted read, unrefused prohibited case, crash, hang or wrong refusal code fails that form or variant; preserve the case; timeout gives no passing result for omitted cases |
| Evidence class | source and synthetic; this bench uses a four-slot fake fixture, never the real engine |
| Cannot show | All-program safety, authorship, understanding, absence of covert channels, product latency or an acceptable false-refusal threshold |
| Product left behind | Digest-frozen refusal and allowed-tool corpora, canonical result hashes, per-form refusal lists and reproducibility/enforcement counterexamples |

Step 0 adds no route candidate. The open and closed controls run once per cell
and host condition, across three form names and two variants. Form names on a
control are host conditions, not implemented flow mechanisms. No ten-run series
is performed. A reached time limit is no pass.

The corpus's raw `FREEZE.json` pin is
`d7eff17503ee95d45ddca0efe38ec3bb09418b0248a6279afe2dce0a90ea6627`.
Control sources, rendered bytes, charge mapping and dependencies are in their
`bindings.json`. `MANIFEST.json` pins the retained artifacts, including the
acceptance check and summaries. Actual expanded keys and canonical payload
hashes are in the stored summaries; the run records are regenerable under build.

Exact fields, canonical encoding, display precision and all excluded fields are
copied by reference from the frozen `corpus/profile.json` (raw SHA-256
`b46ec428c9fb44612ae5312ff56a4c615796a0d4b9343b8300d7a004f5338e74`).
Excluded fields: wall_clock_time, elapsed_seconds, cpu_seconds, machine, host,
platform, user_name, physical_paths, working_directory, locale, timings,
diagnostic_text, transport_framing, compression_metadata, process_id,
per_launch_handles, job_id, preview_id, command_receipts,
session_local_revision_refs, cache_diagnostics. Approximate coordinates are
checked separately against the rational references and pairwise; exact numeric
fields, literal words, coverage, counters and stop positions have no tolerance.

Only Python's standard library is used. Production and verification commands
are in `RESULT.md` and `harness/README.md`. This is no installation request.
The public decision fields LQ2, LQ9, LQ21, BS1 to BS4 and LP1 metering stay open.

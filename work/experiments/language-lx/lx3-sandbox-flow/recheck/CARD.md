# LX-3 frozen-bench recheck

| Field | Record |
| --- | --- |
| LX card / experiment | LX-3, chapter 11 RTE.44 to RTE.46 / E-2.4-L3 recheck |
| Owner allowance | `docs/wiki/decisions/owner-decisions-2026-11-05.md`, decision 2, and this isolated packet |
| Time box | what can be done now; one wrapper call of at most two hours |
| Question | Do the stored observations survive a stricter preservation predicate, and can unreachable or superseded writes erase persisted taint? |
| Method | Reproduce the classifier defect with a synthetic candidate; wrap the frozen host in every matrix child; run one pass per key for five prototypes and two controls; add separate two-run taint probes |
| Inputs | Frozen lx3-neutral-v2, the frozen bench and seven candidate declarations/bindings; exact pins in MANIFEST.json and the original manifests |
| Kill criteria | A preservation violation or forbidden installed preview is a negative result; a wrong refusal or a false refusal remains negative. A pin mismatch, broken control or nonrepeating stored outcome fails acceptance |
| Evidence | Source and synthetic; CPython and Node on Windows; no performance claim |
| Cannot show | All-program safety, authorship, understanding, absence of covert channels, product latency or an acceptable false-refusal threshold |
| Decision / decider | open / owner; no route, engine language, syntax, LQ or ADR decision |

The corpus, bench, candidates, counts and ten-run evidence are unchanged. This
pass adds a predicate, not a reproducibility series. B10 remains as frozen;
unsupported cells and the known B12/shared bench crashes retain their outcomes.
Additional probes never enter the corpus denominator. A reached limit is no pass.

The strict wrapper snapshots the complete session, pending preview, and every
workspace record's type, value and provenance. It checks before each sanctioned
mutator (`begin`, `workspace_put`, `preview`), at every refusal and at completion.
Only those frozen host methods advance the checkpoint. Checking before advancing
prevents a later legitimate write from concealing earlier corruption. Other host
methods do not legitimately change these three fields. The audit travels in the
excluded host envelope; original exact payloads, run keys and policy remain intact.

This is trusted synthetic host instrumentation, not protection against a malicious
Python adapter tampering with the instrumentation or a transient write restored
between observed boundaries. Existing preview, session, leakage, approval and
result checks still apply. The wrapper also reaches actual process-loss handshakes
and both overlapping shared-cache invocations.

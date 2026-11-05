# LX-9 Linux follow-up

Decision: **open**; decider: owner. Evidence: synthetic. Linux CPython 3.14.4.
Candidate and input identities are checked against the Windows manifests.

| Step | Requests | Pair | Target-bit differences | Exact rotation differences |
| --- | --- | --- | --- | --- |
| 1 | 13600 | reference/Linux | 852 | 0 |
| 1 | 13600 | pure/Linux | 852 | 0 |
| 1 | 13600 | rust/Linux | 1673 | 0 |
| 2 | 2890 | reference/Linux | 864 | 6 |
| 2 | 2890 | pure/Linux | 864 | 6 |
| 2 | 2890 | rust/Linux | 1739 | 6 |

Additional step-1 requests carried into step 2: 786.
Every difference is retained in `linux-differences.tsv` and the result tables.
No difference is a finite-sample result only; any different rotation triggers
unaccepted amendment evidence. This cannot show agreement for every request,
input latency, or an approved A1 change. Menu-mode drags remain untested.

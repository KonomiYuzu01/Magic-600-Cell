# PS1.38 summary

Stored leaf slots are incomplete source fact candidates; admitted EVD leaves are zero.

| Stage | Records | Source stored bytes | Source canonical bytes | Envelope canonical bytes | Envelope gzip bytes | Leaf slots | AND | OR | Edges |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 14 | 68 | 1462676 | 1074812 | 1558140 | 125566 | 2973 | 0 | 0 | 0 |
| 15 | 87 | 801742 | 633652 | 1664394 | 184814 | 2206 | 0 | 0 | 0 |
| 16 | 109 | 554204 | 289815 | 434074 | 139201 | 123 | 0 | 0 | 0 |
| 17 | 56 | 372420 | 204574 | 293250 | 95062 | 62 | 0 | 0 | 0 |
| 18 | 98 | 5962245 | 36973294 | 273901 | 98798 | 5 | 0 | 0 | 0 |
| 19 | 79 | 4259482 | 15105876 | 6200971 | 512657 | 6098 | 0 | 0 | 0 |

| Measure | Bytes or count |
| --- | ---: |
| AND | 0 |
| OR | 0 |
| admitted_leaves | 0 |
| captured_source_plus_envelope_package_bytes | 15050156 |
| common_input_captured_bytes | 558918 |
| common_input_gzip_bytes | 135832 |
| dependency_catalogue_canonical_bytes | 1987313 |
| dependency_catalogue_gzip_bytes | 262100 |
| edges | 0 |
| envelope_canonical_bytes | 10424730 |
| envelope_gzip_bytes | 1156098 |
| envelope_package_canonical_bytes | 13242586 |
| envelope_package_gzip_bytes | 1637387 |
| fact_groups | 15210 |
| labels | 22513 |
| leaf_slots | 11467 |
| manifest_canonical_bytes | 271625 |
| manifest_gzip_bytes | 83357 |
| mapped_labels | 21693 |
| original_canonical_bytes | 54282023 |
| original_canonical_gzip_bytes | 6273978 |
| original_decoded_bytes | 58976540 |
| original_stored_bytes | 13412769 |

All 497 source members are included in both columns, even without any labels.
Source stored bytes retain the original 417 JSON / 80 gzip mix. Canonical gzip columns
compress every member separately at level 9, mtime 0. These are different boundaries.
Package rows include the shared dependency catalogue, pinned common inputs and manifest once.
The source-preserving row additionally includes every captured original member once.
Unresolved reported digests are not fetched and their absent bytes are not counted.

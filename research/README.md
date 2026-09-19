# Full 600-cell research materials

> **Research supporting 0.4.** Reports retain their original mathematical and engineering scope; the 0.3 technical report below is not a 0.4 acceptance report. See the [0.4 release guide](../docs/RELEASE_0_4.md) for current application status.

**[Certified Control of the Full 600-Cell](Full_600cell_Technical_Report.pdf)** is the 34-page C600 Studio 0.3 technical report, typeset entirely in LaTeX with its [complete source](Full_600cell_Technical_Report.tex). It connects the retained design transcript and algorithm reference to the current workbench. The mathematical development preserves 31 numbered equations, two propositions with proofs, the complete 35-orbit census, and the independently checked counting bounds. The 0.3 engineering sections explain canonical C1-C600 navigation, exact dual-graph layers, identity-aware inspection, shared auxiliary geometry views, atomic native revision updates, and actual Windows performance observations. Historical validation, current measurements, finite mathematical checks, and their limitations remain separate. The [Puzzle theory supplement](PUZZLE_THEORY.md) and its proof-review artifacts are unchanged.

| Material | Introduction |
| --- | --- |
| [Puzzle theory](PUZZLE_THEORY.md) | New English LaTeX supplement: exact geometry, three-sphere topology, H4 and legal group actions, 15 Rethlas-reviewed lemmas, the 35-orbit census, and sound solver/rendering optimization. Includes proof, verdict, provenance, and a standalone exact geometry checker. |
| [Technical report (PDF)](Full_600cell_Technical_Report.pdf) | Develops piece/frame group actions, orientation invariants, alternating-group reachability, state-count bounds, word metrics, scramble distributions, guarded setup graphs, and nonabelian corrections. It also explains transactional history, bridge equivalence, exact filtering and picking, canonical cell and vertex incidence, inspection semantics, independent auxiliary cameras, revision deltas, log validation, process recovery, and rendering costs. |
| [Original 35-orbit algorithms paper (PDF)](reference/Full_600cell_35_Orbit_Algorithms.pdf) | The unchanged 51-page reference: one card per moving orbit, explicit seed constructions, setup/frame procedures, orientation handling, and workload analysis. Statements about untested native integration describe that historical edition. |
| [Curated reference and replay supplement (ZIP)](Full_600cell_Curated_Reference_and_Replay.zip) | Includes the original algorithm paper and LaTeX source, complete forward/inverse seed words, model/frame data, selected generated reports, and three deterministic synthetic solution histories. Its README explains replay commands and omitted historical records. |
| [Independent configuration-bound audit](audit/README.md) | A separate checker replays nine pure controllers over every labelled slot and independently checks buffer setup coverage and exact integer inequalities. Source, data hashes, and completed results are supplied. |
| [Extended derivation checker](audit/verify_extended_derivations.py) and [results](audit/extended-results.json) | Recomputes all 35 census/controller rows, structural tree and collateral-order checks, small-domain algebra identities, state-count arithmetic, a worst-case move lower bound, the short-scramble support bound, and constructive workload bounds. It does not run the application or access a session. |
| [Documentary source guide](ARCHIVAL_SOURCE_GUIDE.md) | Maps the retained technical evidence to page ranges in the privately held original conversation, without publishing personal exchanges or account-bearing links. |
| [Full Detail Rotation requirements](FULL_DETAIL_ROTATION_REQUIREMENTS.md) | Defines the 1920 x 1080 / 30 FPS qualification target and explains why no minimum GPU model is yet certified. |

The full puzzle contains **177,120 pieces and 259,800 sticker slots**. Three fresh compact replays passed with every labelled slot solved and all 35 stages checked. The companion records these new checks separately from historical timing reports. Compact macro replay is not primitive-by-primitive native animation.

The expanded analysis proves a **worst-case distance of at least 46,648 turns** in the specified 1,800-symbol lab alphabet and shows why a 1,000-step scramble from solved is far from a uniform random reachable state. These are counting results, not a minimum solution length for each scramble or a measure of human difficulty. Software sections explain both verified behavior and current limitations, including the absence of a qualified GPU minimum for continuous full-detail 1080p/30 FPS. The 0.3 measurements use the final interaction/rendering implementation before the release-version banner update. A real 1920 x 1080 backbuffer on Intel HD Graphics 620 required 622.25 ms per complete changed-camera frame on average; a short production drag using 1,500 sampled stickers submitted 31.07 FPS. Full-detail 30 FPS and instant-turn p95 below 100 ms remain unmet, and one graph-activation p95 narrowly missed its 50 ms gate. These short samples are not sustained certification of the packaged release; see the complete [performance methods and results](../docs/DEVELOPMENT_PERFORMANCE.md).

Primary credit for **Magic Puzzle Ultimate and its original renderer belongs to [Andrey Astrelin](https://superliminal.com/andrey/mpu/)**. C600 Studio is a mod. Nan Ma's solving methods and ivan216's software projects are acknowledged as background. The research includes AI-assisted analysis and is a technical report, not a peer-reviewed publication.

## Build the report

Use a current TeX Live, TinyTeX, or MiKTeX installation with the packages named in the source preamble. Compile from this directory:

```text
pdflatex -interaction=nonstopmode -halt-on-error Full_600cell_Technical_Report.tex
pdflatex -interaction=nonstopmode -halt-on-error Full_600cell_Technical_Report.tex
```

Repeat compilation if LaTeX requests another pass for long-table widths or cross-references. No external image, system font, application binary, private transcript, or shell escape is required. The original algorithm paper's separate LaTeX source is retained inside the companion archive.

## Scope and integrity

[SHA256SUMS.txt](SHA256SUMS.txt) identifies the four downloadable PDF/ZIP artifacts. The repository's [source manifest](../SOURCE_MANIFEST.json) covers the complete public source inventory; the ZIP contains its own member-level provenance and checksums.

The original conversation export, personal sessions, screenshots, raw diagnostics, third-party solve logs and derived personal-history traces are excluded. The reference paper discusses five histories; only its three synthetic seed 600/601/602 histories are included. Multi-gigabyte expanded native logs are excluded. This report accompanies **C600 Studio 0.3**. Its mathematical reference PDFs, curated replay ZIP, Puzzle Theory source/PDF, and retained proof-review records remain unchanged. Historical **0.2.4** test counts and timings are explicitly identified by their own workload and edition.

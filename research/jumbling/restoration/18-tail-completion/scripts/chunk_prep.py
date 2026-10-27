"""Stage 18: split one recorded step's full word into chunks for a twist-by-twist replay (README section 3.3). The word
is X, W, V', W^-1 (as P.Q.P^-1) and, in the four-segment form, V'^-1. The pose table at every chunk boundary is
computed by slp_apply's exact composition (each SAFE-ALL span, X, V' or V'^-1, with one alphabet built at the span's
start; W and W^-1 played twist by twist), so that the chunks can be played twist by twist in parallel
(chunk_run.py); the replay is valid only if every chunk's twist-by-twist end digest equals the next chunk's start
digest and the last equals the recorded end (chunk_verify.py). Writes c<j>.json (the chunk's word), t<j>.json (the
table after chunk j, for j < k) and manifest.json (with the SHA-256 of every chunk's word and of the whole word).
Research only, headless; exact geometry.
Usage (from the repository root): OPENBLAS_NUM_THREADS=1 python3 -I -B <this script> <start table> <step dir> <3|4>
<chunk letters> <new out dir> [<expected end digest>]"""
import hashlib
import json
import sys
from pathlib import Path
from types import SimpleNamespace

import numpy as np

S12 = Path('research/jumbling/restoration/12-low-height/scripts')
sys.path[:0] = [str(S12.resolve())]
import slp_apply as A  # noqa: E402


def sha(word):
    return hashlib.sha256(json.dumps(word).encode()).hexdigest()


def spans_of(d, form, M):
    """The step's spans in the order the record plays them, as [(kind, word)] with kind 'safe' or 'twist'; an empty X
    is kept as an empty span."""
    X = A.read_json(d / 'x.json.gz')['word'] if (d / 'x.json.gz').exists() else []
    W = A.read_json(d / 'w.json')['word']
    V = A.read_json(d / 'v.json.gz')['word']
    core = A.read_json(d / 'winv.json')
    Winv = core['P'] + core['Q'] + A.inverse_word(M, core['P'])
    parts = [('safe', X), ('twist', W), ('safe', V), ('twist', Winv)]
    if form == 4:
        parts.append(('safe', A.inverse_word(M, V)))
    return [(kind, [list(x) for x in w]) for kind, w in parts]


def main(argv):
    table, d, form, size, out = Path(argv[0]), Path(argv[1]), int(argv[2]), int(argv[3]), Path(argv[4])
    expect = argv[5] if len(argv) > 5 else None
    assert form in (3, 4) and size > 0
    out.mkdir(parents=True, exist_ok=False)
    B, screen = A.modules()
    st, M, _ = screen.load('table', str(table))
    start = st.digest()
    parts = spans_of(d, form, M)
    marks = {'short_end': sum(len(w) for _, w in parts[:4])} if form == 4 else {}  # the end of W^-1
    segs = [(kind, w) for kind, w in parts if len(w)]
    word = [x for _, w in segs for x in w]
    A.word_pairs(word, M)
    spans, pos = [], 0
    for kind, w in segs:
        spans.append((pos, pos + len(w), kind))
        pos += len(w)
    n = pos
    bounds = set()
    for b in range(size, n, size):
        for a, e, kind in spans:
            if a < b < e and kind == 'twist':
                b = e  # never cut inside a twist span
        if 0 < b < n:
            bounds.add(b)
    bounds |= {b for b in marks.values() if 0 < b < n}
    bounds = sorted(bounds)
    cuts = sorted(set(bounds) | {a for a, _, _ in spans} | {n})
    rotations = A.Rotations(st.ctx.kplus)
    G = None
    tables = {}
    for p, q in zip(cuts, cuts[1:]):
        kind, a = next((k, a) for a, e, k in spans if a <= p < e)
        piece = word[p:q]
        if kind == 'safe':
            if p == a:
                G = B.Alphabet(SimpleNamespace(st=st), M)  # one alphabet per SAFE-ALL span (it keeps the off pieces)
            numbers = np.empty(len(piece), np.int32)
            for j, pair in enumerate(piece):
                key = tuple(pair)
                assert key in G.index, f'letter {p + j}: {key} is not a SAFE-ALL letter'
                numbers[j] = G.index[key]
            A.apply_labelled(st, G, rotations, A.literal_action(G, rotations, numbers))
        else:
            A.apply_segment(st, M, {'word': piece}, 0, rotations)
        if q in bounds:
            j = bounds.index(q) + 1
            doc = B.export_pose_table(st, str(out / f't{j}.json'))
            tables[q] = doc['digest']
            print(json.dumps({'boundary': q, 'table': f't{j}.json', 'digest': doc['digest'],
                              'off': int(st.off_lattice_count())}), flush=True)
    end = st.digest()
    assert expect is None or end == expect, f'composed end {end} is not the expected end {expect}'
    edges = [0] + bounds + [n]
    chunks = []
    for j, (p, q) in enumerate(zip(edges, edges[1:]), 1):
        (out / f'c{j}.json').write_text(json.dumps({'word': word[p:q]}) + '\n')
        chunks.append({'j': j, 'from': p, 'to': q, 'letters': q - p, 'word': f'c{j}.json',
                       'word_sha256': sha(word[p:q]),
                       'start_table': str(table) if j == 1 else str(out / f't{j - 1}.json'),
                       'start_digest': start if j == 1 else tables[p], 'end_digest': tables.get(q, end)})
    manifest = {'start_table': str(table), 'start_digest': start, 'step_dir': str(d), 'form': form,
                'unit': str(d / 'unit.json'), 'letters': n, 'word_sha256': sha(word), 'end_digest': end,
                'marks': {k: {'at': b, 'digest': tables[b]} for k, b in marks.items()}, 'chunks': chunks}
    (out / 'manifest.json').write_text(json.dumps(manifest, indent=1) + '\n')
    print(json.dumps({'done': True, 'letters': n, 'chunks': len(chunks), 'end_digest': end}), flush=True)


if __name__ == '__main__':
    main(sys.argv[1:])

"""Shared exact jam loader and helpers for the stage 16 research scripts.

Run the tools from the repository root with python3 -I -B. Supplied label
caches are read-only, including on a cache miss. New caches and JSON records
belong in external scratch directories. All geometry is exact and synthetic.
"""
from functools import lru_cache
import json
import os
from pathlib import Path
import sys
import tempfile
import time
from zipfile import BadZipFile

sys.dont_write_bytecode = True
for _var in ('OPENBLAS_NUM_THREADS', 'OMP_NUM_THREADS', 'MKL_NUM_THREADS'):
    os.environ[_var] = '1'
S16 = Path(__file__).resolve().parents[1]
RESTORATION = S16.parent
sys.path[:0] = [str(RESTORATION/'15-lowering-machine'/'scripts'),
                str(RESTORATION/'14-blocking-structure'/'scripts')]
import pstar as S  # noqa: E402

P, A, BM, J, np, CLS = S.P, S.A, S.BM, S.J, S.np, S.CLS
T0 = time.process_time()


def cpu():
    """The second analyst's rounded, process-relative CPU clock."""
    return round(time.process_time() - T0, 1)


@lru_cache(maxsize=1)
def jams():
    return json.loads((S16/'jams.json').read_text())


def jam(seed):
    A.require(str(seed) in jams(), 'seed', 'expected one of the six jams')
    return jams()[str(seed)]


def jam_word(seed, word=''):
    rec = jam(seed)
    return ','.join([rec['chain'], *rec['macros'], *([word] if word else [])])


def grips(raw):
    return [P.grip(int(g)) for g in raw.split(',') if g]


def cached_labels(path, digest, pieces):
    """Read a valid digest-matched array, or return None without any write."""
    if path is None or not Path(path).is_file():
        return None
    try:
        with np.load(path, allow_pickle=False) as data:
            if str(data['digest']) != digest:
                return None
            lab = data['labels']
            if lab.shape != (pieces, 600) or lab.dtype != np.uint8 or (lab > 2).any():
                return None
            return lab
    except (OSError, ValueError, KeyError, TypeError, EOFError, BadZipFile):
        return None


def write_labels(path, lab, digest):
    """Create a new S14-format cache exclusively; never replace an input."""
    path = P.external_path(path)
    A.require(not os.path.lexists(path), 'labels exists', 'use a new cache path')
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open('xb') as stream:
        np.savez_compressed(stream, labels=lab, digest=np.array(digest))


def load(seed, word='', labels=None, save_labels=None):
    """Return the S14 State at a jam followed by word, with read-only inputs.

    The table, chronological word and trees follow t3p.load_input exactly.
    Missing, mismatching and invalid label caches are computed in memory.
    save_labels is checked before loading and created with exclusive access.
    """
    if save_labels is not None:
        save_labels = P.external_path(save_labels)
        A.require(not os.path.lexists(save_labels), 'labels exists', 'use a new cache path')
    rec = jam(seed)
    st = BM.load_table(rec['table'])
    table_digest = st.digest()
    A.require(table_digest == rec['table_digest'], 'table_digest')
    for c, i in P.parse_word(jam_word(seed, word)):
        if not J.twist(st, c, P.matrix(c, i)):
            raise ValueError(f'--word: inadmissible letter {c}:{i}')
    h, b = BM.tree_info(st)
    digest = st.digest()
    if not word:
        A.require(digest == rec['end_digest'], 'jam digest')
        A.require(J.phi(h) == rec['end_phi'], 'jam phi')
    lab = cached_labels(labels, digest, len(h))
    if lab is None:
        lab = BM.all_labels(st, workers=1)
    if save_labels is not None:
        write_labels(save_labels, lab, digest)
    state = P.State(st, lab, h, b)
    state.table_digest = table_digest
    return state


def record(kind, seed, state, word='', options=None):
    return dict(format=f's16-{kind}/1', seed=str(seed), table_digest=state.table_digest,
                word=word, digest=state.st.digest(), phi=J.phi(state.h), options=options or {})


def atomic_json(path, obj):
    """Publish a complete JSON record by a rename in the output directory."""
    path = P.external_path(path)
    path.parent.mkdir(parents=True, exist_ok=True)
    fd, temp = tempfile.mkstemp(prefix=path.name+'.', suffix='.tmp', dir=path.parent)
    try:
        with os.fdopen(fd, 'w') as stream:
            json.dump(obj, stream, indent=1, allow_nan=False)
            stream.write('\n')
        os.replace(temp, path)
    finally:
        if os.path.exists(temp):
            os.unlink(temp)


@lru_cache(maxsize=None)
def _mats(c):
    return {P.matrix(c, i): i for i in range(60)}


def prod(c, a, b):
    """The item a.b at grip c (b first, then a)."""
    return _mats(c)[A.to_tuple(A.matmul(P.matrix(c, a), P.matrix(c, b)))]


def inv(c, i):
    return A.inverse_item(c, i)


def pre(c, i):
    """pre[x] is i^-1 x, the retained column-reading convention."""
    return A.retained(c, i)[1]

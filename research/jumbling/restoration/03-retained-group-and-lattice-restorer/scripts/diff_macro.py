"""Correction macros from two endings of one start (card 2, Lemma 2; card 5, S).

Let E be any configuration and let a, b be words with E a and E b on the lattice. Then
D = a^-1 b is admissible from E a and returns to the lattice; since E a and the solved state have
the same decorated shape (both are lattice configurations), D is admissible from solved and ends
on the lattice, and it acts on every reachable lattice configuration by one fixed relabelling.
E need not be on the lattice or reachable for this. nu(D) is computed here directly from the end
state of D replayed from solved, so the macro does not depend on the tables' own nu values.

usage: diff_macro.py <scriptdir> <repo root> <research/jumbling> <out json>
                     <name>=<word.json>[+<word.json>...] <name>=<word.json>[+...] ...
The first ending is the reference; one macro is written for every other ending.
Word files hold {"word": [[grip, menu index], ...]} in the I_a menu.
"""
import json
import sys
import time
from pathlib import Path

import numpy as np

sys.path.insert(0, sys.argv[1])
from nucoords import NuCoords                   # noqa: E402

ROOT, J, OUT = sys.argv[2], Path(sys.argv[3]).resolve(), Path(sys.argv[4])
for p in (J / 'theory', J, J / 'fixtures'):
    sys.path.insert(0, str(p))
sys.dont_write_bytecode = True
import restore as RS                            # noqa: E402
from sim.state import State                     # noqa: E402
from sim.twists import Twist                    # noqa: E402

T0 = time.time()
ctx = RS.ctx
M = RS.menu('I_a')
NC = NuCoords(ROOT)

endings = []
for arg in sys.argv[5:]:
    name, files = arg.split('=', 1)
    word = []
    for f in files.split('+'):
        word += [(int(c), int(i)) for c, i in json.loads(Path(f).read_text())['word']]
    endings.append((name, word))


def tw(c, i):
    return Twist(ctx, c, RS.element(M, c, i))


ref_name, ref = endings[0]
macros = []
for name, word in endings[1:]:
    ts = [tw(c, i).inverse() for c, i in reversed(ref)] + [tw(c, i) for c, i in word]
    st = State(ctx, menu=M)
    for k, t in enumerate(ts):
        out = st.apply(t)
        if not out.applied:
            raise SystemExit(f'{name}: twist {k} not admissible from solved ({out.status})')
    if not st.is_lattice():
        raise SystemExit(f'{name}: ends with {st.off_lattice_count()} pieces off the lattice')
    nu = NC.nu(np.asarray(st.lattice_stickers()['labels'], np.int64))
    macros.append({'id': len(macros), 'family': f'diff:{ref_name}->{name}',
                   'records': [t.record() for t in ts], 'length': len(ts),
                   'nu': {str(o): list(v) for o, v in nu.items()}, 'moved': int(st.moved_count()),
                   'end_digest': st.digest()})
    print(json.dumps({'macro': f'{ref_name}->{name}', 'length': len(ts), 'moved': int(st.moved_count()),
                      'nu': macros[-1]['nu'], 'sec': round(time.time() - T0)}), flush=True)
OUT.write_text(json.dumps({'format': 'I_a-diff-macros-v1', 'library': macros}))

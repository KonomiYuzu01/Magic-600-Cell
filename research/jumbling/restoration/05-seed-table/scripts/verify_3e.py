"""Independent replay check of the 3E stage outputs (menu I_a), using only the simulator and the files:
 1. every seed of the seed library: replayed from solved, every twist admissible, exactly one off piece, its
    position class equals the recorded class, exactly one hole equal to the recorded hole;
 2. every k-seed: replayed from solved, the recorded number of off pieces and the recorded holes;
 3. every pipeline run with steps: the input pose table is loaded (digest checked), every recorded preparation and
    restoration word is applied in order (every twist admissible), and the state digest after each step equals the
    recorded end digest; the final state has the recorded number of off pieces.
usage: verify_3e.py <research/jumbling> <classes.json> <arrays.npz> <seeds json> <kseeds json> <out json>
                    <run json> <input table> [<run json> <input table> ...]"""
import json, sys, time
from pathlib import Path
import numpy as np
sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
from e3lib import Lib  # noqa: E402
t0 = time.time()
L = Lib(sys.argv[1], sys.argv[2], sys.argv[3])
from sim.kplus import matrix_from_json  # noqa: E402
from sim.state import Pose  # noqa: E402
TW = {}


def tw(l):
    l = tuple(l)
    if l not in TW:
        TW[l] = L.sim.Twist(L.ctx, l[0], L.RS.element(L.M, l[0], l[1]), 'menu', {'menu': L.M.name})
    return TW[l]


def from_solved(word):
    st = L.sim.State(L.ctx, menu=L.M)
    for l in word:
        if not st.apply(tw(l), journal=False).applied:
            return None
    return st


res = {'seeds': {'checked': 0, 'failed': []}, 'kseeds': {'checked': 0, 'failed': []}, 'runs': []}
for q in json.load(open(sys.argv[4])):
    st = from_solved(q['word'])
    ok = st is not None
    if ok:
        lat = L.lattice_chambers(st)
        off = np.flatnonzero(lat < 0)
        holes = L.vacancies(st).tolist()
        ok = len(off) == 1 and holes == [q['hole']]
        if ok:
            c = L.canon(int(off[0]), L.fm(st.pose(int(off[0]))))
            ok = c is not None and int(L.cls_of[c[2]]) == q['class']
    res['seeds']['checked'] += 1
    if not ok:
        res['seeds']['failed'].append(q['name'])
print('seeds', res['seeds']['checked'], 'failed', len(res['seeds']['failed']), '%.0fs' % (time.time() - t0), flush=True)
for ks in json.load(open(sys.argv[5])):
    st = from_solved(ks['word'])
    ok = st is not None and sorted(L.vacancies(st).tolist()) == sorted(ks['holes'])
    res['kseeds']['checked'] += 1
    if not ok:
        res['kseeds']['failed'].append(ks['name'])
print('kseeds', res['kseeds']['checked'], 'failed', res['kseeds']['failed'], flush=True)
args = sys.argv[7:]
for run_path, table in zip(args[0::2], args[1::2]):
    run = json.load(open(run_path))
    doc = json.loads(Path(table).read_text())
    st = L.sim.State(L.ctx, menu=L.M)
    st._poses, st._lookup = [Pose(0, None, ('K', 0))], {('K', 0): 0}
    ids = np.array([st._intern_matrix(matrix_from_json(m)) for m in doc['poses']], np.int32)
    st.pose_id = ids[np.asarray(doc['pose_id'], np.int64)]
    st._compact()
    st.journal, st._anchor_cache = [], {}
    row = {'run': Path(run_path).name, 'input_digest_ok': st.digest() == doc['digest'] == run['summary']['input_digest'],
           'steps': len(run['steps']), 'step_digests_ok': True, 'admissible': True}
    for s in run['steps']:
        for l in s.get('prep', []) + s.get('word', []):
            if not st.apply(tw(l), journal=False).applied:
                row['admissible'] = False
                break
        if not row['admissible']:
            break
        if st.digest() != s.get('end_digest'):
            row['step_digests_ok'] = False
    row['end_digest'] = st.digest()
    row['end_matches_summary'] = run['summary'].get('end_digest') in (None, st.digest())
    row['off_end'] = int((st._kpose()[st.pose_id] < 0).sum())
    row['off_end_matches'] = row['off_end'] == run['summary']['remaining']
    res['runs'].append(row)
    print(row, flush=True)
res['seconds'] = round(time.time() - t0, 1)
json.dump(res, open(sys.argv[6], 'w'), indent=1)

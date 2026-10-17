# E-B6: independent replay of the strict four-letter words that eb6.py found. For each unit of an eb6.py output with
# strict words, every word is replayed on a fresh copy of the state: every twist must be admissible, every piece
# outside the unit must end at its exact pose (compared by exact pose keys, lattice pieces included), the unit's
# pieces must end at one common pose, and its left height after the word is recomputed. The state is rebuilt from
# its source for each unit and checked against the digest that eb6.py recorded.
# Exact geometry; research only, headless Linux; no Windows, DirectX, input or performance claim.
# Run from the repository root:
#   python3 -I -B eb6_replay.py <eb6 output json> <state source arguments as given to eb6.py> <out json>
# for example: python3 -I -B eb6_replay.py eb6-X.json witness <H6030.json> 156 <out json>
import json
import sys
import time
from pathlib import Path

import numpy as np

REPO = Path.cwd()
sys.path[:0] = [str(REPO / 'research/jumbling/theory'), str(REPO / 'research/jumbling'),
                str(REPO / 'research/jumbling/fixtures'), str(Path(__file__).resolve().parent)]
import restore as R  # noqa: E402
import sim  # noqa: E402
import trees as T  # noqa: E402

T0 = time.time()


def build(args):
    """The state of eb6.py's source arguments, rebuilt independently of eb6.py."""
    mode = args[0]
    if mode == 'record':
        from screen import load as screen_load
        st, M, _ = screen_load('record', args[1])
        return st, M
    C = sim.get_context()
    if mode == 'fixture':
        import wj
        fx, M = wj.load(C, 'I_a')
        st = sim.State(C, menu=M)
        for rec in fx['journal']['records'][:int(args[1])]:
            assert st.apply(sim.Twist.from_record(C, rec)).applied
        return st, M
    if mode == 'witness':
        M = R.menu('I_a')
        word = json.loads(Path(args[1]).read_text())['word']
        st = sim.State(C, filtered=False, menu=M)
        for c, j in word[:int(args[2])]:
            o = st.apply(sim.Twist(C, c, R.element(M, c, j), 'menu', {'menu': M.name}), journal=False)
            assert o.applied and not o.uncertain
        return st, M
    raise SystemExit(f'unknown mode {mode}')


def keys(st):
    return [st._poses[i].key for i in st.pose_id.tolist()]


def main():
    src = json.loads(Path(sys.argv[1]).read_text())
    args, out_path = sys.argv[2:-1], Path(sys.argv[-1])
    st, M = build(args)
    assert st.digest() == src['digest'], 'the rebuilt state differs from the screened one'
    st.journal = []
    k0 = keys(st)
    off0 = np.flatnonzero(st._kpose()[st.pose_id] < 0)
    hl0 = {}
    for pid in np.unique(st.pose_id[off0]).tolist():
        hl0[pid] = int(T.dist(T.factors(st._pose_matrix(pid))[0]))
    rows = []
    for u in src['units']:
        words = u.get('comm4', {}).get('strict_words', []) + u.get('comm4', {}).get('strict_words_other', [])
        if not words:
            continue
        U = u['piece_ids']
        pid = int(st.pose_id[U[0]])
        assert all(int(st.pose_id[p]) == pid for p in U) and hl0[pid] == u['height']
        for w in words:
            ok_adm = True
            done = 0
            for c, j in w['word']:
                o = st.apply(sim.Twist(st.ctx, c, R.element(M, c, j), 'menu', {'menu': M.name}))
                assert not o.uncertain
                if not o.applied:
                    ok_adm = False
                    break
                done += 1
            row = {'unit': u['first'], 'word': w['word'], 'admissible': ok_adm}
            if ok_adm:
                k1 = keys(st)
                moved = [p for p in range(len(k1)) if k1[p] != k0[p]]
                row['moved'] = len(moved)
                row['moved_first'] = moved[:3]
                Ub = st.pose_id[moved]
                row['moved_one_pose'] = bool(len(moved) and (Ub == Ub[0]).all())
                row['moved_is_unit'] = moved == U
                if row['moved_one_pose']:
                    hl, hr, bl, _ = T.pose_tree(st._pose_matrix(int(Ub[0])))
                    row.update(height_after=int(hl), right_height_after=int(hr), branch_after=int(bl))
                row['ok'] = (row['moved_is_unit'] and row['moved_one_pose']
                             and row.get('height_after') == w['height_after'])
            else:
                row['ok'] = False
            rows.append(row)
            for _ in range(done):
                st.undo()
            assert st.digest() == src['digest']
    res = {'source': src['source'], 'digest': src['digest'], 'words': len(rows),
           'ok': sum(1 for r in rows if r['ok']), 'rows': rows, 'seconds': round(time.time() - T0, 1)}
    out_path.write_text(json.dumps(res, indent=1) + '\n')
    print(json.dumps({k: v for k, v in res.items() if k != 'rows'}))


if __name__ == '__main__':
    main()

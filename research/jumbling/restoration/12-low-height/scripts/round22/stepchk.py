# Round 22, line (c), the adjudicating experiments: replay one step word from a pose table and check the strict' contract
# for a unit given by its pieces (every twist admissible; the unit on the lattice at the end; no lattice piece of the
# start leaves the lattice; every other off piece of the start on the lattice or at its exact pose). Optionally
# exports the end state as a pose table for the next step of a chain. Exact; headless; research only.
# Usage (from the repository root): python3 -I -B stepchk.py <table> <unit json> <word json> <out json> [<end table>]
import json
import sys
from pathlib import Path

REPO = Path.cwd()
J = REPO / 'research/jumbling'
sys.path[:0] = [str(J / 'theory'), str(J), str(J / 'fixtures')]
import buffer as B  # noqa: E402

O = B.O


def pose(s, p):
    return B.matrix_json(s._pose_matrix(int(s.pose_id[p])))


def main():
    table, ufile, wfile, outp = sys.argv[1:5]
    endt = sys.argv[5] if len(sys.argv) > 5 else None
    U = set(int(p) for p in json.loads(Path(ufile).read_text())['pieces'])
    W = json.loads(Path(wfile).read_text())
    word = W['word']
    R, M = O.start(table)
    st = R.st
    z = st.digest()
    if 'input_digest' in W:
        assert W['input_digest'] == z, 'not a word at this state'
    off0 = B.off_pieces(st).tolist()
    pose0 = {p: pose(st, p) for p in off0}
    for c, i in word:
        R.apply(int(c), int(i))                     # raises when a twist is not admissible
    off1 = set(B.off_pieces(st).tolist())
    others = [p for p in off0 if p not in U]
    kept = sum(1 for p in others if p in off1 and pose(st, p) == pose0[p])
    landed = sum(1 for p in others if p not in off1)
    row = {'start_digest': z, 'end_digest': st.digest(), 'letters': len(word), 'unit_size': len(U),
           'unit_on_lattice': sum(1 for p in U if p not in off1), 'collateral': len(off1 - set(off0)),
           'others': len(others), 'others_exact': kept, 'others_landed': landed, 'off_before': len(off0),
           'off_after': len(off1)}
    row['strict_prime'] = (row['unit_on_lattice'] == len(U) and row['collateral'] == 0
                           and kept + landed == len(others))
    if 'output_digest' in W:
        row['output_digest_matches'] = W['output_digest'] == row['end_digest']
    if endt:
        doc = B.export_pose_table(st, endt)
        row['end_table_digest'] = doc['digest']
    Path(outp).write_text(json.dumps(row, indent=1) + '\n')
    print(json.dumps(row), flush=True)


if __name__ == '__main__':
    main()

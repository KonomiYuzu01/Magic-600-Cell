# E-B4: the exact-pose filter of cellpure4.py at each closure, from the work directory of eb4_close.py. For every unit
# k, u<k>/cp3.json holds the digest of the state Y it read, the model-passing solutions that it replayed exactly, in
# order (solution index, word length, other off pieces kept at their exact pose, unit pieces on the lattice after),
# and the exact summary of the word it kept. This script copies those fields, numbers the tries from 1 and marks the
# accepted one (the first that lands the unit and keeps every other off pose).
# Usage (from the repository root):
#   python3 -I -B eb4_tries.py <eb4_close work dir> <out json>
import json
import sys
from pathlib import Path


def main():
    work, outp = Path(sys.argv[1]), Path(sys.argv[2])
    rows = []
    for d in sorted(work.glob('u[0-9]*'), key=lambda p: int(p.name[1:])):
        rec = json.loads((d / 'cp3.json').read_text())
        ex = rec['exact']
        tries = [{'try': i + 1, 'solution': t['solution'], 'word_len': t['word_len'],
                  'others_exact_pose_kept': t['others_exact_pose_kept'], 'unit_on_after': t['carrier_on_after']}
                 for i, t in enumerate(rec['exact_tries'])]
        ok = [t for t in tries
              if t['others_exact_pose_kept'] == ex['others_off'] and t['unit_on_after'] == ex['carrier_size']]
        assert ok and ok[0] is tries[-1], f'{d.name}: the kept word is not the first passing try'
        assert ok[0]['word_len'] == rec['word_len'] == len(rec['word'])
        rows.append({'unit': int(d.name[1:]), 'Y_digest': rec['digest'], 'unit_size': ex['carrier_size'],
                     'others_off': ex['others_off'], 'tries': tries, 'accepted_try': ok[0]['try'],
                     'word_len': rec['word_len'],
                     'exact': {k: ex[k] for k in ('all_admissible', 'off_before', 'off_after', 'digest_after',
                                                  'others_exact_pose_kept', 'lattice_pieces_in_X_before',
                                                  'newly_off_eq_lattice_pieces_of_X')}})
    res = {'closures': len(rows), 'filtered': sum(1 for r in rows if r['accepted_try'] > 1),
           'rejected': [{'unit': r['unit'], 'rejected_tries': r['accepted_try'] - 1,
                         'others_changed': sorted({r['others_off'] - t['others_exact_pose_kept']
                                                   for t in r['tries'][:-1]}),
                         'accepted_try': r['accepted_try']} for r in rows if r['accepted_try'] > 1],
           'rows': rows}
    outp.write_text(json.dumps(res, indent=1) + '\n')
    print(json.dumps({k: v for k, v in res.items() if k != 'rows'}))


if __name__ == '__main__':
    main()

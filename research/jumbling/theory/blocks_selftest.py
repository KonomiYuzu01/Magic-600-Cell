"""Exact synthetic-geometry acceptance checks for blocks.py; all outputs are temporary.

Run from the repository root: python research/jumbling/theory/blocks_selftest.py
The long part replays the certified e-chain words on the exact simulator.
"""
import json
import shutil
import sys
import tempfile
import time
from collections import Counter
from pathlib import Path

sys.dont_write_bytecode = True

import numpy as np

import blocks as B
from sim.kplus import matrix_json


def check(condition, message):
    if not condition:
        raise AssertionError(message)


def pose_table(st, path):
    path.write_text(json.dumps({'poses': [matrix_json(st._pose_matrix(i)) for i in range(len(st._poses))],
                                'pose_id': st.pose_id.tolist(), 'digest': st.digest()}) + '\n')


def check_tree_file(path, count, root):
    rows = np.load(path, allow_pickle=False)
    check(rows.dtype.names == ('key', 'parent_key', 'native_grip', 'native_matrix_index'),
          'predecessor file columns')
    check(len(rows) == count, 'predecessor file must contain every reached state')
    check(int(rows[0]['key']) == root and int(rows[0]['parent_key']) == -1, 'predecessor root')
    seen = {root}
    for row in rows[1:]:
        key, parent = int(row['key']), int(row['parent_key'])
        check(parent in seen and key not in seen, 'predecessor must precede a new child')
        check(0 <= int(row['native_grip']) < 600, 'native grip range')
        check(0 <= int(row['native_matrix_index']) < 7200, 'native matrix range')
        seen.add(key)


def main():
    t0 = time.time()
    tmp = Path(tempfile.mkdtemp(prefix='blocks-selftest-'))
    old_cache = B.O.seams.CACHE
    B.O.seams.CACHE = tmp / 'home_centroids.npy'
    group = None
    try:
        # A straddling intermediary joins two pieces on opposite sides; a separate
        # cut still separates the fourth piece. This checks connected components,
        # rather than grouping equal inside signatures.
        components = B.rigid_components(np.array([10, 20, 30, 40]),
                                        np.array([[1, 0], [2, 0], [0, 0], [0, 1]], np.uint8))
        check([c.tolist() for c in components] == [[10, 20, 30], [40]], 'straddling bridge')
        check(B.rigid_components(np.array([], np.int64), np.empty((0, 600), np.uint8)) == [],
              'empty off-lattice set')

        R, M = B.O.start()
        model = B.Blocks(R, M)
        expected_sizes = [48, 43, 17, 3, 50, 12, 48, 29, 45, 86, 111, 87, 111]
        check(len(model.off) == 690, 'E2 must have 690 off-lattice pieces')
        check([len(b) for b in model.blocks] == expected_sizes, 'E2 rigid block sizes/order')
        check(len(model.straddling_grips) == 26, 'E2 straddling grip count')
        check(not (model.status == 3).any(), 'E2 status 3')
        B.log(selftest='E2 components', sizes=expected_sizes)

        group = B.GroupTables(M, tmp)
        expected_reached = [7086, 1, 7185, 1, 7174, 7132, 7170, 1, 1, 1, 1, 1, 1]
        trees = []
        for b, count in enumerate(expected_reached):
            tree = model.isolated(b, group)
            check(tree.complete and tree.count == count, f'E2 isolated block {b}: {tree.summary()}')
            check(tree.longest == (6 if count > 1 else 0), f'E2 isolated block {b} diameter')
            trees.append(tree)
        B.log(selftest='E2 isolated transport', reached=[t.count for t in trees])

        for b, count in ((2, 7185), (5, 7132)):
            tree = model.coupled((b,), group)
            check(tree.complete and tree.count == count, f'E2 singleton coupled block {b}')
            check(set(tree.records['key'][:tree.count].tolist()) ==
                  set(trees[b].records['key'][:trees[b].count].tolist()), 'singleton reached multipliers')
            check(tree.summary()['states_with_only_this_block_moved'][str(b)] == count - 1,
                  'singleton return-to-identity slice')
        capped = model.coupled((2,), group, max_states=1)
        check(capped.count == 1 and not capped.complete, 'state limit retains the root and reports truncation')
        frozen = model.coupled((1,), group, max_states=1)
        check(frozen.count == 1 and frozen.complete, 'exhausted frozen tree is complete at the limit')

        tree_path = tmp / 'isolated-2.npy'
        trees[2].save(tree_path)
        check_tree_file(tree_path, 7185, trees[2].root)
        checks = model.replay(trees[2], group, 2, seed=7)
        check(len(checks) == 2 and all(r['passed'] for r in checks), 'E2 block 2 physical replay')
        check(checks[0]['length'] == 6, 'farthest state must be replayed first')
        check(R.st.digest() == model.digest, 'E2 replay must undo to the start')
        B.log(selftest='E2 physical replay', lengths=[r['length'] for r in checks])

        # Exercise the real start/table/word seam. Replaying these files is the
        # independent certificate for the 277-piece state, not a stored pose fixture.
        e2_table = tmp / 'e2-table.json'
        pose_table(R.st, e2_table)
        words = sorted((B.HERE / 'results' / 'e-chain').glob('*.json'))
        check(len(words) == 6, 'six e-chain files required')
        words.append(B.HERE / 'results' / 'e-chain-104' / '07-word-277-preparation.json')
        R277, _ = B.O.start(e2_table, [str(w) for w in words])
        check(R277.st.off_lattice_count() == 277, 'prepared state off-lattice count')
        check(R277.st.digest().startswith('3c086301091a'), 'prepared state digest')
        table277 = tmp / '277-table.json'
        pose_table(R277.st, table277)

        # Exercise the CLI, JSON and saved coupled predecessor tree together.
        output = tmp / '277-blocks.json'
        B.main(['--table', str(table277), '--coupled', '4,6', '--max-states', '30000',
                '--replay', '3', '--seed', '7', '--out', str(output)])
        doc = json.loads(output.read_text())
        check(doc['source_digest'] == R277.st.digest(), 'CLI source digest')
        check(doc['off_lattice'] == 277, 'CLI off-lattice count')
        check([b['size'] for b in doc['blocks']] == [43, 17, 3, 12, 29, 86, 87], '277 block sizes/order')
        check(not doc['status_3'], '277 status 3')
        expected_locked = {2: [57073, 57074, 57075], 3: [58358, 58361],
                           5: [119124, 119141, 119151, 128877, 128888]}
        for b, row in enumerate(doc['blocks']):
            locked = expected_locked.get(b, [])
            check(row['locked_chambers'] == locked, f'277 locked chambers of block {b}')
            check(row['vacant_locked_chambers'] == (locked if b == 5 else []),
                  f'277 locked vacancies of block {b}')
        moving = doc['admissible_block_moving_grips']
        check(len(moving) == 17, '277 admissible block-moving grips')
        check(Counter(tuple(r['inside_blocks']) for r in moving) ==
              Counter({(0, 1): 1, (0, 1, 2): 5, (1,): 1, (3,): 4,
                       (3, 5, 6): 1, (4, 5, 6): 3, (4, 6): 2}), '277 inside-set multiplicities')
        coupled = doc['coupled']
        check(coupled['blocks'] == [4, 6] and coupled['states'] == 30000 and not coupled['complete'],
              '277 coupled state limit')
        check(len(coupled['replay']) == 3 and all(r['passed'] for r in coupled['replay']),
              '277 coupled physical replay')
        check_tree_file(output.parent / coupled['predecessor_file'], 30000, coupled['root_key'])
        print(json.dumps({'result': 'PASS', 'checks': 7, 'seconds': round(time.time() - t0, 1)}), flush=True)
    finally:
        if group is not None:
            group.close()
        B.O.seams.CACHE = old_cache
        shutil.rmtree(tmp)


if __name__ == '__main__':
    main()

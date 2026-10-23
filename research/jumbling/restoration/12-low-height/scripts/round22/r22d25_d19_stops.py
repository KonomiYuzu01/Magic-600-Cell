# Theory round 22, line (d), after round 6: the start tables of the R22D-25 kernel runs with the real D19 program at six
# height-4 stops. Each table is rebuilt from committed files only. The script reads one census record, plays its history
# and all its descent twists (every twist admissible and certain) and checks the stop digest and the record's off count.
# Then it plays a fixed prefix of letters taken from committed runs (each admissible and certain), checks the digests
# listed below on the way and the end digest at the end, and exports the state as a pose table (the format of
# orient.set_pose_table, written by buffer.export_pose_table). It reloads the table exactly as r22d25_search.load_start
# does with --table and checks the digest and the full piece count again.
# The six states (record paths below; letters and digests from these committed outputs):
#   bbef1c8e  seed 7302, stage 11 runs/heights/ia-k3000-s7302.json; R22D-75's seven letters (runs/round22/
#             r22d75_height_first.txt) and the five letters of R22D-80 that the rule of R22D-81 chooses
#             (runs/round22/r22d81_rule.txt, r22d80_check.txt).
#   2430f4cc  the same record and twelve letters, then the 13 letters of the cloud height-4 line
#             (runs/round22/cloud/height4-descent-bbef1c8e/end_summary.txt).
#   ac127dc3, cc5ef7ec, 16c4aefe, 895b45de  the ends of R22D-75 at the cloud census stops of seeds 7304, 7305, 7306 and
#             7307 (runs/round22/cloud/census-k3000-s730x/, R22D-75's output there); R22D-81 hands over at each with
#             no letter.
# It prints LOADED (the record, its twist count, the stop digest, its off count and CPU seconds after the record replay)
# and TABLE (the end digest, its off count after the reload, the prefix length, the output file name, its sha256 and CPU
# seconds). It writes only <out>, apart from the cache of theory/seams.py (below). About 25 CPU minutes per table (1,378
# to 1,628 CPU seconds in the integrator's runs), nearly all of it the replay of the record.
# theory/seams.py keeps its cache in work/jumbling-cache/ unless JUMBLING_HOME_CACHE names a file.
# Its checks are assert statements, so it refuses to run under python -O.
# The integrator's script. Research only, headless Linux; exact geometry.
# Run from the repository root: OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 python3 -I -B -u research/jumbling/restoration/12-low-height/scripts/round22/r22d25_d19_stops.py table bbef1c8e <dir>/bbef1c8e-table.json
import hashlib
import json
import sys
import time
from pathlib import Path

if not __debug__:
    sys.exit('r22d25_d19_stops.py: run without -O; every check of this script is an assert statement')
sys.path[:0] = ['research/jumbling/restoration/12-low-height/scripts/round22',
                'research/jumbling/restoration/12-low-height/scripts', 'research/jumbling/theory',
                'research/jumbling', 'research/jumbling/fixtures']
import r22d25_state as K

RUNS11 = 'research/jumbling/restoration/11-scramble-census/runs/'
CLOUD = 'research/jumbling/restoration/12-low-height/runs/round22/cloud/'
STOP_7302 = '015d839ecbb37bbdde74ae9391d5d7f274dea7fee4b4fdf0092d224eef907a1f'
R75_7302 = [[64, 15], [185, 14], [357, 12], [471, 12], [263, 14], [407, 13], [523, 12]]
R80 = [[124, 2], [274, 13], [46, 1], [46, 3], [104, 14]]
H4_LINE = [[23, 14], [357, 1], [357, 3], [593, 3], [593, 43], [541, 2], [593, 54], [482, 15], [553, 5], [535, 3],
           [535, 6], [363, 10], [535, 8]]
S7 = '1ff0061fc5863c8ccbc1d734b003ff49afa5b58969b0031e51fd1e5cf1a07d81'
BBEF = 'bbef1c8ec0de78dead1473322a8c1236e48f9ccfe0017ce724461bb7f37ae299'

# state: (census record, stop digest, prefix, {letters played: digest}, end digest)
STATES = {
    'bbef1c8e': (RUNS11 + 'heights/ia-k3000-s7302.json', STOP_7302, R75_7302 + R80, {7: S7}, BBEF),
    '2430f4cc': (RUNS11 + 'heights/ia-k3000-s7302.json', STOP_7302, R75_7302 + R80 + H4_LINE, {7: S7, 12: BBEF},
                 '2430f4cc99ea121a6d3476ec21b66fc401e277d13809ad5e1c979e6e0abe1472'),
    'ac127dc3': (CLOUD + 'census-k3000-s7304/ia-k3000-s7304.json',
                 '7e2304b2a4a053b8714dd806e43ff64c2de9e11f5ef2258f3b8a65c9c1a936d4',
                 [[170, 12], [395, 12], [500, 12], [540, 12], [217, 15], [148, 15]], {},
                 'ac127dc3488832b1d1b75a043d63e84b75addd1f4e73fe7e961407aea2ff0980'),
    'cc5ef7ec': (CLOUD + 'census-k3000-s7305/ia-k3000-s7305.json',
                 '273eab69dee9e3a2f4a3997a75f6b8d9ad16814fa26b941bf7a219247f9bab6d',
                 [[10, 12], [10, 15], [67, 14], [134, 13], [195, 14], [233, 15], [421, 12], [421, 12], [484, 15],
                  [122, 12], [175, 14]], {},
                 'cc5ef7ecf0510ea9a74fc52bafe324d6d8f58294c4b0e20b40b97b414dada062'),
    '16c4aefe': (CLOUD + 'census-k3000-s7306/ia-k3000-s7306.json',
                 '9ebdc4904438ffec5eb18ec35f71ee01d3e037cae70587a786335ce095b3c83e',
                 [[208, 12], [76, 13], [90, 12], [109, 12], [233, 15], [387, 15], [485, 13], [403, 13], [512, 15]], {},
                 '16c4aefe2f15a043693937cae1d320b8a820e2151a744cf9ad8df9d11e36d96b'),
    '895b45de': (CLOUD + 'census-k3000-s7307/ia-k3000-s7307.json',
                 'ba5371aebe00ac9411567545fe79e90fa3d2487d000d2c26d45613924d4e33ff',
                 [[275, 14], [516, 13], [50, 15], [474, 13], [494, 15], [147, 14]], {},
                 '895b45de8a9a83604ca8bf988a3c9a287e404e59bd22a5004703d0852b252507'),
}

E = K.E
if len(sys.argv) != 4 or sys.argv[1] != 'table' or sys.argv[2] not in STATES:
    sys.exit('usage: r22d25_d19_stops.py table {' + ','.join(STATES) + '} <out>')
name, out = sys.argv[2], sys.argv[3]
source, stop, prefix, checkpoints, expect = STATES[name]
t0 = time.process_time()
C = E.sim.get_context()
M = E.R.menu('I_a')


def play(st, letters):
    for c, j in letters:
        o = st.apply(E.sim.Twist(C, c, E.R.element(M, c, j), 'menu', {'menu': M.name}), journal=False)
        assert o.applied and not o.uncertain, (c, j)


r = json.loads(Path(source).read_text())
st = E.sim.State(C, filtered=False, menu=M)
play(st, r['history'] + r['moves'])
assert st.digest() == stop, st.digest()
assert int(st.off_lattice_count()) == r['off_now']
print('LOADED', json.dumps({'record': source, 'twists': len(r['history']) + len(r['moves']), 'digest': stop,
                            'off': int(st.off_lattice_count()), 'cpu': round(time.process_time() - t0, 1)}), flush=True)
for n, (c, j) in enumerate(prefix, 1):
    play(st, [(c, j)])
    if n in checkpoints:
        assert st.digest() == checkpoints[n], (n, st.digest())
assert st.digest() == expect, st.digest()
st.journal = []
K.B.export_pose_table(st, out)
# Reload exactly as r22d25_search.load_start does with --table.
again = E.sim.State(C, filtered=False, menu=M)
K.orient.set_pose_table(again, json.loads(Path(out).read_text()))
assert again.digest() == expect
assert len(again.pose_id) == len(again.ctx.data.sigbits)
print('TABLE', json.dumps({'digest': expect, 'off': int(again.off_lattice_count()), 'prefix': len(prefix),
                           'out': Path(out).name, 'sha256': hashlib.sha256(Path(out).read_bytes()).hexdigest(),
                           'cpu': round(time.process_time() - t0, 1)}), flush=True)

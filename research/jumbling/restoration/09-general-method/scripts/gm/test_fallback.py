"""End-to-end test of the fallback path of gdrive.py: one jumble twist at cap 0 of the solved state, then the driver with
--fallback-only. The fallback must find a clean step of length 1 at cap 0, the run must reach the lattice, and
verify_u.py must accept the chain. Takes about two minutes. Run: python3 -I -B test_fallback.py
Research only: headless Linux, exact synthetic geometry; no Windows, DirectX, input or performance claim.
"""
import json
import subprocess
import sys
import tempfile
from pathlib import Path

sys.dont_write_bytecode = True
HERE = Path(__file__).resolve().parent
J = HERE.parents[3]
sys.path[:0] = [str(J / 'theory'), str(J)]
import buffer  # noqa: E402
import restore  # noqa: E402
import sim  # noqa: E402


def main():
    M = restore.menu('I_a')
    st = sim.State(restore.ctx, menu=M)
    j = restore.jumble_items(M)[0]
    assert st.apply(sim.Twist(restore.ctx, 0, restore.element(M, 0, j), 'menu', {'menu': M.name})).applied
    off = st.off_lattice_count()
    with tempfile.TemporaryDirectory() as tmp:
        tmp = Path(tmp)
        buffer.export_pose_table(st, str(tmp / 'start.json'))
        cmd = [sys.executable, '-I', '-B', str(HERE / 'gdrive.py'), str(J), str(HERE.parent / 'echain'),
               str(tmp / 'start.json'), 'fbtest', str(tmp / 'work'), '--fallback-only']
        r = subprocess.run(cmd, capture_output=True, text=True)
        rows = [json.loads(x) for x in (tmp / 'work' / 'gdrive.jsonl').read_text().splitlines() if x.strip()]
    ev = {}
    for row in rows:
        ev.setdefault(row['event'], []).append(row)
    fb = ev['fallback'][0]
    steps = ev['step']
    end = ev['off_lattice_phase_end'][0]
    ver = ev['verify_u'][0]
    checks = {'driver exit 0': r.returncode == 0,
              'start off pieces': ev['state'][0]['off'] == off > 0,
              'fallback found a length-1 word': fb['result'] == 'found' and fb['length'] == 1,
              'one fallback step': len(steps) == 1 and steps[0]['kind'] == 'fallback' and steps[0]['off_after'] == 0,
              'reached the lattice': end['reached_lattice'] is True and ev['state'][-1]['off'] == 0,
              'verify_u accepts the chain': ver['all_ok'] is True and ver['final_off'] == 0}
    print(json.dumps({'off_at_start': off, 'jumble_item': j, 'fallback': {k: fb[k] for k in ('length', 'tried')},
                      'checks': checks}))
    return 0 if all(checks.values()) else 1


if __name__ == '__main__':
    sys.exit(main())

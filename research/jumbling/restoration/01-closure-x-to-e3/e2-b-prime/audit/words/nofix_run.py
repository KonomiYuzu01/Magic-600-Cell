"""Adjudicating run: a constructor with the W-J fixture removed from O.start.

orient.start(table) normally calls seams._fixture_state('I_a', 40), which reads research/jumbling/fixtures/wj-I_a.json,
builds the menu from its record and replays its first 40 journal records before the pose table replaces the state.
Here seams._fixture_state is replaced by a blank simulator state with the I_a menu built from the theory code
(restore.menu -> wj.make_menu -> groups.build), so the fixture file is never opened. An audit hook records every
'open' to prove it. The constructor then runs unchanged.

Usage: python3 -I -B nofix_run.py <trace json> <theory dir> <script> [script args...]
"""
import json
import os
import sys
from pathlib import Path

sys.dont_write_bytecode = True
OPENS = {}
ACTIVE = [True]


def hook(event, args):
    if ACTIVE[0] and event == 'open':
        p = args[0]
        if isinstance(p, bytes):
            p = p.decode('utf-8', 'replace')
        key = (str(p), str(args[1]))
        OPENS[key] = OPENS.get(key, 0) + 1


def main():
    out, th, script = sys.argv[1], Path(sys.argv[2]).resolve(), sys.argv[3]
    sys.addaudithook(hook)
    sys.path.insert(0, str(th))
    sys.path.insert(0, str(th.parent))
    import restore
    import seams
    import sim

    def blank_state(name, t):
        M = restore.menu(name)
        return sim.State(restore.ctx, menu=M), M
    seams._fixture_state = blank_state
    sys.argv = [script] + sys.argv[4:]
    import runpy
    status = 'ok'
    try:
        runpy.run_path(script, run_name='__main__')
    except SystemExit as exc:
        status = f'SystemExit {exc.code}'
    except BaseException as exc:  # noqa: BLE001
        status = f'error {type(exc).__name__}: {str(exc)[:300]}'
    finally:
        ACTIVE[0] = False
        rows = [{'path': os.path.realpath(p) if not p.isdigit() else p, 'mode': m, 'count': n}
                for (p, m), n in sorted(OPENS.items())]
        data = [r for r in rows if not r['path'].endswith(('.py', '.pyc'))
                and not r['path'].startswith(('/usr/lib/python3', '/usr/local/lib/python3'))]
        fixture_opened = any(r['path'].endswith('wj-I_a.json') for r in rows)
        with open(out, 'w') as fh:
            json.dump({'script': script, 'argv': sys.argv, 'status': status, 'fixture_opened': fixture_opened,
                       'data_opens': data}, fh, indent=1)
        print(f'nofix trace written: {status}, fixture opened: {fixture_opened}', file=sys.stderr)
    if status != 'ok':
        sys.exit(1)


if __name__ == '__main__':
    main()

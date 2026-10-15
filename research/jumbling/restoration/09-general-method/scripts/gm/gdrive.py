"""General driver (menu I_a): the general method M* (SENIOR-REVIEW.md section 9), from a reachable state to the solved
configuration, reading only the current pose table at every step. Research only: headless Linux, exact synthetic
geometry; no Windows, DirectX, input or performance claim.

Version history: v4 (the runs of 15 October 2026) stopped after --max-steps (default 80) and when both heuristic
steps missed. Since the senior review's milestone audit the driver implements M*: a heuristic miss goes to the
exhaustive fallback, there is no step limit by default, and the outer loop (mstar.py) checks the off count before
any budget, so a run whose last permitted step reaches the lattice still runs the lattice phase.

At each state Z (a pose table):
1. Units: mkblocks2.py (the rigid blocks; one unit per frame part of a multi-frame block).
2. Driver step: cbrun3.py --max-steps 1 at Z. Block-family words, single letters, a fresh dfam2 search and comm3
   refinement; each candidate W is closed as V W V^-1 by tclose2.py and accepted only when its unit is restored
   with collateral 0 and every off piece outside the step's free set keeps its exact pose (a clean step, Lemma U).
   The word library is the --seed directories plus every search made earlier in this run.
3. If the driver restores no unit at Z: pure-twist step. cellpure3.py runs on the units, largest first (ties by
   key), PAR at a time: the class-cell pure twist W (commutator powers around a triangle of edge cells of the unit's
   class, model check, exact replay). The units with a word are then closed, fewest lattice pieces in the carrier
   cell first (the pieces V must move away), with tclose2.py (V from the peeling certificate, greedy fallback) and,
   on 'no V', with tclose2m3.py (targeted depth-2 peel). The first success in that order is the step.
4. If neither kind restores a unit: the exhaustive fallback (fallback.py) for the largest unit (ties by key). It
   searches the menu words in order of length and takes the first clean step; it always finds one at a reachable
   state (M1-01), with no bound on its length. Each heuristic subprocess is killed after --heuristic-seconds, which
   then counts as a miss, so every heuristic batch ends and the fallback is always reached.
5. When no piece is off the lattice: the lattice phase (restore_lattice_late.py with the correction library; then
   verify_late.py --chain, which rebuilds the state from the start table through every step word).
Every step is appended to <work>/chain as a verify_u.py chain step (word with digests, unit, free set); at the end
verify_u.py replays the chain independently.

usage: gdrive.py <research/jumbling> <E-chain scripts> <start table> <name> <work dir> [--seed <dir>]...
       [--par K] [--greedy-seconds N] [--pure-greedy-seconds N] [--pure-max N] [--heuristic-seconds N]
       [--max-steps N] [--fallback-max-length N] [--fallback-only]
       [--lattice <scriptdir> <repo root> <library json> <checker dir>]
--max-steps (default 0, no limit) and --fallback-max-length (default 0, no limit) are experiment bounds outside M*.
--heuristic-seconds defaults to 21600. --fallback-only skips the heuristic steps (a test of the fallback path).
"""
import json
import os
import re
import shutil
import signal
import subprocess
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import mstar  # noqa: E402

J, SC, START, NAME, WORK = sys.argv[1:6]
a = sys.argv[6:]


def opt(name, default):
    return a[a.index(name) + 1] if name in a else default


PAR = int(opt('--par', 2))
GSEC = opt('--greedy-seconds', '600')
PGSEC = opt('--pure-greedy-seconds', '300')
MAXS = int(opt('--max-steps', 0))
PMAX = int(opt('--pure-max', 8))
HSEC = int(opt('--heuristic-seconds', 21600))
FBMAX = int(opt('--fallback-max-length', 0))
FB_ONLY = '--fallback-only' in a
SEEDS = [a[i + 1] for i, x in enumerate(a) if x == '--seed']
LAT = a[a.index('--lattice') + 1:a.index('--lattice') + 5] if '--lattice' in a else None
TH = str(Path(J) / 'theory')
WORK = Path(WORK).resolve()
WORK.mkdir(parents=True, exist_ok=True)
CH = WORK / 'chain'
CH.mkdir(exist_ok=True)
LOG = WORK / 'gdrive.jsonl'
PY = [sys.executable, '-I']
T0 = time.time()


def log(**kw):
    kw['sec'] = round(time.time() - T0)
    with LOG.open('a') as fh:
        fh.write(json.dumps(kw) + '\n')
    print(json.dumps(kw), flush=True)


def run(cmd, logf, timeout=None):
    """Run cmd with its output in logf. With a timeout, run it in its own process group, kill the whole group when
    the time runs out and return 'timeout'."""
    with open(logf, 'w') as fh:
        p = subprocess.Popen(cmd, stdout=fh, stderr=subprocess.STDOUT, start_new_session=timeout is not None)
        try:
            return p.wait(timeout=timeout)
        except subprocess.TimeoutExpired:
            os.killpg(p.pid, signal.SIGKILL)
            p.wait()
            fh.write(f'\n[gdrive] killed after {timeout} s\n')
            return 'timeout'


def units_of(table, tag):
    out = WORK / 'tg' / tag
    r = subprocess.run(PY + [str(HERE / 'mkblocks2.py'), TH, SC, table, tag, str(out)], capture_output=True, text=True)
    info = json.loads(r.stdout.strip().splitlines()[-1])
    units = [{'key': int(k), 'block': int(bf), 'size': int(sz), 'file': tf, 'free': ff}
             for k, bf, sz, tf, ff in info['units']]
    return info, units


# ---- chain
chain = {'name': NAME, 'start': 'start-table.json', 'start_digest': None, 'steps': []}
shutil.copyfile(START, CH / 'start-table.json')


def add_step(d, label, kind):
    """Append the closure in directory d (word-full.json, pre.json of tclose2/tclose2m3) as a chain step."""
    w = json.loads((d / 'word-full.json').read_text())
    pre = json.loads((d / 'pre.json').read_text())
    if chain['steps']:
        prev = json.loads((CH / chain['steps'][-1]['word']).read_text())['output_digest']
        assert w['input_digest'] == prev, 'step does not start where the chain ends'
    else:
        chain['start_digest'] = w['input_digest']
    T = sorted(int(p) for p in pre['target_block'])
    off = {int(p) for b in pre['blocks'] for p in b}
    F = sorted(off - set(T) - {int(p) for p in pre['protected_off']})
    n = len(chain['steps']) + 1
    (CH / f'step{n:03d}-word.json').write_text(json.dumps(w) + '\n')
    (CH / f'step{n:03d}-unit.json').write_text(json.dumps({'pieces': T}) + '\n')
    if F:
        (CH / f'step{n:03d}-free.json').write_text(json.dumps({'pieces': F}) + '\n')
    chain['steps'].append({'label': label, 'kind': kind, 'word': f'step{n:03d}-word.json',
                           'unit': f'step{n:03d}-unit.json', 'free': f'step{n:03d}-free.json' if F else None,
                           'size': len(T), 'free_size': len(F), 'length': len(w['word']), 'src': str(d)})
    (CH / 'chain.json').write_text(json.dumps(chain, indent=1) + '\n')
    return w


# ---- driver step
def driver_step(table, k):
    wd = WORK / 'drv' / f'k{k:02d}'
    cmd = PY + [str(HERE / 'cbrun3.py'), J, SC, table, f'{NAME}-k{k:02d}', str(wd), '--max-steps', '1',
                '--par', str(PAR), '--greedy-seconds', GSEC]
    for sd in SEEDS:
        cmd += ['--seed', sd]
    if (WORK / 'drv').exists() and any((WORK / 'drv').iterdir()):
        cmd += ['--seed', str(WORK / 'drv')]
    wd.mkdir(parents=True, exist_ok=True)
    t = time.time()
    rc = run(cmd, str(wd) + '.log', HSEC)
    rows = [json.loads(x) for x in (wd / 'run.jsonl').read_text().splitlines() if x.strip()] \
        if (wd / 'run.jsonl').exists() else []
    ok = [r for r in rows
          if r.get('event') == 'try' and r.get('ok') and mstar.closure_ok(Path(r['end_table']).parent)]
    stop = next((r for r in rows if r.get('event') == 'stop'), None)
    tries = sum(1 for r in rows if r.get('event') == 'try')
    if not ok:
        return None, {'rc': rc, 'tries': tries, 'stop': stop, 'seconds': round(time.time() - t)}

    def tix(r):
        m = re.search(r'-t(\d+)$', Path(r['end_table']).parent.name)
        return int(m.group(1))
    win = min(ok, key=tix)  # all successes lie in the last batch; the first in batch order is the one taken
    return win, {'rc': rc, 'tries': tries, 'seconds': round(time.time() - t)}


# ---- pure-twist step
def cp3_reason(out, logf):
    txt = Path(logf).read_text() if Path(logf).exists() else ''
    if 'AssertionError' in txt:
        return 'precondition: ' + txt.strip().splitlines()[-1][:200]
    if 'Traceback' in txt:
        return 'crash: ' + txt.strip().splitlines()[-1][:200]
    rec = json.loads(Path(out).read_text()) if Path(out).exists() else {}
    if rec.get('cycles_found') == 0:
        return 'no cycle of edge cells'
    if rec.get('model_check_pass') is False:
        return 'model check fails'
    return 'no word'


def pool(jobs):
    """Run (key, function) jobs PAR at a time in threads; return {key: result}."""
    from concurrent.futures import ThreadPoolExecutor
    with ThreadPoolExecutor(max_workers=PAR) as ex:
        futs = {k: ex.submit(f) for k, f in jobs}
        return {k: fu.result() for k, fu in futs.items()}


def pure_step(table, k, units):
    wd = WORK / 'pure' / f'k{k:02d}'
    wd.mkdir(parents=True, exist_ok=True)
    cand = sorted(units, key=lambda u: (-u['size'], u['key']))
    if len(cand) > PMAX:
        log(event='pure_dropped', step=k, units=[[u['size'], u['key']] for u in cand[PMAX:]])
        cand = cand[:PMAX]
    res = {}

    def word_job(u):
        def f():
            out = wd / f'cp3-{u["key"]}.json'
            lf = wd / f'cp3-{u["key"]}.log'
            rc = run(PY + [str(HERE / 'cellpure3.py'), TH, table, u['file'], str(out), '--search'], str(lf), HSEC)
            wf = wd / f'cp3-{u["key"]}-word.json'
            if rc == 'timeout':
                return {'word': None, 'reason': f'timeout after {HSEC} s'}
            if rc != 0 or not wf.exists():
                return {'word': None, 'reason': cp3_reason(out, lf)}
            rec = json.loads(out.read_text())
            return {'word': json.loads(wf.read_text())['word'], 'exact': rec.get('exact'),
                    'cell': rec.get('carrier_cell'), 'class': rec.get('class')}
        return f
    t = time.time()
    words = pool([(u['key'], word_job(u)) for u in cand])
    for u in cand:
        r = words[u['key']]
        res[u['key']] = {'size': u['size'], 'word_len': len(r['word']) if r['word'] else None,
                         'reason': r.get('reason'), 'class': r.get('class'),
                         'lattice_in_cell': (r.get('exact') or {}).get('lattice_pieces_in_X_before')}
    log(event='pure_words', step=k, seconds=round(time.time() - t), units=res)
    live = [u for u in cand if words[u['key']]['word']]
    live.sort(key=lambda u: ((words[u['key']].get('exact') or {}).get('lattice_pieces_in_X_before', 10 ** 9),
                             -u['size'], u['key']))

    def close_job(u):
        def f():
            wf = wd / f'w-{u["key"]}.json'
            wf.write_text(json.dumps({'P': [], 'Q': words[u['key']]['word']}) + '\n')
            out = []
            for script, tag in (('tclose2.py', 'c2'), ('tclose2m3.py', 'c3')):
                d = wd / f'{tag}-{u["key"]}'
                rc = run(PY + [str(HERE / script), TH, SC, table, u['file'], str(wf), str(d), '--greedy-seconds',
                               PGSEC], str(d) + '.log', HSEC)
                r = json.loads((d / 'result.json').read_text()) if rc == 0 and (d / 'result.json').exists() \
                    else {'result': 'timeout' if rc == 'timeout' else 'crash'}
                end = r.get('end')
                ok = bool(end and end['T_restored'] == u['size'] and end['collateral'] == 0
                          and end['protected_exact_pose_changed'] == 0 and mstar.closure_ok(d))
                out.append({'script': script, 'rc': rc, 'result': r.get('result', 'closed'), 'ok': ok,
                            'V': len(json.loads((d / 'parts' / 'V.json').read_text())['word'])
                            if (d / 'parts' / 'V.json').exists() else None, 'length': r.get('length')})
                if ok:
                    return d, out
                if r.get('result') != 'no V':
                    break
            return None, out
        return f
    for i in range(0, len(live), PAR):
        batch = live[i:i + PAR]
        t = time.time()
        got = pool([(u['key'], close_job(u)) for u in batch])
        for u in batch:
            res[u['key']]['close'] = got[u['key']][1]
        log(event='pure_close', step=k, seconds=round(time.time() - t),
            units={str(u['key']): got[u['key']][1] for u in batch})
        for u in batch:
            if got[u['key']][0] is not None:
                return got[u['key']][0], u, res
    return None, None, res


# ---- the steps of M*
def state_of(table, k):
    info, units = units_of(table, f'{NAME}-k{k:02d}')
    log(event='state', step=k, digest=info['digest'], off=info['off'], blocks=len(info['blocks']),
        units=sorted([[u['size'], u['key'], bool(u['free'])] for u in units], reverse=True))
    return info, units


def h_driver(table, k, info, units):
    win, dinfo = driver_step(table, k)
    if win is None:
        log(event='driver_stop', step=k, **dinfo)
        return None
    d = Path(win['end_table']).parent
    w = add_step(d, f'{NAME}-k{k:02d}-drv-u{win["unit"]}', 'driver')
    log(event='step', step=k, kind='driver', unit=win['unit'], size=win['size'], partial=win.get('partial'),
        moved_blocks=win.get('moved_blocks'), length=len(w['word']), off_after=win['end']['off'], **dinfo)
    return str(d / 'end-table.json')


def h_pure(table, k, info, units):
    d, u, pres = pure_step(table, k, units)
    if d is None:
        log(event='pure_stop', step=k, off=info['off'], result='no pure-twist step at this state', pure=pres)
        return None
    w = add_step(d, f'{NAME}-k{k:02d}-pure-u{u["key"]}', 'pure')
    end = json.loads((d / 'result.json').read_text())['end']
    log(event='step', step=k, kind='pure', unit=u['key'], size=u['size'], closure=d.name, length=len(w['word']),
        off_after=end['off'])
    return str(d / 'end-table.json')


def h_fallback(table, k, info, units):
    u = sorted(units, key=lambda x: (-x['size'], x['key']))[0]
    d = WORK / 'fallback' / f'k{k:02d}-u{u["key"]}'
    d.parent.mkdir(parents=True, exist_ok=True)
    cmd = PY + [str(HERE / 'fallback.py'), TH, SC, table, u['file'], str(d)]
    if FBMAX:
        cmd += ['--max-length', str(FBMAX)]
    t = time.time()
    rc = run(cmd, str(d) + '.log')
    r = json.loads((d / 'result.json').read_text()) if (d / 'result.json').exists() else {}
    log(event='fallback', step=k, unit=u['key'], size=u['size'], rc=rc, result=r.get('result'),
        length=r.get('length'), tried=r.get('tried'), seconds=round(time.time() - t))
    if r.get('result') == 'found' and rc == 0 and mstar.closure_ok(d):
        w = add_step(d, f'{NAME}-k{k:02d}-fb-u{u["key"]}', 'fallback')
        log(event='step', step=k, kind='fallback', unit=u['key'], size=u['size'], length=len(w['word']),
            off_after=r['end']['off'])
        return str(d / 'end-table.json')
    if rc == 3 and FBMAX:
        return None
    raise RuntimeError(f'fallback failed at step {k} (rc {rc}); see {d}.log')


# ---- main loop
log(event='start', name=NAME, start=START, seeds=SEEDS, par=PAR, greedy=GSEC, pure_greedy=PGSEC, max_steps=MAXS,
    heuristic_seconds=HSEC, fallback_max_length=FBMAX, fallback_only=FB_ONLY)
try:
    table, _, reason = mstar.drive(str(Path(START).resolve()), state_of, [] if FB_ONLY else [h_driver, h_pure],
                                   h_fallback, budget=MAXS, log=log)
except RuntimeError as e:
    log(event='stop', result=f'error: {e}')
    table, reason = None, 'error'
final = table if reason == 'lattice' else None

summary = {'name': NAME, 'steps': len(chain['steps']), 'reached_lattice': final is not None,
           'kinds': {kd: sum(1 for s in chain['steps'] if s['kind'] == kd) for kd in ('driver', 'pure', 'fallback')},
           'twists': sum(s['length'] for s in chain['steps'])}
log(event='off_lattice_phase_end', **summary)
if chain['steps']:
    rc = run(PY + [str(HERE / 'verify_u.py'), J, SC, str(CH / 'chain.json'), str(WORK / 'replay.jsonl')],
             str(WORK / 'replay.log'))
    last = json.loads((WORK / 'replay.jsonl').read_text().splitlines()[-1]) if (WORK / 'replay.jsonl').exists() else {}
    log(event='verify_u', rc=rc, **{k_: last.get(k_) for k_ in ('all_ok', 'steps', 'final_off', 'all_on_lattice',
                                                                  'final_digest')})
if final is not None and LAT is not None:
    sdir, root, lib, cdir = LAT
    ld = WORK / 'lattice'
    rc = run(PY + ['-B', str(Path(sdir) / 'restore_lattice_late.py'), sdir, root, J, lib, str(ld), f'table:{final}'],
             str(WORK / 'lattice-restore.log'))
    log(event='lattice_restore', rc=rc)
    words = [str(CH / s['word']) for s in chain['steps']]
    rc = run(PY + ['-B', str(Path(cdir) / 'verify_late.py'), root, J, lib, str(ld), str(ld / 'e2e-report.json'),
                   '--chain', str(CH / 'start-table.json')] + words, str(WORK / 'lattice-verify.log'))
    tail = Path(WORK / 'lattice-verify.log').read_text().strip().splitlines()[-1:] \
        if (WORK / 'lattice-verify.log').exists() else []
    log(event='lattice_verify', rc=rc, tail=tail)
log(event='done')

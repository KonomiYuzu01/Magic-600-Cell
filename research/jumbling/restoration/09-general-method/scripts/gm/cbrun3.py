"""General block-by-block run with free conflict blocks (menu I_a): Lemma U steps over restoration units.

A step restores one unit T (a single-frame block, or one frame part of a multi-frame block) with a word V W V^-1:
every lattice piece stays on K, T ends on K, and every off piece outside T and the step's free set keeps its exact
pose. The free set is the rest of T's block (for a multi-frame block) plus the other blocks that W moves but can
carry (their ghost stays alive). Free pieces may move and may stay off; the off count still falls by at least |T|
(Lemma U), so the run terminates.

At each state:
1. The units are listed (mkblocks2.py). The single letters at the movers of each unit's block join its pool. Every
   word in a unit's pool is scored at this state (score4.py): T on K,
   pieces W cannot carry (T, the unit's free set, other blocks: then W is not usable here), the other blocks W moves
   (they join the free set), Bad(W), its occupied chambers, the per-type deficit (Lemma D) and the vacancy
   conditions N1, N2 for the SAFE-ALL alphabet of the state (vac_ok: necessary for any V of SAFE-ALL letters).
2. Units without a usable vac_ok word get a fresh word search at this state (dfam2.py: D-type and C-type
   words), and the new words are scored.
3. The usable vac_ok words are tried with tclose2.py, PAR at a time, in the order: failed before; occupied
   Bad = 0 (W needs no V: larger units first); then smaller units first (they use fewer holes), occupied Bad,
   moved pieces, Bad. With --hard-first the units are ordered instead by the smallest Bad among their vac_ok words,
   largest first (Lemma D: |Bad_t(W)| <= off_t(Z), and off_t only falls along a run, so a unit whose words all need
   a large Bad set must go while the holes are many; a unit with a swap word, Bad = |T|, can go last). The first
   success in that order is taken (collateral 0, every protected off piece at its exact pose, T restored) and the
   run continues from its end table.
4. If no try succeeds, one refinement round runs (comm3.py on the usable words that move no other block: the
   commutators [W, k] and their deficits), the new words are scored and tried the same way.
The run stops when no unit is left (or no focus unit, with --only), or when no unit can be restored at a state.

usage: cbrun3.py <research/jumbling> <E-chain scripts> <start table> <name> <work dir>
       [--max-steps N] [--par K] [--max-tries N] [--greedy-seconds N] [--refine-q N] [--refine-k N]
       [--seed <dir with *.q.jsonl/*.summary.json or *.r.jsonl>]... [--only <json {pieces}>] [--no-search]
       [--dmax-q N]   (dfam2.py --max-q: how many D-type and C-type candidates are scored; default 1500)
       [--no-sletters] (do not add the single letters at the movers of each unit's block to its pool)
       [--hard-first] (order the units by the smallest Bad of their vac_ok words, largest first)
"""
import hashlib
import json
import subprocess
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
J, SC, TABLE, NAME, WORK = sys.argv[1:6]
a = sys.argv[6:]


def opt(name, default):
    return a[a.index(name) + 1] if name in a else default


MAXS = int(opt('--max-steps', 60))
PAR = int(opt('--par', 3))
MAXT = int(opt('--max-tries', 12))
GSEC = opt('--greedy-seconds', '600')
RQ = opt('--refine-q', '120')
RK = opt('--refine-k', '4')
SEEDS = [a[i + 1] for i, x in enumerate(a) if x == '--seed']
ONLY = set(json.loads(Path(opt('--only', '')).read_text())['pieces']) if '--only' in a else None
NOSEARCH = '--no-search' in a
NOSLETTERS = '--no-sletters' in a
HARD = '--hard-first' in a
DARGS = ['--max-q', opt('--dmax-q', '1500'), '--good-q', '300', '--min-q', '600', '--max-park', '800', '--no-pole',
         '--commutator']
WORK = Path(WORK)
WORK.mkdir(parents=True, exist_ok=True)
LOG = WORK / 'run.jsonl'
T0 = time.time()
PY = [sys.executable, '-I']


def log(**kw):
    kw['sec'] = round(time.time() - T0)
    with LOG.open('a') as fh:
        fh.write(json.dumps(kw) + '\n')
    print(json.dumps(kw), flush=True)


def run(cmd, timeout):
    return subprocess.run(cmd, capture_output=True, text=True, timeout=timeout)


def pool_run(jobs):
    """Run (cmd, log path, tag) jobs PAR at a time; return the tags in completion order."""
    procs, done = [], []
    todo = list(jobs)
    while todo or procs:
        while todo and len(procs) < PAR:
            cmd, lp, tag = todo.pop(0)
            fh = open(lp, 'w')
            procs.append((subprocess.Popen(cmd, stdout=fh, stderr=subprocess.STDOUT), tag, fh))
        time.sleep(2)
        for p in list(procs):
            if p[0].poll() is not None:
                p[2].close()
                procs.remove(p)
                done.append(p[1])
    return done


words = {}        # unit key -> {word key: word}
searched = set()  # (table, unit key)
refined = set()   # (table, unit key)
failed = set()    # (table, unit key, word key)
failcount = {}


def wk(w):
    return json.dumps(w)


def add(f, w):
    d = words.setdefault(int(f), {})
    k = wk(w)
    if k in d:
        return 0
    d[k] = w
    return 1


def seed(d):
    """Words of earlier searches: dfam2 rows (T on K) keyed by the target's first piece, comm3 rows by file name."""
    n = 0
    for sp in sorted(Path(d).rglob('*.summary.json')):
        s = json.loads(sp.read_text())
        T = s['target'] if isinstance(s['target'], list) else json.loads(s['target'])
        qp = Path(str(sp)[:-len('.summary.json')] + '.q.jsonl')
        if not qp.exists():
            continue
        for line in qp.open():
            r = json.loads(line)
            if r.get('T_on') == len(T) and r.get('T_dead', 0) == 0:
                n += add(min(T), r['Q'])
    for rp in sorted(list(Path(d).rglob('f*.r.jsonl')) + list(Path(d).rglob('f*.n.jsonl'))):
        f = int(rp.name[1:].split('.')[0])
        for line in rp.open():
            r = json.loads(line)
            if 'W' in r:
                n += add(f, r['W'])
    return n


def sletters(units, movers):
    """Single-letter words: every letter at every mover grip of the unit's block (score4.py keeps those that put the
    unit on K; with occupied Bad = 0 such a letter is a step by itself, e.g. the fresh-cap step of Lemma BL5)."""
    n = 0
    for f, u in units.items():
        for c in movers.get(u['block'], []):
            for i in range(60):
                n += add(f, [[int(c), i]])
    return n


def units_of(table, tag):
    out = WORK / 'tg' / tag
    r = run(PY + [str(HERE / 'mkblocks2.py'), J + '/theory', SC, table, tag, str(out)], 7200)
    info = json.loads(r.stdout.strip().splitlines()[-1])
    blocks = []
    for j, size, movers, first, nfr in info['blocks']:
        b = json.loads((out / f'{tag}-b{j:02d}.json').read_text())
        blocks.append((int(first), b['pieces'], [int(c) for c in b['movers']]))
    bj = out / 'blocks.json'
    bj.write_text(json.dumps([[f, ps] for f, ps, _ in blocks]) + '\n')
    dj = out / 'dblocks.json'
    dj.write_text(json.dumps({'blocks': [{'pieces': ps} for _, ps, _ in blocks]}) + '\n')
    units = {}
    for key, bfirst, size, tf, ff in info['units']:
        units[int(key)] = {'file': tf, 'free': ff, 'size': size, 'block': int(bfirst)}
    return info, units, {f: ps for f, ps, _ in blocks}, str(bj), str(dj), {f: mv for f, _, mv in blocks}


def score(table, units, tag, bj):
    """score4.py on every unit's pool; returns {unit key: [rows]} (usable rows only)."""
    outd = WORK / 'sc' / tag
    outd.mkdir(parents=True, exist_ok=True)
    jobs = []
    for f, u in units.items():
        ws = words.get(f)
        if not ws:
            continue
        wf = outd / f'f{f}.in.jsonl'
        wf.write_text(''.join(json.dumps({'W': w}) + '\n' for w in ws.values()))
        out = outd / f'f{f}.jsonl'
        cmd = PY + [str(HERE / 'score4.py'), J + '/theory', SC, table, bj, u['file'], str(wf), str(out)]
        if u['free']:
            cmd += ['--free', u['free']]
        jobs.append((cmd, str(outd / f'f{f}.log'), (f, str(out))))
    res = {}
    for f, out in pool_run(jobs):
        p = Path(out)
        rows = [json.loads(x) for x in p.open()] if p.exists() else []
        res[f] = [r for r in rows if r['usable']]
    return res


def search(table, units, tag, dj):
    outd = WORK / 'df' / tag
    outd.mkdir(parents=True, exist_ok=True)
    jobs = []
    for key, u in units.items():
        pre = str(outd / f'f{key}')
        park = ['--park-q', '0'] if u['free'] else ['--park-q', '3']
        cmd = PY + [str(HERE / 'dfam2.py'), J + '/theory', SC, table, u['file'], pre] + DARGS + park + ['--blocks', dj]
        if u['free']:
            cmd += ['--free', u['free']]
        jobs.append((cmd, pre + '.log', (key, pre)))
    n = 0
    for key, pre in pool_run(jobs):
        qp = Path(pre + '.q.jsonl')
        if qp.exists():
            for line in qp.open():
                r = json.loads(line)
                if r.get('T_on') == units[key]['size'] and r.get('T_dead', 0) == 0:
                    n += add(key, r['Q'])
    return n


def refine(table, units, tag, scored):
    outd = WORK / 'rf' / tag
    outd.mkdir(parents=True, exist_ok=True)
    jobs = []
    for f, u in units.items():
        src = [r for r in scored.get(f, []) if not r['moved_blocks']]
        if not src or (table, f) in refined:
            continue
        refined.add((table, f))
        src.sort(key=lambda r: (r['deficit'], r['occ'], r['bad']))
        qf = outd / f'f{f}.r.in.jsonl'
        qf.write_text(''.join(json.dumps({'Q': r['W'], 'T_on': u['size']}) + '\n' for r in src[:int(RQ)]))
        out = outd / f'f{f}.r.jsonl'
        cmd = PY + [str(HERE / 'comm3.py'), J + '/theory', SC, table, u['file'], str(qf), str(out),
                    '--rescore', '--max-q', RQ, '--max-k', RK]
        if u['free']:
            cmd += ['--free', u['free']]
        jobs.append((cmd, str(outd / f'f{f}.r.log'), (f, str(out))))
    n = 0
    for f, out in pool_run(jobs):
        p = Path(out)
        if p.exists():
            for line in p.open():
                r = json.loads(line)
                if 'W' in r:
                    n += add(f, r['W'])
    return n


def order(units, scored, table):
    per, live = {}, {}
    for f, rows in scored.items():
        if f not in units:
            continue
        for r in rows:
            if r.get('vac_ok', r['deficit'] == 0) and (table, f, wk(r['W'])) not in failed:
                live.setdefault(f, []).append(r)
    hb = {f: min(r['bad'] for r in rows) for f, rows in live.items()}
    for f, rows in live.items():
        size = units[f]['size']
        for r in rows:
            k = (f, wk(r['W']))
            z = r['occ'] == 0
            if HARD:
                key = (failcount.get(k, 0), -hb[f], f, r['occ'], r['moved_pieces'], r['bad'])
            else:
                key = (failcount.get(k, 0), 0 if z else 1, -size if z else size, r['occ'], r['moved_pieces'], r['bad'])
            per.setdefault(f, []).append((key, f, r))
    for v in per.values():
        v.sort(key=lambda x: x[0])
    out, rank = [], 0
    while any(rank < len(v) for v in per.values()):
        layer = sorted((v[rank] for v in per.values() if rank < len(v)), key=lambda x: x[0])
        out.extend((x[1], x[2]) for x in layer)
        rank += 1
    return out


def try_batch(table, tag, units, bpieces, batch, novac, base, step):
    """tclose2.py on a batch in parallel; the first success in batch order wins and stops the rest."""
    meta, procs = [], {}
    sk = WORK / f'skip-{tag}.json'
    sk.write_text(json.dumps(novac) + '\n')
    for t, (f, r) in enumerate(batch, start=base):
        wf = WORK / f'w-{tag}-f{f}-t{t:02d}.json'
        wf.write_text(json.dumps({'P': [], 'Q': r['W']}) + '\n')
        free = set(json.loads(Path(units[f]['free']).read_text())['pieces']) if units[f]['free'] else set()
        for b in r['moved_blocks']:
            free |= set(bpieces[b])
        out = WORK / f'close-{tag}-f{f}-t{t:02d}'
        cmd = PY + [str(HERE / 'tclose2.py'), J + '/theory', SC, table, units[f]['file'], str(wf), str(out),
                    '--skip-bad', str(sk), '--greedy-seconds', GSEC]
        if free:
            ff = WORK / f'free-{tag}-f{f}-t{t:02d}.json'
            ff.write_text(json.dumps({'pieces': sorted(free)}) + '\n')
            cmd += ['--free', str(ff)]
        fh = open(str(out) + '.log', 'w')
        procs[t] = (subprocess.Popen(cmd, stdout=fh, stderr=subprocess.STDOUT), fh)
        meta.append((t, f, r, out, len(free)))
    t1 = time.time()
    state = {}

    def read(t, f, r, out, nfree):
        res = json.loads((out / 'result.json').read_text()) if (out / 'result.json').exists() else {'result': 'crash'}
        pre = json.loads((out / 'pre.json').read_text()) if (out / 'pre.json').exists() else {}
        end = res.get('end')
        ok = bool(end and end['T_restored'] == units[f]['size'] and end['collateral'] == 0
                  and end['protected_exact_pose_changed'] == 0)
        if res.get('result') in ('no V', 'known no V'):
            dg = pre.get('ghost', {}).get('badW_digest')
            if dg and dg not in novac:
                novac.append(dg)
        log(event='try', step=step, table=table, word=r['W'], end_table=str(out / 'end-table.json'), unit=f,
            block=units[f]['block'], size=units[f]['size'], partial=bool(units[f]['free']),
            moved_blocks=r['moved_blocks'], free=nfree, bad=r['bad'], occ=r['occ'], deficit=r['deficit'], ok=ok,
            result=res.get('result'), end=end, length=res.get('length'), V=res.get('V_source'),
            seconds=round(time.time() - t1))
        return ok
    while True:
        for t, f, r, out, nfree in meta:
            if t in state:
                continue
            p, fh = procs[t]
            if p.poll() is not None:
                fh.close()
                state[t] = read(t, f, r, out, nfree)
        winner = None
        for t, f, r, out, nfree in meta:
            if state.get(t, 'run') == 'run':
                break
            if state[t]:
                winner = t
                break
        if winner is not None or len(state) == len(meta):
            for t, f, r, out, nfree in meta:
                if t not in state:
                    procs[t][0].kill()
                    procs[t][0].wait()
                    procs[t][1].close()
                    state[t] = None
                    log(event='stopped', step=step, unit=f, occ=r['occ'], bad=r['bad'])
            break
        time.sleep(2)
    return [(state[t], f, r, out) for t, f, r, out, _ in meta]


def attempt(table, tag, units, bpieces, scored, novac, base, step):
    """Try the vac_ok candidates in order; return (new table or None, next base)."""
    cand = order(units, scored, table)
    tries = 0
    while cand and tries < MAXT:
        batch, cand = cand[:PAR], cand[PAR:]
        tries += len(batch)
        res = try_batch(table, tag, units, bpieces, batch, novac, base, step)
        base += len(batch)
        for ok, f, r, out in res:
            if ok is None:
                continue
            if ok:
                return str(out / 'end-table.json'), base
            k = (f, wk(r['W']))
            failed.add((table,) + k)
            failcount[k] = failcount.get(k, 0) + 1
    return None, base


for sd in SEEDS:
    log(event='seed', dir=sd, added=seed(sd))
table = TABLE
step = 0
log(event='start', table=TABLE, words=sum(len(v) for v in words.values()),
    only=None if ONLY is None else len(ONLY))
while step < MAXS:
    tag = f'{NAME}-k{step:02d}'
    info, units, bpieces, bj, dj, movers = units_of(table, tag)
    if ONLY is not None:
        units = {k: u for k, u in units.items() if ONLY & set(json.loads(Path(u['file']).read_text())['pieces'])}
    if not NOSLETTERS:
        sletters(units, movers)
    log(event='state', step=step, digest=info['digest'], off=info['off'], blocks=len(info['blocks']),
        units=sorted([u['size'], k, u['block'], bool(u['free'])] for k, u in units.items()))
    if not units:
        log(event='done', step=step, result='all off pieces restored' if not info['blocks']
            else 'all focus pieces restored')
        break
    novac, base, new = [], 1, None
    scored = score(table, units, tag, bj)
    need = {f: u for f, u in units.items()
            if not any(r.get('vac_ok') for r in scored.get(f, [])) and (table, f) not in searched}
    log(event='scored', step=step, usable={str(f): len(v) for f, v in scored.items()},
        deficit0={str(f): sum(1 for r in v if r['deficit'] == 0) for f, v in scored.items()},
        vac_ok={str(f): sum(1 for r in v if r.get('vac_ok')) for f, v in scored.items()},
        occ0={str(f): sum(1 for r in v if r['occ'] == 0) for f, v in scored.items()}, need_search=sorted(need))
    if need and not NOSEARCH:
        searched.update((table, f) for f in need)
        n = search(table, need, tag, dj)
        sc2 = score(table, need, tag + '-s', bj)
        scored.update(sc2)
        log(event='search', step=step, searched=sorted(need), added=n,
            vac_ok={str(f): sum(1 for r in v if r.get('vac_ok')) for f, v in sc2.items()})
    new, base = attempt(table, tag, units, bpieces, scored, novac, base, step)
    if new is None:
        n = refine(table, units, tag, scored)
        log(event='refine', step=step, added=n)
        if n:
            scored = score(table, units, tag + '-r', bj)
            new, base = attempt(table, tag, units, bpieces, scored, novac, base, step)
    if new is None:
        log(event='stop', step=step, result='no unit restored at this state',
            remaining=sorted([u['size'], f, u['block'], bool(u['free'])] for f, u in units.items()),
            best={str(f): sorted([[r.get('n2'), r.get('n1'), r['deficit'], r['occ'], r['bad'], len(r['moved_blocks'])]
                                  for r in v])[:5]
                  for f, v in scored.items()})
        break
    table = new
    step += 1

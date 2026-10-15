"""Block-by-block restoration run (menu I_a): the greedy order of Theorem CB on one input, with exact replay.

State k is a pose table. Its rigid blocks are listed by mkblocks.py. A block is named by its least piece. Candidate
words for a block come from dfam.py runs (Q alone, or a parked W = P Q P^-1). A candidate carries its Bad size and
its conflict set: the blocks whose pieces its ghost moves or kills at the state of the search. Pieces of unrestored
blocks keep their exact poses from step to step (every step keeps the other off pieces exact), so a candidate's
ghost on them is unchanged and its conflict set can only shrink as blocks are restored. At each step the run takes
the block and candidate with an empty current conflict set and the fewest occupied Bad chambers at the search state
(then the smallest Bad), runs tclose.py (ghost premises,
SAFE-ALL menu, vacancy certificate, V, exact replay of V W V^-1 with every twist checked) and continues from the
exported end table when the block is restored with collateral 0 and every other off piece exact. When no block has
a clean candidate, dfam.py is run on the remaining blocks at the current state; if that adds none, the run stops and
records the blocking structure.

A failed attempt is recorded per state. A Bad(W) set found not vacatable at a state is passed to later attempts at
that state (tclose.py --skip-bad), so a word with the same Bad(W) stops after its ghost premises. Candidates that failed
at an earlier state are tried again, after the others. At most --max-tries tclose.py attempts are made per state.

--inbox <dir>: dfam.py outputs placed there later are harvested at the next state. --resume: continue a run in the
same work dir from its run.jsonl (last restored state, failed attempts, searched states) and its own dfam outputs.

usage: blockrun.py <research/jumbling> <E-chain scripts> <start table> <name> <work dir> [--max-steps N]
       [--par K] [--dfam-args "..."] [--seed-dfam <dir>]... [--max-tries N] [--greedy-seconds N] [--inbox <dir>]
       [--resume]
"""
import json
import shlex
import subprocess
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
J, SC, TABLE, NAME, WORK = sys.argv[1:6]
a = sys.argv[6:]
MAXS = int(a[a.index('--max-steps') + 1]) if '--max-steps' in a else 40
PAR = int(a[a.index('--par') + 1]) if '--par' in a else 3
DARGS = shlex.split(a[a.index('--dfam-args') + 1]) if '--dfam-args' in a else \
    ['--max-q', '1500', '--good-q', '300', '--min-q', '600', '--park-q', '3', '--max-park', '800', '--no-pole']
SEEDS = [a[i + 1] for i, x in enumerate(a) if x == '--seed-dfam']
MAXT = int(a[a.index('--max-tries') + 1]) if '--max-tries' in a else 12
GSEC = a[a.index('--greedy-seconds') + 1] if '--greedy-seconds' in a else '600'
INBOX = Path(a[a.index('--inbox') + 1]) if '--inbox' in a else None
RESUME = '--resume' in a
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


def blocks_of(table, tag):
    out = WORK / 'tg' / tag
    r = run(PY + [str(HERE / 'mkblocks.py'), J + '/theory', SC, table, tag, str(out)], 1800)
    info = json.loads(r.stdout.strip().splitlines()[-1])
    res = {}
    for j, size, movers, first in info['blocks']:
        res[int(first)] = {'file': str(out / f'{tag}-b{j:02d}.json'), 'size': size, 'movers': movers}
    return info, res


lib = {}          # first piece -> list of candidates
harvested = set()  # dfam prefixes already read
failed = set()    # (table, first piece, word key)
failcount = {}    # (first piece, word key) -> failed attempts at earlier states
state_tries = {}  # table -> tclose.py attempts made at that state


def wkey(w):
    return json.dumps([w['P'], w['Q']])


def harvest(prefix):
    """Add the candidates of one dfam run (rows with T restored; conflict sets mapped to first pieces)."""
    sp = Path(prefix + '.summary.json')
    if not sp.exists() or prefix in harvested:
        return 0
    harvested.add(prefix)
    d = json.loads(sp.read_text())
    T = d['target']
    first = min(T)
    bl = {j: f for j, _, f in d.get('blocks', [])}
    n = 0

    def conflicts(r):
        if 'conflict_blocks' in r:
            return frozenset(bl[j] for j in r['conflict_blocks'])
        if r.get('prot_dead', 0) == 0 and r.get('prot_moved', 0) == 0:
            return frozenset()
        return None
    rows = []
    qp = Path(prefix + '.q.jsonl')
    if qp.exists():
        for line in qp.open():
            r = json.loads(line)
            if r['T_on'] == len(T) and r.get('T_dead', 0) == 0:
                cf = conflicts(r)
                if cf is not None:
                    rows.append((r['badQ'], r.get('badQ_occupied', r['badQ']), cf, {'P': [], 'Q': r['Q']}))
    pp = Path(prefix + '.park.jsonl')
    if pp.exists():
        for line in pp.open():
            r = json.loads(line)
            if r['T_on'] == len(T):
                cf = conflicts(r)
                if cf is not None:
                    rows.append((r['badW'], r.get('badW_occupied', r['badW']), cf, {'P': r['P'], 'Q': r['Q']}))
    best = {}
    for bad, occ, cf, w in sorted(rows, key=lambda x: (x[1], x[0])):
        lst = best.setdefault(cf, [])
        if len(lst) < 8:
            lst.append((bad, occ, w))
    for cf, lst in best.items():
        for bad, occ, w in lst:
            lib.setdefault(first, []).append({'bad': bad, 'occ': occ, 'conflicts': cf, 'word': w, 'source': prefix})
            n += 1
    return n


def dfam_all(table, blocks, tag):
    """dfam.py on every given block of the state, PAR at a time."""
    outd = WORK / 'df' / tag
    outd.mkdir(parents=True, exist_ok=True)
    procs, done = [], []
    todo = list(blocks.items())
    while todo or procs:
        while todo and len(procs) < PAR:
            first, b = todo.pop(0)
            pre = str(outd / f'f{first}')
            cmd = PY + [str(HERE / 'dfam.py'), J + '/theory', SC, table, b['file'], pre] + DARGS
            fh = open(pre + '.log', 'w')
            procs.append((subprocess.Popen(cmd, stdout=subprocess.DEVNULL, stderr=fh), pre, fh))
        time.sleep(2)
        for p in list(procs):
            if p[0].poll() is not None:
                p[2].close()
                procs.remove(p)
                done.append(p[1])
    return sum(harvest(pre) for pre in done)


for sd in SEEDS:
    for sp in sorted(Path(sd).glob('*.summary.json')):
        harvest(str(sp)[:-len('.summary.json')])
table = TABLE
step = 0
searched_at = set()
resumed = {}
if RESUME and LOG.exists():
    for line in LOG.open():
        ev = json.loads(line)
        if ev.get('event') == 'try' and 'table' in ev:
            if ev['ok']:
                table, step = ev['end_table'], ev['step'] + 1
            else:
                k = (ev['block'], ev['wkey'])
                failed.add((ev['table'],) + k)
                failcount[k] = failcount.get(k, 0) + 1
            state_tries[ev['table']] = state_tries.get(ev['table'], 0) + 1
        if ev.get('event') == 'search' and 'table' in ev:
            searched_at.add(ev['table'])
    for sp in sorted((WORK / 'df').glob('*/*.summary.json')):
        harvest(str(sp)[:-len('.summary.json')])
    resumed = {'resumed_table': table, 'resumed_step': step, 'failed': len(failed), 'searched': len(searched_at)}
log(event='start', table=TABLE, seeded=sum(len(v) for v in lib.values()), **resumed)
while step < MAXS:
    tag = f'{NAME}-k{step:02d}'
    cur = table
    if INBOX is not None and INBOX.exists():
        got = sum(harvest(str(sp)[:-len('.summary.json')]) for sp in sorted(INBOX.glob('*.summary.json')))
        if got:
            log(event='inbox', step=step, added=got)
    info, blocks = blocks_of(table, tag)
    present = set(blocks)
    log(event='state', step=step, digest=info['digest'], off=info['off'],
        blocks=sorted([b['size'], f] for f, b in blocks.items()))
    if not blocks:
        log(event='done', step=step, result='all off pieces restored')
        break
    choice = []
    for f, b in blocks.items():
        for c in lib.get(f, []):
            k = (f, wkey(c['word']))
            if (table,) + k in failed:
                continue
            if c['conflicts'] & present:
                continue
            choice.append((failcount.get(k, 0), c.get('occ', c['bad']), c['bad'], -b['size'], f, c))
    choice.sort(key=lambda x: (x[0], x[1], x[2], x[3]))
    progressed = False
    novac = []
    tries = 0
    for _, _, bad, _, f, c in choice:
        if tries >= MAXT:
            break
        tries += 1
        wf = WORK / f'w-{tag}-f{f}-t{tries:02d}.json'
        wf.write_text(json.dumps(c['word']) + '\n')
        sk = WORK / f'skip-{tag}.json'
        sk.write_text(json.dumps(novac) + '\n')
        out = WORK / f'close-{tag}-f{f}-t{tries:02d}'
        t1 = time.time()
        r = run(PY + [str(HERE / 'tclose.py'), J + '/theory', SC, table, blocks[f]['file'], str(wf), str(out),
                      '--skip-bad', str(sk), '--greedy-seconds', GSEC], 7200)
        res = json.loads((out / 'result.json').read_text()) if (out / 'result.json').exists() else {'result': 'crash',
                                                                                                    'stderr': r.stderr[-400:]}
        pre = json.loads((out / 'pre.json').read_text()) if (out / 'pre.json').exists() else {}
        end = res.get('end')
        ok = bool(end and end['T_restored'] == blocks[f]['size'] and end['collateral'] == 0
                  and end['protected_exact_pose_changed'] == 0)
        if res.get('result') in ('no V', 'known no V'):
            dg = pre.get('ghost', {}).get('badW_digest')
            if dg and dg not in novac:
                novac.append(dg)
        if res.get('result') in ('known no V', 'ghost premises fail'):
            tries -= 1
        log(event='try', step=step, table=table, wkey=wkey(c['word']), end_table=str(out / 'end-table.json'),
            block=f, size=blocks[f]['size'], bad=bad, P=len(c['word']['P']),
            ok=ok, result=res.get('result'), end=end, length=res.get('length'), V=res.get('V_source'),
            certificate=pre.get('certificate', {}).get('all_reached'), greedy=pre.get('greedy'),
            seconds=round(time.time() - t1))
        if ok:
            table = str(out / 'end-table.json')
            step += 1
            progressed = True
            break
        k = (f, wkey(c['word']))
        failed.add((table,) + k)
        failcount[k] = failcount.get(k, 0) + 1
    state_tries[cur] = state_tries.get(cur, 0) + tries
    if progressed:
        continue
    untried = [1 for f in blocks for c in lib.get(f, [])
               if not (c['conflicts'] & present) and (table, f, wkey(c['word'])) not in failed]
    if table in searched_at:
        if untried and state_tries[table] < 3 * MAXT:
            continue
        log(event='stop', step=step, result='no block restored at this state after a search', tries=state_tries[table],
            untried=len(untried), remaining=sorted([b['size'], f] for f, b in blocks.items()),
            candidates={str(f): [[c['bad'], sorted(c['conflicts'])] for c in lib.get(f, [])][:12] for f in blocks})
        break
    searched_at.add(table)
    clean_left = {f for f in blocks for c in lib.get(f, [])
                  if not (c['conflicts'] & present) and (table, f, wkey(c['word'])) not in failed}
    todo = {f: b for f, b in blocks.items() if f not in clean_left}
    n = dfam_all(table, todo, tag) if todo else 0
    log(event='search', step=step, table=table, searched=sorted(todo), added=n)

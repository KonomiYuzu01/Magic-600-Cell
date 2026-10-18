# Round 22, line (a): the replay continued past closures whose V exists only as an R22B-10 certificate. From a pose
# table, each step of a plan is a literal word (a census word, a T1' word, any word), a closure V.W.V^-1 or a T1' step
# W.V'.W^-1.V'^-1 (R22C-T1'). For a closure, N1 and N2 of its complete Bad(W) are read at the current state first
# (jordan_target.py); a unit whose target fails them is recorded with its deficits and skipped. Otherwise V is built
# at the current state (r22b10_cert.py), checked (check_r22b10.py) and applied exactly as a labelled chamber
# permutation (slp_apply.py), with W = P.Q.P^-1 from a tclose2 core between V and V^-1. For a T1' step the landing
# word W is applied literally (stepchk.py); Z1 = Z.W, the lattice pieces Q that W took off and W^-1 are exported
# (prep_t1prime.py), and tclose2.py computes T = Bad(W^-1) at Z1 and tries a literal V' (peeling certificate, then a
# short greedy search); without one, N1 and N2 of T are read at Z1 and V' is built there as a certificate, as for a
# closure. With "t1prime": "search", W is the class-cell pure twist that cellpure4.py --search finds for the unit at
# the current state (as exp_b.py runs it, without a preparation). Every step's strict' contract is checked by
# slp_apply.py --unit against the state in front of it; a step that is not strict' is recorded and the replay
# continues from the last state. A step whose unit is already on the lattice (stepchk.py with the empty word) is
# skipped, so a plan may try a unit by T1' first and by its closure after. One JSON line per step goes to
# seq.jsonl in the work directory, and a final line {"done": true, ...}; the files of step k for unit u are named
# <kk>-<u>-... there (z-<kk>-<u>.json for its end table), and the record names files by base name only and withholds
# a child's error message that contains a slash or a backslash, so no work-directory or input path enters it.
# Exact; headless; research only.
# Usage (from the repository root): python3 -I -B replay_slp.py <start pose table> <plan json> <new work dir>
# Plan: {"steps": [{"unit": "b12", "unit_file": <unit json>, "word": <word json>},
#                  {"unit": "b14", "unit_file": <unit json>, "core": <core json>, "target": <target json>},
#                  {"unit": "b03", "unit_file": <unit json>, "t1prime": <W json>, "greedy_seconds": 60},
#                  {"unit": "b15", "unit_file": <unit json>, "t1prime": "search"}, ...]}
# A target file holds the complete Bad(W) of the closure's W (keys T and T_digest, as jordan_target.py reads them).
import hashlib
import json
import os
import subprocess
import sys
from pathlib import Path

S = Path('research/jumbling/restoration/12-low-height/scripts')
TH = Path('research/jumbling/theory')
GM = Path('research/jumbling/restoration/09-general-method/scripts')
PY = [sys.executable, '-I', '-B']
ENV = dict(os.environ, OPENBLAS_NUM_THREADS='1', OMP_NUM_THREADS='1')


def digest(table):
    return json.loads(Path(table).read_text())['digest']


def run(args, log, py=PY):
    with open(log, 'w') as f:
        return subprocess.run(py + [str(a) for a in args], stdout=f, stderr=subprocess.STDOUT, env=ENV).returncode


def condition(log):
    for line in Path(log).read_text().splitlines():
        if line.startswith('CONDITION '):
            return json.loads(line[len('CONDITION '):])
    return None


def json_lines(log):
    rows = []
    for line in Path(log).read_text().splitlines():
        if line.startswith('{'):
            rows.append(json.loads(line))
    return rows


def safe_error(msg):
    """A child's error message for the record, withheld when it contains a slash or a backslash (it may name a path),
    so no work-directory or input path enters a record; the step's apply log keeps the message."""
    msg = str(msg)
    return 'withheld: the message names a path (see the apply log)' if '/' in msg or '\\' in msg else msg


def unit_landed(tag, wd, cur, unit_file):
    """True when every piece of the unit is on the lattice at the state of cur (stepchk.py with the empty word), None
    when the check fails."""
    ef, res = wd / f'{tag}-empty.json', wd / f'{tag}-landed.json'
    ef.write_text(json.dumps({'word': []}) + '\n')
    run([S / 'round22/stepchk.py', cur, unit_file, ef, res], wd / f'{tag}-landed.log')
    if not res.exists():
        return None
    r = json.loads(res.read_text())
    return r['unit_on_lattice'] == r['unit_size']


def emit(out, row):
    print(json.dumps(row), file=out, flush=True)
    print(json.dumps(row), flush=True)


def vacancy_slp(tag, wd, table, zt, t, row):
    """N1 and N2 of the target t at the state of table (digest zt), then an R22B-10 certificate built and checked there.
    Returns the certificate file, or None with the reason in row."""
    assert hashlib.sha256(json.dumps(t['T']).encode()).hexdigest() == t['T_digest'], 'target digest'
    tf = wd / f'{tag}-target.json'
    tf.write_text(json.dumps(dict(t, state_digest=zt)) + '\n')
    run([S / 'round22/jordan_target.py', tf, table], wd / f'{tag}-jordan.txt')
    c = condition(wd / f'{tag}-jordan.txt')
    row.update(target=len(t['T']), T_digest=t['T_digest'])
    if c is None:
        row.update(how='none', reason='jordan_target.py gave no condition')
        return None
    row.update(N1=c['fixed_vacant'], N2=c['necessary'])
    if not (c['fixed_vacant'] and c['necessary']):
        row.update(how='none', reason='N1 or N2 fails', fixed_occupied=c['fixed_occupied'],
                   deficits=c['necessary_deficits'])
        return None
    cert = wd / f'{tag}-cert.json.gz'
    run([S / 'r22b10_cert.py', 'table', table, tf, cert, '--seconds', '3600'], wd / f'{tag}-build.log')
    run([S / 'check_r22b10.py', 'table', table, tf, cert], wd / f'{tag}-check.txt')
    lines = Path(wd / f'{tag}-check.txt').read_text().split()
    if not lines or lines[-1] != 'PASS':
        row.update(how='none', reason='certificate incomplete or its check failed')
        return None
    row.update(cert=cert.name, check='PASS')
    return cert


def closure_program(step, tag, wd, cur, z, row):
    """Check N1 and N2, build and check V; return the program, or None with the reason in row."""
    t = json.loads(Path(step['target']).read_text())
    cert = vacancy_slp(tag, wd, cur, z, {'T': t['T'], 'T_digest': t['T_digest']}, row)
    if cert is None:
        return None
    return {'segments': [{'slp': str(cert), 'node': 'V', 'state_digest': z}, {'core_file': step['core']},
                         {'slp': str(cert), 'node': 'V', 'inverse': True}]}


def t1prime_program(step, tag, wd, cur, z, row):
    """W lands the unit; V' vacates T = Bad(W^-1) at Z1 = Z.W, as a literal word or a certificate. Returns the
    program of W.V'.W^-1.V'^-1 (W alone when W is already strict'), or None with the reason in row."""
    if step['t1prime'] == 'search':
        cp = wd / f'{tag}-cp4.json'
        run([S / 'cellpure4.py', TH, cur, step['unit_file'], cp, '--search'], wd / f'{tag}-cp4.log')
        if not (wd / f'{tag}-cp4-word.json').exists():
            row.update(how='none', reason='cellpure4.py found no W here')
            return None
        w = json.loads((wd / f'{tag}-cp4-word.json').read_text())['word']
    else:
        w = json.loads(Path(step['t1prime']).read_text())['word']
    wf = wd / f'{tag}-W.json'
    wf.write_text(json.dumps({'word': w, 'input_digest': z}) + '\n')
    run([S / 'round22/stepchk.py', cur, step['unit_file'], wf, wd / f'{tag}-W-check.json'], wd / f'{tag}-W-check.log')
    if not (wd / f'{tag}-W-check.json').exists():
        row.update(how='none', reason='W is not admissible here')
        return None
    chk = json.loads((wd / f'{tag}-W-check.json').read_text())
    row.update(W=len(w), Q=chk['collateral'])
    if chk['unit_on_lattice'] != chk['unit_size'] or chk['others_exact'] + chk['others_landed'] != chk['others']:
        row.update(how='none', reason='W does not land the unit with every other off piece exact or landed')
        return None
    if chk['strict_prime']:
        return {'segments': [{'word_file': str(wf)}]}
    wf.write_text(json.dumps({'word': w, 'input_digest': z, 'output_digest': chk['end_digest']}) + '\n')
    run([S / 'round22/prep_t1prime.py', cur, wf, tag, wd], wd / f'{tag}-prep.log')
    z1t, qf, winv = wd / f'{tag}-z1-table.json', wd / f'{tag}-Q.json', wd / f'{tag}-winv.json'
    if not (z1t.exists() and qf.exists() and winv.exists()):
        row.update(how='none', reason='prep_t1prime.py failed')
        return None
    zt1 = digest(z1t)
    c2 = wd / f'{tag}-c2'
    run([GM / 'gm/tclose2.py', TH, GM / 'echain', z1t, qf, winv, c2, '--greedy-seconds',
         step.get('greedy_seconds', 60)], wd / f'{tag}-c2.log', py=PY[:2])
    if (c2 / 'word-full.json').exists():
        row.update(V='word')
        return {'segments': [{'word_file': str(wf)}, {'word_file': str(c2 / 'word-full.json')}]}
    if not (c2 / 'pre.json').exists():
        row.update(how='none', reason='tclose2.py wrote no premises')
        return None
    g = json.loads((c2 / 'pre.json').read_text())['ghost']
    if g['T_on_lattice_at_end'] != g['T_size'] or not g['protected_exact_at_end']:
        row.update(how='none', reason='ghost premises of W^-1 fail at Z1')
        return None
    t = {'T': g['badW'], 'T_digest': hashlib.sha256(json.dumps(g['badW']).encode()).hexdigest()}
    assert t['T_digest'] == g['badW_digest'], 'Bad(W^-1) digest'
    cert = vacancy_slp(f'{tag}-v', wd, z1t, zt1, t, row)
    if cert is None:
        return None
    row.update(V='certificate')
    return {'segments': [{'word_file': str(wf)}, {'slp': str(cert), 'node': 'V', 'state_digest': zt1},
                         {'core_file': str(winv)}, {'slp': str(cert), 'node': 'V', 'inverse': True}]}


def main():
    start, plan_file, wd = sys.argv[1], sys.argv[2], Path(sys.argv[3])
    if wd.exists() and any(wd.iterdir()):
        sys.exit(f'work directory {wd} is not empty')
    wd.mkdir(parents=True, exist_ok=True)
    plan = json.loads(Path(plan_file).read_text())
    cur, landed = Path(start), 0
    with open(wd / 'seq.jsonl', 'w') as out:
        for k, step in enumerate(plan['steps']):
            u, z = step['unit'], digest(cur)
            tag = f'{k:02d}-{u}'
            row = {'step': k, 'unit': u, 'start_digest': z}
            if unit_landed(tag, wd, cur, step['unit_file']):
                emit(out, dict(row, how='skip', reason='the unit is on the lattice here'))
                continue
            if 'word' in step:
                how, prog = 'word', {'segments': [{'word_file': step['word']}]}
            else:
                if 't1prime' in step:
                    how, prog = 't1prime', t1prime_program(step, tag, wd, cur, z, row)
                else:
                    how, prog = 'close', closure_program(step, tag, wd, cur, z, row)
                if prog is None:
                    emit(out, row)
                    continue
            pf = wd / f'{tag}-program.json'
            pf.write_text(json.dumps(prog) + '\n')
            end = wd / f'z-{tag}.json'
            rc = run([S / 'slp_apply.py', 'table', cur, pf, end, '--unit', step['unit_file']], wd / f'{tag}-apply.log')
            rows = json_lines(wd / f'{tag}-apply.log')
            summary = rows[-1] if rows else {}
            if rc == 0 and summary.get('strict_prime'):
                letters = sum(int(r['length']) for r in rows if 'segment' in r)
                row.update(how=how, letters=str(letters), off_before=summary['off_before'],
                           off_after=summary['off_after'], strict_prime=True, end_digest=summary['end_digest'],
                           end_table=end.name)
                cur, landed = end, landed + 1
            else:
                keys = ('unit_on_lattice', 'collateral', 'others', 'others_exact', 'others_landed', 'error')
                row.update(how='none', reason='not strict prime' if rc == 0 else 'slp_apply.py failed',
                           **{k: safe_error(summary[k]) if k == 'error' else summary[k] for k in keys if k in summary})
            emit(out, row)
        emit(out, {'done': True, 'landed': landed, 'last': {'file': Path(cur).name, 'digest': digest(cur)}})


if __name__ == '__main__':
    main()

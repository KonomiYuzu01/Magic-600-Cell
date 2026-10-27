"""Stage 18: play the chunks of chunk_prep.py manifests twist by twist with stepchk.py (one twist per letter; a twist
that is not admissible raises), at most <jobs> at once, skipping chunks already done. Each chunk writes r<j>.json (the
stepchk report), e<j>.json (its end table) and done<j>.json (exit code, the SHA-256 of the chunk word handed to
stepchk and the child's CPU seconds); a chunk counts as done only with exit 0 and the manifest's word hash. Safe to
rerun after an interruption. Research only, headless; exact geometry.
Usage (from the repository root): python3 -I -B <this script> <jobs> <manifest dir> ..."""
import hashlib
import json
import os
import subprocess
import sys
import time
from pathlib import Path

CHK = 'research/jumbling/restoration/12-low-height/scripts/round22/stepchk.py'
ENV = dict(os.environ, OPENBLAS_NUM_THREADS='1', OMP_NUM_THREADS='1', MKL_NUM_THREADS='1')


def word_sha(path):
    return hashlib.sha256(json.dumps(json.loads(Path(path).read_text())['word']).encode()).hexdigest()


def todo(dirs):
    for d in dirs:
        m = json.loads((d / 'manifest.json').read_text())
        for c in m['chunks']:
            done = d / f'done{c["j"]}.json'
            if done.exists():
                row = json.loads(done.read_text())
                if row.get('rc') == 0 and row.get('word_sha256') == c['word_sha256']:
                    continue
            yield d, m, c


def main(argv):
    jobs, dirs = int(argv[0]), [Path(a) for a in argv[1:]]
    queue, running = list(todo(dirs)), {}
    print(json.dumps({'start': time.strftime('%H:%M:%S', time.gmtime()), 'chunks': len(queue)}), flush=True)
    while queue or running:
        while queue and len(running) < jobs:
            d, m, c = queue.pop(0)
            j = c['j']
            for f in (d / f'done{j}.json', d / f'r{j}.json', d / f'e{j}.json'):
                f.unlink(missing_ok=True)
            digest = word_sha(d / c['word'])
            cmd = ['nice', '-n', '19', sys.executable, '-I', '-B', CHK, c['start_table'], m['unit'], str(d / c['word']),
                   str(d / f'r{j}.json'), str(d / f'e{j}.json')]
            err = open(d / f'err{j}.txt', 'w')
            p = subprocess.Popen(cmd, stdout=subprocess.DEVNULL, stderr=err, env=ENV)
            running[p.pid] = (p, err, d, c, digest, time.time())  # keep the Popen so that nothing else reaps it
        pid, status, usage = os.wait4(-1, 0)
        if pid not in running:
            continue
        p, err, d, c, digest, t0 = running.pop(pid)
        err.close()
        rc = os.waitstatus_to_exitcode(status)
        p.returncode = rc  # reaped here
        row = {'j': c['j'], 'rc': rc, 'word_sha256': digest, 'cpu_s': round(usage.ru_utime + usage.ru_stime, 1),
               'wall_s': round(time.time() - t0, 1), 'letters': c['letters']}
        (d / f'done{c["j"]}.json').write_text(json.dumps(row) + '\n')
        print(json.dumps(dict(row, dir=d.name, end=time.strftime('%H:%M:%S', time.gmtime()))), flush=True)
    print(json.dumps({'all done': time.strftime('%H:%M:%S', time.gmtime())}), flush=True)


if __name__ == '__main__':
    main(sys.argv[1:])

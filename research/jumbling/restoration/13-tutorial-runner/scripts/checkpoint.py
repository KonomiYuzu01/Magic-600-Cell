"""Commit consistent tutorial-run snapshots beside a live run (headless research).

  checkpoint.py --run DIR --pid PID [--every 900] [--trailer TEXT ...] [--no-push]
  checkpoint.py --run DIR --restore

Only DIR-checkpoint is staged. Restore it into an empty DIR, then give the runner
its original command with --resume. No force pushes or history rewrites.
"""
import argparse
import gzip
import hashlib
import json
import math
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import time
import zlib

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import check_run as checker  # noqa: E402; share its path resolution, not runner decisions
import runner_api as api  # noqa: E402


class CheckpointError(ValueError):
    pass


# The checker's path keys, plus a kernel attempt's `dir` (high ends stuck with its record there).
RUN_PATH_KEYS = checker.PATH_KEYS | {'dir'}
ROUND_ERRORS = (CheckpointError, OSError, ValueError, KeyError, TypeError, EOFError, zlib.error)


def references(doc, trail=()):
    """Run-local checker path keys; repository provenance is never a run dependency."""
    if isinstance(doc, dict):
        for key, value in doc.items():
            where = trail + (key,)
            if where in (('config', 'acceptance_config'), ('config', 'kernel', 'd19'), ('config', 'scripts')):
                continue
            if key in RUN_PATH_KEYS and value is not None and not (key == 'unit' and type(value) is int):
                yield key, value, doc
            yield from references(value, where)
    elif isinstance(doc, list):
        for value in doc:
            yield from references(value, trail)


def file_sha(path, uncompressed=False):
    raw = path.read_bytes()
    if uncompressed and raw.startswith(b'\x1f\x8b'):
        raw = gzip.decompress(raw)
    return hashlib.sha256(raw).hexdigest()


def verify_copy(base, record):
    if checker.read_json(base / 'start-table.json')['digest'] != record['start']['digest']:
        raise ValueError('start table digest mismatch')
    for step in record['steps']:
        word = checker.inside(base, step['word'])
        if step.get('encoding') == 'ia16':
            actual = api.ia16_digest(word)
        else:
            letters = checker.read_json(word)['word']
            actual = {'sha256': checker.word_sha256(letters), 'letters': len(letters)}
        if actual != {'sha256': step['word_sha256'], 'letters': step['letters']}:
            raise ValueError('word digest mismatch')
        tree = step['tree']
        if file_sha(checker.inside(base, tree['file']), uncompressed=True) != tree['sha256']:
            raise ValueError('tree digest mismatch')
    for key, name, parent in references(record):
        path = checker.inside(base, name)
        if key == 'table':
            expected = parent.get('digest', parent.get('after', {}).get('digest'))
            if expected is None or checker.read_json(path)['digest'] != expected:
                raise ValueError('table digest mismatch')
        # Non-tree evidence (notably the lattice report) uses the file's own bytes.
        if key == 'file' and 'sha256' in parent and path.parts[-2:-1] != ('step-trees',):
            if file_sha(path) != parent['sha256']:
                raise ValueError('evidence digest mismatch')
        if key == 'dir' and parent.get('record_sha256') is not None:
            if file_sha(path / 'run.json') != parent['record_sha256']:
                raise ValueError('kernel record digest mismatch')


def copy_round(run, destination):
    """Read R once, capture its complete dependencies, then validate against that R."""
    raw = (run / 'run.json').read_bytes()
    record = json.loads(raw, object_pairs_hook=checker.no_duplicates)
    if record.get('format') != 'runner-run/3':
        raise ValueError('checkpoint requires runner-run/3')
    destination.mkdir()
    (destination / 'run.json').write_bytes(raw)
    names = {'start-table.json'}
    for key, name, _ in references(record):
        source = checker.inside(run, name)
        if key == 'dir' and not source.exists():
            continue                            # a kernel that failed before writing its record
        if source.is_dir():
            for path in source.rglob('*'):
                if path.is_file() and not path.name.endswith('.tmp'):
                    # Check directory members too: a symlink may not escape the run.
                    relative = path.relative_to(run).as_posix()
                    checker.inside(run, relative)
                    names.add(relative)
        else:
            names.add(name)
    for name in sorted(names):
        if Path(name).name.endswith('.tmp'):
            raise ValueError('record references a temporary file')
        source = checker.inside(run, name)
        target = checker.inside(destination, name)
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(source, target)
    verify_copy(destination, record)
    if (run / 'run.json').read_bytes() != raw:
        raise ValueError('run changed while copying')
    return raw, record


class Checkpointer:
    def __init__(self, run, *, no_push=False, trailers=()):
        self.run = Path(run).resolve()
        parent = self.run
        while not parent.exists():
            parent = parent.parent
        try:
            self.repo = Path(subprocess.check_output(['git', 'rev-parse', '--show-toplevel'], cwd=parent,
                                                    stderr=subprocess.DEVNULL).decode().strip()).resolve()
        except (OSError, subprocess.CalledProcessError) as exc:
            raise CheckpointError('run must be inside a Git repository') from exc
        self.checkpoint = self.run.with_name(self.run.name + '-checkpoint')
        if (not self.run.is_relative_to(self.repo) or not self.checkpoint.is_relative_to(self.repo)
                or self.run == self.repo or '.git' in self.run.relative_to(self.repo).parts):
            raise CheckpointError('run and checkpoint must be inside the repository')
        self.relative = self.checkpoint.relative_to(self.repo).as_posix()
        self.no_push, self.trailers = no_push, list(trailers)
        self.last_sha, self.pending_push = None, False
        self.guard()
        previous = self.checkpoint / 'run.json'
        committed = self.git('show', 'HEAD:' + self.relative + '/run.json', check=False)
        # A previous helper may have died after committing but before pushing.
        self.pending_push = not self.no_push and not committed.returncode
        if previous.is_file():
            if not committed.returncode and committed.stdout == previous.read_bytes():
                try:
                    verify_copy(self.checkpoint, checker.read_json(previous))
                except (OSError, ValueError, KeyError, TypeError, EOFError, zlib.error):
                    pass
                else:
                    self.last_sha = file_sha(previous)

    def git(self, *args, check=True):
        result = subprocess.run(['git', '--no-optional-locks', *args], cwd=self.repo,
                                stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        if check and result.returncode:
            raise CheckpointError('git ' + args[0] + ' failed: ' + result.stderr.decode(errors='replace').strip())
        return result

    def guard(self):
        head = self.git('symbolic-ref', '--quiet', '--short', 'HEAD', check=False)
        if head.returncode:
            raise CheckpointError('checkpoint refuses a detached HEAD')
        branch = head.stdout.decode().strip()
        if branch in ('main', 'claude/cowork-chat-memories-0j732y'):
            raise CheckpointError('checkpoint refuses protected branch ' + branch)
        staged = self.git('diff', '--cached', '--no-renames', '--name-only', '-z').stdout.split(b'\0')
        for name in staged:
            if name and not os.fsdecode(name).startswith(self.relative + '/'):
                raise CheckpointError('staged changes outside the checkpoint directory')

    def push(self):
        if self.no_push or not self.pending_push:
            return
        network = ('could not resolve', 'connection', 'unable to access', 'network', 'remote end hung up',
                   'could not read from remote repository', 'tls', 'timed out')
        for attempt in range(5):
            result = self.git('push', '-u', 'origin', 'HEAD', check=False)
            if not result.returncode:
                self.pending_push = False
                return
            message = result.stderr.decode(errors='replace')
            if not any(s in message.lower() for s in network):
                raise CheckpointError('git push failed: ' + message.strip())
            if attempt < 4:
                time.sleep(2 ** (attempt + 1))
        print('checkpoint push unavailable; will retry next round', file=sys.stderr)

    def round(self):
        self.guard()
        self.push()
        if not (self.run / 'run.json').is_file():
            return False
        if (file_sha(self.run / 'run.json') == self.last_sha and (self.checkpoint / 'run.json').is_file()
                and not self.git('diff', '--cached', '--quiet', '--', self.relative, check=False).returncode):
            return False
        for _ in range(5):
            # The run output is excluded from its code fingerprint. Keeping the
            # temporary copy here also makes directory renames stay on one filesystem.
            with tempfile.TemporaryDirectory(prefix='.checkpoint-', dir=self.run) as temporary:
                snapshot = Path(temporary) / 'snapshot'
                try:
                    raw, record = copy_round(self.run, snapshot)
                except (OSError, ValueError, KeyError, TypeError, EOFError, zlib.error):
                    continue
                self.guard()
                # Staging happens only after the complete verified copy replaces its predecessor.
                backup = Path(temporary) / 'previous'
                if self.checkpoint.exists():
                    os.replace(self.checkpoint, backup)
                try:
                    os.replace(snapshot, self.checkpoint)
                except OSError:
                    if backup.exists():
                        os.replace(backup, self.checkpoint)
                    raise
                self.git('add', '-A', '-f', '--', self.relative)
                if self.git('diff', '--cached', '--quiet', check=False).returncode:
                    message = f'Checkpoint {self.run.name}: step {len(record["steps"])}'
                    arguments = ['commit', '-m', message]
                    for trailer in self.trailers:
                        arguments += ['--trailer', trailer]
                    self.git(*arguments)
                    self.pending_push = not self.no_push
                self.last_sha = hashlib.sha256(raw).hexdigest()
                self.push()
                return True
        print('checkpoint skipped: no consistent copy in five attempts', file=sys.stderr)
        return False


def restore(run):
    run = Path(run).resolve()
    checkpoint = run.with_name(run.name + '-checkpoint')
    if run.exists() and (not run.is_dir() or any(run.iterdir())):
        raise CheckpointError('restore requires an empty run directory')
    # Validation uses the checkpoint's own bytes, before creating the destination.
    with tempfile.TemporaryDirectory(prefix='tutorial-restore-') as temporary:
        snapshot = Path(temporary) / 'snapshot'
        copy_round(checkpoint, snapshot)
        shutil.copytree(snapshot, run, dirs_exist_ok=True)


def alive(pid):
    try:
        os.kill(pid, 0)
    except ProcessLookupError:
        return False
    except PermissionError:
        return True
    # kill(pid, 0) also succeeds for an exited, unreaped child on Linux.
    try:
        if (Path('/proc') / str(pid) / 'stat').read_text().rsplit(')', 1)[1].split()[0] in ('Z', 'X'):
            return False
    except (OSError, IndexError):
        pass
    return True


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--run', required=True, type=Path)
    parser.add_argument('--pid', type=int)
    parser.add_argument('--every', type=float, default=900)
    parser.add_argument('--trailer', action='append', default=[])
    parser.add_argument('--no-push', action='store_true')
    parser.add_argument('--restore', action='store_true')
    args = parser.parse_args(argv)
    if not math.isfinite(args.every) or args.every <= 0:
        parser.error('every must be finite and positive')
    if not args.restore and (args.pid is None or args.pid <= 0):
        parser.error('pid must be positive')
    try:
        cp = Checkpointer(args.run, no_push=args.no_push, trailers=args.trailer)
        if args.restore:
            restore(cp.run)
        else:
            deadline = time.monotonic()
            while alive(args.pid):
                if time.monotonic() >= deadline:
                    try:
                        cp.round()
                    except ROUND_ERRORS as exc:
                        # A busy index, a file staged by the session or a rejected push: retry next round.
                        print('checkpoint round failed: ' + str(exc), file=sys.stderr)
                    deadline = time.monotonic() + args.every
                time.sleep(min(1, max(0, deadline - time.monotonic())))
            cp.round()
    except ROUND_ERRORS as exc:
        print('checkpoint refused: ' + str(exc), file=sys.stderr)
        return 2
    return 0


if __name__ == '__main__':
    sys.exit(main())

"""The Rust jumbling candidate (LP2, M5) behind the differential harness.

A standard-library client of the JSON-lines adapter of `rust/c600-jumble` (`src/bin/adapter.rs`). The
candidate is built from `assets/` alone, so nothing here reads J1 (PHS.18), and under the candidate import
policy (`differential/independence.py`) it imports no harness module either: the caller passes the harness's
`StepResult` as `step_result`. `RustJumbleEngine` serves `harness.compare` (start, apply, survey,
straddlers) and exposes the adapter's other requests for the tests: full outcomes with a budget or
protected pieces, pose-table records, identity blocks, stored words and the observations.
"""

from collections import namedtuple
import json
import os
from pathlib import Path
import shutil
import subprocess

REPO = Path(__file__).resolve().parents[4]
RUST = REPO / 'rust'
ASSETS = REPO / 'assets'
BINARY = RUST / 'target' / 'release' / ('c600-jumble-adapter.exe' if os.name == 'nt' else 'c600-jumble-adapter')
BUILD_SECONDS = 1200


class CandidateError(RuntimeError):
    """The adapter answered with an error: an input it does not read, or a broken invariant."""

    def __init__(self, kind, message):
        super().__init__(f'{kind}: {message}')
        self.kind = kind
        self.message = message


def build(cargo=None):
    """Build the adapter with `cargo build --release --locked --offline` in rust/ and return its path."""
    cargo = cargo or shutil.which('cargo')
    if cargo is None:
        raise CandidateError('setup', 'cargo is not on PATH, so the Rust candidate cannot be built')
    command = [cargo, 'build', '--release', '--locked', '--offline', '-p', 'c600-jumble',
               '--bin', 'c600-jumble-adapter']
    done = subprocess.run(command, cwd=RUST, capture_output=True, text=True, timeout=BUILD_SECONDS)
    if done.returncode != 0:
        raise CandidateError('setup', f'cargo build exited with {done.returncode}:\n{done.stderr[-4000:]}')
    if not BINARY.is_file():
        raise CandidateError('setup', f'cargo build left no adapter at {BINARY.relative_to(REPO)}')
    return BINARY


def _pairs(rows):
    return {key: value for key, value in rows}


# The fields of one step, in the harness's order; the default result type when no `step_result` is passed.
Step = namedtuple('Step', 'status moved moved_set digest lattice')


class RustJumbleEngine:
    """One adapter process; each request is one JSON line and each answer one JSON line."""

    def __init__(self, binary=BINARY, assets=ASSETS, name='rust-candidate', step_result=Step):
        self.name = name
        self._step_result = step_result
        self._process = subprocess.Popen([str(binary), str(assets)], stdin=subprocess.PIPE,
                                         stdout=subprocess.PIPE)
        ready = self._read()
        if not ready.get('ready'):
            self.close()
            raise CandidateError('setup', 'the adapter did not report ready')
        self.model_id = ready['model_id']
        self.load_ms = ready['load_ms']

    def _read(self):
        line = self._process.stdout.readline()
        if not line:
            raise CandidateError('engine', f'the adapter exited with {self._process.poll()}')
        answer = json.loads(line)
        if not answer.get('ok'):
            error = answer.get('error', {})
            raise CandidateError(error.get('kind', 'engine'), error.get('message', 'no message'))
        return answer['result']

    def call(self, op, **fields):
        """Send one request and return its result; an error answer raises CandidateError."""
        request = dict(fields, op=op)
        self._process.stdin.write(json.dumps(request, separators=(',', ':')).encode('utf-8') + b'\n')
        self._process.stdin.flush()
        return self._read()

    def close(self):
        if self._process.poll() is None:
            try:
                self._process.stdin.write(b'{"op":"quit"}\n')
                self._process.stdin.close()
            except OSError:
                pass
            try:
                self._process.wait(timeout=30)
            except subprocess.TimeoutExpired:
                self._process.kill()
                self._process.wait()
        self._process.stdout.close()

    def __enter__(self):
        return self

    def __exit__(self, *exc):
        self.close()

    # ---------------------------------------------------------------------------- the harness Engine
    def _step(self, result):
        return self._step_result(result['status'], result['moved'], result['moved_set'], result['digest'],
                                 result['lattice'])

    def start(self, menu):
        return self._step(self.call('start', menu=menu))

    def apply(self, step):
        return self._step(self.call('apply', step=step))

    def survey(self, grips):
        return _pairs(self.call('survey', grips=list(grips))['grips'])

    def straddlers(self, caps, budget=None):
        rows = _pairs(self.call('straddlers', caps=list(caps), budget=budget)['caps'])
        for row in rows.values():
            row['certificates'] = _pairs(row['certificates'])
        return rows

    # -------------------------------------------------------------------------------- other requests
    def outcome(self, step, *, budget=None, protect=None):
        """The full outcome of one step: the StepResult fields plus code, message, protection report,
        certificate, uncertain pieces and exact evaluations."""
        return self.call('apply', step=step, budget=budget, protect=list(protect or ()))

    def certify(self, cap, piece, certificate):
        return self.call('certify', cap=cap, piece=piece, certificate=certificate)['valid']

    def record(self):
        return self.call('record')

    def load(self, record):
        return self._step(self.call('load', record=record))

    def identity(self):
        return self.call('identity')

    def word(self, identity, letters, *, budget=None):
        return self.call('word', identity=identity, letters=[list(letter) for letter in letters],
                         budget=budget)

    def model(self):
        return self.call('model')

    # ------------------------------------------------------------------------------- observations
    @staticmethod
    def _letters(letters):
        return [list(letter) for letter in letters]

    def heights(self):
        return self.call('heights')

    def units(self, pieces=None):
        return self.call('units', pieces=None if pieces is None else list(pieces))

    def seam_facts(self, scope='all', *, chunk, budget, rows='none', cancel_after=None, eval_budget=None):
        return self.call('seam_facts', scope=scope, rows=rows, chunk=chunk, budget=budget,
                         cancel_after=cancel_after, eval_budget=eval_budget)

    def seam_totals(self, cells):
        return self.call('seam_totals', cells=list(cells))

    def prefix_observations(self, letters, *, chunk, budget, fields=None, cap=None, keep=False,
                            cancel_after=None, eval_budget=None):
        return self.call('prefix_observations', letters=self._letters(letters),
                         fields=None if fields is None else list(fields), cap=cap, keep=keep,
                         chunk=chunk, budget=budget, cancel_after=cancel_after, eval_budget=eval_budget)

    def word_check(self, letters, *, unit=None, cap=None, eval_budget=None):
        return self.call('word_check', letters=self._letters(letters),
                         unit=None if unit is None else list(unit), cap=cap, eval_budget=eval_budget)

    def keep(self, name):
        return self.call('keep', name=name)

    def use(self, name):
        return self._step(self.call('use', name=name))

    def release(self, name):
        return self.call('release', name=name)['released']

    def cayley(self, grip, omega):
        """The record of the exact Cayley rotation with parameter omega (three [a, b, d] triples) at grip."""
        return self.call('cayley', grip=grip, omega=[list(w) for w in omega])['record']

    # ---------------------------------------------------------------------- levels and end tests (M8)
    def project(self, level, *, state=None, values=False):
        """KN1.9's projection of the current state, or of the stored state `state`, to `level`: total,
        partial with what is missing, or refused; `values` adds the label or chamber arrays (base64 int32)."""
        return self.call('project', level=level, state=state, values=values)

    def sticker_test(self, *, state=None):
        return self.call('sticker_test', state=state)

    def retained_checkpoint(self, *, witness=None, journal=None, state=None):
        """Section 5's retained-state checkpoint from a witness word of signed primitive ids, or from journal
        records from solved."""
        return self.call('retained_checkpoint', witness=None if witness is None else list(witness),
                         journal=None if journal is None else list(journal), state=state)

    def label_checkpoint(self, reachability, *, nu=None, state=None):
        """A5's label checkpoint under a reachability claim {kind, evidence} (a witnessed claim names its
        `journal` from solved), with ν from the ν row when known."""
        return self.call('label_checkpoint', reachability=reachability,
                         nu=None if nu is None else [int(x) for x in nu], state=state)

    def retained_word(self, word, *, budget=None):
        """The primitive twists of a retained word (signed 1-based ids), refused whole at its first refused
        letter."""
        return self.call('retained_word', word=[int(m) for m in word], budget=budget)

    @staticmethod
    def _u32(values):
        """Integers as base64 little-endian u32, or None."""
        if values is None:
            return None
        import base64
        import struct
        values = [int(v) for v in values]
        return base64.b64encode(struct.pack('<%dI' % len(values), *values)).decode('ascii')

    def gauge(self, sigma=None, t=None, *, state=None):
        """X·γ of KN1.7 for the gauge (sigma, t), one value per piece (None for the identity), of the current
        state or of the stored state `state`; the image becomes the current state."""
        return self.call('gauge', sigma=self._u32(sigma), t=self._u32(t), state=state)

    def gauge_between(self, other, *, state=None):
        """A gauge γ with X·γ equal to the stored state `other` (sigma and t as base64 u32), or None."""
        return self.call('gauge_between', other=other, state=state)

    # ------------------------------------------------------------------------ derived states (M8)
    def derive(self, name, identity, base, *, letters=None, records=None, mode=None, protect=None):
        """KN2.11's derive(base, word): `base` is {'derived': name} or {'state': name or None, 'journal':
        records from solved}; the word is `letters` ([grip, item] pairs) or twist `records`; `mode` is 'net'
        (the default) or 'strict'. The answer is the derived value, kept under `name` and never adopted, or
        the refusal with its first refused prefix, outcome and certificates."""
        return self.call('derive', name=name, identity=identity, base=base,
                         letters=None if letters is None else self._letters(letters),
                         records=None if records is None else list(records), mode=mode,
                         protect=None if protect is None else [int(p) for p in protect])

    def import_table(self, name, record, identity, reachability):
        """KN2.14: a raw pose table as a derived value under `name`, with an open or promised claim."""
        return self.call('import', name=name, record=record, identity=identity, reachability=reachability)

    def compare(self, a, b, level):
        """ONE.25's comparison at `level` of two operands, each {'state': name or None} or {'derived': name}."""
        return self.call('compare', a=a, b=b, level=level)

    def read_derived(self, op, name, **fields):
        """`project`, `sticker_test`, `retained_checkpoint` or `label_checkpoint` on the derived value `name`."""
        return self.call(op, derived=name, **fields)

"""EH-10: rebuild the research lattice tables and compare their recorded ν.

The optional --cert-lb replay covers all 40 CERT-LB-V2 words. Evidence is
J1/source and fixture replay, never product-engine or native performance.
Research imports are lazy so an invalid --out is refused before loading J1.
"""

import sys

sys.dont_write_bytecode = True

import argparse
from dataclasses import asdict, dataclass
from functools import lru_cache
import gzip
import hashlib
import json
import os
from pathlib import Path
import platform
import subprocess
from time import perf_counter

ROOT = Path(__file__).resolve().parents[4]
JUMBLING = ROOT / 'research' / 'jumbling'
RESTORATION = JUMBLING / 'restoration'
STAGE04 = RESTORATION / '04-card5-lattice-invariants'
TABLE_NU = STAGE04 / 'certs' / 'table-nu.txt'
RESTORE_SUMMARY = RESTORATION / '09-general-method/earlier/x-e3-solved/restore-summary.json'
CERT_LB = STAGE04 / 'certs' / 'cert-lb-v2.json'
CERT_WORDS = STAGE04 / 'words' / 'generators.jsonl.gz'
TABLES = {
    'E3': RESTORATION / '01-closure-x-to-e3/e2-b-prime/D3/E3-table.json.gz',
    'I1': RESTORATION / '02-three-block-family/runs/I1/end-table.json.gz',
    'I2': RESTORATION / '02-three-block-family/runs/I2/end-table.json.gz',
}


class ReferenceDataError(ValueError):
    """A recorded reference is missing, partial or cannot be matched uniquely."""


class TableDigestError(ReferenceDataError):
    """The rebuilt pose state's digest differs from the table's digest."""

    def __init__(self, candidate, reference):
        self.candidate, self.reference = candidate, reference
        super().__init__(f'table state digest: candidate {candidate}, recorded {reference}')


@dataclass(frozen=True)
class TableNuRow:
    path: Path
    sha256: str
    row: tuple


@dataclass(frozen=True)
class NuComparison:
    table: str
    digest: str
    sha256: str
    value: tuple
    table_row: tuple
    summary_row: tuple
    difference: dict | None

    @property
    def ok(self):
        return self.difference is None

    def as_dict(self):
        result = asdict(self)
        result['ok'] = self.ok
        return result


@lru_cache(maxsize=1)
def _runtime():
    if str(JUMBLING) not in sys.path:
        sys.path.insert(0, str(JUMBLING))
    # First: census redirects the research centroid cache outside the repo.
    from differential.leads import census
    from differential.references import nu
    return census, nu


def _bytes(path):
    path = Path(path)
    raw = path.read_bytes()
    return gzip.decompress(raw) if path.suffix == '.gz' else raw


def table_state(path):
    """Rebuild (State, original table record), as stage 04's table_nu.py does."""
    census, _ = _runtime()
    import numpy as np
    from sim.kplus import matrix_from_json
    from sim.state import State

    record = json.loads(_bytes(path))
    state = State(census.restore.ctx, menu=census.menu())
    ids = np.asarray([state._intern_matrix(matrix_from_json(m)) for m in record['poses']], np.int32)
    indices = np.asarray(record['pose_id'], np.int64)
    if (indices.shape != state.pose_id.shape or np.any(indices < 0)
            or np.any(indices >= len(ids))):
        raise ReferenceDataError('table pose_id: wrong piece count or pose index')
    state.pose_id = ids[indices]
    state._compact()
    state.journal, state._anchor_cache = [], {}
    actual = state.digest()
    if actual != record['digest']:
        raise TableDigestError(actual, record['digest'])
    return state, record


def table_nu_rows():
    """Match every TABLE-NU row bijectively to a table by decompressed SHA-256."""
    _, nu = _runtime()
    records = [json.loads(line) for line in TABLE_NU.read_text(encoding='utf-8').splitlines()
               if line.strip()]
    by_sha = {}
    for record in records:
        sha = record['sha256']
        if sha in by_sha:
            raise ReferenceDataError(f'TABLE-NU duplicate sha256: {sha}')
        if record['digest_ok'] is not True or record['lattice'] is not True:
            raise ReferenceDataError(f'TABLE-NU {sha}: digest_ok/lattice is not true')
        by_sha[sha] = record
    matched = []
    for name, path in TABLES.items():
        sha = hashlib.sha256(_bytes(path)).hexdigest()
        if sha not in by_sha:
            raise ReferenceDataError(f'table {name}: sha256 {sha} matches no TABLE-NU row')
        matched.append(TableNuRow(path, sha, nu.row_from_research(by_sha.pop(sha)['nu'])))
    if by_sha:
        raise ReferenceDataError(f'TABLE-NU row matches no table: {next(iter(by_sha))}')
    if len(matched) != 3:
        raise ReferenceDataError(f'TABLE-NU requires three tables, got {len(matched)}')
    return tuple(matched)


def restore_nu_rows():
    """The three independent nu_input records, keyed by exact state digest."""
    _, nu = _runtime()
    records = json.loads(RESTORE_SUMMARY.read_text(encoding='utf-8'))['runs']
    rows = {record['input_digest']: nu.row_from_research(record['nu_input']) for record in records}
    if len(records) != 3 or len(rows) != 3:
        raise ReferenceDataError('restore-summary.json requires three unique input digests')
    return rows


def _difference(value, reference, source):
    _, nu = _runtime()
    difference = nu.first_difference(value, reference)
    if difference is None:
        return None
    name, candidate, expected = difference
    return {'reference': source, 'coordinate': name, 'candidate': candidate,
            'reference_value': expected}


def compare_table(path):
    """Compare ν with TABLE-NU and restore-summary's nu_input for this state."""
    _, nu = _runtime()
    sha = hashlib.sha256(_bytes(path)).hexdigest()
    matches = [row for row in table_nu_rows() if row.sha256 == sha]
    if len(matches) != 1:
        raise ReferenceDataError(f'table sha256 {sha}: expected one TABLE-NU match, got {len(matches)}')
    state, record = table_state(path)
    summary = restore_nu_rows()
    if record['digest'] not in summary:
        raise ReferenceDataError(f"restore-summary.json has no input digest {record['digest']}")
    value = nu.nu(state)
    nu.check_fixed(value)
    row, summary_row = matches[0].row, summary[record['digest']]
    difference = (_difference(value, row, 'TABLE-NU') or
                  _difference(value, summary_row, 'restore-summary.json:nu_input'))
    # Reports contain repository-relative fixture names, never private paths.
    return NuComparison(str(matches[0].path.relative_to(ROOT)), record['digest'], sha,
                        value, row, summary_row, difference)


def cert_words():
    """All 40 words paired by name with complete CERT-LB-V2 results."""
    _, nu = _runtime()
    words = [json.loads(line) for line in _bytes(CERT_WORDS).splitlines() if line.strip()]
    certificate = json.loads(CERT_LB.read_text(encoding='utf-8'))
    records = certificate['words']
    by_name = {record['name']: record for record in records}
    if (len(words) != 40 or len(records) != 40 or len(by_name) != 40
            or len({word['name'] for word in words}) != 40
            or certificate['summary']['words'] != 40):
        raise ReferenceDataError('CERT-LB-V2 requires 40 unique words and results')
    paired = []
    for word in words:
        record = by_name.pop(word['name'], None)
        if record is None:
            raise ReferenceDataError(f"CERT-LB-V2 has no result for {word['name']}")
        if ('letters' in word) == ('records' in word):
            raise ReferenceDataError(f"{word['name']}: expected letters or exact records")
        length = len(word.get('letters', word.get('records')))
        if length != record['length']:
            raise ReferenceDataError(f"{word['name']}: length {length}, recorded {record['length']}")
        for field in ('admissible_every_prefix', 'end_on_lattice'):
            if record[field] is not True:
                raise ReferenceDataError(f"{word['name']}: recorded {field} is not true")
        difference = _difference(nu.row_from_research(word['claimed']),
                                 nu.row_from_research(record['nu']), 'CERT-LB-V2')
        if difference:
            raise ReferenceDataError(f"{word['name']}: claimed ν differs: {difference}")
        paired.append((word, record))
    if by_name:
        raise ReferenceDataError(f'CERT-LB-V2 result has no word: {next(iter(by_name))}')
    return tuple(paired)


def replay_cert_word(word, reference):
    """Replay the caller's word from solved; a partial replay can never pass."""
    census, nu = _runtime()
    from differential.j1_state import twists_from_letters
    from sim.state import State
    from sim.twists import Twist

    started = perf_counter()
    state = State(census.restore.ctx, menu=census.menu())
    if 'letters' in word:
        twists = twists_from_letters(state.menu, word['letters'])
    else:
        twists = [Twist.from_record(state.ctx, record) for record in word['records']]
    difference = None
    for index, twist in enumerate(twists):
        outcome = state.apply(twist, journal=False)
        if not outcome.applied:
            difference = {'reference': 'CERT-LB-V2', 'field': 'admissible_every_prefix',
                          'letter_index': index, 'candidate': outcome.status, 'reference_value': True}
            break
    admissible = difference is None
    lattice = admissible and bool(state.is_lattice())
    value = nu.nu(state) if lattice else None
    if admissible and not lattice:
        difference = {'reference': 'CERT-LB-V2', 'field': 'end_on_lattice',
                      'candidate': False, 'reference_value': reference['end_on_lattice']}
    if lattice:
        nu.check_fixed(value)
        difference = _difference(value, nu.row_from_research(reference['nu']), 'CERT-LB-V2')
    length_ok = len(twists) == reference['length']
    if difference is None and not length_ok:
        difference = {'reference': 'CERT-LB-V2', 'field': 'length',
                      'candidate': len(twists), 'reference_value': reference['length']}
    if difference:
        difference = dict(word=word['name'], **difference)
    return {'name': word['name'], 'length': len(twists), 'seconds': perf_counter() - started,
            'admissible_every_prefix': admissible, 'end_on_lattice': lattice,
            'nu': None if value is None else nu.nonzero(value),
            'nu_agrees_with_certificate': lattice and
            _difference(value, nu.row_from_research(reference['nu']), 'CERT-LB-V2') is None,
            'ok': difference is None, 'difference': difference}


def output_directory(path, filenames):
    """Refuse repository output, including output-file links, before any computation."""
    directory = Path(path).resolve()
    if directory.is_relative_to(ROOT):
        raise ValueError('--out must be outside the repository')
    for name in filenames:
        if (directory / name).resolve().is_relative_to(ROOT):
            raise ValueError(f'--out must be outside the repository: {name} resolves inside it')
    return directory


def run_metadata():
    import numpy as np
    try:
        commit = subprocess.run(['git', 'rev-parse', 'HEAD'], cwd=ROOT, capture_output=True,
                                text=True, check=True).stdout.strip()
    except (OSError, subprocess.CalledProcessError):
        commit = None
    return {'evidence_kind': 'source and fixture replay; cloud timings of research code',
            'python': platform.python_version(), 'numpy': np.__version__,
            'cpu_count': os.cpu_count(), 'commit': commit}


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--out', required=True)
    parser.add_argument('--cert-lb', action='store_true')
    args = parser.parse_args(argv)
    names = ['table-nu.json'] + (['cert-lb.jsonl', 'cert-lb-summary.json'] if args.cert_lb else [])
    try:
        directory = output_directory(args.out, names)
    except ValueError as error:
        parser.error(str(error))
    rows = [compare_table(path).as_dict() for path in TABLES.values()]
    metadata = run_metadata()
    directory.mkdir(parents=True, exist_ok=True)
    (directory / 'table-nu.json').write_text(
        json.dumps({'tables': rows, 'ok': all(row['ok'] for row in rows), **metadata}, indent=2) + '\n',
        encoding='utf-8')
    ok = all(row['ok'] for row in rows)
    if args.cert_lb:
        results = []
        with (directory / 'cert-lb.jsonl').open('w', encoding='utf-8') as target:
            for word, reference in cert_words():
                result = replay_cert_word(word, reference)
                results.append(result)
                target.write(json.dumps(result) + '\n')
                target.flush()
        summary = {'words': len(results), 'passed': sum(row['ok'] for row in results),
                   'all_admissible': all(row['admissible_every_prefix'] for row in results),
                   'all_on_lattice': all(row['end_on_lattice'] for row in results),
                   'all_nu_agree': all(row['nu_agrees_with_certificate'] for row in results),
                   'ok': len(results) == 40 and all(row['ok'] for row in results), **metadata}
        (directory / 'cert-lb-summary.json').write_text(json.dumps(summary, indent=2) + '\n', encoding='utf-8')
        ok = ok and summary['ok']
    return 0 if ok else 1


if __name__ == '__main__':
    raise SystemExit(main())

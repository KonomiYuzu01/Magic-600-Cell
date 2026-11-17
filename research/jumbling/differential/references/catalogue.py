"""DT-J1 O7 (a), JI.13: the Rust engine's shipped correction catalogue, written from the research library.

The 36 corrections of the pinned research library (`corrections.LIBRARY`, whose decompressed SHA-256 is pinned) ship
in `c600_jumble::corrections` as constructions that the solver selects by id. Each carries its family, its literal
word as (grip, item) letters of the pinned I_a menu, matched exactly by J1 (`j1_state.letters_from_records`), and its
45-value ν row in A5's order. The file is bound to the model ID of `assets/manifest.json`, the I_a menu identity and
order digest, and the engine's contract revision.

`--out DIR` writes `corrections-I_a.json` to DIR, which must be outside the repository as for every command here.
The shipped copy is `rust/c600-jumble/data/corrections-I_a.json`; `tests/test_jumbling_rust_corrections.py`
regenerates it with `catalogue_bytes()` and compares the bytes. Nothing here chooses, ranks or combines corrections.
Research imports are lazy, so an invalid --out is refused before J1 loads.
"""

import sys

sys.dont_write_bytecode = True

import argparse  # noqa: E402
import json  # noqa: E402
from pathlib import Path  # noqa: E402

ROOT = Path(__file__).resolve().parents[4]
JUMBLING = ROOT / 'research' / 'jumbling'
if str(JUMBLING) not in sys.path:
    sys.path.insert(0, str(JUMBLING))

from differential.references import nu_refs  # noqa: E402

FILENAME = 'corrections-I_a.json'
SHIPPED = ROOT / 'rust' / 'c600-jumble' / 'data' / FILENAME
FORMAT = 'c600-jumble-corrections-v1'
# The Rust engine's stamp (`c600_jumble::CONTRACT_REVISION`); the ν reference stamps A1 to A5.
CONTRACT_REVISION = 'state-contract 2026-11-15 A1-A6'


def document():
    """The catalogue as a JSON-ready mapping, corrections in id order."""
    census, nu = nu_refs._runtime()
    from differential.references import corrections

    library = corrections.load_library()
    manifest = json.loads((ROOT / 'assets' / 'manifest.json').read_bytes())
    return {
        'format': FORMAT,
        'binding': {
            'model_id': manifest['model_id'],
            'menu_identity': census.menu().identity,
            'menu_order': census.I_A_ORDER_DIGEST,
            'contract_revision': CONTRACT_REVISION,
        },
        'source': {
            'library': corrections.LIBRARY.relative_to(ROOT).as_posix(),
            'sha256': library.sha256,
        },
        'names': list(nu.NAMES),
        'corrections': [
            {'id': c.id, 'family': c.family, 'row': list(c.row), 'letters': [list(x) for x in c.letters]}
            for c in library
        ],
    }


def text(doc):
    """Deterministic ASCII bytes: the head on one line, then one correction per line."""
    head = {key: value for key, value in doc.items() if key != 'corrections'}
    opening = json.dumps(head, separators=(',', ':'), ensure_ascii=True)[:-1] + ',"corrections":['
    rows = [json.dumps(c, separators=(',', ':'), ensure_ascii=True) for c in doc['corrections']]
    return (opening + '\n' + ',\n'.join(rows) + '\n]}\n').encode('ascii')


def catalogue_bytes():
    """The bytes the shipped catalogue must have."""
    return text(document())


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument('--out', required=True, help=f'a directory outside the repository for {FILENAME}')
    args = parser.parse_args(argv)
    try:
        directory = nu_refs.output_directory(args.out, [FILENAME])
    except ValueError as error:
        parser.error(str(error))
    data = catalogue_bytes()
    directory.mkdir(parents=True, exist_ok=True)
    (directory / FILENAME).write_bytes(data)
    return 0


if __name__ == '__main__':
    raise SystemExit(main())

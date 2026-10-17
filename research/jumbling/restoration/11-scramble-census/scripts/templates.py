# Unpack the template library of stage 06 (block words found at the mid-depth samples for the shapes da4b45 (111
# pieces), fca5f0 (86) and abd791 (87)) into <out dir>, and write <out dir>/templates.json with the paths there, as
# fxrun.py expects. Run from the repository root:  python3 -I -B templates.py <out dir>
import gzip, json, shutil, sys
from pathlib import Path
src = Path('research/jumbling/restoration/06-block-family/data/templates')
out = Path(sys.argv[1]).resolve(); out.mkdir(parents=True, exist_ok=True)
for f in sorted(src.iterdir()):
    if f.suffix == '.gz':
        with gzip.open(f, 'rb') as a, open(out / f.stem, 'wb') as b:
            shutil.copyfileobj(a, b)
    else:
        shutil.copy(f, out / f.name)
t = json.loads((src / 'templates-fxrun1.json').read_text())
for e in t:
    e['table'] = str(out / Path(e['table']).name)
    e['block'] = str(out / Path(e['block']).name)
    e['rows'] = [str(out / Path(r).name) for r in e['rows']]
    for p in [e['table'], e['block']] + e['rows']:
        assert Path(p).exists(), p
(out / 'templates.json').write_text(json.dumps(t, indent=1) + '\n')
print(len(t), 'templates', sorted({e['shape'] for e in t}))

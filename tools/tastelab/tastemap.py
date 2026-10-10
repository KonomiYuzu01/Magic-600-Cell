import marimo

__generated_with = "0.25.0"
app = marimo.App(width="medium", app_title="Taste Lab taste map")

with app.setup:
    # The owner's taste map (marimo notebook; tastelab environment). Private: it reads
    # the Taste Lab data folder and writes only <data>/reports/tastemap-<day>.*
    #   python tools/tastelab/tastemap.py [--data DIR]    write the report
    #   python tools/tastelab/explore.py [--data DIR]     explore it in the marimo editor
    # The editor caches every output; outside script mode the notebook shows nothing
    # private unless those caches stay in the data folder (explore.py arranges that).
    import os
    import sys
    from pathlib import Path

    _here = os.path.normcase(str(Path(__file__).resolve().parent))
    sys.path[:] = [str(Path(__file__).resolve().parents[1])] + [
        p for p in sys.path if os.path.normcase(os.path.abspath(p or os.curdir)) != _here]

    from tastelab import common, embed, learn, mapping, seeds
    from tastelab.store import DB_NAME, Store


@app.cell
def _():
    import marimo as mo
    return (mo,)


@app.cell
def _(mo):
    root = common.data_root(mo.cli_args().get("data"))
    _exposed = mo.app_meta().mode not in ("script", "test") and not mapping.cache_contained(root)
    mo.stop(_exposed, mo.md("Open the taste map with `python tools/tastelab/explore.py`, which keeps "
                            "marimo's caches in the private data folder."))
    _missing = not (root / DB_NAME).is_file()
    if _missing:
        print("No Taste Lab data yet: fetch and rate first.")
    mo.stop(_missing, mo.md("No Taste Lab data yet: fetch and rate first."))
    store = Store(root)
    probes = seeds.load_probes(seeds.ensure(root, seeds.PROBES_FILE, seeds.PROBES_DEFAULT))
    return probes, root, store


@app.cell
def _(probes, store):
    model = learn.TasteModel(store, embed.MODEL_ID, record=False)
    report, sheets = mapping.build_report(store, probes, embed.MODEL_ID, model, day=common.local_day())
    return report, sheets


@app.cell
def _(report, root, sheets, store):
    path = mapping.write_report(root / "reports", report, sheets, store.thumb_path)
    print("Taste map written in the private data folder.")
    return (path,)


@app.cell
def _(mo, path, report, sheets):
    _images = [mo.image(src=path.parent / f"tastemap-{report['day']}-{name}.jpg", caption=name) for name in sheets]
    mo.vstack([mo.md(mapping.markdown(report)), *_images])
    return


if __name__ == "__main__":
    app.run()

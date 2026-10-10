"""Taste-map analysis: named axes, liked themes, anti-goals and like rates (tastelab environment).

tastemap.py (marimo) calls `build_report` and `write_report`; the report is
private and stays in <data>/reports/. The category is a report label only, never
a model feature.

Axes: each probe pair (negative pole, positive pole) in probes.yaml gives a text
direction, v(positive) - v(negative) normalised, in the embedding space of the
images. The class A rated images are projected on it and the projection is scored
against like (1) and dislike (0): an AUC above 0.5 means the owner prefers the
positive pole, below 0.5 the negative pole. The map keeps the MIN_AXES to
MAX_AXES strongest axes, skips an axis whose projection correlates above
REDUNDANT with an axis already kept, and marks an axis weak when its bootstrap
interval contains 0.5. The liked poles and liked themes are material for the
metaphor (M2); the disliked poles and themes are candidates for anti-goals (M3).
"""
from __future__ import annotations

import io
import json
import os
import sys
import tempfile
from pathlib import Path

import numpy as np

from tastelab import common, embed

MIN_AXES = 4
MAX_AXES = 6
REDUNDANT = 0.8
MIN_PER_CLASS = 10          # labelled ratings per class before axes are reported
MIN_THEME = 5               # images before a set of liked or disliked images gets themes
BOOTSTRAP = 200
LEVEL = 0.9
SHEET_TILE = 160
SHEET_COLUMNS = 8
BORDER = {"like": (46, 125, 50), "love": (212, 160, 23), "dislike": (198, 40, 40), "skip": (120, 120, 120), None: (60, 60, 60)}
PRIVATE = ("Private: this report stays under work/loop-memory/tastelab/. It may show tier B images; "
           "never commit, publish or upload it.")


def auc(scores, y) -> float:
    """Rank AUC of `scores` for binary labels `y` (ties count half); NaN without both classes."""
    from scipy.stats import rankdata

    scores, y = np.asarray(scores, dtype=np.float64), np.asarray(y)
    pos, neg = int((y == 1).sum()), int((y == 0).sum())
    if pos == 0 or neg == 0:
        return float("nan")
    ranks = rankdata(scores)
    return float((ranks[y == 1].sum() - pos * (pos + 1) / 2) / (pos * neg))


def bootstrap_interval(scores, y, *, rounds: int = BOOTSTRAP, level: float = LEVEL, seed: int = 0) -> tuple[float, float]:
    """Percentile interval of the AUC, resampling likes and dislikes separately."""
    scores, y = np.asarray(scores, dtype=np.float64), np.asarray(y)
    rng = np.random.default_rng(seed)
    pos, neg = np.flatnonzero(y == 1), np.flatnonzero(y == 0)
    if len(pos) == 0 or len(neg) == 0:
        return (float("nan"), float("nan"))
    values = []
    for _ in range(rounds):
        idx = np.concatenate([rng.choice(pos, len(pos)), rng.choice(neg, len(neg))])
        values.append(auc(scores[idx], y[idx]))
    tail = (1 - level) / 2 * 100
    return (float(np.percentile(values, tail)), float(np.percentile(values, 100 - tail)))


def quintile_rates(scores, y) -> list:
    """Like rate in each fifth of the projection, lowest first (None for an empty fifth)."""
    scores, y = np.asarray(scores, dtype=np.float64), np.asarray(y)
    order = np.argsort(scores, kind="stable")
    return [round(float(y[part].mean()), 3) if len(part) else None for part in np.array_split(order, 5)]


def axis_directions(probes, text_vectors: dict) -> list[tuple[str, str, np.ndarray]]:
    """(negative pole, positive pole, unit direction) for every probe pair whose phrased texts have vectors."""
    out = []
    for negative, positive in probes.axes:
        a, b = text_vectors.get(probes.phrase(negative)), text_vectors.get(probes.phrase(positive))
        if a is None or b is None:
            continue
        d = np.asarray(b, dtype=np.float64) - np.asarray(a, dtype=np.float64)
        norm = np.linalg.norm(d)
        if norm > 1e-9:
            out.append((negative, positive, d / norm))
    return out


def rank_axes(X, y, axes, *, min_axes: int = MIN_AXES, max_axes: int = MAX_AXES, redundant: float = REDUNDANT,
              seed: int = 0) -> list[dict]:
    """The strongest non-redundant axes, at least `min_axes` when that many exist, weak ones marked."""
    X, y = np.asarray(X, dtype=np.float64), np.asarray(y)
    scored = []
    for negative, positive, d in axes:
        s = X @ d
        a = auc(s, y)
        if np.isnan(a):
            continue
        lo, hi = bootstrap_interval(s, y, seed=seed)
        scored.append({"negative": negative, "positive": positive, "auc": round(a, 3),
                       "interval": [round(lo, 3), round(hi, 3)], "strength": round(abs(a - 0.5) * 2, 3),
                       "preferred": positive if a >= 0.5 else negative,
                       "avoided": negative if a >= 0.5 else positive,
                       "weak": bool(lo <= 0.5 <= hi), "quintiles": quintile_rates(s, y),
                       "_scores": s, "_direction": d})
    scored.sort(key=lambda r: (r["weak"], -r["strength"]))
    kept = []
    for r in scored:
        if len(kept) >= max_axes or (r["weak"] and len(kept) >= min_axes):
            break
        if any(abs(np.corrcoef(r["_scores"], k["_scores"])[0, 1]) > redundant for k in kept):
            continue
        kept.append(r)
    return kept


def explained_share(direction, axis_dirs) -> float | None:
    """Share of the unit preference direction that lies in the span of the kept axes (0 to 1)."""
    if direction is None or not len(axis_dirs):
        return None
    w = np.asarray(direction, dtype=np.float64)
    w = w / max(np.linalg.norm(w), 1e-12)
    axes = np.stack([np.asarray(d, dtype=np.float64) for d in axis_dirs], axis=1)
    q, singular, _ = np.linalg.svd(axes, full_matrices=False)
    tolerance = max(axes.shape) * np.finfo(np.float64).eps * singular[0]
    q = q[:, singular > tolerance]
    return round(float(np.clip(np.sum((q.T @ w) ** 2), 0, 1)), 3)


def separate_terms(liked, avoided):
    """De-duplicated liked and avoided term lists; a term found on both sides is left out of both (TL-B08)."""
    shared = set(liked) & set(avoided)
    return ([t for t in dict.fromkeys(liked) if t not in shared],
            [t for t in dict.fromkeys(avoided) if t not in shared])


def term_ranking(direction, term_vectors: dict, k: int = 5) -> tuple[list, list]:
    """Disjoint top/bottom terms, or one complete ranking and an empty list when fewer than 2*k exist."""
    if direction is None or not term_vectors:
        return [], []
    terms = list(term_vectors)
    m = embed.normalise(np.stack([term_vectors[t] for t in terms])).astype(np.float64)
    w = np.asarray(direction, dtype=np.float64)
    cos = m @ (w / max(np.linalg.norm(w), 1e-12))
    order = np.argsort(-cos, kind="stable")
    ranked = [(terms[i], round(float(cos[i]), 3)) for i in order]
    if len(ranked) < 2 * k:
        return ranked, []
    return ranked[:k], ranked[::-1][:k]


def themes(X, shas, term_vectors: dict, *, k_max: int = MAX_AXES, examples: int = 6, seed: int = 0) -> list[dict]:
    """K-means groups of a set of images, largest first, each named by its nearest probe terms."""
    from sklearn.cluster import KMeans

    X = np.asarray(X, dtype=np.float64)
    if len(X) < MIN_THEME:
        return []
    k = max(1, min(k_max, len(X) // 10))
    labels = KMeans(n_clusters=k, n_init=3, random_state=seed).fit_predict(X) if k > 1 else np.zeros(len(X), dtype=int)
    terms = list(term_vectors)
    tm = embed.normalise(np.stack([term_vectors[t] for t in terms])).astype(np.float64) if terms else None
    out = []
    for c in range(k):
        members = np.flatnonzero(labels == c)
        if not len(members):
            continue
        centre = X[members].mean(axis=0)
        centre = centre / max(np.linalg.norm(centre), 1e-12)
        near = members[np.argsort(-(X[members] @ centre), kind="stable")][:examples]
        names = [terms[i] for i in np.argsort(-(tm @ centre), kind="stable")[:3]] if tm is not None else []
        out.append({"size": int(len(members)), "terms": names, "examples": [shas[i] for i in near]})
    out.sort(key=lambda t: -t["size"])
    return out


def category_rates(rows) -> list[dict]:
    """Like rate per category from (category, verdict) rows; skips are counted but not in the rate."""
    table = {}
    for category, verdict in rows:
        table.setdefault(category, {"like": 0, "dislike": 0, "skip": 0})[verdict] += 1
    out = []
    for category, c in table.items():
        labelled = c["like"] + c["dislike"]
        out.append({"category": category, "likes": c["like"], "dislikes": c["dislike"], "skips": c["skip"],
                    "like_rate": round(c["like"] / labelled, 3) if labelled else None})
    out.sort(key=lambda r: (r["like_rate"] is None, -(r["like_rate"] or 0), r["category"]))
    return out


def cache_contained(root) -> bool:
    """True when marimo keeps this notebook's caches and exports inside the data folder `root`.

    marimo writes session caches (every cell output) and exports to a __marimo__
    folder beside the notebook, or, while sys.pycache_prefix is set
    (PYTHONPYCACHEPREFIX), to a mirror of the notebook's folder under that prefix
    (notebook_output_dir in marimo/_utils/paths.py). explore.py sets the prefix to
    <data>/marimo."""
    prefix = getattr(sys, "pycache_prefix", None)
    if not prefix:
        return False
    try:
        cache = common.check_input(prefix, kind="notebook cache")
        data = common.data_root(root)
    except common.Refused:
        return False
    return cache.is_relative_to(data)


def build_report(store, probes, model_id: str, model, *, day: str, seed: int = 0) -> tuple[dict, dict]:
    """The taste map of the store's current ratings and the contact sheets to draw: (report, sheets).

    `model` is a TasteModel with record=False; `sheets` maps a sheet name to rows of
    (sha256, verdict) tiles."""
    images = {m.sha256: m for m in store.images() if not store.is_blocked(m.sha256)}
    ratings = [r for r in store.ratings() if r.sha256 in images]
    category = {sha: m.category for sha, m in images.items()}
    counts = store.counts()
    state = model.refresh(force=True)
    direction = model.direction()
    labelled = [(r.sha256, r.verdict) for r in ratings
                if images[r.sha256].tier == "A" and r.verdict in ("like", "dislike")]
    blobs = store.embeddings(model_id, [s for s, _ in labelled])
    labelled = [(s, v) for s, v in labelled if s in blobs]
    dim = len(next(iter(blobs.values()))) // 4 if blobs else 0
    X = embed.from_blobs([blobs[s] for s, _ in labelled], dim) if blobs else np.zeros((0, 0), dtype=np.float32)
    y = np.array([1 if v == "like" else 0 for _, v in labelled], dtype=int)
    shas = [s for s, _ in labelled]
    phrased = {t: probes.phrase(t) for t in probes.texts()}
    stored = store.text_embeddings(model_id, list(phrased.values()))
    text_vectors = {p: embed.from_blobs([b], len(b) // 4)[0] for p, b in stored.items()}
    term_vectors = {t: text_vectors[phrased[t]] for t in probes.terms if phrased[t] in text_vectors}
    report = {
        "day": day, "model_id": model_id, "counts": counts,
        "model": {"ready": state.ready, "auc": state.auc, "auc_delta": state.auc_delta, "stable": state.stable,
                  "weak": state.weak, "likes": state.likes, "dislikes": state.dislikes},
        "missing_probe_vectors": len(set(phrased.values()) - set(text_vectors)),
        "axes": [], "explained": None, "favoured_terms": [], "disfavoured_terms": [], "ranked_terms": [],
        "placements": [],
        "liked_themes": [], "anti_goal_themes": [], "metaphor_material": [], "anti_goals": [],
        "categories": category_rates((category.get(r.sha256, "?"), r.verdict) for r in ratings),
        "notes": {"like": [], "dislike": [], "skip": []},
        "loved": [], "private_loved": 0,
        "screen": {s: {"checked": c, "discarded": d} for s, (c, d) in sorted(store.screen_counts().items())},
        "model_runs": [{k: run[k] for k in ("ts", "ratings", "likes", "dislikes", "auc", "auc_delta", "stable")}
                       for run in store.model_runs() if run["model"] == model_id][-20:],
        "insufficient": None,
    }
    private_loved = []
    for r in ratings[::-1]:
        if r.note and len(report["notes"][r.verdict]) < 50:
            report["notes"][r.verdict].append({"sha256": r.sha256, "note": r.note, "ts": r.ts})
        if r.love:
            if images[r.sha256].tier == "A" and len(report["loved"]) < 50:
                report["loved"].append({"sha256": r.sha256, "note": r.note, "ts": r.ts})
            elif images[r.sha256].tier == "B":
                private_loved.append(r.sha256)
    report["private_loved"] = len(private_loved)
    sheets = {}
    for name, candidates in (("loved", [r["sha256"] for r in report["loved"]]), ("private-loved", private_loved)):
        if candidates:
            sheets[name] = [[(sha, "love") for sha in candidates[i:i + SHEET_COLUMNS]]
                            for i in range(0, len(candidates), SHEET_COLUMNS)]
    kept_directions = []
    likes, dislikes = int(y.sum()), int(len(y) - y.sum())
    if min(likes, dislikes) < MIN_PER_CLASS:
        report["insufficient"] = (f"Axes need at least {MIN_PER_CLASS} likes and {MIN_PER_CLASS} dislikes with "
                                  f"class A embeddings; there are {likes} and {dislikes}.")
    else:
        kept = rank_axes(X, y, axis_directions(probes, text_vectors), seed=seed)
        verdict = dict(labelled)
        for i, axis in enumerate(kept, 1):
            order = np.argsort(axis.pop("_scores"), kind="stable")
            axis["sheet"] = f"axis-{i}"
            sheets[axis["sheet"]] = [[(shas[j], verdict[shas[j]]) for j in order[::-1][:SHEET_COLUMNS]],
                                     [(shas[j], verdict[shas[j]]) for j in order[:SHEET_COLUMNS]]]
        kept_directions = [a.pop("_direction") for a in kept]
        report["explained"] = explained_share(direction, kept_directions)
        report["axes"] = kept
    report["favoured_terms"], report["disfavoured_terms"] = term_ranking(direction, term_vectors)
    if not report["disfavoured_terms"]:
        report["ranked_terms"], report["favoured_terms"] = report["favoured_terms"], []
    for key, label in (("liked_themes", 1), ("anti_goal_themes", 0)):
        idx = np.flatnonzero(y == label)
        found = themes(X[idx], [shas[i] for i in idx], term_vectors, seed=seed) if len(idx) else []
        report[key] = found
        if found:
            sheets[key.replace("_", "-")] = [[(s, "like" if label else "dislike") for s in t["examples"]] for t in found]
    report["metaphor_material"], report["anti_goals"] = separate_terms(
        [a["preferred"] for a in report["axes"] if not a["weak"]] + [t for t, _ in report["favoured_terms"]]
        + [t for theme in report["liked_themes"] for t in theme["terms"]],
        [a["avoided"] for a in report["axes"] if not a["weak"]] + [t for t, _ in report["disfavoured_terms"]]
        + [t for theme in report["anti_goal_themes"] for t in theme["terms"]])
    private_blobs = store.embeddings(model_id, [sha for sha, m in images.items() if m.tier == "B"])
    private_shas = list(private_blobs)
    if private_shas:
        private_X = embed.from_blobs(list(private_blobs.values()), len(next(iter(private_blobs.values()))) // 4)
        probabilities = model.predict_vectors(private_X)
        projections = private_X @ np.stack(kept_directions, axis=1) if kept_directions else np.zeros((len(private_shas), 0))
        report["placements"] = [{"sha256": sha, "probability": round(float(probabilities[i]), 3),
                                  "axes": [round(float(s), 3) for s in projections[i]]}
                                 for i, sha in enumerate(private_shas)]
        verdicts = {r.sha256: r.verdict for r in ratings}
        sheets["private-references"] = [[(sha, verdicts.get(sha)) for sha in private_shas[i:i + SHEET_COLUMNS]]
                                         for i in range(0, len(private_shas), SHEET_COLUMNS)]
    return report, sheets


def contact_sheet(rows, thumb, *, tile: int = SHEET_TILE) -> bytes:
    """A JPEG grid of thumbnails, one row per list, each tile framed by its verdict's colour."""
    from PIL import Image, ImageOps

    width = max((len(r) for r in rows), default=1)
    sheet = Image.new("RGB", (max(width, 1) * tile, max(len(rows), 1) * tile), (24, 24, 24))
    for r, row in enumerate(rows):
        for c, (sha, verdict) in enumerate(row):
            frame = Image.new("RGB", (tile, tile), BORDER.get(verdict, BORDER[None]))
            try:
                with Image.open(thumb(sha)) as im:
                    frame.paste(ImageOps.fit(im.convert("RGB"), (tile - 8, tile - 8)), (4, 4))
            except OSError:
                pass
            sheet.paste(frame, (c * tile, r * tile))
    out = io.BytesIO()
    sheet.save(out, format="JPEG", quality=85)
    return out.getvalue()


def _cell(text) -> str:
    return " ".join(str(text).split()).replace("|", "\\|")


def markdown(report: dict) -> str:
    """The report as Markdown; contact sheets are linked by file name next to it."""
    day, m = report["day"], report["model"]
    stem = f"tastemap-{day}"
    lines = [f"# Taste map, {day}", "", PRIVATE, ""]
    c = report["counts"]
    auc_text = f"{m['auc']:.2f}" if m["auc"] is not None else "n/a"
    state = "stable" if m["stable"] else ("plateau with a weak AUC" if m["weak"] else "still learning")
    lines += [f"Ratings: {c['total']} ({c['like']} like, {c['dislike']} dislike, {c['skip']} skip, {c['love']} loved). "
              f"Class A model `{report['model_id']}`: AUC {auc_text}, {state}.", "",
              "## Loved images (candidates for annotated references and nexus cards)", ""]
    if report["loved"]:
        lines += [f"- {r['sha256'][:12]}: {_cell(r['note'] or 'No note.')}" for r in report["loved"]]
        lines += ["", f"![loved images]({stem}-loved.jpg)"]
    else:
        lines.append("None yet.")
    if report["private_loved"]:
        lines += ["", f"Private loved images: {report['private_loved']}.",
                  f"![private loved images]({stem}-private-loved.jpg)"]
    lines += ["", "Classifier, axes, themes and terms are fitted on class A only. Class B references are placed locally.", ""]
    if report["missing_probe_vectors"]:
        lines += [f"{report['missing_probe_vectors']} probe texts have no vector yet; run embed.py.", ""]
    lines += ["## Axes", ""]
    if report["insufficient"]:
        lines += [report["insufficient"], ""]
    elif not report["axes"]:
        lines += ["No probe axis separates likes from dislikes yet.", ""]
    else:
        lines += ["| # | axis (negative to positive) | preferred | AUC (90 % interval) | like rate by fifth, low to high | |",
                  "|---|---|---|---|---|---|"]
        for i, a in enumerate(report["axes"], 1):
            fifths = " ".join("-" if q is None else f"{q:.2f}" for q in a["quintiles"])
            lines.append(f"| {i} | {_cell(a['negative'])} to {_cell(a['positive'])} | {_cell(a['preferred'])} | "
                         f"{a['auc']:.2f} ({a['interval'][0]:.2f} to {a['interval'][1]:.2f}) | {fifths} | "
                         f"{'weak' if a['weak'] else ''} |")
        lines += ["", "Each axis sheet shows the rated images furthest towards the positive pole (top row) and the "
                  "negative pole (bottom row); a green frame is a like, a red frame a dislike.", ""]
        lines += [f"- Axis {i}: ![axis {i}]({stem}-{a['sheet']}.jpg)" for i, a in enumerate(report["axes"], 1)]
        lines.append("")
        if report["explained"] is not None:
            lines += [f"The kept axes span {report['explained']:.0%} of the learned preference direction; "
                      "the rest has no name in probes.yaml yet.", ""]
    if report["ranked_terms"]:
        lines += ["## Probe term ranking", "", "One ranking along the class A preference direction: "
                  + ", ".join(f"{_cell(t)} ({v:+.2f})" for t, v in report["ranked_terms"]) + ".", ""]
    lines += ["## What the owner likes (material for the metaphor, M2)", ""]
    if report["favoured_terms"]:
        lines.append("Probe terms along the preference direction: "
                     + ", ".join(f"{_cell(t)} ({v:+.2f})" for t, v in report["favoured_terms"]) + ".")
    for i, t in enumerate(report["liked_themes"], 1):
        lines.append(f"- Liked theme {i} ({t['size']} images): {', '.join(_cell(x) for x in t['terms'])}")
    if report["liked_themes"]:
        lines += ["", f"![liked themes]({stem}-liked-themes.jpg)"]
    lines += ["", "## What the owner avoids (anti-goal candidates, M3)", ""]
    if report["disfavoured_terms"]:
        lines.append("Probe terms against the preference direction: "
                     + ", ".join(f"{_cell(t)} ({v:+.2f})" for t, v in report["disfavoured_terms"]) + ".")
    for i, t in enumerate(report["anti_goal_themes"], 1):
        lines.append(f"- Disliked theme {i} ({t['size']} images): {', '.join(_cell(x) for x in t['terms'])}")
    if report["anti_goal_themes"]:
        lines += ["", f"![disliked themes]({stem}-anti-goal-themes.jpg)"]
    if report["placements"]:
        lines += ["", "## Private reference placements", "",
                  "Class B references scored on the class A model; axis coordinates follow the order above.", "",
                  "| image id | probability of like | axis coordinates |", "|---|---|---|"]
        for p in report["placements"]:
            coords = ", ".join(f"{s:+.3f}" for s in p["axes"]) or "n/a"
            lines.append(f"| {p['sha256']} | {p['probability']:.3f} | {coords} |")
        lines += ["", f"![private references]({stem}-private-references.jpg)"]
    lines += ["", "## Like rate per category", "", "The category is a report label only, never a model feature.", "",
              "| category | likes | dislikes | skips | like rate |", "|---|---|---|---|---|"]
    for r in report["categories"]:
        rate = "n/a" if r["like_rate"] is None else f"{r['like_rate']:.2f}"
        lines.append(f"| {_cell(r['category'])} | {r['likes']} | {r['dislikes']} | {r['skips']} | {rate} |")
    lines += ["", "## Notes", ""]
    for verdict in ("like", "dislike", "skip"):
        for n in report["notes"][verdict]:
            lines.append(f"- {verdict}: {_cell(n['note'])}")
    if not any(report["notes"].values()):
        lines.append("No notes yet.")
    lines += ["", "## Content screen", "", "| source | checked | discarded |", "|---|---|---|"]
    lines += [f"| {_cell(s)} | {v['checked']} | {v['discarded']} |" for s, v in report["screen"].items()]
    lines += ["", "## Model runs (latest 20)", "", "| time | ratings | likes | dislikes | AUC | change | stable |",
              "|---|---|---|---|---|---|---|"]
    for r in report["model_runs"]:
        a = "n/a" if r["auc"] is None else f"{r['auc']:.3f}"
        d = "n/a" if r["auc_delta"] is None else f"{r['auc_delta']:+.3f}"
        lines.append(f"| {r['ts']} | {r['ratings']} | {r['likes']} | {r['dislikes']} | {a} | {d} | {'yes' if r['stable'] else ''} |")
    return "\n".join(lines) + "\n"


def _write(path: Path, data: bytes) -> None:
    fd, tmp = tempfile.mkstemp(dir=path.parent, prefix=".part-")
    try:
        with os.fdopen(fd, "wb") as fh:
            fh.write(data)
        os.replace(tmp, path)
    except BaseException:
        Path(tmp).unlink(missing_ok=True)
        raise


def write_report(reports_dir, report: dict, sheets: dict, thumb) -> Path:
    """Write tastemap-<day>.md, .json and the contact sheets into `reports_dir`; returns the Markdown path."""
    reports_dir = Path(reports_dir)
    reports_dir.mkdir(parents=True, exist_ok=True)
    stem = f"tastemap-{report['day']}"
    for name, rows in sheets.items():
        _write(reports_dir / f"{stem}-{name}.jpg", contact_sheet(rows, thumb))
    data = json.dumps(report, indent=2, ensure_ascii=False, default=lambda o: o.item() if hasattr(o, "item") else str(o))
    _write(reports_dir / f"{stem}.json", data.encode("utf-8"))
    path = reports_dir / f"{stem}.md"
    _write(path, markdown(report).encode("utf-8"))
    return path

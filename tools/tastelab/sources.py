"""Eight approved adapters; terms checked on 1 October 2026.

Class A requires item-level open-licence evidence and creator/credit metadata.
Class B is private-reference on archive, demozoo and safebooru only. Safebooru
queries force rating:general and exclude every tag below; each post is checked
again. Per-item APIs skip known ids before requesting them. AIC paging stops at
unmatched scores. Demozoo shares a cursor for queries with the same listing
filter, and skips lost productions without an item request.

Hosts and intervals are fixed below. Adapters fetch metadata only; the pipeline
downloads on the same allowlist after admission. Openverse uses its API's
thumbnail proxy, never an arbitrary provider host. NASA uses search data only.
"""
from __future__ import annotations

import html
import json
import re
from dataclasses import dataclass
from html.parser import HTMLParser
from urllib.parse import parse_qs, quote, urlencode

from tastelab import common, licences, net

TIER_A_LICENCES = ("CC0-1.0", "CC-PDM-1.0", "public-domain", "US-Gov-PD")
PUBLIC_DOMAIN_LICENCES = TIER_A_LICENCES
PRIVATE_REFERENCE = "private-reference"
AIC_MIN_SCORE = 1.0
MET_SEARCH = "https://collectionapi.metmuseum.org/public/collection/v1.1/search"
CC0_URL = "https://creativecommons.org/publicdomain/zero/1.0/"
PDM_URL = "https://creativecommons.org/publicdomain/mark/1.0/"
SAFEBOORU_EXCLUDED_TAGS = (
    "nude", "nipples", "underwear", "panties", "bra", "lingerie", "swimsuit", "bikini", "cleavage",
    "breasts", "ass", "pantyshot", "upskirt", "bath", "bathing", "towel", "bondage", "blood", "gore",
    "guro", "injury", "corpse", "suggestive", "partially_visible_vulva", "sexually_suggestive",
)
NASA_CENTRES = {"NASA", "JSC", "KSC", "GSFC", "JPL", "ARC", "AFRC", "DFRC", "GRC", "LARC", "MSFC",
                "SSC", "HQ", "GLENN", "GODDARD", "KENNEDY", "JOHNSON", "LANGLEY", "AMES", "MARSHALL",
                "STENNIS", "ARMSTRONG", "JET PROPULSION LABORATORY"}


@dataclass
class Candidate:
    source: str
    source_id: str
    image_url: str
    page_url: str | None
    licence: str
    licence_url: str | None
    attribution: str
    title: str | None = None
    anime: bool = False
    keywords: tuple = ()


class _Text(HTMLParser):
    def __init__(self):
        super().__init__(convert_charrefs=True)
        self.parts = []

    def handle_data(self, data):
        self.parts.append(data)


def clean(value):
    if isinstance(value, list):
        value = "; ".join(v for v in value if isinstance(v, str))
    if not isinstance(value, str):
        return ""
    parser = _Text()
    parser.feed(value)
    return " ".join(html.unescape(" ".join(parser.parts)).split())[:300]


class Adapter:
    name = ""
    tier = "A"
    hosts = ()
    image_hosts = ()
    min_interval = 1.0
    terms_url = ""

    def headers(self):
        return {}

    def cursor_key(self, query):
        return query

    def _json(self, client, url):
        response = client.get(url, hosts=self.hosts, min_interval=self.min_interval, max_bytes=4 * 1024 * 1024,
                              accept="application/json", extra_headers=self.headers())
        try:
            return json.loads(response.body)
        except (ValueError, UnicodeError):
            raise net.NetError(f"{self.name}: invalid metadata JSON") from None

    def _admitted(self, items):
        return [item for item in items if admit(item, self.tier)]


def admit(candidate, tier):
    source = candidate.source
    adapter = ADAPTERS.get(source)
    if adapter is None or tier not in common.TIERS or getattr(candidate, "tier", tier) != tier:
        return False
    if (not isinstance(candidate.source_id, str) or not 1 <= len(candidate.source_id) <= 200
            or not licences.printable(candidate.source_id) or not clean(candidate.attribution)):
        return False
    try:
        net._host(candidate.image_url, adapter.image_hosts)
    except net.NetError:
        return False
    if adapter.tier == "A":
        return (licences.licence_ok(source, candidate.licence, candidate.licence_url)
                and licences.page_url_ok(source, candidate.page_url))
    return (tier == "B" and candidate.licence == PRIVATE_REFERENCE
            and candidate.licence_url == adapter.terms_url and licences.https_url(candidate.page_url) is not None
            and net.host_allowed(licences.https_url(candidate.page_url).hostname, adapter.hosts))


class Wikimedia(Adapter):
    name = "wikimedia"
    hosts = ("commons.wikimedia.org", "upload.wikimedia.org", "thumb.wikimedia.org")
    image_hosts = ("upload.wikimedia.org", "thumb.wikimedia.org")
    min_interval = 0.5

    def search(self, client, query, cursor, *, page_size=20, seen=None):
        params = {"action": "query", "format": "json", "formatversion": 2, "generator": "search", "gsrsearch": query,
                  "gsrnamespace": 6, "gsrlimit": min(page_size, 50), "gsroffset": int(cursor or 0),
                  "prop": "imageinfo", "iiprop": "url|extmetadata", "iiurlwidth": 960, "maxlag": 5}
        data = self._json(client, "https://commons.wikimedia.org/w/api.php?" + urlencode(params))
        pages = data.get("query", {}).get("pages", [])
        if isinstance(pages, dict):
            pages = list(pages.values())
        items = []
        for page in pages:
            infos = page.get("imageinfo") or []
            if not infos:
                continue
            info, source_id = infos[0], str(page.get("pageid", ""))
            if seen and seen(source_id):
                continue
            metadata = info.get("extmetadata", {})
            field = lambda k: metadata.get(k, {}).get("value", "")
            raw = clean(field("License") or field("LicenseShortName")).lower().replace(" ", "-")
            licence, url = "", field("LicenseUrl")
            if raw in ("cc0", "cc0-1.0"):
                licence, url = "CC0-1.0", CC0_URL
            elif raw in ("pdm", "public-domain-mark", "public-domain-mark-1.0", "cc-pdm-1.0"):
                licence, url = "CC-PDM-1.0", PDM_URL
            elif raw in ("pd", "public-domain"):
                licence, url = "public-domain", info.get("descriptionurl")
            else:
                match = re.fullmatch(r"cc-(by(?:-sa)?)-([0-9]+\.[0-9]+)(?:-([a-z]{2,5}))?", raw)
                if match:
                    family, version, port = match.groups()
                    licence = "CC-" + family.upper() + "-" + version + ("-" + port.upper() if port else "")
                    if not url:
                        url = f"https://creativecommons.org/licenses/{family}/{version}/" + (port + "/" if port else "")
            items.append(Candidate(self.name, source_id, info.get("thumburl", ""), info.get("descriptionurl"),
                                   licence, url, clean(field("Artist")) or clean(field("Credit")),
                                   clean(field("ObjectName")) or clean(page.get("title")),
                                   keywords=tuple(clean(field("Categories")).split("|"))))
        offset = data.get("continue", {}).get("gsroffset")
        return self._admitted(items), str(offset) if offset is not None else None


class Met(Adapter):
    name = "met"
    hosts = ("collectionapi.metmuseum.org", "images.metmuseum.org")
    image_hosts = ("images.metmuseum.org",)
    min_interval = 0.5

    def search(self, client, query, cursor, *, page_size=20, seen=None):
        offset, limit = int(cursor or 0), min(page_size, 500)
        data = self._json(client, MET_SEARCH + "?" + urlencode({"q": query, "hasImages": "true", "offset": offset, "limit": limit}))
        ids, items = data.get("objectIDs") or [], []
        for ident in ids:
            ident = str(ident)
            if seen and seen(ident):
                continue
            row = self._json(client, "https://collectionapi.metmuseum.org/public/collection/v1/objects/" + quote(ident, safe=""))
            if row.get("isPublicDomain") is not True or not row.get("primaryImageSmall"):
                continue
            items.append(Candidate(self.name, ident, row["primaryImageSmall"], row.get("objectURL"), "CC0-1.0", CC0_URL,
                                   clean(row.get("artistDisplayName")) or clean(row.get("creditLine")), clean(row.get("title")),
                                   keywords=tuple(clean(tag.get("term")) for tag in row.get("tags") or [])))
        next_offset = offset + len(ids)
        return self._admitted(items), str(next_offset) if ids and next_offset < data.get("total", 0) else None


class Aic(Adapter):
    name = "aic"
    hosts = ("api.artic.edu", "www.artic.edu")
    image_hosts = ("www.artic.edu",)

    def headers(self):
        return {"AIC-User-Agent": common.USER_AGENT}

    def search(self, client, query, cursor, *, page_size=20, seen=None):
        page = int(cursor or 1)
        fields = "id,title,artist_title,artist_display,credit_line,image_id,is_public_domain"
        data = self._json(client, "https://api.artic.edu/api/v1/artworks/search?" +
                          urlencode({"q": query, "page": page, "limit": min(page_size, 100), "fields": fields}))
        items, unmatched = [], False
        for row in data.get("data", []):
            if row.get("_score", 0) < AIC_MIN_SCORE:
                unmatched = True
                break
            ident = str(row.get("id", ""))
            if seen and seen(ident):
                continue
            if row.get("is_public_domain") is not True or not row.get("image_id"):
                continue
            items.append(Candidate(self.name, ident, "https://www.artic.edu/iiif/2/" + quote(row["image_id"], safe="") +
                                   "/full/843,/0/default.jpg", "https://www.artic.edu/artworks/" + ident,
                                   "CC0-1.0", CC0_URL, clean(row.get("artist_title")) or clean(row.get("artist_display")) or
                                   clean(row.get("credit_line")), clean(row.get("title"))))
        more = not unmatched and page < data.get("pagination", {}).get("total_pages", 1)
        return self._admitted(items), str(page + 1) if more else None


class Openverse(Adapter):
    name = "openverse"
    hosts = image_hosts = ("api.openverse.org",)
    min_interval = 3.0

    def search(self, client, query, cursor, *, page_size=20, seen=None):
        page = int(cursor or 1)
        data = self._json(client, "https://api.openverse.org/v1/images/?" +
                          urlencode({"q": query, "page": page, "page_size": min(page_size, 20), "mature": "false"}))
        items = []
        for row in data.get("results", []):
            if row.get("mature") is not False or row.get("unstable__sensitivity") != []:
                continue
            ident = str(row.get("id", ""))
            if seen and seen(ident):
                continue
            code, version = row.get("license"), str(row.get("license_version", ""))
            licence = {"cc0": "CC0-1.0", "pdm": "CC-PDM-1.0", "by": "CC-BY-" + version,
                       "by-sa": "CC-BY-SA-" + version}.get(code, "")
            items.append(Candidate(self.name, ident, row.get("thumbnail", ""), "https://openverse.org/image/" + ident,
                                   licence, row.get("license_url"), clean(row.get("creator")) or clean(row.get("attribution")),
                                   clean(row.get("title")), keywords=tuple(clean(t.get("name")) for t in row.get("tags") or [])))
        return self._admitted(items), str(page + 1) if page < data.get("page_count", 1) else None


class Nasa(Adapter):
    name = "nasa"
    hosts = ("images-api.nasa.gov", "images-assets.nasa.gov")
    image_hosts = ("images-assets.nasa.gov",)
    terms_url = "https://www.nasa.gov/nasa-brand-center/images-and-media/"

    def search(self, client, query, cursor, *, page_size=20, seen=None):
        page = int(cursor or 1)
        data = self._json(client, "https://images-api.nasa.gov/search?" +
                          urlencode({"q": query, "media_type": "image", "page": page, "page_size": min(page_size, 100)}))
        items = []
        collection = data.get("collection", {})
        for item in collection.get("items", []):
            rows = item.get("data") or []
            if not rows:
                continue
            row = rows[0]
            ident, centre = str(row.get("nasa_id", "")), clean(row.get("center")).upper()
            if seen and seen(ident):
                continue
            credit = clean(row.get("photographer"))
            rights = " ".join(clean(row.get(k)) for k in ("photographer", "secondary_creator", "description"))
            if (centre not in NASA_CENTRES and not re.search(r"\bNASA\b", credit, re.I)) or re.search(
                    r"copyright|courtesy|\(c\)|©|\b(?:ESA|NOAA|JAXA|DLR|CSA|ISRO)\b", rights, re.I):
                continue
            preview = next((link.get("href", "") for link in item.get("links", []) if
                            link.get("rel") == "preview" and link.get("render") == "image"), "")
            items.append(Candidate(self.name, ident, preview, "https://images.nasa.gov/details/" + ident, "US-Gov-PD",
                                   self.terms_url, "NASA" + ("/" + centre if centre and centre != "NASA" else "") +
                                   ("; " + credit if credit else ""), clean(row.get("title")),
                                   keywords=tuple(clean(t) for t in row.get("keywords") or [])))
        more = any(link.get("rel") == "next" for link in collection.get("links", []))
        return self._admitted(items), str(page + 1) if more else None


class Archive(Adapter):
    name, tier = "archive", "B"
    hosts = image_hosts = ("archive.org", "*.us.archive.org")
    terms_url = "https://archive.org/about/terms.php"

    def search(self, client, query, cursor, *, page_size=20, seen=None):
        page = int(cursor or 1)
        params = {"q": query + " AND mediatype:image", "fl[]": ["identifier", "title"], "rows": min(page_size, 100),
                  "page": page, "output": "json"}
        data = self._json(client, "https://archive.org/advancedsearch.php?" + urlencode(params, doseq=True))
        rows, items = data.get("response", {}).get("docs", []), []
        for row in rows:
            ident = row.get("identifier", "")
            if not ident or seen and seen(ident):
                continue
            detail = self._json(client, "https://archive.org/metadata/" + quote(ident, safe=""))
            filename = next((f.get("name") for f in detail.get("files", []) if
                             str(f.get("name", "")).lower().endswith((".jpg", ".jpeg", ".png", ".webp", ".gif"))), None)
            if not filename:
                continue
            metadata = detail.get("metadata", {})
            items.append(Candidate(self.name, ident, "https://archive.org/download/" + quote(ident, safe="") + "/" +
                                   quote(filename, safe="/"), "https://archive.org/details/" + quote(ident, safe=""),
                                   PRIVATE_REFERENCE, self.terms_url, clean(metadata.get("creator")) or "Internet Archive: " + ident,
                                   clean(metadata.get("title")) or clean(row.get("title"))))
        total = data.get("response", {}).get("numFound", 0)
        return self._admitted(items), str(page + 1) if rows and page * min(page_size, 100) < total else None


class Demozoo(Adapter):
    name, tier = "demozoo", "B"
    hosts = ("demozoo.org", "media.demozoo.org")
    image_hosts = ("media.demozoo.org",)
    min_interval = 10.0
    terms_url = "https://demozoo.org/about/"

    def cursor_key(self, query):
        params = parse_qs(query)
        chosen = {"supertype": "graphics"}
        if params.get("supertype", [""])[0] in ("graphics", "production"):
            chosen["supertype"] = params["supertype"][0]
        if params.get("platform", [""])[0].isdigit():
            chosen["platform"] = params["platform"][0]
        return urlencode(sorted(chosen.items()))

    def search(self, client, query, cursor, *, page_size=20, seen=None):
        url = cursor or "https://demozoo.org/api/v1/productions/?" + self.cursor_key(query)
        data = self._json(client, url)
        items = []
        for row in data.get("results", []):
            ident = str(row.get("id", ""))
            if "lost" in row.get("tags", []) or seen and seen(ident):
                continue
            detail = self._json(client, "https://demozoo.org/api/v1/productions/" + quote(ident, safe="") + "/")
            screenshots = detail.get("screenshots") or []
            if not screenshots:
                continue
            shot = screenshots[0]
            image_url = shot.get("original_url") or shot.get("thumbnail_url") or shot.get("url", "")
            credit = "; ".join(clean(n.get("name")) for n in detail.get("author_nicks", []))
            items.append(Candidate(self.name, ident, image_url, detail.get("demozoo_url"), PRIVATE_REFERENCE,
                                   self.terms_url, credit or "Demozoo: " + ident, clean(detail.get("title"))))
        return self._admitted(items), data.get("next")


class Safebooru(Adapter):
    name, tier = "safebooru", "B"
    hosts = image_hosts = ("safebooru.org",)
    terms_url = "https://safebooru.org/index.php?page=static&s=terms"

    def search(self, client, query, cursor, *, page_size=20, seen=None):
        page, limit = int(cursor or 0), min(page_size, 100)
        tags = [t for t in query.split() if not t.startswith("rating:")]
        tags += ["rating:general"] + ["-" + t for t in SAFEBOORU_EXCLUDED_TAGS]
        url = "https://safebooru.org/index.php?" + urlencode({"page": "dapi", "s": "post", "q": "index", "json": 1,
                                                           "pid": page, "limit": limit, "tags": " ".join(tags)})
        data = self._json(client, url)
        items = []
        for row in data:
            keywords = tuple(str(row.get("tags", "")).lower().split())
            ident = str(row.get("id", ""))
            if row.get("rating") not in ("general", "safe") or set(keywords).intersection(SAFEBOORU_EXCLUDED_TAGS):
                continue
            if seen and seen(ident):
                continue
            items.append(Candidate(self.name, ident, row.get("sample_url") or row.get("file_url", ""),
                                   "https://safebooru.org/index.php?page=post&s=view&id=" + ident, PRIVATE_REFERENCE,
                                   self.terms_url, "Safebooru: " + ident, clean(row.get("title")), True, keywords))
        return self._admitted(items), str(page + 1) if len(data) == limit else None


ADAPTERS = {adapter.name: adapter for adapter in (Wikimedia(), Openverse(), Met(), Aic(), Nasa(), Archive(), Demozoo(), Safebooru())}

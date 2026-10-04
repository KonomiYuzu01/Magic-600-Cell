"""The versioned class A licence and page-link contract, also used by the page."""
import re
import unicodedata
from urllib.parse import urlsplit

TABLE = {
    "version": 1,
    "licences": [
        {"family": "CC0-1.0", "pattern": r"^CC0-1\.0$", "prefixes": ["https://creativecommons.org/publicdomain/zero/1.0/"]},
        {"family": "CC-PDM-1.0", "pattern": r"^CC-PDM-1\.0$", "prefixes": ["https://creativecommons.org/publicdomain/mark/1.0/"]},
        {"family": "CC-BY", "pattern": r"^CC-BY-[0-9]+\.[0-9]+(-[A-Z]{2,5})?$", "prefixes": ["https://creativecommons.org/licenses/by/"]},
        {"family": "CC-BY-SA", "pattern": r"^CC-BY-SA-[0-9]+\.[0-9]+(-[A-Z]{2,5})?$", "prefixes": ["https://creativecommons.org/licenses/by-sa/"]},
        {"family": "public-domain", "pattern": r"^public-domain$", "sources": ["wikimedia"], "prefixes": ["https://commons.wikimedia.org/wiki/"]},
        {"family": "US-Gov-PD", "pattern": r"^US-Gov-PD$", "sources": ["nasa"], "prefixes": ["https://www.nasa.gov/"]}
    ],
    "pageHosts": {
        "wikimedia": ["commons.wikimedia.org"],
        "openverse": ["openverse.org"],
        "met": ["www.metmuseum.org"],
        "aic": ["www.artic.edu"],
        "nasa": ["images.nasa.gov"]
    }
}


def printable(text) -> bool:
    return isinstance(text, str) and not any(unicodedata.category(c) == "Cc" for c in text)


def https_url(url):
    if not printable(url) or not 1 <= len(url) <= 500 or any(c.isspace() for c in url):
        return None
    try:
        parts = urlsplit(url)
        if parts.scheme != "https" or not parts.hostname or parts.username is not None or parts.password is not None:
            return None
        if ":" in parts.netloc or "\\" in parts.netloc:
            return None
        return parts
    except ValueError:
        return None


def licence_family(licence):
    matches = [row for row in TABLE["licences"] if isinstance(licence, str) and re.fullmatch(row["pattern"], licence)]
    return matches[0]["family"] if len(matches) == 1 else None


def class_a_source(source) -> bool:
    return isinstance(source, str) and source in TABLE["pageHosts"]


def licence_ok(source, licence, url) -> bool:
    family = licence_family(licence)
    if not class_a_source(source) or family is None or https_url(url) is None:
        return False
    row = next(row for row in TABLE["licences"] if row["family"] == family)
    if "sources" in row and source not in row["sources"]:
        return False
    return any((url if url.endswith("/") else url + "/").startswith(prefix) for prefix in row["prefixes"])


def page_url_ok(source, url) -> bool:
    parts = https_url(url) if class_a_source(source) else None
    return parts is not None and parts.hostname in TABLE["pageHosts"][source]

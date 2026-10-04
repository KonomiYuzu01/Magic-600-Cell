"""The licence table exists twice: `tools/tastelab/licences.py` for the local tool and
`tools/tastelab/page/licences.js` for the page (plan section 6.1, TLPLAN-03). This test
checks that both copies hold the same table, in the same key order, and give the same
answer for every case below, including URLs that Python's and the browser's URL parsers
read differently."""

import json
import shutil
import subprocess
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "tools"))

from tastelab import licences  # noqa: E402

NODE = shutil.which("node")
PAGE_MODULE = (ROOT / "tools" / "tastelab" / "page" / "licences.js").as_uri()

# Reads [source, licence, licenceUrl, pageUrl] cases from stdin and prints the table and
# [family, licenceOk, pageUrlOk] for each case.
SCRIPT = """
const { TABLE, licenceFamily, licenceOk, pageUrlOk } = await import(process.argv[1]);
let input = "";
for await (const chunk of process.stdin) input += chunk;
const answers = JSON.parse(input).map(([source, licence, url, page]) =>
  [licenceFamily(licence), licenceOk(source, licence, url), pageUrlOk(source, page)]);
process.stdout.write(JSON.stringify({ table: TABLE, answers }));
"""

CC0 = "https://creativecommons.org/publicdomain/zero/1.0/"
PDM = "https://creativecommons.org/publicdomain/mark/1.0/"
BY = "https://creativecommons.org/licenses/by/4.0/"
BY_SA = "https://creativecommons.org/licenses/by-sa/4.0/"
WM_PAGE = "https://commons.wikimedia.org/wiki/File:Example.jpg"
NASA = "https://www.nasa.gov/nasa-brand-center/images-and-media/"

LICENCES = ["CC0-1.0", "CC-PDM-1.0", "CC-BY-4.0", "CC-BY-2.5-SCOTLAND", "CC-BY-3.0-IGO", "CC-BY-SA-4.0",
            "CC-BY-SA-2.0-UK", "public-domain", "US-Gov-PD", "cc0-1.0", "CC0-1.0 ", "CC-BY-4", "CC-BY-SA",
            "CC-BY-4.0-de", "private-reference", "", "CC-BY-4.0\n"]

LICENCE_URLS = [CC0, CC0.rstrip("/"), PDM, BY, BY.rstrip("/"), BY_SA, "https://creativecommons.org/licenses/by-sa",
                "https://commons.wikimedia.org/wiki/File:Example.jpg", NASA, "https://www.nasa.gov",
                "http://creativecommons.org/licenses/by/4.0/", "HTTPS://creativecommons.org/licenses/by/4.0/",
                "https://CreativeCommons.org/licenses/by/4.0/", "https://creativecommons.org:443/licenses/by/4.0/",
                "https://creativecommons.org:/licenses/by/4.0/", "https://user@creativecommons.org/licenses/by/4.0/",
                "https://creativecommons.org/licenses/by/4.0/ ", "https://creativecommons.org/licenses/by/\t4.0/",
                "https://creative\ncommons.org/licenses/by/4.0/", "https://creativecommons.org\\@evil.example/licenses/by/",
                "https://creativecommons.org.evil.example/licenses/by/4.0/", "https://creativecommons.org/licenses/by/4.0/?x=1#y",
                "https://creativecommons.org/licenses/by/" + "a" * 470 + "/", "https://creativecommons.org/licenses/by/" + "a" * 480 + "/",
                "", "creativecommons.org/licenses/by/4.0/"]

PAGES = {
    "wikimedia": [WM_PAGE, "https://Commons.Wikimedia.org/wiki/File:Example.jpg", "https://commons.wikimedia.org.evil.example/wiki/x",
                  "https://commons.wikimedia.org\\@evil.example/wiki/x", "https://evil.example\\@commons.wikimedia.org/wiki/x",
                  "https://commons.wikimedia.org:8443/wiki/x", "http://commons.wikimedia.org/wiki/x", "https://commons.wikimedia.org./wiki/x",
                  "https://commons.wikimedia.org/wiki/x y", "https://xn--commons-wikimedia.org/wiki/x", "javascript:alert(1)"],
    "openverse": ["https://openverse.org/image/0f6a", "https://www.openverse.org/image/0f6a"],
    "met": ["https://www.metmuseum.org/art/collection/search/1", "https://metmuseum.org/art/collection/search/1"],
    "aic": ["https://www.artic.edu/artworks/1", "https://artic.edu/artworks/1"],
    "nasa": ["https://images.nasa.gov/details/PIA00001", "https://images-assets.nasa.gov/image/x.jpg"],
    "archive": ["https://archive.org/details/x"],
    "safebooru": ["https://safebooru.org/index.php?page=post&s=view&id=1"],
}


def cases():
    out = []
    for source, pages in PAGES.items():
        for licence in LICENCES:
            for url in LICENCE_URLS:
                out.append([source, licence, url, pages[0]])
        for page in pages:
            out.append([source, "CC0-1.0", CC0, page])
    for source in (None, 1, "", "unknown", "Wikimedia", ["wikimedia"]):  # not class A sources
        out.append([source, "CC0-1.0", CC0, WM_PAGE])
    return out


@unittest.skipIf(NODE is None, "node is not on PATH; this check does not count toward acceptance")
class LicenceTablesAgree(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.cases = cases()
        run = subprocess.run([NODE, "--input-type=module", "-e", SCRIPT, PAGE_MODULE], input=json.dumps(cls.cases),
                             capture_output=True, text=True, encoding="utf-8", timeout=120)
        if run.returncode != 0:
            raise AssertionError(f"node failed: {run.stderr.strip()[-2000:]}")
        cls.page = json.loads(run.stdout)

    def test_tables_are_identical_with_the_same_key_order(self):
        self.assertEqual(json.dumps(self.page["table"]), json.dumps(licences.TABLE))

    def test_every_case_gets_the_same_answer(self):
        mismatches = []
        for case, page_answer in zip(self.cases, self.page["answers"]):
            source, licence, url, page = case
            local = [licences.licence_family(licence), licences.licence_ok(source, licence, url),
                     licences.page_url_ok(source, page)]
            if local != page_answer:
                mismatches.append({"case": case, "python": local, "page": page_answer})
        self.assertEqual(len(self.page["answers"]), len(self.cases))
        self.assertEqual(mismatches[:10], [], f"{len(mismatches)} of {len(self.cases)} cases differ")

    def test_expected_answers(self):
        expect = [
            (["wikimedia", "CC-BY-SA-4.0", BY_SA.rstrip("/"), WM_PAGE], ["CC-BY-SA", True, True]),
            (["wikimedia", "public-domain", WM_PAGE, WM_PAGE], ["public-domain", True, True]),
            (["openverse", "public-domain", WM_PAGE, "https://openverse.org/image/0f6a"], ["public-domain", False, True]),
            (["nasa", "US-Gov-PD", NASA, "https://images.nasa.gov/details/PIA00001"], ["US-Gov-PD", True, True]),
            (["met", "CC0-1.0", "https://creativecommons.org:443/licenses/by/4.0/", "https://metmuseum.org/x"], ["CC0-1.0", False, False]),
            (["aic", "CC-BY-4.0", "https://creativecommons.org\\@evil.example/licenses/by/", "https://www.artic.edu/artworks/1"],
             ["CC-BY", False, True]),
            (["wikimedia", "private-reference", WM_PAGE, "https://evil.example\\@commons.wikimedia.org/wiki/x"], [None, False, False]),
            (["archive", "CC0-1.0", CC0, "https://archive.org/details/x"], ["CC0-1.0", False, False]),
            (["safebooru", "CC-BY-4.0", BY, "https://safebooru.org/index.php?page=post&s=view&id=1"], ["CC-BY", False, False]),
        ]
        for (source, licence, url, page), answer in expect:
            with self.subTest(case=(source, licence, url, page)):
                self.assertEqual([licences.licence_family(licence), licences.licence_ok(source, licence, url),
                                  licences.page_url_ok(source, page)], answer)


if __name__ == "__main__":
    unittest.main(verbosity=2)

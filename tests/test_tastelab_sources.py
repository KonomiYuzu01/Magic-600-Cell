"""Offline transport, adapter and in-memory pipeline checks; invented fixtures only."""
from __future__ import annotations

import copy
import hashlib
import io
import json
import os
import sys
import unittest
from dataclasses import replace
from pathlib import Path
from unittest import mock
from urllib.parse import parse_qs, urlsplit

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "tools"))
import numpy as np
from PIL import Image
from tastelab import common, embed, fetch, licences, net, screen, seeds, sources, store
import test_tastelab_store as test_store
import test_tastelab_images as runner


def fixture(name):
    return json.loads((ROOT / "tests" / "fixtures" / "tastelab" / (name + ".json")).read_text(encoding="utf-8"))


RESULT_PATHS = {"wikimedia": ("query", "pages"), "met": ("objectIDs",), "aic": ("data",),
                "openverse": ("results",), "nasa": ("collection", "items"), "archive": ("response", "docs"),
                "demozoo": ("results",), "safebooru": ()}


def result_response(name, results):
    value = results
    for key in reversed(RESULT_PATHS[name]):
        value = {key: value}
    return value


def metadata_errors(name):
    empty = result_response(name, [])
    return [{"error": {"code": "synthetic-transient-error", "detail": "PRIVATE-CANARY" * 1000},
             **(empty if isinstance(empty, dict) else {})}, None, "wrong type", 42, {},
            result_response(name, None), result_response(name, {}), result_response(name, [None])]


class JSONClient:
    def __init__(self, responses):
        self.responses, self.calls = list(responses), []

    def get(self, url, **kwargs):
        self.calls.append((url, kwargs))
        if not self.responses:
            raise AssertionError("unexpected synthetic metadata request")
        return net.Response(url, 200, {"content-type": "application/json"}, json.dumps(self.responses.pop(0)).encode())


def candidate(name="met", ident="901", title="Invented object"):
    return sources.Candidate(name, ident, "https://images.metmuseum.org/synthetic/" + ident + ".png",
                             "https://www.metmuseum.org/art/collection/search/" + ident, "CC0-1.0", sources.CC0_URL,
                             "Invented Artist", title)


def png(color=(80, 130, 170), *, animated=False):
    image = Image.new("RGB", (96, 96), color)
    output = io.BytesIO()
    if animated:
        image.save(output, format="PNG", save_all=True, append_images=[Image.new("RGB", (96, 96), "red")], duration=100)
    else:
        image.save(output, format="PNG")
    return output.getvalue()


class NetTests(unittest.TestCase):
    def client(self, responses):
        self.now, self.waits = 0.0, []
        def wait(seconds):
            self.waits.append(seconds)
            self.now += seconds
        replay = net.ReplayTransport(responses)
        return net.Client(replay, sleep=wait, clock=lambda: self.now, wall_clock=lambda: 0), replay

    def get(self, client, url="https://api.example.invalid/start", **kwargs):
        params = dict(hosts=("api.example.invalid", "image.example.invalid"), min_interval=1, max_bytes=100)
        params.update(kwargs)
        return client.get(url, **params)

    def test_only_adapter_https_hosts_and_safe_headers(self):
        client, replay = self.client({})
        for url in ("http://api.example.invalid/", "https://api.example.invalid.evil/", "https://user@api.example.invalid/",
                    "https://api.example.invalid:443/", "https://api.example.invalid/a\n", "file:///tmp/image"):
            with self.subTest(url=url), self.assertRaises(net.NetError):
                self.get(client, url)
        self.assertEqual(replay.calls, [])
        for headers in ({"Authorization": "synthetic"}, {"Cookie": "synthetic"}, {"Referer": "synthetic"}):
            with self.assertRaises(net.NetError):
                self.get(client, extra_headers=headers)
        self.assertTrue(net.host_allowed("ia123.us.archive.org", ("*.us.archive.org",)))
        self.assertFalse(net.host_allowed("us.archive.org", ("*.us.archive.org",)))
        self.assertFalse(net.host_allowed("evil.us.archive.org.evil", ("*.us.archive.org",)))

    def test_redirects_and_source_interval_cover_cross_host_hops(self):
        a, b = "https://api.example.invalid/start", "https://image.example.invalid/image"
        client, replay = self.client({a: (302, {"Location": b}, b""), b: (200, {}, b"image")})
        self.assertEqual(self.get(client).body, b"image")
        self.assertEqual(self.waits, [1])
        self.assertEqual([url for url, _ in replay.calls], [a, b])
        self.assertEqual(replay.calls[0][1], {"User-Agent": common.USER_AGENT, "Accept": "*/*"})

    def test_unapproved_redirect_is_never_requested(self):
        client, replay = self.client({"https://api.example.invalid/start": (302, {"location": "https://outside.invalid/"}, b"")})
        with self.assertRaises(net.NetError):
            self.get(client)
        self.assertEqual(len(replay.calls), 1)

    def test_redirect_limit_size_caps_and_unrecorded_replay_fail(self):
        a = "https://api.example.invalid/start"
        for reply in ((302, {"location": a}, b""), (200, {}, b"x" * 101)):
            client, replay = self.client({a: reply})
            with self.assertRaises(net.NetError):
                self.get(client)
            self.assertLessEqual(len(replay.calls), 4)
        client, _ = self.client({})
        with self.assertRaisesRegex(net.NetError, "unrecorded"):
            self.get(client)

    def test_bounded_429_and_5xx_retry_after_seconds_and_dates(self):
        a = "https://api.example.invalid/start"
        client, replay = self.client({a: [(429, {"Retry-After": "2"}, b""),
                                         (500, {"retry-after": "Thu, 01 Jan 1970 00:00:05 GMT"}, b""), (200, {}, b"ok")]})
        self.assertEqual(self.get(client).body, b"ok")
        self.assertEqual(self.waits, [2, 5])
        self.assertEqual(len(replay.calls), 3)
        client, replay = self.client({a: (503, {"retry-after": "0"}, b"")})
        with self.assertRaisesRegex(net.NetError, "exhausted"):
            self.get(client)
        self.assertEqual(len(replay.calls), net.RETRIES + 1)
        client, replay = self.client({a: (429, {"retry-after": "121"}, b"")})
        with self.assertRaisesRegex(net.NetError, "bounded"):
            self.get(client)
        self.assertEqual(len(replay.calls), 1)

    def test_urllib_uses_empty_proxy_no_auth_cookies_and_get(self):
        response = mock.MagicMock()
        response.headers, response.code = {"Content-Length": "2"}, 200
        response.read.return_value = b"ok"
        response.__enter__.return_value = response
        built = net.build_opener
        captured = []
        def opener(*handlers):
            got = built(*handlers)
            captured.extend(got.handlers)
            self.assertEqual(handlers[0].proxies, {})
            got.open = mock.Mock(return_value=response)
            self.open = got.open
            return got
        with mock.patch.dict(os.environ, {"HTTPS_PROXY": "http://proxy.invalid", "HTTP_PROXY": "http://proxy.invalid",
                                          "NETRC": "synthetic-credential-file"}), mock.patch.object(net, "build_opener", side_effect=opener):
            result = net.urllib_transport("https://api.example.invalid/", {"User-Agent": common.USER_AGENT}, 10, 30)
        self.assertEqual(result.body, b"ok")
        names = {type(handler).__name__ for handler in captured}
        self.assertFalse(any("Auth" in name or "Cookie" in name for name in names))
        request = self.open.call_args.args[0]
        self.assertEqual(request.get_method(), "GET")
        self.assertEqual(request.headers, {"User-agent": common.USER_AGENT})
        response.read.assert_called_once_with(11)

    def test_transport_content_length_cap_before_body_read(self):
        response = mock.MagicMock()
        response.headers, response.code = {"Content-Length": "101"}, 200
        response.__enter__.return_value = response
        opener = mock.Mock()
        opener.open.return_value = response
        with mock.patch.object(net, "build_opener", return_value=opener), self.assertRaisesRegex(net.NetError, "size cap"):
            net.urllib_transport("https://api.example.invalid/", {}, 100, 30)
        response.read.assert_not_called()


class AdapterTests(unittest.TestCase):
    def search(self, name):
        data = fixture(name)
        if name == "met":
            replies = [data["search"], *data["objects"].values()]
        elif name == "archive":
            replies = [data["search"], data["metadata"]]
        elif name == "demozoo":
            replies = [data["search"], data["detail"]]
        else:
            replies = [data]
        client = JSONClient(replies)
        items, cursor = sources.ADAPTERS[name].search(client, "invented query", None)
        return client, items, cursor

    def test_eight_synthetic_source_fixtures_and_fixed_intervals(self):
        self.assertEqual(set(sources.ADAPTERS), set(seeds.SOURCE_TIER))
        intervals = dict(wikimedia=0.5, met=0.5, aic=1, nasa=1, openverse=3, archive=1, demozoo=10, safebooru=1)
        for name, adapter in sources.ADAPTERS.items():
            with self.subTest(name=name):
                client, items, _ = self.search(name)
                self.assertEqual(len(items), 1)
                self.assertTrue(sources.admit(items[0], adapter.tier))
                for _, kwargs in client.calls:
                    self.assertEqual(kwargs["min_interval"], intervals[name])
                    self.assertEqual(kwargs["hosts"], adapter.hosts)
                if adapter.tier == "B":
                    self.assertEqual(items[0].licence, "private-reference")
                    self.assertEqual(items[0].licence_url, adapter.terms_url)
                    self.assertFalse(sources.admit(items[0], "A"))

    def test_each_class_a_admitted_item_passes_exact_licence_table(self):
        for name in licences.TABLE["pageHosts"]:
            with self.subTest(name=name):
                _, items, _ = self.search(name)
                item = items[0]
                self.assertIsNotNone(licences.licence_family(item.licence))
                self.assertTrue(licences.licence_ok(name, item.licence, item.licence_url))
                self.assertTrue(licences.page_url_ok(name, item.page_url))

    def test_met_uses_v11_offset_limit_skips_seen_before_objects(self):
        data = fixture("met")
        client = JSONClient([data["search"], data["objects"]["902"]])
        items, cursor = sources.ADAPTERS["met"].search(client, "dial", "500", page_size=999, seen=lambda ident: ident == "901")
        self.assertEqual(items, [])
        self.assertEqual(len(client.calls), 2)
        parts = urlsplit(client.calls[0][0])
        self.assertEqual(parts.path, "/public/collection/v1.1/search")
        self.assertEqual(parse_qs(parts.query), {"q": ["dial"], "hasImages": ["true"], "offset": ["500"], "limit": ["500"]})
        self.assertTrue(client.calls[1][0].endswith("/objects/902"))

    def test_aic_score_stops_paging_and_credit_fallbacks(self):
        client, items, cursor = self.search("aic")
        self.assertIsNone(cursor)
        self.assertEqual(items[0].page_url, "https://www.artic.edu/artworks/601")
        self.assertIn("/full/843,/0/default.jpg", items[0].image_url)
        self.assertEqual(client.calls[0][1]["extra_headers"], {"AIC-User-Agent": common.USER_AGENT})
        data = fixture("aic")
        data["data"] = data["data"][:1]
        data["data"][0]["artist_title"] = ""
        values, next_cursor = sources.ADAPTERS["aic"].search(JSONClient([data]), "dial", None)
        self.assertEqual(values[0].attribution, "Invented maker, imaginary era")
        self.assertEqual(next_cursor, "2")

    def test_openverse_maps_licences_uses_proxy_and_filters_sensitive(self):
        client, items, cursor = self.search("openverse")
        self.assertEqual(cursor, "2")
        self.assertEqual(parse_qs(urlsplit(client.calls[0][0]).query)["mature"], ["false"])
        self.assertEqual(urlsplit(items[0].image_url).hostname, "api.openverse.org")
        for code, licence, url in (("cc0", "CC0-1.0", sources.CC0_URL), ("pdm", "CC-PDM-1.0", sources.PDM_URL),
                                   ("by", "CC-BY-4.0", "https://creativecommons.org/licenses/by/4.0/"),
                                   ("by-sa", "CC-BY-SA-4.0", "https://creativecommons.org/licenses/by-sa/4.0/")):
            data = fixture("openverse")
            data["results"][0].update(license=code, license_url=url, creator=None)
            got, _ = sources.ADAPTERS["openverse"].search(JSONClient([data]), "x", None)
            self.assertEqual(got[0].licence, licence)
            self.assertEqual(got[0].attribution, "Synthetic attribution")

    def test_wikimedia_licence_ports_public_domain_and_html_credit(self):
        for raw, licence, url in (("cc0", "CC0-1.0", sources.CC0_URL), ("public-domain-mark", "CC-PDM-1.0", sources.PDM_URL),
                                   ("public-domain", "public-domain", None),
                                   ("cc-by-3.0-de", "CC-BY-3.0-DE", "https://creativecommons.org/licenses/by/3.0/de/")):
            data = fixture("wikimedia")
            info = data["query"]["pages"][0]["imageinfo"][0]
            metadata = info["extmetadata"]
            metadata["License"]["value"] = raw
            metadata["LicenseUrl"]["value"] = url
            got, _ = sources.ADAPTERS["wikimedia"].search(JSONClient([data]), "x", None)
            self.assertEqual(got[0].licence, licence)
            self.assertEqual(got[0].attribution, "Invented Maker")
            self.assertEqual(got[0].licence_url, url or info["descriptionurl"])

    def test_nasa_search_only_rejects_other_rights_holders(self):
        client, items, _ = self.search("nasa")
        self.assertEqual(len(client.calls), 1)
        self.assertEqual(items[0].page_url, "https://images.nasa.gov/details/synthetic-nasa-1")
        self.assertEqual(items[0].licence, "US-Gov-PD")
        for credit in ("Courtesy Invented Company", "NASA / ESA", "Copyright Invented Maker", "(c) Invented Maker"):
            data = fixture("nasa")
            data["collection"]["items"][0]["data"][0]["photographer"] = credit
            got, _ = sources.ADAPTERS["nasa"].search(JSONClient([data]), "x", None)
            self.assertEqual(got, [])

    def test_nasa_checks_complete_rights_fields_at_start_middle_and_end(self):
        notice = " Copyright Synthetic Rights Holder "
        filler = "A" * (10_000 - len(notice))
        for field in ("photographer", "secondary_creator", "description"):
            for position in (0, len(filler) // 2, len(filler)):
                with self.subTest(field=field, position=position):
                    data = fixture("nasa")
                    data["collection"]["items"][0]["data"][0][field] = filler[:position] + notice + filler[position:]
                    got, _ = sources.ADAPTERS["nasa"].search(JSONClient([data]), "x", None)
                    self.assertEqual(got, [])
        data = fixture("nasa")
        data["collection"]["items"][0]["data"][0]["photographer"] = "A" * 10_000
        got, _ = sources.ADAPTERS["nasa"].search(JSONClient([data]), "x", None)
        self.assertEqual(len(got), 1)
        self.assertTrue(got[0].attribution.endswith("A" * 300))

    def test_wikimedia_checks_complete_licence_fields(self):
        notice = " Copyright Synthetic Rights Holder "
        filler = "A" * (10_000 - len(notice))
        for field in ("License", "LicenseShortName"):
            for position in (0, len(filler) // 2, len(filler)):
                with self.subTest(field=field, position=position):
                    data = fixture("wikimedia")
                    metadata = data["query"]["pages"][0]["imageinfo"][0]["extmetadata"]
                    metadata["License"]["value"] = ""
                    metadata[field]["value"] = filler[:position] + notice + filler[position:]
                    got, _ = sources.ADAPTERS["wikimedia"].search(JSONClient([data]), "x", None)
                    self.assertEqual(got, [])

    def test_every_adapter_rejects_invalid_result_containers_with_bounded_errors(self):
        for name, adapter in sources.ADAPTERS.items():
            for index, payload in enumerate(metadata_errors(name)):
                with self.subTest(source=name, payload=index):
                    with self.assertRaisesRegex(net.NetError, "metadata") as error:
                        adapter.search(JSONClient([payload]), "instrument", None)
                    self.assertLess(len(str(error.exception)), 100)
                    self.assertNotIn("PRIVATE-CANARY", str(error.exception))

    def test_item_metadata_errors_are_bounded(self):
        for name in ("met", "archive", "demozoo"):
            data = fixture(name)
            path = RESULT_PATHS[name]
            listing = data["search"]
            rows = listing
            for key in path[:-1]:
                rows = rows[key]
            rows[path[-1]] = rows[path[-1]][:1]
            invalid = [{"error": {"code": "synthetic-transient-error"}}, None, [], "wrong type"]
            if name != "met":
                field = "files" if name == "archive" else "screenshots"
                invalid += ([] if name == "archive" else [{}]) + [{field: None}, {field: {}}, {field: [None]}]
            for index, payload in enumerate(invalid):
                with self.subTest(source=name, payload=index), self.assertRaisesRegex(net.NetError, "metadata"):
                    sources.ADAPTERS[name].search(JSONClient([listing, payload]), "instrument", None)

    def test_documented_empty_answers_complete_and_near_misses_are_refused(self):
        empty = {"wikimedia": {"batchcomplete": True}, "met": {"total": 0, "objectIDs": None}}
        for name, payload in empty.items():
            with self.subTest(source=name):
                self.assertEqual(sources.ADAPTERS[name].search(JSONClient([payload]), "instrument", None), ([], None))
        near_misses = {"wikimedia": [{"batchcomplete": True, "error": {"code": "synthetic"}}, {"batchcomplete": False},
                                     {"batchcomplete": "true"}, {"batchcomplete": True, "query": None}],
                       "met": [{"total": 1, "objectIDs": None}, {"total": False, "objectIDs": None},
                               {"total": "0", "objectIDs": None}, {"total": 0, "objectIDs": None, "error": "synthetic"}]}
        for name, payloads in near_misses.items():
            for index, payload in enumerate(payloads):
                with self.subTest(source=name, payload=index), self.assertRaisesRegex(net.NetError, "metadata"):
                    sources.ADAPTERS[name].search(JSONClient([payload]), "instrument", None)

    def test_archive_skips_an_unknown_item_and_keeps_the_rest_of_the_page(self):
        data = fixture("archive")
        listing = data["search"]
        docs = listing["response"]["docs"]
        self.assertGreaterEqual(len(docs), 1)
        listing["response"]["docs"] = [{"identifier": "synthetic-dark-item", "title": "Dark"}] + docs[:1]
        client = JSONClient([listing, {}, data["metadata"]])
        got, _ = sources.ADAPTERS["archive"].search(client, "x", None)
        self.assertEqual([item.source_id for item in got], [docs[0]["identifier"]])
        self.assertEqual(len(client.calls), 3)

    def test_archive_and_demozoo_skip_seen_and_lost_without_item_requests(self):
        for name in ("archive", "demozoo"):
            data = fixture(name)
            client = JSONClient([data["search"]])
            got, _ = sources.ADAPTERS[name].search(client, "x", None, seen=lambda ident: True)
            self.assertEqual(got, [])
            self.assertEqual(len(client.calls), 1)
        adapter = sources.ADAPTERS["demozoo"]
        self.assertEqual(adapter.cursor_key("first text"), adapter.cursor_key("second text"))
        self.assertEqual(adapter.cursor_key("platform=12&supertype=production"), "platform=12&supertype=production")

    def test_safebooru_forces_general_and_every_excluded_tag(self):
        client, items, _ = self.search("safebooru")
        tags = parse_qs(urlsplit(client.calls[0][0]).query)["tags"][0].split()
        self.assertIn("rating:general", tags)
        self.assertTrue(all("-" + tag in tags for tag in sources.SAFEBOORU_EXCLUDED_TAGS))
        self.assertTrue(items[0].anime)
        for tag in sources.SAFEBOORU_EXCLUDED_TAGS:
            data = fixture("safebooru")[:1]
            data[0]["tags"] = "scenery " + tag
            got, _ = sources.ADAPTERS["safebooru"].search(JSONClient([data]), "x rating:explicit", None)
            self.assertEqual(got, [])


class LicenceTests(unittest.TestCase):
    def test_exact_patterns_and_sources(self):
        self.assertIsNone(licences.licence_family("CC0-1.0\n"))
        self.assertIsNone(licences.licence_family("CC-BY-NC-4.0"))
        self.assertIsNone(licences.licence_family("private-reference"))
        self.assertEqual(licences.licence_family("CC-BY-SA-3.0-US"), "CC-BY-SA")
        self.assertFalse(licences.licence_ok("met", "public-domain", "https://commons.wikimedia.org/wiki/File:x"))
        self.assertFalse(licences.licence_ok("wikimedia", "US-Gov-PD", "https://www.nasa.gov/image"))
        self.assertTrue(licences.licence_ok("met", "CC0-1.0", sources.CC0_URL[:-1]))

    def test_url_constraints_and_prefixes(self):
        for url in ("http://creativecommons.org/publicdomain/zero/1.0/", "https://user@creativecommons.org/publicdomain/zero/1.0/",
                    "https://creativecommons.org:443/publicdomain/zero/1.0/", sources.CC0_URL + "\n", sources.CC0_URL + " ",
                    sources.CC0_URL + "\x7f", sources.CC0_URL + "x" * 501, "https://creativecommons.org.evil/publicdomain/zero/1.0/",
                    "https://creativecommons.org/Publicdomain/zero/1.0/", "https://creativecommons.org/publicdomain/zero/1.0evil"):
            with self.subTest(url=url):
                self.assertFalse(licences.licence_ok("met", "CC0-1.0", url))
        for url in ("https://www.metmuseum.org.evil/item", "https://user@www.metmuseum.org/item", "https://www.metmuseum.org:443/item",
                    "https://www.metmuseum.org/item\n", "https://www.metmuseum.org/item\x01"):
            self.assertFalse(licences.page_url_ok("met", url))


class CalibrationTests(test_store.TempDir):
    def setUp(self):
        super().setUp()
        self.fake = embed.FakeEmbedder()
        self.prototypes = {name: AdapterTests().search(name)[1][0] for name in ("met", "aic", "nasa", "wikimedia")}
        self.query_groups = {query: name for name, spec in screen.CALIBRATION_GROUPS.items() for _, query in spec["queries"]}
        self.labels = {"nudity": "adult nude study", "suggestive": "adult burlesque dancer study",
                       "underwear_swimwear": "adult swimsuit study", "gore": "adult anatomical dissection study"}
        self.number = 0

    def searches(self, name):
        def search(client, query, cursor, **kwargs):
            self.number += 1
            title = self.labels.get(self.query_groups.get(query), "Invented instrument")
            return [replace(self.prototypes[name], source_id=f"synthetic-{self.number}-{i}", title=title)
                    for i in range(screen.GROUP_SIZE)], None
        return search

    def calibrate(self, score, cases=None):
        patches = [mock.patch.object(sources.ADAPTERS[name], "search", side_effect=self.searches(name)) for name in self.prototypes]
        for patch in patches:
            patch.start()
            self.addCleanup(patch.stop)
        client = mock.Mock()
        client.get.return_value = net.Response("synthetic", 200, {}, png())
        scorer = mock.Mock()
        scorer.unsafe_score.return_value = [score]
        with mock.patch.object(screen, "Screen", return_value=scorer):
            result = fetch.calibrate(self.fake, client, log=lambda _: None, cases=cases)
        return result

    def test_calibration_all_nan_counts_unscorable_fails_and_writes_counts_only(self):
        report = self.calibrate(np.nan)
        self.assertFalse(report["passed"])
        self.assertEqual((report["threshold"], report["strict_threshold"]), screen.LOWERED)
        for group in report["groups"].values():
            self.assertEqual((group["n"], group["scorable"], group["unscorable"], group["caught_strict"]),
                             (screen.GROUP_SIZE, 0, screen.GROUP_SIZE, 0))
        self.assertEqual(report["negatives"]["scorable"], 0)
        self.assertEqual(list(self.tmp.iterdir()), [])
        path = fetch.write_calibration(common.default_data_root(), report)
        text = path.read_text(encoding="utf-8")
        self.assertNotIn("Invented", text)
        self.assertNotIn("synthetic-", text)
        self.assertNotIn("image_url", text)
        self.assertNotIn("https://", text)
        self.assertNotIn("NaN", text)
        self.assertEqual([p for p in self.tmp.rglob("*") if p.is_file()], [path])
        with self.assertRaisesRegex(common.Refused, "passing --calibrate"):
            fetch.load_screen(self.fake, common.default_data_root())

    def test_passing_report_bound_to_model_weights_probes_and_approved_thresholds(self):
        report = self.calibrate(1.0)
        self.assertTrue(report["passed"])
        self.assertEqual((report["threshold"], report["strict_threshold"]), (screen.THRESHOLD, screen.STRICT_THRESHOLD))
        path = fetch.write_calibration(common.default_data_root(), report)
        got = fetch.load_screen(self.fake, common.default_data_root())
        self.assertEqual(got.threshold, screen.THRESHOLD)
        for field, value in (("model", "other"), ("weightsSha256", "f" * 64), ("probesSha256", "f" * 64),
                             ("passed", False), ("threshold", 0.5)):
            broken = copy.deepcopy(report)
            broken[field] = value
            path.write_text(json.dumps(broken), encoding="utf-8")
            with self.subTest(field=field), self.assertRaises(common.Refused):
                fetch.load_screen(self.fake, common.default_data_root())

    def test_calibration_labels_require_adults_and_public_domain(self):
        self.assertFalse(fetch._adult_subject(candidate(title="nude child portrait"), positive=True))
        self.assertFalse(fetch._adult_subject(candidate(title="nude study"), positive=True))
        self.assertTrue(fetch._adult_subject(candidate(title="adult nude study"), positive=True))
        self.assertTrue(fetch._adult_subject(candidate(title="Invented teapot"), positive=False))
        item = replace(candidate(title="adult nude study"), licence="CC-BY-4.0", licence_url="https://creativecommons.org/licenses/by/4.0/")
        self.assertTrue(sources.admit(item, "A"))
        with mock.patch.object(sources.ADAPTERS["met"], "search", return_value=([item], None)), \
                mock.patch.object(sources.ADAPTERS["aic"], "search", return_value=([], None)), \
                mock.patch.object(sources.ADAPTERS["wikimedia"], "search", return_value=([], None)), \
                mock.patch.object(sources.ADAPTERS["nasa"], "search", return_value=([], None)):
            client = mock.Mock()
            report = fetch.calibrate(self.fake, client, log=lambda _: None)
        self.assertFalse(report["passed"])
        client.get.assert_not_called()
        self.assertTrue(all(group["n"] == 0 for group in report["groups"].values()))

    def test_calibration_cli_failed_gate_returns_one_without_database_or_images(self):
        report = self.calibrate(np.nan)
        with mock.patch.object(embed, "load_embedder", return_value=self.fake), \
                mock.patch.object(net, "Client"), mock.patch.object(fetch, "calibrate", return_value=report), \
                mock.patch.object(store, "Store", side_effect=AssertionError("calibration must not open a store")), \
                mock.patch("sys.stdout", io.StringIO()):
            self.assertEqual(fetch.main(["--calibrate"]), 1)
        files = [p for p in self.tmp.rglob("*") if p.is_file()]
        self.assertEqual([p.name for p in files], ["calibration.json"])

    def test_diagnose_keeps_metadata_of_missed_and_discarded_cases_only(self):
        # A positive is missed below the approved threshold; a negative is discarded at or over the lowest strict one.
        # A score the gate cannot use (non-finite or outside [0, 1]) is recorded as None: a missed positive, never a
        # discarded negative.
        for score, keeps in ((0.10, "both"), (screen.THRESHOLD, "negatives"), (screen.LOWERED[1], "both"),
                             (screen.LOWERED[1] - 1e-9, "positives"), (np.nan, "positives"), (np.inf, "positives"),
                             (1.01, "positives"), (-0.01, "positives")):
            with self.subTest(score=score):
                cases = []
                report = self.calibrate(score, cases=cases)
                positives = sum(group["n"] for group in report["groups"].values())
                negatives = report["negatives"]["n"]
                self.assertEqual((positives, len(cases)), (screen.GROUP_SIZE * len(screen.CALIBRATION_GROUPS), positives + negatives))
                self.assertGreater(negatives, 0)
                kept = fetch.diagnostic_cases(cases)
                expected = {"both": positives + negatives, "negatives": negatives, "positives": positives}[keeps]
                self.assertEqual(len(kept), expected)
                self.assertEqual(sum(case["group"] == "negative" for case in kept), 0 if keeps == "positives" else negatives)
                for case in kept:
                    self.assertEqual(set(case), {"group", "query", "source", "id", "title", "keywords", "score"})
                    self.assertEqual(case["score"] is None, not 0 <= score <= 1)
                text = json.dumps(kept, allow_nan=False)
                self.assertNotIn("https://", text)
                self.assertNotIn("image_url", text)
                self.assertNotIn("synthetic-", json.dumps(report))

    def test_diagnose_replaces_link_like_text_in_titles_and_keywords(self):
        links = ("https://images.metmuseum.org/synthetic/901.png", "www.example.org/item", "metmuseum.org/art/collection/9",
                 "data:image/png;base64,AAAA", "data:image/svg+xml,%3Csvg%2F%3E", "C:\\Users\\someone\\picture.png",
                 "ftp://example.org/x", "mailto:someone@example.org", "javascript:alert(1)", "metmuseum.org",
                 "192.168.0.1", "/home/someone/picture.png", "\\\\server\\share\\picture.png", "//cdn.example.org/x.png",
                 "//cdn.example.co.uk", "//cdn.example.co.uk?case=901", "example．org", "someone＠example", "[::1]:8080",
                 "localhost:8080")
        self.labels["nudity"] = "adult nude study " + " ".join(links)
        self.prototypes = {name: replace(item, keywords=(*links, "oil on canvas")) for name, item in self.prototypes.items()}
        cases = []
        self.calibrate(0.10, cases=cases)
        kept = fetch.diagnostic_cases(cases)
        nudity = [case for case in kept if case["group"] == "nudity"]
        self.assertEqual(len(nudity), screen.GROUP_SIZE)
        self.assertEqual(nudity[0]["title"], "adult nude study " + " ".join(["[link]"] * len(links)))
        self.assertEqual(nudity[0]["keywords"], (["[link]"] * len(links) + ["oil on canvas"])[:20])
        text = json.dumps(kept)
        for fragment in (":/", "www.", "metmuseum", "example", "base64", "svg", "Users", "picture", "someone", "alert",
                         "192.168", "server", "cdn", "localhost", "::1"):
            self.assertNotIn(fragment, text)
        for ordinary in ("Kneeling Nude Woman with Drapery, 1890; plate 3 of 5", "St. Jerome, oil on canvas", "Venus (after Titian)"):
            self.assertEqual(fetch._plain(ordinary), ordinary)
        # Decomposed accents, a scheme followed by a space and over-redacted ordinary words.
        self.assertEqual(fetch._plain("café.com data: ,DIAG javascript: alert(1) Study: nude"),
                         "[link] [link] ,DIAG [link] alert(1) [link] nude")
        self.assertEqual(fetch._plain("x" * 1000 + " https://example.org/a"), "x" * 200)
        self.assertEqual(fetch._plain("nude " + "https://example.org/" + "a" * 2000), "nude [link]")

    def test_calibration_skips_document_files_and_labelled_negatives(self):
        self.assertIn("a vintage photo of women in bathing costumes", screen.UNSAFE_PROBES)
        base = replace(self.prototypes["wikimedia"], licence="CC0-1.0", licence_url=sources.CC0_URL, keywords=())

        def item(ident, title, suffix=".jpg", keywords=()):
            return replace(base, source_id=ident, title=title, keywords=keywords,
                           page_url=f"https://commons.wikimedia.org/wiki/File:Invented_{ident}{suffix}")

        pages = {"nude painting": [item("1", "adult nude study", ".pdf"), item("2", "adult nude study", ".DjVu"),
                                   item("3", "adult nude study", keywords=("Books with Wikidata item",))],
                 "botanical illustration": [item("4", "Bloodroot botanical illustration"),
                                            item("5", "Invented allegory", keywords=("Female Nudes",)),
                                            item("6", "Invented herbal", ".pdf")]}
        lines, cases = [], []
        client = mock.Mock()
        client.get.return_value = net.Response("synthetic", 200, {}, png())
        scorer = mock.Mock()
        scorer.unsafe_score.return_value = [0.5]
        empty = mock.Mock(return_value=([], None))
        with mock.patch.object(sources.ADAPTERS["wikimedia"], "search",
                               side_effect=lambda client, query, cursor, **kwargs: (pages.get(query, []), None)), \
                mock.patch.object(sources.ADAPTERS["met"], "search", empty), \
                mock.patch.object(sources.ADAPTERS["aic"], "search", empty), \
                mock.patch.object(sources.ADAPTERS["nasa"], "search", empty), \
                mock.patch.object(screen, "Screen", return_value=scorer):
            report = fetch.calibrate(self.fake, client, log=lines.append, cases=cases)
        self.assertEqual([(case["group"], case["id"]) for case in cases], [("nudity", "3"), ("negative", "4")])
        self.assertEqual((report["groups"]["nudity"]["n"], report["negatives"]["n"]), (1, 1))
        self.assertIn('Calibration skipped: {"document": 3, "labelled_negative": 1}', lines)

    def test_diagnose_cli_writes_case_metadata_beside_the_report(self):
        with self.assertRaises(SystemExit), mock.patch("sys.stderr", io.StringIO()):
            fetch.main(["--diagnose"])
        collected = []
        report = self.calibrate(0.10, cases=collected)

        def run(model, client, *, cases=None, **kwargs):
            cases.extend(collected)
            return report

        with mock.patch.object(embed, "load_embedder", return_value=self.fake), \
                mock.patch.object(net, "Client"), mock.patch.object(fetch, "calibrate", side_effect=run), \
                mock.patch.object(store, "Store", side_effect=AssertionError("calibration must not open a store")), \
                mock.patch("sys.stdout", io.StringIO()):
            self.assertEqual(fetch.main(["--calibrate", "--diagnose"]), 0 if report["passed"] else 1)
        files = sorted(p.name for p in self.tmp.rglob("*") if p.is_file())
        self.assertEqual(files, ["calibration-cases.json", "calibration.json"])
        reports = common.default_data_root() / "reports"
        written = json.loads((reports / fetch.CALIBRATION_CASES).read_text(encoding="utf-8"))
        self.assertEqual(written["cases"], fetch.diagnostic_cases(collected))
        self.assertNotIn("https://", (reports / fetch.CALIBRATION_CASES).read_text(encoding="utf-8"))
        self.assertNotIn("synthetic-", (reports / fetch.CALIBRATION_REPORT).read_text(encoding="utf-8"))


class PipelineTests(test_store.TempDir):
    def setUp(self):
        super().setUp()
        self.library = store.Store(self.tmp / "library")
        self.addCleanup(self.library.close)
        self.fake = embed.FakeEmbedder()
        self.content_screen = mock.Mock()
        self.content_screen.check.return_value = [False]

    def pipeline(self, item=None, data=None, **kwargs):
        item, data = item or candidate(), data or png()
        replay = net.ReplayTransport({item.image_url: (200, {"content-type": "image/png"}, data)})
        client = net.Client(replay, sleep=lambda _: None, clock=lambda: 0)
        pipeline = fetch.Pipeline(self.library, self.fake, self.content_screen, client, **kwargs)
        return pipeline, item, replay

    def ingest(self, pipeline, item, tier="A"):
        return pipeline.ingest_candidate(item, category="synthetic", query="instrument", tier=tier, adapter=sources.ADAPTERS[item.source])

    def test_pipeline_commits_original_hash_thumbnail_and_embedding(self):
        data = png()
        pipeline, item, _ = self.pipeline(data=data)
        outcome = self.ingest(pipeline, item)
        sha = hashlib.sha256(data).hexdigest()
        self.assertEqual((outcome.kind, outcome.sha256), ("stored", sha))
        self.assertEqual(self.library.original_path(sha).read_bytes(), data)
        self.assertNotEqual(hashlib.sha256(self.library.thumb_path(sha).read_bytes()).hexdigest(), sha)
        self.assertIn(sha, self.library.embeddings(self.fake.model_id))
        self.assertTrue(self.library.has_seen("met", "901"))

    def test_discard_keeps_counts_only_no_seen_hash_metadata_or_files(self):
        self.content_screen.check.return_value = [True]
        pipeline, item, _ = self.pipeline()
        before = set(self.library.root.rglob("*"))
        outcome = self.ingest(pipeline, item)
        self.assertEqual((outcome.kind, outcome.sha256), ("discarded", None))
        self.assertEqual(self.library.screen_counts(), {"met": (1, 1)})
        for table in ("images", "seen", "blocked", "embeddings", "file_ops"):
            self.assertEqual(self.library.db.execute(f"SELECT COUNT(*) FROM {table}").fetchone()[0], 0)
        self.assertEqual(set(self.library.root.rglob("*")), before)

    def test_embedder_failure_and_invalid_vectors_discard(self):
        for value in (np.full((1, 16), np.nan), np.zeros((1, 15)), np.zeros((1, 16))):
            pipeline, item, _ = self.pipeline()
            with mock.patch.object(self.fake, "embed_images", return_value=value):
                self.assertEqual(self.ingest(pipeline, item).kind, "discarded")
        with mock.patch.object(self.fake, "embed_images", side_effect=RuntimeError("injected")):
            self.assertEqual(self.ingest(pipeline, item).kind, "discarded")
        self.assertEqual(self.library.count_images(), 0)

    def test_rejected_seen_and_blocked_items_do_not_decode_or_embed(self):
        pipeline, item, replay = self.pipeline()
        with mock.patch.object(self.fake, "embed_images") as embeddings:
            self.assertEqual(self.ingest(pipeline, replace(item, attribution="")).kind, "rejected")
            self.assertEqual(replay.calls, [])
            self.library.mark_seen("met", item.source_id)
            self.assertEqual(self.ingest(pipeline, item).kind, "duplicate")
            self.assertEqual(replay.calls, [])
            item = replace(item, source_id="902")
            self.library.remove_image(hashlib.sha256(png()).hexdigest())
            self.assertEqual(self.ingest(pipeline, item).kind, "blocked")
            embeddings.assert_not_called()

    def test_animations_no_original_and_tier_b_is_always_strict(self):
        pipeline, item, _ = self.pipeline(data=png(animated=True))
        outcome = self.ingest(pipeline, item, tier="B")
        self.assertEqual(outcome.kind, "stored")
        self.assertIsNone(self.library.original_path(outcome.sha256))
        self.content_screen.check.assert_called_once()
        self.assertTrue(self.content_screen.check.call_args.kwargs["strict"])
        self.assertEqual(self.library.image(outcome.sha256).tier, "B")

    def test_relevance_and_near_duplicate_checks_before_commit(self):
        pipeline, item, _ = self.pipeline(relevance_min=1)
        self.assertEqual(self.ingest(pipeline, item).kind, "irrelevant")
        self.assertEqual(self.library.count_images(), 0)
        pipeline, item, _ = self.pipeline()
        first = self.ingest(pipeline, item)
        pipeline, item, _ = self.pipeline(item=replace(item, source_id="903"), data=png((81, 130, 170)))
        self.assertEqual(self.ingest(pipeline, item).kind, "duplicate")
        self.assertEqual(self.library.count_images(), 1)
        self.assertTrue(self.library.has_image(first.sha256))

    def test_metadata_failures_leave_cursors_retryable_then_valid_empty_search_completes(self):
        number = 0
        for name, adapter in sources.ADAPTERS.items():
            plan = seeds.parse({"version": 1, "sources": {name: True}, "categories": {
                "synthetic": {"kind": "focus", "target": 1, "sources": [name], "terms": ["instrument"]}}})
            key = adapter.cursor_key("instrument")
            resumed = "https://demozoo.org/api/v1/productions/?page=2" if name == "demozoo" else "2"
            for initial in (None, resumed):
                for index, payload in enumerate(metadata_errors(name)):
                    number += 1
                    with self.subTest(source=name, initial=initial, payload=index), store.Store(self.tmp / str(number)) as library:
                        if initial is not None:
                            library.set_cursor(name, key, initial, False)
                        before = [tuple(row) for row in library.db.execute("SELECT * FROM fetch_state")]
                        client = JSONClient([payload, result_response(name, [])])
                        counts = fetch.run(library, self.fake, self.content_screen, client, plan, sources=[name], log=lambda _: None)
                        self.assertEqual(counts, {"error": 1})
                        self.assertEqual([tuple(row) for row in library.db.execute("SELECT * FROM fetch_state")], before)
                        self.assertEqual(library.cursor(name, key), (initial, False))
                        self.assertEqual(len(client.calls), 1)
                        counts = fetch.run(library, self.fake, self.content_screen, client, plan, sources=[name], log=lambda _: None)
                        self.assertEqual(counts, {})
                        self.assertEqual(library.cursor(name, key), (None, True))
                        self.assertEqual(len(client.calls), 2)
                        self.assertEqual(library.count_images(), 0)

    def test_item_metadata_failure_retries_and_recovers_an_admitted_candidate(self):
        for name in ("met", "archive", "demozoo"):
            with self.subTest(source=name), store.Store(self.tmp / name) as library:
                data = fixture(name)
                listing = data["search"]
                rows = listing
                path = RESULT_PATHS[name]
                for key in path[:-1]:
                    rows = rows[key]
                rows[path[-1]] = rows[path[-1]][:1]
                detail = data["objects"]["901"] if name == "met" else data["metadata" if name == "archive" else "detail"]
                client = JSONClient([listing, {"error": {"code": "synthetic-transient-error"}}, listing, detail])
                plan = seeds.parse({"version": 1, "sources": {name: True}, "categories": {
                    "synthetic": {"kind": "focus", "target": 1, "sources": [name], "terms": ["instrument"]}}})
                with mock.patch.object(fetch.Pipeline, "ingest_candidate", return_value=fetch.Outcome("stored")) as ingest:
                    first = fetch.run(library, self.fake, self.content_screen, client, plan, sources=[name], limit=1, log=lambda _: None)
                    self.assertEqual(first, {"error": 1})
                    self.assertEqual(library.db.execute("SELECT COUNT(*) FROM fetch_state").fetchone()[0], 0)
                    ingest.assert_not_called()
                    second = fetch.run(library, self.fake, self.content_screen, client, plan, sources=[name], limit=1, log=lambda _: None)
                    self.assertEqual(second, {"stored": 1})
                    ingest.assert_called_once()
                    self.assertTrue(sources.admit(ingest.call_args.args[0], sources.ADAPTERS[name].tier))

    def test_recovery_missing_committed_bytes_can_refetch(self):
        pipeline, item, replay = self.pipeline()
        with mock.patch.object(self.library, "_reconcile"):
            first = self.ingest(pipeline, item)
        self.assertEqual(first.kind, "stored")
        for path in self.library.root.rglob(".part-*"):
            path.unlink()
        self.library._reconcile()
        self.assertFalse(self.library.has_image(first.sha256))
        self.assertFalse(self.library.has_seen(item.source, item.source_id))
        self.assertEqual(self.library.db.execute("SELECT COUNT(*) FROM file_ops").fetchone()[0], 0)
        second = self.ingest(pipeline, item)
        self.assertEqual((second.kind, second.sha256), ("stored", first.sha256))
        self.assertEqual(len(replay.calls), 2)
        self.assertTrue(self.library.has_seen(item.source, item.source_id))
        self.assertTrue(self.library.thumb_path(second.sha256).is_file())

    def test_recovery_keeps_removed_and_blocked_candidates_suppressed(self):
        for disposition in ("removed", "blocked"):
            with self.subTest(disposition=disposition), store.Store(self.tmp / disposition) as library:
                pipeline, item, replay = self.pipeline()
                pipeline.store = library
                with mock.patch.object(library, "_reconcile"):
                    first = self.ingest(pipeline, item)
                self.assertEqual(first.kind, "stored")
                if disposition == "removed":
                    library.remove_image(first.sha256)
                else:
                    with library.db:
                        library.db.execute("INSERT INTO blocked VALUES (?, ?)", (first.sha256, common.now_iso()))
                    for path in library.root.rglob(".part-*"):
                        path.unlink()
                    library._reconcile()
                self.assertFalse(library.has_image(first.sha256))
                self.assertTrue(library.is_blocked(first.sha256))
                self.assertTrue(library.has_seen(item.source, item.source_id))
                replay.calls.clear()
                self.assertEqual(self.ingest(pipeline, item).kind, "duplicate")
                self.assertEqual(replay.calls, [])
                self.assertEqual(library.db.execute("SELECT COUNT(*) FROM file_ops").fetchone()[0], 0)

    def test_run_resumes_midpage_without_losing_unhandled_items(self):
        plan = seeds.parse({"version": 1, "sources": {"met": True}, "categories": {
            "synthetic": {"kind": "focus", "target": 2, "sources": ["met"], "terms": ["instrument"]}}})
        a, b = candidate(ident="901"), candidate(ident="902")
        replay = net.ReplayTransport({a.image_url: (200, {}, png()), b.image_url: (200, {}, png((240, 10, 10)))})
        client = net.Client(replay, sleep=lambda _: None, clock=lambda: 0)
        with mock.patch.object(sources.ADAPTERS["met"], "search", return_value=([a, b], "20")):
            first = fetch.run(self.library, self.fake, self.content_screen, client, plan, limit=1, log=lambda _: None)
            self.assertEqual(first["stored"], 1)
            self.assertEqual(self.library.cursor("met", "instrument"), (None, False))
            second = fetch.run(self.library, self.fake, self.content_screen, client, plan, limit=1, log=lambda _: None)
        self.assertEqual(second["stored"], 1)
        self.assertEqual(self.library.count_images(), 2)
        self.assertEqual(self.library.cursor("met", "instrument"), ("20", False))

    def test_runner_missing_interpreter_requires_actual_execution(self):
        output = io.StringIO()
        from contextlib import redirect_stdout
        with redirect_stdout(output):
            self.assertEqual(runner.main(["--python", str(self.tmp / "missing.exe")]), 0)
        self.assertIn("does not count toward acceptance", output.getvalue())
        with redirect_stdout(io.StringIO()):
            self.assertEqual(runner.main(["--python", str(self.tmp / "missing.exe"), "--require"]), 1)


if __name__ == "__main__":
    unittest.main()

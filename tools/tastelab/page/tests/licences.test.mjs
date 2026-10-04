import test from "node:test";
import assert from "node:assert/strict";
import { TABLE, licenceFamily, licenceOk, pageUrlOk } from "../licences.js";

test("licence table has the exact version, values and key order of the packet", () => {
  const expected = {
    version: 1,
    licences: [
      { family: "CC0-1.0", pattern: "^CC0-1\\.0$", prefixes: ["https://creativecommons.org/publicdomain/zero/1.0/"] },
      { family: "CC-PDM-1.0", pattern: "^CC-PDM-1\\.0$", prefixes: ["https://creativecommons.org/publicdomain/mark/1.0/"] },
      { family: "CC-BY", pattern: "^CC-BY-[0-9]+\\.[0-9]+(-[A-Z]{2,5})?$", prefixes: ["https://creativecommons.org/licenses/by/"] },
      { family: "CC-BY-SA", pattern: "^CC-BY-SA-[0-9]+\\.[0-9]+(-[A-Z]{2,5})?$", prefixes: ["https://creativecommons.org/licenses/by-sa/"] },
      { family: "public-domain", pattern: "^public-domain$", sources: ["wikimedia"], prefixes: ["https://commons.wikimedia.org/wiki/"] },
      { family: "US-Gov-PD", pattern: "^US-Gov-PD$", sources: ["nasa"], prefixes: ["https://www.nasa.gov/"] },
    ],
    pageHosts: {
      wikimedia: ["commons.wikimedia.org"], openverse: ["openverse.org"], met: ["www.metmuseum.org"],
      aic: ["www.artic.edu"], nasa: ["images.nasa.gov"],
    },
  };
  assert.equal(JSON.stringify(TABLE), JSON.stringify(expected));
});

for (const [licence, family, source, url] of [
  ["CC0-1.0", "CC0-1.0", "met", "https://creativecommons.org/publicdomain/zero/1.0/"],
  ["CC-PDM-1.0", "CC-PDM-1.0", "aic", "https://creativecommons.org/publicdomain/mark/1.0/"],
  ["CC-BY-4.0", "CC-BY", "openverse", "https://creativecommons.org/licenses/by/4.0/"],
  ["CC-BY-3.0-US", "CC-BY", "wikimedia", "https://creativecommons.org/licenses/by/3.0/us/"],
  ["CC-BY-SA-4.0", "CC-BY-SA", "wikimedia", "https://creativecommons.org/licenses/by-sa/4.0/"],
  ["CC-BY-SA-3.0-ABCDE", "CC-BY-SA", "nasa", "https://creativecommons.org/licenses/by-sa/3.0/"],
  ["public-domain", "public-domain", "wikimedia", "https://commons.wikimedia.org/wiki/Commons:Licensing"],
  ["US-Gov-PD", "US-Gov-PD", "nasa", "https://www.nasa.gov/nasa-brand-center/images-and-media/"],
]) test(`allowed licence ${licence} maps to ${family} and its canonical prefix`, () => {
  assert.equal(licenceFamily(licence), family);
  assert.equal(licenceOk(source, licence, url), true);
});

test("licence prefixes accept exactly one appended slash and remain case-sensitive", () => {
  assert.ok(licenceOk("met", "CC0-1.0", "https://creativecommons.org/publicdomain/zero/1.0"));
  assert.ok(licenceOk("met", "CC-BY-4.0", "https://creativecommons.org/licenses/by"));
  for (const url of [
    "https://creativecommons.org/licenses/by-sa/4.0/", "https://creativecommons.org/licenses/by-other/",
    "https://creativecommons.org/Licenses/by/", "https://CreativeCommons.org/licenses/by/",
    "https://creativecommons.org.evil.invalid/licenses/by/",
  ]) assert.equal(licenceOk("met", "CC-BY-4.0", url), false, url);
});

test("raw licence paths reject traversal, percent encodings and backslashes", () => {
  const base = "https://creativecommons.org/licenses/by/";
  for (const segment of [".", "..", "%2e", "%2E", "%2e%2e", "%2E%2E", "%2e%2E", ".%2e", "%2E.", "%252e"]) {
    const url = base + segment + "/by-nc/4.0/";
    assert.equal(licenceOk("openverse", "CC-BY-4.0", url), false, url);
  }
  for (const path of [String.raw`4.0/..\..\by-nc/4.0/`, String.raw`4.0\extra`, "4.0/%41"]) {
    assert.equal(licenceOk("openverse", "CC-BY-4.0", base + path), false, path);
  }
});

test("licence queries and fragments do not change the allowed raw path", () => {
  for (const url of [
    "https://creativecommons.org/licenses/by/2.0/?ref=openverse",
    "https://creativecommons.org/licenses/by/2.0?ref=openverse#reference",
    "https://creativecommons.org/licenses/by?ref=openverse",
    "https://creativecommons.org/licenses/by/4.0/?ref=%2e%2e#../by-nc",
    String.raw`https://creativecommons.org/licenses/by/4.0/?ref=\#reference`,
  ]) assert.equal(licenceOk("openverse", "CC-BY-4.0", url), true, url);
  assert.equal(licenceOk("openverse", "CC-BY-4.0", "https://creativecommons.org/licenses/by-nc/4.0/?ref=openverse"), false);
});

for (const licence of ["private-reference", "CC-BY-NC-4.0", "CC-BY-ND-4.0", "CC-BY-SA-4", "CC-BY-4x0",
  "CC-BY-4.0-us", "CC-BY-4.0-U", "CC-BY-4.0-ABCDEF", " CC0-1.0", "CC0-1.0\n", "CC-BY-4.0\u0085", "", null, 1]) {
  test(`unknown or partial licence ${JSON.stringify(licence)} is refused`, () => {
    assert.equal(licenceFamily(licence), null);
    assert.equal(licenceOk("met", licence, "https://creativecommons.org/licenses/by/4.0/"), false);
  });
}

test("source-restricted licences and every unknown source are refused", () => {
  assert.equal(licenceOk("met", "public-domain", "https://commons.wikimedia.org/wiki/Commons:Licensing"), false);
  assert.equal(licenceOk("wikimedia", "US-Gov-PD", "https://www.nasa.gov/"), false);
  for (const source of ["archive", "demozoo", "safebooru", "private-reference", "__proto__", "toString", "", null,
    ["wikimedia"], {}, { toString: null }]) {
    assert.equal(licenceOk(source, "CC0-1.0", "https://creativecommons.org/publicdomain/zero/1.0/"), false);
    assert.equal(pageUrlOk(source, "https://commons.wikimedia.org/wiki/Test"), false);
  }
});

for (const [source, host] of Object.entries({ wikimedia: "commons.wikimedia.org", openverse: "openverse.org",
  met: "www.metmuseum.org", aic: "www.artic.edu", nasa: "images.nasa.gov" })) {
  test(`class A source ${source} accepts only its page host`, () => {
    assert.ok(pageUrlOk(source, `https://${host}/synthetic`));
    assert.ok(pageUrlOk(source, `https://${host.toUpperCase()}/synthetic`));
    assert.equal(pageUrlOk(source, `https://${host}.evil.invalid/synthetic`), false);
    assert.equal(pageUrlOk(source, "https://safebooru.org/index.php"), false);
  });
}

const unsafe = [
  ["http", (host) => `http://${host}/`],
  ["missing authority", (host) => `https:${host}/`],
  ["user info", (host) => `https://owner:secret@${host}/`],
  ["empty user info", (host) => `https://@${host}/`],
  ["explicit default port", (host) => `https://${host}:443/`],
  ["explicit other port", (host) => `https://${host}:444/`],
  ["empty explicit port", (host) => `https://${host}:/`],
  ["space", (host) => `https://${host}/a b`],
  ["newline", (host) => `https://${host}/\n`],
  ["tab", (host) => `https://${host}/\t`],
  ["C0 control", (host) => `https://${host}/\u0000`],
  ["DEL control", (host) => `https://${host}/\u007f`],
  ["C1 control", (host) => `https://${host}/\u0085`],
  ["Unicode whitespace", (host) => `https://${host}/\u00a0`],
  ["backslash authority", (host) => `https://${host}\\path`],
  ["over 500 characters", (host) => `https://${host}/` + "x".repeat(500)],
];
for (const [name, makeUrl] of unsafe) test(`both URL rules refuse ${name}`, () => {
  assert.equal(pageUrlOk("met", makeUrl("www.metmuseum.org")), false);
  assert.equal(licenceOk("nasa", "US-Gov-PD", makeUrl("www.nasa.gov")), false);
});

test("URL length 500 is inclusive and non-strings are refused", () => {
  const page = "https://www.metmuseum.org/", licence = "https://www.nasa.gov/";
  assert.ok(pageUrlOk("met", page + "x".repeat(500 - page.length)));
  assert.ok(licenceOk("nasa", "US-Gov-PD", licence + "x".repeat(500 - licence.length)));
  for (const value of [null, undefined, {}, 500, "not a URL"]) {
    assert.equal(pageUrlOk("met", value), false);
    assert.equal(licenceOk("nasa", "US-Gov-PD", value), false);
  }
});

// Class A admission rules. Keep this table identical to the local exporter.
export const TABLE = {
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
    wikimedia: ["commons.wikimedia.org"],
    openverse: ["openverse.org"],
    met: ["www.metmuseum.org"],
    aic: ["www.artic.edu"],
    nasa: ["images.nasa.gov"],
  },
};

const rules = TABLE.licences.map((rule) => ({ ...rule, regex: new RegExp(rule.pattern) }));
function licenceRule(licence) {
  if (typeof licence !== "string") return null;
  // JS's $ also matches before a final newline; require the entire match.
  const matches = rules.filter((rule) => rule.regex.exec(licence)?.[0] === licence);
  return matches.length === 1 ? matches[0] : null;
}

function safeUrl(value) {
  if (typeof value !== "string" || value.length > 500 || /[\s\u0000-\u001f\u007f-\u009f]/u.test(value)) return null;
  const authority = /^https:\/\/([^/?#]+)/i.exec(value)?.[1];
  // Inspect the spelling too: URL() removes an explicit default port and
  // accepts empty user info and backslashes as URL separators.
  if (!authority || /[@:\\]/.test(authority)) return null;
  try {
    const url = new URL(value);
    return url.protocol === "https:" && !url.username && !url.password && !url.port ? url : null;
  } catch {
    return null;
  }
}

export function licenceFamily(licence) {
  return licenceRule(licence)?.family ?? null;
}

export function licenceOk(source, licence, url) {
  if (typeof source !== "string" || !Object.hasOwn(TABLE.pageHosts, source) || !safeUrl(url)) return false;
  const rule = licenceRule(licence);
  if (!rule || (rule.sources && !rule.sources.includes(source))) return false;
  const withSlash = url.endsWith("/") ? url : url + "/";
  return rule.prefixes.some((prefix) => withSlash.startsWith(prefix));
}

export function pageUrlOk(source, value) {
  if (typeof source !== "string" || !Object.hasOwn(TABLE.pageHosts, source)) return false;
  const url = safeUrl(value);
  return Boolean(url && TABLE.pageHosts[source].includes(url.hostname));
}

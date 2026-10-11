import test from "node:test";
import assert from "node:assert/strict";
import { readFile, readdir } from "node:fs/promises";

function remoteResources(source) {
  const patterns = [
    /<(?:link|script|img|source|iframe|style)\b[^>]*https?:/gi,
    /<style\b[^>]*>[\s\S]*?https?:[\s\S]*?<\/style\s*>/gi,
    /\bstyle\s*=\s*(?:"[^"]*https?:[^"]*"|'[^']*https?:[^']*')/gi,
    /(?:@import\s*(?:url\(\s*)?["']?|url\(\s*["']?)https?:/gi,
    /\b(?:fetch|XMLHttpRequest|WebSocket|EventSource)\s*\(/g,
    /\bnavigator\s*(?:\.\s*sendBeacon|\[\s*["']sendBeacon["']\s*\])\s*\(/g,
    /\b(?:import|export)\s+(?:[^;\n]*?\s+from\s*)?["']https?:/g,
    /\bimport\s*\(\s*["']https?:/g,
    /\.\s*(?:src|srcset|poster)\s*=\s*["'`]https?:/g,
    /\.\s*setAttribute\s*\(\s*["'](?:src|srcset|poster|style)["']\s*,\s*["'`]https?:/g,
  ];
  return patterns.flatMap((pattern) => [...source.matchAll(pattern)].map((match) => match[0]));
}

const HOSTED_CALL = "fetch(url, { ...REQUEST, signal })";
const HOSTED_REQUEST = '{ cache: "no-cache", credentials: "same-origin", redirect: "error" }';
function hostedResources(source) {
  const errors = remoteResources(source.replace(HOSTED_CALL, ""));
  if (!source.includes(HOSTED_CALL) || /(?:\.\s*|[\w$])$/.test(source.split(HOSTED_CALL)[0])) errors.push("Invalid hosted fetch call.");
  if (!/export async function loadHostedBundles\(\{\s*fetch,/.test(source)) errors.push("Hosted fetch must be injected.");
  if (source.match(/export const REQUEST = (\{[^}]*\});/)?.[1] !== HOSTED_REQUEST) errors.push("Invalid hosted request options.");
  // response.body is required for streaming. Request method/body fields are not.
  if (/\b(?:method|body)\s*:|https?:\/\//i.test(source)) errors.push("Hosted requests must be same-site reads.");
  return errors;
}

async function modules(folder) {
  const entries = await readdir(folder, { withFileTypes: true }), files = [];
  for (const entry of entries) {
    const path = new URL(entry.name + (entry.isDirectory() ? "/" : ""), folder);
    if (entry.isDirectory()) files.push(...await modules(path));
    else if (entry.name.endsWith(".js")) files.push(path);
  }
  return files;
}

test("the page permits only the injected same-site hosted loader request", async () => {
  // The page also imports the shared modules in core/.
  const page = new URL("../../index.html", import.meta.url), folder = new URL("../", import.meta.url);
  const core = new URL("../../core/", import.meta.url);
  for (const file of [page, ...await modules(folder), ...await modules(core)]) {
    const source = await readFile(file, "utf8");
    const violations = file.href === new URL("../hosted.js", import.meta.url).href ? hostedResources(source) : remoteResources(source);
    assert.deepEqual(violations, [], file.pathname);
  }
});

test("the offline check detects each remote element, CSS resource and network API", () => {
  for (const tag of ["link", "script", "img", "source", "iframe", "style"]) {
    for (const scheme of ["http:", "https:"]) assert.ok(remoteResources(`<${tag} src="${scheme}//example.invalid/resource">`).length, tag);
  }
  for (const source of [
    '<style>@import "https://example.invalid/font.css";</style>',
    '<div style="background: url(https://example.invalid/image)">',
    "@import 'http://example.invalid/font.css';", 'url("https://example.invalid/image")',
    "fetch('/request')", "window.fetch('/request')", "new XMLHttpRequest()", "new WebSocket('/socket')", "new EventSource('/events')",
    "navigator.sendBeacon('/request', data)", "navigator['sendBeacon']('/request', data)",
    'import x from "https://example.invalid/module.js"', 'import("https://example.invalid/module.js")',
    'image.src = "https://example.invalid/image"', 'image.setAttribute("src", "http://example.invalid/image")',
  ]) assert.ok(remoteResources(source).length, source);
});

test("validated attribution URLs may be used in noopener noreferrer anchors", () => {
  assert.deepEqual(remoteResources(`
    const link = document.createElement("a");
    link.href = item.pageUrl;
    link.target = "_blank";
    link.rel = "noopener noreferrer";
    const validatedPrefix = "https://creativecommons.org/licenses/by/";
  `), []);
});

test("the hosted exception allows exactly its injected request and rejects any second call", () => {
  const source = `export const REQUEST = ${HOSTED_REQUEST};
    export async function loadHostedBundles({ fetch, base }) {
      await ${HOSTED_CALL};
    }`;
  assert.deepEqual(hostedResources(source), []);
  assert.ok(hostedResources(source + `\n${HOSTED_CALL};`).length, "a second hosted fetch is rejected");
  assert.ok(remoteResources(HOSTED_CALL).length, "the same fetch in any other module is rejected");
  for (const bad of [
    source.replace("fetch, base", "base"), source.replace(HOSTED_CALL, "globalThis." + HOSTED_CALL),
    source.replace('credentials: "same-origin"', 'credentials: "include"'),
    source.replace('redirect: "error"', 'redirect: "follow"'), source.replace('cache: "no-cache"', 'cache: "default"'),
    source + '\nconst options = { method: "POST" };', source + '\nconst options = { body: data };',
    source + '\nconst url = "http://example.invalid/";', source + '\nconst url = "https://example.invalid/";',
    source + '\nnew XMLHttpRequest();', source + '\nnew WebSocket("/socket");',
    source + '\nnew EventSource("/events");', source + '\nnavigator.sendBeacon("/send", data);',
  ]) assert.ok(hostedResources(bad).length, bad);
});

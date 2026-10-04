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

async function modules(folder) {
  const entries = await readdir(folder, { withFileTypes: true }), files = [];
  for (const entry of entries) {
    const path = new URL(entry.name + (entry.isDirectory() ? "/" : ""), folder);
    if (entry.isDirectory()) files.push(...await modules(path));
    else if (entry.name.endsWith(".js")) files.push(path);
  }
  return files;
}

test("the page and all page modules declare no remote resources or network calls", async () => {
  // The page also imports the shared modules in core/.
  const page = new URL("../../index.html", import.meta.url), folder = new URL("../", import.meta.url);
  const core = new URL("../../core/", import.meta.url);
  for (const file of [page, ...await modules(folder), ...await modules(core)]) {
    assert.deepEqual(remoteResources(await readFile(file, "utf8")), [], file.pathname);
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

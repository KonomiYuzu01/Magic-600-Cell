// Headless check of the explorer page with Playwright and software WebGL (SwiftShader).
//
//   NODE_PATH=$(npm root -g) PLAYWRIGHT_BROWSERS_PATH=/opt/pw-browsers \
//     node research/jumbling/explorer/check_page.mjs [--vendor DIR]
//
// It serves this folder on a local port, wraps index.html in the document skeleton that the
// publishing host adds (doctype, charset, viewport), loads the page at desktop and phone width
// in light and dark, and fails on any console error, page error, failed request, horizontal
// page scroll or blank WebGL canvas. Screenshots go to shots/.
//
// --vendor DIR answers the CDN and font requests from local copies instead of the network:
// DIR/three.module.js, DIR/OrbitControls.js (three@0.160.0), DIR/fonts.css and DIR/fonts/*
// (Google Fonts, file names are the gstatic path with '/' replaced by '_').
import { createRequire } from 'node:module';
import http from 'node:http';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const require = createRequire(import.meta.url);
const { chromium } = require('playwright');
const here = path.dirname(fileURLToPath(import.meta.url));
const vendorArg = process.argv.indexOf('--vendor');
const vendor = vendorArg > 0 ? process.argv[vendorArg + 1] : null;
const results = JSON.parse(fs.readFileSync(path.join(here, 'explorer-results.json'), 'utf8'));
const menus = new Map(results.menus.map((m) => [m.name, m]));
for (const [name, size] of [['s4', 24], ['i_a', 60], ['i_b', 60]]) {
  const m = menus.get(name);
  if (!m || m.family !== 'group' || m.size !== size || !m.exact_q_sqrt5) {
    throw new Error(`run --preset --menus s4,i_a,i_b first: missing exact group ${name} (${size})`);
  }
  if (m.j1_relation?.[name] !== 'equal') throw new Error(`${name}: expected exact J1 group equality`);
  for (const kind of ['lattice', 'all']) {
    if (!results.runs.some((r) => r.menu === name && r.twisting === kind)) {
      throw new Error(`${name}: missing ${kind} preset run`);
    }
  }
}
for (const m of results.menus.filter((m) => m.exact_q_sqrt5)) {
  if (!/^[0-9a-f]{64}$/.test(m.j1_menu_identity)
      || !['s4', 'i_a', 'i_b'].every((name) => ['equal', 'contained', 'not-contained'].includes(m.j1_relation?.[name]))) {
    throw new Error(`${m.name}: missing J1 identity or relation`);
  }
}
if (menus.get('class-00')?.j1_relation?.s4 !== 'contained') {
  throw new Error('class-00 must be reported as strictly contained in S4');
}

const TYPES = { '.html': 'text/html; charset=utf-8', '.js': 'text/javascript', '.json': 'application/json', '.png': 'image/png' };
const SKELETON = '<!doctype html><html><head><meta charset="utf-8">'
  + '<meta name="viewport" content="width=device-width, initial-scale=1, viewport-fit=cover"></head><body>';

const server = http.createServer((req, res) => {
  const url = new URL(req.url, 'http://localhost');
  const rel = url.pathname === '/' ? 'index.html' : decodeURIComponent(url.pathname.slice(1));
  const file = path.join(here, rel);
  if (!file.startsWith(here) || !fs.existsSync(file) || fs.statSync(file).isDirectory()) {
    res.writeHead(404);
    res.end();
    return;
  }
  let body = fs.readFileSync(file);
  if (rel === 'index.html') body = Buffer.concat([Buffer.from(SKELETON), body, Buffer.from('</body></html>')]);
  res.writeHead(200, { 'content-type': TYPES[path.extname(file)] || 'application/octet-stream' });
  res.end(body);
});
await new Promise((r) => server.listen(0, '127.0.0.1', r));
const base = `http://127.0.0.1:${server.address().port}/`;

const browser = await chromium.launch({
  args: ['--use-gl=angle', '--use-angle=swiftshader', '--enable-unsafe-swiftshader', '--ignore-gpu-blocklist'],
});

async function route(context) {
  if (!vendor) return;
  const serve = (route, file, type) => route.fulfill({ status: 200, contentType: type, body: fs.readFileSync(file) });
  await context.route('https://cdn.jsdelivr.net/npm/three@0.160.0/**', (r) => {
    const name = r.request().url().endsWith('OrbitControls.js') ? 'OrbitControls.js' : 'three.module.js';
    return serve(r, path.join(vendor, name), 'text/javascript');
  });
  await context.route('https://fonts.googleapis.com/**', (r) => serve(r, path.join(vendor, 'fonts.css'), 'text/css'));
  await context.route('https://fonts.gstatic.com/**', (r) => {
    const name = r.request().url().replace('https://fonts.gstatic.com/', '').replace(/\//g, '_');
    return serve(r, path.join(vendor, 'fonts', name), 'font/woff2');
  });
}

const shots = [
  { name: 'desktop-light', viewport: { width: 1280, height: 900 }, scheme: 'light', hash: '#class-00' },
  { name: 'desktop-dark-all', viewport: { width: 1280, height: 900 }, scheme: 'dark', hash: '#plane-10', kind: 'all' },
  { name: 'phone-light', viewport: { width: 390, height: 844 }, scheme: 'light', hash: '#class-05', mobile: true },
  { name: 'desktop-s4', viewport: { width: 1280, height: 900 }, scheme: 'light', hash: '#s4' },
  { name: 'desktop-i_a-all', viewport: { width: 1280, height: 900 }, scheme: 'dark', hash: '#i_a', kind: 'all' },
  { name: 'phone-i_b', viewport: { width: 390, height: 844 }, scheme: 'light', hash: '#i_b', mobile: true },
];
fs.mkdirSync(path.join(here, 'shots'), { recursive: true });
const problems = [];
for (const s of shots) {
  const context = await browser.newContext({
    viewport: s.viewport, colorScheme: s.scheme, deviceScaleFactor: 1, isMobile: !!s.mobile, hasTouch: !!s.mobile,
    reducedMotion: 'reduce',
  });
  await route(context);
  const page = await context.newPage();
  page.on('console', (m) => { if (m.type() === 'error') problems.push(`${s.name}: console error: ${m.text()}`); });
  page.on('pageerror', (e) => problems.push(`${s.name}: page error: ${e.message}`));
  page.on('requestfailed', (r) => problems.push(`${s.name}: request failed: ${r.url()} ${r.failure()?.errorText}`));
  await page.goto(base + s.hash, { waitUntil: 'networkidle' });
  await page.waitForSelector('#facts dl', { timeout: 30000 });
  if (s.kind === 'all') {
    await page.click('label[for="kind-all"]');
    await page.waitForFunction(() => document.querySelector('#stage-note').textContent.includes('every grip'));
  }
  await page.waitForTimeout(1500);
  const check = await page.evaluate(() => {
    const c = document.getElementById('scene');
    const g = document.createElement('canvas');
    g.width = 64;
    g.height = 48;
    const ctx = g.getContext('2d');
    ctx.drawImage(c, 0, 0, 64, 48);
    const px = ctx.getImageData(0, 0, 64, 48).data;
    const seen = new Set();
    for (let i = 0; i < px.length; i += 4) seen.add(`${px[i] >> 3},${px[i + 1] >> 3},${px[i + 2] >> 3}`);
    return {
      scrollWidth: document.documentElement.scrollWidth,
      innerWidth: window.innerWidth,
      colours: seen.size,
      note: document.getElementById('stage-note').textContent,
      rows: document.querySelectorAll('#levels tbody tr').length,
      identity: document.querySelector('#facts [data-j1-identity]')?.getAttribute('data-j1-identity'),
      identityTitle: document.querySelector('#facts [data-j1-identity]')?.title,
      identityText: document.querySelector('#facts [data-j1-identity]')?.textContent,
      relation: Array.from(document.querySelectorAll('#facts dt'))
        .find((dt) => dt.textContent === 'J1 relation')?.nextElementSibling.textContent,
      groups: Array.from(document.querySelectorAll('#menu optgroup[label="Groups"] option'))
        .map((o) => ({ name: o.value, text: o.textContent })),
    };
  });
  if (check.scrollWidth > check.innerWidth) problems.push(`${s.name}: horizontal scroll ${check.scrollWidth} > ${check.innerWidth}`);
  if (check.colours < 4) problems.push(`${s.name}: WebGL canvas looks blank (${check.colours} colours)`);
  const menu = menus.get(s.hash.slice(1));
  if (check.identity !== menu.j1_menu_identity || check.identityTitle !== menu.j1_menu_identity
      || check.identityText !== `${menu.j1_menu_identity.slice(0, 16)}…`) {
    problems.push(`${s.name}: missing or incorrect shortened J1 identity and full hover value`);
  }
  const groupNames = { s4: 'S4₀', i_a: 'I_a', i_b: 'I_b' };
  const relations = { equal: 'equals', contained: 'contained in', 'not-contained': 'not contained in' };
  const relation = Object.entries(menu.j1_relation)
    .map(([name, value]) => `${relations[value]} ${groupNames[name]}`).join('; ');
  if (check.relation !== relation) problems.push(`${s.name}: missing or incorrect J1 relation`);
  for (const name of ['s4', 'i_a', 'i_b']) {
    const option = check.groups.find((g) => g.name === name);
    if (!option || !option.text.includes(`group · ${menus.get(name).size} elements`)) {
      problems.push(`${s.name}: group picker lacks ${name}'s family and size`);
    }
  }
  console.log(`${s.name}: ${check.note}; ${check.rows} table rows; ${check.colours} canvas colours`);
  await page.screenshot({ path: path.join(here, 'shots', `${s.name}.png`), fullPage: true });
  await context.close();
}

// A late points response must not replace the current selection's points (review finding J4R002).
{
  const context = await browser.newContext({ viewport: { width: 1280, height: 900 }, reducedMotion: 'reduce' });
  await route(context);
  await context.route('**/points/class-00__lattice.json', async (r) => {
    await new Promise((done) => setTimeout(done, 1500));
    await r.continue();
  });
  const page = await context.newPage();
  page.on('pageerror', (e) => problems.push(`late-response: page error: ${e.message}`));
  await page.goto(base + '#class-05', { waitUntil: 'networkidle' });
  await page.waitForSelector('#facts dl', { timeout: 30000 });
  const expected = await page.evaluate(() => document.getElementById('stage-note').textContent);
  await page.selectOption('#menu', 'class-00');
  await page.selectOption('#menu', 'class-05');
  await page.waitForTimeout(3000);
  const note = await page.evaluate(() => document.getElementById('stage-note').textContent);
  if (note !== expected) problems.push(`late-response: stage shows "${note}", expected "${expected}"`);
  console.log(`late-response: ${note}`);
  await context.close();
}

// The ball and its captions follow the threshold recorded in the results (review finding J4R003).
{
  const context = await browser.newContext({ viewport: { width: 1280, height: 900 }, reducedMotion: 'reduce' });
  await route(context);
  const custom = { ...results, parameters: { ...results.parameters, threshold_deg: 30 } };
  await context.route('**/explorer-results.json', (r) => r.fulfill({
    status: 200, contentType: 'application/json', body: JSON.stringify(custom) }));
  const page = await context.newPage();
  page.on('pageerror', (e) => problems.push(`threshold: page error: ${e.message}`));
  await page.goto(base + '#class-00', { waitUntil: 'networkidle' });
  await page.waitForSelector('#facts dl', { timeout: 30000 });
  const labels = await page.evaluate(() => Array.from(document.querySelectorAll('[data-threshold]'), (n) => n.textContent));
  if (labels.length < 2 || labels.some((t) => t !== '30°')) problems.push(`threshold: captions ${JSON.stringify(labels)}, expected 30°`);
  console.log(`threshold: ${labels.length} captions show ${[...new Set(labels)].join(', ')}`);
  await context.close();
}
await browser.close();
server.close();
if (problems.length) {
  console.log(problems.join('\n'));
  process.exit(1);
}
console.log('page check passed: no console errors, no failed requests, no horizontal scroll');

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
    };
  });
  if (check.scrollWidth > check.innerWidth) problems.push(`${s.name}: horizontal scroll ${check.scrollWidth} > ${check.innerWidth}`);
  if (check.colours < 4) problems.push(`${s.name}: WebGL canvas looks blank (${check.colours} colours)`);
  console.log(`${s.name}: ${check.note}; ${check.rows} table rows; ${check.colours} canvas colours`);
  await page.screenshot({ path: path.join(here, 'shots', `${s.name}.png`), fullPage: true });
  await context.close();
}
await browser.close();
server.close();
if (problems.length) {
  console.log(problems.join('\n'));
  process.exit(1);
}
console.log('page check passed: no console errors, no failed requests, no horizontal scroll');

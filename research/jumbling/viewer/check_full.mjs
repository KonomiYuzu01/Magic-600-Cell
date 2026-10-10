// Source/fixture evidence only. The integrator runs this OUTSIDE the implementation sandbox.
// node research/jumbling/viewer/check_full.mjs [--vendor DIR] [--port 8601]
// node research/jumbling/viewer/check_full.mjs --serve [--port 8601]
import { createServer } from 'node:http';
import { createHash } from 'node:crypto';
import { readFileSync, existsSync, statSync } from 'node:fs';
import { dirname, join, normalize, extname, sep } from 'node:path';
import { fileURLToPath } from 'node:url';
import { createRequire } from 'node:module';
import { execSync } from 'node:child_process';

const HERE = dirname(fileURLToPath(import.meta.url));
const args = process.argv.slice(2);
const opt = (name) => { const i = args.indexOf(name); return i >= 0 ? args[i + 1] : null; };
const CDN = 'https://cdn.jsdelivr.net/npm/three@0.160.0/';
const PINNED = {
  'build/three.module.js': ['three.module.js', '76dea8151bc9352aef3528b4262e249b2604f62543828328db978d060d61a495'],
  'examples/jsm/controls/OrbitControls.js': ['OrbitControls.js', '5a44a9e86a2a0fb11933eed69bc2cd33c76a496854c1aed6ed776efa87d7b064'],
};
const sha = (body) => createHash('sha256').update(body).digest('hex');
const json = (file) => JSON.parse(readFileSync(file, 'utf8'));
const failures = [];
let assertions = 0;
function check(ok, what) {
  assertions++;
  console.log(`${ok ? 'ok  ' : 'FAIL'} ${what}`);
  if (!ok) failures.push(what);
}
const close = (a, b, tolerance = 1e-4) => a.length === b.length && a.every((x, i) =>
  Number.isFinite(x) && Number.isFinite(b[i]) && Math.abs(x - b[i]) <= tolerance * (1 + Math.abs(b[i])));
const matColumn = (m, x) => [0, 1, 2, 3].map((r) => x.reduce((s, c, k) => s + m[k * 4 + r] * c, 0));
const dot = (a, b) => a.reduce((s, x, i) => s + x * b[i], 0);
function expectedProjection(asset, parameters) {
  const x = matColumn(parameters.Q, asset.map((v) => v / parameters.radius));
  const p = parameters.kind === 'perspective'
    ? x.slice(0, 3).map((v) => parameters.d4 * v / (parameters.d4 - x[3]))
    : parameters.basis.map((b) => dot(x, b) / (1 - dot(x, parameters.pole)));
  const clip = matColumn(parameters.camera3, matColumn(parameters.view, [...p, 1]));
  return { projected: p, ndc: clip.slice(0, 3).map((v) => v / clip[3]) };
}

function serve(port) {
  const types = { '.html': 'text/html; charset=utf-8', '.js': 'text/javascript; charset=utf-8',
    '.json': 'application/json', '.b64': 'text/plain; charset=utf-8' };
  const server = createServer((req, res) => {
    let rel;
    try { rel = decodeURIComponent(new URL(req.url, 'http://localhost').pathname).replace(/^\/+/, ''); }
    catch { res.writeHead(400); res.end(); return; }
    if (!rel) rel = 'full.html';
    if (rel === 'favicon.ico') { res.writeHead(204); res.end(); return; }
    const path = normalize(join(HERE, rel));
    if (!path.startsWith(HERE + sep) || !existsSync(path) || !statSync(path).isFile()) {
      res.writeHead(404); res.end('not found'); return;
    }
    res.writeHead(200, { 'content-type': types[extname(path)] || 'application/octet-stream' });
    res.end(readFileSync(path));
  });
  return new Promise((resolve, reject) => {
    server.once('error', reject);
    server.listen(port, '127.0.0.1', () => resolve(server));
  });
}
async function loadPlaywright() {
  try { return await import('playwright'); } catch { /* same fallback as check_viewer.mjs */ }
  const root = execSync('npm root -g').toString().trim();
  return createRequire(join(root, 'noop.js'))('playwright');
}
function vendorFiles(dir) {
  const out = {};
  for (const [path, [name, digest]] of Object.entries(PINNED)) {
    const body = readFileSync(join(dir, name));
    if (sha(body) !== digest) throw new Error(`${name}: pinned three.js 0.160.0 digest mismatch`);
    out[CDN + path] = body;
  }
  return out;
}
function checkedArrays(directory, header) {
  const A = {};
  for (const [name, d] of Object.entries(header.files)) {
    const raw = readFileSync(join(directory, name));
    check(sha(raw) === d.sha256, `${directory.split(sep).slice(-2).join('/')}/${name}: header SHA-256`);
    if (!d.dtype) continue;
    const Type = { '<f4': Float32Array, '<i4': Int32Array, '<u4': Uint32Array }[d.dtype];
    if (!Type || raw.length !== d.shape.reduce((a, b) => a * b, 1) * 4) throw new Error(`${name}: invalid type or shape`);
    A[name] = new Type(Uint8Array.from(raw).buffer);
  }
  return A;
}

async function main() {
  const server = await serve(Number(opt('--port') || 8601));
  const base = `http://127.0.0.1:${server.address().port}/`;
  if (args.includes('--serve')) { console.log(`Full-state viewer: ${base} (Ctrl+C to stop)`); return; }
  let browser;
  try {
    const vendor = opt('--vendor') ? vendorFiles(opt('--vendor')) : {};
    const { chromium } = await loadPlaywright();
    browser = await chromium.launch({ args: ['--use-angle=swiftshader', '--enable-unsafe-swiftshader', '--ignore-gpu-blocklist'] });
    const forbidden = [], requested = [], moduleCache = new Map();
    const geometry = json(join(HERE, 'full/geometry.json'));
    checkedArrays(join(HERE, 'full'), geometry);

    async function openPage(hash = '#S4-end', profile = 'light', fault = {}) {
      const context = await browser.newContext({ viewport: profile === 'phone'
        ? { width: 390, height: 844 } : { width: 1280, height: 860 },
      colorScheme: profile === 'dark' ? 'dark' : 'light', deviceScaleFactor: 1 });
      const page = await context.newPage();
      page.setDefaultTimeout(180000);
      const errors = [];
      page.on('console', (m) => {
        if (m.type() === 'error' && !(fault.fallback && m.text().includes('404'))) errors.push(m.text());
      });
      page.on('pageerror', (e) => errors.push(String(e)));
      await page.addInitScript(() => {
        window._fullInstancedDraws = [];
        window._fullTexUploads = 0;
        const proto = WebGL2RenderingContext.prototype;
        const draw = proto.drawArraysInstanced;
        proto.drawArraysInstanced = function (mode, first, count, instances) {
          window._fullInstancedDraws.push({ mode, first, count, instances });
          if (window._fullInstancedDraws.length > 64) window._fullInstancedDraws.shift();
          return draw.call(this, mode, first, count, instances);
        };
        const upload = proto.texImage2D;
        proto.texImage2D = function (...args) { window._fullTexUploads++; return upload.apply(this, args); };
      });
      await page.route('**/*', async (route) => {
        const u = route.request().url();
        requested.push(u);
        if (u.startsWith(base)) {
          const rel = decodeURIComponent(new URL(u).pathname).replace(/^\/+/, '');
          if (fault.dimension && rel === 'full/S4/end/header.json') {
            const h = json(join(HERE, rel)); h.dimension = 5;
            return route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify(h) });
          }
          if (fault.fallback && rel === 'full/S4/end/pose_index.i32')
            return route.fulfill({ status: 404, body: 'text-only host' });
          const binary = rel.endsWith('.b64') ? rel.slice(0, -4) : rel;
          if ((fault.b64 && rel.endsWith('.b64')) || (fault.tamper && binary === 'full/S4/end/pose_index.i32')) {
            const file = normalize(join(HERE, binary));
            if (!file.startsWith(HERE + sep) || !existsSync(file)) return route.abort();
            const body = Buffer.from(readFileSync(file));
            if (fault.tamper && binary === 'full/S4/end/pose_index.i32') body[0] ^= 1;
            return route.fulfill({ status: 200, contentType: rel.endsWith('.b64') ? 'text/plain' : 'application/octet-stream',
              body: rel.endsWith('.b64') ? body.toString('base64') : body });
          }
          return route.continue();
        }
        const pinnedPath = Object.keys(PINNED).find((path) => u === CDN + path);
        if (pinnedPath) {
          let body = vendor[u] || moduleCache.get(u);
          if (!body) {
            const response = await route.fetch();
            body = await response.body();
            if (!response.ok() || sha(body) !== PINNED[pinnedPath][1]) {
              forbidden.push(`unpinned module: ${u}`); return route.abort();
            }
            moduleCache.set(u, body);
          }
          return route.fulfill({ status: 200, contentType: 'text/javascript', body });
        }
        if (u.startsWith('https://fonts.googleapis.com/'))
          return route.fulfill({ status: 200, contentType: 'text/css', body: '' });
        if (u.startsWith('https://fonts.gstatic.com/')) return route.fulfill({ status: 200, body: '' });
        forbidden.push(u);
        return route.abort();
      });
      await page.goto(`${base}full.html${fault.queryB64 ? '?b64' : ''}${hash}`, { waitUntil: 'domcontentloaded', timeout: 180000 });
      await page.waitForFunction(() => window.fullViewer && (window.fullViewer.ready || window.fullViewer.errors.length), null, { timeout: 180000 });
      return { page, context, errors };
    }
    const choose = async (page, menu, stage) => {
      await page.evaluate(async ([m, s]) => { await window.fullViewer.goto(m, s); }, [menu, stage]);
      check(await page.evaluate(() => window.fullViewer.ready), `${menu}/${stage}: ready`);
    };

    for (const menu of ['S4', 'I_a', 'I_b']) {
      const fixture = json(join(HERE, '..', 'fixtures', `wj-${menu}.json`));
      const header = json(join(HERE, 'full', menu, 'end', 'header.json'));
      const A = checkedArrays(join(HERE, 'full', menu, 'end'), header);
      const off = Array.from(A['pose_index.i32']).filter((p) => A['pose_lattice.i32'][p] < 0).length;
      check(off === fixture.stages.end.off_lattice, `${menu}: array off-lattice count equals the fixture`);
      const { page, context, errors } = await openPage(`#${menu}-end`);
      const summary = await page.evaluate(() => ({ ready: window.fullViewer.ready, model: window.fullViewer.model,
        off: window.fullViewer.offLattice, header: window.fullViewer.header, drawn: window.fullViewer.drawn,
        glDraw: window._fullInstancedDraws.at(-1), uploads: window._fullTexUploads }));
      check(summary.ready && summary.header.pieces === header.pieces && summary.model.pieces === header.pieces
        && summary.glDraw?.mode === 4 && summary.glDraw.first === 0 && summary.glDraw.count === geometry.base_vertices
        && summary.glDraw.instances === geometry.cells && summary.drawn.calls === 1
        && summary.drawn.instances === geometry.cells && summary.drawn.triangles === geometry.base_vertices * geometry.cells / 3
        && summary.glDraw.instances * geometry.base_stickers === geometry.slots && summary.model.slots === geometry.slots,
      `${menu}/end: one actual instanced draw covers the header's 600 facets and 259800 sticker slots`);
      check(summary.off === off && (await page.locator('#offLattice').innerText()).startsWith(`${off} / 177120`),
        `${menu}/end: panel off-lattice count comes from the arrays`);
      check(summary.header.digest === fixture.stages.end.digest && summary.header.revision === fixture.stages.end.records,
        `${menu}/end: revision and exact digest match J1`);
      check(await page.locator('#blocked option').count() === fixture.end_survey.blocked
        && await page.locator('#blockedCount').innerText() === String(fixture.end_survey.blocked),
      `${menu}/end: every blocked grip is listed`);
      for (const projection of ['perspective', 'stereographic']) {
        await page.locator('#projection').selectOption(projection);
        const coverage = await page.evaluate(() => window.fullViewer.coverage());
        check(coverage > 0.001, `${menu}/end: ${projection} has non-background readPixels (${(coverage * 100).toFixed(2)}%)`);
      }

      const cert = fixture.end_survey.certificates[0];
      await page.locator('#blocked').selectOption(String(cert.grip));
      let markers = await page.evaluate(() => { window.fullViewer.coverage(); return window.fullViewer.markerData(); });
      check(markers.length === 2 && await page.locator('#markers .below').count() === 1
        && await page.locator('#markers .above').count() === 1
        && await page.evaluate((piece) => window.fullViewer.focusPiece === piece, cert.piece),
      `${menu}: selecting a blocked grip highlights its piece and places two labelled markers`);
      for (const projection of ['perspective', 'stereographic']) {
        await page.locator('#projection').selectOption(projection);
        const actual = await page.evaluate(() => { window.fullViewer.coverage(); return {
          markers: window.fullViewer.markerData(), parameters: window.fullViewer.projectionParameters() }; });
        check(actual.markers.every((m) => {
          const point = cert[`point_${m.side}_float`];
          const projected = expectedProjection(point, actual.parameters);
          return close(m.asset, point, 1e-10) && close(m.projected, projected.projected, 1e-8)
            && close(m.uploaded, projected.projected) && close(m.ndc, projected.ndc);
        }), `${menu}: certificate points use the CPU ${projection} and camera3 chain`);
        markers = actual.markers;
        await page.evaluate(() => {
          document.getElementById('facetShrink').value = '0.55';
          document.getElementById('facetShrink').dispatchEvent(new Event('input'));
          document.getElementById('stickerShrink').value = '0.6';
          document.getElementById('stickerShrink').dispatchEvent(new Event('input'));
        });
        const shrunk = await page.evaluate(() => { window.fullViewer.coverage(); return window.fullViewer.markerData(); });
        check(shrunk.every((m, i) => close(m.uploaded, markers[i].uploaded, 1e-8)), `${menu}: shrink leaves certificate points fixed`);
      }
      await page.locator('#isolate').click();
      check(await page.locator('#filter').inputValue() === 'isolate' && await page.locator('#piece').inputValue() === String(cert.piece)
        && await page.evaluate(() => window.fullViewer.coverage()) > 0, `${menu}: certificate isolate view draws the piece and its markers`);
      await page.locator('#clear').click();
      await page.locator('#resetView').click();
      for (const filter of ['off', 'on', 'all']) {
        await page.locator('#filter').selectOption(filter);
        const result = await page.evaluate(() => ({ coverage: window.fullViewer.coverage(), off: window.fullViewer.offLattice,
          digest: window.fullViewer.header.digest, pieces: window.fullViewer.model.pieces }));
        check(result.coverage > 0 && result.off === off && result.digest === header.digest && result.pieces === header.pieces,
          `${menu}: ${filter} filter changes drawing and preserves the adopted state`);
      }
      await page.locator('#tint').check();
      check(await page.evaluate(() => window.fullViewer.coverage()) > 0, `${menu}: off-lattice tint draws`);
      await page.locator('#tint').uncheck();

      await choose(page, menu, 'solved');
      check(await page.evaluate(() => window.fullViewer.offLattice === 0 && window.fullViewer.coverage() > 0
        && document.getElementById('blocked').options.length === 0), `${menu}: shared solved state draws with no end-state certificates`);
      await choose(page, menu, 'sweep-before');
      check(await page.locator('#blocked option').count() === 0, `${menu}: before-state drawing does not reuse end certificates`);
      await page.evaluate(() => {
        document.getElementById('rotateX').value = '17';
        document.getElementById('rotateX').dispatchEvent(new Event('input'));
        document.getElementById('rotateY').value = '-11';
        document.getElementById('rotateY').dispatchEvent(new Event('input'));
      });
      for (const projection of ['perspective', 'stereographic']) {
        await page.locator('#projection').selectOption(projection);
        for (const [t, phase] of [[0, 'start'], [0.5, 'mid'], [1, 'end'], [1.5, 'mid'], [2, 'start']]) {
          // Dispatch the UI input, rather than setting only the test API's angle.
          await page.evaluate((t) => {
            const range = document.getElementById('scrub'); range.value = String(t); range.dispatchEvent(new Event('input'));
          }, t);
          const probe = await page.evaluate(() => window.fullViewer.sampleSweep());
          check(probe.t === t && probe.rows.length === 16 && probe.rows.every((row) => {
            const source = fixture.sweep.samples.find((s) => s.piece === row.piece);
            const expected = expectedProjection(source[phase], probe.parameters);
            return close(row.world, source[phase]) && close(row.projected, expected.projected) && close(row.ndc, expected.ndc);
          }), `${menu}: 16 GPU samples at ${t} (${phase}) match fixture poses through ${projection}, Q and camera3`);
        }
      }
      await page.locator('#play').click();
      await page.waitForFunction(() => window.fullViewer.playing && window.fullViewer.view.t > 0);
      await page.locator('#play').click();
      const t1 = await page.evaluate(() => window.fullViewer.view.t);
      await page.waitForTimeout(350);
      const t2 = await page.evaluate(() => window.fullViewer.view.t);
      check(t1 > 0 && t1 === t2 && await page.locator('#play').getAttribute('aria-pressed') === 'false', `${menu}: play advances and pause holds`);
      check(errors.length === 0 && await page.evaluate(() => window.fullViewer.errors.length === 0),
        `${menu}: no desktop light console or page errors${errors.length ? `: ${errors.join(' | ')}` : ''}`);
      await context.close();

      for (const profile of ['dark', 'phone']) {
        const p = await openPage(`#${menu}-end`, profile);
        check(await p.page.evaluate(() => window.fullViewer.ready && window.fullViewer.coverage() > 0), `${menu}: ${profile} draws the full state`);
        if (profile === 'dark') check(await p.page.evaluate(() => getComputedStyle(document.body).backgroundColor) === 'rgb(18, 22, 27)', `${menu}: dark colour tokens apply`);
        else check(await p.page.evaluate(() => document.documentElement.scrollWidth <= document.documentElement.clientWidth), `${menu}: no horizontal scroll at 390 px`);
        check(p.errors.length === 0 && await p.page.evaluate(() => window.fullViewer.errors.length === 0), `${menu}: no ${profile} console or page errors`);
        await p.context.close();
      }
    }

    const linked = await openPage('#I_b-end');
    check(await linked.page.evaluate(() => window.fullViewer.view.menu === 'I_b' && window.fullViewer.view.state === 'end'
      && document.getElementById('menu').value === 'I_b' && document.getElementById('state').value === 'end'
      && location.hash === '#I_b-end'), 'deep link #I_b-end restores both controls');
    await linked.page.locator('#menu').selectOption('S4');
    await linked.page.waitForFunction(() => window.fullViewer.ready && window.fullViewer.view.menu === 'S4');
    check(await linked.page.evaluate(() => location.hash === '#S4-end'), 'menu control writes a bare anchor');
    await linked.context.close();

    for (const fault of [{ b64: true, queryB64: true }, { b64: true, fallback: true }]) {
      const page = await openPage('#S4-end', 'light', fault);
      check(await page.page.evaluate(() => window.fullViewer.ready && window.fullViewer.coverage() > 0)
        && page.errors.length === 0, fault.queryB64 ? '?b64 decodes and checks every array' : 'binary HTTP failure falls back to checked base64');
      await page.context.close();
    }
    for (const [name, fault] of [['tampered binary array', { tamper: true }],
      ['tampered base64 array', { tamper: true, b64: true, queryB64: true }], ['dimension 5 header', { dimension: true }]]) {
      const p = await openPage('#S4-end', 'light', fault);
      const refusal = await p.page.evaluate(() => ({ ready: window.fullViewer.ready, model: window.fullViewer.model,
        draws: window._fullInstancedDraws.length, uploads: window._fullTexUploads, error: document.getElementById('error').textContent,
        coverage: window.fullViewer.coverage() }));
      check(!refusal.ready && !refusal.model && refusal.draws === 0 && refusal.uploads === 0 && refusal.coverage === 0
        && refusal.error.includes(fault.dimension ? 'dimension 5' : 'SHA-256 mismatch'), `${name} is refused before any GPU upload or draw`);
      await p.context.close();
    }
    // A failed later adoption must clear the previous drawing too.
    const stale = await openPage('#S4-end');
    await stale.page.route('**/full/I_b/end/pose_index.i32', (route) => {
      const body = Buffer.from(readFileSync(join(HERE, 'full/I_b/end/pose_index.i32'))); body[0] ^= 1;
      return route.fulfill({ status: 200, contentType: 'application/octet-stream', body });
    });
    await stale.page.locator('#menu').selectOption('I_b');
    await stale.page.waitForFunction(() => window.fullViewer.errors.length > 0);
    check(await stale.page.evaluate(() => !window.fullViewer.ready && !window.fullViewer.model && window.fullViewer.coverage() === 0),
      'failed adoption clears the previously drawn state');
    await stale.context.close();
    check(forbidden.length === 0, `no request leaves the index.html allowlist (${forbidden.length} refused)`);
    check(requested.every((u) => u.startsWith(base) || Object.keys(PINNED).some((p) => u === CDN + p)
      || u.startsWith('https://fonts.googleapis.com/') || u.startsWith('https://fonts.gstatic.com/')), 'every requested URL is allowlisted');
    console.log(`${assertions - failures.length}/${assertions} full-state browser assertions passed`);
    process.exitCode = failures.length ? 1 : 0;
  } finally {
    if (browser) await browser.close();
    await new Promise((resolve) => server.close(resolve));
  }
}
main().catch((e) => { console.error(e); process.exitCode = 1; });

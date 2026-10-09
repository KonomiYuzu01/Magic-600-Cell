// Headless check of the J2 jumbling viewer with Playwright (Chromium, software WebGL).
//
//   node research/jumbling/viewer/check_viewer.mjs [--vendor DIR] [--shots]
//   node research/jumbling/viewer/check_viewer.mjs --serve [--port 8600]
//
// It serves this directory over local HTTP, wrapping index.html in the same minimal document
// skeleton that the private-page host adds at publish time, loads the page in headless Chromium
// with SwiftShader WebGL, and asserts: no console errors or page errors, both views draw
// non-background pixels, the grip counts of every exact state match scene.json, the animated
// twist families end on the exported poses, and the page has no horizontal scroll at phone width.
// With --shots it writes four small PNG screenshots to shots/.
//
// --vendor DIR serves three.js 0.160.0 (three.module.js, OrbitControls.js) from DIR instead of
// the CDN, for sessions whose browser cannot reach cdn.jsdelivr.net; the files must match the
// pinned SHA-256 digests below. Google Fonts requests get an empty stylesheet, so the check
// renders with the fallback fonts. Other external requests are refused and fail the check.
// Evidence kind: source and synthetic geometry in a headless browser; nothing here is Windows,
// Direct3D 12, input or performance evidence.
import { createServer } from 'node:http';
import { createHash } from 'node:crypto';
import { readFileSync, existsSync, mkdirSync, statSync } from 'node:fs';
import { dirname, join, normalize, extname, sep } from 'node:path';
import { fileURLToPath } from 'node:url';
import { createRequire } from 'node:module';
import { execSync } from 'node:child_process';

const HERE = dirname(fileURLToPath(import.meta.url));
const args = process.argv.slice(2);
const opt = (name) => { const i = args.indexOf(name); return i >= 0 ? args[i + 1] : null; };
const has = (name) => args.includes(name);

const CDN = 'https://cdn.jsdelivr.net/npm/three@0.160.0/';
const PINNED = {
  'build/three.module.js': ['three.module.js', '76dea8151bc9352aef3528b4262e249b2604f62543828328db978d060d61a495'],
  'examples/jsm/controls/OrbitControls.js': ['OrbitControls.js', '5a44a9e86a2a0fb11933eed69bc2cd33c76a496854c1aed6ed776efa87d7b064'],
};
const SKELETON = ['<!doctype html><html lang="en"><head><meta charset="utf-8">',
  '<meta name="viewport" content="width=device-width, initial-scale=1, viewport-fit=cover">',
  '<style>:root{color-scheme:light;padding-top:env(safe-area-inset-top,0px);padding-bottom:env(safe-area-inset-bottom,0px)}',
  'body{margin:0;font:14px system-ui,sans-serif}img{max-width:100%}[hidden]{display:none!important}</style></head><body>'].join('');
const TYPES = { '.html': 'text/html; charset=utf-8', '.js': 'text/javascript; charset=utf-8', '.json': 'application/json',
  '.bin': 'application/octet-stream', '.png': 'image/png', '.md': 'text/plain; charset=utf-8' };

function serve(port) {
  const server = createServer((req, res) => {
    const url = new URL(req.url, 'http://localhost');
    let rel = decodeURIComponent(url.pathname).replace(/^\/+/, '');
    if (rel === '' || rel === 'index.html') {
      const body = SKELETON + readFileSync(join(HERE, 'index.html'), 'utf8') + '</body></html>';
      res.writeHead(200, { 'content-type': TYPES['.html'] });
      res.end(body);
      return;
    }
    if (rel === 'favicon.ico') { res.writeHead(204); res.end(); return; }
    const file = normalize(join(HERE, rel));
    if (!file.startsWith(HERE + sep) || !existsSync(file) || !statSync(file).isFile()) {
      res.writeHead(404); res.end('not found'); return;
    }
    res.writeHead(200, { 'content-type': TYPES[extname(file)] || 'application/octet-stream' });
    res.end(readFileSync(file));
  });
  return new Promise((resolve) => server.listen(port, '127.0.0.1', () => resolve(server)));
}

async function loadPlaywright() {
  try { return await import('playwright'); } catch { /* fall back to the global module */ }
  const root = execSync('npm root -g').toString().trim();
  return createRequire(join(root, 'noop.js'))('playwright');
}

function vendorFiles(dir) {
  const out = {};
  for (const [path, [name, digest]] of Object.entries(PINNED)) {
    const body = readFileSync(join(dir, name));
    const got = createHash('sha256').update(body).digest('hex');
    if (got !== digest) throw new Error(`${name} does not match the pinned three.js 0.160.0 digest`);
    out[CDN + path] = body;
  }
  return out;
}

const failures = [];
const check = (ok, what) => { console.log(`${ok ? 'ok  ' : 'FAIL'} ${what}`); if (!ok) failures.push(what); };

function certificateMatches(text, cert) {
  if (!(cert.h_below < 0 && cert.h_above > 0)) return false;
  return text.trim() === `vertex ${cert.vertex_below}: h = −${Math.abs(cert.h_below).toFixed(5)}; `
    + `vertex ${cert.vertex_above}: h = +${cert.h_above.toFixed(5)} (exact signs)`;
}

async function main() {
  const port = Number(opt('--port') || 8600);
  const server = await serve(port);
  const base = `http://127.0.0.1:${port}/`;
  if (has('--serve')) { console.log(`serving the viewer at ${base} (Ctrl+C to stop)`); return; }
  const vendor = opt('--vendor') ? vendorFiles(opt('--vendor')) : null;
  const scene = JSON.parse(readFileSync(join(HERE, 'scene.json'), 'utf8'));
  const sceneBytes = readFileSync(join(HERE, scene.bin.file));
  const buffer = Uint8Array.from(sceneBytes).buffer;
  const typed = { float32: Float32Array, uint8: Uint8Array, uint16: Uint16Array, uint32: Uint32Array, int32: Int32Array };
  const A = Object.fromEntries(Object.entries(scene.bin.arrays).map(([name, d]) => [name, new typed[d.dtype](buffer, d.offset, d.length)]));
  const { chromium } = await loadPlaywright();
  const browser = await chromium.launch({ args: ['--use-angle=swiftshader', '--enable-unsafe-swiftshader', '--ignore-gpu-blocklist'] });
  const refused = [];

  async function openPage(viewport, colorScheme, hash = '', data = null) {
    const context = await browser.newContext({ viewport, colorScheme, deviceScaleFactor: 1 });
    const page = await context.newPage();
    const errors = [];
    page.on('console', (m) => { if (m.type() === 'error') errors.push(m.text()); });
    page.on('pageerror', (e) => errors.push(String(e)));
    await page.route('**/*', (route) => {
      const u = route.request().url();
      if (data && u === base + 'scene.json') return route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify(data.header) });
      if (data && u === base + data.header.bin.file) return route.fulfill({ status: 200,
        contentType: data.header.bin.encoding === 'base64' ? 'text/plain' : 'application/octet-stream', body: data.body });
      if (u.startsWith(base)) return route.continue();
      if (vendor && vendor[u]) return route.fulfill({ status: 200, contentType: 'text/javascript', body: vendor[u] });
      if (u.startsWith(CDN)) return route.continue();
      if (u.startsWith('https://fonts.googleapis.com/')) return route.fulfill({ status: 200, contentType: 'text/css', body: '' });
      refused.push(u);
      return route.abort();
    });
    await page.goto(base + hash);
    await page.waitForFunction(() => window.jumbleViewer && (window.jumbleViewer.ready || window.jumbleViewer.errors.length), null, { timeout: 60000 });
    return { page, context, errors };
  }

  const shots = has('--shots');
  if (shots) mkdirSync(join(HERE, 'shots'), { recursive: true });

  // desktop, light
  const { page, context, errors } = await openPage({ width: 1280, height: 860 }, 'light');
  const ready = await page.evaluate(() => window.jumbleViewer.ready);
  check(ready, 'viewer loads scene.json and scene.bin and reports ready');
  const model = await page.evaluate(() => window.jumbleViewer.model);
  check(model.pieces === scene.counts.pieces && model.stickers === scene.counts.stickers,
    `model has ${model.pieces} pieces and ${model.stickers} stickers, as scene.json says`);
  const fam = await page.evaluate(() => window.jumbleViewer.familyCheck());
  check(fam < 1e-9, `animated twist families end on the exported poses (max float deviation ${fam.toExponential(2)})`);

  const clipOf = async (selectors) => {
    const boxes = [];
    for (const s of selectors) boxes.push(await page.locator(s).first().boundingBox());
    const x = Math.min(...boxes.map((b) => b.x)), y = Math.min(...boxes.map((b) => b.y));
    const r = Math.max(...boxes.map((b) => b.x + b.width)), btm = Math.max(...boxes.map((b) => b.y + b.height));
    return { x, y, width: r - x, height: btm - y };
  };

  for (let k = 0; k < scene.states.length; k++) {
    await page.evaluate((t) => window.jumbleViewer.goto(t), k);
    const counts = await page.locator('#gripCounts').innerText();
    const st = scene.states[k];
    check(counts.includes(`${st.admissible}`) && counts.includes(`${st.blocked}`),
      `S${k}: panel shows ${st.admissible} admissible and ${st.blocked} blocked grips`);
    const badge = await page.locator('#gBadge').innerText();
    check(badge.includes('exact'), `S${k}: views are labelled as an exact state`);
  }
  await page.evaluate(() => window.jumbleViewer.goto(1.5));
  const pbadge = await page.locator('#gBadge').innerText();
  check(pbadge.includes('uncertified'), 'between states the views are labelled as an uncertified float preview');

  // A retained turn moves piece 1035 between two lattice poses; the intermediate pose is a preview.
  await page.evaluate(() => { window.jumbleViewer.goto(1); window.jumbleViewer.clearGrip(); window.jumbleViewer.selectPiece(1035); });
  const latticeBefore = (await page.locator('#lHint').innerText()).includes('On the lattice');
  await page.evaluate(() => window.jumbleViewer.goto(1.5));
  const movingInfo = await page.locator('#pieceInfo').innerText();
  const movingHint = await page.locator('#lHint').innerText();
  const movingCaption = await page.locator('#lWhat').innerText();
  check(movingInfo.includes('moving (float preview)') && movingHint.includes('Moving (float preview)')
    && !/on the lattice|sits in a slot|at its lattice slot/i.test([movingInfo, movingHint, movingCaption].join(' ')),
    'S1.5, piece 1035: retained-turn motion is a float preview without a lattice or slot claim');
  await page.evaluate(() => window.jumbleViewer.goto(2));
  check(latticeBefore && (await page.locator('#lHint').innerText()).includes('On the lattice'),
    'piece 1035 keeps its lattice status at both exact endpoints of the retained turn');

  // shot 1: solved
  await page.evaluate(() => { window.jumbleViewer.goto(0); window.jumbleViewer.clearGrip(); });
  let cov = await page.evaluate(() => window.jumbleViewer.coverage());
  check(cov.global > 0.02, `S0: Global view draws the patch (${(cov.global * 100).toFixed(1)}% of pixels)`);
  if (shots) await page.screenshot({ path: join(HERE, 'shots', '1-solved.png'), fullPage: true, clip: await clipOf(['main.views', '.rail']) });

  // shot 2: after (c, g), the negative-control grip 1 selected
  await page.evaluate((id) => { window.jumbleViewer.goto(1); window.jumbleViewer.selectGrip(1); window.jumbleViewer.selectPiece(id); },
    scene.attempts[0].certificate.piece_id);
  cov = await page.evaluate(() => window.jumbleViewer.coverage());
  check(cov.global > 0.02 && cov.local > 0.01, `S1, grip 1: both views draw (${(cov.global * 100).toFixed(1)}%, ${(cov.local * 100).toFixed(1)}%)`);
  const cert1 = await page.locator('#pieceInfo dt:has-text("Certificate") + dd').innerText();
  const att = scene.attempts[0].certificate;
  check(certificateMatches(cert1, att),
    'S1, grip 1: the Certificate row matches both vertex identities and signed h values of witness.py');
  check(!certificateMatches(cert1.replace('h = −', 'h = +'), att)
    && !certificateMatches(cert1.replace('h = +', 'h = −'), att),
    'the certificate assertion rejects a reversed below sign or above sign');

  // Piece 7's above-cut vertex was moved below the cut by the default sticker shrink.
  const p7 = A.piece_ids.indexOf(7);
  let r7 = -1;
  for (let r = A.cert_start[1]; r < A.cert_start[2]; r++) {
    if (A.cert_grip[r] === 1 && A.cert_piece[r] === p7) { r7 = r; break; }
  }
  check(r7 >= 0, 'scene exports the S1, grip 1, piece 7 regression certificate');
  if (r7 >= 0) {
    const mat = scene.poses[A.state_pose[A.piece_ids.length + p7]].m;
    const expectedWorld = [A.cert_vertex_below[r7], A.cert_vertex_above[r7]].map((j) => {
      const o = (A.vert_start[p7] + j) * 4;
      return [0, 1, 2, 3].map((row) => [0, 1, 2, 3].reduce((sum, col) => sum + mat[row * 4 + col] * A.verts[o + col], 0));
    });
    const pole = scene.grips.units['1'];
    await page.evaluate(() => window.jumbleViewer.selectPiece(7));
    for (const proj of ['persp', 'stereo']) {
      await page.evaluate((proj) => window.jumbleViewer.set({ proj, cell: 0.78, sticker: 0.86 }), proj);
      const markers = await page.evaluate(() => window.jumbleViewer.globalCertificate());
      await page.evaluate(() => window.jumbleViewer.set({ cell: 1, sticker: 1 }));
      const unshrunk = await page.evaluate(() => window.jumbleViewer.globalCertificate());
      const h = markers ? markers.world.map((x) => x.reduce((sum, v, i) => sum + v * pole[i], 0) - 121 / 125) : [];
      check(markers && unshrunk && h[0] < 0 && h[1] > 0
        && Math.abs(h[0] - A.cert_h_below[r7]) < 1e-6 && Math.abs(h[1] - A.cert_h_above[r7]) < 1e-6
        && markers.world.every((x, i) => x.every((v, j) => Math.abs(v - expectedWorld[i][j]) < 1e-9))
        && markers.projected.every((v, i) => Math.abs(v - unshrunk.projected[i]) < 1e-6),
        `S1, grip 1, piece 7 (${proj}): Global certificate markers keep their signed h and posed vertex positions at default shrink`);
    }
    await page.evaluate((id) => { window.jumbleViewer.set({ proj: 'persp', cell: 0.78, sticker: 0.86 }); window.jumbleViewer.selectPiece(id); }, att.piece_id);
  }
  if (shots) await page.screenshot({ path: join(HERE, 'shots', '2-after-g.png'), fullPage: true, clip: await clipOf(['main.views', '.rail']) });

  // shot 4: Local view after T_d, grip c blocked, with its certificate points
  await page.evaluate(() => { window.jumbleViewer.goto(2); window.jumbleViewer.selectGrip(0); });
  const tags = await page.locator('#lOverlay .tag.below, #lOverlay .tag.above').count();
  check(tags === 2, 'S2, grip c: the Local view labels the two certificate points');
  cov = await page.evaluate(() => window.jumbleViewer.coverage());
  check(cov.local > 0.01, `S2, grip c: Local view draws the straddling piece (${(cov.local * 100).toFixed(1)}%)`);
  if (shots) await page.locator('section.view').nth(1).screenshot({ path: join(HERE, 'shots', '4-local.png') });

  // piece mode with the lattice ghost, then back
  await page.evaluate(() => { window.jumbleViewer.clearGrip(); });
  cov = await page.evaluate(() => window.jumbleViewer.coverage());
  check(cov.local > 0.01, `S2, no grip: Local view draws the focus piece and its lattice ghost (${(cov.local * 100).toFixed(1)}%)`);
  await page.evaluate(() => window.jumbleViewer.set({ proj: 'stereo' }));
  cov = await page.evaluate(() => window.jumbleViewer.coverage());
  check(cov.global > 0.02, `stereographic projection draws (${(cov.global * 100).toFixed(1)}%)`);
  // linking: a grip marker clicked in Global drives the Local view; the pager moves the focus.
  // The views redraw on the next animation frame, so wait for the Local caption to change.
  const settled = async (contains, differsFrom) => {
    try {
      await page.waitForFunction(([c, d]) => {
        const s = document.getElementById('lWhat').innerText;
        return (c == null || s.includes(c)) && (d == null || s !== d);
      }, [contains, differsFrom], { timeout: 5000 });
    } catch { /* the check below reports the failure */ }
    return page.locator('#lWhat').innerText();
  };
  await page.evaluate(() => window.jumbleViewer.set({ proj: 'persp' }));
  const at = await page.evaluate(() => window.jumbleViewer.gripScreen(13));
  await page.mouse.click(at.x, at.y);
  const picked = await page.evaluate(() => window.jumbleViewer.view.grip);
  const lwhat = await settled('grip 13');
  check(picked === 13 && lwhat.includes('grip 13'), `clicking the marker of grip d in Global selects it and the Local view follows (${picked})`);
  await page.evaluate(() => window.jumbleViewer.selectGrip(0));
  const before = await page.evaluate(() => window.jumbleViewer.view.piece);
  const lwhat1 = await page.locator('#lWhat').innerText();
  await page.locator('#pager button').last().click();
  const after = await page.evaluate(() => window.jumbleViewer.view.piece);
  const lwhat2 = await settled(null, lwhat1);
  check(after !== before && lwhat2 !== lwhat1, 'the straddling-piece pager moves the focus piece in both views');

  // play advances along the sequence and pause holds the position
  await page.evaluate(() => window.jumbleViewer.goto(0));
  await page.locator('#bPlay').click();
  await page.waitForTimeout(700);
  await page.locator('#bPlay').click();
  const t1 = await page.evaluate(() => window.jumbleViewer.view.t);
  await page.waitForTimeout(300);
  const t2 = await page.evaluate(() => window.jumbleViewer.view.t);
  check(t1 > 0 && t1 === t2, `play moves along the twist family and pause holds it (t = ${t1.toFixed(3)})`);
  check(errors.length === 0, `no console errors on desktop (${errors.length})${errors.length ? ': ' + errors.slice(0, 3).join(' | ') : ''}`);
  await context.close();

  // shot 3: dark theme, after T_d with blocked grips
  const dark = await openPage({ width: 1280, height: 860 }, 'dark', '#s2-g0');
  const bg = await dark.page.evaluate(() => getComputedStyle(document.body).backgroundColor);
  check(bg === 'rgb(18, 22, 27)', `dark theme applies the dark body background (${bg})`);
  const counts2 = await dark.page.locator('#gripCounts').innerText();
  check(counts2.includes('65'), 'deep link #s2-g0 opens S2 with 65 blocked grips');
  if (shots) await dark.page.screenshot({ path: join(HERE, 'shots', '3-after-td-blocked-dark.png'), fullPage: true, clip: await (async () => {
    const boxes = [];
    for (const s of ['main.views', '.rail', '#gripPanel']) boxes.push(await dark.page.locator(s).first().boundingBox());
    const x = Math.min(...boxes.map((b) => b.x)), y = Math.min(...boxes.map((b) => b.y));
    return { x, y, width: Math.max(...boxes.map((b) => b.x + b.width)) - x, height: Math.max(...boxes.map((b) => b.y + b.height)) - y };
  })() });
  check(dark.errors.length === 0, `no console errors in dark theme (${dark.errors.length})`);
  await dark.context.close();

  // phone width
  const phone = await openPage({ width: 390, height: 844 }, 'light');
  const widths = await phone.page.evaluate(() => [document.documentElement.scrollWidth, document.documentElement.clientWidth]);
  check(widths[0] <= widths[1], `no horizontal scroll at 390 px (scroll width ${widths[0]}, client width ${widths[1]})`);
  const pcov = await phone.page.evaluate(() => window.jumbleViewer.coverage());
  check(pcov.global > 0.02 && pcov.local > 0.01, 'both views draw at phone width');
  check(phone.errors.length === 0, `no console errors at phone width (${phone.errors.length})`);
  await phone.context.close();

  for (const hash of ['#s2-g47', '#s2-g600']) {
    const linked = await openPage({ width: 1280, height: 860 }, 'light', hash);
    const state = await linked.page.evaluate(() => ({ ready: window.jumbleViewer.ready,
      t: window.jumbleViewer.view && window.jumbleViewer.view.t, grip: window.jumbleViewer.view && window.jumbleViewer.view.grip,
      errors: window.jumbleViewer.errors }));
    check(state.ready && state.t === 2 && state.grip === null && state.errors.length === 0 && linked.errors.length === 0,
      `deep link ${hash} ignores a grip without an exported patch pole and loads without errors`);
    await linked.context.close();
  }

  const base64Header = { ...scene, bin: { ...scene.bin, file: 'scene-base64.txt', encoding: 'base64' } };
  const encoded = await openPage({ width: 1280, height: 860 }, 'light', '', { header: base64Header, body: sceneBytes.toString('base64') });
  check(await encoded.page.evaluate(() => window.jumbleViewer.ready && window.jumbleViewer.errors.length === 0)
    && encoded.errors.length === 0, 'the base64 scene path verifies the decoded bytes and loads the model');
  await encoded.context.close();
  const corrupt = Buffer.from(sceneBytes);
  corrupt[0] ^= 1;   // same byte count, different SHA-256
  for (const encoding of ['binary', 'base64']) {
    const bad = await openPage({ width: 1280, height: 860 }, 'light', '', {
      header: encoding === 'base64' ? base64Header : scene,
      body: encoding === 'base64' ? corrupt.toString('base64') : corrupt,
    });
    const state = await bad.page.evaluate(() => ({ ready: window.jumbleViewer.ready, model: window.jumbleViewer.model,
      error: document.getElementById('error').textContent, badge: document.getElementById('gBadge').textContent }));
    check(!state.ready && !state.model && state.badge === 'no data' && state.error.includes('SHA-256 digest does not match'),
      `the ${encoding} scene path rejects a digest mismatch before building a model`);
    await bad.context.close();
  }

  check(refused.length === 0, `no requests to hosts outside the allowlist${refused.length ? ': ' + refused.join(', ') : ''}`);
  await browser.close();
  server.close();
  console.log(failures.length ? `\n${failures.length} check(s) failed` : '\nall checks passed');
  process.exit(failures.length ? 1 : 0);
}

main().catch((err) => { console.error(err); process.exit(2); });

// Headless check of the J2 jumbling viewer with Playwright (Chromium, software WebGL).
//
//   node research/jumbling/viewer/check_viewer.mjs [--vendor DIR] [--shots]
//   node research/jumbling/viewer/check_viewer.mjs --serve [--port 8600]
//
// It serves this directory over local HTTP, wrapping index.html in the same minimal document
// skeleton that the private-page host adds at publish time, loads the page in headless Chromium
// with SwiftShader WebGL, and asserts: no console errors or page errors, both views draw
// non-background pixels, the grip counts of every exact state match each scene, the animated
// twist families end on the exported poses, and the page has no horizontal scroll at phone width.
// With --shots it writes four small PNG screenshots per scene to shots/.
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
  const { chromium } = await loadPlaywright();
  const browser = await chromium.launch({ args: ['--use-angle=swiftshader', '--enable-unsafe-swiftshader', '--ignore-gpu-blocklist'] });
  const refused = [];
  const shots = has('--shots');
  if (shots) mkdirSync(join(HERE, 'shots'), { recursive: true });
  const sceneCheck = check;

  for (const [id, file, prefix] of [['witness', 'scene.json', ''], ['s4', 'scene-s4.json', 's4-']]) {
    const check = (ok, what) => sceneCheck(ok, `${file}: ${what}`);
    const link = (state, grip = null, piece = null) => `#${prefix}s${state}`
      + (grip == null ? '' : `-g${grip}`) + (piece == null ? '' : `-p${piece}`);
    const shot = (name) => join(HERE, 'shots', prefix + name);
    const scene = JSON.parse(readFileSync(join(HERE, file), 'utf8'));
    const sceneBytes = readFileSync(join(HERE, scene.bin.file));
    const buffer = Uint8Array.from(sceneBytes).buffer;
    const typed = { float32: Float32Array, uint8: Uint8Array, uint16: Uint16Array, uint32: Uint32Array, int32: Int32Array };
    const A = Object.fromEntries(Object.entries(scene.bin.arrays).map(([name, d]) => [name, new typed[d.dtype](buffer, d.offset, d.length)]));
    check(sceneBytes.length === scene.bin.bytes && createHash('sha256').update(sceneBytes).digest('hex') === scene.bin.sha256,
      'the exported binary has the header byte count and SHA-256');
    check(Array.from(A.cert_h_below).every((h) => h < 0) && Array.from(A.cert_h_above).every((h) => h > 0),
      'every binary certificate row has a negative below h and a positive above h');
    if (id === 's4') {
      check(Object.values(scene.checks).every((v) => v === true)
        && scene.journal.menu_identity === scene.menu.identity && scene.menu.name === 'S4',
        'J1 header checks pass and the journal records the S4 menu identity');
      check(scene.states.every((st) => st.survey.length === 600
        && st.survey.filter((r) => r.status === 'admissible').length === st.admissible
        && st.survey.filter((r) => r.status === 'blocked').length === st.blocked
        && st.survey.map((r) => r.status === 'blocked' ? 'b' : 'a').join('') === st.grip_status),
        'every exact state matches its J1 survey');
    }

    async function openPage(viewport, colorScheme, hash = (prefix ? link(2, scene.c) : ''), data = null) {
      const context = await browser.newContext({ viewport, colorScheme, deviceScaleFactor: 1 });
      const page = await context.newPage();
      const errors = [];
      page.on('console', (m) => { if (m.type() === 'error') errors.push(m.text()); });
      page.on('pageerror', (e) => errors.push(String(e)));
      await page.route('**/*', (route) => {
        const u = route.request().url();
        if (data && u === base + file) return route.fulfill({ status: 200, contentType: 'application/json', body: JSON.stringify(data.header) });
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

    // desktop, light
    const { page, context, errors } = await openPage({ width: 1280, height: 860 }, 'light');
    const ready = await page.evaluate((id) => window.jumbleViewer.ready && window.jumbleViewer.scene === id, id);
    check(ready, 'viewer loads the selected header and binary and reports ready');
    check((await page.locator('#sceneSel option').allTextContents()).join('|') === 'Witness E2–E4|J1: S4 sequence'
      && await page.locator('#sceneSel').inputValue() === id, 'scene selector names both scenes and selects the loaded one');
    const model = await page.evaluate(() => window.jumbleViewer.model);
    check(model.pieces === scene.counts.pieces && model.stickers === scene.counts.stickers
      && model.states === scene.states.length && model.triangles === scene.counts.triangles,
      `model has ${model.pieces} pieces and ${model.stickers} stickers, as the header says`);
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
      check(counts.includes(`${st.admissible} admissible`) && counts.includes(`${st.blocked} blocked`),
        `S${k}: panel shows ${st.admissible} admissible and ${st.blocked} blocked grips`);
      const badge = await page.locator('#gBadge').innerText();
      check(badge.includes('exact'), `S${k}: views are labelled as an exact state`);
    }
    await page.evaluate(() => window.jumbleViewer.goto(1.5));
    const pbadge = await page.locator('#gBadge').innerText();
    check(pbadge.includes('uncertified'), 'between states the views are labelled as an uncertified float preview');

    // Keep the witness's piece-1035 regression; choose the same kind of moving piece in J1.
    const movingIndex = id === 'witness' ? A.piece_ids.indexOf(1035) : A.piece_ids.findIndex((p, i) =>
      A.state_pose[A.piece_ids.length + i] !== A.state_pose[2 * A.piece_ids.length + i]
      && A.state_lattice[A.piece_ids.length + i] === 1 && A.state_lattice[2 * A.piece_ids.length + i] === 1);
    check(movingIndex >= 0, 'the retained turn has a moving piece with lattice poses at both endpoints');
    const movingId = A.piece_ids[movingIndex];
    await page.evaluate((id) => { window.jumbleViewer.goto(1); window.jumbleViewer.clearGrip(); window.jumbleViewer.selectPiece(id); }, movingId);
    const latticeBefore = (await page.locator('#lHint').innerText()).includes('On the lattice');
    await page.evaluate(() => window.jumbleViewer.goto(1.5));
    const movingInfo = await page.locator('#pieceInfo').innerText();
    const movingHint = await page.locator('#lHint').innerText();
    const movingCaption = await page.locator('#lWhat').innerText();
    check(movingInfo.includes('moving (float preview)') && movingHint.includes('Moving (float preview)')
      && !/on the lattice|sits in a slot|at its lattice slot/i.test([movingInfo, movingHint, movingCaption].join(' ')),
      `S1.5, piece ${movingId}: retained-turn motion is a float preview without a lattice or slot claim`);
    await page.evaluate(() => window.jumbleViewer.goto(2));
    check(latticeBefore && (await page.locator('#lHint').innerText()).includes('On the lattice'),
      `piece ${movingId} keeps its lattice status at both exact endpoints of the retained turn`);

    // shot 1: solved
    await page.evaluate(() => { window.jumbleViewer.goto(0); window.jumbleViewer.clearGrip(); });
    let cov = await page.evaluate(() => window.jumbleViewer.coverage());
    check(cov.global > 0.02, `S0: Global view draws the patch (${(cov.global * 100).toFixed(1)}% of pixels)`);
    if (shots) await page.screenshot({ path: shot('1-solved.png'), fullPage: true, clip: await clipOf(['main.views', '.rail']) });

    // The rejected attempt occurs at S1 in the witness and S2 in the J1 sequence.
    const attempt = scene.attempts[0], att = attempt.certificate;
    await page.evaluate((a) => { window.jumbleViewer.goto(a.state); window.jumbleViewer.selectGrip(a.grip); window.jumbleViewer.selectPiece(a.certificate.piece_id); }, attempt);
    cov = await page.evaluate(() => window.jumbleViewer.coverage());
    check(cov.global > 0.02 && cov.local > 0.01, `S${attempt.state}, grip ${attempt.grip}: both views draw (${(cov.global * 100).toFixed(1)}%, ${(cov.local * 100).toFixed(1)}%)`);
    const cert1 = await page.locator('#pieceInfo dt:has-text("Certificate") + dd').innerText();
    check(certificateMatches(cert1, att),
      'the Certificate row matches both vertex identities and signed h values of the exact source');
    check((await page.locator('#gripInfo').innerText()).includes('twist rejected; configuration unchanged')
      && !attempt.applied && attempt.configuration_unchanged, 'the certified blocked attempt is shown as rejected without changing the state');
    check(!certificateMatches(cert1.replace('h = −', 'h = +'), att)
      && !certificateMatches(cert1.replace('h = +', 'h = −'), att),
      'the certificate assertion rejects a reversed below sign or above sign');

    // Witness piece 7 keeps the shrink regression. J1 uses its rejected attempt's certificate.
    const markerState = id === 'witness' ? 1 : attempt.state;
    const markerGrip = id === 'witness' ? 1 : attempt.grip;
    const markerId = id === 'witness' ? 7 : att.piece_id;
    const markerPiece = A.piece_ids.indexOf(markerId);
    let markerRow = -1;
    for (let r = A.cert_start[markerState]; r < A.cert_start[markerState + 1]; r++) {
      if (A.cert_grip[r] === markerGrip && A.cert_piece[r] === markerPiece) { markerRow = r; break; }
    }
    check(markerRow >= 0, `scene exports the S${markerState}, grip ${markerGrip}, piece ${markerId} marker certificate`);
    if (markerRow >= 0) {
      const mat = scene.poses[A.state_pose[markerState * A.piece_ids.length + markerPiece]].m;
      const expectedWorld = [A.cert_vertex_below[markerRow], A.cert_vertex_above[markerRow]].map((j) => {
        const o = (A.vert_start[markerPiece] + j) * 4;
        return [0, 1, 2, 3].map((row) => [0, 1, 2, 3].reduce((sum, col) => sum + mat[row * 4 + col] * A.verts[o + col], 0));
      });
      const pole = scene.grips.units[String(markerGrip)];
      await page.evaluate(([k, g, p]) => { window.jumbleViewer.goto(k); window.jumbleViewer.selectGrip(g); window.jumbleViewer.selectPiece(p); },
        [markerState, markerGrip, markerId]);
      for (const proj of ['persp', 'stereo']) {
        await page.evaluate((proj) => window.jumbleViewer.set({ proj, cell: 0.78, sticker: 0.86 }), proj);
        const markers = await page.evaluate(() => window.jumbleViewer.globalCertificate());
        const local = await page.evaluate(() => window.jumbleViewer.localCertificate());
        await page.evaluate(() => window.jumbleViewer.set({ cell: 1, sticker: 1 }));
        const unshrunk = await page.evaluate(() => window.jumbleViewer.globalCertificate());
        const h = markers ? markers.world.map((x) => x.reduce((sum, v, i) => sum + v * pole[i], 0) - 121 / 125) : [];
        check(markers && unshrunk && h[0] < 0 && h[1] > 0
          && Math.abs(h[0] - A.cert_h_below[markerRow]) < 1e-6 && Math.abs(h[1] - A.cert_h_above[markerRow]) < 1e-6
          && markers.world.every((x, i) => x.every((v, j) => Math.abs(v - expectedWorld[i][j]) < 1e-9))
          && markers.projected.every((v, i) => Math.abs(v - unshrunk.projected[i]) < 1e-6),
          `S${markerState}, grip ${markerGrip}, piece ${markerId} (${proj}): Global certificate markers keep their signed h and posed vertex positions at default shrink`);
        check(local && local[1] < 0 && local[4] > 0, `${proj}: Local certificate markers keep the sign of h`);
      }
      await page.evaluate((a) => { window.jumbleViewer.set({ proj: 'persp', cell: 0.78, sticker: 0.86 }); window.jumbleViewer.goto(a.state); window.jumbleViewer.selectGrip(a.grip); window.jumbleViewer.selectPiece(a.certificate.piece_id); }, attempt);
    }
    if (shots) await page.screenshot({ path: shot('2-after-g.png'), fullPage: true, clip: await clipOf(['main.views', '.rail']) });

    // Local view at S2, with a certified blocked grip in the selected sequence.
    const focusGrip = scene.states[2].grip_status[scene.c] === 'b' ? scene.c : attempt.grip;
    await page.evaluate((g) => { window.jumbleViewer.goto(2); window.jumbleViewer.selectGrip(g); }, focusGrip);
    const tags = await page.locator('#lOverlay .tag.below, #lOverlay .tag.above').count();
    check(tags === 2, `S2, grip ${focusGrip}: the Local view labels the two certificate points`);
    cov = await page.evaluate(() => window.jumbleViewer.coverage());
    check(cov.local > 0.01, `S2, grip ${focusGrip}: Local view draws the straddling piece (${(cov.local * 100).toFixed(1)}%)`);
    if (shots) await page.locator('section.view').nth(1).screenshot({ path: shot('4-local.png') });

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
    const at = await page.evaluate((d) => window.jumbleViewer.gripScreen(d), scene.d);
    await page.mouse.click(at.x, at.y);
    const picked = await page.evaluate(() => window.jumbleViewer.view.grip);
    const lwhat = await settled(`grip ${scene.d}`);
    check(picked === scene.d && lwhat.includes(`grip ${scene.d}`), `clicking the marker of grip d in Global selects it and the Local view follows (${picked})`);
    await page.evaluate((g) => window.jumbleViewer.selectGrip(g), focusGrip);
    const before = await page.evaluate(() => window.jumbleViewer.view.piece);
    const lwhat1 = await page.locator('#lWhat').innerText();
    await page.locator('#pager button').last().click();
    const after = await page.evaluate(() => window.jumbleViewer.view.piece);
    const certCount = Array.from(A.cert_grip.slice(A.cert_start[2], A.cert_start[3])).filter((g) => g === focusGrip).length;
    const lwhat2 = certCount > 1 ? await settled(null, lwhat1) : await page.locator('#lWhat').innerText();
    check(certCount > 1 ? after !== before && lwhat2 !== lwhat1 : after === before,
      'the straddling-piece pager cycles through the exported certificates');

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
    const other = id === 'witness' ? 's4' : 'witness';
    const otherHeader = JSON.parse(readFileSync(join(HERE, other === 's4' ? 'scene-s4.json' : 'scene.json'), 'utf8'));
    await page.locator('#sceneSel').selectOption(other);
    await page.waitForFunction((id) => window.jumbleViewer?.ready && window.jumbleViewer.scene === id, other);
    check(await page.evaluate(([id, counts]) => window.jumbleViewer.scene === id
      && window.jumbleViewer.model.pieces === counts.pieces && window.jumbleViewer.model.stickers === counts.stickers
      && window.jumbleViewer.errors.length === 0 && /^#[A-Za-z0-9-]+$/.test(location.hash), [other, otherHeader.counts]),
      'selecting the other scene loads its model and keeps a bare anchor token');
    await context.close();

    // shot 3: dark theme, after T_d with blocked grips
    const darkHash = link(2, scene.c);
    const dark = await openPage({ width: 1280, height: 860 }, 'dark', darkHash);
    const bg = await dark.page.evaluate(() => getComputedStyle(document.body).backgroundColor);
    check(bg === 'rgb(18, 22, 27)', `dark theme applies the dark body background (${bg})`);
    const counts2 = await dark.page.locator('#gripCounts').innerText();
    check(counts2.includes(`${scene.states[2].blocked} blocked`)
      && await dark.page.evaluate((id) => window.jumbleViewer.scene === id && window.jumbleViewer.view.t === 2, id),
      `deep link ${darkHash} opens the correct scene at S2 with ${scene.states[2].blocked} blocked grips`);
    if (shots) await dark.page.screenshot({ path: shot('3-after-td-blocked-dark.png'), fullPage: true, clip: await (async () => {
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

    const omittedGrip = Array.from({ length: 600 }, (_, e) => e).find((e) => !scene.grips.patch.includes(e));
    for (const hash of [link(2, omittedGrip), link(2, 600)]) {
      const linked = await openPage({ width: 1280, height: 860 }, 'light', hash);
      const state = await linked.page.evaluate(() => ({ ready: window.jumbleViewer.ready,
        t: window.jumbleViewer.view && window.jumbleViewer.view.t, grip: window.jumbleViewer.view && window.jumbleViewer.view.grip,
        scene: window.jumbleViewer.scene, errors: window.jumbleViewer.errors }));
      check(state.ready && state.scene === id && state.t === 2 && state.grip === null && state.errors.length === 0 && linked.errors.length === 0,
        `deep link ${hash} ignores a grip without an exported patch pole and loads without errors`);
      await linked.context.close();
    }

    const focusHash = link(markerState, markerGrip, markerId);
    const focused = await openPage({ width: 1280, height: 860 }, 'light', focusHash);
    check(await focused.page.evaluate(([id, k, g, p]) => window.jumbleViewer.scene === id
      && window.jumbleViewer.view.t === k && window.jumbleViewer.view.grip === g && window.jumbleViewer.view.piece === p,
    [id, markerState, markerGrip, markerPiece]), `deep link ${focusHash} restores scene, state, grip and focus piece`);
    await focused.context.close();

    const base64Header = { ...scene, bin: { ...scene.bin, file: 'scene-base64.txt', encoding: 'base64' } };
    const encoded = await openPage({ width: 1280, height: 860 }, 'light', link(0), { header: base64Header, body: sceneBytes.toString('base64') });
    check(await encoded.page.evaluate(() => window.jumbleViewer.ready && window.jumbleViewer.errors.length === 0)
      && encoded.errors.length === 0, 'the base64 scene path verifies the decoded bytes and loads the model');
    await encoded.context.close();
    const corrupt = Buffer.from(sceneBytes);
    corrupt[0] ^= 1;   // same byte count, different SHA-256
    for (const encoding of ['binary', 'base64']) {
      const bad = await openPage({ width: 1280, height: 860 }, 'light', link(0), {
        header: encoding === 'base64' ? base64Header : scene,
        body: encoding === 'base64' ? corrupt.toString('base64') : corrupt,
      });
      const state = await bad.page.evaluate(() => ({ ready: window.jumbleViewer.ready, model: window.jumbleViewer.model,
        error: document.getElementById('error').textContent, badge: document.getElementById('gBadge').textContent }));
      check(!state.ready && !state.model && state.badge === 'no data' && state.error.includes('SHA-256 digest does not match'),
        `the ${encoding} scene path rejects a digest mismatch before building a model`);
      await bad.context.close();
    }
  }

  check(refused.length === 0, `no requests to hosts outside the allowlist${refused.length ? ': ' + refused.join(', ') : ''}`);
  await browser.close();
  server.close();
  console.log(failures.length ? `\n${failures.length} check(s) failed` : '\nall checks passed');
  process.exit(failures.length ? 1 : 0);
}

main().catch((err) => { console.error(err); process.exit(2); });

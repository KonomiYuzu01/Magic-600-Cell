// Offline, headless research pictures. No native-rendering or performance claim.
// node capture_tutorial.mjs --data DIR --list shots.json --out DIR --vendor DIR [--check]
import { createServer } from 'node:http';
import { createHash } from 'node:crypto';
import { readFileSync, writeFileSync, mkdirSync, mkdtempSync, rmSync, realpathSync } from 'node:fs';
import { dirname, join, resolve, relative, isAbsolute, sep } from 'node:path';
import { tmpdir } from 'node:os';
import { fileURLToPath } from 'node:url';
import { createRequire } from 'node:module';
import { execFileSync } from 'node:child_process';

const HERE = dirname(fileURLToPath(import.meta.url));
const ROOT = resolve(HERE, '../../..');
const CDN = 'https://cdn.jsdelivr.net/npm/three@0.160.0/';
// Same immutable three.js 0.160.0 pins as check_full.mjs. Never fetch modules.
const PINNED = {
  'build/three.module.js': ['three.module.js', '76dea8151bc9352aef3528b4262e249b2604f62543828328db978d060d61a495'],
  'examples/jsm/controls/OrbitControls.js': ['OrbitControls.js', '5a44a9e86a2a0fb11933eed69bc2cd33c76a496854c1aed6ed776efa87d7b064'],
};
const NAME = /^[A-Za-z0-9_-]{1,64}$/;
const sha = (body) => createHash('sha256').update(body).digest('hex');
const json = (body) => JSON.parse(body.toString('utf8'));
const require = createRequire(import.meta.url);
function insist(ok, message) { if (!ok) throw new Error(message); }

function options() {
  const result = {};
  const args = process.argv.slice(2);
  for (let i = 0; i < args.length; i++) {
    const key = args[i];
    insist(['--data', '--list', '--out', '--vendor', '--check'].includes(key) && !(key in result), `Unknown or repeated option: ${key}`);
    if (key === '--check') result[key] = true;
    else {
      insist(args[i + 1] && !args[i + 1].startsWith('--'), `${key} needs a value`);
      result[key] = resolve(args[++i]);
    }
  }
  for (const key of ['--data', '--list', '--out']) insist(result[key], `${key} is required`);
  return result;
}

function shotsList(path) {
  const shots = json(readFileSync(path));
  insist(Array.isArray(shots) && shots.length > 0, 'The shot list must be a non-empty array');
  const names = new Set();
  return shots.map((s) => {
    insist(s && typeof s === 'object' && !Array.isArray(s)
      && Object.keys(s).every((k) => ['name', 'state', 'view', 'proj', 'emph', 'cap', 'w', 'h'].includes(k)), 'Invalid shot entry');
    insist(typeof s.name === 'string' && NAME.test(s.name) && !names.has(s.name), 'Invalid or duplicate shot name');
    insist(typeof s.state === 'string' && NAME.test(s.state), `Invalid state for ${s.name}`);
    names.add(s.name);
    const shot = { view: 'global', proj: 'persp', emph: 'moved', w: 1600, h: 1000, ...s };
    insist(['global', 'local'].includes(shot.view) && ['persp', 'stereo'].includes(shot.proj)
      && ['moved', 'off', 'none'].includes(shot.emph), `Invalid view, proj or emph for ${shot.name}`);
    insist(!('cap' in shot) || (Number.isInteger(shot.cap) && shot.cap >= 0 && shot.cap < 600),
      `Invalid cap for ${shot.name} (0..599)`);
    insist(['w', 'h'].every((k) => Number.isInteger(shot[k]) && shot[k] > 0 && shot[k] <= 8192),
      `Invalid dimensions for ${shot.name} (1..8192)`);
    return shot;
  });
}

function insideFile(base, name) {
  const path = realpathSync(join(base, name));
  const rel = relative(base, path);
  insist(rel && !isAbsolute(rel) && rel !== '..' && !rel.startsWith(`..${sep}`), `Data file escapes its directory: ${name}`);
  return readFileSync(path);
}

function vendorFiles(dir) {
  if (!dir) {
    try { dir = resolve(dirname(require.resolve('three')), '..'); }
    catch { throw new Error('Pinned three.js files are not installed; supply --vendor DIR (no downloads are made)'); }
  }
  const files = new Map();
  for (const [path, [name, digest]] of Object.entries(PINNED)) {
    let body;
    try { body = readFileSync(join(dir, name)); }
    catch { body = readFileSync(join(dir, path)); }
    insist(sha(body) === digest, `${name}: pinned three.js 0.160.0 digest mismatch`);
    files.set(`/vendor/${path}`, { body, type: 'text/javascript; charset=utf-8' });
  }
  return files;
}

function geometryFiles(files) {
  // The same retained bytes consumed by export_full.py, exposed at the page's
  // existing full/ URLs. No fixture export or repository writes are needed.
  const manifestRaw = readFileSync(join(ROOT, 'assets/manifest.json'));
  const manifest = json(manifestRaw);
  const specs = {
    'mesh_vertices.f32': ['<f4', [30480, 4]], 'mesh_sticker.u32': ['<u4', [30480]],
    'mesh_centers.f32': ['<f4', [433, 4]], 'cell_frames.f32': ['<f4', [600, 4, 4]],
    'slot_piece.u32': ['<u4', [259800]], 'mesh.json': null,
  };
  const descriptors = {}, identity = {};
  for (const name of ['model.npz', 'primitives.npz', ...Object.keys(specs)]) {
    const body = readFileSync(join(ROOT, 'assets', name)), digest = sha(body);
    insist(digest === manifest.files[`assets/${name}`], `assets/${name}: manifest SHA-256 mismatch`);
    if (!(name in specs)) { identity[`assets/${name}`] = digest; continue; }
    const desc = { sha256: digest, bytes: body.length };
    if (specs[name]) {
      const [dtype, shape] = specs[name];
      insist(body.length === 4 * shape.reduce((a, b) => a * b, 1), `${name}: invalid geometry shape`);
      Object.assign(desc, { dtype, shape });
    }
    descriptors[name] = desc;
    files.set(`/full/${name}`, { body, type: name.endsWith('.json') ? 'application/json' : 'application/octet-stream' });
  }
  const header = { format: 'magic600-jumbling-geometry/1', dimension: 4,
    base_vertices: 30480, base_stickers: 433, cells: 600, slots: 259800, pieces: 177120,
    model_identity: identity, retained_model_id: manifest.model_id, manifest_sha256: sha(manifestRaw), files: descriptors };
  files.set('/full/geometry.json', { body: Buffer.from(JSON.stringify(header)), type: 'application/json' });
}

function dataFiles(files, base, shots) {
  const states = new Map();
  for (const { state } of shots) {
    if (states.has(state)) continue;
    const tutorialRaw = insideFile(base, `${state}/tutorial.json`);
    const headerRaw = insideFile(base, `${state}/header.json`);
    const tutorial = json(tutorialRaw), header = json(headerRaw);
    insist(tutorial.format === 'magic600-tutorial-shot/1' && tutorial.digest === header.digest,
      `${state}: tutorial and header digest mismatch`);
    const names = ['header.json', 'tutorial.json', 'pose_index.i32', 'poses.f32', 'pose_lattice.i32', 'moved.i32'];
    for (const name of names) {
      const body = name === 'header.json' ? headerRaw : name === 'tutorial.json' ? tutorialRaw : insideFile(base, `${state}/${name}`);
      if (!name.endsWith('.json')) {
        const desc = name === 'moved.i32' ? tutorial.moved : header.files?.[name];
        insist(desc && sha(body) === desc.sha256, `${state}/${name}: SHA-256 mismatch`);
      }
      files.set(`/tutorial/${state}/${name}`, { body, type: name.endsWith('.json') ? 'application/json' : 'application/octet-stream' });
    }
    states.set(state, { header_digest: header.digest, tutorial_sha256: sha(tutorialRaw) });
  }
  for (const shot of shots) {
    const tutorial = json(files.get(`/tutorial/${shot.state}/tutorial.json`).body);
    shot.cap ??= tutorial.caps[0] ?? null;
    insist(shot.view !== 'local' || shot.cap !== null, `${shot.name}: local view needs a cap`);
  }
  return states;
}

async function loadPlaywright() {
  // Respect the repository's preinstalled browser bundle; this never installs.
  process.env.PLAYWRIGHT_BROWSERS_PATH ??= '/opt/pw-browsers';
  try { return { api: require('playwright'), version: require('playwright/package.json').version }; }
  catch {
    const root = execFileSync('npm', ['root', '-g'], { encoding: 'utf8' }).trim();
    const globalRequire = createRequire(join(root, 'noop.js'));
    return { api: globalRequire('playwright'), version: globalRequire('playwright/package.json').version };
  }
}

async function serve(files, forbidden) {
  const server = createServer((req, res) => {
    // Match raw request paths, without URL normalization that could hide traversal.
    const path = req.url.startsWith('/full.html?') ? '/full.html' : req.url;
    const file = files.get(path);
    if (req.method !== 'GET' || !file) {
      forbidden.push(`Unexpected request: ${req.method} ${req.url}`);
      res.writeHead(403); res.end('request refused'); return;
    }
    res.writeHead(200, { 'content-type': file.type, 'cache-control': 'no-store' });
    res.end(file.body);
  });
  await new Promise((resolveListen, reject) => {
    server.once('error', reject);
    server.listen(0, '127.0.0.1', resolveListen);
  });
  return server;
}

async function render(browser, base, files, forbidden, shots, states, directory) {
  mkdirSync(directory, { recursive: true });
  const results = [];
  for (const shot of shots) {
    const context = await browser.newContext({ viewport: { width: shot.w, height: shot.h }, deviceScaleFactor: 1,
      colorScheme: 'light', reducedMotion: 'reduce', locale: 'en-US', timezoneId: 'UTC', serviceWorkers: 'block' });
    try {
      const page = await context.newPage(), errors = [];
      page.setDefaultTimeout(180000);
      page.on('pageerror', (e) => errors.push(e.message));
      page.on('console', (m) => { if (m.type() === 'error') errors.push(m.text()); });
      await page.route('**/*', (route) => {
        const url = new URL(route.request().url());
        if (url.origin === base.slice(0, -1) && files.has(url.pathname) && !url.hash
          && (url.pathname === '/full.html' || !url.search)) return route.continue();
        forbidden.push(`Unexpected browser request: ${url.href}`);
        return route.abort();
      });
      const query = new URLSearchParams({ shot: shot.state, view: shot.view, proj: shot.proj,
        emph: shot.emph, w: String(shot.w), h: String(shot.h) });
      if (shot.cap !== null) query.set('cap', String(shot.cap));
      await page.goto(`${base}full.html?${query}`, { waitUntil: 'load' });
      await page.waitForFunction(() => ['1', 'error'].includes(document.documentElement.dataset.shotReady));
      const ready = await page.evaluate(() => ({ ...document.documentElement.dataset }));
      insist(ready.shotReady === '1', `${shot.name}: ${ready.shotError || 'drawing failed'}`);
      insist(!errors.length, `${shot.name}: browser errors: ${errors.join('; ')}`);
      insist(!forbidden.length, forbidden.join('\n'));
      const png = await page.locator('#canvas').screenshot({ type: 'png', animations: 'disabled' });
      writeFileSync(join(directory, `${shot.name}.png`), png);
      results.push({ ...shot, ...states.get(shot.state), png_sha256: sha(png) });
      console.log(`captured ${shot.name} (${shot.w} x ${shot.h})`);
    } finally { await context.close(); }
  }
  return results;
}

async function main() {
  const args = options(), shots = shotsList(args['--list']);
  const files = vendorFiles(args['--vendor']);
  const html = readFileSync(join(HERE, 'full.html')), js = readFileSync(join(HERE, 'full.js'));
  const servedHtml = html.toString('utf8').replace(`${CDN}build/three.module.js`, '/vendor/build/three.module.js')
    .replace(`${CDN}examples/jsm/`, '/vendor/examples/jsm/').replace('</head>', '<link rel="icon" href="data:,">\n</head>');
  files.set('/full.html', { body: Buffer.from(servedHtml), type: 'text/html; charset=utf-8' });
  files.set('/full.js', { body: js, type: 'text/javascript; charset=utf-8' });
  geometryFiles(files);
  const states = dataFiles(files, realpathSync(args['--data']), shots);
  const sources = { 'full.html': sha(html), 'full.js': sha(js),
    ...Object.fromEntries(Object.values(PINNED).map(([name, digest]) => [name, digest])) };
  const baseline = args['--check'] ? json(readFileSync(join(args['--out'], 'manifest.json'))) : null;
  if (baseline) insist(baseline.format === 'magic600-tutorial-capture/1' && Array.isArray(baseline.shots), 'Invalid capture manifest');
  const forbidden = [];
  const server = await serve(files, forbidden);
  const base = `http://127.0.0.1:${server.address().port}/`;
  let browser, temporary;
  try {
    const playwright = await loadPlaywright();
    browser = await playwright.api.chromium.launch({ headless: true,
      args: ['--use-angle=swiftshader', '--enable-unsafe-swiftshader', '--ignore-gpu-blocklist', '--disable-background-networking'] });
    const directory = baseline ? (temporary = mkdtempSync(join(tmpdir(), 'magic600-shot-check-'))) : args['--out'];
    const results = await render(browser, base, files, forbidden, shots, states, directory);
    insist(!forbidden.length, forbidden.join('\n'));
    if (baseline) {
      const different = results.filter((s) => {
        const previous = baseline.shots.find((p) => p.name === s.name);
        return !previous || Object.keys(s).some((k) => previous[k] !== s[k]);
      }).map((s) => s.name);
      for (const s of baseline.shots) if (!results.some((r) => r.name === s.name)) different.push(s.name);
      if (different.length) { console.error(`Shots differ: ${different.join(', ')}`); process.exitCode = 1; }
      else console.log(`check: ${results.length} PNG hashes and shot inputs match the manifest`);
    } else {
      writeFileSync(join(directory, 'manifest.json'), JSON.stringify({ format: 'magic600-tutorial-capture/1',
        versions: { node: process.version, playwright: playwright.version, chromium: browser.version() },
        sources, shots: results }, null, 1) + '\n');
    }
  } finally {
    if (browser) await browser.close();
    await new Promise((resolveClose) => server.close(resolveClose));
    if (temporary) rmSync(temporary, { recursive: true, force: true });
  }
}
main().catch((e) => { console.error(`tutorial capture: FAIL (${e.message})`); process.exitCode = 1; });

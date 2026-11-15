// LX-5 experiment: Node BigInt, built-in I/O and SHA-256 only.
import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';

const abs = a => a < 0n ? -a : a;
const sign = a => a < 0n ? -1n : a > 0n ? 1n : 0n;
function gcd(a, b) {
  a = abs(a); b = abs(b);
  while (b) [a, b] = [b, a % b];
  return a;
}
function floorDiv(a, b) {
  const q = a / b, r = a % b;
  return r && sign(a) !== sign(b) ? q - 1n : q;
}
function rat(a, d) {
  if (!d) throw Error('zero denominator');
  const g = gcd(a, d) * sign(d);
  return [a / g, d / g];
}
function q5(a, b, d) {
  if (!d) throw Error('zero denominator');
  const g = gcd(gcd(a, b), d) * sign(d);
  return [a / g, b / g, d / g];
}
function qsign([a, b]) {
  if (!a) return sign(b);
  if (!b || sign(a) === sign(b)) return sign(a);
  return sign(a) * sign(a * a - 5n * b * b);
}
function word(file) {
  const hash = crypto.createHash('sha256');
  const counts = new Uint32Array(65536), block = Buffer.alloc(65536);
  let length = 0n, total = 0n;
  const fd = fs.openSync(file, 'r');
  try {
    for (;;) {
      const n = fs.readSync(fd, block, 0, block.length, null);
      if (!n) break;
      if (n % 2) throw Error('odd ia16 bytes');
      hash.update(block.subarray(0, n));
      let subtotal = 0; // At most 32768 * 65535: exact as a Number.
      for (let i = 0; i < n; i += 2) {
        const letter = block.readUInt16LE(i);
        if (counts[letter] === 0xffffffff) throw Error('counter overflow');
        counts[letter]++; subtotal += letter;
      }
      length += BigInt(n / 2); total += BigInt(subtotal);
    }
  } finally { fs.closeSync(fd); }
  const histogram = [];
  for (let i = 0; i < counts.length; i++) if (counts[i]) histogram.push(`${i}:${counts[i]}`);
  return `${length}|${total}|${hash.digest('hex')}|${histogram.join(',')}`;
}

function evaluate(c, wordDir) {
  const x = c.x, a = x[0], b = x[1];
  switch (c.op) {
    case 'add': return String(a + b);
    case 'sub': return String(a - b);
    case 'mul': return String(a * b);
    case 'div': return String(floorDiv(a, b));
    case 'rem': return String(a - floorDiv(a, b) * b);
    case 'gcd': return String(gcd(a, b));
    case 'cmp': return String(sign(a - b));
    case 'shl': return String(a << b);
    case 'shr': return String(a >> b);
    case 'bits': return String(a ? abs(a).toString(2).length : 0);
    case 'dec': return String(BigInt(a.toString(10)));
    case 'hex': {
      const hex = abs(a).toString(16);
      return (a < 0n ? '-' : '') + hex + '|' + String(BigInt('0x' + hex) * (a < 0n ? -1n : 1n));
    }
    case 'sum': return String(x.reduce((s, v) => s + v, 0n));
    case 'rat_add': return rat(a * x[3] + x[2] * b, b * x[3]).join(',');
    case 'rat_mul': return rat(a * x[2], b * x[3]).join(',');
    case 'rat_cmp': {
      const [p, d] = rat(a, b), [q, e] = rat(x[2], x[3]);
      return String(sign(p * e - q * d));
    }
    case 'q_add': case 'q_mul': case 'q_conj': case 'q_norm': case 'q_sign': {
      const [p, q, d] = q5(...x.slice(0, 3));
      if (c.op === 'q_conj') return [p, -q, d].join(',');
      if (c.op === 'q_norm') return rat(p * p - 5n * q * q, d * d).join(',');
      if (c.op === 'q_sign') return String(qsign([p, q]));
      const [r, s, e] = q5(...x.slice(3));
      return (c.op === 'q_add' ? q5(p * e + r * d, q * e + s * d, d * e)
        : q5(p * r + 5n * q * s, p * s + q * r, d * e)).join(',');
    }
    case 'word': return word(path.join(wordDir, c.file));
    default: throw Error('unknown operation');
  }
}

const [input, wordDir, output, mode = 'run', sampleArg = '3', seriesArg = '1'] = process.argv.slice(2);
const values = new Map(), cases = [];
for (const line of fs.readFileSync(input, 'ascii').trimEnd().split('\n')) {
  const f = line.split('\t');
  if (f[0] === 'value') values.set(f[1], BigInt(f[2]));
  else {
    const [, id, size, op, measured, ...args] = f;
    cases.push({id, size, op, measured: measured === 'm',
      x: op === 'word' ? [] : args.map(k => values.get(k)), file: args[0]});
  }
}
const selected = mode === 'bench' ? cases.filter(c => c.measured) : cases;
const answers = new Map(), timings = [];
if (mode === 'bench') {
  for (let series = 0; series < Number(seriesArg); series++) {
    for (const c of selected) {
      const warm = evaluate(c, wordDir);
      for (let sample = 0; sample < Number(sampleArg); sample++) {
        const start = process.hrtime.bigint();
        const result = evaluate(c, wordDir);
        const elapsed = process.hrtime.bigint() - start;
        if (result !== warm) throw Error('nonrepeatable result: ' + c.id);
        answers.set(c.id, result);
        timings.push(`${c.id}\t${series}\t${sample}\t${elapsed}\n`);
      }
    }
  }
  fs.writeFileSync(output + '.timings.tsv', timings.join(''), 'ascii');
} else {
  for (const c of selected) answers.set(c.id, evaluate(c, wordDir));
}
fs.writeFileSync(output, selected.map(c => c.id + '\t' + answers.get(c.id) + '\n').join(''), 'ascii');

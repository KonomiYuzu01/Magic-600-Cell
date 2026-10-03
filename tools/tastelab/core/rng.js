// Mulberry32 and a cached Box–Muller draw. No ambient randomness is used.
export function createRng(seed) {
  if (!Number.isSafeInteger(seed)) throw new TypeError('seed must be an integer');
  let state = seed >>> 0;
  let spare;
  function next() {
    state = (state + 0x6d2b79f5) >>> 0;
    let t = state;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  }
  return {
    next,
    normal() {
      if (spare !== undefined) {
        const value = spare;
        spare = undefined;
        return value;
      }
      const r = Math.sqrt(-2 * Math.log(1 - next()));
      const angle = 2 * Math.PI * next();
      spare = r * Math.sin(angle);
      return r * Math.cos(angle);
    },
    int(n) {
      if (!Number.isInteger(n) || n < 1 || n > 4294967296) {
        throw new RangeError('n must be in 1..2^32');
      }
      // Rejection avoids bias for bounds that do not divide 2^32.
      const limit = 4294967296 - (4294967296 % n);
      let value;
      do value = Math.floor(next() * 4294967296); while (value >= limit);
      return value % n;
    },
  };
}

// Independent comparisons call the original Taste Lab functions, without rewriting them.
import {pathToFileURL} from 'node:url';
import {resolve} from 'node:path';
const root = process.argv[2];
const load = file => import(pathToFileURL(resolve(root, file)).href);
const colour = await load('tools/tastelab/core/color.js');
const space = await load('tools/tastelab/core/space.js');
const {ease} = await load('tools/tastelab/page/preview.js');
const {properColouring} = await load('tools/tastelab/page/geometry.js');
let text = '';
for await (const chunk of process.stdin) text += chunk;
const input = JSON.parse(text);
let output;
if (input.task === 'colouring') {
  output = Array.from({length: 5}, (_, i) => Array.from(properColouring(20, input.pairs, i + 4)));
} else if (input.task === 'easing') {
  output = input.values.map(([t, a, b]) => ease(t, a, b));
} else if (input.task === 'hard-check') {
  output = space.hardCheck(input.look, input.pairs, {deltaE: 0, bgL: 0});
} else if (input.task === 'colour') {
  output = {
    conversions: input.rgb.map(rgb => ({lab: colour.linearToOklab(rgb), roundTrip: colour.oklabToLinear(colour.linearToOklab(rgb)),
      cvd: Object.fromEntries(colour.CVD_KINDS.map(mode => [mode, colour.simulateCvd(rgb, mode)]))})),
    mapped: input.lch.map(([L, C, h]) => colour.mapToGamut(L, C, h)),
    reports: input.looks.map((look, index) => {
      const palette = space.palette(look).map(c => c.mapped), bg = space.background(look), pairs = input.pairs[index];
      const modes = Object.fromEntries(['normal', ...colour.CVD_KINDS].map(mode => {
        const labs = palette.map(c => mode === 'normal' ? c.lab : colour.linearToOklab(colour.simulateCvd(c.linear, mode)));
        const background = mode === 'normal' ? bg.lab : colour.linearToOklab(colour.simulateCvd(bg.linear, mode));
        return [mode, {distance: Math.min(...pairs.map(([a,b]) => colour.deltaE(labs[a], labs[b]))),
          background: Math.min(...labs.map(lab => Math.abs(lab[0] - background[0])))}];
      }));
      return {palette, bg, modes, hardCheck: space.hardCheck(look, pairs, {deltaE: 0, bgL: 0})};
    }),
    ranges: space.defaultSpace().params,
  };
} else throw new Error('unknown oracle task');
process.stdout.write(JSON.stringify(output));

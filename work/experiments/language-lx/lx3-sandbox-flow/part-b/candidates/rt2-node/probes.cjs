/* Safe, candidate-local probes of the UNCHECKED vm, outside corpus counts.
 * Escapes read only the public Node version, never files, environment or network.
 */
'use strict';
const vm = require('node:vm');
const {context} = require('./runtime.cjs');
const probes = [
  ['host_function_constructor', 'tick.constructor.constructor("return process.version")()'],
  ['this_constructor', 'this.constructor.constructor("return typeof process")()'],
  ['host_function_prototype_walk', 'Object.getPrototypeOf(tick).constructor("return process.version")()'],
  ['global_prototype_walk', 'Object.getPrototypeOf(this).constructor.constructor("return typeof process")()'],
  ['object_constructor', '({}).constructor.constructor("return 7")()'],
  ['direct_eval', 'eval("1+1")'],
  ['direct_process', 'typeof process'],
  ['direct_require', 'typeof require'],
  ['ambient_date_intrinsic', 'typeof Date'],
  ['ambient_random_intrinsic', 'typeof Math.random'],
  ['no_host_call_loop', 'while (true) {}'],
];
const results = probes.map(([id, source]) => {
  const ctx = context({functions: ['tick'], items: {}, inputs: {}, run: 'r1'}, () => null);
  try {
    const value = new vm.Script(source).runInContext(ctx, {timeout: 100});
    return {id, source, runtime: 'returned', value};
  } catch (error) {
    return {id, source, runtime: error.code === 'ERR_SCRIPT_EXECUTION_TIMEOUT' ? 'timeout' : error.name};
  }
});
process.stdout.write(JSON.stringify(results) + '\n');

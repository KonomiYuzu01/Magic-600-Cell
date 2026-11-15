/* Experiment only. vm is NOT treated as a security boundary. */
'use strict';
const fs = require('node:fs');
const vm = require('node:vm');
let pending = Buffer.alloc(0);

function readLine() {
  while (true) {
    const end = pending.indexOf(10);
    if (end >= 0) {
      const line = pending.subarray(0, end).toString('utf8');
      pending = pending.subarray(end + 1);
      return JSON.parse(line);
    }
    const chunk = Buffer.alloc(8192);
    const size = fs.readSync(0, chunk, 0, chunk.length, null);
    if (!size) return null; // Parent pipe closure also ends the Node process.
    pending = Buffer.concat([pending, chunk.subarray(0, size)]);
  }
}

function send(value) {
  fs.writeSync(1, JSON.stringify(value) + '\n');
}

function context(request, rpc) {
  const sandbox = Object.create(null);
  for (const name of request.functions) {
    sandbox[name] = (...args) => rpc(name, args);
  }
  sandbox.merge = (path, outer, local) => {
    for (const key of Object.keys(outer)) {
      if (key !== request.items[path]) outer[key] = local[key];
    }
  };
  sandbox.dispatch = runs => runs[request.run](
    Object.assign(Object.create(null), request.inputs), [], 0, null);
  // Native intrinsics remain. The checker excludes arbitrary property walks,
  // Date, random, eval, Function, imports and every token outside its templates.
  return vm.createContext(sandbox, {codeGeneration: {strings: false, wasm: false}});
}

if (require.main === module) {
  let request;
  while ((request = readLine()) !== null) {
    const stopped = Object.freeze({});
    const rpc = (name, args) => {
      send({event: 'call', name, args});
      const reply = readLine();
      if (reply === null) process.exit(0);
      if (reply.stop) throw stopped;
      return reply.value;
    };
    try {
      const result = new vm.Script(request.source, {filename: 'approved-subset.js'})
        .runInContext(context(request, rpc), {timeout: 2000});
      send({event: 'done', value: result === undefined ? null : result});
    } catch (error) {
      if (error === stopped) send({event: 'stopped'});
      else send({event: error.code === 'ERR_SCRIPT_EXECUTION_TIMEOUT' ? 'timeout' : 'runtime_error',
                 name: error.name || 'Error'});
    }
  }
}

module.exports = {context};

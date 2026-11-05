// Experiment-only Wasm host. All user values stay behind Python handles.
// fs is used solely for the fixed stdin/stdout protocol, never a program path.
"use strict";
const fs = require("node:fs");
let buffered = "";
function readLine() {
  for (;;) {
    const newline = buffered.indexOf("\n");
    if (newline >= 0) {
      const line = buffered.slice(0, newline);
      buffered = buffered.slice(newline + 1);
      return JSON.parse(line);
    }
    const chunk = Buffer.alloc(65536);
    const size = fs.readSync(0, chunk, 0, chunk.length, null);
    if (!size) throw new Error("closed bridge");
    buffered += chunk.toString("utf8", 0, size);
  }
}
function send(value) {
  const data = Buffer.from(JSON.stringify(value) + "\n", "utf8");
  let offset = 0;
  while (offset < data.length) offset += fs.writeSync(1, data, offset);
}
const names = ["input", "literal", "meta", "arg", "step", "meter", "perform",
  "truth", "length", "control", "loop_begin", "loop_test", "item", "loop_done",
  "done", "load", "call_enter", "call_exit"];
let instance;
function rpc(method, args) {
  send({method, args});
  for (;;) {
    const reply = readLine();
    if (Object.hasOwn(reply, "charge")) {
      instance.exports.charge(reply.charge);
      send({charged: reply.charge});
    } else {
      if (reply.stop) throw new Error("host stop");
      return reply.value;
    }
  }
}
try {
  const request = readLine();
  const module = new WebAssembly.Module(Buffer.from(request.wasm_hex, "hex"));
  const imports = WebAssembly.Module.imports(module);
  if (imports.length !== names.length || imports.some((item, i) =>
      item.module !== "host" || item.kind !== "function" || item.name !== names[i])) {
    throw new Error("unexpected authority");
  }
  const host = Object.fromEntries(names.map(method => [method, (...args) => {
    const result = rpc(method, args);
    return ["step", "meter", "loop_test"].includes(method) ? BigInt(result) : result;
  }]));
  instance = new WebAssembly.Instance(module, {host});
  const [handle, flags] = instance.exports.run(0, 0, -1);
  send({finished: true, handle, flags, fuel: instance.exports.fuel.value.toString()});
} catch (error) {
  // No paths, exception messages, program data, or machine diagnostics.
  send({failed: true, kind: error instanceof WebAssembly.CompileError ? "compile" : "execution"});
  process.exitCode = 1;
}

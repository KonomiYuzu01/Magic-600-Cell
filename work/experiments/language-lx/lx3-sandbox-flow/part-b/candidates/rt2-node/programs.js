"use strict";
// neutral-json {"inputs":{"caps":{"origin":"manual","type":"Targets","value":["c0","c1","c2"]},"target":{"origin":"manual","type":"Target","value":"c0"},"word":{"origin":"manual","type":"Word","value":["a"]},"words":{"origin":"manual","type":"Words","value":[["a"],["b"],["c"]]}},"modules":{},"runs":[{"author":"built_in","body":[{"args":[],"as":"n","op":"kernel","row":"counts","type":"Ints"},{"args":[{"ref":"n"}],"as":"i","fn":"argmax_first","op":"calc","type":"Int"},{"args":[{"ref":"caps"},{"ref":"i"}],"as":"t","fn":"index","op":"calc","type":"Target"},{"as":"receipt","op":"preview","target":{"ref":"t"},"type":"Receipt","word":{"ref":"word"}}],"boundary_class":"analysis","id":"r1","invocation":"command","profile":"solving"}]}
function r0(e, c, d, i) {
  {
    e["n"] = kernel("r1/body/0", e, c, d, i);
    complete("r1/body/0", d);
  }
  {
    e["i"] = calc("r1/body/1", e, c, d, i);
    complete("r1/body/1", d);
  }
  {
    e["t"] = calc("r1/body/2", e, c, d, i);
    complete("r1/body/2", d);
  }
  {
    e["receipt"] = preview("r1/body/3", e, c, d, i);
    complete("r1/body/3", d);
  }
}
dispatch({"r1":r0});
"use strict";
// neutral-json {"inputs":{"caps":{"origin":"manual","type":"Targets","value":["c0","c1","c2"]},"target":{"origin":"manual","type":"Target","value":"c0"},"word":{"origin":"manual","type":"Word","value":["a"]},"words":{"origin":"manual","type":"Words","value":[["a"],["b"],["c"]]}},"modules":{},"runs":[{"author":"built_in","body":[{"args":[],"as":"n","op":"kernel","row":"counts","type":"Ints"},{"args":[{"ref":"n"}],"as":"rank","fn":"rank_desc_indices","op":"calc","type":"Ints"},{"args":[{"ref":"rank"},{"type":"Int","value":0}],"as":"i","fn":"index","op":"calc","type":"Int"},{"args":[{"ref":"caps"},{"ref":"i"}],"as":"t","fn":"index","op":"calc","type":"Target"},{"as":"receipt","op":"preview","target":{"ref":"t"},"type":"Receipt","word":{"ref":"word"}}],"boundary_class":"analysis","id":"r1","invocation":"command","profile":"solving"}]}
function r0(e, c, d, i) {
  {
    e["n"] = kernel("r1/body/0", e, c, d, i);
    complete("r1/body/0", d);
  }
  {
    e["rank"] = calc("r1/body/1", e, c, d, i);
    complete("r1/body/1", d);
  }
  {
    e["i"] = calc("r1/body/2", e, c, d, i);
    complete("r1/body/2", d);
  }
  {
    e["t"] = calc("r1/body/3", e, c, d, i);
    complete("r1/body/3", d);
  }
  {
    e["receipt"] = preview("r1/body/4", e, c, d, i);
    complete("r1/body/4", d);
  }
}
dispatch({"r1":r0});
"use strict";
// neutral-json {"inputs":{"caps":{"origin":"manual","type":"Targets","value":["c0","c1","c2"]},"target":{"origin":"manual","type":"Target","value":"c0"},"word":{"origin":"manual","type":"Word","value":["a"]},"words":{"origin":"manual","type":"Words","value":[["a"],["b"],["c"]]}},"modules":{},"runs":[{"author":"built_in","body":[{"args":[{"type":"Ints","value":[]}],"as":"scores","fn":"identity","op":"calc","type":"Ints"},{"body":[{"args":[{"ref":"w"}],"as":"s","op":"kernel","row":"score","type":"Int"},{"args":[{"ref":"scores"},{"ref":"s"}],"as":"scores","fn":"append","op":"calc","type":"Ints"}],"item":"w","items":{"ref":"words"},"op":"for","type":"Word"},{"args":[{"ref":"scores"}],"as":"i","fn":"argmin_first","op":"calc","type":"Int"},{"args":[{"ref":"words"},{"ref":"i"}],"as":"chosen","fn":"index","op":"calc","type":"Word"},{"args":[{"type":"Word","value":[]},{"ref":"chosen"}],"as":"rebuilt","fn":"concat","op":"calc","type":"Word"},{"as":"receipt","op":"preview","target":{"ref":"target"},"type":"Receipt","word":{"ref":"rebuilt"}}],"boundary_class":"analysis","id":"r1","invocation":"command","profile":"solving"}]}
function r0(e, c, d, i) {
  {
    e["scores"] = calc("r1/body/0", e, c, d, i);
    complete("r1/body/0", d);
  }
  {
    const seq = sequence("r1/body/1", e, c, d, i);
    const outer = e;
    const previous = i;
    const parent = c;
    c = c.concat([seq]);
    for (i = 0; loop("r1/body/1", seq, i); i++) {
      e = Object.assign(Object.create(null), outer);
      e["w"] = item("r1/body/1", seq, c, i);
      {
        e["s"] = kernel("r1/body/1/body/0", e, c, d, i);
        complete("r1/body/1/body/0", d);
      }
      {
        e["scores"] = calc("r1/body/1/body/1", e, c, d, i);
        complete("r1/body/1/body/1", d);
      }
      merge("r1/body/1", outer, e);
      covered("r1/body/1", i);
    }
    e = outer;
    i = previous;
    c = parent;
    complete("r1/body/1", d);
  }
  {
    e["i"] = calc("r1/body/2", e, c, d, i);
    complete("r1/body/2", d);
  }
  {
    e["chosen"] = calc("r1/body/3", e, c, d, i);
    complete("r1/body/3", d);
  }
  {
    e["rebuilt"] = calc("r1/body/4", e, c, d, i);
    complete("r1/body/4", d);
  }
  {
    e["receipt"] = preview("r1/body/5", e, c, d, i);
    complete("r1/body/5", d);
  }
}
dispatch({"r1":r0});
"use strict";
// neutral-json {"inputs":{"caps":{"origin":"manual","type":"Targets","value":["c0","c1","c2"]},"target":{"origin":"manual","type":"Target","value":"c0"},"word":{"origin":"manual","type":"Word","value":["a"]},"words":{"origin":"manual","type":"Words","value":[["a"],["b"],["c"]]}},"modules":{},"runs":[{"author":"built_in","body":[{"args":[{"type":"Ints","value":[]}],"as":"scores","fn":"identity","op":"calc","type":"Ints"},{"body":[{"args":[{"ref":"w"}],"as":"s","op":"kernel","row":"score","type":"Int"},{"args":[{"ref":"scores"},{"ref":"s"}],"as":"scores","fn":"append","op":"calc","type":"Ints"}],"item":"w","items":{"ref":"words"},"op":"for","type":"Word"},{"args":[{"ref":"scores"}],"as":"i","fn":"argmin_first","op":"calc","type":"Int"},{"args":[{"ref":"words"},{"ref":"i"}],"as":"chosen","fn":"index","op":"calc","type":"Word"},{"key":"selected_index","op":"workspace_put","value":{"ref":"i"}}],"boundary_class":"analysis","id":"r1","invocation":"command","profile":"research"},{"author":"built_in","body":[{"as":"i","key":"selected_index","op":"workspace_get","type":"Int"},{"args":[{"ref":"words"},{"ref":"i"}],"as":"rebuilt","fn":"index","op":"calc","type":"Word"},{"as":"receipt","op":"preview","target":{"ref":"target"},"type":"Receipt","word":{"ref":"rebuilt"}}],"boundary_class":"analysis","id":"r2","invocation":"command","profile":"solving"}]}
function r0(e, c, d, i) {
  {
    e["scores"] = calc("r1/body/0", e, c, d, i);
    complete("r1/body/0", d);
  }
  {
    const seq = sequence("r1/body/1", e, c, d, i);
    const outer = e;
    const previous = i;
    const parent = c;
    c = c.concat([seq]);
    for (i = 0; loop("r1/body/1", seq, i); i++) {
      e = Object.assign(Object.create(null), outer);
      e["w"] = item("r1/body/1", seq, c, i);
      {
        e["s"] = kernel("r1/body/1/body/0", e, c, d, i);
        complete("r1/body/1/body/0", d);
      }
      {
        e["scores"] = calc("r1/body/1/body/1", e, c, d, i);
        complete("r1/body/1/body/1", d);
      }
      merge("r1/body/1", outer, e);
      covered("r1/body/1", i);
    }
    e = outer;
    i = previous;
    c = parent;
    complete("r1/body/1", d);
  }
  {
    e["i"] = calc("r1/body/2", e, c, d, i);
    complete("r1/body/2", d);
  }
  {
    e["chosen"] = calc("r1/body/3", e, c, d, i);
    complete("r1/body/3", d);
  }
  {
    workspace_put("r1/body/4", e, c, d, i);
    complete("r1/body/4", d);
  }
}
function r1(e, c, d, i) {
  {
    e["i"] = workspace_get("r2/body/0", e, c, d, i);
    complete("r2/body/0", d);
  }
  {
    e["rebuilt"] = calc("r2/body/1", e, c, d, i);
    complete("r2/body/1", d);
  }
  {
    e["receipt"] = preview("r2/body/2", e, c, d, i);
    complete("r2/body/2", d);
  }
}
dispatch({"r1":r0,"r2":r1});
"use strict";
// neutral-json {"inputs":{"target":{"origin":"manual","type":"Target","value":"c0"},"word":{"origin":"manual","type":"Word","value":["a"]}},"modules":{},"runs":[{"author":"built_in","body":[{"args":[],"as":"nu","op":"kernel","row":"nu2","type":"Ints"},{"args":[{"type":"Matrix","value":[[1,0],[0,1]]},{"ref":"nu"},{"type":"Int","value":2}],"as":"coeff","fn":"linear_solve_mod","op":"calc","type":"Ints"},{"args":[{"type":"Words","value":[["a"],["b"]]},{"ref":"coeff"}],"as":"correction","fn":"combine_corrections","op":"calc","type":"Word"},{"as":"receipt","op":"preview","target":{"ref":"target"},"type":"Receipt","word":{"ref":"correction"}}],"boundary_class":"analysis","id":"r1","invocation":"command","profile":"solving"}]}
function r0(e, c, d, i) {
  {
    e["nu"] = kernel("r1/body/0", e, c, d, i);
    complete("r1/body/0", d);
  }
  {
    e["coeff"] = calc("r1/body/1", e, c, d, i);
    complete("r1/body/1", d);
  }
  {
    e["correction"] = calc("r1/body/2", e, c, d, i);
    complete("r1/body/2", d);
  }
  {
    e["receipt"] = preview("r1/body/3", e, c, d, i);
    complete("r1/body/3", d);
  }
}
dispatch({"r1":r0});
"use strict";
// neutral-json {"inputs":{"target":{"origin":"manual","type":"Target","value":"c0"},"word":{"origin":"manual","type":"Word","value":["a"]}},"modules":{},"runs":[{"author":"built_in","body":[{"args":[],"as":"nu","op":"kernel","row":"nu5","type":"Ints"},{"args":[{"type":"Matrix","value":[[1,0],[0,1]]},{"ref":"nu"},{"type":"Int","value":5}],"as":"coeff","fn":"linear_solve_mod","op":"calc","type":"Ints"},{"args":[{"type":"Words","value":[["c"],["d"]]},{"ref":"coeff"}],"as":"correction","fn":"combine_corrections","op":"calc","type":"Word"},{"as":"receipt","op":"preview","target":{"ref":"target"},"type":"Receipt","word":{"ref":"correction"}}],"boundary_class":"analysis","id":"r1","invocation":"command","profile":"solving"}]}
function r0(e, c, d, i) {
  {
    e["nu"] = kernel("r1/body/0", e, c, d, i);
    complete("r1/body/0", d);
  }
  {
    e["coeff"] = calc("r1/body/1", e, c, d, i);
    complete("r1/body/1", d);
  }
  {
    e["correction"] = calc("r1/body/2", e, c, d, i);
    complete("r1/body/2", d);
  }
  {
    e["receipt"] = preview("r1/body/3", e, c, d, i);
    complete("r1/body/3", d);
  }
}
dispatch({"r1":r0});
"use strict";
// neutral-json {"inputs":{"caps":{"origin":"manual","type":"Targets","value":["c0","c1","c2"]},"target":{"origin":"manual","type":"Target","value":"c0"},"word":{"origin":"manual","type":"Word","value":["a"]},"words":{"origin":"manual","type":"Words","value":[["a"],["b"],["c"]]}},"modules":{},"runs":[{"author":"built_in","body":[{"args":[],"as":"n","op":"kernel","row":"counts","type":"Ints"},{"args":[{"ref":"n"}],"as":"i","fn":"argmax_first","op":"calc","type":"Int"},{"args":[{"ref":"words"},{"ref":"i"}],"as":"w","fn":"index","op":"calc","type":"Word"},{"key":"saved","op":"workspace_put","value":{"ref":"w"}}],"boundary_class":"analysis","id":"r1","invocation":"command","profile":"tool_authoring"},{"author":"built_in","body":[{"as":"w","key":"saved","op":"workspace_get","type":"Word"},{"args":[{"ref":"w"}],"as":"renamed","fn":"identity","op":"calc","type":"Word"},{"key":"renamed","op":"workspace_put","value":{"ref":"renamed"}},{"as":"receipt","op":"preview","target":{"ref":"target"},"type":"Receipt","word":{"ref":"renamed"}}],"boundary_class":"analysis","id":"r2","invocation":"command","profile":"solving"}]}
function r0(e, c, d, i) {
  {
    e["n"] = kernel("r1/body/0", e, c, d, i);
    complete("r1/body/0", d);
  }
  {
    e["i"] = calc("r1/body/1", e, c, d, i);
    complete("r1/body/1", d);
  }
  {
    e["w"] = calc("r1/body/2", e, c, d, i);
    complete("r1/body/2", d);
  }
  {
    workspace_put("r1/body/3", e, c, d, i);
    complete("r1/body/3", d);
  }
}
function r1(e, c, d, i) {
  {
    e["w"] = workspace_get("r2/body/0", e, c, d, i);
    complete("r2/body/0", d);
  }
  {
    e["renamed"] = calc("r2/body/1", e, c, d, i);
    complete("r2/body/1", d);
  }
  {
    workspace_put("r2/body/2", e, c, d, i);
    complete("r2/body/2", d);
  }
  {
    e["receipt"] = preview("r2/body/3", e, c, d, i);
    complete("r2/body/3", d);
  }
}
dispatch({"r1":r0,"r2":r1});
"use strict";
// neutral-json {"inputs":{"legacy":{"origin":"unknown","type":"Word","value":["a"]},"target":{"origin":"manual","type":"Target","value":"c0"}},"modules":{},"runs":[{"author":"built_in","body":[{"key":"legacy","op":"workspace_put","value":{"ref":"legacy"}}],"boundary_class":"analysis","id":"r1","invocation":"command","profile":"tool_authoring"},{"author":"built_in","body":[{"as":"w","key":"legacy","op":"workspace_get","type":"Word"},{"as":"receipt","op":"preview","target":{"ref":"target"},"type":"Receipt","word":{"ref":"w"}}],"boundary_class":"analysis","id":"r2","invocation":"command","profile":"solving"}]}
function r0(e, c, d, i) {
  {
    workspace_put("r1/body/0", e, c, d, i);
    complete("r1/body/0", d);
  }
}
function r1(e, c, d, i) {
  {
    e["w"] = workspace_get("r2/body/0", e, c, d, i);
    complete("r2/body/0", d);
  }
  {
    e["receipt"] = preview("r2/body/1", e, c, d, i);
    complete("r2/body/1", d);
  }
}
dispatch({"r1":r0,"r2":r1});
"use strict";
// neutral-json {"inputs":{"target":{"origin":"manual","type":"Target","value":"c0"},"word":{"origin":"manual","type":"Word","value":["a"]}},"modules":{},"runs":[{"author":"built_in","body":[{"args":[],"as":"n","op":"kernel","row":"counts","type":"Ints"},{"args":[{"ref":"n"},{"type":"Int","value":0}],"as":"n0","fn":"index","op":"calc","type":"Int"},{"args":[{"ref":"n0"},{"type":"Int","value":0}],"as":"test","fn":"gt","op":"calc","type":"Bool"},{"else":[{"as":"receipt","op":"preview","target":{"ref":"target"},"type":"Receipt","word":{"type":"Word","value":["b"]}}],"op":"if","test":{"ref":"test"},"then":[{"as":"receipt","op":"preview","target":{"ref":"target"},"type":"Receipt","word":{"type":"Word","value":["a"]}}]}],"boundary_class":"analysis","id":"r1","invocation":"command","profile":"solving"}]}
function r0(e, c, d, i) {
  {
    e["n"] = kernel("r1/body/0", e, c, d, i);
    complete("r1/body/0", d);
  }
  {
    e["n0"] = calc("r1/body/1", e, c, d, i);
    complete("r1/body/1", d);
  }
  {
    e["test"] = calc("r1/body/2", e, c, d, i);
    complete("r1/body/2", d);
  }
  {
    const t = branch("r1/body/3", e, c, d, i);
    const parent = c;
    c = c.concat([t.id]);
    if (t.truth) {
      {
        e["receipt"] = preview("r1/body/3/then/0", e, c, d, i);
        complete("r1/body/3/then/0", d);
      }
    } else {
      {
        e["receipt"] = preview("r1/body/3/else/0", e, c, d, i);
        complete("r1/body/3/else/0", d);
      }
    }
    c = parent;
    complete("r1/body/3", d);
  }
}
dispatch({"r1":r0});
"use strict";
// neutral-json {"inputs":{"target":{"origin":"manual","type":"Target","value":"c0"},"word":{"origin":"manual","type":"Word","value":["a"]}},"modules":{},"runs":[{"author":"built_in","body":[{"args":[],"as":"bits","op":"kernel","row":"bits","type":"Ints"},{"args":[{"ref":"bits"},{"type":"Int","value":1}],"as":"b","fn":"index","op":"calc","type":"Int"},{"args":[{"ref":"b"},{"type":"Int","value":1}],"as":"test","fn":"eq","op":"calc","type":"Bool"},{"else":[{"args":[{"type":"Word","value":["b"]}],"as":"choice","fn":"identity","op":"calc","type":"Word"}],"op":"if","test":{"ref":"test"},"then":[{"args":[{"type":"Word","value":["a"]}],"as":"choice","fn":"identity","op":"calc","type":"Word"}]},{"as":"receipt","op":"preview","target":{"ref":"target"},"type":"Receipt","word":{"ref":"choice"}}],"boundary_class":"analysis","id":"r1","invocation":"command","profile":"solving"}]}
function r0(e, c, d, i) {
  {
    e["bits"] = kernel("r1/body/0", e, c, d, i);
    complete("r1/body/0", d);
  }
  {
    e["b"] = calc("r1/body/1", e, c, d, i);
    complete("r1/body/1", d);
  }
  {
    e["test"] = calc("r1/body/2", e, c, d, i);
    complete("r1/body/2", d);
  }
  {
    const t = branch("r1/body/3", e, c, d, i);
    const parent = c;
    c = c.concat([t.id]);
    if (t.truth) {
      {
        e["choice"] = calc("r1/body/3/then/0", e, c, d, i);
        complete("r1/body/3/then/0", d);
      }
    } else {
      {
        e["choice"] = calc("r1/body/3/else/0", e, c, d, i);
        complete("r1/body/3/else/0", d);
      }
    }
    c = parent;
    complete("r1/body/3", d);
  }
  {
    e["receipt"] = preview("r1/body/4", e, c, d, i);
    complete("r1/body/4", d);
  }
}
dispatch({"r1":r0});
"use strict";
// neutral-json {"inputs":{"target":{"origin":"manual","type":"Target","value":"c0"},"word":{"origin":"manual","type":"Word","value":["a"]}},"modules":{"selector":{"body":[{"args":[],"as":"n","op":"kernel","row":"nu2","type":"Ints"},{"args":[{"ref":"n"},{"type":"Int","value":0}],"as":"v","fn":"index","op":"calc","type":"Int"},{"args":[{"ref":"v"},{"type":"Int","value":1}],"as":"b","fn":"eq","op":"calc","type":"Bool"},{"else":[{"op":"return","value":{"type":"Word","value":["b"]}}],"op":"if","test":{"ref":"b"},"then":[{"op":"return","value":{"type":"Word","value":["a"]}}]}],"parameters":{},"result":"Word"}},"runs":[{"author":"built_in","body":[{"args":[],"as":"w","module":"selector","op":"call","type":"Word"},{"as":"receipt","op":"preview","target":{"ref":"target"},"type":"Receipt","word":{"ref":"w"}}],"boundary_class":"analysis","id":"r1","invocation":"command","profile":"solving"}]}
function m0(e, c, d, i) {
  {
    e["n"] = kernel("modules/selector/body/0", e, c, d, i);
    complete("modules/selector/body/0", d);
  }
  {
    e["v"] = calc("modules/selector/body/1", e, c, d, i);
    complete("modules/selector/body/1", d);
  }
  {
    e["b"] = calc("modules/selector/body/2", e, c, d, i);
    complete("modules/selector/body/2", d);
  }
  {
    const t = branch("modules/selector/body/3", e, c, d, i);
    const parent = c;
    c = c.concat([t.id]);
    if (t.truth) {
      {
        return finish("modules/selector/body/3/then/0", e, c, d, i);
      }
    } else {
      {
        return finish("modules/selector/body/3/else/0", e, c, d, i);
      }
    }
    c = parent;
    complete("modules/selector/body/3", d);
  }
}
function r0(e, c, d, i) {
  {
    const local = enter("r1/body/0", e, c, d, i);
    const value = m0(local, c, d + 1, null);
    e["w"] = leave("r1/body/0", value, e, c);
    complete("r1/body/0", d);
  }
  {
    e["receipt"] = preview("r1/body/1", e, c, d, i);
    complete("r1/body/1", d);
  }
}
dispatch({"r1":r0});
"use strict";
// neutral-json {"inputs":{"target":{"origin":"manual","type":"Target","value":"c0"},"word":{"origin":"manual","type":"Word","value":["a"]}},"modules":{"copy":{"body":[{"op":"return","value":{"ref":"x"}}],"parameters":{"x":"Word"},"result":"Word"},"selector":{"body":[{"args":[],"as":"n","op":"kernel","row":"nu2","type":"Ints"},{"args":[{"ref":"n"},{"type":"Int","value":0}],"as":"v","fn":"index","op":"calc","type":"Int"},{"args":[{"ref":"v"},{"type":"Int","value":1}],"as":"b","fn":"eq","op":"calc","type":"Bool"},{"else":[{"op":"return","value":{"type":"Word","value":["b"]}}],"op":"if","test":{"ref":"b"},"then":[{"op":"return","value":{"type":"Word","value":["a"]}}]}],"parameters":{},"result":"Word"}},"runs":[{"author":"built_in","body":[{"args":[],"as":"selected","module":"selector","op":"call","type":"Word"},{"args":[{"ref":"selected"}],"as":"copied","module":"copy","op":"call","type":"Word"},{"as":"receipt","op":"preview","target":{"ref":"target"},"type":"Receipt","word":{"ref":"copied"}}],"boundary_class":"analysis","id":"r1","invocation":"command","profile":"solving"}]}
function m0(e, c, d, i) {
  {
    return finish("modules/copy/body/0", e, c, d, i);
  }
}
function m1(e, c, d, i) {
  {
    e["n"] = kernel("modules/selector/body/0", e, c, d, i);
    complete("modules/selector/body/0", d);
  }
  {
    e["v"] = calc("modules/selector/body/1", e, c, d, i);
    complete("modules/selector/body/1", d);
  }
  {
    e["b"] = calc("modules/selector/body/2", e, c, d, i);
    complete("modules/selector/body/2", d);
  }
  {
    const t = branch("modules/selector/body/3", e, c, d, i);
    const parent = c;
    c = c.concat([t.id]);
    if (t.truth) {
      {
        return finish("modules/selector/body/3/then/0", e, c, d, i);
      }
    } else {
      {
        return finish("modules/selector/body/3/else/0", e, c, d, i);
      }
    }
    c = parent;
    complete("modules/selector/body/3", d);
  }
}
function r0(e, c, d, i) {
  {
    const local = enter("r1/body/0", e, c, d, i);
    const value = m1(local, c, d + 1, null);
    e["selected"] = leave("r1/body/0", value, e, c);
    complete("r1/body/0", d);
  }
  {
    const local = enter("r1/body/1", e, c, d, i);
    const value = m0(local, c, d + 1, null);
    e["copied"] = leave("r1/body/1", value, e, c);
    complete("r1/body/1", d);
  }
  {
    e["receipt"] = preview("r1/body/2", e, c, d, i);
    complete("r1/body/2", d);
  }
}
dispatch({"r1":r0});
"use strict";
// neutral-json {"inputs":{"caps":{"origin":"manual","type":"Targets","value":["c0","c1","c2"]},"target":{"origin":"manual","type":"Target","value":"c0"},"word":{"origin":"manual","type":"Word","value":["a"]},"words":{"origin":"manual","type":"Words","value":[["a"],["b"],["c"]]}},"modules":{},"runs":[{"author":"built_in","body":[{"body":[{"as":"receipt","op":"preview","target":{"ref":"target"},"type":"Receipt","word":{"ref":"w"}},{"args":[{"ref":"w"}],"as":"s","op":"kernel","row":"score","type":"Int"},{"args":[{"ref":"s"},{"type":"Int","value":0}],"as":"done","fn":"eq","op":"calc","type":"Bool"},{"else":[],"op":"if","test":{"ref":"done"},"then":[{"op":"stop","reason":"requested"}]}],"item":"w","items":{"ref":"words"},"op":"for","type":"Word"}],"boundary_class":"analysis","id":"r1","invocation":"command","profile":"solving"}]}
function r0(e, c, d, i) {
  {
    const seq = sequence("r1/body/0", e, c, d, i);
    const outer = e;
    const previous = i;
    const parent = c;
    c = c.concat([seq]);
    for (i = 0; loop("r1/body/0", seq, i); i++) {
      e = Object.assign(Object.create(null), outer);
      e["w"] = item("r1/body/0", seq, c, i);
      {
        e["receipt"] = preview("r1/body/0/body/0", e, c, d, i);
        complete("r1/body/0/body/0", d);
      }
      {
        e["s"] = kernel("r1/body/0/body/1", e, c, d, i);
        complete("r1/body/0/body/1", d);
      }
      {
        e["done"] = calc("r1/body/0/body/2", e, c, d, i);
        complete("r1/body/0/body/2", d);
      }
      {
        const t = branch("r1/body/0/body/3", e, c, d, i);
        const parent = c;
        c = c.concat([t.id]);
        if (t.truth) {
          {
            stop("r1/body/0/body/3/then/0", e, c, d, i);
            complete("r1/body/0/body/3/then/0", d);
          }
        } else {
        }
        c = parent;
        complete("r1/body/0/body/3", d);
      }
      merge("r1/body/0", outer, e);
      covered("r1/body/0", i);
    }
    e = outer;
    i = previous;
    c = parent;
    complete("r1/body/0", d);
  }
}
dispatch({"r1":r0});
"use strict";
// neutral-json {"inputs":{"target":{"origin":"manual","type":"Target","value":"c0"},"word":{"origin":"manual","type":"Word","value":["a"]}},"modules":{},"runs":[{"author":"built_in","body":[{"args":[],"as":"bits","op":"kernel","row":"bits","type":"Ints"},{"args":[{"ref":"bits"},{"type":"Int","value":0}],"as":"b","fn":"index","op":"calc","type":"Int"},{"args":[{"ref":"b"},{"type":"Int","value":1}],"as":"test","fn":"eq","op":"calc","type":"Bool"},{"else":[],"op":"if","test":{"ref":"test"},"then":[{"op":"stop","reason":"requested"}]},{"as":"receipt","op":"preview","target":{"ref":"target"},"type":"Receipt","word":{"ref":"word"}}],"boundary_class":"analysis","id":"r1","invocation":"command","profile":"solving"}]}
function r0(e, c, d, i) {
  {
    e["bits"] = kernel("r1/body/0", e, c, d, i);
    complete("r1/body/0", d);
  }
  {
    e["b"] = calc("r1/body/1", e, c, d, i);
    complete("r1/body/1", d);
  }
  {
    e["test"] = calc("r1/body/2", e, c, d, i);
    complete("r1/body/2", d);
  }
  {
    const t = branch("r1/body/3", e, c, d, i);
    const parent = c;
    c = c.concat([t.id]);
    if (t.truth) {
      {
        stop("r1/body/3/then/0", e, c, d, i);
        complete("r1/body/3/then/0", d);
      }
    } else {
    }
    c = parent;
    complete("r1/body/3", d);
  }
  {
    e["receipt"] = preview("r1/body/4", e, c, d, i);
    complete("r1/body/4", d);
  }
}
dispatch({"r1":r0});
"use strict";
// neutral-json {"inputs":{"target":{"origin":"manual","type":"Target","value":"c0"},"word":{"origin":"manual","type":"Word","value":["a"]}},"modules":{},"runs":[{"author":"built_in","body":[{"as":"receipt","op":"preview","target":{"ref":"target"},"type":"Receipt","word":{"type":"Word","value":["d"]}},{"args":[{"ref":"receipt"}],"as":"outcome","op":"kernel","row":"preview_outcome","type":"Record"},{"args":[{"ref":"outcome"},{"type":"Text","value":"code"}],"as":"field","fn":"field","op":"calc","type":"Text"},{"args":[{"ref":"field"},{"type":"Text","value":"KN2:protection.piece_would_move"}],"as":"test","fn":"eq","op":"calc","type":"Bool"},{"else":[{"as":"receipt","op":"preview","target":{"ref":"target"},"type":"Receipt","word":{"type":"Word","value":["b"]}}],"op":"if","test":{"ref":"test"},"then":[{"as":"receipt","op":"preview","target":{"ref":"target"},"type":"Receipt","word":{"type":"Word","value":["a"]}}]}],"boundary_class":"analysis","id":"r1","invocation":"command","profile":"solving"}]}
function r0(e, c, d, i) {
  {
    e["receipt"] = preview("r1/body/0", e, c, d, i);
    complete("r1/body/0", d);
  }
  {
    e["outcome"] = kernel("r1/body/1", e, c, d, i);
    complete("r1/body/1", d);
  }
  {
    e["field"] = calc("r1/body/2", e, c, d, i);
    complete("r1/body/2", d);
  }
  {
    e["test"] = calc("r1/body/3", e, c, d, i);
    complete("r1/body/3", d);
  }
  {
    const t = branch("r1/body/4", e, c, d, i);
    const parent = c;
    c = c.concat([t.id]);
    if (t.truth) {
      {
        e["receipt"] = preview("r1/body/4/then/0", e, c, d, i);
        complete("r1/body/4/then/0", d);
      }
    } else {
      {
        e["receipt"] = preview("r1/body/4/else/0", e, c, d, i);
        complete("r1/body/4/else/0", d);
      }
    }
    c = parent;
    complete("r1/body/4", d);
  }
}
dispatch({"r1":r0});
"use strict";
// neutral-json {"inputs":{"target":{"origin":"manual","type":"Target","value":"c0"},"word":{"origin":"manual","type":"Word","value":["a"]}},"modules":{},"runs":[{"author":"built_in","body":[{"as":"receipt","op":"preview","target":{"ref":"target"},"type":"Receipt","word":{"type":"Word","value":["c"]}},{"args":[{"ref":"receipt"}],"as":"outcome","op":"kernel","row":"preview_outcome","type":"Record"},{"args":[{"ref":"outcome"},{"type":"Text","value":"protected"}],"as":"field","fn":"field","op":"calc","type":"Bool"},{"args":[{"ref":"field"},{"type":"Bool","value":false}],"as":"test","fn":"eq","op":"calc","type":"Bool"},{"else":[{"as":"receipt","op":"preview","target":{"ref":"target"},"type":"Receipt","word":{"type":"Word","value":["b"]}}],"op":"if","test":{"ref":"test"},"then":[{"as":"receipt","op":"preview","target":{"ref":"target"},"type":"Receipt","word":{"type":"Word","value":["a"]}}]}],"boundary_class":"analysis","id":"r1","invocation":"command","profile":"solving"}]}
function r0(e, c, d, i) {
  {
    e["receipt"] = preview("r1/body/0", e, c, d, i);
    complete("r1/body/0", d);
  }
  {
    e["outcome"] = kernel("r1/body/1", e, c, d, i);
    complete("r1/body/1", d);
  }
  {
    e["field"] = calc("r1/body/2", e, c, d, i);
    complete("r1/body/2", d);
  }
  {
    e["test"] = calc("r1/body/3", e, c, d, i);
    complete("r1/body/3", d);
  }
  {
    const t = branch("r1/body/4", e, c, d, i);
    const parent = c;
    c = c.concat([t.id]);
    if (t.truth) {
      {
        e["receipt"] = preview("r1/body/4/then/0", e, c, d, i);
        complete("r1/body/4/then/0", d);
      }
    } else {
      {
        e["receipt"] = preview("r1/body/4/else/0", e, c, d, i);
        complete("r1/body/4/else/0", d);
      }
    }
    c = parent;
    complete("r1/body/4", d);
  }
}
dispatch({"r1":r0});
"use strict";
// neutral-json {"inputs":{"caps":{"origin":"manual","type":"Targets","value":["c0","c1","c2"]},"pick0":{"origin":"manual","type":"Int","value":0},"target":{"origin":"manual","type":"Target","value":"c0"},"word":{"origin":"manual","type":"Word","value":["a"]},"words":{"origin":"manual","type":"Words","value":[["a"],["b"],["c"]]}},"modules":{},"runs":[{"author":"built_in","body":[{"args":[],"as":"n","op":"kernel","row":"counts","type":"Ints"},{"args":[{"ref":"n"}],"as":"i","fn":"argmax_first","op":"calc","type":"Int"},{"args":[{"ref":"caps"},{"ref":"i"}],"as":"t","fn":"index","op":"calc","type":"Target"},{"args":[{"ref":"t"}],"as":"one","fn":"singleton","op":"calc","type":"Targets"},{"as":"chosen","event":"pick0","op":"pick","options":{"ref":"one"},"type":"Target"},{"as":"receipt","op":"preview","target":{"ref":"chosen"},"type":"Receipt","word":{"ref":"word"}}],"boundary_class":"analysis","id":"r1","invocation":"command","profile":"solving"}]}
function r0(e, c, d, i) {
  {
    e["n"] = kernel("r1/body/0", e, c, d, i);
    complete("r1/body/0", d);
  }
  {
    e["i"] = calc("r1/body/1", e, c, d, i);
    complete("r1/body/1", d);
  }
  {
    e["t"] = calc("r1/body/2", e, c, d, i);
    complete("r1/body/2", d);
  }
  {
    e["one"] = calc("r1/body/3", e, c, d, i);
    complete("r1/body/3", d);
  }
  {
    e["chosen"] = pick("r1/body/4", e, c, d, i);
    complete("r1/body/4", d);
  }
  {
    e["receipt"] = preview("r1/body/5", e, c, d, i);
    complete("r1/body/5", d);
  }
}
dispatch({"r1":r0});
"use strict";
// neutral-json {"inputs":{"pick0":{"origin":"manual","type":"Int","value":0},"target":{"origin":"manual","type":"Target","value":"c0"}},"modules":{},"runs":[{"author":"built_in","body":[{"args":[],"as":"nu","op":"kernel","row":"nu2","type":"Ints"},{"args":[{"type":"Words","value":[["a"],["b"]]},{"ref":"nu"}],"as":"w","fn":"combine_corrections","op":"calc","type":"Word"},{"args":[{"ref":"w"}],"as":"one","fn":"singleton","op":"calc","type":"Words"},{"as":"chosen","event":"pick0","op":"pick","options":{"ref":"one"},"type":"Word"},{"as":"receipt","op":"preview","target":{"ref":"target"},"type":"Receipt","word":{"ref":"chosen"}}],"boundary_class":"analysis","id":"r1","invocation":"command","profile":"solving"}]}
function r0(e, c, d, i) {
  {
    e["nu"] = kernel("r1/body/0", e, c, d, i);
    complete("r1/body/0", d);
  }
  {
    e["w"] = calc("r1/body/1", e, c, d, i);
    complete("r1/body/1", d);
  }
  {
    e["one"] = calc("r1/body/2", e, c, d, i);
    complete("r1/body/2", d);
  }
  {
    e["chosen"] = pick("r1/body/3", e, c, d, i);
    complete("r1/body/3", d);
  }
  {
    e["receipt"] = preview("r1/body/4", e, c, d, i);
    complete("r1/body/4", d);
  }
}
dispatch({"r1":r0});
"use strict";
// neutral-json {"inputs":{"target":{"origin":"manual","type":"Target","value":"c0"},"word":{"origin":"manual","type":"Word","value":["a"]}},"modules":{},"runs":[{"author":"built_in","body":[{"as":"receipt","op":"preview","target":{"ref":"target"},"type":"Receipt","word":{"type":"Word","value":["a"]}},{"args":[],"as":"bits","op":"kernel","row":"bits","type":"Ints"},{"args":[{"ref":"bits"},{"type":"Int","value":0}],"as":"bit","fn":"index","op":"calc","type":"Int"},{"args":[{"ref":"bit"},{"type":"Int","value":1}],"as":"test","fn":"eq","op":"calc","type":"Bool"},{"else":[],"op":"if","test":{"ref":"test"},"then":[{"op":"stop","reason":"requested"}]},{"as":"receipt","op":"preview","target":{"ref":"target"},"type":"Receipt","word":{"type":"Word","value":["b"]}}],"boundary_class":"analysis","id":"r1","invocation":"command","profile":"solving"},{"author":"built_in","body":[{"as":"receipt","op":"preview","target":{"ref":"target"},"type":"Receipt","word":{"type":"Word","value":["a"]}},{"args":[],"as":"bits","op":"kernel","row":"bits","type":"Ints"},{"args":[{"ref":"bits"},{"type":"Int","value":1}],"as":"bit","fn":"index","op":"calc","type":"Int"},{"args":[{"ref":"bit"},{"type":"Int","value":1}],"as":"test","fn":"eq","op":"calc","type":"Bool"},{"else":[],"op":"if","test":{"ref":"test"},"then":[{"op":"stop","reason":"requested"}]},{"as":"receipt","op":"preview","target":{"ref":"target"},"type":"Receipt","word":{"type":"Word","value":["b"]}}],"boundary_class":"analysis","id":"r2","invocation":"command","profile":"solving"},{"author":"built_in","body":[{"as":"receipt","op":"preview","target":{"ref":"target"},"type":"Receipt","word":{"type":"Word","value":["a"]}},{"args":[],"as":"bits","op":"kernel","row":"bits","type":"Ints"},{"args":[{"ref":"bits"},{"type":"Int","value":2}],"as":"bit","fn":"index","op":"calc","type":"Int"},{"args":[{"ref":"bit"},{"type":"Int","value":1}],"as":"test","fn":"eq","op":"calc","type":"Bool"},{"else":[],"op":"if","test":{"ref":"test"},"then":[{"op":"stop","reason":"requested"}]},{"as":"receipt","op":"preview","target":{"ref":"target"},"type":"Receipt","word":{"type":"Word","value":["b"]}}],"boundary_class":"analysis","id":"r3","invocation":"command","profile":"solving"}]}
function r0(e, c, d, i) {
  {
    e["receipt"] = preview("r1/body/0", e, c, d, i);
    complete("r1/body/0", d);
  }
  {
    e["bits"] = kernel("r1/body/1", e, c, d, i);
    complete("r1/body/1", d);
  }
  {
    e["bit"] = calc("r1/body/2", e, c, d, i);
    complete("r1/body/2", d);
  }
  {
    e["test"] = calc("r1/body/3", e, c, d, i);
    complete("r1/body/3", d);
  }
  {
    const t = branch("r1/body/4", e, c, d, i);
    const parent = c;
    c = c.concat([t.id]);
    if (t.truth) {
      {
        stop("r1/body/4/then/0", e, c, d, i);
        complete("r1/body/4/then/0", d);
      }
    } else {
    }
    c = parent;
    complete("r1/body/4", d);
  }
  {
    e["receipt"] = preview("r1/body/5", e, c, d, i);
    complete("r1/body/5", d);
  }
}
function r1(e, c, d, i) {
  {
    e["receipt"] = preview("r2/body/0", e, c, d, i);
    complete("r2/body/0", d);
  }
  {
    e["bits"] = kernel("r2/body/1", e, c, d, i);
    complete("r2/body/1", d);
  }
  {
    e["bit"] = calc("r2/body/2", e, c, d, i);
    complete("r2/body/2", d);
  }
  {
    e["test"] = calc("r2/body/3", e, c, d, i);
    complete("r2/body/3", d);
  }
  {
    const t = branch("r2/body/4", e, c, d, i);
    const parent = c;
    c = c.concat([t.id]);
    if (t.truth) {
      {
        stop("r2/body/4/then/0", e, c, d, i);
        complete("r2/body/4/then/0", d);
      }
    } else {
    }
    c = parent;
    complete("r2/body/4", d);
  }
  {
    e["receipt"] = preview("r2/body/5", e, c, d, i);
    complete("r2/body/5", d);
  }
}
function r2(e, c, d, i) {
  {
    e["receipt"] = preview("r3/body/0", e, c, d, i);
    complete("r3/body/0", d);
  }
  {
    e["bits"] = kernel("r3/body/1", e, c, d, i);
    complete("r3/body/1", d);
  }
  {
    e["bit"] = calc("r3/body/2", e, c, d, i);
    complete("r3/body/2", d);
  }
  {
    e["test"] = calc("r3/body/3", e, c, d, i);
    complete("r3/body/3", d);
  }
  {
    const t = branch("r3/body/4", e, c, d, i);
    const parent = c;
    c = c.concat([t.id]);
    if (t.truth) {
      {
        stop("r3/body/4/then/0", e, c, d, i);
        complete("r3/body/4/then/0", d);
      }
    } else {
    }
    c = parent;
    complete("r3/body/4", d);
  }
  {
    e["receipt"] = preview("r3/body/5", e, c, d, i);
    complete("r3/body/5", d);
  }
}
dispatch({"r1":r0,"r2":r1,"r3":r2});
"use strict";
// neutral-json {"inputs":{"target":{"origin":"manual","type":"Target","value":"c0"},"word":{"origin":"manual","type":"Word","value":["a"]}},"modules":{},"runs":[{"author":"built_in","body":[{"args":[],"as":"bits","op":"kernel","row":"bits","type":"Ints"},{"args":[{"ref":"bits"},{"type":"Int","value":0}],"as":"bit","fn":"index","op":"calc","type":"Int"},{"args":[{"ref":"bit"},{"type":"Int","value":1}],"as":"test","fn":"eq","op":"calc","type":"Bool"},{"else":[],"op":"if","test":{"ref":"test"},"then":[{"op":"stop","reason":"requested"}]},{"as":"receipt","op":"preview","target":{"ref":"target"},"type":"Receipt","word":{"type":"Word","value":["b"]}}],"boundary_class":"analysis","id":"r1","invocation":"command","profile":"solving"},{"author":"built_in","body":[{"args":[],"as":"bits","op":"kernel","row":"bits","type":"Ints"},{"args":[{"ref":"bits"},{"type":"Int","value":1}],"as":"bit","fn":"index","op":"calc","type":"Int"},{"args":[{"ref":"bit"},{"type":"Int","value":1}],"as":"test","fn":"eq","op":"calc","type":"Bool"},{"else":[],"op":"if","test":{"ref":"test"},"then":[{"op":"stop","reason":"requested"}]},{"as":"receipt","op":"preview","target":{"ref":"target"},"type":"Receipt","word":{"type":"Word","value":["b"]}}],"boundary_class":"analysis","id":"r2","invocation":"command","profile":"solving"},{"author":"built_in","body":[{"args":[],"as":"bits","op":"kernel","row":"bits","type":"Ints"},{"args":[{"ref":"bits"},{"type":"Int","value":2}],"as":"bit","fn":"index","op":"calc","type":"Int"},{"args":[{"ref":"bit"},{"type":"Int","value":1}],"as":"test","fn":"eq","op":"calc","type":"Bool"},{"else":[],"op":"if","test":{"ref":"test"},"then":[{"op":"stop","reason":"requested"}]},{"as":"receipt","op":"preview","target":{"ref":"target"},"type":"Receipt","word":{"type":"Word","value":["b"]}}],"boundary_class":"analysis","id":"r3","invocation":"command","profile":"solving"}]}
function r0(e, c, d, i) {
  {
    e["bits"] = kernel("r1/body/0", e, c, d, i);
    complete("r1/body/0", d);
  }
  {
    e["bit"] = calc("r1/body/1", e, c, d, i);
    complete("r1/body/1", d);
  }
  {
    e["test"] = calc("r1/body/2", e, c, d, i);
    complete("r1/body/2", d);
  }
  {
    const t = branch("r1/body/3", e, c, d, i);
    const parent = c;
    c = c.concat([t.id]);
    if (t.truth) {
      {
        stop("r1/body/3/then/0", e, c, d, i);
        complete("r1/body/3/then/0", d);
      }
    } else {
    }
    c = parent;
    complete("r1/body/3", d);
  }
  {
    e["receipt"] = preview("r1/body/4", e, c, d, i);
    complete("r1/body/4", d);
  }
}
function r1(e, c, d, i) {
  {
    e["bits"] = kernel("r2/body/0", e, c, d, i);
    complete("r2/body/0", d);
  }
  {
    e["bit"] = calc("r2/body/1", e, c, d, i);
    complete("r2/body/1", d);
  }
  {
    e["test"] = calc("r2/body/2", e, c, d, i);
    complete("r2/body/2", d);
  }
  {
    const t = branch("r2/body/3", e, c, d, i);
    const parent = c;
    c = c.concat([t.id]);
    if (t.truth) {
      {
        stop("r2/body/3/then/0", e, c, d, i);
        complete("r2/body/3/then/0", d);
      }
    } else {
    }
    c = parent;
    complete("r2/body/3", d);
  }
  {
    e["receipt"] = preview("r2/body/4", e, c, d, i);
    complete("r2/body/4", d);
  }
}
function r2(e, c, d, i) {
  {
    e["bits"] = kernel("r3/body/0", e, c, d, i);
    complete("r3/body/0", d);
  }
  {
    e["bit"] = calc("r3/body/1", e, c, d, i);
    complete("r3/body/1", d);
  }
  {
    e["test"] = calc("r3/body/2", e, c, d, i);
    complete("r3/body/2", d);
  }
  {
    const t = branch("r3/body/3", e, c, d, i);
    const parent = c;
    c = c.concat([t.id]);
    if (t.truth) {
      {
        stop("r3/body/3/then/0", e, c, d, i);
        complete("r3/body/3/then/0", d);
      }
    } else {
    }
    c = parent;
    complete("r3/body/3", d);
  }
  {
    e["receipt"] = preview("r3/body/4", e, c, d, i);
    complete("r3/body/4", d);
  }
}
dispatch({"r1":r0,"r2":r1,"r3":r2});
"use strict";
// neutral-json {"inputs":{"target":{"origin":"manual","type":"Target","value":"c0"},"word":{"origin":"manual","type":"Word","value":["a"]}},"modules":{},"runs":[{"author":"built_in","body":[{"args":[],"as":"v","op":"kernel","row":"counts","type":"Ints"},{"op":"return","value":{"ref":"v"}}],"boundary_class":"tracking","id":"r1","invocation":"command","profile":"solving"}]}
function r0(e, c, d, i) {
  {
    e["v"] = kernel("r1/body/0", e, c, d, i);
    complete("r1/body/0", d);
  }
  {
    return finish("r1/body/1", e, c, d, i);
  }
}
dispatch({"r1":r0});
"use strict";
// neutral-json {"inputs":{"target":{"origin":"manual","type":"Target","value":"c0"},"word":{"origin":"manual","type":"Word","value":["a"]}},"modules":{},"runs":[{"author":"built_in","body":[{"args":[],"as":"v","op":"kernel","row":"floors","type":"Ints"},{"op":"return","value":{"ref":"v"}}],"boundary_class":"tracking","id":"r1","invocation":"command","profile":"solving"}]}
function r0(e, c, d, i) {
  {
    e["v"] = kernel("r1/body/0", e, c, d, i);
    complete("r1/body/0", d);
  }
  {
    return finish("r1/body/1", e, c, d, i);
  }
}
dispatch({"r1":r0});
"use strict";
// neutral-json {"inputs":{"target":{"origin":"manual","type":"Target","value":"c0"},"word":{"origin":"manual","type":"Word","value":["a"]}},"modules":{},"runs":[{"author":"built_in","body":[{"args":[],"as":"v","op":"kernel","row":"blocked","type":"Bools"},{"op":"return","value":{"ref":"v"}}],"boundary_class":"tracking","id":"r1","invocation":"command","profile":"solving"}]}
function r0(e, c, d, i) {
  {
    e["v"] = kernel("r1/body/0", e, c, d, i);
    complete("r1/body/0", d);
  }
  {
    return finish("r1/body/1", e, c, d, i);
  }
}
dispatch({"r1":r0});
"use strict";
// neutral-json {"inputs":{"target":{"origin":"manual","type":"Target","value":"c0"},"word":{"origin":"manual","type":"Word","value":["a"]}},"modules":{},"runs":[{"author":"built_in","body":[{"args":[],"as":"v","op":"kernel","row":"snapshot","type":"State"},{"op":"return","value":{"ref":"v"}}],"boundary_class":"tracking","id":"r1","invocation":"command","profile":"solving"}]}
function r0(e, c, d, i) {
  {
    e["v"] = kernel("r1/body/0", e, c, d, i);
    complete("r1/body/0", d);
  }
  {
    return finish("r1/body/1", e, c, d, i);
  }
}
dispatch({"r1":r0});
"use strict";
// neutral-json {"inputs":{"target":{"origin":"manual","type":"Target","value":"c0"},"word":{"origin":"manual","type":"Word","value":["a"]}},"modules":{},"runs":[{"author":"built_in","body":[{"args":[],"as":"n","op":"kernel","row":"counts","type":"Ints"},{"args":[{"type":"Targets","value":["c0","c1","c2"]},{"ref":"n"}],"as":"rows","fn":"zip","op":"calc","type":"Rows"},{"op":"return","value":{"ref":"rows"}}],"boundary_class":"tracking","id":"r1","invocation":"command","profile":"solving"}]}
function r0(e, c, d, i) {
  {
    e["n"] = kernel("r1/body/0", e, c, d, i);
    complete("r1/body/0", d);
  }
  {
    e["rows"] = calc("r1/body/1", e, c, d, i);
    complete("r1/body/1", d);
  }
  {
    return finish("r1/body/2", e, c, d, i);
  }
}
dispatch({"r1":r0});
"use strict";
// neutral-json {"inputs":{"target":{"origin":"manual","type":"Target","value":"c0"},"word":{"origin":"manual","type":"Word","value":["a"]}},"modules":{},"runs":[{"author":"built_in","body":[{"args":[],"as":"n","op":"kernel","row":"counts","type":"Ints"},{"args":[{"ref":"n"}],"as":"sum","fn":"sum","op":"calc","type":"Int"},{"op":"return","value":{"ref":"sum"}}],"boundary_class":"analysis","id":"r1","invocation":"command","profile":"solving"}]}
function r0(e, c, d, i) {
  {
    e["n"] = kernel("r1/body/0", e, c, d, i);
    complete("r1/body/0", d);
  }
  {
    e["sum"] = calc("r1/body/1", e, c, d, i);
    complete("r1/body/1", d);
  }
  {
    return finish("r1/body/2", e, c, d, i);
  }
}
dispatch({"r1":r0});
"use strict";
// neutral-json {"inputs":{"target":{"origin":"manual","type":"Target","value":"c0"},"word":{"origin":"manual","type":"Word","value":["a"]}},"modules":{},"runs":[{"author":"built_in","body":[{"args":[],"as":"f","op":"kernel","row":"floors","type":"Ints"},{"args":[{"ref":"f"},{"type":"Int","value":0}],"as":"z","fn":"count_equal","op":"calc","type":"Int"},{"op":"return","value":{"ref":"z"}}],"boundary_class":"analysis","id":"r1","invocation":"command","profile":"solving"}]}
function r0(e, c, d, i) {
  {
    e["f"] = kernel("r1/body/0", e, c, d, i);
    complete("r1/body/0", d);
  }
  {
    e["z"] = calc("r1/body/1", e, c, d, i);
    complete("r1/body/1", d);
  }
  {
    return finish("r1/body/2", e, c, d, i);
  }
}
dispatch({"r1":r0});
"use strict";
// neutral-json {"inputs":{"word":{"origin":"manual","type":"Word","value":["a","a","b"]}},"modules":{},"runs":[{"author":"built_in","body":[{"args":[{"ref":"word"}],"as":"n","fn":"length","op":"calc","type":"Int"},{"op":"return","value":{"ref":"n"}}],"boundary_class":"analysis","id":"r1","invocation":"command","profile":"solving"}]}
function r0(e, c, d, i) {
  {
    e["n"] = calc("r1/body/0", e, c, d, i);
    complete("r1/body/0", d);
  }
  {
    return finish("r1/body/1", e, c, d, i);
  }
}
dispatch({"r1":r0});
"use strict";
// neutral-json {"inputs":{"target":{"origin":"manual","type":"Target","value":"c0"},"word":{"origin":"manual","type":"Word","value":["a"]}},"modules":{},"runs":[{"author":"built_in","body":[{"args":[{"type":"Int","value":9007199254740993},{"type":"Int","value":7}],"as":"n","fn":"add","op":"calc","type":"Int"},{"op":"return","value":{"ref":"n"}}],"boundary_class":"analysis","id":"r1","invocation":"command","profile":"solving"}]}
function r0(e, c, d, i) {
  {
    e["n"] = calc("r1/body/0", e, c, d, i);
    complete("r1/body/0", d);
  }
  {
    return finish("r1/body/1", e, c, d, i);
  }
}
dispatch({"r1":r0});
"use strict";
// neutral-json {"inputs":{"target":{"origin":"manual","type":"Target","value":"c0"},"word":{"origin":"manual","type":"Word","value":["a"]}},"modules":{},"runs":[{"author":"built_in","body":[{"args":[{"type":"Ints","value":[1,2]},{"type":"Ints","value":[3,1]},{"type":"Int","value":5}],"as":"n","fn":"mod_dot","op":"calc","type":"Int"},{"op":"return","value":{"ref":"n"}}],"boundary_class":"analysis","id":"r1","invocation":"command","profile":"solving"}]}
function r0(e, c, d, i) {
  {
    e["n"] = calc("r1/body/0", e, c, d, i);
    complete("r1/body/0", d);
  }
  {
    return finish("r1/body/1", e, c, d, i);
  }
}
dispatch({"r1":r0});
"use strict";
// neutral-json {"inputs":{"target":{"origin":"manual","type":"Target","value":"c0"},"word":{"origin":"manual","type":"Word","value":["a"]}},"modules":{},"runs":[{"author":"built_in","body":[{"args":[],"as":"v","op":"kernel","row":"counts","type":"Ints"},{"args":[{"ref":"v"}],"as":"coords","fn":"indexed_points","op":"calc","type":"Rows"},{"coordinates":{"ref":"coords"},"exact":{"type":"Record","value":{"exact_properties":{"values":[2,6,6]},"kind":"points","object_ids":["c0","c1","c2"],"order":[0,1,2]}},"op":"publish"},{"op":"return","value":{"type":"Text","value":"published"}}],"boundary_class":"view_aid","id":"r1","invocation":"command","profile":"solving"}]}
function r0(e, c, d, i) {
  {
    e["v"] = kernel("r1/body/0", e, c, d, i);
    complete("r1/body/0", d);
  }
  {
    e["coords"] = calc("r1/body/1", e, c, d, i);
    complete("r1/body/1", d);
  }
  {
    publish("r1/body/2", e, c, d, i);
    complete("r1/body/2", d);
  }
  {
    return finish("r1/body/3", e, c, d, i);
  }
}
dispatch({"r1":r0});
"use strict";
// neutral-json {"inputs":{"target":{"origin":"manual","type":"Target","value":"c0"},"word":{"origin":"manual","type":"Word","value":["a"]}},"modules":{},"runs":[{"author":"built_in","body":[{"args":[],"as":"v","op":"kernel","row":"floors","type":"Ints"},{"args":[{"ref":"v"}],"as":"coords","fn":"indexed_points","op":"calc","type":"Rows"},{"coordinates":{"ref":"coords"},"exact":{"type":"Record","value":{"exact_properties":{"values":[0,2,2]},"kind":"points","object_ids":["c0","c1","c2"],"order":[0,1,2]}},"op":"publish"},{"op":"return","value":{"type":"Text","value":"published"}}],"boundary_class":"view_aid","id":"r1","invocation":"command","profile":"solving"}]}
function r0(e, c, d, i) {
  {
    e["v"] = kernel("r1/body/0", e, c, d, i);
    complete("r1/body/0", d);
  }
  {
    e["coords"] = calc("r1/body/1", e, c, d, i);
    complete("r1/body/1", d);
  }
  {
    publish("r1/body/2", e, c, d, i);
    complete("r1/body/2", d);
  }
  {
    return finish("r1/body/3", e, c, d, i);
  }
}
dispatch({"r1":r0});
"use strict";
// neutral-json {"inputs":{"target":{"origin":"manual","type":"Target","value":"c1"}},"modules":{},"runs":[{"author":"built_in","body":[{"args":[],"as":"n","op":"kernel","row":"counts","type":"Ints"},{"args":[{"ref":"n"},{"type":"Int","value":1}],"as":"v","fn":"index","op":"calc","type":"Int"},{"args":[{"type":"Int","value":1},{"ref":"v"}],"as":"xy","fn":"point","op":"calc","type":"Rows"},{"coordinates":{"ref":"xy"},"exact":{"type":"Record","value":{"exact_properties":{"value":6},"kind":"points","object_ids":["c1"],"order":[0]}},"op":"publish"},{"op":"return","value":{"type":"Text","value":"published"}}],"boundary_class":"view_aid","id":"r1","invocation":"command","profile":"solving"}]}
function r0(e, c, d, i) {
  {
    e["n"] = kernel("r1/body/0", e, c, d, i);
    complete("r1/body/0", d);
  }
  {
    e["v"] = calc("r1/body/1", e, c, d, i);
    complete("r1/body/1", d);
  }
  {
    e["xy"] = calc("r1/body/2", e, c, d, i);
    complete("r1/body/2", d);
  }
  {
    publish("r1/body/3", e, c, d, i);
    complete("r1/body/3", d);
  }
  {
    return finish("r1/body/4", e, c, d, i);
  }
}
dispatch({"r1":r0});
"use strict";
// neutral-json {"inputs":{"target":{"origin":"manual","type":"Target","value":"c0"},"word":{"origin":"manual","type":"Word","value":["a"]}},"modules":{},"runs":[{"author":"built_in","body":[{"args":[{"ref":"word"}],"as":"check","op":"kernel","row":"word_check","type":"Record"},{"coordinates":{"type":"Rows","value":[[0,1],[1,0]]},"exact":{"type":"Record","value":{"exact_properties":{"word":["a"]},"kind":"word_path","object_ids":["slot0","slot1"],"order":[0,1]}},"op":"publish"},{"op":"return","value":{"type":"Text","value":"published"}}],"boundary_class":"view_aid","id":"r1","invocation":"command","profile":"solving"}]}
function r0(e, c, d, i) {
  {
    e["check"] = kernel("r1/body/0", e, c, d, i);
    complete("r1/body/0", d);
  }
  {
    publish("r1/body/1", e, c, d, i);
    complete("r1/body/1", d);
  }
  {
    return finish("r1/body/2", e, c, d, i);
  }
}
dispatch({"r1":r0});
"use strict";
// neutral-json {"inputs":{"target":{"origin":"manual","type":"Target","value":"c0"},"word":{"origin":"manual","type":"Word","value":["a"]}},"modules":{},"runs":[{"author":"built_in","body":[{"coordinates":{"type":"Rows","value":[[0,0],[{"denominator":3,"numerator":1},1]]},"exact":{"type":"Record","value":{"exact_properties":{"fraction":{"denominator":3,"numerator":1}},"kind":"reference","object_ids":["origin","third"],"order":[0,1]}},"op":"publish"},{"op":"return","value":{"type":"Text","value":"published"}}],"boundary_class":"view_aid","id":"r1","invocation":"command","profile":"solving"}]}
function r0(e, c, d, i) {
  {
    publish("r1/body/0", e, c, d, i);
    complete("r1/body/0", d);
  }
  {
    return finish("r1/body/1", e, c, d, i);
  }
}
dispatch({"r1":r0});
"use strict";
// neutral-json {"inputs":{"target":{"origin":"manual","type":"Target","value":"c0"},"word":{"origin":"manual","type":"Word","value":["a"]}},"modules":{},"runs":[{"author":"built_in","body":[{"args":[{"ref":"word"}],"as":"check","op":"kernel","row":"word_check","type":"Record"},{"op":"return","value":{"ref":"check"}}],"boundary_class":"analysis","id":"r1","invocation":"command","profile":"solving"}]}
function r0(e, c, d, i) {
  {
    e["check"] = kernel("r1/body/0", e, c, d, i);
    complete("r1/body/0", d);
  }
  {
    return finish("r1/body/1", e, c, d, i);
  }
}
dispatch({"r1":r0});
"use strict";
// neutral-json {"inputs":{"word":{"origin":"manual","type":"Word","value":["D","a"]}},"modules":{},"runs":[{"author":"built_in","body":[{"args":[{"ref":"word"}],"as":"inverse","fn":"inverse","op":"calc","type":"Word"},{"op":"return","value":{"ref":"inverse"}}],"boundary_class":"analysis","id":"r1","invocation":"command","profile":"solving"}]}
function r0(e, c, d, i) {
  {
    e["inverse"] = calc("r1/body/0", e, c, d, i);
    complete("r1/body/0", d);
  }
  {
    return finish("r1/body/1", e, c, d, i);
  }
}
dispatch({"r1":r0});
"use strict";
// neutral-json {"inputs":{"left":{"origin":"manual","type":"Word","value":["a","b"]},"right":{"origin":"manual","type":"Word","value":["c"]}},"modules":{},"runs":[{"author":"built_in","body":[{"args":[{"ref":"left"},{"ref":"right"}],"as":"combined","fn":"concat","op":"calc","type":"Word"},{"op":"return","value":{"ref":"combined"}}],"boundary_class":"analysis","id":"r1","invocation":"command","profile":"solving"}]}
function r0(e, c, d, i) {
  {
    e["combined"] = calc("r1/body/0", e, c, d, i);
    complete("r1/body/0", d);
  }
  {
    return finish("r1/body/1", e, c, d, i);
  }
}
dispatch({"r1":r0});
"use strict";
// neutral-json {"inputs":{"word":{"origin":"manual","type":"Word","value":["a","a"]}},"modules":{},"runs":[{"author":"built_in","body":[{"args":[{"ref":"word"}],"as":"check","op":"kernel","row":"word_check","type":"Record"},{"op":"return","value":{"ref":"check"}}],"boundary_class":"analysis","id":"r1","invocation":"command","profile":"solving"}]}
function r0(e, c, d, i) {
  {
    e["check"] = kernel("r1/body/0", e, c, d, i);
    complete("r1/body/0", d);
  }
  {
    return finish("r1/body/1", e, c, d, i);
  }
}
dispatch({"r1":r0});
"use strict";
// neutral-json {"inputs":{"target":{"origin":"manual","type":"Target","value":"c0"},"word":{"origin":"manual","type":"Word","value":["a"]}},"modules":{},"runs":[{"author":"built_in","body":[{"op":"return","value":{"type":"Text","value":"Named setup: c1; labels: \u00e9 / e\u0301 / \u03bd."}}],"boundary_class":"analysis","id":"r1","invocation":"command","profile":"solving"}]}
function r0(e, c, d, i) {
  {
    return finish("r1/body/0", e, c, d, i);
  }
}
dispatch({"r1":r0});
"use strict";
// neutral-json {"inputs":{"word":{"origin":"manual","type":"Word","value":["a"]}},"modules":{},"runs":[{"author":"built_in","body":[{"args":[{"ref":"word"}],"as":"check","op":"kernel","row":"word_check","type":"Record"},{"op":"return","value":{"ref":"check"}}],"boundary_class":"checking","id":"r1","invocation":"command","profile":"solving"}]}
function r0(e, c, d, i) {
  {
    e["check"] = kernel("r1/body/0", e, c, d, i);
    complete("r1/body/0", d);
  }
  {
    return finish("r1/body/1", e, c, d, i);
  }
}
dispatch({"r1":r0});
"use strict";
// neutral-json {"inputs":{"word":{"origin":"manual","type":"Word","value":["b"]}},"modules":{},"runs":[{"author":"built_in","body":[{"args":[{"ref":"word"}],"as":"check","op":"kernel","row":"word_check","type":"Record"},{"op":"return","value":{"ref":"check"}}],"boundary_class":"checking","id":"r1","invocation":"command","profile":"solving"}]}
function r0(e, c, d, i) {
  {
    e["check"] = kernel("r1/body/0", e, c, d, i);
    complete("r1/body/0", d);
  }
  {
    return finish("r1/body/1", e, c, d, i);
  }
}
dispatch({"r1":r0});
"use strict";
// neutral-json {"inputs":{"word":{"origin":"manual","type":"Word","value":["c"]}},"modules":{},"runs":[{"author":"built_in","body":[{"args":[{"ref":"word"}],"as":"check","op":"kernel","row":"word_check","type":"Record"},{"op":"return","value":{"ref":"check"}}],"boundary_class":"checking","id":"r1","invocation":"command","profile":"solving"}]}
function r0(e, c, d, i) {
  {
    e["check"] = kernel("r1/body/0", e, c, d, i);
    complete("r1/body/0", d);
  }
  {
    return finish("r1/body/1", e, c, d, i);
  }
}
dispatch({"r1":r0});
"use strict";
// neutral-json {"inputs":{"target":{"origin":"manual","type":"Target","value":"c0"},"word":{"origin":"manual","type":"Word","value":["a"]}},"modules":{},"runs":[{"author":"built_in","body":[{"as":"receipt","op":"preview","target":{"ref":"target"},"type":"Receipt","word":{"ref":"word"}},{"op":"return","value":{"type":"Text","value":"requested"}}],"boundary_class":"analysis","id":"r1","invocation":"command","profile":"solving"}]}
function r0(e, c, d, i) {
  {
    e["receipt"] = preview("r1/body/0", e, c, d, i);
    complete("r1/body/0", d);
  }
  {
    return finish("r1/body/1", e, c, d, i);
  }
}
dispatch({"r1":r0});
"use strict";
// neutral-json {"inputs":{"target":{"origin":"manual","type":"Target","value":"c0"},"word":{"origin":"manual","type":"Word","value":["a"]}},"modules":{},"runs":[{"author":"built_in","body":[{"args":[],"as":"n","op":"kernel","row":"counts","type":"Ints"},{"op":"emit","value":{"ref":"n"}},{"as":"receipt","op":"preview","target":{"ref":"target"},"type":"Receipt","word":{"ref":"word"}},{"op":"return","value":{"type":"Text","value":"requested"}}],"boundary_class":"analysis","id":"r1","invocation":"command","profile":"solving"}]}
function r0(e, c, d, i) {
  {
    e["n"] = kernel("r1/body/0", e, c, d, i);
    complete("r1/body/0", d);
  }
  {
    emit("r1/body/1", e, c, d, i);
    complete("r1/body/1", d);
  }
  {
    e["receipt"] = preview("r1/body/2", e, c, d, i);
    complete("r1/body/2", d);
  }
  {
    return finish("r1/body/3", e, c, d, i);
  }
}
dispatch({"r1":r0});
"use strict";
// neutral-json {"inputs":{"target":{"origin":"manual","type":"Target","value":"c0"}},"modules":{},"runs":[{"author":"built_in","body":[{"args":[{"ref":"target"}],"as":"setups","op":"kernel","row":"setup","type":"Words"},{"op":"return","value":{"ref":"setups"}}],"boundary_class":"setup_search_named_target","id":"r1","invocation":"command","profile":"solving"}]}
function r0(e, c, d, i) {
  {
    e["setups"] = kernel("r1/body/0", e, c, d, i);
    complete("r1/body/0", d);
  }
  {
    return finish("r1/body/1", e, c, d, i);
  }
}
dispatch({"r1":r0});
"use strict";
// neutral-json {"inputs":{"target":{"origin":"manual","type":"Target","value":"c1"}},"modules":{},"runs":[{"author":"built_in","body":[{"args":[{"ref":"target"}],"as":"setups","op":"kernel","row":"setup","type":"Words"},{"op":"return","value":{"ref":"setups"}}],"boundary_class":"setup_search_named_target","id":"r1","invocation":"command","profile":"solving"}]}
function r0(e, c, d, i) {
  {
    e["setups"] = kernel("r1/body/0", e, c, d, i);
    complete("r1/body/0", d);
  }
  {
    return finish("r1/body/1", e, c, d, i);
  }
}
dispatch({"r1":r0});
"use strict";
// neutral-json {"inputs":{"target":{"origin":"manual","type":"Target","value":"c2"}},"modules":{},"runs":[{"author":"built_in","body":[{"args":[{"ref":"target"}],"as":"setups","op":"kernel","row":"setup","type":"Words"},{"op":"return","value":{"ref":"setups"}}],"boundary_class":"setup_search_named_target","id":"r1","invocation":"command","profile":"solving"}]}
function r0(e, c, d, i) {
  {
    e["setups"] = kernel("r1/body/0", e, c, d, i);
    complete("r1/body/0", d);
  }
  {
    return finish("r1/body/1", e, c, d, i);
  }
}
dispatch({"r1":r0});
"use strict";
// neutral-json {"inputs":{"pick0":{"origin":"manual","type":"Int","value":0},"target":{"origin":"manual","type":"Target","value":"c1"}},"modules":{},"runs":[{"author":"built_in","body":[{"args":[{"ref":"target"}],"as":"setups","op":"kernel","row":"setup","type":"Words"},{"as":"w","event":"pick0","op":"pick","options":{"ref":"setups"},"type":"Word"},{"as":"receipt","op":"preview","target":{"ref":"target"},"type":"Receipt","word":{"ref":"w"}},{"op":"return","value":{"type":"Text","value":"requested"}}],"boundary_class":"setup_search_named_target","id":"r1","invocation":"command","profile":"solving"}]}
function r0(e, c, d, i) {
  {
    e["setups"] = kernel("r1/body/0", e, c, d, i);
    complete("r1/body/0", d);
  }
  {
    e["w"] = pick("r1/body/1", e, c, d, i);
    complete("r1/body/1", d);
  }
  {
    e["receipt"] = preview("r1/body/2", e, c, d, i);
    complete("r1/body/2", d);
  }
  {
    return finish("r1/body/3", e, c, d, i);
  }
}
dispatch({"r1":r0});
"use strict";
// neutral-json {"inputs":{"pick0":{"origin":"manual","type":"Int","value":0},"target":{"origin":"manual","type":"Target","value":"c1"}},"modules":{},"runs":[{"author":"built_in","body":[{"args":[],"as":"n","op":"kernel","row":"counts","type":"Ints"},{"op":"emit","value":{"ref":"n"}},{"args":[{"ref":"target"}],"as":"setups","op":"kernel","row":"setup","type":"Words"},{"as":"w","event":"pick0","op":"pick","options":{"ref":"setups"},"type":"Word"},{"as":"receipt","op":"preview","target":{"ref":"target"},"type":"Receipt","word":{"ref":"w"}},{"op":"return","value":{"type":"Text","value":"requested"}}],"boundary_class":"setup_search_named_target","id":"r1","invocation":"command","profile":"solving"}]}
function r0(e, c, d, i) {
  {
    e["n"] = kernel("r1/body/0", e, c, d, i);
    complete("r1/body/0", d);
  }
  {
    emit("r1/body/1", e, c, d, i);
    complete("r1/body/1", d);
  }
  {
    e["setups"] = kernel("r1/body/2", e, c, d, i);
    complete("r1/body/2", d);
  }
  {
    e["w"] = pick("r1/body/3", e, c, d, i);
    complete("r1/body/3", d);
  }
  {
    e["receipt"] = preview("r1/body/4", e, c, d, i);
    complete("r1/body/4", d);
  }
  {
    return finish("r1/body/5", e, c, d, i);
  }
}
dispatch({"r1":r0});
"use strict";
// neutral-json {"inputs":{"target":{"origin":"manual","type":"Target","value":"c0"},"word":{"origin":"manual","type":"Word","value":["a"]}},"modules":{},"runs":[{"author":"built_in","body":[{"args":[],"as":"nu","op":"kernel","row":"nu2","type":"Ints"},{"args":[{"type":"Matrix","value":[[1,0],[0,1]]},{"ref":"nu"},{"type":"Int","value":2}],"as":"coef","fn":"linear_solve_mod","op":"calc","type":"Ints"},{"args":[{"type":"Words","value":[["a"],["b"]]},{"ref":"coef"}],"as":"w","fn":"combine_corrections","op":"calc","type":"Word"},{"op":"return","value":{"ref":"w"}}],"boundary_class":"analysis","id":"r1","invocation":"command","profile":"research"}]}
function r0(e, c, d, i) {
  {
    e["nu"] = kernel("r1/body/0", e, c, d, i);
    complete("r1/body/0", d);
  }
  {
    e["coef"] = calc("r1/body/1", e, c, d, i);
    complete("r1/body/1", d);
  }
  {
    e["w"] = calc("r1/body/2", e, c, d, i);
    complete("r1/body/2", d);
  }
  {
    return finish("r1/body/3", e, c, d, i);
  }
}
dispatch({"r1":r0});
"use strict";
// neutral-json {"inputs":{"target":{"origin":"manual","type":"Target","value":"c0"},"word":{"origin":"manual","type":"Word","value":["a"]}},"modules":{},"runs":[{"author":"built_in","body":[{"args":[],"as":"nu","op":"kernel","row":"nu5","type":"Ints"},{"args":[{"type":"Matrix","value":[[1,0],[0,1]]},{"ref":"nu"},{"type":"Int","value":5}],"as":"coef","fn":"linear_solve_mod","op":"calc","type":"Ints"},{"args":[{"type":"Words","value":[["c"],["d"]]},{"ref":"coef"}],"as":"w","fn":"combine_corrections","op":"calc","type":"Word"},{"op":"return","value":{"ref":"w"}}],"boundary_class":"analysis","id":"r1","invocation":"command","profile":"research"}]}
function r0(e, c, d, i) {
  {
    e["nu"] = kernel("r1/body/0", e, c, d, i);
    complete("r1/body/0", d);
  }
  {
    e["coef"] = calc("r1/body/1", e, c, d, i);
    complete("r1/body/1", d);
  }
  {
    e["w"] = calc("r1/body/2", e, c, d, i);
    complete("r1/body/2", d);
  }
  {
    return finish("r1/body/3", e, c, d, i);
  }
}
dispatch({"r1":r0});
"use strict";
// neutral-json {"inputs":{"caps":{"origin":"manual","type":"Targets","value":["c0","c1","c2"]},"target":{"origin":"manual","type":"Target","value":"c0"},"word":{"origin":"manual","type":"Word","value":["a"]},"words":{"origin":"manual","type":"Words","value":[["a"],["b"],["c"]]}},"modules":{},"runs":[{"author":"built_in","body":[{"args":[{"type":"Ints","value":[]}],"as":"scores","fn":"identity","op":"calc","type":"Ints"},{"body":[{"args":[{"ref":"w"}],"as":"s","op":"kernel","row":"score","type":"Int"},{"args":[{"ref":"scores"},{"ref":"s"}],"as":"scores","fn":"append","op":"calc","type":"Ints"}],"item":"w","items":{"ref":"words"},"op":"for","type":"Word"},{"args":[{"ref":"scores"}],"as":"i","fn":"argmin_first","op":"calc","type":"Int"},{"args":[{"ref":"words"},{"ref":"i"}],"as":"chosen","fn":"index","op":"calc","type":"Word"},{"op":"return","value":{"ref":"chosen"}}],"boundary_class":"analysis","id":"r1","invocation":"command","profile":"research"}]}
function r0(e, c, d, i) {
  {
    e["scores"] = calc("r1/body/0", e, c, d, i);
    complete("r1/body/0", d);
  }
  {
    const seq = sequence("r1/body/1", e, c, d, i);
    const outer = e;
    const previous = i;
    const parent = c;
    c = c.concat([seq]);
    for (i = 0; loop("r1/body/1", seq, i); i++) {
      e = Object.assign(Object.create(null), outer);
      e["w"] = item("r1/body/1", seq, c, i);
      {
        e["s"] = kernel("r1/body/1/body/0", e, c, d, i);
        complete("r1/body/1/body/0", d);
      }
      {
        e["scores"] = calc("r1/body/1/body/1", e, c, d, i);
        complete("r1/body/1/body/1", d);
      }
      merge("r1/body/1", outer, e);
      covered("r1/body/1", i);
    }
    e = outer;
    i = previous;
    c = parent;
    complete("r1/body/1", d);
  }
  {
    e["i"] = calc("r1/body/2", e, c, d, i);
    complete("r1/body/2", d);
  }
  {
    e["chosen"] = calc("r1/body/3", e, c, d, i);
    complete("r1/body/3", d);
  }
  {
    return finish("r1/body/4", e, c, d, i);
  }
}
dispatch({"r1":r0});
"use strict";
// neutral-json {"inputs":{"target":{"origin":"manual","type":"Target","value":"c0"},"word":{"origin":"manual","type":"Word","value":["a"]}},"modules":{},"runs":[{"author":"built_in","body":[{"args":[],"as":"nu","op":"kernel","row":"nu2","type":"Ints"},{"key":"research_nu","op":"workspace_put","value":{"ref":"nu"}},{"as":"saved","key":"research_nu","op":"workspace_get","type":"Ints"},{"op":"return","value":{"ref":"saved"}}],"boundary_class":"analysis","id":"r1","invocation":"command","profile":"research"}]}
function r0(e, c, d, i) {
  {
    e["nu"] = kernel("r1/body/0", e, c, d, i);
    complete("r1/body/0", d);
  }
  {
    workspace_put("r1/body/1", e, c, d, i);
    complete("r1/body/1", d);
  }
  {
    e["saved"] = workspace_get("r1/body/2", e, c, d, i);
    complete("r1/body/2", d);
  }
  {
    return finish("r1/body/3", e, c, d, i);
  }
}
dispatch({"r1":r0});
"use strict";
// neutral-json {"inputs":{"target":{"origin":"manual","type":"Target","value":"c0"},"word":{"origin":"manual","type":"Word","value":["a"]}},"modules":{},"runs":[{"author":"built_in","body":[{"args":[],"as":"nu","op":"kernel","row":"nu2","type":"Ints"},{"args":[{"type":"Words","value":[["a"],["b"]]},{"ref":"nu"}],"as":"w","fn":"combine_corrections","op":"calc","type":"Word"},{"as":"receipt","op":"preview","target":{"ref":"target"},"type":"Receipt","word":{"ref":"w"}},{"op":"return","value":{"type":"Text","value":"captured"}}],"boundary_class":"analysis","id":"r1","invocation":"command","profile":"research"}]}
function r0(e, c, d, i) {
  {
    e["nu"] = kernel("r1/body/0", e, c, d, i);
    complete("r1/body/0", d);
  }
  {
    e["w"] = calc("r1/body/1", e, c, d, i);
    complete("r1/body/1", d);
  }
  {
    e["receipt"] = preview("r1/body/2", e, c, d, i);
    complete("r1/body/2", d);
  }
  {
    return finish("r1/body/3", e, c, d, i);
  }
}
dispatch({"r1":r0});
"use strict";
// neutral-json {"inputs":{},"modules":{},"runs":[{"author":"own","body":[{"as":"leak","logical_resource":"clock_probe","op":"ambient","resource":"clock","type":"Text"},{"op":"emit","value":{"ref":"leak"}},{"op":"return","value":{"ref":"leak"}}],"boundary_class":null,"id":"r1","invocation":"command","profile":"tool_authoring"}]}
function r0(e, c, d, i) {
  {
    e["leak"] = ambient("r1/body/0", e, c, d, i);
    complete("r1/body/0", d);
  }
  {
    emit("r1/body/1", e, c, d, i);
    complete("r1/body/1", d);
  }
  {
    return finish("r1/body/2", e, c, d, i);
  }
}
dispatch({"r1":r0});
"use strict";
// neutral-json {"inputs":{},"modules":{},"runs":[{"author":"own","body":[{"as":"leak","logical_resource":"file_probe","op":"ambient","resource":"file","type":"Text"},{"op":"emit","value":{"ref":"leak"}},{"op":"return","value":{"ref":"leak"}}],"boundary_class":null,"id":"r1","invocation":"command","profile":"tool_authoring"}]}
function r0(e, c, d, i) {
  {
    e["leak"] = ambient("r1/body/0", e, c, d, i);
    complete("r1/body/0", d);
  }
  {
    emit("r1/body/1", e, c, d, i);
    complete("r1/body/1", d);
  }
  {
    return finish("r1/body/2", e, c, d, i);
  }
}
dispatch({"r1":r0});
"use strict";
// neutral-json {"inputs":{},"modules":{},"runs":[{"author":"own","body":[{"as":"leak","logical_resource":"network_probe","op":"ambient","resource":"network","type":"Text"},{"op":"emit","value":{"ref":"leak"}},{"op":"return","value":{"ref":"leak"}}],"boundary_class":null,"id":"r1","invocation":"command","profile":"tool_authoring"}]}
function r0(e, c, d, i) {
  {
    e["leak"] = ambient("r1/body/0", e, c, d, i);
    complete("r1/body/0", d);
  }
  {
    emit("r1/body/1", e, c, d, i);
    complete("r1/body/1", d);
  }
  {
    return finish("r1/body/2", e, c, d, i);
  }
}
dispatch({"r1":r0});
"use strict";
// neutral-json {"inputs":{},"modules":{},"runs":[{"author":"own","body":[{"as":"leak","logical_resource":"process_probe","op":"ambient","resource":"process","type":"Text"},{"op":"emit","value":{"ref":"leak"}},{"op":"return","value":{"ref":"leak"}}],"boundary_class":null,"id":"r1","invocation":"command","profile":"tool_authoring"}]}
function r0(e, c, d, i) {
  {
    e["leak"] = ambient("r1/body/0", e, c, d, i);
    complete("r1/body/0", d);
  }
  {
    emit("r1/body/1", e, c, d, i);
    complete("r1/body/1", d);
  }
  {
    return finish("r1/body/2", e, c, d, i);
  }
}
dispatch({"r1":r0});
"use strict";
// neutral-json {"inputs":{},"modules":{},"runs":[{"author":"own","body":[{"op":"tick"},{"op":"tick"},{"op":"tick"}],"boundary_class":null,"id":"r1","invocation":"command","profile":"tool_authoring"}]}
function r0(e, c, d, i) {
  {
    tick("r1/body/0", e, c, d, i);
    complete("r1/body/0", d);
  }
  {
    tick("r1/body/1", e, c, d, i);
    complete("r1/body/1", d);
  }
  {
    tick("r1/body/2", e, c, d, i);
    complete("r1/body/2", d);
  }
}
dispatch({"r1":r0});
"use strict";
// neutral-json {"inputs":{},"modules":{},"runs":[{"author":"own","body":[{"args":[],"as":"a","op":"kernel","row":"counts","type":"Ints"},{"args":[],"as":"b","op":"kernel","row":"counts","type":"Ints"}],"boundary_class":null,"id":"r1","invocation":"command","profile":"tool_authoring"}]}
function r0(e, c, d, i) {
  {
    e["a"] = kernel("r1/body/0", e, c, d, i);
    complete("r1/body/0", d);
  }
  {
    e["b"] = kernel("r1/body/1", e, c, d, i);
    complete("r1/body/1", d);
  }
}
dispatch({"r1":r0});
"use strict";
// neutral-json {"inputs":{},"modules":{},"runs":[{"author":"own","body":[{"as":"a","bytes":16,"op":"retain","type":"Bytes"},{"as":"b","bytes":16,"op":"retain","type":"Bytes"}],"boundary_class":null,"id":"r1","invocation":"command","profile":"tool_authoring"}]}
function r0(e, c, d, i) {
  {
    e["a"] = retain("r1/body/0", e, c, d, i);
    complete("r1/body/0", d);
  }
  {
    e["b"] = retain("r1/body/1", e, c, d, i);
    complete("r1/body/1", d);
  }
}
dispatch({"r1":r0});
"use strict";
// neutral-json {"inputs":{},"modules":{},"runs":[{"author":"own","body":[{"op":"emit","value":{"type":"Text","value":"abcd"}},{"op":"emit","value":{"type":"Text","value":"abcd"}}],"boundary_class":null,"id":"r1","invocation":"command","profile":"tool_authoring"}]}
function r0(e, c, d, i) {
  {
    emit("r1/body/0", e, c, d, i);
    complete("r1/body/0", d);
  }
  {
    emit("r1/body/1", e, c, d, i);
    complete("r1/body/1", d);
  }
}
dispatch({"r1":r0});
"use strict";
// neutral-json {"inputs":{},"modules":{},"runs":[{"author":"own","body":[{"args":[],"as":"n","op":"kernel","row":"counts","type":"Ints"}],"boundary_class":null,"id":"r1","invocation":"command","profile":"tool_authoring"}]}
function r0(e, c, d, i) {
  {
    e["n"] = kernel("r1/body/0", e, c, d, i);
    complete("r1/body/0", d);
  }
}
dispatch({"r1":r0});
"use strict";
// neutral-json {"inputs":{},"modules":{},"runs":[{"author":"own","body":[{"args":[],"as":"n","op":"kernel","row":"counts","type":"Ints"}],"boundary_class":null,"id":"r1","invocation":"command","profile":"tool_authoring"}]}
function r0(e, c, d, i) {
  {
    e["n"] = kernel("r1/body/0", e, c, d, i);
    complete("r1/body/0", d);
  }
}
dispatch({"r1":r0});
"use strict";
// neutral-json {"inputs":{},"modules":{"recurse":{"body":[{"args":[],"as":"result","module":"recurse","op":"call","type":"Int"},{"op":"return","value":{"ref":"result"}}],"parameters":{},"result":"Int"}},"runs":[{"author":"own","body":[{"args":[],"as":"result","module":"recurse","op":"call","type":"Int"}],"boundary_class":null,"id":"r1","invocation":"command","profile":"tool_authoring"}]}
function m0(e, c, d, i) {
  {
    const local = enter("modules/recurse/body/0", e, c, d, i);
    const value = m0(local, c, d + 1, null);
    e["result"] = leave("modules/recurse/body/0", value, e, c);
    complete("modules/recurse/body/0", d);
  }
  {
    return finish("modules/recurse/body/1", e, c, d, i);
  }
}
function r0(e, c, d, i) {
  {
    const local = enter("r1/body/0", e, c, d, i);
    const value = m0(local, c, d + 1, null);
    e["result"] = leave("r1/body/0", value, e, c);
    complete("r1/body/0", d);
  }
}
dispatch({"r1":r0});
"use strict";
// neutral-json {"inputs":{},"modules":{},"runs":[{"author":"own","body":[{"body":[{"op":"tick"}],"item":"i","items":{"type":"Ints","value":[0,1,2,3,4,5,6,7,8,9]},"op":"for","type":"Int"}],"boundary_class":null,"id":"r1","invocation":"command","profile":"tool_authoring"}]}
function r0(e, c, d, i) {
  {
    const seq = sequence("r1/body/0", e, c, d, i);
    const outer = e;
    const previous = i;
    const parent = c;
    c = c.concat([seq]);
    for (i = 0; loop("r1/body/0", seq, i); i++) {
      e = Object.assign(Object.create(null), outer);
      e["i"] = item("r1/body/0", seq, c, i);
      {
        tick("r1/body/0/body/0", e, c, d, i);
        complete("r1/body/0/body/0", d);
      }
      merge("r1/body/0", outer, e);
      covered("r1/body/0", i);
    }
    e = outer;
    i = previous;
    c = parent;
    complete("r1/body/0", d);
  }
}
dispatch({"r1":r0});
"use strict";
// neutral-json {"inputs":{},"modules":{},"runs":[{"author":"own","body":[{"args":[{"type":"Words","value":[["a"],["b"],["c"]]}],"as":"batch","op":"kernel","row":"batch_check","type":"Rows"}],"boundary_class":null,"id":"r1","invocation":"command","profile":"tool_authoring"}]}
function r0(e, c, d, i) {
  {
    e["batch"] = kernel("r1/body/0", e, c, d, i);
    complete("r1/body/0", d);
  }
}
dispatch({"r1":r0});
"use strict";
// neutral-json {"inputs":{},"modules":{},"runs":[{"author":"own","body":[{"body":[{"op":"tick"}],"item":"i","items":{"type":"Ints","value":[0,1,2,3,4,5,6,7,8,9]},"op":"for","type":"Int"}],"boundary_class":null,"id":"r1","invocation":"command","profile":"tool_authoring"}]}
function r0(e, c, d, i) {
  {
    const seq = sequence("r1/body/0", e, c, d, i);
    const outer = e;
    const previous = i;
    const parent = c;
    c = c.concat([seq]);
    for (i = 0; loop("r1/body/0", seq, i); i++) {
      e = Object.assign(Object.create(null), outer);
      e["i"] = item("r1/body/0", seq, c, i);
      {
        tick("r1/body/0/body/0", e, c, d, i);
        complete("r1/body/0/body/0", d);
      }
      merge("r1/body/0", outer, e);
      covered("r1/body/0", i);
    }
    e = outer;
    i = previous;
    c = parent;
    complete("r1/body/0", d);
  }
}
dispatch({"r1":r0});
"use strict";
// neutral-json {"inputs":{},"modules":{},"runs":[{"author":"own","body":[{"body":[{"op":"tick"}],"item":"i","items":{"type":"Ints","value":[0,1,2,3,4,5,6,7,8,9]},"op":"for","type":"Int"}],"boundary_class":null,"id":"r1","invocation":"command","profile":"tool_authoring"}]}
function r0(e, c, d, i) {
  {
    const seq = sequence("r1/body/0", e, c, d, i);
    const outer = e;
    const previous = i;
    const parent = c;
    c = c.concat([seq]);
    for (i = 0; loop("r1/body/0", seq, i); i++) {
      e = Object.assign(Object.create(null), outer);
      e["i"] = item("r1/body/0", seq, c, i);
      {
        tick("r1/body/0/body/0", e, c, d, i);
        complete("r1/body/0/body/0", d);
      }
      merge("r1/body/0", outer, e);
      covered("r1/body/0", i);
    }
    e = outer;
    i = previous;
    c = parent;
    complete("r1/body/0", d);
  }
}
dispatch({"r1":r0});
"use strict";
// neutral-json {"inputs":{},"modules":{},"runs":[{"author":"own","body":[{"args":[],"as":"a","op":"kernel","row":"counts","type":"Ints"},{"args":[],"as":"b","op":"kernel","row":"counts","type":"Ints"},{"op":"return","value":{"ref":"b"}}],"boundary_class":null,"id":"r1","invocation":"command","profile":"tool_authoring"}]}
function r0(e, c, d, i) {
  {
    e["a"] = kernel("r1/body/0", e, c, d, i);
    complete("r1/body/0", d);
  }
  {
    e["b"] = kernel("r1/body/1", e, c, d, i);
    complete("r1/body/1", d);
  }
  {
    return finish("r1/body/2", e, c, d, i);
  }
}
dispatch({"r1":r0});
"use strict";
// neutral-json {"inputs":{},"modules":{},"runs":[{"author":"own","body":[{"op":"tick"}],"boundary_class":null,"id":"r1","invocation":"command","profile":"tool_authoring"}]}
function r0(e, c, d, i) {
  {
    tick("r1/body/0", e, c, d, i);
    complete("r1/body/0", d);
  }
}
dispatch({"r1":r0});
"use strict";
// neutral-json {"inputs":{},"modules":{},"runs":[{"author":"own","body":[{"op":"tick"}],"boundary_class":null,"id":"r1","invocation":"command","profile":"tool_authoring"}]}
function r0(e, c, d, i) {
  {
    tick("r1/body/0", e, c, d, i);
    complete("r1/body/0", d);
  }
}
dispatch({"r1":r0});
"use strict";
// neutral-json {"inputs":{},"modules":{},"runs":[{"author":"own","body":[{"op":"dynamic","source":{"type":"Text","value":"constant arithmetic"}},{"op":"return","value":{"type":"Int","value":2}}],"boundary_class":null,"id":"r1","invocation":"command","profile":"tool_authoring"}]}
function r0(e, c, d, i) {
  {
    dynamic("r1/body/0", e, c, d, i);
    complete("r1/body/0", d);
  }
  {
    return finish("r1/body/1", e, c, d, i);
  }
}
dispatch({"r1":r0});
"use strict";
// neutral-json {"inputs":{},"modules":{"constant":{"body":[{"op":"return","value":{"type":"Int","value":1}}],"parameters":{},"result":"Int"}},"runs":[{"author":"own","body":[{"args":[],"as":"n","module":"constant","op":"call","type":"Int"},{"op":"return","value":{"ref":"n"}}],"boundary_class":null,"id":"r1","invocation":"command","profile":"tool_authoring"}]}
function m0(e, c, d, i) {
  {
    return finish("modules/constant/body/0", e, c, d, i);
  }
}
function r0(e, c, d, i) {
  {
    const local = enter("r1/body/0", e, c, d, i);
    const value = m0(local, c, d + 1, null);
    e["n"] = leave("r1/body/0", value, e, c);
    complete("r1/body/0", d);
  }
  {
    return finish("r1/body/1", e, c, d, i);
  }
}
dispatch({"r1":r0});

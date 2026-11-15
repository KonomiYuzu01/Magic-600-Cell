# Mechanically rendered experiment code; no host or syntax selection.
def r_7231(lib):
    v_63617073 = lib.input('caps')
    v_746172676574 = lib.input('target')
    v_776f7264 = lib.input('word')
    v_776f726473 = lib.input('words')
    lib.step('r1/body/0', None)
    v_6e4072312f626f6479 = lib.kernel('r1/body/0', 'Ints', 'counts', [], None)
    lib.done(0)
    lib.step('r1/body/1', None)
    v_694072312f626f6479 = lib.calc('r1/body/1', 'Int', 'argmax_first', [v_6e4072312f626f6479], None)
    lib.done(1)
    lib.step('r1/body/2', None)
    v_744072312f626f6479 = lib.calc('r1/body/2', 'Target', 'index', [v_63617073, v_694072312f626f6479], None)
    lib.done(2)
    lib.step('r1/body/3', None)
    v_726563656970744072312f626f6479 = lib.preview('r1/body/3', v_776f7264, v_744072312f626f6479)
    lib.done(3)

# Mechanically rendered experiment code; no host or syntax selection.
def r_7231(lib):
    v_63617073 = lib.input('caps')
    v_746172676574 = lib.input('target')
    v_776f7264 = lib.input('word')
    v_776f726473 = lib.input('words')
    lib.step('r1/body/0', None)
    v_6e4072312f626f6479 = lib.kernel('r1/body/0', 'Ints', 'counts', [], None)
    lib.done(0)
    lib.step('r1/body/1', None)
    v_72616e6b4072312f626f6479 = lib.calc('r1/body/1', 'Ints', 'rank_desc_indices', [v_6e4072312f626f6479], None)
    lib.done(1)
    lib.step('r1/body/2', None)
    v_694072312f626f6479 = lib.calc('r1/body/2', 'Int', 'index', [v_72616e6b4072312f626f6479, lib.literal('Int', 0, 'r1/body/2/args/1/literal')], None)
    lib.done(2)
    lib.step('r1/body/3', None)
    v_744072312f626f6479 = lib.calc('r1/body/3', 'Target', 'index', [v_63617073, v_694072312f626f6479], None)
    lib.done(3)
    lib.step('r1/body/4', None)
    v_726563656970744072312f626f6479 = lib.preview('r1/body/4', v_776f7264, v_744072312f626f6479)
    lib.done(4)

# Mechanically rendered experiment code; no host or syntax selection.
def r_7231(lib):
    v_63617073 = lib.input('caps')
    v_746172676574 = lib.input('target')
    v_776f7264 = lib.input('word')
    v_776f726473 = lib.input('words')
    lib.step('r1/body/0', None)
    v_73636f7265734072312f626f6479 = lib.calc('r1/body/0', 'Ints', 'identity', [lib.literal('Ints', [], 'r1/body/0/args/0/literal')], None)
    lib.done(0)
    lib.step('r1/body/1', None)
    for v_696e6465784072312f626f64792f31, v_774072312f626f64792f31 in lib.iterate('r1/body/1', v_776f726473, 'Word', False):
        lib.step('r1/body/1/body/0', v_696e6465784072312f626f64792f31)
        v_734072312f626f64792f312f626f6479 = lib.kernel('r1/body/1/body/0', 'Int', 'score', [v_774072312f626f64792f31], v_696e6465784072312f626f64792f31)
        lib.step('r1/body/1/body/1', v_696e6465784072312f626f64792f31)
        v_73636f7265734072312f626f6479 = lib.calc('r1/body/1/body/1', 'Ints', 'append', [v_73636f7265734072312f626f6479, v_734072312f626f64792f312f626f6479], v_696e6465784072312f626f64792f31)
    lib.done(1)
    lib.step('r1/body/2', None)
    v_694072312f626f6479 = lib.calc('r1/body/2', 'Int', 'argmin_first', [v_73636f7265734072312f626f6479], None)
    lib.done(2)
    lib.step('r1/body/3', None)
    v_63686f73656e4072312f626f6479 = lib.calc('r1/body/3', 'Word', 'index', [v_776f726473, v_694072312f626f6479], None)
    lib.done(3)
    lib.step('r1/body/4', None)
    v_72656275696c744072312f626f6479 = lib.calc('r1/body/4', 'Word', 'concat', [lib.literal('Word', [], 'r1/body/4/args/0/literal'), v_63686f73656e4072312f626f6479], None)
    lib.done(4)
    lib.step('r1/body/5', None)
    v_726563656970744072312f626f6479 = lib.preview('r1/body/5', v_72656275696c744072312f626f6479, v_746172676574)
    lib.done(5)

# Mechanically rendered experiment code; no host or syntax selection.
def r_7231(lib):
    v_63617073 = lib.input('caps')
    v_746172676574 = lib.input('target')
    v_776f7264 = lib.input('word')
    v_776f726473 = lib.input('words')
    lib.step('r1/body/0', None)
    v_73636f7265734072312f626f6479 = lib.calc('r1/body/0', 'Ints', 'identity', [lib.literal('Ints', [], 'r1/body/0/args/0/literal')], None)
    lib.done(0)
    lib.step('r1/body/1', None)
    for v_696e6465784072312f626f64792f31, v_774072312f626f64792f31 in lib.iterate('r1/body/1', v_776f726473, 'Word', False):
        lib.step('r1/body/1/body/0', v_696e6465784072312f626f64792f31)
        v_734072312f626f64792f312f626f6479 = lib.kernel('r1/body/1/body/0', 'Int', 'score', [v_774072312f626f64792f31], v_696e6465784072312f626f64792f31)
        lib.step('r1/body/1/body/1', v_696e6465784072312f626f64792f31)
        v_73636f7265734072312f626f6479 = lib.calc('r1/body/1/body/1', 'Ints', 'append', [v_73636f7265734072312f626f6479, v_734072312f626f64792f312f626f6479], v_696e6465784072312f626f64792f31)
    lib.done(1)
    lib.step('r1/body/2', None)
    v_694072312f626f6479 = lib.calc('r1/body/2', 'Int', 'argmin_first', [v_73636f7265734072312f626f6479], None)
    lib.done(2)
    lib.step('r1/body/3', None)
    v_63686f73656e4072312f626f6479 = lib.calc('r1/body/3', 'Word', 'index', [v_776f726473, v_694072312f626f6479], None)
    lib.done(3)
    lib.step('r1/body/4', None)
    lib.workspace_put('r1/body/4', 'selected_index', v_694072312f626f6479)
    lib.done(4)

def r_7232(lib):
    v_63617073 = lib.input('caps')
    v_746172676574 = lib.input('target')
    v_776f7264 = lib.input('word')
    v_776f726473 = lib.input('words')
    lib.step('r2/body/0', None)
    v_694072322f626f6479 = lib.workspace_get('r2/body/0', 'selected_index')
    lib.done(0)
    lib.step('r2/body/1', None)
    v_72656275696c744072322f626f6479 = lib.calc('r2/body/1', 'Word', 'index', [v_776f726473, v_694072322f626f6479], None)
    lib.done(1)
    lib.step('r2/body/2', None)
    v_726563656970744072322f626f6479 = lib.preview('r2/body/2', v_72656275696c744072322f626f6479, v_746172676574)
    lib.done(2)

# Mechanically rendered experiment code; no host or syntax selection.
def r_7231(lib):
    v_746172676574 = lib.input('target')
    v_776f7264 = lib.input('word')
    lib.step('r1/body/0', None)
    v_6e754072312f626f6479 = lib.kernel('r1/body/0', 'Ints', 'nu2', [], None)
    lib.done(0)
    lib.step('r1/body/1', None)
    v_636f6566664072312f626f6479 = lib.calc('r1/body/1', 'Ints', 'linear_solve_mod', [lib.literal('Matrix', [[1, 0], [0, 1]], 'r1/body/1/args/0/literal'), v_6e754072312f626f6479, lib.literal('Int', 2, 'r1/body/1/args/2/literal')], None)
    lib.done(1)
    lib.step('r1/body/2', None)
    v_636f7272656374696f6e4072312f626f6479 = lib.calc('r1/body/2', 'Word', 'combine_corrections', [lib.literal('Words', [['a'], ['b']], 'r1/body/2/args/0/literal'), v_636f6566664072312f626f6479], None)
    lib.done(2)
    lib.step('r1/body/3', None)
    v_726563656970744072312f626f6479 = lib.preview('r1/body/3', v_636f7272656374696f6e4072312f626f6479, v_746172676574)
    lib.done(3)

# Mechanically rendered experiment code; no host or syntax selection.
def r_7231(lib):
    v_746172676574 = lib.input('target')
    v_776f7264 = lib.input('word')
    lib.step('r1/body/0', None)
    v_6e754072312f626f6479 = lib.kernel('r1/body/0', 'Ints', 'nu5', [], None)
    lib.done(0)
    lib.step('r1/body/1', None)
    v_636f6566664072312f626f6479 = lib.calc('r1/body/1', 'Ints', 'linear_solve_mod', [lib.literal('Matrix', [[1, 0], [0, 1]], 'r1/body/1/args/0/literal'), v_6e754072312f626f6479, lib.literal('Int', 5, 'r1/body/1/args/2/literal')], None)
    lib.done(1)
    lib.step('r1/body/2', None)
    v_636f7272656374696f6e4072312f626f6479 = lib.calc('r1/body/2', 'Word', 'combine_corrections', [lib.literal('Words', [['c'], ['d']], 'r1/body/2/args/0/literal'), v_636f6566664072312f626f6479], None)
    lib.done(2)
    lib.step('r1/body/3', None)
    v_726563656970744072312f626f6479 = lib.preview('r1/body/3', v_636f7272656374696f6e4072312f626f6479, v_746172676574)
    lib.done(3)

# Mechanically rendered experiment code; no host or syntax selection.
def r_7231(lib):
    v_63617073 = lib.input('caps')
    v_746172676574 = lib.input('target')
    v_776f7264 = lib.input('word')
    v_776f726473 = lib.input('words')
    lib.step('r1/body/0', None)
    v_6e4072312f626f6479 = lib.kernel('r1/body/0', 'Ints', 'counts', [], None)
    lib.done(0)
    lib.step('r1/body/1', None)
    v_694072312f626f6479 = lib.calc('r1/body/1', 'Int', 'argmax_first', [v_6e4072312f626f6479], None)
    lib.done(1)
    lib.step('r1/body/2', None)
    v_774072312f626f6479 = lib.calc('r1/body/2', 'Word', 'index', [v_776f726473, v_694072312f626f6479], None)
    lib.done(2)
    lib.step('r1/body/3', None)
    lib.workspace_put('r1/body/3', 'saved', v_774072312f626f6479)
    lib.done(3)

def r_7232(lib):
    v_63617073 = lib.input('caps')
    v_746172676574 = lib.input('target')
    v_776f7264 = lib.input('word')
    v_776f726473 = lib.input('words')
    lib.step('r2/body/0', None)
    v_774072322f626f6479 = lib.workspace_get('r2/body/0', 'saved')
    lib.done(0)
    lib.step('r2/body/1', None)
    v_72656e616d65644072322f626f6479 = lib.calc('r2/body/1', 'Word', 'identity', [v_774072322f626f6479], None)
    lib.done(1)
    lib.step('r2/body/2', None)
    lib.workspace_put('r2/body/2', 'renamed', v_72656e616d65644072322f626f6479)
    lib.done(2)
    lib.step('r2/body/3', None)
    v_726563656970744072322f626f6479 = lib.preview('r2/body/3', v_72656e616d65644072322f626f6479, v_746172676574)
    lib.done(3)

# Mechanically rendered experiment code; no host or syntax selection.
def r_7231(lib):
    v_6c6567616379 = lib.input('legacy')
    v_746172676574 = lib.input('target')
    lib.step('r1/body/0', None)
    lib.workspace_put('r1/body/0', 'legacy', v_6c6567616379)
    lib.done(0)

def r_7232(lib):
    v_6c6567616379 = lib.input('legacy')
    v_746172676574 = lib.input('target')
    lib.step('r2/body/0', None)
    v_774072322f626f6479 = lib.workspace_get('r2/body/0', 'legacy')
    lib.done(0)
    lib.step('r2/body/1', None)
    v_726563656970744072322f626f6479 = lib.preview('r2/body/1', v_774072322f626f6479, v_746172676574)
    lib.done(1)

# Mechanically rendered experiment code; no host or syntax selection.
def r_7231(lib):
    v_746172676574 = lib.input('target')
    v_776f7264 = lib.input('word')
    lib.step('r1/body/0', None)
    v_6e4072312f626f6479 = lib.kernel('r1/body/0', 'Ints', 'counts', [], None)
    lib.done(0)
    lib.step('r1/body/1', None)
    v_6e304072312f626f6479 = lib.calc('r1/body/1', 'Int', 'index', [v_6e4072312f626f6479, lib.literal('Int', 0, 'r1/body/1/args/1/literal')], None)
    lib.done(1)
    lib.step('r1/body/2', None)
    v_746573744072312f626f6479 = lib.calc('r1/body/2', 'Bool', 'gt', [v_6e304072312f626f6479, lib.literal('Int', 0, 'r1/body/2/args/1/literal')], None)
    lib.done(2)
    lib.step('r1/body/3', None)
    if v_746573744072312f626f6479:
        lib.step('r1/body/3/then/0', None)
        v_726563656970744072312f626f6479 = lib.preview('r1/body/3/then/0', lib.literal('Word', ['a'], 'r1/body/3/then/0/word/literal'), v_746172676574)
    else:
        lib.step('r1/body/3/else/0', None)
        v_726563656970744072312f626f6479 = lib.preview('r1/body/3/else/0', lib.literal('Word', ['b'], 'r1/body/3/else/0/word/literal'), v_746172676574)
    lib.done(3)

# Mechanically rendered experiment code; no host or syntax selection.
def r_7231(lib):
    v_746172676574 = lib.input('target')
    v_776f7264 = lib.input('word')
    lib.step('r1/body/0', None)
    v_626974734072312f626f6479 = lib.kernel('r1/body/0', 'Ints', 'bits', [], None)
    lib.done(0)
    lib.step('r1/body/1', None)
    v_624072312f626f6479 = lib.calc('r1/body/1', 'Int', 'index', [v_626974734072312f626f6479, lib.literal('Int', 1, 'r1/body/1/args/1/literal')], None)
    lib.done(1)
    lib.step('r1/body/2', None)
    v_746573744072312f626f6479 = lib.calc('r1/body/2', 'Bool', 'eq', [v_624072312f626f6479, lib.literal('Int', 1, 'r1/body/2/args/1/literal')], None)
    lib.done(2)
    lib.step('r1/body/3', None)
    if v_746573744072312f626f6479:
        lib.step('r1/body/3/then/0', None)
        v_63686f6963654072312f626f6479 = lib.calc('r1/body/3/then/0', 'Word', 'identity', [lib.literal('Word', ['a'], 'r1/body/3/then/0/args/0/literal')], None)
    else:
        lib.step('r1/body/3/else/0', None)
        v_63686f6963654072312f626f6479 = lib.calc('r1/body/3/else/0', 'Word', 'identity', [lib.literal('Word', ['b'], 'r1/body/3/else/0/args/0/literal')], None)
    lib.done(3)
    lib.step('r1/body/4', None)
    v_726563656970744072312f626f6479 = lib.preview('r1/body/4', v_63686f6963654072312f626f6479, v_746172676574)
    lib.done(4)

# Mechanically rendered experiment code; no host or syntax selection.
def m_73656c6563746f72(lib):
    v_746172676574 = lib.input('target')
    v_776f7264 = lib.input('word')
    lib.step('modules/selector/body/0', None)
    v_6e406d6f64756c65732f73656c6563746f722f626f6479 = lib.kernel('modules/selector/body/0', 'Ints', 'nu2', [], None)
    lib.step('modules/selector/body/1', None)
    v_76406d6f64756c65732f73656c6563746f722f626f6479 = lib.calc('modules/selector/body/1', 'Int', 'index', [v_6e406d6f64756c65732f73656c6563746f722f626f6479, lib.literal('Int', 0, 'modules/selector/body/1/args/1/literal')], None)
    lib.step('modules/selector/body/2', None)
    v_62406d6f64756c65732f73656c6563746f722f626f6479 = lib.calc('modules/selector/body/2', 'Bool', 'eq', [v_76406d6f64756c65732f73656c6563746f722f626f6479, lib.literal('Int', 1, 'modules/selector/body/2/args/1/literal')], None)
    lib.step('modules/selector/body/3', None)
    if v_62406d6f64756c65732f73656c6563746f722f626f6479:
        lib.step('modules/selector/body/3/then/0', None)
        return lib.result('modules/selector/body/3/then/0', lib.literal('Word', ['a'], 'modules/selector/body/3/then/0/literal'), False)
    else:
        lib.step('modules/selector/body/3/else/0', None)
        return lib.result('modules/selector/body/3/else/0', lib.literal('Word', ['b'], 'modules/selector/body/3/else/0/literal'), False)

def r_7231(lib):
    v_746172676574 = lib.input('target')
    v_776f7264 = lib.input('word')
    lib.step('r1/body/0', None)
    v_774072312f626f6479 = lib.call('r1/body/0', 'Word', m_73656c6563746f72, [])
    lib.done(0)
    lib.step('r1/body/1', None)
    v_726563656970744072312f626f6479 = lib.preview('r1/body/1', v_774072312f626f6479, v_746172676574)
    lib.done(1)

# Mechanically rendered experiment code; no host or syntax selection.
def m_636f7079(lib, v_78):
    v_746172676574 = lib.input('target')
    v_776f7264 = lib.input('word')
    lib.step('modules/copy/body/0', None)
    return lib.result('modules/copy/body/0', v_78, False)

def m_73656c6563746f72(lib):
    v_746172676574 = lib.input('target')
    v_776f7264 = lib.input('word')
    lib.step('modules/selector/body/0', None)
    v_6e406d6f64756c65732f73656c6563746f722f626f6479 = lib.kernel('modules/selector/body/0', 'Ints', 'nu2', [], None)
    lib.step('modules/selector/body/1', None)
    v_76406d6f64756c65732f73656c6563746f722f626f6479 = lib.calc('modules/selector/body/1', 'Int', 'index', [v_6e406d6f64756c65732f73656c6563746f722f626f6479, lib.literal('Int', 0, 'modules/selector/body/1/args/1/literal')], None)
    lib.step('modules/selector/body/2', None)
    v_62406d6f64756c65732f73656c6563746f722f626f6479 = lib.calc('modules/selector/body/2', 'Bool', 'eq', [v_76406d6f64756c65732f73656c6563746f722f626f6479, lib.literal('Int', 1, 'modules/selector/body/2/args/1/literal')], None)
    lib.step('modules/selector/body/3', None)
    if v_62406d6f64756c65732f73656c6563746f722f626f6479:
        lib.step('modules/selector/body/3/then/0', None)
        return lib.result('modules/selector/body/3/then/0', lib.literal('Word', ['a'], 'modules/selector/body/3/then/0/literal'), False)
    else:
        lib.step('modules/selector/body/3/else/0', None)
        return lib.result('modules/selector/body/3/else/0', lib.literal('Word', ['b'], 'modules/selector/body/3/else/0/literal'), False)

def r_7231(lib):
    v_746172676574 = lib.input('target')
    v_776f7264 = lib.input('word')
    lib.step('r1/body/0', None)
    v_73656c65637465644072312f626f6479 = lib.call('r1/body/0', 'Word', m_73656c6563746f72, [])
    lib.done(0)
    lib.step('r1/body/1', None)
    v_636f706965644072312f626f6479 = lib.call('r1/body/1', 'Word', m_636f7079, [v_73656c65637465644072312f626f6479])
    lib.done(1)
    lib.step('r1/body/2', None)
    v_726563656970744072312f626f6479 = lib.preview('r1/body/2', v_636f706965644072312f626f6479, v_746172676574)
    lib.done(2)

# Mechanically rendered experiment code; no host or syntax selection.
def r_7231(lib):
    v_63617073 = lib.input('caps')
    v_746172676574 = lib.input('target')
    v_776f7264 = lib.input('word')
    v_776f726473 = lib.input('words')
    lib.step('r1/body/0', None)
    for v_696e6465784072312f626f64792f30, v_774072312f626f64792f30 in lib.iterate('r1/body/0', v_776f726473, 'Word', True):
        lib.step('r1/body/0/body/0', v_696e6465784072312f626f64792f30)
        v_726563656970744072312f626f64792f302f626f6479 = lib.preview('r1/body/0/body/0', v_774072312f626f64792f30, v_746172676574)
        lib.step('r1/body/0/body/1', v_696e6465784072312f626f64792f30)
        v_734072312f626f64792f302f626f6479 = lib.kernel('r1/body/0/body/1', 'Int', 'score', [v_774072312f626f64792f30], v_696e6465784072312f626f64792f30)
        lib.step('r1/body/0/body/2', v_696e6465784072312f626f64792f30)
        v_646f6e654072312f626f64792f302f626f6479 = lib.calc('r1/body/0/body/2', 'Bool', 'eq', [v_734072312f626f64792f302f626f6479, lib.literal('Int', 0, 'r1/body/0/body/2/args/1/literal')], v_696e6465784072312f626f64792f30)
        lib.step('r1/body/0/body/3', v_696e6465784072312f626f64792f30)
        if v_646f6e654072312f626f64792f302f626f6479:
            lib.step('r1/body/0/body/3/then/0', v_696e6465784072312f626f64792f30)
            lib.stop('r1/body/0/body/3/then/0', v_696e6465784072312f626f64792f30)
        else:
            pass

# Mechanically rendered experiment code; no host or syntax selection.
def r_7231(lib):
    v_746172676574 = lib.input('target')
    v_776f7264 = lib.input('word')
    lib.step('r1/body/0', None)
    v_626974734072312f626f6479 = lib.kernel('r1/body/0', 'Ints', 'bits', [], None)
    lib.done(0)
    lib.step('r1/body/1', None)
    v_624072312f626f6479 = lib.calc('r1/body/1', 'Int', 'index', [v_626974734072312f626f6479, lib.literal('Int', 0, 'r1/body/1/args/1/literal')], None)
    lib.done(1)
    lib.step('r1/body/2', None)
    v_746573744072312f626f6479 = lib.calc('r1/body/2', 'Bool', 'eq', [v_624072312f626f6479, lib.literal('Int', 1, 'r1/body/2/args/1/literal')], None)
    lib.done(2)
    lib.step('r1/body/3', None)
    if v_746573744072312f626f6479:
        lib.step('r1/body/3/then/0', None)
        lib.stop('r1/body/3/then/0', None)
    else:
        pass
    lib.done(3)
    lib.step('r1/body/4', None)
    v_726563656970744072312f626f6479 = lib.preview('r1/body/4', v_776f7264, v_746172676574)
    lib.done(4)

# Mechanically rendered experiment code; no host or syntax selection.
def r_7231(lib):
    v_746172676574 = lib.input('target')
    v_776f7264 = lib.input('word')
    lib.step('r1/body/0', None)
    v_726563656970744072312f626f6479 = lib.preview('r1/body/0', lib.literal('Word', ['d'], 'r1/body/0/word/literal'), v_746172676574)
    lib.done(0)
    lib.step('r1/body/1', None)
    v_6f7574636f6d654072312f626f6479 = lib.kernel('r1/body/1', 'Record', 'preview_outcome', [v_726563656970744072312f626f6479], None)
    lib.done(1)
    lib.step('r1/body/2', None)
    v_6669656c644072312f626f6479 = lib.calc('r1/body/2', 'Text', 'field', [v_6f7574636f6d654072312f626f6479, lib.literal('Text', 'code', 'r1/body/2/args/1/literal')], None)
    lib.done(2)
    lib.step('r1/body/3', None)
    v_746573744072312f626f6479 = lib.calc('r1/body/3', 'Bool', 'eq', [v_6669656c644072312f626f6479, lib.literal('Text', 'KN2:protection.piece_would_move', 'r1/body/3/args/1/literal')], None)
    lib.done(3)
    lib.step('r1/body/4', None)
    if v_746573744072312f626f6479:
        lib.step('r1/body/4/then/0', None)
        v_726563656970744072312f626f6479 = lib.preview('r1/body/4/then/0', lib.literal('Word', ['a'], 'r1/body/4/then/0/word/literal'), v_746172676574)
    else:
        lib.step('r1/body/4/else/0', None)
        v_726563656970744072312f626f6479 = lib.preview('r1/body/4/else/0', lib.literal('Word', ['b'], 'r1/body/4/else/0/word/literal'), v_746172676574)
    lib.done(4)

# Mechanically rendered experiment code; no host or syntax selection.
def r_7231(lib):
    v_746172676574 = lib.input('target')
    v_776f7264 = lib.input('word')
    lib.step('r1/body/0', None)
    v_726563656970744072312f626f6479 = lib.preview('r1/body/0', lib.literal('Word', ['c'], 'r1/body/0/word/literal'), v_746172676574)
    lib.done(0)
    lib.step('r1/body/1', None)
    v_6f7574636f6d654072312f626f6479 = lib.kernel('r1/body/1', 'Record', 'preview_outcome', [v_726563656970744072312f626f6479], None)
    lib.done(1)
    lib.step('r1/body/2', None)
    v_6669656c644072312f626f6479 = lib.calc('r1/body/2', 'Bool', 'field', [v_6f7574636f6d654072312f626f6479, lib.literal('Text', 'protected', 'r1/body/2/args/1/literal')], None)
    lib.done(2)
    lib.step('r1/body/3', None)
    v_746573744072312f626f6479 = lib.calc('r1/body/3', 'Bool', 'eq', [v_6669656c644072312f626f6479, lib.literal('Bool', False, 'r1/body/3/args/1/literal')], None)
    lib.done(3)
    lib.step('r1/body/4', None)
    if v_746573744072312f626f6479:
        lib.step('r1/body/4/then/0', None)
        v_726563656970744072312f626f6479 = lib.preview('r1/body/4/then/0', lib.literal('Word', ['a'], 'r1/body/4/then/0/word/literal'), v_746172676574)
    else:
        lib.step('r1/body/4/else/0', None)
        v_726563656970744072312f626f6479 = lib.preview('r1/body/4/else/0', lib.literal('Word', ['b'], 'r1/body/4/else/0/word/literal'), v_746172676574)
    lib.done(4)

# Mechanically rendered experiment code; no host or syntax selection.
def r_7231(lib):
    v_63617073 = lib.input('caps')
    v_7069636b30 = lib.input('pick0')
    v_746172676574 = lib.input('target')
    v_776f7264 = lib.input('word')
    v_776f726473 = lib.input('words')
    lib.step('r1/body/0', None)
    v_6e4072312f626f6479 = lib.kernel('r1/body/0', 'Ints', 'counts', [], None)
    lib.done(0)
    lib.step('r1/body/1', None)
    v_694072312f626f6479 = lib.calc('r1/body/1', 'Int', 'argmax_first', [v_6e4072312f626f6479], None)
    lib.done(1)
    lib.step('r1/body/2', None)
    v_744072312f626f6479 = lib.calc('r1/body/2', 'Target', 'index', [v_63617073, v_694072312f626f6479], None)
    lib.done(2)
    lib.step('r1/body/3', None)
    v_6f6e654072312f626f6479 = lib.calc('r1/body/3', 'Targets', 'singleton', [v_744072312f626f6479], None)
    lib.done(3)
    lib.step('r1/body/4', None)
    v_63686f73656e4072312f626f6479 = lib.pick('r1/body/4', 'Target', v_6f6e654072312f626f6479, 'pick0')
    lib.done(4)
    lib.step('r1/body/5', None)
    v_726563656970744072312f626f6479 = lib.preview('r1/body/5', v_776f7264, v_63686f73656e4072312f626f6479)
    lib.done(5)

# Mechanically rendered experiment code; no host or syntax selection.
def r_7231(lib):
    v_7069636b30 = lib.input('pick0')
    v_746172676574 = lib.input('target')
    lib.step('r1/body/0', None)
    v_6e754072312f626f6479 = lib.kernel('r1/body/0', 'Ints', 'nu2', [], None)
    lib.done(0)
    lib.step('r1/body/1', None)
    v_774072312f626f6479 = lib.calc('r1/body/1', 'Word', 'combine_corrections', [lib.literal('Words', [['a'], ['b']], 'r1/body/1/args/0/literal'), v_6e754072312f626f6479], None)
    lib.done(1)
    lib.step('r1/body/2', None)
    v_6f6e654072312f626f6479 = lib.calc('r1/body/2', 'Words', 'singleton', [v_774072312f626f6479], None)
    lib.done(2)
    lib.step('r1/body/3', None)
    v_63686f73656e4072312f626f6479 = lib.pick('r1/body/3', 'Word', v_6f6e654072312f626f6479, 'pick0')
    lib.done(3)
    lib.step('r1/body/4', None)
    v_726563656970744072312f626f6479 = lib.preview('r1/body/4', v_63686f73656e4072312f626f6479, v_746172676574)
    lib.done(4)

# Mechanically rendered experiment code; no host or syntax selection.
def r_7231(lib):
    v_746172676574 = lib.input('target')
    v_776f7264 = lib.input('word')
    lib.step('r1/body/0', None)
    v_726563656970744072312f626f6479 = lib.preview('r1/body/0', lib.literal('Word', ['a'], 'r1/body/0/word/literal'), v_746172676574)
    lib.done(0)
    lib.step('r1/body/1', None)
    v_626974734072312f626f6479 = lib.kernel('r1/body/1', 'Ints', 'bits', [], None)
    lib.done(1)
    lib.step('r1/body/2', None)
    v_6269744072312f626f6479 = lib.calc('r1/body/2', 'Int', 'index', [v_626974734072312f626f6479, lib.literal('Int', 0, 'r1/body/2/args/1/literal')], None)
    lib.done(2)
    lib.step('r1/body/3', None)
    v_746573744072312f626f6479 = lib.calc('r1/body/3', 'Bool', 'eq', [v_6269744072312f626f6479, lib.literal('Int', 1, 'r1/body/3/args/1/literal')], None)
    lib.done(3)
    lib.step('r1/body/4', None)
    if v_746573744072312f626f6479:
        lib.step('r1/body/4/then/0', None)
        lib.stop('r1/body/4/then/0', None)
    else:
        pass
    lib.done(4)
    lib.step('r1/body/5', None)
    v_726563656970744072312f626f6479 = lib.preview('r1/body/5', lib.literal('Word', ['b'], 'r1/body/5/word/literal'), v_746172676574)
    lib.done(5)

def r_7232(lib):
    v_746172676574 = lib.input('target')
    v_776f7264 = lib.input('word')
    lib.step('r2/body/0', None)
    v_726563656970744072322f626f6479 = lib.preview('r2/body/0', lib.literal('Word', ['a'], 'r2/body/0/word/literal'), v_746172676574)
    lib.done(0)
    lib.step('r2/body/1', None)
    v_626974734072322f626f6479 = lib.kernel('r2/body/1', 'Ints', 'bits', [], None)
    lib.done(1)
    lib.step('r2/body/2', None)
    v_6269744072322f626f6479 = lib.calc('r2/body/2', 'Int', 'index', [v_626974734072322f626f6479, lib.literal('Int', 1, 'r2/body/2/args/1/literal')], None)
    lib.done(2)
    lib.step('r2/body/3', None)
    v_746573744072322f626f6479 = lib.calc('r2/body/3', 'Bool', 'eq', [v_6269744072322f626f6479, lib.literal('Int', 1, 'r2/body/3/args/1/literal')], None)
    lib.done(3)
    lib.step('r2/body/4', None)
    if v_746573744072322f626f6479:
        lib.step('r2/body/4/then/0', None)
        lib.stop('r2/body/4/then/0', None)
    else:
        pass
    lib.done(4)
    lib.step('r2/body/5', None)
    v_726563656970744072322f626f6479 = lib.preview('r2/body/5', lib.literal('Word', ['b'], 'r2/body/5/word/literal'), v_746172676574)
    lib.done(5)

def r_7233(lib):
    v_746172676574 = lib.input('target')
    v_776f7264 = lib.input('word')
    lib.step('r3/body/0', None)
    v_726563656970744072332f626f6479 = lib.preview('r3/body/0', lib.literal('Word', ['a'], 'r3/body/0/word/literal'), v_746172676574)
    lib.done(0)
    lib.step('r3/body/1', None)
    v_626974734072332f626f6479 = lib.kernel('r3/body/1', 'Ints', 'bits', [], None)
    lib.done(1)
    lib.step('r3/body/2', None)
    v_6269744072332f626f6479 = lib.calc('r3/body/2', 'Int', 'index', [v_626974734072332f626f6479, lib.literal('Int', 2, 'r3/body/2/args/1/literal')], None)
    lib.done(2)
    lib.step('r3/body/3', None)
    v_746573744072332f626f6479 = lib.calc('r3/body/3', 'Bool', 'eq', [v_6269744072332f626f6479, lib.literal('Int', 1, 'r3/body/3/args/1/literal')], None)
    lib.done(3)
    lib.step('r3/body/4', None)
    if v_746573744072332f626f6479:
        lib.step('r3/body/4/then/0', None)
        lib.stop('r3/body/4/then/0', None)
    else:
        pass
    lib.done(4)
    lib.step('r3/body/5', None)
    v_726563656970744072332f626f6479 = lib.preview('r3/body/5', lib.literal('Word', ['b'], 'r3/body/5/word/literal'), v_746172676574)
    lib.done(5)

# Mechanically rendered experiment code; no host or syntax selection.
def r_7231(lib):
    v_746172676574 = lib.input('target')
    v_776f7264 = lib.input('word')
    lib.step('r1/body/0', None)
    v_626974734072312f626f6479 = lib.kernel('r1/body/0', 'Ints', 'bits', [], None)
    lib.done(0)
    lib.step('r1/body/1', None)
    v_6269744072312f626f6479 = lib.calc('r1/body/1', 'Int', 'index', [v_626974734072312f626f6479, lib.literal('Int', 0, 'r1/body/1/args/1/literal')], None)
    lib.done(1)
    lib.step('r1/body/2', None)
    v_746573744072312f626f6479 = lib.calc('r1/body/2', 'Bool', 'eq', [v_6269744072312f626f6479, lib.literal('Int', 1, 'r1/body/2/args/1/literal')], None)
    lib.done(2)
    lib.step('r1/body/3', None)
    if v_746573744072312f626f6479:
        lib.step('r1/body/3/then/0', None)
        lib.stop('r1/body/3/then/0', None)
    else:
        pass
    lib.done(3)
    lib.step('r1/body/4', None)
    v_726563656970744072312f626f6479 = lib.preview('r1/body/4', lib.literal('Word', ['b'], 'r1/body/4/word/literal'), v_746172676574)
    lib.done(4)

def r_7232(lib):
    v_746172676574 = lib.input('target')
    v_776f7264 = lib.input('word')
    lib.step('r2/body/0', None)
    v_626974734072322f626f6479 = lib.kernel('r2/body/0', 'Ints', 'bits', [], None)
    lib.done(0)
    lib.step('r2/body/1', None)
    v_6269744072322f626f6479 = lib.calc('r2/body/1', 'Int', 'index', [v_626974734072322f626f6479, lib.literal('Int', 1, 'r2/body/1/args/1/literal')], None)
    lib.done(1)
    lib.step('r2/body/2', None)
    v_746573744072322f626f6479 = lib.calc('r2/body/2', 'Bool', 'eq', [v_6269744072322f626f6479, lib.literal('Int', 1, 'r2/body/2/args/1/literal')], None)
    lib.done(2)
    lib.step('r2/body/3', None)
    if v_746573744072322f626f6479:
        lib.step('r2/body/3/then/0', None)
        lib.stop('r2/body/3/then/0', None)
    else:
        pass
    lib.done(3)
    lib.step('r2/body/4', None)
    v_726563656970744072322f626f6479 = lib.preview('r2/body/4', lib.literal('Word', ['b'], 'r2/body/4/word/literal'), v_746172676574)
    lib.done(4)

def r_7233(lib):
    v_746172676574 = lib.input('target')
    v_776f7264 = lib.input('word')
    lib.step('r3/body/0', None)
    v_626974734072332f626f6479 = lib.kernel('r3/body/0', 'Ints', 'bits', [], None)
    lib.done(0)
    lib.step('r3/body/1', None)
    v_6269744072332f626f6479 = lib.calc('r3/body/1', 'Int', 'index', [v_626974734072332f626f6479, lib.literal('Int', 2, 'r3/body/1/args/1/literal')], None)
    lib.done(1)
    lib.step('r3/body/2', None)
    v_746573744072332f626f6479 = lib.calc('r3/body/2', 'Bool', 'eq', [v_6269744072332f626f6479, lib.literal('Int', 1, 'r3/body/2/args/1/literal')], None)
    lib.done(2)
    lib.step('r3/body/3', None)
    if v_746573744072332f626f6479:
        lib.step('r3/body/3/then/0', None)
        lib.stop('r3/body/3/then/0', None)
    else:
        pass
    lib.done(3)
    lib.step('r3/body/4', None)
    v_726563656970744072332f626f6479 = lib.preview('r3/body/4', lib.literal('Word', ['b'], 'r3/body/4/word/literal'), v_746172676574)
    lib.done(4)

# Mechanically rendered experiment code; no host or syntax selection.
def r_7231(lib):
    v_746172676574 = lib.input('target')
    v_776f7264 = lib.input('word')
    lib.step('r1/body/0', None)
    v_764072312f626f6479 = lib.kernel('r1/body/0', 'Ints', 'counts', [], None)
    lib.done(0)
    lib.step('r1/body/1', None)
    return lib.result('r1/body/1', v_764072312f626f6479, True)

# Mechanically rendered experiment code; no host or syntax selection.
def r_7231(lib):
    v_746172676574 = lib.input('target')
    v_776f7264 = lib.input('word')
    lib.step('r1/body/0', None)
    v_764072312f626f6479 = lib.kernel('r1/body/0', 'Ints', 'floors', [], None)
    lib.done(0)
    lib.step('r1/body/1', None)
    return lib.result('r1/body/1', v_764072312f626f6479, True)

# Mechanically rendered experiment code; no host or syntax selection.
def r_7231(lib):
    v_746172676574 = lib.input('target')
    v_776f7264 = lib.input('word')
    lib.step('r1/body/0', None)
    v_764072312f626f6479 = lib.kernel('r1/body/0', 'Bools', 'blocked', [], None)
    lib.done(0)
    lib.step('r1/body/1', None)
    return lib.result('r1/body/1', v_764072312f626f6479, True)

# Mechanically rendered experiment code; no host or syntax selection.
def r_7231(lib):
    v_746172676574 = lib.input('target')
    v_776f7264 = lib.input('word')
    lib.step('r1/body/0', None)
    v_764072312f626f6479 = lib.kernel('r1/body/0', 'State', 'snapshot', [], None)
    lib.done(0)
    lib.step('r1/body/1', None)
    return lib.result('r1/body/1', v_764072312f626f6479, True)

# Mechanically rendered experiment code; no host or syntax selection.
def r_7231(lib):
    v_746172676574 = lib.input('target')
    v_776f7264 = lib.input('word')
    lib.step('r1/body/0', None)
    v_6e4072312f626f6479 = lib.kernel('r1/body/0', 'Ints', 'counts', [], None)
    lib.done(0)
    lib.step('r1/body/1', None)
    v_726f77734072312f626f6479 = lib.calc('r1/body/1', 'Rows', 'zip', [lib.literal('Targets', ['c0', 'c1', 'c2'], 'r1/body/1/args/0/literal'), v_6e4072312f626f6479], None)
    lib.done(1)
    lib.step('r1/body/2', None)
    return lib.result('r1/body/2', v_726f77734072312f626f6479, True)

# Mechanically rendered experiment code; no host or syntax selection.
def r_7231(lib):
    v_746172676574 = lib.input('target')
    v_776f7264 = lib.input('word')
    lib.step('r1/body/0', None)
    v_6e4072312f626f6479 = lib.kernel('r1/body/0', 'Ints', 'counts', [], None)
    lib.done(0)
    lib.step('r1/body/1', None)
    v_73756d4072312f626f6479 = lib.calc('r1/body/1', 'Int', 'sum', [v_6e4072312f626f6479], None)
    lib.done(1)
    lib.step('r1/body/2', None)
    return lib.result('r1/body/2', v_73756d4072312f626f6479, True)

# Mechanically rendered experiment code; no host or syntax selection.
def r_7231(lib):
    v_746172676574 = lib.input('target')
    v_776f7264 = lib.input('word')
    lib.step('r1/body/0', None)
    v_664072312f626f6479 = lib.kernel('r1/body/0', 'Ints', 'floors', [], None)
    lib.done(0)
    lib.step('r1/body/1', None)
    v_7a4072312f626f6479 = lib.calc('r1/body/1', 'Int', 'count_equal', [v_664072312f626f6479, lib.literal('Int', 0, 'r1/body/1/args/1/literal')], None)
    lib.done(1)
    lib.step('r1/body/2', None)
    return lib.result('r1/body/2', v_7a4072312f626f6479, True)

# Mechanically rendered experiment code; no host or syntax selection.
def r_7231(lib):
    v_776f7264 = lib.input('word')
    lib.step('r1/body/0', None)
    v_6e4072312f626f6479 = lib.calc('r1/body/0', 'Int', 'length', [v_776f7264], None)
    lib.done(0)
    lib.step('r1/body/1', None)
    return lib.result('r1/body/1', v_6e4072312f626f6479, True)

# Mechanically rendered experiment code; no host or syntax selection.
def r_7231(lib):
    v_746172676574 = lib.input('target')
    v_776f7264 = lib.input('word')
    lib.step('r1/body/0', None)
    v_6e4072312f626f6479 = lib.calc('r1/body/0', 'Int', 'add', [lib.literal('Int', 9007199254740993, 'r1/body/0/args/0/literal'), lib.literal('Int', 7, 'r1/body/0/args/1/literal')], None)
    lib.done(0)
    lib.step('r1/body/1', None)
    return lib.result('r1/body/1', v_6e4072312f626f6479, True)

# Mechanically rendered experiment code; no host or syntax selection.
def r_7231(lib):
    v_746172676574 = lib.input('target')
    v_776f7264 = lib.input('word')
    lib.step('r1/body/0', None)
    v_6e4072312f626f6479 = lib.calc('r1/body/0', 'Int', 'mod_dot', [lib.literal('Ints', [1, 2], 'r1/body/0/args/0/literal'), lib.literal('Ints', [3, 1], 'r1/body/0/args/1/literal'), lib.literal('Int', 5, 'r1/body/0/args/2/literal')], None)
    lib.done(0)
    lib.step('r1/body/1', None)
    return lib.result('r1/body/1', v_6e4072312f626f6479, True)

# Mechanically rendered experiment code; no host or syntax selection.
def r_7231(lib):
    v_746172676574 = lib.input('target')
    v_776f7264 = lib.input('word')
    lib.step('r1/body/0', None)
    v_764072312f626f6479 = lib.kernel('r1/body/0', 'Ints', 'counts', [], None)
    lib.done(0)
    lib.step('r1/body/1', None)
    v_636f6f7264734072312f626f6479 = lib.calc('r1/body/1', 'Rows', 'indexed_points', [v_764072312f626f6479], None)
    lib.done(1)
    lib.step('r1/body/2', None)
    lib.publish('r1/body/2', lib.literal('Record', {'exact_properties': {'values': [2, 6, 6]}, 'kind': 'points', 'object_ids': ['c0', 'c1', 'c2'], 'order': [0, 1, 2]}, 'r1/body/2/exact/literal'), v_636f6f7264734072312f626f6479)
    lib.done(2)
    lib.step('r1/body/3', None)
    return lib.result('r1/body/3', lib.literal('Text', 'published', 'r1/body/3/literal'), True)

# Mechanically rendered experiment code; no host or syntax selection.
def r_7231(lib):
    v_746172676574 = lib.input('target')
    v_776f7264 = lib.input('word')
    lib.step('r1/body/0', None)
    v_764072312f626f6479 = lib.kernel('r1/body/0', 'Ints', 'floors', [], None)
    lib.done(0)
    lib.step('r1/body/1', None)
    v_636f6f7264734072312f626f6479 = lib.calc('r1/body/1', 'Rows', 'indexed_points', [v_764072312f626f6479], None)
    lib.done(1)
    lib.step('r1/body/2', None)
    lib.publish('r1/body/2', lib.literal('Record', {'exact_properties': {'values': [0, 2, 2]}, 'kind': 'points', 'object_ids': ['c0', 'c1', 'c2'], 'order': [0, 1, 2]}, 'r1/body/2/exact/literal'), v_636f6f7264734072312f626f6479)
    lib.done(2)
    lib.step('r1/body/3', None)
    return lib.result('r1/body/3', lib.literal('Text', 'published', 'r1/body/3/literal'), True)

# Mechanically rendered experiment code; no host or syntax selection.
def r_7231(lib):
    v_746172676574 = lib.input('target')
    lib.step('r1/body/0', None)
    v_6e4072312f626f6479 = lib.kernel('r1/body/0', 'Ints', 'counts', [], None)
    lib.done(0)
    lib.step('r1/body/1', None)
    v_764072312f626f6479 = lib.calc('r1/body/1', 'Int', 'index', [v_6e4072312f626f6479, lib.literal('Int', 1, 'r1/body/1/args/1/literal')], None)
    lib.done(1)
    lib.step('r1/body/2', None)
    v_78794072312f626f6479 = lib.calc('r1/body/2', 'Rows', 'point', [lib.literal('Int', 1, 'r1/body/2/args/0/literal'), v_764072312f626f6479], None)
    lib.done(2)
    lib.step('r1/body/3', None)
    lib.publish('r1/body/3', lib.literal('Record', {'exact_properties': {'value': 6}, 'kind': 'points', 'object_ids': ['c1'], 'order': [0]}, 'r1/body/3/exact/literal'), v_78794072312f626f6479)
    lib.done(3)
    lib.step('r1/body/4', None)
    return lib.result('r1/body/4', lib.literal('Text', 'published', 'r1/body/4/literal'), True)

# Mechanically rendered experiment code; no host or syntax selection.
def r_7231(lib):
    v_746172676574 = lib.input('target')
    v_776f7264 = lib.input('word')
    lib.step('r1/body/0', None)
    v_636865636b4072312f626f6479 = lib.kernel('r1/body/0', 'Record', 'word_check', [v_776f7264], None)
    lib.done(0)
    lib.step('r1/body/1', None)
    lib.publish('r1/body/1', lib.literal('Record', {'exact_properties': {'word': ['a']}, 'kind': 'word_path', 'object_ids': ['slot0', 'slot1'], 'order': [0, 1]}, 'r1/body/1/exact/literal'), lib.literal('Rows', [[0, 1], [1, 0]], 'r1/body/1/coordinates/literal'))
    lib.done(1)
    lib.step('r1/body/2', None)
    return lib.result('r1/body/2', lib.literal('Text', 'published', 'r1/body/2/literal'), True)

# Mechanically rendered experiment code; no host or syntax selection.
def r_7231(lib):
    v_746172676574 = lib.input('target')
    v_776f7264 = lib.input('word')
    lib.step('r1/body/0', None)
    lib.publish('r1/body/0', lib.literal('Record', {'exact_properties': {'fraction': {'denominator': 3, 'numerator': 1}}, 'kind': 'reference', 'object_ids': ['origin', 'third'], 'order': [0, 1]}, 'r1/body/0/exact/literal'), lib.literal('Rows', [[0, 0], [{'denominator': 3, 'numerator': 1}, 1]], 'r1/body/0/coordinates/literal'))
    lib.done(0)
    lib.step('r1/body/1', None)
    return lib.result('r1/body/1', lib.literal('Text', 'published', 'r1/body/1/literal'), True)

# Mechanically rendered experiment code; no host or syntax selection.
def r_7231(lib):
    v_746172676574 = lib.input('target')
    v_776f7264 = lib.input('word')
    lib.step('r1/body/0', None)
    v_636865636b4072312f626f6479 = lib.kernel('r1/body/0', 'Record', 'word_check', [v_776f7264], None)
    lib.done(0)
    lib.step('r1/body/1', None)
    return lib.result('r1/body/1', v_636865636b4072312f626f6479, True)

# Mechanically rendered experiment code; no host or syntax selection.
def r_7231(lib):
    v_776f7264 = lib.input('word')
    lib.step('r1/body/0', None)
    v_696e76657273654072312f626f6479 = lib.calc('r1/body/0', 'Word', 'inverse', [v_776f7264], None)
    lib.done(0)
    lib.step('r1/body/1', None)
    return lib.result('r1/body/1', v_696e76657273654072312f626f6479, True)

# Mechanically rendered experiment code; no host or syntax selection.
def r_7231(lib):
    v_6c656674 = lib.input('left')
    v_7269676874 = lib.input('right')
    lib.step('r1/body/0', None)
    v_636f6d62696e65644072312f626f6479 = lib.calc('r1/body/0', 'Word', 'concat', [v_6c656674, v_7269676874], None)
    lib.done(0)
    lib.step('r1/body/1', None)
    return lib.result('r1/body/1', v_636f6d62696e65644072312f626f6479, True)

# Mechanically rendered experiment code; no host or syntax selection.
def r_7231(lib):
    v_776f7264 = lib.input('word')
    lib.step('r1/body/0', None)
    v_636865636b4072312f626f6479 = lib.kernel('r1/body/0', 'Record', 'word_check', [v_776f7264], None)
    lib.done(0)
    lib.step('r1/body/1', None)
    return lib.result('r1/body/1', v_636865636b4072312f626f6479, True)

# Mechanically rendered experiment code; no host or syntax selection.
def r_7231(lib):
    v_746172676574 = lib.input('target')
    v_776f7264 = lib.input('word')
    lib.step('r1/body/0', None)
    return lib.result('r1/body/0', lib.literal('Text', 'Named setup: c1; labels: é / é / ν.', 'r1/body/0/literal'), True)

# Mechanically rendered experiment code; no host or syntax selection.
def r_7231(lib):
    v_776f7264 = lib.input('word')
    lib.step('r1/body/0', None)
    v_636865636b4072312f626f6479 = lib.kernel('r1/body/0', 'Record', 'word_check', [v_776f7264], None)
    lib.done(0)
    lib.step('r1/body/1', None)
    return lib.result('r1/body/1', v_636865636b4072312f626f6479, True)

# Mechanically rendered experiment code; no host or syntax selection.
def r_7231(lib):
    v_776f7264 = lib.input('word')
    lib.step('r1/body/0', None)
    v_636865636b4072312f626f6479 = lib.kernel('r1/body/0', 'Record', 'word_check', [v_776f7264], None)
    lib.done(0)
    lib.step('r1/body/1', None)
    return lib.result('r1/body/1', v_636865636b4072312f626f6479, True)

# Mechanically rendered experiment code; no host or syntax selection.
def r_7231(lib):
    v_776f7264 = lib.input('word')
    lib.step('r1/body/0', None)
    v_636865636b4072312f626f6479 = lib.kernel('r1/body/0', 'Record', 'word_check', [v_776f7264], None)
    lib.done(0)
    lib.step('r1/body/1', None)
    return lib.result('r1/body/1', v_636865636b4072312f626f6479, True)

# Mechanically rendered experiment code; no host or syntax selection.
def r_7231(lib):
    v_746172676574 = lib.input('target')
    v_776f7264 = lib.input('word')
    lib.step('r1/body/0', None)
    v_726563656970744072312f626f6479 = lib.preview('r1/body/0', v_776f7264, v_746172676574)
    lib.done(0)
    lib.step('r1/body/1', None)
    return lib.result('r1/body/1', lib.literal('Text', 'requested', 'r1/body/1/literal'), True)

# Mechanically rendered experiment code; no host or syntax selection.
def r_7231(lib):
    v_746172676574 = lib.input('target')
    v_776f7264 = lib.input('word')
    lib.step('r1/body/0', None)
    v_6e4072312f626f6479 = lib.kernel('r1/body/0', 'Ints', 'counts', [], None)
    lib.done(0)
    lib.step('r1/body/1', None)
    lib.emit('r1/body/1', v_6e4072312f626f6479)
    lib.done(1)
    lib.step('r1/body/2', None)
    v_726563656970744072312f626f6479 = lib.preview('r1/body/2', v_776f7264, v_746172676574)
    lib.done(2)
    lib.step('r1/body/3', None)
    return lib.result('r1/body/3', lib.literal('Text', 'requested', 'r1/body/3/literal'), True)

# Mechanically rendered experiment code; no host or syntax selection.
def r_7231(lib):
    v_746172676574 = lib.input('target')
    lib.step('r1/body/0', None)
    v_7365747570734072312f626f6479 = lib.kernel('r1/body/0', 'Words', 'setup', [v_746172676574], None)
    lib.done(0)
    lib.step('r1/body/1', None)
    return lib.result('r1/body/1', v_7365747570734072312f626f6479, True)

# Mechanically rendered experiment code; no host or syntax selection.
def r_7231(lib):
    v_746172676574 = lib.input('target')
    lib.step('r1/body/0', None)
    v_7365747570734072312f626f6479 = lib.kernel('r1/body/0', 'Words', 'setup', [v_746172676574], None)
    lib.done(0)
    lib.step('r1/body/1', None)
    return lib.result('r1/body/1', v_7365747570734072312f626f6479, True)

# Mechanically rendered experiment code; no host or syntax selection.
def r_7231(lib):
    v_746172676574 = lib.input('target')
    lib.step('r1/body/0', None)
    v_7365747570734072312f626f6479 = lib.kernel('r1/body/0', 'Words', 'setup', [v_746172676574], None)
    lib.done(0)
    lib.step('r1/body/1', None)
    return lib.result('r1/body/1', v_7365747570734072312f626f6479, True)

# Mechanically rendered experiment code; no host or syntax selection.
def r_7231(lib):
    v_7069636b30 = lib.input('pick0')
    v_746172676574 = lib.input('target')
    lib.step('r1/body/0', None)
    v_7365747570734072312f626f6479 = lib.kernel('r1/body/0', 'Words', 'setup', [v_746172676574], None)
    lib.done(0)
    lib.step('r1/body/1', None)
    v_774072312f626f6479 = lib.pick('r1/body/1', 'Word', v_7365747570734072312f626f6479, 'pick0')
    lib.done(1)
    lib.step('r1/body/2', None)
    v_726563656970744072312f626f6479 = lib.preview('r1/body/2', v_774072312f626f6479, v_746172676574)
    lib.done(2)
    lib.step('r1/body/3', None)
    return lib.result('r1/body/3', lib.literal('Text', 'requested', 'r1/body/3/literal'), True)

# Mechanically rendered experiment code; no host or syntax selection.
def r_7231(lib):
    v_7069636b30 = lib.input('pick0')
    v_746172676574 = lib.input('target')
    lib.step('r1/body/0', None)
    v_6e4072312f626f6479 = lib.kernel('r1/body/0', 'Ints', 'counts', [], None)
    lib.done(0)
    lib.step('r1/body/1', None)
    lib.emit('r1/body/1', v_6e4072312f626f6479)
    lib.done(1)
    lib.step('r1/body/2', None)
    v_7365747570734072312f626f6479 = lib.kernel('r1/body/2', 'Words', 'setup', [v_746172676574], None)
    lib.done(2)
    lib.step('r1/body/3', None)
    v_774072312f626f6479 = lib.pick('r1/body/3', 'Word', v_7365747570734072312f626f6479, 'pick0')
    lib.done(3)
    lib.step('r1/body/4', None)
    v_726563656970744072312f626f6479 = lib.preview('r1/body/4', v_774072312f626f6479, v_746172676574)
    lib.done(4)
    lib.step('r1/body/5', None)
    return lib.result('r1/body/5', lib.literal('Text', 'requested', 'r1/body/5/literal'), True)

# Mechanically rendered experiment code; no host or syntax selection.
def r_7231(lib):
    v_746172676574 = lib.input('target')
    v_776f7264 = lib.input('word')
    lib.step('r1/body/0', None)
    v_6e754072312f626f6479 = lib.kernel('r1/body/0', 'Ints', 'nu2', [], None)
    lib.done(0)
    lib.step('r1/body/1', None)
    v_636f65664072312f626f6479 = lib.calc('r1/body/1', 'Ints', 'linear_solve_mod', [lib.literal('Matrix', [[1, 0], [0, 1]], 'r1/body/1/args/0/literal'), v_6e754072312f626f6479, lib.literal('Int', 2, 'r1/body/1/args/2/literal')], None)
    lib.done(1)
    lib.step('r1/body/2', None)
    v_774072312f626f6479 = lib.calc('r1/body/2', 'Word', 'combine_corrections', [lib.literal('Words', [['a'], ['b']], 'r1/body/2/args/0/literal'), v_636f65664072312f626f6479], None)
    lib.done(2)
    lib.step('r1/body/3', None)
    return lib.result('r1/body/3', v_774072312f626f6479, True)

# Mechanically rendered experiment code; no host or syntax selection.
def r_7231(lib):
    v_746172676574 = lib.input('target')
    v_776f7264 = lib.input('word')
    lib.step('r1/body/0', None)
    v_6e754072312f626f6479 = lib.kernel('r1/body/0', 'Ints', 'nu5', [], None)
    lib.done(0)
    lib.step('r1/body/1', None)
    v_636f65664072312f626f6479 = lib.calc('r1/body/1', 'Ints', 'linear_solve_mod', [lib.literal('Matrix', [[1, 0], [0, 1]], 'r1/body/1/args/0/literal'), v_6e754072312f626f6479, lib.literal('Int', 5, 'r1/body/1/args/2/literal')], None)
    lib.done(1)
    lib.step('r1/body/2', None)
    v_774072312f626f6479 = lib.calc('r1/body/2', 'Word', 'combine_corrections', [lib.literal('Words', [['c'], ['d']], 'r1/body/2/args/0/literal'), v_636f65664072312f626f6479], None)
    lib.done(2)
    lib.step('r1/body/3', None)
    return lib.result('r1/body/3', v_774072312f626f6479, True)

# Mechanically rendered experiment code; no host or syntax selection.
def r_7231(lib):
    v_63617073 = lib.input('caps')
    v_746172676574 = lib.input('target')
    v_776f7264 = lib.input('word')
    v_776f726473 = lib.input('words')
    lib.step('r1/body/0', None)
    v_73636f7265734072312f626f6479 = lib.calc('r1/body/0', 'Ints', 'identity', [lib.literal('Ints', [], 'r1/body/0/args/0/literal')], None)
    lib.done(0)
    lib.step('r1/body/1', None)
    for v_696e6465784072312f626f64792f31, v_774072312f626f64792f31 in lib.iterate('r1/body/1', v_776f726473, 'Word', False):
        lib.step('r1/body/1/body/0', v_696e6465784072312f626f64792f31)
        v_734072312f626f64792f312f626f6479 = lib.kernel('r1/body/1/body/0', 'Int', 'score', [v_774072312f626f64792f31], v_696e6465784072312f626f64792f31)
        lib.step('r1/body/1/body/1', v_696e6465784072312f626f64792f31)
        v_73636f7265734072312f626f6479 = lib.calc('r1/body/1/body/1', 'Ints', 'append', [v_73636f7265734072312f626f6479, v_734072312f626f64792f312f626f6479], v_696e6465784072312f626f64792f31)
    lib.done(1)
    lib.step('r1/body/2', None)
    v_694072312f626f6479 = lib.calc('r1/body/2', 'Int', 'argmin_first', [v_73636f7265734072312f626f6479], None)
    lib.done(2)
    lib.step('r1/body/3', None)
    v_63686f73656e4072312f626f6479 = lib.calc('r1/body/3', 'Word', 'index', [v_776f726473, v_694072312f626f6479], None)
    lib.done(3)
    lib.step('r1/body/4', None)
    return lib.result('r1/body/4', v_63686f73656e4072312f626f6479, True)

# Mechanically rendered experiment code; no host or syntax selection.
def r_7231(lib):
    v_746172676574 = lib.input('target')
    v_776f7264 = lib.input('word')
    lib.step('r1/body/0', None)
    v_6e754072312f626f6479 = lib.kernel('r1/body/0', 'Ints', 'nu2', [], None)
    lib.done(0)
    lib.step('r1/body/1', None)
    lib.workspace_put('r1/body/1', 'research_nu', v_6e754072312f626f6479)
    lib.done(1)
    lib.step('r1/body/2', None)
    v_73617665644072312f626f6479 = lib.workspace_get('r1/body/2', 'research_nu')
    lib.done(2)
    lib.step('r1/body/3', None)
    return lib.result('r1/body/3', v_73617665644072312f626f6479, True)

# Mechanically rendered experiment code; no host or syntax selection.
def r_7231(lib):
    v_746172676574 = lib.input('target')
    v_776f7264 = lib.input('word')
    lib.step('r1/body/0', None)
    v_6e754072312f626f6479 = lib.kernel('r1/body/0', 'Ints', 'nu2', [], None)
    lib.done(0)
    lib.step('r1/body/1', None)
    v_774072312f626f6479 = lib.calc('r1/body/1', 'Word', 'combine_corrections', [lib.literal('Words', [['a'], ['b']], 'r1/body/1/args/0/literal'), v_6e754072312f626f6479], None)
    lib.done(1)
    lib.step('r1/body/2', None)
    v_726563656970744072312f626f6479 = lib.preview('r1/body/2', v_774072312f626f6479, v_746172676574)
    lib.done(2)
    lib.step('r1/body/3', None)
    return lib.result('r1/body/3', lib.literal('Text', 'captured', 'r1/body/3/literal'), True)

# Mechanically rendered experiment code; no host or syntax selection.
def r_7231(lib):
    lib.step('r1/body/0', None)
    v_6c65616b4072312f626f6479 = lib.ambient('r1/body/0', 'Text', 'clock', 'clock_probe')
    lib.done(0)
    lib.step('r1/body/1', None)
    lib.emit('r1/body/1', v_6c65616b4072312f626f6479)
    lib.done(1)
    lib.step('r1/body/2', None)
    return lib.result('r1/body/2', v_6c65616b4072312f626f6479, True)

# Mechanically rendered experiment code; no host or syntax selection.
def r_7231(lib):
    lib.step('r1/body/0', None)
    v_6c65616b4072312f626f6479 = lib.ambient('r1/body/0', 'Text', 'file', 'file_probe')
    lib.done(0)
    lib.step('r1/body/1', None)
    lib.emit('r1/body/1', v_6c65616b4072312f626f6479)
    lib.done(1)
    lib.step('r1/body/2', None)
    return lib.result('r1/body/2', v_6c65616b4072312f626f6479, True)

# Mechanically rendered experiment code; no host or syntax selection.
def r_7231(lib):
    lib.step('r1/body/0', None)
    v_6c65616b4072312f626f6479 = lib.ambient('r1/body/0', 'Text', 'network', 'network_probe')
    lib.done(0)
    lib.step('r1/body/1', None)
    lib.emit('r1/body/1', v_6c65616b4072312f626f6479)
    lib.done(1)
    lib.step('r1/body/2', None)
    return lib.result('r1/body/2', v_6c65616b4072312f626f6479, True)

# Mechanically rendered experiment code; no host or syntax selection.
def r_7231(lib):
    lib.step('r1/body/0', None)
    v_6c65616b4072312f626f6479 = lib.ambient('r1/body/0', 'Text', 'process', 'process_probe')
    lib.done(0)
    lib.step('r1/body/1', None)
    lib.emit('r1/body/1', v_6c65616b4072312f626f6479)
    lib.done(1)
    lib.step('r1/body/2', None)
    return lib.result('r1/body/2', v_6c65616b4072312f626f6479, True)

# Mechanically rendered experiment code; no host or syntax selection.
def r_7231(lib):
    lib.step('r1/body/0', None)
    lib.done(0)
    lib.step('r1/body/1', None)
    lib.done(1)
    lib.step('r1/body/2', None)
    lib.done(2)

# Mechanically rendered experiment code; no host or syntax selection.
def r_7231(lib):
    lib.step('r1/body/0', None)
    v_614072312f626f6479 = lib.kernel('r1/body/0', 'Ints', 'counts', [], None)
    lib.done(0)
    lib.step('r1/body/1', None)
    v_624072312f626f6479 = lib.kernel('r1/body/1', 'Ints', 'counts', [], None)
    lib.done(1)

# Mechanically rendered experiment code; no host or syntax selection.
def r_7231(lib):
    lib.step('r1/body/0', None)
    v_614072312f626f6479 = lib.retain('r1/body/0', 16)
    lib.done(0)
    lib.step('r1/body/1', None)
    v_624072312f626f6479 = lib.retain('r1/body/1', 16)
    lib.done(1)

# Mechanically rendered experiment code; no host or syntax selection.
def r_7231(lib):
    lib.step('r1/body/0', None)
    lib.emit('r1/body/0', lib.literal('Text', 'abcd', 'r1/body/0/literal'))
    lib.done(0)
    lib.step('r1/body/1', None)
    lib.emit('r1/body/1', lib.literal('Text', 'abcd', 'r1/body/1/literal'))
    lib.done(1)

# Mechanically rendered experiment code; no host or syntax selection.
def r_7231(lib):
    lib.step('r1/body/0', None)
    v_6e4072312f626f6479 = lib.kernel('r1/body/0', 'Ints', 'counts', [], None)
    lib.done(0)

# Mechanically rendered experiment code; no host or syntax selection.
def r_7231(lib):
    lib.step('r1/body/0', None)
    v_6e4072312f626f6479 = lib.kernel('r1/body/0', 'Ints', 'counts', [], None)
    lib.done(0)

# Mechanically rendered experiment code; no host or syntax selection.
def m_72656375727365(lib):
    lib.step('modules/recurse/body/0', None)
    v_726573756c74406d6f64756c65732f726563757273652f626f6479 = lib.call('modules/recurse/body/0', 'Int', m_72656375727365, [])
    lib.step('modules/recurse/body/1', None)
    return lib.result('modules/recurse/body/1', v_726573756c74406d6f64756c65732f726563757273652f626f6479, False)

def r_7231(lib):
    lib.step('r1/body/0', None)
    v_726573756c744072312f626f6479 = lib.call('r1/body/0', 'Int', m_72656375727365, [])

# Mechanically rendered experiment code; no host or syntax selection.
def r_7231(lib):
    lib.step('r1/body/0', None)
    for v_696e6465784072312f626f64792f30, v_694072312f626f64792f30 in lib.iterate('r1/body/0', lib.literal('Ints', [0, 1, 2, 3, 4, 5, 6, 7, 8, 9], 'r1/body/0/literal'), 'Int', True):
        lib.step('r1/body/0/body/0', v_696e6465784072312f626f64792f30)

# Mechanically rendered experiment code; no host or syntax selection.
def r_7231(lib):
    lib.step('r1/body/0', None)
    v_62617463684072312f626f6479 = lib.kernel('r1/body/0', 'Rows', 'batch_check', [lib.literal('Words', [['a'], ['b'], ['c']], 'r1/body/0/args/0/literal')], None)

# Mechanically rendered experiment code; no host or syntax selection.
def r_7231(lib):
    lib.step('r1/body/0', None)
    for v_696e6465784072312f626f64792f30, v_694072312f626f64792f30 in lib.iterate('r1/body/0', lib.literal('Ints', [0, 1, 2, 3, 4, 5, 6, 7, 8, 9], 'r1/body/0/literal'), 'Int', True):
        lib.step('r1/body/0/body/0', v_696e6465784072312f626f64792f30)

# Mechanically rendered experiment code; no host or syntax selection.
def r_7231(lib):
    lib.step('r1/body/0', None)
    for v_696e6465784072312f626f64792f30, v_694072312f626f64792f30 in lib.iterate('r1/body/0', lib.literal('Ints', [0, 1, 2, 3, 4, 5, 6, 7, 8, 9], 'r1/body/0/literal'), 'Int', True):
        lib.step('r1/body/0/body/0', v_696e6465784072312f626f64792f30)

# Mechanically rendered experiment code; no host or syntax selection.
def r_7231(lib):
    lib.step('r1/body/0', None)
    v_614072312f626f6479 = lib.kernel('r1/body/0', 'Ints', 'counts', [], None)
    lib.done(0)
    lib.step('r1/body/1', None)
    v_624072312f626f6479 = lib.kernel('r1/body/1', 'Ints', 'counts', [], None)
    lib.done(1)
    lib.step('r1/body/2', None)
    return lib.result('r1/body/2', v_624072312f626f6479, True)

# Mechanically rendered experiment code; no host or syntax selection.
def r_7231(lib):
    lib.step('r1/body/0', None)
    lib.done(0)

# Mechanically rendered experiment code; no host or syntax selection.
def r_7231(lib):
    lib.step('r1/body/0', None)
    lib.done(0)

# Mechanically rendered experiment code; no host or syntax selection.
def r_7231(lib):
    lib.step('r1/body/0', None)
    lib.dynamic('r1/body/0', lib.literal('Text', 'constant arithmetic', 'r1/body/0/literal'))
    lib.done(0)
    lib.step('r1/body/1', None)
    return lib.result('r1/body/1', lib.literal('Int', 2, 'r1/body/1/literal'), True)

# Mechanically rendered experiment code; no host or syntax selection.
def m_636f6e7374616e74(lib):
    lib.step('modules/constant/body/0', None)
    return lib.result('modules/constant/body/0', lib.literal('Int', 1, 'modules/constant/body/0/literal'), False)

def r_7231(lib):
    lib.step('r1/body/0', None)
    v_6e4072312f626f6479 = lib.call('r1/body/0', 'Int', m_636f6e7374616e74, [])
    lib.done(0)
    lib.step('r1/body/1', None)
    return lib.result('r1/body/1', v_6e4072312f626f6479, True)


def r_7231():
    v_746172676574 = h_input('target')
    v_776f7264 = h_input('word')
    v_6e75 = h_kernel('r1/body/0', 'Ints', 'nu2', [], [])
    v_77 = h_calc('r1/body/1', 'Word', 'combine_corrections', [h_literal('Words', '[["a"],["b"]]', 'r1/body/1/literal'), v_6e75], [])
    v_72656365697074 = h_preview('r1/body/2', v_77, v_746172676574, [])
    return h_return('r1/body/3', h_literal('Text', '"captured"', 'r1/body/3/literal'), [])


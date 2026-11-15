def r_7231():
    v_746172676574 = h_input('target')
    v_776f7264 = h_input('word')
    v_6e75 = h_kernel('r1/body/0', 'Ints', 'nu2', [], [])
    v_636f6566 = h_calc('r1/body/1', 'Ints', 'linear_solve_mod', [h_literal('Matrix', '[[1,0],[0,1]]', 'r1/body/1/literal'), v_6e75, h_literal('Int', '2', 'r1/body/1/literal')], [])
    v_77 = h_calc('r1/body/2', 'Word', 'combine_corrections', [h_literal('Words', '[["a"],["b"]]', 'r1/body/2/literal'), v_636f6566], [])
    return h_return('r1/body/3', v_77, [])


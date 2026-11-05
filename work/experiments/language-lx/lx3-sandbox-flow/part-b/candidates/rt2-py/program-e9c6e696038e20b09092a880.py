def r_7231():
    v_7069636b30 = h_input('pick0')
    v_746172676574 = h_input('target')
    v_6e75 = h_kernel('r1/body/0', 'Ints', 'nu2', [], [])
    v_77 = h_calc('r1/body/1', 'Word', 'combine_corrections', [h_literal('Words', '[["a"],["b"]]', 'r1/body/1/literal'), v_6e75], [])
    v_6f6e65 = h_calc('r1/body/2', 'Words', 'singleton', [v_77], [])
    v_63686f73656e = h_pick('r1/body/3', 'Word', v_6f6e65, 'pick0', [])
    v_72656365697074 = h_preview('r1/body/4', v_63686f73656e, v_746172676574, [])


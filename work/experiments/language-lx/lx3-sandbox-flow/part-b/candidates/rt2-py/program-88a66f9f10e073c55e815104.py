def r_7231():
    v_63617073 = h_input('caps')
    v_746172676574 = h_input('target')
    v_776f7264 = h_input('word')
    v_776f726473 = h_input('words')
    v_6e = h_kernel('r1/body/0', 'Ints', 'counts', [], [])
    v_69 = h_calc('r1/body/1', 'Int', 'argmax_first', [v_6e], [])
    v_74 = h_calc('r1/body/2', 'Target', 'index', [v_63617073, v_69], [])
    v_72656365697074 = h_preview('r1/body/3', v_776f7264, v_74, [])


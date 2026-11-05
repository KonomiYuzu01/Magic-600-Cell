def r_7231():
    v_63617073 = h_input('caps')
    v_746172676574 = h_input('target')
    v_776f7264 = h_input('word')
    v_776f726473 = h_input('words')
    v_6e = h_kernel('r1/body/0', 'Ints', 'counts', [], [])
    v_72616e6b = h_calc('r1/body/1', 'Ints', 'rank_desc_indices', [v_6e], [])
    v_69 = h_calc('r1/body/2', 'Int', 'index', [v_72616e6b, h_literal('Int', '0', 'r1/body/2/literal')], [])
    v_74 = h_calc('r1/body/3', 'Target', 'index', [v_63617073, v_69], [])
    v_72656365697074 = h_preview('r1/body/4', v_776f7264, v_74, [])


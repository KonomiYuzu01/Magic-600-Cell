def r_7231():
    v_63617073 = h_input('caps')
    v_746172676574 = h_input('target')
    v_776f7264 = h_input('word')
    v_776f726473 = h_input('words')
    v_73636f726573 = h_calc('r1/body/0', 'Ints', 'identity', [h_literal('Ints', '[]', 'r1/body/0/literal')], [])
    for v_72312f626f64792f312f6974656d2f77 in h_iter('r1/body/1', 'Word', v_776f726473, []):
        v_72312f626f64792f312f6c6f63616c2f73 = h_kernel('r1/body/1/body/0', 'Int', 'score', [v_72312f626f64792f312f6974656d2f77], [])
        v_73636f726573 = h_calc('r1/body/1/body/1', 'Ints', 'append', [v_73636f726573, v_72312f626f64792f312f6c6f63616c2f73], [])
    h_done('r1/body/1')
    v_69 = h_calc('r1/body/2', 'Int', 'argmin_first', [v_73636f726573], [])
    v_63686f73656e = h_calc('r1/body/3', 'Word', 'index', [v_776f726473, v_69], [])
    return h_return('r1/body/4', v_63686f73656e, [])


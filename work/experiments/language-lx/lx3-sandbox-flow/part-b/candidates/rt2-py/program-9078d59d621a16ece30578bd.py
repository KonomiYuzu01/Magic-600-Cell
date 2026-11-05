def r_7231():
    v_746172676574 = h_input('target')
    v_776f7264 = h_input('word')
    v_66 = h_kernel('r1/body/0', 'Ints', 'floors', [], [])
    v_7a = h_calc('r1/body/1', 'Int', 'count_equal', [v_66, h_literal('Int', '0', 'r1/body/1/literal')], [])
    return h_return('r1/body/2', v_7a, [])


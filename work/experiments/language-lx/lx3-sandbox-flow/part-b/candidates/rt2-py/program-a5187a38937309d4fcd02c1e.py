def r_7231():
    v_746172676574 = h_input('target')
    v_776f7264 = h_input('word')
    v_6e = h_kernel('r1/body/0', 'Ints', 'counts', [], [])
    v_726f7773 = h_calc('r1/body/1', 'Rows', 'zip', [h_literal('Targets', '["c0","c1","c2"]', 'r1/body/1/literal'), v_6e], [])
    return h_return('r1/body/2', v_726f7773, [])


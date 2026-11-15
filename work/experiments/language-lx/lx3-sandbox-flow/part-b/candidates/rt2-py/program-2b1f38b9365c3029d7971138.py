def r_7231():
    v_746172676574 = h_input('target')
    v_776f7264 = h_input('word')
    v_6e = h_kernel('r1/body/0', 'Ints', 'counts', [], [])
    v_6e30 = h_calc('r1/body/1', 'Int', 'index', [v_6e, h_literal('Int', '0', 'r1/body/1/literal')], [])
    v_74657374 = h_calc('r1/body/2', 'Bool', 'gt', [v_6e30, h_literal('Int', '0', 'r1/body/2/literal')], [])
    c_72312f626f64792f33 = h_condition('r1/body/3', v_74657374, [])
    if h_truth(c_72312f626f64792f33):
        v_72656365697074 = h_preview('r1/body/3/then/0', h_literal('Word', '["a"]', 'r1/body/3/then/0/literal'), v_746172676574, [c_72312f626f64792f33])
    else:
        v_72656365697074 = h_preview('r1/body/3/else/0', h_literal('Word', '["b"]', 'r1/body/3/else/0/literal'), v_746172676574, [c_72312f626f64792f33])
    h_done('r1/body/3')


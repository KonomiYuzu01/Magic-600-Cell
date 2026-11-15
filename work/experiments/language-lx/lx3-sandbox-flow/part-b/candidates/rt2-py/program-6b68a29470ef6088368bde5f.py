def r_7231():
    v_746172676574 = h_input('target')
    v_776f7264 = h_input('word')
    v_72656365697074 = h_preview('r1/body/0', h_literal('Word', '["c"]', 'r1/body/0/literal'), v_746172676574, [])
    v_6f7574636f6d65 = h_kernel('r1/body/1', 'Record', 'preview_outcome', [v_72656365697074], [])
    v_6669656c64 = h_calc('r1/body/2', 'Bool', 'field', [v_6f7574636f6d65, h_literal('Text', '"protected"', 'r1/body/2/literal')], [])
    v_74657374 = h_calc('r1/body/3', 'Bool', 'eq', [v_6669656c64, h_literal('Bool', 'false', 'r1/body/3/literal')], [])
    c_72312f626f64792f34 = h_condition('r1/body/4', v_74657374, [])
    if h_truth(c_72312f626f64792f34):
        v_72656365697074 = h_preview('r1/body/4/then/0', h_literal('Word', '["a"]', 'r1/body/4/then/0/literal'), v_746172676574, [c_72312f626f64792f34])
    else:
        v_72656365697074 = h_preview('r1/body/4/else/0', h_literal('Word', '["b"]', 'r1/body/4/else/0/literal'), v_746172676574, [c_72312f626f64792f34])
    h_done('r1/body/4')


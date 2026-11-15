def r_7231():
    v_746172676574 = h_input('target')
    v_776f7264 = h_input('word')
    v_62697473 = h_kernel('r1/body/0', 'Ints', 'bits', [], [])
    v_62 = h_calc('r1/body/1', 'Int', 'index', [v_62697473, h_literal('Int', '0', 'r1/body/1/literal')], [])
    v_74657374 = h_calc('r1/body/2', 'Bool', 'eq', [v_62, h_literal('Int', '1', 'r1/body/2/literal')], [])
    c_72312f626f64792f33 = h_condition('r1/body/3', v_74657374, [])
    if h_truth(c_72312f626f64792f33):
        h_stop('r1/body/3/then/0', [c_72312f626f64792f33])
    else:
        pass
    h_done('r1/body/3')
    v_72656365697074 = h_preview('r1/body/4', v_776f7264, v_746172676574, [])


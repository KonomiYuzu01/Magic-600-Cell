def r_7231():
    v_746172676574 = h_input('target')
    v_776f7264 = h_input('word')
    v_62697473 = h_kernel('r1/body/0', 'Ints', 'bits', [], [])
    v_626974 = h_calc('r1/body/1', 'Int', 'index', [v_62697473, h_literal('Int', '0', 'r1/body/1/literal')], [])
    v_74657374 = h_calc('r1/body/2', 'Bool', 'eq', [v_626974, h_literal('Int', '1', 'r1/body/2/literal')], [])
    c_72312f626f64792f33 = h_condition('r1/body/3', v_74657374, [])
    if h_truth(c_72312f626f64792f33):
        h_stop('r1/body/3/then/0', [c_72312f626f64792f33])
    else:
        pass
    h_done('r1/body/3')
    v_72656365697074 = h_preview('r1/body/4', h_literal('Word', '["b"]', 'r1/body/4/literal'), v_746172676574, [])

def r_7232():
    v_746172676574 = h_input('target')
    v_776f7264 = h_input('word')
    v_62697473 = h_kernel('r2/body/0', 'Ints', 'bits', [], [])
    v_626974 = h_calc('r2/body/1', 'Int', 'index', [v_62697473, h_literal('Int', '1', 'r2/body/1/literal')], [])
    v_74657374 = h_calc('r2/body/2', 'Bool', 'eq', [v_626974, h_literal('Int', '1', 'r2/body/2/literal')], [])
    c_72322f626f64792f33 = h_condition('r2/body/3', v_74657374, [])
    if h_truth(c_72322f626f64792f33):
        h_stop('r2/body/3/then/0', [c_72322f626f64792f33])
    else:
        pass
    h_done('r2/body/3')
    v_72656365697074 = h_preview('r2/body/4', h_literal('Word', '["b"]', 'r2/body/4/literal'), v_746172676574, [])

def r_7233():
    v_746172676574 = h_input('target')
    v_776f7264 = h_input('word')
    v_62697473 = h_kernel('r3/body/0', 'Ints', 'bits', [], [])
    v_626974 = h_calc('r3/body/1', 'Int', 'index', [v_62697473, h_literal('Int', '2', 'r3/body/1/literal')], [])
    v_74657374 = h_calc('r3/body/2', 'Bool', 'eq', [v_626974, h_literal('Int', '1', 'r3/body/2/literal')], [])
    c_72332f626f64792f33 = h_condition('r3/body/3', v_74657374, [])
    if h_truth(c_72332f626f64792f33):
        h_stop('r3/body/3/then/0', [c_72332f626f64792f33])
    else:
        pass
    h_done('r3/body/3')
    v_72656365697074 = h_preview('r3/body/4', h_literal('Word', '["b"]', 'r3/body/4/literal'), v_746172676574, [])


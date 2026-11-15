def r_7231():
    v_63617073 = h_input('caps')
    v_746172676574 = h_input('target')
    v_776f7264 = h_input('word')
    v_776f726473 = h_input('words')
    for v_72312f626f64792f302f6974656d2f77 in h_iter('r1/body/0', 'Word', v_776f726473, []):
        v_72312f626f64792f302f6c6f63616c2f72656365697074 = h_preview('r1/body/0/body/0', v_72312f626f64792f302f6974656d2f77, v_746172676574, [])
        v_72312f626f64792f302f6c6f63616c2f73 = h_kernel('r1/body/0/body/1', 'Int', 'score', [v_72312f626f64792f302f6974656d2f77], [])
        v_72312f626f64792f302f6c6f63616c2f646f6e65 = h_calc('r1/body/0/body/2', 'Bool', 'eq', [v_72312f626f64792f302f6c6f63616c2f73, h_literal('Int', '0', 'r1/body/0/body/2/literal')], [])
        c_72312f626f64792f302f626f64792f33 = h_condition('r1/body/0/body/3', v_72312f626f64792f302f6c6f63616c2f646f6e65, [])
        if h_truth(c_72312f626f64792f302f626f64792f33):
            h_stop('r1/body/0/body/3/then/0', [c_72312f626f64792f302f626f64792f33])
        else:
            pass
        h_done('r1/body/0/body/3')
    h_done('r1/body/0')


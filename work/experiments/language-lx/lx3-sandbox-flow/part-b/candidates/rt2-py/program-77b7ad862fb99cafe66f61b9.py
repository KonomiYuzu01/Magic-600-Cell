def m_73656c6563746f72():
    v_746172676574 = h_input('target')
    v_776f7264 = h_input('word')
    v_6e = h_kernel('modules/selector/body/0', 'Ints', 'nu2', [], [])
    v_76 = h_calc('modules/selector/body/1', 'Int', 'index', [v_6e, h_literal('Int', '0', 'modules/selector/body/1/literal')], [])
    v_62 = h_calc('modules/selector/body/2', 'Bool', 'eq', [v_76, h_literal('Int', '1', 'modules/selector/body/2/literal')], [])
    c_6d6f64756c65732f73656c6563746f722f626f64792f33 = h_condition('modules/selector/body/3', v_62, [])
    if h_truth(c_6d6f64756c65732f73656c6563746f722f626f64792f33):
        return h_return('modules/selector/body/3/then/0', h_literal('Word', '["a"]', 'modules/selector/body/3/then/0/literal'), [c_6d6f64756c65732f73656c6563746f722f626f64792f33])
    else:
        return h_return('modules/selector/body/3/else/0', h_literal('Word', '["b"]', 'modules/selector/body/3/else/0/literal'), [c_6d6f64756c65732f73656c6563746f722f626f64792f33])
    h_done('modules/selector/body/3')

def r_7231():
    v_746172676574 = h_input('target')
    v_776f7264 = h_input('word')
    v_77 = h_call('r1/body/0', 'Word', m_73656c6563746f72, [], [])
    v_72656365697074 = h_preview('r1/body/1', v_77, v_746172676574, [])


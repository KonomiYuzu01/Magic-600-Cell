def r_7231():
    v_746172676574 = h_input('target')
    v_776f7264 = h_input('word')
    v_6e = h_kernel('r1/body/0', 'Ints', 'counts', [], [])
    h_emit('r1/body/1', v_6e, [])
    v_72656365697074 = h_preview('r1/body/2', v_776f7264, v_746172676574, [])
    return h_return('r1/body/3', h_literal('Text', '"requested"', 'r1/body/3/literal'), [])


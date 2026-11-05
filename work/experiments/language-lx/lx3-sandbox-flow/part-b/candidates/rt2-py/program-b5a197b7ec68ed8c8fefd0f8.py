def r_7231():
    v_7069636b30 = h_input('pick0')
    v_746172676574 = h_input('target')
    v_6e = h_kernel('r1/body/0', 'Ints', 'counts', [], [])
    h_emit('r1/body/1', v_6e, [])
    v_736574757073 = h_kernel('r1/body/2', 'Words', 'setup', [v_746172676574], [])
    v_77 = h_pick('r1/body/3', 'Word', v_736574757073, 'pick0', [])
    v_72656365697074 = h_preview('r1/body/4', v_77, v_746172676574, [])
    return h_return('r1/body/5', h_literal('Text', '"requested"', 'r1/body/5/literal'), [])


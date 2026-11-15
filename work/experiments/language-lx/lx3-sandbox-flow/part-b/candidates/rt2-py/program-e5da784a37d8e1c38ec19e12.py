def r_7231():
    v_63617073 = h_input('caps')
    v_746172676574 = h_input('target')
    v_776f7264 = h_input('word')
    v_776f726473 = h_input('words')
    v_6e = h_kernel('r1/body/0', 'Ints', 'counts', [], [])
    v_69 = h_calc('r1/body/1', 'Int', 'argmax_first', [v_6e], [])
    v_77 = h_calc('r1/body/2', 'Word', 'index', [v_776f726473, v_69], [])
    h_workspace_put('r1/body/3', 'saved', v_77, [])

def r_7232():
    v_63617073 = h_input('caps')
    v_746172676574 = h_input('target')
    v_776f7264 = h_input('word')
    v_776f726473 = h_input('words')
    v_77 = h_workspace_get('r2/body/0', 'Word', 'saved', [])
    v_72656e616d6564 = h_calc('r2/body/1', 'Word', 'identity', [v_77], [])
    h_workspace_put('r2/body/2', 'renamed', v_72656e616d6564, [])
    v_72656365697074 = h_preview('r2/body/3', v_72656e616d6564, v_746172676574, [])


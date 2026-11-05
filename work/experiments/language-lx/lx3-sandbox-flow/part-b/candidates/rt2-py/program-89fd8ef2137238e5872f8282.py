def r_7231():
    v_746172676574 = h_input('target')
    v_776f7264 = h_input('word')
    v_6e75 = h_kernel('r1/body/0', 'Ints', 'nu2', [], [])
    h_workspace_put('r1/body/1', 'research_nu', v_6e75, [])
    v_7361766564 = h_workspace_get('r1/body/2', 'Ints', 'research_nu', [])
    return h_return('r1/body/3', v_7361766564, [])


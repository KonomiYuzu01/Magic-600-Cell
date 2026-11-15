def r_7231():
    v_6c6567616379 = h_input('legacy')
    v_746172676574 = h_input('target')
    h_workspace_put('r1/body/0', 'legacy', v_6c6567616379, [])

def r_7232():
    v_6c6567616379 = h_input('legacy')
    v_746172676574 = h_input('target')
    v_77 = h_workspace_get('r2/body/0', 'Word', 'legacy', [])
    v_72656365697074 = h_preview('r2/body/1', v_77, v_746172676574, [])


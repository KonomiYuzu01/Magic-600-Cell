def r_7231():
    v_746172676574 = h_input('target')
    v_6e = h_kernel('r1/body/0', 'Ints', 'counts', [], [])
    v_76 = h_calc('r1/body/1', 'Int', 'index', [v_6e, h_literal('Int', '1', 'r1/body/1/literal')], [])
    v_7879 = h_calc('r1/body/2', 'Rows', 'point', [h_literal('Int', '1', 'r1/body/2/literal'), v_76], [])
    h_publish('r1/body/3', h_literal('Record', '{"exact_properties":{"value":6},"kind":"points","object_ids":["c1"],"order":[0]}', 'r1/body/3/literal'), v_7879, [])
    return h_return('r1/body/4', h_literal('Text', '"published"', 'r1/body/4/literal'), [])


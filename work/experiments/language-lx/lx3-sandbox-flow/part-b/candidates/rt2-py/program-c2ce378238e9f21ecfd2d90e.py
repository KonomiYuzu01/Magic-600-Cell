def r_7231():
    v_746172676574 = h_input('target')
    v_776f7264 = h_input('word')
    v_76 = h_kernel('r1/body/0', 'Ints', 'counts', [], [])
    v_636f6f726473 = h_calc('r1/body/1', 'Rows', 'indexed_points', [v_76], [])
    h_publish('r1/body/2', h_literal('Record', '{"exact_properties":{"values":[2,6,6]},"kind":"points","object_ids":["c0","c1","c2"],"order":[0,1,2]}', 'r1/body/2/literal'), v_636f6f726473, [])
    return h_return('r1/body/3', h_literal('Text', '"published"', 'r1/body/3/literal'), [])


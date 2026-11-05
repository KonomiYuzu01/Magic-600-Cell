def r_7231():
    v_746172676574 = h_input('target')
    v_776f7264 = h_input('word')
    h_publish('r1/body/0', h_literal('Record', '{"exact_properties":{"fraction":{"denominator":3,"numerator":1}},"kind":"reference","object_ids":["origin","third"],"order":[0,1]}', 'r1/body/0/literal'), h_literal('Rows', '[[0,0],[{"denominator":3,"numerator":1},1]]', 'r1/body/0/literal'), [])
    return h_return('r1/body/1', h_literal('Text', '"published"', 'r1/body/1/literal'), [])


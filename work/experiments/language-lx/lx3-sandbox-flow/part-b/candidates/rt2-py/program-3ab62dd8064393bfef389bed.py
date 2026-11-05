def r_7231():
    v_746172676574 = h_input('target')
    v_776f7264 = h_input('word')
    v_636865636b = h_kernel('r1/body/0', 'Record', 'word_check', [v_776f7264], [])
    h_publish('r1/body/1', h_literal('Record', '{"exact_properties":{"word":["a"]},"kind":"word_path","object_ids":["slot0","slot1"],"order":[0,1]}', 'r1/body/1/literal'), h_literal('Rows', '[[0,1],[1,0]]', 'r1/body/1/literal'), [])
    return h_return('r1/body/2', h_literal('Text', '"published"', 'r1/body/2/literal'), [])


"""LX-3 synthetic bench. This package is not a candidate or a product runtime."""

import sys

sys.dont_write_bytecode = True
sys.set_int_max_str_digits(0)  # The pinned notation uses arbitrary-precision Int.


def self_test():
    assert sys.dont_write_bytecode

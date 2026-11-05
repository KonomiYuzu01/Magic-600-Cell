"""Frozen input loading and exact, immutable JSON values."""

import hashlib
import json
from collections.abc import Mapping
from math import gcd
from pathlib import Path
from types import MappingProxyType

ROOT = Path(__file__).resolve().parents[1]
CORPUS = ROOT.parent / "corpus"
FREEZE_SHA256 = "d7eff17503ee95d45ddca0efe38ec3bb09418b0248a6279afe2dce0a90ea6627"
CASE_FILES = ("must-refuse", "allowed-tools", "planted", "budgets", "approval")


def thaw(value):
    if isinstance(value, Mapping):
        return {k: thaw(v) for k, v in value.items()}
    if isinstance(value, (tuple, list)):
        return [thaw(v) for v in value]
    return value


def freeze(value):
    if isinstance(value, Mapping):
        return MappingProxyType({k: freeze(v) for k, v in value.items()})
    if isinstance(value, (tuple, list)):
        return tuple(freeze(v) for v in value)
    return value


def validate_exact(value):
    if value is None or type(value) in (bool, int):
        return
    if isinstance(value, str):
        value.encode("utf-8", "strict")
    elif isinstance(value, (list, tuple)):
        for item in value:
            validate_exact(item)
    elif isinstance(value, Mapping):
        for key, item in value.items():
            if type(key) is not str:
                raise ValueError("JSON keys must be text")
            validate_exact(key)
            validate_exact(item)
        if set(value) == {"numerator", "denominator"}:
            n, d = value["numerator"], value["denominator"]
            if type(n) is not int or type(d) is not int or d <= 0 or gcd(n, d) != 1:
                raise ValueError("noncanonical rational")
    else:
        raise ValueError("exact JSON permits no floating point or foreign object")


def canonical(value):
    validate_exact(value)
    return json.dumps(thaw(value), ensure_ascii=False, sort_keys=True,
                      separators=(",", ":"), allow_nan=False).encode("utf-8")


def sha(data):
    return hashlib.sha256(data).hexdigest()


def digest(value):
    return sha(canonical(value))


def load(name):
    return json.loads((CORPUS / name).read_bytes())


def cases():
    return [(group, case) for group in CASE_FILES
            for case in load(group + ".json")["cases"]]


def write_json(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(canonical(value) + b"\n")


def self_test():
    assert canonical({"z": 2, "a": "e\u0301"}) == b'{"a":"e\xcc\x81","z":2}'
    assert digest("\u00e9") != digest("e\u0301")
    assert len(canonical(10**5000)) == 5001
    assert thaw(freeze({"a": [1]})) == {"a": [1]}
    for bad in (1.0, float("nan"), "\ud800", {"numerator": 2, "denominator": 4}):
        try:
            canonical(bad)
        except (ValueError, UnicodeError):
            pass
        else:
            raise AssertionError("invalid exact value admitted")
    assert len(cases()) == 75

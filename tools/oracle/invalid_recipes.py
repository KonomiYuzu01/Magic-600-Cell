"""Recipe JSON that 0.4's `Model.normalize` refuses, for the kernel's invalid-input checks (LP2, M4).

Each entry is a pair (name, recipe as JSON text). The text is what a case file carries, so lexemes such as `-0`,
`1e0` and `9223372036854775808` reach a reader exactly as written, and `json.loads(text)` is the value 0.4 would
normalize. `tests/test_kernel_oracle.py` appends every entry to a copy of the frozen case O07 and requires the Rust
kernel's adapter to report each as an `invalid-input` step with O07's unchanged state; `tests/test_oracle.py` checks
in the engine environment that `Model.normalize` raises `ValueError` for every entry, so the expectation is the
reference's. Standard library only.
"""
from __future__ import annotations

# A valid one-letter word step.
VALID_STEP = '{"kind":"word","moves":[1]}'
# In a case document the recipe sits at depth 3 and a word's first letter at depth 6 (case, steps, step, recipe, step,
# moves), so the innermost of 123 lists around a letter is at depth 128, the first depth `json.rs` refuses.
NESTED_LISTS = 123


def word(letters: str) -> str:
    return '[{"kind":"word","moves":[' + letters + "]}]"


def star(orbit: str = "0", node: str = "0", sign: str = "1", *, omit: str = "") -> str:
    fields = {"orbit": orbit, "node": node, "sign": sign}
    return '[{"kind":"star",' + ",".join(f'"{key}":{value}' for key, value in fields.items() if key != omit) + "}]"


def steps(items: list[str]) -> str:
    return "[" + ",".join(items) + "]"


def type_and_limit_family() -> list[tuple[str, str]]:
    """Lead L4: type, shape and limit confusions beyond O07's ten inputs."""
    family = [(f"letter {lexeme}", word(lexeme))
              for lexeme in ("true", '"1"', "null", "[1]", "1.0", "1e0", "-0", "9223372036854775808")]
    family += [
        ("word without moves", '[{"kind":"word"}]'),
        ("word moves as a string", '[{"kind":"word","moves":"1"}]'),
        ("empty word", word("")),
        ("word of 100,001 letters", word(",".join(["1"] * 100_001))),
        ("recipe null", "null"),
        ("recipe {}", "{}"),
        ("recipe of 257 steps", steps([VALID_STEP] * 257)),
        ("null step", "[null]"),
        ("kind missing", '[{"moves":[1]}]'),
        ('kind "Word"', '[{"kind":"Word","moves":[1]}]'),
        ("kind 1", '[{"kind":1,"moves":[1]}]'),
    ]
    family += [(f"star orbit {value}", star(orbit=value)) for value in ('"0"', "0.0", "true", "-1")]
    family += [(f"star sign {value}", star(sign=value)) for value in ("true", "1.0", "null", "2")]
    family += [(f"star node {value}", star(node=value)) for value in ("true", "0.0", "-1")]
    family.append(("star node missing", star(omit="node")))
    family.append(("a bad step after 255 valid ones", steps([VALID_STEP] * 255 + [word("0")[1:-1]])))
    return family


def json_edges() -> list[tuple[str, str]]:
    """Lead L1: recipe JSON that Python reads and `Model.normalize` refuses, but a strict reader cannot represent or
    refuses. The read of L1's fix added a repeated member name, of which Python keeps the last, and the literals
    `NaN`, `Infinity` and `-Infinity`, which Python reads as floats."""
    deep = "[" * NESTED_LISTS + '{"a":NaN,"a":-Infinity,"b":[Infinity]}' + "]" * NESTED_LISTS
    return [
        (f"a letter inside {NESTED_LISTS} nested lists", word("[" * NESTED_LISTS + "1" + "]" * NESTED_LISTS)),
        ("a kind with a lone surrogate", '[{"kind":"\\ud800"}]'),
        ("moves repeated, the last with a bool letter", '[{"kind":"word","moves":[1],"moves":[true]}]'),
        ("letter NaN", word("NaN")),
        ("star sign Infinity", star(sign="Infinity")),
        ("star node -Infinity", star(node="-Infinity")),
        (f"a letter inside {NESTED_LISTS} nested lists around repeated names and NaN", word(deep)),
    ]

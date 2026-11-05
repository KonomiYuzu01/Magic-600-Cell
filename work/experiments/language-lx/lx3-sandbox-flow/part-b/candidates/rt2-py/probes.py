"""Candidate-local escape attempts. Never added to frozen corpus denominators."""

from bridge import Bridge
from subset import SubsetError, approved_tree


ATTEMPTS = {
    "subclasses": "(1).__class__.__mro__[1].__subclasses__()",
    "helper_globals": "h_literal.__globals__",
    "format_attribute_walk": "'{0.__class__.__mro__}'.format(1)",
    "import_builtin": "__import__('os')",
    "eval_builtin": "eval('1 + 1')",
    "open_builtin": "open('candidate-local-unused-file')",
}


def observations():
    rows = []
    for name, expression in ATTEMPTS.items():
        source = ("def r_7231():\n    return h_return('r1/body/0', " + expression + ", [])\n").encode("utf-8")
        try:
            approved_tree(source)
        except (SubsetError, SyntaxError, ValueError):
            checker = "rejected"
        else:
            checker = "accepted"
        # Deliberately bypass the checker in this separate negative control.
        # These expressions only inspect object metadata. No recovered authority
        # is invoked, no file is opened, and no diagnostic/object data are saved.
        namespace = {"__builtins__": {}, "h_literal": Bridge.h_literal,
                     "h_return": lambda path, value, controls: value}
        try:
            exec(compile(source, "escape-probe.py", "exec"), namespace, namespace)
            value = namespace["r_7231"]()
            runtime = "returned " + type(value).__name__ + " metadata"
            if name == "helper_globals":
                runtime = "helper globals reachable; builtins present=" + str("__builtins__" in value)
        except NameError:
            runtime = "NameError (builtin unavailable)"
        rows.append({"attempt": name, "expression": expression, "checker": checker,
                     "approved_execution": "not entered" if checker == "rejected" else "unexpected admission",
                     "runtime_without_checker": runtime})
    return rows

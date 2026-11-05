"""Binding expressions and left-to-right pipelines. Experiment only."""
from syntax import Parser, parse as parse_document


class Expressions(Parser):
    starts = {"fn", "cmd", "view", "@"}

    def has_header(self):
        return self.at("@") and self.tokens[self.pos + 1].text == "edition"

    def header(self):
        self.expect("@")
        self.expect("edition")
        self.expect("(")
        value = self.string()
        self.expect(")")
        self.expect(";")
        return value

    def module(self):
        self.expect("@")
        self.expect("module")
        self.expect("(")
        name = self.name()
        self.expect(",")
        effects = self.names()
        self.expect(")")
        self.expect(";")
        return name, effects

    def declaration(self):
        start = self.token
        if self.accept("@"):
            self.expect("use")
            self.expect("(")
            name = self.name()
            self.expect(",")
            digest = self.string()
            self.expect(")")
            self.expect(";")
            return self.node(start, "import", name=name, digest=digest)
        kind = self.name()
        if kind not in {"fn", "cmd", "view"}:
            self.expect("fn, cmd or view")
        name, params = self.name(), self.params()
        self.expect(":")
        result = self.type()
        self.expect("!")
        effects = self.names()
        self.expect("@")
        charges = self.charges()
        metadata = None
        if kind == "cmd":
            self.expect("with")
            self.expect("(")
            row = self.string()
            self.expect(",")
            command_kind = self.name()
            self.expect(",")
            contexts = self.names()
            self.expect(",")
            permits = self.names()
            self.expect(")")
            metadata = {"row": row, "kind": command_kind, "contexts": contexts, "permits": permits}
        self.expect("=")
        bindings = []
        while self.accept("bind"):
            binding = self.name()
            self.expect("<-")
            bindings.append({"name": binding, "value": self.expression()})
            self.expect("in")
        body = self.expression()
        self.expect(";")
        return self.node(start, {"fn": "function", "cmd": "command"}.get(kind, kind),
                         name=name, params=params, result=result, effects=effects,
                         charges=charges, metadata=metadata, bindings=bindings, body=body)

    def expression(self):
        start = self.token
        if self.accept("case"):
            subject = self.atom()
            self.expect("of")
            arms = [self.arm()]
            while self.accept("|"):
                arms.append(self.arm())
            self.expect("end")
            node = self.node(start, "match", subject=subject, arms=arms)
        else:
            node = self.atom()
        while self.accept("|>"):
            name = self.name()
            self.expect("(")
            args = self.separated(")", self.expression)
            node = self.node(start, "call", name=name, args=[node, *args])
        return node


def parse(data):
    return parse_document(data, Expressions)

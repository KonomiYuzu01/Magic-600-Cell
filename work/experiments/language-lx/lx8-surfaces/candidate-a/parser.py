"""Keyword and brace surface. Experiment only."""
from syntax import Parser, parse as parse_document


class Braces(Parser):
    def has_header(self):
        return self.at("edition")

    def header(self):
        self.expect("edition")
        value = self.string()
        self.expect(";")
        return value

    def module(self):
        self.expect("module")
        name = self.name()
        self.expect("requires")
        effects = self.names()
        self.expect(";")
        return name, effects

    def declaration(self):
        start = self.token
        if self.accept("import"):
            name, digest = self.name(), self.string()
            self.expect(";")
            return self.node(start, "import", name=name, digest=digest)
        kind = self.name()
        if kind not in {"fn", "command", "view"}:
            self.expect("fn, command or view")
        name, params = self.name(), self.params()
        self.expect("->")
        result = self.type()
        self.expect("effects")
        effects = self.names()
        self.expect("charge")
        charges = self.charges()
        metadata = self.metadata() if kind == "command" else None
        self.expect("{")
        bindings = []
        while self.accept("let"):
            binding = self.name()
            self.expect("=")
            bindings.append({"name": binding, "value": self.expression()})
            self.expect(";")
        self.expect("return")
        body = self.expression()
        self.expect(";")
        self.expect("}")
        return self.node(start, "function" if kind == "fn" else kind, name=name,
                         params=params, result=result, effects=effects, charges=charges,
                         metadata=metadata, bindings=bindings, body=body)

    def expression(self):
        if not self.at("match"):
            return self.atom()
        start = self.take()
        subject = self.atom()
        self.expect("{")
        arms = []
        while not self.at("}"):
            arms.append(self.arm())
            self.expect(";")
        self.expect("}")
        return self.node(start, "match", subject=subject, arms=arms)


def parse(data):
    return parse_document(data, Braces)

"""Line and indentation surface. Experiment only."""
from syntax import Parser, SyntaxProblem, parse as parse_document


class Indentation(Parser):
    layout = True
    starts = {"function", "command", "view", "use"}

    def has_header(self):
        return self.at("edition")

    def header(self):
        self.expect("edition")
        self.expect(":")
        value = self.string()
        self.line_end()
        return value

    def module(self):
        self.expect("module")
        name = self.name()
        self.expect("requires")
        effects = self.names()
        self.line_end()
        return name, effects

    def indent(self, column):
        if self.token.column != column:
            raise SyntaxProblem(self.token, f"expected column {column}")

    def declaration(self):
        start = self.token
        if self.accept("use"):
            name = self.name()
            self.expect("from")
            digest = self.string()
            self.line_end()
            return self.node(start, "import", name=name, digest=digest)
        kind = self.name()
        if kind not in {"function", "command", "view"}:
            self.expect("function, command or view")
        name, params = self.name(), self.params()
        self.expect("returns")
        result = self.type()
        self.expect("using")
        effects = self.names()
        self.expect("charge")
        charges = self.charges()
        metadata = self.metadata() if kind == "command" else None
        self.expect(":")
        self.line_end()
        bindings = []
        while not self.at("yield"):
            self.indent(5)
            binding = self.name()
            self.expect(":=")
            bindings.append({"name": binding, "value": self.expression()})
            self.line_end()
        self.indent(5)
        self.expect("yield")
        body = self.expression()
        self.line_end()
        return self.node(start, kind, name=name, params=params, result=result, effects=effects,
                         charges=charges, metadata=metadata, bindings=bindings, body=body)

    def expression(self):
        if not self.at("choose"):
            return self.atom()
        start = self.take()
        subject = self.atom()
        self.expect(":")
        self.line_end()
        arms = []
        while self.token.column == 9:
            arms.append(self.arm())
            # Leave the final newline for the enclosing statement.
            if self.token.kind != "NL":
                raise SyntaxProblem(self.token, "expected case line end")
            position = self.pos
            self.blanks()
            if self.token.column != 9:
                self.pos = position
                break
        if not arms:
            raise SyntaxProblem(self.token, "expected indented cases")
        return self.node(start, "match", subject=subject, arms=arms)


def parse(data):
    return parse_document(data, Indentation)

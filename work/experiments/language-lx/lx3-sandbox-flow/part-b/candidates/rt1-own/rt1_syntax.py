"""Own lexer and parser for a deliberately temporary, non-product surface."""

from dataclasses import dataclass
import json
import re


TYPES = frozenset("Int Bool Text Word Target Ints Bools Words Targets Matrix Record Rows State Receipt Bytes".split())
TOKEN = re.compile(rb'\s+|"(?:[^"\\\x00-\x1f]|\\(?:["\\/bfnrt]|u[0-9a-fA-F]{4}))*"|-?(?:0|[1-9][0-9]*)|[A-Za-z_][A-Za-z_0-9]*|[()\[\]{},:]')


class SourceError(ValueError):
    pass


@dataclass(frozen=True)
class Token:
    text: str
    start: int
    end: int


def lex(source):
    tokens, offset = [], 0
    source.decode("utf-8", "strict")
    while offset < len(source):
        match = TOKEN.match(source, offset)
        if match is None:
            raise SourceError("invalid token at byte " + str(offset))
        raw = match.group()
        if not raw.isspace():
            tokens.append(Token(raw.decode("utf-8"), offset, match.end()))
        offset = match.end()
    return tokens + [Token("<eof>", offset, offset)]


@dataclass(frozen=True)
class Expression:
    name: str | None
    type: str | None
    data: object = None


@dataclass(frozen=True)
class Statement:
    op: str
    path: str
    span: tuple
    name: str | None
    type: str | None
    args: tuple
    details: tuple
    branches: tuple


@dataclass(frozen=True)
class Module:
    parameters: tuple
    result: str
    body: tuple


@dataclass(frozen=True)
class Program:
    source: bytes
    inputs: dict
    modules: dict
    runs: dict


class Parser:
    def __init__(self, source):
        self.source, self.tokens, self.index = source, lex(source), 0

    def peek(self):
        return self.tokens[self.index].text

    def take(self, expected=None):
        token = self.tokens[self.index]
        if expected is not None and token.text != expected:
            raise SourceError("expected " + expected + " at byte " + str(token.start))
        if token.text == "<eof>" and expected != "<eof>":
            raise SourceError("unexpected end of source")
        self.index += 1
        return token

    def text(self):
        token = self.take()
        if not token.text.startswith('"'):
            raise SourceError("expected quoted name")
        value = json.loads(token.text)  # String escape decoding only, never code/AST.
        value.encode("utf-8", "strict")
        return value

    def type(self):
        value = self.take().text
        if value not in TYPES:
            raise SourceError("unknown type " + value)
        return value

    def data(self):
        token = self.peek()
        if token.startswith('"'):
            return self.text()
        if token in ("true", "false", "null"):
            self.take()
            return {"true": True, "false": False, "null": None}[token]
        if token == "[":
            self.take()
            result = []
            while self.peek() != "]":
                if result:
                    self.take(",")
                result.append(self.data())
            self.take("]")
            return result
        if token == "{":
            self.take()
            result = {}
            while self.peek() != "}":
                if result:
                    self.take(",")
                key = self.text()
                if key in result:
                    raise SourceError("duplicate literal field")
                self.take(":")
                result[key] = self.data()
            self.take("}")
            return result
        if re.fullmatch(r"-?(?:0|[1-9][0-9]*)", token):
            return int(self.take().text)
        raise SourceError("expected exact literal")

    def expression(self):
        self.take("(")
        kind = self.take().text
        if kind == "ref":
            expr = Expression(self.text(), None)
        elif kind == "lit":
            expr = Expression(None, self.type(), self.data())
        else:
            raise SourceError("unknown expression")
        self.take(")")
        return expr

    def body(self, prefix):
        body = []
        while self.peek() != ")":
            body.append(self.statement(prefix + "/" + str(len(body))))
        return tuple(body)

    def statement(self, path):
        start = self.take("(").start
        op, name, kind, args, details, branches = self.take().text, None, None, [], [], []
        if op in ("kernel", "calc", "call"):
            name, kind = self.text(), self.type()
            details.append(self.text())
            while self.peek() != ")":
                args.append(self.expression())
        elif op == "if":
            args.append(self.expression())
            for branch in ("then", "else"):
                self.take("(")
                self.take(branch)
                branches.append(self.body(path + "/" + branch))
                self.take(")")
        elif op == "for":
            name, kind = self.text(), self.type()
            args.append(self.expression())
            branches.append(self.body(path + "/body"))
        elif op == "workspace_get":
            name, kind = self.text(), self.type()
            details.append(self.text())
        elif op == "workspace_put":
            details.append(self.text())
            args.append(self.expression())
        elif op == "preview":
            name, kind = self.text(), self.type()
            args.extend((self.expression(), self.expression()))
        elif op == "pick":
            name, kind = self.text(), self.type()
            args.append(self.expression())
            details.append(self.text())
        elif op in ("emit", "return", "dynamic"):
            args.append(self.expression())
        elif op == "publish":
            args.extend((self.expression(), self.expression()))
        elif op == "retain":
            name, kind = self.text(), self.type()
            size = self.data()
            if type(size) is not int or size < 0:
                raise SourceError("invalid logical size")
            details.append(size)
        elif op == "ambient":
            name, kind = self.text(), self.type()
            details.extend((self.text(), self.text()))
        elif op == "stop":
            details.append(self.text())
            if details != ["requested"]:
                raise SourceError("unknown stop reason")
        elif op != "tick":
            raise SourceError("unknown statement " + op)
        end = self.take(")").end
        return Statement(op, path, (start, end), name, kind, tuple(args), tuple(details), tuple(branches))

    def program(self):
        self.take("(")
        self.take("rt1")
        inputs, modules, runs = {}, {}, {}
        while self.peek() != ")":
            self.take("(")
            kind, name = self.take().text, self.text()
            if kind == "input":
                if name in inputs:
                    raise SourceError("duplicate input")
                inputs[name] = self.type()
            elif kind == "module":
                if name in modules:
                    raise SourceError("duplicate module")
                self.take("(")
                self.take("params")
                params = []
                while self.peek() != ")":
                    self.take("(")
                    params.append((self.text(), self.type()))
                    self.take(")")
                self.take(")")
                if len(dict(params)) != len(params) or params != sorted(params):
                    raise SourceError("duplicate or unordered parameters")
                result = self.type()
                modules[name] = Module(tuple(params), result, self.body("modules/" + name + "/body"))
            elif kind == "run":
                if name in runs:
                    raise SourceError("duplicate run")
                runs[name] = self.body(name + "/body")
            else:
                raise SourceError("unknown declaration")
            self.take(")")
        self.take(")")
        self.take("<eof>")
        return Program(self.source, inputs, modules, runs)


def parse(source):
    return Parser(source).program()

"""Shared tokens and value grammar for the LX-8 experiment, not a language API."""

from dataclasses import dataclass
import hashlib
import json
from math import gcd
import re


EDITION = "lx8-0"
SYNTAX_CODES = {
    "LG1:syntax.edition_missing", "LG1:syntax.edition_unsupported",
    "LG1:syntax.invalid_encoding", "LG1:syntax.invalid_literal",
    "LG1:syntax.unexpected_token", "LG1:syntax.recovery_gap",
}


def integer(text):
    """Decimal conversion without CPython's decimal-string digit ceiling."""
    sign = -1 if text.startswith("-") else 1
    digits = text.lstrip("+-")
    value = 0
    for start in range(0, len(digits), 9):
        part = digits[start:start + 9]
        value = value * 10 ** len(part) + int(part)
    return sign * value


def decimal(value):
    if value == 0:
        return "0"
    sign = "-" if value < 0 else ""
    value = abs(value)
    parts = []
    while value:
        value, part = divmod(value, 10 ** 9)
        parts.append(part)
    return sign + str(parts[-1]) + "".join(f"{p:09d}" for p in reversed(parts[:-1]))


def rational(text):
    numerator, denominator = text.split("/")
    n, d = integer(numerator), integer(denominator)
    if d == 0:
        raise ValueError("zero denominator")
    divisor = gcd(n, d) * (-1 if d < 0 else 1)
    return [decimal(n // divisor), decimal(d // divisor)]


@dataclass(frozen=True)
class Token:
    kind: str
    text: str
    start: int
    end: int
    line: int
    column: int

    def span(self):
        return {"start": self.start, "end": self.end,
                "line": self.line, "column": self.column}


class SyntaxProblem(Exception):
    def __init__(self, token, message, code="unexpected_token"):
        super().__init__(message)
        self.diagnostic = {"code": "LG1:syntax." + code,
                           "message": message, "source": token.span()}


TOKEN = re.compile(
    r'(?P<SPACE>[ \t]+)|(?P<COMMENT>\#[^\n]*)|(?P<NL>\n)'
    r'|(?P<STRING>"(?:[^"\\\n]|\\[^\n])*")'
    r'|(?P<RAT>[+-]?[0-9]+/[+-]?[0-9]+)'
    r'|(?P<BAD_NUMBER>[+-]?[0-9]+\.[0-9]*(?:[eE][+-]?[0-9]+)?)'
    r'|(?P<INT>[+-]?[0-9]+)|(?P<ID>[A-Za-z_][A-Za-z_0-9]*)'
    r'|(?P<PUNCT>->|=>|:=|<-|\|>|[{}()\[\],;:<>!=@~|])'
    r'|(?P<BAD>.)'
)


def lex(source, layout):
    offsets = [0]
    for char in source:
        offsets.append(offsets[-1] + len(char.encode("utf-8")))
    tokens, line, line_start = [], 1, 0
    for match in TOKEN.finditer(source):
        kind, text = match.lastgroup, match.group()
        column = match.start() - line_start + 1
        if kind == "SPACE" and layout and "\t" in text:
            kind = "BAD"
        if kind not in {"SPACE", "COMMENT"} and (kind != "NL" or layout):
            tokens.append(Token(kind, text, offsets[match.start()], offsets[match.end()],
                                line, column))
        if kind == "NL":
            line += 1
            line_start = match.end()
    tokens.append(Token("EOF", "", offsets[-1], offsets[-1], line,
                        len(source) - line_start + 1))
    return tokens


class Parser:
    """Cursor, shared atom grammar, and declaration-boundary draft recovery."""
    layout = False
    starts = {"fn", "command", "view", "import"}

    def __init__(self, tokens):
        self.tokens, self.pos = tokens, 0

    @property
    def token(self):
        return self.tokens[self.pos]

    def take(self):
        token = self.token
        if token.kind != "EOF":
            self.pos += 1
        return token

    def at(self, text):
        return self.token.text == text

    def accept(self, text):
        if self.at(text):
            return self.take()
        return None

    def expect(self, text):
        if not self.at(text):
            raise SyntaxProblem(self.token, "expected " + repr(text))
        return self.take()

    def name(self):
        if self.token.kind != "ID":
            raise SyntaxProblem(self.token, "expected an identifier")
        return self.take().text

    def string(self):
        token = self.token
        if token.kind != "STRING":
            raise SyntaxProblem(token, "expected a quoted string")
        self.take()
        try:
            value = json.loads(token.text)
            value.encode("utf-8")
            return value
        except (ValueError, UnicodeError):
            raise SyntaxProblem(token, "invalid string literal", "invalid_literal") from None

    def natural(self):
        token = self.token
        if token.kind != "INT":
            raise SyntaxProblem(token, "expected an exact nonnegative integer", "invalid_literal")
        self.take()
        if integer(token.text) < 0:
            raise SyntaxProblem(token, "negative count", "invalid_literal")
        return decimal(integer(token.text))

    def separated(self, close, item):
        values = []
        if not self.at(close):
            values.append(item())
            while self.accept(","):
                values.append(item())
        self.expect(close)
        return values

    def names(self):
        self.expect("[")
        return self.separated("]", self.name)

    def charges(self):
        self.expect("[")
        values = self.separated("]", self.natural)
        if len(values) != 4:
            raise SyntaxProblem(self.token, "charge needs steps, kernel, memory, output")
        return dict(zip(("steps", "kernel", "memory", "output"), values))

    def type(self):
        name = self.name()
        if self.accept("<"):
            args = self.separated(">", self.type)
            if not args:
                raise SyntaxProblem(self.token, "empty type argument list")
            name += "<" + ",".join(args) + ">"
        return name

    def params(self):
        def param():
            name = self.name()
            self.expect(":")
            return {"name": name, "type": self.type()}
        self.expect("(")
        return self.separated(")", param)

    def node(self, start, kind, **fields):
        span = start.span()
        span["end"] = self.tokens[max(self.pos - 1, 0)].end
        return {"kind": kind, **fields, "source": span}

    def expression(self):
        return self.atom()

    def atom(self):
        token = self.token
        if token.kind == "INT":
            self.take()
            return self.node(token, "int", value=decimal(integer(token.text)))
        if token.kind == "RAT":
            self.take()
            try:
                value = rational(token.text)
            except ValueError:
                raise SyntaxProblem(token, "zero denominator", "invalid_literal") from None
            return self.node(token, "rat", value=value)
        if token.kind == "STRING":
            return self.node(token, "text", value=self.string())
        if token.kind == "BAD_NUMBER":
            raise SyntaxProblem(token, "approximation needs a precision marker", "invalid_literal")
        if self.accept("~"):
            precision = self.natural()
            value = self.string()
            if precision not in {"24", "53", "113"} or not re.fullmatch(
                    r"[+-]?[0-9]+(?:\.[0-9]+)?(?:[eE][+-]?[0-9]+)?", value):
                raise SyntaxProblem(token, "invalid approximate precision or decimal", "invalid_literal")
            return self.node(token, "approx", value={"decimal": value, "precision_bits": precision})
        if self.accept("q5"):
            self.expect("(")
            a = self.atom()
            self.expect(",")
            b = self.atom()
            self.expect(")")
            if a["kind"] != "rat" or b["kind"] != "rat":
                raise SyntaxProblem(token, "q5 needs two rational coefficients", "invalid_literal")
            return self.node(token, "q5", value={"one": a["value"], "sqrt5": b["value"]})
        if self.at("word") or self.at("menu"):
            kind = self.take().text
            self.expect("[")

            def letter():
                start = self.token
                if kind == "word":
                    if start.kind != "INT" or not 1 <= abs(integer(start.text)) <= 1200:
                        raise SyntaxProblem(start, "retained generator outside 1..1200", "invalid_literal")
                    self.take()
                    return decimal(integer(start.text))
                grip = self.natural()
                self.expect(":")
                rotation = self.name()
                if integer(grip) >= 120 or rotation not in {"spin", "unspin", "identity"}:
                    raise SyntaxProblem(start, "unknown fixture menu letter", "invalid_literal")
                return {"grip": grip, "rotation": rotation}

            return self.node(token, kind, value=self.separated("]", letter))
        if self.accept("("):
            node = self.expression()
            self.expect(")")
            return node
        name = self.name()
        if self.accept("("):
            args = self.separated(")", self.expression)
            return self.node(token, "call", name=name, args=args)
        return self.node(token, "ref", name=name)

    def metadata(self):
        self.expect("row")
        row = self.string()
        self.expect("kind")
        kind = self.name()
        self.expect("contexts")
        contexts = self.names()
        self.expect("permits")
        permits = self.names()
        return {"row": row, "kind": kind, "contexts": contexts, "permits": permits}

    def arm(self):
        start = self.token
        tag = self.name()
        self.expect("(")
        binder = self.name()
        self.expect(")")
        self.expect("=>")
        return {"tag": tag, "binder": binder, "body": self.expression(), "source": start.span()}

    def blanks(self):
        while self.token.kind == "NL":
            self.take()

    def line_end(self):
        if self.token.kind != "NL":
            raise SyntaxProblem(self.token, "expected end of line")
        self.blanks()

    def declaration_start(self):
        return self.token.column == 1 and self.token.text in self.starts

    def recover(self, old_position):
        if self.pos <= old_position:
            self.take()
        while self.token.kind != "EOF" and not self.declaration_start():
            self.take()


def parse(data, parser_class):
    result = {"edition": None, "module": None, "requires": [], "imports": [],
              "declarations": [], "diagnostics": [], "holes": [],
              "digest": hashlib.sha256(data).hexdigest()}
    try:
        source = data.decode("utf-8", errors="strict")
    except UnicodeDecodeError as error:
        prefix = data[:error.start].decode("utf-8")
        token = Token("BAD", "", error.start, error.end, prefix.count("\n") + 1,
                      len(prefix.rsplit("\n", 1)[-1]) + 1)
        result["diagnostics"].append(SyntaxProblem(token, "source is not UTF-8", "invalid_encoding").diagnostic)
        return result
    if "\r" in source or source.startswith("\ufeff"):
        index = source.find("\r") if "\r" in source else 0
        prefix = source[:index]
        token = Token("BAD", "", len(prefix.encode("utf-8")), len(prefix.encode("utf-8")) + 1,
                      prefix.count("\n") + 1, len(prefix.rsplit("\n", 1)[-1]) + 1)
        result["diagnostics"].append(SyntaxProblem(token, "source must be LF UTF-8 without BOM", "invalid_encoding").diagnostic)
        return result
    parser = parser_class(lex(source, parser_class.layout))
    parser.blanks()
    try:
        edition_token = parser.token
        if not parser.has_header():
            raise SyntaxProblem(parser.token, "edition header required", "edition_missing")
        result["edition"] = parser.header()
        if result["edition"] != EDITION:
            raise SyntaxProblem(edition_token, "unsupported edition", "edition_unsupported")
        result["module"], result["requires"] = parser.module()
    except SyntaxProblem as error:
        result["diagnostics"].append(error.diagnostic)
        return result
    while parser.token.kind != "EOF":
        parser.blanks()
        if parser.token.kind == "EOF":
            break
        position, start = parser.pos, parser.token
        try:
            if start.column != 1:
                raise SyntaxProblem(start, "declaration must start in column 1")
            item = parser.declaration()
            result["imports" if item["kind"] == "import" else "declarations"].append(item)
        except SyntaxProblem as error:
            result["diagnostics"].append(error.diagnostic)
            parser.recover(position)
            span = start.span()
            span["end"] = parser.token.start
            result["holes"].append({"kind": "hole", "source": span})
            result["diagnostics"].append({"code": "LG1:syntax.recovery_gap",
                                           "message": "declaration omitted; draft cannot lower", "source": span})
    return result

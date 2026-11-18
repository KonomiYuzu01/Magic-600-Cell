"""Source scans of the language workspace for LP2: the emitted-code halves of CT-ERR-01 to CT-ERR-17, CT-BND-24,
CT-KN6-02 and CT-RPI-20.

The scans read the Rust workspace in place (`rust/Cargo.toml`'s members; `rust/vendor` and `target` only where a
dependency is inspected) and the Python tools that compare a producer's output. Chapter 13's registry comes from
`tests/test_error_registry.py`, which parses it in place. Each scan also runs on planted text or a planted temporary
crate, which shows that it can fail.

This is source evidence. A scan proves what the current source contains, not how a running system behaves; a scan
that finds nothing to check is no pass (PHS.63), and each test names the half it does not run. Nothing here is
Windows, DirectX, input, long-session or performance evidence.
"""
from __future__ import annotations

import ast
import bisect
import dataclasses
import re
import sys
import tempfile
import unittest
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import test_error_registry as reg  # noqa: E402

ROOT = Path(__file__).resolve().parents[1]
RUST = ROOT / "rust"
TOOLS = ROOT / "tools"

# The members allowed to call Windows APIs: the platform layer of RPI.52. Its first member holds the session lock
# file's open (the owner's reading of finding E1 of the KN5 boundary attack, 18 November 2026,
# docs/wiki/decisions/owner-decisions-2026-11-18.md; the layer-boundary change approved under PHS.52).
PLATFORM_LAYER: frozenset[str] = frozenset({"c600-platform"})
# KN6 checkers, by source path, with the producer modules they must not import (KN6.2). LP2 has none: KN6.1's checker
# kinds arrive with LP5 (CT-KN6-01, CT-KN6-03).
KN6_CHECKERS: dict[str, tuple[str, ...]] = {}
# Sources that define KN6.1's `CheckReport` to read one and run no check, with the producer crates they must not
# import. The evidence library admits a report as one leaf (EVD.46) and reads no program or session (EVD.14, EVD.54),
# so it imports neither the interpreter that runs programs nor the session store that runs their jobs.
KN6_REPORT_READERS: dict[str, tuple[str, ...]] = {
    "rust/c600-evidence/src/sink.rs": ("c600_run", "c600_session"),
}
# LP2's one checker of a producer's output: the O01 to O07 oracle comparison. It must import neither the producers of
# its references (the case maker and the trace replayer) nor the 0.4 engine they construct them with.
ORACLE_CHECKERS = {
    "tools/oracle/compare.py": ("tools.oracle.make_cases", "tools.oracle.replay_trace", "core", "session"),
}
# KN6.1's checker interface; a definition of one of these names makes a file a checker that must be declared above.
CHECKER_NAMES = ("CheckReport", "CheckerKind", "CertificateRef")
# Slugs whose codes no crate may emit or assemble yet (ERR.5, ERR.22, ERR.24; RM.13).
GUARDED_SLUGS = {
    "flow_refused": "LG3:effect.flow_refused is retired; LG3:capability.flow_refused waits for LQ2 (ERR.24)",
    "display_plan_limit": "KN7:budget.display_plan_limit is reserved (RM.13)",
    "recursion_exhausted": "KN7:resource_limit.recursion_exhausted is reserved",
    "chart_subset_violation": "LG2:type.chart_subset_violation is withdrawn",
    "outside_certified_domain": "the outside-domain spelling is withdrawn, not renamed (ERR.22)",
}
# Names a crate defines for a reserved code only to recognise it: (path, name, code). The definition must read exactly
# `pub const <name>: &str = "<code>";`, and every other use of the name compares a code with it, so no source builds the
# reserved code (ERR.25: the fixed-ceiling draft never emits KN7:budget.display_plan_limit; RM.13).
RESERVED_NAMES = (
    ("rust/c600-display/src/error.rs", "PLAN_LIMIT", "KN7:budget.display_plan_limit"),
)
# Codes that LP3's command layer and LP2's session store name as drafts, which chapter 13 does not register yet, by
# path (PHS.53: the command table's drafts settle at LP3; a new registry row is a change to chapter 13, PHS.52). ERR.1
# is not run for them, and the list is exact: a code that becomes registered or leaves its source fails the scan until
# the list follows.
UNREGISTERED_DRAFTS = frozenset({
    ("rust/c600-command/src/layer.rs", "KN2:consistency.star_certificate"),
    ("rust/c600-command/src/receipt.rs", "KN5:input.unknown_row"),
    ("rust/c600-command/src/receipt.rs", "KN5:input.argument_form"),
    ("rust/c600-command/src/receipt.rs", "KN5:input.row_not_built"),
    ("rust/c600-command/src/receipt.rs", "KN5:input.overlay_row_conflict"),
    ("rust/c600-command/src/receipt.rs", "KN4:input.session_exists"),
    ("rust/c600-command/src/receipt.rs", "KN5:consistency.unadmitted_journal_action"),
    ("rust/c600-command/src/receipt.rs", "KN5:capability.preview_shown"),
    ("rust/c600-command/src/receipt.rs", "KN5:capability.outside_solver_command"),
    ("rust/c600-session/src/types.rs", "KN4:busy.session_locked"),
})

CODE_IN_TEXT = re.compile(r"(?<![A-Za-z0-9_])([A-Z]{2,3}[0-9]?):([a-z_]+)\.([a-z0-9_]+)(?![A-Za-z0-9_])")
SHORT_CODE = re.compile(r"^([a-z_]+)\.([a-z0-9_]+)$")
TOKEN = re.compile(r"""//|/\*|(?<![A-Za-z0-9_])b?r(#*)"|(?<![A-Za-z0-9_])b"|"|'""")
# Handling that decides by an error's text instead of its code (ERR.6).
TEXT_FIELD = r"(?:message|summary|diagnostic)"
TEXT_HANDLING = (
    re.compile(rf"\.{TEXT_FIELD}\s*(?:==|!=)"),
    re.compile(rf"(?:==|!=)\s*&?\s*[\w.]*\.{TEXT_FIELD}\b(?!\s*\()"),
    re.compile(rf"\.{TEXT_FIELD}\s*\.\s*(?:as_str\(\)\s*\.\s*)?(?:contains|starts_with|ends_with|find|rfind|matches|"
               rf"strip_prefix|strip_suffix|split|eq|eq_ignore_ascii_case)\s*\("),
    re.compile(rf"\bmatch\s+&?\s*[\w.]*\.{TEXT_FIELD}\b"),
    re.compile(r"\.to_string\(\)\s*\.\s*(?:contains|starts_with|ends_with|find)\s*\("),
)
# Windows APIs in Rust sources (RPI.51 to RPI.53), read with comments removed.
WINDOWS_SOURCE = (
    (re.compile(r"\bstd\s*::\s*os\s*::\s*windows\b"), "std::os::windows"),
    (re.compile(r"(?<![.\w:])(?:windows|windows_sys|windows_core|windows_targets|winapi)\s*::"),
     "a Windows API crate"),
    (re.compile(r'\bextern\s+"(?:system|stdcall|win64|vectorcall|thiscall|fastcall)(?:-unwind)?"'), "a Windows ABI"),
    (re.compile(r"#\s*!?\s*\[\s*link\s*\("), "a #[link] attribute"),
    (re.compile(r"\bwindows_subsystem\b"), "windows_subsystem"),
    (re.compile(r"rustc-link-lib"), "a native link instruction"),
)
WINDOWS_CFG = re.compile(r"""\bwindows\b|"windows"|"msvc"|"gnullvm\"""")
CFG_OPEN = re.compile(r"\bcfg(?:_attr)?\s*!?\s*\(")
WINDOWS_PACKAGE = re.compile(r"^(?:windows|winapi|winreg|winres|embed-resource|windows-sys|windows-core|"
                             r"windows-targets|windows_[a-z0-9_]+|winapi-[a-z0-9_-]+)$")
SHADER_SUFFIXES = (".hlsl", ".hlsli", ".fx", ".fxh")


@dataclasses.dataclass(frozen=True)
class Lexed:
    """A Rust file with comments blanked (`code`), with comments and literal contents blanked (`bare`), and its
    string literals with the line each starts on. Blanking keeps every newline, so offsets keep their lines."""
    code: str
    bare: str
    literals: tuple[tuple[int, str], ...]


@dataclasses.dataclass(frozen=True)
class Emitted:
    path: str
    line: int
    code: str
    literal: str


def blank(text: str) -> str:
    return "".join(c if c == "\n" else " " for c in text)


def lex_rust(text: str) -> Lexed:
    """A small Rust lexer: line and nested block comments, plain, byte and raw strings, and char literals, which it
    tells apart from lifetimes and labels."""
    newlines = [i for i, c in enumerate(text) if c == "\n"]
    code, bare, literals, i = [], [], [], 0

    def emit(kept: str, stripped: str) -> None:
        code.append(kept)
        bare.append(stripped)

    while True:
        m = TOKEN.search(text, i)
        if m is None:
            emit(text[i:], text[i:])
            break
        emit(text[i:m.start()], text[i:m.start()])
        start, token = m.start(), m.group(0)
        if token == "//":
            end = text.find("\n", start)
            end = len(text) if end < 0 else end
            emit(blank(text[start:end]), blank(text[start:end]))
            i = end
        elif token == "/*":
            depth, end = 1, m.end()
            while depth and end < len(text):
                if text.startswith("/*", end):
                    depth, end = depth + 1, end + 2
                elif text.startswith("*/", end):
                    depth, end = depth - 1, end + 2
                else:
                    end += 1
            emit(blank(text[start:end]), blank(text[start:end]))
            i = end
        elif token == "'":
            if text.startswith("\\", start + 1):
                end = text.find("'", start + 3)
            elif start + 2 < len(text) and text[start + 2] == "'":
                end = start + 2
            else:
                emit("'", "'")
                i = start + 1
                continue
            emit(text[start:end + 1], "'" + blank(text[start + 1:end]) + "'")
            i = end + 1
        else:
            body = m.end()
            if m.group(1) is not None:
                close = '"' + m.group(1)
                end = text.find(close, body)
                if end < 0:
                    raise ValueError(f"unterminated raw string at offset {start}")
                after = end + len(close)
            else:
                end = body
                while text[end] != '"':
                    end += 2 if text[end] == "\\" else 1
                after = end + 1
            literals.append((bisect.bisect_left(newlines, body) + 1, text[body:end]))
            emit(text[start:after], token + blank(text[body:end]) + text[end:after])
            i = after
    return Lexed("".join(code), "".join(bare), tuple(literals))


def line_of(text: str, offset: int) -> int:
    return text.count("\n", 0, offset) + 1


def workspace_members(rust: Path = RUST) -> list[str]:
    manifest = (rust / "Cargo.toml").read_text(encoding="utf-8")
    match = re.search(r"^members\s*=\s*\[(.*?)\]", manifest, re.MULTILINE | re.DOTALL)
    if match is None:
        raise ValueError("rust/Cargo.toml lists no workspace members")
    return re.findall(r'"([^"]+)"', match.group(1))


def crate_sources(crate: Path) -> list[Path]:
    """A crate's product sources: src/ (with src/bin) and a build script; never its tests, benches or examples."""
    found = sorted((crate / "src").rglob("*.rs"))
    if (crate / "build.rs").is_file():
        found.append(crate / "build.rs")
    return found


def lexed_sources(rust: Path = RUST) -> dict[str, Lexed]:
    root = rust.parent
    return {path.relative_to(root).as_posix(): lex_rust(path.read_text(encoding="utf-8"))
            for member in workspace_members(rust) for path in crate_sources(rust / member)}


def reservation_lines(sources: dict[str, Lexed]) -> dict[tuple[str, int], str]:
    """The lines that define a reserved code's name exactly as RESERVED_NAMES records it, with the code each names."""
    found = {}
    for path, name, code in RESERVED_NAMES:
        if path in sources:
            for n, text in enumerate(sources[path].code.split("\n"), 1):
                if text.strip() == f'pub const {name}: &str = "{code}";':
                    found[(path, n)] = code
    return found


def reservation_problems(sources: dict[str, Lexed]) -> list[str]:
    """Each use of a reserved code's name, other than its definition, compares a code with it (ERR.25, RM.13)."""
    defined = reservation_lines(sources)
    problems = []
    for _, name, _ in RESERVED_NAMES:
        use = re.compile(rf"(?<![A-Za-z0-9_]){name}(?![A-Za-z0-9_])")
        compared = re.compile(rf"(?:==|!=)\s*(?:[A-Za-z_]\w*\s*::\s*)*{name}(?![A-Za-z0-9_])|"
                              rf"(?<![A-Za-z0-9_]){name}\s*(?:==|!=)")
        for path, lexed in sources.items():
            for n, text in enumerate(lexed.bare.split("\n"), 1):
                if (path, n) not in defined and len(use.findall(text)) > len(compared.findall(text)):
                    problems.append(f"{path}:{n}: the reserved {name} is used other than in a comparison")
    return problems


def emitted_codes(sources: dict[str, Lexed], registry: reg.Registry) -> list[Emitted]:
    """Every registry-shaped code inside a string literal: a prefix that is a registry source or a category that is a
    registry category. Other identifiers of that shape are no error codes. A reserved code's definition under
    RESERVED_NAMES names the code without emitting it, and reservation_problems checks how that name is used."""
    reserved = reservation_lines(sources)
    found = []
    for path, lexed in sources.items():
        for line, literal in lexed.literals:
            if reserved.get((path, line)) == literal:
                continue
            for m in CODE_IN_TEXT.finditer(literal):
                if m.group(1) in registry.sources or m.group(2) in registry.categories:
                    found.append(Emitted(path, line, m.group(0), literal))
    return found


def draft_problems(sources: dict[str, Lexed], registry: reg.Registry) -> list[str]:
    """UNREGISTERED_DRAFTS is exact: each draft is still named at its path and still unregistered."""
    emitted = {(e.path, e.code) for e in emitted_codes(sources, registry)}
    return [f"{path}: {code} is " + ("registered now" if registry.row(code) else "no longer named there")
            for path, code in sorted(UNREGISTERED_DRAFTS)
            if registry.row(code) is not None or (path, code) not in emitted]


def emitted_problems(sources: dict[str, Lexed], registry: reg.Registry) -> list[str]:
    """ERR.1, ERR.2, ERR.4, ERR.5: each emitted code is an active registry row, and none drops its prefix. A draft under
    UNREGISTERED_DRAFTS, at its own path, is the one exception (not run; draft_problems keeps the list exact)."""
    problems = []
    for e in emitted_codes(sources, registry):
        row = registry.row(e.code)
        if row is None:
            if (e.path, e.code) not in UNREGISTERED_DRAFTS:
                problems.append(f"{e.path}:{e.line}: {e.code} is not registered")
        elif row.status != "active":
            problems.append(f"{e.path}:{e.line}: {e.code} is {row.status} and cannot be emitted")
    short = {f"{row.category}.{row.slug}": row.code for row in registry.rows}
    for path, lexed in sources.items():
        for line, literal in lexed.literals:
            if SHORT_CODE.match(literal) and literal in short:
                problems.append(f"{path}:{line}: {literal!r} drops the prefix of {short[literal]}")
    return problems


def guarded_problems(sources: dict[str, Lexed]) -> list[str]:
    reserved = reservation_lines(sources)
    return [f"{path}:{line}: a literal names {slug!r}: {why}" for path, lexed in sources.items()
            for line, literal in lexed.literals for slug, why in GUARDED_SLUGS.items()
            if slug in literal and reserved.get((path, line)) != literal]


def category_problems(sources: dict[str, Lexed], registry: reg.Registry) -> list[str]:
    """ERR.16, ERR.18, ERR.23: emitted consistency codes come from KN1 to KN7, emitted budget codes are active KN7
    budget rows on a declared allowance, and no code has the KN3 prefix."""
    problems = []
    for e in emitted_codes(sources, registry):
        prefix, category, _ = reg.split_code(e.code)
        row = registry.row(e.code)
        if prefix == "KN3":
            problems.append(f"{e.path}:{e.line}: {e.code} uses KN3, a rule source and never a raising family")
        if category == "consistency" and not re.fullmatch(r"KN[1-7]", prefix):
            problems.append(f"{e.path}:{e.line}: {e.code} freezes outside the kernel")
        if category == "budget" and (prefix != "KN7" or row is None or row.status != "active"
                                     or not row.meaning.startswith("Declared")):
            problems.append(f"{e.path}:{e.line}: {e.code} is no active KN7 budget on a declared allowance")
    return problems


def text_handling_problems(sources: dict[str, Lexed]) -> list[str]:
    problems = []
    for path, lexed in sources.items():
        for pattern in TEXT_HANDLING:
            for m in pattern.finditer(lexed.bare):
                problems.append(f"{path}:{line_of(lexed.bare, m.start())}: handling reads text: {m.group(0)!r}")
    return problems


def move_selection_problems(sources: dict[str, Lexed]) -> list[str]:
    return [f"{path}:{line}: a message selects a twist or correction: {literal!r}"
            for path, lexed in sources.items() for line, literal in lexed.literals
            if reg.SELECTS_A_MOVE.search(literal)]


def python_imports(path: Path, root: Path = ROOT) -> set[str]:
    """Every module a Python file imports: import statements (relative ones resolved), and the constant arguments of
    importlib.import_module, __import__, runpy and file-location loaders."""
    tree = ast.parse(path.read_text(encoding="utf-8"), filename=str(path))
    package = path.relative_to(root).with_suffix("").parts[:-1]
    names = set()
    for node in ast.walk(tree):
        if isinstance(node, ast.Import):
            names |= {alias.name for alias in node.names}
        elif isinstance(node, ast.ImportFrom):
            base = node.module or ""
            if node.level:
                parent = package[:len(package) - node.level + 1]
                base = ".".join(part for part in (*parent, *base.split(".")) if part)
            names.add(base)
            names |= {f"{base}.{alias.name}" if base else alias.name for alias in node.names}
        elif isinstance(node, ast.Call):
            func = node.func
            name = func.attr if isinstance(func, ast.Attribute) else func.id if isinstance(func, ast.Name) else ""
            if name in ("import_module", "__import__", "run_module", "run_path", "spec_from_file_location",
                        "SourceFileLoader"):
                for arg in node.args:
                    if isinstance(arg, ast.Constant) and isinstance(arg.value, str):
                        value = arg.value.replace("\\", "/")
                        if value.endswith(".py"):
                            names |= {value[:-3].replace("/", "."), value[:-3].rsplit("/", 1)[-1]}
                        else:
                            names.add(value)
    return names


def imports_module(names: set[str], module: str) -> bool:
    """Whether `names` reach `module`: by its full name, or, for a module inside a package, by its own name as a
    sibling import or a loaded file would give it."""
    forms = (module, module.rsplit(".", 1)[-1]) if "." in module else (module,)
    return any(name in forms or name.startswith(tuple(form + "." for form in forms))
               or name.endswith(tuple("." + form for form in forms if "." in form)) for name in names)


def python_import_problems(checkers: dict[str, tuple[str, ...]], root: Path = ROOT) -> list[str]:
    problems = []
    for rel, producers in checkers.items():
        names = python_imports(root / rel, root)
        problems += [f"{rel} imports producer code {module}" for module in producers if imports_module(names, module)]
    return problems


def cargo_dependencies(manifest: str) -> set[str]:
    """The package names of every dependency table of a Cargo manifest, with `-` read as `_`."""
    names, section = set(), ""
    for line in manifest.split("\n"):
        stripped = line.strip()
        header = re.match(r"^\[(.+)\]$", stripped)
        if header:
            section = header.group(1)
            dotted = re.match(r"^(?:target\..+\.)?(?:dev-|build-)?dependencies\.([A-Za-z0-9_-]+)$", section)
            if dotted:
                names.add(dotted.group(1).replace("-", "_"))
            continue
        if re.search(r"(?:^|\.)(?:dev-|build-)?dependencies$", section):
            key = re.match(r"^([A-Za-z0-9_-]+)\s*=", stripped)
            if key:
                names.add(key.group(1).replace("-", "_"))
                package = re.search(r'\bpackage\s*=\s*"([^"]+)"', stripped)
                if package:
                    names.add(package.group(1).replace("-", "_"))
    return names


def rust_import_problems(crate: Path, producers: tuple[str, ...]) -> list[str]:
    """KN6.2 for a Rust checker crate: no dependency on a producer crate, no path into a producer's module, and no
    `#[path]` or `include!` that reads a producer's file. A producer is a crate name or a crate::module path."""
    problems = []
    dependencies = cargo_dependencies((crate / "Cargo.toml").read_text(encoding="utf-8"))
    crates = {producer.split("::", 1)[0] for producer in producers}
    for name in sorted(crates & dependencies):
        if name in producers:
            problems.append(f"{crate.name}: depends on producer crate {name}")
    for path in crate_sources(crate):
        lexed = lex_rust(path.read_text(encoding="utf-8"))
        rel = path.relative_to(crate).as_posix()
        named = {}
        for producer in sorted(producers, key=len):
            pattern = re.compile(r"(?<![\w:])" + r"\s*::\s*".join(map(re.escape, producer.split("::"))) + r"\b")
            for m in pattern.finditer(lexed.bare):
                named[m.start()] = producer
        for offset, producer in sorted(named.items()):
            problems.append(f"{crate.name}/{rel}:{line_of(lexed.bare, offset)}: names producer code {producer}")
        for m in re.finditer(r'#\s*\[\s*path\s*=\s*"([^"]*)"|\binclude(?:_str|_bytes)?!\s*\(\s*"([^"]*)"', lexed.code):
            target = (path.parent / (m.group(1) or m.group(2))).resolve()
            for producer in crates:
                home = (crate.parent / producer.replace("_", "-")).resolve()
                if home in target.parents:
                    problems.append(f"{crate.name}/{rel}:{line_of(lexed.code, m.start())}: reads producer file "
                                    f"{target.name}")
    return problems


def checker_inventory(sources: dict[str, Lexed], tools: Path = TOOLS) -> set[str]:
    """Files that define KN6.1's checker interface: Rust sources of the workspace and Python files under tools/."""
    names = "|".join(CHECKER_NAMES)
    rust = re.compile(rf"\b(?:struct|enum|type|trait|union)\s+(?:{names})\b")
    found = {path for path, lexed in sources.items() if rust.search(lexed.bare)}
    root = tools.parent
    for path in sorted(tools.rglob("*.py")):
        if ".venv" in path.parts or "node_modules" in path.parts:
            continue
        tree = ast.parse(path.read_text(encoding="utf-8"), filename=str(path))
        if any(isinstance(node, ast.ClassDef) and node.name in CHECKER_NAMES for node in ast.walk(tree)):
            found.add(path.relative_to(root).as_posix())
    return found


def cfg_windows(code: str) -> list[int]:
    """Offsets of every cfg(...), cfg!(...) or cfg_attr(...) predicate that names a Windows target."""
    found = []
    for m in CFG_OPEN.finditer(code):
        depth, end = 1, m.end()
        while depth and end < len(code):
            depth += {"(": 1, ")": -1}.get(code[end], 0)
            end += 1
        if WINDOWS_CFG.search(code[m.end():end]):
            found.append(m.start())
    return found


def windows_problems(rust: Path = RUST, platform: frozenset[str] = PLATFORM_LAYER) -> list[str]:
    """RPI.51 to RPI.53: Windows packages in the lock file, Windows dependencies or targets in a member's manifest,
    and Windows APIs in member or vendored sources, outside the platform layer."""
    problems = []
    lock = rust / "Cargo.lock"
    if lock.is_file():
        for name in re.findall(r'^name\s*=\s*"([^"]+)"', lock.read_text(encoding="utf-8"), re.MULTILINE):
            if WINDOWS_PACKAGE.match(name):
                problems.append(f"Cargo.lock: package {name}")
    crates = [(member, rust / member) for member in workspace_members(rust)]
    vendor = rust / "vendor"
    if vendor.is_dir():
        crates += [(f"vendor/{path.name}", path) for path in sorted(vendor.iterdir()) if path.is_dir()]
    for name, crate in crates:
        if name in platform:
            continue
        manifest = (crate / "Cargo.toml").read_text(encoding="utf-8")
        for dependency in sorted(cargo_dependencies(manifest)):
            if WINDOWS_PACKAGE.match(dependency.replace("_", "-")) or WINDOWS_PACKAGE.match(dependency):
                problems.append(f"{name}/Cargo.toml: depends on {dependency}")
        for m in re.finditer(r"^\[target\.([^\]]+)\]", manifest, re.MULTILINE):
            if "windows" in m.group(1) or cfg_windows(m.group(1)):
                problems.append(f"{name}/Cargo.toml: a Windows target table [target.{m.group(1)}]")
        for path in crate_sources(crate):
            code = lex_rust(path.read_text(encoding="utf-8")).code
            rel = f"{name}/{path.relative_to(crate).as_posix()}"
            for pattern, what in WINDOWS_SOURCE:
                problems += [f"{rel}:{line_of(code, m.start())}: {what}" for m in pattern.finditer(code)]
            problems += [f"{rel}:{line_of(code, offset)}: a Windows cfg" for offset in cfg_windows(code)]
    return problems


def shader_sources(rust: Path = RUST) -> list[str]:
    return sorted(path.relative_to(rust.parent).as_posix() for path in rust.rglob("*")
                  if path.suffix.lower() in SHADER_SUFFIXES and "target" not in path.relative_to(rust).parts)


def planted(text: str, path: str = "rust/planted/src/lib.rs") -> dict[str, Lexed]:
    return {path: lex_rust(text)}


def write(path: Path, text: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(text.encode("utf-8"))


class ScanTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.registry = reg.load_registry()
        cls.sources = lexed_sources()
        cls.emitted = emitted_codes(cls.sources, cls.registry)

    def test_the_lexer_reads_rust_literals_and_comments(self):
        """The lexer finds string literals and skips comments, so a code in a comment or doc comment is never read
        as emitted and a lifetime is never read as a char literal."""
        text = "\n".join([
            "/// `KN5:stale.token_used` in a doc comment", "//! KN5:stale.token_used", "/* outer /* nested */ KN5:x */",
            "fn f<'a>(x: &'a str) -> char { let _ = \"KN1:input.unknown_model\";"
            " let _ = r#\"a \"q\" KN2:input.recipe_shape\"#;",
            "    let _ = b\"LG1:syntax.invalid_encoding\"; let _ = '\\''; let _ = '\"'; 'z' }",
            "let s = \"esc \\\" KN7:budget.steps\";"])
        lexed = lex_rust(text)
        self.assertEqual([(line, CODE_IN_TEXT.search(lit).group(0)) for line, lit in lexed.literals
                          if CODE_IN_TEXT.search(lit)],
                         [(4, "KN1:input.unknown_model"), (4, "KN2:input.recipe_shape"),
                          (5, "LG1:syntax.invalid_encoding"), (6, "KN7:budget.steps")])
        self.assertNotIn("token_used", lexed.code)
        self.assertEqual(lexed.code.count("\n"), text.count("\n"))
        self.assertNotIn("KN1", lexed.bare)
        self.assertIn("'a", lexed.bare)

    def test_ct_err_01_emitted_codes_are_registered_and_active(self):
        """CT-ERR-01 (part: the emitted half; not run: envelope.category and the reported origin of a raised envelope,
        which exist only from LP3's command layer; the registry half is in tests/test_error_registry.py): every code
        the workspace members' sources name in a string literal is an active registry row with its own prefix, and no
        literal drops a registered code's prefix. This is also CT-ERR-04's emitted half (forwarded codes keep their
        full prefixes) and CT-ERR-17's (the withdrawn chart-subset code is not emitted). Not run either: ERR.1 for the
        seven drafts that LP3's command layer and LP2's session store name and chapter 13 does not register
        (UNREGISTERED_DRAFTS, a finding for the owner under PHS.52); the list is exact, and the same code at another
        path fails. Controls: an unregistered code, a reserved code, a retired code, a short form and a draft outside
        its path fail; a code in a comment is not read."""
        self.assertEqual(emitted_problems(self.sources, self.registry), [])
        self.assertEqual(draft_problems(self.sources, self.registry), [])
        self.assertGreaterEqual(len({e.code for e in self.emitted}), 25)
        for crate in ("c600-check", "c600-kernel", "c600-run", "c600-syntax", "c600-command", "c600-display"):
            self.assertTrue(any(e.path.startswith(f"rust/{crate}/") for e in self.emitted), crate)
        found = emitted_problems(planted("\n".join([
            '// "KN5:stale.never_registered" in a comment',
            'const A: &str = "KN5:stale.never_registered";', 'const B: &str = "KN7:budget.display_plan_limit";',
            'const C: &str = "KN7:budget.over_ceiling";', 'const D: &str = "stale.token_used";',
            'const E: &str = "KN5:input.unknown_row";'])), self.registry)
        self.assertEqual(sorted(p.split(": ", 1)[0] for p in found),
                         [f"rust/planted/src/lib.rs:{n}" for n in (2, 3, 4, 5, 6)])
        stale = draft_problems(planted('const E: &str = "KN5:input.unknown_row";', "rust/c600-command/src/receipt.rs"),
                               self.registry)
        self.assertEqual(len(stale), len(UNREGISTERED_DRAFTS) - 1)

    def test_ct_err_15_retired_and_reserved_codes_are_not_emitted(self):
        """CT-ERR-15 (part: the emitted half for the present source, which includes RM.13's fixed-ceiling path in
        LP3's c600-display; not run: a display run at its ceiling, which is LP3's): no member names or assembles
        LG3:effect.flow_refused, LG3:capability.flow_refused, KN7:resource_limit.recursion_exhausted, or the
        withdrawn chart-subset and outside-domain codes, in any string literal. The reserved
        KN7:budget.display_plan_limit appears once, as c600-display's constant for ERR.25's reservation, and every
        other use of that constant compares a code with it, so no source builds the code. Controls: each guarded slug
        planted in a literal fails, and in a comment it does not; the reservation constant outside its path, or used
        to build an error, fails."""
        self.assertEqual(guarded_problems(self.sources), [])
        codes = {e.code for e in self.emitted}
        self.assertTrue(codes.isdisjoint({"LG3:effect.flow_refused", "LG3:capability.flow_refused",
                                          "KN7:budget.display_plan_limit", "KN7:resource_limit.recursion_exhausted"}))
        self.assertIn("EV3:resource_limit.display_plan_ceiling", codes)
        self.assertEqual(sorted(path for path, _ in reservation_lines(self.sources)),
                         [path for path, _, _ in RESERVED_NAMES])
        self.assertEqual(reservation_problems(self.sources), [])
        uses = [path for path, lexed in self.sources.items() if "PLAN_LIMIT" in lexed.bare]
        self.assertGreaterEqual(len(uses), 2, "the scan reads the constant's definition and a comparison with it")
        for slug in GUARDED_SLUGS:
            text = f'// {slug} in a comment\nconst X: &str = "prefix {slug}";\n'
            self.assertEqual(len(guarded_problems(planted(text))), 1, slug)
        definition = '    pub const PLAN_LIMIT: &str = "KN7:budget.display_plan_limit";\n'
        self.assertEqual(guarded_problems(planted(definition, RESERVED_NAMES[0][0])), [])
        self.assertEqual(len(guarded_problems(planted(definition))), 1)
        built = planted(definition + "fn f() -> Error { Error::new(codes::PLAN_LIMIT, \"x\") }\n"
                        "fn g(e: &Error) -> bool { e.code == codes::PLAN_LIMIT || codes::PLAN_LIMIT != e.code }\n",
                        RESERVED_NAMES[0][0])
        self.assertEqual([p.split(": ", 1)[0] for p in reservation_problems(built)],
                         [f"{RESERVED_NAMES[0][0]}:2"])

    def test_ct_bnd_24_flow_refusal_codes_are_not_emitted(self):
        """CT-BND-24 (whole, for the workspace's sources): LG3:effect.flow_refused is retired (its registry row says so)
        and no member emits it, and LG3:capability.flow_refused, whose rule LQ2 has not set, is emitted by no member;
        no literal even holds the slug `flow_refused`, so neither code can be assembled. The capability host that
        would raise it (c600-check, c600-run) is among the scanned members. Controls: the slug planted in a format
        string and a full code planted in a literal fail."""
        retired = self.registry.row("LG3:effect.flow_refused")
        self.assertEqual(retired.status, "retired")
        self.assertIn("LQ2", self.registry.row("LG3:capability.flow_refused").meaning)
        flow = [e for e in self.emitted if e.code.endswith(".flow_refused")]
        self.assertEqual(flow, [])
        self.assertEqual([p for p in guarded_problems(self.sources) if "flow_refused" in p], [])
        self.assertTrue(any(path.startswith("rust/c600-check/src/") for path in self.sources))
        self.assertTrue(any(path.startswith("rust/c600-run/src/") for path in self.sources))
        bad = planted('fn f(c: &str) -> String { format!("LG3:{c}.flow_refused") }\n'
                      'const X: &str = "LG3:capability.flow_refused";\n')
        self.assertEqual(len(guarded_problems(bad)), 2)
        self.assertEqual(len(emitted_codes(bad, self.registry)), 1)

    def test_ct_err_07_08_16_emitted_category_rules(self):
        """CT-ERR-07, CT-ERR-08 and CT-ERR-16 (part: the emitted halves; not run: the session-freeze, budget-stop and
        rotation-request fixtures, which need the session store (M6), the command layer and renderer (LP3) and the
        rotation-request parser of the jumbling engine half (M5)): every emitted budget code is an active KN7 budget
        row on a declared allowance, every emitted consistency code comes from KN1 to KN7 (the session store and
        LP3's crates emit KN2 and KN5 ones), and no member emits a KN3 code. Controls: a budget code outside KN7, a
        retired budget code, a non-kernel consistency code and a KN3 code fail."""
        self.assertEqual(category_problems(self.sources, self.registry), [])
        consistency = {e.code for e in self.emitted if reg.split_code(e.code)[1] == "consistency"}
        self.assertTrue(consistency, "the scan reads emitted consistency codes")
        budgets = sorted({e.code for e in self.emitted if reg.split_code(e.code)[1] == "budget"})
        self.assertEqual(budgets, ["KN7:budget.kernel_cost", "KN7:budget.memory", "KN7:budget.output",
                                   "KN7:budget.steps"])
        self.assertEqual([e for e in self.emitted if e.code.startswith("KN3:")], [])
        bad = planted('const A: &str = "EV3:budget.display_plan";\nconst B: &str = "KN7:budget.over_ceiling";\n'
                      'const C: &str = "EV2:consistency.replay";\nconst D: &str = "KN3:input.not_exact";\n')
        self.assertEqual(len(category_problems(bad, self.registry)), 4)

    def test_ct_err_06_handling_never_reads_message_text(self):
        """CT-ERR-06 (part: the handling half in the members' sources; not run: changing or translating the summary
        and diagnostic of a raised envelope, which has those fields only from LP3's command layer, and the test-file
        half, where c600-kernel's tests still compare refusal texts, reported as a finding): no member's source
        compares, searches or matches an error's message, summary or diagnostic text; handling decides by code or
        by type. Controls: planted comparisons, searches and matches on message text fail; a field read for display
        does not."""
        self.assertEqual(text_handling_problems(self.sources), [])
        bad = planted("\n".join([
            'fn a(r: &Refusal) -> bool { r.message == "Recipe must contain 1..256 steps" }',
            'fn b(r: &Refusal) -> bool { r.message.contains("protected") }',
            'fn c(d: &Diagnostic) -> bool { "x" != d.message }',
            'fn d(r: &Refusal) -> u8 { match r.message.as_str() { "x" => 1, _ => 0 } }',
            'fn e(r: &Refusal) -> bool { r.to_string().starts_with("KN2") }',
            'fn ok(r: &Refusal) { eprintln!("{}", r.message); let code = r.code; }']))
        found = text_handling_problems(bad)
        self.assertEqual(sorted({p.split(": ", 1)[0] for p in found}),
                         [f"rust/planted/src/lib.rs:{n}" for n in range(1, 6)])

    def test_ct_err_13_no_message_selects_a_move(self):
        """CT-ERR-13 (part: the emitted half; not run: the typed context and message data of raised envelopes, which
        exist from LP3's command layer, and the Jumble-off refusals of the jumbling engine half (M5)): no string
        literal in a member's source tells the solver which twist, letter or correction to make (ERR.13, DT-J1 A5).
        Control: a planted message that selects a twist fails; a message that only names letters does not."""
        self.assertEqual(move_selection_problems(self.sources), [])
        self.assertTrue(any("letter" in literal for lexed in self.sources.values() for _, literal in lexed.literals))
        bad = planted('const A: &str = "apply the twist A3 to restore the cap";\n'
                      'const B: &str = "a menu letter is grip:rotation";\n')
        self.assertEqual(len(move_selection_problems(bad)), 1)

    def test_ct_kn6_02_no_checker_imports_producer_code(self):
        """CT-KN6-02 (part: not run: KN6.1's five checker kinds, which arrive with LP5 (CT-KN6-01, CT-KN6-03), so the
        Rust half has no checker to scan): no workspace source and no Python tool defines KN6.1's checker interface
        unless it is declared with its producers (no checker in LP2, and one reader of a report, the evidence
        library's admission (EVD.46), which imports neither the interpreter nor the session store); LP2's one checker
        of a producer's output, the O01 to O07 oracle comparison (tools/oracle/compare.py), imports neither the case
        maker nor the trace replayer nor the 0.4 engine they build references with, by import statement, relative
        import or dynamic loading. Controls: a
        planted Rust checker crate that depends on a producer crate, names its search module and includes its file
        fails three ways while one that uses only an allowed engine passes; planted Python checkers importing a
        producer statically, relatively and dynamically fail; an undeclared checker interface fails the inventory."""
        self.assertEqual(checker_inventory(self.sources), set(KN6_CHECKERS) | set(KN6_REPORT_READERS))
        for path, producers in KN6_REPORT_READERS.items():
            self.assertEqual(rust_import_problems(ROOT.joinpath(*Path(path).parts[:2]), producers), [])
        self.assertEqual(python_import_problems(ORACLE_CHECKERS), [])
        names = python_imports(ROOT / "tools/oracle/compare.py")
        self.assertTrue({"json", "zlib"} <= names, names)
        make_cases = python_imports(ROOT / "tools/oracle/make_cases.py")
        self.assertTrue(imports_module(make_cases, "core") and imports_module(make_cases, "session"))
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            write(root / "rust/producer/Cargo.toml", '[package]\nname = "producer"\n')
            write(root / "rust/producer/src/search.rs", "pub fn best() {}\n")
            write(root / "rust/checker/Cargo.toml",
                  '[package]\nname = "checker"\n\n[dependencies]\nproducer = { path = "../producer" }\n'
                  'kernel = { path = "../kernel" }\n')
            write(root / "rust/checker/src/lib.rs",
                  "use kernel::engine::exact;\nuse producer::search::best;\n"
                  '#[path = "../../producer/src/search.rs"]\nmod copied;\n'
                  "pub struct CheckReport;\n")
            write(root / "rust/clean/Cargo.toml", '[package]\nname = "clean"\n\n[dependencies]\nkernel = "1"\n')
            write(root / "rust/clean/src/lib.rs", "use kernel::engine::exact;\n// producer::search is not used\n")
            found = rust_import_problems(root / "rust/checker", ("producer", "producer::search"))
            self.assertEqual(len(found), 3, found)
            self.assertEqual(rust_import_problems(root / "rust/clean", ("producer", "producer::search")), [])
            sources = {"rust/checker/src/lib.rs": lex_rust((root / "rust/checker/src/lib.rs").read_text())}
            self.assertEqual(checker_inventory(sources, root / "tools"), {"rust/checker/src/lib.rs"})

            write(root / "tools/oracle/make_cases.py", "")
            write(root / "tools/oracle/static_check.py", "from tools.oracle.make_cases import CASES\n")
            write(root / "tools/oracle/relative_check.py", "from . import make_cases\n")
            write(root / "tools/oracle/dynamic_check.py",
                  'import importlib\nm = importlib.import_module("tools.oracle.make_cases")\n')
            write(root / "tools/oracle/path_check.py",
                  'import runpy\nrunpy.run_path("C:/elsewhere/tools/oracle/make_cases.py")\n')
            write(root / "tools/oracle/sibling_check.py",
                  "import sys\nsys.path.insert(0, 'tools/oracle')\nimport make_cases\n")
            write(root / "tools/oracle/clean_check.py",
                  'import json\nimport numpy.core\nKEYS = ("make_cases_sha256", "core")\n')
            write(root / "tools/oracle/declared.py", "class CheckReport:\n    pass\n")
            checkers = {f"tools/oracle/{name}.py": ("tools.oracle.make_cases", "core")
                        for name in ("static_check", "relative_check", "dynamic_check", "path_check", "sibling_check",
                                     "clean_check")}
            found = python_import_problems(checkers, root)
            self.assertEqual(sorted(p.split(" ", 1)[0] for p in found),
                             [f"tools/oracle/{name}.py" for name in ("dynamic_check", "path_check", "relative_check",
                                                                     "sibling_check", "static_check")])
            self.assertEqual(checker_inventory({}, root / "tools"), {"tools/oracle/declared.py"})

    def test_ct_rpi_20_windows_apis_only_behind_platform_interfaces(self):
        """CT-RPI-20 (part: the dependency and Windows-API half; not run: one portable HLSL source per stage, since no
        product shader stage exists before LP3's display and renderer, whose sources this test refuses until that
        half is written): Cargo.lock has no Windows package, no member or vendored crate depends on one or has a
        Windows target table, and no member or vendored source uses std::os::windows, a Windows API crate, a Windows
        ABI, a #[link] attribute, a Windows cfg, windows_subsystem or a native link instruction outside the platform
        layer (PLATFORM_LAYER, a workspace member), so every such finding fails (RPI.51 to RPI.53). Controls: the
        same scan without the platform layer finds the layer's own Windows code and nothing else; a planted
        workspace with each kind of finding fails once per kind, and a slice's `.windows(2)` is no finding."""
        self.assertEqual(windows_problems(), [])
        members = workspace_members()
        self.assertLessEqual(PLATFORM_LAYER, set(members), members)
        unguarded = windows_problems(RUST, frozenset())
        self.assertTrue(unguarded, "the platform layer holds no Windows code the scan sees")
        self.assertEqual([p for p in unguarded if p.split("/", 1)[0] not in PLATFORM_LAYER], [], unguarded)
        core = {"c600-ir", "c600-check", "c600-digest", "c600-syntax", "c600-run", "c600-kernel"}
        self.assertLessEqual(core, set(members), members)
        self.assertTrue(all((RUST / member / "Cargo.toml").is_file() for member in members), members)
        self.assertGreaterEqual(sum(len(lexed.code) for lexed in self.sources.values()), 100_000)
        self.assertEqual(shader_sources(), [], "write CT-RPI-20's HLSL half before product shader sources land")
        with tempfile.TemporaryDirectory() as tmp:
            rust = Path(tmp) / "rust"
            write(rust / "Cargo.toml", 'members = ["app", "plat"]\n')
            write(rust / "Cargo.lock", '[[package]]\nname = "app"\n\n[[package]]\nname = "windows-sys"\n')
            write(rust / "app/Cargo.toml", '[package]\nname = "app"\n\n[dependencies]\nwinapi = "0.3"\n\n'
                                           "[target.'cfg(windows)'.dependencies]\nlibc = \"1\"\n")
            write(rust / "app/build.rs", 'fn main() { println!("cargo:rustc-link-lib=user32"); }\n')
            write(rust / "app/src/lib.rs", "\n".join([
                "use std::os::windows::ffi::OsStrExt;", "use windows_sys::Win32::Foundation::HWND;",
                'extern "system" { fn GetTickCount() -> u32; }', '#[link(name = "d3d12")]', "extern {}",
                '#[cfg(target_os = "windows")]', "fn win() {}", "#![windows_subsystem = \"windows\"]",
                "fn slice(v: &[u8]) -> bool { v.windows(2).all(|w| w[0] <= w[1]) }",
                "// windows::Win32 in a comment is no finding", ""]))
            write(rust / "app/src/shader.hlsl", "float4 main() : SV_Target { return 1; }\n")
            write(rust / "plat/Cargo.toml", '[package]\nname = "plat"\n\n[dependencies]\nwindows = "0.58"\n')
            write(rust / "plat/src/lib.rs", "use windows::Win32::Graphics::Direct3D12::ID3D12Device;\n")
            found = windows_problems(rust, frozenset())
            kinds = ("Cargo.lock: package windows-sys", "app/Cargo.toml: depends on winapi",
                     "app/Cargo.toml: a Windows target table", "app/build.rs:1: a native link instruction",
                     "app/src/lib.rs:1: std::os::windows", "app/src/lib.rs:2: a Windows API crate",
                     "app/src/lib.rs:3: a Windows ABI", "app/src/lib.rs:4: a #[link] attribute",
                     "app/src/lib.rs:6: a Windows cfg", "app/src/lib.rs:8: windows_subsystem",
                     "plat/Cargo.toml: depends on windows", "plat/src/lib.rs:1: a Windows API crate")
            for kind in kinds:
                self.assertTrue(any(p.startswith(kind) for p in found), (kind, found))
            self.assertEqual(len(found), len(kinds), found)
            behind = windows_problems(rust, frozenset({"plat"}))
            self.assertFalse(any(p.startswith("plat/") for p in behind), behind)
            self.assertEqual(shader_sources(rust), ["rust/app/src/shader.hlsl"])


if __name__ == "__main__":
    unittest.main()

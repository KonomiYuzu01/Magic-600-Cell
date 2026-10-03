"""Make probe results publishable: placeholders for paths, no user, host or account identifiers.

`sanitize` rewrites every string in a JSON-safe value; `leaks` lists what would still be
private in a text, and run_probes.py refuses to write a public result while it finds any.
"""
from __future__ import annotations

import os
import re
from pathlib import Path

FILE_URI = re.compile(r'(?i)\bfile:/[^\s"\'<>|]*')
DRIVE_PATH = re.compile(r'(?i)(?<![A-Za-z])[a-z]:[\\/][^\s"\'<>|]*')
# \\server\share, \\?\ and \\.\ prefixes, and //server/share where a path starts (not the // of a URL).
UNC_PATH = re.compile(r'(?m)(?:\\\\|(?:^|(?<=[\s"\'(=,\[]))//(?=[^\s/]))[^\s"\'<>|]+')
SID = re.compile(r'(?i)\bS-1-\d+(?:-\d+)+')


def _private_words() -> list:
    """User, host and domain names of any length (review finding MPB-02)."""
    words = [os.environ.get(name, '') for name in ('USERNAME', 'COMPUTERNAME', 'USERDOMAIN')]
    words.append(Path.home().name)
    return sorted({word for word in words if word}, key=len, reverse=True)


def _word(word: str):
    """A name of three or more characters matches anywhere; a shorter one only as a whole word."""
    pattern = re.escape(word)
    if len(word) < 3:
        pattern = r'(?<![A-Za-z0-9])' + pattern + r'(?![A-Za-z0-9])'
    return re.compile(pattern, re.IGNORECASE)


def _replacements(roots: dict) -> list:
    pairs = []
    for placeholder, path in roots.items():
        resolved = str(Path(path).resolve())
        for form in {resolved, resolved.replace('\\', '/'), Path(resolved).as_uri()}:
            pairs.append((form, placeholder))
    return sorted(pairs, key=lambda pair: len(pair[0]), reverse=True)


def sanitize(value, roots: dict):
    """`roots` maps placeholders such as '<run>' or '<repo>' to directories."""
    pairs = _replacements(roots)
    words = _private_words()

    def clean(text: str) -> str:
        for form, placeholder in pairs:
            text = re.sub(re.escape(form), lambda match, value=placeholder: value, text, flags=re.IGNORECASE)
        text = FILE_URI.sub('<path>', text)
        text = DRIVE_PATH.sub('<path>', text)
        text = UNC_PATH.sub('<path>', text)
        text = SID.sub('<sid>', text)
        for word in words:
            text = _word(word).sub('<private>', text)
        return text

    def walk(item):
        if isinstance(item, str):
            return clean(item)
        if isinstance(item, dict):
            return {clean(str(key)): walk(entry) for key, entry in item.items()}
        if isinstance(item, (list, tuple)):
            return [walk(entry) for entry in item]
        return item

    return walk(value)


def leaks(text: str) -> list:
    found = []
    if FILE_URI.search(text) or DRIVE_PATH.search(text) or UNC_PATH.search(text):
        found.append('absolute-path')
    if SID.search(text):
        found.append('sid')
    found.extend('private-word' for word in _private_words() if _word(word).search(text))
    return sorted(set(found))


def leaks_in(value) -> list:
    """`leaks` for every decoded key and string of a JSON-safe value. Serialized JSON escapes newlines and
    non-ASCII characters, which can hide a path start or a name boundary (review finding MPB-03)."""
    found = set()

    def walk(item):
        if isinstance(item, str):
            found.update(leaks(item))
        elif isinstance(item, dict):
            for key, entry in item.items():
                walk(str(key))
                walk(entry)
        elif isinstance(item, (list, tuple)):
            for entry in item:
                walk(entry)

    walk(value)
    return sorted(found)

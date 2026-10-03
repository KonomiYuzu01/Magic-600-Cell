"""Make probe results publishable: placeholders for paths, no user, host or account identifiers.

`sanitize` rewrites every string in a JSON-safe value; `leaks` lists what would still be
private in a text, and run_probes.py refuses to write a public result while it finds any.
"""
from __future__ import annotations

import os
import re
from pathlib import Path

DRIVE_PATH = re.compile(r'(?i)(?<![A-Za-z])[a-z]:[\\/][^\s"\'<>|]*')
UNC_PATH = re.compile(r'\\\\[^\s"\'<>|]+')
SID = re.compile(r'S-1-5-21(?:-\d+){3,}')


def _private_words() -> list:
    words = []
    for name in ('USERNAME', 'COMPUTERNAME', 'USERDOMAIN'):
        value = os.environ.get(name, '')
        if len(value) >= 3:
            words.append(value)
    home = Path.home().name
    if len(home) >= 3:
        words.append(home)
    return sorted(set(words), key=len, reverse=True)


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
        text = DRIVE_PATH.sub('<path>', text)
        text = UNC_PATH.sub('<path>', text)
        text = SID.sub('<sid>', text)
        for word in words:
            text = re.sub(re.escape(word), '<private>', text, flags=re.IGNORECASE)
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
    if DRIVE_PATH.search(text) or UNC_PATH.search(text):
        found.append('absolute-path')
    if SID.search(text):
        found.append('sid')
    found.extend('private-word' for word in _private_words() if word.lower() in text.lower())
    return sorted(set(found))

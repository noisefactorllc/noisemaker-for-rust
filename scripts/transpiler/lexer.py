"""GLSL ES 3.00 tokenizer (post-preprocess).

Consumes already-normalized/preprocessed GLSL (no #directives). Produces a flat
token list for the recursive-descent parser.
"""

from __future__ import annotations

import re

# Multi-char operators, longest first so the scanner is greedy.
_OPS = [
    "<<=",
    ">>=",
    "++",
    "--",
    "<<",
    ">>",
    "<=",
    ">=",
    "==",
    "!=",
    "&&",
    "||",
    "+=",
    "-=",
    "*=",
    "/=",
    "%=",
    "&=",
    "|=",
    "^=",
    "+",
    "-",
    "*",
    "/",
    "%",
    "<",
    ">",
    "=",
    "!",
    "~",
    "&",
    "|",
    "^",
    "?",
    ":",
    ".",
    ",",
    ";",
    "(",
    ")",
    "[",
    "]",
    "{",
    "}",
]

_NUM = re.compile(
    r"""
    (?:
        0[xX][0-9a-fA-F]+            # hex int
      | (?:\d+\.\d*|\.\d+|\d+)       # decimal, with optional fraction
        (?:[eE][+-]?\d+)?            # optional exponent
    )
    [uUfF]?                          # optional type suffix
    """,
    re.VERBOSE,
)
_IDENT = re.compile(r"[A-Za-z_][A-Za-z0-9_]*")
_WS = re.compile(r"\s+")
_LINE_COMMENT = re.compile(r"//[^\n]*")
_BLOCK_COMMENT = re.compile(r"/\*.*?\*/", re.DOTALL)


class Token:
    __slots__ = ("kind", "pos", "value")

    def __init__(self, kind, value, pos):
        self.kind = kind  # "num" | "id" | "op"
        self.value = value
        self.pos = pos

    def __repr__(self):
        return f"Token({self.kind},{self.value!r})"


def tokenize(source: str) -> list:
    tokens = []
    i = 0
    n = len(source)
    while i < n:
        c = source[i]
        if c in " \t\r\n":
            m = _WS.match(source, i)
            i = m.end()
            continue
        if source.startswith("//", i):
            m = _LINE_COMMENT.match(source, i)
            i = m.end()
            continue
        if source.startswith("/*", i):
            m = _BLOCK_COMMENT.match(source, i)
            if not m:
                raise SyntaxError(f"unterminated block comment at {i}")
            i = m.end()
            continue
        if c.isdigit() or (c == "." and i + 1 < n and source[i + 1].isdigit()):
            m = _NUM.match(source, i)
            tokens.append(Token("num", m.group(), i))
            i = m.end()
            continue
        if c.isalpha() or c == "_":
            m = _IDENT.match(source, i)
            tokens.append(Token("id", m.group(), i))
            i = m.end()
            continue
        for op in _OPS:
            if source.startswith(op, i):
                tokens.append(Token("op", op, i))
                i += len(op)
                break
        else:
            raise SyntaxError(f"unexpected character {c!r} at {i}")
    tokens.append(Token("op", "<eof>", n))
    return tokens

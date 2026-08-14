"""GLSL ES 3.00 recursive-descent parser -> clean AST.

AST nodes are dicts with a "k" (kind) field. Consumes tokens from lexer.tokenize
on already-preprocessed GLSL (no #directives, no structs beyond flat ones).
"""

from __future__ import annotations

from typing import ClassVar

from .lexer import tokenize

SCALAR = {"void", "bool", "int", "uint", "float"}
VEC = {f"{p}vec{n}" for p in ("", "i", "u", "b") for n in (2, 3, 4)}
MAT = {f"mat{n}" for n in (2, 3, 4)} | {f"mat{a}x{b}" for a in (2, 3, 4) for b in (2, 3, 4)}
SAMPLER = {"sampler2D", "sampler3D", "samplerCube", "sampler2DArray"}
TYPES = SCALAR | VEC | MAT | SAMPLER

QUALIFIERS = {
    "const",
    "uniform",
    "in",
    "out",
    "inout",
    "flat",
    "smooth",
    "noperspective",
    "centroid",
    "invariant",
    "highp",
    "mediump",
    "lowp",
    "precise",
}
CONTROL = {"if", "else", "for", "while", "do", "return", "break", "continue", "discard", "struct"}

_ASSIGN = {"=", "+=", "-=", "*=", "/=", "%=", "&=", "|=", "^=", "<<=", ">>="}


class Parser:
    def __init__(self, tokens):
        self.toks = tokens
        self.i = 0
        self.struct_types = set()

    # ---- cursor ----
    def peek(self, k=0):
        return self.toks[self.i + k]

    def at(self, value):
        t = self.toks[self.i]
        return t.value == value

    def at_type(self):
        t = self.toks[self.i]
        return t.kind == "id" and (t.value in TYPES or t.value in self.struct_types)

    def next(self):
        t = self.toks[self.i]
        self.i += 1
        return t

    def expect(self, value):
        t = self.toks[self.i]
        if t.value != value:
            raise SyntaxError(f"expected {value!r} got {t.value!r} at token {self.i}")
        self.i += 1
        return t

    def eat(self, value):
        if self.toks[self.i].value == value:
            self.i += 1
            return True
        return False

    # ---- top level ----
    def parse_program(self):
        decls = []
        while not self.at("<eof>"):
            d = self.external_decl()
            if d is not None:
                decls.append(d)
        return {"k": "program", "decls": decls}

    def external_decl(self):
        if self.at("precision"):
            while not self.eat(";"):
                self.next()
            return None
        if self.at("struct"):
            return self.struct_decl()
        # skip layout(...) qualifier prefixes
        quals = self.qualifiers()
        # interface (uniform) block: `uniform Name { members } [inst];`
        if "uniform" in quals and self.peek().kind == "id" and self.peek(1).value == "{":
            return self.uniform_block()
        typ = self.type_spec()
        # function or variable?
        name = self.next().value
        if self.at("("):
            return self.function_rest(typ, name, quals)
        return self.var_decl_rest(typ, name, quals, top=True)

    def uniform_block(self):
        self.next()  # block type name (irrelevant without an instance)
        self.expect("{")
        members = []
        while not self.at("}"):
            self.qualifiers()
            mtype = self.type_spec()
            mname = self.next().value
            arr = None
            if self.eat("["):
                arr = self.expr()
                self.expect("]")
            members.append({"type": mtype, "name": mname, "array": arr})
            self.expect(";")
        self.expect("}")
        inst = None
        if self.peek().kind == "id":
            inst = self.next().value
        self.expect(";")
        return {"k": "ubo", "members": members, "inst": inst}

    def qualifiers(self):
        q = []
        while True:
            t = self.peek()
            if t.value == "layout":
                self.next()
                self.expect("(")
                depth = 1
                while depth:
                    v = self.next().value
                    if v == "(":
                        depth += 1
                    elif v == ")":
                        depth -= 1
                continue
            if t.kind == "id" and t.value in QUALIFIERS:
                q.append(self.next().value)
                continue
            break
        return q

    def type_spec(self):
        t = self.next()
        return t.value

    def struct_decl(self):
        self.expect("struct")
        name = self.next().value
        self.struct_types.add(name)
        self.expect("{")
        fields = []
        while not self.at("}"):
            self.qualifiers()
            ftype = self.type_spec()
            fname = self.next().value
            arr = None
            if self.eat("["):
                arr = self.expr()
                self.expect("]")
            fields.append((ftype, fname, arr))
            self.expect(";")
        self.expect("}")
        # optional instance name: `struct S {...} inst;`
        inst = None
        if self.peek().kind == "id":
            inst = self.next().value
        self.expect(";")
        return {"k": "struct", "name": name, "fields": fields, "inst": inst}

    def function_rest(self, ret, name, quals):
        self.expect("(")
        params = []
        if not self.at(")"):
            while True:
                pquals = self.qualifiers()
                if self.at("void") and self.peek(1).value == ")":
                    self.next()
                    break
                ptype = self.type_spec()
                pname = self.next().value if self.peek().kind == "id" else None
                array = None
                if self.eat("["):
                    array = True if self.at("]") else self.expr()
                    self.expect("]")
                params.append((ptype, pname, pquals, array))
                if not self.eat(","):
                    break
        self.expect(")")
        if self.eat(";"):  # prototype
            return {"k": "proto", "ret": ret, "name": name, "params": params}
        body = self.block()
        return {"k": "func", "ret": ret, "name": name, "params": params, "body": body}

    def var_decl_rest(self, typ, name, quals, top=False):
        declarators = []
        while True:
            arr = None
            if self.eat("["):
                arr = True if self.at("]") else self.expr()
                self.expect("]")
            init = None
            if self.eat("="):
                init = self.assign_expr()
            declarators.append({"name": name, "array": arr, "init": init})
            if not self.eat(","):
                break
            name = self.next().value
        self.expect(";")
        return {"k": "decl", "type": typ, "quals": quals, "declarators": declarators, "top": top}

    # ---- statements ----
    def block(self):
        self.expect("{")
        stmts = []
        while not self.at("}"):
            stmts.append(self.statement())
        self.expect("}")
        return stmts

    def statement(self):
        t = self.peek()
        if t.value == "{":
            return {"k": "block", "body": self.block()}
        if t.value == "if":
            return self.if_stmt()
        if t.value == "for":
            return self.for_stmt()
        if t.value == "while":
            self.next()
            self.expect("(")
            cond = self.expr()
            self.expect(")")
            return {"k": "while", "cond": cond, "body": self.statement()}
        if t.value == "do":
            self.next()
            body = self.statement()
            self.expect("while")
            self.expect("(")
            cond = self.expr()
            self.expect(")")
            self.expect(";")
            return {"k": "dowhile", "cond": cond, "body": body}
        if t.value == "return":
            self.next()
            val = None if self.at(";") else self.expr()
            self.expect(";")
            return {"k": "return", "value": val}
        if t.value == "break":
            self.next()
            self.expect(";")
            return {"k": "break"}
        if t.value == "continue":
            self.next()
            self.expect(";")
            return {"k": "continue"}
        if t.value == "discard":
            self.next()
            self.expect(";")
            return {"k": "discard"}
        # declaration vs expression: [const] TYPE ident ...
        if self.at_decl_start():
            quals = self.qualifiers()
            typ = self.type_spec()
            name = self.next().value
            return self.var_decl_rest(typ, name, quals)
        e = self.expr()
        self.expect(";")
        return {"k": "expr", "expr": e}

    def at_decl_start(self):
        t = self.peek()
        if t.kind != "id":
            return False
        if t.value in QUALIFIERS:
            return True
        if t.value in TYPES or t.value in self.struct_types:
            # a type keyword followed by an ident (decl) or by `(` (constructor expr)
            return self.peek(1).kind == "id"
        return False

    def if_stmt(self):
        self.next()
        self.expect("(")
        cond = self.expr()
        self.expect(")")
        then = self.statement()
        els = None
        if self.eat("else"):
            els = self.statement()
        return {"k": "if", "cond": cond, "then": then, "els": els}

    def for_stmt(self):
        self.next()
        self.expect("(")
        if self.eat(";"):
            init = None
        elif self.at_decl_start():
            quals = self.qualifiers()
            typ = self.type_spec()
            name = self.next().value
            init = self.var_decl_rest(typ, name, quals)
        else:
            init = {"k": "expr", "expr": self.expr()}
            self.expect(";")
        cond = None if self.at(";") else self.expr()
        self.expect(";")
        update = None if self.at(")") else self.expr()
        self.expect(")")
        body = self.statement()
        return {"k": "for", "init": init, "cond": cond, "update": update, "body": body}

    # ---- expressions (precedence climbing) ----
    def expr(self):
        e = self.assign_expr()
        while self.at(","):  # comma operator: keep last
            self.next()
            e = self.assign_expr()
        return e

    def assign_expr(self):
        left = self.conditional()
        if self.peek().value in _ASSIGN:
            op = self.next().value
            right = self.assign_expr()
            return {"k": "assign", "op": op, "target": left, "value": right}
        return left

    def conditional(self):
        c = self.binary(0)
        if self.eat("?"):
            a = self.expr()
            self.expect(":")
            b = self.assign_expr()
            return {"k": "cond", "c": c, "a": a, "b": b}
        return c

    _BIN: ClassVar = [
        {"||"},
        {"&&"},
        {"|"},
        {"^"},
        {"&"},
        {"==", "!="},
        {"<", ">", "<=", ">="},
        {"<<", ">>"},
        {"+", "-"},
        {"*", "/", "%"},
    ]

    def binary(self, level):
        if level >= len(self._BIN):
            return self.unary()
        left = self.binary(level + 1)
        while self.peek().value in self._BIN[level]:
            op = self.next().value
            right = self.binary(level + 1)
            left = {"k": "binary", "op": op, "l": left, "r": right}
        return left

    def unary(self):
        t = self.peek()
        if t.value in ("+", "-", "!", "~", "++", "--"):
            self.next()
            return {"k": "unary", "op": t.value, "x": self.unary()}
        return self.postfix()

    def postfix(self):
        e = self.primary()
        while True:
            t = self.peek()
            if t.value == ".":
                self.next()
                field = self.next().value
                if self.at("(") and field == "length":  # arr.length() -> array size
                    self.expect("(")
                    self.expect(")")
                    e = {"k": "call", "name": "__array_length", "args": [e]}
                else:
                    e = {"k": "member", "obj": e, "field": field}
            elif t.value == "[":
                self.next()
                idx = self.expr()
                self.expect("]")
                e = {"k": "index", "obj": e, "idx": idx}
            elif t.value == "(":
                e = self.call_rest(e)
            elif t.value in ("++", "--"):
                self.next()
                e = {"k": "post", "op": t.value, "x": e}
            else:
                break
        return e

    def call_rest(self, callee):
        self.expect("(")
        args = []
        if not self.at(")"):
            while True:
                args.append(self.assign_expr())
                if not self.eat(","):
                    break
        self.expect(")")
        name = callee["name"] if callee.get("k") == "id" else callee.get("type")
        return {"k": "call", "name": name, "args": args}

    def primary(self):
        t = self.peek()
        if t.value == "(":
            self.next()
            e = self.expr()
            self.expect(")")
            return e
        if t.kind == "num":
            self.next()
            return {"k": "num", "value": t.value}
        if t.value in ("true", "false"):
            self.next()
            return {"k": "bool", "value": t.value == "true"}
        if t.kind == "id":
            # constructor: TYPE(...) or TYPE[N](...)
            if (t.value in TYPES or t.value in self.struct_types) and self.peek(1).value in ("(", "["):
                self.next()
                arr = None
                if self.eat("["):
                    arr = True if self.at("]") else self.expr()
                    self.expect("]")
                self.expect("(")
                args = []
                if not self.at(")"):
                    while True:
                        args.append(self.assign_expr())
                        if not self.eat(","):
                            break
                self.expect(")")
                return {"k": "construct", "type": t.value, "array": arr, "args": args}
            self.next()
            return {"k": "id", "name": t.value}
        raise SyntaxError(f"unexpected token {t.value!r} at {self.i}")


def parse(source_or_tokens):
    tokens = tokenize(source_or_tokens) if isinstance(source_or_tokens, str) else source_or_tokens
    return Parser(tokens).parse_program()

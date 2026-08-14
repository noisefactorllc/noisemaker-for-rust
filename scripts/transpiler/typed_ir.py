"""Resolve normalized GLSL into a deterministic, executable structural IR.

The output deliberately contains data only.  It is consumed by the Rust VM;
no Python source, closures, or callable objects are embedded in the bundle.
"""

from __future__ import annotations

import copy
import re

from .parser import parse
from .preprocess import normalize


SCALARS = {"void", "bool", "int", "uint", "float"}
VECTORS = {f"{prefix}vec{width}" for prefix in ("", "i", "u", "b") for width in (2, 3, 4)}
MATRICES = {f"mat{width}" for width in (2, 3, 4)} | {
    f"mat{columns}x{rows}" for columns in (2, 3, 4) for rows in (2, 3, 4)
}
SAMPLERS = {"sampler2D", "sampler3D", "samplerCube", "sampler2DArray"}
TYPES = SCALARS | VECTORS | MATRICES | SAMPLERS


def _width(type_name: str) -> int:
    if type_name.endswith("[]"):
        return _width(type_name[:-2])
    match = re.fullmatch(r"[biu]?vec([234])", type_name)
    if match:
        return int(match.group(1))
    match = re.fullmatch(r"mat([234])(?:x([234]))?", type_name)
    if match:
        return int(match.group(1)) * int(match.group(2) or match.group(1))
    return 1


def _base(type_name: str) -> str:
    if type_name.endswith("[]"):
        return _base(type_name[:-2])
    if type_name.startswith("bvec") or type_name == "bool":
        return "bool"
    if type_name.startswith("ivec") or type_name == "int":
        return "int"
    if type_name.startswith("uvec") or type_name == "uint":
        return "uint"
    if type_name.startswith("sampler"):
        return "sampler"
    if type_name.startswith("mat"):
        return "float"
    return "float" if type_name in TYPES else type_name


def _type_for(base: str, width: int) -> str:
    if width == 1:
        return base
    prefix = {"float": "vec", "int": "ivec", "uint": "uvec", "bool": "bvec"}.get(base)
    return f"{prefix}{width}" if prefix else base


def _mangled(name: str, parameter_types: list[str]) -> str:
    return f"{name}__{'_'.join(parameter_types) if parameter_types else 'void'}"


class Scope:
    def __init__(self, parent: "Scope | None" = None):
        self.parent = parent
        self.values: dict[str, tuple[str, str]] = {}

    def child(self) -> "Scope":
        return Scope(self)

    def define(self, name: str, type_name: str, storage: str) -> None:
        self.values[name] = (type_name, storage)

    def resolve(self, name: str) -> tuple[str, str] | None:
        scope = self
        while scope is not None:
            if name in scope.values:
                return scope.values[name]
            scope = scope.parent
        return None


class TypedIrEmitter:
    _BUILTIN_VALUES = {
        "gl_FragCoord": "vec4",
        "gl_PointCoord": "vec2",
        "gl_FragDepth": "float",
        "gl_VertexID": "int",
        "gl_InstanceID": "int",
        "gl_PointSize": "float",
    }

    def __init__(self, program: dict, outputs: list[str], varyings: list[str]):
        self.program = program
        self.outputs = list(outputs)
        self.varyings = list(varyings)
        self.root = Scope()
        self.structs: dict[str, list[tuple[str, str, object]]] = {}
        self.functions: dict[str, list[dict]] = {}
        self.function_nodes: list[tuple[dict, dict]] = []
        self.globals: list[dict] = []
        self.uniforms: list[dict] = []

    def emit(self) -> dict:
        self._collect()
        functions = [self._function(node, signature) for node, signature in self.function_nodes]
        if not any(function["name"] == "main" for function in functions):
            raise SyntaxError("shader has no main()")
        return {
            "outputs": self.outputs,
            "varyings": self.varyings,
            "structs": [
                {
                    "name": name,
                    "fields": [
                        {"name": field_name, "type": field_type, "arraySize": self._array_literal(array)}
                        for field_type, field_name, array in fields
                    ],
                }
                for name, fields in sorted(self.structs.items())
            ],
            "uniforms": self.uniforms,
            "globals": self.globals,
            "functions": functions,
        }

    def _collect(self) -> None:
        for name, type_name in self._BUILTIN_VALUES.items():
            self.root.define(name, type_name, "builtin")
        for name in self.varyings:
            self.root.define(name, "vec2", "varying")
        for declaration in self.program["decls"]:
            if declaration["k"] == "struct":
                self.structs[declaration["name"]] = declaration["fields"]
        for declaration in self.program["decls"]:
            kind = declaration["k"]
            if kind == "func":
                parameter_types = [
                    self._declared_type(parameter[0], parameter[3] if len(parameter) > 3 else None)
                    for parameter in declaration["params"]
                ]
                signature = {
                    "name": declaration["name"],
                    "mangledName": _mangled(declaration["name"], parameter_types),
                    "returnType": self._declared_type(declaration["ret"], None),
                    "parameterTypes": parameter_types,
                    "node": declaration,
                }
                self.functions.setdefault(declaration["name"], []).append(signature)
                self.function_nodes.append((declaration, signature))
            elif kind == "decl":
                self._collect_global(declaration)
            elif kind == "ubo":
                for member in declaration["members"]:
                    type_name = self._declared_type(member["type"], member.get("array"))
                    self.root.define(member["name"], type_name, "uniform")
                    self.uniforms.append({"name": member["name"], "type": type_name})
            elif kind in {"struct", "proto"}:
                continue
            else:
                raise SyntaxError(f"typed IR: unsupported top-level declaration {kind!r}")
        self.uniforms.sort(key=lambda item: item["name"])

    def _collect_global(self, declaration: dict) -> None:
        storage = "uniform" if "uniform" in declaration.get("quals", []) else "global"
        for declarator in declaration["declarators"]:
            type_name = self._declared_type(declaration["type"], declarator.get("array"))
            initializer = (
                self._expression(declarator["init"], self.root) if declarator.get("init") is not None else None
            )
            self.root.define(declarator["name"], type_name, storage)
            value = {"name": declarator["name"], "type": type_name}
            if storage == "uniform":
                self.uniforms.append(value)
            else:
                value["initializer"] = initializer
                value["arraySize"] = self._array_expression(declarator.get("array"), self.root)
                self.globals.append(value)

    def _function(self, node: dict, signature: dict) -> dict:
        scope = self.root.child()
        parameters = []
        for index, (parameter, type_name) in enumerate(
            zip(node["params"], signature["parameterTypes"], strict=True)
        ):
            _, name, qualifiers, *_ = parameter
            parameter_name = name or f"_unnamed{index}"
            qualifier = next((q for q in qualifiers if q in {"in", "out", "inout"}), "in")
            scope.define(parameter_name, type_name, "parameter")
            parameters.append({"name": parameter_name, "type": type_name, "qualifier": qualifier})
        return {
            "name": node["name"],
            "mangledName": signature["mangledName"],
            "returnType": signature["returnType"],
            "parameters": parameters,
            "body": self._block(node["body"], scope),
        }

    def _block(self, statements: list[dict], scope: Scope) -> list[dict]:
        return [
            self._statement(statement, scope)
            for statement in statements
            if not self._is_preprocessor_marker(statement)
        ]

    def _statement(self, statement: dict, scope: Scope) -> dict:
        kind = statement["k"]
        if kind == "block":
            return {"kind": "block", "body": self._block(statement["body"], scope.child())}
        if kind == "decl":
            declarations = []
            for declarator in statement["declarators"]:
                type_name = self._declared_type(statement["type"], declarator.get("array"))
                initializer = (
                    self._expression(declarator["init"], scope) if declarator.get("init") is not None else None
                )
                array_size = self._array_expression(declarator.get("array"), scope)
                scope.define(declarator["name"], type_name, "local")
                declarations.append({
                    "name": declarator["name"], "type": type_name,
                    "initializer": initializer, "arraySize": array_size,
                })
            return {"kind": "declaration", "declarations": declarations}
        if kind == "expr":
            return {"kind": "expression", "expression": self._expression(statement["expr"], scope)}
        if kind == "if":
            hoisted = {}
            if self._is_preprocessor_branch(statement["then"]):
                hoisted.update(self._branch_declarations(statement["then"]))
            if self._is_preprocessor_branch(statement.get("els")):
                hoisted.update(self._branch_declarations(statement.get("els")))
            declarations = []
            for name, type_name in sorted(hoisted.items()):
                if scope.resolve(name) is None:
                    scope.define(name, type_name, "local")
                    declarations.append({"name": name, "type": type_name})
            result = {
                "kind": "if",
                "hoistedDeclarations": declarations,
                "condition": self._expression(statement["cond"], scope),
                "then": self._statement(statement["then"], scope.child()),
            }
            result["else"] = (
                self._statement(statement["els"], scope.child()) if statement.get("els") is not None else None
            )
            return result
        if kind == "for":
            loop_scope = scope.child()
            return {
                "kind": "for",
                "initializer": self._statement(statement["init"], loop_scope) if statement.get("init") else None,
                "condition": self._expression(statement["cond"], loop_scope) if statement.get("cond") else None,
                "update": self._expression(statement["update"], loop_scope) if statement.get("update") else None,
                "body": self._statement(statement["body"], loop_scope),
            }
        if kind in {"while", "dowhile"}:
            return {
                "kind": "doWhile" if kind == "dowhile" else "while",
                "condition": self._expression(statement["cond"], scope),
                "body": self._statement(statement["body"], scope.child()),
            }
        if kind == "return":
            return {
                "kind": "return",
                "value": self._expression(statement["value"], scope) if statement.get("value") is not None else None,
            }
        if kind in {"break", "continue", "discard"}:
            return {"kind": kind}
        raise SyntaxError(f"typed IR: unsupported statement {kind!r}")

    @staticmethod
    def _is_preprocessor_marker(statement: dict) -> bool:
        return (
            statement.get("k") == "expr"
            and statement.get("expr", {}).get("k") == "id"
            and statement["expr"].get("name") == "__nm_preprocessor_branch__"
        )

    def _is_preprocessor_branch(self, statement: dict | None) -> bool:
        return bool(
            statement
            and statement.get("k") == "block"
            and statement.get("body")
            and self._is_preprocessor_marker(statement["body"][0])
        )

    def _branch_declarations(self, statement: dict | None) -> dict[str, str]:
        """Collect declarations exposed by preprocessor-lowered branches.

        The sibling normalizer lowers runtime ``#if`` to an ordinary if.  GLSL
        preprocessor branches do not create a scope, so declarations can be
        referenced after ``#endif``.  Hoisting all branch declarations is safe
        for ordinary GLSL too (such a reference would otherwise be invalid).
        """
        if statement is None:
            return {}
        kind = statement.get("k")
        if kind == "block":
            declarations = {}
            for child in statement["body"]:
                if child.get("k") == "decl":
                    for declarator in child["declarators"]:
                        declarations[declarator["name"]] = self._declared_type(
                            child["type"], declarator.get("array")
                        )
            return declarations
        if kind == "decl":
            return {
                declarator["name"]: self._declared_type(statement["type"], declarator.get("array"))
                for declarator in statement["declarators"]
            }
        if kind == "if":
            declarations = self._branch_declarations(statement["then"])
            declarations.update(self._branch_declarations(statement.get("els")))
            return declarations
        return {}

    def _expression(self, node: dict, scope: Scope) -> dict:
        kind = node["k"]
        method = getattr(self, f"_expr_{kind}", None)
        if method is None:
            raise SyntaxError(f"typed IR: unsupported expression {kind!r}")
        return method(node, scope)

    def _expr_num(self, node: dict, _scope: Scope) -> dict:
        raw = node["value"]
        low = raw.lower()
        if low.endswith("u"):
            type_name, value = "uint", int(raw[:-1], 0)
        elif low.startswith("0x"):
            type_name, value = "int", int(raw, 16)
        elif "." in raw or "e" in low or low.endswith("f"):
            type_name, value = "float", float(raw.rstrip("fF"))
        else:
            type_name, value = "int", int(raw)
        return {"kind": "literal", "type": type_name, "value": value, "source": raw}

    def _expr_bool(self, node: dict, _scope: Scope) -> dict:
        return {"kind": "literal", "type": "bool", "value": node["value"]}

    def _expr_id(self, node: dict, scope: Scope) -> dict:
        resolved = scope.resolve(node["name"])
        if resolved is None and node["name"] in {"v_texCoord", "vTexCoord", "texCoord"}:
            resolved = ("vec2", "varying")
        if resolved is None:
            raise SyntaxError(f"typed IR: unresolved identifier {node['name']!r}")
        type_name, storage = resolved
        return {"kind": "identifier", "type": type_name, "name": node["name"], "storage": storage}

    def _expr_member(self, node: dict, scope: Scope) -> dict:
        obj = self._expression(node["obj"], scope)
        field = node["field"]
        if obj["type"] in self.structs:
            match = next((item for item in self.structs[obj["type"]] if item[1] == field), None)
            if match is None:
                raise SyntaxError(f"typed IR: unknown field {field!r} on {obj['type']}")
            type_name = self._declared_type(match[0], match[2])
        else:
            if not set(field) <= set("xyzwrgbastpq"):
                raise SyntaxError(f"typed IR: invalid swizzle {field!r}")
            type_name = _type_for(_base(obj["type"]), len(field))
        return {"kind": "member", "type": type_name, "object": obj, "field": field}

    def _expr_index(self, node: dict, scope: Scope) -> dict:
        obj = self._expression(node["obj"], scope)
        index = self._expression(node["idx"], scope)
        type_name = obj["type"][:-2] if obj["type"].endswith("[]") else _type_for(_base(obj["type"]), 1)
        if obj["type"].startswith("mat"):
            match = re.fullmatch(r"mat([234])(?:x([234]))?", obj["type"])
            type_name = f"vec{match.group(2) or match.group(1)}"
        return {"kind": "index", "type": type_name, "object": obj, "index": index}

    def _expr_unary(self, node: dict, scope: Scope) -> dict:
        operand = self._expression(node["x"], scope)
        type_name = "bool" if node["op"] == "!" else operand["type"]
        return {"kind": "unary", "type": type_name, "operator": node["op"], "operand": operand}

    def _expr_post(self, node: dict, scope: Scope) -> dict:
        operand = self._expression(node["x"], scope)
        return {"kind": "postfix", "type": operand["type"], "operator": node["op"], "lvalue": operand}

    def _expr_cond(self, node: dict, scope: Scope) -> dict:
        condition = self._expression(node["c"], scope)
        when_true = self._expression(node["a"], scope)
        when_false = self._expression(node["b"], scope)
        type_name = self._common_type(when_true["type"], when_false["type"])
        return {
            "kind": "conditional", "type": type_name, "condition": condition,
            "whenTrue": when_true, "whenFalse": when_false,
        }

    def _expr_binary(self, node: dict, scope: Scope) -> dict:
        left = self._expression(node["l"], scope)
        right = self._expression(node["r"], scope)
        if node["op"] in {"==", "!=", "<", ">", "<=", ">=", "&&", "||"}:
            type_name = "bool"
        elif node["op"] == "*" and (left["type"].startswith("mat") or right["type"].startswith("mat")):
            type_name = right["type"] if left["type"].startswith("mat") else left["type"]
        else:
            type_name = self._common_type(left["type"], right["type"])
        return {"kind": "binary", "type": type_name, "operator": node["op"], "left": left, "right": right}

    def _expr_assign(self, node: dict, scope: Scope) -> dict:
        target = self._expression(node["target"], scope)
        if target["kind"] not in {"identifier", "member", "index"}:
            raise SyntaxError(f"typed IR: invalid assignment target {target['kind']!r}")
        value = self._expression(node["value"], scope)
        return {
            "kind": "assignment", "type": target["type"], "operator": node["op"],
            "lvalue": target, "value": value,
        }

    def _expr_construct(self, node: dict, scope: Scope) -> dict:
        arguments = [self._expression(argument, scope) for argument in node["args"]]
        type_name = self._declared_type(node["type"], node.get("array"))
        return {"kind": "construct", "type": type_name, "arguments": arguments}

    def _expr_call(self, node: dict, scope: Scope) -> dict:
        arguments = [self._expression(argument, scope) for argument in node["args"]]
        argument_types = [argument["type"] for argument in arguments]
        name = node["name"]
        if name in self.functions:
            signature = self._resolve_overload(name, argument_types)
            target = signature["mangledName"]
            type_name = signature["returnType"]
        else:
            target = f"builtin:{name}"
            type_name = self._builtin_return(name, argument_types)
        return {"kind": "call", "type": type_name, "name": name, "target": target, "arguments": arguments}

    def _resolve_overload(self, name: str, argument_types: list[str]) -> dict:
        candidates = [item for item in self.functions[name] if len(item["parameterTypes"]) == len(argument_types)]
        exact = [item for item in candidates if item["parameterTypes"] == argument_types]
        if len(exact) == 1:
            return exact[0]
        compatible = [
            item for item in candidates
            if all(_width(left) == _width(right) for left, right in zip(item["parameterTypes"], argument_types, strict=True))
        ]
        if len(compatible) == 1:
            return compatible[0]
        if len(candidates) == 1:
            return candidates[0]
        raise SyntaxError(f"typed IR: cannot resolve overload {name}({', '.join(argument_types)})")

    def _builtin_return(self, name: str, arguments: list[str]) -> str:
        if name in TYPES or name in self.structs:
            return name
        if name in {"texture", "textureLod", "texelFetch"}:
            return "vec4"
        if name == "textureSize":
            if arguments and arguments[0] in {"sampler3D", "samplerCube", "sampler2DArray"}:
                return "ivec3"
            return "ivec2"
        if name in {"length", "distance", "dot", "determinant"}:
            return "float"
        if name in {"any", "all"}:
            return "bool"
        if name == "isnan":
            return _type_for("bool", _width(arguments[0]) if arguments else 1)
        if name in {"lessThan", "lessThanEqual", "greaterThan", "greaterThanEqual", "equal", "notEqual"}:
            return _type_for("bool", _width(arguments[0]) if arguments else 1)
        if name == "packHalf2x16":
            return "uint"
        if name == "floatBitsToUint":
            return "uint" if not arguments or _width(arguments[0]) == 1 else _type_for("uint", _width(arguments[0]))
        if name == "unpackHalf2x16":
            return "vec2"
        if name == "uintBitsToFloat":
            return "float" if not arguments or _width(arguments[0]) == 1 else _type_for("float", _width(arguments[0]))
        if name == "__array_length":
            return "int"
        if name == "step" and len(arguments) >= 2:
            return arguments[1]
        if name == "smoothstep" and len(arguments) >= 3:
            return arguments[2]
        if not arguments:
            return "float"
        return arguments[0]

    def _common_type(self, left: str, right: str) -> str:
        if left == right:
            return left
        width = max(_width(left), _width(right))
        bases = {_base(left), _base(right)}
        base = "uint" if "uint" in bases else "int" if bases == {"int"} else "float"
        return _type_for(base, width)

    def _declared_type(self, name: str, array: object) -> str:
        type_name = name
        return f"{type_name}[]" if array is not None else type_name

    def _array_literal(self, array: object) -> object:
        if array is None or array is True:
            return None
        if isinstance(array, dict) and array.get("k") == "num":
            return int(array["value"], 0)
        return None

    def _array_expression(self, array: object, scope: Scope) -> dict | None:
        return None if array is None or array is True else self._expression(array, scope)


def emit_typed_ir(
    source: str,
    outputs: list[str] | None = None,
    varyings: list[str] | None = None,
    runtime_defines: dict[str, str] | None = None,
) -> dict:
    """Normalize, parse, and fully resolve one GLSL program."""
    normalized = normalize(source, runtime_defines or {})
    actual_outputs = list(outputs) if outputs is not None else normalized["outputs"]
    actual_varyings = list(varyings) if varyings is not None else normalized["varyings"]
    program = parse(normalized["source"])
    return TypedIrEmitter(copy.deepcopy(program), actual_outputs, actual_varyings).emit()

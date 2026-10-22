"""Resolve normalized GLSL into a deterministic, executable structural IR.

The output deliberately contains data only.  It is consumed by the Rust VM;
no Python source, closures, or callable objects are embedded in the bundle.
"""

from __future__ import annotations

import copy
import re
import struct

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
    def __init__(self, parent: "Scope | None" = None, owner: str | None = None):
        self.parent = parent
        self.owner = owner or (parent.owner if parent else "global")
        self.values: dict[str, tuple[str, str]] = {}
        self.bindings: dict[str, dict] = {}

    def child(self, owner: str | None = None) -> "Scope":
        return Scope(self, owner)

    def define(self, name: str, type_name: str, storage: str, initializer=None, constant=False) -> None:
        self.values[name] = (type_name, storage)
        self.bindings[name] = {"type": type_name, "storage": storage, "initializer": initializer,
                               "constant": constant, "scope": self}

    def binding(self, name: str) -> dict | None:
        scope = self
        while scope is not None:
            if name in scope.bindings:
                return scope.bindings[name]
            scope = scope.parent
        return None

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

    def __init__(self, program: dict, outputs: list[str], varyings: list[str], *, original_source="", runtime_defines=None, effect_id=None):
        self.program = program
        self.outputs = list(outputs)
        self.varyings = list(varyings)
        self.varying_types = program.get("varyingTypes") or {}
        self.root = Scope()
        self.structs: dict[str, list[tuple[str, str, object]]] = {}
        self.functions: dict[str, list[dict]] = {}
        self.function_nodes: list[tuple[dict, dict]] = []
        self.globals: list[dict] = []
        self.uniforms: list[dict] = []
        self.runtime_defines = set(runtime_defines or {})
        self.original_source = original_source
        self.effect_id = effect_id
        self.integer_names = set()
        for pattern in (r"\buniform\s+int\s+([A-Za-z_$]\w*)", r"\bint\s+([A-Za-z_$]\w*)\s*(?:=|;)",
                        r"\bivec[234]\s+([A-Za-z_$]\w*)", r"\bint\s+([A-Za-z_$]\w*)\s*(?:,|\))"):
            self.integer_names.update(re.findall(pattern, original_source))
        if effect_id == "synth/testPattern" and ("int temp = number;" not in original_source or "temp /= 10;" not in original_source):
            raise ValueError("testPattern digit division source changed")

    def emit(self) -> dict:
        self._collect()
        functions = [self._function(node, signature) for node, signature in self.function_nodes]
        if not any(function["name"] == "main" for function in functions):
            raise SyntaxError("shader has no main()")
        return {
            "cpuMurmurFunctions": self._murmur_functions(),
            "outputs": self.outputs,
            "varyings": self.varyings,
            **({"varyingTypes": dict(self.varying_types)} if self.varying_types else {}),
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
            self.root.define(name, self.varying_types.get(name, "vec2"), "varying")
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
            if initializer is not None:
                self._restore_decl_division(declarator["name"], declarator["init"], initializer, self.root)
            self.root.define(declarator["name"], type_name, storage, declarator.get("init"), "const" in declaration.get("quals", []))
            value = {"name": declarator["name"], "type": type_name}
            if storage == "uniform":
                self.uniforms.append(value)
            else:
                value["initializer"] = initializer
                value["arraySize"] = self._array_expression(declarator.get("array"), self.root)
                self.globals.append(value)

    def _function(self, node: dict, signature: dict) -> dict:
        scope = self.root.child(node["name"])
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
                if initializer is not None:
                    self._restore_decl_division(declarator["name"], declarator["init"], initializer, scope)
                array_size = self._array_expression(declarator.get("array"), scope)
                scope.define(declarator["name"], type_name, "local", declarator.get("init"), "const" in statement.get("quals", []))
                declarations.append({
                    "name": declarator["name"], "type": type_name,
                    "initializer": initializer, "arraySize": array_size,
                })
            return {"kind": "declaration", "declarations": declarations}
        if kind == "expr":
            lowered = self._lower_conditional_vector_assignment(statement["expr"], scope)
            if lowered is not None:
                return lowered
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
            value = struct.unpack("f", struct.pack("f", value))[0]
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
        operator = "metal/" if node["op"] == "/" and self._metal_division(node["l"], node["r"], left, right, scope) else node["op"]
        if operator == "metal/" and "mat2" in {left["type"], right["type"]}:
            type_name = "mat2"
        return {"kind": "binary", "type": type_name, "operator": operator, "left": left, "right": right}

    def _lower_conditional_vector_assignment(self, node: dict, scope: Scope) -> dict | None:
        """Match glsl-transpiler's lowering of `target = cond ? a : b` for vectors.

        The transpiler lowers a whole-vector assignment by appending the
        vector-copy reducer to the right-hand side text; for a conditional
        right-hand side the reducer binds to the false arm only, so the
        emitted JavaScript is `cond ? a : b.reduce(..., target)`: the true
        arm's value is computed and discarded and the target keeps its prior
        value. The reference compiler reproduces this verbatim (it rewrites
        only pointsBillboardRender's outRGB line to an honest if/else), so the
        typed IR keeps the same lossy semantics here.
        """
        if node.get("k") != "assign" or node.get("op") != "=":
            return None
        target = self._expression(node["target"], scope)
        if target["kind"] not in {"identifier", "member", "index"}:
            return None
        if _width(target["type"]) <= 1:
            return None
        value_node = node["value"]
        if value_node.get("k") != "cond":
            return None
        if self.effect_id == "render/pointsBillboardRender" and target["kind"] == "identifier" and target.get("name") == "outRGB":
            return None
        if target["kind"] == "identifier" and target.get("name") == "i1" and self._is_simplex_axis_choice(value_node):
            return None
        if self.effect_id == "points/physical" and target["kind"] == "identifier" and target.get("name") == "outRGBA":
            return None
        condition = self._expression(value_node["c"], scope)
        when_true = self._expression(value_node["a"], scope)
        when_false = self._expression(value_node["b"], scope)
        return {
            "kind": "if",
            "hoistedDeclarations": [],
            "condition": condition,
            "then": {"kind": "expression", "expression": when_true},
            "else": {"kind": "expression", "expression": {
                "kind": "assignment", "type": target["type"], "operator": "=",
                "lvalue": target, "value": when_false,
            }},
        }

    def _is_simplex_axis_choice(self, value_node: dict) -> bool:
        """True for `i1 = (x0.x > x0.y) ? vec2(1.0, 0.0) : vec2(0.0, 1.0)`.

        The reference compiler rewrites exactly this simplex-noise statement
        to an honest if/else before transpiling (compile-glsl.js applies the
        replacement to every effect), so it keeps both arms.
        """
        if value_node.get("k") != "cond":
            return False
        true_arm, false_arm = value_node.get("a"), value_node.get("b")
        for arm, comps in ((true_arm, ("1.0", "0.0")), (false_arm, ("0.0", "1.0"))):
            if not arm or arm.get("k") != "construct" or len(arm.get("args", [])) != 2:
                return False
            for item, expected in zip(arm["args"], comps):
                if item.get("k") != "num" or str(item.get("value")) != expected:
                    return False
        condition = value_node.get("c")
        return bool(condition and condition.get("k") == "binary" and condition.get("op") == ">")

    def _expr_assign(self, node: dict, scope: Scope) -> dict:
        target = self._expression(node["target"], scope)
        if target["kind"] not in {"identifier", "member", "index"}:
            raise SyntaxError(f"typed IR: invalid assignment target {target['kind']!r}")
        value = self._expression(node["value"], scope)
        operator = node["op"]
        right = node["value"]
        if operator == "/=" and _width(target["type"]) > 1:
            splat = (right.get("k") == "construct" and right.get("type") in VECTORS
                     and len(right["args"]) == 1 and _width(self._cpu_type(right["args"][0], scope)) == 1)
            if (_width(value["type"]) == 1 or splat) and self._metal_division(node["target"], right, target, value, scope):
                operator = "metal/="
        if self.effect_id == "synth/testPattern" and operator == "/=" and node["target"].get("name") == "temp" and self._cpu_text(right, scope) == "10":
            operator = "trunc/="
        return {
            "kind": "assignment", "type": target["type"], "operator": operator,
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

    def _murmur_functions(self) -> list[str]:
        source = re.sub(r"/\*.*?\*/|//[^\n]*", "", self.original_source, flags=re.S)
        names = []
        for name, argument, body in re.findall(r"\buint\s+([A-Za-z_]\w*)\s*\(\s*uint\s+([A-Za-z_]\w*)\s*\)\s*\{([^{}]*)\}", source):
            statements = [re.sub(r"\s+", "", part) for part in body.split(";") if part.strip()]
            if not statements or statements.pop() != "return" + argument:
                continue
            steps = []
            for statement in statements:
                shift = re.fullmatch(rf"{argument}(?:\^=|={argument}\^)\(?{argument}>>(\d+)u\)?", statement)
                multiply = re.fullmatch(rf"{argument}(?:\*=|={argument}\*)(0x[0-9A-Fa-f]+)u", statement)
                steps.append("^>>" + shift[1] if shift else "*" + multiply[1].lower() if multiply else None)
            if steps == ["^>>16", "*0x7feb352d", "^>>15", "*0x846ca68b", "^>>16"]:
                names.append(name)
        return sorted(set(names))

    def _cpu_type(self, node: dict, scope: Scope) -> str:
        kind = node["k"]
        if kind == "id":
            return (scope.resolve(node["name"]) or ("float", ""))[0]
        if kind == "num":
            raw = node["value"].lower()
            return "uint" if raw.endswith("u") else "int" if raw.startswith("0x") or re.fullmatch(r"\d+", raw) else "float"
        if kind == "construct":
            target = node["type"]
            if len(node["args"]) == 1:
                argument = node["args"][0]
                inner = self._cpu_type(argument, scope)
                if target == "float" and self._stripped_float_cast(node):
                    return inner
                if target in VECTORS and _width(target) == _width(inner) and _width(inner) > 1:
                    return inner
            return target
        if kind in {"member", "index"}:
            obj = self._cpu_type(node["obj"], scope)
            if obj in self.structs:
                return obj
            width = len(node["field"]) if kind == "member" else 1
            return _type_for(_base(obj), width)
        if kind in {"unary", "post"}:
            return self._cpu_type(node["x"], scope)
        if kind == "binary":
            return self._common_type(self._cpu_type(node["l"], scope), self._cpu_type(node["r"], scope))
        if kind == "cond":
            return self._common_type(self._cpu_type(node["a"], scope), self._cpu_type(node["b"], scope))
        if kind == "call":
            if node["name"] in self.functions:
                return self.functions[node["name"]][0]["returnType"]
            return self._builtin_return(node["name"], [self._cpu_type(arg, scope) for arg in node["args"]])
        return "float"

    @staticmethod
    def _stripped_float_cast(node: dict) -> bool:
        return (node.get("k") == "construct" and node.get("type") == "float" and len(node["args"]) == 1
                and (node["args"][0].get("k") == "id" or
                     (node["args"][0].get("k") == "member" and len(node["args"][0]["field"]) == 1)))

    def _cpu_text(self, node: dict, scope: Scope) -> str:
        kind = node["k"]
        if kind == "id":
            text = node["name"]
        elif kind == "num":
            raw = node["value"]
            text = str(int(raw.rstrip("uU"), 16)) if raw.lower().startswith("0x") else raw.rstrip("uUfF")
        elif kind == "member":
            obj = self._cpu_text(node["obj"], scope)
            field = node["field"]
            if self._cpu_type(node["obj"], scope) in self.structs:
                text = obj + "." + field
            elif len(field) == 1:
                text = f"{obj}[{'xyzwrgbastpq'.index(field) % 4}]"
            else:
                text = f"swizzle({obj}, {field})"
        elif kind == "index":
            text = f"{self._cpu_text(node['obj'], scope)}[{self._cpu_text(node['idx'], scope)}]"
        elif kind == "construct":
            args = [self._cpu_text(arg, scope) for arg in node["args"]]
            if node["type"] == "mat2":
                args = [str(int(float(text))) if arg.get("k") == "num" and float(text).is_integer() else text
                        for arg, text in zip(node["args"], args)]
            if self._stripped_float_cast(node):
                text = f"({args[0]})"
            elif node["type"] in {"int", "uint"} and len(args) == 1:
                text = str(int(float(args[0]))) if node["args"][0].get("k") == "num" else args[0] + "|0"
            elif node["type"] in VECTORS and len(args) == 1 and _width(self._cpu_type(node["args"][0], scope)) == _width(node["type"]):
                text = args[0]
            else:
                text = node["type"] + "(" + ", ".join(args) + ")"
        elif kind == "binary":
            left, right = self._cpu_text(node["l"], scope), self._cpu_text(node["r"], scope)
            if node["l"].get("k") == "binary" and node["op"] in {"+", "-"} and node["l"]["op"] in {"+", "-"}:
                left = left[1:-1]
            text = f"({left} {node['op']} {right})"
        elif kind in {"unary", "post"}:
            text = node["op"] + self._cpu_text(node["x"], scope)
        elif kind == "call":
            text = node["name"] + "(" + ", ".join(self._cpu_text(arg, scope) for arg in node["args"]) + ")"
        elif kind == "cond":
            text = " ? ".join(self._cpu_text(node[key], scope) for key in ("c", "a", "b"))
        else:
            text = ""
        return "(" * node.get("groups", 0) + text + ")" * node.get("groups", 0)

    def _operand_kind(self, node: dict, scope: Scope) -> str | None:
        text = self._cpu_text(node, scope).strip()
        if re.fullmatch(r"[-+]?\d+", text):
            return "int"
        if re.fullmatch(r"[-+]?(?:\d*\.?\d+|\d+\.?\d*)(?:[eE][-+]?\d+)?", text) or re.fullmatch(r"\(\s*[A-Za-z_]\w*(?:\[\d+\])?\s*\)", text):
            return "float"
        bare = re.fullmatch(r"([A-Za-z_]\w*)(?:\[\d+\])?", text)
        entry = scope.binding(bare[1]) if bare else None
        if entry is None and re.search(r"(?<![\w.])(?:\d+\.\d*|\.\d+)(?:[eE][-+]?\d+)?(?![\w.])", text) and not re.search(r"\|0|cpu_[iu]vec|trunc", text):
            return "float"
        type_name = entry["type"] if entry else self._cpu_type(node, scope)
        return "int" if _base(type_name) in {"int", "uint"} else "float" if type_name in {"float", "vec2", "vec3", "vec4"} else None

    def _divisor_kind(self, node: dict, scope: Scope, names=None) -> str:
        names = {} if names is None else names
        result = "constant"
        for name in re.findall(r"[A-Za-z_][\w$]*", self._cpu_text(node, scope)):
            if name == "gl_FragCoord":
                result = "runtime"
                continue
            if name in self.runtime_defines:
                continue
            entry = scope.binding(name)
            if entry is None or entry["constant"]:
                continue
            if entry["storage"] == "uniform":
                result = "runtime"
                continue
            if entry["scope"].owner not in {"main", "global"} or entry["initializer"] is None:
                return "unknown"
            if name not in names:
                names[name] = "unknown"
                names[name] = self._divisor_kind(entry["initializer"], entry["scope"], names)
            if names[name] == "unknown":
                return "unknown"
            if names[name] == "runtime":
                result = "runtime"
        return result

    def _metal_division(self, left, right, typed_left, typed_right, scope) -> bool:
        widths = [_width(typed_left["type"]), _width(typed_right["type"])]
        width = max(widths)
        if width > 4 or any(value not in {1, width} for value in widths) or any(node["type"].startswith("mat") and node["type"] != "mat2" for node in (typed_left, typed_right)):
            return False
        return "float" in (self._operand_kind(left, scope), self._operand_kind(right, scope)) and self._divisor_kind(right, scope) == "runtime"

    def _restore_decl_division(self, name, source, value, scope):
        def ordinary_division(ast, expression):
            return ast.get("k") == "binary" and ast.get("op") == "/" and expression.get("operator") == "/"

        if ordinary_division(source, value):
            left, right = self._cpu_text(source["l"], scope), self._cpu_text(source["r"], scope)
            if right in self.integer_names and (re.fullmatch(r"[A-Za-z_$]\w*\[\d+\]", left) or left in self.integer_names and left != right):
                value["operator"] = "trunc/"
        elif source.get("k") == "cond" and name in self.integer_names:
            arms = [(source[key], value[field]) for key, field in (("a", "whenTrue"), ("b", "whenFalse"))]
            if all(ordinary_division(ast, expression) and self._cpu_text(ast["r"], scope) in self.integer_names
                   and re.fullmatch(r"[A-Za-z_$]\w*|\([^()]*\)", self._cpu_text(ast["l"], scope))
                   and (self._cpu_text(ast["l"], scope).startswith("(") or self._cpu_text(ast["l"], scope) in self.integer_names)
                   for ast, expression in arms):
                for _, expression in arms:
                    expression["operator"] = "trunc/"
        if self.effect_id == "render/pointsBillboardRender" and "int start = (index / (2 * runLength)) * (2 * runLength);" in self.original_source:
            if "int mid = (low + high) / 2;" not in self.original_source:
                raise ValueError("Billboard depth merge midpoint source changed")
            text = self._cpu_text(source, scope)
            if name == "start" and text == "((index / (2 * runLength)) * (2 * runLength))":
                value["left"]["operator"] = "trunc/"
            elif name == "mid" and text == "((low + high) / 2)":
                value["operator"] = "trunc/"


def emit_typed_ir(
    source: str,
    outputs: list[str] | None = None,
    varyings: list[str] | None = None,
    runtime_defines: dict[str, str] | None = None,
    effect_id: str | None = None,
) -> dict:
    """Normalize, parse, and fully resolve one GLSL program."""
    normalized = normalize(source, runtime_defines or {}, effect_id=effect_id)
    actual_outputs = list(outputs) if outputs is not None else normalized["outputs"]
    actual_varyings = list(varyings) if varyings is not None else normalized["varyings"]
    program = parse(normalized["source"])
    program["varyingTypes"] = normalized.get("varyingTypes", {})
    return TypedIrEmitter(copy.deepcopy(program), actual_outputs, actual_varyings,
                          original_source=source, runtime_defines=runtime_defines, effect_id=effect_id).emit()

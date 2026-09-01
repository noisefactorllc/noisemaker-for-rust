import hashlib
import json
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock


ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT))

from scripts.generate_bundle import (  # noqa: E402
    check_generated,
    generate_bundle,
    sync_param_contract_provenance,
)
from scripts.transpiler.typed_ir import emit_typed_ir  # noqa: E402


GLSL = """
uniform float gain;
out vec4 fragColor;
float choose(float value) { return value; }
vec2 choose(vec2 value) { return value; }
void bump(inout float value, out vec2 pair) {
  value += 1.0;
  pair = vec2(value, 2.5);
}
void main() {
  float local = choose(gain);
  vec2 pair;
  bump(local, pair);
  fragColor = vec4(pair, local, 1.0);
}
"""


def fixture_source(root: Path) -> Path:
    source = root / "source"
    source.mkdir()
    shader = GLSL.strip()
    key = "synth/test:main"
    metadata = {
        "provenance": {"source": "fixture", "version": "1.0", "base": "fixture://bundle"},
        "effects": {
            "synth/test": {
                "namespace": "synth",
                "func": "test",
                "kind": "generator",
                "domain": "image",
                "params": {"gain": {"type": "float", "default": 1.0, "uniform": "gain"}},
                "textures": {},
                "passes": [{
                    "name": "main",
                    "program": "main",
                    "key": key,
                    "inputs": {},
                    "outputs": {"fragColor": "outputTex"},
                }],
            }
        },
    }
    extracted = {"synth/test": {"programs": {"main": shader}}}
    digest = hashlib.sha256(shader.encode()).hexdigest()
    (source / "metadata.json").write_text(json.dumps(metadata), encoding="utf-8")
    (source / "effects.json").write_text(json.dumps(extracted), encoding="utf-8")
    (source / "bundle-lock.json").write_text(json.dumps({
        "source": "fixture://bundle", "version": "1.0", "hashes": {key: digest}
    }), encoding="utf-8")
    (source / "param-contract.json").write_text(json.dumps({
        "source": "fixture://upstream", "revision": "fixture-revision",
        "effects": {
            "synth/test": {
                "paramNames": ["gain"],
                "paramAliases": {"strength": "gain"},
            }
        },
    }), encoding="utf-8")
    return source


def walk(value):
    yield value
    if isinstance(value, dict):
        for child in value.values():
            yield from walk(child)
    elif isinstance(value, list):
        for child in value:
            yield from walk(child)


class TypedIrTests(unittest.TestCase):
    def test_types_literals_identifiers_calls_and_qualifiers(self):
        ir = emit_typed_ir(GLSL, outputs=["fragColor"], varyings=[])
        nodes = list(walk(ir))
        literals = [n for n in nodes if isinstance(n, dict) and n.get("kind") == "literal"]
        self.assertTrue(any(n.get("type") == "float" and n.get("value") == 2.5 for n in literals))
        identifiers = [n for n in nodes if isinstance(n, dict) and n.get("kind") == "identifier"]
        self.assertTrue(any(n.get("name") == "gain" and n.get("storage") == "uniform" for n in identifiers))
        self.assertTrue(any(n.get("name") == "local" and n.get("storage") == "local" for n in identifiers))
        calls = [n for n in nodes if isinstance(n, dict) and n.get("kind") == "call"]
        self.assertTrue(any(n.get("target") == "choose__float" for n in calls))
        bump = next(f for f in ir["functions"] if f["mangledName"] == "bump__float_vec2")
        self.assertEqual([p["qualifier"] for p in bump["parameters"]], ["inout", "out"])
        assignments = [n for n in nodes if isinstance(n, dict) and n.get("kind") == "assignment"]
        self.assertTrue(assignments)
        self.assertTrue(all("lvalue" in n for n in assignments))

    def test_unresolved_identifier_is_rejected(self):
        with self.assertRaisesRegex(SyntaxError, "unresolved identifier.*missing"):
            emit_typed_ir("out vec4 fragColor; void main(){ fragColor=vec4(missing); }", ["fragColor"], [])

    def test_every_expression_has_a_resolved_type(self):
        ir = emit_typed_ir(GLSL, outputs=["fragColor"], varyings=[])
        expressions = [
            node for node in walk(ir)
            if isinstance(node, dict) and node.get("kind") in {
                "literal", "identifier", "member", "index", "unary", "postfix",
                "conditional", "binary", "assignment", "construct", "call",
            }
        ]
        self.assertTrue(expressions)
        self.assertTrue(all(isinstance(node.get("type"), str) and node["type"] for node in expressions))

    def test_only_preprocessor_branches_hoist_declarations(self):
        source = """
        uniform int MODE;
        out vec4 fragColor;
        void main() {
          #if MODE == 1
          float selected = 1.0;
          #else
          float selected = 2.0;
          #endif
          if (selected > 0.0) { float ordinary = 1.0; }
          float ordinary = 2.0;
          fragColor = vec4(selected + ordinary);
        }
        """
        ir = emit_typed_ir(source, runtime_defines={"MODE": "int"})
        body = next(function for function in ir["functions"] if function["name"] == "main")["body"]
        self.assertEqual([item["name"] for item in body[0]["hoistedDeclarations"]], ["selected"])
        self.assertEqual(body[1]["hoistedDeclarations"], [])

    def test_pack_unpack_bitcast_and_isnan_builtin_types(self):
        ir = emit_typed_ir("""
        out vec4 fragColor;
        void main() {
          uint packed = packHalf2x16(vec2(1.0));
          vec2 unpacked = unpackHalf2x16(packed);
          uvec2 bits = floatBitsToUint(unpacked);
          vec2 restored = uintBitsToFloat(bits);
          bvec2 invalid = isnan(restored);
          fragColor = vec4(restored, float(any(invalid)), 1.0);
        }
        """)
        calls = {
            node["name"]: node["type"]
            for node in walk(ir)
            if isinstance(node, dict) and node.get("kind") == "call"
        }
        self.assertEqual(calls["packHalf2x16"], "uint")
        self.assertEqual(calls["unpackHalf2x16"], "vec2")
        self.assertEqual(calls["floatBitsToUint"], "uvec2")
        self.assertEqual(calls["uintBitsToFloat"], "vec2")
        self.assertEqual(calls["isnan"], "bvec2")


class GeneratorTests(unittest.TestCase):
    def test_check_generated_rejects_stale_parameter_contract_provenance(self):
        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            source = fixture_source(root)
            output = root / "generated"
            generate_bundle(source, output)
            contract_path = source / "param-contract.json"
            contract = json.loads(contract_path.read_text())
            contract["revision"] = "next-revision"
            contract_path.write_text(json.dumps(contract), encoding="utf-8")
            with self.assertRaisesRegex(RuntimeError, "generated paramContractRevision drift"):
                check_generated(output, contract_path)

    def test_sync_parameter_contract_provenance_updates_both_generated_artifacts(self):
        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            source = fixture_source(root)
            output = root / "generated"
            generate_bundle(source, output)
            contract_path = source / "param-contract.json"
            contract = json.loads(contract_path.read_text())
            contract.update({"source": "next-source", "revision": "next-revision"})
            contract_path.write_text(json.dumps(contract), encoding="utf-8")
            sync_param_contract_provenance(output, contract_path)
            for name in ("catalog.json", "shaders.json"):
                provenance = json.loads((output / name).read_text())["provenance"]
                self.assertEqual(provenance["paramContractSource"], "next-source")
                self.assertEqual(provenance["paramContractRevision"], "next-revision")

    def test_sync_rejects_incompatible_contract_and_preserves_generated_output(self):
        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            source = fixture_source(root)
            output = root / "generated"
            generate_bundle(source, output)
            original = {name: (output / name).read_bytes() for name in ("catalog.json", "shaders.json")}
            contract_path = source / "param-contract.json"
            contract = json.loads(contract_path.read_text())
            contract["revision"] = "incompatible-revision"
            contract["effects"]["synth/test"]["paramNames"] = []
            contract_path.write_text(json.dumps(contract), encoding="utf-8")
            with self.assertRaisesRegex(RuntimeError, "generated paramNames drift"):
                sync_param_contract_provenance(output, contract_path)
            for name, expected in original.items():
                self.assertEqual((output / name).read_bytes(), expected)

    def test_sync_treats_post_commit_backup_cleanup_as_best_effort(self):
        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            source = fixture_source(root)
            output = root / "generated"
            generate_bundle(source, output)
            contract_path = source / "param-contract.json"
            contract = json.loads(contract_path.read_text())
            contract["revision"] = "next-revision"
            contract_path.write_text(json.dumps(contract), encoding="utf-8")
            real_rmtree = shutil.rmtree

            def fail_backup_cleanup(path, *args, **kwargs):
                if ".generated.backup-" in Path(path).name:
                    raise OSError("simulated backup cleanup failure")
                return real_rmtree(path, *args, **kwargs)

            with mock.patch("scripts.generate_bundle.shutil.rmtree", side_effect=fail_backup_cleanup):
                sync_param_contract_provenance(output, contract_path)
            catalog = json.loads((output / "catalog.json").read_text())
            self.assertEqual(catalog["provenance"]["paramContractRevision"], "next-revision")
            backups = list(root.glob(".generated.backup-*"))
            self.assertEqual(len(backups), 1)
            real_rmtree(backups[0])

    def test_canonical_parameter_order_and_aliases_are_emitted_from_locked_contract(self):
        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            source = fixture_source(root)
            metadata = json.loads((source / "metadata.json").read_text())
            metadata["effects"]["synth/test"]["params"]["offset"] = {
                "type": "float", "default": 0.0, "uniform": "offset"
            }
            (source / "metadata.json").write_text(json.dumps(metadata), encoding="utf-8")
            contract = json.loads((source / "param-contract.json").read_text())
            contract["effects"]["synth/test"]["paramNames"] = ["offset", "gain"]
            (source / "param-contract.json").write_text(json.dumps(contract), encoding="utf-8")
            output = root / "out"
            generate_bundle(source, output)
            effect = json.loads((output / "catalog.json").read_text())["effects"]["synth/test"]
            self.assertEqual(effect["paramNames"], ["offset", "gain"])
            self.assertEqual(effect["paramAliases"], {"strength": "gain"})

    def test_parameter_contract_rejects_alias_collision_and_missing_target(self):
        for aliases, message in [
            ({"gain": "gain"}, "collides with canonical parameter"),
            ({"strength": "missing"}, "targets unknown parameter"),
        ]:
            with self.subTest(aliases=aliases), tempfile.TemporaryDirectory() as td:
                root = Path(td)
                source = fixture_source(root)
                contract = json.loads((source / "param-contract.json").read_text())
                contract["effects"]["synth/test"]["paramAliases"] = aliases
                (source / "param-contract.json").write_text(json.dumps(contract), encoding="utf-8")
                with self.assertRaisesRegex(RuntimeError, message):
                    generate_bundle(source, root / "out")

    def test_parameter_contract_restores_canonical_numeric_bounds(self):
        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            source = fixture_source(root)
            metadata = json.loads((source / "metadata.json").read_text())
            metadata["effects"]["synth/test"]["params"]["gain"].update(
                {"default": 300, "min": 50, "max": 10}
            )
            (source / "metadata.json").write_text(json.dumps(metadata), encoding="utf-8")
            contract = json.loads((source / "param-contract.json").read_text())
            contract["effects"]["synth/test"]["paramOverrides"] = {
                "gain": {"max": 1000}
            }
            (source / "param-contract.json").write_text(json.dumps(contract), encoding="utf-8")

            output = root / "out"
            generate_bundle(source, output)
            spec = json.loads((output / "catalog.json").read_text())["effects"]["synth/test"]["params"]["gain"]
            self.assertEqual(spec, {
                "default": 300,
                "max": 1000,
                "min": 50,
                "type": "float",
                "uniform": "gain",
            })

    def test_parameter_contract_rejects_unknown_override_fields(self):
        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            source = fixture_source(root)
            contract = json.loads((source / "param-contract.json").read_text())
            contract["effects"]["synth/test"]["paramOverrides"] = {
                "gain": {"unexpected": 1000}
            }
            (source / "param-contract.json").write_text(json.dumps(contract), encoding="utf-8")
            with self.assertRaisesRegex(RuntimeError, "parameter override synth/test.gain is invalid"):
                generate_bundle(source, root / "out")

    def test_deterministic_sorted_outputs_and_exact_inventory(self):
        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            source = fixture_source(root)
            first, second = root / "first", root / "second"
            generate_bundle(source, first)
            generate_bundle(source, second)
            for name in ("catalog.json", "shaders.json", "bundle-lock.json"):
                self.assertEqual((first / name).read_bytes(), (second / name).read_bytes())
                text = (first / name).read_text(encoding="utf-8")
                self.assertEqual(text, json.dumps(json.loads(text), indent=2, sort_keys=True) + "\n")
            catalog = json.loads((first / "catalog.json").read_text())
            shaders = json.loads((first / "shaders.json").read_text())
            lock = json.loads((first / "bundle-lock.json").read_text())
            pass_keys = {p["key"] for e in catalog["effects"].values() for p in e["passes"] if p["key"]}
            self.assertEqual(pass_keys, set(shaders["programs"]))
            self.assertEqual(pass_keys, set(lock["hashes"]))

    def test_source_hash_drift_is_a_hard_failure_and_preserves_output(self):
        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            source = fixture_source(root)
            out = root / "out"
            out.mkdir()
            marker = out / "marker"
            marker.write_text("untouched", encoding="utf-8")
            effects = json.loads((source / "effects.json").read_text())
            effects["synth/test"]["programs"]["main"] += "\n// drift"
            (source / "effects.json").write_text(json.dumps(effects), encoding="utf-8")
            with self.assertRaisesRegex(RuntimeError, "shader drift"):
                generate_bundle(source, out)
            self.assertEqual(marker.read_text(), "untouched")
            self.assertEqual({p.name for p in out.iterdir()}, {"marker"})

    def test_cli_returns_nonzero_for_unresolved_identifier(self):
        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            source = fixture_source(root)
            effects = json.loads((source / "effects.json").read_text())
            bad = "out vec4 fragColor; void main(){ fragColor=vec4(missing); }"
            effects["synth/test"]["programs"]["main"] = bad
            (source / "effects.json").write_text(json.dumps(effects), encoding="utf-8")
            lock = json.loads((source / "bundle-lock.json").read_text())
            lock["hashes"]["synth/test:main"] = hashlib.sha256(bad.encode()).hexdigest()
            (source / "bundle-lock.json").write_text(json.dumps(lock), encoding="utf-8")
            result = subprocess.run(
                [sys.executable, str(ROOT / "scripts" / "generate_bundle.py"),
                 "--source", str(source), "--out", str(root / "out")],
                cwd=ROOT, capture_output=True, text=True, check=False,
            )
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("unresolved identifier", result.stderr)


if __name__ == "__main__":
    unittest.main()

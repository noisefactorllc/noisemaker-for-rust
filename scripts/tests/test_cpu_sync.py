import hashlib
import json
import struct
import tempfile
import unittest
from pathlib import Path

from scripts.generate_bundle import generate_bundle
from scripts.tests.test_generate_bundle import fixture_source, walk
from scripts.transpiler.preprocess import normalize
from scripts.transpiler.typed_ir import emit_typed_ir


def declarations(ir):
    return {
        declaration["name"]: declaration["initializer"]
        for node in walk(ir) if isinstance(node, dict) and node.get("kind") == "declaration"
        for declaration in node["declarations"]
    }


class CpuDivisionTests(unittest.TestCase):
    def expression(self, expression, *, prefix="", defines=None):
        source = """
uniform float width;
uniform int CW;
uniform int a;
uniform int b;
uniform vec2 dims;
uniform ivec2 idims;
out vec4 fragColor;
""" + prefix + "\nvoid main(){float x=" + expression + ";fragColor=vec4(x);}"
        return declarations(emit_typed_ir(source, runtime_defines=defines))["x"]

    def test_runtime_folded_helper_and_uninitialized_divisors(self):
        cases = [
            ("1.0/width", "", "metal/"),
            ("1.0/41.0", "", "/"),
            ("1.0/C", "const float C=41.0;", "/"),
            ("1.0/globalWidth", "float globalWidth=width;", "metal/"),
        ]
        for expression, prefix, expected in cases:
            with self.subTest(expression=expression):
                self.assertEqual(self.expression(expression, prefix=prefix)["operator"], expected)
        ir = emit_typed_ir("""
uniform float width;
out vec4 fragColor;
float helper(float parameter){float local=width;return 1.0/(parameter+local);}
void main(){float runtimeWidth=width+1.0;float unset;float folded=41.0;
float x=1.0/runtimeWidth;float y=1.0/unset;float z=1.0/folded;fragColor=vec4(x,y,z,helper(width));}
""")
        values = declarations(ir)
        self.assertEqual([values[name]["operator"] for name in ("x", "y", "z")], ["metal/", "/", "/"])
        helper = next(function for function in ir["functions"] if function["name"] == "helper")
        self.assertEqual(helper["body"][-1]["value"]["operator"], "/")

    def test_emitted_grouping_cast_and_integer_vector_evidence(self):
        for expression, expected in [
            ("a/CW", "trunc/"), ("(a)/CW", "metal/"), ("((a))/CW", "/"),
            ("1/(CW)", "metal/"), ("1/((CW))", "/"),
            ("float(a)/CW", "metal/"), ("(float(a))/CW", "/"),
            ("(a+float(b))/CW", "/"), ("(a+0.5)/CW", "metal/"),
            ("vec2(idims)/vec2(idims)", "/"),
            ("vec2(a)/vec2(idims)", "metal/"),
            ("mat3(1.0)/width", "/"),
        ]:
            with self.subTest(expression=expression):
                self.assertEqual(self.expression(expression)["operator"], expected)

    def test_compound_assignment_uses_actual_cpu_operand_shapes(self):
        cases = [
            ("vec2 p=vec2(1.0);p/=width;", "metal/="),
            ("vec2 p=vec2(1.0);p/=dims;", "/="),
            ("vec2 p=vec2(1.0);p.xy/=vec2(width);", "metal/="),
            ("vec2 p=vec2(1.0);p/=vec2(width,width);", "/="),
            ("vec2 p[1];p[0]=vec2(1.0);p[0]/=width;", "metal/="),
            ("float p=1.0;p/=width;", "/="),
            ("vec2 p=vec2(1.0);p[0]/=width;", "/="),
        ]
        for body, expected in cases:
            with self.subTest(body=body):
                ir = emit_typed_ir("uniform float width;uniform vec2 dims;out vec4 fragColor;void main(){" + body + "fragColor=vec4(1.0);}")
                assignments = [node for node in walk(ir) if isinstance(node, dict) and node.get("kind") == "assignment" and node["operator"].endswith("/=")]
                self.assertEqual(assignments[-1]["operator"], expected)

    def test_statement_integer_restoration_is_source_and_shape_checked(self):
        for expression, expected in [("a/CW", "trunc/"), ("float(a)/CW", "metal/"), ("(a)/CW", "metal/"), ("((a))/CW", "/"), ("1/CW", "/"), ("a/a", "/")]:
            source = "uniform int a;uniform int CW;out vec4 fragColor;void main(){int x=" + expression + ";fragColor=vec4(float(x));}"
            with self.subTest(expression=expression):
                self.assertEqual(declarations(emit_typed_ir(source))["x"]["operator"], expected)
        source = "uniform int CW;out vec4 fragColor;void main(){int a=1,b=1;int x=b/CW;fragColor=vec4(float(x));}"
        self.assertEqual(declarations(emit_typed_ir(source))["x"]["operator"], "/")
        source = "uniform int a;uniform int b;uniform int CW;out vec4 fragColor;void main(){int x=a>0?a/CW:(a-CW+1)/CW;fragColor=vec4(float(x));}"
        value = declarations(emit_typed_ir(source))["x"]
        self.assertEqual([value[key]["operator"] for key in ("whenTrue", "whenFalse")], ["trunc/", "trunc/"])

    def test_synthetic_defines_are_constant_without_integer_source_evidence(self):
        source = "#ifndef CW\n#define CW 41\n#endif\nout vec4 fragColor;\nvoid main(){int a=1;int x=a/CW;fragColor=vec4(float(x));}"
        self.assertEqual(declarations(emit_typed_ir(source, runtime_defines={"CW": "int"}))["x"]["operator"], "/")

    def test_integer_conditional_divisor_retains_cpu_integer_evidence(self):
        source = "uniform bool flag;uniform int width;uniform int height;out vec4 fragColor;void main(){int n=1;int q=n/(flag ? width : height);fragColor=vec4(float(q));}"
        value = declarations(emit_typed_ir(source))["q"]
        self.assertEqual(value["right"]["type"], "int")
        self.assertEqual(value["operator"], "/")

    def test_source_checked_digit_and_billboard_integer_exceptions(self):
        pattern = "uniform int number;out vec4 fragColor;void main(){int temp = number; temp /= 10; fragColor=vec4(float(temp));}"
        ir = emit_typed_ir(pattern, effect_id="synth/testPattern")
        self.assertTrue(any(node.get("operator") == "trunc/=" for node in walk(ir) if isinstance(node, dict)))
        with self.assertRaisesRegex(ValueError, "testPattern digit division source changed"):
            emit_typed_ir(pattern.replace("temp /= 10;", "temp /= 9;"), effect_id="synth/testPattern")
        billboard = """uniform int index;uniform int runLength;uniform int low;uniform int high;
out vec4 fragColor;
void main(){int start = (index / (2 * runLength)) * (2 * runLength);
int mid = (low + high) / 2;fragColor=vec4(float(start),float(mid),0.0,1.0);}
"""
        values = declarations(emit_typed_ir(billboard, effect_id="render/pointsBillboardRender"))
        self.assertEqual(values["start"]["left"]["operator"], "trunc/")
        self.assertEqual(values["mid"]["operator"], "trunc/")
        ordinary = declarations(emit_typed_ir(billboard))
        self.assertEqual(ordinary["start"]["left"]["operator"], "/")
        self.assertEqual(ordinary["mid"]["operator"], "/")

    def test_literals_and_murmur_metadata(self):
        ir = emit_typed_ir("""
uint mix(uint v){v^=v>>16u;v*=0x7feb352du;v^=v>>15u;v*=0x846ca68bu;v^=v>>16u;return v;}
uint near(uint v){v^=v>>16u;v*=0x7feb352du;v^=v>>15u;v*=0x846ca68bu;v^=v>>16u;return v+1u;}
out vec4 fragColor;
void main(){float x=101.7;fragColor=vec4(x);}
""")
        self.assertEqual(ir["cpuMurmurFunctions"], ["mix"])
        self.assertEqual(declarations(ir)["x"]["value"], struct.unpack("f", struct.pack("f", 101.7))[0])
        self.assertEqual(emit_typed_ir("out vec4 fragColor;void main(){fragColor=vec4(1.0);}")["cpuMurmurFunctions"], [])

    def test_hexadecimal_uint_cast_divisors_retain_all_digits(self):
        self.assertEqual(self.expression("1.0/float(uint(0xffffffffu))")["operator"], "/")

    def test_mat2_runtime_divisions_match_four_component_cpu_lowering(self):
        for expression, expected in [("matrix/width", "metal/"), ("1.0/matrix", "metal/"),
                                     ("mat2(1.0)/CW", "/"), ("mat2(0.5)/CW", "metal/"),
                                     ("mat2(1.0)/matrix", "/")]:
            with self.subTest(expression=expression):
                source = "uniform mat2 matrix;uniform float width;uniform int CW;out vec4 fragColor;void main(){mat2 z=" + expression + ";fragColor=vec4(1.0);}"
                result = declarations(emit_typed_ir(source))["z"]
                self.assertEqual(result["operator"], expected)
                if expected == "metal/":
                    self.assertEqual(result["type"], "mat2")
        ir = emit_typed_ir("uniform mat2 matrix;uniform float width;out vec4 fragColor;void main(){mat2 z=matrix;z/=width;fragColor=vec4(1.0);}")
        self.assertTrue(any(node.get("operator") == "metal/=" for node in walk(ir) if isinstance(node, dict)))


class CpuAdaptationTests(unittest.TestCase):
    def test_lattice_hashes_keep_corner_and_seed_addition_order(self):
        for width, effect in ((4, "synth3d/noise3d"), (3, "synth/perlin")):
            vector, hash_name, gradient = f"vec{width}", f"hash{width}", f"grad{width}"
            seed = "vec4 ps = p + float(seed) * 0.1;" if width == 4 else "// Add seed to input to vary the noise pattern\n    p = p + float(seed) * 0.1;"
            comment = "" if width == 4 else "    // Generate 3 independent random values\n"
            hashes = "\n".join(f"    float h{index + 1} = {hash_name}(p{offset});" for index, offset in enumerate(("", " + 127.1", " + 269.5", " + 419.2")[:width]))
            corners = []
            for index in range(1 << width):
                base = f"vec4(i.xyz, iw{index >> 3})" if width == 4 else f"vec3(i.xy, iz{index >> 2})"
                offset = ",".join([str((index >> bit) & 1) for bit in range(width - 1)] + ["0"])
                corners.append(f"{vector} c{index} = {gradient}({base} + {vector}({offset}));")
            source = f"float {hash_name}({vector} p) {{\n    {seed}\n}}\n{vector} {gradient}({vector} p) {{\n{comment}{hashes}\n}}\n" + "\n".join(corners)
            normalized = normalize(source, effect_id=effect)["source"]
            self.assertIn(f"float {hash_name}({vector} p, {vector} k)", normalized)
            self.assertIn("float cpuSeedOffset = float(seed * 0.1);", normalized)
            self.assertIn(f"{hash_name}(p, k + 127.1)", normalized)
            self.assertNotIn(f"{gradient}({base} +", normalized)
            if width == 4:
                self.assertIn("vec4 cpuLatticeOffset = k + cpuSeedOffset;", normalized)
                self.assertIn("vec4 ps = p + cpuLatticeOffset;", normalized)
            else:
                self.assertIn("float(p.z + cpuSeedOffset) + k.z", normalized)
            with self.assertRaisesRegex(ValueError, "expected .* lattice corners"):
                normalize(source.replace(corners[-1], ""), effect_id=effect)
        cell = "vec3 hash3(vec3 p) {\n    p = p + float(seed) * 0.1;\n}\nvec3 randomOffset = hash3(cellPos);"
        normalized = normalize(cell, effect_id="synth3d/cell3d")["source"]
        self.assertIn("vec3 hash3(vec3 base, vec3 neighbor)", normalized)
        self.assertIn("base.x + float(neighbor.x + cpuSeedOffset)", normalized)
        self.assertIn("cpuCellPos.y + cpuSeedOffset", normalized)
        self.assertIn("hash3(i, neighbor)", normalized)

    def test_mirror_scatter_extrude_and_navier_boundaries(self):
        mirror = normalize("float x=mod(uv.x + 1.0, 2.0) - 1.0;")["source"]
        self.assertIn("(uv.x - 2.0 * floor((uv.x + 1.0) * 0.5))", mirror)
        scatter = normalize("vec2 rnd = hash22(hashCoord + float(seed) * 101.7);\nreturn fract((p3.xx + p3.yz) * p3.zy);", effect_id="filter/scatter")["source"]
        self.assertIn("hash22(hashCoord + float(seed * 101.7))", scatter)
        self.assertIn("float(float(p3.x + p3.y) * p3.z)", scatter)
        navier = normalize("float hash11(float x) {\nreturn fract(sin(x * 12.9898) * 43758.5453);\n}", effect_id="synth/navierStokes")["source"]
        self.assertIn("sin(float(float(x) * 12.9898))", navier)
        extrude = normalize("float s = 1.0 + h * (depth / 100.0) * 0.4;\nbool topHit = all(lessThanEqual(abs(P - faceCenter), faceHalf));", effect_id="filter/extrude")["source"]
        self.assertIn("float s = float(1.0 + h * (depth / 100.0) * 0.4);", extrude)
        self.assertIn("abs((P - imgCenter) - (cellC - imgCenter) * s)", extrude)

    def test_simulation_seed_hash_groups_dot_terms(self):
        source = """float hash3(vec3 p) {
    p = p + float(seed) * 0.1;
    p = fract(p * vec3(0.1031, 0.1030, 0.0973));
    p += dot(p, p.yxz + 33.33);
    return fract((p.x + p.y) * p.z);
}"""
        for effect in ("synth3d/reactionDiffusion3d", "synth3d/cellularAutomata3d"):
            result = normalize(source, effect_id=effect)["source"]
            self.assertIn("float(seed * 0.1)", result)
            self.assertIn("float(cpuHashDot + cpuHashDot)", result)


class CpuGeneratorTests(unittest.TestCase):
    def test_generator_applies_effect_adaptation_without_changing_source_hashes(self):
        with tempfile.TemporaryDirectory() as directory:
            source = fixture_source(Path(directory))
            shader = """uniform float h;
uniform float depth;
out vec4 fragColor;
void main(){
vec2 P=vec2(20.5),imgCenter=vec2(20.5),cellC=vec2(20.0),faceHalf=vec2(0.5);
float s = 1.0 + h * (depth / 100.0) * 0.4;
vec2 faceCenter=imgCenter+(cellC-imgCenter)*s;
bool topHit = all(lessThanEqual(abs(P - faceCenter), faceHalf));
fragColor=vec4(float(topHit));
}"""
            metadata = json.loads((source / "metadata.json").read_text())
            effect = metadata["effects"].pop("synth/test")
            effect["passes"][0]["key"] = "filter/extrude:main"
            metadata["effects"]["filter/extrude"] = effect
            lock = json.loads((source / "bundle-lock.json").read_text())
            digest = hashlib.sha256(shader.encode()).hexdigest()
            lock["hashes"] = {"filter/extrude:main": digest}
            contract = json.loads((source / "param-contract.json").read_text())
            contract["effects"]["filter/extrude"] = contract["effects"].pop("synth/test")
            for name, value in (("metadata.json", metadata), ("bundle-lock.json", lock), ("param-contract.json", contract),
                                ("effects.json", {"filter/extrude": {"programs": {"main": shader}}})):
                (source / name).write_text(json.dumps(value))
            output = Path(directory) / "generated"
            generate_bundle(source, output)
            program = json.loads((output / "shaders.json").read_text())["programs"]["filter/extrude:main"]
            self.assertEqual(program["sourceSha256"], digest)
            self.assertEqual(declarations(program["ir"])["s"]["kind"], "construct")
            self.assertEqual(json.loads((output / "bundle-lock.json").read_text())["hashes"], lock["hashes"])


if __name__ == "__main__":
    unittest.main()

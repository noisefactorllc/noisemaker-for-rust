#!/usr/bin/env python3
"""Compare the generated catalog against the CPU snapshot records.

The CPU snapshot (src/effects/generated/upstream-snapshot.js in the CPU
checkout at NOISEMAKER_JS_CPU_DIR) is the authority for every definition
field. The generated catalog may additionally carry presentation fields the
snapshot does not model (ui, step, rand*) on parameters and a register key on
passes; every snapshot-defined field must match exactly.
"""
import json
import os
import subprocess
import sys
from pathlib import Path

REPOSITORY = Path(__file__).resolve().parents[1]
SNAPSHOT_EXPORT = """
const { effectRecords } = await import(process.env.NOISEMAKER_JS_CPU_DIR + '/src/effects/generated/upstream-snapshot.js');
process.stdout.write(JSON.stringify(effectRecords));
"""

PRESENTATION_FIELDS = frozenset({"ui", "step", "randChance", "randMin", "randMax", "randChoices"})


def load_snapshot() -> list:
    cpu_dir = os.environ.get("NOISEMAKER_JS_CPU_DIR")
    if not cpu_dir:
        raise RuntimeError("NOISEMAKER_JS_CPU_DIR must point at the CPU checkout")
    raw = subprocess.run(
        ["node", "--input-type=module", "-e", SNAPSHOT_EXPORT],
        check=True,
        capture_output=True,
        text=True,
    ).stdout
    return json.loads(raw)


def compare(catalog: dict, records: list) -> list:
    problems = []
    effects = catalog["effects"]
    if set(effects) != {record["id"] for record in records}:
        problems.append("effect id sets differ")
    for record in records:
        effect_id = record["id"]
        effect = effects.get(effect_id)
        if effect is None:
            continue
        for field, expected in record.items():
            if field == "params":
                for name, spec in expected.items():
                    actual = effect.get("params", {}).get(name)
                    if actual is None:
                        problems.append(f"{effect_id}: param {name} missing")
                        continue
                    for key, value in spec.items():
                        if key in PRESENTATION_FIELDS:
                            continue
                        if actual.get(key) != value:
                            problems.append(
                                f"{effect_id}: param {name}.{key} "
                                f"snapshot={json.dumps(value, sort_keys=True)} "
                                f"catalog={json.dumps(actual.get(key), sort_keys=True)}"
                            )
                continue
            if field == "passes":
                actual_passes = {
                    render_pass.get("name"): render_pass
                    for render_pass in effect.get("passes", [])
                }
                if len(actual_passes) != len(effect.get("passes", [])):
                    problems.append(f"{effect_id}: duplicate pass names")
                for render_pass in expected:
                    actual = actual_passes.get(render_pass.get("name"))
                    if actual is None:
                        problems.append(f"{effect_id}: pass {render_pass.get('name')} missing")
                        continue
                    for key, value in render_pass.items():
                        if key == "key":
                            continue
                        if actual.get(key) != value:
                            problems.append(
                                f"{effect_id}: pass {render_pass.get('name')}.{key} "
                                f"snapshot={json.dumps(value, sort_keys=True)} "
                                f"catalog={json.dumps(actual.get(key), sort_keys=True)}"
                            )
                continue
            if effect.get(field) != expected:
                problems.append(
                    f"{effect_id}: {field} snapshot={json.dumps(expected, sort_keys=True)} "
                    f"catalog={json.dumps(effect.get(field), sort_keys=True)}"
                )
    return problems


def main() -> int:
    catalog = json.loads((REPOSITORY / "src/generated/catalog.json").read_text())
    problems = compare(catalog, load_snapshot())
    for problem in problems[:40]:
        print(problem)
    print(f"snapshot-equivalence problems: {len(problems)}")
    return 1 if problems else 0


if __name__ == "__main__":
    sys.exit(main())

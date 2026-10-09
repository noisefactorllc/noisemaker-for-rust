import unittest
from pathlib import PureWindowsPath

from scripts import parity


class StrictParityTest(unittest.TestCase):
    def test_external_input_harness_uses_file_urls_for_windows_paths(self):
        script = parity._external_input_script(PureWindowsPath("D:/work/noisemaker-for-cpu"))
        self.assertIn("from 'file:///D:/work/noisemaker-for-cpu/src/index.js'", script)
        self.assertNotIn("from 'D:", script)

    def test_external_input_harness_accepts_a_sweep_program_override(self):
        script = parity._external_input_script(PureWindowsPath("D:/work/noisemaker-for-cpu"))
        self.assertIn("const program = programOverride ?? programs[caseId]", script)

    def test_one_byte_difference_fails_without_changing_the_over_two_metric(self):
        metrics = parity._metrics(bytes([60, 105, 165, 255]), bytes([61, 105, 165, 255]))
        self.assertFalse(metrics["pass"])
        self.assertEqual(metrics["max_delta"], 1)
        self.assertEqual(metrics["differing_channels"], 1)
        self.assertEqual(metrics["channels_over_2"], 0)
        record = {"id": "filter/crt", "status": "compared", **metrics}
        report = parity._summarize([record], 1, 0.25, 1)
        with self.assertRaisesRegex(ValueError, "unapproved delta"):
            parity._validate(report, ["filter/crt"])

    def test_identical_bytes_pass_with_zero_tolerance(self):
        metrics = parity._metrics(bytes([60, 105, 165, 255]), bytes([60, 105, 165, 255]))
        report = parity._summarize([{"id": "filter/crt", "status": "compared", **metrics}], 1, 0.25, 1)
        parity._validate(report, ["filter/crt"])
        self.assertEqual(report["byte_exact"], 1)
        self.assertEqual(report["tolerance"], 0)

    def test_default_summary_denominators_are_validated(self):
        metrics = parity._metrics(bytes(4), bytes(4))
        report = parity._summarize([{"id": "filter/crt", "status": "compared", **metrics}], 1, 0.25, 1)
        for key, value in (("expected", 2), ("executed", 0), ("catalog", 5)):
            broken = dict(report)
            broken[key] = value
            with self.assertRaisesRegex(ValueError, "summary denominator mismatch"):
                parity._validate(broken, ["filter/crt"])

    def test_worm_overlay_effects_are_compared_in_ready_mode_not_skipped(self):
        catalog = parity._catalog()
        self.assertEqual(
            parity.OVERLAY_READY_IDS,
            {"filter/fibers", "filter/scratches", "filter/strayHair"},
        )
        for effect_id in sorted(parity.OVERLAY_READY_IDS):
            self.assertIn(effect_id, catalog)
            metrics = parity._metrics(bytes(4), bytes(4))
            record = {
                "id": effect_id,
                "status": "compared",
                "overlay_one_shot": "ready",
                **metrics,
            }
            report = parity._summarize([record], 1, 0.25, 1)
            parity._validate(report, [effect_id])
            self.assertEqual(report["compared"], 1)

    def test_default_summary_states_expected_and_executed_denominators(self):
        metrics = parity._metrics(bytes(4), bytes(4))
        report = parity._summarize([{"id": "filter/crt", "status": "compared", **metrics}], 1, 0.25, 1)
        self.assertEqual(report["expected"], 1)
        self.assertEqual(report["executed"], 1)
        self.assertEqual(report["tolerance"], 0)


class SweepValueTest(unittest.TestCase):
    def test_numeric_parameters_use_bounds_then_step(self):
        self.assertEqual(parity._sweep_value({"type": "float", "default": 0.5, "min": 0, "max": 1}), 1)
        self.assertEqual(parity._sweep_value({"type": "int", "default": 4, "min": -3, "max": 4}), -3)
        self.assertIsNone(parity._sweep_value({"type": "int", "default": 4, "min": 4, "max": 4}))
        self.assertEqual(parity._sweep_value({"type": "float", "default": 1, "step": 0.5}), 1.5)

    def test_cost_scaling_iteration_parameters_capped_at_twice_the_default(self):
        self.assertEqual(
            parity._variant_candidates("synth/mandelbrot", parity._catalog()["synth/mandelbrot"]),
            [("iterations", 1000), ("zoomSpeed", 5)],
        )
        candidates = dict(
            parity._variant_candidates("points/buddhabrot", parity._catalog()["points/buddhabrot"])
        )
        self.assertEqual(candidates.get("maxIter"), 400)
        uncapped = dict(parity._variant_candidates("synth/sides", {
            "domain": "image", "kind": "generator", "namespace": "synth", "func": "sides",
            "paramNames": ["sides"],
            "params": {"sides": {"type": "int", "default": 8, "min": 3, "max": 32, "uniform": "sides"}},
        }))
        self.assertEqual(uncapped.get("sides"), 32)
        collapsed = parity._variant_candidates("synth/warp", {
            "domain": "image", "kind": "generator", "namespace": "synth", "func": "warp",
            "paramNames": ["warpIterations"],
            "params": {"warpIterations": {"type": "int", "default": 0, "min": 0, "max": 4, "uniform": "warpIterations"}},
        })
        self.assertEqual(collapsed, [("warpIterations", 1)])

    def test_boolean_color_and_vec3_candidates_differ_from_the_default(self):
        self.assertIs(parity._sweep_value({"type": "boolean", "default": False}), True)
        self.assertEqual(parity._sweep_value({"type": "color", "default": [1, 0, 0]}), [0, 1, 1])
        self.assertEqual(parity._sweep_value({"type": "color", "default": [1, 0, 0, 1]}), [0, 1, 1, 0.5])
        spec = {"type": "vec3", "default": [0.5, 0.5, 1], "min": [-1, -1, -1], "max": [1, 1, 1]}
        self.assertEqual(parity._sweep_value(spec), [1, 0, 0.5])
        per_component = {"type": "vec3", "default": [0, 0, 0], "min": [0, -1, 0], "max": [0.6, 1, 0.4]}
        self.assertEqual(parity._sweep_value(per_component), [0.6, 0, 0.4])

    def test_string_and_member_choices_skip_the_default_and_whitespace(self):
        spec = {
            "type": "string", "default": "Nunito",
            "choices": {"nunito": "Nunito", "serif": "serif", "text": "Hello World"},
        }
        self.assertEqual(parity._sweep_value(spec), "serif")
        member = {"type": "member", "default": "oscType.sine", "choices": {"sine": "oscType.sine", "square": "oscType.square"}}
        self.assertEqual(parity._sweep_value(member), "oscType.square")

    def test_resource_and_compile_time_parameters_have_no_candidate(self):
        self.assertIsNone(parity._sweep_value({"type": "surface", "default": "inputTex"}))
        self.assertIsNone(parity._sweep_value({"type": "palette", "default": 32}))
        self.assertIsNone(parity._sweep_value({"type": "mat3", "default": [1, 0, 0, 0, 1, 0, 0, 0, 1]}))
        self.assertIsNone(parity._sweep_value({"type": "boolean", "default": False, "define": "RIDGES"}))

    def test_variants_are_bounded_and_skip_structural_arguments(self):
        catalog = parity._catalog()
        for effect_id, effect in sorted(catalog.items()):
            candidates = parity._variant_candidates(effect_id, effect)
            self.assertLessEqual(len(candidates), parity.SWEEP_VARIANTS, effect_id)
            base = parity._arguments(effect)
            for name, value in candidates:
                self.assertNotIn(name, base, effect_id)
                self.assertNotEqual(parity._literal(value), parity._literal(effect["params"][name].get("default")), effect_id)
                self.assertFalse(
                    any(character.isspace() for character in parity._literal(value)),
                    f"{effect_id}:{name}",
                )


class SweepPlanTest(unittest.TestCase):
    def _catalog(self):
        return {
            "synth/alpha": {
                "domain": "image", "kind": "generator", "namespace": "synth", "func": "alpha",
                "paramNames": ["alpha", "seed"],
                "params": {
                    "alpha": {"type": "float", "default": 0.5, "min": 0, "max": 1, "uniform": "alpha"},
                    "seed": {"type": "int", "default": 0, "min": 0, "max": 100, "uniform": "seed"},
                },
            },
            "filter/none": {
                "domain": "image", "kind": "filter", "namespace": "filter", "func": "none",
                "paramNames": [], "params": {},
            },
            "points/life": {
                "domain": "image", "kind": "filter", "namespace": "points", "func": "life",
                "iterated": True,
                "paramNames": ["stateSize", "symmetricForces"],
                "params": {
                    "stateSize": {"type": "int", "default": 64, "uniform": "stateSize"},
                    "symmetricForces": {"type": "boolean", "default": False, "uniform": "symmetricForces"},
                },
            },
        }

    def test_plan_has_a_default_case_for_every_effect(self):
        plan = parity._sweep_plan(self._catalog())
        self.assertEqual(sorted(plan), ["filter/none", "points/life", "synth/alpha"])
        for cases in plan.values():
            self.assertEqual(cases[0]["kind"], "default")

    def test_plan_binds_bounded_non_default_variants(self):
        plan = parity._sweep_plan(self._catalog())
        self.assertEqual(
            [case["id"] for case in plan["synth/alpha"]],
            ["synth/alpha", "synth/alpha#param:alpha", "synth/alpha#param:seed"],
        )
        variants = [case for case in plan["synth/alpha"] if case["kind"] == "param"]
        self.assertEqual([(case["param"], case["value"]) for case in variants], [("alpha", 1), ("seed", 100)])

    def test_plan_adds_multi_frame_cases_only_to_stateful_effects(self):
        plan = parity._sweep_plan(self._catalog())
        frames = [case for case in plan["points/life"] if case["kind"] == "frame"]
        self.assertEqual(len(frames), parity.SWEEP_FRAMES)
        self.assertEqual([case["time"] for case in frames], list(parity.SWEEP_TIMES))
        self.assertFalse(any(case["kind"] == "frame" for case in plan["synth/alpha"]))

    def test_full_catalog_plan_ids_are_unique_and_sorted(self):
        plan = parity._sweep_plan(parity._catalog())
        ids = [case["id"] for effect_id in sorted(plan) for case in plan[effect_id]]
        self.assertEqual(ids, sorted(set(ids)))

    def test_sweep_programs_bind_overrides_on_both_cli_paths(self):
        catalog = parity._catalog()
        program = parity._program("synth/gradient", catalog["synth/gradient"], {"color1": [0, 1, 1]})
        self.assertIn("gradient(color1:[0,1,1]", program)
        external = parity._external_program("synth/roll", catalog["synth/roll"], {"gain": 5})
        self.assertIn("roll(gain:5)", external)
        plain = parity._external_program("synth/roll", catalog["synth/roll"], {})
        self.assertIn("roll()", plain)


class SweepValidationTest(unittest.TestCase):
    def _plan(self):
        catalog = {
            "synth/alpha": {
                "domain": "image", "kind": "generator", "namespace": "synth", "func": "alpha",
                "paramNames": ["alpha"],
                "params": {"alpha": {"type": "float", "default": 0.5, "min": 0, "max": 1, "uniform": "alpha"}},
            },
            "points/life": {
                "domain": "image", "kind": "filter", "namespace": "points", "func": "life",
                "paramNames": ["stateSize"],
                "params": {"stateSize": {"type": "int", "default": 64, "uniform": "stateSize"}},
            },
        }
        return parity._sweep_plan(catalog)

    def _results(self, plan):
        results = []
        for effect_id in sorted(plan):
            for case in plan[effect_id]:
                results.append({
                    "id": case["id"], "effect": effect_id,
                    "case": parity._sweep_case_dict(case),
                    "status": "compared", **parity._metrics(bytes(4), bytes(4)),
                })
        return results

    def test_full_plan_validates_with_exact_denominators(self):
        plan = self._plan()
        results = self._results(plan)
        report = parity._summarize_sweep(results, plan, 1, 0.25, 1)
        parity._validate_sweep(report, plan)
        self.assertEqual(report["expected"], len(results))
        self.assertEqual(report["executed"], len(results))
        self.assertEqual(report["byte_exact"], len(results))
        self.assertEqual(report["failed"], 0)
        self.assertEqual(report["tolerance"], 0)
        self.assertEqual(report["sweep"]["variants"], parity.SWEEP_VARIANTS)
        self.assertEqual(report["sweep"]["frames"], parity.SWEEP_FRAMES)
        self.assertEqual(report["sweep"]["times"], list(parity.SWEEP_TIMES))

    def test_a_byte_difference_fails_at_tolerance_zero(self):
        plan = self._plan()
        results = self._results(plan)
        results[1]["max_delta"] = 1
        results[1]["differing_channels"] = 1
        results[1]["pass"] = False
        report = parity._summarize_sweep(results, plan, 1, 0.25, 1)
        with self.assertRaisesRegex(ValueError, "unapproved delta"):
            parity._validate_sweep(report, plan)

    def test_a_missing_case_breaks_the_denominator(self):
        plan = self._plan()
        results = self._results(plan)[1:]
        report = parity._summarize_sweep(results, plan, 1, 0.25, 1)
        with self.assertRaisesRegex(ValueError, "sweep denominator"):
            parity._validate_sweep(report, plan)

    def test_an_error_record_counts_as_failed(self):
        plan = self._plan()
        results = self._results(plan)
        results[0].update({"status": "error", "reason": "timeout after 30s"})
        for key in ("max_delta", "mean_delta", "differing_channels", "channels_over_2", "pass"):
            results[0].pop(key)
        report = parity._summarize_sweep(results, plan, 1, 0.25, 1)
        with self.assertRaisesRegex(ValueError, "sweep contains 1 failure"):
            parity._validate_sweep(report, plan)
        self.assertEqual(report["failed"], 1)
        self.assertEqual(report["executed"], len(results) - 1)


if __name__ == "__main__":
    unittest.main()

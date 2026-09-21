import json
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parent.parent.parent


class ExportKitTest(unittest.TestCase):
    def test_export_kit_config_is_valid_and_matches_catalog(self):
        config_path = ROOT / "export-kit" / "kit.config.json"
        self.assertTrue(config_path.is_file(), "missing export-kit/kit.config.json")
        with open(config_path, "r", encoding="utf-8") as f:
            config = json.load(f)

        self.assertEqual(config.get("id"), "rust")
        self.assertEqual(config.get("compat", {}).get("mode"), "list")

        metadata_rel = config.get("compat", {}).get("fromBundleMetadata")
        self.assertIsNotNone(metadata_rel, "compat.fromBundleMetadata missing")

        metadata_path = ROOT / metadata_rel
        self.assertTrue(metadata_path.is_file(), f"missing {metadata_rel}")

        with open(metadata_path, "r", encoding="utf-8") as f:
            metadata = json.load(f)

        effects = metadata.get("effects", {})
        self.assertEqual(len(effects), 205, "expected 205 catalog effects")
        for effect_id in [
            "points/heightGrid",
            "render/renderLandscape3d",
            "synth3d/heightmap3d",
        ]:
            self.assertIn(effect_id, effects, f"expected {effect_id} in catalog")
        for effect_id, effect in effects.items():
            self.assertTrue(effect.get("func"), f"effect {effect_id} should declare a non-empty func")
            self.assertTrue(effect.get("domain"), f"effect {effect_id} should declare a non-empty domain")


if __name__ == "__main__":
    unittest.main()

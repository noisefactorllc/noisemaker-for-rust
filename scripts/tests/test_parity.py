import unittest

from scripts import parity


class StrictParityTest(unittest.TestCase):
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


if __name__ == "__main__":
    unittest.main()

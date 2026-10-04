import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

SCRIPT = Path(__file__).with_name("update_readme.py")


class ReadmeResultsTests(unittest.TestCase):
    def run_generator(self, data):
        directory = tempfile.TemporaryDirectory()
        self.addCleanup(directory.cleanup)
        root = Path(directory.name)
        results = root / "results.json"
        readme = root / "README.md"
        results.write_text(json.dumps(data), encoding="utf-8")
        readme.write_text("<!-- BENCHMARK_RESULTS_START -->\nold\n<!-- BENCHMARK_RESULTS_END -->\n", encoding="utf-8")
        subprocess.run([sys.executable, str(SCRIPT), str(results), "--readme", str(readme)], check=True)
        return readme.read_text(encoding="utf-8")

    def test_empty_results_leave_metrics_unavailable(self):
        output = self.run_generator({"results": []})
        self.assertIn("| Parakeet | — | — | — | — | — |", output)

    def test_wer_is_weighted_by_reference_word_count(self):
        results = [
            {"status": "ok", "metrics": {"backend": "sherpa-onnx Parakeet", "accuracy": {"substitutions": 1, "insertions": 0, "deletions": 0, "reference_words": 1}}},
            {"status": "ok", "metrics": {"backend": "Parakeet", "accuracy": {"substitutions": 0, "insertions": 0, "deletions": 0, "reference_words": 9}}},
        ]
        output = self.run_generator({"validation_status": "passed", "results": results})
        self.assertIn("| Parakeet | 10.0% |", output)

    def test_unvalidated_scores_are_not_inserted_into_readme(self):
        results = [{"status": "ok", "metrics": {"backend": "Parakeet", "accuracy": {"substitutions": 0, "insertions": 0, "deletions": 0, "reference_words": 1}}}]
        output = self.run_generator({"validation_status": "pending", "results": results})
        self.assertIn("| Parakeet | — |", output)

    def test_validated_summary_artifact_publishes_its_engine_metrics_and_failures(self):
        data = {
            "validation_status": "validated_with_case_failures",
            "corpus": "pinned test corpus",
            "cases_per_engine": 10,
            "engine_summaries": {
                "Native": {
                    "counts": {"total": 10, "successful": 8, "failed": 2, "unsupported_locale": 1},
                    "metrics": {"wer": 0.2, "cer": 0.1, "first_partial_ms": 100, "finalization_latency_ms": 200, "rtf": 0.4, "average_cpu_percent": 12.0, "peak_cpu_percent": 25.0, "average_rss_bytes": 100 * 1024 * 1024, "peak_rss_bytes": 120 * 1024 * 1024, "model_load_ms": 50, "partial_count": 3, "partial_stability": 0.9, "dropped_audio": 0, "deadline_misses": 1},
                }
            },
            "environment": {"os": "macOS", "architecture": "Apple Silicon", "logical_cpu_count": 12},
            "dataset_integrity": "verified",
            "full_artifacts_location": "local run output",
        }
        output = self.run_generator(data)
        self.assertIn("| Native Apple | 20.0% | 10.0% | 100 ms | 200 ms | 0.400 | 120 MB* |", output)
        self.assertNotIn("Successful", output)
        self.assertNotIn("unsupported locale", output)


if __name__ == "__main__":
    unittest.main()

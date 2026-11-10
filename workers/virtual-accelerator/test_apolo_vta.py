import unittest
from apolo_vta import assess, estimate_capacity


class VtaPlannerTests(unittest.TestCase):
    def test_no_workers_means_zero_capacity(self):
        result = estimate_capacity([])
        self.assertEqual(result["active_workers"], 0)
        self.assertEqual(result["estimated_gpu_parameters_bf16_adamw"], 0)

    def test_gpu_capacity_estimate(self):
        workers = [{
            "status": "online", "last_seen_at": "now", "gpu_free_memory_bytes": 16 * 1024**3,
            "gpu_name": "Example GPU", "ram_available_bytes": 8 * 1024**3
        }]
        result = estimate_capacity(workers)
        self.assertEqual(result["gpu_workers"], 1)
        self.assertGreater(result["estimated_gpu_parameters_bf16_adamw"], 0)

    def test_googolplex_is_rejected(self):
        result = assess("100 googolplex", estimate_capacity([]))
        self.assertFalse(result["feasible"])

    def test_centillion_is_rejected(self):
        result = assess("centilhão", estimate_capacity([]))
        self.assertFalse(result["feasible"])

    def test_100_novemdecillion_is_rejected(self):
        result = assess("100 novemdecillion", estimate_capacity([]))
        self.assertFalse(result["feasible"])
        self.assertIn("capacidade física", result["reason"])


if __name__ == "__main__":
    unittest.main()

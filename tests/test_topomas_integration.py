import unittest
import sys
import os

sys.path.append(os.path.abspath(os.path.join(os.path.dirname(__file__), '../src/topomas')))

class TestTopoMASIntegration(unittest.TestCase):
    def test_imports(self):
        try:
            import topomas_v9_2
            import space_scorer
            import digital_twin_agent
            self.assertTrue(True)
        except Exception as e:
            self.fail(f"Import failed: {e}")

if __name__ == '__main__':
    unittest.main()

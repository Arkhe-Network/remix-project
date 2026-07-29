from cathedral.client import CathedralSdk
import unittest

class TestCathedralSdk(unittest.TestCase):
    def test_init(self):
        sdk = CathedralSdk()
        self.assertIsNotNone(sdk)

if __name__ == '__main__':
    unittest.main()

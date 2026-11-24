import subprocess
import sys
import unittest
from pathlib import Path


class ShoutTest(unittest.TestCase):
    def test_upper(self):
        script = Path(__file__).parent.parent / "shout.py"
        out = subprocess.run([sys.executable, str(script)], input="a", capture_output=True, text=True)
        self.assertEqual(out.stdout, "A")

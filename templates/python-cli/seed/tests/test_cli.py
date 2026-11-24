import contextlib
import io
import unittest

from {{snake}} import __version__
from {{snake}}.cli import main


class CliTest(unittest.TestCase):
    def test_version(self) -> None:
        out = io.StringIO()
        with contextlib.redirect_stdout(out), self.assertRaises(SystemExit) as done:
            main(["--version"])
        self.assertEqual(done.exception.code, 0)
        self.assertIn(__version__, out.getvalue())


if __name__ == "__main__":
    unittest.main()

"""`{{command_synopsis}}`: {{command_about}}

Yours: anything-anything never overwrites this file.

args:
{{command_args_md}}
"""

from types import SimpleNamespace

from ..output import Output


def run(args: SimpleNamespace) -> Output:
    # ANYTHING:BUILD replace this with the real {{command}}
    raise NotImplementedError(f"`{{command}}` isn't built yet ({vars(args)}): run anything-anything build")

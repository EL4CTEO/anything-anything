from pathlib import Path
from types import SimpleNamespace

from ..output import Output


def run(args: SimpleNamespace) -> Output:
    files, lines_out, total = [], [], 0
    for path in args.files:
        try:
            lines = len(Path(path).read_text(encoding="utf-8").splitlines())
        except OSError as e:
            raise RuntimeError(f"can't read {path}") from e
        if lines < args.min:
            continue
        total += lines
        lines_out.append(f"{path}: {lines}")
        files.append({"path": path, "lines": lines})
    lines_out.append(f"total: {total}")
    return Output("\n".join(lines_out), {"files": files, "total": total})

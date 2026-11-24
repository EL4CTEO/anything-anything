from types import SimpleNamespace

from ..output import Output


def run(args: SimpleNamespace) -> Output:
    greeting = f"Hello, {args.who}!"
    if args.shout:
        greeting = greeting.upper()
    return Output(greeting, {"greeting": greeting})

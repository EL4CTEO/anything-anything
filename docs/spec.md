# anything.toml

The one file you write. Everything else is generated from it, built against it, or checked by it.

```toml
name = "todo-anything"                 # lower-kebab, ends in -anything
pitch = "Find every TODO, FIXME and HACK in any codebase, in any language."   # one sentence, used everywhere
tagline = "Every TODO, everywhere."     # optional, the README headline
when = "Use when the user asks what's left to do in a codebase."              # optional, for agents' skill matching
version = "0.1.0"
stack = "rust-cli"                     # rust-cli, python-cli, node-cli, web, any  (anything-anything stacks)
owner = "your-github-name"             # GitHub user or org; filled in from gh when you run new
npm = "@you/todo-anything"             # optional, the npm package (default @<owner>/<name>)
color = "#d97706"                      # optional, #rrggbb for the logo and badges
keywords = ["todo", "fixme"]
notes = """
How it should work, for whoever builds it: the approach, the edge cases, what's out of scope.
"""
```

## [[command]] (CLI stacks)

Each command becomes a subcommand, an MCP tool (`todo_scan`), a line in the README and the skill, and a file that's yours to write.

```toml
[[command]]
name = "scan"                          # lower-kebab; not mcp/help, not a keyword in Rust, Python or JavaScript
about = "List every TODO under a folder"

[[command.arg]]
name = "path"
kind = "path"                          # string, path, int, float, bool (a flag), list (comma-separated)
help = "Folder to scan"
positional = true                      # todo-anything scan src  instead of  --path src
default = "."                          # as typed on the command line
[[command.arg]]
name = "tag"
kind = "list"
default = "TODO,FIXME,HACK"
[[command.arg]]
name = "max"
kind = "int"
required = true                        # no default; positional ones can't follow an optional one
```

Every command gets `--json` for free: it prints the command's data as JSON. Every CLI gets `mcp`: it runs as an MCP server.

## [[example]]: the proof

Every command needs at least one. Each runs in a fresh, empty folder that holds only its `files`. Exactly one of `args`, `sh`, `http`.

```toml
[[example]]
name = "finds a todo in python"
args = ["scan", "."]                   # run the CLI with these
files = { "app.py" = "x = 1  # TODO: rename\n" }
stdin = "..."                          # optional
exit = 0                               # default 0
stdout = ["app.py:1", "rename"]        # each must appear in the output
not_stdout = ["FIXME"]                 # none may appear
json = { "/count" = 1, "/items/0/tag" = "TODO" }   # JSON pointers into the output of the same args + --json
files_after = { "out.md" = "# TODOs" } # files that must exist afterwards, containing this text
timeout = 60                           # seconds

[[example]]
name = "the page has a title"
http = "GET /"                         # a request to [run] serve (web); POST/PUT/PATCH take body = "..."
status = 200
stdout = ["<title>"]                   # checked on the response body; json too

[[example]]
name = "game logic tests"
sh = "node --test"                     # any shell line, run in the example folder; {root} is the project
stdout = ["pass"]
```

Output is compared with `\r\n` as `\n` and `\\` as `/`, so the same example passes on Windows.

## [run] and [[step]]: override the stack, or define the any stack

Every stack has its own; set these to change them. The `any` stack has none, so it needs both.
`{root}` is the project folder, `{exe}` is `.exe` on Windows, `{python}` the Python on PATH, `{port}` the server's port.

```toml
[run]
cmd = ["go", "run", "{root}"]          # the CLI that example args go after
serve = ["npx", "vite", "preview", "--port", "{port}", "--strictPort"]   # a server for http examples, also given $PORT
ready = "/"                            # answers once the server is up
env = { GOFLAGS = "-mod=mod" }

[[step]]                               # verify runs these in order; a failed one stops the rest unless soft
name = "build"
run = ["go", "build", "./..."]
probe = ["go", "version"]              # skip the step when this doesn't run
fix = false                            # a formatter: it never fails
soft = false                           # a linter: when it fails, the next steps still run
timeout = 900
env = {}
```

## verify

`anything-anything verify` runs every gate, in order: **spec** (this file is valid), **files** (the generated files are as generated),
**stubs** (nothing is left unbuilt), the stack's **steps** (format, build, lint, test), **mcp** (the server answers and lists every command),
and **examples**. It passes when every gate does.

---
name: anything-anything
description: Make a new tool, app, game or library from a one-line idea, in any stack (Rust, Python or TypeScript CLIs with an MCP server, Vite web apps and games, or any other language), and prove it works with its own executable examples. Use when the user wants to create, scaffold or build "a tool that...", an "-anything" project, a CLI, a small web app or game, or wants something built and verified end to end.
---

# anything-anything

`anything-anything` turns a spec (`anything.toml`) into a project and judges it: you write the code, `verify` says whether it works.
The same steps are MCP tools: `anything_spec_format`, `anything_create`, `anything_task`, `anything_sync`, `anything_verify`, `anything_status`, `anything_stacks`, `anything_hub`.

## Loop

1. **Stack**: `anything-anything stacks`. A CLI in the language its users install with (`rust-cli`, `python-cli`, `node-cli`),
   `web` for anything people use in a browser (apps, games), `any` for everything else (then you write its [[step]]s and [run]).
2. **Spec**: `anything-anything spec` shows the format. Write `anything.toml`: a one-line pitch, the commands, `notes` on how it works,
   and examples that check real behaviour with exact output (`stdout`, `json`, `files_after`, `http`). At least two per command.
3. **Create**: `anything-anything new <what> --spec anything.toml` (or `--stack <id>` and an idea). It generates the project and `.anything/TASK.md`.
4. **Build**: do what `.anything/TASK.md` says. Write code only in the files that are yours (it lists the generated ones).
5. **Verify** after each piece: `anything-anything verify`. Fix what fails. Done when every gate passes.
6. **Publish** only when the user says so: `anything-anything publish` shows the plan, `--yes` does it.

Or hand the whole thing to another agent: `anything-anything new <what> "<idea>" --agent claude`.

## Don't

- Don't edit `anything.toml` or the generated files to make a gate pass. If an example is wrong, fix the spec with the user, then `anything-anything sync`.
- Don't leave stubs: `ANYTHING:BUILD`, `todo!()`, `NotImplementedError` fail verify.
- Don't publish without asking: it creates a public repository.

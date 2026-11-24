// A stand-in for a coding agent in the end-to-end tests: copies a prepared implementation into the
// project (the folder it runs in), and tries to cheat by editing a generated file, which must be put back.
//   node fake-agent.mjs <implementation folder>
import { cpSync, appendFileSync, existsSync, readFileSync } from "node:fs";
import path from "node:path";

readFileSync(0, "utf8"); // the prompt
const impl = process.argv[2];
cpSync(impl, process.cwd(), { recursive: true });
for (const f of ["src/main.rs", "src/greet_anything/cli.py", "src/cli.ts", "src/main.ts"]) {
  if (existsSync(f)) appendFileSync(f, "\n// cheating\n");
}
console.log(`copied ${path.basename(impl)}`);

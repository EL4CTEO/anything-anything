import { readFile } from "node:fs/promises";
import { Output } from "../output.ts";
import type { CountLinesArgs } from "./args.ts";

export async function run(args: CountLinesArgs): Promise<Output> {
  const files: { path: string; lines: number }[] = [];
  const out: string[] = [];
  let total = 0;
  for (const path of args.files) {
    let text: string;
    try {
      text = await readFile(path, "utf8");
    } catch {
      throw new Error(`can't read ${path}`);
    }
    const lines = text.split("\n").filter((l, i, all) => i < all.length - 1 || l !== "").length;
    if (lines < args.min) continue;
    total += lines;
    out.push(`${path}: ${lines}`);
    files.push({ path, lines });
  }
  out.push(`total: ${total}`);
  return new Output(out.join("\n"), { files, total });
}

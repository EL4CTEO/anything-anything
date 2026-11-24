import { Output } from "../output.ts";
import type { HelloArgs } from "./args.ts";

export async function run(args: HelloArgs): Promise<Output> {
  let greeting = `Hello, ${args.who}!`;
  if (args.shout) greeting = greeting.toUpperCase();
  return new Output(greeting, { greeting });
}

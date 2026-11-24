// `{{command_synopsis}}`: {{command_about}}
// Yours: anything-anything never overwrites this file.

import { Output } from "../output.ts";
import type { {{command_pascal}}Args } from "./args.ts";

export async function run(args: {{command_pascal}}Args): Promise<Output> {
  // ANYTHING:BUILD replace this with the real {{command}}
  throw new Error(`\`{{command}}\` isn't built yet (${JSON.stringify(args)}): run anything-anything build`);
}

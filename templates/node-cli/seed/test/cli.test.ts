import assert from "node:assert/strict";
import { test } from "node:test";
import { main } from "../src/cli.ts";

test("--version answers", async () => {
  assert.equal(await main(["--version"]), 0);
});

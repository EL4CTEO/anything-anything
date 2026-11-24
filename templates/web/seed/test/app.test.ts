import assert from "node:assert/strict";
import { test } from "node:test";
import { title } from "../src/app.ts";

test("it has a title", () => {
  assert.ok(title().length > 0);
});

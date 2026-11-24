import assert from "node:assert/strict";
import { test } from "node:test";
import { roll, title } from "../src/app.ts";

test("title", () => assert.equal(title(), "Dice Anything"));
test("roll stays in range", () => {
  assert.equal(roll(6, () => 0), 1);
  assert.equal(roll(6, () => 0.999), 6);
});

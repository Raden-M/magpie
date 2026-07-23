// Parity tests for the shared normalizer. These mirror the Rust unit tests in
// backend/src/reader/normalize.rs case-for-case: if this file and that file
// agree, the capture/render round-trip holds.

import { test } from "node:test";
import assert from "node:assert/strict";
import { normalizePieces, normalizeStr } from "../src/shared/normalize.ts";

test("collapses whitespace and trims", () => {
  const n = normalizePieces(["  the   quick\n\tbrown  "]);
  assert.equal(n.text, "the quick brown");
  assert.equal(n.map.length, Array.from(n.text).length);
});

test("folds smart punctuation one-to-one", () => {
  const n = normalizePieces(["“don’t”—really"]);
  assert.equal(n.text, '"don\'t"-really');
});

test("joins pieces with a space when whitespace between", () => {
  const n = normalizePieces(["foo ", "bar"]);
  assert.equal(n.text, "foo bar");
  const bIndex = Array.from(n.text).indexOf("b");
  assert.deepEqual(n.map[bIndex], { piece: 1, offset: 0 });
});

test("map points at source offsets", () => {
  const n = normalizePieces(["Hello  world"]);
  assert.equal(n.text, "Hello world");
  const wIndex = Array.from(n.text).indexOf("w");
  assert.deepEqual(n.map[wIndex], { piece: 0, offset: 7 });
});

test("normalizeStr matches piece form", () => {
  assert.equal(normalizeStr("  a\tb  "), "a b");
});

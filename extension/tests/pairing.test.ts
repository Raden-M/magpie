// Pairing input: what the options page accepts in the backend URL field, and
// the host permission it asks for as a result.

import { test } from "node:test";
import assert from "node:assert/strict";
import { originPatternFor, parseConnectString } from "../src/shared/pairing.ts";

test("a plain URL parses to itself, with no token", () => {
  const { backendUrl, token } = parseConnectString("  http://127.0.0.1:8787/  ");
  assert.equal(backendUrl, "http://127.0.0.1:8787");
  assert.equal(token, undefined);
});

test("a connect string splits into URL and token", () => {
  const { backendUrl, token } = parseConnectString("https://magpie.example.com#s3cret");
  assert.equal(backendUrl, "https://magpie.example.com");
  assert.equal(token, "s3cret");
});

test("an empty fragment carries no token", () => {
  assert.equal(parseConnectString("http://127.0.0.1:8787#").token, undefined);
});

test("the origin pattern covers the whole backend, port included", () => {
  assert.equal(originPatternFor("http://127.0.0.1:8787"), "http://127.0.0.1:8787/*");
  assert.equal(originPatternFor("https://magpie.example.com/"), "https://magpie.example.com/*");
});

test("anything unparseable or non-http asks for no permission", () => {
  for (const value of ["", "not a url", "ftp://example.com", "javascript:alert(1)"]) {
    assert.equal(originPatternFor(value), null, value);
  }
});

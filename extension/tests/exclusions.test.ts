// Excluded sites: what the options page accepts, and which pages a stored
// entry then covers.

import { test } from "node:test";
import assert from "node:assert/strict";
import {
  exclusionFor,
  formatExclusions,
  isExcluded,
  normalizeExclusion,
  parseExclusions,
  withoutExclusionsFor,
} from "../src/shared/exclusions.ts";

test("a bare host matches that host, and only that host", () => {
  assert.equal(isExcluded("https://mail.google.com/u/0", ["mail.google.com"]), true);
  assert.equal(isExcluded("http://mail.google.com/", ["mail.google.com"]), true);
  assert.equal(isExcluded("https://google.com/", ["mail.google.com"]), false);
  assert.equal(isExcluded("https://drive.google.com/", ["mail.google.com"]), false);
  // A host is a label boundary, not a substring: this is the one that bites.
  assert.equal(isExcluded("https://notmail.google.com/", ["mail.google.com"]), false);
});

test("a *. entry covers the domain and its subdomains", () => {
  assert.equal(isExcluded("https://example.com/", ["*.example.com"]), true);
  assert.equal(isExcluded("https://app.example.com/", ["*.example.com"]), true);
  assert.equal(isExcluded("https://a.b.example.com/", ["*.example.com"]), true);
  assert.equal(isExcluded("https://badexample.com/", ["*.example.com"]), false);
});

test("a path narrows an entry to one app on a shared host", () => {
  const list = ["example.com/app"];
  assert.equal(isExcluded("https://example.com/app", list), true);
  assert.equal(isExcluded("https://example.com/app/inbox/2", list), true);
  assert.equal(isExcluded("https://example.com/", list), false);
  assert.equal(isExcluded("https://example.com/apple", list), false);
  assert.equal(isExcluded("https://example.com/blog/app", list), false);
});

test("a port is only compared when the entry names one", () => {
  assert.equal(isExcluded("http://localhost:3000/", ["localhost:3000"]), true);
  assert.equal(isExcluded("http://localhost:8080/", ["localhost:3000"]), false);
  assert.equal(isExcluded("http://localhost:8080/", ["localhost"]), true);
  // The default port is absent from a URL's host, so an entry must be too.
  assert.equal(isExcluded("https://example.com/", ["example.com:443"]), false);
});

test("the query string and fragment never decide a match", () => {
  assert.equal(isExcluded("https://example.com/app?utm_source=x#top", ["example.com/app"]), true);
});

test("only http(s) pages can be excluded", () => {
  for (const url of ["about:blank", "file:///home/x/notes.html", "not a url"]) {
    assert.equal(isExcluded(url, ["*.example.com", "example.com"]), false);
  }
});

test("input is tidied into a stored entry, or dropped", () => {
  // The host folds to lowercase; the path keeps its case, because URL paths are
  // case-sensitive and a site with /App would stop matching if we folded it.
  assert.equal(normalizeExclusion("  HTTPS://Example.COM/App/?q=1#top  "), "example.com/App");
  assert.equal(normalizeExclusion("example.com/"), "example.com");
  assert.equal(normalizeExclusion("//example.com"), "example.com");
  for (const junk of ["", "   ", "# a comment", "*", ".example.com", "exa*mple.com"]) {
    assert.equal(normalizeExclusion(junk), null, junk);
  }
});

test("a pasted URL still excludes the page it came from", () => {
  const entry = normalizeExclusion("https://mail.google.com/u/0/#inbox");
  assert.equal(isExcluded("https://mail.google.com/u/0/", [entry!]), true);
});

test("the textarea round-trips: blank and junk lines drop, duplicates collapse", () => {
  const entries = parseExclusions("example.com\n\n  EXAMPLE.com/  \n# note\n*.figma.com\n");
  assert.deepEqual(entries, ["example.com", "*.figma.com"]);
  assert.equal(formatExclusions(entries), "example.com\n*.figma.com");
  assert.deepEqual(parseExclusions(formatExclusions(entries)), entries);
});

test("the context menu adds a host-wide entry, port included when it isn't the default", () => {
  assert.equal(exclusionFor("https://Mail.Google.com/u/0/#inbox"), "mail.google.com");
  assert.equal(exclusionFor("http://localhost:3000/app"), "localhost:3000");
  assert.equal(exclusionFor("https://example.com:443/"), "example.com");
  assert.equal(exclusionFor("about:blank"), null);
});

test("re-enabling a site drops every entry covering it, wildcards included", () => {
  const list = ["*.example.com", "example.com/app", "other.com"];
  assert.deepEqual(withoutExclusionsFor("https://example.com/app/x", list), ["other.com"]);
  // Entries that don't cover this page survive - the wildcard is dropped, the
  // path entry for a section the user isn't on is not.
  assert.deepEqual(withoutExclusionsFor("https://example.com/blog", list), [
    "example.com/app",
    "other.com",
  ]);
});

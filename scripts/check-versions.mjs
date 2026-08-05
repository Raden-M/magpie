// Assert the version is declared identically everywhere, and matches the tag.
//
// The version lives in three files that nothing keeps in step: a release whose
// binary says 0.2.0 and whose extension says 0.1.0 is indistinguishable from a
// working one until someone tries to debug it. Run first in the release
// workflow, so a mismatched tag fails before anything is published.
//
//   node scripts/check-versions.mjs           # just check they agree
//   node scripts/check-versions.mjs v0.2.0    # ...and that they match the tag

import { readFile } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");

async function read(path) {
  return readFile(resolve(root, path), "utf8");
}

const cargo = (await read("backend/Cargo.toml")).match(/^version\s*=\s*"([^"]+)"/m);
if (!cargo) throw new Error("no version found in backend/Cargo.toml");

const sources = [
  { file: "backend/Cargo.toml", version: cargo[1] },
  { file: "extension/package.json", version: JSON.parse(await read("extension/package.json")).version },
  { file: "extension/manifest.json", version: JSON.parse(await read("extension/manifest.json")).version },
];

const tag = process.argv[2];
if (tag) sources.push({ file: "git tag", version: tag.replace(/^v/, "") });

const distinct = [...new Set(sources.map((s) => s.version))];
for (const s of sources) console.log(`  ${s.version.padEnd(12)} ${s.file}`);

if (distinct.length !== 1) {
  console.error(`\nversions disagree: ${distinct.join(", ")}`);
  process.exit(1);
}
console.log(`\nversion ${distinct[0]} is consistent`);

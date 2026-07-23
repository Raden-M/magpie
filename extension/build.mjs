import { build, context } from "esbuild";
import { cp, mkdir, readFile, rm, writeFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { dirname, resolve } from "node:path";

const root = dirname(fileURLToPath(import.meta.url));
const watch = process.argv.includes("--watch");
// `manifest.json` is the canonical (Chromium) manifest; the Firefox variant is
// derived from it so there is a single source of truth.
const firefox = process.argv.includes("--firefox");
const outdir = resolve(root, firefox ? "dist-firefox" : "dist");

/** Each MV3 context is bundled as its own IIFE - no shared runtime chunks. */
const entryPoints = {
  background: resolve(root, "src/background/index.ts"),
  content: resolve(root, "src/content/index.ts"),
  options: resolve(root, "src/options/options.ts"),
};

const buildOptions = {
  entryPoints,
  outdir,
  bundle: true,
  format: "iife",
  target: "es2022",
  platform: "browser",
  sourcemap: watch ? "inline" : false,
  minify: !watch,
  logLevel: "info",
};

/** Derive the Firefox manifest from the canonical Chromium one. */
function toFirefox(manifest) {
  const m = structuredClone(manifest);
  // Firefox MV3 uses a non-persistent event page, not a service worker.
  m.background = { scripts: ["background.js"] };
  // Firefox requires an explicit add-on id and a minimum version.
  m.browser_specific_settings = {
    gecko: { id: "magpie@raden.me", strict_min_version: "115.0" },
  };
  // Firefox expects options under options_ui.
  if (m.options_page) {
    m.options_ui = { page: m.options_page, open_in_tab: true };
    delete m.options_page;
  }
  return m;
}

async function writeManifest() {
  const source = JSON.parse(await readFile(resolve(root, "manifest.json"), "utf8"));
  const manifest = firefox ? toFirefox(source) : source;
  await writeFile(resolve(outdir, "manifest.json"), JSON.stringify(manifest, null, 2));
}

async function copyStatic() {
  await mkdir(outdir, { recursive: true });
  await cp(resolve(root, "src/options/options.html"), resolve(outdir, "options.html"));
  await writeManifest();
}

async function run() {
  await rm(outdir, { recursive: true, force: true });
  await copyStatic();

  if (watch) {
    const ctx = await context(buildOptions);
    await ctx.watch();
    console.log(`watching for changes -> ${firefox ? "dist-firefox" : "dist"}/`);
  } else {
    await build(buildOptions);
    console.log(`built extension -> ${firefox ? "dist-firefox" : "dist"}/`);
  }
}

run().catch((err) => {
  console.error(err);
  process.exit(1);
});

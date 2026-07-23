import { build, context } from "esbuild";
import { cp, mkdir, rm } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { dirname, resolve } from "node:path";

const root = dirname(fileURLToPath(import.meta.url));
const outdir = resolve(root, "dist");
const watch = process.argv.includes("--watch");

/** Each MV3 context is bundled as its own IIFE - no shared runtime chunks. */
const entryPoints = {
  background: resolve(root, "src/background/index.ts"),
  content: resolve(root, "src/content/index.ts"),
  options: resolve(root, "src/options/options.ts"),
};

/** Static files copied verbatim into the packaged extension. */
const staticFiles = [
  ["manifest.json", "manifest.json"],
  ["src/options/options.html", "options.html"],
];

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

async function copyStatic() {
  await mkdir(outdir, { recursive: true });
  for (const [from, to] of staticFiles) {
    await cp(resolve(root, from), resolve(outdir, to));
  }
}

async function run() {
  await rm(outdir, { recursive: true, force: true });
  await copyStatic();

  if (watch) {
    const ctx = await context(buildOptions);
    await ctx.watch();
    console.log("watching for changes...");
  } else {
    await build(buildOptions);
    console.log("built extension -> dist/");
  }
}

run().catch((err) => {
  console.error(err);
  process.exit(1);
});

// Rasterize the magpie mark into the PNG sizes Chromium's MV3 manifest requires.
//
// `src/icon/magpie.svg` is the bare mark - that is what the options page shows,
// on its own dark ground. A toolbar icon has no ground of its own, so the tile
// is composed here rather than being baked into the source: one artwork, used
// bare in the page and tiled in the manifest.
import { Resvg } from "@resvg/resvg-js";
import { readFile, writeFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { dirname, resolve } from "node:path";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const mark = await readFile(resolve(root, "src/icon/magpie.svg"), "utf8");

const SIZES = [16, 32, 48, 128];
const TILE = "#101116";
/** Fits the mark's 110x90 of artwork inside a 128 tile, optically centred. */
const PLACE = "translate(12,5.6) scale(0.8)";

/** The mark's own markup, lifted out of its `<svg>` so it can be re-wrapped. */
function artwork(svg) {
  const inner = svg.replace(/^[\s\S]*?<svg[^>]*>/, "").replace(/<\/svg>\s*$/, "");
  if (!inner.trim()) throw new Error("could not read the mark's artwork");
  return inner;
}

// Gradients inside the artwork are userSpaceOnUse, so they resolve in the
// transformed group's space - the same space they were drawn in.
const tiled = `<svg width="128" height="128" viewBox="0 0 128 128" fill="none" xmlns="http://www.w3.org/2000/svg">
  <rect width="128" height="128" rx="30" fill="${TILE}"/>
  <g transform="${PLACE}">${artwork(mark)}</g>
</svg>`;

for (const size of SIZES) {
  const png = new Resvg(tiled, { fitTo: { mode: "width", value: size } }).render().asPng();
  await writeFile(resolve(root, `src/icon/magpie-${size}.png`), png);
  console.log(`icon ${size}x${size} -> src/icon/magpie-${size}.png`);
}

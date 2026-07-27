/**
 * Generate Tauri app icons from BrandLogo.tsx (single source of truth).
 * Usage: node --experimental-strip-types tools/generate-app-icon.mts
 */
import { execFileSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { createRequire } from "node:module";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const root = path.resolve(__dirname, "..");
const brandLogoPath = path.join(root, "apps/desktop/src/components/BrandLogo.tsx");
const cssPath = path.join(root, "apps/desktop/src/App.css");
const iconsDir = path.join(root, "apps/desktop/src-tauri/icons");
const masterPng = path.join(iconsDir, "icon-master-1024.png");
const derivedSvg = path.join(iconsDir, "app-icon-from-brandlogo.svg");

function readCssVar(css: string, name: string): string {
  const re = new RegExp(`${name}:\\s*(#[0-9a-fA-F]{3,8})`);
  const m = css.match(re);
  if (!m) throw new Error(`CSS var ${name} not found in App.css`);
  return m[1];
}

function extractBrandSvg(tsx: string): string {
  const m = tsx.match(/<svg\b[^>]*viewBox="0 0 40 40"[^>]*>([\s\S]*?)<\/svg>/);
  if (!m) throw new Error("Could not find 40×40 brand <svg> in BrandLogo.tsx");
  return m[1].trim();
}

function toStandaloneSvg(inner: string, ink: string, accent: string, bg: string): string {
  // Resolve React/CSS: currentColor → ink, brand-logo-dot → accent fill
  let body = inner
    .replace(/\bstrokeWidth=/g, "stroke-width=")
    .replace(/\bclassName="/g, 'class="')
    .replace(/\bstroke="currentColor"/g, `stroke="${ink}"`)
    .replace(
      /<circle([^>]*?)\sclass="brand-logo-dot"\s*\/>/g,
      `<circle$1 fill="${accent}"/>`,
    )
    .replace(/\sclass="brand-logo-dot"/g, "");

  return `<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" width="1024" height="1024" viewBox="0 0 40 40">
  <!-- Derived from apps/desktop/src/components/BrandLogo.tsx — do not edit by hand -->
  <rect width="40" height="40" fill="${bg}"/>
  ${body}
</svg>
`;
}

const tsx = fs.readFileSync(brandLogoPath, "utf8");
const css = fs.readFileSync(cssPath, "utf8");
const ink = readCssVar(css, "--ink");
const accent = readCssVar(css, "--accent");
const bg = readCssVar(css, "--bg0");

const svg = toStandaloneSvg(extractBrandSvg(tsx), ink, accent, bg);
fs.mkdirSync(iconsDir, { recursive: true });
fs.writeFileSync(derivedSvg, svg, "utf8");
console.log("Wrote", path.relative(root, derivedSvg));

// Rasterize via @resvg/resvg-js (install transiently next to this script's require)
const require = createRequire(import.meta.url);
let Resvg: new (svg: string | Buffer, opts?: object) => { render: () => { asPng: () => Buffer } };
try {
  ({ Resvg } = require("@resvg/resvg-js"));
} catch {
  throw new Error(
    "@resvg/resvg-js missing — run npm install from the repo root first",
  );
}

const resvg = new Resvg(svg, {
  fitTo: { mode: "width", value: 1024 },
  background: bg,
});
fs.writeFileSync(masterPng, resvg.render().asPng());
console.log("Wrote", path.relative(root, masterPng));

// Quote path: workspace folder name contains spaces ("Disag Reddot 2")
execFileSync(
  `npm exec -- tauri icon "${masterPng}"`,
  {
    cwd: path.join(root, "apps/desktop"),
    stdio: "inherit",
    shell: true,
  },
);

// Desktop-only: drop mobile extras from `tauri icon`
for (const extra of ["ios", "android"]) {
  fs.rmSync(path.join(iconsDir, extra), { recursive: true, force: true });
}

// Remove old hand-authored duplicate if present
const oldHand = path.join(iconsDir, "app-icon-source.svg");
if (fs.existsSync(oldHand)) fs.unlinkSync(oldHand);
const oldPs1 = path.join(iconsDir, "generate-icon.ps1");
if (fs.existsSync(oldPs1)) fs.unlinkSync(oldPs1);

console.log("Done — icons regenerated from BrandLogo.tsx");

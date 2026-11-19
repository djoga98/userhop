#!/usr/bin/env node
// Assembles the landing page, the built demo and the README media for GitHub Pages.
// Usage: node scripts/build-site.mjs [outDir]  (run `vite build -c vite.demo.config.ts` first)
import { access, cp, mkdir, readdir, rm } from "node:fs/promises";
import { join } from "node:path";

const outDir = process.argv[2] ?? "dist-site";
const demoDir = "dist-demo";
const mediaSource = "assets/readme";
const mediaPattern = /\.(mp4|webp)$/;

try {
  await access(join(demoDir, "demo", "index.html"));
} catch {
  throw new Error(`${demoDir} is missing; run "vite build -c vite.demo.config.ts" first`);
}

await rm(outDir, { recursive: true, force: true });
await cp("site", outDir, { recursive: true });
await cp(demoDir, outDir, { recursive: true });
await mkdir(join(outDir, "media"), { recursive: true });
const media = (await readdir(mediaSource)).filter((name) => mediaPattern.test(name));
await Promise.all(media.map((name) => cp(join(mediaSource, name), join(outDir, "media", name))));
console.log(`site assembled in ${outDir} with the demo and ${media.length} media files`);

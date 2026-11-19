#!/usr/bin/env node
// Assembles the landing page and its README media into one directory for GitHub Pages.
// Usage: node scripts/build-site.mjs [outDir]
import { cp, mkdir, readdir, rm } from "node:fs/promises";
import { join } from "node:path";

const outDir = process.argv[2] ?? "dist-site";
const mediaSource = "assets/readme";
const mediaPattern = /\.(mp4|webp)$/;

await rm(outDir, { recursive: true, force: true });
await cp("site", outDir, { recursive: true });
await mkdir(join(outDir, "media"), { recursive: true });
const media = (await readdir(mediaSource)).filter((name) => mediaPattern.test(name));
await Promise.all(media.map((name) => cp(join(mediaSource, name), join(outDir, "media", name))));
console.log(`site assembled in ${outDir} with ${media.length} media files`);

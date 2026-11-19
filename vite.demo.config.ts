import { fileURLToPath } from "node:url";
import { defineConfig, mergeConfig } from "vite";

import base from "./vite.config.ts";

export default mergeConfig(
  base,
  defineConfig({
    base: "./",
    build: {
      outDir: "dist-demo",
      emptyOutDir: true,
      target: "es2022",
      rollupOptions: {
        input: fileURLToPath(new URL("./demo/index.html", import.meta.url)),
      },
    },
  }),
);

import { fileURLToPath } from "node:url";
import { defineConfig, mergeConfig } from "vite";

import base from "./vite.config";

export default mergeConfig(
  base,
  defineConfig({
    base: "./",
    build: {
      outDir: "dist-preview",
      emptyOutDir: true,
      rollupOptions: {
        input: fileURLToPath(new URL("./preview/index.html", import.meta.url)),
      },
    },
  }),
);

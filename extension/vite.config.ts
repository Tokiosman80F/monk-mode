import { defineConfig } from "vitest/config";

export default defineConfig({
  base: "",
  build: {
    rollupOptions: {
      input: {
        background: "src/background.ts",
        content: "src/content.ts",
        block: "block.html",
      },
      output: {
        entryFileNames: "[name].js",
        chunkFileNames: "chunks/[name]-[hash].js",
        assetFileNames: "assets/[name]-[hash][extname]",
      },
    },
  },
});

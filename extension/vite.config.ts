import { defineConfig } from "vite";

export default defineConfig({
  build: {
    rollupOptions: {
      input: "src/background.ts",
      output: { entryFileNames: "[name].js" },
    },
  },
});

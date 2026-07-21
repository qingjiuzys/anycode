import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import path from "node:path";

export default defineConfig({
  plugins: [react()],
  resolve: {
    alias: {
      "@anycode/site-urls": path.resolve(__dirname, "../../brand/site-urls.ts"),
      "@anycode/brand-mark": path.resolve(__dirname, "../../brand/anycode-mark.svg"),
    },
  },
  server: {
    host: true,
    port: 43201,
    fs: {
      allow: [path.resolve(__dirname, "../..")],
    },
    proxy: {
      "/api": "http://127.0.0.1:43200",
      "/health": "http://127.0.0.1:43200",
    },
  },
  build: {
    outDir: "dist",
    emptyOutDir: true,
  },
});

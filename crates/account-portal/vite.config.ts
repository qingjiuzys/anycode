import { defineConfig, type Plugin } from "vite";
import react from "@vitejs/plugin-react";
import path from "node:path";

/** Public demos live under /demos/<name>/index.html — Vite dev must not SPA-fallback them. */
function demosIndexFallback(): Plugin {
  return {
    name: "demos-index-fallback",
    configureServer(server) {
      server.middlewares.use((req, _res, next) => {
        if (!req.url) return next();
        const q = req.url.indexOf("?");
        const pathname = q === -1 ? req.url : req.url.slice(0, q);
        const search = q === -1 ? "" : req.url.slice(q);
        if (pathname.startsWith("/demos/") && pathname.endsWith("/")) {
          req.url = `${pathname}index.html${search}`;
        }
        next();
      });
    },
  };
}

export default defineConfig({
  plugins: [react(), demosIndexFallback()],
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

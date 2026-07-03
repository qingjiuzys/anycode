import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";
import path from "node:path";

export default defineConfig({
  plugins: [
    react(),
    tailwindcss(),
    {
      name: "html-build-stamp",
      transformIndexHtml(html) {
        const stamp = `<meta name="anycode-ui-build" content="${new Date().toISOString()}" />`;
        const apiBootstrap = `<script>
  (function () {
    try {
      var base = sessionStorage.getItem("anycode_api_base");
      if (base) window.__ANYCODE_API_BASE__ = base;
    } catch (e) {}
  })();
</script>`;
        return html.replace("<head>", `<head>\n    ${stamp}\n    ${apiBootstrap}`);
      },
    },
  ],
  resolve: {
    alias: {
      "@": path.resolve(__dirname, "./src"),
    },
  },
  server: {
    port: 5173,
    proxy: {
      "/api": {
        target: `http://127.0.0.1:${process.env.ANYCODE_DASHBOARD_DEV_PORT ?? "43180"}`,
        changeOrigin: true,
      },
    },
  },
  build: {
    outDir: "dist",
    emptyOutDir: true,
    base: "./",
    rollupOptions: {
      output: {
        manualChunks(id) {
          if (id.includes("node_modules/echarts")) return "echarts";
          if (id.includes("node_modules/reactflow")) return "reactflow";
          if (id.includes("node_modules/@tanstack/react-router")) return "router";
          if (id.includes("node_modules/@tanstack/react-query")) return "query";
          if (id.includes("node_modules/react-dom") || id.includes("node_modules/react/")) {
            return "react";
          }
        },
      },
    },
  },
});

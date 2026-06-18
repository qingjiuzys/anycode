import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

export default defineConfig({
  plugins: [react()],
  base: "/ops/",
  server: {
    port: 43220,
    proxy: {
      "/api": "http://127.0.0.1:43200",
    },
  },
});

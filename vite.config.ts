import { fileURLToPath, URL } from "node:url";
import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";

/** 多页入口：主窗口 index.html + 胶囊窗口 capsule.html */
const page = (name: string) => fileURLToPath(new URL(`./${name}`, import.meta.url));

export default defineConfig({
  plugins: [vue()],
  clearScreen: false,
  server: { port: 1420, strictPort: true, host: "127.0.0.1" },
  build: {
    target: "chrome105",
    minify: "esbuild",
    sourcemap: false,
    rollupOptions: {
      input: {
        main: page("index.html"),
        capsule: page("capsule.html")
      }
    }
  }
});

/**
 * 前端开发与构建配置，固定端口以匹配 Tauri 开发地址。
 *
 * @author lhy
 */
import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";
export default defineConfig({
  plugins: [vue()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    watch: { ignored: ["**/src-tauri/**", "**/work/**", "**/build/**"] },
  },
  build: { target: "es2022" },
});

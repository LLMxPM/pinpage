// 功能：配置 Vue 构建与 Tauri 使用的固定开发端口。
import { defineConfig } from 'vite';
import vue from '@vitejs/plugin-vue';

export default defineConfig({
  plugins: [vue()],
  clearScreen: false,
  server: {
    host: '127.0.0.1',
    port: 1420,
    strictPort: true,
    watch: { ignored: ['**/src-tauri/**', '**/runtime-data/**'] },
  },
  build: { target: 'es2022' },
});

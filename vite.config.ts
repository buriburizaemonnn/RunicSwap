/*
import { fileURLToPath, URL } from "url";
import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";
import environment from "vite-plugin-environment";
import dotenv from "dotenv";

dotenv.config({ path: "../../.env" });

export default defineConfig({
  build: {
    emptyOutDir: true,
  },
  optimizeDeps: {
    esbuildOptions: {
      define: {
        global: "globalThis",
      },
    },
  },
  server: {
    proxy: {
      "/api": {
        target: "http://127.0.0.1:4943",
        changeOrigin: true,
      },
    },
  },
  plugins: [
    react(),
    environment("all", { prefix: "CANISTER_" }),
    environment("all", { prefix: "DFX_" }),
  ],
  resolve: {
    alias: [
      {
        find: "declarations",
        replacement: fileURLToPath(new URL("../declarations", import.meta.url)),
      },
    ],
  },
});
*/

import { fileURLToPath, URL } from "url";
import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";
import environment from "vite-plugin-environment";
import dotenv from "dotenv";
import wasm from "vite-plugin-wasm";
import { createHtmlPlugin } from "vite-plugin-html";
import { nodePolyfills } from "vite-plugin-node-polyfills";

dotenv.config({ path: "./.env" });

export default defineConfig({
  build: {
    emptyOutDir: true,
    minify: "terser",
    terserOptions: {
      compress: {
        drop_console: process.env.PROVIDER ? true : false,
        drop_debugger: process.env.PROVIDER ? true : false,
      },
    },
  },
  optimizeDeps: {
    esbuildOptions: {
      define: {
        global: "globalThis",
      },
      supported: {
        bigint: true,
      },
      jsx: "automatic",
    },
  },
  server: {
    proxy: {
      "/api": {
        target: "http://127.0.0.1:4943",
        changeOrigin: true,
      },
    },
  },
  plugins: [
    react(),
    wasm(),
    nodePolyfills({
      globals: {
        Buffer: false,
        global: true,
        process: true,
      },
      protocolImports: false,
    }),
    createHtmlPlugin({
      minify: true,
    }),
    environment("all", { prefix: "CANISTER_" }),
    environment("all", { prefix: "DFX_" }),
  ],
  resolve: {
    alias: [
      {
        find: "declarations",
        replacement: fileURLToPath(new URL("../declarations", import.meta.url)),
      },
    ],
  },
  define: {
    "process.env.DFX_NETWORK": JSON.stringify(process.env.DFX_NETWORK),
    "process.env.PROVIDER": process.env.PROVIDER
      ? JSON.stringify(process.env.PROVIDER)
      : JSON.stringify("br5f7-7uaaa-aaaaa-qaaca-cai"),
  },
});

import { defineConfig } from "vitest/config";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import { svelteTesting } from "@testing-library/svelte/vite";
import path from "node:path";

export default defineConfig({
  plugins: [svelte({ hot: false }), svelteTesting()],
  resolve: {
    alias: {
      "@": path.resolve(__dirname, "src"),
      "@testing-library/svelte": path.resolve(__dirname, "node_modules/@testing-library/svelte"),
    },
  },
  test: {
    environment: "happy-dom",
    globals: true,
    setupFiles: ["../tests/setup.ts"],
    include: [
      "../tests/unit/**/*.test.ts",
      "../tests/integration/**/*.test.ts",
      "../tests/components/**/*.test.ts",
    ],
  },
});

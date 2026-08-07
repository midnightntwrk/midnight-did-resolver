import { defineConfig } from "vitest/config";

export default defineConfig({
  mode: "node",
  test: {
    deps: {
      interopDefault: true,
    },
    globals: true,
    environment: "node",
    include: ["src/test/**/*.test.ts"],
    root: ".",
    coverage: {
      provider: "v8",
      reporter: ["text", "json", "json-summary", "html"],
      include: ["src/**/*.ts"],
      thresholds: {
        lines: 90,
        statements: 90,
        functions: 90,
        branches: 80,
      },
      exclude: [
        "dist/**",
        "**/*.d.ts",
        "src/test/**",
        "**/*.test.ts",
        "src/types.ts",
        "vitest.config.ts",
      ],
    },
  },
  resolve: {
    extensions: [".ts", ".js"],
    conditions: ["import", "node", "default"],
  },
});

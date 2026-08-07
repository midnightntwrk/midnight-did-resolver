import { defineConfig } from "vitest/config";

export default defineConfig({
  mode: "node",
  test: {
    include: ["src/test/**/*.test.ts"],
    exclude: ["src/test/integration/**"],
    globals: true,
    environment: "node",
    reporters: ["default"],
    coverage: {
      reporter: ["text", "json", "json-summary", "html"],
      thresholds: {
        lines: 85,
        statements: 85,
        functions: 85,
        branches: 75,
      },
      include: ["src/**/*.ts"],
      exclude: ["src/test/**", "**/*.d.ts", "src/types.ts"],
    },
  },
});

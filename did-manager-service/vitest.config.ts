import { defineConfig } from 'vitest/config';

export default defineConfig({
  test: {
    include: ['src/test/**/*.test.ts'],
    exclude: ['src/e2e/**'],
    coverage: {
      provider: 'v8',
      reporter: ['text', 'json', 'json-summary', 'html'],
      thresholds: {
        lines: 80,
        statements: 80,
        functions: 80,
        branches: 70,
      },
      include: ['src/**/*.ts'],
      exclude: ['src/test/**', 'src/e2e/**', '**/*.d.ts', 'src/types.ts', 'vitest.config.ts'],
    },
  },
});

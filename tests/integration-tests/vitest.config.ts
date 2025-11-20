import { defineConfig } from 'vitest/config';

export default defineConfig({
  test: {
    // Test environment
    environment: 'node',

    // Test timeout (10 minutes for integration tests)
    testTimeout: 600_000,

    // Hook timeouts
    hookTimeout: 30_000,

    // Include test files
    include: ['src/**/*.{test,spec}.{ts,tsx}'],

    // Global setup/teardown
    globals: false,

    // Run tests sequentially (important for integration tests that may have side effects)
    threads: false,

    // Run test files sequentially to avoid database lock conflicts
    fileParallelism: false,

    // Reporters
    reporters: ['verbose'],

    // Coverage configuration (optional)
    coverage: {
      provider: 'v8',
      reporter: ['text', 'json', 'html'],
      exclude: ['node_modules/', 'vendor/', 'dist/'],
    },
  },
});

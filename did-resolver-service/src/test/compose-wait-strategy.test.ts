import { readFileSync } from "node:fs";

import { describe, expect, it } from "vitest";

// Testcontainers 11 Compose wait keys are parsed container names (service-1),
// not service names. An unmatched HTTP wait silently falls back to a shell-based
// port probe, which cannot inspect the shell-free production image.
describe("resolver Compose readiness wiring", () => {
  it.each(["resolver.container.test.ts", "resolver.did-flow.test.ts"])(
    "%s registers the real HTTP health wait for replica one",
    (file) => {
      const source = readFileSync(
        new URL(`./integration/${file}`, import.meta.url),
        "utf8",
      );
      expect(source).toMatch(
        /\.withWaitStrategy\(\s*"did-resolver-1",\s*Wait\.forHttp\("\/health",\s*3001\)/,
      );
      expect(source).not.toMatch(/\.withWaitStrategy\(\s*"did-resolver",/);
      if (file === "resolver.did-flow.test.ts") {
        expect(source).toMatch(
          /\.withWaitStrategy\(\s*"indexer-1",\s*Wait\.forHealthCheck\(\)/,
        );
      }
    },
  );
});

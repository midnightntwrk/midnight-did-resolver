// Copyright (C) Midnight Foundation
// SPDX-License-Identifier: Apache-2.0
import assert from "node:assert/strict";
import { mkdtempSync, mkdirSync, readFileSync, writeFileSync, symlinkSync, existsSync, rmSync, lstatSync } from "node:fs";
import os from "node:os";
import path from "node:path";
import { test } from "node:test";
import { exportRuntime, inspectRuntime } from "./export-runtime.mjs";

function fixture(t) {
  const tmp = mkdtempSync(path.join(os.tmpdir(), "runtime-export-"));
  t.after(() => rmSync(tmp, { recursive: true, force: true }));
  const source = path.join(tmp, "source");
  const file = (name, value = "payload") => {
    const target = path.join(source, name);
    mkdirSync(path.dirname(target), { recursive: true });
    writeFileSync(target, value);
  };
  file("package.json", '{"type":"module","version":"1.0.0"}');
  file("version", "1.0.0\n");
  for (const dir of ["did-resolver-service", "did-manager-service", "secret-storage", "docs-site"]) {
    const name = dir === "docs-site" ? dir : "@midnight-ntwrk/midnight-did-" + (dir === "secret-storage" ? dir : dir.replace("did-", ""));
    file(dir + "/package.json", JSON.stringify({ name }));
    file(dir + "/dist/index.js", "export {};");
    file(dir + "/dist/index.d.ts");
    file(dir + "/dist/index.js.map");
    file(dir + "/src/private.ts");
    const link = path.join(source, "node_modules", name);
    mkdirSync(path.dirname(link), { recursive: true });
    symlinkSync(path.relative(path.dirname(link), path.join(source, dir)), link);
  }
  for (const name of ["README.md", "LICENSE", "compiler/info.json", "static/logo.svg", "prebuilds/linux-x64/classic-level.node", "node_modules/nested/index.js", "snippets/example.js", "runtime.wasm"]) file("node_modules/vendor/" + name);
  file("did-manager-service/node_modules/local/index.js");
  return { source, output: path.join(tmp, "output"), file };
}

for (const service of ["did-resolver-service", "did-manager-service"]) test(service + " exports only built own workspaces and preserves vendor payload", (t) => {
  const { source, output } = fixture(t);
  const result = exportRuntime(source, output, service);
  assert.equal(result.links, service === "did-manager-service" ? 2 : 1);
  assert.ok(result.files > 8);
  assert.equal(existsSync(path.join(output, service, "src")), false);
  assert.equal(existsSync(path.join(output, service, "dist/index.d.ts")), false);
  assert.equal(existsSync(path.join(output, service, "dist/index.js.map")), false);
  assert.ok(existsSync(path.join(source, service, "dist/index.js.map")));
  assert.ok(existsSync(path.join(output, "node_modules/vendor/runtime.wasm")));
  assert.ok(existsSync(path.join(output, "node_modules/vendor/node_modules/nested/index.js")));
  assert.ok(existsSync(path.join(output, "node_modules/vendor/LICENSE")));
  assert.equal(existsSync(path.join(output, "docs-site")), false);
  if (service === "did-manager-service") assert.ok(existsSync(path.join(output, service, "node_modules/local/index.js")));
  assert.throws(() => exportRuntime(source, output, service), /must not exist/);
});

test("reject missing builds, unknown service, dangling and escaping links", (t) => {
  const { source, output } = fixture(t);
  assert.throws(() => exportRuntime(source, output, "other"), /Unknown/);
  rmSync(path.join(source, "did-resolver-service/dist/index.js"));
  assert.throws(() => exportRuntime(source, output, "did-resolver-service"), /Missing built/);
  const link = path.join(source, "bad-link");
  symlinkSync("absent", link);
  assert.throws(() => inspectRuntime(source), /ENOENT/);
  rmSync(link);
  symlinkSync(os.tmpdir(), link);
  assert.throws(() => inspectRuntime(source), /escapes/);
});

test("removes only export-root npm installation metadata", (t) => {
  const { source, output, file } = fixture(t);
  file("node_modules/.package-lock.json", '{"install":"metadata"}');
  file("node_modules/vendor/.package-lock.json", '{"vendor":"payload"}');
  file("node_modules/vendor/package-lock.json", '{"vendor":"lock"}');
  exportRuntime(source, output, "did-manager-service");
  assert.equal(existsSync(path.join(output, "node_modules/.package-lock.json")), false);
  assert.ok(existsSync(path.join(source, "node_modules/.package-lock.json")));
  for (const name of [".package-lock.json", "package-lock.json"]) {
    assert.equal(readFileSync(path.join(output, "node_modules/vendor", name), "utf8"),
      readFileSync(path.join(source, "node_modules/vendor", name), "utf8"));
  }
});

test("both Dockerfiles obey base/export policy and preserve runtime contracts", () => {
  const policy = JSON.parse(readFileSync(new URL("./runtime-base-policy.json", import.meta.url)));
  assert.equal(policy.schemaVersion, 1);
  assert.equal(policy.nodeVersion, "24.22.3");
  assert.deepEqual(Object.keys(policy.platforms), ["linux/amd64", "linux/arm64"]);
  for (const service of ["did-resolver-service", "did-manager-service"]) {
    const dockerfile = readFileSync(new URL("../" + service + "/Dockerfile", import.meta.url), "utf8");
    assert.equal(dockerfile.split("\n").filter(line => line.startsWith("FROM ")).length, 2);
    assert.ok(dockerfile.startsWith("FROM " + policy.builder.reference.replace("docker.io/library/", "") + " AS builder"));
    assert.ok(dockerfile.includes("FROM " + policy.reference + " AS runtime"));
    assert.equal(policy.builder.nodeVersion, policy.nodeVersion);
    assert.equal(policy.builder.indexDigest, "sha256:dc26ae92c9fb15cf6b1d63e20f0944b2efe0f115b20e00be48fb23fceb061c54");
    assert.equal(policy.indexDigest, "sha256:6abfcb367ebc291572c774c1592a986be2ab26a2486f73093401327495a5962a");
    assert.ok(dockerfile.includes("npm ci --ignore-scripts"));
    assert.ok(dockerfile.includes("npm prune --omit=dev --ignore-scripts"));
    assert.ok(!dockerfile.includes("--omit=optional"));
    assert.ok(dockerfile.includes("/runtime " + service));
    assert.ok(dockerfile.includes("COPY --from=builder --chown=1000:1000 /runtime/"));
    assert.ok(dockerfile.includes("USER 1000:1000"));
    assert.ok(dockerfile.includes("ENV PATH=/nodejs/bin:"));
    assert.ok(dockerfile.includes("ENV HOME=/home/node"));
    assert.ok(dockerfile.includes("ENTRYPOINT []"));
    assert.ok(dockerfile.includes("RUN mkdir -m 0750 /runtime-home"));
    assert.ok(dockerfile.includes("COPY --from=builder --chown=1000:1000 /runtime-home/ /home/node/"));
    const finalStage = dockerfile.split(" AS runtime")[1];
    assert.ok(!/^RUN /m.test(finalStage));
    assert.ok(!/npm start|sh -c|apt-get/.test(finalStage));
    assert.ok(dockerfile.includes('CMD ["node", "--experimental-specifier-resolution=node", "dist/index.js"]'));
  }
});

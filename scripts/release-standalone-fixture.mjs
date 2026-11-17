// This file is part of midnightntwrk/midnight-did-resolver.
// SPDX-License-Identifier: Apache-2.0
// Host orchestration only: stdlib + Docker argv; no host application imports.
import assert from "node:assert/strict";
import { createHash, randomUUID } from "node:crypto";
import { mkdtemp, writeFile, readFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { validateRuntimeFixture } from "./release-runtime-smoke.mjs";
const hash = bytes => createHash("sha256").update(bytes).digest("hex");
export const dependencyImages = Object.freeze({
  node: "docker.io/midnightntwrk/midnight-node:2.1.101@sha256:0c1ad548d523c1109aa1f16daab1bf72dcb78c7e17b8a8c6d97424ad0eba7b8a",
  indexer: "docker.io/midnightntwrk/indexer-standalone:4.4.101@sha256:119f35db548ba1c390845f215bb1e928e73038bef7c315825aee0461bd54499d",
  proof: "docker.io/midnightntwrk/proof-server:9.0.1@sha256:90e36a14f92c2004127f0ed1cfd6ee89d770cf1f6e9cd7f02026ffd4ecb42f37",
});
export function composeSpecification() {
  const ports = port => [{ target: port, published: "0", host_ip: "127.0.0.1", protocol: "tcp" }];
  const health = (test, interval, retries, start) => ({ test, interval, timeout: "5s", retries, start_period: start });
  return { services: {
    node: { image: dependencyImages.node, platform: "linux/amd64", ports: ports(9944),
      environment: { CFG_PRESET: "dev", SIDECHAIN_BLOCK_BENEFICIARY: "04bcf7ad3be7a5c790460be82a713af570f22e0f801f6659ab8e84a52be6969e" },
      healthcheck: health(["CMD-SHELL", "curl -sf http://localhost:9944/health >/dev/null && curl -sf -H 'Content-Type: application/json' -d '{\"id\":1,\"jsonrpc\":\"2.0\",\"method\":\"chain_getHeader\",\"params\":[]}' http://localhost:9944 | grep -Eq '\"number\":\"0x0*[1-9a-f][0-9a-f]*\"'"], "2s", 20, "20s") },
    indexer: { image: dependencyImages.indexer, platform: "linux/amd64", ports: ports(8088),
      environment: { APP__INFRA__NODE__URL: "ws://node:9944", APP__INFRA__SPO_NODE__URL: "ws://node:9944",
        APP__INFRA__STORAGE__PASSWORD: "indexer", APP__INFRA__PUB_SUB__PASSWORD: "indexer",
        APP__INFRA__LEDGER_STATE_STORAGE__PASSWORD: "indexer", APP__INFRA__SPO_NODE__BLOCKFROST_ID: "dummy-not-using-spo",
        APP__INFRA__SECRET: "303132333435363738393031323334353637383930313233343536373839303132",
        APP__APPLICATION__NETWORK_ID: "undeployed",
        RUST_LOG: "indexer=info,chain_indexer=info,indexer_api=info,wallet_indexer=info,indexer_common=info,fastrace_opentelemetry=off,info" },
      healthcheck: health(["CMD-SHELL", "cat /var/run/indexer-standalone/running"], "10s", 20, "10s"),
      depends_on: { node: { condition: "service_healthy" } } },
    "proof-server": { image: dependencyImages.proof, platform: "linux/amd64", ports: ports(6300),
      command: ["midnight-proof-server", "-v"], environment: { RUST_BACKTRACE: "full" },
      healthcheck: health(["CMD", "/bin/bash", "-c",
        "exec 3<>/dev/tcp/127.0.0.1/6300 || exit 1; printf 'GET /version HTTP/1.1\\r\\nHost: localhost\\r\\nConnection: close\\r\\n\\r\\n' >&3; IFS= read -r -t 3 status <&3 || exit 1; [[ \"$$status\" == \"HTTP/1.1 200 \"* || \"$$status\" == \"HTTP/1.0 200 \"* ]]"], "10s", 60, "10s") },
  } };
}
export function mappedPort(output) {
  assert.match(output.trim(), /^127\.0\.0\.1:[1-9][0-9]*$/);
  const port = Number(output.trim().split(":")[1]);
  assert.ok(Number.isInteger(port) && port <= 65535); return port;
}
export async function withRuntimeFixture({ externalFixture, externalFixtureApproved = false, managerGraph, exec, save, run }) {
  if (externalFixture) {
    assert.equal(externalFixtureApproved, true, "External fixture needs explicit dispatcher approval");
    return run(validateRuntimeFixture(externalFixture));
  }
  assert.equal(managerGraph?.service, "manager");
  const child = managerGraph.platforms.find(p => p.platform === "linux/amd64");
  assert.match(child?.manifestDigest, /^sha256:[a-f0-9]{64}$/);
  assert.equal(managerGraph.repository, "ghcr.io/midnightntwrk/midnight-did-manager-service");
  const project = `release-fixture-${randomUUID()}`;
  const dir = await mkdtemp(join(tmpdir(), project + "-"));
  const file = join(dir, "compose.json"), specification = composeSpecification();
  const helper = fileURLToPath(new URL("./release-fixture-deploy.mjs", import.meta.url));
  assert.ok(!helper.includes(","));
  const scriptSHA256 = hash(await readFile(helper));
  await writeFile(file, JSON.stringify(specification));
  const compose = (...args) => exec("docker", ["compose", "--project-name", project, "--file", file, ...args]);
  const deployerName = project + "-deployer";
  let projectStarted = false, deployerAttempted = false;
  const cleanup = { project, ownedComposeSHA256: hash(JSON.stringify(specification)), cleaned: false };
  try {
    await save("standalone-compose.json", specification);
    await save("standalone-helper.json", { script: "scripts/release-fixture-deploy.mjs", sha256: scriptSHA256, dependencyImages });
    projectStarted = true; // up may partially create containers; always down this exact project/file
    await compose("up", "--detach", "--wait", "--wait-timeout", "240", "--no-build");
    const ports = {};
    for (const [service, port] of [["node", 9944], ["indexer", 8088], ["proof-server", 6300]]) {
      ports[service] = mappedPort((await compose("port", service, String(port))).stdout);
    }
    const config = { schemaVersion: 1, indexer: `http://127.0.0.1:${ports.indexer}/api/v4/graphql`,
      indexerWS: `ws://127.0.0.1:${ports.indexer}/api/v4/graphql/ws`,
      node: `http://127.0.0.1:${ports.node}`, proofServer: `http://127.0.0.1:${ports["proof-server"]}` };
    const image = `${managerGraph.repository}@${child.manifestDigest}`;
    await exec("docker", ["pull", "--platform", "linux/amd64", image]);
    deployerAttempted = true;
    const deployed = await exec("docker", ["run", "--name", deployerName, "--platform", "linux/amd64", "--network", "host",
      "--mount", `type=bind,src=${helper},dst=/release-fixture-deploy.mjs,readonly`,
      "--env", "MIDNIGHT_NETWORK=undeployed", "--entrypoint", "node", image, "/release-fixture-deploy.mjs", JSON.stringify(config)],
      { timeout: 1800000 });
    const fixture = validateRuntimeFixture(JSON.parse(deployed.stdout));
    assert.equal(fixture.indexerHTTP, config.indexer); assert.equal(fixture.indexerWS, config.indexerWS);
    fixture.dependencyImages = dependencyImages;
    fixture.deployment = { pulledReference: image, digestPullVerified: true, scriptSHA256, platform: "linux/amd64" };
    await save("runtime-fixture.json", fixture);
    return await run(fixture); // external state survives all four service lifecycles
  } finally {
    try {
      // Diagnostics contain only lifecycle/status, never wallet stdout/seed/key logs.
      if (projectStarted) {
        const ps = await compose("ps", "--format", "json");
        await save("standalone-status.txt", ps.stdout);
      }
    } finally {
      try { if (deployerAttempted) await exec("docker", ["rm", "--force", deployerName]); }
      finally {
        try { if (projectStarted) await compose("down", "--volumes", "--timeout", "30"); cleanup.cleaned = true; }
        finally { await save("standalone-cleanup.json", cleanup); await rm(dir, { recursive: true, force: true }); }
      }
    }
  }
}

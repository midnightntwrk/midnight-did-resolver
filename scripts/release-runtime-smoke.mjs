// This file is part of midnightntwrk/midnight-did-resolver.
// SPDX-License-Identifier: Apache-2.0
import assert from "node:assert/strict";
import { randomUUID, createHash } from "node:crypto";
import { readFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { arch } from "node:os";
import { createServer } from "node:net";

const digestPattern = /^sha256:[a-f0-9]{64}$/;
const repositories = ["resolver", "manager"].map(s => `ghcr.io/midnightntwrk/midnight-did-${s}-service`);
export function validateRuntimeFixture(fixture) {
  assert.equal(fixture?.schemaVersion, 1, "Approved public runtime fixture required");
  assert.equal(fixture.kind, "standalone-external-state", "No public network/funds or source-build fallback");
  assert.match(fixture.did, /^did:midnight:[^\s]+$/);
  assert.equal(fixture.expectedResolution?.didDocument?.id, fixture.did, "Positive resolution fixture required, not an empty/error payload");
  assert.equal(fixture.expectedResolution.didResolutionMetadata?.error, null, "Positive fixture must not contain a resolution error");
  assert.ok(fixture.expectedResolution.didDocument.verificationMethod?.some(m => m.id.startsWith(fixture.did + "#")), "Known public fixture method required");
  for (const [key, protocols] of [["indexerHTTP", ["http:"]], ["indexerWS", ["ws:"]]]) {
    const u = new URL(fixture[key]);
    assert.ok(protocols.includes(u.protocol) && ["127.0.0.1", "localhost", "[::1]"].includes(u.hostname));
    assert.equal(u.username + u.password + u.hash, "", "No credentials in public fixture");
  }
  return fixture;
}
export async function requestJSON(base, path, body, fetchImpl = fetch) {
  const url = new URL(base);
  assert.ok(["127.0.0.1", "localhost", "[::1]"].includes(url.hostname) && url.protocol === "http:");
  const response = await fetchImpl(new URL(path, url), {
    method: body === undefined ? "GET" : "POST", redirect: "error",
    headers: body === undefined ? {} : { "content-type": "application/json" },
    body: body === undefined ? undefined : JSON.stringify(body),
    signal: AbortSignal.timeout(10000),
  });
  assert.equal(response.status, 200, `Runtime HTTP gate failed: ${path}`);
  return response.json();
}
export async function checkHTTP(base, { service, fixture, fetchImpl = fetch, selected = false }) {
  assert.equal((await requestJSON(base, "/health", undefined, fetchImpl)).status, "ok");
  assert.equal((await requestJSON(base, "/ready", undefined, fetchImpl)).status, "ready");
  const spec = await requestJSON(base, "/docs/json", undefined, fetchImpl);
  assert.match(spec.openapi, /^3\./);
  assert.equal(spec.info.version, "1.0.0");
  const docs = await fetchImpl(new URL("/docs/", base), { redirect: "error", signal: AbortSignal.timeout(10000) });
  assert.equal(docs.status, 200);
  const html = await docs.text();
  const assets = [...html.matchAll(/(?:src|href)=["']([^"']*(?:swagger-ui[^"']*\.(?:js|css)))["']/g)].map(m => m[1]);
  assert.ok(assets.length >= 2, "Swagger JavaScript and stylesheet must actually be served");
  for (const asset of assets) {
    const target = new URL(asset, new URL("/docs/", base));
    assert.equal(target.origin, new URL(base).origin);
    const result = await fetchImpl(target, { redirect: "error", signal: AbortSignal.timeout(10000) });
    assert.equal(result.status, 200);
    assert.ok((await result.text()).length > 100, "Missing swagger asset");
  }
  if (service === "manager") {
    if (!selected) {
      const result = await requestJSON(base, "/api/profiles/select", { name: "release-public-fixture" }, fetchImpl);
      assert.equal(result.ok, true);
    }
    const profiles = await requestJSON(base, "/api/profiles", undefined, fetchImpl);
    assert.equal(profiles.ok, true);
    assert.equal(profiles.data.profile, "standalone");
    assert.equal(profiles.data.activeProfileName, "release-public-fixture");
    assert.ok(profiles.data.availableProfileNames.includes("release-public-fixture"));
  } else {
    assert.deepEqual(await requestJSON(base, `/resolve/${encodeURIComponent(fixture.did)}`, undefined, fetchImpl), fixture.expectedResolution);
  }
}
async function unusedPort() {
  const s = createServer();
  await new Promise((ok, fail) => { s.once("error", fail); s.listen(0, "127.0.0.1", ok); });
  const port = s.address().port;
  await new Promise((ok, fail) => s.close(e => e ? fail(e) : ok()));
  return port;
}
// Only an existing, digest-pinned child image may run. Caller supplies execFile
// adapter, which revalidates canonical source before *every* Docker invocation.
export async function runtimeSmoke({ repository, platform, manifestDigest, fixture, exec, fetchImpl = fetch, hostArch = arch() }) {
  assert.ok(repositories.includes(repository));
  assert.ok(["linux/amd64", "linux/arm64"].includes(platform));
  assert.match(manifestDigest, digestPattern);
  assert.equal(hostArch, "x64", "This profile records native amd64 + QEMU arm64 only");
  validateRuntimeFixture(fixture);
  const service = repository.includes("-manager-") ? "manager" : "resolver";
  const image = `${repository}@${manifestDigest}`;
  const name = `release-${service}-${platform.split("/")[1]}-${randomUUID()}`;
  const volume = `${name}-data`;
  let volumeCreated = false;
  const port = await unusedPort();
  const base = `http://127.0.0.1:${port}`;
  let created = false, probeAttempted = false;
  const probeName = name + "-native-probe";
  try {
    await exec("docker", ["pull", "--platform", platform, image]);
    // Pull success is recorded only after the exact child/platform command exits 0.
    const pulledReference = image;
    const helper = fileURLToPath(new URL("./release-runtime-consumer.mjs", import.meta.url));
    assert.ok(!helper.includes(","));
    const probeScriptSHA256 = createHash("sha256").update(await readFile(helper)).digest("hex");
    const config = JSON.parse((await exec("docker", ["image", "inspect", image])).stdout)[0];
    assert.equal(config.Architecture, platform.split("/")[1]);
    assert.ok(["node", "1000", "1000:1000"].includes(config.Config.User), "Nonroot image default required");
    const env = Object.fromEntries(config.Config.Env.map(s => [s.slice(0, s.indexOf("=")), s.slice(s.indexOf("=") + 1)]));
    const dataDir = env.DID_MANAGER_DATA_DIR || `${env.HOME || "/home/node"}/.midnight-did`;
    assert.match(dataDir, /^\/[a-zA-Z0-9/_.-]+$/);
    assert.ok(!["/", "/app", "/home", "/usr"].includes(dataDir), "Dedicated default data directory required");
    probeAttempted = true;
    const probeResult = await exec("docker", ["run", "--name", probeName, "--platform", platform, "--network", "none",
      "--mount", `type=bind,src=${helper},dst=/release-runtime-consumer.mjs,readonly`,
      "--entrypoint", "node", image, "/release-runtime-consumer.mjs", service,
      platform === "linux/amd64" ? "x64" : "arm64"], { timeout: 300000 });
    const nativeProbe = JSON.parse(probeResult.stdout);
    assert.equal(nativeProbe.verified, true); assert.equal(nativeProbe.service, service);
    assert.equal(nativeProbe.node, "v24.22.3"); assert.equal(nativeProbe.uid, 1000);
    assert.equal(nativeProbe.processArch, platform === "linux/amd64" ? "x64" : "arm64");
    assert.equal(nativeProbe.ledgerWasmRoundtrip, true);
    assert.equal(nativeProbe.hdRole4Derived, true);
    assert.equal(nativeProbe.classicLevel, "open-put-get-close-reopen-get-close");
    assert.equal(nativeProbe.artifactVersion, "0.21.0");
    assert.equal(nativeProbe.verifierReads, 12); assert.equal(nativeProbe.rejectedIncompatibleRuntime, true);
    assert.equal(nativeProbe.artifacts?.length, 36);
    assert.equal(new Set(nativeProbe.artifacts.map(a => a.circuit + "|" + a.kind)).size, 36);
    assert.ok(nativeProbe.artifacts.every(a => ["prover", "verifier", "zkir"].includes(a.kind) && a.bytes > 0 && /^[a-f0-9]{64}$/.test(a.sha256)));
    if (service === "manager") assert.equal(nativeProbe.storageSignVerify, true);
    // Docker-owned volume initialization changes only its UID, never assumes a
    // host GID and never chmods host paths. Main service still uses image USER.
    await exec("docker", ["volume", "create", "--label", "midnight.release-smoke=owned", volume]);
    volumeCreated = true;
    await exec("docker", ["run", "--rm", "--platform", platform, "--network", "none", "--user", "0",
      "--mount", `type=volume,src=${volume},dst=/release-data`, "--entrypoint", "node", image, "-e",
      "const fs=require('node:fs');fs.chownSync('/release-data',1000,-1);fs.chmodSync('/release-data',0o700)"]);
    const runArgs = ["run", "--detach", "--name", name, "--platform", platform, "--network", "host",
      "--mount", `type=volume,src=${volume},dst=${dataDir}`,
      "--env", "MIDNIGHT_NETWORK=undeployed",
      "--env", `MIDNIGHT_INDEXER_HTTP_URL=${fixture.indexerHTTP}`,
      "--env", `MIDNIGHT_INDEXER_WS_URL=${fixture.indexerWS}`,
      "--env", "DID_MANAGER_SETUP=standalone",
      "--env", `${service === "manager" ? "DID_MANAGER" : "RESOLVER"}_HOST=127.0.0.1`,
      "--env", `${service === "manager" ? "DID_MANAGER" : "RESOLVER"}_PORT=${port}`, image];
    const start = async () => { await exec("docker", runArgs); created = true; };
    const healthy = async (selected) => {
      let last;
      for (let i = 0; i < 30; i++) {
        try { await requestJSON(base, "/health", undefined, fetchImpl); last = undefined; break; }
        catch (e) { last = e; await new Promise(ok => setTimeout(ok, 1000)); }
      }
      if (last) throw last;
      const runtime = JSON.parse((await exec("docker", ["exec", name, "node", "-e",
        "console.log(JSON.stringify({node:process.version,arch:process.arch,uid:process.getuid(),data:process.env.DID_MANAGER_DATA_DIR||process.env.HOME+'/.midnight-did'}))"])).stdout);
      assert.equal(runtime.node, "v24.22.3");
      assert.equal(runtime.uid, 1000);
      assert.equal(runtime.arch, platform === "linux/amd64" ? "x64" : "arm64");
      if (service === "manager") assert.equal(runtime.data, dataDir);
      await checkHTTP(base, { service, fixture, fetchImpl, selected });
    };
    await start(); await healthy(false);
    await exec("docker", ["restart", name]); await healthy(true);
    await exec("docker", ["rm", "--force", name]); created = false;
    await start(); await healthy(true);
    return { repository, platform, manifestDigest, digestPullVerified: true, pulledReference,
      probeScriptSHA256, nativeProbe, verified: true, health: true, ready: true,
      restart: true, recreate: true, persistence: true,
      persistenceScope: service === "manager" ? "public-profile-selection-and-list-metadata" : "stateless-repeat-resolution-against-external-state",
      persistenceMode: service === "manager" ? "public-profile-metadata" : "stateless-external-state",
      architectureMode: platform === "linux/amd64" ? "native-amd64" : "qemu-arm64",
      processArch: platform === "linux/amd64" ? "x64" : "arm64",
      nodeVersion: "24.22.3", uid: 1000, defaultDataDir: dataDir,
      limitations: ["No funded wallet, ledger write, session/contract restoration or native-arm64 certification"] };
  } finally {
    try { if (created) await exec("docker", ["rm", "--force", name]); }
    finally {
      try { if (probeAttempted) await exec("docker", ["rm", "--force", probeName]); }
      finally { if (volumeCreated) await exec("docker", ["volume", "rm", volume]); }
    }
  }
}

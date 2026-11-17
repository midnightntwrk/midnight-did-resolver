// This file is part of midnightntwrk/midnight-did-resolver.
// SPDX-License-Identifier: Apache-2.0
// All external CLIs are recording stubs; HTTP is loopback only.
import assert from "node:assert/strict";
import { createServer } from "node:http";
import { chmod, mkdtemp, readFile, writeFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";
import { argvExec, execute, githubAPI, hash, openBundle, pins, promotePair, provisionedRecord, sealBundle, signatureArgs, verifyPublishedPair } from "./release-execution.mjs";
import { checkHTTP, runtimeSmoke, validateRuntimeFixture } from "./release-runtime-smoke.mjs";
import { repository, workflow, resolveContext } from "./release-context.mjs";
import { composeSpecification, dependencyImages, mappedPort, withRuntimeFixture } from "./release-standalone-fixture.mjs";
import { deployFixture, validateDeploymentConfig } from "./release-fixture-deploy.mjs";
const sha = "a".repeat(40);
const context = resolveContext({
  GITHUB_EVENT_NAME: "workflow_dispatch", GITHUB_REF_TYPE: "branch", GITHUB_REF: "refs/heads/develop",
  GITHUB_REPOSITORY: repository, GITHUB_SHA: sha, GITHUB_WORKFLOW_REF: `${workflow}@refs/heads/develop`,
  GITHUB_WORKFLOW_SHA: sha, INPUT_EXPECTED_SOURCE_SHA: sha, INPUT_CHANNEL: "rc",
  INPUT_VERSION: "1.0.0-rc.1", INPUT_PUBLISH: "true",
}, Array(4).fill("1.0.0"));
const env = { GITHUB_RUN_ID: "123", GITHUB_RUN_ATTEMPT: "2", GITHUB_ACTOR: "fixture-actor", GITHUB_TOKEN: "fixture-secret" };
const fixture = { schemaVersion: 1, kind: "standalone-external-state", did: "did:midnight:undeployed:public-fixture",
  expectedResolution: { didDocument: { id: "did:midnight:undeployed:public-fixture", verificationMethod: [{ id: "did:midnight:undeployed:public-fixture#public-fixture" }] }, didResolutionMetadata: { contentType: "application/did+ld+json", error: null } },
  indexerHTTP: "http://127.0.0.1:8088/api/v4/graphql", indexerWS: "ws://127.0.0.1:8088/api/v4/graphql/ws" };
const roots = ["resolver", "manager"].map(service => {
  const bytes = Buffer.from(JSON.stringify({ schemaVersion: 2, service }));
  return { service, repository: context[`${service}_repository`], indexDigest: `sha256:${hash(bytes)}`, bytes };
});
const evidence = () => ({ schemaVersion: 1, ...env, runID: env.GITHUB_RUN_ID, runAttempt: env.GITHUB_RUN_ATTEMPT,
  sourceSHA: sha, verified: true, graphs: roots.map(({ bytes, ...g }) => g),
  scanReceipts: [1, 2, 3, 4], runtimeReceipts: [1, 2, 3, 4], signatureReceipts: [1, 2] });
const assertEvidence = (e, c) => {
  // Orchestration fixture contract, NOT a substitute for core's graph/content policy.
  assert.equal(e.sourceSHA, c.source_sha); assert.equal(e.verified, true);
  for (const [key, count] of [["graphs", 2], ["scanReceipts", 4], ["runtimeReceipts", 4], ["signatureReceipts", 2]]) assert.equal(e[key].length, count);
};
function promotionHarness({ failService, ambiguous = false, latestFail, stable = false, versionDrift = false } = {}) {
  const log = [], tags = new Map(), reconciled = [], latest = new Map();
  const selectedContext = stable ? { ...context, channel: "release", version: "1.0.0", ref: "refs/heads/main" } : context;
  const clients = Object.fromEntries(roots.map(g => [g.service, {
    async assertVersionAbsent(v) { log.push(["absent", g.service, v]); assert.ok(!tags.has(g.service), "occupied"); },
    async getManifest(ref) {
      log.push(["get", g.service, ref]);
      if (ref === selectedContext.version && !tags.has(g.service)) throw new Error("not found");
      return { bytes: g.bytes, digest: versionDrift && latest.size && ref === selectedContext.version ? "sha256:" + "0".repeat(64) : tags.get(g.service) || g.indexDigest, mediaType: "application/vnd.oci.image.index.v1+json" };
    },
    async getLatestManifest() { log.push(["latest-get", g.service]); const value = latest.get(g.service); return value ? { bytes: g.bytes, digest: value } : null; },
    async promoteStableLatest(v, raw) {
      log.push(["latest-put", g.service, v]); assert.equal(stable, true); assert.equal(v, "1.0.0");
      assert.equal(tags.size, 2); assert.equal(raw.bytes, g.bytes);
      const previousDigest = latest.get(g.service) || null;
      if (latestFail === g.service) { if (ambiguous) latest.set(g.service, g.indexDigest); throw Object.assign(new Error("ambiguous latest"), { previousDigest }); }
      latest.set(g.service, g.indexDigest);
      return { repository: g.repository, version: v, tag: "latest", digest: g.indexDigest, previousDigest, confirmed: true };
    },
    async promoteIndex(v, raw) {
      log.push(["put", g.service, v]); assert.equal(raw.bytes, g.bytes);
      if (failService === g.service) { if (ambiguous) tags.set(g.service, g.indexDigest); throw new Error("ambiguous"); }
      tags.set(g.service, g.indexDigest);
      return { repository: g.repository, version: v, digest: g.indexDigest, confirmed: true };
    },
  }]));
  return { log, tags, latest, reconciled, clients, boundary: async () => { log.push(["boundary"]); },
    verifySignature: async g => { log.push(["signature", g.service]); },
    reconcile: async observation => { reconciled.push(observation); }, assertEvidence, context: selectedContext, evidence: evidence() };
}
function nativeProbeStub(args) {
  if (!args.includes("/release-runtime-consumer.mjs")) return null;
  return { stdout: JSON.stringify({ schemaVersion: 1, verified: true, service: args.at(-2), node: "v24.22.3", uid: 1000,
    processArch: args.at(-1), ledgerWasmRoundtrip: true, hdRole4Derived: true, classicLevel: "open-put-get-close-reopen-get-close",
    artifactVersion: "0.21.0", verifierReads: 12, rejectedIncompatibleRuntime: true, storageSignVerify: true,
    artifacts: Array.from({ length: 36 }, (_, i) => ({ circuit: "fixture-" + Math.floor(i / 3), kind: ["prover", "verifier", "zkir"][i % 3], bytes: 1, sha256: "a".repeat(64) })) }) };
}
test("prepare-only never promotes; all prepared gates precede first final PUT", async () => {
  const h = promotionHarness();
  await assert.rejects(promotePair({ ...h, context: { ...context, publish: "false" } }), /Prepare-only/);
  assert.equal(h.log.length, 0);
  for (const key of ["graphs", "scanReceipts", "runtimeReceipts", "signatureReceipts"]) {
    const x = promotionHarness(); x.evidence[key].pop();
    await assert.rejects(promotePair(x));
    assert.equal(x.log.length, 0, key);
  }
  const mismatch = promotionHarness(); mismatch.evidence.sourceSHA = "b".repeat(40);
  await assert.rejects(promotePair(mismatch)); assert.equal(mismatch.log.length, 0);
  const sig = promotionHarness(); sig.verifySignature = async () => { throw new Error("signature gate"); };
  await assert.rejects(promotePair(sig)); assert.equal(sig.log.length, 0);
  const result = await promotePair(h); assert.equal(result.complete, true);
  const firstPUT = h.log.findIndex(x => x[0] === "put");
  assert.equal(h.log.slice(0, firstPUT).filter(x => x[0] === "signature").length, 2);
  for (const service of ["resolver", "manager"]) assert.ok(h.log.slice(0, firstPUT).some(x => x[0] === "absent" && x[1] === service));
  assert.deepEqual(h.log.filter(x => x[0] === "put").map(x => x[2]), [context.version, context.version]);
});
test("occupied pair/own tag and corrupt bytes stop writes; partial/ambiguous failures only reconcile", async () => {
  const occupied = promotionHarness(); occupied.tags.set("manager", roots[1].indexDigest);
  await assert.rejects(promotePair(occupied)); assert.equal(occupied.log.some(x => x[0] === "put"), false);
  const corrupt = promotionHarness();
  corrupt.clients.resolver.getManifest = async () => ({ bytes: Buffer.from("changed"), digest: roots[0].indexDigest });
  await assert.rejects(promotePair(corrupt)); assert.equal(corrupt.log.some(x => x[0] === "put"), false);
  for (const ambiguous of [false, true]) {
    const h = promotionHarness({ failService: "manager", ambiguous });
    await assert.rejects(promotePair(h), /Incomplete/);
    assert.equal(h.log.filter(x => x[0] === "put").length, 2);
    assert.equal(h.reconciled.length, 1);
    assert.equal(h.reconciled[0].incompleteDraft, true);
    assert.equal(h.tags.get("resolver"), roots[0].indexDigest);
  }
});

test("stable latest runs only after BOTH immutable version readbacks; previous latest is recorded", async () => {
  const h = promotionHarness({ stable: true });
  const result = await promotePair(h);
  assert.equal(result.complete, true);
  assert.equal(result.latestReceipts.length, 2);
  assert.deepEqual(result.previousLatest, [{ service: "resolver", digest: null }, { service: "manager", digest: null }]);
  const firstLatest = h.log.findIndex(x => x[0] === "latest-put");
  for (const service of ["resolver", "manager"]) {
    assert.ok(h.log.slice(0, firstLatest).some(x => x[0] === "get" && x[1] === service && x[2] === "1.0.0"));
    assert.ok(h.log.slice(0, firstLatest).some(x => x[0] === "put" && x[1] === service));
  }
  assert.equal(h.log.filter(x => x[0] === "latest-put").length, 2);
});
test("RC has zero latest calls; invalid branch/channel/version has zero registry mutations", async () => {
  const h = promotionHarness(); await promotePair(h);
  assert.equal(h.log.some(x => x[0].startsWith("latest")), false);
  for (const patch of [
    { channel: "release" }, { channel: "release", version: "1.0.0", ref: "refs/heads/develop" },
    { channel: "release", version: "1.0.0", ref: "refs/heads/main-evil" }, { channel: "snapshot" },
    { channel: "rc", version: "1.0.0" }, { channel: "rc", ref: "refs/tags/develop" },
  ]) {
    const bad = promotionHarness(); bad.context = { ...bad.context, ...patch };
    await assert.rejects(promotePair(bad)); assert.equal(bad.log.length, 0);
  }
});
for (const ambiguous of [false, true]) test("partial stable latest is incomplete; no automatic retry, ambiguous=" + ambiguous, async () => {
  const h = promotionHarness({ stable: true, latestFail: "manager", ambiguous });
  await assert.rejects(promotePair(h), /Incomplete/);
  assert.equal(h.tags.size, 2); assert.equal(h.log.filter(x => x[0] === "latest-put").length, 2);
  assert.equal(h.reconciled[0].incompleteDraft, true);
  assert.equal(h.reconciled[0].observedLatest.length, 2);
  assert.equal(h.reconciled[0].latestReceipts.length, 1);
});
test("final independent stable readback rejects drift; RC cannot inspect latest", async () => {
  const h = promotionHarness({ stable: true }); await promotePair(h);
  h.latest.set("manager", "sha256:" + "f".repeat(64));
  await assert.rejects(verifyPublishedPair({ ...h, latest: true }), /latest pair/);
  assert.equal(h.log.filter(x => x[0] === "latest-put").length, 2);
  const rc = promotionHarness();
  await assert.rejects(verifyPublishedPair({ ...rc, latest: true }), /RC/); assert.equal(rc.log.length, 0);
});
test("changed immutable version before second latest blocks second latest and final success", async () => {
  const h = promotionHarness({ stable: true, versionDrift: true });
  await assert.rejects(promotePair(h), /Incomplete/);
  assert.equal(h.log.filter(x => x[0] === "latest-put").length, 1);
});
test("compose is digest pinned, three dependencies only, dynamic loopback ports and exact standalone preset", async () => {
  const spec = composeSpecification();
  assert.deepEqual(Object.keys(spec.services), ["node", "indexer", "proof-server"]);
  for (const service of Object.values(spec.services)) {
    assert.match(service.image, /@sha256:[a-f0-9]{64}$/); assert.equal(service.build, undefined);
    assert.equal(service.container_name, undefined); assert.equal(service.privileged, undefined);
    assert.equal(service.platform, "linux/amd64");
    assert.equal(service.ports[0].host_ip, "127.0.0.1"); assert.equal(service.ports[0].published, "0");
    assert.ok(service.healthcheck.test.length);
  }
  assert.equal(spec.services.node.environment.CFG_PRESET, "dev");
  assert.equal(spec.services.indexer.environment.APP__APPLICATION__NETWORK_ID, "undeployed");
  const envExample = await readFile(new URL("../infrastructure/standalone.env.example", import.meta.url), "utf8");
  assert.equal(spec.services.indexer.environment.APP__INFRA__SECRET, envExample.match(/INDEXER_SECRET_PREFIX:-([^}]+)/)[1] + envExample.match(/INDEXER_SECRET_SUFFIX:-([^}]+)/)[1]);
  for (const input of ["0.0.0.0:9944", "127.0.0.1:0", "127.0.0.1:65536", "::1:1234", "garbage"]) assert.throws(() => mappedPort(input));
  assert.equal(mappedPort("127.0.0.1:12345\n"), 12345);
});
function fixtureHarness({ startupFailure = false, deploymentFailure = false } = {}) {
  const calls = [], saved = new Map();
  const exec = async (cmd, args) => {
    assert.equal(cmd, "docker"); calls.push(args);
    if (args.includes("up") && startupFailure) throw new Error("startup fixture failure");
    if (args.includes("port")) {
      const service = args[args.indexOf("port") + 1]; return { stdout: "127.0.0.1:" + ({ node: 19944, indexer: 18088, "proof-server": 16300 }[service]) + "\n" };
    }
    if (args.includes("/release-fixture-deploy.mjs")) {
      if (deploymentFailure) throw new Error("deployment fixture failure");
      return { stdout: JSON.stringify({ ...fixture, indexerHTTP: "http://127.0.0.1:18088/api/v4/graphql",
        indexerWS: "ws://127.0.0.1:18088/api/v4/graphql/ws" }) };
    }
    return { stdout: "[]" };
  };
  const managerGraph = { service: "manager", repository: roots[1].repository,
    platforms: [{ platform: "linux/amd64", manifestDigest: roots[1].indexDigest }] };
  return { calls, saved, exec, managerGraph, save: async (name, content) => saved.set(name, content) };
}
test("managed fixture deploys INSIDE verified manager child; external state survives callback and finally cleans ONLY own project", async () => {
  const h = fixtureHarness();
  await withRuntimeFixture({ ...h, run: async f => {
    assert.equal(f.deployment.digestPullVerified, true);
    assert.equal(f.deployment.pulledReference, roots[1].repository + "@" + roots[1].indexDigest);
    assert.equal(h.calls.some(a => a.includes("down")), false);
    assert.deepEqual(f.dependencyImages, dependencyImages);
  } });
  const composeCalls = h.calls.filter(a => a[0] === "compose");
  assert.equal(new Set(composeCalls.map(a => a[a.indexOf("--project-name") + 1])).size, 1);
  assert.equal(new Set(composeCalls.map(a => a[a.indexOf("--file") + 1])).size, 1);
  assert.ok(composeCalls.at(-1).includes("down"));
  assert.equal(h.saved.get("standalone-cleanup.json").cleaned, true);
  const deploy = h.calls.find(a => a[0] === "run");
  assert.ok(deploy.some(a => a.endsWith("dst=/release-fixture-deploy.mjs,readonly")));
  assert.equal(deploy.some(a => /GITHUB_TOKEN|GH_TOKEN|seed/i.test(a)), false);
  assert.equal(deploy[deploy.indexOf("--network") + 1], "host");
  assert.equal(h.calls.filter(a => a.includes("up")).length, 1);
});
for (const failure of ["startup", "deployment", "consumer"]) test("fixture failure cleans exact owned resources without whole-gate retries: " + failure, async () => {
  const h = fixtureHarness({ startupFailure: failure === "startup", deploymentFailure: failure === "deployment" });
  await assert.rejects(withRuntimeFixture({ ...h, run: async () => { throw new Error("consumer fixture failure"); } }));
  assert.equal(h.calls.filter(a => a.includes("up")).length, 1);
  assert.equal(h.calls.filter(a => a.includes("down")).length, 1);
  assert.equal(h.saved.get("standalone-cleanup.json").cleaned, true);
});

const deploymentConfig = { schemaVersion: 1, indexer: "http://127.0.0.1:18088/api/v4/graphql",
  indexerWS: "ws://127.0.0.1:18088/api/v4/graphql/ws", node: "http://127.0.0.1:19944", proofServer: "http://127.0.0.1:16300" };
function publicAPIRecordingFixture({ dustFailures = 0, createFailure = false, noResult = false } = {}) {
  const calls = [], did = "did:midnight:undeployed:recording", wallet = { stop: async () => calls.push("wallet.stop") };
  let createAttempts = 0, clock = 0, method;
  const api = {
    setLogger: () => calls.push("logger.silent"), StandaloneConfig: class {},
    buildWalletAndWaitForFunds: async (cfg, seed) => { calls.push("wallet.fund"); assert.equal(seed, "0".repeat(63) + "1"); assert.equal(cfg.indexer, deploymentConfig.indexer); return { wallet, unshieldedKeystore: {} }; },
    registerForDustGeneration: async (w, k, opts) => { calls.push("dust.register"); assert.equal(opts.timeoutMs, 120000); },
    createIndexerIndexedBlockTime: () => "recording-indexed-time",
    configureProviders: async () => { calls.push("providers"); return { publicDataProvider: { dispose: async () => calls.push("providers.dispose") } }; },
    initPrivateState: async () => { calls.push("private-state"); return {}; },
    createDID: async () => {
      calls.push("did.create"); createAttempts++;
      if (createFailure) throw new Error("not retryable");
      if (createAttempts <= dustFailures) throw new Error("Not enough Dust generated to pay the fee");
      return { deployTxData: { public: { contractAddress: "recording-address" } } };
    },
    getMidnightNetwork: () => "undeployed",
    addVerificationMethod: async (contract, providers, value) => { calls.push("method.add"); method = value; },
  };
  const modules = {
    "@midnight-ntwrk/midnight-did-api": api,
    "@midnight-ntwrk/midnight-did": { parseContractAddress: value => { assert.equal(value, "recording-address"); return value; },
      createMidnightDIDString: () => did, MidnightNetwork: { Undeployed: "undeployed" },
      MidnightDIDResolver: class { constructor(opts) { this.opts = opts; } async resolveResult(value) {
        assert.equal(value, did); await this.opts.ledgerReader("recording-address");
        return noResult ? null : { didDocument: { id: did, verificationMethod: [method] }, didDocumentMetadata: { versionId: "1" } };
      } } },
    "@midnight-ntwrk/midnight-did-domain": { createVerificationMethod: value => value, VerificationMethodType: { JsonWebKey: "JsonWebKey" }, KeyType: { OKP: "OKP" }, CurveType: { Ed25519: "Ed25519" } },
    "@midnight-ntwrk/compact-runtime": { ContractState: "v9-owning-decoder" },
    "@midnight-ntwrk/midnight-did-contract": { DIDContract: { ledger: data => data } },
    "@midnight-ntwrk/midnight-js-indexer-public-data-provider": { indexerPublicDataProvider: () => ({ dispose: async () => calls.push("reference.dispose"), queryContractState: async () => ({ contractState: { deserialize: ({ v9 }) => { assert.equal(v9, "v9-owning-decoder"); return { data: "recording-ledger" }; } } }) }) },
    pino: { default: () => ({}) },
  };
  return { calls, options: { load: async name => { assert.ok(modules[name], "Only known public exports"); return modules[name]; },
    runtime: { version: "v24.22.3", arch: "x64", getuid: () => 1000 },
    sleepImpl: async ms => { calls.push(["sleep", ms]); clock += ms; }, now: () => clock } };
}
test("public API deployment recording follows tested sequence, Ledger9 decode and positive method; ONLY Dust 2x8s retries", async () => {
  const h = publicAPIRecordingFixture({ dustFailures: 2 });
  const result = await deployFixture(deploymentConfig, h.options);
  assert.equal(validateRuntimeFixture(result), result);
  assert.deepEqual(h.calls.filter(x => typeof x === "string"), ["logger.silent", "wallet.fund", "dust.register", "providers", "private-state", "did.create", "did.create", "did.create", "method.add", "reference.dispose", "providers.dispose", "wallet.stop"]);
  assert.deepEqual(h.calls.filter(Array.isArray), [["sleep", 8000], ["sleep", 8000]]);
});
test("non-Dust failure does not retry; wallet always stops; missing positive state never becomes fixture", async () => {
  for (const options of [{ createFailure: true }, { noResult: true }, { dustFailures: 3 }]) {
    const h = publicAPIRecordingFixture(options);
    await assert.rejects(deployFixture(deploymentConfig, h.options));
    assert.equal(h.calls.filter(x => x === "wallet.stop").length, 1);
    if (options.createFailure) assert.equal(h.calls.filter(x => x === "did.create").length, 1);
    if (options.dustFailures) assert.equal(h.calls.filter(x => x === "did.create").length, 3);
  }
});
test("deployer endpoints reject non-loopback/public/credential protocols before any SDK API", () => {
  for (const patch of [{ node: "https://public.example" }, { indexer: "http://token@127.0.0.1/x" }, { indexerWS: "wss://127.0.0.1/ws" }]) assert.throws(() => validateDeploymentConfig({ ...deploymentConfig, ...patch }));
});
test("external fixture override needs explicit approval; valid approved override never starts Docker", async () => {
  const h = fixtureHarness();
  await assert.rejects(withRuntimeFixture({ ...h, externalFixture: fixture, run: async () => {} }), /approval/);
  await withRuntimeFixture({ ...h, externalFixture: fixture, externalFixtureApproved: true, run: async f => assert.equal(f, fixture) });
  assert.equal(h.calls.length, 0);
});
test("artifact manifest checksums/source/run/attempt/member set fail closed", async () => {
  const dir = await mkdtemp(join(tmpdir(), "release-bundle-test-"));
  try {
    await writeFile(join(dir, "evidence.json"), JSON.stringify(evidence()));
    const checksum = await sealBundle(dir, context, env);
    assert.equal((await openBundle(dir, checksum, context, env)).sourceSHA, sha);
    for (const [digest, c, e] of [["0".repeat(64), context, env], [checksum, { ...context, source_sha: "b".repeat(40) }, env],
      [checksum, context, { ...env, GITHUB_RUN_ATTEMPT: "3" }], [checksum, context, { ...env, GITHUB_RUN_ID: "456" }]]) {
      await assert.rejects(openBundle(dir, digest, c, e));
    }
    await writeFile(join(dir, "unlisted.txt"), "no");
    await assert.rejects(openBundle(dir, checksum, context, env), /Unlisted/);
    await rm(join(dir, "unlisted.txt"));
    await writeFile(join(dir, "evidence.json"), "{}");
    await assert.rejects(openBundle(dir, checksum, context, env), /checksum/);
  } finally { await rm(dir, { recursive: true, force: true }); }
});
test("GitHub unknown API errors never mean absent; precreation is mandatory and no create endpoints", async () => {
  for (const status of [401, 403, 404, 409, 429, 500, 502]) {
    const api = githubAPI({ token: "fixture-secret", fetchImpl: async () => new Response("not proof", { status }) });
    await assert.rejects(api(`/repos/${repository}/releases`), new RegExp(String(status)));
  }
  const calls = [];
  const api = githubAPI({ token: "fixture-secret", fetchImpl: async (url, options) => {
    calls.push([url, options]); return new Response("{}", { headers: { "content-type": "application/json" } });
  } });
  await assert.rejects(api(`/repos/${repository}/releases`, { method: "POST", body: {} }), /Only prepared/);
  await assert.rejects(api("/repos/attacker/repo/releases"), /Noncanonical/);
  await api(`/repos/${repository}/releases`);
  assert.equal(calls[0][1].redirect, "error");
  assert.equal(calls.length, 1);
  const draft = { id: 12, draft: true, prerelease: true, tag_name: "v1.0.0-rc.1", target_commitish: sha, assets: [] };
  const getter = async (path) => path.includes("/git/ref/") ? { object: { type: "commit", sha } } : [draft];
  assert.equal((await provisionedRecord(getter, context)).id, 12);
  for (const patch of [{ target_commitish: "develop" }, { draft: false }, { prerelease: false }]) {
    await assert.rejects(provisionedRecord(async p => p.includes("/git/ref/") ? { object: { type: "commit", sha } } : [{ ...draft, ...patch }], context));
  }
  await assert.rejects(provisionedRecord(async p => p.includes("/git/ref/") ? { object: { type: "commit", sha: "b".repeat(40) } } : [draft], context), /Git tag/);
  await assert.rejects(provisionedRecord(async () => { throw new Error("unavailable"); }, context));
});
test("prepare-only record step validates evidence without any GitHub writes", async () => {
  const dir = await mkdtemp(join(tmpdir(), "prepare-only-test-"));
  const prepareContext = { ...context, publish: "false" };
  try {
    await writeFile(join(dir, "evidence.json"), JSON.stringify(evidence()));
    const checksum = await sealBundle(dir, prepareContext, env);
    const core = { createRegistryClient: () => ({}), assertPreparedEvidence: assertEvidence };
    await execute("record-prepare", { env: { ...env, EVIDENCE_DIR: dir, EXPECTED_EVIDENCE_SHA256: checksum },
      boundary: async () => prepareContext, core, fetchImpl: async () => { throw new Error("No external API allowed"); } });
    for (const command of ["promote", "record-finalize"]) await assert.rejects(execute(command, {
      env: { ...env, EVIDENCE_DIR: dir, EXPECTED_EVIDENCE_SHA256: checksum },
      boundary: async () => prepareContext, core, fetchImpl: async () => { throw new Error("No external API allowed"); },
    }), /Prepare-only/);
  } finally { await rm(dir, { recursive: true, force: true }); }
});
test("source boundary failure runs no tools, API requests, signing or final record operations", async () => {
  for (const command of ["install-tools", "preflight", "verify", "record-prepare", "promote", "record-finalize"]) {
    await assert.rejects(execute(command, {
      boundary: async () => { throw new Error("source mismatch"); },
      exec: async () => assert.fail("tool reached"), fetchImpl: async () => assert.fail("network reached"),
    }), /source mismatch/);
  }
});
test("literal Cosign identity/SHA/event; actual child CLI receives metacharacters as one argv", async () => {
  const args = signatureArgs(context, roots[0]);
  assert.deepEqual(args, ["verify", `${roots[0].repository}@${roots[0].indexDigest}`,
    "--certificate-identity", context.identity, "--certificate-oidc-issuer", context.issuer,
    "--certificate-github-workflow-sha", sha, "--certificate-github-workflow-trigger", "workflow_dispatch"]);
  const dir = await mkdtemp(join(tmpdir(), "release-argv-test-"));
  const oldPath = process.env.PATH, oldToken = process.env.GITHUB_TOKEN;
  try {
    const stub = join(dir, "docker");
    await writeFile(stub, `#!${process.execPath}\nconsole.log(JSON.stringify({args:process.argv.slice(2),hasToken:!!process.env.GITHUB_TOKEN}));\n`);
    await chmod(stub, 0o755);
    process.env.PATH = `${dir}:${oldPath}`; process.env.GITHUB_TOKEN = "fixture-secret";
    const payload = ";touch never-run $(echo forbidden) \\n";
    const result = JSON.parse((await argvExec("docker", ["inspect", payload])).stdout);
    assert.deepEqual(result.args, ["inspect", payload]); assert.equal(result.hasToken, false);
    await assert.rejects(argvExec("bash", ["-c", "true"]), /Unsupported/);
  } finally {
    process.env.PATH = oldPath;
    if (oldToken === undefined) delete process.env.GITHUB_TOKEN; else process.env.GITHUB_TOKEN = oldToken;
    await rm(dir, { recursive: true, force: true });
  }
});
test("runtime fixture threshold is a positive external-state resolution, never missing/empty/public network", () => {
  assert.equal(validateRuntimeFixture(fixture), fixture);
  for (const f of [null, {}, { ...fixture, kind: "mainnet" }, { ...fixture, expectedResolution: {} },
    { ...fixture, indexerHTTP: "https://public.example/graphql" }, { ...fixture, indexerWS: "ws://token@127.0.0.1/x" }]) assert.throws(() => validateRuntimeFixture(f));
});
async function httpFixture(run) {
  let selected = false;
  const server = createServer(async (req, res) => {
    res.setHeader("content-type", "application/json");
    const path = new URL(req.url, "http://127.0.0.1").pathname;
    if (path === "/health") return res.end(JSON.stringify({ status: "ok" }));
    if (path === "/ready") return res.end(JSON.stringify({ status: "ready" }));
    if (path === "/docs/json") return res.end(JSON.stringify({ openapi: "3.0.3", info: { version: "1.0.0" } }));
    if (path === "/docs/") { res.setHeader("content-type", "text/html"); return res.end('<link href="./static/swagger-ui.css"><script src="./static/swagger-ui-bundle.js"></script>'); }
    if (path.startsWith("/docs/static/")) return res.end("fixture asset ".repeat(30));
    if (path === "/api/profiles/select" && req.method === "POST") { let body = ""; for await (const chunk of req) body += chunk; selected = JSON.parse(body).name === "release-public-fixture"; return res.end('{"ok":true}'); }
    if (path === "/api/profiles") return res.end(JSON.stringify({ ok: true, data: { profile: "standalone", activeProfileName: selected ? "release-public-fixture" : "default", availableProfileNames: selected ? ["default", "release-public-fixture"] : ["default"] } }));
    if (path.startsWith("/resolve/")) return res.end(JSON.stringify(fixture.expectedResolution));
    res.writeHead(404); res.end("{}");
  });
  await new Promise(ok => server.listen(0, "127.0.0.1", ok));
  try { await run(`http://127.0.0.1:${server.address().port}`, { reset: () => { selected = false; } }); }
  finally { await new Promise(ok => server.close(ok)); }
}
test("loopback HTTP fixtures exercise health/ready/OpenAPI/swagger and real profile select/list", async () => {
  await httpFixture(async base => {
    await assert.rejects(checkHTTP(base, { service: "manager", fixture, selected: true }));
    await checkHTTP(base, { service: "manager", fixture });
    await checkHTTP(base, { service: "manager", fixture, selected: true });
    await checkHTTP(base, { service: "resolver", fixture });
    await assert.rejects(checkHTTP(base, { service: "resolver", fixture: { ...fixture, expectedResolution: {} } }));
  });
});
test("runtime lifecycle recording stub pins each child/platform/default user and reuses SAME mount", async () => {
  await httpFixture(async base => {
    for (const service of ["resolver", "manager"]) for (const platform of ["linux/amd64", "linux/arm64"]) {
      const graph = roots.find(g => g.service === service), calls = [];
      const fakeFetch = (url, options) => fetch(new URL(new URL(url).pathname, base), options);
      const exec = async (cmd, args) => {
        assert.equal(cmd, "docker"); calls.push(args);
        if (nativeProbeStub(args)) return nativeProbeStub(args);
        if (args[0] === "image") return { stdout: JSON.stringify([{ Architecture: platform.split("/")[1], Config: { User: "node", Env: ["HOME=/home/node"] } }]) };
        if (args[0] === "exec") return { stdout: JSON.stringify({ node: "v24.22.3", uid: 1000, arch: platform === "linux/amd64" ? "x64" : "arm64", data: "/home/node/.midnight-did" }) };
        return { stdout: "" };
      };
      const receipt = await runtimeSmoke({ repository: graph.repository, platform, manifestDigest: graph.indexDigest, fixture, exec, fetchImpl: fakeFetch, hostArch: "x64" });
      assert.equal(receipt.verified, true);
      assert.equal(receipt.digestPullVerified, true);
      assert.equal(receipt.pulledReference, `${graph.repository}@${graph.indexDigest}`);
      assert.match(receipt.probeScriptSHA256, /^[a-f0-9]{64}$/);
      assert.equal(receipt.architectureMode, platform === "linux/amd64" ? "native-amd64" : "qemu-arm64");
      const runs = calls.filter(a => a[0] === "run" && a.includes("--detach")); assert.equal(runs.length, 2);
      assert.deepEqual(runs[0], runs[1]);
      assert.equal(runs[0].at(-1), `${graph.repository}@${graph.indexDigest}`);
      assert.equal(runs[0][runs[0].indexOf("--network") + 1], "host");
      assert.ok(runs[0].some(a => a.endsWith("_HOST=127.0.0.1")));
      assert.equal(calls.filter(a => a[0] === "restart").length, 1);
      assert.equal(calls.some(a => ["build", "compose"].includes(a[0])), false);
      const init = calls.find(a => a[0] === "run" && a.includes("--rm"));
      assert.equal(init[init.indexOf("--network") + 1], "none");
      assert.ok(init.at(-1).includes("1000,-1"));
      assert.equal(init.at(-1).includes("0o777"), false);
      assert.equal(calls.filter(a => a[0] === "volume" && a[1] === "rm").length, 1);
    }
  });
});
test("profile metadata lost on restart/recreate fails; no empty-directory persistence pass", async () => {
  for (const loss of ["restart", "recreate"]) {
    await httpFixture(async (base, state) => {
      let runs = 0;
      const exec = async (cmd, args) => {
        if (nativeProbeStub(args)) return nativeProbeStub(args);
        if (args[0] === "image") return { stdout: JSON.stringify([{ Architecture: "amd64", Config: { User: "node", Env: ["HOME=/home/node"] } }]) };
        if (args[0] === "exec") return { stdout: JSON.stringify({ node: "v24.22.3", uid: 1000, arch: "x64", data: "/home/node/.midnight-did" }) };
        if (args[0] === "restart" && loss === "restart") state.reset();
        if (args[0] === "run" && args.includes("--detach") && ++runs === 2 && loss === "recreate") state.reset();
        return { stdout: "" };
      };
      await assert.rejects(runtimeSmoke({ repository: roots[1].repository, platform: "linux/amd64", manifestDigest: roots[1].indexDigest,
        fixture, exec, fetchImpl: (url, options) => fetch(new URL(new URL(url).pathname, base), options), hostArch: "x64" }));
    });
  }
});

test("failed exact child pull issues no native/runtime run and cannot emit digest-pull receipt", async () => {
  const calls = [];
  await assert.rejects(runtimeSmoke({ repository: roots[0].repository, platform: "linux/arm64",
    manifestDigest: roots[0].indexDigest, fixture, hostArch: "x64", exec: async (cmd, args) => {
      calls.push(args); throw new Error("recording pull failure");
    } }));
  assert.equal(calls.length, 1); assert.equal(calls[0][0], "pull");
});
test("native loader-only or missing WASM/storage/artifact execution cannot become a verified runtime receipt", async () => {
  for (const patch of [{ ledgerWasmRoundtrip: false }, { hdRole4Derived: false }, { classicLevel: "imported" },
    { verifierReads: 0 }, { artifacts: [] }, { storageSignVerify: false }]) {
    const calls = [];
    await assert.rejects(runtimeSmoke({ repository: roots[1].repository, platform: "linux/amd64",
      manifestDigest: roots[1].indexDigest, fixture, hostArch: "x64", exec: async (cmd, args) => {
        calls.push(args);
        if (args[0] === "image") return { stdout: JSON.stringify([{ Architecture: "amd64", Config: { User: "node", Env: ["HOME=/home/node"] } }]) };
        const probe = nativeProbeStub(args);
        if (probe) return { stdout: JSON.stringify({ ...JSON.parse(probe.stdout), ...patch }) };
        return { stdout: "" };
      } }));
    assert.equal(calls.some(a => a.includes("--detach")), false);
  }
});
test("public consumer and deploy helpers use candidate cwd exports, not producer paths or host deps", async () => {
  const consumer = await readFile(new URL("./release-runtime-consumer.mjs", import.meta.url), "utf8");
  const deploy = await readFile(new URL("./release-fixture-deploy.mjs", import.meta.url), "utf8");
  for (const text of [consumer, deploy]) {
    assert.match(text, /createRequire\(.*process.cwd\(\)/);
    assert.match(text, /import\(require.resolve\(name\)\)/);
    assert.doesNotMatch(text, /import\([^)]*src\/|libs\/midnight|npm (?:ci|install)/);
  }
  for (const required of ["entryPointHash", "encodeRawTokenType", "deriveKeyAt", "db.put", "db.get", "reader.getVerifierKeys", "checkRuntimeVersion"]) assert.ok(consumer.includes(required));
});
test("runtime architecture/user/version mismatch cannot become a passing receipt", async () => {
  for (const patch of [{ arch: "x64" }, { uid: 0 }, { node: "v24.0.0" }]) {
    await httpFixture(async base => {
      const exec = async (cmd, args) => {
        if (nativeProbeStub(args)) return nativeProbeStub(args);
        if (args[0] === "image") return { stdout: JSON.stringify([{ Architecture: "arm64", Config: { User: "node", Env: ["HOME=/home/node"] } }]) };
        if (args[0] === "exec") return { stdout: JSON.stringify({ node: "v24.22.3", uid: 1000, arch: "arm64", ...patch }) };
        return { stdout: "" };
      };
      await assert.rejects(runtimeSmoke({ repository: roots[0].repository, platform: "linux/arm64", manifestDigest: roots[0].indexDigest,
        fixture, exec, fetchImpl: (url, options) => fetch(new URL(new URL(url).pathname, base), options), hostArch: "x64" }));
    });
  }
});

// This file is part of midnightntwrk/midnight-did-resolver.
// SPDX-License-Identifier: Apache-2.0
// Privileged release consumers: Node stdlib only. No shell command interpolation.
import assert from "node:assert/strict";
import { execFile } from "node:child_process";
import { createHash } from "node:crypto";
import { readFile, writeFile, mkdir, readdir, lstat } from "node:fs/promises";
import { appendFileSync } from "node:fs";
import { resolve, join } from "node:path";
import { fileURLToPath } from "node:url";
import { promisify } from "node:util";
import { resolveContext, checkBoundary, manifests, repository, validateDigests } from "./release-context.mjs";
import { runtimeSmoke, validateRuntimeFixture } from "./release-runtime-smoke.mjs";
import { withRuntimeFixture } from "./release-standalone-fixture.mjs";
const execFileAsync = promisify(execFile);
const services = ["resolver", "manager"];
export const pins = Object.freeze({
  buildx: "v0.30.1",
  buildkit: "moby/buildkit:v0.25.1@sha256:79cc6476ab1a3371c9afd8b44e7c55610057c43e18d9b39b68e2b0c2475cc1b6",
  trivyVersion: "0.76.2",
  trivyURL: "https://github.com/aquasecurity/trivy/releases/download/v0.76.2/trivy_0.76.2_Linux-64bit.tar.gz",
  trivySHA256: "bc0fc726e017b8927a62b2d3531d7a84ee01769daf5c99ed866dced54089fdbf",
  trivyChecksumsURL: "https://github.com/aquasecurity/trivy/releases/download/v0.76.2/trivy_0.76.2_checksums.txt",
  trivyChecksumsSHA256: "802e4ac1418ed967cd3c4b901f4f71ca00b7363ede67778665a22d9f13f58c83",
  cosign: "v2.6.1",
});
export const hash = bytes => createHash("sha256").update(bytes).digest("hex");
const json = value => JSON.stringify(value, null, 2) + "\n";
const check = (value, message) => assert.ok(value, message);
export async function argvExec(command, args, options = {}) {
  check(["git", "docker", "cosign", "trivy", "tar"].includes(command), "Unsupported release tool");
  check(Array.isArray(args) && args.every(x => typeof x === "string"), "Argument vector required");
  const env = { ...process.env }; delete env.GITHUB_TOKEN; delete env.GH_TOKEN;
  try { return await execFileAsync(command, args, { env, encoding: "utf8", timeout: 600000, maxBuffer: 64 * 1024 * 1024, ...options, shell: false }); }
  catch (error) {
    // Never include command/env/raw stderr: they can contain credentials or signed URLs.
    const failure = new Error(`${command} failed (exit ${Number.isInteger(error.code) ? error.code : "unavailable"})`);
    failure.exitCode = error.code; throw failure;
  }
}
export async function canonical(env = process.env, exec = argvExec) {
  const versions = await Promise.all(manifests.map(async p => JSON.parse(await readFile(p, "utf8")).version));
  const context = resolveContext(env, versions);
  checkBoundary(context, env.RELEASE_CONTEXT);
  check((await exec("git", ["rev-parse", "HEAD"])).stdout.trim() === context.source_sha, "Wrong checkout");
  check((await exec("git", ["status", "--porcelain", "--untracked-files=no"])).stdout.trim() === "", "Dirty source");
  check(/^[1-9][0-9]*$/.test(env.GITHUB_RUN_ID) && /^[1-9][0-9]*$/.test(env.GITHUB_RUN_ATTEMPT), "Run identity required");
  return context;
}
const safeFile = name => /^[A-Za-z0-9][A-Za-z0-9_.-]*$/.test(name) && name !== "manifest.json";
// Preserve the raw scanner classifications and the exact primary evidence used
// to resolve only the reviewed UNKNOWN rows; no ignored or rewritten findings.
export async function captureScanEvidence(core, reports, graphs, save) {
  const policyBytes = await readFile(new URL("./release-advisory-policy.json", import.meta.url));
  check(`sha256:${hash(policyBytes)}` === core.advisoryPolicyDigest, "Advisory policy bytes differ from verifier");
  const summary = core.validateScanReports(reports, { graphs });
  check(summary?.verified === true && summary.advisoryPolicy?.digest === core.advisoryPolicyDigest, "Missing verified scan summary or policy binding");
  const policy = JSON.parse(policyBytes);
  const primaryRecords = [];
  for (const entry of policy.entries) {
    check(/^CVE-\d{4}-\d{4,}$/.test(entry.vulnerabilityID), "Invalid primary evidence identifier");
    const bytes = await readFile(new URL(`./release-advisory-evidence/${entry.vulnerabilityID}.json`, import.meta.url));
    check(hash(bytes) === entry.rawSHA256, "Primary advisory evidence checksum mismatch");
    primaryRecords.push([`vendor-${entry.vulnerabilityID}.json`, bytes]);
  }
  await save("scan-advisory-policy.json", policyBytes);
  for (const [name, bytes] of primaryRecords) await save(name, bytes);
  await save("scan-summary.json", summary);
  return summary;
}
export async function sealBundle(dir, context, env) {
  const files = [];
  for (const name of (await readdir(dir)).sort()) {
    if (name === "manifest.json") continue;
    check(safeFile(name) && (await lstat(join(dir, name))).isFile(), "Unsafe artifact member");
    const bytes = await readFile(join(dir, name));
    files.push({ name, size: bytes.length, sha256: hash(bytes) });
  }
  const manifest = { schemaVersion: 1, context, runID: env.GITHUB_RUN_ID, runAttempt: env.GITHUB_RUN_ATTEMPT, files };
  const bytes = json(manifest); await writeFile(join(dir, "manifest.json"), bytes);
  return hash(bytes);
}
export async function openBundle(dir, expectedHash, context, env) {
  check(/^[a-f0-9]{64}$/.test(expectedHash), "Expected evidence checksum required");
  check((await lstat(join(dir, "manifest.json"))).isFile(), "Unsafe manifest");
  const bytes = await readFile(join(dir, "manifest.json"));
  check(hash(bytes) === expectedHash, "Evidence manifest checksum mismatch");
  const manifest = JSON.parse(bytes);
  assert.deepEqual(manifest.context, context);
  check(manifest.schemaVersion === 1 && manifest.runID === env.GITHUB_RUN_ID && manifest.runAttempt === env.GITHUB_RUN_ATTEMPT, "Artifact source/run binding mismatch");
  const names = new Set();
  for (const file of manifest.files) {
    check(safeFile(file.name) && !names.has(file.name), "Unsafe/duplicate artifact member");
    names.add(file.name);
    const path = join(dir, file.name);
    check((await lstat(path)).isFile(), "Symlink/nonfile artifact");
    const content = await readFile(path);
    check(content.length === file.size && hash(content) === file.sha256, "Artifact checksum mismatch");
  }
  assert.deepEqual((await readdir(dir)).sort(), [...names, "manifest.json"].sort(), "Unlisted artifact");
  check(names.has("evidence.json"), "Missing prepared evidence");
  const evidence = JSON.parse(await readFile(join(dir, "evidence.json"), "utf8"));
  check(evidence.runID === env.GITHUB_RUN_ID && evidence.runAttempt === env.GITHUB_RUN_ATTEMPT, "Evidence run substitution");
  return evidence;
}
export function signatureArgs(context, graph) {
  return ["verify", `${graph.repository}@${graph.indexDigest}`,
    "--certificate-identity", context.identity, "--certificate-oidc-issuer", context.issuer,
    "--certificate-github-workflow-sha", context.source_sha,
    "--certificate-github-workflow-trigger", "workflow_dispatch"];
}
// GitHub adapter never creates a tag/release, never follows redirects with auth,
// and never treats unknown errors (including 403/404) as an empty namespace.
export function githubAPI({ token, fetchImpl = fetch, boundary = async () => {} }) {
  check(typeof token === "string" && token.length > 0, "GitHub token required");
  return async (path, { method = "GET", body, bytes, upload = false } = {}) => {
    await boundary();
    check(path.startsWith(`/repos/${repository}/`), "Noncanonical GitHub API path");
    check(["GET", "POST", "PATCH"].includes(method), "Unsupported GitHub operation");
    if (method === "POST") check(upload && /^\/repos\/[^/]+\/[^/]+\/releases\/[0-9]+\/assets\?name=[A-Za-z0-9_.-]+$/.test(path), "Only prepared asset uploads permitted");
    if (method === "PATCH") check(/^\/repos\/[^/]+\/[^/]+\/releases\/[0-9]+$/.test(path), "Only release record finalization permitted");
    let response;
    try {
      response = await fetchImpl(`https://${upload ? "uploads" : "api"}.github.com${path}`, {
        method, redirect: "error", signal: AbortSignal.timeout(30000),
        headers: { Authorization: `Bearer ${token}`, Accept: "application/vnd.github+json",
          "X-GitHub-Api-Version": "2022-11-28", "Content-Type": bytes ? "application/octet-stream" : "application/json" },
        body: bytes || (body ? JSON.stringify(body) : undefined),
      });
    } catch { throw new Error("GitHub API transport failed"); }
    check(response.ok, `GitHub API rejected ${method}: HTTP ${response.status}`);
    if (bytes || response.headers.get("content-type")?.includes("json")) return response.json();
    return Buffer.from(await response.arrayBuffer());
  };
}
export async function provisionedRecord(api, context, { draft = true } = {}) {
  const tag = `v${context.version}`;
  let ref = await api(`/repos/${repository}/git/ref/tags/${tag}`);
  for (let depth = 0; ref.object.type === "tag" && depth < 5; depth++) {
    check(/^[a-f0-9]{40}$/.test(ref.object.sha), "Invalid annotated Git tag");
    ref = await api(`/repos/${repository}/git/tags/${ref.object.sha}`);
  }
  check(ref.object.type === "commit" && ref.object.sha === context.source_sha, "Pre-provisioned Git tag must bind approved source SHA");
  const matches = [];
  for (let page = 1; page <= 10; page++) {
    const releases = await api(`/repos/${repository}/releases?per_page=100&page=${page}`);
    check(Array.isArray(releases), "Invalid releases response");
    matches.push(...releases.filter(r => r.tag_name === tag));
    if (releases.length < 100) break;
    check(page < 10, "Release pagination limit");
  }
  check(matches.length === 1, "Exactly one pre-provisioned release record required");
  const record = matches[0];
  check(record.draft === draft && record.prerelease === (context.channel === "rc"), "Wrong pre-provisioned release state");
  check(record.target_commitish === context.source_sha, "Draft target must be exact approved SHA");
  check(Number.isSafeInteger(record.id) && record.id > 0, "Invalid release id");
  return record;
}
export function assertPublicationChannel(context) {
  check(typeof context.version === "string" && !/\\s/.test(context.version), "Canonical version required");
  check((context.channel === "rc" && /^\d+\.\d+\.\d+-rc\.[1-9][0-9]*$/.test(context.version) &&
    ["refs/heads/develop", "refs/heads/main"].includes(context.ref)) ||
    (context.channel === "release" && /^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)$/.test(context.version) &&
    context.ref === "refs/heads/main"), "Invalid publication channel/ref/version");
}
export async function verifyPublishedPair({ context, evidence, clients, verifySignature, latest = false }) {
  assertPublicationChannel(context);
  check(!latest || context.channel === "release", "RC cannot inspect/promote latest");
  for (const graph of evidence.graphs) {
    const client = clients[graph.service];
    const version = await client.getManifest(context.version);
    check(version.digest === graph.indexDigest && `sha256:${hash(version.bytes)}` === graph.indexDigest, "Final version bytes changed");
    if (latest) {
      const current = await client.getLatestManifest();
      check(current?.digest === graph.indexDigest && current.bytes.equals(version.bytes), "Stable latest pair mismatch");
    }
    await verifySignature(graph);
  }
}
export async function promotePair({ context, evidence, clients, verifySignature, boundary, assertEvidence, reconcile }) {
  check(context.publish === "true", "Prepare-only cannot write final tags");
  assertPublicationChannel(context);
  assertEvidence(evidence, context);
  for (const graph of evidence.graphs) await verifySignature(graph);
  for (const service of services) await clients[service].assertVersionAbsent(context.version);
  const receipts = [], latestReceipts = [], previousLatest = [];
  try {
    for (const service of services) {
      await boundary();
      const graph = evidence.graphs.find(g => g.repository === context[`${service}_repository`]);
      const client = clients[service];
      // Obtain original bytes from the same allowlisted repository; never JSON stringify.
      const raw = await client.getManifest(graph.indexDigest);
      check(raw.digest === graph.indexDigest && `sha256:${hash(raw.bytes)}` === graph.indexDigest, "Prepared index changed");
      await client.assertVersionAbsent(context.version);
      await boundary();
      receipts.push(await client.promoteIndex(context.version, raw));
    }
    await verifyPublishedPair({ context, evidence, clients, verifySignature });
    if (context.channel === "release") {
      for (const service of services) {
        const previous = await clients[service].getLatestManifest();
        previousLatest.push({ service, digest: previous?.digest ?? null });
      }
      for (const service of services) {
        // Both immutable tags and both signatures precede EACH mutable update.
        await boundary();
        await verifyPublishedPair({ context, evidence, clients, verifySignature });
        const graph = evidence.graphs.find(g => g.service === service);
        const raw = await clients[service].getManifest(graph.indexDigest);
        await boundary();
        latestReceipts.push(await clients[service].promoteStableLatest(context.version, raw));
      }
      await verifyPublishedPair({ context, evidence, clients, verifySignature, latest: true });
    }
    return { schemaVersion: 1, complete: true, sourceSHA: context.source_sha, receipts, previousLatest, latestReceipts };
  } catch (error) {
    // Read-only reconciliation, never retry/overwrite/delete/rollback after partial or ambiguous PUT.
    const observed = [];
    for (const service of services) {
      try { const m = await clients[service].getManifest(context.version); observed.push({ service, digest: m.digest }); }
      catch { observed.push({ service, state: "unresolved-not-proof-of-absence" }); }
    }
    const observedLatest = [];
    if (context.channel === "release") for (const service of services) {
      try { const m = await clients[service].getLatestManifest(); observedLatest.push({ service, digest: m?.digest ?? null }); }
      catch { observedLatest.push({ service, state: "unresolved-not-proof-of-absence" }); }
    }
    await reconcile({ complete: false, incompleteDraft: true, observed, previousLatest, latestReceipts, observedLatest,
      ambiguousPreviousDigest: error?.previousDigest ?? null });
    throw new Error("Incomplete publication; owner-directed recovery required; no automatic retry");
  }
}
export async function execute(command, { env = process.env, exec = argvExec, fetchImpl = fetch, core, boundary: suppliedBoundary } = {}) {
  check(["install-tools", "preflight", "verify", "record-prepare", "promote", "record-finalize"].includes(command), "Unknown execution command");
  const boundary = suppliedBoundary || (() => canonical(env, exec));
  const context = await boundary();
  const tool = async (cmd, args, options) => { await boundary(); return exec(cmd, args, options); };
  const dir = resolve(env.EVIDENCE_DIR || "release-evidence");
  await mkdir(dir, { recursive: true });
  const save = (name, value) => { check(safeFile(name), "Unsafe evidence filename"); return writeFile(join(dir, name), Buffer.isBuffer(value) ? value : typeof value === "string" ? value : json(value)); };
  const output = (key, value) => { check(env.GITHUB_OUTPUT, "Output file required"); appendFileSync(env.GITHUB_OUTPUT, `${key}=${value}\n`); };
  if (command === "install-tools") {
    for (const [url, expected, name] of [[pins.trivyChecksumsURL, pins.trivyChecksumsSHA256, "trivy-checksums.txt"], [pins.trivyURL, pins.trivySHA256, "trivy.tar.gz"]]) {
      await boundary();
      const response = await fetchImpl(url, { signal: AbortSignal.timeout(120000) });
      check(response.ok, "Pinned Trivy download failed");
      const bytes = Buffer.from(await response.arrayBuffer());
      check(hash(bytes) === expected, "Official Trivy asset checksum mismatch");
      await save(name, bytes);
    }
    const checksums = await readFile(join(dir, "trivy-checksums.txt"), "utf8");
    check(checksums.includes(`${pins.trivySHA256}  trivy_0.76.2_Linux-64bit.tar.gz`), "Official checksum list mismatch");
    const bin = resolve(dir, "bin"); await mkdir(bin, { recursive: true });
    await tool("tar", ["-xzf", join(dir, "trivy.tar.gz"), "-C", bin, "trivy"]);
    check(env.GITHUB_PATH, "GITHUB_PATH required"); appendFileSync(env.GITHUB_PATH, bin + "\n");
    // Installation files are not prepared image evidence.
    return;
  }
  core ||= { ...await import("./release-registry.mjs"), ...await import("./release-verification.mjs") };
  assertPublicationChannel(context);
  const clients = Object.fromEntries(services.map(service => [service, core.createRegistryClient({
    repository: context[`${service}_repository`], actor: env.GITHUB_ACTOR, token: env.GITHUB_TOKEN, fetchImpl,
  })]));
  const api = githubAPI({ token: env.GITHUB_TOKEN, fetchImpl, boundary });
  if (command === "preflight") {
    for (const service of services) { await boundary(); await clients[service].assertVersionAbsent(context.version); }
    if (context.publish === "true") await provisionedRecord(api, context);
    return;
  }
  const verifySignature = async graph => {
    const result = await tool("cosign", signatureArgs(context, graph));
    const signatures = JSON.parse(result.stdout);
    check(Array.isArray(signatures) && signatures.length > 0, "No verified signature output");
    return { repository: graph.repository, indexDigest: graph.indexDigest, verified: true,
      identity: context.identity, issuer: context.issuer, sourceSHA: context.source_sha, event: "workflow_dispatch" };
  };
  if (command === "verify") {
    validateDigests(env);
    const externalFixture = env.INPUT_RUNTIME_FIXTURE ? validateRuntimeFixture(JSON.parse(env.INPUT_RUNTIME_FIXTURE)) : undefined;
    const graphs = [];
    const basePolicy = JSON.parse(await readFile("scripts/runtime-base-policy.json", "utf8"));
    await save("runtime-base-policy.json", basePolicy);
    const buildx = (await tool("docker", ["buildx", "version"])).stdout;
    check(buildx.split(/\s+/).includes(pins.buildx), "Buildx version mismatch");
    const builder = (await tool("docker", ["buildx", "inspect", "--bootstrap"])).stdout;
    check(/BuildKit version:\s+v0\.25\.1(?:\s|$)/.test(builder), "BuildKit version mismatch");
    const scanner = JSON.parse((await tool("trivy", ["--version", "--format", "json"])).stdout);
    check(scanner.Version === pins.trivyVersion, "Scanner version mismatch");
    // Always refresh DB; no cache, ignored findings or exception files.
    const dbResult = await tool("trivy", ["image", "--download-db-only", "--cache-dir", join(dir, "trivy-cache")]);
    check(!/\b(?:WARN|ERROR)\b/.test(dbResult.stderr || ""), "Scanner database warnings require review");
    const metadata = JSON.parse(await readFile(join(dir, "trivy-cache", "db", "metadata.json"), "utf8"));
    const scanReceipts = [], runtimeReceipts = [], signatureReceipts = [];
    for (const service of services) {
      await boundary();
      const graph = await core.verifyImageGraph({ client: clients[service],
        indexDigest: env[`${service.toUpperCase()}_DIGEST`], sourceSHA: context.source_sha,
        dockerfile: `did-${service}-service/Dockerfile`, service, baseVersion: context.base, basePolicy,
        onArtifact: async ({ kind, digest, bytes }) => save(`${service}-${kind.replace(/[^a-zA-Z0-9_-]/g, "_")}-${digest.replace(":", "-")}.raw`, bytes) });
      graphs.push(graph);
    }
    for (const graph of graphs) for (const platform of graph.platforms) {
      const name = `${graph.service}-${platform.platform.split("/")[1]}`;
      const reportPath = join(dir, `${name}-scan.json`);
      let exitCode = 0, warnings = [];
      try { const scanResult = await tool("trivy", ["image", "--image-src", "remote", "--platform", platform.platform,
        "--cache-dir", join(dir, "trivy-cache"), "--skip-db-update", "--ignorefile", "/dev/null", "--scanners", "vuln", "--list-all-pkgs", "--format", "json",
        "--severity", "UNKNOWN,LOW,MEDIUM,HIGH,CRITICAL", "--exit-code", "0",
        "--output", reportPath, `${graph.repository}@${platform.manifestDigest}`]);
        if (/\b(?:WARN|ERROR)\b/.test(scanResult.stderr || "")) warnings.push("scanner-reported-warning-requires-review");
      }
      catch (error) { exitCode = error.exitCode ?? -1; }
      const report = JSON.parse(await readFile(reportPath, "utf8"));
      scanReceipts.push({ repository: graph.repository, platform: platform.platform,
        manifestDigest: platform.manifestDigest, scannerVersion: scanner.Version, dbUpdatedAt: metadata.UpdatedAt,
        completedAt: new Date().toISOString(), exitCode, warnings, report });
    }
    // HIGH/CRITICAL and unresolved UNKNOWN fail. Reviewed, time-bounded CNA
    // classifications retain raw UNKNOWN rows and their primary-source evidence.
    const scanSummary = await captureScanEvidence(core, scanReceipts, graphs, save);
    await withRuntimeFixture({ externalFixture, externalFixtureApproved: env.INPUT_EXTERNAL_FIXTURE_APPROVED === "true", managerGraph: graphs.find(g => g.service === "manager"), exec: tool, save,
      run: async fixture => {
        await save("runtime-fixture.json", fixture);
        for (const graph of graphs) for (const platform of graph.platforms) {
          runtimeReceipts.push(await runtimeSmoke({ repository: graph.repository, platform: platform.platform,
            manifestDigest: platform.manifestDigest, fixture, exec: tool, fetchImpl }));
        }
      } });
    for (const graph of graphs) {
      await tool("cosign", ["sign", "--yes", `${graph.repository}@${graph.indexDigest}`]);
      signatureReceipts.push(await verifySignature(graph));
      // Signed root authenticates the exact descriptor/blob chain, not cosign attestations.
      await core.verifyImageGraph({ client: clients[graph.service], indexDigest: graph.indexDigest,
        sourceSHA: context.source_sha, dockerfile: graph.dockerfile, service: graph.service,
        baseVersion: context.base, basePolicy });
    }
    const evidence = { schemaVersion: 1, repository, sourceSHA: context.source_sha, sourceRef: context.ref,
      channel: context.channel, version: context.version, base: context.base, runID: env.GITHUB_RUN_ID,
      runAttempt: env.GITHUB_RUN_ATTEMPT, graphs, scanReceipts, scanSummary, runtimeReceipts, signatureReceipts,
      tools: { pins, buildx, builder, scanner, database: metadata }, verified: true,
      rollback: "Deploy only the separately approved previous digest pair; never move/delete version tags.",
      limitations: ["QEMU arm64 is not native arm64", "Profile metadata persistence is not funded DID/session restoration"] };
    core.assertPreparedEvidence(evidence, context);
    // Trivy DB/cache and installation artifacts must not enter the sealed flat bundle.
    const { rm } = await import("node:fs/promises");
    await rm(join(dir, "trivy-cache"), { recursive: true, force: true });
    await save("evidence.json", evidence);
    await save("rollback.txt", evidence.rollback);
    output("evidence_sha256", await sealBundle(dir, context, env));
    return;
  }
  const evidence = await openBundle(dir, env.EXPECTED_EVIDENCE_SHA256, context, env);
  core.assertPreparedEvidence(evidence, context);
  if (command === "record-prepare") {
    if (context.publish !== "true") return; // prepare-only never publishes/changes a GitHub record
    const record = await provisionedRecord(api, context);
    const prefix = `prepared-${env.GITHUB_RUN_ID}-${env.GITHUB_RUN_ATTEMPT}-`;
    const manifestBytes = await readFile(join(dir, "manifest.json"));
    const manifest = JSON.parse(manifestBytes);
    for (const name of [...manifest.files.map(f => f.name), "manifest.json"]) {
      const bytes = await readFile(join(dir, name));
      const assetName = prefix + name;
      check(!(record.assets || []).some(a => a.name === assetName), "Prepared assets already occupied; no overwrite/rerun");
      const asset = await api(`/repos/${repository}/releases/${record.id}/assets?name=${assetName}`, { method: "POST", bytes, upload: true });
      check(asset.name === assetName && asset.size === bytes.length && asset.digest === `sha256:${hash(bytes)}`, "Uploaded prepared asset digest mismatch");
    }
    await save("record-prepared.json", { id: record.id, sourceSHA: context.source_sha, preparedHash: env.EXPECTED_EVIDENCE_SHA256, prefix });
    output("evidence_sha256", await sealBundle(dir, context, env));
    return;
  }
  check(context.publish === "true", "Prepare-only cannot promote or finalize");
  const prepared = JSON.parse(await readFile(join(dir, "record-prepared.json"), "utf8"));
  const record = await provisionedRecord(api, context);
  check(record.id === prepared.id && prepared.sourceSHA === context.source_sha, "Wrong prepared record/source");
  const assets = await api(`/repos/${repository}/releases/${record.id}/assets?per_page=100`);
  check(Array.isArray(assets) && assets.length < 100, "Prepared asset list incomplete");
  const expectedManifest = assets.find(a => a.name === prepared.prefix + "manifest.json");
  check(expectedManifest?.digest === `sha256:${prepared.preparedHash}`, "Prepared remote evidence manifest missing/changed");
  // Check every pre-uploaded content digest, not just an invented successful-record boolean.
  for (const file of JSON.parse(await readFile(join(dir, "manifest.json"), "utf8")).files.filter(f => f.name !== "record-prepared.json" && f.name !== "publication.json")) {
    const remote = assets.find(a => a.name === prepared.prefix + file.name);
    check(remote?.digest === `sha256:${file.sha256}` && remote.size === file.size, "Prepared release asset changed/missing");
  }
  if (command === "promote") {
    const publication = await promotePair({ context, evidence, clients, boundary, verifySignature,
      assertEvidence: core.assertPreparedEvidence,
      reconcile: observation => save("incomplete-publication.json", observation) });
    await save("publication.json", publication);
    output("evidence_sha256", await sealBundle(dir, context, env));
    return;
  }
  const publication = JSON.parse(await readFile(join(dir, "publication.json"), "utf8"));
  check(publication.complete && publication.sourceSHA === context.source_sha, "Missing final pair receipt");
  check(publication.receipts?.length === 2, "Missing immutable pair receipts");
  check(context.channel === "rc" ? publication.latestReceipts?.length === 0 :
    publication.latestReceipts?.length === 2, "Incorrect stable/RC latest receipts");
  await verifyPublishedPair({ context, evidence, clients, verifySignature, latest: context.channel === "release" });
  // Final evidence is uploaded before publishing draft; assets are mutable, hashes retained.
  for (const name of ["publication.json", "manifest.json"]) {
    const bytes = await readFile(join(dir, name)); const assetName = `final-${env.GITHUB_RUN_ID}-${env.GITHUB_RUN_ATTEMPT}-${name}`;
    const asset = await api(`/repos/${repository}/releases/${record.id}/assets?name=${assetName}`, { method: "POST", upload: true, bytes });
    check(asset.digest === `sha256:${hash(bytes)}`, "Final asset checksum mismatch");
  }
  await boundary();
  await api(`/repos/${repository}/releases/${record.id}`, { method: "PATCH", body: {
    draft: false, prerelease: context.channel === "rc", make_latest: context.channel === "rc" ? "false" : "true",
  } });
  await provisionedRecord(api, context, { draft: false });
}
if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  execute(process.argv[2]).catch(() => { console.error("Release gate failed; inspect retained nonsensitive evidence; no automatic recovery."); process.exitCode = 1; });
}

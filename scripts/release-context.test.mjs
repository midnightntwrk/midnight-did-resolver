// This file is part of midnightntwrk/midnight-did-resolver.
// SPDX-License-Identifier: Apache-2.0
import assert from "node:assert/strict";
import { execFileSync, spawnSync } from "node:child_process";
import { chmodSync, mkdtempSync, mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import test from "node:test";
import { parse } from "yaml";
import { checkBoundary, manifests, repository, resolveContext, validateDigests, workflow } from "./release-context.mjs";

const sha = "a".repeat(40);
const versions = Array(4).fill("0.1.0");
const request = (patch = {}) => ({
  GITHUB_EVENT_NAME: "workflow_dispatch", GITHUB_REF_TYPE: "branch", GITHUB_REF: "refs/heads/develop",
  GITHUB_REPOSITORY: repository, GITHUB_SHA: sha,
  GITHUB_WORKFLOW_REF: `${workflow}@refs/heads/develop`, GITHUB_WORKFLOW_SHA: sha,
  INPUT_CHANNEL: "rc", INPUT_VERSION: "0.1.0-rc.3", ...patch,
});
const contextFor = (patch = {}, packages = versions) => resolveContext(request(patch), packages);

test("exact allowed channel/ref matrix and coherent image tags", () => {
  for (const [channel, ref, version] of [
    ["rc", "develop", "0.1.0-rc.1"], ["rc", "main", "0.1.0-rc.12"], ["release", "main", "0.1.0"],
  ]) {
    const context = contextFor({ INPUT_CHANNEL: channel, INPUT_VERSION: version,
      GITHUB_REF: `refs/heads/${ref}`, GITHUB_WORKFLOW_REF: `${workflow}@refs/heads/${ref}` });
    for (const service of ["resolver", "manager"]) {
      const image = `ghcr.io/midnightntwrk/midnight-did-${service}-service`;
      assert.equal(context[`${service}_image`], `${image}:${version}`);
      assert.deepEqual(context[`${service}_tags`].split("\n"), channel === "rc" ? [`${image}:${version}`] : [`${image}:${version}`, `${image}:latest`]);
    }
    assert.equal(context.identity, `https://github.com/${workflow}@refs/heads/${ref}`);
    assert.equal(context.source_sha, sha);
    assert.equal(context.platforms, "linux/amd64,linux/arm64");
  }
});

test("reject unsupported events, types, channels and exact/nested/lookalike refs", () => {
  for (const event of ["", "push", "pull_request", "workflow_call", "schedule", "release"]) {
    assert.throws(() => contextFor({ GITHUB_EVENT_NAME: event }));
  }
  for (const type of ["", "tag", "Branch"]) assert.throws(() => contextFor({ GITHUB_REF_TYPE: type }));
  for (const channel of ["", "snapshot", "RC", "stable", "release\n", undefined]) assert.throws(() => contextFor({ INPUT_CHANNEL: channel }));
  for (const channel of ["rc", "release"]) {
    for (const ref of ["", "main", "develop", "refs/tags/main", "refs/tags/develop", "refs/tags/v0.1.0-rc.1",
      "refs/heads/main-evil", "refs/heads/develop-evil", "refs/heads/feature/main", "refs/heads/feature/develop",
      "refs/heads/main/foo", "refs/heads/develop/foo", "refs/heads/milestone-one", "refs/pull/1/merge", "refs/heads/main\n",
      ...(channel === "release" ? ["refs/heads/develop"] : [])]) {
      assert.throws(() => contextFor({ INPUT_CHANNEL: channel, INPUT_VERSION: channel === "rc" ? "0.1.0-rc.1" : "0.1.0",
        GITHUB_REF: ref, GITHUB_WORKFLOW_REF: `${workflow}@${ref}` }), `${channel}:${ref}`);
    }
  }
});

test("canonical full version grammar and RC positive integer semantics", () => {
  for (const version of [undefined, "", "0.1.0", "0.1.0-rc.0", "0.1.0-rc.00", "0.1.0-rc.01", "0.1.0-rc.-1",
    "0.1.0-rc.+1", "0.1.0-rc.1.2", "0.1.0-rc.", "0.1.0-rc1", "0.1.0-RC.1", "0.1.0-beta.1", "0.1.0-rc.1+build",
    "v0.1.0-rc.1", "00.1.0-rc.1", "0.01.0-rc.1", "0.1.00-rc.1", "0.1-rc.1", " 0.1.0-rc.1", "0.1.0-rc.1 ",
    "0.1.0-rc.1\n", "0.1.0-rc.1\nlatest", "0.1.0-rc.1\r", "0.1.0-rc.1\0"]) {
    assert.throws(() => contextFor({ INPUT_VERSION: version }), String(version));
  }
  for (const version of ["0.1.0-rc.1", "v0.1.0", "00.1.0", "0.01.0", "0.1.00", "0.1.0+build", "0.1.0\n", "", "0.1"]) {
    assert.throws(() => contextFor({ INPUT_CHANNEL: "release", GITHUB_REF: "refs/heads/main",
      GITHUB_WORKFLOW_REF: `${workflow}@refs/heads/main`, INPUT_VERSION: version }));
  }
  for (const n of ["1", "12", "9007199254740993", "999999999999999999999999999999"]) {
    assert.equal(contextFor({ INPUT_VERSION: `0.1.0-rc.${n}` }).version, `0.1.0-rc.${n}`);
  }
  assert.equal(contextFor({ INPUT_VERSION: "0.0.0-rc.1" }, Array(4).fill("0.0.0")).base, "0.0.0");
  const longest = `0.1.0-rc.${"9".repeat(119)}`;
  assert.equal(longest.length, 128);
  assert.equal(contextFor({ INPUT_VERSION: longest }).version, longest);
  assert.throws(() => contextFor({ INPUT_VERSION: `${longest}9` }), /128/);
});

test("checked-in 1.0 base versions align with the selected planned Docker candidate", () => {
  const actualVersions = manifests.map((path) =>
    JSON.parse(readFileSync(new URL('../' + path, import.meta.url), 'utf8')).version);
  assert.deepEqual(actualVersions, Array(4).fill("1.0.0"));
  assert.equal(readFileSync(new URL("../version", import.meta.url), "utf8").trim(), "1.0.0");
  const context = contextFor({ INPUT_VERSION: "1.0.0-rc.1" }, actualVersions);
  assert.equal(context.base, "1.0.0");
  assert.equal(context.version, "1.0.0-rc.1");
  for (const service of ["resolver", "manager"]) {
    const image = `ghcr.io/midnightntwrk/midnight-did-${service}-service:1.0.0-rc.1`;
    assert.equal(context[`${service}_image`], image);
    assert.deepEqual(context[`${service}_tags`].split("\n"), [image]);
  }
  for (let i = 0; i < actualVersions.length; i++) {
    const mismatched = [...actualVersions]; mismatched[i] = "0.1.0";
    assert.throws(() => contextFor({ INPUT_VERSION: "1.0.0-rc.1" }, mismatched));
  }
  assert.throws(() => contextFor({ INPUT_VERSION: "0.1.0-rc.4" }, actualVersions));
});

test("all four runtime manifests must match the canonical base", () => {
  for (let i = 0; i < 4; i++) {
    for (const bad of [undefined, "", "0.2.0", "0.1.0-rc.3", "0.01.0"]) {
      const mismatched = [...versions]; mismatched[i] = bad;
      assert.throws(() => contextFor({}, mismatched), manifests[i]);
    }
  }
  assert.throws(() => contextFor({}, versions.slice(1)));
  assert.throws(() => contextFor({ INPUT_VERSION: "0.2.0-rc.1" }));
});

test("source/workflow identity, SHA, platform and boundary substitution fail closed", () => {
  for (const patch of [
    { GITHUB_REPOSITORY: "attacker/repo" }, { GITHUB_SHA: "a".repeat(39) }, { GITHUB_SHA: `${sha}\n` },
    { GITHUB_SHA: "A".repeat(40) }, { GITHUB_SHA: "z".repeat(40) }, { GITHUB_WORKFLOW_SHA: "b".repeat(40) },
    { GITHUB_WORKFLOW_REF: `${workflow}@refs/heads/main` }, { GITHUB_WORKFLOW_REF: `${repository}/.github/workflows/other.yml@refs/heads/develop` },
    { INPUT_PLATFORMS: "linux/amd64\n" }, { INPUT_PLATFORMS: "linux/386" }, { INPUT_PLATFORMS: "linux/amd64,evil" },
  ]) assert.throws(() => contextFor(patch));
  assert.equal(contextFor({ INPUT_PLATFORMS: "linux/arm64" }).platforms, "linux/arm64");
  const context = contextFor();
  checkBoundary(context, JSON.stringify(context));
  for (const [key, value] of Object.entries(context)) {
    assert.throws(() => checkBoundary(context, JSON.stringify({ ...context, [key]: `${value}-tampered` })), key);
    const missing = { ...context }; delete missing[key];
    assert.throws(() => checkBoundary(context, JSON.stringify(missing)), key);
  }
  for (const service of ["resolver", "manager"]) {
    assert.throws(() => checkBoundary(context, JSON.stringify({ ...context,
      [`${service}_tags`]: `${context[`${service}_tags`]}\nghcr.io/midnightntwrk/midnight-did-${service}-service:latest` })));
  }
  assert.throws(() => checkBoundary(context, JSON.stringify({ ...context, extra: "latest" })));
  assert.throws(() => checkBoundary(context, "not json"));
});

test("both digests validated before any signing or verification", () => {
  const digest = `sha256:${"f".repeat(64)}`;
  validateDigests({ RESOLVER_DIGEST: digest, MANAGER_DIGEST: digest });
  for (const key of ["RESOLVER_DIGEST", "MANAGER_DIGEST"]) {
    for (const bad of [undefined, "", "latest", "sha256:abc", `${digest}\n`, `${digest};echo evil`]) {
      assert.throws(() => validateDigests({ RESOLVER_DIGEST: digest, MANAGER_DIGEST: digest, [key]: bad }));
    }
  }
});

test("actual CLI orchestration pins clean source, revalidates boundaries and invokes exact Cosign argv (stub only)", () => {
  const temp = mkdtempSync(join(tmpdir(), "resolver-release-test-"));
  const cwd = join(temp, "source"); mkdirSync(cwd);
  const cli = fileURLToPath(new URL("release-context.mjs", import.meta.url));
  const git = (...args) => execFileSync("git", args, { cwd, encoding: "utf8" }).trim();
  try {
    for (const path of manifests) {
      const full = join(cwd, path); mkdirSync(dirname(full), { recursive: true });
      writeFileSync(full, JSON.stringify({ version: "0.1.0" }));
    }
    git("init", "--quiet"); git("add", ".");
    git("-c", "commit.gpgsign=false", "-c", "user.name=Test", "-c", "user.email=test@example.invalid", "commit", "--quiet", "-m", "fixture");
    const head = git("rev-parse", "HEAD");
    const bin = join(temp, "bin"); mkdirSync(bin);
    const log = join(temp, "cosign.jsonl");
    const stub = join(bin, "cosign");
    writeFileSync(stub, `#!${process.execPath}\nrequire('node:fs').appendFileSync(process.env.COSIGN_LOG, JSON.stringify(process.argv.slice(2))+'\\n');\n`);
    chmodSync(stub, 0o755);
    const output = join(temp, "output");
    const env = { ...process.env, ...request({ GITHUB_SHA: head, GITHUB_WORKFLOW_SHA: head }),
      PATH: `${bin}:${process.env.PATH}`, COSIGN_LOG: log, GITHUB_OUTPUT: output, GITHUB_STEP_SUMMARY: join(temp, "summary") };
    const invoke = (command, patch = {}) => spawnSync(process.execPath, [cli, command], { cwd, env: { ...env, ...patch }, encoding: "utf8" });
    let result = invoke("resolve"); assert.equal(result.status, 0, result.stderr);
    const records = Object.fromEntries([...readFileSync(output, "utf8").matchAll(/(\w+)<<RELEASE_OUTPUT\n([\s\S]*?)\nRELEASE_OUTPUT\n/g)].map((m) => [m[1], m[2]]));
    const canonical = resolveContext(env, versions);
    assert.deepEqual(JSON.parse(records.context), canonical);
    for (const [key, value] of Object.entries(canonical)) assert.equal(records[key], value);
    env.RELEASE_CONTEXT = records.context;
    for (const command of ["boundary", "sign", "verify", "summary"]) {
      const patch = { RESOLVER_DIGEST: `sha256:${"1".repeat(64)}`, MANAGER_DIGEST: `sha256:${"2".repeat(64)}` };
      result = invoke(command, patch); assert.equal(result.status, 0, result.stderr);
    }
    const calls = readFileSync(log, "utf8").trim().split("\n").map(JSON.parse);
    assert.equal(calls.length, 4);
    for (const [i, service] of ["resolver", "manager"].entries()) {
      const image = `${canonical[`${service}_image`]}@sha256:${String(i + 1).repeat(64)}`;
      assert.deepEqual(calls[i], ["sign", "--yes", image]);
      assert.deepEqual(calls[i + 2], ["verify", image, "--certificate-identity", canonical.identity,
        "--certificate-oidc-issuer", canonical.issuer, "--certificate-github-workflow-sha", head,
        "--certificate-github-workflow-trigger", "workflow_dispatch"]);
    }
    assert.match(readFileSync(env.GITHUB_STEP_SUMMARY, "utf8"), new RegExp(head));
    assert.equal(git("status", "--porcelain", "--untracked-files=no"), "");
    const before = readFileSync(log, "utf8");
    // Drive the real workflow's CLI/consumer sequence. Build actions are local
    // recording stubs, never Docker or registry calls. Failed metadata must
    // stop before either a build-action stub or the Cosign stub can run.
    const release = parse(readFileSync(new URL("../.github/workflows/release-docker.yml", import.meta.url), "utf8"));
    const publishLog = join(temp, "publish.jsonl");
    const publisher = join(temp, "publish.cjs");
    writeFileSync(publisher, "require('node:fs').appendFileSync(process.env.PUBLISH_LOG, process.argv[2]+'\\n');");
    const wiredPublication = (version) => {
      const outputs = {};
      const expand = (value) => value.replace(/\$\{\{ steps\.(\w+)\.outputs\.(\w+) \}\}/g, (_, id, key) => outputs[id][key]);
      for (const step of release.jobs.release.steps) {
        if (step.run) {
          const stepEnv = Object.fromEntries(Object.entries(step.env || {}).map(([key, value]) => [key, expand(value)]));
          const command = step.run.replace("node scripts/release-context.mjs ", "");
          const result = invoke(command, { INPUT_VERSION: version, ...stepEnv });
          if (result.status !== 0) return result;
          if (step.id) outputs[step.id] = Object.fromEntries([...readFileSync(output, "utf8").matchAll(/(\w+)<<RELEASE_OUTPUT\n([\s\S]*?)\nRELEASE_OUTPUT\n/g)].map((m) => [m[1], m[2]]));
        } else if (step.uses.startsWith("docker/build-push-action@")) {
          const inputs = Object.fromEntries(Object.entries(step.with).map(([key, value]) => [key, typeof value === "string" ? expand(value) : value]));
          const result = spawnSync(process.execPath, [publisher, JSON.stringify(inputs)], { env: { ...process.env, PUBLISH_LOG: publishLog } });
          assert.equal(result.status, 0);
          outputs[step.id] = { digest: `sha256:${"3".repeat(64)}` };
        }
      }
      return { status: 0 };
    };
    const wiredResult = wiredPublication("0.1.0-rc.3");
    assert.equal(wiredResult.status, 0, wiredResult.stderr);
    const published = readFileSync(publishLog, "utf8");
    assert.equal(published.trim().split("\n").length, 2);
    for (const inputs of published.trim().split("\n").map(JSON.parse)) {
      assert.equal(inputs.tags.includes(":latest"), false);
      assert.match(inputs.labels, new RegExp(head));
    }
    const signed = readFileSync(log, "utf8");
    for (const version of ["0.1.0-rc.0", "malformed", `0.1.0-rc.${"9".repeat(120)}`]) {
      assert.notEqual(wiredPublication(version).status, 0);
      assert.equal(readFileSync(publishLog, "utf8"), published, "Invalid version must not reach publishing stub");
      assert.equal(readFileSync(log, "utf8"), signed, "Invalid version must not reach signing stub");
    }
    // Restore only the log to simplify assertions about subsequent rejects.
    writeFileSync(log, before);
    for (const command of ["boundary", "sign", "verify"]) {
      for (const patch of [
        { GITHUB_SHA: sha, GITHUB_WORKFLOW_SHA: sha }, { GITHUB_EVENT_NAME: "push" },
        { INPUT_VERSION: "malformed" }, { INPUT_VERSION: `0.1.0-rc.${"9".repeat(120)}` },
        { GITHUB_WORKFLOW_REF: `${workflow}@refs/heads/main` },
        { RELEASE_CONTEXT: JSON.stringify({ ...canonical, resolver_tags: `${canonical.resolver_tags}\nlatest` }) },
        { RELEASE_CONTEXT: JSON.stringify({ ...canonical, manager_image: "attacker:latest" }) },
        { RELEASE_CONTEXT: JSON.stringify({ ...canonical, identity: "https://attacker" }) },
        { MANAGER_DIGEST: "" },
      ]) {
        if (command === "boundary" && Object.keys(patch)[0] === "MANAGER_DIGEST") continue;
        result = invoke(command, { RESOLVER_DIGEST: `sha256:${"1".repeat(64)}`, MANAGER_DIGEST: `sha256:${"2".repeat(64)}`, ...patch });
        assert.notEqual(result.status, 0, `${command}: ${JSON.stringify(patch)}`);
      }
    }
    assert.equal(readFileSync(log, "utf8"), before, "Rejected requests must not invoke Cosign");
    writeFileSync(join(cwd, "package.json"), JSON.stringify({ version: "0.2.0" }));
    assert.notEqual(invoke("resolve").status, 0);
    writeFileSync(join(cwd, "package.json"), JSON.stringify({ version: "0.1.0", changed: true }));
    assert.notEqual(invoke("boundary").status, 0, "Dirty source must fail even if base matches");
    git("add", "."); assert.notEqual(invoke("boundary").status, 0, "Staged changes must fail");
    assert.equal(readFileSync(log, "utf8"), before);
  } finally { rmSync(temp, { recursive: true, force: true }); }
});

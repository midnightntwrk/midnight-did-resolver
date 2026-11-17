// This file is part of midnightntwrk/midnight-did-resolver.
// SPDX-License-Identifier: Apache-2.0
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { appendFileSync, readFileSync } from "node:fs";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";

export const repository = "midnightntwrk/midnight-did-resolver";
export const workflow = `${repository}/.github/workflows/release-docker.yml`;
export const manifests = [
  "package.json",
  "did-resolver-service/package.json",
  "did-manager-service/package.json",
  "secret-storage/package.json",
];
const stable = "(0|[1-9][0-9]*)\\.(0|[1-9][0-9]*)\\.(0|[1-9][0-9]*)";
const stableVersion = new RegExp(`^${stable}$`);
const rcVersion = new RegExp(`^${stable}-rc\\.[1-9][0-9]*$`);
const requireThat = (condition, message) => assert.ok(condition, message);

export function resolveContext(env, versions) {
  requireThat(env.GITHUB_EVENT_NAME === "workflow_dispatch", "Manual dispatch required");
  requireThat(env.GITHUB_REF_TYPE === "branch", "Branch ref required");
  const channel = env.INPUT_CHANNEL;
  const ref = env.GITHUB_REF;
  requireThat(
    (channel === "rc" && ["refs/heads/develop", "refs/heads/main"].includes(ref)) ||
      (channel === "release" && ref === "refs/heads/main"),
    "Unsupported channel/source ref",
  );
  const version = env.INPUT_VERSION;
  requireThat(typeof version === "string" && (channel === "rc" ? rcVersion : stableVersion).test(version),
    "Version must match channel: X.Y.Z-rc.N (positive N, no leading zeros) or X.Y.Z");
  // RegExp $ may match before a trailing newline; never normalize dispatch inputs.
  requireThat(!/\s/.test(version), "Version must not contain whitespace");
  requireThat(version.length <= 128, "Image version tag exceeds the registry limit of 128 characters");
  const base = version.split("-")[0];
  requireThat(versions.length === manifests.length && versions.every((value) => value === base),
    "Root, resolver, manager and secret-storage must match the dispatched base version");
  requireThat(env.GITHUB_REPOSITORY === repository, "Unexpected repository");
  const source_sha = env.GITHUB_SHA;
  requireThat(typeof source_sha === "string" && /^[0-9a-f]{40}$/.test(source_sha) && source_sha.length === 40,
    "Full dispatched source SHA required");
  requireThat(env.INPUT_EXPECTED_SOURCE_SHA === source_sha, "Expected reviewed source SHA mismatch");
  const publish = env.INPUT_PUBLISH ?? "false";
  requireThat(publish === "true" || publish === "false", "Explicit boolean publish required");
  const workflow_ref = `${workflow}@${ref}`;
  requireThat(env.GITHUB_WORKFLOW_REF === workflow_ref, "Unexpected workflow identity");
  requireThat(env.GITHUB_WORKFLOW_SHA === source_sha, "Workflow/source SHA mismatch");
  const platforms = env.INPUT_PLATFORMS || "linux/amd64,linux/arm64";
  requireThat(/^linux\/(amd64|arm64)(,linux\/(amd64|arm64))*$/.test(platforms) && !/\s/.test(platforms),
    "Unsupported build platforms");
  requireThat(platforms === "linux/amd64,linux/arm64", "Both canonical platforms required");
  const imageRepository = (service) => `ghcr.io/midnightntwrk/midnight-did-${service}-service`;
  const image = (service) => `${imageRepository(service)}:${version}`;
  const tags = (service) => [image(service), ...(channel === "release" ? [image(service).replace(/:[^:]+$/, ":latest")] : [])].join("\n");
  return {
    schemaVersion: 1, channel, version, base, ref, source_sha, platforms, publish,
    build_context: `https://github.com/${repository}.git#${source_sha}`,
    resolver_repository: imageRepository("resolver"), manager_repository: imageRepository("manager"),
    identity: `https://github.com/${workflow_ref}`,
    issuer: "https://token.actions.githubusercontent.com",
    resolver_image: image("resolver"), manager_image: image("manager"),
    resolver_tags: tags("resolver"), manager_tags: tags("manager"),
  };
}

export function checkBoundary(context, serialized) {
  assert.deepStrictEqual(JSON.parse(serialized), context, "Publication metadata differs from dispatched context");
}

export function validateDigests(env) {
  for (const key of ["RESOLVER_DIGEST", "MANAGER_DIGEST"]) {
    requireThat(typeof env[key] === "string" && /^sha256:[0-9a-f]{64}$/.test(env[key]) && env[key].length === 71,
      `Invalid ${key}`);
  }
}

export function run(command, env = process.env) {
  requireThat(["resolve", "boundary", "sign", "verify", "summary"].includes(command), "Unknown release command");
  const versions = manifests.map((path) => JSON.parse(readFileSync(path, "utf8")).version);
  const context = resolveContext(env, versions);
  const git = (...args) => execFileSync("git", args, { encoding: "utf8" }).trim();
  requireThat(git("rev-parse", "HEAD") === context.source_sha, "Checkout is not the dispatched source SHA");
  // OCI labels/output operations must not mutate the source used for either build.
  requireThat(git("status", "--porcelain", "--untracked-files=no") === "", "Tracked source differs from dispatched commit");
  if (command !== "resolve") checkBoundary(context, env.RELEASE_CONTEXT);
  if (["resolve", "boundary"].includes(command)) {
    requireThat(Boolean(env.GITHUB_OUTPUT), "GITHUB_OUTPUT required");
    for (const [key, value] of Object.entries({ ...context, context: JSON.stringify(context) })) {
      appendFileSync(env.GITHUB_OUTPUT, `${key}<<RELEASE_OUTPUT\n${value}\nRELEASE_OUTPUT\n`);
    }
  } else {
    validateDigests(env);
    for (const service of ["resolver", "manager"]) {
      const digest = env[`${service.toUpperCase()}_DIGEST`];
      const image = `${context[`${service}_repository`]}@${digest}`;
      if (command === "sign") {
        execFileSync("cosign", ["sign", "--yes", image], { stdio: "inherit" });
      } else if (command === "verify") {
        execFileSync("cosign", ["verify", image,
          "--certificate-identity", context.identity,
          "--certificate-oidc-issuer", context.issuer,
          "--certificate-github-workflow-sha", context.source_sha,
          "--certificate-github-workflow-trigger", "workflow_dispatch"], { stdio: "inherit" });
      } else {
        console.log(`${service}=${image}`);
      }
    }
    if (command === "summary") {
      const summary = `## Docker publication\n\nSource: ${context.ref} @ ${context.source_sha}\n\nChannel: ${context.channel}; version: ${context.version}; platforms: ${context.platforms}\n\n` +
        `Resolver: ${context.resolver_image}@${env.RESOLVER_DIGEST}\n\nManager: ${context.manager_image}@${env.MANAGER_DIGEST}\n\nCertificate identity: ${context.identity}\n`;
      requireThat(Boolean(env.GITHUB_STEP_SUMMARY), "GITHUB_STEP_SUMMARY required");
      appendFileSync(env.GITHUB_STEP_SUMMARY, summary);
    }
  }
  return context;
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try { run(process.argv[2]); } catch (error) {
    console.error(`Release validation failed: ${error.message}`);
    process.exitCode = 1;
  }
}

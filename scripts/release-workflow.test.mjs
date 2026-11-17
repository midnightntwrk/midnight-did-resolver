// This file is part of midnightntwrk/midnight-did-resolver.
// SPDX-License-Identifier: Apache-2.0
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { readFileSync } from "node:fs";
import test from "node:test";
import { parse } from "yaml";
import { pins } from "./release-execution.mjs";
const read = path => readFileSync(new URL(`../${path}`, import.meta.url), "utf8");
const publication = parse(read(".github/workflows/release-docker.yml"));
const ci = parse(read(".github/workflows/ci.yml"));
const out = (id, key) => `\${{ steps.${id}.outputs.${key} }}`;
const guard = "set -euo pipefail\n[[ \"${EVENT_NAME}\" == workflow_dispatch && \"${REF_TYPE}\" == branch ]]\ncase \"${CHANNEL}:${REF}\" in\n  rc:refs/heads/develop|rc:refs/heads/main|release:refs/heads/main) ;;\n  *) echo \"::error::Unsupported publication channel/source ref\"; exit 1 ;;\nesac\n[[ \"${EXPECTED_SOURCE_SHA}\" =~ ^[0-9a-f]{40}$ && \"${EXPECTED_SOURCE_SHA}\" == \"${SOURCE_SHA}\" ]]\n";
const releaseIf = "github.event_name == 'workflow_dispatch' && github.ref_type == 'branch' && ((inputs.channel == 'rc' && (github.ref == 'refs/heads/develop' || github.ref == 'refs/heads/main')) || (inputs.channel == 'release' && github.ref == 'refs/heads/main'))";
const normalize = s => s.replace(/\s+/g, " ").trim();
const inputsEnv = {
  INPUT_CHANNEL: "${{ inputs.channel }}", INPUT_VERSION: "${{ inputs.version }}",
  INPUT_PLATFORMS: "${{ inputs.platforms }}", INPUT_EXPECTED_SOURCE_SHA: "${{ inputs.expected_source_sha }}",
  INPUT_PUBLISH: "${{ inputs.publish }}",
};
const actions = {
  checkout: "actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1",
  node: "actions/setup-node@249970729cb0ef3589644e2896645e5dc5ba9c38",
  qemu: "docker/setup-qemu-action@c7c53464625b32c7a7e944ae62b3e17d2b600130",
  buildx: "docker/setup-buildx-action@8d2750c68a42422c14e847fe6c8ac0403b4cbd6f",
  login: "docker/login-action@c94ce9fb468520275223c153574b00df6fe4bcc9",
  build: "docker/build-push-action@10e90e3645eae34f1e60eeb005ba3a3d33f178e8",
  cosign: "sigstore/cosign-installer@7e8b541eb2e61bf99390e1afd4be13a184e9ebc5",
  upload: "actions/upload-artifact@ea165f8d65b6e75b540449e92b4886f43607fa02",
  download: "actions/download-artifact@634f93cb2916e3fdff6788551b99b062d0335ce0",
};
function checkSteps(steps, expected) {
  assert.equal(steps.length, expected.length, "No extra/omitted privileged consumers");
  for (const [index, step] of steps.entries()) {
    assert.equal(step.if, expected[index].if);
    assert.equal(step["continue-on-error"], undefined);
    assert.equal(step["working-directory"], undefined);
    assert.equal(step.shell, undefined);
    const actual = { ...step }; delete actual.name;
    assert.deepEqual(actual, expected[index], `Consumer position ${index}`);
  }
}
const checkout = [
  { uses: actions.checkout, with: { ref: "${{ github.sha }}", "persist-credentials": false } },
  { uses: actions.node, with: { "node-version": "24", "package-manager-cache": false } },
];
const cosign = { uses: actions.cosign, with: { "cosign-release": pins.cosign } };
const upload = name => ({ uses: actions.upload, with: {
  name: `${name}-\${{ github.run_id }}-\${{ github.run_attempt }}`,
  path: "release-evidence/", "if-no-files-found": "error", "retention-days": 90,
} });
const download = name => ({ uses: actions.download, with: {
  name: `${name}-\${{ github.run_id }}-\${{ github.run_attempt }}`, path: "release-evidence/",
} });
function checkWorkflow(w, c) {
  assert.deepEqual(w.on, { workflow_dispatch: { inputs: {
    channel: { description: "Publication channel (source eligibility is not release approval)", required: true, type: "choice", default: "rc", options: ["rc", "release"] },
    version: { description: "Full image version: X.Y.Z-rc.N for rc, X.Y.Z for release", required: true, type: "string" },
    platforms: { description: "Buildx target platforms", required: false, default: "linux/amd64,linux/arm64", type: "string" },
    expected_source_sha: { description: "Independently reviewed full source SHA (must equal event SHA)", required: true, type: "string" },
    publish: { description: "Explicit final publication; false only stages and verifies", required: true, type: "boolean", default: false },
    external_fixture_approved: { description: "Explicit approval for a supplied external fixture override", required: false, type: "boolean", default: false },
    runtime_fixture: { description: "Optional approved public fixture override JSON; absent creates an owned standalone fixture", required: false, type: "string" },
  } } });
  assert.deepEqual(w.permissions, { contents: "read" });
  assert.equal(w.env, undefined);
  assert.deepEqual(w.concurrency, { group: "midnight-did-release-publication", "cancel-in-progress": false });
  assert.deepEqual(Object.keys(w.jobs), ["validate-dispatch", "stage-and-verify", "record-prepare", "promote", "record-finalize"]);
  const pre = w.jobs["validate-dispatch"];
  assert.deepEqual(pre.permissions, {});
  assert.equal(pre.if, undefined);
  checkSteps(pre.steps, [{ env: { EVENT_NAME: "${{ github.event_name }}", REF_TYPE: "${{ github.ref_type }}",
    REF: "${{ github.ref }}", CHANNEL: "${{ inputs.channel }}", EXPECTED_SOURCE_SHA: "${{ inputs.expected_source_sha }}", SOURCE_SHA: "${{ github.sha }}" }, run: guard }]);
  const stage = w.jobs["stage-and-verify"];
  assert.equal(stage.needs, "validate-dispatch");
  assert.equal(normalize(stage.if), releaseIf);
  assert.deepEqual(stage.permissions, { contents: "read", packages: "write", "id-token": "write" });
  assert.deepEqual(stage.outputs, { context: out("meta", "context"), evidence_sha256: out("verify", "evidence_sha256") });
  assert.deepEqual(stage.env, { ...inputsEnv, INPUT_RUNTIME_FIXTURE: "${{ inputs.runtime_fixture }}", INPUT_EXTERNAL_FIXTURE_APPROVED: "${{ inputs.external_fixture_approved }}", DOCKER_BUILD_RECORD_UPLOAD: "false" });
  const expected = [...checkout,
    { id: "meta", run: "node scripts/release-context.mjs resolve" },
    { env: { RELEASE_CONTEXT: out("meta", "context"), GITHUB_TOKEN: "${{ secrets.GITHUB_TOKEN }}" }, run: "node scripts/release-execution.mjs preflight" },
    { uses: actions.qemu },
    { uses: actions.buildx, with: { version: pins.buildx, "driver-opts": `image=${pins.buildkit}`, "cache-binary": false } },
    { uses: actions.login, with: { registry: "ghcr.io", username: "${{ github.actor }}", password: "${{ secrets.GITHUB_TOKEN }}" } },
    { env: { RELEASE_CONTEXT: out("meta", "context"), EVIDENCE_DIR: "release-tools" }, run: "node scripts/release-execution.mjs install-tools" },
  ];
  for (const service of ["resolver", "manager"]) {
    expected.push({ id: `${service}_context`, env: { RELEASE_CONTEXT: out("meta", "context") }, run: "node scripts/release-context.mjs boundary" });
    expected.push({ id: `build_${service}`, uses: actions.build, with: {
      context: `https://github.com/midnightntwrk/midnight-did-resolver.git#${out(service + "_context", "source_sha")}`,
      file: `did-${service}-service/Dockerfile`, platforms: out(service + "_context", "platforms"),
      outputs: `type=image,name=${out(service + "_context", service + "_repository")},push-by-digest=true,name-canonical=true,oci-mediatypes=true,push=true`,
      labels: `org.opencontainers.image.revision=${out(service + "_context", "source_sha")}\norg.opencontainers.image.source=https://github.com/midnightntwrk/midnight-did-resolver\norg.opencontainers.image.version=${out(service + "_context", "version")}\n`,
      provenance: "mode=max,version=v0.2", sbom: true,
    } });
  }
  expected.push(cosign, { id: "verify", env: { RELEASE_CONTEXT: out("meta", "context"),
    RESOLVER_DIGEST: out("build_resolver", "digest"), MANAGER_DIGEST: out("build_manager", "digest"), GITHUB_TOKEN: "${{ secrets.GITHUB_TOKEN }}" },
    run: "node scripts/release-execution.mjs verify" }, upload("prepared"),
    { if: "failure()", uses: actions.upload, with: {
      name: "stage-failed-${{ github.run_id }}-${{ github.run_attempt }}",
      path: "release-evidence/*.json\nrelease-evidence/*.raw\nrelease-evidence/*.txt\n",
      "if-no-files-found": "error", "retention-days": 90,
    } });
  checkSteps(stage.steps, expected);
  for (const [id, needs, artifact, previous, permissions] of [
    ["record-prepare", ["stage-and-verify"], "prepared", "stage-and-verify", { contents: "write" }],
    ["promote", ["stage-and-verify", "record-prepare"], "record-prepared", "record-prepare", { contents: "read", packages: "write" }],
    ["record-finalize", ["stage-and-verify", "record-prepare", "promote"], "promoted", "promote", { contents: "write", packages: "read" }],
  ]) {
    const job = w.jobs[id];
    assert.deepEqual(job.needs, needs); assert.equal(job.if, "${{ inputs.publish == true }}");
    assert.deepEqual(job.permissions, permissions);
    assert.deepEqual(job.env, { ...inputsEnv, RELEASE_CONTEXT: "${{ needs.stage-and-verify.outputs.context }}",
      EXPECTED_EVIDENCE_SHA256: `\${{ needs.${previous}.outputs.evidence_sha256 }}` });
    assert.deepEqual(job.outputs, id === "record-finalize" ? undefined : { evidence_sha256: out("execute", "evidence_sha256") });
    checkSteps(job.steps, [...checkout, download(artifact), ...(id === "record-prepare" ? [] : [
      { uses: actions.login, with: { registry: "ghcr.io", username: "${{ github.actor }}", password: "${{ secrets.GITHUB_TOKEN }}" } }, cosign]),
      { id: "execute", env: { GITHUB_TOKEN: "${{ secrets.GITHUB_TOKEN }}" }, run: `node scripts/release-execution.mjs ${id}` },
      ...(id === "record-finalize" ? [] : [upload(id === "record-prepare" ? "record-prepared" : "promoted")]),
      ...(id === "promote" ? [{ if: "failure()", uses: actions.upload, with: {
        name: "incomplete-${{ github.run_id }}-${{ github.run_attempt }}", path: "release-evidence/incomplete-publication.json",
        "if-no-files-found": "error", "retention-days": 90,
      } }] : [])]);
  }
  for (const job of Object.values(w.jobs)) {
    assert.equal(job["continue-on-error"], undefined); assert.equal(job.environment, undefined);
    assert.equal(job["runs-on"], "ubuntu-24.04");
    for (const step of job.steps) if (step.uses) assert.match(step.uses, /@[a-f0-9]{40}$/);
  }
  assert.deepEqual(c.on, { push: { branches: ["develop", "main"] }, pull_request: { branches: ["develop", "main"] }, workflow_dispatch: {} });
  assert.deepEqual(c.permissions, { contents: "read" });
  for (const run of ["npm run test:release-policy", "npm audit --omit=dev --audit-level=high"]) {
    assert.ok(c.jobs.validate.steps.some(s => s.run === run && s.if === undefined && s["continue-on-error"] === undefined));
  }
}
test("strict staged DAG, source-bound digest exporters, checksums and privilege separation", () => {
  checkWorkflow(publication, ci);
  const pkg = JSON.parse(read("package.json"));
  assert.equal(pkg.scripts["test:release-policy"], "node --test scripts/release-context.test.mjs scripts/release-workflow.test.mjs scripts/release-execution.test.mjs scripts/release-publication.test.mjs scripts/release-scan-evidence.test.mjs scripts/export-runtime.test.mjs");
});
test("actual pre-checkout event/ref/channel/SHA guard matrix", () => {
  const script = publication.jobs["validate-dispatch"].steps[0].run;
  for (const channel of ["rc", "release", "snapshot", "", "RC"]) for (const ref of [
    "refs/heads/develop", "refs/heads/main", "refs/tags/main", "refs/tags/develop", "refs/tags/v0.1.0",
    "refs/heads/feature/main", "refs/heads/main/child", "refs/heads/develop/child", "refs/heads/develop-evil", "refs/heads/main-evil", "refs/heads/milestone-x",
  ]) for (const event of ["workflow_dispatch", "push"]) {
    const result = spawnSync("bash", ["-c", script], { env: { ...process.env, CHANNEL: channel, REF: ref, EVENT_NAME: event, REF_TYPE: "branch", SOURCE_SHA: "a".repeat(40), EXPECTED_SOURCE_SHA: "a".repeat(40) }, encoding: "utf8" });
    const allowed = event === "workflow_dispatch" && ((channel === "rc" && ["refs/heads/develop", "refs/heads/main"].includes(ref)) || (channel === "release" && ref === "refs/heads/main"));
    assert.equal(result.status === 0, allowed, `${event}:${channel}:${ref}`);
  }
  for (const patch of [{ REF_TYPE: "tag" }, { EXPECTED_SOURCE_SHA: "b".repeat(40) }, { EXPECTED_SOURCE_SHA: "" }]) {
    assert.notEqual(spawnSync("bash", ["-c", script], { env: { ...process.env, CHANNEL: "rc", REF: "refs/heads/develop", EVENT_NAME: "workflow_dispatch", REF_TYPE: "branch", SOURCE_SHA: "a".repeat(40), EXPECTED_SOURCE_SHA: "a".repeat(40), ...patch } }).status, 0);
  }
});
test("every privileged step cannot be skipped, reordered, replaced or gain cache/permissions", () => {
  for (const [id, job] of Object.entries(publication.jobs)) for (let i = 0; i < job.steps.length; i++) {
    for (const change of [s => { s.if = "false"; }, s => { s["continue-on-error"] = true; }, s => { s.run = "true"; }, s => { s["working-directory"] = "/tmp"; }]) {
      const w = structuredClone(publication); change(w.jobs[id].steps[i]);
      assert.throws(() => checkWorkflow(w, ci), `${id}:${i}:${change}`);
    }
    const missing = structuredClone(publication); missing.jobs[id].steps.splice(i, 1);
    assert.throws(() => checkWorkflow(missing, ci));
  }
});
test("original and staged workflow security-negative mutations remain blocked", () => {
  const stage = w => w.jobs["stage-and-verify"];
  const mutations = [
    w => { w.on.push = { tags: ["v*"] }; }, w => { w.on.pull_request = {}; }, w => { w.on.workflow_call = {}; },
    w => { w.on.workflow_dispatch.inputs.channel.options.push("snapshot"); },
    w => { w.on.workflow_dispatch.inputs.version.required = false; },
    w => { w.on.workflow_dispatch.inputs.expected_source_sha.required = false; },
    w => { w.on.workflow_dispatch.inputs.publish.default = true; },
    w => { delete w.concurrency; }, w => { w.concurrency["cancel-in-progress"] = true; },
    w => { w.concurrency.group = "${{ github.ref }}"; },
    w => { stage(w).needs = []; }, w => { stage(w).if = "always()"; },
    w => { stage(w).steps[0].with.ref = "develop"; }, w => { stage(w).steps[0].with["persist-credentials"] = true; },
    w => { delete stage(w).steps[1].with["package-manager-cache"]; }, w => { stage(w).steps[1].with.cache = "npm"; },
    w => { stage(w).env.GITHUB_SHA = "attacker"; },
    w => { stage(w).steps[8].env.RELEASE_CONTEXT = "{}"; },
    w => { stage(w).steps[9].with.tags = "ghcr.io/midnightntwrk/midnight-did-resolver-service:latest"; },
    w => { stage(w).steps[11].with.tags = out("meta", "manager_tags"); },
    w => { stage(w).steps[9].with.labels = "org.opencontainers.image.revision=wrong"; },
    w => { stage(w).steps[11].with.context = "https://github.com/attacker/repo.git"; },
    w => { stage(w).steps[9].with.context = "."; },
    w => { stage(w).steps[9].with.provenance = false; }, w => { stage(w).steps[11].with.sbom = false; },
    w => { stage(w).steps[9].with.outputs += ",name=attacker:latest"; },
    w => { stage(w).steps[13].env.RESOLVER_DIGEST = "sha256:wrong"; },
    w => { stage(w).steps[13].run = "cosign verify --certificate-identity-regexp '.*' attacker:latest"; },
    w => { stage(w).steps[12].with["cosign-release"] = "main"; }, w => { stage(w).permissions.contents = "write"; },
    w => { w.jobs.promote.needs = ["stage-and-verify"]; }, w => { w.jobs.promote.if = "always()"; },
    w => { delete w.jobs.promote.env.EXPECTED_EVIDENCE_SHA256; },
    w => { delete w.jobs["record-prepare"].steps[2].with.name; },
    w => { w.jobs["record-prepare"].steps[3].run = "node scripts/release-execution.mjs record-finalize"; },
    w => { w.jobs["record-finalize"].permissions.packages = "write"; },
    w => { w.jobs["record-prepare"].permissions["id-token"] = "write"; },
  ];
  for (const mutate of mutations) { const w = structuredClone(publication); mutate(w); assert.throws(() => checkWorkflow(w, ci), String(mutate)); }
  const c = structuredClone(ci); c.jobs.validate.steps = c.jobs.validate.steps.filter(s => s.run !== "npm run test:release-policy");
  assert.throws(() => checkWorkflow(publication, c));
});

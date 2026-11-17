// This file is part of midnightntwrk/midnight-did-resolver.
// SPDX-License-Identifier: Apache-2.0
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";
import { captureScanEvidence, hash } from "./release-execution.mjs";

const policyBytes = await readFile(new URL("./release-advisory-policy.json", import.meta.url));
const digest = `sha256:${hash(policyBytes)}`;
const summary = { verified: true, rawUnknownCount: 8,
  rawSeverityCounts: { UNKNOWN: 8 }, effectiveSeverityCounts: { LOW: 4, MEDIUM: 4 },
  severityResolutionReceipts: [{ originalSeverity: "UNKNOWN", effectiveSeverity: "LOW" }],
  advisoryPolicy: { digest } };
// These are orchestration stubs, not substitutes for the core's 268 policy tests.
const core = { advisoryPolicyDigest: digest, validateScanReports: () => summary };

test("retain unchanged classifications, policy bytes and hash-bound primary records", async () => {
  const saved = new Map();
  assert.equal(await captureScanEvidence(core, [], [], async (name, bytes) => saved.set(name, bytes)), summary);
  assert.deepEqual(saved.get("scan-advisory-policy.json"), policyBytes);
  assert.equal(saved.get("scan-summary.json"), summary);
  assert.equal(summary.rawUnknownCount, 8);
  const policy = JSON.parse(policyBytes);
  for (const entry of policy.entries) {
    const bytes = saved.get(`vendor-${entry.vulnerabilityID}.json`);
    assert.equal(hash(bytes), entry.rawSHA256);
    const record = JSON.parse(bytes);
    assert.equal(record.cveMetadata.cveId, entry.vulnerabilityID);
    assert.equal(record.containers.cna.providerMetadata.orgId, entry.cna.orgID);
    assert.ok(record.containers.cna.metrics.some(m =>
      m.cvssV3_1?.baseScore === entry.cvssScore &&
      m.cvssV3_1?.baseSeverity === entry.effectiveSeverity &&
      m.cvssV3_1?.vectorString === entry.cvssVector));
  }
  assert.equal(saved.size, 4);
});

test("policy digest substitution fails before evidence writes", async () => {
  const saved = [];
  await assert.rejects(captureScanEvidence({ ...core, advisoryPolicyDigest: `sha256:${"0".repeat(64)}` }, [], [], async n => saved.push(n)), /policy bytes differ/);
  assert.equal(saved.length, 0);
});

test("missing or failed scan summaries never become retained success evidence", async () => {
  for (const result of [undefined, { verified: false }, { ...summary, advisoryPolicy: { digest: "wrong" } }]) {
    const saved = [];
    await assert.rejects(captureScanEvidence({ ...core, validateScanReports: () => result }, [], [], async n => saved.push(n)), /Missing verified scan/);
    assert.equal(saved.length, 0);
  }
});

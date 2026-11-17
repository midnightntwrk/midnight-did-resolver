// This file is part of midnightntwrk/midnight-did-resolver.
// SPDX-License-Identifier: Apache-2.0
// All fixtures are synthetic policy/transport tests, NOT captured release evidence.
import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { createRegistryClient, repositories, sha256, OCI_INDEX, OCI_MANIFEST } from './release-registry.mjs';
import { verifyImageGraph, validateScanReports as validateScanReportsReal, assertPreparedEvidence as assertPreparedEvidenceReal, advisoryPolicyDigest } from './release-verification.mjs';
const TEST_NOW = Date.parse('2026-11-17T12:00:00Z');
const validateScanReports = (reports, options) => validateScanReportsReal(reports, { nowMs: TEST_NOW, ...options });
const assertPreparedEvidence = (evidence, context) => assertPreparedEvidenceReal(evidence, context, { nowMs: TEST_NOW });
const reviewedAdvisoryPolicy = JSON.parse(readFileSync(new URL('./release-advisory-policy.json', import.meta.url)));

const SHA = 'a'.repeat(40);
const VERSION = '1.0.0-rc.1';
const CONFIG = 'application/vnd.oci.image.config.v1+json';
const INTOTO = 'application/vnd.in-toto+json';
const SPDX = 'https://spdx.dev/Document';
const SLSA = 'https://slsa.dev/provenance/v0.2';
const GIT = `https://github.com/midnightntwrk/midnight-did-resolver.git#${SHA}`;
const platforms = ['linux/amd64', 'linux/arm64'];
const basePolicy = {
  "schemaVersion": 1,
  "reference": "gcr.io/distroless/nodejs24-debian13:nonroot@sha256:6abfcb367ebc291572c774c1592a986be2ab26a2486f73093401327495a5962a",
  "indexDigest": "sha256:6abfcb367ebc291572c774c1592a986be2ab26a2486f73093401327495a5962a",
  "nodeVersion": "24.22.3",
  "platforms": {
    "linux/amd64": {
      "manifestDigest": "sha256:d4b8c34560df58c821ebda89a2897d7d183220c0debaf474a2b67a6c7cc22064"
    },
    "linux/arm64": {
      "manifestDigest": "sha256:6ff17c676ac284211c900bc2f512dd0b4e90ec6188657c91a4d6440c1f100b90"
    }
  },
  "builder": {
    "reference": "docker.io/library/node:24.22.3-bookworm-slim@sha256:dc26ae92c9fb15cf6b1d63e20f0944b2efe0f115b20e00be48fb23fceb061c54",
    "indexDigest": "sha256:dc26ae92c9fb15cf6b1d63e20f0944b2efe0f115b20e00be48fb23fceb061c54",
    "nodeVersion": "24.22.3",
    "platforms": {
      "linux/amd64": {
        "manifestDigest": "sha256:0e2646ab57e77b7a384a0cb59b6815b182c77e1f9c3ba36bd8435bd9c4e6680e"
      },
      "linux/arm64": {
        "manifestDigest": "sha256:5f8fabfed57b75127de2f584593d67a9e8c90e6b0f65652871f9ef0f759c0299"
      }
    }
  }
};
const clone = value => structuredClone(value);

test('source runtime policy is exactly the reviewed two-role fixture', () => {
  assert.deepEqual(JSON.parse(readFileSync(new URL('./runtime-base-policy.json', import.meta.url))), basePolicy);
});
for (const mode of ['index', 'child', 'node-aliases']) test(`both base roles support approved ${mode} provenance forms`, async () => {
  const f = graphFixture('resolver', (kind, v, platform) => {
    if (kind !== 'provenance') return;
    if (mode === 'index') {
      v.predicate.materials[1].digest.sha256 = basePolicy.builder.indexDigest.slice(7);
      v.predicate.materials[2].digest.sha256 = basePolicy.indexDigest.slice(7);
    }
    if (mode === 'child') for (const [index, p] of [basePolicy.builder, basePolicy].entries()) {
      v.predicate.buildConfig.llbDefinition[index].op.Op.source.identifier = 'docker-image://' + p.reference.split('@')[0] + '@' + p.platforms[platform].manifestDigest;
    }
    if (mode === 'node-aliases') v.predicate.materials[1].uri = 'pkg:docker/docker.io/library/node@24.22.3-bookworm-slim?platform=' + encodeURIComponent(platform);
  });
  assert.equal((await verifyImageGraph(f.input)).verified, true);
});
const approvedDockerfile = () => `FROM ${basePolicy.builder.reference} AS builder\nFROM ${basePolicy.reference} AS runtime\n`;
for (const [label, edit] of [
  ['reversed roles', text => text.split('\n').slice(0, 2).reverse().join('\n')],
  ['extra stage', text => text + 'FROM ' + basePolicy.reference + ' AS extra\n'],
  ['missing runtime', text => text.split('\n')[0]],
  ['ARG declaration', text => 'ARG NODE_VERSION=24.22.3\n' + text],
  ['ARG substitution', text => text.replace('24.22.3-bookworm-slim', '${NODE_VERSION}-bookworm-slim')],
  ['unpinned runtime', text => text.replace('@' + basePolicy.indexDigest, '')],
  ['builder as runtime', text => text.replace(basePolicy.reference, basePolicy.builder.reference)],
  ['runtime wrong tag', text => text.replace(':nonroot@', ':latest@')],
  ['runtime child instead of reviewed index', text => text.replace(basePolicy.indexDigest, basePolicy.platforms['linux/amd64'].manifestDigest)],
]) test(`embedded Dockerfile rejects ${label}`, async () => {
  const f = graphFixture('resolver', (kind, v) => {
    if (kind === 'provenance') v.predicate.metadata['https://mobyproject.org/buildkit@v1#metadata'].source.infos[0].data = Buffer.from(edit(approvedDockerfile())).toString('base64');
  });
  await assert.rejects(verifyImageGraph(f.input));
});

function addVendorUnknowns(evidence) {
  for (const r of evidence.scanReceipts) {
    r.report.Metadata.OS = { Family: 'debian', Name: '13.7' };
    const result = r.report.Results[0];
    result.Packages.push({ Name: 'libc6', Version: '2.41-12+deb13u4' });
    result.Vulnerabilities = reviewedAdvisoryPolicy.entries.map(e => ({
      VulnerabilityID: e.vulnerabilityID, PkgName: 'libc6', InstalledVersion: '2.41-12+deb13u4', Severity: 'UNKNOWN', Status: 'affected',
    }));
  }
}
test('reviewed advisory policy has only two unique primary CNA classifications and exact raw hashes', () => {
  const p = reviewedAdvisoryPolicy;
  assert.equal(p.entries.length, 2); assert.equal(new Set(p.entries.map(e => e.vulnerabilityID)).size, 2);
  assert.deepEqual(p.entries.map(e => [e.vulnerabilityID, e.effectiveSeverity, e.cvssScore, e.rawSHA256]), [
    ['CVE-2026-100290', 'LOW', 3.7, 'b6934af827b05e2b6d4dd959203d86043deac6367d016d5ae346d40463a3e508'],
    ['CVE-2026-115819', 'MEDIUM', 4.8, '8ce8faa72fca71320a47725835d8aa2eca0b37a50cf5ab5677dda7462fe1b00b'],
  ]);
  assert.equal(advisoryPolicyDigest, sha256(readFileSync(new URL('./release-advisory-policy.json', import.meta.url))));
});
test('eight UNKNOWN scan rows resolve below threshold without mutating or deduplicating raw findings', async () => {
  const { evidence, context } = await preparedFixture(); addVendorUnknowns(evidence);
  const raw = JSON.stringify(evidence.scanReceipts);
  const result = validateScanReports(evidence.scanReceipts, { graphs: evidence.graphs, advisoryPolicy: clone(reviewedAdvisoryPolicy) });
  assert.equal(result.rawUnknownCount, 8); assert.equal(result.severityResolutionReceipts.length, 8);
  assert.deepEqual(result.rawSeverityCounts, { UNKNOWN: 8, LOW: 4, MEDIUM: 0, HIGH: 0, CRITICAL: 0 });
  assert.deepEqual(result.effectiveSeverityCounts, { UNKNOWN: 0, LOW: 8, MEDIUM: 4, HIGH: 0, CRITICAL: 0 });
  assert.equal(new Set(result.severityResolutionReceipts.map(r => r.repository + r.platform + r.vulnerabilityID)).size, 8);
  for (const r of result.severityResolutionReceipts) {
    assert.equal(r.originalSeverity, 'UNKNOWN'); assert.equal(r.cna.name, 'glibc');
    assert.equal(r.policyDigest, advisoryPolicyDigest); assert.equal(r.reachability, 'UNVERIFIED'); assert.equal(r.affectedStatus, 'affected-unfixed');
    assert.match(r.source, /^https:\/\/cveawg.mitre.org\/api\/cve\/CVE-2026-/);
  }
  assert.equal(JSON.stringify(evidence.scanReceipts), raw);
  assert.equal(assertPreparedEvidence(evidence, context), evidence);
});
test('full per-report low/moderate inventories remain visible with eight separately resolved UNKNOWN rows', async () => {
  const { evidence: e } = await preparedFixture(); addVendorUnknowns(e);
  for (const r of e.scanReceipts) r.report.Results[1].Vulnerabilities = [
    ...Array.from({ length: 13 }, (_, i) => ({ VulnerabilityID: 'synthetic-low-' + i, PkgName: 'fixture', InstalledVersion: '1', Severity: 'LOW' })),
    ...Array.from({ length: 49 }, (_, i) => ({ VulnerabilityID: 'synthetic-medium-' + i, PkgName: 'fixture', InstalledVersion: '1', Severity: 'MEDIUM' })),
  ];
  const raw = JSON.stringify(e.scanReceipts);
  const result = validateScanReports(e.scanReceipts, { graphs: e.graphs });
  assert.deepEqual(result.rawSeverityCounts, { UNKNOWN: 8, LOW: 52, MEDIUM: 196, HIGH: 0, CRITICAL: 0 });
  assert.deepEqual(result.effectiveSeverityCounts, { UNKNOWN: 0, LOW: 56, MEDIUM: 200, HIGH: 0, CRITICAL: 0 });
  assert.equal(result.rawUnknownCount, 8); assert.equal(result.severityResolutionReceipts.length, 8);
  assert.equal(JSON.stringify(e.scanReceipts), raw);
});
for (const severity of ['HIGH', 'CRITICAL']) test(`listed CVE raw ${severity} is never downgraded`, async () => {
  const { evidence: e } = await preparedFixture(); addVendorUnknowns(e);
  e.scanReceipts[0].report.Results[0].Vulnerabilities[0].Severity = severity;
  assert.throws(() => validateScanReports(e.scanReceipts, { graphs: e.graphs }), { code: 'SCAN_BLOCKING_FINDING' });
});
for (const [label, edit] of [
  ['unlisted CVE', r => { r.report.Results[0].Vulnerabilities[0].VulnerabilityID = 'CVE-2026-000001'; }],
  ['wrong package', r => { r.report.Results[0].Vulnerabilities[0].PkgName = 'libc-bin'; }],
  ['wrong installed version', r => { r.report.Results[0].Vulnerabilities[0].InstalledVersion = '2.41-12+deb13u3'; }],
  ['wrong OS family', r => { r.report.Metadata.OS.Family = 'ubuntu'; }],
  ['wrong OS version', r => { r.report.Metadata.OS.Name = '12'; }],
  ['missing OS metadata', r => { delete r.report.Metadata.OS; }],
  ['missing severity', r => { delete r.report.Results[0].Vulnerabilities[0].Severity; }],
  ['wrong analyzer', r => { r.report.Results[0].Type = 'ubuntu'; }],
  ['missing package inventory', r => { r.report.Results[0].Packages = [{ Name: 'other', Version: '1' }]; }],
]) test(`UNKNOWN policy fails closed for ${label}`, async () => {
  const { evidence: e } = await preparedFixture(); addVendorUnknowns(e); edit(e.scanReceipts[0]);
  assert.throws(() => validateScanReports(e.scanReceipts, { graphs: e.graphs }));
});
for (const [label, nowMs] of [['expired', Date.parse('2026-11-24T08:32:35Z')], ['before observation', Date.parse('2026-11-17T08:32:34Z')]]) test(`UNKNOWN resolution policy ${label} fails with otherwise fresh scans`, async () => {
  const { evidence: e } = await preparedFixture(); addVendorUnknowns(e);
  for (const r of e.scanReceipts) { r.dbUpdatedAt = new Date(nowMs - 3600000).toISOString(); r.completedAt = new Date(nowMs - 1000).toISOString(); }
  assert.throws(() => validateScanReports(e.scanReceipts, { graphs: e.graphs, nowMs }), { code: 'ADVISORY_POLICY_EXPIRED_OR_NOT_YET_VALID' });
});
for (const [label, edit] of [
  ['empty', p => { p.entries = []; }],
  ['arbitrary CVE', p => { p.entries[0].vulnerabilityID = 'CVE-2026-000001'; }],
  ['arbitrary severity', p => { p.entries[0].effectiveSeverity = 'LOW'; p.entries[1].effectiveSeverity = 'LOW'; }],
  ['extended validity', p => { p.reviewValidUntil = '2027-11-24T08:32:35Z'; }],
  ['raw source hash substitution', p => { p.entries[0].rawSHA256 = '0'.repeat(64); }],
]) test(`policy injection cannot override approved facts: ${label}`, async () => {
  const { evidence: e } = await preparedFixture(); addVendorUnknowns(e); const p = clone(reviewedAdvisoryPolicy); edit(p);
  assert.throws(() => validateScanReports(e.scanReceipts, { graphs: e.graphs, advisoryPolicy: p }), { code: 'ADVISORY_POLICY' });
});

function graphFixture(service = 'resolver', mutate = () => {}) {
  const stored = new Map();
  const put = (kind, value, mediaType, platform) => {
    mutate(kind, value, platform);
    const bytes = Buffer.isBuffer(value) ? Buffer.from(value) : Buffer.from(JSON.stringify(value));
    const d = { digest: sha256(bytes), size: bytes.length, mediaType };
    stored.set(d.digest, { bytes, ...d, kind }); return d;
  };
  const manifests = [];
  for (const platform of platforms) {
    const arch = platform.split('/')[1];
    const layer = put('layer', Buffer.from(`fixture-layer-${service}-${arch}`), 'application/vnd.oci.image.layer.v1.tar', platform);
    const config = put('config', { os: 'linux', architecture: arch, rootfs: { type: 'layers', diff_ids: [layer.digest] } }, CONFIG, platform);
    const child = put('manifest', { schemaVersion: 2, mediaType: OCI_MANIFEST, config, layers: [layer] }, OCI_MANIFEST, platform);
    const runtime = { ...child, platform: { os: 'linux', architecture: arch } };
    manifests.push(runtime);
    const subject = [{ name: `pkg:docker/${service}?platform=${encodeURIComponent(platform)}`, digest: { sha256: child.digest.slice(7) } }];
    const app = `@midnight-ntwrk/midnight-did-${service}-service`;
    const sbom = put('sbom', {
      _type: 'https://in-toto.io/Statement/v0.1', subject: clone(subject), predicateType: SPDX,
      predicate: {
        spdxVersion: 'SPDX-2.3', SPDXID: 'SPDXRef-DOCUMENT', dataLicense: 'CC0-1.0', name: 'fixture-only', documentNamespace: 'https://example.invalid/synthetic',
        creationInfo: { creators: ['Tool: synthetic-fixture'], created: '2026-11-17T00:00:00Z' },
        packages: [{ SPDXID: 'SPDXRef-app', name: app, versionInfo: '1.0.0' }, { SPDXID: 'SPDXRef-node', name: 'node', versionInfo: '24.22.3' }, { SPDXID: 'SPDXRef-did', name: '@midnight-ntwrk/midnight-did', versionInfo: '1.0.0-rc1' }],
        relationships: [{ spdxElementId: 'SPDXRef-DOCUMENT', relationshipType: 'DESCRIBES', relatedSpdxElement: 'SPDXRef-app' }],
      },
    }, INTOTO, platform);
    sbom.annotations = { 'in-toto.io/predicate-type': SPDX };
    const dockerfile = `did-${service}-service/Dockerfile`;
    const prov = put('provenance', {
      _type: 'https://in-toto.io/Statement/v0.1', subject: clone(subject), predicateType: SLSA,
      predicate: {
        buildType: 'https://mobyproject.org/buildkit@v1',
        invocation: { configSource: { uri: GIT, digest: { sha1: SHA }, entryPoint: dockerfile }, parameters: { frontend: 'dockerfile.v0' } },
        materials: [{ uri: GIT, digest: { sha1: SHA } },
          { uri: `pkg:docker/node@24.22.3-bookworm-slim?platform=${encodeURIComponent(platform)}`, digest: { sha256: basePolicy.builder.platforms[platform].manifestDigest.slice(7) } },
          { uri: `pkg:docker/gcr.io/distroless/nodejs24-debian13@nonroot?platform=${encodeURIComponent(platform)}`, digest: { sha256: basePolicy.platforms[platform].manifestDigest.slice(7) } }],
        buildConfig: { llbDefinition: [{ id: 'step0', op: { Op: { source: { identifier: `docker-image://${basePolicy.builder.reference}` } } } }, { id: 'step1', op: { Op: { source: { identifier: `docker-image://${basePolicy.reference}` } } } }] },
        metadata: { completeness: { parameters: true, materials: true, environment: true }, 'https://mobyproject.org/buildkit@v1#metadata': { source: { infos: [{ filename: dockerfile, data: Buffer.from(`FROM node:24.22.3-bookworm-slim@${basePolicy.builder.indexDigest} AS builder\nFROM ${basePolicy.reference} AS runtime\n`).toString('base64') }] } } },
      },
    }, INTOTO, platform);
    prov.annotations = { 'in-toto.io/predicate-type': SLSA };
    const ac = put('attestation-config', { architecture: 'unknown', os: 'unknown', rootfs: { type: 'layers', diff_ids: [] } }, CONFIG, platform);
    const att = put('attestation-manifest', { schemaVersion: 2, mediaType: OCI_MANIFEST, config: ac, layers: [sbom, prov], subject: clone(child) }, OCI_MANIFEST, platform);
    manifests.push({ ...att, platform: { os: 'unknown', architecture: 'unknown' }, annotations: { 'vnd.docker.reference.type': 'attestation-manifest', 'vnd.docker.reference.digest': child.digest } });
  }
  const root = put('index', { schemaVersion: 2, mediaType: OCI_INDEX, manifests }, OCI_INDEX);
  const get = async digest => {
    const v = stored.get(digest); assert.ok(v, 'fixture missing');
    return { ...v, bytes: Buffer.from(v.bytes), json: JSON.parse(v.mediaType.endsWith('json') ? v.bytes.toString() : '{}') };
  };
  const client = { repository: repositories[service === 'resolver' ? 0 : 1], getManifest: get, getBlob: get };
  return { input: { client, indexDigest: root.digest, sourceSHA: SHA, dockerfile: `did-${service}-service/Dockerfile`, service, baseVersion: '1.0.0', basePolicy: clone(basePolicy) }, stored, root };
}
function registryFixture(override = () => undefined, options = {}) {
  const calls = []; const tags = new Map();
  const reply = (body, status = 200, headers = {}) => new Response(Buffer.isBuffer(body) ? body : JSON.stringify(body), { status, headers });
  const fetchImpl = async (url, init) => {
    const call = { url: String(url), ...init }; calls.push(call);
    const overridden = await override(call, { calls, tags, reply });
    if (overridden !== undefined) return overridden;
    const u = new URL(url);
    if (u.pathname === '/token') return reply({ token: 'private-bearer' });
    if (u.pathname.endsWith('/tags/list')) return reply({ name: (options.repository ?? repositories[0]).slice('ghcr.io/'.length), tags: [...tags.keys()] });
    const ref = u.pathname.split('/').at(-1);
    if (init.method === 'PUT') { tags.set(ref, Buffer.from(init.body)); return reply({}, 201, { 'docker-content-digest': sha256(init.body) }); }
    if (tags.has(ref)) return reply(tags.get(ref), 200, { 'content-type': OCI_INDEX, 'docker-content-digest': sha256(tags.get(ref)) });
    return reply({ errors: [{ code: 'MANIFEST_UNKNOWN' }] }, 404);
  };
  return { client: createRegistryClient({ repository: repositories[0], actor: 'fixture-actor', token: 'fixture-secret', fetchImpl, ...options }), calls, tags, reply };
}
const rootBytes = () => Buffer.from(JSON.stringify({ schemaVersion: 2, mediaType: OCI_INDEX, manifests: [] }));
test('fixed latest reader proves absence, and reads occupied latest without arbitrary-tag API', async () => {
  const f = registryFixture();
  assert.equal(await f.client.getLatestManifest(), null);
  f.tags.set('latest', rootBytes());
  assert.equal((await f.client.getLatestManifest()).digest, sha256(rootBytes()));
  await assert.rejects(f.client.getManifest('latest'), { code: 'INVALID_REFERENCE' });
  assert.equal(f.calls.filter(c => c.method === 'PUT').length, 0);
  assert.equal(f.calls.filter(c => c.url.includes('/manifests/')).every(c => c.url.endsWith('/latest')), true);
});
for (const version of [VERSION, '1.0.0-rc.2', '01.0.0', 'latest', '1.0.0\n', '1.0.0/other', '1.0.0-beta.1']) test(`stable latest rejects nonstable/canonical input ${JSON.stringify(version)} before auth`, async () => {
  const f = registryFixture();
  await assert.rejects(f.client.promoteStableLatest(version, promotion()), { code: 'STABLE_VERSION_REQUIRED' });
  assert.equal(f.calls.length, 0);
});
for (const mode of ['missing', 'drift']) test(`latest requires immutable stable version first: ${mode}`, async () => {
  const f = registryFixture();
  if (mode === 'drift') f.tags.set('1.0.0', Buffer.from(JSON.stringify({ schemaVersion: 2, mediaType: OCI_INDEX, manifests: [], drift: true })));
  await assert.rejects(f.client.promoteStableLatest('1.0.0', promotion()), e => { assert.notEqual(e.publicationMayHaveOccurred, true); return true; });
  assert.equal(f.calls.filter(c => c.method === 'PUT' || c.url.endsWith('/latest')).length, 0);
});
for (const previous of [null, rootBytes(), Buffer.from(JSON.stringify({ schemaVersion: 2, mediaType: OCI_INDEX, manifests: [], previous: true }))]) test(`stable latest exact bytes and previous digest ${previous === null ? 'absent' : sha256(previous)}`, async () => {
  const f = registryFixture(); const p = promotion(); f.tags.set('1.0.0', p.bytes);
  if (previous) f.tags.set('latest', previous);
  const result = await f.client.promoteStableLatest('1.0.0', p);
  assert.deepEqual(result, { repository: repositories[0], version: '1.0.0', tag: 'latest', digest: p.digest, previousDigest: previous ? sha256(previous) : null, confirmed: true });
  assert.deepEqual(f.tags.get('latest'), p.bytes);
  assert.deepEqual(f.tags.get('1.0.0'), p.bytes);
  const puts = f.calls.filter(c => c.method === 'PUT');
  assert.equal(puts.length, 1); assert.equal(puts[0].url.endsWith('/manifests/latest'), true);
  assert.equal(f.calls.find(c => c.url.includes('/manifests/')).url.endsWith('/1.0.0'), true);
  assert.ok(f.calls.findIndex(c => c.url.endsWith('/1.0.0')) < f.calls.indexOf(puts[0]));
});
for (const status of [401, 403, 429, 500, 302]) test(`latest read ${status} blocks write`, async () => {
  const f = registryFixture((c, { reply }) => c.url.endsWith('/latest') ? reply({}, status) : undefined);
  f.tags.set('1.0.0', rootBytes());
  await assert.rejects(f.client.getLatestManifest());
  await assert.rejects(f.client.promoteStableLatest('1.0.0', promotion()));
  assert.equal(f.calls.filter(c => c.method === 'PUT').length, 0);
});
for (const status of [401, 403, 404]) test(`latest repository-access ${status} blocks write despite stable read`, async () => {
  const f = registryFixture((c, { reply }) => c.url.includes('/tags/list') ? reply({}, status) : undefined);
  f.tags.set('1.0.0', rootBytes());
  await assert.rejects(f.client.promoteStableLatest('1.0.0', promotion()));
  assert.equal(f.calls.filter(c => c.method === 'PUT').length, 0);
});
for (const body of [{}, { errors: [{ code: 'NAME_UNKNOWN' }] }, { errors: [{ code: 'MANIFEST_UNKNOWN' }, { code: 'DENIED' }] }, '<html>404</html>']) test(`latest masked 404 fails closed ${JSON.stringify(body)}`, async () => {
  const f = registryFixture((c, { reply }) => c.url.endsWith('/latest') ? reply(body, 404) : undefined);
  f.tags.set('1.0.0', rootBytes());
  await assert.rejects(f.client.getLatestManifest());
  await assert.rejects(f.client.promoteStableLatest('1.0.0', promotion()));
  assert.equal(f.calls.filter(c => c.method === 'PUT').length, 0);
});
for (const mode of ['throw', '500', 'header', 'readback-throw', 'readback-drift', 'readback-absent']) test(`latest ambiguous ${mode} is single-attempt and records previous digest`, async () => {
  const old = Buffer.from(JSON.stringify({ schemaVersion: 2, mediaType: OCI_INDEX, manifests: [], old: true }));
  const f = registryFixture((c, { reply, calls }) => {
    if (c.method === 'PUT') {
      assert.equal(c.url.endsWith('/latest'), true);
      if (mode === 'throw') throw new Error('fixture-secret');
      if (mode === '500') return reply({}, 500);
      if (mode === 'header') return reply({}, 201, { 'docker-content-digest': 'sha256:' + '0'.repeat(64) });
    }
    if (c.method === 'GET' && c.url.endsWith('/latest') && calls.some(c => c.method === 'PUT')) {
      if (mode === 'readback-throw') throw new Error('fixture-secret https://signed.invalid');
      if (mode === 'readback-absent') return reply({ errors: [{ code: 'MANIFEST_UNKNOWN' }] }, 404);
      if (mode === 'readback-drift') return reply(old, 200, { 'content-type': OCI_INDEX });
    }
  });
  f.tags.set('1.0.0', rootBytes()); f.tags.set('latest', old);
  await assert.rejects(f.client.promoteStableLatest('1.0.0', promotion()), error => {
    assert.equal(error.publicationMayHaveOccurred, true); assert.equal(error.previousDigest, sha256(old));
    assert.doesNotMatch(error.message, /fixture-secret|https:/); return true;
  });
  assert.equal(f.calls.filter(c => c.method === 'PUT').length, 1);
  assert.deepEqual(f.tags.get('1.0.0'), rootBytes());
});
test('latest rejects altered candidate bytes/digest/type before any request', async () => {
  for (const change of [p => { p.bytes[0] ^= 1; }, p => { p.digest = 'sha256:' + '0'.repeat(64); }, p => { p.mediaType = OCI_MANIFEST; }]) {
    const f = registryFixture(); const p = promotion(); change(p);
    await assert.rejects(f.client.promoteStableLatest('1.0.0', p)); assert.equal(f.calls.length, 0);
  }
});

const promotion = () => { const bytes = rootBytes(); return { bytes, digest: sha256(bytes), mediaType: OCI_INDEX }; };

test('registry authenticates canonical token and repository before absence, with no secret properties', async () => {
  const f = registryFixture();
  assert.deepEqual(await f.client.assertVersionAbsent(VERSION), { repository: repositories[0], version: VERSION, absent: true });
  assert.equal(f.calls.length, 3);
  assert.equal(new URL(f.calls[0].url).origin + new URL(f.calls[0].url).pathname, 'https://ghcr.io/token');
  assert.equal(new URL(f.calls[0].url).searchParams.get('scope'), 'repository:midnightntwrk/midnight-did-resolver-service:pull,push');
  assert.match(f.calls[0].headers.Authorization, /^Basic /);
  assert.equal(f.calls[1].headers.Authorization, 'Bearer private-bearer');
  assert.equal(JSON.stringify(f.client).includes('secret'), false);
});
for (const repository of ['evil.example/a', `${repositories[0]}:latest`, 'ghcr.io/other/name']) test(`repository reject ${repository}`, () => assert.throws(() => createRegistryClient({ repository, actor: 'a', token: 't' })));
for (const version of ['latest', '01.0.0', '1.0.0-rc.0', '1.0.0-rc.01', '1.0.0\n', '1.0.0/evil', 'sha256:' + 'A'.repeat(64)]) test(`reference reject ${JSON.stringify(version)}`, async () => {
  const f = registryFixture(); await assert.rejects(f.client.getManifest(version)); assert.equal(f.calls.length, 0);
});
for (const status of [401, 403, 404, 429, 500, 302]) test(`access ${status} cannot prove absence`, async () => {
  const f = registryFixture((c, { reply }) => c.url.includes('/tags/list') ? reply({}, status) : undefined);
  await assert.rejects(f.client.assertVersionAbsent(VERSION)); assert.equal(f.calls.filter(c => c.method === 'PUT').length, 0);
});
for (const status of [401, 403, 429, 500, 302]) test(`manifest ${status} cannot prove absence`, async () => {
  const f = registryFixture((c, { reply }) => c.url.includes('/manifests/') ? reply({}, status) : undefined);
  await assert.rejects(f.client.assertVersionAbsent(VERSION));
});
for (const body of [{}, { errors: [{ code: 'NAME_UNKNOWN' }] }, { errors: [{ code: 'MANIFEST_UNKNOWN' }, { code: 'DENIED' }] }, '<html>404</html>']) test(`masked 404 ${JSON.stringify(body)}`, async () => {
  const f = registryFixture((c, { reply }) => c.url.includes('/manifests/') ? reply(body, 404) : undefined);
  await assert.rejects(f.client.assertVersionAbsent(VERSION));
});
test('matching, conflicting and partial occupied tags always stop with zero PUTs', async () => {
  for (const bytes of [rootBytes(), Buffer.from('different')]) {
    const f = registryFixture(); f.tags.set(VERSION, bytes);
    await assert.rejects(f.client.promoteIndex(VERSION, promotion()), { code: 'VERSION_EXISTS' });
    assert.equal(f.calls.filter(c => c.method === 'PUT').length, 0);
  }
});
test('exact-byte single PUT and readback', async () => {
  const f = registryFixture(); const input = promotion();
  assert.equal((await f.client.promoteIndex(VERSION, input)).confirmed, true);
  assert.deepEqual(f.tags.get(VERSION), input.bytes);
  assert.equal(f.calls.filter(c => c.method === 'PUT').length, 1);
  await assert.rejects(f.client.promoteIndex(VERSION, input), { code: 'VERSION_EXISTS' });
});
for (const mode of ['throw', '500', 'drift', 'readback-throw', 'put-header']) test(`ambiguous promotion ${mode}, never retries`, async () => {
  const f = registryFixture((c, { reply, calls }) => {
    if (c.method === 'PUT') {
      if (mode === 'throw') throw new Error('fixture-secret https://signed.invalid/?token=secret');
      if (mode === '500') return reply({}, 500);
      if (mode === 'put-header') return reply({}, 201, { 'docker-content-digest': 'sha256:' + '0'.repeat(64) });
    }
    if (calls.some(c => c.method === 'PUT') && c.method === 'GET') {
      if (mode === 'readback-throw') throw new Error('private-bearer');
      if (mode === 'drift') return reply(Buffer.from(JSON.stringify({ schemaVersion: 2, mediaType: OCI_INDEX, manifests: [], changed: true })), 200, { 'content-type': OCI_INDEX });
    }
  });
  await assert.rejects(f.client.promoteIndex(VERSION, promotion()), error => {
    assert.equal(error.publicationMayHaveOccurred, true); assert.doesNotMatch(error.message, /fixture-secret|private-bearer|https:/); return true;
  });
  assert.equal(f.calls.filter(c => c.method === 'PUT').length, 1);
});
test('either existing tag blocks a pair preflight, including matching and partial pairs', async () => {
  for (const occupied of [[0], [1], [0, 1]]) {
    const pair = repositories.map(repository => registryFixture(undefined, { repository }));
    for (const index of occupied) pair[index].tags.set(VERSION, rootBytes());
    await assert.rejects(Promise.all(pair.map(f => f.client.assertVersionAbsent(VERSION))));
    assert.equal(pair.flatMap(f => f.calls).filter(c => c.method === 'PUT').length, 0);
  }
});
for (const path of ['/token', '/tags/list', '/manifests/']) test(`null JSON at ${path} fails sanitized`, async () => {
  const f = registryFixture((c, { reply }) => c.url.includes(path) ? reply(null, path === '/manifests/' ? 404 : 200) : undefined);
  await assert.rejects(f.client.assertVersionAbsent(VERSION), error => { assert.equal(error.name, 'RegistryError'); return true; });
});
test('external error cannot impersonate sanitized registry error', async () => {
  const f = registryFixture(() => { const e = new Error('fixture-secret https://signed.invalid'); e.name = 'RegistryError'; throw e; });
  await assert.rejects(f.client.assertVersionAbsent(VERSION), { name: 'RegistryError', code: 'TRANSPORT', message: 'Registry operation failed: TRANSPORT' });
});
test('invalid bytes fail before auth or PUT', async () => {
  const f = registryFixture(); const p = promotion(); p.bytes[0] ^= 1;
  await assert.rejects(f.client.promoteIndex(VERSION, p)); assert.equal(f.calls.length, 0);
});
for (const mode of ['hash', 'header', 'size', 'media', 'huge']) test(`raw manifest validation ${mode}`, async () => {
  const bytes = rootBytes();
  const f = registryFixture((c, { reply }) => c.url.includes('/manifests/') ? reply(bytes, 200, {
    'content-type': mode === 'media' ? 'text/html' : OCI_INDEX,
    ...(mode === 'header' ? { 'docker-content-digest': 'sha256:' + '0'.repeat(64) } : {}),
    ...(mode === 'size' ? { 'content-length': String(bytes.length + 1) } : {}),
    ...(mode === 'huge' ? { 'content-length': '999999999' } : {}),
  }) : undefined);
  await assert.rejects(f.client.getManifest(mode === 'hash' ? 'sha256:' + '0'.repeat(64) : VERSION));
});
test('bounded body stream without Content-Length', async () => {
  const f = registryFixture((c, { reply }) => c.url.includes('/manifests/') ? reply(Buffer.alloc(300)) : undefined, { maxBytes: 256 });
  await assert.rejects(f.client.getManifest(VERSION), { code: 'BODY_LIMIT' });
});
test('timeout sanitized and bounded', async () => {
  const f = registryFixture(() => new Promise(() => {}), { timeoutMs: 10 });
  await assert.rejects(f.client.assertVersionAbsent(VERSION), { code: 'TIMEOUT' });
});
test('token redirect never follows or leaks Basic', async () => {
  const f = registryFixture((c, { reply }) => reply({}, 302, { location: 'https://evil.invalid' }));
  await assert.rejects(f.client.assertVersionAbsent(VERSION)); assert.equal(f.calls.length, 1);
});
test('allowlisted blob redirect strips all Authorization', async () => {
  const bytes = Buffer.from('blob'); const digest = sha256(bytes);
  const f = registryFixture((c, { reply }) => {
    if (new URL(c.url).hostname === 'ghcr.io' && c.url.includes('/blobs/')) return reply({}, 307, { location: 'https://pkg-containers.githubusercontent.com/ghcr1/blobs/x?sig=private' });
    if (c.url.includes('pkg-containers')) { assert.equal(c.headers.Authorization, undefined); return reply(bytes); }
  });
  assert.deepEqual((await f.client.getBlob(digest)).bytes, bytes);
});
for (const url of ['https://evil.invalid/blob', 'http://pkg-containers.githubusercontent.com/x', 'https://pkg-containers.githubusercontent.com.evil.invalid/x', 'https://user:pw@pkg-containers.githubusercontent.com/x', 'https://pkg-containers.githubusercontent.com:444/x']) test(`blob redirect denied ${url}`, async () => {
  const f = registryFixture((c, { reply }) => c.url.includes('/blobs/') ? reply({}, 307, { location: url }) : undefined);
  await assert.rejects(f.client.getBlob(sha256(Buffer.from('x')))); assert.equal(f.calls.length, 2);
});
test('blob corruption, redirect cycle rejected', async () => {
  const f = registryFixture((c, { reply }) => c.url.includes('/blobs/') ? reply(Buffer.from('wrong')) : undefined);
  await assert.rejects(f.client.getBlob(sha256(Buffer.from('x'))), { code: 'DIGEST_MISMATCH' });
  const cycle = registryFixture((c, { reply }) => c.url.includes('/blobs/') || c.url.includes('pkg-containers') ? reply({}, 307, { location: 'https://pkg-containers.githubusercontent.com/x' }) : undefined);
  await assert.rejects(cycle.client.getBlob(sha256(Buffer.from('x'))));
});

test('synthetic metadata/attestation graph validates retained raw artifacts and both platforms', async () => {
  const f = graphFixture(); const artifacts = [];
  const receipt = await verifyImageGraph({ ...f.input, onArtifact: async a => artifacts.push(a) });
  assert.equal(receipt.platforms.length, 2); assert.equal(receipt.verified, true);
  assert.equal(artifacts.length, 13); // root + two × (manifest/config/attmanifest/attconfig/SPDX/SLSA)
  assert.equal(receipt.layerPayloadVerification, 'consumer-required');
  assert.ok(receipt.platforms.every(p => p.runtimeLayers.length === 1));
  assert.equal(artifacts.some(a => a.kind === 'layer'), false);
  for (const a of artifacts) assert.equal(sha256(a.bytes), a.digest);
});
test('runtime filesystem corruption is consumer-required, never claimed as graph-verified', async () => {
  const f = graphFixture(); const requested = []; const getBlob = f.input.client.getBlob;
  for (const v of f.stored.values()) if (v.kind === 'layer') v.bytes[0] ^= 1;
  f.input.client.getBlob = async digest => {
    requested.push(digest);
    assert.notEqual(f.stored.get(digest).kind, 'layer', 'metadata graph must not download filesystem blobs');
    return getBlob(digest);
  };
  const graph = await verifyImageGraph(f.input);
  assert.equal(graph.layerPayloadVerification, 'consumer-required');
  assert.ok(graph.platforms.every(p => p.runtimeLayers.every(d => !requested.includes(d.digest))));
});
test('large runtime descriptors do not allocate or download layer payloads', async () => {
  const f = graphFixture('resolver', (kind, v) => { if (kind === 'manifest') v.layers[0].size = 1024 ** 3; });
  const getBlob = f.input.client.getBlob;
  f.input.client.getBlob = async digest => { assert.notEqual(f.stored.get(digest).kind, 'layer'); return getBlob(digest); };
  const graph = await verifyImageGraph(f.input);
  assert.equal(graph.platforms[0].runtimeLayers[0].size, 1024 ** 3);
});
for (const kind of ['index', 'manifest', 'config', 'attestation-manifest', 'attestation-config', 'sbom', 'provenance']) test(`corrupt raw ${kind} rejected`, async () => {
  const f = graphFixture(); const target = [...f.stored.values()].find(v => v.kind === kind); target.bytes[0] ^= 1;
  await assert.rejects(verifyImageGraph(f.input));
});
const graphMutations = [
  ['root schema', 'index', v => { v.schemaVersion = 1; }],
  ['layer count budget', 'manifest', v => { v.layers = Array(257).fill(v.layers[0]); }],
  ['graph byte budget', 'manifest', v => { v.config.size = 3 * 1024 ** 3; }],
  ['root platform missing', 'index', v => { v.manifests.pop(); }],
  ['duplicate architecture', 'index', v => { v.manifests[2].platform.architecture = 'amd64'; }],
  ['descriptor size', 'index', v => { v.manifests[0].size++; }],
  ['descriptor URL', 'index', v => { v.manifests[0].urls = ['https://evil.invalid']; }],
  ['orphan attestation', 'index', v => { v.manifests[1].annotations['vnd.docker.reference.digest'] = 'sha256:' + '0'.repeat(64); }],
  ['cross-platform attestation', 'index', v => { v.manifests[1].annotations['vnd.docker.reference.digest'] = v.manifests[2].digest; }],
  ['forged annotation', 'index', v => { v.manifests[1].annotations['vnd.docker.reference.type'] = 'runtime'; }],
  ['child schema', 'manifest', v => { v.schemaVersion = 1; }],
  ['config architecture', 'config', v => { v.architecture = 's390x'; }],
  ['config missing diff_ids', 'config', v => { delete v.rootfs.diff_ids; }],
  ['config malformed diff_id', 'config', v => { v.rootfs.diff_ids[0] = 'sha256:invalid'; }],
  ['config diff_ids count', 'config', v => { v.rootfs.diff_ids = []; }],
  ['config size', 'manifest', v => { v.config.size++; }],
  ['invalid runtime layer size', 'manifest', v => { v.layers[0].size = -1; }],
  ['zero runtime layer size', 'manifest', v => { v.layers[0].size = 0; }],
  ['invalid runtime layer digest', 'manifest', v => { v.layers[0].digest = 'sha256:not-a-digest'; }],
  ['external runtime layer URL', 'manifest', v => { v.layers[0].urls = ['https://evil.invalid']; }],
  ['unsupported runtime layer media', 'manifest', v => { v.layers[0].mediaType = 'application/octet-stream'; }],
  ['wrong attestation subject', 'attestation-manifest', v => { v.subject.digest = 'sha256:' + '1'.repeat(64); }],
  ['missing provenance', 'attestation-manifest', v => { v.layers.pop(); }],
  ['predicate annotation', 'attestation-manifest', v => { v.layers[0].annotations['in-toto.io/predicate-type'] = SLSA; }],
  ['unknown attestation schema', 'provenance', v => { v.predicateType = 'https://slsa.dev/provenance/v1'; }],
  ['wrong statement subject', 'sbom', v => { v.subject[0].digest.sha256 = '2'.repeat(64); }],
  ['weak SPDX', 'sbom', v => { v.predicate.packages = []; }],
  ['SPDX no app', 'sbom', v => { v.predicate.packages[0].name = 'unrelated'; }],
  ['SPDX wrong node', 'sbom', v => { v.predicate.packages[1].versionInfo = '20.0.0'; }],
  ['SPDX missing relationships', 'sbom', v => { delete v.predicate.relationships; }],
  ['SPDX unknown version', 'sbom', v => { v.predicate.spdxVersion = 'SPDX-3.0'; }],
  ['missing Git SHA', 'provenance', v => { delete v.predicate.invocation.configSource.digest; }],
  ['Git material missing', 'provenance', v => { v.predicate.materials.shift(); }],
  ['Git material wrong source', 'provenance', v => { v.predicate.materials[0].digest.sha1 = 'b'.repeat(40); }],
  ['additional substituted Git source', 'provenance', v => { v.predicate.materials.push({ uri: 'https://github.com/evil/repo.git#main', digest: { sha1: SHA } }); }],
  ['extra foreign LLB image input', 'provenance', v => { v.predicate.buildConfig.llbDefinition.push({ op: { Op: { source: { identifier: 'docker-image://evil/image:latest' } } } }); }],
  ['Git local hints only', 'provenance', v => { v.predicate.invocation.configSource = { entryPoint: 'Dockerfile' }; v.predicate.metadata.vcs = { revision: SHA }; }],
  ['wrong Dockerfile', 'provenance', v => { v.predicate.invocation.configSource.entryPoint = 'evil/Dockerfile'; }],
  ['unknown build type', 'provenance', v => { v.predicate.buildType = 'custom'; }],
  ['min mode', 'provenance', v => { delete v.predicate.buildConfig; }],
  ['incomplete materials', 'provenance', v => { v.predicate.metadata.completeness.materials = false; }],
  ['wrong platform base', 'provenance', v => { v.predicate.materials[1].digest.sha256 = '0'.repeat(64); }],
  ['missing builder material', 'provenance', v => { v.predicate.materials.splice(1, 1); }],
  ['missing runtime material', 'provenance', v => { v.predicate.materials.pop(); }],
  ['runtime material wrong child', 'provenance', v => { v.predicate.materials[2].digest.sha256 = basePolicy.builder.indexDigest.slice(7); }],
  ['runtime material wrong ref', 'provenance', v => { v.predicate.materials[2].uri = 'pkg:docker/gcr.io/distroless/nodejs24-debian13@latest?platform=linux%2Famd64'; }],
  ['runtime material wrong architecture', 'provenance', v => { v.predicate.materials[2].uri = 'pkg:docker/gcr.io/distroless/nodejs24-debian13@nonroot?platform=linux%2Fs390x'; }],
  ['material duplicated platform qualifier', 'provenance', v => { v.predicate.materials[2].uri += '&platform=linux%2Farm64'; }],
  ['missing builder LLB', 'provenance', v => { v.predicate.buildConfig.llbDefinition.shift(); }],
  ['missing runtime LLB', 'provenance', v => { v.predicate.buildConfig.llbDefinition.pop(); }],
  ['swapped LLB child roles', 'provenance', v => { v.predicate.buildConfig.llbDefinition[1].op.Op.source.identifier = 'docker-image://' + basePolicy.reference.split('@')[0] + '@' + basePolicy.builder.indexDigest; }],
  ['base material architecture', 'provenance', v => { v.predicate.materials[1].uri = 'pkg:docker/node@24.22.3-bookworm-slim?platform=linux%2Fs390x'; }],
  ['unpinned base', 'provenance', v => { v.predicate.buildConfig.llbDefinition[0].op.Op.source.identifier = 'docker-image://node:latest'; }],
  ['wrong Dockerfile base', 'provenance', v => { v.predicate.metadata['https://mobyproject.org/buildkit@v1#metadata'].source.infos[0].data = Buffer.from('FROM node:latest').toString('base64'); }],
  ['Dockerfile base literal tag periods', 'provenance', v => { v.predicate.metadata['https://mobyproject.org/buildkit@v1#metadata'].source.infos[0].data = Buffer.from(`FROM node:24x22y3-bookworm-slim@${basePolicy.builder.indexDigest} AS builder\nFROM ${basePolicy.reference} AS runtime\n`).toString('base64'); }],
];
for (const [label, kind, change] of graphMutations) test(`graph rejects ${label}`, async () => {
  const f = graphFixture('resolver', (k, value) => { if (k === kind) change(value); });
  await assert.rejects(verifyImageGraph(f.input));
});
for (const [label, change] of [
  ['root digest', i => { i.indexDigest = 'sha256:' + '0'.repeat(64); }],
  ['service substitution', i => { i.service = 'manager'; }],
  ['source malformed', i => { i.sourceSHA = 'a'.repeat(39); }],
  ['one platform', i => { i.platforms = ['linux/amd64']; }],
  ['base policy substitution', i => { i.basePolicy.indexDigest = 'sha256:' + '0'.repeat(64); }],
  ['base child substitution', i => { i.basePolicy.platforms['linux/arm64'].manifestDigest = 'sha256:' + '0'.repeat(64); }],
  ['missing builder role', i => { delete i.basePolicy.builder; }],
  ['builder node version drift', i => { i.basePolicy.builder.nodeVersion = '24.22.2'; }],
  ['runtime node version drift', i => { i.basePolicy.nodeVersion = '24.22.2'; }],
  ['builder child drift', i => { i.basePolicy.builder.platforms['linux/arm64'].manifestDigest = i.basePolicy.platforms['linux/arm64'].manifestDigest; }],
  ['swapped roles', i => { const b = i.basePolicy.builder; i.basePolicy.builder = { ...i.basePolicy }; delete i.basePolicy.builder.builder; Object.assign(i.basePolicy, b); }],
  ['extra role', i => { i.basePolicy.other = clone(i.basePolicy.builder); }],
  ['schema drift', i => { i.basePolicy.schemaVersion = 2; }],
]) test(`graph rejects input ${label}`, async () => { const f = graphFixture(); change(f.input); await assert.rejects(verifyImageGraph(f.input)); });

async function preparedFixture() {
  const graphs = await Promise.all(['resolver', 'manager'].map(s => verifyImageGraph(graphFixture(s).input)));
  const now = TEST_NOW;
  const scanReceipts = graphs.flatMap(g => g.platforms.map(p => ({
    repository: g.repository, platform: p.platform, manifestDigest: p.manifestDigest, scannerVersion: '0.67.2', dbUpdatedAt: new Date(now - 3600000).toISOString(), completedAt: new Date(now - 1000).toISOString(), exitCode: 0,
    report: { SchemaVersion: 2, ArtifactType: 'container_image', ArtifactName: `${g.repository}@${p.manifestDigest}`, Metadata: { ImageID: p.configDigest, ImageConfig: { architecture: p.platform.split('/')[1], os: 'linux' }, RepoDigests: [`${g.repository}@${p.manifestDigest}`] }, Results: [
      { Target: 'Debian', Class: 'os-pkgs', Type: 'debian', Packages: [{ Name: 'libc6', Version: '2.36' }] },
      { Target: 'node_modules/fixture/package.json', Class: 'lang-pkgs', Type: 'node-pkg', Packages: [{ Name: 'fixture', Version: '1.0.0' }], Vulnerabilities: [{ VulnerabilityID: 'fixture-low', PkgName: 'fixture', InstalledVersion: '1.0.0', Severity: 'LOW' }] },
    ] },
  })));
  const context = { repository: 'midnightntwrk/midnight-did-resolver', resolver_repository: repositories[0], manager_repository: repositories[1], source_sha: SHA, ref: 'refs/heads/develop', channel: 'rc', version: VERSION, base: '1.0.0', platforms: platforms.join(','), identity: 'https://github.com/midnightntwrk/midnight-did-resolver/.github/workflows/release-docker.yml@refs/heads/develop', issuer: 'https://token.actions.githubusercontent.com', runID: '123', runAttempt: '1' };
  const evidence = { schemaVersion: 1, repository: context.repository, sourceSHA: SHA, sourceRef: context.ref, channel: context.channel, version: VERSION, base: '1.0.0', runID: '123', runAttempt: '1', graphs, scanReceipts,
    runtimeReceipts: graphs.flatMap(g => g.platforms.map(p => ({ repository: g.repository, platform: p.platform, manifestDigest: p.manifestDigest, digestPullVerified: true, pulledReference: `${g.repository}@${p.manifestDigest}`, verified: true, health: true, ready: true, restart: true, recreate: true, persistence: true, ...(g.service === 'resolver' ? { persistenceMode: 'stateless-external-state' } : {}), architectureMode: p.platform === 'linux/amd64' ? 'native-amd64' : 'qemu-arm64', processArch: p.platform === 'linux/amd64' ? 'x64' : 'arm64' }))),
    signatureReceipts: graphs.map(g => ({ repository: g.repository, indexDigest: g.indexDigest, verified: true, identity: context.identity, issuer: context.issuer, sourceSHA: SHA, event: 'workflow_dispatch' })), verified: true,
  };
  return { context, evidence };
}
test('scan reports preserve low findings and evidence validates complete pair', async () => {
  const { context, evidence } = await preparedFixture();
  const scans = validateScanReports(evidence.scanReceipts, { graphs: evidence.graphs });
  assert.deepEqual([scans.verified, scans.platformCount, scans.high, scans.critical, scans.rawUnknownCount], [true, 4, 0, 0, 0]);
  assert.equal(scans.rawSeverityCounts.LOW, 4);
  assert.equal(scans.effectiveSeverityCounts.LOW, 4);
  assert.deepEqual(scans.severityResolutionReceipts, []);
  assert.equal(evidence.scanReceipts[0].report.Results[1].Vulnerabilities.length, 1);
  assert.equal(assertPreparedEvidence(evidence, context), evidence);
});
const scanMutations = [
  ['exit failure', (r) => { r[3].exitCode = 1; }],
  ['missing report', r => { delete r[0].report; }],
  ['missing Results', r => { delete r[0].report.Results; }],
  ['empty Results', r => { r[0].report.Results = []; }],
  ['wrong schema', r => { r[0].report.SchemaVersion = 1; }],
  ['no packages', r => { delete r[0].report.Results[0].Packages; }],
  ['missing scanner version', r => { delete r[0].scannerVersion; }],
  ['DB unknown', r => { r[0].dbUpdatedAt = 'unknown'; }],
  ['DB stale', r => { r[0].dbUpdatedAt = '2020-01-01T00:00:00Z'; }],
  ['DB future', r => { r[0].dbUpdatedAt = '2099-01-01T00:00:00Z'; }],
  ['missing completed time', r => { delete r[0].completedAt; }],
  ['wrong ArtifactName', r => { r[0].report.ArtifactName = 'some:tag'; }],
  ['wrong ImageID', r => { r[0].report.Metadata.ImageID = 'sha256:' + '0'.repeat(64); }],
  ['wrong RepoDigests', r => { r[0].report.Metadata.RepoDigests = ['other@sha256:' + '0'.repeat(64)]; }],
  ['architecture collapsed', r => { r[1].report.Metadata.ImageConfig.architecture = 'amd64'; }],
  ['digest substitution', r => { r[1].manifestDigest = r[0].manifestDigest; }],
  ['duplicate receipt', r => { r[1] = clone(r[0]); }],
  ['missing receipt', r => { r.pop(); }],
  ['HIGH arm64 unfixed', r => { r[1].report.Results[1].Vulnerabilities[0].Severity = 'HIGH'; }],
  ['CRITICAL manager unfixed', r => { r[3].report.Results[1].Vulnerabilities[0].Severity = 'CRITICAL'; }],
  ['UNKNOWN severity', r => { r[0].report.Results[1].Vulnerabilities[0].Severity = 'UNKNOWN'; }],
  ['unsupported severity', r => { r[0].report.Results[1].Vulnerabilities[0].Severity = 'NEGLIGIBLE'; }],
  ['suppression', r => { r[0].report.Results[1].ExperimentalModifiedFindings = [{}]; }],
  ['warnings', r => { r[0].warnings = ['unsupported analyzer']; }],
  ['missing language analyzer', r => { r[0].report.Results.pop(); }],
];
for (const [label, change] of scanMutations) test(`scan rejects ${label}`, async () => { const { evidence: e } = await preparedFixture(); change(e.scanReceipts); assert.throws(() => validateScanReports(e.scanReceipts, { graphs: e.graphs })); });
const evidenceMutations = [
  ['fake verified bool', e => { e.verified = 'true'; }],
  ['graph source substitution', e => { e.graphs[0].sourceSHA = 'b'.repeat(40); }],
  ['graph root substitution', e => { e.graphs[0].indexDigest = 'sha256:' + '0'.repeat(64); }],
  ['graph service substitution', e => { e.graphs[0].service = 'manager'; }],
  ['graph missing attestation', e => { e.graphs[0].platforms[0].attestations = []; }],
  ['fake SPDX inventory count', e => { e.graphs[0].platforms[0].attestations[0].spdxPackageCount = Infinity; }],
  ['scan missing', e => { e.scanReceipts = []; }],
  ['graph falsely claims payloads verified', e => { e.graphs[0].layerPayloadVerification = 'verified'; }],
  ['graph layer descriptors missing', e => { delete e.graphs[0].platforms[0].runtimeLayers; }],
  ['runtime missing', e => { e.runtimeReceipts.pop(); }],
  ['runtime digest pull missing', e => { delete e.runtimeReceipts[0].digestPullVerified; }],
  ['runtime digest pull false', e => { e.runtimeReceipts[0].digestPullVerified = false; }],
  ['runtime digest pull fake boolean', e => { e.runtimeReceipts[0].digestPullVerified = 'true'; }],
  ['runtime pulled reference missing', e => { delete e.runtimeReceipts[0].pulledReference; }],
  ['runtime pulled mutable tag', e => { e.runtimeReceipts[0].pulledReference = `${repositories[0]}:${VERSION}`; }],
  ['runtime pulled other platform', e => { e.runtimeReceipts[0].pulledReference = e.runtimeReceipts[1].pulledReference; }],
  ['runtime pulled other repository', e => { e.runtimeReceipts[0].pulledReference = `${repositories[1]}@${e.runtimeReceipts[0].manifestDigest}`; }],
  ['runtime fake boolean', e => { e.runtimeReceipts[0].verified = 'true'; }],
  ['health only runtime', e => { e.runtimeReceipts[0].ready = false; }],
  ['runtime restart skipped', e => { e.runtimeReceipts[0].restart = false; }],
  ['runtime recreate skipped', e => { e.runtimeReceipts[0].recreate = false; }],
  ['runtime persistence failure', e => { e.runtimeReceipts[2].persistence = false; }],
  ['resolver fake file persistence', e => { delete e.runtimeReceipts[0].persistenceMode; }],
  ['runtime substituted child', e => { e.runtimeReceipts[0].manifestDigest = e.runtimeReceipts[1].manifestDigest; }],
  ['runtime wrong processArch', e => { e.runtimeReceipts[1].processArch = 'x64'; }],
  ['runtime fake native arm64', e => { e.runtimeReceipts[1].architectureMode = 'native-amd64'; }],
  ['runtime duplicate', e => { e.runtimeReceipts[1] = clone(e.runtimeReceipts[0]); }],
  ['signature missing', e => { e.signatureReceipts.pop(); }],
  ['signature wrong root', e => { e.signatureReceipts[0].indexDigest = e.graphs[1].indexDigest; }],
  ['signature duplicate', e => { e.signatureReceipts[1] = clone(e.signatureReceipts[0]); }],
  ['signature fake verified', e => { e.signatureReceipts[0].verified = 'true'; }],
  ['signature identity', e => { e.signatureReceipts[0].identity += 'evil'; }],
  ['signature issuer', e => { e.signatureReceipts[0].issuer = 'https://evil.invalid'; }],
  ['signature source SHA', e => { e.signatureReceipts[0].sourceSHA = 'b'.repeat(40); }],
  ['signature event', e => { e.signatureReceipts[0].event = 'push'; }],
  ['run substitution', e => { e.runID = '999'; }],
  ['attempt substitution', e => { e.runAttempt = '2'; }],
  ['version substitution', e => { e.version = '1.0.0-rc.2'; }],
];
for (const [label, change] of evidenceMutations) test(`prepared rejects ${label}`, async () => { const { context, evidence } = await preparedFixture(); change(evidence); assert.throws(() => assertPreparedEvidence(evidence, context)); });
for (const [label, change] of [
  ['repo', c => { c.resolver_repository = repositories[1]; }],
  ['source', c => { c.source_sha = 'b'.repeat(40); }],
  ['signer', c => { c.identity = 'https://evil.invalid'; }],
  ['issuer', c => { c.issuer = 'https://evil.invalid'; }],
  ['ref', c => { c.ref = 'refs/heads/unreviewed'; }],
  ['platforms', c => { c.platforms = 'linux/amd64'; }],
]) test(`prepared rejects fake context ${label}`, async () => { const { context, evidence } = await preparedFixture(); change(context); assert.throws(() => assertPreparedEvidence(evidence, context)); });

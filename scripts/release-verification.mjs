// This file is part of midnightntwrk/midnight-did-resolver.
// SPDX-License-Identifier: Apache-2.0
import { readFileSync } from 'node:fs';
import { isDeepStrictEqual } from 'node:util';
import { repositories, OCI_INDEX, OCI_MANIFEST, sha256, isDigest, isVersion, parseJSON } from './release-registry.mjs';

const CONFIG = 'application/vnd.oci.image.config.v1+json';
const INTOTO = 'application/vnd.in-toto+json';
const SPDX = 'https://spdx.dev/Document';
const SLSA = 'https://slsa.dev/provenance/v0.2';
const GIT = 'https://github.com/midnightntwrk/midnight-did-resolver.git';
const GH = 'midnightntwrk/midnight-did-resolver';
const PLATFORMS = ['linux/amd64', 'linux/arm64'];
const BUILDER = Object.freeze({
  reference: 'docker.io/library/node:24.22.3-bookworm-slim@sha256:dc26ae92c9fb15cf6b1d63e20f0944b2efe0f115b20e00be48fb23fceb061c54',
  indexDigest: 'sha256:dc26ae92c9fb15cf6b1d63e20f0944b2efe0f115b20e00be48fb23fceb061c54', nodeVersion: '24.22.3',
  platforms: {
    'linux/amd64': { manifestDigest: 'sha256:0e2646ab57e77b7a384a0cb59b6815b182c77e1f9c3ba36bd8435bd9c4e6680e' },
    'linux/arm64': { manifestDigest: 'sha256:5f8fabfed57b75127de2f584593d67a9e8c90e6b0f65652871f9ef0f759c0299' },
  },
});
const RUNTIME = Object.freeze({
  reference: 'gcr.io/distroless/nodejs24-debian13:nonroot@sha256:6abfcb367ebc291572c774c1592a986be2ab26a2486f73093401327495a5962a',
  indexDigest: 'sha256:6abfcb367ebc291572c774c1592a986be2ab26a2486f73093401327495a5962a', nodeVersion: '24.22.3',
  platforms: {
    'linux/amd64': { manifestDigest: 'sha256:d4b8c34560df58c821ebda89a2897d7d183220c0debaf474a2b67a6c7cc22064' },
    'linux/arm64': { manifestDigest: 'sha256:6ff17c676ac284211c900bc2f512dd0b4e90ec6188657c91a4d6440c1f100b90' },
  },
});
const policyBytes = readFileSync(new URL('./release-advisory-policy.json', import.meta.url));
const approvedAdvisoryPolicy = parseJSON(policyBytes);
export const advisoryPolicyDigest = sha256(policyBytes);
const escapeRegExp = value => value.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
const nonempty = value => typeof value === 'string' && value.trim().length > 0;
const sourceSHAValid = value => typeof value === 'string' && /^[0-9a-f]{40}$/.test(value) && value.length === 40;
function requireThat(ok, code) {
  if (!ok) { const e = new Error(`Release evidence failed: ${code}`); e.name = 'ReleaseVerificationError'; e.code = code; throw e; }
}
function exactPlatforms(platforms) {
  requireThat(Array.isArray(platforms) && platforms.length === 2 && new Set(platforms).size === 2 && PLATFORMS.every(p => platforms.includes(p)), 'PLATFORMS');
}
function descriptor(d) {
  requireThat(d && isDigest(d.digest) && Number.isSafeInteger(d.size) && d.size >= 0 && nonempty(d.mediaType) && d.urls === undefined && d.data === undefined, 'DESCRIPTOR');
}
function checkedBytes(value, d) {
  descriptor(d);
  requireThat(value && Buffer.isBuffer(value.bytes) && value.bytes.length === d.size && sha256(value.bytes) === d.digest, 'DESCRIPTOR_INTEGRITY');
  if (value.digest !== undefined) requireThat(value.digest === d.digest, 'FETCH_DIGEST');
  if (value.size !== undefined) requireThat(value.size === d.size, 'FETCH_SIZE');
  if (value.mediaType !== undefined) requireThat(value.mediaType === d.mediaType, 'FETCH_MEDIA_TYPE');
  return value.bytes;
}
function imageManifest(json) {
  requireThat(json?.schemaVersion === 2 && json.mediaType === OCI_MANIFEST && Array.isArray(json.layers) && json.layers.length <= 256, 'IMAGE_MANIFEST');
  descriptor(json.config);
  requireThat(json.config.mediaType === CONFIG, 'CONFIG_MEDIA_TYPE');
  for (const layer of json.layers) descriptor(layer);
}
function basePolicyCheck(policy) {
  requireThat(isDeepStrictEqual(policy, { schemaVersion: 1, ...RUNTIME, builder: BUILDER }), 'BASE_POLICY_ROLES');
}
function baseRoles(platform) {
  return [
    { name: 'builder', policy: BUILDER, purls: ['pkg:docker/node@24.22.3-bookworm-slim', 'pkg:docker/library/node@24.22.3-bookworm-slim', 'pkg:docker/docker.io/library/node@24.22.3-bookworm-slim'] },
    { name: 'runtime', policy: RUNTIME, purls: ['pkg:docker/gcr.io/distroless/nodejs24-debian13@nonroot'] },
  ].map(role => ({ ...role, digests: [role.policy.indexDigest, role.policy.platforms[platform].manifestDigest] }));
}
function spdxInventory(p, service, baseVersion) {
  requireThat(p && ['SPDX-2.2', 'SPDX-2.3'].includes(p.spdxVersion) && p.SPDXID === 'SPDXRef-DOCUMENT' && p.dataLicense === 'CC0-1.0' && nonempty(p.name) && nonempty(p.documentNamespace) && /^https?:\/\//.test(p.documentNamespace), 'SPDX_DOCUMENT');
  requireThat(Array.isArray(p.creationInfo?.creators) && p.creationInfo.creators.length > 0 && p.creationInfo.creators.every(nonempty) && Number.isFinite(Date.parse(p.creationInfo.created)), 'SPDX_CREATION');
  requireThat(Array.isArray(p.packages) && p.packages.length >= 3 && p.packages.every(x => nonempty(x.SPDXID) && nonempty(x.name) && nonempty(x.versionInfo)), 'SPDX_INVENTORY');
  const ids = new Set(p.packages.map(x => x.SPDXID));
  requireThat(ids.size === p.packages.length && Array.isArray(p.relationships) && p.relationships.some(r => r.spdxElementId === p.SPDXID && r.relationshipType === 'DESCRIBES' && ids.has(r.relatedSpdxElement)), 'SPDX_RELATIONSHIPS');
  const has = (names, version) => p.packages.some(x => names.includes(x.name) && x.versionInfo === version);
  const app = `midnight-did-${service}-service`;
  requireThat(has([app, `@midnight-ntwrk/${app}`], baseVersion) && has(['node', 'nodejs'], '24.22.3') && has(['midnight-did', '@midnight-ntwrk/midnight-did'], '1.0.0-rc1'), 'SPDX_APPLICATION_RUNTIME');
  return p.packages.length;
}
function provenance(p, { sourceSHA, dockerfile, platform }) {
  requireThat(p?.buildType === 'https://mobyproject.org/buildkit@v1', 'PROVENANCE_BUILD_TYPE');
  const source = p.invocation?.configSource;
  const uri = `${GIT}#${sourceSHA}`;
  requireThat(source?.uri === uri && source.digest?.sha1 === sourceSHA && source.entryPoint === dockerfile, 'PROVENANCE_CONFIG_SOURCE');
  requireThat(Array.isArray(p.materials) && p.materials.some(m => m.uri === uri && m.digest?.sha1 === sourceSHA), 'PROVENANCE_GIT_MATERIAL');
  requireThat(p.metadata?.completeness?.parameters === true && p.metadata.completeness.materials === true && p.metadata.completeness.environment === true && Array.isArray(p.buildConfig?.llbDefinition) && p.buildConfig.llbDefinition.length > 0, 'PROVENANCE_MAX_MODE');
  requireThat(p.invocation.parameters?.frontend === 'dockerfile.v0', 'PROVENANCE_FRONTEND');
  const roles = baseRoles(platform);
  const materialRoles = new Set();
  for (const m of p.materials) {
    if (m.uri === uri) { requireThat(m.digest?.sha1 === sourceSHA, 'PROVENANCE_GIT_MATERIAL'); continue; }
    requireThat(typeof m.uri === 'string', 'PROVENANCE_MATERIAL_URI');
    const [name, query, extra] = m.uri.split('?');
    const params = new URLSearchParams(query);
    const role = roles.find(r => r.purls.includes(name));
    requireThat(role && extra === undefined && [...params.keys()].length === 1 && params.get('platform') === platform && role.digests.some(d => d.slice(7) === m.digest?.sha256), 'PROVENANCE_BASE_MATERIAL');
    materialRoles.add(role.name);
  }
  requireThat(materialRoles.size === 2, 'PROVENANCE_BASE_ROLE_COVERAGE');
  const sources = p.buildConfig.llbDefinition.map(s => s.op?.Op?.source?.identifier).filter(s => s?.startsWith('docker-image://'));
  const sourceRoles = new Set();
  for (const source of sources) {
    const role = roles.find(r => r.digests.some(d => source === `docker-image://${r.policy.reference.split('@')[0]}@${d}`));
    requireThat(role, 'PROVENANCE_BASE_PIN');
    sourceRoles.add(role.name);
  }
  requireThat(sourceRoles.size === 2, 'PROVENANCE_LLB_ROLE_COVERAGE');
  const infos = p.metadata['https://mobyproject.org/buildkit@v1#metadata']?.source?.infos;
  requireThat(Array.isArray(infos), 'PROVENANCE_DOCKERFILE_SOURCE');
  const info = infos.find(i => i.filename === dockerfile || (i.filename === 'Dockerfile' && dockerfile.endsWith('/Dockerfile')));
  requireThat(nonempty(info?.data) && /^[A-Za-z0-9+/]*={0,2}$/.test(info.data), 'PROVENANCE_DOCKERFILE_BYTES');
  const text = Buffer.from(info.data, 'base64').toString('utf8');
  const froms = text.split(/\r?\n/).filter(line => /^\s*FROM\s/i.test(line));
  const expectedFroms = [`FROM ${BUILDER.reference} AS builder`, `FROM ${RUNTIME.reference} AS runtime`];
  requireThat(!/^\s*ARG\s/im.test(text) && froms.length === 2 && froms.every((line, index) => {
    const normalized = line.trim().replace(/^FROM node:/, 'FROM docker.io/library/node:');
    return new RegExp(`^${escapeRegExp(expectedFroms[index])}$`).test(normalized);
  }), 'PROVENANCE_DOCKERFILE_BASE');
}

// This authenticates the metadata/attestation descriptor chain under an expected
// root, NOT a signature or filesystem layer payloads. Digest-pinned consumers
// verify runtime layers. Caller separately verifies Cosign identity/SHA/event.
export async function verifyImageGraph({ client, indexDigest, sourceSHA, dockerfile, service, baseVersion, basePolicy, platforms = PLATFORMS, onArtifact }) {
  requireThat(client && repositories.includes(client.repository) && ['resolver', 'manager'].includes(service) && client.repository === repositories[['resolver', 'manager'].indexOf(service)], 'GRAPH_REPOSITORY');
  requireThat(isDigest(indexDigest) && sourceSHAValid(sourceSHA) && baseVersion === '1.0.0' && dockerfile === `did-${service}-service/Dockerfile`, 'GRAPH_INPUT');
  exactPlatforms(platforms); basePolicyCheck(basePolicy);
  const emit = async (kind, d, bytes) => { if (onArtifact) await onArtifact({ kind, digest: d.digest, mediaType: d.mediaType, bytes: Buffer.from(bytes) }); };
  const rootValue = await client.getManifest(indexDigest);
  requireThat(Buffer.isBuffer(rootValue.bytes), 'ROOT_BYTES');
  const root = { digest: indexDigest, size: rootValue.bytes.length, mediaType: OCI_INDEX };
  const bytes = checkedBytes(rootValue, root);
  const index = parseJSON(bytes);
  requireThat(index.schemaVersion === 2 && index.mediaType === OCI_INDEX && Array.isArray(index.manifests) && index.manifests.length === 4, 'INDEX_SCHEMA');
  await emit('index', root, bytes);
  const runnable = new Map(); const attestations = [];
  const seen = new Set();
  for (const d of index.manifests) {
    descriptor(d); requireThat(d.mediaType === OCI_MANIFEST && !seen.has(d.digest), 'INDEX_DESCRIPTOR'); seen.add(d.digest);
    const p = `${d.platform?.os}/${d.platform?.architecture}`;
    if (p === 'unknown/unknown') {
      requireThat(d.annotations?.['vnd.docker.reference.type'] === 'attestation-manifest' && isDigest(d.annotations?.['vnd.docker.reference.digest']), 'ATTESTATION_ANNOTATIONS');
      attestations.push(d);
    } else {
      requireThat(PLATFORMS.includes(p) && !runnable.has(p) && d.annotations?.['vnd.docker.reference.type'] === undefined && (!d.platform.variant || (p === 'linux/arm64' && d.platform.variant === 'v8')), 'RUNTIME_PLATFORM');
      runnable.set(p, d);
    }
  }
  requireThat(runnable.size === 2 && attestations.length === 2, 'PLATFORM_COVERAGE');
  const receipts = [];
  let remainingBytes = 2 * 1024 ** 3; let artifacts = 0;
  async function load(d, kind, isManifest = false) {
    descriptor(d);
    remainingBytes -= d.size; artifacts++;
    requireThat(remainingBytes >= 0 && artifacts <= 512, 'GRAPH_BUDGET');
    const value = await (isManifest ? client.getManifest(d.digest) : client.getBlob(d.digest));
    const raw = checkedBytes(value, d);
    await emit(kind, d, raw);
    return raw;
  }
  for (const platform of PLATFORMS) {
    const d = runnable.get(platform);
    requireThat(d, 'MISSING_PLATFORM');
    const m = parseJSON(await load(d, 'manifest', true)); imageManifest(m);
    requireThat(!m.subject && m.layers.length > 0, 'RUNTIME_MANIFEST');
    const config = parseJSON(await load(m.config, 'config'));
    requireThat(config.os === 'linux' && config.architecture === platform.split('/')[1] && config.rootfs?.type === 'layers' && Array.isArray(config.rootfs.diff_ids) && config.rootfs.diff_ids.length === m.layers.length && config.rootfs.diff_ids.every(isDigest), 'CONFIG_PLATFORM');
    for (const layer of m.layers) {
      requireThat(['application/vnd.oci.image.layer.v1.tar', 'application/vnd.oci.image.layer.v1.tar+gzip', 'application/vnd.oci.image.layer.v1.tar+zstd'].includes(layer.mediaType), 'LAYER_MEDIA_TYPE');
      // Filesystem payloads are verified by the digest-pinned Docker/Trivy
      // consumer, not downloaded again by this metadata/attestation graph.
      requireThat(layer.size > 0, 'RUNTIME_LAYER_SIZE');
    }
    const linked = attestations.filter(a => a.annotations['vnd.docker.reference.digest'] === d.digest);
    requireThat(linked.length === 1, 'ATTESTATION_BINDING');
    const a = linked[0];
    const am = parseJSON(await load(a, 'attestation-manifest', true)); imageManifest(am);
    if (am.subject) { descriptor(am.subject); requireThat(am.subject.digest === d.digest && am.subject.size === d.size && am.subject.mediaType === d.mediaType, 'ATTESTATION_SUBJECT'); }
    const ac = parseJSON(await load(am.config, 'attestation-config'));
    requireThat(ac.architecture === 'unknown' && ac.os === 'unknown', 'ATTESTATION_CONFIG');
    requireThat(am.layers.length === 2, 'ATTESTATION_LAYERS');
    const receipt = { manifestDigest: a.digest, sourceSHA }; const types = new Set();
    for (const layer of am.layers) {
      requireThat(layer.mediaType === INTOTO, 'ATTESTATION_MEDIA_TYPE');
      const statement = parseJSON(await load(layer, 'attestation'));
      requireThat(statement._type === 'https://in-toto.io/Statement/v0.1' && [SPDX, SLSA].includes(statement.predicateType) && !types.has(statement.predicateType), 'ATTESTATION_SCHEMA');
      types.add(statement.predicateType);
      requireThat(layer.annotations?.['in-toto.io/predicate-type'] === statement.predicateType, 'PREDICATE_ANNOTATION');
      requireThat(Array.isArray(statement.subject) && statement.subject.length === 1 && statement.subject[0]?.digest?.sha256 === d.digest.slice(7) && nonempty(statement.subject[0]?.name), 'STATEMENT_SUBJECT');
      if (statement.predicateType === SPDX) {
        receipt.spdxPackageCount = spdxInventory(statement.predicate, service, baseVersion); receipt.sbomDigest = layer.digest;
      } else {
        provenance(statement.predicate, { sourceSHA, dockerfile, platform }); receipt.provenanceDigest = layer.digest;
      }
    }
    requireThat(types.size === 2, 'ATTESTATION_COVERAGE');
    receipts.push({ platform, manifestDigest: d.digest, configDigest: m.config.digest,
      runtimeLayers: m.layers.map(({ digest, size, mediaType }) => ({ digest, size, mediaType })),
      attestations: [receipt] });
  }
  return { schemaVersion: 1, repository: client.repository, indexDigest, sourceSHA, dockerfile, service, baseVersion, verified: true, root, layerPayloadVerification: 'consumer-required', platforms: receipts };
}

function graphPairs(graphs) {
  requireThat(Array.isArray(graphs) && graphs.length === 2 && new Set(graphs.map(g => g.repository)).size === 2 && repositories.every(r => graphs.some(g => g.repository === r)), 'GRAPH_PAIR');
  const pairs = new Map();
  for (const g of graphs) {
    requireThat(g.schemaVersion === 1 && g.verified === true && isDigest(g.indexDigest) && g.root?.digest === g.indexDigest && g.root.mediaType === OCI_INDEX && Number.isSafeInteger(g.root.size) && g.root.size > 0, 'GRAPH_RECEIPT');
    requireThat(g.layerPayloadVerification === 'consumer-required', 'GRAPH_LAYER_VERIFICATION_SCOPE');
    exactPlatforms(g.platforms?.map(p => p.platform));
    for (const p of g.platforms) {
      requireThat(Array.isArray(p.runtimeLayers) && p.runtimeLayers.length > 0 && p.runtimeLayers.length <= 256, 'GRAPH_RUNTIME_LAYERS');
      for (const layer of p.runtimeLayers) {
        descriptor(layer);
        requireThat(layer.size > 0 && ['application/vnd.oci.image.layer.v1.tar', 'application/vnd.oci.image.layer.v1.tar+gzip', 'application/vnd.oci.image.layer.v1.tar+zstd'].includes(layer.mediaType), 'GRAPH_RUNTIME_LAYER_DESCRIPTOR');
      }
      requireThat(isDigest(p.manifestDigest) && isDigest(p.configDigest) && Array.isArray(p.attestations) && p.attestations.length === 1 && p.attestations.every(a => isDigest(a.manifestDigest) && isDigest(a.sbomDigest) && isDigest(a.provenanceDigest) && Number.isSafeInteger(a.spdxPackageCount) && a.spdxPackageCount >= 3 && a.sourceSHA === g.sourceSHA), 'GRAPH_PLATFORM_RECEIPT');
      pairs.set(`${g.repository}|${p.platform}`, { ...p, repository: g.repository });
    }
  }
  requireThat(new Set([...pairs.values()].map(p => p.manifestDigest)).size === 4, 'DISTINCT_IMAGE_CHILDREN');
  return pairs;
}
function time(value) {
  if (typeof value !== 'string' || !/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d{1,9})?Z$/.test(value)) return NaN;
  const ms = Date.parse(value);
  return Number.isFinite(ms) && new Date(ms).toISOString().slice(0, 19) === value.slice(0, 19) ? ms : NaN;
}

function severityPolicy(policy) {
  // Injection is testable, not a policy override API: only byte-equivalent facts
  // from the tracked, source-reviewed policy are accepted. Never read env/payload maps.
  requireThat(isDeepStrictEqual(policy, approvedAdvisoryPolicy) && policy?.schemaVersion === 1 && policy.policyID === 'glibc-cna-severity-2026-11-17', 'ADVISORY_POLICY');
  requireThat(Array.isArray(policy.entries) && policy.entries.length === 2 && new Set(policy.entries.map(e => e.vulnerabilityID)).size === 2 &&
    ['CVE-2026-100290', 'CVE-2026-115819'].every(id => policy.entries.some(e => e.vulnerabilityID === id)), 'ADVISORY_POLICY_ENTRIES');
  const observed = time(policy.observedAt), until = time(policy.reviewValidUntil);
  requireThat(Number.isFinite(observed) && Number.isFinite(until) && until > observed && until - observed <= 7 * 86400000, 'ADVISORY_POLICY_WINDOW');
  for (const e of policy.entries) requireThat(
    e.packageName === 'libc6' && e.installedVersion === '2.41-12+deb13u4' && e.os?.family === 'debian' && e.os.version === '13.7' &&
    e.cna?.orgID === '3ff69d7a-14f2-4f67-a097-88dee7810d18' && e.cna.name === 'glibc' &&
    e.source === `https://cveawg.mitre.org/api/cve/${e.vulnerabilityID}` && /^[0-9a-f]{64}$/.test(e.rawSHA256) &&
    Number.isFinite(time(e.recordUpdatedAt)) && time(e.recordUpdatedAt) <= observed &&
    ['LOW', 'MEDIUM'].includes(e.effectiveSeverity) && Number.isFinite(e.cvssScore) && e.cvssScore > 0 && e.cvssScore < 7 &&
    e.cvssVector?.startsWith('CVSS:3.1/') && e.reachability === 'UNVERIFIED' && e.affectedStatus === 'affected-unfixed', 'ADVISORY_POLICY_FACTS');
  return { observed, until };
}

// Trivy Report SchemaVersion=2; upstream pkg/types/report.go v0.67.2.
// Require --list-all-pkgs and unfiltered inventories. Missing Results is not zero.
export function validateScanReports(reports, { graphs, nowMs = Date.now(), maxDBAgeHours = 24, advisoryPolicy = approvedAdvisoryPolicy }) {
  const pairs = graphPairs(graphs);
  requireThat(Array.isArray(reports) && reports.length === 4 && Number.isFinite(nowMs) && Number.isFinite(maxDBAgeHours) && maxDBAgeHours > 0 && maxDBAgeHours <= 24, 'SCAN_COVERAGE');
  const policyWindow = severityPolicy(advisoryPolicy);
  const counts = () => ({ UNKNOWN: 0, LOW: 0, MEDIUM: 0, HIGH: 0, CRITICAL: 0 });
  const rawSeverityCounts = counts(), effectiveSeverityCounts = counts();
  const severityResolutionReceipts = [];
  const seen = new Set();
  for (const r of reports) {
    const key = `${r.repository}|${r.platform}`; const p = pairs.get(key);
    requireThat(p && !seen.has(key) && r.manifestDigest === p.manifestDigest, 'SCAN_IMAGE_BINDING'); seen.add(key);
    requireThat(r.exitCode === 0 && typeof r.scannerVersion === 'string' && /^v?\d+\.\d+\.\d+(?:[-+][A-Za-z0-9.-]+)?$/.test(r.scannerVersion) && !/\s/.test(r.scannerVersion), 'SCANNER_EXECUTION');
    const db = time(r.dbUpdatedAt); const completed = time(r.completedAt);
    requireThat(Number.isFinite(db) && Number.isFinite(completed) && completed <= nowMs && db <= completed && nowMs - db <= maxDBAgeHours * 3600000, 'SCAN_DATABASE_TIME');
    const report = r.report;
    requireThat(report?.SchemaVersion === 2 && report.ArtifactType === 'container_image' && Array.isArray(report.Results) && report.Results.length > 0, 'SCAN_REPORT_SCHEMA');
    if (report.ArtifactName !== undefined) requireThat(report.ArtifactName === `${r.repository}@${p.manifestDigest}`, 'SCAN_ARTIFACT_NAME');
    if (report.Metadata?.ImageID !== undefined) requireThat(report.Metadata.ImageID === p.configDigest, 'SCAN_IMAGE_ID');
    // Architecture is an observed scanner fact, not the wrapper's requested platform.
    requireThat(report.Metadata?.ImageConfig?.architecture === r.platform.split('/')[1] && report.Metadata.ImageConfig.os === 'linux', 'SCAN_ARCHITECTURE');
    if (report.Metadata.RepoDigests !== undefined) requireThat(Array.isArray(report.Metadata.RepoDigests) && report.Metadata.RepoDigests.includes(`${r.repository}@${p.manifestDigest}`), 'SCAN_REPO_DIGEST');
    requireThat(!report.Warnings?.length && !r.warnings?.length, 'SCANNER_WARNINGS');
    const classes = new Set();
    for (const [resultIndex, result] of report.Results.entries()) {
      requireThat(nonempty(result.Target) && ['os-pkgs', 'lang-pkgs'].includes(result.Class) && nonempty(result.Type) && Array.isArray(result.Packages) && result.Packages.length > 0 && result.Packages.every(p => nonempty(p.Name) && nonempty(p.Version)), 'SCAN_INVENTORY');
      classes.add(result.Class);
      requireThat(result.ExperimentalModifiedFindings === undefined || (Array.isArray(result.ExperimentalModifiedFindings) && result.ExperimentalModifiedFindings.length === 0), 'SCAN_SUPPRESSED_FINDINGS');
      requireThat(result.Vulnerabilities === undefined || Array.isArray(result.Vulnerabilities), 'SCAN_VULNERABILITIES');
      for (const [findingIndex, v] of (result.Vulnerabilities ?? []).entries()) {
        requireThat(nonempty(v.VulnerabilityID) && nonempty(v.PkgName) && nonempty(v.InstalledVersion) && ['UNKNOWN', 'LOW', 'MEDIUM', 'HIGH', 'CRITICAL'].includes(v.Severity), 'SCAN_FINDING_SCHEMA');
        rawSeverityCounts[v.Severity]++;
        // A vendor classification never downgrades an actual HIGH/CRITICAL scan.
        requireThat(!['HIGH', 'CRITICAL'].includes(v.Severity), 'SCAN_BLOCKING_FINDING');
        let effective = v.Severity;
        if (v.Severity === 'UNKNOWN') {
          requireThat(nowMs >= policyWindow.observed && nowMs < policyWindow.until, 'ADVISORY_POLICY_EXPIRED_OR_NOT_YET_VALID');
          const e = advisoryPolicy.entries.find(e => e.vulnerabilityID === v.VulnerabilityID && e.packageName === v.PkgName && e.installedVersion === v.InstalledVersion &&
            report.Metadata.OS?.Family === e.os.family && report.Metadata.OS?.Name === e.os.version);
          requireThat(e && result.Class === 'os-pkgs' && result.Type === 'debian' && result.Packages.some(p => p.Name === e.packageName && p.Version === e.installedVersion), 'SCAN_UNRESOLVED_UNKNOWN');
          effective = e.effectiveSeverity;
          severityResolutionReceipts.push({
            repository: r.repository, platform: r.platform, manifestDigest: r.manifestDigest, resultIndex, findingIndex,
            vulnerabilityID: v.VulnerabilityID, packageName: v.PkgName, installedVersion: v.InstalledVersion,
            originalSeverity: 'UNKNOWN', effectiveSeverity: effective, policyID: advisoryPolicy.policyID, policyDigest: advisoryPolicyDigest,
            cna: { ...e.cna }, source: e.source, rawSHA256: e.rawSHA256, recordUpdatedAt: e.recordUpdatedAt,
            cvssVector: e.cvssVector, cvssScore: e.cvssScore, observedAt: advisoryPolicy.observedAt, reviewValidUntil: advisoryPolicy.reviewValidUntil,
            os: { ...e.os }, reachability: e.reachability, affectedStatus: e.affectedStatus,
          });
        }
        effectiveSeverityCounts[effective]++;
      }
    }
    requireThat(classes.has('os-pkgs') && classes.has('lang-pkgs'), 'SCAN_ANALYZER_COVERAGE');
  }
  return { verified: true, platformCount: 4, high: 0, critical: 0,
    rawUnknownCount: rawSeverityCounts.UNKNOWN, rawSeverityCounts, effectiveSeverityCounts, severityResolutionReceipts,
    advisoryPolicy: { policyID: advisoryPolicy.policyID, digest: advisoryPolicyDigest, observedAt: advisoryPolicy.observedAt, reviewValidUntil: advisoryPolicy.reviewValidUntil } };
}

// This is structural cross-job revalidation, not cryptographic receipt verification.
// Artifact hashes/run identity and actual Cosign/runtime execution belong to caller.
export function assertPreparedEvidence(evidence, context, { nowMs = Date.now() } = {}) {
  const c = context;
  requireThat(c && (c.repository === undefined || c.repository === GH) && c.resolver_repository === repositories[0] && c.manager_repository === repositories[1] && sourceSHAValid(c.source_sha) && isVersion(c.version) && c.base === '1.0.0' && c.version.split('-')[0] === c.base, 'CONTEXT');
  requireThat((c.channel === 'rc' && /^1\.0\.0-rc\.[1-9][0-9]*$/.test(c.version) && ['refs/heads/develop', 'refs/heads/main'].includes(c.ref)) || (c.channel === 'release' && c.version === '1.0.0' && c.ref === 'refs/heads/main'), 'CONTEXT_CHANNEL');
  exactPlatforms(typeof c.platforms === 'string' ? c.platforms.split(',') : c.platforms);
  const identity = `https://github.com/${GH}/.github/workflows/release-docker.yml@${c.ref}`;
  const issuer = 'https://token.actions.githubusercontent.com';
  requireThat(c.identity === identity && c.issuer === issuer, 'CONTEXT_SIGNER');
  const e = evidence;
  requireThat(e?.schemaVersion === 1 && e.verified === true && e.repository === GH && e.sourceSHA === c.source_sha && e.sourceRef === c.ref && e.channel === c.channel && e.version === c.version && e.base === c.base, 'EVIDENCE_CONTEXT');
  const positiveID = v => (typeof v === 'string' && /^[1-9][0-9]*$/.test(v)) || (Number.isSafeInteger(v) && v > 0);
  requireThat(positiveID(e.runID) && positiveID(e.runAttempt), 'EVIDENCE_RUN');
  if (c.runID !== undefined) requireThat(String(e.runID) === String(c.runID), 'RUN_IDENTITY');
  if (c.runAttempt !== undefined) requireThat(String(e.runAttempt) === String(c.runAttempt), 'RUN_ATTEMPT');
  const pairs = graphPairs(e.graphs);
  for (const g of e.graphs) {
    const service = g.repository === repositories[0] ? 'resolver' : 'manager';
    requireThat(g.sourceSHA === c.source_sha && g.service === service && g.dockerfile === `did-${service}-service/Dockerfile` && g.baseVersion === c.base, 'EVIDENCE_GRAPH_CONTEXT');
  }
  validateScanReports(e.scanReceipts, { graphs: e.graphs, nowMs });
  requireThat(Array.isArray(e.runtimeReceipts) && e.runtimeReceipts.length === 4, 'RUNTIME_COVERAGE');
  const seen = new Set();
  for (const r of e.runtimeReceipts) {
    const key = `${r.repository}|${r.platform}`; const p = pairs.get(key);
    requireThat(p && !seen.has(key) && r.manifestDigest === p.manifestDigest, 'RUNTIME_IMAGE'); seen.add(key);
    requireThat(['verified', 'health', 'ready', 'restart', 'recreate', 'persistence'].every(k => r[k] === true), 'RUNTIME_GATES');
    // Structural execution receipt only: caller sets this after a successful
    // docker --platform pull of this exact platform child digest.
    requireThat(r.digestPullVerified === true && r.pulledReference === `${r.repository}@${p.manifestDigest}`, 'RUNTIME_DIGEST_PULL');
    requireThat(r.architectureMode === (r.platform === 'linux/amd64' ? 'native-amd64' : 'qemu-arm64') && r.processArch === (r.platform === 'linux/amd64' ? 'x64' : 'arm64'), 'RUNTIME_ARCHITECTURE');
    // A resolver has no filesystem state to persist; receipt must say what was tested.
    if (r.repository === repositories[0]) requireThat(r.persistenceMode === 'stateless-external-state', 'RESOLVER_STATE_MODEL');
  }
  requireThat(Array.isArray(e.signatureReceipts) && e.signatureReceipts.length === 2, 'SIGNATURE_COVERAGE');
  const signatures = new Set();
  for (const s of e.signatureReceipts) {
    const g = e.graphs.find(g => g.repository === s.repository);
    requireThat(g && !signatures.has(s.repository) && s.indexDigest === g.indexDigest && s.verified === true && s.identity === identity && s.issuer === issuer && s.sourceSHA === c.source_sha && s.event === 'workflow_dispatch', 'SIGNATURE_BINDING'); signatures.add(s.repository);
  }
  return e;
}

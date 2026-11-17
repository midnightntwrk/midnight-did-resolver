// This file is part of midnightntwrk/midnight-did-resolver.
// SPDX-License-Identifier: Apache-2.0
import { createHash } from 'node:crypto';

export const repositories = Object.freeze(['resolver', 'manager'].map(s => `ghcr.io/midnightntwrk/midnight-did-${s}-service`));
export const OCI_INDEX = 'application/vnd.oci.image.index.v1+json';
export const OCI_MANIFEST = 'application/vnd.oci.image.manifest.v1+json';
const manifestTypes = [OCI_INDEX, OCI_MANIFEST, 'application/vnd.docker.distribution.manifest.list.v2+json', 'application/vnd.docker.distribution.manifest.v2+json'];
export const sha256 = bytes => `sha256:${createHash('sha256').update(bytes).digest('hex')}`;
export const isDigest = value => typeof value === 'string' && /^sha256:[0-9a-f]{64}$/.test(value) && value.length === 71;
export const isVersion = value => typeof value === 'string' && value.length <= 128 && /^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)(-rc\.[1-9][0-9]*)?$/.test(value) && !/\s/.test(value);
const safeErrors = new WeakSet();
function failure(code, status) {
  const error = new Error(`Registry operation failed: ${code}${Number.isInteger(status) ? ` (${status})` : ''}`);
  error.name = 'RegistryError'; error.code = code;
  if (Number.isInteger(status)) error.status = status;
  safeErrors.add(error);
  return error;
}
export function parseJSON(bytes) {
  try { return JSON.parse(new TextDecoder('utf-8', { fatal: true }).decode(bytes)); }
  catch { throw failure('INVALID_JSON'); }
}

// Credentials are closed over, never attached to the client, errors or receipts.
export function createRegistryClient({ repository, actor, token, fetchImpl = fetch, timeoutMs = 30000, maxBytes = 67108864 }) {
  if (!repositories.includes(repository)) throw failure('REPOSITORY_NOT_ALLOWED');
  if (typeof actor !== 'string' || !actor || /[\s:]/.test(actor) || typeof token !== 'string' || !token || /[\r\n]/.test(token)) throw failure('CREDENTIAL_INPUT');
  if (!Number.isSafeInteger(timeoutMs) || timeoutMs < 1 || timeoutMs > 120000 || !Number.isSafeInteger(maxBytes) || maxBytes < 1 || maxBytes > 268435456) throw failure('INVALID_BOUNDS');
  const name = repository.slice('ghcr.io/'.length);
  const endpoint = `https://ghcr.io/v2/${name}`;
  let bearer;

  async function request(url, { method = 'GET', headers = {}, body } = {}) {
    const controller = new AbortController();
    const timer = setTimeout(() => controller.abort(), timeoutMs);
    try {
      // Race includes body consumption: injected transports must not defeat the deadline.
      return await Promise.race([
        (async () => {
          const response = await fetchImpl(url, { method, headers, body, redirect: 'manual', signal: controller.signal });
          const length = response.headers.get('content-length');
          if (length !== null && (!/^\d+$/.test(length) || Number(length) > maxBytes)) throw failure('BODY_LIMIT');
          const chunks = []; let size = 0;
          if (response.body) {
            for await (const chunk of response.body) {
              size += chunk.byteLength;
              if (size > maxBytes) { const error = failure('BODY_LIMIT'); controller.abort(error); throw error; }
              chunks.push(Buffer.from(chunk));
            }
          }
          const bytes = Buffer.concat(chunks);
          if (length !== null && bytes.length !== Number(length)) throw failure('CONTENT_LENGTH_MISMATCH');
          return { status: response.status, headers: response.headers, bytes };
        })(),
        new Promise((_, reject) => controller.signal.addEventListener('abort', () => reject(safeErrors.has(controller.signal.reason) ? controller.signal.reason : failure('TIMEOUT')), { once: true })),
      ]);
    } catch (error) {
      // Never expose transport exception text, which may contain credentials/signed URLs.
      const safe = safeErrors.has(error) ? error : failure('TRANSPORT');
      controller.abort(safe);
      throw safe;
    } finally { clearTimeout(timer); }
  }
  async function authorize() {
    if (bearer) return bearer;
    const url = new URL('https://ghcr.io/token');
    url.searchParams.set('service', 'ghcr.io');
    url.searchParams.set('scope', `repository:${name}:pull,push`);
    const response = await request(url.href, { headers: { Authorization: `Basic ${Buffer.from(`${actor}:${token}`).toString('base64')}` } });
    if (response.status !== 200) throw failure('TOKEN_STATUS', response.status);
    const data = parseJSON(response.bytes);
    if (typeof data?.token !== 'string' || !data.token || /[\s]/.test(data.token)) throw failure('TOKEN_RESPONSE');
    bearer = data.token;
    return bearer;
  }
  async function registry(path, options = {}) {
    const auth = await authorize();
    return request(`${endpoint}/${path}`, { ...options, headers: { ...options.headers, Authorization: `Bearer ${auth}` } });
  }
  function content(response, expectedDigest) {
    const digest = sha256(response.bytes);
    const header = response.headers.get('docker-content-digest');
    if ((expectedDigest && expectedDigest !== digest) || (header !== null && (!isDigest(header) || header !== digest))) throw failure('DIGEST_MISMATCH');
    return { bytes: response.bytes, digest, size: response.bytes.length };
  }
  function checkReference(reference) {
    if (!isDigest(reference) && !isVersion(reference)) throw failure('INVALID_REFERENCE');
  }
  function manifest(response, reference) {
    if (response.status !== 200) throw failure('MANIFEST_STATUS', response.status);
    const mediaType = response.headers.get('content-type')?.split(';')[0].trim();
    if (!manifestTypes.includes(mediaType)) throw failure('MANIFEST_MEDIA_TYPE');
    const data = content(response, isDigest(reference) ? reference : undefined);
    const json = parseJSON(data.bytes);
    if (json?.mediaType !== mediaType || json.schemaVersion !== 2) throw failure('MANIFEST_SCHEMA');
    return { ...data, mediaType, json };
  }
  async function assertRepositoryAccess() {
    const access = await registry('tags/list?n=1');
    if (access.status !== 200) throw failure('REPOSITORY_ACCESS', access.status);
    const tags = parseJSON(access.bytes);
    if (tags?.name !== name || !(tags.tags === null || (Array.isArray(tags.tags) && tags.tags.every(t => typeof t === 'string')))) throw failure('REPOSITORY_ACCESS_SCHEMA');
  }
  function assertExpectedAbsence(response) {
    if (response.status !== 404) throw failure('ABSENCE_STATUS', response.status);
    const data = parseJSON(response.bytes);
    if (!Array.isArray(data?.errors) || data.errors.length !== 1 || data.errors[0]?.code !== 'MANIFEST_UNKNOWN') throw failure('ABSENCE_NOT_PROVEN', 404);
  }
  async function assertVersionAbsent(version) {
    if (!isVersion(version)) throw failure('INVALID_VERSION');
    await assertRepositoryAccess();
    const response = await registry(`manifests/${version}`, { headers: { Accept: manifestTypes.join(', ') } });
    if (response.status === 200) throw failure('VERSION_EXISTS', 200);
    assertExpectedAbsence(response);
    return { repository, version, absent: true };
  }
  async function getManifest(reference) {
    checkReference(reference);
    return manifest(await registry(`manifests/${reference}`, { headers: { Accept: manifestTypes.join(', ') } }), reference);
  }
  // Fixed latest reader is the only mutable-tag read interface. null means
  // authenticated repository access plus the expected MANIFEST_UNKNOWN response.
  async function getLatestManifest() {
    await assertRepositoryAccess();
    const response = await registry('manifests/latest', { headers: { Accept: manifestTypes.join(', ') } });
    if (response.status === 200) return manifest(response, 'latest');
    assertExpectedAbsence(response);
    return null;
  }
  async function getBlob(digest) {
    if (!isDigest(digest)) throw failure('INVALID_DIGEST');
    let response = await registry(`blobs/${digest}`);
    const seen = new Set();
    for (let redirects = 0; [301, 302, 303, 307, 308].includes(response.status); redirects++) {
      if (redirects >= 3) throw failure('BLOB_REDIRECT_LIMIT');
      let url;
      try { url = new URL(response.headers.get('location')); } catch { throw failure('BLOB_REDIRECT'); }
      // Intentionally narrow. Add a host only after reviewed GHCR transport evidence.
      if (url.protocol !== 'https:' || url.hostname !== 'pkg-containers.githubusercontent.com' || url.port || url.username || url.password || url.hash || seen.has(url.href)) throw failure('BLOB_REDIRECT_NOT_ALLOWED');
      seen.add(url.href);
      response = await request(url.href); // No auth, cookies, or caller headers across origins.
    }
    if (response.status !== 200) throw failure('BLOB_STATUS', response.status);
    return content(response, digest);
  }
  async function promoteIndex(version, { bytes, digest, mediaType }) {
    if (!Buffer.isBuffer(bytes) || bytes.length > maxBytes || !isDigest(digest) || sha256(bytes) !== digest || mediaType !== OCI_INDEX) throw failure('PROMOTION_INPUT');
    const json = parseJSON(bytes);
    if (json?.schemaVersion !== 2 || json.mediaType !== OCI_INDEX || !Array.isArray(json.manifests)) throw failure('PROMOTION_INDEX');
    // Snapshot avoids a caller mutating its Buffer during the absence check.
    const raw = Buffer.from(bytes);
    await assertVersionAbsent(version);
    try {
      // Exactly one attempt. Even a rejected status is conservatively ambiguous.
      const response = await registry(`manifests/${version}`, { method: 'PUT', headers: { 'Content-Type': mediaType }, body: raw });
      if (response.status !== 201) throw failure('PUT_STATUS', response.status);
      const header = response.headers.get('docker-content-digest');
      if (header !== null && header !== digest) throw failure('PUT_DIGEST');
      const readback = await getManifest(version);
      if (readback.digest !== digest || !readback.bytes.equals(raw)) throw failure('READBACK_DRIFT');
      return { repository, version, digest, confirmed: true };
    } catch (error) {
      const safe = safeErrors.has(error) ? error : failure('PROMOTION_FAILED');
      safe.publicationMayHaveOccurred = true;
      throw safe;
    }
  }
  // Caller MUST enforce release/main and complete prepared/version-pair gates.
  // This method cannot infer branch authority; it independently rejects every RC.
  async function promoteStableLatest(stableVersion, { bytes, digest, mediaType }) {
    if (!isVersion(stableVersion) || stableVersion.includes('-')) throw failure('STABLE_VERSION_REQUIRED');
    if (!Buffer.isBuffer(bytes) || bytes.length > maxBytes || !isDigest(digest) || sha256(bytes) !== digest || mediaType !== OCI_INDEX) throw failure('PROMOTION_INPUT');
    const json = parseJSON(bytes);
    if (json?.schemaVersion !== 2 || json.mediaType !== OCI_INDEX || !Array.isArray(json.manifests)) throw failure('PROMOTION_INDEX');
    const raw = Buffer.from(bytes);
    // Never create or overwrite the immutable stable version here.
    const versionManifest = await getManifest(stableVersion);
    if (versionManifest.digest !== digest || versionManifest.mediaType !== mediaType || !versionManifest.bytes.equals(raw)) throw failure('STABLE_VERSION_DRIFT');
    const previous = await getLatestManifest();
    const previousDigest = previous?.digest ?? null;
    try {
      const response = await registry('manifests/latest', { method: 'PUT', headers: { 'Content-Type': mediaType }, body: raw });
      if (response.status !== 201) throw failure('PUT_STATUS', response.status);
      const header = response.headers.get('docker-content-digest');
      if (header !== null && header !== digest) throw failure('PUT_DIGEST');
      const readback = await getLatestManifest();
      if (!readback || readback.digest !== digest || !readback.bytes.equals(raw)) throw failure('READBACK_DRIFT');
      return { repository, version: stableVersion, tag: 'latest', digest, previousDigest, confirmed: true };
    } catch (error) {
      const safe = safeErrors.has(error) ? error : failure('PROMOTION_FAILED');
      safe.publicationMayHaveOccurred = true;
      // Non-sensitive reconciliation facts; never retry an ambiguous write.
      safe.previousDigest = previousDigest;
      throw safe;
    }
  }
  return Object.freeze({ repository, assertVersionAbsent, getManifest, getLatestManifest, getBlob, promoteIndex, promoteStableLatest });
}

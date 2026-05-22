import { Buffer } from 'node:buffer';

import type { MidnightDIDDocument } from '@midnight-ntwrk/midnight-did';
import type {
  FileSecretStore,
  PublicJwk,
  StoredKeyMeta,
} from '@midnight-ntwrk/midnight-did-secret-storage';
import { verifyWithPublicJwk } from '@midnight-ntwrk/midnight-did-secret-storage';

import { ManagerInvalidRequestError, ManagerNotFoundError } from '../errors.js';
import type {
  SignatureFormat,
  SignPayloadRequest,
  SignPayloadResponse,
  VerificationSource,
  VerifyPayloadRequest,
  VerifyPayloadResponse,
} from '../types.js';
import { normalizePayload } from './payload-normalization.js';

type ResolvedVerificationMethod = {
  did: string;
  verificationMethodId: string;
  publicJwk: PublicJwk;
};

type ResolveVerificationMethod = (
  verificationMethodId: string,
) => Promise<ResolvedVerificationMethod>;

const toBase64Url = (bytes: Uint8Array): string =>
  Buffer.from(bytes).toString('base64url');

const base64UrlPattern = /^[A-Za-z0-9_-]+$/;

const fromBase64Url = (value: string): Uint8Array => {
  if (!base64UrlPattern.test(value) || value.length % 4 === 1) {
    throw new ManagerInvalidRequestError('Signature must be a valid base64url-encoded byte string.');
  }
  return new Uint8Array(Buffer.from(value, 'base64url'));
};

const samePublicJwk = (left: PublicJwk, right: PublicJwk): boolean =>
  left.kty === right.kty &&
  left.crv === right.crv &&
  left.x === right.x &&
  (left.y ?? null) === (right.y ?? null);

const signatureFormatFor = (publicJwk: Pick<PublicJwk, 'crv'>): SignatureFormat => {
  if (publicJwk.crv === 'Ed25519') return 'ed25519-raw';
  if (publicJwk.crv === 'Jubjub') return 'jubjub-raw-96';
  if (publicJwk.crv === 'P-256') return 'ecdsa-der';
  throw new ManagerInvalidRequestError(`Unsupported signature curve ${String(publicJwk.crv)}`);
};

const findStoredKey = async (
  secretStore: FileSecretStore,
  keyRef: string,
): Promise<StoredKeyMeta> => {
  const key = (await secretStore.listKeys()).find((entry) => entry.keyRef === keyRef);
  if (key === undefined) {
    throw new ManagerNotFoundError('secretNotFound', `Key not found in secret storage: ${keyRef}`);
  }
  return key;
};

const didFromVerificationMethodId = (verificationMethodId: string): string => {
  const fragmentIndex = verificationMethodId.indexOf('#');
  if (fragmentIndex <= 0) {
    throw new ManagerInvalidRequestError(
      'Verification method id must be an absolute Midnight DID URL with a fragment.',
    );
  }
  return verificationMethodId.slice(0, fragmentIndex);
};

const findVerificationMethodForPublicKey = (
  didDocument: MidnightDIDDocument,
  publicJwk: PublicJwk,
): string | null => {
  const method = didDocument.verificationMethod?.find((entry) =>
    samePublicJwk(entry.publicKeyJwk as PublicJwk, publicJwk),
  );
  return method?.id ?? null;
};

export const signPayload = async (input: {
  secretStore: FileSecretStore;
  didDocument: MidnightDIDDocument;
  request: SignPayloadRequest;
}): Promise<SignPayloadResponse> => {
  const { secretStore, didDocument, request } = input;
  const key = await findStoredKey(secretStore, request.keyRef);
  if (key.did !== undefined && key.did !== didDocument.id) {
    throw new ManagerInvalidRequestError(
      `Selected key is associated with ${key.did}, not the active DID ${didDocument.id}.`,
    );
  }

  const publicJwk = await secretStore.getPublicKey(request.keyRef);
  const verificationMethodId = findVerificationMethodForPublicKey(
    didDocument,
    publicJwk,
  );
  if (verificationMethodId === null) {
    throw new ManagerInvalidRequestError(
      'Selected key is not published in the active DID document as a verification method.',
    );
  }

  const normalized = normalizePayload(request.payloadType, request.payload);
  const { signature } = await secretStore.sign({
    keyRef: request.keyRef,
    payload: normalized.bytes,
  });

  return {
    did: didDocument.id,
    verificationMethodId,
    keyRef: request.keyRef,
    algorithm: {
      kty: publicJwk.kty,
      crv: publicJwk.crv,
    },
    payloadType: request.payloadType,
    canonicalText: normalized.canonicalText,
    canonicalHex: normalized.canonicalHex,
    canonicalPayloadBase64Url: toBase64Url(normalized.bytes),
    signatureBase64Url: toBase64Url(signature),
    signatureFormat: signatureFormatFor(publicJwk),
    publicJwk,
  };
};

export const verifyPayload = async (input: {
  secretStore?: FileSecretStore;
  request: VerifyPayloadRequest;
  resolveVerificationMethod?: ResolveVerificationMethod;
}): Promise<VerifyPayloadResponse> => {
  const { secretStore, request, resolveVerificationMethod } = input;
  const normalized = normalizePayload(request.payloadType, request.payload);
  const signatureBytes = fromBase64Url(request.signatureBase64Url);
  const sourceCount = [
    request.keyRef !== undefined,
    request.publicJwk !== undefined,
    request.verificationMethodId !== undefined,
  ].filter(Boolean).length;
  if (sourceCount !== 1) {
    throw new ManagerInvalidRequestError(
      'Verification requires exactly one source: keyRef, publicJwk, or verificationMethodId.',
    );
  }

  let source: VerificationSource;
  let did: string | null = null;
  let verificationMethodId: string | null = null;
  let publicJwk: PublicJwk;

  if (request.keyRef !== undefined) {
    if (secretStore === undefined) {
      throw new ManagerInvalidRequestError('Local key verification requires an active secret store session.');
    }
    publicJwk = await secretStore.getPublicKey(request.keyRef);
    source = 'localKey';
  } else if (request.publicJwk !== undefined) {
    publicJwk = request.publicJwk;
    source = 'publicJwk';
  } else if (request.verificationMethodId !== undefined) {
    if (resolveVerificationMethod === undefined) {
      throw new ManagerInvalidRequestError('DID verification requires a verification method resolver.');
    }
    const expectedDid = didFromVerificationMethodId(request.verificationMethodId);
    const resolved = await resolveVerificationMethod(request.verificationMethodId);
    if (resolved.did !== expectedDid) {
      throw new ManagerInvalidRequestError(
        `Verification method ${request.verificationMethodId} resolved to ${resolved.did}, expected ${expectedDid}.`,
      );
    }
    if (resolved.verificationMethodId !== request.verificationMethodId) {
      throw new ManagerInvalidRequestError(
        `Verification method resolver returned ${resolved.verificationMethodId}, expected ${request.verificationMethodId}.`,
      );
    }
    did = resolved.did;
    verificationMethodId = resolved.verificationMethodId;
    publicJwk = resolved.publicJwk;
    source = 'didDocument';
  } else {
    throw new ManagerInvalidRequestError(
      'Verification requires exactly one source: keyRef, publicJwk, or verificationMethodId.',
    );
  }

  const verified = await verifyWithPublicJwk(
    publicJwk,
    normalized.bytes,
    signatureBytes,
  );

  return {
    verified,
    source,
    did,
    verificationMethodId,
    algorithm: {
      kty: publicJwk.kty,
      crv: publicJwk.crv,
    },
    payloadType: request.payloadType,
    canonicalText: normalized.canonicalText,
    canonicalHex: normalized.canonicalHex,
    canonicalPayloadBase64Url: toBase64Url(normalized.bytes),
    signatureBase64Url: request.signatureBase64Url,
    signatureFormat: signatureFormatFor(publicJwk),
    publicJwk,
  };
};

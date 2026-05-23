import { randomUUID } from "node:crypto";
import { chmod, mkdir, open, readFile, rename, rm } from "node:fs/promises";
import path from "node:path";

import {
  decryptJsonWithKey,
  deriveKey,
  type EncryptedPayload,
  encryptJsonWithKey,
  generateEncryptionSalt,
} from "./crypto.js";
import {
  generateCurveKey,
  importCurveKey,
  normalizePublicForLedger,
  signWithCurveKey,
  type StoredPrivateRecord,
  verifyWithPublicJwk,
} from "./curve-support.js";
import {
  SecretNotFoundError,
  SecretStoreInitError,
  SecretStoreLockedError,
} from "./errors.js";
import { deriveCurvePrivateFromSeed } from "./hd-derivation.js";
import type {
  DeriveKeyFromSeedInput,
  GenerateKeyInput,
  ImportKeyInput,
  PublicJwk,
  SecretStorage,
  StoredKeyMeta,
  VerifyInput,
} from "./types.js";

type StoredEntry = {
  meta: StoredKeyMeta;
  privateRecord: StoredPrivateRecord;
  publicJwk: PublicJwk;
};

type StoreFile = {
  version: 1;
  keys: Record<string, StoredEntry>;
};

type FileEnvelope = {
  version: 1;
  encrypted: EncryptedPayload;
};

const STORE_FILE_MODE = 0o600;

const nowIso = (): string => new Date().toISOString();

const chmodPrivate = async (location: string): Promise<void> => {
  await chmod(location, STORE_FILE_MODE);
};

const writePrivateAtomic = async (
  location: string,
  contents: string,
): Promise<void> => {
  const tempLocation = `${location}.${process.pid}.${Date.now()}.${Math.random()
    .toString(16)
    .slice(2)}.tmp`;
  let handle: Awaited<ReturnType<typeof open>> | undefined;

  try {
    handle = await open(tempLocation, "wx", STORE_FILE_MODE);
    await handle.writeFile(contents, "utf8");
    await handle.sync();
    await handle.close();
    handle = undefined;
    await rename(tempLocation, location);
    await chmodPrivate(location);
  } catch (error) {
    if (handle) {
      await handle.close().catch(() => undefined);
    }
    await rm(tempLocation, { force: true }).catch(() => undefined);
    throw error;
  }
};

export class FileSecretStore implements SecretStorage {
  private location = "";
  private encryptionKey?: Buffer;
  private encryptionSalt?: Buffer;
  private store: StoreFile = { version: 1, keys: {} };

  async initialize(params: {
    location: string;
    passphrase?: string;
  }): Promise<void> {
    this.clearEncryptionMaterial();
    this.location = params.location;
    if (!params.passphrase) {
      throw new SecretStoreLockedError();
    }

    const dir = path.dirname(this.location);
    await mkdir(dir, { recursive: true });

    try {
      const raw = await readFile(this.location, "utf8");
      const envelope = JSON.parse(raw) as FileEnvelope;
      const salt = Buffer.from(envelope.encrypted.salt, "base64");
      await this.unlockWithPassphrase(params.passphrase, salt);
      salt.fill(0);
      const decrypted = decryptJsonWithKey(
        envelope.encrypted,
        this.requireEncryptionKey(),
      );
      this.store = JSON.parse(decrypted) as StoreFile;
      await chmodPrivate(this.location);
    } catch (error) {
      const maybeErr = error as { code?: string };
      if (maybeErr.code === "ENOENT") {
        this.store = { version: 1, keys: {} };
        const salt = generateEncryptionSalt();
        try {
          await this.unlockWithPassphrase(params.passphrase, salt);
        } finally {
          salt.fill(0);
        }
        await this.persist();
        return;
      }
      this.clearEncryptionMaterial();
      throw new SecretStoreInitError(
        `Failed to initialize file secret store: ${String(error)}`,
      );
    }
  }

  lock(): void {
    this.clearEncryptionMaterial();
    this.store = { version: 1, keys: {} };
  }

  async listKeys(filter?: { did?: string }): Promise<StoredKeyMeta[]> {
    const keys = Object.values(this.store.keys).map((entry) => entry.meta);
    if (!filter?.did) return keys;
    return keys.filter((entry) => entry.did === filter.did);
  }

  async generateKey(
    params: GenerateKeyInput,
  ): Promise<{ keyRef: string; publicJwk: PublicJwk }> {
    const generated = await generateCurveKey(params.kty, params.crv);
    const keyRef = randomUUID();
    const timestamp = nowIso();
    this.store.keys[keyRef] = {
      meta: {
        id: params.id,
        keyRef,
        did: params.did,
        purpose: params.purpose,
        createdAt: timestamp,
        updatedAt: timestamp,
        algorithm: {
          kty: params.kty,
          crv: params.crv,
        },
      },
      privateRecord: generated.record,
      publicJwk: generated.publicJwk,
    };
    await this.persist();
    return { keyRef, publicJwk: generated.publicJwk };
  }

  async importKey(
    params: ImportKeyInput,
  ): Promise<{ keyRef: string; publicJwk: PublicJwk }> {
    const imported = await importCurveKey(params);
    const keyRef = randomUUID();
    const timestamp = nowIso();
    this.store.keys[keyRef] = {
      meta: {
        id: params.id,
        keyRef,
        did: params.did,
        purpose: params.purpose,
        createdAt: timestamp,
        updatedAt: timestamp,
        algorithm: {
          kty: params.kty,
          crv: params.crv,
        },
      },
      privateRecord: imported.record,
      publicJwk: imported.publicJwk,
    };
    await this.persist();
    return { keyRef, publicJwk: imported.publicJwk };
  }

  async deriveKeyFromSeed(
    params: DeriveKeyFromSeedInput,
  ): Promise<{ keyRef: string; publicJwk: PublicJwk }> {
    for (let candidate = 0; candidate < 512; candidate += 1) {
      try {
        const derived = deriveCurvePrivateFromSeed(params, candidate);
        try {
          return await this.importKey({
            id: params.id,
            privateKey: derived.privateKey,
            kty: derived.kty,
            crv: derived.crv,
            did: params.did,
            purpose: params.purpose,
          });
        } finally {
          derived.privateKey.fill(0);
        }
      } catch (error) {
        if (
          error instanceof Error &&
          error.message.includes("not representable in Midnight Compact fields")
        ) {
          continue;
        }
        throw error;
      }
    }
    throw new Error("Failed to derive a ledger-compatible key from seed");
  }

  async getPublicKey(keyRef: string): Promise<PublicJwk> {
    const entry = this.store.keys[keyRef];
    if (!entry) throw new SecretNotFoundError(keyRef);
    return entry.publicJwk;
  }

  async sign(input: {
    keyRef: string;
    payload: Uint8Array;
  }): Promise<{ signature: Uint8Array; format: "raw" }> {
    const entry = this.store.keys[input.keyRef];
    if (!entry) throw new SecretNotFoundError(input.keyRef);
    return {
      signature: await signWithCurveKey(entry.privateRecord, input.payload),
      format: "raw",
    };
  }

  async verify(input: VerifyInput): Promise<boolean> {
    const publicJwk =
      input.publicJwk ??
      (input.keyRef ? await this.getPublicKey(input.keyRef) : undefined);
    if (!publicJwk) {
      throw new SecretStoreInitError("verify requires keyRef or publicJwk");
    }
    return verifyWithPublicJwk(publicJwk, input.payload, input.signature);
  }

  async deleteKey(keyRef: string): Promise<void> {
    if (!this.store.keys[keyRef]) throw new SecretNotFoundError(keyRef);
    delete this.store.keys[keyRef];
    await this.persist();
  }

  async getPublicForLedger(keyRef: string): Promise<{
    kty: "EC" | "OKP";
    crv: "Ed25519" | "Jubjub" | "P-256";
    x: bigint;
    y: bigint;
  }> {
    const publicJwk = await this.getPublicKey(keyRef);
    return normalizePublicForLedger(publicJwk);
  }

  private async unlockWithPassphrase(
    passphrase: string,
    salt: Buffer,
  ): Promise<void> {
    const key = await deriveKey(passphrase, salt);
    this.clearEncryptionMaterial();
    this.encryptionKey = key;
    this.encryptionSalt = Buffer.from(salt);
  }

  private requireEncryptionKey(): Buffer {
    if (!this.encryptionKey) throw new SecretStoreLockedError();
    return this.encryptionKey;
  }

  private requireEncryptionSalt(): Buffer {
    if (!this.encryptionSalt) throw new SecretStoreLockedError();
    return this.encryptionSalt;
  }

  private clearEncryptionMaterial(): void {
    this.encryptionKey?.fill(0);
    this.encryptionSalt?.fill(0);
    this.encryptionKey = undefined;
    this.encryptionSalt = undefined;
  }

  private async persist(): Promise<void> {
    const encrypted = encryptJsonWithKey(
      JSON.stringify(this.store),
      this.requireEncryptionKey(),
      this.requireEncryptionSalt(),
    );
    const envelope: FileEnvelope = {
      version: 1,
      encrypted,
    };
    await writePrivateAtomic(this.location, JSON.stringify(envelope, null, 2));
  }
}

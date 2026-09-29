import {
  mkdtemp,
  readdir,
  readFile,
  rm,
  stat,
  writeFile,
} from "node:fs/promises";
import os from "node:os";
import path from "node:path";

import { afterEach, describe, expect, it } from "vitest";

import { decryptJson, type EncryptedPayload, encryptJson } from "../crypto.js";
import {
  SecretNotFoundError,
  SecretStoreInitError,
  SecretStoreLockedError,
} from "../errors.js";
import { FileSecretStore } from "../file-secret-store.js";

const tempDirs: string[] = [];
const payload = Buffer.from("file-secret-store-payload", "utf8");
const seedHex =
  "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f";
const passphrase = "midnight-passphrase";

type TestStoreFile = {
  version: 1;
  keys: Record<
    string,
    {
      publicJwk: { kty: string; crv: string; x: string; y?: string };
    }
  >;
};

const toLegacyLittleEndianCoordinate = (coordinate: string): string => {
  const bytes = Buffer.from(coordinate, "base64url");
  return Buffer.from(bytes).reverse().toString("base64url");
};

const rewriteJubjubCoordinatesAsLegacy = async (
  location: string,
  keyRef: string,
): Promise<void> => {
  const envelope = JSON.parse(await readFile(location, "utf8")) as {
    version: 1;
    encrypted: EncryptedPayload;
  };
  const store = JSON.parse(
    await decryptJson(envelope.encrypted, passphrase),
  ) as TestStoreFile;
  const publicJwk = store.keys[keyRef]?.publicJwk;
  if (!publicJwk?.y) throw new Error("Expected a stored Jubjub public key");
  publicJwk.x = toLegacyLittleEndianCoordinate(publicJwk.x);
  publicJwk.y = toLegacyLittleEndianCoordinate(publicJwk.y);
  envelope.encrypted = await encryptJson(JSON.stringify(store), passphrase);
  await writeFile(location, JSON.stringify(envelope, null, 2), {
    mode: 0o600,
  });
};

const createTempPath = async (): Promise<string> => {
  const dir = await mkdtemp(path.join(os.tmpdir(), "midnight-secret-store-"));
  tempDirs.push(dir);
  return path.join(dir, "secrets.json");
};

const expectPrivateFileMode = async (location: string): Promise<void> => {
  const mode = (await stat(location)).mode & 0o777;
  expect(mode).toBe(0o600);
};

const expectNoTempWritesLeftBehind = async (
  location: string,
): Promise<void> => {
  const entries = await readdir(path.dirname(location));
  expect(entries.filter((entry) => entry.endsWith(".tmp"))).toEqual([]);
};

afterEach(async () => {
  await Promise.all(
    tempDirs
      .splice(0)
      .map(async (dir) => rm(dir, { recursive: true, force: true })),
  );
});

describe("FileSecretStore", () => {
  it("requires a passphrase to initialize", async () => {
    const store = new FileSecretStore();
    const location = await createTempPath();

    await expect(store.initialize({ location })).rejects.toThrow(
      SecretStoreLockedError,
    );
  });

  it("creates, persists, lists, signs, verifies and deletes keys", async () => {
    const location = await createTempPath();
    const store = new FileSecretStore();
    await store.initialize({ location, passphrase });

    await expectPrivateFileMode(location);
    await expectNoTempWritesLeftBehind(location);
    expect(await store.listKeys()).toEqual([]);

    const generated = await store.generateKey({
      id: "auth-main",
      kty: "OKP",
      crv: "Ed25519",
      did: "did:midnight:undeployed:abc",
      purpose: "authentication",
    });

    await expectPrivateFileMode(location);
    await expectNoTempWritesLeftBehind(location);

    const listed = await store.listKeys({ did: "did:midnight:undeployed:abc" });
    expect(listed).toHaveLength(1);
    expect(await store.getPublicKey(generated.keyRef)).toEqual(
      generated.publicJwk,
    );

    const signed = await store.sign({ keyRef: generated.keyRef, payload });
    await expect(
      store.verify({
        keyRef: generated.keyRef,
        payload,
        signature: signed.signature,
      }),
    ).resolves.toBe(true);
    await expect(
      store.verify({
        publicJwk: generated.publicJwk,
        payload,
        signature: signed.signature,
      }),
    ).resolves.toBe(true);

    const ledgerPublic = await store.getPublicForLedger(generated.keyRef);
    expect(ledgerPublic).toMatchObject({
      kty: "OKP",
      crv: "Ed25519",
      x: expect.any(BigInt),
      y: expect.any(BigInt),
    });

    await store.deleteKey(generated.keyRef);
    await expectPrivateFileMode(location);
    await expectNoTempWritesLeftBehind(location);
    await expect(store.getPublicKey(generated.keyRef)).rejects.toThrow(
      SecretNotFoundError,
    );
  });

  it("imports and derives keys, then persists encrypted state on disk", async () => {
    const location = await createTempPath();
    const store = new FileSecretStore();
    await store.initialize({ location, passphrase });

    const derived = await store.deriveKeyFromSeed({
      id: "seeded-p256",
      seedHex,
      kty: "EC",
      crv: "P-256",
      did: "did:midnight:undeployed:def",
    });
    const imported = await store.importKey({
      id: "jubjub-import",
      privateKey: new Uint8Array(32).fill(7),
      kty: "EC",
      crv: "Jubjub",
    });

    await expectPrivateFileMode(location);
    await expectNoTempWritesLeftBehind(location);

    const rawFile = JSON.parse(await readFile(location, "utf8")) as {
      encrypted?: unknown;
      keys?: unknown;
    };
    expect(rawFile.encrypted).toBeDefined();
    expect(rawFile.keys).toBeUndefined();

    const reopened = new FileSecretStore();
    await reopened.initialize({ location, passphrase });
    await expectPrivateFileMode(location);
    expect(await reopened.getPublicKey(derived.keyRef)).toEqual(
      derived.publicJwk,
    );
    expect(await reopened.getPublicKey(imported.keyRef)).toEqual(
      imported.publicJwk,
    );
  });

  it("migrates persisted Jubjub public keys to canonical 0.7 coordinates", async () => {
    const location = await createTempPath();
    const store = new FileSecretStore();
    await store.initialize({ location, passphrase });
    const imported = await store.importKey({
      id: "legacy-jubjub",
      privateKey: new Uint8Array(32).fill(9),
      kty: "EC",
      crv: "Jubjub",
    });

    await rewriteJubjubCoordinatesAsLegacy(location, imported.keyRef);

    const reopened = new FileSecretStore();
    await reopened.initialize({ location, passphrase });

    expect(await reopened.getPublicKey(imported.keyRef)).toEqual(
      imported.publicJwk,
    );
    await expectPrivateFileMode(location);

    const reopenedAgain = new FileSecretStore();
    await reopenedAgain.initialize({ location, passphrase });
    expect(await reopenedAgain.getPublicKey(imported.keyRef)).toEqual(
      imported.publicJwk,
    );
  });

  it("rejects invalid reopen/verify flows", async () => {
    const location = await createTempPath();
    const store = new FileSecretStore();
    await store.initialize({ location, passphrase });

    await expect(
      store.verify({ payload, signature: new Uint8Array(64) }),
    ).rejects.toThrow(SecretStoreInitError);

    const reopened = new FileSecretStore();
    await expect(
      reopened.initialize({ location, passphrase: "wrong-passphrase" }),
    ).rejects.toThrow(SecretStoreInitError);
  });
});

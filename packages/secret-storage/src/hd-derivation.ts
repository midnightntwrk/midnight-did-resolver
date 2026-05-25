import { hkdfSync } from "node:crypto";

import { HDWallet, Roles } from "@midnight-ntwrk/wallet-sdk-hd";

import { UnsupportedCurveError } from "./errors.js";
import { seedToBuffer } from "./seed.js";
import type { DeriveKeyFromSeedInput, ImportKeyInput } from "./types.js";

const P256_ORDER = BigInt(
  "0xffffffff00000000ffffffffffffffffbce6faada7179e84f3b9cac2fc632551",
);

const HKDF_SALT = Buffer.from("midnight-did-secret-storage-v1", "utf8");

const bigintTo32Be = (value: bigint): Buffer => {
  const hex = value.toString(16).padStart(64, "0");
  return Buffer.from(hex, "hex");
};

const normalizeP256Private = (privateKey: Buffer): Buffer => {
  const asInt = BigInt(`0x${privateKey.toString("hex")}`);
  const normalized = (asInt % (P256_ORDER - 1n)) + 1n;
  return bigintTo32Be(normalized);
};

const deriveMetadataKey = (
  seedHex: string,
  account: number,
  index: number,
): Uint8Array => {
  const seed = seedToBuffer(seedHex);
  let hdWalletResult: ReturnType<typeof HDWallet.fromSeed>;
  try {
    hdWalletResult = HDWallet.fromSeed(seed);
  } finally {
    seed.fill(0);
  }

  if (hdWalletResult.type !== "seedOk") {
    throw new Error("Failed to initialize HD wallet from seed");
  }

  const wallet = hdWalletResult.hdWallet;
  try {
    const derivationResult = wallet
      .selectAccount(account)
      .selectRole(Roles.Metadata)
      .deriveKeyAt(index);

    if (derivationResult.type !== "keyDerived") {
      throw new Error("Failed to derive metadata key from seed");
    }

    return derivationResult.key;
  } finally {
    wallet.clear();
  }
};

export const deriveCurvePrivateFromSeed = (
  params: DeriveKeyFromSeedInput,
  candidate = 0,
): Pick<ImportKeyInput, "privateKey" | "kty" | "crv"> => {
  const account = params.account ?? 0;
  const index = params.index ?? 0;
  if (account < 0 || index < 0 || candidate < 0) {
    throw new Error(
      "account, index, and candidate must be non-negative integers",
    );
  }

  const metadataKey = deriveMetadataKey(params.seedHex, account, index);
  const hkdfInput = Buffer.from(metadataKey);
  const info = Buffer.from(
    `midnight-did:key:v1:${params.kty}:${params.crv}:${account}:${index}:${candidate}`,
    "utf8",
  );
  const derived = Buffer.from(
    hkdfSync("sha256", hkdfInput, HKDF_SALT, info, 32),
  );

  try {
    if (params.kty === "OKP" && params.crv === "Ed25519") {
      return {
        kty: params.kty,
        crv: params.crv,
        privateKey: Uint8Array.from(derived),
      };
    }

    if (params.kty === "EC" && params.crv === "Jubjub") {
      return {
        kty: params.kty,
        crv: params.crv,
        privateKey: Uint8Array.from(derived),
      };
    }

    if (params.kty === "EC" && params.crv === "P-256") {
      const normalized = normalizeP256Private(derived);
      try {
        return {
          kty: params.kty,
          crv: params.crv,
          privateKey: Uint8Array.from(normalized),
        };
      } finally {
        normalized.fill(0);
      }
    }

    throw new UnsupportedCurveError(`${params.kty}/${params.crv}`);
  } finally {
    metadataKey.fill(0);
    hkdfInput.fill(0);
    info.fill(0);
    derived.fill(0);
  }
};

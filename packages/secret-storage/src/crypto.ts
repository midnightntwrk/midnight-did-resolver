import {
  createCipheriv,
  createDecipheriv,
  randomBytes,
  scrypt as scryptCb,
} from "node:crypto";
import { promisify } from "node:util";

const scrypt = promisify(scryptCb);
const KEY_SIZE = 32;
const IV_SIZE = 12;

export type EncryptedPayload = {
  salt: string;
  iv: string;
  tag: string;
  ciphertext: string;
};

export const generateEncryptionSalt = (): Buffer => randomBytes(16);

export const deriveKey = async (
  passphrase: string,
  salt: Buffer,
): Promise<Buffer> => {
  const key = await scrypt(passphrase, salt, KEY_SIZE);
  return Buffer.from(key as ArrayBuffer);
};

export const encryptJsonWithKey = (
  plaintext: string,
  key: Buffer,
  salt: Buffer,
): EncryptedPayload => {
  const iv = randomBytes(IV_SIZE);
  const cipher = createCipheriv("aes-256-gcm", key, iv);
  const ciphertext = Buffer.concat([
    cipher.update(plaintext, "utf8"),
    cipher.final(),
  ]);
  const tag = cipher.getAuthTag();

  try {
    return {
      salt: salt.toString("base64"),
      iv: iv.toString("base64"),
      tag: tag.toString("base64"),
      ciphertext: ciphertext.toString("base64"),
    };
  } finally {
    ciphertext.fill(0);
    tag.fill(0);
    iv.fill(0);
  }
};

export const decryptJsonWithKey = (
  payload: EncryptedPayload,
  key: Buffer,
): string => {
  const iv = Buffer.from(payload.iv, "base64");
  const tag = Buffer.from(payload.tag, "base64");
  const ciphertext = Buffer.from(payload.ciphertext, "base64");
  const decipher = createDecipheriv("aes-256-gcm", key, iv);
  decipher.setAuthTag(tag);
  const plaintext = Buffer.concat([
    decipher.update(ciphertext),
    decipher.final(),
  ]);

  try {
    return plaintext.toString("utf8");
  } finally {
    plaintext.fill(0);
    ciphertext.fill(0);
    tag.fill(0);
    iv.fill(0);
  }
};

export const encryptJson = async (
  plaintext: string,
  passphrase: string,
): Promise<EncryptedPayload> => {
  const salt = generateEncryptionSalt();
  const key = await deriveKey(passphrase, salt);
  try {
    return encryptJsonWithKey(plaintext, key, salt);
  } finally {
    key.fill(0);
    salt.fill(0);
  }
};

export const decryptJson = async (
  payload: EncryptedPayload,
  passphrase: string,
): Promise<string> => {
  const salt = Buffer.from(payload.salt, "base64");
  const key = await deriveKey(passphrase, salt);
  try {
    return decryptJsonWithKey(payload, key);
  } finally {
    key.fill(0);
    salt.fill(0);
  }
};

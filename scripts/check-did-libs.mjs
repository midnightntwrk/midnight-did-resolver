#!/usr/bin/env node
import { existsSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const requiredTarballs = [
  "midnight-ntwrk-midnight-did-api-0.1.0.tgz",
  "midnight-ntwrk-midnight-did-contract-0.1.0.tgz",
  "midnight-ntwrk-midnight-did-domain-0.1.0.tgz",
  "midnight-ntwrk-midnight-did-jubjub-schnorr-0.1.0.tgz",
  "midnight-ntwrk-midnight-did-0.1.0.tgz",
];

const missing = requiredTarballs.filter((name) => !existsSync(path.join(root, "libs", "midnight-did", name)));

if (missing.length > 0) {
  console.error("[check-did-libs] Missing DID package tarballs:");
  for (const name of missing) console.error(`  - libs/midnight-did/${name}`);
  console.error("\nRefresh them from the workspace root:");
  console.error("  ./scripts/sync-package-tarballs.sh --source did --destination midnight-did-resolver");
  process.exit(1);
}

#!/usr/bin/env node
import { existsSync, readFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const packageJson = JSON.parse(readFileSync(path.join(root, "package.json"), "utf8"));

const didPackageSpecs = [
  packageJson.dependencies,
  packageJson.devDependencies,
  packageJson.overrides,
  packageJson.pnpm?.overrides,
];

const requiredTarballs = didPackageSpecs
  .flatMap((section) => Object.values(section ?? {}))
  .filter((value) => typeof value === "string" && value.startsWith("file:libs/midnight-did/"))
  .map((value) => value.slice("file:libs/midnight-did/".length))
  .sort();

const missing = requiredTarballs.filter((name) => !existsSync(path.join(root, "libs", "midnight-did", name)));

if (missing.length > 0) {
  console.error("[check-did-libs] Missing DID package tarballs:");
  for (const name of missing) console.error(`  - libs/midnight-did/${name}`);
  console.error("\nRefresh them from the workspace root:");
  console.error("  ./scripts/sync-package-tarballs.sh --source did --destination midnight-did-resolver");
  process.exit(1);
}

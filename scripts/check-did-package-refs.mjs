#!/usr/bin/env node
import { existsSync, readdirSync, readFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const packageFiles = [
  "package.json",
  "did-resolver-service/package.json",
  "did-manager-service/package.json",
  "secret-storage/package.json",
];

const didPackageRefs = new Map([
  ["@midnight-ntwrk/midnight-did", "0.5.0"],
  ["@midnight-ntwrk/midnight-did-api", "0.5.0"],
  ["@midnight-ntwrk/midnight-did-contract", "0.5.0"],
  ["@midnight-ntwrk/midnight-did-domain", "0.5.0"],
  ["@midnight-ntwrk/midnight-did-jubjub-schnorr", "0.5.0"],
]);

const readJson = (relativePath) =>
  JSON.parse(readFileSync(path.join(root, relativePath), "utf8"));

const failures = [];
const rootPackageJson = readJson("package.json");

if (
  rootPackageJson.dependencies?.["@midnight-ntwrk/contract"] !==
  "npm:@midnight-ntwrk/midnight-did-contract@0.5.0"
) {
  failures.push(
    "package.json dependencies.@midnight-ntwrk/contract must alias the 0.5.0 contract package for the DID API default ZK artifact path",
  );
}

if (rootPackageJson.overrides?.["@midnight-ntwrk/ledger-v8"] !== "8.1.0") {
  failures.push(
    "package.json overrides.@midnight-ntwrk/ledger-v8 must be 8.1.0 to keep DID API wallet WASM classes on one ledger instance",
  );
}

if (
  rootPackageJson.overrides?.["@midnight-ntwrk/midnight-js-network-id"] !==
  "4.0.2"
) {
  failures.push(
    "package.json overrides.@midnight-ntwrk/midnight-js-network-id must be 4.0.2 to share DID API network configuration across the contract graph",
  );
}

for (const [packageName, expectedRef] of didPackageRefs) {
  const dependencyRef = rootPackageJson.dependencies?.[packageName];
  const overrideRef = rootPackageJson.overrides?.[packageName];

  if (dependencyRef !== expectedRef) {
    failures.push(
      `package.json dependencies.${packageName} must be ${expectedRef}`,
    );
  }
  if (overrideRef !== expectedRef) {
    failures.push(
      `package.json overrides.${packageName} must be ${expectedRef}`,
    );
  }
}

for (const packageFile of packageFiles) {
  const packageJson = readJson(packageFile);
  for (const field of [
    "dependencies",
    "devDependencies",
    "peerDependencies",
    "optionalDependencies",
  ]) {
    for (const [packageName, spec] of Object.entries(packageJson[field] ?? {})) {
      if (!didPackageRefs.has(packageName)) {
        continue;
      }

      if (packageFile === "package.json") {
        continue;
      }

      const expectedRef = didPackageRefs.get(packageName);
      if (spec !== expectedRef) {
        failures.push(
          `${packageFile} ${field}.${packageName} must use ${expectedRef} and rely on root overrides`,
        );
      }
    }
  }
}

const didLibDir = path.join(root, "libs", "midnight-did");
const remainingTarballs = existsSync(didLibDir)
  ? readdirSync(didLibDir, { withFileTypes: true })
      .filter((entry) => entry.isFile() && entry.name.endsWith(".tgz"))
      .map((entry) => entry.name)
      .sort()
  : [];

if (remainingTarballs.length > 0) {
  failures.push(
    `libs/midnight-did must not contain checked-in DID tarballs: ${remainingTarballs.join(", ")}`,
  );
}

if (failures.length > 0) {
  console.error("[check-did-package-refs] DID package reference check failed:");
  for (const failure of failures) {
    console.error(`- ${failure}`);
  }
  process.exit(1);
}

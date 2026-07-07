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
  [
    "@midnight-ntwrk/midnight-did",
    "git+https://github.com/midnightntwrk/midnight-did.git#npm-midnight-did-v0.4.0",
  ],
  [
    "@midnight-ntwrk/midnight-did-api",
    "git+https://github.com/midnightntwrk/midnight-did.git#npm-midnight-did-api-v0.4.0",
  ],
  [
    "@midnight-ntwrk/midnight-did-contract",
    "git+https://github.com/midnightntwrk/midnight-did.git#npm-midnight-did-contract-v0.4.0",
  ],
  [
    "@midnight-ntwrk/midnight-did-domain",
    "git+https://github.com/midnightntwrk/midnight-did.git#npm-midnight-did-domain-v0.4.0",
  ],
  [
    "@midnight-ntwrk/midnight-did-jubjub-schnorr",
    "git+https://github.com/midnightntwrk/midnight-did.git#npm-midnight-did-jubjub-schnorr-v0.4.0",
  ],
]);

const readJson = (relativePath) =>
  JSON.parse(readFileSync(path.join(root, relativePath), "utf8"));

const failures = [];
const rootPackageJson = readJson("package.json");

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

      if (spec !== "0.4.0") {
        failures.push(
          `${packageFile} ${field}.${packageName} must use 0.4.0 and rely on root overrides`,
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

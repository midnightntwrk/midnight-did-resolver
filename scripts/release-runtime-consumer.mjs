// This file is part of midnightntwrk/midnight-did-resolver.
// SPDX-License-Identifier: Apache-2.0
// Mounted read-only into the candidate. Procedure adapted from parent-owned
// runtime-evidence/continuation-0726/{consumer,runtime-guard}-continuation.mjs.
// Never imports application/private producer files or host node_modules.
import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { createRequire } from "node:module";
import { createHash } from "node:crypto";
import { pathToFileURL } from "node:url";
const sha256 = bytes => createHash("sha256").update(bytes).digest("hex");
export async function probeCandidate({ service, expectedArch, load, require: candidateRequire }) {
  assert.ok(["manager", "resolver"].includes(service));
  assert.equal(process.version, "v24.22.3");
  assert.equal(process.arch, expectedArch);
  assert.equal(process.getuid(), 1000);
  const ledger = await load("@midnightntwrk/ledger-v9"); // actual owning WASM initialization
  const wasmHash = ledger.entryPointHash("release-public-runtime-probe");
  assert.match(wasmHash, /^[0-9a-f]{64}$/);
  assert.equal(ledger.decodeRawTokenType(ledger.encodeRawTokenType("00".repeat(32))), "00".repeat(32));
  await load("@midnight-ntwrk/midnight-did-api");
  const hd = await load("@midnight-ntwrk/wallet-sdk-hd");
  const seed = Buffer.alloc(32); seed[31] = 1; // public local fixture only
  const generated = hd.HDWallet.fromSeed(seed);
  assert.equal(generated.type, "seedOk");
  try {
    const key = generated.hdWallet.selectAccount(0).selectRole(4).deriveKeyAt(0);
    assert.equal(key.type, "keyDerived"); assert.equal(key.key.length, 32); key.key.fill(0);
  } finally { generated.hdWallet.clear(); seed.fill(0); }
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), "release-native-probe-"));
  const { ClassicLevel } = await load("classic-level");
  let db;
  try {
    db = new ClassicLevel(path.join(dir, "level"), { valueEncoding: "json" });
    await db.open(); await db.put("public-fixture", { persisted: true });
    assert.deepEqual(await db.get("public-fixture"), { persisted: true }); await db.close();
    db = new ClassicLevel(path.join(dir, "level"), { valueEncoding: "json" });
    await db.open(); assert.deepEqual(await db.get("public-fixture"), { persisted: true }); await db.close();
    if (service === "manager") {
      const { FileSecretStore } = await load("@midnight-ntwrk/midnight-did-secret-storage");
      const store = new FileSecretStore();
      await store.initialize({ location: path.join(dir, "public-storage.json"), passphrase: "public-disposable-fixture-only" });
      const { keyRef } = await store.deriveKeyFromSeed({ id: "release-public-key", seedHex: "0".repeat(63) + "1", kty: "OKP", crv: "Ed25519", account: 0, index: 0 });
      const payload = new TextEncoder().encode("public release runtime probe");
      const { signature } = await store.sign({ keyRef, payload });
      assert.equal(await store.verify({ keyRef, payload, signature }), true);
    }
  } finally { if (db) await db.close(); fs.rmSync(dir, { recursive: true, force: true }); }
  const { NodeZkConfigProvider } = await load("@midnight-ntwrk/midnight-js-node-zk-config-provider");
  const { makeLedger9ContractExecutableRuntime } = await load("@midnight-ntwrk/midnight-js-types");
  const { checkRuntimeVersion } = await load("@midnight-ntwrk/compact-runtime");
  const { DIDContract } = await load("@midnight-ntwrk/midnight-did-contract");
  const { CompiledContract, ZKConfiguration } = await load("@midnight-ntwrk/midnight-js-protocol/v9/compact-js/effect");
  const { Effect } = await load("effect");
  // Public package entry selects its companion assets, never producer workspace.
  const bundle = path.join(path.dirname(candidateRequire.resolve("@midnight-ntwrk/contract")), "managed/did");
  const provider = new NodeZkConfigProvider(bundle);
  const artifactVersion = await provider.getArtifactRuntimeVersion();
  assert.equal(artifactVersion, "0.21.0"); checkRuntimeVersion(artifactVersion);
  assert.throws(() => checkRuntimeVersion("999.0.0"), /Version mismatch/);
  const circuits = fs.readdirSync(path.join(bundle, "zkir")).filter(n => n.endsWith(".bzkir")).map(n => n.slice(0, -6));
  assert.equal(circuits.length, 12);
  const artifacts = [];
  for (const circuit of circuits) for (const [kind, method] of [["prover", "getProverKey"], ["verifier", "getVerifierKey"], ["zkir", "getZKIR"]]) {
    const bytes = await provider[method](circuit); assert.ok(bytes.length > 0);
    artifacts.push({ circuit, kind, bytes: bytes.length, sha256: sha256(bytes) });
  }
  const compiled = CompiledContract.make("release-public-runtime-probe", DIDContract.Contract);
  const runtime = makeLedger9ContractExecutableRuntime(provider, { coinPublicKey: "00".repeat(32) });
  try {
    const keys = await runtime.runPromise(Effect.gen(function* () {
      const cfg = yield* ZKConfiguration.ZKConfiguration;
      const reader = yield* cfg.createReader(compiled);
      return yield* reader.getVerifierKeys(circuits);
    }));
    assert.equal(keys.length, circuits.length);
    for (const [id, key] of keys) { assert.ok(circuits.includes(id)); assert.equal(key._tag, "Some"); }
  } finally { await runtime.dispose(); }
  return { schemaVersion: 1, verified: true, service, node: process.version, processArch: process.arch, uid: process.getuid(),
    ledgerWasmRoundtrip: true, hdRole4Derived: true, classicLevel: "open-put-get-close-reopen-get-close",
    storageSignVerify: service === "manager" ? true : "not-in-resolver-payload",
    artifactVersion, artifacts, verifierReads: 12, rejectedIncompatibleRuntime: true,
    scope: "Owning Ledger9 WASM + HD4 + native database + actual public executable-runtime configuration; not a funded session restore" };
}
if (process.argv[1] && pathToFileURL(path.resolve(process.argv[1])).href === import.meta.url) {
  const emit = process.stdout.write.bind(process.stdout);
  // SDK logging must not dump public-fixture key material or pollute JSON stdout.
  console.log = console.info = console.warn = console.error = () => {};
  const require = createRequire(path.join(process.cwd(), "package.json"));
  probeCandidate({ service: process.argv[2], expectedArch: process.argv[3], require,
    load: name => import(require.resolve(name)) })
    .then(result => emit(JSON.stringify(result) + "\n"))
    .catch(() => { process.stderr.write("Candidate native/public-runtime probe failed\n"); process.exitCode = 1; });
}

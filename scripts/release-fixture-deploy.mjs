// This file is part of midnightntwrk/midnight-did-resolver.
// SPDX-License-Identifier: Apache-2.0
// Runs ONLY inside verified manager/amd64 bytes. Public package exports only.
// Public API sequence follows resolver.did-flow.test.ts beforeAll and first update.
import assert from "node:assert/strict";
import { createRequire } from "node:module";
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { resolve, join } from "node:path";
import { pathToFileURL } from "node:url";
const sleep = ms => new Promise(ok => setTimeout(ok, ms));
export function validateDeploymentConfig(config) {
  assert.equal(config.schemaVersion, 1);
  for (const [name, protocol] of [["indexer", "http:"], ["indexerWS", "ws:"], ["node", "http:"], ["proofServer", "http:"]]) {
    const url = new URL(config[name]); assert.equal(url.hostname, "127.0.0.1");
    assert.equal(url.protocol, protocol); assert.equal(url.username + url.password + url.hash, "");
  }
  return config;
}
export async function deployFixture(config, { load, sleepImpl = sleep, now = Date.now, runtime = process } = {}) {
  validateDeploymentConfig(config);
  assert.equal(runtime.version, "v24.22.3"); assert.equal(runtime.arch, "x64"); assert.equal(runtime.getuid(), 1000);
  const api = await load("@midnight-ntwrk/midnight-did-api");
  const didPackage = await load("@midnight-ntwrk/midnight-did");
  const domain = await load("@midnight-ntwrk/midnight-did-domain");
  const { ContractState } = await load("@midnight-ntwrk/compact-runtime");
  const { DIDContract } = await load("@midnight-ntwrk/midnight-did-contract");
  const { indexerPublicDataProvider } = await load("@midnight-ntwrk/midnight-js-indexer-public-data-provider");
  const pinoModule = await load("pino"); api.setLogger(pinoModule.default({ level: "silent" }));
  const dir = await mkdtemp(join(tmpdir(), "release-fixture-deploy-"));
  let walletCtx, providers, publicProvider;
  try {
    const cfg = { ...new api.StandaloneConfig(), indexer: config.indexer, indexerWS: config.indexerWS,
      node: config.node, proofServer: config.proofServer, midnightDbName: join(dir, "wallet-db"), logDir: join(dir, "logs") };
    // Public deterministic genesis funds on this owned standalone only.
    walletCtx = await api.buildWalletAndWaitForFunds(cfg, "0".repeat(63) + "1");
    await api.registerForDustGeneration(walletCtx.wallet, walletCtx.unshieldedKeystore, {
      indexedBlockTime: api.createIndexerIndexedBlockTime(cfg.indexer), timeoutMs: 120000,
    });
    providers = await api.configureProviders(walletCtx, cfg);
    const privateState = await api.initPrivateState(providers);
    let contract;
    for (let attempt = 0; attempt <= 2; attempt++) {
      try { contract = await api.createDID(providers, privateState); break; }
      catch (error) {
        if (attempt === 2 || !/Not enough Dust generated to pay the fee|could not balance dust/i.test(error?.message || "")) throw error;
        await sleepImpl(8000);
      }
    }
    const address = didPackage.parseContractAddress(contract.deployTxData.public.contractAddress);
    const did = didPackage.createMidnightDIDString(address, api.getMidnightNetwork());
    const method = domain.createVerificationMethod({ id: `${did}#release-public-key`,
      type: domain.VerificationMethodType.JsonWebKey, controller: did,
      publicKeyJwk: { kty: domain.KeyType.OKP, crv: domain.CurveType.Ed25519, x: "KioqKioqKioqKioqKioqKioqKioqKioqKioqKioqKio" } });
    await api.addVerificationMethod(contract, providers, method);
    publicProvider = indexerPublicDataProvider(cfg.indexer, cfg.indexerWS);
    const resolver = new didPackage.MidnightDIDResolver({
      expectedNetwork: didPackage.MidnightNetwork.Undeployed,
      ledgerReader: async contractAddress => {
        const read = await publicProvider.queryContractState(contractAddress);
        return read === null ? null : DIDContract.ledger(read.contractState.deserialize({ v9: ContractState }).data);
      },
    });
    const deadline = now() + 180000;
    while (now() < deadline) {
      // Read-only indexer visibility polling, never retry deploy/fund/full gate.
      let timer;
      let result;
      try { result = await Promise.race([resolver.resolveResult(did), new Promise((_, reject) => {
        timer = setTimeout(() => reject(new Error("Fixture resolution timeout")), 15000);
      })]); } finally { clearTimeout(timer); }
      if (result?.didDocument?.id === did && result.didDocument.verificationMethod?.some(m => m.id === method.id)) {
        return { schemaVersion: 1, kind: "standalone-external-state", did,
          indexerHTTP: cfg.indexer, indexerWS: cfg.indexerWS,
          expectedResolution: { ...result, didResolutionMetadata: { contentType: "application/did+ld+json", error: null } },
          consumerLimits: ["Owned standalone genesis only", "No production funds", "Metadata persistence, not manager funded-session restoration"] };
      }
      await sleepImpl(2500);
    }
    throw new Error("Positive public fixture never became visible");
  } finally {
    try { if (publicProvider) await publicProvider.dispose(); }
    finally {
      try { if (providers?.publicDataProvider) await providers.publicDataProvider.dispose(); }
      finally {
        try { if (walletCtx) await walletCtx.wallet.stop(); }
        finally { await rm(dir, { recursive: true, force: true }); }
      }
    }
  }
}
if (process.argv[1] && pathToFileURL(resolve(process.argv[1])).href === import.meta.url) {
  const emit = process.stdout.write.bind(process.stdout);
  console.log = console.info = console.warn = console.error = () => {};
  const require = createRequire(join(process.cwd(), "package.json"));
  Promise.resolve().then(() => deployFixture(JSON.parse(process.argv[2]), { load: name => import(require.resolve(name)) }))
    .then(fixture => emit(JSON.stringify(fixture) + "\n"))
    .catch(() => { process.stderr.write("Owned standalone public fixture deployment failed\n"); process.exitCode = 1; });
}

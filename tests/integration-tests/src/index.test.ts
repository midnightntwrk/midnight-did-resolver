import { describe, test, beforeAll, expect } from 'vitest';
import * as api from '@midnight-ntwrk/midnight-did-api';
import * as did from '@midnight-ntwrk/midnight-did';

const RESOLVER_URL = process.env.RESOLVER_URL || 'http://localhost:8080';
const GENESIS_MINT_WALLET_SEED = '0000000000000000000000000000000000000000000000000000000000000001';

const didConfig = new api.StandaloneConfig();
const logger = await api.createLogger("test.log");
api.setLogger(logger);

let wallet: Awaited<ReturnType<typeof api.buildWalletAndWaitForFunds>>;
let providers: Awaited<ReturnType<typeof api.configureProviders>>;

async function resolveDID(didStr: string) {
  const resolverEndpoint = `${RESOLVER_URL}/api/dids/${encodeURIComponent(didStr)}`;
  const response = await fetch(resolverEndpoint, {
    method: 'GET',
    headers: {
      'Accept': 'application/did-resolution',
    },
  });

  const resolutionResult = await response.json();
  return resolutionResult;
}

describe('Midnight DID Resolver - Integration Tests', () => {
  beforeAll(async () => {
    logger.info('═══════════════════════════════════════════════════════════════');
    logger.info('  Midnight DID Resolver - Integration Tests');
    logger.info('═══════════════════════════════════════════════════════════════');
    logger.info('');
    logger.info('Configuration:');
    logger.info(`  resolver URL: ${RESOLVER_URL}`);
    for (const [key, value] of Object.entries(didConfig)) {
      logger.info(`  ${key}: ${value}`);
    }
    logger.info('');

    // Initialize midnight network
    wallet = await api.buildWalletAndWaitForFunds(didConfig, GENESIS_MINT_WALLET_SEED, '');
    providers = await api.configureProviders(wallet, didConfig);
  });

  describe('DID Resolution', () => {
    test('should resolves newly created empty DID', async () => {
      const privateState = await api.initPrivateState(providers);
      const didContract = await api.createDID(providers, privateState);
      const contractAddress = did.parseContractAddress(didContract.deployTxData.public.contractAddress);
      const didStr = did.createMidnightDIDString(contractAddress, api.midnightNetwork);
      const resolutionResult = await resolveDID(didStr);

      // Verify successful resolution
      expect(resolutionResult).toBeDefined();
      expect(resolutionResult.didResolutionMetadata).toBeDefined();
      expect(resolutionResult.didResolutionMetadata.error).toBeNull();

      // Verify DID Document structure
      const didDocument = resolutionResult.didDocument;
      expect(didDocument).toBeDefined();

      // Verify @context
      expect(didDocument['@context']).toEqual([
        'https://www.w3.org/ns/did/v1',
        'https://w3c.github.io/vc-jws-2020/contexts/v1'
      ]);

      // Verify id matches the requested DID
      expect(didDocument.id).toBe(didStr);

      // Verify all arrays are empty
      expect(didDocument.alsoKnownAs).toEqual([]);
      expect(didDocument.verificationMethod).toEqual([]);
      expect(didDocument.authentication).toEqual([]);
      expect(didDocument.assertionMethod).toEqual([]);
      expect(didDocument.keyAgreement).toEqual([]);
      expect(didDocument.capabilityInvocation).toEqual([]);
      expect(didDocument.capabilityDelegation).toEqual([]);
      expect(didDocument.service).toEqual([]);

      // Verify metadata
      const metadata = resolutionResult.didDocumentMetadata;
      expect(metadata).toBeDefined();
      expect(metadata.created).toBeDefined();
      expect(metadata.created).toMatch(/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}Z$/);
      expect(metadata.updated).toBeDefined();
      expect(metadata.updated).toMatch(/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}Z$/);
      expect(metadata.deactivated).toBeDefined();
      expect(metadata.deactivated).toBe(false);
      expect(metadata.versionId).toBeDefined();
      expect(metadata.versionId).toBe("0");
    });
  });
});

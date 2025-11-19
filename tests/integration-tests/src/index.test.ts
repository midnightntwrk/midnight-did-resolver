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

  describe('Basic DID Resolution (Empty State)', () => {
    test('should resolve empty DID with minimal document', async () => {
      const privateState = await api.initPrivateState(providers);
      console.log("before createDID");
      const didContract = await api.createDID(providers, privateState);
      console.log("after createDID");
      const contractAddress = did.parseContractAddress(didContract.deployTxData.public.contractAddress);
      const didStr = did.createMidnightDIDString(contractAddress, api.midnightNetwork);
      const initialDocument: did.MidnightDIDDocument = did.createMidnightDIDDocument({ id: didStr });
      console.log(`DID created: ${didStr}`);

      await new Promise(resolve => setTimeout(resolve, 5000));

      const resolutionResult = await resolveDID(didStr);
      console.log('resolutionResult', resolutionResult);

      // TODO: assert did document
    });
  });
});

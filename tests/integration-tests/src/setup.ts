import * as api from '@midnight-ntwrk/midnight-did-api';
import * as did from '@midnight-ntwrk/midnight-did';

// Singleton pattern to ensure setup runs only once
let isInitialized = false;
let wallet: Awaited<ReturnType<typeof api.buildWalletAndWaitForFunds>>;
let providers: Awaited<ReturnType<typeof api.configureProviders>>;
let logger: Awaited<ReturnType<typeof api.createLogger>>;
let didConfig: api.StandaloneConfig;

// Constants
export const RESOLVER_URL = process.env.RESOLVER_URL || 'http://localhost:8080';
export const GENESIS_MINT_WALLET_SEED = '0000000000000000000000000000000000000000000000000000000000000001';

/**
 * Setup function - idempotent, runs only once
 */
export async function setupOnce() {
  if (isInitialized) {
    return;
  }

  didConfig = new api.StandaloneConfig();
  logger = await api.createLogger("test.log");
  api.setLogger(logger);

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

  // Initialize midnight network (EXPENSIVE OPERATION)
  wallet = await api.buildWalletAndWaitForFunds(didConfig, GENESIS_MINT_WALLET_SEED, '');
  providers = await api.configureProviders(wallet, didConfig);

  isInitialized = true;
}

/**
 * Get wallet instance
 */
export function getWallet() {
  if (!isInitialized) {
    throw new Error('Setup not initialized. Call setupOnce() first.');
  }
  return wallet;
}

/**
 * Get providers instance
 */
export function getProviders() {
  if (!isInitialized) {
    throw new Error('Setup not initialized. Call setupOnce() first.');
  }
  return providers;
}

/**
 * Get logger instance
 */
export function getLogger(): Awaited<ReturnType<typeof api.createLogger>> {
  if (!isInitialized) {
    throw new Error('Setup not initialized. Call setupOnce() first.');
  }
  return logger;
}

/**
 * Resolve a DID via the resolver API
 */
export async function resolveDID(didStr: string) {
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

/**
 * Create a test DID
 */
export async function createTestDID(): Promise<{
  didContract: Awaited<ReturnType<typeof api.createDID>>;
  didStr: string;
  contractAddress: ReturnType<typeof did.parseContractAddress>;
  privateState: Awaited<ReturnType<typeof api.initPrivateState>>;
}> {
  const providers = getProviders();
  
  const privateState = await api.initPrivateState(providers);
  const didContract = await api.createDID(providers, privateState);
  const contractAddress = did.parseContractAddress(
    didContract.deployTxData.public.contractAddress
  );
  const didStr = did.createMidnightDIDString(contractAddress, api.midnightNetwork);
  
  return {
    didContract,
    didStr,
    contractAddress,
    privateState
  };
}

/**
 * Wait for DID resolution with retry logic
 * Useful when waiting for indexer to process transactions
 */
export async function waitForResolution(
  didStr: string, 
  maxAttempts: number = 10,
  delayMs: number = 2000
): Promise<any> {
  for (let attempt = 1; attempt <= maxAttempts; attempt++) {
    try {
      const result = await resolveDID(didStr);
      if (result.didDocument) {
        return result;
      }
    } catch (error) {
      if (attempt === maxAttempts) {
        throw error;
      }
    }
    await new Promise(resolve => setTimeout(resolve, delayMs));
  }
  throw new Error(`Failed to resolve DID after ${maxAttempts} attempts`);
}

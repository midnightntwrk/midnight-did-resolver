import path from 'node:path';

import { describe, expect, it, vi } from 'vitest';

const apiMock = vi.hoisted(() => ({
  MainnetConfig: vi.fn(),
  PreprodConfig: vi.fn(),
  StandaloneConfig: vi.fn(),
  deriveUnshieldedAddressFromSeed: vi.fn(() => 'mn_addr_derived'),
  getMidnightDIDLedgerState: vi.fn(),
  joinContract: vi.fn(),
}));
const networkMock = vi.hoisted(() => ({
  getNetworkId: vi.fn(() => 'undeployed'),
  setNetworkId: vi.fn(),
}));

vi.mock('@midnight-ntwrk/midnight-did-api', () => apiMock);
vi.mock('@midnight-ntwrk/midnight-did', () => ({
  MidnightNetwork: {
    Undeployed: 'undeployed', DevNet: 'devnet', Testnet: 'testnet',
    Mainnet: 'mainnet', Preview: 'preview', Preprod: 'preprod',
  },
  createMidnightDIDString: vi.fn((address, network) => `did:midnight:${network}:${address}`),
  parseContractAddress: vi.fn((address) => address),
}));
vi.mock('@midnight-ntwrk/midnight-did-domain', () => ({
  KeyType: { EC: 'EC', OKP: 'OKP' },
  VerificationMethodType: { JsonWebKey: 'JsonWebKey' },
  createVerificationMethod: vi.fn((input) => input),
}));
vi.mock('@midnight-ntwrk/midnight-did-secret-storage', () => ({
  FileSecretStore: class {
    initialize = vi.fn().mockResolvedValue(undefined);
  },
  parseSeed: vi.fn((seed) => `parsed:${seed}`),
}));
vi.mock('@midnight-ntwrk/midnight-js-network-id', () => networkMock);

import {
  buildProfileConfig,
  buildVerificationMethod,
  createSecretStore,
  faucetUrl,
  generateSeedHex,
  joinExistingContract,
  midnightDbPath,
  nowIso,
  profileNamePattern,
  withTimeout,
} from '../manager/helpers.js';
import { buildSessionStatus, buildSetupStatus, deriveUnshieldedAddress, resolveSeedInput } from '../manager/wallet-session-service.js';

const config = {
  standalone: { indexer: 'i1', indexerWS: 'w1', node: 'n1', proofServer: 'p1' },
  preprod: { indexer: 'i2', indexerWS: 'w2', node: 'n2', proofServer: 'p2' },
  mainnet: { indexer: 'i3', indexerWS: 'w3', node: 'n3', proofServer: 'p3' },
} as any;

describe('manager helper coverage', () => {
  it('covers deterministic seed, path, profile, and timeout helpers', async () => {
    expect(nowIso()).toMatch(/^\d{4}-\d{2}-\d{2}T/);
    expect(generateSeedHex()).toMatch(/^[0-9a-f]{64}$/);
    expect(profileNamePattern.test('valid_profile-1')).toBe(true);
    expect(profileNamePattern.test(' bad')).toBe(false);
    expect(midnightDbPath('/tmp/profile', 'abc')).toBe(path.join('/tmp/profile', 'midnight-level-db', 'abc'));
    expect(faucetUrl('preprod')).toContain('faucet.preprod');
    expect(faucetUrl('standalone')).toBeNull();
    await expect(withTimeout(Promise.resolve('ok'), 100, 'quick')).resolves.toBe('ok');
    await expect(withTimeout(new Promise(() => undefined), 1, 'slow')).rejects.toThrow('slow timed out after 1ms');
  });

  it('builds profile configs and creates a secret store', async () => {
    expect(buildProfileConfig(config, 'standalone')).toMatchObject({ indexer: 'i1', node: 'n1' });
    expect(buildProfileConfig(config, 'preprod')).toMatchObject({ indexer: 'i2', node: 'n2' });
    expect(buildProfileConfig(config, 'mainnet')).toMatchObject({ indexer: 'i3', node: 'n3' });
    expect(apiMock.StandaloneConfig).toHaveBeenCalled();
    expect(apiMock.PreprodConfig).toHaveBeenCalled();
    expect(apiMock.MainnetConfig).toHaveBeenCalled();
    const store = await createSecretStore('/tmp/secrets', undefined, 'default-passphrase');
    expect(store).toBeDefined();
  });

  it('joins an existing contract only when ledger state is present', async () => {
    apiMock.getMidnightDIDLedgerState.mockResolvedValueOnce(null);
    await expect(joinExistingContract({} as never, 'a'.repeat(64), 'standalone')).rejects.toThrow('was not found');
    apiMock.getMidnightDIDLedgerState.mockResolvedValueOnce({ version: 1 });
    apiMock.joinContract.mockResolvedValue('joined');
    await expect(joinExistingContract({} as never, 'a'.repeat(64), 'standalone')).resolves.toBe('joined');
  });

  it('builds a verification method and delegates address derivation', () => {
    expect(buildVerificationMethod({ deployTxData: { public: { contractAddress: 'a'.repeat(64) } } } as never, 'key-1', {
      kty: 'OKP', crv: 'Ed25519', x: 'x',
    })).toMatchObject({ id: 'key-1', controller: expect.stringContaining('did:midnight') });
    expect(deriveUnshieldedAddress('seed')).toBe('mn_addr_derived');
  });

  it('builds setup and session status for each profile shape', () => {
    expect(buildSetupStatus(config, 'standalone')).toMatchObject({ profile: 'standalone', faucetUrl: null });
    expect(buildSetupStatus(config, 'preprod')).toMatchObject({ profile: 'preprod', faucetUrl: expect.any(String) });
    expect(buildSetupStatus(config, 'mainnet')).toMatchObject({ profile: 'mainnet', faucetUrl: null });
    expect(buildSessionStatus('standalone', 'default', true, undefined, null, { night: null, dust: null }, { phase: 'locked' } as never, { phase: 'none' } as never, false))
      .toMatchObject({ profileName: 'default', unlocked: false, seedAvailable: false });
    expect(resolveSeedInput('preprod', { seed: 'stored' } as never, { seedMode: 'reuse' })).toEqual({ seed: 'parsed:stored' });
    expect(resolveSeedInput('preprod', undefined, { seedMode: 'generated' }).generatedSeed).toMatch(/^[0-9a-f]{64}$/);
    expect(resolveSeedInput('preprod', undefined, { seedMode: 'provided', seed: 'provided' })).toEqual({ seed: 'parsed:provided' });
  });
});

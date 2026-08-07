import { describe, expect, it, vi } from 'vitest';

const apiMock = vi.hoisted(() => ({
  addAlsoKnownAs: vi.fn(),
  addService: vi.fn(),
  addSchnorrJubjubVerificationMethod: vi.fn(),
  addVerificationMethod: vi.fn(),
  addVerificationMethodRelation: vi.fn(),
  createDID: vi.fn(),
  deactivate: vi.fn(),
  getMidnightDIDLedgerState: vi.fn(),
  initPrivateState: vi.fn(),
  removeAlsoKnownAs: vi.fn(),
  removeService: vi.fn(),
  removeSchnorrJubjubVerificationMethod: vi.fn(),
  removeVerificationMethod: vi.fn(),
  removeVerificationMethodRelation: vi.fn(),
  resolve: vi.fn(),
  registerForDustGeneration: vi.fn(),
  updateService: vi.fn(),
  updateSchnorrJubjubVerificationMethod: vi.fn(),
  updateVerificationMethod: vi.fn(),
}));

vi.mock('@midnight-ntwrk/midnight-did-api', () => apiMock);
vi.mock('@midnight-ntwrk/midnight-did', () => ({
  LedgerToDomain: { toJSON: vi.fn((state) => ({ json: state })) },
  MidnightNetwork: {
    Undeployed: 'undeployed', DevNet: 'devnet', Testnet: 'testnet',
    Mainnet: 'mainnet', Preview: 'preview', Preprod: 'preprod',
  },
  parseContractAddress: vi.fn((address) => address),
}));
vi.mock('@midnight-ntwrk/midnight-did-domain', () => ({
  createService: vi.fn((input) => input),
}));
vi.mock('@midnight-ntwrk/midnight-did-secret-storage', () => ({
  normalizePublicForLedger: vi.fn((publicJwk) => ({ x: publicJwk.x, y: publicJwk.y })),
}));

import {
  addAlsoKnownAs,
  addRelation,
  addService,
  addVerificationMethod,
  buildNormalizedVerificationMethod,
  deactivateDid,
  deployDidContract,
  getDidDocument,
  getDidState,
  listStoredContracts,
  removeAlsoKnownAs,
  removeRelation,
  removeService,
  removeVerificationMethod,
  updateService,
  updateVerificationMethod,
} from '../manager/did-lifecycle-service.js';

const contract = {
  deployTxData: { public: { contractAddress: 'a'.repeat(64) } },
} as never;
const providers = { id: 'providers' } as never;
const profile = 'standalone' as const;

const method = (crv: 'Ed25519' | 'Jubjub') => ({
  id: `did:midnight:undeployed:${'a'.repeat(64)}#${crv}`,
  type: 'JsonWebKey' as const,
  controller: `did:midnight:undeployed:${'a'.repeat(64)}`,
  publicKeyJwk: { kty: crv === 'Jubjub' ? 'EC' : 'OKP', crv, x: 'x', y: 'y' },
});

const persist = vi.fn().mockResolvedValue(undefined);

describe('DID lifecycle service', () => {
  it('lists empty, locked, available, missing, and failed stored contracts', async () => {
    expect(await listStoredContracts({
      addresses: [], selectedAddress: null, unlocked: false, providers: null, profile,
    })).toEqual([]);

    const locked = await listStoredContracts({
      addresses: ['one', 'two'], selectedAddress: 'two', unlocked: false, providers: null, profile,
    });
    expect(locked[1]).toMatchObject({ address: 'two', selected: true, available: null });

    apiMock.getMidnightDIDLedgerState
      .mockResolvedValueOnce(null)
      .mockResolvedValueOnce({ deactivated: false, version: 3n, operationCount: 4n })
      .mockRejectedValueOnce(new Error('lookup failed'));
    const listed = await listStoredContracts({
      addresses: ['missing', 'available', 'failed'],
      selectedAddress: 'available', unlocked: true, providers, profile,
    });
    expect(listed).toEqual([
      expect.objectContaining({ address: 'missing', available: false }),
      expect.objectContaining({ address: 'available', available: true, version: 3, operationCount: 4 }),
      expect.objectContaining({ address: 'failed', available: null, message: 'lookup failed' }),
    ]);
  });

  it('deploys a DID after registering dust and persists the contract', async () => {
    const walletCtx = { wallet: { id: 'wallet' }, unshieldedKeystore: { id: 'keystore' } } as never;
    const privateState = { id: 'private-state' } as never;
    apiMock.initPrivateState.mockResolvedValue(privateState);
    apiMock.createDID.mockResolvedValue(contract);

    await expect(deployDidContract({
      logger: { info: vi.fn() } as never,
      walletCtx,
      providers,
      onDidContract: vi.fn(),
      onPersist: persist,
    })).resolves.toEqual({ contractAddress: 'a'.repeat(64) });
    expect(apiMock.registerForDustGeneration).toHaveBeenCalledWith(walletCtx.wallet, walletCtx.unshieldedKeystore);
    expect(apiMock.initPrivateState).toHaveBeenCalledWith(providers);
    expect(persist).toHaveBeenCalled();
  });

  it('reads DID state and resolves the DID document', async () => {
    apiMock.getMidnightDIDLedgerState.mockResolvedValue({ version: 1 });
    apiMock.resolve.mockResolvedValue({ didDocument: { id: 'did:midnight:undeployed:test' } });
    await expect(getDidState(providers, contract)).resolves.toEqual({
      contractAddress: 'a'.repeat(64), didState: { json: { version: 1 } },
    });
    await expect(getDidDocument(providers, contract)).resolves.toEqual({
      didDocument: { id: 'did:midnight:undeployed:test' },
    });
  });

  it('normalizes a key and delegates all verification-method operations', async () => {
    const secretStore = { getPublicKey: vi.fn().mockResolvedValue({ kty: 'EC', crv: 'Jubjub', x: 'x', y: 'y' }) } as never;
    await expect(buildNormalizedVerificationMethod(contract, secretStore, 'key', 'method', vi.fn((id) => ({ id })) as never))
      .resolves.toMatchObject({ didContract: contract, method: { id: 'method' } });

    await addVerificationMethod(contract, providers, method('Jubjub') as never, persist);
    await addVerificationMethod(contract, providers, method('Ed25519') as never, persist);
    await updateVerificationMethod(contract, providers, method('Jubjub') as never, persist);
    await updateVerificationMethod(contract, providers, method('Ed25519') as never, persist);
    expect(apiMock.addSchnorrJubjubVerificationMethod).toHaveBeenCalled();
    expect(apiMock.addVerificationMethod).toHaveBeenCalled();
    expect(apiMock.updateSchnorrJubjubVerificationMethod).toHaveBeenCalled();
    expect(apiMock.updateVerificationMethod).toHaveBeenCalled();

    apiMock.resolve
      .mockResolvedValueOnce({ didDocument: { verificationMethod: [method('Jubjub')] } })
      .mockResolvedValueOnce({ didDocument: { verificationMethod: [method('Ed25519')] } })
      .mockResolvedValueOnce({ didDocument: { verificationMethod: [] } });
    await removeVerificationMethod(contract, providers, 'Jubjub', persist);
    await removeVerificationMethod(contract, providers, 'Ed25519', persist);
    await removeVerificationMethod(contract, providers, 'missing', persist);
    expect(apiMock.removeSchnorrJubjubVerificationMethod).toHaveBeenCalled();
    expect(apiMock.removeVerificationMethod).toHaveBeenCalled();
  });

  it('delegates relations, services, aliases, and deactivation', async () => {
    persist.mockClear();
    await addRelation(contract, providers, 'authentication' as never, 'method', persist);
    await removeRelation(contract, providers, 'authentication' as never, 'method', persist);
    await addService(contract, providers, { id: 'svc', type: 'LinkedDomains', serviceEndpoint: 'https://example.com' } as never, persist);
    await updateService(contract, providers, { id: 'svc', type: 'LinkedDomains', serviceEndpoint: 'https://example.com' } as never, persist);
    await removeService(contract, providers, 'svc', persist);
    await addAlsoKnownAs(contract, providers, 'https://example.com', persist);
    await removeAlsoKnownAs(contract, providers, 'https://example.com', persist);
    await deactivateDid(contract, providers, persist);

    expect(apiMock.addVerificationMethodRelation).toHaveBeenCalled();
    expect(apiMock.removeVerificationMethodRelation).toHaveBeenCalled();
    expect(apiMock.addService).toHaveBeenCalled();
    expect(apiMock.updateService).toHaveBeenCalled();
    expect(apiMock.removeService).toHaveBeenCalled();
    expect(apiMock.addAlsoKnownAs).toHaveBeenCalled();
    expect(apiMock.removeAlsoKnownAs).toHaveBeenCalled();
    expect(apiMock.deactivate).toHaveBeenCalled();
    expect(persist).toHaveBeenCalledTimes(8);
  });
});

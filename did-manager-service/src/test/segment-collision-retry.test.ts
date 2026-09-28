import { beforeEach, describe, expect, it, vi } from 'vitest';

const apiMock = vi.hoisted(() => ({
  configureProviders: vi.fn(),
}));

vi.mock('@midnight-ntwrk/midnight-did-api', () => apiMock);

import {
  configureProvidersWithCollisionRetry,
  isSegmentCollision,
  maxBalanceAttempts,
  SegmentCollisionError,
  withCollisionRevert,
  withSegmentCollisionRetry,
} from '../manager/segment-collision-retry.js';

const collision = () => new Error('Error: key (segment_id) collision during intents merge: 60064');
const logger = { warn: vi.fn() } as never;

const createWallet = () => {
  const balancingTransaction = { id: 'fee-tx' };
  const wallet = {
    label: 'facade',
    balanceUnboundTransaction: vi.fn().mockResolvedValue({
      type: 'UNBOUND_TRANSACTION',
      baseTransaction: { id: 'base-tx' },
      balancingTransaction,
    }),
    finalizeRecipe: vi.fn(),
    revert: vi.fn().mockResolvedValue(undefined),
    describe() {
      return this.label;
    },
  };
  return { wallet, balancingTransaction };
};

// Mirrors midnight-did-api's wallet provider: balance through the facade, then finalize the recipe.
const configureProvidersLikeDidApi = async (ctx: { wallet: ReturnType<typeof createWallet>['wallet'] }) => {
  const walletProvider = {
    getCoinPublicKey: () => 'coin-public-key',
    async balanceTx(tx: unknown) {
      const recipe = await ctx.wallet.balanceUnboundTransaction(tx);
      return await ctx.wallet.finalizeRecipe(recipe);
    },
    submitTx: vi.fn(),
  };
  return { id: 'providers', walletProvider, midnightProvider: walletProvider };
};

describe('segment collision retry', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('recognizes only intent segment collisions', () => {
    expect(isSegmentCollision(collision())).toBe(true);
    expect(isSegmentCollision(new Error('Insufficient funds'))).toBe(false);
    expect(isSegmentCollision('key (segment_id) collision during intents merge')).toBe(false);
  });

  it('releases the fee transaction when finalizing collides', async () => {
    const { wallet, balancingTransaction } = createWallet();
    wallet.finalizeRecipe.mockRejectedValueOnce(collision());
    const recipe = await wallet.balanceUnboundTransaction({ id: 'call-tx' });

    const guarded = withCollisionRevert(wallet as never, logger);

    await expect(guarded.finalizeRecipe(recipe)).rejects.toBeInstanceOf(SegmentCollisionError);
    expect(wallet.revert).toHaveBeenCalledExactlyOnceWith(balancingTransaction);
    expect((guarded as unknown as typeof wallet).describe()).toBe('facade');
  });

  it('still reports the collision when releasing the fee transaction fails', async () => {
    const { wallet } = createWallet();
    wallet.finalizeRecipe.mockRejectedValueOnce(collision());
    wallet.revert.mockRejectedValueOnce(new Error('revert failed'));
    const warn = vi.fn();

    await expect(withCollisionRevert(wallet as never, { warn } as never).finalizeRecipe(
      await wallet.balanceUnboundTransaction({ id: 'call-tx' }),
    )).rejects.toBeInstanceOf(SegmentCollisionError);
    expect(warn).toHaveBeenCalledOnce();
  });

  it('passes other finalize errors through without reverting', async () => {
    const { wallet } = createWallet();
    const failure = new Error('prover unavailable');
    wallet.finalizeRecipe.mockRejectedValueOnce(failure);

    await expect(withCollisionRevert(wallet as never, logger).finalizeRecipe(
      await wallet.balanceUnboundTransaction({ id: 'call-tx' }),
    )).rejects.toBe(failure);
    expect(wallet.revert).not.toHaveBeenCalled();
  });

  it('stops rebalancing after the attempt limit', async () => {
    const balanceTx = vi.fn().mockRejectedValue(new SegmentCollisionError(collision()));
    const provider = withSegmentCollisionRetry({ balanceTx } as never, logger);

    await expect(provider.balanceTx({} as never)).rejects.toBeInstanceOf(SegmentCollisionError);
    expect(balanceTx).toHaveBeenCalledTimes(maxBalanceAttempts);
  });

  it('does not rebalance after other balancing errors', async () => {
    const failure = new Error('Insufficient funds');
    const balanceTx = vi.fn().mockRejectedValue(failure);
    const provider = withSegmentCollisionRetry({ balanceTx } as never, logger);

    await expect(provider.balanceTx({} as never)).rejects.toBe(failure);
    expect(balanceTx).toHaveBeenCalledOnce();
  });

  it('rebalances a colliding transaction through the configured providers', async () => {
    const { wallet, balancingTransaction } = createWallet();
    wallet.finalizeRecipe
      .mockRejectedValueOnce(collision())
      .mockResolvedValueOnce({ id: 'finalized-tx' });
    apiMock.configureProviders.mockImplementation(configureProvidersLikeDidApi);
    const walletCtx = { wallet, dustSecretKey: { id: 'dust-key' } };

    const providers = await configureProvidersWithCollisionRetry(walletCtx as never, { id: 'config' } as never, logger);

    expect(apiMock.configureProviders).toHaveBeenCalledExactlyOnceWith(
      expect.objectContaining({ dustSecretKey: walletCtx.dustSecretKey }),
      { id: 'config' },
    );
    expect(providers.midnightProvider).toBe(providers.walletProvider);
    expect(providers.walletProvider.getCoinPublicKey()).toBe('coin-public-key');
    await expect(providers.walletProvider.balanceTx({ id: 'call-tx' } as never)).resolves.toEqual({ id: 'finalized-tx' });
    expect(wallet.balanceUnboundTransaction).toHaveBeenCalledTimes(2);
    expect(wallet.revert).toHaveBeenCalledExactlyOnceWith(balancingTransaction);
  });
});

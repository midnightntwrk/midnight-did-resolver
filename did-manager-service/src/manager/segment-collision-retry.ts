import * as api from '@midnight-ntwrk/midnight-did-api';
import type { Logger } from 'pino';

type WalletFacade = api.MidnightDIDWalletContext['wallet'];
type BalancingRecipe = Parameters<WalletFacade['finalizeRecipe']>[0];
type WalletAndMidnightProvider = Awaited<ReturnType<typeof api.configureProviders>>['walletProvider'];

// midnight-js call transactions and wallet-sdk dust fee transactions each pick a random
// intent segment id, so merging them in `finalizeRecipe` occasionally fails with this error.
const segmentCollisionPattern = /segment_id\) collision during intents merge/;

export const maxBalanceAttempts = 3;

export class SegmentCollisionError extends Error {
  constructor(cause: unknown) {
    super(cause instanceof Error ? cause.message : String(cause), { cause });
    this.name = 'SegmentCollisionError';
  }
}

export const isSegmentCollision = (error: unknown): boolean =>
  error instanceof Error && segmentCollisionPattern.test(error.message);

const balancingTransactionOf = (recipe: BalancingRecipe) => {
  switch (recipe.type) {
    case 'FINALIZED_TRANSACTION':
    case 'UNBOUND_TRANSACTION':
      return recipe.balancingTransaction;
    default:
      return undefined;
  }
};

/**
 * Wraps the wallet so a colliding merge releases the dust spends reserved for the fee
 * transaction. The merge fails before submission, and the facade only reverts on a failed
 * submit, so without this the reserved dust stays pending for the rest of the session.
 * Only the balancing transaction is reverted: unshielded balancing of the base transaction
 * happens in place and is reused when the transaction is balanced again.
 */
export const withCollisionRevert = (wallet: WalletFacade, logger: Logger): WalletFacade => {
  const finalizeRecipe: WalletFacade['finalizeRecipe'] = async (recipe) => {
    try {
      return await wallet.finalizeRecipe(recipe);
    } catch (error) {
      if (!isSegmentCollision(error)) throw error;
      const balancingTransaction = balancingTransactionOf(recipe);
      if (balancingTransaction !== undefined) {
        try {
          await wallet.revert(balancingTransaction);
        } catch (revertError) {
          logger.warn({ err: revertError }, 'Failed to release dust reserved by colliding fee transaction');
        }
      }
      throw new SegmentCollisionError(error);
    }
  };

  return new Proxy(wallet, {
    get(target, property) {
      if (property === 'finalizeRecipe') return finalizeRecipe;
      const value: unknown = Reflect.get(target, property, target);
      return typeof value === 'function' ? value.bind(target) : value;
    },
  });
};

/** Rebalances a transaction whose fee transaction collided with it, picking a fresh segment id. */
export const withSegmentCollisionRetry = (
  provider: WalletAndMidnightProvider,
  logger: Logger,
): WalletAndMidnightProvider => ({
  ...provider,
  async balanceTx(tx, ttl) {
    for (let attempt = 1; ; attempt += 1) {
      try {
        return await provider.balanceTx(tx, ttl);
      } catch (error) {
        if (!(error instanceof SegmentCollisionError) || attempt >= maxBalanceAttempts) throw error;
        logger.warn({ attempt, err: error }, 'Fee transaction segment collided, rebalancing');
      }
    }
  },
});

export const configureProvidersWithCollisionRetry = async (
  walletCtx: api.MidnightDIDWalletContext,
  config: api.Config,
  logger: Logger,
): Promise<api.MidnightDIDProviders> => {
  const providers = await api.configureProviders(
    { ...walletCtx, wallet: withCollisionRevert(walletCtx.wallet, logger) },
    config,
  );
  const walletProvider = withSegmentCollisionRetry(providers.walletProvider, logger);
  return { ...providers, walletProvider, midnightProvider: walletProvider };
};

import {
  MidnightDIDResolver,
  MidnightNetwork,
} from "@midnight-ntwrk/midnight-did";
import { DIDContract } from "@midnight-ntwrk/midnight-did-contract";
import { indexerPublicDataProvider } from "@midnight-ntwrk/midnight-js-indexer-public-data-provider";
import {
  ChargedState,
  ContractState,
  StateValue,
} from "@midnight-ntwrk/onchain-runtime-v3";

import { didResolutionErrorPayload } from "./did-resolution-response.js";
import { IndexerEndpointPolicy } from "./indexer-endpoint-policy.js";
import {
  classifyResolutionError,
  type ResolutionErrorCode,
  statusCodeForResolutionError,
} from "./resolution-errors.js";
import { type ResolveRequestOptions } from "./types.js";

export type MidnightResolutionResult = NonNullable<
  Awaited<ReturnType<MidnightDIDResolver["resolveResult"]>>
>;

export type ResolveResponse =
  | {
      statusCode: 200;
      payload: MidnightResolutionResult & {
        didResolutionMetadata: {
          contentType: "application/did+ld+json";
          error: null;
        };
      };
    }
  | {
      statusCode: 400 | 404 | 500;
      payload: {
        didDocument: null;
        didDocumentMetadata: {};
        didResolutionMetadata: {
          contentType: null;
          error: ResolutionErrorCode;
        };
      };
    };

export type ResolverServiceOptions = {
  indexerHttpUrl: string;
  indexerWsUrl: string;
  expectedNetwork?: MidnightNetwork;
  debug?: boolean;
  logger?: ResolverLogger;
  resolveTimeoutMs?: number;
};

export const RESOLVER_CACHE_MAX_SIZE = 64;
const DEFAULT_RESOLVE_TIMEOUT_MS = 15_000;

export type ResolverLogger = {
  error: (message: string, context?: Record<string, unknown>) => void;
};

const defaultLogger: ResolverLogger = {
  error: (message, context) => {
    if (context === undefined) {
      console.error(message);
      return;
    }
    console.error(message, context);
  },
};

type ContractStateData = {
  readonly serialize: () => Uint8Array;
  readonly data?: unknown;
};
type HasData = { readonly data?: unknown };

const isChargedState = (value: unknown): value is ChargedState =>
  value != null &&
  typeof value === "object" &&
  "state" in value &&
  isStateValue((value as { state: unknown }).state);

const isStateValue = (value: unknown): value is StateValue => {
  return (
    value != null &&
    typeof value === "object" &&
    "type" in value &&
    typeof (value as { type: unknown }).type === "function" &&
    "encode" in value &&
    typeof (value as { encode: unknown }).encode === "function"
  );
};

const coerceToChargedState = (contractState: unknown): ChargedState => {
  if (isChargedState(contractState)) {
    return contractState;
  }
  const asContractState = contractState as ContractStateData;
  if (asContractState === null || asContractState === undefined) {
    throw new Error("Unable to deserialize contract state payload");
  }
  if (typeof asContractState.serialize !== "function") {
    throw new Error("Unable to deserialize contract state payload");
  }
  return ContractState.deserialize(asContractState.serialize()).data;
};

const normalizeContractState = (contractState: unknown): ChargedState => {
  if (isChargedState(contractState)) {
    return contractState;
  }
  if (contractState != null && typeof contractState === "object") {
    const nestedState = (contractState as HasData).data;
    if (isChargedState(nestedState)) {
      return nestedState;
    }
  }
  return coerceToChargedState(contractState);
};

export class ResolverService {
  private readonly expectedNetwork: MidnightNetwork | undefined;
  private readonly endpointPolicy: IndexerEndpointPolicy;
  private readonly debug: boolean;
  private readonly logger: ResolverLogger;
  private readonly resolveTimeoutMs: number;
  private readonly resolverCache = new Map<string, MidnightDIDResolver>();

  constructor(options: ResolverServiceOptions) {
    this.expectedNetwork = options.expectedNetwork;
    this.debug = options.debug ?? false;
    this.logger = options.logger ?? defaultLogger;
    this.resolveTimeoutMs =
      options.resolveTimeoutMs ?? DEFAULT_RESOLVE_TIMEOUT_MS;
    this.endpointPolicy = new IndexerEndpointPolicy({
      indexerHttpUrl: options.indexerHttpUrl,
      indexerWsUrl: options.indexerWsUrl,
    });
  }

  private logResolutionFailure(
    did: string,
    options: ResolveRequestOptions | undefined,
    errorCode: ResolutionErrorCode,
    error: unknown,
  ): void {
    if (!this.debug) return;
    const message =
      error instanceof Error ? error.message : "Unexpected resolve error";
    const stack = error instanceof Error ? error.stack : undefined;
    this.logger.error("[did-resolver-service] resolve failed", {
      did,
      options,
      errorCode,
      message,
      stack,
    });
  }

  private touchCache(cacheKey: string, resolver: MidnightDIDResolver): void {
    if (this.resolverCache.has(cacheKey)) {
      this.resolverCache.delete(cacheKey);
    }
    this.resolverCache.set(cacheKey, resolver);
    if (this.resolverCache.size <= RESOLVER_CACHE_MAX_SIZE) {
      return;
    }
    const oldestKey = this.resolverCache.keys().next().value;
    if (oldestKey !== undefined) {
      this.resolverCache.delete(oldestKey);
    }
  }

  private resolverFor(options?: ResolveRequestOptions): MidnightDIDResolver {
    const { indexerHttpUrl, indexerWsUrl } =
      this.endpointPolicy.resolve(options);
    const cacheKey = `${indexerHttpUrl}|${indexerWsUrl}|${this.expectedNetwork ?? "any"}`;
    const cached = this.resolverCache.get(cacheKey);
    if (cached !== undefined) {
      this.touchCache(cacheKey, cached);
      return cached;
    }
    const publicDataProvider = indexerPublicDataProvider(
      indexerHttpUrl,
      indexerWsUrl,
    );
    const resolver = new MidnightDIDResolver({
      expectedNetwork: this.expectedNetwork,
      ledgerReader: async (contractAddress) => {
        const contractState =
          await publicDataProvider.queryContractState(contractAddress);
        return contractState === null
          ? null
          : DIDContract.ledger(normalizeContractState(contractState));
      },
    });
    this.touchCache(cacheKey, resolver);
    return resolver;
  }

  private async resolveResultWithTimeout(
    did: string,
    options?: ResolveRequestOptions,
  ): Promise<Awaited<ReturnType<MidnightDIDResolver["resolveResult"]>>> {
    const abortController = new AbortController();
    let timer: ReturnType<typeof globalThis.setTimeout> | null = null;
    const timeout = new Promise<never>((_, reject) => {
      timer = globalThis.setTimeout(() => {
        abortController.abort();
        reject(
          new Error(
            `Resolution timed out after ${this.resolveTimeoutMs}ms for DID: ${did}`,
          ),
        );
      }, this.resolveTimeoutMs);
    });

    try {
      return await Promise.race([
        this.resolverFor(options).resolveResult(did),
        timeout,
      ]);
    } finally {
      if (timer !== null) {
        globalThis.clearTimeout(timer);
      }
      abortController.abort();
    }
  }

  async resolve(
    did: string,
    options?: ResolveRequestOptions,
  ): Promise<ResolveResponse> {
    try {
      const result = await this.resolveResultWithTimeout(did, options);
      if (result === null) {
        return {
          statusCode: 404,
          payload: didResolutionErrorPayload("notFound"),
        };
      }

      return {
        statusCode: 200,
        payload: {
          ...result,
          didResolutionMetadata: {
            contentType: "application/did+ld+json",
            error: null,
          },
        },
      };
    } catch (error) {
      const resolveError = classifyResolutionError(error);
      this.logResolutionFailure(did, options, resolveError, error);
      const statusCode = statusCodeForResolutionError(resolveError);
      return {
        statusCode,
        payload: didResolutionErrorPayload(resolveError),
      };
    }
  }
}

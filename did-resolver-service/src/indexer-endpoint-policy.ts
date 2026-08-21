export type IndexerEndpoints = {
  indexerHttpUrl: string;
  indexerWsUrl: string;
};

/**
 * Resolves the immutable indexer pair configured at service startup.
 *
 * Request-level endpoint selection is deliberately not supported. This keeps
 * the resolver's outbound network boundary operator-controlled and prevents
 * callers from turning the service into an SSRF proxy.
 */
export class IndexerEndpointPolicy {
  constructor(private readonly defaultEndpoints: IndexerEndpoints) {}

  resolve(): IndexerEndpoints {
    return { ...this.defaultEndpoints };
  }
}

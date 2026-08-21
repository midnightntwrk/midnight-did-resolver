import { describe, expect, it } from "vitest";

import { IndexerEndpointPolicy } from "../indexer-endpoint-policy";

describe("did-resolver-service indexer endpoint policy", () => {
  it("returns the immutable startup endpoint pair", () => {
    const policy = new IndexerEndpointPolicy({
      indexerHttpUrl: "http://default.example/api/v3/graphql",
      indexerWsUrl: "ws://default.example/api/v3/graphql/ws",
    });

    expect(policy.resolve()).toEqual({
      indexerHttpUrl: "http://default.example/api/v3/graphql",
      indexerWsUrl: "ws://default.example/api/v3/graphql/ws",
    });
  });

  it("returns a copy so callers cannot mutate configured endpoints", () => {
    const policy = new IndexerEndpointPolicy({
      indexerHttpUrl: "https://indexer.example/graphql",
      indexerWsUrl: "wss://indexer.example/graphql/ws",
    });

    const resolved = policy.resolve();
    resolved.indexerHttpUrl = "https://attacker.example/graphql";

    expect(policy.resolve().indexerHttpUrl).toBe(
      "https://indexer.example/graphql",
    );
  });
});

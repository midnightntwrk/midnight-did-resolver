import { isIP } from "node:net";

import { type ResolveRequestOptions } from "./types.js";

export type IndexerEndpoints = {
  indexerHttpUrl: string;
  indexerWsUrl: string;
};

const privateHostMessage = (field: string): string =>
  `${field} must not target localhost, private, link-local, or otherwise non-public hosts`;

const parseIpv4 = (value: string): number[] | null => {
  const parts = value.split(".");
  if (parts.length !== 4) return null;
  const parsed = parts.map((part) => Number(part));
  if (
    !parsed.every((part) => Number.isInteger(part) && part >= 0 && part <= 255)
  ) {
    return null;
  }
  return parsed;
};

const isPrivateIpv4 = (value: string): boolean => {
  const parts = parseIpv4(value);
  if (parts === null) return false;
  const [a, b] = parts;
  return (
    a === 0 ||
    a === 10 ||
    a === 127 ||
    (a === 100 && b >= 64 && b <= 127) ||
    (a === 169 && b === 254) ||
    (a === 172 && b >= 16 && b <= 31) ||
    (a === 192 && b === 168) ||
    (a === 198 && (b === 18 || b === 19)) ||
    a >= 224
  );
};

const isPrivateIpv6 = (value: string): boolean => {
  const normalized = value.toLowerCase();
  const mappedIpv4 = normalized.match(/^::ffff:(\d+\.\d+\.\d+\.\d+)$/);
  if (mappedIpv4 !== null) return isPrivateIpv4(mappedIpv4[1]);
  return (
    normalized === "::" ||
    normalized === "::1" ||
    normalized.startsWith("fe80:") ||
    normalized.startsWith("fc") ||
    normalized.startsWith("fd")
  );
};

const isPrivateHost = (hostname: string): boolean => {
  const normalized = hostname
    .toLowerCase()
    .replace(/^\[|\]$/g, "")
    .replace(/\.$/, "");
  if (normalized === "localhost" || normalized.endsWith(".localhost"))
    return true;
  const ipVersion = isIP(normalized);
  if (ipVersion === 4) return isPrivateIpv4(normalized);
  if (ipVersion === 6) return isPrivateIpv6(normalized);
  return false;
};

export class IndexerEndpointPolicy {
  private readonly defaultIndexerHttpUrl: string;
  private readonly defaultIndexerWsUrl: string;

  constructor(defaultEndpoints: IndexerEndpoints) {
    this.defaultIndexerHttpUrl = defaultEndpoints.indexerHttpUrl;
    this.defaultIndexerWsUrl = defaultEndpoints.indexerWsUrl;
  }

  private static normalizeUrl(
    value: string,
    protocols: readonly string[],
    protocolMessage: string,
    field: "indexerUrl" | "indexerWsUrl",
  ): string {
    const trimmed = value.trim();
    const parsed = new URL(trimmed);
    if (!protocols.includes(parsed.protocol)) {
      throw new Error(protocolMessage);
    }
    if (parsed.username !== "" || parsed.password !== "") {
      throw new Error(`${field} must not include credentials`);
    }
    if (isPrivateHost(parsed.hostname)) {
      throw new Error(privateHostMessage(field));
    }
    return parsed.toString().replace(/\/+$/, "");
  }

  private static normalizeIndexerHttpUrl(value: string): string {
    return IndexerEndpointPolicy.normalizeUrl(
      value,
      ["http:", "https:"],
      "indexerUrl must use http or https",
      "indexerUrl",
    );
  }

  private static normalizeIndexerWsUrl(value: string): string {
    return IndexerEndpointPolicy.normalizeUrl(
      value,
      ["ws:", "wss:"],
      "indexerWsUrl must use ws or wss",
      "indexerWsUrl",
    );
  }

  private static deriveWsUrl(indexerHttpUrl: string): string {
    const parsed = new URL(indexerHttpUrl);
    parsed.protocol = parsed.protocol === "https:" ? "wss:" : "ws:";
    if (parsed.pathname.endsWith("/graphql")) {
      parsed.pathname = `${parsed.pathname}/ws`;
    }
    return parsed.toString().replace(/\/+$/, "");
  }

  resolve(options?: ResolveRequestOptions): IndexerEndpoints {
    const indexerHttpUrl =
      options?.indexerUrl !== undefined
        ? IndexerEndpointPolicy.normalizeIndexerHttpUrl(options.indexerUrl)
        : this.defaultIndexerHttpUrl;
    const indexerWsUrl =
      options?.indexerWsUrl !== undefined
        ? IndexerEndpointPolicy.normalizeIndexerWsUrl(options.indexerWsUrl)
        : options?.indexerUrl !== undefined
          ? IndexerEndpointPolicy.deriveWsUrl(indexerHttpUrl)
          : this.defaultIndexerWsUrl;
    return { indexerHttpUrl, indexerWsUrl };
  }
}

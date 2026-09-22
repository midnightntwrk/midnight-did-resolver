import {
  decodeJubjubJwkCoordinate,
  encodeJubjubJwkCoordinate,
} from "@midnight-ntwrk/midnight-did-domain";
import { describe, expect, it } from "vitest";

describe("Midnight DID 0.7 compatibility", () => {
  it("uses canonical fixed-width big-endian Jubjub JWK coordinates", () => {
    const encoded = encodeJubjubJwkCoordinate(0x0102n);

    expect(encoded).toBe("AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAQI");
    expect(decodeJubjubJwkCoordinate(encoded)).toBe(0x0102n);
  });
});

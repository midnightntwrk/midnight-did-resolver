import { describe, test, expect } from 'vitest';
import * as api from '@midnight-ntwrk/midnight-did-api';
import * as did from '@midnight-ntwrk/midnight-did';
import * as domain from '@midnight-ntwrk/midnight-did-domain';
import { createTestDID, resolveDID } from '../setup';

export function verificationMethodTests() {
  describe('Verification Methods', () => {
    describe('Multiple Verification Methods with Different Key Types', () => {
      test.skip('should serialize all supported key types', async () => {
        const { didContract, didStr } = await createTestDID();
        await api.update(didContract, [
          {
            type: did.DIDOperationType.AddVerificationMethod,
            verificationMethod: {
              id: 'key-ed25519-1' as domain.DIDKeyID,
              type: domain.VerificationMethodType.JsonWebKey,
              controller: didStr as domain.DIDString,
              publicKeyJwk: {
                kty: domain.KeyType.OKP,
                crv: domain.CurveType.Ed25519,
                x: 'VCpo2LMLhn6iWku8MKvSLg2ZAoC-nlOyPVQaO3FxVeQ',
              },
            },
          },
          {
            type: did.DIDOperationType.AddVerificationMethod,
            verificationMethod: {
              id: 'key-jubjub' as domain.DIDKeyID,
              type: domain.VerificationMethodType.JsonWebKey,
              controller: didStr as domain.DIDString,
              publicKeyJwk: {
                kty: domain.KeyType.EC,
                crv: domain.CurveType.Jubjub,
                x: '3045022100f8c1e4a2d3b5c6e7f8a9b0c1d2e3f4a5b6c7d8e9f0a1b2c3d4e5f6a7b8c9',
                y: '00ab5910f4832a6b9c8d7e6f5a4b3c2d1e0f9a8b7c6d5e4f3a2b1c0d9e8f7a6b',
              },
            },
          },
          {
            type: did.DIDOperationType.AddVerificationMethod,
            verificationMethod: {
              id: 'key-ed25519-2' as domain.DIDKeyID,
              type: domain.VerificationMethodType.JsonWebKey,
              controller: didStr as domain.DIDString,
              publicKeyJwk: {
                kty: domain.KeyType.OKP,
                crv: domain.CurveType.Ed25519,
                x: 'abcdefghijklmnopqrstuvwxyz012345ABCDEFGHIJKLMNOP',
              },
            },
          },
        ]);
        const result = await resolveDID(didStr);

        // Verify successful resolution
        expect(result.didResolutionMetadata.error).toBeNull();
        expect(result.didDocument.verificationMethod).toHaveLength(3);

        // Verify all three methods are present with correct IDs
        const vmIds = result.didDocument.verificationMethod.map((vm: any) => vm.id);
        expect(vmIds).toContain(`${didStr}#key-ed25519-1`);
        expect(vmIds).toContain(`${didStr}#key-jubjub`);
        expect(vmIds).toContain(`${didStr}#key-ed25519-2`);

        // Verify Ed25519 keys by ID with exact value assertions
        const ed25519Key1 = result.didDocument.verificationMethod.find(
          (vm: any) => vm.id === `${didStr}#key-ed25519-1`
        );
        expect(ed25519Key1).toBeDefined();
        expect(ed25519Key1.type).toBe('JsonWebKey');
        expect(ed25519Key1.controller).toBe(didStr);
        expect(ed25519Key1.publicKeyJwk.kty).toBe('OKP');
        expect(ed25519Key1.publicKeyJwk.crv).toBe('Ed25519');
        expect(ed25519Key1.publicKeyJwk.x).toBe('VCpo2LMLhn6iWku8MKvSLg2ZAoC-nlOyPVQaO3FxVeQ');

        const ed25519Key2 = result.didDocument.verificationMethod.find(
          (vm: any) => vm.id === `${didStr}#key-ed25519-2`
        );
        expect(ed25519Key2).toBeDefined();
        expect(ed25519Key2.type).toBe('JsonWebKey');
        expect(ed25519Key2.controller).toBe(didStr);
        expect(ed25519Key2.publicKeyJwk.kty).toBe('OKP');
        expect(ed25519Key2.publicKeyJwk.crv).toBe('Ed25519');
        expect(ed25519Key2.publicKeyJwk.x).toBe('abcdefghijklmnopqrstuvwxyz012345ABCDEFGHIJKLMNOP');

        // Verify JubJub key by ID with exact value assertions for x and y
        const jubjubKey = result.didDocument.verificationMethod.find(
          (vm: any) => vm.id === `${didStr}#key-jubjub`
        );
        expect(jubjubKey).toBeDefined();
        expect(jubjubKey.type).toBe('JsonWebKey');
        expect(jubjubKey.controller).toBe(didStr);
        expect(jubjubKey.publicKeyJwk.kty).toBe('EC');
        expect(jubjubKey.publicKeyJwk.crv).toBe('Jubjub');
        expect(jubjubKey.publicKeyJwk.x).toBe(
          '3045022100f8c1e4a2d3b5c6e7f8a9b0c1d2e3f4a5b6c7d8e9f0a1b2c3d4e5f6a7b8c9'
        );
        expect(jubjubKey.publicKeyJwk.y).toBe(
          '00ab5910f4832a6b9c8d7e6f5a4b3c2d1e0f9a8b7c6d5e4f3a2b1c0d9e8f7a6b'
        );
      });
    });
  });
}

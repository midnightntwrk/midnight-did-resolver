import { describe, test, expect } from 'vitest';
import * as api from '@midnight-ntwrk/midnight-did-api';
import * as did from '@midnight-ntwrk/midnight-did';
import * as domain from '@midnight-ntwrk/midnight-did-domain';
import { createTestDID, resolveDID } from '../setup';

export function verificationMethodTests() {
  describe('Verification Methods', () => {
    test('should serialize all supported key types', async () => {
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
              x: 'QgA=',
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
              x: 'QgAB',
              y: 'QgAC',
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
              x: 'QgAAAQ==',
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
      expect(ed25519Key1.publicKeyJwk.x).toBe('QgA');

      const ed25519Key2 = result.didDocument.verificationMethod.find(
        (vm: any) => vm.id === `${didStr}#key-ed25519-2`
      );
      expect(ed25519Key2).toBeDefined();
      expect(ed25519Key2.type).toBe('JsonWebKey');
      expect(ed25519Key2.controller).toBe(didStr);
      expect(ed25519Key2.publicKeyJwk.kty).toBe('OKP');
      expect(ed25519Key2.publicKeyJwk.crv).toBe('Ed25519');
      expect(ed25519Key2.publicKeyJwk.x).toBe('QgAAAQ');

      // Verify JubJub key by ID with exact value assertions for x and y
      const jubjubKey = result.didDocument.verificationMethod.find(
        (vm: any) => vm.id === `${didStr}#key-jubjub`
      );
      expect(jubjubKey).toBeDefined();
      expect(jubjubKey.type).toBe('JsonWebKey');
      expect(jubjubKey.controller).toBe(didStr);
      expect(jubjubKey.publicKeyJwk.kty).toBe('EC');
      expect(jubjubKey.publicKeyJwk.crv).toBe('Jubjub');
      expect(jubjubKey.publicKeyJwk.x).toBe('QgAB');
      expect(jubjubKey.publicKeyJwk.y).toBe('QgAC');
    });
  });

  describe('Verification Relationships', () => {
    test('should serialize verification relationships', async () => {
      const { didContract, didStr } = await createTestDID();

      await api.update(didContract, [
        {
          type: did.DIDOperationType.AddVerificationMethod,
          verificationMethod: {
            id: 'key-multi' as domain.DIDKeyID,
            type: domain.VerificationMethodType.JsonWebKey,
            controller: didStr as domain.DIDString,
            publicKeyJwk: {
              kty: domain.KeyType.OKP,
              crv: domain.CurveType.Ed25519,
              x: 'VCpo2LMLhn6iWku8MKvSLg2ZAoC-nlOyPVQaO3FxVeQ',
            },
          },
        },
      ]);
      await api.update(didContract, [
        {
          type: did.DIDOperationType.AddVerificationMethodRelation,
          relation: domain.VerificationMethodRelationType.Authentication,
          methodId: 'key-multi' as domain.DIDKeyID,
        },
        {
          type: did.DIDOperationType.AddVerificationMethodRelation,
          relation: domain.VerificationMethodRelationType.AssertionMethod,
          methodId: 'key-multi' as domain.DIDKeyID,
        },
        {
          type: did.DIDOperationType.AddVerificationMethodRelation,
          relation: domain.VerificationMethodRelationType.CapabilityInvocation,
          methodId: 'key-multi' as domain.DIDKeyID,
        },
      ]);

      const result = await resolveDID(didStr);
      const expectedKeyRef = `${didStr}#key-multi`;

      // Verify successful resolution
      expect(result.didResolutionMetadata.error).toBeNull();

      // Verify verification method exists
      expect(result.didDocument.verificationMethod).toHaveLength(1);
      expect(result.didDocument.verificationMethod[0].id).toBe(expectedKeyRef);
      expect(result.didDocument.verificationMethod[0].type).toBe('JsonWebKey');
      expect(result.didDocument.verificationMethod[0].controller).toBe(didStr);

      // Verify the same key appears in multiple verification relationships
      expect(result.didDocument.authentication).toContain(expectedKeyRef);
      expect(result.didDocument.assertionMethod).toContain(expectedKeyRef);
      expect(result.didDocument.capabilityInvocation).toContain(expectedKeyRef);

      // Verify other relationships remain empty
      expect(result.didDocument.keyAgreement).toEqual([]);
      expect(result.didDocument.capabilityDelegation).toEqual([]);
    });
  });
}

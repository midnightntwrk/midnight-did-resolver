import { describe, test, expect } from 'vitest';
import * as api from '@midnight-ntwrk/midnight-did-api';
import * as did from '@midnight-ntwrk/midnight-did';
import { createTestDID, resolveDID } from '../setup';

export function basicResolutionTests() {
  describe('Empty DID Document', () => {
    test('should resolve newly created empty DID document', async () => {
      const { didStr } = await createTestDID();
      const resolutionResult = await resolveDID(didStr);

      // Verify successful resolution
      expect(resolutionResult).toBeDefined();
      expect(resolutionResult.didResolutionMetadata).toBeDefined();
      expect(resolutionResult.didResolutionMetadata.error).toBeNull();

      // Verify DID Document structure
      const didDocument = resolutionResult.didDocument;
      expect(didDocument).toBeDefined();
      expect(didDocument['@context']).toEqual([
        'https://www.w3.org/ns/did/v1',
        'https://w3c.github.io/vc-jws-2020/contexts/v1',
      ]);
      expect(didDocument.id).toBe(didStr);
      expect(didDocument.alsoKnownAs).toEqual([]);
      expect(didDocument.verificationMethod).toEqual([]);
      expect(didDocument.authentication).toEqual([]);
      expect(didDocument.assertionMethod).toEqual([]);
      expect(didDocument.keyAgreement).toEqual([]);
      expect(didDocument.capabilityInvocation).toEqual([]);
      expect(didDocument.capabilityDelegation).toEqual([]);
      expect(didDocument.service).toEqual([]);

      // Verify metadata
      const metadata = resolutionResult.didDocumentMetadata;
      expect(metadata).toBeDefined();
      expect(metadata.created).toBeDefined();
      expect(metadata.created).toMatch(/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}Z$/);
      expect(metadata.updated).toBeDefined();
      expect(metadata.updated).toMatch(/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}Z$/);
      expect(metadata.deactivated).toBeDefined();
      expect(metadata.deactivated).toBe(false);
      expect(metadata.versionId).toBeDefined();
      expect(metadata.versionId).toBe('0');
    });
  });

  describe('DID Document Metadata', () => {
    test('should increment versionId with each DID document update', async () => {
      const { didContract, didStr } = await createTestDID();
      let resolutionResult = await resolveDID(didStr);
      expect(resolutionResult.didDocumentMetadata.versionId).toBe('0');

      // First update
      await api.update(didContract, [
        {
          type: did.DIDOperationType.AddAlsoKnownAs,
          aliasUri: 'did:example:alias1',
        },
      ]);
      resolutionResult = await resolveDID(didStr);
      expect(resolutionResult.didDocumentMetadata.versionId).toBe('1');
      expect(resolutionResult.didDocument.alsoKnownAs).toEqual(['did:example:alias1']);

      // Second update
      await api.update(didContract, [
        {
          type: did.DIDOperationType.AddAlsoKnownAs,
          aliasUri: 'did:example:alias2',
        },
      ]);
      resolutionResult = await resolveDID(didStr);
      expect(resolutionResult.didDocumentMetadata.versionId).toBe('2');
      expect(resolutionResult.didDocument.alsoKnownAs).toEqual([
        'did:example:alias1',
        'did:example:alias2',
      ]);
      expect(resolutionResult.didResolutionMetadata.error).toBeNull();
    });
  });
}

import { describe, test, expect } from 'vitest';
import * as api from '@midnight-ntwrk/midnight-did-api';
import * as did from '@midnight-ntwrk/midnight-did';
import * as domain from '@midnight-ntwrk/midnight-did-domain';
import { createTestDID, resolveDID } from '../setup';

export function serviceEndpointTests() {
  describe('Service Endpoints', () => {
    test('should handle all service endpoint type variations', async () => {
      const { didContract, didStr } = await createTestDID();
      await api.update(didContract, [
        // Simple service endpoint (string) - simpleService
        {
          type: did.DIDOperationType.AddService,
          service: {
            id: 'service-1' as domain.ServiceId,
            type: 'DIDCommV2',
            serviceEndpoint: 'https://example.com/didcomm',
          },
        },
        // Complex service endpoint (object) - complexService
        {
          type: did.DIDOperationType.AddService,
          service: {
            id: 'didcomm-1' as domain.ServiceId,
            type: 'DIDCommV2',
            serviceEndpoint: {
              uri: 'https://example.com/didcomm',
              accept: ['didcomm/v2'],
              routingKeys: ['did:example:mediator#key-1'],
            },
          },
        },
      ]);
      await api.update(didContract, [
        // Service endpoint array with mixed types - arrayService
        {
          type: did.DIDOperationType.AddService,
          service: {
            id: 'service-array' as domain.ServiceId,
            type: ['DIDCommV2'],
            serviceEndpoint: [
              'https://example.com/endpoint1',
              {
                uri: 'wss://example.com/endpoint2',
                routingKeys: ['did:example:mediator'],
              },
            ],
          },
        },
      ]);

      const result = await resolveDID(didStr);

      // Verify successful resolution
      expect(result.didResolutionMetadata.error).toBeNull();
      expect(result.didDocument.service).toBeDefined();
      expect(result.didDocument.service).toHaveLength(3);

      // Verify simple service endpoint (string)
      const simpleService = result.didDocument.service.find(
        (s: any) => s.id === `${didStr}#service-1`
      );
      expect(simpleService).toBeDefined();
      expect(simpleService.id).toBe(`${didStr}#service-1`);
      expect(simpleService.type).toBe('DIDCommV2');
      expect(simpleService.serviceEndpoint).toBe('https://example.com/didcomm');

      // Verify complex service endpoint (object)
      const complexService = result.didDocument.service.find(
        (s: any) => s.id === `${didStr}#didcomm-1`
      );
      expect(complexService).toBeDefined();
      expect(complexService.id).toBe(`${didStr}#didcomm-1`);
      expect(complexService.type).toBe('DIDCommV2');
      expect(complexService.serviceEndpoint).toBeDefined();
      expect(typeof complexService.serviceEndpoint).toBe('object');
      expect(complexService.serviceEndpoint.uri).toBe('https://example.com/didcomm');
      expect(complexService.serviceEndpoint.accept).toEqual(['didcomm/v2']);
      expect(complexService.serviceEndpoint.routingKeys).toEqual(['did:example:mediator#key-1']);

      // Verify service endpoint array with mixed types
      const arrayService = result.didDocument.service.find(
        (s: any) => s.id === `${didStr}#service-array`
      );
      expect(arrayService).toBeDefined();
      expect(arrayService.id).toBe(`${didStr}#service-array`);
      expect(Array.isArray(arrayService.type)).toBe(true);
      expect(arrayService.type).toEqual(['DIDCommV2']);
      expect(Array.isArray(arrayService.serviceEndpoint)).toBe(true);
      expect(arrayService.serviceEndpoint).toHaveLength(2);
      expect(arrayService.serviceEndpoint[0]).toBe('https://example.com/endpoint1');
      expect(typeof arrayService.serviceEndpoint[1]).toBe('object');
      expect(arrayService.serviceEndpoint[1].uri).toBe('wss://example.com/endpoint2');
      expect(arrayService.serviceEndpoint[1].routingKeys).toEqual(['did:example:mediator']);

      // Verify all service IDs are unique and properly formatted
      const serviceIds: string[] = result.didDocument.service.map((s: any) => s.id);
      const uniqueIds = new Set<string>(serviceIds);
      expect(uniqueIds.size).toBe(3);
      serviceIds.forEach((id: string) => {
        expect(id).toMatch(new RegExp(`^${didStr}#.+`));
      });
      expect(serviceIds).toContain(`${didStr}#service-1`);
      expect(serviceIds).toContain(`${didStr}#didcomm-1`);
      expect(serviceIds).toContain(`${didStr}#service-array`);
    });
  });
}

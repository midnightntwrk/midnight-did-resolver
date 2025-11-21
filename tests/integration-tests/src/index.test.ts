import { describe, beforeAll } from 'vitest';
import { setupOnce } from './setup';
import { basicResolutionTests } from './scenarios/basic-resolution';
import { verificationMethodTests } from './scenarios/verification-methods';
import { serviceEndpointTests } from './scenarios/service-endpoints';

describe('Midnight DID Resolver - Integration Tests', () => {
  beforeAll(async () => {
    await setupOnce();
  });

  basicResolutionTests();
  verificationMethodTests();
  serviceEndpointTests();
});

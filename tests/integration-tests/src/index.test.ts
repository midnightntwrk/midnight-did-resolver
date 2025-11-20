import { describe, beforeAll } from 'vitest';
import { setupOnce } from './setup';
import { basicResolutionTests } from './scenarios/basic-resolution';
import { verificationMethodTests } from './scenarios/verification-methods';

/**
 * Midnight DID Resolver - Integration Tests
 *
 * Single entry point for all integration tests.
 * This ensures that the expensive setup (wallet initialization, network connection)
 * is executed only once via a single top-level beforeAll hook.
 *
 * All test scenarios are imported and executed within this context.
 */
describe('Midnight DID Resolver - Integration Tests', () => {
  beforeAll(async () => {
    await setupOnce();
  });

  // basicResolutionTests();
  verificationMethodTests();
});

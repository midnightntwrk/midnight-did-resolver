# Integration Test Implementation Plan

## Overview

This document outlines the implementation plan for the Midnight DID Resolver integration tests based on the scenarios defined in `tmp/scenarios.md`.

## Test Structure (Simplified Approach)

```
tests/integration-tests/
├── src/
│   ├── helpers/
│   │   ├── setup.ts                          # Test environment setup
│   │   ├── resolver.ts                       # DID resolution utilities
│   │   ├── contract-operations.ts            # Contract operation builders
│   │   ├── assertions.ts                     # Custom assertions
│   │   └── fixtures.ts                       # Test data fixtures
│   │
│   ├── 01-basic-resolution.test.ts           # Section 1 (2 scenarios)
│   ├── 02-verification-methods.test.ts       # Section 2 (4 scenarios)
│   ├── 03-verification-relationships.test.ts # Section 3 (6 scenarios)
│   ├── 04-service-endpoints.test.ts          # Section 4 (5 scenarios)
│   ├── 05-also-known-as.test.ts              # Section 5 (3 scenarios)
│   ├── 06-metadata.test.ts                   # Section 6 (4 scenarios)
│   ├── 07-deactivation.test.ts               # Section 7 (2 scenarios)
│   ├── 08-edge-cases.test.ts                 # Section 8 (7 scenarios)
│   └── 09-error-handling.test.ts             # Section 9 (6 scenarios)
│
├── package.json
├── tsconfig.json
├── vitest.config.ts
├── compose.yml
└── .env.example
```

## Design Principles

### 1. Modular Organization by Feature Category
- Each major section from scenarios.md becomes a single test file
- Tests are numbered (01-09) to match documentation sections
- Each file contains 2-7 test scenarios (manageable file size: 150-300 lines)

### 2. Shared Helpers (`helpers/` directory)
All reusable code lives in the helpers directory to eliminate duplication and improve maintainability.

### 3. Clear Test Naming Convention
- Files: `NN-category-name.test.ts` (e.g., `02-verification-methods.test.ts`)
- Describe blocks: Match scenario numbers (e.g., "2.1 Ed25519 Verification Method Serialization")
- Tests: Descriptive of the specific behavior being tested

## Helper Modules Specification

### `helpers/setup.ts`
**Purpose**: Centralize test environment initialization and common setup tasks.

```typescript
import * as api from '@midnight-ntwrk/midnight-did-api';
import * as did from '@midnight-ntwrk/midnight-did';

export interface TestEnvironment {
  wallet: Awaited<ReturnType<typeof api.buildWalletAndWaitForFunds>>;
  providers: Awaited<ReturnType<typeof api.configureProviders>>;
  config: api.StandaloneConfig;
  logger: any;
  resolverUrl: string;
}

/**
 * Initialize the test environment with wallet, providers, and configuration.
 * This should be called once in beforeAll hooks.
 */
export async function setupTestEnvironment(): Promise<TestEnvironment>;

/**
 * Create a new empty DID contract.
 * Returns the contract instance and DID string.
 */
export async function createEmptyDID(testEnv: TestEnvironment): Promise<{
  contract: any;
  didString: string;
  contractAddress: string;
  privateState: any;
}>;

/**
 * Create a DID and wait for it to be indexed.
 * Useful for tests that need to immediately resolve the DID.
 */
export async function createAndWaitForDID(
  testEnv: TestEnvironment,
  maxWaitMs?: number
): Promise<{ contract: any; didString: string }>;
```

### `helpers/resolver.ts`
**Purpose**: DID resolution utilities and related helper functions.

```typescript
/**
 * Resolve a DID using the resolver endpoint.
 * Returns the full DID Resolution Result.
 */
export async function resolveDID(
  didStr: string,
  resolverUrl?: string
): Promise<{
  didResolutionMetadata: any;
  didDocument: any;
  didDocumentMetadata: any;
}>;

/**
 * Resolve a DID and assert that it resolves successfully.
 * Throws if resolution fails.
 */
export async function resolveAndAssertSuccess(
  didStr: string,
  resolverUrl?: string
): Promise<{ didDocument: any; metadata: any }>;

/**
 * Extract the contract address from a DID string.
 */
export function extractContractAddress(didStr: string): string;

/**
 * Parse a DID string into its components (method, network, address).
 */
export function parseDIDString(didStr: string): {
  method: string;
  network: string;
  address: string;
};
```

### `helpers/contract-operations.ts`
**Purpose**: Builder functions for DID contract operations.

```typescript
/**
 * Build an AddVerificationMethod operation.
 */
export function buildAddVerificationMethod(
  id: string,
  publicKeyJwk: object
): any;

/**
 * Build an AddService operation.
 */
export function buildAddService(
  id: string,
  type: string,
  serviceEndpoint: string | object | any[]
): any;

/**
 * Build an AddVerificationMethodRelation operation.
 */
export function buildAddVerificationMethodRelation(
  methodId: string,
  relation: 'Authentication' | 'AssertionMethod' | 'KeyAgreement' | 'CapabilityInvocation' | 'CapabilityDelegation'
): any;

/**
 * Build an AddAlsoKnownAs operation.
 */
export function buildAddAlsoKnownAs(alias: string): any;

/**
 * Build a RemoveVerificationMethod operation.
 */
export function buildRemoveVerificationMethod(methodId: string): any;

/**
 * Build a RemoveService operation.
 */
export function buildRemoveService(serviceId: string): any;

/**
 * Build a Deactivate operation.
 */
export function buildDeactivate(): any;

/**
 * Execute a contract operation and wait for it to be processed.
 */
export async function executeOperation(
  contract: any,
  operation: any,
  testEnv: any
): Promise<void>;
```

### `helpers/assertions.ts`
**Purpose**: Custom assertion helpers for DID documents and metadata.

```typescript
import { expect } from 'vitest';

/**
 * Assert that a DID document has the required structure and fields.
 */
export function assertValidDIDDocument(didDocument: any): void;

/**
 * Assert that DID document metadata has the required structure.
 */
export function assertValidMetadata(metadata: any): void;

/**
 * Assert that a verification method has the correct structure.
 */
export function assertValidVerificationMethod(
  method: any,
  expectedId: string,
  expectedController: string
): void;

/**
 * Assert that a service has the correct structure.
 */
export function assertValidService(
  service: any,
  expectedId: string,
  expectedType: string
): void;

/**
 * Assert that a timestamp is in valid ISO 8601 format.
 */
export function assertValidTimestamp(timestamp: string): void;

/**
 * Assert that a DID resolution result indicates an error.
 */
export function assertResolutionError(
  resolutionResult: any,
  expectedError: string
): void;

/**
 * Assert that two timestamps are within a certain range of each other.
 */
export function assertTimestampsClose(
  timestamp1: string,
  timestamp2: string,
  maxDiffSeconds?: number
): void;
```

### `helpers/fixtures.ts`
**Purpose**: Reusable test data and constants.

```typescript
// Test keys
export const ED25519_TEST_KEY = {
  kty: 'OKP',
  crv: 'Ed25519',
  x: 'VCpo2LMLhn6iWku8MKvSLg2ZAoC-nlOyPVQaO3FxVeQ'
};

export const ED25519_TEST_KEY_2 = {
  kty: 'OKP',
  crv: 'Ed25519',
  x: 'anotherbase64urlencoded32bytevaluehere'
};

export const JUBJUB_TEST_KEY = {
  kty: 'EC',
  crv: 'JubJub',
  x: '3045022100...',
  y: '00ab5910f48...'
};

// Service endpoints
export const SIMPLE_SERVICE_ENDPOINT = 'https://example.com/didcomm';

export const COMPLEX_SERVICE_ENDPOINT = {
  uri: 'https://example.com/didcomm',
  accept: ['didcomm/v2'],
  routingKeys: ['did:example:mediator#key-1']
};

export const ARRAY_SERVICE_ENDPOINT = [
  'https://example.com/endpoint1',
  {
    uri: 'wss://example.com/endpoint2',
    routingKeys: ['did:example:mediator']
  }
];

// Service types
export const SERVICE_TYPE_DIDCOMM_V2 = 'DIDCommV2';
export const SERVICE_TYPE_LINKED_DOMAINS = 'LinkedDomains';

// Aliases
export const EXAMPLE_ALIAS_DID = 'did:example:alias1';
export const EXAMPLE_ALIAS_HTTPS = 'https://example.com/profile';

// Test DIDs
export const EXAMPLE_MEDIATOR_DID = 'did:example:mediator';

// Timestamps (milliseconds since epoch)
export const TEST_TIMESTAMP_1 = 1704067200000; // 2024-01-01T00:00:00Z
export const TEST_TIMESTAMP_2 = 1704070800000; // 2024-01-01T01:00:00Z
export const TEST_TIMESTAMP_3 = 1704153000000; // 2024-01-02T00:00:00Z

// Expected @context values
export const EXPECTED_CONTEXT = [
  'https://www.w3.org/ns/did/v1',
  'https://w3c.github.io/vc-jws-2020/contexts/v1'
];

// Common test IDs
export const TEST_KEY_ID_1 = 'key-1';
export const TEST_KEY_ID_2 = 'key-2';
export const TEST_KEY_ID_JUBJUB = 'key-jubjub';
export const TEST_KEY_ID_ENC = 'key-enc';
export const TEST_KEY_ID_DELEGATE = 'key-delegate';
export const TEST_SERVICE_ID_1 = 'service-1';
export const TEST_SERVICE_ID_2 = 'service-2';
```

## Test File Structure Pattern

Each test file should follow this pattern:

```typescript
import { describe, test, beforeAll, expect } from 'vitest';
import { setupTestEnvironment, createEmptyDID } from './helpers/setup';
import { resolveDID } from './helpers/resolver';
import { buildAddVerificationMethod } from './helpers/contract-operations';
import { assertValidDIDDocument, assertValidVerificationMethod } from './helpers/assertions';
import { ED25519_TEST_KEY, TEST_KEY_ID_1 } from './helpers/fixtures';
import type { TestEnvironment } from './helpers/setup';

describe('NN - Category Name', () => {
  let testEnv: TestEnvironment;

  beforeAll(async () => {
    testEnv = await setupTestEnvironment();
  });

  describe('N.1 Scenario Name', () => {
    test('should do something specific', async () => {
      // Arrange
      const { contract, didString } = await createEmptyDID(testEnv);
      const operation = buildAddVerificationMethod(TEST_KEY_ID_1, ED25519_TEST_KEY);
      
      // Act
      await executeOperation(contract, operation, testEnv);
      const resolutionResult = await resolveDID(didString);
      
      // Assert
      assertValidDIDDocument(resolutionResult.didDocument);
      expect(resolutionResult.didDocument.verificationMethod).toHaveLength(1);
      assertValidVerificationMethod(
        resolutionResult.didDocument.verificationMethod[0],
        `${didString}#${TEST_KEY_ID_1}`,
        didString
      );
    });
  });

  describe('N.2 Another Scenario', () => {
    test('should do another thing', async () => {
      // Test implementation
    });
  });
});
```

## Test File Breakdown

### `01-basic-resolution.test.ts`
- **Scenarios**: 2
- **Focus**: Empty DID resolution, contract version handling
- **Dependencies**: setup, resolver, assertions

### `02-verification-methods.test.ts`
- **Scenarios**: 4
- **Focus**: Ed25519, JubJub, multiple methods, fragment identifiers
- **Dependencies**: All helpers

### `03-verification-relationships.test.ts`
- **Scenarios**: 6
- **Focus**: All 5 relationship types + multiple relationships per key
- **Dependencies**: All helpers

### `04-service-endpoints.test.ts`
- **Scenarios**: 5
- **Focus**: String, object, array endpoints, JSON deserialization, multiple services
- **Dependencies**: All helpers, complex fixtures

### `05-also-known-as.test.ts`
- **Scenarios**: 3
- **Focus**: Single, multiple, non-DID aliases
- **Dependencies**: setup, resolver, contract-operations, assertions

### `06-metadata.test.ts`
- **Scenarios**: 4
- **Focus**: Created/updated timestamps, version counter, timestamp precision
- **Dependencies**: setup, resolver, assertions, timestamp fixtures

### `07-deactivation.test.ts`
- **Scenarios**: 2
- **Focus**: Deactivation operation, deactivated DID structure
- **Dependencies**: All helpers

### `08-edge-cases.test.ts`
- **Scenarios**: 7
- **Focus**: Stress tests, Unicode, empty collections, large numbers, networks, parsing
- **Dependencies**: All helpers, special fixtures

### `09-error-handling.test.ts`
- **Scenarios**: 6
- **Focus**: Invalid states, missing fields, malformed data, validation errors
- **Dependencies**: resolver, assertions (error-specific)

## Implementation Order

### Phase 1: Foundation (Week 1)
1. ✅ Create directory structure
2. ✅ Implement `helpers/setup.ts`
3. ✅ Implement `helpers/resolver.ts`
4. ✅ Implement `helpers/fixtures.ts`
5. ✅ Implement `helpers/assertions.ts`
6. ✅ Implement `helpers/contract-operations.ts`

### Phase 2: Core Tests (Week 2)
1. ✅ Implement `01-basic-resolution.test.ts`
2. ✅ Implement `02-verification-methods.test.ts`
3. ✅ Implement `03-verification-relationships.test.ts`

### Phase 3: Extended Features (Week 3)
1. ✅ Implement `04-service-endpoints.test.ts`
2. ✅ Implement `05-also-known-as.test.ts`
3. ✅ Implement `06-metadata.test.ts`
4. ✅ Implement `07-deactivation.test.ts`

### Phase 4: Edge Cases & Error Handling (Week 4)
1. ✅ Implement `08-edge-cases.test.ts`
2. ✅ Implement `09-error-handling.test.ts`
3. ✅ Run full test suite and fix issues
4. ✅ Document any gaps or known issues

## Running Tests

```bash
# Run all tests
npm test

# Run specific section
npm test 02-verification-methods

# Run multiple sections
npm test -- 02-verification 03-verification

# Watch mode for development
npm run test:watch

# Run with coverage
npm run test:coverage

# Run tests for a specific scenario pattern
npm test -- -t "2.1"
```

## Success Criteria

- ✅ All 40+ test scenarios from `scenarios.md` are implemented
- ✅ Tests are organized into 9 manageable files
- ✅ Helper utilities eliminate code duplication
- ✅ Tests can run in parallel (isolated state)
- ✅ Clear test output with scenario numbers
- ✅ High code coverage of resolver/serialization logic
- ✅ Easy to navigate and maintain

## Benefits of This Approach

### Simplicity
- Only 9 test files + 5 helper modules
- Flat directory structure
- Easy to find tests by section number

### Maintainability
- Shared utilities eliminate duplication
- Tests follow consistent patterns
- Clear separation of concerns

### Extensibility
- Easy to add new scenarios to existing files
- Helper functions make complex setups simple
- Fixtures provide reusable test data

### Developer Experience
- Clear mapping to documentation
- Tests are self-documenting with descriptive names
- Can run individual sections during development
- Fast feedback loop with watch mode

## Notes

- Tests focus exclusively on resolver/serialization logic
- Contract operation logic should be tested in contract's own test suite
- All tests use the existing `index.test.ts` pattern as reference
- Vitest configuration already set for integration tests (10min timeout, sequential execution)

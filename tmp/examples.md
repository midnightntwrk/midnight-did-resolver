# Midnight DID Resolver Integration Tests - Examples and Documentation

## Overview

This document provides comprehensive examples and documentation for implementing integration tests for the `midnight-did-resolver` project. The tests verify that the Rust-based resolver correctly resolves DID Documents from the Midnight blockchain.

---

## Table of Contents

1. [API Understanding](#api-understanding)
2. [Test Architecture](#test-architecture)
3. [Setup Requirements](#setup-requirements)
4. [Example Test: Scenario 1.1 - Basic DID Resolution](#example-test-scenario-11---basic-did-resolution)
5. [Example Test: Scenario 2.1 - Single Verification Method (Ed25519)](#example-test-scenario-21---single-verification-method-ed25519)
6. [API Functions Reference](#api-functions-reference)
7. [DID Operations Reference](#did-operations-reference)
8. [Common Assertions](#common-assertions)
9. [Testing Pattern Guide](#testing-pattern-guide)
10. [Tips and Best Practices](#tips-and-best-practices)

---

## API Understanding

The `@midnight-ntwrk/midnight-did-api` package provides the following key functions:

### Core Functions

| Function | Purpose | Returns |
|----------|---------|---------|
| `createDID(providers, privateState)` | Deploys a new DID contract to the blockchain | `DeployedMidnightDIDContract` |
| `update(contract, operations)` | Applies DID operations (add/remove verification methods, services, etc.) | `FinalizedTxData` |
| `resolve(providers, contract)` | Resolves DID documents using the API's internal resolver | `{ didDocument, didDocumentMetadata }` |
| `initPrivateState(providers)` | Initializes the private state for DID operations | `MidnightDIDPrivateState` |
| `configureProviders(wallet, config)` | Sets up all necessary providers (indexer, proof server, etc.) | `MidnightDIDProviders` |
| `buildFreshWallet(config)` | Creates a new wallet with funds | `Wallet & Resource` |
| `getMidnightDIDLedgerState(providers, address)` | Gets the raw ledger state from the blockchain | `DIDContract.Ledger` |

### Key Types

- **`DIDOperation`** - Operations to modify DIDs (AddVerificationMethod, AddService, etc.)
- **`DeployedMidnightDIDContract`** - The deployed contract instance
- **`MidnightDIDProviders`** - Configuration for wallet, indexer, proof server, etc.
- **`MidnightDIDDocument`** - The resolved DID Document structure
- **`DIDDocumentMetadata`** - Metadata including created, updated, versionId, deactivated

---

## Test Architecture

### Testing Flow

```
┌─────────────────────────────────────────────────────────────┐
│ 1. Setup (beforeAll)                                        │
│    ├── Create wallet with funds                             │
│    ├── Configure providers (indexer, proof server, etc.)    │
│    └── Deploy DID contract (creates new DID)                │
└─────────────────────────────────────────────────────────────┘
                           ↓
┌─────────────────────────────────────────────────────────────┐
│ 2. Test Action (it block)                                   │
│    ├── Use @midnight-ntwrk/midnight-did-api to modify DID   │
│    ├── Wait for blockchain confirmation                     │
│    ├── Call midnight-did-resolver HTTP API to resolve DID   │
│    └── Assert results match expectations                    │
└─────────────────────────────────────────────────────────────┘
                           ↓
┌─────────────────────────────────────────────────────────────┐
│ 3. Cleanup (afterAll)                                       │
│    └── Close wallet and release resources                   │
└─────────────────────────────────────────────────────────────┘
```

### Key Principle

The tests verify the **midnight-did-resolver** (Rust binary) by:
1. Using the TypeScript API to CREATE/UPDATE DIDs on the blockchain
2. Using the Rust resolver HTTP API to RESOLVE DIDs
3. Comparing the resolved result with expected values

---

## Setup Requirements

### 1. Dependencies

```json
{
  "dependencies": {
    "@midnight-ntwrk/midnight-did-api": "file:./vendor/api",
    "@midnight-ntwrk/midnight-did": "file:./vendor/did",
    "@midnight-ntwrk/midnight-did-contract": "file:./vendor/contract",
    "@midnight-ntwrk/midnight-did-domain": "file:./vendor/domain"
  },
  "devDependencies": {
    "@types/node": "^24.10.1",
    "vitest": "^latest",
    "tsx": "^4.20.6",
    "typescript": "^5.9.3"
  }
}
```

### 2. Environment Setup

**Start the midnight-did-resolver server:**
```bash
cargo run -- serve --indexer-url http://localhost:8088/api/v1/graphql --port 8080
```

**Ensure Midnight infrastructure is running:**
- Midnight node
- Midnight indexer (GraphQL API)
- Proof server

**Configure environment variables:**
```bash
export MIDNIGHT_INDEXER_URL="http://localhost:8088/api/v1/graphql"
export SYNC_CACHE="./cache"  # Directory for wallet cache
```

### 3. Project Structure

```
tests/integration-tests/
├── src/
│   ├── index.ts                    # Main test file
│   ├── scenarios/
│   │   ├── scenario-1-1.test.ts    # Basic DID Resolution
│   │   ├── scenario-2-1.test.ts    # Ed25519 Verification Method
│   │   └── ...                     # Additional scenarios
│   └── helpers/
│       ├── setup.ts                # Test setup utilities
│       └── assertions.ts           # Common assertion helpers
├── vendor/                         # Symlinked packages from midnight-did
├── package.json
└── tsconfig.json
```

---

## Example Test: Scenario 1.1 - Basic DID Resolution

**Test Goal:** Verify that a newly deployed DID (empty state) resolves to a minimal DID Document with only the `id` field populated and metadata present.

### Complete Test Code

```typescript
import { expect, describe, it, beforeAll, afterAll } from 'vitest';
import * as api from '@midnight-ntwrk/midnight-did-api';
import { 
  parseContractAddress, 
  createMidnightDIDString 
} from '@midnight-ntwrk/midnight-did-domain';

describe('Scenario 1.1: Basic DID Resolution - Empty State', () => {
  let wallet: any;
  let providers: api.MidnightDIDProviders;
  let contract: api.DeployedMidnightDIDContract;
  let didString: string;
  const resolverUrl = 'http://localhost:8080';
  
  beforeAll(async () => {
    // Configure test logger
    const logger = createTestLogger();
    api.setLogger(logger);
    
    // Setup configuration (adjust based on your environment)
    const config = {
      indexer: 'http://localhost:8088/api/v1/graphql',
      indexerWS: 'ws://localhost:8088/api/v1/graphql',
      node: 'http://localhost:1313',
      proofServer: 'http://localhost:6300'
    };
    
    // Create wallet with funds
    wallet = await api.buildFreshWallet(config);
    
    // Configure providers
    providers = await api.configureProviders(wallet, config);
    
    // Create a new DID (deploys contract with empty state)
    const privateState = await api.initPrivateState(providers);
    contract = await api.createDID(providers, privateState);
    
    // Generate DID string
    const contractAddress = parseContractAddress(
      contract.deployTxData.public.contractAddress
    );
    didString = createMidnightDIDString(contractAddress, api.midnightNetwork);
    
    // Wait for blockchain confirmation
    await waitForBlockConfirmation(contract.deployTxData.public.blockHeight);
  }, 60000); // 60 second timeout for setup
  
  afterAll(async () => {
    if (wallet) {
      await wallet.close();
    }
  });
  
  it('should resolve newly deployed DID with minimal document', async () => {
    // Resolve using the midnight-did-resolver (Rust HTTP API)
    const response = await fetch(
      `${resolverUrl}/1.0/identifiers/${didString}`
    );
    
    expect(response.status).toBe(200);
    const resolutionResult = await response.json();
    
    // Verify minimal DID Document structure
    expect(resolutionResult.didDocument).toBeDefined();
    expect(resolutionResult.didDocument.id).toBe(didString);
    
    // All arrays should be empty or undefined
    expect(
      resolutionResult.didDocument.verificationMethod?.length ?? 0
    ).toBe(0);
    expect(
      resolutionResult.didDocument.authentication?.length ?? 0
    ).toBe(0);
    expect(
      resolutionResult.didDocument.assertionMethod?.length ?? 0
    ).toBe(0);
    expect(
      resolutionResult.didDocument.keyAgreement?.length ?? 0
    ).toBe(0);
    expect(
      resolutionResult.didDocument.capabilityInvocation?.length ?? 0
    ).toBe(0);
    expect(
      resolutionResult.didDocument.capabilityDelegation?.length ?? 0
    ).toBe(0);
    expect(
      resolutionResult.didDocument.service?.length ?? 0
    ).toBe(0);
    expect(
      resolutionResult.didDocument.alsoKnownAs?.length ?? 0
    ).toBe(0);
    
    // Metadata should contain created timestamp
    expect(resolutionResult.didDocumentMetadata).toBeDefined();
    expect(resolutionResult.didDocumentMetadata.created).toBeDefined();
    expect(resolutionResult.didDocumentMetadata.created).toMatch(
      /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}Z$/
    );
    
    // versionId should be present
    expect(resolutionResult.didDocumentMetadata.versionId).toBeDefined();
    
    // Should not be deactivated
    expect(resolutionResult.didDocumentMetadata.deactivated).toBeUndefined();
  });
  
  it('should have correct @context', async () => {
    const response = await fetch(
      `${resolverUrl}/1.0/identifiers/${didString}`
    );
    const resolutionResult = await response.json();
    
    expect(Array.isArray(resolutionResult.didDocument['@context'])).toBe(true);
    expect(resolutionResult.didDocument['@context']).toContain(
      'https://www.w3.org/ns/did/v1'
    );
    expect(resolutionResult.didDocument['@context']).toContain(
      'https://w3c.github.io/vc-jws-2020/contexts/v1'
    );
  });
  
  it('should match DID format: did:midnight:<network>:<address>', async () => {
    const response = await fetch(
      `${resolverUrl}/1.0/identifiers/${didString}`
    );
    const resolutionResult = await response.json();
    
    const didPattern = /^did:midnight:(undeployed|devnet|testnet|mainnet):[0-9a-f]{68}$/;
    expect(resolutionResult.didDocument.id).toMatch(didPattern);
  });
});

// Helper function to wait for block confirmation
async function waitForBlockConfirmation(
  blockHeight: bigint,
  maxWaitMs = 30000
): Promise<void> {
  const startTime = Date.now();
  console.log(`Waiting for block ${blockHeight} to be confirmed...`);
  
  while (Date.now() - startTime < maxWaitMs) {
    // Simple delay - in production, you might query the indexer
    // to check if the block has been indexed
    await new Promise(resolve => setTimeout(resolve, 2000));
  }
  
  console.log(`Block confirmation wait completed`);
}

// Helper to create test logger
function createTestLogger() {
  return {
    info: (msg: string) => console.log(`[INFO] ${msg}`),
    warn: (msg: string) => console.warn(`[WARN] ${msg}`),
    error: (msg: string) => console.error(`[ERROR] ${msg}`),
  };
}
```

### What This Test Validates

1. ✅ DID resolves successfully (HTTP 200)
2. ✅ DID Document contains correct `id` field
3. ✅ All relationship arrays are empty
4. ✅ `@context` contains required W3C DID Core specifications
5. ✅ Metadata includes `created` timestamp in ISO 8601 format
6. ✅ Metadata includes `versionId`
7. ✅ DID is not deactivated
8. ✅ DID format matches specification

---

## Example Test: Scenario 2.1 - Single Verification Method (Ed25519)

**Test Goal:** Verify that adding an Ed25519 verification method to a DID is correctly serialized, stored on-chain, and deserialized by the resolver.

### Complete Test Code

```typescript
import { expect, describe, it, beforeAll, afterAll } from 'vitest';
import * as api from '@midnight-ntwrk/midnight-did-api';
import { 
  parseContractAddress, 
  createMidnightDIDString,
  DIDOperation,
  DIDOperationType,
  VerificationMethodType,
  KeyType,
  CurveType,
  parseDIDKeyID
} from '@midnight-ntwrk/midnight-did-domain';

describe('Scenario 2.1: Single Verification Method - Ed25519', () => {
  let wallet: any;
  let providers: api.MidnightDIDProviders;
  let contract: api.DeployedMidnightDIDContract;
  let didString: string;
  const resolverUrl = 'http://localhost:8080';
  
  beforeAll(async () => {
    // Setup logger
    const logger = createTestLogger();
    api.setLogger(logger);
    
    // Setup configuration
    const config = {
      indexer: 'http://localhost:8088/api/v1/graphql',
      indexerWS: 'ws://localhost:8088/api/v1/graphql',
      node: 'http://localhost:1313',
      proofServer: 'http://localhost:6300'
    };
    
    // Create wallet and configure providers
    wallet = await api.buildFreshWallet(config);
    providers = await api.configureProviders(wallet, config);
    
    // Create a new DID
    const privateState = await api.initPrivateState(providers);
    contract = await api.createDID(providers, privateState);
    
    // Generate DID string
    const contractAddress = parseContractAddress(
      contract.deployTxData.public.contractAddress
    );
    didString = createMidnightDIDString(contractAddress, api.midnightNetwork);
    
    // Wait for initial deployment confirmation
    await waitForBlockConfirmation(contract.deployTxData.public.blockHeight);
  }, 60000);
  
  afterAll(async () => {
    if (wallet) {
      await wallet.close();
    }
  });
  
  it('should add and correctly resolve Ed25519 verification method', async () => {
    // Step 1: Define the verification method
    const methodId = parseDIDKeyID(`${didString}#key-1`);
    const publicKeyJwk = {
      kty: KeyType.OKP,
      crv: CurveType.Ed25519,
      x: 'Kg' // Base64url-encoded 32-byte value
    };
    
    // Step 2: Create the DID operation
    const operations: DIDOperation[] = [
      {
        type: DIDOperationType.AddVerificationMethod,
        verificationMethod: {
          id: methodId,
          type: VerificationMethodType.JsonWebKey,
          controller: didString,
          publicKeyJwk
        }
      }
    ];
    
    // Step 3: Apply the operation to the blockchain
    console.log('Adding verification method to DID...');
    const txResult = await api.update(contract, operations);
    expect(txResult.txId).toMatch(/[0-9a-f]{64}/);
    console.log(`Transaction ID: ${txResult.txId}`);
    
    // Step 4: Wait for transaction confirmation
    await waitForBlockConfirmation(txResult.blockHeight);
    
    // Step 5: Resolve using midnight-did-resolver
    console.log('Resolving DID with midnight-did-resolver...');
    const response = await fetch(
      `${resolverUrl}/1.0/identifiers/${didString}`
    );
    
    expect(response.status).toBe(200);
    const resolutionResult = await response.json();
    
    // Step 6: Assert the verification method is correctly deserialized
    expect(resolutionResult.didDocument.verificationMethod).toBeDefined();
    expect(resolutionResult.didDocument.verificationMethod.length).toBe(1);
    
    const vm = resolutionResult.didDocument.verificationMethod[0];
    
    // Verify all fields
    expect(vm.id).toBe('#key-1'); // Should be fragment only
    expect(vm.type).toBe(VerificationMethodType.JsonWebKey);
    expect(vm.controller).toBe(didString);
    
    // Verify JWK structure
    expect(vm.publicKeyJwk).toBeDefined();
    expect(vm.publicKeyJwk.kty).toBe(KeyType.OKP);
    expect(vm.publicKeyJwk.crv).toBe(CurveType.Ed25519);
    expect(vm.publicKeyJwk.x).toBe('Kg');
    
    // Ed25519 should NOT have a y coordinate
    expect(vm.publicKeyJwk.y).toBeUndefined();
    
    console.log('✅ Verification method correctly resolved');
  });
  
  it('should correctly handle verification method with various base64url values', async () => {
    // Test different x values to ensure proper encoding/decoding
    const testCases = [
      { name: 'minimum', x: 'AA', description: 'all zeros' },
      { name: 'small', x: 'SGVsbG8', description: 'small value' },
      { name: 'medium', x: 'SGVsbG8gV29ybGQ', description: 'medium value' },
      { 
        name: 'large', 
        x: 'abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-_',
        description: 'large value with special chars'
      }
    ];
    
    for (const testCase of testCases) {
      console.log(`Testing ${testCase.name}: ${testCase.description}`);
      
      const methodId = parseDIDKeyID(`${didString}#key-${testCase.name}`);
      
      const publicKeyJwk = {
        kty: KeyType.OKP,
        crv: CurveType.Ed25519,
        x: testCase.x
      };
      
      const operations: DIDOperation[] = [
        {
          type: DIDOperationType.AddVerificationMethod,
          verificationMethod: {
            id: methodId,
            type: VerificationMethodType.JsonWebKey,
            controller: didString,
            publicKeyJwk
          }
        }
      ];
      
      const txResult = await api.update(contract, operations);
      expect(txResult.txId).toMatch(/[0-9a-f]{64}/);
      
      await waitForBlockConfirmation(txResult.blockHeight);
      
      const response = await fetch(
        `${resolverUrl}/1.0/identifiers/${didString}`
      );
      const resolutionResult = await response.json();
      
      const vm = resolutionResult.didDocument.verificationMethod.find(
        (v: any) => v.id === `#key-${testCase.name}`
      );
      
      expect(vm).toBeDefined();
      expect(vm.publicKeyJwk.x).toBe(testCase.x);
      
      console.log(`✅ ${testCase.name} value correctly encoded/decoded`);
    }
  });
  
  it('should handle adding verification method with authentication relationship', async () => {
    // Add a verification method and immediately assign it to authentication
    const methodId = parseDIDKeyID(`${didString}#key-auth`);
    
    const operations: DIDOperation[] = [
      {
        type: DIDOperationType.AddVerificationMethod,
        verificationMethod: {
          id: methodId,
          type: VerificationMethodType.JsonWebKey,
          controller: didString,
          publicKeyJwk: {
            kty: KeyType.OKP,
            crv: CurveType.Ed25519,
            x: 'YXV0aA' // "auth" in base64url
          }
        }
      },
      {
        type: DIDOperationType.AddVerificationMethodRelation,
        relation: 'authentication',
        methodId: methodId
      }
    ];
    
    const txResult = await api.update(contract, operations);
    await waitForBlockConfirmation(txResult.blockHeight);
    
    const response = await fetch(
      `${resolverUrl}/1.0/identifiers/${didString}`
    );
    const resolutionResult = await response.json();
    
    // Verify method exists
    const vm = resolutionResult.didDocument.verificationMethod.find(
      (v: any) => v.id === '#key-auth'
    );
    expect(vm).toBeDefined();
    
    // Verify authentication relationship
    expect(resolutionResult.didDocument.authentication).toBeDefined();
    expect(resolutionResult.didDocument.authentication).toContain('#key-auth');
    
    console.log('✅ Verification method with relationship correctly resolved');
  });
});

// Helper functions
async function waitForBlockConfirmation(
  blockHeight: bigint,
  maxWaitMs = 30000
): Promise<void> {
  const startTime = Date.now();
  console.log(`Waiting for block ${blockHeight} to be confirmed...`);
  
  while (Date.now() - startTime < maxWaitMs) {
    await new Promise(resolve => setTimeout(resolve, 2000));
  }
  
  console.log(`Block confirmation wait completed`);
}

function createTestLogger() {
  return {
    info: (msg: string) => console.log(`[INFO] ${msg}`),
    warn: (msg: string) => console.warn(`[WARN] ${msg}`),
    error: (msg: string) => console.error(`[ERROR] ${msg}`),
  };
}
```

### What This Test Validates

1. ✅ Ed25519 verification method is correctly added
2. ✅ Transaction returns valid transaction ID
3. ✅ Verification method is properly deserialized
4. ✅ All JWK fields are correct (kty, crv, x)
5. ✅ Ed25519 does NOT have y coordinate
6. ✅ Method ID is correctly formatted as fragment (`#key-1`)
7. ✅ Controller field matches DID
8. ✅ Various base64url values are correctly encoded/decoded
9. ✅ Verification relationships can be added alongside methods

---

## API Functions Reference

### Wallet and Provider Setup

```typescript
// Create a new wallet with funds
const wallet = await api.buildFreshWallet(config);

// Or restore from seed
const wallet = await api.buildWalletAndWaitForFunds(
  config,
  'your-seed-hex',
  'cache-filename'
);

// Configure all providers
const providers = await api.configureProviders(wallet, config);
```

### DID Lifecycle

```typescript
// Initialize private state
const privateState = await api.initPrivateState(providers);

// Create new DID (deploy contract)
const contract = await api.createDID(providers, privateState);

// Join existing DID contract
const contract = await api.joinContract(providers, contractAddress);

// Get contract address and DID string
const contractAddress = parseContractAddress(
  contract.deployTxData.public.contractAddress
);
const didString = createMidnightDIDString(contractAddress, api.midnightNetwork);
```

### DID Operations

```typescript
// Apply operations (returns transaction data)
const txResult = await api.update(contract, operations);

// Access transaction details
console.log(`TX ID: ${txResult.txId}`);
console.log(`Block: ${txResult.blockHeight}`);
```

### Resolution

```typescript
// Resolve using API's internal resolver
const result = await api.resolve(providers, contract);
console.log(result.didDocument);
console.log(result.didDocumentMetadata);

// Get raw ledger state
const ledgerState = await api.getMidnightDIDLedgerState(
  providers,
  contractAddress
);
```

---

## DID Operations Reference

### Available Operation Types

```typescript
enum DIDOperationType {
  AddVerificationMethod = 'AddVerificationMethod',
  RemoveVerificationMethod = 'RemoveVerificationMethod',
  AddVerificationMethodRelation = 'AddVerificationMethodRelation',
  RemoveVerificationMethodRelation = 'RemoveVerificationMethodRelation',
  AddService = 'AddService',
  UpdateService = 'UpdateService',
  RemoveService = 'RemoveService',
  AddAlsoKnownAs = 'AddAlsoKnownAs',
  RemoveAlsoKnownAs = 'RemoveAlsoKnownAs',
  Deactivate = 'Deactivate'
}
```

### Operation Examples

#### Add Verification Method

```typescript
const operation: DIDOperation = {
  type: DIDOperationType.AddVerificationMethod,
  verificationMethod: {
    id: parseDIDKeyID(`${didString}#key-1`),
    type: VerificationMethodType.JsonWebKey,
    controller: didString,
    publicKeyJwk: {
      kty: KeyType.OKP,
      crv: CurveType.Ed25519,
      x: 'base64url-encoded-value'
    }
  }
};
```

#### Add JubJub Verification Method

```typescript
const operation: DIDOperation = {
  type: DIDOperationType.AddVerificationMethod,
  verificationMethod: {
    id: parseDIDKeyID(`${didString}#key-jubjub`),
    type: VerificationMethodType.JsonWebKey,
    controller: didString,
    publicKeyJwk: {
      kty: KeyType.EC,
      crv: CurveType.Jubjub,
      x: 'field-element-x',
      y: 'field-element-y'
    }
  }
};
```

#### Add Verification Relationship

```typescript
const operation: DIDOperation = {
  type: DIDOperationType.AddVerificationMethodRelation,
  relation: 'authentication', // or 'assertionMethod', 'keyAgreement', etc.
  methodId: parseDIDKeyID(`${didString}#key-1`)
};
```

#### Add Service Endpoint (String)

```typescript
const operation: DIDOperation = {
  type: DIDOperationType.AddService,
  service: parseService({
    id: '#service-1',
    type: 'DIDCommV2',
    serviceEndpoint: 'https://example.com/endpoint'
  })
};
```

#### Add Service Endpoint (Array)

```typescript
const operation: DIDOperation = {
  type: DIDOperationType.AddService,
  service: parseService({
    id: '#service-messaging',
    type: 'Messaging',
    serviceEndpoint: [
      'https://primary.example.com',
      'https://backup.example.com'
    ]
  })
};
```

#### Add Service Endpoint (Object)

```typescript
const operation: DIDOperation = {
  type: DIDOperationType.AddService,
  service: parseService({
    id: '#service-agent',
    type: 'AgentService',
    serviceEndpoint: {
      uri: 'https://agent.example.com',
      routingKeys: ['did:example:mediator#key-1'],
      accept: ['didcomm/v2']
    }
  })
};
```

#### Update Service

```typescript
const operation: DIDOperation = {
  type: DIDOperationType.UpdateService,
  service: parseService({
    id: '#service-1',
    type: 'DIDCommV2',
    serviceEndpoint: 'https://updated.example.com/endpoint'
  })
};
```

#### Remove Service

```typescript
const operation: DIDOperation = {
  type: DIDOperationType.RemoveService,
  serviceId: ServiceIdSchema.parse('service-1') // without #
};
```

#### Add Also Known As

```typescript
const operation: DIDOperation = {
  type: DIDOperationType.AddAlsoKnownAs,
  aliasUri: 'did:example:alias123'
};
```

#### Deactivate DID

```typescript
const operation: DIDOperation = {
  type: DIDOperationType.Deactivate
};
```

---

## Common Assertions

### DID Document Structure

```typescript
// Basic structure
expect(resolutionResult.didDocument).toBeDefined();
expect(resolutionResult.didDocument.id).toBe(expectedDIDString);
expect(resolutionResult.didDocument['@context']).toContain(
  'https://www.w3.org/ns/did/v1'
);

// DID format
const didPattern = /^did:midnight:(undeployed|devnet|testnet|mainnet):[0-9a-f]{68}$/;
expect(didString).toMatch(didPattern);
```

### Verification Methods

```typescript
// Find specific method
const vm = resolutionResult.didDocument.verificationMethod.find(
  (v: any) => v.id === '#key-1'
);
expect(vm).toBeDefined();

// Check Ed25519 method
expect(vm.type).toBe('JsonWebKey');
expect(vm.publicKeyJwk.kty).toBe('OKP');
expect(vm.publicKeyJwk.crv).toBe('Ed25519');
expect(vm.publicKeyJwk.x).toBeDefined();
expect(vm.publicKeyJwk.y).toBeUndefined(); // Ed25519 has no y

// Check JubJub method
expect(vm.publicKeyJwk.kty).toBe('EC');
expect(vm.publicKeyJwk.crv).toBe('Jubjub');
expect(vm.publicKeyJwk.x).toBeDefined();
expect(vm.publicKeyJwk.y).toBeDefined(); // JubJub has both x and y
```

### Verification Relationships

```typescript
// Check authentication
expect(resolutionResult.didDocument.authentication).toBeDefined();
expect(resolutionResult.didDocument.authentication).toContain('#key-1');

// Check all relationship types
const relationships = [
  'authentication',
  'assertionMethod',
  'keyAgreement',
  'capabilityInvocation',
  'capabilityDelegation'
];

for (const rel of relationships) {
  if (resolutionResult.didDocument[rel]) {
    expect(Array.isArray(resolutionResult.didDocument[rel])).toBe(true);
  }
}
```

### Services

```typescript
// Find specific service
const service = resolutionResult.didDocument.service.find(
  (s: any) => s.id === '#service-1'
);
expect(service).toBeDefined();
expect(service.type).toBe('DIDCommV2');

// String endpoint
expect(typeof service.serviceEndpoint).toBe('string');
expect(service.serviceEndpoint).toBe('https://example.com/endpoint');

// Array endpoint
expect(Array.isArray(service.serviceEndpoint)).toBe(true);
expect(service.serviceEndpoint).toContain('https://primary.example.com');

// Object endpoint
expect(typeof service.serviceEndpoint).toBe('object');
expect(service.serviceEndpoint.uri).toBe('https://agent.example.com');
```

### Metadata

```typescript
// Created timestamp
expect(resolutionResult.didDocumentMetadata.created).toBeDefined();
expect(resolutionResult.didDocumentMetadata.created).toMatch(
  /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}Z$/
);

// Updated timestamp (if DID has been updated)
if (resolutionResult.didDocumentMetadata.updated) {
  expect(resolutionResult.didDocumentMetadata.updated).toMatch(
    /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}Z$/
  );
}

// Version ID
expect(resolutionResult.didDocumentMetadata.versionId).toBeDefined();
expect(typeof resolutionResult.didDocumentMetadata.versionId).toBe('string');

// Deactivated status
if (resolutionResult.didDocumentMetadata.deactivated) {
  expect(resolutionResult.didDocumentMetadata.deactivated).toBe(true);
}
```

### Error Cases

```typescript
// Not found
const response = await fetch(`${resolverUrl}/1.0/identifiers/${invalidDID}`);
expect(response.status).toBe(404);

// Invalid DID format
const response = await fetch(`${resolverUrl}/1.0/identifiers/did:invalid:format`);
expect(response.status).toBe(400);
```

---

## Testing Pattern Guide

### Standard Test Template

```typescript
describe('Scenario X.Y: Test Description', () => {
  let wallet: any;
  let providers: api.MidnightDIDProviders;
  let contract: api.DeployedMidnightDIDContract;
  let didString: string;
  const resolverUrl = 'http://localhost:8080';
  
  beforeAll(async () => {
    // Setup logger
    const logger = createTestLogger();
    api.setLogger(logger);
    
    // Setup configuration
    const config = {
      indexer: 'http://localhost:8088/api/v1/graphql',
      indexerWS: 'ws://localhost:8088/api/v1/graphql',
      node: 'http://localhost:1313',
      proofServer: 'http://localhost:6300'
    };
    
    // Create wallet and configure providers
    wallet = await api.buildFreshWallet(config);
    providers = await api.configureProviders(wallet, config);
    
    // Create DID
    const privateState = await api.initPrivateState(providers);
    contract = await api.createDID(providers, privateState);
    
    const contractAddress = parseContractAddress(
      contract.deployTxData.public.contractAddress
    );
    didString = createMidnightDIDString(contractAddress, api.midnightNetwork);
    
    await waitForBlockConfirmation(contract.deployTxData.public.blockHeight);
  }, 60000);
  
  afterAll(async () => {
    if (wallet) {
      await wallet.close();
    }
  });
  
  it('should [specific behavior]', async () => {
    // 1. Define DID operations
    const operations: DIDOperation[] = [
      // ... operations
    ];
    
    // 2. Apply operations to blockchain
    const txResult = await api.update(contract, operations);
    expect(txResult.txId).toMatch(/[0-9a-f]{64}/);
    
    // 3. Wait for confirmation
    await waitForBlockConfirmation(txResult.blockHeight);
    
    // 4. Resolve using midnight-did-resolver
    const response = await fetch(
      `${resolverUrl}/1.0/identifiers/${didString}`
    );
    expect(response.status).toBe(200);
    
    const resolutionResult = await response.json();
    
    // 5. Assert expectations
    expect(resolutionResult.didDocument.someField).toBe(expectedValue);
  });
});
```

### Multi-Step Test Pattern

For tests that require multiple updates:

```typescript
it('should handle progressive updates correctly', async () => {
  // Step 1: Add verification method
  let operations: DIDOperation[] = [
    { type: DIDOperationType.AddVerificationMethod, /* ... */ }
  ];
  let txResult = await api.update(contract, operations);
  await waitForBlockConfirmation(txResult.blockHeight);
  
  let response = await fetch(`${resolverUrl}/1.0/identifiers/${didString}`);
  let result = await response.json();
  expect(result.didDocument.verificationMethod.length).toBe(1);
  
  // Step 2: Add service
  operations = [
    { type: DIDOperationType.AddService, /* ... */ }
  ];
  txResult = await api.update(contract, operations);
  await waitForBlockConfirmation(txResult.blockHeight);
  
  response = await fetch(`${resolverUrl}/1.0/identifiers/${didString}`);
  result = await response.json();
  expect(result.didDocument.verificationMethod.length).toBe(1);
  expect(result.didDocument.service.length).toBe(1);
  
  // Step 3: Deactivate
  operations = [
    { type: DIDOperationType.Deactivate }
  ];
  txResult = await api.update(contract, operations);
  await waitForBlockConfirmation(txResult.blockHeight);
  
  response = await fetch(`${resolverUrl}/1.0/identifiers/${didString}`);
  result = await response.json();
  expect(result.didDocumentMetadata.deactivated).toBe(true);
});
```

### Batch Operations Pattern

```typescript
it('should handle batch operations atomically', async () => {
  // All operations are applied in a single transaction
  const operations: DIDOperation[] = [
    {
      type: DIDOperationType.AddVerificationMethod,
      verificationMethod: { /* method 1 */ }
    },
    {
      type: DIDOperationType.AddVerificationMethod,
      verificationMethod: { /* method 2 */ }
    },
    {
      type: DIDOperationType.AddVerificationMethodRelation,
      relation: 'authentication',
      methodId: parseDIDKeyID(`${didString}#key-1`)
    },
    {
      type: DIDOperationType.AddService,
      service: parseService({ /* service */ })
    }
  ];
  
  const txResult = await api.update(contract, operations);
  await waitForBlockConfirmation(txResult.blockHeight);
  
  const response = await fetch(`${resolverUrl}/1.0/identifiers/${didString}`);
  const result = await response.json();
  
  // All changes should be present
  expect(result.didDocument.verificationMethod.length).toBe(2);
  expect(result.didDocument.authentication.length).toBe(1);
  expect(result.didDocument.service.length).toBe(1);
});
```

---

## Tips and Best Practices

### 1. Wait for Blockchain Confirmation

Always wait for transactions to be confirmed before resolving:

```typescript
const txResult = await api.update(contract, operations);
await waitForBlockConfirmation(txResult.blockHeight);
// Now safe to resolve
```

### 2. Use Meaningful IDs

Use descriptive IDs for verification methods and services:

```typescript
// Good
#key-auth-ed25519
#key-signing-jubjub
#service-messaging-didcomm
#service-domain-link

// Avoid
#1
#x
#temp
```

### 3. Test Edge Cases

Don't just test happy paths:

```typescript
// Empty values
x: 'AA' // all zeros

// Maximum values
x: 'very-long-base64url-string...'

// Special characters in base64url
x: 'abc-_123'

// Minimum arrays
serviceEndpoint: []

// Maximum arrays
serviceEndpoint: [/* 100 items */]
```

### 4. Verify Transaction IDs

Always check that operations return valid transaction IDs:

```typescript
const txResult = await api.update(contract, operations);
expect(txResult.txId).toMatch(/[0-9a-f]{64}/);
```

### 5. Clean Up Resources

Always close wallets and resources:

```typescript
afterAll(async () => {
  if (wallet) {
    await wallet.close();
  }
});
```

### 6. Use Test Helpers

Create reusable helper functions:

```typescript
// helpers/assertions.ts
export function assertValidDIDDocument(doc: any, expectedId: string) {
  expect(doc).toBeDefined();
  expect(doc.id).toBe(expectedId);
  expect(doc['@context']).toContain('https://www.w3.org/ns/did/v1');
}

export function assertValidMetadata(metadata: any) {
  expect(metadata).toBeDefined();
  expect(metadata.created).toMatch(/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}Z$/);
  expect(metadata.versionId).toBeDefined();
}

// helpers/setup.ts
export async function setupTestDID() {
  const wallet = await api.buildFreshWallet(config);
  const providers = await api.configureProviders(wallet, config);
  const privateState = await api.initPrivateState(providers);
  const contract = await api.createDID(providers, privateState);
  
  const contractAddress = parseContractAddress(
    contract.deployTxData.public.contractAddress
  );
  const didString = createMidnightDIDString(contractAddress, api.midnightNetwork);
  
  return { wallet, providers, contract, didString };
}
```

### 7. Test Both Success and Failure

```typescript
it('should reject invalid verification method', async () => {
  const operations: DIDOperation[] = [
    {
      type: DIDOperationType.AddVerificationMethod,
      verificationMethod: {
        id: parseDIDKeyID(`${didString}#invalid`),
        type: 'InvalidType', // Invalid type
        controller: didString,
        publicKeyJwk: { /* ... */ }
      }
    }
  ];
  
  // Should throw or return error
  await expect(api.update(contract, operations)).rejects.toThrow();
});
```

### 8. Use Descriptive Test Names

```typescript
// Good
it('should correctly deserialize Ed25519 key with base64url-encoded x coordinate')

// Better than
it('should work')
```

### 9. Log Important Information

```typescript
console.log(`Created DID: ${didString}`);
console.log(`Transaction ID: ${txResult.txId}`);
console.log(`Block height: ${txResult.blockHeight}`);
```

### 10. Test Resolution Across Different Networks

```typescript
const networks = ['undeployed', 'devnet', 'testnet'];

for (const network of networks) {
  it(`should resolve DID on ${network}`, async () => {
    // Test resolution on different networks
  });
}
```

---

## Key Files Reference

### Source Files

- **API Implementation**: `tmp/midnight-did/api/src/lib.ts`
- **API Tests (reference)**: `tmp/midnight-did/api/src/test/did.api.test.ts`
- **Domain Types**: `tmp/midnight-did/domain/src/index.ts`
- **DID Document Types**: `tmp/midnight-did/domain/src/did-document.ts`
- **Test Scenarios**: `tmp/scenarios.md`

### Integration Test Files

- **Test Location**: `./tests/integration-tests/`
- **Package Config**: `./tests/integration-tests/package.json`
- **Main Test File**: `./tests/integration-tests/src/index.ts`

---

## Next Steps

To implement these tests:

1. **Setup the test environment**
   ```bash
   cd tests/integration-tests
   npm install
   ```

2. **Start the infrastructure**
   ```bash
   # Terminal 1: Start Midnight node and indexer (via Docker Compose)
   docker-compose up
   
   # Terminal 2: Start the resolver
   cd ../..
   cargo run -- serve --indexer-url http://localhost:8088/api/v1/graphql
   ```

3. **Create test files**
   ```bash
   mkdir -p src/scenarios
   # Copy example tests to src/scenarios/
   ```

4. **Run the tests**
   ```bash
   npm test
   ```

5. **Implement remaining scenarios**
   - Use the examples as templates
   - Follow the patterns documented above
   - Test edge cases thoroughly

---

## Conclusion

This documentation provides:

- ✅ Complete understanding of the `@midnight-ntwrk/midnight-did-api`
- ✅ Two fully-implemented example tests
- ✅ Comprehensive API reference
- ✅ Testing patterns and best practices
- ✅ Common assertions and helpers
- ✅ Tips for junior QA engineers

Use these examples as templates to implement the remaining test scenarios from `tmp/scenarios.md`. The patterns are consistent across all scenarios - the main differences are:

1. The DID operations being applied
2. The specific fields being validated
3. The edge cases being tested

Good luck with implementing the rest of the test suite!

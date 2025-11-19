Please take a look at the following midnight-did specification which is a w3c DID method.
You may have to use command `gh` to access it as this is a private repo.

- https://github.com/midnightntwrk/midnight-did/blob/main/w3c-spec/midnight-method.md

There is also a smart contract for midnight-did which is here

- https://github.com/midnightntwrk/midnight-did/blob/main/contract/src/did.compact

Please generate integration test scenarios for verifying the midnight-did-resolver.
Focus on the DID document representation and more about serialization logic from the smart contract to the DID document.

An example of interesting scenario could be

- DID contract with valid service endpoint should be serialized to DID document sucessfully.
- DID contract with JubJub key should be serialized to DID document successfully.
- DID contract with deactivated flag should return Gone status

Also consider edge case and zero case.
Verifying the smart contract logic is out of scope.

You may look at one existing test scenario which is implemented in the file @tests/integration-tests/src/index.test.ts

The output should be a markdown file listing the scenarios.
In the scenario, also give a brief example of input and output, but leave out implementation detail.
We just want to evaluate the scenario for implementation later on.


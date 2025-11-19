Investigate the repo `https://github.com/midnightntwrk/midnight-did`.
This is a private repo, so you may need to clone and investigate the sources.
Check if you have already clone it to the directory `./tmp/midnight-did`, if not, clone it to that directory.

You will investigate the source code and learn how to use the typescript package `@midnight-ntwrk/midnight-did-api`.
This is the api we will use to generate the test cases in the @tmp/scenarios.md .

The test should be structured as follow:

- Use `@midnight-ntwrk/midnight-did-api` api to create / update / deactivate the DID as neccessary.
- Wait for the operation to be confirmed on the midnight blockchain.
- Use the `midnight-did-resovler` (this project) to resolve the DID and assert the result with expectation.

This is the type script tests and it will be located at the `./tests/integration-test`.

Please research how to do the test and provide examples on the first 2 scenarios.
These examples should provide useful information for junior QA to implement the rest of the tests.

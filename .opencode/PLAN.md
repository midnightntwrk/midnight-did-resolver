## Task

Refactor component of a `midnight-did-sources` into a new crate `midnight-did-indexer-client`.

## Context

In the crate `midnight-did-sources`, we have the midnight did serde CLI and an indexer client.
Those are different concerns of our application.
We want to split the code related to the `indexer-api` to a new crate.

## Requirement

- A new crate called `midnight-did-indexer-client` is created
- Code related to indexer API client should be moved to the new crate.
- Code related to indexer API client should be removed from the old crate.
- Run compile and test to check the implmentation is compilable and correct

Please ask clarifying questions if you have any


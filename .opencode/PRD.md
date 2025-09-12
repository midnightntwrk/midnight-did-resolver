# Product Requirement Document (PRD): Refactor Indexer API Client into New Crate

## Overview
Refactor the Rust project by moving all code, modules, helpers, models, and tests related to the indexer API client from the `midnight-did-sources` crate into a new crate named `midnight-did-indexer-client`. This will improve code organization and maintainability.

## Goals
- Separate indexer API client code from unrelated components.
- Ensure both crates compile and pass all tests after the refactor.
- Minimize dependencies in the new crate to only those required for the indexer client.

## Functional Requirements
1. **New Crate Creation**
   - Create a new Rust crate named `midnight-did-indexer-client`.
   - Initialize with its own `Cargo.toml`, specifying only the dependencies required for the indexer client.

2. **Code Migration**
   - Move all indexer API client-related code, modules, helpers, models, and tests from `midnight-did-sources` to the new crate.
   - Update module paths and imports as needed.

3. **Reference Cleanup**
   - Remove all references, re-exports, and integrations related to the indexer client from `midnight-did-sources` (including `lib.rs`, test files, and any other affected files).

4. **Dependency Management**
   - Ensure the new crate’s `Cargo.toml` includes only the necessary dependencies for the indexer client.
   - Remove any indexer client-specific dependencies from `midnight-did-sources/Cargo.toml` if they are no longer needed.

5. **Build and Test Verification**
   - Ensure both `midnight-did-sources` and `midnight-did-indexer-client` compile successfully.
   - Ensure all tests pass in both crates after the refactor.

## Non-Functional Requirements
- Maintain existing code style and formatting standards.
- The refactor should not break the build or tests at any point.
- Do not create or update any README or documentation files.
- The new crate is for internal use unless publishing is specified.

## Acceptance Criteria
- [ ] A new crate named `midnight-did-indexer-client` exists with all indexer client-related code and tests.
- [ ] All indexer client code and references are removed from `midnight-did-sources`.
- [ ] Both crates compile and pass all tests.
- [ ] The new crate’s `Cargo.toml` contains only required dependencies.
- [ ] No broken imports, references, or test failures remain after the refactor.

## Implementation Steps
1. Create the new crate.
2. Identify and move all indexer client-related code and tests.
3. Update references and clean up the original crate.
4. Manage dependencies in both crates.
5. Verify build and test success.
6. Final review for code style and completeness.

## Edge Cases & Error Handling
- If any code is shared between the indexer client and other components, clarify whether it should be duplicated, moved to a shared crate, or left in place.
- Ensure all tests related to the indexer client are moved and still function correctly in the new crate.
- Resolve any dependency version conflicts that may arise during the move.

## Open Questions
1. Are there any modules, helpers, or models used by both the indexer client and other components in `midnight-did-sources`? If so, should these be duplicated, moved to a shared crate, or left in place?
2. Are there any tests that cover both the indexer client and other functionality? If so, how should these be handled?
3. Should the new crate be prepared for publishing (e.g., versioning, metadata), or is it strictly for internal use?
4. Is `midnight-did-indexer-client` the final name, or could it change?

---

If you have answers to any open questions, please provide them before implementation begins.

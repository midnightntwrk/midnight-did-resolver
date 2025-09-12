# Task List: Refactor Indexer API Client into New Crate

## Clarifying Questions (Flag for Review Before Implementation)
- [ ] Are there any modules, helpers, or models used by both the indexer client and other components in `midnight-did-sources`? If so, should these be duplicated, moved to a shared crate, or left in place?
- [ ] Are there any tests that cover both the indexer client and other functionality? If so, how should these be handled?
- [ ] Should the new crate be prepared for publishing (e.g., versioning, metadata), or is it strictly for internal use?
- [ ] Is `midnight-did-indexer-client` the final name, or could it change?
- [ ] Confirm if any project-specific coding standards or naming conventions should be applied to the new crate.

---

## Milestone 1: Create New Crate

- [ ] Create a new Rust crate named `midnight-did-indexer-client` in the project root.
    - Input: Crate name, project root path.
    - Output: New directory with `Cargo.toml` and `src/lib.rs`.
    - Acceptance Criteria: Directory and files exist, crate compiles.
- [ ] Initialize `Cargo.toml` with only the dependencies required for the indexer client.
    - Input: List of required dependencies (to be determined from code migration).
    - Output: Minimal `Cargo.toml`.
    - Acceptance Criteria: No unnecessary dependencies present.
- [ ] Verify the new crate compiles (run `cargo build` in the new crate directory).
    - Acceptance Criteria: Build succeeds without errors.

**After completing this section, pause and request feedback before proceeding to the next major section.**
> Feedback template: "Please confirm the new crate structure and dependency setup are correct before continuing to code migration."

---

## Milestone 2: Identify and Migrate Indexer Client Code and Tests

- [ ] Identify all code, modules, helpers, models, and tests related to the indexer API client in `midnight-did-sources`.
    - Input: Source code in `midnight-did-sources/src/indexer_api.rs`, related files, and tests.
    - Output: List of files/modules to move.
    - Acceptance Criteria: All relevant code and tests are identified.
- [ ] For any code shared between indexer client and other components, flag for review and clarify handling (duplicate, move to shared crate, or leave in place).
    - Acceptance Criteria: No ambiguous shared code remains.
- [ ] Move identified code and tests to `midnight-did-indexer-client/src/` and `tests/` as appropriate.
    - Input: List of files/modules.
    - Output: Code and tests in new crate.
    - Acceptance Criteria: All indexer client code and tests are present in the new crate.
- [ ] Update module paths and imports in the moved code to reflect the new crate structure.
    - Input: Moved code.
    - Output: Updated imports and paths.
    - Acceptance Criteria: No broken imports or unresolved references.
- [ ] Verify the new crate compiles and all tests pass (`cargo build` and `cargo test`).
    - Acceptance Criteria: Build and tests succeed.

**After completing this section, pause and request feedback before proceeding to the next major section.**
> Feedback template: "Please confirm all indexer client code and tests have been migrated and function correctly in the new crate."

---

## Milestone 3: Reference Cleanup in Original Crate

- [ ] Remove all references, re-exports, and integrations related to the indexer client from `midnight-did-sources` (including `lib.rs`, test files, and any other affected files).
    - Input: Source code in `midnight-did-sources`.
    - Output: Cleaned codebase with no indexer client references.
    - Acceptance Criteria: No indexer client code or references remain.
- [ ] Remove any indexer client-specific dependencies from `midnight-did-sources/Cargo.toml` if they are no longer needed.
    - Input: Dependency list.
    - Output: Updated `Cargo.toml`.
    - Acceptance Criteria: No unnecessary dependencies remain.
- [ ] Verify `midnight-did-sources` crate compiles and all tests pass (`cargo build` and `cargo test`).
    - Acceptance Criteria: Build and tests succeed.

**After completing this section, pause and request feedback before proceeding to the next major section.**
> Feedback template: "Please confirm all indexer client references and dependencies have been removed from the original crate and it still builds and passes tests."

---

## Milestone 4: Final Dependency and Build Verification

- [ ] Review `Cargo.toml` in `midnight-did-indexer-client` to ensure only required dependencies are present.
    - Acceptance Criteria: No unnecessary dependencies.
- [ ] Review `Cargo.toml` in `midnight-did-sources` to ensure no indexer client-specific dependencies remain.
    - Acceptance Criteria: No unnecessary dependencies.
- [ ] Run `cargo build` and `cargo test` in both crates to verify successful compilation and test passing.
    - Acceptance Criteria: Both crates build and pass all tests.
- [ ] Resolve any dependency version conflicts that arise.
    - Acceptance Criteria: No dependency conflicts remain.

**After completing this section, pause and request feedback before proceeding to the next major section.**
> Feedback template: "Please confirm both crates have correct dependencies and pass all builds/tests."

---

## Milestone 5: Final Review for Code Style and Completeness

- [ ] Review code in both crates for adherence to existing code style and formatting standards (run `cargo fmt` if applicable).
    - Acceptance Criteria: Code style matches project standards.
- [ ] Ensure no broken imports, references, or test failures remain.
    - Acceptance Criteria: No errors or warnings.
- [ ] Do not create or update any README or documentation files.
    - Acceptance Criteria: No changes to documentation.
- [ ] Confirm all acceptance criteria from the PRD are met.

**After completing this section, pause and request final feedback before closing the refactor.**
> Feedback template: "Please confirm the refactor meets all acceptance criteria and is ready for final review/merge."

---

## Edge Case Handling

- [ ] For any shared code between indexer client and other components, clarify and implement the agreed approach (duplicate, move to shared crate, or leave in place).
- [ ] For any tests that cover both indexer client and other functionality, clarify and implement the agreed approach (split, duplicate, or refactor).
- [ ] Resolve any dependency version conflicts during migration.
- [ ] Ensure all migrated tests function correctly in the new crate.

---

## Completion Checklist

- [ ] A new crate named `midnight-did-indexer-client` exists with all indexer client-related code and tests.
- [ ] All indexer client code and references are removed from `midnight-did-sources`.
- [ ] Both crates compile and pass all tests.
- [ ] The new crate’s `Cargo.toml` contains only required dependencies.
- [ ] No broken imports, references, or test failures remain after the refactor.

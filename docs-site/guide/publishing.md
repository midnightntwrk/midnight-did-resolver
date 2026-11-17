# Publishing

The docs site is built with VitePress and is intended to be published as GitHub Pages for this repository.

## Local build

```bash
npm install
npm run docs:build
```

The local preview server uses the root path:

```bash
npm run docs:preview
```

## GitHub Pages behavior

When the docs workflow runs on GitHub Actions, the VitePress `base` path is set automatically from the repository name.

For this repository that means:

- local: `/`
- GitHub Pages: `/midnight-did-resolver/`

The config is implemented in:

- `docs-site/.vitepress/config.ts`

## Deployment workflow

The workflow:

1. checks out the repository
2. installs the Compact toolchain used by generated API documentation
3. installs dependencies with `npm ci`
4. syncs repository markdown into internal docs pages
5. generates API reference from TypeScript entry points
6. builds the site with `npm run docs:build`
7. uploads the VitePress output as a Pages artifact on publishable `main` runs
8. deploys it to GitHub Pages on publishable `main` runs

Workflow file:

- `.github/workflows/docs.yml`

## Repository settings

GitHub Pages must be configured to use the workflow-based deployment model.

Repository configuration:

1. Open `Settings -> Pages`
2. In `Build and deployment`
3. Set `Source` to `GitHub Actions`

Do not use the older `Deploy from a branch` mode for this repository.

Reason:

- the docs site has a build step
- the build also generates:
  - mirrored source markdown pages
  - TypeDoc API reference
- the recommended GitHub Pages flow for this is artifact deployment through Actions, not pushing built files to a `gh-pages` branch

## Branch behavior

- pushes to `main`:
  - build and deploy
- pushes to `develop`:
  - build only; Pages configuration, artifact upload, and deploy are skipped
- pull requests targeting `main` or `develop`:
  - build only; Pages configuration, artifact upload, and deploy are skipped
- `workflow_dispatch`:
  - build on the selected ref
  - deploy only when the selected ref is `main`

This keeps manual docs verification available on branches without accidentally publishing preview content as the production Pages site.

## Docker publication policy

The channel-aware `.github/workflows/release-docker.yml` described here is the
workflow change proposed for `develop` in [#165](https://github.com/midnightntwrk/midnight-did-resolver/issues/165).
It is **not evidence of a release or an already-deployed change to `main`**.
Until separately reviewed promotion installs it on `main`/the default branch,
that branch can still contain the older main-only manual and `v*` tag-push
publisher. Inspect the workflow at the selected ref before planning a release;
do not create a tag or dispatch an older publisher to test this transition.
The manual-only rule applies to refs containing the changed workflow. Existing
tags, images and historical signature identities are not rewritten or removed.

The selected next Docker candidate is **`1.0.0-rc.1` (planned, not published)**,
with root, resolver, manager and secret-storage package base versions `1.0.0`.
This metadata alignment does not authorize candidate or stable publication.
Stable `1.0.0` examples are future illustrative usage only after separate
approval, reviewed `main` promotion and the reconciliation required by #137.

### Inputs and source eligibility

| Channel | Exact allowed source refs | Required full version | Tags on **each** service image |
| --- | --- | --- | --- |
| `rc` | `refs/heads/develop` or `refs/heads/main` | `X.Y.Z-rc.N` | Exact version only; never `latest` |
| `release` | `refs/heads/main` only | `X.Y.Z` | Exact version and `latest` |

Both `channel` and `version` are required. `channel` defaults to `rc` in the UI,
not in runtime validation. There is no implicit version fallback. Base components
must be canonical decimal integers (`0` or a nonzero digit followed by digits).
The RC index must be positive with no leading zero: `1.0.0-rc.1` is valid;
`1.0.0-rc.0`, `1.0.0-rc.01`, `1.0.0-rc1`, `v1.0.0`, whitespace, build metadata
and other prerelease suffixes are rejected. The full version tag must be at most
128 characters. The base (`1.0.0` in this example) must match all four checked-in
versions: root, resolver, manager and secret-storage `package.json`. The docs
workspace is unversioned and excluded. The workflow does not rewrite package
manifests: image tags and OCI version labels use the full publication version;
package manifests retain the matching base version.

`platforms` remains optional, defaulting to `linux/amd64,linux/arm64`; only those
Linux architectures are supported. Keep both unless a separately approved
release plan specifies otherwise. Both images have coherent versions:

- `ghcr.io/midnightntwrk/midnight-did-resolver-service:<version>`
- `ghcr.io/midnightntwrk/midnight-did-manager-service:<version>`

Example commands **for a separately approved release, after the selected ref has
this workflow**, not instructions to publish as part of implementing #165:

```bash
# RC source eligibility; main is also eligible for rc.
gh workflow run release-docker.yml --ref develop -f channel=rc -f version=1.0.0-rc.1 \
  -f expected_source_sha="<approved full source SHA>" -F publish=false

# Future illustrative stable usage, only after separate approval/main promotion.
# Never dispatch release from develop.
gh workflow run release-docker.yml --ref main -f channel=release -f version=1.0.0 \
  -f expected_source_sha="<approved full main SHA>" -F publish=false
```

Dispatch selects an allowed branch, not a tag, arbitrary SHA or nested/lookalike
branch. GitHub records the branch commit as the run's source SHA; checkout and
both local Docker build contexts use **that exact commit**, even if the branch
moves afterwards. Workflow ref/SHA, checked-out HEAD, tracked source integrity,
workspace versions and canonical tags are revalidated at publication/signing
boundaries. Both images receive OCI revision/source/version labels. The run
summary records source ref/SHA, channel, version, platforms and both final index
digests. Inspect the recorded SHA and required checks against the reviewed
release plan; branch eligibility alone is not authorization.

For refs containing the new workflow, branch pushes, merges and `v*` tag pushes
cannot trigger Docker publication. Ordinary CI, security scanning and the
separate Pages workflow retain their existing triggers. Git tags are no longer
a publication mechanism in the changed workflow; any release-record/tag policy
remains a separately authorized concern.

### Staged digest publication (implementation under review)

The staged workflow supersedes the earlier sequential-build description above.
It is not publication evidence or stable-release approval. Dispatch requires
`expected_source_sha`, independently checked against the approved commit; it
must equal the actual event/workflow/checkout SHA. `publish` defaults to
`false`: this stages and verifies a new pair but cannot create final image tags
or publish a GitHub record. An explicit `true` builds and verifies its own pair;
it does not silently reuse an earlier experimental pair.

The DAG is validate-dispatch → stage-and-verify → record-prepare → promote →
record-finalize, with one constant publication concurrency namespace and no
active-run cancellation. Builds use the literal canonical remote Git repository
at that exact SHA, digest-only OCI exports (no version/latest build tags), pinned
Buildx v0.30.1 and BuildKit v0.25.1, maximum v0.2 provenance and SBOMs.
The privileged helpers use only Node stdlib; dependency/YAML tests remain in
ordinary CI. Publisher jobs do not install npm dependencies or use dependency
caches. Buildx's automatic build-record export is disabled; only explicitly
named evidence bundles cross publication jobs.

Both service indexes must have verified raw descriptor/blob/source/subject
chains, four fresh complete Trivy inventories (official checksum-pinned
v0.76.2), zero HIGH/CRITICAL and unresolved UNKNOWN findings including unfixed
vulnerabilities, four actual digest-pinned runtime lifecycles, and literal
keyless signature identity/SHA/event verification. The full scan JSON retains
all original classifications. The source-reviewed, seven-day advisory policy
resolves only two exact UNKNOWN glibc rows to their authoritative CNA LOW/MEDIUM
scores; it never downgrades a scanner HIGH/CRITICAL finding. Policy bytes, raw
primary records, checksums and per-row resolution receipts are attached alongside
the unchanged scans. Both advisories remain affected/unfixed with unverified
service reachability; this is classification evidence, not a vulnerability waiver
or a claim of non-exploitability. Missing, changed or expired classifications fail
closed. A Node policy failure is the nonzero publication exit gate.
Signatures authenticate the root indexes; unsigned BuildKit statements are not
passed to `cosign verify-attestation`.

After both graphs and all four scans pass, runtime automatically provisions an
owned standalone node 2.1.101, indexer 4.4.101 and proof server 9.0.1 when no
override is supplied. Official dependency index digests are pinned in
`scripts/release-standalone-fixture.mjs`. Generated JSON Compose has only those
three dependencies, dynamic loopback ports, exact standalone presets/health
checks and no service builds. A unique project/file scopes every operation;
bounded startup has no whole-project retry, and finally retains status/cleanup
evidence and removes only that owned project and volumes.

A read-only, source-hashed helper runs inside the verified manager amd64 child
with public candidate package exports, not host workspace dependencies. It uses
only the owned standalone's deterministic public genesis fixture, registers
Dust, configures providers, deploys a DID (at most two 8-second Dust-only
retries), adds a known public verification method and obtains a positive
reference document through the owning Ledger9 public resolver/provider.
Wallet stop is in finally; the external chain remains alive through all four
service/platform consumers. No public network funds or source build fallback
is allowed. The helper prints only sanitized fixture JSON, never seeds/keys.

An explicit external override remains available as a **public, nonsecret**
`runtime_fixture` JSON object:
`{schemaVersion:1,kind:"standalone-external-state",did,expectedResolution,indexerHTTP,indexerWS}`.
It additionally requires `external_fixture_approved=true`. The expected
resolution must contain the same positive `didDocument.id`, a known public
method and null resolution error. Indexer endpoints must be loopback HTTP/WS.
Missing/empty/error reference results never become passing fixtures.
The runner is native amd64; arm64 results are QEMU, not native certification.
Both services bind only loopback using Linux host networking. Tests inspect
Node 24.22.3, uid 1000, process architecture, health, readiness, OpenAPI 1.0.0
and Swagger assets. Manager selection/list metadata must survive restart and
remove/recreate on the **same owned mount and default data directory**.
Resolver repeats positive resolution against external state; it has no local
filesystem persistence claim. Each exact child is pulled with its explicit
platform before `digestPullVerified/pulledReference` are recorded. A separate
read-only mounted, source-hashed candidate-native probe executes HD4 role-4
derivation, owning Ledger9/WASM/public API loading, ClassicLevel
open/put/get/close/reopen, manager storage sign/verify, packaged proving assets
and the public Ledger9 executable-runtime configuration/version guard under
Node 24.22.3 and uid 1000. No host npm install or producer/private imports occur.
These probes supplement—not replace—HTTP/persistence assertions. Full manager
funded session/contract restoration and native-arm64 guarantees remain
separate, unproven coverage.

For `publish=true`, the parent/maintainer must pre-provision the exact-source
Git tag `v<version>` and a matching draft prerelease **after the final source
is reviewed**. The workflow never creates them and never receives a PAT or
imaginary `workflows:write` permission. GitHub's workflow-file-difference
restriction can otherwise prevent record creation with `GITHUB_TOKEN`.
The contents-write-only prepare job validates this record and uploads
run/attempt/source-bound, checksummed prepared assets before any final image tag.
Unknown API errors are failures, not evidence of absence or permission.

Promotion revalidates all prepared evidence and both signatures, checks both
version tags absent before the pair and each own write, then uses the core
client's same-repository raw-index PUT and digest readback. No rebuild,
overwrite, automatic retry, deletion or rollback is allowed. GHCR has no
two-repository transaction or portable create-if-absent guarantee: a partial
or ambiguous result remains an incomplete draft with read-only tag
reconciliation; owner-directed recovery is separate. A record-only recovery
must not rerun builds or tag writes. RC never writes `latest`.

The final contents-write/packages-read job independently verifies both tags,
signatures and source tag, uploads final checksums/receipts, and only then
publishes with `prerelease:true, make_latest:"false"`. GitHub assets are **not
intrinsically immutable**; preserve the hash manifests and approved previous
rollback digest pair. Rollback changes deployment digests, never version tags.

For separately approved stable publication, main-only/full prepared gates
remain mandatory. Both immutable version tags and exact-byte/signature
readbacks must pass **before either latest update**. Only then may the guarded
core `promoteStableLatest` operation update each fixed latest pointer, once.
Previous latest digests and partial/ambiguous outcomes are recorded; no
automatic rollback/retry/overwrite of version tags occurs. Before each latest
write both immutable versions/signatures are checked again. Finalization
independently rechecks both version and latest pairs. RC performs no latest
registry calls and always uses GitHub `make_latest:"false"`. This preserves
stable policy, not stable publication approval or a cross-repository transaction.

**Remaining real-experiment/integration blockers:** standalone startup and SDK
deployment, actual BuildKit exporter/provenance shape, scanner schema/DB
coverage, both-platform native probes/runtime persistence and GHCR exact-byte/
signature/latest behavior are not established by offline recording fixtures.
Core's approved Node bookworm base policy is not weakened to accept a different
experimental runtime variant.
The full combined final-source checks, source approval, protections/P1
requirements and separately approved stable/history work remain mandatory.

### Stable promotion and approval prerequisites

`develop` RC eligibility removes the need to promote unfinished work merely to
obtain an RC image. It does **not** replace the stable `develop` → `main`
review/promotion process or the reconciliation decision tracked in
[#137](https://github.com/midnightntwrk/midnight-did-resolver/issues/137).
Before stable approval, deliberately reconcile `main` history and changelog/release
metadata with the candidate line; review the exact resulting `main` commit and
retain evidence for both images. Do not equate a tested develop SHA with a later
squashed/merged main SHA. #137 remains open for the durable promotion strategy
and outstanding reconciliation; this workflow change neither chooses a merge
method nor performs that reconciliation.

Publication eligibility is not release approval. Preserve signed commits,
code-owner/release reviews, branch protections, registry permissions and all
security gates. The current baseline workflow declares no protected release
environment; adding one or changing eligible branches/reviewers requires separate
authorization. If an environment is introduced separately, it must explicitly
permit the intended source branch **and** enforce its release approvals; never
bypass it to obtain a develop RC.

The broader go/no-go obligations in
[#109](https://github.com/midnightntwrk/midnight-did-resolver/issues/109) and
[#92](https://github.com/midnightntwrk/midnight-did-resolver/issues/92) still apply:
reviewed/reconciled source, both-platform vulnerability scans and explicit risk
acceptances, immutable exact-version policy, staged/atomic two-service publication,
verified signatures/SBOM/provenance, digest-pinned runtime/restart/persistence
checks, and an authoritative release evidence/rollback record. The current
sequential builds/signing are **not atomic**, do not enforce exact-tag immutability,
and can leave partial or unsigned publication on failure; do not blindly rerun
or overwrite a version. This bounded policy change does not complete those
controls, waive their gates or authorize candidate/stable publication. Binary/artifact
scope in [#40](https://github.com/midnightntwrk/midnight-did-resolver/issues/40)
is also separate. Passing non-publishing tests is not publication evidence.

## Container image verification

Both image builds retain maximum provenance and SBOM attestations; the workflow
keylessly signs and verifies their multi-architecture index digests. For a new
manual publication, use the **literal workflow identity for its authorized source
ref**, the exact source SHA and GitHub Actions issuer. For example, a develop RC:

```bash
SOURCE_SHA='<full source SHA from the approved run>'
IDENTITY='https://github.com/midnightntwrk/midnight-did-resolver/.github/workflows/release-docker.yml@refs/heads/develop'

cosign verify \
  "ghcr.io/midnightntwrk/midnight-did-resolver-service@sha256:<resolver-digest>" \
  --certificate-identity "$IDENTITY" \
  --certificate-oidc-issuer 'https://token.actions.githubusercontent.com' \
  --certificate-github-workflow-sha "$SOURCE_SHA" \
  --certificate-github-workflow-trigger workflow_dispatch

cosign verify \
  "ghcr.io/midnightntwrk/midnight-did-manager-service@sha256:<manager-digest>" \
  --certificate-identity "$IDENTITY" \
  --certificate-oidc-issuer 'https://token.actions.githubusercontent.com' \
  --certificate-github-workflow-sha "$SOURCE_SHA" \
  --certificate-github-workflow-trigger workflow_dispatch
```

For an approved main RC or stable run, use exactly:

```bash
IDENTITY='https://github.com/midnightntwrk/midnight-did-resolver/.github/workflows/release-docker.yml@refs/heads/main'
```

Then run both verification commands with that identity, that run's exact source
SHA and digests. Do not broaden the identity to trust arbitrary branches/tags.
The pinned installer selects Cosign v2.6.1, which supports these SHA/event checks.
Verify the published attestations and run both digest-pinned services separately
before deployment; signature verification alone does not prove release readiness.
Historical artifacts need their actual historical identity/evidence and must not
be presented as newly signed or validated by this implementation.

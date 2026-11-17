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
gh workflow run release-docker.yml --ref develop -f channel=rc -f version=1.0.0-rc.1

# Future illustrative stable usage, only after separate approval/main promotion.
# Never dispatch release from develop.
gh workflow run release-docker.yml --ref main -f channel=release -f version=1.0.0
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

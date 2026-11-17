FROM node:24.22.3-bookworm-slim@sha256:dc26ae92c9fb15cf6b1d63e20f0944b2efe0f115b20e00be48fb23fceb061c54 AS builder

WORKDIR /workspace

# Build from this repository root.
# DID dependencies are consumed from the public npmjs registry.
COPY . /workspace/midnight-did-resolver

RUN npm ci --ignore-scripts --prefix /workspace/midnight-did-resolver

RUN npm run build -w @midnight-ntwrk/midnight-did-resolver-service --prefix /workspace/midnight-did-resolver

RUN npm prune --omit=dev --ignore-scripts --prefix /workspace/midnight-did-resolver

# Keep the pruned production dependency superset; export only built workspaces.
RUN node /workspace/midnight-did-resolver/scripts/export-runtime.mjs export /workspace/midnight-did-resolver /runtime did-resolver-service

# Empty home only: no checkout state, credentials or sessions are copied.
RUN mkdir -m 0750 /runtime-home

FROM gcr.io/distroless/nodejs24-debian13:nonroot@sha256:6abfcb367ebc291572c774c1592a986be2ab26a2486f73093401327495a5962a AS runtime

# Distroless has no shell or package managers; services use packaged artifacts.
# Keep bare-node healthchecks/Compose consumers and the existing exec CMD.
ENV PATH=/nodejs/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin
ENV HOME=/home/node
ENTRYPOINT []
COPY --from=builder --chown=1000:1000 /runtime-home/ /home/node/

WORKDIR /workspace
ENV NODE_ENV=production
# Containers must bind beyond loopback or the published port is unreachable
# (config defaults RESOLVER_HOST to 127.0.0.1 for local dev).
ENV RESOLVER_HOST=0.0.0.0

COPY --from=builder --chown=1000:1000 /runtime/ /workspace/midnight-did-resolver/

EXPOSE 3001

HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
  CMD ["node", "-e", "fetch('http://127.0.0.1:3001/health').then(r => process.exit(r.ok ? 0 : 1)).catch(() => process.exit(1))"]

# Numeric identity preserves mount ownership; distroless has no named node user.
USER 1000:1000
WORKDIR /workspace/midnight-did-resolver/did-resolver-service
CMD ["node", "--experimental-specifier-resolution=node", "dist/index.js"]

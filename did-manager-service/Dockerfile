FROM node:24-bookworm-slim AS builder

WORKDIR /workspace

# Build from this repository root.
# DID dependencies are consumed from the public npmjs registry.
COPY . /workspace/midnight-did-resolver

RUN npm ci --ignore-scripts --prefix /workspace/midnight-did-resolver

RUN npm run build -w @midnight-ntwrk/midnight-did-manager-service --prefix /workspace/midnight-did-resolver


FROM node:24-bookworm-slim AS runtime

WORKDIR /workspace
ENV NODE_ENV=production
# Containers must bind beyond loopback so deployed services are reachable
ENV DID_MANAGER_HOST=0.0.0.0

COPY --from=builder /workspace/midnight-did-resolver /workspace/midnight-did-resolver

EXPOSE 3010

HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
  CMD ["node", "-e", "fetch('http://127.0.0.1:3010/health').then(r => process.exit(r.ok ? 0 : 1)).catch(() => process.exit(1))"]

USER node
WORKDIR /workspace/midnight-did-resolver/did-manager-service
CMD ["node", "--experimental-specifier-resolution=node", "dist/index.js"]

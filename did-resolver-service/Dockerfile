FROM node:24-bookworm-slim AS builder

WORKDIR /workspace

# This Dockerfile expects to be built from the midnight-identity-workspace root.
# DID dependencies are consumed from tarballs already copied into
# midnight-did-resolver/libs/midnight-did by the workspace sync script.
COPY midnight-did-resolver /workspace/midnight-did-resolver

RUN npm ci --ignore-scripts --prefix /workspace/midnight-did-resolver
# Temporary pin keeps the resolver build compatible with the workspace local runtime.
RUN npm install --ignore-scripts --no-save --prefix /workspace/midnight-did-resolver @midnight-ntwrk/wallet-sdk-address-format@3.1.0

RUN npm run build -w @midnight-ntwrk/midnight-did-resolver-service --prefix /workspace/midnight-did-resolver


FROM node:24-bookworm-slim AS runtime

WORKDIR /workspace
ENV NODE_ENV=production

COPY --from=builder /workspace/midnight-did-resolver /workspace/midnight-did-resolver

EXPOSE 3001

USER node
WORKDIR /workspace/midnight-did-resolver/did-resolver-service
CMD ["node", "--experimental-specifier-resolution=node", "dist/index.js"]

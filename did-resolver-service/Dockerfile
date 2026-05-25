FROM node:24-bookworm-slim AS builder

WORKDIR /workspace

# This Dockerfile expects to be built from the midnight-identity-workspace root,
# where midnight-did and midnight-did-resolver are siblings.
COPY midnight-did-resolver /workspace/midnight-did-resolver
COPY midnight-did /workspace/midnight-did

RUN npm ci --ignore-scripts --prefix /workspace/midnight-did-resolver
RUN npm ci --ignore-scripts --prefix /workspace/midnight-did
# Temporary pin keeps the resolver build compatible with the workspace local runtime.
RUN npm install --ignore-scripts --no-save --prefix /workspace/midnight-did-resolver @midnight-ntwrk/wallet-sdk-address-format@3.1.0
# Temporary workaround for class identity checks between separate runtime package copies.
RUN rm -rf /workspace/midnight-did-resolver/node_modules/@midnight-ntwrk/onchain-runtime-v3 \
  && mkdir -p /workspace/midnight-did-resolver/node_modules/@midnight-ntwrk \
  && ln -s /workspace/midnight-did/node_modules/@midnight-ntwrk/onchain-runtime-v3 \
    /workspace/midnight-did-resolver/node_modules/@midnight-ntwrk/onchain-runtime-v3

RUN npm run build -w @midnight-ntwrk/midnight-did-resolver-service --prefix /workspace/midnight-did-resolver


FROM node:24-bookworm-slim AS runtime

WORKDIR /workspace
ENV NODE_ENV=production

COPY --from=builder /workspace/midnight-did-resolver /workspace/midnight-did-resolver
COPY --from=builder /workspace/midnight-did /workspace/midnight-did

EXPOSE 3001

USER node
WORKDIR /workspace/midnight-did-resolver/did-resolver-service
CMD ["node", "--experimental-specifier-resolution=node", "dist/index.js"]

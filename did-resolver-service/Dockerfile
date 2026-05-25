FROM node:24-bookworm-slim AS builder

WORKDIR /workspace

# This Dockerfile expects to be built from the midnight-identity-workspace root,
# where midnight-did and midnight-did-resolver are siblings.
COPY midnight-did-resolver/package.json midnight-did-resolver/package-lock.json /workspace/midnight-did-resolver/
COPY midnight-did-resolver/did-resolver-service/package.json /workspace/midnight-did-resolver/did-resolver-service/
COPY midnight-did/package.json /workspace/midnight-did/
COPY midnight-did/package-lock.json /workspace/midnight-did/
COPY midnight-did/packages/jubjub-schnorr/package.json /workspace/midnight-did/packages/jubjub-schnorr/
COPY midnight-did/packages/contract/package.json /workspace/midnight-did/packages/contract/
COPY midnight-did/packages/domain/package.json /workspace/midnight-did/packages/domain/
COPY midnight-did/packages/did/package.json /workspace/midnight-did/packages/did/
COPY midnight-did/packages/api/package.json /workspace/midnight-did/packages/api/

RUN npm ci --ignore-scripts --prefix /workspace/midnight-did-resolver
RUN npm ci --ignore-scripts --prefix /workspace/midnight-did
# Temporary pin keeps the resolver build compatible with the workspace local runtime.
RUN npm install --ignore-scripts --no-save --prefix /workspace/midnight-did-resolver @midnight-ntwrk/wallet-sdk-address-format@3.1.0
# Temporary workaround for class identity checks between separate runtime package copies.
RUN rm -rf /workspace/midnight-did-resolver/node_modules/@midnight-ntwrk/onchain-runtime-v3 \
  && mkdir -p /workspace/midnight-did-resolver/node_modules/@midnight-ntwrk \
  && ln -s /workspace/midnight-did/node_modules/@midnight-ntwrk/onchain-runtime-v3 \
    /workspace/midnight-did-resolver/node_modules/@midnight-ntwrk/onchain-runtime-v3

COPY midnight-did /workspace/midnight-did
COPY midnight-did-resolver/did-resolver-service /workspace/midnight-did-resolver/did-resolver-service

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

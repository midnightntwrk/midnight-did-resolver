FROM node:24-bookworm-slim AS builder

WORKDIR /app

COPY package.json pnpm-lock.yaml pnpm-workspace.yaml ./
COPY apps/did-resolver-service/package.json apps/did-resolver-service/package.json
COPY packages/secret-storage/package.json packages/secret-storage/package.json
COPY jubjub-schnorr/package.json jubjub-schnorr/package.json
COPY contract/package.json contract/package.json
COPY domain/package.json domain/package.json
COPY did/package.json did/package.json

RUN corepack enable && pnpm install --frozen-lockfile --ignore-scripts

COPY packages/secret-storage packages/secret-storage
COPY jubjub-schnorr jubjub-schnorr
COPY contract contract
COPY domain domain
COPY did did
COPY apps/did-resolver-service apps/did-resolver-service

RUN pnpm --filter contract build \
  && pnpm --filter domain build \
  && pnpm --filter did build \
  && pnpm --filter @midnight-ntwrk/midnight-did-resolver-service build


FROM node:24-bookworm-slim AS runtime

WORKDIR /app
ENV NODE_ENV=production

COPY package.json pnpm-lock.yaml pnpm-workspace.yaml ./
COPY apps/did-resolver-service/package.json apps/did-resolver-service/package.json
COPY packages/secret-storage/package.json packages/secret-storage/package.json
COPY jubjub-schnorr/package.json jubjub-schnorr/package.json
COPY contract/package.json contract/package.json
COPY domain/package.json domain/package.json
COPY did/package.json did/package.json

RUN corepack enable && pnpm install --frozen-lockfile --ignore-scripts --prod

COPY --from=builder /app/packages/secret-storage/dist packages/secret-storage/dist
COPY --from=builder /app/jubjub-schnorr/dist jubjub-schnorr/dist
COPY --from=builder /app/contract/dist contract/dist
COPY --from=builder /app/domain/dist domain/dist
COPY --from=builder /app/did/dist did/dist
COPY --from=builder /app/apps/did-resolver-service/dist apps/did-resolver-service/dist

EXPOSE 3001

USER node
WORKDIR /app/apps/did-resolver-service
CMD ["node", "--experimental-specifier-resolution=node", "dist/index.js"]

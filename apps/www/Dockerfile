# syntax=docker/dockerfile:1
FROM node:24-bookworm-slim AS dependencies
RUN npm install --global pnpm@11.17.0
WORKDIR /app/apps/www
COPY apps/www/package.json apps/www/pnpm-lock.yaml apps/www/pnpm-workspace.yaml ./
RUN pnpm install --frozen-lockfile

FROM dependencies AS builder
COPY apps/www/ ./
COPY shared/ /app/shared/
RUN pnpm build

FROM dependencies AS production-dependencies
RUN pnpm prune --prod

FROM node:24-bookworm-slim AS runtime
ENV NODE_ENV=production \
    HOST=0.0.0.0 \
    PORT=4321
WORKDIR /app
COPY --from=production-dependencies --chown=node:node /app/apps/www/node_modules ./node_modules
COPY --from=builder --chown=node:node /app/apps/www/dist ./dist
COPY --from=builder --chown=node:node /app/apps/www/package.json ./package.json
USER node
EXPOSE 4321
CMD ["node", "./dist/server/entry.mjs"]

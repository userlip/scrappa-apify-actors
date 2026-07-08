FROM apify/actor-node:20 AS build

WORKDIR /usr/src/app

COPY --chown=myuser:myuser . ./

RUN npm install --include=dev \
    && npm run build \
    && npm prune --omit=dev

FROM apify/actor-node:20

WORKDIR /usr/src/app

COPY --from=build --chown=myuser:myuser /usr/src/app/package*.json ./
COPY --from=build --chown=myuser:myuser /usr/src/app/node_modules ./node_modules
COPY --from=build --chown=myuser:myuser /usr/src/app/dist ./dist

CMD npm run start

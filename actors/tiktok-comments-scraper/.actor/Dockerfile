FROM apify/actor-node:20

COPY --chown=myuser:myuser . ./

RUN npm install --include=dev \
    && npm run build \
    && npm prune --omit=dev

CMD npm run start

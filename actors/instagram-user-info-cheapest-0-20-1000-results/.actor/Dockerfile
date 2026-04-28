FROM apify/actor-node:20

COPY package*.json ./

RUN npm --quiet set progress=false \
    && npm ci --omit=dev --omit=optional \
    && npm cache clean --force

COPY . ./

CMD npm start --silent

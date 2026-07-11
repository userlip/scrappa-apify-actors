FROM apify/actor-node:22
COPY package*.json ./
RUN npm --quiet set progress=false && npm install --omit=dev --include=dev
COPY . ./
RUN npm run build
CMD npm start --silent

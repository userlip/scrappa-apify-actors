# Google News Scraper

Apify actor for Scrappa's Google News API.

Use `queries` to run multiple keyword searches in one Apify run. Legacy single-query `q` and token-based requests still work for topic, publication, section, story, and Knowledge Graph pages.

## Development

```bash
npm install
npm test
```

## Run locally

```bash
SCRAPPA_API_KEY=... npm run build
SCRAPPA_API_KEY=... apify run --input='{"queries":["artificial intelligence","climate technology"],"gl":"us","hl":"en","page":1}'
```

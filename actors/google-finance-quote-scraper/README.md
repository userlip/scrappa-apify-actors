# Google Finance Quote Scraper

Apify actor for Scrappa's Google Finance quote API.

## Development

```bash
npm install
npm test
```

## Run locally

```bash
SCRAPPA_API_KEY=... npm run build
SCRAPPA_API_KEY=... apify run --input='{"symbol":"AAPL","exchange":"NASDAQ","period_type":"quarterly","hl":"en","gl":"us"}'
```

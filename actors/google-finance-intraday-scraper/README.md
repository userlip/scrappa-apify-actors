# Google Finance Intraday Scraper

Apify actor for Scrappa's Google Finance intraday API.

## Development

```bash
npm install
npm test
```

## Run locally

```bash
SCRAPPA_API_KEY=... npm run build
SCRAPPA_API_KEY=... apify run --input='{"symbols":[{"symbol":"AAPL","exchange":"NASDAQ"}],"hl":"en","gl":"us"}'
```

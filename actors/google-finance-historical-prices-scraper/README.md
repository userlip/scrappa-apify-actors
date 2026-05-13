# Google Finance Historical Prices Scraper

Apify actor for Scrappa's Google Finance historical prices API.

## Development

```bash
npm install
npm test
```

## Run locally

```bash
SCRAPPA_API_KEY=... npm run build
SCRAPPA_API_KEY=... apify run --input='{"symbol":"AAPL","exchange":"NASDAQ","range":6,"interval":"daily","hl":"en","gl":"us"}'
```

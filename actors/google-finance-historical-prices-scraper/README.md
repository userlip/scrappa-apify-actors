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

Use preset `range` input for the most stable historical data. Scrappa also exposes `start_date` and `end_date`; if a custom date range returns no data, the actor exits successfully with an empty dataset and writes the no-data details to `OUTPUT`.

# Google Trends Interest Scraper

Apify actor for Scrappa's Google Trends interest-over-time API.

## Development

```bash
npm install
npm test
```

## Run locally

```bash
SCRAPPA_API_KEY=... npm run build
SCRAPPA_API_KEY=... apify run --input='{"q":"tesla","geo":"US","time_range":"1y","hl":"en","search_type":"web"}'
```

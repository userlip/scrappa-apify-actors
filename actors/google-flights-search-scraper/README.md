# Google Flights Search Scraper

Apify actor for Scrappa's Google Flights one-way and round-trip APIs.

## Development

```bash
npm install
npm test
```

## Run locally

```bash
SCRAPPA_API_KEY=... npm run build
SCRAPPA_API_KEY=... apify run --input='{"trip_type":"one_way","origin":"JFK","destination":"LAX","departure_date":"2026-09-15","adults":1,"cabin_class":"economy","currency":"USD","hl":"en","gl":"us"}'
```

# Google Flights Search Scraper

Apify actor for Scrappa's Google Flights one-way and round-trip APIs.

## Development

```bash
cargo test --locked
```

## Run locally

```bash
docker build -f .actor/Dockerfile -t google-flights-search-scraper .
SCRAPPA_API_KEY=... apify run --input='{"trip_type":"one_way","origin":"JFK","destination":"LAX","departure_date":"45 days","adults":1,"cabin_class":"economy","currency":"USD","hl":"en","gl":"us"}'
```

Set `SCRAPPA_API_KEY` in the local environment for local runs and in the Actor environment for Apify runs. The actor calls Scrappa's Google Flights API, writes one dataset item per flight, and saves the raw API response to the key-value store record `OUTPUT`.

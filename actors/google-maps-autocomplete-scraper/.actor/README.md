# Google Maps Autocomplete

Get autocomplete suggestions from Google Maps through Scrappa's Maps API.

## Input

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `query` | string | Yes | Partial search term, such as `time sq` or `starbucks new` |

The Apify input form prefills `query` with `new york`.

## Output

Each object returned in Scrappa's `suggestions` array is written unchanged as a separate default dataset item. The full upstream response is also stored in the default key-value store under `OUTPUT`.

The actor makes one request for the query. The Maps autocomplete response has no page loop; if Scrappa includes pagination metadata, it remains in `OUTPUT`.

## API key and request behavior

Configure `SCRAPPA_API_KEY` in the actor's environment variables. The actor sends it in the `X-API-Key` header to `https://scrappa.co/api/maps/autocomplete` with the input query. Scrappa requests have a 60-second deadline and are not retried; HTTP, validation, and timeout errors fail the run. Apify storage and run API calls have a 360-second request timeout and retry network errors, HTTP 429, and 5xx responses up to eight times with exponential backoff starting at 500 ms.

## PAY_PER_EVENT budget

Dataset output uses Apify's default `apify-default-dataset-item` event, so each saved suggestion is charged as one result under PAY_PER_EVENT pricing. The actor reads current event prices and charged event counts, then saves only the affordable prefix under `maxTotalChargeUsd`. It keeps the full Scrappa response in `OUTPUT`, including suggestions omitted from the dataset by the spending limit.

## Local development

Run focused tests with `cargo test --locked`. Build the actor image from this directory with:

```sh
docker build -f .actor/Dockerfile -t google-maps-autocomplete-scraper .
```

# YouTube Search Suggestions Scraper

Get YouTube autocomplete search suggestions for a query. The actor calls Scrappa's YouTube suggestions endpoint and stores each suggestion as a separate dataset item for clean exports and usage-aligned billing.

## Input

```json
{
  "q": "javascript",
  "hl": "en",
  "gl": "US"
}
```

## Output

```json
{
  "query": "javascript",
  "suggestion": "javascript tutorial",
  "position": 4,
  "hl": "en",
  "gl": "US"
}
```

## Notes

- `q` is required.
- `hl` is prefilled as `en` in the Apify input form and sets the YouTube interface language code.
- `gl` is prefilled as `US` in the Apify input form and sets the country code used for localized suggestions.
- This actor uses Scrappa's public legacy YouTube suggestions endpoint, so it does not require `SCRAPPA_API_KEY`.
- On pay-per-event runs, the actor reads the run's current event charges and saves only the affordable prefix of suggestions under `maxTotalChargeUsd`; the fetched count can therefore exceed the saved count.

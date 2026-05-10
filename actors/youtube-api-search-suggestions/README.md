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
- `hl` defaults to `en` in the Apify input form and sets the YouTube interface language code.
- `gl` defaults to `US` in the Apify input form and sets the country code used for localized suggestions.
- This actor uses Scrappa's public legacy YouTube suggestions endpoint, so it does not require `SCRAPPA_API_KEY`.

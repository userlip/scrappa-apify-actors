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
- `hl` is the YouTube interface language code.
- `gl` is the country code used for localized suggestions.

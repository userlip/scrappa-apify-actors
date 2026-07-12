# Google Hotels Autocomplete Scraper

Discover destinations and resolve hotel names before running a full Google Hotels search. Submit multiple queries in one run; the Actor saves one dataset row per unique suggestion for each source query.

## Example input

```json
{
  "queries": ["Berlin", "Paris hotels"],
  "gl": "de",
  "hl": "en",
  "currency": "EUR",
  "type": "all"
}
```

`queries` also accepts a comma-separated string through the API. Singular `q` is supported for compatibility. Inputs are trimmed and case-insensitively deduplicated.

## What you get

- Ranked location and accommodation suggestions
- Normalized autocomplete values and suggestion types
- Optional hotel `property_token` values and thumbnails
- Ready-to-use Scrappa Google Hotels search links
- The source query and localization settings on every row

Use location rows for destination discovery. Pass an accommodation row's `property_token` to the Google Hotels Search Scraper to resolve that exact property. Optional upstream fields are returned as `null` when unavailable.

Results cost **$0.00025 each** through the `hotel-suggestion-result` event. Only successfully saved dataset rows are charged. A single `OUTPUT` record summarizes partial query failures; there are no per-result key-value-store writes.

This Actor is a thin wrapper around `GET https://scrappa.co/api/google-hotels/autocomplete`. For higher-volume usage, [upgrade to Scrappa's direct API](https://scrappa.co) to avoid Apify run overhead and connect autocomplete directly to the Google Hotels Search endpoint.

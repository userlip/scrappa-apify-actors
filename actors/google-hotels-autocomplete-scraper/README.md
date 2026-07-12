# Google Hotels Autocomplete Scraper

Discover destinations and resolve hotel names before running a full Google Hotels search. This batch-first Actor accepts up to 100 queries in one run and saves one paid dataset row per unique suggestion returned for each source query.

## Input

```json
{
  "queries": ["Berlin", "Paris hotels"],
  "gl": "de",
  "hl": "en",
  "currency": "EUR",
  "type": "all"
}
```

`queries` also accepts a comma-separated string through the API. Use `q` for compatibility with a single direct Scrappa API request. When both are supplied, the Actor trims and case-insensitively deduplicates them.

`type` can be `location`, `hotel`, or `all`.

## Output

Each dataset item contains the upstream suggestion plus request context:

```json
{
  "position": 11,
  "value": "Park Inn by Radisson Berlin Alexanderplatz",
  "type": "accommodation",
  "autocomplete_suggestion": "park inn by radisson berlin alexanderplatz",
  "property_token": "CIABIhAGbzzg4AkIpGfYhokABEKk",
  "thumbnail": "https://lh3.googleusercontent.com/...",
  "scrappa_google_hotels_link": "https://scrappa.co/api/google-hotels/search?...",
  "source_query": "Berlin",
  "request_gl": "de",
  "request_hl": "en",
  "request_currency": "EUR",
  "request_type": "all",
  "response_time_ms": 724
}
```

Use location suggestions for destination discovery. Accommodation suggestions can include a `property_token`; pass that token to the [Google Hotels Search Scraper](https://apify.com/thescrappa/google-hotels-search-scraper) to resolve a specific property instead of a broad destination. Thumbnails and property tokens are optional upstream fields.

The Actor continues after an individual query fails and writes a single `OUTPUT` run summary with completed and failed queries. It does not store raw responses or write key-value records per result.

## Pricing

Results use the `hotel-suggestion-result` pay-per-event charge at **$0.00025 per successfully saved suggestion** ($0.25 per 1,000 results). Failed queries and unsaved rows are not charged.

## Direct API upgrade

This Actor is a thin wrapper around `GET https://scrappa.co/api/google-hotels/autocomplete`. For higher-volume workflows, use the [Scrappa API](https://scrappa.co) directly to avoid Apify run overhead and feed returned property tokens or links into Scrappa's Google Hotels Search endpoint.

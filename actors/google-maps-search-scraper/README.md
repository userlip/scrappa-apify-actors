# Google Maps Search Scraper

Search Google Maps for local places with names, categories, star ratings and addresses. Enter a business or place query and set the region when you need locally focused results.

## What data can you extract?

Place and route details follow the public Google Maps page; optional ratings, links and photos may not be shown for every record.

| Field | Type | Description |
| --- | --- | --- |
| `name` | text | Name of the Google Maps place, as shown by Google Maps; null when no name is published. |
| `type` | text | Category assigned to the Google Maps place by Google Maps; null when Google Maps does not provide the value. |
| `rating` | number | Rating for this Google Maps place, on a 1-to-5 star scale; null when no score is shown. |
| `review_count` | number | Number of reviews shown by Google Maps, as a whole number; zero is possible, and null means no count was reported. |
| `full_address` | text | Full address shown for the Google Maps place by Google Maps, in the format used by the source; null when it is omitted. |
| `latitude` | number | Latitude for this Google Maps place on Google Maps, in decimal degrees; null when the source provides no coordinates. |
| `longitude` | number | Longitude for this Google Maps place on Google Maps, in decimal degrees; null when the source provides no coordinates. |
| `phone_numbers` | array of text | List of phone numbers associated with this Google Maps place on Google Maps; empty when the source returns no entries. |
| `website` | link | Website url for this Google Maps place on Google Maps; null when the source does not provide a URL. |
| `business_id` | text | Google Maps place ID for the Google Maps place, assigned by Google Maps; null when the source does not expose it. |
| `place_id` | text | Google Maps place ID for the Google Maps place, assigned by Google Maps; null when the source does not expose it. |
| `timezone` | text | Timezone shown for the Google Maps place by Google Maps, in the format used by the source; null when it is omitted. |

## Use cases

- Local sales teams can build prospect lists from businesses in a chosen area.
- Directory operators can compare addresses, ratings and websites while checking listings.
- Researchers can map business types across nearby neighborhoods.

## How to use

1. Open the Actor’s **Input** tab and start with the JSON below.
2. Set `query` to the search term or source identifier you want to look up, then use the optional filters listed below.
3. Start the run and open its default dataset to inspect or download the rows.

```json
{
  "query": "starbucks times square new york",
  "gl": "us"
}
```

The run dataset can be downloaded as JSON, CSV, Excel or another format offered by Apify.

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `query` | string | Yes | What to search for (e.g., 'restaurants in NYC', 'coffee shops', 'plumber') |
| `hl` | string | No | Two-letter language code, optionally with a two-letter region (e.g., 'en', 'de', 'es', 'fr', 'en-US', 'de-DE') |
| `gl` | string | No | ISO 3166-1 alpha-2 country code for region-specific results (e.g., 'us', 'de', 'fr', 'uk') |
| `debug` | boolean | No | Enable Scrappa debug output for troubleshooting. This is only useful for accounts with debug access. |
| `use_cache` | boolean | No | Use cached results if available to reduce costs and speed up results |
| `maximum_cache_age` | integer | No | Maximum age of cached results in seconds. Set to 0 to always fetch fresh data. Constraints: minimum 0. |
| `fallback_zoom` | integer | No | Zoom level used if the simple search endpoint has a transient upstream failure and the actor retries through advanced search. Constraints: minimum 3; maximum 21. |

## Output example

The record below is synthetic. Names and links are examples, and private contact fields are omitted.

```json
{
  "name": "Juniper Street Coffee",
  "type": "Video",
  "rating": 4.7,
  "review_count": 184,
  "full_address": "418 Pine Street, Seattle, WA 98101",
  "latitude": 47.6101,
  "longitude": -122.3421,
  "phone_numbers": null
}
```

## Pricing

**Current live price:** $0.30 per 1,000 results.

Each saved source match counts as one result.

## FAQ

### Is it legal to collect public information?

This Actor is for information visible on the public source pages it reads. You are responsible for checking the source terms, privacy and copyright rules, and the laws that apply to your use.

### How many records will a run return?

The result count depends on the input limits, pagination settings and what the source makes available. A small query or unavailable page can return fewer records, including none.

### Can I call it through the API or connect it to other tools?

Yes. Send the same JSON input to `POST https://api.apify.com/v2/acts/thescrappa~google-maps-search-scraper/runs`, then read the run’s default dataset. Make, Zapier and n8n can start runs and pass dataset rows to the next step; Google Sheets can receive rows through those workflows or a dataset export.

### What happens when a request fails?

Check the run log for the source or input error, correct the input and retry after a temporary source issue. A run can contain rows saved before a later request failed.

### Can Google Maps Search target a country or language?

Use `query` for the place or business and set `gl` or `hl` when you need a market or language. Search results vary with location and source coverage.

## Related Scrappa Actors

- [Google Maps Advanced Search Scraper](https://apify.com/thescrappa/google-maps-advanced-search-scraper)
- [Google Maps Autocomplete Scraper](https://apify.com/thescrappa/google-maps-autocomplete-scraper)
- [Google Maps Business Details Scraper](https://apify.com/thescrappa/google-maps-business-details-scraper)
- [Google Maps Directions Scraper](https://apify.com/thescrappa/google-maps-directions-scraper)
- [Google Maps Photos Scraper](https://apify.com/thescrappa/google-maps-photos-scraper)

# Google Maps Advanced Search Scraper

Find local businesses on Google Maps with names, categories, star ratings and addresses. Set the map center and a business category to find places in a chosen local area.

## What data can you extract?

Place and route details follow the public Google Maps page; optional ratings, links and photos may not be shown for every record.

| Field | Type | Description |
| --- | --- | --- |
| `name` | text | Name of the Google Maps place, as shown by Google Maps; null when no name is published. |
| `rating` | number | Rating for this Google Maps place, on a 1-to-5 star scale; null when no score is shown. |
| `review_count` | number | Number of reviews shown by Google Maps, as a whole number; zero is possible, and null means no count was reported. |
| `full_address` | text | Full address shown for the Google Maps place by Google Maps, in the format used by the source; null when it is omitted. |
| `phone_numbers` | array of text | List of phone numbers associated with this Google Maps place on Google Maps; empty when the source returns no entries. |
| `website` | link | Website url for this Google Maps place on Google Maps; null when the source does not provide a URL. |
| `latitude` | number | Latitude for this Google Maps place on Google Maps, in decimal degrees; null when the source provides no coordinates. |
| `longitude` | number | Longitude for this Google Maps place on Google Maps, in decimal degrees; null when the source provides no coordinates. |

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
  "query": "coffee shops",
  "zoom": 15,
  "latitude": 40.7128,
  "longitude": -74.006,
  "limit": 10,
  "gl": "us"
}
```

The run dataset can be downloaded as JSON, CSV, Excel or another format offered by Apify.

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `query` | string | Yes | What to search for (e.g., 'coffee shops', 'restaurants') |
| `zoom` | integer | Yes | Map zoom level for precision (3=broad overview, 21=street level) Constraints: minimum 3; maximum 21. |
| `latitude` | number | No | Center latitude for search (optional, auto-resolved from query if blank) |
| `longitude` | number | No | Center longitude for search (optional, auto-resolved from query if blank) |
| `limit` | integer | No | Maximum results to return (1 or more) Constraints: minimum 1. |
| `hl` | string | No | ISO 639-1 language code (e.g., 'en', 'de', 'es', 'fr' or with region 'en-US', 'de-DE') |
| `gl` | string | No | ISO 3166-1 alpha-2 country code for region-specific results (e.g., 'us', 'de', 'fr', 'uk') |

## Output example

The record below is synthetic. Names and links are examples, and private contact fields are omitted.

```json
{
  "phone_numbers": null,
  "name": "Juniper Street Coffee",
  "rating": 4.7,
  "review_count": 184,
  "full_address": "418 Pine Street, Seattle, WA 98101",
  "website": "https://northstar.example",
  "latitude": 47.6101,
  "longitude": -122.3421
}
```

## Pricing

**Current live price:** $5.00 per 1,000 searches; plus $0.30 per 1,000 results.

Each processed search or query is counted according to the rate shown above.

## FAQ

### Is it legal to collect public information?

This Actor is for information visible on the public source pages it reads. You are responsible for checking the source terms, privacy and copyright rules, and the laws that apply to your use.

### How many records will a run return?

The result count depends on the input limits, pagination settings and what the source makes available. A small query or unavailable page can return fewer records, including none.

### Can I call it through the API or connect it to other tools?

Yes. Send the same JSON input to `POST https://api.apify.com/v2/acts/thescrappa~google-maps-advanced-search-scraper/runs`, then read the run’s default dataset. Make, Zapier and n8n can start runs and pass dataset rows to the next step; Google Sheets can receive rows through those workflows or a dataset export.

### What happens when a request fails?

Check the run log for the source or input error, correct the input and retry after a temporary source issue. A run can contain rows saved before a later request failed.

### How do I center a Google Maps Advanced Search?

Set `query` to a business type or phrase and provide the map center with `latitude` and `longitude`. `zoom` and `limit` control the requested local search.

## Related Scrappa Actors

- [Google Maps Autocomplete Scraper](https://apify.com/thescrappa/google-maps-autocomplete-scraper)
- [Google Maps Business Details Scraper](https://apify.com/thescrappa/google-maps-business-details-scraper)
- [Google Maps Directions Scraper](https://apify.com/thescrappa/google-maps-directions-scraper)
- [Google Maps Photos Scraper](https://apify.com/thescrappa/google-maps-photos-scraper)
- [Google Maps Reviews Scraper](https://apify.com/thescrappa/google-maps-reviews-scraper)

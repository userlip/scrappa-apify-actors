# Google Maps Autocomplete Scraper

Find place and business suggestions from Google Maps for a city, address or venue name. Enter the start of an address, business or place name to see matching suggestions.

## What data can you extract?

Suggestions reflect matching public place names and categories; address details can be incomplete.

| Field | Type | Description |
| --- | --- | --- |
| `main_text` | text | Main text from Google Maps for this Google Maps place; null when the source has no text to show. |
| `type` | text | Category assigned to the Google Maps place by Google Maps; null when Google Maps does not provide the value. |
| `country` | text | Country shown for the Google Maps place by Google Maps; null when Google Maps does not provide the value. |
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
  "query": "new york"
}
```

The run dataset can be downloaded as JSON, CSV, Excel or another format offered by Apify.

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `query` | string | Yes | Partial search term for autocomplete (e.g., 'time sq', 'starbucks new') |

## Output example

The record below is synthetic. Names and links are examples, and private contact fields are omitted.

```json
{
  "main_text": "Juniper Street Coffee",
  "type": "Video",
  "country": "United States",
  "latitude": 47.6101,
  "longitude": -122.3421
}
```

## Pricing

**Current live price:** $0.00005 per Actor Start event; plus $0.30 per 1,000 results.

The listed amount is charged once when a run starts.

## FAQ

### Is it legal to collect public information?

This Actor is for information visible on the public source pages it reads. You are responsible for checking the source terms, privacy and copyright rules, and the laws that apply to your use.

### How many records will a run return?

The result count depends on the input limits, pagination settings and what the source makes available. A small query or unavailable page can return fewer records, including none.

### Can I call it through the API or connect it to other tools?

Yes. Send the same JSON input to `POST https://api.apify.com/v2/acts/thescrappa~google-maps-autocomplete-scraper/runs`, then read the run’s default dataset. Make, Zapier and n8n can start runs and pass dataset rows to the next step; Google Sheets can receive rows through those workflows or a dataset export.

### What happens when a request fails?

Check the run log for the source or input error, correct the input and retry after a temporary source issue. A run can contain rows saved before a later request failed.

### What query works best with Google Maps Autocomplete?

Enter a short city, address, venue or business phrase in `query`. The Actor returns suggestions for that text rather than full business profiles.

## Related Scrappa Actors

- [Google Maps Advanced Search Scraper](https://apify.com/thescrappa/google-maps-advanced-search-scraper)
- [Google Maps Business Details Scraper](https://apify.com/thescrappa/google-maps-business-details-scraper)
- [Google Maps Directions Scraper](https://apify.com/thescrappa/google-maps-directions-scraper)
- [Google Maps Photos Scraper](https://apify.com/thescrappa/google-maps-photos-scraper)
- [Google Maps Reviews Scraper](https://apify.com/thescrappa/google-maps-reviews-scraper)

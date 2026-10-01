# Google Maps Business Details Scraper

Look up a Google Maps business profile with its address, rating, hours and website. Pass one or more place IDs to look up the matching Google Maps business profiles.

## What data can you extract?

Place and route details follow the public Google Maps page; optional ratings, links and photos may not be shown for every record.

| Field | Type | Description |
| --- | --- | --- |
| `name` | text | Name of the business profile, as shown by Google Maps; null when no name is published. |
| `rating` | number | Rating for this business profile, on a 1-to-5 star scale; null when no score is shown. |
| `review_count` | number | Number of reviews shown by Google Maps, as a whole number; zero is possible, and null means no count was reported. |
| `full_address` | text | Full address shown for the business profile by Google Maps, in the format used by the source; null when it is omitted. |
| `phone_number` | text | Public phone number shown by Google Maps; null when the profile or listing does not publish contact details. |
| `website` | link | Website url for this business profile on Google Maps; null when the source does not provide a URL. |
| `type` | text | Category assigned to the business profile by Google Maps; null when Google Maps does not provide the value. |

## Use cases

- Local sales teams can build prospect lists from businesses in a chosen area.
- Directory operators can compare addresses, ratings and websites while checking listings.
- Researchers can map business types across nearby neighborhoods.

## How to use

1. Open the Actor’s **Input** tab and start with the JSON below.
2. Put the supported targets in `business_ids` and use the identifier or URL format required by Google Maps.
3. Start the run and open its default dataset to inspect or download the rows.

```json
{
  "business_ids": [
    "0x808fba02425dad8f:0x6c296c66619367e0"
  ]
}
```

The run dataset can be downloaded as JSON, CSV, Excel or another format offered by Apify.

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `business_ids` | array of string | No | Recommended. Process many Google Maps business IDs in one Apify run so run startup and storage overhead are shared across results. Constraints: minimum 1 items; maximum 10 items. |
| `business_id` | string | No | Backward-compatible single Google Maps business ID in format: 0x[hex]:0x[hex]. Prefer business_ids for normal usage, especially when processing more than one business. |
| `use_cache` | boolean | No | Use cached results if available to reduce costs and speed up results |
| `maximum_cache_age` | integer | No | Maximum age of cached results in seconds. Set to 0 to always fetch fresh data. Constraints: minimum 0. |

## Output example

The record below is synthetic. Names and links are examples, and private contact fields are omitted.

```json
{
  "name": "Juniper Street Coffee",
  "rating": 4.7,
  "review_count": 184,
  "full_address": "418 Pine Street, Seattle, WA 98101",
  "website": "https://northstar.example",
  "type": "Video",
  "phone_number": null
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

Yes. Send the same JSON input to `POST https://api.apify.com/v2/acts/thescrappa~google-maps-business-details-scraper/runs`, then read the run’s default dataset. Make, Zapier and n8n can start runs and pass dataset rows to the next step; Google Sheets can receive rows through those workflows or a dataset export.

### What happens when a request fails?

Check the run log for the source or input error, correct the input and retry after a temporary source issue. A run can contain rows saved before a later request failed.

### Where can I get Google Maps place IDs for business details?

Use Google Maps Search or Advanced Search to find a place, then pass its `business_id` in this Actor. You can submit several place IDs in one run.

## Related Scrappa Actors

- [Google Maps Advanced Search Scraper](https://apify.com/thescrappa/google-maps-advanced-search-scraper)
- [Google Maps Autocomplete Scraper](https://apify.com/thescrappa/google-maps-autocomplete-scraper)
- [Google Maps Directions Scraper](https://apify.com/thescrappa/google-maps-directions-scraper)
- [Google Maps Photos Scraper](https://apify.com/thescrappa/google-maps-photos-scraper)
- [Google Maps Reviews Scraper](https://apify.com/thescrappa/google-maps-reviews-scraper)

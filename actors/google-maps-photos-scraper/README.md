# Google Maps Photos Scraper

Browse photos shared on Google Maps place pages, with image links and contributor details. Submit place IDs for one or more locations whose public photos you want to catalog.

## What data can you extract?

Photo links and contributor details are available when the public place page provides them.

| Field | Type | Description |
| --- | --- | --- |
| `photo_url_large` | link | Large photo url for this place photo on Google Maps; null when the source does not provide a URL. |
| `width` | number | Image width of the image shown by Google Maps, in pixels; null when the source does not publish the dimension. |
| `height` | number | Image height of the image shown by Google Maps, in pixels; null when the source does not publish the dimension. |
| `contributor_name` | text | Photo contributor name shown for the place photo by Google Maps, in the format used by the source; null when it is omitted. |
| `posted_at` | text | Time the post was published shown by Google Maps, in ISO 8601 date and time; null if the source omits the date. |
| `photo_id` | text | photo ID for the place photo, assigned by Google Maps; null when the source does not expose it. |

## Use cases

- Location managers can check public photos before refreshing a business listing.
- Travel editors can review images visitors have shared for a venue or destination.
- Researchers can compare photo dates and contributors across nearby places.

## How to use

1. Open the Actor’s **Input** tab and start with the JSON below.
2. Put the supported targets in `business_ids` and use the identifier or URL format required by Google Maps.
3. Start the run and open its default dataset to inspect or download the rows.

```json
{
  "business_ids": [
    "0x808fba02425dad8f:0x6c296c66619367e0",
    "ChIJj61dQgK6j4AR4GeTYWZsKWw"
  ]
}
```

The run dataset can be downloaded as JSON, CSV, Excel or another format offered by Apify.

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `business_ids` | array of string | No | Recommended. Process many Google Maps business IDs, ChIJ... Place IDs, or supported Maps URLs in one Apify run so run startup and storage overhead are shared across photo results. Constraints: minimum 1 items; maximum 10 items. |
| `business_id` | string | No | Backward-compatible single Google Maps business ID, ChIJ... Place ID, or supported Maps URL. Prefer business_ids for normal usage, especially when processing more than one business. |
| `use_cache` | boolean | No | Use cached results if available to reduce costs and speed up results |
| `maximum_cache_age` | integer | No | Maximum age of cached results in seconds. Set to 0 to always fetch fresh data. Constraints: minimum 0. |

## Output example

The record below is synthetic. Names and links are examples, and private contact fields are omitted.

```json
{
  "photo_url_large": "https://images.example.com/places/juniper-street-coffee-large.jpg",
  "width": 1280,
  "height": 720,
  "contributor_name": "Riley Park",
  "posted_at": "2026-09-25T09:15:00Z",
  "photo_id": "photo_demo_8K4m2"
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

Yes. Send the same JSON input to `POST https://api.apify.com/v2/acts/thescrappa~google-maps-photos-scraper/runs`, then read the run’s default dataset. Make, Zapier and n8n can start runs and pass dataset rows to the next step; Google Sheets can receive rows through those workflows or a dataset export.

### What happens when a request fails?

Check the run log for the source or input error, correct the input and retry after a temporary source issue. A run can contain rows saved before a later request failed.

### How do I choose a place for Google Maps Photos?

Submit a Google Maps place ID in `business_id` or the batch field supported by Input. The photos and contributor details depend on the public place page.

## Related Scrappa Actors

- [Google Maps Advanced Search Scraper](https://apify.com/thescrappa/google-maps-advanced-search-scraper)
- [Google Maps Autocomplete Scraper](https://apify.com/thescrappa/google-maps-autocomplete-scraper)
- [Google Maps Business Details Scraper](https://apify.com/thescrappa/google-maps-business-details-scraper)
- [Google Maps Directions Scraper](https://apify.com/thescrappa/google-maps-directions-scraper)
- [Google Maps Reviews Scraper](https://apify.com/thescrappa/google-maps-reviews-scraper)

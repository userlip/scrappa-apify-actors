# Google Maps Photos Scraper

Extract photo URLs and visual metadata from a Google Maps business listing by business ID. Use this actor to build location galleries, audit brand imagery, monitor customer-uploaded photos, and collect visual evidence for local business research.

Apify actor: [`gLbfii9Nq4H7auMnN`](https://console.apify.com/actors/gLbfii9Nq4H7auMnN)

Pricing: **$0.30 per 1,000 results**. No Google Maps API key required.

## What you get

- Photo URLs, large photo URLs, dimensions, and photo IDs.
- Contributor names, contributor profile URLs, and relative posting age when available.
- One dataset item per photo for CSV, JSON, Excel, and integration exports.
- Cache controls for faster repeat runs and lower-cost monitoring jobs.
- Structured error items for missing businesses instead of failed empty exports.

## Best for

- Directory builders enriching local business profiles with visual data.
- Brand, reputation, and operations teams reviewing location imagery.
- Franchise, hospitality, healthcare, agency, and retail analysts.
- Data teams joining Google Maps search, details, reviews, and photos exports.

## Input

Provide a Google Maps `business_id` in the `0x[hex]:0x[hex]` format. Google Place IDs such as `ChIJ...` are not accepted by this actor; use the Google Maps business ID format shown below.

```json
{
  "business_id": "0x808fba02425dad8f:0x6c296c66619367e0",
  "use_cache": true,
  "maximum_cache_age": 3600
}
```

### Input fields

| Field | Type | Required | Default | Description |
| --- | --- | --- | --- | --- |
| `business_id` | string | Yes | - | Google Maps business ID for the listing you want to inspect, in `0x[hex]:0x[hex]` format. |
| `use_cache` | boolean | No | `true` | Uses cached Scrappa results when available for faster and lower-cost runs. |
| `maximum_cache_age` | integer | No | `3600` | Controls how old cached results can be, in seconds. Set to `0` when you need the freshest available data. |

## Output

### Dataset items

The actor pushes one dataset item per photo. Available fields can vary by listing and Google Maps source data, but successful photo records commonly include:

```json
{
  "photo_id": "CIABIhCZJXgWXJBLW3f-sOE573RB",
  "photo_url": "https://lh3.googleusercontent.com/...",
  "photo_url_large": "https://lh3.googleusercontent.com/...",
  "width": 4032,
  "height": 2268,
  "contributor_name": "Roger Hall",
  "contributor_url": "https://www.google.com/maps/contrib/106568860865951283765?hl=en",
  "posted_at": "7 months ago"
}
```

Other records may include additional metadata such as `latitude`, `longitude`, `photo_index`, `source`, `author`, `published_at`, `is_owner`, `likes`, or `video_thumbnail_url` when Google Maps provides those values.

### Key-value store summary

The actor also writes an `OUTPUT` key-value store record with:

```json
{
  "photos": [
    {
      "photo_id": "CIABIhCZJXgWXJBLW3f-sOE573RB",
      "photo_url": "https://lh3.googleusercontent.com/..."
    }
  ],
  "total": 1,
  "nextPage": null
}
```

This key-value store record is a summary envelope for the run. `photos` contains the same photo objects pushed to the dataset, `total` is the number of photos returned, and `nextPage` is included when the upstream response provides pagination context.

If the business ID is not found, the dataset receives a structured error item instead of an unhandled failure.

```json
{
  "success": false,
  "business_id": "0x0000000000000000:0x0000000000000000",
  "error": "Business not found"
}
```

## Cache behavior

Caching is enabled by default because photo lists usually do not change minute by minute. This makes repeat runs faster and can reduce request cost. Use `maximum_cache_age` to tune freshness:

- `3600` - Good default for normal enrichment and gallery building.
- `86400` - Useful for daily monitoring or bulk research where speed matters more than immediate freshness.
- `0` - Use when you are checking a recent listing update or validating newly uploaded photos.

## Example visual-data workflows

- Build photo galleries for local business directories, lead lists, and internal CRM records.
- Monitor new customer-uploaded photos for brand, cleanliness, merchandising, or store-condition signals.
- Compare storefront, room, menu, venue, or product imagery across locations.
- Enrich Google Maps search or business-details datasets with high-resolution image URLs.
- Collect visual audit evidence for reputation management, competitor tracking, site selection, and field operations.

## Direct API

For higher-volume or direct API workflows, use Scrappa's Google Maps endpoints directly and keep this actor as the Apify-ready no-code runner.

```bash
curl 'https://scrappa.co/api/maps/photos?business_id=0x808fba02425dad8f:0x6c296c66619367e0&use_cache=1&maximum_cache_age=3600' \
  -H "X-API-Key: YOUR_SCRAPPA_API_KEY" \
  -H "Accept: application/json"
```

## Notes

- Photo availability and metadata depend on the public Google Maps listing data available for the business.
- `photo_url` and `photo_url_large` values are direct image URLs returned by the upstream data source.
- Cached responses are best for repeat enrichment and monitoring jobs where minute-level freshness is not required.

## Support

If a run returns no photos for a business that visibly has Google Maps photos, confirm the `business_id` format first. For repeat failures, include the actor run ID, the input business ID, and whether cache was enabled when contacting support.

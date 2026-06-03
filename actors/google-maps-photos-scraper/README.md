# Google Maps Photos Scraper

Extract photo URLs and visual metadata from one or many Google Maps business listings by business ID. Use this actor to build location galleries, audit brand imagery, monitor customer-uploaded photos, and collect visual evidence for local business research.

Apify actor: [`gLbfii9Nq4H7auMnN`](https://console.apify.com/actors/gLbfii9Nq4H7auMnN)

Pricing: **$0.30 per 1,000 results**. No Google Maps API key required.

## What you get

- Photo URLs, large photo URLs, dimensions, and photo IDs.
- Contributor names, contributor profile URLs, and relative posting age when available.
- Batch input with one dataset item per photo for CSV, JSON, Excel, and integration exports.
- Cache controls for faster repeat runs and lower-cost monitoring jobs.
- Structured error items for missing businesses instead of failed empty exports.

## Best for

- Local SEO teams.
- Brand and reputation teams.
- Field operations teams.
- Data engineers and enrichment developers.

## Input

Provide Google Maps `business_ids` in the `0x[hex]:0x[hex]` format, Google Place IDs such as `ChIJ...`, or Google Maps URLs that contain one of those identifiers. URLs copied from Google Maps sometimes include only coordinates and a place name; for those, first run Google Maps Search or Business Details and use the returned `business_id` or `place_id`.

```json
{
  "business_ids": [
    "0x808fba02425dad8f:0x6c296c66619367e0",
    "ChIJj61dQgK6j4AR4GeTYWZsKWw"
  ],
  "use_cache": true,
  "maximum_cache_age": 3600
}
```

### Input fields

| Field | Type | Required | Default | Description |
| --- | --- | --- | --- | --- |
| `business_ids` | array | Yes, unless using legacy `business_id` | - | Recommended input. Process multiple Google Maps business IDs, Place IDs, or supported Maps URLs in one Apify run. |
| `business_id` | string | Yes, unless using `business_ids` | - | Legacy single-business input. Use `business_ids` for normal usage, especially when processing more than one business. |
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
  "posted_at": "7 months ago",
  "input_business_id": "0x808fba02425dad8f:0x6c296c66619367e0",
  "business_id": "0x808fba02425dad8f:0x6c296c66619367e0"
}
```

Dataset records may also include `latitude`, `longitude`, `photo_index`, `source`, `author`, `published_at`, `is_owner`, `likes`, or `video_thumbnail_url` when Google Maps provides those values.

### Key-value store summary

For legacy single-business runs, the actor also writes an `OUTPUT` key-value store record with:

```json
{
  "photos": [
    {
      "photo_id": "CIABIhCZJXgWXJBLW3f-sOE573RB",
      "photo_url": "https://lh3.googleusercontent.com/..."
    }
  ],
  "total": 1,
  "nextPage": "CAESBkVnSUl..."
}
```

This key-value store record is a summary envelope for the run. `photos` contains the same photo objects pushed to the dataset, `total` is the number of photos returned, and `nextPage` is included when the upstream response provides pagination context. Batch runs use the dataset as the primary result channel and write a compact per-business summary to `OUTPUT`.

If the business ID is not found, or if a Maps URL does not contain a supported identifier, the dataset receives a structured error item instead of an unhandled failure.

```json
{
  "success": false,
  "input_business_id": "0x0000000000000000:0x0000000000000000",
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

- Export Google Maps photo galleries into CSV, JSON, Excel, or downstream review queues.
- Monitor new customer-uploaded photos for brand, cleanliness, merchandising, or store-condition signals.
- Compare storefront, room, menu, venue, or product imagery across locations.
- Enrich Google Maps search or business-details datasets with high-resolution image URLs.
- Collect visual audit evidence for reputation management, competitor tracking, site selection, and field operations.

## Recommended workflow

1. Run Google Maps Search, Advanced Search, Autocomplete, or Business Details to collect `business_id` values.
2. Put those IDs into `business_ids` and run this actor once for the whole batch.
3. Export photo records to your data warehouse, local SEO audit, visual review queue, or media monitoring pipeline.
4. Store `business_id` with every photo record so you can join photos back to business details, reviews, ratings, categories, and location data.

## Tips for better results

- Use exact `business_id` or `place_id` values from recent Google Maps discovery runs.
- Keep `use_cache` enabled for recurring audits, deduplication, and large enrichment jobs.
- Disable cache only for freshness-sensitive checks, such as monitoring whether a competitor added new photos this week.
- Use `photo_url_large` when you need higher-resolution images and `photo_url` for lightweight previews.
- Expect fields to vary by business because Google Maps does not expose the same metadata for every photo.

## Direct API

For Apify usage, put many businesses in `business_ids` so a single run can produce many photo records. For higher-volume or direct API workflows, use Scrappa's Google Maps endpoints directly and keep this actor as the Apify-ready no-code runner.

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

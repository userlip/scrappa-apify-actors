# Google Maps Photos Scraper

Extract Google Maps photos for a single business by `business_id`. This actor is built for local SEO audits, visual profile monitoring, location pages, business intelligence, and workflows that already discovered a place with a Google Maps Search, Advanced Search, Autocomplete, or Business Details actor.

Use it when you need photo URLs, image metadata, coordinates, author signals, and owner-upload flags for a known Google Maps business.

## What It Does

- Looks up photos for one Google Maps business by `business_id`.
- Returns individual photo records in the default Apify dataset.
- Captures photo URLs, larger image URLs when available, coordinates, author metadata, source, publish time, owner-upload status, likes, and photo ordering fields.
- Supports cached responses for faster repeat runs and lower cost.
- Lets you control cache freshness with `maximum_cache_age`.
- Saves the full response to the `OUTPUT` key-value store record with `photos`, `total`, and `nextPage`.
- Handles missing businesses gracefully with a structured `success: false` dataset item.

## Common Use Cases

- Build image galleries for store locators, local landing pages, travel guides, and venue directories.
- Monitor whether competitors or franchise locations are adding new Google Maps photos.
- Audit owner-uploaded photos versus user-generated photos across business profiles.
- Enrich Google Maps lead lists with visual evidence before outreach or verification.
- Archive photo URLs and coordinates for internal research, QA, and local SEO workflows.

## Input

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `business_id` | string | Yes | Google Maps business identifier, typically in the `0x...:0x...` format. Use a `business_id` returned by Scrappa Google Maps Search, Advanced Search, Autocomplete, Reviews, Business Details, or another Google Maps discovery workflow. |
| `use_cache` | boolean | No | Use cached data when available. Defaults to `true`. Set to `false` when you need the freshest available photos. |
| `maximum_cache_age` | integer | No | Maximum allowed cache age in seconds. Defaults to `3600`. Set to `0` to request fresh data when cache is enabled. |

## Example Input

Googleplex example:

```json
{
  "business_id": "0x808fba02425dad8f:0x6c296c66619367e0",
  "use_cache": true,
  "maximum_cache_age": 3600
}
```

## Output

Each photo is saved as a separate record in the default Apify dataset. Fields vary by what Google Maps exposes for the business, but each item can include:

- `photo_id`
- `photo_url`
- `photo_url_large`
- `latitude`
- `longitude`
- `video_thumbnail_url`
- `photo_index`
- `source`
- `author`
- `published_at`
- `is_owner`
- `likes`

## Example Output

```json
{
  "photo_id": "AF1QipNExamplePhotoId",
  "photo_url": "https://lh3.googleusercontent.com/p/AF1QipNExamplePhotoId=w408-h306-k-no",
  "photo_url_large": "https://lh3.googleusercontent.com/p/AF1QipNExamplePhotoId=s1600-w1600-h1200",
  "latitude": 37.422,
  "longitude": -122.0841,
  "video_thumbnail_url": null,
  "photo_index": 1,
  "source": "Google Maps",
  "author": "Google",
  "published_at": "2025-11-18T14:22:00Z",
  "is_owner": true,
  "likes": 42
}
```

The full response is also stored in the key-value store under `OUTPUT`:

```json
{
  "photos": [
    {
      "photo_id": "AF1QipNExamplePhotoId",
      "photo_url": "https://lh3.googleusercontent.com/p/AF1QipNExamplePhotoId=w408-h306-k-no",
      "photo_url_large": "https://lh3.googleusercontent.com/p/AF1QipNExamplePhotoId=s1600-w1600-h1200",
      "latitude": 37.422,
      "longitude": -122.0841,
      "photo_index": 1,
      "author": "Google",
      "is_owner": true,
      "likes": 42
    }
  ],
  "total": 1,
  "nextPage": null
}
```

When a business cannot be found, the actor returns a structured dataset item:

```json
{
  "success": false,
  "business_id": "0xinvalid:0xinvalid",
  "error": "Business not found"
}
```

## Recommended Workflow

1. Run Google Maps Search, Google Maps Advanced Search, or Google Maps Business Details to collect `business_id` values.
2. Run this actor once per business to extract photo records.
3. Join photo records back to your business table by `business_id` in your CRM, data warehouse, or content workflow.
4. Use Apify scheduling for recurring monitoring, and use Scrappa direct API access when you need higher throughput, custom integration, or larger-scale Google Maps photo collection.

## Caching Behavior

Caching is enabled by default. With the default input, the actor sends `use_cache=1` to Scrappa and accepts cached results up to `maximum_cache_age` seconds old.

Set `use_cache` to `false` when you need the freshest available Google Maps photo data. Set `maximum_cache_age` to a lower value for tighter freshness requirements, or keep the default `3600` seconds for repeat enrichment jobs where speed and cost control matter more than minute-level freshness.

## Scale With Scrappa API

Apify is ideal for scheduled jobs, no-code exports, and batch workflows. For high-volume Google Maps photo extraction across thousands of businesses, use Scrappa direct API access for larger throughput, tighter application integration, and custom pipeline control.

Start from this actor on Apify, then move heavy or always-on workloads to the Scrappa API when you outgrow marketplace runs.

## Pricing

$0.30 per 1,000 results. No Google Maps API key required.

## Support

For issues or questions, contact us through Apify.

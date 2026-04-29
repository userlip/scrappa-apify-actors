# Google Maps Photos Scraper

Extract photo records for a specific Google Maps business by `business_id`. This actor is built for local SEO audits, competitive research, storefront monitoring, travel and hospitality datasets, visual QA, and workflows that already discovered a place with a Google Maps Search, Advanced Search, Autocomplete, or Business Details actor.

Use it when you need Google Maps business photos as structured records with image URLs, dimensions, contributor metadata, posting age, and pagination data for downstream collection.

## What It Does

- Looks up a single Google Maps business by `business_id`.
- Returns photo records with image URLs, photo identifiers, dimensions, contributor metadata, posting age, coordinates, ownership flags, likes, timestamps, and video thumbnails when Google exposes them.
- Saves each photo as a separate item in the default Apify dataset for easy export to JSON, CSV, Excel, or integrations.
- Saves the full Scrappa API response to the `OUTPUT` key-value store record, including `photos`, `total`, and `nextPage`.
- Supports cached responses for faster repeat runs and lower cost.
- Lets you control cache freshness with `maximum_cache_age`.
- Handles missing businesses gracefully with a structured `success: false` dataset item instead of a failed empty export.

## Common Use Cases

- Build image datasets for restaurants, hotels, attractions, clinics, retail locations, and local service businesses.
- Monitor competitors for new storefront, menu, product, room, or venue photos.
- Enrich Google Maps lead lists with visual proof, gallery images, and location-level media.
- Audit franchise and multi-location brand profiles for missing, stale, or user-generated images.
- Power internal review workflows where teams need to inspect business photos outside Google Maps.
- Join search, business details, reviews, and photos data using the same `business_id`.

## Input

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `business_id` | string | Yes | Google Maps business identifier, typically in the `0x...:0x...` format. Use a `business_id` returned by Scrappa Google Maps Search, Advanced Search, Autocomplete, Business Details, Reviews, or another Google Maps discovery workflow. |
| `use_cache` | boolean | No | Use cached data when available. Defaults to `true`. Set to `false` when you need the freshest available photo list. |
| `maximum_cache_age` | integer | No | Maximum allowed cache age in seconds. Defaults to `3600`. Set to `0` to always request fresh data when cache is enabled. |

## Example Input

```json
{
  "business_id": "0x808fba02425dad8f:0x6c296c66619367e0",
  "use_cache": true,
  "maximum_cache_age": 3600
}
```

## Output

The actor saves each photo as a separate dataset item when photos are found. Fields vary by what Google Maps exposes for the business, but each item can include:

- `photo_id`
- `photo_url`
- `photo_url_large`
- `width`
- `height`
- `contributor_name`
- `contributor_url`
- `posted_at`
- `latitude`
- `longitude`
- `video_thumbnail_url`
- `photo_index`
- `source`
- `author`
- `published_at`
- `is_owner`
- `likes`

The full response is also stored in the key-value store under `OUTPUT`:

```json
{
  "photos": [
    {
      "photo_id": "AF1QipExamplePhotoId",
      "photo_url": "https://lh3.googleusercontent.com/p/example=w408-h306-k-no",
      "photo_url_large": "https://lh3.googleusercontent.com/p/example=w1200-h900-k-no",
      "width": 4032,
      "height": 2268,
      "contributor_name": "Example Local Guide",
      "contributor_url": "https://www.google.com/maps/contrib/example",
      "posted_at": "7 months ago"
    }
  ],
  "total": 1,
  "nextPage": "CAESBkVnSUl..."
}
```

## Example Dataset Item

```json
{
  "photo_id": "AF1QipExamplePhotoId",
  "photo_url": "https://lh3.googleusercontent.com/p/example=w408-h306-k-no",
  "photo_url_large": "https://lh3.googleusercontent.com/p/example=w1200-h900-k-no",
  "width": 4032,
  "height": 2268,
  "contributor_name": "Example Local Guide",
  "contributor_url": "https://www.google.com/maps/contrib/example",
  "posted_at": "7 months ago"
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

## Cache Behavior

Cache is enabled by default because photo collections are often reused across audits, enrichment jobs, and repeat exports.

- Leave `use_cache` as `true` for faster repeat runs and lower-cost workflows.
- Set `maximum_cache_age` to the maximum age you will accept, in seconds. The default is `3600` seconds.
- Set `maximum_cache_age` to `0` when cache is enabled but you want the freshest available response.
- Set `use_cache` to `false` when you are checking for newly added or recently removed Google Maps photos.

## Recommended Workflow

1. Run Google Maps Search, Advanced Search, or Business Details to collect `business_id` values.
2. Run this actor once per target `business_id`.
3. Export dataset items to your data warehouse, local SEO audit, visual review queue, or media monitoring pipeline.
4. Store `business_id` with every photo record so you can join photos back to business details, reviews, ratings, categories, and location data.

## Tips For Better Results

- Use exact `business_id` values from recent Google Maps discovery runs.
- Keep `use_cache` enabled for recurring audits, deduplication, and large enrichment jobs.
- Disable cache only for freshness-sensitive checks, such as monitoring whether a competitor added new photos this week.
- Use `photo_url_large` when you need higher-resolution images and `photo_url` for lightweight previews.
- Expect fields to vary by business because Google Maps does not expose the same metadata for every photo.

## Scale With Scrappa

This actor is a good fit for Apify workflows, scheduled jobs, exports, and no-code integrations. For larger Google Maps photo pipelines, direct API access through Scrappa can support higher-throughput enrichment, tighter backend integration, and coordinated use of related endpoints such as Google Maps Search, Business Details, Reviews, and Photos.

If you are collecting photos across thousands of businesses or running recurring location-monitoring jobs, use this actor for Apify-native automation and Scrappa direct API access when you need more control over batching, retries, and downstream data delivery.

## Pricing

$0.30 per 1,000 results. No Google Maps API key required.

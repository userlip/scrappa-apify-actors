# Google Maps Photos Scraper

Extract photo URLs and visual metadata from a Google Maps business listing by business ID. Use this actor to build location galleries, audit brand imagery, monitor customer-uploaded photos, and collect visual evidence for local business research.

## Input

Provide a Google Maps `business_id` in the `0x[hex]:0x[hex]` format.

```json
{
  "business_id": "0x808fba02425dad8f:0x6c296c66619367e0",
  "use_cache": true,
  "maximum_cache_age": 3600
}
```

### Input fields

- `business_id` - Required. Google Maps business ID for the listing you want to inspect.
- `use_cache` - Optional. Defaults to `true`. Uses cached Scrappa results when available for faster and lower-cost runs.
- `maximum_cache_age` - Optional. Defaults to `3600` seconds. Controls how old cached results can be. Set to `0` when you need the freshest available data.

## Output

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

The actor also writes an `OUTPUT` key-value store record with:

```json
{
  "photos": [],
  "total": 0,
  "nextPage": null
}
```

If the business ID is not found, the dataset receives a structured error item instead of an unhandled failure.

## Cache behavior

Caching is enabled by default because photo lists usually do not change minute by minute. This makes repeat runs faster and can reduce request cost. Use `maximum_cache_age` to tune freshness:

- `3600` - Good default for normal enrichment and gallery building.
- `86400` - Useful for daily monitoring or bulk research where speed matters more than immediate freshness.
- `0` - Use when you are checking a recent listing update or validating newly uploaded photos.

## Visual-data use cases

- Build photo galleries for local business directories, lead lists, and internal CRM records.
- Monitor new customer-uploaded photos for brand, cleanliness, merchandising, or store-condition signals.
- Compare location imagery across franchises, hotels, restaurants, clinics, agencies, and retail chains.
- Enrich Google Maps search or business-details datasets with high-resolution image URLs.
- Collect visual audit evidence for reputation management, competitor tracking, site selection, and field operations.

For higher-volume or direct API workflows, use Scrappa's Google Maps endpoints directly and keep this actor as the Apify-ready no-code runner.

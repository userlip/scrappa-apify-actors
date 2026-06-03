# Google Maps Business Details Scraper

Fetch complete details for one or many Google Maps businesses by `business_id`. This actor is built for lead enrichment, local business research, CRM cleanup, competitor monitoring, and workflows that already discovered places with a Google Maps Search or Autocomplete actor and now need the full business profiles.

Use it when you need authoritative business records with contact data, ratings, location fields, opening hours, photos, and Google Maps identifiers.

## What It Does

- Looks up one or many Google Maps businesses by `business_ids`.
- Returns detailed business data such as name, rating, review count, address, phone, website, coordinates, categories, hours, photos, and Google identifiers when available.
- Supports cached responses for faster repeat enrichment and lower cost.
- Lets you control cache freshness with `maximum_cache_age`.
- Saves one business record per result to the default Apify dataset.
- Keeps the full Scrappa API response in `OUTPUT` for legacy single-business runs; batch runs write a compact summary to `OUTPUT`.
- Handles missing businesses gracefully with a structured `success: false` dataset item instead of an empty failed export.

## Common Use Cases

- Enrich search results from Google Maps Search, Advanced Search, or Autocomplete with full business details.
- Build local lead lists with phone numbers, websites, addresses, coordinates, categories, and operating hours.
- Refresh CRM records for restaurants, hotels, clinics, agencies, retail stores, service businesses, and franchise locations.
- Monitor competitor locations for profile changes, contact updates, and availability of web or phone data.
- Validate a single place before running downstream review, photo, or outreach workflows.

## Input

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `business_ids` | array | Yes, unless using legacy `business_id` | Recommended input. Process multiple Google Maps business identifiers in one Apify run. Use IDs returned by Scrappa Google Maps Search, Advanced Search, Autocomplete, Reviews, or another Google Maps discovery workflow. |
| `business_id` | string | Yes, unless using `business_ids` | Legacy single-business input. Use `business_ids` for normal usage, especially when processing more than one business. |
| `use_cache` | boolean | No | Use cached data when available. Defaults to `true`. Set to `false` when you need the freshest available profile. |
| `maximum_cache_age` | integer | No | Maximum allowed cache age in seconds. Defaults to `3600`. Set to `0` to always request fresh data when cache is enabled. |

## Example Input

```json
{
  "business_ids": [
    "0x808fba02425dad8f:0x6c296c66619367e0",
    "0x80c2c7c292fef33d:0x9a4f4f5f89f8b8c7"
  ],
  "use_cache": true,
  "maximum_cache_age": 3600
}
```

## Output

The actor saves one business details record to the default dataset when the business is found. Fields vary by the data Google Maps exposes for the business, but each item can include:

- `business_id`
- `name`
- `rating`
- `review_count`
- `phone_number`
- `international_phone_number`
- `formatted_phone_number`
- `website`
- `email`
- `full_address`
- `formatted_address`
- `address_components`
- `district`
- `latitude`
- `longitude`
- `place_id`
- `type`
- `types`
- `subtypes`
- `price_level`
- `opening_hours`
- `current_status`
- `url`
- `vicinity`
- `utc_offset`
- `photos`
- `icon`
- `icon_mask_base_uri`
- `icon_background_color`

For legacy single-business runs, the full API response is also stored in the key-value store under `OUTPUT`. Batch runs use the dataset as the primary result channel and write a compact per-business summary to `OUTPUT`.

## Example Output

```json
{
  "business_id": "0x808fba02425dad8f:0x6c296c66619367e0",
  "input_business_id": "0x808fba02425dad8f:0x6c296c66619367e0",
  "name": "Example Coffee",
  "rating": 4.6,
  "review_count": 842,
  "phone_number": "+12125550100",
  "international_phone_number": "+1 212-555-0100",
  "website": "https://example.com",
  "full_address": "123 Example Ave, New York, NY 10036, United States",
  "latitude": 40.7581,
  "longitude": -73.9856,
  "place_id": "ChIJExamplePlaceId",
  "type": "Coffee shop",
  "types": ["cafe", "food", "point_of_interest", "establishment"],
  "subtypes": ["Coffee shop", "Cafe"],
  "opening_hours": [
    {
      "day": "Monday",
      "hours": ["7:00 AM-6:00 PM"]
    }
  ],
  "url": "https://www.google.com/maps/place/?q=place_id:ChIJExamplePlaceId"
}
```

When a business cannot be found, the actor returns a structured dataset item:

```json
{
  "success": false,
  "input_business_id": "0xinvalid:0xinvalid",
  "business_id": "0xinvalid:0xinvalid",
  "error": "Business not found"
}
```

## Recommended Workflow

1. Run Google Maps Search or Google Maps Advanced Search for a query such as `dentists in Austin` or `coffee shops near Times Square`.
2. Export the `business_id` values from the results you want to enrich.
3. Put those IDs into `business_ids` and run this actor once for the whole batch.
4. Send enriched records to your CRM, warehouse, outreach workflow, or review/photo collection pipeline.

## Tips For Better Results

- Use exact `business_id` values from a recent Google Maps discovery run.
- Keep `use_cache` enabled for repeat enrichment, audits, deduplication, and large lead workflows.
- Set `use_cache` to `false` when checking whether a business recently changed its website, phone, hours, or address.
- Store both `business_id` and `place_id` in downstream systems so you can join search, details, reviews, and photos data later.
- For Apify usage, put many businesses in `business_ids` so a single run can produce many business records. Use Scrappa direct API access when you need tighter integration or larger throughput.

## Pricing

$0.30 per 1,000 results. No Google Maps API key required.

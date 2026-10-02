# Kayak Hotel Details Scraper

Retrieve Kayak hotel property details including name, address, rating, telephone, description, amenities, and image links. Batch property paths to enrich lodging records.

## What data can you extract?

| Field | Type | Description |
| --- | --- | --- |
| `success` | Boolean | True when KAYAK returned a parsed hotel property response. |
| `operation` | String | KAYAK operation name used for this property lookup. |
| `data` | Object | Property details, page metadata, amenities, and source information. |
| `meta` | Object | Fetch timing, freshness, completeness, and request attempt details. |
| `input_path` | String | KAYAK property path supplied for the detail lookup. |
| `scraped_at` | String | UTC date and time when this record was collected. |

## Use cases

- Travel businesses can enrich hotel catalogs with public property details.
- Hotel analysts can compare ratings, locations, amenities, and descriptions.
- Destination researchers can assemble structured hotel information by market.

## How to use

1. Copy a KAYAK hotel property path into each entry in `hotels`.
2. Set `maxResults` to cap the saved property records.
3. Use property details and source metadata in your lodging workflow.

```json
{
  "hotels": [
    {
      "path": "/Las-Vegas-Hotels-Park-MGM-Las-Vegas.15297.ksp"
    }
  ],
  "maxResults": 10
}
```

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `hotels` | Array\<object\> | Yes | KAYAK property paths copied from hotel listing URLs. |
| `hotels[].path` | string | Yes per entry | KAYAK property path copied from a hotel or rental search URL. |
| `maxResults` | integer | No | Maximum dataset items to save across this entire run. |

## Output example

```json
{
  "success": true,
  "operation": "hotel-property",
  "data": {
    "results": [
      {
        "name": "Park MGM Las Vegas",
        "url": "https://www.kayak.com/Las-Vegas-Hotels-Park-MGM-Las-Vegas.15297.ksp",
        "description": "A resort hotel on the Las Vegas Strip with dining, pool, and entertainment options.",
        "telephone": "+1-702-730-7777",
        "starRating": "4 stars",
        "aggregateRating": {
          "ratingValue": "8.7",
          "bestRating": "10",
          "ratingCount": "8420",
          "@type": "AggregateRating"
        },
        "priceRange": "$$",
        "address": {
          "streetAddress": "3770 S Las Vegas Blvd",
          "addressLocality": "Las Vegas",
          "postalCode": "89109",
          "addressRegion": "Nevada",
          "addressCountry": "US",
          "@type": "PostalAddress"
        },
        "amenityFeature": [
          "Pool",
          "Wi-Fi",
          "Fitness center"
        ],
        "image": "https://images.kayak.com/hotel/park-mgm-main.jpg",
        "@type": "Hotel",
        "@context": "https://schema.org"
      }
    ],
    "provenance": {
      "provider": "kayak",
      "freshness": "live"
    },
    "source_data": {
      "kayak": {
        "hotelId": 15297,
        "policies": [
          {
            "type": "check-in",
            "title": "Check-in",
            "dataText": "From 3:00 PM"
          }
        ],
        "amenitiesData": {
          "topAmenities": [
            {
              "code": "pool",
              "localizedName": "Pool",
              "icon": "pool"
            }
          ]
        }
      }
    }
  },
  "meta": {
    "freshness": "live",
    "exhaustive": true,
    "duration_ms": 1250,
    "attempts": 1
  },
  "input_path": "/Las-Vegas-Hotels-Park-MGM-Las-Vegas.15297.ksp",
  "scraped_at": "2026-10-01T12:00:00Z"
}
```

## Pricing

$0.50 per 1,000 results. Apify saves one dataset item per result and applies the Actor’s per-result price to saved items.

Set **maxResults** to cap the number of dataset items saved in one run.

## FAQ

### Is scraping this information legal?

Rules depend on the source, location, data type, and intended use. Check applicable laws and source terms, and make sure your workflow follows privacy and data protection requirements.

### What limits apply?

Submit up to 100 batch entries per run. Use **maxResults** to cap saved items. Results also depend on source availability and your Scrappa API plan.

### Can I start runs through the Apify API?

Yes. Send a POST request to `https://api.apify.com/v2/acts/thescrappa~kayak-hotel-details-scraper/runs` with your Actor input, or use an Apify client library. Read the output from the run dataset.

### Can I connect the results to other tools?

Yes. Apify integrations and APIs can pass dataset results to Make, Zapier, n8n, Google Sheets, and other data workflows.

### What happens when one input fails?

The Actor logs a short source error and continues with the remaining entries. Transient rate limits and gateway errors are retried; if every entry fails, the run reports an error.

### How do I copy a property path?

Use the path portion of a KAYAK hotel URL, including the hotel identifier and `.ksp` suffix.

## Related Scrappa Actors

- [Kayak Hotels Search Scraper](https://apify.com/thescrappa/kayak-hotels-search-scraper)
- [Kayak Hotel Rates Scraper](https://apify.com/thescrappa/kayak-hotel-rates-scraper)
- [Kayak Hotel Reviews Scraper](https://apify.com/thescrappa/kayak-hotel-reviews-scraper)

# Ohne-Makler Listing Details Scraper

Retrieve an Ohne-Makler listing by numeric ID or object number. The record includes asking price, room count, area, condition, energy information, location, gallery, and description.

## What data can you extract?

| Field | Type | Description |
| --- | --- | --- |
| `success` | Boolean | True when the Ohne-Makler request returned a successful response. |
| `listing` | Object | Full Ohne-Makler property record with asking price, features, location, media, and listing details. |
| `title` | String | Name or title assigned to this Ohne-Makler property listing. |
| `price` | Integer | Price amount for this property listing; currency follows the selected source market. |
| `rooms` | Integer | Number of rooms reported for the property. |
| `living_area_m2` | Integer | Living area m2 measurement in the units supplied by Ohne-Makler. |
| `construction_year` | Integer | Year the residential property was built. |
| `location_text` | String | The source location name or code used to place this property listing on Ohne-Makler. |
| `url` | String | Public Ohne-Makler URL for the property listing or linked resource. |
| `input_id` | String | Id submitted to retrieve this property listing. |
| `scraped_at` | String | UTC ISO 8601 timestamp when this dataset record was collected. |

## Use cases

- Property buyers can review full details for listings found in search.
- Investors can compare asking price, area, and construction year across properties.
- Housing analysts can enrich listing IDs with location and energy fields.

## How to use

1. Add a numeric ID or OM-prefixed object number to `listing_ids`.
2. Set `maxResults` to cap the number of detailed property records.
3. Export one complete listing record for each ID.

```json
{
  "listing_ids": [
    {
      "id": "235423"
    }
  ],
  "maxResults": 1
}
```

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `listing_ids` | Array\<object\> | Yes | Ohne-Makler numeric IDs or OM-prefixed object numbers. |
| `listing_ids[].id` | string | Yes per entry | Listing id: numeric \(498791\) or object number \(OM-498791\). |
| `maxResults` | integer | No | Maximum dataset items to save across this entire run. |

## Output example

```json
{
  "success": true,
  "listing": {
    "id": 8000206,
    "object_number": "OM-583214",
    "title": "Bright two-bedroom flat in Berlin",
    "url": "https://www.ohne-makler.net/immobilie/offer/583214/",
    "available": true,
    "price": 549,
    "price_label": "€ 32,900",
    "price_on_request": true,
    "price_breakdown": [
      {
        "label": "Verified source detail",
        "value": "Balcony"
      }
    ],
    "rooms": 2,
    "living_area_m2": 78,
    "plot_area_m2": null,
    "condition": "New",
    "construction_year": 2023,
    "object_art": "Verified source detail",
    "object_type": "Residential property",
    "energy": null,
    "summary": [
      {
        "label": "Verified source detail",
        "value": "Balcony"
      }
    ],
    "seller_type": "Morgenrot Handelskontor",
    "latitude": 52.52,
    "longitude": 13.405,
    "gallery": [
      "Verified source detail"
    ],
    "attachments_available": true,
    "description": "Bright two-bedroom home with a balcony near local transport.",
    "location_text": "Hafenallee 17, 10115 Berlin",
    "sections": [
      {
        "heading": "Verified source detail",
        "tables": [
          {
            "headers": [
              "Verified marketplace detail"
            ],
            "rows": [
              [
                "Verified source detail"
              ]
            ]
          }
        ]
      }
    ],
    "aggregate_rating": {
      "rating_value": "Verified source detail",
      "best_rating": "Verified source detail",
      "rating_count": "Verified source detail"
    }
  },
  "title": "Bright two-bedroom flat in Berlin",
  "price": 549,
  "rooms": 2,
  "living_area_m2": 78,
  "construction_year": 2023,
  "location_text": "Hafenallee 17, 10115 Berlin",
  "url": "https://www.ohne-makler.net/immobilie/offer/583214/",
  "input_id": "235423",
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

Yes. Send a POST request to `https://api.apify.com/v2/acts/thescrappa~ohne-makler-listing-scraper/runs` with your Actor input, or use an Apify client library. Read the output from the run dataset.

### Can I connect the results to other tools?

Yes. Apify integrations and APIs can pass dataset results to Make, Zapier, n8n, Google Sheets, and other data workflows.

### What happens when one input fails?

The Actor logs a short source error and continues with the remaining entries. Transient rate limits and gateway errors are retried; if every entry fails, the run reports an error.

### Can I use an object number instead of a numeric ID?

Yes. The input accepts a numeric listing ID or an object number such as an OM-prefixed value.

## Related Scrappa Actors

- [Ohne-Makler Property Search Scraper](https://apify.com/thescrappa/ohne-makler-search-scraper)
- [Ohne-Makler Property Prices Scraper](https://apify.com/thescrappa/ohne-makler-property-prices-scraper)
- [Idealista Listing Details Scraper](https://apify.com/thescrappa/idealista-listing-scraper)

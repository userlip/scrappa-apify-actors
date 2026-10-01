# ImmobilienScout24 Property Details Scraper

Look up an ImmobilienScout24 listing by property ID. Review rent, rooms, address, features, energy class, and images in one structured record.

## What data can you extract?

| Field | Type | Description |
| --- | --- | --- |
| `success` | Boolean | Indicates whether ImmobilienScout24 returned a property record. |
| `id` | String | ImmobilienScout24 property ID used for this details lookup. |
| `title` | String | Listing headline for the German apartment or property. |
| `description` | String | Advertiser text describing the German property. |
| `type` | String | ImmobilienScout24 listing category for the property. |
| `is_tenant_network` | Boolean | True when the listing is marked as part of a tenant network. |
| `price` | Object | Advertised rent or sale amount and billing period when provided. |
| `details` | Object | Property attributes such as rooms, area, floor, and availability. |
| `address` | Object | Property address and district details, with coordinates when available. |
| `features` | Array\<string\> | Amenities and property features included in the listing. |
| `energy_class` | String | Energy-efficiency class shown for the property. |
| `images` | Array\<object\> | Property image URLs and captions attached to the listing. |
| `agent` | Object | Advertiser or real-estate agent details shown on the listing. |
| `url` | String | Public ImmobilienScout24 page URL for this property. |
| `input_id` | String | Property ID submitted for this details lookup. |
| `scraped_at` | String | UTC date and time when this property record was collected. |

## Use cases

- Rental analysts can enrich listings with rent, location, room, and energy details.
- Property managers can compare features and availability across German listings.
- Real-estate teams can review listing images and advertiser information.

## How to use

1. Add one numeric property ID to `property_ids` for each listing.
2. Set `maxResults` to cap property records saved.
3. Match each dataset item to your property using its ID.

```json
{
  "property_ids": [
    {
      "id": "170528322"
    }
  ],
  "maxResults": 20
}
```

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `property_ids` | Array\<object\> | Yes | Numeric ImmobilienScout24 listing IDs to look up. |
| `property_ids[].id` | string | Yes per entry | ImmobilienScout24 property ID \(digits only\). |
| `maxResults` | integer | No | Maximum dataset items to save across this entire run. |

## Output example

```json
{
  "success": true,
  "id": "170000001",
  "title": "Bright two-bedroom apartment near Stadtpark",
  "description": "A sunny apartment with a balcony, elevator access, and a renovated kitchen.",
  "type": "apartment",
  "is_tenant_network": false,
  "price": {
    "cold_rent": 900,
    "warm_rent": 1100,
    "cold_rent_formatted": "€900",
    "warm_rent_formatted": "€1,100",
    "deposit": 1800,
    "currency": "EUR"
  },
  "details": {
    "rooms": 2,
    "size_m2": 68,
    "floor": 2,
    "available_from": "2026-10-01",
    "year_built": 2018
  },
  "address": {
    "street": "Friedrichstrasse 18",
    "postal_code": "10117",
    "city": "Berlin",
    "district": "Mitte",
    "lat": 52.53,
    "lon": 13.38
  },
  "features": [
    "balcony",
    "elevator"
  ],
  "energy_class": "B",
  "images": [
    {
      "url": "https://www.immobilienscout24.de/expose/170000001/images/room-1",
      "caption": "Living room"
    }
  ],
  "agent": {
    "name": "Nordlicht Immobilien",
    "logo_url": "https://www.immobilienscout24.de/expose/170000001/images/agent-logo"
  },
  "url": "https://www.immobilienscout24.de/expose/170000001",
  "input_id": "170528322",
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

Yes. Send a POST request to `https://api.apify.com/v2/acts/thescrappa~immobilienscout24-property-details-scraper/runs` with your Actor input, or use an Apify client library. Read the output from the run dataset.

### Can I connect the results to other tools?

Yes. Apify integrations and APIs can pass dataset results to Make, Zapier, n8n, Google Sheets, and other data workflows.

### What happens when one input fails?

The Actor logs a short source error and continues with the remaining entries. Transient rate limits and gateway errors are retried; if every entry fails, the run reports an error.

### Which ID should I provide?

Use the numeric ImmobilienScout24 listing ID from the property URL or listing reference.

## Related Scrappa Actors

- [ImmobilienScout24 Search Scraper](https://apify.com/thescrappa/immobilienscout24-search-scraper)
- [ImmobilienScout24 Price Insights Scraper](https://apify.com/thescrappa/immobilienscout24-price-insights-scraper)
- [Immowelt Property Search Scraper](https://apify.com/thescrappa/immowelt-property-search-scraper)

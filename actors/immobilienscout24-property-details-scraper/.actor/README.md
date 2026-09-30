# ImmobilienScout24 Property Details Scraper

Retrieve detailed ImmobilienScout24 property data by listing ID.

## Data you get

- **id**: Source property or product identifier.
- **title**: Result title or listing name.
- **price**: Listed product or property price.
- **details**: Property size, rooms, floor, and availability.
- **address**: Property address fields when supplied.
- **url**: Canonical result URL.

## Use cases

- Property listing enrichment
- Real estate research
- Listing change monitoring

## How to use

Add one or more entries to **property_ids**. Each entry maps its **id** value to the Scrappa **** input. Shared endpoint options can be set at the top level.

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

## Output example

This synthetic example shows the response fields and the input value attached to each result.

```json
{
  "success": true,
  "id": "170000001",
  "title": "Example apartment in Example City",
  "description": "Synthetic property description.",
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
    "street": "Example Street",
    "postal_code": "10115",
    "city": "Example City",
    "district": "Example District",
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
      "url": "https://example.com/room.jpg",
      "caption": "Living room"
    }
  ],
  "agent": {
    "name": "Example Realty",
    "logo_url": "https://example.com/logo.png"
  },
  "url": "https://www.immobilienscout24.de/expose/170000001",
  "input_id": "170528322",
  "scraped_at": "2026-01-01T00:00:00Z"
}
```

## Pricing

$0.50 per 1,000 results. The Actor writes one dataset item for each result.

The Actor saves up to **maxResults** dataset items across the run.

## FAQ

### Is scraping this data legal?

Scraping rules depend on the source, the data, and how you use it. Review the applicable laws, source terms, and privacy requirements for your use case. You are responsible for your collection and use of the data.

### Are there request limits?

You can submit up to 100 batch entries per run. Set **maxResults** to cap saved rows. Scrappa API limits and source availability also apply.

### Can I use the output with integrations or the API?

Yes. Read results from the Apify dataset, use the Apify API or client libraries, or connect the dataset to your existing data workflow. Each row includes **input_id** and **scraped_at** for traceability.

## Related Actors

- [Immobilienscout24 Search Scraper](https://apify.com/thescrappa/immobilienscout24-search-scraper)
- [Redfin Property Details Scraper](https://apify.com/thescrappa/redfin-property-details-scraper)

## Search terms

`ImmobilienScout24 Property Details Scraper`, `id`, `title`, `price`, `/immobilienscout24/property/{id} API`

# Idealista Property Search Scraper

Search Idealista properties for sale or rent across Spain, Italy, Portugal, and France.

## Data you get

- **propertyCode**: Idealista listing code.
- **price**: Listed product or property price.
- **size**: Property area in square meters.
- **rooms**: Number of rooms.
- **municipality**: Municipality containing the property.
- **url**: Canonical result URL.

## Use cases

- Property market research
- Rental and sale listing monitoring
- Real estate lead discovery

## How to use

Add one or more entries to **location_ids**. Each entry maps its **location_id** value to the Scrappa **location_id** input. Shared endpoint options can be set at the top level.

```json
{
  "location_ids": [
    {
      "location_id": "0-EU-ES-28-07-001-079"
    }
  ],
  "country": "es",
  "operation": "sale",
  "property_type": "homes",
  "maxResults": 20,
  "maxPages": 2
}
```

## Output example

This synthetic example shows the response fields and the input value attached to each result.

```json
{
  "propertyCode": "10000001",
  "thumbnail": "https://example.com/property.jpg",
  "numPhotos": 8,
  "price": 250000,
  "propertyType": "flat",
  "operation": "sale",
  "size": 75,
  "rooms": 3,
  "bathrooms": 1,
  "municipality": "Example City",
  "province": "Example Province",
  "country": "Spain",
  "locationId": "example-location",
  "latitude": 40.4,
  "longitude": -3.7,
  "url": "https://www.idealista.com/en/inmueble/10000001/",
  "description": "Synthetic property description.",
  "input_location_id": "0-EU-ES-28-07-001-079",
  "scraped_at": "2026-01-01T00:00:00Z"
}
```

## Pricing

$0.30 per 1,000 results. The Actor writes one dataset item for each result.

This Actor supports pagination and stops at the configured **maxPages** or **maxResults** limit.

## FAQ

### Is scraping this data legal?

Scraping rules depend on the source, the data, and how you use it. Review the applicable laws, source terms, and privacy requirements for your use case. You are responsible for your collection and use of the data.

### Are there request limits?

You can submit up to 100 batch entries per run. Set **maxResults** to cap saved rows and **maxPages** to bound pagination for each entry. Scrappa API limits and source availability also apply.

### Can I use the output with integrations or the API?

Yes. Read results from the Apify dataset, use the Apify API or client libraries, or connect the dataset to your existing data workflow. Each row includes **input_location_id** and **scraped_at** for traceability.

## Related Actors

- [Immobilienscout24 Search Scraper](https://apify.com/thescrappa/immobilienscout24-search-scraper)
- [Immowelt Property Search Scraper](https://apify.com/thescrappa/immowelt-property-search-scraper)

## Search terms

`Idealista Property Search Scraper`, `propertyCode`, `price`, `size`, `/idealista/search API`

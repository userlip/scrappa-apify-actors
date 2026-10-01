# Idealista Property Search Scraper

Search Idealista by location, operation, and property type. Collect listing IDs, prices, sizes, rooms, locations, coordinates, photo totals, and links across pages.

## What data can you extract?

| Field | Type | Description |
| --- | --- | --- |
| `propertyCode` | String | Unique Idealista code identifying this property listing. |
| `thumbnail` | String | Preview image URL for the Idealista property. |
| `numPhotos` | Integer | Number of property photos attached to the listing. |
| `price` | Integer | Advertised sale or rent price in the listing currency. |
| `propertyType` | String | Idealista property category, such as a home, room, or office. |
| `operation` | String | Whether the listing advertises a sale or rental. |
| `size` | Integer | Advertised property area in square metres when reported. |
| `rooms` | Integer | Number of rooms reported in the property listing. |
| `bathrooms` | Integer | Number of bathrooms reported in the property listing. |
| `municipality` | String | Municipality where Idealista places the property. |
| `province` | String | Province containing the property listing. |
| `country` | String | Country associated with this Idealista result. |
| `locationId` | String | Idealista location code for the listing area. |
| `latitude` | Number | Latitude of the property location in decimal degrees. |
| `longitude` | Number | Longitude of the property location in decimal degrees. |
| `url` | String | Public Idealista page URL for this property. |
| `description` | String | Listing summary describing the property and its features. |
| `input_location_id` | String | Idealista location code searched for this result set. |
| `scraped_at` | String | UTC date and time when this property listing was collected. |

## Use cases

- Property investors can compare listing prices, areas, and room counts by location.
- Housing analysts can track supply across Idealista markets.
- Real-estate teams can collect listing links and photos for property shortlists.

## How to use

1. Add a location ID to `location_ids` for each area.
2. Choose country, operation, and property type, then add filters.
3. Set `maxPages` and `maxResults`, then export listing rows.

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

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `location_ids` | Array\<object\> | Yes | Idealista location IDs to search. |
| `location_ids[].location_id` | string | Yes per entry | Idealista location id such as 0-EU-ES-28. |
| `location_ids[].country` | string | No | Market: es, it, pt, or fr. Default es. |
| `location_ids[].operation` | string | No | sale or rent. Room stock uses bedrooms + rent. |
| `location_ids[].property_type` | string | No | homes, newDevelopments, bedrooms, offices, premises, transfers, buildings, storageRooms, garages, vacationRentals, luxury, or lands. |
| `location_ids[].center` | string | No | lat,lon for near-me search. |
| `location_ids[].distance` | integer | No | Radius in meters when using center. |
| `location_ids[].max_items` | integer | No | Page size honored by origin, 1-5. |
| `location_ids[].num_page` | integer | No | 1-based page number. |
| `location_ids[].order` | string | No | Sort direction or ordering mode for property results. |
| `location_ids[].sort` | string | No | Listing sort mode, such as price or publication date. |
| `location_ids[].quality` | string | No | Filter results by listing quality when supported. |
| `location_ids[].gallery` | boolean | No | Filter for listings that include gallery photos. |
| `location_ids[].locale` | string | No | Language and regional formatting for the listing search. |
| `location_ids[].microsite_short_name` | string | No | Anonymous agency stock filter, for example clikalia. |
| `location_ids[].chalet` | boolean | No | Apply-filter key. Send true on homes. |
| `location_ids[].current_occupation_type` | string | No | free, bareOwnership, tenanted, illegallyOccupied, or viager. |
| `location_ids[].auction` | string | No | IT only: onlyAuctions or excludeAuctions. |
| `location_ids[].home_type` | string | No | Idealista home subtype, such as an apartment or house. |
| `location_ids[].bank_offer` | string | No | Filter listings associated with a bank offer. |
| `location_ids[].advertiser_type` | string | No | Filter results by advertiser category. |
| `location_ids[].min_price` | integer | No | Minimum asking price for a property listing. |
| `location_ids[].max_price` | integer | No | Maximum asking price for a property listing. |
| `location_ids[].min_size` | integer | No | Minimum property area in square metres. |
| `location_ids[].max_size` | integer | No | Maximum property area in square metres. |
| `location_ids[].min_rooms` | integer | No | Minimum room count for a property listing. |
| `location_ids[].max_rooms` | integer | No | Maximum room count for a property listing. |
| `location_ids[].shape` | string | No | Geographic search shape used to restrict Idealista results. |
| `country` | string | Yes | Market: es, it, pt, or fr. Default es. |
| `operation` | string | Yes | sale or rent. Room stock uses bedrooms + rent. |
| `property_type` | string | Yes | homes, newDevelopments, bedrooms, offices, premises, transfers, buildings, storageRooms, garages, vacationRentals, luxury, or lands. |
| `center` | string | No | lat,lon for near-me search. |
| `distance` | integer | No | Radius in meters when using center. |
| `max_items` | integer | No | Page size honored by origin, 1-5. |
| `num_page` | integer | No | 1-based page number. |
| `order` | string | No | Sort direction or ordering mode for property results. |
| `sort` | string | No | Listing sort mode, such as price or publication date. |
| `quality` | string | No | Filter results by listing quality when supported. |
| `gallery` | boolean | No | Filter for listings that include gallery photos. |
| `locale` | string | No | Language and regional formatting for the listing search. |
| `microsite_short_name` | string | No | Anonymous agency stock filter, for example clikalia. |
| `chalet` | boolean | No | Apply-filter key. Send true on homes. |
| `current_occupation_type` | string | No | free, bareOwnership, tenanted, illegallyOccupied, or viager. |
| `auction` | string | No | IT only: onlyAuctions or excludeAuctions. |
| `home_type` | string | No | Idealista home subtype, such as an apartment or house. |
| `bank_offer` | string | No | Filter listings associated with a bank offer. |
| `advertiser_type` | string | No | Filter results by advertiser category. |
| `min_price` | integer | No | Minimum asking price for a property listing. |
| `max_price` | integer | No | Maximum asking price for a property listing. |
| `min_size` | integer | No | Minimum property area in square metres. |
| `max_size` | integer | No | Maximum property area in square metres. |
| `min_rooms` | integer | No | Minimum room count for a property listing. |
| `max_rooms` | integer | No | Maximum room count for a property listing. |
| `shape` | string | No | Geographic search shape used to restrict Idealista results. |
| `maxResults` | integer | No | Maximum dataset items to save across this entire run. |
| `maxPages` | integer | No | Maximum pages to request for each batch entry. |

## Output example

```json
{
  "propertyCode": "10000001",
  "thumbnail": "https://images.unsplash.com/photo-1600607687939-ce8a6c25118c",
  "numPhotos": 8,
  "price": 250000,
  "propertyType": "flat",
  "operation": "sale",
  "size": 75,
  "rooms": 3,
  "bathrooms": 1,
  "municipality": "Madrid",
  "province": "Madrid",
  "country": "Spain",
  "locationId": "0-EU-ES-28",
  "latitude": 40.4,
  "longitude": -3.7,
  "url": "https://www.idealista.com/en/inmueble/10000001/",
  "description": "Bright three-room apartment with a balcony near public transport.",
  "input_location_id": "0-EU-ES-28-07-001-079",
  "scraped_at": "2026-10-01T12:00:00Z"
}
```

## Pricing

$0.30 per 1,000 results. Apify saves one dataset item per result and applies the Actor’s per-result price to saved items.

Pagination follows the source response. Set **maxPages** per batch entry and **maxResults** across the run.

## FAQ

### Is scraping this information legal?

Rules depend on the source, location, data type, and intended use. Check applicable laws and source terms, and make sure your workflow follows privacy and data protection requirements.

### What limits apply?

Submit up to 100 batch entries per run. Use **maxResults** to cap saved items and **maxPages** to limit pages for each entry. Results also depend on source availability and your Scrappa API plan.

### Can I start runs through the Apify API?

Yes. Send a POST request to `https://api.apify.com/v2/acts/thescrappa~idealista-search-scraper/runs` with your Actor input, or use an Apify client library. Read the output from the run dataset.

### Can I connect the results to other tools?

Yes. Apify integrations and APIs can pass dataset results to Make, Zapier, n8n, Google Sheets, and other data workflows.

### What happens when one input fails?

The Actor logs a short source error and continues with the remaining entries. Transient rate limits and gateway errors are retried; if every entry fails, the run reports an error.

### How does pagination stop?

Collection stops when the source reports no next page, the reported page does not advance, or your configured limits are reached.

## Related Scrappa Actors

- [ImmobilienScout24 Search Scraper](https://apify.com/thescrappa/immobilienscout24-search-scraper)
- [Immowelt Property Search Scraper](https://apify.com/thescrappa/immowelt-property-search-scraper)
- [Redfin Property Search Scraper](https://apify.com/thescrappa/redfin-property-search-scraper)

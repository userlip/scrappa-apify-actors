# Realestate.com.au Property Search Scraper

Search realestate.com.au by Australian suburb or locality and collect listing prices, addresses, property features, agencies, inspection times, and URLs. Choose buy, rent, or sold results.

## What data can you extract?

| Field | Type | Description |
| --- | --- | --- |
| `id` | String | realestate.com.au listing identifier. |
| `name` | String or null | Listing or development name, when supplied. |
| `url` | String | Public realestate.com.au URL for the property listing. |
| `property_type` | String | Property type such as house, apartment, or townhouse. |
| `description` | String | Listing description shown by the advertiser. |
| `address` | Object | Property address details, including suburb, state, postcode, and display address. |
| `price` | String | Advertised price text or price guide shown on the listing. |
| `features` | Object | Property feature counts, including bedrooms, bathrooms, and parking spaces. |
| `property_sizes` | Object | Land and building area measurements, when supplied. |
| `media` | Object | Listing images, floorplans, tours, and other media links. |
| `listing_company` | Object | Agency details associated with the listing. |
| `child_listings` | String or null | Related listings grouped under the same property record, when present. |
| `listers` | Array\<Object\> | Agent records attached to the listing. |
| `auction` | Object or null | Auction date and online auction links, when scheduled. |
| `inspections` | Array\<Object\> | Inspection dates and times advertised for the property. |
| `sold_date` | String or null | Date the property sold, for sold listings. |
| `input_location` | String | Location submitted to retrieve this property listing. |
| `scraped_at` | String | UTC ISO 8601 timestamp when this dataset record was collected. |

## Use cases

- Home buyers can compare listings and features across nearby suburbs.
- Agents can monitor prices and competing inventory by location.
- Property researchers can collect sold, rental, and sale listings for market analysis.

## How to use

1. Add one suburb or location to `locations` for each search.
2. Choose a buy, rent, or sold channel and optional property filters.
3. Set `maxPages` and `maxResults`, then export the listing rows.

```json
{
  "locations": [
    {
      "location": "Richmond, VIC 3121"
    }
  ],
  "channel": "buy",
  "page": 1,
  "maxResults": 5,
  "maxPages": 1
}
```

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `locations` | Array\<object\> | Yes | Suburbs or locality text to search on realestate.com.au. |
| `locations[].location` | string | Yes per entry | Location search text such as Sydney, NSW. |
| `locations[].channel` | string | No | Listing channel: buy, rent, or sold. |
| `locations[].page` | integer | No | Page number. |
| `locations[].sort` | string | No | realestate.com.au sort type, such as new-desc or sold-date-desc. |
| `locations[].min_price` | integer | No | Minimum price filter. |
| `locations[].max_price` | integer | No | Maximum price filter. |
| `locations[].min_bedrooms` | integer | No | Minimum bedroom count. |
| `locations[].max_bedrooms` | integer | No | Maximum bedroom count. |
| `locations[].min_bathrooms` | integer | No | Minimum bathroom count. |
| `locations[].min_carspaces` | integer | No | Minimum parking spaces. |
| `locations[].property_types` | string | No | Comma-separated realestate.com.au property type filters. |
| `locations[].keywords` | string | No | Comma-separated keyword filters. |
| `channel` | string | No | Listing channel: buy, rent, or sold. |
| `page` | integer | No | Page number. |
| `sort` | string | No | realestate.com.au sort type, such as new-desc or sold-date-desc. |
| `min_price` | integer | No | Minimum price filter. |
| `max_price` | integer | No | Maximum price filter. |
| `min_bedrooms` | integer | No | Minimum bedroom count. |
| `max_bedrooms` | integer | No | Maximum bedroom count. |
| `min_bathrooms` | integer | No | Minimum bathroom count. |
| `min_carspaces` | integer | No | Minimum parking spaces. |
| `property_types` | string | No | Comma-separated realestate.com.au property type filters. |
| `keywords` | string | No | Comma-separated keyword filters. |
| `maxResults` | integer | No | Maximum dataset items to save across this entire run. |
| `maxPages` | integer | No | Maximum pages to request for each batch entry. |

## Output example

```json
{
  "id": "152000001",
  "name": null,
  "url": "https://www.realestate.com.au/property-house-vic-richmond-152000001",
  "property_type": "Residential property",
  "description": "Bright two-bedroom home with a balcony near local transport.",
  "address": {
    "display": {
      "shortAddress": "24 Bridge Road, Richmond VIC 3121",
      "fullAddress": "24 Bridge Road, Richmond VIC 3121",
      "__typename": "PropertyAddress"
    },
    "suburb": "Richmond",
    "state": "VIC",
    "postcode": "3121",
    "__typename": "PropertyAddress"
  },
  "price": "A$720,000",
  "features": {
    "bedrooms": {
      "value": 8,
      "__typename": "PropertyFeatures"
    },
    "__typename": "PropertyFeatures",
    "bathrooms": {
      "value": 8,
      "__typename": "PropertyFeatures"
    },
    "parkingSpaces": {
      "value": 8,
      "__typename": "PropertyFeatures"
    },
    "studies": {
      "value": 8,
      "__typename": "PropertyFeatures"
    }
  },
  "property_sizes": {
    "land": {
      "displayValue": "Richmond, VIC 3121",
      "sizeUnit": {
        "displayValue": "Richmond, VIC 3121",
        "__typename": "PropertyListing"
      },
      "__typename": "PropertyListing"
    },
    "__typename": "PropertyListing",
    "building": null,
    "preferred": {
      "sizeType": "LAND",
      "size": {
        "displayValue": "Richmond, VIC 3121",
        "sizeUnit": {
          "displayValue": "Richmond, VIC 3121",
          "__typename": "PropertyListing"
        },
        "__typename": "PropertyListing"
      },
      "__typename": "PropertyListing"
    }
  },
  "media": {
    "mainImage": {
      "templatedUrl": "https://www.realestate.com.au/property-house-vic-richmond-152000001",
      "__typename": "SourceRecord"
    },
    "images": [
      {
        "__typename": "SourceRecord",
        "templatedUrl": "https://www.realestate.com.au/property-house-vic-richmond-152000001"
      }
    ],
    "__typename": "SourceRecord",
    "statementOfInformation": {
      "href": "https://www.realestate.com.au/property-house-vic-richmond-152000001",
      "__typename": "SourceRecord"
    },
    "floorplans": [
      {
        "templatedUrl": "https://www.realestate.com.au/property-house-vic-richmond-152000001",
        "__typename": "SourceRecord"
      }
    ],
    "threeDimensionalToursCompat": [
      {
        "url": "https://media.northstar.invalid/tour/aurora",
        "title": "Property walkthrough"
      }
    ],
    "videos": [
      "Verified marketplace detail"
    ],
    "threeDimensionalTours": [
      {
        "url": "https://media.northstar.invalid/tour/aurora",
        "title": "Property walkthrough"
      }
    ]
  },
  "listing_company": {
    "name": "Harbourline Property Group",
    "_links": {
      "canonical": {
        "href": "https://www.realestate.com.au/property-house-vic-richmond-152000001",
        "__typename": "PropertyListing"
      },
      "__typename": "PropertyListing"
    },
    "__typename": "PropertyListing",
    "id": "152000001",
    "branding": {
      "primaryColour": "Harbourline Property Group",
      "__typename": "PropertyListing",
      "textColour": "Harbourline Property Group"
    },
    "media": {
      "logo": {
        "templatedUrl": "https://www.realestate.com.au/property-house-vic-richmond-152000001",
        "__typename": "PropertyListing"
      },
      "__typename": "PropertyListing",
      "mainImage": {
        "templatedUrl": "https://www.realestate.com.au/property-house-vic-richmond-152000001",
        "__typename": "PropertyListing"
      }
    },
    "address": {
      "display": {
        "fullAddress": "24 Bridge Road, Richmond VIC 3121",
        "__typename": "PropertyAddress"
      },
      "__typename": "PropertyAddress"
    },
    "ratingsReviews": {
      "avgRating": 94,
      "totalReviews": 28,
      "__typename": "PropertyListing"
    }
  },
  "child_listings": null,
  "listers": [
    {
      "id": "152000001",
      "agentId": "152000001",
      "name": "Jordan Ellis",
      "jobTitle": "Morgan Lee",
      "photo": {
        "templatedUrl": "https://www.realestate.com.au/property-house-vic-richmond-152000001",
        "__typename": "SourceRecord"
      },
      "_links": {
        "canonical": {
          "href": "https://www.realestate.com.au/property-house-vic-richmond-152000001",
          "__typename": "SourceRecord"
        },
        "__typename": "SourceRecord"
      },
      "showInMediaViewer": true,
      "__typename": "SourceRecord"
    }
  ],
  "auction": {
    "dateTime": {
      "value": "Balcony",
      "__typename": "SourceRecord",
      "display": {
        "longLabel": "Verified source detail",
        "__typename": "SourceRecord",
        "shortLabel": "Verified source detail",
        "time": "Verified source detail"
      }
    },
    "__typename": "SourceRecord",
    "onlineLinks": [
      "Verified marketplace detail"
    ]
  },
  "inspections": [
    {
      "startTime": "Verified source detail",
      "endTime": "Verified source detail",
      "__typename": "SourceRecord",
      "display": {
        "longLabel": "Verified source detail",
        "__typename": "SourceRecord",
        "shortLabel": "Verified source detail"
      }
    }
  ],
  "sold_date": null,
  "input_location": "Richmond, VIC 3121",
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

Yes. Send a POST request to `https://api.apify.com/v2/acts/thescrappa~realestate-com-au-search-scraper/runs` with your Actor input, or use an Apify client library. Read the output from the run dataset.

### Can I connect the results to other tools?

Yes. Apify integrations and APIs can pass dataset results to Make, Zapier, n8n, Google Sheets, and other data workflows.

### What happens when one input fails?

The Actor logs a short source error and continues with the remaining entries. Transient rate limits and gateway errors are retried; if every entry fails, the run reports an error.

### Which listing channels are available?

The source accepts buy, rent, and sold channels. Results depend on what is currently listed for the selected location.

## Related Scrappa Actors

- [Realestate.com.au Property Details Scraper](https://apify.com/thescrappa/realestate-com-au-property-scraper)
- [Realestate.com.au Agents Scraper](https://apify.com/thescrappa/realestate-com-au-agents-scraper)
- [Redfin Property Search Scraper](https://apify.com/thescrappa/redfin-property-search-scraper)

# mobile.de Car Search Scraper

Search mobile.de for cars by keyword and available filters. Export vehicle titles, make and model, price, mileage, registration details, images, and listing identifiers.

## What data can you extract?

| Field | Type | Description |
| --- | --- | --- |
| `isEyeCatcher` | Boolean | True when mobile.de marks the listing as an eye-catching result. |
| `numImages` | Integer | Number of vehicle images available for the listing. |
| `attr` | Object | Vehicle attributes encoded by mobile.de, including make, model, fuel, mileage, and registration details. |
| `siteId` | String | mobile.de site identifier associated with the listing. |
| `p` | String | Source vehicle filter value returned with the listing. |
| `st` | String | Vehicle body or segment code returned with the listing. |
| `shortTitle` | String | Compact vehicle title displayed in search results. |
| `subTitle` | String | Secondary vehicle title with additional model or trim details. |
| `hasDamage` | Boolean | True when the listing declares vehicle damage. |
| `isVideoEnabled` | Boolean | True when a video is available for the listing. |
| `readyToDrive` | Boolean | True when the listing marks the vehicle as ready to drive. |
| `created` | Integer | Unix timestamp in milliseconds when the listing was created. |
| `modified` | Integer | Unix timestamp in milliseconds when the listing was last modified. |
| `renewed` | Integer | Unix timestamp in milliseconds when the listing was renewed. |
| `version` | Integer | mobile.de listing schema version. |
| `makeId` | Integer | mobile.de identifier for the vehicle make. |
| `modelId` | Integer | mobile.de identifier for the vehicle model. |
| `make` | Object | Vehicle make name and localized label. |
| `model` | Object | Vehicle model name and localized label. |
| `financePlans` | Array\<Object\> | Financing plans attached to the vehicle listing. |
| `images` | Array\<Object\> | Vehicle image records with image URLs and dimensions. |
| `sellerId` | Integer | mobile.de identifier for the seller. |
| `priceRating` | Object | mobile.de assessment of the advertised price compared with similar vehicles. |
| `segment` | String | Vehicle market segment assigned by mobile.de. |
| `title` | String | Full vehicle title shown on the listing. |
| `url` | String | mobile.de URL for the vehicle listing. |
| `vc` | String | mobile.de vehicle category code. |
| `category` | String | Vehicle category assigned to the listing. |
| `id` | Integer | mobile.de numeric identifier for the vehicle listing. |
| `price` | Object | Advertised vehicle price and currency details. |
| `kba` | Object | German Federal Motor Transport Authority vehicle type code, when reported. |
| `listing_id` | String | mobile.de listing identifier used for a detail lookup. |
| `badges` | Array\<String\> | Badges displayed on the vehicle listing. |
| `isDamageCase` | Boolean | True when mobile.de classifies the listing as a damage case. |
| `deliveryOption` | String | Delivery choice offered for the vehicle. |
| `nationalDelivery` | Object | National delivery details, when offered. |
| `highlights` | Array\<String\> | Vehicle features highlighted by the seller. |
| `isNew` | Boolean | True when the vehicle is classified as new. |
| `input_query` | String | Query submitted to retrieve this vehicle listing. |
| `scraped_at` | String | UTC ISO 8601 timestamp when this dataset record was collected. |

## Use cases

- Car buyers can compare listings by model, price, year, and mileage.
- Dealers can monitor comparable stock for selected makes and models.
- Automotive analysts can collect marketplace inventory for pricing studies.

## How to use

1. Add a vehicle query to `searches` for each search.
2. Set optional price, registration-year, mileage, make, or model filters.
3. Choose the page size and result cap, then export matching car rows.

```json
{
  "searches": [
    {
      "query": "golf"
    }
  ],
  "vehicle_type": "car",
  "offset": 0,
  "limit": 5,
  "sort": "relevance",
  "maxResults": 5,
  "maxPages": 1
}
```

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `searches` | Array\<object\> | Yes | Free-text vehicle queries for mobile.de car listings. |
| `searches[].query` | string | Yes per entry | Optional free-text vehicle search. |
| `searches[].vehicle_type` | string | No | Vehicle type. v1 supports car. |
| `searches[].offset` | integer | No | Zero-based result offset. Defaults to 0. |
| `searches[].limit` | integer | No | Listings to return, from 1 to 20. |
| `searches[].price_min` | integer | No | Optional non-negative minimum price. |
| `searches[].price_max` | integer | No | Optional non-negative maximum price. |
| `searches[].registration_year_min` | integer | No | Optional minimum first-registration year. |
| `searches[].registration_year_max` | integer | No | Optional maximum first-registration year. |
| `searches[].mileage_min` | integer | No | Optional non-negative minimum mileage. |
| `searches[].mileage_max` | integer | No | Optional non-negative maximum mileage. |
| `searches[].make_id` | string | No | Validated marketplace make identifier. |
| `searches[].model_id` | string | No | Validated marketplace model identifier. Requires make\_id. |
| `searches[].sort` | string | No | relevance, price\_asc, or price\_desc. |
| `vehicle_type` | string | No | Vehicle type. v1 supports car. |
| `offset` | integer | No | Zero-based result offset. Defaults to 0. |
| `limit` | integer | No | Listings to return, from 1 to 20. |
| `price_min` | integer | No | Optional non-negative minimum price. |
| `price_max` | integer | No | Optional non-negative maximum price. |
| `registration_year_min` | integer | No | Optional minimum first-registration year. |
| `registration_year_max` | integer | No | Optional maximum first-registration year. |
| `mileage_min` | integer | No | Optional non-negative minimum mileage. |
| `mileage_max` | integer | No | Optional non-negative maximum mileage. |
| `make_id` | string | No | Validated marketplace make identifier. |
| `model_id` | string | No | Validated marketplace model identifier. Requires make\_id. |
| `sort` | string | No | relevance, price\_asc, or price\_desc. |
| `maxResults` | integer | No | Maximum dataset items to save across this entire run. |
| `maxPages` | integer | No | Maximum pages to request for each batch entry. |

## Output example

```json
{
  "isEyeCatcher": true,
  "numImages": 9,
  "attr": {
    "cn": "Volvo",
    "z": "XC60",
    "loc": "Berlin",
    "fr": "2023",
    "pw": "145 kW",
    "ft": "DIESEL",
    "ml": "64,000 km",
    "cc": "1,969 cc",
    "tr": "AUTOMATIC_GEAR",
    "gi": "6-speed automatic",
    "ecol": "Verified source detail",
    "eu": "EURO6",
    "door": "5",
    "sc": "4",
    "c": "Used",
    "emc": "201",
    "pvo": "4",
    "nw": "140 kW"
  },
  "siteId": "DE-29185",
  "p": "€ 32,900",
  "st": "SUV",
  "shortTitle": "2023 Volvo XC60 B4 AWD",
  "subTitle": "Verified source detail",
  "hasDamage": false,
  "isVideoEnabled": true,
  "readyToDrive": true,
  "created": 1790950400,
  "modified": 1791036800,
  "renewed": 1791123200,
  "version": 1,
  "makeId": 8000634,
  "modelId": 8000750,
  "make": {
    "id": "457981580",
    "localized": "Volvo"
  },
  "model": {
    "id": "457981580",
    "localized": "Volvo"
  },
  "financePlans": [
    {
      "type": "Supplier",
      "url": "https://www.mobile.de/angebote/volvo-xc60-b4-awd",
      "shortFlow": true,
      "promotion": true,
      "showInGallery": true,
      "offer": {
        "bankName": "Northstar Outdoor Supply",
        "loanBroker": "Verified source detail",
        "loanType": "Residential property",
        "downPayment": 8,
        "creditTerm": 8,
        "yearlyMileage": 2023,
        "creditAmount": 8
      }
    }
  ],
  "images": [
    {
      "uri": "https://cdn.northstar.invalid/images/aurora-front.webp"
    }
  ],
  "sellerId": 8000873,
  "priceRating": {
    "rating": "Verified source detail",
    "ratingLabel": "Verified source detail",
    "thresholdLabels": [
      "Verified source detail"
    ],
    "vehiclePriceOffset": 629
  },
  "segment": "SUV",
  "title": "2023 Volvo XC60 B4 AWD",
  "url": "https://www.mobile.de/angebote/volvo-xc60-b4-awd",
  "vc": "SUV",
  "category": "Industrial supplies",
  "id": 8000233,
  "price": {
    "grs": {
      "amount": 549,
      "currency": "EUR",
      "localized": "Volvo"
    },
    "type": "Supplier"
  },
  "kba": {
    "hsn": "Current listing detail",
    "tsn": "Current listing detail"
  },
  "listing_id": "457981580",
  "badges": [
    "Verified source detail"
  ],
  "isDamageCase": false,
  "deliveryOption": "Verified source detail",
  "nationalDelivery": {
    "radius": "Verified source detail",
    "period": "Verified source detail"
  },
  "highlights": [
    "Verified source detail"
  ],
  "isNew": true,
  "input_query": "golf",
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

Yes. Send a POST request to `https://api.apify.com/v2/acts/thescrappa~mobile-de-search-scraper/runs` with your Actor input, or use an Apify client library. Read the output from the run dataset.

### Can I connect the results to other tools?

Yes. Apify integrations and APIs can pass dataset results to Make, Zapier, n8n, Google Sheets, and other data workflows.

### What happens when one input fails?

The Actor logs a short source error and continues with the remaining entries. Transient rate limits and gateway errors are retried; if every entry fails, the run reports an error.

### Can I search vehicles other than cars?

The current mobile.de v1 search accepts car inventory. Choose car as the vehicle type or leave the default.

## Related Scrappa Actors

- [mobile.de Listing Details Scraper](https://apify.com/thescrappa/mobile-de-listing-scraper)
- [mobile.de Dealer Inventory Scraper](https://apify.com/thescrappa/mobile-de-dealer-inventory-scraper)
- [Google Search Scraper](https://apify.com/thescrappa/google-search)

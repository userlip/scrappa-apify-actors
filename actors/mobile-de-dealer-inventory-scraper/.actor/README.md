# mobile.de Dealer Inventory Scraper

Retrieve cars offered by mobile.de dealers, including vehicle titles, make and model, prices, mileage, registration, and listing links. Batch dealer IDs to compare inventory.

## What data can you extract?

| Field | Type | Description |
| --- | --- | --- |
| `isEyeCatcher` | Boolean | Flag indicating whether mobile.de marks this vehicle listing as is eye catcher. |
| `numImages` | Integer | Number of num images reported for this vehicle listing. |
| `attr` | Object | Compact mobile.de vehicle attributes, including make, model, mileage, fuel, and transmission codes. |
| `siteId` | String | mobile.de listing identifier used in the public vehicle URL. |
| `p` | String | Formatted advertised vehicle price and currency. |
| `st` | String | Vehicle body style, such as SUV or estate. |
| `shortTitle` | String | Short vehicle title shown in dealer inventory results. |
| `subTitle` | String | Additional vehicle trim or engine details shown below the title. |
| `hasDamage` | Boolean | Flag indicating whether mobile.de marks this vehicle listing as has damage. |
| `isVideoEnabled` | Boolean | Flag indicating whether mobile.de marks this vehicle listing as is video enabled. |
| `readyToDrive` | Boolean | Flag indicating whether mobile.de marks this vehicle listing as ready to drive. |
| `created` | Integer | Unix timestamp in seconds when the listing was created. |
| `modified` | Integer | Unix timestamp in seconds when the listing was last changed. |
| `renewed` | Integer | Unix timestamp in seconds when the listing was renewed. |
| `version` | Integer | mobile.de listing record version number. |
| `makeId` | Integer | mobile.de identifier for the vehicle manufacturer. |
| `modelId` | Integer | mobile.de identifier for the vehicle model. |
| `make` | Object | Vehicle manufacturer identifier and localized name. |
| `model` | Object | Vehicle model identifier and localized name. |
| `financePlans` | Array\<Object\> | List of finance plans records for this vehicle listing; each record carries `type`, `url`, `shortFlow`. |
| `images` | Array\<Object\> | List of images records for this vehicle listing; each record carries `uri`. |
| `sellerId` | Integer | mobile.de identifier for the selling dealer or private seller. |
| `priceRating` | Object | Price rating amount for this vehicle listing; currency follows the selected source market. |
| `segment` | String | Vehicle segment classification returned by mobile.de. |
| `title` | String | Name or title assigned to this mobile.de vehicle listing. |
| `url` | String | Public mobile.de URL for the vehicle listing or linked resource. |
| `vc` | String | Vehicle category code returned by mobile.de. |
| `category` | String | Product or listing category assigned by mobile.de. |
| `id` | Integer | Numeric identifier for this mobile.de vehicle listing. |
| `price` | Object | Price amount for this vehicle listing; currency follows the selected source market. |
| `kba` | Object | German vehicle registration identifiers, including HSN and TSN. |
| `listing_id` | String | Listing id identifier for this mobile.de vehicle listing. |
| `isDamageCase` | Boolean | Flag indicating whether mobile.de marks this vehicle listing as is damage case. |
| `vat` | String | Value-added tax details for the vehicle price. |
| `isNew` | Boolean | Flag indicating whether mobile.de marks this vehicle listing as is new. |
| `input_dealer_id` | String | Dealer id submitted to retrieve this vehicle listing. |
| `scraped_at` | String | UTC ISO 8601 timestamp when this dataset record was collected. |

## Use cases

- Dealers can benchmark their stock against other mobile.de sellers.
- Automotive researchers can compare inventory depth by dealer.
- Car buyers can review vehicles listed by a known dealer before visiting.

## How to use

1. Add one mobile.de dealer ID to `dealer_ids` per inventory search.
2. Set a page size and optional price, year, mileage, or model filters.
3. Limit pages and results, then export the vehicle rows.

```json
{
  "dealer_ids": [
    {
      "dealer_id": "47546627"
    }
  ],
  "vehicle_type": "car",
  "offset": 0,
  "limit": 5,
  "maxResults": 5,
  "maxPages": 1
}
```

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `dealer_ids` | Array\<object\> | Yes | mobile.de dealer identifiers whose car stock you want to browse. |
| `dealer_ids[].dealer_id` | string | Yes per entry | Dealer identifier returned by a listing. |
| `dealer_ids[].vehicle_type` | string | No | Vehicle type. v1 supports car. |
| `dealer_ids[].offset` | integer | No | Zero-based result offset. |
| `dealer_ids[].limit` | integer | No | Listings to return, from 1 to 20. |
| `dealer_ids[].query` | string | No | Optional free-text vehicle search. |
| `dealer_ids[].price_min` | integer | No | Optional non-negative minimum price. |
| `dealer_ids[].price_max` | integer | No | Optional non-negative maximum price. |
| `dealer_ids[].registration_year_min` | integer | No | Optional minimum first-registration year. |
| `dealer_ids[].registration_year_max` | integer | No | Optional maximum first-registration year. |
| `dealer_ids[].mileage_min` | integer | No | Optional non-negative minimum mileage. |
| `dealer_ids[].mileage_max` | integer | No | Optional non-negative maximum mileage. |
| `dealer_ids[].make_id` | string | No | Validated marketplace make identifier. |
| `dealer_ids[].model_id` | string | No | Validated marketplace model identifier. Requires make\_id. |
| `dealer_ids[].sort` | string | No | relevance, price\_asc, or price\_desc. |
| `vehicle_type` | string | No | Vehicle type. v1 supports car. |
| `offset` | integer | No | Zero-based result offset. |
| `limit` | integer | No | Listings to return, from 1 to 20. |
| `query` | string | No | Optional free-text vehicle search. |
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
  "isDamageCase": false,
  "vat": "Current listing detail",
  "isNew": true,
  "input_dealer_id": "47546627",
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

Yes. Send a POST request to `https://api.apify.com/v2/acts/thescrappa~mobile-de-dealer-inventory-scraper/runs` with your Actor input, or use an Apify client library. Read the output from the run dataset.

### Can I connect the results to other tools?

Yes. Apify integrations and APIs can pass dataset results to Make, Zapier, n8n, Google Sheets, and other data workflows.

### What happens when one input fails?

The Actor logs a short source error and continues with the remaining entries. Transient rate limits and gateway errors are retried; if every entry fails, the run reports an error.

### Where can I find a dealer ID?

Use the dealer identifier associated with a mobile.de listing or dealer profile.

## Related Scrappa Actors

- [mobile.de Car Search Scraper](https://apify.com/thescrappa/mobile-de-search-scraper)
- [mobile.de Listing Details Scraper](https://apify.com/thescrappa/mobile-de-listing-scraper)
- [Google Search Scraper](https://apify.com/thescrappa/google-search)

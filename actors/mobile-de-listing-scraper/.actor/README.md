# mobile.de Listing Details Scraper

Retrieve the full mobile.de record for a listing ID, including vehicle attributes, photos, seller details, finance options, and related links. Add multiple IDs to inspect cars together.

## What data can you extract?

| Field | Type | Description |
| --- | --- | --- |
| `success` | Boolean | True when the mobile.de request returned a successful response. |
| `data` | Object | mobile.de response payload for this vehicle listing, including `listing` and related source fields. |
| `meta` | Object | Response metadata from mobile.de, such as request duration and endpoint family. |
| `listing_title` | String | Vehicle title assembled from the model year, make, model, and trim. |
| `make` | String | Vehicle manufacturer name. |
| `model` | String | Vehicle model name. |
| `input_listing_id` | String | Listing id submitted to retrieve this vehicle listing. |
| `scraped_at` | String | UTC ISO 8601 timestamp when this dataset record was collected. |

## Use cases

- Car shoppers can compare specifications and photos for saved listings.
- Dealers can check seller and financing details for selected vehicles.
- Automotive data teams can enrich listing IDs with structured vehicle attributes.

## How to use

1. Add a mobile.de listing ID to `listing_ids` for each car.
2. Set `maxResults` to control how many listing records are saved.
3. Export the results and inspect the nested attributes, images, and finance data.

```json
{
  "listing_ids": [
    {
      "listing_id": "457981580"
    }
  ],
  "maxResults": 1
}
```

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `listing_ids` | Array\<object\> | Yes | Numeric mobile.de listing IDs to retrieve. |
| `listing_ids[].listing_id` | string | Yes per entry | Listing identifier returned by search. |
| `maxResults` | integer | No | Maximum dataset items to save across this entire run. |

## Output example

```json
{
  "success": true,
  "data": {
    "listing": {
      "attributes": [
        {
          "label": "Verified source detail",
          "tag": "Verified source detail",
          "value": "Balcony"
        }
      ],
      "adMobPlacements": {
        "contentUrl": "https://www.mobile.de/angebote/volvo-xc60-b4-awd",
        "slots": {
          "lightboxtextlink": {
            "adId": "457981580",
            "adSizes": [
              "Verified source detail"
            ]
          },
          "textlink": {
            "adId": "457981580",
            "adSizes": [
              "Verified source detail"
            ]
          },
          "vip_1": {
            "adId": "457981580",
            "adSizes": [
              "Verified source detail"
            ]
          },
          "vip_2": {
            "adId": "457981580",
            "adSizes": [
              "Verified source detail"
            ]
          },
          "vip_3": {
            "adId": "457981580",
            "adSizes": [
              "Verified source detail"
            ]
          },
          "vip_gal": {
            "adId": "457981580",
            "adSizes": [
              "Verified source detail"
            ]
          },
          "vip_gal2": {
            "adId": "457981580",
            "adSizes": [
              "Verified source detail"
            ]
          }
        }
      },
      "mediaGallery": {
        "slideShow": true,
        "additionalAds": true
      },
      "shortTitle": "2023 Volvo XC60 B4 AWD",
      "subTitle": "Verified source detail",
      "makeKey": "Verified source detail",
      "modelKey": "Verified source detail",
      "isNew": true,
      "onCustomerBehalf": true,
      "readyToDrive": true,
      "isConditionNew": true,
      "created": 1791036800,
      "modified": 1791123200,
      "renewed": 1791209600,
      "version": 1,
      "makeId": 8000635,
      "modelId": 8000751,
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
      "insurancePlans": [
        {
          "href": "https://www.mobile.de/angebote/volvo-xc60-b4-awd",
          "provider": "Verified source detail",
          "title": "2023 Volvo XC60 B4 AWD",
          "description": "Compact design with dependable performance and a two-year warranty.",
          "icon": "primary-image",
          "iconDark": "Verified source detail",
          "insuranceOffers": [
            {
              "id": "457981580",
              "name": "Northstar Outdoor Supply",
              "startPrice": 629,
              "href": "https://www.mobile.de/angebote/volvo-xc60-b4-awd"
            }
          ]
        }
      ],
      "links": [
        {
          "rel": "Current listing detail",
          "href": "https://www.mobile.de/angebote/volvo-xc60-b4-awd"
        }
      ],
      "images": [
        {
          "uri": "https://cdn.northstar.invalid/images/aurora-front.webp"
        }
      ],
      "features": [
        "Verified source detail"
      ],
      "sellerId": 8000877,
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
      "id": 8000237,
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
      "onLoadFlags": {
        "welcomeMessage": true,
        "dealershipDetails": true,
        "map": true
      },
      "htmlDescription": "Verified source detail",
      "carfaxEligible": true,
      "listing_id": "457981580"
    }
  },
  "meta": {
    "duration_ms": 8,
    "endpoint_family": "Verified source detail"
  },
  "listing_title": "2023 Volvo XC60 B4 AWD",
  "make": "Volvo",
  "model": "Volvo",
  "input_listing_id": "457981580",
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

Yes. Send a POST request to `https://api.apify.com/v2/acts/thescrappa~mobile-de-listing-scraper/runs` with your Actor input, or use an Apify client library. Read the output from the run dataset.

### Can I connect the results to other tools?

Yes. Apify integrations and APIs can pass dataset results to Make, Zapier, n8n, Google Sheets, and other data workflows.

### What happens when one input fails?

The Actor logs a short source error and continues with the remaining entries. Transient rate limits and gateway errors are retried; if every entry fails, the run reports an error.

### Where do I find a mobile.de listing ID?

Use the numeric ID in a mobile.de listing URL or take it from a mobile.de search result.

## Related Scrappa Actors

- [mobile.de Car Search Scraper](https://apify.com/thescrappa/mobile-de-search-scraper)
- [mobile.de Dealer Inventory Scraper](https://apify.com/thescrappa/mobile-de-dealer-inventory-scraper)
- [Google Search Scraper](https://apify.com/thescrappa/google-search)

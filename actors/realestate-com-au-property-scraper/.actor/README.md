# Realestate.com.au Property Details Scraper

Retrieve a realestate.com.au listing by its full URL. The response contains the asking price, address, property features, media, agency information, and lister details.

## What data can you extract?

| Field | Type | Description |
| --- | --- | --- |
| `success` | Boolean | True when the realestate.com.au request returned a successful response. |
| `data` | Object | realestate.com.au response payload for this property listing, including `url`, `listing_id`, `hidden_price`, `listing` and related source fields. |
| `listing_id` | String | Listing id identifier for this realestate.com.au property listing. |
| `price` | String | Price amount for this property listing; currency follows the selected source market. |
| `property_type` | String | Residential property category, such as house or apartment. |
| `suburb` | String | The source location name or code used to place this property listing on realestate.com.au. |
| `bedrooms` | Integer | Number of bedrooms reported for the property. |
| `bathrooms` | Integer | Number of bathrooms reported for the property. |
| `parking_spaces` | Integer | Number of vehicle spaces included with the property. |
| `agency_name` | String | Real estate agency marketing the property. |
| `listing_url` | String | Public realestate.com.au URL for the property listing or linked resource. |
| `input_url` | String | Url submitted to retrieve this property listing. |
| `scraped_at` | String | UTC ISO 8601 timestamp when this dataset record was collected. |

## Use cases

- Buyers can review property features, photos, and agency details for saved listings.
- Property analysts can enrich listing URLs with structured address and price information.
- Agents can compare current property pages and listing presentation.

## How to use

1. Add one full realestate.com.au property URL to `property_urls` per lookup.
2. Set `maxResults` to limit the number of listing records saved.
3. Export the full listing data and use the flattened price and feature fields for sorting.

```json
{
  "property_urls": [
    {
      "url": "https://www.realestate.com.au/property-house-vic-richmond-152489476"
    }
  ],
  "maxResults": 1
}
```

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `property_urls` | Array\<object\> | Yes | Full realestate.com.au listing URLs to retrieve. |
| `property_urls[].url` | string | Yes per entry | Full realestate.com.au property URL. |
| `maxResults` | integer | No | Maximum dataset items to save across this entire run. |

## Output example

```json
{
  "success": true,
  "data": {
    "url": "https://www.realestate.com.au/property-house-vic-richmond-152000001",
    "listing_id": "152000001",
    "hidden_price": null,
    "listing": {
      "id": "152000001",
      "name": null,
      "url": "https://www.realestate.com.au/property-house-vic-richmond-152000001",
      "property_type": "Residential property",
      "description": "Bright two-bedroom home with a balcony near local transport.",
      "address": {
        "suburb": "Richmond",
        "state": "VIC",
        "postcode": "3121",
        "display": {
          "shortAddress": "24 Bridge Road, Richmond VIC 3121",
          "__typename": "PropertyAddress",
          "fullAddress": "24 Bridge Road, Richmond VIC 3121",
          "geocode": {
            "latitude": -37.82,
            "longitude": 144.999,
            "__typename": "PropertyAddress"
          }
        },
        "__typename": "PropertyAddress"
      },
      "price": "A$720,000",
      "features": {
        "bedrooms": {
          "value": 8,
          "__typename": "PropertyFeatures"
        },
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
        },
        "__typename": "PropertyFeatures"
      },
      "property_sizes": {
        "building": null,
        "land": {
          "displayValue": "Richmond, VIC 3121",
          "sizeUnit": {
            "displayValue": "Richmond, VIC 3121",
            "__typename": "PropertyListing"
          },
          "__typename": "PropertyListing"
        },
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
        },
        "__typename": "PropertyListing"
      },
      "media": {
        "statementOfInformation": {
          "href": "https://www.realestate.com.au/property-house-vic-richmond-152000001",
          "__typename": "PropertyListing"
        },
        "__typename": "PropertyListing",
        "mainImage": {
          "templatedUrl": "https://www.realestate.com.au/property-house-vic-richmond-152000001",
          "__typename": "PropertyListing"
        },
        "images": [
          {
            "templatedUrl": "https://www.realestate.com.au/property-house-vic-richmond-152000001",
            "__typename": "PropertyListing"
          }
        ],
        "floorplans": [
          {
            "templatedUrl": "https://www.realestate.com.au/property-house-vic-richmond-152000001",
            "__typename": "PropertyListing"
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
        ]
      },
      "listing_company": {
        "id": "152000001",
        "name": "Harbourline Property Group",
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
        "__typename": "PropertyListing",
        "_links": {
          "canonical": {
            "href": "https://www.realestate.com.au/property-house-vic-richmond-152000001",
            "__typename": "PropertyListing",
            "templated": true
          },
          "__typename": "PropertyListing"
        },
        "address": {
          "display": {
            "fullAddress": "24 Bridge Road, Richmond VIC 3121",
            "__typename": "PropertyAddress"
          },
          "__typename": "PropertyAddress"
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
            "__typename": "PropertyListing"
          },
          "_links": {
            "canonical": {
              "href": "https://www.realestate.com.au/property-house-vic-richmond-152000001",
              "__typename": "PropertyListing"
            },
            "__typename": "PropertyListing"
          },
          "showInMediaViewer": true
        }
      ],
      "auction": {
        "dateTime": {
          "value": "Balcony",
          "__typename": "PropertyListing",
          "display": {
            "longLabel": "Verified source detail",
            "__typename": "PropertyListing"
          }
        },
        "__typename": "PropertyListing",
        "onlineLinks": [
          "Verified marketplace detail"
        ]
      },
      "inspections": [
        {
          "display": {
            "longLabel": "Verified source detail",
            "__typename": "PropertyListing"
          },
          "__typename": "PropertyListing",
          "startTime": "Verified source detail",
          "endTime": "Verified source detail"
        }
      ],
      "sold_date": null
    },
    "raw": {
      "id": "152000001",
      "_links": {
        "canonical": {
          "path": "https://www.realestate.com.au/property-house-vic-richmond-152000001",
          "__typename": "SourceRecord",
          "href": "https://www.realestate.com.au/property-house-vic-richmond-152000001"
        },
        "__typename": "SourceRecord",
        "submitEnquiry": {
          "href": "https://www.realestate.com.au/property-house-vic-richmond-152000001",
          "__typename": "SourceRecord"
        }
      },
      "__typename": "SourceRecord",
      "parent": null,
      "aboveTheFoldId": "152000001",
      "badge": {
        "colour": "navy",
        "label": "Verified source detail",
        "__typename": "SourceRecord"
      },
      "address": {
        "suburb": "Richmond",
        "state": "VIC",
        "postcode": "3121",
        "display": {
          "shortAddress": "24 Bridge Road, Richmond VIC 3121",
          "__typename": "PropertyAddress",
          "fullAddress": "24 Bridge Road, Richmond VIC 3121",
          "geocode": {
            "latitude": -37.82,
            "longitude": 144.999,
            "__typename": "PropertyAddress"
          }
        },
        "__typename": "PropertyAddress"
      },
      "propertyType": {
        "id": "152000001",
        "display": "Richmond, VIC 3121",
        "__typename": "PropertyListing"
      },
      "viewConfiguration": {
        "details": {
          "showBreadcrumbs": true,
          "__typename": "SourceRecord",
          "agencyBrandingOnSidePanel": true,
          "branding": {
            "header": {
              "size": "Verified source detail",
              "__typename": "SourceRecord"
            },
            "__typename": "SourceRecord"
          },
          "posterBoard": true,
          "showWalkthroughVideo": true,
          "showWalkthroughVideoV2": true
        },
        "__typename": "SourceRecord",
        "searchResults": {
          "showDisplayPrice": true,
          "__typename": "SourceRecord",
          "showAgencyInMediaViewer": true
        }
      },
      "listingCompany": {
        "id": "152000001",
        "name": "Harbourline Property Group",
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
        "__typename": "PropertyListing",
        "_links": {
          "canonical": {
            "href": "https://www.realestate.com.au/property-house-vic-richmond-152000001",
            "__typename": "PropertyListing",
            "templated": true
          },
          "__typename": "PropertyListing"
        },
        "address": {
          "display": {
            "fullAddress": "24 Bridge Road, Richmond VIC 3121",
            "__typename": "PropertyAddress"
          },
          "__typename": "PropertyAddress"
        }
      },
      "generalFeatures": {
        "bedrooms": {
          "value": 8,
          "__typename": "PropertyFeatures"
        },
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
        },
        "__typename": "PropertyFeatures"
      },
      "propertySizes": {
        "building": null,
        "land": {
          "displayValue": "Richmond, VIC 3121",
          "sizeUnit": {
            "displayValue": "Richmond, VIC 3121",
            "__typename": "PropertyListing"
          },
          "__typename": "PropertyListing"
        },
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
        },
        "__typename": "PropertyListing"
      },
      "price": {
        "display": "Richmond, VIC 3121",
        "__typename": "SourceRecord",
        "searchRange": null,
        "information": null,
        "disclaimer": null
      },
      "media": {
        "statementOfInformation": {
          "href": "https://www.realestate.com.au/property-house-vic-richmond-152000001",
          "__typename": "SourceRecord"
        },
        "__typename": "SourceRecord",
        "mainImage": {
          "templatedUrl": "https://www.realestate.com.au/property-house-vic-richmond-152000001",
          "__typename": "SourceRecord"
        },
        "images": [
          {
            "templatedUrl": "https://www.realestate.com.au/property-house-vic-richmond-152000001",
            "__typename": "SourceRecord"
          }
        ],
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
        ]
      },
      "inspections": [
        {
          "display": {
            "longLabel": "Verified source detail",
            "__typename": "SourceRecord"
          },
          "__typename": "SourceRecord",
          "startTime": "Verified source detail",
          "endTime": "Verified source detail"
        }
      ],
      "description": "Bright two-bedroom home with a balcony near local transport.",
      "auction": {
        "dateTime": {
          "value": "Balcony",
          "__typename": "SourceRecord",
          "display": {
            "longLabel": "Verified source detail",
            "__typename": "SourceRecord"
          }
        },
        "__typename": "SourceRecord",
        "onlineLinks": [
          "Verified marketplace detail"
        ]
      },
      "productDepth": "Verified source detail",
      "introduction": null,
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
          "showInMediaViewer": true
        }
      ],
      "faq": null,
      "inspectionTimes": {
        "registrationsEnabled": true,
        "__typename": "SourceRecord"
      }
    }
  },
  "listing_id": "152000001",
  "price": "A$720,000",
  "property_type": "Residential property",
  "suburb": "Richmond",
  "bedrooms": 8,
  "bathrooms": 8,
  "parking_spaces": 8,
  "agency_name": "Harbourline Property Group",
  "listing_url": "https://www.realestate.com.au/property-house-vic-richmond-152000001",
  "input_url": "https://www.realestate.com.au/property-house-vic-richmond-152489476",
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

Yes. Send a POST request to `https://api.apify.com/v2/acts/thescrappa~realestate-com-au-property-scraper/runs` with your Actor input, or use an Apify client library. Read the output from the run dataset.

### Can I connect the results to other tools?

Yes. Apify integrations and APIs can pass dataset results to Make, Zapier, n8n, Google Sheets, and other data workflows.

### What happens when one input fails?

The Actor logs a short source error and continues with the remaining entries. Transient rate limits and gateway errors are retried; if every entry fails, the run reports an error.

### What URL format is accepted?

Use a realestate.com.au property page URL, including the property type, location, and listing number.

## Related Scrappa Actors

- [Realestate.com.au Property Search Scraper](https://apify.com/thescrappa/realestate-com-au-search-scraper)
- [Realestate.com.au Agents Scraper](https://apify.com/thescrappa/realestate-com-au-agents-scraper)
- [Redfin Property Details Scraper](https://apify.com/thescrappa/redfin-property-details-scraper)

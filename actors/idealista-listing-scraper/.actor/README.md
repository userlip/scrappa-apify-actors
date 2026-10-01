# Idealista Listing Details Scraper

Retrieve detailed Idealista records by listing ID, including price, property type, room counts, location, media, agency information, and page links. Choose the country for each market.

## What data can you extract?

| Field | Type | Description |
| --- | --- | --- |
| `success` | Boolean | True when the Idealista request returned a successful response. |
| `data` | Object | Idealista response payload for this property listing, including `adid`, `price`, `priceInfo`, `operation` and related source fields. |
| `meta` | Object | Response metadata from Idealista, such as request duration and endpoint family. |
| `listing_id` | Integer | Numeric identifier for this Idealista property listing. |
| `price` | Integer | Price amount for this property listing; currency follows the selected source market. |
| `operation` | String | Listing transaction type, such as sale or rent. |
| `property_type` | String | Idealista property category, such as apartment or house. |
| `rooms` | Integer | Number of rooms reported for the property. |
| `bathrooms` | Integer | Number of bathrooms reported for the property. |
| `location_name` | String | Named town, district, or neighborhood attached to the listing. |
| `listing_url` | String | Public Idealista URL for the property listing or linked resource. |
| `input_ad_id` | String | Ad id submitted to retrieve this property listing. |
| `scraped_at` | String | UTC ISO 8601 timestamp when this dataset record was collected. |

## Use cases

- Property researchers can enrich saved listing IDs with detailed attributes and photos.
- Agents can compare advertised price, rooms, and location for a property shortlist.
- Housing analysts can build consistent records from multiple Idealista markets.

## How to use

1. Add an Idealista listing ID to `listing_ids` for each property.
2. Choose the market and any supported language or quality options.
3. Set `maxResults` and export one detailed record per listing ID.

```json
{
  "listing_ids": [
    {
      "ad_id": "104570830"
    }
  ],
  "country": "es",
  "maxResults": 1
}
```

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `listing_ids` | Array\<object\> | Yes | Idealista listing IDs to retrieve across supported markets. |
| `listing_ids[].ad_id` | string | Yes per entry | Listing identifier from search. |
| `listing_ids[].country` | string | No | es, it, pt, or fr. |
| `listing_ids[].typology` | string | No |  |
| `listing_ids[].from_date` | string | No |  |
| `listing_ids[].to_date` | string | No |  |
| `listing_ids[].guests` | integer | No |  |
| `listing_ids[].quality` | string | No |  |
| `listing_ids[].locale` | string | No |  |
| `listing_ids[].language` | string | No |  |
| `listing_ids[].max_items` | integer | No |  |
| `country` | string | Yes | es, it, pt, or fr. |
| `typology` | string | No |  |
| `from_date` | string | No |  |
| `to_date` | string | No |  |
| `guests` | integer | No |  |
| `quality` | string | No |  |
| `locale` | string | No |  |
| `language` | string | No |  |
| `max_items` | integer | No |  |
| `maxResults` | integer | No | Maximum dataset items to save across this entire run. |

## Output example

```json
{
  "success": true,
  "data": {
    "adid": 8000403,
    "price": 549,
    "priceInfo": {
      "amount": 549,
      "currencySuffix": "EUR"
    },
    "operation": "sale",
    "propertyType": "Apartment",
    "extendedPropertyType": "Residential property",
    "homeType": "Residential property",
    "state": "berlin",
    "multimedia": {
      "images": [
        {
          "url": "https://www.idealista.com/en/inmueble/99999117/",
          "tag": "Verified source detail",
          "localizedName": "Northstar Outdoor Supply",
          "multimediaId": 8000384,
          "deeplinkUrl": "https://www.idealista.com/en/inmueble/99999117/",
          "madeByIdealista": true,
          "width": 1200
        }
      ],
      "videos": [
        {
          "url": "https://www.idealista.com/en/inmueble/99999117/",
          "isProfessionalVideo": true,
          "thumbnail": "https://cdn.northstar.invalid/images/aurora-front.webp",
          "multimediaId": 8000385,
          "deeplinkUrl": "https://www.idealista.com/en/inmueble/99999117/",
          "madeByIdealista": true
        }
      ],
      "virtual3DTours": [
        {
          "url": "https://www.idealista.com/en/inmueble/99999117/",
          "tourType": "Residential property",
          "creationDate": 8,
          "thumbnail": "https://cdn.northstar.invalid/images/aurora-front.webp",
          "category": "Residential property",
          "deeplinkUrl": "https://www.idealista.com/en/inmueble/99999117/",
          "madeByIdealista": true
        }
      ],
      "hasMultimediasMadeByIdealista": true
    },
    "propertyComment": "Bright two-bedroom home with a balcony near local transport.",
    "ubication": {
      "title": "Bright two-bedroom flat in Berlin",
      "latitude": 40.4168,
      "longitude": -3.7038,
      "hasHiddenAddress": false,
      "administrativeAreaLevel4": "Verified source detail",
      "administrativeAreaLevel3": "Verified source detail",
      "administrativeAreaLevel2": "Verified source detail",
      "administrativeAreaLevel1": "Verified source detail",
      "locationId": "99999117",
      "administrativeAreaLevel1Id": "99999117",
      "locationName": "Madrid"
    },
    "country": "Spain",
    "contactInfo": {
      "commercialName": "Morgenrot Immobilien GmbH",
      "phone1": {
        "phoneNumber": "+49 30 555 0184",
        "formattedPhone": "+49 30 555 0184",
        "prefix": "Verified source detail",
        "phoneNumberForMobileDialing": "+49 30 555 0184",
        "nationalNumber": true,
        "formattedPhoneWithPrefix": "+49 30 555 0184"
      },
      "contactName": "Morgan Lee",
      "externalReference": "WEB-583214",
      "userType": "Residential property",
      "agencyLogo": "https://cdn.northstar.invalid/images/aurora-front.webp",
      "contactMethod": "Verified source detail",
      "micrositeShortName": "Northstar Outdoor Supply",
      "address": {
        "streetName": "Northstar Outdoor Supply",
        "streetNumber": 8,
        "locationName": "Madrid",
        "postalCode": "Verified source detail"
      },
      "agentInfo": {
        "name": "Jordan Ellis",
        "proAgent": true
      },
      "inVirtualMicrosite": true,
      "sharedSeekerProfile": true,
      "corporateVideo": {
        "thumbnail": "https://cdn.northstar.invalid/images/aurora-front.webp",
        "url": "https://www.idealista.com/en/inmueble/99999117/"
      },
      "corporatePhrase": {
        "text": "High quality and prompt delivery.",
        "autoTranslated": true
      },
      "totalAds": 28,
      "needLoginForContact": true,
      "needLoginForPhone": true,
      "chatEnabled": true,
      "professional": true
    },
    "priceDropInfo": {
      "priceDropValue": 629,
      "priceDropPercentage": 629,
      "dropDate": 8
    },
    "moreCharacteristics": {
      "communityCosts": 8,
      "roomNumber": 2,
      "isStudio": true,
      "bathNumber": 8,
      "isAuction": false,
      "exterior": true,
      "housingFurnitures": "Verified source detail",
      "agencyIsABank": true,
      "energyPerformance": 8,
      "isPenthouse": true,
      "energyCertificationType": "Residential property",
      "swimmingPool": true,
      "flatLocation": "Verified source detail",
      "modificationDate": 8,
      "isInTopFloor": true,
      "constructedArea": 78,
      "lift": true,
      "garden": true,
      "boxroom": true,
      "isDuplex": true,
      "floor": "Verified source detail",
      "status": "Published"
    },
    "translatedTexts": {
      "floorNumberDescription": "Verified source detail",
      "layoutDescription": "Verified source detail",
      "characteristicsDescriptions": [
        {
          "key": "primary-image",
          "title": "Bright two-bedroom flat in Berlin",
          "detailFeatures": [
            {
              "phrase": "Verified source detail"
            }
          ]
        }
      ]
    },
    "suggestedTexts": {
      "title": "Bright two-bedroom flat in Berlin"
    },
    "detailedType": {
      "typology": "Verified source detail",
      "subTypology": "Verified source detail"
    },
    "link": {
      "url": "https://www.idealista.com/en/inmueble/99999117/"
    },
    "comments": [
      {
        "propertyComment": "Bright two-bedroom home with a balcony near local transport.",
        "autoTranslated": true,
        "language": "de",
        "defaultLanguage": true
      }
    ],
    "detailWebLink": "Verified source detail",
    "energyCertification": {
      "title": "Bright two-bedroom flat in Berlin",
      "energyConsumption": {
        "prefix": "Verified source detail",
        "suffix": "Verified source detail",
        "value": 8,
        "type": "Apartment"
      },
      "emissions": {
        "prefix": "Verified source detail",
        "suffix": "Verified source detail",
        "value": 8,
        "type": "Apartment"
      }
    },
    "allowsCounterOffers": true,
    "allowsRemoteVisit": true,
    "allowsMortgageSimulator": true,
    "allowsAiAssistant": true,
    "allowsProfileQualification": true,
    "tracking": {
      "isSuitableForRecommended": true,
      "commercialDataId": 8000796
    },
    "has360VHS": true,
    "labels": [
      "Verified marketplace detail"
    ],
    "showSuggestedPrice": true,
    "allowsRecommendation": true,
    "modificationDate": {
      "value": 8,
      "text": "High quality and prompt delivery."
    }
  },
  "meta": {
    "duration_ms": 8,
    "endpoint_family": "Verified source detail"
  },
  "listing_id": 8000403,
  "price": 549,
  "operation": "sale",
  "property_type": "Apartment",
  "rooms": 2,
  "bathrooms": 8,
  "location_name": "Madrid",
  "listing_url": "https://www.idealista.com/en/inmueble/99999117/",
  "input_ad_id": "104570830",
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

Yes. Send a POST request to `https://api.apify.com/v2/acts/thescrappa~idealista-listing-scraper/runs` with your Actor input, or use an Apify client library. Read the output from the run dataset.

### Can I connect the results to other tools?

Yes. Apify integrations and APIs can pass dataset results to Make, Zapier, n8n, Google Sheets, and other data workflows.

### What happens when one input fails?

The Actor logs a short source error and continues with the remaining entries. Transient rate limits and gateway errors are retried; if every entry fails, the run reports an error.

### Where can I find a listing ID?

Use the numeric identifier in an Idealista property URL or copy it from an Idealista search result.

## Related Scrappa Actors

- [Idealista Property Search Scraper](https://apify.com/thescrappa/idealista-search-scraper)
- [Idealista Agency Scraper](https://apify.com/thescrappa/idealista-agency-scraper)
- [Immowelt Property Search Scraper](https://apify.com/thescrappa/immowelt-property-search-scraper)

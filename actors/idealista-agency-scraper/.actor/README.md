# Idealista Agency Scraper

Look up Idealista agencies by microsite short name and collect company details, contact information, service defaults, listing totals, languages, and media. Choose the market for each profile.

## What data can you extract?

| Field | Type | Description |
| --- | --- | --- |
| `success` | Boolean | True when the Idealista request returned a successful response. |
| `data` | Object | Idealista response payload for this agency profile, including `shortName`, `activeSinceYear`, `commercialName`, `agencyWebUrl` and related source fields. |
| `meta` | Object | Response metadata from Idealista, such as request duration and endpoint family. |
| `agency_name` | String | Public name of the Idealista real estate agency. |
| `agency_website` | String | Agency website URL listed in the Idealista profile. |
| `active_since_year` | String | Year the agency began operating on Idealista. |
| `listing_count` | Integer | Number of listing records reported for this agency profile. |
| `agency_short_name` | String | Short agency identifier used in the Idealista profile URL. |
| `input_microsite_short_name` | String | Microsite short name submitted to retrieve this agency profile. |
| `scraped_at` | String | UTC ISO 8601 timestamp when this dataset record was collected. |

## Use cases

- Real estate teams can compare agency coverage, listing totals, and service focus.
- Market researchers can map agency profiles and languages across supported countries.
- Property investors can identify agencies linked to saved listing records.

## How to use

1. Add an Idealista microsite short name to `agencies` for each lookup.
2. Choose Spain, Italy, Portugal, or France as the market.
3. Set `maxResults` and export one agency profile per short name.

```json
{
  "agencies": [
    {
      "microsite_short_name": "aproperties-mad"
    }
  ],
  "country": "es",
  "maxResults": 1
}
```

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `agencies` | Array\<object\> | Yes | Idealista agency short names from agency microsite URLs. |
| `agencies[].microsite_short_name` | string | Yes per entry | Agency short name, for example clikalia. |
| `agencies[].country` | string | No | es, it, pt, or fr. |
| `country` | string | Yes | es, it, pt, or fr. |
| `maxResults` | integer | No | Maximum dataset items to save across this entire run. |

## Output example

```json
{
  "success": true,
  "data": {
    "shortName": "Northstar Outdoor Supply",
    "activeSinceYear": "Verified source detail",
    "commercialName": "Morgenrot Immobilien GmbH",
    "agencyWebUrl": "https://www.idealista.com/en/inmueble/99999117/",
    "slogan": "Clear advice for every move.",
    "defaultTypology": "Verified source detail",
    "defaultOperation": "sale",
    "contactInfo": {
      "phone": "+49 30 555 0184",
      "contactPhone": {
        "phoneNumber": "+49 30 555 0184",
        "formattedPhone": "+49 30 555 0184",
        "prefix": "Verified source detail",
        "phoneNumberForMobileDialing": "+49 30 555 0184",
        "nationalNumber": true,
        "formattedPhoneWithPrefix": "+49 30 555 0184"
      },
      "showContactButton": true,
      "address": {
        "streetName": "Northstar Outdoor Supply",
        "streetNumber": 8,
        "locationName": "Madrid",
        "postalCode": "Verified source detail",
        "latitude": 40.4168,
        "longitude": -3.7038
      },
      "proAgent": true
    },
    "multimedias": {
      "mainImage": "https://cdn.northstar.invalid/images/aurora-front.webp",
      "brandingLogo": "https://cdn.northstar.invalid/images/aurora-front.webp"
    },
    "total": 28,
    "languages": [
      "de"
    ],
    "commercialDataId": 8000779,
    "hasChatEnabled": true,
    "micrositeLeadAllowed": true,
    "corporateVideo": {
      "url": "https://www.idealista.com/en/inmueble/99999117/",
      "thumbnail": "https://cdn.northstar.invalid/images/aurora-front.webp"
    },
    "corporatePhrase": {
      "text": "High quality and prompt delivery.",
      "autoTranslated": true
    }
  },
  "meta": {
    "duration_ms": 8,
    "endpoint_family": "Verified source detail"
  },
  "agency_name": "Morgenrot Immobilien GmbH",
  "agency_website": "https://www.idealista.com/en/inmueble/99999117/",
  "active_since_year": "Verified source detail",
  "listing_count": 28,
  "agency_short_name": "Northstar Outdoor Supply",
  "input_microsite_short_name": "aproperties-mad",
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

Yes. Send a POST request to `https://api.apify.com/v2/acts/thescrappa~idealista-agency-scraper/runs` with your Actor input, or use an Apify client library. Read the output from the run dataset.

### Can I connect the results to other tools?

Yes. Apify integrations and APIs can pass dataset results to Make, Zapier, n8n, Google Sheets, and other data workflows.

### What happens when one input fails?

The Actor logs a short source error and continues with the remaining entries. Transient rate limits and gateway errors are retried; if every entry fails, the run reports an error.

### What is an agency short name?

It is the microsite identifier used in an Idealista agency profile URL. Use the short name, not the full URL.

## Related Scrappa Actors

- [Idealista Listing Details Scraper](https://apify.com/thescrappa/idealista-listing-scraper)
- [Idealista Property Search Scraper](https://apify.com/thescrappa/idealista-search-scraper)
- [LinkedIn Company Scraper - $0.30/1k results](https://apify.com/thescrappa/linkedin-company-scraper)

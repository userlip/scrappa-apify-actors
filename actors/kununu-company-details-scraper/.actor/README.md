# Kununu Company Profile Scraper

Retrieve Kununu employer profiles with company identity, location, industry, ratings, recommendation rates, and review totals. Batch company slugs with a selected country.

## What data can you extract?

| Field | Type | Description |
| --- | --- | --- |
| `success` | Boolean | True when Kununu returned a parsed company profile. |
| `data` | Object | Company identity, ratings, organization metadata, and profile information. |
| `meta` | Object | Profile URL, fetch timing, and cache status. |
| `input_company_slug` | String | Kununu employer profile slug used for this lookup. |
| `scraped_at` | String | UTC date and time when this record was collected. |

## Use cases

- Recruiters can review employer ratings and profile information.
- Workplace researchers can compare recommendation and review totals.
- B2B analysts can enrich employer records with location and industry context.

## How to use

1. Add a Kununu company slug to each entry in `companies`.
2. Choose the profile country, such as Germany, Austria, or Switzerland.
3. Read employer details and ratings from the returned profile object.

```json
{
  "companies": [
    {
      "company_slug": "sap"
    }
  ],
  "country": "de",
  "maxResults": 10
}
```

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `companies` | Array\<object\> | Yes | Kununu company profile slugs to retrieve. |
| `companies[].company_slug` | string | Yes per entry | Company slug from the public Kununu employer profile URL. |
| `companies[].country` | string | No | Kununu country market code, such as de, at, or ch. Accepted values: de, at, ch. |
| `companies[].slug` | string | No | Alias for company\_slug, for chaining from the search endpoint. |
| `companies[].countryCode` | string | No | Alias for country \(de, at, ch\). Accepted values: de, at, ch. |
| `country` | string | Yes | Kununu country market code, such as de, at, or ch. Accepted values: de, at, ch. |
| `slug` | string | No | Alias for company\_slug, for chaining from the search endpoint. |
| `countryCode` | string | No | Alias for country \(de, at, ch\). Accepted values: de, at, ch. |
| `maxResults` | integer | No | Maximum dataset items to save across this entire run. |

## Output example

```json
{
  "success": true,
  "data": {
    "basic_info": {
      "name": "SAP SE",
      "uuid": "0b4f44e3-65a8-4f3a-b435-7c66e5d555c4",
      "slug": "sap-se",
      "city": "Walldorf",
      "country": "de",
      "industry": 12,
      "employer_segment": "Large enterprise",
      "url": "https://www.kununu.com/de/sap-se",
      "website": "https://www.sap.com"
    },
    "ratings": {
      "overall_score": 4.1,
      "rounded_score": 4,
      "recommendation_rate": 82,
      "total_reviews": 23840,
      "employee_reviews": 20600,
      "applicant_reviews": 3240,
      "reviews_with_text": 15200,
      "reviews_with_response": 4170,
      "response_time_days": 18,
      "industry_average_score": 3.8
    },
    "structured_data": {
      "organization": {
        "@type": "Organization",
        "name": "SAP SE",
        "url": "https://www.sap.com",
        "sameAs": [
          "https://www.kununu.com/de/sap-se"
        ]
      }
    },
    "metadata": {
      "title": "SAP SE Reviews and Ratings",
      "canonical_url": "https://www.kununu.com/de/sap-se"
    }
  },
  "meta": {
    "url": "https://www.kununu.com/de/sap-se",
    "duration_ms": 1020,
    "scraped_at": "2026-09-28T14:20:00Z",
    "cached": true
  },
  "input_company_slug": "sap",
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

Yes. Send a POST request to `https://api.apify.com/v2/acts/thescrappa~kununu-company-details-scraper/runs` with your Actor input, or use an Apify client library. Read the output from the run dataset.

### Can I connect the results to other tools?

Yes. Apify integrations and APIs can pass dataset results to Make, Zapier, n8n, Google Sheets, and other data workflows.

### What happens when one input fails?

The Actor logs a short source error and continues with the remaining entries. Transient rate limits and gateway errors are retried; if every entry fails, the run reports an error.

### How do I find a company slug?

Search with Kununu Company Search Scraper and copy the slug from its profile URL or result.

## Related Scrappa Actors

- [Kununu Company Search Scraper](https://apify.com/thescrappa/kununu-company-search-scraper)
- [Kununu Company Salaries Scraper](https://apify.com/thescrappa/kununu-company-salaries-scraper)
- [Kununu Top Companies Scraper](https://apify.com/thescrappa/kununu-top-companies-scraper)

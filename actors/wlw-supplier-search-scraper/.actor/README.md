# WLW Supplier Search Scraper

Search WLW \(Wer liefert was\) for suppliers and products by keyword. Collect company names, descriptions, categories, selling points, available pricing hints, and order quantities.

## What data can you extract?

| Field | Type | Description |
| --- | --- | --- |
| `id` | String | WLW identifier for the supplier result. |
| `auction_id` | Integer | WLW internal identifier for the supplier listing. |
| `name` | String | Supplier or product name shown in WLW search results. |
| `description` | String | Supplier description or product summary shown in WLW. |
| `slug` | String | URL slug for the supplier or product record. |
| `slug_id` | String | WLW slug identifier for the result. |
| `category` | String | WLW category assigned to the supplier result. |
| `language` | String | Language code used for the supplier result. |
| `is_showcased` | Boolean | True when WLW highlights the supplier result as a showcase. |
| `sell_points` | Array\<String\> | Supplier selling points displayed in the search result. |
| `price` | Object or null | Price range or pricing hint, with currency when available. |
| `minimum_order_quantity` | Object or null | Minimum order quantity and unit, when supplied. |
| `images` | Array\<Object\> | Supplier or product image records with image URLs. |
| `company` | Object | Company identity and public supplier details. |
| `input_q` | String | Q submitted to retrieve this supplier result. |
| `scraped_at` | String | UTC ISO 8601 timestamp when this dataset record was collected. |

## Use cases

- Procurement teams can build supplier lists for a product category.
- B2B researchers can compare company profiles, countries, and supplier attributes.
- Buyers can shortlist manufacturers and distributors from WLW results.

## How to use

1. Add one supplier or product keyword to `queries` per search.
2. Choose a country, category, supplier type, or verification filter.
3. Set `maxPages` and `maxResults`, then export supplier records.

```json
{
  "queries": [
    {
      "q": "schrauben"
    }
  ],
  "page": 1,
  "per_page": 5,
  "country": "DE",
  "language": "de",
  "maxResults": 5,
  "maxPages": 1
}
```

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `queries` | Array\<object\> | Yes | Product or service keywords to search on WLW \(Wer liefert was\). |
| `queries[].q` | string | Yes per entry | Search query \(e.g., "agentur", "software", "logistik"\) |
| `queries[].page` | integer | No | Page number \(default: 1\) |
| `queries[].per_page` | integer | No | Results per page \(1-50, default: 15\) |
| `queries[].country` | string | No | Country code: DE \(Germany\), AT \(Austria\), CH \(Switzerland\). Default: DE |
| `queries[].countries` | string | No | Filter by country codes \(comma-separated, e.g., "DE,AT"\) |
| `queries[].category` | string | No | Filter by category ID |
| `queries[].supplier_type` | string | No | Filter by supplier type \(comma-separated\) |
| `queries[].attributes` | string | No | Filter by attribute codes \(comma-separated\) |
| `queries[].verified` | boolean | No | Only show verified suppliers |
| `queries[].top_responder` | boolean | No | Only show top responders |
| `queries[].latitude` | number | No | Location filter: latitude |
| `queries[].longitude` | number | No | Location filter: longitude |
| `queries[].radius` | integer | No | Search radius in km \(requires lat/lng\) |
| `queries[].sort` | string | No | Sort: relevance, recency, trending, distance |
| `queries[].language` | string | No | Language code \(default: de\) |
| `page` | integer | No | Page number \(default: 1\) |
| `per_page` | integer | No | Results per page \(1-50, default: 15\) |
| `country` | string | No | Country code: DE \(Germany\), AT \(Austria\), CH \(Switzerland\). Default: DE |
| `countries` | string | No | Filter by country codes \(comma-separated, e.g., "DE,AT"\) |
| `category` | string | No | Filter by category ID |
| `supplier_type` | string | No | Filter by supplier type \(comma-separated\) |
| `attributes` | string | No | Filter by attribute codes \(comma-separated\) |
| `verified` | boolean | No | Only show verified suppliers |
| `top_responder` | boolean | No | Only show top responders |
| `latitude` | number | No | Location filter: latitude |
| `longitude` | number | No | Location filter: longitude |
| `radius` | integer | No | Search radius in km \(requires lat/lng\) |
| `sort` | string | No | Sort: relevance, recency, trending, distance |
| `language` | string | No | Language code \(default: de\) |
| `maxResults` | integer | No | Maximum dataset items to save across this entire run. |
| `maxPages` | integer | No | Maximum pages to request for each batch entry. |

## Output example

```json
{
  "id": "WLW-73915",
  "auction_id": 8000156,
  "name": "Nordstern Industrial Supply GmbH",
  "description": "Precision fasteners and assembly components for industrial buyers.",
  "slug": "northstar-industrial-supply",
  "slug_id": "WLW-73915",
  "category": "Industrial supplies",
  "language": "de",
  "is_showcased": true,
  "sell_points": [
    "Small batch orders accepted"
  ],
  "price": {
    "min": 549,
    "currency": "EUR",
    "kind": "Retail offer"
  },
  "minimum_order_quantity": {
    "value": "Balcony",
    "unit": "pieces"
  },
  "images": [
    {
      "url": "https://www.wlw.de/de/firma/nordstern-industries"
    }
  ],
  "company": {
    "id": "WLW-73915",
    "uuid": "WLW-73915",
    "name": "Morgenrot Immobilien GmbH",
    "slug": "Morgenrot Immobilien GmbH",
    "logo": "https://cdn.northstar.invalid/images/aurora-front.webp",
    "country_code": "DE",
    "founding_year": "Morgenrot Immobilien GmbH",
    "distribution_area": "Morgenrot Immobilien GmbH",
    "is_customer": true,
    "certificates_count": 28,
    "has_email": true,
    "response_rate": "Morgenrot Immobilien GmbH",
    "average_response_time": "Morgenrot Immobilien GmbH"
  },
  "input_q": "schrauben",
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

Yes. Send a POST request to `https://api.apify.com/v2/acts/thescrappa~wlw-supplier-search-scraper/runs` with your Actor input, or use an Apify client library. Read the output from the run dataset.

### Can I connect the results to other tools?

Yes. Apify integrations and APIs can pass dataset results to Make, Zapier, n8n, Google Sheets, and other data workflows.

### What happens when one input fails?

The Actor logs a short source error and continues with the remaining entries. Transient rate limits and gateway errors are retried; if every entry fails, the run reports an error.

### Can I filter suppliers by country?

Yes. Choose one country code or provide a comma-separated list of country codes in the country filter.

## Related Scrappa Actors

- [LinkedIn Company Scraper - $0.30/1k results](https://apify.com/thescrappa/linkedin-company-scraper)
- [Trustpilot Company Details Scraper](https://apify.com/thescrappa/trustpilot-company-details-scraper)
- [Google Maps Reviews Scraper](https://apify.com/thescrappa/google-maps-reviews-scraper)

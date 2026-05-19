# Trustpilot Business Search Scraper

Find companies and category business listings on Trustpilot through Scrappa. Use this Actor to build prospect lists, monitor competitors, discover reviewed businesses in a category, or collect company domains before running the Trustpilot Company Reviews Scraper.

## Features

- Search Trustpilot companies by brand, company name, or domain
- Browse Trustpilot businesses by category slug, country, claimed status, sort order, and minimum TrustScore
- Fetch one or more one-based result pages per run
- Extract company name, domain, website, Trustpilot profile URL, TrustScore, stars, review count, claimed/verified status, categories, country, city, email, and phone fields when available
- Dataset rows optimized for Apify table views
- Full Scrappa page responses saved to the `OUTPUT` key-value-store record

## Input

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `search_type` | string | No | `company_search` for keyword/domain discovery or `category` for category browsing. Default `company_search` |
| `query` | string | For company search | Company, brand, or domain query, for example `amazon` |
| `category` | string | For category search | Trustpilot category slug, for example `electronics_technology` or `restaurants_bars` |
| `country` | string | No | ISO-2 country filter, for example `US`, `GB`, or `DE` |
| `page` | integer | No | First one-based result page. Default `1` |
| `max_pages` | integer | No | Number of pages to fetch, 1-10. Default `1` |
| `per_page` | integer | No | Company-search results per page, 1-50. Default `20` |
| `min_rating` | number | No | Minimum TrustScore for company search |
| `min_review_count` | integer | No | Minimum review count for company search |
| `sort` | string | No | Category sort: `reviews_count` or `latest_review` |
| `claimed` | boolean | No | Filter category results to claimed businesses |
| `limit` | integer | No | Category results per page, 1-50. Default `20` |
| `trustscore` | number | No | Minimum TrustScore for category search |

## Example Company Search Input

```json
{
  "search_type": "company_search",
  "query": "amazon",
  "country": "US",
  "min_rating": 3,
  "min_review_count": 100,
  "page": 1,
  "max_pages": 2
}
```

## Example Category Search Input

```json
{
  "search_type": "category",
  "category": "electronics_technology",
  "country": "US",
  "sort": "reviews_count",
  "limit": 20,
  "max_pages": 2
}
```

## Output

Each Trustpilot business is saved as one dataset item:

```json
{
  "business_name": "Amazon",
  "identifying_name": "amazon.com",
  "website_url": "https://www.amazon.com",
  "profile_url": "https://www.trustpilot.com/review/amazon.com",
  "trust_score": 1.4,
  "stars": 1.5,
  "review_count": 38765,
  "is_claimed": true,
  "is_verified": false,
  "country": "United States",
  "country_code": "US",
  "city": "Seattle",
  "category_names": "Marketplace, Electronics",
  "request_search_type": "company_search",
  "request_query": "amazon",
  "request_page": 1,
  "total_results": 42,
  "total_pages": 3
}
```

The `OUTPUT` record includes the request summary, endpoint used, pages fetched, business count, reported Trustpilot totals, and raw Scrappa responses for the fetched pages.

## Pricing

This Actor is designed for paid, usage-aligned runs. Charge per saved `business-result` dataset item so users pay for delivered company records.

## Notes

Trustpilot search returns public company and category listing data. For higher-volume Trustpilot discovery, details, review monitoring, or direct API access, use Scrappa at https://scrappa.co/api/trustpilot/company-search and https://scrappa.co/api/trustpilot/businesses.

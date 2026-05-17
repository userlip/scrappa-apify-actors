# Trusted Shops Search Scraper

Search Trusted Shops shop profiles by keyword and market through Scrappa. Use it to find reviewed e-commerce stores, monitor competitors, build trust-score datasets, or collect TSIDs for later shop profile and review enrichment.

## Features

- Search Trusted Shops by brand, shop, or domain query
- Filter by Trusted Shops market: Germany, UK, Austria, Switzerland, Netherlands, Spain, Italy, France, Belgium, Poland, or Portugal
- Fetch one or more zero-based search result pages per run
- Extract shop TSID, account name, shop URL, profile URL, rating, review count, certification status, categories, logo, and description
- Dataset rows optimized for Apify table views
- Full Scrappa page responses saved to the `OUTPUT` key-value-store record

## Input

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `q` | string | Yes | Shop, brand, or domain query, for example `zalando` |
| `market` | string | No | Trusted Shops market code. Default `DEU` |
| `page` | integer | No | First zero-based result page, 0-100. Default `0` |
| `max_pages` | integer | No | Number of pages to fetch, 1-10. Default `1` |

## Example Input

```json
{
  "q": "zalando",
  "market": "DEU",
  "page": 0,
  "max_pages": 2
}
```

## Output

Each shop profile is saved as one dataset item:

```json
{
  "accountName": "Example Shop GmbH",
  "shopName": "example-shop.de",
  "tsID": "XFB15FFBDE1DEE7A55D292A7D48598A6A",
  "averageRating": 4.8,
  "reviewCount": 173960,
  "certificationState": true,
  "profile_url": "https://www.trustedshops.de/bewertung/info_XFB15FFBDE1DEE7A55D292A7D48598A6A.html",
  "shop_url": "https://www.example-shop.de",
  "category_names": "Fashion, Shoes",
  "request_q": "zalando",
  "request_market": "DEU",
  "request_page": 0
}
```

The `OUTPUT` record includes the request summary, pages fetched, shop count, reported Trusted Shops totals, and raw Scrappa responses for the fetched pages.

## Notes

Trusted Shops search returns public shop profile summaries. For shop details, review feeds, higher-volume access, or direct API usage, use Scrappa at https://scrappa.co.

# Similarweb Traffic Analytics Scraper

Analyze website traffic, rankings, engagement, acquisition channels, top countries, monthly visits, and top keywords for one or more domains. The actor wraps Scrappa's `/api/similarweb` endpoint and writes one Apify dataset item per analyzed domain.

## What you get

- Global, country, and category ranking data
- Monthly visit estimates and latest-month visit totals
- Engagement metrics: visits, time on site, pages per visit, and bounce rate
- Traffic source shares for direct, search, social, referrals, mail, and paid referrals
- Top countries, top keywords, and screenshot URL when Similarweb returns them
- Batch input so many domains can be analyzed in one Apify run

## Input

Single domain:

```json
{
  "domain": "google.com"
}
```

Batch domains:

```json
{
  "domains": [
    "google.com",
    "https://www.github.com/features",
    "shopify.com"
  ]
}
```

You can also provide both `domain` and `domains`; duplicates are normalized and processed once.

## Output

The dataset contains one item per analyzed domain:

```json
{
  "success": true,
  "domain": "google.com",
  "site_name": "google.com",
  "title": "Google",
  "category": "computers_electronics_and_technology/search_engines",
  "global_rank_value": 1,
  "country_rank_value": 1,
  "country_code": "US",
  "visits": 84172772881,
  "time_on_site": 592.7030576468261,
  "page_per_visit": 8.358154686471503,
  "bounce_rate": 0.28502974800803477,
  "traffic_search": 0.07654766669343356,
  "latest_month": "2025-12-01",
  "latest_month_visits": 84172772881,
  "top_countries": [
    { "country_code": "US", "country_id": 840, "share": 0.24660383506940103 }
  ],
  "top_keywords": [
    { "keyword": "gmail", "share": 91692190, "volume": 114101130, "cpc": 1.55 }
  ],
  "request_domain": "google.com",
  "input_domain": "google.com"
}
```

Domains with a completed no-data response still produce a dataset item:

```json
{
  "success": false,
  "domain": "new-example-site.invalid",
  "request_domain": "new-example-site.invalid",
  "error": "No traffic data available"
}
```

## Notes

Use batch input for competitor monitoring, market maps, prospect qualification, and SEO research lists. For higher-volume Similarweb traffic data or direct API access, use Scrappa at `https://scrappa.co/api/similarweb`.

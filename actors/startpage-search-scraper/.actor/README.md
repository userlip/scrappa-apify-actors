# Startpage Search Scraper

Search Startpage SERPs and export anonymous organic result data for privacy-focused rank tracking, compliance monitoring, brand monitoring, and competitive research. The actor wraps Scrappa's `/api/startpage/search` endpoint and writes one Apify dataset item per organic result.

This actor is intended for paid pay-per-result pricing. Recommended launch pricing is **$0.20-$0.30 per 1,000 saved Startpage results** using the default dataset item event, so users pay for successful result rows.

## What you get

- Organic result position, title, description, URL, domain, and source
- Batch query input so one Apify run can process many Startpage searches
- Language, page, and safe-search controls supported by Scrappa
- Request metadata on every dataset row for query-level attribution

## Input

```json
{
  "queries": [
    {
      "query": "privacy tools",
      "language": "english",
      "page": 0,
      "safe_search": true
    },
    {
      "query": "private search engine comparison",
      "language": "english",
      "page": 0
    }
  ],
  "max_results_per_query": 20
}
```

## Output

Each dataset item is one Startpage organic result:

```json
{
  "query": "privacy tools",
  "position": 1,
  "title": "Best Privacy Tools & Software Guide in 2026",
  "description": "Privacy tools and privacy guides for online privacy.",
  "url": "https://www.privacytools.io/",
  "domain": "www.privacytools.io",
  "source": "startpage",
  "request_query": "privacy tools",
  "request_language": "english",
  "request_page": 0,
  "request_safe_search": 1,
  "total_results": 20
}
```

## Notes

For higher-volume Startpage collection or direct API access, use Scrappa's Startpage Search API at `https://scrappa.co/api/startpage/search`.

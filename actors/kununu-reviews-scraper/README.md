# Kununu Reviews Scraper for Campaign Research

The Kununu Reviews Scraper for Campaign Research collects reviews, ratings, and comment details from kununu. Provide the fields listed below; the actor saves source fields such as `company_name`, `company_country`, `company_slug`, and `rating` to an Apify dataset.

## What data can you extract?

The dataset contains fields returned by kununu. The field names below match the Actor output schema. A source may leave optional values empty or omit fields when they are not available for a result.

| Field | Type | Description |
| --- | --- | --- |
| `company_name` | text | Company returned for this result. |
| `company_country` | text | Country returned for this result. |
| `company_slug` | text | Slug returned for this result. |
| `rating` | number | Rating returned for this result. |
| `rounded_rating` | number | Rounded Rating returned for this result. |
| `title` | text | Title returned for this result. |
| `text` | text | Review returned for this result. |
| `date` | date | Date returned for this result. |
| `review_type` | text | Type returned for this result. |
| `reviewer_position` | text | Position returned for this result. |
| `reviewer_department` | text | Department returned for this result. |
| `reviewer_employment_status` | text | Employment Status returned for this result. |
| `reviewer_recommended` | boolean | Recommended returned for this result. |
| `review_id` | text | Review ID returned for this result. |
| `page` | number | Page returned for this result. |
| `source_url` | link | Source URL returned for this result. |

## Use cases

- Collect reviews, ratings, and comment details to support reputation research.
- Compare records across the input queries or entities you provide.
- Export structured results to research and reporting workflows.

## How to use

1. Open the **Input** tab and use the example JSON below.
2. Change the query, URL, identifier, or other fields you need. For multi-target work, use `targets` and start with a short list.
3. Start the Actor. Open the run's default dataset to inspect, download, or export the returned records.

```json
{
  "targets": [
    "de/bmwgroup"
  ],
  "company_slug": "bmwgroup",
  "page": 1
}
```

Apify stores the run output in a dataset. You can download the dataset in JSON, CSV, Excel, XML, or other available formats from the run page.

## Output example

This illustrative record uses synthetic values. It shows the real output field names; optional source values may be null or absent.

```json
{
  "company_name": "Example Company",
  "company_country": "42",
  "company_slug": "Example Company",
  "rating": 4.7,
  "rounded_rating": 4.7,
  "title": "Example result",
  "text": "Example public text."
}
```

## Input fields

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `targets` | array of string | No | Kununu company slugs, country/slug pairs, or full Kununu company URLs. Up to 25 companies per run. Constraints: maximum 25 items. |
| `company_slug` | string | No | Single Kununu company slug for compatibility. Ignored when Companies is provided. |
| `country` | string | No | Kununu country for bare slugs. Kununu supports Germany, Austria, and Switzerland. Constraints: allowed values: de, at, ch. |
| `page` | integer | No | First Kununu review page to fetch. Constraints: minimum 1; maximum 100. |
| `max_pages` | integer | No | Number of pages to fetch per company target. Constraints: minimum 1; maximum 25. |
| `review_type` | string | No | Employee reviews or candidate interview reviews. Constraints: allowed values: employees, candidates. |
| `sort` | string | No | Optional Kununu sort order. Leave empty for Kununu relevance order. Constraints: allowed values: newest, oldest, best, worst. |
| `score_filters` | array of string | No | Filter by Kununu score buckets. |
| `recommended_filters` | array of string | No | Filter by recommendation status. |
| `jobstatus_filters` | array of string | No | Filter employee reviews by current or former employees. |
| `position_filters` | array of string | No | Filter by reviewer position. |
| `department_filters` | array of string | No | Filter by department. |
| `response_filters` | array of string | No | Filter by whether the company responded. |
| `date_filters` | array of string | No | Filter by review age. |
| `fetch_factor_scores` | boolean | No | Include detailed factor ratings when Kununu provides them. |
| `include_raw_review` | boolean | No | Include the full Scrappa review object in each dataset item. Leave disabled for smaller, cheaper dataset output. |
| `include_raw_responses` | boolean | No | Include full per-page Scrappa responses in the OUTPUT key-value-store record. Leave disabled for large batch runs. |

## Pricing

**Current live price:** $0.25 per 1,000 results.

The price is based on the latest live Apify pricing entry. Per-result charges depend on the number of billed results returned. If the price line lists subscription tiers, the rate shown for each tier applies to that Apify subscription level.

## FAQ

### Is it legal to scrape this data?

This Actor is intended for data that is publicly available from kununu. You are responsible for following the source site's terms, privacy and copyright rules, and the laws that apply to your use of the data. Only collect information you have a lawful basis to use.

### What limits should I expect?

Use the field constraints and limits in the input table. The source controls which records are available, so a narrow query or unavailable page can return fewer results or none. Keep batch lists small when you need a quick first run.

### Can I run it through the API or connect it to other tools?

Yes. Send the same JSON input to `POST https://api.apify.com/v2/actors/thescrappa~kununu-reviews-scraper/runs`. The response includes `defaultDatasetId`; use it to fetch the run's dataset items. See the [Apify Run Actor API](https://docs.apify.com/api/v2/actors-runs-post) and [Actor runs guide](https://docs.apify.com/api/v2/actors-actor-runs). Apify integrations for [Make](https://docs.apify.com/integrations/make), [Zapier](https://docs.apify.com/integrations/zapier), and [n8n](https://docs.apify.com/integrations/n8n) can trigger runs and pass results to the next workflow step. Send dataset rows to Google Sheets through a Sheets step or a workflow integration.

### What happens if a request fails?

Apify reports input validation and source request errors in the run details. Review the error, correct the input, and retry after a temporary source problem. Depending on when a request stops, the dataset may be empty or contain results collected before the failure.

## Related Scrappa Actors

- [Google Maps Reviews Scraper for Local Reputation](https://apify.com/thescrappa/google-maps-reviews-scraper)
- [Jameda Reviews Scraper for Lead Research](https://apify.com/thescrappa/jameda-reviews-scraper)
- [TrustedShops Reviews Scraper for Campaign Research](https://apify.com/thescrappa/trustedshops-reviews-scraper)
- [Trusted Shops Search Scraper for Campaign Research](https://apify.com/thescrappa/trustedshops-search-scraper)
- [TrustedShops Shop Profile Scraper for Marketing](https://apify.com/thescrappa/trustedshops-shop-profile-scraper)

# Kununu Reviews Scraper

Extract Kununu company reviews through Scrappa. Use it for DACH employer reputation monitoring, HR competitor analysis, candidate-experience research, and company-rating datasets.

This Actor is a thin wrapper: Apify validates input, calls `https://scrappa.co/api/kununu/reviews`, and writes one dataset item per Kununu review. Scraping runs on Scrappa infrastructure.

## Features

- Batch `targets` so one Apify run can process multiple Kununu companies
- Accepts Kununu slugs, `country/slug` pairs, and full Kununu company URLs
- Employee and candidate review modes
- Review scores, title, text blocks, dates, and reviewer role metadata
- Filters for score, recommendation, job status, position, department, response status, date, and factor scores
- Dataset rows optimized for Apify table views
- One paid `review-result` event per saved review

## Input

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `targets` | array | No | Kununu company slugs, `de/bmwgroup` pairs, or full Kununu URLs. Up to 25 companies per run. |
| `company_slug` | string | No | Single company slug for compatibility when `targets` is not provided. |
| `country` | string | No | Default country for bare slugs: `de`, `at`, or `ch`. Default `de`. |
| `page` | integer | No | First review page to fetch. Default `1`. |
| `max_pages` | integer | No | Pages to fetch per company target. Default `1`, max `25`. |
| `review_type` | string | No | `employees` or `candidates`. Default `employees`. |
| `sort` | string | No | `newest`, `oldest`, `best`, or `worst`. Empty uses Kununu relevance order. |
| `score_filters` | array | No | `excellent`, `good`, `satisfactory`, `subpar`. |
| `recommended_filters` | array | No | `yes` or `no`. |
| `jobstatus_filters` | array | No | `current` or `former`. |
| `position_filters` | array | No | Reviewer position filters. |
| `department_filters` | array | No | Department filters. |
| `response_filters` | array | No | `yes` or `no` company-response filter. |
| `date_filters` | array | No | `24months`, `12months`, `6months`, or `30days`. |
| `fetch_factor_scores` | boolean | No | Include detailed rating factors when available. |
| `include_raw_review` | boolean | No | Include the full raw review object in each dataset item. Default `false`. |
| `include_raw_responses` | boolean | No | Include full per-page Scrappa responses in `OUTPUT`. Default `false`. |

## Example Input

```json
{
  "targets": [
    "de/bmwgroup",
    "https://www.kununu.com/de/sap-se"
  ],
  "page": 1,
  "max_pages": 2,
  "review_type": "employees",
  "sort": "newest",
  "score_filters": ["excellent", "good"],
  "fetch_factor_scores": true,
  "include_raw_review": false,
  "include_raw_responses": false
}
```

## Output

Each Kununu review is saved as one dataset item:

```json
{
  "review_id": "example-review-id",
  "company_target": "de/bmwgroup",
  "company_country": "de",
  "company_slug": "bmwgroup",
  "company_name": "BMW Group",
  "rating": 4.2,
  "rounded_rating": 4,
  "title": "Good employer",
  "text": "Pros and cons from the review text blocks.",
  "date": "2026-05-01T12:30:00.000Z",
  "review_type": "employees",
  "reviewer_position": "employee",
  "reviewer_department": "it",
  "reviewer_employment_status": "current",
  "reviewer_recommended": true,
  "page": 1,
  "source_url": "https://www.kununu.com/de/bmwgroup"
}
```

The default dataset stays compact and flattened for Apify table/export usage. Enable `include_raw_review` only when you need full per-review details such as raw rating factors or company responses.

The `OUTPUT` key-value-store record includes target/page metadata and per-page summaries. Enable `include_raw_responses` only when you need full Scrappa page payloads for debugging.

## Notes

Kununu operates in Germany, Austria, and Switzerland. For higher-volume review monitoring or direct API access, use Scrappa at https://scrappa.co.

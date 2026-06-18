# LinkedIn Job Details Scraper

Extract structured public LinkedIn job posting details from one or more LinkedIn job URLs through Scrappa. No LinkedIn login, cookies, or browser automation required.

## What It Returns

- Job title, company, location, employment type, seniority, and posted date when present
- Applicant count and apply URL when LinkedIn exposes them
- Original input URL, normalized LinkedIn job URL, cache metadata, and the raw Scrappa response fields
- One dataset item per requested job URL, including recoverable per-job failure rows

## Input

Use `urls` for normal batch usage. The legacy `url` field is still supported for one-off runs.

```json
{
  "urls": [
    "https://www.linkedin.com/jobs/view/1234567890/",
    "https://www.linkedin.com/jobs/view/software-engineer-at-example-2345678901/?trk=public_jobs_topcard-title"
  ],
  "use_cache": true,
  "maximum_cache_age": 2592000
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `urls` | string[] | No | Recommended batch list of LinkedIn job posting URLs. |
| `url` | string | No | Backward-compatible single LinkedIn job URL. |
| `use_cache` | boolean | No | Use cached Scrappa data when available. Defaults to `true`. |
| `maximum_cache_age` | integer | No | Maximum cache age in seconds. Defaults to 30 days. |

At least one URL must be supplied through `urls` or `url`.

## Output

Each LinkedIn job URL produces one dataset item.

```json
{
  "success": true,
  "title": "Software Engineer",
  "company": "Example Corp",
  "location": "New York, NY",
  "employment_type": "Full-time",
  "seniority_level": "Mid-Senior level",
  "posted_date": "2026-06-01",
  "applicants": "23 applicants",
  "apply_url": "https://www.linkedin.com/jobs/view/1234567890/",
  "url": "https://www.linkedin.com/jobs/view/1234567890",
  "input_url": "https://www.linkedin.com/jobs/view/1234567890/?trk=public_jobs_topcard-title",
  "normalized_url": "https://www.linkedin.com/jobs/view/1234567890"
}
```

Recoverable misses are saved as dataset rows instead of failing the entire batch:

```json
{
  "success": false,
  "input_url": "https://www.linkedin.com/jobs/view/missing-job",
  "normalized_url": "https://www.linkedin.com/jobs/view/missing-job",
  "url": "https://www.linkedin.com/jobs/view/missing-job",
  "message": "Job not found",
  "status_code": 404,
  "error_type": "scrappa_api_error"
}
```

## Batch Workflow

This actor is designed for the common workflow where LinkedIn Jobs Search returns job URLs and a second step enriches those URLs with posting details. Submit many job URLs in a single run so Apify run startup and storage overhead are shared across results.

## Direct API

For higher-volume or direct API access, use Scrappa's LinkedIn Job endpoint at `https://scrappa.co/api`.

## Support

For issues or questions, contact us through Apify.
